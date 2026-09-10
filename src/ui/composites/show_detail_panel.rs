//! Show side-panel composite.
//!
//! ADR 0063 moves card detail and the cuelist into one trailing panel. This
//! composite owns panel geometry, scrolling, detail row layout, and the action
//! buttons for section detail. The caller supplies already-projected display
//! state and command slots.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    div, prelude::*, AnyElement, App, ClickEvent, ClipboardItem, FontWeight, InteractiveElement,
    IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window,
};

use crate::ui::control_styles::ControlStyle;
use crate::ui::icons::IconName;
use crate::ui::primitives::{
    status_badge::render_state_badge, Button, ContextMenu, ContextMenuItem, ContextMenuItemDisplay,
    ContextMenuScope,
};
use crate::ui::shells::queue_now_playing::render_queue_cuelist;
use crate::ui::tokens::{color, FontSize, SemanticColor, Size, Spacing};
use crate::view_models::queue_now_playing::QueueNowPlayingPageVm;
use crate::view_models::show::{
    EventActionDisplay, EventControlDisplay, EventControlIntent, EventSectionDisplay,
    PublisherActionDisplay, PublisherSectionDisplay, PublisherServiceDisplay, PublisherServiceRole,
    ShowCardKind, ShowItemBadge, ShowPanelActionDisplay, ShowPanelChromeDisplay, ShowPanelMode,
    SourceReadinessActionDisplay, SourceSectionDisplay, StreamActionDisplay, StreamSectionDisplay,
};

type EventSelectHandler = Rc<dyn Fn(String, &mut Window, &mut App)>;
type EventControlHandler = Rc<dyn Fn(EventControlIntent, &ClickEvent, &mut Window, &mut App)>;
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
    event_select: Option<EventSelectHandler>,
    event_logs: Option<PanelClickHandler>,
    event_control: Option<EventControlHandler>,
    stream_connect: Option<PanelClickHandler>,
    stream_disconnect: Option<PanelClickHandler>,
}

impl ShowDetailPanelSlots {
    pub(crate) fn on_select_event(
        mut self,
        handler: impl Fn(String, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.event_select = Some(Rc::new(handler));
        self
    }
    pub(crate) fn on_open_event_logs(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.event_logs = Some(Rc::new(handler));
        self
    }
    pub(crate) fn on_event_control(
        mut self,
        handler: impl Fn(EventControlIntent, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.event_control = Some(Rc::new(handler));
        self
    }

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
            stream,
        } = self.display;
        let sections = ShowDetailSections {
            source,
            publisher,
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
    let mut detail = detail_stack(cx);
    if let Some(event) = section.event {
        detail = detail.child(render_event_detail(event, slots, cx));
    }
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
        badge,
        state: _,
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
        .child(render_item_header(label, badge, None, cx))
        .child(compact_detail(unit_name, cx))
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

fn render_item_header(
    label: &'static str,
    badge: ShowItemBadge,
    activity: Option<&'static str>,
    cx: &App,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(Spacing::SM.scaled(cx))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_size(FontSize::Body.scaled(cx))
                .font_weight(FontWeight::MEDIUM)
                .child(label),
        )
        .children(activity.map(|text| {
            compact_detail(text.to_owned(), cx)
                .flex_shrink_0()
                .whitespace_nowrap()
        }))
        .child(render_state_badge(badge.label.to_owned(), badge.kind, cx))
}

fn compact_detail(text: String, cx: &App) -> gpui::Div {
    div()
        .min_w_0()
        .text_size(FontSize::Micro.scaled(cx))
        .text_color(color(cx, SemanticColor::SecondaryLabel))
        .child(SharedString::from(text))
}

fn event_handler(
    intent: EventControlIntent,
    slots: &ShowDetailPanelSlots,
) -> Option<PanelClickHandler> {
    slots.event_control.clone().map(|handler| {
        Rc::new(
            move |event: &ClickEvent, window: &mut Window, cx: &mut App| {
                handler(intent, event, window, cx);
            },
        ) as PanelClickHandler
    })
}

fn event_menu_item(
    control: EventControlDisplay,
    slots: &ShowDetailPanelSlots,
    feed_tag: Option<String>,
) -> ContextMenuItem {
    let disabled = control.action.disabled();
    let display = ContextMenuItemDisplay {
        id: control.action.id.into(),
        label: control.action.label.into(),
        a11y_label: control.action.a11y_label.into(),
        destructive: false,
        disabled,
    };
    let handler = event_handler(control.intent, slots);
    ContextMenuItem::new(display).on_select(move |window, cx| {
        if control.intent == EventControlIntent::CopyFeedTag {
            if let Some(tag) = &feed_tag {
                cx.write_to_clipboard(ClipboardItem::new_string(tag.clone()));
            }
        } else if let Some(handler) = &handler {
            handler(&ClickEvent::default(), window, cx);
        }
    })
}

fn render_event_detail(
    section: EventSectionDisplay,
    slots: &ShowDetailPanelSlots,
    cx: &App,
) -> impl IntoElement {
    let picker_disabled = section.picker.availability.disabled();
    let mut picker = ContextMenu::new(
        "show-event-picker",
        ContextMenuScope::WorkspaceFrame,
        section.picker.a11y_label,
    )
    .trigger_label(section.picker.label)
    .trigger_icon(IconName::ChevronDown)
    .disabled(picker_disabled);
    for entry in section.picker.entries {
        let handler = slots.event_select.clone();
        picker = picker.item(
            ContextMenuItem::new(ContextMenuItemDisplay {
                id: SharedString::from(entry.event_id.clone()),
                label: entry.label.into(),
                a11y_label: entry.a11y_label.into(),
                destructive: false,
                disabled: picker_disabled,
            })
            .description(entry.detail)
            .on_select(move |window, cx| {
                if let Some(handler) = &handler {
                    handler(entry.event_id.clone(), window, cx);
                }
            }),
        );
    }
    let overflow = ContextMenu::new(
        "show-event-actions",
        ContextMenuScope::WorkspaceFrame,
        "More event actions",
    )
    .trigger_label("More")
    .items(
        section
            .overflow
            .into_iter()
            .map(|control| event_menu_item(control, slots, section.feed_tag.clone())),
    );
    let mut controls = Vec::new();
    if let Some(primary) = section.primary {
        controls.push(
            event_action_button(primary.action, event_handler(primary.intent, slots))
                .into_any_element(),
        );
    }
    controls.push(event_action_button(section.logs, slots.event_logs.clone()).into_any_element());
    controls.push(overflow.into_any_element());
    div()
        .flex()
        .flex_col()
        .min_w_0()
        .gap(Spacing::XS.scaled(cx))
        .child(render_item_header(
            "Event",
            section.badge,
            section.activity,
            cx,
        ))
        .child(picker)
        .child(render_controls(controls, cx))
        .children(section.hint.map(|hint| compact_detail(hint, cx)))
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
