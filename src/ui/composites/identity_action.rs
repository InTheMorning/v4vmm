//! Identity-action button composite.
//!
//! Screens own the click behavior; this composite owns the shared visual
//! vocabulary for identity affordances such as website links and Nostr IDs.
//!
//! Accessibility note (ADR 0038 task 005): [`IdentityActionButtonDisplay`]
//! carries an `a11y_label` sourced from a view-model. GPUI 0.2.x does not yet
//! expose an accessibility-label sink on its button widget, so the field is
//! contract-only today.

#![warn(clippy::pedantic)]

use gpui::SharedString;

use crate::ui::control_styles::ControlStyle;
use crate::ui::icons::IconName;
use crate::ui::primitives::Button;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityActionKind {
    Website,
    Nostr,
    Rss,
}

impl IdentityActionKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Website => "Website",
            Self::Nostr => "Nostr",
            Self::Rss => "RSS",
        }
    }

    const fn leading_icon(self) -> Option<IconName> {
        match self {
            Self::Website => None,
            Self::Nostr => Some(IconName::Nostr),
            Self::Rss => Some(IconName::Rss),
        }
    }

    /// Default screen-reader label when the view-model has no richer text.
    /// The visible label only carries the protocol ("Website", "Nostr",
    /// "RSS"); this expands to a verb so the action reads unambiguously.
    #[must_use]
    pub const fn default_a11y_label(self) -> &'static str {
        match self {
            Self::Website => "Open website",
            Self::Nostr => "Copy Nostr identifier",
            Self::Rss => "Open RSS feed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityActionButtonDisplay {
    pub id: SharedString,
    pub kind: IdentityActionKind,
    pub a11y_label: SharedString,
}

pub fn identity_action_button(display: IdentityActionButtonDisplay) -> Button {
    let IdentityActionButtonDisplay {
        id,
        kind,
        a11y_label: _,
    } = display;
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
    fn rss_action_uses_brand_icon() {
        assert_eq!(IdentityActionKind::Rss.leading_icon(), Some(IconName::Rss));
    }

    #[test]
    fn website_action_is_text_only_until_catalog_has_link_icon() {
        assert_eq!(IdentityActionKind::Website.leading_icon(), None);
    }

    #[test]
    fn default_a11y_label_disambiguates_action_for_each_kind() {
        assert_eq!(
            IdentityActionKind::Website.default_a11y_label(),
            "Open website"
        );
        assert_eq!(
            IdentityActionKind::Nostr.default_a11y_label(),
            "Copy Nostr identifier"
        );
        assert_eq!(
            IdentityActionKind::Rss.default_a11y_label(),
            "Open RSS feed"
        );
    }
}
