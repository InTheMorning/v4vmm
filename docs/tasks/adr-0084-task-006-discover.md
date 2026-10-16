# ADR 0084 Task 006: Discover

Status: Implementation recorded - 2026-10-16. The mechanical checks are Green. V1 to V5 passed on 2026-10-16. V6 failed at step 13, and V6 and V7 are open.

## Goal

Discover is the first destination and the default when Music opens. Its list column holds its views. The first view is "New releases": the Index releases, newest first, as tiles or rows, in the detail column. It keeps the recency order and the paging of ADR 0062.
A release opens its page in the detail column, and Back returns to "New releases".

The saved-search code that no path loads goes. A later packet adds saved searches with their storage.

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md) Decision 3 (Discover and "New releases"), Decision 8 (Discover shows everything, with the state word on each Library item) and Decision 9 (the location path).
- [ADR 0062](../adr/0062-music-content-surface.md): the recency order and the paging. ADR 0084 replaces its three-state control.
- AGENTS.md: code that no entry point reaches is deleted, not parked.

## Recorded Facts - 2026-10-16

- [Task 005](archive/adr-0084-task-005-destinations-sidebar.md) made `FrameNavigationEntry::Destination(MusicDestination)` the root of each sidebar path. Music opens on `Destination(Discover)`. The Discover list column shows the content list with the `RecentMusic` source (recent Index releases, paged), the view-mode control (Tiles, List) and the three-state filter control (All, Library, Index).
- The filter control shows on Discover only. Its owners are `LibraryFilterControlDisplay` and `LibraryFilterControlTreatment` in `src/view_models/workspace/chrome.rs`, the composite `src/ui/composites/library_filter_control.rs`, `content_library_filter_control` and `set_content_filter` in `src/library/app_impl.rs`, and the frame shell. The guards `adr_0047_task_010_content_list_library_filter_is_frame_local` and `adr_0062_library_tri_state_control_contract_is_vm_owned` pin it.
- A second live reader of the filter state: `render_library_feed_detail` in `src/ui/shells/library/feed_detail.rs` reads `LibraryViewModel::content_filter()` and hides the tracks of the open Library release that do not match it. A filter set on Discover thus changed the release page. The task 006 agent found it on 2026-10-16 and stopped, as the packet required.
- The search results keep their own All, Library and Index chips (`FilterChipStripDisplay`). This packet does not change them. `ContentFilter` stays for them.
- Saved searches have no storage and no save action. `LibraryViewModel::set_saved_searches` is test-only, `src/db.rs` has no saved-search table, and no path creates one. The sidebar "Saved Searches" section, `open_saved_search` in `src/library/app_impl.rs`, `LibraryAppEvent::OpenSavedSearch` and `TopApp::open_saved_search` in `src/app.rs` serve that empty list only. The operator decided on 2026-10-16 to delete them and to add saved searches in a later packet.
- The content-list renderer `render_library_content_list` in `src/ui/shells/library/content_list.rs` draws rows and tiles, with auto paging and "Load more". Each row carries the Library state word and "New" (task 004 status line).
- The prototype canvas shows the Discover list column with the view "New releases" and the subtitle "Newest first, from the Index". The detail column shows the tiles, with the breadcrumb "Discover › New releases".

## Design

### Discover views and the location path

```rust
/// A view of the Discover column (ADR 0084 Decision 3).
pub(crate) enum DiscoverView { NewReleases }
```

- `FrameNavigationEntry::DiscoverView(DiscoverView)` is a page. `opens_detail_page` is `true` for it, so the narrow layout shows the view and Back returns to the views list.
- A click on Discover in the sidebar, and the opening of Music, give the path `Destination(Discover)`, then `DiscoverView(NewReleases)`. The breadcrumb reads "Discover › New releases".
- A release in "New releases" opens its page in the current path: "Discover › New releases › title". Back returns to "New releases", with its scroll position.

### The columns

