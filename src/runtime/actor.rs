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

use tokio::sync::{mpsc, watch};

use super::vm_bus::VmBus;

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
pub trait Actor: Send + 'static {
    type Message: Send + 'static;
    type State: Snapshot;

    /// Initial state published when the actor starts.
    fn initial_state(&self) -> Self::State;

    /// Handle a single inbox message. Return `Some(new_state)` to
    /// publish a new snapshot, `None` to skip publishing.
    fn handle(&mut self, message: Self::Message, bus: &VmBus) -> Option<Self::State>;
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

    tokio::spawn(async move {
        while let Some(message) = inbox_rx.recv().await {
            if let Some(new_state) = actor.handle(message, &bus) {
                let _ = snapshot_tx.send(new_state);
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
}
