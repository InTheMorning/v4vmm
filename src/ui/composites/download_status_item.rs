//! Download progress item composite (ADR 0084 Decision 10, packet 022).
//!
//! The item shows the download batch on each section, for example
//! "Downloading 3 of 9". It renders the display facts of
//! [`DownloadProgressDisplay`] and exposes one callback: "Dismiss" of a
//! finished batch. It makes no text, no count and no time. The text wraps,
//! so a narrow window does not clip it (ADR 0063).

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    div, prelude::*, App, ClickEvent, FontWeight, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window,
};

use crate::ui::control_styles::ControlStyle;
use crate::ui::icons::{Icon, IconName, IconSize};
use crate::ui::primitives::{Button as UiButton, Tooltip};
use crate::ui::tokens::{color, FontSize, SemanticColor, Spacing};
use crate::view_models::download_progress::{
    DownloadProgressDismissAction, DownloadProgressDisplay, DownloadProgressIconRole,
};

type DismissHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// Download progress item element.
#[derive(IntoElement)]
#[must_use]
pub(crate) struct DownloadStatusItem {
    display: DownloadProgressDisplay,
    on_dismiss: DismissHandler,
}

/// Creates a download progress item. `on_dismiss` runs when the operator
/// clicks "Dismiss" of a finished batch.
pub(crate) fn download_status_item(
    display: DownloadProgressDisplay,
    on_dismiss: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> DownloadStatusItem {
    DownloadStatusItem {
        display,
        on_dismiss: Rc::new(on_dismiss),
    }
}

impl RenderOnce for DownloadStatusItem {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let DownloadProgressDisplay {
            id,
            icon_role,
            title,
            item_name,
            failure_text,
            failure_lines,
            a11y_label,
            dismiss,
            ..
        } = self.display;
        let icon_color = color(cx, icon_color(icon_role));
        let tooltip = Tooltip::new(a11y_label);

        div()
            .id(id)
            .flex()
            .flex_row()
            .items_start()
            .gap(Spacing::MD.scaled(cx))
            .flex_shrink_0()
            .min_w_0()
            .px(Spacing::XL.scaled(cx))
            .py(Spacing::XS.scaled(cx))
            .border_b_1()
            .border_color(color(cx, SemanticColor::Separator))
            .bg(color(cx, SemanticColor::SecondarySystemBackground))
            .child(
                div()
                    .flex_shrink_0()
                    .text_color(icon_color)
                    .child(Icon::new(icon(icon_role)).size(IconSize::Action)),
            )
            .child(
                div()
                    .id("download-progress-summary")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap(Spacing::XXS.scaled(cx))
                    .tooltip(move |window, cx| tooltip.build(window, cx))
                    .child(
                        div()
                            .text_size(FontSize::Body.scaled(cx))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(color(cx, SemanticColor::Label))
                            .child(SharedString::from(title)),
                    )
                    .when(item_name.is_some() || failure_text.is_some(), |el| {
                        el.child(render_detail_line(item_name, failure_text, cx))
                    })
                    .children(failure_lines.into_iter().map(|line| {
                        div()
                            .text_size(FontSize::Caption.scaled(cx))
                            .text_color(color(cx, SemanticColor::DangerLabel))
                            .child(SharedString::from(line))
                    })),
            )
            .when_some(dismiss, |el, dismiss| {
                el.child(render_dismiss_button(dismiss, &self.on_dismiss))
            })
    }
}

fn render_detail_line(
    item_name: Option<String>,
    failure_text: Option<String>,
    cx: &App,
) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .gap(Spacing::SM.scaled(cx))
        .text_size(FontSize::Caption.scaled(cx))
        .when_some(item_name, |el, name| {
            el.child(
                div()
                    .min_w_0()
                    .text_color(color(cx, SemanticColor::SecondaryLabel))
                    .child(SharedString::from(name)),
            )
        })
        .when_some(failure_text, |el, text| {
            el.child(
                div()
                    .flex_shrink_0()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(color(cx, SemanticColor::WarningLabel))
                    .child(SharedString::from(text)),
            )
        })
}

fn render_dismiss_button(
    dismiss: DownloadProgressDismissAction,
    handler: &DismissHandler,
) -> UiButton {
    let handler = Rc::clone(handler);
    UiButton::styled(dismiss.id, ControlStyle::Secondary)
        .leading_icon(IconName::Close)
        .label(dismiss.label)
        .a11y_label(dismiss.a11y_label)
        .tooltip(dismiss.a11y_label)
        .on_click(move |event, window, cx| handler(event, window, cx))
}

const fn icon(role: DownloadProgressIconRole) -> IconName {
    match role {
        DownloadProgressIconRole::InProgress => IconName::Info,
        DownloadProgressIconRole::Success => IconName::Check,
        DownloadProgressIconRole::Warning => IconName::Warning,
    }
}

const fn icon_color(role: DownloadProgressIconRole) -> SemanticColor {
    match role {
        DownloadProgressIconRole::InProgress => SemanticColor::InfoLabel,
        DownloadProgressIconRole::Success => SemanticColor::SuccessLabel,
        DownloadProgressIconRole::Warning => SemanticColor::WarningLabel,
    }
}
