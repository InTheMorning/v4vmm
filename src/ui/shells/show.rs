//! Show screen shell.
//!
//! ADR 0060 gives playback its own screen mount instead of a workspace frame.
//! ADR 0063 arranges Show as a dashboard: the shell composes the summary, card
//! grid, transport deck, and trailing detail panel without taking ownership of
//! section detail rendering.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    div, prelude::*, App, ClickEvent, FontWeight, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window,
};

use crate::ui::composites::{
    ShowCard, ShowDetailPanel, ShowDetailPanelDisplay, ShowDetailPanelSlots,
};
use crate::ui::shells::queue_now_playing::{render_queue_transport, QueueNowPlayingSlots};
use crate::ui::tokens::{color, FontSize, SemanticColor, Spacing};
use crate::view_models::show::{
    PublisherServiceRole, ShowCardDisplay, ShowCardKind, ShowEmptyStateDisplay,
    ShowNowPlayingDisplay, ShowPageVm, ShowPanelMode, ShowWidthClass,
};

/// Callback slots supplied by the application-owned Show screen.
#[must_use]
pub(crate) struct ShowSlots {
    queue: QueueNowPlayingSlots,
    card: ShowCardSlots,
    panel: ShowDetailPanelSlots,
}

impl Default for ShowSlots {
    fn default() -> Self {
        Self {
            queue: QueueNowPlayingSlots::new(),
            card: ShowCardSlots::default(),
            panel: ShowDetailPanelSlots::new(),
        }
    }
}

type ShowCardClickHandler = Rc<dyn Fn(ShowCardKind, &ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(Default)]
struct ShowCardSlots {
    select: Option<ShowCardClickHandler>,
}

impl ShowSlots {
    /// Creates empty Show-screen slots.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Supplies the previous-track callback.
    pub(crate) fn on_skip_previous(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.queue = self.queue.on_skip_previous(handler);
        self
    }

    /// Supplies the play/pause callback.
    pub(crate) fn on_play_pause(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.queue = self.queue.on_play_pause(handler);
        self
    }

    /// Supplies the next-track callback.
    pub(crate) fn on_skip_next(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.queue = self.queue.on_skip_next(handler);
        self
    }

    /// Supplies the Source readiness open callback.
    pub(crate) fn on_open_broadcast_readiness(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.panel = self.panel.on_open_broadcast_readiness(handler);
        self
    }

