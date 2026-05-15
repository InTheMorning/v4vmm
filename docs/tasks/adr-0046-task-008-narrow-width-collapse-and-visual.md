# ADR 0046 Task 008: Narrow-Width Collapse and Phase 3 Visual

Status: Implemented - 2026-05-15.

## Goal

Add narrow-width collapse rules for the workspace, architecture guards
for Phase 3 invariants, and visual proof (light + dark) for the
default and collapsed layouts. Optional frames collapse before primary
nav/search at narrow widths. Collapsed frames remain accessible via
toolbar menu or keybinding.

## Files to Inspect

- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/tasks/adr-0046-task-007-workspace-layout-render.md`
- `src/ui/layouts.rs`
- `src/ui/shells/workspace.rs`
- `tests/architecture_tests.rs`
- `docs/reviews/adr-0044-review-checklist.md` (checklist precedent)

## Files Likely to Change

- `src/ui/layouts.rs`
- `src/ui/shells/workspace.rs`
- `tests/architecture_tests.rs`
- `docs/reviews/adr-0046-review-checklist.md`

## Do Not Touch

- Page VM internals
- Toolbar search submit behavior (per ADR 0043, primary actions stay
  visible)
- Playback engine, db

## Constraints

- Define collapse breakpoints as constants in `src/ui/layouts.rs`
  (e.g., `WORKSPACE_QUEUE_COLLAPSE_BREAKPOINT`,
  `WORKSPACE_SECONDARY_DETAIL_COLLAPSE_BREAKPOINT`).
- Collapse order: optional frames first
  (`QueueNowPlaying`, secondary detail), then secondary content. Never
  collapse `SourceList` below its primary state.
- Apple HIG: collapsed frames must remain reachable through a menu
  (frame add command) or keybinding. Toolbar primary actions remain
  visible regardless.
- Architecture guards must enforce Phase 3 invariants without
  duplicating Phase 2 guards.
- Visual proof must cover default layout, narrow with queue
  collapsed, narrow with queue + secondary detail collapsed, in both
  themes.

## Implementation Steps

1. Add collapse breakpoint constants to `src/ui/layouts.rs`.
2. Update `src/ui/shells/workspace.rs` to hide frames whose
   eligibility allows collapse when the window is below the
   relevant breakpoint.
3. Show a compact restore affordance (menu item or status hint) for
   each collapsed frame.
4. Add architecture guards:
   - `src/ui/shells/workspace.rs` references each
     `WORKSPACE_*_COLLAPSE_BREAKPOINT` constant.
   - Frame chrome (frame_shell) renders Back/Forward, not the
     toolbar.
   - Toolbar still renders global search (per ADR 0043 guards).
5. Update `docs/reviews/adr-0046-review-checklist.md` with Phase 3
   readiness, visual proof, and merge recommendation.
6. Capture visual proof per the checklist after manual review.

## Acceptance Criteria

- [x] Breakpoint constants defined and consumed by the workspace
  shell.
- [x] Optional frames collapse before primary nav/search.
- [x] Collapsed frames remain accessible via menu/keybinding.
- [x] Architecture guards lock the breakpoint usage and frame-chrome
  ownership of Back/Forward.
- [x] Review checklist records pass/fail per gate; visual proof
  attached or referenced.

## Implementation Notes

- Implemented as part of ADR 0046 Phase 3/4 workspace visual follow-up.
- Final visual evidence and user confirmations are recorded in
  `docs/reviews/adr-0046-review-checklist.md`.

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
- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/tasks/adr-0046-task-007-workspace-layout-render.md`
- `src/ui/layouts.rs`
- `src/ui/shells/workspace.rs`
- `tests/architecture_tests.rs`
- `docs/reviews/adr-0044-review-checklist.md`

Goal:
- Add narrow-width collapse rules, Phase 3 architecture guards, and
  the Phase 3 visual-readiness checklist.

Constraints:
- Collapse optional frames before primary nav/search.
- Collapsed frames stay reachable via menu/keybinding.
- Toolbar primary actions remain visible.

Do not touch:
- Page VM internals
- Toolbar search submit behavior
- `src/db.rs`, playback engine

Acceptance criteria:
- Breakpoints land and are referenced by workspace shell.
- Architecture guards confirm frame chrome owns Back/Forward.
- Review checklist exists and records readiness.

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

- Visual proof reveals collapse rules hide a primary global action.
- Collapsed-frame restore affordance cannot live in the toolbar
  without violating ADR 0043 (escalate to design a keybinding-only
  fallback).