- List column: the Discover views, as `ListRow` rows. "New releases" has the subtitle "Newest first, from the Index". The open view shows as selected. A view model gives the rows.
- Detail column: the open view. For "New releases", the present content-list renderer with the `RecentMusic` source, its view-mode control (Tiles, List), its auto paging and "Load more". It has no filter control. Each Library release keeps its state word (Decision 8).
- The detail column is wide, so the tiles wrap into a grid.

### The filter control goes

Decision 8 removes the three-state control. With no filter control on Discover, no surface shows it. Delete the control, its view-model display, its composite and its frame-shell slot. Also delete the content-list filter state and each item that only it used. The "New releases" source shows each Index release, and marks each Library release with its state word.

### The release page shows each track

The orchestrator decided on 2026-10-16, under ADR 0084 Decisions 4 and 8. The Library release page always shows each of its tracks, each with its state word. It reads no list filter. Delete the filter of `render_library_feed_detail` with `content_filter()`. This is the one change that this packet makes to a release page.

### Saved searches go

Delete the sidebar section, the event, both `open_saved_search` functions, `SavedSearchEntry`, `SavedSearchesSectionDisplay` and each item that only they used. The phase plan registers a later packet for saved searches.

## Required Changes

1. Add `DiscoverView` and `FrameNavigationEntry::DiscoverView`, with its label, its breadcrumb id and `opens_detail_page`.
2. Change the Discover sidebar click and the Music opening path to `Destination(Discover)`, then `DiscoverView(NewReleases)`.
3. Add the Discover views view model and its list-column renderer.
4. Render the open Discover view in the detail column: the content list with the `RecentMusic` source and the view-mode control.
5. Delete the three-state filter control and its state, as "The filter control goes" gives.
6. Delete the saved-search code, as "Saved searches go" gives.
6a. Delete the track filter of the Library release page, as "The release page shows each track" gives. Add a test that the release page view model, or its row projection, gives each track of the release.
7. Update the guards that pin the deleted control or the saved-search code. Delete the text that asserts a replaced rule. Name ADR 0084 Decision 8 or the operator decision of 2026-10-16. Add the guards of the mechanical criteria.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_discover_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R6-01 | The Discover views view model gives "New releases" first, with its subtitle and its open entry |
| R6-02 | The Music opening path and the Discover sidebar click give `Destination(Discover)`, then `DiscoverView(NewReleases)` |
| R6-03 | A release opened from "New releases" gives a path with `Destination(Discover)`, `DiscoverView(NewReleases)` and the release, and Back returns to `DiscoverView(NewReleases)` |
| R6-04 | `DiscoverView` opens a detail page, so the narrow layout shows it |
| R6-05 | The "New releases" content is the `RecentMusic` source, with each Index release, and the Library state word on each Library release |
| R6-06 | A guard names ADR 0084 Decision 8: no `LibraryFilterControlDisplay` and no `library_filter_control` in `src/` |
| R6-07 | A guard records the operator decision of 2026-10-16: no `SavedSearchEntry`, no `OpenSavedSearch` and no `saved_searches_section` in `src/`, until the saved-search packet |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: Music opens on Discover. The list column shows "New releases" as selected, and the detail column shows the newest Index releases. The breadcrumb reads "Discover › New releases".
- V2: Tiles and List change the view in the detail column. The tiles wrap into a grid. A scroll to the end loads more releases.
- V3: a release opens its page in the detail column. Back returns to "New releases" at the same scroll position.
- V4: each release in your Library shows its state word. No All, Library or Index control shows above Discover.
- V5: the sidebar shows no Saved Searches section. A Library release page shows each of its tracks.
- V6: under 940 px, Discover shows "New releases", and Back shows the views list.
- V7: Light and Dark, and a larger scale, show no clipped tile, row or control.

## Exclusions

- No saved searches. A later packet adds them with their storage.
- No other Discover view, for example musicL playlists or genre tags.
- No change to the search results and their chips.
- No release kind, no scope control and no singles groups. Packet 007 owns them.
- No schema change.

## Files To Inspect

