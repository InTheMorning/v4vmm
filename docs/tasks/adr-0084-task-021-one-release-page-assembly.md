# ADR 0084 Task 021: One Release Page Assembly

Status: Ready - 2026-10-20.

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
