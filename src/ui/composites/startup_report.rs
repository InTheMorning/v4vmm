//! Responsive startup report, disclosure and actions (ADR 0066).

use std::rc::Rc;

use gpui::{div, prelude::*, App, IntoElement, SharedString, Window};
use gpui_component::scroll::ScrollableElement;

use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::Button;
use crate::ui::tokens::{color, FontSize, SemanticColor, Spacing};
use crate::view_models::startup::capabilities::{
    CapabilityAction, CapabilityActionDisplay, CapabilityReportVm,
};
use crate::view_models::startup::{StartupAction, StartupAvailability, StartupReportVm};

type Handler = Rc<dyn Fn(StartupAction, &mut Window, &mut App)>;
pub(crate) type CapabilityHandler = Rc<dyn Fn(CapabilityAction, &mut Window, &mut App)>;

/// Shared Settings report and compact normal-shell notice.
pub(crate) fn capability_report(
    vm: &CapabilityReportVm,
    expanded: bool,
    handler: CapabilityHandler,
    cx: &App,
) -> Option<gpui::AnyElement> {
    let rows = vm.rows(expanded);
    if !expanded && rows.is_empty() {
        return None;
    }
    let mut body = div()
        .min_w_0()
        .flex()
        .flex_col()
        .flex_shrink_0()
        .p(Spacing::MD.scaled(cx))
        .gap(Spacing::SM.scaled(cx))
        .bg(color(cx, SemanticColor::SecondarySystemBackground))
        .text_color(color(cx, SemanticColor::Label));
    if expanded {
        body = body
            .child(
                div()
                    .text_size(FontSize::Title3.scaled(cx))
                    .child(CapabilityReportVm::TITLE),
            )
            .child(
                div()
                    .min_w_0()
                    .w_full()
                    .whitespace_normal()
                    .text_size(FontSize::Body.scaled(cx))
                    .child(vm.report()),
            );
    }
    for display in rows {
        let mut row = div()
            .min_w_0()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(Spacing::SM.scaled(cx))
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .whitespace_normal()
                    .child(display.label),
            );
        for action in display.actions {
            row = row.child(capability_button(action, handler.clone()));
        }
        body = body.child(row);
    }
    if expanded {
        body = body.child(capability_button(
            vm.action(CapabilityAction::CopyReport),
            handler,
        ));
    } else if let Some(feedback) = vm.feedback() {
        body = body.child(
            div()
                .min_w_0()
                .whitespace_normal()
                .text_size(FontSize::Caption.scaled(cx))
                .child(feedback),
        );
    }
    Some(body.into_any_element())
}

fn capability_button(display: CapabilityActionDisplay, handler: CapabilityHandler) -> Button {
    let action = display.action;
    Button::styled(
        SharedString::from(format!("capability-{action:?}")),
        ControlStyle::Secondary,
    )
    .label(display.label)
    .a11y_label(display.a11y_label)
    .disabled(display.availability != StartupAvailability::Available)
    .on_activate(move |window, cx| handler(action, window, cx))
}

pub(crate) fn startup_report(vm: &StartupReportVm, handler: Handler, cx: &App) -> impl IntoElement {
    let mut actions = div()
        .flex()
        .flex_wrap()
        .gap(Spacing::SM.scaled(cx))
        .flex_shrink_0();
    for intent in [
        StartupAction::CheckAgain,
        StartupAction::OpenApp,
        StartupAction::CopyReport,
        StartupAction::Quit,
    ] {
        let display = vm.action(intent);
        let callback = handler.clone();
        actions = actions.child(
            Button::styled(
                SharedString::from(format!("startup-{intent:?}")),
                if intent == StartupAction::OpenApp {
                    ControlStyle::Primary
                } else {
                    ControlStyle::Secondary
                },
            )
            .label(display.label)
            .a11y_label(display.a11y_label)
            .disabled(display.availability != StartupAvailability::Available)
            .on_activate(move |window, cx| callback(intent, window, cx)),
        );
    }
    let details = vm.action(StartupAction::Details);
    let mut body = div()
        .id("startup-report-body")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .gap(Spacing::MD.scaled(cx))
        .overflow_y_scrollbar()
        .child(
            div()
                .text_size(FontSize::Body.scaled(cx))
                .child(vm.summary()),
        )
        .child(
            Button::styled("startup-details", ControlStyle::Secondary)
                .label(details.label)
                .a11y_label(details.a11y_label)
                .disabled(details.availability != StartupAvailability::Available)
                .on_activate(move |window, cx| handler(StartupAction::Details, window, cx)),
        );
    if vm.details {
        // Normal text wrapping is deliberate. Full copy uses the same original
        // report string, regardless of the viewport or disclosure state.
        body = body.child(
            div()
                .w_full()
                .min_w_0()
                .whitespace_normal()
                .text_size(FontSize::Body.scaled(cx))
                .child(vm.report()),
        );
    }
    let mut heading = div()
        .flex()
        .flex_col()
        .flex_shrink_0()
        .min_w_0()
        .gap(Spacing::SM.scaled(cx))
        .child(
            div()
                .text_size(FontSize::Title2.scaled(cx))
                .child(vm.title()),
        );
    if let Some(feedback) = vm.feedback() {
        // Keep the last completion above disclosure and the scrolling report.
        heading = heading.child(
            div()
                .id("startup-check-feedback")
                .min_w_0()
                .whitespace_normal()
                .text_size(FontSize::Body.scaled(cx))
                .child(feedback),
        );
    }
    div()
        .size_full()
        .min_w_0()
        .min_h_0()
        .flex()
        .flex_col()
        .p(Spacing::LG.scaled(cx))
        .gap(Spacing::MD.scaled(cx))
        .bg(color(cx, SemanticColor::SystemBackground))
        .text_color(color(cx, SemanticColor::Label))
        .child(heading)
        .child(body)
        .child(actions)
}
