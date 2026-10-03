//! Payment-route tag repair commands for ADR 0065.
//!
//! ADR 0076 Decision 9 amends the source of the route: the repair writes the
//! route stored in the database. It asks `MusicIndex` only when the database
//! has no route for the track, and it stores that response before the write.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use rusqlite::Connection;
use serde::Serialize;

use crate::api::{self, Client, Feed, Track};
use crate::application::application_query_service::ApplicationQueryService;
use crate::application::command_bus::{ApplicationCommand, CommandOutcome, CommandResult};
use crate::application::command_context::CommandContext;
use crate::application::errors::command::CommandError;
use crate::application::events::metadata::MetadataEvent;
use crate::application::events::ApplicationEvent;
use crate::application::queries::broadcast::NO_RSS_RECIPIENTS_REASON;
use crate::application::queries::broadcast::{self, BroadcastReadinessState};
use crate::audio_tags::{read_audio_tags, write_id3v24_edits};
use crate::db::{self, LocalMetadataFactInput, LocalMetadataOwner, LocalMetadataValue, TrackRow};
use crate::metadata::{
    audio_tags_have_ready_value_routes, MUSICINDEX_METADATA_SOURCE,
    MUSICINDEX_PAYMENT_ROUTES_ABSENT_FACT_KEY,
};
use crate::metadata_service::{with_stored_route_frame, RouteFrameWrite};

type SharedConnection = Arc<Mutex<Connection>>;

const PAYMENT_ROUTES_INCLUDE: &str = "payment_routes";

/// Per-track payment-route repair status.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PaymentRouteRepairStatus {
    /// The command wrote the embedded value-routes tag.
    Repaired,
    /// MusicIndex has no routes for the track or its feed.
    NoRoutesUpstream,
    /// The command could not fetch or write the repair.
    Failed,
    /// The file already carried a usable value-routes tag.
    Skipped,
}

/// Result for one track repair request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct PaymentRouteRepairTrackResult {
    /// Local track database id.
    pub(crate) track_id: i64,
    /// Best available title for human output.
    pub(crate) title: Option<String>,
    /// Repair status for this track.
    pub(crate) status: PaymentRouteRepairStatus,
    /// Number of frames written to the file.
    pub(crate) frames_written: usize,
    /// Failure or skip detail.
    pub(crate) reason: Option<String>,
}

impl PaymentRouteRepairTrackResult {
    fn repaired(track: &TrackRow, frames_written: usize) -> Self {
        Self {
            track_id: track.id,
            title: track_title(track),
            status: PaymentRouteRepairStatus::Repaired,
            frames_written,
            reason: None,
        }
    }

    fn no_routes_upstream(track_id: i64, title: Option<String>) -> Self {
        Self {
            track_id,
            title,
            status: PaymentRouteRepairStatus::NoRoutesUpstream,
            frames_written: 0,
            reason: Some("MusicIndex has no payment routes for this track or feed.".to_owned()),
        }
    }

    fn no_routes_in_rss(track_id: i64, title: Option<String>) -> Self {
        Self {
            track_id,
            title,
            status: PaymentRouteRepairStatus::NoRoutesUpstream,
            frames_written: 0,
            reason: Some(NO_RSS_RECIPIENTS_REASON.to_owned()),
        }
    }

    fn failed(track_id: i64, title: Option<String>, reason: impl Into<String>) -> Self {
        Self {
            track_id,
            title,
            status: PaymentRouteRepairStatus::Failed,
            frames_written: 0,
            reason: Some(reason.into()),
        }
    }

    fn skipped(track: &TrackRow, reason: impl Into<String>) -> Self {
        Self {
            track_id: track.id,
            title: track_title(track),
            status: PaymentRouteRepairStatus::Skipped,
            frames_written: 0,
            reason: Some(reason.into()),
        }
    }
}

/// Aggregate counts for a repair-all request.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub(crate) struct PaymentRouteRepairSummary {
    /// Tracks whose route tag was written.
    pub(crate) repaired: usize,
    /// Tracks known to have no upstream routes.
    pub(crate) no_routes_upstream: usize,
    /// Tracks whose fetch or write failed.
    pub(crate) failed: usize,
    /// Tracks skipped because no write was needed.
    pub(crate) skipped: usize,
}

impl PaymentRouteRepairSummary {
    fn record(&mut self, status: &PaymentRouteRepairStatus) {
        match status {
            PaymentRouteRepairStatus::Repaired => self.repaired += 1,
            PaymentRouteRepairStatus::NoRoutesUpstream => self.no_routes_upstream += 1,
            PaymentRouteRepairStatus::Failed => self.failed += 1,
            PaymentRouteRepairStatus::Skipped => self.skipped += 1,
        }
    }
}

/// Result for a repair-all request.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub(crate) struct PaymentRouteRepairBatchResult {
    /// Aggregate repair counts.
    pub(crate) summary: PaymentRouteRepairSummary,
    /// One result for each scanned not-ready track.
    pub(crate) tracks: Vec<PaymentRouteRepairTrackResult>,
}

