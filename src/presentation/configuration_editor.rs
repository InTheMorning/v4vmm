//! Shared input entity and independent-worker correction marshalling (ADR 0066).

#![warn(clippy::pedantic)]

use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    App, AppContext, ClipboardItem, Context, Entity, FocusHandle, IntoElement, Render,
    Subscription, Window,
};
use gpui_component::input::{InputEvent, TextareaState};

use crate::application::commands::maintenance::{CorrectionAccess, CorrectionResult};
use crate::config::ConfigSnapshot;
use crate::ui::composites::maintenance_forms::{configuration_correction, CorrectionCallback};
use crate::view_models::startup::correction::{CorrectionAction, CorrectionVm};
use crate::view_models::startup::StartupAvailability;

use super::maintenance_executor::MaintenanceClient;
use super::startup_presenter::present_startup;

pub(crate) enum CorrectionEvent {
    EndSession,
    Saved(
        Arc<ConfigSnapshot>,
        Vec<crate::application::capability::Dependency>,
    ),
}

pub(crate) type CorrectionEventCallback = Rc<dyn Fn(CorrectionEvent, &mut Window, &mut App)>;

pub(crate) struct ConfigurationEditor {
    pub(crate) vm: CorrectionVm,
    input: Entity<TextareaState>,
    disclosure_focus: FocusHandle,
    logs: crate::ui::composites::log_frame::LogFrames,
    worker: Option<MaintenanceClient>,
    callback: CorrectionEventCallback,
    requested_field: Option<crate::config::correction::CorrectionField>,
    _subscription: Subscription,
}

impl ConfigurationEditor {
    pub(crate) fn new(
        worker: Option<MaintenanceClient>,
        callback: CorrectionEventCallback,
        logs: crate::ui::composites::log_frame::LogFrames,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = cx.new(|cx| TextareaState::new(window, cx));
        let subscription = cx.subscribe(&input, |this, input, event, cx| {
            if matches!(event, InputEvent::Change) {
                this.vm.edit(input.read(cx).value().to_string());
                cx.notify();
            }
        });
        Self {
            vm: CorrectionVm::new(worker.is_some()),
            input,
            disclosure_focus: cx.focus_handle(),
            logs,
            worker,
            callback,
            requested_field: None,
            _subscription: subscription,
        }
    }

    pub(crate) fn set_access(&mut self, access: CorrectionAccess, cx: &mut Context<Self>) {
        self.vm.access = access;
        self.vm.suspended = false;
        cx.notify();
    }

    pub(crate) fn open_for(
        &mut self,
        field: crate::config::correction::CorrectionField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.requested_field = Some(field);
        self.vm.reopen_editor();
        if self.vm.source.is_none() {
            self.request(CorrectionAction::Load, window, cx);
        } else if self.vm.saved() {
            self.request(CorrectionAction::Reload, window, cx);
        } else {
            self.focus_requested_field(window, cx);
        }
        cx.notify();
    }

    fn focus_requested_field(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.vm.is_working() {
            return;
        }
        if let Some(field) = self.requested_field.take() {
            self.vm.focus_field(field);
            self.sync_input(window, cx);
        }
    }

    fn action(&mut self, action: CorrectionAction, window: &mut Window, cx: &mut Context<Self>) {
        if self.vm.action(action).availability != StartupAvailability::Available {
            return;
        }
        match action {
            CorrectionAction::CloseEditor => {
                self.vm.close_editor();
                self.disclosure_focus.focus(window, cx);
            }
            CorrectionAction::ReopenEditor => self.vm.reopen_editor(),
            CorrectionAction::Select(field) => {
                if self.vm.select(field) {
                    self.sync_input(window, cx);
                }
            }
            CorrectionAction::CopyReport => {
                cx.write_to_clipboard(ClipboardItem::new_string(self.vm.report.clone()));
            }
            CorrectionAction::CopyDraft => {
                cx.write_to_clipboard(ClipboardItem::new_string(self.vm.copy_draft()));
            }
            CorrectionAction::EndSession => {
                (self.callback)(CorrectionEvent::EndSession, window, cx);
            }
            CorrectionAction::Load
            | CorrectionAction::Reload
            | CorrectionAction::Validate
            | CorrectionAction::TestConverter
            | CorrectionAction::Save => self.request(action, window, cx),
        }
        cx.notify();
    }

    fn sync_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let value = self.vm.value();
        self.input
            .update(cx, |input, cx| input.set_value(value, window, cx));
    }

    fn request(&mut self, action: CorrectionAction, window: &mut Window, cx: &mut Context<Self>) {
        // Path resolution does not read/create the document or touch storage.
        let Ok(path) = crate::config::config_path() else {
            self.vm.worker_available = false;
            return;
        };
        let Some((generation, command)) = self.vm.begin(action, path) else {
            return;
        };
        let Some(worker) = &self.worker else {
            return;
        };
        match worker.submit(move || command.execute()) {
            Ok(receiver) => present_startup(
                receiver,
                worker.clone(),
                window,
                cx,
                move |this, result, window, cx| {
                    let result = result.unwrap_or_else(|_| CorrectionResult::Failed("The independent maintenance worker did not return a result. Reload the file before retrying.".into()));
                    let changed = this.vm.changed_dependencies(&result);
                    if let Some(snapshot) = this.vm.complete(generation, result) {
                        (this.callback)(CorrectionEvent::Saved(snapshot, changed), window, cx);
                    }
                    if matches!(action, CorrectionAction::Load | CorrectionAction::Reload) {
                        this.sync_input(window, cx);
                        this.focus_requested_field(window, cx);
                    }
                },
            ),
            Err(_) => {
                self.vm.complete(generation, CorrectionResult::Failed("The independent maintenance worker is busy or unavailable. Try the action again after the current check finishes.".into()));
            }
        }
    }
}

impl Render for ConfigurationEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.weak_entity();
        let callback: CorrectionCallback = Rc::new(move |action, window, cx| {
            let _ = entity.update(cx, |this, cx| this.action(action, window, cx));
        });
        configuration_correction(
            &self.vm,
            &self.input,
            &callback,
            &self.logs,
            &self.disclosure_focus,
            cx,
        )
    }
}
