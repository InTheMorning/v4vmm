# ADR 0054 Task 001: Metadata Source-Fact Schema And DB Helpers

## Goal

Add additive SQLite storage and DB helpers for feed/track metadata source
facts, without changing ingest, query hydration, view models, renderers, or UI.

## Files To Inspect

- `docs/adr/0054-local-metadata-source-fact-persistence.md`
- `docs/plans/adr-0054-local-metadata-source-fact-persistence-phase-plan.md`
- `docs/adr/0028-local-identity-source-fact-persistence.md`
- `src/db.rs`
- `src/local_identity.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/db.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/ui/**`
- `src/view_models/**`
- `src/views.rs`
- `src/identity_ingest.rs`
- `src/feed_service.rs`
- `src/subscribe_service.rs`
- `src/rss/**`
- `src/discover/**`
- migrations outside the existing `src/db.rs` registry

## Constraints

- Follow ADR 0054 exactly; do not redesign the metadata source-fact model.
- Do not persist or hydrate real MusicIndex/RSS fields in this task.
- Do not add renderer fallback text.
- Keep metadata facts separate from identity source facts.
- Source replacement is scoped to one `(owner_kind, owner id, source)` only.
- Reject empty source tokens and empty fact keys.
- Enforce owner-shape `CHECK` constraints for `feed` and `track` rows.
- Enforce that exactly one typed value slot is populated.

## Implementation Steps

1. Add `LocalMetadataOwner`, `LocalMetadataValue`, metadata fact input, and
   metadata fact row types near the existing local source-fact DB types.
2. Add `entity_metadata_facts` schema creation under the existing DB migration
   registry.
3. Add DB helpers:
   - `replace_local_metadata_facts`
   - `local_metadata_facts`
4. Add DB tests for schema creation, round trip, source-scoped replacement,
   invalid owner shape, invalid value shape, empty source/fact rejection, and
   feed/track cascade.
5. Add or strengthen an architecture test that prevents UI/view-model layers
   from querying `entity_metadata_facts` directly.

## Acceptance Criteria

- The new table is additive and created by `init_schema`.
- Source-scoped replacement preserves other source rows for the same owner.
- Feed deletion removes feed metadata facts.
- Track deletion removes track metadata facts.
- Invalid owner/value shapes fail at the database boundary.
- No UI, VM, ingest, subscribe, or renderer files change.
- Architecture tests guard the table boundary.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test db::tests::test_metadata_source_fact --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

## Expected Final Report Format

1. Files changed
2. Tests run
3. Behavior changed
4. Deviations from task
5. Unresolved concerns

## Escalation Triggers

- A needed field requires ingest or view-model changes.
- Existing schema migration helpers cannot support an additive table safely.
- The implementation appears to need renderer logic.
- Any change would collapse RSS `podcast_medium` into MusicIndex
  `release_kind`.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0054-local-metadata-source-fact-persistence.md`
- `docs/plans/adr-0054-local-metadata-source-fact-persistence-phase-plan.md`
- `docs/adr/0028-local-identity-source-fact-persistence.md`
- `src/db.rs`
- `tests/architecture_tests.rs`

Goal:
- Add additive SQLite storage and DB helpers for feed/track metadata source
  facts.

Constraints:
- Do not add ingest, query hydration, view-model, renderer, UI, or RSS behavior.
- Keep metadata facts separate from identity source facts.
- Source replacement must be scoped to one `(owner_kind, owner id, source)`.
- Reject empty source tokens and empty fact keys.
- Enforce owner-shape checks and exactly-one-typed-value checks.
- Use existing `src/db.rs` migration/test patterns.

Do not touch:
- `src/ui/**`
- `src/view_models/**`
- `src/views.rs`
- `src/identity_ingest.rs`
- `src/feed_service.rs`
- `src/subscribe_service.rs`
- `src/rss/**`
- `src/discover/**`

Acceptance criteria:
- `entity_metadata_facts` exists and is additive.
- `replace_local_metadata_facts` and `local_metadata_facts` exist.
- DB tests cover round trip, source-scoped replacement, invalid owner/value
  shape, empty source/fact rejection, and cascade behavior.
- Architecture tests block direct UI/view-model access to
  `entity_metadata_facts`.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test db::tests::test_metadata_source_fact --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
