#![warn(clippy::pedantic)]

//! Broadcast relay observation actor for ADR 0059.
//!
//! The actor polls the relay snapshot for the selected broadcast event and
//! publishes display-free facts over a `tokio::sync::watch` channel. It does
//! not write database state; registry status updates stay in
//! `crate::broadcast::registry`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{oneshot, watch};

use crate::api;

const BROADCAST_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Snapshot published after each broadcast observation tick.
#[derive(Clone, Debug)]
pub struct BroadcastObservationSnapshot {
    /// Capture time for ordering/debugging. Consumers can ignore it.
    pub at: Instant,
    /// Relay observation outcome reduced to display-free facts.
    pub outcome: BroadcastObservationOutcome,
}

impl BroadcastObservationSnapshot {
    fn new(outcome: BroadcastObservationOutcome) -> Self {
        Self {
            at: Instant::now(),
            outcome,
        }
    }
}

/// Plain relay observation outcome for GPUI-free reduction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BroadcastObservationOutcome {
    /// No broadcast event is selected for observation.
    NoEvent,
    /// The relay has a listener-visible payload for the event.
    Live {
        seq: u64,
        updated_at: String,
        title: Option<String>,
        destination_count: usize,
    },
    /// The event exists and returned an empty metadata payload.
    Empty,
    /// The relay reports no metadata for the event.
    Dead,
    /// Polling failed; the actor stays alive for the next tick.
    Error(String),
}

/// Caller-side handle for the broadcast observation actor.
pub struct BroadcastObservationHandle {
    snapshot: watch::Receiver<BroadcastObservationSnapshot>,
    shutdown: Option<oneshot::Sender<()>>,
}

impl BroadcastObservationHandle {
    /// Subscribe to broadcast observation snapshots.
    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<BroadcastObservationSnapshot> {
        self.snapshot.clone()
    }
}

impl Drop for BroadcastObservationHandle {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}

/// Spawns the broadcast observation actor on the current tokio runtime.
#[must_use]
pub fn start(endpoint: String, event_id: Option<String>) -> BroadcastObservationHandle {
    start_with_reader(
        normalized_event_id(event_id),
        Arc::new(ApiRelaySnapshotReader::new(endpoint)),
        BROADCAST_POLL_INTERVAL,
    )
}

trait RelaySnapshotReader: Send + Sync + 'static {
    fn fetch_live_metadata_optional(
        &self,
        event_id: &str,
    ) -> Result<Option<api::LiveMetadataSnapshot>, String>;
}

type SharedRelaySnapshotReader = Arc<dyn RelaySnapshotReader>;

struct ApiRelaySnapshotReader {
    client: api::Client,
}

impl ApiRelaySnapshotReader {
    fn new(endpoint: String) -> Self {
        Self {
            client: api::Client::new_with_base_url(endpoint),
        }
    }
}

impl RelaySnapshotReader for ApiRelaySnapshotReader {
    fn fetch_live_metadata_optional(
        &self,
        event_id: &str,
    ) -> Result<Option<api::LiveMetadataSnapshot>, String> {
        self.client
            .fetch_live_metadata_optional(event_id)
            .map_err(|error| format!("{error:#}"))
    }
}

fn start_with_reader(
    event_id: Option<String>,
    reader: SharedRelaySnapshotReader,
    interval: Duration,
) -> BroadcastObservationHandle {
    let (snapshot_tx, snapshot_rx) = watch::channel(BroadcastObservationSnapshot::new(
        BroadcastObservationOutcome::NoEvent,
    ));
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                biased;
                _ = &mut shutdown_rx => break,
                () = tokio::time::sleep(interval) => {
                    let outcome =
                        observe_selected_event(event_id.clone(), Arc::clone(&reader)).await;
                    let _ = snapshot_tx.send(BroadcastObservationSnapshot::new(outcome));
                }
            }
        }
    });

    BroadcastObservationHandle {
        snapshot: snapshot_rx,
        shutdown: Some(shutdown_tx),
    }
}

async fn observe_selected_event(
    event_id: Option<String>,
    reader: SharedRelaySnapshotReader,
) -> BroadcastObservationOutcome {
    let Some(event_id) = event_id else {
        return BroadcastObservationOutcome::NoEvent;
    };

    tokio::task::spawn_blocking(move || observe_selected_event_blocking(&event_id, &reader))
        .await
        .unwrap_or_else(|error| BroadcastObservationOutcome::Error(format!("{error:#}")))
}

fn observe_selected_event_blocking(
    event_id: &str,
    reader: &SharedRelaySnapshotReader,
) -> BroadcastObservationOutcome {
    match reader.fetch_live_metadata_optional(event_id) {
        Ok(Some(snapshot)) => outcome_from_snapshot(snapshot),
        Ok(None) => BroadcastObservationOutcome::Dead,
        Err(error) => BroadcastObservationOutcome::Error(error),
    }
}

fn outcome_from_snapshot(snapshot: api::LiveMetadataSnapshot) -> BroadcastObservationOutcome {
    if metadata_is_empty(&snapshot.metadata) {
        return BroadcastObservationOutcome::Empty;
    }

    BroadcastObservationOutcome::Live {
        seq: snapshot.seq,
        updated_at: snapshot.updated_at,
        title: snapshot
            .metadata
            .get("title")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        destination_count: snapshot
            .metadata
            .get("value")
            .and_then(|value| value.get("destinations"))
            .and_then(serde_json::Value::as_array)
            .map_or(0, Vec::len),
    }
}

fn metadata_is_empty(metadata: &serde_json::Value) -> bool {
    metadata.is_null() || metadata.as_object().is_some_and(serde_json::Map::is_empty)
}

