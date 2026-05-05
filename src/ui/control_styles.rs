//! Semantic control-style roles mapped onto native button primitives.
//!
//! Screens choose product intent (`MetadataAction`, `RowAction`, ...). The
//! design system owns the concrete native button variant, size, typography,
//! border, and token choices.

#![warn(clippy::pedantic)]

use crate::ui::primitives::{ButtonSize, ButtonVariant};
use crate::ui::tokens::{FontSize, Radius, SemanticColor};

/// Reusable button/action roles admitted by ADR 0025.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlStyle {
    Primary,
    Secondary,
    Ghost,
    Destructive,
    ToolbarIcon,
    RowAction,
    DestructiveRowAction,
    MetadataAction,
    Pill,
}

/// Pure style mapping consumed by the native [`crate::ui::primitives::Button`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlStyleSpec {
    pub variant: ButtonVariant,
    pub size: ButtonSize,
    pub font_size: FontSize,
    pub radius: Radius,
    pub foreground: Option<SemanticColor>,
    pub border: Option<SemanticColor>,
}

impl ControlStyle {
    /// Return the pure role-to-token mapping for this control style.
    #[must_use]
    pub const fn spec(self) -> ControlStyleSpec {
        match self {
            Self::Primary => ControlStyleSpec {
                variant: ButtonVariant::Filled,
                size: ButtonSize::Md,
                font_size: FontSize::Body,
                radius: Radius::MD,
                foreground: None,
                border: None,
            },
            Self::Secondary => ControlStyleSpec {
                variant: ButtonVariant::Tinted,
                size: ButtonSize::Md,
                font_size: FontSize::Body,
                radius: Radius::MD,
                foreground: None,
                border: None,
            },
            Self::Ghost => ControlStyleSpec {
                variant: ButtonVariant::Plain,
                size: ButtonSize::Md,
                font_size: FontSize::Body,
                radius: Radius::MD,
                foreground: Some(SemanticColor::Accent),
                border: None,
            },
            Self::Destructive => ControlStyleSpec {
                variant: ButtonVariant::Destructive,
                size: ButtonSize::Md,
                font_size: FontSize::Body,
                radius: Radius::MD,
                foreground: None,
                border: None,
            },
            Self::ToolbarIcon | Self::RowAction => ControlStyleSpec {
                variant: ButtonVariant::Plain,
                size: ButtonSize::Sm,
                font_size: FontSize::Caption,
                radius: Radius::SM,
                foreground: Some(SemanticColor::Accent),
                border: None,
            },
            Self::DestructiveRowAction => ControlStyleSpec {
                variant: ButtonVariant::Plain,
                size: ButtonSize::Sm,
                font_size: FontSize::Caption,
                radius: Radius::SM,
                foreground: Some(SemanticColor::DangerLabel),
                border: None,
            },
            Self::MetadataAction => ControlStyleSpec {
                variant: ButtonVariant::Plain,
                size: ButtonSize::Sm,
                font_size: FontSize::Micro,
                radius: Radius::SM,
                foreground: Some(SemanticColor::Accent),
                border: Some(SemanticColor::Accent),
            },
            Self::Pill => ControlStyleSpec {
                variant: ButtonVariant::Tinted,
                size: ButtonSize::Sm,
                font_size: FontSize::Micro,
                radius: Radius::Full,
                foreground: Some(SemanticColor::OnAccent),
                border: None,
            },
        }
    }

    /// Whether this compact action role should expose hover help by default.
    #[must_use]
    pub const fn prefers_tooltip(self) -> bool {
        matches!(
            self,
            Self::ToolbarIcon | Self::RowAction | Self::DestructiveRowAction
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_action_is_accent_bordered_plain_button() {
        let spec = ControlStyle::MetadataAction.spec();
        assert_eq!(spec.variant, ButtonVariant::Plain);
        assert_eq!(spec.size, ButtonSize::Sm);
        assert_eq!(spec.font_size, FontSize::Micro);
        assert_eq!(spec.radius, Radius::SM);
        assert_eq!(spec.foreground, Some(SemanticColor::Accent));
        assert_eq!(spec.border, Some(SemanticColor::Accent));
    }

    #[test]
    fn destructive_uses_native_destructive_variant() {
        let spec = ControlStyle::Destructive.spec();
        assert_eq!(spec.variant, ButtonVariant::Destructive);
        assert!(spec.border.is_none());
        assert!(spec.foreground.is_none());
    }

    #[test]
    fn toolbar_and_row_actions_share_compact_plain_mapping() {
        let toolbar = ControlStyle::ToolbarIcon.spec();
        let row = ControlStyle::RowAction.spec();
        assert_eq!(toolbar, row);
        assert_eq!(toolbar.variant, ButtonVariant::Plain);
        assert_eq!(toolbar.size, ButtonSize::Sm);
    }

    #[test]
    fn destructive_row_action_is_compact_plain_danger_text() {
        let spec = ControlStyle::DestructiveRowAction.spec();
        assert_eq!(spec.variant, ButtonVariant::Plain);
        assert_eq!(spec.size, ButtonSize::Sm);
        assert_eq!(spec.font_size, FontSize::Caption);
        assert_eq!(spec.radius, Radius::SM);
        assert_eq!(spec.foreground, Some(SemanticColor::DangerLabel));
        assert!(spec.border.is_none());
    }

    #[test]
    fn compact_action_roles_prefer_tooltips() {
        assert!(ControlStyle::ToolbarIcon.prefers_tooltip());
        assert!(ControlStyle::RowAction.prefers_tooltip());
        assert!(ControlStyle::DestructiveRowAction.prefers_tooltip());
        assert!(!ControlStyle::Primary.prefers_tooltip());
        assert!(!ControlStyle::Ghost.prefers_tooltip());
    }
}
