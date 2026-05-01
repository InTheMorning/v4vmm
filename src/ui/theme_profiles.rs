//! Profile-specific semantic color resolution.
//!
//! [`crate::theme_profile::ThemeProfile`] stays GPUI-free at the crate root.
//! This module is the UI-layer resolver that maps a persisted profile onto
//! concrete token colors.

#![warn(clippy::pedantic)]

use gpui::Rgba;

use crate::theme_profile::ThemeProfile;
use crate::ui::tokens::{Appearance, SemanticColor};

/// Resolve a semantic color through a named visual profile.
#[must_use]
pub fn resolve_profile_color(profile: ThemeProfile, token: SemanticColor) -> Rgba {
    match profile {
        ThemeProfile::System | ThemeProfile::Dark => token.resolve(Appearance::Dark),
        ThemeProfile::Light => token.resolve(Appearance::Light),
        ThemeProfile::HighContrastDark => high_contrast_dark(token),
        ThemeProfile::HighContrastLight => high_contrast_light(token),
    }
}

/// Resolve a profile to its base light/dark appearance.
#[must_use]
pub const fn appearance_for_profile(profile: ThemeProfile) -> Appearance {
    match profile {
        ThemeProfile::System | ThemeProfile::Dark | ThemeProfile::HighContrastDark => {
            Appearance::Dark
        }
        ThemeProfile::Light | ThemeProfile::HighContrastLight => Appearance::Light,
    }
}

#[expect(
    clippy::match_same_arms,
    reason = "different roles share accessible colors"
)]
fn high_contrast_dark(token: SemanticColor) -> Rgba {
    match token {
        SemanticColor::SystemBackground => hex(0x00_0000),
        SemanticColor::SecondarySystemBackground => hex(0x08_0808),
        SemanticColor::TertiarySystemBackground => hex(0x16_1616),

        SemanticColor::Label => hex(0xff_ffff),
        SemanticColor::SecondaryLabel => hex(0xf2_f2f2),
        SemanticColor::TertiaryLabel => hex(0xd8_d8d8),
        SemanticColor::QuaternaryLabel => hex(0xb8_b8b8),

        SemanticColor::SystemFill => hex(0x2a_2a2a),
        SemanticColor::SecondaryFill => hex(0x1f_1f1f),
        SemanticColor::TertiaryFill => hex(0x16_1616),

        SemanticColor::SelectedContent => hex(0x00_3352),

        SemanticColor::Separator => hex(0x7a_7a7a),
        SemanticColor::OpaqueSeparator => hex(0xd0_d0d0),

        SemanticColor::Accent => hex(0x66_d9ff),
        SemanticColor::AccentHover => hex(0x9a_e8ff),
        SemanticColor::AccentPressed => hex(0x38_caff),
        SemanticColor::OnAccent => hex(0x00_0000),
        SemanticColor::Focus => hex(0xff_f06a),

        SemanticColor::Success => hex(0x00_e676),
        SemanticColor::Warning => hex(0xff_d60a),
        SemanticColor::Danger => hex(0xff_6b6b),
        SemanticColor::Info => hex(0x66_d9ff),

        SemanticColor::SuccessLabel => hex(0x00_e676),
        SemanticColor::WarningLabel => hex(0xff_d60a),
        SemanticColor::DangerLabel => hex(0xff_6b6b),
        SemanticColor::InfoLabel => hex(0x66_d9ff),

        SemanticColor::OnSuccess => hex(0x00_0000),
        SemanticColor::OnWarning => hex(0x00_0000),
        SemanticColor::OnDanger => hex(0x00_0000),
        SemanticColor::OnInfo => hex(0x00_0000),

        SemanticColor::DiffMatch => hex(0x00_e676),
        SemanticColor::DiffDifferent => hex(0xff_d60a),
        SemanticColor::DiffMissing => hex(0xff_8a65),
    }
}

#[expect(
    clippy::match_same_arms,
    reason = "different roles share accessible colors"
)]
fn high_contrast_light(token: SemanticColor) -> Rgba {
    match token {
        SemanticColor::SystemBackground => hex(0xff_ffff),
        SemanticColor::SecondarySystemBackground => hex(0xf7_f7f7),
        SemanticColor::TertiarySystemBackground => hex(0xee_eeee),

        SemanticColor::Label => hex(0x00_0000),
        SemanticColor::SecondaryLabel => hex(0x1c_1c1e),
        SemanticColor::TertiaryLabel => hex(0x3a_3a3c),
        SemanticColor::QuaternaryLabel => hex(0x5f_5f63),

        SemanticColor::SystemFill => hex(0xe0_e0e0),
        SemanticColor::SecondaryFill => hex(0xea_eaea),
        SemanticColor::TertiaryFill => hex(0xf2_f2f2),

        SemanticColor::SelectedContent => hex(0xbf_d3ff),

        SemanticColor::Separator => hex(0x76_7676),
        SemanticColor::OpaqueSeparator => hex(0x3a_3a3c),

        SemanticColor::Accent => hex(0x00_40dd),
        SemanticColor::AccentHover => hex(0x00_53ff),
        SemanticColor::AccentPressed => hex(0x00_32a8),
        SemanticColor::OnAccent => hex(0xff_ffff),
        SemanticColor::Focus => hex(0x00_40dd),

        SemanticColor::Success => hex(0x00_6d2c),
        SemanticColor::Warning => hex(0x7a_4b00),
        SemanticColor::Danger => hex(0xb0_0020),
        SemanticColor::Info => hex(0x00_40dd),

        SemanticColor::SuccessLabel => hex(0x00_6d2c),
        SemanticColor::WarningLabel => hex(0x7a_4b00),
        SemanticColor::DangerLabel => hex(0xb0_0020),
        SemanticColor::InfoLabel => hex(0x00_40dd),

        SemanticColor::OnSuccess => hex(0xff_ffff),
        SemanticColor::OnWarning => hex(0xff_ffff),
        SemanticColor::OnDanger => hex(0xff_ffff),
        SemanticColor::OnInfo => hex(0xff_ffff),

        SemanticColor::DiffMatch => hex(0x00_6d2c),
        SemanticColor::DiffDifferent => hex(0x7a_4b00),
        SemanticColor::DiffMissing => hex(0xb0_0020),
    }
}

#[inline]
const fn hex(rgb: u32) -> Rgba {
    #[expect(
        clippy::cast_precision_loss,
        reason = "byte values 0..=255 fit exactly in f32"
    )]
    Rgba {
        r: ((rgb >> 16) & 0xff) as f32 / 255.0,
        g: ((rgb >> 8) & 0xff) as f32 / 255.0,
        b: (rgb & 0xff) as f32 / 255.0,
        a: 1.0,
    }
}
