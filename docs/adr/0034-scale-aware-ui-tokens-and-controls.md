# ADR 0034: Scale-Aware UI Tokens and Controls

## Status

Proposed - 2026-05-02.

## Context

The app has a persisted `ui_scale` setting and a token system with
`Spacing::scaled(cx)`, `Radius::scaled(cx)`, `FontSize::scaled(cx)`, and
`Size::scaled(cx)`. Several shared primitives and composites still render
user-facing dimensions with base `.px()` values, which means the size setting
does not consistently affect padding, text, row affordances, icon size, or
popover dimensions.

This is a Human Interface structure problem, not only a playlist-popover
cosmetic problem. Apple HIG guidance for accessibility, layout, and popovers
expects interfaces to adapt to text-size and context changes while preserving
clear grouping, readable controls, and compact transient surfaces. A size
setting that changes some text while leaving surrounding hit targets and
spacing fixed can make dense controls feel cramped, visually inconsistent, or
harder to use.

ADR 0033 already requires UI code to use named tokens instead of ad hoc
literals. ADR 0034 tightens that rule: shared UI must use the scale-aware
token accessors for user-facing dimensions.

## Decision

Shared UI primitives and composites must resolve user-facing spacing,
typography, radius, icon sizes, control sizes, and semantic container sizes
through the active `ScaleFactor` whenever an `App` context is available.

In practice:

- `.scaled(cx)` is the default accessor for `Spacing`, `Radius`, `FontSize`,
  `Size`, and icon-size roles inside `RenderOnce` implementations.
- `.px()` remains available for token definitions, tests, baseline
  comparisons, low-level constants without an `App` context, hairlines, and
  documented fixed media/artwork dimensions.
- Shared primitives own scale behavior. Screens and domain composites should
  not compensate locally for a primitive that failed to scale.
- Popovers remain compact, but their padding, inner gaps, button affordances,
  text, icons, and menu dimensions must change coherently with `ui_scale`.
- Architecture tests must fail when new shared UI render code introduces
  unscaled token `.px()` calls for user-facing dimensions without an explicit
  allowlist entry and reason.

## Invariants

- `ui_scale` affects all shared primitives that render text, controls,
  spacing, radius, and popover/menu dimensions.
- Shared UI render methods do not call token `.px()` for user-facing layout
  unless the call is allowlisted with a specific reason.
- Screens do not patch scale behavior locally.
- Playlist popovers use `AddToPlaylistPopover` and the canonical popover
  primitive; no screen-local popover layout is reintroduced.
- Dense desktop layout is still allowed, but density must be scale-aware and
  legible at every named size step.
- Visual smoke for any affected surface is user-provided screenshot evidence,
  not automated pointer wrestling.

## Non-Goals

- No visual redesign.
- No rewrite to SwiftUI/AppKit.
- No change to backend, schema, playlist semantics, playback semantics, or API
  response shapes.
- No guarantee that every legacy screen-local layout becomes scale-perfect in
  this ADR. This ADR fixes the shared primitive/composite layer first.

## Alternatives Considered

- Tune playlist popover padding only. Rejected because the same unscaled-token
  issue exists in `Surface`, `Button`, `Label`, `MultilineText`, and icons; a
  local popover patch would leave the system structurally inconsistent.
- Raise default base token sizes. Rejected because it would change medium-scale
  density without making the UI adaptive.
- Let screens opt into scaled behavior manually. Rejected because screens are
  composition glue under ADR 0033; scale policy belongs in primitives and
  shared composites.

## Consequences

- Shared primitives may visually change at non-medium scale settings.
- Existing tests that assert raw pixel values in rendered shared UI may need to
  assert token roles or named scale outcomes instead.
- Architecture tests will need a narrow allowlist for legitimate `.px()` uses
  in shared UI.
- Future UI feature work should be delayed if it depends on popovers, buttons,
  labels, or surface spacing that still bypasses the active scale.
