# ADR 0025 Task 008: Layout Boundary

## Status

Implemented.

## Task Goal

Move fixed layout geometry out of the style compatibility module and into the
named design-system layout boundary from the ideal architecture diagrams.

## Files To Inspect

- `docs/adr/0025-theme-icon-style-boundary.md`
- `docs/plans/adr-0025-visual-system-phase-plan.md`
- `docs/architecture/architecture-diagrams.md`
- `src/ui/style.rs`
- `src/ui/mod.rs`
- `src/app.rs`
- `src/app/bootstrap.rs`
- `src/app/tab_bar.rs`
- `src/library.rs`
- `src/search.rs`
- `src/ui/composites/split_pane.rs`
- `src/ui/composites/track_row.rs`
- `src/ui/icons.rs`
- `tests/architecture_tests.rs`

## Files Changed

- `src/ui/layouts.rs`
- `src/ui/mod.rs`
- `src/ui/style.rs`
- layout imports in screen and UI composite modules
- `tests/architecture_tests.rs`
- ADR 0025 docs and review docs

## Do Not Touch

- database migrations
- application commands, queries, and services
- visual dimensions themselves, unless required to preserve compilation

## Constraints

- Preserve current layout values and behavior.
- Keep this as a boundary move, not a redesign.
- Do not move spacing, radius, or typography constants in this slice.
- Add an architecture guard so callers do not reintroduce `ui::style::layout`.

## Implementation Summary

- Added `src/ui/layouts.rs` for fixed layout geometry.
- Removed `layout` from `src/ui/style.rs`.
- Updated screen and UI composite imports from `ui::style::layout` to
  `ui::layouts`.
- Added an architecture test that rejects the old `ui::style::layout`
  namespace.
- Updated ADR 0025 and the phase plan to show layout constants as their own
  design-system boundary.

## Acceptance Criteria

- [x] `src/ui/layouts.rs` owns fixed layout constants.
- [x] `src/ui/style.rs` no longer defines a `layout` module.
- [x] Screens and composites import layout constants from `ui::layouts`.
- [x] Architecture tests reject `ui::style::layout`.
- [x] Current layout values are preserved.

## Test Commands

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`

## Expected Final Summary Format

1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
