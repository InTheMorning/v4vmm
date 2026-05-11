# ADR 0043 Review Checklist

## Reviewed Artifacts

- `docs/adr/0043-top-toolbar-global-search.md`
- `docs/plans/adr-0043-top-toolbar-global-search-phase-plan.md`
- `docs/tasks/adr-0043-task-001-app-toolbar-frame.md`
- `docs/tasks/adr-0043-task-002-global-search-contract.md`
- `docs/tasks/adr-0043-task-003-search-workspace-results.md`
- `docs/tasks/adr-0043-task-004-guards-and-visual-readiness.md`

## Gate Status

Status: Tasks 001-003 implemented on 2026-05-11. Task 004 pending.

Readiness decision: Pending.

## Required Checks

- [x] Toolbar has stable leading navigation, center global search, and
  trailing Now Playing frame.
- [x] Now Playing remains app-shell-owned under `src/app/`.
- [x] Now Playing is not extracted into a single-use composite.
- [x] One visible search field exists in the app toolbar.
- [x] Library and Search screens do not render duplicate visible search
  input chrome.
- [x] `cmd-f` focuses the global toolbar search.
- [x] Search scope labels, placeholder, ids, and accessibility labels
  come from view-model display contracts.
- [x] `All` scope renders grouped Library results before MusicIndex
  results.
- [x] `Library` scope does not call MusicIndex.
- [x] `Index` scope does not render local Library results.
- [x] MusicIndex type filters apply only to MusicIndex results.
- [x] Recent feeds/discovery root remains visible when Search has no
  query.
- [x] Local Library query returns only in-library tracks.
- [x] Architecture tests cover toolbar ownership.
- [x] Architecture tests cover global-search contract and local-query
  boundary.
- [x] Architecture tests cover duplicate-search
  prevention.
- [x] Toolbar search field renders with a search icon and a clear affordance.
- [x] Toolbar search button label comes from the toolbar view-model contract.
- [x] Index-only Search workspace filters are hidden for Library scope.
- [x] App-shell tab naming uses Search instead of Discover for the global
  search workspace.
- [ ] Light-theme visual proof reviewed at normal and narrow widths.
- [ ] Dark-theme visual proof reviewed at normal and narrow widths.
- [x] `cargo fmt -- --check` green.
- [x] `cargo check` green.
- [x] `cargo test` green.
- [x] `cargo clippy -- -D warnings` green.

## Required Fixes

- Task 004 visual readiness remains pending.
- Visual proof is blocked in this session: `DISPLAY=:0 wmctrl -l` fails with
  `Authorization required, but no authorization protocol specified` and
  `Cannot open display`, including after an escalated retry.

## Optional Improvements

- No drift in Tasks 001-003. Toolbar display strings and ids route through
  `src/view_models/app_toolbar.rs`; Now Playing remains app-shell-owned in
  `src/app/playback_bar.rs`. The local Library search query routes through
  `ApplicationQueryService`, `library_service`, and `db` without changing
  MusicIndex API behavior. Grouped Search results are source-aware in
  `src/view_models/search.rs`, and local Library rows open local track detail
  instead of reusing MusicIndex ids. Recent feeds remain the empty-query Search
  root so the toolbar query stays the single source of search state. The
  app-shell tab/key/focus naming now uses Search; legacy `discover` shell
  module names remain as existing surface structure.

## Architectural Drift

- Visual proof is still pending because the current execution environment has
  no display server. Final light/dark evidence remains assigned to Task 004.

## Missing Tests

- Task 004 still needs final light/dark visual proof at normal and narrow
  widths.

## Merge Recommendation

Pending implementation and review.
