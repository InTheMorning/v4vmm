# ADR 0047 Task 013: Frame Shell Breadcrumb Render

Status: Proposed - 2026-05-14.

## Goal

Extend `frame_shell` composite (ADR 0046) to render breadcrumbs from
`BreadcrumbDisplay` (task 012) with middle-ellipsis truncation. Back
chevron from ADR 0046 remains; breadcrumb is an additive chrome row.

## Files to Inspect

- `docs/plans/library-search-unification-plan.md`
- `docs/tasks/adr-0047-task-012-frame-breadcrumb-vm.md`
- `docs/adr/0046-workspace-frame-architecture.md`
- `src/view_models/workspace.rs`
- `src/ui/composites/frame_shell.rs`
- `src/ui/icons.rs`

## Files Likely to Change

- `src/view_models/workspace.rs` (extend `FrameShellDisplay` with
  optional `breadcrumb`)
- `src/ui/composites/frame_shell.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- Screens
- Backend, db, playback

## Constraints

- `FrameShellDisplay::breadcrumb: Option<BreadcrumbDisplay>`.
- Breadcrumb segments render as text + chevron separators. Current
  segment is dimmed or visually distinct.
- Clicking a non-current segment dispatches `BreadcrumbSelect(id)`.
- Middle-ellipsis truncation: when width is constrained, hide middle
  segments and render `…` between origin and current.
- No raw `rgb()` / `px()` literals; consume tokens.
- HIG: breadcrumb path-bar pattern.

## Implementation Steps

1. Extend `FrameShellDisplay` with `breadcrumb: Option<
   BreadcrumbDisplay>`.
2. Extend `FrameShellSlots` with an `on_breadcrumb_select` callback.
3. Render the breadcrumb row in `frame_shell` between the title row
   and the content slot (or as a sub-row of the title row).
4. Implement middle-ellipsis collapse when width constraints apply.
5. Architecture guards:
   - `frame_shell` renders breadcrumb from the optional display.
   - No raw chrome-API call introduced.
   - No screen module touched.

## Acceptance Criteria

- [ ] `frame_shell` renders breadcrumbs when display present.
- [ ] Middle-ellipsis truncation works at narrow widths.
- [ ] Clicking a segment dispatches the callback.
- [ ] Back chevron and other chrome controls unchanged.
- [ ] Architecture guard locks the contract.

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
- `docs/tasks/adr-0047-task-012-frame-breadcrumb-vm.md`
- `docs/adr/0046-workspace-frame-architecture.md`
- `src/view_models/workspace.rs`
- `src/ui/composites/frame_shell.rs`

Goal:
- Extend `frame_shell` composite with optional breadcrumb render and
  middle-ellipsis truncation.

Constraints:
- Tokens only.
- HIG path-bar pattern.
- Additive: existing chrome unchanged.

Do not touch:
- Screens
- Backend, db, playback

Acceptance criteria:
- Composite renders breadcrumbs when display present; truncation
  works; click dispatches callback.
- Architecture guard records contract.

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

- `FrameShellDisplay` cannot host the optional breadcrumb without a
  signature change that breaks ADR 0046 task 005/006 guards.
