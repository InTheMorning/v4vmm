//! Queue/Now Playing workspace-frame shell.
//!
//! ADR 0046 Phase 4 gives playback status, transport, liveValue output, and
//! volume controls their own frame-owned surface. The global toolbar remains a
//! compact status affordance.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    div, prelude::*, App, ClickEvent, FontWeight, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window,
};
use gpui_component::slider::{Slider, SliderState};

use crate::ui::control_styles::ControlStyle;
use crate::ui::icons::{Icon, IconName, IconSize};
use crate::ui::primitives::{
    Button, ContextMenu, ContextMenuItem, ContextMenuItemDisplay, ContextMenuScope, Tooltip,
};
use crate::ui::tokens::{color, FontSize, Radius, SemanticColor, Size, Spacing};
use crate::view_models::queue_now_playing::{
    LiveValueDeviceDisplay, QueueNowPlayingPageVm, QueueRowDisplay, TransportDisplay,
    TransportState, VolumeDisplay,
};
use crate::view_models::workspace::FrameChromeButtonDisplay;

type QueueClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// Callback slots supplied by the application-owned queue frame.
#[derive(Default)]
#[must_use]
pub(crate) struct QueueNowPlayingSlots {
    skip_previous: Option<QueueClickHandler>,
    play_pause: Option<QueueClickHandler>,
    skip_next: Option<QueueClickHandler>,
}

impl QueueNowPlayingSlots {
    /// Creates empty queue-frame slots.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Supplies the previous-track callback.
    pub(crate) fn on_skip_previous(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.skip_previous = Some(Rc::new(handler));
        self
    }

    /// Supplies the play/pause callback.
    pub(crate) fn on_play_pause(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.play_pause = Some(Rc::new(handler));
        self
    }

    /// Supplies the next-track callback.
    pub(crate) fn on_skip_next(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.skip_next = Some(Rc::new(handler));
        self
    }
}

/// Queue/Now Playing frame shell element.
#[derive(IntoElement)]
#[must_use]
pub(crate) struct QueueNowPlayingShell {
    vm: QueueNowPlayingPageVm,
    slots: QueueNowPlayingSlots,
}

/// Creates the Queue/Now Playing frame shell.
pub(crate) fn render_queue_now_playing(
    vm: QueueNowPlayingPageVm,
    slots: QueueNowPlayingSlots,
) -> QueueNowPlayingShell {
    QueueNowPlayingShell { vm, slots }
}

impl RenderOnce for QueueNowPlayingShell {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let QueueNowPlayingPageVm {
            rows,
            transport,
            live_value,
            volume,
            empty_label,
        } = self.vm;
        let slots = self.slots;

        div()
            .size_full()
            .flex()
            .flex_col()
            .min_h_0()
            .min_w_0()
            .overflow_hidden()
            .child(render_queue_list(rows, empty_label, cx))
            .child(render_control_deck(
                transport, live_value, volume, slots, window, cx,
            ))
    }
}

fn render_queue_list(
    rows: Vec<QueueRowDisplay>,
    empty_label: &'static str,
    cx: &App,
) -> impl IntoElement {
    let row_gap = Spacing::XXS.scaled(cx);

    if rows.is_empty() {
        return div()
            .flex()
            .flex_1()
            .min_h_0()
            .items_center()
            .justify_center()
            .text_size(FontSize::Caption.scaled(cx))
            .text_color(color(cx, SemanticColor::TertiaryLabel))
            .child(SharedString::from(empty_label))
            .into_any_element();
    }

    div()
        .id("queue-now-playing-list")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .overflow_y_scroll()
        .p(Spacing::MD.scaled(cx))
        .gap(row_gap)
        .children(rows.into_iter().map(|row| render_queue_row(row, cx)))
        .into_any_element()
}

fn render_queue_row(row: QueueRowDisplay, cx: &App) -> impl IntoElement {
    let label_color = color(cx, SemanticColor::Label);
    let secondary_label = color(cx, SemanticColor::SecondaryLabel);
    let tertiary_label = color(cx, SemanticColor::TertiaryLabel);
    let accent = color(cx, SemanticColor::Accent);
    let fill = color(cx, SemanticColor::TertiaryFill);

    div()
        .id(SharedString::from(row.id.clone()))
        .min_h(Size::RowLg.scaled(cx))
        .flex()
        .flex_row()
        .items_center()
        .gap(Spacing::SM.scaled(cx))
        .px(Spacing::SM.scaled(cx))
        .py(Spacing::XS.scaled(cx))
        .rounded(Radius::MD.scaled(cx))
        .when(row.now_playing, |el| el.bg(fill))
        .tooltip({
            let label = SharedString::from(row.a11y_label);
            move |window, cx| Tooltip::new(label.clone()).build(window, cx)
        })
        .child(
            div()
                .w(Size::ButtonSm.scaled(cx))
                .flex()
                .items_center()
                .justify_center()
                .when(row.now_playing, |el| {
                    el.child(
                        Icon::new(IconName::Play)
                            .size(IconSize::Action)
                            .color(accent),
                    )
                }),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .text_size(FontSize::Body.scaled(cx))
                        .text_color(label_color)
                        .font_weight(if row.now_playing {
                            FontWeight::MEDIUM
                        } else {
                            FontWeight::NORMAL
                        })
                        .truncate()
                        .child(SharedString::from(row.title)),
                )
                .when_some(row.artist, |el, artist| {
                    el.child(
                        div()
                            .text_size(FontSize::Micro.scaled(cx))
                            .text_color(secondary_label)
                            .truncate()
                            .child(SharedString::from(artist)),
                    )
                }),
        )
        .when_some(row.duration_label, |el, duration| {
            el.child(
                div()
                    .text_size(FontSize::Caption.scaled(cx))
                    .text_color(tertiary_label)
                    .child(SharedString::from(duration)),
            )
        })
}

