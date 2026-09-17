//! Startup screen wiring; the VM owns admission and the composite owns layout (ADR 0066).

use std::rc::Rc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use gpui::{AppContext, ClipboardItem, Context, Entity, IntoElement, Render, Window};

use crate::application::commands::maintenance::CorrectionAccess;
use crate::application::session_lifecycle::MaintenanceSession;
use crate::presentation::configuration_editor::{
    ConfigurationEditor, CorrectionEvent, CorrectionEventCallback,
};
use crate::presentation::database_tools::DatabaseTools;
use crate::presentation::maintenance_executor::MaintenanceClient;
use crate::presentation::session_transition::SessionTransition;
use crate::presentation::startup_presenter::{mount_current, present_startup};
use crate::startup::{
    CheckIntent, CoreCheckOutcome, CoreResult, StartupBackend, StartupIssue, StartupStage,
};
use crate::ui::composites::maintenance_forms::session_drain;
use crate::ui::composites::startup_report::startup_report;
use crate::view_models::startup::session::{SessionAction, SessionReportVm};
use crate::view_models::startup::{StartupAction, StartupAvailability, StartupReportVm};

use super::{bootstrap, TopApp};

pub(super) struct StartupScreen {
    vm: StartupReportVm,
    log_frames: crate::ui::composites::log_frame::LogFrames,
    worker: Option<MaintenanceClient>,
    backend: Arc<Mutex<StartupBackend>>,
    normal: Option<Entity<TopApp>>,
    opened: Arc<AtomicBool>,
    draining: Option<Arc<Mutex<SessionTransition>>>,
    session_vm: Option<SessionReportVm>,
    maintenance: Option<MaintenanceSession>,
    editor: Option<Entity<ConfigurationEditor>>,
    editor_subscription: Option<gpui::Subscription>,
    database_tools: Option<Entity<DatabaseTools>>,
    database_subscription: Option<gpui::Subscription>,
}
impl StartupScreen {
    pub(super) fn new(worker: Option<MaintenanceClient>, opened: Arc<AtomicBool>) -> Self {
        let available = worker.is_some();
        let mut vm = StartupReportVm::new(available);
        if !available {
            vm.outcome = CoreCheckOutcome::blocked(worker_issue());
        }
        Self {
            vm,
            log_frames: crate::ui::composites::log_frame::LogFrames::default(),
            worker,
            backend: Arc::new(Mutex::new(StartupBackend::new(None))),
            normal: None,
            opened,
            draining: None,
            session_vm: None,
            maintenance: None,
            editor: None,
            editor_subscription: None,
            database_tools: None,
            database_subscription: None,
        }
    }
    pub(super) fn begin(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let parent = cx.weak_entity();
        let callback: CorrectionEventCallback = Rc::new(move |event, window, cx| {
            let parent = parent.clone();
            window.defer(cx, move |window, cx| {
                let _ = parent.update(cx, |this, cx| match event {
                    CorrectionEvent::EndSession => {
                        this.session_action(SessionAction::EndSession, window, cx);
                    }
                    CorrectionEvent::Saved(snapshot, dependencies) => {
                        if let Some(normal) = &this.normal {
                            normal.update(cx, |app, cx| {
                                app.refresh_corrected_settings(&snapshot, window, cx);
                                app.check_saved_capabilities(dependencies, window, cx);
                            });
                        } else {
                            this.vm.return_to_recovery(this.vm.report());
                            cx.notify();
                        }
                    }
                });
            });
        });
        self.editor = Some(cx.new(|cx| {
            ConfigurationEditor::new(
                self.worker.clone(),
                callback,
                self.log_frames.clone(),
                window,
                cx,
            )
        }));
        self.editor_subscription = self
            .editor
            .as_ref()
            .map(|editor| cx.observe(editor, |_, _, cx| cx.notify()));
        self.database_tools = Some(cx.new(|cx| {
            DatabaseTools::new(self.worker.clone(), self.log_frames.clone(), window, cx)
        }));
        self.database_subscription = self
            .database_tools
            .as_ref()
            .map(|tools| cx.observe(tools, |_, _, cx| cx.notify()));
        if self.worker.is_none() {
            eprintln!("{}", self.vm.report());
        }
        self.request(StartupAction::CheckAgain, CheckIntent::Initial, window, cx);
    }
    fn action(&mut self, action: StartupAction, window: &mut Window, cx: &mut Context<Self>) {
        if self.vm.action(action).availability != StartupAvailability::Available {
            return;
        }
        match action {
            StartupAction::CopyReport => {
                cx.write_to_clipboard(ClipboardItem::new_string(self.vm.report()));
            }
            StartupAction::Details => {
                self.vm.details = !self.vm.details;
                cx.notify();
            }
            StartupAction::Quit => {
                self.vm.close();
                cx.quit();
            }
            StartupAction::CheckAgain => self.request(action, CheckIntent::Check, window, cx),
            StartupAction::OpenApp => {
                if let Some(bytes) = self.vm.outcome.checked_bytes() {
                    self.request(
                        action,
                        CheckIntent::Open {
                            checked_bytes: bytes.to_vec(),
                        },
                        window,
                        cx,
                    );
                }
            }
        }
    }
    fn request(
        &mut self,
        action: StartupAction,
        intent: CheckIntent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.vm.maintenance_busy = self
            .editor
            .as_ref()
            .is_some_and(|editor| editor.read(cx).vm.is_working())
            || self
                .database_tools
                .as_ref()
                .is_some_and(|tools| tools.read(cx).vm.is_working());
        let Some(generation) = self.vm.begin(action) else {
            return;
        };
        if self.worker.is_none() {
            return;
        }
        if action == StartupAction::OpenApp {
            if let Some(maintenance) = &mut self.maintenance {
                if !maintenance.begin_resume() {
                    return;
                }
            }
        }
        self.suspend_maintenance_forms(true, cx);
        let backend = self.backend.clone();
        let worker = self.worker.as_ref().expect("worker availability checked");
        let receiver = worker.submit(move || {
            match backend
                .lock()
                .expect("startup worker owns backend")
                .execute(intent)
            {
                CoreResult::Checked(outcome) => Err(outcome),
                CoreResult::Prepared(core) => bootstrap::prepare_normal(*core),
            }
        });
        if let Ok(receiver) = receiver {
            present_startup(
                receiver,
                worker.clone(),
                window,
                cx,
                move |this, result, window, cx| {
                    this.suspend_maintenance_forms(false, cx);
                    if !this.vm.accepts(generation) {
                        if let Some(worker) = &this.worker {
                            worker.retire(result);
                        }
                        return;
                    }
                    match result {
                        Ok(Ok(prepared)) => {
                            if let Some(normal) = mount_current(&mut this.vm, generation, || {
                                bootstrap::mount_normal(prepared, this.worker.clone(), window, cx)
                            }) {
                                this.install_normal_session(normal, cx);
                            }
                        }
                        Ok(Err(outcome)) => {
                            if let Some(maintenance) = &mut this.maintenance {
                                maintenance.resume_failed();
                            }
                            this.vm.complete(generation, outcome);
                            eprintln!("{}", this.vm.report());
                        }
                        Err(_) => {
                            this.vm.worker_available = false;
                            this.vm
                                .complete(generation, CoreCheckOutcome::blocked(worker_issue()));
                            eprintln!("{}", this.vm.report());
                            this.vm.close();
                            cx.quit();
                        }
                    }
                },
            );
        } else {
            self.suspend_maintenance_forms(false, cx);
            if let Some(maintenance) = &mut self.maintenance {
                maintenance.resume_failed();
            }
            self.vm
                .complete(generation, CoreCheckOutcome::blocked(worker_issue()));
        }

        cx.notify();
    }

