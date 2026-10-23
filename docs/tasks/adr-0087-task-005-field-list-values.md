# ADR 0087 Task 005: The Values Of A Field In The Field List

Status: Ready - 2026-10-23.

Orchestrator note - 2026-10-23: packets 001 to 004 are implemented. The run record lists their deviations. Packet 003 gives `Motion::SearchPause`, `present_after` and `crate::view_models::live_run::{LiveRunGate, PauseTicket, RunGeneration}`. `LiveRunGate::current_run` and `run_generation()` are `#[cfg(test)]`. Remove the attribute of each name that this packet calls.

## Goal

After a field name and its colon, the field list shows the values of that field:

- `artist:` shows the matching artist names of the Library and of `GET /v1/artist-names?from=`.
- `release:` shows the matching release titles of the Library.
- `playlist:` shows the matching playlist names.

Packet 004 already shows the fixed values of `kind:`, `in:`, `is:` and `has:` (its D5). This
packet does not change them.

The Library values come from the Library tree in memory. They update on each change of the text.
The MusicIndex names come from one request after each pause, as operator answer 1 of ADR 0087
says. A loading row shows while the curator types and while the request runs. A failed request
shows an error row, and the Library rows stay. An answer of an older request never changes the
list.

Each value row has a source mark in words, "Library" or "MusicIndex". Tab, Enter or a click puts
the value into the text. A value with whitespace gets double quotes.

## Authority

- [ADR 0087](../adr/0087-search-fields-and-live-results.md) Decision 3: the value rows of
  `artist:`, `release:` and `playlist:`, the keys, the focus and the view model rule.
- [ADR 0087](../adr/0087-search-fields-and-live-results.md) operator answer 1 of 2026-10-23: the
  field list sends one `GET /v1/artist-names?from=` request after each pause, with the Library
  names.
- [ADR 0087](../adr/0087-search-fields-and-live-results.md) Decision 4: the pause, the generation
  and the rule that the app drops a result of an older generation.
- [ADR 0087](../adr/0087-search-fields-and-live-results.md) invariants "One parser reads the query
  text" and "A Library value goes into SQL only as a bound parameter, with its `LIKE` characters
  escaped".
- [ADR 0085](../adr/0085-artists-by-confirmation.md) Decision 1: no interface text calls a feed
  "publisher". Decision 2: the confirmation levels of an artist.
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 7: the sort key of a name.
- [ADR 0040](../adr/0040-async-vm-runtime.md): background work runs through the command runner, never through
  `cx.spawn` in a screen.
- [ADR 0063](../adr/0063-show-dashboard-layout.md): column text does not call `truncate()`.
- The [ADR 0087 phase plan](../plans/adr-0087-search-fields-phase-plan.md), packet 005. It depends
  on packets 003 and 004.
- [Packet 001](archive/adr-0087-task-001-query-parser.md), [packet 002](adr-0087-task-002-library-fields-and-index-words.md)
  and [packet 004](adr-0087-task-004-field-list.md). Packet 003 gives the pause and the run
  generation. These four packets are not implemented.
- [AGENTS.md](../../AGENTS.md): renderer portability, token discipline, typed action state,
  accessibility, "Treat RSS and MusicIndex responses as untrusted input" and "A guard names its
  class and its ADR".

## Recorded Facts - 2026-10-23

The orchestrator read each fact below in the working tree on 2026-10-23. The last two subsections
give facts from the designs of packets 001 to 004. Those facts are not in the code today.

### The artist name request

- `src/api.rs:97-100` `ArtistNamesResponse` holds `data: Vec<ArtistNameRow>` and `pagination`.
- `src/api.rs:105-117` `ArtistNameKind` has `Publisher`, `Name` and `Unknown`. `Unknown` takes
  each kind that the app does not know.
- `src/api.rs:120-138` `ArtistNameRow` holds `kind`, `name`, `sort_name`, `key` and the link
  facts. The link facts come only with `include=link_facts`, and only on a `publisher` row.
- `src/api.rs:1004-1019` `Client::fetch_artist_names(limit, cursor, from, profile)` sends
  `GET /v1/artist-names`. It adds `from` when it is `Some`. A `debug_assert_eq!` requires the path
  shape `ArtistNameList`.
- `tests/fixtures/musicindex-openapi-0.13.0.json`, path `/v1/artist-names`: the rows come in the
  order of `sort_name`, then `kind`, then `key`. `from` starts the list at the first row whose
  sort key is equal to or after the sort key of the text. `limit` is from 1 to 200. `include`
  supports `link_facts`.
- `src/view_models/index_name_list.rs:114-119` `name_sort_key` is the Stophammer sort key. It
  removes the whitespace at the two ends, makes each internal whitespace run one space, and makes
  each character Unicode lowercase.
- `src/application/queries/search.rs:170-182` `index_name_row_exists` calls
  `fetch_artist_names(1, None, Some(key), &INDEX_ARTIST_NAME_CHECK)`. The key is a
  `name_sort_key` value.
- Packet 017 recorded that `GET /v1/artist-names?from=haleen&limit=6` gave one `name` row "Haleen"
  and two `publisher` rows "Haleen" (`docs/tasks/adr-0084-task-017-name-level-artist-page.md:126`).
  Thus one name can give more than one row.
- `src/application/request_profiles.rs:144-145` `INDEX_ARTIST_NAME_CHECK` has the path shape
  `ArtistNameList` and no include. `:150-151` `INDEX_ARTIST_NAME_LIST` has `link_facts`. `:187-200`
  `ALL` holds 12 profiles. It does not hold `INDEX_ARTIST_NAME_CHECK`.
