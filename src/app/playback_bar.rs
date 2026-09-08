//! Top-level playback command binding.

use std::sync::Arc;

use gpui::Context;

use crate::application::commands::playback::{
    PausePlayback, PlayPlaylistAt, ResumePlayback, SkipPlaybackNext, SkipPlaybackPrevious,
};
use crate::application::{ApplicationCommand, CommandContext};
use crate::presentation::present_command;
use crate::view_models::queue_now_playing::TransportState;

use super::TopApp;

impl TopApp {
    pub(super) fn play_playlist_at(
        &mut self,
        playlist_id: i64,
        playlist_position: i64,
        cx: &mut Context<Self>,
    ) {
        let command = PlayPlaylistAt::new(
            Arc::clone(&self.conn),
            Arc::clone(&self.playback_owner),
            playlist_id,
            playlist_position,
        );
        self.run_playback_command(command, cx);
    }

    pub(super) fn skip_playback_next(&mut self, cx: &mut Context<Self>) {
        let command =
            SkipPlaybackNext::new(Arc::clone(&self.conn), Arc::clone(&self.playback_owner));
        self.run_playback_command(command, cx);
    }

    pub(super) fn skip_playback_previous(&mut self, cx: &mut Context<Self>) {
        let command =
            SkipPlaybackPrevious::new(Arc::clone(&self.conn), Arc::clone(&self.playback_owner));
        self.run_playback_command(command, cx);
    }

    pub(super) fn toggle_playback_paused(&mut self, cx: &mut Context<Self>) {
        if self.show_page.queue.transport.play_pause_state == TransportState::Paused {
            let command =
                ResumePlayback::new(Arc::clone(&self.conn), Arc::clone(&self.playback_owner));
            self.run_playback_command(command, cx);
        } else {
            let command =
                PausePlayback::new(Arc::clone(&self.conn), Arc::clone(&self.playback_owner));
            self.run_playback_command(command, cx);
        }
    }

    fn run_playback_command<C>(&self, command: C, cx: &mut Context<Self>)
    where
        C: ApplicationCommand<
            Output = crate::application::commands::playback::PlaybackCommandResult,
        >,
    {
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            |this, result, cx| {
                this.settings_status = result.message().to_string();
                this.refresh_show_page(cx);
            },
            |this, error, _cx| {
                this.settings_status = format!("Playback error: {error:#}");
            },
        );
    }
}
