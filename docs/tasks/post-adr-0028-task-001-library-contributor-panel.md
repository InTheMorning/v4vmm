# Post-ADR 0028 Task 001: Library Contributor Panel

## Status

Implemented - 2026-05-01.

## Goal

Render already-hydrated `FeedView::contributors` in Library release detail so
local contributor `href`, `image_url`, and `nostr_npub` facts can be inspected
without switching to Discover.

## Read

- `docs/adr/0028-local-identity-source-fact-persistence.md`
- `docs/plans/post-adr-0028-follow-up-plan.md`
- `src/ui_entity.rs`
- `src/library.rs`
- `src/search.rs`
- `src/view_models/entity_detail.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/ui_entity.rs`
- `src/library.rs`
- `src/search.rs`
- `tests/architecture_tests.rs`
- `docs/tasks/post-adr-0028-task-001-library-contributor-panel.md`
- `docs/reviews/post-adr-0028-task-001-review.md`

## Do Not Touch

- Do not change database schema or migrations.
- Do not change MusicIndex, RSS, ingest, subscription, download, playlist,
  playback, or MusicBrainz behavior.
- Do not add Library network fetching for contributors.
- Do not move click behavior into `src/view_models`.
- Do not introduce global artist/person identity matching.

## Constraints

- Render from `ReleaseDetailVm::contributors()` / `ContributorListVm`.
- Keep shared helper code free of screen modules, services, DB, and API row
  types.
- Keep website-open and Nostr-copy handlers in screen adapters.
- Preserve Discover contributor behavior while removing duplicated contributor
  row rendering where practical.
- Use text labels with identity buttons so actions are not color-only.

## Implementation Steps

1. Add a shared contributor panel/row helper to `src/ui_entity.rs`.
2. Let screen adapters supply thumbnails and action elements through a narrow
   callback or slot.
3. Route Discover's lazy contributor rows through the shared helper without
   changing lazy/collapsed behavior.
4. Add a Library release-detail `after_section` panel when the hydrated
   contributor list is non-empty.
5. Extend architecture tests to prevent returning to API-shaped contributor
   rendering in screen paths.
6. Write a review under `docs/reviews/`.

## Acceptance Criteria

- [x] Library release detail renders a Contributors section when
  `FeedView::contributors` is non-empty.
- [x] Contributor rows show grouped labels, optional website text, and supplied
  Website/Nostr actions.
- [x] Discover contributor rendering still uses the same lazy panel behavior.
- [x] Shared contributor rendering does not import screen, DB, service, or API
  modules.
- [x] Required verification commands pass.

## Implementation Summary

- Added shared contributor panel and row rendering in `src/ui_entity.rs`.
- Routed Discover's lazy contributor rows through the shared helper while
  preserving its collapsed/loading/empty states.
- Added a Library release-detail contributor panel that renders already-hydrated
  local contributor facts and reuses the existing thumbnail cache.
- Kept Website/Nostr click behavior in the screen adapters.
- Extended architecture tests so both Library and Discover stay on
  `ContributorView` plus shared contributor projections.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test view_models::entity_detail::tests::contributor_list_groups_by_group_name
cargo test views::tests::from_local_feed_hydrates_identity_facts_and_contributors
cargo test ui_entity::tests::contributor_rows_use_shared_projection_groups
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
cargo test
```

Verified 2026-05-01.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/tasks/post-adr-0028-task-001-library-contributor-panel.md`
- `src/ui_entity.rs`
- `src/library.rs`
- `src/search.rs`
- `src/view_models/entity_detail.rs`
- `tests/architecture_tests.rs`

Goal:
- Render already-hydrated Library release contributors through a shared
  contributor panel/row helper.

Constraints:
- Use `ReleaseDetailVm::contributors()` / `ContributorListVm`.
- Keep click behavior screen-owned.
- Keep shared helper code free of screen modules, services, DB, and API row
  types.
- Do not change persistence, ingest, schema, or network behavior.

Do not touch:
- `src/db.rs`
- `src/identity_ingest.rs`
- `src/rss/`
- download/playback/playlist/MusicBrainz command code

Acceptance criteria:
- Library renders a Contributors section for non-empty hydrated contributors.
- Discover still renders lazy contributor rows through the same behavior.
- Architecture tests cover the boundary.
- Verification commands pass.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test view_models::entity_detail::tests::contributor_list_groups_by_group_name`
- `cargo test views::tests::from_local_feed_hydrates_identity_facts_and_contributors`
- `cargo test ui_entity::tests::contributor_rows_use_shared_projection_groups`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`
- `cargo test`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Rendering contributors requires schema or ingest changes.
- Library needs network fetching to show the contributor panel.
- The shared helper needs to import screen modules or services.
