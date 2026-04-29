//! Multi-line truncated text primitive.
//!
//! Borrowed from the `SwiftUI` `Text(value).lineLimit(n)` shape with
//! tail truncation. Splits `value` on `\n`, keeps at most `max_lines`
//! lines, and replaces the last visible line with `"…"` when the
//! source has more lines than the limit. Each rendered line is
//! single-line truncated so a long word doesn't break layout.
//!
//! Empty input renders as a single blank line so the row reserves
//! its baseline height — matches the legacy
//! `ui_common::compare_value_line_elements` contract that several
//! detail-grid call sites depend on.
//!
//! ```ignore
//! MultilineText::new(value).max_lines(6)
//! ```

#![warn(clippy::pedantic)]

use gpui::{div, prelude::*, px, App, IntoElement, Pixels, RenderOnce, Rgba, SharedString, Window};

use crate::ui::tokens::{Appearance, FontSize, SemanticColor};

const ELLIPSIS: &str = "...";
const BLANK_PLACEHOLDER: &str = " ";

#[derive(IntoElement)]
#[must_use]
pub struct MultilineText {
    value: SharedString,
    max_lines: usize,
    color: Option<SemanticColor>,
    color_raw: Option<Rgba>,
    size: Option<FontSize>,
    line_height: Option<Pixels>,
    appearance: Option<Appearance>,
}

impl MultilineText {
    pub fn new(value: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            max_lines: 6,
            color: None,
            color_raw: None,
            size: None,
            line_height: None,
            appearance: None,
        }
    }

    /// Maximum number of lines to render before truncating with an
    /// ellipsis line. Mirrors `SwiftUI` `.lineLimit(n)`.
    pub fn max_lines(mut self, max_lines: usize) -> Self {
        self.max_lines = max_lines.max(1);
        self
    }

    pub fn color(mut self, color: SemanticColor) -> Self {
        self.color = Some(color);
        self
    }

    /// Escape hatch for callers that already resolved a non-tokenized
    /// color (e.g. ID3 frame version palette in the compare grid).
    /// Prefer [`Self::color`] in new code.
    pub fn color_raw(mut self, color: Rgba) -> Self {
        self.color_raw = Some(color);
        self
    }

    pub fn size(mut self, size: FontSize) -> Self {
        self.size = Some(size);
        self
    }

    /// Override line height in pixels. Mirrors `SwiftUI`
    /// `.lineSpacing()` semantics for compact rows.
    pub fn line_height(mut self, line_height: Pixels) -> Self {
        self.line_height = Some(line_height);
        self
    }

    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = Some(appearance);
        self
    }
}

impl RenderOnce for MultilineText {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let appearance = self.appearance.unwrap_or_else(|| Appearance::current(cx));
        let size = self.size.unwrap_or(FontSize::Micro);

        let mut container = div().flex().flex_col().text_size(size.px());
        if let Some(color) = self.color {
            container = container.text_color(color.resolve(appearance));
        } else if let Some(raw) = self.color_raw {
            container = container.text_color(raw);
        }
        if let Some(lh) = self.line_height {
            container = container.line_height(lh);
        } else {
            container = container.line_height(px(17.0));
        }

        for line in lines_for_render(&self.value, self.max_lines) {
            container = container.child(div().truncate().child(SharedString::from(line)));
        }
        container
    }
}

/// Pure projection of `value` into the lines we'll render. Public so
/// the unit tests can pin behaviour without a `Window` / `App`.
fn lines_for_render(value: &str, max_lines: usize) -> Vec<String> {
    let max_lines = max_lines.max(1);
    let mut lines: Vec<&str> = value.lines().collect();
    if lines.is_empty() {
        lines.push("");
    }
    let truncated_total = lines.len() > max_lines;
    lines
        .into_iter()
        .take(max_lines)
        .enumerate()
        .map(|(index, line)| {
            if truncated_total && index + 1 == max_lines {
                ELLIPSIS.to_string()
            } else if line.is_empty() {
                BLANK_PLACEHOLDER.to_string()
            } else {
                line.to_string()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_value_renders_single_blank_line() {
        let lines = lines_for_render("", 4);
        assert_eq!(lines, vec![BLANK_PLACEHOLDER.to_string()]);
    }

    #[test]
    fn under_limit_keeps_every_line() {
        let lines = lines_for_render("a\nb", 5);
        assert_eq!(lines, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn over_limit_replaces_last_visible_line_with_ellipsis() {
        let lines = lines_for_render("a\nb\nc\nd\ne", 3);
        assert_eq!(
            lines,
            vec!["a".to_string(), "b".to_string(), ELLIPSIS.to_string()]
        );
    }

    #[test]
    fn blank_lines_are_replaced_with_a_space_so_baseline_is_kept() {
        let lines = lines_for_render("a\n\nc", 5);
        assert_eq!(
            lines,
            vec![
                "a".to_string(),
                BLANK_PLACEHOLDER.to_string(),
                "c".to_string()
            ]
        );
    }

    #[test]
    fn max_lines_zero_is_clamped_to_one() {
        let lines = lines_for_render("a\nb", 0);
        // With max=1 and 2 input lines, the single rendered line is the
        // ellipsis.
        assert_eq!(lines, vec![ELLIPSIS.to_string()]);
    }
}
