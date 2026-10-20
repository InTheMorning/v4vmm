//! The one scrollbar choice for a Music column (ADR 0084 Decision 1,
//! amended 2026-10-17, and ADR 0063's shared scrollbar gutter).
//!
//! A Music column that scrolls calls one of the two functions below. No
//! shell in Music picks its own scroll method. The caller still wraps its
//! own content in [`page_scroll_content`](super::page_scroll_content), so
//! the scrollbar covers no content.

#![warn(clippy::pedantic)]

use std::cell::Cell;
use std::rc::Rc;

use gpui::{canvas, div, point, prelude::*, AnyElement, ElementId, Pixels, ScrollHandle};
use gpui_component::scroll::ScrollableElement;

use crate::view_models::list_selection::FocusStep;

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

/// A request to keep the focused row of a Music detail page in view (ADR
/// 0084 Decision 10, packet 018).
///
/// The rows of a detail page are not direct children of its scroll
/// container, so `ScrollHandle::scroll_to_item` cannot find them. The
/// focused row carries [`Self::probe`] instead. After a key moves the
/// focus, the caller calls [`Self::request`]. At the next layout, the
/// probe reads the bounds of its row and scrolls the column by the
/// smallest distance that shows the full row.
#[derive(Clone, Debug, Default)]
pub(crate) struct ScrollReveal {
    scroll: ScrollHandle,
    pending: Rc<Cell<bool>>,
}

impl ScrollReveal {
    /// A reveal request for the column that scrolls with `scroll`.
    #[must_use]
    pub(crate) fn new(scroll: ScrollHandle) -> Self {
        Self {
            scroll,
            pending: Rc::new(Cell::new(false)),
        }
    }

    /// Asks the next layout to show the row that carries the probe.
    pub(crate) fn request(&self) {
        self.pending.set(true);
    }

    /// An empty layer over the focused row. It paints nothing. The row
    /// gives it its own size.
    #[must_use]
    pub(crate) fn probe(&self) -> AnyElement {
        let scroll = self.scroll.clone();
        let pending = Rc::clone(&self.pending);
        canvas(
            move |bounds, window, _cx| {
                if !pending.replace(false) {
                    return;
                }
                let viewport = scroll.bounds();
                let offset = scroll.offset();
                if let Some(offset_y) = reveal_offset_y(
                    (f32::from(viewport.top()), f32::from(viewport.bottom())),
                    (f32::from(bounds.top()), f32::from(bounds.bottom())),
                    f32::from(offset.y),
                    f32::from(scroll.max_offset().y),
                ) {
                    scroll.set_offset(point(offset.x, Pixels::from(offset_y)));
                    window.on_next_frame(|window, _cx| window.refresh());
                }
            },
            |_bounds, (), _window, _cx| {},
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .into_any_element()
    }
}

/// The scroll offset that shows a full row, or `None` when the row is
/// already in view. `viewport` and `row` are the top and the bottom edge
/// in pixels, with the present `offset_y` applied. The offset is 0 at the
/// top and `-max_offset_y` at the bottom.
fn reveal_offset_y(
    viewport: (f32, f32),
    row: (f32, f32),
    offset_y: f32,
    max_offset_y: f32,
) -> Option<f32> {
    let (view_top, view_bottom) = viewport;
    let (row_top, row_bottom) = row;
    let next = if row_top < view_top {
        offset_y + (view_top - row_top)
    } else if row_bottom > view_bottom {
        offset_y - (row_bottom - view_bottom)
    } else {
        return None;
    };
    Some(next.clamp(-max_offset_y.max(0.0), 0.0))
}

/// The scroll offset in pixels after a key on a Music detail page with no
/// selectable row (ADR 0084 Decision 10, packet 018). Up and Down move one
/// `step`. Page Up and Page Down move one view height less one `step`.
/// Home and End go to the top and the bottom. The offset is 0 at the top
/// and `-max_offset_y` at the bottom.
#[must_use]
pub(crate) fn key_scroll_offset_y(
    key: FocusStep,
    offset_y: f32,
    view_height: f32,
    max_offset_y: f32,
    step: f32,
) -> f32 {
    let page = (view_height - step).max(step);
    let next = match key {
        FocusStep::Up => offset_y + step,
        FocusStep::Down => offset_y - step,
        FocusStep::PageUp => offset_y + page,
        FocusStep::PageDown => offset_y - page,
        FocusStep::Home => 0.0,
        FocusStep::End => -max_offset_y,
    };
    next.clamp(-max_offset_y.max(0.0), 0.0)
}

/// Scrolls `scroll` for one key on a Music detail page with no selectable
/// row. `step` is the scaled [`crate::ui::tokens::Size::KeyScrollStep`].
pub(crate) fn key_scroll(scroll: &ScrollHandle, key: FocusStep, step: Pixels) {
    let offset = scroll.offset();
    let offset_y = key_scroll_offset_y(
        key,
        f32::from(offset.y),
        f32::from(scroll.bounds().size.height),
        f32::from(scroll.max_offset().y),
        f32::from(step),
    );
    scroll.set_offset(point(offset.x, Pixels::from(offset_y)));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R18-09: the focused row scrolls into view by the smallest distance,
    /// and a row that is in view does not scroll.
    #[test]
    fn adr_0084_detail_selection_r18_09_reveal_scrolls_the_smallest_distance() {
        let view = (100.0, 500.0);
        assert_eq!(reveal_offset_y(view, (200.0, 240.0), -50.0, 1000.0), None);
        assert_eq!(
            reveal_offset_y(view, (520.0, 560.0), -50.0, 1000.0),
            Some(-110.0)
        );
        assert_eq!(
            reveal_offset_y(view, (60.0, 100.0), -50.0, 1000.0),
            Some(-10.0)
        );
    }
}
