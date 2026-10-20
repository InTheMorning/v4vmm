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
use std::sync::Arc;

use gpui::{
    div, prelude::*, App, ClickEvent, ElementId, InteractiveElement, IntoElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window,
};

use crate::ui::control_styles::ControlStyle;
use crate::ui::icons::{Icon, IconName, IconSize};
use crate::ui::primitives::{Button, Label, MultilineText, SectionHeader};
use crate::ui::tokens::{FontSize, Radius, SemanticColor, Size, Spacing};

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// The chevron has the icon size of the checkbox mark, so the two leading
/// controls of a row match. The operator found on 2026-10-19 that the
/// smaller `Transport` chevron in `TertiaryLabel` was hard to see and hard
/// to hit.
const DISCLOSURE_ICON_SIZE: IconSize = IconSize::Action;
/// `SecondaryLabel` meets 4.5:1 on each surface (`src/ui/contrast.rs`),
/// more than the 3:1 that a graphic needs.
const DISCLOSURE_ICON_COLOR: SemanticColor = SemanticColor::SecondaryLabel;
/// The box of the chevron is the box of the checkbox.
const DISCLOSURE_INDICATOR_BOX: Size = Size::CheckboxIndicator;
/// The leading disclosure column is the pointer hit target, the same as
/// the hit square of the row checkbox.
const DISCLOSURE_COLUMN_SIDE: Size = Size::PointerHitTarget;

#[derive(IntoElement)]
#[must_use]
pub struct DisclosureGroup {
    id: ElementId,
    label: SharedString,
    a11y_label: SharedString,
    collapsed: bool,
    on_toggle: Option<ClickHandler>,
}

/// Fixed-width disclosure chevron used beside tree and sidebar labels.
///
/// ADR 0083 Decision 10: draws a Lucide chevron icon, not a character. The
/// icon turns from pointing right to pointing down when the row expands.
#[derive(IntoElement)]
#[must_use]
pub struct DisclosureIndicator {
    expanded: bool,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisclosureIndicatorDisplay {
    /// ADR 0083 Decision 10: a typed disclosure state, not a character.
    pub expanded: bool,
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
    /// The text to show. `None` shows no body (ADR 0047 item 8, amended
    /// 2026-10-16: a short, collapsed description shows no body).
    pub body: Option<SharedString>,
    pub collapsed: bool,
    /// "Show more" label and accessibility label, at the end of a
    /// preview. `None` when the full text shows, or when the description
    /// is short (ADR 0047 item 8, amended 2026-10-16).
    pub show_more: Option<(SharedString, SharedString)>,
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
            expanded: display.expanded,
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
        let icon_name = if self.expanded {
            IconName::ChevronDown
        } else {
            IconName::ChevronRight
        };
        let box_size = DISCLOSURE_INDICATOR_BOX.scaled(cx);
        div()
            .flex_none()
            .w(box_size)
            .h(box_size)
            .flex()
            .items_center()
            .justify_center()
            .child(
                Icon::new(icon_name)
                    .size(DISCLOSURE_ICON_SIZE)
                    .color(crate::ui::tokens::color(cx, DISCLOSURE_ICON_COLOR)),
            )
    }
}

/// The fixed leading disclosure column of a list row (ADR 0084 Decision 6).
///
/// The column is a square of the pointer hit target, the same as the hit
/// square of the row checkbox. A row that can open puts its
/// [`DisclosureIndicator`] in the column. A row that cannot open keeps the
/// empty column, so each cover in the list aligns.
#[must_use]
pub fn disclosure_column(cx: &App) -> gpui::Div {
    let side = DISCLOSURE_COLUMN_SIDE.scaled(cx);
    div()
        .flex_none()
        .w(side)
        .h(side)
        .flex()
        .items_center()
        .justify_center()
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
        let panel_id = self.display.id.clone();
        let mut header = DisclosureGroup::new(DisclosureGroupDisplay {
            id: self.display.id,
            label: self.display.label,
            a11y_label: self.display.a11y_label,
        })
        .collapsed(collapsed);
        if let Some(handler) = self.on_toggle.clone() {
            header = header.on_toggle(move |event, window, cx| handler(event, window, cx));
        }

        let mut panel = div()
            .border_1()
            .border_color(crate::ui::tokens::color(cx, SemanticColor::Separator))
            .rounded(Radius::MD.scaled(cx))
            .p(Spacing::SM.scaled(cx))
            .child(header);

        if let Some(body) = self.display.body {
            panel = panel.child(
                div().mt(Spacing::XS.scaled(cx)).child(
                    MultilineText::new(body)
                        .max_lines(usize::MAX)
                        .wrap_lines()
                        .size(FontSize::Micro)
                        .color(SemanticColor::Label),
                ),
            );
        }

        if let Some((label, a11y_label)) = self.display.show_more {
            let show_more_id = ElementId::NamedChild(Arc::new(panel_id), "show-more".into());
            let mut show_more = Button::styled(show_more_id, ControlStyle::Ghost)
                .label(label)
                .a11y_label(a11y_label);
            if let Some(handler) = self.on_toggle {
                show_more = show_more.on_click(move |event, window, cx| handler(event, window, cx));
            }
            panel = panel.child(div().mt(Spacing::XS.scaled(cx)).child(show_more));
        }

        panel
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
    fn indicator_owns_display_expanded_state() {
        let collapsed = DisclosureIndicator::new(DisclosureIndicatorDisplay { expanded: false });
        assert!(!collapsed.expanded);

        let expanded = DisclosureIndicator::new(DisclosureIndicatorDisplay { expanded: true });
        assert!(expanded.expanded);
    }

    #[test]
    fn indicator_matches_the_checkbox_tokens() {
        // ADR 0084 Decision 6: the chevron and the checkbox are the two
        // leading controls of a Releases row. They have the same icon
        // size, the same box and the same hit target.
        assert_eq!(DISCLOSURE_ICON_SIZE, IconSize::Action);
        assert_eq!(DISCLOSURE_ICON_COLOR, SemanticColor::SecondaryLabel);
        assert_eq!(DISCLOSURE_INDICATOR_BOX, Size::CheckboxIndicator);
        assert_eq!(DISCLOSURE_COLUMN_SIDE, Size::PointerHitTarget);
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
            body: Some("Long form text".into()),
            collapsed: true,
            show_more: None,
        });

        assert_eq!(panel.display.label, SharedString::from("Description"));
        assert_eq!(
            panel.display.body,
            Some(SharedString::from("Long form text"))
        );
        assert!(panel.display.collapsed);
        assert!(panel.display.show_more.is_none());
        assert!(panel.on_toggle.is_none());
    }

    #[test]
    fn text_panel_owns_its_show_more_pair() {
        let panel = DisclosureTextPanel::new(DisclosureTextPanelDisplay {
            id: "description".into(),
            label: "Description".into(),
            a11y_label: "Toggle Description section".into(),
            body: Some("A preview of a long text…".into()),
            collapsed: true,
            show_more: Some(("Show more".into(), "Show the full description".into())),
        });

        assert_eq!(
            panel.display.show_more,
            Some((
                SharedString::from("Show more"),
                SharedString::from("Show the full description")
            ))
        );
    }

    #[test]
    fn text_panel_sets_toggle_handler() {
        let panel = DisclosureTextPanel::new(DisclosureTextPanelDisplay {
            id: "description".into(),
            label: "Description".into(),
            a11y_label: "Toggle Description section".into(),
            body: Some("Long form text".into()),
            collapsed: false,
            show_more: None,
        })
        .on_toggle(|_, _, _| {});

        assert!(panel.on_toggle.is_some());
    }
}
