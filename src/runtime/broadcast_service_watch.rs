#![warn(clippy::pedantic)]

//! Publisher-side service watch actor for ADR 0059.
//!
//! The actor observes service state for the local broadcast chain and publishes
//! GPUI-free snapshots over `tokio::sync::watch`. Blocking service reads run on
//! Tokio's blocking pool so the desktop render thread never waits on the rig.
//!
//! The actor reads at the pace its caller sets with
//! [`BroadcastServiceWatchHandle::set_pace`]. `Viewed` reads the units each
//! second and the encoder each 5 seconds. `Background` reads both each 10
//! seconds. A unit-only tick keeps the last encoder value. `refresh_now` and
//! a change to `Viewed` always read the encoder too, so the caller never
//! waits past the new pace for a confirming read.

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::application::session_lifecycle::SessionLifecycle;

use tokio::sync::{mpsc, oneshot, watch};

use crate::broadcast::control::{self, ServiceState, UnitRef};
use crate::broadcast::encoder::{self, EncoderStatus, EncoderTarget};
use crate::broadcast::transport::Transport;

const INBOX_CAPACITY: usize = 8;

/// Unit read interval at the `Viewed` pace (ADR 0059).
const VIEWED_UNIT_INTERVAL: Duration = Duration::from_secs(1);
/// Encoder read interval at the `Viewed` pace (ADR 0059).
const VIEWED_ENCODER_INTERVAL: Duration = Duration::from_secs(5);
/// Unit and encoder read interval at the `Background` pace (ADR 0059).
const BACKGROUND_INTERVAL: Duration = Duration::from_secs(10);

/// How often the service watch reads (ADR 0059).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BroadcastServiceWatchPace {
    /// Show is open, or a Show command waits for its result.
    Viewed,
    /// Each other condition.
    Background,
}

impl BroadcastServiceWatchPace {
    /// Interval between unit reads at this pace.
    const fn unit_interval(self) -> Duration {
        match self {
            Self::Viewed => VIEWED_UNIT_INTERVAL,
            Self::Background => BACKGROUND_INTERVAL,
        }
    }

    /// Interval between encoder reads at this pace.
    const fn encoder_interval(self) -> Duration {
        match self {
            Self::Viewed => VIEWED_ENCODER_INTERVAL,
            Self::Background => BACKGROUND_INTERVAL,
        }
    }
}

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
    /// Invalid settings have no command target and are distinct from absent setup.
    #[must_use]
    pub fn unavailable() -> Self {
        Self {
            server_name: "Configuration unavailable".to_owned(),
            target: None,
        }
    }
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

    /// Set the pace the watch reads at (ADR 0059).
    ///
    /// A change to `Viewed` reads all services at once. Returns `false` when
    /// the actor has shut down or its inbox is full.
    #[must_use]
    pub fn set_pace(&self, pace: BroadcastServiceWatchPace) -> bool {
        self.inbox
            .try_send(BroadcastServiceWatchCommand::SetPace(pace))
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
    SetPace(BroadcastServiceWatchPace),
}

