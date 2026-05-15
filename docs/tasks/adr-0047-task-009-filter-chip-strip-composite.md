# ADR 0047 Task 009: Filter Chip Strip Composite

Status: Implemented - 2026-05-15.

## Goal

Add a shared composite that renders `FilterChipStripDisplay` (task
001) inside frame chrome, with narrow-width pull-down collapse.
Composite reuses existing segmented-control and pull-down primitives.

## Files to Inspect

- `docs/plans/library-search-unification-plan.md`
- `docs/tasks/adr-0047-task-001-content-filter-vm.md`
- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/adr/0038-presentation-contract-enforcement.md`
- `src/view_models/workspace.rs`
- `src/ui/composites/frame_shell.rs`
- `src/ui/composites.rs`
- `src/ui/primitives/segmented_control.rs`
- `src/ui/primitives/pull_down.rs` (or equivalent)
- `src/ui/icons.rs`

## Files Likely to Change

- `src/ui/composites/filter_chip_strip.rs` (new)
- `src/ui/composites.rs` (re-export)
- `src/view_models/workspace.rs` (extend `FrameShellDisplay` to carry
  an optional `FilterChipStripDisplay`)
- `tests/architecture_tests.rs`

## Do Not Touch

- Screens (`src/library*`, `src/search*`, `src/app*`)
- Backend, db, playback

## Constraints

- Composite accepts `FilterChipStripDisplay` + an
  `on_select(ContentFilter)` callback.
- No raw `rgb(...)` / `px(...)` literals; consume tokens.
- No `.absolute()` / `.fixed()` / `.z_index(...)` /
  `gpui_component::popover` calls outside permitted primitives.
- Narrow-width pull-down uses the existing pull-down primitive; the
  trigger label shows the active filter.
- HIG: single-select; accent fill on selected chip; chrome surface
  tone on unselected.
- `FrameShellDisplay::filter_chip_strip: Option<FilterChipStripDisplay>`
  is the integration point; frame_shell renders the strip below the
  chrome header when present.

## Implementation Steps

1. Add `src/ui/composites/filter_chip_strip.rs` exporting
   `filter_chip_strip(display, slots) -> impl IntoElement`.
2. Implement segmented-control render path for wide widths.
3. Implement pull-down render path for narrow widths.
4. Re-export from `src/ui/composites.rs`.
5. Extend `FrameShellDisplay` with optional `filter_chip_strip` and
   project from frame VM.
6. Update `frame_shell` composite to render the strip below the
   chrome header when the field is present.
7. Architecture guards:
   - Composite uses tokens only.
   - Composite reuses existing primitives.
   - Frame shell renders the strip from the optional display field.

## Acceptance Criteria

- [x] Composite compiles and renders the segmented strip on wide
  widths.
- [x] Composite collapses to pull-down on narrow widths.
- [x] Frame shell renders the optional strip when present.
- [x] No raw color/spacing literal.
- [x] Architecture guards record the contracts.

## Implementation Notes

- `src/ui/composites/filter_chip_strip.rs` renders
  `FilterChipStripDisplay` through shared segmented-control and
  context-menu primitives.
- `FrameShellDisplay` carries the optional strip and `frame_shell`
  renders it when present.
- Visible filter behavior remains deferred to Task 010 because the
  current `ContentList` frame is still a transitional whole-screen
  mount.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/plans/library-search-unification-plan.md`
- `docs/tasks/adr-0047-task-001-content-filter-vm.md`
- `docs/adr/0033-hig-ui-architecture-governance.md`
- `src/view_models/workspace.rs`
- `src/ui/composites/frame_shell.rs`
- `src/ui/primitives/segmented_control.rs`
- `src/ui/primitives/pull_down.rs`

Goal:
- Add `filter_chip_strip` composite + extend `FrameShellDisplay`
  with optional strip field. Frame shell renders the strip when
  present. Narrow-width pull-down collapse.

Constraints:
- Tokens only.
- Reuse existing primitives.
- Single-select; HIG accent fill on selected.

Do not touch:
- Screens
- Backend, db, playback

Acceptance criteria:
- Composite + frame shell extension compile.
- Strip renders inline; narrow widths collapse to pull-down.
- Architecture guards lock contracts.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Pull-down primitive missing or wrong shape (escalate; do not
  hand-roll).
- Frame shell signature cannot host an optional strip without a
  broader contract change.
