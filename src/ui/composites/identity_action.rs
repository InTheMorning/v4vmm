//! Identity-action button composite.
//!
//! Screens own the click behavior; this composite owns the shared visual
//! vocabulary for identity affordances such as website links and Nostr IDs.

#![warn(clippy::pedantic)]

use gpui::ElementId;

use crate::ui::control_styles::ControlStyle;
use crate::ui::icons::IconName;
use crate::ui::primitives::Button;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityActionKind {
    Website,
    Nostr,
}

impl IdentityActionKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Website => "Website",
            Self::Nostr => "Nostr",
        }
    }

    const fn leading_icon(self) -> Option<IconName> {
        match self {
            Self::Website => None,
            Self::Nostr => Some(IconName::Nostr),
        }
    }
}

pub fn identity_action_button(id: impl Into<ElementId>, kind: IdentityActionKind) -> Button {
    let mut button = Button::styled(id, ControlStyle::RowAction).label(kind.label());
    if let Some(icon) = kind.leading_icon() {
        button = button.leading_icon(icon);
    }
    button
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nostr_action_uses_brand_icon() {
        assert_eq!(
            IdentityActionKind::Nostr.leading_icon(),
            Some(IconName::Nostr)
        );
    }

    #[test]
    fn website_action_is_text_only_until_catalog_has_link_icon() {
        assert_eq!(IdentityActionKind::Website.leading_icon(), None);
    }
}
