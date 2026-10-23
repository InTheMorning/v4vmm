//! The field list under the toolbar search field (ADR 0087 Decision 3).
//!
//! The list shows the query fields, the fixed values of a field, the error
//! of the field under the text cursor and the scope conflict of the query.
//! This module gives each row, its text, its accessibility label, its typed
//! action and its availability. It also gives the open state of the list,
//! the outcome of each list key and the text edit of each row.
//!
//! This module reads the query text only through `search_query`: the
//! `cursor_term` function, the `SearchQuery` parser and the field table. It
//! has no GPUI type and no I/O. The renderer only composes the rows.

#![warn(clippy::pedantic)]

use std::ops::Range;

use crate::view_models::search_query::{
    cursor_term, fields_with_prefix, CursorPartial, FieldValueState, FixedValueInfo,
    QueryFieldInfo, QueryFieldName, QueryScope, SearchQuery, TermKind,
};

/// The note of a row that `in:index` makes unavailable.
const NOTE_INDEX_SCOPE: &str = "Not available with in:index";

/// The note of the `in:index` row in a Library-only query.
const NOTE_LIBRARY_SCOPE: &str = "Not available with a Library-only field or in:library";

/// The text of the scope conflict notice.
const SCOPE_CONFLICT_TEXT: &str =
    "This query asks for the Library only and for MusicIndex only. It shows no rows.";

/// The open state of the field list.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FieldListOpening {
    /// The list does not show.
    #[default]
    Closed,
    /// A focus or a click opened the list. An empty field name shows each
    /// field.
    Requested,
    /// The curator types. An empty field name after other text shows no
    /// field.
    Typing,
}

/// One event that changes the open state of the field list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldListEvent {
    /// The search field got the keyboard focus.
    Focus,
    /// The search field lost the keyboard focus.
    Blur,
    /// A left mouse button went down in the search field.
    Click,
    /// The text of the search field changed.
    Change,
    /// The cursor or the selection moved, and the text did not change.
    CursorMoved,
    /// The Escape key closed the list.
    Escape,
    /// The search ran.
    Submit,
    /// The view put the edit of a row into the text.
    Inserted,
}

/// The open state, the focus and the selected row of the field list.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FieldListState {
    /// The open state.
    pub opening: FieldListOpening,
    /// `true` while the search field has the keyboard focus.
    pub focused: bool,
    /// The index of the selected row.
    pub selected: Option<usize>,
}

impl FieldListState {
    /// Changes the state for one event.
    pub fn apply(&mut self, event: FieldListEvent) {
        match event {
            FieldListEvent::Focus => {
                self.focused = true;
                self.opening = FieldListOpening::Requested;
                self.selected = None;
            }
            FieldListEvent::Click => {
                self.opening = FieldListOpening::Requested;
                self.selected = None;
            }
            FieldListEvent::Change => {
                if self.focused {
                    self.opening = FieldListOpening::Typing;
                }
                self.selected = None;
            }
            FieldListEvent::CursorMoved => self.selected = None,
            FieldListEvent::Escape | FieldListEvent::Submit | FieldListEvent::Inserted => {
                self.opening = FieldListOpening::Closed;
            }
            FieldListEvent::Blur => {
                self.focused = false;
                self.opening = FieldListOpening::Closed;
            }
        }
    }

    /// Sets the selected row.
    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index;
    }
}

/// The kind of one row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldListRowKind {
    /// One query field.
    Field { field: QueryFieldName },
    /// One fixed value of a field.
    FixedValue {
        field: QueryFieldName,
        word: &'static str,
    },
}

/// The typed action of one row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldListRowAction {
    /// Puts the field token into the text.
    InsertField(QueryFieldName),
    /// Puts the value word into the text.
    InsertValue {
        field: QueryFieldName,
        word: &'static str,
    },
}

/// The reason that a row is not available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldListRowUnavailable {
    /// The row conflicts with the scope of the query.
    ScopeConflict,
}

/// The availability of one row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldListRowAvailability {
    Available,
    Unavailable(FieldListRowUnavailable),
}

impl FieldListRowAvailability {
    /// `true` when the row is available.
    #[must_use]
    pub fn is_available(self) -> bool {
        self == Self::Available
    }
}

