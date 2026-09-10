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

use crate::ui::primitives::Tooltip;
use crate::ui::tokens::{color, FontSize, Radius, SemanticColor, Size, Spacing};
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
            ));

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

fn render_state_badge(state_label: String, state: ShowCardStateKind, cx: &App) -> impl IntoElement {
    let (fill, label_color) = state_badge_tokens(state);
    div()
        .flex_shrink_0()
        .max_w(Size::MenuRegular.scaled(cx))
        .overflow_hidden()
        .rounded(Radius::SM.scaled(cx))
        .bg(color(cx, fill))
        .px(Spacing::SM.scaled(cx))
        .py(Spacing::XXS.scaled(cx))
        .text_size(FontSize::Micro.scaled(cx))
        .font_weight(FontWeight::BOLD)
        .text_color(color(cx, label_color))
        .truncate()
        .child(SharedString::from(state_label))
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
    // No `truncate()` here. In this column it renders the ellipsis and drops the
    // text. The card already clips with `overflow_hidden`.
    div()
        .min_h(FontSize::Body.scaled(cx))
        .text_size(FontSize::Body.scaled(cx))
        .text_color(color(cx, text_color))
        .child(SharedString::from(value))
}

const fn state_badge_tokens(state: ShowCardStateKind) -> (SemanticColor, SemanticColor) {
    match state {
        ShowCardStateKind::Ok => (SemanticColor::Success, SemanticColor::OnSuccess),
        ShowCardStateKind::Attention => (SemanticColor::Warning, SemanticColor::OnWarning),
        ShowCardStateKind::Failed => (SemanticColor::Danger, SemanticColor::OnDanger),
        ShowCardStateKind::Unknown => (SemanticColor::Info, SemanticColor::OnInfo),
    }
}

const fn show_card_id(kind: ShowCardKind) -> &'static str {
    match kind {
        ShowCardKind::Source => "show-card-source",
        ShowCardKind::LiveMetadata => "show-card-live-metadata",
        ShowCardKind::Stream => "show-card-stream",
    }
}
