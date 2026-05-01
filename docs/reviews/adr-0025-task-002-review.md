# ADR 0025 Task 002 Review: Semantic Icon Catalog

## Reviewed Scope

- `src/ui/icons.rs`
- `src/ui/mod.rs`
- `src/search.rs`
- `src/library.rs`
- `src/ui/composites/now_playing_bar.rs`
- `src/ui/contrast.rs`
- `tests/architecture_tests.rs`

## Verdict

Pass.

Task 002 can be treated as complete. The next ADR 0025 implementation packet is
Task 003, control style roles.

## Required Fixes

None.

## Architectural Review

- RSS and Nostr SVG assets now live behind `ui::icons`.
- Screens still own click behavior and identifiers, but request semantic icons
  through `IconName` and `IconSize`.
- The Library RSS link helper moved out of `ui/mod.rs` and into
  `ui::icons`.
- Now-playing transport controls use semantic playback icons instead of local
  glyph literals.
- Brand/protocol icon fills have contrast coverage against the current dark
  canvas.
- Architecture tests reject new screen-level inline SVG icon helpers.

## Tests Run

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo test`
- `cargo clippy --lib --tests -- -D warnings`
- `git diff --check`

## Residual Risk

The catalog still renders RSS and Nostr as inline SVG strings internally. That
is intentional for this task: the boundary moved out of screens without adding
an asset pipeline. A later task can move catalog internals to asset files if
the icon set grows.
