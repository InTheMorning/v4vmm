//! Feed command family.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::application::command_bus::{ApplicationCommand, CommandOutcome, CommandResult};
use crate::application::command_context::CommandContext;
use crate::application::commands::payment_routes::{
    PaymentRouteRepairBatchResult, RepairMissingPaymentRouteTags,
};
use crate::application::errors::command::{
    attach_observation_receipts, observation_storage_failure, CommandError,
};
use crate::application::events::download::DownloadEvent;
use crate::application::events::feed::FeedEvent;
use crate::application::events::library::LibraryEvent;
use crate::application::events::metadata::MetadataEvent;
use crate::application::events::ApplicationEvent;
use crate::application::ports::download_manager::{DownloadError, DownloadManager};
use crate::provider_observation::{ObservationReceipt, ProviderObservationRecorder};
use crate::subscribe_service::{SubscribeFeedOutcome, SubscribeFeedRequest};
use crate::{db, feed_service};
use feed_service::StaleFeed;

type SharedConnection = Arc<Mutex<Connection>>;

/// Resolves the music directory that each feed update reads.
type MusicDirSource<'a> = &'a dyn Fn() -> anyhow::Result<PathBuf>;

/// Command result for checking one feed for remote updates.
#[derive(Clone, Debug)]
pub struct CheckFeedStalenessResult {
    feed_id: i64,
    stale: Option<StaleFeed>,
    /// ADR 0075 keeps committed receipts outside the business value.
    observation_receipts: Vec<ObservationReceipt>,
}

impl CheckFeedStalenessResult {
    /// Creates a feed staleness check result.
    #[must_use]
    pub fn new(feed_id: i64, stale: Option<StaleFeed>) -> Self {
        Self {
            feed_id,
            stale,
            observation_receipts: Vec::new(),
        }
    }

    /// Returns the receipts that this check committed.
    pub fn observation_receipts(&self) -> &[ObservationReceipt] {
        &self.observation_receipts
    }

    /// Returns the checked feed id.
    #[must_use]
    pub const fn feed_id(&self) -> i64 {
        self.feed_id
    }

    /// Returns the stale feed entry, if one is available.
    #[must_use]
    pub const fn stale(&self) -> Option<&StaleFeed> {
        self.stale.as_ref()
    }

    /// Consumes the result and returns the stale feed entry.
    #[must_use]
    pub fn into_stale(self) -> Option<StaleFeed> {
        self.stale
    }
}

/// Checks one feed for remote updates.
#[derive(Clone, Debug)]
pub struct CheckFeedStaleness {
    conn: SharedConnection,
    musicindex_endpoint: crate::config::MusicIndexEndpoint,
    feed_id: i64,
}

impl CheckFeedStaleness {
    /// Creates a single-feed staleness check command.
    #[must_use]
    pub fn new(
        conn: SharedConnection,
        musicindex_endpoint: impl Into<crate::config::MusicIndexEndpoint>,
        feed_id: i64,
    ) -> Self {
        Self {
            conn,
            musicindex_endpoint: musicindex_endpoint.into(),
            feed_id,
        }
    }
}

impl ApplicationCommand for CheckFeedStaleness {
    type Output = CheckFeedStalenessResult;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        let recorder = Arc::new(ProviderObservationRecorder::new(Arc::clone(&self.conn)));
        let result = (|| {
            if context.cancellation().is_cancelled() {
                return Err(CommandError::Cancelled);
            }
            let stale = feed_service::check_feed_staleness(
                &self.conn,
                &self.musicindex_endpoint,
                self.feed_id,
                &recorder,
            )
            .map_err(|error| feed_command_error(&error))?;
            Ok(CheckFeedStalenessResult::new(self.feed_id, stale))
        })();
        let result = assemble_observed_feed_command(&recorder, result, |value, receipts| {
            value.observation_receipts.extend(receipts);
        })?;
        Ok(CommandOutcome::without_events(result))
    }
}

/// Command result for applying remote feed updates to local tracks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplyFeedUpdatesResult {
    tracks_updated: usize,
    edits_written: usize,
    id3_errors: Vec<String>,
    feed_errors: Vec<String>,
    message: String,
    /// ADR 0075 keeps committed receipts outside the business value.
    observation_receipts: Vec<ObservationReceipt>,
}

impl ApplyFeedUpdatesResult {
    /// Creates a feed-update result.
    #[must_use]
    pub fn new(
        tracks_updated: usize,
        edits_written: usize,
        id3_errors: Vec<String>,
        feed_errors: Vec<String>,
    ) -> Self {
        let message = feed_apply_message(tracks_updated, edits_written, &id3_errors, &feed_errors);
        Self {
            tracks_updated,
            edits_written,
            id3_errors,
            feed_errors,
            message,
            observation_receipts: Vec::new(),
        }
    }

    /// Returns the receipts that this update committed.
    pub fn observation_receipts(&self) -> &[ObservationReceipt] {
        &self.observation_receipts
    }

    /// Returns how many tracks had tag edits written.
    #[must_use]
    pub const fn tracks_updated(&self) -> usize {
        self.tracks_updated
    }

    /// Returns how many ID3 edits were written.
    #[must_use]
    pub const fn edits_written(&self) -> usize {
        self.edits_written
    }

    /// Returns ID3 write error messages.
    #[must_use]
    pub fn id3_errors(&self) -> &[String] {
        &self.id3_errors
    }

    /// Returns feed update error messages.
    #[must_use]
    pub fn feed_errors(&self) -> &[String] {
        &self.feed_errors
    }

    /// Returns the user-facing status message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Applies remote feed updates to local downloaded tracks.
#[derive(Clone, Debug)]
pub struct ApplyFeedUpdates {
    conn: SharedConnection,
    musicindex_endpoint: crate::config::MusicIndexEndpoint,
    stale: Vec<StaleFeed>,
}

impl ApplyFeedUpdates {
    /// Creates a feed-update apply command.
    #[must_use]
    pub fn new(
        conn: SharedConnection,
        musicindex_endpoint: impl Into<crate::config::MusicIndexEndpoint>,
        stale: Vec<StaleFeed>,
    ) -> Self {
        Self {
            conn,
            musicindex_endpoint: musicindex_endpoint.into(),
            stale,
        }
    }
}

impl ApplicationCommand for ApplyFeedUpdates {
    type Output = ApplyFeedUpdatesResult;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        let recorder = Arc::new(ProviderObservationRecorder::new(Arc::clone(&self.conn)));
        let result = apply_stale_feed_updates(
            &self.conn,
            &self.musicindex_endpoint,
            &self.stale,
            &recorder,
            context,
        );
        let result = assemble_observed_feed_command(&recorder, result, |value, receipts| {
            value.observation_receipts.extend(receipts);
        })?;
        Ok(CommandOutcome::new(result, feed_update_events()))
    }
}

/// ADR 0075 drains the recorder once, after the complete result is known.
fn assemble_observed_feed_command<T>(
    recorder: &ProviderObservationRecorder,
    result: Result<T, CommandError>,
    attach: impl FnOnce(&mut T, Vec<ObservationReceipt>),
) -> Result<T, CommandError> {
    let receipts = recorder.take_receipts();
    match result {
        Ok(mut value) => {
            attach(&mut value, receipts);
            Ok(value)
        }
        Err(error) => Err(attach_observation_receipts(error, receipts)),
    }
}

