//! Shell presenter of the "Update n file(s)" button, its popup and its
//! report (ADR 0076 Decision 8).
//!
//! The view model `view_models::tag_update` supplies each text, each mark
//! and the button availability. This presenter maps them to the shared
//! button primitive and the shared confirmation composite.

#![warn(clippy::pedantic)]

use gpui::{div, prelude::*, AnyElement, App, ClickEvent, Context, SharedString, Window};
use gpui_component::WindowExt;

use crate::ui::composites::{
    confirmation_dialog, ConfirmationDialogDisplay, ConfirmationDialogHandlers,
    ConfirmationDialogItem,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::{Button, Label};
use crate::ui::tokens::{FontSize, SemanticColor, Spacing};
use crate::view_models::tag_update::{
    TagUpdateButtonDisplay, TagUpdateMarkRole, TagUpdatePopupDisplay, TagUpdateReportDisplay,
    TagUpdateReportRole,
};

/// Renders the "Update n file(s)" button from its typed display.
pub(crate) fn render_tag_update_button(
    display: TagUpdateButtonDisplay,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    Button::styled(display.button_id, ControlStyle::Secondary)
        .label(display.label)
        .a11y_label(display.a11y_label)
        .disabled(!display.enabled)
        .on_click(on_click)
        .into_any_element()
}

/// Renders the report of the latest confirm. Stacked text uses
/// `overflow_hidden()` and no truncation (ADR 0063).
pub(crate) fn render_tag_update_report(report: TagUpdateReportDisplay, cx: &App) -> AnyElement {
    div()
        .id("tag-update-result")
        .flex()
        .flex_col()
        .gap(Spacing::XXS.scaled(cx))
        .w_full()
        .min_w_0()
        .overflow_hidden()
        .px(Spacing::MD.scaled(cx))
        .py(Spacing::XS.scaled(cx))
        .child(
            Label::new(report.summary)
                .size(FontSize::Micro)
                .color(SemanticColor::SecondaryLabel),
        )
        .children(report.lines.into_iter().map(|line| {
            let color = match line.role {
                TagUpdateReportRole::Written => SemanticColor::SecondaryLabel,
                TagUpdateReportRole::NotWritten => SemanticColor::WarningLabel,
                TagUpdateReportRole::Failed => SemanticColor::DangerLabel,
            };
            Label::new(line.text).size(FontSize::Micro).color(color)
        }))
        .into_any_element()
}

/// Opens the popup with one confirm button and one cancel button.
pub(crate) fn open_tag_update_confirmation_dialog<T>(
    window: &mut Window,
    cx: &mut Context<T>,
    display: TagUpdatePopupDisplay,
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

fn confirmation_display(display: TagUpdatePopupDisplay) -> ConfirmationDialogDisplay {
    ConfirmationDialogDisplay {
        title: SharedString::from(display.title),
        message: SharedString::from(display.message),
        cancel_button_id: SharedString::from(display.cancel_button_id),
        cancel_label: SharedString::from(display.cancel_label),
        cancel_a11y_label: SharedString::from(display.cancel_a11y_label),
        confirm_button_id: SharedString::from(display.confirm_button_id),
        confirm_label: SharedString::from(display.confirm_label),
        confirm_a11y_label: SharedString::from(display.confirm_a11y_label),
        destructive: false,
        items: display
            .files
            .into_iter()
            .map(|file| ConfirmationDialogItem {
                title: SharedString::from(file.title),
                subtitle: file.album.map(SharedString::from),
                mark: file.mark.map(|(label, role)| {
                    let color = match role {
                        TagUpdateMarkRole::InUse => SemanticColor::WarningLabel,
                        TagUpdateMarkRole::Unreadable => SemanticColor::DangerLabel,
                    };
                    (SharedString::from(label), color)
                }),
                lines: file.lines.into_iter().map(SharedString::from).collect(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_models::tag_update::{TagUpdateFileDisplay, IN_USE_MARK};

    #[test]
    fn adr_0076_tag_update_presenter_maps_files_to_dialog_items() {
        let display = TagUpdatePopupDisplay {
            title: "Update 1 file?".into(),
            message: "Message".into(),
            cancel_button_id: "cancel",
            cancel_label: "Cancel",
            cancel_a11y_label: "Close",
            confirm_button_id: "confirm",
            confirm_label: "Write Tags",
            confirm_a11y_label: "Write".into(),
            files: vec![TagUpdateFileDisplay {
                title: "Song 1".into(),
                album: Some("Album One".into()),
                lines: vec!["TIT2: Song 1 (file: Old title)".into()],
                mark: Some((IN_USE_MARK, TagUpdateMarkRole::InUse)),
            }],
        };

        let dialog = confirmation_display(display);

        assert!(!dialog.destructive);
        assert_eq!(dialog.confirm_label, SharedString::from("Write Tags"));
        assert_eq!(dialog.items.len(), 1);
        let item = &dialog.items[0];
        assert_eq!(item.subtitle, Some(SharedString::from("Album One")));
        assert_eq!(
            item.mark,
            Some((SharedString::from(IN_USE_MARK), SemanticColor::WarningLabel))
        );
    }
}
