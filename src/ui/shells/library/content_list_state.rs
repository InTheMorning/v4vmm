//! The state message of the Music content list: loading, empty and failed
//! (ADR 0062).
//!
//! Split out of `content_list.rs` to keep that file under the ADR 0038
//! Task 007 line ceiling (ADR 0086 packet 002).

#![warn(clippy::pedantic)]

use gpui::{div, AnyElement, App, IntoElement, ParentElement, SharedString, Styled};

use crate::ui::primitives::Label;
use crate::ui::tokens::{FontSize, SemanticColor, Spacing};
use crate::view_models::library::ContentListPageStateDisplay;

/// The loading, empty or failed state of a content list (ADR 0062).
pub(crate) fn render_content_list_state(
    display: ContentListPageStateDisplay,
    cx: &App,
) -> AnyElement {
    match display {
        ContentListPageStateDisplay::Loading(display) => div()
            .text_center()
            .p(Spacing::LG.scaled(cx))
            .child(
                Label::new(display.label)
                    .size(FontSize::Caption)
                    .color(SemanticColor::TertiaryLabel),
            )
            .into_any_element(),
        ContentListPageStateDisplay::Empty(display) => render_content_list_message(
            display.title,
            display.secondary,
            SemanticColor::TertiaryLabel,
            cx,
        ),
        ContentListPageStateDisplay::Failed(display) => {
            render_content_list_message(display.title, display.detail, SemanticColor::Danger, cx)
        }
    }
}

fn render_content_list_message(
    title: impl Into<SharedString>,
    detail: impl Into<SharedString>,
    color: SemanticColor,
    cx: &App,
) -> AnyElement {
    div()
        .text_center()
        .p(Spacing::XXL.scaled(cx))
        .child(Label::new(title).size(FontSize::Caption).color(color))
        .child(
            Label::new(detail)
                .size(FontSize::Micro)
                .color(SemanticColor::TertiaryLabel),
        )
        .into_any_element()
}
