# Discovery and Library UI Fixes Plan

Status: reviewed import
Date: 2026-05-01
Imported from: `/home/citizen/.claude/plans/typing-a-backslash-stateless-sun.md`

Related docs:

- [ADR 0023: Design system and view models](../adr/0023-design-system-and-view-models.md)
- [ADR 0025: Theme icon style boundary](../adr/0025-theme-icon-style-boundary.md)
- [ADR 0026: Shared entity projection layer](../adr/0026-shared-entity-projection-layer.md)
- [ADR 0027: Shared entity action state](../adr/0027-shared-entity-action-state.md)
- [Shared entity projection phase plan](adr-0026-shared-entity-projection-phase-plan.md)
- [Unify Discover and Library views](unify-discover-library-views.md)

## Review Summary

This plan is a post-ADR-0026 UI consistency and correctness follow-up. It is
appropriate for `docs/plans/` because it describes several bounded
implementation slices, not a new long-lived architecture by itself.

The plan is acceptable with these constraints:

- Implement one slice per session or task packet. Do not combine all six fixes
  into one large implementation pass.
- Treat the line numbers from the imported source as discovery hints. Re-check
  the current code before editing.
- Keep changes inside the existing ADR 0023, ADR 0025, ADR 0026, and ADR 0027
  boundaries. If implementation introduces a new composite family, new
  ownership boundary, new persistence model, or non-additive projection
  contract, write an ADR first.
- Keep shared view models GPUI-free. Screen modules may own command dispatch,
  image cache resolution, popovers, subscriptions, and async service calls.
- Preserve provenance for metadata. Contributor formatting must not discard raw
  source fields or collapse conflicting source facts.

Apple HIG review notes for the macOS desktop surface:

- Search input should be forgiving and should not leave the view in a broken
  state after a typed structural character.
- Scrollable detail panes must support expected macOS input paths: wheel or
  trackpad scrolling, scroll bars, and keyboard navigation.
- Inspector/detail surfaces should show properties relevant to the selected
  object and suppress irrelevant actions, such as Library-only compare actions
  in Discovery.
- Header/action layout should make labels, identity facts, and actions easy to
  scan without mixing data fields into button rows.
- Destructive or broad actions introduced later, such as "Remove all", must not
  be styled as a primary action.

Rust review notes:

- Extend existing helpers instead of duplicating query normalization, feed title
  fallback, entity action gating, contributor grouping, or release detail shells.
- Add focused tests around every changed pure helper or projection.
- Keep UI-specific formatting out of metadata persistence. A display helper may
  format contributor data, but source-fact storage and single-line summaries
  should remain available for existing callers.
- Prefer additive public/internal APIs in composites and view models. Avoid
  parallel `FeedHeader` or context flag types when existing types already carry
  the needed intent.

## Goal

Fix six visible Discovery and Library problems while preserving the shared
projection and design-system boundaries already established by ADR 0023 through
ADR 0027.

The six user-visible problems are:

