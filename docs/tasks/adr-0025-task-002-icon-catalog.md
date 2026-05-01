# ADR 0025 Task 002: Semantic Icon Catalog

## Status

Implemented - 2026-05-01.

## Task Goal

Create a semantic icon catalog and migrate the highest-value duplicated icon
helpers behind it.

## Files To Inspect

- `docs/adr/0025-theme-icon-style-boundary.md`
- `src/ui/mod.rs`
- `src/search.rs`
- `src/library.rs`
- `src/app/playback_bar.rs`
- `src/ui/composites/now_playing_bar.rs`
- `src/ui/theme.rs`
- `src/ui/contrast.rs`

## Files Likely To Change

- `src/ui/icons.rs`
- `src/ui/mod.rs`
- `src/ui/contrast.rs`
- `src/search.rs`
- `src/library.rs`
- `src/ui/composites/now_playing_bar.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/application/**`
- service modules
- database migrations
- playlist, download, metadata, or playback command behavior

## Constraints

- Preserve current click behavior and hit target sizes.
- Migrate only icon rendering, not workflow logic.
- Icons must take semantic size/color roles.
- Brand/protocol colors must live inside the catalog, not screens.
- Brand/protocol icon colors must pass non-text contrast checks for their
  intended usage.
- Do not rely on Apple platform SF Symbols runtime.

## Implementation Steps

1. Add an `IconName` enum covering only migrated call sites.
2. Add rendering helpers or an `Icon` primitive facade that can produce the
   current SVG/image/text equivalent.
3. Move RSS and Nostr icon rendering behind the catalog first.
4. Move playback/status glyphs only where the existing component boundary makes
   the edit small.
5. Add contrast coverage for RSS/Nostr brand-color usages, using
   `src/ui/contrast.rs` or an equivalent focused test.
6. Add or update architecture tests to prevent new screen-level inline icon SVG
   helpers after migration.

## Acceptance Criteria

- [ ] RSS and Nostr icons render through `ui::icons`.
- [ ] Migrated icons preserve current colors, sizes, and click behavior.
- [ ] Brand/protocol icon colors have non-text contrast coverage.
- [ ] No workflow behavior changes.
- [ ] New screen-level inline icon SVG helpers are rejected or documented as
      temporary allowlist debt.

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
5. unresolved concerns

## Escalation Triggers

- Icon rendering requires a new asset pipeline.
- Migrating icons changes button size, alignment, or click targets.
- A screen-specific icon cannot be represented semantically.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0025-theme-icon-style-boundary.md`
- `src/ui/mod.rs`
- `src/search.rs`
- `src/library.rs`
- `src/ui/composites/now_playing_bar.rs`
- `tests/architecture_tests.rs`

Goal:
- Add a semantic icon catalog and migrate duplicated RSS/Nostr icon rendering
  behind it.

Constraints:
- Preserve current behavior, size, and click targets.
- Do not change workflows.
- Keep brand/protocol colors in the icon catalog.
- Add non-text contrast coverage for brand/protocol icon colors.

Do not touch:
- `src/application/**`
- service modules
- database migrations

Acceptance criteria:
- RSS/Nostr icons are rendered via `ui::icons`.
- Brand/protocol icon colors have contrast coverage.
- Architecture tests prevent obvious regression to new screen-level inline SVG.
- Existing focused tests pass.

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
5. unresolved concerns
