# ADR 0087 Task 001: One Parser Reads The Search Query Text

Status: Ready - 2026-10-23.

## Goal

Add one parser in the view model layer. The parser reads the text of the toolbar search field and
gives a typed query. The typed query holds the words, the phrases, the fields, the exclusions and
one typed error for each field value that is not correct.

The module also holds the field table as typed data and the scope rules. It gives the words for
the Index `q` and the term under the text cursor. Packets 002 to 005 read these facts. They do not state them
again.

This packet changes no interface and no behavior. It adds one pure module, its unit tests and one
situational guard. No production caller reads the parser until packet 002.

## Authority

- [ADR 0087](../adr/0087-search-fields-and-live-results.md) Decision 1: one query text with
  words, phrases and fields. One parser in the view model layer reads the text. A field name has
  no case. A colon with an unknown name stays one word. A field value that is not correct applies
  nothing.
- [ADR 0087](../adr/0087-search-fields-and-live-results.md) Decision 2: the field set, the value
  kinds, the inclusive dates, the month rule, the Library-only fields and the scope conflict.
- [ADR 0087](../adr/0087-search-fields-and-live-results.md) Decision 3: the field list reads the
  fields, the fixed values and the error text of each field.
- [ADR 0087](../adr/0087-search-fields-and-live-results.md) Decision 4: a text with no word,
  phrase or correct field runs nothing.
- [ADR 0087](../adr/0087-search-fields-and-live-results.md) Decision 5: `q` holds the words and
  the phrases only.
- [ADR 0087](../adr/0087-search-fields-and-live-results.md) invariant "One parser reads the query
  text. No other code reads field syntax."
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 5: the release kinds Album, EP and
  Single.
- The [ADR 0087 phase plan](../plans/adr-0087-search-fields-phase-plan.md), packet 001.
- [AGENTS.md](../../AGENTS.md): renderer portability, typed action state, "A guard names its
  class and its ADR" and "An acceptance criterion states how it is checked".

## Recorded Facts - 2026-10-23

The orchestrator read each fact below in the working tree on 2026-10-23.

### The search path today

- `src/app.rs:604-614` `on_global_search_event` calls `submit_global_search` only on
  `InputEvent::PressEnter`.
- `src/app/search_dispatch.rs:223-226` `submit_global_search` reads the input value and calls
  `open_search_results_in_content_list`. That function trims the text and returns when the text
  is empty (`src/app/search_dispatch.rs:234-237`). It reads no field syntax.
- `src/application/queries/search.rs:209-216` `normalized_global_search_query` trims the text. It
  gives `None` when the text has no alphanumeric character (`char::is_alphanumeric`). The only
  caller is `search_local_library_tracks` (`src/application/queries/search.rs:192-206`).
- `src/db.rs:561-601` `search_library_tracks` binds one `LIKE` pattern with `ESCAPE '\\'` against
  five columns. `t.duration_seconds` and `t.pub_date` are columns of the same row.
- `src/application/queries/search.rs:226-275` `fetch_index_search_result_rows` sends the full
  text to three `GET /v1/search` requests.
- `../stophammer/src/search.rs:224-242` `sanitize_fts5_query` changes `"`, `(`, `)`, `*`, `-`,
  `^`, `{` and `}` to a space. It keeps a colon and an apostrophe.
- No file under `src/` holds a type named `SearchQuery`.
- No file under `src/` holds a string literal that starts with `artist:`, `title:`, `release:`, `playlist:`, `kind:`, `in:`, `after:`, `before:`,
  `duration:`, `is:` or `has:`.

### The view model layer

- `src/view_models/text_filter.rs:8-19` holds `normalize` and `contains_normalized`. These
  compare text without case. They do not read terms.
- `src/view_models/mod.rs:62-109` declares the modules. A module name is a snake_case noun, for
  example `text_filter` and `release_kind`. `src/lib.rs:41` declares `pub mod view_models`.
- `Cargo.toml:4` sets `edition = "2021"`. The file has no `[lints]` table.

### The release kind

- `src/view_models/release_kind.rs:11-16` declares `pub(crate) enum ReleaseKind` with `Album`,
  `Ep` and `Single`. `src/view_models/mod.rs:94` declares the module `pub(crate)`.
- `src/view_models/release_kind.rs:43-53` `from_stated_value` reads `album`, `ep` and `single`
  without case. It is private.
