# ADR 0063: Show Dashboard Layout

## Status

Accepted - 2026-09-09.

Implementation partial: dashboard tasks 001-004 complete; compact per-item
badges and Event diagnostics under ADR 0059 task 017 await implementation and
operator verification.

Amended 2026-09-09: the operator approved a compact Event item, labeled badges
for Event/Producer/Publisher, and Event diagnostics in the shared bottom pane.
The accepted task 016 layout exposed too much configuration and pushed service
controls down the side panel. [Task 017](../tasks/adr-0059-task-017-compact-event-controls-and-badges.md)
owns the new open visual gate. ADR 0059 owns selection, action, and readiness
semantics; this amendment owns their arrangement and disclosure.

Earlier amendments on 2026-09-09 moved the log to a bottom pane, reduced the
grid to three cards after ADR 0059 made Event a row of Live Metadata, and
recorded the column-text rule.

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
- `Detail`, which shows identity, state, and controls for one selected card.
  Verbose event configuration and diagnostics open through Logs in the bottom
  pane; they do not expand the side panel item.

Selecting a card puts the panel in `Detail` for that card. Closing the detail
returns the panel to `Cuelist`. The panel never shows both at once.

The panel closes. When it is closed, the card grid takes the whole width.
Selecting a card while the panel is closed opens the panel in `Detail`.

### Live Metadata Has Three Compact Items

Amended 2026-09-09.

Normal event content has three rows: heading with its state badge, picker, and
actions. A condition that needs explanation may add one short line. The badge
replaces the repeated State heading and state paragraph. Full errors never
expand this item into a diagnostic transcript. Producer and Publisher likewise
place their state badge beside their heading, retaining their unit detail and
controls below it.

```text
Live Metadata

Event                    [Not attached]
[ Sep 9, 20:30 · …a71c              ▾ ]
[ Attach ]   [ Logs ]   [ ⋯ ]

──────────────────────────────────────
Producer                       [Active]
…existing unit detail and controls…

──────────────────────────────────────
Publisher                      [Active]
…existing unit detail and controls…
```

The example describes hierarchy. Named tokens and shared control owners
determine geometry. Long names must not turn into tall paragraphs; the picker
and diagnostics provide the full identity. Column text follows the Column Text
Does Not Truncate decision below.

### Item Badges Share The Card Badge Presentation

Amended 2026-09-09. Put one labeled badge beside each heading: Event, Producer,
and Publisher. It replaces the separate State heading and repeated explanation.
The three items and the card use one shared badge owner for geometry and token
mapping; extract the existing card badge presentation instead of copying it.

