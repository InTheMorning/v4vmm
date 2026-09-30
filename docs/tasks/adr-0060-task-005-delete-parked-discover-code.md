# ADR 0060 Task 005: Delete The Parked Discover Code

Status: Ready - 2026-09-30, after division. The first session completed Required Change 1 and stopped at the size limit.
The orchestrator divided the work on 2026-09-30. This packet deletes the UI and state layer. [Packet 006](adr-0060-task-006-delete-parked-discover-queries.md) deletes the query layer.

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
| R60-5-05 | A guard, named for ADR 0060, fails when `src/discover.rs` or a file under `src/ui/shells/discover/` has `allow(dead_code)` |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: Music search, the Index feed and track pages, the Library track page and the album page work as before.
- V2: the metadata compare grid on a Library track page works as before, with its MusicBrainz column.

## Exclusions

- No change of behavior on a live screen.
- No deletion outside the UI and state layer. Packet 006 owns the query layer. Section 3 only records.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
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

None. This packet made no lasting source change. The nine files touched for
measurement are back to their committed state.

### Tests And Guards Deleted Or Changed

None.

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

## Operator Visual Check

The implementer writes this section at completion. It gives numbered steps for V1 and V2.
It states the needed state, what counts as wrong, and the cleanup. The check only reads pages.
Do not delete `/tmp/v4vmm-governance.ie6k8TQf`. Color alone is not a valid difference.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
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
