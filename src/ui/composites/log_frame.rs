//! Shared framed log viewport and retained renderer handles (ADR 0063).
//!
//! App roots own a `LogFrames` collection. Hiding a source does not discard its
//! reading model or scroll handle; neither is written to configuration.

#![warn(clippy::pedantic)]

use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use gpui::{
    canvas, div, prelude::*, App, AppContext, Context, Entity, IntoElement, Render, RenderOnce,
    ScrollHandle, Window,
};
use gpui_component::scroll::{Scrollbar, ScrollbarShow};

use crate::ui::composites::SelectableText;
use crate::ui::control_styles::ControlStyle;
use crate::ui::icons::IconName;
use crate::ui::layouts;
use crate::ui::primitives::Button;
use crate::ui::tokens::{
    color, log_font_family, FontSize, Radius, SemanticColor, Spacing, LOG_LINE_HEIGHT,
    LOG_TEXT_SIZE,
};
use crate::view_models::log_view::{FollowAvailability, LogReadingVm, LogSource};

#[derive(Clone, Default)]
pub(crate) struct LogFrames(Rc<RefCell<BTreeMap<LogSource, Entity<LogFrameState>>>>);

#[derive(IntoElement)]
pub(crate) struct LogFrame {
    frames: LogFrames,
    source: LogSource,
    text: String,
    fill: bool,
    pending: bool,
}

impl LogFrame {
    pub(crate) fn new(frames: &LogFrames, source: LogSource, text: impl Into<String>) -> Self {
        Self {
            frames: frames.clone(),
            source,
            text: text.into(),
            fill: false,
            pending: false,
        }
    }

    pub(crate) fn fill(mut self) -> Self {
        self.fill = true;
        self
    }

    /// Reopening a source keeps its last snapshot until the new read completes.
    pub(crate) fn pending(mut self, pending: bool) -> Self {
        self.pending = pending;
        self
    }
}

impl RenderOnce for LogFrame {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let entity = self
            .frames
            .0
            .borrow_mut()
            .entry(self.source)
            .or_insert_with(|| cx.new(|_| LogFrameState::default()))
            .clone();
        entity.update(cx, |state, cx| {
            state.observe_user_scroll();
            if (!self.pending || state.vm.text().is_empty()) && state.vm.replace(&self.text) {
                state.restore = true;
                cx.notify();
            }
        });
        let mut frame = div().flex().flex_col().w_auto().min_w_0();
        frame.style().align_self = Some(gpui::AlignItems::Stretch);
        if self.fill {
            frame = frame.flex_1().min_h_0();
        } else {
            frame = frame
                .h(layouts::scaled_dimension(layouts::LOG_FRAME_HEIGHT, cx))
                .flex_shrink_0();
        }
        frame.child(entity)
    }
}

#[derive(Default)]
struct LogFrameState {
    vm: LogReadingVm,
    scroll: ScrollHandle,
    last_top: f32,
    line_height: f32,
    restore: bool,
}

impl LogFrameState {
    fn footer(&self, cx: &mut Context<Self>) -> gpui::Div {
        let latest_owner = cx.weak_entity();
        let action = self.vm.action();
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .flex_shrink_0()
            .gap(Spacing::SM.scaled(cx))
            .px(Spacing::SM.scaled(cx))
            .pb(Spacing::SM.scaled(cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(FontSize::Caption.scaled(cx))
                    .text_color(color(cx, SemanticColor::SecondaryLabel))
                    .child(self.vm.status()),
            )
            .child(
                Button::styled("log-go-latest", ControlStyle::RowAction)
                    .label(action.label)
                    .a11y_label(action.a11y_label)
                    .leading_icon(IconName::ChevronDown)
                    .disabled(action.availability != FollowAvailability::Available)
                    .on_activate(move |_, cx| {
                        let _ = latest_owner.update(cx, |this, cx| {
                            this.vm.latest();
                            cx.notify();
                        });
                    }),
            )
    }

    fn observe_user_scroll(&mut self) {
        if self.restore {
            return;
        }
        let top = -f32::from(self.scroll.offset().y);
        if (top - self.last_top).abs() > f32::EPSILON {
            self.vm.scroll(
                top,
                f32::from(self.scroll.max_offset().height),
                self.line_height,
            );
            self.last_top = top;
        }
    }
}

impl Render for LogFrameState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.observe_user_scroll();
        let line_height = f32::from(LOG_TEXT_SIZE.scaled(cx)) * LOG_LINE_HEIGHT;
        if (line_height - self.line_height).abs() > f32::EPSILON {
            self.line_height = line_height;
            self.restore = true;
        }
        if self.vm.following() {
            self.scroll.scroll_to_bottom();
        } else if self.restore {
            let mut offset = self.scroll.offset();
            offset.y = gpui::px(-self.vm.reading_top(line_height));
            self.scroll.set_offset(offset);
        }
        self.restore = false;
        let owner = cx.weak_entity();
        div()
            .id("log-frame")
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .border(layouts::LOG_FRAME_BORDER)
            .border_color(color(cx, SemanticColor::Separator))
            .rounded(Radius::SM.scaled(cx))
            .bg(color(cx, SemanticColor::SecondarySystemBackground))
            .text_color(color(cx, SemanticColor::Label))
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .m(Spacing::SM.scaled(cx))
                    .child(
                        div()
                            .id("log-viewport")
                            .size_full()
                            .min_w_0()
                            .min_h_0()
                            .flex()
                            .flex_col()
                            .items_start()
                            .overflow_scroll()
                            .track_scroll(&self.scroll)
                            .text_size(LOG_TEXT_SIZE.scaled(cx))
                            .font_family(log_font_family(cx))
                            .line_height(gpui::relative(LOG_LINE_HEIGHT))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_start()
                                    .flex_shrink_0()
                                    .pr(layouts::scaled_dimension(
                                        layouts::LOG_SCROLLBAR_GUTTER,
                                        cx,
                                    ))
                                    .pb(layouts::scaled_dimension(
                                        layouts::LOG_SCROLLBAR_GUTTER,
                                        cx,
                                    ))
                                    .child(SelectableText::new(
                                        "log-text",
                                        self.vm.text().to_owned(),
                                    )),
                            ),
                    )
                    .child(Scrollbar::new(&self.scroll).scrollbar_show(ScrollbarShow::Always))
                    .child(
                        canvas(
                            move |_, _, cx| {
                                let _ = owner.update(cx, |state, _| {
                                    state.last_top = -f32::from(state.scroll.offset().y);
                                });
                            },
                            |_, (), _, _| {},
                        )
                        .absolute()
                        .size_full(),
                    ),
            )
            .child(self.footer(cx))
    }
}
