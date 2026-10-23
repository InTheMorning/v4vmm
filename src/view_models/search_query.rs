//! The parser of the Music search query text (ADR 0087 Decisions 1 and 2).
//!
//! The toolbar search field holds one query text with words, phrases and
//! fields. This module is the one parser of that text. It gives a typed
//! query: the words, the phrases, the fields, the exclusions and one typed
//! error for each field value that is not correct. No other code reads the
//! field syntax (ADR 0087 invariant "One parser reads the query text").
//!
//! The module also holds the field table, the fixed values of each field
//! and the scope rules. It gives the words for the Index `q` and the term at
//! the text cursor. Each range is a byte range on a character boundary of
//! the text.
//!
//! The module has no GPUI type and no I/O.

#![warn(clippy::pedantic)]

use std::ops::Range;

use chrono::{Months, NaiveDate, NaiveTime};

/// The double quote character that opens and closes a phrase.
const QUOTE: char = '\u{22}';

/// The character that escapes a quote or a backslash inside quotes.
const BACKSLASH: char = '\\';

/// The number of seconds in one day.
const SECONDS_PER_DAY: i64 = 86_400;

/// The name of one query field (ADR 0087 Decision 2).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum QueryFieldName {
    Artist,
    Title,
    Release,
    Kind,
    In,
    After,
    Before,
    Duration,
    Is,
    Playlist,
    Has,
}

/// The kind of value that one query field reads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldValueKind {
    /// A text that is not empty, bare or in quotes.
    Text,
    /// One or more release kinds, with commas.
    KindList,
    /// `library` or `index`.
    Scope,
    /// A date or a month.
    Date,
    /// A comparison or a range of durations.
    Duration,
    /// One Library state of a track.
    LibraryState,
    /// One kind of content.
    Content,
}

/// One row of the field table that the field list shows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QueryFieldInfo {
    /// The field name.
    pub name: QueryFieldName,
    /// The field name in lowercase with its colon, for example `artist:`.
    pub token: &'static str,
    /// The kind of value that the field reads.
    pub value_kind: FieldValueKind,
    /// A short text that names the correct values.
    pub values_text: &'static str,
    /// One correct example of the field, with its token.
    pub example: &'static str,
    /// What the field matches.
    pub description: &'static str,
    /// `true` when only the Library can apply the field.
    pub library_only: bool,
}

/// One fixed value of a field, with a short description.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixedValueInfo {
    /// The value word, in lowercase.
    pub word: &'static str,
    /// What the value means.
    pub description: &'static str,
}

/// The field table, in the order of ADR 0087 Decision 2.
pub const QUERY_FIELDS: [QueryFieldInfo; 11] = [
    QueryFieldInfo {
        name: QueryFieldName::Artist,
        token: "artist:",
        value_kind: FieldValueKind::Text,
        values_text: "an artist name",
        example: "artist:jsmovie",
        description: "The track artist or the album artist",
        library_only: false,
    },
    QueryFieldInfo {
        name: QueryFieldName::Title,
        token: "title:",
        value_kind: FieldValueKind::Text,
        values_text: "a title",
        example: "title:\"night drive\"",
        description: "The track title or the release title",
        library_only: false,
    },
    QueryFieldInfo {
        name: QueryFieldName::Release,
        token: "release:",
        value_kind: FieldValueKind::Text,
        values_text: "a release title",
        example: "release:\"aged friends\"",
        description: "The release title",
        library_only: false,
    },
    QueryFieldInfo {
        name: QueryFieldName::Kind,
        token: "kind:",
        value_kind: FieldValueKind::KindList,
        values_text: "album, ep, single",
        example: "kind:single",
        description: "The release kind. Use commas for more than one kind",
        library_only: false,
    },
    QueryFieldInfo {
        name: QueryFieldName::In,
        token: "in:",
        value_kind: FieldValueKind::Scope,
        values_text: "library, index",
        example: "in:library",
        description: "Search only the Library or only the Index",
        library_only: false,
    },
    QueryFieldInfo {
        name: QueryFieldName::After,
        token: "after:",
        value_kind: FieldValueKind::Date,
        values_text: "a date or a month",
        example: "after:2026-10-01",
        description: "Released on this date or after it",
        library_only: false,
    },
    QueryFieldInfo {
        name: QueryFieldName::Before,
        token: "before:",
        value_kind: FieldValueKind::Date,
        values_text: "a date or a month",
        example: "before:2026-10",
        description: "Released on this date or before it",
        library_only: false,
    },
    QueryFieldInfo {
        name: QueryFieldName::Duration,
        token: "duration:",
        value_kind: FieldValueKind::Duration,
        values_text: ">5m, <3:30, 3m..5m",
        example: "duration:3m..5m",
        description: "The track duration",
        library_only: false,
    },
    QueryFieldInfo {
        name: QueryFieldName::Is,
        token: "is:",
        value_kind: FieldValueKind::LibraryState,
        values_text: "downloaded, partial, ready, removed",
        example: "is:downloaded",
        description: "The Library state of the track",
        library_only: true,
    },
    QueryFieldInfo {
        name: QueryFieldName::Playlist,
        token: "playlist:",
        value_kind: FieldValueKind::Text,
        values_text: "a playlist name",
        example: "playlist:\"friday show\"",
        description: "The tracks of a playlist",
        library_only: true,
    },
    QueryFieldInfo {
        name: QueryFieldName::Has,
        token: "has:",
        value_kind: FieldValueKind::Content,
        values_text: "split, image",
        example: "has:split",
        description: "A value split or an image of its own",
        library_only: true,
    },
];

/// The field table as a static, so that a row reference lives for the
/// full program.
static FIELD_ROWS: [QueryFieldInfo; 11] = QUERY_FIELDS;

const KIND_VALUES: &[FixedValueInfo] = &[
    FixedValueInfo {
        word: "album",
        description: "A release with many tracks",
    },
    FixedValueInfo {
        word: "ep",
        description: "A short release",
    },
    FixedValueInfo {
        word: "single",
        description: "A release with one track",
    },
];

const SCOPE_VALUES: &[FixedValueInfo] = &[
    FixedValueInfo {
        word: "library",
        description: "Only the Library",
    },
    FixedValueInfo {
        word: "index",
        description: "Only MusicIndex",
    },
];

const LIBRARY_STATE_VALUES: &[FixedValueInfo] = &[
    FixedValueInfo {
        word: "downloaded",
        description: "Each track is downloaded",
    },
    FixedValueInfo {
        word: "partial",
        description: "Some tracks are downloaded",
    },
    FixedValueInfo {
        word: "ready",
        description: "Ready for a show",
    },
    FixedValueInfo {
        word: "removed",
        description: "Removed from its feed",
    },
];

const CONTENT_VALUES: &[FixedValueInfo] = &[
    FixedValueInfo {
        word: "split",
        description: "A value block with payment routes",
    },
    FixedValueInfo {
        word: "image",
        description: "An image of its own",
    },
];

