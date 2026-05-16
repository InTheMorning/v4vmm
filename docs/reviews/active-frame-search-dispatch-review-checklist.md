# Active-Frame Search Dispatch Review Checklist

Status: Implemented - 2026-05-16.

## Reviewed Artifacts

- `docs/plans/active-frame-search-dispatch-plan.md`
- `docs/tasks/active-frame-search-dispatch-task-001-workspace-descriptor.md`
- `docs/tasks/active-frame-search-dispatch-task-002-page-vm-text-filters.md`
- `docs/tasks/active-frame-search-dispatch-task-003-entity-detail-text-filters.md`
- `src/view_models/workspace.rs`
- `src/view_models/library.rs`
- `src/view_models/search_results.rs`
- `src/view_models/queue_now_playing.rs`
- `src/view_models/track_detail.rs`
- `src/view_models/feed.rs`
- `src/view_models/artist_detail.rs`
- `src/view_models/playlist_detail.rs`
- `src/view_models/paged_feed_detail.rs`
- `src/view_models/paged_playlist_detail.rs`
- `tests/architecture_tests.rs`

## Gate Status

Status: Phase 1 VM contracts implemented and verified - 2026-05-16.

## Required Checks

- [x] Task 001: workspace search descriptor is GPUI-free and lives in
      `src/view_models/workspace.rs`.
- [x] Task 001: descriptor projects focused frame id, kind, current navigation
      entry, scope, and placeholder.
- [x] Task 001: empty layouts return no descriptor.
- [x] Task 001: tests cover source-list, content-list, search-detail,
      entity-detail, and queue descriptors.
- [x] Task 002: content-list text filtering is VM-owned and composes with
      `ContentFilter`.
- [x] Task 002: search-results inspector query can be updated and cleared
      through VM methods.
- [x] Task 002: queue text filtering preserves original queue rows and clears
      on empty input.
- [x] Task 003: applicable entity-detail VMs own text filtering for track row
      projections.
- [x] Task 003: VMs without track row projections are intentionally left
      unchanged and reported.
- [x] Architecture guard records that toolbar search dispatch must use the
      workspace descriptor path.
- [x] No app dispatch, toolbar UI, DB, backend, playback, or network behavior
      changes in Phase 1.
- [x] `cargo fmt -- --check` is green.
- [x] `cargo check` is green.
- [x] Relevant focused tests are green.
- [x] `cargo clippy -- -D warnings` is green.

## Required Fixes

- Do not implement Phase 2 toolbar binding until Phase 1 checks are green.
- Do not introduce a second search scope enum outside the workspace VM.
- Do not put fallback labels, placeholders, or query interpretation in
  renderers.
- Do not wire result loading for search-results inspector in this plan.

## Optional Improvements

- Split entity-detail filtering into narrower follow-up packets if the current
  detail VM ownership is too uneven for one bounded task.
- Revisit the content-list settings placeholder once Settings has a real
  frame-local page VM.

## Architectural Drift Watchlist

- Toolbar input remains a dispatcher; it does not own page-specific search
  semantics.
- `WorkspaceLayout` remains GPUI-free.
- Page VMs own text/query state and row projection.
- Empty-query submit clears the focused VM filter once Phase 2 wiring lands.
- Cmd/Ctrl+Enter remains the only v1 modifier for opening a new search-results
  Detail frame.
- Search result loading remains a separate follow-up plan.

## Merge Recommendation

Phase 1 can merge. Phase 2 should wire toolbar placeholder and submit dispatch
through `WorkspaceLayout::focused_search_descriptor()` without changing the
search-results loader scope.
