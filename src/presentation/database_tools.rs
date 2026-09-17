//! Shared database input and maintenance-worker marshalling (ADR 0066).

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    App, AppContext, ClipboardItem, Context, Entity, IntoElement, Render, Subscription, Window,
};
use gpui_component::input::{InputEvent, InputState};

use crate::ui::composites::log_frame::LogFrames;
use crate::ui::composites::maintenance_forms::{database_tools, DatabaseCallback};
use crate::view_models::startup::database::{DatabaseAction, DatabaseVm};
use crate::view_models::startup::StartupAvailability;

use super::maintenance_executor::MaintenanceClient;
use super::startup_presenter::present_startup;

pub(crate) enum DatabaseEvent {
    EndSession,
    Preserve(
        u64,
        crate::application::commands::maintenance::DatabaseCommand,
    ),
}
pub(crate) type DatabaseEventCallback = Rc<dyn Fn(DatabaseEvent, &mut Window, &mut App)>;

pub(crate) struct DatabaseTools {
    pub(crate) vm: DatabaseVm,
    source: Entity<InputState>,
    destination: Entity<InputState>,
    worker: Option<MaintenanceClient>,
    logs: LogFrames,
    callback: DatabaseEventCallback,
    _subscriptions: Vec<Subscription>,
}
impl DatabaseTools {
    pub(crate) fn new(
        worker: Option<MaintenanceClient>,
        logs: LogFrames,
        callback: DatabaseEventCallback,
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
            callback,
            _subscriptions: subscriptions,
        }
    }
    fn action(&mut self, action: DatabaseAction, window: &mut Window, cx: &mut Context<Self>) {
        if self.vm.action(action).availability != StartupAvailability::Available {
            return;
        }
        match action {
            DatabaseAction::EndSession => (self.callback)(DatabaseEvent::EndSession, window, cx),
            DatabaseAction::Preserve => {
                if let Some((generation, command)) =
                    self.vm.begin(action, std::path::PathBuf::new())
                {
                    (self.callback)(DatabaseEvent::Preserve(generation, command), window, cx);
                }
            }
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
                                this.complete(generation, result, window, cx);
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

    pub(crate) fn complete(
        &mut self,
        generation: u64,
        result: crate::application::commands::maintenance::DatabaseResult,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let prior_length = self.vm.report.len();
        if self.vm.complete(generation, result) {
            self.source.update(cx, |input, cx| {
                input.set_value(self.vm.source.clone(), window, cx);
            });
        }
        if self.vm.report.len() > prior_length {
            eprintln!("{}", &self.vm.report[prior_length..]);
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
