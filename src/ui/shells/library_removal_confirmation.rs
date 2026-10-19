//! Shell presenter for local-library removal confirmations.

#![warn(clippy::pedantic)]

use gpui::{App, Context, SharedString, Window};
use gpui_component::WindowExt;

use crate::ui::composites::{
    confirmation_dialog, ConfirmationDialogDisplay, ConfirmationDialogHandlers,
    ConfirmationDialogItem,
};
use crate::ui::tokens::SemanticColor;
use crate::view_models::library_removal::{
    LibraryRemovalConfirmationDisplay, SelectionRemovalConfirmationDisplay,
    SelectionRemovalItemDisplay,
};

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

/// Opens the whole-selection removal confirmation (ADR 0084 Decision
/// 10): one confirmation lists each item of the selection, with the
/// playlists that refer to it.
pub(crate) fn open_selection_removal_confirmation_dialog<T>(
    window: &mut Window,
    cx: &mut Context<T>,
    display: SelectionRemovalConfirmationDisplay,
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
            selection_confirmation_display(display.clone()),
            handlers.clone(),
            cx,
        )
    });
}

fn selection_confirmation_display(
    display: SelectionRemovalConfirmationDisplay,
) -> ConfirmationDialogDisplay {
    ConfirmationDialogDisplay {
        title: SharedString::from(display.title),
        message: SharedString::from(
            "Review each item below. \"Remove\" takes every one out of the Library.",
        ),
        cancel_button_id: SharedString::from(display.cancel_button_id),
        cancel_label: SharedString::from(display.cancel_label),
        cancel_a11y_label: SharedString::from(display.cancel_a11y_label),
        confirm_button_id: SharedString::from(display.remove_button_id),
        confirm_label: SharedString::from(display.remove_label),
        confirm_a11y_label: SharedString::from(display.remove_a11y_label),
        destructive: true,
        items: display
            .items
            .into_iter()
            .map(selection_removal_item)
            .collect(),
    }
}

/// Projects one item of the selection removal confirmation (ADR 0084
/// Decision 10). The mark names each referring playlist, so the warning
/// never relies on color alone.
fn selection_removal_item(item: SelectionRemovalItemDisplay) -> ConfirmationDialogItem {
    let mark = if item.referring_playlists.is_empty() {
        None
    } else {
        Some((
            SharedString::from(format!("In: {}", item.referring_playlists.join(", "))),
            SemanticColor::WarningLabel,
        ))
    };
    ConfirmationDialogItem {
        title: SharedString::from(item.title),
        subtitle: Some(SharedString::from(item.impact_message)),
        mark,
        lines: Vec::new(),
    }
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
        items: Vec::new(),
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

    /// R9-07: the selection removal confirmation's dialog contract lists
    /// each item by its title, with the mark naming each referring
    /// playlist.
    #[test]
    fn adr_0084_selection_r9_07_presenter_maps_each_item_and_playlist_mark() {
        use crate::view_models::library_removal::SelectionRemovalItemDisplay;

        let display = SelectionRemovalConfirmationDisplay {
            title: "Remove 2 tracks from the Library?".to_owned(),
            items: vec![
                SelectionRemovalItemDisplay {
                    title: "Face Pale".to_owned(),
                    impact_message: "This track is in 1 playlist.".to_owned(),
                    referring_playlists: vec!["Late Show".to_owned()],
                },
                SelectionRemovalItemDisplay {
                    title: "Hollow".to_owned(),
                    impact_message: "This track is in no playlist.".to_owned(),
                    referring_playlists: Vec::new(),
                },
            ],
            cancel_button_id: "library-selection-removal-cancel",
            cancel_label: "Cancel",
            cancel_a11y_label: "Cancel removing the selection from the library",
            remove_button_id: "library-selection-removal-confirm",
            remove_label: "Remove",
            remove_a11y_label: "Remove the selection from the library",
        };

        let dialog = selection_confirmation_display(display);

        assert_eq!(dialog.items.len(), 2);
        assert_eq!(dialog.items[0].title, SharedString::from("Face Pale"));
        assert_eq!(
            dialog.items[0].mark,
            Some((
                SharedString::from("In: Late Show"),
                SemanticColor::WarningLabel
            ))
        );
        assert_eq!(dialog.items[1].title, SharedString::from("Hollow"));
        assert_eq!(dialog.items[1].mark, None);
    }
}
