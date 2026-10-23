# ADR 0087 Task 002: The Library Search Applies The Fields, And The Index Gets The Words Only

Status: Ready - 2026-10-23.

Orchestrator note - 2026-10-23: ADR 0086 packet 005 is implemented. It changed the saved search rows before this packet. Keep these names and their callers:

- `SearchResultsInspectorPageVm::{with_library_rows_partial, release_rows_complete, visible_items, window_group_for_row}`, `IndexSearchResultRows::feed_rows_partial` and `IndexFeedSearchRows::has_more`.
- `FeedResultDisplay::{artist_text, with_artist_text}`, and the `SearchLibraryLookup` methods `release_window_facts`, `window_key_for_name`, `window_feed_title` and `window_length`.
- `search_results_detail_for_query` sets `with_library_rows_partial(len >= DEFAULT_LOCAL_LIBRARY_SEARCH_LIMIT)`. `search_results_detail_for_run` must keep this call.
- Guard R86-5-24 requires one `client.search(` call and no `.cursor` in `fetch_index_feed_result_rows`.

## Goal

The Library search and the Index search read the typed query of packet 001, never the text.

The Library search applies each field of ADR 0087 Decision 2 in one SQL statement. Each field has
its own query builder. Each value goes into SQL only as a bound parameter, with its `LIKE`
characters escaped. Words and phrases keep the `LIKE` match of today on the five columns. The
50-track limit stays.

The Index search sends only `SearchQuery::index_q()` in `q`. A query with an Index filter term sends
no Index request. The page then shows the note of Decision 5, which names the field. A query with
a Library-only field sends no Index request. A query whose Index part has only words keeps the
three requests of today.

A saved search parses its text at each run, through the same typed path (Decision 6). The search
still runs only on Enter. Packet 003 adds the live runs.

## Authority

- [ADR 0087](../adr/0087-search-fields-and-live-results.md) Decision 1: the search runs read the
  typed query. A field value that is not correct applies nothing.
- ADR 0087 Decision 2: the fields, their Library meaning and the scope rules. Operator answer 2:
  `title:` matches the track title and the release title. `release:` matches the release title
  only.
- ADR 0087 Decision 5: `q` holds the words and the phrases. Until Stophammer supports a filter, a
  query with that field shows no Index rows, and the page names the field.
- ADR 0087 Decision 6: a saved search keeps its text, and each run parses it again.
- ADR 0087 invariants: one parser, bound Library values with escaped `LIKE` characters, no field
  syntax in `q`, and Index rows only when Stophammer applies each Index field.
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 5: the release kind.
  [ADR 0086](../adr/0086-new-releases-singles-window.md) Decision 1: the release date of a Library
  release.
- ADR 0059: the show readiness. ADR 0076: the "Removed from feed" mark and the stored payment
  route.
- [Packet 001](archive/adr-0087-task-001-query-parser.md) and its "Names For Later Packets", decisions D3,
  D8 and D9.
- [ADR 0086 packet 005](adr-0086-task-005-saved-search-window-groups.md): it changes the same
  search functions before this packet (operator answer 3 of ADR 0087).
- The [ADR 0087 phase plan](../plans/adr-0087-search-fields-phase-plan.md), packet 002.
- [AGENTS.md](../../AGENTS.md): renderer portability, typed action state, "Current-view state must
  update in place", "A guard names its class and its ADR" and "Collect responsiveness concerns".

## Recorded Facts - 2026-10-23

The orchestrator read each fact in the working tree on 2026-10-23. Packet 001 and ADR 0086 packets
002 to 005 are not implemented. The last subsection lists the facts that come from the packet 001
design and are not in the code today.

### The Library search today

- `src/db.rs:561-601` `search_library_tracks(conn, query: &str, limit)` trims the text. It binds
  one pattern `?1` with `ESCAPE '\\'` against `t.track_title`, `t.artist_name`, `t.album_title`,
  `t.album_artist_name` and `f.title`. It binds `LIMIT ?2`. It reads only rows with
  `t.is_in_library = 1`.
