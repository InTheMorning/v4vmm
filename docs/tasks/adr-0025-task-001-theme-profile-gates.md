# ADR 0025 Task 001: Theme Profile Contract And Gates

## Status

Planned.

## Task Goal

Add the dormant theme-profile boundary and architecture gates without changing
current visual behavior.

## Files To Inspect

- `docs/adr/0025-theme-icon-style-boundary.md`
- `docs/plans/adr-0025-visual-system-phase-plan.md`
- `src/ui/tokens.rs`
- `src/ui/theme.rs`
- `src/ui/theme_bridge.rs`
- `src/app/bootstrap.rs`
- `src/ui/contrast.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/ui/theme_profile.rs`
- `src/ui/mod.rs`
- `src/ui/theme_bridge.rs`
- `src/ui/theme.rs`
- `src/ui/contrast.rs`
- `src/app/bootstrap.rs`
- `tests/architecture_tests.rs`
- `docs/plans/adr-0025-visual-system-phase-plan.md`
- `docs/tasks/adr-0025-task-001-theme-profile-gates.md`

## Do Not Touch

- `src/library.rs`
- `src/search.rs`
- `src/app.rs`
- `src/application/**`
- database migrations

## Constraints

- Preserve current visual behavior.
- Do not migrate call sites in this task.
- Do not introduce user-facing settings yet.
- Keep the new type GPUI-light; only bridge code should need `App`.
- Do not add arbitrary custom color input.
- `theme_bridge::install_theme` must take `ThemeProfile`, not leave the
  signature decision to the implementer.
- High-contrast profiles must have contrast tests in this task.
- Delete `theme::glyphs`; it has no callers and should not be migrated.

## Implementation Steps

1. Add a `ThemeProfile` type with at least `System`, `Dark`, `Light`,
   `HighContrastDark`, and `HighContrastLight`.
2. Add role-resolution methods only where they are needed for theme
   installation or tests.
3. Change `theme_bridge::install_theme` to
   `install_theme(profile: ThemeProfile, scale: ScaleFactor, cx: &mut App)`.
4. Add `ThemeProfile::appearance()` or an equivalent internal resolver and
   update both `src/app/bootstrap.rs` call sites to pass `ThemeProfile::Dark`.
5. Extend `src/ui/contrast.rs` so `HighContrastDark` and `HighContrastLight`
   have contrast-matrix coverage before they can be exposed in Settings.
6. Delete `theme::glyphs` from `src/ui/theme.rs`.
7. Extend architecture tests so new screen files cannot add deprecated visual
   helper usage once replacement phases begin. Keep existing call sites
   allowlisted if needed.
8. Document any allowlist as temporary ADR 0025 compatibility debt.

## Acceptance Criteria

- [ ] `ThemeProfile` exists and compiles.
- [ ] `theme_bridge::install_theme` takes `ThemeProfile`.
- [ ] Both bootstrap theme-install call sites pass `ThemeProfile::Dark`.
- [ ] High-contrast profile contrast tests exist and pass.
- [ ] `theme::glyphs` is deleted.
- [ ] Current default dark behavior is unchanged.
- [ ] Architecture tests still pass.
- [ ] The tests make it possible to ratchet down deprecated helper usage in
      later tasks.
- [ ] No screen behavior or layout changes.

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

- Theme profile wiring requires touching many screen render paths.
- `gpui_component` cannot accept the profile bridge without behavior changes.
- Architecture gates would need to allow broad new deprecated helper usage.
- High-contrast profiles cannot pass the contrast matrix without changing
  visible token values.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0025-theme-icon-style-boundary.md`
- `docs/plans/adr-0025-visual-system-phase-plan.md`
- `src/ui/tokens.rs`
- `src/ui/theme.rs`
- `src/ui/theme_bridge.rs`
- `src/ui/contrast.rs`
- `src/app/bootstrap.rs`
- `tests/architecture_tests.rs`

Goal:
- Add the dormant theme-profile boundary and architecture-test hooks without
  changing current visuals.

Constraints:
- Preserve current dark default behavior.
- Change `install_theme` to accept `ThemeProfile`.
- Add high-contrast contrast tests.
- Delete dead `theme::glyphs`.
- Do not migrate screen call sites.
- Do not add settings UI.
- Do not add arbitrary custom color input.

Do not touch:
- `src/library.rs`
- `src/search.rs`
- `src/application/**`
- database migrations

Acceptance criteria:
- `ThemeProfile` exists and compiles.
- `theme_bridge::install_theme` takes `ThemeProfile`.
- Bootstrap passes `ThemeProfile::Dark`.
- High-contrast profile tests exist and pass.
- `theme::glyphs` is deleted.
- Architecture tests still pass and can be tightened in later ADR 0025 tasks.
- Current UI behavior is unchanged.

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
