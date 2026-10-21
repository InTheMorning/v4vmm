//! The status row of a destination list in the scope "Everything" (ADR
//! 0084 packet 016 decision D8): the loading row of page 1, the error row
//! with "Try again", and "Load more".
//!
//! The view model gives each text and each availability. This shell owns
//! only layout and interaction wiring, with named tokens.

#![warn(clippy::pedantic)]

use gpui::{div, prelude::*, AnyElement, ClickEvent, Context, SharedString, Styled};

use crate::library::LibraryApp;
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::{Button as UiButton, Label};
use crate::ui::style::spacing;
use crate::ui::tokens::{FontSize, SemanticColor};
use crate::view_models::index_name_list::{IndexListActionDisplay, IndexListStatusRowVm};

/// A `LibraryApp` action that a status row button runs.
pub(crate) type IndexListAction = fn(&mut LibraryApp, &mut Context<LibraryApp>);

/// The two actions of a status row: "Try again" and "Load more". A page
/// with no "Load more" row, for example the group page of packet 026
/// decision D11, gives `None` for `load_more` instead of a closure that
/// does nothing.
#[derive(Clone, Copy)]
pub(crate) struct IndexListActions {
    pub(crate) retry: IndexListAction,
    pub(crate) load_more: Option<IndexListAction>,
}

/// Renders one status row.
pub(crate) fn render_index_list_status_row(
    status: IndexListStatusRowVm,
    actions: IndexListActions,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    match status {
        IndexListStatusRowVm::Loading { element_id, label } => div()
            .id(SharedString::from(element_id))
            .text_center()
            .p(spacing::LG)
            .child(
                Label::new(label)
                    .size(FontSize::Caption)
                    .color(SemanticColor::TertiaryLabel),
            )
            .into_any_element(),
        IndexListStatusRowVm::Failed {
            element_id,
            message,
            detail,
            retry,
            ..
        } => div()
            .id(SharedString::from(element_id))
            .flex()
            .flex_col()
            .gap(spacing::XS)
            .p(spacing::MD)
            .child(
                Label::new(message)
                    .size(FontSize::Caption)
                    .color(SemanticColor::Danger),
            )
            .child(
                Label::new(detail)
                    .size(FontSize::Micro)
                    .color(SemanticColor::TertiaryLabel),
            )
            .child(action_button(retry, actions.retry, cx))
            .into_any_element(),
        // A page with no "Load more" row sends no `LoadMore` status, so
        // `actions.load_more` is always `Some` here. `None` renders no row,
        // rather than a button with no action.
        IndexListStatusRowVm::LoadMore(load_more) => match actions.load_more {
            Some(action) => div()
                .pt(spacing::SM)
                .child(action_button(load_more, action, cx))
                .into_any_element(),
            None => div().into_any_element(),
        },
    }
}

fn action_button(
    display: IndexListActionDisplay,
    action: IndexListAction,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    UiButton::styled(SharedString::from(display.id), ControlStyle::Ghost)
        .label(display.label)
        .a11y_label(display.a11y_label)
        .disabled(!display.available)
        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
            action(this, cx);
        }))
        .into_any_element()
}
