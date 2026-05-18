# ADR 0054 Task 003: MusicIndex Track Metadata Ingest

## Goal

Persist track-level MusicIndex metadata facts into `entity_metadata_facts`
without mis-owning feed-default copied fields as track source facts.

## Files To Inspect

- `docs/adr/0054-local-metadata-source-fact-persistence.md`
- `docs/plans/adr-0054-local-metadata-source-fact-persistence-phase-plan.md`
- `docs/tasks/adr-0054-task-001-schema-and-db-helpers.md`
- `docs/tasks/adr-0054-task-002-musicindex-feed-metadata-ingest.md`
- `src/db.rs`
- `src/identity_ingest.rs`
- `src/subscribe_service.rs`
- `src/api.rs`
- `src/feed_service.rs`

## Files Likely To Change

- `src/identity_ingest.rs`
- `src/subscribe_service.rs`

## Do Not Touch

- `src/ui/**`
- `src/view_models/**`
- `src/views.rs`
- `src/local_identity.rs`
- `src/feed_service.rs`
- `src/rss/**`
- `src/discover/**`
- schema / migration definitions in `src/db.rs`

## Constraints

- Implement only MusicIndex track metadata ingest.
- Do not render or hydrate persisted facts into `TrackView` or any UI.
- Do not persist feed/release metadata as track facts.
- Do not persist `source_release_claims` as track metadata in this task; those
  are release claims and can be feed-default copied.
- Do not add track language or lyrics/annotation; product scope is unresolved.
- Use the existing source-scoped replacement helper from Task 001.
- Preserve RSS/other source rows when MusicIndex track metadata is refreshed.
- `track_with_feed_defaults` may still be used for display/download context,
  but persistence must receive the non-defaulted track payload.

## Fact Keys

Persist these track-level MusicIndex facts when non-empty / present:

- `publisher_text` from `api::Track::publisher_text`
- `description` from `api::Track::description`
- `pub_date` from `api::Track::pub_date`
- `explicit` from `api::Track::explicit`

For top-level `api::Track` fields, `raw_json` may be the serialized full track
object and `extraction_path` should identify the field path.

## Implementation Steps

1. Add a helper in `src/identity_ingest.rs` that converts an `api::Track` into
   `LocalMetadataFactInput` rows under the MusicIndex source token.
2. Call the helper from `persist_musicindex_track` after identity facts and
   artist bindings.
3. In `src/subscribe_service.rs`, ensure the persistence call inside
   `subscribe_track_from_search_internal` uses the original non-defaulted track
   payload, not the `track_with_feed_defaults` result.
4. In the bulk feed-download path, pass a non-defaulted track context into
   `subscribe_track_from_search_internal`; continue using the enriched
   defaulted context for ID3 edit generation.
5. For public search-track download callers that only have a display/download
   context, fetch an authoritative MusicIndex track payload by track GUID before
   persistence. If no authoritative payload is available, skip track persistence
   rather than falling back to a feed-defaulted context.
6. Add unit tests proving:
   - track metadata facts are persisted for all supported top-level fields
   - empty strings are skipped and `explicit = false` is preserved
   - MusicIndex replacement preserves existing RSS track metadata rows
   - feed-default copied publisher/description are not written as track facts
   - public search-track fallback does not persist defaulted context metadata

## Acceptance Criteria

- Existing calls to `persist_musicindex_track` persist only track-level
  MusicIndex metadata facts.
- Feed-default copied metadata is not persisted as track-owned metadata.
- Public search-track callers do not persist metadata from a defaulted
  display/download context when an authoritative MusicIndex track payload cannot
  be fetched.
- No UI, view-model, view, RSS, Discover, or schema files change.
- No `source_release_claims` are persisted as track metadata.
- Existing identity and artist-binding persistence tests still pass.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test identity_ingest::tests::musicindex_track_metadata --lib --quiet`
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
- The implementation needs to infer track language or lyrics/annotation.
- The implementation cannot separate download/display defaults from persisted
  track source facts.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0054-local-metadata-source-fact-persistence.md`
- `docs/plans/adr-0054-local-metadata-source-fact-persistence-phase-plan.md`
- `docs/tasks/adr-0054-task-003-musicindex-track-metadata-ingest.md`
- `src/db.rs`
- `src/identity_ingest.rs`
- `src/subscribe_service.rs`
- `src/api.rs`

Goal:
- Persist track-level MusicIndex metadata facts from `persist_musicindex_track`
  into `entity_metadata_facts`, and keep feed-default copied metadata out of
  track-owned facts.

Constraints:
- Do not render or hydrate persisted facts into UI/read models.
- Do not persist feed/release metadata as track facts.
- Do not persist `source_release_claims` as track metadata.
- Do not touch schema/migrations.
- Do not add track language, lyrics, or annotation.
- Use `replace_local_metadata_facts` / `LocalMetadataOwner::Track`.
- Keep display/download use of `track_with_feed_defaults`, but use the
  non-defaulted track payload for persistence.

Do not touch:
- `src/ui/**`
- `src/view_models/**`
- `src/views.rs`
- `src/local_identity.rs`
- `src/feed_service.rs`
- `src/rss/**`
- `src/discover/**`
- schema / migration definitions in `src/db.rs`

Acceptance criteria:
- Existing calls to `persist_musicindex_track` persist `publisher_text`,
  `description`, `pub_date`, and `explicit` when present.
- Empty text facts are skipped and `explicit = false` is preserved.
- Existing RSS/other track metadata rows survive a MusicIndex replacement.
- Feed-default copied publisher/description are not persisted as track facts.
- No `source_release_claims` are persisted as track metadata.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test identity_ingest::tests::musicindex_track_metadata --lib --quiet`
- `cargo test --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