- `src/view_models/release_kind.rs:67-73` `noun` gives `album`, `EP` and `single`. The `EP` noun
  is not the query word `ep`.
- `tests/architecture_tests.rs:21880-21911` guard R15-09 rejects the text `ReleaseKind::Single`
  in production code outside `release_kind.rs` and `singles_group.rs`.

### The dates and the durations

- `src/view_models/format.rs:28-30` `fmt_date` shows a Unix time as a UTC date. The view model
  layer has no date parser.
- `Cargo.toml:14` holds `chrono` 0.4 with the features `std` and `serde`. `chrono::NaiveDate`
  parses a date with these features (`src/metadata.rs:2589`).
- ADR 0086 Decision 1: the release date of a Library single is the latest `pub_date` of its
  tracks. The release date of a MusicIndex single is its `newest_item_at`. Both are Unix seconds.
- `docs/plans/v4vmm-open-requests.md` request 10 asks for `min_duration` and `max_duration` in
  seconds.

### The cursor

- The toolbar field is a `gpui_component` `InputState` (`src/app.rs:147`). `gpui-component` 0.6.1
  re-exports it from `gpui_base`, where `InputState` is `InputBaseState<InputMode>`. The public
  `InputBaseState::cursor` gives the UTF-8 byte offset of the cursor
  (`~/.cargo/git/checkouts/gpui-kit-7335f17435243f46/5463fe4/crates/base/src/input/base/state.rs:2736`).
  `set_selected_range` (`:2816`) moves the cursor.

## Design

### The module

The parser is `src/view_models/search_query.rs`, declared `pub mod search_query;` in
`src/view_models/mod.rs`. The module has no GPUI type and no I/O. Each item that later packets
read is `pub`, so the library crate reaches it and `dead_code` does not fire (D14).

### The grammar

The parser reads the text from left to right. A whitespace character (`char::is_whitespace`)
outside quotes ends a term.

```text
query        = { space } , [ term , { space , { space } , term } ] , { space }
term         = [ "-" ] , ( field | phrase | word )
field        = known-name , ":" , [ value ]
known-name   = one name of QUERY_FIELDS, compared without case
value        = quoted | bare
phrase       = quoted
quoted       = '"' , { char other than '"' or "\" | "\" , any char } , [ '"' ]
word         = one or more chars other than whitespace
```

- A word that starts with `name:` for a name that is not in the table stays one word. `Re:Union`
  is the word `Re:Union`.
- A `-` applies only at the start of a term. A `-` inside a word is part of the word, for example
  `lo-fi`. A `-` with nothing after it gives no term.
- Inside quotes, `\"` gives a quote and `\\` gives a backslash. Each other backslash stays in the
  text. Outside quotes, a backslash is an ordinary character.
- A quote with no closing quote closes at the end of the text.
- A quote inside a word that does not start with a quote is an ordinary character.
- `artist: jsmovie` gives an incomplete `artist:` field and the word `jsmovie`.
- An empty phrase `""` gives no term.

### The value kinds

| Field | Value kind | Correct values | Typed value |
|---|---|---|---|
| `artist:`, `title:`, `release:`, `playlist:` | Text | Each text that is not empty, bare or in quotes. The text keeps its case | `FieldValue::Text(String)` |
| `kind:` | Kind list | `album`, `ep`, `single`, without case, with commas. Empty items are ignored. A repeated item counts once | `FieldValue::Kinds(Vec<KindValue>)` |
| `in:` | Scope | `library` or `index`, without case. One value | `FieldValue::Scope(ScopeValue)` |
| `after:`, `before:` | Date | `YYYY-MM-DD` or `YYYY-MM`, with four year digits and two month and day digits | `FieldValue::Date(DateValue)` |
| `duration:` | Duration | The grammar of "The duration grammar" | `FieldValue::Duration(DurationRange)` |
| `is:` | Library state | `downloaded`, `partial`, `ready`, `removed`, without case. One value | `FieldValue::LibraryState(LibraryStateValue)` |
| `has:` | Content | `split`, `image`, without case. One value | `FieldValue::Content(ContentValue)` |

A `DateValue` holds `first_day` and `last_day`, as `chrono::NaiveDate`. A day gives the same day
twice. A month gives its first day and its last day.

