# ADR 0025 Task 006: Retire Theme Compatibility Shim

## Status

Planned.

## Task Goal

Retire or sharply narrow `src/ui/theme.rs` after the semantic theme, icon,
control-style, and badge boundaries are in use.

## Files To Inspect

- `docs/adr/0025-theme-icon-style-boundary.md`
- `docs/plans/adr-0025-visual-system-phase-plan.md`
- `src/ui/theme.rs`
- `src/ui/tokens.rs`
- `src/ui/theme_profile.rs`
- `src/ui/icons.rs`
- `src/ui/control_styles.rs`
- `src/search.rs`
- `src/library.rs`
- `src/app.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/ui/theme.rs`
- `src/ui/tokens.rs`
- `src/ui/mod.rs`
- `src/search.rs`
- `src/library.rs`
- `src/app.rs`
- `tests/architecture_tests.rs`
- `docs/adr/0025-theme-icon-style-boundary.md`
- `docs/plans/adr-0025-visual-system-phase-plan.md`
- `docs/reviews/adr-0025-review-checklist.md`

## Do Not Touch

- `src/application/**`
- service modules
- database migrations
- playback/download/metadata workflow behavior

## Constraints

- Do not start this task until Phases 1-5 are complete or explicitly deferred.
- Migrated screen files must have zero `theme::color::*`, `theme::badges`, and
  `theme::glyphs` call sites before the deprecated namespace ban becomes
  unconditional.
- `theme::glyphs` should have zero remaining callers before this task deletes
  or narrows the deprecated namespace.
- Remaining layout constants may stay only if they are documented as fixed
  geometry or moved to token/layout roles.
- Preserve current behavior and layout.

## Implementation Steps

1. Audit `app.rs`, `library.rs`, `search.rs`, and app submodules for
   `theme::color::*`, `theme::badges`, and `theme::glyphs`.
2. Migrate any remaining visual helper calls to tokens, theme profiles, icons,
   control styles, or typed badge/status roles.
3. Reduce `theme.rs` to layout constants only, or move remaining constants to a
   clearer token/layout module if the local pattern supports it.
4. Make the architecture-test ban on deprecated visual namespaces
   unconditional for migrated screen files.
5. Update ADR 0025 and the phase plan status if this closes the ADR.

## Acceptance Criteria

- [ ] `app.rs`, `library.rs`, and `search.rs` have zero
      `theme::color::*`, `theme::badges`, and `theme::glyphs` call sites.
- [ ] `theme::glyphs` does not exist.
- [ ] `theme.rs` contains only documented layout/fixed-geometry compatibility
      constants, or is removed.
- [ ] Architecture tests unconditionally reject deprecated visual helper usage
      in migrated screen files.
- [ ] Current visual behavior and layout are preserved.

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

- A remaining `theme.rs` helper is needed by a primitive/composite and has no
  clear token/profile/control replacement.
- Retiring a helper would require visual redesign rather than mechanical
  migration.
- Architecture tests would need a broad allowlist after this task.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0025-theme-icon-style-boundary.md`
- `docs/plans/adr-0025-visual-system-phase-plan.md`
- `src/ui/theme.rs`
- `src/ui/tokens.rs`
- `src/ui/theme_profile.rs`
- `src/ui/icons.rs`
- `src/ui/control_styles.rs`
- `src/search.rs`
- `src/library.rs`
- `src/app.rs`
- `tests/architecture_tests.rs`

Goal:
- Retire or sharply narrow the legacy `theme.rs` compatibility shim.

Constraints:
- Preserve current behavior and layout.
- Do not touch application workflows, services, or database migrations.
- Migrated screen files must have zero deprecated visual helper call sites.

Do not touch:
- `src/application/**`
- service modules
- database migrations
- playback/download/metadata workflow behavior

Acceptance criteria:
- `app.rs`, `library.rs`, and `search.rs` have zero `theme::color::*`,
  `theme::badges`, and `theme::glyphs` call sites.
- `theme::glyphs` does not exist.
- Architecture tests unconditionally reject deprecated helper usage in migrated
  screen files.
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
5. unresolved concerns