- `src/application/queries/name_lists.rs:282-322` `FetchArtistNamePage` is an
  `ApplicationCommand`. It checks the cancellation, calls `fetch_artist_names` and maps an error to
  `CommandError::Query`. `:41` `NAME_LIST_PAGE_LIMIT` is 100. `:334` `serve` is a local HTTP test
  server that records each request path.
- `src/view_models/artists_everything.rs:133` `IndexArtistClass`, `:154` `is_shown` and `:162-178`
  `classify_index_artist`. `is_shown` is true for `Artist`, `UnverifiedArtist` and `Name`. A
  `publisher` row needs its link facts for this class.

### The Library in memory

- `src/view_models/library.rs:556-558` `LibraryTree` holds `artists: Vec<ArtistNode>`. `:524-527`
  `ArtistNode` holds `name` and `albums`. `:533-551` `AlbumNode` holds `tracks: Vec<TrackRow>`.
- `src/db.rs:23-24` `TrackRow` derives `Default`. `:30` `artist_name`, `:31` `album_title`, `:32`
  `album_artist_name` and `:40` `feed_title` are `Option<String>`.
- `src/db.rs:79-86` `Playlist` holds `id` and `name`.
- `src/view_models/library.rs:581-592` `LibrarySnapshot` holds `tree` and `playlists`. `:3012`
  `tree()` and `:3319` `playlists()` read them.
- `src/view_models/library.rs:3028-3032` `replace_tree` and `:3324-3326` `replace_playlists` are
  the only assignments of `snapshot.tree` and `snapshot.playlists`. `:3339` sorts the playlists and
  does not change a name.
- `src/library/app_impl.rs:1454` calls `replace_tree` in the success callback of the Library load,
  on the main thread.
- `src/library/app_impl.rs:767-786` `LibraryApp::artist_search_facts` builds search facts from the
  tree. `src/app/search_dispatch.rs:276` reads it with `self.library.read(cx)`.
- `src/db.rs:1350-1364` `like_contains_pattern` is private to `src/db.rs`.

### The runtime path

- `src/app.rs:132` declares `TopApp`. `:134` holds `library: Entity<LibraryApp>`, `:178`
  `musicindex_endpoint` and `:213` `command_runner: AsyncCommandRunner`.
- `src/presentation/async_command_presenter.rs:18-57` `present_command` dispatches a command
  through the runner. It drops a result of an older runner session. It has no check for an older
  request of the same kind.
- `src/app/search_dispatch.rs:364-447` `dispatch_index_search` uses `present_command`. Its success
  callback checks the query of the mounted page before it changes the page.
- `src/ui/tokens.rs:1080-1097` `Motion` has one token, `SpringLoadDelay`. No search pause token
  exists today.
- `src/view_models/artist_identity.rs:26` `ARTIST_NAME_ROW_MARK` is "by name".
- `tests/architecture_tests.rs:19567` `adr_0085_feed_page_guard_no_interface_text_names_a_feed_publisher`
  rejects interface literals that call a feed "publisher".

### Facts from the designs of packets 001, 002 and 004

- Packet 001: `cursor_term(text, cursor)` gives `CursorPartial::FieldValue { field, prefix,
  value_range }` when the cursor is after a known name and its colon. Inside quotes, `\"` gives a
  quote and `\\` gives a backslash. An open quote closes at the end of the text.
- Packet 001: the guard R87-1-30 rejects a production string literal that starts with a field
  token outside `src/view_models/search_query.rs`.
- Packet 001: `SearchQuery::scope` gives `LibraryOnly` for `in:library` and for a Library-only
  field, and `Conflict` for `in:index` with a Library-only field.
- Packet 002: `artist:` matches a part of `t.artist_name` or `t.album_artist_name`. `release:`
  matches a part of `t.album_title` or `f.title`. `playlist:` matches a part of the playlist name.
  Each match is a `LIKE` contains match without ASCII case.
- Packet 004: `FieldListVm::build(text, selection, state)`, the row model, the edit rule of a
  `FixedValue` row and the keys. The keys skip an unavailable row. Its "Names For Later Packets"
  gives the names that this packet adds. R87-4-04 says that `artist:` gives `None` today.

### Facts that packet 003 gives

Packet 003 is not written on 2026-10-23. This packet uses "the pause and the run generation of
packet 003" and no name for them. The prompt stops when packet 003 gives no reusable owner.

## Design

### The layers

| Layer | File | Owns |
|---|---|---|
| Parser | `src/view_models/search_query.rs` | The quotes of a value and the text of a quoted prefix |
| Value model | `src/view_models/search_field_values.rs` (new) | The Library values, the MusicIndex names, the request key, the text checks and the limits |
| Row model | `src/view_models/search_field_list.rs` | The value rows, the loading row and the error row, with their labels, marks, actions and availability |
| Library state | `src/view_models/library.rs` and `src/library/app_impl.rs` | One `LibraryFieldValues` that each change of the tree or of the playlists builds again |
| Command | `src/application/queries/name_lists.rs` and `src/application/request_profiles.rs` | The one MusicIndex request and its profile |
| Screen glue | `src/app/search_field_list.rs` | The request state, the stale check and the dispatch through the command runner |
| Composite | `src/ui/composites/search_field_list.rs` | No new text. It draws the new rows with the row layout of packet 004 |

### The quotes

Packet 005 adds two functions to `src/view_models/search_query.rs`, the owner of the field syntax:

- `quote_field_value(text: &str) -> String` gives the text as one field value. When the text holds
  a whitespace character or a `"`, it gives the text in double quotes. Inside the quotes, `\`
  becomes `\\` and `"` becomes `\"`. Else, it gives the text with no change.