/// The mark of one row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldListMark {
    /// Only the Library can apply the field.
    LibraryOnly,
}

impl FieldListMark {
    /// The word of the mark.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::LibraryOnly => "Library only",
        }
    }
}

/// The text edit of one row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldListEdit {
    /// The byte range of the old text that the edit replaces.
    pub range: Range<usize>,
    /// The new text for that range.
    pub text: String,
    /// The cursor offset in the changed text.
    pub cursor: usize,
}

/// One row of the field list.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldListRowVm {
    /// The element id of the row.
    pub element_id: String,
    /// The field or the fixed value of the row.
    pub kind: FieldListRowKind,
    /// The field token or the value word.
    pub label: String,
    /// The values of the field, or the description of the value.
    pub values_text: Option<String>,
    /// One example of the field.
    pub example_text: Option<String>,
    /// The mark of the row.
    pub mark: Option<FieldListMark>,
    /// The reason that the row is not available.
    pub note: Option<String>,
    /// The accessibility label of the row.
    pub a11y_label: String,
    /// The typed action of the row.
    pub action: FieldListRowAction,
    /// The availability of the row.
    pub availability: FieldListRowAvailability,
    /// The text edit that the row puts into the text.
    pub edit: FieldListEdit,
    /// `true` for the selected row.
    pub selected: bool,
}

/// The kind of one notice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldListNoticeKind {
    /// The field under the cursor has a value that is not correct.
    FieldError,
    /// The query asks for the Library only and for the Index only.
    ScopeConflict,
}

/// One notice of the field list. A notice is not a row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldListNoticeVm {
    /// The kind of the notice.
    pub kind: FieldListNoticeKind,
    /// The text of the notice.
    pub text: String,
    /// The accessibility label of the notice.
    pub a11y_label: String,
}

/// One key that the field list reads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldListKey {
    Up,
    Down,
    Tab,
    Enter,
    Escape,
}

/// The outcome of one key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FieldListKeyOutcome {
    /// Select the row with this index.
    Select(usize),
    /// Put this edit into the text.
    Apply(FieldListEdit),
    /// Close the list.
    Close,
    /// The list does not use the key. The search field gets it.
    Pass,
}

/// The field list under the toolbar search field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldListVm {
    /// The rows, in the order of the field table or of the fixed values.
    pub rows: Vec<FieldListRowVm>,
    /// The notices. The field error comes first.
    pub notices: Vec<FieldListNoticeVm>,
    /// The index of the selected row.
    pub selected: Option<usize>,
    /// The accessibility label of the list.
    pub a11y_label: String,
}

impl FieldListVm {
    /// The element id of the scrolling row list.
    pub const LIST_ID: &'static str = "search-field-list";

    /// Builds the field list for the text, the selection and the state.
    ///
    /// `selection` is the byte range of the selection. An empty range is
    /// the cursor. The result is `None` when the list is closed, when the
    /// search field has no focus, when the selection is not empty, or when
    /// the list has no row and no notice.
    #[must_use]
    pub fn build(text: &str, selection: Range<usize>, state: &FieldListState) -> Option<Self> {
        if state.opening == FieldListOpening::Closed || !state.focused {
            return None;
        }
        if !selection.is_empty() {
            return None;
        }
        let cursor = selection.start.min(text.len());
        let query = SearchQuery::parse(text);
        let scope = query.scope();
        let term = cursor_term(text, cursor);
        let mut rows = match &term.partial {
            CursorPartial::FieldName { prefix } => {
                let hide_all = prefix.is_empty()
                    && state.opening == FieldListOpening::Typing
                    && text.chars().any(|ch| !ch.is_whitespace());
                if hide_all {
                    Vec::new()
                } else {
                    let start = term.range.start;
                    let edit_range = start..cursor.max(start);
                    fields_with_prefix(prefix)
                        .into_iter()
                        .map(|info| field_row(info, edit_range.clone(), scope))
                        .collect()
                }
            }
            CursorPartial::FieldValue {
                field,
                prefix,
                value_range,
            } => {
                let add_space = value_range.end == term.range.end
                    && !text[value_range.end..]
                        .chars()
                        .next()
                        .is_some_and(char::is_whitespace);
                field
                    .values_with_prefix(prefix)
                    .into_iter()
                    .map(|value| value_row(*field, value, value_range.clone(), add_space, scope))
                    .collect()
            }
            CursorPartial::Other => Vec::new(),
        };
        let notices = notices(&query, scope, cursor);
        if rows.is_empty() && notices.is_empty() {
            return None;
        }
        let selected = state.selected.filter(|index| *index < rows.len());
        if let Some(index) = selected {
            rows[index].selected = true;
        }
        let a11y_label = list_a11y_label(rows.len());
        Some(Self {
            rows,
            notices,
            selected,
            a11y_label,
        })
    }

