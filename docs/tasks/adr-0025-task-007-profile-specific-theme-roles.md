# ADR 0025 Task 007: Profile-Specific Theme Roles

## Status

Implemented - 2026-05-01.

## Task Goal

Make named theme profiles resolve semantic colors through a profile-specific
UI-layer resolver, so high-contrast profiles are visually distinct from base
Dark and Light while keeping `ThemeProfile` GPUI-free.

## Files Changed

- `src/ui/theme_profiles.rs`
- `src/ui/theme_bridge.rs`
- `src/ui/style.rs`
- `src/ui/tokens.rs`
- `src/ui/contrast.rs`
- `src/ui/mod.rs`

## Constraints

- Keep `src/theme_profile.rs` GPUI-free.
- Keep `ThemeProfile::System` hidden from Settings until it follows real
  system appearance.
- Keep high-contrast profiles hidden from Settings until profile-specific
  colors pass contrast tests and visual smoke. They may be exposed after that
  gate is complete.
- Preserve existing Dark and Light behavior.
- Do not add arbitrary custom colors or user theme editing.

## Implementation Summary

- Added `ui::theme_profiles`, the UI-layer resolver for
  `ThemeProfile -> SemanticColor -> Rgba`.
- Changed `theme_bridge` to install colors through profile resolution instead
  of resolving every token directly through `Appearance`.
- Extended `Environment` with the active `ThemeProfile` so
  `tokens::color(cx, ...)` can resolve through the selected profile.
- Changed `ui::style` compatibility colors to track the installed
  `ThemeProfile`, not only light/dark appearance.
- Added real high-contrast dark and high-contrast light palettes.
- Extended contrast tests so high-contrast profiles must pass the WCAG matrix
  and differ from their base Dark/Light profiles.

## Acceptance Criteria

- [x] `ThemeProfile` remains GPUI-free.
- [x] A UI-layer resolver owns profile-specific semantic color resolution.
- [x] `theme_bridge` resolves installed theme colors through the profile
      resolver.
- [x] `tokens::color(cx, ...)` uses the active profile from `Environment`.
- [x] `ui::style` compatibility colors use the active profile.
- [x] Dark and Light remain mapped to their existing palettes.
- [x] High-contrast dark and high-contrast light are visually distinct from
      base Dark/Light.
- [x] High-contrast profiles pass the contrast matrix.
- [x] High-contrast profiles passed manual visual smoke.
- [x] High-contrast profiles are exposed in Settings after the visual-smoke
      gate.
- [x] Design-system primitives and composites resolve default colors through
      the active `ThemeProfile`, not only the light/dark `Appearance`.
- [x] No custom theme editor or arbitrary color input is added.

## Test Commands

- `cargo fmt -- --check`
- `cargo check`
- `cargo test ui::contrast::tests`
- `cargo test --test architecture_tests`
- `cargo test`
- `cargo clippy --lib --tests -- -D warnings`
- `cargo build`
- Manual visual smoke for High Contrast Dark and High Contrast Light across
  Library, Discover, Settings, and playback controls.
- Manual visual smoke for the exposed Settings theme selector.

## Follow-Up Work

- `ThemeProfile::System` remains hidden until OS appearance detection exists.