impl QueryFieldName {
    /// Reads a field name without its colon, without case.
    ///
    /// Returns `None` for a name that is not in the field table.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        FIELD_ROWS
            .iter()
            .find(|info| {
                info.token
                    .strip_suffix(':')
                    .is_some_and(|bare| bare.eq_ignore_ascii_case(name))
            })
            .map(|info| info.name)
    }

    /// The row of the field table for this field.
    #[must_use]
    pub fn info(self) -> &'static QueryFieldInfo {
        &FIELD_ROWS[self as usize]
    }

    /// `true` when only the Library can apply this field.
    #[must_use]
    pub fn is_library_only(self) -> bool {
        self.info().library_only
    }

    /// The fixed values of this field. A field with free values gives an
    /// empty slice.
    #[must_use]
    pub fn fixed_values(self) -> &'static [FixedValueInfo] {
        match self {
            Self::Kind => KIND_VALUES,
            Self::In => SCOPE_VALUES,
            Self::Is => LIBRARY_STATE_VALUES,
            Self::Has => CONTENT_VALUES,
            Self::Artist
            | Self::Title
            | Self::Release
            | Self::After
            | Self::Before
            | Self::Duration
            | Self::Playlist => &[],
        }
    }

    /// The fixed values of this field whose word starts with `prefix`,
    /// compared without case.
    #[must_use]
    pub fn values_with_prefix(self, prefix: &str) -> Vec<&'static FixedValueInfo> {
        let prefix = prefix.to_lowercase();
        self.fixed_values()
            .iter()
            .filter(|value| value.word.starts_with(&prefix))
            .collect()
    }

    /// The token of this field, for example `after:`.
    fn token(self) -> &'static str {
        self.info().token
    }

    /// The example value of this field, without its token.
    fn example_value(self) -> &'static str {
        let info = self.info();
        info.example
            .strip_prefix(info.token)
            .unwrap_or(info.example)
    }
}

/// The rows of the field table whose token starts with `prefix`, compared
/// without case. An empty prefix gives each row.
#[must_use]
pub fn fields_with_prefix(prefix: &str) -> Vec<&'static QueryFieldInfo> {
    let prefix = prefix.to_lowercase();
    FIELD_ROWS
        .iter()
        .filter(|info| info.token.starts_with(&prefix))
        .collect()
}

/// One release kind that `kind:` reads (ADR 0084 Decision 5).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KindValue {
    Album,
    Ep,
    Single,
}

impl KindValue {
    /// The query word of this kind: `album`, `ep` or `single`.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Album => "album",
            Self::Ep => "ep",
            Self::Single => "single",
        }
    }

    fn from_word(word: &str) -> Option<Self> {
        [Self::Album, Self::Ep, Self::Single]
            .into_iter()
            .find(|kind| kind.word().eq_ignore_ascii_case(word))
    }
}

/// The scope that `in:` sets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScopeValue {
    Library,
    Index,
}

/// The Library state that `is:` reads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LibraryStateValue {
    Downloaded,
    Partial,
    Ready,
    Removed,
}

/// The content that `has:` reads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContentValue {
    Split,
    Image,
}

/// A date or a month that `after:` or `before:` reads.
///
/// A day gives the same day twice. A month gives its first day and its last
/// day. The dates are in UTC, the same time zone as `fmt_date`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DateValue {
    /// The first day that the value includes.
    pub first_day: NaiveDate,
    /// The last day that the value includes.
    pub last_day: NaiveDate,
}

impl DateValue {
    /// The first second of `first_day` in UTC, as Unix seconds.
    #[must_use]
    pub fn start_unix(&self) -> i64 {
        self.first_day
            .and_time(NaiveTime::MIN)
            .and_utc()
            .timestamp()
    }

    /// The last second of `last_day` in UTC, as Unix seconds.
    #[must_use]
    pub fn end_unix(&self) -> i64 {
        self.last_day.and_time(NaiveTime::MIN).and_utc().timestamp() + SECONDS_PER_DAY - 1
    }
}

/// A duration range that `duration:` reads. Both limits are inclusive
/// seconds. A limit with no value does not apply.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DurationRange {
    /// The shortest duration that matches.
    pub min_secs: Option<u32>,
    /// The longest duration that matches.
    pub max_secs: Option<u32>,
}

impl DurationRange {
    /// `true` when `secs` is in the range.
    #[must_use]
    pub fn contains(&self, secs: u32) -> bool {
        self.min_secs.is_none_or(|min| secs >= min) && self.max_secs.is_none_or(|max| secs <= max)
    }
}

/// The typed value of a correct field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FieldValue {
    Text(String),
    Kinds(Vec<KindValue>),
    Scope(ScopeValue),
    Date(DateValue),
    Duration(DurationRange),
    LibraryState(LibraryStateValue),
    Content(ContentValue),
}

/// The error of one field value that is not correct.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FieldValueError {
    /// The value is not a word of the fixed values of the field.
    UnknownValue {
        field: QueryFieldName,
        value: String,
    },
    /// The field takes one value, but the value has a comma.
    OneValueOnly { field: QueryFieldName },
    /// The value of `after:` or `before:` is not a date or a month.
    NotADate { field: QueryFieldName },
    /// The value of `duration:` is not a comparison or a range.
    NotADuration,
    /// `in:` has a `-` (packet decision D6).
    ExcludedScope,
}

impl FieldValueError {
    /// The text that the field list shows. It starts with the field token
    /// and gives one correct example.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::UnknownValue { field, value } => format!(
                "{} cannot read {value}. Use one of these values: {}",
                field.token(),
                field.info().values_text
            ),
            Self::OneValueOnly { field } => format!(
                "{} takes one value, for example {}",
                field.token(),
                field.info().example
            ),
            Self::NotADate { field } => format!(
                "{} needs a date, for example {}",
                field.token(),
                field.example_value()
            ),
            Self::NotADuration => {
                let field = QueryFieldName::Duration;
                format!(
                    "{} needs a comparison or a range, for example {}",
                    field.token(),
                    field.example_value()
                )
            }
            Self::ExcludedScope => {
                let field = QueryFieldName::In;
                format!(
                    "{} cannot have a -. Use {}library or {}index",
                    field.token(),
                    field.token(),
                    field.token()
                )
            }
        }
    }
}

/// The state of the value of one field term.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FieldValueState {
    /// The value is correct.
    Valid(FieldValue),
    /// The value is empty. The field applies nothing, and this is not an
    /// error (packet decision D5).
    Incomplete,
    /// The value is not correct. The field applies nothing.
    Invalid(FieldValueError),
}

/// One field term: a field name and the state of its value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldTerm {
    /// The field name.
    pub name: QueryFieldName,
    /// The state of the field value.
    pub value: FieldValueState,
}

