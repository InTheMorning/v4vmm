//! Shell presenter for local-library removal confirmations.

#![warn(clippy::pedantic)]

use gpui::{App, Context, SharedString, Window};
use gpui_component::WindowExt;

use crate::ui::composites::{
    confirmation_dialog, ConfirmationDialogDisplay, ConfirmationDialogHandlers,
};
use crate::view_models::library_removal::LibraryRemovalConfirmationDisplay;

/// Opens the shared local-library removal confirmation alert.
pub(crate) fn open_library_removal_confirmation_dialog<T>(
    window: &mut Window,
    cx: &mut Context<T>,
    display: LibraryRemovalConfirmationDisplay,
    on_cancel: impl Fn(&mut T, &mut Context<T>) + 'static,
    on_confirm: impl Fn(&mut T, &mut Context<T>) + 'static,
) where
    T: 'static,
{
    let entity = cx.weak_entity();
    let cancel_entity = entity.clone();
    let confirm_entity = entity;
    let handlers = ConfirmationDialogHandlers::new(
        move |_, cx: &mut App| {
            let _ = cancel_entity.update(cx, |this, cx| on_cancel(this, cx));
        },
        move |_, cx: &mut App| {
            let _ = confirm_entity.update(cx, |this, cx| on_confirm(this, cx));
        },
    );
    window.open_dialog(cx, move |dialog, _window, cx| {
        confirmation_dialog(
            dialog,
            confirmation_display(display.clone()),
            handlers.clone(),
            cx,
        )
    });
}

fn confirmation_display(display: LibraryRemovalConfirmationDisplay) -> ConfirmationDialogDisplay {
    ConfirmationDialogDisplay {
        title: SharedString::from(display.title),
        message: SharedString::from(display.message),
        cancel_button_id: SharedString::from(display.cancel_button_id),
        cancel_label: SharedString::from(display.cancel_label),
        cancel_a11y_label: SharedString::from(display.cancel_a11y_label),
        confirm_button_id: SharedString::from(display.remove_button_id),
        confirm_label: SharedString::from(display.remove_label),
        confirm_a11y_label: SharedString::from(display.remove_a11y_label),
        destructive: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_removal_presenter_maps_alert_display_contract() {
        let display = LibraryRemovalConfirmationDisplay {
            title: "Remove Feed from Library?",
            message: "1 track from this feed is in playlists.".into(),
            cancel_button_id: "library-removal-cancel",
            cancel_label: "Cancel",
            cancel_a11y_label: "Cancel removing feed from library",
            remove_button_id: "library-removal-confirm",
            remove_label: "Remove",
            remove_a11y_label: "Remove feed from library",
        };

        let dialog = confirmation_display(display);

        assert_eq!(
            dialog.title,
            SharedString::from("Remove Feed from Library?")
        );
        assert_eq!(dialog.cancel_label, SharedString::from("Cancel"));
        assert_eq!(dialog.confirm_label, SharedString::from("Remove"));
        assert!(dialog.destructive);
    }
}
