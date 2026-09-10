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
use crate::broadcast::encoder::{self, EncoderStatus, EncoderTarget};
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

/// Stream encoder observed by the broadcast watch actor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BroadcastEncoderWatchTarget {
    /// Curator-facing server label used by the Stream section.
    pub server_name: String,
    /// Command target for a configured encoder.
    pub target: Option<EncoderTarget>,
}

impl BroadcastEncoderWatchTarget {
    /// Create a configured encoder watch target.
    #[must_use]
    pub fn configured(server_name: impl Into<String>, target: EncoderTarget) -> Self {
        Self {
            server_name: server_name.into(),
            target: Some(target),
        }
    }

    /// Create an unconfigured encoder target.
    #[must_use]
    pub fn not_configured() -> Self {
        Self {
            server_name: "Not configured".to_owned(),
            target: None,
        }
    }
}

/// Stream encoder state observed by the actor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BroadcastEncoderSnapshot {
    /// Curator-facing server label used by the Stream section.
    pub server_name: String,
    /// Whether the operator configured an encoder target.
    pub configured: bool,
    /// Parsed encoder status.
    pub status: EncoderStatus,
}

/// Snapshot published after each service observation tick.
#[derive(Clone, Debug)]
pub struct BroadcastServiceWatchSnapshot {
    /// Time before the batch began reading, used to establish command freshness.
    pub read_started_at: Instant,
    /// Capture time for ordering/debugging. Consumers can ignore it.
    pub at: Instant,
    /// State for each observed unit.
    pub units: Vec<BroadcastServiceUnitSnapshot>,
    /// State of the stream encoder control interface.
    pub encoder: BroadcastEncoderSnapshot,
}

impl BroadcastServiceWatchSnapshot {
    fn unknown(units: &[BroadcastServiceWatchUnit], encoder: &BroadcastEncoderWatchTarget) -> Self {
        Self::new(
            Instant::now(),
            units
                .iter()
                .map(|unit| BroadcastServiceUnitSnapshot {
                    role: unit.role,
                    host_name: unit.host_name.clone(),
                    unit_name: unit.unit.unit().to_owned(),
                    state: ServiceState::Unknown,
                })
                .collect(),
            unknown_encoder_snapshot(encoder),
        )
    }

