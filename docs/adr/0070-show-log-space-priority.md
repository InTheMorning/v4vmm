# ADR 0070: Show Log Space Priority

## Status

Implemented - 2026-09-13.

Reconciled 2026-09-13: the shared-log packet records mechanical checks Green,
completed operator acceptance, final preservation and confirmed fixture cleanup.

The operator requested this correction during ADR 0063 task 005 acceptance:
an open log must remain readable when the cards form one column, at the
expense of card space, while the sidebar's Logs buttons remain reachable.
It supersedes ADR 0063's requirement to keep every card visible simultaneously
only while a log is open. All other ADR 0063 contracts remain binding.
Implementation and operator acceptance belong to the current
[shared-log packet](../tasks/adr-0063-task-005-shared-log-frames-and-following.md).

Amended 2026-09-13 after the operator accepted the initial allocation retest:
the log header uses a single-line source plus one metadata row. A long service
name must not consume the reclaimed log height through wrapping.

Amended 2026-09-13 after the XL narrow-width inspection: shared log footers stay
on one row. Their renderer-free width class selects a short state and an
icon-only Go to latest control in tight space, retaining full hover descriptions
and the typed keyboard action. Clipped line counts have their own hover text.

Mechanical checks and the operator's allocation, header, XL footer/count-hover
and Show Light/Dark XS–XL checks are accepted on 2026-09-13. The shared-log
packet's V1–V3 visual checks and later configuration-editor disclosure and
Escape checks and final fixture preservation inspection are accepted. The
operator confirmed final fixture cleanup on 2026-09-13, closing the packet.

## Context

The Show log split reserves the full height of every card row before it
allocates the journal. At the single-column breakpoint, three rows consume
the available height and leave only the log header visible. The operator's
2026-09-13 screenshots reproduce this with a service journal open.

ADR 0063 rejected scrolling dashboard status out of sight. The operator now
prioritizes reading an explicitly opened log over seeing every card at once.
ADR 0057 requires a new decision for this reversal.

## Decision

An open log receives its preferred height before the card viewport is sized.
The minimum log budget is 200 unscaled layout units for the source header,
shared log frame, text and following controls. A small card viewport retains
64 units when both budgets fit. If the region is shorter, card space yields
first; the log is bounded by the physical region. Closing the log restores
the existing dashboard allocation.

The card grid scrolls vertically inside its allocated viewport while a log
is open. Card order, equal card heights, labels, badges, selection, and
view-model column count stay unchanged. The existing shared scrollbar gutter
keeps cards clear of the scroll track.

The log stays below the main column. The sidebar keeps its full allocation,
independent scrolling, and existing Event/Producer/Publisher Logs actions.
There is no overlay, automatic sidebar closure, or relocation of those actions.

The renderer-free Show log view model owns height limits and preserves the
user's preferred height when a smaller window temporarily constrains it.
The Show log composite supplies scaled split-handle measurements and composes
the scrolling card viewport. The shared splitter owns measurement and resize
geometry; renderers do not infer a budget from the number of card rows.

The shared Show log header has two text rows: complete source text clipped to
one line, then line count and a brief service state where applicable. Hover
discloses the full source and service details. Source text uses the existing
selectable-text owner so Ctrl+A and keyboard/context-menu Copy retain the
complete identity despite clipping. The view model supplies the brief state;
the renderer does not parse the longer service explanation. Header actions
remain beside the text and cannot be displaced by wrapping source text.

Footer text never wraps vertically. Below 280 unscaled units of measured
viewport width, the shared reading model supplies a compact footer mode: a
short state label and an icon-only Go to latest action. Wider frames retain
the text action and longer state label. Overflow clips inside the state region;
hover retains the complete explanation, including a lost reading anchor.
The control retains its hit target, accessibility label, keyboard activation
and typed availability. The same footer owner serves recovery and Diagnostics.

## Verification And Invariants

- Situational ADR 0070 view-model tests cover the minimum log budget, card-space
  priority at shorter heights, bounded resize, invalid measurements and
  restoration after enlarging the region or closing/reopening logs.
- The existing ADR 0063 guards retain source identity, request ownership,
  exact copying, and sidebar/transport independence. A situational ADR 0070
  guard checks the scrolling-card composition and view-model geometry route.
- The packet's manual regression checks cross the single-column breakpoint
  with a log open, inspect readable text and footer, scroll all three cards,
  reach all sidebar Logs actions, drag the splitter, and repeat with larger
  text and both themes. The operator gate stays open until inspected.

## Alternatives And Consequences

Reserving all card rows is rejected for an open log because it reproduces the
reported loss of log text. Overlaying or shortening the sidebar introduces
unnecessary competition with the actions that select the log source.

Scrolling means some card statuses require an action to see while logs are
open. This is the operator's requested tradeoff. Compact card preferences,
full-width log docking, card-title clipping and inactive transport visibility
remain in the separate narrow-layout proposal; this correction does not
implement those features or reopen prior accepted packets.
