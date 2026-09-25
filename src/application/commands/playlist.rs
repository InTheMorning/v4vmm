//! Playlist command family.

use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::application::command_bus::{ApplicationCommand, CommandOutcome, CommandResult};
use crate::application::command_context::CommandContext;
use crate::application::errors::command::CommandError;
use crate::application::events::library::LibraryEvent;
use crate::application::events::playlist::PlaylistEvent;
use crate::application::events::ApplicationEvent;
use crate::playlist_service;
use crate::runtime::{PlaylistRssCheckHandle, RssCheckTrigger};

type SharedConnection = Arc<Mutex<Connection>>;

/// Command result for creating a playlist.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CreatePlaylistResult {
    playlist_id: i64,
}

impl CreatePlaylistResult {
    /// Creates a result for a newly-created playlist.
    #[must_use]
    pub const fn new(playlist_id: i64) -> Self {
        Self { playlist_id }
    }

    /// Returns the new playlist id.
    #[must_use]
    pub const fn playlist_id(self) -> i64 {
        self.playlist_id
    }
}

/// Command result for appending existing tracks to a playlist.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppendTracksToPlaylistResult {
    appended: usize,
}

impl AppendTracksToPlaylistResult {
    /// Creates an append result.
    #[must_use]
    pub const fn new(appended: usize) -> Self {
        Self { appended }
    }

    /// Returns how many tracks were appended.
    #[must_use]
    pub const fn appended(self) -> usize {
        self.appended
    }
}

/// Creates a playlist.
#[derive(Clone, Debug)]
pub struct CreatePlaylist {
    conn: SharedConnection,
    name: String,
}

impl CreatePlaylist {
    /// Creates a playlist command.
    #[must_use]
    pub fn new(conn: SharedConnection, name: impl Into<String>) -> Self {
        Self {
            conn,
            name: name.into(),
        }
    }
}

impl ApplicationCommand for CreatePlaylist {
    type Output = CreatePlaylistResult;

    fn execute(self, _context: &CommandContext) -> CommandResult<Self::Output> {
        let conn = self.conn.lock().map_err(|_| poisoned_lock())?;
        let playlist_id = playlist_service::create(&conn, &self.name)
            .map_err(|error| playlist_command_error(&error))?;
        Ok(CommandOutcome::new(
            CreatePlaylistResult::new(playlist_id),
            playlist_changed_events(),
        ))
    }
}

/// Renames a playlist.
#[derive(Clone, Debug)]
pub struct RenamePlaylist {
    conn: SharedConnection,
    playlist_id: i64,
    new_name: String,
}

impl RenamePlaylist {
    /// Creates a playlist rename command.
    #[must_use]
    pub fn new(conn: SharedConnection, playlist_id: i64, new_name: impl Into<String>) -> Self {
        Self {
            conn,
            playlist_id,
            new_name: new_name.into(),
        }
    }
}

impl ApplicationCommand for RenamePlaylist {
    type Output = ();

    fn execute(self, _context: &CommandContext) -> CommandResult<Self::Output> {
        let conn = self.conn.lock().map_err(|_| poisoned_lock())?;
        playlist_service::rename(&conn, self.playlist_id, &self.new_name)
            .map_err(|error| playlist_command_error(&error))?;
        Ok(CommandOutcome::new((), playlist_changed_events()))
    }
}

/// Deletes a playlist.
#[derive(Clone, Debug)]
pub struct DeletePlaylist {
    conn: SharedConnection,
    playlist_id: i64,
}

impl DeletePlaylist {
    /// Creates a playlist delete command.
    #[must_use]
    pub const fn new(conn: SharedConnection, playlist_id: i64) -> Self {
        Self { conn, playlist_id }
    }
}

impl ApplicationCommand for DeletePlaylist {
    type Output = ();

