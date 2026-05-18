# ADR 0024 Library / Index Data Parity Task 006 - Readiness Gate

## Goal

Close the ADR 0024 Library / Index data-parity loading-shape follow-up with a
final readiness guard.

Tasks 001-005 added the individual loading-shape slices. This task adds the
cross-cutting regression ratchet that keeps those slices on the intended
architecture path.

## Files To Inspect

- `docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md`
- `docs/tasks/adr-0024-library-index-data-parity-task-001-feed-language.md`
- `docs/tasks/adr-0024-library-index-data-parity-task-002-index-track-detail.md`
- `docs/tasks/adr-0024-library-index-data-parity-task-003-local-track-pubdate-explicit.md`
- `docs/tasks/adr-0024-library-index-data-parity-task-004-index-artist-feed-scope.md`
- `docs/tasks/adr-0024-library-index-data-parity-task-005-playlist-local-detail.md`
- `src/app.rs`
- `src/app/search_dispatch.rs`
- `src/views.rs`
- `src/metadata.rs`
- `src/view_models/library.rs`
- `src/view_models/playlist_detail.rs`
- `src/view_models/track_detail.rs`
- `src/view_models/search_results/results.rs`
- `src/view_models/search_results/index_detail.rs`
- `src/ui/shells/library/feed_detail.rs`
- `src/ui/shells/playlist.rs`
- `src/ui/shells/search_results_inspector.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `tests/architecture_tests.rs`

## Do Not Touch

- `src/discover/**`
- `src/app.rs`
- `src/app/**`
- `src/ui/**`
- `src/view_models/**`
- `src/db.rs`
- SQLite schema / migrations
- ADR 0053 source-fact design
- runtime behavior

## Constraints

- This is an architecture/readiness guard only.
- Do not change runtime code.
- Do not add schema, query, renderer, or VM behavior.
- Do not delete or revive the parked `src/discover.rs` module.
- Guard the live ADR 0024 parity path against `crate::discover` /
  `SearchApp` / `render_discover` dependencies.
- Guard that Index result ids stay source-prefixed and remote Index ids are not
  parsed as local database ids.
- Guard that surfaced parity fields come through the existing VM/query
  contracts, not renderer-only label inference.
- No new `#[allow(...)]`; use `#[expect(...)]` only if an intentional lint
  suppression is truly required.

## Implementation Steps

1. Add one focused architecture test in `tests/architecture_tests.rs`.
2. The test should verify the five completed slice guards still exist:
   - `local_feed_language_parity_is_loaded_through_read_model_path`
   - `adr_0024_index_track_detail_uses_rich_track_view_path`
   - `local_track_pubdate_and_explicit_projection_path_is_guarded`
   - `index_artist_activation_is_scoped_feed_route_not_detail_page`
   - `adr_0024_playlist_local_detail_metadata_is_vm_owned_without_index_detail`
3. In the same test, assert live ADR 0024 Index parity files do not depend on
   the parked Discover module:
   - `src/app/search_dispatch.rs`
   - `src/view_models/search_results/results.rs`
   - `src/view_models/search_results/index_detail.rs`
   - `src/ui/shells/search_results_inspector.rs`
4. Assert Index selection dispatch handles `index-track:`, `index-feed:`, and
   `index-artist:` prefixes before local id parsing, and that Index detail
   routes store string ids in `FrameNavigationEntry::IndexFeedDetail` /
   `IndexTrackDetail`.
5. Assert renderer-only parity labels are not introduced in shells that should
   consume VM rows:
   - no `"Language"` in `src/ui/shells/library/feed_detail.rs`
   - no `"Created"`, `"Modified"`, or `"Description"` in
     `src/ui/shells/playlist.rs`
   - no `"Explicit"` in `src/ui/shells/search_results_inspector.rs`
6. Assert VM/query ownership remains visible for the surfaced fields:
   - `FeedView::from_local_with_identity` gets language through
     `nonempty_owned(f.language)`
   - `TrackView::from_api` and `TrackView::from_local_with_identity` preserve
     track parity fields
   - `PlaylistDetailPageVm::detail_rows` passes through
     `PlaylistDetailVm::detail_rows`.

## Acceptance Criteria

- Architecture tests fail if a later change moves ADR 0024 parity fields into
  renderers.
- Architecture tests fail if live search-result parity code imports the parked
  Discover module.
- Architecture tests fail if Index ids are routed through local database id
  parsing.
- Existing per-slice architecture guards remain discoverable from the final
  readiness gate.
- No runtime behavior changes.

## Test Commands

```bash
cargo fmt -- --check
cargo check --quiet
cargo build --quiet
cargo test --lib --quiet
cargo test --test architecture_tests --quiet
cargo clippy --quiet -- -D warnings
git diff --check
```

## Expected Final Summary Format

1. Files changed.
2. Tests run.
3. Behavior changed.
4. Deviations from task.
5. Unresolved concerns.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/tasks/adr-0024-library-index-data-parity-task-006-readiness-gate.md`
- `docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md`
- `tests/architecture_tests.rs`
- `src/app/search_dispatch.rs`
- `src/view_models/workspace/nav.rs`
- `src/views.rs`
- `src/metadata.rs`
- `src/view_models/library.rs`
- `src/view_models/playlist_detail.rs`
- `src/view_models/track_detail.rs`
- `src/view_models/search_results/results.rs`
- `src/view_models/search_results/index_detail.rs`
- `src/ui/shells/library/feed_detail.rs`
- `src/ui/shells/playlist.rs`
- `src/ui/shells/search_results_inspector.rs`

Goal:
- Add the final ADR 0024 loading-shape readiness architecture guard.

Constraints:
- Only edit `tests/architecture_tests.rs`.
- Do not change runtime code.
- Do not touch Discover, app/search, UI, VM, DB, schema, or ADR 0053 code.
- Guard no renderer-only field inference, no live parked-Discover dependency,
  no Index-id-as-local-id parsing, and VM/query ownership for surfaced fields.
- No new `#[allow(...)]`.
- You are not alone in the codebase. Do not revert unrelated edits.

Do not touch:
- `src/discover/**`
- `src/app.rs`
- `src/app/**`
- `src/ui/**`
- `src/view_models/**`
- `src/db.rs`
- SQLite schema / migrations
- ADR 0053 source-fact design

Acceptance criteria:
- One focused architecture test is added.
- It verifies the five per-slice ADR 0024 guard tests still exist.
- It verifies live Index parity files do not import/use parked Discover
  module patterns.
- It verifies Index-prefixed ids are handled before local id parsing.
- It verifies renderer-only parity labels are not present in the relevant
  shells.
- It verifies VM/query ownership strings for the surfaced parity fields.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo build --quiet`
- `cargo test --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`
- `git diff --check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- The guard cannot distinguish parked Discover compatibility code from the live
  search-results parity path.
- The current search dispatch shape routes Index ids through local-id parsing.
- Adding a useful guard requires runtime code changes.
