#![warn(clippy::pedantic)]

//! Publisher-side service watch actor for ADR 0059.
//!
//! The actor observes service state for the local broadcast chain and publishes
//! GPUI-free snapshots over `tokio::sync::watch`. Blocking service reads run on
//! Tokio's blocking pool so the desktop render thread never waits on the rig.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, oneshot, watch};

use crate::broadcast::control::{self, ServiceState, UnitRef};
use crate::broadcast::transport::Transport;

const BROADCAST_SERVICE_POLL_INTERVAL: Duration = Duration::from_secs(1);
const INBOX_CAPACITY: usize = 8;

/// Role of one service in the broadcast chain.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BroadcastServiceRole {
    /// The process that publishes listener-facing live metadata.
    Publisher,
    /// The process that writes now-playing input for the publisher.
    Producer,
}

/// Service unit observed by the actor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BroadcastServiceWatchUnit {
    /// Role this unit plays in the chain.
    pub role: BroadcastServiceRole,
    /// Curator-facing host name for this unit.
    pub host_name: String,
    /// Transport used to read this unit.
    pub transport: Transport,
    /// User unit name to observe.
    pub unit: UnitRef,
}

impl BroadcastServiceWatchUnit {
    /// Create a watch unit.
    #[must_use]
    pub fn new(
        role: BroadcastServiceRole,
        host_name: impl Into<String>,
        transport: Transport,
        unit: UnitRef,
    ) -> Self {
        Self {
            role,
            host_name: host_name.into(),
            transport,
            unit,
        }
    }
}

/// State for one observed service unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BroadcastServiceUnitSnapshot {
    /// Role this unit plays in the chain.
    pub role: BroadcastServiceRole,
    /// Curator-facing host name for this unit.
    pub host_name: String,
    /// Complete user unit name.
    pub unit_name: String,
    /// Reduced service state.
    pub state: ServiceState,
}

/// Snapshot published after each service observation tick.
#[derive(Clone, Debug)]
pub struct BroadcastServiceWatchSnapshot {
    /// Capture time for ordering/debugging. Consumers can ignore it.
    pub at: Instant,
    /// State for each observed unit.
    pub units: Vec<BroadcastServiceUnitSnapshot>,
}

impl BroadcastServiceWatchSnapshot {
    fn unknown(units: &[BroadcastServiceWatchUnit]) -> Self {
        Self::new(
            units
                .iter()
                .map(|unit| BroadcastServiceUnitSnapshot {
                    role: unit.role,
                    host_name: unit.host_name.clone(),
                    unit_name: unit.unit.unit().to_owned(),
                    state: ServiceState::Unknown,
                })
                .collect(),
        )
    }

    fn new(units: Vec<BroadcastServiceUnitSnapshot>) -> Self {
        Self {
            at: Instant::now(),
            units,
        }
    }
}

/// Caller-side handle for the service watch actor.
pub struct BroadcastServiceWatchHandle {
    snapshot: watch::Receiver<BroadcastServiceWatchSnapshot>,
    inbox: mpsc::Sender<BroadcastServiceWatchCommand>,
    shutdown: Option<oneshot::Sender<()>>,
}

impl BroadcastServiceWatchHandle {
    /// Subscribe to service snapshots.
    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<BroadcastServiceWatchSnapshot> {
        self.snapshot.clone()
    }

    /// Return the latest known snapshot.
    #[must_use]
    pub fn latest(&self) -> BroadcastServiceWatchSnapshot {
        self.snapshot.borrow().clone()
    }

    /// Request an immediate service read.
    ///
    /// Returns `false` when the actor has shut down or its inbox is full.
    #[must_use]
    pub fn refresh_now(&self) -> bool {
        self.inbox
            .try_send(BroadcastServiceWatchCommand::RefreshNow)
            .is_ok()
    }
}

