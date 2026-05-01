# ADR 0025 Task 004 Review: Typed Badge Role Migration

## Reviewed Artifact

- Task packet: `docs/tasks/adr-0025-task-004-badge-role-migration.md`
- Diff scope: typed entity badge colors, metadata provenance roles, screen
  badge call-site migration, architecture tests.

## Verdict

Pass.

## Required Fixes

- None.

## Optional Improvements

- The remaining `color::diff_missing()` in `src/search.rs` is a status-message
  error color, not metadata provenance. A later status-role task should migrate
  it into a typed status role instead of leaving it under the general color shim.
- `theme::badges` remains in `src/ui/theme.rs` as compatibility debt. Task 006
  should retire it once the broader `theme.rs` shim removal is in scope.

## Architectural Drift

- None. `src/library.rs` and `src/search.rs` now use `TagBadge`, `EntityKind`,
  and `ProvenanceRole` instead of screen-level `theme::badges` or loose
  diff glyph/color pairing.

## Missing Tests

- No new rendering snapshot tests were added. The regression coverage is via
  `tests/architecture_tests.rs`, which now holds screen `theme::badges` usage
  at zero and prevents loose provenance/diff helper growth.

## Merge Recommendation

- Mergeable after the standard verification commands pass.
