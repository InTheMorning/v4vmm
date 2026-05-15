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
//!
//! Accessibility note (ADR 0038 task 005): [`DisclosureGroupDisplay`] carries
//! the VM-sourced action label for the toggle affordance. GPUI 0.2.x does not
//! expose a final accessibility sink for this `div`, so the field is retained
//! as contract data.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    div, prelude::*, App, ClickEvent, ElementId, InteractiveElement, IntoElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window,
};

use crate::ui::primitives::{Label, MultilineText, SectionHeader};
use crate::ui::tokens::{FontSize, Radius, SemanticColor, Size, Spacing};

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
#[must_use]
pub struct DisclosureGroup {
    id: ElementId,
    label: SharedString,
    a11y_label: SharedString,
    collapsed: bool,
    on_toggle: Option<ClickHandler>,
}

/// Fixed-width disclosure glyph used beside tree and sidebar labels.
#[derive(IntoElement)]
#[must_use]
pub struct DisclosureIndicator {
    glyph: SharedString,
}

/// Muted supplemental label shown at the trailing edge of disclosure rows.
#[derive(IntoElement)]
#[must_use]
pub struct DisclosureSupplementLabel {
    label: SharedString,
}

/// Bordered text panel with a disclosure header.
#[derive(IntoElement)]
#[must_use]
pub struct DisclosureTextPanel {
    display: DisclosureTextPanelDisplay,
    on_toggle: Option<ClickHandler>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisclosureIndicatorDisplay {
    pub glyph: SharedString,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisclosureSupplementDisplay {
    pub label: SharedString,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisclosureGroupDisplay {
    pub id: ElementId,
    pub label: SharedString,
    pub a11y_label: SharedString,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisclosureTextPanelDisplay {
    pub id: ElementId,
    pub label: SharedString,
    pub a11y_label: SharedString,
    pub body: SharedString,
    pub collapsed: bool,
}

impl DisclosureGroup {
    pub fn new(display: DisclosureGroupDisplay) -> Self {
        Self {
            id: display.id,
            label: display.label,
            a11y_label: display.a11y_label,
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

impl DisclosureIndicator {
    pub fn new(display: DisclosureIndicatorDisplay) -> Self {
        Self {
            glyph: display.glyph,
        }
    }
}

impl DisclosureSupplementLabel {
    pub fn new(display: DisclosureSupplementDisplay) -> Self {
        Self {
            label: display.label,
        }
    }
}

impl DisclosureTextPanel {
    pub fn new(display: DisclosureTextPanelDisplay) -> Self {
        Self {
            display,
            on_toggle: None,
        }
    }

    pub fn on_toggle(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for DisclosureIndicator {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div().w(Spacing::MD.scaled(cx)).child(
            Label::new(self.glyph)
                .size(FontSize::Caption)
                .color(SemanticColor::TertiaryLabel),
        )
    }
}

impl RenderOnce for DisclosureSupplementLabel {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        Label::new(self.label)
            .size(FontSize::Caption)
            .color(SemanticColor::TertiaryLabel)
    }
}

impl RenderOnce for DisclosureGroup {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let _a11y_label = self.a11y_label;
        let mut row = div()
            .id(self.id)
            .min_h(Size::MinHitTarget.scaled(cx))
            .cursor_pointer()
            .child(SectionHeader::new(self.label).disclosure(self.collapsed));
        if let Some(handler) = self.on_toggle {
            row = row.on_click(move |event, window, cx| handler(event, window, cx));
        }
        row
    }
}

impl RenderOnce for DisclosureTextPanel {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let collapsed = self.display.collapsed;
        let mut header = DisclosureGroup::new(DisclosureGroupDisplay {
            id: self.display.id,
            label: self.display.label,
            a11y_label: self.display.a11y_label,
        })
        .collapsed(collapsed);
        if let Some(handler) = self.on_toggle {
            header = header.on_toggle(move |event, window, cx| handler(event, window, cx));
        }

        div()
            .border_1()
            .border_color(crate::ui::tokens::color(cx, SemanticColor::Separator))
            .rounded(Radius::MD.scaled(cx))
            .p(Spacing::SM.scaled(cx))
            .child(header)
            .when(!collapsed, |el| {
                el.child(
                    div().mt(Spacing::XS.scaled(cx)).child(
                        MultilineText::new(self.display.body)
                            .max_lines(usize::MAX)
                            .wrap_lines()
                            .size(FontSize::Micro)
                            .color(SemanticColor::Label),
                    ),
                )
            })
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
            a11y_label: "Toggle Hello section".into(),
        });
        assert!(!g.collapsed);
        assert!(g.on_toggle.is_none());
        assert_eq!(g.a11y_label, SharedString::from("Toggle Hello section"));
    }

    #[test]
    fn modifiers_set_their_fields() {
        let g = DisclosureGroup::new(DisclosureGroupDisplay {
            id: "d".into(),
            label: "Hello".into(),
            a11y_label: "Toggle Hello section".into(),
        })
        .collapsed(true)
        .on_toggle(|_, _, _| {});
        assert!(g.collapsed);
        assert!(g.on_toggle.is_some());
    }

    #[test]
    fn indicator_owns_display_glyph() {
        let indicator = DisclosureIndicator::new(DisclosureIndicatorDisplay {
            glyph: SharedString::from("\u{25B6}"),
        });

        assert_eq!(indicator.glyph, SharedString::from("\u{25B6}"));
    }

    #[test]
    fn supplement_label_owns_display_text() {
        let label = DisclosureSupplementLabel::new(DisclosureSupplementDisplay {
            label: SharedString::from("(2 albums)"),
        });

        assert_eq!(label.label, SharedString::from("(2 albums)"));
    }

    #[test]
    fn text_panel_owns_display_and_defaults_without_handler() {
        let panel = DisclosureTextPanel::new(DisclosureTextPanelDisplay {
            id: "description".into(),
            label: "Description".into(),
            a11y_label: "Toggle Description section".into(),
            body: "Long form text".into(),
            collapsed: true,
        });

        assert_eq!(panel.display.label, SharedString::from("Description"));
        assert_eq!(panel.display.body, SharedString::from("Long form text"));
        assert!(panel.display.collapsed);
        assert!(panel.on_toggle.is_none());
    }

    #[test]
    fn text_panel_sets_toggle_handler() {
        let panel = DisclosureTextPanel::new(DisclosureTextPanelDisplay {
            id: "description".into(),
            label: "Description".into(),
            a11y_label: "Toggle Description section".into(),
            body: "Long form text".into(),
            collapsed: false,
        })
        .on_toggle(|_, _, _| {});

        assert!(panel.on_toggle.is_some());
    }
}