impl Drop for BroadcastServiceWatchHandle {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BroadcastServiceWatchCommand {
    RefreshNow,
}

/// Spawns the broadcast service watch actor on the current tokio runtime.
#[must_use]
pub fn start(units: Vec<BroadcastServiceWatchUnit>) -> BroadcastServiceWatchHandle {
    start_with_reader(
        units,
        Arc::new(ControlServiceStateReader),
        BROADCAST_SERVICE_POLL_INTERVAL,
    )
}

trait ServiceStateReader: Send + Sync + 'static {
    fn show(&self, transport: &Transport, unit: &UnitRef) -> Result<ServiceState, String>;
}

type SharedServiceStateReader = Arc<dyn ServiceStateReader>;

struct ControlServiceStateReader;

impl ServiceStateReader for ControlServiceStateReader {
    fn show(&self, transport: &Transport, unit: &UnitRef) -> Result<ServiceState, String> {
        control::show(transport, unit).map_err(|error| format!("{error:#}"))
    }
}

fn start_with_reader(
    units: Vec<BroadcastServiceWatchUnit>,
    reader: SharedServiceStateReader,
    interval: Duration,
) -> BroadcastServiceWatchHandle {
    let (snapshot_tx, snapshot_rx) = watch::channel(BroadcastServiceWatchSnapshot::unknown(&units));
    let (inbox_tx, mut inbox_rx) = mpsc::channel::<BroadcastServiceWatchCommand>(INBOX_CAPACITY);
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        publish_snapshot(&snapshot_tx, units.clone(), Arc::clone(&reader)).await;
        loop {
            tokio::select! {
                biased;
                _ = &mut shutdown_rx => break,
                command = inbox_rx.recv() => {
                    let Some(BroadcastServiceWatchCommand::RefreshNow) = command else {
                        break;
                    };
                    publish_snapshot(&snapshot_tx, units.clone(), Arc::clone(&reader)).await;
                }
                () = tokio::time::sleep(interval) => {
                    publish_snapshot(&snapshot_tx, units.clone(), Arc::clone(&reader)).await;
                }
            }
        }
    });

    BroadcastServiceWatchHandle {
        snapshot: snapshot_rx,
        inbox: inbox_tx,
        shutdown: Some(shutdown_tx),
    }
}

async fn publish_snapshot(
    snapshot_tx: &watch::Sender<BroadcastServiceWatchSnapshot>,
    units: Vec<BroadcastServiceWatchUnit>,
    reader: SharedServiceStateReader,
) {
    let fallback = BroadcastServiceWatchSnapshot::unknown(&units);
    let snapshot = read_services(units, reader).await.unwrap_or(fallback);
    let _ = snapshot_tx.send(snapshot);
}

async fn read_services(
    units: Vec<BroadcastServiceWatchUnit>,
    reader: SharedServiceStateReader,
) -> Result<BroadcastServiceWatchSnapshot, tokio::task::JoinError> {
    tokio::task::spawn_blocking(move || read_services_blocking(&units, &reader)).await
}

