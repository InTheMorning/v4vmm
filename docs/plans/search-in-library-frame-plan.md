# Render search inside Library frame, breadcrumb-navigated, resizable

Status: Implemented - 2026-05-16. Amended by ADR 0048 completion work on
2026-05-16.

Implementation amendments:

- The toolbar app tabs are now Library / Settings only; Search is a toolbar
  command, not a tab or `WorkspaceScreenMount`.
- `FrameNavigationEntry::Settings` mounts Settings in the ContentList body.
- Library tab activation resets ContentList nav to `SourceList`; Settings tab
  activation resets it to `Settings`.
- ContentList frame back is wired through `FrameShellSlots::on_back` and shares
  the same derived-state sync path as breadcrumb selection.
- Search results seed local Library rows synchronously and load remote
  MusicIndex Index rows asynchronously into `SearchResultsInspectorPageVm`.
- `WorkspaceLayout::open_search_results_frame` was retired; Detail remains a
  frame kind but no longer has a toolbar-search helper.

## Context

Today the UI is unusable for search:

- Toolbar submit opens a *separate Detail pane* alongside the Library
  frame. The user sees Library on the left, Queue sandwiched in the
  middle (rendering bug), Detail far right. That's three sibling panes
  competing for narrow widths.
- Clicking a search-result row does nothing — the row's `on_click` is
  never wired (`SearchResultsInspectorSlots::on_result_select` has a
  `#[expect(dead_code, reason = "Phase F wires result drill-down")]`
  marker and no caller passes a handler at
  `src/app.rs:914`).
- All workspace frames are non-resizable. `WorkspaceShell::render`
  (`src/ui/shells/workspace.rs:177-231`) lays frames out as plain flex
  children with no `SplitPane` wrapper. The composite at
  `src/ui/composites/split_pane.rs` exists and is proven (Library
  internal sidebar uses it).
- `submit_global_search` always routes to `SubmitModifier::NewFrame`
  (`src/app.rs:373-375`), which calls
  `WorkspaceLayout::open_search_results_frame` and creates/reuses the
  Detail frame. That contract — Detail-as-search-pane — is what the
  user is rejecting.

The user's desired model:

- Search results render **inside the Library (ContentList) frame**, on
  top of (replacing) the library tree.
- The Library frame's chrome shows a breadcrumb so the user navigates
  back to the library tree, or forward into a result's entity detail.
- Clicking a result row pushes another nav entry (TrackDetail,
  FeedDetail, ArtistDetail, etc.) and the body switches to the entity
  inspector inside the same frame.
- Queue stays on the right. No standalone Detail sibling pane for
  search.
- Drag-handles between adjacent frames for resize.

This plan supersedes the active-frame-search-dispatch model. The
per-frame text-filter VM contracts from Phase 1 of that plan remain
useful for a future inline find/filter affordance and are not removed.

## Goal

- Toolbar Search submit opens `FrameNavigationEntry::Search(query)`
  in the ContentList (Library) frame's nav stack. It pushes from
  non-search content and replaces the active search flow from an existing
  search, including a detail reached from search results. No Detail frame is
  spawned for search.
- ContentList frame body renders the
  `SearchResultsInspectorPageVm` shell when its nav top is
  `Search(_)`; `Settings` renders Settings; `SourceList` and entity
  detail entries render the Library-backed surface.
- Frame-shell breadcrumb is visible on the ContentList frame whenever
  its nav stack has more than the root entry. Clicking a breadcrumb
  segment pops the stack back to that segment. The ContentList back
  chevron pops the same stack by one entry.
- Clicking a search-result row pushes the corresponding
  entity-detail nav entry onto the ContentList stack and the body
  switches to the appropriate entity inspector inside the same frame.
- Adjacent workspace frames (Library and Queue, in v1) are separated
  by a draggable resize handle via `SplitPane`.
