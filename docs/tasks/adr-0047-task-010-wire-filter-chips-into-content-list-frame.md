# ADR 0047 Task 010: Wire Filter Chips into ContentList Frame

Status: Implemented - visual confirmation closed - 2026-05-18.

## Goal

Wire the filter chip strip into the `ContentList` frame VM and
shell. Filter changes apply only to that frame's visible rows.
Dispatch `SetFrameFilter(frame_id, ContentFilter)` to mutate state.

## Prerequisite Resolution

2026-05-15 exploration found that the current `ContentList` frame is still the
ADR 0046 transitional whole-screen mount around Library/Search/Settings. There
is no real GPUI-free `ContentList` page VM that can own per-frame
`filter_state`, filter-aware row projection, and empty-filter state. The
escalation trigger applies; do not implement this task by rendering chips over
the transitional mount without row filtering.

Task 010a (`adr-0047-task-010a-content-list-page-vm-ownership`) landed the
GPUI-free `ContentListPageVm` ownership contract consumed by this task. The
Task 010 implementation wires only the Library-backed `ContentList` frame,
because Search and Settings still render through their transitional whole-screen
mounts. Search scope retirement remains Task 011; search-results inspector
routing remains Phase E.

## Files to Inspect

- `docs/plans/library-search-unification-plan.md`
- `docs/tasks/adr-0047-task-001-content-filter-vm.md`
- `docs/tasks/adr-0047-task-009-filter-chip-strip-composite.md`
- `src/view_models/workspace.rs`
- `src/view_models/library.rs` (ContentList page VM)
- `src/ui/shells/workspace.rs`
- `src/ui/shells/library/playlist_detail.rs` (row-rendering precedent)

## Files Likely to Change

- `src/view_models/library.rs` (consume `ContentListPageVm` from Task 010a)
- `src/library/app_impl.rs` (LibraryApp bridge for content-list filter state)
- `src/ui/shells/workspace.rs` (consume the optional strip slot)
- `src/app.rs` (frame-filter dispatch into the active Library content frame)
- `tests/architecture_tests.rs`

## Do Not Touch

- Backend / Musicindex / db
- Playback
- Toolbar global search (ADR 0043)

## Constraints

- Each content-showing frame VM owns its own `filter_state`. No
  global filter store.
- `SetFrameFilter(frame_id, ContentFilter)` is the only mutator.
- Row projection consumes `filter_state`; cached rows are filtered
  in place when possible.
- Default filter is `ContentFilter::All`.
- Empty filter result triggers the empty-state contract from task 004
  (re-use that contract where possible, or add a minimal one for
  content-list).

## Implementation Steps

1. Consume the Task 010a `ContentListPageVm` from `LibraryViewModel`.
2. Refresh the page VM row cache when the Library tree snapshot changes,
   preserving the selected frame-local filter.
3. Project Library tree rows through `ContentListPageVm.visible_row_ids()`.
4. Expose the VM-owned `FilterChipStripDisplay` through `LibraryApp`.
5. Add a workspace content-list filter slot and callback that passes through
   to `frame_shell` without importing Library/Search state.
6. Add `TopApp::set_frame_filter(frame_id, ContentFilter)` dispatch that
   validates the visible `ContentList` frame and mutates the active Library
   VM.
7. Architecture guards:
   - `ContentListPageVm` remains the owner of filter state and row projection.
   - Workspace shell passes the strip display and callback to `frame_shell`.
   - No global filter store.

## Acceptance Criteria

- [x] Library-backed `ContentList` frame renders the filter chip strip.
- [x] Filter changes apply to Library frame visible rows through
      `ContentListPageVm`.
- [x] Empty filter result renders a VM-owned empty-state notice.
- [x] Architecture guards record the contracts.
- [x] Operator visual confirmation: strip position, narrow collapse, and
      Library/Index empty-state behavior in the running UI.

## Implementation Notes

- `LibraryViewModel` owns a `ContentListPageVm` and refreshes its cached row
  set from the Library tree snapshot.
- `LibraryViewModel::tree_projection()` filters via
  `filter_tree_to_content_rows(&tree, &self.content_list_page)`, so frame
  chips are not decorative.
- `WorkspaceSlots` carries `content_list_filter_chip_strip` plus
  `on_content_list_filter_select`; `src/ui/shells/workspace.rs` still imports
  no screen, backend, or service modules.
- `TopApp::set_frame_filter(frame_id, ContentFilter)` validates the visible
  `ContentList` frame id before routing to the active Library VM.
- Search and Settings transitional mounts do not receive content-list chips in
  this task. Task 011 remains responsible for retiring the toolbar
  `GlobalSearchScope` only after frame-local filtering is sufficient for the
  search path.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test library
cargo test workspace
cargo test --test architecture_tests
cargo clippy -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/plans/library-search-unification-plan.md`
- `docs/tasks/adr-0047-task-001-content-filter-vm.md`
- `docs/tasks/adr-0047-task-009-filter-chip-strip-composite.md`
- `src/view_models/workspace.rs`
- `src/view_models/library.rs`
- `src/library/app_impl.rs`
- `src/app.rs`
- `src/ui/shells/workspace.rs`

Goal:
- Consume the Task 010a `ContentListPageVm`, render the Library-backed
  `ContentList` filter chip strip in frame chrome, and dispatch filter changes
  through `TopApp::set_frame_filter(frame_id, ContentFilter)`.

Constraints:
- No global filter store.
- Row projection consumes `ContentListPageVm` state.
- Do not attach chips to Search or Settings while they remain transitional
  whole-screen mounts.

Do not touch:
- Backend, db, Musicindex
- Toolbar global search

Acceptance criteria:
- Strip renders for the Library-backed `ContentList`; filter changes only that
  frame's rows.
- Empty-state notice renders for zero-row filters.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test library`
- `cargo test workspace`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- ContentList page VM cannot host filter state without splitting
  responsibilities (escalate).
