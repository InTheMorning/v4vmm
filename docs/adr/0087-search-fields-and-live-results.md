# ADR 0087: Search Has Fields, A Field List And Live Results

## Status

Accepted - 2026-10-23. The operator asked for this design, answered its first four questions and accepted it on 2026-10-23 (see "Operator Answers").
The [ADR 0087 phase plan](../plans/adr-0087-search-fields-phase-plan.md) holds its packets.

It amends [ADR 0084](0084-music-column-browser.md) Decision 2. The toolbar search field shows a field list and reads fields in the query text. It shows results while the curator types. Search stays a toolbar command, never a section (ADR 0060).
It amends [ADR 0084](0084-music-column-browser.md) Decision 3: a saved search runs its fields against the Library and the Index.

## Context

On 2026-10-23 the operator asked for a powerful search. The search field must show the field headers when the curator clicks it, as Discord does. The curator must be able to type fields in the query, as in the library search of Mixxx.

The orchestrator recorded these facts on 2026-10-23:

- The toolbar field is a `gpui_component` `InputState` that `TopApp` owns (`src/app.rs:147`). It reacts only to `InputEvent::PressEnter` (`on_global_search_event`, `src/app.rs:604-614`). Nothing runs while the curator types.
- No code parses a query. The Library search is one SQL `LIKE` match against the track title, the artist, the album title, the album artist and the feed title (`db::search_library_tracks`, `src/db.rs:561-601`). It gives at most 50 tracks (`DEFAULT_LOCAL_LIBRARY_SEARCH_LIMIT`).
- The Index search sends three `GET /v1/search` requests, for `type=feed`, `type=track` and `type=publisher` (`fetch_index_search_result_rows`, `src/application/queries/search.rs:226-275`).
- Stophammer gives `q` to FTS5 after it removes some operator characters (`sanitize_fts5_query`, `../stophammer/src/search.rs:224-242`). The FTS5 columns are `name`, `title`, `description` and `tags`. Thus `title:foo` works by chance, `artist:foo` gives `400`, and an apostrophe gives `400` (open request 10).
- No Stophammer route filters a search by release kind, date or duration.
- A saved search stores the raw query text only (`src/db/saved_searches.rs`, migration 21).
- The popover primitive keeps the keyboard focus inside the popover (`src/ui/primitives/popover.rs`). A field list must keep the focus in the search field.

## Decision

### 1. One Query Text With Words, Phrases And Fields

The query text holds terms. The terms of one query all apply together.

- A **word** matches as today, for example `haleen`.
- A **phrase** is text in double quotes, for example `"night drive"`. It matches as one string.
- A **field** is a field name, a colon and a value, for example `artist:jsmovie` or `release:"aged friends"`. Decision 2 lists the fields.
- A `-` before a word, a phrase or a field excludes the matches, for example `-title:remix`.
- A field name has no case. `Artist:` is the same as `artist:`.
- A text with a colon whose name is not a field stays one word. Thus `Re:Union` searches for "Re:Union".
- A field with a value that is not correct, for example `after:yesterday`, applies nothing. The field list shows the error of that field (Decision 3).

One parser in the view model layer reads the text. The parser gives a typed query: the words, the phrases, the fields and each error. The Library search and the Index search read the typed query, never the text.

### 2. The Fields

| Field | Values | Library | Index |
|---|---|---|---|
| `artist:` | text | The track artist or the album artist | Stophammer filter (Decision 5) |
| `title:` | text | The track title or the release title | Stophammer filter |
| `release:` | text | The release title | Stophammer filter |
| `kind:` | `album`, `ep`, `single`, with commas for more than one | The release kind of ADR 0084 Decision 5 | Stophammer filter |
| `in:` | `library`, `index` | Sets the scope | Sets the scope |
| `after:`, `before:` | a date, `2026-10-01`, or a month, `2026-10` | The release date of ADR 0086 Decision 1, and the `pub_date` of a track | Stophammer filter |
| `duration:` | `>5m`, `<3:30`, `3m..5m` | The duration of a track | Stophammer filter |
| `is:` | `downloaded`, `partial`, `ready`, `removed` | The Library state of ADR 0084 Decision 8, the show readiness of ADR 0059, and the "Removed from feed" mark of ADR 0076 | None |
| `playlist:` | a playlist name | The tracks of that playlist | None |
| `has:` | `split`, `image` | A value block with payment routes, or an image of its own | None |

- `after:` and `before:` include their date. A month includes each day of the month.
- `duration:` applies to tracks. A release row shows when one or more of its tracks match.
- `is:`, `playlist:` and `has:` apply to the Library only. A query with one of them shows no Index rows. The field list marks these fields "Library only".
- `in:index` shows no Library rows. A query with `in:index` and a Library-only field shows no rows, and the field list shows the conflict.

### 3. The Field List

A click in the empty search field opens the field list under the field. Each row gives one field, its values and one example, for example `kind:  album, ep, single  ·  kind:single`.

- While the curator types a field name, the list shows the fields that start with that text.
- After a field name and its colon, the list shows the values of the field:
  - `artist:` shows the matching artist names of the Library and of `GET /v1/artist-names?from=`.
  - `release:` shows the matching release titles of the Library.
  - `playlist:` shows the matching playlist names.
  - `kind:`, `in:`, `is:` and `has:` show their fixed values.
