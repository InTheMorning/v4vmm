//! Actor primitives (ADR 0040).
//!
//! An [`Actor`] is a tokio task that owns mutable view-model state and
//! exposes:
//!
//! * an inbox (`tokio::sync::mpsc`) for typed messages, and
//! * a snapshot channel (`tokio::sync::watch`) the GPUI bridge polls
//!   once per frame.
//!
//! The skeleton in this module is deliberately small. Concrete actors
//! (library, search, playback) plug into it by implementing `Actor` and
//! routing message handlers through `handle`.
//!
//! ### Why `watch` not `broadcast`?
//!
//! `watch` keeps only the latest value, which matches GPUI's "render the
//! current frame" model: a slow consumer never re-renders stale snapshots
//! because the channel coalesces. The cross-actor `VmBus` (broadcast) is
//! reserved for invalidation events that must not be coalesced.

#![warn(clippy::pedantic)]

use tokio::sync::{broadcast, mpsc, watch};

use super::vm_bus::{VmBus, VmEvent};

const INBOX_CAPACITY: usize = 64;

/// View-model state that an actor publishes. Must be cheap to clone
/// because every frame `GpuiVmBridge` clones the latest watch value.
pub trait Snapshot: Clone + Send + Sync + 'static {}

impl<T> Snapshot for T where T: Clone + Send + Sync + 'static {}

/// Trait every actor implements.
///
/// `Self::Message` carries typed inbox commands; `Self::State` is the
/// snapshot type. `handle` mutates internal state and returns the new
/// snapshot to publish (or `None` when nothing changed).
///
/// Actors are *automatically* subscribed to the runtime [`VmBus`].
/// Override [`Actor::handle_event`] to react to invalidations
/// (the default implementation discards every event). The spawn loop
/// uses `tokio::select!` so inbox and bus events are handled with
/// equal priority; if the broadcast channel lags (slow consumer +
/// burst), the loop synthesizes a [`VmEvent::InvalidateAll`] so the
/// actor can drop caches conservatively.
pub trait Actor: Send + 'static {
    type Message: Send + 'static;
    type State: Snapshot;

    /// Initial state published when the actor starts.
    fn initial_state(&self) -> Self::State;

    /// Handle a single inbox message. Return `Some(new_state)` to
    /// publish a new snapshot, `None` to skip publishing.
    fn handle(&mut self, message: Self::Message, bus: &VmBus) -> Option<Self::State>;

    /// Handle a single [`VmBus`] invalidation. Defaults to a no-op so
    /// existing actors compile unchanged. Override when the actor
    /// caches state derived from another actor's domain.
    #[allow(unused_variables)]
    fn handle_event(&mut self, event: VmEvent, bus: &VmBus) -> Option<Self::State> {
        None
    }
}

/// Caller-side handle to a running actor.
#[derive(Debug)]
pub struct ActorHandle<M, S> {
    inbox: mpsc::Sender<M>,
    snapshot: watch::Receiver<S>,
}

impl<M, S> Clone for ActorHandle<M, S>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inbox: self.inbox.clone(),
            snapshot: self.snapshot.clone(),
        }
    }
}

impl<M, S> ActorHandle<M, S>
where
    M: Send + 'static,
    S: Snapshot,
{
    /// Try to enqueue a message. Returns `false` when the actor's
    /// inbox is full (back-pressure) or the actor has shut down.
    pub fn try_send(&self, message: M) -> bool {
        self.inbox.try_send(message).is_ok()
    }

    /// Enqueue a message, awaiting buffer space if full. Returns
    /// `false` when the actor has shut down.
    pub async fn send(&self, message: M) -> bool {
        self.inbox.send(message).await.is_ok()
    }

    /// Borrow the latest published snapshot without cloning.
    #[must_use]
    pub fn borrow(&self) -> watch::Ref<'_, S> {
        self.snapshot.borrow()
    }

    /// Subscribe a fresh `watch::Receiver` to the snapshot channel.
    /// `GpuiVmBridge` uses this to install per-screen receivers.
    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<S> {
        self.snapshot.clone()
    }
}

