//! Window-level presentation layers for GPUI component root affordances.

#![warn(clippy::pedantic)]

use gpui::{AnyElement, App, IntoElement, Window};
use gpui_component::Root;

/// Renders the layers managed by `gpui_component::Root`.
///
/// `Root::open_dialog`, `open_sheet`, and notification APIs only update root
/// state. The app shell must also render these layers for them to become
/// visible and interactive.
pub(crate) fn render_window_layers(window: &mut Window, cx: &mut App) -> Vec<AnyElement> {
    let mut layers = Vec::new();

    if let Some(sheet) = Root::render_sheet_layer(window, cx) {
        layers.push(sheet.into_any_element());
    }
    if let Some(dialog) = Root::render_dialog_layer(window, cx) {
        layers.push(dialog.into_any_element());
    }
    if let Some(notification) = Root::render_notification_layer(window, cx) {
        layers.push(notification.into_any_element());
    }

    layers
}
