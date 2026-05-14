# ADR 0046 Task 012: Frame Add/Remove and Layout Persistence

Status: Proposed - 2026-05-14.

## Goal

Add workspace VM operations for adding and removing frames with
deterministic focus invariants, and persist the workspace layout in
`config.toml`. No schema migration. Old configs load with the default
layout.

## Files to Inspect

- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/tasks/adr-0046-task-001-workspace-model-types.md`
- `docs/tasks/adr-0046-task-007-workspace-layout-render.md`
- `src/view_models/workspace.rs`
- `src/config.rs`
- `src/ui/shells/workspace.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/view_models/workspace.rs`
- `src/config.rs`
- `src/ui/shells/workspace.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- Database schema
- Playback engine
- Page VM internals

## Constraints

- Layout serialization is additive: missing fields fall back to
  default; unknown fields ignored.
- Focus invariant: workspace always has at least one frame and
  exactly one focused frame.
- Add/remove operations return `Result<_, WorkspaceLayoutError>`; do
  not panic on invariant violation.
- Removal focuses the left sibling, else the first remaining frame.
- Addition appends to the focused region with a caller-specified
  kind.
- Persistence happens on layout change and on app shutdown. Loading
  happens on startup; malformed config falls back to default with a
  warning log.

## Implementation Steps

1. Add `WorkspaceLayoutError` enum (e.g.,
   `FrameNotFound`, `LastFrameRemoval`).
2. Add `WorkspaceLayout::add_frame(kind) -> Result<WorkspaceFrameId,
   WorkspaceLayoutError>`.
3. Add `WorkspaceLayout::remove_frame(id) -> Result<(),
   WorkspaceLayoutError>` enforcing the focus invariant.
4. Extend `Config` with a `workspace_layout` optional struct that
   captures frame kinds + order; missing field uses default.
5. Add `WorkspaceLayout::to_config()` + `WorkspaceLayout::from_config()`
   conversions.
6. Wire load on `TopApp` startup; wire save on layout mutation.
7. Unit tests cover: add appends, remove preserves focus, removing
   last frame returns `Err`, save/load round-trip, malformed config
   falls back.
8. Architecture guard asserts `Config` references
   `workspace_layout` and that workspace ops return `Result`.

## Acceptance Criteria

- [ ] Add/remove operations return `Result` with documented errors.
- [ ] Focus invariants verified by unit tests.
- [ ] `config.toml` round-trip works; malformed config falls back to
  default without crash.
- [ ] Architecture guard records the persistence wiring.
- [ ] No schema migration.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test workspace
cargo test config
cargo test --test architecture_tests
cargo clippy -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/tasks/adr-0046-task-001-workspace-model-types.md`
- `docs/tasks/adr-0046-task-007-workspace-layout-render.md`
- `src/view_models/workspace.rs`
- `src/config.rs`
- `src/ui/shells/workspace.rs`

Goal:
- Implement add/remove operations with focus invariants and
  workspace layout persistence in `config.toml`.

Constraints:
- Additive config shape; missing/unknown fields tolerated.
- Result-returning operations; no panics on invariant violation.
- Removal focuses left sibling, else first remaining frame.

Do not touch:
- Database schema
- Playback engine
- Page VM internals

Acceptance criteria:
- Add/remove return `Result`; unit tests verify invariants.
- `config.toml` round-trip; malformed config falls back.
- Architecture guard records persistence wiring.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test workspace`
- `cargo test config`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Config shape outgrows TOML quickly (escalate to revisit
  ADR 0046 open-question 4 before adding a preferences table).
- Focus invariant cannot be expressed without GPUI-side coordination
  (signals frame ownership needs adjustment).
