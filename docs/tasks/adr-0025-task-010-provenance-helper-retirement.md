# ADR 0025 Task 010: Provenance Helper Retirement

## Status

Implemented.

## Task Goal

Remove the final loose `style::color::diff_*` helper path so provenance and diff
color semantics are owned only by `ProvenanceRole`.

## Files To Inspect

- `docs/adr/0025-theme-icon-style-boundary.md`
- `docs/plans/adr-0025-visual-system-phase-plan.md`
- `src/ui/style.rs`
- `src/ui/composites/tag_badge.rs`
- `src/search.rs`
- `tests/architecture_tests.rs`

## Files Changed

- `src/search.rs`
- `src/ui/style.rs`
- `tests/architecture_tests.rs`
- ADR 0025 docs and review docs

## Do Not Touch

- application commands and services
- database migrations
- metadata comparison behavior

## Constraints

- Preserve existing visual color by using `ProvenanceRole::Missing`.
- Do not add a new status/provenance role unless the existing role does not fit.
- Remove `style::color::diff_*` only after all callers are gone.
- Tighten architecture tests to keep the helper count at zero.

## Implementation Summary

- Migrated the final Discover `color::diff_missing()` call site to
  `ProvenanceRole::Missing.color(cx)`.
- Removed `style::color::diff_match`, `diff_different`, and `diff_missing`.
- Tightened the screen provenance-diff helper baseline to zero.
- Added an architecture test that rejects reintroducing diff helpers in
  `ui::style`.

## Acceptance Criteria

- [x] No screen uses `color::diff_*`.
- [x] `ui::style` no longer defines `diff_*` color helpers.
- [x] `ProvenanceRole` owns diff color resolution.
- [x] Architecture tests enforce zero loose provenance helper usage.

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
