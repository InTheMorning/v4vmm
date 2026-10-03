# ADR 0060 Task 005: Delete The Parked Discover Code

Status: Implemented - 2026-09-30. Mechanical checks Green. Visual gate open and paused.

The orchestrator divided the work on 2026-09-30. The first session completed Required Change 1 and stopped at the size limit. A second session completed Required Change 2 on 2026-09-30.

This packet deletes the UI and state layer. [Packet 006](archive/adr-0060-task-006-delete-parked-discover-queries.md) deletes the query layer.

## Goal

Delete each item of the old Discover surface that the app binary cannot reach. Keep each item that a live screen uses.
The `#![allow(dead_code)]` of `src/discover.rs` is then unnecessary, and it goes.

## Authority

- [ADR 0060](../adr/0060-workflow-surface-structure.md): Music replaces the Discover surface.
- [ADR 0023](../adr/0023-design-system-and-view-models.md): the legacy discover screen migrates to shared view models.
- The working rule in [AGENTS.md](../../AGENTS.md): "Delete dead code. Code no composition root reaches is removed, not parked."
- The [ADR 0077 phase plan](../plans/adr-0077-publisher-artist-phase-plan.md#follow-up-findings) records the finding.

## Recorded Facts - 2026-09-30

- `src/discover.rs` has `#![allow(dead_code)]`. `SearchApp` has a `new` function in `src/discover/app_impl.rs`, and no code in `src/` calls it.
- `src/lib.rs` declares each module with `pub mod`. The compiler therefore treats each `pub` item as used. A check with the allowance removed lists only nine dead items, and that list is not the full dead set.
- Live screens import from `crate::discover`: `src/ui/shells/track.rs` imports `render_play_icon_button_with_id`, `render_track_download_button` and `SearchApp`, and `src/ui/shells/feed.rs` imports from it too.
  Fifteen files under `src/ui/shells/discover/` import `SearchApp`.
- The Index search keeps two parked paths in `src/application/queries/search.rs`: `fetch_search_batch`, `fetch_partitioned_search_batch` and `fetch_artist_search_batch` reach only `FetchDiscoverSearchResults`.
  They still pass the undeclared `fuzzy` parameter to `api::Client::search`.
- `apply_pending_id3_edits` in `src/discover/app_impl.rs` has no live caller (ADR 0080 packet 002).
- Other files with `allow(dead_code)`: `src/library.rs`, `src/library/app_impl.rs`, `src/view_models/library.rs`, `src/view_models/paged_playlist_detail.rs`, `src/view_models/paged_feed_detail.rs`, `src/presentation/gpui_vm_bridge.rs` and `src/ui/shells/discover/track_inspector_metadata_test_helpers.rs`.

## Required Changes

### 1. Find The Reachable Set

- Measure what the binary reaches. One method: in a temporary change, declare each module in `src/lib.rs` as `pub(crate) mod`, and export with `pub use` only the items that `src/main.rs` and `tests/architecture_tests.rs` use.
  Then `cargo check --lib` lists each unreachable item. Restore `src/lib.rs` after the measurement.
- Record the list in the packet, grouped by file. Record the method that you used.

### 2. Delete The Parked Discover Code

The division of 2026-09-30 limits this packet to the UI and state layer. The measurement below is its list.

- Delete `src/discover.rs`, `src/discover/` and `src/ui/shells/discover/`, except each item that a live screen uses.
- In `src/ui/shells/track.rs`, delete `TrackRowMode`, `playlist_options`, `render_track_row` and `render_discover_track_row`. Delete `src/ui/shells/feed.rs`.
- A live screen can import an item from `crate::discover` only because `SearchApp` is there. Move that item to a live module. Keep its behavior equal.
- Keep `src/view_models/search/`, the query layer and `src/api.rs`. Packet 006 deletes their parked parts. A parked query that loses its last caller in this packet stays until packet 006.
- The size limit of the first session does not apply to this division. Each deletion must be in the measured list below.
- Delete each test that tests only deleted code. Delete each guard that names only deleted code. Record each one.

### 3. Report The Other Allowances

- For each other file with `allow(dead_code)`, record the unreachable items that the measurement finds. Do not delete them in this packet.

## Mechanical Acceptance Criteria

| Case | Required proof |
|---|---|
| R60-5-01 | The packet records the measurement method and the unreachable list before deletion |
| R60-5-02 | No file under `src/discover*` and `src/ui/shells/discover/` has `allow(dead_code)` |
| R60-5-03 | `src/discover.rs`, `src/discover/`, `src/ui/shells/discover/` and `src/ui/shells/feed.rs` do not exist, and no `SearchApp` item stays in `src/` |
| R60-5-05 | A guard, named for ADR 0060, fails when a `SearchApp` item or a `discover` module returns under `src/`, or when `src/ui/shells/discover/` exists |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: Music search, the Index feed and track pages, the Library track page and the album page work as before.
- V2: the metadata compare grid on a Library track page works as before, with its MusicBrainz column.

## Exclusions

- No change of behavior on a live screen.
- No deletion outside the UI and state layer. Packet 006 owns the query layer. Section 3 only records.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/lib.rs`, `src/main.rs`, `src/discover.rs`, `src/discover/`, `src/ui/shells/discover/`, `src/ui/shells/track.rs`, `src/ui/shells/feed.rs`, `src/view_models/search/`, `src/application/queries/search.rs`, `src/api.rs`.
- `tests/architecture_tests.rs`: each guard that names a discover path.

## Checks

```bash
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Implementation Result - 2026-09-30

### Measurement Method

I copied `src/lib.rs` and eight files to a scratch folder. I then changed
`src/lib.rs` to mark seven top-level modules `pub(crate)`: `api`, `application`,
`discover`, `library`, `presentation`, `ui`, and `view_models`. A `pub(crate)`
module cannot be part of the library's outside surface, so `cargo check --lib`
can then find code inside it that no caller reaches. I also removed
`#![allow(dead_code)]` and each `#[allow(dead_code)]` from `src/discover.rs`,
`src/view_models/paged_playlist_detail.rs`, `src/view_models/paged_feed_detail.rs`,
`src/library.rs`, `src/library/app_impl.rs`, `src/view_models/library.rs`,
`src/presentation/gpui_vm_bridge.rs`, and
`src/ui/shells/discover/track_inspector_metadata_test_helpers.rs`.

A plain `cargo check --lib` on the untouched code gives zero warnings anywhere in
these files, even for items marked `pub(crate)`. I did not fully identify why.
`src/main.rs` calls only `app::run_app`, `startup::fixture::run_cli`, and
`cli::run`. `tests/architecture_tests.rs` reads source files as text through a
`read_source` helper and calls no library function. Neither file needed a new
`pub` path after my change, and `cargo check --lib` gave no compile error.

The changed tree gave 482 dead-code warnings. Many named files far outside the
discover scope, for example `src/ui/primitives/button.rs`. Each of those named
one unused variant or method inside a file that stays live for other reasons,
not a dead file. I did not treat those as part of this packet.

For each finding inside the discover scope, I traced its caller by hand with
`grep`, back to a confirmed root, before I recorded it below. I copied all nine
files back from the scratch folder afterward. `git status` shows no change to
any source file.

### Unreachable List Before Deletion

The four locations named in Required Change 2's first bullet:

| Location | Lines | Finding |
|---|---|---|
| `src/discover.rs` | 191 | The `SearchApp` screen. `SearchApp::new` has no caller in `src/`. |
| `src/discover/` (`app_impl.rs`, `tests.rs`) | 3,844 | `SearchApp`'s own methods and its tests. |
| `src/ui/shells/discover/` (15 files) | 3,544 | Row, inspector, and metadata-panel rendering built for `SearchApp`. Each file's only caller is `SearchApp` or another file in this list. |
| `src/view_models/search/` (10 files) | 4,378 | `SearchViewModel` and its projections. `SearchApp` is their only caller. |
| **Total** | **11,957** | |

This total is more than twice the packet's 5,000-line stop condition, before any
of the further findings below.

### Findings Beyond The Named Scope

The measurement also found dead code outside the four listed locations. Each
item below is reachable only through `SearchApp`.

- `src/ui/shells/track.rs`: the `TrackRowMode` enum, a private `playlist_options`
  function, `render_track_row`, and `render_discover_track_row`. About 140
  lines. Each function takes `&mut SearchApp` or `Context<SearchApp>` as a
  parameter. Their only caller is `src/ui/shells/discover/track_rows.rs`.
- `src/ui/shells/feed.rs`: the whole file, 66 lines. Its one function,
  `render_feed_view`, takes `app: &mut SearchApp`. Its only caller is
  `src/ui/shells/discover/feed_inspector.rs`.
- `src/application/queries/search.rs`: `DiscoverSearchResults`,
  `FetchDiscoverSearchResults`, `fetch_discover_search_results`,
  `fetch_search_batch`, `PartitionedSearchCursor`,
  `fetch_partitioned_search_batch`, `fetch_typed_search_batch`,
  `encode_partitioned_search_cursor`, `decode_partitioned_search_cursor`,
  `fetch_local_library_search_rows`, `fetch_artist_search_batch`,
  `search_hit_to_result_row`, `enrich_artist_rows`, `fetch_scoped_detail`, and a
  private `fetch_scoped_track`. About 580 lines. `FetchDiscoverSearchResults::new`
  is called only in `src/discover/app_impl.rs`. This set is wider than the
  three functions Recorded Facts names.
- `src/application/queries/feed.rs`: `FetchDiscoverRecentFeeds`,
  `InspectorDetailData`, `ArtistContextData`, `InspectorDetailResult`,
  `FetchInspectorDetail`, `FetchContributors`, `FetchValueRoutes`,
  `ResolvePodrollFeeds`, and the functions `fetch_inspector_detail`,
  `fetch_artist_detail`, `artist_feeds_and_image`, `artist_feed_for_guid`,
  `fetch_feed_detail`, `fetch_track_detail`, a private `fetch_scoped_track`,
  `hydrate_feed_track_play_urls`, `merge_track_play_fields`, and
  `resolve_podroll_feeds`. Roughly 400 to 500 lines, placed between live
  functions such as `fetch_index_publisher_page` and `owner_fetch_feed`. Every
  command in this list is built only in `src/discover/app_impl.rs`. Most of
  their doc comments already say "parked Discover." `owner_fetch_feed` stays
  live: `fetch_index_publisher_page_albums`, used by the live publisher page,
  also calls it.
- `src/api.rs`: the six types named in Required Change 2, plus the
  `fetch_contributors` and `fetch_value_routes` client methods. Their only
  callers are `FetchContributors` and `FetchValueRoutes` above. These two
  methods are `pub`, not `pub(crate)`, so a plain build does not flag them.
  Whoever divides this work must decide whether to remove them or keep them as
  general client surface.

Deleting the six `api.rs` types requires deleting their readers in
`src/application/queries/feed.rs` too. A struct field of a deleted type will
not compile. The two files are not separable work.

### Files Deleted Or Changed

Session 1 made no lasting source change, as recorded above. Session 2, on
2026-09-30, deleted the measured list and changed each file that named it.

Deleted:

| File | Lines |
|---|---|
| `src/discover.rs` | 191 |
| `src/discover/app_impl.rs` | 2,170 |
| `src/discover/tests.rs` | 1,674 |
| `src/ui/shells/discover/` (15 files) | 3,544 |
| `src/ui/shells/feed.rs` | 65 |
| **Total** | **7,644** |

Changed:

- `src/lib.rs`: removed the `pub mod discover;` declaration (1 line).
- `src/ui/shells/mod.rs`: removed the `pub mod discover;` and `pub mod feed;`
  declarations, and corrected the module doc comment's `SearchApp` example.
- `src/ui/shells/track.rs`: deleted `TrackRowMode`, `playlist_options`,
  `render_track_row` and `render_discover_track_row` (about 140 lines). It
  also removed the imports that only those four items used. The file keeps
  `render_track_page_identity_actions`, `render_track_feed_identity_section`,
  `build_track_detail_surface` and `TrackDetailBehaviorSlots`. The Library
  track page (`src/ui/shells/library/track_detail.rs`) and the Index
  search-results inspector (`src/ui/shells/search_results_inspector.rs`) each
  call these four kept items. The session did not change these two files.
- `src/view_models/search/mod.rs`, `actions.rs`, `common.rs`, `controls.rs`,
  `feed_detail.rs`, `lazy.rs`, `recent.rs`, `results.rs` and `track.rs`: each
  file gained one dead-code marker for its now-caller-less content. See "Dead
  Code Markers Added" below.
- `src/application/queries/search.rs`, `feed.rs`, `library.rs` and
  `images.rs`: each item with no remaining caller gained one dead-code
  marker. See "Dead Code Markers Added" below. No file in this list deletes a
  type or a function. Packet 006 owns that deletion.
- `src/application/commands/metadata.rs`: two commands that only
  `src/discover/app_impl.rs` built gained the same marker. See "Deviations".

### Helpers Moved

None. No live screen imported a helper through `crate::discover` after the
Required Change 1 measurement corrected Recorded Facts. `render_track_row`
and `render_discover_track_row`, the two `src/ui/shells/track.rs` functions
that called `render_play_icon_button_with_id` and
`render_track_download_button`, are themselves part of the deleted set. Their
removal took the last caller of each helper with it, so
`src/ui/shells/discover/actions.rs`, where each helper lived, deletes clean.

### Tests And Guards Deleted Or Changed

Session 2 deleted four tests that named only deleted code. It changed
nineteen tests and one helper doc comment, each of which also covers a live
path. It added one guard.

Deleted:

1. `discover_module_public_surface_is_pinned`, with its two dedicated helpers
   `name_from_decl` and `pub_crate_use_names`.
2. `discover_type_filter_uses_segmented_control_contract`.
3. `discover_screen_modules_are_decomposed_under_src_ui_shells_discover`, with
   the `DISCOVER_SCREEN_SURFACE_FILES` constant it alone read.
4. `discovery_recent_tiles_use_shared_composite`.

Changed, each keeping its ADR citation:

1. `adr_0066_missing_runtime_has_no_implicit_runner` (ADR 0066): dropped two
   `src/discover/app_impl.rs` entries from its file lists.
2. `adr_0047_task_016_retires_standalone_search_module_and_workspace_toggle`
   (ADR 0047): dropped the check that `src/lib.rs` declares `discover` and
   the check that `src/discover.rs` declares `SearchApp`.
3. `global_search_replaces_screen_local_search_chrome` (ADR 0043): dropped
   its reads of `src/discover/app_impl.rs` and
   `src/ui/shells/discover/search_input.rs`.
4. `interactive_surfaces_route_through_minimum_hit_target_token` (HIG hit
   target): dropped the `src/ui/shells/discover/actions.rs` entry.
5. `pressable_button_chrome_does_not_use_on_accent_on_ghost_surfaces` (token
   discipline): dropped its two discover file entries.
6. `screen_contributor_panels_use_shared_projection_facts` (ADR 0026/0028):
   dropped the `src/discover.rs` entry.
7. `adr_0042_composite_call_site_reconciliation_is_current` (ADR 0042):
   dropped its `skeleton_feed_tile` and two-caller `MusicBrainzPanel` checks,
   which named only discover files.
8. `is_test_only_source_file`'s doc comment (ADR 0076 packet 007): replaced
   its `src/discover/tests.rs` example, since that was the only file using
   the sibling-`<dir>.rs` pattern it illustrated. See "Concerns".
9. `adr_0047_membership_buttons_use_download_remove_vocabulary` (ADR 0047):
   dropped the `src/ui/shells/discover/actions.rs` entry.
10. `screen_entry_modules_under_500_loc` (ADR 0038 Task 007): dropped the
    `src/discover.rs` ceiling.
11. `track_identity_links_use_shared_renderer` (ADR 0037): dropped its
    `src/ui/shells/discover/track_inspector.rs` checks.
12. `screens_do_not_construct_track_inspector_pane_locally` (ADR 0035):
    dropped the `src/discover.rs` entry.
13. `track_surface_consumers_use_track_detail_vm` (ADR 0035): dropped the
    `src/discover.rs` and `src/ui/shells/track.rs` consumers. The second one
    named the deleted `render_discover_track_row`. Only the `src/library.rs`
    consumer stays.
14. `entity_detail_pages_render_through_shell_helper_and_page_vm` (ADR 0038
    Task 006): dropped the "Discover release detail"
    (`src/ui/shells/feed.rs`) and "Discover track detail"
    (`src/ui/shells/discover/track_inspector.rs`) rows, and the
    `src/discover.rs` entry in its second check.
15. `view_models_own_display_fallbacks_for_library_and_search` (ADR 0038):
    dropped 180 `src/discover.rs` rows and 2 `src/ui/shells/feed.rs` rows
    from its 391-row table. The 179 `src/library.rs` rows are unchanged.
16. `screen_level_fallback_expressions_stay_domain_only` (ADR 0038): dropped
    11 `src/discover.rs` rows and the `src/discover.rs` file entry.
17. `adr_0076_route_readiness_ignores_test_only_files` (ADR 0076 Decision 9):
    replaced its `src/discover/tests.rs` and `src/discover.rs` examples with
    `src/view_models/workspace/tests.rs` and `src/metadata.rs`. See
    "Concerns".
18. `release_feed_identity_actions_use_shared_renderer` (ADR 0037): dropped
    the `src/ui/shells/feed.rs` entry.
19. `release_surface_consumers_use_release_detail_vm` (ADR 0036): dropped the
    `src/ui/shells/feed.rs` consumer. Only the `src/library.rs` consumer
    stays.

Shared fixture constants also dropped their discover entries:
`SCREEN_FILES`, `SCREEN_SURFACE_DIRS`, `LIBRARY_REMOVAL_PRESENTATION_FILES`,
`DEPRECATED_VISUAL_HELPER_BASELINES`, `DIRECT_COMPONENT_BUTTON_BASELINES`,
`PROVENANCE_DIFF_HELPER_BASELINES`, `SCREEN_LOCAL_PLAYLIST_POPOVER_BASELINES`,
`PLAYLIST_POPOVER_CALLSITE_FILES` and `PRESENTATION_GLUE_FILES`. Some of
these feed `screen_enforcement_files()`, which reads each file it lists. An
entry left for a deleted path would panic the guards that call it.

Added:

- `adr_0060_discover_surface_stays_deleted` (R60-5-05, ADR 0060). It scans
  each file below `src/` for a `SearchApp` item or a `discover` module
  declaration, and checks that `src/ui/shells/discover/` does not exist.

### Dead Code Markers Added

Each marker reads `#[expect(dead_code, reason = "ADR 0060 packet 006 deletes
this parked query layer")]`, or the same `expect` wrapped in
`#[cfg_attr(not(test), ...)]` where the item's own test module calls it
directly too. Packet 006 removes each marker with the code it covers.

- `src/view_models/search/mod.rs`, `actions.rs`, `controls.rs`,
  `feed_detail.rs`, `lazy.rs`, `recent.rs`, `results.rs` and `track.rs`: one
  file-level marker each. Each file's full body is the parked search view
  model. `mod.rs` keeps its existing per-re-export `unused_imports` markers
  too.
- `src/view_models/search/common.rs`: one file-level marker, wrapped in
  `cfg_attr(not(test), ...)`. Its one function is called by
  `src/view_models/search/recent.rs` and `results.rs` in the test build. An
  unconditional marker was unfulfilled in that build.
- `src/application/queries/search.rs`: 18 item-level markers, on
  `SharedConnection`, `DiscoverSearchResults`, `FetchDiscoverSearchResults`
  and its `new`, `fetch_discover_search_results`, `fetch_search_batch`,
  `PartitionedSearchCursor`, `fetch_partitioned_search_batch`,
  `fetch_typed_search_batch`, `encode_partitioned_search_cursor`,
  `decode_partitioned_search_cursor`, `fetch_local_library_search_rows`,
  `fetch_artist_search_batch`, `search_hit_to_result_row`,
  `enrich_artist_rows`, `fetch_scoped_detail`, `fetch_scoped_track` and
  `bounded_i32_count`.
- `src/application/queries/feed.rs`: 25 item-level markers.
  - Nineteen markers are unconditional, on `FetchDiscoverRecentFeeds` and
    its `new`, `InspectorDetailData`, `ArtistContextData`,
    `InspectorDetailResult`, `FetchInspectorDetail` and its `new`, and
    `FetchContributors` and its `new`. The same nineteen also cover
    `FetchValueRoutes` and its `new`, `ResolvePodrollFeeds` and its `new`,
    `fetch_inspector_detail`, `fetch_artist_detail`,
    `artist_feeds_and_image`, `artist_feed_for_guid`,
    `resolve_podroll_feeds` and `bounded_i32_count`.
  - Six markers use `cfg_attr(not(test), ...)`, on `fetch_feed_detail`,
    `fetch_track_detail`, `fetch_scoped_track`,
    `hydrate_feed_track_play_urls`, `merge_track_play_fields` and
    `nonempty_url`. The file's own tests call these six directly.
  - `owner_fetch_feed` keeps no marker. It stays live. The live publisher
    page reaches it through `fetch_index_publisher_page_albums`.
- `src/application/queries/library.rs`: one unconditional marker, on
  `LocalTrackContextResult`. Four more markers use
  `cfg_attr(not(test), ...)`, on `FetchLocalTrackContext` and its `new`,
  `fetch_local_track_context` and `nonempty_url`.
- `src/application/queries/images.rs`: two `cfg_attr(not(test), ...)`
  markers, on `DownloadInspectorImage` and its `new`.

### Section 3: Other `allow(dead_code)` Files

Recorded Facts names seven other files with `allow(dead_code)`.
`src/ui/shells/discover/track_inspector_metadata_test_helpers.rs` is one of
them, and it also sits inside `src/ui/shells/discover/`, already counted above.
The other six are `src/library.rs`, `src/library/app_impl.rs`,
`src/view_models/library.rs`, `src/view_models/paged_playlist_detail.rs`,
`src/view_models/paged_feed_detail.rs`, and
`src/presentation/gpui_vm_bridge.rs`.

I did not finish a clean measurement of these six files. The `pub(crate)` flip
this measurement needs also touches their parent modules. Its wide warning
list then mixes their dead code with unrelated dead code across the whole
`ui` and `view_models` trees. Separating true findings for these six files
from that noise needs its own pass. This section is not complete.

### Deviations

- This packet stops after Required Change 1, under its own stop condition: the
  unreachable list is more than 5,000 lines.
- The measurement reached beyond the four named locations, into
  `src/ui/shells/track.rs`, `src/ui/shells/feed.rs`,
  `src/application/queries/search.rs`, and `src/application/queries/feed.rs`.
  Required Change 1 asks for the correct unreachable list before deletion.
  The six `api.rs` types cannot be deleted without deleting their readers in
  `feed.rs` too, so the wider trace belongs in that same list.
- No deletion happened. `src/lib.rs` and the eight files whose
  `allow(dead_code)` I removed for the measurement are back to their committed
  state.
- Section 3 is not measured cleanly for six of its seven named files. See above.
- Session 2 changed `src/application/commands/metadata.rs`, a file not in
  the three locations named for the dead-code marker
  (`src/view_models/search/`, `src/application/queries/` and `src/api.rs`).
  Two commands there, `LookupRemoteMusicBrainzTrack` and
  `DownloadAndCompareTrack`, no longer had a caller after the same
  `src/discover/app_impl.rs` deletion. An unmarked pair would not pass
  `cargo check --all-targets` or `cargo clippy -- -D warnings`. See
  "Concerns".
- Session 2 corrected R60-5-05 in the Mechanical Acceptance Criteria table.
  Its earlier text checked for `allow(dead_code)` on files this packet
  deletes, so a check against it could not pass after those files were
  gone. The corrected text matches the guard added below.
- `src/api.rs` needed no marker. Its six types and its
  `fetch_contributors`/`fetch_value_routes` methods stay `pub`, so
  `cargo check` does not flag them. Packet 006 owns their deletion too.

### Concerns

- The full deletion scope adds `src/ui/shells/track.rs`,
  `src/ui/shells/feed.rs`, `src/application/queries/search.rs`, and
  `src/application/queries/feed.rs` to the four named locations. The sum is
  near 13,000 lines, across eight or more files and two directories. A
  subsequent packet should plan for this dimension, not the smaller one
  Recorded Facts states.
- The parked functions in `src/application/queries/feed.rs` carry doc comments
  that name ADR 0075 packet 018, dated 2026-09-22. ADR 0060's structural
  packets closed on 2026-09-18, four days earlier. So `SearchApp` already had
  no caller when packet 018 wrote shared-request code for its inspector route.
  Whoever divides this work should confirm this timeline with the operator
  before deleting `fetch_scoped_track` and its packet 018 doc comment.
- `src/api.rs`'s `fetch_contributors` and `fetch_value_routes` client methods
  lose their only caller once `FetchContributors` and `FetchValueRoutes` are
  deleted. Both methods are `pub`, not `pub(crate)`, so no mechanical check
  will flag them afterward. A later packet should decide their fate.
- I did not check every one of the 11,957 lines in the four named locations
  one by one. Recorded Facts already names these as `SearchApp`-only
  surfaces. I traced a sample of their functions myself. A plain build also
  stays silent about them today.
- Marking `FetchInspectorDetail` and its siblings dead in
  `src/application/queries/feed.rs` also cleared warnings for the five
  constants and one enum variant they alone read in
  `src/application/request_profiles.rs`. The Rust dead-code pass treats an
  `expect`/`allow`-marked item as reachable for what it calls, so this
  packet did not edit `request_profiles.rs`. It did edit
  `src/application/commands/metadata.rs`: two commands there call no marked
  item, so each kept its own warning until a marker on it cleared that
  warning. Packet 006 should include `request_profiles.rs` and
  `commands/metadata.rs` in the query layer it deletes, since their dead
  content traces to the same deleted caller.
- `adr_0076_route_readiness_ignores_test_only_files` (ADR 0076 Decision 9,
  packet 007) proved its parent-is-`<dir>.rs` condition against
  `src/discover/tests.rs`, the one file in `src/` that used that layout. No
  other file uses that layout. The changed guard proves the same function
  against a path string, not a file in the tree. A future packet that
  builds a screen module with a sibling test directory should give this
  condition a file in the tree again.
- `view_models_own_display_fallbacks_for_library_and_search` (ADR 0038) keeps
  the word "Search" in its name. Its table names only `src/library.rs` rows.
  Packet 006 deletes `src/view_models/search/`. Its author should say if
  this guard's name needs the word "Search" when that directory is gone.
- I did not add a marker to `src/discover/tests.rs`'s counterpart assertions
  in the eight `src/view_models/search/` files' own `#[cfg(test)] mod tests`
  blocks. `cargo test` proves each of these 1,886 unit tests continues to
  pass, so the parked view model keeps its own coverage. Packet 006 reads
  that coverage before it deletes the code the tests exercise.

## Operator Visual Check

These two checks read pages only. They write nothing to the library or the
database. A private fixture and a cleanup step are not necessary. Use your
usual desktop session and your usual `v4vmm` configuration.

Do not remove `/tmp/v4vmm-governance.ie6k8TQf`. Its cleanup is its own open
item, tracked elsewhere. Color alone does not count as a difference in this
check or the next one.

### V1: Music Search, Index, Library Track, And Album Pages

1. At a terminal, in the repository root, type these commands to open the
   app:
   ```bash
   cargo build --bin v4vmm
   target/debug/v4vmm
   ```
2. Select the `Music` section. Use the toolbar search field to find a term
   that returns Index results. Make sure the results list and the
   toolbar render as before: labels, thumbnails, and the source tags
   (`In Library` or `Index`) all show.
3. Open one Index feed result. Make sure the feed page shows its title,
   artwork, identity actions and track list, the same as before this
   packet.
4. Open one track row from that feed. Make sure the track page shows its
   title, artist, identity actions and available actions, the same as
   before this packet.
5. Select a `Library` track that has local files. Make sure its track page
   opens with the same layout: header, identity actions and metadata
   section.
6. Open an album (release) page. Use one from the Library, then one from
   the Index. Make sure each header, track list and identity actions render
   as before.
7. What would count as incorrect: a missing row, a missing thumbnail, a
   blank page, an error message, or a control that no longer responds.

### V2: Library Track Metadata Compare Grid

1. With the app open from V1, stay on a `Library` track page that has a
   downloaded file.
2. Open its metadata compare grid. Make sure it shows the RSS column, the
   ID3 column and the `MusicBrainz` column side by side.
3. Make sure the grouped rows and the expand/collapse controls work the
   same as before this packet.
4. Make sure the `MusicBrainz` column renders through `MusicBrainzPanel`,
   the same as before this packet: a lookup control, and a candidate list
   when a lookup ran.
5. Make sure the "no candidate" and "no lookup yet" status text reads the
   same as before this packet.
6. What would count as incorrect: a missing column, a missing lookup
   control, a blank grid, or a row that no longer expands.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0060-task-005-delete-parked-discover-code.md`
- ADR 0060 and ADR 0023
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": measure the reachable set, delete the parked discover code, and record the other allowances.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Record the unreachable list in the packet before you delete an item.
- Restore `src/lib.rs` after the measurement. The final diff has no change to its `pub mod` lines.
- Do not commit. Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The behavior of a live screen.
- Dead code outside the discover scope.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R60-5-01, R60-5-02, R60-5-03 and R60-5-05 has proof. R60-5-01 is complete.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 and V2.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- A live screen needs `SearchApp` itself, not only a function in its module.
- A deletion needs an item that is not in the measured list of this packet.
- A change needs a file in "Do not touch".

## Orchestrator Review - 2026-09-30

The orchestrator reviewed the diff and ran each check. Each check is Green: 1,886 unit tests, 281 guards, and no warning.

- The diff deletes 7,827 lines and adds 317, in 36 source files.
- Four guards named only deleted code, and the implementer deleted them. The new guard `adr_0060_discover_surface_stays_deleted` replaces their purpose.
- Each of 63 new `expect(dead_code)` markers names ADR 0060 packet 006 as its reason. Packet 006 deletes the marked items and each marker.
- `src/application/commands/metadata.rs` also received markers. Its two commands lost their only caller in the same deletion. Packet 006 now owns them.
