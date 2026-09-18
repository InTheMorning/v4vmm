//! Shared session-maintenance explanation, actions and report layout (ADR 0066).

use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, App, Window};
use gpui_component::StyleSized as _;

use crate::ui::composites::log_frame::{LogFrame, LogFrames};
use crate::ui::control_styles::ControlStyle;
use crate::ui::layouts::{scaled_dimension, CONFIGURATION_EDITOR_HEIGHT};
use crate::ui::primitives::primary_selection::PrimarySelectionExt as _;
use crate::ui::primitives::Button;
use crate::ui::sizable_bridge::SizableScaled as _;
use crate::ui::tokens::{color, FontSize, SemanticColor, Spacing};
use crate::view_models::log_view::LogSource;
use crate::view_models::startup::session::{SessionAction, SessionActionDisplay, SessionReportVm};
use crate::view_models::startup::StartupAvailability;

pub(crate) type CorrectionCallback =
    Rc<dyn Fn(crate::view_models::startup::correction::CorrectionAction, &mut Window, &mut App)>;

/// The same editor geometry is mounted in Settings and core recovery.
pub(crate) fn configuration_correction(
    vm: &crate::view_models::startup::correction::CorrectionVm,
    input: &gpui::Entity<gpui_component::input::TextareaState>,
    callback: &CorrectionCallback,
    logs: &LogFrames,
    disclosure_focus: &gpui::FocusHandle,
    cx: &App,
) -> AnyElement {
    use crate::view_models::startup::correction::{CorrectionAction, CorrectionVm};
    let close = vm.action(CorrectionAction::CloseEditor);
    let close_focus = disclosure_focus.clone();
    let mut body = div()
        .flex()
        .flex_col()
        .flex_shrink_0()
        .w_full()
        .min_w_0()
        .when(
            close.availability == StartupAvailability::Available,
            |body| {
                body.on_action(move |_: &gpui_component::input::Escape, window, cx| {
                    cx.stop_propagation();
                    close_focus.focus(window, cx);
                })
            },
        )
        .gap(Spacing::SM.scaled(cx))
        .child(configuration_header(vm, callback, disclosure_focus, cx))
        .child(div().whitespace_normal().child(CorrectionVm::EXPLANATION));
    if !vm.editor_open() {
        body = body.child(
            correction_button(vm.entry_action(), callback.clone()).track_focus(disclosure_focus),
        );
    } else if vm.source.is_none() {
        body = body.child(correction_button(
            vm.action(CorrectionAction::Load),
            callback.clone(),
        ));
    } else {
        let mut fields = div()
            .flex()
            .flex_wrap()
            .min_w_0()
            .gap(Spacing::XS.scaled(cx));
        for field in vm.fields() {
            fields = fields.child(correction_button(
                vm.action(CorrectionAction::Select(field)),
                callback.clone(),
            ));
        }
        body = body.child(fields);
        if vm.converter_selected() {
            use crate::view_models::startup::converter;
            body = body
                .child(
                    div()
                        .text_size(FontSize::Title3.scaled(cx))
                        .child(converter::TITLE),
                )
                .child(div().whitespace_normal().child(vm.configured_converter()))
                .child(div().whitespace_normal().child(converter::HELP))
                .child(div().whitespace_normal().child(converter::INSTALLATION));
        }
        body = body
            .child(div().whitespace_normal().child(CorrectionVm::CLOSE_HELP))
            .child(div().whitespace_normal().child(vm.input_help()))
            .child(configuration_input_frame(vm, input, cx));
        let mut actions = div().flex().flex_wrap().gap(Spacing::SM.scaled(cx));
        if vm.converter_selected() {
            actions = actions.child(correction_button(
                vm.action(CorrectionAction::TestConverter),
                callback.clone(),
            ));
        }
        for action in [
            CorrectionAction::Validate,
            CorrectionAction::Save,
            CorrectionAction::CopyDraft,
            CorrectionAction::Reload,
        ] {
            actions = actions.child(correction_button(vm.action(action), callback.clone()));
        }
        if vm.needs_maintenance() {
            actions = actions.child(correction_button(
                vm.action(CorrectionAction::EndSession),
                callback.clone(),
            ));
        }
        body = body.child(actions);
    }
    if let Some(message) = vm.work_message() {
        body = body.child(div().whitespace_normal().child(message));
    }
    if !vm.report.is_empty() {
        body = body
            .child(LogFrame::new(
                logs,
                LogSource::Configuration,
                vm.report.clone(),
            ))
            .child(correction_button(
                vm.action(CorrectionAction::CopyReport),
                callback.clone(),
            ));
    }
    body.into_any_element()
}

fn configuration_header(
    vm: &crate::view_models::startup::correction::CorrectionVm,
    callback: &CorrectionCallback,
    disclosure_focus: &gpui::FocusHandle,
    cx: &App,
) -> gpui::Div {
    use crate::view_models::startup::correction::{CorrectionAction, CorrectionVm};
    div()
        .flex()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap(Spacing::SM.scaled(cx))
        .child(
            div()
                .text_size(FontSize::Title3.scaled(cx))
                .child(CorrectionVm::TITLE),
        )
        .when(vm.editor_open(), |header| {
            header.child(
                correction_button(vm.action(CorrectionAction::CloseEditor), callback.clone())
                    .track_focus(disclosure_focus),
            )
        })
}

