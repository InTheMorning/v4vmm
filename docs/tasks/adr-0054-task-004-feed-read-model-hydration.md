# ADR 0054 Task 004: Feed Read-Model Hydration

## Goal

Hydrate local feed detail read models from persisted feed metadata source facts
without querying metadata storage from UI/render code or inventing renderer
fallbacks.

## Files To Inspect

- `docs/adr/0054-local-metadata-source-fact-persistence.md`
- `docs/plans/adr-0054-local-metadata-source-fact-persistence-phase-plan.md`
- `docs/tasks/adr-0054-task-001-schema-and-db-helpers.md`
- `docs/tasks/adr-0054-task-002-musicindex-feed-metadata-ingest.md`
- `src/db.rs`
- `src/local_identity.rs`
- `src/views.rs`
- `src/sources.rs`
- `src/view_models/library.rs`
- `src/library/app_impl.rs`
- `src/ui/shells/library/feed_detail.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/lib.rs`
- `src/local_metadata.rs`
- `src/views.rs`
- `src/sources.rs`
- `src/view_models/library.rs`
- `src/library/app_impl.rs`
- `src/ui/shells/library/feed_detail.rs`
- `tests/architecture_tests.rs` only for a narrow guard update if needed

## Do Not Touch

- `src/db.rs` schema or helper behavior
- `src/identity_ingest.rs`
- `src/subscribe_service.rs`
- `src/rss/**`
- Track read-model hydration
- Renderer-local metadata inference

## Constraints

- Implement only feed read-model hydration.
- Metadata fact table access must stay in a GPUI-free helper adjacent to
  `local_identity`; UI shells and view models must receive projected facts.
- Do not map RSS `podcast_medium` to MusicIndex `release_kind`.
- Do not add schema, migration, or ingest changes.
- Preserve scalar fallbacks for existing local feed `language` and
  `description`, but prefer preserved metadata facts when present.
- `FeedView` may gain a local-facts constructor, but renderers must not contain
  source-priority conditionals.
- Local Library detail and `LocalSource` paths must hydrate the same metadata
  fields.

## Feed Fields To Hydrate

From persisted feed metadata facts:

- `publisher_text` from `publisher_text`
- `release_kind` from `musicindex_release_kind`
- `release_date` from `release_date`
- `language` from `language`
- `explicit` from `explicit`
- `description` from `description`

Description should prefer source-specific description facts over the
MusicIndex top-level description when both are present, matching remote
`FeedView::from_api` preference for source release claims.

## Implementation Steps

1. Add a `src/local_metadata.rs` helper that loads
   `db::local_metadata_facts(LocalMetadataOwner::Feed(feed_id))` and maps rows
   into a GPUI-free feed metadata projection type.
2. Add feed metadata projection types to `src/views.rs`, and add a local
   `FeedView` constructor that accepts identity facts plus metadata facts.
3. Keep `FeedView::from_local` and `FeedView::from_local_with_identity`
   backwards-compatible by delegating to the new constructor with default
   metadata facts.
4. Update `src/sources.rs::local_feed_view` to load local feed metadata facts.
5. Extend `AlbumNode` to carry local feed metadata facts. Populate them in
   `build_tree` and `album_for_detail_by_feed_id`.
6. Update `render_library_feed_detail` to pass prehydrated metadata facts into
   `FeedView`; do not query DB or source facts from the renderer.
7. Add regression tests proving:
   - `FeedView` projects feed metadata facts into release detail fields
   - scalar `language` / `description` fallbacks still work when facts are absent
   - local source feed fetch hydrates metadata facts
   - library tree album nodes carry metadata facts

## Acceptance Criteria

- Library local feed detail can display publisher, release kind, release date,
  language, explicit state, and description from persisted feed metadata facts.
- Existing scalar language/description fallbacks remain for feeds without
  metadata facts.
- UI shells and view models do not query `entity_metadata_facts` or
  `db::local_metadata_facts` directly.
- RSS `podcast_medium` remains unmapped to MusicIndex `release_kind`.
- No track metadata read-model behavior changes.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test local_metadata --lib --quiet`
- `cargo test views::tests::from_local_feed --lib --quiet`
- `cargo test sources --lib --quiet`
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
- RSS `podcast_medium` appears necessary to fill `release_kind`.
- Track read-model hydration becomes necessary to satisfy this task.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0054-local-metadata-source-fact-persistence.md`
- `docs/plans/adr-0054-local-metadata-source-fact-persistence-phase-plan.md`
- `docs/tasks/adr-0054-task-004-feed-read-model-hydration.md`
- `src/db.rs`
- `src/local_identity.rs`
- `src/views.rs`
- `src/sources.rs`
- `src/view_models/library.rs`
- `src/library/app_impl.rs`
- `src/ui/shells/library/feed_detail.rs`

Goal:
- Hydrate local feed read models from persisted feed metadata facts.

Constraints:
- Metadata storage access must stay out of UI shells and view models.
- Do not change schema, ingest, RSS, subscribe, or track hydration.
- Do not map RSS `podcast_medium` to MusicIndex `release_kind`.
- Prefer metadata facts over scalar feed columns; preserve scalar
  `language`/`description` fallbacks when facts are absent.
- Keep renderers free of source-priority conditionals.

Do not touch:
- `src/db.rs` schema/helper behavior
- `src/identity_ingest.rs`
- `src/subscribe_service.rs`
- `src/rss/**`
- Track read-model hydration

Acceptance criteria:
- `FeedView` can project persisted feed metadata facts into publisher,
  release kind, release date, language, explicit, and description fields.
- `LocalSource::fetch_feed` and Library feed detail receive the same hydrated
  metadata facts.
- UI/view-model architecture guards still pass.
- Existing Library and local source behavior remains intact when facts are
  absent.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test local_metadata --lib --quiet`
- `cargo test views::tests::from_local_feed --lib --quiet`
- `cargo test sources --lib --quiet`
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
