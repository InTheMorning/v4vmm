# ADR 0025 Task 001 Review: Theme Profile Contract And Gates

## Reviewed Scope

- `src/ui/theme_profile.rs`
- `src/ui/theme_bridge.rs`
- `src/ui/contrast.rs`
- `src/app/bootstrap.rs`
- theme-install call updates in `src/app.rs` and `src/search.rs`
- `tests/architecture_tests.rs`
- ADR 0025 docs and phase/task updates

## Verdict

Pass.

Task 001 can be treated as complete. The next ADR 0025 implementation packet is
Task 002, the semantic icon catalog.

## Required Fixes

None.

## Deviations From Task

The original task packet said `theme::glyphs` had no callers and should be
deleted. Current code still uses `theme::glyphs` in Library and Discover
provenance/status rendering, so deleting it would have forced screen migration
outside Task 001. The ADR, phase plan, and task packet now record this as
temporary compatibility debt owned by the icon/badge phases.

## Architectural Review

- `ThemeProfile` is the new profile boundary and resolves to the lower-level
  `Appearance` token layer.
- `theme_bridge::install_theme` now accepts `ThemeProfile`; no appearance-only
  public install entry point remains.
- Current default behavior remains dark through explicit `ThemeProfile::Dark`
  call sites.
- High-contrast profile coverage exists in the contrast matrix before any
  settings exposure.
- Architecture tests now prevent deprecated visual-helper usage from growing
  and forbid it in screen files without an explicit baseline.

## Tests Run

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo test`
- `cargo clippy --lib --tests -- -D warnings`
- `git diff --check`

## Residual Risk

High-contrast profiles currently resolve to the base dark/light palettes. This
is acceptable for Task 001 because the profile contract and test path now
exist, but actual higher-contrast role values remain future ADR 0025 work.