fn configuration_input_frame(
    vm: &crate::view_models::startup::correction::CorrectionVm,
    input: &gpui::Entity<gpui_component::input::TextareaState>,
    cx: &App,
) -> gpui::Div {
    let widget = gpui_component::input::Textarea::new(input)
        .disabled(!vm.input_enabled())
        .input_text_size(crate::ui::sizable_bridge::scaled(
            gpui_component::Size::Small,
            cx,
        ))
        .h_full()
        .absolute()
        .inset_0()
        .w_auto()
        .min_w_0()
        .with_primary_selection(input);
    // Percentage width resolved to zero here while auto-width siblings
    // stretched correctly. Let the column allocate this viewport (ADR 0066 V1).
    let mut frame = div()
        .relative()
        .w_auto()
        .h(scaled_dimension(CONFIGURATION_EDITOR_HEIGHT, cx))
        .flex_shrink_0()
        .min_w_0()
        .child(widget);
    frame.style().align_self = Some(gpui::AlignItems::Stretch);
    frame
}

fn correction_button(
    display: crate::view_models::startup::correction::CorrectionActionDisplay,
    callback: CorrectionCallback,
) -> Button {
    let action = display.action;
    Button::styled(
        gpui::SharedString::from(format!("correction-{action:?}")),
        ControlStyle::Secondary,
    )
    .label(display.label)
    .a11y_label(display.a11y_label)
    .disabled(display.availability != StartupAvailability::Available)
    .on_activate(move |window, cx| callback(action, window, cx))
}

pub(crate) type SessionCallback = Rc<dyn Fn(SessionAction, &mut Window, &mut App)>;

pub(crate) fn session_entry(
    display: SessionActionDisplay,
    generation: u64,
    previous_report: &str,
    logs: &LogFrames,
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
        .when(!previous_report.is_empty(), |body| {
            body.child(LogFrame::new(logs, LogSource::Session, previous_report))
        })
        .into_any_element()
}

pub(crate) fn session_drain(
    vm: &SessionReportVm,
    callback: &SessionCallback,
    logs: &LogFrames,
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
        .child(LogFrame::new(logs, LogSource::Session, vm.report.clone()).fill())
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

pub(crate) type DatabaseCallback =
    Rc<dyn Fn(crate::view_models::startup::database::DatabaseAction, &mut Window, &mut App)>;

/// Database tools use the same geometry and log owner in Settings and core recovery.
pub(crate) fn database_tools(
    vm: &crate::view_models::startup::database::DatabaseVm,
    inputs: [&gpui::Entity<gpui_component::input::InputState>; 3],
    callback: &DatabaseCallback,
    logs: &LogFrames,
    cx: &App,
) -> AnyElement {
    use crate::view_models::startup::database::{DatabaseAction, DatabaseVm};
    let mut body = div()
        .flex()
        .flex_col()
        .flex_shrink_0()
        .w_full()
        .min_w_0()
        .gap(Spacing::SM.scaled(cx))
        .child(
            div()
                .text_size(FontSize::Title3.scaled(cx))
                .child(DatabaseVm::TITLE),
        )
        .child(div().whitespace_normal().child(DatabaseVm::SCOPE))
        .child(div().whitespace_normal().child(DatabaseVm::HELP))
        .child(div().whitespace_normal().child(DatabaseVm::PRESERVATION));
    for (label, input) in [
        DatabaseVm::SOURCE,
        DatabaseVm::DESTINATION,
        DatabaseVm::RESTORE_SOURCE,
    ]
    .into_iter()
    .zip(inputs)
    {
        body = body.child(div().whitespace_normal().child(label)).child(
            div().w_full().min_w_0().flex().flex_row().child(
                gpui_component::input::Input::new(input)
                    .scaled(gpui_component::Size::Small, cx)
                    .flex_1()
                    .min_w_0()
                    .disabled(!vm.input_enabled())
                    .input_text_size(crate::ui::sizable_bridge::scaled(
                        gpui_component::Size::Small,
                        cx,
                    ))
                    .with_primary_selection(input),
            ),
        );
    }
    body = body.child(div().whitespace_normal().child(DatabaseVm::RESTORE_HELP));
    if let Some(confirmation) = vm.restore_confirmation() {
        body = body.child(div().whitespace_normal().child(confirmation));
    }
    let mut actions = div()
        .flex()
        .flex_wrap()
        .min_w_0()
        .gap(Spacing::SM.scaled(cx));
    for action in [
        DatabaseAction::ConfiguredSource,
        DatabaseAction::Check,
        DatabaseAction::Backup,
        DatabaseAction::EndSession,
        DatabaseAction::Preserve,
        DatabaseAction::ReviewRestore,
        DatabaseAction::Restore,
        DatabaseAction::Cancel,
        DatabaseAction::CopyReport,
    ] {
        let display = vm.action(action);
        let callback = callback.clone();
        actions = actions.child(
            Button::styled(
                gpui::SharedString::from(format!("database-{:?}", display.action)),
                if display.destructive {
                    ControlStyle::Destructive
                } else {
                    ControlStyle::Secondary
                },
            )
            .label(display.label)
            .a11y_label(display.a11y_label)
            .disabled(display.availability != StartupAvailability::Available)
            .on_activate(move |window, cx| callback(action, window, cx)),
        );
    }
    body = body.child(actions);
    if let Some(message) = vm.working_message() {
        body = body.child(div().whitespace_normal().child(message));
    }
    if !vm.report.is_empty() {
        body = body.child(LogFrame::new(logs, LogSource::Database, vm.report.clone()));
    }
    body.into_any_element()
}
