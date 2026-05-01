//! Entity-type tag badge — small uppercase pill identifying what kind of
//! record (artist, feed, track, …) a card represents.
//!
//! HIG note: at body-text sizes badges sit at 11pt **bold** with high
//! contrast text on a saturated fill. Light/dark palettes are handled by
//! choosing token colors per [`Appearance`]; we never hand-pick hex.

#![warn(clippy::pedantic)]

use gpui::{
    div, App, FontWeight, IntoElement, ParentElement, RenderOnce, Rgba, SharedString, Styled,
    Window,
};

use crate::ui::tokens::{
    color, resolve_color, Appearance, FontSize, Radius, SemanticColor, Spacing,
};

/// Domain entity kinds the badge knows how to color.
///
/// Kept as a closed enum so the compiler forces a decision when a new
/// kind is introduced; the legacy string-keyed helpers are bridged via
/// [`EntityKind::from_legacy_str`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Artist,
    Feed,
    Track,
    Publisher,
    Release,
    Recording,
    Playlist,
    /// Unknown / generic — uses the `Accent` token.
    Generic,
}

impl EntityKind {
    /// Map the legacy `&'static str` keys (e.g. `"feed"`) used throughout
    /// the existing screen code. Unknown values fall back to
    /// [`EntityKind::Generic`] so this never panics on stale call sites.
    #[must_use]
    pub fn from_legacy_str(s: &str) -> Self {
        match s {
            "artist" => Self::Artist,
            "feed" => Self::Feed,
            "track" => Self::Track,
            "publisher" => Self::Publisher,
            "release" => Self::Release,
            "recording" => Self::Recording,
            "playlist" => Self::Playlist,
            _ => Self::Generic,
        }
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Artist => "artist",
            Self::Feed => "feed",
            Self::Track => "track",
            Self::Publisher => "publisher",
            Self::Release => "release",
            Self::Recording => "recording",
            Self::Playlist => "playlist",
            Self::Generic => "item",
        }
    }

    /// Single-character glyph used by [`Thumbnail`] when no image is
    /// available. Kept here so `EntityKind` is the one source of truth
    /// for an entity kind's visual identity.
    #[must_use]
    pub fn emoji(self) -> &'static str {
        match self {
            Self::Artist => "\u{1F3A4}",                    // 🎤
            Self::Feed => "\u{1F4E1}",                      // 📡
            Self::Track => "\u{1F3B6}",                     // 🎶
            Self::Publisher => "\u{1F3E2}",                 // 🏢
            Self::Release => "\u{1F4BF}",                   // 💿
            Self::Playlist => "\u{1F4DD}",                  // 📝
            Self::Recording | Self::Generic => "\u{1F3B5}", // 🎵
        }
    }

    /// Token used as the badge fill / thumbnail tint. We map to the v4vmm
    /// status palette so the colors stay theme-aware (light vs dark) and
    /// pass the WCAG matrix tests against [`SemanticColor::OnAccent`] /
    /// [`SemanticColor::Label`] friends.
    #[must_use]
    pub fn fill_token(self) -> SemanticColor {
        match self {
            Self::Artist => SemanticColor::Success,
            Self::Feed => SemanticColor::Warning,
            Self::Track | Self::Playlist => SemanticColor::Info,
            Self::Publisher => SemanticColor::Danger,
            Self::Release | Self::Recording => SemanticColor::Accent,
            Self::Generic => SemanticColor::SystemFill,
        }
    }

    #[must_use]
    pub fn on_fill_token(self) -> SemanticColor {
        match self {
            Self::Artist => SemanticColor::OnSuccess,
            Self::Feed => SemanticColor::OnWarning,
            Self::Track | Self::Playlist => SemanticColor::OnInfo,
            Self::Publisher => SemanticColor::OnDanger,
            Self::Release | Self::Recording => SemanticColor::OnAccent,
            Self::Generic => SemanticColor::Label,
        }
    }

    #[must_use]
    pub fn fill_color(self, cx: &App) -> Rgba {
        color(cx, self.fill_token())
    }

    #[must_use]
    pub fn on_fill_color(self, cx: &App) -> Rgba {
        color(cx, self.on_fill_token())
    }
}

impl From<&str> for EntityKind {
    fn from(value: &str) -> Self {
        Self::from_legacy_str(value)
    }
}

/// Visual role for metadata provenance/diff states.
///
/// Color and glyph resolve together so comparison state never depends on
/// color alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvenanceRole {
    Match,
    Different,
    Missing,
}

impl ProvenanceRole {
    #[must_use]
    pub fn color_token(self) -> SemanticColor {
        match self {
            Self::Match => SemanticColor::DiffMatch,
            Self::Different => SemanticColor::DiffDifferent,
            Self::Missing => SemanticColor::DiffMissing,
        }
    }

    #[must_use]
    pub fn color(self, cx: &App) -> Rgba {
        color(cx, self.color_token())
    }

    #[must_use]
    pub fn glyph(self) -> &'static str {
        match self {
            Self::Match => "=",
            Self::Different => "\u{2260}",
            Self::Missing => "\u{2205}",
        }
    }

    #[must_use]
    pub fn accessibility_label(self) -> &'static str {
        match self {
            Self::Match => "matches",
            Self::Different => "different",
            Self::Missing => "missing",
        }
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct TagBadge {
    kind: EntityKind,
    /// Optional override for the displayed label (defaults to
    /// [`EntityKind::label`]).
    label: Option<SharedString>,
    appearance: Option<Appearance>,
}

impl TagBadge {
    pub fn new(kind: EntityKind) -> Self {
        Self {
            kind,
            label: None,
            appearance: None,
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = Some(appearance);
        self
    }
}

impl RenderOnce for TagBadge {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let bg = resolve_color(cx, self.kind.fill_token(), self.appearance);
        let fg = resolve_color(cx, self.kind.on_fill_token(), self.appearance);
        let label = self
            .label
            .unwrap_or_else(|| SharedString::from(self.kind.label()));

        div()
            .flex_none()
            .text_size(FontSize::Micro.scaled(cx))
            .font_weight(FontWeight::BOLD)
            .text_color(fg)
            .bg(bg)
            .px(Spacing::SM.scaled(cx))
            .py(Spacing::XXS.scaled(cx))
            .rounded(Radius::SM.scaled(cx))
            .child(label)
    }
}
