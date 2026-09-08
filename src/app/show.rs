//! Show screen adapter.
//!
//! ADR 0060 moves queue and transport presentation into a screen mount. This
//! adapter binds existing playback projection and transport callbacks to the
//! Show shell without introducing a workspace frame.

use std::sync::{Arc, Mutex};

use gpui::Context;
use rusqlite::Connection;

use crate::application::{
    ApplicationCommand, ApplicationServices, CommandContext, CommandError, CommandOutcome,
};
use crate::presentation::present_command;
use crate::ui::composites::{live_status_strip, LiveStatusStrip, LiveStatusStripSlots};
use crate::ui::shells::show::{render_show, ShowShell, ShowSlots};
use crate::view_models::live_status::LiveStatusDisplay;
use crate::view_models::show::ShowPageVm;

use super::queue_now_playing::{queue_now_playing_vm, queue_transport_action};
use super::{AppTab, TopApp, WorkspaceScreenMount};

pub(super) fn build_show_screen(app: &TopApp, cx: &mut Context<TopApp>) -> ShowShell {
    let entity = cx.entity();
    render_show(
        app.show_page.clone(),
        ShowSlots::new()
            .on_skip_previous(queue_transport_action(
                entity.clone(),
                TopApp::skip_playback_previous,
            ))
            .on_play_pause(queue_transport_action(
                entity.clone(),
                TopApp::toggle_playback_paused,
            ))
            .on_skip_next(queue_transport_action(entity, TopApp::skip_playback_next)),
    )
}

pub(super) fn build_live_status_strip(
    app: &TopApp,
    mount: WorkspaceScreenMount,
    cx: &mut Context<TopApp>,
) -> Option<LiveStatusStrip> {
    if !matches!(
        mount,
        WorkspaceScreenMount::Music | WorkspaceScreenMount::Settings
    ) {
        return None;
    }
    let display = LiveStatusDisplay::from_show_page(&app.show_page);
    if !display.active {
        return None;
    }

    let entity = cx.entity();
    Some(live_status_strip(
        display,
        LiveStatusStripSlots::new().on_open_show(move |_, _, cx| {
            entity.update(cx, |this, cx| {
                this.select_tab(AppTab::Show, cx);
            });
        }),
    ))
}

impl TopApp {
    pub(super) fn refresh_show_page(&self, cx: &mut Context<Self>) {
        let command = RefreshShowPage::new(
            Arc::clone(&self.conn),
            Arc::clone(&self.application_services),
            self.queue_text_filter.clone(),
        );
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            |this, show_page, _cx| {
                this.show_page = show_page;
            },
            |this, error, _cx| {
                this.settings_status = format!("Show status error: {error:#}");
            },
        );
    }
}

struct RefreshShowPage {
    conn: Arc<Mutex<Connection>>,
    application_services: Arc<ApplicationServices>,
    queue_text_filter: Option<String>,
}

impl RefreshShowPage {
    fn new(
        conn: Arc<Mutex<Connection>>,
        application_services: Arc<ApplicationServices>,
        queue_text_filter: Option<String>,
    ) -> Self {
        Self {
            conn,
            application_services,
            queue_text_filter,
        }
    }
}

impl ApplicationCommand for RefreshShowPage {
    type Output = ShowPageVm;

    fn execute(self, context: &CommandContext) -> crate::application::CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }

        let conn = self
            .conn
            .lock()
            .map_err(|_| CommandError::Query("database lock poisoned".to_string()))?;
        Ok(CommandOutcome::without_events(ShowPageVm::from_queue(
            queue_now_playing_vm(&self.application_services, &conn, self.queue_text_filter),
        )))
    }
}
