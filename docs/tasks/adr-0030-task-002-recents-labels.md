# ADR 0030 Task 002: Discovery Recents Labels

## Status

Pending.

## Goal

Ensure Discovery recent-feed tiles display stable title and artist or publisher
labels from current `/v1/feeds/recent` responses.

## Files To Inspect

- `docs/adr/0030-discovery-library-ui-fixes.md`
- `docs/plans/discovery-library-ui-fixes.md`
- `src/api.rs`
- `src/views.rs`
- `src/search.rs`
- `src/view_models/search.rs`

## Files Likely To Change

- `src/api.rs`
- `src/views.rs`
- `src/search.rs`
- `src/view_models/search.rs`

## Do Not Touch

- Database schema or migrations.
- Download, playback, playlist, MusicBrainz, or metadata write behavior.
- Header layout work from Task 003.

## Constraints

- Confirm actual deserialization before changing render fallbacks.
- Prefer adding missing `serde` aliases over screen-local inference.
- Reuse existing title and artist fallback helpers.

## Implementation Steps

1. Locate the `Feed` type used by `RecentFeedsResponse`.
2. Add a focused deserialization test with minimized recent-feed JSON.
3. Add missing aliases or adjust the established fallback chain.
4. Verify recents tile rendering still uses the shared fallback helper.

## Acceptance Criteria

- Recent-feed test data populates title and artist or publisher fields.
- The recents tile path displays labels when those source fields are present.
- No new metadata inference is introduced.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test api::tests
cargo test view_models::search
cargo clippy -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0030-discovery-library-ui-fixes.md`
- `docs/tasks/adr-0030-task-002-recents-labels.md`
- `src/api.rs`
- `src/views.rs`
- `src/search.rs`
- `src/view_models/search.rs`

Goal:
- Restore visible title and artist/publisher labels for Discovery recent-feed
  tiles.

Constraints:
- Confirm deserialization first.
- Prefer aliases or existing fallback helpers.
- Do not alter unrelated Discovery layout.

Do not touch:
- `src/db.rs`
- `src/metadata.rs`
- `src/ui/composites/detail_header.rs`

Acceptance criteria:
- A recent-feed response fixture hydrates the fields used by recents tiles.
- Recents labels render from source fields without invented metadata.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- focused tests for the changed deserialization/view-model path
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
