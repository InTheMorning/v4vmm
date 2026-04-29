//! Legacy color/spacing/typography helpers. Kept as a backward-compatibility
//! shim so existing call sites compile unchanged while we migrate the codebase
//! to the new [`crate::ui::tokens`] system.
//!
//! New code should prefer `crate::ui::tokens::SemanticColor` and the
//! primitives in `crate::ui::primitives`. Once all call sites are migrated,
//! this module can be removed.

use gpui::{px, FontWeight, Pixels, Rgba, Styled};

use super::tokens::{Appearance, SemanticColor};

/// Resolve a token to its dark-mode value (the only mode v4vmm currently
/// ships). Once the app supports a runtime appearance switch, the call sites
/// using the `color::*` helpers below should migrate to
/// `crate::ui::tokens::color(cx, …)` so they re-resolve automatically.
fn dark(token: SemanticColor) -> Rgba {
    token.resolve(Appearance::Dark)
}

pub mod color {
    use super::{dark, Rgba, SemanticColor};

    pub fn bg_canvas() -> Rgba {
        dark(SemanticColor::SystemBackground)
    }
    pub fn bg_surface() -> Rgba {
        dark(SemanticColor::SecondarySystemBackground)
    }
    pub fn bg_surface_hi() -> Rgba {
        dark(SemanticColor::TertiarySystemBackground)
    }
    pub fn bg_selected() -> Rgba {
        dark(SemanticColor::SelectedContent)
    }

    pub fn border_subtle() -> Rgba {
        dark(SemanticColor::Separator)
    }
    pub fn border_strong() -> Rgba {
        dark(SemanticColor::OpaqueSeparator)
    }

    pub fn text_primary() -> Rgba {
        dark(SemanticColor::Label)
    }
    pub fn text_secondary() -> Rgba {
        dark(SemanticColor::SecondaryLabel)
    }
    pub fn text_muted() -> Rgba {
        dark(SemanticColor::TertiaryLabel)
    }
    pub fn text_on_accent() -> Rgba {
        dark(SemanticColor::OnAccent)
    }

    pub fn accent() -> Rgba {
        dark(SemanticColor::Accent)
    }
    pub fn accent_hover() -> Rgba {
        dark(SemanticColor::AccentHover)
    }
    pub fn accent_pressed() -> Rgba {
        dark(SemanticColor::AccentPressed)
    }

    pub fn focus_ring() -> Rgba {
        dark(SemanticColor::Focus)
    }

    pub fn status_success() -> Rgba {
        dark(SemanticColor::Success)
    }
    pub fn status_warning() -> Rgba {
        dark(SemanticColor::Warning)
    }
    pub fn status_danger() -> Rgba {
        dark(SemanticColor::Danger)
    }

    pub fn diff_match() -> Rgba {
        dark(SemanticColor::DiffMatch)
    }
    pub fn diff_different() -> Rgba {
        dark(SemanticColor::DiffDifferent)
    }
    pub fn diff_missing() -> Rgba {
        dark(SemanticColor::DiffMissing)
    }
}

pub mod badges {
    use super::{dark, Rgba, SemanticColor};
    use gpui::rgb;

    pub fn type_color(entity_type: &str) -> Rgba {
        match entity_type {
            "feed" => rgb(0xe8943a),
            "track" => rgb(0x3ac4c4),
            "publisher" => rgb(0xe84393),
            "artist" => rgb(0x4caf82),
            "release" => rgb(0x6c7cff),
            "recording" => rgb(0xb06cf4),
            _ => dark(SemanticColor::Accent),
        }
    }

    pub fn text_color(entity_type: &str) -> Rgba {
        match entity_type {
            "feed" | "track" => rgb(0x111318),
            _ => rgb(0xffffff),
        }
    }

    pub fn emoji(entity_type: &str) -> &'static str {
        match entity_type {
            "feed" => "\u{1F4E1}",
            "track" => "\u{1F3B6}",
            "publisher" => "\u{1F3E2}",
            _ => "\u{1F3B5}",
        }
    }
}

pub mod layout {
    use super::{px, Pixels};
    pub const TAB_BAR_HEIGHT: Pixels = px(44.0);
    pub const ROW_HEIGHT: Pixels = px(36.0);
    pub const HIT_TARGET_MIN: Pixels = px(28.0);
    pub const INSPECTOR_WIDTH: Pixels = px(360.0);
}

pub mod glyphs {
    pub const DIFF_MATCH: &str = "=";
    pub const DIFF_DIFFERENT: &str = "\u{2260}";
    pub const DIFF_MISSING: &str = "\u{2205}";
    pub const STATUS_SUCCESS: &str = "\u{2713}";
    pub const STATUS_WARNING: &str = "\u{26A0}";
    pub const STATUS_DANGER: &str = "\u{2717}";
}

pub mod spacing {
    use super::{px, Pixels};
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

    pub const SIZE_TITLE: Pixels = px(20.0);
    pub const SIZE_HEADLINE: Pixels = px(15.0);
    pub const SIZE_BODY: Pixels = px(13.0);
    pub const SIZE_CAPTION: Pixels = px(12.0);
    pub const SIZE_MICRO: Pixels = px(11.0);

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