`after:` applies from the start of
`first_day`. `before:` applies to the end of `last_day`. Thus each date includes its day, and a
month includes each day of the month. `DateValue::start_unix` and `DateValue::end_unix` give the
first second and the last second in UTC, the same time zone as `fmt_date`.

### The duration grammar

```text
duration-value = compare , amount | amount , ".." , amount
compare        = ">" | ">=" | "<" | "<="
amount         = clock | units
clock          = minutes , ":" , two-digits | hours , ":" , two-digits , ":" , two-digits
units          = [ digits , "h" ] , [ digits , "m" ] , [ digits , "s" ]   (one or more parts)
```

- The units `h`, `m` and `s` have no case. They come in this order, each one time at most.
  `1h30m`, `90s` and `3m30s` are correct. `30s3m` and `5` are not correct.
- In a clock, the seconds and the minutes after the hours have two digits from `00` to `59`.
  `3:30` is 210 seconds and `1:02:03` is 3723 seconds.
- `>` and `<` exclude the amount. `>=` and `<=` include it. A range includes both ends.
- The typed value is a `DurationRange` with an inclusive `min_secs` and an inclusive `max_secs`,
  each `Option<u32>`. `>5m` gives `min_secs` 301. `<3:30` gives `max_secs` 209. `3m..5m` gives
  180 and 300.
- These values are not correct: a bare amount, a decimal and an open range such as `3m..`.
- A range whose start is after its end, `<0s` and an amount larger than `u32::MAX` seconds are
  also not correct.

### The terms and the errors

`SearchQuery::parse` gives a `SearchQuery` with a `Vec<QueryTerm>` in text order. Each
`QueryTerm` holds its byte range in the text, its exclusion flag and a `TermKind`: a word, a
phrase or a field. A field holds its `QueryFieldName` and a `FieldValueState`:

- `Valid(FieldValue)`: the value is correct.
- `Incomplete`: the value is empty, for example `artist:` or `artist:""`. This is not an error,
  because the curator is possibly still typing. The field applies nothing.
- `Invalid(FieldValueError)`: the value is not correct. The field applies nothing.

`FieldValueError::message` gives the text that the field list shows. Each message names the field
and gives one correct example, for example "after: needs a date, for example 2026-10-01".

### The scope

`SearchQuery::scope` reads the `Valid` fields only:

- A Library-only field (`is:`, `playlist:` or `has:`), with or without `-`, gives `LibraryOnly`.
  The Index cannot apply the field and cannot apply its exclusion.
- `in:library` gives `LibraryOnly`. `in:index` gives `IndexOnly`.
- `in:index` with a Library-only field gives `Conflict`. `in:library` with `in:index` gives
  `Conflict`. A `Conflict` query shows no rows.
- With none of these, the scope is `Everything`.

### The Index words and filters

- `SearchQuery::index_q` gives the words and the phrases without `-`, in text order, with one
  space between them. A phrase keeps its double quotes, and a quote inside it is written `\"`, as
  request 10 asks. With no such word or phrase, it gives `None`. It never holds a field.
- `SearchQuery::index_filter_terms` gives each term that the Index must apply as a filter: each
  `Valid` field other than `in:`, and each word or phrase with `-`. Packet 002 shows no Index rows
  and the note of Decision 5 when the stored contract cannot apply one of them.

### The runnable content

`SearchQuery::has_runnable_content` is `true` when the query holds one term without `-` of these
kinds:

- a word or a phrase with one or more alphanumeric characters, the same test as
  `normalized_global_search_query` today
- a `Valid` field other than `in:`

### The cursor term

`cursor_term(text, cursor)` reads the term under the cursor. `cursor` is a byte offset into
`text`. The function moves an offset that is not on a character boundary back to the boundary,
and an offset after the end to the end. The term under the cursor is the term whose range holds
the cursor or ends at the cursor. With no such term, the term is empty at the cursor.

The `CursorTerm` gives:

- `range`: the byte range of the term after its `-`. The field list replaces this range when the
  curator selects a field.
- `excluded`: `true` when the term has a `-`.
- `partial`, one of these:
  - `FieldName { prefix }`: the term has no colon of a known field. `prefix` is the text from the
    term start to the cursor. `ar` gives the prefix `ar`. An empty term gives an empty prefix.
  - `FieldValue { field, prefix, value_range }`: the term is a known name and a colon, and the
    cursor is after the colon. `prefix` is the value text from the start of the value item to the
    cursor. For `kind:`, the value item starts after the last comma before the cursor.
    `value_range` is the byte range of that item. `kind:s` gives `Kind`, `s` and the range of `s`.
  - `Other`: the cursor is in a phrase, or in a word whose colon name is not a field.