impl PaymentRouteRepairBatchResult {
    fn from_tracks(tracks: Vec<PaymentRouteRepairTrackResult>) -> Self {
        let mut summary = PaymentRouteRepairSummary::default();
        for track in &tracks {
            summary.record(&track.status);
        }
        Self { summary, tracks }
    }
}

/// Repairs the value-routes tag for one local track.
#[derive(Clone, Debug)]
pub(crate) struct RepairPaymentRoutesForTrack {
    conn: SharedConnection,
    musicindex_endpoint: crate::config::MusicIndexEndpoint,
    music_dir: PathBuf,
    track_id: i64,
}

impl RepairPaymentRoutesForTrack {
    /// Creates a single-track payment-route repair command.
    #[must_use]
    pub(crate) fn new(
        conn: SharedConnection,
        musicindex_endpoint: impl Into<crate::config::MusicIndexEndpoint>,
        music_dir: PathBuf,
        track_id: i64,
    ) -> Self {
        Self {
            conn,
            musicindex_endpoint: musicindex_endpoint.into(),
            music_dir,
            track_id,
        }
    }
}

impl ApplicationCommand for RepairPaymentRoutesForTrack {
    type Output = PaymentRouteRepairTrackResult;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let client = Client::new_with_base_url(self.musicindex_endpoint);
        let db = self
            .conn
            .lock()
            .map_err(|_| CommandError::Metadata("database lock poisoned".to_owned()))?;
        // The operator targeted this track, so ask upstream again.
        let result = repair_payment_routes_for_track_with_client(
            &db,
            &self.music_dir,
            self.track_id,
            &client,
            RecheckUpstream::Ask,
        );
        let events = metadata_events_for_results(std::slice::from_ref(&result));
        Ok(CommandOutcome::new(result, events))
    }
}

/// Repairs value-routes tags for every not-ready local track.
#[derive(Clone, Debug)]
pub(crate) struct RepairMissingPaymentRouteTags {
    conn: SharedConnection,
    musicindex_endpoint: crate::config::MusicIndexEndpoint,
    music_dir: PathBuf,
}

impl RepairMissingPaymentRouteTags {
    /// Creates a repair-all command for broadcast readiness.
    #[must_use]
    pub(crate) fn new(
        conn: SharedConnection,
        musicindex_endpoint: impl Into<crate::config::MusicIndexEndpoint>,
        music_dir: PathBuf,
    ) -> Self {
        Self {
            conn,
            musicindex_endpoint: musicindex_endpoint.into(),
            music_dir,
        }
    }
}

impl ApplicationCommand for RepairMissingPaymentRouteTags {
    type Output = PaymentRouteRepairBatchResult;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let client = Client::new_with_base_url(self.musicindex_endpoint);
        let db = self
            .conn
            .lock()
            .map_err(|_| CommandError::Metadata("database lock poisoned".to_owned()))?;
        let result = repair_missing_payment_routes_with_client(&db, &self.music_dir, &client)
            .map_err(|error| CommandError::Query(format!("{error:#}")))?;
        let events = metadata_events_for_results(&result.tracks);
        Ok(CommandOutcome::new(result, events))
    }
}

trait PaymentRouteApi {
    fn fetch_track(&self, track_guid: &str, include: Option<&str>) -> Result<Track>;

    fn fetch_feed_track(
        &self,
        feed_guid: &str,
        track_guid: &str,
        include: Option<&str>,
    ) -> Result<Track>;

    fn fetch_feed(&self, feed_guid: &str, include: Option<&str>) -> Result<Feed>;
}

impl PaymentRouteApi for Client {
    fn fetch_track(&self, track_guid: &str, include: Option<&str>) -> Result<Track> {
        Client::fetch_track(self, track_guid, include)
    }

    fn fetch_feed_track(
        &self,
        feed_guid: &str,
        track_guid: &str,
        include: Option<&str>,
    ) -> Result<Track> {
        Client::fetch_feed_track(self, feed_guid, track_guid, include)
    }

    fn fetch_feed(&self, feed_guid: &str, include: Option<&str>) -> Result<Feed> {
        Client::fetch_feed(self, feed_guid, include)
    }
}

