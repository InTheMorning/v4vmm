//! Show cards and bottom log pane (ADRs 0063, 0070 and 0073).
//!
//! The cards scroll whenever their allocated viewport is too small, with or without logs;
//! display text, request state, and pane height belong to the Show view model.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    div, prelude::*, AnyElement, App, ClickEvent, ClipboardItem, FontWeight, IntoElement,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, RenderOnce, SharedString, Window,
};
use gpui_component::scroll::ScrollableElement;

use crate::ui::composites::log_frame::{LogFrame, LogFrames};
use crate::ui::composites::page_scroll_content::page_scroll_content;
use crate::ui::composites::selectable_text::SelectableText;
use crate::ui::composites::split_pane::{SplitPane, SplitPaneAxis};
use crate::ui::control_styles::ControlStyle;
use crate::ui::icons::IconName;
use crate::ui::layouts;
use crate::ui::primitives::{Button, Tooltip};
use crate::ui::tokens::{color, FontSize, ScaleFactor, SemanticColor, Spacing};
use crate::view_models::show::ShowLogPaneDisplay;

type CloseHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
type ResizeStartHandler = Rc<dyn Fn(&MouseDownEvent, &mut Window, &mut App)>;
type ResizeMoveHandler = Rc<dyn Fn(&MouseMoveEvent, &mut Window, &mut App)>;
type ResizeEndHandler = Rc<dyn Fn(&MouseUpEvent, &mut Window, &mut App)>;
type LayoutHandler = Rc<dyn Fn(f32, f32, &mut Window, &mut App)>;

/// Application callbacks for the independent log pane.
#[derive(Default)]
pub(crate) struct ShowLogPaneSlots {
    pub(crate) frames: LogFrames,
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
}

impl ShowLogPane {
    pub(crate) fn new(display: ShowLogPaneDisplay, main: AnyElement) -> Self {
        Self {
            display,
            slots: ShowLogPaneSlots::default(),
            main,
        }
    }

    pub(crate) fn slots(mut self, slots: ShowLogPaneSlots) -> Self {
        self.slots = slots;
        self
    }
}

impl RenderOnce for ShowLogPane {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let cards = div()
            .id("show-card-scroll")
            .size_full()
            .min_h_0()
            .min_w_0()
            .flex()
            .flex_col()
            .overflow_y_scrollbar()
            .child(page_scroll_content(cx).child(self.main));
        if !self.display.open {
            let main = div()
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .min_w_0()
                .child(cards);
            #[cfg(test)]
            let main = main.debug_selector(|| "show-card-viewport".to_owned());
            return main.into_any_element();
        }

        let handle_height = layouts::SPLIT_HANDLE_WIDTH;
        let main_height =
            (layouts::scaled_f32(self.display.region_height - self.display.height, cx)
                - handle_height)
                .max(gpui::Pixels::ZERO);
        // ADR 0039: un-scaling drag bounds is geometry — CHROME domain.
        let scale = ScaleFactor::current(cx).chrome_multiplier();
        let mut split = SplitPane::new("show-log-split")
            .axis(SplitPaneAxis::Vertical)
            .resize_handle_id("show-log-resize-handle")
            .leading_height(main_height)
            .leading_min_height(gpui::Pixels::ZERO)
            .leading(cards.into_any_element())
            .trailing(render_log_output(
                self.display,
                self.slots.close,
                &self.slots.frames,
                cx,
            ));

