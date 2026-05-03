//! `DisclosureGroup` composite — `SwiftUI`-style collapsible section
//! header with click-to-toggle.
//!
//! Borrowed from `SwiftUI`:
//!
//! ```swift
//! DisclosureGroup("Label", isExpanded: $expanded) { body }
//! ```
//!
//! v4vmm uses this exclusively for headers today (the body is rendered
//! conditionally by the screen below the header), so the composite
//! exposes only the header. Clicking the entire row fires the
//! `on_toggle` callback. The row is fully wrapped as a stateful
//! pointer cursor — callers don't need to remember to add
//! `cursor_pointer()` themselves.
//!
//! ```ignore
//! DisclosureGroup::new(DisclosureGroupDisplay {
//!     id: "disco-section".into(),
//!     label: "Recently played".into(),
//! })
//!     .collapsed(self.recently_played_collapsed)
//!     .on_toggle(cx.listener(|this, _, _, cx| {
//!         this.toggle_recently_played(cx);
//!     }))
//! ```
//!
//! The `collapsed` flag mirrors the legacy `render_clickable_section_heading`
//! contract — pass `true` for the collapsed state, `false` for expanded.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    div, prelude::*, App, ClickEvent, ElementId, InteractiveElement, IntoElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window,
};

use crate::ui::primitives::SectionHeader;

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
#[must_use]
pub struct DisclosureGroup {
    id: ElementId,
    label: SharedString,
    collapsed: bool,
    on_toggle: Option<ClickHandler>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisclosureGroupDisplay {
    pub id: ElementId,
    pub label: SharedString,
}

impl DisclosureGroup {
    pub fn new(display: DisclosureGroupDisplay) -> Self {
        Self {
            id: display.id,
            label: display.label,
            collapsed: false,
            on_toggle: None,
        }
    }

    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    pub fn on_toggle(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for DisclosureGroup {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut row = div()
            .id(self.id)
            .cursor_pointer()
            .child(SectionHeader::new(self.label).disclosure(self.collapsed));
        if let Some(handler) = self.on_toggle {
            row = row.on_click(move |event, window, cx| handler(event, window, cx));
        }
        row
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_expanded_with_no_handler() {
        let g = DisclosureGroup::new(DisclosureGroupDisplay {
            id: "d".into(),
            label: "Hello".into(),
        });
        assert!(!g.collapsed);
        assert!(g.on_toggle.is_none());
    }

    #[test]
    fn modifiers_set_their_fields() {
        let g = DisclosureGroup::new(DisclosureGroupDisplay {
            id: "d".into(),
            label: "Hello".into(),
        })
        .collapsed(true)
        .on_toggle(|_, _, _| {});
        assert!(g.collapsed);
        assert!(g.on_toggle.is_some());
    }
}
