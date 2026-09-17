//! Top-level playback command binding.

use std::sync::Arc;

use gpui::Context;

use crate::application::capability::Dependency;
use crate::application::capability_recovery::{PlaybackOperation, RecoveryAction, RecoveryIntent};
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
        track_id: i64,
        cx: &mut Context<Self>,
    ) {
        let action = RecoveryAction::Playback {
            operation: PlaybackOperation::Playlist {
                playlist_id,
                position: playlist_position,
            },
            track_id: Some(track_id),
            queue: Vec::new(),
        };
        let Some(owner) = self.available_playback_owner(&action, cx) else {
            return;
        };
        let command = PlayPlaylistAt::new(
            Arc::clone(&self.conn),
            Arc::clone(&owner),
            playlist_id,
            playlist_position,
        );
        self.run_playback_command(command, action, None, cx);
    }

    pub(super) fn skip_playback_next(&mut self, cx: &mut Context<Self>) {
        let action = self.playback_action(PlaybackOperation::Next);
        let Some(owner) = self.available_playback_owner(&action, cx) else {
            return;
        };
        let command = SkipPlaybackNext::new(Arc::clone(&self.conn), Arc::clone(&owner));
        self.run_playback_command(command, action, None, cx);
    }

    pub(super) fn skip_playback_previous(&mut self, cx: &mut Context<Self>) {
        let action = self.playback_action(PlaybackOperation::Previous);
        let Some(owner) = self.available_playback_owner(&action, cx) else {
            return;
        };
        let command = SkipPlaybackPrevious::new(Arc::clone(&self.conn), Arc::clone(&owner));
        self.run_playback_command(command, action, None, cx);
    }

    pub(super) fn toggle_playback_paused(&mut self, cx: &mut Context<Self>) {
        let paused = self.show_page.queue.transport.play_pause_state == TransportState::Paused;
        let action = self.playback_action(if paused {
            PlaybackOperation::Resume
        } else {
            PlaybackOperation::Pause
        });
        let Some(owner) = self.available_playback_owner(&action, cx) else {
            return;
        };
        if paused {
            let command = ResumePlayback::new(Arc::clone(&self.conn), Arc::clone(&owner));
            self.run_playback_command(command, action, None, cx);
        } else {
            let command = PausePlayback::new(Arc::clone(&self.conn), Arc::clone(&owner));
            self.run_playback_command(command, action, None, cx);
        }
    }

    fn playback_action(&self, operation: PlaybackOperation) -> RecoveryAction {
        RecoveryAction::Playback {
            operation,
            track_id: self.show_page.queue.current_track_id(),
            queue: self.show_page.queue.track_ids(),
        }
    }

    pub(super) fn retry_playback(&mut self, intent: RecoveryIntent, cx: &mut Context<Self>) {
        let Some(owner) = self.playback_owner.clone() else {
            self.capability_vm.pending.finish(
                intent.id,
                "App has no verified player. Check playback again.".into(),
            );
            return;
        };
        let RecoveryAction::Playback { operation, .. } = intent.action else {
            return;
        };
        match operation {
            PlaybackOperation::Playlist {
                playlist_id,
                position,
            } => self.run_retained_playback(
                PlayPlaylistAt::new(self.conn.clone(), owner, playlist_id, position),
                intent,
                cx,
            ),
            PlaybackOperation::Pause => {
                self.run_retained_playback(
                    PausePlayback::new(self.conn.clone(), owner),
                    intent,
                    cx,
                );
            }
            PlaybackOperation::Resume => self.run_retained_playback(
                ResumePlayback::new(self.conn.clone(), owner),
                intent,
                cx,
            ),
            PlaybackOperation::Next => self.run_retained_playback(
                SkipPlaybackNext::new(self.conn.clone(), owner),
                intent,
                cx,
            ),
            PlaybackOperation::Previous => self.run_retained_playback(
                SkipPlaybackPrevious::new(self.conn.clone(), owner),
                intent,
                cx,
            ),
        }
    }

    fn run_retained_playback<C>(&self, command: C, intent: RecoveryIntent, cx: &mut Context<Self>)
    where
        C: ApplicationCommand<
            Output = crate::application::commands::playback::PlaybackCommandResult,
        >,
    {
        let id = intent.id;
        let action = intent.action.clone();
        self.run_playback_command(self.retry_command(command, intent), action, Some(id), cx);
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
        .with_checking(self.capability_vm.running_dependency())
    }

    fn available_playback_owner(
        &mut self,
        action: &RecoveryAction,
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
            self.retain_failed_action(action.clone(), reason.dependency, cx);
            self.settings_status = reason.to_string();
            self.reproject_show_page_from_current_queue();
            cx.notify();
            return None;
        }
        self.playback_owner.clone()
    }

    fn run_playback_command<C>(
        &self,
        command: C,
        action: RecoveryAction,
        retry_id: Option<u64>,
        cx: &mut Context<Self>,
    ) where
        C: ApplicationCommand<
            Output = crate::application::commands::playback::PlaybackCommandResult,
        >,
    {
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, result, cx| {
                if let Some(id) = retry_id {
                    this.capability_vm
                        .pending
                        .succeed(id, result.message().to_owned());
                }
                this.maybe_start_playback_polling(cx);
                this.settings_status = result.message().to_string();
                this.refresh_show_page(cx);
            },
            move |this, error, cx| {
                if let Some(id) = retry_id {
                    this.capability_vm.pending.finish(
                        id,
                        crate::diagnostics::redact_endpoint_details(&format!(
                            "App could not retry playback: {error}"
                        )),
                    );
                } else {
                    this.retain_failed_action(action, Dependency::Playback, cx);
                }
                this.settings_status = format!("Playback error: {error:#}");
            },
        );
    }
}