    fn suspend_maintenance_forms(&self, suspended: bool, cx: &mut Context<Self>) {
        if let Some(tools) = &self.database_tools {
            tools.update(cx, |tools, cx| {
                tools.vm.suspended = suspended;
                cx.notify();
            });
        }
        if let Some(editor) = &self.editor {
            editor.update(cx, |editor, cx| {
                editor.vm.suspended = suspended;
                cx.notify();
            });
        }
    }

    fn install_normal_session(&mut self, normal: Entity<TopApp>, cx: &mut Context<Self>) {
        if let Some(editor) = &self.editor {
            editor.update(cx, |editor, cx| {
                editor.set_access(CorrectionAccess::OptionalOnly, cx);
            });
        }
        let parent = cx.weak_entity();
        let callback: crate::ui::composites::maintenance_forms::SessionCallback =
            Rc::new(move |action, window, cx| {
                let parent = parent.clone();
                window.defer(cx, move |window, cx| {
                    let _ = parent.update(cx, |this, cx| this.session_action(action, window, cx));
                });
            });
        if let Some(report) = &mut self.session_vm {
            let fresh = normal.read(cx).command_runner.session().generation();
            report.resumed(fresh);
        }
        normal.update(cx, |app, cx| {
            app.log_frames.clone_from(&self.log_frames);
            app.database_tools.clone_from(&self.database_tools);
            app.database_tools_subscription = self
                .database_tools
                .as_ref()
                .map(|tools| cx.observe(tools, |_, _, cx| cx.notify()));
            app.configuration_editor.clone_from(&self.editor);
            app.configuration_editor_subscription = self
                .editor
                .as_ref()
                .map(|editor| cx.observe(editor, |_, _, cx| cx.notify()));
            app.session_callback = Some(callback);
            app.previous_session_report = self
                .session_vm
                .as_ref()
                .map_or_else(String::new, |report| report.report.clone());
        });
        self.maintenance.take();
        self.draining.take();
        #[cfg(debug_assertions)]
        {
            let app = normal.read(cx);
            crate::startup::fixture::observe_session(
                &app.command_runner,
                app.cfg_path.clone(),
                app.conn.clone(),
            );
        }
        self.normal = Some(normal);
        self.opened.store(true, Ordering::Release);
    }

