//! The one scrollbar choice for a Music column (ADR 0084 Decision 1,
//! amended 2026-10-17, and ADR 0063's shared scrollbar gutter).
//!
//! A Music column that scrolls calls one of the three functions below. No
//! shell in Music picks its own scroll method. The caller still wraps its
//! own content in [`page_scroll_content`](super::page_scroll_content), so
//! the scrollbar covers no content.
//!
//! A long list uses [`music_column_list_scrollbar`] with a
//! [`MusicColumnList`]: a virtual list that renders only the items in view
//! and in the overdraw area (ADR 0084 task 037).

#![warn(clippy::pedantic)]

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    canvas, div, point, prelude::*, AnyElement, App, ElementId, List, ListAlignment, ListState,
    Pixels, ScrollHandle, Window,
};
use gpui_component::scroll::ScrollableElement;

use crate::ui::tokens::Size;
use crate::view_models::list_selection::FocusStep;
use crate::view_models::releases_everything::item_id_runs;

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

/// The state of one virtual Music list (ADR 0084 task 037, decision D1):
/// the gpui [`ListState`] and the element id of each item.
///
/// [`Self::sync`] compares the new ids with the old ids. Each run of added
/// or removed ids is one [`ListState::splice`], so the first row in view
/// stays in view, and an item with the same id keeps its measured size.
pub(crate) struct MusicColumnList {
    state: ListState,
    ids: Vec<String>,
    /// The height that an item counts before its first layout, so the
    /// scrollbar thumb has its correct size.
    item_height: Pixels,
    /// The list width when the items last got [`Self::item_height`].
    /// `None` before the first sync.
    hinted_width: Option<Pixels>,
}

impl MusicColumnList {
    /// An empty list. The overdraw and the item height come from named
    /// tokens.
    #[must_use]
    pub(crate) fn new(cx: &App) -> Self {
        Self {
            state: ListState::new(0, ListAlignment::Top, Size::ListOverdraw.scaled(cx)),
            ids: Vec::new(),
            item_height: Size::MinHitTarget.scaled(cx),
            hinted_width: None,
        }
    }

    /// The gpui state of the list, for the `list` element and its
    /// scrollbar.
    #[must_use]
    pub(crate) const fn state(&self) -> &ListState {
        &self.state
    }

    /// Makes the list hold the items `ids`. Call it at each render of the
    /// list. It never calls [`ListState::reset`], because that clears the
    /// scroll position.
    pub(crate) fn sync(&mut self, ids: &[String], window: &Window) {
        let changed = self.ids != ids;
        if changed {
            for run in item_id_runs(&self.ids, ids).into_iter().rev() {
                self.state.splice(run.old, run.new_count);
            }
            self.ids = ids.to_vec();
        }
        // An item has no height until its first layout, so the scrollbar
        // counts it as 0 px. A splice gives a new item no height, and a
        // layout at a new width removes the height of each item. This call
        // gives each item with no height the item height. A measured item
        // keeps its size, and the scroll position does not change.
        // `ListState` is a shared handle, so the call on a clone changes
        // this state.
        let width = self.state.viewport_bounds().size.width;
        let new_width = self.hinted_width != Some(width);
        if changed || new_width {
            let _ = self
                .state
                .clone()
                .with_uniform_item_height(self.item_height);
            self.hinted_width = Some(width);
        }
        // The layout after a new width removes the heights again, so the
        // next frame gives them one more time.
        if new_width {
            window.request_animation_frame();
        }
    }

    /// Scrolls the list by the smallest distance that shows the item
    /// `index` (ADR 0084 Decision 10).
    pub(crate) fn scroll_to_reveal_item(&self, index: usize) {
        self.state.scroll_to_reveal_item(index);
    }
}

