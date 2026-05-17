# ADR 0050: Post-ADR-0048 module decomposition

## Status

Accepted - 2026-05-16. Implementation pending.

## Context

The ADR-0047 / 0048 / 0049 arc landed the ContentList-frame breadcrumb search
in a single push. The post-implementation review
(`docs/reviews/adr-0047-0048-0049-implementation-review.md`) flagged file-size
drift in three load-bearing modules:

| File | LOC | Reason for growth |
|---|---:|---|
| `src/app.rs` | 2,922 | `render_workspace_content` body switch, `handle_search_result_selected`, `handle_content_list_breadcrumb_select`, `sync_search_results_detail_with_nav`, `start_index_search_for_query`, `RemoteDetailThumbnailState`, fluid-resize handlers, Library/Settings tab switching all colocated in the root composition module |
| `src/view_models/workspace.rs` | 2,904 | Frame state, nav state, breadcrumb projection, chrome contracts, content-filter chip strip, search-results frame helpers, and 53 unit tests in one file |
| `src/view_models/search_results.rs` | 1,408 | `SearchResultsInspectorPageVm`, `SearchResultsPagedTab<Row>`, `IndexSearchResultRows`, `IndexDetailKind`, `IndexDetailDisplay`, `EmptyStateDisplay`, three `*ResultDisplay` row types, and tabs/origin enums in one file |

These three files are now the largest in the UI tree. ADR 0042 (layer
consolidation) targeted shrinking top-level orchestration and module
boundaries; the same intent applies here. Continued growth in this region
compounds each future ADR's diff and reviewer load.

`src/app/` is already a submodule directory
(`bootstrap`, `events`, `keyboard`, `menu`, `playback_bar`, `queue_now_playing`,
`tab_bar`). The decomposition pattern is established; this ADR extends it.

## Decision

Decompose the three files into submodule directories along behaviour seams:

### `src/app.rs` → keep root module thin, extract handler clusters

Add three submodules under `src/app/`:

- `src/app/search_dispatch.rs` — `submit_global_search`,
  `submit_global_search_with`, `handle_search_result_selected`,
  `start_index_search_for_query`, `sync_search_results_detail_with_nav`,
  `RemoteDetailThumbnailState`, plus the helpers they call.
- `src/app/breadcrumb.rs` — `handle_content_list_breadcrumb_select` and any
  breadcrumb labeler helpers.
- `src/app/resize.rs` — `begin_content_pane_resize`, `resize_content_pane`,
  `end_content_pane_resize`, `is_content_pane_resizing`, and the
  `content_pane_width` accessor pair.

Submodules host `impl TopApp` blocks. The root `src/app.rs` keeps
`TopApp` struct definition, `Render` impl, `render_workspace_content`
(the dispatcher), `Application` boot, and module wiring.

Target: `src/app.rs` shrinks to roughly 1,800-2,000 LOC after the move
(orchestration plus dispatcher only).

### `src/view_models/workspace.rs` → `src/view_models/workspace/`

| New file | Owns |
|---|---|
| `mod.rs` | `WorkspaceLayout`, `WorkspaceLayoutConfig`, `WorkspaceFrameConfig`, `WorkspaceModelError`, and re-exports |
| `frame.rs` | `WorkspaceFrameId`, `WorkspaceFrameKind`, `FrameDetachEligibility`, `FrameDockTarget`, `FrameSearchScope`, `FrameSearchDescriptor`, `WorkspaceFrameState` |
| `chrome.rs` | `FrameChromeButtonDisplay`, `FrameChromeMenuItemDisplay`, `FrameShellDisplay`, `FilterChipOption`, `FilterChipStripDisplay`, `ContentFilter` |
| `nav.rs` | `FrameNavigationEntry`, `FrameNavigationState` |
| `breadcrumb.rs` | `BreadcrumbTruncation`, `BreadcrumbSegment`, `BreadcrumbDisplay` |
| `tests.rs` | The existing inline `#[cfg(test)] mod tests` |

