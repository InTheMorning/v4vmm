//! Show screen adapter.
//!
//! ADR 0060 moves queue and transport presentation into a screen mount. This
//! adapter binds existing playback projection and transport callbacks to the
//! Show shell without introducing a workspace frame.

use gpui::Context;

use crate::ui::shells::show::{render_show, ShowShell, ShowSlots};
use crate::view_models::show::ShowPageVm;

use super::queue_now_playing::{queue_now_playing_vm, queue_transport_action};
use super::TopApp;

pub(super) fn build_show_screen(app: &TopApp, cx: &mut Context<TopApp>) -> ShowShell {
    let entity = cx.entity();
    render_show(
        ShowPageVm::from_queue(queue_now_playing_vm(app)),
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