- [Agent rules](../../AGENTS.md), the [source map](../architecture/source-map.md), the [phase plan](../plans/adr-0084-music-column-browser-phase-plan.md) and the archived [task 005](archive/adr-0084-task-005-destinations-sidebar.md), with its Orchestrator Review.
- `src/view_models/workspace/nav.rs`, `breadcrumb.rs`, `chrome.rs`, `mod.rs`, `tests.rs`.
- `src/app.rs`, `src/app/breadcrumb.rs`.
- `src/library.rs`, `src/library/app_impl.rs`, `src/library/column_view.rs`.
- `src/view_models/library.rs`, `src/view_models/recent_feeds.rs`.
- `src/ui/shells/library/content_list.rs`, `sidebar.rs`, `destination_lists.rs`.
- `src/ui/composites/library_filter_control.rs`, `frame_shell.rs`, `src/ui/shells/workspace.rs`.
- `tests/architecture_tests.rs`.

## Checks

```bash
cargo test --lib adr_0084_discover_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md`, `docs/architecture/source-map.md` and `docs/plans/adr-0084-music-column-browser-phase-plan.md`
- This packet: `docs/tasks/adr-0084-task-006-discover.md`
- ADR 0084 Decisions 3, 8 and 9, and ADR 0062
- The archived task 005 packet, with its Orchestrator Review
- Each file in "Files To Inspect"