fn render_control_deck(
    transport: TransportDisplay,
    live_value: LiveValueDeviceDisplay,
    volume: VolumeDisplay,
    slots: QueueNowPlayingSlots,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let border = color(cx, SemanticColor::Separator);
    let secondary_label = color(cx, SemanticColor::SecondaryLabel);

    div()
        .flex()
        .flex_col()
        .flex_shrink_0()
        .gap(Spacing::MD.scaled(cx))
        .border_t_1()
        .border_color(border)
        .p(Spacing::MD.scaled(cx))
        .child(render_transport(transport, slots, cx))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(Spacing::SM.scaled(cx))
                .child(
                    div()
                        .text_size(FontSize::Micro.scaled(cx))
                        .text_color(secondary_label)
                        .child("Output"),
                )
                .child(render_output_picker(live_value))
                .child(render_volume(volume, window, cx)),
        )
}

fn render_transport(
    transport: TransportDisplay,
    slots: QueueNowPlayingSlots,
    cx: &App,
) -> impl IntoElement {
    let icon = match transport.play_pause_state {
        TransportState::Playing => IconName::Pause,
        TransportState::Paused | TransportState::Stopped => IconName::Play,
    };

    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_center()
        .gap(Spacing::SM.scaled(cx))
        .child(transport_button(
            transport.skip_previous,
            IconName::Previous,
            slots.skip_previous,
        ))
        .child(transport_button(
            FrameChromeButtonDisplay::new(
                transport.play_pause_id,
                transport.play_pause_a11y_label,
                transport.disabled,
            ),
            icon,
            slots.play_pause,
        ))
        .child(transport_button(
            transport.skip_next,
            IconName::Next,
            slots.skip_next,
        ))
}

fn transport_button(
    display: FrameChromeButtonDisplay,
    icon: IconName,
    handler: Option<QueueClickHandler>,
) -> Button {
    let disabled = display.disabled;
    let mut button = Button::styled(SharedString::from(display.id), ControlStyle::ToolbarIcon)
        .leading_icon(icon)
        .a11y_label(display.a11y_label)
        .tooltip(display.a11y_label)
        .disabled(disabled);

    if !disabled {
        if let Some(handler) = handler {
            button = button.on_click(move |event, window, cx| {
                handler(event, window, cx);
            });
        }
    }

    button
}

fn render_output_picker(display: LiveValueDeviceDisplay) -> impl IntoElement {
    let selected_label = display.selected_label().to_string();
    let disabled = display.disabled;
    let mut menu = ContextMenu::new(
        display.picker_id,
        ContextMenuScope::WorkspaceFrame,
        display.a11y_label,
    )
    .trigger_label(selected_label);

    for option in display.options {
        let option_id = SharedString::from(option.id);
        menu = menu.item(ContextMenuItem::new(ContextMenuItemDisplay {
            id: option_id,
            label: SharedString::from(option.label),
            a11y_label: SharedString::from(option.a11y_label),
            destructive: false,
            disabled: option.disabled,
        }));
    }

    div().when(disabled, |el| el.opacity(0.6)).child(menu)
}

fn render_volume(display: VolumeDisplay, window: &mut Window, cx: &mut App) -> impl IntoElement {
    let VolumeDisplay {
        slider_id,
        level,
        a11y_label,
        disabled,
    } = display;
    let secondary_label = color(cx, SemanticColor::SecondaryLabel);
    let slider_key = SharedString::from(format!("{slider_id}-state"));
    let level = level.mul_add(100.0, 0.0);
    let state = window.use_keyed_state(slider_key, cx, |_window, _cx| {
        SliderState::new()
            .min(0.0)
            .max(100.0)
            .step(1.0)
            .default_value(level)
    });

    div()
        .flex()
        .flex_col()
        .gap(Spacing::XS.scaled(cx))
        .when(disabled, |el| el.opacity(0.6))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .text_size(FontSize::Micro.scaled(cx))
                .text_color(secondary_label)
                .child(SharedString::from(a11y_label))
                .child(SharedString::from(format!("{level:.0}%"))),
        )
        .child(Slider::new(&state).horizontal().disabled(disabled))
}
