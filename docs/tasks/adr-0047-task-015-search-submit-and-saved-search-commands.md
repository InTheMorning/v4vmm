# ADR 0047 Task 015: Search Submit + Saved Search Commands

Status: Implemented / documentation updated - 2026-05-15.

## Goal

Wire the toolbar search-submit action to open or update a `Detail`
frame rendering `SearchResultsInspector`. Wire saved-search activation
in `SourceList` to dispatch `OpenSavedSearch(saved_search_id, query)`
with the same Detail-inspector path.

## Files to Inspect

- `docs/plans/library-search-unification-plan.md`
- `docs/tasks/adr-0047-task-004-search-results-inspector-vm.md`
- `docs/tasks/adr-0047-task-012-frame-breadcrumb-vm.md`
- `docs/tasks/adr-0047-task-014-search-results-inspector-shell.md`
- `src/app/tab_bar.rs`
- `src/app.rs`
- `src/library.rs`
- `src/view_models/workspace.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/app.rs` or `src/app/tab_bar.rs` (submit handler)
- `src/view_models/workspace.rs` (`OpenSavedSearch` + search-frame
  helpers)
- `src/library/app_impl.rs` (source-list click handler for saved
  searches)
- `tests/architecture_tests.rs`

## Do Not Touch

- Backend HTTP / Musicindex (this task wires command dispatch only;
  result fetching uses existing services)
- `src/db.rs`
- Playback
- Entity composites

## Constraints

- `SubmitGlobalSearch(query)` opens or focuses a `Detail` frame
  whose content is `SearchResultsInspectorPageVm` seeded with the
  query. The frame's `FrameNavigationState` records a
  `Search(query)` entry as the origin.
- Re-submitting from the toolbar with a `Detail` frame already
  showing search results updates that frame in place rather than
  spawning a second or switching to the Search tab.
- `OpenSavedSearch(saved_search_id, query)` carries the source-list
  saved-search identity and query to `TopApp`, which reuses the same
  Detail-inspector opener path.
- Source-list selection state is not disturbed by either command.
- HIG: search submit animation acceptable but not required in this
  task.

## Implementation Steps

1. Add a workspace-VM helper:
   `open_search_results_frame(query) -> WorkspaceFrameId` that opens
   or updates the appropriate `Detail` frame.
2. Add `OpenSavedSearch(saved_search_id, query)` command dispatch from
   saved-search source-list rows and delegate from `TopApp` to the
   same Detail-inspector path.
3. Update the toolbar submit handler in
   `src/app/tab_bar.rs` (or wherever it lives) to call the helper
   instead of switching tabs.
4. Update source-list click handlers in
   `src/library/app_impl.rs` to dispatch `OpenSavedSearch` for
   saved-search entries.
5. Architecture guards:
   - Toolbar submit no longer switches a tab.
   - Source-list saved-search click dispatches `OpenSavedSearch`.
   - Helper lives on workspace VM, not in `LibraryApp` or `TopApp`.

## Acceptance Criteria

- [x] Submitting a search opens a `Detail` frame with the
  `SearchResultsInspectorPageVm` inspector through the workspace VM
  `open_search_results_frame` helper.
- [x] Re-submitting updates/reuses the existing search-results frame in
  place.
- [x] Search submit does not switch to the Search tab.
- [x] Saved-search source-list rows dispatch
  `LibraryAppEvent::OpenSavedSearch` with `saved_search_id` and
  `query`, then `TopApp` opens the same Detail inspector path.
- [x] `SearchResultsInspectorPageVm` is mounted in the `Detail` slot
  with frame-local filter chips and tab callbacks.
- [x] Frame breadcrumb display is projected by the workspace shell from
  the Detail frame `FrameNavigationEntry::Search` query.
- [x] Source-list selection is unaffected by either command.
- [x] Architecture guards record the contracts.
- [x] Task015 visual smoke entry closed by operator confirmation. Codex attempted
  `cargo run` on 2026-05-15, but GPUI failed to initialize the local
  X11/GPU context before opening a window. ADR 0048 later moved the active
  search path from Detail to ContentList.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test
cargo test --test architecture_tests
cargo clippy -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/plans/library-search-unification-plan.md`
- `docs/tasks/adr-0047-task-004-search-results-inspector-vm.md`
- `docs/tasks/adr-0047-task-012-frame-breadcrumb-vm.md`
- `docs/tasks/adr-0047-task-014-search-results-inspector-shell.md`
- `src/app/tab_bar.rs`
- `src/app.rs`
- `src/library.rs`
- `src/library/app_impl.rs`
- `src/view_models/workspace.rs`
- `tests/architecture_tests.rs`

Goal:
- Route search submit and saved-search activation to open a `Detail`
  frame rendering the search-results inspector. Re-submission
  updates in place.

Constraints:
- Workspace-VM-owned helper.
- Source-list selection unaffected.

Do not touch:
- Backend HTTP / Musicindex
- `src/db.rs`
- Playback

Acceptance criteria:
- Commands wire and dispatch correctly; architecture guards record.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Workspace VM lacks frame add/focus helpers (signals ADR 0046 phase
  5 task 012 must land first).