Goal:
- Discover with its views column and "New releases", as "Design" gives. Delete the three-state filter control and the saved-search code.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- View models carry no GPUI type. The Library keeps no navigation state.
- Keep the recency order, the paging and the stale-result checks of the `RecentMusic` source.
- Keep each scrolling column a flex column with `min_h_0`, and each row and tile grid non-shrinking (task 004 review).
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git add` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The search results view, its rows and its chips.
- The release, track, artist and playlist pages, except the track filter that Required Change 6a deletes.
- The Settings and Show screens.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R6-01 to R6-07 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result" section and an "Operator Visual Check" section for V1 to V7. The check gives numbered steps, the expected result, an incorrect result, and the cleanup.

Test commands:
- Each command in "Checks".

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- A live path other than Discover reads the three-state filter state.
- A live path loads or creates a saved search.
- The content list cannot render in the detail column with its paging.
- A change needs a file in "Do not touch".

## Implementation Result - 2026-10-16

Each Required Change is done, with Required Change 6a from the orchestrator's resolution of 2026-10-16.

### Files Changed

- `src/view_models/workspace/nav.rs`: new `DiscoverView` enum, with `ALL`, `label()` and
  `subtitle()`. `FrameNavigationEntry::DiscoverView(DiscoverView)` is a page: `opens_detail_page`
  gives `true` for it with no added code, since the exclusion list already names only
  `Destination`, `Search` and `Settings`. New `MusicDestination::default_child_page()` gives
  `DiscoverView(NewReleases)` for Discover and `None` for every other destination. New
  `FrameNavigationState::new_with_page(destination, page)` builds an initial location path with
  no back or forward history, for the Music opening path.
- `src/view_models/workspace/mod.rs`: `ensure_frame_navigation_entries` now gives the
  `SourceList` and `ContentList` frames the path `Destination(Discover)`, then
  `DiscoverView(NewReleases)`, through the new `default_navigation_child` function. The guard
  text `fn default_navigation_entry(kind: WorkspaceFrameKind) -> FrameNavigationEntry` stays
  exact, so the existing guard keeps pinning it.
- `src/view_models/workspace/breadcrumb.rs`: new `discover_view_id`, so each Discover view has
  a stable breadcrumb element id.
- `src/view_models/workspace/chrome.rs`: deletes `LibraryFilterControlTreatment`,
  `LibraryFilterControlStateDisplay`, `LibraryFilterControlDisplay`, the
  `FrameShellDisplay.library_filter_control` field and `with_library_filter_control`.
- `src/view_models/workspace/tests.rs`: deletes the three `library_filter_control_*` tests and
  the `frame_shell_display_accepts_optional_library_filter_control` test. Adds R6-02 to R6-04 as
  `adr_0084_discover_r6_02_*`, `r6_03_*` and `r6_04_*` tests. Fixes
  `adr_0046_forward_pop_nav_forward_restores_entry_back_left`, whose expected "back-left" entry
  changes from `Destination(Discover)` to `DiscoverView(NewReleases)`, since the default
  location is now two pages deep.
- `src/app.rs`: `open_music_page` reads `MusicDestination::default_child_page()` for a new path
  that starts at a destination, and opens that child page, not the bare destination, when one
  exists. The `select_tab` fallback for Settings-to-Music with no saved navigation builds the
  same two-page path. `content_list_frame_title` gives a `DiscoverView`'s own label. This file
  deletes `open_saved_search` and `set_frame_filter`.
- `src/app.rs` also changes event handling. The `LibraryAppEvent` subscription deletes the
  `OpenSavedSearch` arm and adds a `SetContentViewMode` arm. This new arm calls the existing
  `set_content_list_view_mode`, so the Discover detail header's view-mode choice reaches the
  same path the content-list frame chrome uses for Tracks. `render_workspace_content` deletes
  the Discover branch of its content-list filter and view-mode wiring, since Discover no longer
  shows either control in that frame.
- `src/library.rs`: new `LibraryDetail::Discover(DiscoverView)` variant. Deletes
  `LibraryAppEvent::OpenSavedSearch`, replaced by `LibraryAppEvent::SetContentViewMode(ContentViewMode)`.
- `src/library/app_impl.rs`: new `open_discover_view`, `request_content_view_mode` and
  `show_discover_view` methods. `hydrate_detail_from_nav` gets a `DiscoverView` arm.
  `show_destination_list`'s Discover arm becomes a no-op, like Releases, Artists and Playlists.
  The list column shows the views list there, not the content list.
- `src/library/app_impl.rs` also changes how it renders the two Music columns.
  `render_list_column` renders the Discover views list for Discover, and keeps the content list
  for Tracks and Needs attention. `render_detail_column` splits into itself (the outer canvas
  wrapper), `render_discover_view_detail` (the Discover branch) and `render_library_detail_pane`
  (the existing release, track, artist and playlist dispatch, unchanged).
- `src/library/app_impl.rs` also changes `new_with_content_view_mode`. The method calls
  `show_discover_view(NewReleases)` once, after the construction-time recency load starts. The
  first render needs `self.detail` set, because the detail column reads `self.detail`, not only
  the navigation path. This file deletes `open_saved_search`, `content_library_filter_control`
  and `set_content_filter`, and the Saved Searches rendering block in `render_sidebar_column`.
- `src/view_models/library.rs` deletes `SavedSearchEntry`, `SavedSearchesSectionDisplay`, the
  `saved_searches` field and its three methods. It deletes `ContentListRowSource::matches_filter`,
  `ContentListPageVm.filter_state`, `filter()`, `set_filter()`, `library_filter_control()` and
  `library_filter_control_display()`. It deletes `LibraryViewModel::content_filter()`,
  `set_content_filter()` and `content_library_filter_control()`.
- `src/view_models/library.rs` simplifies `visible_rows()` to union `cached_rows` with
  `library_rows` when the source is `RecentMusic`, with no filter branch. Decision 8 keeps the
  merge, because Discover shows everything. It simplifies `is_loading()` and `has_more()` to
  remove the filter guard. `ContentListEmptyStateDisplay` removes `clear_filter_action_id`,
  because no renderer read that field. `for_filter(ContentFilter)` becomes the parameterless
  `for_no_rows()`.
- `src/view_models/library.rs` adds the `DiscoverViewRowVm` struct and the
  `discover_view_rows()` method, giving "New releases" with its subtitle and its open entry. It
  also adds `AlbumNode::release_track_rows()`, the Required Change 6a row projection. This method
  gives each track of the release with no filter. It replaces the `ContentFilter` match that
  `render_library_feed_detail` used on `album.tracks`.
- `src/ui/shells/library/feed_detail.rs` reads `album.release_track_rows()`. It no longer
  filters `album.tracks` by `library_vm.content_filter()`. This is the one change this packet
  makes to a release page, as the orchestrator decided on 2026-10-16.
- `src/ui/shells/library/detail.rs`: `render_library_detail` gets an unreachable
  `LibraryDetail::Discover(_)` arm, named and explained, since `render_detail_column` always
  renders a Discover view before this dispatcher runs.
- `src/ui/shells/library/content_list.rs` adds `render_discover_new_releases_detail`. It renders
  the view-mode control in its own header row above the existing content-list body. Below that
  header, it renders the content list itself, through the existing `render_library_content_list`,
  unchanged. This keeps the Library as the one owner of the content-list data and its scroll
  handle (`recent_music_scroll`). Back to "New releases" then keeps its scroll position.
- `src/ui/shells/library/destination_lists.rs`: new `render_library_discover_views_list` and
  `render_discover_view_row`, rendering the Discover views list column with `ListRow`.
- `src/ui/composites/library_filter_control.rs`: deleted. No surface shows the filter control
  (ADR 0084 Decision 8).
- `src/ui/composites/mod.rs`: deletes the `library_filter_control` module and export.
- `src/ui/composites/frame_shell.rs`: deletes the `library_filter_control` import, the
  `library_filter_control_display` variable, its render branch and its parameter in
  `frame_filter_row`.
- `src/ui/composites/filter_chip_strip.rs`: updates one doc comment that named the deleted
  composite.
- `src/ui/shells/workspace.rs`: deletes `content_list_library_filter_control` (the field, the
  builder and the `library_filter_control_for` lookup) and its one call site in `WorkspaceShell::render`.
- `src/application/commands/playlist.rs`: the ADR 0076 packet 001 guard reads the
  `PlayPlaylistAt` match arm text. It now stops at `LibraryAppEvent::SetContentViewMode`, the arm
  that follows it, since `OpenSavedSearch` is gone.
- `tests/architecture_tests.rs` deletes the stale bullets in
  `adr_0047_phase_b_view_model_contracts_are_gpui_free_and_shared` that named
  `LibraryFilterControlDisplay`, `LibraryFilterControlStateDisplay`, `LibraryFilterControlTreatment`,
  `SavedSearchEntry` and `set_saved_searches`. It rewrites
  `adr_0047_phase_d_filter_controls_render_through_frame_shell` to remove each
  `library_filter_control.rs` check, and keeps the `filter_chip_strip.rs` checks. It deletes
  `adr_0047_task_010a_content_list_page_vm_owns_filter_projection` and
  `adr_0047_task_010_content_list_library_filter_is_frame_local`.
  `adr_0084_discover_r6_06_three_state_filter_control_is_deleted` (R6-06) replaces both.
- `tests/architecture_tests.rs` renames
  `adr_0047_task_015_search_submit_and_saved_search_commands` to
  `adr_0047_task_015_search_submit_commands`, and moves its saved-search checks into the new
  `adr_0084_discover_r6_07_saved_searches_are_deleted` (R6-07).
- `tests/architecture_tests.rs` deletes `adr_0062_library_tri_state_control_contract_is_vm_owned`.
  `adr_0084_discover_library_rows_still_merge_into_recent_music` replaces it, and keeps only the
  surviving `library_rows` data-source contract. It fixes the `open_saved_search` and
  `set_frame_filter` text-boundary markers in `adr_0060_search_uses_music_section_navigation`
  and `adr_0060_music_surface_vocabulary_and_primary_filter_are_guarded`. It also deletes the
  checks in the second test that named the deleted filter wiring.
- `tests/architecture_tests.rs` updates the `adr_0049_inspector_source_ownership_is_guarded`
  check on `feed_detail.rs` from the deleted filter match to `release_track_rows()`. It fixes the
  `source_between` end marker in `adr_0062_mixed_entity_row_contract_is_kind_backed`, which used
  the renamed `ContentListEmptyStateDisplay` doc comment as a boundary.

### Behavior Changed

- Music opens on Discover, then "New releases", in one step: the location path and the first
  render both start two pages deep. A sidebar click on Discover gives the same path.
- Discover's list column shows its views ("New releases" today), not the content list. The
  content list, with its recency order, its paging, "Load more" and the Tiles/List control, now
  renders in the detail column. The view-mode control sits in a small header above the content
  list, inside the detail column, not in the content-list frame chrome.
- A release opened from "New releases" opens in the current path, under "New releases". Back
  returns to "New releases", with the same scroll handle and the same scroll position.
- No surface shows the three-state filter control (All, Library, Index) above Discover, or
  anywhere else in Music. The Discover content always shows every row: the recency page from the
  Index, and the full local library, deduped, each with its own Library-state word.
- The Library release page shows every track of the release, with no filter. Before this packet,
  a filter set on Discover also hid tracks on an open release page. That cross-surface link is
  gone.
- The sidebar shows no Saved Searches section. The saved-search event, its two handler functions
  and its view-model storage are gone.

### Checks

| Command | Result |
|---|---|
| `cargo test --lib adr_0084_discover_` | Green, 8 tests |
| `cargo test` | Green: 1,860 unit tests, 311 guards, 10 ignored doc tests |
| `cargo test --test architecture_tests` | Green, 311 tests |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

### Deviations From The Task

1. The task's own stop condition fired first, in an earlier session. The release page's
   `library_vm.content_filter()` read is a live path other than Discover that reads the
   three-state filter state. That session stopped and reported it with no code change, as the
   packet asked. The orchestrator resolved it on 2026-10-16 with Required Change 6a, recorded
   above in "Recorded Facts" and "Design". This session carries out that resolution.
2. `LibraryApp::new_with_content_view_mode` calls `show_discover_view(NewReleases)` once, at
   the end of construction, though the task does not name this call. Without it, the first
   render after app startup would read `self.detail` as `LibraryDetail::None`. The window would
   then show "Select an item in the list", not "New releases". The detail column now reads
   `self.detail`, and nothing before this fix set it at construction time. The navigation path
   was correct already, but the Library's own detail condition needed this same fix. This is a
   direct, mechanical requirement of Required Change 4, not a design change.
3. Deleted several items beyond the ones Required Change 5 names, each with no caller left
   after the control's removal: `ContentListRowSource::matches_filter`,
   `ContentListEmptyStateDisplay.clear_filter_action_id` (no renderer read it, even before this
   packet) and the `Library`/`Index` branches of what is now `for_no_rows()`. AGENTS.md asks for
   dead code to be deleted, not parked, in the same change as the cause.
4. Updated guards the task does not name by number, beyond Required Change 7:
   `adr_0047_phase_b_view_model_contracts_are_gpui_free_and_shared`,
   `adr_0049_inspector_source_ownership_is_guarded`,
   `adr_0060_search_uses_music_section_navigation`,
   `adr_0060_music_surface_vocabulary_and_primary_filter_are_guarded`,
   `adr_0062_mixed_entity_row_contract_is_kind_backed`, and
   `adr_0046_forward_pop_nav_forward_restores_entry_back_left` in `workspace/tests.rs`. Each
   broke as a direct, mechanical result of the Required Changes (a deleted function used as a
   text-boundary marker, a deleted field, a renamed doc comment, or a default location that is
   now two pages deep, not one). Fixing them stays inside this packet's scope, since each break
   is a direct consequence of a Required Change.
5. No `config.toml` migration was needed. `WorkspaceLayoutConfig` and `WorkspaceFrameConfig`
   persist only frame ids and kinds. The one persisted Music content setting is
   `content_list_view_mode`, under `[workspace.layout]`. The three-state filter and the
   navigation path were never persisted, so no saved config can go stale from this change.
6. This packet does not edit `docs/pending-human-checks.md` or
   `docs/plans/broadcast-chain-delivery-order.md`. AGENTS.md asks for the open visual gate to go
   in both files. This task's own rules name only this packet file as an editable document.
   Tasks 004 and 005 recorded the same scope limit, in their own deviations. The open V1 to V7
   gate still needs a record in both files.

### Unresolved Concerns

- The two files named in Deviation 6 still need the open V1 to V7 gate recorded, outside this
  packet's own editable scope.
- Other documents under `docs/` (for example `docs/adr/0062-music-content-surface.md`, the
  overhaul plan, and a 2026-09-18 review) still describe the three-state filter and saved
  searches as live. ADR 0062 already carries an ADR 0084 amendment notice at its head. The other
  documents do not carry that notice. This packet's rules name only the packet file as editable,
  so none of them changed here.
- Deviation 2's startup fix has no mechanical test. `LibraryApp::new_with_content_view_mode`
  builds GPUI entities (`InputState`, focus handles), so this file's plain `#[cfg(test)]` unit
  tests do not construct it. The Operator Visual Check's V1 step is the only proof that Music
  opens on "New releases" without a prior tab round-trip.