- `src/db.rs:1350-1364` `like_contains_pattern` escapes `%`, `_` and `\` and adds `%` at the two
  ends. It is private to `src/db.rs`.
- `src/db.rs:1-9`: `src/db.rs` imports no `crate::view_models` item.
- `src/db.rs:1530` already binds a JSON array through `json_each(?6)`. `Cargo.toml:11` builds
  `rusqlite` 0.38 with `bundled`, `backup` and `hooks`. It has no `functions` feature.
- `src/db.rs:3834` stores `tracks.pub_date` as `TEXT`. `src/db.rs:1319` and `:1332-1340` parse it
  as an RFC 2822 date into `TrackRow::pub_date: Option<i64>`. Thus SQL cannot compare the date.
- `src/db.rs:3843` `tracks.duration_seconds` is an `INTEGER`. `src/db.rs:3869-3884` hold
  `playlists(id, name)` and `playlist_tracks(playlist_id, track_id, position)`.
- `src/db/rss_field_holds.rs:144` adds `tracks.removed_from_feed_at`.
  `src/db/payment_routes.rs:80-81` adds `payment_routes_json` to `tracks` and to `feeds`.
  `src/db/payment_routes.rs:160-185` `stored_route` reads the track column, else the feed column.
  The value is a JSON array, and `[]` means no recipient.
- `src/library_service.rs:35-41` `search_library_tracks` calls `db::search_library_tracks`.
- `src/application/queries/search.rs:29` `DEFAULT_LOCAL_LIBRARY_SEARCH_LIMIT` is 50.
  `:191-206` `search_local_library_tracks(conn, query: &str, limit)` calls
  `normalized_global_search_query` (`:209-216`), which needs one alphanumeric character.
- Other callers of `search_local_library_tracks` are tests: `src/application/queries/search.rs:956`,
  `:983`, `:1011`, `:1017` and `:1042`, `src/rss/subscribe.rs:803` and
  `src/application/async_command_runner.rs:436`.

### The Library facts that SQL does not hold

- `src/view_models/library.rs:5658-5663` `AlbumNode::release_kind` resolves the stated kind of
  `metadata_facts` and the count of the album tracks. `src/application/queries/library.rs:423-427`
  loads `metadata_facts` from `local_metadata::feed_facts` when the tree loads.
- `src/application/queries/library.rs:76-90` builds the Library tree from `library_service::
  library_tracks`. `src/db.rs:536-559` reads only `t.is_in_library = 1`. Thus each tree track is a
  Library track, and the tree and the search read the same set of tracks.
- `src/view_models/singles_group.rs:405` gives the date of a Library single as the latest
  `pub_date` of the album tracks. No `AlbumNode` function gives this date today.
- `src/view_models/library.rs:5569-5577` and `:2121-2129` count `track.is_in_library` over
  `album.tracks`. Each tree track is in the Library, so a Library release row never shows
  `Partial` today.
- `src/application/queries/broadcast.rs:221-305` `readiness_track` reads the tags of each file
  (`read_audio_tags`, line 297). `src/library/app_impl.rs:991-1020` makes the report at load,
  after a feed check, after a repair and after a download or a removal. `src/view_models/library.rs:3081-3098`
  keeps only the problem count and the problem rows. It keeps no set of ready tracks.
- `src/library/app_impl.rs:767-785` `LibraryApp::artist_search_facts` builds the search facts of
  the artist rows from the tree.

### The Index search today

- `src/application/queries/search.rs:31-55` `FetchIndexSearchResults::new(endpoint, query, matches)`
  takes the text. `:226-276` `fetch_index_search_result_rows` sends it to the three
  `GET /v1/search` requests.
- `src/application/queries/search.rs:336-363` `index_artist_rows` sends one
  `GET /v1/artist-names` check (`index_name_row_exists`, `:170-182`) for each candidate name with
  no single feed match. Thus one search sends three requests and one more for each such name.
- `src/application/queries/search.rs:1283-1330` holds the test `Fixture`, and the tests call
  `serve(n, handler)` and read each request path (for example `:1921-1953`).

### The search pages today

- `src/app.rs:604-614` runs `submit_global_search` only on `InputEvent::PressEnter`.
- `src/app/search_dispatch.rs:223-266` `submit_global_search` and
  `open_search_results_in_content_list` trim the text and return only when it is empty. The
  restore of the new entry starts the Index search (`src/app.rs:832-850`).
- `src/app/search_dispatch.rs:269-286` `search_results_detail_for_query` runs the Library search on
  the main thread, with the database lock held.
- `src/app/search_dispatch.rs:288-329` `start_index_search_for_query`, `:331-361`
  `retry_index_search` and `:364-447` `dispatch_index_search` serve the toolbar page.
  `src/app/search_dispatch.rs:472-494` `sync_search_results_detail_with_nav` rebuilds the page on
  Back and Forward.
- `src/app/saved_search_dispatch.rs:86-150` `run_saved_search` calls
  `search_results_detail_for_query(&search.query)` and `FetchIndexSearchResults::new` with
  `search.query`. It has its own stale check, `saved_search_result_is_current`.
- `src/view_models/search_results/saved_search.rs:22-44` `SavedSearch` holds the trimmed text.
  `src/db/saved_searches.rs` stores the text in migration 21.
- `src/view_models/search_results/mod.rs:58-78` `SearchResultsInspectorPageVm` holds
  `index_loading`, `index_error` and `empty_state`. `:337-342` `index_notice` gives `index_error`
  when the tab has rows. `:565-579` `refresh_empty_state` gives `index_error` or
  `empty_state_for` when the tab is empty.
- `src/ui/shells/search_results_inspector.rs:186-190` renders `empty_state()` or the rows with
  `index_notice()`. `:460-470` renders the notice after the rows. Thus a new note needs no renderer
  change when it comes through these two methods.
- `src/view_models/search_results/results.rs:448-490` `local_library_result_rows` builds the three
  tabs from one track list. `src/view_models/search_results/saved_search.rs:363` calls
  `from_local_library_tracks` only in a test.

### The guards that pin the text of today

- `tests/architecture_tests.rs:1841` `adr_0066_missing_runtime_has_no_implicit_runner` (line 1903).
- `tests/architecture_tests.rs:2918` `adr_0084_search_list_submit_restores_through_the_back_path`
  uses `"fn search_results_detail_for_query("` as the end of its source range (line 2923).
- `tests/architecture_tests.rs:4002` `adr_0047_task_015_search_submit_commands` (lines 4044-4048).
- `tests/architecture_tests.rs:5787` `global_search_contract_has_toolbar_vm_and_local_query_boundary`
  (lines 5838-5873).
- `tests/architecture_tests.rs:11541` `adr_0048_index_search_is_async_and_vm_owned` (line 11562).
- `tests/architecture_tests.rs:21880-21911` guard R15-09 rejects `ReleaseKind::Single` outside
  `release_kind.rs` and `singles_group.rs`.

### Facts from the packet 001 design

These facts are not in the code today. Packet 001 adds them in
`src/view_models/search_query.rs`.

- `SearchQuery::parse`, `terms`, `scope`, `index_q`, `index_filter_terms` and
  `has_runnable_content`, with the exact signatures of packet 001 "Names For Later Packets".
- `QueryTerm`, `TermKind`, `FieldTerm`, `FieldValueState`, `FieldValue`, `KindValue`,
  `LibraryStateValue`, `ContentValue`, `DateValue` with `start_unix` and `end_unix`, and
  `DurationRange` with inclusive `min_secs` and `max_secs`.
- `QueryScope::{Everything, LibraryOnly, IndexOnly, Conflict}`. A Library-only field with `-` also
  gives `LibraryOnly` (D8).
- `index_filter_terms` gives each `Valid` field other than `in:`, and each excluded word or phrase
  (D9).
- `QueryFieldName::info(self).token` gives the field token, for example `"after:"`.
- Repeated fields all apply (D3). Ranges are byte ranges (D13).

## Design

### The run of one query

`SearchRun::new(text)` trims the text and parses it one time. It also computes the Index plan. Each
function that receives a query text builds one `SearchRun`. The functions are the toolbar submit,
the restore of a `Search` entry, Back and Forward, the retry and the saved search run.

A run is runnable when `SearchQuery::has_runnable_content` is `true`. A run that is not runnable
opens no page and sends nothing. This check replaces the alphanumeric check of
`normalized_global_search_query`, which this packet deletes.

The scope of the typed query decides the parts that run:

| `QueryScope` | Library part | Index part |
|---|---|---|
| `Everything` | Runs | Runs when the plan is `Send` |
| `LibraryOnly` | Runs | No request |
| `IndexOnly` | No SQL. No Library rows | Runs when the plan is `Send` |
| `Conflict` | No SQL. No Library rows | No request |

### The Library query

The application layer converts the typed query into one `db::LibraryTrackSearch`. The database
layer has its own typed clause, because `src/db.rs` imports no view model item (D2).

Each term of the query gives at most one `LibraryTrackClause`. A term without `-` adds its
condition with `AND`. A term with `-` adds `AND NOT (condition)`. Each condition is safe for
`NULL`, so a row with an empty column passes an exclusion (D13).

| Term | Clause | SQL condition |
|---|---|---|
| Word or phrase | `AnyText(text)` | One pattern against the five columns of today, joined with `OR` |
| `artist:` | `Artist(text)` | `t.artist_name` or `t.album_artist_name` |
| `title:` | `Title(text)` | `t.track_title`, `t.album_title` or `f.title` |
| `release:` | `Release(text)` | `t.album_title` or `f.title` |
| `kind:` | `TrackIds(ids)` | `t.id IN (SELECT value FROM json_each(?n))` |
| `after:`, `before:` | `TrackIds(ids)` | The same, with the union of the two date sets (D6) |
| `duration:` | `Duration { min_secs, max_secs }` | `t.duration_seconds IS NOT NULL`, with a bound minimum and a bound maximum when present |
| `is:downloaded` | `Downloaded` | No counted track of the same feed is missing (D8) |
| `is:partial` | `Partial` | One or more counted tracks of the feed are missing, and one or more are present (D8) |
| `is:removed` | `Removed` | `t.removed_from_feed_at IS NOT NULL` |
| `is:ready` | `TrackIds(ids)` | The ready set of the last readiness report (D10) |
| `playlist:` | `Playlist(text)` | `EXISTS` a `playlist_tracks` row of the track whose playlist name matches the pattern |
| `has:split` | `HasSplit` | The stored route of the track, else of its feed, is a JSON array with one or more entries (D12) |
| `has:image` | `HasImage` | `t.track_image_href` is not empty and is not the release image (D12) |
| `in:` | None | The scope only |
| `Incomplete` or `Invalid` field | None | Applies nothing (D14) |

Each text value goes through `like_contains_pattern` and binds as `?n` with `ESCAPE '\'`. Each
number and each JSON array of ids binds as `?n` too. The SQL text holds placeholders only. The
`ORDER BY` of today stays. `LIMIT` binds as the last parameter.