- The keyboard focus stays in the search field. Up and Down move the selected row. Tab and Enter put the selected row into the text. Escape closes the list. A click on a row puts it into the text.
- Enter with no selected row runs the search.
- The list shows the error of a field, for example "after: needs a date, for example 2026-10-01".
- The view model gives each row, its text, its accessibility label and its typed action state. The renderer only composes.
- A shared composite under `src/ui/composites` draws the list. It does not use the popover primitive, because that primitive takes the focus.

### 4. Results Show While The Curator Types

After the curator stops typing for a pause, the search runs against the Library and the Index. Enter runs it at once. A named token holds the pause, 300 ms at first.

- Each run has a generation. The app drops a result of an older generation.
- A text with no word, phrase or correct field runs nothing, as today.
- The mounted search page updates in place. A run does not add a new page to the navigation stack. Enter, or a click on a result, keeps the page in the stack (ADR 0084 Decision 9).
- Each run sends at most three Index requests, the same as one search today.

### 5. The Index Receives Filters, Never Syntax

v4vmm sends the words and the phrases in `q`, and each Index field as its own parameter. Stophammer never parses the field syntax. [Open request 10](../plans/v4vmm-open-requests.md#10-search-accepts-words-phrases-and-field-filters) asks for these parameters.

Until Stophammer supports a filter, a query with that field shows no Index rows. The page names the field, for example "MusicIndex cannot filter by after: yet. The Library rows show." The page never shows Index rows that the filter did not reduce.

### 6. A Saved Search Keeps Its Text

A saved search stores the query text, as today. Each run parses the text again. Migration 21 does not change.

## Invariants

- One parser reads the query text. No other code reads field syntax.
- A Library value goes into SQL only as a bound parameter, with its `LIKE` characters escaped.
- No Index request carries field syntax in `q`.
- Index rows show only when each Index field of the query is applied by Stophammer.
- A result of an older run never replaces the result of a newer run.

## Non-Goals

- Colored chips for the fields in the text. The text stays plain in this ADR.
- Search operators `OR`, parentheses and fuzzy matches.
- Fields for genre, BPM and key. The Library stores them only as file tags.
- A change to the result tabs or to the search page layout.

## Consequences

- A curator can find "the singles of jSmovie after October 1" with one query.
- Live results send Index requests while the curator types. The pause and the generation limit the cost.
- `is:ready` reads the show readiness of each matched track. On a large Library this can be slow.
- The app adds one shared composite, the field list, and one parser module.
- The Library search gets one query builder for each field. The 50-track limit stays.
- Until Stophammer answers request 10, the Index fields show no Index rows.

## Alternatives Considered

- **Filter the loaded Index rows in v4vmm.** Rejected by the operator on 2026-10-23: only the first page of each type gets filtered, so a filter gives a wrong Index list.
- **The fields apply to the Library only.** Rejected by the operator on 2026-10-23.
- **The Library live and the Index on Enter.** Rejected by the operator on 2026-10-23: each result updates as the curator types.
- **The field headers only, with no values.** Rejected by the operator on 2026-10-23.
- **Stophammer parses the syntax.** Rejected: v4vmm then cannot tell the curator which field is not correct. Also, the two clients of Stophammer must then agree on one syntax.
- **The popover primitive for the field list.** Rejected: it takes the keyboard focus from the search field.

## Operator Answers - 2026-10-23

1. **Value sources for `artist:`.** The field list sends one `GET /v1/artist-names?from=` request after each pause, with the Library names.
2. **`title:` scope.** `title:` matches the track title and the release title. `release:` matches the release title only.
3. **Order with the remaining work.** The packets of this ADR come after ADR 0086 packet 005 and before ADR 0069 task 002.

## Verification

### Mechanical Criteria

A unit test in the owner layer proves each criterion.

- The parser gives the words, the phrases, the fields, the exclusions and the errors of a query text. `artist:jsmovie kind:single after:2026-10-01 -title:remix "night drive"` gives five terms.
- The parser keeps `Re:Union` as one word, and gives an error for `after:yesterday`.
- The Library query builder binds each value as a parameter and escapes `%`, `_` and `\`.
- A Library query with each field of Decision 2 gives the expected tracks from a recorded database.
- A query with a Library-only field gives no Index request.
- A query with an Index field that the stored contract does not support gives no Index rows and the note of Decision 5.
- No Index request holds a field name in `q`.
- The field list view model gives the fields for an empty text, the matching fields for `ar`, and the values for `kind:`.
- The field list view model moves the selection with Up and Down, and Tab puts the selected row into the text.
- A result of an older generation does not change the search page view model.
- A saved search with field text gives the same typed query after a restart.

### Visual Criteria

A person inspects each criterion in the running app.

- The field list opens under the field on a click, in Light and Dark, at normal and narrow width.
- The focus stays in the field while the curator moves through the list with the keys.
- The values of `artist:` and `playlist:` show and update while the curator types.
- The results change in place while the curator types, with no flash of old results.
- The error text of a field that is not correct.
- The note of Decision 5 for an Index field that Stophammer does not support.
