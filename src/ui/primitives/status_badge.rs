//! Shared labeled Show state badge geometry and semantic colors (ADR 0063),
//! and the neutral track row badge (ADR 0083 task 006).
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

/// The neutral badge of a track row, for example "New" (ADR 0083 task
/// 006, decision D11). The word carries the state, so the badge does not
/// rely on color alone.
pub(crate) fn render_row_badge(label: impl Into<SharedString>, cx: &App) -> impl IntoElement {
    div()
        .flex_shrink_0()
        .rounded(Radius::SM.scaled(cx))
        .bg(color(cx, SemanticColor::TertiaryFill))
        .px(Spacing::XS.scaled(cx))
        .text_size(FontSize::Micro.scaled(cx))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(color(cx, SemanticColor::SecondaryLabel))
        .whitespace_nowrap()
        .child(label.into())
}

const fn state_badge_tokens(state: ShowCardStateKind) -> (SemanticColor, SemanticColor) {
    match state {
        ShowCardStateKind::Ok => (SemanticColor::Success, SemanticColor::OnSuccess),
        ShowCardStateKind::Attention => (SemanticColor::Warning, SemanticColor::OnWarning),
        ShowCardStateKind::Failed => (SemanticColor::Danger, SemanticColor::OnDanger),
        ShowCardStateKind::Unknown => (SemanticColor::Info, SemanticColor::OnInfo),
    }
}
