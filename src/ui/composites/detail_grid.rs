//! Vertical key/value table used by inspector views.
//!
//! ## Display contract: `DetailRow` / `DetailTextRow`
//!
//! Replaces the legacy `ui_common::render_detail_grid` helper. Sizing is
//! scale-aware (key column width, gaps, and font size all flow through
//! `.scaled(cx)`).

#![warn(clippy::pedantic)]

use gpui::{
    div, AnyElement, App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window,
};

use crate::ui::primitives::{HStack, VStack};
use crate::ui::tokens::{resolve_color, Appearance, FontSize, ScaleFactor, SemanticColor, Spacing};

/// Single key/value row in a [`DetailGrid`]. The value is an
/// already-built `AnyElement` so callers may render rich content
/// (multi-line, links, etc.) without a fixed string formatting pass.
pub struct DetailRow {
    pub key: SharedString,
    pub value: AnyElement,
}

/// Display-ready rich key/value row.
pub struct DetailElementRow {
    pub key: SharedString,
    pub value: AnyElement,
}

/// Display-ready plain-text key/value row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailTextRow {
    pub key: SharedString,
    pub value: String,
    pub max_lines: usize,
}

impl DetailRow {
    #[must_use]
    pub fn new(row: DetailElementRow) -> Self {
        Self {
            key: row.key,
            value: row.value,
        }
    }

    /// Convenience for plain string values; clamps long values to a
    /// caller-controlled max number of lines.
    #[must_use]
    pub fn text(row: DetailTextRow) -> Self {
        let elements = compare_value_lines(&row.value, row.max_lines);
        Self::new(DetailElementRow {
            key: row.key,
            value: div()
                .flex()
                .flex_col()
                .children(elements.into_iter().map(|line| {
                    div()
                        .truncate()
                        .child(SharedString::from(line))
                        .into_any_element()
                }))
                .into_any_element(),
        })
    }
}

fn compare_value_lines(value: &str, max_lines: usize) -> Vec<String> {
    let mut lines: Vec<String> = value.lines().map(str::to_owned).collect();
    if lines.is_empty() {
        lines.push(String::new());
    }
    let truncated = lines.len() > max_lines;
    lines
        .into_iter()
        .take(max_lines)
        .enumerate()
        .map(|(index, line)| {
            if truncated && index + 1 == max_lines {
                "...".to_owned()
            } else if line.is_empty() {
                " ".to_owned()
            } else {
                line
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_row_uses_display_contract() {
        let row = DetailRow::text(DetailTextRow {
            key: "Website".into(),
            value: "https://example.com".to_string(),
            max_lines: 2,
        });

        assert_eq!(row.key, SharedString::from("Website"));
    }

    #[test]
    fn text_row_clamps_empty_values_for_baseline() {
        assert_eq!(compare_value_lines("", 2), vec![" ".to_string()]);
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct DetailGrid {
    rows: Vec<DetailRow>,
    appearance: Option<Appearance>,
}

impl DetailGrid {
    pub fn new(rows: Vec<DetailRow>) -> Self {
        Self {
            rows,
            appearance: None,
        }
    }

    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = Some(appearance);
        self
    }
}

/// Base width of the key column — Apple-style "right-aligned label"
/// pattern uses ~120pt at 1.0× scale; we honor `ScaleFactor`.
const KEY_COL_BASE: f32 = 124.0;

impl RenderOnce for DetailGrid {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let key_color = resolve_color(cx, SemanticColor::SecondaryLabel, self.appearance);
        let mult = ScaleFactor::current(cx).multiplier();
        let key_width = gpui::px(KEY_COL_BASE * mult);
        let body_size = FontSize::Micro.scaled(cx);

        let mut stack = VStack::new().spacing(Spacing::XS).stretch();
        for row in self.rows {
            stack = stack.child(
                HStack::new()
                    .spacing(Spacing::MD)
                    .top()
                    .child(
                        div()
                            .w(key_width)
                            .flex_shrink_0()
                            .text_color(key_color)
                            .whitespace_nowrap()
                            .text_size(body_size)
                            .child(row.key),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(body_size)
                            .child(row.value),
                    ),
            );
        }
        stack
    }
}