/// Spawns the broadcast service watch actor on the current tokio runtime.
#[must_use]
pub fn start(
    units: Vec<BroadcastServiceWatchUnit>,
    encoder: BroadcastEncoderWatchTarget,
    session: &SessionLifecycle,
) -> BroadcastServiceWatchHandle {
    start_in_session(
        units,
        encoder,
        Arc::new(ControlBroadcastStatusReader),
        session,
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

#[cfg(test)]
fn start_with_reader(
    units: Vec<BroadcastServiceWatchUnit>,
    encoder: BroadcastEncoderWatchTarget,
    reader: SharedBroadcastStatusReader,
) -> BroadcastServiceWatchHandle {
    start_in_session(units, encoder, reader, &SessionLifecycle::new())
}

fn start_in_session(
    units: Vec<BroadcastServiceWatchUnit>,
    encoder: BroadcastEncoderWatchTarget,
    reader: SharedBroadcastStatusReader,
    session: &SessionLifecycle,
) -> BroadcastServiceWatchHandle {
    let (snapshot_tx, snapshot_rx) =
        watch::channel(BroadcastServiceWatchSnapshot::unknown(&units, &encoder));
    let (inbox_tx, mut inbox_rx) = mpsc::channel::<BroadcastServiceWatchCommand>(INBOX_CAPACITY);
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

    let stop = session.stop_token();
    session.spawn_actor("Broadcast service observations", async move {
        // The first read at start always reads the encoder too (ADR 0059).
        let mut pace = BroadcastServiceWatchPace::Background;
        let mut last_encoder = publish_snapshot(
            &snapshot_tx,
            units.clone(),
            encoder.clone(),
            Arc::clone(&reader),
            true,
            unknown_encoder_snapshot(&encoder),
        )
        .await;
        let mut last_encoder_read = tokio::time::Instant::now();
        let mut next_unit_tick = tokio::time::Instant::now() + pace.unit_interval();
        loop {
            tokio::select! {
                biased;
                () = stop.cancelled() => break,
                _ = &mut shutdown_rx => break,
                command = inbox_rx.recv() => {
                    match command {
                        None => break,
                        Some(BroadcastServiceWatchCommand::RefreshNow) => {
                            last_encoder = publish_snapshot(
                                &snapshot_tx,
                                units.clone(),
                                encoder.clone(),
                                Arc::clone(&reader),
                                true,
                                last_encoder,
                            )
                            .await;
                            last_encoder_read = tokio::time::Instant::now();
                            next_unit_tick = tokio::time::Instant::now() + pace.unit_interval();
                        }
                        Some(BroadcastServiceWatchCommand::SetPace(new_pace)) => {
                            if new_pace != pace {
                                pace = new_pace;
                                // A change to Viewed reads all services at once (ADR 0059).
                                if pace == BroadcastServiceWatchPace::Viewed {
                                    last_encoder = publish_snapshot(
                                        &snapshot_tx,
                                        units.clone(),
                                        encoder.clone(),
                                        Arc::clone(&reader),
                                        true,
                                        last_encoder,
                                    )
                                    .await;
                                    last_encoder_read = tokio::time::Instant::now();
                                }
                                next_unit_tick = tokio::time::Instant::now() + pace.unit_interval();
                            }
                        }
                    }
                }
                () = tokio::time::sleep_until(next_unit_tick) => {
                    let now = tokio::time::Instant::now();
                    let read_encoder =
                        now.saturating_duration_since(last_encoder_read) >= pace.encoder_interval();
                    last_encoder = publish_snapshot(
                        &snapshot_tx,
                        units.clone(),
                        encoder.clone(),
                        Arc::clone(&reader),
                        read_encoder,
                        last_encoder,
                    )
                    .await;
                    if read_encoder {
                        last_encoder_read = now;
                    }
                    next_unit_tick = now + pace.unit_interval();
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

/// Publish one snapshot. Returns the published encoder snapshot, so the
/// caller can carry it over into a later unit-only tick (ADR 0059).
async fn publish_snapshot(
    snapshot_tx: &watch::Sender<BroadcastServiceWatchSnapshot>,
    units: Vec<BroadcastServiceWatchUnit>,
    encoder: BroadcastEncoderWatchTarget,
    reader: SharedBroadcastStatusReader,
    read_encoder: bool,
    previous_encoder: BroadcastEncoderSnapshot,
) -> BroadcastEncoderSnapshot {
    let fallback = BroadcastServiceWatchSnapshot::unknown(&units, &encoder);
    let snapshot = read_broadcast_status(units, encoder, reader, read_encoder, previous_encoder)
        .await
        .unwrap_or(fallback);
    let published_encoder = snapshot.encoder.clone();
    let _ = snapshot_tx.send(snapshot);
    published_encoder
}

async fn read_broadcast_status(
    units: Vec<BroadcastServiceWatchUnit>,
    encoder: BroadcastEncoderWatchTarget,
    reader: SharedBroadcastStatusReader,
    read_encoder: bool,
    previous_encoder: BroadcastEncoderSnapshot,
) -> Result<BroadcastServiceWatchSnapshot, tokio::task::JoinError> {
    tokio::task::spawn_blocking(move || {
        read_broadcast_status_blocking(&units, &encoder, &reader, read_encoder, &previous_encoder)
    })
    .await
}

/// Read the units, and the encoder only when `read_encoder` is true.
///
/// When `read_encoder` is false, the returned snapshot keeps
/// `previous_encoder` unchanged (ADR 0059).
fn read_broadcast_status_blocking(
    units: &[BroadcastServiceWatchUnit],
    encoder: &BroadcastEncoderWatchTarget,
    reader: &SharedBroadcastStatusReader,
    read_encoder: bool,
    previous_encoder: &BroadcastEncoderSnapshot,
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
        if read_encoder {
            read_encoder_snapshot(encoder, reader)
        } else {
            previous_encoder.clone()
        },
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
    use std::sync::atomic::{AtomicUsize, Ordering};
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
            true,
            &unknown_encoder_snapshot(&watch_encoder()),
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
            true,
            unknown_encoder_snapshot(&watch_encoder()),
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
            true,
            unknown_encoder_snapshot(&watch_encoder()),
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
            true,
            unknown_encoder_snapshot(&BroadcastEncoderWatchTarget::not_configured()),
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

    #[tokio::test(flavor = "current_thread", start_paused = true)]
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
        // The default Background pace reads again only after 10 seconds, and
        // this test never advances time, so only refresh_now can publish again.
        let handle = start_with_reader(
            units,
            watch_encoder(),
            Arc::clone(&reader) as SharedBroadcastStatusReader,
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

    /// R59-18-01 (ADR 0059): `Background` reads the units and the encoder no
    /// more than every 10 seconds.
    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn adr_0059_watch_pace_background_reads_units_and_encoder_at_most_every_ten_seconds() {
        let reader = Arc::new(CountingBroadcastStatusReader::new());
        let handle = start_with_reader(
            one_watch_unit(),
            watch_encoder(),
            Arc::clone(&reader) as SharedBroadcastStatusReader,
        );
        let mut rx = handle.subscribe();
        rx.changed().await.expect("the first read at start");
        let start_units = reader.unit_reads();
        let start_encoder = reader.encoder_reads();

        for _ in 0..3 {
            rx.changed().await.expect("a Background tick");
        }

        assert_eq!(reader.unit_reads() - start_units, 3);
        assert_eq!(reader.encoder_reads() - start_encoder, 3);
        drop(handle);
    }

    /// R59-18-02 (ADR 0059): `Viewed` reads the units each second and the
    /// encoder each 5 seconds.
    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn adr_0059_watch_pace_viewed_reads_units_each_second_and_encoder_each_five_seconds() {
        let reader = Arc::new(CountingBroadcastStatusReader::new());
        let handle = start_with_reader(
            one_watch_unit(),
            watch_encoder(),
            Arc::clone(&reader) as SharedBroadcastStatusReader,
        );
        let mut rx = handle.subscribe();
        rx.changed().await.expect("the first read at start");

        assert!(handle.set_pace(BroadcastServiceWatchPace::Viewed));
        rx.changed()
            .await
            .expect("the pace change reads all services at once");
        let start_units = reader.unit_reads();
        let start_encoder = reader.encoder_reads();

        for _ in 0..10 {
            rx.changed().await.expect("a Viewed tick");
        }

        assert_eq!(reader.unit_reads() - start_units, 10);
        assert_eq!(reader.encoder_reads() - start_encoder, 2);
        drop(handle);
    }

    /// R59-18-03 (ADR 0059): a change to `Viewed` reads all services at once.
    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn adr_0059_watch_pace_set_pace_viewed_reads_all_services_at_once() {
        let reader = Arc::new(CountingBroadcastStatusReader::new());
        let handle = start_with_reader(
            one_watch_unit(),
            watch_encoder(),
            Arc::clone(&reader) as SharedBroadcastStatusReader,
        );
        let mut rx = handle.subscribe();
        rx.changed().await.expect("the first read at start");
        let start_units = reader.unit_reads();
        let start_encoder = reader.encoder_reads();

        assert!(handle.set_pace(BroadcastServiceWatchPace::Viewed));
        rx.changed()
            .await
            .expect("the pace change reads all services at once");

        assert_eq!(reader.unit_reads() - start_units, 1);
        assert_eq!(reader.encoder_reads() - start_encoder, 1);
        drop(handle);
    }

    /// R59-18-04 (ADR 0059): `refresh_now` reads the units and the encoder,
    /// also before the encoder interval ends.
    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn adr_0059_watch_pace_refresh_now_reads_before_the_encoder_interval_ends() {
        let reader = Arc::new(CountingBroadcastStatusReader::new());
        let handle = start_with_reader(
            one_watch_unit(),
            watch_encoder(),
            Arc::clone(&reader) as SharedBroadcastStatusReader,
        );
        let mut rx = handle.subscribe();
        rx.changed().await.expect("the first read at start");
        assert!(handle.set_pace(BroadcastServiceWatchPace::Viewed));
        rx.changed()
            .await
            .expect("the pace change reads all services at once");
        // One second after the last encoder read, under the five-second
        // Viewed encoder interval, so this tick does not read the encoder.
        rx.changed().await.expect("a Viewed unit tick");
        let start_units = reader.unit_reads();
        let start_encoder = reader.encoder_reads();

        assert!(handle.refresh_now());
        rx.changed()
            .await
            .expect("refresh_now reads before the encoder interval ends");

        assert_eq!(reader.unit_reads() - start_units, 1);
        assert_eq!(reader.encoder_reads() - start_encoder, 1);
        drop(handle);
    }

    /// R59-18-05 (ADR 0059): a snapshot between two encoder reads keeps the
    /// last encoder value.
    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn adr_0059_watch_pace_unit_only_ticks_keep_the_last_encoder_value() {
        let reader = Arc::new(CountingBroadcastStatusReader::new());
        let handle = start_with_reader(
            one_watch_unit(),
            watch_encoder(),
            Arc::clone(&reader) as SharedBroadcastStatusReader,
        );
        let mut rx = handle.subscribe();
        rx.changed().await.expect("the first read at start");
        assert!(handle.set_pace(BroadcastServiceWatchPace::Viewed));
        rx.changed()
            .await
            .expect("the pace change reads all services at once");
        let encoder_after_pace_change = rx.borrow().encoder.clone();

        for _ in 0..4 {
            rx.changed().await.expect("a unit-only Viewed tick");
            assert_eq!(rx.borrow().encoder, encoder_after_pace_change);
        }
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

    /// One unit, for a pace test that only needs to count read batches.
    fn one_watch_unit() -> Vec<BroadcastServiceWatchUnit> {
        vec![BroadcastServiceWatchUnit::new(
            BroadcastServiceRole::Publisher,
            "Local",
            Transport::local(),
            UnitRef::publisher("mixxx").expect("publisher unit"),
        )]
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

    /// Counts reads for the pace tests. Each encoder read reports a distinct
    /// listener count, so a test can tell a fresh read from a kept value.
    struct CountingBroadcastStatusReader {
        unit_reads: AtomicUsize,
        encoder_reads: AtomicUsize,
    }

    impl CountingBroadcastStatusReader {
        fn new() -> Self {
            Self {
                unit_reads: AtomicUsize::new(0),
                encoder_reads: AtomicUsize::new(0),
            }
        }

        fn unit_reads(&self) -> usize {
            self.unit_reads.load(Ordering::SeqCst)
        }

        fn encoder_reads(&self) -> usize {
            self.encoder_reads.load(Ordering::SeqCst)
        }
    }

    impl BroadcastStatusReader for CountingBroadcastStatusReader {
        fn show(&self, _transport: &Transport, _unit: &UnitRef) -> Result<ServiceState, String> {
            self.unit_reads.fetch_add(1, Ordering::SeqCst);
            Ok(ServiceState::Active)
        }

        fn encoder_status(&self, _target: &EncoderTarget) -> Result<EncoderStatus, String> {
            let read = self.encoder_reads.fetch_add(1, Ordering::SeqCst);
            Ok(EncoderStatus {
                state: EncoderState::Connected,
                recording: RecordingState::Stopped {
                    seconds: Some(0),
                    path: None,
                },
                signal: AudioSignalState::Present,
                listeners: ListenerCount::Known(read as u64),
                song: None,
                stream_seconds: Some(read as u64),
            })
        }
    }
}
