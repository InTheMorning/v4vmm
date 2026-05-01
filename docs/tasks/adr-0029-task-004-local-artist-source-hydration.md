# ADR 0029 Task 004: Local Artist Source Hydration

## Status

Implemented - 2026-05-01.

## Goal

Hydrate a local `ArtistView` from stored artist source facts when the caller
already has an explicit MusicIndex artist id.

## Read

- `docs/adr/0029-artist-person-identity-persistence.md`
- `docs/tasks/adr-0029-task-003-musicindex-artist-ingest.md`
- `src/views.rs`
- `src/sources.rs`
- `src/db.rs`
- `src/local_identity.rs`
- `src/view_models/artist.rs`

## Files Likely To Change

- `src/views.rs`
- `src/sources.rs`
- `docs/tasks/adr-0029-task-004-local-artist-source-hydration.md`
- `docs/reviews/adr-0029-task-004-review.md`

## Do Not Touch

- Do not add a `tracks` artist-subject binding.
- Do not hydrate `ArtistRef::LocalArtistName` from source facts.
- Do not merge artists by name.
- Do not change MusicIndex remote fetch semantics.

## Constraints

- `ArtistRef::Musicindex(id)` may be resolved locally from
  `artist_source_facts`.
- `ArtistRef::LocalArtistName(name)` must keep the current local track-row
  behavior.
- Convert `begin_year` and `end_year` from `i64` to `i32` only with checked
  conversion.
- Map source links and source ids through the same GPUI-free view fact types as
  feed and track hydration.

## Acceptance Criteria

- [x] LocalSource can fetch `ArtistRef::Musicindex` from persisted facts.
- [x] LocalSource still fetches `ArtistRef::LocalArtistName` from local tracks.
- [x] Missing explicit artist facts return a clear not-found error.
- [x] No local name-derived artist receives source facts.
- [x] Required verification commands pass.

## Implementation Summary

- Added `ArtistView::from_artist_source_fact` to map
  `db::ArtistSourceFactRow` into the GPUI-free projection.
- Added source-fact row conversions for local identity links and ids.
- Used checked `i64` to `i32` conversion for active years.
- Updated `LocalSource::fetch_artist` so `ArtistRef::Musicindex(id)` resolves
  from `artist_source_facts`.
- Left `ArtistRef::LocalArtistName` on the existing local track-row path.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test sources::tests::local_source_fetch_musicindex_artist
cargo test views::tests::artist_source_fact
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
cargo test
```

Verified 2026-05-01.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0029-artist-person-identity-persistence.md`
- `docs/tasks/adr-0029-task-004-local-artist-source-hydration.md`
- `src/views.rs`
- `src/sources.rs`
- `src/db.rs`
- `src/local_identity.rs`

Goal:
- Add local explicit-source artist hydration for `ArtistRef::Musicindex`.

Constraints:
- No name matching.
- No track binding.
- No UI rendering changes.
- Keep constructors GPUI-free.

Do not touch:
- schema migrations
- `src/search.rs`
- `src/library.rs`
- MusicIndex client fetch code

Acceptance criteria:
- Local explicit MusicIndex artist lookup renders stored scalar facts.
- Name-derived local artists remain unchanged.
- Verification commands pass.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Hydration appears to require matching local tracks by artist name.
- A source row needs to be combined with local track counts in this task.