- Remote Index feed and track rows load asynchronously after local Library rows.
  Artist rows are derived from those Index rows, and loading/error states stay
  VM-owned.

## Non-goals

- The standalone Detail frame is not deleted; it stays available for
  future workflows (e.g., side-by-side compare). It is just no longer
  the destination for toolbar search submit.
- Saved-search persistence (no DB table, no save UI).
- Discover/Search module deletion. It remains compiled but unmounted until a
  follow-up cleanup pass.
- Settings-field search.
- Per-pane width persistence in `config.toml`. v1 ships in-memory
  resize only; persistence is a follow-up.

## Current state

- Toolbar search now uses `WorkspaceLayout::open_search_results_in_content_list`
  and opens `FrameNavigationEntry::Search(query)` in ContentList.
- `WorkspaceLayout::push_nav` / `pop_nav` / `reset_nav` /
  `frame_nav_mut` exist (`src/view_models/workspace.rs:805-841`) and
  already support nav-stack mutation on any frame.
- `FrameShellDisplay::with_breadcrumb` exists; `frame_shell` already
  renders breadcrumb chrome when the display carries one
  (`src/ui/composites/frame_shell.rs` post Phase E Task 013).
- `should_render_breadcrumb` in `src/ui/shells/workspace.rs:246-249`
  currently fires only for `WorkspaceFrameKind::Detail` with a Search
  nav entry. Extend it to fire for ContentList whenever the nav stack
  has depth.
- `SearchResultsInspectorPageVm::from_local_library_tracks(query, rows)`
  exists (`src/view_models/search_results.rs:386`) and is already used
  by `TopApp::search_results_detail_for_query` (`src/app.rs:409-419`).
  Reuse verbatim.
- `SearchResultsInspectorSlots::on_result_select` exists
  (`src/ui/shells/search_results_inspector.rs:65-71`). Drop its
  `#[expect(dead_code)]` marker once the new call site wires the
  handler at `src/app.rs:914`.
- `render_result_row` (`src/ui/shells/search_results_inspector.rs:319-323`)
  already attaches the `on_click` when the handler is `Some(_)`. No
  shell-side changes needed for the click path.
- `SplitPane` (`src/ui/composites/split_pane.rs`) is the existing
  resize composite, ready to wrap adjacent frame children.

## Target state

### Library frame breadcrumb-driven navigation

`WorkspaceFrameKind::ContentList` owns the visible body. Default root nav entry
is `SourceList`. Toolbar Search submit pushes
`FrameNavigationEntry::Search(query)` from non-search content and replaces the
active search flow when search is already current or when the user is viewing a
detail reached from search. Settings tab activation resets the stack to
`FrameNavigationEntry::Settings`. Clicking a result row pushes a
`TrackDetail(id)` / `AlbumDetail(id)` / `ArtistDetail(name)` / etc. Clicking a
breadcrumb segment calls `pop_nav_until` until that segment is the top;
clicking the back chevron calls `pop_nav`.

ContentList body render branches on the nav top:

| Nav top | Body |
|---|---|
| `SourceList` (or default) | Library-backed surface |
| `Settings` | Settings surface |
| `Search(query)` | `render_search_results_inspector(vm, slots, cx)` with local rows from `search_results_detail_for_query(query)` and async Index rows merged into the VM |
| `TrackDetail(id)` / `FeedDetail(id)` / `AlbumDetail(id)` / `ArtistDetail(name)` / `PlaylistDetail(id)` | the existing entity inspector renderer for that kind (today these live inside `LibraryApp.detail` — reuse) |

### Breadcrumb chrome

Extend `should_render_breadcrumb` to fire for ContentList when the
frame's nav stack depth is > 1. Use the existing
`BreadcrumbDisplay::project` path that's already in
`src/ui/shells/workspace.rs:196-203`. Breadcrumb segment labels come
from `FrameNavigationEntry::display_label`.

### Result row drill-down

