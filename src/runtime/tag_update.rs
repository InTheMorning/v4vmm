//! Tag update actor for ADR 0076 Decision 8 (packet 004).
//!
//! The actor owns the difference scan and the confirmed tag write. Both read
//! files, so both run on the blocking pool of the ADR 0040 runtime and never
//! on the render thread. The actor publishes a snapshot that the Music
//! section reads through its view model.
//!
//! After a confirm, the actor sends `VmEvent::TrackChanged` for each written
//! track, and then it scans again. The new count includes each file that
//! the show played or held.

#![warn(clippy::pedantic)]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use tokio::sync::{mpsc, watch};

use crate::application::commands::tag_update::{write_tag_updates, TagUpdateWriteReport};
use crate::application::queries::tag_update::{
    compare_planned_files, plan_tag_update_scan, TagUpdateFile, TagUpdateScan,
};
use crate::application::session_lifecycle::SessionLifecycle;
use crate::runtime::vm_bus::{VmBus, VmEvent};

const INBOX_CAPACITY: usize = 8;

/// The latest scan and the latest confirm of the actor.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct TagUpdateSnapshot {
    /// The latest completed scan.
    pub(crate) scan: Option<TagUpdateScan>,
    /// The error of the latest scan, when it failed.
    pub(crate) scan_error: Option<String>,
    /// `true` while a scan runs.
    pub(crate) scanning: bool,
    /// `true` while a confirm writes files.
    pub(crate) writing: bool,
    /// The result of the latest confirm.
    pub(crate) last_write: Option<TagUpdateWriteReport>,
    /// The error of the latest confirm, when it wrote no file.
    pub(crate) write_error: Option<String>,
}

#[derive(Clone, Debug)]
enum TagUpdateMessage {
    Scan {
        music_dir: PathBuf,
    },
    Confirm {
        music_dir: PathBuf,
        files: Vec<TagUpdateFile>,
    },
}

/// Caller-side handle of the actor.
#[derive(Clone, Debug)]
pub(crate) struct TagUpdateHandle {
    inbox: mpsc::Sender<TagUpdateMessage>,
    snapshot: watch::Receiver<TagUpdateSnapshot>,
}

impl TagUpdateHandle {
    /// Request a scan. Returns `false` when the actor stopped or its inbox
    /// is full.
    #[must_use]
    pub(crate) fn scan(&self, music_dir: PathBuf) -> bool {
        self.inbox
            .try_send(TagUpdateMessage::Scan { music_dir })
            .is_ok()
    }

    /// Write the listed files, and then scan again. Returns `false` when
    /// the actor stopped or its inbox is full.
    #[must_use]
    pub(crate) fn confirm(&self, music_dir: PathBuf, files: Vec<TagUpdateFile>) -> bool {
        self.inbox
            .try_send(TagUpdateMessage::Confirm { music_dir, files })
            .is_ok()
    }

    /// Subscribe to snapshots.
    #[must_use]
    pub(crate) fn subscribe(&self) -> watch::Receiver<TagUpdateSnapshot> {
        self.snapshot.clone()
    }

    /// The latest snapshot.
    #[must_use]
    pub(crate) fn latest(&self) -> TagUpdateSnapshot {
        self.snapshot.borrow().clone()
    }
}

/// Spawn the actor on the current tokio runtime.
#[must_use]
pub(crate) fn spawn(
    conn: Arc<Mutex<Connection>>,
    bus: VmBus,
    session: &SessionLifecycle,
) -> TagUpdateHandle {
    let (snapshot_tx, snapshot_rx) = watch::channel(TagUpdateSnapshot::default());
    let (inbox_tx, mut inbox_rx) = mpsc::channel::<TagUpdateMessage>(INBOX_CAPACITY);
    let stop = session.stop_token();
    session.spawn_actor("Library tag update", async move {
        loop {
            tokio::select! {
                biased;
                () = stop.cancelled() => break,
                message = inbox_rx.recv() => {
                    let Some(message) = message else {
                        break;
                    };
                    handle(&conn, &bus, &snapshot_tx, message).await;
                }
            }
        }
    });
    TagUpdateHandle {
        inbox: inbox_tx,
        snapshot: snapshot_rx,
    }
}

