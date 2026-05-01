//! Named theme profiles for the visual system boundary.
//!
//! [`ThemeProfile`] is the persisted appearance choice. It stays GPUI-free so
//! config, command tests, and non-UI callers can carry the profile without
//! importing the UI layer.

#![warn(clippy::pedantic)]

use serde::Deserialize;

/// Complete visual profiles supported by the design-system boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeProfile {
    /// Follow the platform light/dark appearance.
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
    pub const USER_SELECTABLE: [Self; 5] = [
        Self::System,
        Self::Dark,
        Self::Light,
        Self::HighContrastDark,
        Self::HighContrastLight,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Dark => "dark",
            Self::Light => "light",
            Self::HighContrastDark => "high-contrast-dark",
            Self::HighContrastLight => "high-contrast-light",
        }
    }

    #[must_use]
    pub const fn settings_label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Dark => "Dark",
            Self::Light => "Light",
            Self::HighContrastDark => "High Contrast Dark",
            Self::HighContrastLight => "High Contrast Light",
        }
    }
}
