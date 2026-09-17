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

type ResizeHandler = Box<dyn Fn(&SplitPaneResize, &mut Window, &mut App) + 'static>;

/// A clamped pane extent, measured from the split viewport origin (ADR 0046).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum SplitPaneResize {
    Width(gpui::Pixels),
    Height(gpui::Pixels),
}

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
    fit_bounds: Option<gpui::Bounds<gpui::Pixels>>,
    preferred_stacked_height: Option<gpui::Pixels>,
    resize_min: gpui::Pixels,
    resize_max: gpui::Pixels,
    on_resize: Option<ResizeHandler>,
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
            fit_bounds: None,
            preferred_stacked_height: None,
            resize_min: gpui::Pixels::ZERO,
            resize_max: gpui::Pixels::ZERO,
            on_resize: None,
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

    /// Retains a separate preferred height for the stacked layout.
    pub(crate) fn stacked_height(mut self, height: Option<gpui::Pixels>) -> Self {
        self.preferred_stacked_height = height;
        self
    }

    /// Fits both panes without changing their preferred horizontal or vertical extents.
    pub(crate) fn fit_to(
        mut self,
        bounds: Option<gpui::Bounds<gpui::Pixels>>,
        trailing_min_width: gpui::Pixels,
        stacked_min_height: gpui::Pixels,
    ) -> Self {
        let Some(bounds) = bounds else { return self };
        self.fit_bounds = Some(bounds);
        let available_width =
            (bounds.size.width - layout::SPLIT_HANDLE_WIDTH).max(gpui::Pixels::ZERO);
        if available_width >= self.leading_min_width + trailing_min_width {
            self.resize_min = self.leading_min_width;
            self.resize_max = available_width - trailing_min_width;
            self.leading_width = self.leading_width.clamp(self.resize_min, self.resize_max);
        } else {
            self.axis = SplitPaneAxis::Vertical;
            let available_height =
                (bounds.size.height - layout::SPLIT_HANDLE_WIDTH).max(gpui::Pixels::ZERO);
            self.resize_min = stacked_min_height.min(available_height / 2.0);
            self.resize_max = available_height - self.resize_min;
            self.leading_height = self
                .preferred_stacked_height
                .unwrap_or(available_height * layout::SPLIT_STACKED_LEADING_FRACTION)
                .clamp(self.resize_min, self.resize_max);
            self.leading_min_height = gpui::Pixels::ZERO;
        }
        self
    }

    /// Reports clamped resize intent; the caller retains the extent in its view model.
    pub(crate) fn on_resize(
        mut self,
        handler: impl Fn(&SplitPaneResize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_resize = Some(Box::new(handler));
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
        let root = div()
            .relative()
            .flex()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .overflow_hidden();
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
        #[cfg(test)]
        {
            handle = handle.debug_selector(|| "split-resize-handle".to_owned());
        }
        if let Some(on_resize_start) = self.on_resize_start {
            handle = handle.on_mouse_down(MouseButton::Left, move |event, window, cx| {
                on_resize_start(event, window, cx);
                // ADR 0071: Root must not start text selection from a divider drag.
                cx.stop_propagation();
            });
        }

        #[cfg(test)]
        let leading_pane = leading_pane.debug_selector(|| "split-leading-pane".to_owned());
        #[cfg(test)]
        let trailing_pane = trailing_pane.debug_selector(|| "split-trailing-pane".to_owned());

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
                .top_0()
                .left_0()
                .size_full(),
            );
        }

        if let (Some(on_resize), Some(bounds)) = (self.on_resize, self.fit_bounds) {
            let axis = self.axis;
            let (min, max) = (self.resize_min, self.resize_max);
            root = root.on_mouse_move(move |event, window, cx| {
                let extent = match axis {
                    SplitPaneAxis::Horizontal => {
                        SplitPaneResize::Width((event.position.x - bounds.origin.x).clamp(min, max))
                    }
                    SplitPaneAxis::Vertical => SplitPaneResize::Height(
                        (event.position.y - bounds.origin.y).clamp(min, max),
                    ),
                };
                on_resize(&extent, window, cx);
            });
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
    use gpui::{Context, Entity, Modifiers, Pixels, Render, TestAppContext};

    use super::*;

    struct FitTest {
        size: gpui::Size<Pixels>,
        scale: f32,
        resizing: bool,
        height: Option<Pixels>,
        last_resize: Option<SplitPaneResize>,
    }

    impl Render for FitTest {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .mt(gpui::px(70.))
                .w(self.size.width)
                .h(self.size.height)
                .flex()
                .flex_col()
                .child(
                    SplitPane::new("fit-test")
                        .leading_width(gpui::px(800.))
                        .leading_min_width(layout::INSPECTOR_MIN_WIDTH * self.scale)
                        .stacked_height(self.height)
                        .fit_to(
                            Some(gpui::Bounds::new(
                                gpui::point(gpui::px(0.), gpui::px(70.)),
                                self.size,
                            )),
                            layout::CONTENT_PANE_MIN_WIDTH * self.scale,
                            layout::SPLIT_STACKED_MIN_HEIGHT * self.scale,
                        )
                        .leading(div().child("Library navigation").into_any_element())
                        .trailing(div().child("Startup fixture playlist").into_any_element())
                        .on_resize_start(cx.listener(|this, _, _, _| this.resizing = true))
                        .on_resize(cx.listener(|this, extent, _, cx| {
                            if this.resizing {
                                this.last_resize = Some(*extent);
                                if let SplitPaneResize::Height(height) = *extent {
                                    this.height = Some(height);
                                }
                                cx.notify();
                            }
                        }))
                        .on_resize_end(cx.listener(|this, _, _, _| this.resizing = false)),
                )
        }
    }

    /// Situational ADR 0046: both panes remain reachable across viewport and scale changes.
    #[gpui::test]
    fn adr_0046_split_fits_both_panes_and_restores_preferred_width(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (view, cx) = cx.add_window_view(|_, _| FitTest {
            size: gpui::size(gpui::px(463.), gpui::px(320.)),
            scale: 1.,
            resizing: false,
            height: None,
            last_resize: None,
        });
        for (width, height, scale, stacked) in [
            (463., 320., 1., true),
            (620., 480., 1., false),
            (1200., 640., 1., false),
            (463., 180., 1., true),
            (620., 320., 1.5, true),
            (1200., 480., 1., false),
        ] {
            let size = gpui::size(gpui::px(width), gpui::px(height));
            view.update(cx, |this, cx| {
                this.size = size;
                this.scale = scale;
                this.resizing = false;
                cx.notify();
            });
            cx.update(|window, cx| {
                let _ = window.draw(cx);
            });
            let leading = cx.debug_bounds("split-leading-pane").unwrap();
            let trailing = cx.debug_bounds("split-trailing-pane").unwrap();
            let handle = cx.debug_bounds("split-resize-handle").unwrap();
            if stacked {
                assert_eq!(leading.size.width, size.width);
                assert_eq!(trailing.size.width, size.width);
                assert!(leading.size.height > Pixels::ZERO);
                assert!(trailing.size.height > Pixels::ZERO);
                assert!(trailing.top() >= leading.bottom());
                assert!(trailing.bottom() <= size.height + gpui::px(70.));
            } else {
                assert!(trailing.size.width >= layout::CONTENT_PANE_MIN_WIDTH * scale);
                assert!(trailing.left() >= leading.right());
                assert!(trailing.right() <= size.width);
                assert_eq!(leading.size.height, size.height);
                if width > 1000. {
                    assert_eq!(leading.size.width, gpui::px(800.));
                }
            }
            cx.simulate_mouse_down(handle.center(), MouseButton::Left, Modifiers::default());
            cx.update(|_, cx| assert!(view.read(cx).resizing));
            let destination = handle.center() + gpui::point(gpui::px(25.), gpui::px(35.));
            cx.simulate_mouse_move(destination, MouseButton::Left, Modifiers::default());
            cx.update(|_, cx| {
                let resized = view.read(cx).last_resize.unwrap();
                if stacked {
                    let available = size.height - layout::SPLIT_HANDLE_WIDTH;
                    let min = (layout::SPLIT_STACKED_MIN_HEIGHT * scale).min(available / 2.0);
                    assert_eq!(
                        resized,
                        SplitPaneResize::Height(
                            (destination.y - gpui::px(70.)).clamp(min, available - min)
                        )
                    );
                } else {
                    assert!(matches!(resized, SplitPaneResize::Width(_)));
                }
            });
            cx.simulate_mouse_up(destination, MouseButton::Left, Modifiers::default());
        }
    }

    struct ResizeTest {
        axis: SplitPaneAxis,
        extent: Pixels,
        dragging: bool,
    }

    impl Render for ResizeTest {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let text = "alpha /tmp/café/music.flac ".repeat(40);
            let text = format!("{text}\n").repeat(40);
            SplitPane::new("resize-test")
                .axis(self.axis)
                .leading_width(self.extent)
                .leading_min_width(Pixels::ZERO)
                .leading_height(self.extent)
                .leading_min_height(Pixels::ZERO)
                .leading(
                    crate::ui::composites::SelectableText::new("before", text.clone())
                        .into_any_element(),
                )
                .trailing(
                    crate::ui::composites::SelectableText::new("after", text).into_any_element(),
                )
                .on_resize_start(cx.listener(|this, _, _, _| this.dragging = true))
                .on_resize_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                    if this.dragging && event.dragging() {
                        this.extent = match this.axis {
                            SplitPaneAxis::Horizontal => event.position.x,
                            SplitPaneAxis::Vertical => event.position.y,
                        };
                        cx.notify();
                    }
                }))
                .on_resize_end(cx.listener(|this, _, _, _| this.dragging = false))
        }
    }

    struct ResizeRoot(Entity<ResizeTest>, Entity<gpui_component::Root>);

    impl Render for ResizeRoot {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().flex().flex_col().child(self.1.clone())
        }
    }

    /// Situational ADR 0071: Root text selection must not claim a divider drag.
    #[gpui::test]
    fn adr_0071_split_resize_does_not_start_text_selection(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        for axis in [SplitPaneAxis::Horizontal, SplitPaneAxis::Vertical] {
            let (root, cx) = cx.add_window_view(|window, cx| {
                let state = cx.new(|_| ResizeTest {
                    axis,
                    extent: gpui::px(100.),
                    dragging: false,
                });
                ResizeRoot(
                    state.clone(),
                    cx.new(|cx| gpui_component::Root::new(state, window, cx)),
                )
            });
            cx.update(|window, cx| {
                let _ = window.draw(cx);
            });
            let handle = cx.debug_bounds("split-resize-handle").unwrap().center();
            let destination = handle + gpui::point(gpui::px(35.), gpui::px(35.));
            cx.simulate_mouse_down(handle, MouseButton::Left, Modifiers::default());
            cx.simulate_mouse_move(destination, MouseButton::Left, Modifiers::default());
            cx.update(|window, cx| {
                assert!(root.read(cx).0.read(cx).dragging);
                assert!(root.read(cx).0.read(cx).extent > gpui::px(100.));
                assert!(
                    !gpui_base::TextSelection::has_selection(window, cx),
                    "{axis:?}: divider drag also started text selection"
                );
            });
            cx.simulate_mouse_up(destination, MouseButton::Left, Modifiers::default());
            cx.update(|_, cx| assert!(!root.read(cx).0.read(cx).dragging));
            let extent = cx.update(|_, cx| root.read(cx).0.read(cx).extent);
            cx.simulate_mouse_move(
                gpui::point(gpui::px(20.), gpui::px(20.)),
                None,
                Modifiers::default(),
            );
            cx.update(|window, cx| {
                let _ = window.draw(cx);
                assert_eq!(root.read(cx).0.read(cx).extent, extent);
                assert!(!gpui_base::TextSelection::has_selection(window, cx));
            });
            cx.simulate_mouse_down(
                gpui::point(gpui::px(5.), gpui::px(12.)),
                MouseButton::Left,
                Modifiers::default(),
            );
            cx.simulate_mouse_move(
                gpui::point(gpui::px(40.), gpui::px(12.)),
                MouseButton::Left,
                Modifiers::default(),
            );
            cx.simulate_mouse_up(
                gpui::point(gpui::px(40.), gpui::px(12.)),
                MouseButton::Left,
                Modifiers::default(),
            );
            cx.update(|window, cx| {
                assert!(gpui_base::TextSelection::has_selection(window, cx));
                assert_eq!(root.read(cx).0.read(cx).extent, extent);
            });
        }
    }

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