The `WorkspaceLayout` impl stays in `mod.rs` (it cross-cuts every submodule).
All public API stays `pub(crate)`-visible through `mod.rs` re-exports so
callers in `src/app.rs` and shells do not need import-path churn.

### `src/view_models/search_results.rs` → `src/view_models/search_results/`

| New file | Owns |
|---|---|
| `mod.rs` | `SearchResultsInspectorPageVm`, re-exports |
| `tabs.rs` | `SearchResultsTab`, `SearchResultOrigin`, `SearchResultItemId` |
| `results.rs` | `ArtistResultDisplay`, `FeedResultDisplay`, `TrackResultDisplay` |
| `paged_tab.rs` | `SearchResultsPagedTab<Row>` |
| `index_detail.rs` | `IndexSearchResultRows`, `IndexDetailKind`, `IndexDetailDisplay` |
| `empty_state.rs` | `EmptyStateDisplay` |
| `tests.rs` | Existing inline tests |

## Invariants

- File-organization-only refactor. No behavior change. No new public API. No
  changes to GPUI render paths or view-model contracts.
- All existing arch guards in `tests/architecture_tests.rs` must continue to
  pass without test edits, except for guards that hard-code file paths
  (those move to the new paths).
- Re-export discipline: every type/fn that was `pub(crate)` visible before is
  visible at the same path after, via `mod.rs` re-exports. Call sites in
  `src/app.rs`, `src/library/*`, `src/ui/shells/*` must not change their
  `use` statements.
- The `WorkspaceLayout` impl stays in one place (`workspace/mod.rs`); splitting
  it into trait impls per submodule is out of scope here.

## Non-Goals

- No new tokens, composites, primitives, or VM types.
- No change to the `discover/` parked module status (handled by a separate
  task).
- No change to text-filter helper duplication (handled by a separate task).
- No change to `frame_shell` single-caller exception (handled by an ADR-0048
  amendment).
- No commit-stacking policy change.
- No file size budget enforced as an arch test; this ADR is a one-shot
  cleanup, not a recurring rule.

## Alternatives Considered

- **Defer until next ADR.** Rejected. The same files will keep growing; the
  longer the delay, the bigger the consolidation churn.
- **Single mega-ADR also covering text-filter, discover docs, render
  collapse.** Rejected. Those concerns are independent; bundling them
  recreates the `bee1ac2` rollback-granularity problem the review flagged.
- **File-size budget as an architecture test.** Rejected for now. A budget
  produces drive-by file shuffles and rewards locality over coherence. Revisit
  if file-size drift recurs after this pass.
- **Split `WorkspaceLayout` impl by trait per submodule.** Rejected. Adds
  trait surface without a consumer; cross-cutting one large `impl` block is
  fine.

## Consequences

Positive:

- Three files drop into the 200-800 LOC sweet spot for code review.
- New ADR work on the workspace, search-results, or app layers gets its own
  submodule to grow into.
- Subagent task scope becomes natural: one module per task.

Negative / risks:

- `git blame` continuity for moved code requires `git log --follow`.
  Mitigation: land each file's split as one focused commit with `git mv` so
  rename detection works.
- Test layout: the existing `#[cfg(test)] mod tests` blocks in
  `workspace.rs` and `search_results.rs` are large. Moving them to
  `tests.rs` submodule files preserves test discoverability but may surface
  visibility issues for items that were `pub(super)`-tested. Mitigation:
  bump visibility to `pub(crate)` only where the test demands it; document
  the bumps in the task notes.
- Brief reviewer disruption while CODEOWNERS / lint allowlists / arch tests
  catch up to new paths.

## References

- ADR 0042 - Layer consolidation
- ADR 0046 - Workspace frame architecture
- ADR 0047 - Library and search unification
- ADR 0048 - ContentList frame breadcrumb search
- ADR 0049 - Inspector source ownership
- `docs/reviews/adr-0047-0048-0049-implementation-review.md`
- `docs/plans/adr-0050-module-decomposition-phase-plan.md`