/// Makes the virtual `list` a Music column with the scrollbar of `state`
/// (ADR 0084 Decision 1 and task 037). The scrollbar and its gutter are
/// the same as in [`music_column_tracked_scrollbar`]. Each item of `list`
/// keeps the gutter clear in its own
/// [`page_scroll_content`](super::page_scroll_content).
pub(crate) fn music_column_list_scrollbar(list: List, state: &ListState) -> AnyElement {
    div()
        .relative()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .child(list.size_full())
        .vertical_scrollbar(state)
        .into_any_element()
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
    use gpui::{list, px, ListOffset, TestAppContext};

    /// The window of the R37-07 and R37-08 tests: a virtual Music list of
    /// constructed rows, 40 px high each, in a column 600 px high.
    struct VirtualListTest {
        list: MusicColumnList,
        ids: Vec<String>,
    }

    impl Render for VirtualListTest {
        fn render(
            &mut self,
            window: &mut gpui::Window,
            _cx: &mut gpui::Context<Self>,
        ) -> impl IntoElement {
            self.list.sync(&self.ids, window);
            let ids = Rc::new(self.ids.clone());
            let rows = list(self.list.state().clone(), move |index, _window, _cx| {
                let id = ids[index].clone();
                let selector = id.clone();
                div()
                    .h(px(40.))
                    .debug_selector(move || selector)
                    .child(id)
                    .into_any_element()
            });
            div()
                .w(px(400.))
                .h(px(600.))
                .flex()
                .flex_col()
                .child(music_column_list_scrollbar(rows, self.list.state()))
        }
    }

    fn row_ids(count: usize) -> Vec<String> {
        (1..=count).map(|row| format!("r37-row-{row}")).collect()
    }

    /// R37-07 (ADR 0084 task 037, decision D1): a virtual list of 1,000
    /// rows in a column 600 px high renders row 1 and does not render row
    /// 1,000.
    #[gpui::test]
    fn adr_0084_task_037_r37_07_list_renders_only_the_rows_in_view(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (view, cx) = cx.add_window_view(|_, cx| VirtualListTest {
            list: MusicColumnList::new(cx),
            ids: row_ids(1_000),
        });
        // The first layout gives the list its width. The next frame gives
        // each item that did not render its item height again.
        for _ in 0..2 {
            cx.update(|window, cx| {
                let _ = window.draw(cx);
            });
        }
        assert!(cx.debug_bounds("r37-row-1").is_some(), "row 1 is in view");
        assert!(
            cx.debug_bounds("r37-row-1000").is_none(),
            "row 1,000 is out of view and renders nothing"
        );
        // A row that did not render counts the item height, so the
        // scrollbar thumb has its size for the full list (V5).
        let max_offset = view.update(cx, |test, _| test.list.state().max_offset_for_scrollbar().y);
        assert!(
            max_offset > px(30_000.),
            "the full list counts: {max_offset:?}"
        );
    }

    /// R37-08 (ADR 0084 task 037, Design step 8, packet 036 option A):
    /// with the list scrolled to row 500, a sync that adds one row above
    /// row 500 keeps row 500 as the logical scroll top.
    #[gpui::test]
    fn adr_0084_task_037_r37_08_a_row_above_keeps_the_scroll_top(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (view, cx) = cx.add_window_view(|_, cx| VirtualListTest {
            list: MusicColumnList::new(cx),
            ids: row_ids(1_000),
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        view.update(cx, |test, cx| {
            test.list.state().scroll_to(ListOffset {
                item_ix: 499,
                offset_in_item: px(0.),
            });
            cx.notify();
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        let top = cx.debug_bounds("r37-row-500").expect("row 500 is in view");

        view.update(cx, |test, cx| {
            test.ids.insert(10, "r37-row-added".to_owned());
            cx.notify();
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        let scroll_top = view.update(cx, |test, _| test.list.state().logical_scroll_top());
        assert_eq!(
            scroll_top.item_ix, 500,
            "row 500 is now item 500 and stays at the top"
        );
        assert_eq!(scroll_top.offset_in_item, px(0.));
        assert_eq!(
            cx.debug_bounds("r37-row-500").map(|bounds| bounds.top()),
            Some(top.top()),
            "row 500 does not move"
        );
    }

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
