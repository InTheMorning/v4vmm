//! Shared resizable split-pane shell.
//!
//! Screens provide the pane content and GPUI event adapters. This composite
//! owns the structural layout and resize-handle styling so Library and
//! Discover cannot drift into different shells.

#![warn(clippy::pedantic)]

use gpui::{
    div, prelude::*, AnyElement, App, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, ParentElement, RenderOnce, SharedString, Styled, Window,
};

use crate::ui::theme::{color, layout};

type MouseMoveHandler = Box<dyn Fn(&MouseMoveEvent, &mut Window, &mut App) + 'static>;
type MouseUpHandler = Box<dyn Fn(&MouseUpEvent, &mut Window, &mut App) + 'static>;
type MouseDownHandler = Box<dyn Fn(&MouseDownEvent, &mut Window, &mut App) + 'static>;

/// Two-pane shell with a shared resize handle.
#[derive(IntoElement)]
#[must_use]
pub struct SplitPane {
    id: SharedString,
    handle_id: SharedString,
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
            leading_width: layout::INSPECTOR_WIDTH,
            leading_min_width: layout::INSPECTOR_MIN_WIDTH,
            leading: None,
            trailing: None,
            on_resize_move: None,
            on_resize_end: None,
            on_resize_start: None,
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
        let leading = self.leading.unwrap_or_else(|| div().into_any_element());
        let trailing = self.trailing.unwrap_or_else(|| div().into_any_element());

        let mut handle = div()
            .id(self.handle_id)
            .w(layout::SPLIT_HANDLE_WIDTH)
            .cursor_col_resize()
            .bg(color::border_subtle())
            .hover(|s| s.bg(color::accent()))
            .flex_shrink_0();
        if let Some(on_resize_start) = self.on_resize_start {
            handle = handle.on_mouse_down(MouseButton::Left, move |event, window, cx| {
                on_resize_start(event, window, cx);
            });
        }

        let mut root = div()
            .id(self.id)
            .flex()
            .flex_row()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .child(
                div()
                    .w(self.leading_width)
                    .min_w(self.leading_min_width)
                    .flex_shrink_0()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(leading),
            )
            .child(handle)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(trailing),
            );

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