/// ADR 0075 shares one observed feed-check loop with both check roots.
///
/// Ordinary check failures stay skipped. A provider storage failure stops the
/// loop with its typed capsule.
fn check_feed_batch_for_updates(
    conn: &SharedConnection,
    musicindex_endpoint: &crate::config::MusicIndexEndpoint,
    feeds: &[db::FeedStaleCheckRow],
    recorder: &Arc<ProviderObservationRecorder>,
    context: &CommandContext,
) -> Result<Vec<StaleFeed>, CommandError> {
    let mut stale = Vec::new();
    for feed in feeds {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        match feed_service::check_feed_staleness(conn, musicindex_endpoint, feed.id, recorder) {
            Ok(Some(entry)) => stale.push(entry),
            Ok(None) => {}
            Err(error) => {
                if let Some(failure) = observation_storage_failure(&error) {
                    return Err(CommandError::ObservationWriteFailure(Arc::new(failure)));
                }
            }
        }
    }
    Ok(stale)
}

/// ADR 0075 shares one observed update loop with both update roots.
fn apply_stale_feed_updates(
    conn: &SharedConnection,
    musicindex_endpoint: &crate::config::MusicIndexEndpoint,
    stale: &[StaleFeed],
    recorder: &Arc<ProviderObservationRecorder>,
    context: &CommandContext,
) -> Result<ApplyFeedUpdatesResult, CommandError> {
    apply_stale_feed_updates_from(
        conn,
        musicindex_endpoint,
        stale,
        recorder,
        context,
        &feed_service::configured_music_dir,
    )
}

/// The music directory source keeps the existing configuration read for each feed.
///
/// Ordinary update failures keep their per-feed message. A provider storage
/// failure stops the loop with its typed capsule.
fn apply_stale_feed_updates_from(
    conn: &SharedConnection,
    musicindex_endpoint: &crate::config::MusicIndexEndpoint,
    stale: &[StaleFeed],
    recorder: &Arc<ProviderObservationRecorder>,
    context: &CommandContext,
    music_dir: MusicDirSource<'_>,
) -> Result<ApplyFeedUpdatesResult, CommandError> {
    let mut total_tracks = 0usize;
    let mut total_edits = 0usize;
    let mut id3_errors = Vec::new();
    let mut feed_errors = Vec::new();
    for entry in stale {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let applied = music_dir().and_then(|music_dir| {
            feed_service::apply_feed_updates(conn, musicindex_endpoint, entry, &music_dir, recorder)
        });
        match applied {
            Ok(outcome) => {
                total_tracks += outcome.tracks_updated;
                total_edits += outcome.edits_written;
                id3_errors.extend(outcome.id3_errors);
            }
            Err(error) => {
                if let Some(failure) = observation_storage_failure(&error) {
                    return Err(CommandError::ObservationWriteFailure(Arc::new(failure)));
                }
                let label = entry
                    .title
                    .clone()
                    .unwrap_or_else(|| entry.feed_guid.clone());
                feed_errors.push(format!("{label}: {error:#}"));
            }
        }
    }
    Ok(ApplyFeedUpdatesResult::new(
        total_tracks,
        total_edits,
        id3_errors,
        feed_errors,
    ))
}

/// Result for checking feeds, applying updates, and repairing route tags.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CheckFeedsAndRepairRoutesResult {
    feeds_checked: usize,
    stale_feed_count: usize,
    feed_updates: Option<ApplyFeedUpdatesResult>,
    route_repairs: PaymentRouteRepairBatchResult,
    /// ADR 0075 keeps committed feed receipts outside the business value.
    observation_receipts: Vec<ObservationReceipt>,
}

impl CheckFeedsAndRepairRoutesResult {
    fn new(
        feeds_checked: usize,
        stale_feed_count: usize,
        feed_updates: Option<ApplyFeedUpdatesResult>,
        route_repairs: PaymentRouteRepairBatchResult,
    ) -> Self {
        Self {
            feeds_checked,
            stale_feed_count,
            feed_updates,
            route_repairs,
            observation_receipts: Vec::new(),
        }
    }

    /// Returns the feed receipts that this combined command committed.
    ///
    /// Packet 044 owns route-repair request retention, so these receipts cover
    /// the feed check and the feed updates only.
    pub(crate) fn observation_receipts(&self) -> &[ObservationReceipt] {
        &self.observation_receipts
    }

    /// Returns the number of subscribed feeds checked.
    #[must_use]
    pub(crate) const fn feeds_checked(&self) -> usize {
        self.feeds_checked
    }

    /// Returns the number of feeds with applied updates.
    #[must_use]
    pub(crate) const fn stale_feed_count(&self) -> usize {
        self.stale_feed_count
    }

    /// Returns feed-update results when stale feeds were applied.
    #[must_use]
    pub(crate) const fn feed_updates(&self) -> Option<&ApplyFeedUpdatesResult> {
        self.feed_updates.as_ref()
    }

    /// Returns the route-repair batch result.
    #[must_use]
    pub(crate) const fn route_repairs(&self) -> &PaymentRouteRepairBatchResult {
        &self.route_repairs
    }
}

/// Checks all feeds, applies stale updates, then repairs route tags.
#[derive(Clone, Debug)]
pub(crate) struct CheckFeedsAndRepairRoutes {
    conn: SharedConnection,
    musicindex_endpoint: crate::config::MusicIndexEndpoint,
    music_dir: PathBuf,
    feeds: Vec<db::FeedStaleCheckRow>,
}

impl CheckFeedsAndRepairRoutes {
    /// Creates the combined `Check all feeds` command for ADR 0065.
    #[must_use]
    pub(crate) fn new(
        conn: SharedConnection,
        musicindex_endpoint: impl Into<crate::config::MusicIndexEndpoint>,
        music_dir: PathBuf,
        feeds: Vec<db::FeedStaleCheckRow>,
    ) -> Self {
        Self {
            conn,
            musicindex_endpoint: musicindex_endpoint.into(),
            music_dir,
            feeds,
        }
    }
}

impl ApplicationCommand for CheckFeedsAndRepairRoutes {
    type Output = CheckFeedsAndRepairRoutesResult;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        let feeds_checked = self.feeds.len();
        let recorder = Arc::new(ProviderObservationRecorder::new(Arc::clone(&self.conn)));
        let mut events = Vec::new();
        let result = (|| {
            let stale = check_feed_batch_for_updates(
                &self.conn,
                &self.musicindex_endpoint,
                &self.feeds,
                &recorder,
                context,
            )?;
            let stale_feed_count = stale.len();

            let feed_updates = if stale.is_empty() {
                None
            } else {
                let applied = apply_stale_feed_updates(
                    &self.conn,
                    &self.musicindex_endpoint,
                    &stale,
                    &recorder,
                    context,
                )?;
                events.extend(feed_update_events());
                Some(applied)
            };

            let repair = RepairMissingPaymentRouteTags::new(
                Arc::clone(&self.conn),
                self.musicindex_endpoint.clone(),
                self.music_dir.clone(),
            )
            .execute(context)?;
            let (route_repairs, repair_events) = repair.into_parts();
            events.extend(repair_events);

            Ok(CheckFeedsAndRepairRoutesResult::new(
                feeds_checked,
                stale_feed_count,
                feed_updates,
                route_repairs,
            ))
        })();
        let result = assemble_observed_feed_command(&recorder, result, |value, receipts| {
            value.observation_receipts.extend(receipts);
        })?;
        Ok(CommandOutcome::new(result, events))
    }
}

/// Command result for subscribing/downloading a feed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscribeFeedResult {
    downloaded: usize,
    applied_edits: usize,
    skipped: usize,
    message: String,
}

