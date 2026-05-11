# ADR 0043 Review Checklist

## Reviewed Artifacts

- `docs/adr/0043-top-toolbar-global-search.md`
- `docs/plans/adr-0043-top-toolbar-global-search-phase-plan.md`
- `docs/tasks/adr-0043-task-001-app-toolbar-frame.md`
- `docs/tasks/adr-0043-task-002-global-search-contract.md`
- `docs/tasks/adr-0043-task-003-search-workspace-results.md`
- `docs/tasks/adr-0043-task-004-guards-and-visual-readiness.md`

## Gate Status

Status: Tasks 001-002 implemented on 2026-05-11. Tasks 003-004 pending.

Readiness decision: Pending.

## Required Checks

- [ ] Toolbar has stable leading navigation, center global search, and
  trailing Now Playing frame.
- [x] Now Playing remains app-shell-owned under `src/app/`.
- [x] Now Playing is not extracted into a single-use composite.
- [ ] One visible search field exists in the app toolbar.
- [ ] Library and Search screens do not render duplicate visible search
  input chrome.
- [ ] `cmd-f` focuses the global toolbar search.
- [x] Search scope labels, placeholder, ids, and accessibility labels
  come from view-model display contracts.
- [ ] `All` scope renders grouped Library results before MusicIndex
  results.
- [ ] `Library` scope does not call MusicIndex.
- [ ] `Index` scope does not render local Library results.
- [ ] MusicIndex type filters apply only to MusicIndex results.
- [ ] Recent feeds/discovery root remains visible when Search has no
  query.
- [x] Local Library query returns only in-library tracks.
- [x] Architecture tests cover toolbar ownership.
- [x] Architecture tests cover global-search contract and local-query
  boundary.
- [ ] Architecture tests cover duplicate-search
  prevention.
- [ ] Light-theme visual proof reviewed at normal and narrow widths.
- [ ] Dark-theme visual proof reviewed at normal and narrow widths.
- [x] `cargo fmt -- --check` green.
- [x] `cargo check` green.
- [x] `cargo test` green.
- [x] `cargo clippy -- -D warnings` green.

## Required Fixes

- Tasks 003-004 remain pending.

## Optional Improvements

- No drift in Tasks 001-002. Toolbar display strings and ids route through
  `src/view_models/app_toolbar.rs`; Now Playing remains app-shell-owned in
  `src/app/playback_bar.rs`. The local Library search query routes through
  `ApplicationQueryService`, `library_service`, and `db` without changing
  MusicIndex API behavior.

## Architectural Drift

- Visual proof is still pending because the current execution environment has
  no display server. Final light/dark evidence remains assigned to Task 004.

## Missing Tests

- Task 003 still needs visible global-search rendering, duplicate screen-search
  removal, grouped result behavior, and `cmd-f` focus on the toolbar input.

## Merge Recommendation

Pending implementation and review.
