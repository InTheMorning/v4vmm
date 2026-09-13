//! Content clearance for pages with nested scrolling controls (ADR 0063).

#![warn(clippy::pedantic)]

use gpui::{div, prelude::*, App, Div};

use crate::ui::layouts::OVERLAY_SCROLLBAR_WIDTH;
use crate::ui::tokens::Spacing;

/// Mount inside the page's scrolling area, with its overlay scrollbar outside.
pub(crate) fn page_scroll_content(cx: &App) -> Div {
    div()
        .flex()
        .flex_col()
        .flex_shrink_0()
        .min_w_0()
        .pr(OVERLAY_SCROLLBAR_WIDTH + Spacing::SM.scaled(cx))
}
