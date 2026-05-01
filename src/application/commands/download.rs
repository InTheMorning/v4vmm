//! Download command family.

use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::application::command_bus::{ApplicationCommand, CommandOutcome, CommandResult};
use crate::application::command_context::CommandContext;
use crate::application::errors::command::CommandError;
use crate::application::events::download::DownloadEvent;
use crate::application::events::feed::FeedEvent;
use crate::application::events::library::LibraryEvent;
use crate::application::events::ApplicationEvent;
use crate::library_service;

type SharedConnection = Arc<Mutex<Connection>>;

/// Command result for removing a track from the local library.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoveTrackFromLibraryResult {
    message: String,
}

impl RemoveTrackFromLibraryResult {
    /// Creates a track removal result.
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

/// Command result for setting local track library membership.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetTrackLibraryMembershipResult {
    in_library: bool,
    message: String,
}

impl SetTrackLibraryMembershipResult {
    /// Creates a track membership result.
    #[must_use]
    pub fn new(in_library: bool, message: impl Into<String>) -> Self {
        Self {
            in_library,
            message: message.into(),
        }
    }

    /// Returns whether the track is now in the library.
    #[must_use]
    pub const fn in_library(&self) -> bool {
        self.in_library
    }

    /// Returns the user-facing completion message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Sets local track library membership by local track id.
#[derive(Clone, Debug)]
pub struct SetTrackLibraryMembership {
    conn: SharedConnection,
    track_id: i64,
    in_library: bool,
}

impl SetTrackLibraryMembership {
    /// Creates a track membership command.
    #[must_use]
    pub const fn new(conn: SharedConnection, track_id: i64, in_library: bool) -> Self {
        Self {
            conn,
            track_id,
            in_library,
        }
    }
}

impl ApplicationCommand for SetTrackLibraryMembership {
    type Output = SetTrackLibraryMembershipResult;

    fn execute(self, _context: &CommandContext) -> CommandResult<Self::Output> {
        let conn = self.conn.lock().map_err(|_| download_lock_error())?;
        library_service::set_track_in_library(&conn, self.track_id, self.in_library)
            .map_err(|error| download_command_error(&error))?;
        let message = if self.in_library {
            "Subscribed track"
        } else {
            "Unsubscribed track"
        };
        Ok(CommandOutcome::new(
            SetTrackLibraryMembershipResult::new(self.in_library, message),
            vec![ApplicationEvent::Library(LibraryEvent::Changed)],
        ))
    }
}

/// Removes a local track from the library by local track id.
#[derive(Clone, Debug)]
pub struct RemoveTrackFromLibrary {
    conn: SharedConnection,
    track_id: i64,
}

impl RemoveTrackFromLibrary {
    /// Creates a track removal command.
    #[must_use]
    pub const fn new(conn: SharedConnection, track_id: i64) -> Self {
        Self { conn, track_id }
    }
}

impl ApplicationCommand for RemoveTrackFromLibrary {
    type Output = RemoveTrackFromLibraryResult;

    fn execute(self, _context: &CommandContext) -> CommandResult<Self::Output> {
        let conn = self.conn.lock().map_err(|_| download_lock_error())?;
        library_service::set_track_in_library(&conn, self.track_id, false)
            .map_err(|error| download_command_error(&error))?;
        Ok(CommandOutcome::new(
            RemoveTrackFromLibraryResult::new("Removed track"),
            track_removed_events(false),
        ))
    }
}

/// Removes a local track from the library by feed/item/enclosure identity.
#[derive(Clone, Debug)]
pub struct RemoveTrackFromLibraryByMatch {
    conn: SharedConnection,
    feed_url: Option<String>,
    item_guid: Option<String>,
    enclosure_url: Option<String>,
}

impl RemoveTrackFromLibraryByMatch {
    /// Creates a matched track removal command.
    #[must_use]
    pub fn new(
        conn: SharedConnection,
        feed_url: Option<String>,
        item_guid: Option<String>,
        enclosure_url: Option<String>,
    ) -> Self {
        Self {
            conn,
            feed_url,
            item_guid,
            enclosure_url,
        }
    }
}

impl ApplicationCommand for RemoveTrackFromLibraryByMatch {
    type Output = RemoveTrackFromLibraryResult;

