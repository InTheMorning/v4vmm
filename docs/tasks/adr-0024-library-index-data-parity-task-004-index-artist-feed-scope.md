# ADR 0024 Library / Index Data Parity Task 004 - Index Artist Feed Scope

## Goal

Resolve the artist detail decision slice from
`docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md`.

Index artist activation remains a scoped Index feed-results drill-down, not a
dedicated remote artist detail page. Make that decision explicit in navigation
and regression guards so future parity work does not accidentally treat a
result scope as an artist identity/detail surface.

## Files To Inspect

- `docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md`
- `docs/reviews/library-discover-parity-triage-artist-playlist.md`
- `src/app.rs`
- `src/app/search_dispatch.rs`
- `src/view_models/workspace/nav.rs`
- `src/view_models/workspace/breadcrumb.rs`
- `src/view_models/workspace/tests.rs`
- `src/view_models/search_results/index_detail.rs`
- `src/view_models/search_results/results.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/app.rs`
- `src/app/search_dispatch.rs`
- `src/view_models/workspace/nav.rs`
- `src/view_models/workspace/breadcrumb.rs`
- `src/view_models/workspace/tests.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/discover/**`
- `src/ui/shells/library/**`
- `src/view_models/library.rs`
- `src/view_models/artist_detail.rs`
- `src/db.rs`
- SQLite schema / migrations
- ADR 0053 source-fact design
- artist/person identity persistence or reconciliation

## Decision

- Do not introduce an `IndexArtistDetail` page in this task.
- Keep the existing scoped Index feed-results behavior when an Index artist
  result is selected.
- Rename the navigation concept away from `IndexArtistDetail` to an
  artist-feed-scope name, because the destination is a filtered feed list.
- Keep breadcrumb behavior: `Library > Search: query > Artist Name > Feed`.
- Keep the selected artist breadcrumb segment selectable so operators can go
  back up from a feed drill-down to the scoped feed list.
- Keep Index artist result rows as search results only; they should not carry
  or imply canonical person identity facts.

## Constraints

- This is a route/ownership cleanup, not a new remote detail feature.
- Do not add an `IndexDetailKind::Artist`.
- Do not reuse `ArtistDetailPageVm` for Index artist rows.
- Do not change Library artist detail behavior.
- Do not change Index feed or track detail behavior.
- Do not infer artist sort name, area, active years, website, aliases,
  external ids, biography, or explicit state in renderers.
- No new `#[allow(...)]`; use `#[expect(...)]` only if an intentional lint
  suppression is truly required.

## Implementation Steps

1. Rename `FrameNavigationEntry::IndexArtistDetail(String)` to a scoped
   feed-results name such as `IndexArtistFeedScope(String)`.
2. Update selection dispatch so Index artist result activation pushes the
   renamed scoped-feed navigation entry.
3. Update `render_workspace_content` and frame-title logic so the scoped entry
   renders `SearchResultsHeaderMode::Scoped` for `SearchResultsTab::Feeds` and
   `ContentFilter::Index`.
4. Preserve breadcrumb ids, labels, and selectability for the scoped artist
   parent segment.
5. Update workspace/breadcrumb tests to assert the new route name and the
   preserved path semantics.
6. Add or strengthen an architecture guard proving:
   - `IndexArtistDetail` no longer exists as a route name;
   - no `IndexDetailKind::Artist` exists;
   - Index artist activation still routes to a scoped feed-results page;
   - the scoped artist segment remains in the breadcrumb path.

## Acceptance Criteria

- No live code refers to `FrameNavigationEntry::IndexArtistDetail`.
- Index artist result selection still opens scoped Index feed results.
- Breadcrumbs still include `Library > Search: query > Artist Name` before an
  Index feed drill-down, and the artist segment remains selectable.
- There is no fake Index artist detail page, no `IndexDetailKind::Artist`, and
  no renderer-side artist source-fact inference.
- Existing Index feed and track drill-down behavior remains intact.

## Test Commands

```bash
cargo fmt -- --check
cargo check --quiet
cargo build --quiet
cargo test --lib --quiet
cargo test --test architecture_tests --quiet
cargo clippy --quiet -- -D warnings
git diff --check
```

## Expected Final Summary Format

1. Files changed.
2. Tests run.
3. Behavior changed.
4. Deviations from task.
5. Unresolved concerns.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/tasks/adr-0024-library-index-data-parity-task-004-index-artist-feed-scope.md`
- `docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md`
- `docs/reviews/library-discover-parity-triage-artist-playlist.md`
- `src/app.rs`
- `src/app/search_dispatch.rs`
- `src/view_models/workspace/nav.rs`
- `src/view_models/workspace/breadcrumb.rs`
- `src/view_models/workspace/tests.rs`
- `src/view_models/search_results/index_detail.rs`
- `tests/architecture_tests.rs`

Goal:
- Make Index artist activation explicitly owned as a scoped Index feed-results
  route, not an Index artist detail page.

Constraints:
- Rename `FrameNavigationEntry::IndexArtistDetail` to a scoped-feed route name.
- Keep behavior: artist result -> scoped Index feeds; feed drill-down preserves
  breadcrumb parent.
- Do not add `IndexDetailKind::Artist`.
- Do not use `ArtistDetailPageVm` for Index artist rows.
- Do not touch Library artist detail, Discover, schema, migrations, or ADR
  0053 source-fact persistence.
- No renderer-side artist source-fact inference.
- No new `#[allow(...)]`.
- You are not alone in the codebase. Do not revert unrelated edits.

Do not touch:
- `src/discover/**`
- `src/ui/shells/library/**`
- `src/view_models/library.rs`
- `src/view_models/artist_detail.rs`
- `src/db.rs`
- SQLite schema / migrations
- ADR 0053 source-fact design

Acceptance criteria:
- `IndexArtistDetail` route naming is retired.
- The renamed route renders the same scoped Index feed result page.
- Breadcrumb tests prove the artist scope remains a selectable parent.
- Architecture tests guard against fake Index artist detail.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo build --quiet`
- `cargo test --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`
- `git diff --check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- The current scoped feed-result behavior cannot be preserved with a route
  rename.
- Any implementation path requires artist source-fact persistence, person
  identity reconciliation, or renderer-side source-fact inference.
- The implementation needs to change Library artist detail behavior.
