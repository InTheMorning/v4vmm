# ADR 0038 Task 003: Library/Search VM Consolidation

## Status

In progress - first Discover feed-link slice implemented on 2026-05-03.
May split into Task 003a (Library) and Task 003b (Discover) once the
full inventory is in hand.

## Goal

Hoist all fallback policy from `src/library.rs` and `src/search.rs` into
view-models. Screens read `display_*` accessors; screens never decide
what an empty value means.

## Inventory To Verify Before Starting

Grep evidence (verified 2026-05-02):

- `library.rs:164-165` — track display title with feed-title fallback.
- `library.rs:1604-1605` — `Unknown Artist` fallback.
- `library.rs:1609-1610` — `Unknown Album` fallback.
- `library.rs:2171` — `[untitled]` playlist row title.
- `library.rs:2384` — `feed_url.unwrap_or_default()`.
- `library.rs:2980` — `Tags` section title.
- `ui/shells/track.rs:103` (post-relocation) — track identity branch.
- Multiple `unwrap_or_default()` and `unwrap_or("")` sites in both
  `library.rs` and `search.rs` (~15 from the 2026-05-02 audit).

Re-grep when starting; the 2026-05-02 inventory is a starting list, not
authoritative.

Verified starting notes, 2026-05-03:

- The `Tags` section-title fallback is already owned by
  `TrackMetadataGridVm::tag_column_label`.
- Discover track-inspector feed-link fallback was still split between
  `TrackInspectorHeaderVm` and `src/search.rs`; this task's first slice
  moves the complete feed-link display contract into the VM.

## Files Likely To Change

- `src/view_models/track.rs` — `display_title`, `display_artist`,
  `display_album` accessors.
- `src/view_models/feed.rs` — `display_url -> Option<String>`.
- `src/view_models/library.rs`, `src/view_models/search.rs` — possible
  new accessors; consider splitting under
  `src/view_models/library/` and `src/view_models/discover/` once they
  approach 3,000 LOC.
- `src/view_models/track_metadata_grid.rs` — own the `Tags` fallback.
- (possibly new) `src/view_models/playlist.rs` for `display_name`.
- `src/library.rs`, `src/search.rs` — call-site sweep.
- `src/ui/shells/*.rs` — call-site sweep.
- `tests/architecture_tests.rs` — tighten existing fallback guards;
  add `view_models_own_display_fallbacks_for_library_and_search`.

## Migration Order

1. Discover track-inspector feed link
   - Introduce `TrackFeedLinkDisplay` in `view_models::search`.
   - Make `TrackInspectorHeaderVm::feed_link_display()` own guid
     presence, feed-title label fallback, and URL fallback.
   - Remove `src/search.rs` render-glue fallback from
     `feed_link_label.unwrap_or_else`.
   - Add `view_models_own_display_fallbacks_for_library_and_search`.
2. Re-grep the remaining `library.rs` / `search.rs` fallback inventory.
3. Payment-route address display
   - Add `PaymentRouteVm::address()` so value-route address presence
     and empty-string preservation live in the VM.
   - Remove `src/search.rs` render-glue coercion from
     `route.address.clone().unwrap_or_default()`.
   - Extend `view_models_own_display_fallbacks_for_library_and_search`.
4. Payment-route custom field display
   - Add `PaymentRouteVm::custom_fields()` so `key ...` / `value ...`
     formatting and empty-string preservation live in the VM.
   - Remove `src/search.rs` render-glue checks of `custom_key` and
     `custom_value`.
   - Extend `view_models_own_display_fallbacks_for_library_and_search`.
5. Payment-route summary display
   - Add `PaymentRouteVm::summary()` so recipient, route type, split,
     and fee/split label fallbacks enter the renderer as one display
     string.
   - Remove `src/search.rs` render-glue formatting of the primary
     payment-route line.
   - Extend `view_models_own_display_fallbacks_for_library_and_search`.
6. Discover feed-list tile display
   - Extend `RecentFeedTileDisplay` with the tile id and episode-count
     note.
   - Make `RecentFeedTileVm::display()` own missing-guid and
     missing-episode-note handling for feed-list tiles.
   - Remove `src/search.rs` render-glue formatting of the feed tile id,
     title fallback, and episode note.
   - Extend `view_models_own_display_fallbacks_for_library_and_search`.
7. Library tree track-number prefix
   - Add `LibraryTrackRowVm::tree_number_prefix()` so the Library
     tree-row zero-padded track-number prefix is VM-owned.
   - Remove `src/library.rs` render-glue formatting of
     `"{n:02} - "`.
   - Extend `view_models_own_display_fallbacks_for_library_and_search`.
8. Migrate one remaining fallback at a time, smallest blast radius first.

## Constraints

- Each VM accessor lands with three-case unit tests: present, empty
  string, `None`.
