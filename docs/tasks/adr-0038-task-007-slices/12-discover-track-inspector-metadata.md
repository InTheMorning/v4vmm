# ADR 0038 Task 007 — Slice D6: Discover Track Inspector Metadata

## Goal

Move the metadata portion of Discover track inspector into
`src/ui/shells/discover/track_inspector_metadata.rs`. Finishes the
two-slice track inspector split.

**Metadata scope**: ID3 frame editor grid, tag-compare diff overlay,
MusicBrainz lookup candidate panel.

Mutators staying on `SearchApp` (called from this surface):
`toggle_id3_frame_group` (734–743), `toggle_metadata_cell`
(744–753), `stage_id3_drag_copy` (754–783),
`apply_pending_id3_edits` (797–883), `clear_pending_id3_edits`
(884–896), `toggle_tag_compare` (1516–1566),
`redownload_tag_compare` (1567–1570), `reread_tag_compare`
(1571–1574), `reload_tag_compare` (1575–1617),
`toggle_musicbrainz_lookup` (1618–1674),
`select_musicbrainz_candidate` (1675–1688).

## Preconditions

- Slice D5 landed.

## Files to Create

1. `src/ui/shells/discover/track_inspector_metadata.rs`.

## Files to Modify

1. `src/ui/shells/discover/mod.rs` — add
   `pub mod track_inspector_metadata;`.
2. `src/search.rs`:
   - Lift the remaining metadata render code (lower portion of
     `render_discover_track_inspector`) into
     `render_discover_track_inspector_metadata`.
   - Update entry-module render to compose two calls (core + metadata)
     in the same outer container the original used.
   - Track metadata helpers (`track_metadata_rows_for_frame` 3153–3177,
     `track_metadata_action_state` 3178–3186, `metadata_panel_state<T>`
     3187–3199, id3/MB styling helpers) move WITH the metadata render
     if Discover-only. Verify with `rg`.
   - If `render_lazy_sections` still lives in `search.rs` after
     Slice D5 because it dispatches to both contributors/value-routes
     (now-moved) AND metadata (now-moving), restructure: each lazy
     section's dispatch belongs in its respective surface. Move the
     metadata dispatch into the new module; remove the now-empty
     `render_lazy_sections` wrapper if appropriate.

## Verifying Helper Usage Before Moving

```
rg -n "track_metadata_rows_for_frame|track_metadata_action_state|metadata_panel_state|id3_frame_color|id3_cell_status_color|pending_source_color" src/library.rs src/search.rs
```

Helpers used by both Library and Discover track metadata stay in
`src/search.rs` for now (or in `src/library.rs` — wherever they
already live). The new metadata shell module imports from
`crate::search` if needed. Cross-screen helper consolidation is out
of scope for Task 007.

## Surface Module Signature

```rust
#![warn(clippy::pedantic)]
//! Discover track inspector metadata — ID3 frame editor grid,
//! tag-compare diff, MusicBrainz lookup panel.

use gpui::{prelude::*, AnyElement, Context};
use crate::search::SearchApp;

pub(crate) fn render_discover_track_inspector_metadata(
    cx: &mut Context<SearchApp>,
    // TrackDetailPageVm metadata projection, ID3 frame VMs,
    // tag-compare state, MB candidate VMs, callbacks...
) -> AnyElement {
    // body lifted from src/search.rs lower portion of
    // render_discover_track_inspector
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

`wc -l src/search.rs` should drop by ~350 LOC, bringing the entry
module close to ≤500 LOC. The new metadata file should be ≤500 LOC.

## Commit Message Template

```
Move Discover track inspector metadata to ui::shells::discover

Slice D6 of ADR 0038 task 007. Complete the track inspector split
by lifting the ID3 frame editor grid, tag-compare diff, and
MusicBrainz lookup panel into
`src/ui/shells/discover/track_inspector_metadata.rs`. Mutators
stay on `SearchApp` and dispatch via `cx.listener`. No behavior
change.
```

## Constraints

- No shared-helper extraction across Library and Discover.
- If the metadata file ends up >500 LOC, document the additional
  sub-split and pause for review.
- No behavior changes.
