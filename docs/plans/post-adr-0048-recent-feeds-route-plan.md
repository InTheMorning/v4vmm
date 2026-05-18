# Post-ADR-0048 Recent Feeds Route Plan

## Status

Implemented - 2026-05-18.

## Goal

Restore reachability for **Recent Feeds** browsing inside the ContentList
frame, as a first-class navigation route, without reviving the parked
`src/discover/` module.

The current build dropped Recent Feeds when ADR 0048 made search a global
toolbar command. The previous workflow ("empty toolbar query shows Recent
Feeds") is now stale because:

- `src/app/search_dispatch.rs:60` short-circuits on empty queries
  (`if query.is_empty() { return; }`), so no nav entry is opened.
- The legacy Recent Feeds renderer lives under the parked Discover module
  (`docs/notes/2026-05-discover-module-parked.md`), which has no render
  path from the composition root and is not eligible for re-mount.
- The MusicIndex endpoint is still working — `src/api.rs:426` exposes
  `fetch_recent_feeds(limit, cursor)` returning `RecentFeedsResponse`.

The only user-visible workaround today is hitting
`https://api.musicindex.org/v1/feeds/recent?limit=20` directly. That is a
reachability bug, not a design decision.

## Decision

Add a new `FrameNavigationEntry::RecentFeeds` variant to
`src/view_models/workspace/nav.rs` and route to it via a toolbar command,
following the same shape ADR 0048 established for `FrameNavigationEntry::Search`.

- The variant is singleton (no payload).
- `display_label()` returns "Recent Feeds".
- ContentList body switches on the nav-top variant and renders a
  feeds-only list backed by a new `RecentFeedsPageVm`.
- A toolbar action — sibling to the existing global-search submit
  button in `src/app/tab_bar.rs` — opens the route.
- The route defaults to an artwork-first tiled view and provides a
  VM-owned view-mode selector to switch to the compact list view.
- Async load follows the same pattern as `start_index_search_for_query`:
  spawn, run `fetch_recent_feeds` on the background executor, match the
  active nav entry against the in-flight request, write into the VM,
  `cx.notify()`.
- `RecentFeedsPageVm` owns cursor pagination state (`cursor`,
  `has_more`, and `loading`). The route appends cursor pages on scroll,
  exposes a fallback "Load more" button, and eagerly prefetches page two
  after the initial page when the Index reports more rows.
- Result rows reuse the feed-result row rendering already used by
  Index search results via the shared result-row shell helper so we do
  not duplicate feed-row visual logic. Activation of a row pushes the existing
  `FrameNavigationEntry::IndexFeedDetail` so drill-down lands in the
  same Index feed inspector users already get from search.
- Breadcrumb chrome works automatically: ContentList nav top of
  `RecentFeeds` produces a single-segment breadcrumb; drilling into a feed
  pushes the IndexFeedDetail entry and the breadcrumb pop returns to
  the Recent Feeds list.

## Why not other options

- **Restore empty-query search semantics.** Rejected. ADR 0048 invariant
  `Toolbar Search submit is workspace-VM-owned: it pushes from non-search
  content and replaces the active search flow from an existing search`
  is incompatible with an empty-query special case that resolves to a
  different content surface. Empty-query overload also hides the
  affordance behind invisible behavior.
- **Revive the parked Discover module.** Rejected. The parked-module note
  (`docs/notes/2026-05-discover-module-parked.md`) lists explicit
  conditions for deletion; re-mounting is not one of them and would
  contradict the ADR 0048 single-pane visible-layout invariant.
- **Sidebar "Recent" pseudo-source row.** Deferred. Requires
  SourceList to grow a non-feed pseudo-source affordance. Reasonable
  follow-up if this route earns persistent placement, but the toolbar
  command is the smaller first step and matches the search-as-command
  pattern from ADR 0048.

## Invariants

(Inherits all ADR 0048 ContentList nav invariants and adds the following.)

- `FrameNavigationEntry::RecentFeeds` is a singleton route variant.
  Submitting the toolbar action while the active nav entry is already
  `RecentFeeds` is a refresh (re-fetch), not a stack push.
- Recent Feeds row activation pushes
  `FrameNavigationEntry::IndexFeedDetail` exactly as Index search row
  activation does. No new detail variant; no detail divergence.
- `RecentFeedsPageVm` is GPUI-free and lives under
  `src/view_models/recent_feeds.rs` (new) or `src/view_models/search_results/`
  if the implementer judges the recent-feeds surface to be a third
  branch alongside search and index detail.
- Recent Feeds presentation state is VM-owned. The shell renders the
  selected mode; it does not infer or store the current mode.
- Recent Feeds cursor pagination is VM-owned. Shell scroll/listener
  code dispatches a load-more intent but does not own cursor state.
- Async completions are race-guarded against stale loads in the same
  way `start_index_search_for_query` guards search: if the active nav
  entry is no longer `RecentFeeds` when the response lands, drop the
  response.
- Empty `RecentFeedsResponse.data` renders as an empty-state display
  through the existing `render_empty_state` path, not as raw transport
  text.
- No empty-query branch is restored in `submit_global_search`.

## Non-Goals

- No SourceList pseudo-source row.
- No Discover module re-mount.
- No persistent "last visited recent feeds at X" state.
- No new ADR. This is a follow-up under ADR 0048's existing nav
  contract; the new variant is documented in this plan and an architecture
  guard pins its behavior.

## Shipped Scope Notes

The implemented route intentionally includes two pieces that were not in
the first draft of this plan:

- **Tile/list presentation mode.** User smoke confirmed the old Discover
  default was an artwork-first tiled browser. The route now defaults to
  tiles and keeps the list as a secondary view.
- **Cursor pagination.** User smoke caught the loss of scroll-to-load-more
  behavior from Discover. The route now preserves that workflow with
  VM-owned cursor state, scroll auto-pagination, eager page-two prefetch,
  and a fallback "Load more" control.
- **Shared pagination helper.** Extracting the scroll pagination policy
  from the legacy search VM required import-only retargets in parked
  Discover shells that still compile. This does not re-mount Discover or
  change its presentation behavior; it keeps pagination policy owned by
  `src/view_models/pagination.rs`.

## Proposed Sequence

1. **Task 001 — Implement Recent Feeds route.** Single task file at
   `docs/tasks/post-adr-0048-task-recent-feeds-route.md`. Implements all
   the items in the Decision section behind one architecture guard.

The work is small enough for one bounded slice; no phase plan is needed.

## References

- ADR 0048 — ContentList frame breadcrumb search
- ADR 0049 — Inspector source ownership
- `src/app/search_dispatch.rs:60` — empty-query short-circuit
- `src/view_models/workspace/nav.rs:23` — `FrameNavigationEntry` enum
- `src/api.rs:426` — `fetch_recent_feeds`
- `src/app/tab_bar.rs:155-174` — toolbar search button siblings
- `docs/notes/2026-05-discover-module-parked.md` — parked Discover note
