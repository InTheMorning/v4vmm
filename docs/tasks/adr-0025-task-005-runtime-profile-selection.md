# ADR 0025 Task 005: Runtime Theme Profile Selection

## Status

Implemented.

## Task Goal

Persist and apply a theme-profile setting after the main render paths consume
semantic visual boundaries.

## Files To Inspect

- `docs/adr/0025-theme-icon-style-boundary.md`
- `src/config.rs`
- `src/app.rs`
- `src/app/bootstrap.rs`
- `src/ui/theme_bridge.rs`
- `src/ui/theme_profile.rs`
- `src/ui/composites/segmented_control.rs`

## Files Likely To Change

- `src/config.rs`
- `src/app.rs`
- `src/app/bootstrap.rs`
- `src/ui/theme_bridge.rs`
- `src/ui/theme_profile.rs`
- tests for config/theme behavior where they exist

## Do Not Touch

- database migrations
- application workflow commands
- service modules
- unrelated Settings fields

## Constraints

- Do not expose high contrast unless high-contrast profile tests exist.
- Do not expose `ThemeProfile::System` while it resolves to a no-op dark
  profile; exposing a no-op control is misleading.
- Persist settings compatibly with existing config behavior.
- Runtime changes must reinstall the theme and refresh windows.
- Avoid screen-specific repaint code.

## Implementation Steps

1. Add config persistence for the selected theme profile.
2. Update bootstrap to install the configured profile.
3. Add Settings UI for the profiles that are fully tested and semantically
   distinct.
4. Ensure changes call the theme bridge and refresh windows.
5. Add or update tests for config parsing/defaults and theme-profile mapping.

## Acceptance Criteria

- [x] Theme profile is persisted.
- [x] Default behavior remains compatible with existing config.
- [x] Runtime profile changes repaint without screen-specific theme code.
- [x] Only tested profiles are exposed.
- [x] `ThemeProfile::System` is not exposed unless it follows real OS/system
      appearance.

## Implementation Notes

- Added `theme_profile` to config with a backward-compatible default of
  `dark`.
- Startup now installs the configured profile after config load.
- Settings exposes only `Dark` and `Light`; `System` remains hidden because it
  still resolves to the dark fallback, and high-contrast remains hidden until
  it has distinct profile values.
- Selecting theme or scale reinstalls through `theme_bridge`, which refreshes
  windows centrally.

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

- Config compatibility requires migration logic.
- Existing render paths still use dark-only helpers widely enough that runtime
  profile switching would expose a broken theme.
- High-contrast profiles do not pass contrast tests.
- `ThemeProfile::System` still resolves to Dark and would be user-visible as a
  no-op setting.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0025-theme-icon-style-boundary.md`
- `src/config.rs`
- `src/app.rs`
- `src/app/bootstrap.rs`
- `src/ui/theme_bridge.rs`
- `src/ui/theme_profile.rs`

Goal:
- Persist and apply a theme-profile setting after semantic visual boundaries
  are in place.

Constraints:
- Expose only tested profiles.
- Do not expose `System` until it follows real OS/system appearance.
- Preserve config compatibility.
- Runtime changes must repaint through the theme bridge.

Do not touch:
- database migrations
- application workflow commands
- service modules
- unrelated Settings fields

Acceptance criteria:
- Theme profile persists and applies at startup.
- Settings expose only tested, semantically distinct profiles.
- Runtime profile changes repaint without screen-specific theme code.
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
