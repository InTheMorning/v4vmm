# ADR 0047: Library and Search Unification

## Status

Proposed - 2026-05-14. Promotes
`docs/plans/library-search-unification-plan.md`, which is retained as
the pre-ADR concept artifact.

## Context

ADR 0046 introduces a workspace-frame architecture with typed
`SourceList`, `ContentList`, `Detail`, and `QueueNowPlaying` frames.
That architecture removes inspector-owned "back" buttons and gives
navigation, history, and frame chrome a single home.

The app still treats Library and Search as separate top-level surfaces:
`src/library.rs` and `src/search.rs` render overlapping artist, feed,
and track entities through parallel paths, while toolbar search opens a
Search tab whose local left pane owns result filters and selection. This
has caused repeated regressions: Recent Feeds becoming unreachable after
search, search filters drifting from the visible result sections,
inspector navigation being tacked onto entity detail views, and library
status not being consistently expressed across search-origin and
library-origin rows.

The next workspace step needs one content model and one inspector model.
Search should be a global command that opens searchable content in
frames, not a separate screen with duplicated chrome.

## Decision

Unify Library and Search around ADR 0046 frames.

1. `SourceList` remains the stable navigation frame for library tree,
   playlists, saved searches, and settings entries.
2. `ContentList` becomes the single content surface for library rows,
   playlist rows, saved searches, and other source selections.
3. Global toolbar search submits into a `Detail` frame as a
   `SearchResultsInspector`, rather than switching to a Search tab.
4. Artist, feed, and track inspectors are shared across library and
   search origins.
5. Filter controls become per-frame content filters (`All`,
   `Library`, `Index`) rendered by `frame_shell` chrome, not global
   toolbar scope chips.
6. Breadcrumbs are frame chrome, projected from frame navigation state,
   and are rendered by `frame_shell`; entity inspectors do not own
   return controls.
7. Track inspector advanced panels are explicit disclosure state:
   Compare ID3 and MusicBrainz are disabled for non-downloaded tracks,
   and library-extra fields render only when the corresponding panel is
   expanded for downloaded tracks.
8. Long descriptions use disclosure state with an auto-collapse
   threshold, matching the existing Contributors and Value Routes
   pattern.
9. Saved searches appear as source-list entries and open the same
   `SearchResultsInspector` as ad-hoc toolbar searches.
10. After the unified workspace path is verified, `src/search.rs`,
    `GlobalSearchScope`, and the `WORKSPACE_RENDER_ENABLED` toggle are
    retired.

## Invariants

- Search results must not disturb the current library/source-list
  selection.
- Recent Feeds must remain reachable after any search attempt.
- Content filters apply to the visible frame that owns them; there is
  no global content-filter store.
- Search, library, and playlist result rows expose library membership
  state consistently.
- Frame breadcrumbs and filter chips extend `FrameShellDisplay`; no
  sibling frame-chrome composite is introduced.
- `SearchResultsInspector` tab, filter, and breadcrumb state are
  GPUI-free view-model state.
- Disabled controls have no click handler, render in a dimmed state,
  and expose explanatory tooltip or accessibility text.
- No raw transport errors are shown as normal inspector content for
  optional metadata panels.
- Architecture guards land with each phase that changes a structural
  invariant.

## Phasing

Implementation follows
`docs/plans/adr-0047-library-search-unification-phase-plan.md`.

- Phase A ratifies this ADR and the phase plan.
- Phase B adds GPUI-free view-model contracts.
- Phase C rewires inspector advanced panels and description disclosure.
- Phase D moves content filters into per-frame chrome.
- Phase E adds search-results inspector routing and breadcrumbs.
- Phase F retires the standalone Search screen module.
- Phase G completes guards and light/dark visual proof.

Phase E depends on ADR 0046 Phase 5 Task 012, because frame navigation
state needs to be owned by the workspace VM and keyed by frame id before
breadcrumbs can be implemented without a second refactor.

## Apple HIG Alignment

- Search remains a global toolbar command, while filtering moves to the
  content frame where its effect is local and visible.
- Breadcrumbs follow the path-bar pattern with middle truncation at
  narrow widths.
- Per-frame filter chips use a single-select segmented/pill control and
  collapse to a pull-down at narrow widths.
- Disabled controls communicate state through dimming and explanatory
  help, rather than silently doing nothing.
- Disclosure controls use the existing chevron convention and predictable
  placement.
- Empty filter states render explicit content-unavailable messaging and
  a clear recovery action.

## Alternatives Considered

- **Keep the Search tab.** Rejected. It preserves duplicated screen
  chrome and keeps search-result navigation outside the workspace frame
  model.
- **Move all search results into `ContentList` only.** Rejected. The
  user needs to keep source-list and content context visible while
  inspecting search results.
- **Keep global All/Library/Index scope chips.** Rejected. Global scope
  chips hide which frame they affect once multiple content surfaces
  exist.
- **Introduce a second breadcrumb composite.** Rejected. ADR 0046 makes
  `frame_shell` the owner of frame chrome; breadcrumbs and filter chips
  extend that display contract.
- **Retire `src/search.rs` first.** Rejected. Code deletion comes after
  shared VMs and inspector shells are in place.

## Consequences

Positive:

- Library and search no longer drift through separate rendering paths.
- Search can coexist with library browsing without destroying context.
- Frame chrome owns breadcrumbs, filters, and navigation consistently.
- Advanced inspector panels become explicit user intent instead of
  always-visible clutter.
- Saved searches naturally fit the workspace model.

Negative / risks:

- The change spans VM contracts, frame chrome, inspectors, and routing.
  Mitigation: implement through bounded task packets with guards.
- Frame navigation ownership moves from `LibraryApp` to the workspace
  VM. Mitigation: sequence this behind ADR 0046 Phase 5 Task 012.
- Removing `src/search.rs` can expose hidden coupling. Mitigation:
  retire the module only in Phase F after shared inspectors are active.
- Filter chips may become ambiguous in narrow frames. Mitigation:
  collapse to a pull-down and capture visual proof in Phase G.

## References

- ADR 0033 - HIG UI architecture governance
- ADR 0034 - scale-aware UI tokens and controls
- ADR 0038 - presentation contract enforcement
- ADR 0041 - windowed paged view models
- ADR 0043 - top toolbar global search
- ADR 0046 - workspace frame architecture
- `docs/plans/library-search-unification-plan.md`
- `docs/plans/adr-0047-library-search-unification-phase-plan.md`