    fn execute(self, _context: &CommandContext) -> CommandResult<Self::Output> {
        let conn = self.conn.lock().map_err(|_| poisoned_lock())?;
        playlist_service::delete(&conn, self.playlist_id)
            .map_err(|error| playlist_command_error(&error))?;
        Ok(CommandOutcome::new((), playlist_changed_events()))
    }
}

/// Removes a track at a playlist position.
#[derive(Clone, Debug)]
pub struct RemovePlaylistTrackAt {
    conn: SharedConnection,
    playlist_id: i64,
    position: i64,
}

impl RemovePlaylistTrackAt {
    /// Creates a playlist track removal command.
    #[must_use]
    pub const fn new(conn: SharedConnection, playlist_id: i64, position: i64) -> Self {
        Self {
            conn,
            playlist_id,
            position,
        }
    }
}

impl ApplicationCommand for RemovePlaylistTrackAt {
    type Output = ();

    fn execute(self, _context: &CommandContext) -> CommandResult<Self::Output> {
        let mut conn = self.conn.lock().map_err(|_| poisoned_lock())?;
        playlist_service::remove_track_at(&mut conn, self.playlist_id, self.position)
            .map_err(|error| playlist_command_error(&error))?;
        Ok(CommandOutcome::new(
            (),
            playlist_tracks_changed_events(self.playlist_id),
        ))
    }
}

/// Reorders a track inside a playlist.
#[derive(Clone, Debug)]
pub struct ReorderPlaylistTrack {
    conn: SharedConnection,
    playlist_id: i64,
    from: i64,
    to: i64,
}

impl ReorderPlaylistTrack {
    /// Creates a playlist track reorder command.
    #[must_use]
    pub const fn new(conn: SharedConnection, playlist_id: i64, from: i64, to: i64) -> Self {
        Self {
            conn,
            playlist_id,
            from,
            to,
        }
    }
}

impl ApplicationCommand for ReorderPlaylistTrack {
    type Output = ();

    fn execute(self, _context: &CommandContext) -> CommandResult<Self::Output> {
        let mut conn = self.conn.lock().map_err(|_| poisoned_lock())?;
        playlist_service::reorder(&mut conn, self.playlist_id, self.from, self.to)
            .map_err(|error| playlist_command_error(&error))?;
        Ok(CommandOutcome::new(
            (),
            playlist_tracks_changed_events(self.playlist_id),
        ))
    }
}

/// Appends existing tracks to a playlist without subscription/download work.
#[derive(Clone, Debug)]
pub struct AppendTracksToPlaylist {
    conn: SharedConnection,
    playlist_id: i64,
    track_ids: Vec<i64>,
}

impl AppendTracksToPlaylist {
    /// Creates a playlist append command for existing local tracks.
    #[must_use]
    pub const fn new(conn: SharedConnection, playlist_id: i64, track_ids: Vec<i64>) -> Self {
        Self {
            conn,
            playlist_id,
            track_ids,
        }
    }
}

impl ApplicationCommand for AppendTracksToPlaylist {
    type Output = AppendTracksToPlaylistResult;

    fn execute(self, _context: &CommandContext) -> CommandResult<Self::Output> {
        let conn = self.conn.lock().map_err(|_| poisoned_lock())?;
        let mut appended = 0;
        for track_id in self.track_ids {
            playlist_service::append_track(&conn, self.playlist_id, track_id)
                .map_err(|error| playlist_command_error(&error))?;
            appended += 1;
        }
        Ok(CommandOutcome::new(
            AppendTracksToPlaylistResult::new(appended),
            playlist_tracks_changed_events(self.playlist_id),
        ))
    }
}

/// Starts the RSS check of a playlist (ADR 0076 Decision 2).
///
/// The command sends `Start` to the playlist RSS check actor and returns at
/// once. It does not wait for the check. A start for a playlist with a
/// running check joins that check.
#[derive(Clone, Debug)]
pub struct CheckPlaylistRss {
    checker: PlaylistRssCheckHandle,
    playlist_id: i64,
    trigger: RssCheckTrigger,
}

