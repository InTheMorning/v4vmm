//! Renderer-free text selection and Copy action contract for ADR 0063.

#![warn(clippy::pedantic)]

use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TextCopyAvailability {
    Available,
    NoSelection,
}

impl TextCopyAvailability {
    pub(crate) const fn disabled(self) -> bool {
        matches!(self, Self::NoSelection)
    }
}

pub(crate) struct TextCopyActionDisplay {
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) a11y_label: &'static str,
    pub(crate) availability: TextCopyAvailability,
}

#[derive(Default)]
pub(crate) struct TextSelection {
    pub(crate) anchor: usize,
    pub(crate) head: usize,
}

impl TextSelection {
    pub(crate) fn copy_action(&self, text: &str) -> TextCopyActionDisplay {
        TextCopyActionDisplay {
            id: "copy-selection",
            label: "Copy",
            a11y_label: "Copy selected text",
            availability: if self.selected_text(text).is_some() {
                TextCopyAvailability::Available
            } else {
                TextCopyAvailability::NoSelection
            },
        }
    }

    pub(crate) fn at(text: &str, index: usize) -> Self {
        let index = char_boundary(text, index);
        Self {
            anchor: index,
            head: index,
        }
    }

    pub(crate) fn extend(&mut self, text: &str, index: usize) {
        self.head = char_boundary(text, index);
    }

    pub(crate) fn range(&self) -> Range<usize> {
        self.anchor.min(self.head)..self.anchor.max(self.head)
    }

    pub(crate) fn selected_text<'a>(&self, text: &'a str) -> Option<&'a str> {
        let range = self.range();
        (!range.is_empty()).then(|| text.get(range)).flatten()
    }
}

fn char_boundary(text: &str, index: usize) -> usize {
    let mut index = index.min(text.len());
    while !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Situational ADR 0063: only an exact, nonempty selection enables Copy.
    #[test]
    fn copy_action_requires_a_valid_selection_in_the_current_text() {
        let text = "  café 🦀\n";
        let mut selection = TextSelection::default();
        let action = selection.copy_action(text);
        assert!(action.availability.disabled());
        assert_eq!(action.label, "Copy");
        assert_eq!(action.a11y_label, "Copy selected text");
        selection.extend(text, text.len());
        assert!(!selection.copy_action(text).availability.disabled());
        assert!(selection.copy_action("x").availability.disabled());
        assert_eq!(selection.selected_text(text), Some(text));
    }

    /// Situational ADR 0063: copied log selections retain whitespace and line boundaries.
    #[test]
    fn selection_copies_exact_multiline_text_in_either_drag_direction() {
        let text = "  first <tag> & value\n\n  deuxième 🦀\t\n";
        let mut forward = TextSelection::at(text, 0);
        forward.extend(text, text.len());
        assert_eq!(forward.selected_text(text), Some(text));
        let mut backward = TextSelection::at(text, text.len());
        backward.extend(text, 0);
        assert_eq!(backward.selected_text(text), Some(text));
    }

    /// Situational ADR 0063: pointer offsets never split Unicode or escape the text buffer.
    #[test]
    fn selection_clamps_offsets_and_copies_partial_unicode_text() {
        let text = "a🦀éz";
        let mut selection = TextSelection::at(text, 3);
        selection.extend(text, 6);
        assert_eq!(selection.selected_text(text), Some("🦀"));
        selection.extend(text, usize::MAX);
        assert_eq!(selection.selected_text(text), Some("🦀éz"));
        assert!(TextSelection::at(text, 0).selected_text(text).is_none());
    }
}
