# Show Narrow Layout And Inactive Playback Bar

## Status

Proposed - 2026-09-10. Requested during ADR 0059 task 017 visual inspection.
No layout or playback behavior has changed. The operator accepted the pane
resize/navigation walkthrough while identifying the narrow-window limitation.

Playback scope resolved by the operator on 2026-09-10: hide the entire bar and
its spacing when every typed action is unavailable; retain working controls.
The layout amendment and implementation packet remain to be written in the
[approved delivery order](broadcast-chain-delivery-order.md#current-delivery-order).
Include A10 card-title clipping in that packet's shared-owner work and visual gate.

On acceptance, move the decisions into the owning ADR amendment or a new ADR
where a decision is reversed, and create a bounded implementation packet.
Archive this proposal with a superseding link and update current-plan links
in the same change. If rejected, delete this proposal and its index links.

## Problem And Evidence

The operator's narrow-window screenshot shows three stacked cards beside Live
Metadata detail. Only the Event log header fits below the cards; no log text
is visible. A disabled playback bar consumes more height below it.

The current owners explain this allocation:

- `ShowCard::render` in `src/ui/composites/show_card.rs` gives each card the
  fixed `Size::MenuCompact` height: 160 unscaled layout units.
- `ShowLogPane::render` in `src/ui/composites/show_log_pane.rs` reserves every
  full-height grid row before allocating logs. `ShowLogPaneDisplay::resize`
  allows its nominal minimum to shrink to the remaining space. A header can
  consume that entire allocation.
- `ShowShell::render` in `src/ui/shells/show.rs` mounts logs only under the
  main card region and always mounts `render_queue_transport`, even when its
  actions are unavailable.

These are layout constraints, not evidence that log text was lost.

## Recommended Layout

### Compact Cards With Automatic Selection And Manual Preference

Offer card density Auto, Compact, and Standard; default to Auto. Auto considers
both available width and height, including the space needed for an open log.
It selects compact cards before the log body is squeezed away. Manual Compact
also serves operators who prefer denser status at a larger window size.

Compact cards retain the three sections, their order, labeled badges, selection
behavior, and the two summary lines. Remove excess vertical spacing and use a
shared compact height appropriate for the width class. All cards in a resolved
layout retain equal height; live status changes must not make them reflow.
Do not solve the problem by shrinking text to unreadable sizes or putting the
status cards in a scroll region.

### Logs Can Span The Dashboard Width

Offer log width Auto, Main column, and Full width; default to Auto. In Full
width mode, the existing log pane docks below both the cards and detail panel.
The upper region still contains the cards and a shorter, independently
scrollable detail panel. This uses the width requested for logs without
covering controls or adding a second log pane.

Auto chooses full width when the main column cannot provide a usable log
viewport and header controls. A manual Full width choice remains available
at wider sizes. Closing logs restores the detail panel's available height.
Moving the same pane must preserve its source, text selection, resize
preference, and request ownership.

### Reserve Space For Actual Log Text

An open log needs a minimum text area in addition to its header and controls.
Propose at least four complete text lines at the selected UI scale, with a
named token for that budget. Fit calculations must include the wrapped header
and any reserved controls from the
[log-following proposal](hig-product-polish-backlog.md#a12---follow-latest-logs-and-remember-each-reading-position).

Manual preferences must not force an unreadable layout. Mark a mode unavailable
when it cannot fit; retain the saved preference and restore it when space
permits. Resolve the complete layout from measured bounds and named budgets
in the view model. Keep breakpoint decisions out of renderers and avoid a
feedback loop between pane measurement and automatic mode changes.

## Playback Bar Scope

The operator chose this scope on 2026-09-10: omit the entire bar and its
spacing when every typed transport action is unavailable. Retain working
controls, including playing or paused built-in playback. Record visibility in
the ADR 0063 amendment and derive it in the view model from `TransportDisplay`.
An active external broadcast does not establish built-in transport availability.

`ShowSlots` wires the existing controls. The chosen scope preserves the
ADR 0060/0063 placement of working transport inside Show and does not retire
the active-playback surface.

## Alternatives And Consequences

- Compact cards alone recover height but leave long log lines in a narrow
  column. Full-width docking addresses the width problem as well.
- A log overlay over the lower side panel is an alternative the operator
  requested for consideration. It could offer automatic/manual activation,
  but would hide detail controls and require correct occlusion, dismissal,
  and focus restoration. Prefer docking. Any overlay that hides dashboard
  status reverses ADR 0063's status-visibility decision and requires a new ADR.
- A manual-only layout leaves the default narrow window unreadable until the
  operator discovers its controls. Prefer Auto with explicit manual choices.
- Removing the disabled bar alone recovers some space but cannot fix three
  full-height rows consuming the log allocation.

Full-width docking reduces the visible detail-panel height while logs are
open. Compact cards reduce whitespace. Both tradeoffs need human inspection
with long labels, increased UI scale, and actual log updates.

## Ownership And Verification

ADR 0063 already directs future space pressure toward compact cards. Correct
the fit/allocation contract there while retaining three visible section badges
and independent logs. ADR 0059 continues to own readiness and command intent.
Record any reversal separately according to ADR 0057; approval of this proposal
is not operator acceptance of its eventual implementation.

Implementation owners: `ShowPageVm`/`ShowLogPaneDisplay` and typed preferences;
`ShowShell`, shared `ShowCard`, `ShowLogPane`, and splitter composition;
`TransportDisplay` for action facts; named density, spacing, and log-body tokens.
Defaults must load older configuration without migration surprises.

Review and update these existing guards by their symbols, preserving every
still-binding rule they enforce:

- `adr_0063_show_card_contract_is_renderer_free_and_column_only`
- `adr_0063_show_card_grid_shell_uses_vm_contract`
- `adr_0063_show_detail_panel_owns_detail_and_transport_stays_on_show`
- `adr_0063_logs_use_an_independent_bottom_pane_and_current_request`
- `show_log_height_reserves_cards_and_survives_close`

Mechanical checks must cover the layout resolver at narrow/wide and short/tall
bounds, scaling, panel/log open states, manual preferences, minimum log-body
allocation, and the agreed typed transport visibility. Preserve
the existing source, selection/Copy, and late-result guards.

Visual checks must demonstrate readable log text at the screenshot's narrow
size, all three section badges visible, reachable detail controls, no overlap,
working resize and mode changes, and the agreed playback-bar behavior. The
future packet owns setup, cleanup, and its new pending-human-checks entry.

[UTC timestamps](broadcast-chain-delivery-order.md#consistent-utc-log-timestamps)
and independent per-log follow/reading positions remain their linked follow-ups;
this proposal does not implement or silently redefine them.
