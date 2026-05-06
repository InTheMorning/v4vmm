# ADR 0028 Task 002: Ingest Persistence

## Status

Implemented - 2026-05-01.

## Goal

Persist known MusicIndex and RSS identity source facts into the ADR 0028 local
source-fact tables during ingest/update workflows, without hydrating Library or
Discover views from those tables yet.

## Read

- `docs/adr/0028-local-identity-source-fact-persistence.md`
- `docs/plans/adr-0028-local-identity-source-fact-persistence-phase-plan.md`
- `docs/tasks/adr-0028-task-001-schema-and-db-helpers.md`
- `src/db.rs`
- `src/api.rs`
- `src/rss/subscribe.rs`
- `src/rss/helpers.rs`
- `src/subscribe_service.rs`
- `src/feed_service.rs`

## Files Likely To Change

- `src/db.rs`
- `src/lib.rs`
- `src/identity_ingest.rs`
- `src/rss/subscribe.rs`
- `src/subscribe_service.rs`
- `src/feed_service.rs`
- `docs/tasks/adr-0028-task-002-ingest-persistence.md`
- `docs/reviews/adr-0028-task-002-review.md`

## Do Not Touch

- Do not hydrate `FeedView::from_local` or `TrackView::from_local` yet.
- Do not change Library or Discover rendering.
- Do not introduce global artist/person reconciliation.
- Do not infer facts from names, titles, publisher text, filenames, or fuzzy
  matching.
- Do not remove or repurpose existing JSON/scalar columns.

## Constraints

- MusicIndex source facts must preserve raw `source_links`, `source_ids`, and
  contributor rows where provided by API payloads.
- RSS source facts may persist only direct source rows already present in the
  parsed feed/item, such as `podcast:person`, channel link, and transcript URL.
- Replacement must remain source-scoped. A MusicIndex refresh must not delete
  RSS rows, and an RSS refresh must not delete MusicIndex rows.
- Missing optional source vectors mean "not loaded" and must not clear existing
  rows. Present empty vectors may clear that source for the relevant owner.
- Contributor order remains source-snapshot position, not durable person
  identity.
- Helper code may translate concrete `api::*` or RSS structs at the ingest
  boundary, but DB helpers and shared projections must stay API-free.

## Implementation Steps

1. Add a small ingest-boundary helper module for MusicIndex API source facts.
2. Add local lookup support needed to map feed URL / item GUID to local owner
   IDs without duplicating ad hoc SQL at call sites.
3. Persist MusicIndex feed/track source links, source ids, and contributors
   after RSS subscription has created local feed/track rows.
4. Persist MusicIndex facts during feed update workflows that fetch fresh
   MusicIndex detail for existing local rows.
5. Persist RSS channel/item contributor rows and direct RSS source links from
   `rss::subscribe_feed`.
6. Add tests covering MusicIndex source preservation, source-scoped replacement,
   RSS contributor/transcript mapping, and lack of UI hydration changes.
7. Update this task and add a review file with verification results.

## Acceptance Criteria

- [x] MusicIndex feed source links, source ids, and contributors persist under
  the local feed owner.
- [x] MusicIndex track source links, source ids, and contributors persist under
  the local track owner.
- [x] RSS `podcast:person` rows persist as feed/track contributors with raw
  provenance where available.
- [x] RSS direct links such as channel website and item transcript URL persist
  as source links without inference.
- [x] Source-scoped replacement preserves unrelated source rows.
- [x] No Library/Discover local hydration or visual behavior changes are made.
- [x] Required verification commands pass.

## Implementation Notes

- Added `src/identity_ingest.rs` for MusicIndex API-to-local source fact
  translation at the ingest boundary.
- Wired MusicIndex source fact persistence in subscription and feed update
  workflows after local feed/track rows exist.
- Wired RSS `podcast:person`, channel link, and item transcript URL persistence
  from `rss::subscribe_feed`.
- Added `db::feed_id_by_url` to map feed URL to local owner ID without
  duplicating ad hoc SQL at call sites.
- Intentionally did not hydrate local Library/Discover views from these rows;
  that remains Task 003.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test identity_ingest
cargo test rss::subscribe
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
cargo test
```

## Expected Final Report

1. Files changed.
2. Tests run.
3. Ingest paths wired.
4. Behavior intentionally not wired yet.
5. Unresolved concerns for Task 003.

## Escalation Triggers

- API source facts cannot be mapped to local feed/track owners without changing
  subscription workflow ownership.
- RSS source rows require inference from display text.
- Source-scoped replacement would require deleting facts from unrelated
  sources.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0028-local-identity-source-fact-persistence.md`
- `docs/tasks/adr-0028-task-001-schema-and-db-helpers.md`
- `src/db.rs`
- `src/api.rs`
- `src/rss/subscribe.rs`
- `src/subscribe_service.rs`
- `src/feed_service.rs`

Goal:
- Persist MusicIndex and RSS identity source facts into the local ADR 0028 DB
  helpers during ingest/update workflows.

Constraints:
- Keep replacement source-scoped.
- Do not hydrate local views or change UI.
- Do not infer identity from names/titles/publisher text.
- Keep shared projections API-free and DB-free.

Do not touch:
- Library/Discover rendering.
- `src/views.rs` local hydration.
- Global artist/person reconciliation.

Acceptance criteria:
- MusicIndex feed/track links, ids, and contributors persist.
- RSS person rows and direct RSS links persist.
- Tests prove source separation and no unrelated-source deletion.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test identity_ingest`
- `cargo test rss::subscribe`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`
- `cargo test`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
