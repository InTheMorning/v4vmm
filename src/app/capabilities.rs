//! Shell wiring for scoped background issues and explicit retries (ADR 0066).

use std::rc::Rc;
use std::sync::Arc;

use gpui::{ClipboardItem, Context, Window};

use crate::application::capability::{
    CapabilityAction, CapabilityFailure, CapabilityObservation, CapabilityObservations, Dependency,
};
use crate::application::capability_recovery::setup::{self, CapabilityCheck, PreparedCapability};
use crate::application::capability_recovery::{RecoveryAction, RecoveryIntent, RetryCommand};
use crate::application::AsyncCommandRunner;
use crate::presentation::maintenance_executor::MaintenanceClient;
use crate::presentation::startup_presenter::present_startup;
use crate::presentation::{bridge_watch, RuntimeHost};
use crate::ui::composites::startup_report::capability_report;
use crate::view_models::startup::capabilities::CapabilityReportVm;
use crate::view_models::startup::capabilities::{canonical_dependency, correction_field};
use crate::view_models::startup::StartupAvailability;

use super::{bootstrap, AppTab, TopApp};

enum RecoveredCapability {
    Runtime(Arc<RuntimeHost>, Option<Arc<crate::config::ConfigSnapshot>>),
    Optional(Result<CapabilityCheck, &'static str>),
    Cache,
}

impl TopApp {
    pub(super) fn install_capability_controls(
        &mut self,
        observations: CapabilityObservations,
        worker: Option<MaintenanceClient>,
        cx: &mut Context<Self>,
    ) {
        let mut receiver = observations.subscribe();
        self.capability_vm =
            CapabilityReportVm::new(receiver.borrow_and_update().clone(), worker.is_some());
        self.capability_observations = observations;
        self.maintenance_worker = worker;
        bridge_watch(
            receiver,
            |this: &mut Self, snapshot, _cx| {
                this.capability_vm.observations = snapshot;
            },
            cx,
        );
    }

    pub(super) fn render_capabilities(
        &self,
        expanded: bool,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let entity = cx.weak_entity();
        capability_report(
            &self.capability_vm,
            expanded,
            &self.log_frames,
            Rc::new(move |action, window, cx| {
                let _ = entity.update(cx, |this, cx| this.capability_action(action, window, cx));
            }),
            cx,
        )
    }

    pub(super) fn capability_action(
        &mut self,
        action: CapabilityAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.capability_vm.action(action).availability != StartupAvailability::Available {
            return;
        }
        match action {
            CapabilityAction::OpenReport | CapabilityAction::Review(_) => {
                self.settings
                    .dispatch(crate::view_models::settings::SettingsAction::OpenReport);
                self.settings_scroll.show_start(self.settings.selected());
                self.select_tab(AppTab::Settings, window, cx);
            }
            CapabilityAction::Repair(id) => {
                if let Some(entry) = self
                    .capability_vm
                    .pending
                    .entries()
                    .iter()
                    .find(|entry| entry.id == id)
                {
                    self.capability_action(
                        CapabilityAction::Configure(entry.dependency),
                        window,
                        cx,
                    );
                }
            }
            CapabilityAction::Verify(id) => {
                if let Some(entry) = self
                    .capability_vm
                    .pending
                    .entries()
                    .iter()
                    .find(|entry| entry.id == id)
                {
                    self.check_capability(entry.dependency, window, cx);
                }
            }
            CapabilityAction::Retry(id) => self.retry_retained_action(id, window, cx),
            CapabilityAction::Dismiss(id) => self.capability_vm.pending.dismiss(id),
            CapabilityAction::Configure(dependency) => {
                let route = if correction_field(dependency).is_some() {
                    crate::view_models::settings::SettingsAction::OpenRepair
                } else {
                    crate::view_models::settings::SettingsAction::OpenReport
                };
                self.settings.dispatch(route);
                self.settings_scroll.show_start(self.settings.selected());
                self.select_tab(AppTab::Settings, window, cx);
                if let (Some(field), Some(editor)) =
                    (correction_field(dependency), &self.configuration_editor)
                {
                    editor.update(cx, |editor, cx| editor.open_for(field, window, cx));
                }
            }
            CapabilityAction::CopyReport => {
                cx.write_to_clipboard(ClipboardItem::new_string(self.capability_vm.report()));
            }
            CapabilityAction::CheckAgain(dependency) => {
                self.check_capability(dependency, window, cx);
            }
        }
        cx.notify();
    }