- `unquote_field_value_prefix(prefix: &str) -> String` gives the text of a value prefix. It
  removes one `"` at the start, and resolves `\"` and `\\` after it. A prefix with no `"` at the
  start does not change.

For each text `t` that the value model accepts, `SearchQuery::parse` of the field token and
`quote_field_value(t)` gives `FieldValue::Text(t)`.

### The Library values

`LibraryFieldValues` holds three sorted lists of `FieldValueEntry { text: String, key: String }`,
for the artists, the releases and the playlists. `key` is `name_sort_key(&text)`.

- `LibraryFieldValues::from_library(tree: &LibraryTree, playlists: &[db::Playlist]) -> Self`
  reads `artist_name` and `album_artist_name` of each track for the artists. It reads
  `album_title` and `feed_title` of each track for the releases. It reads the `name` of each
  playlist. These are the columns that packet 002 matches.
- `LibraryFieldValues::from_texts(artists: &[&str], releases: &[&str], playlists: &[&str]) -> Self`
  makes the same lists from texts. `from_library` calls it. Tests call it.
- Each list keeps one entry for each key. With two texts of one key, the entry keeps the text that
  comes first in `String` order.
- Each list drops a text that `value_text_is_usable` rejects (see "The untrusted text").
- The lists are in key order.

`LibraryFieldValues::matching(&self, field: QueryFieldName, prefix: &str) -> Vec<&FieldValueEntry>`
reads the list of `Artist`, `Release` or `Playlist`. Each other field gives no entry. The match
key is `name_sort_key(&unquote_field_value_prefix(prefix))`.

- An entry matches when its key contains the match key. An empty match key matches each entry.
- The entries whose key starts with the match key come first, in key order. The other matches
  follow, in key order.
- The function gives at most `LIBRARY_VALUE_ROW_LIMIT` entries, 8.

`LibraryViewModel` holds `field_values: Arc<LibraryFieldValues>`. `replace_tree` and
`replace_playlists` build it again after they change the snapshot.
`LibraryViewModel::field_values(&self) -> Arc<LibraryFieldValues>` gives a clone of the `Arc`.
`LibraryApp::search_field_values(&self) -> Arc<LibraryFieldValues>` gives the same value to
`src/app`.

### The MusicIndex names

`artist_value_request_key(text: &str, cursor: usize) -> Option<String>` gives the key of the one
request. It gives `Some(key)` only when all these conditions are true:

- `cursor_term(text, cursor).partial` is `FieldValue` with the field `Artist`.
- `key` is `name_sort_key(&unquote_field_value_prefix(prefix))`, and `key` is not empty.
- `SearchQuery::parse(text).scope()` is `Everything` or `IndexOnly`.

`IndexArtistValues` is the state of the request in the value model:

| Variant | Meaning |
|---|---|
| `NotRequested` | No request applies to the text |
| `Loading { key }` | The pause or the request for `key` runs |
| `Loaded { key, names: Vec<String> }` | The usable names of the answer for `key` |
| `Failed { key }` | The request for `key` failed |

`IndexArtistValues::loaded(key: String, rows: &[ArtistNameRow]) -> Self` gives `Loaded`. It keeps
a row only when all these conditions are true:

- `classify_index_artist(row).is_shown()` is true. Thus a feed that is not an artist by ADR 0085
  Decision 2 gives no name.
- `name_sort_key(&row.name)` starts with `key`. The `from` order gives the rows after the key too,
  and this test removes them.
- `value_text_is_usable(&row.name)` is true.

It keeps one name for each `name_sort_key`, in the order of the answer. It keeps at most
`INDEX_VALUE_ROW_LIMIT` names, 8. A `name` row and a `publisher` row give the same thing: the
`name` text as a field value. The `artist:` field matches text, not a feed.

`IndexArtistValues::key(&self) -> Option<&str>` gives the key of each variant other than
`NotRequested`.

### The untrusted text

`value_text_is_usable(text: &str) -> bool` is private to `search_field_values.rs`. It gives `false`
for each of these texts:

- an empty text, or a text with only whitespace
- a text with a `char::is_control` character, for example a line feed or a tab
- a text with a bidirectional control character, U+202A to U+202E or U+2066 to U+2069
- a text with more than `FIELD_VALUE_MAX_CHARS` characters, 200

A usable text goes into the row with no other change. The label shows the same text that Tab
puts into the field. Markup such as `<b>` stays text, because no renderer reads markup.

### The rows

`FieldListVm::build` gets one more argument: `values: &FieldValueRows<'_>`.
`FieldValueRows<'a> { library: &'a LibraryFieldValues, index: &'a IndexArtistValues }` holds the
two sources. `FieldValueRows::EMPTY` holds an empty `LibraryFieldValues` and `NotRequested`.

For `CursorPartial::FieldValue { field, prefix, .. }`, `build` gives the rows in this order:

1. The `FixedValue` rows of packet 004, for `kind:`, `in:`, `is:` and `has:`.
2. One `Value` row with the source `Library` for each entry of
   `values.library.matching(field, prefix)`.
3. For `Artist` only, the MusicIndex rows. `build` calls `artist_value_request_key(text,
   selection.start)`. When that key is `Some` and equals `values.index.key()`, the state gives:
   - `Loaded`: one `Value` row with the source `Index` for each name whose key is not the key of
     a Library row of step 2.
   - `Loading`: one `Loading` row.
   - `Failed`: one `IndexError` row.

   Else, step 3 gives no row.

A name in the Library and in MusicIndex thus gives one row, with the source `Library` (D4).