### The field table

`QUERY_FIELDS` is a `pub const` array of 11 `QueryFieldInfo` rows, in the order of ADR 0087
Decision 2. Packet 004 reads it and does not state it again.

| Name | Token | Value kind | Values text | Example | Description | Library only |
|---|---|---|---|---|---|---|
| `Artist` | `artist:` | `Text` | an artist name | `artist:jsmovie` | The track artist or the album artist | no |
| `Title` | `title:` | `Text` | a title | `title:"night drive"` | The track title or the release title | no |
| `Release` | `release:` | `Text` | a release title | `release:"aged friends"` | The release title | no |
| `Kind` | `kind:` | `KindList` | album, ep, single | `kind:single` | The release kind. Use commas for more than one kind | no |
| `In` | `in:` | `Scope` | library, index | `in:library` | Search only the Library or only the Index | no |
| `After` | `after:` | `Date` | a date or a month | `after:2026-10-01` | Released on this date or after it | no |
| `Before` | `before:` | `Date` | a date or a month | `before:2026-10` | Released on this date or before it | no |
| `Duration` | `duration:` | `Duration` | >5m, <3:30, 3m..5m | `duration:3m..5m` | The track duration | no |
| `Is` | `is:` | `LibraryState` | downloaded, partial, ready, removed | `is:downloaded` | The Library state of the track | yes |
| `Playlist` | `playlist:` | `Text` | a playlist name | `playlist:"friday show"` | The tracks of a playlist | yes |
| `Has` | `has:` | `Content` | split, image | `has:split` | A value split or an image of its own | yes |

`QueryFieldName::fixed_values` gives the `FixedValueInfo` rows of `kind:`, `in:`, `is:` and
`has:`, each with its word and a short description. Each other field gives an empty slice.

| Field | Word | Description |
|---|---|---|
| `kind:` | `album` | A release with many tracks |
| `kind:` | `ep` | A short release |
| `kind:` | `single` | A release with one track |
| `in:` | `library` | Only the Library |
| `in:` | `index` | Only MusicIndex |
| `is:` | `downloaded` | Each track is downloaded |
| `is:` | `partial` | Some tracks are downloaded |
| `is:` | `ready` | Ready for a show |
| `is:` | `removed` | Removed from its feed |
| `has:` | `split` | A value block with payment routes |
| `has:` | `image` | An image of its own |

### Names For Later Packets

Module: `crate::view_models::search_query` (`src/view_models/search_query.rs`).

