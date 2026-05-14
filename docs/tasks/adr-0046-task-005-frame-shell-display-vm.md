# ADR 0046 Task 005: Frame Shell Display View Model

Status: Proposed - 2026-05-14.

## Goal

Define the GPUI-free `FrameShellDisplay` contract that the shared
frame shell composite (task 006) will consume. The display contract
projects title, optional subtitle, optional status, back/forward
button display, close button display, action-menu display, and a
content-slot identifier.

## Files to Inspect

- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/tasks/adr-0046-task-001-workspace-model-types.md`
- `src/view_models/workspace.rs`
- `src/view_models/library.rs` (display-contract precedent)
- `src/ui/icons.rs` (icon catalog references)
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/view_models/workspace.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- Any `src/ui/*` rendering
- `src/library*`, `src/search*`, `src/app*`
- `src/db.rs`, playback engine

## Constraints

- Zero `gpui::*` imports.
- All public types document with `M-CANONICAL-DOCS`.
- Icon identity referenced as catalog tokens (`IconName::*`-style),
  never raw glyph strings.
- Back/forward/close buttons carry id, a11y label, and disabled flag.
- Builder pattern only if a struct reaches four or more params.
- Tests cover history-driven disabled states (back disabled when
  stack empty, forward disabled when stack empty).

## Implementation Steps

1. Add `FrameChromeButtonDisplay { id, a11y_label, disabled }`.
2. Add `FrameChromeMenuItemDisplay { id, label, a11y_label, disabled }`
   matching project menu-item shape.
3. Add `FrameShellDisplay`:
   - `frame_id: WorkspaceFrameId`
   - `title: String`
   - `subtitle: Option<String>`
   - `status: Option<String>`
   - `back: FrameChromeButtonDisplay`
   - `forward: FrameChromeButtonDisplay`
   - `close: Option<FrameChromeButtonDisplay>`
   - `action_menu_items: Vec<FrameChromeMenuItemDisplay>`
   - `content_slot_id: String`
4. Add `FrameShellDisplay::from_layout(frame, nav, allow_close)` or
   equivalent projector reading from `WorkspaceFrameState` +
   `FrameNavigationState`.
5. Unit tests cover: empty history yields back/forward disabled;
   single back entry enables back; close hidden when `allow_close=false`;
   title/subtitle/status pass through.
6. Architecture guard asserts `FrameShellDisplay` exists and stays in
   `src/view_models/workspace.rs`.

## Acceptance Criteria

- [ ] `FrameShellDisplay` and supporting display types compile and are
  documented.
- [ ] Unit tests cover boundary disabled states and field passthrough.
- [ ] No `gpui` imports in `src/view_models/workspace.rs`.
- [ ] Architecture guard records the contract name and module.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test workspace
cargo test --test architecture_tests
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/tasks/adr-0046-task-001-workspace-model-types.md`
- `src/view_models/workspace.rs`
- `src/view_models/library.rs` (display-contract style)
- `src/ui/icons.rs`
- `tests/architecture_tests.rs`

Goal:
- Add `FrameShellDisplay` and supporting display types in
  `src/view_models/workspace.rs` with a projector from
  `WorkspaceFrameState` + `FrameNavigationState`.

Constraints:
- GPUI-free module.
- Documented public types.
- Icon identity through catalog tokens.

Do not touch:
- `src/ui/*`
- Screen modules

Acceptance criteria:
- `FrameShellDisplay` compiles; unit tests pass.
- Disabled states track nav-stack boundaries.
- Architecture guard records the contract.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test workspace`
- `cargo test --test architecture_tests`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Display contract cannot be projected from frame nav state without
  GPUI types (escalate before adding `use gpui`).