`library_track_search_sql(search, limit)` builds the SQL text and the parameter list. One private
function for each `LibraryClauseKind` gives its condition. `search_library_tracks` calls the
builder, then prepares and runs the statement. The module is `src/db/library_search.rs`, a child
of `src/db.rs`, so it can call the private `track_row_from_sql` and `like_contains_pattern`.

### The Library facts from the tree

`LibrarySearchFacts::from_tree(tree, ready)` gives one `LibraryTrackSearchFacts` for each track
of each `AlbumNode`:

- `release_kind`: `AlbumNode::release_kind()`.
- `release_date`: `AlbumNode::release_date()`, the latest `pub_date` of the album tracks (ADR 0086
  Decision 1).
- `pub_date`: `TrackRow::pub_date`.

`ready` is the set of track ids with the state `Ready` in the last readiness report. The Library
view model keeps it (D10). `LibraryViewModel::library_search_facts()` builds the facts for one run.

`library_track_search(query, facts)` converts the typed query:

- `kind:` gives the ids whose `release_kind` is one of the kinds. `ReleaseKind::from_query_kind`
  maps each `KindValue` in `release_kind.rs` (packet 001 D2).
- `is:ready` gives the ids of the ready set. With no report, the set is empty.
- The date terms give two sets. `release_track_ids` holds the ids whose `release_date` passes
  each date term. `track_track_ids` holds the ids whose `pub_date` passes each date term. A missing
  date does not pass a term without `-`, and passes a term with `-`.

`after:` passes a date at or after `DateValue::start_unix`. `before:` passes a date at or before
`DateValue::end_unix`. A query with date terms gives one `TrackIds` clause with the union of the
two sets. After SQL, `search_local_library_tracks` splits the rows:

- `releases`: the rows in `release_track_ids`. The Releases tab and the Artists tab read them.
- `tracks`: the rows in `track_track_ids`. The Tracks tab reads them.

Without a date term, `releases` and `tracks` hold the same rows. `matched_count` is the row count
of SQL before the split. ADR 0086 packet 005 reads it for `with_library_rows_partial`.

### The Index plan

`IndexSearchPlan::for_query(query)` gives one of two values:

- `Send(IndexSearchRequest { q })` when the scope is `Everything` or `IndexOnly`,
  `index_filter_terms` is empty and `index_q` is `Some`.
- `Skip(IndexSkip)` in each other case:
  - `Conflict` for the scope `Conflict`.
  - `LibraryOnly` for the scope `LibraryOnly`.
  - `Unsupported(UnsupportedIndexTerms)` when `index_filter_terms` is not empty. It holds each
    distinct field token in text order, and `excludes_words` for an excluded word or phrase.
  - `NoIndexPart` when `index_q` is `None` and no other case applies.

Only `IndexSearchPlan::for_query` builds an `IndexSearchRequest`. `FetchIndexSearchResults::new`
takes the request, not a text. Thus no code path can send the query text in `q`. Packet 006 adds
the filter fields to `IndexSearchRequest` and changes the `Unsupported` rule.

### The note of Decision 5

`SearchResultsInspectorPageVm` owns the note. `set_index_skipped(&skip)` sets `index_loading`
to `false`, clears `index_error` and keeps the skip. `index_notice()` and `refresh_empty_state()`
read it. The shell does not change.

| Skip | Rows on the tab | Empty tab |
|---|---|---|
| `Unsupported` | Title from `note_title`, text "The Library rows show." | Title from `note_title`, text of `empty_state_for` |
| `Conflict` | Not possible: the page has no rows | Title "No results", text "The search asks for the Library only and for MusicIndex only. Remove one of the fields." |
| `LibraryOnly` | No note | The empty state of today |
| `NoIndexPart` | No note | The empty state of today |

`UnsupportedIndexTerms::note_title` gives these texts:

| Terms | Title |
|---|---|
| One field token | "MusicIndex cannot filter by after: yet" |
| Two field tokens | "MusicIndex cannot filter by after: and kind: yet" |
| Three or more | "MusicIndex cannot filter by after:, kind: and duration: yet" |
| Excluded words only | "MusicIndex cannot exclude words yet" |
| Field tokens and excluded words | "MusicIndex cannot filter by after: or exclude words yet" |

With two or more tokens and excluded words, the list of tokens follows the same rule, for example
"MusicIndex cannot filter by after: and kind: or exclude words yet".