- Preserve the empty-vs-unknown distinction. `Option<String>` is
  preferred for fields where an empty state has a different visual
  treatment from a labeled fallback.
- One fallback at a time per commit.
- Existing `screens_do_not_inline_unknown_artist_or_album_fallbacks` and
  related guards must stay green throughout.

## Open Questions

1. **`view_models/library.rs` and `view_models/search.rs` size.** Both
   are ~2,800 LOC. Split now, or after this task? Recommendation:
   split as the consolidation lands, so new accessors land in the new
   submodule rather than enlarging the monolith.
2. **Feed-title fallback chain.** `track.title or track.feed_title or
   "Untitled"` is multi-source. The VM accessor must take both.
   Confirm the precedence rule (title-first is current behavior).
3. **Playlist row fallback.** `[untitled]` versus `Untitled` — pick one
   and document it. Current code has both.

## Definition of Done

- Every fallback string in the grep inventory has an owning VM
  accessor.
- Screens contain zero string-literal fallbacks for these concepts.
- New guard
  `view_models_own_display_fallbacks_for_library_and_search` is green.
- VM unit tests cover present / empty / `None` per accessor.

## First-Slice Implementation Notes

- `TrackFeedLinkDisplay` now carries the Discover track-inspector feed
  guid, visible label, and target URL as one VM-projected display
  contract.
- `TrackInspectorHeaderVm::feed_link_display()` returns `None` when
  there is no usable feed guid; otherwise it uses the existing
  `feed_title -> guid` label fallback and `feed_url -> feed_guid` URL
  fallback.
- `src/search.rs` no longer accepts an optional feed-link label and no
  longer reconstructs the guid fallback in render glue.
- The new architecture guard
  `view_models_own_display_fallbacks_for_library_and_search` blocks the
  separated feed-link fallback calls from returning to `src/search.rs`.

## Second-Slice Implementation Notes

- `PaymentRouteVm::address()` now carries optional payment-route address
  display while preserving the old distinction between `Some("")` and
  `None`.
- `src/search.rs` no longer checks `route.address.is_some()` or coerces
  `route.address.clone().unwrap_or_default()` inside the value-routes
  renderer.
- The architecture guard now also blocks screen-local payment-route
  address presence/coercion from returning to `src/search.rs`.

## Third-Slice Implementation Notes

- `PaymentRouteVm::custom_fields()` now carries optional `key ...` /
  `value ...` display while preserving the old distinction between
  present-empty values and absent values.
- `src/search.rs` no longer checks `route.custom_key` or
  `route.custom_value` directly inside the value-routes renderer.
- The architecture guard now also blocks screen-local payment-route
  custom-field presence/formatting from returning to `src/search.rs`.

## Fourth-Slice Implementation Notes

- `PaymentRouteVm::summary()` now carries the primary payment-route
  display line, including recipient fallback, route-type fallback, split
  fallback, and fee/split label.
- `src/search.rs` no longer assembles the primary value-route label from
  `recipient_name`, `route_type`, `split`, and `kind_label`.
- The architecture guard now also blocks screen-local payment-route
  summary formatting from returning to `src/search.rs`.

## Fifth-Slice Implementation Notes

- `RecentFeedTileDisplay` now carries the Discover feed-list tile id and
  optional episode note in addition to title, subtitle, and image URL.
- `RecentFeedTileVm::display()` owns the legacy
  `feed_guid.unwrap_or_default()` id fallback and the optional
  `"{count} tracks"` episode note.
- `src/search.rs` no longer derives feed-list tile ids, title fallback,
  or episode notes locally in `render_feed_list_section`.
- The architecture guard now also blocks screen-local feed-list tile id,
  title, and episode-note fallback from returning to `src/search.rs`.

## Sixth-Slice Implementation Notes

- `LibraryTrackRowVm::tree_number_prefix()` now carries the Library
  tree-row zero-padded track-number prefix.
- `src/library.rs` no longer formats `"{n:02} - "` while rendering the
  album-expanded track rows in the Library left tree.
- The architecture guard now also blocks the screen-local tree-row
  track-number prefix from returning to `src/library.rs`.

## Test Commands

```sh
cargo fmt -- --check
cargo check
cargo test track_inspector_header_vm_projects_feed_link_display_contract
cargo test payment_route_vm_projects_address_without_coercing_presence
cargo test payment_route_vm_projects_custom_fields_without_coercing_presence
cargo test payment_route_vm_projects_primary_summary
cargo test recent_feed_tile_vm_projects_id_and_episode_note
cargo test tree_number_prefix_preserves_legacy_zero_padded_display
cargo test view_models_own_display_fallbacks_for_library_and_search
cargo test
cargo clippy -- -D warnings
git diff --check
```

## Expected Final Report

- Name the fallback migrated.
- Name the VM/display contract that owns it.
- Name the guard added or tightened.
- Report automated gate status.
- Explicitly say whether visual evidence was needed and, if not, why.
