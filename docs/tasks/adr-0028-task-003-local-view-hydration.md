# ADR 0028 Task 003: Local View Hydration

## Status

Implemented - 2026-05-01.

## Goal

Hydrate local `FeedView`, `TrackView`, and contributor view inputs from the
persisted ADR 0028 source-fact rows, without changing Library or Discover
visual layout in this task.

## Read

- `docs/adr/0028-local-identity-source-fact-persistence.md`
- `docs/tasks/adr-0028-task-001-schema-and-db-helpers.md`
- `docs/tasks/adr-0028-task-002-ingest-persistence.md`
- `src/db.rs`
- `src/views.rs`
- `src/sources.rs`
- `src/feed_service.rs`
- `src/library_service.rs`

## Files Likely To Change

- `src/db.rs`
- `src/views.rs`
- `src/sources.rs`
- `src/feed_service.rs`
- `docs/plans/adr-0028-local-identity-source-fact-persistence-phase-plan.md`
- `docs/tasks/adr-0028-task-003-local-view-hydration.md`
- `docs/reviews/adr-0028-task-003-review.md`

## Do Not Touch

- Do not change Library or Discover screen layout/rendering.
- Do not add SQLite or service imports to `src/view_models/entity_detail.rs`.
- Do not add GPUI imports to `src/views.rs`.
- Do not infer identity from names, titles, publisher text, filenames, or fuzzy
  matching.
- Do not build global artist/person reconciliation.

## Constraints

- `src/views.rs` may accept already-loaded local identity facts, but must not
  query SQLite.
- Local hydration must preserve raw source-link/source-id vectors.
- Convenience fields such as website and Nostr identity actions must derive
  from persisted source facts.
- Contributor rows must hydrate from persisted contributor rows, including
  `href`, `image_url`, and `nostr_npub`.
- Existing local constructors should remain usable for callers that do not yet
  load identity facts.
- Track and feed local hydration should prefer persisted identity facts without
  discarding existing scalar image/transcript fallbacks.

## Implementation Steps

1. Add small DB-to-view fact conversion helpers or DB-owned bundle structs.
2. Add local hydration constructors in `src/views.rs` that accept loaded
   identity links, ids, and contributors.
3. Wire `LocalSource::fetch_feed`, `LocalSource::fetch_track`, and local
   artist feed listing to load persisted facts before constructing views.
4. Wire `feed_service::track_row_to_track_context` local context hydration if
   it can be done without network or UI coupling.
5. Add tests that local feed and track views expose persisted source links,
   source ids, website/Nostr convenience fields, and contributor identity.
6. Update this task and add a review file with verification results.

## Acceptance Criteria

- [x] Local feed views hydrate persisted `source_links` and `source_ids`.
- [x] Local feed views hydrate persisted contributors with `href`,
  `image_url`, and `nostr_npub`.
- [x] Local track views hydrate persisted `source_links` and `source_ids`.
- [x] Local track views hydrate persisted contributors.
- [x] Convenience identity fields are derived from persisted source facts.
- [x] `src/views.rs` remains GPUI-free and database-free.
- [x] No Library/Discover rendering code is directly changed.
- [x] Required verification commands pass.

## Implementation Notes

- Added `views::LocalIdentityFacts` and local constructors that accept
  already-loaded identity facts and contributors.
- Wired `LocalSource` feed/track fetches to load persisted source facts before
  constructing local views.
- Added `feed_service::track_row_to_track_context_with_local_identity` for
  non-UI metadata paths that need local persisted source facts.
- Preserved existing constructors for callers that do not load identity facts.
- Did not directly change Library or Discover rendering code; Task 004 owns
  visual smoke and any narrowly scoped display follow-up.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test views::tests
cargo test sources::tests
cargo test feed_service::tests
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
cargo test
```

## Expected Final Report

1. Files changed.
2. Tests run.
3. Local hydration paths wired.
4. Behavior intentionally not changed visually yet.
5. Unresolved concerns for Task 004.

## Escalation Triggers

- Local hydration requires screen code to query SQLite.
- Hydration would require moving API structs into DB helpers.
- Existing constructors cannot remain compatible without broad visual rewrites.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0028-local-identity-source-fact-persistence.md`
- `docs/tasks/adr-0028-task-001-schema-and-db-helpers.md`
- `src/db.rs`
- `src/views.rs`
- `src/sources.rs`
- `src/feed_service.rs`

Goal:
- Hydrate local feed/track/contributor views from persisted ADR 0028 source
  facts.

Constraints:
- Keep `src/views.rs` DB-free and GPUI-free.
- Do not change Library or Discover rendering.
- Do not infer identity from names/titles/publisher text.
- Preserve raw source-link/source-id vectors.

Do not touch:
- Library/Discover screen layout.
- Global artist/person identity.
- `src/view_models/entity_detail.rs` service or DB boundaries.

Acceptance criteria:
- Local feed and track views expose persisted links, ids, contributors, and
  convenience website/Nostr fields.
- Architecture tests remain green.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test views::tests`
- `cargo test sources::tests`
- `cargo test feed_service::tests`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`
- `cargo test`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
