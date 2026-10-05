//! Entity-type tag badge — a small dot in the entity color, followed by the
//! kind word in a label color, identifying what kind of record (artist,
//! feed, track, …) a card represents.
//!
//! ADR 0083 Decision 2: an entity color shows only as a dot. No text is
//! drawn on an entity color. Light/dark palettes are handled by choosing
//! token colors per [`Appearance`]. We never hand-pick hex.

#![warn(clippy::pedantic)]

use gpui::{
    div, App, FontWeight, IntoElement, ParentElement, RenderOnce, Rgba, SharedString, Styled,
    Window,
};

use crate::ui::icons::IconName;
use crate::ui::tokens::{
    color, resolve_color, Appearance, FontSize, Radius, SemanticColor, Spacing,
};
use crate::view_models::track_metadata_grid::TrackMetadataComparisonRole;

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

    /// Two-letter type monogram used by [`Thumbnail`] when no image is
    /// available. Kept here so `EntityKind` is the one source of truth
    /// for an entity kind's visual identity.
    ///
    /// ADR 0083 Decision 4: a missing cover shows a tinted placeholder with
    /// this monogram, not an emoji.
    #[must_use]
    pub fn monogram(self) -> &'static str {
        match self {
            Self::Artist => "AR",
            Self::Feed => "FD",
            Self::Track => "TR",
            Self::Publisher => "PB",
            Self::Release => "AL",
            Self::Recording => "RC",
            Self::Playlist => "PL",
            Self::Generic => "IT",
        }
    }

    /// Entity-palette token for this kind's identity dot.
    ///
    /// ADR 0083 Decision 2: an entity color marks a stated kind and never
    /// reuses a status, accent, or diff token. `Release` takes the feed
    /// color and `Recording` takes the track color, because `search.html`
    /// groups them in the same parent color. `Generic` states no kind, so
    /// it uses a neutral fill, not an entity color.
    #[must_use]
    pub fn fill_token(self) -> SemanticColor {
        match self {
            Self::Artist => SemanticColor::EntityArtist,
            Self::Feed | Self::Release => SemanticColor::EntityFeed,
            Self::Track | Self::Recording => SemanticColor::EntityTrack,
            Self::Playlist => SemanticColor::EntityPlaylist,
            Self::Publisher => SemanticColor::EntityPublisher,
            Self::Generic => SemanticColor::SystemFill,
        }
    }

    #[must_use]
    pub fn fill_color(self, cx: &App) -> Rgba {
        color(cx, self.fill_token())
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

impl From<TrackMetadataComparisonRole> for ProvenanceRole {
    fn from(role: TrackMetadataComparisonRole) -> Self {
        match role {
            TrackMetadataComparisonRole::Match => Self::Match,
            TrackMetadataComparisonRole::Different => Self::Different,
            TrackMetadataComparisonRole::Missing => Self::Missing,
        }
    }
}

/// Visual role for general status messages.
///
/// Color and icon resolve together so status does not rely on color alone.
/// ADR 0083 Decision 10: the icon is an `IconName`, drawn as a Lucide icon.
/// A status mark keeps its word beside the icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusRole {
    Success,
    Warning,
    Danger,
}

impl StatusRole {
    #[must_use]
    pub fn color_token(self) -> SemanticColor {
        match self {
            Self::Success => SemanticColor::Success,
            Self::Warning => SemanticColor::Warning,
            Self::Danger => SemanticColor::Danger,
        }
    }

    #[must_use]
    pub fn color(self, cx: &App) -> Rgba {
        color(cx, self.color_token())
    }

    #[must_use]
    pub const fn icon(self) -> IconName {
        match self {
            Self::Success => IconName::Check,
            Self::Warning => IconName::Warning,
            Self::Danger => IconName::Close,
        }
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct TagBadge {
    kind: EntityKind,
    label: Option<SharedString>,
    appearance: Option<Appearance>,
}

/// Display-ready badge fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagBadgeDisplay {
    pub kind: EntityKind,
    pub label: Option<SharedString>,
}

impl TagBadge {
    pub fn new(display: TagBadgeDisplay) -> Self {
        Self {
            kind: display.kind,
            label: display.label,
            appearance: None,
        }
    }

    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = Some(appearance);
        self
    }
}

