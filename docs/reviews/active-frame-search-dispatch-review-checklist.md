# Active-Frame Search Dispatch Review Checklist

Status: Superseded - 2026-05-16.

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

Status: Phases 1-4 implemented and verified - 2026-05-15.
Correction applied - 2026-05-16: user visual testing showed primary
toolbar Search and secondary "Search in new frame" produced different
surfaces for the same query. Primary toolbar Search now routes through
the same Search Results Detail path as the secondary action. Follow-up
smoke then showed that the unified Search Results surface still failed
because local Library matches were not loaded; local Library-origin
Artists, Feeds, and Tracks now hydrate from
`search_local_library_tracks`.

- Phase 1: FrameSearchScope / FrameSearchDescriptor VM contracts ✓
- Phase 2: Active-frame dispatcher via focused_search_descriptor(); six scopes routed ✓
- Phase 3: Secondary toolbar Search Results Detail button added; placeholder bind deferred ✓
- Phase 4: Architecture guards added for active-frame dispatch and toolbar Search parity ✓

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
- [x] Architecture guard records that active-frame dispatch must use the
      workspace descriptor path.
- [x] Architecture guard records that primary toolbar Search and secondary
      "Search in new frame" use the same Search Results Detail surface.
- [x] Search Results Detail loads local Library-origin result rows for
      matching Artists, Feeds, and Tracks.
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
- Do not bind primary toolbar Search to active-frame filtering; that
  produces a Library/Queue/entity filter while the secondary action
  opens Search Results Detail.
- Do not ship a local Library query that opens an empty Search Results
  inspector when matching local rows exist.

## Optional Improvements

- Split entity-detail filtering into narrower follow-up packets if the current
  detail VM ownership is too uneven for one bounded task.
- Revisit the content-list settings placeholder once Settings has a real
  frame-local page VM.

## Architectural Drift Watchlist

- Toolbar input primary Search opens Search Results Detail. Frame-local
  dispatch remains infrastructure for a future explicit in-frame
  find/filter affordance.
- `WorkspaceLayout` remains GPUI-free.
- Page VMs own text/query state and row projection.
- Empty-query behavior for future frame-local filtering must be scoped in a
  separate visible affordance task.
- Primary Search and secondary "Search in new frame" must remain the same
  Search Results Detail surface.
- Remote Index result loading remains a separate follow-up plan.

## Sandbox Limitations

GPUI initialization requires X11/GPU, unavailable in the sandbox. User
visual evidence on 2026-05-16 identified both the mismatched search
surfaces and the empty local Search Results inspector; post-fix visual
confirmation requires operator-led testing.

## Merge Recommendation

Dispatcher infrastructure for Phase 1-4 remains in place and is not
removed. However, the primary toolbar Search behavior that once
dispatched to focused-frame filtering has been retired in favor of
breadcrumb-driven search navigation within the ContentList frame
(see `docs/plans/search-in-library-frame-plan.md`). The
`FrameSearchScope`, `FrameSearchDescriptor`, and `set_text_filter`
contracts now exist as follow-up infrastructure for a future explicit
in-frame find/filter affordance, not as the active toolbar UX.
