//! Queue/Now Playing workspace-frame shell.
//!
//! ADR 0046 Phase 4 gives playback status and transport controls their own
//! surface. The global toolbar remains a compact status affordance.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    div, prelude::*, App, ClickEvent, FontWeight, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window,
};

use crate::ui::control_styles::ControlStyle;
use crate::ui::icons::{Icon, IconName, IconSize};
use crate::ui::primitives::{Button, Tooltip};
use crate::ui::tokens::{color, FontSize, Radius, SemanticColor, Size, Spacing};
use crate::view_models::queue_now_playing::{
    QueueNowPlayingPageVm, QueueRowDisplay, TransportDisplay, TransportState,
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

/// Queue cuelist shell element.
#[derive(IntoElement)]
#[must_use]
pub(crate) struct QueueCuelistShell {
    vm: QueueNowPlayingPageVm,
}

/// Queue transport shell element.
#[derive(IntoElement)]
#[must_use]
pub(crate) struct QueueTransportShell {
    transport: TransportDisplay,
    slots: QueueNowPlayingSlots,
}

/// Creates a queue-only shell for panel cuelist mode.
pub(crate) fn render_queue_cuelist(vm: QueueNowPlayingPageVm) -> QueueCuelistShell {
    QueueCuelistShell { vm }
}

/// Creates a transport-only shell for the Show surface.
pub(crate) fn render_queue_transport(
    transport: TransportDisplay,
    slots: QueueNowPlayingSlots,
) -> QueueTransportShell {
    QueueTransportShell { transport, slots }
}

impl RenderOnce for QueueCuelistShell {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let QueueNowPlayingPageVm {
            rows, empty_label, ..
        } = self.vm;

        render_queue_list(rows, empty_label, cx)
    }
}

impl RenderOnce for QueueTransportShell {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        render_control_deck(self.transport, self.slots, cx)
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
        .debug_selector(|| "queue-now-playing-row".to_owned())
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
    slots: QueueNowPlayingSlots,
    cx: &mut App,
) -> impl IntoElement {
    let border = color(cx, SemanticColor::Separator);

    div()
        .flex()
        .flex_col()
        .flex_shrink_0()
        .gap(Spacing::MD.scaled(cx))
        .border_t_1()
        .border_color(border)
        .p(Spacing::MD.scaled(cx))
        .child(render_transport(transport, slots, cx))
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

// ---------------------------------------------------------------------------
// ADR 0039 task 002 M1: fixed geometry does not depend on string length.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use gpui::{div, prelude::*, px, TestAppContext};

    use super::render_queue_row;
    use crate::view_models::queue_now_playing::QueueRowDisplay;

    fn row(
        title: &str,
        artist: Option<&str>,
        duration: Option<&str>,
        now_playing: bool,
    ) -> QueueRowDisplay {
        QueueRowDisplay {
            track_id: 1,
            id: "queue-row-1".to_owned(),
            title: title.to_owned(),
            artist: artist.map(str::to_owned),
            duration_label: duration.map(str::to_owned),
            now_playing,
            a11y_label: title.to_owned(),
        }
    }

    struct RowHeightTest {
        display: QueueRowDisplay,
    }

    impl gpui::Render for RowHeightTest {
        fn render(
            &mut self,
            _window: &mut gpui::Window,
            cx: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            div()
                .w(px(320.0))
                .child(render_queue_row(self.display.clone(), cx))
        }
    }

    /// ADR 0039 task 002 M1: the queue row is a floor
    /// (`.min_h(Size::RowLg.scaled(cx))`), not a cap, but its height must
    /// still stay independent of title/artist string length and unaffected
    /// by which optional slots (artist, duration, now-playing) are present —
    /// title/artist clip via `.truncate()` inside their `flex_1()` ancestor
    /// rather than growing the row.
    #[gpui::test]
    fn adr_0039_queue_row_height_is_independent_of_title_length(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let short = row("A", None, None, false);
        let (view, cx) = cx.add_window_view(|_, _| RowHeightTest {
            display: short.clone(),
        });

        let long_title = "A very long queue row title that would otherwise wrap this row".repeat(3);
        let cases = [
            short.clone(),
            row(&long_title, None, None, false),
            row("A", Some("Artist"), None, false),
            row(
                &long_title,
                Some("A very long artist name too"),
                None,
                false,
            ),
            row("A", Some("Artist"), Some("3:45"), false),
            row(
                &long_title,
                Some("A very long artist name too"),
                Some("3:45"),
                true,
            ),
        ];

        let mut heights = Vec::new();
        for display in cases {
            view.update(cx, |this, cx| {
                this.display = display;
                cx.notify();
            });
            cx.update(|window, cx| {
                let _ = window.draw(cx);
            });
            let bounds = cx
                .debug_bounds("queue-now-playing-row")
                .expect("queue row bounds recorded");
            heights.push(bounds.size.height);
        }

        // Toggling `now_playing`, `artist` or `duration_label` legitimately
        // changes the row (a highlighted background, an extra line, a
        // trailing label) — those are named optional states, not a
        // string-length dependency. What must never move is the height
        // *within* a fixed set of present slots when only string length
        // changes: cases 0 vs 1 (title only) and cases 2 vs 3 (title+artist).
        assert_eq!(
            heights[0], heights[1],
            "queue row height changed with title length alone: {heights:?}"
        );
        assert_eq!(
            heights[2], heights[3],
            "queue row height changed with title/artist length alone: {heights:?}"
        );
    }
}
