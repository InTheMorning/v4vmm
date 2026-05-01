# ADR 0030 Task 005: Contributor Tree Metadata

## Status

Pending.

## Goal

Render expanded contributor metadata cells as one contributor with indented role
children while preserving existing single-line summaries for callers that need
them.

## Files To Inspect

- `docs/adr/0030-discovery-library-ui-fixes.md`
- `docs/plans/discovery-library-ui-fixes.md`
- `src/metadata.rs`
- `src/library.rs`
- `src/search.rs`
- `src/ui_entity.rs`

## Files Likely To Change

- `src/metadata.rs`
- `src/library.rs`
- `src/search.rs`

## Do Not Touch

- Contributor persistence or identity ingest.
- Dedicated contributor panel behavior unless needed to reuse helpers.
- Metadata compare normalization semantics.

## Constraints

- Reuse `grouped_contributor_entries`.
- Keep `summarize_contributors` and `summarize_contributor_value`.
- Treat the new helper as display formatting only.
- Do not discard or merge source facts.

## Implementation Steps

1. Add one tree-format helper in `src/metadata.rs`.
2. Add focused tests for multi-role and multi-person values.
3. Use the helper in expanded metadata-cell display in Library and Discovery.
4. Preserve collapsed and compare-diff summaries unless directly required.

## Acceptance Criteria

- Expanded `TXXX:MusicIndex Contributors` cells show tree-shaped content.
- Existing single-line summary callers continue to work.
- Source-fact comparison and persistence are unchanged.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test metadata::tests
cargo clippy -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0030-discovery-library-ui-fixes.md`
- `docs/tasks/adr-0030-task-005-contributor-tree-metadata.md`
- `src/metadata.rs`
- `src/library.rs`
- `src/search.rs`

Goal:
- Show contributor metadata cells as person rows with indented roles.

Constraints:
- Use existing contributor grouping.
- Keep existing summary helpers.
- Keep new formatting display-only.

Do not touch:
- `src/db.rs`
- `src/identity_ingest.rs`
- migrations

Acceptance criteria:
- Expanded metadata cell tree display works in both Library and Discovery.
- Metadata storage and compare normalization are unchanged.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- focused metadata tests
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
