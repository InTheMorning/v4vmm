# ADR 0063 Task 004: Log Output Bottom Pane

Status: Implemented - 2026-09-09. Mechanical and operator visual acceptance met,
including the bottom pane, keyboard selection, and right-click Copy menu.

## Result

Service logs have a resizable bottom pane in the Show main region. The detail
panel keeps its service rows and actions. The transport is outside the split.

`ShowPageVm.log_pane` owns the unit label, text, actual returned line count,
loading/empty/error labels, height, typed close action, and current request ID.
Queue and service reprojections carry this state forward. Each read gets a new
ID; success and failure apply only to that pending request. Card selection and
log operations have independent state.

`SplitPaneAxis::Vertical` extends the existing shared `SplitPane`. Width callers
keep their builder shape, layout, and resize cursor. The pane reserves the card
rows using the existing size and spacing tokens; dragging updates the VM height.
Only the log viewport scrolls, in both directions, with unwrapped lines.

The 2026-09-09 copy/paste addition uses the shared `SelectableText` composite.
Dragging selects across lines; Ctrl+C copies that selection and Ctrl+A selects
all log text. Text stays read-only, and copied whitespace is preserved.
Right-click opens Copy at the pointer without changing the selection. Copy is
disabled when there is no selection. Escape or an outside click dismisses the
menu and returns focus to the text; replacing the text dismisses the old menu.

## Owners And Files

- `src/view_models/show.rs`: pane state, action cycling, request filtering,
  display text, height bounds, and unit tests.
- `src/app/show.rs`, `src/app.rs`: read completion and resize adapters;
  the separate publisher log state is removed.
- `src/ui/composites/show_log_pane.rs`: main-region split, pane chrome, and
  journal viewport. Exported by `src/ui/composites/mod.rs`.
- `src/ui/composites/split_pane.rs`: both split axes and the shared handle.
- `src/ui/composites/selectable_text.rs`: focus, selection highlighting, menu
  wiring, and the one exact clipboard path shared by keyboard and mouse.
- `src/view_models/text_selection.rs`: renderer-free selection ranges, exact
  selected text, and Copy labels and typed availability.
- `src/ui/primitives/context_menu.rs`: pointer anchoring, focus and dismissal,
  with the same menu rows used by existing action menus. Surface, size, and
  spacing use the shared tokens.
- `src/ui/composites/show_detail_panel.rs`: service actions remain; log text,
  log close slot, and obsolete rendering helpers are removed.
- `src/ui/shells/show.rs`: mounts the pane above the existing transport.
- `tests/architecture_tests.rs`: current placement and request guards replace
  the guard that required a log result to open Live Metadata detail.

## Mechanical Acceptance

The following tests are situational guards owned by ADR 0063:

| Check | Evidence |
|---|---|
| Both service actions name their unit; cards and panel closure preserve log text; log closure preserves panel and queue state | `show_logs_name_each_unit_and_survive_card_selection_and_panel_close` |
| Same-service presses close; another service switches; late results do not replace the current service | `show_logs_cycle_same_unit_and_switch_other_unit` |
| Close/reopen of the same unit and reprojection preserve request identity; late errors and duplicate completions are discarded | `show_logs_discard_old_and_duplicate_results_after_close_and_reprojection` |
| A lost host does not disable the action that closes its open logs | `show_logs_can_close_after_the_service_becomes_unreachable` |
| Empty logs and actual line counts are display-ready | `show_logs_empty_read_is_display_ready` and the unit-name test |
| Height bounds reserve the grid and survive closure | `show_log_height_reserves_cards_and_survives_close` |
| Existing width defaults, overrides, pane geometry, and cursor are preserved | `split_pane_width_defaults_and_overrides_are_preserved`, added and passed before vertical mode; `split_pane_width_render_geometry_is_unchanged` |
| Top/bottom geometry uses the shared split | `split_pane_height_render_geometry_uses_the_vertical_axis` |
| Exact multiline and Unicode selection, and Copy available only for a valid nonempty selection | `selection_copies_exact_multiline_text_in_either_drag_direction`, `selection_clamps_offsets_and_copies_partial_unicode_text`, `copy_action_requires_a_valid_selection_in_the_current_text` |
| Keyboard and mouse use one clipboard path; right-click preserves selection; changed text closes the menu; action state comes from the VM and menu chrome stays shared | `adr_0063_log_copy_menu_preserves_selection_and_uses_typed_actions` |
| No log text in detail, no panel changes in log callbacks, no truncation/wrapping, and one shared resize handle | `adr_0063_logs_use_an_independent_bottom_pane_and_current_request`, `adr_0063_column_text_does_not_truncate` |

