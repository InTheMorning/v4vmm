//! Show side-panel composite.
//!
//! ADR 0063 moves card detail and the cuelist into one trailing panel. This
//! composite owns panel geometry, scrolling, detail row layout, and the action
//! buttons for section detail. The caller supplies already-projected display
//! state and command slots.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    div, prelude::*, AnyElement, App, ClickEvent, FontWeight, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, Styled, Window,
};

use crate::ui::control_styles::ControlStyle;
use crate::ui::icons::IconName;
use crate::ui::primitives::Button;
use crate::ui::shells::queue_now_playing::render_queue_cuelist;
use crate::ui::tokens::{color, FontSize, Radius, SemanticColor, Size, Spacing};
use crate::view_models::queue_now_playing::QueueNowPlayingPageVm;
use crate::view_models::show::{
    EventActionDisplay, EventSectionDisplay, PublisherActionDisplay, PublisherSectionDisplay,
    PublisherServiceDisplay, PublisherServiceRole, ShowCardKind, ShowPanelActionDisplay,
    ShowPanelChromeDisplay, ShowPanelMode, SourceReadinessActionDisplay, SourceSectionDisplay,
    StreamActionDisplay, StreamSectionDisplay,
};

type PanelClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;
type PublisherClickHandler =
    Rc<dyn Fn(PublisherServiceRole, &ClickEvent, &mut Window, &mut App) + 'static>;

/// Display inputs for the Show side panel.
#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub(crate) struct ShowDetailPanelDisplay {
    /// Current panel mode.
    pub(crate) panel_mode: ShowPanelMode,
    /// Whether the trailing panel is open.
    pub(crate) panel_open: bool,
    /// Display-ready panel chrome actions.
    pub(crate) panel_chrome: ShowPanelChromeDisplay,
    /// Queue display used by cuelist mode.
    pub(crate) queue: QueueNowPlayingPageVm,
    /// Source detail display, when available.
    pub(crate) source: Option<SourceSectionDisplay>,
    /// Live Metadata detail display, when available.
    pub(crate) publisher: Option<PublisherSectionDisplay>,
    /// Event detail display, when available.
    pub(crate) event: Option<EventSectionDisplay>,
    /// Stream detail display, when available.
    pub(crate) stream: Option<StreamSectionDisplay>,
}

/// Callback slots for the Show side panel.
#[derive(Clone, Default)]
#[must_use]
pub(crate) struct ShowDetailPanelSlots {
    open_panel: Option<PanelClickHandler>,
    close_panel: Option<PanelClickHandler>,
    show_cuelist: Option<PanelClickHandler>,
    open_readiness: Option<PanelClickHandler>,
    publisher_start: Option<PublisherClickHandler>,
    publisher_stop: Option<PublisherClickHandler>,
    publisher_reset: Option<PublisherClickHandler>,
    publisher_open_logs: Option<PublisherClickHandler>,
    event_attach: Option<PanelClickHandler>,
    event_detach: Option<PanelClickHandler>,
    stream_connect: Option<PanelClickHandler>,
    stream_disconnect: Option<PanelClickHandler>,
}

impl ShowDetailPanelSlots {
    /// Creates empty side-panel slots.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Supplies the panel-open callback.
    pub(crate) fn on_open_panel(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.open_panel = Some(Rc::new(handler));
        self
    }

    /// Supplies the panel-close callback.
    pub(crate) fn on_close_panel(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.close_panel = Some(Rc::new(handler));
        self
    }

    /// Supplies the callback that returns detail mode to the cuelist.
    pub(crate) fn on_show_cuelist(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.show_cuelist = Some(Rc::new(handler));
        self
    }

    /// Supplies the Source readiness open callback.
    pub(crate) fn on_open_broadcast_readiness(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.open_readiness = Some(Rc::new(handler));
        self
    }

