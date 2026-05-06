# Post-ADR 0026 Task 003: Artwork Source Expansion

## Status

Applied.

## Goal

Make each supported `ArtworkRef` source render through screen-owned adapters
without moving GPUI image handles or cache access into shared projections.

## Read

- `docs/adr/0026-shared-entity-projection-layer.md`
- `docs/plans/post-adr-0026-follow-up-plan.md`
- `src/views.rs`
- `src/view_models/entity_detail.rs`
- `src/ui_entity.rs`
- `src/ui_feed.rs`
- `src/library.rs`
- `src/search.rs`

## Files Likely to Change

- `src/ui_feed.rs`
- `src/library.rs`
- `src/search.rs`
- `src/ui_entity.rs`, only if shell slots need a narrow adapter input
- `tests/architecture_tests.rs`, only if a new boundary needs regression
  coverage
- `docs/reviews/post-adr-0026-task-003-artwork-source-expansion-review.md`

## Do Not Touch

- Do not add GPUI imports to `src/views.rs`.
- Do not add GPUI imports to `src/view_models/entity_detail.rs`.
- Do not change database schema without a separate ADR.
- Do not silently downgrade unsupported artwork sources to unrelated fallback
  art.

## Constraints

- Shared projections continue to expose plain `ArtworkRef` values.
- GPUI image handles, image-cache lookups, and file/path resolution stay in
  screen or adapter code.
- Unsupported artwork variants must be explicit in code or review notes.
- Any cache, DB, or public artwork contract change requires a new ADR before
  implementation.

## Implementation Steps

1. Audit every `ArtworkRef` variant used by ADR 0026 projections.
2. Identify which variants render today in Discover and Library.
3. Add or plan screen-owned adapter paths for supported non-URL variants.
4. Add focused tests or architecture checks for any newly protected boundary.
5. Record unsupported variants and the reason they remain unsupported.

## Acceptance Criteria

- [x] Each supported artwork source has an adapter path in the owning screen layer.
- [x] Unsupported variants are documented explicitly.
- [x] Shared projections remain GPUI-free.
- [x] No new boundary rule was introduced, so no architecture-test update was
  needed.
- [x] No runtime changes were made, so runtime verification commands were not
  required.

## Applied Summary

- Added `docs/reviews/post-adr-0026-task-003-artwork-source-expansion-review.md`.
- Confirmed `ArtworkRef::Url` is the only constructed and supported variant.
- Documented `CacheKey`, `LocalPath`, and `EmbeddedBytesKey` as unsupported
  until their resolver/storage contracts are defined.
- No architecture-test change was needed because runtime boundaries did not
  change.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
```

Run broader `cargo test` before marking the task implemented if runtime code
changes.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0026-shared-entity-projection-layer.md`
- `docs/plans/post-adr-0026-follow-up-plan.md`
- `src/views.rs`
- `src/view_models/entity_detail.rs`
- `src/ui_entity.rs`
- `src/ui_feed.rs`
- `src/library.rs`
- `src/search.rs`

Goal:
- Add or document screen-owned rendering paths for non-URL `ArtworkRef`
  variants.

Constraints:
- Keep `src/views.rs` and `src/view_models/entity_detail.rs` GPUI-free.
- Keep image-cache and file/path resolution outside shared projections.
- Do not change schema, cache contracts, or public API contracts without a
  separate ADR.

Do not touch:
- database schema or migrations
- download, playback, playlist, or MusicBrainz behavior
- unrelated style or layout code

Acceptance criteria:
- Supported artwork variants have explicit rendering adapter paths.
- Unsupported variants are explicit and documented.
- Architecture tests cover any new boundary expectation.
- Verification commands pass.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Rendering a variant requires a cache, schema, or API contract change.
- The implementation would need GPUI in shared projection modules.
- Existing image-cache ownership is too coupled to a single screen to adapt
  safely in one task.