### The two search pages

`TopApp::search_results_detail_for_run(run)` replaces `search_results_detail_for_query`. It builds
the Library facts and runs the Library part. It builds the page with
`SearchResultsInspectorPageVm::from_library_search` and sets the skip when the plan is `Skip`.

`TopApp::start_index_search_for_run(slot, run)` replaces `start_index_search_for_query`. The toolbar
page and the saved search page both call it:

1. When the plan is `Skip`, it returns. It sends nothing, and it shows no availability error.
2. Else it checks the availability, as today. The toolbar slot keeps its retained action. The saved
   search slot keeps none, as today.
3. It marks the page of `slot` as loading and dispatches `FetchIndexSearchResults` with the request.
4. The success and the failure keep the stale check of each slot. The toolbar slot uses
   `content_list_nav_matches_search(run.text())`. The saved search slot uses
   `saved_search_result_is_current(id, run.text())`, with the id of the mounted page.

`retry_index_search` builds a `SearchRun` from the stored text. When its plan is `Skip`, it
finishes the intent with the text "App did not send the MusicIndex search again. The search has no
MusicIndex part." and sends nothing.

`run_saved_search` builds one `SearchRun` from `search.query`. It calls
`search_results_detail_for_run` and `start_index_search_for_run(SearchPageSlot::SavedSearch, ..)`.
The saved text, the restore and the "Saved" compare do not change (D20).

### Names For Later Packets

Packets 003, 004, 005 and 006 use these exact names. A later packet adds a name only through its
own packet text.

| Name | Module | Packets |
|---|---|---|
| `pub(crate) struct SearchRun`, with `fn new(text: &str) -> SearchRun`, `fn text(&self) -> &str`, `fn query(&self) -> &SearchQuery`, `fn index_plan(&self) -> &IndexSearchPlan`, `fn is_runnable(&self) -> bool` and `fn runs_library(&self) -> bool` | `src/view_models/search_results/search_run.rs` | 003 adds `generation` |
| `pub(crate) enum IndexSearchPlan { Send(IndexSearchRequest), Skip(IndexSkip) }`, with `fn for_query(query: &SearchQuery) -> IndexSearchPlan` | same | 003, 006 |
| `pub(crate) struct IndexSearchRequest { pub(crate) q: String }` | same | 006 adds the filter fields |
| `pub(crate) enum IndexSkip { NoIndexPart, LibraryOnly, Conflict, Unsupported(UnsupportedIndexTerms) }` | same | 006 |
| `pub(crate) struct UnsupportedIndexTerms { pub(crate) field_tokens: Vec<&'static str>, pub(crate) excludes_words: bool }`, with `fn note_title(&self) -> String` | same | 006 |
| `pub(crate) struct LibrarySearchRows { pub(crate) releases: Vec<TrackRow>, pub(crate) tracks: Vec<TrackRow>, pub(crate) matched_count: usize }` | same | 003 |
| `pub(crate) const INDEX_SKIP_ROWS_TEXT`, `SCOPE_CONFLICT_TITLE` and `SCOPE_CONFLICT_TEXT` | same | 004 |
| `SearchResultsInspectorPageVm::from_library_search(text, rows: &LibrarySearchRows, artist_facts: &ArtistSearchFacts) -> Self` | `src/view_models/search_results/mod.rs` | 003 |
| `SearchResultsInspectorPageVm::set_index_skipped(&mut self, skip: &IndexSkip)` and `fn index_skip(&self) -> Option<&IndexSkip>` | same | 003, 006 |
| `pub(crate) struct LibrarySearchFacts`, with `fn from_tree(tree: &LibraryTree, ready: Option<&BTreeSet<i64>>) -> Self` and `fn track(&self, id: i64) -> Option<&LibraryTrackSearchFacts>` | `src/view_models/library_search_facts.rs` | 003 |
| `pub(crate) struct LibraryTrackSearchFacts { release_kind: ReleaseKind, release_date: Option<i64>, pub_date: Option<i64> }` | same | none |
| `LibraryViewModel::library_search_facts(&self) -> LibrarySearchFacts`, and the field `ready_track_ids: Option<BTreeSet<i64>>` | `src/view_models/library.rs` | 003 |
| `AlbumNode::release_date(&self) -> Option<i64>` | same | none |
| `LibraryApp::library_search_facts(&self) -> LibrarySearchFacts` | `src/library/app_impl.rs` | 003 |
| `ReleaseKind::from_query_kind(kind: KindValue) -> ReleaseKind` | `src/view_models/release_kind.rs` | none |
| `pub struct LibraryTrackSearch { pub clauses: Vec<LibraryTrackClause> }` | `src/db/library_search.rs` | 006 |
| `pub struct LibraryTrackClause { pub excluded: bool, pub kind: LibraryClauseKind }` | same | none |
| `pub enum LibraryClauseKind { AnyText(String), Artist(String), Title(String), Release(String), Duration { min_secs: Option<u32>, max_secs: Option<u32> }, TrackIds(Vec<i64>), Playlist(String), Downloaded, Partial, Removed, HasSplit, HasImage }` | same | none |
| `pub fn search_library_tracks(conn: &Connection, search: &LibraryTrackSearch, limit: usize) -> Result<Vec<TrackRow>>` | same, re-exported from `src/db.rs` | none |
| `pub fn library_track_search_sql(search: &LibraryTrackSearch, limit: usize) -> (String, Vec<rusqlite::types::Value>)` | same | none |
| `library_service::search_library_tracks(conn, search: &LibraryTrackSearch, limit: usize)` | `src/library_service.rs` | none |
| `pub(crate) fn library_track_search(query: &SearchQuery, facts: &LibrarySearchFacts) -> LibrarySearchPlan` | `src/application/queries/search.rs` | 003 |
| `pub(crate) struct LibrarySearchPlan { search: LibraryTrackSearch, date_split: Option<DateSplit> }` and `pub(crate) struct DateSplit { release_track_ids: BTreeSet<i64>, track_track_ids: BTreeSet<i64> }` | same | none |
| `ApplicationQueryService::search_local_library_tracks(&self, conn: &Connection, query: &SearchQuery, facts: &LibrarySearchFacts, limit: Option<usize>) -> Result<LibrarySearchRows, CommandError>` | same | 003 |
| `FetchIndexSearchResults::new(endpoint, request: IndexSearchRequest, matches: NameFeedMatches)` | same | 003, 006 |
| `TopApp::search_results_detail_for_run(&self, run: &SearchRun, cx: &App) -> SearchResultsInspectorPageVm` | `src/app/search_dispatch.rs` | 003 |
| `TopApp::start_index_search_for_run(&mut self, slot: SearchPageSlot, run: &SearchRun, cx: &mut Context<Self>)` | same | 003 |