Verification commands:

```bash
cargo fmt -- --check
cargo check --quiet
cargo test show --lib --quiet
cargo test --lib --quiet
cargo test --test architecture_tests --quiet
cargo clippy --quiet -- -D warnings
```

Copy/paste verification on 2026-09-09: Green. `cargo check`, formatting, strict
Clippy, and `cargo build` passed, along with both focused selection tests and
all 211 architecture guards. The selection tests cover exact multiline copying,
reverse dragging, Unicode boundaries, and offset bounds. The guard
`adr_0063_log_text_is_selectable_and_copied_without_rewriting` covers the mounted
selection owner and plain-text clipboard path. Operator command syntax and
changed-document links were checked without launching the app. The visual gate
also covers selection highlighting, Ctrl+C/Ctrl+A, and read-only behavior.

Right-click Copy verification on 2026-09-09: Green. `cargo test --quiet`
passed all 1,226 unit tests and 212 architecture guards; the 10 existing ignored
doctests remain ignored. `cargo check`, `cargo fmt -- --check`, strict Clippy,
and `cargo build` passed. Operator shell syntax and document links passed.
The operator confirmed the new menu works as intended on 2026-09-09.

## Deviations And Acceptance

No change to broadcast/runtime code, the card contract or width class, the queue
contract, or transport controls. The removed `PublisherLogPanelState` is replaced
by the page-owned pane display. No architectural deviation.

The operator confirmed all bottom-pane, keyboard selection, and right-click
Copy menu visual checks on 2026-09-09. Acceptance is complete in this packet and
the delivery order; its entry is removed from
[Pending Human Checks](../pending-human-checks.md).

## Operator visual check

Operator confirmation: the menu works as intended, 2026-09-09. These steps and
the accepted bottom-pane check below remain available for regression checks.

