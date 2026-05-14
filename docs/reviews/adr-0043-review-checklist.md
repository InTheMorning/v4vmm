# ADR 0043 Review Checklist

## Reviewed Artifacts

- `docs/adr/0043-top-toolbar-global-search.md`
- `docs/plans/adr-0043-top-toolbar-global-search-phase-plan.md`
- `docs/tasks/adr-0043-task-001-app-toolbar-frame.md`
- `docs/tasks/adr-0043-task-002-global-search-contract.md`
- `docs/tasks/adr-0043-task-003-search-workspace-results.md`
- `docs/tasks/adr-0043-task-004-guards-and-visual-readiness.md`

## Gate Status

Status: Awaiting operator visual recheck after follow-up fixes on 2026-05-14.

Readiness decision: Pending visual verification.

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

- User visual screenshots on 2026-05-13 showed narrow-toolbar clipping risk:
  scope labels could be partially visible between Settings and the Now
  Playing frame.
- Initial mitigation on 2026-05-13 made app-toolbar scope controls and the
  submit button progressively hide at named layout-token breakpoints. This was
  superseded by the 2026-05-14 HIG fix below so the primary Search action
  remains visible.
- Follow-up visual review on 2026-05-14 still showed clipping at a narrow dark
  toolbar width: scope controls and the Search submit control could remain
  visible while the global search field was no longer legible.
- Fixed on 2026-05-14: the named toolbar breakpoints were raised so optional
  scope and submit controls collapse earlier, preserving the global search
  field and Now Playing frame as the narrow-width toolbar priorities.
- Second follow-up visual review on 2026-05-14 still showed the global search
  field clipped because the Now Playing frame kept its full-width size in a
  compact toolbar.
- Fixed on 2026-05-14: Now Playing now uses a named compact-width rule below
  the toolbar breakpoint, preserving its frame while yielding enough center
  toolbar space for the search field.
- HIG drift review on 2026-05-14 found that narrow-width hiding removed the
  trailing primary Search action and made scope switching unavailable. Fixed
  on 2026-05-14: Search submit stays inline above the compact breakpoint,
  Search/scope commands collapse to the shared menu primitive below it, and
  toolbar width is computed once in `render_tab_bar` before being passed into
  search rendering.
- Operator screenshot review on 2026-05-14 showed the toolbar still clipping
  at compact width. Fixed on 2026-05-14: compact Now Playing now uses the
  `MenuRegular` width and compact global search renders as input plus overflow
  menu, avoiding the partial scope label and submit-button overlap.
- Visual proof still needs operator recheck because this execution session
  cannot inspect the running display directly.

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

- Visual proof is still pending after the 2026-05-14 narrow-toolbar fix.
  Final light/dark evidence remains assigned to Task 004.

## Missing Tests

- Task 004 still needs final light/dark visual proof at normal and narrow
  widths after the 2026-05-14 fix.

## Merge Recommendation

Pending. Do not mark ADR 0043 ready until light and dark visual proof
confirms the toolbar, global search field, Search workspace, and Now Playing
frame remain legible at normal and narrow widths.