Packet 003 uses `search_results_detail_for_run` and `start_index_search_for_run` as the one
entry point of a run on the mounted page. It adds the generation to `SearchRun` and to the result
checks of `start_index_search_for_run`.

Packet 004 reads `SearchRun::query()` for the error texts of the field list. This packet shows no
field error on the page (D14).

Packet 005 uses no name of this packet. Its value queries escape `LIKE` characters through
`like_contains_pattern` (`src/db.rs:1350`) or through a prefix function beside it.

Packet 006 adds the filter fields to `IndexSearchRequest`, fills them in
`IndexSearchPlan::for_query`, and makes `Unsupported` hold only the terms that the new contract
cannot apply.

## Orchestrator Decisions

- **D1. Each term is one clause of one SQL statement.** A fact that SQL cannot compute becomes a
  bound set of track ids. Thus `LIMIT` stays in SQL and gives the first 50 rows that pass each
  term. A filter after SQL would give fewer than 50 rows when more rows match.
- **D2. The database layer has its own clause type.** `src/db.rs` imports no view model item
  today. The application layer converts the typed query, so the dependency direction stays the
  same.
- **D3. Each word and each phrase is one `AnyText` clause, and all clauses apply together.** A
  word matches the five columns of today. Thus `night drive` without quotes now matches two words
  in any column. Before this packet, it matched the one string "night drive". A phrase keeps the
  one-string match.
- **D4. The text fields use these columns.** `artist:` uses `t.artist_name` and
  `t.album_artist_name` (Decision 2). The release title is `t.album_title` or `f.title`, because
  both columns are in the five columns of today. `title:` adds `t.track_title` (operator answer 2).
- **D5. `kind:` reads the kind that the Library shows.** `AlbumNode::release_kind` reads a stated
  value of the metadata facts, which SQL cannot read simply. The ids come from the same tree, so
  the filter and the badge of a row agree. The map goes in `release_kind.rs`, because guard R15-09
  rejects `ReleaseKind::Single` in each other file.
- **D6. A release row reads the release date, and a track row reads its own `pub_date`.**
  `pub_date` is RFC 2822 text, so SQL cannot compare it, and the dates come from the tree. The two
  date sets go into SQL as one union, and the rows split after SQL. The limit counts the rows of
  the union. A tab thus can show fewer than 50 rows when the other date set holds more of the
  first 50 rows.
- **D7. `duration:` reads `t.duration_seconds`.** A track with no duration does not match
  `duration:`, and it passes `-duration:`.
- **D8. `is:downloaded` and `is:partial` read the stored tracks of the same feed.** The counted
  tracks of a feed are its tracks that are not removed from the feed, and each downloaded track. A
  track is downloaded when `is_in_library = 1` and a `local_files` row names it. The fixed value
  texts of packet 001 give this meaning.

  Flag: the release row icon counts only the tree tracks
  (`src/view_models/library.rs:5569-5577`). Each tree track is in the Library, so a row that
  `is:partial` finds shows the "Downloaded" icon. This packet does not change the icon. The
  orchestrator records the difference for the operator.
- **D9. `is:removed` reads `t.removed_from_feed_at IS NOT NULL`.** This is the mark of ADR 0076.
  A confirmed mark stays a mark, because the track is still removed from its feed.
- **D10. `is:ready` reads the last readiness report that the Library already makes.**
  `show_needs_attention_report` and `refresh_needs_attention_report` keep the ids with the state
  `Ready` in `ready_track_ids`. The search reads no file, so `is:ready` adds no file read to a run.
  The set changes at the same events as the Needs attention count. With no report, `is:ready`
  matches no track, and `-is:ready` excludes no track.
- **D11. `playlist:` matches a part of the playlist name, without ASCII case.** It uses the same
  `LIKE` pattern as the other text fields. Thus `playlist:friday` finds "Friday show".
- **D12. `has:split` and `has:image` are safe for `NULL` and for bad JSON.** `has:split` reads
  `COALESCE(t.payment_routes_json, f.payment_routes_json)`, the order of `stored_route`. It counts
  the array only when `json_valid` is true, because `json_array_length` fails on bad JSON. An
  empty array `[]` has no recipient, so it is not a split. `has:image` needs a track image that is
  not empty and is not the same as `f.album_image_href`.
- **D13. An exclusion is `NOT (condition)`, and each condition gives `TRUE` or `FALSE`, never
  `NULL`.** A `NULL` inside `NOT` removes the row, so each condition uses `COALESCE` or
  `IS NOT NULL`.
- **D14. A field with an error or an empty value applies nothing, and the page shows no field
  error.** ADR 0087 Decision 3 gives the error to the field list of packet 004. A second error
  text on the page then needs a removal in packet 004. The search does not fail, because SQL never
  sees such a field.
- **D15. The scope decides the parts that run.** `IndexOnly` and `Conflict` run no SQL.
  `LibraryOnly` and `Conflict` send no Index request. `LibraryOnly` shows no note, because the
  curator typed the Library-only field, and packet 004 marks it "Library only".
- **D16. Only `IndexSearchPlan::for_query` builds an `IndexSearchRequest`.** The type then proves
  the invariant "No Index request carries field syntax in `q`". The guard keeps the struct literal
  in its owner file.
- **D17. A skipped Index part is not an Index error.** `set_index_skipped` clears `index_loading`
  and `index_error`. `start_index_search_for_run` returns before the availability check, so an
  unavailable MusicIndex shows no error for a search that sends no request.
- **D18. The note names each field once, in text order.** The token comes from
  `QueryFieldName::info(self).token`, so the note holds no field literal of its own (packet 001
  guard).
- **D19. `has_runnable_content` replaces the alphanumeric check.** `normalized_global_search_query`
  has no other caller, so this packet deletes it. A text such as `!!!` or `-remix` now opens no
  page. Before this packet, `!!!` opened a page and sent the Index requests.
- **D20. A saved search keeps its text and parses it at each run.** The restore, the stale check
  and the "Saved" compare keep reading the text. Thus no migration is necessary. A saved text from
  before packet 001 that holds a colon keeps its meaning by the word rule of Decision 1.
