# ADR 0038 Task 007 — Slice L4: Library Feed Detail

## Goal

Move Library feed (album) detail rendering into
`src/ui/shells/library/feed_detail.rs`. The album detail surface
shows the right-hand pane when a feed/album is selected: hero metadata
grid, action row, episode/track table.

Already routed through the shared
`src/ui/shells/feed.rs::render_feed_detail_shell`; this slice adds a
Library-specific wrapper that consumes the page VM and the
Library-only behavior slots (action callbacks, etc.) without
expanding the shared shell.

Mutators that stay on `LibraryApp`: `check_feed_on_view` (760–784),
`check_all_feeds` (785–836), `apply_all_feed_updates` (837–861),
`unsubscribe_feed` (915–925).

## Preconditions

- Slice 0, L1, L2, L3 landed.

## Files to Create

1. `src/ui/shells/library/feed_detail.rs` — new shell module.

## Files to Modify

1. `src/ui/shells/library/mod.rs` — add `pub mod feed_detail;`.
2. `src/library.rs`:
   - Lift the body of `render_album_detail` (or whichever fn renders
     the album detail pane in the current code) into
     `render_library_feed_detail`. Confirm the boundary by reading
     2466–2765 first.
   - Entry-module wrapper becomes a 2-line forwarder.
   - Keep mutators in place.

## Boundary Confirmation (Before Editing)

```
rg -n "fn render_album_detail|fn render_feed_detail|fn render_release" src/library.rs
```

Pin down which function name the current Library uses for album/feed
detail rendering. The survey identified ~300 LOC of render. If the
function is large, factor any obvious internal helpers out as
`fn render_library_feed_detail_action_row(...)`,
`fn render_library_feed_detail_table(...)`, etc., kept as
private fns in the new shell module — only if doing so does not
expand the surface count beyond what's needed.

## Surface Module Signature

```rust
#![warn(clippy::pedantic)]
//! Library feed (album) detail surface.

use gpui::{prelude::*, AnyElement, Context};
use crate::library::LibraryApp;

pub(crate) fn render_library_feed_detail(
    cx: &mut Context<LibraryApp>,
    // ReleaseDetailPageVm or equivalent projection, hero image,
    // metadata grid VM, action callbacks...
) -> AnyElement {
    // body lifted from src/library.rs feed-detail render region
    todo!()
}
```

If feed detail already routes through
`src/ui/shells/feed.rs::render_feed_detail_shell`, the new
Library-specific wrapper just builds the `Library`-flavored
`FeedDetailBehaviorSlots` (action callbacks, MusicBrainz pull, etc.)
and forwards.

## Verification

```
cargo fmt -- --check
cargo clippy --lib -- -D warnings
cargo test --lib
cargo test --tests
```

`wc -l src/library.rs` decreases by ~300 LOC.

## Commit Message Template

```
Move Library feed detail to ui::shells::library::feed_detail

Slice L4 of ADR 0038 task 007. Lift the album/feed detail render
region into `src/ui/shells/library/feed_detail.rs`, building
`Library`-flavored behavior slots and forwarding to the shared
feed shell. Feed mutators stay on `LibraryApp`. No behavior change.
```

## Constraints

- Do NOT change `src/ui/shells/feed.rs`. Library-specific behavior
  stays in the new Library shell wrapper.
- If the feed detail render touches track listing within the album,
  that listing belongs to feed_detail (it's the same surface — track
  list inside an album view). Per Task 007 spec rule 4 ("don't fork
  on internal boundaries"), do not split the album track table into
  its own surface.
- The new file may exceed 400 LOC. If it crosses 500, stop and
  report — sub-splitting feed_detail was not part of the plan and
  needs human input.
