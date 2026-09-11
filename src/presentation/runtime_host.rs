//! Runtime host: owns the tokio runtime + cross-actor [`VmBus`] for the
//! lifetime of a desktop GPUI session (ADRs 0040 and 0066).
//!
//! ### Why a host?
//!
//! GPUI's foreground executor is single-threaded and intentionally doesn't
//! drive arbitrary async work — that's tokio's job. The host holds:
//!
//! * a multi-thread tokio [`Runtime`], so [`crate::runtime::actor::spawn`]
//!   can be called from any thread (including from a GPUI listener), and
//! * a single [`VmBus`] every actor subscribes to for invalidation events.
//!
//! Each normal-session preparation constructs a host and shares it across
//! screens via [`Arc<RuntimeHost>`]. Managed maintenance waits for actor
//! completion, then releases the runtime on the independent worker before
//! another session can open.
//!
//! ### Layer rules
//!
//! Lives under `src/presentation/` because it bridges `gpui` callers to
//! `runtime` actors. Screens hold an
//! `Arc<RuntimeHost>` but never touch [`tokio::runtime::Runtime`] directly.

#![warn(clippy::pedantic)]

use std::sync::Arc;

use tokio::runtime::{Builder, Runtime};

use crate::application::session_lifecycle::{SessionLifecycle, SessionWork};
use crate::runtime::VmBus;

/// Owned tokio runtime + cross-actor invalidation bus.
///
/// Cheap to share via [`Arc`]. The runtime is shut down on drop; in-flight
/// actor tasks observe their handles closing and exit gracefully.
pub struct RuntimeHost {
    runtime: Runtime,
    bus: VmBus,
    _session_owner: SessionWork,
}

impl RuntimeHost {
    /// Desktop creation seam; fixture failures exist only in debug builds.
    pub(crate) fn for_config(
        path: &std::path::Path,
        session: SessionLifecycle,
    ) -> std::io::Result<Arc<Self>> {
        #[cfg(debug_assertions)]
        if crate::startup::fixture::injected_failure(
            path,
            crate::application::capability::Dependency::BackgroundRuntime,
        ) {
            return Err(std::io::Error::other("fixture runtime factory unavailable"));
        }
        #[cfg(not(debug_assertions))]
        let _ = path;
        Self::new_in_session(session)
    }

    /// Build a host with a multi-thread tokio runtime.
    ///
    /// # Errors
    ///
    /// Returns the underlying [`tokio::io::Error`] if the runtime cannot
    /// be created (e.g. exhausted file descriptors).
    pub fn new() -> std::io::Result<Arc<Self>> {
        Self::new_in_session(SessionLifecycle::new())
    }

    fn new_in_session(session: SessionLifecycle) -> std::io::Result<Arc<Self>> {
        Self::build_in_session(session, || {
            Builder::new_multi_thread()
                .enable_all()
                .thread_name("v4vmm-runtime")
                .build()
        })
    }

    #[cfg(test)]
    fn new_with(build: impl FnOnce() -> std::io::Result<Runtime>) -> std::io::Result<Arc<Self>> {
        Self::build_in_session(SessionLifecycle::new(), build)
    }

    fn build_in_session(
        session: SessionLifecycle,
        build: impl FnOnce() -> std::io::Result<Runtime>,
    ) -> std::io::Result<Arc<Self>> {
        let owner = session
            .own("Background runtime")
            .ok_or_else(|| std::io::Error::other("app session is ending"))?;
        let runtime = build()?;
        Ok(Arc::new(Self {
            runtime,
            bus: VmBus::for_session(session),
            _session_owner: owner,
        }))
    }

    /// Borrow the cross-actor invalidation bus.
    pub fn bus(&self) -> &VmBus {
        &self.bus
    }

    /// Borrow the tokio handle so callers can `enter()` to spawn actors
    /// from a non-tokio thread (e.g. from inside a GPUI listener).
    pub fn handle(&self) -> &tokio::runtime::Handle {
        self.runtime.handle()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::VmEvent;

    #[test]
    fn adr_0066_failed_runtime_factory_has_no_fallback() {
        assert!(tokio::runtime::Handle::try_current().is_err());
        let result = RuntimeHost::new_with(|| Err(std::io::Error::other("injected failure")));
        assert_eq!(result.err().unwrap().kind(), std::io::ErrorKind::Other);
        assert!(tokio::runtime::Handle::try_current().is_err());
    }

    #[test]
    fn adr_0066_independent_worker_restores_an_explicit_usable_runner() {
        use crate::application::{
            ApplicationCommand, ApplicationEventBus, AsyncCommandRunner, CommandBus,
            CommandContext, CommandOutcome, CommandResult,
        };
        use crate::presentation::maintenance_executor::MaintenanceWorker;
        struct Probe;
        impl ApplicationCommand for Probe {
            type Output = bool;
            fn execute(self, _: &CommandContext) -> CommandResult<bool> {
                Ok(CommandOutcome::without_events(true))
            }
        }
        assert!(tokio::runtime::Handle::try_current().is_err());
        let worker = MaintenanceWorker::start().unwrap();
        let host = worker
            .client
            .submit(RuntimeHost::new)
            .unwrap()
            .blocking_recv()
            .unwrap()
            .unwrap();
        let runner = AsyncCommandRunner::with_vm_bus_on_handle(
            Arc::new(CommandBus::new()),
            Arc::new(ApplicationEventBus::new()),
            host.bus().clone(),
            host.handle().clone(),
        );
        assert!(runner.availability().is_ok());
        assert!(*runner
            .dispatch(Probe, CommandContext::next())
            .blocking_recv()
            .unwrap()
            .unwrap()
            .value());
        worker.finish();
        assert!(tokio::runtime::Handle::try_current().is_err());
    }

    #[test]
    fn new_succeeds_and_returns_a_usable_handle() {
        let host = RuntimeHost::new().expect("runtime host");
        // Spawning a trivial future on the runtime must work.
        let handle = host.handle().clone();
        let result = handle.block_on(async { 21 + 21 });
        assert_eq!(result, 42);
    }

    #[test]
    fn bus_is_shared_across_clones() {
        let host = RuntimeHost::new().expect("runtime host");
        let bus_a = host.bus().clone();
        let mut rx = bus_a.subscribe();
        host.bus().publish(VmEvent::TrackChanged { track_id: 7 });
        let event = host
            .handle()
            .block_on(async { rx.recv().await })
            .expect("recv");
        assert!(matches!(event, VmEvent::TrackChanged { track_id: 7 }));
    }

    #[test]
    fn host_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<RuntimeHost>();
        assert_send_sync::<Arc<RuntimeHost>>();
    }
}