fn repair_missing_payment_routes_with_client<C: PaymentRouteApi>(
    conn: &Connection,
    music_dir: &Path,
    client: &C,
) -> Result<PaymentRouteRepairBatchResult> {
    let report = ApplicationQueryService::new()
        .broadcast_readiness_report(conn, music_dir)
        .map_err(|error| anyhow!("{error}"))?;
    let mut results = Vec::new();
    for track in report.problem_tracks() {
        match track.state {
            // ADR 0076 packet 003: a removed track waits for the operator, and
            // packet 004 owns the write of an out-of-date route.
            BroadcastReadinessState::Ready
            | BroadcastReadinessState::RemovedFromFeed
            | BroadcastReadinessState::RouteOutOfDate => {}
            BroadcastReadinessState::NoRoutesUpstream => {
                results.push(PaymentRouteRepairTrackResult::no_routes_upstream(
                    track.track_id,
                    Some(track.title.clone()),
                ));
            }
            BroadcastReadinessState::NoRouteTag
            | BroadcastReadinessState::FileMissing
            | BroadcastReadinessState::NotDownloaded => {
                results.push(repair_payment_routes_for_track_with_client(
                    conn,
                    music_dir,
                    track.track_id,
                    client,
                    RecheckUpstream::Trust,
                ));
            }
        }
    }
    Ok(PaymentRouteRepairBatchResult::from_tracks(results))
}

/// How a repair treats a track already recorded as having no upstream routes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RecheckUpstream {
    /// Trust the recorded answer and do not ask again. A batch run uses this, so
    /// it does not refetch every track a publisher has not fixed.
    Trust,
    /// Ask again. An operator who targets one track is saying "try again", and a
    /// publisher may have added the routes since the last answer.
    Ask,
}

fn repair_payment_routes_for_track_with_client<C: PaymentRouteApi>(
    conn: &Connection,
    music_dir: &Path,
    track_id: i64,
    client: &C,
    recheck: RecheckUpstream,
) -> PaymentRouteRepairTrackResult {
    let track = match db::track_row_by_id(conn, track_id) {
        Ok(Some(track)) => track,
        Ok(None) => {
            return PaymentRouteRepairTrackResult::failed(track_id, None, "track was not found");
        }
        Err(error) => {
            return PaymentRouteRepairTrackResult::failed(
                track_id,
                None,
                format!("read track row: {error:#}"),
            );
        }
    };

    repair_loaded_payment_routes_track(conn, music_dir, &track, client, recheck)
}

fn repair_loaded_payment_routes_track<C: PaymentRouteApi>(
    conn: &Connection,
    music_dir: &Path,
    track: &TrackRow,
    client: &C,
    recheck: RecheckUpstream,
) -> PaymentRouteRepairTrackResult {
    let title = track_title(track);
    let Some(path) = track
        .local_path
        .as_ref()
        .map(|path| path.resolve(music_dir))
    else {
        return PaymentRouteRepairTrackResult::failed(
            track.id,
            title,
            "track has no downloaded file",
        );
    };
    if !path.is_file() {
        return PaymentRouteRepairTrackResult::failed(
            track.id,
            title,
            format!("recorded local file is missing: {}", path.display()),
        );
    }

    if let Ok(tags) = read_audio_tags(&path) {
        if audio_tags_have_ready_value_routes(&tags) {
            return PaymentRouteRepairTrackResult::skipped(
                track,
                "file already carries payment routes",
            );
        }
    }

    // ADR 0076 Decision 9: a stored route is the source of the write, and the
    // repair then asks MusicIndex nothing.
    match db::payment_routes::stored_route(conn, track.id) {
        Ok(Some(routes)) if routes.is_empty() => {
            return PaymentRouteRepairTrackResult::no_routes_in_rss(track.id, title);
        }
        Ok(Some(_)) => return write_stored_route(conn, &path, track, None),
        Ok(None) => {}
        Err(error) => {
            return PaymentRouteRepairTrackResult::failed(
                track.id,
                title,
                format!("read stored payment route: {error:#}"),
            );
        }
    }

    match broadcast::track_has_no_upstream_payment_routes(conn, track) {
        // Trust the recorded answer only for a batch run. Without the second
        // arm, a track recorded once could never be retried, not even by an
        // operator who targets it after the publisher added the routes.
        Ok(true) if recheck == RecheckUpstream::Trust => {
            return PaymentRouteRepairTrackResult::no_routes_upstream(track.id, title)
        }
        Ok(true | false) => {}
        Err(error) => {
            return PaymentRouteRepairTrackResult::failed(
                track.id,
                title,
                format!("read payment-route repair state: {error:#}"),
            );
        }
    }

    let (fetched_track, _fetched_feed) = match fetch_musicindex_payment_routes(client, track) {
        Ok(context) => context,
        Err(error) => {
            return PaymentRouteRepairTrackResult::failed(
                track.id,
                title,
                format!("fetch MusicIndex payment routes: {error:#}"),
            );
        }
    };

    if fetched_track
        .payment_routes
        .as_ref()
        .is_none_or(Vec::is_empty)
    {
        if let Err(error) = record_payment_routes_absent(conn, track.id, true, &fetched_track) {
            return PaymentRouteRepairTrackResult::failed(
                track.id,
                title,
                format!("record no upstream routes: {error:#}"),
            );
        }
        return PaymentRouteRepairTrackResult::no_routes_upstream(track.id, title);
    }

    // The database has no route, so the MusicIndex response goes into the
    // database first. The frame then uses the stored value.
    write_stored_route(conn, &path, track, Some(&fetched_track))
}

