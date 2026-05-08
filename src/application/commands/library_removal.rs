//! Application commands for canonical local-library removal.

use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::application::command_bus::{ApplicationCommand, CommandOutcome, CommandResult};
use crate::application::command_context::CommandContext;
use crate::application::errors::command::CommandError;
use crate::application::events::download::DownloadEvent;
use crate::application::events::feed::FeedEvent;
use crate::application::events::library::LibraryEvent;
use crate::application::events::ApplicationEvent;
use crate::application::library_removal::{
    execute_library_removal, LibraryRemovalExecution, LibraryRemovalTarget,
};

type SharedConnection = Arc<Mutex<Connection>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoveFromLibraryResult {
    message: &'static str,
    target: LibraryRemovalTarget,
}

impl RemoveFromLibraryResult {
    #[must_use]
    pub const fn new(message: &'static str, target: LibraryRemovalTarget) -> Self {
        Self { message, target }
    }

    #[must_use]
    pub const fn message(&self) -> &'static str {
        self.message
    }

    #[must_use]
    pub const fn target(&self) -> LibraryRemovalTarget {
        self.target
    }
}

#[derive(Clone, Debug)]
pub struct RemoveFromLibrary {
    conn: SharedConnection,
    target: LibraryRemovalTarget,
}

impl RemoveFromLibrary {
    #[must_use]
    pub const fn new(conn: SharedConnection, target: LibraryRemovalTarget) -> Self {
        Self { conn, target }
    }
}

impl ApplicationCommand for RemoveFromLibrary {
    type Output = RemoveFromLibraryResult;

    fn execute(self, _context: &CommandContext) -> CommandResult<Self::Output> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| CommandError::Other("database lock poisoned".into()))?;
        let execution = execute_library_removal(&conn, self.target)
            .map_err(|error| CommandError::Other(format!("{error:#}")))?;
        Ok(CommandOutcome::new(
            RemoveFromLibraryResult::new(execution.message(), execution.target()),
            removal_events(&execution),
        ))
    }
}

fn removal_events(execution: &LibraryRemovalExecution) -> Vec<ApplicationEvent> {
    let mut events = match execution.target() {
        LibraryRemovalTarget::Track(_) => vec![
            ApplicationEvent::Download(DownloadEvent::Changed),
            ApplicationEvent::Library(LibraryEvent::Changed),
        ],
        LibraryRemovalTarget::Feed(_) => vec![
            ApplicationEvent::Feed(FeedEvent::Changed),
            ApplicationEvent::Library(LibraryEvent::Changed),
        ],
    };
    if matches!(execution.target(), LibraryRemovalTarget::Track(_)) && execution.feed_changed() {
        events.push(ApplicationEvent::Feed(FeedEvent::Changed));
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn setup_test_db() -> anyhow::Result<Connection> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        db::init_schema(&conn)?;
        db::migrate_schema(&conn)?;
        Ok(conn)
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
    fn remove_from_library_track_mutates_membership() -> anyhow::Result<()> {
        let conn = Arc::new(Mutex::new(setup_test_db()?));
        let track_id = {
            let db = conn.lock().expect("lock db");
            let feed_id = create_feed(&db, "https://example.test/feed.xml")?;
            create_library_track(&db, feed_id)?
        };

        let outcome =
            RemoveFromLibrary::new(Arc::clone(&conn), LibraryRemovalTarget::Track(track_id))
                .execute(&CommandContext::next())?;

        let db = conn.lock().expect("lock db");
        let track = db::track_row_by_id(&db, track_id)?.expect("track");
        assert!(!track.is_in_library);
        assert_eq!(outcome.value().message(), "Removed track");
        assert_eq!(
            outcome.events(),
            &[
                ApplicationEvent::Download(DownloadEvent::Changed),
                ApplicationEvent::Library(LibraryEvent::Changed),
                ApplicationEvent::Feed(FeedEvent::Changed),
            ]
        );
        Ok(())
    }

    #[test]
    fn remove_from_library_feed_mutates_all_tracks() -> anyhow::Result<()> {
        let conn = Arc::new(Mutex::new(setup_test_db()?));
        let (feed_id, track_id) = {
            let db = conn.lock().expect("lock db");
            let feed_id = create_feed(&db, "https://example.test/feed.xml")?;
            db::set_feed_subscribed(&db, feed_id, true)?;
            let track_id = create_library_track(&db, feed_id)?;
            (feed_id, track_id)
        };

        let outcome =
            RemoveFromLibrary::new(Arc::clone(&conn), LibraryRemovalTarget::Feed(feed_id))
                .execute(&CommandContext::next())?;

        let db = conn.lock().expect("lock db");
        let track = db::track_row_by_id(&db, track_id)?.expect("track");
        assert!(!track.is_in_library);
        assert_eq!(outcome.value().message(), "Removed feed");
        assert_eq!(
            outcome.events(),
            &[
                ApplicationEvent::Feed(FeedEvent::Changed),
                ApplicationEvent::Library(LibraryEvent::Changed),
            ]
        );
        Ok(())
    }
}
