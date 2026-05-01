# ADR 0030 Task 004: Discovery Compare Actions

## Status

Implemented - 2026-05-01.

## Goal

Suppress Library-only Compare ID3 and Compare MusicBrainz actions from
Discovery projections.

## Files To Inspect

- `docs/adr/0030-discovery-library-ui-fixes.md`
- `docs/plans/discovery-library-ui-fixes.md`
- `src/view_models/entity_detail.rs`
- `src/ui_feed.rs`
- `src/search.rs`
- `src/library.rs`

## Files Likely To Change

- `src/view_models/entity_detail.rs`
- possibly `src/search.rs` or `src/ui_feed.rs` if rendering bypasses projection

## Do Not Touch

- Compare/download implementation.
- MusicBrainz lookup behavior.
- Track metadata comparison rows.

## Constraints

- Use existing `EntitySurfaceContext`.
- Gate projection, not only button rendering.
- Preserve Library compare actions.

## Implementation Steps

1. Locate compare-action construction in `entity_detail`.
2. Gate it to `EntitySurfaceContext::Library`.
3. Add a unit test for Discover context producing no compare actions.
4. Verify Library context still projects compare actions.

## Acceptance Criteria

- [x] Discovery track views receive no compare action descriptors.
- [x] Library track views retain compare action descriptors.
- [x] The behavior is covered in view-model tests.

## Implementation Summary

- Added `EntitySurfaceContext` to `TrackMetadataActionState`.
- Passed `EntitySurfaceContext::Library` from Library track detail and
  `EntitySurfaceContext::Discover` from Discovery track detail.
- Gated Compare ID3 and MusicBrainz action descriptors so they only project in
  Library context.
- Added a unit test proving Discover context returns no compare or MusicBrainz
  metadata actions.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test view_models::entity_detail
cargo clippy -- -D warnings
```

Verified 2026-05-01.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0030-discovery-library-ui-fixes.md`
- `docs/tasks/adr-0030-task-004-discovery-compare-actions.md`
- `src/view_models/entity_detail.rs`
- `src/search.rs`
- `src/library.rs`

Goal:
- Prevent Discovery from receiving compare action descriptors.

Constraints:
- Use `EntitySurfaceContext`.
- Preserve Library behavior.
- Add a focused unit test.

Do not touch:
- `src/metadata.rs`
- `src/track_compare.rs`
- download or MusicBrainz service code

Acceptance criteria:
- Discover context has no Compare ID3 or Compare MusicBrainz actions.
- Library context still has them when applicable.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test view_models::entity_detail`
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
