#![warn(clippy::pedantic)]
//! Generic list row layout — a horizontal stack with HIG-conformant padding,
//! gap, and corner radius. Callers fill it with arbitrary children
//! (thumbnail, title, trailing actions). All dimensions scale.

use gpui::{
    div, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled, Window,
};

use crate::ui::primitives::HStack;
use crate::ui::tokens::{Radius, Spacing};

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
    children: Vec<gpui::AnyElement>,
}

impl ListRow {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            density: ListRowDensity::default(),
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
        div()
            .id(self.id)
            .px(pad_x)
            .py(pad_y)
            .rounded(radius)
            .child(stack.children(self.children))
    }
}
