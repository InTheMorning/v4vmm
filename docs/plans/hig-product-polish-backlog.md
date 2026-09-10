# HIG Product Polish Backlog

## Status

Active backlog - 2026-05-18.

## Purpose

Keep Apple HIG product-completeness work visible without reopening the
strategic UI restructuring work that has already landed.

ADR 0060 binds the workflow surface structure: `Music`, `Show`, and `Settings`,
and `Show` as a screen mount. A HIG polish packet that touches those surfaces
follows ADR 0060.

`docs/plans/curator-workflow-ui-design-brief.md` holds the product intent behind
that ADR. It is advisory context, not a binding rule.

The HIG review does not change the structural verdict: toolbar search,
sidebar/source list, split-view layout, breadcrumb/path-bar chrome, SF Symbol
usage, one global search entry, one visible content pane for search, and
content-unavailable empty states remain the right structure.

## HIG References

- `components/toolbars.md` - toolbar search, back/close controls, item
  grouping, customization, and menu-command availability.
- `components/sidebars.md` - sidebar disclosure, familiar symbols,
  show/hide affordances, customization, and Liquid Glass sidebar layer.
- `components/split-views.md` - pane resizing, hide/reveal affordances, and
  multiple reveal paths through toolbar buttons, menu commands, and shortcuts.
- `patterns/searching.md` - one searchable location, recent searches,
  suggestions, search scope clarity, and privacy controls.
- `summaries/navigation-complete.md` - breadcrumbs/path orientation, toolbar
  search, recent searches, suggestions, and scoped options.
- `summaries/liquid-glass.md` - macOS 26+ material treatment for toolbars,
  sidebars, search fields, menus, and popovers.
- `inputs/keyboards.md` - standard shortcut behavior, Command as the primary
  custom modifier, and avoiding conflicts with system-standard shortcuts.

## Backlog Tracks

### Track A - Tactical Structural Mop-Ups

Items 1-6 remain tactical structural mop-ups owned by the existing ADR/task
artifacts that introduced them. They are not a strategic redesign request.

Rules:

- Do not reopen the completed ADR 0047/0048 search restructuring to complete
  these mop-ups.
- Keep each mop-up bounded to its existing owner: view model, primitive,
  composite, frame shell, or architecture guard.
- Use the UI ownership gate in `AGENTS.md` and
  `docs/architecture/ui-backend-boundary.md`.
- Preserve the existing toolbar/sidebar/split-view/breadcrumb/search
  structure unless a new ADR explicitly changes it.

#### A7 - The Feed Check Result Has No Room To Read