    fn session_action(
        &mut self,
        action: SessionAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match action {
            SessionAction::EndSession => {
                if self
                    .database_tools
                    .as_ref()
                    .is_some_and(|tools| tools.read(cx).vm.is_working())
                    || self.draining.is_some()
                    || self
                        .editor
                        .as_ref()
                        .is_some_and(|editor| editor.read(cx).vm.is_working())
                {
                    return;
                }
                let Some(normal) = self.normal.as_ref() else {
                    return;
                };
                let previous = normal.read(cx).capability_vm.report();
                let Some(resources) = normal.update(cx, TopApp::begin_session_drain) else {
                    return;
                };
                let mut report = SessionReportVm::new(resources.drain.session.generation());
                if let Some(prior) = &self.session_vm {
                    report.retain_previous(&prior.report);
                }
                report.retain_previous(&previous);
                self.session_vm = Some(report);
                self.draining = Some(Arc::new(Mutex::new(resources)));
                if let Some(editor) = &self.editor {
                    editor.update(cx, |editor, cx| {
                        editor.vm.suspended = true;
                        cx.notify();
                    });
                }
                self.run_drain(window, cx);
            }
            SessionAction::RetryDrain => {
                if self.session_vm.as_ref().is_some_and(|vm| !vm.working) {
                    self.run_drain(window, cx);
                }
            }
            SessionAction::CopyReport => {
                if let Some(vm) = &self.session_vm {
                    cx.write_to_clipboard(ClipboardItem::new_string(vm.report.clone()));
                }
            }
            SessionAction::Quit => cx.quit(),
        }
        cx.notify();
    }

