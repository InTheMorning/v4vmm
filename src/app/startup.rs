//! Startup screen wiring; the VM owns admission and the composite owns layout (ADR 0066).

use std::rc::Rc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use gpui::{ClipboardItem, Context, Entity, IntoElement, Render, Window};

use crate::presentation::maintenance_executor::MaintenanceClient;
use crate::presentation::startup_presenter::{mount_current, present_startup};
use crate::startup::{
    CheckIntent, CoreCheckOutcome, CoreResult, StartupBackend, StartupIssue, StartupStage,
};
use crate::ui::composites::startup_report::startup_report;
use crate::view_models::startup::{StartupAction, StartupAvailability, StartupReportVm};

use super::{bootstrap, TopApp};

pub(super) struct StartupScreen {
    vm: StartupReportVm,
    worker: Option<MaintenanceClient>,
    backend: Arc<Mutex<StartupBackend>>,
    normal: Option<Entity<TopApp>>,
    opened: Arc<AtomicBool>,
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
            worker,
            backend: Arc::new(Mutex::new(StartupBackend::new(None))),
            normal: None,
            opened,
        }
    }
    pub(super) fn begin(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
        let Some(generation) = self.vm.begin(action) else {
            return;
        };
        let Some(worker) = &self.worker else {
            return;
        };
        let backend = self.backend.clone();
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
        match receiver {
            Ok(receiver) => {
                present_startup(receiver, window, cx, move |this, result, window, cx| {
                    if !this.vm.accepts(generation) {
                        return;
                    }
                    match result {
                        Ok(Ok(prepared)) => {
                            if let Some(normal) = mount_current(&mut this.vm, generation, || {
                                bootstrap::mount_normal(prepared, this.worker.clone(), window, cx)
                            }) {
                                this.normal = Some(normal);
                                this.opened.store(true, Ordering::Release);
                            }
                        }
                        Ok(Err(outcome)) => {
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
                });
            }
            Err(_) => {
                self.vm
                    .complete(generation, CoreCheckOutcome::blocked(worker_issue()));
            }
        }
        cx.notify();
    }
}
impl Drop for StartupScreen {
    fn drop(&mut self) {
        self.vm.close();
    }
}
impl Render for StartupScreen {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(normal) = &self.normal {
            return normal.clone().into_any_element();
        }
        let entity = cx.weak_entity();
        startup_report(
            &self.vm,
            Rc::new(move |action, window, cx| {
                let _ = entity.update(cx, |this, cx| this.action(action, window, cx));
            }),
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
