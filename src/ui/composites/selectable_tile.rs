//! A content tile with the selection facts of ADR 0084 Decision 10
//! (packet 020).
//!
//! In selection mode, the checkbox shows at the top-left corner of the
//! cover, on a circle backing, so that it reads on each cover. A selected
//! tile gets the `Accent` outline and the `SelectedContent` fill. A focused
//! tile gets the `Focus` outline. The outline is a layer over the tile, so
//! the tile keeps its size in each state. The rule of the outline is
//! `row_outline` of `list_row.rs`.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    div, prelude::*, AnyElement, App, ClickEvent, ElementId, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, Styled, Window,
};

use crate::ui::composites::list_row::{row_outline, RowOutline};
use crate::ui::tokens::{color, Radius, SemanticColor, Size, Spacing};

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// The size of the checkbox backing. It holds the `Checkbox` hit square.
const CHECKBOX_BACKING: Size = Size::PointerHitTarget;

/// The inset of the checkbox backing from the corner of the cover.
const CHECKBOX_INSET: Spacing = Spacing::XS;

/// One content tile: a cover, the text below it, and the selection facts.
/// The caller gives the cover, the `Checkbox` and the text. The tile owns
/// the chrome, the checkbox place, the outline and the fill.
#[derive(IntoElement)]
#[must_use]
pub(crate) struct SelectableTile {
    id: ElementId,
    artwork: AnyElement,
    checkbox: Option<AnyElement>,
    selected: bool,
    focused: bool,
    scroll_probe: Option<AnyElement>,
    on_click: Option<ClickHandler>,
    children: Vec<AnyElement>,
}

impl SelectableTile {
    pub(crate) fn new(id: impl Into<ElementId>, artwork: AnyElement) -> Self {
        Self {
            id: id.into(),
            artwork,
            checkbox: None,
            selected: false,
            focused: false,
            scroll_probe: None,
            on_click: None,
            children: Vec::new(),
        }
    }

    /// The selection checkbox. A caller gives one only in selection mode.
    /// `None` shows no checkbox.
    pub(crate) fn checkbox(mut self, checkbox: Option<AnyElement>) -> Self {
        self.checkbox = checkbox;
        self
    }

    pub(crate) fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub(crate) fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// The scroll probe of the focused tile (packet 018). It paints
    /// nothing.
    pub(crate) fn scroll_probe(mut self, probe: Option<AnyElement>) -> Self {
        self.scroll_probe = probe;
        self
    }

    pub(crate) fn on_click<F>(mut self, handler: F) -> Self
    where
        F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for SelectableTile {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for SelectableTile {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let radius = Radius::MD.scaled(cx);
        let hover_bg = color(cx, SemanticColor::SecondarySystemBackground);
        let outline = match row_outline(self.selected, self.focused) {
            RowOutline::None => None,
            RowOutline::Selected => Some(
                div()
                    .border_1()
                    .border_color(color(cx, SemanticColor::Accent)),
            ),
            RowOutline::Focus => Some(
                div()
                    .border_2()
                    .border_color(color(cx, SemanticColor::Focus)),
            ),
        };
        let checkbox = self.checkbox.map(|checkbox| {
            div()
                .absolute()
                .top(CHECKBOX_INSET.scaled(cx))
                .left(CHECKBOX_INSET.scaled(cx))
                .size(CHECKBOX_BACKING.scaled(cx))
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(color(cx, SemanticColor::SystemBackground))
                .border_1()
                .border_color(color(cx, SemanticColor::Separator))
                .child(checkbox)
        });

        let mut tile = div()
            .id(self.id)
            .relative()
            .flex()
            .flex_col()
            .gap(Spacing::SM.scaled(cx))
            .w(Size::ContentTileWidth.scaled(cx))
            .p(Spacing::SM.scaled(cx))
            .rounded(radius)
            .when(self.selected, |el| {
                el.bg(color(cx, SemanticColor::SelectedContent))
            })
            .child(
                div()
                    .relative()
                    .flex_shrink_0()
                    .child(self.artwork)
                    .children(checkbox),
            )
            .children(self.children)
            .children(outline.map(|outline| {
                outline
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .rounded(radius)
            }))
            .children(self.scroll_probe);

        if let Some(handler) = self.on_click {
            tile = tile
                .cursor_pointer()
                .hover(move |el| el.bg(hover_bg))
                .on_click(move |event, window, cx| handler(event, window, cx));
        }

        tile
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R20-07: the checkbox backing is the pointer hit square of the
    /// `Checkbox` primitive, inset by the named spacing token.
    #[test]
    fn adr_0084_discover_selection_tile_checkbox_backing_uses_named_tokens() {
        assert_eq!(CHECKBOX_BACKING, Size::PointerHitTarget);
        assert_eq!(CHECKBOX_INSET, Spacing::XS);
    }
}
