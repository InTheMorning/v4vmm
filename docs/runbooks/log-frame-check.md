# Shared Log Frame Check

Owner: [ADR 0063 task 005](../tasks/adr-0063-task-005-shared-log-frames-and-following.md).
Operator gate open. These checks cover the new shared presentation without
reopening ADR 0066 task 006's accepted repair behavior.

Use a Linux desktop, Python 3.11+, this checkout's debug binary, and an ordinary
user account. No audio device, publisher, encoder or external server is required.
The fixtures use a prepared three-track library, a Null player, loopback-only
endpoints and isolated command stubs. Agents must not run the GUI.

## V1 — Recovery, Configuration Repair And Diagnostics

From the repository root:

```bash
cargo build --quiet
repair_fixture=$(python3 docs/runbooks/startup-recovery-fixture.py setup)
python3 docs/runbooks/startup-recovery-fixture.py mode "$repair_fixture" repair-toml
python3 docs/runbooks/startup-recovery-fixture.py run "$repair_fixture"
```

1. Open startup details. The report must occupy a bordered log frame with compact
   monospace text, visible scrollbars when content overflows, and Go to latest
   below the text. The startup title, summary and Check again/Open app actions
   stay outside that viewport. No report text may be drawn over the surrounding
   chrome. Resize from wide to narrow and back several times, including while
   scrolled horizontally. Both scrollbar tracks and thumbs must stay inside
   the text viewport, above the footer; neither may overlap Configuration
   repair or its explanation. Drag each visible thumb after resizing and
   confirm it still scrolls the log. Repeat after opening the editor in step 2.
   This is the situational ADR 0063 regression check for displaced log overlays.
2. Choose Edit configuration. Remove the final `invalid = [` line and its
   following comment. The editable document remains a separate input. Choose
   Test draft and paths repeatedly until the repair history overflows its frame.
   New results must stay visible while following. Scroll upward using both the
   wheel and the scrollbar; another Test must leave the reading line in place
   and show Following paused (Paused in a compact footer). Go to latest must
   return to the bottom. Tab to
   that control and activate it with the keyboard; also resume by scrolling to
   the bottom. Horizontal scrolling must reach the end of the full file paths.
   While the surrounding page can scroll, keep the pointer over the log text or
   blank space inside its viewport. Wheel in both directions, through the middle
   and beyond each end: only the log may move; the surrounding heading, editor
   and buttons must stay put. Repeat over the short startup-details log when
   its text does not overflow. Move the pointer outside the log viewport and
   confirm the page still scrolls. This is the situational ADR 0063 regression
   check for wheel events moving both nested log and containing page.
   Add blank lines to the draft until the editor also scrolls. Keep the page
   scrolled so both its scrollbar and the editor's scrollbar are visible beside
   the input. Their tracks must have clear horizontal separation at wide and
   narrow widths. Drag the editor's thumb: only the draft view should move.
   Drag the page's thumb: the page should move. No thumb may be obscured by the
   other track or require moving the page to expose it. This is the situational
   ADR 0063 regression check for overlapping nested scrollbar hit areas. Remove
   the added blank lines after checking; keep the syntax correction.
3. Select report text and copy with Ctrl+C and right-click Copy. Paste into a
   scratch editor and compare exact whitespace, Unicode and complete paths.
   Append another Test result while a selection is active: selected old text
   must remain selected. Copy repair report must still copy the full report.
4. Save correction, Check again, then Open app. In Settings → Diagnostics and
   Library, the same configuration history must retain its reading position.
   Background tools must use the same frame and text size. Switching Settings
   groups must not reset a log's reading position. Startup-details disclosure
   retention has the separate check below. Compare Light/Dark and each available UI scale using General's
   preview controls; do not use its Save action for this check. Confirm Music
   still contains one playlist and three tracks.
   At narrow widths, confirm the Settings page scrollbar also stays clear of
   the editor and log scrollbars at every available UI scale.
   Widen the window: logs and the correction editor must use the available
   Settings width, without the former fixed narrow column or overlapping the
   page scrollbar. Make the window short enough for Library and Diagnostics
   pages to scroll. Leave each page at a different identifiable position, then
   alternate groups and visit General. Each page must restore its own position;
   a first visit starts at the top, and a short General page must not reset the
   others. Visit Music, then Settings, and confirm the same restoration. These
   are the situational ADR 0069 width/page-position regression checks, accepted
   under this packet. Log reading positions remain independent of page positions.