impl CheckPlaylistRss {
    /// Creates a playlist RSS check command.
    #[must_use]
    pub fn new(
        checker: PlaylistRssCheckHandle,
        playlist_id: i64,
        trigger: RssCheckTrigger,
    ) -> Self {
        Self {
            checker,
            playlist_id,
            trigger,
        }
    }
}

impl ApplicationCommand for CheckPlaylistRss {
    type Output = ();

    fn execute(self, _context: &CommandContext) -> CommandResult<Self::Output> {
        if self.checker.start(self.playlist_id, self.trigger) {
            Ok(CommandOutcome::new((), Vec::new()))
        } else {
            Err(CommandError::Playlist(
                "The RSS check actor stopped, or it has too many waiting requests. Try again."
                    .to_owned(),
            ))
        }
    }
}

/// Records that the operator confirmed a track with the "removed from feed"
/// mark for the show (ADR 0076 Decision 7). The track keeps its mark, and its
/// readiness row then shows its route state.
#[derive(Clone, Debug)]
pub(crate) struct ConfirmRemovedTrack {
    conn: SharedConnection,
    track_id: i64,
}

impl ConfirmRemovedTrack {
    /// Creates a removed-track confirmation command.
    #[must_use]
    pub(crate) const fn new(conn: SharedConnection, track_id: i64) -> Self {
        Self { conn, track_id }
    }
}

impl ApplicationCommand for ConfirmRemovedTrack {
    /// Whether the command changed the track. A track that has no
    /// unconfirmed mark stays unchanged.
    type Output = bool;

    fn execute(self, _context: &CommandContext) -> CommandResult<Self::Output> {
        let conn = self.conn.lock().map_err(|_| poisoned_lock())?;
        let confirmed_at_us = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| {
                i64::try_from(elapsed.as_micros()).unwrap_or(i64::MAX)
            });
        let changed = crate::db::rss_field_holds::confirm_removed_track(
            &conn,
            self.track_id,
            confirmed_at_us,
        )
        .map_err(|error| playlist_command_error(&error))?;
        Ok(CommandOutcome::new(changed, Vec::new()))
    }
}

/// Removes a track from each playlist that holds it, after the operator
/// confirmed the list of playlists (ADR 0076 Decision 7, operator decision
/// 2026-09-24). The output is the ids of the changed playlists.
#[derive(Clone, Debug)]
pub(crate) struct RemoveTrackFromAllPlaylists {
    conn: SharedConnection,
    track_id: i64,
}

impl RemoveTrackFromAllPlaylists {
    /// Creates a removal command for each playlist entry of one track.
    #[must_use]
    pub(crate) const fn new(conn: SharedConnection, track_id: i64) -> Self {
        Self { conn, track_id }
    }
}

impl ApplicationCommand for RemoveTrackFromAllPlaylists {
    type Output = Vec<i64>;

    fn execute(self, _context: &CommandContext) -> CommandResult<Self::Output> {
        let mut conn = self.conn.lock().map_err(|_| poisoned_lock())?;
        let changed = playlist_service::remove_track_everywhere(&mut conn, self.track_id)
            .map_err(|error| playlist_command_error(&error))?;
        let events = changed
            .iter()
            .flat_map(|playlist_id| playlist_tracks_changed_events(*playlist_id))
            .collect();
        Ok(CommandOutcome::new(changed, events))
    }
}

fn playlist_changed_events() -> Vec<ApplicationEvent> {
    vec![
        ApplicationEvent::Playlist(PlaylistEvent::Changed),
        ApplicationEvent::Library(LibraryEvent::Changed),
    ]
}

fn playlist_tracks_changed_events(playlist_id: i64) -> Vec<ApplicationEvent> {
    vec![
        ApplicationEvent::Playlist(PlaylistEvent::TracksChanged { playlist_id }),
        ApplicationEvent::Library(LibraryEvent::Changed),
    ]
}