## Operator Visual Check

This check reads the app only. It writes no file. It needs a Linux
desktop session, this checkout, and a reachable MusicIndex endpoint, so Discover has recency
rows to show. It also needs a Library with one release that has two tracks or more. One of the
recent results must also be a release already in that Library, so a Library-state word has
something to show.

1. Close v4vmm. Make the binary.

   ```bash
   cd /home/citizen/build/v4vmm
   cargo build --bin v4vmm
   ```
2. Open the app.

   ```bash
   target/debug/v4vmm
   ```

**V1 - Music opens on Discover, with "New releases" selected and loaded**

3. Look at the window right after it opens. Do not click anything yet.
   - Correct: the sidebar shows Discover selected. The list column shows one row, "New
     releases", selected, with the subtitle "Newest first, from the Index". The detail column
     shows tiles of the newest Index releases. The breadcrumb above the detail column reads
     "Discover › New releases".
   - Incorrect: the detail column shows "Select an item in the list", or any other empty state,
     with no tiles. The list column shows more than one row, or shows the old content list, not
     the views list.

**V2 - Tiles and List, and "Load more"**

4. Click "List" in the small control above the tiles.
   - Correct: the detail column switches to rows, with the same releases, each with its title,
     its Library-state word, and its entity badge.
5. Click "Tiles" again.
   - Correct: the detail column returns to tiles, wrapped into a grid that fills the column
     width.
