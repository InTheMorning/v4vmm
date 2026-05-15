//! Top-level playback command/query binding plus the compact
//! now-playing status UI.

use std::sync::Arc;

use gpui::{
    div, prelude::*, App, Context, Image, IntoElement, RenderOnce, SharedString, Styled, Window,
};

use crate::application::commands::playback::{
    PausePlayback, PlayPlaylistAt, ResumePlayback, SkipPlaybackNext, SkipPlaybackPrevious,
};
use crate::application::{ApplicationCommand, CommandContext};
use crate::playback;
use crate::ui::composites::{EntityKind, Thumbnail, ThumbnailSize};
use crate::ui::primitives::Label;
use crate::ui::tokens::{color, FontSize, SemanticColor, Spacing};

use super::TopApp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Debug, Clone, Default)]
pub struct NowPlayingData {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub state: Option<PlaybackState>,
    pub thumbnail: Option<Arc<Image>>,
}

#[derive(IntoElement)]
#[must_use]
pub struct NowPlayingBar {
    data: NowPlayingData,
}

impl NowPlayingBar {
    pub fn new() -> Self {
        Self {
            data: NowPlayingData::default(),
        }
    }

    pub fn data(mut self, data: NowPlayingData) -> Self {
        self.data = data;
        self
    }
}

impl Default for NowPlayingBar {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for NowPlayingBar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let gap = Spacing::MD.scaled(cx);
        let tertiary_label = color(cx, SemanticColor::TertiaryLabel);
        let secondary_label = color(cx, SemanticColor::SecondaryLabel);

        let title = self.data.title.clone();
        let artist = self.data.artist.clone();
        let state = self.data.state;
        let thumbnail = self.data.thumbnail.clone();

        let is_active = state.is_some_and(|s| s != PlaybackState::Stopped);

        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(gap)
            .px(Spacing::LG.scaled(cx))
            .child(Thumbnail::new(EntityKind::Track, ThumbnailSize::Sm).image(thumbnail))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .when(is_active, |el| {
                        el.child(
                            Label::new(title.clone().unwrap_or_else(|| "Nothing playing".into()))
                                .size(FontSize::Body)
                                .truncated(),
                        )
                    })
                    .when(!is_active, |el| {
                        el.child(
                            div()
                                .text_color(tertiary_label)
                                .text_size(FontSize::Body.scaled(cx))
                                .child(SharedString::from("Nothing playing")),
                        )
                    })
                    .when_some(artist.clone(), |el, art| {
                        el.child(
                            div()
                                .text_color(secondary_label)
                                .text_size(FontSize::Caption.scaled(cx))
                                .child(SharedString::from(art)),
                        )
                    }),
            )
    }
}

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

pub(super) fn build_playback_bar(app: &TopApp) -> NowPlayingBar {
    let state = app.playback_bar_state();
    let np_state = if !state.is_active() {
        None
    } else if state.is_paused() {
        Some(PlaybackState::Paused)
    } else {
        Some(PlaybackState::Playing)
    };
    NowPlayingBar::new().data(NowPlayingData {
        title: state.title().map(str::to_string),
        artist: None,
        state: np_state,
        thumbnail: None,
    })
}