    /// Supplies the publisher service start callback.
    pub(crate) fn on_start_publisher_service(
        mut self,
        handler: impl Fn(PublisherServiceRole, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.publisher_start = Some(Rc::new(handler));
        self
    }

    /// Supplies the publisher service stop callback.
    pub(crate) fn on_stop_publisher_service(
        mut self,
        handler: impl Fn(PublisherServiceRole, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.publisher_stop = Some(Rc::new(handler));
        self
    }

    /// Supplies the publisher service reset callback.
    pub(crate) fn on_reset_publisher_service(
        mut self,
        handler: impl Fn(PublisherServiceRole, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.publisher_reset = Some(Rc::new(handler));
        self
    }

    /// Supplies the publisher log open callback.
    pub(crate) fn on_open_publisher_logs(
        mut self,
        handler: impl Fn(PublisherServiceRole, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.publisher_open_logs = Some(Rc::new(handler));
        self
    }

    /// Supplies the Event target attach callback.
    pub(crate) fn on_attach_event_target(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.event_attach = Some(Rc::new(handler));
        self
    }

    /// Supplies the Event target detach callback.
    pub(crate) fn on_detach_event_target(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.event_detach = Some(Rc::new(handler));
        self
    }

    /// Supplies the stream connect callback.
    pub(crate) fn on_connect_stream(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.stream_connect = Some(Rc::new(handler));
        self
    }

    /// Supplies the stream disconnect callback.
    pub(crate) fn on_disconnect_stream(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.stream_disconnect = Some(Rc::new(handler));
        self
    }
}

/// Collapsible trailing panel for Show cuelist and card detail.
#[derive(IntoElement)]
#[must_use]
pub(crate) struct ShowDetailPanel {
    display: ShowDetailPanelDisplay,
    slots: ShowDetailPanelSlots,
}

struct ShowDetailSections {
    source: Option<SourceSectionDisplay>,
    publisher: Option<PublisherSectionDisplay>,
    event: Option<EventSectionDisplay>,
    stream: Option<StreamSectionDisplay>,
}

impl ShowDetailPanel {
    /// Creates a Show detail panel.
    pub(crate) fn new(display: ShowDetailPanelDisplay, slots: ShowDetailPanelSlots) -> Self {
        Self { display, slots }
    }
}

impl RenderOnce for ShowDetailPanel {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let slots = self.slots;
        let ShowDetailPanelDisplay {
            panel_mode,
            panel_open,
            panel_chrome,
            queue,
            source,
            publisher,
            event,
            stream,
        } = self.display;
        let sections = ShowDetailSections {
            source,
            publisher,
            event,
            stream,
        };

        if !panel_open {
            return render_closed_panel(panel_chrome.open_panel, &slots, cx).into_any_element();
        }

        div()
            .id("show-detail-panel")
            .h_full()
            .w(Size::ColumnRegular.scaled(cx))
            .min_w(Size::ColumnShort.scaled(cx))
            .flex()
            .flex_col()
            .flex_shrink_0()
            .min_h_0()
            .border_l_1()
            .border_color(color(cx, SemanticColor::Separator))
            .bg(color(cx, SemanticColor::SecondarySystemBackground))
            .child(render_panel_header(panel_mode, panel_chrome, &slots, cx))
            .child(render_panel_body(panel_mode, queue, sections, &slots, cx))
            .into_any_element()
    }
}

fn render_closed_panel(
    open_panel: ShowPanelActionDisplay,
    slots: &ShowDetailPanelSlots,
    cx: &App,
) -> impl IntoElement {
    // A layout child, never absolute. An overlaid rail clips the card beneath it.
    div()
        .id("show-detail-panel")
        .h_full()
        .w(Size::MinHitTarget.scaled(cx))
        .flex()
        .flex_col()
        .flex_shrink_0()
        .items_center()
        .border_l_1()
        .border_color(color(cx, SemanticColor::Separator))
        .bg(color(cx, SemanticColor::SecondarySystemBackground))
        .pt(Spacing::XS.scaled(cx))
        .child(panel_icon_button(
            open_panel,
            IconName::ChevronLeft,
            slots.open_panel.clone(),
        ))
}

fn render_panel_header(
    panel_mode: ShowPanelMode,
    panel_chrome: ShowPanelChromeDisplay,
    slots: &ShowDetailPanelSlots,
    cx: &App,
) -> impl IntoElement {
    div()
        .id("show-detail-panel-header")
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(Spacing::SM.scaled(cx))
        .flex_shrink_0()
        .px(Spacing::MD.scaled(cx))
        .py(Spacing::SM.scaled(cx))
        .border_b_1()
        .border_color(color(cx, SemanticColor::Separator))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(Spacing::XS.scaled(cx))
                .min_w_0()
                .when(matches!(panel_mode, ShowPanelMode::Detail(_)), |el| {
                    el.child(panel_label_button(
                        panel_chrome.show_cuelist,
                        IconName::Back,
                        slots.show_cuelist.clone(),
                    ))
                })
                .child(
                    div()
                        .flex_1()
                        .truncate()
                        .text_size(FontSize::Headline.scaled(cx))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(color(cx, SemanticColor::Label))
                        .child(SharedString::from(panel_mode.title())),
                ),
        )
        .child(panel_icon_button(
            panel_chrome.close_panel,
            IconName::ChevronRight,
            slots.close_panel.clone(),
        ))
}

fn render_panel_body(
    panel_mode: ShowPanelMode,
    queue: QueueNowPlayingPageVm,
    sections: ShowDetailSections,
    slots: &ShowDetailPanelSlots,
    cx: &App,
) -> AnyElement {
    match panel_mode {
        ShowPanelMode::Cuelist => div()
            .id("show-detail-panel-cuelist")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .child(render_queue_cuelist(queue))
            .into_any_element(),
        ShowPanelMode::Detail(kind) => div()
            .id("show-detail-panel-detail")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .overflow_y_scroll()
            .child(render_card_detail(kind, sections, slots, cx))
            .into_any_element(),
    }
}

fn render_card_detail(
    kind: ShowCardKind,
    sections: ShowDetailSections,
    slots: &ShowDetailPanelSlots,
    cx: &App,
) -> AnyElement {
    match kind {
        ShowCardKind::Source => sections.source.map_or_else(
            || render_missing_detail(kind, cx),
            |section| render_source_detail(section, slots, cx).into_any_element(),
        ),
        ShowCardKind::LiveMetadata => sections.publisher.map_or_else(
            || render_missing_detail(kind, cx),
            |section| render_publisher_detail(section, slots, cx).into_any_element(),
        ),
        ShowCardKind::Event => sections.event.map_or_else(
            || render_missing_detail(kind, cx),
            |section| render_event_detail(section, slots, cx).into_any_element(),
        ),
        ShowCardKind::Stream => sections.stream.map_or_else(
            || render_missing_detail(kind, cx),
            |section| render_stream_detail(section, slots, cx).into_any_element(),
        ),
    }
}

fn render_source_detail(
    section: SourceSectionDisplay,
    slots: &ShowDetailPanelSlots,
    cx: &App,
) -> impl IntoElement {
    let mut detail = detail_stack(cx)
        .child(render_detail_row(
            "Host",
            section.host_name,
            Some(section.summary),
            cx,
        ))
        .child(render_detail_row(
            "Reachability",
            section.reachability.label,
            Some(section.reachability.detail.to_owned()),
            cx,
        ));

    detail = if let Some(readiness) = section.readiness {
        detail
            .child(render_detail_row(
                "Readiness",
                readiness.count_label,
                Some(readiness.detail),
                cx,
            ))
            .child(render_controls(
                vec![
                    source_action_button(readiness.action, slots.open_readiness.clone())
                        .into_any_element(),
                ],
                cx,
            ))
    } else {
        detail.child(render_detail_row(
            "Readiness",
            "Not checked",
            Some("No readiness snapshot is available.".to_owned()),
            cx,
        ))
    };

    detail
}

fn render_publisher_detail(
    section: PublisherSectionDisplay,
    slots: &ShowDetailPanelSlots,
    cx: &App,
) -> impl IntoElement {
    let mut detail =
        detail_stack(cx).child(render_detail_row("Summary", section.summary, None, cx));

    for service in section.services {
        detail = detail.child(render_publisher_service(service, slots, cx));
    }

    detail
}

fn render_publisher_service(
    service: PublisherServiceDisplay,
    slots: &ShowDetailPanelSlots,
    cx: &App,
) -> impl IntoElement {
    let PublisherServiceDisplay {
        role,
        id,
        label,
        unit_name,
        state,
        actions,
        logs,
    } = service;

    div()
        .id(SharedString::from(id))
        .flex()
        .flex_col()
        .gap(Spacing::XS.scaled(cx))
        .py(Spacing::SM.scaled(cx))
        .border_t_1()
        .border_color(color(cx, SemanticColor::Separator))
        .child(render_detail_row("Service", label, Some(unit_name), cx))
        .child(render_detail_row(
            "State",
            state.label(),
            Some(state.detail()),
            cx,
        ))
        .child(render_controls(
            vec![
                publisher_action_button(actions.start, role, slots.publisher_start.clone())
                    .into_any_element(),
                publisher_action_button(actions.stop, role, slots.publisher_stop.clone())
                    .into_any_element(),
                publisher_action_button(actions.reset, role, slots.publisher_reset.clone())
                    .into_any_element(),
                publisher_action_button(logs.action, role, slots.publisher_open_logs.clone())
                    .into_any_element(),
            ],
            cx,
        ))
}

fn render_event_detail(
    section: EventSectionDisplay,
    slots: &ShowDetailPanelSlots,
    cx: &App,
) -> impl IntoElement {
    let mut detail = detail_stack(cx)
        .child(render_detail_row(
            "Event",
            section.event.label,
            Some(section.summary),
            cx,
        ))
        .child(render_detail_row(
            "State",
            section.event.state.label,
            Some(section.event.state.detail.to_owned()),
            cx,
        ))
        .child(render_detail_row(
            "Target",
            section.target.label,
            Some(section.target.detail),
            cx,
        ));

    if let Some(event_id) = section.event.event_id {
        detail = detail.child(render_detail_row("Event ID", event_id, None, cx));
    }
    if let Some(endpoint) = section.event.endpoint {
        detail = detail.child(render_detail_row("Endpoint", endpoint, None, cx));
    }
    if let Some(token_path) = section.event.token_path {
        detail = detail.child(render_detail_row("Token", token_path, None, cx));
    }
    if let Some(feed_tag) = section.feed_tag {
        detail = detail.child(render_code_block(
            "show-event-feed-tag",
            "Feed tag",
            feed_tag,
            cx,
        ));
    }
    if let Some(hint) = section.hint {
        detail = detail.child(render_detail_row("Hint", hint, None, cx));
    }

    detail.child(render_controls(
        vec![
            event_action_button(section.actions.attach, slots.event_attach.clone())
                .into_any_element(),
            event_action_button(section.actions.detach, slots.event_detach.clone())
                .into_any_element(),
        ],
        cx,
    ))
}

fn render_stream_detail(
    section: StreamSectionDisplay,
    slots: &ShowDetailPanelSlots,
    cx: &App,
) -> impl IntoElement {
    let mut detail = detail_stack(cx)
        .child(render_detail_row("Server", section.server_label, None, cx))
        .child(render_detail_row(
            "Connection",
            section.connection.label,
            Some(section.connection.detail.to_owned()),
            cx,
        ))
        .child(render_detail_row(
            "Signal",
            section.signal.label,
            Some(section.signal.detail.to_owned()),
            cx,
        ))
        .child(render_detail_row(
            "Recording",
            section.recording.label,
            Some(section.recording.detail.to_owned()),
            cx,
        ))
        .child(render_detail_row(
            "Listeners",
            section.listeners.label,
            Some(section.listeners.detail.to_owned()),
            cx,
        ));

    if let Some(elapsed) = section.recording.elapsed_label {
        detail = detail.child(render_detail_row("Recording elapsed", elapsed, None, cx));
    }
    if let Some(path) = section.recording.path {
        detail = detail.child(render_detail_row("Recording file", path, None, cx));
    }
    if let Some(song) = section.encoder_song {
        detail = detail.child(render_detail_row("Encoder song", song, None, cx));
    }
    if let Some(elapsed) = section.stream_elapsed_label {
        detail = detail.child(render_detail_row("Stream elapsed", elapsed, None, cx));
    }
    if let Some(actions) = section.actions {
        detail = detail.child(render_controls(
            vec![
                stream_action_button(actions.connect, slots.stream_connect.clone())
                    .into_any_element(),
                stream_action_button(actions.disconnect, slots.stream_disconnect.clone())
                    .into_any_element(),
            ],
            cx,
        ));
    }

    detail
}

fn render_missing_detail(kind: ShowCardKind, cx: &App) -> AnyElement {
    detail_stack(cx)
        .child(render_detail_row(
            kind.title(),
            "Detail unavailable",
            Some("The selected card no longer has current data.".to_owned()),
            cx,
        ))
        .into_any_element()
}

fn detail_stack(cx: &App) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .min_w_0()
        .p(Spacing::MD.scaled(cx))
        .gap(Spacing::SM.scaled(cx))
}

fn render_detail_row(
    label: &'static str,
    value: impl Into<SharedString>,
    detail: Option<String>,
    cx: &App,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .min_w_0()
        .gap(Spacing::XXS.scaled(cx))
        .child(
            div()
                .text_size(FontSize::Micro.scaled(cx))
                .font_weight(FontWeight::MEDIUM)
                .text_color(color(cx, SemanticColor::TertiaryLabel))
                .child(SharedString::from(label)),
        )
        .child(
            div()
                .overflow_hidden()
                .text_size(FontSize::Body.scaled(cx))
                .text_color(color(cx, SemanticColor::Label))
                .child(value.into()),
        )
        .when_some(detail, |el, detail| {
            el.child(
                div()
                    .min_w_0()
                    .text_size(FontSize::Micro.scaled(cx))
                    .text_color(color(cx, SemanticColor::SecondaryLabel))
                    .child(SharedString::from(detail)),
            )
        })
}

fn render_code_block(
    id: &'static str,
    label: &'static str,
    value: String,
    cx: &App,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .min_w_0()
        .gap(Spacing::XXS.scaled(cx))
        .child(
            div()
                .text_size(FontSize::Micro.scaled(cx))
                .font_weight(FontWeight::MEDIUM)
                .text_color(color(cx, SemanticColor::TertiaryLabel))
                .child(SharedString::from(label)),
        )
        .child(
            div()
                .id(id)
                .min_w_0()
                .overflow_x_scroll()
                .rounded(Radius::SM.scaled(cx))
                .bg(color(cx, SemanticColor::TertiarySystemBackground))
                .p(Spacing::SM.scaled(cx))
                .text_size(FontSize::Micro.scaled(cx))
                .text_color(color(cx, SemanticColor::SecondaryLabel))
                .child(SharedString::from(value)),
        )
}

fn render_controls(controls: Vec<AnyElement>, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(Spacing::SM.scaled(cx))
        .flex_wrap()
        .children(controls)
}

fn panel_icon_button(
    display: ShowPanelActionDisplay,
    icon: IconName,
    handler: Option<PanelClickHandler>,
) -> Button {
    let disabled = display.disabled() || handler.is_none();
    let mut button = Button::styled(display.id, ControlStyle::ToolbarIcon)
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

fn panel_label_button(
    display: ShowPanelActionDisplay,
    icon: IconName,
    handler: Option<PanelClickHandler>,
) -> Button {
    let disabled = display.disabled() || handler.is_none();
    let mut button = Button::styled(display.id, ControlStyle::RowAction)
        .leading_icon(icon)
        .label(display.label)
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

fn source_action_button(
    display: SourceReadinessActionDisplay,
    handler: Option<PanelClickHandler>,
) -> Button {
    let disabled = display.disabled() || handler.is_none();
    detail_action_button(
        display.id,
        display.label,
        display.a11y_label,
        disabled,
        handler,
    )
}

fn event_action_button(display: EventActionDisplay, handler: Option<PanelClickHandler>) -> Button {
    let disabled = display.disabled() || handler.is_none();
    detail_action_button(
        display.id,
        display.label,
        display.a11y_label,
        disabled,
        handler,
    )
}

fn stream_action_button(
    display: StreamActionDisplay,
    handler: Option<PanelClickHandler>,
) -> Button {
    let disabled = display.disabled() || handler.is_none();
    detail_action_button(
        display.id,
        display.label,
        display.a11y_label,
        disabled,
        handler,
    )
}

fn publisher_action_button(
    display: PublisherActionDisplay,
    role: PublisherServiceRole,
    handler: Option<PublisherClickHandler>,
) -> Button {
    let disabled = display.disabled() || handler.is_none();
    let mut button = Button::styled(SharedString::from(display.id), ControlStyle::RowAction)
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

fn detail_action_button(
    id: impl Into<gpui::ElementId>,
    label: &'static str,
    a11y_label: String,
    disabled: bool,
    handler: Option<PanelClickHandler>,
) -> Button {
    let mut button = Button::styled(id, ControlStyle::RowAction)
        .label(label)
        .a11y_label(a11y_label.clone())
        .tooltip(a11y_label)
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