impl SubscribeFeedResult {
    /// Creates a feed subscription result from the service outcome.
    #[must_use]
    pub fn from_outcome(outcome: &SubscribeFeedOutcome) -> Self {
        let message = if outcome.skipped == 0 {
            format!(
                "Downloaded feed; downloaded {} track{}, applied {} ID3 edit{}",
                outcome.downloaded,
                plural(outcome.downloaded),
                outcome.applied_edits,
                plural(outcome.applied_edits)
            )
        } else {
            format!(
                "Downloaded feed; downloaded {} track{}, applied {} ID3 edit{}, skipped {}",
                outcome.downloaded,
                plural(outcome.downloaded),
                outcome.applied_edits,
                plural(outcome.applied_edits),
                outcome.skipped
            )
        };
        Self {
            downloaded: outcome.downloaded,
            applied_edits: outcome.applied_edits,
            skipped: outcome.skipped,
            message,
        }
    }

    /// Returns how many tracks were downloaded.
    #[must_use]
    pub const fn downloaded(&self) -> usize {
        self.downloaded
    }

    /// Returns how many ID3 edits were applied.
    #[must_use]
    pub const fn applied_edits(&self) -> usize {
        self.applied_edits
    }

    /// Returns how many tracks were skipped.
    #[must_use]
    pub const fn skipped(&self) -> usize {
        self.skipped
    }

    /// Returns the user-facing completion message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Subscribes/downloads one feed.
pub struct SubscribeFeed {
    conn: SharedConnection,
    download_manager: Arc<dyn DownloadManager>,
    request: SubscribeFeedRequest,
}

impl SubscribeFeed {
    /// Creates a feed subscription command.
    #[must_use]
    pub fn new(
        conn: SharedConnection,
        download_manager: Arc<dyn DownloadManager>,
        request: SubscribeFeedRequest,
    ) -> Self {
        Self {
            conn,
            download_manager,
            request,
        }
    }
}

impl ApplicationCommand for SubscribeFeed {
    type Output = SubscribeFeedResult;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        let outcome = self
            .download_manager
            .subscribe_feed(self.conn, self.request, context)
            .map_err(feed_download_error)?;
        Ok(CommandOutcome::new(
            SubscribeFeedResult::from_outcome(&outcome),
            feed_download_changed_events(),
        ))
    }
}

/// Command result for removing a feed subscription.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnsubscribeFeedResult {
    message: String,
}

impl UnsubscribeFeedResult {
    /// Creates an unsubscribe result.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Returns the user-facing completion message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Removes a local feed subscription by local feed id.
#[derive(Clone, Debug)]
pub struct UnsubscribeFeedById {
    conn: SharedConnection,
    feed_id: i64,
}

impl UnsubscribeFeedById {
    /// Creates a feed unsubscribe command.
    #[must_use]
    pub const fn new(conn: SharedConnection, feed_id: i64) -> Self {
        Self { conn, feed_id }
    }
}

impl ApplicationCommand for UnsubscribeFeedById {
    type Output = UnsubscribeFeedResult;

    fn execute(self, _context: &CommandContext) -> CommandResult<Self::Output> {
        let conn = self.conn.lock().map_err(|_| feed_lock_error())?;
        db::set_feed_subscribed(&conn, self.feed_id, false)
            .map_err(|error| feed_command_error(&error))?;
        db::unsubscribe_feed_tracks(&conn, self.feed_id)
            .map_err(|error| feed_command_error(&error))?;
        Ok(CommandOutcome::new(
            UnsubscribeFeedResult::new("Removed feed"),
            feed_changed_events(),
        ))
    }
}

/// Removes a local feed subscription by feed URL.
#[derive(Clone, Debug)]
pub struct UnsubscribeFeedByUrl {
    conn: SharedConnection,
    feed_url: Option<String>,
}

impl UnsubscribeFeedByUrl {
    /// Creates a feed unsubscribe command.
    #[must_use]
    pub fn new(conn: SharedConnection, feed_url: Option<String>) -> Self {
        Self { conn, feed_url }
    }
}

impl ApplicationCommand for UnsubscribeFeedByUrl {
    type Output = UnsubscribeFeedResult;

    fn execute(self, _context: &CommandContext) -> CommandResult<Self::Output> {
        let feed_url = self
            .feed_url
            .ok_or_else(|| CommandError::Feed("feed has no RSS URL".to_string()))?;
        let conn = self.conn.lock().map_err(|_| feed_lock_error())?;
        db::set_feed_subscribed_by_url(&conn, &feed_url, false)
            .map_err(|error| feed_command_error(&error))?;
        Ok(CommandOutcome::new(
            UnsubscribeFeedResult::new("Removed feed"),
            feed_changed_events(),
        ))
    }
}

fn feed_changed_events() -> Vec<ApplicationEvent> {
    vec![
        ApplicationEvent::Feed(FeedEvent::Changed),
        ApplicationEvent::Library(LibraryEvent::Changed),
    ]
}

fn feed_download_changed_events() -> Vec<ApplicationEvent> {
    vec![
        ApplicationEvent::Feed(FeedEvent::Changed),
        ApplicationEvent::Download(DownloadEvent::Changed),
        ApplicationEvent::Library(LibraryEvent::Changed),
    ]
}

fn feed_update_events() -> Vec<ApplicationEvent> {
    vec![
        ApplicationEvent::Feed(FeedEvent::Changed),
        ApplicationEvent::Library(LibraryEvent::Changed),
        ApplicationEvent::Metadata(MetadataEvent::Changed),
    ]
}

fn feed_lock_error() -> CommandError {
    CommandError::Feed("database lock poisoned".to_string())
}

/// ADR 0075 classifies a provider storage failure before the feed family.
fn feed_command_error(error: &anyhow::Error) -> CommandError {
    if let Some(failure) = observation_storage_failure(error) {
        return CommandError::ObservationWriteFailure(Arc::new(failure));
    }
    CommandError::Feed(format!("{error:#}"))
}

fn feed_download_error(error: DownloadError) -> CommandError {
    match error {
        DownloadError::Cancelled => CommandError::Cancelled,
        DownloadError::Failed(message) => CommandError::Feed(message),
    }
}

fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

fn feed_apply_message(
    tracks_updated: usize,
    edits_written: usize,
    id3_errors: &[String],
    feed_errors: &[String],
) -> String {
    let mut parts = Vec::new();
    parts.push(if tracks_updated == 0 {
        "No edits written".into()
    } else {
        format!("Applied {edits_written} edit(s) to {tracks_updated} track(s)")
    });
    if !id3_errors.is_empty() {
        parts.push(format!(
            "Tag write errors ({}): {}",
            id3_errors.len(),
            id3_errors.join("; ")
        ));
    }
    if !feed_errors.is_empty() {
        parts.push(format!(
            "Feed errors ({}): {}",
            feed_errors.len(),
            feed_errors.join("; ")
        ));
    }
    parts.join(" — ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::application::command_bus::CommandBus;
    use crate::application::command_context::CommandContext;
    use crate::application::ports::download_manager::{DownloadOutcome, DownloadRequest};
    use crate::library_service::AppendToPlaylistOutcome;
    use crate::subscribe_service::SubscribeTrackOutcome;

    fn setup_test_db() -> anyhow::Result<SharedConnection> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        db::init_schema(&conn)?;
        db::migrate_schema(&conn)?;
        Ok(Arc::new(Mutex::new(conn)))
    }

    fn create_feed(conn: &Connection, feed_url: &str) -> anyhow::Result<i64> {
        conn.execute(
            "INSERT INTO feeds (feed_url, feed_guid, title, is_subscribed)
             VALUES (?1, ?2, ?3, 1)",
            rusqlite::params![feed_url, "feed-guid", "Feed Title"],
        )?;
        Ok(conn.last_insert_rowid())
    }

    fn create_library_track(conn: &Connection, feed_id: i64) -> anyhow::Result<i64> {
        conn.execute(
            "INSERT INTO tracks (feed_id, item_guid, track_title, is_in_library)
             VALUES (?1, ?2, ?3, 1)",
            rusqlite::params![feed_id, "item-guid", "Track Title"],
        )?;
        Ok(conn.last_insert_rowid())
    }

    #[test]
    fn check_feed_staleness_missing_feed_returns_none() -> anyhow::Result<()> {
        let conn = setup_test_db()?;

        let outcome = CommandBus::new().execute(
            CheckFeedStaleness::new(Arc::clone(&conn), "https://api.example.test", i64::MAX),
            &CommandContext::next(),
        )?;

        assert_eq!(outcome.value().feed_id(), i64::MAX);
        assert!(outcome.value().stale().is_none());
        assert!(outcome.events().is_empty());

        Ok(())
    }

    #[test]
    fn check_feed_batch_empty_input_returns_no_stale_entries() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let recorder = Arc::new(ProviderObservationRecorder::new(Arc::clone(&conn)));

        let stale = check_feed_batch_for_updates(
            &conn,
            &"https://api.example.test".into(),
            &[],
            &recorder,
            &CommandContext::next(),
        )
        .expect("empty batch succeeds");

        assert!(stale.is_empty());
        assert!(recorder.take_receipts().is_empty());

        Ok(())
    }

