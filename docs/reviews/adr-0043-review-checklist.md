# ADR 0043 Review Checklist

## Reviewed Artifacts

- `docs/adr/0043-top-toolbar-global-search.md`
- `docs/plans/adr-0043-top-toolbar-global-search-phase-plan.md`
- `docs/tasks/adr-0043-task-001-app-toolbar-frame.md`
- `docs/tasks/adr-0043-task-002-global-search-contract.md`
- `docs/tasks/adr-0043-task-003-search-workspace-results.md`
- `docs/tasks/adr-0043-task-004-guards-and-visual-readiness.md`

## Gate Status

Status: Not started.

Readiness decision: Pending.

## Required Checks

- [ ] Toolbar has stable leading navigation, center global search, and
  trailing Now Playing frame.
- [ ] Now Playing remains app-shell-owned under `src/app/`.
- [ ] Now Playing is not extracted into a single-use composite.
- [ ] One visible search field exists in the app toolbar.
- [ ] Library and Search screens do not render duplicate visible search
  input chrome.
- [ ] `cmd-f` focuses the global toolbar search.
- [ ] Search scope labels, placeholder, ids, and accessibility labels
  come from view-model display contracts.
- [ ] `All` scope renders grouped Library results before MusicIndex
  results.
- [ ] `Library` scope does not call MusicIndex.
- [ ] `Index` scope does not render local Library results.
- [ ] MusicIndex type filters apply only to MusicIndex results.
- [ ] Recent feeds/discovery root remains visible when Search has no
  query.
- [ ] Local Library query returns only in-library tracks.
- [ ] Architecture tests cover toolbar ownership and duplicate-search
  prevention.
- [ ] Light-theme visual proof reviewed at normal and narrow widths.
- [ ] Dark-theme visual proof reviewed at normal and narrow widths.
- [ ] `cargo fmt -- --check` green.
- [ ] `cargo check` green.
- [ ] `cargo test` green.
- [ ] `cargo clippy -- -D warnings` green.

## Required Fixes

- None recorded yet.

## Optional Improvements

- None recorded yet.

## Architectural Drift

- None recorded yet.

## Missing Tests

- None recorded yet.

## Merge Recommendation

Pending implementation and review.
