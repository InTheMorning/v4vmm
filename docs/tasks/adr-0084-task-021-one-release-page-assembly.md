# ADR 0084 Task 021: One Release Page Assembly

Status: Implemented - 2026-10-20. Cases R21-01 to R21-12 pass. Mechanical checks are Green. V1 to V3 are open, in the final walk of the run.

## Goal

The Library and the Index build the release page through one view model and one shell function. Each release track row has one view model for both origins.
The origin selects only the data source and the commands. The page does not change, except for one title rule below.

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md) Decision 4: one page for each item. The origin selects the data source, not the route or the layout.
- The durable set in [AGENTS.md](../../AGENTS.md): renderer portability. View models carry no renderer type, and screens stay thin.
- The [run record of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md): the operator added this packet to the run. The orchestrator decided the title rule and the contributors panel rule below.

## Recorded Facts - 2026-10-20

- Both origins already call `ReleaseDetailVm::new(feed, EntitySurfaceContext::Library)` on a `FeedView`, and `render_release_detail_shell` (`src/ui/shells/entity.rs`). The header view model is not duplicated.
- The slot assembly, the actions, the description, the rows and the row view models are duplicated:
  - Library: `render_library_feed_detail`, `run_library_album_action`, `LibraryTrackRowContext` and `render_library_track_row` in `src/ui/shells/library/feed_detail.rs`, with `LibraryTrackRowVm`.
  - Index: `index_feed_detail_slots`, `index_feed_track_rows` and `index_feed_primary_actions` in `src/app/search_dispatch.rs`, `render_index_feed_detail` in `src/ui/shells/search_results_inspector.rs`, and `SharedTrackRowVm` with `TrackListVm::rows` in `src/view_models/entity_detail.rs`.
- `feed_detail.rs` builds a `db::FeedRow` and a `FeedView` from the `AlbumNode` inside the screen. That is product logic in a renderer.
- `feed_detail.rs` has 498 code lines. The ADR 0038 ceiling of `src/ui/shells/library` is 500.
- The Library page shows the contributors panel. The Index page does not.
- `SharedTrackRowVm::title` does not trim the title. `TrackDetailVm::display_title` trims it and skips a title that has only spaces.
- In `render_index_detail_display`, the `feed` and `track` branches cannot run, because the one caller handles them first.
- `LibraryColumnView` observes `LibraryApp`, so a Library notify renders the Library page again in place. `TopApp` does not observe `LibraryApp`.

## Design

### The view model

Add `src/view_models/release_page.rs`. It has no renderer type.

- `ReleasePageFacts` is the origin-neutral input: the `FeedView`, the origin facts, the track facts, the name links and the row selections.
  - `ReleasePageFacts::from_library_album` takes the code that builds the `FeedView` in `feed_detail.rs` today.
  - `ReleasePageFacts::from_index_feed` takes the Index `FeedView` and its description state.
- `release_page_vm(&facts)` gives `ReleasePageVm`: the page, the actions, the action id prefix, the description, `contributors_panel` and the rows.
- `ReleaseTrackRowVm` is the one track row view model. It holds the row id, `TrackRowVm`, the primary action and its id, the status and the state label.
  It also holds the playlist popover id, the row key, the open entry, the open label, the selection and the thumbnail URL.
- `ReleasePageCommand` is a typed enum of each release command and each row command, for example `DownloadRelease`, `ToggleDescription`, `TrackPrimary(row)` and `ClickRow(row, modifiers)`.

### The shell

Add `src/ui/shells/release_page.rs` with `render_release_page(vm, images, dispatch)`. It renders the release page body and its track rows for both origins.

- Each entity stays the host of its data. It gives the facts, the resolved images and one dispatch function built from `cx.entity()`.
- `LibraryApp::run_release_page_command` replaces `run_library_album_action` and the row listeners.
- `TopApp::run_index_release_page_command` keeps the Index downloads and the Index playlist commands in `TopApp`.
- The dispatch closure holds only the entity handle and the row index. An Index command reads the open feed again through `index_release_page_feed`, so it never acts on an old `FeedView`.
- The singles group page builds a `ReleaseTrackRowVm` for each single and uses the same row renderer.

### The two rules

- The title rule: an Index track title uses the rule of `TrackDetailVm::display_title`. The title is trimmed, and a title that has only spaces shows the fallback. The orchestrator decided this on 2026-10-20, because Decision 4 gives one page and thus one title rule.
- The contributors panel: `contributors_panel` is true for the Library origin only, as today. The orchestrator records the Index gap as an open finding in the phase plan. This packet does not close it.

