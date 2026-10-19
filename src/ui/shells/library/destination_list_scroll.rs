//! The Releases and Artists destination lists' scroll containers (ADR
//! 0084 Decisions 1 and 10).
//!
//! Split out of `destination_lists.rs` to keep that file under the ADR
//! 0038 Task 007 line ceiling, the same reason `content_list.rs` keeps
//! its scroll wire-up in the sibling `content_list_scroll.rs`.

#![warn(clippy::pedantic)]

use gpui::{div, prelude::*, AnyElement, Context, ScrollHandle, SharedString, Styled};

use crate::library::LibraryApp;
use crate::ui::composites::music_column_scroll::{
    music_column_scrollbar, music_column_tracked_scrollbar,
};
use crate::ui::composites::page_scroll_content::page_scroll_content;
use crate::ui::style::spacing;
use crate::ui::tokens::SemanticColor;

/// The scroll container of a Releases or Artists destination list.
/// `empty_label` names the row kind for the empty-list message and the
/// scrollbar id.
pub(crate) fn render_destination_list_container(
    children: Vec<AnyElement>,
    empty_label: &'static str,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let mut content = page_scroll_content(cx).gap(spacing::XXS);

    if children.is_empty() {
        content = content.child(
            div()
                .text_center()
                .p(spacing::XXL)
                .text_color(crate::ui::tokens::resolve_color(
                    cx,
                    SemanticColor::TertiaryLabel,
                    None,
                ))
                .child(format!("No {empty_label} yet")),
        );
    } else {
        content = content.children(children);
    }

    let container = div()
        .id("music-destination-list-rows")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .px(spacing::MD)
        .pb(spacing::MD)
        .child(content);

    // ADR 0084 task 014 Recorded Facts: the Releases list and the Artists
    // list call this function from the same line, so the scrollbar id
    // must name the row kind. Without it, the two lists would share one
    // scroll position.
    music_column_scrollbar(
        container,
        SharedString::from(format!(
            "music-destination-list-rows-{empty_label}-scrollbar"
        )),
    )
}

/// The Releases list's scroll container (ADR 0084 Decision 10): a held
/// `scroll_handle`, so a keyboard focus move keeps the focused row in
/// view. `track_scroll` (inside `music_column_tracked_scrollbar`) reads
/// the bounds of its direct children to find a row by index, so this
/// div's rows are its direct children; an outer wrapper would give the
/// tracked handle one child and no way to scroll to one row.
pub(crate) fn render_destination_list_container_tracked(
    children: Vec<AnyElement>,
    empty_label: &'static str,
    scroll_handle: &ScrollHandle,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let mut content = page_scroll_content(cx)
        .id("music-destination-list-rows")
        .gap(spacing::XXS)
        .px(spacing::MD)
        .pb(spacing::MD);

    if children.is_empty() {
        content = content.child(
            div()
                .text_center()
                .p(spacing::XXL)
                .text_color(crate::ui::tokens::resolve_color(
                    cx,
                    SemanticColor::TertiaryLabel,
                    None,
                ))
                .child(format!("No {empty_label} yet")),
        );
    } else {
        content = content.children(children);
    }

    music_column_tracked_scrollbar(content, scroll_handle, None)
}
