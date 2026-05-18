# ADR 0054 Task 002: MusicIndex Feed Metadata Ingest

## Goal

Persist feed-level MusicIndex metadata facts into `entity_metadata_facts` from
the existing MusicIndex feed ingest path, without rendering or hydrating new
Library fields.

## Files To Inspect

- `docs/adr/0054-local-metadata-source-fact-persistence.md`
- `docs/plans/adr-0054-local-metadata-source-fact-persistence-phase-plan.md`
- `docs/tasks/adr-0054-task-001-schema-and-db-helpers.md`
- `src/db.rs`
- `src/identity_ingest.rs`
- `src/api.rs`
- `src/feed_service.rs`
- `src/library/app_impl.rs`

## Files Likely To Change

- `src/identity_ingest.rs`
- `tests/architecture_tests.rs` only if an architecture guard needs a narrow
  update

## Do Not Touch

- `src/ui/**`
- `src/view_models/**`
- `src/views.rs`
- `src/local_identity.rs`
- `src/feed_service.rs`
- `src/subscribe_service.rs`
- `src/rss/**`
- `src/discover/**`
- schema / migration definitions in `src/db.rs`

## Constraints

- Implement only MusicIndex feed metadata ingest.
- Do not render or hydrate persisted facts into `FeedView` or any UI.
- Do not persist track metadata in this task.
- Do not map RSS `podcast_medium` to MusicIndex `release_kind`.
- Keep `source_release_claims` provenance when a claim is persisted.
- Use the existing source-scoped replacement helper from Task 001.
- Preserve source-scoped replacement semantics for RSS/other source rows.

## Fact Keys

Persist these feed-level MusicIndex facts when non-empty / present:

- `publisher_text` from `api::Feed::publisher_text`
- `musicindex_release_kind` from `api::Feed::release_kind`
- `release_date` from `api::Feed::release_date`
- `language` from `api::Feed::language`
- `explicit` from `api::Feed::explicit`
- `description` from `api::Feed::description`
- `description` from `api::Feed::source_release_claims` rows where
  `claim_type == "description"` and `claim_value` is non-empty

For top-level `api::Feed` fields, `raw_json` may be the serialized full feed
object and `extraction_path` should identify the field path. For
`SourceReleaseClaim` rows, use the claim's own `source`, `extraction_path`,
`observed_at`, and raw JSON.

## Implementation Steps

1. Add a helper in `src/identity_ingest.rs` that converts an `api::Feed` into
   grouped `LocalMetadataFactInput` rows by source token.
2. Call the helper from `persist_musicindex_feed` after identity links, ids,
   and contributors are persisted.
3. Keep the helper private to `identity_ingest.rs`.
4. Add unit tests proving:
   - feed metadata facts are persisted for all supported top-level fields
   - source release description claims persist as metadata facts with claim
     provenance
   - empty strings are skipped
   - MusicIndex replacement preserves existing RSS metadata rows

## Acceptance Criteria

- Existing calls to `persist_musicindex_feed` persist feed metadata facts.
- No UI, view-model, view, RSS, subscribe, or Discover files change.
- Source-specific claim rows do not get collapsed into the MusicIndex source
  token when they carry their own source.
- No track metadata facts are written.
- Existing identity persistence tests still pass.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test identity_ingest::tests::musicindex_feed_metadata --lib --quiet`
- `cargo test --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

## Expected Final Report Format

1. Files changed
2. Tests run
3. Behavior changed
4. Deviations from task
5. Unresolved concerns

## Escalation Triggers

- A required field needs schema changes.
- A persisted fact would need renderer or view-model logic to validate.
- The implementation needs to infer release kind from RSS `podcast_medium`.
- Feed and track metadata cannot be separated because of feed-default copying.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0054-local-metadata-source-fact-persistence.md`
- `docs/plans/adr-0054-local-metadata-source-fact-persistence-phase-plan.md`
- `docs/tasks/adr-0054-task-002-musicindex-feed-metadata-ingest.md`
- `src/db.rs`
- `src/identity_ingest.rs`
- `src/api.rs`

Goal:
- Persist feed-level MusicIndex metadata facts from `persist_musicindex_feed`
  into `entity_metadata_facts`.

Constraints:
- Do not render or hydrate persisted facts into UI/read models.
- Do not persist track metadata.
- Do not touch schema/migrations.
- Do not map RSS `podcast_medium` to MusicIndex `release_kind`.
- Preserve source-specific provenance from `SourceReleaseClaim` rows.
- Use `replace_local_metadata_facts` / `LocalMetadataOwner::Feed`.

Do not touch:
- `src/ui/**`
- `src/view_models/**`
- `src/views.rs`
- `src/local_identity.rs`
- `src/feed_service.rs`
- `src/subscribe_service.rs`
- `src/rss/**`
- `src/discover/**`
- schema / migration definitions in `src/db.rs`

Acceptance criteria:
- Existing calls to `persist_musicindex_feed` persist `publisher_text`,
  `musicindex_release_kind`, `release_date`, `language`, `explicit`,
  top-level `description`, and description source-release claims when present.
- Empty text facts are skipped.
- Source-specific claim rows keep their source token.
- Existing RSS/other metadata rows survive a MusicIndex replacement.
- No track metadata facts are written.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test identity_ingest::tests::musicindex_feed_metadata --lib --quiet`
- `cargo test --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