### Keep

- Each element id stays the same, so GPUI keeps each popover and hover state.
- The stale result checks of the Index fetch (`index_feed_detail_result_is_current`, `index_release_page_detail`).
- The in-place update after a download (`show_downloaded_index_feed`, `apply_release_page_source`).
- The two click routes of packet 018: `click_library_detail_row` and `TopApp::click_detail_row`.
- The two description state stores.
- `LibraryTrackRowVm`, because other code uses it.

### Delete

- `LibraryTrackRowContext`, `render_library_track_row`, `render_index_feed_detail`, `index_feed_detail_slots`, `index_feed_track_rows`, `ReleaseTrackRowSlot`, `render_release_track_row`, `render_track_rows`, `SharedTrackRowVm`, `TrackListVm::rows` and `TrackRow::from_shared_track_row`, when no caller remains.
- The dead `feed` and `track` branches of `render_index_detail_display`.
- `LibraryTrackRowPlace` in `src/ui/shells/library/detail.rs`, when no caller remains.

## Required Changes

1. Add the view model file and its tests.
2. Add the shell file. Make both origins and the singles group page use it.
3. Add the two dispatchers. Delete the replaced functions and types.
4. Update each guard that names a deleted function. Add the guards of this packet.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_release_page_` for unit tests beside the owning code. Each new guard cites ADR 0084 Decision 4.

| Case | Required proof |
|---|---|
| R21-01 | `release_page_vm` gives equal `page` and `rows` display for a Library album and the same feed as an Index `FeedView`. Only the actions and the description id differ |
| R21-02 | `ReleasePageFacts::from_library_album` gives the `FeedView` that `feed_detail.rs` gives today, for a recorded album with stored values and named feeds |
| R21-03 | For each `TrackMembershipState` and MusicBrainz status, a Library `ReleaseTrackRowVm` has the primary action, the status and the state label of `LibraryTrackRowVm` |
| R21-04 | An Index `ReleaseTrackRowVm` has the primary label "Download", no status, no state label, and the element ids of today |
| R21-05 | An Index row with no track GUID has no key and no open entry |
| R21-06 | A singles group row opens the release of its single and has the date text of the single |
| R21-07 | `contributors_panel` is true for the Library origin and false for the Index origin |
| R21-08 | An Index title with spaces before and after it is trimmed. A title of only spaces gives the fallback |
| R21-09 | A source guard: `src/` has one definition of `render_release_page`, and no caller of `render_release_detail_shell` outside `release_page.rs` |
| R21-10 | A source guard: no file in `src/` names `SharedTrackRowVm`, `render_library_track_row`, `render_index_feed_detail`, `LibraryTrackRowContext` or `from_shared_track_row` |
| R21-11 | A source guard: `src/ui/shells/release_page.rs` names no `LibraryApp`, no `TopApp` and no `db::` |
| R21-12 | The packet 018 tests and guards, and the stale result tests of `src/app/search_dispatch.rs`, pass with no change to their assertions |

## Visual Acceptance Criteria

These are for the operator, in the final walk of the run. No test proves them.

- V1: a Library release page and an Index release page look as before: the header, the actions, the description, the rows, the thumbnails and the selection of packet 018.
- V2: the singles group page looks as before.
- V3: an Index release with a padded track title shows the title with no extra space.

## Exclusions

- No change to the track page. Packet 023 owns one track page assembly.
- No contributors panel on the Index page.
- No change to the download paths. Packet 022 owns them.
- No schema change.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/ui/shells/library/feed_detail.rs`, `src/ui/shells/library/singles_group.rs`, `src/ui/shells/library/detail.rs`, `src/ui/shells/library/column_view.rs`.
- `src/app/search_dispatch.rs`, `src/app/detail_selection.rs`, `src/library/detail_selection.rs`, `src/library/app_impl.rs`.
- `src/ui/shells/entity.rs`, `src/ui/shells/search_results_inspector.rs`, `src/ui/composites/track_row.rs`, `src/ui/composites/release_detail_surface.rs`.
- `src/view_models/entity_detail.rs`, `src/view_models/track_detail.rs`, `src/view_models/library.rs`, `src/view_models/album_page.rs`, `src/view_models/detail_selection.rs`.
- `tests/architecture_tests.rs`: `adr_0083_both_album_page_origins_build_one_shared_surface`, `adr_0049_inspector_source_ownership_is_guarded`, `adr_0084_index_page_r13_06_*`, `adr_0024_index_track_detail_uses_rich_track_view_path`, `adr_0047_phase_c_*`, `entity_detail_pages_render_through_shell_helper_and_page_vm`, `view_models_own_display_fallbacks_for_library`, `library_release_detail_playlist_popovers_use_shared_composite`, `PLAYLIST_POPOVER_CALLSITE_FILES`, `LIBRARY_SCREEN_SURFACE_FILES` and `surface_modules_under_500_loc`.

