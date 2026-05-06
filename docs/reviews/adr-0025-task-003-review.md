# ADR 0025 Task 003 Review: Control Style Boundary

## Reviewed Scope

- `src/ui/control_styles.rs`
- `src/ui/primitives/button.rs`
- `src/ui/composites/action_button.rs`
- `src/ui/mod.rs`
- `tests/architecture_tests.rs`

## Verdict

Pass.

Task 003 can be treated as complete. The next ADR 0025 implementation packet is
Task 003b, the screen button style sweep.

## Required Fixes

None.

## Architectural Review

- `ControlStyle` is a semantic role layer mapped to native
  `ui::primitives::Button` specs.
- `ActionButton` now uses `ControlStyle::MetadataAction` and no longer wraps
  `gpui_component::Button`.
- Pure role mapping tests cover metadata action, destructive action, and the
  shared compact mapping for toolbar/row actions.
- Native `Button` now accepts control-style specs and keeps the existing
  `action_button(...).disabled(...).danger().on_click(...)` call chains
  compiling without migrating screen render paths in this task.
- Architecture tests now ratchet unmarked direct `gpui_component::Button`
  usage and define the `CONTROL-COMPAT(reason): ...` marker path for Task 003b.

## Role Admission Examples Used

- `ToolbarIcon`: playlist sort/add controls listed in ADR 0025.
- `RowAction`: playlist row move/remove controls listed in ADR 0025.
- `Pill`: Discover filter controls listed in ADR 0025.
- `Ghost`: load-more/default/back controls listed in ADR 0025.
- `MetadataAction`: existing reusable `ActionButton` metadata inspector
  pattern.

## Tests Run

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo test`
- `cargo clippy --lib --tests -- -D warnings`

## Residual Risk

Task 003 intentionally leaves screen-level `gpui_component::Button` call sites
in place. The new architecture test prevents that debt from growing until Task
003b migrates or marks the remaining direct component-button uses.
