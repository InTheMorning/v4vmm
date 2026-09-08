# ADR 0063: Show Dashboard Layout

## Status

Accepted - 2026-09-08.

Amends ADR 0060, which made `Show` a screen mount but did not say how the
sections inside it are arranged. ADR 0059 keeps the section set and the section
order. This record covers arrangement only.

## Context

`Show` grew four sections: `Source`, `Live Metadata`, `Event`, and `Stream`.
Each one renders as a full-width strip in a vertical stack, and the queue holds
the lower half of the window.

An operator screenshot on 2026-09-08 showed the result:

- A strip is about 1400 pixels wide and carries about 200 pixels of content.
  The middle of the window is empty in every section.
- The queue divides the window horizontally, does not close, and pushes the
  section stack off the screen.
- The `Live Metadata` section is cut in half, so the operator can not read the
  state of the service that needs attention.

A scroll region was added to the section stack on the same day and reverted on
the same day. A scroll region moves status out of sight. A surface that an
operator watches while a show is live must show its state without an action.

The section stack also changes height as service state changes. A card that
grows and shrinks moves everything under it.

## Decision

### Show Is A Dashboard Of Cards, Not A Stack Of Strips

Each section renders as a card. A card holds a title, a state summary, and
nothing else. Cards lay out in a grid that fills the width of the window.

A card never grows to hold detail. Every card in the grid keeps the same
compact height, so the grid does not reflow when a service changes state.

### The View Model Owns The Column Count

The view model exposes a width class and the column count for that class. The
shell renders the grid with that count and adds no breakpoint logic of its own.

This follows `src/ui/composites/track_metadata_grid.rs`, which already renders
`.grid().grid_cols(self.vm.columns())`. A renderer that is not GPUI reads the
same count.

### Detail Opens In The Side Panel

`Show` has one collapsible panel on the trailing edge. The panel has two modes:

- `Cuelist`, the default. It shows the queue.
- `Detail`, which shows the full content of one selected card.

Selecting a card puts the panel in `Detail` for that card. Closing the detail
returns the panel to `Cuelist`. The panel never shows both at once.

The panel closes. When it is closed, the card grid takes the whole width.
Selecting a card while the panel is closed opens the panel in `Detail`.

The log output of a service is `Detail` content for the `Live Metadata` card. It
is not a strip inside the section any more.

### Transport Stays Outside The Panel

The transport controls do not move into the panel. They stay on `Show` itself,
below the card grid, and they are visible when the panel is closed.

A closed panel must never remove play, pause, or skip from an operator during a
live show.

### Status Does Not Scroll

The card grid does not scroll. Every card is visible at once at every supported
window size. The panel scrolls its own content.

If a future section makes the grid taller than the window, the answer is a more
compact card, not a scroll region.

## Invariants

- A card shows a summary. Detail lives in the panel.
- Every card in the grid has the same height, whatever its state.
- The card grid does not scroll.
- The view model owns the column count. The shell reads it.
- The panel shows the cuelist or one card detail, never both.
- The transport controls stay reachable when the panel is closed.
- The panel closes, and the grid continues to work when it is closed.
- The section set and the section order stay as ADR 0059 states them.

## Alternatives Considered

### Keep The Vertical Stack And Add A Scroll Region

Rejected, and reverted on the day it shipped. It hides the state of a live
broadcast behind a scroll position. It also keeps the unused horizontal space,
because a full-width row is still full width.

### Cards Expand In Place

Rejected. An expanding card reflows the grid around it, so the layout moves
whenever an operator opens detail. The same class of defect closed a gate on
2026-09-08, when an optional line changed the height of a row.

### Detail In A Modal Sheet

Rejected. A sheet covers the dashboard. An operator who opens the publisher log
during a live show loses sight of the stream state at the same moment.

### A Left Rail Of Sections With A Detail Pane

Rejected for now. It shows one section at a time, so it is not a dashboard. It
stays a reasonable answer if the section count grows past what one screen holds.

## Consequences

- `ShowPageVm` gains a width class, a column count, a panel mode, and a selected
  card. The four section fields stay.
- The publisher log panel state moves from the section to the panel.
- `render_show` renders a grid and a panel, not a column of sections.
- The queue keeps its display contract. Only its container changes.
- The transport moves out of the queue container, so it survives a closed
  panel.
- A visual check for this layout is open until a person runs it.

## Follow-Up Work

- A packet for the card contract and the width class in the view model.
- A packet for the card composite and the grid shell.
- A packet for the collapsible panel and its two modes.

## References

- ADR 0059, broadcast control surface, for the section set and order
- ADR 0060, workflow surface structure, for `Show` as a screen mount
- ADR 0061, current-state governance, for the mechanical and visual rule
- `src/ui/composites/track_metadata_grid.rs`, for the column-count precedent
- `docs/pending-human-checks.md`, for the open visual checks
