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
        let Some(owner) = self.available_playback_owner(cx) else {
            return;
        };
        let command = PlayPlaylistAt::new(
            Arc::clone(&self.conn),
            Arc::clone(&owner),
            playlist_id,
            playlist_position,
        );
        self.run_playback_command(command, cx);
    }

    pub(super) fn skip_playback_next(&mut self, cx: &mut Context<Self>) {
        let Some(owner) = self.available_playback_owner(cx) else {
            return;
        };
        let command = SkipPlaybackNext::new(Arc::clone(&self.conn), Arc::clone(&owner));
        self.run_playback_command(command, cx);
    }

    pub(super) fn skip_playback_previous(&mut self, cx: &mut Context<Self>) {
        let Some(owner) = self.available_playback_owner(cx) else {
            return;
        };
        let command = SkipPlaybackPrevious::new(Arc::clone(&self.conn), Arc::clone(&owner));
        self.run_playback_command(command, cx);
    }

    pub(super) fn toggle_playback_paused(&mut self, cx: &mut Context<Self>) {
        let Some(owner) = self.available_playback_owner(cx) else {
            return;
        };
        if self.show_page.queue.transport.play_pause_state == TransportState::Paused {
            let command = ResumePlayback::new(Arc::clone(&self.conn), Arc::clone(&owner));
            self.run_playback_command(command, cx);
        } else {
            let command = PausePlayback::new(Arc::clone(&self.conn), Arc::clone(&owner));
            self.run_playback_command(command, cx);
        }
    }

    pub(super) fn playback_availability(
        &self,
    ) -> Result<(), crate::application::capability::ExecutionUnavailable> {
        self.feature_availability()
            .require(crate::application::capability::Dependency::Playback)
    }

    pub(super) fn feature_availability(
        &self,
    ) -> crate::application::capability::FeatureAvailability {
        crate::application::capability::FeatureAvailability::from_resources(
            &self.musicindex_endpoint,
            &self.broadcast,
            self.playback_owner.is_some(),
        )
        .with_runtime(self.command_runner.availability())
        .with_observations(&self.capability_observations.snapshot())
    }

    fn available_playback_owner(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Option<
        Arc<
            std::sync::Mutex<
                crate::playback_owner::PlaybackOwner<
                    crate::playback_driver::ConfiguredPlaybackDriver,
                >,
            >,
        >,
    > {
        if let Err(reason) = self.playback_availability() {
            self.settings_status = reason.to_string();
            self.reproject_show_page_from_current_queue();
            cx.notify();
            return None;
        }
        self.playback_owner.clone()
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
                this.maybe_start_playback_polling(cx);
                this.settings_status = result.message().to_string();
                this.refresh_show_page(cx);
            },
            |this, error, _cx| {
                this.settings_status = format!("Playback error: {error:#}");
            },
        );
    }
}