fn normalized_event_id(event_id: Option<String>) -> Option<String> {
    event_id
        .map(|event_id| event_id.trim().to_owned())
        .filter(|event_id| !event_id.is_empty())
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use serde_json::json;
    use tokio::time::timeout;

    use super::*;

    #[tokio::test(flavor = "current_thread")]
    async fn broadcast_observation_no_event_publishes_no_event_without_reader_call() {
        let stub = Arc::new(StubRelaySnapshotReader::new(Vec::new()));
        let outcome =
            observe_selected_event(None, Arc::clone(&stub) as SharedRelaySnapshotReader).await;

        assert_eq!(outcome, BroadcastObservationOutcome::NoEvent);
        assert!(
            stub.calls().is_empty(),
            "no selected event should not poll the relay"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn broadcast_observation_live_snapshot_reduces_payload_facts() {
        let stub = Arc::new(StubRelaySnapshotReader::new(vec![Ok(Some(live_metadata(
            7,
            json!({
                "title": "Now Playing",
                "value": {
                    "destinations": [
                        {"name": "Alice"},
                        {"name": "Bob"}
                    ]
                }
            }),
        )))]));

        let outcome = observe_selected_event(
            Some("event-one".to_owned()),
            Arc::clone(&stub) as SharedRelaySnapshotReader,
        )
        .await;

        assert_eq!(
            outcome,
            BroadcastObservationOutcome::Live {
                seq: 7,
                updated_at: "2026-09-07T00:00:00Z".to_owned(),
                title: Some("Now Playing".to_owned()),
                destination_count: 2,
            }
        );
        assert_eq!(stub.calls(), vec!["event-one"]);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn broadcast_observation_empty_snapshot_maps_to_empty() {
        let stub = Arc::new(StubRelaySnapshotReader::new(vec![Ok(Some(live_metadata(
            1,
            json!({}),
        )))]));

        let outcome = observe_selected_event(
            Some("event-one".to_owned()),
            Arc::clone(&stub) as SharedRelaySnapshotReader,
        )
        .await;

        assert_eq!(outcome, BroadcastObservationOutcome::Empty);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn broadcast_observation_404_maps_to_dead() {
        let stub = Arc::new(StubRelaySnapshotReader::new(vec![Ok(None)]));

        let outcome = observe_selected_event(
            Some("event-one".to_owned()),
            Arc::clone(&stub) as SharedRelaySnapshotReader,
        )
        .await;

        assert_eq!(outcome, BroadcastObservationOutcome::Dead);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn broadcast_observation_transport_failure_maps_to_error() {
        let stub = Arc::new(StubRelaySnapshotReader::new(vec![Err(
            "relay connection failed".to_owned(),
        )]));

        let outcome = observe_selected_event(
            Some("event-one".to_owned()),
            Arc::clone(&stub) as SharedRelaySnapshotReader,
        )
        .await;

        assert_eq!(
            outcome,
            BroadcastObservationOutcome::Error("relay connection failed".to_owned())
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn broadcast_observation_actor_keeps_running_after_error_tick() {
        let stub = Arc::new(StubRelaySnapshotReader::new(vec![
            Err("relay connection failed".to_owned()),
            Ok(Some(live_metadata(2, json!({"title": "Recovered"})))),
        ]));
        let handle = start_with_reader(
            Some(" event-one ".to_owned()),
            Arc::clone(&stub) as SharedRelaySnapshotReader,
            Duration::from_millis(50),
        );
        let mut receiver = handle.subscribe();

        let first = next_outcome(&mut receiver).await;
        assert_eq!(
            first,
            BroadcastObservationOutcome::Error("relay connection failed".to_owned())
        );

        let second = next_outcome(&mut receiver).await;
        assert_eq!(
            second,
            BroadcastObservationOutcome::Live {
                seq: 2,
                updated_at: "2026-09-07T00:00:00Z".to_owned(),
                title: Some("Recovered".to_owned()),
                destination_count: 0,
            }
        );
        drop(handle);
    }

    async fn next_outcome(
        receiver: &mut watch::Receiver<BroadcastObservationSnapshot>,
    ) -> BroadcastObservationOutcome {
        timeout(Duration::from_secs(1), receiver.changed())
            .await
            .expect("actor should publish before timeout")
            .expect("actor should keep the watch channel open");
        receiver.borrow().outcome.clone()
    }

    fn live_metadata(seq: u64, metadata: serde_json::Value) -> api::LiveMetadataSnapshot {
        api::LiveMetadataSnapshot {
            event_id: "event-one".to_owned(),
            seq,
            updated_at: "2026-09-07T00:00:00Z".to_owned(),
            metadata,
        }
    }

    struct StubRelaySnapshotReader {
        calls: Mutex<Vec<String>>,
        responses: Mutex<VecDeque<Result<Option<api::LiveMetadataSnapshot>, String>>>,
    }

    impl StubRelaySnapshotReader {
        fn new(responses: Vec<Result<Option<api::LiveMetadataSnapshot>, String>>) -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                responses: Mutex::new(VecDeque::from(responses)),
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().expect("calls lock").clone()
        }
    }

    impl RelaySnapshotReader for StubRelaySnapshotReader {
        fn fetch_live_metadata_optional(
            &self,
            event_id: &str,
        ) -> Result<Option<api::LiveMetadataSnapshot>, String> {
            self.calls
                .lock()
                .expect("calls lock")
                .push(event_id.to_owned());
            self.responses
                .lock()
                .expect("responses lock")
                .pop_front()
                .unwrap_or_else(|| Err("stub response exhausted".to_owned()))
        }
    }
}
