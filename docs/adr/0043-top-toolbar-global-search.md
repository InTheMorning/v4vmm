# ADR 0043: Top Toolbar Now Playing Frame and Global Search

## Status

Accepted - 2026-05-08.

Implementation partial: toolbar search is built; the surviving normal/narrow,
light/dark readability check remains open in
[task 004](../tasks/adr-0043-task-004-guards-and-visual-readiness.md).

Amended 2026-09-10: reconciled this record with ADRs 0046, 0047, 0048, 0060,
and 0062. Retired the toolbar player, global scope controls, Search workspace,
and separate Recent Feeds requirements. The
[review checklist](../reviews/adr-0043-review-checklist.md) names each
replacement and the surviving gate; no new visual acceptance is claimed.

## Context

The app currently has a top tab bar that also hosts the Now Playing
control group. Library and Discover each own their own search field:
Library search filters local library tree state, while Discover search
queries MusicIndex and owns result filters.

This makes two related problems visible:

- Now Playing does not read as its own framed control space. Track
  identity and transport controls share the same visual plane as app
  navigation.
- Search is split across local and remote contexts even though people
  expect one clear place to find content in an app. Apple HIG guidance
  favors a distinct searchable location, starting broad and allowing
  scope refinement.

The recent UI direction in ADRs 0033, 0038, and 0042 requires that this
be handled structurally, not by adding screen-local wrappers or copied
search rows.

## Decision

The toolbar owns one global search input. `TopApp` binds input and commands;
`AppToolbarVm` owns presentation facts. Enter and the Search action submit the
query. Local matches remain limited to library members.

The current routing, navigation, and section set belong to ADRs 0047, 0048,
and 0060. Source filtering belongs to the content frame under ADR 0047.
ADR 0062 owns Music's default content. This ADR retains search input ownership,
display contracts, keyboard focus, and readable toolbar geometry.

The former three-zone toolbar, `GlobalSearchScope`, Search tab, and separate
recent-feeds empty state are retired. The checklist's retirement table names
the superseding ADR for each; those structures are not acceptance criteria.

## Alternatives Considered

- Keep separate Library and Discover search fields and only restyle
  them. Rejected because it preserves the split mental model and does
  not satisfy the one-searchable-location goal.
- Put global results in an inline toolbar popover. Rejected for v1
  because it introduces more floating chrome and makes grouped result
  rendering harder to verify.
- The original rejection of bottom transport placement is retired by
  ADRs 0046 and 0060; Show transport placement is now owned by ADR 0063.
- Run remote search live while typing. Deferred because existing
  command behavior is Enter/Search driven and live remote search would
  require cancellation/coalescing policy.

## Consequences

Positive:

- Search becomes a predictable top-level command, aligned with HIG
  toolbar search guidance.
- Playback placement is decided separately from toolbar search.
- Music and search-origin details share the global search command.
- The architecture gains explicit toolbar view-model contracts and guards.

Negative:

- `TopApp` owns the input entity and binds workspace-owned search routing.
- Search results need a small local query path for in-library matches.
- Narrow toolbar changes require human inspection of input and action
  readability in both themes.

## Invariants

- Toolbar display strings, placeholders, ids, and accessibility labels live
  in a GPUI-free view model.
- The toolbar and app menu do not expose a product name before naming is
  decided. MusicIndex attribution belongs in a future About/settings surface,
  not persistent top-level chrome.
- Entity surfaces do not duplicate the global toolbar search field. A
  frame-local content filter is a separate ADR 0047 affordance.
- Local Library search returns only tracks currently in the library.
- Current content filters follow ADR 0047; the retired global scope enum and
  grouped Search workspace must not be restored to satisfy this record.
- Visual proof is required in light and dark themes before the feature
  is called complete.

## Non-Goals

- No playback queue, scrubber, volume, output picker, or expanded player.
- No Spotlight/system search integration.
- No schema migration.
- No live/debounced remote search in this ADR.
- No palette redesign or broad typography reset.

## Follow-Up Work

- Revisit live local filtering or debounced remote search after the
  toolbar/result architecture is stable.
- Playback follow-ups belong to the current Show owners under ADRs 0060/0063.