/// Writes the route frame from the stored route (ADR 0076 Decision 9). A
/// `MusicIndex` track is stored first when the database has no route.
fn write_stored_route(
    conn: &Connection,
    path: &Path,
    track: &TrackRow,
    musicindex: Option<&Track>,
) -> PaymentRouteRepairTrackResult {
    let title = track_title(track);
    let edits = match with_stored_route_frame(
        conn,
        track.id,
        musicindex.and_then(|fetched| fetched.payment_routes.as_deref()),
        Vec::new(),
        RouteFrameWrite::Always,
    ) {
        Ok(edits) if edits.is_empty() => {
            return PaymentRouteRepairTrackResult::failed(
                track.id,
                title,
                "the stored payment route gave no value-routes edit",
            );
        }
        Ok(edits) => edits,
        Err(error) => {
            return PaymentRouteRepairTrackResult::failed(
                track.id,
                title,
                format!("read stored payment route: {error:#}"),
            );
        }
    };

    match write_id3v24_edits(path, &edits) {
        Ok(write) if write.applied > 0 => {
            if let Some(fetched_track) = musicindex {
                if let Err(error) =
                    record_payment_routes_absent(conn, track.id, false, fetched_track)
                {
                    return PaymentRouteRepairTrackResult::failed(
                        track.id,
                        title,
                        format!("record repaired route state: {error:#}"),
                    );
                }
            }
            PaymentRouteRepairTrackResult::repaired(track, write.applied)
        }
        Ok(_) => PaymentRouteRepairTrackResult::failed(
            track.id,
            title,
            "payment-route edit wrote no frames",
        ),
        Err(error) => PaymentRouteRepairTrackResult::failed(
            track.id,
            title,
            format!("write value-routes tag: {error:#}"),
        ),
    }
}

fn fetch_musicindex_payment_routes<C: PaymentRouteApi>(
    client: &C,
    track: &TrackRow,
) -> Result<(Track, Option<Feed>)> {
    let fetched_track = match track.feed_guid.as_deref() {
        Some(feed_guid) => client
            .fetch_feed_track(feed_guid, &track.item_guid, Some(PAYMENT_ROUTES_INCLUDE))
            .or_else(|_| client.fetch_track(&track.item_guid, Some(PAYMENT_ROUTES_INCLUDE)))?,
        None => client.fetch_track(&track.item_guid, Some(PAYMENT_ROUTES_INCLUDE))?,
    };
    let feed_guid = fetched_track
        .feed_guid
        .as_deref()
        .or(track.feed_guid.as_deref());
    let fetched_feed = if fetched_track.payment_routes.is_none() {
        feed_guid
            .map(|feed_guid| client.fetch_feed(feed_guid, Some(PAYMENT_ROUTES_INCLUDE)))
            .transpose()?
    } else {
        None
    };
    let fetched_track = api::track_with_feed_defaults(fetched_track, fetched_feed.as_ref());
    Ok((fetched_track, fetched_feed))
}

fn record_payment_routes_absent(
    conn: &Connection,
    track_id: i64,
    absent: bool,
    fetched_track: &Track,
) -> Result<()> {
    db::replace_local_metadata_fact(
        conn,
        LocalMetadataOwner::Track(track_id),
        MUSICINDEX_METADATA_SOURCE,
        &LocalMetadataFactInput {
            fact_key: MUSICINDEX_PAYMENT_ROUTES_ABSENT_FACT_KEY.to_owned(),
            value: LocalMetadataValue::Boolean(absent),
            extraction_path: Some("$.payment_routes".to_owned()),
            observed_at: fetched_track.updated_at,
            raw_json: serde_json::to_string(fetched_track).ok(),
        },
    )
}

fn metadata_events_for_results(results: &[PaymentRouteRepairTrackResult]) -> Vec<ApplicationEvent> {
    results
        .iter()
        .filter(|result| result.status == PaymentRouteRepairStatus::Repaired)
        .map(|result| {
            ApplicationEvent::Metadata(MetadataEvent::TrackTagged {
                track_id: result.track_id,
            })
        })
        .collect()
}

