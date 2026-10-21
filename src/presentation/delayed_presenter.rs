//! GPUI timer for a short interface delay (ADR 0040, ADR 0083 task 008).
//!
//! A screen owns the state that a delay changes. It starts the delay here,
//! because ADR 0040 keeps each `cx.spawn` out of the screens. The timer
//! does no domain work: it only gives the end of the delay back to the
//! screen entity.

use std::time::Duration;

use gpui::{AsyncApp, Context, WeakEntity};

/// Calls `apply` on the entity when `delay` ends. When the entity is gone,
/// nothing occurs.
pub(crate) fn present_after<T, F>(delay: Duration, cx: &mut Context<T>, apply: F)
where
    T: 'static,
    F: FnOnce(&mut T, &mut Context<T>) + 'static,
{
    cx.spawn(async move |this: WeakEntity<T>, cx: &mut AsyncApp| {
        cx.background_executor().timer(delay).await;
        this.update(cx, |this, cx| {
            apply(this, cx);
            cx.notify();
        })
        .ok();
    })
    .detach();
}
