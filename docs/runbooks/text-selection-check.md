# Shared Text Selection Check

Owner: [ADR 0071 task 001](../tasks/adr-0071-task-001-shared-text-selection.md).
This is the regression procedure; the task records accepted checks and current
gates. Acceptance and cleanup completed on 2026-09-15. Use fresh fixtures for a
new full pass or [ADR 0072 correction retry](#adr-0072-correction-retry).
The completed log packet and its accepted Escape behavior stay closed.

Needs a Linux desktop, a middle mouse button (or desktop middle-button
emulation), Rust 1.97.1 (pinned by the repository), Python 3.11+, the debug binary, and a separate editor that supports
primary selection. Record whether the app and editor use X11, Wayland or
XWayland. On Wayland, primary-selection protocol support is required. An
unavailable protocol leaves cross-application delivery unverified; do not use
Ctrl+C as a substitute. An IME is needed for the composition subcheck.
No audio hardware, player, real service or reachable Index is needed. Fixture
services are isolated stubs. Keep all text edits unsaved.

For xfce4-terminal, use a separate tab for scratch text. Run `cat > /dev/null`
there before receiving test pastes: the terminal displays input while `cat`
consumes it without executing shell commands or saving a file. Use
Ctrl+Shift+C/Ctrl+Shift+V (or Edit → Copy/Paste) for the explicit clipboard and
middle-click for PRIMARY. Select source text from printed output without its
line ending. These two paste routes are described in the
[Xfce terminal usage guide](https://docs.xfce.org/apps/xfce4-terminal/usage).
Leave the tab running during the checks; Ctrl+C ends the scratch reader during
cleanup. A terminal can cover clipboard transfer; use a text editor with
visible-whitespace support for V3's exact CRLF comparison.

## Operator Visual Check

1. Close the normal app. From a desktop terminal, create a fresh log fixture:

   ```bash
   cd /home/citizen/build/v4vmm
   cargo build --locked --offline --quiet
   selection_fixture=$(python3 docs/runbooks/log-frame-fixture.py setup)
   python3 docs/runbooks/log-frame-fixture.py verify "$selection_fixture"
   python3 docs/runbooks/startup-recovery-fixture.py session-release "$selection_fixture"
   python3 docs/runbooks/startup-recovery-fixture.py run "$selection_fixture"
   ```

   Releasing the fixture command allows normal Quit. Keep this terminal and
   variable. Open an unsaved scratch document in the separate editor.

2. **V1 — Settings and toolbar search.** In the scratch editor type
   `CLIPBOARD-KEEP`, select it and press Ctrl+C. Then type and select, without
   copying, `/tmp/café/café/music_dir/👩‍💻.flac`. The second accent uses a combining
   character. This selected path is primary; `CLIPBOARD-KEEP` is the clipboard.
   In Settings → Library, middle-click in the MusicIndex endpoint at an obvious
   position. The exact path must insert there, keeping surrounding text.
   Also middle-click an empty field, after the last character, and on later
   editor lines. Ctrl+Z must undo that paste alone; Ctrl+Y must redo it. Undo again and paste
   different primary text: Ctrl+Y must not restore the abandoned edit. Do not Save or use
   Defaults. Repeat with the flac field and toolbar search (Ctrl+F), without
   submitting a search. Music directory is a current-session readout;
   [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md) routes
   its editing through configuration repair. Check the editable `music_dir`
   value under V2 instead.

   In each field double-click `café` and `music_dir`: exactly that word must be
   selected. Clicking path separators or the dot selects the punctuation.
   Triple-click must select the full value, including spaces and portions
   outside the visible field. Release the button and verify the selection remains.
   Try dragging in both directions. Repeat with a long URL and `中文 👩‍💻`.
   Accented words must stay intact. Compare CJK and joined-emoji selection units
   between logs and inputs: character classes define words, and ADR 0072 keeps
   their endpoints on complete graphemes. Each CJK character and the complete
   joined emoji must be selectable. Copied and pasted Unicode must remain exact.

   Select a field value with dragging, Shift+arrows and Ctrl+A, one at a time,
   then middle-click in the scratch editor. Exact selected text must arrive.
   Ctrl+V in a scratch buffer must still yield `CLIPBOARD-KEEP`. Deliberately
   copy a new sentinel with Ctrl+C; later primary changes must preserve that
   new clipboard value. Middle-click after selecting a range in the destination
   inserts at the clicked position; Ctrl+V replaces the highlighted range.
   Test primary from the scratch editor back into another app field as well.
   Switching focus or collapsing a selection must not clear primary.
   Missing/duplicate text, replacement on middle-click, a changed clipboard,
   or undo removing earlier typing fails V1. Do not save any field changes.

3. **V2 — shared logs and Settings editor.** In a second desktop terminal,
   append the Unicode samples to the exact fixture directory from step 1:

   ```bash
   python3 docs/runbooks/log-frame-fixture.py selection-sample "$selection_fixture"
   ```

   A separate terminal does not inherit the shell variable; set it to the
   directory printed by setup first. Open Show → Live Metadata → Producer Logs,
   then Publisher Logs. Before selecting text, drag the log divider up and down.
   Release, move the pointer onto the toolbar, and leave the app idle for ten
   seconds. Resizing must track the pointer and stop on release without selecting
   log text. Continuing cursor flicker, an unusable app, or sustained use of a
   full CPU core after release fails this migration check. Compare idle CPU
   before and after dragging in the same view. Retain a failed fixture.
   Find the appended `Unicode path:`, `Final emoji:` and
   `Long value:` lines. Compare double-click selection of `中`, `文` and the final
   combined emoji with the input results; record any failure separately.
   Whole-line and drag copying must preserve the emoji's U+200D joiner even
   when the external terminal displays its parts separately.
   Double-click `café`; triple-click an
   entry containing a long path. Paste primary into the scratch editor to
   compare the whole logical entry. Upstream triple-click excludes its LF
   terminator; dragging and Select All preserve selected newlines. Horizontal
   scrolling and window width must not shorten the selected line. Try reverse
   dragging, Ctrl+A, Ctrl+C and right-click Copy; logs must remain read-only.
   With log text focused, Ctrl+A must select the entire log body, including
   rows outside the viewport. Without Copy, middle-click into the terminal
   scratch reader and compare the first and last entries with the log. Lines
   must stay separate and long paths must remain complete. Explicit clipboard
   paste must still return the value copied before Select All.
   To check read-only behavior, triple-click the `Unicode path:` line to focus
   and select it, then type `X` and press Backspace, Delete and Ctrl+V. None
   may insert, remove or replace log text. The original line must remain
   unchanged and the app must remain responsive. Keep fixture edits unsaved.
   For context-menu Copy, press and hold the button briefly before release:
   selection and enabled Copy must remain intact. Release must close the menu
   and return focus to the log. Explicit terminal paste must contain the exact
   selected line, including trailing spaces, rather than the clipboard marker.
   Reopen the menu and press Escape once: it must close without copying,
   retain the selection and keep the log pane open. Explicit terminal paste
   must still return the previous clipboard value. Reopen the menu, press
   Escape, then Ctrl+C without clicking: terminal paste must now return the
   selected line, proving focus returned to the log. A menu that stays open
   fails even if the selection and pane remain intact. Clicking outside can
   clear selection but must preserve both clipboard buffers.
   With a selected older entry, append from a second terminal using the exact
   fixture directory and confirm the selection and primary remain intact.
   Prepare the append command before establishing the selection and clipboard
   marker, so copying the command does not replace either buffer under test:

   ```bash
   read -r selection_append_ready && python3 docs/runbooks/log-frame-fixture.py append "$selection_fixture"
   ```

   The command waits for Enter. While it waits, explicitly copy `café` in the
   log, then triple-click `Unicode path:` without Copy. Return to the waiting
   terminal and press Enter. After the app receives the new entries, the
   selected line must remain highlighted. Without selecting it again,
   terminal middle-click must return that whole line and explicit paste must
   still return `café`. Retain the appended fixture for later checks.

   In Settings → Diagnostics choose Edit configuration, select
   `musicindex_endpoint`, and append unsaved scratch text. Use Test draft and
   paths to obtain a repair report, without Save correction. Repeat word/line
   selection and outgoing primary on that report. Check report context-menu
   Copy through press, hold and release. With the report menu open, Escape
   must dismiss only the menu, retain the selected report text and keep the
   unsaved editor open. Ctrl+C immediately afterward, without clicking, must
   copy the selected report text. With report text focused and selected, try
   typing, Backspace, Delete and Ctrl+V using a different clipboard value.
   After each action, both the report and the open editor's unsaved value must
   remain unchanged. For incoming PRIMARY in the editor, type `PRIMARY-EDITOR`
   in the terminal scratch reader and drag-select it without Copy. Middle-click
   at the endpoint value's end to append it. One Undo must remove only the
   insertion; Redo must restore it once. Undo again to retain the previous
   draft, and verify the explicit clipboard remains unchanged.
   For multiline cases, replace only the editable endpoint draft with this
   temporary sample:

   ```text
   FIRST /tmp/café/café/music_dir
   SECOND 中文 👩‍💻

   LAST omega
   ```

   Keep four logical lines, including the blank third line, with no newline
   after `omega`. After pasting, Ctrl+End must land immediately after `omega`;
   remove a pasted final newline if the caret instead lands on an empty fifth
   line. Do not test or save this sample as configuration. Reset the clipboard
   by copying `validated` from the repair report. Triple-clicking the second
   line must publish exactly `SECOND 中文 👩‍💻` to PRIMARY, without its newline
   or neighboring rows; explicit clipboard paste must remain `validated`.
   Triple-clicking the final line must publish exactly `LAST omega` without
   a newline. Triple-click the blank third line and type `B`: only that row
   becomes `B`, with the surrounding rows and line breaks intact. One Undo
   restores the blank row. The explicit clipboard must remain `validated`.
   Double-click each accented path word (`café`, `café`) and `music_dir` on
   the first line, then `中`, `文` and `👩‍💻` individually on the second.
   Terminal middle-click must reproduce each selected token exactly and
   explicit clipboard paste must remain `validated`. Record the row-ending
   emoji separately from the final-value emoji case in V1l.
   In the terminal scratch reader, type `PASTE-ONE`, press Enter, then type
   `PASTE-TWO` without Enter. Select exactly those two lines without Copy,
   including their internal newline but no final newline. Middle-click at
   the text start of the editor's blank third row, without first left-clicking
   in the editor. The two pasted rows must appear between `SECOND` and `LAST`,
   leaving both Unicode rows unchanged. One Undo must restore the blank third
   row, Redo must restore both inserted rows once, and a final Undo must
   restore the original four-line draft. Explicit clipboard paste in the
   terminal must still return `validated`.
   For soft wrapping, copy the following as one logical line without a final
   newline. Triple-click the editor's `FIRST` line and paste to replace only
   that line:

   ```text
   WRAP-BEGIN /tmp/café/café/music_dir/中文/👩‍💻.flac segment01 segment02 segment03 segment04 segment05 segment06 segment07 segment08 segment09 segment10 segment11 segment12 segment13 segment14 segment15 segment16 segment17 segment18 segment19 segment20 WRAP-END
   ```

   Narrow the app window until this line wraps onto multiple display rows.
   Reset the explicit clipboard by copying `validated` from the repair
   report. Triple-click a wrapped continuation row: selection and terminal
   middle-click must include the entire logical line from `WRAP-BEGIN`
   through `WRAP-END`, with no inserted newline or neighboring logical line.
   Terminal wrapping is only visual; the caret must remain at the end of the
   pasted scratch input. Explicit clipboard paste must remain `validated`.
   Return focus to the draft and Undo once to restore its original first
   line, then restore the previous window width. Keep the draft unsaved.
   With the original four-line draft restored, focus the editor and press
   Ctrl+End, then Enter. Expect a new blank fifth line, with focus remaining
   in the editor and no validation, save or close action. One Undo must
   remove the newline, Redo must restore it once, and a final Undo must
   restore the original draft with no newline after `omega`. Press Ctrl+End,
   then Tab: indentation must be inserted after `omega`, with the caret
   remaining in the editor. One Undo must remove the indentation, Redo must
   restore it once, and a final Undo must restore the original final line.
   The repair report must remain unchanged throughout, and terminal explicit
   clipboard paste must still return `validated`. Keep the draft unsaved.
   Double-click `omega` in the editor and right-click the selected word to
   open its input context menu. Escape must dismiss that menu first while
   leaving the editor open. Without clicking elsewhere, press Escape again:
   the draft must stay open and Close editor must receive focus. Press Enter
   to close the editor once; it must stay closed after key release. Click
   Reopen editor and confirm the selected `musicindex_endpoint` field,
   exact four-line draft and repair report are retained. Explicit clipboard
   paste in the terminal must still return `validated`. Space on the focused
   Close editor button must also close once and retain the draft on reopening.
   For the V2w Enter correction retry after rebuilding, quit without saving
   the scratch draft and relaunch the same fixture with the run command above.
   In Settings → Diagnostics reopen the configuration editor, select
   `musicindex_endpoint`, append `/selection-check`, and use Test draft and
   paths once to recreate a report. Do not save. Repeat the menu/Escape/Close
   sequence, holding Enter briefly before releasing it: the editor must close
   once and remain closed. Reopen must retain that endpoint draft and report.
   Repeat Close/Reopen using Space after Escape focuses Close, then leave the
   draft open and unsaved for the remaining checks.
   Select `music_dir` and repeat word/whole-value selection, outgoing PRIMARY,
   incoming PRIMARY and Undo/Redo on its unsaved draft value. Do not save a
   correction or change the running session's music directory.
   Use the terminal scratch reader (`cat > /dev/null`) and reset the explicit
   clipboard by copying `validated` from the repair report. Double-click the
   final directory name in the editor's path; terminal middle-click must
   reproduce that selected word. Triple-click the path; terminal middle-click
   must reproduce the complete value without quotes or a newline. In the
   terminal, clear the pending line with Ctrl+U, type `-PRIMARY`, and drag-select
   it without Copy. Middle-click directly after the editor path's last
   character: only that suffix must be appended. One Undo must restore the
   original path, Redo must restore the suffix once, and a final Undo must
   restore the original path again. Explicit clipboard paste must remain
   `validated`. Return to `musicindex_endpoint` and confirm its unsaved
   `/selection-check` suffix survived the field changes. Keep the draft
   unsaved; do not use Test draft, Save correction or End session during
   this path check.

   Click Reload file (discard draft) once and wait for a new report entry
   stating that the app loaded the configuration. Select `musicindex_endpoint`
   if reload selected another field: its value must return to the saved
   endpoint without `/selection-check`. Select `music_dir` and confirm its
   original path without `-PRIMARY`. Close and reopen the editor, then select
   the endpoint again; the discarded suffix must not return. A stale draft,
   failure to update the mounted editor, or a save action fails this check.
   Keep the editor and fixture open for the remaining migration checks.
   Preview Light/Dark at normal and narrow widths to inspect
   selection visibility, focus, input sizing, scrollbars, menus and popovers.
   Start in Settings → General with Light and M as unsaved previews. At
   normal width, inspect selection and the right-click menu in the Library
   group's endpoint input, then in Diagnostics' configuration editor and
   its report. In Show → Live Metadata → Producer Logs, select a word and
   a whole line, open the context menu and scroll the log. Selection text,
   focus indication, menu items and scrollbar controls must remain readable
   and reachable, with no stale theme colors, overlap or clipping. Dismiss
   each menu with Escape. Repeat those surfaces at about 560 pixels wide,
   then restore normal width. Retain Light/M for the next appearance check;
   do not save the preview or remove the fixture.
   Switch General to Dark, keeping M, and repeat the same input, editor,
   report and Producer Log checks at normal and approximately 560-pixel
   widths. Watch for unreadable highlights, stale Light-theme colors,
   clipped controls and missing or inaccessible scrollbars. Restore normal
   width and retain Dark/M without saving before checking the other scales.
   With M accepted in both appearances, keep Dark and check XS, S, L and XL
   individually in that order. For each scale, inspect word/line selection,
   right-click menus and Escape dismissal in the Library endpoint input,
   Diagnostics editor/report and Producer Logs. Scroll the logs and reports.
   Repeat at normal and approximately 560-pixel widths. Cropped text,
   highlights displaced from their glyphs, missing focus indicators or
   unreachable controls/scrollbars fail the scale check. Restore normal
   width after each scale and retain that unsaved scale preview until the
   next check. After XL, restore Dark/M before quitting.
   Before preservation inspection, verify explicit log Copy keeps whitespace,
   separately from PRIMARY. In Producer Logs, triple-click the `Long value:`
   sample and press Ctrl+C. In the terminal scratch reader, Ctrl+U, type `[`,
   paste with Ctrl+Shift+V, then type `]` without Enter. Expect the complete
   line, including three spaces after `Long value:` and exactly two spaces
   between `END.flac` and `]`, with no added newline. Drag-select only `END`
   in the terminal and copy it with Ctrl+Shift+C to replace the clipboard.
   Triple-click the app's long sample again, right-click it, and press, hold
   and release Copy. The menu must close and the same bracketed clipboard
   comparison must reproduce the whole line with its trailing spaces. Pasting
   the `END` marker or losing whitespace fails. Keep the fixture open.

4. Quit normally and inspect the first fixture:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py inspect "$selection_fixture"
   ```

   Expect only permitted workspace-preference changes; configuration values,
   three tracks, one playlist, music, bindings and migration records remain
   preserved, with no probes. Record the complete inspection output. Keep a
   failed fixture for diagnosis; also retain a passing fixture while an open
   selection failure still needs it. Cleanup follows the packet's acceptance.

5. **V3 — recovery, line endings and IME.** Create a separate broken-TOML fixture:

   ```bash
   selection_repair=$(python3 docs/runbooks/startup-recovery-fixture.py setup)
   python3 docs/runbooks/startup-recovery-fixture.py verify "$selection_repair"
   python3 docs/runbooks/startup-recovery-fixture.py mode "$selection_repair" repair-toml
   python3 docs/runbooks/startup-recovery-fixture.py run "$selection_repair"
   ```

   Record the exact new directory printed by verify. The app must open
   configuration recovery with a TOML error and an Edit configuration action.
   Keep the first fixture unchanged; only this new directory receives the
   repair-toml mode. Do not save a correction or open a normal app session.
   In the terminal scratch reader, type `RECOVERY-KEEP`, select it and use
   Ctrl+Shift+C to set the explicit clipboard. Show details in recovery if
   the report is collapsed. Double-click `TOML` in the report body; terminal
   Ctrl+U then middle-click must paste exactly `TOML`. Triple-click the report
   line containing `config.toml`; terminal middle-click must paste the whole
   logical line, including the full fixture configuration path, without an
   added newline or neighboring line. Before each terminal paste, clear the
   pending scratch input with Ctrl+U. After each app selection, explicit
   clipboard paste must still return `RECOVERY-KEEP`. Keep recovery open.
   Choose Edit configuration and confirm the whole TOML document is editable.
   Keep its intentional `invalid = [` line and following comment. Focus the
   document and press Ctrl+End to reveal its blank final row. If the document
   fits, shorten the window until it scrolls, then press Ctrl+End again. In the
   terminal scratch reader, Ctrl+U, type `PRIMARY-RECOVERY`, and drag-select it without
   Copy. Middle-click on the document's blank final row, a few character widths
   right of the text start; the token must insert at that row's start without
   replacing existing document text. A no-op after scrolling fails V3c.
   One Undo must remove only that insertion, Redo must restore it once, and
   a final Undo must restore the original broken document. Explicit clipboard
   paste must still return `RECOVERY-KEEP`; editing must not trigger validation,
   saving or a new recovery attempt. Restore the normal window height if changed.
   Keep the editor open and the draft unsaved. For a retry after rebuilding,
   quit without saving and run this same fixture again; reset both terminal
   sentinels after the launch command, then repeat this blank-row check.
   For V3d, paste this single line into the document's blank final row using
   Ctrl+V, leaving the preceding broken TOML intact:

   ```text
   RECOVERY /tmp/café/café/music_dir/中文/👩‍💻.flac
   ```

   Reset the terminal's explicit clipboard to `RECOVERY-KEEP`. Double-click
   `café` in the appended path; terminal Ctrl+U then middle-click must reproduce
   exactly `café`. Triple-click the appended line; terminal Ctrl+U then
   middle-click must reproduce the complete line, including `RECOVERY ` and
   the Unicode path, without a neighboring line or added newline. Explicit
   terminal clipboard paste must still return `RECOVERY-KEEP`. One Undo in
   the document must remove the appended sample and restore the broken
   original. Keep the editor open and unsaved, and retain both fixtures.
   For V3e, append the same sample again at the blank final row with Ctrl+V,
   then reset the terminal's explicit clipboard to `RECOVERY-KEEP`. Select
   only the path by dragging backward from immediately after `.flac` to
   immediately before `/tmp/`; exclude `RECOVERY ` and the line ending.
   Terminal Ctrl+U then middle-click must reproduce exactly
   `/tmp/café/café/music_dir/中文/👩‍💻.flac`. Terminal Ctrl+U then Ctrl+Shift+V
   must still return `RECOVERY-KEEP`. One Undo in the document must remove
   the appended sample. Keep the editor open and unsaved; retain both fixtures.
   For V3f, paste these two lines into the terminal scratch reader and press
   Enter so they remain available above the pending input:

   ```text
   RECOVERY-ONE café
   RECOVERY-TWO 👩‍💻
   ```

   Terminal Ctrl+U, type `RECOVERY-KEEP`, select it and Ctrl+Shift+C to reset
   the explicit clipboard. In the app, focus the document and press Ctrl+End.
   Back in the terminal, drag-select both sample lines from before
   `RECOVERY-ONE` through the final emoji, including their internal newline
   but excluding the final newline; do not Copy. Middle-click the document's
   blank final row. Both lines must insert in order, preserving preceding
   TOML; Ctrl+End must land after the emoji, with no additional blank row.
   One Undo must remove both pasted lines, Redo must restore them once, and
   a final Undo must restore the broken original. Terminal Ctrl+U then
   Ctrl+Shift+V must still return `RECOVERY-KEEP`. Keep the editor open and
   unsaved; retain both fixtures.
   For V3g, focus the document, press Ctrl+End and type `RECOVERY-DRAFT`
   without pressing Enter. Double-click `DRAFT`, then right-click the selected
   word to open the input menu. Escape must dismiss only the menu. Without
   clicking elsewhere, Escape again must focus Close editor while leaving
   the editor open. Briefly hold Enter, then release: the editor must close
   once and stay closed. Reopen editor must retain `RECOVERY-DRAFT`, the
   preceding broken TOML and the recovery report. Focus the document again,
   press Escape to focus Close editor, then Space to close; reopen and check
   the same retained draft and report. Terminal Ctrl+U then Ctrl+Shift+V must
   still return `RECOVERY-KEEP`. To remove the typed marker, focus the document
   and press Ctrl+End, Shift+Home, Backspace; only `RECOVERY-DRAFT` must be
   removed. Keep the editor open and unsaved, and retain both fixtures.
   **V3h — Mousepad CRLF source inspection.** Mousepad handles file line endings
   separately in its [save implementation](https://raw.githubusercontent.com/xfce-mirror/mousepad/master/mousepad/mousepad-file.c).
   Inspect PRIMARY itself before counting this as a CRLF paste. In a new
   desktop terminal tab with a shell prompt (not the scratch `cat` reader), run:

   ```bash
   selection_crlf=$(mktemp /tmp/v4vmm-selection-crlf-XXXXXX.txt)
   printf 'CRLF-ONE café\r\nCRLF-TWO 👩‍💻' > "$selection_crlf"
   printf '%s\n' "$selection_crlf"
   mousepad "$selection_crlf" &
   ```

   Mousepad must show two lines, without a final blank row. The file contains
   one CRLF separator and no final newline. In that terminal, prepare this
   command without pressing Enter:

   ```bash
   xclip -selection primary -out -target UTF8_STRING | python3 -c 'import sys; print(repr(sys.stdin.buffer.read().decode("utf-8")))'
   ```

   Focus Mousepad and Ctrl+A without Copy; return to the prepared terminal and
   press Enter. Report the printed scratch path and complete payload output.
   An internal `\r\n` proves CRLF transfer; `\n` alone records normalization
   by the source before app insertion and does not accept raw CRLF handling.
   Read binary stdin before decoding; a Python text stream can normalize CRLF
   and invalidate this measurement. Keep the shell tab, Mousepad scratch file
   and both app fixtures for the following paste checks. Recovery stays open
   and unsaved. After the CRLF checks, close only this Mousepad scratch document
   without saving, then remove only its file with `rm -- "$selection_crlf"`
   and `unset selection_crlf` in the same terminal; record cleanup separately.
   **V3i — raw CRLF incoming PRIMARY.** If Mousepad supplied LF, use xclip
   only as the external test source to publish the original file bytes. In
   recovery, focus the document and Ctrl+End to show the blank final row.
   In the retained shell tab that owns `selection_crlf`, run:

   ```bash
   printf '%s\n' "$selection_crlf"
   printf '%s' RECOVERY-KEEP | xclip -selection clipboard
   xclip -selection primary -in -target UTF8_STRING "$selection_crlf"
   xclip -selection primary -out -target UTF8_STRING | python3 -c 'import sys; print(repr(sys.stdin.buffer.read().decode("utf-8")))'
   ```

   The payload must print `'CRLF-ONE café\r\nCRLF-TWO 👩\u200d💻'`. Stop and
   report any different payload or command error. Without selecting other text,
   middle-click the app's blank final row. Both lines must insert once, in
   order, without replacing preceding TOML, adding an empty row between them
   or displaying a stray carriage-return character. Ctrl+End must land after
   the emoji without a final blank row. One Undo must remove both pasted lines,
   Redo must restore them once, and a final Undo must restore the broken
   original. In the terminal scratch reader, Ctrl+U then Ctrl+Shift+V must
   still paste `RECOVERY-KEEP`. Report the scratch path and the V3i result.
   Keep recovery open and unsaved, with both fixtures and the scratch file
   retained for the single-line check. Subsequent ordinary selections and
   explicit Copy replace the temporary xclip selection owners.
   The single-line Settings/search CRLF check remains open until returning to
   the separate normal fixture; keep the scratch file for it. Complete the
   recovery IME check and preservation inspection first.
   **V3j — IME cancellation in recovery.** If no IME is configured, report
   `no IME`; record this coverage as unavailable, not passed. Otherwise record
   the input method and language, focus the document's blank final row and
   activate the usual input method. Start composition without committing it.
   While composing text or showing candidates, Escape must go to the IME,
   leaving the document focused and the editor open. Cancel the composition
   using the input method's normal Escape sequence; Close editor must not
   receive focus while composition remains active. Once composition is fully
   cancelled, one additional Escape must focus Close editor without closing
   it. Restore the usual input mode and confirm no composed text remains in
   the document. Terminal Ctrl+U then Ctrl+Shift+V must still paste
   `RECOVERY-KEEP`. Leave recovery open and unsaved and retain all scratch
   files/fixtures until the result is recorded.
   **V3k — recovery preservation.** After the V3j result is recorded, keep the
   broken original unsaved and Quit without Save correction or opening a normal
   app session. The launch terminal must report `Fixture app exit code: 0`.
   Inspect the recovery fixture from the repository terminal:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py repair-inspect "$selection_repair"
   ```

   Expect unchanged configuration bytes, preserved library/music/bindings/
   migrations and no probes or candidates. No backup is required because no
   correction was saved. Report both complete JSON objects and any command
   error. `original_backed_up`, `external_revision_preserved` and
   `normal_workspace_preferences_only` can be false in this unsaved broken-TOML
   case; those fields are not failed preservation checks. Keep both fixtures
   and the Mousepad scratch file until the remaining selection checks finish.
   **V3l — single-line CRLF paste.** After recovery preservation is accepted,
   leave recovery closed and reopen the separate normal fixture:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py run "$selection_fixture"
   ```

   Open toolbar search with Ctrl+F and clear any existing query. Do not submit
   a search. In the retained shell tab that owns `selection_crlf`, run:

   ```bash
   printf '%s' RECOVERY-KEEP | xclip -selection clipboard
   xclip -selection primary -in -target UTF8_STRING "$selection_crlf"
   ```

   Without selecting other text, middle-click the empty search input. Expect
   `CRLF-ONE caféCRLF-TWO 👩‍💻` on one line. The pinned input engine removes
   CR and LF without inserting a space. No search should be submitted. One
   Undo must empty the query, and Redo must restore the entire value once.
   Terminal scratch-reader Ctrl+U then Ctrl+Shift+V must still paste
   `RECOVERY-KEEP`. Keep the query in place for V3m; retain all fixtures and
   the scratch file. This visual check does not prove absence of hidden CR/LF.
   **V3m — exact single-line PRIMARY value.** After the clipboard check, prepare
   the following command in the shell tab without pressing Enter:

   ```bash
   xclip -selection primary -out -target UTF8_STRING | python3 -c 'import sys; print(repr(sys.stdin.buffer.read().decode("utf-8")))'
   ```

   In the app, focus the query and Ctrl+A without Copy. Return to the prepared
   shell and Enter. Expect `'CRLF-ONE caféCRLF-TWO 👩\u200d💻'`, with no `\r`
   or `\n`. Report the complete output. Return to the input and Undo once to
   remove the test paste, leaving the query empty and unsubmitted. Preparing
   the inspection command may replace the explicit clipboard; V3l checks its
   marker before that step. Keep the normal fixture for the V1l failure and
   inspect it again after the remaining checks, before cleanup.
   **V1l — final-emoji comparison.** Copy the complete value `中文 👩‍💻`.
   In toolbar search, Ctrl+A then Ctrl+V to replace the prior query.
   Double-click the middle of the joined emoji and record whether it selects
   nothing, part of the emoji or the complete emoji. Next copy the complete
   value `中文 👩‍💻.`. In toolbar search, Ctrl+A then Ctrl+V to replace the
   whole query again. Double-click the same emoji and record the result
   separately. Paste both complete values; do not type a period into the
   previous selection. The earlier edit-based procedure produced
   `中文 .\u200d💻`, with the woman scalar replaced; the reported text alone
   does not establish which selection or shortcut caused the replacement.
   Do not submit a search. Expect the complete joined emoji to be selected
   in both cases, excluding the trailing period in the second case.
   This comparison narrows the existing failure; it does not accept a fix.
   Leave the scratch query for the next diagnosis step; clear it before final
   normal-fixture inspection and retain both fixtures until acceptance/cleanup.
   **V1l — exact emoji selection with a following period.** Keep the second
   sample, `中文 👩‍💻.`, in toolbar search. Prepare this command in the desktop
   shell without pressing Enter:

   ```bash
   xclip -selection primary -out -target UTF8_STRING | python3 -c 'import sys; print(repr(sys.stdin.buffer.read().decode("utf-8")))'
   ```

   Return to search and press Right to collapse any selection, then double-click
   the emoji, without Select All or Copy.
   Return to the prepared shell and press Enter. Report the complete output.
   The whole joined emoji is `'👩\u200d💻'`; a missing woman, joiner or laptop,
   or included neighboring text, does not meet that expectation. This checks
   the selected text independently of the highlight; it does not close the
   final-emoji failure. Preparing the command can change the explicit clipboard,
   so this is not a clipboard-independence check. Do not submit the search.
   Retain both fixtures and the scratch file; clear the test query before final
   normal-fixture inspection.

   Both V1l failures are captured in the
   [upstream PR preparation record](../reviews/adr-0071-gpui-base-emoji-selection.md),
   with separate test and correction patches. The adopted gpui-base correction
   is pinned by ADR 0072. Retry on the rebuilt binary as described below;
   repeating them on the previous binary does not close either failure.

6. Record V1–V3 results, protocol(s), editor, theme/width, Unicode and path
   comparisons, clipboard sentinels, Undo/Redo, Escape, and both inspections.
   After acceptance, remove only these new fixtures and discard scratch buffers:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py cleanup "$selection_fixture"
   python3 docs/runbooks/startup-recovery-fixture.py cleanup "$selection_repair"
   unset selection_fixture selection_repair
   ```

   Confirm both removal messages. Existing operator fixtures are unrelated.

## ADR 0072 Correction Retry

Run these checks one at a time on X11 with xfce4-terminal and xclip installed.
The app must use gpui-base commit `5463fe4e72fd740b0db08da92003488b32661867`.
No real service or audio hardware is needed. Keep every edit unsaved and do
not submit a search. This is a focused selection follow-up; it does not reopen
the completed log packet or discard earlier accepted checks.

The accepted fixtures were removed. For a future run, close the normal app
and create a fresh fixture from a desktop terminal in the v4vmm root:

```bash
cargo build --locked --offline
selection_fixture=$(python3 docs/runbooks/log-frame-fixture.py setup)
python3 docs/runbooks/log-frame-fixture.py verify "$selection_fixture"
python3 docs/runbooks/log-frame-fixture.py selection-sample "$selection_fixture"
python3 docs/runbooks/startup-recovery-fixture.py session-release "$selection_fixture"
python3 docs/runbooks/startup-recovery-fixture.py run "$selection_fixture"
```

Keep the printed directory and this terminal's variable for inspection and
cleanup. A second terminal does not inherit the variable.

1. **Final emoji, exact PRIMARY.** Open toolbar search with Ctrl+F. Replace
   the complete query using Ctrl+A/Ctrl+V with `中文 👩‍💻`, without punctuation
   or a newline. In another terminal, prepare this command without pressing
   Enter:

   ```bash
   xclip -selection primary -out -target UTF8_STRING | python3 -c 'import sys; print(repr(sys.stdin.buffer.read().decode("utf-8")))'
   ```

   Return to the input, press Right to collapse any selection, then double-click
   the middle of the emoji. Return to the prepared terminal and press Enter.
   Require a highlight over the whole emoji and exact output `'👩\u200d💻'`.
   A missing selection or any partial payload fails, regardless of appearance.
2. **Following period, exact PRIMARY.** Replace the entire query with
   `中文 👩‍💻.` and repeat step 1's inspection. The result must again be exactly
   `'👩\u200d💻'`, excluding the period. Construct this sample by replacing the
   whole value, not by inserting a period into an existing emoji selection.
3. **Other shared inputs and ordinary words.** Repeat both complete-value
   cases in Settings → Library's MusicIndex endpoint and in the unsaved field
   draft under Settings → Diagnostics → Edit configuration. Undo each test
   replacement to restore the preceding value; do not Save or Test the sample.
   Also check `/tmp/café/café/music_dir/中文/👩‍💻.flac`: double-click accented
   words, `music_dir`, each CJK character and the joined emoji; triple-click
   must select the complete logical line/value. Verify exact outgoing PRIMARY.
4. **Clipboard, replacement, Undo/Redo and Escape.** Use the unsaved editor
   sample `中文 👩‍💻`. Reset the external explicit clipboard to `CLIPBOARD-KEEP`
   after inserting the sample and before selecting the emoji. In the terminal
   used for inspection, run:

   ```bash
   printf '%s' 'CLIPBOARD-KEEP' | xclip -selection clipboard
   cat > /dev/null
   ```

   Return to the editor and double-click the emoji without Copy. Terminal
   middle-click must paste the complete emoji. Use Ctrl+U in the terminal to
   clear the pending input line, then Ctrl+Shift+V: explicit paste must yield
   `CLIPBOARD-KEEP`. Keep the scratch reader running for the next check.
   Ctrl+C in the input must then copy the complete emoji; explicit terminal
   paste must contain it intact. Type `.` over the selected emoji: require
   `中文 .`, without a remaining joiner or laptop. One Undo restores the exact
   sample, Redo restores the replacement, and another Undo restores the sample.
   Escape must focus Close without activating it; activating Close and reopening
   the editor must retain the unsaved draft. Restore the previous draft after
   the check. Keep the existing separate middle-click insertion/atomic-undo
   behavior when pasting the complete selected emoji back into an editable field.
5. **Log boundary regression.** In the fixture's Producer Logs, double-click
   the complete emoji on `Final emoji:` and the accented words, `music_dir`
   and CJK characters on `Unicode path:`. Verify exact PRIMARY and explicit
   Copy. Triple-click must retain the entire path line. The pane stays read-only.

After all retries pass, clear the scratch query and restore unsaved field
values, then Quit. Inspect the normal fixture in its original launch terminal:

```bash
python3 docs/runbooks/startup-recovery-fixture.py inspect "$selection_fixture"
```

Require preserved configuration apart from normal workspace preferences,
music/library/bindings/migrations and tool blockers, with no residual probes.
This narrow retry needs no recovery fixture; the original recovery acceptance
is recorded in the task. After inspection passes, remove this run's fixture:

```bash
python3 docs/runbooks/startup-recovery-fixture.py cleanup "$selection_fixture"
unset selection_fixture
```

Stop the terminal scratch reader with Ctrl+C. Close any scratch editor document
without saving and remove only its exact temporary file. Retain failed fixtures
for diagnosis. Record preservation and each removal before closing the new run.

## Resize Responsiveness Regression

This is the manual ADR 0071 guard for the debug layout optimization. A faster
mock layout does not accept the native freeze or cursor-flicker check. Use the
X11 fixture created by the procedure above, xfce4-terminal and a normal debug build. No live broadcast
service or audio hardware is required.

1. Close the previous fixture app, or press Ctrl+C in its launch terminal if
   it remains unresponsive. From the repository, build and reopen the retained
   fixture (the agent may already have completed the build):

   ```bash
   cargo build --locked --offline
   python3 docs/runbooks/startup-recovery-fixture.py run "$selection_fixture"
   ```

2. Open Show → Live Metadata → Producer Logs. At the window size and UI scale
   that failed, move over the divider, then drag it up and down. It must follow
   the pointer without freezing. Release it, move elsewhere in the app, and
   leave the pointer stationary for five seconds. The divider must stop moving,
   the cursor must stop alternating, and sustained near-100% app CPU must end.
   Periodic service polling may cause brief CPU activity. Record pass or fail,
   window size/scale and whether idle CPU drops before proceeding to Copy.

3. Keep edits unsaved and retain the fixture and diagnostic captures for the
   remaining selection checks. Quit normally after testing; on another freeze,
   stop the launch terminal with Ctrl+C. Use the existing fixture inspection
   before final cleanup. The fixture cleanup at the end of V1–V3 removes the
   isolated services, journals and captures; no production settings were changed.

## Resize Freeze Diagnostic

Use this when the native app freezes or keeps alternating cursor shapes after
dragging the log divider. The agent's mock layout test does not reproduce this
failure; collect the native stack before attempting another correction.

1. Keep the existing failed fixture. If the app has stopped, reopen that exact
   fixture from the desktop terminal and reproduce the divider failure once:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py run "$selection_fixture"
   ```

2. While the app is frozen, set `selection_fixture` in a second desktop terminal
   to the exact directory printed by setup, then run the commands below. It needs GDB
   and permission to attach to the process. This machine's Linux ptrace setting
   restricts sibling-process attachment, so the debugger runs with `sudo`.
   GDB briefly pauses the app, records stacks without argument or local-variable
   values, then detaches. It does not save a core dump or download debug files.

   ```bash
   sudo gdb --batch --nx \
     -iex 'set debuginfod enabled off' \
     -iex 'set print frame-arguments none' \
     -iex 'set print entry-values no' \
     -p "$(cat "$selection_fixture/app.pid")" \
     -ex 'set pagination off' \
     -ex 'thread apply all bt 30' \
     -ex detach \
     > "$selection_fixture/resize-stacks.txt" 2>&1
   cat "$selection_fixture/resize-stacks.txt"
   ```

   Share the output. If attachment fails, share that error; do not change system
   ptrace settings. The trace must name the attached process and contain thread
   stacks to count as collected evidence. A successful trace does not accept
   resizing, Copy or fixture preservation.

3. If the stack ends inside nested layout measurements, collect a five-second
   CPU profile while the app is still frozen. This needs Linux `perf` and the
   same attachment permission. Keep the pointer stationary during collection;
   do not rebuild the binary between recording and generating the report.
   The software CPU-clock event does not require a hardware performance counter.

   ```bash
   sudo perf record -e cpu-clock:u -F 99 --call-graph dwarf,32768 \
     -p "$(cat "$selection_fixture/app.pid")" \
     -o "$selection_fixture/resize-perf.data" -- sleep 5
   sudo perf report --stdio --stdio-color never --no-children \
     --call-graph graph,1,caller --percent-limit 1 \
     -i "$selection_fixture/resize-perf.data" \
     > "$selection_fixture/resize-profile.txt"
   cp "$selection_fixture/resize-profile.txt" ./resize-profile.txt
   ```

   Share `resize-profile.txt`, which contains sampled symbols and call chains.
   Keep the raw `resize-perf.data` in the fixture. If recording fails, share
   the error instead of changing kernel settings. A useful profile contains
   nonzero samples and resolved v4vmm/GPUI/Taffy symbols. Record whether CPU
   usage subsided when the pointer stopped. This diagnostic accepts no UI gate.

4. After capture, close the app normally if it responds, or use Ctrl+C in its
   launch terminal. Inspect the retained fixture with the existing `inspect`
   command. Keep the trace with the fixture until diagnosis and acceptance;
   the normal fixture cleanup removes the diagnostic files afterward. After
   diagnosis, remove the copied workspace reports when the operator no longer
   needs them; do not include these generated captures in a commit.
