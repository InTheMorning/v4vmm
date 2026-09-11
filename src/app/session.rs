//! Thin normal-session resource handoff to the independent worker (ADR 0066).

use gpui::Context;

use crate::application::session_lifecycle::SessionDrain;
use crate::presentation::session_transition::SessionTransition;

use super::TopApp;

impl TopApp {
    pub(super) fn begin_session_drain(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Option<SessionTransition> {
        if self.maintenance_worker.is_none() || self.capability_vm.is_working() {
            return None;
        }
        let session = self.command_runner.session().clone();
        if !session.begin_drain() {
            return None;
        }
        self.playback_polling.take();
        self.broadcast_readiness_watch.take();
        self.publisher_service_watch.take();
        self.library
            .update(cx, |library, _| library.release_session_actors());
        Some(SessionTransition {
            drain: SessionDrain::new(session, self.conn.clone(), self.playback_owner.take()),
            runtime: self.runtime_host.take(),
            #[cfg(debug_assertions)]
            config_path: self.cfg_path.clone(),
        })
    }
}
