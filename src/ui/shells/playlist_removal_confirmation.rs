//! Shell presenter of the "Remove from all playlists" confirmation
//! (ADR 0076 Decision 7, operator decision 2026-09-24).
//!
//! The view model `view_models::playlist_rss_check` supplies each text. The
//! presenter lists each playlist that holds the track through the item list
//! of the shared confirmation composite.

#![warn(clippy::pedantic)]

use gpui::{App, Context, SharedString, Window};
use gpui_component::WindowExt;

use crate::ui::composites::{
    confirmation_dialog, ConfirmationDialogDisplay, ConfirmationDialogHandlers,
    ConfirmationDialogItem,
};
use crate::view_models::playlist_rss_check::RemoveFromAllPlaylistsConfirmationDisplay;

/// Opens the confirmation with one confirm button and one cancel button.
pub(crate) fn open_remove_from_all_playlists_dialog<T>(
    window: &mut Window,
    cx: &mut Context<T>,
    display: RemoveFromAllPlaylistsConfirmationDisplay,
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

fn confirmation_display(
    display: RemoveFromAllPlaylistsConfirmationDisplay,
) -> ConfirmationDialogDisplay {
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
        items: display
            .playlists
            .into_iter()
            .map(|name| ConfirmationDialogItem {
                title: SharedString::from(name),
                subtitle: None,
                mark: None,
                lines: Vec::new(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_models::playlist_rss_check::remove_from_all_playlists_confirmation;

    /// R3-14: the confirmation names each playlist that holds the track.
    #[test]
    fn adr_0076_route_readiness_remove_everywhere_dialog_lists_each_playlist() {
        let display = confirmation_display(remove_from_all_playlists_confirmation(
            "Gone Song",
            &[(1, "Friday Show".to_owned()), (4, "Warm Up".to_owned())],
        ));
        let titles = display
            .items
            .iter()
            .map(|item| item.title.to_string())
            .collect::<Vec<_>>();
        assert_eq!(titles, vec!["Friday Show", "Warm Up"]);
        assert!(display.destructive);
        assert_eq!(
            display.title.as_ref(),
            "Remove Gone Song from all playlists?"
        );
    }
}