| Field | `Value` row | `Loading` row | `IndexError` row |
|---|---|---|---|
| `element_id` | `search-field-list-value-`, the token without the colon, `-`, `library` or `index`, `-` and the row position in its source | `search-field-list-loading-` and the token without the colon | `search-field-list-index-error-` and the token without the colon |
| `kind` | `Value { field, text, source }` | `Loading { field }` | `IndexError { field }` |
| `label` | The text | "Loading names from MusicIndex" | "Names from MusicIndex did not load" |
| `values_text` | `None` | `None` | `None` |
| `example_text` | `None` | `None` | `None` |
| `mark` | `Some(FieldListMark::Source(source))` | `None` | `None` |
| `note` | The text of the unavailable reason, or `None` | `None` | "The Library names show. The next change of the text tries again." |
| `a11y_label` | "Value `text` for `token`, from the Library" or "Value `text` for `token`, from MusicIndex", with ", " and the note when unavailable | "Loading artist names from MusicIndex" | "Artist names from MusicIndex did not load. The Library names show." |
| `action` | `InsertText { field, text }` | `Inert` | `Inert` |
| `availability` | `Available`, or `Unavailable(ScopeConflict)` | `Unavailable(Loading)` | `Unavailable(IndexError)` |
| `edit` | The edit rule of a `FixedValue` row, with the text `quote_field_value(text)` | The empty edit at the cursor | The empty edit at the cursor |

- `FieldListMark::Source(FieldValueSource::Library).label()` gives "Library".
  `FieldListMark::Source(FieldValueSource::Index).label()` gives "MusicIndex". The mark is a word,
  so the color is not the only cue.
- With `IndexOnly`, each `Value` row of `playlist:` is `Unavailable(ScopeConflict)`, with the note
  of packet 004 "Not available with in:index". This is the rule of the `is:` and `has:` rows.
- The empty edit has the range `cursor..cursor`, an empty text and the same cursor. No key and no
  click applies it, because its row is not available.
- The `key` function of packet 004 does not change. It skips the `Loading` and `IndexError` rows.

### The request

`SearchFieldListView` holds `index_values: IndexArtistValues` and one run generation of packet 003.
After each event that packet 004 sends to `FieldListState`, the view calls
`artist_value_request_key` with the new text and cursor:

- `None`: `index_values` becomes `NotRequested`. The generation goes up, so a running answer is
  dropped.
- The same key as `index_values.key()`: nothing changes. A cursor move in the same value sends no
  new request.
- A new key: the generation goes up. `index_values` becomes `Loading { key }`. The view starts the
  pause of packet 003. When the pause ends with the same generation, the view emits
  `FieldValueRequest { key, generation }`.

`TopApp` subscribes to `FieldValueRequest` in `src/app/search_field_list.rs`.
`TopApp::dispatch_field_value_request` sends `FetchArtistNameValues::new(endpoint, key)` through
`present_command`, with the current `musicindex_endpoint` and `command_runner`.

- The success callback calls `SearchFieldListView::apply_index_names(generation, key, rows, cx)`.
- The error callback calls `SearchFieldListView::apply_index_failure(generation, key, cx)`.
- Each function changes `index_values` only when `generation` is the current generation and `key`
  equals `index_values.key()`. Else it does nothing. Then it calls `cx.notify()`.
- The error text of the command does not go into the list. It can hold endpoint details.

`FetchArtistNameValues` is an `ApplicationCommand` in `src/application/queries/name_lists.rs`. Its
`Output` is `Vec<ArtistNameRow>`. It checks the cancellation, then calls
`fetch_artist_names(FIELD_LIST_ARTIST_NAME_LIMIT, None, Some(&key), &INDEX_FIELD_LIST_ARTIST_NAMES)`.
`FIELD_LIST_ARTIST_NAME_LIMIT` is 20. `INDEX_FIELD_LIST_ARTIST_NAMES` is
`RequestProfile::new(RequestPathShape::ArtistNameList, Some("link_facts"))`.

### Names For Later Packets

Module `crate::view_models::search_query` (packet 001 owns the module):

| Name | Kind |
|---|---|
| `quote_field_value(text: &str) -> String` | function, `pub` |
| `unquote_field_value_prefix(prefix: &str) -> String` | function, `pub` |

Module `crate::view_models::search_field_values` (`src/view_models/search_field_values.rs`, new):

| Name | Kind |
|---|---|
| `FieldValueSource::{Library, Index}` with `label(self) -> &'static str` | enum |
| `FieldValueEntry { text: String, key: String }` | struct |
| `LibraryFieldValues` with `from_library`, `from_texts` and `matching` | struct, `Default` |
| `IndexArtistValues::{NotRequested, Loading { key }, Loaded { key, names }, Failed { key }}` with `loaded` and `key` | enum |
| `FieldValueRows<'a> { library, index }` with `EMPTY` | struct |
| `artist_value_request_key(text: &str, cursor: usize) -> Option<String>` | function |
| `LIBRARY_VALUE_ROW_LIMIT: usize = 8` | constant |
| `INDEX_VALUE_ROW_LIMIT: usize = 8` | constant |
| `FIELD_VALUE_MAX_CHARS: usize = 200` | constant |

Module `crate::view_models::search_field_list` (packet 004 owns the module):

