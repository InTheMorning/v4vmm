//! Independent-worker teardown of runtime and configured resources (ADR 0066).

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::application::session_lifecycle::{
    MaintenanceSession, SessionDrain, SESSION_DRAIN_TIMEOUT,
};
use crate::presentation::RuntimeHost;

pub(crate) struct SessionTransition {
    pub(crate) drain: SessionDrain,
    pub(crate) runtime: Option<Arc<RuntimeHost>>,
    #[cfg(debug_assertions)]
    pub(crate) config_path: std::path::PathBuf,
}

impl SessionTransition {
    pub(crate) fn wait_for_work(&self) -> Result<(), Vec<String>> {
        self.drain.session.wait_for_work(SESSION_DRAIN_TIMEOUT)
    }

    /// Called after unmount; actual ownership transfers and SQLite close authorize maintenance.
    pub(crate) fn close(&mut self) -> Result<MaintenanceSession, Vec<String>> {
        let deadline = Instant::now() + SESSION_DRAIN_TIMEOUT;
        loop {
            let result = self.close_once();
            if result.is_ok() || Instant::now() >= deadline {
                return result;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn close_once(&mut self) -> Result<MaintenanceSession, Vec<String>> {
        self.drain.session.wait_for_work(Duration::ZERO)?;
        if let Some(runtime) = self.runtime.take() {
            match Arc::try_unwrap(runtime) {
                Ok(runtime) => drop(runtime),
                Err(runtime) => {
                    self.runtime = Some(runtime);
                    return Err(vec![
                        "Background runtime still has an outstanding owner".into()
                    ]);
                }
            }
        }
        let maintenance = self.drain.finish()?;
        #[cfg(debug_assertions)]
        crate::startup::fixture::record_session_release(
            &self.config_path,
            maintenance.generation(),
        );
        Ok(maintenance)
    }
}
