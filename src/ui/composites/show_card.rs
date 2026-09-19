//! Show dashboard card composite.
//!
//! ADR 0063 renders each Show section as a compact summary card. This
//! composite consumes the view-model card contract directly: text and state
//! labels come from `ShowCardDisplay`, while this module owns only card
//! geometry, selection styling, and semantic state colors.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    div, App, ClickEvent, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window,
};

use crate::ui::layouts;
use crate::ui::primitives::{status_badge::render_state_badge, Tooltip};
use crate::ui::tokens::{color, FontSize, Radius, ScaleFactor, SemanticColor, Size, Spacing};
use crate::view_models::show::{ShowCardDisplay, ShowCardKind, ShowCardStateKind};

type ShowCardClickHandler = Rc<dyn Fn(ShowCardKind, &ClickEvent, &mut Window, &mut App) + 'static>;

/// Selectable dashboard card for one Show section.
#[derive(IntoElement)]
#[must_use]
pub(crate) struct ShowCard {
    display: ShowCardDisplay,
    selected: bool,
    on_select: Option<ShowCardClickHandler>,
}

impl ShowCard {
    /// Creates a Show dashboard card.
    pub(crate) fn new(display: ShowCardDisplay) -> Self {
        Self {
            display,
            selected: false,
            on_select: None,
        }
    }

    /// Marks the card as selected.
    pub(crate) fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Supplies the card selection callback.
    pub(crate) fn on_select(
        mut self,
        handler: impl Fn(ShowCardKind, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ShowCard {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let card_id = show_card_id(self.display.kind);
        // ADR 0039 task 002: prove the fixed `.h(Size::MenuCompact.scaled(cx))`
        // cap below always holds the reserved header+summary block, at every
        // scale step, before it clips anything silently. Debug-only: this
        // never changes what release builds render, and the reservation is
        // sized against the ADR's numeric proposal as a ceiling, not against
        // today's identity type output, so it stays true ahead of task 003's
        // ratification too. If this ever fires, the fix is to bring the
        // measured mismatch back to ADR 0039 — never to shrink the font, drop
        // a line or grow chrome here to silence it.
        debug_assert!(
            layouts::show_card_summary_reservation(ScaleFactor::current(cx))
                <= layouts::show_card_available_inner_height(ScaleFactor::current(cx)),
            "ADR 0039 (docs/adr/0039-dynamic-type-ramp.md#wrapping-and-fixed-height-reservation): \
             ShowCard's reserved header+summary block exceeds Size::MenuCompact's available \
             inner height at this scale step. Fix: report the measured mismatch to ADR 0039; \
             do not shrink the font, drop a line, or grow chrome to pass this check."
        );
        let card_kind = self.display.kind;
        let selected = self.selected;
        let on_select = self.on_select.clone();
        let border = if selected {
            SemanticColor::Accent
        } else {
            SemanticColor::Separator
        };
        let background = if selected {
            SemanticColor::SelectedContent
        } else {
            SemanticColor::SecondarySystemBackground
        };
        let hover_background = if selected {
            SemanticColor::SelectedContent
        } else {
            SemanticColor::TertiarySystemBackground
        };
        let tooltip_label = SharedString::from(self.display.a11y_label.clone());

        let mut card = div()
            .id(card_id)
            .h(Size::MenuCompact.scaled(cx))
            .min_w_0()
            .overflow_hidden()
            .flex()
            .flex_col()
            .justify_between()
            .gap(Spacing::SM.scaled(cx))
            .p(Spacing::MD.scaled(cx))
            .rounded(Radius::MD.scaled(cx))
            .border_1()
            .border_color(color(cx, border))
            .bg(color(cx, background))
            .tooltip(move |window, cx| Tooltip::new(tooltip_label.clone()).build(window, cx))
            .child(render_header(
                self.display.title,
                self.display.state_label,
                self.display.state,
                cx,
            ))
            .child(render_summary_lines(
                self.display.primary,
                self.display.secondary,
                cx,
            ))
            .debug_selector(|| card_id.to_owned());

        if let Some(handler) = on_select {
            card = card
                .cursor_pointer()
                .hover(move |el| el.bg(color(cx, hover_background)))
                .on_click(move |event, window, cx| handler(card_kind, event, window, cx));
        }

        card
    }
}

fn render_header(
    title: &'static str,
    state_label: String,
    state: ShowCardStateKind,
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
            div()
                .flex_1()
                .min_w_0()
                .text_size(FontSize::Headline.scaled(cx))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color(cx, SemanticColor::Label))
                .truncate()
                .child(SharedString::from(title)),
        )
        .child(render_state_badge(state_label, state, cx))
}