/// The kind of one query term.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TermKind {
    Word(String),
    Phrase(String),
    Field(FieldTerm),
}

/// One term of the query text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryTerm {
    /// The byte range of the term in the text, with its `-`.
    pub span: Range<usize>,
    /// `true` when the term has a `-`.
    pub excluded: bool,
    /// The word, the phrase or the field.
    pub kind: TermKind,
}

/// The scope of a query (ADR 0087 Decision 2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryScope {
    /// The Library and the Index.
    Everything,
    /// Only the Library.
    LibraryOnly,
    /// Only the Index.
    IndexOnly,
    /// The fields cannot apply together. The query shows no rows.
    Conflict,
}

/// The typed query of one query text (ADR 0087 Decision 1).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SearchQuery {
    terms: Vec<QueryTerm>,
}

impl SearchQuery {
    /// Parses the query text into its terms, in text order.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let terms = lex(text)
            .into_iter()
            .filter_map(RawTerm::into_term)
            .collect();
        Self { terms }
    }

    /// The terms, in text order.
    #[must_use]
    pub fn terms(&self) -> &[QueryTerm] {
        &self.terms
    }

    /// The scope that the correct fields set.
    #[must_use]
    pub fn scope(&self) -> QueryScope {
        let mut library_field = false;
        let mut in_library = false;
        let mut in_index = false;
        for field in self.valid_fields() {
            if field.name.is_library_only() {
                library_field = true;
            }
            match field.value {
                FieldValueState::Valid(FieldValue::Scope(ScopeValue::Library)) => {
                    in_library = true;
                }
                FieldValueState::Valid(FieldValue::Scope(ScopeValue::Index)) => in_index = true,
                _ => {}
            }
        }
        if in_index && (library_field || in_library) {
            QueryScope::Conflict
        } else if in_index {
            QueryScope::IndexOnly
        } else if library_field || in_library {
            QueryScope::LibraryOnly
        } else {
            QueryScope::Everything
        }
    }

    /// The words and the phrases without `-`, for the Index `q`.
    ///
    /// The terms keep their text order, with one space between them. A
    /// phrase keeps its double quotes. Inside a phrase, a quote is written
    /// `\"` and a backslash is written `\\`. With no such word or phrase,
    /// the result is `None`. The result never holds a field.
    #[must_use]
    pub fn index_q(&self) -> Option<String> {
        let parts: Vec<String> = self
            .terms
            .iter()
            .filter(|term| !term.excluded)
            .filter_map(|term| match &term.kind {
                TermKind::Word(word) => Some(word.clone()),
                TermKind::Phrase(phrase) => Some(quote_phrase(phrase)),
                TermKind::Field(_) => None,
            })
            .collect();
        (!parts.is_empty()).then(|| parts.join(" "))
    }

    /// The terms that the Index must apply as a filter: each correct field
    /// other than `in:`, and each word or phrase with `-`.
    #[must_use]
    pub fn index_filter_terms(&self) -> Vec<&QueryTerm> {
        self.terms
            .iter()
            .filter(|term| match &term.kind {
                TermKind::Word(_) | TermKind::Phrase(_) => term.excluded,
                TermKind::Field(field) => is_valid_filter(field),
            })
            .collect()
    }

    /// `true` when the query holds one term without `-` that can run: a word
    /// or a phrase with an alphanumeric character, or a correct field other
    /// than `in:` (ADR 0087 Decision 4).
    #[must_use]
    pub fn has_runnable_content(&self) -> bool {
        self.terms
            .iter()
            .filter(|term| !term.excluded)
            .any(|term| match &term.kind {
                TermKind::Word(text) | TermKind::Phrase(text) => {
                    text.chars().any(char::is_alphanumeric)
                }
                TermKind::Field(field) => is_valid_filter(field),
            })
    }

    /// Each field value error with its term, in text order.
    #[must_use]
    pub fn field_errors(&self) -> Vec<(&QueryTerm, &FieldValueError)> {
        self.terms
            .iter()
            .filter_map(|term| match &term.kind {
                TermKind::Field(FieldTerm {
                    value: FieldValueState::Invalid(error),
                    ..
                }) => Some((term, error)),
                _ => None,
            })
            .collect()
    }

    fn valid_fields(&self) -> impl Iterator<Item = &FieldTerm> {
        self.terms.iter().filter_map(|term| match &term.kind {
            TermKind::Field(field) if matches!(field.value, FieldValueState::Valid(_)) => {
                Some(field)
            }
            _ => None,
        })
    }
}

/// `true` for a correct field other than `in:`.
fn is_valid_filter(field: &FieldTerm) -> bool {
    field.name != QueryFieldName::In && matches!(field.value, FieldValueState::Valid(_))
}

/// Writes a phrase in double quotes, with each quote and each backslash
/// escaped.
fn quote_phrase(phrase: &str) -> String {
    let mut out = String::with_capacity(phrase.len() + 2);
    out.push(QUOTE);
    for ch in phrase.chars() {
        if ch == QUOTE || ch == BACKSLASH {
            out.push(BACKSLASH);
        }
        out.push(ch);
    }
    out.push(QUOTE);
    out
}

/// The term under the text cursor, for the field list.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CursorTerm {
    /// The byte range of the term after its `-`.
    pub range: Range<usize>,
    /// `true` when the term has a `-`.
    pub excluded: bool,
    /// The part of the term that the curator types.
    pub partial: CursorPartial,
}

/// The part of the term under the cursor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CursorPartial {
    /// The term has no colon of a known field. `prefix` is the text from the
    /// term start to the cursor.
    FieldName { prefix: String },
    /// The cursor is after the colon of a known field. `prefix` is the value
    /// text from the start of the value item to the cursor. `value_range` is
    /// the byte range of that value item.
    FieldValue {
        field: QueryFieldName,
        prefix: String,
        value_range: Range<usize>,
    },
    /// The cursor is in a phrase, or in a word whose colon name is not a
    /// field.
    Other,
}