6. Scroll the detail column to its end.
   - Correct: more releases load in, with no visible gap or jump. A further scroll keeps
     loading more, until the Index has no further page.

**V3 - a release opens in the detail column, and Back keeps the scroll position**

7. Scroll partway down the tiles or rows. Click a release.
   - Correct: the detail column shows that release's page. The list column still shows "New
     releases", selected. The breadcrumb reads "Discover › New releases › " then the release
     title.
8. Click Back in the detail column.
   - Correct: the detail column returns to "New releases", scrolled to the same position as
     step 7, not back to the top.
   - Incorrect: the detail column returns to the top of the list, or to the empty "Select an
     item" state.

**V4 - the Library-state word, and no filter control**

9. Find a release in "New releases" that is also in your Library. Look at its tile or row.
   - Correct: it shows "In library" or an equivalent Library-state word, beside or under its
     title. A release you do not own shows "Not in library" or "New" instead.
10. Look above the tiles, in the small header row and in the whole Discover column.
    - Correct: no All, Library or Index control shows anywhere above or beside Discover.

**V5 - no Saved Searches section, and the release page shows every track**

11. Look at the full-width sidebar (expand it first if it is collapsed).
    - Correct: the sidebar lists the six destinations, then the feed area. No Saved Searches
      heading or row shows between them.
