#![warn(clippy::pedantic)]
//! Generic list row layout — a horizontal stack with HIG-conformant padding,
//! gap, and corner radius. Callers fill it with arbitrary children
//! (thumbnail, title, trailing actions). All dimensions scale.
//!
//! Accessibility note (ADR 0038 task 005): clickable rows can carry a
//! VM-sourced `a11y_label`. GPUI 0.2.x does not expose a row-level label sink
//! for `div`; the field is retained as contract data until the framework can
//! consume it.

use std::rc::Rc;

use gpui::{
    div, prelude::*, App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, Styled, Window,
};

use crate::ui::primitives::HStack;
use crate::ui::tokens::{color, Radius, SemanticColor, Spacing};

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListRowA11yLabel {
    pub label: gpui::SharedString,
}

/// Vertical density for a [`ListRow`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ListRowDensity {
    /// Tight rows for dense lists (e.g., track tables).
    Compact,
    /// Default — comfortable for most lists.
    #[default]
    Comfortable,
}

impl ListRowDensity {
    fn padding(self) -> Spacing {
        match self {
            Self::Compact => Spacing::XS,
            Self::Comfortable => Spacing::SM,
        }
    }

    fn gap(self) -> Spacing {
        match self {
            Self::Compact => Spacing::SM,
            Self::Comfortable => Spacing::MD,
        }
    }
}

/// One row in a list. Wraps an [`HStack`] with padding, gap, and rounding.
#[derive(IntoElement)]
#[must_use]
pub struct ListRow {
    id: ElementId,
    density: ListRowDensity,
    selected: bool,
    focused: bool,
    a11y_label: Option<gpui::SharedString>,
    on_click: Option<ClickHandler>,
    children: Vec<gpui::AnyElement>,
}

impl ListRow {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            density: ListRowDensity::default(),
            selected: false,
            focused: false,
            a11y_label: None,
            on_click: None,
            children: Vec::new(),
        }
    }

    pub fn compact(id: impl Into<ElementId>) -> Self {
        Self::new(id).density(ListRowDensity::Compact)
    }

    pub fn density(mut self, density: ListRowDensity) -> Self {
        self.density = density;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn a11y_label(mut self, label: ListRowA11yLabel) -> Self {
        self.a11y_label = Some(label.label);
        self
    }

    pub fn on_click<F>(mut self, handler: F) -> Self
    where
        F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for ListRow {
    fn extend(&mut self, elements: impl IntoIterator<Item = gpui::AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for ListRow {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let pad_x = self.density.padding().scaled(cx);
        let pad_y = self.density.padding().scaled(cx);
        let radius = Radius::SM.scaled(cx);
        let stack = HStack::new().spacing(self.density.gap()).center();
        let selected = self.selected;
        let focused = self.focused;
        let on_click = self.on_click.clone();
        let selected_bg = color(cx, SemanticColor::SelectedContent);
        let accent = color(cx, SemanticColor::Accent);
        let focus = color(cx, SemanticColor::Focus);
        let hover_bg = color(cx, SemanticColor::SecondarySystemBackground);

        let mut row = div()
            .id(self.id)
            .px(pad_x)
            .py(pad_y)
            .rounded(radius)
            .border_1()
            .border_color(gpui::transparent_black())
            .when(selected, |el| el.bg(selected_bg).border_color(accent))
            .when(selected && focused, |el| el.border_2().border_color(focus))
            .child(stack.children(self.children));

        if let Some(handler) = on_click {
            row = row
                .cursor_pointer()
                .hover(move |el| el.bg(hover_bg))
                .on_click(move |event, window, cx| {
                    handler(event, window, cx);
                });
        }

        row
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clickable_row_carries_accessibility_label() {
        let row = ListRow::new("result:1").a11y_label(ListRowA11yLabel {
            label: "Open search result".into(),
        });

        assert_eq!(
            row.a11y_label,
            Some(gpui::SharedString::from("Open search result"))
        );
    }
}
