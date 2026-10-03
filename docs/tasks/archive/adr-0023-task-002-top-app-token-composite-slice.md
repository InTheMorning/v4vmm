# ADR 0023 Task 002: TopApp Token And Composite Slice

## Status

Completed 2026-04-30.

## Goal

Finish the root app slice of ADR 0023 by routing `app.rs` through semantic
tokens and the `NowPlayingBar` composite instead of hand-rolled playback
controls and legacy theme helpers.

## Scope

- Keep `TopApp` as the screen owner for GPUI handles, subscriptions, settings
  inputs, and playback owner callbacks.
- Use `ui::tokens::{color, SemanticColor, Spacing, Radius, FontSize, Size}`
  for app-level color, spacing, radius, type, and size values.
- Bind the persistent playback strip through `ui::composites::NowPlayingBar`.
- Preserve current keyboard shortcuts, settings persistence, playback
  callbacks, and tab focus behavior.
- Enable `#![warn(clippy::pedantic)]` only for files touched by this slice and
  fix or explicitly justify new warnings.

## Out Of Scope

- Building a full playback view-model.
- Changing playback service behavior.
- Migrating `library.rs` or `search.rs` literals except where needed for
  compilation.

## Tests

- Focused `cargo check`.
- `cargo clippy --lib --tests -- -D warnings`.
- `cargo fmt -- --check`.

## Result

- `TopApp` root chrome and settings render path now use semantic tokens for
  app-level color, spacing, type, and radius values.
- The root playback strip is bound through `NowPlayingBar`; inactive sessions
  keep transport controls disabled, matching the previous inline behavior.
- `clippy::pedantic` is active for the touched Rust modules. The large legacy
  `library.rs` screen keeps explicit migration-scoped expectations for
  existing warnings while this ADR continues.
- Verified with `cargo fmt -- --check`, `cargo check`,
  `cargo clippy --lib --tests -- -D warnings`, focused view-model tests,
  `cargo test`, and `cargo build`.

## Prompt For Lower-Context Coding Model

You are implementing only the root app slice of ADR 0023. In `src/app.rs`, use
semantic tokens for screen-level visuals and bind the top playback strip to
`NowPlayingBar`. Do not redesign settings or playback behavior. Preserve all
existing event handlers. If clippy pedantic warns on your changes, fix the code
or add a narrow `#[expect(..., reason = "...")]`. Run the tests listed above.