Closed - 2026-09-10. Implementation, mechanical checks, and operator visual
acceptance passed in [Show action feedback task 001](../tasks/show-action-feedback-task-001-command-state-and-result.md#operator-visual-check).

The operator reported unreadable counts on 2026-09-09 after ADR 0065 expanded
the result. The situational guard
`adr_0065_feed_check_result_has_its_own_full_width_row` now covers its placement;
`feed_check_route_repair_status_reports_counts_separately` covers the wording.
The operator confirmed full readability at the smallest sidebar width.

#### A8 - Service Actions Can Briefly Repaint The Previous State

Closed - 2026-09-10. Implementation, mechanical checks, and operator visual
acceptance passed in [Show action feedback task 001](../tasks/show-action-feedback-task-001-command-state-and-result.md#operator-visual-check).

The operator reported stale service-state flashes on 2026-09-09. The packet's
named `show_command_*` tests and
`adr_0059_show_command_feedback_survives_all_reprojections` cover command
ownership, read-start freshness, failures, and bounded transition release
(situational, ADR 0059). The operator confirmed feedback during normal, failed,
and disagreeing readback.

#### A9 - Stream Command Buttons Briefly Disappear

Closed - 2026-09-10. Implementation, mechanical checks, and operator visual
acceptance passed in [Show action feedback task 001](../tasks/show-action-feedback-task-001-command-state-and-result.md#operator-visual-check).

The operator reported disappearing controls on 2026-09-09.
`show_command_stream_keeps_controls_and_uses_the_service_release_policy` and
`adr_0059_stream_working_retains_typed_actions` cover the typed action row
(situational, ADR 0059). The operator confirmed stable control placement and row
height during Connect/Disconnect (ADR 0063).

#### A10 - Narrow Show Card Titles Clip Abruptly

Open - 2026-09-10. Deferred for future work at the operator's request during
[task 017 visual inspection](../tasks/adr-0059-task-017-compact-event-controls-and-badges.md#operator-visual-check).
In a narrow window with the detail panel open, the Live Metadata card title
cuts off mid-word beside its state badge. The three detail items and their
badges remain readable together. The clipping cause has not been established.

Owner: ADR 0063's shared Show card composite and dashboard width allocation.
Investigate the title/badge layout in that shared owner. Consult the
[column text truncation investigation](../troubleshooting/column-text-truncation.md)
before choosing an overflow treatment.

Visual acceptance for a future fix: inspect all three card titles and state
badges at normal and minimum supported widths, with the detail panel open and
closed. Titles must remain recognizable, badges readable, and the existing
common card height and two-line summary contract preserved. This deferred item
does not add a gate to task 017.

#### A11 - Long Log Lines Are Hard To Inspect

Open - 2026-09-10. Deferred for future work at the operator's request during
[task 017 visual inspection](../tasks/adr-0059-task-017-compact-event-controls-and-badges.md#operator-visual-check).
The Event Logs screenshot shows long token paths and HTTP error lines extending
beyond the visible pane. Reading the full message needs a clearer interaction.
The screenshot alone does not establish lost text or broken horizontal scrolling.

Owner: ADR 0063's shared bottom log pane and selectable-text presentation.
Review long-line readability and horizontal navigation across Event, Producer,
and Publisher logs. Any display treatment must preserve the original text for
selection and copying, including full paths, IDs, and feed tags.

Visual acceptance for a future fix: reach the start, middle, and end of a long
error line at narrow and normal widths. Verify Ctrl+C and right-click Copy
preserve exact text across the visible edge, with pane resizing and transport
controls still usable. The presentation choice belongs in a future bounded
packet; this deferred item does not add a gate to task 017.

#### A12 - Follow Latest Logs And Remember Each Reading Position

Open - 2026-09-10. Operator requirements captured during
[task 017 visual inspection](../tasks/adr-0059-task-017-compact-event-controls-and-badges.md#operator-visual-check).
This is follow-up work; the existing log-source check passed. Record the
viewport behavior in an ADR 0063 amendment and a bounded packet before
implementation. It does not add an acceptance gate to task 017.

Required behavior:

- First opening a log shows its latest entries at the bottom and follows new
  entries. A log at the bottom keeps following until the operator scrolls up.
- Scrolling up pauses following for that log. New entries must not pull the
  operator away from the text being read. Scrolling back to the bottom resumes
  following.
- An obvious down-arrow control returns to the latest entry and resumes
  following. Reserve space for it outside the text viewport; it must not cover
  any log text. Give it a clear label such as Go to latest and keyboard access.
- Remember each log's reading position and follow state independently when
  switching sources or closing/reopening the pane. Scope identity to the event
  and endpoint, or service unit and host/instance, rather than a shared role or
  displayed title. Persistence across app restarts needs an explicit decision
  in the future packet.
- A subtle bottom gradient is an optional cue for content below the viewport.
  It supplements the arrow and must preserve text readability and selection.

Owners to inspect: `ShowLogPaneDisplay` and the app adapter for source identity,
follow state, and command intent; the shared `ShowLogPane`/selectable-text owner
for scroll position and reserved control space; named tokens for the arrow and
optional gradient. Renderer handles stay outside the view model.

The packet must also define how visible logs receive fresh entries. Current
`ServiceControl::logs` reads a finite journal snapshot. Event Logs remains a
configuration snapshot plus the latest result per action under ADR 0063;
viewport following must not silently turn it into a persistent history. Define
how to retain a reading anchor when rows change or history is trimmed, and how
to report when that anchor is no longer available.

Mechanical acceptance: source-keyed state tests cover initial following,
manual pause, updates while paused, return to latest, independent source
restoration, and replaced/trimmed content. Visual acceptance: use enough live
entries to overflow the pane; inspect follow/pause/resume while switching logs,
closing/reopening, and resizing. Confirm the arrow covers no text, the optional
gradient preserves readability, and keyboard/right-click Copy still returns
the selected text. See also the
[UTC timestamp follow-up](broadcast-chain-delivery-order.md#consistent-utc-log-timestamps).

#### A13 - Narrow Show Layout Leaves No Visible Log Body

Open - 2026-09-10. The operator passed task 017's pane resize/navigation check
and supplied a narrow screenshot where all three cards and detail controls
remain visible, but only the log header fits. Disabled playback controls
consume further space. This is a known limitation, not proof of narrow log
readability.

The [narrow-layout proposal](show-narrow-layout-proposal.md) owns the proposed
automatic/manual compact cards, full-width log docking, overlap alternative,
minimum log-body allocation, and requested playback-bar removal. The scope of
playback removal awaits operator clarification. No fix is implemented or
visually accepted. The proposal records the decision and verification work
needed before implementation; it does not expand task 017's current gate.

### Track B - HIG Product-Completeness Gaps

These are product polish gaps, not restructuring mandates. Implement one item
per bounded task packet.

#### 7. Recent Searches and Search Suggestions

Gap: saved searches exist as a Library/source-list feature, but the global
toolbar search input does not surface recent or suggested searches.

Acceptance direction:

- Keep the global toolbar input as the single search entry.
- Add a GPUI-free suggestion/recent-search projection before rendering UI.
- Distinguish saved searches from recent searches. Do not move Library saved
  searches into toolbar state by accident.
- Provide a clear way to clear recent search history if it is displayed.
- Add unit coverage for suggestion/recent projection and an architecture guard
  preventing a second search entry.
- Capture visual proof for empty, recent-only, suggestion, and active-query
  states.

#### 8. Sidebar Show/Hide and Customization

Gap: macOS HIG recommends sidebar show/hide affordances and customization for
apps with meaningful sidebars.

Acceptance direction:

- Do not hide the sidebar by default.
- Expose show/hide through a stable toolbar/menu command and preserve keyboard
  reachability when available.
- Keep collapsed/sidebar state in the workspace or source-list VM, not in
  screen-local layout conditionals.
- Customization must start bounded: order/visibility of noncritical sidebar
  groups only, with defaults preserved.
- Add guards for reachability, scroll-chain bounds, and no duplicated sidebar
  renderer.
- Capture visual proof for shown, hidden, narrow, and restored states.

#### 9. Liquid Glass Material Adoption

Gap: the current semantic color/token system does not yet model Liquid Glass
materials for toolbar, sidebar, search field, menus, or popovers.

Acceptance direction:

- Route material adoption through tokens/theme/profile/environment roles. Do
  not add raw alpha, blur, color, or material literals in screens.
- Preserve contrast and readable text in light, dark, and high-contrast modes.
- Respect transparency-reduction behavior where GPUI/platform support exists.
  Otherwise document the default behavior explicitly.
- Start with toolbar/sidebar/search-field surfaces before lower-priority cards.
- Add architecture guards that keep material roles out of screen renderers.
- Capture visual proof in light and dark themes.

#### 10. Keyboard Shortcut Coverage

Gap: keyboard coverage should match frequent workflow commands such as search
focus, back navigation, sidebar reveal, and settings.

Acceptance direction:

- Audit desired shortcuts against `inputs/keyboards.md` before binding them.
  Do not repurpose standard shortcuts when the app action does not match.
- Prefer Command-based shortcuts for frequent app commands and descriptive
  command titles.
- Route shortcuts through the app command/keyboard layer, not one-off screen
  key handlers.
- Search focus should support the platform-appropriate Find/Search shortcut
  without creating a second search surface.
- Back navigation should use a platform-safe back equivalent for the frame
  history model, such as Command-[, and only use Command-Left if it is proven
  non-conflicting in the target GPUI/macOS context.
- Add architecture/unit coverage for command routing and smoke coverage for
  search focus and breadcrumb back navigation.

## Non-Goals

- No return to a standalone Search tab.
- No second visible search results pane.
- No screen-local duplication of search suggestions, sidebar chrome, material
  effects, or keyboard handlers.
- No broad visual redesign of music rows, buttons, or layout density.
- No Liquid Glass simulation that weakens contrast or bypasses token roles.
- No curator-facing UI packet that skips ADR 0060 when it affects the surfaces
  that ADR defines.

## Test Strategy

Each HIG product-polish task must include:

- `cargo fmt -- --check`
- `cargo check --quiet`
- targeted unit or architecture tests for the owner it changes
- `cargo test --test architecture_tests --quiet` when architecture guards move
- `cargo clippy --quiet -- -D warnings`
- visual proof or an explicit residual-risk note for visible UI changes

## Routing

- Search suggestions/recent searches: ADR 0043/0048 follow-up task unless the
  projection contract changes enough to require a new ADR.
- Sidebar show/hide/customization: ADR 0046 follow-up task. New ADR only if
  persistence, menu architecture, or source-list customization contracts
  change broadly.
- Liquid Glass: ADR 0025/0034 follow-up, likely ADR-backed if token/material
  roles change.
- Keyboard shortcuts: ADR 0046/app-command follow-up task. ADR-backed only if
  command architecture changes.
