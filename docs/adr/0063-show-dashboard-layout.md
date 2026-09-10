# ADR 0063: Show Dashboard Layout

## Status

Implemented - 2026-09-10.

Dashboard tasks 001-004 and ADR 0059 task 017 are complete, including compact
per-item badges, Event diagnostics, operator acceptance, and fixture cleanup.
Show action feedback task 001 is complete, including operator acceptance of
control placement and row height.

Reconciled 2026-09-10: the operator confirmed action feedback task 001 tested and
passed. Its visual gate is closed. Task 017's operator acceptance also passed,
with the narrow-window limitation recorded as deferred work in the packet.
The operator confirmed fixture cleanup, closing task 017's final gate and
returning this ADR to Implemented. No operator acceptance check remains open.

Amended 2026-09-10: [Show action feedback task 001](../tasks/show-action-feedback-task-001-command-state-and-result.md)
keeps Stream controls mounted through commands. ADR 0059 owns the command-state
guards; the packet records operator acceptance of stable placement and height.

Amended 2026-09-09: the operator approved a compact Event item, labeled badges
for Event/Producer/Publisher, and Event diagnostics in the shared bottom pane.
The accepted task 016 layout exposed too much configuration and pushed service
controls down the side panel. [Task 017](../tasks/adr-0059-task-017-compact-event-controls-and-badges.md)
owns the acceptance walkthrough. ADR 0059 owns selection, action, and readiness
semantics; this amendment owns their arrangement and disclosure.

Earlier amendments on 2026-09-09 moved the log to a bottom pane, reduced the
grid to three cards after ADR 0059 made Event a row of Live Metadata, and
recorded the column-text rule.

Amends ADR 0060, which made `Show` a screen mount but did not say how the
sections inside it are arranged. ADR 0059 keeps the section set and the section
order. This record covers arrangement only.

Amended 2026-09-10: the operator found Event diagnostics written around internal
state names. Event Logs now requires timestamped, concrete action results and
omits actions nobody requested. Task 017 owns implementation and visual retest.

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

`adr_0063_compact_items_share_badges_and_event_log_disclosure` enforces the
compact item composition, bounded picker, short explanation, and shared badge
owner (situational, ADR 0063). Full paths and diagnostic transcripts belong in
Logs so the service controls stay easy to reach. `event_hint` retains the
remote-host/local-missing-token explanation without claiming that the remote
file is missing.

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

Implemented by `adr_0063_compact_items_share_badges_and_event_log_disclosure`
(situational, ADR 0063). `src/ui/primitives/status_badge.rs` is the single
geometry/token owner for cards and all three items. Its guard holds the four
semantic fill/foreground mappings. State labels accompany every color.
`adr_0063_item_activity_keeps_its_width_and_single_line` guards pending activity
against vertical wrapping in the shared item header (situational, ADR 0063).

[ADR 0059](0059-broadcast-control-surface.md#item-readiness-determines-section-readiness)
owns kind/label projection and aggregate readiness. The existing card-grid
and two-summary-line guards remain binding. Separate Checking activity preserves
the meaning of a confirmed Attached badge.

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

Implemented 2026-09-10. `compact_event_logs_picker_and_diagnostics_are_identity_scoped`
in `src/view_models/show.rs` covers snapshot fields, exact feed text, identity
switching, and rejection of stale journal responses. The shared presentation
guard above retains pane independence and source/title ownership. These are
situational ADR 0063 guards; the existing selection/Copy and splitter guards
continue to apply.

Event Logs is a configuration snapshot plus each operation's latest session
result. Action rows name the subject, verb, and object and explain the outcome.
`event_report_times_subjects_and_idle_rows_are_honest` and
`adr_0063_event_reports_use_recorded_times_and_plain_text` enforce recorded UTC
times, preserved subjects, no idle rows, and no renderer clock (situational,
ADR 0063). `event_report_check_failures_use_typed_response_facts` keeps
technical details after the explanation (situational, ADR 0059).
It is not a persistent audit log or a service journal. A restart restores
registry facts without inventing a history of commands. Keeping diagnostics in
the existing pane reduces clutter and gives long text a readable width. Changing
events updates an open Event pane, while an open service journal keeps its source.

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

Task 017 is built. The [verification inventory](../tasks/adr-0059-task-017-compact-event-controls-and-badges.md#verification)
records the shared-presentation and source-ownership guards. Those guards are
situational, ADR 0063; readiness semantics have separate ADR 0059 tests.

The [task 017 operator visual check](../tasks/adr-0059-task-017-compact-event-controls-and-badges.md#operator-visual-check)
is the passed situational ADR 0063 manual check for compactness, badge readability,
control reach, source titles, resizing, and mouse/keyboard copying. Those
properties require inspection in a desktop session; matching state kinds in a
test is not visual proof. The packet records the narrow-window limitation as
deferred work. The operator confirmed fixture cleanup on 2026-09-10; the gate
is removed from pending human checks and recorded as complete in the delivery
order. Earlier dashboard and task 004 log acceptance remain completed for
their shipped scope.

## Follow-Up Work

- The [narrow-layout proposal](../plans/show-narrow-layout-proposal.md) records
  compact cards, wider logs, and playback-bar scope. The
  [polish backlog](../plans/hig-product-polish-backlog.md) records long-line
  readability and per-log following/reading positions. These remain future
  work and do not reopen task 017's accepted scope.

- Find the root cause of the column truncation defect, and restore the
  ellipsis. `docs/troubleshooting/column-text-truncation.md` holds what is
  known, the three attempts, and the open questions. The mitigation clips text
  and gives no ellipsis.

Dashboard tasks 001-004 and task 017's amendment are complete. The column
truncation investigation remains separate.

## References

- ADR 0059, broadcast control surface, for the section set and order
- ADR 0060, workflow surface structure, for `Show` as a screen mount
- ADR 0061, current-state governance, for the mechanical and visual rule
- `src/ui/composites/track_metadata_grid.rs`, for the column-count precedent
- `docs/pending-human-checks.md`, for the open visual checks
