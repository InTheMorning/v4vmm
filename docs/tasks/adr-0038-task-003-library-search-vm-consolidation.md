# ADR 0038 Task 003: Library/Search VM Consolidation (Stub)

## Status

Stub. Starts after Task 002 (Composite Display-Contract Audit) lands.
May split into Task 003a (Library) and Task 003b (Discover) once an
inventory is in hand.

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

## When To Start

After Task 002 lands. Replace this stub with a fully-specified task
listing the fallback migration order (smallest blast radius first per
the original `one-owner-per-surface-plan`).
