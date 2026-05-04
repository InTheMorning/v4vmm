# ADR 0038 Task 007 — Slice L5: Library Track Detail (Core)

## Goal

Move the *core* (non-metadata) portion of Library track detail into
`src/ui/shells/library/track_detail.rs`. This is the first half of a
two-slice split because the full track detail surface (~817 LOC)
exceeds the 500-LOC ceiling.

**Core scope** (this slice): track header, hero image, primary action
buttons (play / subscribe / remove), basic identity rows. Excludes the
ID3 metadata grid, tag-compare overlay, MusicBrainz lookup panel —
those land in Slice L6.

Mutators in core: `select_track` (862–906), `remove_track` (926–936),
`subscribe_track` (937–970), `toggle_local_subscription` (1109–1157).

## Preconditions

- Slice 0, L1, L2, L3, L4 landed.
- The shared `src/ui/shells/track.rs::build_track_detail_surface`
  helper already accepts `TrackDetailBehaviorSlots` from Task 006.

## Files to Create

1. `src/ui/shells/library/track_detail.rs` — new shell module.

## Files to Modify

1. `src/ui/shells/library/mod.rs` — add `pub mod track_detail;`.
2. `src/library.rs`:
   - In `render_track_detail` (2826–3168), peel off the core sections
     (header / hero / primary actions / identity rows) and lift them
     into `render_library_track_detail_core` in the new module.
   - The metadata sections (id3 grid, tag-compare, MB) **stay in
     `library.rs` for now**; Slice L6 moves them. This means the
     entry-module `render_track_detail` will, after L5, call
     `render_library_track_detail_core` for the upper portion and
     keep its own code for the metadata portion. That's expected —
     intermediate state is not a final shape.
   - Mutators stay on `LibraryApp`.

## Surface Boundary Inside `render_track_detail`

Read the function once. Identify the boundary between:

- Upper: header, hero image, primary action row, basic identity rows
  (artist, album, value-routes summary, contributor lines).
- Lower: ID3 frame editor grid, tag-compare diff display, MusicBrainz
  candidate selector, advanced collapse panels.

The natural boundary is usually a clear container `div().child(...)`
where one section ends and the next begins. If unclear, **stop and
report** rather than guessing — the spec forbids structural ambiguity
during the split.

## Surface Module Signature

```rust
#![warn(clippy::pedantic)]
//! Library track detail core surface — header, hero, actions,
//! identity. Metadata editing lives in `track_detail_metadata`.

use gpui::{prelude::*, AnyElement, Context};
use crate::library::LibraryApp;

pub(crate) fn render_library_track_detail_core(
    cx: &mut Context<LibraryApp>,
    // TrackDetailPageVm projection, hero image, contributor rows,
    // primary action callbacks...
) -> AnyElement {
    // body lifted from the core portion of src/library.rs:2826..=3168
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

`wc -l src/library.rs` decreases by ~300 LOC. The new file should be
≤300 LOC. The remaining metadata portion of `render_track_detail`
in `library.rs` is still ~500 LOC — that's expected; Slice L6
extracts it.

## Commit Message Template

```
Move Library track detail core to ui::shells::library::track_detail

Slice L5 of ADR 0038 task 007. Lift the track detail header/hero/
primary actions/identity sections into
`src/ui/shells/library/track_detail.rs`. Metadata grid and MB
panel remain in `library.rs` pending slice L6. Track core mutators
stay on `LibraryApp`. No behavior change.
```

## Constraints

- Do not move metadata code in this slice. The split is intentionally
  partial.
- If the upper/lower boundary inside `render_track_detail` is not
  obvious, stop and report. Document the obstacle in this slice plan.
- No behavior changes.

## Rollback

If L6's metadata extraction proves harder than expected, the L5
checkpoint is a self-contained intermediate state — the entry module
keeps the metadata render and is still functional.
