# ADR 0023 Final Implementation Review

## Reviewed Artifact

ADR 0023 remainder implementation on 2026-04-30:

- `library-token-intent`
- `search-inspector-token`
- final screen color/layout literal audit

## Result

Pass.

## Required Fixes

None.

## Architectural Drift

No broad CommandBus, QueryService, EventBus, or screen-directory split was
introduced. The implementation stayed inside the ADR 0023 boundary:
view-models own pure labels/status classification; screens keep GPUI event
wiring and service dispatch.

## Design-System Review

- Screen-level `rgb(...)` literals are removed from `app.rs`, `library.rs`,
  and `search.rs`.
- Screen-level numeric `px(...)` literals are removed from `app.rs`,
  `library.rs`, and `search.rs`.
- Fixed geometry that remains screen-visible is routed through named
  `theme::layout` or `theme::typography` constants.
- Discover feed detail and Library album detail now share core
  `DetailHeader` / `TrackRow` composites, so the same release has the same
  structural presentation across modes.
- Ghost action buttons now default to accent text instead of on-accent text,
  addressing low-contrast secondary controls on dark surfaces.
- New projection code remains GPUI-free.

## Missing Tests

No missing unit tests for the moved pure projections. The remaining risk is
visual: the layout-constant sweep is mechanically equivalent but should still
be checked in an interactive app run before a release build.

## Merge Recommendation

Mergeable after the documented verification commands are green.
