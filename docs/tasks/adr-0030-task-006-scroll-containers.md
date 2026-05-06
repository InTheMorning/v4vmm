# ADR 0030 Task 006: Scroll Containers

## Status

Implemented.

## Goal

Make Library detail, Discovery inspector, and settings panes scroll reliably in
bounded GPUI flex layouts.

## Files To Inspect

- `docs/adr/0030-discovery-library-ui-fixes.md`
- `docs/plans/discovery-library-ui-fixes.md`
- `src/ui/composites/release_detail_surface.rs`
- `src/library.rs`
- `src/search.rs`
- `src/app.rs`

## Files Likely To Change

- `src/ui/composites/release_detail_surface.rs`
- `src/library.rs`
- `src/app.rs`
- possibly `src/search.rs` if an ancestor lacks bounded height

## Do Not Touch

- Data models, services, metadata, compare, download, playback, or playlist
  behavior.
- Visual styling beyond scroll-container sizing.

## Constraints

- Prefer `flex_1().min_h_0().overflow_y_scroll()` for scrollable children in
  flex-column parents.
- Avoid nesting same-orientation scroll views.
- If a leaf fix does not work, fix the missing bounded-height ancestor.

## Implementation Steps

1. [x] Update the scrollable branch in `ReleaseDetailSurface`.
2. [x] Update direct detail scroll containers that use `size_full()` in flex
   contexts.
3. [x] Build and run focused tests.
4. [ ] Perform manual smoke for wheel, scrollbar, and keyboard scrolling when a
   GUI run is available.

## Implementation Summary

- Changed `ReleaseDetailSurface` scrollable mode to use bounded flex sizing:
  `flex_1().min_h_0().min_w_0().overflow_y_scroll()`.
- Added bounded flex ancestors for Library and Discovery split-pane content.
- Updated Library artist, playlist, and track detail scroll leaves to use
  bounded flex sizing.
- Added `min_h_0()` to Library, Discovery results, and inspector scroll leaves.
- Updated Settings scrolling to use bounded flex sizing inside the active tab.

## Verification

Green on 2026-05-01:

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

Manual GUI smoke was not run in this packet.

## Acceptance Criteria

- Library artist, album/feed, playlist, and track details scroll to the end.
- Discovery inspector details scroll to the end.
- Settings scrolls to the end.
- The changes do not introduce nested vertical scroll views.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
cargo clippy -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0030-discovery-library-ui-fixes.md`
- `docs/tasks/adr-0030-task-006-scroll-containers.md`
- `src/ui/composites/release_detail_surface.rs`
- `src/library.rs`
- `src/search.rs`
- `src/app.rs`

Goal:
- Fix broken vertical scrolling in detail and settings panes.

Constraints:
- Use bounded flex scroll children.
- Avoid nested same-axis scroll views.
- Do not change unrelated layout or behavior.

Do not touch:
- `src/db.rs`
- `src/metadata.rs`
- service modules

Acceptance criteria:
- Target panes can scroll to the end with normal macOS input paths.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
