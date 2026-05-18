# ADR 0054 Task 005: Track Read-Model Hydration

## Goal

Hydrate local track detail read models from persisted track metadata source
facts without querying metadata storage from UI/render code or inventing
renderer fallbacks.

## Files To Inspect

- `docs/adr/0054-local-metadata-source-fact-persistence.md`
- `docs/plans/adr-0054-local-metadata-source-fact-persistence-phase-plan.md`
- `docs/tasks/adr-0054-task-003-musicindex-track-metadata-ingest.md`
- `docs/tasks/adr-0054-task-004-feed-read-model-hydration.md`
- `src/db.rs`
- `src/local_metadata.rs`
- `src/views.rs`
- `src/sources.rs`
- `src/feed_service.rs`
- `src/library/app_impl.rs`
- `src/ui/shells/library/track_detail.rs`
- `src/view_models/track_detail.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/local_metadata.rs`
- `src/views.rs`
- `src/sources.rs`
- `src/feed_service.rs`
- `src/library/app_impl.rs`
- tests in the same modules

## Do Not Touch

- `src/db.rs` schema or helper behavior
- `src/identity_ingest.rs`
- `src/subscribe_service.rs`
- `src/rss/**`
- Feed read-model hydration except for shared helper reuse
- Renderer-local metadata inference

## Constraints

- Implement only track read-model hydration.
- Metadata fact table access must stay in `src/local_metadata.rs` or
  non-UI service code. UI shells and view models must receive projected facts
  or already-hydrated contexts.
- Do not add schema, migration, ingest, or subscribe changes.
- Preserve scalar fallbacks for existing local track `pub_date` and `explicit`,
  but prefer preserved metadata facts when present.
- `TrackView` may gain a local-facts constructor, but renderers must not
  contain source-priority conditionals.
- Library track detail must hydrate local track metadata even when remote
  MusicIndex detail fetch is unavailable.
- When remote MusicIndex detail is available but omits one of the Task005
  track metadata fields, persisted local facts may fill the missing field but
  must not overwrite non-empty remote source text.
- Do not add unresolved fields such as track language, lyrics, or annotation.

## Track Fields To Hydrate

From persisted track metadata facts:

- `publisher_text` from `publisher_text`
- `description` from `description`
- `pub_date` from `pub_date`
- `explicit` from `explicit`

## Implementation Steps

1. Extend `src/local_metadata.rs` with a `TrackMetadataFacts` projection for
   `db::local_metadata_facts(LocalMetadataOwner::Track(track_id))`.
2. Add track metadata projection types to `src/views.rs`, and add a local
   `TrackView` constructor that accepts identity facts plus metadata facts.
3. Keep `TrackView::from_local` and `TrackView::from_local_with_identity`
   backwards-compatible by delegating to the new constructor with default
   metadata facts.
4. Update `src/sources.rs::local_track_view` to load local track metadata
   facts.
5. Update `src/feed_service.rs::track_row_to_track_context_with_local_identity`
   so local `TrackContext` hydration applies persisted track metadata facts to
   the API-shaped `Track` used by Library/Search detail surfaces.
6. Update Library track detail loading so remote MusicIndex failure still
   falls back to the local hydrated `TrackContext`, and remote success can use
   local metadata only for missing Task005 fields.
7. Add regression tests proving:
   - `TrackView` projects track metadata facts into publisher, description,
     pubdate, and explicit fields
   - scalar `pub_date` / `explicit` fallbacks still work when facts are absent
   - `LocalSource::fetch_track` hydrates track metadata facts
   - local `TrackContext` hydration applies track metadata facts
   - Library track context loading has a local hydrated fallback when remote
     detail is unavailable
   - local track metadata fills missing remote context fields without
     overwriting non-empty remote values

## Acceptance Criteria

- Local track detail can display publisher, description, pubdate, and explicit
  state from persisted track metadata facts.
- Existing scalar pubdate/explicit fallbacks remain for tracks without metadata
  facts.
- UI shells and view models do not query `entity_metadata_facts` or
  `db::local_metadata_facts` directly.
- No feed read-model behavior regresses.
- No track language, lyrics, annotation, schema, or ingest behavior changes.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test local_metadata --lib --quiet`
- `cargo test views::tests::from_local_track --lib --quiet`
- `cargo test sources --lib --quiet`
- `cargo test feed_service --lib --quiet`
- `cargo test library::app_impl --lib --quiet`
- `cargo test --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

## Expected Final Report Format

1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Hydration requires a schema or ingest change.
- A renderer must choose between source facts.
- RSS/feed defaults appear necessary to fill track-owned metadata facts.
- Track language, lyrics, or annotation appear necessary to satisfy this task.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0054-local-metadata-source-fact-persistence.md`
- `docs/plans/adr-0054-local-metadata-source-fact-persistence-phase-plan.md`
- `docs/tasks/adr-0054-task-005-track-read-model-hydration.md`
- `src/db.rs`
- `src/local_metadata.rs`
- `src/views.rs`
- `src/sources.rs`
- `src/feed_service.rs`
- `src/library/app_impl.rs`
- `src/ui/shells/library/track_detail.rs`
- `src/view_models/track_detail.rs`

Goal:
- Hydrate local track read models and local track contexts from persisted
  track metadata facts.

Constraints:
- Metadata storage access must stay out of UI shells and view models.
- Do not change schema, ingest, RSS, subscribe, or feed hydration behavior.
- Prefer metadata facts over scalar track columns; preserve scalar
  `pub_date`/`explicit` fallbacks when facts are absent.
- Keep renderers free of source-priority conditionals.
- Do not add track language, lyrics, or annotation.

Do not touch:
- `src/db.rs` schema/helper behavior
- `src/identity_ingest.rs`
- `src/subscribe_service.rs`
- `src/rss/**`

Acceptance criteria:
- `TrackView` can project persisted track metadata facts into publisher,
  description, pubdate, and explicit fields.
- `LocalSource::fetch_track` and local `TrackContext` construction receive the
  same hydrated metadata facts.
- Library track detail falls back to the local hydrated context when remote
  MusicIndex detail is unavailable.
- Remote Library track detail keeps remote metadata values and only uses local
  persisted track metadata for missing Task005 fields.
- UI/view-model architecture guards still pass.
- Existing behavior remains intact when facts are absent.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test local_metadata --lib --quiet`
- `cargo test views::tests::from_local_track --lib --quiet`
- `cargo test sources --lib --quiet`
- `cargo test feed_service --lib --quiet`
- `cargo test library::app_impl --lib --quiet`
- `cargo test --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
