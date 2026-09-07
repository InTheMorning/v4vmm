//! Broadcast workspace-frame shell.
//!
//! ADR 0059 makes this pane a control surface for listener truth, publisher
//! state, and local event registry affordances. The shell consumes only the
//! GPUI-free page VM and callback slots supplied by the app adapter.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    div, prelude::*, App, ClickEvent, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window,
};

use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::{
    Button, Label, LabelVariant, MultilineText, SectionHeader, Surface, SurfaceElevation,
};
use crate::ui::tokens::{resolve_color, FontSize, Radius, SemanticColor, Spacing};
use crate::view_models::broadcast::{
    ActionDisplay, BroadcastPageVm, EventSectionDisplay, EventState, PublisherSectionDisplay,
    SectionEmptyStateDisplay, ServiceState, SourceSectionDisplay, SourceState,
};

type BroadcastClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;
type BroadcastFeedTagHandler = Rc<dyn Fn(&str, &ClickEvent, &mut Window, &mut App) + 'static>;

/// Callback slots supplied by the application-owned Broadcast frame.
#[derive(Default)]
#[must_use]
pub(crate) struct BroadcastSlots {
    create_event: Option<BroadcastClickHandler>,
    resume_event: Option<BroadcastClickHandler>,
    forget_event: Option<BroadcastClickHandler>,
    copy_feed_tag: Option<BroadcastFeedTagHandler>,
    start_service: Option<BroadcastClickHandler>,
    stop_service: Option<BroadcastClickHandler>,
    reset_service: Option<BroadcastClickHandler>,
    open_logs: Option<BroadcastClickHandler>,
    select_source: Option<BroadcastClickHandler>,
}

impl BroadcastSlots {
    /// Creates empty Broadcast-frame slots.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Supplies the create-event callback.
    pub(crate) fn on_create_event(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.create_event = Some(Rc::new(handler));
        self
    }

    /// Supplies the resume-event callback.
    pub(crate) fn on_resume_event(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.resume_event = Some(Rc::new(handler));
        self
    }

    /// Supplies the forget-event callback.
    pub(crate) fn on_forget_event(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.forget_event = Some(Rc::new(handler));
        self
    }

    /// Supplies the copy-feed-tag callback.
    pub(crate) fn on_copy_feed_tag(
        mut self,
        handler: impl Fn(&str, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.copy_feed_tag = Some(Rc::new(handler));
        self
    }

    /// Supplies the start-service callback.
    pub(crate) fn on_start_service(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.start_service = Some(Rc::new(handler));
        self
    }

    /// Supplies the stop-service callback.
    pub(crate) fn on_stop_service(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.stop_service = Some(Rc::new(handler));
        self
    }

    /// Supplies the reset-service callback.
    pub(crate) fn on_reset_service(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.reset_service = Some(Rc::new(handler));
        self
    }

    /// Supplies the open-logs callback.
    pub(crate) fn on_open_logs(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.open_logs = Some(Rc::new(handler));
        self
    }

    /// Supplies the select-source callback.
    pub(crate) fn on_select_source(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.select_source = Some(Rc::new(handler));
        self
    }
}

/// Broadcast frame shell element.
#[derive(IntoElement)]
#[must_use]
pub(crate) struct BroadcastShell {
    vm: BroadcastPageVm,
    slots: BroadcastSlots,
}

/// Creates the Broadcast frame shell.
pub(crate) fn render_broadcast(vm: BroadcastPageVm, slots: BroadcastSlots) -> BroadcastShell {
    BroadcastShell { vm, slots }
}

impl RenderOnce for BroadcastShell {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let BroadcastPageVm {
            source,
            publisher,
            event,
            empty_label: _,
        } = self.vm;
        let BroadcastSlots {
            create_event,
            resume_event,
            forget_event,
            copy_feed_tag,
            start_service,
            stop_service,
            reset_service,
            open_logs,
            select_source,
        } = self.slots;

        div()
            .size_full()
            .flex()
            .flex_col()
            .min_h_0()
            .min_w_0()
            .overflow_hidden()
            .child(
                div()
                    .id("broadcast-sections")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_y_scroll()
                    .p(Spacing::MD.scaled(cx))
                    .gap(Spacing::MD.scaled(cx))
                    .child(render_source_section(source, select_source, cx))
                    .child(render_publisher_section(
                        publisher,
                        start_service,
                        stop_service,
                        reset_service,
                        open_logs,
                        cx,
                    ))
                    .child(render_event_section(
                        event,
                        create_event,
                        resume_event,
                        forget_event,
                        copy_feed_tag,
                        cx,
                    )),
            )
    }
}