fn render_summary_lines(primary: String, secondary: String, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .min_w_0()
        .gap(Spacing::XS.scaled(cx))
        .child(render_summary_line(primary, SemanticColor::Label, cx))
        .child(render_summary_line(
            secondary,
            SemanticColor::SecondaryLabel,
            cx,
        ))
}

fn render_summary_line(value: String, text_color: SemanticColor, cx: &App) -> impl IntoElement {
    // ADR 0039 task 002: single-line discipline, following the playlist row's
    // mechanism (src/ui/shells/playlist.rs:754-755,764-765) — a silent clip
    // with no ellipsis. Tail truncation would fail
    // `adr_0063_column_text_does_not_truncate` here: this div chain has none
    // of `flex_1()`/`max_w(`/`.w(`, so tail truncation would render an
    // ellipsis and drop the text (ADR 0063). `overflow_hidden()` on the card
    // (line 85 above) already clips a wrapped second line silently;
    // `whitespace_nowrap()` here stops that wrap from ever happening.
    div()
        .min_h(FontSize::Body.scaled(cx))
        .whitespace_nowrap()
        .overflow_hidden()
        .text_size(FontSize::Body.scaled(cx))
        .text_color(color(cx, text_color))
        .child(SharedString::from(value))
}

const fn show_card_id(kind: ShowCardKind) -> &'static str {
    match kind {
        ShowCardKind::Source => "show-card-source",
        ShowCardKind::LiveMetadata => "show-card-live-metadata",
        ShowCardKind::Stream => "show-card-stream",
    }
}

#[cfg(test)]
mod tests {
    use gpui::{div, prelude::*, px, TestAppContext};

    use super::{show_card_id, ShowCard};
    use crate::view_models::show::{ShowCardDisplay, ShowCardKind, ShowCardStateKind};

    fn fixture(primary: String, secondary: String) -> ShowCardDisplay {
        ShowCardDisplay {
            kind: ShowCardKind::Source,
            title: "Source",
            state_label: "Attached".to_owned(),
            state: ShowCardStateKind::Ok,
            primary,
            secondary,
            a11y_label: "Source, Attached".to_owned(),
        }
    }

    struct CardHeightTest {
        display: ShowCardDisplay,
        selected: bool,
    }

    impl gpui::Render for CardHeightTest {
        fn render(
            &mut self,
            _window: &mut gpui::Window,
            _cx: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            div()
                .w(px(400.0))
                .child(ShowCard::new(self.display.clone()).selected(self.selected))
        }
    }

    /// ADR 0039 task 002 M1/M4: `ShowCard`'s fixed
    /// `.h(Size::MenuCompact.scaled(cx))` height (pinned by
    /// `adr_0063_show_card_grid_shell_uses_vm_contract`) does not depend on
    /// summary-line string length or selection state. Long text clips
    /// silently (`render_summary_line`'s `whitespace_nowrap()` +
    /// `overflow_hidden()`) instead of growing the card — this is the
    /// mechanical proof for the M4 single-line fix, at every case a
    /// visual inspector could not distinguish by outer bounds alone.
    #[gpui::test]
    fn adr_0039_show_card_height_is_independent_of_summary_length(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let short = fixture("Attached".to_owned(), "Ready".to_owned());
        let long = fixture(
            "A very long primary summary line that would wrap to a second visual row \
             if nothing clipped it, repeated for good measure."
                .repeat(3),
            "A very long secondary summary line that would also wrap without the \
             single-line fix, repeated for good measure."
                .repeat(3),
        );

        let (view, cx) = cx.add_window_view(|_, _| CardHeightTest {
            display: short.clone(),
            selected: false,
        });

        for (case_display, selected) in [
            (short.clone(), false),
            (long.clone(), false),
            (short, true),
            (long, true),
        ] {
            view.update(cx, |this, cx| {
                this.display = case_display;
                this.selected = selected;
                cx.notify();
            });
            cx.update(|window, cx| {
                let _ = window.draw(cx);
            });
            let bounds = cx
                .debug_bounds(show_card_id(ShowCardKind::Source))
                .expect("show card bounds recorded");
            assert_eq!(
                bounds.size.height,
                px(160.0),
                "ShowCard height changed with selected={selected}"
            );
        }
    }
}