- **D21. One Index entry serves both pages.** The two paths of today differ only in the stale
  check and in the retained action. One function with a `slot` gives packet 003 one place for the
  generation.
- **D22. ADR 0086 packet 005 reads `matched_count`.** That packet compares the Library row count
  with the limit. After the date split, the count of one tab can be lower than the SQL count. The
  SQL count is the correct test for a partial Library part.
- **D23. A skipped Index part counts as complete for ADR 0086 packet 005.** With no request, no
  later Index page can change a group. `release_rows_complete` then depends on the Library part
  only.
- **D24. This packet updates the five guards of "The guards that pin the text of today".** Each
  guard pins a text that this packet replaces. Each updated guard pins the typed path with the
  same intent.

## Required Changes

1. `src/view_models/release_kind.rs`: add `ReleaseKind::from_query_kind`, and a unit test for each
   `KindValue`.
2. `src/db/library_search.rs` (new): add the names of the `src/db/library_search.rs` rows of
   "Names For Later Packets". Start it with a `//!` comment that names ADR 0087 Decision 2 and its
   invariant on bound values, and `#![warn(clippy::pedantic)]`. Add one private condition function
   for each `LibraryClauseKind`.
3. `src/db.rs`: declare `mod library_search;` and re-export its public names. Delete the old
   `search_library_tracks`. Change the three tests at `src/db.rs:4790-4910` to the new signature.
4. `src/library_service.rs`: change `search_library_tracks` to take `&LibraryTrackSearch`.
5. `src/view_models/library_search_facts.rs` (new): add `LibrarySearchFacts` and
   `LibraryTrackSearchFacts`, with a `//!` comment that names ADR 0087 Decision 2 and ADR 0086
   Decision 1. Declare the module in `src/view_models/mod.rs`.
6. `src/view_models/library.rs`: add `AlbumNode::release_date`, the field `ready_track_ids`, its
   update in `show_needs_attention_report` and `refresh_needs_attention_report`, and
   `library_search_facts`. When ADR 0086 packet 002 or 005 added a function with the result of
   `release_date`, call that function and do not add a second one.
7. `src/library/app_impl.rs`: add `LibraryApp::library_search_facts`.
8. `src/view_models/search_results/search_run.rs` (new): add `SearchRun`, `IndexSearchPlan`,
   `IndexSearchRequest`, `IndexSkip`, `UnsupportedIndexTerms`, `LibrarySearchRows` and the three
   text constants. Declare the module in `src/view_models/search_results/mod.rs` and re-export the
   names.
9. `src/view_models/search_results/mod.rs`: add the field `index_skip`, `from_library_search`,
   `set_index_skipped` and `index_skip`. Change `index_notice` and `refresh_empty_state` to read
   the skip. Move `from_local_library_tracks` into the `#[cfg(test)]` impl block. It calls
   `from_library_search` with the same rows in both lists.
10. `src/view_models/search_results/results.rs`: `local_library_result_rows` takes the release
    rows and the track rows as two slices.
11. `src/application/queries/search.rs`: add `library_track_search`, `LibrarySearchPlan` and
    `DateSplit`. Change `search_local_library_tracks` and `FetchIndexSearchResults::new`. Delete
    `normalized_global_search_query`. Change the test callers at `:956`, `:983`, `:1011`, `:1017`
    and `:1042`.
12. `src/rss/subscribe.rs:803` and `src/application/async_command_runner.rs:436`: change the test
    callers to `SearchQuery::parse` and `LibrarySearchFacts::default()`.
13. `src/app/search_dispatch.rs`: replace `search_results_detail_for_query` with
    `search_results_detail_for_run`, and `start_index_search_for_query` with
    `start_index_search_for_run`. Change `submit_global_search`,
    `open_search_results_in_content_list`, `retry_index_search`, `dispatch_index_search` and
    `sync_search_results_detail_with_nav` to build one `SearchRun` each. Keep the call
    `self.restore_content_list_nav_entry(content_list_id, &FrameNavigationEntry::Search(query), cx)`.
    Keep the call of ADR 0086 packet 005 to `with_library_rows_partial`, with `matched_count`.
14. `src/app.rs`: `restore_content_list_nav_entry` calls `start_index_search_for_run` with
    `SearchPageSlot::ToolbarSearch` for a `Search` entry.
15. `src/app/saved_search_dispatch.rs`: `run_saved_search` builds one `SearchRun` and calls the two
    new functions. Delete its own `FetchIndexSearchResults` dispatch.
