//! Token-driven text helpers — small one-line text element builders
//! used across screens (truncated lists, multi-line compare rows).
//! Lifted out of the deprecated `ui_common` module.
//!
//! ## Status — transitional
//!
//! These remain free functions returning `Div` / `AnyElement` because
//! every existing call site chains Div modifiers
//! (`.font_weight(...)`, `.text_size(...)`, `.flex_1()`,
//! `.text_color(...)`) onto the result. Promoting them to proper
//! SwiftUI-style primitives — `Label::caption(text).truncated()`,
//! `MultilineText::new(value).max_lines(n)` — is the right end
//! state, borrowing the `SwiftUI` `Text(...).lineLimit(n)` shape, but
//! it requires rewriting every chaining call site. That work is
//! tracked as the `swiftui-text-primitives` todo and lands during
//! the `screen-search` / `screen-library` migration.
//!
//! `SectionHeader` (the `SwiftUI` `Section { ... } header: { ... }`
//! shape) has already moved to
//! [`crate::ui::primitives::SectionHeader`].
//!
//! Until then: do not add new callers. Use [`crate::ui::primitives`]
//! for any new text rendering.

#![warn(clippy::pedantic)]

use gpui::{div, AnyElement, Div, IntoElement, ParentElement, SharedString, Styled};

use crate::ui::theme::{color, typography};

/// Single-line text clamped with `truncate()`. Width is constrained
/// via `min_w_0` so the parent flex row collapses it correctly.
#[must_use]
pub fn truncated(text: String) -> Div {
    div()
        .min_w_0()
        .truncate()
        .text_size(typography::SIZE_MICRO)
        .child(SharedString::from(text))
}

/// Same as [`truncated`], but in muted text color. Used for secondary
/// row metadata (artist · feed · date).
#[must_use]
pub fn truncated_muted(text: String) -> Div {
    truncated(text).text_color(color::text_muted())
}

/// Build the per-line truncated `AnyElement`s for a multi-line value
/// in a detail-grid row. Empty input becomes a single blank line; if
/// the value exceeds `max_lines`, the last visible line is replaced
/// with `"..."`. Kept identical to the original `ui_common`
/// behaviour.
#[must_use]
pub fn compare_value_line_elements(value: &str, max_lines: usize) -> Vec<AnyElement> {
    let mut lines = value.lines().collect::<Vec<_>>();
    if lines.is_empty() {
        lines.push("");
    }
    let truncated_total = lines.len() > max_lines;
    lines
        .into_iter()
        .take(max_lines)
        .enumerate()
        .map(|(index, line)| {
            let line = if truncated_total && index + 1 == max_lines {
                "..."
            } else if line.is_empty() {
                " "
            } else {
                line
            };
            div()
                .truncate()
                .child(SharedString::from(line.to_string()))
                .into_any_element()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_value_line_elements_empty_returns_one_blank() {
        assert_eq!(compare_value_line_elements("", 3).len(), 1);
    }

    #[test]
    fn compare_value_line_elements_truncates_with_ellipsis() {
        let v = "a\nb\nc\nd\ne";
        assert_eq!(compare_value_line_elements(v, 3).len(), 3);
    }

    #[test]
    fn compare_value_line_elements_keeps_all_when_under_limit() {
        let v = "a\nb";
        assert_eq!(compare_value_line_elements(v, 5).len(), 2);
    }
}
