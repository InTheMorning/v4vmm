//! Shell presenter of the delete confirmation of a saved search (ADR 0084
//! Decision 3, packet 012).
//!
//! The view model `view_models::search_results` supplies each text. "Delete…"
//! is a destructive action, so the operator confirms it first.

#![warn(clippy::pedantic)]

use gpui::{App, Context, SharedString, Window};
use gpui_component::WindowExt;

use crate::ui::composites::{
    confirmation_dialog, ConfirmationDialogDisplay, ConfirmationDialogHandlers,
};
use crate::view_models::search_results::SavedSearchDeleteConfirmation;

/// Opens the confirmation with one confirm button and one cancel button.
pub(crate) fn open_saved_search_delete_dialog<T>(
    window: &mut Window,
    cx: &mut Context<T>,
    display: SavedSearchDeleteConfirmation,
    on_confirm: impl Fn(&mut T, &mut Context<T>) + 'static,
) where
    T: 'static,
{
    let entity = cx.weak_entity();
    let handlers = ConfirmationDialogHandlers::new(
        |_, _: &mut App| {},
        move |_, cx: &mut App| {
            let _ = entity.update(cx, |this, cx| on_confirm(this, cx));
        },
    );
    let display = confirmation_display(display);
    window.open_dialog(cx, move |dialog, _window, cx| {
        confirmation_dialog(dialog, display.clone(), handlers.clone(), cx)
    });
}

fn confirmation_display(display: SavedSearchDeleteConfirmation) -> ConfirmationDialogDisplay {
    ConfirmationDialogDisplay {
        title: SharedString::from(display.title),
        message: SharedString::from(display.message),
        cancel_button_id: SharedString::from(display.cancel_button_id),
        cancel_label: SharedString::from(display.cancel_label),
        cancel_a11y_label: SharedString::from(display.cancel_a11y_label),
        confirm_button_id: SharedString::from(display.confirm_button_id),
        confirm_label: SharedString::from(display.confirm_label),
        confirm_a11y_label: SharedString::from(display.confirm_a11y_label),
        destructive: true,
        items: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::confirmation_display;
    use crate::view_models::search_results::delete_saved_search_confirmation;

    /// R12-08: the dialog is destructive, names the query and gives the
    /// "Delete" button.
    #[test]
    fn adr_0084_saved_search_r12_08_dialog_is_destructive() {
        let display = confirmation_display(delete_saved_search_confirmation("bluegrass"));
        assert!(display.destructive);
        assert_eq!(display.confirm_label.as_ref(), "Delete");
        assert!(display.message.contains("\"bluegrass\""));
    }
}