1. Discovery search errors when a query contains `\`.
2. Discovery recents tiles render artwork without visible title or artist
   labels.
3. Library and Discovery feed-view headers show the same facts with inconsistent
   structure and action placement.
4. Discovery track views show Library-only compare actions.
5. Contributor metadata cells render a flat `name: role` summary where the UI
   should show one person with indented roles.
6. Multiple detail surfaces do not scroll reliably.

## Non-Goals

- Do not redesign the overall Library or Discovery navigation model.
- Do not create a new source abstraction or undo the ADR 0026 projection layer.
- Do not infer or synthesize metadata facts that are not already present in the
  source data.
- Do not change database schema for these fixes unless a later ADR approves it.
- Do not change subscription, download, or compare semantics beyond hiding
  irrelevant controls in Discovery.

## Current State

- Discovery search forwards a backslash to the remote search endpoint because
  `sanitize_api_query_value` strips control and null characters but not `\`.
- Recents tiles use feed title and artist fallback chains, but current API
  payload aliases may no longer populate the fields those chains read.
- Library and Discovery feed details both use shared release-detail machinery,
  but `DetailHeader` only carries a narrow title/subtitle/image shape, so
  additional facts and actions are arranged differently by each screen.
- `EntitySurfaceContext` already distinguishes `Discover` from `Library`, but
  compare actions are still projected or rendered where Discovery can see them.
- A contributor tree composite already exists for the dedicated Contributors
  panel, while metadata-cell rendering can still use a flat contributor string.
- Several scroll containers use `size_full().overflow_y_scroll()` in flex-column
  contexts that need bounded `flex_1().min_h_0()` behavior.

## Target State

- Search queries containing `\` are normalized before reaching the API query
  parser, and subsequent searches continue to work.
- Discovery recents tiles show stable title and artist labels from current API
  responses.
- Feed detail headers render the same data structure in Library and Discovery:
  artwork, title, subtitle or artist, publisher, description, npub, website,
  then action rows.
- Discovery projections do not expose Compare ID3 or Compare MusicBrainz
  actions.
- Contributor metadata displays can expand to a tree shape while existing
  single-line summaries remain available to callers that need them.
- Library detail, Discovery inspector, and settings panes scroll with mouse,
  trackpad, scroll bar, and keyboard input.

## Implementation Slices

### Slice 1: Discovery Backslash Search

Likely files:

- `src/api.rs`
- `src/search.rs`
- `src/view_models/search.rs`

Implementation:

- Extend `sanitize_api_query_value` so `\` is replaced with a space, matching
  the existing treatment of characters that should never reach the server query
  parser.
- Add sanitizer coverage for `\`, `\\`, and embedded backslashes, asserting that
  the final URL value contains no `%5C`.
- If manual testing still finds a sticky error state after unrelated server
  failures, clear the previous status before beginning the next request.

Acceptance criteria:

- Typing `\` in Discovery and pressing Enter does not break the session.
- Typing a mixed query such as `john\doe` starts a search with a sanitized query.
- The sanitizer change is covered by a unit test.

### Slice 2: Recents Tile Labels

Likely files:

- `src/api.rs`
- `src/views.rs`
- `src/search.rs`
- `src/view_models/search.rs`

Implementation:

- Add or update a deserialization test using the shape returned by
  `/v1/feeds/recent`.
- Confirm which response fields populate `Feed` title and artist data.
- Restore missing `serde` aliases or update the existing fallback chain to read
  the current field names.
- Reuse the existing `feed_title()` fallback instead of adding another title
  helper.

Acceptance criteria:

- Default Discovery recents tiles show artwork, title, and an artist or
  publisher label when present in the API response.
- A focused deserialization or view-model test covers the field alias.

### Slice 3: Unified Feed-View Header

Likely files:

- `src/ui/composites/detail_header.rs`
- `src/ui/composites/release_detail_surface.rs`
- `src/ui_entity.rs`
- `src/ui_feed.rs`
- `src/search.rs`
- `src/library.rs`

Implementation:

- Extend the existing `DetailHeader` additively with optional structured data
  slots for description, publisher, website, and npub.
- Keep action buttons out of the header data block. Route actions through
  `ReleaseDetailSlots.action_row` or `identity_actions`.
- Feed the same fields from Library and Discovery call sites.
- Do not create a parallel `FeedHeader`.

ADR trigger:

- No new ADR is required for a strictly additive extension of the existing
  composite and slot pattern.
- Write an ADR first if implementation changes ownership boundaries, replaces
  the shell pattern, or introduces a new family of entity-header composites.

Acceptance criteria:

- Library album/feed detail and Discovery feed detail share the same header
  structure.
- Data fields and actions are visually separated.
- Existing feed, track, and contributor detail behavior remains intact.

### Slice 4: Hide Compare Buttons in Discovery

Likely files:

- `src/view_models/entity_detail.rs`
- `src/ui_feed.rs`
- `src/library.rs`
- `src/search.rs`

Implementation:

- Gate compare-action projection on `EntitySurfaceContext::Library`.
- Use the existing `EntitySurfaceContext` enum. Do not introduce a second
  boolean or screen flag.
- Add a unit test asserting that Discover-context projections contain no compare
  actions.

Acceptance criteria:

- Discovery track views show no Compare ID3 or Compare MusicBrainz buttons.
- Library track views retain compare actions.
- The projection-layer test documents the contract.

### Slice 5: Contributor Metadata Tree

Likely files:

- `src/metadata.rs`
- `src/library.rs`
- `src/search.rs`
- `src/ui_entity.rs`

Implementation:

- Reuse `grouped_contributor_entries` to format expanded metadata-cell display
  as:

  ```text
  Name
    - role
    - role
  ```

- Keep `summarize_contributors` and `summarize_contributor_value` available for
  callers that need a single-line string.
- Add one helper for tree formatting rather than duplicating formatting in
  Library and Discovery.
- Do not alter source-fact storage or discard conflicting contributor data.

Acceptance criteria:

- Dedicated Contributors panels remain tree-based.
- Expanded `TXXX:MusicIndex Contributors` metadata cells render one node per
  person with indented roles.
- Collapsed summaries and compare diff text keep their existing single-line
  behavior unless explicitly changed by the slice.

### Slice 6: Scroll Containers

Likely files:

- `src/ui/composites/release_detail_surface.rs`
- `src/library.rs`
- `src/app.rs`
- `src/search.rs`

Implementation:

- In `ReleaseDetailSurface::render`, change the scrollable branch from
  `size_full().overflow_y_scroll()` to a bounded flex child shape such as
  `flex_1().min_h_0().overflow_y_scroll()`, preserving existing padding.
- Apply the same bounded-scroll correction to direct detail panes that use
  `size_full().overflow_y_scroll()` in flex-column parents.
- If scrolling still fails, fix the missing bounded-height ancestor instead of
  adding leaf-level workarounds.

Acceptance criteria:

- Library artist, playlist, track, and album details scroll to the end.
- Discovery inspector surfaces scroll to the end.
- Settings scrolls to the end.
- Mouse or trackpad scrolling, scroll bar dragging, and arrow-key/page-key
  navigation work where the framework supports them.

## Files To Re-Check Before Implementation

The imported source listed these files as likely touch points. Re-check the
current code before editing because line numbers may drift.

- `src/api.rs`
- `src/app.rs`
- `src/library.rs`
- `src/metadata.rs`
- `src/search.rs`
- `src/ui/composites/detail_header.rs`
- `src/ui/composites/release_detail_surface.rs`
- `src/ui_entity.rs`
- `src/ui_feed.rs`
- `src/view_models/entity_detail.rs`
- `src/view_models/search.rs`
- `src/views.rs`

## Risks

- Header unification can turn into an architecture change if it creates a new
  composite family instead of extending the existing slot pattern.
- Backslash normalization may alter literal search intent. The current plan
  accepts that tradeoff because the character is unsafe for the remote query
  parser.
- Recents labels may involve an API payload change rather than a local alias
  regression. Confirm with a fixture before editing render fallbacks.
- Scroll fixes may require parent-chain changes. Leaf-only edits may appear to
  work in one pane while leaving another broken.
- Contributor display formatting can accidentally conflate provenance if the
  formatter is reused for storage or source-fact comparison. Keep it display
  only.

## Rollback Strategy

- Implement each slice in its own commit or task packet.
- Roll back by reverting the slice-specific commit. The slices are intended to
  be independent except that the shared header and scroll work both touch the
  release detail shell.
- If header unification causes unexpected layout regressions, revert the
  `DetailHeader` additions and restore caller-local placement while preserving
  non-header fixes.
- If query sanitization causes unacceptable search behavior, revert only the
  sanitizer predicate change and keep any status-reset fix that is independently
  valid.

## Verification

Required command gates for implementation work:

```bash
cargo fmt -- --check
cargo check
cargo test
cargo clippy -- -D warnings
```

Focused test coverage to add:

- Sanitizer tests covering backslash queries.
- Recent feed deserialization or view-model tests covering title and artist
  aliases.
- Entity-detail projection test proving Discovery has no compare actions.
- Contributor tree formatter tests for one person with multiple roles and
  multiple people with one role each.

Manual smoke:

- Discovery: type `\`, press Enter, then run another search in the same session.
- Discovery: type `john\doe`; verify a sanitized search runs and renders
  results or a normal empty state.
- Discovery default view: verify recents tiles show title and artist/publisher
  labels.
- Open a Discovery feed and a Library album/feed: verify matching header data
  structure and action placement.
- Open a Discovery track view: verify compare actions are absent.
- Open a Library track view: verify compare actions remain.
- Open a release with multi-role contributors: verify the Contributors panel and
  expanded contributor metadata cell render tree-shaped content.
- Scroll Library artist, playlist, track, album, Discovery inspector, and
  settings panes to the end using pointer and keyboard input.

## Open Questions

- Is the current `/v1/feeds/recent` payload available as a stable fixture, or
  should the test use a minimized local sample based on observed fields?
- Should backslash normalization be API-client-wide for all query parameters, or
  only search query values?
- Should the contributor tree formatter live in `metadata.rs`, or should it be
  a view-model helper if the final use is display-only?
- Does GPUI expose enough keyboard-scroll behavior to test automatically, or
  should keyboard scrolling remain a manual smoke item?
