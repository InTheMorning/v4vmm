//! Shell wiring for scoped background issues and explicit retries (ADR 0066).

use std::rc::Rc;
use std::sync::Arc;

use gpui::{ClipboardItem, Context, Window};

use crate::application::capability::{
    CapabilityAction, CapabilityFailure, CapabilityObservation, CapabilityObservations, Dependency,
};
use crate::application::AsyncCommandRunner;
use crate::presentation::maintenance_executor::MaintenanceClient;
use crate::presentation::startup_presenter::present_startup;
use crate::presentation::{bridge_watch, RuntimeHost};
use crate::ui::composites::startup_report::capability_report;
use crate::view_models::startup::capabilities::CapabilityReportVm;
use crate::view_models::startup::StartupAvailability;

use super::{bootstrap, AppTab, TopApp};

enum RecoveredCapability {
    Runtime(Arc<RuntimeHost>),
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
            Rc::new(move |action, window, cx| {
                let _ = entity.update(cx, |this, cx| this.capability_action(action, window, cx));
            }),
            cx,
        )
    }

    fn capability_action(
        &mut self,
        action: CapabilityAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.capability_vm.action(action).availability != StartupAvailability::Available {
            return;
        }
        match action {
            CapabilityAction::Configure(_) => self.select_tab(AppTab::Settings, window, cx),
            CapabilityAction::CopyReport => {
                cx.write_to_clipboard(ClipboardItem::new_string(self.capability_vm.report()));
            }
            CapabilityAction::CheckAgain(dependency) => {
                self.check_capability(dependency, window, cx);
            }
        }
    }

    fn check_capability(
        &mut self,
        dependency: Dependency,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(generation) = self.capability_vm.begin(dependency) else {
            return;
        };
        let Some(worker) = self.maintenance_worker.clone() else {
            return;
        };
        let cfg_path = self.cfg_path.clone();
        let cache = self.image_cache.clone();
        let receiver = worker.submit(move || match dependency {
            Dependency::BackgroundRuntime => RuntimeHost::for_config(&cfg_path)
                .map(RecoveredCapability::Runtime)
                .map_err(|error| CapabilityFailure::RuntimeStart(error.kind())),
            Dependency::ThumbnailMaintenance => cache
                .check_maintenance(bootstrap::cache_worker_for_config(&cfg_path))
                .map(|()| RecoveredCapability::Cache),
        });
        match receiver {
            Ok(receiver) => present_startup(receiver, window, cx, move |this, result, _, cx| {
                let result = result.unwrap_or(Err(CapabilityFailure::MaintenanceUnavailable));
                this.complete_capability(dependency, generation, result, cx);
            }),
            Err(_) => self.complete_capability(
                dependency,
                generation,
                Err(CapabilityFailure::MaintenanceUnavailable),
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
        cx: &mut Context<Self>,
    ) {
        if !self.capability_vm.complete(dependency, generation) {
            return;
        }
        let failure = match result {
            Ok(RecoveredCapability::Runtime(host)) => {
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
        if dependency == Dependency::BackgroundRuntime
            || failure == Some(CapabilityFailure::MaintenanceUnavailable)
        {
            self.capability_observations
                .record(CapabilityObservation::new(dependency, failure));
        }
        self.capability_vm.observations = self.capability_observations.snapshot();
        cx.notify();
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
        self.maybe_start_playback_polling(cx);
        self.maybe_start_broadcast_readiness_watch(cx);
        self.maybe_start_broadcast_service_watch(cx);
        self.refresh_show_page(cx);
        self.reload_cached(cx);
    }
}
