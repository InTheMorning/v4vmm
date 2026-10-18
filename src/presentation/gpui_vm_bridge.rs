//! GPUI ↔ runtime view-model bridge (ADR 0040 Phase D).
//!
//! [`bridge_watch`] installs a per-entity reactive subscription against
//! a [`tokio::sync::watch::Receiver`] published by a runtime [`Actor`].
//! Whenever the actor publishes a new snapshot, the bridge applies it
//! to the GPUI entity and calls [`Context::notify`].
//!
//! [`bridge_watch_with_redraw`] (ADR 0059) is the same bridge, but `apply`
//! reports whether the snapshot changed what the screen shows. The bridge
//! calls [`Context::notify`] only when `apply` returns `true`. Use it for a
//! watch whose snapshots carry a changed read time on almost every publish,
//! so a redraw on each one would cost more than it shows.
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
//! the ADR 0040 runtime.
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
/// Neither the watch channel nor this GPUI foreground loop needs a Tokio
/// runtime. ADR 0066 also uses it for independent maintenance observations.
pub fn bridge_watch<T, S, F>(rx: watch::Receiver<S>, mut apply: F, cx: &mut Context<T>)
where
    T: 'static,
    S: Clone + Send + Sync + 'static,
    F: FnMut(&mut T, S, &mut Context<T>) + 'static,
{
    bridge_watch_with_redraw(
        rx,
        move |entity, snapshot, cx| {
            apply(entity, snapshot, cx);
            true
        },
        cx,
    );
}

/// Installs a reactive bridge that redraws only when `apply` reports a change.
///
/// This is [`bridge_watch`], except `apply` returns `bool`, and the bridge
/// calls `cx.notify()` only when it returns `true` (ADR 0059). Use it for a
/// watch whose snapshots carry no visible change on most publishes.
pub fn bridge_watch_with_redraw<T, S, F>(rx: watch::Receiver<S>, mut apply: F, cx: &mut Context<T>)
where
    T: 'static,
    S: Clone + Send + Sync + 'static,
    F: FnMut(&mut T, S, &mut Context<T>) -> bool + 'static,
{
    let mut rx = rx;
    cx.spawn(async move |entity, cx| loop {
        if rx.changed().await.is_err() {
            break;
        }
        let snapshot = rx.borrow_and_update().clone();
        if entity
            .update(cx, |this, cx| {
                if apply(this, snapshot, cx) {
                    cx.notify();
                }
            })
            .is_err()
        {
            break;
        }
    })
    .detach();
}
