# ADR 0084 Task 012: Saved Searches

Status: Ready - 2026-10-20. It depends on packet 010. Migration 21: make the backup of the Rollback section first.

## Goal

The operator can save a toolbar search. Each saved search is a view in the Discover column, after "New releases", for example "Saved: bluegrass".
A click on the view runs its query against the Library and the Index. The results show in the detail column, with the tabs, the rows and the selection of packet 010.
The operator can delete a saved search. Migration 21 stores the saved searches.

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md):
  - Decision 2: saved searches are views in the Discover column.
  - Decision 3: each saved search is a view after "New releases". A saved search runs its query against the Library and the Index.
  - Decision 9: each page change goes into the one navigation stack. The breadcrumb shows the location.
  - Decision 10: the selection model and the bar.
- [ADR 0047](../adr/0047-library-search-unification.md) item 9, as ADR 0084 amends it: saved searches are views of the Discover column.
- [ADR 0016](../adr/README.md): each schema change goes through the migration registry.
- [ADR 0040](../adr/README.md): background work runs on the async command runner. No screen calls `cx.spawn`.
- Operator decision of 2026-10-16: packet 006 deleted the saved-search code that no path loaded. Saved searches come back in a separate packet, with their storage.
- The durable set in [AGENTS.md](../../AGENTS.md): typed action state, accessibility and cautious destructive actions.

## Recorded Facts - 2026-10-20

- Schema version 20 is current. `CURRENT_VERSION` is 20 and `MIGRATIONS` in `src/db.rs` ends with migration 20 `publisher_feed_image`.
- `migrate_schema_internal` in `src/db.rs` runs migrations 12 and later in one transaction. After each of migrations 16 to 20, it calls `upgrades::verify_target` and compares `upgrades::retained_digest` with the value before the migration.
- `schema_contract` in `src/db.rs` gives the columns of each version. The failure text in `src/db/maintenance/upgrade.rs` names "migrations 12 to 20".
- `src/db.rs` has no saved-search table. No `src/` file holds a saved-search type.
- The guard `adr_0084_discover_r6_07_saved_searches_are_deleted` in `tests/architecture_tests.rs` forbids `SavedSearchEntry`, `SavedSearchesSectionDisplay`, `OpenSavedSearch`, `saved_searches_section` and `fn open_saved_search(` in `src/`. Its text says "until the saved-search packet".
- `DiscoverView` in `src/view_models/workspace/nav.rs` has one value, `NewReleases`. It is `Copy`, and `label` and `subtitle` are `const fn` that give `&'static str`.
- `LibraryViewModel::discover_view_rows` in `src/view_models/library.rs` builds one `DiscoverViewRowVm` for each value of `DiscoverView::ALL`. `DiscoverViewRowVm.label` and `.subtitle` are `&'static str`.
- `LibraryApp::open_discover_view` in `src/library/app_impl.rs` pushes `FrameNavigationEntry::DiscoverView(view)` with `NavigationPathStart::ListRow`. The nav arm `FrameNavigationEntry::DiscoverView(view)` calls `show_discover_view`.
- "New releases" shows in the detail column through `render_discover_new_releases_detail` in `src/ui/shells/library/content_list.rs`.
- `content_list_frame_title` in `src/app.rs` gives the label of the view as the frame title. `breadcrumb_entry_id` and `discover_view_id` in `src/view_models/workspace/breadcrumb.rs` give the breadcrumb id.
- `TopApp` holds the search view model in `search_results_detail`. `search_results_detail_for_query` and `FetchIndexSearchResults` in `src/app/search_dispatch.rs` give the Library rows and the Index rows of one query.
- `config.rs` gives the default database path `v4vmm.sqlite` in the data directory of the project. The config key `db_path` can name another file.

## Design

### The storage

Add `src/db/saved_searches.rs` with migration 21, `saved_searches`:

```sql
CREATE TABLE saved_searches (
    id INTEGER PRIMARY KEY,
    query TEXT NOT NULL,
    start_tab TEXT NOT NULL CHECK (start_tab IN ('releases', 'artists', 'tracks')),
    created_at INTEGER NOT NULL
);
CREATE UNIQUE INDEX saved_searches_query ON saved_searches (query COLLATE NOCASE);
```

