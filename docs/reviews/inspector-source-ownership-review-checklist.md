# Inspector Source Ownership Review Checklist

## Reviewed Scope

- ADR 0049 implementation.
- ContentList filter ownership.
- Index-result drill-down.
- Album remove/download same-view refresh.

## Required Checks

- [x] Library source tree remains local-navigation only.
- [x] Content filter state belongs to the active inspector/detail VM.
- [x] `index-*` result IDs are not parsed as SQLite IDs.
- [x] Remote detail activation uses MusicIndex identity fields.
- [x] Index artist activation pushes `IndexArtistDetail`, shows Index feeds,
      and provides a breadcrumb back to the search result.
- [x] Uncached Index feed/track activation pushes `IndexFeedDetail` or
      `IndexTrackDetail` and renders an Index detail surface.
- [x] Remove/download success refreshes or primes the currently mounted
      inspector.
- [x] Removed local rows with remote identity remain visible under `All` and
      `Index` with `Download`.
- [x] Renderers bind VM-owned membership/action state without ad hoc
      inference.
- [x] Architecture tests guard the ownership rules.

Note: the implemented navigation variant is named `IndexArtistFeedScope`; it is
the accepted realization of ADR 0049's `IndexArtistDetail` intent because the
surface is a scoped Index feed list.

## Required Commands

```bash
cargo fmt -- --check
cargo check
cargo clippy -- -D warnings
cargo test
```

## Visual Smoke

- Search a query with remote-only Index results.
- Drill into an Index-only artist/feed/track.
- Open a Library album, remove one track, and verify the current inspector row
  changes to `Download` without leaving the album.
- Toggle `All`, `Library`, and `Index`; verify only the inspector/detail row set
  changes, not the left Library tree.

Operator visual smoke passed on 2026-05-18.

## Merge Recommendation

Merge recommendation: complete. Do not reopen unless a new regression breaks an
ADR 0049 invariant.
