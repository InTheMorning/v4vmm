//! Shared resizable split-pane shell (ADRs 0046 and 0063).
//!
//! Screens provide the pane content and GPUI event adapters. This composite
//! owns the structural layout and resize-handle styling so Library and
//! Discover cannot drift into different shells.

#![warn(clippy::pedantic)]

use gpui::{
    canvas, div, prelude::*, AnyElement, App, IntoElement, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ParentElement, RenderOnce, SharedString, Styled, Window,
};

use crate::ui::layouts as layout;
use crate::ui::style::color;

type LayoutHandler = Box<dyn Fn(gpui::Bounds<gpui::Pixels>, &mut Window, &mut App) + 'static>;

type MouseMoveHandler = Box<dyn Fn(&MouseMoveEvent, &mut Window, &mut App) + 'static>;
type MouseUpHandler = Box<dyn Fn(&MouseUpEvent, &mut Window, &mut App) + 'static>;
type MouseDownHandler = Box<dyn Fn(&MouseDownEvent, &mut Window, &mut App) + 'static>;

/// Direction of the shared split; existing callers default to left and right.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SplitPaneAxis {
    #[default]
    Horizontal,
    Vertical,
}

/// Two-pane shell with a shared resize handle.
#[derive(IntoElement)]
#[must_use]
pub struct SplitPane {
    id: SharedString,
    handle_id: SharedString,
    axis: SplitPaneAxis,
    leading_height: gpui::Pixels,
    leading_min_height: gpui::Pixels,
    on_layout: Option<LayoutHandler>,
    leading_width: gpui::Pixels,
    leading_min_width: gpui::Pixels,
    leading: Option<AnyElement>,
    trailing: Option<AnyElement>,
    on_resize_move: Option<MouseMoveHandler>,
    on_resize_end: Option<MouseUpHandler>,
    on_resize_start: Option<MouseDownHandler>,
}

