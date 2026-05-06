# ADR 0033 HIG UI Architecture Governance Phase Plan

## Goal

Make Apple HIG-inspired UI discipline enforceable in code by tightening the
shared UI/backend boundary and preventing screen-local floating chrome.

## Non-Goals

- No service or schema changes.
- No complete redesign of Library or Discover.
- No migration of unrelated legacy screen panels in this phase.
- No dependency on Apple platform SDKs.

## Assumptions

- v4vmm remains a Rust/GPUI desktop app with a custom design system.
- Apple HIG is applied as product-design guidance: consistency, clear roles,
  adaptive layout, anchored transient surfaces, and accessible hierarchy.
- Existing ADR 0023, 0025, 0031, and 0032 boundaries are valid and should be
  strengthened, not replaced.

## Affected Modules

- `src/ui/composites/playlist_popover.rs`
- `src/ui/composites/mod.rs`
- `src/library.rs`
- `src/search.rs`
- `src/ui_track.rs`
- `tests/architecture_tests.rs`

## Proposed Sequence

1. Introduce ADR 0033 and task/review artifacts.
2. Replace backend-shaped playlist popover input with a display-ready
   `PlaylistOption`.
3. Add architecture tests that forbid backend/screen imports in shared UI
   primitives/composites.
4. Add architecture tests that forbid screen-local floating chrome.
5. Run formatting, check, focused tests, architecture tests, clippy, and diff
   whitespace checks.

## Schema/API Implications

None.

## Risk Areas

- Playlist popover call sites must preserve existing select/create callbacks.
- Overbroad architecture patterns could block legitimate screen composition.
- Tests should target reusable chrome and backend dependencies, not normal
  screen event wiring.

## Test Strategy

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo test view_models::library`
- `cargo test view_models::search`
- `cargo clippy --lib --tests -- -D warnings`
- `git diff --check`

## Rollback Strategy

Revert the ADR 0033 commit. This restores `AddToPlaylistPopover` accepting
`db::Playlist` directly and removes the new architecture gates without schema
changes.