async fn handle(
    conn: &Arc<Mutex<Connection>>,
    bus: &VmBus,
    snapshot_tx: &watch::Sender<TagUpdateSnapshot>,
    message: TagUpdateMessage,
) {
    match message {
        TagUpdateMessage::Scan { music_dir } => scan(conn, snapshot_tx, music_dir).await,
        TagUpdateMessage::Confirm { music_dir, files } => {
            snapshot_tx.send_modify(|snapshot| {
                snapshot.writing = true;
                snapshot.write_error = None;
            });
            let conn_for_write = Arc::clone(conn);
            let bus = bus.clone();
            let written = tokio::task::spawn_blocking(move || {
                write_tag_updates(&conn_for_write, &files, |track_id| {
                    bus.publish(VmEvent::TrackChanged { track_id });
                })
                .map_err(|error| format!("{error:#}"))
            })
            .await
            .unwrap_or_else(|error| Err(format!("The tag write stopped: {error}")));
            // The rescan starts in the same snapshot that ends the write. A
            // snapshot with neither flag set would show the count of the
            // earlier scan after the files changed.
            snapshot_tx.send_modify(|snapshot| {
                snapshot.writing = false;
                snapshot.scanning = true;
                match written {
                    Ok(report) => snapshot.last_write = Some(report),
                    Err(error) => snapshot.write_error = Some(error),
                }
            });
            scan(conn, snapshot_tx, music_dir).await;
        }
    }
}

async fn scan(
    conn: &Arc<Mutex<Connection>>,
    snapshot_tx: &watch::Sender<TagUpdateSnapshot>,
    music_dir: PathBuf,
) {
    snapshot_tx.send_modify(|snapshot| snapshot.scanning = true);
    let conn = Arc::clone(conn);
    let result = tokio::task::spawn_blocking(move || {
        // The database lock is held only for the plan step. The file reads
        // of the compare step hold no lock.
        let planned = {
            let conn = conn
                .lock()
                .map_err(|_| "The database lock is not available.".to_owned())?;
            plan_tag_update_scan(&conn, &music_dir).map_err(|error| format!("{error:#}"))?
        };
        Ok::<_, String>(compare_planned_files(planned))
    })
    .await
    .unwrap_or_else(|error| Err(format!("The tag scan stopped: {error}")));
    snapshot_tx.send_modify(|snapshot| {
        snapshot.scanning = false;
        match result {
            Ok(scan) => {
                snapshot.scan = Some(scan);
                snapshot.scan_error = None;
            }
            Err(error) => snapshot.scan_error = Some(error),
        }
    });
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::timeout;

    use super::*;
    use crate::application::queries::tag_update::test_support::library;

    async fn wait_for(
        receiver: &mut watch::Receiver<TagUpdateSnapshot>,
        done: impl Fn(&TagUpdateSnapshot) -> bool,
    ) -> TagUpdateSnapshot {
        loop {
            {
                let snapshot = receiver.borrow_and_update();
                if done(&snapshot) {
                    return snapshot.clone();
                }
            }
            timeout(Duration::from_secs(5), receiver.changed())
                .await
                .expect("the actor publishes before the timeout")
                .expect("the actor keeps the watch channel open");
        }
    }

    /// R4-10: the scan runs in the runtime actor on the blocking pool. A
    /// confirm writes the files, sends `VmEvent::TrackChanged` for each
    /// written track, and then scans again.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn adr_0076_tag_update_actor_scans_writes_and_sends_track_changed() {
        let dir = tempfile::tempdir().unwrap();
        let conn = Arc::new(Mutex::new(library(dir.path(), &[1, 2])));
        let bus = VmBus::new();
        let mut events = bus.subscribe();
        let session = SessionLifecycle::new();
        let handle = spawn(Arc::clone(&conn), bus.clone(), &session);
        let mut receiver = handle.subscribe();

        assert!(handle.scan(dir.path().to_path_buf()));
        let scanned = wait_for(&mut receiver, |snapshot| {
            snapshot.scan.is_some() && !snapshot.scanning
        })
        .await;
        let files = scanned.scan.unwrap().files;
        assert_eq!(files.len(), 2);

        assert!(handle.confirm(dir.path().to_path_buf(), files));
        let confirmed = wait_for(&mut receiver, |snapshot| {
            snapshot.last_write.is_some() && !snapshot.writing && !snapshot.scanning
        })
        .await;

        assert_eq!(confirmed.last_write.unwrap().written_count(), 2);
        assert_eq!(confirmed.scan.unwrap().count(), 0);
        let mut changed = Vec::new();
        while let Ok(VmEvent::TrackChanged { track_id }) = events.try_recv() {
            changed.push(track_id);
        }
        assert_eq!(changed, vec![1, 2]);
    }
}