        if let Some(handler) = self.slots.layout {
            split = split.on_layout(move |bounds, window, cx| {
                handler(
                    f32::from(bounds.size.height) / scale,
                    f32::from(handle_height) / scale,
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
    frames: &LogFrames,
    cx: &App,
) -> AnyElement {
    let source = display.source.clone();
    let pending = display.reading();
    let header_text = render_header_text(&display, cx);
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
                .child(header_text)
                .children(
                    display
                        .copy_feed_tag
                        .zip(display.feed_tag)
                        .map(|(action, tag)| {
                            let disabled = action.disabled();
                            Button::styled(action.id, ControlStyle::RowAction)
                                .label(action.label)
                                .a11y_label(action.a11y_label)
                                .disabled(disabled)
                                .on_activate(move |_, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(tag.clone()));
                                })
                        }),
                )
                .child(close),
        )
        .child(
            LogFrame::new(frames, source, display.text)
                .pending(pending)
                .fill(),
        )
        .into_any_element()
}

fn render_header_text(display: &ShowLogPaneDisplay, cx: &App) -> gpui::Div {
    let source_label = SharedString::from(display.unit_name.clone());
    let count_label = SharedString::from(display.line_count_label.clone());
    let service_detail = display.service_detail.clone().map(SharedString::from);
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .overflow_hidden()
        .child(
            div()
                .id("show-log-source")
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_size(FontSize::Body.scaled(cx))
                .font_weight(FontWeight::MEDIUM)
                .text_color(color(cx, SemanticColor::Label))
                .tooltip(move |window, cx| Tooltip::new(source_label.clone()).build(window, cx))
                .child(SelectableText::new(
                    "show-log-source-text",
                    display.unit_name.clone(),
                )),
        )
        .child(
            div()
                .id("show-log-metadata")
                .flex()
                .items_center()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .gap(Spacing::SM.scaled(cx))
                .text_size(FontSize::Micro.scaled(cx))
                .text_color(color(cx, SemanticColor::TertiaryLabel))
                .child(
                    div()
                        .id("show-log-line-count")
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .tooltip(move |window, cx| {
                            Tooltip::new(count_label.clone()).build(window, cx)
                        })
                        .child(SharedString::from(display.line_count_label.clone())),
                )
                .children(display.header_status.map(|status| {
                    div()
                        .flex_shrink_0()
                        .text_color(color(cx, SemanticColor::SecondaryLabel))
                        .child(status)
                }))
                .when_some(service_detail, |metadata, detail| {
                    metadata
                        .tooltip(move |window, cx| Tooltip::new(detail.clone()).build(window, cx))
                }),
        )
}

#[cfg(test)]
mod tests {
    use gpui::{Context, Render, ScrollDelta, ScrollWheelEvent, TestAppContext, TouchPhase};

    use super::*;

    struct CardOverflowTest {
        height: f32,
        log_open: bool,
    }

    impl Render for CardOverflowTest {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let mut display = ShowLogPaneDisplay::closed();
            display.open = self.log_open;
            display.update_geometry(self.height - 40., f32::from(layouts::SPLIT_HANDLE_WIDTH));
            let grid = div().flex().flex_col().children(
                [
                    ("overflow-card-0", "Source"),
                    ("overflow-card-1", "Live Metadata"),
                    ("overflow-card-2", "Stream"),
                ]
                .into_iter()
                .map(|(id, label)| {
                    div()
                        .h(gpui::px(180.))
                        .flex_shrink_0()
                        .debug_selector(move || id.to_owned())
                        .child(label)
                }),
            );
            div()
                .w(gpui::px(320.))
                .h(gpui::px(self.height))
                .flex()
                .flex_col()
                .overflow_hidden()
                .child(ShowLogPane::new(display, grid.into_any_element()))
                .child(
                    div()
                        .h(gpui::px(40.))
                        .flex_shrink_0()
                        .debug_selector(|| "overflow-transport".to_owned())
                        .child("Transport"),
                )
        }
    }

    /// Situational ADR 0073: all cards remain reachable without opening a log.
    #[gpui::test]
    fn adr_0073_cards_scroll_with_logs_closed_and_open(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        for (height, log_open) in [(340., false), (500., false), (340., true), (500., true)] {
            let (_, cx) = cx.add_window_view(|_, _| CardOverflowTest { height, log_open });
            cx.update(|window, cx| {
                let _ = window.draw(cx);
            });
            let viewport = cx
                .debug_bounds(if log_open {
                    "split-leading-pane"
                } else {
                    "show-card-viewport"
                })
                .unwrap();
            let transport = cx.debug_bounds("overflow-transport").unwrap();
            assert!(viewport.size.height > gpui::Pixels::ZERO);
            assert!(
                viewport.bottom() <= transport.top(),
                "height {height}, log open {log_open}: viewport {viewport:?}, transport {transport:?}"
            );
            assert_eq!(transport.bottom(), gpui::px(height));
            if !log_open {
                assert_eq!(viewport.bottom(), transport.top());
            }
            cx.simulate_event(ScrollWheelEvent {
                position: viewport.center(),
                delta: ScrollDelta::Pixels(gpui::point(gpui::px(0.), gpui::px(-1000.))),
                modifiers: Default::default(),
                touch_phase: TouchPhase::Moved,
            });
            cx.update(|window, cx| {
                let _ = window.draw(cx);
            });
            let last = cx.debug_bounds("overflow-card-2").unwrap();
            assert!(last.top() < viewport.bottom());
            assert!(last.bottom() <= viewport.bottom());
            assert_eq!(cx.debug_bounds("overflow-transport").unwrap(), transport);
        }
    }
}
