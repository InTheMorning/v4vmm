# ADR 0074: Repair And Diagnostics Pages

## Status

Implemented - 2026-09-18.

The operator requested pages, smaller consistent text, predictable scrolling and
separate action controls during ADR 0066 task 013 acceptance. This correction
belongs to that packet. Its presentation gate is accepted on 2026-09-18.
Task 013's final preservation inspections are accepted and cleanup of both
fixtures is confirmed. The task packet is complete. ADR 0066 retains task 004's
independent gate.

Implementation and mechanical checks are Green — 2026-09-18. The
[task packet](../tasks/adr-0066-task-013-interrupted-upgrade-repair.md#presentation-correction--adr-0074)
records 1,484 unit tests, 259 guards and the normal desktop build. The
[operator check](../runbooks/startup-recovery-check.md#repair-and-diagnostics-pages--adr-0074)
has passed its visual and preservation steps. Fixture cleanup is confirmed.

The operator accepted the page choices and text size, then rejected the short
window layout on 2026-09-18. A screenshot shows navigation rows consuming most
of a 382-pixel Settings area. The replacement Settings layout is accepted at
short height, normal/narrow widths and XL scale in Light and Dark themes.

The restored Settings title and single view-toggle button are now accepted,
including placement and switching in both directions. Database Check passed
short-window scrolling, text selection and complete report copy. Database task
navigation, retained destination input and report reading position also passed.
Configuration draft retention, Escape focus, Close/Reopen and draft reload
without saving passed. Settings theme/scale checks, menu mouse and keyboard
selection, Escape and restoration of the original preferences also passed.
The earlier recovery walkthrough pass was followed by a normal Settings
screenshot. A later screenshot confirms the unsupported fixture's recovery
window after startup check 1, with Open app disabled. The operator then accepted
explicit Check again: check 2 still reported the unsupported schema and left
Open app disabled. Recovery Startup, Configuration and Database then passed
normal/narrow and short-height layout, scrolling, view switching and report
copy. The operator confirmed that the repair action is absent.

The operator found an empty Background tools page during the theme and scale
check. The page reused the compact notice's failure-only list. The corrected
tool list is now accepted at normal and narrow widths. Instructions and actions
remain reachable, including the final entries. The list remains after switching
to Report and back. All visual checks are accepted.

## Context

Diagnostics puts several complete tools in one scrolling page. Recovery puts
configuration repair and database tools below the startup report. Each report
has a nested vertical scroll view that captures mouse-wheel events. Much of the
instruction text inherits the toolkit's default font size.

## Decision

Settings keeps General, Library and Diagnostics. Diagnostics selects one page:
Database, Configuration, Background tools, App session or Cached files. Recovery
selects Startup, Configuration or Database. Page selection runs no command and
retains inputs, reports, selection and reading positions.

Start every Settings group with the same Settings title. Keep the group
selector and current Diagnostics page selector in that shared header.
Use menus for Settings groups, Diagnostics pages and recovery pages.
Use a menu for database tasks. Place the task selector or tool title in the
content container's toolbar. One button at the right of that toolbar changes
between Instructions and Report. Label the button Show report or Show
instructions to name the view it will open. Keep this button inside the content
container at every width. Do not allocate separate view tabs or a view menu.
Menus show the current selection and retain
keyboard access through the shared popover and button owners.

Recovery keeps Check again, Open app, Copy startup report and Quit on the
Startup page, using the same action column or band. Other recovery pages do not
reserve space for a startup command footer. Startup status and feedback stay on
the Startup page and in its report.

Database has separate Check, Backup, Preserve files, Restore and Repair upgrade
pages. Each page supplies its own instructions, fields and actions. The existing
command owner decides action availability. An unsupported schema never enables
repair.

Each tool separates actions from content. At ordinary widths, actions occupy a
left column. At narrow widths, actions occupy a scrollable band above the
content. Allocate one quarter of the available body height, with enough room
for one complete action button and its padding.
Instructions and Report are separate views. A report fills the content area.
No page scroll view contains a log scroll view. Report selection, copying,
following and reading positions retain the ADR 0063 shared owner. Long report
lines retain horizontal scrolling. Configuration text editing retains its
existing input owner and Escape behavior.

Background tools keeps its setup instructions and existing checks visible when
no failure is recorded. The view model supplies this inventory. Show failures
and retained actions first. Keep the compact notice limited to failures and
retained actions. Report only recorded observations. Showing a tool does not
confirm that its setup or external service works, and runs no check.

Use the existing scaled Body token for instructions and Caption for field
labels and secondary text. Use Headline for tool titles. Reports retain the
existing compact monospace token. Apply ASD-STE100 structural rules to changed
instructions and reports: short active sentences, one instruction per sentence,
consistent terms and explicit subjects. Preserve uncertainty and safety
conditions. This does not claim compliance with the official word dictionary.

This decision supersedes ADR 0063's containing-page scroll behavior for Settings
and recovery logs. It also replaces ADR 0069's single scrolling Diagnostics
group with separate pages. Their other rules and accepted behavior remain.

## Ownership And Verification

- Renderer-free navigation and database page contracts live in view models.
- A shared maintenance-page composite owns columns, viewports and typography.
- Settings, recovery and existing tool presenters compose those owners.
- Navigation tests prove that changing pages cannot execute maintenance work.
- View-model tests keep Background tools populated before and after successful
  checks, without creating observations or adding idle tools to the compact notice.
- Renderer tests check separate action/content bounds at normal and narrow
  widths, including the complete Settings navigation at short heights and
  larger scale. An architecture guard prevents mounting reports inside page scrolling.
- The task 013 operator check covers typography, page navigation, mouse-wheel
  scrolling, complete report copy, retained drafts and existing repair behavior.

## Alternatives And Consequences

More inline headings do not separate workflows or remove competing scroll views.
Changing only log wheel propagation retains the long mixed page and makes report
scrolling depend on its position. Separate views require one deliberate
navigation action to read a report. The app retains the current inputs and
report when the operator returns.

No database, configuration format, migration, playback or service behavior
changes. Task 004 retains its independent gate. ADR 0066 remains Accepted.
