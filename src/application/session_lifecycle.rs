//! Admission, resource release and owned maintenance authority (ADR 0066, invariants 5–6).

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Condvar, Mutex,
};
use std::time::{Duration, Instant};

use rusqlite::Connection;
use tokio_util::sync::CancellationToken;

use crate::playback_driver::ConfiguredPlaybackDriver;
use crate::playback_owner::PlaybackOwner;

pub(crate) const SESSION_DRAIN_TIMEOUT: Duration = Duration::from_secs(5);
static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionPhase {
    Running,
    Draining,
    Maintenance,
    Resuming,
}

#[derive(Debug)]
struct State {
    phase: SessionPhase,
    work: BTreeMap<&'static str, usize>,
    owners: BTreeMap<&'static str, usize>,
    failed: bool,
}

#[derive(Debug)]
struct Shared {
    generation: u64,
    state: Mutex<State>,
    changed: Condvar,
    stop: CancellationToken,
}

/// Shared admission for every command runner and actor in one app session.
#[derive(Clone, Debug)]
pub struct SessionLifecycle(Arc<Shared>);

impl Default for SessionLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionLifecycle {
    #[must_use]
    pub fn new() -> Self {
        Self(Arc::new(Shared {
            generation: NEXT_GENERATION.fetch_add(1, Ordering::Relaxed),
            state: Mutex::new(State {
                phase: SessionPhase::Running,
                work: BTreeMap::new(),
                owners: BTreeMap::new(),
                failed: false,
            }),
            changed: Condvar::new(),
            stop: CancellationToken::new(),
        }))
    }

    #[must_use]
    pub fn generation(&self) -> u64 {
        self.0.generation
    }

    /// Current phase.
    ///
    /// # Panics
    /// Panics if a programmer failure poisoned the session mutex.
    #[must_use]
    pub fn phase(&self) -> SessionPhase {
        self.0.state.lock().expect("session state").phase
    }

    #[must_use]
    pub fn accepts(&self, generation: u64) -> bool {
        self.generation() == generation && self.phase() == SessionPhase::Running
    }

    pub(crate) fn admit(&self, name: &'static str) -> Option<SessionWork> {
        self.register(name, false)
    }

    pub(crate) fn own(&self, name: &'static str) -> Option<SessionWork> {
        self.register(name, true)
    }

    fn register(&self, name: &'static str, owner: bool) -> Option<SessionWork> {
        let mut state = self.0.state.lock().expect("session state");
        if state.phase != SessionPhase::Running {
            return None;
        }
        let counts = if owner {
            &mut state.owners
        } else {
            &mut state.work
        };
        *counts.entry(name).or_default() += 1;
        Some(SessionWork {
            session: self.clone(),
            name,
            owner,
        })
    }

    pub(crate) fn begin_drain(&self) -> bool {
        let mut state = self.0.state.lock().expect("session state");
        if state.phase != SessionPhase::Running {
            return false;
        }
        state.phase = SessionPhase::Draining;
        self.0.stop.cancel();
        self.0.changed.notify_all();
        true
    }

    fn failed_owner(&self, name: &str) -> Vec<String> {
        self.0.state.lock().expect("session state").failed = true;
        vec![name.to_owned()]
    }

    pub(crate) fn stop_token(&self) -> CancellationToken {
        self.0.stop.clone()
    }

    /// Spawn an admitted actor; completion acknowledges that its future and resources dropped.
    pub(crate) fn spawn_actor<F>(&self, name: &'static str, future: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let Some(work) = self.admit(name) else {
            return;
        };
        tokio::spawn(async move {
            future.await;
            drop(work);
        });
    }

    pub(crate) fn wait_for_work(&self, timeout: Duration) -> Result<(), Vec<String>> {
        let deadline = Instant::now() + timeout;
        let mut state = self.0.state.lock().expect("session state");
        while !state.work.is_empty() && !state.failed {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            state = self
                .0
                .changed
                .wait_timeout(state, remaining)
                .expect("session wait")
                .0;
        }
        if state.work.is_empty() && !state.failed {
            return Ok(());
        }
        let mut pending = counts(&state.work);
        if state.failed {
            pending.push("A session worker failed before acknowledging completion".into());
        }
        Err(pending)
    }
}

/// The lifetime of admitted work; dropping a receiver never drops this guard.
#[derive(Debug)]
pub(crate) struct SessionWork {
    session: SessionLifecycle,
    name: &'static str,
    owner: bool,
}

impl Drop for SessionWork {
    fn drop(&mut self) {
        let mut state = self.session.0.state.lock().expect("session state");
        if std::thread::panicking() {
            state.failed = true;
        }
        let counts = if self.owner {
            &mut state.owners
        } else {
            &mut state.work
        };
        if let Some(count) = counts.get_mut(self.name) {
            *count -= 1;
            if *count == 0 {
                counts.remove(self.name);
            }
        }
        self.session.0.changed.notify_all();
    }
}