| Name | Kind | Packets |
|---|---|---|
| `SearchQuery` | struct | 002, 003 |
| `SearchQuery::parse(text: &str) -> SearchQuery` | function | 002, 003, 004 |
| `SearchQuery::terms(&self) -> &[QueryTerm]` | function | 002, 004 |
| `SearchQuery::scope(&self) -> QueryScope` | function | 002, 004 |
| `SearchQuery::index_q(&self) -> Option<String>` | function | 002 |
| `SearchQuery::index_filter_terms(&self) -> Vec<&QueryTerm>` | function | 002 |
| `SearchQuery::has_runnable_content(&self) -> bool` | function | 003 |
| `SearchQuery::field_errors(&self) -> Vec<(&QueryTerm, &FieldValueError)>` | function | 004 |
| `QueryTerm { span: Range<usize>, excluded: bool, kind: TermKind }` | struct | 002, 004 |
| `TermKind::{Word(String), Phrase(String), Field(FieldTerm)}` | enum | 002 |
| `FieldTerm { name: QueryFieldName, value: FieldValueState }` | struct | 002, 004 |
| `FieldValueState::{Valid(FieldValue), Incomplete, Invalid(FieldValueError)}` | enum | 002, 004 |
| `FieldValue::{Text(String), Kinds(Vec<KindValue>), Scope(ScopeValue), Date(DateValue), Duration(DurationRange), LibraryState(LibraryStateValue), Content(ContentValue)}` | enum | 002 |
| `FieldValueError` with `message(&self) -> String` | enum | 004 |
| `QueryFieldName::{Artist, Title, Release, Kind, In, After, Before, Duration, Is, Playlist, Has}` | enum | 002, 004, 005 |
| `QueryFieldName::from_name(name: &str) -> Option<QueryFieldName>` | function | 004 |
| `QueryFieldName::info(self) -> &'static QueryFieldInfo` | function | 002, 004 |
| `QueryFieldName::is_library_only(self) -> bool` | function | 002, 004 |
| `QueryFieldName::fixed_values(self) -> &'static [FixedValueInfo]` | function | 005 |
| `QueryFieldName::values_with_prefix(self, prefix: &str) -> Vec<&'static FixedValueInfo>` | function | 005 |
| `QueryFieldInfo { name, token, value_kind, values_text, example, description, library_only }` | struct | 004 |
| `FieldValueKind::{Text, KindList, Scope, Date, Duration, LibraryState, Content}` | enum | 004, 005 |
| `FixedValueInfo { word, description }` | struct | 005 |
| `QUERY_FIELDS: [QueryFieldInfo; 11]` | constant | 004 |
| `fields_with_prefix(prefix: &str) -> Vec<&'static QueryFieldInfo>` | function | 004 |
| `KindValue::{Album, Ep, Single}` with `word(self) -> &'static str` | enum | 002 |
| `ScopeValue::{Library, Index}` | enum | 002 |
| `LibraryStateValue::{Downloaded, Partial, Ready, Removed}` | enum | 002 |
| `ContentValue::{Split, Image}` | enum | 002 |
| `DateValue { first_day, last_day }` with `start_unix(&self) -> i64` and `end_unix(&self) -> i64` | struct | 002 |
| `DurationRange { min_secs: Option<u32>, max_secs: Option<u32> }` with `contains(&self, secs: u32) -> bool` | struct | 002 |
| `QueryScope::{Everything, LibraryOnly, IndexOnly, Conflict}` | enum | 002, 004 |
| `cursor_term(text: &str, cursor: usize) -> CursorTerm` | function | 004, 005 |
| `CursorTerm { range: Range<usize>, excluded: bool, partial: CursorPartial }` | struct | 004, 005 |
| `CursorPartial::{FieldName { prefix }, FieldValue { field, prefix, value_range }, Other}` | enum | 004, 005 |

The fields of each struct are `pub`. A later packet adds a name only through its own packet text.

## Orchestrator Decisions

- **D1. The module is `src/view_models/search_query.rs`.** ADR 0087 Decision 1 puts the parser in
  the view model layer. The name follows the snake_case nouns of `src/view_models/mod.rs`.
  `text_filter.rs` compares text and reads no terms, so the parser does not go into it.
- **D2. The parser has its own `KindValue`.** `ReleaseKind` is `pub(crate)`. A `pub` type that
  holds it causes the `private_interfaces` warning. Guard R15-09 also rejects the text
  `ReleaseKind::Single` outside its two owners.

  Packet 002 maps `KindValue` to `ReleaseKind` in
  `release_kind.rs`. In this packet, a unit test proves that each `KindValue::word` resolves
  through `ReleaseKind::resolve` to the kind with the same name.
- **D3. Repeated fields all apply.** Each term of one query applies (Decision 1). Two text fields
  of one name both must match. `kind:album,ep` matches album or EP. `kind:album kind:single`
  thus matches nothing.

  Two `after:` fields act as the later date, and two `before:` fields act
  as the earlier date. Two `duration:` fields act as the overlap of their ranges. The parser
  keeps each term. It does not merge terms, so each term keeps its own range and its own error.
- **D4. `after:` later than `before:` is not an error.** Each value is correct by itself. The
  query then matches nothing. An error is about one field value (Decision 1).
- **D5. An empty value is `Incomplete`, not an error.** While the curator types `artist:`, an
  error text in the field list is not useful. The field list shows the values. The field applies
  nothing and does not count as runnable content.
- **D6. `-in:` is not correct.** `in:` sets the scope and does not match rows. An exclusion of a
  scope has no clear meaning. The error tells the curator to use `in:library` or `in:index`.
- **D7. `in:`, `is:` and `has:` take one value.** ADR 0087 Decision 2 gives commas to `kind:`
  only. A comma in these values gives an error that names the field.
- **D8. A Library-only field with `-` still sets `LibraryOnly`.** The Index cannot apply the
  field, so it cannot apply its exclusion. ADR 0087 Decision 5 forbids Index rows that a filter
  did not reduce.