    /// Gives the outcome of one key.
    #[must_use]
    pub fn key(&self, key: FieldListKey) -> FieldListKeyOutcome {
        let available: Vec<usize> = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.availability.is_available())
            .map(|(index, _)| index)
            .collect();
        let selected = self.selected.filter(|index| available.contains(index));
        match key {
            FieldListKey::Down => match (available.first(), selected) {
                (None, _) => FieldListKeyOutcome::Pass,
                (Some(first), None) => FieldListKeyOutcome::Select(*first),
                (Some(first), Some(current)) => FieldListKeyOutcome::Select(
                    available
                        .iter()
                        .copied()
                        .find(|index| *index > current)
                        .unwrap_or(*first),
                ),
            },
            FieldListKey::Up => match (available.last(), selected) {
                (None, _) => FieldListKeyOutcome::Pass,
                (Some(last), None) => FieldListKeyOutcome::Select(*last),
                (Some(last), Some(current)) => FieldListKeyOutcome::Select(
                    available
                        .iter()
                        .rev()
                        .copied()
                        .find(|index| *index < current)
                        .unwrap_or(*last),
                ),
            },
            FieldListKey::Tab => selected
                .or_else(|| available.first().copied())
                .map_or(FieldListKeyOutcome::Pass, |index| {
                    FieldListKeyOutcome::Apply(self.rows[index].edit.clone())
                }),
            FieldListKey::Enter => selected.map_or(FieldListKeyOutcome::Pass, |index| {
                FieldListKeyOutcome::Apply(self.rows[index].edit.clone())
            }),
            FieldListKey::Escape => FieldListKeyOutcome::Close,
        }
    }
}

/// Builds the row of one field. `edit_range` is the typed field name.
fn field_row(
    info: &'static QueryFieldInfo,
    edit_range: Range<usize>,
    scope: QueryScope,
) -> FieldListRowVm {
    let availability = if scope == QueryScope::IndexOnly && info.library_only {
        FieldListRowAvailability::Unavailable(FieldListRowUnavailable::ScopeConflict)
    } else {
        FieldListRowAvailability::Available
    };
    let note = unavailable_note(availability, scope);
    let mark = info.library_only.then_some(FieldListMark::LibraryOnly);
    let mut a11y_label = format!(
        "Field {}, {}, for example {}",
        info.token, info.values_text, info.example
    );
    if let Some(mark) = mark {
        a11y_label.push_str(", ");
        a11y_label.push_str(mark.label());
    }
    push_note(&mut a11y_label, note.as_deref());
    let cursor = edit_range.start + info.token.len();
    FieldListRowVm {
        element_id: format!("search-field-list-field-{}", bare_token(info.token)),
        kind: FieldListRowKind::Field { field: info.name },
        label: info.token.to_owned(),
        values_text: Some(info.values_text.to_owned()),
        example_text: Some(info.example.to_owned()),
        mark,
        note,
        a11y_label,
        action: FieldListRowAction::InsertField(info.name),
        availability,
        edit: FieldListEdit {
            range: edit_range,
            text: info.token.to_owned(),
            cursor,
        },
        selected: false,
    }
}

