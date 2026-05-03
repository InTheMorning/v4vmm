# ADR 0036: Feed, Visual, and Provenance Surface Consistency

## Status

Proposed - 2026-05-02.

## Context

ADR 0035 consolidated track detail and row presentation, but normal feed
viewing, shared visual primitives, and advanced Library provenance panels can
still drift independently. The same feed/release can appear in Library and
Discover with different action grouping, slot typing, panel density, and
visual hierarchy.

This is a Human Interface structure problem. Apple HIG layout, typography,
button, list, and popover guidance expects predictable grouping, clear
hierarchy, compact transient surfaces, and adaptive spacing. In this repo,
that means every repeated feed/track/provenance surface must have one owner,
one display contract, one token vocabulary, and one regression guard.

## Decision

Complete three ordered passes:

1. **Feed surface consolidation.** Feed/release detail surfaces must expose
   typed behavior slots and one shell path for Library and Discover. Screens
   may provide command handlers, resolved artwork, and screen-specific panels,
   but they may not rebuild header, summary, track-section, or action chrome.
2. **Visual system enforcement.** Shared spacing, typography, radius, icon,
   button, row, and popover sizing must route through named tokens and shared
   primitives. Pixel tuning without a token/primitive owner is forbidden.
3. **Advanced provenance panel consistency.** Library-only compare,
   MusicBrainz, staged-tag, and provenance panels may stay denser than normal
   detail views, but they need a shared panel grammar and typed display
   contracts so they do not feel like a second app.

## Invariants

- A fix is valid only if it creates or strengthens a shared composite, a
  GPUI-free view-model/display contract, a token role, or an architecture test.
- Feed/release surfaces do not expose free-form `AnyElement` slots from shared
  composites when a typed surface element can express the same boundary.
- Library and Discover feed detail call the same shell and consume the same
  `ReleaseDetailPageVm` contract.
- Visual changes happen in primitives, composites, or tokens, not screen-local
  compensation.
- Advanced provenance panels may contain source-specific labels, but their
  layout grammar and repeated labels belong in panel contracts/composites.
- Visual smoke is user-provided screenshots, not pointer automation.

## Non-Goals

- No backend, schema, API, RSS, ID3, or metadata inference changes.
- No SwiftUI/AppKit port.
- No broad redesign of Library or Discover navigation.
- No new playlist/playback feature work in this ADR.

## Alternatives Considered

- Tune the visible jank directly in screens. Rejected because that is the path
  that caused the current drift.
- Combine all three passes into one large diff. Rejected because repo
  governance requires one verified phase at a time.
- Make advanced Library panels visually identical to normal detail views.
  Rejected because provenance comparison is intentionally denser; only its
  repeated grammar needs consolidation.

## Consequences

- Some existing `AnyElement` compatibility slots will become typed wrappers.
- Architecture tests become stricter, reducing room for quick screen-local UI
  patches.
- Richer playlist and playback features should wait until the feed and visual
  system passes are green.

## Enforcing Tests

- `release_surface_slots_are_typed` blocks free-form shared release surface
  slot APIs.
- `release_surface_consumers_use_release_detail_vm` blocks Library and
  Discover feed detail routes that bypass `ReleaseDetailVm`.
- `playlist_popover_menu_rows_use_leading_alignment_and_token_padding` blocks
  centered menu-row regressions and undersized popover surface padding.
- `release_detail_surface_uses_scale_aware_spacing_tokens` blocks release
  detail surface spacing from regressing to fixed legacy style constants.
- ADR 0033/0034/0035 tests continue to block duplicate rows, popovers,
  fallback labels, raw visual literals, and track-surface forks.