    #[test]
    fn check_feeds_and_repair_routes_result_exposes_counts() {
        let feed_updates = ApplyFeedUpdatesResult::new(1, 2, vec!["tag failed".into()], Vec::new());
        let route_repairs = PaymentRouteRepairBatchResult {
            summary: crate::application::commands::payment_routes::PaymentRouteRepairSummary {
                repaired: 2,
                no_routes_upstream: 1,
                failed: 0,
                skipped: 3,
            },
            tracks: Vec::new(),
        };

        let result = CheckFeedsAndRepairRoutesResult::new(4, 1, Some(feed_updates), route_repairs);

        assert_eq!(result.feeds_checked(), 4);
        assert_eq!(result.stale_feed_count(), 1);
        assert_eq!(
            result
                .feed_updates()
                .map(|updates| updates.id3_errors().len()),
            Some(1)
        );
        assert_eq!(result.route_repairs().summary.repaired, 2);
        assert_eq!(result.route_repairs().summary.no_routes_upstream, 1);
    }

    #[test]
    fn apply_feed_updates_empty_input_emits_feed_metadata_events() -> anyhow::Result<()> {
        let conn = setup_test_db()?;

        let outcome = CommandBus::new().execute(
            ApplyFeedUpdates::new(Arc::clone(&conn), "https://api.example.test", Vec::new()),
            &CommandContext::next(),
        )?;

        assert_eq!(outcome.value().tracks_updated(), 0);
        assert_eq!(outcome.value().edits_written(), 0);
        assert!(outcome.value().id3_errors().is_empty());
        assert!(outcome.value().feed_errors().is_empty());
        assert_eq!(outcome.value().message(), "No edits written");
        assert_eq!(outcome.events(), feed_update_events());

        Ok(())
    }

    #[derive(Debug)]
    struct FakeDownloadManager;

    impl DownloadManager for FakeDownloadManager {
        fn download(
            &self,
            _request: DownloadRequest,
            _context: &CommandContext,
        ) -> Result<DownloadOutcome, DownloadError> {
            Ok(DownloadOutcome::new(PathBuf::from("/tmp/fake.mp3")))
        }

        fn subscribe_track(
            &self,
            _conn: SharedConnection,
            _request: crate::subscribe_service::SubscribeTrackRequest,
            _context: &CommandContext,
        ) -> Result<SubscribeTrackOutcome, DownloadError> {
            Err(DownloadError::Failed("not used".to_string()))
        }

        fn subscribe_feed(
            &self,
            _conn: SharedConnection,
            _request: SubscribeFeedRequest,
            _context: &CommandContext,
        ) -> Result<SubscribeFeedOutcome, DownloadError> {
            Ok(SubscribeFeedOutcome {
                downloaded: 2,
                applied_edits: 3,
                skipped: 1,
            })
        }

        fn subscribe_then_append_to_playlist(
            &self,
            _conn: SharedConnection,
            _playlist_id: i64,
            _track_ids: Vec<i64>,
            _context: &CommandContext,
        ) -> Result<AppendToPlaylistOutcome, DownloadError> {
            Err(DownloadError::Failed("not used".to_string()))
        }
    }

    #[test]
    fn subscribe_feed_uses_download_manager_port_and_emits_events() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let request = SubscribeFeedRequest {
            feed: crate::api::Feed::default(),
            musicindex_endpoint: "https://api.example.test".into(),
        };

        let outcome = CommandBus::new().execute(
            SubscribeFeed::new(Arc::clone(&conn), Arc::new(FakeDownloadManager), request),
            &CommandContext::next(),
        )?;

        assert_eq!(outcome.value().downloaded(), 2);
        assert_eq!(outcome.value().applied_edits(), 3);
        assert_eq!(outcome.value().skipped(), 1);
        assert_eq!(
            outcome.value().message(),
            "Downloaded feed; downloaded 2 tracks, applied 3 ID3 edits, skipped 1"
        );
        assert_eq!(outcome.events(), feed_download_changed_events());

        Ok(())
    }

    #[test]
    fn unsubscribe_feed_by_id_removes_feed_and_track_library_state() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let (feed_id, track_id) = {
            let db = conn.lock().expect("lock test db");
            let feed_id = create_feed(&db, "https://example.test/feed.xml")?;
            let track_id = create_library_track(&db, feed_id)?;
            (feed_id, track_id)
        };

        let outcome = CommandBus::new().execute(
            UnsubscribeFeedById::new(Arc::clone(&conn), feed_id),
            &CommandContext::next(),
        )?;

        assert_eq!(outcome.value().message(), "Removed feed");
        assert_eq!(outcome.events(), feed_changed_events());
        let db = conn.lock().expect("lock test db");
        assert!(!db::feed_is_subscribed_by_url(
            &db,
            "https://example.test/feed.xml"
        )?);
        let track = db::track_row_by_id(&db, track_id)?.expect("track exists");
        assert!(!track.is_in_library);

        Ok(())
    }

    #[test]
    fn unsubscribe_feed_by_url_requires_url() {
        let conn = setup_test_db().expect("test db");

        let error = CommandBus::new()
            .execute(
                UnsubscribeFeedByUrl::new(Arc::clone(&conn), None),
                &CommandContext::next(),
            )
            .expect_err("missing feed URL should fail");

        assert!(
            error.to_string().contains("feed has no RSS URL"),
            "unexpected error: {error}"
        );
    }
}

