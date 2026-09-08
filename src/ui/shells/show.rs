//! Show screen shell.
//!
//! ADR 0060 gives playback its own screen mount instead of a workspace frame.
//! This shell renders the show summary and embeds the Queue/Now Playing
//! surface without adding frame chrome, history, or breadcrumbs.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    div, prelude::*, App, ClickEvent, FontWeight, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window,
};
use gpui_component::scroll::ScrollableElement;

use crate::ui::control_styles::ControlStyle;
use crate::ui::icons::IconName;
use crate::ui::primitives::{Button, MultilineText, SectionHeader, Surface, SurfaceElevation};
use crate::ui::shells::queue_now_playing::{render_queue_now_playing, QueueNowPlayingSlots};
use crate::ui::tokens::{color, FontSize, SemanticColor, Size, Spacing};
use crate::view_models::show::{
    PublisherActionDisplay, PublisherLogPanelState, PublisherSectionDisplay,
    PublisherServiceDisplay, PublisherServiceRole, PublisherServiceStateKind,
    ShowEmptyStateDisplay, ShowNowPlayingDisplay, ShowPageVm,
};

/// Callback slots supplied by the application-owned Show screen.
#[must_use]
pub(crate) struct ShowSlots {
    queue: QueueNowPlayingSlots,
    publisher: PublisherSlots,
}

impl Default for ShowSlots {
    fn default() -> Self {
        Self {
            queue: QueueNowPlayingSlots::new(),
            publisher: PublisherSlots::default(),
        }
    }
}

type PublisherClickHandler =
    Rc<dyn Fn(PublisherServiceRole, &ClickEvent, &mut Window, &mut App) + 'static>;
type PublisherCloseHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(Default)]
struct PublisherSlots {
    start: Option<PublisherClickHandler>,
    stop: Option<PublisherClickHandler>,
    reset: Option<PublisherClickHandler>,
    open_logs: Option<PublisherClickHandler>,
    close_logs: Option<PublisherCloseHandler>,
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