fn poisoned_lock() -> CommandError {
    CommandError::Playlist("database lock poisoned".to_string())
}

fn playlist_command_error(error: &anyhow::Error) -> CommandError {
    CommandError::Playlist(format!("{error:#}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::command_bus::CommandBus;
    use crate::application::command_context::CommandContext;
    use crate::db;

    fn setup_test_db() -> anyhow::Result<SharedConnection> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        db::init_schema(&conn)?;
        db::migrate_schema(&conn)?;
        Ok(Arc::new(Mutex::new(conn)))
    }

    fn create_feed(conn: &Connection) -> anyhow::Result<i64> {
        conn.execute(
            "INSERT INTO feeds (feed_url, feed_guid, title)
             VALUES (?1, ?2, ?3)",
            rusqlite::params!["https://example.test/feed.xml", "feed-guid", "Feed Title"],
        )?;
        Ok(conn.last_insert_rowid())
    }

    fn create_track(conn: &Connection, feed_id: i64, item_guid: &str) -> anyhow::Result<i64> {
        conn.execute(
            "INSERT INTO tracks (feed_id, item_guid, track_title, is_in_library)
             VALUES (?1, ?2, ?3, 1)",
            rusqlite::params![feed_id, item_guid, format!("Track {item_guid}")],
        )?;
        Ok(conn.last_insert_rowid())
    }

    #[test]
    fn create_playlist_emits_playlist_and_library_events() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let result = CommandBus::new().execute(
            CreatePlaylist::new(Arc::clone(&conn), "Focus"),
            &CommandContext::next(),
        )?;

        let playlist_id = result.value().playlist_id();
        assert!(playlist_id > 0);
        assert_eq!(result.events(), playlist_changed_events());

        Ok(())
    }

    #[test]
    fn append_existing_tracks_preserves_order_and_emits_track_event() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let (playlist_id, first_track_id, second_track_id) = {
            let db = conn.lock().expect("lock test db");
            let feed_id = create_feed(&db)?;
            let playlist_id = playlist_service::create(&db, "Focus")?;
            let first_track_id = create_track(&db, feed_id, "first")?;
            let second_track_id = create_track(&db, feed_id, "second")?;
            (playlist_id, first_track_id, second_track_id)
        };

        let result = CommandBus::new().execute(
            AppendTracksToPlaylist::new(
                Arc::clone(&conn),
                playlist_id,
                vec![first_track_id, second_track_id],
            ),
            &CommandContext::next(),
        )?;

        assert_eq!(result.value().appended(), 2);
        assert_eq!(result.events(), playlist_tracks_changed_events(playlist_id));
        let tracks = {
            let db = conn.lock().expect("lock test db");
            playlist_service::tracks(&db, playlist_id)?
        };
        assert_eq!(tracks[0].id, first_track_id);
        assert_eq!(tracks[1].id, second_track_id);

        Ok(())
    }

    #[test]
    fn invalid_rename_returns_shared_command_error() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let playlist_id = {
            let db = conn.lock().expect("lock test db");
            playlist_service::create(&db, "Focus")?
        };

        let error = CommandBus::new()
            .execute(
                RenamePlaylist::new(Arc::clone(&conn), playlist_id, " "),
                &CommandContext::next(),
            )
            .expect_err("blank rename should fail");

        assert!(
            error.to_string().contains("playlist command failed"),
            "unexpected error: {error}"
        );

        Ok(())
    }

    /// R1-14: the playback start sends the check with trigger
    /// `playback_start`, and the playback command completes before the
    /// check finishes.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn adr_0076_playlist_check_playback_start_runs_without_delaying_playback(
    ) -> anyhow::Result<()> {
        use std::path::PathBuf;
        use std::time::Duration;

        use crate::application::commands::playback::PlayPlaylistAt;
        use crate::application::session_lifecycle::SessionLifecycle;
        use crate::playback_driver::NullDriver;
        use crate::playback_owner::PlaybackOwner;
        use crate::runtime::playlist_rss_check::test_support::{
            database, finished, response, FakeClock, FakeFetcher, RSS,
        };
        use crate::runtime::playlist_rss_check::{spawn_with, CheckClock, RssDocumentFetcher};

        let url = "https://play.test/r114/feed.xml";
        let (conn, playlist_id, _) = database(&[(url, 1)]);
        {
            let db = conn.lock().expect("lock test db");
            let track_id: i64 =
                db.query_row("SELECT id FROM tracks LIMIT 1", [], |row| row.get(0))?;
            db::mark_track_downloaded(
                &db,
                track_id,
                &crate::library_path::LibraryRelativePath::for_test("tmp/track.mp3"),
                None,
            )?;
        }
        let clock = FakeClock::new();
        let fetcher =
            FakeFetcher::new(Arc::clone(&clock), |url| response(url, 200, &[], Some(RSS)));
        fetcher.close_gate();
        let session = SessionLifecycle::new();
        let checker = spawn_with(
            Arc::clone(&conn),
            Arc::clone(&fetcher) as Arc<dyn RssDocumentFetcher>,
            Arc::clone(&clock) as Arc<dyn CheckClock>,
            &session,
        );
        let owner = Arc::new(Mutex::new(PlaybackOwner::new(
            NullDriver::new(),
            crate::playback::DEFAULT_SESSION_ID,
            PathBuf::from("/"),
        )));

        // The order of the `PlayPlaylistAt` handling in `src/app.rs`.
        CommandBus::new().execute(
            CheckPlaylistRss::new(checker.clone(), playlist_id, RssCheckTrigger::PlaybackStart),
            &CommandContext::next(),
        )?;
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while fetcher.active().0 == 0 {
            assert!(
                std::time::Instant::now() < deadline,
                "the RSS request starts"
            );
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        let played = CommandBus::new().execute(
            PlayPlaylistAt::new(Arc::clone(&conn), Arc::clone(&owner), playlist_id, 0),
            &CommandContext::next(),
        )?;
        assert!(played.value().update().is_some());
        let running = checker.latest();
        let run = running.run(playlist_id).expect("the check runs");
        assert!(run.is_running(), "playback completed before the check");
        assert_eq!(run.trigger, RssCheckTrigger::PlaybackStart);

        fetcher.open_gate();
        let run = finished(&checker, playlist_id).await;
        assert_eq!(run.trigger, RssCheckTrigger::PlaybackStart);

        let app = include_str!("../../app.rs");
        let arm = app
            .split("LibraryAppEvent::PlayPlaylistAt {")
            .nth(1)
            .and_then(|rest| rest.split("LibraryAppEvent::OpenSavedSearch").next())
            .expect("app.rs handles PlayPlaylistAt");
        assert!(
            arm.contains("RssCheckTrigger::PlaybackStart"),
            "ADR 0076 packet 001: the PlayPlaylistAt handling in src/app.rs must send CheckPlaylistRss with RssCheckTrigger::PlaybackStart"
        );
        Ok(())
    }

    /// Two playlists. "Friday Show" holds the removed track at positions 0
    /// and 2, and "Warm Up" holds it at position 1.
    fn removed_track_fixture(conn: &SharedConnection) -> anyhow::Result<(i64, i64, i64, i64)> {
        let db = conn.lock().expect("lock test db");
        let feed_id = create_feed(&db)?;
        let gone = create_track(&db, feed_id, "gone")?;
        let other = create_track(&db, feed_id, "other")?;
        let friday = playlist_service::create(&db, "Friday Show")?;
        let warm_up = playlist_service::create(&db, "Warm Up")?;
        for track_id in [gone, other, gone] {
            playlist_service::append_track(&db, friday, track_id)?;
        }
        for track_id in [other, gone] {
            playlist_service::append_track(&db, warm_up, track_id)?;
        }
        db.execute(
            "UPDATE tracks SET removed_from_feed_at = 1790244000000000 WHERE id = ?1",
            [gone],
        )?;
        Ok((gone, other, friday, warm_up))
    }

    fn track_ids(conn: &SharedConnection, playlist_id: i64) -> anyhow::Result<Vec<i64>> {
        let db = conn.lock().expect("lock test db");
        Ok(playlist_service::tracks(&db, playlist_id)?
            .into_iter()
            .map(|track| track.id)
            .collect())
    }

    /// R3-13: "Remove from playlist" removes only the entry of that row.
    #[test]
    fn adr_0076_route_readiness_remove_from_playlist_removes_only_that_entry() -> anyhow::Result<()>
    {
        let conn = setup_test_db()?;
        let (gone, other, friday, warm_up) = removed_track_fixture(&conn)?;

        CommandBus::new().execute(
            RemovePlaylistTrackAt::new(Arc::clone(&conn), friday, 2),
            &CommandContext::next(),
        )?;

        assert_eq!(track_ids(&conn, friday)?, vec![gone, other]);
        assert_eq!(track_ids(&conn, warm_up)?, vec![other, gone]);
        Ok(())
    }

    /// R3-14: the confirmation data names each playlist that holds the
    /// track, and the command removes the track from each one.
    #[test]
    fn adr_0076_route_readiness_remove_from_all_playlists_removes_each_entry() -> anyhow::Result<()>
    {
        let conn = setup_test_db()?;
        let (gone, other, friday, warm_up) = removed_track_fixture(&conn)?;
        let names = {
            let db = conn.lock().expect("lock test db");
            playlist_service::playlists_holding_track(&db, gone)?
        };
        assert_eq!(
            names,
            vec![
                (friday, "Friday Show".to_owned()),
                (warm_up, "Warm Up".to_owned())
            ]
        );

        let result = CommandBus::new().execute(
            RemoveTrackFromAllPlaylists::new(Arc::clone(&conn), gone),
            &CommandContext::next(),
        )?;

        assert_eq!(result.value(), &vec![friday, warm_up]);
        assert_eq!(track_ids(&conn, friday)?, vec![other]);
        assert_eq!(track_ids(&conn, warm_up)?, vec![other]);
        let positions: Vec<(i64, i64)> = {
            let db = conn.lock().expect("lock test db");
            let mut statement = db.prepare(
                "SELECT playlist_id, position FROM playlist_tracks ORDER BY playlist_id",
            )?;
            let rows = statement
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<rusqlite::Result<_>>()?;
            rows
        };
        assert_eq!(
            positions,
            vec![(friday, 0), (warm_up, 0)],
            "positions stay gapless"
        );
        let mut events = playlist_tracks_changed_events(friday);
        events.extend(playlist_tracks_changed_events(warm_up));
        assert_eq!(result.events(), events);
        {
            let db = conn.lock().expect("lock test db");
            assert!(
                db::track_row_by_id(&db, gone)?.is_some(),
                "the track stays in the library"
            );
        }
        Ok(())
    }

    /// A confirmed removed track leaves the playlist marks, so its playlist
    /// row shows no error.
    #[test]
    fn adr_0076_route_readiness_confirmed_track_has_no_playlist_row_error() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let (gone, _other, friday, _warm_up) = removed_track_fixture(&conn)?;
        let marks = |conn: &SharedConnection| {
            let db = conn.lock().expect("lock test db");
            db::rss_field_holds::playlist_removed_marks(&db, friday)
        };
        assert_eq!(marks(&conn)?, vec![(gone, 1_790_244_000_000_000)]);

        let result = CommandBus::new().execute(
            ConfirmRemovedTrack::new(Arc::clone(&conn), gone),
            &CommandContext::next(),
        )?;

        assert!(result.value());
        assert!(marks(&conn)?.is_empty());
        Ok(())
    }
}