#[cfg(test)]
mod observation_tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::Duration;

    use crate::application::command_context::{
        CancellationToken, CommandContext, OperationId, TraceId,
    };
    use crate::provider_observation::ObservationOutcome;

    const UPDATE_INCLUDE: &str =
        "?include=source_links%2Csource_ids%2Csource_release_claims%2Csource_contributors%2Cpayment_routes";

    /// Scripted MusicIndex and RSS responses for one disposable database.
    struct Fixture {
        endpoint: crate::config::MusicIndexEndpoint,
        base: String,
        conn: SharedConnection,
        mode: Arc<AtomicUsize>,
        requests: Arc<Mutex<Vec<String>>>,
        commits: Arc<AtomicUsize>,
        cancel_after: Arc<AtomicUsize>,
        token: CancellationToken,
        stop: Arc<AtomicBool>,
        worker: Option<std::thread::JoinHandle<()>>,
    }

    impl Fixture {
        fn start() -> Self {
            let conn = Connection::open_in_memory().unwrap();
            db::upgrades::create_fixture(&conn, 12).unwrap();
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap().to_string();
            let base = format!("http://{address}");
            conn.execute(
                "INSERT INTO feeds(feed_url,feed_guid,title,is_subscribed,musicindex_updated_at)
                 VALUES(?1,'f1','Local feed',1,100)",
                [format!("{base}/feed.xml")],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO tracks(feed_id,item_guid,track_title,is_in_library) VALUES(1,'t1','Local title',1)",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO local_files(path,track_id) VALUES('track.mp3',1)",
                [],
            )
            .unwrap();
            let commits = Arc::new(AtomicUsize::new(0));
            let counted = Arc::clone(&commits);
            conn.commit_hook(Some(move || {
                counted.fetch_add(1, Ordering::SeqCst);
                false
            }))
            .unwrap();
            let conn = Arc::new(Mutex::new(conn));

            let mode = Arc::new(AtomicUsize::new(0));
            let requests = Arc::new(Mutex::new(Vec::new()));
            let cancel_after = Arc::new(AtomicUsize::new(0));
            let token = CancellationToken::new();
            let stop = Arc::new(AtomicBool::new(false));
            let db = Arc::clone(&conn);
            let selected = Arc::clone(&mode);
            let received = Arc::clone(&requests);
            let cancel_at = Arc::clone(&cancel_after);
            let cancellation = token.clone();
            let stopped = Arc::clone(&stop);
            let url = base.clone();
            let worker = std::thread::spawn(move || {
                while !stopped.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            stream
                                .set_read_timeout(Some(Duration::from_secs(2)))
                                .unwrap();
                            let mut bytes = Vec::new();
                            let mut buffer = [0; 4096];
                            while !bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                                match stream.read(&mut buffer) {
                                    Ok(0) | Err(_) => break,
                                    Ok(read) => bytes.extend_from_slice(&buffer[..read]),
                                }
                            }
                            let request = String::from_utf8_lossy(&bytes);
                            let Some(path) = request
                                .lines()
                                .next()
                                .and_then(|line| line.split_whitespace().nth(1))
                            else {
                                continue;
                            };
                            let count = {
                                let mut received = received.lock().unwrap();
                                received.push(path.into());
                                received.len()
                            };
                            let mode = selected.load(Ordering::SeqCst);
                            // Packet 044 owns route-repair request retention. These
                            // requests allocate no generation. The repair holds the
                            // database lock during its HTTP work. This fixture must not
                            // use the database for these requests.
                            // The converted profiles also request payment routes. Only
                            // the repair asks for that include alone.
                            let route_repair = path.ends_with("?include=payment_routes");
                            if !route_repair {
                                let database = db.lock().unwrap();
                                let pending: i64 = database.query_row(
                                    "SELECT count(*) FROM metadata_request_slots WHERE state='pending' AND (resource_id IN (SELECT id FROM metadata_resources WHERE request_uri=?1))",
                                    [format!("{url}{path}")],
                                    |row| row.get(0),
                                ).unwrap();
                                assert_eq!(pending, 1, "generation must commit before HTTP");
                            }
                            let cancel_at = cancel_at.load(Ordering::SeqCst);
                            if cancel_at > 0 && count >= cancel_at {
                                cancellation.cancel();
                            }
                            if !route_repair && mode == 2 && count == 1 {
                                db.lock().unwrap().execute_batch("CREATE TEMP TRIGGER reject_next_allocation BEFORE UPDATE ON metadata_generation BEGIN SELECT RAISE(ABORT,'fixture later allocation'); END").unwrap();
                            }
                            if !route_repair && (mode == 3 || (mode == 6 && count == 2)) {
                                db.lock().unwrap().execute_batch("CREATE TEMP TRIGGER reject_observation AFTER INSERT ON metadata_observations BEGIN SELECT RAISE(ABORT,'fixture response rejection'); END").unwrap();
                            }
                            if mode == 7 && path == "/feed.xml" {
                                db.lock().unwrap().execute_batch("CREATE TEMP TRIGGER reject_rss_observation AFTER INSERT ON metadata_observations BEGIN SELECT RAISE(ABORT,'fixture RSS rejection'); END").unwrap();
                            }
                            let (status, body) = response(&url, path, mode);
                            write!(
                                stream,
                                "HTTP/1.1 {status}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                                body.len()
                            )
                            .unwrap();
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(2));
                        }
                        Err(error) => panic!("fixture listener: {error}"),
                    }
                }
            });
            Self {
                endpoint: base.clone().into(),
                base,
                conn,
                mode,
                requests,
                commits,
                cancel_after,
                token,
                stop,
                worker: Some(worker),
            }
        }

        fn context(&self) -> CommandContext {
            CommandContext::new(OperationId::new(1), self.token.clone(), TraceId::new(1))
        }

        fn feed_rows(&self) -> Vec<db::FeedStaleCheckRow> {
            let conn = self.conn.lock().unwrap();
            db::subscribed_feeds_for_stale_check(&conn).unwrap()
        }

        fn stale_entry(&self) -> StaleFeed {
            StaleFeed {
                feed_id: 1,
                feed_guid: "f1".into(),
                title: Some("Local feed".into()),
                new_updated_at: 200,
            }
        }

        fn check_one(&self) -> Result<CheckFeedStalenessResult, CommandError> {
            CheckFeedStaleness::new(Arc::clone(&self.conn), self.endpoint.clone(), 1)
                .execute(&self.context())
                .map(|outcome| outcome.into_parts().0)
        }

        fn apply(&self, music_dir: &Path) -> Result<ApplyFeedUpdatesResult, CommandError> {
            let recorder = Arc::new(ProviderObservationRecorder::new(Arc::clone(&self.conn)));
            let music_dir = music_dir.to_owned();
            let result = apply_stale_feed_updates_from(
                &self.conn,
                &self.endpoint,
                &[self.stale_entry()],
                &recorder,
                &self.context(),
                &move || Ok(music_dir.clone()),
            );
            assemble_observed_feed_command(&recorder, result, |value, receipts| {
                value.observation_receipts.extend(receipts);
            })
        }

        fn taken_requests(&self) -> Vec<String> {
            std::mem::take(&mut *self.requests.lock().unwrap())
        }

        fn scalar(&self, sql: &str) -> i64 {
            self.conn
                .lock()
                .unwrap()
                .query_row(sql, [], |row| row.get(0))
                .unwrap()
        }

        fn marker(&self) -> Option<i64> {
            self.conn
                .lock()
                .unwrap()
                .query_row(
                    "SELECT musicindex_updated_at FROM feeds WHERE id=1",
                    [],
                    |row| row.get(0),
                )
                .unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            let _ = std::net::TcpStream::connect(self.base.trim_start_matches("http://"));
            self.worker.take().unwrap().join().unwrap();
        }
    }

    fn response(base: &str, path: &str, mode: usize) -> (&'static str, String) {
        let query = path.split('?').nth(1).unwrap_or_default().to_owned();
        let path = path.split('?').next().unwrap();
        if path == "/feed.xml" {
            if mode == 8 {
                return ("503 Service Unavailable", "retained RSS failure".into());
            }
            return ("200 OK", "<rss xmlns:podcast=\"https://podcastindex.org/namespace/1.0\"><channel><title>RSS feed</title><podcast:guid>f1</podcast:guid><podcast:txt purpose=\"npub\">npub1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzqujme</podcast:txt><item><guid>t1</guid><title>RSS title</title><description>RSS description</description><podcast:txt purpose=\"other\">rejected-original</podcast:txt></item></channel></rss>".into());
        }
        if mode == 1 {
            return (
                "503 Service Unavailable",
                "{\"error\":\"retained failure\"}".into(),
            );
        }
        if mode == 4 {
            return ("200 OK", "{broken JSON".into());
        }
        if path.contains("/tracks/") {
            return (
                "200 OK",
                json!({"data":{"track_guid":"t1","feed_guid":"f1",
                    "feed_url":format!("{base}/feed.xml"),
                    "title":"Index title","track_artist":"Index artist",
                    "unknown_member":"retained"}})
                .to_string(),
            );
        }
        if query.is_empty() {
            let updated_at = if mode == 5 { 100 } else { 200 };
            return (
                "200 OK",
                json!({"data":{"feed_guid":"f1","updated_at":updated_at,"unknown_member":"retained"}})
                    .to_string(),
            );
        }
        (
            "200 OK",
            json!({"data":{"feed_guid":"f1","feed_url":format!("{base}/feed.xml"),
                "title":"Index feed","description":"Index description",
                "unknown_member":"retained",
                "source_links":[],"source_ids":[],"source_contributors":[]}})
            .to_string(),
        )
    }

    fn audio_directory() -> tempfile::TempDir {
        use id3::TagLike;
        let directory = tempfile::tempdir().unwrap();
        let mut file = std::fs::File::create(directory.path().join("track.mp3")).unwrap();
        let mut tag = id3::Tag::new();
        tag.set_title("Embedded title");
        tag.set_artist("Embedded artist");
        tag.write_to(&mut file, id3::Version::Id3v24).unwrap();
        directory
    }

    /// F39-01, F39-02, F39-15: durable generations, exact requests, shared order.
    #[test]
    fn adr_0075_feed_observation_check_roots_allocate_generations_before_requests() {
        let fixture = Fixture::start();

        let result = fixture.check_one().unwrap();
        assert_eq!(result.feed_id(), 1);
        assert_eq!(result.stale().map(|stale| stale.new_updated_at), Some(200));
        assert_eq!(result.observation_receipts().len(), 1);
        assert_eq!(fixture.taken_requests(), vec!["/v1/feeds/f1"]);
        let first_generation = result.observation_receipts()[0].generation;

        // The eligible-row filter ignores a missing feed and makes no request.
        let missing =
            CheckFeedStaleness::new(Arc::clone(&fixture.conn), fixture.endpoint.clone(), 99)
                .execute(&fixture.context())
                .unwrap()
                .into_parts()
                .0;
        assert!(missing.stale().is_none());
        assert!(missing.observation_receipts().is_empty());
        assert!(fixture.taken_requests().is_empty());

        // The batch root shares the same descriptor, so generations keep rising.
        let repeated = fixture.check_one().unwrap();
        assert_eq!(fixture.taken_requests(), vec!["/v1/feeds/f1"]);
        assert!(repeated.observation_receipts()[0].generation > first_generation);
        assert_eq!(
            fixture.scalar(
                "SELECT count(*) FROM metadata_resources WHERE request_uri LIKE '%/v1/feeds/f1'"
            ),
            1,
            "equal descriptors share one resource"
        );

        // A feed that is not newer than the stored marker reports no update.
        fixture.mode.store(5, Ordering::SeqCst);
        let current = fixture.check_one().unwrap();
        assert!(current.stale().is_none());
        assert_eq!(current.observation_receipts().len(), 1);
    }

    /// F39-02, F39-07, F39-14, F39-15: request order, files, markers and counts.
    #[test]
    fn adr_0075_feed_observation_update_preserves_requests_files_and_markers() {
        use id3::TagLike;
        let fixture = Fixture::start();
        let directory = audio_directory();
        let path = directory.path().join("track.mp3");
        let mutations = Arc::new(Mutex::new(BTreeMap::<String, usize>::new()));
        let observed = Arc::clone(&mutations);
        fixture
            .conn
            .lock()
            .unwrap()
            .update_hook(Some(
                move |_: rusqlite::hooks::Action, _: &str, table: &str, _: i64| {
                    *observed.lock().unwrap().entry(table.into()).or_default() += 1;
                },
            ))
            .unwrap();

        for repetition in [false, true] {
            mutations.lock().unwrap().clear();
            fixture.commits.store(0, Ordering::SeqCst);
            let result = fixture.apply(directory.path()).unwrap();
            assert!(
                result.feed_errors().is_empty(),
                "{:?}",
                result.feed_errors()
            );
            assert!(result.id3_errors().is_empty(), "{:?}", result.id3_errors());
            assert_eq!(
                fixture.taken_requests(),
                vec![
                    format!("/v1/feeds/f1{UPDATE_INCLUDE}"),
                    format!("/v1/feeds/f1/tracks/t1{UPDATE_INCLUDE}"),
                    format!("/v1/feeds/f1{UPDATE_INCLUDE}"),
                    "/feed.xml".to_owned(),
                ],
                "one feed request plus 2D detail requests and at most D RSS requests"
            );
            assert_eq!(result.observation_receipts().len(), 4);
            assert_eq!(fixture.marker(), Some(200));
            let mutations = mutations.lock().unwrap();
            let legacy: usize = mutations
                .iter()
                .filter(|(table, _)| !table.starts_with("metadata_"))
                .map(|(_, count)| count)
                .sum();
            let observation: usize = mutations
                .iter()
                .filter(|(table, _)| table.starts_with("metadata_"))
                .map(|(_, count)| count)
                .sum();
            let snapshots = fixture.scalar("SELECT count(*) FROM metadata_snapshots");
            println!(
                "ADR0075_FEED_UPDATE repetition={repetition} requests=4 legacy_rows={legacy} observation_rows={observation} snapshot_rows={snapshots} transactions={} tables={mutations:?}",
                fixture.commits.load(Ordering::SeqCst)
            );
            assert!(legacy > 0, "legacy persistence keeps its existing writes");
        }

        // Repeated equal responses reuse the retained body.
        assert!(
            fixture.scalar("SELECT count(*) FROM metadata_bodies")
                < fixture.scalar(
                    "SELECT sum(occurrence_count) FROM metadata_observations WHERE body_sha256 IS NOT NULL"
                ),
            "repeated equal responses reference one retained body"
        );
        assert!(
            fixture.scalar("SELECT max(occurrence_count) FROM metadata_observations") > 1,
            "a repeated response records another occurrence"
        );

        let tag = id3::Tag::read_from_path(&path).unwrap();
        // The explicit update writes the source values into the file. The feed view
        // below makes no file change. That difference is the existing behavior.
        assert_eq!(tag.title(), Some("Index title"));
        assert!(
            tag.frames().any(|frame| frame.id() == "TXXX"),
            "the explicit update writes its generated edits"
        );

        // Viewing a feed never writes tags.
        let before = std::fs::read(&path).unwrap();
        fixture.check_one().unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    /// F39-03: retained evidence survives a database reopen.
    #[test]
    fn adr_0075_feed_observation_retained_bodies_survive_a_database_reopen() {
        let fixture = Fixture::start();
        let directory = audio_directory();
        fixture.apply(directory.path()).unwrap();
        fixture.mode.store(4, Ordering::SeqCst);
        fixture.check_one().unwrap_err();
        fixture.mode.store(1, Ordering::SeqCst);
        fixture.check_one().unwrap_err();

        let target = tempfile::tempdir().unwrap();
        let path = target.path().join("retained.sqlite");
        {
            let mut copy = Connection::open(&path).unwrap();
            let source = fixture.conn.lock().unwrap();
            rusqlite::backup::Backup::new(&source, &mut copy)
                .unwrap()
                .run_to_completion(16, Duration::from_millis(1), None)
                .unwrap();
        }
        let reopened = Connection::open(path).unwrap();
        let bodies: Vec<String> = reopened
            .prepare("SELECT CAST(bytes AS TEXT) FROM metadata_bodies")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<std::result::Result<_, _>>()
            .unwrap();
        assert!(bodies.iter().any(|body| body.contains("unknown_member")));
        assert!(bodies.iter().any(|body| body.contains("{broken JSON")));
        assert!(bodies.iter().any(|body| body.contains("retained failure")));
        assert!(bodies.iter().any(|body| body.contains("rejected-original")));
        assert_eq!(
            reopened
                .query_row(
                    "SELECT count(*) FROM metadata_observations WHERE outcome='failed'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            2,
            "a malformed body and a failed response both stay retained"
        );
        // The RSS contract accepts only the checked identity collections.
        let collections: Vec<String> = reopened
            .prepare(
                "SELECT DISTINCT collection FROM metadata_coverage WHERE observation_id IN \
                 (SELECT id FROM metadata_observations WHERE provider_id IN \
                 (SELECT id FROM metadata_providers WHERE kind='rss'))",
            )
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<std::result::Result<_, _>>()
            .unwrap();
        let contracted: Vec<String> = reopened
            .prepare(
                "SELECT DISTINCT collection FROM metadata_coverage WHERE contract_id IS NOT NULL \
                 AND observation_id IN (SELECT id FROM metadata_observations WHERE provider_id IN \
                 (SELECT id FROM metadata_providers WHERE kind='rss'))",
            )
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<std::result::Result<_, _>>()
            .unwrap();
        assert_eq!(
            contracted,
            vec!["source_ids".to_owned()],
            "only the approved identity collection carries a contract"
        );
        assert!(
            collections.len() > contracted.len(),
            "RSS still retains its other observed evidence: {collections:?}"
        );
    }

    /// F39-04, F39-05, F39-10: storage failures stop dependent work and keep receipts.
    #[test]
    fn adr_0075_feed_observation_storage_failures_stop_dependent_work() {
        // Allocation failure prevents its own request.
        let fixture = Fixture::start();
        let directory = audio_directory();
        fixture.mode.store(2, Ordering::SeqCst);
        let error = fixture.apply(directory.path()).unwrap_err();
        let CommandError::ObservationWriteFailure(failure) = &error else {
            panic!("typed storage failure required, found {error:?}")
        };
        assert_eq!(
            failure.storage_error,
            Some(crate::provider_observation::ObservationStorageError::RequestAllocation)
        );
        assert_eq!(
            failure.receipts.len(),
            1,
            "the earlier receipt stays retained"
        );
        assert_eq!(
            fixture.taken_requests().len(),
            1,
            "the blocked request never ran"
        );
        assert!(error.to_string().contains("did not run"));
        assert!(!format!("{error:?}").contains(&fixture.base));
        assert_eq!(
            fixture.marker(),
            Some(100),
            "a stopped feed keeps its marker"
        );

        // Response-write failure stops legacy persistence and tag generation.
        let fixture = Fixture::start();
        let path = directory.path().join("track.mp3");
        let before = std::fs::read(&path).unwrap();
        fixture.mode.store(3, Ordering::SeqCst);
        let error = fixture.apply(directory.path()).unwrap_err();
        let CommandError::ObservationWriteFailure(failure) = &error else {
            panic!("typed storage failure required, found {error:?}")
        };
        assert!(failure.write_failure.is_some());
        assert!(failure.receipts.is_empty());
        assert_eq!(fixture.taken_requests().len(), 1);
        assert_eq!(
            fixture.scalar("SELECT count(*) FROM entity_identity_links"),
            0,
            "an unretained response never reaches legacy persistence"
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert_eq!(fixture.marker(), Some(100));
        assert!(error
            .to_string()
            .contains("could not confirm response retention"));

        // A later response-write failure keeps every earlier receipt.
        let fixture = Fixture::start();
        fixture.mode.store(6, Ordering::SeqCst);
        let error = fixture.apply(directory.path()).unwrap_err();
        let CommandError::ObservationWriteFailure(failure) = &error else {
            panic!("typed storage failure required, found {error:?}")
        };
        assert_eq!(failure.receipts.len(), 1);
        assert!(failure.write_failure.is_some());
        assert_eq!(fixture.taken_requests().len(), 2);

        // An RSS storage failure stops the track before its legacy write.
        let fixture = Fixture::start();
        fixture.mode.store(7, Ordering::SeqCst);
        let error = fixture.apply(directory.path()).unwrap_err();
        let CommandError::ObservationWriteFailure(failure) = &error else {
            panic!("typed storage failure required, found {error:?}")
        };
        assert_eq!(
            failure.receipts.len(),
            3,
            "three Index receipts stay retained"
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert_eq!(fixture.marker(), Some(100));
    }

    /// F39-05, F39-06, F39-10: ordinary failures keep their skips and messages.
    #[test]
    fn adr_0075_feed_observation_ordinary_failures_keep_skips_and_messages() {
        let fixture = Fixture::start();
        let directory = audio_directory();

        // An ordinary batch-check failure stays skipped after retention.
        fixture.mode.store(1, Ordering::SeqCst);
        let recorder = Arc::new(ProviderObservationRecorder::new(Arc::clone(&fixture.conn)));
        let stale = check_feed_batch_for_updates(
            &fixture.conn,
            &fixture.endpoint,
            &fixture.feed_rows(),
            &recorder,
            &fixture.context(),
        )
        .unwrap();
        assert!(stale.is_empty());
        assert_eq!(recorder.take_receipts().len(), 1);

        // A single-feed root keeps its feed error family and its receipts.
        let error = fixture.check_one().unwrap_err();
        let CommandError::ObservedCommandFailure(failure) = &error else {
            panic!("ordinary wrapper required, found {error:?}")
        };
        assert_eq!(failure.receipts().len(), 1);
        assert!(matches!(failure.cause(), CommandError::Feed(_)));
        assert_eq!(error.to_string(), failure.cause().to_string());
        assert!(!format!("{error:?}").contains(&fixture.base));
        assert_eq!(error.clone(), error);

        // An ordinary update failure keeps its per-feed message and continues.
        let result = fixture.apply(directory.path()).unwrap();
        assert_eq!(result.tracks_updated(), 0);
        assert_eq!(result.feed_errors().len(), 0);
        // The update sequence keeps its four requests. Each failed response leaves
        // its own receipt.
        assert_eq!(result.observation_receipts().len(), 4);
        assert_eq!(
            fixture.marker(),
            Some(200),
            "ordinary skips still advance the marker"
        );

        // A configuration failure keeps the existing per-feed message.
        let recorder = Arc::new(ProviderObservationRecorder::new(Arc::clone(&fixture.conn)));
        let result = apply_stale_feed_updates_from(
            &fixture.conn,
            &fixture.endpoint,
            &[fixture.stale_entry()],
            &recorder,
            &fixture.context(),
            &|| Err(anyhow::anyhow!("configuration unreadable")),
        )
        .unwrap();
        assert_eq!(
            result.feed_errors(),
            ["Local feed: configuration unreadable"]
        );
        assert_eq!(
            result.message(),
            "No edits written — Feed errors (1): Local feed: configuration unreadable"
        );
        assert!(recorder.take_receipts().is_empty());

        // A missing physical file still incurs requests before its tag error.
        let empty = tempfile::tempdir().unwrap();
        fixture.mode.store(0, Ordering::SeqCst);
        let _ = fixture.taken_requests();
        let result = fixture.apply(empty.path()).unwrap();
        assert_eq!(result.id3_errors().len(), 1);
        assert_eq!(result.tracks_updated(), 0);
        assert_eq!(fixture.taken_requests().len(), 4);
        assert_eq!(result.observation_receipts().len(), 4);
        assert_eq!(fixture.marker(), Some(200));
    }

    /// F39-08: cancellation keeps its original cause and every earlier receipt.
    #[test]
    fn adr_0075_feed_observation_cancellation_keeps_cause_and_receipts() {
        let fixture = Fixture::start();
        fixture.token.cancel();
        let error = fixture.check_one().unwrap_err();
        assert_eq!(error, CommandError::Cancelled);
        assert!(fixture.taken_requests().is_empty());

        let fixture = Fixture::start();
        fixture.cancel_after.store(1, Ordering::SeqCst);
        let recorder = Arc::new(ProviderObservationRecorder::new(Arc::clone(&fixture.conn)));
        let result = check_feed_batch_for_updates(
            &fixture.conn,
            &fixture.endpoint,
            &[
                fixture.feed_rows()[0].clone(),
                fixture.feed_rows()[0].clone(),
            ],
            &recorder,
            &fixture.context(),
        );
        let error =
            assemble_observed_feed_command(&recorder, result, |value: &mut Vec<StaleFeed>, _| {
                value.clear();
            })
            .unwrap_err();
        let CommandError::ObservedCommandFailure(failure) = &error else {
            panic!("cancellation wrapper required, found {error:?}")
        };
        assert_eq!(failure.cause(), &CommandError::Cancelled);
        assert_eq!(failure.receipts().len(), 1);
        assert_eq!(error.to_string(), CommandError::Cancelled.to_string());
        assert_eq!(fixture.taken_requests().len(), 1);
    }

    /// F39-09: the combined root keeps feed receipts through route repair.
    #[test]
    fn adr_0075_feed_observation_combined_root_keeps_receipts_through_repair() {
        let fixture = Fixture::start();
        let directory = audio_directory();
        fixture.mode.store(5, Ordering::SeqCst);

        let outcome = CheckFeedsAndRepairRoutes::new(
            Arc::clone(&fixture.conn),
            fixture.endpoint.clone(),
            directory.path().to_owned(),
            fixture.feed_rows(),
        )
        .execute(&fixture.context())
        .unwrap();
        let (result, events) = outcome.into_parts();
        assert_eq!(result.feeds_checked(), 1);
        assert_eq!(result.stale_feed_count(), 0);
        assert!(result.feed_updates().is_none());
        assert_eq!(
            result.observation_receipts().len(),
            1,
            "packet 044 still owns route-repair request retention"
        );
        assert_eq!(
            result.observation_receipts()[0].outcome,
            ObservationOutcome::Success
        );
        // ADR 0065 repairs payment routes after the check. The track request and its
        // feed fallback keep their existing profiles. Packet 044 owns their retention,
        // so neither request allocates a generation here.
        assert_eq!(
            fixture.taken_requests(),
            vec![
                "/v1/feeds/f1",
                "/v1/feeds/f1/tracks/t1?include=payment_routes",
                "/v1/feeds/f1?include=payment_routes",
            ]
        );
        // No feed became stale, so the command adds no feed-update event. The repair
        // scanned the one local track, found no upstream route, and tagged nothing.
        assert_eq!(result.route_repairs().tracks.len(), 1);
        assert!(events.is_empty());

        // An ordinary repair failure still returns the earlier feed receipts.
        let fixture = Fixture::start();
        fixture.mode.store(5, Ordering::SeqCst);
        fixture
            .conn
            .lock()
            .unwrap()
            .execute_batch("CREATE TEMP TRIGGER reject_readiness BEFORE INSERT ON metadata_request_slots BEGIN SELECT 1; END")
            .unwrap();
        let feeds = fixture.feed_rows();
        let error = CheckFeedsAndRepairRoutes::new(
            Arc::clone(&fixture.conn),
            fixture.endpoint.clone(),
            Path::new("/nonexistent-v4vmm-fixture").to_owned(),
            feeds,
        )
        .execute(&fixture.context())
        .map(|outcome| outcome.into_parts().0);
        match error {
            Ok(result) => assert_eq!(result.observation_receipts().len(), 1),
            Err(CommandError::ObservedCommandFailure(failure)) => {
                assert_eq!(failure.receipts().len(), 1);
            }
            Err(other) => panic!("unexpected combined failure {other:?}"),
        }

        // Cancellation after the feed check keeps the original cause.
        let fixture = Fixture::start();
        fixture.mode.store(5, Ordering::SeqCst);
        fixture.cancel_after.store(1, Ordering::SeqCst);
        let feeds = fixture.feed_rows();
        let error = CheckFeedsAndRepairRoutes::new(
            Arc::clone(&fixture.conn),
            fixture.endpoint.clone(),
            directory.path().to_owned(),
            [feeds[0].clone(), feeds[0].clone()].to_vec(),
        )
        .execute(&fixture.context())
        .unwrap_err();
        let CommandError::ObservedCommandFailure(failure) = &error else {
            panic!("cancellation wrapper required, found {error:?}")
        };
        assert_eq!(failure.cause(), &CommandError::Cancelled);
        assert_eq!(failure.receipts().len(), 1);
    }

    /// F39-10: the ordinary wrapper flattens and keeps each error family.
    #[test]
    fn adr_0075_feed_observation_receipt_attachment_keeps_error_families() {
        use crate::application::errors::command::{
            attach_observation_receipts, ObservedQueryFailure,
        };
        let fixture = Fixture::start();
        fixture.check_one().unwrap();
        let receipt = {
            let result = fixture.check_one().unwrap();
            result.observation_receipts()[0].clone()
        };

        let plain = CommandError::Feed("offline".into());
        assert_eq!(
            attach_observation_receipts(plain.clone(), Vec::new()),
            plain,
            "no receipts returns the original error"
        );
        let wrapped = attach_observation_receipts(plain.clone(), vec![receipt.clone()]);
        assert_eq!(wrapped.to_string(), plain.to_string());
        let flattened = attach_observation_receipts(wrapped, vec![receipt.clone()]);
        let CommandError::ObservedCommandFailure(failure) = &flattened else {
            panic!("ordinary wrapper required")
        };
        assert_eq!(failure.cause(), &plain, "the wrapper never nests");
        assert_eq!(failure.receipts().len(), 2);

        let query = CommandError::ObservedQueryFailure(Arc::new(ObservedQueryFailure::new(
            "query cause".into(),
            vec![receipt.clone()],
        )));
        let merged = attach_observation_receipts(query, vec![receipt.clone()]);
        let CommandError::ObservedQueryFailure(failure) = &merged else {
            panic!("the query family must stay a query failure")
        };
        assert_eq!(failure.receipts().len(), 2);
        assert_eq!(failure.message(), "query cause");
    }
}