12. Open a Library release page, from Releases or a release row, with one owned track and
    one unowned track.
    - Correct: every track of the release shows in its track list, whether you own it or not.
    - Incorrect: a track is missing from the list.

**V6 - a narrow width shows "New releases", and Back shows the views list**

13. Resize the window to under 940 px wide, with "New releases" open and no release page open.
    - Correct: the window shows the sidebar and the detail column, with "New releases" and its
      tiles. The list column (the views list) is gone.
14. Click a release.
    - Correct: the detail column shows that release's page.
15. Click Back.
    - Correct: the window shows the sidebar and "New releases", with its tiles.
16. Click "Discover" in the breadcrumb.
    - Correct: the window shows the sidebar and the views list (one row, "New releases"), not
      the detail column. Click "New releases": its tiles show again.
17. Widen the window back past 940 px.
    - Correct: all three columns show again.

**V7 - Light, Dark, and a larger scale show no clipping**

18. Open Settings -> General (`Ctrl+Comma`). Select Light, then Dark, then a larger scale.
    Return to Music after each change, with "New releases" open in the detail column.
    - Correct: the tiles, the rows, the view-mode control, and the breadcrumb stay fully
      visible, with no clipped text, icon, or control, in every combination.
    - Incorrect: a tile, a row, the view-mode control, or the breadcrumb is clipped, cut off, or
      overlaps another.
