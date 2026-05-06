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
}