5. Quit normally, inspect and retain the output:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py repair-inspect "$repair_fixture"
   ```

   Expect the original in an owner-only backup, preserved unedited configuration
   values/library/music/bindings/migrations, and no leftover probes or candidates.
   After the operator accepts these repair/Settings checks and inspection:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py cleanup "$repair_fixture"
   unset repair_fixture
   ```

### Recovery Disclosure Check

Use a separate fresh fixture to inspect startup details without changing its
configuration:

```bash
disclosure_fixture=$(python3 docs/runbooks/startup-recovery-fixture.py setup)
python3 docs/runbooks/startup-recovery-fixture.py mode "$disclosure_fixture" repair-toml
python3 docs/runbooks/startup-recovery-fixture.py run "$disclosure_fixture"
```

1. Show details. Narrow the window until a long report line scrolls horizontally.
   Scroll right and, where vertically scrollable, upward. Note the visible text
   and following state.
2. Hide details, then show them again. Both offsets and the following state must
   return unchanged. Repeat once. Do not edit or save the configuration.
3. Quit normally and inspect:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py repair-inspect "$disclosure_fixture"
   ```

   Expect unchanged configuration bytes, preserved music/library and no probes
   or candidates. No backup is required because this check saves no correction.
   After the operator accepts disclosure retention and preservation:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py cleanup "$disclosure_fixture"
   unset disclosure_fixture
   ```

## V2 — Show Sources, Fresh Entries And Reading Anchors

```bash
log_fixture=$(python3 docs/runbooks/log-frame-fixture.py setup)
python3 docs/runbooks/log-frame-fixture.py verify "$log_fixture"
python3 docs/runbooks/startup-recovery-fixture.py run "$log_fixture"
```

The fixture supplies 80 entries per service, with Unicode and long paths. Its
service stubs permit observation only. It also admits one held fixture command
for V3; that command does no work on music or the database.

1. Open Show and the Producer Logs action. The pane must initially show the
   latest entry and the same frame/typography as recovery. Open Publisher Logs;
   its source title and entries must name the publisher unit. Event Logs must
   use the same frame and keep its existing snapshot semantics.
2. In a second terminal set `log_fixture` to the exact directory shown by setup
   (or use `startup-recovery-fixture.py locate` if only this fixture exists),
   verify it, then append entries:

   ```bash
   python3 docs/runbooks/log-frame-fixture.py verify "$log_fixture"
   python3 docs/runbooks/log-frame-fixture.py append "$log_fixture"
   ```

   The visible service log must update on the next observation cycle, without
   closing it or clicking Logs again. Follow the new bottom automatically.
3. Scroll up in Producer and note its first visible entry. Append again; that
   entry must stay in place. Switch to Publisher and scroll to another position.
   Switch back, close/reopen the pane, and visit Settings then Show. Each source
   must retain its own position and follow state, including while a fresh read
   loads. Go to latest affects only the current source. Event snapshots must
   not replace an open service source.
4. Horizontally scroll a long line to `END-OF-LONG-LINE`. Select across its
   visible edge and copy using keyboard and context menu. Compare with the
   corresponding file under `$log_fixture/logs/`. Neither clipping nor an
   ellipsis may change copied text. Resize the pane and window; scrollbar tracks
   and thumbs must remain inside the text viewport and usable, with no overlap
   onto the footer or surrounding controls. The footer must cover no text.
   Wheel over the log through the middle and beyond both ends; only its text
   may scroll. Wheel outside it to confirm ordinary page scrolling still works.
   Cross the single-column card breakpoint and perform the ADR 0070 allocation
   retest below. Compact card density, full-width docking, card-title readability
   and inactive transport remain separate work.
