# ADR 0038 Task 007 — Slice D2: Discover Result List

## Goal

Move the Discover result list rendering (`render_result_item`
2393–2461 and the surrounding list container) into
`src/ui/shells/discover/result_list.rs`. Selection mutators
(`select_result` 491–501, `move_up` 336–341, `move_down` 343–348)
stay on `SearchApp`.

## Preconditions

- Slice 0 + D1 landed.

## Files to Create

1. `src/ui/shells/discover/result_list.rs`.

## Files to Modify

1. `src/ui/shells/discover/mod.rs` — add `pub mod result_list;`.
2. `src/search.rs`:
   - Lift `render_result_item` and the list container that wraps it
     into `render_discover_result_list`.
   - Mutators stay on `SearchApp`.

## Surface Module Signature

```rust
#![warn(clippy::pedantic)]
//! Discover result list — vertical list of search results with
//! per-row select/play/add-to-playlist.

use gpui::{prelude::*, AnyElement, Context};
use crate::search::SearchApp;

pub(crate) fn render_discover_result_list(
    cx: &mut Context<SearchApp>,
    // SearchViewModel results projection, resolved row thumbnails,
    // selected result id, etc.
) -> AnyElement {
    // body lifted from result list region in src/search.rs
    todo!()
}
```

## Boundary Identification

```
rg -n "render_result_item|render_results_list|render_search_results|fn render_lazy_results" src/search.rs
```

The result list surface is the list container plus per-row render. If
there's a `render_results_list` wrapper, move it too; if rows are
inlined into a larger `render`, extract a wrapper.

## Verification

```
cargo fmt -- --check
cargo clippy --lib -- -D warnings
cargo test --lib
cargo test --tests
```

`wc -l src/search.rs` decreases by ~70 LOC.

## Commit Message Template

```
Move Discover result list to ui::shells::discover::result_list

Slice D2 of ADR 0038 task 007. Lift the search result list render
(list container + per-row) into
`src/ui/shells/discover/result_list.rs`. Selection mutators stay on
`SearchApp` and dispatch via `cx.listener`. No behavior change.
```

## Constraints

- Thumbnails resolve in the entry-module wrapper (call
  `self.thumbnail_for_url(url, cx)` per row before invoking the shell
  render). The shell receives a pre-resolved
  `Vec<Option<Arc<Image>>>` aligned with the result list ordering.
  This is option 1 from Slice L3's listener wiring notes.
- No mutator moves. No behavior changes.
