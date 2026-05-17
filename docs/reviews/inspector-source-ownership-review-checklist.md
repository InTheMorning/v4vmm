# Inspector Source Ownership Review Checklist

## Reviewed Scope

- ADR 0049 implementation.
- ContentList filter ownership.
- Index-result drill-down.
- Album remove/download same-view refresh.

## Required Checks

- [ ] Library source tree remains local-navigation only.
- [ ] Content filter state belongs to the active inspector/detail VM.
- [ ] `index-*` result IDs are not parsed as SQLite IDs.
- [ ] Remote detail activation uses MusicIndex identity fields.
- [ ] Index artist activation pushes `IndexArtistDetail`, shows Index feeds,
      and provides a breadcrumb back to the search result.
- [ ] Uncached Index feed/track activation pushes `IndexFeedDetail` or
      `IndexTrackDetail` and renders an Index detail surface.
- [ ] Remove/download success refreshes or primes the currently mounted
      inspector.
- [ ] Removed local rows with remote identity remain visible under `All` and
      `Index` with `Download`.
- [ ] Renderers bind VM-owned membership/action state without ad hoc
      inference.
- [ ] Architecture tests guard the ownership rules.

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

## Merge Recommendation

Do not merge if any ownership invariant is enforced only by renderer conditionals
or manual navigation away/back.