/// Reads the term under the cursor. `cursor` is a byte offset into `text`.
///
/// An offset that is not on a character boundary moves back to the
/// boundary. An offset after the end moves to the end. The term under the
/// cursor holds the cursor or ends at the cursor. With no such term, the
/// term is empty at the cursor.
#[must_use]
pub fn cursor_term(text: &str, cursor: usize) -> CursorTerm {
    let mut cursor = cursor.min(text.len());
    while !text.is_char_boundary(cursor) {
        cursor -= 1;
    }
    let Some(raw) = lex(text)
        .into_iter()
        .find(|raw| raw.span.start <= cursor && cursor <= raw.span.end)
    else {
        return CursorTerm {
            range: cursor..cursor,
            excluded: false,
            partial: CursorPartial::FieldName {
                prefix: String::new(),
            },
        };
    };
    let body_start = raw.body_start;
    let typed_end = cursor.max(body_start);
    let partial = match &raw.shape {
        RawShape::Empty => CursorPartial::FieldName {
            prefix: String::new(),
        },
        RawShape::Phrase { .. } => CursorPartial::Other,
        RawShape::Word { text: word } => {
            if word.contains(':') {
                CursorPartial::Other
            } else {
                CursorPartial::FieldName {
                    prefix: text[body_start..typed_end].to_owned(),
                }
            }
        }
        RawShape::Field {
            name,
            colon,
            value_raw,
            ..
        } => {
            if typed_end <= *colon {
                CursorPartial::FieldName {
                    prefix: text[body_start..typed_end].to_owned(),
                }
            } else {
                let typed = typed_end.clamp(value_raw.start, value_raw.end);
                let (item_start, item_end) = if *name == QueryFieldName::Kind {
                    let item_start = text[value_raw.start..typed]
                        .rfind(',')
                        .map_or(value_raw.start, |comma| value_raw.start + comma + 1);
                    let item_end = text[typed..value_raw.end]
                        .find(',')
                        .map_or(value_raw.end, |comma| typed + comma);
                    (item_start, item_end)
                } else {
                    (value_raw.start, value_raw.end)
                };
                CursorPartial::FieldValue {
                    field: *name,
                    prefix: text[item_start..typed].to_owned(),
                    value_range: item_start..item_end,
                }
            }
        }
    };
    CursorTerm {
        range: body_start..raw.span.end,
        excluded: raw.excluded,
        partial,
    }
}

/// The shape of one raw term, before the parser checks its value.
#[derive(Debug)]
enum RawShape {
    /// A lone `-` with no text after it.
    Empty,
    /// A word. A colon in it does not start a known field.
    Word { text: String },
    /// A phrase in quotes, with its escapes read.
    Phrase { content: String },
    /// A known field name, its colon and its value.
    Field {
        name: QueryFieldName,
        /// The byte offset of the colon.
        colon: usize,
        /// The byte range of the value text, inside the quotes when the
        /// value has quotes.
        value_raw: Range<usize>,
        /// The value text, with its escapes read.
        value: String,
    },
}

/// One raw term of the text, with the byte offsets that the cursor needs.
#[derive(Debug)]
struct RawTerm {
    /// The byte range of the term, with its `-`.
    span: Range<usize>,
    /// The byte offset after the `-`.
    body_start: usize,
    /// `true` when the term has a `-`.
    excluded: bool,
    shape: RawShape,
}

impl RawTerm {
    /// Gives the query term, or `None` for a term with no text.
    fn into_term(self) -> Option<QueryTerm> {
        let kind = match self.shape {
            RawShape::Empty => return None,
            RawShape::Word { text } => TermKind::Word(text),
            RawShape::Phrase { content } if content.is_empty() => return None,
            RawShape::Phrase { content } => TermKind::Phrase(content),
            RawShape::Field { name, value, .. } => TermKind::Field(FieldTerm {
                name,
                value: field_value_state(name, &value, self.excluded),
            }),
        };
        Some(QueryTerm {
            span: self.span,
            excluded: self.excluded,
            kind,
        })
    }
}

/// Reads the text from left to right into raw terms. A whitespace
/// character outside quotes ends a term.
fn lex(text: &str) -> Vec<RawTerm> {
    let mut terms = Vec::new();
    let mut pos = 0;
    while let Some(ch) = text[pos..].chars().next() {
        if ch.is_whitespace() {
            pos += ch.len_utf8();
            continue;
        }
        let excluded = ch == '-';
        let body_start = if excluded { pos + 1 } else { pos };
        let (end, shape) = lex_body(text, body_start);
        terms.push(RawTerm {
            span: pos..end,
            body_start,
            excluded,
            shape,
        });
        pos = end;
    }
    terms
}

/// Reads the term body that starts at `start`, after a possible `-`.
/// Returns the byte offset of the term end and the shape of the term.
fn lex_body(text: &str, start: usize) -> (usize, RawShape) {
    let rest = &text[start..];
    match rest.chars().next() {
        None => return (start, RawShape::Empty),
        Some(ch) if ch.is_whitespace() => return (start, RawShape::Empty),
        Some(QUOTE) => {
            let quoted = read_quoted(text, start);
            return (
                quoted.end,
                RawShape::Phrase {
                    content: quoted.content,
                },
            );
        }
        Some(_) => {}
    }
    let word_end = rest
        .find(char::is_whitespace)
        .map_or(text.len(), |offset| start + offset);
    let word = &text[start..word_end];
    if let Some(colon_offset) = word.find(':') {
        if let Some(name) = QueryFieldName::from_name(&word[..colon_offset]) {
            let colon = start + colon_offset;
            let value_start = colon + 1;
            if text[value_start..].starts_with(QUOTE) {
                let quoted = read_quoted(text, value_start);
                return (
                    quoted.end,
                    RawShape::Field {
                        name,
                        colon,
                        value_raw: value_start + 1..quoted.content_end,
                        value: quoted.content,
                    },
                );
            }
            return (
                word_end,
                RawShape::Field {
                    name,
                    colon,
                    value_raw: value_start..word_end,
                    value: text[value_start..word_end].to_owned(),
                },
            );
        }
    }
    (
        word_end,
        RawShape::Word {
            text: word.to_owned(),
        },
    )
}

/// The result of one quoted text.
struct Quoted {
    /// The text inside the quotes, with its escapes read.
    content: String,
    /// The byte offset after the closing quote, or the text end.
    end: usize,
    /// The byte offset of the closing quote, or the text end.
    content_end: usize,
}

/// Reads a quoted text that starts with the quote at `quote_at`.
///
/// Inside quotes, a backslash before a quote or a backslash gives that
/// character. Each other backslash stays in the text. A quote with no
/// closing quote closes at the end of the text (packet decision D10).
fn read_quoted(text: &str, quote_at: usize) -> Quoted {
    let base = quote_at + QUOTE.len_utf8();
    let mut content = String::new();
    let mut chars = text[base..].char_indices();
    while let Some((offset, ch)) = chars.next() {
        if ch == QUOTE {
            return Quoted {
                content,
                end: base + offset + QUOTE.len_utf8(),
                content_end: base + offset,
            };
        }
        if ch == BACKSLASH {
            match chars.next() {
                Some((_, next)) if next == QUOTE || next == BACKSLASH => content.push(next),
                Some((_, next)) => {
                    content.push(BACKSLASH);
                    content.push(next);
                }
                None => content.push(BACKSLASH),
            }
            continue;
        }
        content.push(ch);
    }
    Quoted {
        content,
        end: text.len(),
        content_end: text.len(),
    }
}