fn render_source_section(
    source: SourceSectionDisplay,
    select_source: Option<BroadcastClickHandler>,
    cx: &App,
) -> impl IntoElement {
    let _reserved_select_source_slot = select_source;
    let (state_label, state_color) = source_state_presentation(source.state);
    let mut body = section_body(cx)
        .child(render_value_row("Name", source.source_name, cx))
        .child(render_value_row("Kind", source.source_kind_label, cx))
        .child(render_value_row("Host", source.host_label, cx))
        .child(render_state_row("Status", state_label, state_color, cx));

    if let Some(readiness) = source.readiness_label {
        body = body.child(render_value_row("Readiness", readiness, cx));
    }

    if let Some(title) = source.current_track_title {
        body = body.child(render_value_row("Track", title, cx));
    }

    if let Some(artist) = source.current_track_artist {
        body = body.child(render_value_row("Artist", artist, cx));
    }

    if let Some(empty) = source.empty_state {
        body = body.child(render_empty_state(&empty, cx));
    }

    render_section("Source", body, cx)
}

fn render_publisher_section(
    publisher: PublisherSectionDisplay,
    start_service: Option<BroadcastClickHandler>,
    stop_service: Option<BroadcastClickHandler>,
    reset_service: Option<BroadcastClickHandler>,
    open_logs: Option<BroadcastClickHandler>,
    cx: &App,
) -> impl IntoElement {
    let (publisher_state, publisher_color) = service_state_presentation(publisher.publisher_state);
    let (producer_state, producer_color) = service_state_presentation(publisher.producer_state);
    let mut body = section_body(cx)
        .child(
            action_row(cx)
                .child(render_action_button(
                    publisher.start,
                    ControlStyle::Primary,
                    start_service,
                ))
                .child(render_action_button(
                    publisher.stop,
                    ControlStyle::Secondary,
                    stop_service,
                ))
                .child(render_action_button(
                    publisher.reset,
                    ControlStyle::Secondary,
                    reset_service,
                ))
                .child(render_action_button(
                    publisher.logs,
                    ControlStyle::Ghost,
                    open_logs,
                )),
        )
        .child(render_value_row(
            "Publisher",
            publisher.publisher_unit_name,
            cx,
        ))
        .child(render_state_row(
            "Publisher state",
            publisher_state,
            publisher_color,
            cx,
        ))
        .child(render_value_row(
            "Producer",
            publisher.producer_unit_name,
            cx,
        ))
        .child(render_state_row(
            "Producer state",
            producer_state,
            producer_color,
            cx,
        ));

    if let Some(reason) = publisher.failure_reason_label {
        body = body.child(render_value_row("Failure", reason, cx));
    }

    if let Some(empty) = publisher.empty_state {
        body = body.child(render_empty_state(&empty, cx));
    }

    render_section("Publisher", body, cx)
}

fn render_event_section(
    event: EventSectionDisplay,
    create_event: Option<BroadcastClickHandler>,
    resume_event: Option<BroadcastClickHandler>,
    forget_event: Option<BroadcastClickHandler>,
    copy_feed_tag: Option<BroadcastFeedTagHandler>,
    cx: &App,
) -> impl IntoElement {
    let (state_label, state_color) = event_state_presentation(event.state);
    let mut body = section_body(cx)
        .child(
            action_row(cx)
                .child(render_action_button(
                    event.create,
                    ControlStyle::Primary,
                    create_event,
                ))
                .child(render_action_button(
                    event.resume,
                    ControlStyle::Secondary,
                    resume_event,
                ))
                .child(render_action_button(
                    event.forget,
                    ControlStyle::Ghost,
                    forget_event,
                )),
        )
        .child(render_state_row("Status", state_label, state_color, cx))
        .child(render_value_row(
            "Listener truth",
            event.listener_truth_line,
            cx,
        ));

    if let Some(label) = event.event_label {
        body = body.child(render_value_row("Event", label, cx));
    }

    if let Some(identifier) = event.event_identifier {
        body = body.child(render_value_row("Identifier", identifier, cx));
    }

    if let Some(endpoint) = event.endpoint {
        body = body.child(render_value_row("Relay", endpoint, cx));
    }

    if let Some(token_path) = event.token_path {
        body = body.child(render_value_row("Token file", token_path, cx));
    }

    if let Some(feed_tag) = event.feed_tag {
        body = body.child(render_feed_tag(
            feed_tag,
            event.copy_feed_tag,
            copy_feed_tag,
            cx,
        ));
    }

    if let Some(empty) = event.empty_state {
        body = body.child(render_empty_state(&empty, cx));
    }

    render_section("Event", body, cx)
}

fn render_section(title: &'static str, body: impl IntoElement, cx: &App) -> impl IntoElement {
    Surface::new(SurfaceElevation::Sunken)
        .padding(Spacing::MD)
        .radius(Radius::MD)
        .child(
            div()
                .flex()
                .flex_col()
                .gap(Spacing::SM.scaled(cx))
                .child(SectionHeader::new(title))
                .child(body),
        )
}

fn section_body(cx: &App) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .min_w_0()
        .gap(Spacing::XS.scaled(cx))
}

fn action_row(cx: &App) -> gpui::Div {
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(Spacing::XS.scaled(cx))
}

