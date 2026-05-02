# ADR 0032: UI Backend Boundary and Popover Contracts

## Status

Implemented - 2026-05-02.

## Context

ADR 0031 made release detail pages render from a typed presentation contract,
but the Library add-to-playlist popovers regressed visually after the track-row
shell was shared. The root cause was boundary drift:

- Discover track rows used the canonical `AddToPlaylistPopover` composite.
- Library album and track rows still built raw screen-local `div()` panels.
- ADR 0031's shared row wrapper treated those raw panels as normal row children,
  so the playlist choices stretched across the full detail pane instead of
  floating from the trigger.

This was not a backend bug. It was a UI ownership bug: screen modules owned
popover chrome that should have belonged to shared UI primitives/composites.

## Decision

Codify a repository-wide UI/backend boundary:

- Data, DB, service, API, and command layers own persistence, network work,
  filesystem work, source facts, and mutations.
- GPUI-free view models own typed presentation contracts, action state, and
  surface policy.
- Shared UI primitives and composites own visual chrome, layout mechanics,
  floating surfaces, popover anchoring, buttons, rows, panels, and tokens.
- Screen modules own event wiring, command dispatch, image resolution,
  popover selection callbacks, and service invocation.
- Screen modules must not hand-roll chrome for any component family that has a
  canonical primitive or composite.

For playlist popovers specifically, screens may decide which playlists are
available and what command runs after selection, but the trigger, floating
surface, width, arrow, dismissal, list row treatment, and empty state belong to
`AddToPlaylistPopover` and the underlying `Popover` primitive.

## Invariants

- View models remain GPUI-free.
- UI primitives/composites do not import DB, services, API clients, or screen
  modules except for inert data types explicitly accepted by an ADR.
- Screen behavior slots may carry handlers, images, overlays, and already
  resolved data, but not alternate chrome for contract-owned zones.
- Raw source facts are preserved and projected; UI layers do not infer or
  discard them to make screens easier to render.
- A screen-local visual implementation is allowed only when no shared primitive
  or composite exists, and it must be routed into a follow-up task if it becomes
  repeated.
- Visual smoke is required when changing popovers, floating panels, row
  overlays, or release detail layout.

## Non-Goals

- No service or schema redesign.
- No navigation redesign.
- No new generalized UI framework.
- No requirement that every existing legacy panel migrate in one change.
- No change to playlist, download, playback, MusicBrainz, or subscription
  semantics.

## Alternatives Considered

- Keep fixing screen-local panels as they regress. Rejected because it leaves
  visual ownership implicit and lets equivalent screens drift.
- Move popover state into view models. Rejected for visual open/close chrome;
  view models should describe action availability, not own floating UI state.
- Let release detail slots carry arbitrary popover elements. Rejected as the
  default pattern because it can bypass canonical chrome. Slots may carry
  behavior or a canonical composite, not a hand-rolled replacement.

## Consequences

- Existing hand-rolled playlist panels should migrate to
  `AddToPlaylistPopover`.
- Architecture tests reject new raw playlist popover panel growth in screen
  modules and hard-ban the Library release-detail regression patterns.
- Task packets touching UI/backend boundaries include explicit boundary rules
  and visual smoke expectations.
- Design regressions like full-width popovers should be treated as boundary
  failures, not merely cosmetic bugs.
