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

Packet: `docs/tasks/show-action-feedback-task-001-command-state-and-result.md`.

Reported by an operator on 2026-09-09, after ADR 0065 gave `Check all feeds`
more to say.

The result message renders in the left sidebar header, beside the button, in a
`flex_row` that the button already shares. See `src/library/app_impl.rs`, the
`feed_status` child of the feed-update row.

The message now names five counts: feeds checked, feeds stale, tracks repaired,
tracks the publisher must fix, and failures. A narrow column cuts it, so the
operator reads the start of a sentence and guesses the rest.

Owner: ADR 0065, the surface the amendment changed.

Bounded fix, in the order to try:

- give the message its own row under the button, at the sidebar width
- or move it to the content region, where the readiness list it describes lives
- or shorten the message and put the counts in the readiness list header

Do not truncate it. `docs/troubleshooting/column-text-truncation.md` records
what stacked text does with `truncate()`, and a cut count reads as a wrong
count.

Related: `Show` had the same defect in a worse form. Every broadcast command
wrote its result to `settings_status`, which only `Settings` rendered, so a
failed action reported nothing. That is fixed. This is the same lesson at a
smaller scale: **a surface that starts an action reports the result of that
action, in a place with room for it.**

#### A8 - Service Actions Can Briefly Repaint The Previous State

Packet: `docs/tasks/show-action-feedback-task-001-command-state-and-result.md`.

Reported by an operator on 2026-09-09, while closing the ADR 0063 follow-up
visual check.

`Start` and `Stop` answer immediately with the expected transition state, but a
watch refresh can briefly repaint the old service state before the final state
arrives. A stop can flash `Started` before `Stopped`, and a start can flash
`Stopped` before `Started`.

Owner: ADR 0059 and ADR 0063, because this is a `Show` service-state
presentation issue after the dashboard restructuring.

Bounded fix:

- keep the operator-requested transition state authoritative until the command
  completes or fails
- only let the service watch replace it when the reported state agrees with the
  requested direction or the command has left the in-flight state
- add unit coverage for start and stop so a stale watch sample cannot override
  the immediate transition display

#### A9 - Stream Command Buttons Briefly Disappear

Packet: `docs/tasks/show-action-feedback-task-001-command-state-and-result.md`.

Reported by an operator on 2026-09-09, while closing the ADR 0059 task 015
visual check.

The stream encoder connect and disconnect commands work, and the connection
state changes as intended, but both action buttons briefly disappear after
either command is pressed.

Owner: ADR 0059 task 015, because this is the `Show` Stream section action
presentation.

Bounded fix:

- keep the action row mounted while a stream command is in flight
- represent temporary command state through typed action availability or labels,
  not by removing both controls
- add view-model coverage so a stream command in flight still projects an
  action row

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
