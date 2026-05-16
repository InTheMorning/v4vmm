# Active-Frame Search Dispatch Task 002: Page VM Text Filters

Status: Implemented - 2026-05-16.

## Goal

Add GPUI-free text filter/query mutators to the content-list, search-results,
and queue page view-models so active-frame search dispatch can later forward a
query into the focused page without renderer conditionals.

## Files to Inspect

- `docs/plans/active-frame-search-dispatch-plan.md`
- `docs/tasks/active-frame-search-dispatch-task-001-workspace-descriptor.md`
- `src/view_models/library.rs`
- `src/view_models/search_results.rs`
- `src/view_models/queue_now_playing.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/view_models/library.rs`
- `src/view_models/search_results.rs`
- `src/view_models/queue_now_playing.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/app.rs`
- `src/app/tab_bar.rs`
- `src/ui/*`
- `src/view_models/workspace.rs`
- Entity-detail view-model files
- Backend, database, playback, and network modules

## Constraints

- Keep all state GPUI-free and owned by the relevant page VM.
- Do not wire dispatch from toolbar or frame chrome.
- Do not add DB-backed result loading.
- Text filters are substring filters over display-ready row facts already owned
  by the VM.
- Empty or whitespace-only input clears the text filter.
- Preserve existing content source filters; text filtering works alongside
  `ContentFilter`, not instead of it.
- `SearchResultsInspectorPageVm::set_query(String)` already exists in the plan
  but must be verified in the current code before changing; add only the minimal
  query/clear mutator required by this task.

## Implementation Steps

1. In `ContentListPageVm`, add `text_filter: Option<String>`.
2. Add `set_text_filter(Option<String>)`, `text_filter()`, and a clear path for
   whitespace-only filters.
3. Apply the text filter in `visible_rows()` after source filtering. Match
   against stable row display text that already belongs to `ContentListRowDisplay`.
4. Preserve `replace_rows()` and `set_filter()` behavior.
5. In `SearchResultsInspectorPageVm`, add `set_query(String)` if absent and add
   `clear_query()` as a thin VM-owned mutator that refreshes empty state.
6. In `QueueNowPlayingPageVm`, add text-filter ownership and row projection. If
   the builder consumes tracks eagerly today, add the smallest cached-row
   structure needed to preserve original rows while projecting filtered rows.
7. Add focused unit tests for default state, setting filters, clearing filters,
   and source-filter plus text-filter interaction.
8. Add or strengthen an architecture guard proving these filters live in page
   VMs rather than renderers.

## Acceptance Criteria

- [x] Content-list filtering can narrow visible rows by text without losing
  `ContentFilter` behavior.
- [x] Search-results inspector query can be updated and cleared through VM methods.
- [x] Queue rows can be filtered by text and cleared back to the full queue.
- [x] Whitespace-only filters clear state.
- [x] No toolbar, app dispatch, renderer, backend, DB, or playback behavior changes.

## Implementation Notes

- `ContentListPageVm` now owns optional text filtering in addition to
  `ContentFilter`.
- `LibraryViewModel` exposes source-list and content-list text mutators for
  later active-frame dispatch.
- `SearchResultsInspectorPageVm` now exposes `set_query` and `clear_query`.
- `QueueNowPlayingPageVm` owns `all_rows` and `text_filter` directly; no global
  side table or renderer-local filter state is used.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test library
cargo test search_results
cargo test queue_now_playing
cargo test --test architecture_tests
cargo clippy -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/plans/active-frame-search-dispatch-plan.md`
- `docs/tasks/active-frame-search-dispatch-task-001-workspace-descriptor.md`
- `src/view_models/library.rs`
- `src/view_models/search_results.rs`
- `src/view_models/queue_now_playing.rs`
- `tests/architecture_tests.rs`

Goal:
- Add GPUI-free text filter/query mutators to content-list,
  search-results-inspector, and queue page VMs.

Constraints:
- VM only. Do not wire toolbar submit or render new controls.
- Preserve existing `ContentFilter` behavior.
- Empty or whitespace-only input clears text filtering.
- Do not load real search results.
- Do not edit workspace descriptor code; another task owns it.

Do not touch:
- `src/app.rs`
- `src/app/tab_bar.rs`
- `src/ui/*`
- `src/view_models/workspace.rs`
- Entity-detail view-model files
- Backend, database, playback, and network modules

Acceptance criteria:
- `ContentListPageVm::set_text_filter(Option<String>)` exists and affects
  visible row projection.
- `SearchResultsInspectorPageVm` exposes query update and clear methods.
- `QueueNowPlayingPageVm` owns queue text filtering without discarding original
  rows.
- Tests cover set, clear, whitespace, and combined source/text filtering where
  applicable.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test library`
- `cargo test search_results`
- `cargo test queue_now_playing`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Row display structs lack enough text facts to implement filtering without
  backend or renderer coupling.
- Queue filtering would require playback state changes.
- Any change appears to require `src/app.rs` or `src/ui/*`.
