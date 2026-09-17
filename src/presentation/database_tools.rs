//! Shared database input and maintenance-worker marshalling (ADR 0066).

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{AppContext, ClipboardItem, Context, Entity, IntoElement, Render, Subscription, Window};
use gpui_component::input::{InputEvent, InputState};

use crate::ui::composites::log_frame::LogFrames;
use crate::ui::composites::maintenance_forms::{database_tools, DatabaseCallback};
use crate::view_models::startup::database::{DatabaseAction, DatabaseVm};
use crate::view_models::startup::StartupAvailability;

use super::maintenance_executor::MaintenanceClient;
use super::startup_presenter::present_startup;

pub(crate) struct DatabaseTools {
    pub(crate) vm: DatabaseVm,
    source: Entity<InputState>,
    destination: Entity<InputState>,
    worker: Option<MaintenanceClient>,
    logs: LogFrames,
    _subscriptions: Vec<Subscription>,
}
impl DatabaseTools {
    pub(crate) fn new(
        worker: Option<MaintenanceClient>,
        logs: LogFrames,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let source = cx.new(|cx| InputState::new(window, cx));
        let destination = cx.new(|cx| InputState::new(window, cx));
        let subscriptions = vec![
            cx.subscribe(&source, |this, input, event, cx| {
                if matches!(event, InputEvent::Change) && this.vm.input_enabled() {
                    this.vm.source = input.read(cx).value().to_string();
                    cx.notify();
                }
            }),
            cx.subscribe(&destination, |this, input, event, cx| {
                if matches!(event, InputEvent::Change) && this.vm.input_enabled() {
                    this.vm.destination = input.read(cx).value().to_string();
                    cx.notify();
                }
            }),
        ];
        Self {
            vm: DatabaseVm::new(worker.is_some()),
            source,
            destination,
            worker,
            logs,
            _subscriptions: subscriptions,
        }
    }
    fn action(&mut self, action: DatabaseAction, window: &mut Window, cx: &mut Context<Self>) {
        if self.vm.action(action).availability != StartupAvailability::Available {
            return;
        }
        match action {
            DatabaseAction::Cancel => self.vm.cancel(),
            DatabaseAction::CopyReport => {
                cx.write_to_clipboard(ClipboardItem::new_string(self.vm.report.clone()));
            }
            DatabaseAction::ConfiguredSource | DatabaseAction::Check | DatabaseAction::Backup => {
                let path = crate::config::config_path().unwrap_or_default();
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
                        move |this, result, window, cx| match result {
                            Ok(result) => {
                                let prior_length = this.vm.report.len();
                                if this.vm.complete(generation, result) {
                                    this.source.update(cx, |input, cx| {
                                        input.set_value(this.vm.source.clone(), window, cx);
                                    });
                                }
                                if this.vm.report.len() > prior_length {
                                    eprintln!("{}", &this.vm.report[prior_length..]);
                                }
                            }
                            Err(_) => this
                                .vm
                                .unavailable(generation, std::time::SystemTime::now()),
                        },
                    ),
                    Err(_) => self
                        .vm
                        .unavailable(generation, std::time::SystemTime::now()),
                }
            }
        }
        cx.notify();
    }
}
impl Render for DatabaseTools {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.weak_entity();
        let callback: DatabaseCallback = Rc::new(move |action, window, cx| {
            let _ = entity.update(cx, |this, cx| this.action(action, window, cx));
        });
        database_tools(
            &self.vm,
            [&self.source, &self.destination],
            &callback,
            &self.logs,
            cx,
        )
    }
}
