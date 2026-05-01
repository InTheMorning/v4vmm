//! Visual roles used while screens finish migrating.
//!
//! This module replaces the old `ui::theme` compatibility shim. Color roles
//! resolve through the profile installed by `theme_bridge`, so runtime theme
//! changes repaint without dark-only helper calls. The spacing, radius, and
//! typography constants are fixed geometry kept for legacy screen parity until
//! those call sites move fully onto token enums.

#![warn(clippy::pedantic)]

use std::sync::atomic::{AtomicU8, Ordering};

use gpui::{px, FontWeight, Pixels, Rgba, Styled};

use crate::theme_profile::ThemeProfile;
use crate::ui::theme_profiles::resolve_profile_color;
use crate::ui::tokens::SemanticColor;

static CURRENT_PROFILE: AtomicU8 = AtomicU8::new(profile_to_raw(ThemeProfile::Dark));

pub(crate) fn install_profile(profile: ThemeProfile) {
    CURRENT_PROFILE.store(profile_to_raw(profile), Ordering::Relaxed);
}

const fn profile_to_raw(profile: ThemeProfile) -> u8 {
    match profile {
        ThemeProfile::System => 0,
        ThemeProfile::Dark => 1,
        ThemeProfile::Light => 2,
        ThemeProfile::HighContrastDark => 3,
        ThemeProfile::HighContrastLight => 4,
    }
}

fn current_profile() -> ThemeProfile {
    match CURRENT_PROFILE.load(Ordering::Relaxed) {
        0 => ThemeProfile::System,
        2 => ThemeProfile::Light,
        3 => ThemeProfile::HighContrastDark,
        4 => ThemeProfile::HighContrastLight,
        _ => ThemeProfile::Dark,
    }
}

fn role(token: SemanticColor) -> Rgba {
    resolve_profile_color(current_profile(), token)
}

pub mod color {
    #![expect(
        clippy::must_use_candidate,
        reason = "compatibility color roles are consumed directly by GPUI builder chains"
    )]

    use super::{role, Rgba, SemanticColor};

    pub fn bg_canvas() -> Rgba {
        role(SemanticColor::SystemBackground)
    }
    pub fn bg_surface() -> Rgba {
        role(SemanticColor::SecondarySystemBackground)
    }
    pub fn bg_surface_hi() -> Rgba {
        role(SemanticColor::TertiarySystemBackground)
    }
    pub fn bg_selected() -> Rgba {
        role(SemanticColor::SelectedContent)
    }

    pub fn border_subtle() -> Rgba {
        role(SemanticColor::Separator)
    }
    pub fn border_strong() -> Rgba {
        role(SemanticColor::OpaqueSeparator)
    }

    pub fn text_primary() -> Rgba {
        role(SemanticColor::Label)
    }
    pub fn text_secondary() -> Rgba {
        role(SemanticColor::SecondaryLabel)
    }
    pub fn text_muted() -> Rgba {
        role(SemanticColor::TertiaryLabel)
    }
    pub fn text_on_accent() -> Rgba {
        role(SemanticColor::OnAccent)
    }

    pub fn accent() -> Rgba {
        role(SemanticColor::Accent)
    }
    pub fn accent_hover() -> Rgba {
        role(SemanticColor::AccentHover)
    }
    pub fn accent_pressed() -> Rgba {
        role(SemanticColor::AccentPressed)
    }

    pub fn focus_ring() -> Rgba {
        role(SemanticColor::Focus)
    }

    pub fn status_success() -> Rgba {
        role(SemanticColor::Success)
    }
    pub fn status_warning() -> Rgba {
        role(SemanticColor::Warning)
    }
    pub fn status_danger() -> Rgba {
        role(SemanticColor::Danger)
    }

    pub fn diff_match() -> Rgba {
        role(SemanticColor::DiffMatch)
    }
    pub fn diff_different() -> Rgba {
        role(SemanticColor::DiffDifferent)
    }
    pub fn diff_missing() -> Rgba {
        role(SemanticColor::DiffMissing)
    }

    pub fn id3_frame_v22() -> Rgba {
        gpui::rgb(0x00b0_6cf4)
    }
    pub fn id3_frame_v23_only() -> Rgba {
        gpui::rgb(0x00ff_c857)
    }
    pub fn id3_frame_v24_only() -> Rgba {
        gpui::rgb(0x003a_c4c4)
    }
    pub fn id3_frame_unknown() -> Rgba {
        gpui::rgb(0x00ff_8a65)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusRole {
    Success,
    Warning,
    Danger,
}

impl StatusRole {
    #[must_use]
    pub fn color(self) -> Rgba {
        match self {
            Self::Success => color::status_success(),
            Self::Warning => color::status_warning(),
            Self::Danger => color::status_danger(),
        }
    }

    #[must_use]
    pub const fn glyph(self) -> &'static str {
        match self {
            Self::Success => "\u{2713}",
            Self::Warning => "\u{26A0}",
            Self::Danger => "\u{2717}",
        }
    }
}

pub mod spacing {
    use super::{px, Pixels};
    pub const NONE: Pixels = px(0.0);
    pub const XXS: Pixels = px(2.0);
    pub const XS: Pixels = px(4.0);
    pub const SM: Pixels = px(8.0);
    pub const MD: Pixels = px(12.0);
    pub const LG: Pixels = px(16.0);
    pub const XL: Pixels = px(24.0);
    pub const XXL: Pixels = px(32.0);
}

pub mod radius {
    use super::{px, Pixels};
    pub const SM: Pixels = px(4.0);
    pub const MD: Pixels = px(6.0);
    pub const LG: Pixels = px(10.0);
}

pub mod typography {
    use super::{px, FontWeight, Pixels, Styled};

    pub const LINE_TIGHT: Pixels = px(14.0);
    pub const LINE_COMPACT: Pixels = px(15.0);
    pub const LINE_BODY: Pixels = px(16.0);
    pub const LINE_DETAIL: Pixels = px(17.0);
    pub const LINE_TITLE: Pixels = px(20.0);
    pub const LINE_HEADER: Pixels = px(23.0);
    pub const SIZE_TITLE: Pixels = px(20.0);
    pub const SIZE_HEADLINE: Pixels = px(15.0);
    pub const SIZE_BODY: Pixels = px(13.0);
    pub const SIZE_CAPTION: Pixels = px(12.0);
    pub const SIZE_MICRO: Pixels = px(11.0);
    pub const SIZE_ARTWORK_FALLBACK: Pixels = px(28.0);

    pub fn type_title<T: Styled>(el: T) -> T {
        el.text_size(SIZE_TITLE).font_weight(FontWeight::SEMIBOLD)
    }

    pub fn type_headline<T: Styled>(el: T) -> T {
        el.text_size(SIZE_HEADLINE)
            .font_weight(FontWeight::SEMIBOLD)
    }

    pub fn type_body<T: Styled>(el: T) -> T {
        el.text_size(SIZE_BODY).font_weight(FontWeight::NORMAL)
    }

    pub fn type_caption<T: Styled>(el: T) -> T {
        el.text_size(SIZE_CAPTION).font_weight(FontWeight::NORMAL)
    }

    pub fn type_caption_strong<T: Styled>(el: T) -> T {
        el.text_size(SIZE_CAPTION).font_weight(FontWeight::MEDIUM)
    }

    pub fn type_micro<T: Styled>(el: T) -> T {
        el.text_size(SIZE_MICRO).font_weight(FontWeight::MEDIUM)
    }
}