/// Builds the row of one fixed value. `value_range` is the value item.
fn value_row(
    field: QueryFieldName,
    value: &'static FixedValueInfo,
    value_range: Range<usize>,
    add_space: bool,
    scope: QueryScope,
) -> FieldListRowVm {
    let token = field.info().token;
    let conflict = match scope {
        QueryScope::IndexOnly => field.is_library_only(),
        QueryScope::LibraryOnly => {
            SearchQuery::parse(&format!("{token}{}", value.word)).scope() == QueryScope::IndexOnly
        }
        QueryScope::Everything | QueryScope::Conflict => false,
    };
    let availability = if conflict {
        FieldListRowAvailability::Unavailable(FieldListRowUnavailable::ScopeConflict)
    } else {
        FieldListRowAvailability::Available
    };
    let note = unavailable_note(availability, scope);
    let mut a11y_label = format!("Value {} for {token}, {}", value.word, value.description);
    push_note(&mut a11y_label, note.as_deref());
    let mut text = value.word.to_owned();
    if add_space {
        text.push(' ');
    }
    let cursor = value_range.start + text.len();
    FieldListRowVm {
        element_id: format!(
            "search-field-list-value-{}-{}",
            bare_token(token),
            value.word
        ),
        kind: FieldListRowKind::FixedValue {
            field,
            word: value.word,
        },
        label: value.word.to_owned(),
        values_text: Some(value.description.to_owned()),
        example_text: None,
        mark: None,
        note,
        a11y_label,
        action: FieldListRowAction::InsertValue {
            field,
            word: value.word,
        },
        availability,
        edit: FieldListEdit {
            range: value_range,
            text,
            cursor,
        },
        selected: false,
    }
}

/// The token without its colon.
fn bare_token(token: &str) -> &str {
    token.strip_suffix(':').unwrap_or(token)
}

/// The note of an unavailable row in this scope.
fn unavailable_note(availability: FieldListRowAvailability, scope: QueryScope) -> Option<String> {
    match availability {
        FieldListRowAvailability::Available => None,
        FieldListRowAvailability::Unavailable(FieldListRowUnavailable::ScopeConflict) => {
            let note = if scope == QueryScope::IndexOnly {
                NOTE_INDEX_SCOPE
            } else {
                NOTE_LIBRARY_SCOPE
            };
            Some(note.to_owned())
        }
    }
}

/// Adds the note to an accessibility label.
fn push_note(label: &mut String, note: Option<&str>) {
    if let Some(note) = note {
        label.push_str(", ");
        label.push_str(note);
    }
}

/// The notices for the query and the cursor. The field error comes first.
fn notices(query: &SearchQuery, scope: QueryScope, cursor: usize) -> Vec<FieldListNoticeVm> {
    let mut notices = Vec::new();
    let error = query.terms().iter().find_map(|term| {
        let under_cursor = term.span.start <= cursor && cursor <= term.span.end;
        match &term.kind {
            TermKind::Field(field) if under_cursor => match &field.value {
                FieldValueState::Invalid(error) => Some(error.message()),
                FieldValueState::Valid(_) | FieldValueState::Incomplete => None,
            },
            _ => None,
        }
    });
    if let Some(text) = error {
        notices.push(FieldListNoticeVm {
            kind: FieldListNoticeKind::FieldError,
            a11y_label: format!("Field error, {text}"),
            text,
        });
    }
    if scope == QueryScope::Conflict {
        notices.push(FieldListNoticeVm {
            kind: FieldListNoticeKind::ScopeConflict,
            text: SCOPE_CONFLICT_TEXT.to_owned(),
            a11y_label: format!("Scope conflict, {SCOPE_CONFLICT_TEXT}"),
        });
    }
    notices
}

/// The accessibility label of the list.
fn list_a11y_label(row_count: usize) -> String {
    if row_count == 1 {
        "Search field list, 1 row".to_owned()
    } else {
        format!("Search field list, {row_count} rows")
    }
}

#[cfg(test)]
impl FieldListEdit {
    /// Gives the changed text. The view puts an edit into the search field
    /// through the input, so only the tests call this function now.
    #[must_use]
    pub fn apply(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len() + self.text.len());
        out.push_str(&text[..self.range.start]);
        out.push_str(&self.text);
        out.push_str(&text[self.range.end..]);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_models::search_query::QUERY_FIELDS;

    fn state(opening: FieldListOpening) -> FieldListState {
        FieldListState {
            opening,
            focused: true,
            selected: None,
        }
    }

    fn build_at(text: &str, cursor: usize, opening: FieldListOpening) -> Option<FieldListVm> {
        FieldListVm::build(text, cursor..cursor, &state(opening))
    }

