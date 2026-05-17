# ADR 0050 — module decomposition phase plan

## Goal

Decompose three oversized modules into submodule directories without
changing behavior, per ADR 0050:

- `src/app.rs` → extract three handler clusters as submodules under
  `src/app/`.
- `src/view_models/workspace.rs` → directory `src/view_models/workspace/`
  with five submodules.
- `src/view_models/search_results.rs` → directory
  `src/view_models/search_results/` with five submodules.

## Non-Goals

- No behavior change. No public-API change. No new types, traits, or render
  paths.
- No edits to the `discover/` module (separate task).
- No text-filter helper extraction (separate task).
- No changes to architecture-test logic, only to file paths inside guards
  that pin specific files.

## Assumptions

- `git mv` preserves blame across the rename when each split lands as one
  focused commit per file.
- The existing test module (`#[cfg(test)] mod tests`) in `workspace.rs` and
  `search_results.rs` can be moved to a sibling `tests.rs` file without
  visibility regressions (some items may need to bump from `pub(super)` to
  `pub(crate)`; document those bumps).
- Re-export discipline keeps `use crate::view_models::workspace::*` and
  `use crate::view_models::search_results::*` import statements in callers
  unchanged.

## Affected Modules

- `src/app.rs` and `src/app/mod.rs`
- `src/view_models/workspace.rs` → `src/view_models/workspace/{mod,frame,chrome,nav,breadcrumb,tests}.rs`
- `src/view_models/search_results.rs` → `src/view_models/search_results/{mod,tabs,results,paged_tab,index_detail,empty_state,tests}.rs`
- `tests/architecture_tests.rs` (path-pinning guards only)

## Proposed Sequence

1. **Phase 1 — `view_models/workspace/` decomposition** (Task 002).
   Workspace VM split is largest and most cross-cutting. Doing it first
   surfaces visibility issues that may affect the other two splits.
2. **Phase 2 — `view_models/search_results/` decomposition** (Task 003).
   Independent of workspace VM internals; depends on workspace re-exports
   being stable.
3. **Phase 3 — `src/app/` decomposition** (Task 001). Depends on both VM
   layers being stable so handler extractions don't fight import drift.

Each phase: one task, one subagent, one focused commit per file move.

## Target State

### `src/view_models/workspace/`

```
mod.rs            ~600 LOC   WorkspaceLayout + impls + WorkspaceLayoutConfig + WorkspaceFrameConfig + WorkspaceModelError + re-exports
frame.rs          ~300 LOC   Frame ids, kinds, dock target, search scope/descriptor, WorkspaceFrameState
chrome.rs         ~450 LOC   Frame chrome contracts: button, menu item, shell display, filter chip strip, ContentFilter
nav.rs            ~150 LOC   FrameNavigationEntry, FrameNavigationState
breadcrumb.rs     ~200 LOC   BreadcrumbTruncation, BreadcrumbSegment, BreadcrumbDisplay
tests.rs          ~1200 LOC  The existing inline #[cfg(test)] mod tests
```

### `src/view_models/search_results/`

```
mod.rs            ~600 LOC   SearchResultsInspectorPageVm + re-exports
tabs.rs           ~80 LOC    SearchResultsTab, SearchResultOrigin, SearchResultItemId
results.rs        ~300 LOC   ArtistResultDisplay, FeedResultDisplay, TrackResultDisplay
paged_tab.rs      ~150 LOC   SearchResultsPagedTab<Row>
index_detail.rs   ~120 LOC   IndexSearchResultRows, IndexDetailKind, IndexDetailDisplay
empty_state.rs    ~50 LOC    EmptyStateDisplay
tests.rs          ~100 LOC   Existing inline tests
```

### `src/app/`

```
src/app.rs        ~1800 LOC  TopApp struct + Render + render_workspace_content + Application boot + module wiring
src/app/search_dispatch.rs   ~450 LOC   impl TopApp { submit_global_search, handle_search_result_selected, start_index_search_for_query, sync_search_results_detail_with_nav, RemoteDetailThumbnailState }
src/app/breadcrumb.rs        ~100 LOC   impl TopApp { handle_content_list_breadcrumb_select } + labeler helpers
src/app/resize.rs            ~100 LOC   impl TopApp { begin/resize/end_content_pane_resize, is_content_pane_resizing }
```

## Schema / API Implications

None. File-organization-only refactor.

## Risk Areas

- **`pub(super)` items in moved tests.** Existing inline tests reach private
  module items. After moving tests to `tests.rs`, visibility may need to
  bump to `pub(crate)`. Audit per file; document each bump.
- **`impl TopApp` splits across files.** Rust supports this freely, but
  `cargo doc` ordering and rustdoc method grouping may shift. Acceptable.
- **Re-export hygiene.** A missed re-export silently breaks callers when
  they `cargo check`. Mitigation: every task ends with a full `cargo check`
  + `cargo test` gate before considered done.
- **Arch tests that pin file paths.** Some guards (e.g., string match for
  `workspace.rs` content) need updating. The task lists the exact guard
  names to touch; do not loosen guards, just retarget paths.

## Test Strategy

After each task:

```bash
cargo fmt -- --check
cargo build
cargo test --lib
cargo test --test architecture_tests
cargo clippy -- -D warnings
```

Visual regression: none expected (no behavior change). Operator may run the
app post-Phase 3 to confirm launch and basic search round-trip.

## Rollback Strategy

Each task lands as its own commit. Revert is per-commit `git revert`. If
intermediate test failures appear inside a task, restore the original file
via `git checkout` and retry with a narrower split (start with the smallest
type moves first, e.g., for `workspace/`: extract `nav.rs` first because
it has the fewest cross-references).

## Open Questions

- Do we want a follow-up `cargo doc` smoke check, or skip? (Recommendation:
  skip; the workspace's docs aren't published.)
- Should `WorkspaceLayout` impl split by trait (`FrameOps`, `NavOps`, etc.)
  in a future ADR? Out of scope for this pass; flag if the impl block in
  `mod.rs` becomes itself oversized.