- **D9. An excluded word or phrase is an Index filter.** `q` cannot exclude today, and
  request 10 does not ask for it. `index_filter_terms` gives it, and packet 002 then shows no
  Index rows with the note. Index rows that ignore an exclusion are not reduced rows.
- **D10. A quote with no closing quote closes at the end of the text.** The curator is then
  probably still typing the phrase. The term applies its text so far, and the live run of
  packet 003 shows the same rows as after the closing quote.
- **D11. A backslash escapes a quote or a backslash inside quotes only.** Outside quotes, a
  backslash is an ordinary character. This keeps each name of today unchanged in a saved search,
  because only a quoted phrase can hold `\"`.
- **D12. A duration needs a comparison or a range.** A bare amount such as `5m` has no clear
  meaning, because track durations are rarely exact minutes. A bare number without a unit is
  also not correct. A curator can type `3m..5m` or `>=5m`.
- **D13. Ranges are in bytes.** Rust strings index by byte. The parser gives each range on a
  character boundary, so `&text[range]` is always correct. `InputState::cursor` already gives a
  byte offset, so packet 004 gives it to `cursor_term` with no conversion.
- **D14. The items are `pub`.** `src/lib.rs:41` declares `pub mod view_models`. A `pub` item in a
  `pub mod` is part of the library interface, so an item that only packet 002 calls is not dead
  code. This packet does not use `#[expect(dead_code)]`. Packet 002 connects the parser to the
  search path in the next session.
- **D15. The dates use UTC.** `fmt_date` shows each release date in UTC. A filter in another time
  zone could then exclude a row that shows the same date as the filter.
- **D16. The guard checks literals, not calls.** The invariant forbids field syntax outside the
  parser. A string literal that starts with a field token is the shortest sign of a second
  parser. The guard also asserts that the owner holds each token, so the guard list and the
  table stay equal.

## Required Changes

1. Add `src/view_models/search_query.rs` with `//!` module text that names ADR 0087 Decisions 1
   and 2, and `#![warn(clippy::pedantic)]`. Add each name of "Names For Later Packets", the
   grammar, the value kinds, the field table and the fixed values of "Design".
2. In `src/view_models/mod.rs`, declare `pub mod search_query;` in alphabetical order.
3. Add a `#[cfg(test)] mod tests` in `src/view_models/search_query.rs` with each test of
   "Mechanical Acceptance Criteria".
4. Add one situational guard `adr_0087_query_parser_r87_1_30_one_parser_reads_field_syntax` in
   `tests/architecture_tests.rs`. It cites ADR 0087 Decision 1 and the invariant "One parser
   reads the query text". It collects each Rust file under `src/` except
   `src/view_models/search_query.rs`.

   It reads `production_source(&code_only(&source))` and its
   `string_literals`. A literal that starts with one of the 11 tokens is a violation. The guard
   also asserts that the owner file holds each token as a literal. The failure message is:

   ```text
   ADR 0087 Decision 1 (one parser reads the query text): <file> holds the field syntax
   literal "<token>". Only src/view_models/search_query.rs reads field syntax. Call
   SearchQuery::parse and read its typed terms, or read the token from QUERY_FIELDS.
   ```

5. Add one row to `docs/architecture/source-map.md`: "Music search query parser: the terms, the
   field table, the scope and the cursor term (ADR 0087 Decision 1, packet 001)", owner
   `src/view_models/search_query.rs`.

## Mechanical Acceptance Criteria

Use the prefix `adr_0087_query_parser_` for each unit test in `src/view_models/search_query.rs`.

