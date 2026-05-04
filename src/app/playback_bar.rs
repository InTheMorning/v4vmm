//! Top-level playback bar command/query binding.

use std::sync::Arc;

use gpui::Context;

use crate::application::commands::playback::{
    PausePlayback, PlayPlaylistAt, ResumePlayback, SkipPlaybackNext, SkipPlaybackPrevious,
    StopPlayback,
};
use crate::application::{ApplicationCommand, CommandContext};
use crate::playback;
use crate::ui::composites::{NowPlayingBar, NowPlayingData, NowPlayingState};

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

    fn skip_playback_next(&mut self, cx: &mut Context<Self>) {
        let command =
            SkipPlaybackNext::new(Arc::clone(&self.conn), Arc::clone(&self.playback_owner));
        self.run_playback_command(command, cx);
    }

    fn skip_playback_previous(&mut self, cx: &mut Context<Self>) {
        let command =
            SkipPlaybackPrevious::new(Arc::clone(&self.conn), Arc::clone(&self.playback_owner));
        self.run_playback_command(command, cx);
    }

    fn toggle_playback_paused(&mut self, cx: &mut Context<Self>) {
        if self.playback_bar_state().is_paused() {
            let command =
                ResumePlayback::new(Arc::clone(&self.conn), Arc::clone(&self.playback_owner));
            self.run_playback_command(command, cx);
        } else {
            let command =
                PausePlayback::new(Arc::clone(&self.conn), Arc::clone(&self.playback_owner));
            self.run_playback_command(command, cx);
        }
    }

    fn stop_playback(&mut self, cx: &mut Context<Self>) {
        let command = StopPlayback::new(Arc::clone(&self.conn), Arc::clone(&self.playback_owner));
        self.run_playback_command(command, cx);
    }

    fn playback_bar_state(&self) -> crate::application::queries::playback::PlaybackSnapshot {
        let conn = self.conn.lock().expect("lock db");
        self.application_services
            .query_service()
            .playback_snapshot(&conn, playback::DEFAULT_SESSION_ID)
            .unwrap_or_default()
    }

    fn run_playback_command<C>(&self, command: C, cx: &mut Context<Self>)
    where
        C: ApplicationCommand<
            Output = crate::application::commands::playback::PlaybackCommandResult,
        >,
    {
        self.command_runner.run(
            command,
            CommandContext::next(),
            cx,
            |this, result, _cx| {
                this.settings_status = result.message().to_string();
            },
            |this, error, _cx| {
                this.settings_status = format!("Playback error: {error:#}");
            },
        );
    }
}

pub(super) fn build_playback_bar(app: &TopApp, cx: &mut Context<TopApp>) -> NowPlayingBar {
    let state = app.playback_bar_state();
    let np_state = if !state.is_active() {
        None
    } else if state.is_paused() {
        Some(NowPlayingState::Paused)
    } else {
        Some(NowPlayingState::Playing)
    };
    NowPlayingBar::new()
        .data(NowPlayingData {
            title: state.title().map(str::to_string),
            artist: None,
            state: np_state,
            thumbnail: None,
            play_pause_a11y_label: if !state.is_active() {
                "Play"
            } else if state.is_paused() {
                "Resume playback"
            } else {
                "Pause playback"
            },
            ..NowPlayingData::default()
        })
        .on_prev(cx.listener(|this, _, _, cx| this.skip_playback_previous(cx)))
        .on_play_pause(cx.listener(|this, _, _, cx| this.toggle_playback_paused(cx)))
        .on_next(cx.listener(|this, _, _, cx| this.skip_playback_next(cx)))
        .on_stop(cx.listener(|this, _, _, cx| this.stop_playback(cx)))
}