    pub(super) fn check_capability(
        &mut self,
        dependency: Dependency,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let dependency = canonical_dependency(dependency);
        self.capability_vm.repair_blocked = self.show_commands.repair_blocked();
        let Some(generation) = self.capability_vm.begin(dependency) else {
            self.capability_vm.check_message = Some(format!(
                "[{}] App could not check {} while another operation is running or the checker is unavailable. App kept the loaded tool unchanged. Use Check again when the operation finishes.",
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                crate::view_models::startup::capabilities::dependency_title(dependency)
            ));
            self.capability_observations
                .record(CapabilityObservation::new(
                    dependency,
                    Some(CapabilityFailure::Preparation),
                ));
            self.capability_vm.observations = self.capability_observations.snapshot();
            self.reproject_show_page_from_current_queue();
            if let Some(next) = self.capability_vm.queued.pop_front() {
                self.check_capability(next, window, cx);
            }
            cx.notify();
            return;
        };
        let Some(worker) = self.maintenance_worker.clone() else {
            return;
        };
        self.reproject_show_page_from_current_queue();
        let cfg_path = self.cfg_path.clone();
        let cache = self.image_cache.clone();
        let session = self.command_runner.session().clone();
        let player = self.playback_owner.clone();
        let existing_runtime = self.runtime_host.clone();
        let music_dir = self.music_dir.clone();
        let conn = self.conn.clone();
        let producer = self
            .feature_availability()
            .require(Dependency::Producer)
            .ok()
            .and_then(|()| self.broadcast.drop_file_producer().ok().flatten());
        let receiver = worker.submit(move || match dependency {
            Dependency::BackgroundRuntime => existing_runtime
                .map_or_else(|| RuntimeHost::for_config(&cfg_path, session), Ok)
                .map(|host| {
                    RecoveredCapability::Runtime(
                        host,
                        crate::config::ConfigSnapshot::read_existing(&cfg_path)
                            .ok()
                            .map(Arc::new),
                    )
                })
                .map_err(|error| CapabilityFailure::RuntimeStart(error.kind())),
            Dependency::ThumbnailMaintenance => cache
                .check_maintenance(bootstrap::cache_worker_for_config(&cfg_path))
                .map(|()| RecoveredCapability::Cache),
            _ => Ok(RecoveredCapability::Optional(setup::check(
                &cfg_path, dependency, player, &music_dir, &conn, producer,
            ))),
        });
        match receiver {
            Ok(receiver) => present_startup(
                receiver,
                worker.clone(),
                window,
                cx,
                move |this, result, window, cx| {
                    let result = result.unwrap_or(Err(CapabilityFailure::MaintenanceUnavailable));
                    this.complete_capability(dependency, generation, result, window, cx);
                },
            ),
            Err(_) => self.complete_capability(
                dependency,
                generation,
                Err(CapabilityFailure::MaintenanceUnavailable),
                window,
                cx,
            ),
        }
        cx.notify();
    }

    fn complete_capability(
        &mut self,
        dependency: Dependency,
        generation: u64,
        result: Result<RecoveredCapability, CapabilityFailure>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.capability_vm.complete(dependency, generation) {
            return;
        }
        let failure = match result {
            Ok(RecoveredCapability::Optional(Ok(check))) => {
                self.install_checked_capability(dependency, check, window, cx);
                None
            }
            Ok(RecoveredCapability::Optional(Err(message))) => {
                self.capability_vm.check_message = Some(format!(
                    "[{}] {message}",
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
                ));
                Some(CapabilityFailure::Preparation)
            }
            Ok(RecoveredCapability::Runtime(host, snapshot)) => {
                if let Some(snapshot) = snapshot {
                    self.capability_vm.pending.checked(dependency, &snapshot);
                }
                self.install_runtime(host, cx);
                None
            }
            Ok(RecoveredCapability::Cache) => None,
            Err(failure) => {
                if failure == CapabilityFailure::MaintenanceUnavailable {
                    self.capability_vm.worker_available = false;
                }
                Some(failure)
            }
        };
        if failure != Some(CapabilityFailure::MaintenanceUnavailable) {
            self.capability_vm.record_completion(
                dependency,
                generation,
                std::time::SystemTime::now(),
            );
        }
        if dependency != Dependency::ThumbnailMaintenance
            || failure == Some(CapabilityFailure::MaintenanceUnavailable)
        {
            self.capability_observations
                .record(CapabilityObservation::new(dependency, failure));
        }
        self.capability_vm.observations = self.capability_observations.snapshot();
        let playback = self.playback_availability();
        self.library.update(cx, |library, cx| {
            library.playback_availability = playback;
            cx.notify();
        });
        self.reproject_show_page_from_current_queue();
        if let Some(next) = self.capability_vm.queued.pop_front() {
            self.check_capability(next, window, cx);
        }
        cx.notify();
    }

    pub(super) fn check_saved_capabilities(
        &mut self,
        dependencies: Vec<Dependency>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.capability_vm.pending.saved();
        self.capability_vm.queued.extend(dependencies);
        if !self.capability_vm.is_working() {
            if let Some(next) = self.capability_vm.queued.pop_front() {
                self.check_capability(next, window, cx);
            }
        }
    }

