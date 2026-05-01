# ADR 0025 Task 009: Status Role Boundary

## Status

Implemented.

## Task Goal

Move general status color/glyph semantics out of the style compatibility module
and into typed UI visual roles.

## Files To Inspect

- `docs/adr/0025-theme-icon-style-boundary.md`
- `docs/plans/adr-0025-visual-system-phase-plan.md`
- `src/ui/style.rs`
- `src/ui/composites/tag_badge.rs`
- `src/ui/composites/mod.rs`
- `src/library.rs`
- `src/search.rs`
- `tests/architecture_tests.rs`

## Files Changed

- `src/ui/composites/tag_badge.rs`
- `src/ui/composites/mod.rs`
- `src/ui/style.rs`
- `src/library.rs`
- `src/search.rs`
- `tests/architecture_tests.rs`
- ADR 0025 docs and review docs

## Do Not Touch

- application commands and services
- database migrations
- status copy or workflow behavior

## Constraints

- Preserve existing status glyphs and colors.
- Keep status color and glyph semantics paired in one typed role.
- Remove the old `style::color::status_*` helpers if all callers migrate.
- Add an architecture guard so `StatusRole` does not return to `ui::style`.

## Implementation Summary

- Added `StatusRole` to `ui::composites::tag_badge` beside `EntityKind` and
  `ProvenanceRole`.
- Re-exported `StatusRole` from `ui::composites`.
- Migrated Library and Discover status call sites to `StatusRole`.
- Removed `style::color::status_success`, `status_warning`, and
  `status_danger`.
- Added an architecture test that rejects reintroducing status roles in
  `ui::style`.

## Acceptance Criteria

- [x] `StatusRole` is exported from typed UI visual roles.
- [x] Status glyph and color token semantics resolve together.
- [x] Screen status messages use `StatusRole`.
- [x] `ui::style` does not define `StatusRole` or `style::color::status_*`.
- [x] Architecture tests enforce the boundary.

## Test Commands

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo test ui::composites::tag_badge`
- `cargo clippy --lib --tests -- -D warnings`

## Expected Final Summary Format

1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