fn render_action_button(
    display: ActionDisplay,
    style: ControlStyle,
    handler: Option<BroadcastClickHandler>,
) -> Button {
    let disabled = display.chrome.disabled;
    let mut button = Button::styled(SharedString::from(display.chrome.id), style)
        .label(display.label)
        .a11y_label(display.chrome.a11y_label)
        .tooltip(display.chrome.a11y_label)
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

fn render_copy_button(
    display: ActionDisplay,
    feed_tag: &str,
    handler: Option<BroadcastFeedTagHandler>,
) -> Button {
    let disabled = display.chrome.disabled;
    let payload = feed_tag.to_owned();
    let mut button = Button::styled(SharedString::from(display.chrome.id), ControlStyle::Ghost)
        .label(display.label)
        .a11y_label(display.chrome.a11y_label)
        .tooltip(display.chrome.a11y_label)
        .disabled(disabled);

    if !disabled {
        if let Some(handler) = handler {
            button = button.on_click(move |event, window, cx| {
                handler(&payload, event, window, cx);
            });
        }
    }

    button
}

fn render_value_row(
    label: &'static str,
    value: impl Into<SharedString>,
    cx: &App,
) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(Spacing::SM.scaled(cx))
        .min_w_0()
        .child(
            Label::new(label)
                .variant(LabelVariant::Caption)
                .color(SemanticColor::TertiaryLabel),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(Label::new(value).variant(LabelVariant::Caption).truncated()),
        )
}

fn render_state_row(
    label: &'static str,
    value: &'static str,
    color: SemanticColor,
    cx: &App,
) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(Spacing::SM.scaled(cx))
        .min_w_0()
        .child(
            Label::new(label)
                .variant(LabelVariant::Caption)
                .color(SemanticColor::TertiaryLabel),
        )
        .child(
            Label::new(value)
                .variant(LabelVariant::Caption)
                .color(color),
        )
}

fn render_feed_tag(
    feed_tag: String,
    copy_display: ActionDisplay,
    copy_feed_tag: Option<BroadcastFeedTagHandler>,
    cx: &App,
) -> impl IntoElement {
    Surface::new(SurfaceElevation::Sunken)
        .padding(Spacing::SM)
        .radius(Radius::SM)
        .child(
            div()
                .flex()
                .flex_col()
                .gap(Spacing::XS.scaled(cx))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .gap(Spacing::SM.scaled(cx))
                        .child(
                            Label::new("Feed tag")
                                .variant(LabelVariant::Caption)
                                .color(SemanticColor::TertiaryLabel),
                        )
                        .child(render_copy_button(copy_display, &feed_tag, copy_feed_tag)),
                )
                .child(
                    MultilineText::new(feed_tag)
                        .size(FontSize::Caption)
                        .color(SemanticColor::Label)
                        .wrap_lines(),
                ),
        )
}

fn render_empty_state(empty: &SectionEmptyStateDisplay, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .min_w_0()
        .gap(Spacing::XXS.scaled(cx))
        .border_l_1()
        .border_color(resolve_color(cx, SemanticColor::Separator, None))
        .pl(Spacing::SM.scaled(cx))
        .child(Label::new(empty.title).variant(LabelVariant::Headline))
        .child(
            Label::new(empty.message)
                .variant(LabelVariant::Caption)
                .color(SemanticColor::SecondaryLabel),
        )
}

const fn source_state_presentation(state: SourceState) -> (&'static str, SemanticColor) {
    match state {
        SourceState::Unknown => ("Unknown", SemanticColor::TertiaryLabel),
        SourceState::Idle => ("Idle", SemanticColor::WarningLabel),
        SourceState::Playing => ("Playing", SemanticColor::SuccessLabel),
        SourceState::Paused => ("Paused", SemanticColor::WarningLabel),
        SourceState::NotReachable => ("Not reachable", SemanticColor::DangerLabel),
    }
}

const fn service_state_presentation(state: ServiceState) -> (&'static str, SemanticColor) {
    match state {
        ServiceState::Unknown => ("Unknown", SemanticColor::TertiaryLabel),
        ServiceState::Active => ("Active", SemanticColor::SuccessLabel),
        ServiceState::Inactive => ("Inactive", SemanticColor::WarningLabel),
        ServiceState::Failed => ("Failed", SemanticColor::DangerLabel),
        ServiceState::NotInstalled => ("Not installed", SemanticColor::DangerLabel),
        ServiceState::NotReachable => ("Not reachable", SemanticColor::DangerLabel),
    }
}

const fn event_state_presentation(state: EventState) -> (&'static str, SemanticColor) {
    match state {
        EventState::None => ("None", SemanticColor::TertiaryLabel),
        EventState::Unknown => ("Unknown", SemanticColor::TertiaryLabel),
        EventState::Live => ("Live", SemanticColor::SuccessLabel),
        EventState::Dead => ("Dead", SemanticColor::DangerLabel),
    }
}