    fn build_end(text: &str, opening: FieldListOpening) -> Option<FieldListVm> {
        build_at(text, text.len(), opening)
    }

    fn labels(vm: &FieldListVm) -> Vec<&str> {
        vm.rows.iter().map(|row| row.label.as_str()).collect()
    }

    fn row<'a>(vm: &'a FieldListVm, label: &str) -> &'a FieldListRowVm {
        vm.rows
            .iter()
            .find(|row| row.label == label)
            .expect("the row exists")
    }

    fn with_selected(mut vm: FieldListVm, index: Option<usize>) -> FieldListVm {
        vm.selected = index;
        for (i, row) in vm.rows.iter_mut().enumerate() {
            row.selected = Some(i) == index;
        }
        vm
    }

    fn tab_text(text: &str, cursor: usize) -> (String, usize) {
        let vm = with_selected(
            build_at(text, cursor, FieldListOpening::Typing).expect("the list shows"),
            Some(0),
        );
        let FieldListKeyOutcome::Apply(edit) = vm.key(FieldListKey::Tab) else {
            panic!("Tab applies the selected row");
        };
        (edit.apply(text), edit.cursor)
    }

    #[test]
    fn adr_0087_field_list_r87_4_01_empty_text_shows_each_field() {
        let vm = build_end("", FieldListOpening::Requested).expect("the list shows");
        assert_eq!(vm.rows.len(), 11);
        for (row, info) in vm.rows.iter().zip(QUERY_FIELDS.iter()) {
            assert_eq!(row.kind, FieldListRowKind::Field { field: info.name });
            assert_eq!(row.label, info.token);
            assert_eq!(row.values_text.as_deref(), Some(info.values_text));
            assert_eq!(row.example_text.as_deref(), Some(info.example));
            assert!(!row.selected);
        }
        assert!(vm.notices.is_empty());
        assert_eq!(vm.selected, None);
    }

    #[test]
    fn adr_0087_field_list_r87_4_02_field_name_prefix_filters_rows() {
        let vm = build_at("ar", 2, FieldListOpening::Typing).expect("the list shows");
        assert_eq!(labels(&vm), vec!["artist:"]);
        let vm = build_at("-ar", 3, FieldListOpening::Typing).expect("the list shows");
        assert_eq!(labels(&vm), vec!["artist:"]);
        assert_eq!(build_at("zz", 2, FieldListOpening::Typing), None);
    }

    #[test]
    fn adr_0087_field_list_r87_4_03_kind_shows_its_fixed_values() {
        let vm = build_at("kind:", 5, FieldListOpening::Typing).expect("the list shows");
        assert_eq!(labels(&vm), vec!["album", "ep", "single"]);
        for (row, value) in vm
            .rows
            .iter()
            .zip(QueryFieldName::Kind.fixed_values().iter())
        {
            assert_eq!(row.values_text.as_deref(), Some(value.description));
            assert_eq!(
                row.kind,
                FieldListRowKind::FixedValue {
                    field: QueryFieldName::Kind,
                    word: value.word,
                }
            );
        }
        let vm = build_end("kind:s", FieldListOpening::Typing).expect("the list shows");
        assert_eq!(labels(&vm), vec!["single"]);
        let vm = build_at("kind:album,", 11, FieldListOpening::Typing).expect("the list shows");
        assert_eq!(vm.rows.len(), 3);
    }

    #[test]
    fn adr_0087_field_list_r87_4_04_fixed_values_of_in_is_and_has() {
        let count = |text: &str| {
            build_end(text, FieldListOpening::Typing)
                .expect("the list shows")
                .rows
                .len()
        };
        assert_eq!(count("in:"), 2);
        assert_eq!(count("is:"), 4);
        assert_eq!(count("has:"), 2);
        assert_eq!(build_at("artist:", 7, FieldListOpening::Typing), None);
    }

    #[test]
    fn adr_0087_field_list_r87_4_05_library_only_fields_have_a_mark() {
        let vm = build_end("", FieldListOpening::Requested).expect("the list shows");
        for row in &vm.rows {
            let library_only = matches!(row.label.as_str(), "is:" | "playlist:" | "has:");
            let expected = library_only.then_some(FieldListMark::LibraryOnly);
            assert_eq!(row.mark, expected, "row {}", row.label);
        }
        assert_eq!(FieldListMark::LibraryOnly.label(), "Library only");
    }

    #[test]
    fn adr_0087_field_list_r87_4_06_field_error_under_the_cursor() {
        let vm = build_at("after:yesterday", 15, FieldListOpening::Typing).expect("a notice");
        assert!(vm.rows.is_empty());
        assert_eq!(vm.notices.len(), 1);
        assert_eq!(vm.notices[0].kind, FieldListNoticeKind::FieldError);
        assert_eq!(
            vm.notices[0].text,
            "after: needs a date, for example 2026-10-01"
        );
        let vm =
            build_at("after:yesterday ar", 18, FieldListOpening::Typing).expect("the list shows");
        assert_eq!(labels(&vm), vec!["artist:"]);
        assert!(vm.notices.is_empty());
    }

    #[test]
    fn adr_0087_field_list_r87_4_07_scope_conflict_makes_rows_unavailable() {
        let vm = build_at("in:index ", 9, FieldListOpening::Requested).expect("the list shows");
        for row in &vm.rows {
            let library_only = matches!(row.label.as_str(), "is:" | "playlist:" | "has:");
            if library_only {
                assert_eq!(
                    row.availability,
                    FieldListRowAvailability::Unavailable(FieldListRowUnavailable::ScopeConflict)
                );
                assert_eq!(row.note.as_deref(), Some("Not available with in:index"));
            } else {
                assert_eq!(row.availability, FieldListRowAvailability::Available);
                assert_eq!(row.note, None);
            }
        }
        let vm = build_at("is:ready in:", 12, FieldListOpening::Typing).expect("the list shows");
        assert_eq!(
            row(&vm, "index").availability,
            FieldListRowAvailability::Unavailable(FieldListRowUnavailable::ScopeConflict)
        );
        assert_eq!(
            row(&vm, "index").note.as_deref(),
            Some("Not available with a Library-only field or in:library")
        );
        assert_eq!(
            row(&vm, "library").availability,
            FieldListRowAvailability::Available
        );
        let vm = build_end("in:index is:ready ", FieldListOpening::Typing).expect("a notice");
        let kinds: Vec<_> = vm.notices.iter().map(|notice| notice.kind).collect();
        assert_eq!(kinds, vec![FieldListNoticeKind::ScopeConflict]);
        assert_eq!(
            vm.notices[0].text,
            "This query asks for the Library only and for MusicIndex only. It shows no rows."
        );
    }

    #[test]
    fn adr_0087_field_list_r87_4_08_up_and_down_wrap_and_skip_unavailable_rows() {
        let vm = build_end("kind:", FieldListOpening::Typing).expect("the list shows");
        assert_eq!(vm.rows.len(), 3);
        let at = |index| with_selected(vm.clone(), index);
        assert_eq!(
            at(None).key(FieldListKey::Down),
            FieldListKeyOutcome::Select(0)
        );
        assert_eq!(
            at(Some(2)).key(FieldListKey::Down),
            FieldListKeyOutcome::Select(0)
        );
        assert_eq!(
            at(None).key(FieldListKey::Up),
            FieldListKeyOutcome::Select(2)
        );
        assert_eq!(
            at(Some(0)).key(FieldListKey::Up),
            FieldListKeyOutcome::Select(2)
        );

        // In `in:index `, the rows 8 to 10 (`is:`, `playlist:`, `has:`)
        // are not available.
        let vm = build_at("in:index ", 9, FieldListOpening::Requested).expect("the list shows");
        let at = |index| with_selected(vm.clone(), index);
        assert_eq!(
            at(Some(7)).key(FieldListKey::Down),
            FieldListKeyOutcome::Select(0)
        );
        assert_eq!(
            at(Some(0)).key(FieldListKey::Up),
            FieldListKeyOutcome::Select(7)
        );
        assert_eq!(
            at(None).key(FieldListKey::Up),
            FieldListKeyOutcome::Select(7)
        );

        let mut vm = build_end("kind:", FieldListOpening::Typing).expect("the list shows");
        for row in &mut vm.rows {
            row.availability =
                FieldListRowAvailability::Unavailable(FieldListRowUnavailable::ScopeConflict);
        }
        assert_eq!(vm.key(FieldListKey::Up), FieldListKeyOutcome::Pass);
        assert_eq!(vm.key(FieldListKey::Down), FieldListKeyOutcome::Pass);
    }

    #[test]
    fn adr_0087_field_list_r87_4_09_tab_puts_the_field_token_into_the_text() {
        let vm = with_selected(
            build_at("ar", 2, FieldListOpening::Typing).expect("the list shows"),
            Some(0),
        );
        assert_eq!(
            vm.key(FieldListKey::Tab),
            FieldListKeyOutcome::Apply(FieldListEdit {
                range: 0..2,
                text: "artist:".to_owned(),
                cursor: 7,
            })
        );
        assert_eq!(tab_text("ar", 2), ("artist:".to_owned(), 7));
        assert_eq!(
            tab_text("haleen -ar", 10),
            ("haleen -artist:".to_owned(), 15)
        );
        assert_eq!(tab_text("arjsmovie", 2), ("artist:jsmovie".to_owned(), 7));
    }

    #[test]
    fn adr_0087_field_list_r87_4_10_tab_puts_the_value_into_the_text() {
        assert_eq!(tab_text("kind:s", 6), ("kind:single ".to_owned(), 12));
        assert_eq!(tab_text("kind:s,ep", 6), ("kind:single,ep".to_owned(), 11));

        let vm = build_end("kind:", FieldListOpening::Typing).expect("the list shows");
        assert_eq!(
            vm.key(FieldListKey::Tab),
            FieldListKeyOutcome::Apply(vm.rows[0].edit.clone())
        );
        let mut vm = vm;
        vm.rows[0].availability =
            FieldListRowAvailability::Unavailable(FieldListRowUnavailable::ScopeConflict);
        assert_eq!(
            vm.key(FieldListKey::Tab),
            FieldListKeyOutcome::Apply(vm.rows[1].edit.clone())
        );
        for row in &mut vm.rows {
            row.availability =
                FieldListRowAvailability::Unavailable(FieldListRowUnavailable::ScopeConflict);
        }
        assert_eq!(vm.key(FieldListKey::Tab), FieldListKeyOutcome::Pass);
    }

    #[test]
    fn adr_0087_field_list_r87_4_11_enter_applies_only_a_selected_row() {
        let vm = build_end("kind:", FieldListOpening::Typing).expect("the list shows");
        let selected = with_selected(vm.clone(), Some(2));
        assert_eq!(
            selected.key(FieldListKey::Enter),
            selected.key(FieldListKey::Tab)
        );
        assert_eq!(
            selected.key(FieldListKey::Enter),
            FieldListKeyOutcome::Apply(vm.rows[2].edit.clone())
        );
        assert_eq!(vm.key(FieldListKey::Enter), FieldListKeyOutcome::Pass);
    }

    #[test]
    fn adr_0087_field_list_r87_4_12_escape_closes_and_change_or_click_opens() {
        let vm = build_end("", FieldListOpening::Requested).expect("the list shows");
        assert_eq!(vm.key(FieldListKey::Escape), FieldListKeyOutcome::Close);

        let mut state = state(FieldListOpening::Typing);
        state.apply(FieldListEvent::Escape);
        assert_eq!(FieldListVm::build("ar", 2..2, &state), None);
        state.apply(FieldListEvent::Change);
        assert!(FieldListVm::build("ar", 2..2, &state).is_some());
        state.apply(FieldListEvent::Escape);
        state.apply(FieldListEvent::Click);
        assert_eq!(state.opening, FieldListOpening::Requested);
        assert!(FieldListVm::build("ar", 2..2, &state).is_some());
    }

    #[test]
    fn adr_0087_field_list_r87_4_13_events_change_the_state() {
        let mut state = state(FieldListOpening::Requested);
        state.apply(FieldListEvent::Blur);
        assert_eq!(state.opening, FieldListOpening::Closed);
        assert!(!state.focused);
        state.apply(FieldListEvent::Change);
        assert_eq!(state.opening, FieldListOpening::Closed);

        for event in [FieldListEvent::Submit, FieldListEvent::Inserted] {
            let mut open = FieldListState {
                opening: FieldListOpening::Typing,
                focused: true,
                selected: None,
            };
            open.apply(event);
            assert_eq!(open.opening, FieldListOpening::Closed, "{event:?}");
        }

        state.apply(FieldListEvent::Focus);
        assert_eq!(state.opening, FieldListOpening::Requested);
        assert!(state.focused);

        for event in [FieldListEvent::Change, FieldListEvent::CursorMoved] {
            state.select(Some(1));
            state.apply(event);
            assert_eq!(state.selected, None, "{event:?}");
        }
        state.select(Some(1));
        let opening = state.opening;
        state.apply(FieldListEvent::CursorMoved);
        assert_eq!(state.opening, opening);
    }

    #[test]
    fn adr_0087_field_list_r87_4_14_typing_after_a_space_shows_no_full_list() {
        assert_eq!(build_at("haleen ", 7, FieldListOpening::Typing), None);
        let vm = build_at("haleen ", 7, FieldListOpening::Requested).expect("the list shows");
        assert_eq!(vm.rows.len(), 11);
        let vm = build_end("", FieldListOpening::Typing).expect("the list shows");
        assert_eq!(vm.rows.len(), 11);
        for text in ["", "ar", "kind:", "haleen "] {
            for opening in [FieldListOpening::Requested, FieldListOpening::Typing] {
                assert_eq!(
                    FieldListVm::build(text, 0..2, &state(opening)),
                    None,
                    "{text:?}"
                );
            }
        }
    }

    #[test]
    fn adr_0087_field_list_r87_4_15_labels_and_actions() {
        let vm = build_end("", FieldListOpening::Requested).expect("the list shows");
        assert_eq!(
            row(&vm, "artist:").a11y_label,
            "Field artist:, an artist name, for example artist:jsmovie"
        );
        assert!(row(&vm, "is:").a11y_label.ends_with(", Library only"));
        for row in &vm.rows {
            let FieldListRowKind::Field { field } = row.kind else {
                panic!("each row is a field row");
            };
            assert_eq!(row.action, FieldListRowAction::InsertField(field));
            assert_eq!(row.availability, FieldListRowAvailability::Available);
        }

        let vm = build_end("kind:", FieldListOpening::Typing).expect("the list shows");
        assert_eq!(
            row(&vm, "single").a11y_label,
            "Value single for kind:, A release with one track"
        );
        for row in &vm.rows {
            let FieldListRowKind::FixedValue { field, word } = row.kind else {
                panic!("each row is a value row");
            };
            assert_eq!(row.action, FieldListRowAction::InsertValue { field, word });
        }

        let vm = build_at("in:index ", 9, FieldListOpening::Requested).expect("the list shows");
        assert_eq!(
            row(&vm, "is:").a11y_label,
            "Field is:, downloaded, partial, ready, removed, for example is:downloaded, Library only, \
             Not available with in:index"
        );
    }

    #[test]
    fn adr_0087_field_list_r87_4_16_edits_keep_character_boundaries() {
        let text = "café ar";
        let vm = with_selected(
            build_at(text, 8, FieldListOpening::Typing).expect("the list shows"),
            Some(0),
        );
        let FieldListKeyOutcome::Apply(edit) = vm.key(FieldListKey::Tab) else {
            panic!("Tab applies the row");
        };
        assert_eq!(edit.range, 6..8);
        let changed = edit.apply(text);
        assert_eq!(changed, "café artist:");
        assert_eq!(edit.cursor, 13);

        for (text, opening) in [
            ("café ar", FieldListOpening::Typing),
            ("é kind:s", FieldListOpening::Typing),
            ("naïve ", FieldListOpening::Requested),
            ("ü in:", FieldListOpening::Typing),
        ] {
            let vm = build_end(text, opening).expect("the list shows");
            for row in &vm.rows {
                assert!(text.is_char_boundary(row.edit.range.start), "{text:?}");
                assert!(text.is_char_boundary(row.edit.range.end), "{text:?}");
                let changed = row.edit.apply(text);
                assert!(changed.is_char_boundary(row.edit.cursor), "{text:?}");
            }
        }
    }
}
