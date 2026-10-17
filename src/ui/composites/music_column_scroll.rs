//! The one scrollbar choice for a Music column (ADR 0084 Decision 1,
//! amended 2026-10-17, and ADR 0063's shared scrollbar gutter).
//!
//! A Music column that scrolls calls one of the two functions below. No
//! shell in Music picks its own scroll method. The caller still wraps its
//! own content in [`page_scroll_content`](super::page_scroll_content), so
//! the scrollbar covers no content.

#![warn(clippy::pedantic)]

use gpui::{div, prelude::*, AnyElement, ElementId, ScrollHandle};
use gpui_component::scroll::ScrollableElement;

/// Makes `container` a Music column with its own scrollbar (ADR 0084
/// Decision 1, amended 2026-10-17). Use this for a column that keeps no
/// scroll position across a navigation change.
///
/// `id` must be stable. When one call site renders more than one page,
/// give each page a different `id`, so the two pages do not share one
/// scroll position (task 014 Recorded Facts).
pub(crate) fn music_column_scrollbar<E>(container: E, id: impl Into<ElementId>) -> AnyElement
where
    E: ScrollableElement + 'static,
{
    container.overflow_y_scrollbar().id(id).into_any_element()
}

/// Makes `container` a Music column that scrolls with `scroll`, with the
/// scrollbar on the returned parent (ADR 0084 Decision 1, amended
/// 2026-10-17). Use this when the column must keep a scroll position
/// across a navigation change, for example Back to "New releases"
/// (task 006 V3). `container` already has a stable `.id(..)`.
///
/// `probe` is an optional extra layer over the column, for a caller that
/// reads the scroll position after each change, as `src/ui/composites/
/// log_frame.rs` reads its own handle. It paints nothing on its own.
///
/// The returned parent is a plain block box, following the gutter
/// pattern of `src/ui/composites/settings.rs`. `container` gets
/// `.size_full()` here, so its own `flex_1()`/`min_h_0()` from a flex
/// ancestor further up stay harmless, not load-bearing, inside it.
pub(crate) fn music_column_tracked_scrollbar<E>(
    container: E,
    scroll: &ScrollHandle,
    probe: Option<AnyElement>,
) -> AnyElement
where
    E: StatefulInteractiveElement + Styled + IntoElement + 'static,
{
    let mut wrapper = div().relative().flex_1().min_h_0().min_w_0().child(
        container
            .size_full()
            .overflow_y_scroll()
            .track_scroll(scroll),
    );
    if let Some(probe) = probe {
        wrapper = wrapper.child(probe);
    }
    wrapper.vertical_scrollbar(scroll).into_any_element()
}