| Name | Kind |
|---|---|
| `FieldListRowKind::Value { field, text, source: FieldValueSource }` | variant |
| `FieldListRowKind::Loading { field }` | variant |
| `FieldListRowKind::IndexError { field }` | variant |
| `FieldListRowAction::InsertText { field, text }` | variant |
| `FieldListRowAction::Inert` | variant |
| `FieldListRowUnavailable::Loading` | variant |
| `FieldListRowUnavailable::IndexError` | variant |
| `FieldListMark::Source(FieldValueSource)` | variant |
| `FieldListVm::build(text, selection, state, values: &FieldValueRows<'_>) -> Option<FieldListVm>` | changed signature |

Other modules:

| Name | Module |
|---|---|
| `LibraryViewModel::field_values(&self) -> Arc<LibraryFieldValues>` | `src/view_models/library.rs` |
| `LibraryApp::search_field_values(&self) -> Arc<LibraryFieldValues>` | `src/library/app_impl.rs` |
| `FetchArtistNameValues` with `new(endpoint, key: String)` | `src/application/queries/name_lists.rs` |
| `FIELD_LIST_ARTIST_NAME_LIMIT: i32 = 20` | `src/application/queries/name_lists.rs` |
| `INDEX_FIELD_LIST_ARTIST_NAMES` | `src/application/request_profiles.rs` |
| `FieldValueRequest { key: String, generation }` | `src/app/search_field_list.rs`, an event of `SearchFieldListView` |
| `SearchFieldListView::new(input, library: Entity<LibraryApp>, window, cx)` | changed signature |
| `SearchFieldListView::apply_index_names` and `apply_index_failure` | `src/app/search_field_list.rs` |
| `TopApp::dispatch_field_value_request` | `src/app/search_field_list.rs` |

The `generation` field has the generation type of packet 003.

## Orchestrator Decisions

- **D1. The Library values come from the tree in memory, not from SQL.** `LibraryViewModel`
  already holds each Library track and each playlist (`library.rs:581-592`). Each tree track is a
  Library track, so the tree and the search read the same set. No value goes into SQL, so the
  `LIKE` invariant holds with no pattern. `like_contains_pattern` thus has no new caller.
- **D2. `LibraryFieldValues` is built at each change of the tree or of the playlists.**
  `replace_tree` and `replace_playlists` are the only assignments (`library.rs:3031`, `:3325`).
  One build walks each track one time. A match then reads only the distinct names, with no walk of
  the tree at each key.
- **D3. The Library match is a contains match. The MusicIndex match is a prefix match.** Packet
  002 matches a part of the name, so each Library row gives search rows. `from` gives the names at
  and after a sort key, so MusicIndex can give only a prefix. The Library rows with the prefix come
  first, so the two sources look the same at the top.
- **D4. A name in both sources gives one row with the source `Library`.** The row model of packet
  004 has one source for each row. The value text is the same, and `artist:` searches both
  sources. The "Library" mark tells the curator that the Library holds the name.
- **D5. A `publisher` row and a `name` row each give a name, never a feed.** The `artist:` field
  matches text. A row of a feed that is not an artist by ADR 0085 Decision 2 gives no name.
  Thus the request asks for `link_facts`. No text of this packet calls a feed "publisher".
- **D6. The request has its own profile, `INDEX_FIELD_LIST_ARTIST_NAMES`.** The check profile
  sends no include, and the list profile names another owner. A profile names its one owner
  (`request_profiles.rs:144-151`). The new profile stays out of `ALL`, as the check profile does.
- **D7. The request asks for 20 rows and the list shows at most 8 MusicIndex names.** One name can
  give three rows (packet 017, `from=haleen`). The rows of feeds that are not artists also take
  places. 8 Library rows and 8 MusicIndex rows fit the scroll height of packet 004.
- **D8. No request for an empty key, and no request in a Library scope.** A request for an empty
  key gives the first names of the alphabet, which do not help. With `in:library` or a
  Library-only field, the query shows no Index rows (ADR 0087 Decision 2). With `artist:` and no
  text, the list shows the first Library names only.
- **D9. The loading row shows from the change of the text to the answer.** The pause of packet
  003 comes before the request. A loading row at once tells the curator that MusicIndex names will
  come. An old answer for another key never shows, because `build` compares the keys.
- **D10. Two checks drop a stale answer.** The run generation of packet 003 drops each answer of
  an older change. The key check drops an answer that is not for the value under the cursor.
  `present_command` drops only an answer of an older runner session (`async_command_presenter.rs:18-57`).
- **D11. A failed request gives one error row, and the Library rows stay.** The row tells what
  failed and what shows. The command error does not go into the list, because it can hold endpoint
  details. The next new key sends a new request.
- **D12. The quotes come from `quote_field_value` in `search_query.rs`.** Packet 004 permits packet
  001 or packet 005 as the owner. The parser module owns each rule of the field syntax (ADR 0087
  invariant). Packet 005 adds the function to that module. The guard of packet 001 then needs no
  exception.
- **D13. A value text with a control character or with more than 200 characters gives no row.**
  RSS and MusicIndex text is untrusted input. A line feed cannot go into a single-line field. A
  changed text or a cut text could match the wrong rows. Thus the row is dropped, and a usable
  text stays exact.
- **D14. Each value row has a source mark, also in `release:` and `playlist:`.** One rule for each
  row is clear. The word is the cue, so the mark is not color alone.
- **D15. The view gets the Library entity, and `TopApp` sends the request.** The view reads the
  `Arc` of the values at each draw. `TopApp` holds the endpoint and the runner, and the endpoint
  can change in Settings. The view only emits the request event. No screen calls `cx.spawn`.
- **D16. The `Loading` and `IndexError` rows have the action `Inert`.** Each row of packet 004
  has one action and one edit. These two rows are never available, so no key and no click applies
  their empty edit.

## Required Changes

