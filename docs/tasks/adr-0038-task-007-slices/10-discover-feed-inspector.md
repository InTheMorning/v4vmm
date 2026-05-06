# ADR 0038 Task 007 — Slice D4: Discover Feed Inspector

## Goal

Move the Discover feed inspector pane (`render_inspector` 2462–2526
plus `render_discover_feed_inspector` 2573–2672) into
`src/ui/shells/discover/feed_inspector.rs`. Mutators
(`load_inspector` 517–591, `load_podroll` 592–621, `inspector_back`
622–633, `pop_inspector` 329–334) stay on `SearchApp`.

## Preconditions

- Slice 0 + D1 + D2 + D3 landed.

## Files to Create

1. `src/ui/shells/discover/feed_inspector.rs`.

## Files to Modify

1. `src/ui/shells/discover/mod.rs` — add `pub mod feed_inspector;`.
2. `src/search.rs`:
   - Lift `render_inspector` and `render_discover_feed_inspector`
     bodies into the new module. The two functions form a single
     surface (inspector frame chrome + feed body).
   - Identify what helpers (e.g., `render_inspector_body`,
     `render_lazy_sections`) `render_inspector` calls; move
     Discover-feed-only helpers, leave shared ones in `search.rs`.
   - Mutators stay on `SearchApp`.

## Boundary Identification

```
rg -n "render_inspector|render_discover_feed_inspector|render_inspector_body|render_lazy_sections" src/search.rs
```

`render_lazy_sections` is shared by feed and track inspector
(survey shows `render_lazy_sections` 2949–2956 calls
`render_rss_lazy_sections`). Leave shared lazy-section code in
`search.rs` until Slice D5/D6, then revisit in Slice F if it makes
sense to consolidate.

## Surface Module Signature

```rust
#![warn(clippy::pedantic)]
//! Discover feed inspector — pane shown after selecting a search
//! result that resolves to a feed.

use gpui::{prelude::*, AnyElement, Context};
use crate::search::SearchApp;

pub(crate) fn render_discover_feed_inspector(
    cx: &mut Context<SearchApp>,
    // ReleaseDetailPageVm or feed inspector VM projection, hero
    // image, contributor rows, podroll VM, back-button state...
) -> AnyElement {
    // body lifted from src/search.rs feed inspector region
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

`wc -l src/search.rs` decreases by ~165 LOC.

## Commit Message Template

```
Move Discover feed inspector to ui::shells::discover::feed_inspector

Slice D4 of ADR 0038 task 007. Lift `render_inspector` chrome and
`render_discover_feed_inspector` body into
`src/ui/shells/discover/feed_inspector.rs`. Inspector mutators
(`load_inspector`, `load_podroll`, `inspector_back`,
`pop_inspector`) stay on `SearchApp` and dispatch via
`cx.listener`. No behavior change.
```

## Constraints

- Do NOT move the track inspector portions yet (Slice D5+D6).
- Leave `render_lazy_sections` in `search.rs` for now — it is shared
  with the track inspector.
- No mutator moves. No behavior changes.
