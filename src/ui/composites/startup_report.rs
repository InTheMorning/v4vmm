//! Responsive startup report, disclosure and actions (ADR 0066).

use std::rc::Rc;

use gpui::{div, prelude::*, App, IntoElement, SharedString, Window};
use gpui_component::scroll::ScrollableElement;

use crate::ui::composites::log_frame::{LogFrame, LogFrames};
use crate::ui::composites::page_scroll_content::page_scroll_content;
use crate::ui::control_styles::ControlStyle;
use crate::ui::layouts;
use crate::ui::primitives::Button;
use crate::ui::tokens::{color, FontSize, SemanticColor, Size, Spacing};
use crate::view_models::log_view::LogSource;
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
    logs: &LogFrames,
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
            .child(LogFrame::new(logs, LogSource::Background, vm.report()));
    }
    for display in rows {
        let mut text = div()
            .min_w_0()
            .flex_auto()
            .flex_basis(Size::ColumnRegular.scaled(cx))
            .flex()
            .flex_col()
            .gap(Spacing::XS.scaled(cx))
            .whitespace_normal()
            .child(display.label);
        if let Some(help) = display.help {
            text = text.child(
                div()
                    .min_w_0()
                    .whitespace_normal()
                    .text_size(FontSize::Caption.scaled(cx))
                    .text_color(color(cx, SemanticColor::SecondaryLabel))
                    .child(help),
            );
        }
        // Keep the report's reading width when actions share a line. If the
        // group no longer fits, move it below the text and wrap its buttons.
        let mut actions = div()
            .min_w_0()
            .max_w_full()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(Spacing::SM.scaled(cx));
        for action in display.actions {
            actions = actions.child(capability_button(action, handler.clone()));
        }
        body = body.child(
            div()
                .min_w_0()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(Spacing::SM.scaled(cx))
                .child(text)
                .child(actions),
        );
    }
    if expanded {
        body = body.child(capability_button(
            vm.action(CapabilityAction::CopyReport),
            handler.clone(),
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
    if expanded {
        return Some(body.into_any_element());
    }
    Some(capability_notice(body, vm, handler, cx))
}

fn capability_notice(
    body: gpui::Div,
    vm: &CapabilityReportVm,
    handler: CapabilityHandler,
    cx: &App,
) -> gpui::AnyElement {
    div()
        .min_w_0()
        .flex()
        .flex_col()
        .flex_shrink_0()
        .bg(color(cx, SemanticColor::SecondarySystemBackground))
        .child(
            div()
                .min_w_0()
                .flex()
                .flex_wrap()
                .items_center()
                .justify_between()
                .gap(Spacing::SM.scaled(cx))
                .px(Spacing::MD.scaled(cx))
                .py(Spacing::XS.scaled(cx))
                .child(
                    div()
                        .text_size(FontSize::Caption.scaled(cx))
                        .child(vm.notice_summary()),
                )
                .child(capability_button(
                    vm.action(CapabilityAction::OpenReport),
                    handler,
                )),
        )
        .child({
            let viewport = div()
                .id("capability-notice-scroll")
                .min_w_0()
                .max_h(layouts::scaled_dimension(
                    layouts::CAPABILITY_NOTICE_MAX_HEIGHT,
                    cx,
                ))
                .overflow_y_scrollbar()
                .child(body);
            #[cfg(test)]
            let viewport = viewport.debug_selector(|| "capability-notice-viewport".to_owned());
            viewport
        })
        .into_any_element()
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

pub(crate) fn startup_report(
    vm: &StartupReportVm,
    handler: Handler,
    editor: Option<gpui::AnyElement>,
    database: Option<gpui::AnyElement>,
    logs: &LogFrames,
    cx: &App,
) -> impl IntoElement {
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
    let mut content = page_scroll_content(cx)
        .gap(Spacing::MD.scaled(cx))
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
        content = content.child(LogFrame::new(logs, LogSource::Startup, vm.report()));
    }
    if let Some(editor) = editor {
        content = content.child(editor);
    }
    if let Some(database) = database {
        content = content.child(database);
    }
    let body = div()
        .id("startup-report-body")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .overflow_y_scrollbar()
        .child(content);
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

#[cfg(test)]
mod notice_tests {
    use gpui::{Context, Render, TestAppContext};

    use super::*;

    struct NoticeTest(CapabilityReportVm);

    impl Render for NoticeTest {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .w(gpui::px(438.))
                .h(gpui::px(700.))
                .flex()
                .flex_col()
                .child(
                    capability_report(
                        &self.0,
                        false,
                        &LogFrames::default(),
                        Rc::new(|_, _, _| {}),
                        cx,
                    )
                    .unwrap(),
                )
                .child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .debug_selector(|| "notice-workspace".to_owned()),
                )
        }
    }

    /// Situational ADR 0066: multiple failures retain a bounded notice and usable workspace.
    #[gpui::test]
    fn adr_0066_normal_notice_preserves_workspace_height(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (_, cx) =
            cx.add_window_view(|_, _| NoticeTest(CapabilityReportVm::setup_failures_fixture()));
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        let notice = cx.debug_bounds("capability-notice-viewport").unwrap();
        let workspace = cx.debug_bounds("notice-workspace").unwrap();
        assert!(notice.size.height <= layouts::CAPABILITY_NOTICE_MAX_HEIGHT);
        assert!(workspace.size.height >= gpui::px(450.));
    }
}
