# ADR 0038 Task 007 — Slice L3: Library Sidebar

## Goal

Move Library sidebar rendering (`render_tree`, ~lines 2138–2316) into
`src/ui/shells/library/sidebar.rs`. The sidebar tree owns:

- Artist/album expand/collapse tree
- Playlist sidebar header + list
- Search input within the sidebar
- Split-pane resize handle

Sidebar mutators (`toggle_artist`, `toggle_album`,
`cycle_playlist_sort`, `select_playlist`, `create_playlist`,
`rename_playlist`, `delete_playlist`) stay on `LibraryApp`.

## Preconditions

- Slice 0, L1, L2 landed.

## Files to Create

1. `src/ui/shells/library/sidebar.rs` — new shell module.

## Files to Modify

1. `src/ui/shells/library/mod.rs` — add `pub mod sidebar;`.
2. `src/library.rs`:
   - Lift `render_tree` body (2138–2316) into
     `render_library_sidebar`.
   - Keep the entry-module wrapper as a 2-line forwarder.
   - Keep all sidebar mutators on `LibraryApp`.
   - Keep `set_hovered_thumb`, `thumbnail_for_url` as-is (entry
     module). Pass `Option<Arc<Image>>` resolved values into the
     shell render where needed; for hover-driven thumbnail loading,
     dispatch through `cx.listener` calling `set_hovered_thumb` and
     `thumbnail_for_url`.

## Surface Module Signature

```rust
#![warn(clippy::pedantic)]
//! Library sidebar — artist/album/playlist tree + search input
//! + split-pane handle.

use gpui::{prelude::*, AnyElement, Context};
use crate::library::LibraryApp;

pub(crate) fn render_library_sidebar(
    cx: &mut Context<LibraryApp>,
    // tree VM projection, playlist list VM projection, search input
    // entity, hovered-thumb URL state, etc.
) -> AnyElement {
    // body lifted from src/library.rs:2138..=2316
    todo!()
}
```

## Listener Wiring Notes

The original render uses listeners like:

```rust
.on_resize_start(cx.listener(|this, _: &MouseDownEvent, _window, cx| {
    this.vm.begin_resize();
    cx.notify();
}))
```

These keep working unchanged in the new module — `cx` and `this` types
are identical.

For thumbnail resolution, the original pattern is `let img =
self.thumbnail_for_url(...)`. After the move, the *render function* is
no longer a method, so it cannot call `self.thumbnail_for_url`
directly. Two options, pick the one that fits cleanest:

1. **Resolve before the call** — the entry-module wrapper resolves
   thumbnails up-front and passes them in as parameters. Cleanest if
   the set of thumbnails is bounded.
2. **Pass through `cx`** — inside the render, do
   `cx.listener(|this, _, _, _cx| this.thumbnail_for_url(...))` for
   hover events; for synchronous resolution during render, prefer
   option 1.

Tree rendering currently resolves thumbnails per row during render.
That's option 1 — resolve in the entry-module wrapper, pass in a map
or pre-resolved list.

## Verification

```
cargo fmt -- --check
cargo clippy --lib -- -D warnings
cargo test --lib
cargo test --tests
```

`wc -l src/library.rs` decreases by ~180 LOC.

## Commit Message Template

```
Move Library sidebar to ui::shells::library::sidebar

Slice L3 of ADR 0038 task 007. Lift `render_tree` (artist/album/
playlist tree + search input + split-pane handle) into
`src/ui/shells/library/sidebar.rs`. Sidebar mutators stay on
`LibraryApp` and dispatch via `cx.listener`. No behavior change.
```

## Constraints

- The split-pane resize handle's listener uses
  `this.vm.begin_resize()` directly. It must continue to call into
  `LibraryApp.vm`; do not move resize state into the shell module.
- If thumbnail pre-resolution requires walking a large tree, prefer
  the existing per-row pattern but funnel through `cx.listener` —
  do not introduce a new caching layer.
- No mutator moves. No behavior changes.