fn read_services_blocking(
    units: &[BroadcastServiceWatchUnit],
    reader: &SharedServiceStateReader,
) -> BroadcastServiceWatchSnapshot {
    BroadcastServiceWatchSnapshot::new(
        units
            .iter()
            .map(|unit| BroadcastServiceUnitSnapshot {
                role: unit.role,
                host_name: unit.host_name.clone(),
                unit_name: unit.unit.unit().to_owned(),
                state: reader
                    .show(&unit.transport, &unit.unit)
                    .unwrap_or(ServiceState::Unknown),
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use tokio::time::timeout;

    use super::*;

    #[tokio::test(flavor = "current_thread")]
    async fn service_watch_reads_all_units_into_snapshot() {
        let units = watch_units();
        let reader = Arc::new(StubServiceStateReader::new(vec![
            Ok(ServiceState::Active),
            Ok(ServiceState::Inactive),
        ]));

        let snapshot = read_services(units, Arc::clone(&reader) as SharedServiceStateReader)
            .await
            .expect("join");

        assert_eq!(
            snapshot.units,
            vec![
                BroadcastServiceUnitSnapshot {
                    role: BroadcastServiceRole::Publisher,
                    host_name: "Local".to_owned(),
                    unit_name: "musicindex-live-publisher@mixxx.service".to_owned(),
                    state: ServiceState::Active,
                },
                BroadcastServiceUnitSnapshot {
                    role: BroadcastServiceRole::Producer,
                    host_name: "Local".to_owned(),
                    unit_name: "mixxx-now-playing.service".to_owned(),
                    state: ServiceState::Inactive,
                },
            ]
        );
        assert_eq!(
            reader.calls(),
            vec![
                "musicindex-live-publisher@mixxx.service",
                "mixxx-now-playing.service",
            ]
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn service_watch_maps_read_errors_to_unknown_state() {
        let units = watch_units();
        let reader = Arc::new(StubServiceStateReader::new(vec![
            Err("cannot read service".to_owned()),
            Ok(ServiceState::Failed {
                reason: "exit-code".to_owned(),
            }),
        ]));

        let snapshot = read_services(units, Arc::clone(&reader) as SharedServiceStateReader)
            .await
            .expect("join");

        assert_eq!(snapshot.units[0].state, ServiceState::Unknown);
        assert_eq!(
            snapshot.units[1].state,
            ServiceState::Failed {
                reason: "exit-code".to_owned(),
            }
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn service_watch_refresh_now_publishes_without_waiting_for_interval() {
        let units = watch_units();
        let reader = Arc::new(StubServiceStateReader::new(vec![
            Ok(ServiceState::Inactive),
            Ok(ServiceState::Inactive),
            Ok(ServiceState::Active),
            Ok(ServiceState::Active),
        ]));
        let handle = start_with_reader(
            units,
            Arc::clone(&reader) as SharedServiceStateReader,
            Duration::from_secs(60),
        );
        let mut receiver = handle.subscribe();

        wait_for_states(
            &mut receiver,
            &[ServiceState::Inactive, ServiceState::Inactive],
        )
        .await;

        assert!(handle.refresh_now());
        wait_for_states(&mut receiver, &[ServiceState::Active, ServiceState::Active]).await;
        drop(handle);
    }

    async fn wait_for_states(
        receiver: &mut watch::Receiver<BroadcastServiceWatchSnapshot>,
        expected: &[ServiceState],
    ) {
        loop {
            let states: Vec<_> = receiver
                .borrow()
                .units
                .iter()
                .map(|unit| unit.state.clone())
                .collect();
            if states == expected {
                return;
            }
            timeout(Duration::from_secs(1), receiver.changed())
                .await
                .expect("actor should publish before timeout")
                .expect("actor should keep the watch channel open");
        }
    }

    fn watch_units() -> Vec<BroadcastServiceWatchUnit> {
        vec![
            BroadcastServiceWatchUnit::new(
                BroadcastServiceRole::Publisher,
                "Local",
                Transport::local(),
                UnitRef::publisher("mixxx").expect("publisher unit"),
            ),
            BroadcastServiceWatchUnit::new(
                BroadcastServiceRole::Producer,
                "Local",
                Transport::local(),
                UnitRef::new("mixxx-now-playing.service").expect("producer unit"),
            ),
        ]
    }

    struct StubServiceStateReader {
        calls: Mutex<Vec<String>>,
        responses: Mutex<VecDeque<Result<ServiceState, String>>>,
    }

    impl StubServiceStateReader {
        fn new(responses: Vec<Result<ServiceState, String>>) -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                responses: Mutex::new(VecDeque::from(responses)),
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().expect("calls lock").clone()
        }
    }

    impl ServiceStateReader for StubServiceStateReader {
        fn show(&self, _transport: &Transport, unit: &UnitRef) -> Result<ServiceState, String> {
            self.calls
                .lock()
                .expect("calls lock")
                .push(unit.unit().to_owned());
            self.responses
                .lock()
                .expect("responses lock")
                .pop_front()
                .expect("stub response")
        }
    }
}