1. `src/view_models/search_query.rs`: add `quote_field_value` and `unquote_field_value_prefix`, as
   "The quotes" says, with their unit tests.
2. Add `src/view_models/search_field_values.rs` with `//!` module text that names ADR 0087
   Decision 3 and operator answer 1, and `#![warn(clippy::pedantic)]`. Add each name of the module
   table in "Names For Later Packets". Declare `pub(crate) mod search_field_values;` in
   `src/view_models/mod.rs`, in alphabetical order.
3. `src/view_models/search_field_list.rs`: add the variants of "Names For Later Packets" and the
   rules of "The rows". Add the argument `values` to `FieldListVm::build`. Update each existing
   test of packet 004 to pass `&FieldValueRows::EMPTY`.
4. `src/view_models/library.rs`: add the field `field_values: Arc<LibraryFieldValues>` to
   `LibraryViewModel`. Build it again in `replace_tree` and in `replace_playlists`. Add
   `field_values`.
5. `src/library/app_impl.rs`: add `LibraryApp::search_field_values`.
6. `src/application/request_profiles.rs`: add `INDEX_FIELD_LIST_ARTIST_NAMES` with a doc comment
   that names its owner, `name_lists::FetchArtistNameValues`. Do not add it to `ALL`.
7. `src/application/queries/name_lists.rs`: add `FetchArtistNameValues` and
   `FIELD_LIST_ARTIST_NAME_LIMIT`.
8. `src/app/search_field_list.rs`: add the `library` argument to `SearchFieldListView::new`.
   Add the field `index_values`, and use the run generation and the pause of packet 003. Add
   `FieldValueRequest`, `apply_index_names`, `apply_index_failure` and
   `TopApp::dispatch_field_value_request`. Pass `FieldValueRows` to `FieldListVm::build` in
   `render`.
9. `src/app.rs`: pass `library.clone()` to `SearchFieldListView::new`. Subscribe `TopApp` to
   `FieldValueRequest`.
10. `src/ui/composites/search_field_list.rs`: draw the new rows with the row layout of packet 004.
    Draw the mark through `render_row_badge(mark.label(), cx)`. Add no string literal.
11. Add each unit test of "Mechanical Acceptance Criteria" beside its owner.
12. Add the situational guard `adr_0087_field_values_r87_5_19_values_come_from_the_value_model` in
    `tests/architecture_tests.rs`. It cites ADR 0087 Decision 3. It asserts these facts:
    - `production_source(&code_only(..))` of `src/view_models/search_field_values.rs` holds no
      `rusqlite`, no `Connection` and no `gpui`.
    - `src/app/search_field_list.rs` holds no `cx.spawn(` and no `fetch_artist_names(`.
    - `src/ui/composites/search_field_list.rs` holds no `quote_field_value(` and no `name_sort_key(`.
    - `src/application/queries/name_lists.rs` holds `INDEX_FIELD_LIST_ARTIST_NAMES`.

    The failure message is:

    ```text
    ADR 0087 Decision 3 (the field list shows the values of a field): <file> <violation>.
    Build each value row in src/view_models/search_field_values.rs and search_field_list.rs,
    quote it with quote_field_value, and send the MusicIndex request through present_command.
    ```

13. `docs/architecture/source-map.md`: add `src/view_models/search_field_values.rs` to the row of
    the Music search field list, with "the values of `artist:`, `release:` and `playlist:` (ADR
    0087 Decision 3, packet 005)".

## Mechanical Acceptance Criteria

Use the prefix `adr_0087_field_values_` for each unit test. "Library H" means
`LibraryFieldValues::from_texts(&["Haleen", "Haleen Moss", "Shaleen", "jSmovie"], &["Aged Friends"],
&["Friday show", "Monday mix"])`.

