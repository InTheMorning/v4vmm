# ADR 0038 Task 007 — Slice 0: Module Structure

## Goal

Establish empty module containers for `src/ui/shells/library/` and
`src/ui/shells/discover/`. Wire them into `src/ui/shells/mod.rs`. No
code moves yet — this slice only unblocks subsequent slices.

## Preconditions

None. Task 006 already landed.

## Files to Create

1. `src/ui/shells/library/mod.rs` — module documentation +
   `pub mod` declarations (initially empty since no surfaces have moved
   yet).
2. `src/ui/shells/discover/mod.rs` — same shape.

## Files to Modify

1. `src/ui/shells/mod.rs` — add `pub mod library;` and
   `pub mod discover;` after the existing `pub mod track;` line. Keep
   alphabetical-ish order if practical.
2. `src/ui/shells/mod.rs` — extend the module-doc paragraph with one
   line: "Screen-specific shells live under `library/` and
   `discover/`. They are allowed to reference their owning screen
   module (`crate::library::LibraryApp` / `crate::search::SearchApp`)
   because they are owned by that screen."

## File Contents

### `src/ui/shells/library/mod.rs`

```rust
//! Library screen-specific shells.
//!
//! Each child module owns one Library surface (sidebar, feed list,
//! feed detail, track detail, playlist detail). Surfaces accept
//! `&mut Context<LibraryApp>` directly and dispatch mutations via
//! `cx.listener(...)` calls into screen-side mutator methods.
//!
//! Selected-entity state stays in `crate::library::LibraryApp.detail`.
//! Surfaces are render-only after their callbacks return — they do
//! not retain state.
//!
//! See `docs/adr/0038-presentation-contract-enforcement.md` and
//! `docs/tasks/adr-0038-task-007-screen-decomposition.md`.

#![warn(clippy::pedantic)]
```

(No `pub mod` lines yet — slices L1..L6 add them as they land.)

### `src/ui/shells/discover/mod.rs`

```rust
//! Discover screen-specific shells.
//!
//! Each child module owns one Discover surface (search input, result
//! list, recent feeds tiles, feed inspector, track inspector).
//! Surfaces accept `&mut Context<SearchApp>` directly and dispatch
//! mutations via `cx.listener(...)` calls into screen-side mutator
//! methods.
//!
//! Selected-entity state stays in
//! `crate::search::SearchApp.inspector_stack`. Surfaces are
//! render-only after their callbacks return — they do not retain
//! state.
//!
//! See `docs/adr/0038-presentation-contract-enforcement.md` and
//! `docs/tasks/adr-0038-task-007-screen-decomposition.md`.

#![warn(clippy::pedantic)]
```

(No `pub mod` lines yet.)

## Verification

```
cargo fmt -- --check
cargo clippy --lib -- -D warnings
cargo test --lib
cargo test --tests
```

All four must pass. No new architecture guards in this slice.

## Commit Message Template

```
Begin ADR 0038 task 007 module structure

Stage empty `src/ui/shells/library/` and `src/ui/shells/discover/`
shell directories so subsequent slices can move one surface per
commit. No code moves yet.
```

## Constraints

- No code moves in this slice. Only module wiring.
- Do NOT add architecture guards yet — Slice F handles all of those
  in one pass after the moves complete.
- Do NOT touch `src/library.rs` or `src/search.rs` in this slice.
