//! GPUI ↔ runtime view-model bridge (ADR 0040 Phase D).
//!
//! [`bridge_watch`] installs a per-entity reactive subscription against
//! a [`tokio::sync::watch::Receiver`] published by a runtime [`Actor`].
//! Whenever the actor publishes a new snapshot, the bridge applies it
//! to the GPUI entity and calls [`Context::notify`].
//!
//! ### Why reactive, not polled?
//!
//! `tokio::sync::watch` is natively coalescing: only the latest value
//! is observable, so awaiting [`watch::Receiver::changed`] resolves
//! exactly when there is something to render. This is strictly better
//! than a 16ms timer poll (no wake-ups when idle, no extra latency on
//! event arrival, no missed renders during bursts because the latest
//! value always wins).
//!
//! ### Backpressure & shutdown
//!
//! * If the GPUI entity is dropped, [`WeakEntity::update`] returns
//!   `Err`; the bridge breaks its loop and the underlying tokio task
//!   completes.
//! * If the actor (and thus the watch sender) is dropped,
//!   `rx.changed()` returns `Err`; the bridge stops gracefully.
//! * Bursts of publishes are coalesced by `watch` itself — the bridge
//!   never re-enters render for stale snapshots.
//!
//! ### Layer rules
//!
//! Lives under `src/presentation/` because it is the only layer
//! permitted to bridge `gpui` to `runtime` (ADR 0040). Gated behind
//! the `async-runtime` Cargo feature.
//!
//! [`Actor`]: crate::runtime::Actor

#![warn(clippy::pedantic)]

use gpui::Context;
use tokio::sync::watch;

/// Installs a reactive bridge from a watch receiver to a GPUI entity.
///
/// Each time the watch publishes a new snapshot, `apply` is invoked on
/// the entity inside an `update` block; `cx.notify()` is called
/// automatically afterwards.
///
/// The returned task is detached — the bridge lives as long as either
/// side of the channel.
///
/// # Panics
///
/// Panics if invoked outside a tokio runtime (the `apply` callback may
/// be called on the GPUI foreground executor; the underlying watch
/// loop runs through `cx.spawn` which lives on the GPUI executor — no
/// tokio runtime is required for the loop itself, only when constructing
/// the watch sender upstream).
pub fn bridge_watch<T, S, F>(rx: watch::Receiver<S>, mut apply: F, cx: &mut Context<T>)
where
    T: 'static,
    S: Clone + Send + Sync + 'static,
    F: FnMut(&mut T, S, &mut Context<T>) + 'static,
{
    let mut rx = rx;
    cx.spawn(async move |entity, cx| loop {
        if rx.changed().await.is_err() {
            break;
        }
        let snapshot = rx.borrow_and_update().clone();
        if entity
            .update(cx, |this, cx| {
                apply(this, snapshot, cx);
                cx.notify();
            })
            .is_err()
        {
            break;
        }
    })
    .detach();
}

#[cfg(test)]
mod tests {
    //! Headful GPUI tests are out of scope; the reactive primitive
    //! used here (`watch::Receiver::changed` + coalescing) is covered
    //! by the runtime suite. We only assert that the public API
    //! compiles and accepts the expected closures.

    use super::*;

    #[allow(dead_code)]
    fn type_check_compiles() {
        // Compile-time check that the generic bounds are satisfiable.
        fn _use<T: 'static>(rx: watch::Receiver<i64>, cx: &mut Context<T>) {
            bridge_watch(
                rx,
                |_this, snapshot, _cx| {
                    let _ = snapshot;
                },
                cx,
            );
        }
    }
}
