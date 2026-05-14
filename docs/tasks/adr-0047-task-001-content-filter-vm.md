# ADR 0047 Task 001: ContentFilter VM + FilterChipStripDisplay

Status: Implemented - 2026-05-14.

## Goal

Introduce the GPUI-free `ContentFilter` enum and `FilterChipStripDisplay`
display contract that downstream phases consume. No visible UI change.

## Files to Inspect

- `docs/adr/0047-library-search-unification.md`
- `docs/plans/library-search-unification-plan.md`
- `docs/plans/adr-0047-library-search-unification-phase-plan.md`
- `src/view_models/workspace.rs` (frame display-contract precedent)
- `src/view_models/library.rs` (VM doc/style precedent)
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/view_models/workspace.rs` (add `ContentFilter`,
  `FilterChipStripDisplay`)
- `tests/architecture_tests.rs`

## Do Not Touch

- Any `src/ui/*` rendering
- `src/library*`, `src/search*`, `src/app*`
- `src/db.rs`, playback engine

## Constraints

- No `gpui::*` imports.
- `M-CANONICAL-DOCS` on every public type and function.
- `ContentFilter` is a typed enum (`All`, `Library`, `Index`); invalid
  states unrepresentable.
- `FilterChipStripDisplay` carries: `id: String`,
  `options: Vec<FilterChipOption>`, `selected: ContentFilter`,
  `narrow_collapse_to_pulldown: bool`.
- `FilterChipOption { value: ContentFilter, label: &'static str,
  a11y_label: &'static str, disabled: bool }`.
- Builder pattern if the display reaches four or more params.
- Derive `Debug`, `Clone`, `Eq`, `PartialEq` where reasonable. No
  smart pointers in public fields.

## Implementation Steps

1. Define `ContentFilter` in `src/view_models/workspace.rs` with
   variants `All`, `Library`, `Index`.
2. Define `FilterChipOption` and `FilterChipStripDisplay`.
3. Add `FilterChipStripDisplay::default_for_content_list()` and
   `::default_for_search_inspector()` constructors so callers do not
   hand-roll the option list.
4. Unit tests covering: default option order (`All`, `Library`,
   `Index`); selected variant round-trip; narrow-collapse flag
   passthrough.
5. Architecture guard: assert `ContentFilter` and
   `FilterChipStripDisplay` live in `src/view_models/workspace.rs`
   and that the module remains GPUI-free.

## Acceptance Criteria

- [ ] `ContentFilter` and `FilterChipStripDisplay` exist and compile.
- [ ] Public types document summary + applicable sections.
- [ ] Unit tests cover default constructors and field passthrough.
- [ ] Architecture guard confirms placement and GPUI-free constraint.
- [ ] No `src/ui/*` or screen module changed.

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
- `docs/plans/library-search-unification-plan.md`
- `docs/plans/adr-0047-library-search-unification-phase-plan.md`
- `src/view_models/workspace.rs`
- `src/view_models/library.rs` (style precedent)
- `tests/architecture_tests.rs`

Goal:
- Add `ContentFilter` enum and `FilterChipStripDisplay` to
  `src/view_models/workspace.rs`. Provide default constructors for
  content-list and search-inspector usage.

Constraints:
- GPUI-free.
- Documented public types.
- Enum models filter state.

Do not touch:
- Any `src/ui/*`
- Screens

Acceptance criteria:
- Types compile with docs and unit tests.
- Architecture guard records placement and GPUI-free constraint.

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

- A filter variant cannot be expressed without GPUI types (signals
  a layering error).