fn track_title(track: &TrackRow) -> Option<String> {
    track
        .track_title
        .clone()
        .or_else(|| track.feed_title.clone())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use anyhow::Context;

    use crate::api::PaymentRoute;
    use crate::audio_tags::{read_audio_tags, Id3v24Edit};
    use crate::library_path::LibraryRelativePath;
    use crate::metadata::audio_tags_have_ready_value_routes;
    use crate::metadata::MUSICINDEX_VALUE_ROUTES_FRAME;

    use super::*;

    fn setup_test_db() -> Result<Connection> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        db::init_schema(&conn)?;
        db::migrate_schema(&conn)?;
        Ok(conn)
    }

    fn create_track_with_file(
        conn: &Connection,
        music_dir: &Path,
        item_guid: &str,
        file_name: &str,
        bytes: &[u8],
    ) -> Result<(i64, PathBuf)> {
        let feed_id = ensure_test_feed(conn)?;
        conn.execute(
            "INSERT INTO tracks (
                 feed_id, item_guid, track_title, artist_name, album_title, is_in_library
             )
             VALUES (?1, ?2, ?3, ?4, ?5, 1)",
            rusqlite::params![feed_id, item_guid, "Track Title", "Artist", "Album"],
        )?;
        let track_id = conn.last_insert_rowid();
        let path = music_dir.join(file_name);
        std::fs::create_dir_all(path.parent().context("path parent")?)?;
        std::fs::write(&path, bytes)?;
        let relative_path = LibraryRelativePath::from_absolute(music_dir, &path)?;
        db::mark_track_downloaded(conn, track_id, &relative_path, None)?;
        Ok((track_id, path))
    }

    fn ensure_test_feed(conn: &Connection) -> Result<i64> {
        conn.execute(
            "INSERT OR IGNORE INTO feeds (feed_url, feed_guid, title)
             VALUES (?1, ?2, ?3)",
            rusqlite::params!["https://example.test/feed.xml", "feed-guid", "Feed Title"],
        )?;
        let feed_id = conn.query_row(
            "SELECT id FROM feeds WHERE feed_guid = ?1",
            ["feed-guid"],
            |row| row.get(0),
        )?;
        Ok(feed_id)
    }

    fn payment_route() -> PaymentRoute {
        PaymentRoute {
            recipient_name: Some("Artist".into()),
            route_type: Some("node".into()),
            split: Some(100.0),
            address: Some("03abcdef".into()),
            ..PaymentRoute::default()
        }
    }

    #[derive(Debug)]
    struct FakePaymentRouteClient {
        track: Track,
        feed: Option<Feed>,
        fetch_error: Option<String>,
        calls: RefCell<Vec<String>>,
    }

    impl FakePaymentRouteClient {
        fn with_track_and_feed(track: Track, feed: Option<Feed>) -> Self {
            Self {
                track,
                feed,
                fetch_error: None,
                calls: RefCell::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }
    }

    impl PaymentRouteApi for FakePaymentRouteClient {
        fn fetch_track(&self, track_guid: &str, include: Option<&str>) -> Result<Track> {
            self.calls
                .borrow_mut()
                .push(format!("track:{track_guid}:{}", include.unwrap_or("")));
            if let Some(error) = self.fetch_error.as_ref() {
                anyhow::bail!("{error}");
            }
            Ok(self.track.clone())
        }

        fn fetch_feed_track(
            &self,
            feed_guid: &str,
            track_guid: &str,
            include: Option<&str>,
        ) -> Result<Track> {
            self.calls.borrow_mut().push(format!(
                "feed-track:{feed_guid}:{track_guid}:{}",
                include.unwrap_or("")
            ));
            if let Some(error) = self.fetch_error.as_ref() {
                anyhow::bail!("{error}");
            }
            Ok(self.track.clone())
        }

        fn fetch_feed(&self, feed_guid: &str, include: Option<&str>) -> Result<Feed> {
            self.calls
                .borrow_mut()
                .push(format!("feed:{feed_guid}:{}", include.unwrap_or("")));
            if let Some(error) = self.fetch_error.as_ref() {
                anyhow::bail!("{error}");
            }
            self.feed
                .clone()
                .ok_or_else(|| anyhow!("feed fixture missing"))
        }
    }

    #[test]
    fn track_with_routes_upstream_is_repaired() -> Result<()> {
        let conn = setup_test_db()?;
        let temp = tempfile::tempdir()?;
        let (track_id, path) = create_track_with_file(
            &conn,
            temp.path(),
            "track-guid",
            "track.mp3",
            b"not really an mp3",
        )?;
        let client = FakePaymentRouteClient::with_track_and_feed(
            Track {
                track_guid: Some("track-guid".into()),
                feed_guid: Some("feed-guid".into()),
                payment_routes: None,
                ..Track::default()
            },
            Some(Feed {
                feed_guid: Some("feed-guid".into()),
                payment_routes: Some(vec![payment_route()]),
                ..Feed::default()
            }),
        );

        let result = repair_payment_routes_for_track_with_client(
            &conn,
            temp.path(),
            track_id,
            &client,
            RecheckUpstream::Trust,
        );

        assert_eq!(result.status, PaymentRouteRepairStatus::Repaired);
        assert_eq!(result.frames_written, 1);
        assert!(audio_tags_have_ready_value_routes(&read_audio_tags(&path)?));
        assert!(client
            .calls()
            .iter()
            .all(|call| call.ends_with(PAYMENT_ROUTES_INCLUDE)));
        Ok(())
    }

    #[test]
    fn stale_feed_state_does_not_gate_missing_route_tag_repair() -> Result<()> {
        let conn = setup_test_db()?;
        let temp = tempfile::tempdir()?;
        let feed_id = ensure_test_feed(&conn)?;
        db::set_feed_subscribed(&conn, feed_id, true)?;
        db::set_feed_musicindex_updated_at(&conn, feed_id, 1)?;
        let (track_id, path) = create_track_with_file(
            &conn,
            temp.path(),
            "stale-feed-track",
            "stale-feed-track.mp3",
            b"not really an mp3",
        )?;
        let client = FakePaymentRouteClient::with_track_and_feed(
            Track {
                track_guid: Some("stale-feed-track".into()),
                feed_guid: Some("feed-guid".into()),
                payment_routes: Some(vec![payment_route()]),
                ..Track::default()
            },
            None,
        );

        let result = repair_payment_routes_for_track_with_client(
            &conn,
            temp.path(),
            track_id,
            &client,
            RecheckUpstream::Trust,
        );

        assert_eq!(result.status, PaymentRouteRepairStatus::Repaired);
        assert!(audio_tags_have_ready_value_routes(&read_audio_tags(&path)?));
        assert_eq!(
            db::feed_stale_check_row(&conn, feed_id)?
                .context("stale check row")?
                .musicindex_updated_at,
            Some(1)
        );
        Ok(())
    }

    #[test]
    fn track_with_no_routes_upstream_writes_no_file() -> Result<()> {
        let conn = setup_test_db()?;
        let temp = tempfile::tempdir()?;
        let (track_id, path) = create_track_with_file(
            &conn,
            temp.path(),
            "track-guid",
            "track.mp3",
            b"not really an mp3",
        )?;
        let client = FakePaymentRouteClient::with_track_and_feed(
            Track {
                track_guid: Some("track-guid".into()),
                feed_guid: Some("feed-guid".into()),
                payment_routes: None,
                ..Track::default()
            },
            Some(Feed {
                feed_guid: Some("feed-guid".into()),
                payment_routes: Some(Vec::new()),
                ..Feed::default()
            }),
        );

        let result = repair_payment_routes_for_track_with_client(
            &conn,
            temp.path(),
            track_id,
            &client,
            RecheckUpstream::Trust,
        );

        assert_eq!(result.status, PaymentRouteRepairStatus::NoRoutesUpstream);
        assert!(!audio_tags_have_ready_value_routes(&read_audio_tags(
            &path
        )?));
        let track = db::track_row_by_id(&conn, track_id)?.context("track row")?;
        assert!(broadcast::track_has_no_upstream_payment_routes(
            &conn, &track
        )?);
        Ok(())
    }

    /// A batch run trusts the recorded answer, so it does not refetch a track
    /// the publisher has not fixed. An operator who targets one track is asking
    /// for a new answer, and a publisher may have added the routes since.
    /// Without this, a recorded track could never be repaired again.
    #[test]
    fn targeting_one_track_asks_upstream_again() -> Result<()> {
        let conn = setup_test_db()?;
        let temp = tempfile::tempdir()?;
        let (track_id, _path) = create_track_with_file(
            &conn,
            temp.path(),
            "track-without-upstream-routes",
            "without-upstream-routes.mp3",
            b"not really an mp3",
        )?;
        let empty = FakePaymentRouteClient::with_track_and_feed(
            Track {
                track_guid: Some("track-without-upstream-routes".into()),
                feed_guid: Some("feed-guid".into()),
                payment_routes: None,
                ..Track::default()
            },
            Some(Feed {
                feed_guid: Some("feed-guid".into()),
                payment_routes: Some(Vec::new()),
                ..Feed::default()
            }),
        );

        let first = repair_payment_routes_for_track_with_client(
            &conn,
            temp.path(),
            track_id,
            &empty,
            RecheckUpstream::Ask,
        );
        assert_eq!(first.status, PaymentRouteRepairStatus::NoRoutesUpstream);

        let calls_after_first = empty.calls();
        let trusted = repair_payment_routes_for_track_with_client(
            &conn,
            temp.path(),
            track_id,
            &empty,
            RecheckUpstream::Trust,
        );
        assert_eq!(trusted.status, PaymentRouteRepairStatus::NoRoutesUpstream);
        assert_eq!(
            empty.calls(),
            calls_after_first,
            "a batch run trusts the recorded answer and asks nobody"
        );

        let asked_again = repair_payment_routes_for_track_with_client(
            &conn,
            temp.path(),
            track_id,
            &empty,
            RecheckUpstream::Ask,
        );
        assert!(
            empty.calls() > calls_after_first,
            "an operator who targets one track gets a new answer"
        );
        assert_eq!(
            asked_again.status,
            PaymentRouteRepairStatus::NoRoutesUpstream
        );
        Ok(())
    }

    #[test]
    fn second_batch_run_does_not_retry_no_routes_upstream() -> Result<()> {
        let conn = setup_test_db()?;
        let temp = tempfile::tempdir()?;
        create_track_with_file(
            &conn,
            temp.path(),
            "track-without-upstream-routes",
            "without-upstream-routes.mp3",
            b"not really an mp3",
        )?;
        let client = FakePaymentRouteClient::with_track_and_feed(
            Track {
                track_guid: Some("track-without-upstream-routes".into()),
                feed_guid: Some("feed-guid".into()),
                payment_routes: None,
                ..Track::default()
            },
            Some(Feed {
                feed_guid: Some("feed-guid".into()),
                payment_routes: Some(Vec::new()),
                ..Feed::default()
            }),
        );

        let first = repair_missing_payment_routes_with_client(&conn, temp.path(), &client)?;
        let calls_after_first = client.calls();
        let second = repair_missing_payment_routes_with_client(&conn, temp.path(), &client)?;

        assert_eq!(first.summary.no_routes_upstream, 1);
        assert_eq!(first.summary.failed, 0);
        assert_eq!(second.summary.no_routes_upstream, 1);
        assert_eq!(second.summary.failed, 0);
        assert_eq!(client.calls(), calls_after_first);
        Ok(())
    }

    #[test]
    fn track_that_already_has_routes_is_skipped() -> Result<()> {
        let conn = setup_test_db()?;
        let temp = tempfile::tempdir()?;
        let (track_id, path) = create_track_with_file(
            &conn,
            temp.path(),
            "track-guid",
            "track.mp3",
            b"not really an mp3",
        )?;
        write_id3v24_edits(
            &path,
            &[Id3v24Edit {
                frame_label: MUSICINDEX_VALUE_ROUTES_FRAME.to_owned(),
                value: serde_json::to_string(&vec![payment_route()])?,
            }],
        )?;
        let client = FakePaymentRouteClient::with_track_and_feed(Track::default(), None);

        let result = repair_payment_routes_for_track_with_client(
            &conn,
            temp.path(),
            track_id,
            &client,
            RecheckUpstream::Trust,
        );

        assert_eq!(result.status, PaymentRouteRepairStatus::Skipped);
        assert!(client.calls().is_empty());
        Ok(())
    }

    #[test]
    fn write_failure_returns_failed_outcome() -> Result<()> {
        let conn = setup_test_db()?;
        let temp = tempfile::tempdir()?;
        let (track_id, _path) = create_track_with_file(
            &conn,
            temp.path(),
            "track-guid",
            "track.wav",
            b"RIFF\0\0\0\0WAVE",
        )?;
        let client = FakePaymentRouteClient::with_track_and_feed(
            Track {
                track_guid: Some("track-guid".into()),
                payment_routes: Some(vec![payment_route()]),
                ..Track::default()
            },
            None,
        );

        let result = repair_payment_routes_for_track_with_client(
            &conn,
            temp.path(),
            track_id,
            &client,
            RecheckUpstream::Trust,
        );

        assert_eq!(result.status, PaymentRouteRepairStatus::Failed);
        assert!(result
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains("cannot tag raw WAV")));
        Ok(())
    }

    #[test]
    fn second_batch_run_repairs_nothing_more() -> Result<()> {
        let conn = setup_test_db()?;
        let temp = tempfile::tempdir()?;
        create_track_with_file(
            &conn,
            temp.path(),
            "track-with-routes",
            "with-routes.mp3",
            b"not really an mp3",
        )?;
        create_track_with_file(
            &conn,
            temp.path(),
            "track-without-routes",
            "without-routes.mp3",
            b"not really an mp3",
        )?;
        let client = FakePaymentRouteClient::with_track_and_feed(
            Track {
                track_guid: Some("track-guid".into()),
                feed_guid: Some("feed-guid".into()),
                payment_routes: None,
                ..Track::default()
            },
            Some(Feed {
                feed_guid: Some("feed-guid".into()),
                payment_routes: Some(vec![payment_route()]),
                ..Feed::default()
            }),
        );

        let first = repair_missing_payment_routes_with_client(&conn, temp.path(), &client)?;
        let second = repair_missing_payment_routes_with_client(&conn, temp.path(), &client)?;
        let first_json = serde_json::to_value(&first)?;

        assert_eq!(first.summary.repaired, 2);
        assert_eq!(first.summary.failed, 0);
        assert_eq!(first_json["summary"]["repaired"], 2);
        assert_eq!(first_json["summary"]["no_routes_upstream"], 0);
        assert_eq!(first_json["summary"]["failed"], 0);
        assert_eq!(second.summary.repaired, 0);
        assert_eq!(second.summary.failed, 0);
        Ok(())
    }

    #[test]
    fn metadata_events_only_include_repaired_tracks() {
        let events = metadata_events_for_results(&[
            PaymentRouteRepairTrackResult {
                track_id: 7,
                title: None,
                status: PaymentRouteRepairStatus::Repaired,
                frames_written: 1,
                reason: None,
            },
            PaymentRouteRepairTrackResult::no_routes_upstream(8, None),
        ]);

        assert_eq!(
            events,
            vec![ApplicationEvent::Metadata(MetadataEvent::TrackTagged {
                track_id: 7
            })]
        );
    }

    fn named_route(name: &str, split: f64) -> PaymentRoute {
        PaymentRoute {
            recipient_name: Some(name.into()),
            route_type: Some("node".into()),
            split: Some(split),
            fee: Some(false),
            address: Some("03abcdef".into()),
            ..PaymentRoute::default()
        }
    }

    fn file_route_names(path: &Path) -> Vec<Option<String>> {
        let tags = read_audio_tags(path).unwrap();
        let value = crate::metadata::audio_tags_value_routes(&tags).unwrap();
        crate::metadata::parse_value_routes(value)
            .unwrap()
            .into_iter()
            .map(|route| route.recipient_name)
            .collect()
    }

    fn set_stored_route(conn: &Connection, track_id: i64, routes: &[PaymentRoute]) {
        conn.execute(
            "UPDATE tracks SET payment_routes_json = ?2 WHERE id = ?1",
            rusqlite::params![track_id, serde_json::to_string(routes).unwrap()],
        )
        .unwrap();
    }

    fn musicindex_client(name: &str) -> FakePaymentRouteClient {
        FakePaymentRouteClient::with_track_and_feed(
            Track {
                track_guid: Some("track-guid".into()),
                feed_guid: Some("feed-guid".into()),
                payment_routes: Some(vec![named_route(name, 100.0)]),
                ..Track::default()
            },
            None,
        )
    }

    /// R3-03 (repair write) and R3-05: with a stored route, the repair
    /// writes the stored route and sends no MusicIndex request, also when
    /// MusicIndex has a different route.
    #[test]
    fn adr_0076_route_readiness_repair_writes_stored_route_without_request() {
        let conn = setup_test_db().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let (track_id, path) = create_track_with_file(
            &conn,
            temp.path(),
            "track-guid",
            "track.mp3",
            b"not really an mp3",
        )
        .unwrap();
        set_stored_route(&conn, track_id, &[named_route("Stored", 100.0)]);
        let client = musicindex_client("MusicIndex");

        let result = repair_payment_routes_for_track_with_client(
            &conn,
            temp.path(),
            track_id,
            &client,
            RecheckUpstream::Ask,
        );

        assert_eq!(result.status, PaymentRouteRepairStatus::Repaired);
        assert_eq!(file_route_names(&path), vec![Some("Stored".to_owned())]);
        assert!(client.calls().is_empty(), "{:?}", client.calls());
    }

    /// R3-04: with no stored route, the repair asks MusicIndex once, stores
    /// the response, and writes the stored value.
    #[test]
    fn adr_0076_route_readiness_repair_stores_musicindex_route_before_write() {
        let conn = setup_test_db().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let (track_id, path) = create_track_with_file(
            &conn,
            temp.path(),
            "track-guid",
            "track.mp3",
            b"not really an mp3",
        )
        .unwrap();
        let client = musicindex_client("MusicIndex");

        let result = repair_payment_routes_for_track_with_client(
            &conn,
            temp.path(),
            track_id,
            &client,
            RecheckUpstream::Ask,
        );

        assert_eq!(result.status, PaymentRouteRepairStatus::Repaired);
        assert_eq!(client.calls().len(), 1, "{:?}", client.calls());
        let stored = db::payment_routes::stored_route(&conn, track_id)
            .unwrap()
            .unwrap();
        assert_eq!(stored[0].recipient_name.as_deref(), Some("MusicIndex"));
        assert_eq!(file_route_names(&path), vec![Some("MusicIndex".to_owned())]);
    }

    /// A stored route with no recipient is a known answer: RSS has no route.
    /// The repair writes nothing and asks nobody.
    #[test]
    fn adr_0076_route_readiness_repair_of_empty_stored_route_asks_nobody() {
        let conn = setup_test_db().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let (track_id, path) = create_track_with_file(
            &conn,
            temp.path(),
            "track-guid",
            "track.mp3",
            b"not really an mp3",
        )
        .unwrap();
        set_stored_route(&conn, track_id, &[]);
        let client = musicindex_client("MusicIndex");

        let result = repair_payment_routes_for_track_with_client(
            &conn,
            temp.path(),
            track_id,
            &client,
            RecheckUpstream::Ask,
        );

        assert_eq!(result.status, PaymentRouteRepairStatus::NoRoutesUpstream);
        assert_eq!(
            result.reason.as_deref(),
            Some(crate::application::queries::broadcast::NO_RSS_RECIPIENTS_REASON)
        );
        assert!(client.calls().is_empty());
        assert!(!audio_tags_have_ready_value_routes(
            &read_audio_tags(&path).unwrap()
        ));
    }
}
