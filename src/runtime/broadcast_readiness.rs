//! Broadcast readiness watch actor for ADR 0059.
//!
//! The actor owns the blocking local file scan and publishes cached snapshots
//! so Show renders payment-route readiness without touching the filesystem.

#![warn(clippy::pedantic)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use rusqlite::Connection;
use tokio::sync::{mpsc, oneshot, watch};

use crate::application::queries::broadcast::{
    broadcast_readiness_report, BroadcastReadinessReport,
};

const BROADCAST_READINESS_POLL_INTERVAL: Duration = Duration::from_secs(300);
const INBOX_CAPACITY: usize = 4;

/// Cached local broadcast-readiness snapshot.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct BroadcastReadinessSnapshot {
    /// Latest successful readiness report.
    pub(crate) report: Option<BroadcastReadinessReport>,
    /// Latest scan error, when the report could not be refreshed.
    pub(crate) error: Option<String>,
}

/// Caller-side handle for the broadcast readiness actor.
pub(crate) struct BroadcastReadinessWatchHandle {
    snapshot: watch::Receiver<BroadcastReadinessSnapshot>,
    inbox: mpsc::Sender<BroadcastReadinessCommand>,
    shutdown: Option<oneshot::Sender<()>>,
}

impl BroadcastReadinessWatchHandle {
    /// Subscribe to readiness snapshots.
    #[must_use]
    pub(crate) fn subscribe(&self) -> watch::Receiver<BroadcastReadinessSnapshot> {
        self.snapshot.clone()
    }

    /// Return the latest cached snapshot.
    #[must_use]
    pub(crate) fn latest(&self) -> BroadcastReadinessSnapshot {
        self.snapshot.borrow().clone()
    }

    /// Request an immediate readiness scan.
    ///
    /// Returns `false` when the actor has shut down or its inbox is full.
    #[must_use]
    pub(crate) fn refresh_now(&self) -> bool {
        self.inbox
            .try_send(BroadcastReadinessCommand::RefreshNow)
            .is_ok()
    }
}

impl Drop for BroadcastReadinessWatchHandle {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BroadcastReadinessCommand {
    RefreshNow,
}

/// Spawns the broadcast readiness actor on the current tokio runtime.
#[must_use]
pub(crate) fn start(conn: Arc<Mutex<Connection>>) -> BroadcastReadinessWatchHandle {
    start_with_interval(conn, BROADCAST_READINESS_POLL_INTERVAL)
}

fn start_with_interval(
    conn: Arc<Mutex<Connection>>,
    interval: Duration,
) -> BroadcastReadinessWatchHandle {
    let (snapshot_tx, snapshot_rx) = watch::channel(BroadcastReadinessSnapshot::default());
    let (inbox_tx, mut inbox_rx) = mpsc::channel::<BroadcastReadinessCommand>(INBOX_CAPACITY);
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        publish_snapshot(&snapshot_tx, Arc::clone(&conn)).await;
        loop {
            tokio::select! {
                biased;
                _ = &mut shutdown_rx => break,
                command = inbox_rx.recv() => {
                    let Some(BroadcastReadinessCommand::RefreshNow) = command else {
                        break;
                    };
                    publish_snapshot(&snapshot_tx, Arc::clone(&conn)).await;
                }
                () = tokio::time::sleep(interval) => {
                    publish_snapshot(&snapshot_tx, Arc::clone(&conn)).await;
                }
            }
        }
    });

    BroadcastReadinessWatchHandle {
        snapshot: snapshot_rx,
        inbox: inbox_tx,
        shutdown: Some(shutdown_tx),
    }
}

async fn publish_snapshot(
    snapshot_tx: &watch::Sender<BroadcastReadinessSnapshot>,
    conn: Arc<Mutex<Connection>>,
) {
    let snapshot = read_broadcast_readiness(conn)
        .await
        .unwrap_or_else(|error| BroadcastReadinessSnapshot {
            report: None,
            error: Some(format!("{error:#}")),
        });
    let _ = snapshot_tx.send(snapshot);
}

async fn read_broadcast_readiness(
    conn: Arc<Mutex<Connection>>,
) -> Result<BroadcastReadinessSnapshot, tokio::task::JoinError> {
    tokio::task::spawn_blocking(move || read_broadcast_readiness_blocking(&conn)).await
}

fn read_broadcast_readiness_blocking(conn: &Arc<Mutex<Connection>>) -> BroadcastReadinessSnapshot {
    let report = conn
        .lock()
        .map_err(|_| "database lock poisoned".to_owned())
        .and_then(|conn| broadcast_readiness_report(&conn).map_err(|error| format!("{error:#}")));
    match report {
        Ok(report) => BroadcastReadinessSnapshot {
            report: Some(report),
            error: None,
        },
        Err(error) => BroadcastReadinessSnapshot {
            report: None,
            error: Some(error),
        },
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use tokio::time::timeout;

    use crate::db;

    use super::*;

    fn setup_test_db() -> anyhow::Result<Arc<Mutex<Connection>>> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        db::init_schema(&conn)?;
        db::migrate_schema(&conn)?;
        Ok(Arc::new(Mutex::new(conn)))
    }

    #[tokio::test(flavor = "current_thread")]
    async fn broadcast_readiness_actor_publishes_cached_report() -> anyhow::Result<()> {
        let conn = setup_test_db()?;

        let snapshot = read_broadcast_readiness(Arc::clone(&conn)).await?;

        assert_eq!(
            snapshot.report.expect("report").summary,
            crate::application::queries::broadcast::BroadcastReadinessSummary::default()
        );
        assert!(snapshot.error.is_none());
        Ok(())
    }

    #[tokio::test(flavor = "current_thread")]
    async fn broadcast_readiness_actor_refreshes_on_command() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        let handle = start_with_interval(Arc::clone(&conn), Duration::from_secs(60));
        let mut receiver = handle.subscribe();

        wait_for_report(&mut receiver).await;

        assert!(handle.refresh_now());
        wait_for_report(&mut receiver).await;
        drop(handle);
        Ok(())
    }

    async fn wait_for_report(receiver: &mut watch::Receiver<BroadcastReadinessSnapshot>) {
        loop {
            if receiver.borrow().report.is_some() {
                return;
            }
            timeout(Duration::from_secs(1), receiver.changed())
                .await
                .expect("actor should publish before timeout")
                .expect("actor should keep the watch channel open");
        }
    }
}
