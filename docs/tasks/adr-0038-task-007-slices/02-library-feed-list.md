# ADR 0038 Task 007 — Slice L2: Library Feed List

## Goal

Move Library feed-list selection rendering into
`src/ui/shells/library/feed_list.rs`. Selection mutators
(`select_album`, `select_artist`, `hydrate_album_identity_on_view`)
stay on `LibraryApp`.

The feed list is currently embedded inside the sidebar tree's render
branches (within `render_tree`, lines 2138–2316). Slice L2 extracts
*only* the album/track list pane that appears after a feed is selected
— the part of `render_tree` (or a sibling render in `render_detail`)
that shows the feed's contents. Slice L3 extracts the rest of the
sidebar tree.

## Preconditions

- Slice 0 + L1 landed.

## Files to Create

1. `src/ui/shells/library/feed_list.rs` — new shell module.

## Files to Modify

1. `src/ui/shells/library/mod.rs` — add `pub mod feed_list;`.
2. `src/library.rs`:
   - Identify the rendering region that produces the feed contents
     pane (look for the album/track listing branch under feed
     selection; see survey notes — likely around 2466–2765 inside
     `render_album_detail`'s sibling rendering, but verify by reading
     the file before splitting).
   - Lift that region into `render_library_feed_list`.
   - Keep `select_album`, `select_artist`,
     `hydrate_album_identity_on_view` on `LibraryApp`.

## Identifying the Surface (Before Editing)

Run these to confirm the boundaries:

```
rg -n "fn render_album_detail|fn render_tree|fn render_detail" src/library.rs
rg -n "select_album|select_artist|hydrate_album_identity" src/library.rs
```

The feed-list surface is the render region whose mutators are exactly
the three above and nothing else from feed_detail's set
(`check_feed_on_view`, `apply_all_feed_updates`, etc.). If the
boundary is not clean, **stop and report** rather than guessing —
this is a structural issue per Task 007 spec ("If the split surfaces
a structural issue ... pause and resolve").

## Surface Module Signature

```rust
#![warn(clippy::pedantic)]
//! Library feed list surface — album/track listing pane.

use gpui::{prelude::*, AnyElement, Context};
use crate::library::LibraryApp;

pub(crate) fn render_library_feed_list(
    cx: &mut Context<LibraryApp>,
    // selected feed VM projection, list of albums/tracks, resolved
    // thumbnails, etc.
) -> AnyElement {
    // body lifted from the identified region in src/library.rs
    todo!()
}
```

## Listener Wiring

`cx.listener(|this, _, _, cx| this.select_album(...))` continues to
work unchanged — see Slice L1 for the pattern.

## Verification

```
cargo fmt -- --check
cargo clippy --lib -- -D warnings
cargo test --lib
cargo test --tests
```

`wc -l src/library.rs` decreases by ~30 LOC of render plus any helper
references; mutators (~74 LOC) stay put.

## Commit Message Template

```
Move Library feed list to ui::shells::library::feed_list

Slice L2 of ADR 0038 task 007. Lift the album/track listing render
into `src/ui/shells/library/feed_list.rs`. Selection mutators stay
on `LibraryApp` and dispatch via `cx.listener`. No behavior change.
```

## Constraints

- If you cannot find a clean render region for the feed-list surface
  separate from the sidebar tree (Slice L3), stop. Document the
  obstacle in the slice plan and report back rather than splitting
  along an arbitrary line.
- No mutator moves in this slice.
- No behavior changes.