5. While paused near the newest entries, trim the journal:

   ```bash
   python3 docs/runbooks/log-frame-fixture.py trim "$log_fixture"
   ```

   A surviving first visible entry must stay anchored. Pause near the top, then
   replace all entries:

   ```bash
   python3 docs/runbooks/log-frame-fixture.py replace "$log_fixture"
   ```

   The log must remain paused, show the oldest available entry, and explain that
   earlier text is no longer available. Go to latest must resume following.

### ADR 0070 — Open Log Height At The Single-Column Breakpoint

This situational manual regression guards loss of the log body when card rows
consume the available height. It uses the same isolated V2 fixture.

If a rebuilt binary needs to replace the running app, first release the held
fixture command from the second terminal:

```bash
python3 docs/runbooks/startup-recovery-fixture.py session-release "$log_fixture"
```

Close the app normally. After it exits, verify the directory, restore only the
held-command marker for the later V3 check, and reopen with the rebuilt binary:

```bash
python3 docs/runbooks/log-frame-fixture.py verify "$log_fixture"
touch "$log_fixture/session.hold"
python3 docs/runbooks/startup-recovery-fixture.py run "$log_fixture"
```

Do not rerun fixture setup or mode: retain the existing journals and library.

1. Open Show → Live Metadata → Producer Logs. Narrow the window until the
   cards form one column. Log text and Go to latest must remain visible.
2. Scroll the cards to reach Source, Live Metadata and Stream. Card scrolling
   must not move the log or sidebar. All three sidebar Logs actions must remain
   reachable, with sidebar scrolling if its own contents overflow.
3. Drag the split both ways. It must retain readable log text at the smallest
   permitted pane height. Shorten, then enlarge the window; the chosen log
   height should return when it fits. Close/reopen logs and confirm the cards
   reclaim their region on close and the log returns on reopen.
   In both service logs, the header source must occupy one line, with line
   count and brief state on the next row. Hover the source to inspect the full
   unit name. Click it, Ctrl+A, then Ctrl+C; paste into a scratch editor and
   compare with the complete unit name in the journal. Right-click Copy must
   preserve the same selected identity. Close the scratch editor afterward.
   Hover the metadata row for the longer service explanation. A wrapped source,
   lost state/count, clipped Close action, or changed copied identity fails.
   If the count clips, hover the count itself to read its complete text.
4. Repeat the breakpoint check in Light/Dark and at XS–XL using General's
   preview controls without Save. Restore Dark/M. Check scrollbar containment,
   text/footer clearance, source switching and wheel isolation at the narrow
   size. A log header with no usable body, overlap onto sidebar actions, or an
   unreachable card fails this check.

   At XL and the very narrow width, the footer must stay one row and leave log
   text visible. It shows a short state and an icon-only Go to latest control;
   hover each for its full description. Scroll up to Paused, then activate the
   icon with mouse and Tab/Enter. Widen the pane to recover the full labels
   without changing the reading state. Repeat on a narrow Diagnostics log to
   guard the shared footer owner. A vertical string of letters or a missing
   following control fails.

Keep the fixture for V2–V3 completion, preservation inspection and the cleanup
commands below. If the retest fails, retain it for diagnosis.

## V3 — Session Drain And Retained Session Reports

Use the still-open V2 fixture. Compare Light/Dark and available scales; restore
Medium/Dark before the final inspection.

1. In Settings → Diagnostics choose End app session. The fixture's held command
   keeps the drain report visible. The report must use the same framed log,
   with Retry drain, Copy report and Quit outside it. Retry until enough report
   entries overflow; verify following, manual pause, and Go to latest.
