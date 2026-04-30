# ADR 0023 Task 005: Release Detail Parity

## Status

Completed 2026-04-30.

## Goal

Make Discover feed detail and Library album detail feel like the same release
surface while preserving mode-specific actions.

## Scope

- Wire Discover feed headers through the shared `DetailHeader` composite.
- Wire Library album track rows through the shared `TrackRow` composite.
- Keep Library-specific actions (`Remove`, `MusicBrainz`, downloaded state,
  add-to-playlist) as trailing controls rather than changing the layout.
- Fix low-contrast default action buttons by using accent text for ghost
  actions.
- Keep service behavior unchanged.

## Out Of Scope

- Broad CommandBus/EventBus work.
- Splitting `library.rs` or `search.rs`.
- Changing subscription, playlist, or MusicBrainz service semantics.

## Tests

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --lib view_models::library`
- `cargo test --lib view_models::search`
- `cargo clippy --lib --tests -- -D warnings`
- `cargo test`
- `cargo build`

## Result

- `render_feed_header` now composes `DetailHeader`, so the `feed` badge is an
  intrinsic badge instead of a full-width strip.
- `render_library_track_row` now composes `TrackRow`, matching Discover's row
  skeleton: number, thumbnail, title, duration, trailing actions.
- `action_button` defaults to accent text for readable ghost/secondary
  controls.
- `TagBadge` is non-flexing so badges retain intrinsic width inside flex
  layouts.

## Prompt For Lower-Context Coding Model

This task is complete. If revisited, preserve service behavior and only adjust
presentation wiring around `DetailHeader`, `TrackRow`, `TagBadge`, and
`action_button`. Do not introduce a broad command bus or split the screen
modules.
