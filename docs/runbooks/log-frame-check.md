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
   chrome. Inspect normal and narrow window widths.
2. Choose Edit configuration. Remove the final `invalid = [` line and its
   following comment. The editable document remains a separate input. Choose
   Test draft and paths repeatedly until the repair history overflows its frame.
   New results must stay visible while following. Scroll upward using both the
   wheel and the scrollbar; another Test must leave the reading line in place
   and show Following paused. Go to latest must return to the bottom. Tab to
   that control and activate it with the keyboard; also resume by scrolling to
   the bottom. Horizontal scrolling must reach the end of the full file paths.
3. Select report text and copy with Ctrl+C and right-click Copy. Paste into a
   scratch editor and compare exact whitespace, Unicode and complete paths.
   Append another Test result while a selection is active: selected old text
   must remain selected. Copy repair report must still copy the full report.
4. Save correction, Check again, then Open app. In Settings → Diagnostics and
   Library, the same configuration history must retain its reading position.
   Background tools must use the same frame and text size. Switching Settings
   groups or hiding/reopening startup details must not reset a log's reading
   position. Compare Light/Dark and each available UI scale using General's
   preview controls; do not use its Save action for this check. Confirm Music
   still contains one playlist and three tracks.
5. Quit normally, inspect and retain the output:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py repair-inspect "$repair_fixture"
   ```

   Expect the original in an owner-only backup, preserved unedited configuration
   values/library/music/bindings/migrations, and no leftover probes or candidates.
   After the operator accepts V1 and inspection:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py cleanup "$repair_fixture"
   unset repair_fixture
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
   ellipsis may change copied text. Resize the pane and window; the footer must
   cover no text. Keep enough window height for the existing Show card grid;
   its separately tracked narrow-layout allocation remains outside this packet.
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

## V3 — Session Drain And Retained Session Reports

Use the still-open V2 fixture. Compare Light/Dark and available scales; restore
Medium/Dark before the final inspection.

1. In Settings → Diagnostics choose End app session. The fixture's held command
   keeps the drain report visible. The report must use the same framed log,
   with Retry drain, Copy report and Quit outside it. Retry until enough report
   entries overflow; verify following, manual pause, and Go to latest.
2. Release only the fixture command from the second terminal:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py session-release "$log_fixture"
   ```

   Retry drain, Check again and Open app. Diagnostics must retain the previous
   session report in the same frame and at its paused reading position where
   the line survives. Its new resumption entry must follow if following was on.
3. Quit normally and inspect:

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
