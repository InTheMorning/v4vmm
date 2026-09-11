# ADR 0067: Platform Shortcut Modifiers

## Status

Implemented - 2026-09-11.

[Task 001](../tasks/adr-0067-task-001-platform-shortcuts.md) records mechanical
verification, operator acceptance, preservation inspection and fixture cleanup.
Amended 2026-09-11: completion is verified after the operator confirmed initial
and section-transition focus, standard editing, command rejection and clean Quit.

Amended 2026-09-10 after the first operator attempt: shortcuts worked only after
repeated attempts. The normal app must focus its persistent selected tab at
mount and section changes, so commands have an app dispatch path before any click
and after a previously focused input leaves the screen. The implementation's
missing focus path is established; a startup stall has not been established.

## Context

The app registers GPUI `cmd` shortcuts on every desktop. On Linux those use
Super, which the operator's window manager reserves. Refresh and playback
therefore never reach the app. Requiring a modifier preference would make
ordinary keyboard access depend on additional setup.

## Decision

App shortcuts use Ctrl on Linux and other non-macOS platforms, and Command on
macOS. Preserve the existing command keys and additional Alt modifiers. The
keyboard adapter resolves the platform modifier once from the shared binding
specifications; screens do not translate keys or bypass typed action dispatch.

On Linux this includes Ctrl+F for toolbar search, Ctrl+1/2/3 for Music/Show/Settings,
Ctrl+N for a new playlist, Ctrl+R for library refresh, Ctrl+Alt+P for playback,
Ctrl+Alt+Left/Right for previous/next, Ctrl+Comma for Settings and Ctrl+Q to quit.
Ctrl+Alt+F remains an alternate search-focus shortcut.

Native macOS Hide/Hide Others actions and their Command shortcuts remain macOS
only. Translating them into Ctrl+H on Linux would collide with text editing.
The existing input widget continues to own copy, cut, paste, undo, selection
and word movement. Bare Enter, Escape and arrows retain their active-pane scope
and must not replace input editing or search submission.

This adds no preference, config key, layout change or command availability rule.
ADR 0066's runtime rejection still applies to every shortcut. Its keyboard
checks use Ctrl on Linux and passed with actual desktop key delivery.

## Invariants

1. App-owned Linux shortcuts do not require Super. macOS retains Command.
2. Keyboard and menu bindings use one platform-modifier resolver and dispatch
   the existing typed actions through their existing command boundaries.
3. App registrations leave standard text-editing shortcuts and input Enter/
   Escape/arrows with their existing owners. Hide commands are macOS only.
4. No configuration migration or user modifier selection is required.
5. Startup and section changes establish focus on the persistent selected tab.
   Focus is set at those transitions, never repeatedly during render; ordinary
   input focus and text editing remain usable.

## Alternatives Considered

- **Make the modifier configurable.** Rejected in favor of the operator's
  requested standard Ctrl behavior; it would add setup and persistence solely
  to make ordinary shortcuts accessible.
- **Keep Super and add Ctrl aliases.** Rejected: Super bindings continue to
  conflict with the desktop and create two shortcut conventions in one app.
- **Translate every macOS menu binding to Ctrl.** Rejected because Hide and
  Hide Others are platform actions, and Ctrl+H may edit text on Linux.
- **Change command routing with the modifier.** Rejected: changing key delivery
  does not change a command's availability or dependency requirements.

## Consequences And Verification

Linux operators can use the app without changing window-manager bindings.
Existing Linux muscle memory for Super shortcuts changes. macOS conventions
remain intact; cross-platform keymap tests cover both mappings.

[Task 001](../tasks/adr-0067-task-001-platform-shortcuts.md) owns implementation,
keymap tests, architecture guards and the operator check. It interrupts the
delivery sequence only to resolve the observed ADR 0066 task 003 keyboard gate.
All accepted startup, repair, preservation and layout checks stay accepted.