- `query` is the trimmed query text, as the search submitted it.
- `start_tab` is the tab that was open when the operator saved the search. It is the type filter of the saved search.
- `created_at` is the time of the save, in Unix seconds, from the caller. The database function makes no time.
- Add `insert_saved_search`, `saved_searches` (in `id` order) and `delete_saved_search`. A second insert of the same query, with another case, gives a typed `AlreadySaved` result.
- Migration 21 changes no other table. Add it to `MIGRATIONS`, `schema_contract` and the version lists of the tests. Set `CURRENT_VERSION` to 21.
- `migrate_schema_internal` verifies the retained records after migration 21, as after migration 20.
- Change "migrations 12 to 20" to "migrations 12 to 21" in `src/db/maintenance/upgrade.rs`.
- Change the schema line in "Stored facts" of `AGENTS.md` to version 21.

### The types

- Add `SavedSearch { id: SavedSearchId, query: String, start_tab: SearchResultsTab, created_at: i64 }` in `src/view_models/search_results/`. `SavedSearchId` wraps `i64`.
- Do not use a name that the R6-07 guard forbids. Delete that guard in this packet, because its end condition is this packet.
- `LibraryApp` holds the loaded `Vec<SavedSearch>`. It reads them at start, and again after each save and each delete.

### "Save search"

- The header of the search results of packet 010 gets a typed action "Save search", with a named control style and no icon.
- When the query is already saved, the action shows "Saved" and is not available. Its accessibility label is "This search is saved in Discover".
- A click saves the query and the open tab, with `chrono::Utc::now()` from the click handler. The action changes to "Saved" at once, with no navigation.
- A save failure shows in the status text of the search view. It names the query and the database error.

### The Discover views

- Add `FrameNavigationEntry::SavedSearch { id: SavedSearchId, query: String }`, a page under the Discover destination.
- `discover_view_rows` gives "New releases" first. Then it gives one row for each saved search, in `id` order.
- A saved-search row has the label "Saved: <query>", the subtitle "Library and Index" and the icon `Search`. Its accessibility label is "Saved search: <query>, Library and Index".
- `DiscoverViewRowVm.label` and `.subtitle` become `String`.
- A click on the row pushes the entry with `NavigationPathStart::ListRow`, as `open_discover_view` does.
- The breadcrumb label is "Saved: <query>". The frame title is "Saved search". The breadcrumb id is `saved-search-<id>`.

### The saved-search page

- The page shows in the detail column, as "New releases" does. It uses the view model and the renderer of packet 010.
- `TopApp` holds `saved_search_page: Option<SavedSearchPage>`, with the id, the query and one `SearchResultsInspectorPageVm`.
- The page opens on `start_tab`. Treat this tab as a tab that the operator selected, so that `select_first_populated_tab_if_automatic` does not change it.
- Each open of the entry runs the query again: the Library rows at once and `FetchIndexSearchResults` on the runtime. Back and Forward to an entry with the same id keep the present rows.
- An Index result writes the page only when the current entry is `SavedSearch` with the same id and query. Keep each stale-result check of packet 010.
- The detail column header gives "Select" or "Done" from the selection of the page. The bar is the bar of packet 010, at the bottom of the detail column.
- The toolbar search and the saved-search page have separate view models. A toolbar search does not change the saved-search page.

### Delete

- The saved-search page header gives a typed action "Delete…" with the destructive control style.
- "Delete…" opens a confirmation: "Delete the saved search "<query>"? The Library and the downloads do not change." The confirm button is "Delete".
- After the delete, the location changes to Discover with "New releases", as a sidebar click does (Decision 9).
- A Back or Forward step to the entry of a deleted saved search shows the empty state "This saved search was deleted." It runs no query.

## Orchestrator Decisions

A decision marked **Flag** changes what the operator sees, and no ADR decides it. The operator can change each flagged decision at the visual check.

