# ADR 0038 Task 007 — Slice D1: Discover Search Input

## Goal

Move the Discover search input bar (input field, fuzzy toggle, filter
buttons) into `src/ui/shells/discover/search_input.rs`. Mutators
(`on_input_event` 355–365, `do_search` 366–420, `toggle_fuzzy_search`
421–429) stay on `SearchApp`.

## Preconditions

- Slice 0 landed.
- This slice can run in parallel with Library slices L1+ — Library
  and Discover are independent after Slice 0.

## Files to Create

1. `src/ui/shells/discover/search_input.rs` — new shell module.

## Files to Modify

1. `src/ui/shells/discover/mod.rs` — add `pub mod search_input;`.
2. `src/search.rs`:
   - Lift `render_filter_button` (2366–2392) into the new module.
   - Identify the surrounding render code that places the input field,
     the fuzzy-search toggle, and the filter-button row. That entire
     region moves as `render_discover_search_input`. Read the file
     and confirm the boundary before editing.
   - Mutators stay on `SearchApp`.

## Boundary Identification

```
rg -n "render_filter_button|on_input_event|do_search|toggle_fuzzy_search|self.input" src/search.rs
```

The search input surface is one logical row at the top of the Discover
pane. The render likely lives inside a larger `render_search_pane` or
`render` method. Move only the input row, not the surrounding
result-list container.

## Surface Module Signature

```rust
#![warn(clippy::pedantic)]
//! Discover search input bar — text field, fuzzy toggle, filter
//! buttons.

use gpui::{prelude::*, AnyElement, Context, Entity};
use gpui_component::input::InputState;
use crate::search::SearchApp;

pub(crate) fn render_discover_search_input(
    cx: &mut Context<SearchApp>,
    input: Entity<InputState>,
    // VM projection: type filter state, fuzzy toggle state, etc.
) -> AnyElement {
    // body lifted from src/search.rs filter button + input region
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

`wc -l src/search.rs` decreases by ~30 LOC.

## Commit Message Template

```
Move Discover search input to ui::shells::discover::search_input

Slice D1 of ADR 0038 task 007. Lift the search input bar (text
field, fuzzy toggle, filter buttons) into
`src/ui/shells/discover/search_input.rs`. Search mutators stay on
`SearchApp` and dispatch via `cx.listener`. No behavior change.
```

## Constraints

- Do not move `do_search` or other mutators.
- No behavior changes.
- Keep the input `Entity<InputState>` owned by `SearchApp` —
  surfaces receive it as a parameter (clone the Entity handle), they
  do not own input state.
