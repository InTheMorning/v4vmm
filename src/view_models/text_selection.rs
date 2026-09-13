//! Renderer-free text selection and Copy contract for ADRs 0063 and 0071.

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
    /// Appended log entries keep an existing exact selection; replacement clears it.
    pub(crate) fn update_text(&mut self, previous: &str, next: &str) {
        if !next.starts_with(previous) {
            *self = Self::default();
        }
    }

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

    pub(crate) fn range(&self) -> Range<usize> {
        self.anchor.min(self.head)..self.anchor.max(self.head)
    }

    pub(crate) fn selected_text<'a>(&self, text: &'a str) -> Option<&'a str> {
        let range = self.range();
        (!range.is_empty()).then(|| text.get(range)).flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selected(text: &str, anchor: usize, head: usize) -> TextSelection {
        let clip = |offset: usize| text.floor_char_boundary(offset.min(text.len()));
        TextSelection {
            anchor: clip(anchor),
            head: clip(head),
        }
    }

    #[test]
    fn adr_0063_log_append_keeps_selection_and_replacement_clears_it() {
        let previous = "répété 🦀\n";
        let mut selection = selected(previous, 0, previous.len());
        let next = format!("{previous}next\n");
        selection.update_text(previous, &next);
        assert_eq!(selection.selected_text(&next), Some(previous));
        selection.update_text(&next, "replacement");
        assert!(selection.selected_text("replacement").is_none());
    }

    /// Situational ADR 0063: only an exact, nonempty selection enables Copy.
    #[test]
    fn copy_action_requires_a_valid_selection_in_the_current_text() {
        let text = "  café 🦀\n";
        let mut selection = TextSelection::default();
        let action = selection.copy_action(text);
        assert!(action.availability.disabled());
        assert_eq!(action.label, "Copy");
        assert_eq!(action.a11y_label, "Copy selected text");
        selection = selected(text, selection.anchor, text.len());
        assert!(!selection.copy_action(text).availability.disabled());
        assert!(selection.copy_action("x").availability.disabled());
        assert_eq!(selection.selected_text(text), Some(text));
    }

    /// Situational ADR 0063: copied log selections retain whitespace and line boundaries.
    #[test]
    fn selection_copies_exact_multiline_text_in_either_drag_direction() {
        let text = "  first <tag> & value\n\n  deuxième 🦀\t\n";
        let forward = selected(text, 0, text.len());
        assert_eq!(forward.selected_text(text), Some(text));
        let backward = selected(text, text.len(), 0);
        assert_eq!(backward.selected_text(text), Some(text));
    }

    /// Situational ADR 0063: pointer offsets never split Unicode or escape the text buffer.
    #[test]
    fn selection_clamps_offsets_and_copies_partial_unicode_text() {
        let text = "a🦀éz";
        let mut selection = selected(text, 3, 6);
        assert_eq!(selection.selected_text(text), Some("🦀"));
        selection = selected(text, selection.anchor, usize::MAX);
        assert_eq!(selection.selected_text(text), Some("🦀éz"));
        assert!(selected(text, 0, 0).selected_text(text).is_none());
    }
}