1. A saved search stores the query text, the start tab and the save time. It stores no scope, because Decision 3 runs each saved search against the Library and the Index.
2. **Flag.** A saved search has no name of its own. Its label is "Saved: <query>". There is no rename. To change a saved search, the operator deletes it and saves a new one.
3. **Flag.** One saved search for each query, without case. When the query is saved, "Save search" shows "Saved" and is not available.
4. **Flag.** The saved searches follow "New releases" in the order of their save. The operator cannot move them.
5. **Flag.** "Save search" is in the header of the search results, in the list column. "Delete…" is in the header of the saved-search page, in the detail column. No context menu holds either action.
6. **Flag.** The results of a saved search show in the detail column, as "New releases" does. The results of a toolbar search stay in the list column (Decision 2). Decision 3 makes a saved search a Discover view, and packet 006 put each Discover view in the detail column.
7. **Flag.** "Delete…" asks for a confirmation, because it is a destructive action. The text tells that the Library and the downloads do not change.
8. **Flag.** After a delete, Discover opens "New releases". An old history entry of the deleted search shows an empty state, and runs no query.
9. Each open of a saved search runs the query again. A Back or Forward step to the same id keeps the rows, so the selection of packet 010 stays.
10. A new navigation entry `SavedSearch { id, query }` holds the query. `DiscoverView` stays a `Copy` type for the built-in views.
11. The R6-07 guard goes in this packet. A new guard keeps each write of the table in `src/db/saved_searches.rs`.

## Required Changes

1. Add migration 21 and `src/db/saved_searches.rs`, with their tests. Update `CURRENT_VERSION`, `schema_contract`, the upgrade text and the schema line of `AGENTS.md`.
2. Add `SavedSearch`, `SavedSearchId` and the list in `LibraryApp`.
3. Add the "Save search" action and its state to the view model and the header of packet 010.
4. Add `FrameNavigationEntry::SavedSearch`, its breadcrumb, its frame title and its nav arms.
5. Add the saved-search rows to `discover_view_rows`.
6. Add the saved-search page, its query run, its stale-result check, its selection and its bar.
7. Add "Delete…", the confirmation, the move to "New releases" and the empty state.
8. Delete the R6-07 guard. Add the guards of this packet.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_saved_search_` for unit tests beside the owning code. Each new guard cites ADR 0084 Decision 3. Tests use an in-memory database, recorded rows and recorded times. No test sends a request to a live host.

| Case | Required proof |
|---|---|
| R12-01 | Migration 21 adds `saved_searches` to a schema 20 database. Each retained record stays, and `CURRENT_VERSION` is 21 |
| R12-02 | `insert_saved_search`, `saved_searches` and `delete_saved_search` keep the `id` order. A second insert of "Bluegrass" after "bluegrass" gives `AlreadySaved` |
| R12-03 | The search view model gives "Save search" available for a new query, and "Saved" not available for a saved query, with the accessibility label |
| R12-04 | `discover_view_rows` gives "New releases" first, then "Saved: <query>" for each saved search in `id` order, with the subtitle and the accessibility label |
| R12-05 | The `SavedSearch` entry gives the breadcrumb label "Saved: bluegrass", the breadcrumb id `saved-search-<id>` and the frame title "Saved search" |
| R12-06 | The saved-search page opens on its `start_tab`, also when the Index rows of another tab arrive first |
| R12-07 | The location changes from saved search A to saved search B. Then an Index result for A does not change the page of B. The same is true after a change from A to a toolbar search |
| R12-08 | The delete confirmation view model gives the text with the query, and the "Delete" action. After the delete, the location is Discover with "New releases" |
| R12-09 | The entry of a deleted saved search gives the empty state, and its view model starts no query |
| R12-10 | The saved-search page gives the header action and the bar of packet 010 for its Releases tab and its Tracks tab |
| R12-11 | A guard: only `src/db/saved_searches.rs` writes the `saved_searches` table, and no file in `src/ui/` reads it |
| R12-12 | Each packet 010 test and each migration test of migrations 12 to 20 still passes |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

The first step of the operator visual check is the backup of the Rollback section. Do it before the first start of the new build.

- V1: a toolbar search for "bluegrass" shows "Save search" in the header. A click changes it to "Saved" at once.
- V2: Discover shows "Saved: bluegrass" after "New releases", with its subtitle and its icon.
- V3: a click on "Saved: bluegrass" shows the results in the detail column, on the tab that was open at the save. The breadcrumb reads "Discover › Saved: bluegrass".
- V4: a row of the saved-search page opens its page. Back returns to the saved-search page with the same rows.
- V5: "Select" on the saved-search page shows the checkboxes, and "Download n releases" queues the downloads.
- V6: "Delete…" asks for a confirmation. After the delete, Discover shows "New releases", and the saved-search row is absent.
- V7: Light and Dark themes, and a narrow width. The saved-search row and the header actions show no clipped text.

## Exclusions

- No rename and no order change of saved searches.
- No saved search in the sidebar. Decision 2 puts saved searches in the Discover column.
- No result count and no "new results" mark on a saved-search row.
- No change to the toolbar search, except the "Save search" action.
- No CLI command for saved searches.
- No other view in Discover, for example shared musicL playlists or genre tags.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- Packet 010: `docs/tasks/adr-0084-task-010-search-results-list-column.md`, and its "Implementation Result".
- `src/db.rs` (`MIGRATIONS`, `CURRENT_VERSION`, `migrate_schema_internal`, `schema_contract`), `src/db/publisher_feed_image.rs` (the model of a migration file), `src/db/upgrades.rs`, `src/db/maintenance/upgrade.rs`.
- `src/view_models/workspace/nav.rs`, `src/view_models/workspace/breadcrumb.rs`, `src/view_models/library.rs` (`discover_view_rows`, `DiscoverViewRowVm`).
- `src/library.rs`, `src/library/app_impl.rs` (`open_discover_view`, `show_discover_view`, the nav arms).
- `src/ui/shells/library/content_list.rs` (the Discover views column and "New releases").
- `src/app.rs` (`content_list_frame_title`, the detail slot), `src/app/search_dispatch.rs`, `src/app/detail_selection.rs`.
- `src/view_models/search_results/`, `src/ui/shells/search_results_inspector.rs`.
- The archived packet 006 and packet 024 (migration 20).
- `tests/architecture_tests.rs`: `adr_0084_discover_r6_07_saved_searches_are_deleted`, and each guard that lists the versions of `MIGRATIONS`.

## Checks

```bash
cargo test --lib adr_0084_saved_search_
cargo test --lib adr_0084_search_list_
cargo test --lib db::
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Migration 21 changes the database. Make the backup before the first start of the new build:

