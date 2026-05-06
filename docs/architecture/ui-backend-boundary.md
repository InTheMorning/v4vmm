# UI Backend Boundary

## Layer Responsibilities

The expected flow is:

```text
db / services / API clients
  -> views and source facts
  -> GPUI-free view models and presentation contracts
  -> screen-owned behavior wiring
  -> shared UI primitives and composites
```

Backend and service code owns data access, network calls, filesystem work,
source-fact preservation, and mutations.

View models own typed projection, surface policy, action availability, labels,
and GPUI-free contract tests.

Screens own event wiring, command dispatch, async spawning, image lookup,
selection state, and callbacks from shared UI components.

UI primitives and composites own chrome: buttons, rows, popovers, floating
surfaces, panels, spacing, radius, typography, tokens, and interaction geometry.

## Popover Rule

If a canonical popover composite exists, screens must use it instead of building
raw `div()` panels.

Screens may provide:

- available choices
- selected target ids
- click handlers
- create handlers when supported

Screens must not provide:

- floating panel width
- popover arrow/chrome
- token selection
- full-width row child panels as popover substitutes
- alternate button styling for the same component family

The Library add-to-playlist regression happened because Library rows bypassed
`AddToPlaylistPopover` and rendered a raw panel below the shared row. The panel
inherited the row section width and became page-wide.

## Review Questions

- Does the view model import GPUI, UI modules, screens, DB, or services?
- Does a shared UI component import services, command handlers, or screen state?
- Is a screen classifying raw metadata that belongs in a projection contract?
- Is a screen hand-rolling a component family that already has a composite?
- Does a behavior slot carry only behavior/assets, or does it override layout?
- Is visual smoke included for layout, row, panel, or popover changes?
- Are source facts preserved and demoted instead of inferred away?