| Case | Owner file | Required proof |
|---|---|---|
| R87-5-01 | `search_field_values.rs` | `matching(Artist, "hal")` of Library H gives `Haleen`, `Haleen Moss`, then `Shaleen`. The two prefix matches come before the contains match. `matching(Title, "hal")` gives no entry. Twelve artist texts give at most 8 entries |
| R87-5-02 | `search_field_values.rs` | `from_texts(&["Haleen", "HALEEN"], ..)` gives one artist entry with the text `HALEEN`. `from_library` of a tree with one track reads `artist_name`, `album_artist_name`, `album_title` and `feed_title`. It reads the name of each playlist |
| R87-5-03 | `search_field_list.rs` | `artist:hal` at cursor 10 with Library H and `Loaded { key: "hal", names: ["Halo Child"] }` gives three `Library` rows: `Haleen`, `Haleen Moss` and `Shaleen`. Then it gives the `Index` row `Halo Child`. The marks give "Library" and "MusicIndex" |
| R87-5-04 | `search_field_list.rs` | With Library H and `Loaded { key: "hal", names: ["haleen", "Halo Child"] }`, the name `haleen` gives no `Index` row. The list has one row of the key `haleen`, with the source `Library` |
| R87-5-05 | `search_field_values.rs` | `loaded("hal", rows)` keeps a `name` row "Haleen", a confirmed artist `publisher` row "Halo Child" and an unverified artist row. It drops an `Unknown` row, a `publisher` row of another role and a row "Hamster". Two rows "Haleen" give one name. Twelve usable rows give 8 names |
| R87-5-06 | `search_field_list.rs` | `release:ag` gives one `Library` row `Aged Friends`. `playlist:fri` gives `Friday show`. `artist_value_request_key` of the two texts gives `None` |
| R87-5-07 | `search_query.rs` | `quote_field_value` of `jsmovie` gives `jsmovie`. Of `aged friends` it gives `"aged friends"`. Of `a "b" \c` it gives `"a \"b\" \\c"`. For each of the three texts, `SearchQuery::parse` of `artist:` and the result gives `FieldValue::Text` of the text |
| R87-5-08 | `search_query.rs` | `unquote_field_value_prefix` of `"aged fr` gives `aged fr`. Of `"a \"b` it gives `a "b`. Of `hal` it gives `hal` |
| R87-5-09 | `search_field_values.rs` | `artist_value_request_key` gives `Some("hal")` for `artist:Hal` at cursor 10 and `Some("haleen m")` for `artist:"Haleen  M`. It gives `None` for `artist:`, `title:hal`, `in:library artist:hal` and `playlist:x artist:hal`. It gives `Some` for `in:index artist:hal` |
| R87-5-10 | `search_field_list.rs` | `Tab` on the row `Haleen Moss` in `artist:hal` gives `artist:"Haleen Moss" ` with the cursor after the space. The row action is `InsertText { field: Artist, text: "Haleen Moss" }`. In `artist:hal kind:single` at cursor 10, `Tab` on `Haleen` gives `artist:Haleen kind:single` with no new space |
| R87-5-11 | `search_field_list.rs` | `artist:hal` with `Loading { key: "hal" }` gives the three Library rows, then one `Loading` row. That row is `Unavailable(Loading)`, has the action `Inert` and the label "Loading names from MusicIndex". `Down` from the third row selects the first row |
| R87-5-12 | `search_field_list.rs` | `artist:hal` with `Failed { key: "hal" }` gives the three Library rows and one `IndexError` row. Its label is "Names from MusicIndex did not load". Its note is "The Library names show. The next change of the text tries again." |
| R87-5-13 | `search_field_list.rs` | `artist:halo` at cursor 11 with `Loaded { key: "hal", .. }` gives no `Index` row and no `Loading` row. `artist:` at cursor 7 with Library H gives four `Library` rows and no `Loading` row |
| R87-5-14 | `search_field_values.rs` | These names give no entry and no name: a name with U+000A, U+0009 or U+202E, and a name of 201 characters. A name of 200 characters gives one entry. The name `<b>x</b>` gives one entry with the same text |
| R87-5-15 | `search_field_list.rs` | `in:index playlist:fri` gives the row `Friday show` as `Unavailable(ScopeConflict)`, with the note "Not available with in:index" |
| R87-5-16 | `name_lists.rs` | With the `serve` test server, `FetchArtistNameValues::new(endpoint, "hal")` sends one request to `/v1/artist-names` with `limit=20`, `include=link_facts` and `from=hal`, and decodes the rows. A body that does not decode gives an error |
| R87-5-17 | `request_profiles.rs` | `INDEX_FIELD_LIST_ARTIST_NAMES` has the path shape `ArtistNameList` and the include `link_facts`. `ALL` still holds 12 profiles |
| R87-5-18 | `library.rs` and `search_field_list.rs` (app) | After `replace_tree` with a new artist and after `replace_playlists` with a new playlist, `field_values()` holds the new text. A `gpui::TestAppContext` test of `SearchFieldListView` types `artist:ha`, then `artist:hal`, then `artist:ha`. `apply_index_names` with the generation of the first `ha` changes nothing. With the current generation, it gives `Loaded` |
| R87-5-19 | `tests/architecture_tests.rs` | The situational guard of "Required Changes" item 12 passes. It fails against a copy of `src/app/search_field_list.rs` that calls `cx.spawn(` |
| R87-5-20 | all | Each test of packets 001 and 004 passes, with `FieldValueRows::EMPTY` in the packet 004 tests. `adr_0085_feed_page_guard_no_interface_text_names_a_feed_publisher` and guard R87-1-30 pass |

## Visual Acceptance Criteria

These are for the operator. No test proves them. The checks need a reachable MusicIndex endpoint
and a Library with artists and playlists.

- V1: type `artist:` in the toolbar search field. The list shows Library artist names, each with
  the mark "Library".
- V2: type one letter after `artist:`, then more letters. The Library names change at each letter.
  A row "Loading names from MusicIndex" shows. After the pause, MusicIndex names show with the
  mark "MusicIndex". No old name stays after a letter that it does not match.
- V3: a name that the Library and MusicIndex both hold shows one time, with the mark "Library".
  No row or mark uses the word "publisher".
- V4: type `playlist:` and a part of a playlist name. The matching playlist names show and update
  while you type. Type `release:` and a part of a release title. The matching titles show.
- V5: select a name with a space with Down, then press Tab. The text gets the name in double
  quotes and one space. The caret is after the space, and the focus stays in the field.
- V6: set an endpoint that cannot be reached in Settings, then type `artist:` and two letters. The
  error row "Names from MusicIndex did not load" shows, and the Library names stay. Set the
  correct endpoint again after the check.
- V7: the marks, the loading row and the error row are clear in Light and in Dark, at a normal and
  a narrow window width. No text is clipped. A long name wraps.

## Exclusions

- No change to the fixed values of `kind:`, `in:`, `is:` and `has:` (packet 004 D5).
- No values for `title:`, `after:`, `before:` and `duration:`.
- No change to the search run, to the Library search or to the Index search (packets 002 and
  003).
- No new pause token and no new generation type. Packet 003 owns them.
- No SQL query and no change to `src/db.rs`.
- No change to `index_name_row_exists`, `INDEX_ARTIST_NAME_CHECK` or `INDEX_ARTIST_NAME_LIST`.
- No retry button in the error row.
- No change to the stored MusicIndex contract.

## Files To Inspect

