# Shared Text Selection Check

Owner: [ADR 0071 task 001](../tasks/adr-0071-task-001-shared-text-selection.md).
V1–V3, preservation and cleanup are open. Use fresh fixtures; the completed
log packet and its accepted Escape behavior stay closed.

Needs a Linux desktop, a middle mouse button (or desktop middle-button
emulation), Rust 1.97.1 (pinned by the repository), Python 3.11+, the debug binary, and a separate editor that supports
primary selection. Record whether the app and editor use X11, Wayland or
XWayland. On Wayland, primary-selection protocol support is required. An
unavailable protocol leaves cross-application delivery unverified; do not use
Ctrl+C as a substitute. An IME is needed for the composition subcheck.
No audio hardware, player, real service or reachable Index is needed. Fixture
services are isolated stubs. Keep all text edits unsaved.

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
   Defaults. Repeat with the music-directory and flac fields and toolbar search
   (Ctrl+F), without submitting a search.

   In each field double-click `café` and `music_dir`: exactly that word must be
   selected. Clicking path separators or the dot selects the punctuation.
   Triple-click must select the full value, including spaces and portions
   outside the visible field. Release the button and verify the selection remains.
   Try dragging in both directions. Repeat with a long URL and `中文 👩‍💻`.
   Accented words must stay intact. Compare CJK and joined-emoji selection units
   between logs and inputs: 0.6.1 uses character classes, so a joined emoji is
   not necessarily one word. Copied and pasted Unicode must remain exact.

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

3. **V2 — shared logs and Settings editor.** Open Show → Live Metadata →
   Producer Logs, then Publisher Logs. Double-click `café`; triple-click an
   entry containing a long path. Paste primary into the scratch editor to
   compare the whole logical entry. Upstream triple-click excludes its LF
   terminator; dragging and Select All preserve selected newlines. Horizontal
   scrolling and window width must not shorten the selected line. Try reverse
   dragging, Ctrl+A, Ctrl+C and right-click Copy; logs must remain read-only.
   With a selected older entry, append from a second terminal using the exact
   fixture directory and confirm the selection and primary remain intact:

   ```bash
   python3 docs/runbooks/log-frame-fixture.py append "$selection_fixture"
   ```

   In Settings → Diagnostics choose Edit configuration, select
   `musicindex_endpoint`, and append unsaved scratch text. Use Test draft and
   paths to obtain a repair report, without Save correction. Repeat word/line
   selection and outgoing primary on that report. In the editor, test
   double/triple-click and incoming primary with two lines separated by Enter,
   a blank line and a final line without a terminator. A soft wrap must not
   limit triple-click; only the logical line is selected. Verify Undo/Redo and
   clipboard independence again. A single-line field inside this multiline
   editor may select its full value because it has no line break.

   With an input context menu open, Escape must dismiss that menu first. The
   next Escape must leave the draft open and focus Close editor, preserving
   the draft, selected field and repair report. Enter or Space on Close closes
   it once. Reopen editor restores the draft. Enter and Tab while editing
   remain editing keys. Reload file (discard draft) explicitly restores the
   original. Preview Light/Dark at normal and narrow widths to inspect
   selection visibility, focus, input sizing, scrollbars, menus and popovers.
   Check all five UI scales, including the configuration editor. Missing chrome,
   clipped controls or colors from the wrong theme fail V2. Restore Dark/M
   before quitting.

4. Quit normally and inspect the first fixture:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py inspect "$selection_fixture"
   ```

   Expect only permitted workspace-preference changes; configuration values,
   three tracks, one playlist, music, bindings and migration records remain
   preserved, with no probes. Keep a failed fixture for diagnosis.

5. **V3 — recovery, line endings and IME.** Create a separate broken-TOML fixture:

   ```bash
   selection_repair=$(python3 docs/runbooks/startup-recovery-fixture.py setup)
   python3 docs/runbooks/startup-recovery-fixture.py mode "$selection_repair" repair-toml
   python3 docs/runbooks/startup-recovery-fixture.py run "$selection_repair"
   ```

   Show details and repeat outgoing log word/line primary selection. Choose
   Edit configuration. In the multiline document repeat incoming primary,
   Unicode, blank/final logical lines, reverse drag and undo. Copy/select a
   two-line CRLF sample in the external editor and primary-paste it here:
   both lines must survive. In a single-line Settings/search field, CR/LF
   must not create hidden line breaks. Compare using the external editor's
   visible-whitespace mode. Check the same Escape/menu/Close/Reopen sequence
   as V2. If an IME is installed, compose and cancel text with Escape before
   checking normal editor Escape; composition handling must happen first.
   Record an unavailable IME subcheck separately. Keep the broken original
   unsaved; Quit without Save correction or opening a normal app session.

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py repair-inspect "$selection_repair"
   ```

   Expect unchanged configuration bytes, preserved library/music/bindings/
   migrations and no probes or candidates. No backup is required because no
   correction was saved.

6. Record V1–V3 results, protocol(s), editor, theme/width, Unicode and path
   comparisons, clipboard sentinels, Undo/Redo, Escape, and both inspections.
   After acceptance, remove only these new fixtures and discard scratch buffers:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py cleanup "$selection_fixture"
   python3 docs/runbooks/startup-recovery-fixture.py cleanup "$selection_repair"
   unset selection_fixture selection_repair
   ```

   Confirm both removal messages. Existing operator fixtures are unrelated.
