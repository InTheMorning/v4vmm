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
use crate::ui::tokens::{color, Radius, SemanticColor, Size, Spacing};

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// The hover group tag of a [`ListRow`] (ADR 0084 Decision 10). Every row
/// opens its own scope with `.group(...)`, so one literal tag is safe:
/// a `.group_hover(...)` lookup finds the nearest ancestor with the tag,
/// never a sibling row's.
const ROW_HOVER_GROUP: &str = "list-row-hover";

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
    /// The leading selection checkbox (ADR 0084 Decision 10). `None` hides
    /// it: a list with no selection model never reserves its space.
    checkbox: Option<gpui::AnyElement>,
    /// The checkbox shows even with the pointer away from the row (ADR
    /// 0084 Decision 10 Design: "The row" — a selection exists, or the
    /// row carries the keyboard focus). `false` shows the checkbox only
    /// while the pointer is on the row, the screen's own hover state.
    checkbox_force_visible: bool,
    /// The audition control position of a track row (ADR 0084 Decision
    /// 11). `None` shows no control and reserves no space, until
    /// [ADR 0068](../../../docs/adr/0068-show-cue-and-audition-isolation.md)
    /// fills it.
    audition: Option<gpui::AnyElement>,
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
            checkbox: None,
            checkbox_force_visible: false,
            audition: None,
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

    /// The leading selection checkbox (ADR 0084 Decision 10). A caller
    /// with no selection model for this row never calls this, so the row
    /// reserves no checkbox space. `force_visible` keeps the checkbox
    /// showing with the pointer away from the row; otherwise the row
    /// shows it only on hover.
    pub fn checkbox(mut self, checkbox: gpui::AnyElement, force_visible: bool) -> Self {
        self.checkbox = Some(checkbox);
        self.checkbox_force_visible = force_visible;
        self
    }

    /// The audition control position of a track row (ADR 0084 Decision
    /// 11). No packet fills this yet, so no caller passes one.
    pub fn audition(mut self, audition: gpui::AnyElement) -> Self {
        self.audition = Some(audition);
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
        let has_click_action = on_click.is_some();

        // ADR 0084 Decision 10 Design: "The row" — the checkbox shows
        // while a selection exists or the row has the keyboard focus
        // (`checkbox_force_visible`), and also while the pointer is on
        // the row. `group_hover` reveals it for that last case; it finds
        // this row's own `.group(ROW_HOVER_GROUP)`, never a sibling's.
        let checkbox = self.checkbox.map(|checkbox| {
            if self.checkbox_force_visible {
                checkbox
            } else {
                div()
                    .invisible()
                    .group_hover(ROW_HOVER_GROUP, gpui::Styled::visible)
                    .child(checkbox)
                    .into_any_element()
            }
        });

        // ADR 0084 Decision 1: a row in a scrolling list keeps its content
        // height. A list column has a fixed height, and a flex child shrinks
        // by default, so a row without this overlaps the next row.
        let mut row = div()
            .id(self.id)
            .group(ROW_HOVER_GROUP)
            .flex_shrink_0()
            .px(pad_x)
            .py(pad_y)
            .when(has_click_action, |el| {
                el.min_h(Size::MinHitTarget.scaled(cx))
            })
            .rounded(radius)
            .border_1()
            .border_color(gpui::transparent_black())
            .when(selected, |el| el.bg(selected_bg))
            .map(|el| match row_outline(selected, focused) {
                RowOutline::None => el,
                RowOutline::Selected => el.border_color(accent),
                RowOutline::Focus => el.border_2().border_color(focus),
            })
            .child(
                stack.children(
                    // ADR 0084 Decision 10: the checkbox leads the row.
                    // Decision 11: the audition slot trails it. Each is
                    // `None` until its owning decision fills it, so neither
                    // reserves space here.
                    checkbox
                        .into_iter()
                        .chain(self.children)
                        .chain(self.audition),
                ),
            );

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

/// The outline of a list row (ADR 0084 Decision 10).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RowOutline {
    None,
    Selected,
    Focus,
}

/// The keyboard focus outline wins, so each focused row shows it, selected
/// or not. The operator found on 2026-10-19 that the focus showed only on a
/// selected row.
const fn row_outline(selected: bool, focused: bool) -> RowOutline {
    match (selected, focused) {
        (_, true) => RowOutline::Focus,
        (true, false) => RowOutline::Selected,
        (false, false) => RowOutline::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adr_0084_selection_focus_outline_shows_on_each_focused_row() {
        assert_eq!(row_outline(false, true), RowOutline::Focus);
        assert_eq!(row_outline(true, true), RowOutline::Focus);
        assert_eq!(row_outline(true, false), RowOutline::Selected);
        assert_eq!(row_outline(false, false), RowOutline::None);
    }

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
