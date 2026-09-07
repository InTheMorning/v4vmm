# Broadcast shell patterns to preserve - 2026-09-07

This note records patterns from `src/ui/shells/broadcast.rs` before ADR 0060
task 001 removes the `Broadcast` workspace frame. It is reference material for
the later `Show` surface, not a new binding rule. ADR 0060 remains the owner.

## Feed tag and copy action

Pattern: the event display may carry a complete `podcast:liveValue` element,
and the shell renders it as wrapped text with a copy action. The shell receives
the complete string from the view model; it does not assemble XML or infer the
event identifier.

Excerpt:

```rust
if let Some(feed_tag) = event.feed_tag {
    body = body.child(render_feed_tag(
        feed_tag,
        event.copy_feed_tag,
        copy_feed_tag,
        cx,
    ));
}
```

```rust
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
```

ADR 0060 rule served: `Show` is the broadcasting surface, while ADR 0060
supersedes only the `Broadcast` workspace frame and retains the rest of ADR
0059. The future `Show` surface still needs the listener-facing live-value feed
tag affordance, but it must not live in a `Broadcast` frame.

## Section composition and three-section layout

Pattern: the shell composed three fixed-order operational sections: `Source`,
`Publisher`, and `Event`. Each section used the same helper so section chrome,
spacing, elevation, and header placement stayed consistent.

Excerpt:

```rust
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
    ));
```

```rust
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
```

ADR 0060 rule served: broadcasting belongs to `Show`, and `Show` is a screen
mount because it needs a task-specific layout instead of workspace frame lanes.
The three operational groups may inform that screen layout, but they are not
app sections and they should not reappear inside the `Music` curation surface.

## Slots builder callback pattern

Pattern: callbacks were supplied as optional slots through a builder. The shell
owned layout only; the app adapter owned command dispatch. Button labels,
availability, IDs, accessibility labels, and tooltips came from the display
contract before rendering.

Excerpt:

```rust
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
```

```rust
pub(crate) fn on_copy_feed_tag(
    mut self,
    handler: impl Fn(&str, &ClickEvent, &mut Window, &mut App) + 'static,
) -> Self {
    self.copy_feed_tag = Some(Rc::new(handler));
    self
}
```

```rust
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
```

ADR 0060 rule served: mode-aware readiness and command availability belong to a
view model, not to renderer conditions. This pattern keeps the future `Show`
renderer passive while still letting the screen mount wire actions through the
composition root.

## Empty-state handling

Pattern: each operational section could append a view-model supplied empty
state. The shell rendered title and message only; it did not invent fallback
copy or convert missing operational capacity into a local error string.

Excerpt:

```rust
if let Some(empty) = publisher.empty_state {
    body = body.child(render_empty_state(&empty, cx));
}
```

```rust
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
```

ADR 0060 rule served: `Music` and `Settings` render no operational surface when
no show is active, and sections that cannot apply to the selected production
mode are absent rather than disabled. Empty states remain useful inside the
active `Show` surface, where the operator is already doing operational work,
but the old wall of unavailable broadcast states must not be carried into
curation.