    fn run_drain(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(resources) = self.draining.clone() else {
            return;
        };
        let Some(worker) = self.worker.as_ref() else {
            return;
        };
        if let Some(vm) = &mut self.session_vm {
            vm.working = true;
        }
        let receiver = worker.submit(move || {
            resources
                .lock()
                .expect("session transition")
                .wait_for_work()
        });
        match receiver {
            Ok(receiver) => present_startup(
                receiver,
                worker.clone(),
                window,
                cx,
                |this, result, window, cx| {
                    match result {
                        Ok(Ok(())) => {
                            // Work acknowledged completion. Unmount children before transferring the
                            // last configured connection and dropping the runtime on the worker.
                            this.normal.take();
                            this.close_session_resources(window, cx);
                        }
                        Ok(Err(pending)) => this.drain_failed(&pending),
                        Err(_) => this.drain_failed(&[
                            "The independent maintenance worker did not return a result".into(),
                        ]),
                    }
                },
            ),
            Err(_) => self.drain_failed(&[
                "The independent maintenance worker is busy or unavailable".into(),
            ]),
        }
        cx.notify();
    }

    fn close_session_resources(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(resources) = self.draining.clone() else {
            return;
        };
        let Some(worker) = self.worker.as_ref() else {
            return;
        };
        let receiver = worker.submit(move || resources.lock().expect("session transition").close());
        match receiver {
            Ok(receiver) => present_startup(
                receiver,
                worker.clone(),
                window,
                cx,
                |this, result, window, cx| match result {
                    Ok(Ok(maintenance)) => {
                        debug_assert_eq!(
                            this.session_vm.as_ref().map(|vm| vm.generation),
                            Some(maintenance.generation())
                        );
                        if let Some(vm) = &mut this.session_vm {
                            vm.released();
                            this.vm.return_to_recovery(vm.report.clone());
                        }
                        this.maintenance = Some(maintenance);
                        if let Some(editor) = &this.editor {
                            editor.update(cx, |editor, cx| {
                                editor.set_access(CorrectionAccess::CoreRecovery, cx);
                            });
                        }
                        this.request(StartupAction::CheckAgain, CheckIntent::Check, window, cx);
                    }
                    Ok(Err(pending)) => this.drain_failed(&pending),
                    Err(_) => this.drain_failed(&[
                        "The independent maintenance worker did not return a result".into(),
                    ]),
                },
            ),
            Err(_) => self.drain_failed(&[
                "The independent maintenance worker is busy or unavailable".into(),
            ]),
        }
    }

    fn drain_failed(&mut self, pending: &[String]) {
        if let Some(vm) = &mut self.session_vm {
            vm.failed(pending);
        }
    }
}
impl Drop for StartupScreen {
    fn drop(&mut self) {
        if let (Some(worker), Some(resources)) = (&self.worker, self.draining.take()) {
            worker.retire(resources);
        }
        self.vm.close();
    }
}
impl Render for StartupScreen {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.draining.is_some() && self.maintenance.is_none() {
            if let Some(vm) = &self.session_vm {
                let entity = cx.weak_entity();
                let callback: crate::ui::composites::maintenance_forms::SessionCallback =
                    Rc::new(move |action, window, cx| {
                        let _ =
                            entity.update(cx, |this, cx| this.session_action(action, window, cx));
                    });
                return session_drain(vm, &callback, &self.log_frames, cx);
            }
        }
        if let Some(normal) = &self.normal {
            return normal.clone().into_any_element();
        }
        let entity = cx.weak_entity();
        self.vm.maintenance_busy = self
            .editor
            .as_ref()
            .is_some_and(|editor| editor.read(cx).vm.is_working())
            || self
                .database_tools
                .as_ref()
                .is_some_and(|tools| tools.read(cx).vm.is_working());
        startup_report(
            &self.vm,
            Rc::new(move |action, window, cx| {
                let _ = entity.update(cx, |this, cx| this.action(action, window, cx));
            }),
            self.editor.clone().map(IntoElement::into_any_element),
            self.database_tools
                .clone()
                .map(IntoElement::into_any_element),
            &self.log_frames,
            cx,
        )
        .into_any_element()
    }
}
fn worker_issue() -> StartupIssue {
    StartupIssue::new(
        StartupStage::Worker,
        None,
        "App could not start or complete work on its independent startup worker.",
        "Copy this report and quit. Free system resources before relaunching the app.",
    )
}
