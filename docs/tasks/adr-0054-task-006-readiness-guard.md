# ADR 0054 Task 006: Readiness Guard

## Goal

Lock ADR 0054's metadata source-fact boundaries after feed and track ingest
and hydration are implemented.

This is a guard-only packet. It should not change product behavior, schema, or
visual presentation.

## Files To Inspect

- `docs/adr/0054-local-metadata-source-fact-persistence.md`
- `docs/plans/adr-0054-local-metadata-source-fact-persistence-phase-plan.md`
- `docs/reviews/adr-0054-review-checklist.md`
- `docs/tasks/adr-0054-task-001-schema-and-db-helpers.md`
- `docs/tasks/adr-0054-task-002-musicindex-feed-metadata-ingest.md`
- `docs/tasks/adr-0054-task-003-musicindex-track-metadata-ingest.md`
- `docs/tasks/adr-0054-task-004-feed-read-model-hydration.md`
- `docs/tasks/adr-0054-task-005-track-read-model-hydration.md`
- `tests/architecture_tests.rs`
- `src/db.rs`
- `src/identity_ingest.rs`
- `src/local_metadata.rs`
- `src/views.rs`
- `src/sources.rs`
- `src/feed_service.rs`
- `src/library/app_impl.rs`
- `src/view_models/**`
- `src/ui/**`

## Files Likely To Change

- `tests/architecture_tests.rs`
- `docs/reviews/adr-0054-review-checklist.md`
- `docs/reviews/adr-0054-task-006-review.md`

## Do Not Touch

- `src/db.rs`
- `src/identity_ingest.rs`
- `src/local_metadata.rs`
- `src/views.rs`
- `src/sources.rs`
- `src/feed_service.rs`
- `src/library/app_impl.rs`
- `src/ui/**`
- `src/view_models/**`
- Schema or migrations
- Runtime behavior

## Constraints

- Add or strengthen guards only.
- Do not change fact-key names, source tokens, ingest behavior, hydration
  behavior, UI layout, or visual strings.
- Architecture guards may inspect source text, but they must avoid brittle
  formatting assumptions where a semantic pattern is enough.
- Preserve existing tests that already cover DB row constraints. Do not move DB
  unit tests into architecture tests.
- Metadata storage access must remain out of UI and view-model layers.
- `musicindex_release_kind` and `rss_podcast_medium` must remain distinct.
- Feed and track metadata fact keys must remain owner-scoped:
  - feed keys: `publisher_text`, `musicindex_release_kind`, `release_date`,
    `language`, `explicit`, `description`, `rss_podcast_medium`
  - track keys: `publisher_text`, `description`, `pub_date`, `explicit`

## Implementation Steps

1. Review existing ADR 0054 architecture and DB tests. Do not duplicate row
   constraint tests already covered by `src/db.rs`.
2. Strengthen `tests/architecture_tests.rs` so future changes cannot:
   - query or mutate `entity_metadata_facts` outside `src/db.rs`
   - call `db::local_metadata_facts` or `db::replace_local_metadata_facts`
     from UI/view-model layers
   - import metadata DB owner/value/input/row types into UI/view-model layers
   - collapse `rss_podcast_medium` into `musicindex_release_kind`
   - introduce unsupported ADR 0054 fact keys in ingest/hydration tests or
     production mappings
3. Keep allowed metadata-storage callers explicit. Expected non-DB callers are
   service/read-model boundaries such as `src/identity_ingest.rs`,
   `src/local_metadata.rs`, and narrow application/service integration paths
   already introduced by Tasks 002-005.
4. Update `docs/reviews/adr-0054-review-checklist.md` with pass/fail status
   for each required check and note the guard names.
5. Add `docs/reviews/adr-0054-task-006-review.md` with the final result,
   verification commands, and merge recommendation.

## Acceptance Criteria

- ADR 0054 architecture tests fail if UI/view-model code directly accesses
  metadata storage helpers or storage types.
- ADR 0054 architecture tests fail if raw `entity_metadata_facts` SQL appears
  outside `src/db.rs`.
- ADR 0054 architecture tests fail if `rss_podcast_medium` is mapped into
  `musicindex_release_kind` or read-model release kind hydration.
- ADR 0054 architecture tests fail if production ingest/hydration mappings add
  unsupported feed or track metadata fact keys without updating the guard.
- Existing DB tests remain the source of truth for row shape, source-scoped
  replacement, exactly-one typed value, empty source/key rejection, and cascade
  behavior.
- Review checklist is updated to reflect ADR 0054 readiness.
- No runtime or visual behavior changes.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo test local_metadata --lib --quiet`
- `cargo test identity_ingest --lib --quiet`
- `cargo test --lib --quiet`
- `cargo clippy --quiet -- -D warnings`
- `cargo build --quiet`
- `git diff --check`

## Expected Final Report Format

1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- A guard requires runtime code changes to pass.
- A fact key outside the approved ADR 0054 set appears necessary.
- UI/view-model code requires direct metadata storage access.
- Readiness cannot be claimed without changing schema or ingest behavior.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0054-local-metadata-source-fact-persistence.md`
- `docs/plans/adr-0054-local-metadata-source-fact-persistence-phase-plan.md`
- `docs/tasks/adr-0054-task-006-readiness-guard.md`
- `docs/reviews/adr-0054-review-checklist.md`
- `tests/architecture_tests.rs`
- `src/db.rs`
- `src/identity_ingest.rs`
- `src/local_metadata.rs`
- `src/views.rs`
- `src/sources.rs`
- `src/feed_service.rs`
- `src/library/app_impl.rs`

Goal:
- Add ADR 0054 readiness guards and update the ADR 0054 review docs.

Constraints:
- Add or strengthen guards only.
- Do not change runtime behavior, schema, ingest, hydration, UI, or view-model
  code.
- Keep metadata storage access out of UI and view-model layers.
- Keep `musicindex_release_kind` and `rss_podcast_medium` distinct.
- Do not add unsupported fact keys.

Do not touch:
- `src/db.rs`
- `src/identity_ingest.rs`
- `src/local_metadata.rs`
- `src/views.rs`
- `src/sources.rs`
- `src/feed_service.rs`
- `src/library/app_impl.rs`
- `src/ui/**`
- `src/view_models/**`

Acceptance criteria:
- Architecture guards cover ADR 0054 storage boundaries, UI/view-model
  coupling, release-kind distinction, and approved feed/track fact keys.
- Existing DB unit tests remain responsible for row-shape and cascade behavior.
- ADR 0054 review checklist and Task006 review are updated.
- All required test commands pass.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo test local_metadata --lib --quiet`
- `cargo test identity_ingest --lib --quiet`
- `cargo test --lib --quiet`
- `cargo clippy --quiet -- -D warnings`
- `cargo build --quiet`
- `git diff --check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
