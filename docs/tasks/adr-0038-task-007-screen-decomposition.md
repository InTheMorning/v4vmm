# ADR 0038 Task 007: Screen Decomposition

## Status

Planned (2026-05-04). Per-slice plans live under
`docs/tasks/adr-0038-task-007-slices/`. Starts after Task 006 (PageVm
Generalization) lands — already complete on 2026-05-04.

## Goal

Split `src/library.rs` (3,888 LOC) and `src/search.rs` (6,448 LOC)
along surface lines under `src/ui/shells/library/` and
`src/ui/shells/discover/`. Each per-surface file ≤ 500 LOC. The
top-level entry modules shrink to thin command/state wiring (target
≤ 500 LOC each; ≤ 300 ideal).

## Layer Decision (Resolves Open Question 1 + 2)

- **Selected-entity state** stays in the entry module:
  - `library.rs` keeps `LibraryApp.detail: LibraryDetail`.
  - `search.rs` keeps `SearchApp.inspector_stack: Vec<InspectorFrame>`.
- **Surface modules** under `src/ui/shells/{library,discover}/` are
  *screen-specific* shells. They may import `LibraryApp` /
  `SearchApp` and take `&mut Context<LibraryApp>` /
  `&mut Context<SearchApp>` directly, matching the existing
  pattern in `src/ui/shells/track.rs` and `src/ui/shells/feed.rs`.
- The architecture guard
  `shared_top_level_ui_shells_do_not_import_screen_modules` keeps its
  allowlist of *shared* shells (`artist.rs`, `entity.rs`,
  `playlist.rs`). New screen-specific shells under the new
  subdirectories are explicitly outside that allowlist.
- Render functions accept the selected entity and resolved thumbnails
  as inputs; mutations dispatch via `cx.listener(|this, _, _, cx|
  this.MUTATOR(cx))`.

## Surface Inventory

### Library (`src/library.rs` → `src/ui/shells/library/`)

| Surface | Render fn (line range) | Mutators (line range) | Est. LOC |
|---|---|---|---|
| sidebar | `render_tree` 2138–2316 | `toggle_artist`, `toggle_album`, `cycle_playlist_sort`, `select_playlist`, `create_playlist`, `rename_playlist`, `delete_playlist` (405–487, 907–914) | ~290 |
| feed_list | (within `render_tree` selection branches) | `select_album`, `select_artist`, `hydrate_album_identity_on_view` (686–759) | ~110 |
| feed_detail | (within `render_detail`) 2466–2765 | `check_feed_on_view`, `check_all_feeds`, `apply_all_feed_updates`, `unsubscribe_feed` (760–861, 915–925) | ~450 |
| track_detail (core) | `render_track_detail` 2826–3168 (subset) | `select_track`, `remove_track`, `subscribe_track`, `toggle_local_subscription` (862–906, 926–970, 1109–1157) | ~300 |
| track_detail_metadata | `render_track_detail` metadata portions | id3 + tag-compare + MusicBrainz (971–1518) | ~500 |
| playlist_detail | `render_playlist_detail` 2766–2825 | `remove_playlist_track_at`, `move_playlist_track`, `add_track_to_playlist`, `create_playlist_and_add_track`, `add_album_to_playlist`, `create_playlist_and_add_album` (488–588) | ~177 |

### Discover (`src/search.rs` → `src/ui/shells/discover/`)

| Surface | Render fn (line range) | Mutators (line range) | Est. LOC |
|---|---|---|---|
| search_input | `render_filter_button` 2366–2392 | `on_input_event`, `do_search`, `toggle_fuzzy_search` (355–429) | ~100 |
| result_list | `render_result_item` 2393–2461 | `select_result`, `move_up`, `move_down` (336–348, 491–501) | ~92 |
| recent | `render_recent_feeds_tiles` 4757–4832 | `load_recent_feeds`, `show_recent_feeds`, `open_recent_feed` (287–317, 430–441, 502–516) | ~134 |
| feed_inspector | `render_inspector` 2462–2526; `render_discover_feed_inspector` 2573–2672 | `load_inspector`, `load_podroll`, `inspector_back`, `pop_inspector` (517–633) | ~272 |
| track_inspector (core) | `render_discover_track_inspector` 2673–2708; lazy section helpers 2949–3018 | subscription + nav (897–930) | ~250 |
| track_inspector_metadata | metadata grid render | `toggle_id3_frame_group`, `toggle_metadata_cell`, `stage_id3_drag_copy`, `apply_pending_id3_edits`, `clear_pending_id3_edits`, `toggle_tag_compare`, tag-compare reload, `toggle_musicbrainz_lookup`, `select_musicbrainz_candidate` (734–896, 1516–1688) | ~350 |

## Sub-Split Decision (Resolves Open Question 4)

`track_detail` (~817 LOC) and `track_inspector` (~560 LOC) exceed the
≤500 LOC ceiling. Each splits into two siblings:

- `track_detail.rs` (core) + `track_detail_metadata.rs` (id3 + tag
  compare + MusicBrainz)
