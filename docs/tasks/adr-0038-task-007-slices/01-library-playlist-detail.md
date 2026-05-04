# ADR 0038 Task 007 — Slice L1: Library Playlist Detail

## Goal

Move Library playlist-detail rendering and supporting playlist mutators
into `src/ui/shells/library/playlist_detail.rs`. The screen entry
module (`src/library.rs`) keeps the playlist mutators as `&mut self`
methods on `LibraryApp` (helpers like `spawn_subscribe_then_append`
they call don't move) but the rendering shifts behind a
`pub(crate) fn render_library_playlist_detail(...)` shell helper.

## Preconditions

- Slice 0 landed: `src/ui/shells/library/mod.rs` exists.
- `cargo test` green at HEAD.

## Files to Create

1. `src/ui/shells/library/playlist_detail.rs` — new shell module.

## Files to Modify

1. `src/ui/shells/library/mod.rs` — add `pub mod playlist_detail;`.
2. `src/library.rs`:
   - Replace the body of `render_playlist_detail` (lines 2766–2825)
     with a single call into the new shell helper, passing the values
     it needs (selected playlist VM projection, resolved thumbnails,
     playlists list, etc.).
   - Keep all `&mut self` mutators in place
     (`remove_playlist_track_at`, `move_playlist_track`,
     `add_track_to_playlist`, `create_playlist_and_add_track`,
     `add_album_to_playlist`, `create_playlist_and_add_album`).
   - Add necessary `use crate::ui::shells::library::playlist_detail::...`.

## What Moves vs. What Stays

| Item | Where to Put It |
|---|---|
| `LibraryApp::render_playlist_detail` (2766–2825) | Move body into shell helper. Method becomes a 2-line forwarder. |
| Local helpers exclusively used by playlist render (e.g. private fn calls inside that range) | Move with the render. |
| `remove_playlist_track_at` (488–508) | **Stays** in `library.rs`. Surface invokes via `cx.listener`. |
| `move_playlist_track` (509–532) | **Stays** in `library.rs`. |
| `add_track_to_playlist` (533–538), `create_playlist_and_add_track` (539–552), `add_album_to_playlist` (553–574), `create_playlist_and_add_album` (575–588) | **Stay** in `library.rs`. |
| `spawn_subscribe_then_append` (589–625) | **Stays** in `library.rs`. Mutators above call it. |
| `thumbnail_for_url` (626–679) | **Stays** in `library.rs`. Entry module resolves before passing into shell render. |

## Surface Module Signature

In `src/ui/shells/library/playlist_detail.rs`:

```rust
#![warn(clippy::pedantic)]
//! Library playlist detail surface.
//!
//! Renders the right-hand pane when a playlist is selected. Consumes
//! [`PlaylistDetailPageVm`] via the shared
//! [`crate::ui::shells::playlist::render_playlist_detail_shell`]
//! helper, then wires Library-specific mutators back to
//! [`LibraryApp`] via `cx.listener` callbacks.

use gpui::{prelude::*, AnyElement, Context};

use crate::library::LibraryApp;

pub(crate) fn render_library_playlist_detail(
    cx: &mut Context<LibraryApp>,
    // pass remaining inputs the original render_playlist_detail
    // captured from `&self` — selected playlist VM, list of all
    // playlists for popover, hero image, etc.
) -> AnyElement {
    // body lifted from src/library.rs:2766..=2825
    todo!()
}
```

The exact parameter list is determined by reading the current
`render_playlist_detail` body and identifying every `self.*`
reference. Each becomes either an explicit parameter or a
`cx.listener(...)` callback.

## Listener Wiring Pattern

The original code uses `cx.listener(|this, _, _, cx| this.MUTATOR(...))`
inside the render body. After the move, the render is no longer a
method, so `cx.listener` still works — `cx` is now `&mut
Context<LibraryApp>`, and `this` is `&mut LibraryApp` inside the
closure exactly as before. No structural rewrite needed; the
listeners just live in a different file.

The thin entry-module wrapper looks like:

```rust
fn render_playlist_detail(&mut self, cx: &mut Context<Self>) -> AnyElement {
    crate::ui::shells::library::playlist_detail::render_library_playlist_detail(
        cx,
        // resolved inputs here
    )
}
```

If the original `render_playlist_detail` captures references to
`self.field` that outlive the render call (e.g., a borrow into
`self.vm`), prefer cloning or passing the VM projection by value to
avoid borrow-checker pain across the module boundary.

## Verification

```
cargo fmt -- --check
cargo clippy --lib -- -D warnings
cargo test --lib
cargo test --tests
```

All green. Spot-check `wc -l src/library.rs` decreases by ~60 LOC.

## Commit Message Template

```
Move Library playlist detail to ui::shells::library::playlist_detail

Slice L1 of ADR 0038 task 007. Lift `render_playlist_detail` body
into `src/ui/shells/library/playlist_detail.rs`. Mutators stay on
`LibraryApp`; the shell helper invokes them through `cx.listener`.
No behavior change.
```

## Constraints

- No behavior changes. Visual smoke must match pre-slice exactly.
- Do not move mutators in this slice. They stay on `LibraryApp` so the
  next slices can keep relying on them.
- Do not extract shared helpers (`thumbnail_for_url`, etc.) — Task 007
  spec forbids it.
- If a borrow-checker problem arises from passing `&self.vm` into the
  shell, clone the small VM projection (it's a value type) rather
  than restructuring the entry module.

## Rollback

If the slice breaks something subtle, revert the single commit. The
entry module's `render_playlist_detail` body is preserved in git
history at the parent commit.