    fn install_checked_capability(
        &mut self,
        dependency: Dependency,
        check: CapabilityCheck,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let snapshot = check.snapshot;
        match check.prepared {
            PreparedCapability::Endpoint(endpoint) => {
                self.musicindex_endpoint = endpoint.clone();
                self.library.update(cx, |library, cx| {
                    library.set_musicindex_endpoint(endpoint, cx);
                });
            }
            PreparedCapability::Playback(player) => self.playback_owner = Some(player),
            PreparedCapability::Configuration => match dependency {
                Dependency::Publisher => {
                    self.broadcast.hosts.clone_from(&snapshot.broadcast_hosts);
                    self.broadcast
                        .selected_host
                        .clone_from(&snapshot.selected_host);
                    self.publisher_service_watch.take();
                    self.maybe_start_broadcast_service_watch(cx);
                }
                Dependency::Producer => {
                    self.broadcast
                        .drop_directory
                        .clone_from(&snapshot.drop_directory);
                    self.broadcast
                        .drop_file_target
                        .clone_from(&snapshot.drop_file_target);
                }
                Dependency::Encoder => {
                    self.broadcast.encoder.clone_from(&snapshot.encoder);
                    self.publisher_service_watch.take();
                    self.maybe_start_broadcast_service_watch(cx);
                }
                Dependency::Presentation => {
                    self.refresh_corrected_settings(&snapshot, window, cx);
                    self.apply_corrected_presentation(&snapshot, cx);
                }
                _ => {}
            },
        }
        self.capability_observations
            .checked_configuration(&snapshot, dependency);
        self.capability_vm.pending.checked(dependency, &snapshot);
        self.capability_vm.check_message = Some(format!(
            "[{}] {}: {} App refreshed that tool; no original action was retried.",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            crate::view_models::startup::capabilities::dependency_title(dependency),
            check.observation
        ));
        let playback = self.playback_availability();
        self.library.update(cx, |library, cx| {
            library.playback_availability = playback;
            cx.notify();
        });
    }

    pub(super) fn retain_failed_action(
        &mut self,
        action: RecoveryAction,
        dependency: Dependency,
        cx: &mut Context<Self>,
    ) {
        self.capability_vm.pending.retain(
            action,
            dependency,
            self.command_runner.session().generation(),
        );
        cx.notify();
    }

    pub(super) fn retry_command<C>(&self, command: C, intent: RecoveryIntent) -> RetryCommand<C> {
        RetryCommand {
            command,
            intent,
            path: self.cfg_path.clone(),
            conn: self.conn.clone(),
            session: self.command_runner.session().clone(),
        }
    }

    pub(super) fn command_with_retry<C>(
        &self,
        command: C,
        intent: Option<RecoveryIntent>,
    ) -> crate::application::capability_recovery::OriginalOrRetry<C> {
        use crate::application::capability_recovery::OriginalOrRetry;
        match intent {
            Some(intent) => OriginalOrRetry::Retry(Box::new(self.retry_command(command, intent))),
            None => OriginalOrRetry::Original(command),
        }
    }

    fn retry_retained_action(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let features = self.feature_availability();
        let intent = match self.capability_vm.pending.begin_retry(
            id,
            self.command_runner.session(),
            features,
        ) {
            Ok(intent) => intent,
            Err(error) => {
                self.capability_vm.pending.finish(id, error.to_string());
                return;
            }
        };
        match intent.action.clone() {
            RecoveryAction::IndexSearch { query } => {
                self.select_tab(AppTab::Music, window, cx);
                self.retry_index_search(&query, intent, cx);
            }
            RecoveryAction::Playback { .. } => self.retry_playback(intent, cx),
            RecoveryAction::Publisher { .. }
            | RecoveryAction::Encoder { .. }
            | RecoveryAction::Event { .. } => self.retry_show_action(intent, cx),
        }
    }

    fn install_runtime(&mut self, host: Arc<RuntimeHost>, cx: &mut Context<Self>) {
        if self.runtime_host.is_some() {
            return;
        }
        self.command_runner = AsyncCommandRunner::with_vm_bus_on_handle(
            self.application_services.command_bus(),
            self.application_services.event_bus(),
            host.bus().clone(),
            host.handle().clone(),
        );
        self.runtime_host = Some(host.clone());
        self.library
            .update(cx, |library, cx| library.install_runtime(host, cx));
        let playback = self.playback_availability();
        self.library.update(cx, |library, cx| {
            library.playback_availability = playback;
            cx.notify();
        });
        self.maybe_start_broadcast_readiness_watch(cx);
        self.maybe_start_broadcast_service_watch(cx);
        self.refresh_show_page(cx);
        self.reload_cached(cx);
    }
}
