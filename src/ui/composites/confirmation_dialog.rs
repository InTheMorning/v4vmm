//! Confirmation dialog composite for focused, content-affecting choices.
//!
//! The composite owns the app's dialog body contract: token-driven text,
//! shared button primitives, explicit Cancel affordance, and no domain logic.
//! Screens provide display-ready strings and callbacks.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{div, prelude::*, App, SharedString, Window};
use gpui_component::{dialog::Dialog, WindowExt};

use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::{Button, ButtonSize, Label, LabelVariant};
use crate::ui::tokens::{SemanticColor, Size, Spacing};

type DialogActionHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfirmationDialogDisplay {
    pub title: SharedString,
    pub message: SharedString,
    pub cancel_button_id: SharedString,
    pub cancel_label: SharedString,
    pub cancel_a11y_label: SharedString,
    pub confirm_button_id: SharedString,
    pub confirm_label: SharedString,
    pub confirm_a11y_label: SharedString,
    pub destructive: bool,
}

#[derive(Clone)]
pub struct ConfirmationDialogHandlers {
    on_cancel: DialogActionHandler,
    on_confirm: DialogActionHandler,
}

impl ConfirmationDialogHandlers {
    pub fn new(
        on_cancel: impl Fn(&mut Window, &mut App) + 'static,
        on_confirm: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            on_cancel: Rc::new(on_cancel),
            on_confirm: Rc::new(on_confirm),
        }
    }
}

#[must_use]
pub fn confirmation_dialog(
    dialog: Dialog,
    display: ConfirmationDialogDisplay,
    handlers: ConfirmationDialogHandlers,
    cx: &App,
) -> Dialog {
    let ConfirmationDialogHandlers {
        on_cancel,
        on_confirm,
    } = handlers;
    let escape_cancel = on_cancel.clone();
    let button_cancel = on_cancel;
    let button_confirm = on_confirm;
    let confirm_style = if display.destructive {
        ControlStyle::Destructive
    } else {
        ControlStyle::Primary
    };

    dialog
        .title(display.title)
        .overlay_closable(false)
        .close_button(false)
        .on_cancel(move |_, window, cx| {
            escape_cancel(window, cx);
            true
        })
        .child(
            div()
                .w(Size::ColumnRegular.scaled(cx))
                .max_w(Size::ColumnTall.scaled(cx))
                .flex()
                .flex_col()
                .gap(Spacing::MD.scaled(cx))
                .child(
                    Label::new(display.message)
                        .variant(LabelVariant::Body)
                        .color(SemanticColor::SecondaryLabel),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .justify_end()
                        .gap(Spacing::SM.scaled(cx))
                        .child(
                            Button::styled(display.cancel_button_id, ControlStyle::Secondary)
                                .size(ButtonSize::Md)
                                .label(display.cancel_label)
                                .a11y_label(display.cancel_a11y_label)
                                .on_click(move |_, window, cx| {
                                    button_cancel(window, cx);
                                    window.close_dialog(cx);
                                }),
                        )
                        .child(
                            Button::styled(display.confirm_button_id, confirm_style)
                                .size(ButtonSize::Md)
                                .label(display.confirm_label)
                                .a11y_label(display.confirm_a11y_label)
                                .on_click(move |_, window, cx| {
                                    button_confirm(window, cx);
                                    window.close_dialog(cx);
                                }),
                        ),
                ),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirmation_display_keeps_cancel_and_confirm_identity() {
        let display = ConfirmationDialogDisplay {
            title: SharedString::from("Remove Track from Library?"),
            message: SharedString::from("This track is in a playlist."),
            cancel_button_id: SharedString::from("cancel"),
            cancel_label: SharedString::from("Cancel"),
            cancel_a11y_label: SharedString::from("Cancel removing track from library"),
            confirm_button_id: SharedString::from("remove"),
            confirm_label: SharedString::from("Remove"),
            confirm_a11y_label: SharedString::from("Remove track from library"),
            destructive: true,
        };

        assert_eq!(display.cancel_label, "Cancel");
        assert_eq!(
            display.cancel_a11y_label,
            SharedString::from("Cancel removing track from library")
        );
        assert_eq!(display.confirm_label, "Remove");
        assert_eq!(
            display.confirm_a11y_label,
            SharedString::from("Remove track from library")
        );
        assert!(display.destructive);
    }
}
