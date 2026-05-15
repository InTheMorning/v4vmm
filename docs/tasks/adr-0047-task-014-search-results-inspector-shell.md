# ADR 0047 Task 014: Search Results Inspector Shell

Status: Implemented - 2026-05-15.

## Goal

Render the tabbed `SearchResultsInspector` shell (Artists / Feeds /
Tracks) consuming `SearchResultsInspectorPageVm` from task 004. The
shell renders inside a `Detail` frame using `frame_shell` chrome and
the new filter chip strip from task 009.

## Files to Inspect

- `docs/plans/library-search-unification-plan.md`
- `docs/tasks/adr-0047-task-004-search-results-inspector-vm.md`
- `docs/tasks/adr-0047-task-009-filter-chip-strip-composite.md`
- `src/view_models/search_results.rs`
- `src/ui/composites/frame_shell.rs`
- `src/ui/composites/filter_chip_strip.rs`
- `src/ui/shells/workspace.rs`
- `src/ui/primitives/segmented_control.rs` (for tabs)

## Files Likely to Change

- `src/ui/shells/search_results_inspector.rs` (new)
- `src/ui/shells/workspace.rs` (mount the inspector)
- `tests/architecture_tests.rs`

## Do Not Touch

- Backend HTTP / Musicindex
- `src/db.rs`
- Playback
- Existing artist/feed/track entity composites (this task wires
  results into them, does not re-implement them)

## Constraints

- Tabs render via existing segmented-control or pill-tab primitive.
- Tab labels: `Artists`, `Feeds`, `Tracks`. Optional row count
  suffix (e.g., `Artists (12)`).
- Filter chip strip renders through `FrameShellDisplay.filter_chip_strip`
  (task 009).
- Active tab's paged result list consumes the existing track-row /
  feed-row / artist-row composites where available; otherwise route
  through shared composites in `src/ui/composites/` (the `ui_artist`,
  `ui_feed`, `ui_track` modules from `unify-discover-library-views.md`
  if/when they exist).
- Empty state: render `EmptyStateDisplay` from task 004 when active
  tab + filter yields zero rows.
- No raw glyph/color/spacing literals.

## Implementation Steps

1. Add `src/ui/shells/search_results_inspector.rs` exporting
   `render_search_results_inspector(vm, slots)`.
2. Render the tab strip at the top of the body.
3. Switch the body by `vm.tab`. Each tab renders its paged result
   list.
4. Empty state branch renders the `EmptyStateDisplay`.
5. Workspace shell mounts the inspector inside the `Detail` frame
   when its content kind is `SearchResults`.
6. Architecture guards:
   - `src/ui/shells/search_results_inspector.rs` does not import
     screen modules.
   - Tabs render through the existing primitive (no hand-rolled
     pill-tab chrome).

## Acceptance Criteria

- [x] Shell renders the three tabs and switches body content.
- [x] Filter chip strip renders via frame chrome.
- [x] Empty state renders when applicable.
- [x] No raw color/spacing literals.
- [x] Architecture guards lock the contracts.

## Implementation Notes

- Added `src/ui/shells/search_results_inspector.rs` as a shared shell that
  consumes `SearchResultsInspectorPageVm`, renders Artists / Feeds / Tracks
  through the existing `SegmentedControl`, and renders active paged rows with
  `ListRow`, `Thumbnail`, `Label`, and `TagBadge`.
- The shell uses `PagedListVm::peek_row` so render does not enqueue loads.
  Pending rows render a stable placeholder row; ready rows use the VM-owned
  display fields and source-membership labels.
- `SearchResultsInspectorPageVm::filter_chip_strip()` now projects
  `FilterChipStripDisplay::default_for_search_inspector`, keeping the filter
  contract GPUI-free.
- `WorkspaceSlots` now accepts a Detail-frame filter strip and filter callback,
  allowing search-results filters to render through `frame_shell` chrome when
  Task 015 mounts the inspector.
- This task is capability-only for visible routing: no global search-submit or
  saved-search command path was changed. Visual proof is deferred to Task 015,
  which owns the first visible mount.

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
- `docs/tasks/adr-0047-task-009-filter-chip-strip-composite.md`
- `src/view_models/search_results.rs`
- `src/ui/composites/frame_shell.rs`
- `src/ui/composites/filter_chip_strip.rs`
- `src/ui/shells/workspace.rs`
- `src/ui/primitives/segmented_control.rs`

Goal:
- Render the tabbed `SearchResultsInspector` shell consuming the
  paged tabbed VM, with filter chip strip in frame chrome.

Constraints:
- Tokens only.
- Reuse primitives + existing entity composites.

Do not touch:
- Backend / Musicindex / db
- Playback
- Entity composites (consume, do not re-implement)

Acceptance criteria:
- Shell renders three tabs + filter chip strip + empty state.
- Architecture guards record contracts.

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

- Shared entity composites missing or wrong shape for paged result
  rendering (escalate; lift composites first per unify-discover-
  library-views plan).
