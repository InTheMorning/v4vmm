//! Show bottom log pane and its shared split geometry (ADR 0063).
//!
//! The main content stays above the logs. Only the journal viewport scrolls;
//! display text, request state, and pane height belong to the Show view model.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    div, prelude::*, AnyElement, App, ClickEvent, FontWeight, IntoElement, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, RenderOnce, SharedString, Window,
};

use crate::ui::composites::split_pane::{SplitPane, SplitPaneAxis};
use crate::ui::composites::SelectableText;
use crate::ui::control_styles::ControlStyle;
use crate::ui::icons::IconName;
use crate::ui::layouts;
use crate::ui::primitives::Button;
use crate::ui::tokens::{color, FontSize, ScaleFactor, SemanticColor, Size, Spacing};
use crate::view_models::show::ShowLogPaneDisplay;

type CloseHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
type ResizeStartHandler = Rc<dyn Fn(&MouseDownEvent, &mut Window, &mut App)>;
type ResizeMoveHandler = Rc<dyn Fn(&MouseMoveEvent, &mut Window, &mut App)>;
type ResizeEndHandler = Rc<dyn Fn(&MouseUpEvent, &mut Window, &mut App)>;
type LayoutHandler = Rc<dyn Fn(f32, f32, &mut Window, &mut App)>;

/// Application callbacks for the independent log pane.
#[derive(Default)]
pub(crate) struct ShowLogPaneSlots {
    pub(crate) close: Option<CloseHandler>,
    pub(crate) resize_start: Option<ResizeStartHandler>,
    pub(crate) resize_move: Option<ResizeMoveHandler>,
    pub(crate) resize_end: Option<ResizeEndHandler>,
    pub(crate) layout: Option<LayoutHandler>,
}

/// Main-region split with an optional log pane below its supplied card grid.
#[derive(IntoElement)]
pub(crate) struct ShowLogPane {
    display: ShowLogPaneDisplay,
    slots: ShowLogPaneSlots,
    main: AnyElement,
    grid_rows: u16,
}

impl ShowLogPane {
    pub(crate) fn new(display: ShowLogPaneDisplay, main: AnyElement, grid_rows: u16) -> Self {
        Self {
            display,
            slots: ShowLogPaneSlots::default(),
            main,
            grid_rows,
        }
    }

    pub(crate) fn slots(mut self, slots: ShowLogPaneSlots) -> Self {
        self.slots = slots;
        self
    }
}

impl RenderOnce for ShowLogPane {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        if !self.display.open {
            return div()
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .min_w_0()
                .child(self.main)
                .into_any_element();
        }

        // These are the card-grid tokens; reserve every row before sizing logs.
        let main_min_height = Size::MenuCompact.scaled(cx) * f32::from(self.grid_rows)
            + Spacing::MD.scaled(cx) * f32::from(self.grid_rows.saturating_sub(1))
            + Spacing::LG.scaled(cx);
        let handle_height = layouts::SPLIT_HANDLE_WIDTH;
        let main_height =
            (layouts::scaled_f32(self.display.region_height - self.display.height, cx)
                - handle_height)
                .max(main_min_height);
        let scale = ScaleFactor::current(cx).multiplier();
        let mut split = SplitPane::new("show-log-split")
            .axis(SplitPaneAxis::Vertical)
            .resize_handle_id("show-log-resize-handle")
            .leading_height(main_height)
            .leading_min_height(main_min_height)
            .leading(self.main)
            .trailing(render_log_output(self.display, self.slots.close, cx));

        if let Some(handler) = self.slots.layout {
            split = split.on_layout(move |bounds, window, cx| {
                let available =
                    (bounds.size.height - main_min_height - handle_height).max(gpui::Pixels::ZERO);
                handler(
                    f32::from(bounds.size.height) / scale,
                    f32::from(available) / scale,
                    window,
                    cx,
                );
            });
        }
        if let Some(handler) = self.slots.resize_start {
            split = split.on_resize_start(move |event, window, cx| handler(event, window, cx));
        }
        if let Some(handler) = self.slots.resize_move {
            split = split.on_resize_move(move |event, window, cx| handler(event, window, cx));
        }
        if let Some(handler) = self.slots.resize_end {
            split = split.on_resize_end(move |event, window, cx| handler(event, window, cx));
        }
        split.into_any_element()
    }
}

fn render_log_output(
    display: ShowLogPaneDisplay,
    close_handler: Option<CloseHandler>,
    cx: &App,
) -> AnyElement {
    let disabled = display.close.disabled() || close_handler.is_none();
    let mut close = Button::styled(
        SharedString::from(display.close.id),
        ControlStyle::ToolbarIcon,
    )
    .leading_icon(IconName::Close)
    .a11y_label(display.close.a11y_label.clone())
    .tooltip(display.close.a11y_label)
    .disabled(disabled);
    if !disabled {
        if let Some(handler) = close_handler {
            close = close.on_click(move |event, window, cx| handler(event, window, cx));
        }
    }

    div()
        .id("show-log-pane")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .bg(color(cx, SemanticColor::SecondarySystemBackground))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .flex_shrink_0()
                .gap(Spacing::SM.scaled(cx))
                .px(Spacing::LG.scaled(cx))
                .py(Spacing::SM.scaled(cx))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .child(
                            div()
                                .text_size(FontSize::Body.scaled(cx))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(color(cx, SemanticColor::Label))
                                .child(SharedString::from(display.unit_name)),
                        )
                        .child(
                            div()
                                .text_size(FontSize::Micro.scaled(cx))
                                .text_color(color(cx, SemanticColor::TertiaryLabel))
                                .child(SharedString::from(display.line_count_label)),
                        ),
                )
                .child(close),
        )
        .child(
            div()
                .id("show-log-output")
                .flex()
                .flex_col()
                .items_start()
                .flex_1()
                .min_h_0()
                .min_w_0()
                .overflow_scroll()
                .p(Spacing::SM.scaled(cx))
                .text_size(FontSize::Micro.scaled(cx))
                .text_color(color(cx, SemanticColor::SecondaryLabel))
                .child(SelectableText::new("show-log-text", display.text)),
        )
        .into_any_element()
}
