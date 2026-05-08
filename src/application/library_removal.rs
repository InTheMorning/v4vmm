//! UI-agnostic planning for removals from the local library.
//!
//! All user-facing "remove from library" affordances should resolve to this
//! plan before mutating state so playlist playback impact is handled
//! consistently across Library and Discover.

#![warn(clippy::pedantic)]

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::{db, library_service};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LibraryRemovalIntent {
    TrackId(i64),
    FeedId(i64),
    TrackMatch {
        feed_url: Option<String>,
        item_guid: Option<String>,
        enclosure_url: Option<String>,
    },
    FeedUrl(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LibraryRemovalTarget {
    Track(i64),
    Feed(i64),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LibraryRemovalImpact {
    Track { playlist_reference_count: i64 },
    Feed { playlist_track_count: i64 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LibraryRemovalPlan {
    target: LibraryRemovalTarget,
    impact: LibraryRemovalImpact,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibraryRemovalExecution {
    target: LibraryRemovalTarget,
    message: &'static str,
    feed_changed: bool,
}

impl LibraryRemovalPlan {
    #[must_use]
    pub const fn new(target: LibraryRemovalTarget, impact: LibraryRemovalImpact) -> Self {
        Self { target, impact }
    }

    #[must_use]
    pub const fn target(self) -> LibraryRemovalTarget {
        self.target
    }

    #[must_use]
    pub const fn impact(self) -> LibraryRemovalImpact {
        self.impact
    }

    #[must_use]
    pub const fn requires_confirmation(self) -> bool {
        match self.impact {
            LibraryRemovalImpact::Track {
                playlist_reference_count,
            } => playlist_reference_count > 0,
            LibraryRemovalImpact::Feed {
                playlist_track_count,
            } => playlist_track_count > 0,
        }
    }
}

impl LibraryRemovalExecution {
    #[must_use]
    pub const fn new(
        target: LibraryRemovalTarget,
        message: &'static str,
        feed_changed: bool,
    ) -> Self {
        Self {
            target,
            message,
            feed_changed,
        }
    }

    #[must_use]
    pub const fn target(&self) -> LibraryRemovalTarget {
        self.target
    }

    #[must_use]
    pub const fn message(&self) -> &'static str {
        self.message
    }

    #[must_use]
    pub const fn feed_changed(&self) -> bool {
        self.feed_changed
    }
}

/// Resolve a removal intent to a canonical local target and playlist impact.
///
/// # Errors
///
/// Returns an error when the local target cannot be resolved or playlist
/// impact cannot be queried.
pub fn plan_library_removal(
    conn: &Connection,
    intent: &LibraryRemovalIntent,
) -> Result<LibraryRemovalPlan> {
    match intent {
        LibraryRemovalIntent::TrackId(track_id) => plan_track_removal(conn, *track_id),
        LibraryRemovalIntent::FeedId(feed_id) => plan_feed_removal(conn, *feed_id),
        LibraryRemovalIntent::TrackMatch {
            feed_url,
            item_guid,
            enclosure_url,
        } => {
            let track_id = library_service::find_track_id(
                conn,
                feed_url.as_deref(),
                item_guid.as_deref(),
                enclosure_url.as_deref(),
            )?
            .context("track is not present in the local library")?;
            plan_track_removal(conn, track_id)
        }
        LibraryRemovalIntent::FeedUrl(feed_url) => {
            let feed_id = db::feed_id_by_url(conn, feed_url)?
                .with_context(|| format!("feed is not present in the local library: {feed_url}"))?;
            plan_feed_removal(conn, feed_id)
        }
    }
}

fn plan_track_removal(conn: &Connection, track_id: i64) -> Result<LibraryRemovalPlan> {
    let playlist_reference_count =
        library_service::playlist_reference_count_for_track(conn, track_id)?;
    Ok(LibraryRemovalPlan::new(
        LibraryRemovalTarget::Track(track_id),
        LibraryRemovalImpact::Track {
            playlist_reference_count,
        },
    ))
}

fn plan_feed_removal(conn: &Connection, feed_id: i64) -> Result<LibraryRemovalPlan> {
    let playlist_track_count =
        library_service::playlist_referenced_library_track_count_for_feed(conn, feed_id)?;
    Ok(LibraryRemovalPlan::new(
        LibraryRemovalTarget::Feed(feed_id),
        LibraryRemovalImpact::Feed {
            playlist_track_count,
        },
    ))
}

/// Execute a planned local-library removal target.
///
/// # Errors
///
/// Returns an error when the library target cannot be mutated.
pub fn execute_library_removal(
    conn: &Connection,
    target: LibraryRemovalTarget,
) -> Result<LibraryRemovalExecution> {
    match target {
        LibraryRemovalTarget::Track(track_id) => execute_track_removal(conn, track_id),
        LibraryRemovalTarget::Feed(feed_id) => execute_feed_removal(conn, feed_id),
    }
}

fn execute_track_removal(conn: &Connection, track_id: i64) -> Result<LibraryRemovalExecution> {
    let feed_url = match library_service::track_row_by_id(conn, track_id)? {
        Some(track) => db::feed_url_by_id(conn, track.feed_id)?,
        None => None,
    };
    library_service::set_track_in_library(conn, track_id, false)?;
    let feed_changed = if let Some(feed_url) = feed_url.as_deref() {
        db::reconcile_feed_subscription_by_url(conn, feed_url)?;
        true
    } else {
        false
    };
    Ok(LibraryRemovalExecution::new(
        LibraryRemovalTarget::Track(track_id),
        "Removed track",
        feed_changed,
    ))
}

fn execute_feed_removal(conn: &Connection, feed_id: i64) -> Result<LibraryRemovalExecution> {
    db::set_feed_subscribed(conn, feed_id, false)?;
    db::unsubscribe_feed_tracks(conn, feed_id)?;
    Ok(LibraryRemovalExecution::new(
        LibraryRemovalTarget::Feed(feed_id),
        "Removed feed",
        true,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Result<Connection> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        db::init_schema(&conn)?;
        db::migrate_schema(&conn)?;
        Ok(conn)
    }

    #[test]
    fn removal_plan_resolves_match_to_track_impact() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn, "https://example.test/feed.xml")?;
        let track_id = create_track(&conn, feed_id, "item", "https://example.test/audio.mp3")?;
        library_service::set_track_in_library(&conn, track_id, true)?;
        let playlist_id = db::playlist_create(&conn, "Mix")?;
        db::playlist_append(&conn, playlist_id, track_id)?;

        let plan = plan_library_removal(
            &conn,
            &LibraryRemovalIntent::TrackMatch {
                feed_url: Some("https://example.test/feed.xml".into()),
                item_guid: Some("item".into()),
                enclosure_url: None,
            },
        )?;

        assert_eq!(plan.target(), LibraryRemovalTarget::Track(track_id));
        assert_eq!(
            plan.impact(),
            LibraryRemovalImpact::Track {
                playlist_reference_count: 1
            }
        );
        assert!(plan.requires_confirmation());
        Ok(())
    }

    #[test]
    fn removal_plan_resolves_feed_url_to_feed_impact() -> Result<()> {
        let conn = setup_test_db()?;
        let feed_id = create_feed(&conn, "https://example.test/feed.xml")?;
        let track_id = create_track(&conn, feed_id, "item", "https://example.test/audio.mp3")?;
        library_service::set_track_in_library(&conn, track_id, true)?;
        let playlist_id = db::playlist_create(&conn, "Mix")?;
        db::playlist_append(&conn, playlist_id, track_id)?;

        let plan = plan_library_removal(
            &conn,
            &LibraryRemovalIntent::FeedUrl("https://example.test/feed.xml".into()),
        )?;

        assert_eq!(plan.target(), LibraryRemovalTarget::Feed(feed_id));
        assert_eq!(
            plan.impact(),
            LibraryRemovalImpact::Feed {
                playlist_track_count: 1
            }
        );
        assert!(plan.requires_confirmation());
        Ok(())
    }

    fn create_feed(conn: &Connection, feed_url: &str) -> Result<i64> {
        conn.execute(
            "INSERT INTO feeds (feed_url, feed_guid, title)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![feed_url, "feed-guid", "Feed Title"],
        )?;
        Ok(conn.last_insert_rowid())
    }

    fn create_track(
        conn: &Connection,
        feed_id: i64,
        item_guid: &str,
        enclosure_url: &str,
    ) -> Result<i64> {
        conn.execute(
            "INSERT INTO tracks (feed_id, item_guid, enclosure_url, track_title)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![feed_id, item_guid, enclosure_url, "Track Title"],
        )?;
        Ok(conn.last_insert_rowid())
    }
}