- `track_inspector.rs` (core) + `track_inspector_metadata.rs` (id3 +
  tag compare + MusicBrainz)

The metadata file is the natural fault line: id3-cell/MB-lookup
machinery has its own lazy-load lifecycle distinct from track
identity/subscription. This is consistent with the spec rule
"don't fork on internal boundaries" — id3 editing is a distinct user
surface (the inline tag editor) layered on top of the track inspector,
not an arbitrary internal split.

## Migration Order (Smallest First)

Library:
1. **Slice 0** — establish module structure (`mod.rs` stubs).
2. **Slice L1** — `playlist_detail` (~177 LOC).
3. **Slice L2** — `feed_list` (~110 LOC).
4. **Slice L3** — `sidebar` (~290 LOC).
5. **Slice L4** — `feed_detail` (~450 LOC).
6. **Slice L5** — `track_detail` core (~300 LOC).
7. **Slice L6** — `track_detail_metadata` (~500 LOC).

Discover:
8. **Slice D1** — `search_input` (~100 LOC).
9. **Slice D2** — `result_list` (~92 LOC).
10. **Slice D3** — `recent` (~134 LOC).
11. **Slice D4** — `feed_inspector` (~272 LOC).
12. **Slice D5** — `track_inspector` core (~250 LOC).
13. **Slice D6** — `track_inspector_metadata` (~350 LOC).

Final:
14. **Slice F** — architecture guards tightened (LOC ceilings, surface
    enumeration), entry-module size verification, README/CHANGELOG
    updates.

Library and Discover slices are independent of each other after
Slice 0; can be parallelised between subagents if desired but each
chain (L*, D*) must run sequentially.

## Architecture Guards (Added in Slice F)

```rust
#[test]
fn library_screen_modules_are_decomposed_under_src_ui_shells_library() {
    // Enumerate expected surface files. Fail if missing or empty.
}

#[test]
fn discover_screen_modules_are_decomposed_under_src_ui_shells_discover() {
    // Enumerate expected surface files. Fail if missing or empty.
}

#[test]
fn screen_entry_modules_under_500_loc() {
    let ceilings = [("src/library.rs", 500), ("src/search.rs", 500)];
    // Fail if any file exceeds its ceiling (excluding tests/comments
    // — measure same way as existing `code_lines` helper).
}

#[test]
fn surface_modules_under_500_loc() {
    // Same ceiling for every file under src/ui/shells/library/
    // and src/ui/shells/discover/.
}
```

## Constraints (Unchanged from Stub)

- Each split commit moves *one* surface to its own file. Compile and
  test green at every commit.
- No behavior changes during the split. Visual smoke must match
  pre-split for every surface.
- If the split surfaces a structural issue (e.g., a function genuinely
  belongs in two surfaces), pause and resolve via Task 002/003
  patterns rather than duplicating.
- Helper methods like `thumbnail_for_url` and
  `spawn_subscribe_then_append` stay as `&mut self` methods on the
  entry struct. Surfaces receive resolved values via parameters or
  invoke helpers via `cx.listener(...)` callbacks. Do not extract
  shared-helper modules during this task — different signatures
  between Library and Discover would force premature unification.

## Definition of Done

- `src/library.rs` and `src/search.rs` are ≤ 500 LOC and contain only
  command wiring, selected-entity state, and listener boilerplate.
- Every surface lives in its own file under
  `src/ui/shells/{library,discover}/`. All ≤ 500 LOC.
- Four new architecture guards green (file presence, two
  ≤500 LOC ceilings, surface enumeration).
- Visual smoke pairs (light + dark) for every surface — environment
  permitting; deferred items tracked in `docs/reviews/adr-0038-review-checklist.md`.

## Per-Slice Plans

Each slice has a self-contained plan in
`docs/tasks/adr-0038-task-007-slices/`. The plans target sonnet-class
subagents: every slice file lists the exact functions to move (with
line ranges), the surface module signature, the listener wiring, the
verification commands, and the commit message template.

| Slice | Plan |
|---|---|
| 0 | `00-module-structure.md` |
| L1 | `01-library-playlist-detail.md` |
| L2 | `02-library-feed-list.md` |
| L3 | `03-library-sidebar.md` |
| L4 | `04-library-feed-detail.md` |
| L5 | `05-library-track-detail-core.md` |
| L6 | `06-library-track-detail-metadata.md` |
| D1 | `07-discover-search-input.md` |
| D2 | `08-discover-result-list.md` |
| D3 | `09-discover-recent.md` |
| D4 | `10-discover-feed-inspector.md` |
| D5 | `11-discover-track-inspector-core.md` |
| D6 | `12-discover-track-inspector-metadata.md` |
| F | `13-final-guards-and-readiness.md` |

## When To Start

Task 006 landed 2026-05-04. Task 007 unblocked. Suggest running
Slice 0 + L1 + D1 first so both decomposition chains are visibly in
motion before scheduling further subagent work.