/// Spawn an actor on the current tokio runtime. Returns the
/// caller-side handle.
pub fn spawn<A>(actor: A, bus: VmBus) -> ActorHandle<A::Message, A::State>
where
    A: Actor,
{
    spawn_with_capacity(actor, bus, INBOX_CAPACITY)
}

/// Spawn an actor with an explicit inbox capacity. Useful for tests
/// that need to exercise back-pressure deterministically.
pub fn spawn_with_capacity<A>(
    mut actor: A,
    bus: VmBus,
    inbox_capacity: usize,
) -> ActorHandle<A::Message, A::State>
where
    A: Actor,
{
    let initial = actor.initial_state();
    let (snapshot_tx, snapshot_rx) = watch::channel(initial);
    let (inbox_tx, mut inbox_rx) = mpsc::channel::<A::Message>(inbox_capacity);
    let mut bus_rx = bus.subscribe();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                biased;
                message = inbox_rx.recv() => {
                    let Some(message) = message else { break };
                    if let Some(new_state) = actor.handle(message, &bus) {
                        let _ = snapshot_tx.send(new_state);
                    }
                }
                event = bus_rx.recv() => {
                    match event {
                        Ok(event) => {
                            if let Some(new_state) = actor.handle_event(event, &bus) {
                                let _ = snapshot_tx.send(new_state);
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            // Bus is gone; keep serving inbox until that
                            // closes too. Resubscribe is unnecessary —
                            // VmBus owners outlive actors in practice.
                            // Re-create a never-ready receiver so the
                            // select arm stays inert.
                            bus_rx = bus.subscribe();
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {
                            // We dropped events — be conservative.
                            if let Some(new_state) =
                                actor.handle_event(VmEvent::InvalidateAll, &bus)
                            {
                                let _ = snapshot_tx.send(new_state);
                            }
                        }
                    }
                }
            }
        }
    });

    ActorHandle {
        inbox: inbox_tx,
        snapshot: snapshot_rx,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Counter {
        n: i64,
    }

    enum Msg {
        Inc,
        Set(i64),
    }

    impl Actor for Counter {
        type Message = Msg;
        type State = i64;

        fn initial_state(&self) -> Self::State {
            self.n
        }

        fn handle(&mut self, message: Msg, _bus: &VmBus) -> Option<Self::State> {
            match message {
                Msg::Inc => self.n += 1,
                Msg::Set(n) => self.n = n,
            }
            Some(self.n)
        }
    }

    #[tokio::test]
    async fn snapshot_reflects_messages_in_order() {
        let bus = VmBus::new();
        let handle = spawn(Counter { n: 0 }, bus);
        let mut rx = handle.subscribe();

        assert!(handle.send(Msg::Inc).await);
        rx.changed().await.expect("change");
        assert_eq!(*rx.borrow(), 1);

        assert!(handle.send(Msg::Set(42)).await);
        rx.changed().await.expect("change");
        assert_eq!(*rx.borrow(), 42);
    }

    #[tokio::test]
    async fn handle_clone_shares_state() {
        let bus = VmBus::new();
        let handle = spawn(Counter { n: 0 }, bus);
        let cloned = handle.clone();

        assert!(handle.send(Msg::Set(7)).await);
        let mut rx = cloned.subscribe();
        rx.changed().await.expect("change");
        assert_eq!(*rx.borrow(), 7);
    }

    #[tokio::test]
    async fn watch_coalesces_burst_into_latest_only() {
        // Watch is the snapshot channel; a slow consumer must observe the
        // latest value and never re-render stale ones.
        let bus = VmBus::new();
        let handle = spawn(Counter { n: 0 }, bus);
        let mut rx = handle.subscribe();

        for n in 1..=100 {
            assert!(handle.send(Msg::Set(n)).await);
        }
        // Drain until we see the terminal value.
        loop {
            rx.changed().await.expect("change");
            let current = *rx.borrow_and_update();
            if current == 100 {
                break;
            }
            assert!(current > 0 && current < 100);
        }
    }

    // NOTE: We do not test inbox back-pressure here. `mpsc::try_send`
    // semantics are tokio's contract and exhaustively covered by tokio's
    // own test suite. The `spawn_with_capacity` constructor exposes the
    // parameter so callers (and integration tests in real actors) can
    // exercise back-pressure deterministically when needed.

    #[tokio::test]
    async fn dropping_handle_shuts_down_actor_loop() {
        let bus = VmBus::new();
        let handle = spawn(Counter { n: 0 }, bus);
        let mut rx = handle.subscribe();
        assert!(handle.send(Msg::Set(5)).await);
        rx.changed().await.expect("change");
        drop(handle);
        // After the inbox sender is gone, the loop ends and the watch
        // sender is dropped along with the task. `changed` must
        // resolve to Err.
        let result = rx.changed().await;
        assert!(result.is_err(), "expected watch to close on actor drop");
    }

    /// Actor that re-publishes its current state every time it sees a
    /// `VmEvent`, tagging the value so we can distinguish bus-triggered
    /// publishes from inbox-triggered ones in tests.
    struct EventReactor {
        value: i64,
    }

    enum ReactorMsg {
        Set(i64),
    }

    impl Actor for EventReactor {
        type Message = ReactorMsg;
        type State = i64;

        fn initial_state(&self) -> Self::State {
            self.value
        }

        fn handle(&mut self, message: ReactorMsg, _bus: &VmBus) -> Option<Self::State> {
            let ReactorMsg::Set(n) = message;
            self.value = n;
            Some(self.value)
        }

        fn handle_event(&mut self, event: VmEvent, _bus: &VmBus) -> Option<Self::State> {
            match event {
                VmEvent::InvalidateAll => {
                    self.value = -1;
                    Some(self.value)
                }
                VmEvent::TrackChanged { track_id } => {
                    self.value = track_id;
                    Some(self.value)
                }
                _ => None,
            }
        }
    }

    #[tokio::test]
    async fn actor_reacts_to_vm_bus_events() {
        let bus = VmBus::new();
        let handle = spawn(EventReactor { value: 0 }, bus.clone());
        let mut rx = handle.subscribe();

        bus.publish(VmEvent::TrackChanged { track_id: 42 });
        rx.changed().await.expect("change");
        assert_eq!(*rx.borrow_and_update(), 42);

        bus.publish(VmEvent::InvalidateAll);
        rx.changed().await.expect("change");
        assert_eq!(*rx.borrow_and_update(), -1);
    }

    #[tokio::test]
    async fn inbox_and_bus_events_interleave() {
        let bus = VmBus::new();
        let handle = spawn(EventReactor { value: 0 }, bus.clone());
        let mut rx = handle.subscribe();

        assert!(handle.send(ReactorMsg::Set(7)).await);
        rx.changed().await.expect("change");
        assert_eq!(*rx.borrow_and_update(), 7);

        bus.publish(VmEvent::TrackChanged { track_id: 99 });
        rx.changed().await.expect("change");
        assert_eq!(*rx.borrow_and_update(), 99);

        assert!(handle.send(ReactorMsg::Set(3)).await);
        rx.changed().await.expect("change");
        assert_eq!(*rx.borrow_and_update(), 3);
    }

    #[tokio::test]
    async fn default_handle_event_is_noop() {
        // Counter does not override handle_event; bus traffic must not
        // produce any snapshot publishes.
        let bus = VmBus::new();
        let handle = spawn(Counter { n: 5 }, bus.clone());
        let mut rx = handle.subscribe();

        // Drain the initial state so `changed` is meaningful.
        let _ = *rx.borrow_and_update();

        bus.publish(VmEvent::InvalidateAll);
        bus.publish(VmEvent::TrackChanged { track_id: 1 });

        // Race the bus publishes against a real inbox publish; the only
        // snapshot we should observe is the inbox-triggered one.
        assert!(handle.send(Msg::Set(11)).await);
        rx.changed().await.expect("change");
        assert_eq!(*rx.borrow_and_update(), 11);
    }
}
