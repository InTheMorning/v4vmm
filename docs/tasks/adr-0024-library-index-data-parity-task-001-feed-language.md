# ADR 0024 Library / Index Data Parity Task 001 — Feed Language Loading Shape

## Goal

Surface already-persisted `feeds.language` in Library album / release detail by
carrying it through the local read model and GPUI-free view-model path.

This task implements only the first loading-shape slice from
`docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md`.

## Files To Inspect

- `docs/adr/0052-library-index-data-parity-triage.md`
- `docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md`
- `docs/reviews/library-discover-parity-triage-album.md`
- `src/db.rs`
- `src/library/app_impl.rs`
- `src/view_models/library.rs`
- `src/views.rs`
- `src/view_models/feed.rs`
- `src/view_models/entity_detail.rs`
- `src/ui/shells/library/feed_detail.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/db.rs`
- `src/library/app_impl.rs`
- `src/view_models/library.rs`
- `src/views.rs`
- `src/ui/shells/library/feed_detail.rs`
- `src/sources.rs`
- Tests in `src/views.rs`, `src/view_models/library.rs`, or
  `tests/architecture_tests.rs`

## Do Not Touch

- `src/discover/**`
- `src/view_models/search_results/**`
- `src/app/search_dispatch.rs`
- `src/ui/shells/search_results_inspector.rs`
- SQLite schema / migrations
- Any source-fact or persistence design from ADR 0053

## Constraints

- Do not infer language in a renderer.
- Do not derive feed language from track data.
- Do not add release date, release kind, publisher, explicit, description, or
  any other parity field.
- Keep the source of truth as persisted `feeds.language`.
- Preserve existing `FeedView::from_api` behavior.
- No new `#[allow(...)]`; use `#[expect(...)]` only if an intentional lint
  suppression is truly required.

## Implementation Steps

1. Add `language: Option<String>` to `db::FeedRow`.
2. Update `db::subscribed_feeds` to select and hydrate `feeds.language`.
3. Update any direct `FeedRow` queries in local source paths to select and
   hydrate `language`.
4. Add `language: Option<String>` to `AlbumNode` so Library album detail can
   carry the local feed language snapshot.
5. Populate `AlbumNode::language` from subscribed feed rows in the tree build
   path and fallback album-detail lookup path.
6. Pass `AlbumNode::language` into the `FeedRow` built by
   `render_library_feed_detail`.
7. Change `FeedView::from_local_with_identity` to copy the local
   `FeedRow::language` through `nonempty_owned`.
8. Add focused regression coverage proving local feed language reaches
   `FeedView` and the shared detail facts without renderer inference.
9. Add or strengthen an architecture guard that keeps the field routed through
   the local DB/read-model/view-model path rather than `src/discover/` or a
   renderer fallback.

## Acceptance Criteria

- Library album detail can show the existing shared `Language` summary fact
  when `feeds.language` is present.
- Empty or whitespace-only local language is filtered out in the VM layer.
- No source-fact schema or persistence design changes are introduced.
- No Index, Discover, search result, or source-fact behavior changes.
- Regression coverage exists for the same loading-shape path.
- Architecture coverage pins that `FeedRow`, `subscribed_feeds`,
  `AlbumNode`, and `FeedView::from_local_with_identity` carry language.

## Test Commands

```bash
cargo fmt -- --check
cargo check --quiet
cargo test --lib --quiet
cargo test --test architecture_tests --quiet
cargo clippy --quiet -- -D warnings
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
- `docs/tasks/adr-0024-library-index-data-parity-task-001-feed-language.md`
- `docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md`
- `docs/reviews/library-discover-parity-triage-album.md`
- `src/db.rs`
- `src/library/app_impl.rs`
- `src/view_models/library.rs`
- `src/views.rs`
- `src/ui/shells/library/feed_detail.rs`
- `src/sources.rs`
- `tests/architecture_tests.rs`

Goal:
- Surface persisted `feeds.language` in Library album / release detail by
  carrying it through `FeedRow`, `AlbumNode`, and local `FeedView`.

Constraints:
- Do not infer language in a renderer.
- Do not derive language from track data.
- Do not add any other parity field.
- Do not touch `src/discover/**`, search result VMs, or Index detail code.
- No schema or migration changes.
- No new `#[allow(...)]`.
- You are not alone in the codebase. Do not revert unrelated edits.

Do not touch:
- `src/discover/**`
- `src/view_models/search_results/**`
- `src/app/search_dispatch.rs`
- `src/ui/shells/search_results_inspector.rs`
- SQLite schema / migrations
- ADR 0053 source-fact design

Acceptance criteria:
- `feeds.language` is loaded into `FeedRow`.
- `AlbumNode` carries language for Library album details.
- `FeedView::from_local_with_identity` sets `language` from local feed rows
  after non-empty filtering.
- Focused regression coverage proves the local language reaches shared detail
  facts.
- Architecture coverage pins the intended loading path.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- You find that local feed language is not actually persisted in
  `feeds.language`.
- Showing language requires touching Index/Discover/search code.
- Existing tests show local `FeedView` intentionally drops language for a
  product reason.
- The change requires a schema migration or source-fact design decision.