The view model supplies a state kind, visible label, and accessibility text.
Map Ok to Success/OnSuccess, Attention to Warning/OnWarning, Failed to
Danger/OnDanger, and Unknown to Info/OnInfo. The state kinds, complete label
mappings, configured-target definition, and card aggregation belong to
[ADR 0059](0059-broadcast-control-surface.md#item-readiness-determines-section-readiness).
The card alone reads Ready; the successful Event item reads Attached and each
running service reads Active. Color always accompanies a readable state label.

Keep the card's shared height and two summary lines. Passive Checking activity
has its own compact text beside the item and its accessibility state. It must
not replace a confirmed Attached badge with an unknown badge or add a card
summary line. ADR 0059 decides when facts expire or a command changes readiness.

### Log Output Is A Bottom Pane

Amended 2026-09-08, after an operator read the log in the panel.

The log placement and shared resize implementation are enforced by
`adr_0063_logs_use_an_independent_bottom_pane_and_current_request` in
`tests/architecture_tests.rs` (situational, ADR 0063).

Amended 2026-09-09: the selectable, read-only plain-text contract is enforced by
`adr_0063_log_text_is_selectable_and_copied_without_rewriting` and the selection
tests in `src/view_models/text_selection.rs` (situational, ADR 0063). The operator check below
covers dragging, Ctrl+C/Ctrl+A, highlighting, and selection across repainting.

The same plain-text selection also supports right-click Copy. The shared
context-menu primitive owns pointer anchoring, dismissal, and menu chrome;
the text-selection view model owns Copy availability and its labels.
`adr_0063_log_copy_menu_preserves_selection_and_uses_typed_actions` enforces
shared copying, selection preservation, and dismissal on changed source text
(situational, ADR 0063).

The panel is narrow, because it holds one card of detail beside the grid. A log
line is long. A narrow column turns every line into a wrapped paragraph, and an
operator who reads a failure reads it one word at a time.

The `show_logs_*` view-model tests enforce pane ownership, unit naming, action
cycling, card independence, and rejection of obsolete log reads. Width and
height geometry tests in `split_pane.rs` protect the shared splitter.

Readability, horizontal scroll reach, drag behavior, and transport visibility
remain operator checks in
[task 004](../tasks/adr-0063-task-004-log-bottom-pane.md#operator-visual-check).
The operator confirmed these checks and the added right-click Copy menu on
2026-09-09. Task 004 has no remaining visual acceptance gate.

### Event Diagnostics Reuses The Bottom Pane

Amended 2026-09-09. Implementation is assigned to task 017 and has not started.

Logs opens the shared bottom pane with an event-specific title and content.
It uses the existing close, resize, selection, and Copy behavior. Opening it
does not expand the event item or cover the service controls in the side panel.
Only one source occupies the bottom pane at a time.

The event content includes:

- A configuration snapshot: full event ID, label, creation time, relay endpoint,
  token file path, exact feed tag, last successful check time, and observed
  publisher association with host/instance context.
- Separate results for registration, liveness checks, publisher configuration
  reads, and publisher changes, including progress and complete safe failure
  details. Token contents never enter the display or clipboard.

This is event diagnostics assembled from registry facts and application command
results, not a service journal. Task 017 retains the latest result
per operation and event for the current app session. Reopening the app restores
the registry snapshot; it does not pretend to restore a historical command log.
A persistent audit log is outside task 017.

While Event diagnostics is open, changing the event updates its title and
snapshot together and clears text selection. Late results stay associated with
their original event, host, instance, and request; they cannot overwrite the
new source. Choosing Producer or Publisher Logs switches to that service's
existing log view. Changing the event does not steal an open service log pane.

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

### Column Text Does Not Truncate

Added 2026-09-09, because a guard cited this record for a rule this record did
not state.

`truncate()` on text stacked in a flex column renders `...` and drops the text.
It shipped on 2026-09-08 and made every card summary line and every panel value
unreadable. A width did not repair it.

Column text uses `overflow_hidden()`. `truncate()` stays where the element has a
definite width: a row item with `flex_1()`, or an element with an explicit
`max_w()` or `w()`.

`docs/troubleshooting/column-text-truncation.md` records what was tried and what
is still unknown. That document explains. This record decides.

## Invariants

- A card shows a summary. Detail lives in the panel.
- Every card in the grid has the same height, whatever its state.
- The card grid does not scroll.
- Text stacked in a column does not call `truncate()`.
- The view model owns the column count. The shell reads it.
- The panel shows the cuelist or one card detail, never both.
- The transport controls stay reachable when the panel is closed.
- The panel closes, and the grid continues to work when it is closed.
- The section set and the section order stay as ADR 0059 states them. It names
  three sections from 2026-09-09, not four.

## Alternatives Considered

### Keep Only The Card Badge

Rejected on 2026-09-09. It requires reading separate state paragraphs to find
which prerequisite failed. Per-item labeled badges expose later failures as
well as the earliest unmet prerequisite named by the card.

### Expand The Event Item Inline

Rejected on 2026-09-09. It restores the vertical growth that pushed Producer
and Publisher out of easy reach. Event diagnostics belongs in the existing
bottom pane.

### Add A Separate Event Diagnostics Pane

Rejected on 2026-09-09. It adds another pane owner, resize interaction, and copy
surface for a task already served by the bottom pane. Reuse the existing pane
with an explicit Event source and current-request ownership.

### Log Output In The Side Panel

Rejected on 2026-09-08, after it shipped. The panel is narrow and a log line is
long, so every line wrapped and the operator read a failure one word at a time.

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

The 2026-09-09 compact-item amendment keeps all three prerequisites easy to
scan while full configuration remains one press away. One source occupies the
bottom pane at a time, so an event snapshot and a service journal cannot be
compared side by side. Source identity and text selection must change together;
a delayed result must not appear under another event or service title. Event
results are session diagnostics, not a newly promised persistent audit log.


- `ShowPageVm` gains a width class, a column count, a panel mode, and a selected
  card. Three section fields stay, and the event display moves inside
  `Live Metadata`.
- The log pane state lives on `ShowPageVm`, independently of the side panel.
- `render_show` renders a grid and a panel, not a column of sections.
- The queue keeps its display contract. Only its container changes.
- The transport moves out of the queue container, so it survives a closed
  panel.
- A visual check for this layout is open until a person runs it.

### The Grid Holds Three Cards

Amended 2026-09-09, when ADR 0059 made `Event` a row of `Live Metadata`.

The grid draws one card for each section, so it holds three. The column count
still comes from the width class, and a width that fits three cards on one row
leaves no gap where the fourth used to be.

`Live Metadata` now carries three rows in its **detail**. Its card summary keeps
the two lines every card holds, because a card that grows reflows the grid,
which this record rejected.

The two lines answer the chain in order. The first names the earliest row that
is not ready, and the second names the state of the section as a whole. A
section with no event says so on the first line, whatever the services report.

## Compact Item Amendment Verification

Task 017 is unimplemented. Its [mechanical criteria](../tasks/adr-0059-task-017-compact-event-controls-and-badges.md#acceptance-criteria)
cover shared badge ownership, source switching, request identity, and retained
selection/Copy contracts. Those presentation guards are situational, ADR 0063;
readiness semantics have separate situational ADR 0059 tests. Update the named
existing assertions in the same implementation commit and replace prose that
new guards enforce with their coverage references under ADR 0061.

The [task 017 operator visual check](../tasks/adr-0059-task-017-compact-event-controls-and-badges.md#operator-visual-check)
is the open situational ADR 0063 manual check for compactness, badge readability,
control reach, source titles, resizing, and mouse/keyboard copying. Those
properties require inspection in a desktop session; matching state kinds in a
test is not visual proof. The gate is listed in pending human checks and the
delivery order. Earlier dashboard and task 004 log acceptance remain completed
for their shipped scope and do not close this new gate.

## Follow-Up Work

- Implement [task 017](../tasks/adr-0059-task-017-compact-event-controls-and-badges.md)
  for compact items, shared badges, and Event diagnostics; obtain operator
  acceptance before marking this amendment implemented.

- Find the root cause of the column truncation defect, and restore the
  ellipsis. `docs/troubleshooting/column-text-truncation.md` holds what is
  known, the three attempts, and the open questions. The mitigation clips text
  and gives no ellipsis.

Dashboard tasks 001-004 are complete. The task 017 amendment above is the new
implementation work; the column truncation investigation remains separate.

## References

- ADR 0059, broadcast control surface, for the section set and order
- ADR 0060, workflow surface structure, for `Show` as a screen mount
- ADR 0061, current-state governance, for the mechanical and visual rule
- `src/ui/composites/track_metadata_grid.rs`, for the column-count precedent
- `docs/pending-human-checks.md`, for the open visual checks
