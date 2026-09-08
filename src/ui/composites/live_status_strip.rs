//! Live status strip composite.
//!
//! ADR 0060 uses this as a compact glance surface above Music and Settings.
//! The composite renders display facts and exposes only one callback: open Show.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    div, prelude::*, App, ClickEvent, FontWeight, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window,
};

use crate::ui::control_styles::ControlStyle;
use crate::ui::icons::{Icon, IconName, IconSize};
use crate::ui::primitives::{Button as UiButton, Tooltip};
use crate::ui::tokens::{color, FontSize, SemanticColor, Spacing};
use crate::view_models::live_status::{
    LiveStatusDisplay, LiveStatusHealthDisplay, LiveStatusHealthIconRole, LiveStatusHealthState,
    LiveStatusNowPlayingDisplay, LiveStatusOpenShowActionDisplay, LiveStatusRecordingDisplay,
};

type LiveStatusClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// Callback slots supplied by the app shell.
#[derive(Default)]
#[must_use]
pub(crate) struct LiveStatusStripSlots {
    open_show: Option<LiveStatusClickHandler>,
}

impl LiveStatusStripSlots {
    /// Creates empty live-status slots.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Supplies the open-Show callback.
    pub(crate) fn on_open_show(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.open_show = Some(Rc::new(handler));
        self
    }
}

/// Live status strip element.
#[derive(IntoElement)]
#[must_use]
pub(crate) struct LiveStatusStrip {
    display: LiveStatusDisplay,
    slots: LiveStatusStripSlots,
}

/// Creates a live status strip.
pub(crate) fn live_status_strip(
    display: LiveStatusDisplay,
    slots: LiveStatusStripSlots,
) -> LiveStatusStrip {
    LiveStatusStrip { display, slots }
}

impl RenderOnce for LiveStatusStrip {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let LiveStatusDisplay {
            id,
            heading_label,
            summary_label,
            now_playing,
            health,
            recording,
            open_show,
            ..
        } = self.display;
        let open_show_button = render_open_show_button(open_show, self.slots.open_show);

        div()
            .id(id)
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .gap(Spacing::MD.scaled(cx))
            .flex_shrink_0()
            .px(Spacing::XL.scaled(cx))
            .py(Spacing::XS.scaled(cx))
            .border_b_1()
            .border_color(color(cx, SemanticColor::Separator))
            .bg(color(cx, SemanticColor::SecondarySystemBackground))
            .child(render_status_summary(
                heading_label,
                summary_label,
                now_playing.as_ref(),
                cx,
            ))
            .child(render_status_actions(
                health,
                recording,
                open_show_button,
                cx,
            ))
    }
}

fn render_status_summary(
    heading_label: &'static str,
    summary_label: String,
    now_playing: Option<&LiveStatusNowPlayingDisplay>,
    cx: &App,
) -> impl IntoElement {
    let summary_tooltip = now_playing.map(|value| Tooltip::new(value.a11y_label.clone()));
    let now_playing_state_label = now_playing.map(|value| value.state_label);

    div()
        .id("live-status-summary")
        .flex()
        .flex_col()
        .min_w_0()
        .gap(Spacing::XXS.scaled(cx))
        .when_some(summary_tooltip, |el, tooltip| {
            el.tooltip(move |window, cx| tooltip.build(window, cx))
        })
        .child(
            div()
                .text_size(FontSize::Micro.scaled(cx))
                .font_weight(FontWeight::MEDIUM)
                .text_color(color(cx, SemanticColor::TertiaryLabel))
                .child(SharedString::from(heading_label)),
        )
        .child(
            div()
                .min_w_0()
                .text_size(FontSize::Body.scaled(cx))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color(cx, SemanticColor::Label))
                .truncate()
                .child(SharedString::from(summary_label)),
        )
        .when_some(now_playing_state_label, |el, state_label| {
            el.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(Spacing::XS.scaled(cx))
                    .text_size(FontSize::Caption.scaled(cx))
                    .text_color(color(cx, SemanticColor::SecondaryLabel))
                    .child(div().flex_shrink_0().child(SharedString::from(state_label))),
            )
        })
}

fn render_status_actions(
    health: LiveStatusHealthDisplay,
    recording: Option<LiveStatusRecordingDisplay>,
    open_show_button: UiButton,
    cx: &App,
) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(Spacing::MD.scaled(cx))
        .flex_shrink_0()
        .child(render_health(health, cx))
        .when_some(recording, |el, recording| {
            el.child(render_recording(recording, cx))
        })
        .child(open_show_button)
}

fn render_health(health: LiveStatusHealthDisplay, cx: &App) -> impl IntoElement {
    let icon = health_icon(health.icon_role);
    let health_color = health_color(health.state);

    div()
        .id("live-status-health")
        .flex()
        .flex_row()
        .items_center()
        .gap(Spacing::XS.scaled(cx))
        .text_color(color(cx, health_color))
        .child(Icon::new(icon).size(IconSize::Action))
        .child(
            div()
                .text_size(FontSize::Caption.scaled(cx))
                .font_weight(FontWeight::MEDIUM)
                .child(SharedString::from(health.label)),
        )
        .tooltip(move |window, cx| Tooltip::new(health.label).build(window, cx))
}

fn render_recording(recording: LiveStatusRecordingDisplay, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(Spacing::XS.scaled(cx))
        .text_color(color(cx, SemanticColor::DangerLabel))
        .child(Icon::new(IconName::Stop).size(IconSize::Action))
        .child(
            div()
                .text_size(FontSize::Caption.scaled(cx))
                .font_weight(FontWeight::MEDIUM)
                .child(SharedString::from(recording.summary_label)),
        )
}

fn render_open_show_button(
    open_show: LiveStatusOpenShowActionDisplay,
    handler: Option<LiveStatusClickHandler>,
) -> UiButton {
    let mut button = UiButton::styled(open_show.id, ControlStyle::Secondary)
        .leading_icon(IconName::Play)
        .label(open_show.label)
        .a11y_label(open_show.a11y_label)
        .tooltip(open_show.a11y_label);
    if let Some(handler) = handler {
        button = button.on_click(move |event, window, cx| {
            handler(event, window, cx);
        });
    } else {
        button = button.disabled(true);
    }
    button
}

const fn health_icon(role: LiveStatusHealthIconRole) -> IconName {
    match role {
        LiveStatusHealthIconRole::Info => IconName::Info,
        LiveStatusHealthIconRole::Success => IconName::Check,
        LiveStatusHealthIconRole::Warning => IconName::Warning,
    }
}

const fn health_color(state: LiveStatusHealthState) -> SemanticColor {
    match state {
        LiveStatusHealthState::Unknown => SemanticColor::InfoLabel,
        LiveStatusHealthState::Healthy => SemanticColor::SuccessLabel,
        LiveStatusHealthState::Degraded => SemanticColor::WarningLabel,
    }
}
