//! Playback polling actor (ADR 0040).
//!
//! The GPUI screen starts this actor when a live playback driver is
//! configured. The actor owns the 1Hz polling cadence and publishes plain
//! snapshots that the screen reduces into status text and renders.

#![warn(clippy::pedantic)]

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::application::session_lifecycle::SessionLifecycle;
use rusqlite::Connection;

use tokio::sync::{oneshot, watch};

use crate::playback_driver::PlaybackDriver;
use crate::playback_owner::{PlaybackOwner, PollOutcome};

const PLAYBACK_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Snapshot published after each playback polling tick.
#[derive(Clone, Debug)]
pub struct PlaybackTickSnapshot {
    /// Capture time for ordering/debugging. The screen can ignore it.
    pub at: Instant,
    /// Polling outcome reduced from `PlaybackOwner::poll`.
    pub outcome: PlaybackTickOutcome,
}

impl PlaybackTickSnapshot {
    fn new(outcome: PlaybackTickOutcome) -> Self {
        Self {
            at: Instant::now(),
            outcome,
        }
    }
}

/// Plain playback polling outcome for GPUI reduction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlaybackTickOutcome {
    /// No active session or no visible playback advancement.
    Idle,
    /// Now-playing state advanced or reconciled and status should clear.
    Advanced,
    /// The playback session changed to the state `stopped`. The screen
    /// refreshes Show and requests one tag update scan (ADR 0076 packet
    /// 005). The next tick of the same stopped session is `Idle`.
    SessionStopped,
    /// Polling failed; the screen should show the playback error.
    Error(String),
}

/// Caller-side handle for the playback polling actor.
pub struct PlaybackPollingHandle {
    snapshot: watch::Receiver<PlaybackTickSnapshot>,
    shutdown: Option<oneshot::Sender<()>>,
}

impl PlaybackPollingHandle {
    /// Subscribe to playback polling snapshots.
    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<PlaybackTickSnapshot> {
        self.snapshot.clone()
    }
}

impl Drop for PlaybackPollingHandle {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}

/// Spawns the playback polling actor on the current tokio runtime.
#[must_use]
pub fn spawn<D>(
    playback_owner: Arc<Mutex<PlaybackOwner<D>>>,
    conn: Arc<Mutex<Connection>>,
    session: &SessionLifecycle,
) -> PlaybackPollingHandle
where
    D: PlaybackDriver + 'static,
{
    let (snapshot_tx, snapshot_rx) =
        watch::channel(PlaybackTickSnapshot::new(PlaybackTickOutcome::Idle));
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

    let stop = session.stop_token();
    session.spawn_actor("Built-in playback polling", async move {
        loop {
            tokio::select! {
                biased;
                () = stop.cancelled() => break,
                _ = &mut shutdown_rx => break,
                () = tokio::time::sleep(PLAYBACK_POLL_INTERVAL) => {
                    let outcome =
                        poll_playback_owner(Arc::clone(&playback_owner), Arc::clone(&conn)).await;
                    let _ = snapshot_tx.send(PlaybackTickSnapshot::new(outcome));
                }
            }
        }
    });

    PlaybackPollingHandle {
        snapshot: snapshot_rx,
        shutdown: Some(shutdown_tx),
    }
}

async fn poll_playback_owner<D>(
    playback_owner: Arc<Mutex<PlaybackOwner<D>>>,
    conn: Arc<Mutex<Connection>>,
) -> PlaybackTickOutcome
where
    D: PlaybackDriver + 'static,
{
    tokio::task::spawn_blocking(move || poll_playback_owner_blocking(&playback_owner, &conn))
        .await
        .unwrap_or_else(|error| PlaybackTickOutcome::Error(format!("{error:#}")))
}