/// Checks the value of one field (ADR 0087 Decision 2).
fn field_value_state(name: QueryFieldName, value: &str, excluded: bool) -> FieldValueState {
    if value.trim().is_empty() {
        return FieldValueState::Incomplete;
    }
    match name.info().value_kind {
        FieldValueKind::Text => FieldValueState::Valid(FieldValue::Text(value.to_owned())),
        FieldValueKind::KindList => kinds_state(value),
        FieldValueKind::Scope if excluded => {
            FieldValueState::Invalid(FieldValueError::ExcludedScope)
        }
        FieldValueKind::Scope => one_value_state(name, value, read_scope, FieldValue::Scope),
        FieldValueKind::Date => match read_date(value) {
            Some(date) => FieldValueState::Valid(FieldValue::Date(date)),
            None => FieldValueState::Invalid(FieldValueError::NotADate { field: name }),
        },
        FieldValueKind::Duration => match read_duration(value) {
            Some(range) => FieldValueState::Valid(FieldValue::Duration(range)),
            None => FieldValueState::Invalid(FieldValueError::NotADuration),
        },
        FieldValueKind::LibraryState => {
            one_value_state(name, value, read_library_state, FieldValue::LibraryState)
        }
        FieldValueKind::Content => one_value_state(name, value, read_content, FieldValue::Content),
    }
}

/// Reads the comma list of `kind:`. Empty items are ignored, and a
/// repeated item counts one time.
fn kinds_state(value: &str) -> FieldValueState {
    let mut kinds = Vec::new();
    for item in value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        let Some(kind) = KindValue::from_word(item) else {
            return FieldValueState::Invalid(FieldValueError::UnknownValue {
                field: QueryFieldName::Kind,
                value: item.to_owned(),
            });
        };
        if !kinds.contains(&kind) {
            kinds.push(kind);
        }
    }
    if kinds.is_empty() {
        FieldValueState::Incomplete
    } else {
        FieldValueState::Valid(FieldValue::Kinds(kinds))
    }
}

/// Reads one fixed value of a field that takes one value only (packet
/// decision D7).
fn one_value_state<T>(
    field: QueryFieldName,
    value: &str,
    read: fn(&str) -> Option<T>,
    wrap: fn(T) -> FieldValue,
) -> FieldValueState {
    if value.contains(',') {
        return FieldValueState::Invalid(FieldValueError::OneValueOnly { field });
    }
    match read(value.trim()) {
        Some(read_value) => FieldValueState::Valid(wrap(read_value)),
        None => FieldValueState::Invalid(FieldValueError::UnknownValue {
            field,
            value: value.to_owned(),
        }),
    }
}

fn read_scope(value: &str) -> Option<ScopeValue> {
    if value.eq_ignore_ascii_case("library") {
        Some(ScopeValue::Library)
    } else if value.eq_ignore_ascii_case("index") {
        Some(ScopeValue::Index)
    } else {
        None
    }
}

fn read_library_state(value: &str) -> Option<LibraryStateValue> {
    [
        ("downloaded", LibraryStateValue::Downloaded),
        ("partial", LibraryStateValue::Partial),
        ("ready", LibraryStateValue::Ready),
        ("removed", LibraryStateValue::Removed),
    ]
    .into_iter()
    .find(|(word, _)| word.eq_ignore_ascii_case(value))
    .map(|(_, state)| state)
}

fn read_content(value: &str) -> Option<ContentValue> {
    if value.eq_ignore_ascii_case("split") {
        Some(ContentValue::Split)
    } else if value.eq_ignore_ascii_case("image") {
        Some(ContentValue::Image)
    } else {
        None
    }
}

/// Reads a number of ASCII digits only. An empty text gives `None`.
fn ascii_number<T: std::str::FromStr>(text: &str) -> Option<T> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// Reads `YYYY-MM-DD` or `YYYY-MM` into a date value.
fn read_date(value: &str) -> Option<DateValue> {
    if !value.is_ascii() {
        return None;
    }
    let bytes = value.as_bytes();
    if bytes.get(4) != Some(&b'-') {
        return None;
    }
    let year: i32 = ascii_number(&value[..4])?;
    match bytes.len() {
        10 if bytes[7] == b'-' => {
            let day = NaiveDate::from_ymd_opt(
                year,
                ascii_number(&value[5..7])?,
                ascii_number(&value[8..10])?,
            )?;
            Some(DateValue {
                first_day: day,
                last_day: day,
            })
        }
        7 => {
            let first_day = NaiveDate::from_ymd_opt(year, ascii_number(&value[5..7])?, 1)?;
            let last_day = first_day.checked_add_months(Months::new(1))?.pred_opt()?;
            Some(DateValue {
                first_day,
                last_day,
            })
        }
        _ => None,
    }
}

/// Reads a duration comparison or a duration range (packet "The duration
/// grammar").
fn read_duration(value: &str) -> Option<DurationRange> {
    if let Some((start, end)) = value.split_once("..") {
        let min = read_amount(start)?;
        let max = read_amount(end)?;
        return (min <= max).then_some(DurationRange {
            min_secs: Some(min),
            max_secs: Some(max),
        });
    }
    if let Some(amount) = value.strip_prefix(">=") {
        return Some(DurationRange {
            min_secs: Some(read_amount(amount)?),
            max_secs: None,
        });
    }
    if let Some(amount) = value.strip_prefix('>') {
        return Some(DurationRange {
            min_secs: Some(read_amount(amount)?.checked_add(1)?),
            max_secs: None,
        });
    }
    if let Some(amount) = value.strip_prefix("<=") {
        return Some(DurationRange {
            min_secs: None,
            max_secs: Some(read_amount(amount)?),
        });
    }
    if let Some(amount) = value.strip_prefix('<') {
        return Some(DurationRange {
            min_secs: None,
            max_secs: Some(read_amount(amount)?.checked_sub(1)?),
        });
    }
    None
}

/// Reads one amount, a clock or units, in seconds.
fn read_amount(text: &str) -> Option<u32> {
    let secs = if text.contains(':') {
        read_clock(text)?
    } else {
        read_units(text)?
    };
    u32::try_from(secs).ok()
}

/// Reads `m:ss` or `h:mm:ss`.
fn read_clock(text: &str) -> Option<u64> {
    let parts: Vec<&str> = text.split(':').collect();
    match parts.as_slice() {
        [minutes, seconds] => ascii_number::<u64>(minutes)?
            .checked_mul(60)?
            .checked_add(read_sexagesimal(seconds)?),
        [hours, minutes, seconds] => ascii_number::<u64>(hours)?
            .checked_mul(3600)?
            .checked_add(read_sexagesimal(minutes)? * 60)?
            .checked_add(read_sexagesimal(seconds)?),
        _ => None,
    }
}

/// Reads two digits from `00` to `59`.
fn read_sexagesimal(text: &str) -> Option<u64> {
    if text.len() != 2 {
        return None;
    }
    ascii_number::<u64>(text).filter(|value| *value < 60)
}

