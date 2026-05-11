//! Top-level playback bar command/query binding plus the
//! `NowPlayingBar` UI (inlined per ADR 0042 — single-call composite).

use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    div, prelude::*, App, ClickEvent, Context, Image, IntoElement, RenderOnce, SharedString,
    Styled, Window,
};

use crate::application::commands::playback::{
    PausePlayback, PlayPlaylistAt, ResumePlayback, SkipPlaybackNext, SkipPlaybackPrevious,
    StopPlayback,
};
use crate::application::{ApplicationCommand, CommandContext};
use crate::playback;
use crate::ui::composites::{EntityKind, Thumbnail, ThumbnailSize};
use crate::ui::control_styles::ControlStyle;
use crate::ui::icons::IconName;
use crate::ui::primitives::{Button, Label};
use crate::ui::tokens::{color, FontSize, SemanticColor, Spacing};

use super::TopApp;

type ClickCallback = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Debug, Clone)]
pub struct NowPlayingData {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub state: Option<PlaybackState>,
    pub thumbnail: Option<Arc<Image>>,
    pub previous_a11y_label: &'static str,
    pub play_pause_a11y_label: &'static str,
    pub next_a11y_label: &'static str,
    pub stop_a11y_label: &'static str,
}

impl Default for NowPlayingData {
    fn default() -> Self {
        Self {
            title: None,
            artist: None,
            state: None,
            thumbnail: None,
            previous_a11y_label: "Previous track",
            play_pause_a11y_label: "Play",
            next_a11y_label: "Next track",
            stop_a11y_label: "Stop playback",
        }
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct NowPlayingBar {
    data: NowPlayingData,
    on_prev: Option<ClickCallback>,
    on_play_pause: Option<ClickCallback>,
    on_next: Option<ClickCallback>,
    on_stop: Option<ClickCallback>,
}

impl NowPlayingBar {
    pub fn new() -> Self {
        Self {
            data: NowPlayingData::default(),
            on_prev: None,
            on_play_pause: None,
            on_next: None,
            on_stop: None,
        }
    }

    pub fn data(mut self, data: NowPlayingData) -> Self {
        self.data = data;
        self
    }

    pub fn on_prev<F>(mut self, handler: F) -> Self
    where
        F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    {
        self.on_prev = Some(Rc::new(handler));
        self
    }

    pub fn on_play_pause<F>(mut self, handler: F) -> Self
    where
        F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    {
        self.on_play_pause = Some(Rc::new(handler));
        self
    }

    pub fn on_next<F>(mut self, handler: F) -> Self
    where
        F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    {
        self.on_next = Some(Rc::new(handler));
        self
    }

    pub fn on_stop<F>(mut self, handler: F) -> Self
    where
        F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    {
        self.on_stop = Some(Rc::new(handler));
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

        let play_pause_icon = match state {
            Some(PlaybackState::Playing) => IconName::Pause,
            _ => IconName::Play,
        };

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
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(Spacing::SM.scaled(cx))
                    .child(transport_btn(
                        "np-prev",
                        IconName::Previous,
                        self.data.previous_a11y_label,
                        self.on_prev,
                        is_active,
                    ))
                    .child(transport_btn(
                        "np-playpause",
                        play_pause_icon,
                        self.data.play_pause_a11y_label,
                        self.on_play_pause,
                        is_active,
                    ))
                    .child(transport_btn(
                        "np-next",
                        IconName::Next,
                        self.data.next_a11y_label,
                        self.on_next,
                        is_active,
                    ))
                    .child(transport_btn(
                        "np-stop",
                        IconName::Stop,
                        self.data.stop_a11y_label,
                        self.on_stop,
                        is_active,
                    )),
            )
    }
}

fn transport_btn(
    id: &'static str,
    icon: IconName,
    a11y_label: &'static str,
    handler: Option<ClickCallback>,
    enabled: bool,
) -> impl IntoElement {
    let mut button = Button::styled(id, ControlStyle::ToolbarIcon)
        .leading_icon(icon)
        .a11y_label(a11y_label)
        .disabled(!enabled);

    if enabled {
        if let Some(h) = handler {
            button = button.on_click(move |event, window, cx| h(event, window, cx));
        }
    }

    button
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
        Some(PlaybackState::Paused)
    } else {
        Some(PlaybackState::Playing)
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