fn counts(values: &BTreeMap<&'static str, usize>) -> Vec<String> {
    values
        .iter()
        .map(|(name, count)| format!("{name}: {count}"))
        .collect()
}

/// Retains database/playback ownership until every release is acknowledged.
pub(crate) struct SessionDrain {
    pub(crate) session: SessionLifecycle,
    connection: Option<Arc<Mutex<Connection>>>,
    closing: Option<Connection>,
    playback: Option<Arc<Mutex<PlaybackOwner<ConfiguredPlaybackDriver>>>>,
}

impl SessionDrain {
    pub(crate) fn new(
        session: SessionLifecycle,
        connection: Arc<Mutex<Connection>>,
        playback: Option<Arc<Mutex<PlaybackOwner<ConfiguredPlaybackDriver>>>>,
    ) -> Self {
        Self {
            session,
            connection: Some(connection),
            closing: None,
            playback,
        }
    }

    /// Runs on the independent worker after normal children and runtime owners release.
    pub(crate) fn finish(&mut self) -> Result<MaintenanceSession, Vec<String>> {
        self.session.wait_for_work(Duration::ZERO)?;
        if self.session.phase() != SessionPhase::Draining {
            return Err(vec!["App session is not draining".into()]);
        }
        if let Some(connection) = self.connection.take() {
            match Arc::try_unwrap(connection) {
                Ok(connection) => {
                    self.closing = Some(connection.into_inner().map_err(|_| {
                        self.session.failed_owner("Configured database lock failed")
                    })?);
                }
                Err(connection) => {
                    self.connection = Some(connection);
                    return Err(vec![
                        "Configured database still has an outstanding connection owner".into(),
                    ]);
                }
            }
        }
        if let Some(playback) = self.playback.take() {
            match Arc::try_unwrap(playback) {
                Ok(owner) => {
                    let mut owner = owner
                        .into_inner()
                        .map_err(|_| self.session.failed_owner("Built-in playback lock failed"))?;
                    if let Err(error) = owner.clear_broadcast_drop_file() {
                        self.playback = Some(Arc::new(Mutex::new(owner)));
                        return Err(vec![format!(
                            "Built-in playback could not remove its now-playing file: {}",
                            crate::diagnostics::redact_endpoint_details(&format!("{error:#}"))
                        )]);
                    }
                    if let Err(error) = owner.driver().shutdown_for_maintenance() {
                        self.playback = Some(Arc::new(Mutex::new(owner)));
                        return Err(vec![format!(
                            "App could not stop its built-in player: {}",
                            crate::diagnostics::redact_endpoint_details(&format!("{error:#}"))
                        )]);
                    }
                    if let Err(error) = owner.finish_session_playback(
                        self.closing
                            .as_ref()
                            .expect("drain owns the configured connection"),
                    ) {
                        self.playback = Some(Arc::new(Mutex::new(owner)));
                        return Err(vec![format!(
                            "App could not record stopped playback: {}",
                            crate::diagnostics::redact_endpoint_details(&format!("{error:#}"))
                        )]);
                    }
                    drop(owner);
                }
                Err(owner) => {
                    self.playback = Some(owner);
                    return Err(vec![
                        "Built-in playback still has an outstanding owner".into()
                    ]);
                }
            }
        }
        if let Some(connection) = self.closing.take() {
            if let Err((connection, error)) = connection.close() {
                self.closing = Some(connection);
                return Err(vec![format!(
                    "Configured database could not close: {error}"
                )]);
            }
        }
        let mut state = self.session.0.state.lock().expect("session state");
        if !state.owners.is_empty() || !state.work.is_empty() || state.failed {
            let mut remaining = counts(&state.owners);
            remaining.extend(counts(&state.work));
            if state.failed {
                remaining.push("A session owner failed to release".into());
            }
            return Err(remaining);
        }
        state.phase = SessionPhase::Maintenance;
        Ok(MaintenanceSession {
            session: self.session.clone(),
        })
    }
}

/// Unique authority produced only by a completed managed drain.
#[derive(Debug)]
pub(crate) struct MaintenanceSession {
    session: SessionLifecycle,
}

impl MaintenanceSession {
    pub(crate) fn generation(&self) -> u64 {
        self.session.generation()
    }

    pub(crate) fn begin_resume(&mut self) -> bool {
        let mut state = self.session.0.state.lock().expect("session state");
        if state.phase != SessionPhase::Maintenance {
            return false;
        }
        state.phase = SessionPhase::Resuming;
        true
    }