## Checks

```bash
cargo test --lib adr_0084_release_page_
cargo test --lib adr_0084_detail_selection_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the commit. This packet adds no migration and no stored data.

## Implementation Result - 2026-10-20

### Files Changed

- `src/view_models/release_page.rs` (new), the view model with no renderer type:
  - `ReleasePageFacts` with `from_library_album`, `from_index_feed`, `map_name_links`, `with_row_selections` and `with_playlists`. `from_library_album` holds the `FeedView` build that was in `feed_detail.rs`.
  - `release_page_vm` gives `ReleasePageVm`: the page, the name links, the named feed rows, the actions, the description, `contributors_panel`, the rows and the playlists.
  - `ReleaseTrackRowVm` with `library`, `library_single` and `index`. `ReleasePageCommand`, `RowClickModifiers`, `ReleasePageActionsVm` and `ReleaseDescriptionVm`.
  - `index_feed_guid`, `index_track_row_click_key`, `index_feed_artwork_url` and `index_track_row_artwork_url` moved here from `src/app/search_dispatch.rs`, with their tests. Tests R21-01 to R21-08 are in this file.
- `src/ui/shells/release_page.rs` (new): `render_release_page`, `render_release_page_track_row`, `ReleasePageImages` and `ReleasePageDispatch`.
  The action row, the menu, the description panel, the contributors panel and the track row of both origins are here. It names no `LibraryApp`, no `TopApp` and no `db::`.
- `src/library/release_page.rs` (new): `LibraryApp::run_release_page_command`. It replaces `run_library_album_action` and the row listeners of the release page and of the singles group page.
- `src/ui/shells/library/feed_detail.rs`: only the Library host. It gives the facts, the images and `library_release_page_dispatch` to the shell. The file has 76 lines.
- `src/ui/shells/library/singles_group.rs`: each single row is a `ReleaseTrackRowVm::library_single`, rendered by `render_release_page_track_row`.
- `src/ui/shells/library/detail.rs`: `LibraryTrackRowPlace`, `LibraryTrackRowContext::with_slot` and `with_detail_row_slot` are deleted. The release page takes no separate playlist argument.
- `src/app/search_dispatch.rs`: `render_index_release_page` and `run_index_release_page_command` replace `index_feed_detail_slots`, `index_feed_primary_actions`, `index_feed_track_rows`, `index_track_row_thumbnail`, `index_track_row_actions` and `index_feed_description_panel`.
  `feed_guid_from_view` calls `index_feed_guid`.
- `src/app/detail_selection.rs`: `index_release_page_feed` is `pub(super)`.
- `src/ui/shells/search_results_inspector.rs`: `render_index_feed_detail` and the dead `feed` and `track` branches of `render_index_detail_display` are deleted.
- `src/ui/shells/entity.rs`: `ReleaseTrackRowSlot`, `render_release_track_row` and `render_track_rows` are deleted. With no track rows, `render_release_detail_shell` shows no track section.
- `src/view_models/entity_detail.rs`: `SharedTrackRowVm` and `TrackListVm::rows` are deleted. `TrackListVm` has no `context` field now. Four tests that read `rows()` changed, see Deviation 6.
- `src/ui/composites/track_row.rs`: `TrackRow::from_shared_track_row` is deleted.
- `src/view_models/track_detail.rs`: the date test of task 015 reads the Index date through `TrackRowVm`.
- `src/view_models/mod.rs`, `src/ui/shells/mod.rs` and `src/library.rs`: the new modules.
- `tests/architecture_tests.rs`: the retargeted guards and the new guards R21-09 to R21-11, see Deviation 9.

### Behavior Changed

- An Index release track row title is trimmed. A title of only spaces shows the track GUID, or "Untitled" when the track has no GUID. The accessibility label of the row uses the same title.
- Each command of a release page reads the open page again at the click. Before, a Library row command used the track of the render.
- No other change is intended. Each element id stays the same.

### Checks - 2026-10-20

| Check | Result |
|---|---|
| `cargo test --lib adr_0084_release_page_` | Green, 8 tests |
| `cargo test --lib adr_0084_detail_selection_` | Green, 9 tests |
| `cargo test` | Green, 1996 unit tests and 333 guards. Ten documentation examples stay ignored |
| `cargo test --test architecture_tests` | Green, 333 guards |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

No application launch and no production-data change occurred.

### Deviations From The Task

1. `ReleasePageCommand::ClickRow` holds `RowClickModifiers`, not `gpui::Modifiers`, because the view model holds no renderer type. The shell copies the keys from the click event.
   Each host copies them back with `click_modifiers` and calls the click routes of packet 018 with no change.
2. `render_release_page` takes a fourth argument, `cx: &App`. The name links and the status colors need it.
3. `ReleasePageImages` also holds the scroll probe of each row and the scroll handle of the detail column. These are renderer values that the host resolves.
4. `ReleasePageVm` has no separate field for the action id prefix. Each action id and the menu id start with the prefix, `album-page` or `index-album-page`. A separate field had no reader and gave a dead-code warning.
5. The Index artwork helpers and the click key helper moved into the view model, because the Index row view model needs them. `index_track_artwork_url` stays in `src/app/search_dispatch.rs` for the track page.
6. After the deletion of `TrackListVm::rows`, four tests in `src/view_models/entity_detail.rs` changed:
   - `release_page_contract_exposes_canonical_zones` reads the track count from the summary.
   - The test that compares the two contexts compares the summary only. The summary holds the track count.
   - `track_list_summary_rolls_up_count_and_duration` keeps its summary assertion. R21-04 now holds the row id.
   - `track_actions_change_by_context_without_changing_layout_contract` reads `TrackActionState::for_context`, with the same assertions.
7. The R13-05 test moved with `index_track_row_click_key` into `src/view_models/release_page.rs`, with the same assertions.
8. A Library row command decides "Remove" or "Download" from the track state at the click, through `LibraryTrackRowVm::primary_action_vm(false)`. The button is disabled while the track is busy, so the result is the same as before.
9. Guard changes. No guard is deleted:
   - Retargeted to the new owners: `local_feed_language_parity_is_loaded_through_read_model_path`, `adr_0047_phase_c_inspector_rewire_uses_vm_state_and_shared_disclosures`, `adr_0084_index_page_r13_06_*` and `adr_0049_inspector_source_ownership_is_guarded`.
   - Also retargeted: `entity_detail_pages_render_through_shell_helper_and_page_vm`, `library_release_detail_playlist_popovers_use_shared_composite`, `index_feed_detail_track_rows_preserve_artwork_fallbacks` and `adr_0060_music_row_state_labels_are_vm_owned`.
   - `PLAYLIST_POPOVER_CALLSITE_FILES` names `src/ui/shells/release_page.rs` and `src/ui/shells/library/singles_group.rs` in place of `feed_detail.rs`.
   - `adr_0083_both_album_page_origins_build_one_shared_surface` checks that both hosts call `render_release_page`, and that the shell builds the shared release shell.
   - `adr_0024_index_track_detail_uses_rich_track_view_path` does not examine the string of the dead branch now. Its other strings stay.
   - Two file lists got the new files: `adr_0047_description_rendering_must_not_infer_placeholder_metadata` and `adr_0047_membership_buttons_use_download_remove_vocabulary`.
   - New guards: `adr_0084_release_page_r21_09_one_release_page_shell`, `adr_0084_release_page_r21_10_the_old_release_row_paths_are_gone` and `adr_0084_release_page_r21_11_the_shell_names_no_host`.
10. R21-12 holds: the tests and the guards of packet 018 and the stale result tests of `src/app/search_dispatch.rs` pass with no change to their assertions.

### Unresolved Concerns

1. V1 to V3 below need the operator. No test proves the rendered page.
2. `docs/architecture/source-map.md` does not name the three new files. The task instructions named only this packet as a document to edit.
3. The new visual checks V1 to V3 are not in `docs/pending-human-checks.md`, for the same reason.
4. A row command carries the row index. If the rows change between the render and the click, the command acts on the row at that index in the new rows. A render follows each change, so this result is not probable.
5. The Index release page continues to show no contributors panel. The orchestrator records this open finding in the phase plan.

## Operator Visual Check

This check needs a Linux desktop session and this checkout. It also needs these items:

- A Library release with a description, contributors and three or more tracks, partly downloaded.
- An Index release that is not in the Library, with a description and two or more tracks.
- A Library singles group with two or more singles.
- For V3, an Index release with a track title that has a space before or after it in the RSS. If you know no such release, record V3 as not walked. The gate then stays open.

1. Build the app: `cargo build --bin v4vmm`. Then start `target/debug/v4vmm` from a desktop session.
2. Open Music, then Releases, in the sidebar.

### V1: A Library release page and an Index release page

3. Open the partly downloaded Library release. Look at the header, the cover, the name links, the action row and the "⋯" menu.
4. Look at the description panel. Click its header two times. The panel must close, then open again.
5. Look at the contributors panel below the track rows.
6. Look at each track row: the number, the thumbnail, the title, the date, the duration, "Download" or "Remove", the "MB" status, the "New" label and "+ Playlist".
7. Click "Select". Click the body of a row. Its checkbox must become checked. Click "Done".
8. Open the Index release, for example from Discover. Do steps 3, 4, 6 and 7 again. The Index page must show no contributors panel, as before.
9. On the Index page, click "Download" on one track row. The page must stay open, and the row must change to the Library state after the download.

**Wrong if:** a part of the two pages moves, changes its size or loses its thumbnail. Also wrong: a popover or a hover state closes when the page draws again, or a click opens a different page.

### V2: The singles group page

10. Open the singles group. Look at each row: the thumbnail, the title, the date of the single, the action and "+ Playlist".
11. Click the body of a row. The release page of that single must open. Push Back.
12. Click "Select", check two rows and look at the bar. Click "Done".

**Wrong if:** a row shows the date of its track in place of the date of its single, or a click opens a track page.

### V3: A padded Index track title

13. Open the Index release with the padded track title.
14. Look at the title of that track row. It must start at the same position as the titles of the other rows, with no space before or after it.

**Wrong if:** the title shows a space before it, or a title of only spaces shows an empty row title.

**Cleanup:** step 9 adds one Index track to the Library. Remove it if you do not want it. Remove each playlist that the check added. Do not delete `/tmp/v4vmm-governance.ie6k8TQf`.

## Orchestrator Review - 2026-10-20

Pass. The code needs no fix.

- `cargo fmt -- --check` and `cargo clippy -- -D warnings` are Green. `cargo test` gives 1996 library tests and 333 guards, all passing.
- A row command holds the row index. Each command reads the open page again at the click, so the index points into the current rows. A render follows each change of the rows. The orchestrator accepts this.
- The deviations agree with the packet. `RowClickModifiers` keeps the view model free of a renderer type.
- The orchestrator added the three new files to the source map.

## Orchestrator Amendment - 2026-10-20: The Row Columns And The Single Rows

The operator found two defects in the final walk and approved the fixes on 2026-10-20.

- The duration of a row with "Download" and "New" was to the left of the other durations. The trailing area of `TrackRow` had no fixed columns.
  `TrackRow` now gives four fixed columns from the token `TrackRowColumn`: the duration, the main action, the state and the playlist action. An empty slot keeps its width. The test `adr_0084_row_fix_empty_columns_keep_their_token_width` holds it.
- Each row of the singles group page showed the track number "1". A single row now has no number column, and the cover leads. The test `adr_0084_row_fix_single_row_has_no_track_number` holds it.
- A row with a MusicBrainz status and "New" puts both texts in the state column. The text can be cut at the column edge. The final walk checks it.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0084-task-021-one-release-page-assembly.md`
- ADR 0084 Decision 4, and the archived packets 013 and 018
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": one view model, one shell function, two dispatchers, and the deletions.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- No behavior change, except the title rule. Keep each element id.
- When a guard names a deleted function, change the guard to the new owner and keep its rule. Delete a guard only when its rule has no meaning after this packet, and record it.
- Tests use recorded values. No test sends a request.
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git add` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The track page assembly.
- The download functions and the download queue.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R21-01 to R21-12 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - 2026-10-20" section and an "Operator Visual Check" section for V1 to V3.

Test commands:
- The commands in "Checks", with `cargo fmt -- --check` after the last edit and `cargo build --bin v4vmm` last.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- A guard rule conflicts with one shell function for both origins, and its ADR is current.
- A file in `src/ui/shells/library` stays above the ADR 0038 line ceiling after the move.
- One render path needs a `TopApp` observation of `LibraryApp`.
- A change needs a file in "Do not touch".