- [AGENTS.md](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- [ADR 0087](../adr/0087-search-fields-and-live-results.md), [ADR 0085](../adr/0085-artists-by-confirmation.md)
  and the [ADR 0087 phase plan](../plans/adr-0087-search-fields-phase-plan.md).
- Packets 001 to 004 and their "Implementation Result" sections.
- `src/view_models/search_query.rs`, `src/view_models/search_field_list.rs`,
  `src/app/search_field_list.rs` and `src/ui/composites/search_field_list.rs`.
- `src/view_models/index_name_list.rs` (`name_sort_key`) and `src/view_models/artists_everything.rs`
  (`classify_index_artist`, `is_shown`).
- `src/view_models/library.rs` (`LibraryTree`, `LibrarySnapshot`, `replace_tree`,
  `replace_playlists`) and `src/library/app_impl.rs` (`artist_search_facts`).
- `src/api.rs` (`ArtistNameRow`, `fetch_artist_names`), `src/application/queries/name_lists.rs`
  (`FetchArtistNamePage`, `serve`) and `src/application/request_profiles.rs`.
- `src/application/queries/search.rs` (`index_name_row_exists`), to read only.
- `src/presentation/async_command_presenter.rs` and `src/app/search_dispatch.rs`
  (`dispatch_index_search`), for the dispatch pattern.
- `tests/fixtures/musicindex-openapi-0.13.0.json`, path `/v1/artist-names`.
- `tests/architecture_tests.rs`: the guard of packet 004, guard R87-1-30 and the ADR 0085 guard.

## Checks

```bash
cargo test --lib adr_0087_field_values_
cargo test --lib adr_0087_field_list_
cargo test --lib adr_0087_query_parser_
cargo check --all-targets
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --test architecture_tests
touch src/main.rs && cargo build --bin v4vmm
```

Run `cargo build --bin v4vmm` last. `cargo test` can leave a binary with GPUI test support at
`target/debug/v4vmm`.

## Rollback

Revert the commit. This packet adds no migration, no stored data and no configuration key. The
field list then shows the fixed values of packet 004 only.

## Responsiveness Concerns

The orchestrator adds these concerns to "Throughout: Responsiveness And Background Feedback" of
the overhaul plan. Item 24 already names the `GET /v1/artist-names` request of each pause. This
packet does not fix the concerns. No measurement confirms a delay.

- 2026-10-23, `src/view_models/library.rs`: `replace_tree` builds `LibraryFieldValues` on the main
  thread. It walks each Library track one time at each Library load.
- 2026-10-23, `src/view_models/search_field_list.rs`: `FieldListVm::build` matches the Library
  values on the main thread at each draw of the list. The work grows with the count of distinct
  names.
- 2026-10-23, `src/app/search_field_list.rs`: on a slow MusicIndex node, the loading row stays
  until the answer comes. The row shows no progress.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0087-task-005-field-list-values.md`
- ADR 0087 Decisions 3 and 4, its invariants and operator answer 1
- ADR 0085 Decisions 1 and 2
- Packets 001, 002, 003 and 004, with their "Implementation Result" sections
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes".

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use
  the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Follow each decision in "Orchestrator Decisions". Do not change a decision.
- Use each name of "Names For Later Packets" exactly. Use the names of packets 001 to 004 exactly.
- Use the pause and the run generation of packet 003. Do not make a second timer or counter.
- Read field syntax only through `src/view_models/search_query.rs`. Do not write a production
  string literal that starts with a field token.
- The view model gives each text, label, mark, id and availability. The composite only composes.
- Use named tokens for size, spacing, color and typography. Column text does not call
  `truncate()`.
- Send the request only through `present_command`. Do not call `cx.spawn` from a screen.
- Treat each MusicIndex name and each Library text as untrusted input.
- Tests use fixed text and the local `serve` server. No test sends a request to a live service.
- Never run `git add`, `git rm`, `git commit`, `git checkout`, `git restore`, `git stash` or
  `git reset`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- `src/db.rs` and each file under `src/db/`.
- `src/application/queries/search.rs`, `src/app/search_dispatch.rs` and
  `src/app/saved_search_dispatch.rs`.
- `src/api.rs` and the stored MusicIndex contract.
- The key bindings of `src/app/keyboard.rs`.
- gpui-kit, gpui-component and `Cargo.toml`.
- Any ADR, and each document other than this packet and `docs/architecture/source-map.md`.

Acceptance criteria:
- Each case R87-5-01 to R87-5-20 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - <date>" section and an "Operator Visual Check"
  section for V1 to V7.

At the end, report: files changed, tests run, behavior changed, deviations, unresolved concerns,
and the document changes that the orchestrator must make.

Stop and report the problem, and do not guess, when:
- Packets 003 or 004 are not complete. Packet 004 is not complete when
  `src/view_models/search_field_list.rs` or `src/app/search_field_list.rs` is absent, or when a
  name of its "Names For Later Packets" is absent.
- Packet 001 is not complete, or `cursor_term` or `CursorPartial::FieldValue` is absent.
- Packet 003 gives no pause and no run generation that `SearchFieldListView` can use for its own
  request. Do not make a timer or a counter in its place.
- The `value_range` of a quoted value does not start at its opening quote. Or the `prefix` of a
  quoted value does not start with the quote.
- `fetch_artist_names` does not send `from` with the profile `INDEX_FIELD_LIST_ARTIST_NAMES`.
- A `publisher` row with `include=link_facts` has no link facts in the `serve` test answer that
  the contract gives.
- A change needs a new architecture decision or a file in "Do not touch".
- A recorded fact of this packet is not true in the working tree, and packets 001 to 004 did not
  cause the difference.
