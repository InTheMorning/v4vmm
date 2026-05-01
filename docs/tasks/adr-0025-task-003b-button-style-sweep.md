# ADR 0025 Task 003b: Screen Button Style Sweep

## Status

Implemented - 2026-05-01.

## Task Goal

Migrate reusable screen-level `gpui_component::Button` style chains in
`app.rs`, `library.rs`, and `search.rs` to the native
`ui::primitives::Button` / `ControlStyle` boundary created by Task 003.

## Files To Inspect

- `docs/adr/0025-theme-icon-style-boundary.md`
- `docs/tasks/adr-0025-task-003-control-styles.md`
- `src/ui/control_styles.rs`
- `src/ui/primitives/button.rs`
- `src/ui/composites/action_button.rs`
- `src/app.rs`
- `src/library.rs`
- `src/search.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/app.rs`
- `src/library.rs`
- `src/search.rs`
- `src/ui/control_styles.rs`
- `src/ui/primitives/button.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/application/**`
- service modules
- database migrations
- workflow behavior for playback, download, metadata, playlists, or search
- unrelated layout refactors

## Constraints

- Preserve current labels, click handlers, disabled states, focus behavior, and
  layout.
- Migrate reusable styled button patterns to `ControlStyle`.
- Leave direct `gpui_component::Button` only when the native primitive cannot
  yet represent a required capability.
- Every remaining direct `gpui_component::Button` compatibility exception in a
  screen file must have a preceding or same-line
  `// CONTROL-COMPAT(reason): ...` marker.
- The architecture test must list every direct screen-file
  `gpui_component::Button` reference that lacks `CONTROL-COMPAT`.
- Do not add new `ControlStyle` roles unless they satisfy the admission rule:
  at least two unrelated call sites, or a state/contrast requirement that a
  generic chain cannot express.

## Implementation Steps

1. Inventory direct styled `gpui_component::Button` chains in `app.rs`,
   `library.rs`, and `search.rs`.
2. Group the inventory by role: primary, secondary, ghost, destructive,
   toolbar icon, row action, metadata action, pill/toggle, one-off, or
   compatibility debt.
3. Migrate reusable patterns to `ControlStyle`.
4. Add `CONTROL-COMPAT(reason): ...` markers for remaining direct
   `gpui_component::Button` style chains that cannot migrate yet.
5. Tighten architecture tests so unmarked direct screen-file
   `gpui_component::Button` usage fails with file/line output.
6. Keep the per-file diff reviewable. If `library.rs` or `search.rs` becomes
   too large for one diff, stop and split this task into file-scoped subtasks
   before editing further.

## Acceptance Criteria

- [ ] Reusable direct styled `gpui_component::Button` chains in `app.rs`,
      `library.rs`, and `search.rs` are migrated to `ControlStyle`.
- [ ] Remaining direct `gpui_component::Button` style chains have
      `CONTROL-COMPAT(reason): ...` markers.
- [ ] Architecture tests reject unmarked direct screen-file
      `gpui_component::Button` usage.
- [ ] The final report includes the inventory of direct button chains found,
      with per-call disposition: migrated, compatibility debt, or one-off.
- [ ] Current behavior, labels, disabled states, focus behavior, and layout are
      preserved.

## Test Commands

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo test`
- `cargo clippy --lib --tests -- -D warnings`

## Expected Final Summary Format

1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. inventory of direct `gpui_component::Button` chains found, with per-call
   disposition: migrated / compatibility debt / one-off
6. unresolved concerns

## Escalation Triggers

- A direct `gpui_component::Button` use cannot be migrated or marked without
  changing behavior.
- A needed `ControlStyle` role has only one call site and no state/contrast
  requirement.
- The sweep becomes too large to verify in one diff.
- Architecture tests cannot reliably distinguish native button usage from
  direct `gpui_component::Button` usage.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0025-theme-icon-style-boundary.md`
- `docs/tasks/adr-0025-task-003-control-styles.md`
- `src/ui/control_styles.rs`
- `src/ui/primitives/button.rs`
- `src/ui/composites/action_button.rs`
- `src/app.rs`
- `src/library.rs`
- `src/search.rs`
- `tests/architecture_tests.rs`

Goal:
- Migrate reusable screen-level `gpui_component::Button` style chains to
  `ControlStyle`, and mark any remaining direct usage as compatibility debt.

Constraints:
- Preserve current behavior, labels, disabled states, focus behavior, and
  layout.
- Do not change workflows.
- Use `CONTROL-COMPAT(reason): ...` for remaining direct
  `gpui_component::Button` compatibility exceptions.

Do not touch:
- `src/application/**`
- service modules
- database migrations
- workflow behavior
- unrelated layout refactors

Acceptance criteria:
- Reusable direct styled button chains are migrated.
- Remaining direct `gpui_component::Button` style chains are marked with
  `CONTROL-COMPAT(reason): ...`.
- Architecture tests reject unmarked direct screen-file
  `gpui_component::Button` usage.
- Final report includes the inventory and disposition of every direct button
  chain found.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo test`
- `cargo clippy --lib --tests -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. inventory of direct `gpui_component::Button` chains found, with per-call
   disposition: migrated / compatibility debt / one-off
6. unresolved concerns
