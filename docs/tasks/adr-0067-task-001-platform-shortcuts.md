# ADR 0067 Task 001: Platform Shortcuts

Status: Implementation recorded - 2026-09-10; mechanical gate Green, including focus correction; operator recheck open.

## Goal And Owners

Make existing app shortcuts reachable with Ctrl on Linux. Read
[ADR 0067](../adr/0067-platform-shortcut-modifiers.md).

- `src/app/keyboard.rs`: shared platform resolver, action binding registry,
  real GPUI keymap tests for both platforms and input contexts.
- `src/app/menu.rs`: Settings/Quit bindings; macOS-only Hide actions.
- `src/app/bootstrap.rs`, `src/app.rs`, `src/app/tab_bar.rs`: initial focus and
  shared section-transition focus; keyboard/menu/capability/live-strip callers
  forward the window to that transition.
- `tests/architecture_tests.rs`: situational ADR 0067 guard for shared modifier
  resolution and unchanged typed dispatch; preserve existing taxonomy guards.
- `docs/runbooks/startup-recovery-check.md`: current Linux instructions.

No config, storage, runtime, playback or view-model availability changes.
Keep text editing and existing active-pane focus guards. Do not run the GUI.

## Acceptance

Mechanical:

1. GPUI keymaps resolve the existing command set to Ctrl on Linux and Command
   on macOS, without duplicate platform bindings or Linux Super aliases.
2. FocusSearch and refresh resolve with an input focused; input Enter/arrows
   remain outside active-pane actions. Representative real input copy/paste,
   cut, selection, undo and word-movement bindings keep priority and identity.
3. Linux menus register only Ctrl+Comma/Ctrl+Q; macOS retains its existing
   Command menu bindings. Ctrl+H is never registered as Linux Hide.
4. Existing unavailable-runtime dispatch guards remain Green.
5. A situational architecture guard verifies explicit focus at normal mount and
   every shared section transition, using the tab bar's existing focus handles.
   Render must not repeatedly reclaim focus. Operator checks below prove actual
   first-attempt delivery and switching away from an input.

Run `cargo fmt -- --check`, `cargo check`, `cargo clippy -- -D warnings`, focused
keyboard/menu tests and `cargo test --test architecture_tests`. Build the debug
binary for the operator. No wider lint cleanup belongs to this packet.

## Operator Visual Check

Open. Use a new isolated startup fixture; the previous accepted fixture has
been removed. A Linux desktop with Ctrl delivered to the app is required.
For the focus recheck, close and relaunch the current shortcut fixture with
the rebuilt binary; do not create another fixture if that one is still available.

1. From the repository root:

   ```bash
   recovery_dir="$(python3 docs/runbooks/startup-recovery-fixture.py setup)"
   python3 docs/runbooks/startup-recovery-fixture.py mode "$recovery_dir" runtime-and-cache-unavailable
   python3 docs/runbooks/startup-recovery-fixture.py run "$recovery_dir"
   ```

2. Before clicking inside the new app, press Ctrl+2/3/1 once each. Show,
   Settings and Music should open immediately. Click a Settings input, then
   repeat Ctrl+2/3/1; switching away from that input must not break subsequent
   shortcuts. Mouse-only activation of the OS window is allowed if the desktop
   did not activate it, but an in-app click must not be required to enable keys.
   Ctrl+Comma should
   open Settings. Ctrl+F should focus the toolbar search, including from a
   Settings input. Type disposable text and try Ctrl+A/C/X/V/Z and Ctrl+Left/
   Right. Editing must work; an app navigation/playback action must not fire.

3. In Music, press Ctrl+R, then Ctrl+Alt+P. Refresh must finish with the
   unavailable-background-tools explanation. Open Settings after the playback
   shortcut; below the settings controls, its result must say playback failed
   because background tools are unavailable. Navigation and repair must stay
   accessible. A permanent loading state or absent command result fails the check. This is the remaining
   ADR 0066 task 003 keyboard check; record actual delivery before closing it.

4. Press Ctrl+Q. Expect exit code 0. Inspect preservation, then clean up:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py inspect "$recovery_dir"
   python3 docs/runbooks/startup-recovery-fixture.py cleanup "$recovery_dir"
   ```

   Config, music and migration records must be preserved; ordinary workspace
   preferences may change. Expect one playlist and no residual probes.

## Evidence And Rollback

First operator attempt: shortcuts initially did nothing, then worked after
repeated attempts. This is not an acceptance pass. Inspection found no explicit
focus at normal mount; GPUI falls back to its window-root dispatch node when
there is no mounted focused control. Focus could also remain on an input removed
by a section change. The correction explicitly focuses the existing selected-tab
handle at both transitions. Whether an additional startup stall occurred is
unconfirmed. The operator subsequently reported a Settings-entry stall; its
[bounded correction and focused recheck](adr-0066-task-003-runtime-failure-and-shell-availability.md#operator-correction-settings-responsiveness)
belong to task 003 under ADR 0040. Retest first-attempt delivery after rebuilding;
do not repeat accepted recovery checks.

Mechanical evidence - 2026-09-10: Green. Formatting, cargo check, strict
production Clippy, nine keyboard tests, four menu tests, all 228 architecture
tests and the debug build passed. All 287 local links checked in the changed
documentation resolved. No GUI was launched.

The new behavioral guards are `adr_0067_platform_shortcuts_route_with_and_without_input_focus`,
`adr_0067_linux_shortcuts_preserve_text_editing`,
`adr_0067_linux_has_no_super_or_duplicate_app_bindings` and
`adr_0067_hide_commands_remain_macos_only`. The situational architecture guard
is `adr_0067_keyboard_and_menu_share_platform_modifier_resolution`.
Existing taxonomy, active-pane input scope, macOS menu and unavailable-runtime
architecture guards passed unchanged.

Focus-correction mechanical verification is Green: cargo check, strict production
Clippy, nine keyboard tests, four menu tests, all 229 architecture tests, formatting
and the rebuilt debug binary passed. The new guard is
`adr_0067_mount_and_section_changes_establish_a_persistent_focus_path`.
Existing section-routing assertions change only to pass `window` into the
same shared transition; their action and destination rules remain.

Task 004's adapter-disposition note was also addressed as documentation only:
C7 requires adapter deletion or an exact justified caller inventory, and removes
the strict startup adapter call and its optional-configuration expect in either
case. Task 004 implementation has not started.

Operator acceptance remains open. Keep this packet, ADR status, delivery row and pending-human-checks
aligned. Mechanical success does not prove desktop key delivery. If rejected,
revert this shortcut change without touching operator configuration or data.
