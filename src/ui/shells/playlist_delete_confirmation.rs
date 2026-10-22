//! Shell presenter of the delete confirmation of a playlist (ADR 0083
//! Decision 5, ADR 0084 task 030).
//!
//! The view model `view_models::library` supplies each text. "Delete…" is a
//! destructive action, so the operator confirms it first. Only the confirm
//! handler deletes the playlist.

#![warn(clippy::pedantic)]

use gpui::{App, Context, SharedString, Window};
use gpui_component::WindowExt;

use crate::ui::composites::{
    confirmation_dialog, ConfirmationDialogDisplay, ConfirmationDialogHandlers,
};
use crate::view_models::library::PlaylistDeleteConfirmation;

/// Opens the confirmation with one confirm button and one cancel button.
/// `on_confirm` runs only after a click on the confirm button.
pub(crate) fn open_playlist_delete_dialog<T>(
    window: &mut Window,
    cx: &mut Context<T>,
    display: PlaylistDeleteConfirmation,
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

fn confirmation_display(display: PlaylistDeleteConfirmation) -> ConfirmationDialogDisplay {
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
    use crate::view_models::library::playlist_delete_confirmation;

    /// R30-05 (ADR 0083 Decision 5): the dialog is destructive, names the
    /// playlist and gives "Delete" and "Cancel".
    #[test]
    fn adr_0084_music_open_defects_r30_05_dialog_is_destructive() {
        let display = confirmation_display(playlist_delete_confirmation("Road mix", 3));
        assert!(display.destructive);
        assert_eq!(display.confirm_label.as_ref(), "Delete");
        assert_eq!(display.cancel_label.as_ref(), "Cancel");
        assert!(display.title.contains("\"Road mix\""));
        assert!(display.message.contains("3 entries"));
    }
}