2. Leave the report paused and note its first visible entry. Release only the
   fixture command from the second terminal:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py session-release "$log_fixture"
   ```

   Retry drain and Check again. After drain finishes and Open app is available,
   restore the marker before opening the next session, to hold that new session
   for the following-state check:

   ```bash
   touch "$log_fixture/session.hold"
   ```

   Choose Open app, then Settings → Diagnostics. The retained session report
   must use the same frame and restore its paused reading position where the
   noted entry survives. Choose Go to latest to inspect the resumption entry.
3. With following enabled, choose End app session again. Confirm the named
   fixture command holds the drain. Leave following enabled, release the
   command with session-release, then Retry drain, Check again and Open app.
   Do not restore the hold marker this time. Diagnostics must retain following
   and show the new resumption entry automatically. Inspect the retained report
   at narrow/wide widths, in Light/Dark and at available scales using General's
   preview controls without Save; restore Dark/M.
4. Complete the [editor Close/Reopen regression](#configuration-editor-close-and-reopen--adr-0066)
   below, then quit normally and inspect:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py inspect "$log_fixture"
   ```

   Expect preserved configuration apart from permitted workspace preferences,
   three tracks and playlist memberships, one playlist, original bindings,
   migrations 1–11, and no probe leftovers. Record V2–V3 and inspection results.
   On failure keep the fixture for diagnosis. After acceptance:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py cleanup "$log_fixture"
   unset log_fixture
   ```

Close scratch report buffers. Cleanup removes only the verified fixture and
its held-command marker, logs and isolated commands; it changes no real service.

## Configuration Editor Close And Reopen — ADR 0066

Use the retained Show/session fixture after V3's report checks. If those checks
used the previous binary, quit normally and reopen the same fixture with the
rebuilt binary. Do not rerun setup or mode:

```bash
python3 docs/runbooks/log-frame-fixture.py verify "$log_fixture"
python3 docs/runbooks/startup-recovery-fixture.py run "$log_fixture"
```

1. In Settings → Diagnostics, choose Edit configuration. Select
   `musicindex_endpoint` and note its current value. Append `/draft-check` to
   its text without saving or testing the draft.
2. Close editor must be visible beside Configuration repair. Activate it with
   mouse and, on subsequent close/reopen cycles, Tab/Enter and Tab/Space. Each
   full press and release must change state once; holding either key through
   auto-repeat and then releasing it must still change state once. The input, field picker
   and editing actions disappear; the repair log remains visible at its reading
   position. Closing must not add a load or save entry to that log.
   With focus inside the text input, verify the visible Escape hint, then press
   Escape. The editor must stay open and focus must visibly move to Close
   editor. The selected field, unsaved text and repair log must stay unchanged.
   Only Enter or Space on Close editor closes it. Reopen it and confirm the
   draft survived; Tab must reach the input again. Enter and Tab inside the
   input must still edit text, not close it. If an input menu is open, Escape
   must dismiss that menu first; a subsequent Escape focuses Close editor.
3. Switch Library/Diagnostics. The editor stays closed. Choose Reopen editor:
   the same field and unsaved value return without a new load report. Select
   another field and return to confirm the retained edit. Use Reload file
   (discard draft) to restore the original value explicitly.
4. Preview XL and narrow the window. Close/Reopen must remain reachable without
   overlapping the title or log. End app session and repeat the draft,
   Close/Reopen, focused-input Escape/focus-return and Reload checks in recovery.
   Also use Tab/Enter and Tab/Space
   on Show details/Hide details: each press/release or hold/release must toggle
   that disclosure once. This is the situational ADR 0069 regression for the
   shared Button dispatching on both key-down and GPUI's synthesized key-up
   click. This checks the shared control
   without saving a configuration correction or recreating repair fixtures.
5. Check again and Open app. Confirm Dark/M; if resumption did not restore it
   automatically, restore it using General's preview controls,
   then return to V3's final inspection and cleanup steps. Close any scratch
   buffers. On failure retain this fixture for diagnosis.
