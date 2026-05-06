# ADR 0038 Task 007 — Slice D3: Discover Recent Feeds

## Goal

Move the recent-feeds tile rendering (`render_recent_feeds_tiles`
4757–4832) into `src/ui/shells/discover/recent.rs`. Mutators
(`load_recent_feeds` 287–317, `show_recent_feeds` 430–441,
`open_recent_feed` 502–516) stay on `SearchApp`.

## Preconditions

- Slice 0 + D1 + D2 landed.

## Files to Create

1. `src/ui/shells/discover/recent.rs`.

## Files to Modify

1. `src/ui/shells/discover/mod.rs` — add `pub mod recent;`.
2. `src/search.rs`:
   - Lift `render_recent_feeds_tiles` body into
     `render_discover_recent`.
   - Mutators stay on `SearchApp`.

## Surface Module Signature

```rust
#![warn(clippy::pedantic)]
//! Discover recent feeds — tile grid shown when search input is
//! empty.

use gpui::{prelude::*, AnyElement, Context};
use crate::search::SearchApp;

pub(crate) fn render_discover_recent(
    cx: &mut Context<SearchApp>,
    // recent feeds VM projection, resolved tile thumbnails, etc.
) -> AnyElement {
    // body lifted from src/search.rs:4757..=4832
    todo!()
}
```

## Verification

```
cargo fmt -- --check
cargo clippy --lib -- -D warnings
cargo test --lib
cargo test --tests
```

`wc -l src/search.rs` decreases by ~75 LOC.

## Commit Message Template

```
Move Discover recent feeds to ui::shells::discover::recent

Slice D3 of ADR 0038 task 007. Lift `render_recent_feeds_tiles`
into `src/ui/shells/discover/recent.rs`. Recent feed mutators
(`load_recent_feeds`, `show_recent_feeds`, `open_recent_feed`)
stay on `SearchApp` and dispatch via `cx.listener`. No behavior
change.
```

## Constraints

- Tile thumbnails pre-resolve in the entry-module wrapper (same
  pattern as Slice D2).
- No mutator moves. No behavior changes.
