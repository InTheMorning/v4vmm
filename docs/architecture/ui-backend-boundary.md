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

## Visual Workflow Ownership Gate

Every UI-facing change must choose the shared owner before code is edited.
Agents must not treat visual polish, button behavior, or workflow fixes as
permission to patch a single renderer when the affordance already belongs to a
view model, primitive, composite, token, or architecture guard.

Default ownership:

- Music presentation facts, fallback labels, availability, empty states,
  summaries, filter state, and command intent belong in GPUI-free view models
  or `src/views.rs`.
- Button/action vocabulary, disabled/enabled state, destructive or primary
  roles, accessibility labels, and workflow command availability belong in the
  view-model contract before they are rendered.
- Button visuals, row chrome, hit targets, menus, popovers, disclosure,
  floating surfaces, spacing, and interaction geometry belong in
  `src/ui/primitives` or `src/ui/composites`.
- Raw colors, spacing, radii, font sizes, weights, icons, and status roles
  belong in named tokens, visual roles, or shared components.
- Screens own only composition, image resolution, callbacks, focus/selection
  state, and dispatch into command/application layers.
- Regression guards own the invariant that made the bug possible: architecture
  tests for ownership drift, unit tests for VM projection behavior, and visual
  smoke for presentation/layout.

If a request asks for a small visual tweak, the same gate applies. The
acceptable small change is the smallest change to the correct shared owner, not
the smallest edit to the visible renderer.

## Forbidden Easy Fixes

Do not:

- adjust button color, label, icon, disabled state, or command copy in one
  screen when the button family appears elsewhere;
- add a renderer-local fallback for music titles, artists, publishers,
  metadata rows, empty states, or transport/error text;
- fork row/card/panel spacing or hit targets in a shell to satisfy one
  screenshot;
- bypass an existing primitive/composite because the direct GPUI chain is
  shorter;
- remove a workflow path, filter behavior, inspector empty state, or scroll
  constraint to make a layout fix easier;
- add a new duplicate `render_*` helper instead of extending the shared
  component or proving with an ADR/task why consolidation is not yet possible.

When an existing shared owner is incomplete, extend that owner. When no owner
exists and the affordance will repeat, create a narrow primitive/composite or
view-model contract before wiring the screen.

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
- Does the change touch visual presentation, button behavior, or user workflow
  without naming the shared owner it strengthens?
- Would the same affordance need the same tweak in Library, Search, Discover,
  inspectors, or playlist surfaces?
- Is visual smoke included for layout, row, panel, or popover changes?
- Are source facts preserved and demoted instead of inferred away?