`SearchResultsInspectorSlots::on_result_select(handler)` is wired in
`src/app.rs::render_workspace_content` at the slot construction site
(currently `src/app.rs:914`). The handler receives `(tab, result_id)`
and dispatches:

- `Artists` → push `FrameNavigationEntry::ArtistDetail(name)` onto the
  ContentList frame's nav stack
- `Feeds` → push `FrameNavigationEntry::FeedDetail(id)` (or
  `AlbumDetail(id)` per existing nav variants)
- `Tracks` → push `FrameNavigationEntry::TrackDetail(id)`

Use existing `LibraryApp` selection methods (e.g.,
`LibraryApp::select_track`, `select_album`) to hydrate detail state
inside the library entity, then mutate the workspace nav stack to
reflect the new top.

### Frame ordering + resize

The workspace renders the visible layout as: ContentList ◢ Queue.
Drop the Detail frame from the *visible* layout at
`app.rs::visible_workspace_layout` for the search/library mount — it's
no longer needed because search results live in ContentList. (Detail
remains in the model for future workflows but does not render unless
explicitly opened by a future feature.)

Replace `WorkspaceShell::render`'s flat flex row
(`src/ui/shells/workspace.rs:166-175,222-230`) with a `SplitPane`
that owns the ContentList ↔ Queue divider. Drag-handle resize is
persisted in `TopApp` state (in-memory `Pixels` width for the leading
pane). No `config.toml` persistence in v1.

If the user later restores Detail (e.g., side-by-side compare), nest
a second `SplitPane` inside or upgrade to a multi-pane shell. Out of
scope for v1.

## Required wiring

### `src/view_models/workspace.rs`

Add:

```rust
/// Open or focus the ContentList frame for a submitted search query.
pub(crate) fn open_search_results_in_content_list(
    &mut self,
    query: impl Into<String>,
) -> Result<WorkspaceFrameId, WorkspaceModelError>;
```

Implementation: locate the ContentList frame (must exist in default
layout; if missing, error). Push `FrameNavigationEntry::Search(query.into())`
when no search entry is active; replace the nearest active `Search(_)` entry
and discard its descendants otherwise. Focus the ContentList frame. This
preserves the path back to pre-search content without stacking repeated query
crumbs.

Do not keep a toolbar-search Detail-frame helper. Future side-by-side workflows
must introduce their own explicit command/ADR instead of reusing toolbar
Search.

### `src/app.rs`

- `submit_global_search` (line 373) routes to
  `open_search_results_in_content_list(query)` instead of
  `open_search_results`. Remove or repurpose `open_search_results`.
- `search_results_detail` field stays (rename optional) — it now holds
  the `SearchResultsInspectorPageVm` for whichever ContentList frame
  is currently displaying search results. It is rebuilt when nav returns to
  `Search(_)` and cleared when nav leaves Search.
- `render_workspace_content` (line ~870) body switching: query the
  ContentList frame's nav top via
  `self.workspace_layout.frame_nav(content_frame_id)`; pick the body
  element accordingly. Existing Library / Settings render paths stay.
- `SearchResultsInspectorSlots` constructor at line ~914 gains
  `.on_result_select(move |tab, result_id, _window, cx| { entity.update(cx, |this, cx| this.handle_search_result_selected(tab, result_id, cx)); })`.
- New helper `TopApp::handle_search_result_selected(tab, result_id, cx)`:
  parse `result_id` per tab, push the appropriate
  `FrameNavigationEntry::*Detail` onto the ContentList frame's nav
  stack, hydrate Library detail state via existing selection methods,
  `cx.notify()`.
- Wire frame-shell breadcrumb selection: when the user clicks a
  breadcrumb segment, call `WorkspaceLayout::pop_nav_until(content_frame_id, target_entry)`
  (new helper, see workspace VM addition above). The
  `FrameShellSlots::on_breadcrumb_select` path already exists.
- Wire `FrameShellSlots::on_back` for ContentList to pop one nav entry and
  share the breadcrumb sync path.
- Start async Index feed and track searches after local rows are displayed.
  Derive Index artist rows from those results so the default Artists tab can
  show remote-only matches. Completion must check ContentList still points at
  the same `Search(query)` before merging rows into
  `SearchResultsInspectorPageVm`.

### `src/ui/shells/workspace.rs`

- Extend `should_render_breadcrumb`
  (`src/ui/shells/workspace.rs:246-249`) to also return true when
  `kind == ContentList && nav.has_history()` (add a
  `FrameNavigationState::has_history` accessor on the VM if missing).
- Replace the flat `flex_row` body
  (`src/ui/shells/workspace.rs:166-175,222-230`) with a `SplitPane`
  shell that owns the divider between the ContentList pane and the
  Queue pane. Initial leading width: use an existing layout token
  (e.g. `layout::CONTENT_FRAME_WIDTH` — verify it exists or pick the
  closest analogue). Mouse-move / mouse-up handlers mutate a
  `TopApp::content_pane_width: Pixels` field via
  `entity.update(cx, |this, _| this.set_content_pane_width(width))`.

### `src/ui/shells/search_results_inspector.rs`

- Drop the `#[cfg_attr(not(test), expect(dead_code, reason = "Phase F wires result drill-down"))]`
  on `on_result_select` (`src/ui/shells/search_results_inspector.rs:58-71`).
  The slot is now wired from `app.rs`.

### `src/view_models/library.rs` and entity-detail call sites

- `LibraryApp` already has `select_track`, `select_album`, etc.
  (`src/library/app_impl.rs`). The new `handle_search_result_selected`
  dispatcher in `app.rs` calls those to hydrate the detail state,
  *and* pushes the matching workspace nav entry. No new library-side
  API required for v1 if the existing selectors cover artist / feed /
  track. Audit and add the missing one if not.

### `tests/architecture_tests.rs`

Update guards:

- The existing
  `adr_0047_task_015_search_submit_and_saved_search_commands` test
  asserts toolbar search opens a Detail frame. Update to assert
  toolbar search opens Search nav in ContentList, replacing an active search
  flow instead of stacking repeated query crumbs.
- Add a guard asserting `on_result_select` is wired at the
  `SearchResultsInspectorSlots::new()` site in `app.rs` (substring
  `.on_result_select(`).
- Add a guard asserting `WorkspaceShell::render` uses `SplitPane`
  (file contains `SplitPane::new(` or similar).
- Drop or update any guard pinning the
  `open_search_results_frame(query)` Detail-frame path if it's only
  used by tests after this plan lands.

## Critical files

| File | Change |
| --- | --- |
| `src/view_models/workspace.rs` | add `open_search_results_in_content_list`, `pop_nav_until`, `has_history`, `FrameNavigationEntry::Settings`; retire `open_search_results_frame` |
| `src/app.rs` | toolbar submit → ContentList nav push; new `handle_search_result_selected`; wire `on_result_select`; route body render on ContentList nav top; track `content_pane_width` |
| `src/ui/shells/workspace.rs` | extend `should_render_breadcrumb` for ContentList; replace flat row with `SplitPane`; consume `content_pane_width` |
| `src/ui/shells/search_results_inspector.rs` | remove the `dead_code` expectation on `on_result_select` |
| `src/ui/composites/split_pane.rs` | reuse as-is; verify the API supplies the mouse handlers `app.rs` needs |
| `src/library/app_impl.rs` | possibly add a `select_artist(name, cx)` helper if missing |
| `tests/architecture_tests.rs` | update Task 015 guard; add `on_result_select` wired guard; add `SplitPane` usage guard |
| `docs/plans/active-frame-search-dispatch-plan.md` | mark superseded; reference this plan |
| `docs/reviews/active-frame-search-dispatch-review-checklist.md` | mark Phase 2-4 as superseded by this plan |

## Reusable utilities

- `WorkspaceLayout::{push_nav, pop_nav, reset_nav, frame_nav, frame_nav_mut, focus_frame}` — already cover all nav-stack mutations needed.
- `FrameNavigationEntry::display_label` — already returns sensible breadcrumb labels (`"Library"`, `"Search: heycitizen"`, `"Track 123"` etc.).
- `BreadcrumbDisplay::project` and `FrameShellDisplay::with_breadcrumb` — already in place.
- `SearchResultsInspectorPageVm::from_local_library_tracks` — already projects local results into Artists/Feeds/Tracks tabs.
- `SplitPane` composite — proven; Library sidebar already uses it.
- `LibraryApp::select_track` / `select_album` — already hydrate detail state on click.
- `ApplicationQueryService::search_local_library_tracks` — already wired into the query service path.

## Stages

1. **Add VM helpers.** `WorkspaceLayout::open_search_results_in_content_list`, `pop_nav_until`, `FrameNavigationState::has_history`. Pure VM, GPUI-free. Unit-tested in `view_models/workspace.rs` tests module.
2. **Switch search submit destination.** Update `submit_global_search` and `submit_global_search_with` to route NewFrame to ContentList instead of Detail. Update render-body switching in `render_workspace_content`. Drop `open_search_results` from the toolbar path (keep helper if referenced elsewhere; otherwise remove).
3. **Wire result-row drill-down.** Implement `handle_search_result_selected`. Call `.on_result_select(...)` at `app.rs:914`. Remove the `dead_code` expectation on the slot. Verify clicking a result navigates the ContentList frame.
4. **Breadcrumb chrome.** Extend `should_render_breadcrumb` for ContentList nav depth > 1. Wire `on_breadcrumb_select` to `pop_nav_until`. Visual confirmation by operator.
5. **Drop Detail from visible default layout.** `visible_workspace_layout` no longer inserts a Detail frame when none exists in the model. Keep the Detail kind in the workspace model — just don't auto-insert.
6. **SplitPane wrap.** Replace the flat row in `WorkspaceShell::render` with `SplitPane(leading=ContentList, trailing=Queue)`. Track `content_pane_width: Pixels` on `TopApp`. v1 in-memory only; no persistence.
7. **Architecture guards.** Update Task 015 guard; add `on_result_select` wire guard; add `SplitPane` usage guard.
8. **Doc closeout.** Mark `active-frame-search-dispatch-plan.md` superseded with a one-line pointer to this plan.

Sequence: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8. Stages 1 and 6 can run in parallel via subagents; 2-5 touch overlapping code paths in `app.rs` and `workspace.rs` so they're sequential.

## Verification

After each stage, all five must pass:

```bash
cargo fmt -- --check
cargo build 2>&1 | tail -5
cargo test --lib 2>&1 | tail -5
cargo test --test architecture_tests 2>&1 | tail -5
cargo clippy -- -D warnings 2>&1 | tail -5
```

Operator end-to-end (after `cargo install --path . --force` and
relaunching `v4vmm`):

1. Open the app. Workspace shows Library tree on the left (full
   width), Queue on the right. No standalone Detail frame.
2. Drag the divider between Library and Queue → both panes resize.
3. Type `heycitizen` in the toolbar and press Enter. The Library
   frame's body switches to the search-results inspector (Artists /
   Feeds / Tracks tabs). The frame chrome shows a breadcrumb:
   `Library › Search: heycitizen`.
4. Click a result row (e.g., a Track). The frame body switches to the
   track inspector. Breadcrumb becomes
   `Library › Search: heycitizen › Track …`.
5. Click the `Search: heycitizen` breadcrumb segment. Body returns to
   the search results. Click `Library`. Body returns to the library
   tree.
6. Queue is unaffected throughout. No third pane appears.

Sandbox cannot init GPUI X11; the cargo gates are the in-sandbox
proof. Operator confirms 1–6 in a working desktop session.
