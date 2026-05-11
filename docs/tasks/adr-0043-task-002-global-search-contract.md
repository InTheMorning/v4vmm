# ADR 0043 Task 002: Global Search Contract and Local Query

Status: Implemented - 2026-05-11

## Goal

Add the non-visual global search contract and local in-library search
query. Do not remove existing Library or Discover search fields yet.

## Files to Inspect

- `docs/adr/0043-top-toolbar-global-search.md`
- `src/app.rs`
- `src/app/keyboard.rs`
- `src/application/queries/search.rs`
- `src/application/queries/library.rs`
- `src/db.rs`
- `src/library_service.rs`
- `src/view_models/search.rs`
- `src/view_models/library.rs`

## Files Likely to Change

- `src/app.rs`
- `src/app/keyboard.rs`
- `src/application/queries/search.rs`
- `src/db.rs`
- `src/library_service.rs`
- `src/view_models/app_toolbar.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- Search result rendering
- Discover result pagination rendering
- Library sidebar rendering
- MusicIndex API client behavior

## Constraints

- Global search execution is Enter/Search-button driven.
- Local search must return only `is_in_library = 1` tracks.
- Local search must be case-insensitive and search track title, artist,
  album, album artist, and feed title.
- Local search limit defaults to 50 for v1.
- No schema migration.

## Implementation Steps

1. [x] Add `GlobalSearchScope::{All, Library, Index}` to the toolbar view
   model contract.
2. [x] Add top-level search input ownership to `TopApp` without changing
   the visible screen-local search fields yet.
3. [x] Add a local library search query under `ApplicationQueryService` in
   the search query family.
4. [x] Add DB/library-service helpers for local in-library search with a
   limit parameter.
5. [x] Add unit tests for scope labels, placeholder, normalization
   handoff, and local query behavior.

Note: `cmd-f` remains routed to the active visible screen search until Task
003 renders the global toolbar search. This preserves current user-visible
behavior while Task 002 establishes the ownership and query contracts.

## Acceptance Criteria

- [x] `TopApp` can own a global search input without changing visible search
  behavior yet.
- [x] Local query returns expected in-library tracks and excludes tracks not
  in the library.
- [x] Query tests cover title, artist, album, album artist, and feed title
  matches.
- [x] No network behavior changes.

## Implementation Notes

- Toolbar search labels, ids, placeholder, and accessibility copy live in
  `src/view_models/app_toolbar.rs`.
- `TopApp` owns the global search input entity so Task 003 can render it
  through the app toolbar instead of screen-local chrome.
- Local search is exposed through
  `ApplicationQueryService::search_local_library_tracks`, routes through
  `library_service`, and uses the shared `db::TrackRow` projection.
- The DB query searches only `is_in_library = 1` tracks, escapes SQL LIKE
  wildcards, searches supported title/artist/album/feed fields, and defaults
  to 50 results at the application-query boundary.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test application::queries::search
cargo test db
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0043-top-toolbar-global-search.md`
- `src/app.rs`
- `src/application/queries/search.rs`
- `src/db.rs`
- `src/library_service.rs`
- `src/view_models/app_toolbar.rs`

Goal:
- Add the global search contract and local in-library search query,
  without changing rendered Search or Library behavior yet.

Constraints:
- Enter/Search-button execution only.
- In-library tracks only.
- No schema migration.
- Limit local results to 50 by default.

Do not touch:
- Search result rendering
- Library sidebar rendering
- MusicIndex API client behavior

Acceptance criteria:
- Scope display contracts exist.
- Local query is tested and excludes non-library tracks.
- Existing UI still behaves as before.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test application::queries::search`
- `cargo test db`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Existing DB helpers cannot return enough display data for Search
  workspace rows without duplicating row-shaping logic.
- `TopApp` input ownership conflicts with GPUI focus lifecycle.
