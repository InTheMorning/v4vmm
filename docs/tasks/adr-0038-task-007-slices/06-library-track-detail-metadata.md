# ADR 0038 Task 007 — Slice L6: Library Track Detail Metadata

## Goal

Move the metadata portion of Library track detail into
`src/ui/shells/library/track_detail_metadata.rs`. This finishes the
two-slice track detail split.

**Metadata scope**: ID3 frame editor grid, tag-compare diff overlay,
MusicBrainz lookup candidate panel, advanced collapse sections.

Mutators staying on `LibraryApp` (called from this surface):
`toggle_id3_frame_group`, `toggle_metadata_cell`,
`apply_pending_id3_edits`, `clear_pending_id3_edits`,
`toggle_tag_compare`, `redownload_tag_compare`, `reread_tag_compare`,
`reload_tag_compare`, `toggle_musicbrainz_lookup`,
`select_musicbrainz_candidate`, `musicbrainz_track`, `musicbrainz_feed`,
plus helpers `selected_track_frame_mut`,
`stage_musicbrainz_lookup_for_track`.

## Preconditions

- Slice L5 landed (track detail core already moved).

## Files to Create

1. `src/ui/shells/library/track_detail_metadata.rs` — new shell
   module.

## Files to Modify

1. `src/ui/shells/library/mod.rs` — add
   `pub mod track_detail_metadata;`.
2. `src/library.rs`:
   - Lift the remaining metadata render code (the lower portion of
     the old `render_track_detail`) into
     `render_library_track_detail_metadata` in the new module.
   - Update the entry-module `render_track_detail` to compose two
     calls:
     ```rust
     let core = render_library_track_detail_core(cx, /* ... */);
     let metadata = render_library_track_detail_metadata(cx, /* ... */);
     // combine into the same outer container the original used
     ```
     If the original render wraps both halves in a single scrolling
     container or split layout, preserve that wrapper in the entry
     module.
   - Track metadata helpers (`track_metadata_rows_for_frame`,
     `track_metadata_action_state`, `metadata_panel_state<T>`,
     `id3_frame_color`, `id3_cell_status_color`,
     `pending_source_color`, etc.) move WITH the metadata render to
     the new module if they're only used by that surface. Verify with
     `rg` before moving.

## Verifying Helper Usage Before Moving

```
rg -n "track_metadata_rows_for_frame|track_metadata_action_state|metadata_panel_state|id3_frame_color|id3_cell_status_color|pending_source_color" src/library.rs src/search.rs
```

Helpers used ONLY by Library track detail metadata can move. Helpers
used by Library and Discover (e.g., shared id3 styling) must NOT
move yet — they'd need a shared module, which Task 007 spec
explicitly forbids during this task. In that case, leave them in
`library.rs` and import from the new metadata module via
`use crate::library::{id3_frame_color, ...};`.

This will look slightly awkward (a shell module importing from the
entry module) but is consistent with how `src/ui/shells/track.rs`
already imports from `crate::search`. The Final slice (F) cleans up
guard expectations to allow this.

## Surface Module Signature

```rust
#![warn(clippy::pedantic)]
//! Library track detail metadata surface — ID3 frame editor grid,
//! tag-compare diff, MusicBrainz lookup panel.

use gpui::{prelude::*, AnyElement, Context};
use crate::library::LibraryApp;

pub(crate) fn render_library_track_detail_metadata(
    cx: &mut Context<LibraryApp>,
    // TrackDetailPageVm metadata projection, ID3 frame VMs,
    // tag-compare state, MB candidate VMs, callbacks...
) -> AnyElement {
    // body lifted from src/library.rs lower portion of
    // render_track_detail
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

`wc -l src/library.rs` decreases by ~500 LOC. After L5+L6, total
Library decrease across both slices is ~800 LOC. `library.rs`
should be approaching ≤500 LOC.

`wc -l src/ui/shells/library/track_detail_metadata.rs` should be
≤500 LOC. If it exceeds, sub-split further into one file per
metadata sub-surface (id3 vs. tag-compare vs. MB) — but only if
necessary, and report the decision in the commit message.

## Commit Message Template

```
Move Library track detail metadata to ui::shells::library

Slice L6 of ADR 0038 task 007. Complete the track detail split by
lifting the ID3 frame editor grid, tag-compare diff, and
MusicBrainz lookup panel into
`src/ui/shells/library/track_detail_metadata.rs`. Track metadata
mutators stay on `LibraryApp` and dispatch via `cx.listener`. No
behavior change.
```

## Constraints

- Do not extract shared id3 styling helpers into a shared module.
  Keep them in `library.rs` for now (or in
  `track_detail_metadata.rs` if Library-only). The Final slice
  decides their final home.
- No behavior changes.
- If the metadata file ends up >500 LOC, document the additional
  sub-split in the slice plan and pause for review.