/// Reads `[digits h][digits m][digits s]`, with one or more parts, in this
/// order, each one time at most. The units have no case.
fn read_units(text: &str) -> Option<u64> {
    if text.is_empty() {
        return None;
    }
    let mut total: u64 = 0;
    let mut last_rank = 0;
    let mut rest = text;
    while !rest.is_empty() {
        let digits_len = rest
            .find(|ch: char| !ch.is_ascii_digit())
            .unwrap_or(rest.len());
        let amount: u64 = ascii_number(&rest[..digits_len])?;
        let unit = rest[digits_len..].chars().next()?;
        let (rank, factor) = match unit.to_ascii_lowercase() {
            'h' => (1, 3600),
            'm' => (2, 60),
            's' => (3, 1),
            _ => return None,
        };
        if rank <= last_rank {
            return None;
        }
        last_rank = rank;
        total = total.checked_add(amount.checked_mul(factor)?)?;
        rest = &rest[digits_len + unit.len_utf8()..];
    }
    Some(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_models::release_kind::ReleaseKind;

    const EXAMPLE: &str =
        r#"artist:jsmovie kind:single after:2026-10-01 -title:remix "night drive""#;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("test date is correct")
    }

    /// The one term of `text`.
    fn only_term(text: &str) -> QueryTerm {
        let query = SearchQuery::parse(text);
        assert_eq!(query.terms().len(), 1, "one term in {text:?}");
        query.terms()[0].clone()
    }

    /// The field of the one term of `text`.
    fn only_field(text: &str) -> FieldTerm {
        match only_term(text).kind {
            TermKind::Field(field) => field,
            other => panic!("{text:?} gives {other:?}, not a field"),
        }
    }

    fn valid(text: &str) -> FieldValue {
        match only_field(text).value {
            FieldValueState::Valid(value) => value,
            other => panic!("{text:?} gives {other:?}, not a correct value"),
        }
    }

    fn is_invalid(text: &str) -> bool {
        matches!(only_field(text).value, FieldValueState::Invalid(_))
    }

    fn duration(text: &str) -> DurationRange {
        match valid(text) {
            FieldValue::Duration(range) => range,
            other => panic!("{text:?} gives {other:?}"),
        }
    }

    fn date_value(text: &str) -> DateValue {
        match valid(text) {
            FieldValue::Date(value) => value,
            other => panic!("{text:?} gives {other:?}"),
        }
    }

    fn text_field(name: QueryFieldName, text: &str) -> TermKind {
        TermKind::Field(FieldTerm {
            name,
            value: FieldValueState::Valid(FieldValue::Text(text.to_owned())),
        })
    }

    #[test]
    fn adr_0087_query_parser_r87_1_01_example_gives_five_typed_terms() {
        let query = SearchQuery::parse(EXAMPLE);
        let kinds: Vec<&TermKind> = query.terms().iter().map(|term| &term.kind).collect();
        let day = date(2026, 10, 1);
        assert_eq!(
            kinds,
            vec![
                &text_field(QueryFieldName::Artist, "jsmovie"),
                &TermKind::Field(FieldTerm {
                    name: QueryFieldName::Kind,
                    value: FieldValueState::Valid(FieldValue::Kinds(vec![KindValue::Single])),
                }),
                &TermKind::Field(FieldTerm {
                    name: QueryFieldName::After,
                    value: FieldValueState::Valid(FieldValue::Date(DateValue {
                        first_day: day,
                        last_day: day,
                    })),
                }),
                &text_field(QueryFieldName::Title, "remix"),
                &TermKind::Phrase("night drive".to_owned()),
            ]
        );
        let excluded: Vec<bool> = query.terms().iter().map(|term| term.excluded).collect();
        assert_eq!(excluded, vec![false, false, false, true, false]);
    }

    #[test]
    fn adr_0087_query_parser_r87_1_02_each_span_gives_its_source_text() {
        let query = SearchQuery::parse(EXAMPLE);
        let texts: Vec<&str> = query
            .terms()
            .iter()
            .map(|term| &EXAMPLE[term.span.clone()])
            .collect();
        assert_eq!(
            texts,
            vec![
                "artist:jsmovie",
                "kind:single",
                "after:2026-10-01",
                "-title:remix",
                "\"night drive\"",
            ]
        );
    }

    #[test]
    fn adr_0087_query_parser_r87_1_03_unknown_colon_name_stays_one_word() {
        assert_eq!(
            only_term("Re:Union").kind,
            TermKind::Word("Re:Union".to_owned())
        );
        assert_eq!(
            only_term("foo:bar").kind,
            TermKind::Word("foo:bar".to_owned())
        );
    }

    #[test]
    fn adr_0087_query_parser_r87_1_04_date_that_is_not_correct_is_invalid() {
        match only_field("after:yesterday").value {
            FieldValueState::Invalid(error) => assert_eq!(
                error.message(),
                "after: needs a date, for example 2026-10-01"
            ),
            other => panic!("after:yesterday gives {other:?}"),
        }
        assert!(is_invalid("after:2026-02-30"));
        assert!(is_invalid("after:2026-1-5"));
    }

    #[test]
    fn adr_0087_query_parser_r87_1_05_field_name_has_no_case() {
        for text in ["Artist:x", "ARTIST:x", "artist:x"] {
            assert_eq!(only_field(text).name, QueryFieldName::Artist, "{text}");
        }
        assert_eq!(
            valid("KIND:Single"),
            FieldValue::Kinds(vec![KindValue::Single])
        );
    }

    #[test]
    fn adr_0087_query_parser_r87_1_06_text_value_keeps_case_and_reads_quotes() {
        assert_eq!(valid("title:Remix"), FieldValue::Text("Remix".to_owned()));
        assert_eq!(
            valid(r#"release:"aged friends""#),
            FieldValue::Text("aged friends".to_owned())
        );
    }

    #[test]
    fn adr_0087_query_parser_r87_1_07_kind_reads_a_comma_list() {
        assert_eq!(
            valid("kind:album,ep"),
            FieldValue::Kinds(vec![KindValue::Album, KindValue::Ep])
        );
        assert_eq!(
            valid("kind:single,single,"),
            FieldValue::Kinds(vec![KindValue::Single])
        );
        match only_field("kind:lp").value {
            FieldValueState::Invalid(error) => assert!(error.message().contains("lp")),
            other => panic!("kind:lp gives {other:?}"),
        }
    }

    #[test]
    fn adr_0087_query_parser_r87_1_08_in_reads_one_scope() {
        assert_eq!(valid("in:library"), FieldValue::Scope(ScopeValue::Library));
        assert_eq!(valid("in:index"), FieldValue::Scope(ScopeValue::Index));
        assert!(is_invalid("in:both"));
        assert!(is_invalid("in:library,index"));
        assert_eq!(
            only_field("-in:index").value,
            FieldValueState::Invalid(FieldValueError::ExcludedScope)
        );
    }

    #[test]
    fn adr_0087_query_parser_r87_1_09_month_gives_its_first_and_last_day() {
        let october = date_value("after:2026-10");
        assert_eq!(october.first_day, date(2026, 10, 1));
        assert_eq!(october.last_day, date(2026, 10, 31));
        assert_eq!(date_value("before:2026-02").last_day, date(2026, 2, 28));
        assert_eq!(date_value("after:2026-10-01").start_unix(), 1_790_812_800);
        assert_eq!(date_value("before:2026-10-31").end_unix(), 1_793_491_199);
    }

    #[test]
    fn adr_0087_query_parser_r87_1_10_duration_gives_inclusive_limits() {
        let range = |min, max| DurationRange {
            min_secs: min,
            max_secs: max,
        };
        assert_eq!(duration("duration:>5m"), range(Some(301), None));
        assert_eq!(duration("duration:>=5m"), range(Some(300), None));
        assert_eq!(duration("duration:<3:30"), range(None, Some(209)));
        assert_eq!(duration("duration:<=3:30"), range(None, Some(210)));
        assert_eq!(duration("duration:3m..5m"), range(Some(180), Some(300)));
        assert_eq!(duration("duration:>=1h30m").min_secs, Some(5400));
        assert_eq!(duration("duration:>=90s").min_secs, Some(90));
        assert_eq!(duration("duration:>=1:02:03").min_secs, Some(3723));
        let middle = range(Some(180), Some(300));
        assert!(middle.contains(180) && middle.contains(300));
        assert!(!middle.contains(179) && !middle.contains(301));
    }

    #[test]
    fn adr_0087_query_parser_r87_1_11_duration_that_is_not_correct_is_invalid() {
        for text in [
            "duration:5m",
            "duration:5",
            "duration:2.5m",
            "duration:3m..",
            "duration:5m..3m",
            "duration:30s3m",
            "duration:3:5",
            "duration:<0s",
            "duration:>99999999999s",
        ] {
            assert!(is_invalid(text), "{text}");
        }
    }

    #[test]
    fn adr_0087_query_parser_r87_1_12_is_and_has_read_their_fixed_values() {
        assert_eq!(
            valid("is:downloaded"),
            FieldValue::LibraryState(LibraryStateValue::Downloaded)
        );
        assert_eq!(
            valid("is:partial"),
            FieldValue::LibraryState(LibraryStateValue::Partial)
        );
        assert_eq!(
            valid("is:ready"),
            FieldValue::LibraryState(LibraryStateValue::Ready)
        );
        assert_eq!(
            valid("is:removed"),
            FieldValue::LibraryState(LibraryStateValue::Removed)
        );
        assert_eq!(valid("has:split"), FieldValue::Content(ContentValue::Split));
        assert_eq!(valid("has:image"), FieldValue::Content(ContentValue::Image));
        assert!(is_invalid("is:new"));
        assert!(is_invalid("has:art"));
    }

    #[test]
    fn adr_0087_query_parser_r87_1_13_empty_value_is_incomplete() {
        assert_eq!(only_field("artist:").value, FieldValueState::Incomplete);
        assert_eq!(
            only_field(r#"artist:"""#).value,
            FieldValueState::Incomplete
        );
        let query = SearchQuery::parse("artist: jsmovie");
        assert_eq!(
            query.terms()[0].kind,
            TermKind::Field(FieldTerm {
                name: QueryFieldName::Artist,
                value: FieldValueState::Incomplete,
            })
        );
        assert_eq!(query.terms()[1].kind, TermKind::Word("jsmovie".to_owned()));
        assert_eq!(query.terms().len(), 2);
    }

    #[test]
    fn adr_0087_query_parser_r87_1_14_open_quote_closes_at_the_text_end() {
        assert_eq!(
            only_term(r#""night drive"#).kind,
            TermKind::Phrase("night drive".to_owned())
        );
        assert_eq!(
            valid(r#"release:"aged fr"#),
            FieldValue::Text("aged fr".to_owned())
        );
    }

    #[test]
    fn adr_0087_query_parser_r87_1_15_backslash_escapes_inside_quotes_only() {
        assert_eq!(
            only_term(r#""the \"best\" song""#).kind,
            TermKind::Phrase(r#"the "best" song"#.to_owned())
        );
        assert_eq!(
            only_term(r#""a\\b""#).kind,
            TermKind::Phrase(r"a\b".to_owned())
        );
        assert_eq!(only_term(r"a\b").kind, TermKind::Word(r"a\b".to_owned()));
    }

    #[test]
    fn adr_0087_query_parser_r87_1_16_minus_applies_at_the_term_start_only() {
        assert_eq!(only_term("lo-fi").kind, TermKind::Word("lo-fi".to_owned()));
        assert!(SearchQuery::parse("-").terms().is_empty());
        let remix = only_term("-remix");
        assert_eq!(remix.kind, TermKind::Word("remix".to_owned()));
        assert!(remix.excluded);
    }

    #[test]
    fn adr_0087_query_parser_r87_1_17_scope_reads_the_correct_fields() {
        let scope = |text: &str| SearchQuery::parse(text).scope();
        assert_eq!(scope("haleen"), QueryScope::Everything);
        assert_eq!(scope("is:ready"), QueryScope::LibraryOnly);
        assert_eq!(scope("-has:split"), QueryScope::LibraryOnly);
        assert_eq!(scope("in:library"), QueryScope::LibraryOnly);
        assert_eq!(scope("in:index"), QueryScope::IndexOnly);
        assert_eq!(scope("in:index is:ready"), QueryScope::Conflict);
        assert_eq!(scope("in:library in:index"), QueryScope::Conflict);
        assert_eq!(scope("is:bogus in:index"), QueryScope::IndexOnly);
    }

    #[test]
    fn adr_0087_query_parser_r87_1_18_index_q_holds_words_and_phrases_only() {
        let index_q = |text: &str| SearchQuery::parse(text).index_q();
        assert_eq!(index_q(EXAMPLE), Some("\"night drive\"".to_owned()));
        assert_eq!(
            index_q(r#"haleen "a \"b\"""#),
            Some(r#"haleen "a \"b\"""#.to_owned())
        );
        assert_eq!(index_q("artist:x -remix"), None);
        for text in [
            EXAMPLE,
            "haleen kind:album title:x",
            "is:ready in:index night",
        ] {
            let q = index_q(text).unwrap_or_default();
            for info in &QUERY_FIELDS {
                assert!(!q.contains(info.token), "{q:?} holds {}", info.token);
            }
        }
    }

    #[test]
    fn adr_0087_query_parser_r87_1_19_index_filter_terms_give_fields_and_exclusions() {
        let query = SearchQuery::parse(EXAMPLE);
        let filters: Vec<&QueryTerm> = query.index_filter_terms();
        assert_eq!(filters.len(), 4);
        assert!(filters
            .iter()
            .all(|term| matches!(term.kind, TermKind::Field(_))));
        let query = SearchQuery::parse("in:index -remix");
        let filters = query.index_filter_terms();
        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0].kind, TermKind::Word("remix".to_owned()));
        assert!(filters[0].excluded);
    }

    #[test]
    fn adr_0087_query_parser_r87_1_20_runnable_content_needs_a_word_or_a_field() {
        let runnable = |text: &str| SearchQuery::parse(text).has_runnable_content();
        for text in ["haleen", "\"night drive\"", "kind:single"] {
            assert!(runnable(text), "{text}");
        }
        for text in [
            "",
            "!!!",
            "-remix",
            "in:library",
            "artist:",
            "after:yesterday",
            "-kind:single",
        ] {
            assert!(!runnable(text), "{text}");
        }
    }

    #[test]
    fn adr_0087_query_parser_r87_1_21_cursor_in_a_field_name_gives_the_prefix() {
        assert_eq!(
            cursor_term("ar", 2),
            CursorTerm {
                range: 0..2,
                excluded: false,
                partial: CursorPartial::FieldName {
                    prefix: "ar".to_owned(),
                },
            }
        );
        let tokens: Vec<&str> = fields_with_prefix("ar")
            .iter()
            .map(|info| info.token)
            .collect();
        assert_eq!(tokens, vec!["artist:"]);
        assert_eq!(fields_with_prefix("").len(), 11);
    }

    #[test]
    fn adr_0087_query_parser_r87_1_22_cursor_after_a_colon_gives_the_value() {
        assert_eq!(
            cursor_term("kind:s", 6).partial,
            CursorPartial::FieldValue {
                field: QueryFieldName::Kind,
                prefix: "s".to_owned(),
                value_range: 5..6,
            }
        );
        let words: Vec<&str> = QueryFieldName::Kind
            .values_with_prefix("s")
            .iter()
            .map(|value| value.word)
            .collect();
        assert_eq!(words, vec!["single"]);
    }

    #[test]
    fn adr_0087_query_parser_r87_1_23_kind_item_and_excluded_term() {
        assert_eq!(
            cursor_term("kind:album,e", 12).partial,
            CursorPartial::FieldValue {
                field: QueryFieldName::Kind,
                prefix: "e".to_owned(),
                value_range: 11..12,
            }
        );
        let term = cursor_term("haleen -ar", 10);
        assert!(term.excluded);
        assert_eq!(term.range, 8..10);
    }

    #[test]
    fn adr_0087_query_parser_r87_1_24_empty_term_and_other_terms() {
        assert_eq!(
            cursor_term("haleen ", 7),
            CursorTerm {
                range: 7..7,
                excluded: false,
                partial: CursorPartial::FieldName {
                    prefix: String::new(),
                },
            }
        );
        assert_eq!(cursor_term("\"night dr", 9).partial, CursorPartial::Other);
        assert_eq!(cursor_term("Re:Un", 5).partial, CursorPartial::Other);
    }

    #[test]
    fn adr_0087_query_parser_r87_1_25_cursor_moves_to_a_character_boundary() {
        let text = "artist:é";
        assert_eq!(text.len(), 9);
        for cursor in [8, 9, 100] {
            let term = cursor_term(text, cursor);
            assert!(text.is_char_boundary(term.range.start));
            assert!(text.is_char_boundary(term.range.end));
            if let CursorPartial::FieldValue {
                prefix,
                value_range,
                ..
            } = &term.partial
            {
                assert!(text.is_char_boundary(value_range.start));
                assert!(text.is_char_boundary(value_range.end));
                assert!(prefix.is_empty() || prefix == "é");
            }
        }
        assert_eq!(
            cursor_term(text, 9).partial,
            CursorPartial::FieldValue {
                field: QueryFieldName::Artist,
                prefix: "é".to_owned(),
                value_range: 7..9,
            }
        );
        assert_eq!(
            cursor_term(text, 8).partial,
            CursorPartial::FieldValue {
                field: QueryFieldName::Artist,
                prefix: String::new(),
                value_range: 7..9,
            }
        );
    }

    #[test]
    fn adr_0087_query_parser_r87_1_26_field_table_order_tokens_and_scope() {
        let names: Vec<QueryFieldName> = QUERY_FIELDS.iter().map(|info| info.name).collect();
        assert_eq!(
            names,
            vec![
                QueryFieldName::Artist,
                QueryFieldName::Title,
                QueryFieldName::Release,
                QueryFieldName::Kind,
                QueryFieldName::In,
                QueryFieldName::After,
                QueryFieldName::Before,
                QueryFieldName::Duration,
                QueryFieldName::Is,
                QueryFieldName::Playlist,
                QueryFieldName::Has,
            ]
        );
        for info in &QUERY_FIELDS {
            assert_eq!(
                info.token,
                format!("{:?}:", info.name).to_lowercase(),
                "{info:?}"
            );
            let bare = info.token.trim_end_matches(':');
            assert_eq!(QueryFieldName::from_name(bare), Some(info.name));
            assert_eq!(info.name.info(), info);
        }
        let library_only: Vec<QueryFieldName> = QUERY_FIELDS
            .iter()
            .filter(|info| info.library_only)
            .map(|info| info.name)
            .collect();
        assert_eq!(
            library_only,
            vec![
                QueryFieldName::Is,
                QueryFieldName::Playlist,
                QueryFieldName::Has
            ]
        );
    }

    #[test]
    fn adr_0087_query_parser_r87_1_27_fixed_values_parse_as_correct_values() {
        for info in &QUERY_FIELDS {
            let expected = match info.name {
                QueryFieldName::Kind => 3,
                QueryFieldName::In | QueryFieldName::Has => 2,
                QueryFieldName::Is => 4,
                _ => 0,
            };
            let values = info.name.fixed_values();
            assert_eq!(values.len(), expected, "{}", info.token);
            for value in values {
                let text = format!("{}{}", info.token, value.word);
                assert!(
                    matches!(only_field(&text).value, FieldValueState::Valid(_)),
                    "{text}"
                );
            }
        }
    }

    #[test]
    fn adr_0087_query_parser_r87_1_28_kind_words_resolve_to_release_kinds() {
        for kind in [KindValue::Album, KindValue::Ep, KindValue::Single] {
            let word = kind.word();
            let resolved = ReleaseKind::resolve(Some(word), 2);
            assert!(resolved.label().eq_ignore_ascii_case(word), "{word}");
        }
    }

    #[test]
    fn adr_0087_query_parser_r87_1_29_field_errors_in_text_order() {
        let query = SearchQuery::parse("after:yesterday kind:lp haleen");
        let errors = query.field_errors();
        assert_eq!(errors.len(), 2);
        let tokens = ["after:", "kind:"];
        for ((term, error), token) in errors.iter().zip(tokens) {
            let message = error.message();
            assert!(!message.is_empty());
            assert!(message.starts_with(token), "{message}");
            assert!(
                matches!(&term.kind, TermKind::Field(field) if field.name.info().token == token)
            );
        }
    }
}