16. `tests/architecture_tests.rs`: update the five guards of D24. Add the guard of case R87-2-29.
17. `docs/architecture/source-map.md`: add a row for `src/db/library_search.rs` ("Library search
    query builder: one clause for each field, bound values, ADR 0087 Decision 2, packet 002"), for
    `src/view_models/library_search_facts.rs` and for `src/view_models/search_results/search_run.rs`.

## Mechanical Acceptance Criteria

Use the prefix `adr_0087_library_fields_` for each unit test. Each test is beside its owner code,
apart from R87-2-29. A "recorded database" is the in-memory database of `setup_test_db` in the
test module of the owner, with the rows that the test writes. No test sends a request to a live
service.

| Case | Owner | Required proof |
|---|---|---|
| R87-2-01 | `src/db/library_search.rs` | With a recorded database, an `AnyText("needle")` clause finds a track by each of the five columns, the same as the old test `search_library_tracks_matches_supported_fields` |
| R87-2-02 | same | Two `AnyText` clauses must both match. The phrase `night drive` finds "Night Drive" and does not find a track with "night" in the title and "drive" in the album |
| R87-2-03 | same | `Artist("jsmovie")` finds a track by `artist_name` and a track by `album_artist_name`. It does not find a track with "jsmovie" in the title only |
| R87-2-04 | same | `Title("remix")` finds a track by `track_title`, by `album_title` and by `f.title`. `Release("aged")` finds a track by `album_title` and by `f.title`, and not by `track_title` |
| R87-2-05 | `src/application/queries/search.rs` | With a recorded database and facts from `from_tree`, `kind:single` gives only the tracks of single releases. `kind:album,ep` gives the album and EP tracks. `-kind:single` gives each other track |
| R87-2-06 | `src/view_models/release_kind.rs` | `ReleaseKind::from_query_kind` gives `Album`, `Ep` and `Single` for the three `KindValue`s |
| R87-2-07 | `src/application/queries/search.rs` | An album has tracks on 2026-10-01 and 2026-10-20. `before:2026-10-10` gives the Oct 1 track in `tracks` and no row in `releases`. `after:2026-10-15` gives the two tracks in `releases` and the Oct 20 track in `tracks` |
| R87-2-08 | same | `after:2026-10` includes a track of 2026-10-01 00:00:00 UTC. `before:2026-10` includes a track of 2026-10-31 23:59:59 UTC. A track with no `pub_date` is not in `after:2026-01-01` and is in `-after:2026-01-01` |
| R87-2-09 | `src/db/library_search.rs` | `Duration { min_secs: Some(301), max_secs: None }` finds a 302-second track and not a 300-second track. A track with no duration is not in `duration:` and is in `-duration:` |
| R87-2-10 | same | Feed A has two Library tracks with files. Feed B has one Library track with a file and one track not in the Library. Feed C is the same as B, but its missing track has `removed_from_feed_at`. `Downloaded` finds A and C. `Partial` finds B only |
| R87-2-11 | same | `Removed` finds a track with `removed_from_feed_at`, also with `removed_from_feed_confirmed_at`. `-is:removed` finds each other track |
| R87-2-12 | `src/application/queries/search.rs` | With a ready set of one id, `is:ready` gives that track only. With no ready set, `is:ready` gives no track and `-is:ready` gives each matched track |
| R87-2-13 | `src/db/library_search.rs` | `Playlist("friday")` finds a track of the playlist "Friday show" and not a track of another playlist. The exclusion finds a track in no playlist |
| R87-2-14 | same | `HasSplit` finds a track with a route array of one entry, and a track with no track route whose feed route has one entry. It does not find `[]`, `NULL` or the bad JSON `{x`, and the bad JSON gives no error. `HasImage` finds a track image that differs from the release image only |
| R87-2-15 | same | `-artist:x` keeps a track whose `artist_name` and `album_artist_name` are `NULL` (D13) |
| R87-2-16 | same | `Artist("100%")`, `Artist("a_b")` and `Artist("a\b")` each find only the literal text. "1000", "aXb" and "ab" do not match |
| R87-2-17 | same | `library_track_search_sql` of a value `'; DROP TABLE tracks; --` gives SQL text without the value, and the value is in the parameter list. After the run, the table `tracks` still exists. The parameter count equals the count of `?` placeholders |
| R87-2-18 | `src/application/queries/search.rs` | 60 tracks match `needle`, and only the last 10 in the SQL order are singles. `needle kind:single` gives 10 rows. `needle` alone gives 50 rows and `matched_count` 50 |
| R87-2-19 | same | `haleen after:yesterday` and `haleen artist:` give the same rows as `haleen`. `-remix`, `!!!`, `in:library` and `after:yesterday` give no rows and run no SQL |
| R87-2-20 | same | `in:index haleen` and `in:index is:ready` give no rows and run no SQL, with a database that holds a matching track |
| R87-2-21 | `src/view_models/search_results/search_run.rs` | `IndexSearchPlan::for_query` gives `Skip(LibraryOnly)` for `haleen is:ready`, `haleen -has:split`, `haleen playlist:x` and `in:library haleen`. It gives `Skip(Conflict)` for `in:index is:ready` |
| R87-2-22 | same | It gives `Skip(Unsupported)` with `["after:"]` for `haleen after:2026-10-01`, with `["kind:", "after:"]` for `kind:single after:2026-10 kind:ep`, and with `excludes_words` for `haleen -remix`. `note_title` gives each text of "The note of Decision 5" |
| R87-2-23 | same, and `src/application/queries/search.rs` | `haleen "night drive"` gives `Send` with `q` equal to `index_q`. `FetchIndexSearchResults` with that request and `serve(3, ..)` sends three requests. `haleen in:index` gives `q` `haleen`. No request path holds `in%3A`, `artist%3A` or another field token |
| R87-2-24 | `src/view_models/search_results/mod.rs` | After `set_index_skipped(Unsupported)`, a tab with Library rows gives the note through `index_notice`, with "The Library rows show.". An empty tab gives the note title and the `empty_state_for` text through `empty_state`. `is_index_loading` is `false` |
| R87-2-25 | same | `set_index_skipped(Conflict)` gives the empty state "No results" with `SCOPE_CONFLICT_TEXT`. `LibraryOnly` and `NoIndexPart` give no `index_notice` and the empty state of today. A later `set_index_error` is not possible after a skip, because no request runs |
| R87-2-26 | same | `from_library_search` with different `releases` and `tracks` fills the Releases tab and the Artists tab from `releases`, and the Tracks tab from `tracks` |
| R87-2-27 | `src/view_models/library_search_facts.rs` | `from_tree` gives the stated kind when the metadata facts state one, else the count rule. `release_date` is the latest `pub_date` of the album. The ready set is the given set |
| R87-2-28 | `src/view_models/search_results/search_run.rs` | A saved search with the text `artist:jsmovie kind:single -title:remix` goes through `src/db/saved_searches.rs` save and load with a recorded database. `SearchRun::new` of the loaded text gives the same terms as `SearchQuery::parse` of the first text. Compare the `Debug` text when the types do not derive `PartialEq` |
| R87-2-29 | `tests/architecture_tests.rs` | The situational guard `adr_0087_library_fields_r87_2_29_typed_query_reaches_both_searches` passes. It fails against a copy of `src/app/saved_search_dispatch.rs` with `FetchIndexSearchResults::new(endpoint, search.query.clone(), matches)` |
| R87-2-30 | all | The five guards of D24 pass in their new form. Each test of ADR 0084 packets 010 and 012, of ADR 0086 packet 005 and of packet 001 still passes |

The guard of R87-2-29 enforces these rules:

- `src/view_models/search_results/search_run.rs` is the only production file that holds the
  struct literal `IndexSearchRequest {`.
- The production source of `FetchIndexSearchResults::new` in `src/application/queries/search.rs`
  holds `request: IndexSearchRequest`.
- No production file calls `library_service::search_library_tracks(` outside
  `src/application/queries/search.rs`. No production file calls `db::search_library_tracks(`
  outside `src/library_service.rs`.
- `src/db/library_search.rs` holds `like_contains_pattern(` and `ESCAPE '\\'`.
- No file under `src/app/` holds `normalized_global_search_query` or
  `search_results_detail_for_query`.

The failure message of the guard is this text:

```text
ADR 0087 Decision 5 and its invariants: <file> <violation>. The Index request takes an
IndexSearchRequest from IndexSearchPlan::for_query, never the query text. A Library value goes
into SQL only as a bound parameter through src/db/library_search.rs. Build one SearchRun and pass
its plan and its typed query.
```

## Visual Acceptance Criteria

These are for the operator. No test proves them. Examine each one in Light and in Dark, at normal
width and at narrow width.

- V1: search `haleen`. The rows are the same as before this packet, and the Index rows show after
  the loading rows.
- V2: search `haleen after:2026-10-01`. Only Library rows show. After the rows, the note
  "MusicIndex cannot filter by after: yet" shows with the text "The Library rows show.". No loading
  rows and no Index error show.
- V3: search a field query with no Library match, for example `zzzz kind:ep`. The empty state shows
  the note title and the text `No releases matched "zzzz kind:ep".`.
- V4: search `haleen -remix`. The note "MusicIndex cannot exclude words yet" shows.
- V5: search `haleen is:downloaded`. Only Library rows show, with no note and no loading rows.
- V6: search `in:index haleen`. Only Index rows show.
- V7: search `in:index is:ready`. The empty state "No results" shows with its text.
- V8: save the search `haleen kind:single`, then open it from Discover. It shows the same rows and
  the same note as the toolbar search.
- V9: at narrow width, the note text wraps inside the column. No text overlaps a row.

## Exclusions

- No field list, no field error on the page and no keys (packet 004).
- No live run, no pause and no generation (packet 003).
- No Index filter parameter and no change to the stored contract (packet 006).
- No change to the release row icon. D8 records the difference for the orchestrator.
- No change to the readiness report or to the time of its run (D10).
- No change to the merge of packet 010 or to the window groups of ADR 0086.
- No change to the renderer under `src/ui/`.
- No schema change, no migration and no configuration key.

## Files To Inspect

- [AGENTS.md](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- [ADR 0087](../adr/0087-search-fields-and-live-results.md), packet 001 and its
  "Implementation Result", and ADR 0086 packet 005 and its "Implementation Result".
- `src/view_models/search_query.rs` (from packet 001).
- `src/db.rs`: `search_library_tracks`, `like_contains_pattern`, `track_row_from_sql`,
  `library_tracks`, the `json_each` query near line 1530, and the search tests near line 4756.
- `src/db/payment_routes.rs` (`stored_route`), `src/db/rss_field_holds.rs` and
  `src/db/saved_searches.rs`.
- `src/library_service.rs`, `src/application/queries/search.rs`,
  `src/application/queries/library.rs` (`build_tree`) and `src/application/queries/broadcast.rs`.
- `src/view_models/library.rs`: `AlbumNode`, `show_needs_attention_report`,
  `refresh_needs_attention_report` and `LibrarySearchLookup`.
- `src/view_models/release_kind.rs` and `src/view_models/singles_group.rs` (line 405).
- `src/view_models/search_results/mod.rs`, `results.rs`, `empty_state.rs`, `saved_search.rs` and
  `tests.rs`.
- `src/app/search_dispatch.rs`, `src/app/saved_search_dispatch.rs`, `src/app/search_selection.rs`
  (`SearchPageSlot`) and `src/app.rs` (`restore_content_list_nav_entry`).
- `src/library/app_impl.rs` (`artist_search_facts`).
- `src/ui/shells/search_results_inspector.rs`, to read only.
- `tests/architecture_tests.rs`: the five guards of D24, guard R15-09 and the guard of packet 001.

## Checks

```bash
cargo test --lib adr_0087_library_fields_
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

Revert the commit. This packet adds no migration, no stored data and no configuration key. A
saved search keeps its text, so a revert keeps each saved search.

## Responsiveness Concerns

The orchestrator adds these concerns to "Throughout: Responsiveness And Background Feedback" of
the overhaul plan. This packet does not fix them.

- 2026-10-23, `src/app/search_dispatch.rs`: the Library search runs on the main thread with the
  database lock held. The new `EXISTS` conditions and the `json_each` id sets add time to each
  run. Packet 003 runs the search after each pause.
- 2026-10-23, `src/application/queries/search.rs`: one search sends one `GET /v1/artist-names`
  check for each candidate name with no single feed match, after the three search requests.
  ADR 0087 Decision 4 permits three Index requests for each run.
- 2026-10-23, `src/library/app_impl.rs`: the readiness report reads the tags of each Library file
  on the main thread. `is:ready` reads the kept result and adds no file read (D10).

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0087-task-002-library-fields-and-index-words.md`
- ADR 0087 Decisions 1, 2, 5 and 6 and its invariants
- Packet 001 and ADR 0086 packet 005, with their "Implementation Result" sections
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes".

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use
  the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Follow each decision in "Orchestrator Decisions". Do not change a decision.
- Use the names of packet 001 "Names For Later Packets" exactly. Use each name of this packet's
  "Names For Later Packets" exactly.
- Read field syntax only through `SearchQuery`. Do not write a string literal that starts with a
  field token in production code.
- Bind each value as a parameter. Do not put a value into the SQL text.
- The view model gives each text. Do not change a file under `src/ui/`.
- Tests use recorded values and recorded databases. No test sends a request to a live service.
- Background work runs through the ADR 0040 runtime. Do not call `cx.spawn` from a screen.
- Never run `git add`, `git rm`, `git commit`, `git checkout`, `git restore`, `git stash` or
  `git reset`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- `src/view_models/search_query.rs`, apart from a read.
- Each file under `src/ui/`.
- The database schema, the migration registry and `src/db/saved_searches.rs`, apart from a test.
- Any ADR, and each document other than this packet and `docs/architecture/source-map.md`.

Acceptance criteria:
- Each case R87-2-01 to R87-2-30 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - <date>" section and an "Operator Visual Check"
  section for V1 to V9.

At the end, report: files changed, tests run, behavior changed, deviations, unresolved concerns,
and the document changes that the orchestrator must make.

Stop and report the problem, and do not guess, when:
- Packet 001 is not complete. This is true when `src/view_models/search_query.rs` is absent, or
  when a name of its "Names For Later Packets" is absent or has a different signature.
- ADR 0086 packet 005 is not complete.
- A recorded fact of this packet is not true in the working tree, and ADR 0086 packets 002 to 005
  did not cause the difference.
- `json_each` or `json_valid` is not available in the bundled SQLite.
- A guard other than the five guards of D24 fails because of this change.
- The note of Decision 5 cannot reach the page without a change under `src/ui/`.
- A change needs a migration, a schema change or a new architecture decision.
- A change needs a file in "Do not touch".