1. In a Linux desktop session, use the commands in step 1 of
   [Accepted bottom-pane check and fixture](#accepted-bottom-pane-check-and-fixture)
   to build and launch the isolated fixture. It requires a user systemd session,
   Cargo, `sh`, `awk`, and a desktop text editor. No broadcast hardware or
   installed publisher is required.
2. Open **Show → Live Metadata → Logs** and wait for the journal text. Drag-select
   part of a line, right-click the text, and click **Copy**. Paste using the
   editor's mouse menu. The selection must stay highlighted when the menu opens;
   the paste must match it exactly. Repeat with several lines and a reverse drag,
   including spaces, accents, emoji, and literal `<tag> &` text.
3. Scroll horizontally and vertically, then right-click near the window's bottom
   and right edges. Copy must remain reachable inside the window. Click in the
   text without dragging and right-click: Copy must be disabled, and must not
   replace the clipboard with the whole log.
4. With a selection, open the menu and dismiss it first with Escape, then with an
   outside click. The selection must remain, and Ctrl+C must still copy it.
   Leave the menu open through an ordinary service refresh: it must remain usable.
5. Close and reopen producer logs to start the eight-second read. Select loading
   text and open its menu before the result arrives. The result must clear the
   old selection and dismiss the menu. A stale menu, lost selection on ordinary
   repaint, clipped menu, changed log text, or incorrect clipboard is a failure.
6. Quit the fixture app, close the scratch editor without saving, and run the
   cleanup commands in step 6 below. Record this menu result in the packet and
   delivery-order row, then remove its pending-human-check entry when it passes.

## Accepted bottom-pane check and fixture

Operator confirmation: all of these visual checks passed on 2026-09-09.
The fixture remains available for the added menu check above.

1. Use a Linux desktop terminal with a running user systemd session, Rust/Cargo,
   `sh`, and `awk`. Quit any existing v4vmm window. This check supplies synthetic
   journal output and delayed reads; it needs no installed publisher unit,
   encoder, audio hardware, failed service, or SSH host. Create an isolated
   config, database, and journal reader:

   ```bash
   cd /home/citizen/build/v4vmm
   cargo build
   log_check_dir=$(mktemp -d /tmp/v4vmm-0063-004.XXXXXX)
   mkdir -p "$log_check_dir/bin" "$log_check_dir/config/v4vmm"
   cat > "$log_check_dir/config/v4vmm/config.toml" <<EOF
   music_dir = "$log_check_dir/music"
   db_path = "$log_check_dir/library.sqlite"
   musicindex_endpoint = "https://api.musicindex.org"
   EOF
   cat > "$log_check_dir/bin/journalctl" <<'SH'
   #!/bin/sh
   unit=
   while [ "$#" -gt 0 ]; do
       case "$1" in -u) shift; unit=$1 ;; esac
       shift
   done
   case "$unit" in mixxx-now-playing.service) sleep 8 ;; *) sleep 1 ;; esac
   awk -v unit="$unit" 'BEGIN {
       for (i = 1; i <= 50; i++) {
           printf "  %s line %02d café 🦀 <tag> & ", unit, i
           for (j = 0; j < 40; j++) printf "wide-log-segment "
           printf "END-OF-LINE-%02d\n", i
       }
   }'
   SH
   chmod +x "$log_check_dir/bin/journalctl"
   PATH="$log_check_dir/bin:$PATH" \
     XDG_CONFIG_HOME="$log_check_dir/config" \
     XDG_DATA_HOME="$log_check_dir/data" \
     target/debug/v4vmm
   ```

2. Open **Show**, wait for the local service rows, then open **Live Metadata**.
   Press **Logs** for `mixxx-now-playing.service`. Its named bottom pane should
   appear immediately with loading text. Select **Stream** during the eight-second
   read. The pane should stay open, and completion must leave Stream selected.
   Log output appearing in the side panel, or detail selection changing on
   completion, is wrong.

3. In the returned journal, scroll vertically to line 50 and horizontally to
   `END-OF-LINE-50`, using a horizontal trackpad gesture or Shift+mouse wheel.
   Each record must occupy one line. Wrapping, an ellipsis, an inaccessible end
   marker, or an incorrect unit label/count is wrong.

   Drag-select part of one line and then several lines; press Ctrl+C and paste
   into a desktop text editor. Repeat with a reverse drag and Ctrl+A followed by
   Ctrl+C. The paste must retain the selected spaces, blank lines, accents,
   emoji, and literal `<tag> &` text. Selection should remain highlighted through
   an ordinary service refresh. Typing, Backspace, Delete, and Ctrl+V must not
   change the log text. A service switch/result should clear the old selection.
   Wrong characters, lost whitespace, absent highlighting, edits to the log, or
   broken horizontal scrolling count as failures. Close the scratch editor
   without saving after this check.

4. Drag the horizontal handle above the log pane to both height limits. Resize
   the window through its default 1120×760 size, a 900×760 two-column size, and
   a tall 680×1100 one-column size at Medium UI scale. Repeat at your usual UI
   scale and smallest working desktop window. All cards, the pane close control,
   and the transport must remain reachable; the grid must not scroll. Record
   the dimensions and scale if anything clips. Close the log pane and the side
   panel separately; the card grid and transport must remain visible. The empty
   fixture has no playable queue, so disabled playback actions are expected.

5. Return to **Live Metadata**. Open the producer logs and press its **Logs**
   again before eight seconds pass. Wait ten seconds: the pane must remain
   closed. Repeat using the pane's close control. Then open producer logs and
   immediately press publisher **Logs**. The pane must switch to
   `musicindex-live-publisher@mixxx.service`, return its text after one second,
   and retain that unit/text when the old producer read finishes. A same-service
   press after completion must also close it. Reopen producer logs after closing
   a pending producer read; the earlier result must not finish the newer read.

6. Cleanup: quit the fixture app and, in the same terminal, run:

   ```bash
   rm -rf -- "$log_check_dir"
   unset log_check_dir
   ```

   The environment overrides applied only to that app process. No real config,
   library, or service unit was changed. Record the visual result in this packet
   and the delivery-order row; remove its pending-human-check entry only after
   all visual steps pass.
