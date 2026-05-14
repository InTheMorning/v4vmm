# ADR 0047 Task 015: Search Submit + Saved Search Commands

Status: Proposed - 2026-05-14.

## Goal

Wire the toolbar search-submit action to open or focus a `Detail`
frame rendering `SearchResultsInspector`. Wire saved-search activation
in `SourceList` to dispatch `OpenSavedSearch(saved_search_id)` with
the same effect.

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
  spawning a second.
- `OpenSavedSearch(id)` resolves the saved query and reuses the
  same opener path.
- Source-list selection state is not disturbed by either command.
- HIG: search submit animation acceptable but not required in this
  task.

## Implementation Steps

1. Add a workspace-VM helper:
   `open_search_results_frame(query) -> WorkspaceFrameId` that opens
   or focuses the appropriate `Detail` frame.
2. Add `OpenSavedSearch(id)` command that looks up the saved query
   and delegates to the helper.
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

- [ ] Submitting a search opens a `Detail` frame with the inspector.
- [ ] Re-submitting updates the existing search-results frame in
  place.
- [ ] Saved-search click opens the same inspector with the saved
  query.
- [ ] Source-list selection unaffected by either command.
- [ ] Architecture guards record the contracts.

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