    pub(crate) fn resume_failed(&mut self) {
        self.session.0.state.lock().expect("session state").phase = SessionPhase::Maintenance;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{atomic::AtomicBool, mpsc, Barrier};

    #[test]
    fn adr_0066_admission_race_tracks_every_accepted_operation() {
        let session = SessionLifecycle::new();
        let barrier = Arc::new(Barrier::new(33));
        let held = session.admit("Already admitted writer").unwrap();
        let accepted = std::thread::scope(|scope| {
            let threads: Vec<_> = (0..32)
                .map(|_| {
                    let session = session.clone();
                    let barrier = barrier.clone();
                    scope.spawn(move || {
                        barrier.wait();
                        session.admit("Racing writer")
                    })
                })
                .collect();
            barrier.wait();
            assert!(session.begin_drain());
            threads
                .into_iter()
                .filter_map(|thread| thread.join().unwrap())
                .collect::<Vec<_>>()
        });
        let active: usize = session.0.state.lock().unwrap().work.values().sum();
        assert_eq!(active, accepted.len() + 1);
        assert!(session.admit("Late writer").is_none());
        assert!(session.wait_for_work(Duration::ZERO).is_err());
        drop(accepted);
        drop(held);
        assert_eq!(session.wait_for_work(Duration::ZERO), Ok(()));
    }

    #[test]
    fn adr_0066_held_connection_and_resource_owner_prevent_maintenance() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("library.sqlite");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch("CREATE TABLE preserved(value); INSERT INTO preserved VALUES (42)")
            .unwrap();
        let connection = Arc::new(Mutex::new(connection));
        let held = connection.clone();
        let session = SessionLifecycle::new();
        let runtime = session.own("Held runtime owner").unwrap();
        let mut drain = SessionDrain::new(session.clone(), connection, None);
        assert!(session.begin_drain());
        assert!(drain
            .finish()
            .unwrap_err()
            .join(" ")
            .contains("outstanding connection"));
        assert_eq!(session.phase(), SessionPhase::Draining);
        assert_eq!(
            held.lock()
                .unwrap()
                .query_row("SELECT value FROM preserved", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            42
        );
        drop(held);
        assert!(drain
            .finish()
            .unwrap_err()
            .join(" ")
            .contains("Held runtime owner"));
        drop(runtime);
        let mut maintenance = drain.finish().unwrap();
        assert_eq!(session.phase(), SessionPhase::Maintenance);
        assert!(maintenance.begin_resume());
        assert!(!maintenance.begin_resume());
        maintenance.resume_failed();
        assert_eq!(session.phase(), SessionPhase::Maintenance);
        assert!(!session.accepts(session.generation()));
        assert!(maintenance.begin_resume());
        let fresh = SessionLifecycle::new();
        assert!(fresh.generation() > session.generation());
        assert!(!fresh.accepts(session.generation()));
        assert_eq!(
            Connection::open(path)
                .unwrap()
                .query_row("SELECT value FROM preserved", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            42
        );
    }

    #[test]
    fn adr_0066_actor_acknowledgement_follows_its_extra_connection_close() {
        struct ExtraConnection {
            connection: Option<Connection>,
            closed: Arc<AtomicBool>,
        }
        impl Drop for ExtraConnection {
            fn drop(&mut self) {
                self.connection.take().unwrap().close().unwrap();
                self.closed.store(true, Ordering::Release);
            }
        }
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let session = SessionLifecycle::new();
        let closed = Arc::new(AtomicBool::new(false));
        let extra = ExtraConnection {
            connection: Some(Connection::open_in_memory().unwrap()),
            closed: closed.clone(),
        };
        let (release, hold) = mpsc::channel();
        let (started, running) = mpsc::channel();
        {
            let _entered = runtime.enter();
            session.spawn_actor("Held actor connection", async move {
                let extra = extra;
                started.send(()).unwrap();
                hold.recv().unwrap();
                drop(extra);
            });
        }
        running.recv().unwrap();
        session.begin_drain();
        let before = Instant::now();
        assert_eq!(
            session
                .wait_for_work(Duration::from_millis(20))
                .unwrap_err(),
            ["Held actor connection: 1"]
        );
        assert!(before.elapsed() < Duration::from_secs(1));
        assert!(!closed.load(Ordering::Acquire));
        release.send(()).unwrap();
        session.wait_for_work(Duration::from_secs(1)).unwrap();
        assert!(closed.load(Ordering::Acquire));
    }

    #[test]
    fn adr_0066_failed_worker_cannot_authorize_maintenance() {
        let session = SessionLifecycle::new();
        let work = session.admit("Failed actor").unwrap();
        assert!(std::thread::spawn(move || {
            let _work = work;
            panic!("injected worker failure");
        })
        .join()
        .is_err());
        session.begin_drain();
        let mut drain = SessionDrain::new(
            session.clone(),
            Arc::new(Mutex::new(Connection::open_in_memory().unwrap())),
            None,
        );
        assert!(drain.finish().is_err());
        assert!(drain.finish().is_err());
        assert_eq!(session.phase(), SessionPhase::Draining);
    }
}