19. Close the app.

**Clean-up**

None. This check reads the app only. It writes no file and changes no stored data.

## Orchestrator Review - 2026-10-16

The orchestrator reviewed the diff against ADR 0084, ADR 0062, this packet and the [review checklist](../reviews/adr-0084-review-checklist.md), and ran each check.

- The agent stopped first at a correct stop condition: the Library release page read the three-state filter. The orchestrator decided Required Change 6a under ADR 0084 Decisions 4 and 8, and the agent then completed the packet.
- The Discover views, the paths, the restore of a view by Back and the deletions keep to the design. No item is renamed to pass a guard.
- `render_library_detail` handled `LibraryDetail::Discover` with `unreachable!`, a panic on the render path. The orchestrator replaced it with the empty state that `LibraryDetail::None` gives.
- The visual check had three errors. The intro and the clean-up named a playlist that the check does not write. Step 15 expected the views list after Back, but a Discover view opens a detail page, so Back shows "New releases". Step 16 now reaches the views list through the breadcrumb.
- The overhaul plan step 7 now points to packet 012 for saved searches. ADR 0062 already carries the ADR 0084 amendment. The flow map stays a dated record.

After the corrections, each check is Green: 1,860 unit tests, 311 guards, `fmt`, `clippy -D warnings`, `check --all-targets` with no warning, and `cargo build --bin v4vmm`.

Merge recommendation: pass. Packet 007, Releases, comes next.

## Operator Check Findings - 2026-10-16

The operator walked V1 and found two defects.

- "New releases" showed the Library tracks after the Index releases. `ContentListPageVm::visible_rows` and `thumbnail_sources` added each Library row to the `RecentMusic` page. The fix removes that addition, so the view shows the Index releases only (ADR 0084 Decision 3). An Index release that the Library holds now shows "In library" and no "New" mark (Decision 8), through `ContentListPageVm::mark_library_releases`. The `replace_tree` and `replace_recent_music_content` functions apply the mark. The test `adr_0084_discover_new_release_in_library_shows_its_state` and the changed test `content_list_page_vm_projects_first_recent_music_page` are the guard.
- Scroll was slow. After the fix above, the operator found that scroll is slow only while the tiles show placeholders. This occurs in the debug build and in the release build. Item 10 of the responsiveness list in the overhaul plan records it and its possible cause. On 2026-10-16 the operator decided that a separate packet fixes it, not this packet.

The operator walked V1 again on 2026-10-16 after the fix, and it passed. "New releases" showed releases only, and a release in the Library showed "In library" with no "New" mark.

The operator walked V2 to V6 on 2026-10-16. V2 to V5 passed. The operator found these defects:

- An Index release showed "Not in library". The operator prefers no word, because then a Library item is easy to see. ADR 0084 Decision 8 gives the state word to each Library item only. The fix gives `ContentListLibraryBadgeDisplay` an optional label, and an Index item has no label. The test `adr_0084_discover_new_release_in_library_shows_its_state` is the guard.
- V6 step 13 failed. A resize of the window to under 940 px stopped the app for about 15 seconds. Then the app showed the new layout and stopped again. The cause is not known yet, and a profile of the app is necessary to find it. V6 and V7 stay open.
- The Index release page defects that the operator found during V3 are outside this packet. The phase plan lists them under "Open Findings".
