//! Shared session-maintenance explanation, actions and report layout (ADR 0066).

use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, App, Window};
use gpui_component::scroll::ScrollableElement;

use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::Button;
use crate::ui::tokens::{color, FontSize, SemanticColor, Spacing};
use crate::view_models::startup::session::{SessionAction, SessionActionDisplay, SessionReportVm};
use crate::view_models::startup::StartupAvailability;

pub(crate) type SessionCallback = Rc<dyn Fn(SessionAction, &mut Window, &mut App)>;

pub(crate) fn session_entry(
    display: SessionActionDisplay,
    generation: u64,
    previous_report: &str,
    callback: SessionCallback,
    cx: &App,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .min_w_0()
        .gap(Spacing::SM.scaled(cx))
        .child(
            div()
                .text_size(FontSize::Title3.scaled(cx))
                .child(SessionReportVm::TITLE),
        )
        .child(
            div()
                .whitespace_normal()
                .child(SessionReportVm::EXPLANATION),
        )
        .child(div().child(SessionReportVm::generation_label(generation)))
        .child(session_button(display, callback))
        .child(div().whitespace_normal().child(previous_report.to_owned()))
        .into_any_element()
}

pub(crate) fn session_drain(
    vm: &SessionReportVm,
    callback: &SessionCallback,
    cx: &App,
) -> AnyElement {
    let mut actions = div().flex().flex_wrap().gap(Spacing::SM.scaled(cx));
    for action in [
        SessionAction::RetryDrain,
        SessionAction::CopyReport,
        SessionAction::Quit,
    ] {
        actions = actions.child(session_button(vm.action(action), callback.clone()));
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
        .child(
            div()
                .text_size(FontSize::Title2.scaled(cx))
                .child(SessionReportVm::TITLE),
        )
        .child(div().whitespace_normal().child(SessionReportVm::WAITING))
        .child(
            div()
                .id("session-maintenance-report")
                .flex_1()
                .min_h_0()
                .min_w_0()
                .overflow_y_scrollbar()
                .whitespace_normal()
                .child(vm.report.clone()),
        )
        .child(actions)
        .into_any_element()
}

fn session_button(display: SessionActionDisplay, callback: SessionCallback) -> Button {
    let action = display.action;
    Button::styled(
        gpui::SharedString::from(format!("session-{action:?}")),
        ControlStyle::Secondary,
    )
    .label(display.label)
    .a11y_label(display.a11y_label)
    .disabled(display.availability != StartupAvailability::Available)
    .on_activate(move |window, cx| callback(action, window, cx))
}
