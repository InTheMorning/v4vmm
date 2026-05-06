# ADR 0030 Review Checklist

## Reviewed Artifacts

- `docs/adr/0030-discovery-library-ui-fixes.md`
- `docs/plans/discovery-library-ui-fixes.md`
- `docs/tasks/adr-0030-task-001-backslash-search.md`
- `docs/tasks/adr-0030-task-002-recents-labels.md`
- `docs/tasks/adr-0030-task-003-feed-header-parity.md`
- `docs/tasks/adr-0030-task-004-discovery-compare-actions.md`
- `docs/tasks/adr-0030-task-005-contributor-tree-metadata.md`
- `docs/tasks/adr-0030-task-006-scroll-containers.md`

## Required Checks Per Task

- Pass/fail stated clearly.
- Diff matches the task packet.
- No architecture drift from ADR 0023, ADR 0025, ADR 0026, ADR 0027, or ADR
  0030.
- No hidden service, DB, or GPUI coupling added to view models.
- No source-fact loss or metadata inference.
- No unrelated cleanup.
- Focused tests added or updated.
- Required command gates run.

## Task-Specific Review Points

- Task 001: Query normalization is centralized in `src/api.rs`; no `%5C`
  reaches query URLs.
- Task 002: Recent-feed labels come from deserialized source fields or existing
  fallbacks, not invented values.
- Task 003: Feed header data is separate from action rows; no parallel
  `FeedHeader` composite.
- Task 004: Discovery receives no compare action descriptors; Library still
  does.
- Task 005: Contributor tree formatting is display-only and preserves existing
  summary helpers.
- Task 006: Scroll containers use bounded flex sizing and do not create nested
  vertical scroll views.

## Merge Recommendation Template

Pass/fail:

Required fixes:

Optional improvements:

Architectural drift:

Missing tests:

Merge recommendation:
