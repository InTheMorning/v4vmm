# Post-ADR-0048 Task — Recent Feeds Route

## Goal

Implement the new `FrameNavigationEntry::RecentFeeds` route under the
ContentList frame so Recent Feeds is reachable again from the live UI.
Add a toolbar command as the entry point. Reuse the existing
MusicIndex feed-result row rendering and Index feed detail drill-down so
there is no new visual surface to maintain.

Reachability bug recap: empty-query search no longer opens Recent Feeds
(`src/app/search_dispatch.rs:60` short-circuits) and the legacy
Recent Feeds renderer is in the parked Discover module
(`docs/notes/2026-05-discover-module-parked.md`). MusicIndex still
exposes `fetch_recent_feeds` at `src/api.rs:426`.

Plan reference: `docs/plans/post-adr-0048-recent-feeds-route-plan.md`.

## Files To Inspect

Read first:

- `docs/plans/post-adr-0048-recent-feeds-route-plan.md` (this task's
  parent plan; *Decision*, *Invariants*, *Non-Goals*).
- `docs/adr/0048-content-list-frame-breadcrumb-search.md` (governing ADR).
- `src/view_models/workspace/nav.rs` — `FrameNavigationEntry` enum and
  `display_label`.
- `src/view_models/workspace/breadcrumb.rs` — breadcrumb derivation from
  nav entries.
- `src/app/search_dispatch.rs` — full file. The `start_index_search_for_query`
  function is the reference pattern for async result loading.
- `src/app/tab_bar.rs` — toolbar render including
  `render_global_search_submit_button`. The new toolbar action sits next
  to it.
- `src/view_models/search_results/index_detail.rs` and
  `src/view_models/search_results/results.rs` — Index feed result row
  types used by `SearchResultsInspector`.
- `src/ui/shells/search_results_inspector.rs` — current Index feed row
  rendering. Identify the smallest reusable rendering call site that
  takes a list of Index feed result rows and renders them.
- `src/api.rs:426` — `fetch_recent_feeds(limit, cursor) -> Result<RecentFeedsResponse>`.
- `src/app.rs` body-switch dispatch for the ContentList frame
  (search for the existing match on `FrameNavigationEntry::Search`).
- `tests/architecture_tests.rs` — locate existing search-flow guards
  (`nav_top_drives_content_list_body_switch`,
  `global_search_routes_to_content_list`,
  `search_results_detail_syncs_with_search_nav_flow`) to match style.

## Files Likely To Change

- `src/view_models/workspace/nav.rs` — add `RecentFeeds` variant; extend
  `display_label`; update any exhaustive matches.
- `src/view_models/workspace/breadcrumb.rs` — handle the new variant.
- `src/view_models/recent_feeds.rs` — NEW. `RecentFeedsPageVm` with
  loading / loaded / error states; pure VM, GPUI-free.
  (Alternatively place under `src/view_models/search_results/recent_feeds.rs`
  if the implementer judges this surface belongs in the search_results
  family. Either is acceptable; the architecture guard names the path.)
- `src/view_models/mod.rs` (or `view_models/search_results/mod.rs`) —
  register the new module.
- `src/app/search_dispatch.rs` — add `open_recent_feeds_in_content_list`
  and `start_recent_feeds_load`, both mirroring the shape of
  `open_search_results_in_content_list` and `start_index_search_for_query`.
- `src/app/tab_bar.rs` — add the toolbar Recent Feeds button.
- `src/app.rs` — extend the ContentList body-switch dispatch with a
  `FrameNavigationEntry::RecentFeeds` arm that renders the new VM.
- `tests/architecture_tests.rs` — new guard
  `recent_feeds_route_is_reachable_from_toolbar` (or similar name) that
  pins the variant existence, the toolbar entry-point function, and the
  body-switch arm.

Probable touches but verify:

- Any `match self.current` on `FrameNavigationEntry` outside the files
  above will need a new arm (compiler will flag).
- Test fixtures that construct `FrameNavigationEntry` values.

## Do Not Touch

- `src/discover/**` (parked module — no revival).
- `src/ui/shells/discover/**`.
- Empty-query semantics in `submit_global_search`. The empty short-circuit
  stays; do not restore the empty-query → Recent Feeds path.
- Public API of `SearchResultsInspectorPageVm`. Recent Feeds gets its own
  VM rather than overloading the search inspector VM.
- The existing Index feed result row composite — reuse, do not edit
  unless the reuse path requires it.

## Constraints

- All ADR 0048 invariants from the parent plan must hold post-change.
- The new variant is a **singleton**: `FrameNavigationEntry::RecentFeeds`
  carries no payload.
- Activating the toolbar action while the active nav entry is already
  `RecentFeeds` triggers a refresh (re-run the async load), not a stack
  push. Treat this analogously to
  `replace_active_search_or_push` for the search variant.
- Row activation from Recent Feeds pushes
  `FrameNavigationEntry::IndexFeedDetail` (existing variant). No new
  detail variant.
- Stale-response guarding: when the async response lands, check that
  the active ContentList nav top is still `RecentFeeds`. If not, drop
  the response (do not write into the VM).
- Empty `RecentFeedsResponse.data` renders through the existing
  `render_empty_state` path. No raw transport text in the body.
- `RecentFeedsPageVm` is GPUI-free. No `gpui::` imports.
- No new `#[allow(...)]` directives.
- Never skip hooks. Don't commit unless explicitly asked.

## Implementation Steps

1. Read this task file, the parent plan, and every file in *Files To
   Inspect*.
2. Add `FrameNavigationEntry::RecentFeeds` to `nav.rs`. Update
   `display_label` to return `"Recent Feeds"`. Compile and fix every
   exhaustive-match error introduced.
3. Update `breadcrumb.rs` so the breadcrumb derivation returns a
   single-segment path for `RecentFeeds`. Decide whether the segment
   acts as a current-position (non-clickable) or a top-of-stack root —
   match whatever `Search` does for consistency.
4. Create `RecentFeedsPageVm` with the following states:
   - `Loading` (initial after a load is kicked off).
   - `Loaded(rows: Vec<IndexFeedResultRow>)` — reuse the existing
     Index feed result row type from `view_models/search_results`.
   - `Error(message: String, detail: String)`.
   Constructor takes no arguments; an initial VM is `Loading`. Provide
   methods `mark_loading()`, `replace_feeds(Vec<IndexFeedResultRow>)`,
   `set_error(message, detail)`, and a getter for the current state
   shape needed by rendering.
5. Add `open_recent_feeds_in_content_list(&mut self, cx)` on `TopApp`
   (in `search_dispatch.rs`) that:
   - If the active ContentList nav top is already `RecentFeeds`, calls
     `replace_current(RecentFeeds)` (idempotent) and re-runs the async
     load.
   - Otherwise pushes `RecentFeeds` onto the ContentList nav stack via
     the workspace layout.
   - Initializes `self.recent_feeds_detail = Some(RecentFeedsPageVm::loading())`.
   - Calls `start_recent_feeds_load(cx)`.
6. Add `start_recent_feeds_load(&mut self, cx)` mirroring
   `start_index_search_for_query`: spawn into the background executor
   via `cx.spawn`, run `fetch_recent_feeds(None, None)` against the
   active endpoint, write the result back through `this.update` after
   confirming the active nav top is still `RecentFeeds`.
7. Add the toolbar button in `src/app/tab_bar.rs` next to the global
   search submit button. Use the same `IconName` family — pick the
   closest available SF Symbol (e.g., `IconName::Rss` if it exists, else
   document the gap in the report and use the closest semantic icon).
   Label: "Recent Feeds". Action: dispatch
   `open_recent_feeds_in_content_list`.
8. Extend the ContentList body-switch dispatch in `src/app.rs` with a
   `FrameNavigationEntry::RecentFeeds` arm that renders the new VM.
   Reuse the Index feed result row rendering from
   `search_results_inspector.rs` — either by factoring a shared helper
   if the existing call site is non-trivial, or by inlining a small
   loop that calls the same row composite. Empty state renders through
   `render_empty_state`.
9. Wire row activation: a click on a feed row pushes
   `FrameNavigationEntry::IndexFeedDetail { id, label }` exactly as
   the search row activation does today.
10. Add the architecture guard
    `recent_feeds_route_is_reachable_from_toolbar` in
    `tests/architecture_tests.rs`. The guard must assert:
    - `FrameNavigationEntry::RecentFeeds` variant exists in `nav.rs`.
    - `tab_bar.rs` contains the toolbar entry point function or button.
    - `search_dispatch.rs` contains `open_recent_feeds_in_content_list`
      and `start_recent_feeds_load`.
    - The ContentList body-switch dispatch in `app.rs` matches on
      `FrameNavigationEntry::RecentFeeds`.
    Use the same source-walk style as the existing search-flow guards.
11. Run the five gates.

## Acceptance Criteria

- `FrameNavigationEntry::RecentFeeds` variant present; `display_label`
  returns `"Recent Feeds"`.
- Toolbar action visible next to the search submit button; clicking it
  opens the route.