impl SplitPane {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            handle_id: SharedString::from("split-pane-resize-handle"),
            axis: SplitPaneAxis::Horizontal,
            leading_height: gpui::Pixels::ZERO,
            leading_min_height: gpui::Pixels::ZERO,
            on_layout: None,
            leading_width: layout::INSPECTOR_WIDTH,
            leading_min_width: layout::INSPECTOR_MIN_WIDTH,
            leading: None,
            trailing: None,
            on_resize_move: None,
            on_resize_end: None,
            on_resize_start: None,
        }
    }

    /// Selects left/right or top/bottom layout without changing existing width callers.
    pub fn axis(mut self, axis: SplitPaneAxis) -> Self {
        self.axis = axis;
        self
    }

    pub fn leading_height(mut self, height: gpui::Pixels) -> Self {
        self.leading_height = height;
        self
    }

    pub fn leading_min_height(mut self, height: gpui::Pixels) -> Self {
        self.leading_min_height = height;
        self
    }

    /// Reports allocated geometry after layout; adapters decide whether state changed.
    pub fn on_layout(
        mut self,
        handler: impl Fn(gpui::Bounds<gpui::Pixels>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_layout = Some(Box::new(handler));
        self
    }

    fn handle_geometry(&self) -> gpui::Div {
        match self.axis {
            SplitPaneAxis::Horizontal => div().w(layout::SPLIT_HANDLE_WIDTH).cursor_col_resize(),
            SplitPaneAxis::Vertical => div().h(layout::SPLIT_HANDLE_WIDTH).cursor_row_resize(),
        }
    }

    fn pane_containers(&self) -> (gpui::Div, gpui::Div, gpui::Div) {
        let root = div().flex().flex_1().min_h_0().overflow_hidden();
        let leading = div().flex_shrink_0().flex().flex_col().overflow_hidden();
        let trailing = div().flex_1().min_w_0().flex().flex_col().overflow_hidden();
        match self.axis {
            SplitPaneAxis::Horizontal => (
                root.flex_row(),
                leading.w(self.leading_width).min_w(self.leading_min_width),
                trailing,
            ),
            SplitPaneAxis::Vertical => (
                root.flex_col().min_w_0(),
                leading
                    .h(self.leading_height)
                    .min_h(self.leading_min_height)
                    .min_w_0(),
                trailing.min_h_0(),
            ),
        }
    }

    pub fn resize_handle_id(mut self, id: impl Into<SharedString>) -> Self {
        self.handle_id = id.into();
        self
    }

    pub fn leading_width(mut self, width: gpui::Pixels) -> Self {
        self.leading_width = width;
        self
    }

    pub fn leading_min_width(mut self, width: gpui::Pixels) -> Self {
        self.leading_min_width = width;
        self
    }

    pub fn leading(mut self, content: AnyElement) -> Self {
        self.leading = Some(content);
        self
    }

    pub fn trailing(mut self, content: AnyElement) -> Self {
        self.trailing = Some(content);
        self
    }

    pub fn on_resize_move<F>(mut self, handler: F) -> Self
    where
        F: Fn(&MouseMoveEvent, &mut Window, &mut App) + 'static,
    {
        self.on_resize_move = Some(Box::new(handler));
        self
    }

    pub fn on_resize_end<F>(mut self, handler: F) -> Self
    where
        F: Fn(&MouseUpEvent, &mut Window, &mut App) + 'static,
    {
        self.on_resize_end = Some(Box::new(handler));
        self
    }

    pub fn on_resize_start<F>(mut self, handler: F) -> Self
    where
        F: Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    {
        self.on_resize_start = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for SplitPane {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let (root, leading_pane, trailing_pane) = self.pane_containers();
        let handle = self.handle_geometry();
        let leading = self.leading.unwrap_or_else(|| div().into_any_element());
        let trailing = self.trailing.unwrap_or_else(|| div().into_any_element());

        let mut handle = handle
            .id(self.handle_id)
            .bg(color::border_subtle())
            .hover(|s| s.bg(color::accent()))
            .flex_shrink_0();
        if let Some(on_resize_start) = self.on_resize_start {
            handle = handle.on_mouse_down(MouseButton::Left, move |event, window, cx| {
                on_resize_start(event, window, cx);
            });
        }

        let mut root = root
            .id(self.id)
            .child(leading_pane.child(leading))
            .child(handle)
            .child(trailing_pane.child(trailing));
        if let Some(on_layout) = self.on_layout {
            root = root.child(
                canvas(
                    move |bounds, window, cx| on_layout(bounds, window, cx),
                    |_, (), _, _| {},
                )
                .absolute()
                .size_full(),
            );
        }

        if let Some(on_resize_move) = self.on_resize_move {
            root = root.on_mouse_move(move |event, window, cx| {
                on_resize_move(event, window, cx);
            });
        }

        if let Some(on_resize_end) = self.on_resize_end {
            root = root.on_mouse_up(MouseButton::Left, move |event, window, cx| {
                on_resize_end(event, window, cx);
            });
        }

        root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Situational ADR 0063: adding height resizing preserves existing width callers.
    #[test]
    fn split_pane_width_defaults_and_overrides_are_preserved() {
        let default = SplitPane::new("inspector");
        assert_eq!(default.axis, SplitPaneAxis::Horizontal);
        assert_eq!(default.leading_width, layout::INSPECTOR_WIDTH);
        assert_eq!(default.leading_min_width, layout::INSPECTOR_MIN_WIDTH);

        let workspace = SplitPane::new("workspace")
            .leading_width(layout::CONTENT_PANE_DEFAULT_WIDTH)
            .leading_min_width(layout::CONTENT_PANE_MIN_WIDTH);
        assert_eq!(workspace.leading_width, layout::CONTENT_PANE_DEFAULT_WIDTH);
        assert_eq!(workspace.leading_min_width, layout::CONTENT_PANE_MIN_WIDTH);
    }

    /// Situational ADR 0063: width render geometry remains the inspector/workspace geometry.
    #[test]
    fn split_pane_width_render_geometry_is_unchanged() {
        let pane = SplitPane::new("workspace")
            .leading_width(layout::CONTENT_PANE_DEFAULT_WIDTH)
            .leading_min_width(layout::CONTENT_PANE_MIN_WIDTH);
        let (mut root, mut leading, mut trailing) = pane.pane_containers();
        let mut handle = pane.handle_geometry();
        assert_eq!(root.style().flex_direction, Some(gpui::FlexDirection::Row));
        assert_eq!(
            leading.style().size.width,
            Some(layout::CONTENT_PANE_DEFAULT_WIDTH.into())
        );
        assert_eq!(
            leading.style().min_size.width,
            Some(layout::CONTENT_PANE_MIN_WIDTH.into())
        );
        assert_eq!(leading.style().flex_shrink, Some(0.0));
        assert_eq!(trailing.style().flex_grow, Some(1.0));
        assert_eq!(
            trailing.style().min_size.width,
            Some(gpui::Pixels::ZERO.into())
        );
        assert_eq!(
            handle.style().size.width,
            Some(layout::SPLIT_HANDLE_WIDTH.into())
        );
        assert_eq!(
            handle.style().mouse_cursor,
            Some(gpui::CursorStyle::ResizeColumn)
        );
    }

    /// Situational ADR 0063: the shared handle and containers support top/bottom sizing.
    #[test]
    fn split_pane_height_render_geometry_uses_the_vertical_axis() {
        let pane = SplitPane::new("show")
            .axis(SplitPaneAxis::Vertical)
            .leading_height(layout::WINDOW_HEIGHT)
            .leading_min_height(layout::TAB_BAR_HEIGHT);
        let (mut root, mut leading, mut trailing) = pane.pane_containers();
        let mut handle = pane.handle_geometry();
        assert_eq!(
            root.style().flex_direction,
            Some(gpui::FlexDirection::Column)
        );
        assert_eq!(
            leading.style().size.height,
            Some(layout::WINDOW_HEIGHT.into())
        );
        assert_eq!(
            leading.style().min_size.height,
            Some(layout::TAB_BAR_HEIGHT.into())
        );
        assert_eq!(
            trailing.style().min_size.height,
            Some(gpui::Pixels::ZERO.into())
        );
        assert_eq!(
            handle.style().size.height,
            Some(layout::SPLIT_HANDLE_WIDTH.into())
        );
        assert_eq!(
            handle.style().mouse_cursor,
            Some(gpui::CursorStyle::ResizeRow)
        );
    }
}
