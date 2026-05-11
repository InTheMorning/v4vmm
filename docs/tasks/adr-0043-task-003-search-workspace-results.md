# ADR 0043 Task 003: Search Workspace Routing and Grouped Results

Status: Implemented - 2026-05-11

## Goal

Route the global toolbar search into the Search workspace, render
grouped Library and MusicIndex results, and remove duplicate visible
Library/Discover search fields.

## Files to Inspect

- `docs/adr/0043-top-toolbar-global-search.md`
- `docs/plans/adr-0043-top-toolbar-global-search-phase-plan.md`
- `src/app.rs`
- `src/app/keyboard.rs`
- `src/search/app_impl.rs`
- `src/library/app_impl.rs`
- `src/ui/shells/discover/search_input.rs`
- `src/ui/shells/discover/result_list.rs`
- `src/view_models/search.rs`
- `src/view_models/library.rs`

## Files Likely to Change

- `src/app.rs`
- `src/app/keyboard.rs`
- `src/search/app_impl.rs`
- `src/library/app_impl.rs`
- `src/ui/shells/discover/search_input.rs`
- `src/ui/shells/discover/result_list.rs`
- `src/view_models/search.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- Playback code
- Settings UI
- MusicIndex API response mapping except where needed to feed the new
  grouped snapshot
- Database schema

## Constraints

- `AppTab::Discover` may remain the internal enum name if renaming it
  would widen the diff; user-facing label becomes `Search`.
- Existing recent-feeds root remains visible when no global query is
  active.
- MusicIndex pagination still applies only to the MusicIndex group.
- MusicIndex type filters do not filter local Library results.
- Remove visible local search fields rather than hiding them behind
  duplicate state.

## Implementation Steps

1. [x] Add `SearchApp::run_global_search(query, scope, cx)` or equivalent
   command-style entry point.
2. [x] Route toolbar Enter/Search to select the Search tab and call that
   entry point.
3. [x] Extend `SearchViewModel` snapshots to represent grouped Library and
   MusicIndex sections with clear headings and empty states.
4. [x] Render local Library results before MusicIndex results for `All`.
5. [x] Retire the visible Library search input row and Discover search
   input row.
6. [x] Preserve Search workspace recents when no global query is active.
7. [x] Add tests for grouped snapshots, scope behavior, and no-query recent
   state.

## Acceptance Criteria

- [x] One visible search field exists in the app toolbar.
- [x] `All` shows Library results first and MusicIndex results second.
- [x] `Library` does not call MusicIndex.
- [x] `Index` does not query or render local Library results.
- [x] `cmd-f` focuses the global toolbar search.
- [x] Recent feeds still appear when the Search workspace has no query.

## Implementation Notes

- The toolbar renders the single visible search field, scope segmented
  control, and submit button from `AppToolbarVm` display contracts.
- `TopApp` owns query input, selected scope, Enter handling, search-button
  routing, and `cmd-f` focus.
- `SearchViewModel` now carries source-aware rows and grouped result sections
  so local Library rows do not masquerade as MusicIndex ids.
- `SearchApp` routes `All`, `Library`, and `Index` through one command-style
  entry point. `All` combines local query rows with MusicIndex rows, while
  pagination and type filters remain MusicIndex-only.
- Local Library result clicks load a local track inspector from SQLite instead
  of calling MusicIndex with a local database id.
- Library and Search screen-local input rows were removed from rendering and
  guarded by architecture tests.
- Recent feeds remain the Search workspace empty-query root instead of a
  separate screen-local command, keeping the toolbar query as the single source
  of search state.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test search
cargo test architecture_tests
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0043-top-toolbar-global-search.md`
- `src/app.rs`
- `src/search/app_impl.rs`
- `src/library/app_impl.rs`
- `src/view_models/search.rs`
- `src/ui/shells/discover/result_list.rs`

Goal:
- Route global toolbar searches into the Search workspace, render
  grouped results, and remove duplicate visible screen-local search
  fields.

Constraints:
- Preserve recent feeds when no query is active.
- Preserve MusicIndex pagination for MusicIndex results only.
- Do not rename internal enum names if that broadens the diff.

Do not touch:
- Playback code
- Settings UI
- Database schema

Acceptance criteria:
- Only toolbar search is visible.
- Scopes route correctly.
- Grouped result snapshots are tested.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test search`
- `cargo test architecture_tests`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Grouped rows require a broad rewrite of Discover result selection or
  inspector navigation.
- Removing local Library search creates unacceptable loss of Library
  filtering behavior not covered by the ADR.
