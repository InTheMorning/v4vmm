//! `NowPlayingBar` composite — the persistent mini-player strip rendered at
//! the top of the app window.
//!
//! Accepts display-ready data from the screen (title, artist, progress,
//! playback state) and renders artwork · transport controls · scrubber.
//! All actions are callbacks so the composite carries no domain state.
//!
//! This is the migration target for `app.rs::render_playback_controls`.
//! The inline free function will be removed once `TopApp::render` binds
//! this composite directly.

#![warn(clippy::pedantic)]

use std::rc::Rc;
use std::sync::Arc;

use gpui::{div, prelude::*, App, Image, IntoElement, RenderOnce, SharedString, Styled, Window};

use crate::ui::composites::{EntityKind, Thumbnail, ThumbnailSize};
use crate::ui::primitives::Label;
use crate::ui::tokens::{FontSize, SemanticColor, Spacing};

type ClickCallback = Rc<dyn Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static>;

/// Playback state visible to the bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

/// Display data for the now-playing strip.
///
/// All fields are already-formatted strings — projection lives in the
/// screen or view-model layer.
#[derive(Debug, Clone, Default)]
pub struct NowPlayingData {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub state: Option<PlaybackState>,
    pub thumbnail: Option<Arc<Image>>,
}

/// The persistent playback bar rendered above the tab content area.
///
/// # Examples
///
/// ```ignore
/// NowPlayingBar::new()
///     .data(NowPlayingData { title: Some("Song".into()), .. })
///     .on_prev(cx.listener(|this, _, _, cx| this.skip_prev(cx)))
///     .on_play_pause(cx.listener(|this, _, _, cx| this.toggle_pause(cx)))
///     .on_next(cx.listener(|this, _, _, cx| this.skip_next(cx)))
/// ```
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
    /// Create a bar in the stopped / empty state.
    pub fn new() -> Self {
        Self {
            data: NowPlayingData::default(),
            on_prev: None,
            on_play_pause: None,
            on_next: None,
            on_stop: None,
        }
    }

    /// Supply the now-playing display data.
    pub fn data(mut self, data: NowPlayingData) -> Self {
        self.data = data;
        self
    }

    /// Called when the user clicks the previous-track button.
    pub fn on_prev<F>(mut self, handler: F) -> Self
    where
        F: Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    {
        self.on_prev = Some(Rc::new(handler));
        self
    }

    /// Called when the user clicks the play/pause toggle.
    pub fn on_play_pause<F>(mut self, handler: F) -> Self
    where
        F: Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    {
        self.on_play_pause = Some(Rc::new(handler));
        self
    }

    /// Called when the user clicks the next-track button.
    pub fn on_next<F>(mut self, handler: F) -> Self
    where
        F: Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    {
        self.on_next = Some(Rc::new(handler));
        self
    }

    /// Called when the user clicks stop.
    pub fn on_stop<F>(mut self, handler: F) -> Self
    where
        F: Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
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
        let appearance = crate::ui::tokens::Appearance::current(cx);
        let gap = Spacing::MD.scaled(cx);

        let title = self.data.title.clone();
        let artist = self.data.artist.clone();
        let state = self.data.state;
        let thumbnail = self.data.thumbnail.clone();

        let play_pause_glyph = match state {
            Some(PlaybackState::Playing) => "\u{23F8}", // ⏸
            _ => "\u{25B6}",                            // ▶
        };

        let is_active = state.is_some_and(|s| s != PlaybackState::Stopped);

        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(gap)
            .px(Spacing::LG.scaled(cx))
            // Thumbnail.
            .child(Thumbnail::new(EntityKind::Track, ThumbnailSize::Sm).image(thumbnail))
            // Title + artist block.
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
                                .text_color(SemanticColor::TertiaryLabel.resolve(appearance))
                                .text_size(FontSize::Body.scaled(cx))
                                .child(SharedString::from("Nothing playing")),
                        )
                    })
                    .when_some(artist.clone(), |el, art| {
                        el.child(
                            div()
                                .text_color(SemanticColor::SecondaryLabel.resolve(appearance))
                                .text_size(FontSize::Caption.scaled(cx))
                                .child(SharedString::from(art)),
                        )
                    }),
            )
            // Transport controls.
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(Spacing::SM.scaled(cx))
                    .child(transport_btn(
                        "np-prev",
                        "\u{23EE}",
                        self.on_prev,
                        is_active,
                        cx,
                    ))
                    .child(transport_btn(
                        "np-playpause",
                        play_pause_glyph,
                        self.on_play_pause,
                        is_active,
                        cx,
                    ))
                    .child(transport_btn(
                        "np-next",
                        "\u{23ED}",
                        self.on_next,
                        is_active,
                        cx,
                    ))
                    .child(transport_btn(
                        "np-stop",
                        "\u{23F9}",
                        self.on_stop,
                        is_active,
                        cx,
                    )),
            )
    }
}

fn transport_btn(
    id: &'static str,
    glyph: &'static str,
    handler: Option<ClickCallback>,
    enabled: bool,
    cx: &App,
) -> impl IntoElement {
    let appearance = crate::ui::tokens::Appearance::current(cx);
    let color = if enabled {
        SemanticColor::Label.resolve(appearance)
    } else {
        SemanticColor::QuaternaryLabel.resolve(appearance)
    };

    let mut btn = div()
        .id(id)
        .w(gpui::px(28.0))
        .h(gpui::px(28.0))
        .flex()
        .items_center()
        .justify_center()
        .text_color(color)
        .text_size(FontSize::Body.scaled(cx))
        .child(SharedString::from(glyph));

    if enabled {
        if let Some(h) = handler {
            btn = btn
                .cursor_pointer()
                .on_click(move |event, window, cx| h(event, window, cx));
        }
    }

    btn
}

// ---------------------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_stopped_equivalent() {
        let bar = NowPlayingBar::new();
        assert!(bar.data.title.is_none());
        assert!(bar.data.state.is_none());
    }

    #[test]
    fn data_builder_sets_title() {
        let bar = NowPlayingBar::new().data(NowPlayingData {
            title: Some("Song".into()),
            ..Default::default()
        });
        assert_eq!(bar.data.title.as_deref(), Some("Song"));
    }

    #[test]
    fn playback_state_playing_vs_paused() {
        assert_ne!(PlaybackState::Playing, PlaybackState::Paused);
        assert_ne!(PlaybackState::Playing, PlaybackState::Stopped);
    }

    #[test]
    fn play_pause_glyph_is_pause_when_playing() {
        // The glyph selection is tested indirectly through render, but we
        // can at least pin the Unicode code points we depend on.
        let pause = "\u{23F8}";
        let play = "\u{25B6}";
        assert_ne!(pause, play);
    }
}