| Case | Required proof |
|---|---|
| R87-1-01 | `artist:jsmovie kind:single after:2026-10-01 -title:remix "night drive"` gives five terms. In text order, they are `Artist` Text `jsmovie`, `Kind` `[Single]`, `After` 2026-10-01, `Title` Text `remix` and Phrase `night drive`. The `After` value has first day and last day 2026-10-01. The `Title` term has `excluded` `true`. Each term without `-` has `excluded` `false` |
| R87-1-02 | The span of each term of R87-1-01 gives its source text, for example `-title:remix` and `"night drive"` |
| R87-1-03 | `Re:Union` gives one Word `Re:Union`. `foo:bar` gives one Word `foo:bar` |
| R87-1-04 | `after:yesterday` gives `Invalid` with the message "after: needs a date, for example 2026-10-01". `after:2026-02-30` and `after:2026-1-5` give `Invalid` |
| R87-1-05 | `Artist:x`, `ARTIST:x` and `artist:x` each give the field `Artist`. `KIND:Single` gives `[Single]` |
| R87-1-06 | `title:Remix` keeps the case of `Remix`. `release:"aged friends"` gives Text `aged friends` |
| R87-1-07 | `kind:album,ep` gives `[Album, Ep]`. `kind:single,single,` gives `[Single]`. `kind:lp` gives `Invalid` and the message names `lp` |
| R87-1-08 | `in:library` and `in:index` give `Scope`. `in:both` and `in:library,index` give `Invalid`. `-in:index` gives `Invalid` (D6) |
| R87-1-09 | `after:2026-10` gives first day 2026-10-01 and last day 2026-10-31. `before:2026-02` gives last day 2026-02-28. `start_unix` of 2026-10-01 is `1790812800` and `end_unix` of 2026-10-31 is `1793491199` |
| R87-1-10 | `duration:>5m` gives `min_secs` 301 and no maximum. `>=5m` gives 300. `<3:30` gives `max_secs` 209. `<=3:30` gives 210. `3m..5m` gives 180 and 300. `1h30m`, `90s` and `1:02:03` give 5400, 90 and 3723 seconds |
| R87-1-11 | `duration:5m`, `duration:5`, `duration:2.5m`, `duration:3m..`, `duration:5m..3m`, `duration:30s3m`, `duration:3:5` and `duration:<0s` each give `Invalid` |
| R87-1-12 | `is:downloaded`, `is:partial`, `is:ready` and `is:removed` give the four `LibraryStateValue`s. `has:split` and `has:image` give the two `ContentValue`s. `is:new` and `has:art` give `Invalid` |
| R87-1-13 | `artist:` and `artist:""` give `Incomplete`, not `Invalid`. `artist: jsmovie` gives an `Incomplete` `artist:` and the Word `jsmovie` |
| R87-1-14 | `"night drive` gives the Phrase `night drive`. `release:"aged fr` gives Text `aged fr` |
| R87-1-15 | `"the \"best\" song"` gives the Phrase `the "best" song`. `"a\\b"` gives `a\b`. The word `a\b` outside quotes stays `a\b` |
| R87-1-16 | `lo-fi` gives one Word `lo-fi`. A lone `-` gives no term. `-remix` gives the Word `remix` with `excluded` `true` |
| R87-1-17 | `scope` gives `Everything` for `haleen`, `LibraryOnly` for `is:ready`, for `-has:split` and for `in:library`, `IndexOnly` for `in:index`, and `Conflict` for `in:index is:ready` and for `in:library in:index`. `is:bogus in:index` gives `IndexOnly` |
| R87-1-18 | `index_q` of R87-1-01 gives `"night drive"`. `index_q` of `haleen "a \"b\""` gives `haleen "a \"b\""`. `index_q` of `artist:x -remix` gives `None`. No `index_q` result holds a field token |
| R87-1-19 | `index_filter_terms` of R87-1-01 gives the four field terms. `index_filter_terms` of `in:index -remix` gives the excluded Word only |
| R87-1-20 | `has_runnable_content` is `true` for `haleen`, `"night drive"` and `kind:single`. It is `false` for an empty text, `!!!`, `-remix`, `in:library`, `artist:`, `after:yesterday` and `-kind:single` |
| R87-1-21 | `cursor_term("ar", 2)` gives `FieldName` with prefix `ar` and range `0..2`. `fields_with_prefix("ar")` gives `artist:` only. `fields_with_prefix("")` gives the 11 rows |
| R87-1-22 | `cursor_term("kind:s", 6)` gives `FieldValue` with `Kind`, prefix `s` and value range `5..6`. `values_with_prefix` of `Kind` and `s` gives `single` only |
| R87-1-23 | `cursor_term("kind:album,e", 12)` gives prefix `e` and value range `11..12`. `cursor_term("haleen -ar", 10)` gives `excluded` `true` and range `8..10` |
| R87-1-24 | `cursor_term("haleen ", 7)` gives an empty `FieldName` prefix and range `7..7`. `cursor_term("\"night dr", 9)` and `cursor_term("Re:Un", 5)` give `Other` |
| R87-1-25 | `cursor_term` with a cursor inside a multi-byte character, and with a cursor after the end, gives a range on a character boundary. `cursor_term("artist:é", 9)` gives prefix `é` |
| R87-1-26 | `QUERY_FIELDS` has 11 rows in the order of Decision 2. Each `token` is `name` in lowercase with a colon. `QueryFieldName::from_name` of each token without the colon gives the row's name. Exactly `Is`, `Playlist` and `Has` have `library_only` `true` |
| R87-1-27 | `fixed_values` gives three words for `Kind`, two for `In`, four for `Is`, two for `Has` and none for each other field. Each fixed word parses as a `Valid` value of its field |
| R87-1-28 | Each `KindValue::word` resolves through `ReleaseKind::resolve(Some(word), 2)` to a kind whose `label` matches the word without case (D2) |
| R87-1-29 | `field_errors` of `after:yesterday kind:lp haleen` gives two entries in text order. Each `FieldValueError::message` is not empty and starts with the field token |
| R87-1-30 | The situational guard of "Required Changes" item 4 passes. It fails against a copy of a production file that holds the literal `"kind:"` |

