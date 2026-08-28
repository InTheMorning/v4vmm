# ADR 0048: ContentList Frame Breadcrumb Search

## Status

Implemented - 2026-05-16. Verified by the ADR 0048 architecture guards in
`tests/architecture_tests.rs`. The earlier "commits TBD" placeholder is
resolved: the guards, not a commit list, are the durable evidence.

## Context

ADR 0047 routed toolbar search into a `Detail` workspace frame that rendered
the `SearchResultsInspector` alongside Library and Queue. The operator UX
review found:

- Three sibling panes (Library | Queue | Detail) crowded narrow screens.
- Queue ended up sandwiched, contradicting the "trailing" intent of ADR 0046.
- The Detail-as-search-frame contract created a sibling pane the user didn't
  expect when invoking toolbar Search.
- A secondary "+ open in new frame" affordance (active-frame-search-dispatch
  plan) implied parallel result surfaces, conflicting with macOS HIG guidance
  against opening new windows as default behavior.

The active-frame-search-dispatch plan (`docs/plans/active-frame-search-dispatch-plan.md`)
attempted to dispatch toolbar text into the focused frame's VM. That plan is
now superseded.

## Decision

Toolbar Search opens a `FrameNavigationEntry::Search(query)` in the
ContentList frame's navigation stack. If ContentList is already showing search
results or a detail reached from search results, the active search entry is
replaced with the new query and its descendants are discarded so successive
searches do not stack query crumbs. If ContentList is showing non-search
content, search pushes onto the stack so back returns to that previous content.
The ContentList body switches on its current nav top:

- `Search(_)` → `SearchResultsInspector`
- `Settings` → Settings
- `TrackDetail(id)` / `AlbumDetail(id)` / `ArtistDetail(name)` / `PlaylistDetail(id)`
  → existing entity inspector rendered inside the Library shell
- `SourceList` / default → Library tree

Result row clicks push the corresponding entity-detail nav entry onto the
stack.

The top toolbar exposes only Library and Settings as app-section tabs. Search
is a toolbar command, not a tab or screen mount. Library tab activation resets
the ContentList nav stack to `SourceList`; Settings tab activation resets it to
`Settings`.

Frame chrome breadcrumb visible whenever the ContentList stack has depth.
Clicking a segment pops the stack via `WorkspaceLayout::pop_nav_until` and
the ContentList body re-renders.

The Detail workspace frame is retained as a `WorkspaceFrameKind` variant but
no longer auto-inserted into the visible layout. Future workflows (e.g.,
side-by-side compare) can opt-in to a second pane explicitly.

A `SplitPane` separates ContentList and QueueNowPlaying with drag-to-resize,
in-memory only for v1.

Search results are seeded synchronously from local Library rows, then the app
starts background MusicIndex feed and track searches. Artist rows are derived
from those Index results so the default Artists tab can show non-library Index
matches. `SearchResultsInspectorPageVm` owns the Index loading, result, and
error states; async completions merge only when ContentList still points at the
same `Search(query)`.

The Discover/Search module remains compiled but unmounted. Retiring it is a
follow-up cleanup after the operator verifies the ContentList search path.

## Invariants

- ContentList nav top is the source of truth for body content. `LibraryApp.detail`
  and `TopApp.search_results_detail` are derived state synced from nav.
- `AppTab` and `WorkspaceScreenMount` have no Search variant.
- Settings is represented by `FrameNavigationEntry::Settings`, not by a stale
  toolbar tab title.
- Toolbar Search submit is workspace-VM-owned: it pushes from non-search
  content and replaces the active search flow from an existing search.
- Search-result row activation only pushes; it never directly mutates
  `LibraryApp.detail` without also pushing nav.
- No "secondary" or "new frame" search action. Toolbar Search is the single
  entry point.
- `SearchResultsInspector` state (`search_results_detail`) is rebuilt when nav
  returns to Search and cleared when nav leaves Search.
- Index results are VM-owned and race-guarded against stale async completions.
- The Detail workspace frame may be in the workspace model but never appears
  in the visible layout for toolbar-search flows.

## Apple HIG Alignment

- Search remains a global toolbar command, but its result surface lives
  in-place rather than as a new pane.
- Top-level navigation contains only stable app sections. Search is not a dead
  tab whose title can drift away from the body.
- Settings switches to a Settings body through the same navigation source of
  truth, avoiding a title-only transition.
- Breadcrumb chrome follows path-bar pattern.
- Single search action; no "open in new window" affordance contradicting HIG
  default-behavior guidance.

## Alternatives Considered

- **Keep the Detail-frame search inspector (ADR 0047 model).** Rejected per
  operator UX review.
- **Active-frame text-filter dispatch (active-frame-search-dispatch plan).**
  Rejected. Implementation showed the contract was unclear: primary Search
  and secondary "+" routed to different surfaces, and a "filter the focused
  frame's rows" model duplicates the existing per-frame filter chip strip.
- **A new top-level "Search" workspace frame kind.** Rejected. Adds a fourth
  frame concept and doesn't help narrow-screen UX.

## Consequences

Positive:

- Single visible content pane for search; Queue stays in its trailing position.
- Breadcrumb chrome makes back-navigation discoverable and clears the path bar's
  role.
- Library / Settings tab switching and frame back navigation now mutate the
  same ContentList nav state that renders the body.
- Remote Index results can load into the inspector without blocking local
  Library results.
- Detail frame kind is preserved for future side-by-side-compare workflows
  without churn.

Negative / risks:

- ContentList must carry richer nav state than under ADR 0047. Mitigation:
  `WorkspaceLayout::push_nav` / `pop_nav` / `pop_nav_until` already in place.
- `LibraryApp.detail` and `search_results_detail` are derived state with sync
  hooks. Risk of drift if a future feature mutates `LibraryApp.detail`
  directly without also pushing nav. Mitigation: architecture guard pins
  `handle_search_result_selected` and `handle_content_list_breadcrumb_select`
  as the only entry points.
- The Discover module is temporarily dead UI. Mitigation: leave it compiled for
  now and remove it in a dedicated cleanup pass after visual verification.

## Composite call-site exception

The `frame_shell` composite (`src/ui/composites/frame_shell.rs`) has one
call site (`WorkspaceShell::render`). ADR 0042 requires composites to have
at least two distinct call sites or collapse into the consuming shell.

`frame_shell` is exempted from that rule because it is the canonical home
of frame chrome under ADR 0033. The composite centralizes title, back,
forward, breadcrumb, filter chip strip, close, and menu so that hand-rolled
floating chrome is forbidden by the matching architecture guard. Even at
one caller, the module earns its existence by enforcing that contract.

If a future ADR introduces a second frame-chrome consumer (a detachable
window, a per-pane mini frame, or a non-workspace shell), that consumer
must also go through `frame_shell` rather than reinvent the chrome.

A module-level doc comment in `src/ui/composites/frame_shell.rs` records
this exception with a pointer back here.

## References

- ADR 0046 — workspace frame architecture
- ADR 0047 — library/search unification
- ADR 0050 — post-ADR-0048 module decomposition
- `docs/plans/search-in-library-frame-plan.md` — implementation plan
  (Implemented - 2026-05-16)
- `docs/plans/active-frame-search-dispatch-plan.md` — Superseded
- `docs/reviews/active-frame-search-dispatch-review-checklist.md` — Superseded
- `docs/reviews/adr-0047-0048-0049-implementation-review.md` — post-merge review