    fn execute(self, _context: &CommandContext) -> CommandResult<Self::Output> {
        let conn = self.conn.lock().map_err(|_| download_lock_error())?;
        library_service::set_track_in_library_by_match(
            &conn,
            self.feed_url.as_deref(),
            self.item_guid.as_deref(),
            self.enclosure_url.as_deref(),
            false,
        )
        .map_err(|error| download_command_error(&error))?;
        let feed_changed = if let Some(feed_url) = self.feed_url.as_deref() {
            crate::db::reconcile_feed_subscription_by_url(&conn, feed_url)
                .map_err(|error| download_command_error(&error))?;
            true
        } else {
            false
        };
        Ok(CommandOutcome::new(
            RemoveTrackFromLibraryResult::new("Removed track"),
            track_removed_events(feed_changed),
        ))
    }
}

fn track_removed_events(feed_changed: bool) -> Vec<ApplicationEvent> {
    let mut events = vec![
        ApplicationEvent::Download(DownloadEvent::Changed),
        ApplicationEvent::Library(LibraryEvent::Changed),
    ];
    if feed_changed {
        events.push(ApplicationEvent::Feed(FeedEvent::Changed));
    }
    events
}

fn download_lock_error() -> CommandError {
    CommandError::Download("database lock poisoned".to_string())
}

fn download_command_error(error: &anyhow::Error) -> CommandError {
    CommandError::Download(format!("{error:#}"))
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

    fn create_feed(conn: &Connection, feed_url: &str) -> anyhow::Result<i64> {
        conn.execute(
            "INSERT INTO feeds (feed_url, feed_guid, title, is_subscribed)
             VALUES (?1, ?2, ?3, 1)",
            rusqlite::params![feed_url, "feed-guid", "Feed Title"],
        )?;
        Ok(conn.last_insert_rowid())
    }

    fn create_library_track(
        conn: &Connection,
        feed_id: i64,
        item_guid: &str,
        enclosure_url: &str,
    ) -> anyhow::Result<i64> {
        conn.execute(
            "INSERT INTO tracks (
                 feed_id, item_guid, enclosure_url, track_title, is_in_library
             )
             VALUES (?1, ?2, ?3, ?4, 1)",
            rusqlite::params![feed_id, item_guid, enclosure_url, "Track Title"],
        )?;
        Ok(conn.last_insert_rowid())
    }

    #[test]
    fn remove_track_by_id_updates_library_state() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let track_id = {
            let db = conn.lock().expect("lock test db");
            let feed_id = create_feed(&db, "https://example.test/feed.xml")?;
            create_library_track(&db, feed_id, "item-guid", "https://cdn.test/audio.mp3")?
        };

        let outcome = CommandBus::new().execute(
            RemoveTrackFromLibrary::new(Arc::clone(&conn), track_id),
            &CommandContext::next(),
        )?;

        assert_eq!(outcome.value().message(), "Removed track");
        assert_eq!(outcome.events(), track_removed_events(false));
        let db = conn.lock().expect("lock test db");
        let track = db::track_row_by_id(&db, track_id)?.expect("track exists");
        assert!(!track.is_in_library);

        Ok(())
    }

    #[test]
    fn set_track_membership_can_subscribe_local_track() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let track_id = {
            let db = conn.lock().expect("lock test db");
            let feed_id = create_feed(&db, "https://example.test/feed.xml")?;
            let track_id =
                create_library_track(&db, feed_id, "item-guid", "https://cdn.test/audio.mp3")?;
            library_service::set_track_in_library(&db, track_id, false)?;
            track_id
        };

        let outcome = CommandBus::new().execute(
            SetTrackLibraryMembership::new(Arc::clone(&conn), track_id, true),
            &CommandContext::next(),
        )?;

        assert!(outcome.value().in_library());
        assert_eq!(outcome.value().message(), "Subscribed track");
        assert_eq!(
            outcome.events(),
            &[ApplicationEvent::Library(LibraryEvent::Changed)]
        );
        let db = conn.lock().expect("lock test db");
        let track = db::track_row_by_id(&db, track_id)?.expect("track exists");
        assert!(track.is_in_library);

        Ok(())
    }

    #[test]
    fn remove_track_by_match_reconciles_feed_subscription() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let track_id = {
            let db = conn.lock().expect("lock test db");
            let feed_id = create_feed(&db, "https://example.test/feed.xml")?;
            create_library_track(&db, feed_id, "item-guid", "https://cdn.test/audio.mp3")?
        };

        let outcome = CommandBus::new().execute(
            RemoveTrackFromLibraryByMatch::new(
                Arc::clone(&conn),
                Some("https://example.test/feed.xml".to_string()),
                Some("item-guid".to_string()),
                Some("https://cdn.test/audio.mp3".to_string()),
            ),
            &CommandContext::next(),
        )?;

        assert_eq!(outcome.value().message(), "Removed track");
        assert_eq!(outcome.events(), track_removed_events(true));
        let db = conn.lock().expect("lock test db");
        let track = db::track_row_by_id(&db, track_id)?.expect("track exists");
        assert!(!track.is_in_library);
        assert!(!db::feed_is_subscribed_by_url(
            &db,
            "https://example.test/feed.xml"
        )?);

        Ok(())
    }
}