    /// Supplies the dashboard card selection callback.
    pub(crate) fn on_select_card(
        mut self,
        handler: impl Fn(ShowCardKind, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.card.select = Some(Rc::new(handler));
        self
    }

    /// Supplies the panel-open callback.
    pub(crate) fn on_open_show_panel(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.panel = self.panel.on_open_panel(handler);
        self
    }

    /// Supplies the panel-close callback.
    pub(crate) fn on_close_show_panel(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.panel = self.panel.on_close_panel(handler);
        self
    }

    /// Supplies the callback that returns detail mode to the cuelist.
    pub(crate) fn on_show_cuelist(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.panel = self.panel.on_show_cuelist(handler);
        self
    }

    /// Supplies the publisher service start callback.
    pub(crate) fn on_start_publisher_service(
        mut self,
        handler: impl Fn(PublisherServiceRole, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.panel = self.panel.on_start_publisher_service(handler);
        self
    }

    /// Supplies the publisher service stop callback.
    pub(crate) fn on_stop_publisher_service(
        mut self,
        handler: impl Fn(PublisherServiceRole, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.panel = self.panel.on_stop_publisher_service(handler);
        self
    }

    /// Supplies the publisher service reset callback.
    pub(crate) fn on_reset_publisher_service(
        mut self,
        handler: impl Fn(PublisherServiceRole, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.panel = self.panel.on_reset_publisher_service(handler);
        self
    }

    /// Supplies the publisher log open callback.
    pub(crate) fn on_open_publisher_logs(
        mut self,
        handler: impl Fn(PublisherServiceRole, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.panel = self.panel.on_open_publisher_logs(handler);
        self
    }

    /// Supplies the publisher log close callback.
    pub(crate) fn on_close_publisher_logs(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.panel = self.panel.on_close_publisher_logs(handler);
        self
    }

    /// Supplies the Event target attach callback.
    pub(crate) fn on_attach_event_target(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.panel = self.panel.on_attach_event_target(handler);
        self
    }

    /// Supplies the Event target detach callback.
    pub(crate) fn on_detach_event_target(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.panel = self.panel.on_detach_event_target(handler);
        self
    }

    /// Supplies the stream connect callback.
    pub(crate) fn on_connect_stream(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.panel = self.panel.on_connect_stream(handler);
        self
    }

    /// Supplies the stream disconnect callback.
    pub(crate) fn on_disconnect_stream(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.panel = self.panel.on_disconnect_stream(handler);
        self
    }
}

/// Show screen element.
#[derive(IntoElement)]
#[must_use]
pub(crate) struct ShowShell {
    vm: ShowPageVm,
    slots: ShowSlots,
}

/// Creates the Show screen shell.
pub(crate) fn render_show(vm: ShowPageVm, slots: ShowSlots) -> ShowShell {
    ShowShell { vm, slots }
}

impl RenderOnce for ShowShell {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ShowPageVm {
            title,
            state_label,
            now_playing,
            empty_state,
            source,
            publisher,
            event,
            stream,
            cards,
            width_class,
            panel_mode,
            panel_open,
            panel_chrome,
            queue,
        } = self.vm;
        let transport = queue.transport.clone();

        div()
            .id("show-screen")
            .size_full()
            .flex()
            .flex_col()
            .min_h_0()
            .min_w_0()
            .overflow_hidden()
            .bg(color(cx, SemanticColor::SystemBackground))
            .child(render_show_summary(
                title,
                state_label,
                now_playing,
                empty_state,
                cx,
            ))
            .child(
                div()
                    .id("show-dashboard-body")
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_hidden()
                    .child(
                        div()
                            .id("show-dashboard-main")
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_h_0()
                            .min_w_0()
                            .overflow_hidden()
                            .child(render_show_card_grid(
                                cards,
                                width_class,
                                panel_mode,
                                panel_open,
                                &self.slots.card,
                                cx,
                            ))
                            .child(render_queue_transport(transport, self.slots.queue)),
                    )
                    .child(ShowDetailPanel::new(
                        ShowDetailPanelDisplay {
                            panel_mode,
                            panel_open,
                            panel_chrome,
                            queue,
                            source,
                            publisher,
                            event,
                            stream,
                        },
                        self.slots.panel,
                    )),
            )
    }
}

fn render_show_summary(
    title: &'static str,
    state_label: &'static str,
    now_playing: Option<ShowNowPlayingDisplay>,
    empty_state: Option<ShowEmptyStateDisplay>,
    cx: &App,
) -> impl IntoElement {
    let secondary_label = color(cx, SemanticColor::SecondaryLabel);
    let tertiary_label = color(cx, SemanticColor::TertiaryLabel);

    let mut summary = div()
        .id("show-summary")
        .flex()
        .flex_col()
        .flex_shrink_0()
        .gap(Spacing::SM.scaled(cx))
        .px(Spacing::XL.scaled(cx))
        .py(Spacing::LG.scaled(cx))
        .child(
            div()
                .text_size(FontSize::Caption.scaled(cx))
                .font_weight(FontWeight::MEDIUM)
                .text_color(tertiary_label)
                .child(SharedString::from(title)),
        );

    if let Some(now_playing) = now_playing {
        summary = summary.child(render_now_playing_summary(
            now_playing,
            state_label,
            secondary_label,
            tertiary_label,
            cx,
        ));
    } else if let Some(empty_state) = empty_state {
        summary = summary.child(render_empty_summary(
            empty_state,
            state_label,
            secondary_label,
            tertiary_label,
            cx,
        ));
    }

    summary
}

fn render_now_playing_summary(
    now_playing: ShowNowPlayingDisplay,
    state_label: &'static str,
    secondary_label: gpui::Rgba,
    tertiary_label: gpui::Rgba,
    cx: &App,
) -> impl IntoElement {
    div()
        .id("show-now-playing-summary")
        .tooltip({
            let label = SharedString::from(now_playing.a11y_label);
            move |window, cx| crate::ui::primitives::Tooltip::new(label.clone()).build(window, cx)
        })
        .flex()
        .flex_col()
        .gap(Spacing::XS.scaled(cx))
        .child(
            div()
                .text_size(FontSize::Title.scaled(cx))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color(cx, SemanticColor::Label))
                .truncate()
                .child(SharedString::from(now_playing.title)),
        )
        .child(render_summary_subtitle(
            now_playing.artist,
            now_playing.duration_label,
            state_label,
            secondary_label,
            tertiary_label,
            cx,
        ))
}

fn render_empty_summary(
    empty_state: ShowEmptyStateDisplay,
    state_label: &'static str,
    secondary_label: gpui::Rgba,
    tertiary_label: gpui::Rgba,
    cx: &App,
) -> impl IntoElement {
    div()
        .id(empty_state.id)
        .flex()
        .flex_col()
        .gap(Spacing::XS.scaled(cx))
        .child(
            div()
                .text_size(FontSize::Title.scaled(cx))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color(cx, SemanticColor::Label))
                .child(SharedString::from(empty_state.title)),
        )
        .child(
            div()
                .text_size(FontSize::Headline.scaled(cx))
                .text_color(secondary_label)
                .child(SharedString::from(empty_state.subtitle)),
        )
        .child(
            div()
                .text_size(FontSize::Caption.scaled(cx))
                .text_color(tertiary_label)
                .child(SharedString::from(state_label)),
        )
}

fn render_summary_subtitle(
    artist: Option<String>,
    duration_label: Option<String>,
    state_label: &'static str,
    secondary_label: gpui::Rgba,
    tertiary_label: gpui::Rgba,
    cx: &App,
) -> impl IntoElement {
    let mut subtitle = div()
        .flex()
        .flex_row()
        .gap(Spacing::SM.scaled(cx))
        .items_center()
        .min_w_0()
        .text_size(FontSize::Headline.scaled(cx))
        .text_color(secondary_label);

    if let Some(artist) = artist {
        subtitle = subtitle.child(div().min_w_0().truncate().child(SharedString::from(artist)));
    }
    if let Some(duration) = duration_label {
        subtitle = subtitle.child(
            div()
                .flex_shrink_0()
                .text_color(tertiary_label)
                .child(SharedString::from(duration)),
        );
    }

    subtitle.child(
        div()
            .flex_shrink_0()
            .text_size(FontSize::Caption.scaled(cx))
            .text_color(tertiary_label)
            .child(SharedString::from(state_label)),
    )
}

fn render_show_card_grid(
    cards: Vec<ShowCardDisplay>,
    width_class: ShowWidthClass,
    panel_mode: ShowPanelMode,
    panel_open: bool,
    slots: &ShowCardSlots,
    cx: &App,
) -> impl IntoElement {
    let select_handler = slots.select.clone();
    div()
        .id("show-card-grid")
        .flex_shrink_0()
        .grid()
        .grid_cols(width_class.columns())
        .gap(Spacing::MD.scaled(cx))
        .px(Spacing::XL.scaled(cx))
        .pb(Spacing::LG.scaled(cx))
        .children(cards.into_iter().map(move |card| {
            let selected = show_card_selected(panel_mode, panel_open, card.kind);
            let mut card = ShowCard::new(card).selected(selected);
            if let Some(handler) = select_handler.clone() {
                card = card.on_select(move |kind, event, window, cx| {
                    handler(kind, event, window, cx);
                });
            }
            card
        }))
}

fn show_card_selected(panel_mode: ShowPanelMode, panel_open: bool, kind: ShowCardKind) -> bool {
    matches!(panel_mode, ShowPanelMode::Detail(selected) if panel_open && selected == kind)
}