fn poll_playback_owner_blocking<D>(
    playback_owner: &Arc<Mutex<PlaybackOwner<D>>>,
    conn: &Arc<Mutex<Connection>>,
) -> PlaybackTickOutcome
where
    D: PlaybackDriver,
{
    let Ok(conn) = conn.lock() else {
        return PlaybackTickOutcome::Error("database lock poisoned".to_string());
    };
    let Ok(mut playback_owner) = playback_owner.lock() else {
        return PlaybackTickOutcome::Error("playback owner lock poisoned".to_string());
    };

    match playback_owner.poll(&conn) {
        Ok(PollOutcome::NoSession | PollOutcome::Reconciled(None)) => PlaybackTickOutcome::Idle,
        Ok(PollOutcome::Reconciled(Some(_)) | PollOutcome::Advanced(_)) => {
            PlaybackTickOutcome::Advanced
        }
        Ok(PollOutcome::Stopped) => PlaybackTickOutcome::SessionStopped,
        Err(error) => PlaybackTickOutcome::Error(format!("{error:#}")),
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use anyhow::{anyhow, Result};

    use super::*;
    use crate::db;
    use crate::playback;
    use crate::playback_driver::{DriverStatus, NullDriver};

    fn setup_test_db() -> Result<Connection> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        db::init_schema(&conn)?;
        db::migrate_schema(&conn)?;
        Ok(conn)
    }

    fn create_downloaded_track(conn: &Connection) -> Result<i64> {
        conn.execute(
            "INSERT INTO feeds (
                 feed_url, feed_guid, title, album_image_href, podcast_value_json
             )
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                "https://example.test/feed.xml",
                "feed-guid",
                "Feed Title",
                "https://example.test/feed.png",
                r#"{"feed":"value"}"#
            ],
        )?;
        let feed_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO tracks (
                 feed_id, item_guid, track_title, artist_name, album_title,
                 duration_seconds, item_value_json, extra_json
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                feed_id,
                "item-guid",
                "Track",
                "Artist",
                "Album",
                123_i64,
                r#"{"item":"value"}"#,
                r#"{"source":"rss"}"#
            ],
        )?;
        let track_id = conn.last_insert_rowid();
        let relative_path = crate::library_path::LibraryRelativePath::for_test("tmp/track.mp3");
        db::mark_track_downloaded(conn, track_id, &relative_path, None)?;
        Ok(track_id)
    }

    #[tokio::test]
    async fn poll_without_session_is_idle() -> Result<()> {
        let conn = Arc::new(Mutex::new(setup_test_db()?));
        let playback_owner = Arc::new(Mutex::new(PlaybackOwner::new(
            NullDriver::new(),
            playback::DEFAULT_SESSION_ID,
            PathBuf::from("/"),
        )));

        let outcome = poll_playback_owner(Arc::clone(&playback_owner), Arc::clone(&conn)).await;

        assert_eq!(outcome, PlaybackTickOutcome::Idle);
        Ok(())
    }

    #[tokio::test]
    async fn poll_reconciled_update_is_advanced() -> Result<()> {
        let conn = Arc::new(Mutex::new(setup_test_db()?));
        {
            let conn = conn.lock().expect("lock db");
            let track_id = create_downloaded_track(&conn)?;
            playback::set_track(&conn, track_id, playback::DEFAULT_SESSION_ID)?;
        };
        let playback_owner = Arc::new(Mutex::new(PlaybackOwner::new(
            NullDriver::new(),
            playback::DEFAULT_SESSION_ID,
            PathBuf::from("/"),
        )));

        let outcome = poll_playback_owner(Arc::clone(&playback_owner), Arc::clone(&conn)).await;

        assert_eq!(outcome, PlaybackTickOutcome::Advanced);
        assert_eq!(
            playback_owner
                .lock()
                .expect("lock playback owner")
                .driver()
                .snapshot()
                .loaded_path
                .as_deref(),
            Some(Path::new("/tmp/track.mp3"))
        );
        Ok(())
    }

    /// R5-04: a session change to `stopped` gives one `SessionStopped`
    /// tick. `TopApp` requests one tag update scan for that tick.
    #[tokio::test]
    async fn adr_0076_follow_up_session_stop_starts_one_tag_scan() -> Result<()> {
        let conn = Arc::new(Mutex::new(setup_test_db()?));
        {
            let conn = conn.lock().expect("lock db");
            let track_id = create_downloaded_track(&conn)?;
            playback::set_track(&conn, track_id, playback::DEFAULT_SESSION_ID)?;
        }
        let playback_owner = Arc::new(Mutex::new(PlaybackOwner::new(
            NullDriver::new(),
            playback::DEFAULT_SESSION_ID,
            PathBuf::from("/"),
        )));
        let first = poll_playback_owner(Arc::clone(&playback_owner), Arc::clone(&conn)).await;
        assert_eq!(first, PlaybackTickOutcome::Advanced);

        // Another process, such as the CLI, stops the session.
        playback::stop(&conn.lock().expect("lock db"), playback::DEFAULT_SESSION_ID)?;
        let mut outcomes = Vec::new();
        for _ in 0..3 {
            outcomes
                .push(poll_playback_owner(Arc::clone(&playback_owner), Arc::clone(&conn)).await);
        }
        assert_eq!(
            outcomes,
            [
                PlaybackTickOutcome::SessionStopped,
                PlaybackTickOutcome::Idle,
                PlaybackTickOutcome::Idle
            ]
        );

        // `TopApp` maps `SessionStopped` to one scan request. No GPUI test
        // drives `TopApp`, so the test reads the reducer arm.
        let app = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/app.rs"),
        )?;
        let arm = app
            .split("PlaybackTickOutcome::SessionStopped =>")
            .nth(1)
            .and_then(|rest| rest.split("PlaybackTickOutcome::").next())
            .expect("apply_playback_tick has a SessionStopped arm");
        assert_eq!(arm.matches("scan_tag_updates()").count(), 1, "{arm}");
        assert_eq!(app.matches("scan_tag_updates()").count(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn poll_error_maps_to_error_outcome() -> Result<()> {
        let conn = Arc::new(Mutex::new(setup_test_db()?));
        {
            let conn = conn.lock().expect("lock db");
            let track_id = create_downloaded_track(&conn)?;
            playback::set_track(&conn, track_id, playback::DEFAULT_SESSION_ID)?;
        }
        let playback_owner = Arc::new(Mutex::new(PlaybackOwner::new(
            FailingDriver,
            playback::DEFAULT_SESSION_ID,
            PathBuf::from("/"),
        )));

        let outcome = poll_playback_owner(Arc::clone(&playback_owner), Arc::clone(&conn)).await;

        assert!(matches!(
            outcome,
            PlaybackTickOutcome::Error(message) if message.contains("driver load failed")
        ));
        Ok(())
    }

    struct FailingDriver;

    impl PlaybackDriver for FailingDriver {
        fn load(&self, _path: &Path, _start_ms: u64) -> Result<()> {
            Err(anyhow!("driver load failed"))
        }

        fn seek(&self, _position_ms: u64) -> Result<()> {
            Ok(())
        }

        fn pause(&self, _paused: bool) -> Result<()> {
            Ok(())
        }

        fn stop(&self) -> Result<()> {
            Ok(())
        }

        fn poll(&self) -> Result<DriverStatus> {
            Ok(DriverStatus::default())
        }
    }
}
