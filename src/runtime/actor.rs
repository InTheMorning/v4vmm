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
    pub fn borrow(&self) -> watch::Ref<'_, S> {
        self.snapshot.borrow()
    }

    /// Subscribe a fresh `watch::Receiver` to the snapshot channel.
    /// `GpuiVmBridge` uses this to install per-screen receivers.
    pub fn subscribe(&self) -> watch::Receiver<S> {
        self.snapshot.clone()
    }
}

/// Spawn an actor on the current tokio runtime. Returns the
/// caller-side handle.
pub fn spawn<A>(mut actor: A, bus: VmBus) -> ActorHandle<A::Message, A::State>
where
    A: Actor,
{
    let initial = actor.initial_state();
    let (snapshot_tx, snapshot_rx) = watch::channel(initial);
    let (inbox_tx, mut inbox_rx) = mpsc::channel::<A::Message>(INBOX_CAPACITY);

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
}
