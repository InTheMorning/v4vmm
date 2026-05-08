# ADR 0043 Top Toolbar Global Search Phase Plan

## Goal

Create a top app toolbar with a distinct Now Playing frame and one
global search field. Global search routes to the Search workspace and
can query Library, MusicIndex, or both.

## Non-Goals

- No playback behavior changes beyond existing Previous, Play/Pause,
  Next, and Stop controls.
- No live remote search while typing.
- No new database tables or migrations.
- No screen-local duplicate search chrome.

## Assumptions

- The current Discover tab becomes `Search` in user-facing text.
- Enter/Search-button execution remains the v1 behavior.
- `All` scope groups Library results first and MusicIndex results
  second.
- Local Library search is capped at 50 rows and searches only
  in-library tracks.

## Affected Modules

- `src/app.rs`, `src/app/tab_bar.rs`, `src/app/playback_bar.rs`, and
  `src/app/keyboard.rs` for toolbar composition, focus, and routing.
- `src/view_models/` for toolbar and grouped search display contracts.
- `src/application/queries/search.rs`, `src/db.rs`, and
  `src/library_service.rs` for local library search.
- `src/search/app_impl.rs` and `src/ui/shells/discover/*` for Search
  workspace rendering and retirement of the local Discover search field.
- `tests/architecture_tests.rs` and focused unit tests for guards.

## Proposed Sequence

1. Toolbar frame and Now Playing visual ownership.
   - Introduce `view_models::app_toolbar`.
   - Rename/reframe `render_tab_bar` as an app toolbar.
   - Add the framed Now Playing region without changing playback
     command behavior.
   - Keep current local search fields temporarily.

2. Global search contract and local Library query.
   - Add `GlobalSearchScope` and top-level search input ownership on
     `TopApp`.
   - Add a local in-library search query under the application query
     service.
   - Add tests for scope display and local query behavior.

3. Search workspace routing and grouped results.
   - Route global search to the Search workspace.
   - Rename Discover user-facing chrome to Search.
   - Render grouped Library and MusicIndex results from one snapshot.
   - Remove visible Library/Discover search fields.

4. Guards, visual proof, and readiness review.
   - Add architecture guards for toolbar ownership, duplicate search
     field retirement, Now Playing layer ownership, and display-contract
     labels.
   - Capture light and dark visual evidence at normal and narrow widths.
   - Update the ADR 0043 review checklist with the readiness decision.

## Schema and API Implications

- No schema changes.
- No external API changes.
- Internal query API adds local library search by normalized query and
  result limit.
- Internal Search workspace API adds a command-style entry point for
  global search requests.

## Risk Areas

- Toolbar overcrowding at narrow widths.
- Search focus regressions for `cmd-f`.
- Mixing Library and MusicIndex result types without clear grouping.
- Accidentally preserving duplicate screen-local search fields.
- Making Now Playing a generic composite before it has a second call
  site.

## Test Strategy

- Run `cargo fmt -- --check`.
- Run `cargo check`.
- Run `cargo test`.
- Run `cargo clippy -- -D warnings`.
- Add unit tests for toolbar display contracts, local search query
  behavior, and Search VM grouped snapshots.
- Add architecture tests for ownership boundaries.
- Perform light/dark visual smoke for toolbar, Now Playing, and grouped
  Search results.

## Rollback Strategy

Each task is independently revertible:

- Reverting Task 001 restores the old top strip without touching search
  behavior.
- Reverting Task 002 removes the unused global-search contract and local
  query before UI routing depends on it.
- Reverting Task 003 restores local screen search fields and Discover
  search behavior.
- Reverting Task 004 removes only guards/review artifacts if they need
  adjustment before relanding.