- The route renders a feeds-only list using the same row composite as
  Index feed search results.
- Row click pushes `IndexFeedDetail` and the existing detail surface
  renders; breadcrumb pop returns to the Recent Feeds list.
- Refresh (toolbar action invoked while route is active) re-runs the
  async load without stacking nav entries.
- Empty result set renders through `render_empty_state`.
- Stale-response guarding prevents a late response from overwriting the
  VM when the user has navigated away.
- Architecture guard
  `recent_feeds_route_is_reachable_from_toolbar` passes; removing any of
  its named anchors causes it to fail.
- All five gates pass.
- No new `#[allow(...)]` directives.
- No code under `src/discover/` or `src/ui/shells/discover/` modified.

## Test Commands

```bash
cargo fmt -- --check
cargo build 2>&1 | tail -5
cargo test --lib 2>&1 | tail -5
cargo test --test architecture_tests 2>&1 | tail -5
cargo clippy -- -D warnings 2>&1 | tail -5
```

Operator-visible UI smoke (if running locally):

1. Click the Recent Feeds toolbar button. ContentList renders a list of
   feeds without typing a query.
2. Click a feed row. Detail inspector for that Index feed renders;
   breadcrumb shows `Recent Feeds > <feed label>`.
3. Click the breadcrumb root. List returns to Recent Feeds without a
   second fetch.
4. Click the toolbar button while already on Recent Feeds. List
   re-fetches; nav stack does not grow.
5. Submit an empty query in the search input. Nothing happens (no
   regression of empty-query → Recent Feeds behavior).

## Prompt for lower-context coding model

You are implementing one bounded feature task.

Read:

- This task file in full.
- `docs/plans/post-adr-0048-recent-feeds-route-plan.md`.
- `docs/adr/0048-content-list-frame-breadcrumb-search.md` (Invariants
  section).
- Every file in *Files To Inspect*.

Goal:

Add a `FrameNavigationEntry::RecentFeeds` singleton variant. Render
Recent Feeds inside ContentList via a new GPUI-free `RecentFeedsPageVm`
that loads results asynchronously from `fetch_recent_feeds`
(`src/api.rs:426`). Add a toolbar button next to the existing global
search submit button as the entry point. Reuse the Index feed result row
rendering already used by `SearchResultsInspector`. Row activation
pushes the existing `FrameNavigationEntry::IndexFeedDetail`. Add one
architecture guard pinning the route's anchors.

Constraints:

- Behavior of all existing ADR 0048 invariants preserved.
- Singleton variant — no payload.
- Toolbar action on an active Recent Feeds route is a refresh
  (replace_current + re-load), not a push.
- Stale-response guarding mirrors `start_index_search_for_query`.
- `RecentFeedsPageVm` is GPUI-free.
- No empty-query branch added back to `submit_global_search`.
- No edits to `src/discover/**` or `src/ui/shells/discover/**`.
- No new `#[allow(...)]`.
- Never skip hooks. Don't commit unless explicitly asked.

Test commands:

- `cargo fmt -- --check`
- `cargo build`
- `cargo test --lib`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:

1. Files changed.
2. Final `FrameNavigationEntry` enum shape.
3. Where the toolbar button was added (file:line, icon used, label).
4. The new `RecentFeedsPageVm` module path and state shape.
5. The async-load function name and the stale-guard check used.
6. Architecture guard name and what it asserts.
7. Five-gate results.
8. Deviations from the plan.
9. Unresolved concerns.

## Escalation Triggers

- The Index feed result row rendering in
  `src/ui/shells/search_results_inspector.rs` is too entangled with the
  search inspector's tab/filter state to be reused directly. Report the
  call shape; propose either extracting a small composite for the row
  list or inlining a minimal copy; do not silently couple Recent Feeds
  to search inspector state.
- `IconName` does not contain a clearly semantic icon for "Recent Feeds"
  (e.g., neither `Rss` nor a clock/recent icon). Use the closest
  available, label it in the report, and flag for follow-up.
- `fetch_recent_feeds` requires an endpoint string that lives somewhere
  other than `self.endpoint_input`. Report where the search dispatch
  currently sources its endpoint and follow the same pattern; do not
  invent a new endpoint location.
- The ContentList body-switch dispatch lives in a different file than
  `src/app.rs` (e.g., one of the post-ADR-0050 submodules). Report and
  add the arm where the existing `Search` arm lives.
- Stale-response guarding requires a helper that does not exist yet
  (the search path uses `content_list_nav_matches_search`). Add a
  sibling helper `content_list_nav_is_recent_feeds` next to it.
