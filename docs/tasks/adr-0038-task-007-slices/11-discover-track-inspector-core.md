# ADR 0038 Task 007 — Slice D5: Discover Track Inspector (Core)

## Goal

Move the *core* (non-metadata) portion of Discover track inspector
into `src/ui/shells/discover/track_inspector.rs`. First half of a
two-slice split because the full track inspector (~560 LOC) exceeds
the 500-LOC ceiling.

**Core scope** (this slice): track header, hero, primary actions,
contributor rows, value-routes panel, lazy-section dispatch.
Excludes id3 metadata grid, tag-compare overlay, MusicBrainz
candidate panel — those land in Slice D6.

Mutators staying on `SearchApp` for core: `toggle_contributors`
(634–686), `toggle_value_routes` (687–733),
`refresh_inspector_subscription_state` (913–930),
`toggle_local_subscription` (897–912),
`render_lazy_sections` / `render_lazy_contributors` /
`render_lazy_value_routes` rendering helpers (2949–3018).

## Preconditions

- Slice 0 + D1 + D2 + D3 + D4 landed.

## Files to Create

1. `src/ui/shells/discover/track_inspector.rs`.

## Files to Modify

1. `src/ui/shells/discover/mod.rs` — add `pub mod track_inspector;`.
2. `src/search.rs`:
   - In `render_discover_track_inspector` (2673–2708), peel off the
     core sections (header, hero, primary actions, contributor rows,
     value routes) and lift them into
     `render_discover_track_inspector_core`.
   - Move `render_lazy_contributors` and `render_lazy_value_routes`
     into the new module (they belong to track-inspector core).
   - Leave `render_lazy_sections` in `search.rs` if it still
     dispatches to metadata sections — Slice D6 finishes that move.
   - Metadata sections (id3 grid, tag-compare, MB) **stay in
     `search.rs` for now**.
   - Mutators stay on `SearchApp`.

## Boundary Inside `render_discover_track_inspector`

Read the function. The boundary mirrors Library track detail:

- Upper: header, hero, primary actions, contributors, value routes.
- Lower: id3 frame editor grid, tag-compare diff, MB candidate
  selector.

If the boundary is not clean, **stop and report**.

## Surface Module Signature

```rust
#![warn(clippy::pedantic)]
//! Discover track inspector core — header, hero, actions,
//! contributors, value routes. Metadata editing lives in
//! `track_inspector_metadata`.

use gpui::{prelude::*, AnyElement, Context};
use crate::search::SearchApp;

pub(crate) fn render_discover_track_inspector_core(
    cx: &mut Context<SearchApp>,
    // TrackDetailPageVm projection, hero image, contributor VMs,
    // value-route VMs, primary action callbacks...
) -> AnyElement {
    // body lifted from upper portion of
    // src/search.rs::render_discover_track_inspector
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

`wc -l src/search.rs` decreases by ~250 LOC. New file ≤ 300 LOC.

## Commit Message Template

```
Move Discover track inspector core to ui::shells::discover

Slice D5 of ADR 0038 task 007. Lift the track inspector header/hero/
actions/contributors/value-routes sections into
`src/ui/shells/discover/track_inspector.rs`. Metadata grid and MB
panel remain in `search.rs` pending slice D6. Core mutators stay on
`SearchApp` and dispatch via `cx.listener`. No behavior change.
```

## Constraints

- Do not move metadata code in this slice.
- If the upper/lower boundary inside the inspector render is not
  obvious, stop and report.
- No behavior changes.
