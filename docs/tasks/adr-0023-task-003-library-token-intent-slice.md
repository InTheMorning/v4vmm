# ADR 0023 Task 003: Library Token And Intent Slice

## Status

Not started.

## Goal

Continue thinning `library.rs` by moving high-value display/status transitions
into `LibraryViewModel` and replacing screen-level literals with tokens or
existing composites.

## Scope

- Prefer small command-intent/result structs when they remove service setup or
  status formatting from `library.rs`.
- Target album detail, playlist detail, metadata panels, and remaining
  entity/action badges before lower-impact geometry constants.
- Keep pure data in `view_models::library`; no GPUI imports.
- Add unit tests for new `LibraryViewModel` transitions or projections.

## Out Of Scope

- Splitting `library.rs` into multiple modules.
- Introducing a broad CommandBus.
- Changing playlist, metadata, or subscription service semantics.

## Tests

- `cargo test --lib view_models::library`
- `cargo check`
- `cargo clippy --lib --tests -- -D warnings`
- `cargo fmt -- --check`

## Prompt For Lower-Context Coding Model

Work only on the library slice described here. Move one focused set of pure
display/status transitions from `library.rs` into `view_models::library`, add
unit tests, and replace nearby color/layout literals with tokens or existing
composites. Do not import GPUI in view-model code and do not introduce a broad
CommandBus. Run the tests listed above.
