# ADR 0023 Task 004: Search Inspector Token Slice

## Status

Completed 2026-04-30.

## Goal

Continue thinning Discover/Search inspector code by moving remaining pure
status and panel transitions into `SearchViewModel` and reducing high-count
literal usage in `search.rs`.

## Scope

- Target inspector section headers, empty states, metadata panels, and action
  rows that still build display strings or status colors inline.
- Keep `SearchApp` responsible for GPUI handles, image maps, focus handles,
  inspector frames containing `Arc<Image>`, and service dispatch.
- Keep `LazyPanel<T>` and related transitions GPUI-free.
- Add focused `view_models::search` tests for moved transitions.

## Out Of Scope

- Replacing all split-pane geometry constants.
- Changing result search semantics or network behavior.
- Introducing a broad CommandBus.

## Tests

- `cargo test --lib view_models::search`
- `cargo check`
- `cargo clippy --lib --tests -- -D warnings`
- `cargo fmt -- --check`

## Prompt For Lower-Context Coding Model

Work only on the search inspector slice. Move one focused group of pure
inspector/status transitions from `search.rs` into `view_models::search`, add
unit tests, and replace nearby raw colors/layout literals with semantic tokens
or existing composites. Keep GPUI-bound fields in `SearchApp`. Run the tests
listed above.

## Result

- Added `TrackRowActionVm` for Discover track-row action keys, labels, busy
  tooltips, and remove/download tooltips.
- Removed remaining screen-level `rgb(...)` literals from `search.rs`.
- Replaced remaining numeric `px(...)` literals in `search.rs` with named
  layout/typography constants or existing token values.
- Replaced legacy thumbnail pixel hints with semantic `ThumbnailSize` values.
- Added focused `view_models::search` unit tests.