    /// Supplies the publisher service start callback.
    pub(crate) fn on_start_publisher_service(
        mut self,
        handler: impl Fn(PublisherServiceRole, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.publisher.start = Some(Rc::new(handler));
        self
    }

    /// Supplies the publisher service stop callback.
    pub(crate) fn on_stop_publisher_service(
        mut self,
        handler: impl Fn(PublisherServiceRole, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.publisher.stop = Some(Rc::new(handler));
        self
    }

    /// Supplies the publisher service reset callback.
    pub(crate) fn on_reset_publisher_service(
        mut self,
        handler: impl Fn(PublisherServiceRole, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.publisher.reset = Some(Rc::new(handler));
        self
    }

    /// Supplies the publisher log open callback.
    pub(crate) fn on_open_publisher_logs(
        mut self,
        handler: impl Fn(PublisherServiceRole, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.publisher.open_logs = Some(Rc::new(handler));
        self
    }

    /// Supplies the publisher log close callback.
    pub(crate) fn on_close_publisher_logs(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.publisher.close_logs = Some(Rc::new(handler));
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
            publisher,
            queue,
        } = self.vm;

        let mut screen = div()
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
            ));

        if let Some(publisher) = publisher {
            screen = screen.child(render_publisher_section(
                publisher,
                self.slots.publisher,
                cx,
            ));
        }

        screen.child(
            div()
                .id("show-queue-transport")
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .min_w_0()
                .overflow_hidden()
                .border_t_1()
                .border_color(color(cx, SemanticColor::Separator))
                .child(render_queue_now_playing(queue, self.slots.queue)),
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

fn render_publisher_section(
    publisher: PublisherSectionDisplay,
    slots: PublisherSlots,
    cx: &App,
) -> impl IntoElement {
    let mut body = div()
        .id("publisher-section-body")
        .flex()
        .flex_col()
        .gap(Spacing::SM.scaled(cx))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .gap(Spacing::SM.scaled(cx))
                .child(SectionHeader::new(publisher.title))
                .child(
                    div()
                        .text_size(FontSize::Caption.scaled(cx))
                        .text_color(color(cx, SemanticColor::SecondaryLabel))
                        .child(SharedString::from(publisher.summary.clone())),
                ),
        );

    for service in publisher.services {
        body = body.child(render_publisher_service(service, &slots, cx));
    }

    if publisher.log_panel.is_open() {
        body = body.child(render_publisher_log_panel(
            publisher.log_panel,
            publisher.close_logs,
            slots.close_logs,
            cx,
        ));
    }

    div()
        .id("show-publisher-section")
        .flex_shrink_0()
        .px(Spacing::XL.scaled(cx))
        .pb(Spacing::LG.scaled(cx))
        .child(
            Surface::new(SurfaceElevation::Sunken)
                .padding(Spacing::MD)
                .child(body),
        )
}

fn render_publisher_service(
    service: PublisherServiceDisplay,
    slots: &PublisherSlots,
    cx: &App,
) -> impl IntoElement {
    let state_color = state_color(service.state.kind());
    let mut text = div()
        .flex()
        .flex_col()
        .min_w_0()
        .gap(Spacing::XXS.scaled(cx))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(Spacing::SM.scaled(cx))
                .child(
                    div()
                        .text_size(FontSize::Headline.scaled(cx))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(color(cx, SemanticColor::Label))
                        .child(SharedString::from(service.label)),
                )
                .child(
                    div()
                        .text_size(FontSize::Caption.scaled(cx))
                        .text_color(color(cx, state_color))
                        .child(SharedString::from(service.state.label())),
                ),
        )
        .child(
            div()
                .text_size(FontSize::Caption.scaled(cx))
                .text_color(color(cx, SemanticColor::TertiaryLabel))
                .truncate()
                .child(SharedString::from(service.unit_name.clone())),
        );

    if let Some(detail) = service.state.detail() {
        text = text.child(
            div()
                .text_size(FontSize::Caption.scaled(cx))
                .text_color(color(cx, detail_color(service.state.kind())))
                .child(SharedString::from(detail)),
        );
    }

    div()
        .id(SharedString::from(service.id.clone()))
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(Spacing::MD.scaled(cx))
        .border_t_1()
        .border_color(color(cx, SemanticColor::Separator))
        .pt(Spacing::SM.scaled(cx))
        .child(text)
        .child(render_publisher_actions(service, slots, cx))
}

fn render_publisher_actions(
    service: PublisherServiceDisplay,
    slots: &PublisherSlots,
    cx: &App,
) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(Spacing::XS.scaled(cx))
        .child(render_publisher_action_button(
            service.role,
            service.actions.start,
            IconName::Play,
            ControlStyle::Primary,
            slots.start.clone(),
        ))
        .child(render_publisher_action_button(
            service.role,
            service.actions.stop,
            IconName::Stop,
            ControlStyle::Destructive,
            slots.stop.clone(),
        ))
        .child(render_publisher_action_button(
            service.role,
            service.actions.reset,
            IconName::Warning,
            ControlStyle::Secondary,
            slots.reset.clone(),
        ))
        .child(render_publisher_action_button(
            service.role,
            service.logs.action,
            IconName::Info,
            ControlStyle::Ghost,
            slots.open_logs.clone(),
        ))
}

fn render_publisher_action_button(
    role: PublisherServiceRole,
    display: PublisherActionDisplay,
    icon: IconName,
    style: ControlStyle,
    handler: Option<PublisherClickHandler>,
) -> Button {
    let disabled = display.disabled();
    let mut button = Button::styled(SharedString::from(display.id), style)
        .leading_icon(icon)
        .label(display.label)
        .a11y_label(display.a11y_label.clone())
        .tooltip(display.a11y_label)
        .disabled(disabled);

    if !disabled {
        if let Some(handler) = handler {
            button = button.on_click(move |event, window, cx| {
                handler(role, event, window, cx);
            });
        }
    }

    button
}

fn render_publisher_log_panel(
    panel: PublisherLogPanelState,
    close_display: PublisherActionDisplay,
    close_logs: Option<PublisherCloseHandler>,
    cx: &App,
) -> impl IntoElement {
    let PublisherLogPanelState::Open {
        unit_name,
        line_count,
        text,
        ..
    } = panel
    else {
        return div().into_any_element();
    };
    let title = format!("{line_count} latest log lines");

    div()
        .id("publisher-log-panel")
        .flex()
        .flex_col()
        .gap(Spacing::SM.scaled(cx))
        .border_t_1()
        .border_color(color(cx, SemanticColor::Separator))
        .pt(Spacing::SM.scaled(cx))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .gap(Spacing::SM.scaled(cx))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .min_w_0()
                        .gap(Spacing::XXS.scaled(cx))
                        .child(
                            div()
                                .text_size(FontSize::Headline.scaled(cx))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(color(cx, SemanticColor::Label))
                                .child(SharedString::from(title)),
                        )
                        .child(
                            div()
                                .text_size(FontSize::Caption.scaled(cx))
                                .text_color(color(cx, SemanticColor::TertiaryLabel))
                                .truncate()
                                .child(SharedString::from(unit_name)),
                        ),
                )
                .child(render_close_logs_button(close_display, close_logs)),
        )
        .child(
            div()
                .max_h(Size::ColumnShort.scaled(cx))
                .overflow_y_scrollbar()
                .border_1()
                .border_color(color(cx, SemanticColor::Separator))
                .bg(color(cx, SemanticColor::SystemBackground))
                .p(Spacing::SM.scaled(cx))
                .child(
                    MultilineText::new(text)
                        .max_lines(line_count)
                        .wrap_lines()
                        .size(FontSize::Caption)
                        .color(SemanticColor::Label),
                ),
        )
        .into_any_element()
}

fn render_close_logs_button(
    display: PublisherActionDisplay,
    close_logs: Option<PublisherCloseHandler>,
) -> Button {
    let disabled = display.disabled();
    let mut button = Button::styled(SharedString::from(display.id), ControlStyle::ToolbarIcon)
        .leading_icon(IconName::Close)
        .a11y_label(display.a11y_label.clone())
        .tooltip(display.a11y_label)
        .disabled(disabled);

    if !disabled {
        if let Some(handler) = close_logs {
            button = button.on_click(move |event, window, cx| {
                handler(event, window, cx);
            });
        }
    }

    button
}

const fn state_color(kind: PublisherServiceStateKind) -> SemanticColor {
    match kind {
        PublisherServiceStateKind::Active => SemanticColor::SuccessLabel,
        PublisherServiceStateKind::Inactive => SemanticColor::SecondaryLabel,
        PublisherServiceStateKind::Failed => SemanticColor::DangerLabel,
        PublisherServiceStateKind::NotInstalled
        | PublisherServiceStateKind::NotReachable
        | PublisherServiceStateKind::Unknown => SemanticColor::WarningLabel,
    }
}

const fn detail_color(kind: PublisherServiceStateKind) -> SemanticColor {
    match kind {
        PublisherServiceStateKind::Failed => SemanticColor::DangerLabel,
        PublisherServiceStateKind::Active
        | PublisherServiceStateKind::Inactive
        | PublisherServiceStateKind::NotInstalled
        | PublisherServiceStateKind::NotReachable
        | PublisherServiceStateKind::Unknown => SemanticColor::SecondaryLabel,
    }
}
