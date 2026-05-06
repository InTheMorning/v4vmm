//! Tooltip primitive for compact action discoverability.
//!
//! macOS help tags should describe the indicated control only, stay brief,
//! and use action-oriented text. This wrapper keeps tooltip construction
//! centralized so screens pass display-ready copy instead of binding directly
//! to `gpui_component` tooltip chrome.

#![warn(clippy::pedantic)]

use gpui::{AnyView, App, SharedString, Window};
use gpui_component::tooltip::Tooltip as ComponentTooltip;

/// Brief hover help for a single interactive control.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tooltip {
    label: SharedString,
}

impl Tooltip {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
        }
    }

    #[must_use]
    pub fn non_empty(label: impl Into<SharedString>) -> Option<Self> {
        let label = label.into();
        (!label.to_string().trim().is_empty()).then_some(Self { label })
    }

    #[must_use]
    pub fn label(&self) -> SharedString {
        self.label.clone()
    }

    pub fn build(&self, window: &mut Window, cx: &mut App) -> AnyView {
        ComponentTooltip::new(self.label.clone()).build(window, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tooltip_rejects_empty_hover_copy() {
        assert!(Tooltip::non_empty("").is_none());
        assert!(Tooltip::non_empty("   ").is_none());
    }

    #[test]
    fn tooltip_carries_display_ready_label() {
        let tooltip = Tooltip::new("Add to playlist");

        assert_eq!(tooltip.label(), SharedString::from("Add to playlist"));
    }
}