    fn new(
        read_started_at: Instant,
        units: Vec<BroadcastServiceUnitSnapshot>,
        encoder: BroadcastEncoderSnapshot,
    ) -> Self {
        Self {
            read_started_at,
            at: Instant::now(),
            units,
            encoder,
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
pub fn start(
    units: Vec<BroadcastServiceWatchUnit>,
    encoder: BroadcastEncoderWatchTarget,
) -> BroadcastServiceWatchHandle {
    start_with_reader(
        units,
        encoder,
        Arc::new(ControlBroadcastStatusReader),
        BROADCAST_SERVICE_POLL_INTERVAL,
    )
}

trait BroadcastStatusReader: Send + Sync + 'static {
    fn show(&self, transport: &Transport, unit: &UnitRef) -> Result<ServiceState, String>;

    fn encoder_status(&self, target: &EncoderTarget) -> Result<EncoderStatus, String>;
}

type SharedBroadcastStatusReader = Arc<dyn BroadcastStatusReader>;

struct ControlBroadcastStatusReader;

impl BroadcastStatusReader for ControlBroadcastStatusReader {
    fn show(&self, transport: &Transport, unit: &UnitRef) -> Result<ServiceState, String> {
        control::show(transport, unit).map_err(|error| format!("{error:#}"))
    }

    fn encoder_status(&self, target: &EncoderTarget) -> Result<EncoderStatus, String> {
        encoder::status(target).map_err(|error| format!("{error:#}"))
    }
}

fn start_with_reader(
    units: Vec<BroadcastServiceWatchUnit>,
    encoder: BroadcastEncoderWatchTarget,
    reader: SharedBroadcastStatusReader,
    interval: Duration,
) -> BroadcastServiceWatchHandle {
    let (snapshot_tx, snapshot_rx) =
        watch::channel(BroadcastServiceWatchSnapshot::unknown(&units, &encoder));
    let (inbox_tx, mut inbox_rx) = mpsc::channel::<BroadcastServiceWatchCommand>(INBOX_CAPACITY);
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        publish_snapshot(
            &snapshot_tx,
            units.clone(),
            encoder.clone(),
            Arc::clone(&reader),
        )
        .await;
        loop {
            tokio::select! {
                biased;
                _ = &mut shutdown_rx => break,
                command = inbox_rx.recv() => {
                    let Some(BroadcastServiceWatchCommand::RefreshNow) = command else {
                        break;
                    };
                    publish_snapshot(
                        &snapshot_tx,
                        units.clone(),
                        encoder.clone(),
                        Arc::clone(&reader),
                    )
                    .await;
                }
                () = tokio::time::sleep(interval) => {
                    publish_snapshot(
                        &snapshot_tx,
                        units.clone(),
                        encoder.clone(),
                        Arc::clone(&reader),
                    )
                    .await;
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
    encoder: BroadcastEncoderWatchTarget,
    reader: SharedBroadcastStatusReader,
) {
    let fallback = BroadcastServiceWatchSnapshot::unknown(&units, &encoder);
    let snapshot = read_broadcast_status(units, encoder, reader)
        .await
        .unwrap_or(fallback);
    let _ = snapshot_tx.send(snapshot);
}

async fn read_broadcast_status(
    units: Vec<BroadcastServiceWatchUnit>,
    encoder: BroadcastEncoderWatchTarget,
    reader: SharedBroadcastStatusReader,
) -> Result<BroadcastServiceWatchSnapshot, tokio::task::JoinError> {
    tokio::task::spawn_blocking(move || read_broadcast_status_blocking(&units, &encoder, &reader))
        .await
}

fn read_broadcast_status_blocking(
    units: &[BroadcastServiceWatchUnit],
    encoder: &BroadcastEncoderWatchTarget,
    reader: &SharedBroadcastStatusReader,
) -> BroadcastServiceWatchSnapshot {
    BroadcastServiceWatchSnapshot::new(
        Instant::now(),
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
        read_encoder_snapshot(encoder, reader),
    )
}

fn read_encoder_snapshot(
    encoder: &BroadcastEncoderWatchTarget,
    reader: &SharedBroadcastStatusReader,
) -> BroadcastEncoderSnapshot {
    match &encoder.target {
        Some(target) => BroadcastEncoderSnapshot {
            server_name: encoder.server_name.clone(),
            configured: true,
            status: reader
                .encoder_status(target)
                .unwrap_or_else(|_| EncoderStatus::unknown()),
        },
        None => BroadcastEncoderSnapshot {
            server_name: encoder.server_name.clone(),
            configured: false,
            status: EncoderStatus::not_installed(),
        },
    }
}

fn unknown_encoder_snapshot(encoder: &BroadcastEncoderWatchTarget) -> BroadcastEncoderSnapshot {
    BroadcastEncoderSnapshot {
        server_name: encoder.server_name.clone(),
        configured: encoder.target.is_some(),
        status: if encoder.target.is_some() {
            EncoderStatus::unknown()
        } else {
            EncoderStatus::not_installed()
        },
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use tokio::time::timeout;

    use crate::broadcast::encoder::{
        AudioSignalState, EncoderState, ListenerCount, RecordingState,
    };

    use super::*;

    /// Situational ADR 0059: freshness starts before every service and encoder read.
    #[test]
    fn show_watch_read_start_precedes_all_reads_and_end_follows_them() {
        struct TimedReader(Mutex<Vec<Instant>>);
        impl BroadcastStatusReader for TimedReader {
            fn show(&self, _: &Transport, _: &UnitRef) -> Result<ServiceState, String> {
                self.0.lock().unwrap().push(Instant::now());
                Ok(ServiceState::Active)
            }

            fn encoder_status(&self, _: &EncoderTarget) -> Result<EncoderStatus, String> {
                self.0.lock().unwrap().push(Instant::now());
                Ok(EncoderStatus::unknown())
            }
        }
        let reader = Arc::new(TimedReader(Mutex::new(Vec::new())));
        let snapshot = read_broadcast_status_blocking(
            &watch_units(),
            &watch_encoder(),
            &(Arc::clone(&reader) as SharedBroadcastStatusReader),
        );
        let reads = reader.0.lock().unwrap();
        assert_eq!(reads.len(), 3);
        for read in reads.iter() {
            assert!(snapshot.read_started_at <= *read);
            assert!(*read <= snapshot.at);
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn service_watch_reads_all_units_and_encoder_into_snapshot() {
        let units = watch_units();
        let encoder_status = connected_encoder_status();
        let reader = Arc::new(StubBroadcastStatusReader::new(
            vec![Ok(ServiceState::Active), Ok(ServiceState::Inactive)],
            vec![Ok(encoder_status.clone())],
        ));

        let snapshot = read_broadcast_status(
            units,
            watch_encoder(),
            Arc::clone(&reader) as SharedBroadcastStatusReader,
        )
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
        assert_eq!(
            snapshot.encoder,
            BroadcastEncoderSnapshot {
                server_name: "default".to_owned(),
                configured: true,
                status: encoder_status,
            }
        );
        assert_eq!(reader.encoder_calls(), vec!["butt"]);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn service_watch_maps_read_errors_to_unknown_state() {
        let units = watch_units();
        let reader = Arc::new(StubBroadcastStatusReader::new(
            vec![
                Err("cannot read service".to_owned()),
                Ok(ServiceState::Failed {
                    reason: "exit-code".to_owned(),
                }),
            ],
            vec![Err("cannot read encoder".to_owned())],
        ));

        let snapshot = read_broadcast_status(
            units,
            watch_encoder(),
            Arc::clone(&reader) as SharedBroadcastStatusReader,
        )
        .await
        .expect("join");

        assert_eq!(snapshot.units[0].state, ServiceState::Unknown);
        assert_eq!(
            snapshot.units[1].state,
            ServiceState::Failed {
                reason: "exit-code".to_owned(),
            }
        );
        assert_eq!(snapshot.encoder.status, EncoderStatus::unknown());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn service_watch_reports_unconfigured_encoder_as_not_installed() {
        let reader = Arc::new(StubBroadcastStatusReader::new(Vec::new(), Vec::new()));

        let snapshot = read_broadcast_status(
            Vec::new(),
            BroadcastEncoderWatchTarget::not_configured(),
            Arc::clone(&reader) as SharedBroadcastStatusReader,
        )
        .await
        .expect("join");

        assert_eq!(
            snapshot.encoder,
            BroadcastEncoderSnapshot {
                server_name: "Not configured".to_owned(),
                configured: false,
                status: EncoderStatus::not_installed(),
            }
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn service_watch_refresh_now_publishes_without_waiting_for_interval() {
        let units = watch_units();
        let reader = Arc::new(StubBroadcastStatusReader::new(
            vec![
                Ok(ServiceState::Inactive),
                Ok(ServiceState::Inactive),
                Ok(ServiceState::Active),
                Ok(ServiceState::Active),
            ],
            vec![Ok(EncoderStatus::unknown()), Ok(connected_encoder_status())],
        ));
        let handle = start_with_reader(
            units,
            watch_encoder(),
            Arc::clone(&reader) as SharedBroadcastStatusReader,
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

    fn watch_encoder() -> BroadcastEncoderWatchTarget {
        BroadcastEncoderWatchTarget::configured(
            "default",
            EncoderTarget::local("butt").expect("encoder target"),
        )
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

    fn connected_encoder_status() -> EncoderStatus {
        EncoderStatus {
            state: EncoderState::Connected,
            recording: RecordingState::Stopped {
                seconds: Some(0),
                path: None,
            },
            signal: AudioSignalState::Present,
            listeners: ListenerCount::Unknown,
            song: Some("Artist - Title".to_owned()),
            stream_seconds: Some(12),
        }
    }

    struct StubBroadcastStatusReader {
        calls: Mutex<Vec<String>>,
        responses: Mutex<VecDeque<Result<ServiceState, String>>>,
        encoder_calls: Mutex<Vec<String>>,
        encoder_responses: Mutex<VecDeque<Result<EncoderStatus, String>>>,
    }

    impl StubBroadcastStatusReader {
        fn new(
            responses: Vec<Result<ServiceState, String>>,
            encoder_responses: Vec<Result<EncoderStatus, String>>,
        ) -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                responses: Mutex::new(VecDeque::from(responses)),
                encoder_calls: Mutex::new(Vec::new()),
                encoder_responses: Mutex::new(VecDeque::from(encoder_responses)),
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().expect("calls lock").clone()
        }

        fn encoder_calls(&self) -> Vec<String> {
            self.encoder_calls
                .lock()
                .expect("encoder calls lock")
                .clone()
        }
    }

    impl BroadcastStatusReader for StubBroadcastStatusReader {
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

        fn encoder_status(&self, target: &EncoderTarget) -> Result<EncoderStatus, String> {
            self.encoder_calls
                .lock()
                .expect("encoder calls lock")
                .push(target.binary_path().to_owned());
            self.encoder_responses
                .lock()
                .expect("encoder responses lock")
                .pop_front()
                .expect("stub encoder response")
        }
    }
}
