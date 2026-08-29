# ADR 0049: Inspector Source Ownership

## Status

Implemented - 2026-05-18.

Ownership invariants are guarded by
`adr_0049_inspector_source_ownership_is_guarded` plus the frame-local filter
guards in `tests/architecture_tests.rs`. Operator visual smoke confirmed the
remove/download, Index drill-down, and content-filter behavior during the
2026-05-18 completion pass.

## Context

ADR 0048 moved toolbar search into the workspace `ContentList` frame and made
search results load local Library rows synchronously plus remote MusicIndex
Index rows asynchronously. Operator testing then exposed three ownership
regressions:

- Search results can show Index-only rows, but selecting those rows still takes
  the old local-library drill-down path.
- Removing a track from a library album updates the left Library tree, while
  the currently mounted album inspector keeps stale membership/action state and
  artwork. Navigating away and back rebuilds from library-only rows, so removed
  tracks vanish instead of becoming downloadable Index rows.
- The `All / Library / Index` content filter in frame chrome is currently
  wired to the Library source tree for Library-backed content. It should filter
  the current inspector/detail surface.
- Settings tab navigation destroys the Library/Search content state. Operator
  smoke testing confirmed that tab switches should preserve the current Library
  or Search navigation state. Breadcrumbs are the user's way to go back.

This violates the ADR 0047 and ADR 0048 goal that content-source filtering is a
frame/inspector concern, not a global source-list concern. It also breaks the
same-view mutation invariant: a successful mutation must update the mounted
view in place.

## Decision

Content ownership is split by surface:

- **Library source tree:** local-library navigation only. It may update after
  removals, but it never owns `All / Library / Index` filtering.
- **ContentList inspector/detail:** owns content-source filtering for the
  currently visible body. `All` shows both local Library and remote Index facts,
  `Library` shows local membership only, and `Index` shows remote/non-library
  source facts.
- **Search result drill-down:** result activation preserves origin identity.
  Local rows drill into local Library-backed detail. Index rows push explicit
  remote navigation entries: `IndexArtistDetail`, `IndexFeedDetail`, and
  `IndexTrackDetail`. Those entries use MusicIndex identities, not local
  database row IDs.
- **Mutation success path:** remove/download commands update or invalidate the
  currently mounted inspector before the operator navigates away. A removed
  local item that is still known by MusicIndex remains visible as Index-origin
  content under `All` and `Index`, with a `Download` action.
- **Tab switching:** Settings is a temporary ContentList mount. Entering
  Settings saves the current Library/Search ContentList navigation state, and
  returning to Library restores it instead of resetting to the Library root.
- **Index artist activation:** activating an Index artist row pushes an
  `IndexArtistDetail` navigation entry and shows Index feeds for that artist.
  The detail surface includes a breadcrumb back to the originating search
  result, giving the operator a reversible drill-down path without inventing a
  local Library detail from remote facts.
- **Uncached Index feed/track activation:** activating an uncached Index feed or
  track pushes `IndexFeedDetail` or `IndexTrackDetail` and renders an Index
  detail surface. It must not rely on a Settings-only status string.

The view-model layer owns default labels, membership/action projection, filter
state, and origin-aware row membership. GPUI renderers bind those contracts and
dispatch commands. They do not infer missing membership or hide stale rows.

## Invariants

- The Library tree never changes its row set because a frame content filter was
  selected.
- Content filters are local to the active ContentList detail/search inspector
  surface.
- Remote Index activation never parses remote IDs as local SQLite IDs.
- Remove/download success updates the currently mounted detail in place.
- Removed local album tracks that remain available from Index are projected as
  remote-only rows with `Download`, not stale `Remove`.
- Downloading one of those rows updates the mounted album row back to Library
  content in place.
- Uncached Index feed/track activation pushes a detail navigation entry and
  renders a breadcrumb-backed Index detail surface.
- Index artist activation shows Index feeds with a breadcrumb back to the search
  result.
- Library/Settings tab switches are non-destructive for Library/Search
  ContentList navigation.
- `LibraryApp.detail` must not be the sole source of truth for mixed
  Library/Index content. A detail VM must carry origin and membership state.
- Renderers must not fix stale source facts by dropping non-empty metadata.

## Non-Goals

- No new database schema is introduced in this ADR.
- No deletion of the compiled Discover module.
- No new top-level Search tab or secondary search frame.
- No playlist, playback, or ID3 metadata behavior changes except where existing
  remove/download commands must refresh current inspector state.

## Alternatives Considered

- **Commit the current code and file follow-up bugs.** Rejected. That would make
  known ownership regressions the baseline and contradict the regression ratchet
  for user-confirmed behavior.
- **Filter the Library tree and inspector together.** Rejected. It makes the
  navigation source list unstable and hides the operator's path back to local
  content.
- **Hide removed rows from the current inspector immediately.** Rejected. This
  loses the MusicIndex source fact and removes the expected way to download the
  item again from the same album context.

## Consequences

Positive:

- Source-list navigation stays predictable.
- Search and Library details share the same origin-aware ownership model.
- Mutation feedback is immediate and reversible through a `Download` action.
- Remote drill-down becomes a first-class path instead of an accidental local-ID
  parse.

Negative / risks:

- Library-backed detail VMs need to carry mixed-origin rows, not only local
  `TrackRow` snapshots.
- Some existing selectors may need remote identity parameters in addition to
  local IDs.
- Tests must distinguish source tree filtering from inspector filtering so the
  behavior does not regress.

## References

- ADR 0047 - Library and search unification
- ADR 0048 - ContentList frame breadcrumb search
- `docs/plans/inspector-source-ownership-phase-plan.md`
