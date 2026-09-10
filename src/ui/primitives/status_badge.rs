//! Shared labeled Show state badge geometry and semantic colors (ADR 0063).
#![warn(clippy::pedantic)]
use crate::ui::tokens::{color, FontSize, Radius, SemanticColor, Size, Spacing};
use crate::view_models::show::ShowCardStateKind;
use gpui::{div, App, FontWeight, IntoElement, ParentElement, SharedString, Styled};

pub(crate) fn render_state_badge(
    state_label: String,
    state: ShowCardStateKind,
    cx: &App,
) -> impl IntoElement {
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

const fn state_badge_tokens(state: ShowCardStateKind) -> (SemanticColor, SemanticColor) {
    match state {
        ShowCardStateKind::Ok => (SemanticColor::Success, SemanticColor::OnSuccess),
        ShowCardStateKind::Attention => (SemanticColor::Warning, SemanticColor::OnWarning),
        ShowCardStateKind::Failed => (SemanticColor::Danger, SemanticColor::OnDanger),
        ShowCardStateKind::Unknown => (SemanticColor::Info, SemanticColor::OnInfo),
    }
}