impl RenderOnce for TagBadge {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        // ADR 0083 Decision 2: the entity color shows only as a dot. The
        // kind word always renders in a label color, never on the entity
        // fill, so color is never the only carrier of the kind.
        let dot_color = resolve_color(cx, self.kind.fill_token(), self.appearance);
        let label_color = resolve_color(cx, SemanticColor::Label, self.appearance);
        let label = self
            .label
            .unwrap_or_else(|| SharedString::from(self.kind.label()));
        let dot_size = Spacing::SM.scaled(cx);

        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(Spacing::XXS.scaled(cx))
            .child(
                div()
                    .flex_none()
                    .w(dot_size)
                    .h(dot_size)
                    .rounded(Radius::Full.scaled(cx))
                    .bg(dot_color),
            )
            .child(
                div()
                    .text_size(FontSize::Micro.scaled(cx))
                    .font_weight(FontWeight::BOLD)
                    .text_color(label_color)
                    .child(label),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_badge_uses_display_contract() {
        let badge = TagBadge::new(TagBadgeDisplay {
            kind: EntityKind::Feed,
            label: Some(SharedString::from("podcast")),
        });

        assert_eq!(badge.kind, EntityKind::Feed);
        assert_eq!(badge.label, Some(SharedString::from("podcast")));
    }

    #[test]
    fn status_roles_resolve_color_and_icon_together() {
        assert_eq!(StatusRole::Success.color_token(), SemanticColor::Success);
        assert_eq!(StatusRole::Warning.color_token(), SemanticColor::Warning);
        assert_eq!(StatusRole::Danger.color_token(), SemanticColor::Danger);
        assert_eq!(StatusRole::Success.icon(), IconName::Check);
        assert_eq!(StatusRole::Warning.icon(), IconName::Warning);
        assert_eq!(StatusRole::Danger.icon(), IconName::Close);
    }

    /// R83-03: each `EntityKind` resolves to an entity token, and no two
    /// kinds with different `search.html` colors share one token.
    #[test]
    fn entity_kind_fill_tokens_match_the_website_color_grouping() {
        use std::collections::HashSet;

        // These five kinds each have their own color on the website.
        let distinctly_colored = [
            EntityKind::Artist,
            EntityKind::Feed,
            EntityKind::Track,
            EntityKind::Publisher,
            EntityKind::Playlist,
        ];
        let mut seen_tokens = HashSet::new();
        for kind in distinctly_colored {
            assert!(
                seen_tokens.insert(kind.fill_token()),
                "{kind:?} must not share its entity token with another \
                 distinctly colored kind"
            );
        }

        // ADR 0083 Decision 1: Release takes Feed's color, and Recording
        // takes Track's color.
        assert_eq!(
            EntityKind::Release.fill_token(),
            EntityKind::Feed.fill_token()
        );
        assert_eq!(
            EntityKind::Recording.fill_token(),
            EntityKind::Track.fill_token()
        );

        // Generic states no kind, so it never collides with a real entity
        // color.
        assert!(!seen_tokens.contains(&EntityKind::Generic.fill_token()));
    }

    /// R83-03 / R83-04: an entity kind never resolves to a status, accent,
    /// or diff token. ADR 0083 Decision 2.
    #[test]
    fn entity_kind_fill_tokens_never_resolve_a_status_accent_or_diff_token() {
        let forbidden = [
            SemanticColor::Success,
            SemanticColor::Warning,
            SemanticColor::Danger,
            SemanticColor::Info,
            SemanticColor::Accent,
            SemanticColor::DiffMatch,
            SemanticColor::DiffDifferent,
            SemanticColor::DiffMissing,
        ];
        for kind in [
            EntityKind::Artist,
            EntityKind::Feed,
            EntityKind::Track,
            EntityKind::Publisher,
            EntityKind::Release,
            EntityKind::Recording,
            EntityKind::Playlist,
            EntityKind::Generic,
        ] {
            assert!(
                !forbidden.contains(&kind.fill_token()),
                "{kind:?} must not resolve a status, accent, or diff token"
            );
        }
    }

    /// R83-15: each `EntityKind` resolves a two-letter monogram, used by
    /// `Thumbnail` when no cover image is available.
    #[test]
    fn entity_kind_monogram_is_two_letters_for_every_kind() {
        for kind in [
            EntityKind::Artist,
            EntityKind::Feed,
            EntityKind::Track,
            EntityKind::Publisher,
            EntityKind::Release,
            EntityKind::Recording,
            EntityKind::Playlist,
            EntityKind::Generic,
        ] {
            assert_eq!(
                kind.monogram().chars().count(),
                2,
                "{kind:?} monogram must be exactly two letters"
            );
        }
    }
}
