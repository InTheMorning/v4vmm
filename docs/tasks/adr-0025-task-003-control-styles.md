# ADR 0025 Task 003: Control Style Boundary

## Status

Implemented - 2026-05-01.

## Task Goal

Introduce reusable control style roles as the public face of
`ui::primitives::Button`, migrate `ActionButton`, and add pure role mapping
tests. The screen-level button style-chain sweep is split into
`docs/tasks/adr-0025-task-003b-button-style-sweep.md`.

## Files To Inspect

- `docs/adr/0025-theme-icon-style-boundary.md`
- `src/ui/composites/action_button.rs`
- `src/ui/primitives/button.rs`
- `src/ui/sizable_bridge.rs`
- `src/ui/theme.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/ui/control_styles.rs`
- `src/ui/mod.rs`
- `src/ui/primitives/button.rs`
- `src/ui/composites/action_button.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/app.rs`
- `src/library.rs`
- `src/search.rs`
- command/query/event behavior
- service modules
- database migrations
- unrelated layout refactors

## Constraints

- Preserve current behavior and labels.
- Do not migrate screen call sites in this task.
- `ActionButton` should become the first consumer.
- `ControlStyle` maps to `ui::primitives::Button`; it must not wrap
  `gpui_component::Button` as a parallel vocabulary.
- Control style roles must be semantic and reusable, not one-off wrappers.
- A new `ControlStyle` role requires at least two unrelated screens/composites
  using the same pattern, or a state/contrast requirement that a generic chain
  cannot express.
- Define the compatibility-debt mechanism for later direct
  `gpui_component::Button` exceptions: a preceding or same-line
  `// CONTROL-COMPAT(reason): ...` marker, enforced by architecture tests.
- Avoid broad visual changes unless needed for contrast correctness.

## Implementation Steps

1. Add a control-style boundary that maps roles onto `ui::primitives::Button`.
2. Add a pure role-to-token/variant mapping so focused tests can validate the
   contract without GPUI rendering.
3. Rebuild `ActionButton` through `ControlStyle::MetadataAction` or an
   equivalent named helper.
4. Add tests for pure role mapping.
5. Add architecture-test support for `CONTROL-COMPAT(reason):` so Task 003b can
   ratchet direct `gpui_component::Button` usage without inventing a new
   mechanism. The test should list every direct screen-file
   `gpui_component::Button` reference that lacks the marker.
6. Document initial role admission examples in ADR 0025 or this task before
   adding the role:
   `ToolbarIcon` (`src/library.rs:1724`, `src/library.rs:1734`),
   `RowAction` (`src/library.rs:2674`, `src/library.rs:2696`),
   `Pill` (`src/search.rs:1723`, `src/search.rs:2251`), and
   `Ghost` (`src/search.rs:1780`, `src/app.rs:563`).

## Acceptance Criteria

- [x] `ActionButton` depends on the shared control-style boundary.
- [x] `ControlStyle` maps to `ui::primitives::Button`.
- [x] Pure role mapping tests exist.
- [x] `CONTROL-COMPAT(reason):` marker semantics are documented and enforced by
      architecture-test support for later screen-sweep work.
- [x] Initial non-obvious role examples are documented before roles are added.
- [x] No behavior changes.
- [x] No new one-off control style roles are introduced.

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
5. role admission examples used
6. unresolved concerns

## Escalation Triggers

- `ui::primitives::Button` cannot represent a required button behavior without
  losing accessibility, focus, disabled, or click behavior.
- A role name would encode a single screen instead of reusable intent.
- Architecture-test support cannot distinguish direct `gpui_component::Button`
  compatibility exceptions from native `ui::primitives::Button` usage.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0025-theme-icon-style-boundary.md`
- `src/ui/composites/action_button.rs`
- `src/ui/primitives/button.rs`
- `src/ui/sizable_bridge.rs`
- `tests/architecture_tests.rs`

Goal:
- Add reusable control style roles and migrate `ActionButton` to the new
  boundary. Do not migrate screen call sites in this task.

Constraints:
- Preserve current behavior and labels.
- Map `ControlStyle` to `ui::primitives::Button`.
- Define the `CONTROL-COMPAT(reason):` marker and architecture-test support for
  later direct `gpui_component::Button` exceptions.
- Keep roles semantic and reusable.

Do not touch:
- `src/app.rs`
- `src/library.rs`
- `src/search.rs`
- `src/application/**`
- service modules
- database migrations
- unrelated layout code

Acceptance criteria:
- `ActionButton` is implemented through the shared control-style boundary.
- `ControlStyle` maps to `ui::primitives::Button`.
- Pure role mapping tests exist.
- `CONTROL-COMPAT(reason):` marker semantics are ready for Task 003b.
- Current behavior is unchanged.
- Verification commands pass.

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
5. role admission examples used
6. unresolved concerns