1. Close the app.
2. Find the database file: the value of `db_path` in the config file. The default is `~/.local/share/v4vmm/v4vmm.sqlite`.
3. Copy that file as the backup: `cp ~/.local/share/v4vmm/v4vmm.sqlite ~/v4vmm-before-schema-21.sqlite`.

To roll back, close the app and revert the commit. Then copy `~/v4vmm-before-schema-21.sqlite` back to the database path. An older build refuses a database at schema 21.

The backup holds no saved search. A rollback thus removes each saved search, and changes no other record.

## Operator Answers - 2026-10-20

- The operator confirmed each decision as written. Rename, reordering and results in the list column are recorded as possible improvements in item 12 of the [HIG product polish backlog](../plans/hig-product-polish-backlog.md).

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0084-task-012-saved-searches.md`
- Packet 010, with its "Implementation Result"
- ADR 0084 Decisions 2, 3, 9 and 10, and ADR 0047 item 9
- The archived packet 006 and packet 024
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the migration, the types, "Save search", the Discover views, the page and "Delete…".

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Add the schema change only through the migration registry of ADR 0016.
- The view model gives each text, each label and each availability. The screen only composes. Use named tokens and named icons.
- Background work runs on the ADR 0040 runtime. No screen calls `cx.spawn`.
- Tests use an in-memory database, recorded rows and recorded times. No test sends a request or writes a file.
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git add` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The rows, the tabs and the selection of packet 010, except the "Save search" action.
- Each table other than `saved_searches`.
- Any ADR, and each document other than this packet and the schema line of `AGENTS.md`.

Acceptance criteria:
- Each case R12-01 to R12-12 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - 2026-10-20" section and an "Operator Visual Check" section for V1 to V7. The backup of the Rollback section is its first step.

Test commands:
- The commands in "Checks", with `cargo fmt -- --check` after the last edit and `cargo build --bin v4vmm` last.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- Packet 010 is not implemented.
- The migration registry needs a change to its own rules.
- The saved-search page cannot use the view model of packet 010 in the detail column.
- A change needs a file in "Do not touch".