## Visual Acceptance Criteria

This packet has none. It changes no interface and no behavior. No production caller reads the
parser until packet 002, so the running app shows the same screens as before this packet. The
visual checks of ADR 0087 belong to packets 002 to 005.

## Exclusions

- No change to `normalized_global_search_query`, `submit_global_search`,
  `search_library_tracks`, `fetch_index_search_result_rows` or the saved searches. Packet 002
  connects the parser.
- No Library query builder, no Index parameter and no note text of Decision 5 (packet 002).
- No pause, generation or live run (packet 003).
- No field list view model, composite or keys (packet 004).
- No value query for `artist:`, `release:` or `playlist:` (packet 005).
- No change to `ReleaseKind` or `release_kind.rs` (D2).
- No schema change and no network request.

## Files To Inspect

- [AGENTS.md](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- [ADR 0087](../adr/0087-search-fields-and-live-results.md) and the
  [ADR 0087 phase plan](../plans/adr-0087-search-fields-phase-plan.md).
- `src/view_models/mod.rs`, `src/view_models/text_filter.rs`, `src/view_models/release_kind.rs`
  and `src/view_models/format.rs` (`fmt_date`).
- `src/application/queries/search.rs` (`normalized_global_search_query`) and
  `src/app/search_dispatch.rs` (`submit_global_search`), to read only.
- `tests/architecture_tests.rs`: `collect_rust_files`, `production_source`, `code_only`,
  `string_literals`, `read_source`, `rel_path`, `manifest_path`, and guard R15-09 for the shape
  of the new guard.

## Checks

```bash
cargo test --lib adr_0087_query_parser_
cargo test --lib adr_0084_release_kind_
cargo check --all-targets
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --test architecture_tests
touch src/main.rs && cargo build --bin v4vmm
```

## Rollback

Revert the commit. This packet adds no migration, no stored data and no caller.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0087-task-001-query-parser.md`
- ADR 0087 Decisions 1 to 5 and its invariants
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes".

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use
  the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Follow each decision in "Orchestrator Decisions". Do not change a decision.
- Use each name of "Names For Later Packets" exactly. Do not add a public name that the packet
  does not give. Private helpers are permitted.
- The module has no GPUI type, no I/O, no `unsafe` and no `unwrap` in production code.
- Use `chrono::NaiveDate` for the dates. Do not add a dependency.
- Tests use fixed text. No test sends a request or opens a database.
- Never run `git add`, `git rm`, `git commit`, `git checkout`, `git restore`, `git stash` or
  `git reset`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- `src/application/queries/search.rs`, `src/app/search_dispatch.rs`, `src/app.rs`, `src/db.rs`
  and `src/db/saved_searches.rs`.
- `src/view_models/release_kind.rs` and `src/view_models/text_filter.rs`.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet and `docs/architecture/source-map.md`.

Acceptance criteria:
- Each case R87-1-01 to R87-1-30 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - 2026-10-23" section.

At the end, report: files changed, tests run, behavior changed, deviations, unresolved concerns,
and the document changes that the orchestrator must make.

Stop and report the problem, and do not guess, when:
- A change needs a new architecture decision.
- A change needs a file in "Do not touch".
- A name of "Names For Later Packets" causes a compiler warning that only a lint exception can
  remove.
- A recorded fact of this packet is not true in the working tree.
