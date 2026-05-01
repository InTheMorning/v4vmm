//! Named theme profiles for the visual system boundary.
//!
//! [`ThemeProfile`] is the screen-facing appearance choice. It resolves to the
//! lower-level [`Appearance`] tokens today and leaves one place for future
//! profile-specific role mapping.

#![warn(clippy::pedantic)]

use gpui::Rgba;

use crate::ui::tokens::{Appearance, SemanticColor};

/// Complete visual profiles supported by the design-system boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ThemeProfile {
    /// Follow the platform appearance once OS detection exists.
    ///
    /// Until then, this resolves to the app's current default dark profile and
    /// should not be exposed as a visible no-op setting.
    System,
    /// Current v4vmm default profile.
    #[default]
    Dark,
    /// Light semantic-token profile.
    Light,
    /// High-contrast dark profile placeholder.
    HighContrastDark,
    /// High-contrast light profile placeholder.
    HighContrastLight,
}

impl ThemeProfile {
    /// Resolve this profile to the base semantic-token appearance.
    #[must_use]
    pub const fn appearance(self) -> Appearance {
        match self {
            Self::System | Self::Dark | Self::HighContrastDark => Appearance::Dark,
            Self::Light | Self::HighContrastLight => Appearance::Light,
        }
    }

    /// Resolve a semantic token through this profile.
    ///
    /// High-contrast profiles intentionally share the current base palettes
    /// until ADR 0025 later phases introduce profile-specific role values. The
    /// profile-level API is still valuable now because tests and bridge code no
    /// longer bypass the named theme contract.
    #[must_use]
    pub fn resolve(self, token: SemanticColor) -> Rgba {
        token.resolve(self.appearance())
    }
}
