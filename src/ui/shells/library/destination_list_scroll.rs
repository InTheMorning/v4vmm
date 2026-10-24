//! The Releases and Artists destination lists' scroll containers (ADR
//! 0084 Decisions 1 and 10).
//!
//! Split out of `destination_lists.rs` to keep that file under the ADR
//! 0038 Task 007 line ceiling, the same reason `content_list.rs` keeps
//! its scroll wire-up in the sibling `content_list_scroll.rs`.
//!
//! Both lists are virtual lists (ADR 0084 task 037). One frame renders
//! only the items in view and in the overdraw area.

#![warn(clippy::pedantic)]

use gpui::{div, list, prelude::*, AnyElement, App, Context, Styled, Window};

use crate::library::LibraryApp;
use crate::ui::composites::music_column_scroll::{music_column_list_scrollbar, MusicColumnList};
use crate::ui::composites::page_scroll_content::page_scroll_content;
use crate::ui::style::spacing;
use crate::ui::tokens::SemanticColor;

/// The virtual list of a Releases or Artists destination list (ADR 0084
/// task 037, Design step 9). `render_item` renders the item at one index,
/// and `item_count` is the count of items in `list_state`. With no item,
/// the list shows "No `empty_label` yet" and no `list` element (Design
/// step 13).
pub(crate) fn render_destination_virtual_list(
    list_state: &MusicColumnList,
    item_count: usize,
    empty_label: &'static str,
    render_item: impl Fn(usize, &mut Window, &mut App) -> AnyElement + 'static,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    if item_count == 0 {
        return div()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .px(spacing::MD)
            .child(
                div()
                    .text_center()
                    .p(spacing::XXL)
                    .text_color(crate::ui::tokens::resolve_color(
                        cx,
                        SemanticColor::TertiaryLabel,
                        None,
                    ))
                    .child(format!("No {empty_label} yet")),
            )
            .into_any_element();
    }
    let rows = list(list_state.state().clone(), move |index, window, cx| {
        let item = render_item(index, window, cx);
        destination_list_item(index, item, cx)
    })
    .pb(spacing::MD);
    music_column_list_scrollbar(rows, list_state.state())
}

/// One item of a destination list, in its own
/// [`page_scroll_content`], so the scrollbar covers no row content (ADR
/// 0063). The space above each item after the first is the row gap of the
/// list.
fn destination_list_item(index: usize, item: AnyElement, cx: &App) -> AnyElement {
    div()
        .px(spacing::MD)
        .when(index > 0, |el| el.pt(spacing::XXS))
        .child(page_scroll_content(cx).child(item))
        .into_any_element()
}
