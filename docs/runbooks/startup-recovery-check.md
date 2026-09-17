# Startup Recovery Check — ADR 0066

## Task 002: Core Checks And Reports

Owner: [task 002](../tasks/adr-0066-task-002-core-checks-and-startup-reports.md).
Operator acceptance passed on 2026-09-10. Retained for regression checks.
This checks the startup recovery window;
it does not repeat Show's event-recovery checks.

Needs: a Linux desktop, Python 3.11 or later, and this checkout's debug binary.
The fixture supplies its own music directory, SQLite database and broken states.
It stubs external control commands. No real relay, publisher, mpv or MusicIndex
service is needed. Agents may verify the backend fixture but must not use `run`.

Use these steps one at a time. Commands are unindented and need no heredoc.
Run them from the repository root.

The fixture's `run` command first builds the normal desktop binary with Cargo,
using the developer environment and cached dependencies. It then launches with
the isolated fixture environment. This prevents a preceding `cargo test` from
leaving GPUI's synchronous test drawing loop in the desktop binary. A build
failure stops launch and preserves the fixture. Cargo and cached dependencies
must be available in the desktop terminal.

### 1. Create And Verify The Fixture

This keeps the check away from your actual configuration and library.

```bash
cargo build --quiet
recovery_dir="$(python3 docs/runbooks/startup-recovery-fixture.py setup)"
python3 docs/runbooks/startup-recovery-fixture.py verify "$recovery_dir"
```

The printed paths must belong to the same `v4vmm-startup-...` directory.
If the shell variable is lost, recover it in that terminal:

```bash
recovery_dir="$(python3 docs/runbooks/startup-recovery-fixture.py locate)"
python3 docs/runbooks/startup-recovery-fixture.py verify "$recovery_dir"
```

### 2. Explain A Broken Configuration

This proves that a TOML error opens recovery instead of the normal app.

```bash
python3 docs/runbooks/startup-recovery-fixture.py mode "$recovery_dir" invalid-toml
python3 docs/runbooks/startup-recovery-fixture.py run "$recovery_dir"
```

Look for the configuration path, line/column, and a clear correction action.
Music and Show must be absent. Open app must be unavailable.
Hide and show details. Copy report and paste into a text editor: all details
must be present, including UTC timestamps. Copy dispatch alone is not a pass.
Choose Quit. The terminal must print a nonzero exit code.

Check that recovery preserved the fixture:

```bash
python3 docs/runbooks/startup-recovery-fixture.py inspect "$recovery_dir"
```

`config_bytes_unchanged`, `music_preserved` and
`migration_records_preserved` must be true. Both probe results must be empty/zero.

### 3. Name The Music Storage Failure

This proves that a missing music directory is not silently recreated.

```bash
python3 docs/runbooks/startup-recovery-fixture.py mode "$recovery_dir" music-missing
python3 docs/runbooks/startup-recovery-fixture.py run "$recovery_dir"
```

Recovery must name the missing music location. Choose Check again; the error
must remain. Quit and run inspect. The original audio stays in `music.saved`.

```bash
python3 docs/runbooks/startup-recovery-fixture.py inspect "$recovery_dir"
python3 docs/runbooks/startup-recovery-fixture.py mode "$recovery_dir" music-file
python3 docs/runbooks/startup-recovery-fixture.py run "$recovery_dir"
```

This time, recovery must say the music path is a file rather than a directory.
Quit, then inspect again. Reusing the TOML error message here is wrong.

```bash
python3 docs/runbooks/startup-recovery-fixture.py inspect "$recovery_dir"
```

### 3a. Confirm Repeated Checks

This proves that an identical failure still acknowledges each completed check.
Close any fixture window, then use the rebuilt binary:

```bash
python3 docs/runbooks/startup-recovery-fixture.py mode "$recovery_dir" music-file
python3 docs/runbooks/startup-recovery-fixture.py run "$recovery_dir"
```

Hide details. Choose Check again twice, letting each check finish. The line
below the title must show a new check number and a recorded UTC completion time
after each check. The error remains and Open app stays unavailable. The line
must remain visible with details hidden or scrolled away. A briefly changing
button alone is insufficient for a fast check. Copy report must include the
same completion line. Quit afterward. Accepted path/data checks need no repeat.

### 4. Recover Without Relaunching

This proves that a locked database can be checked again and opened once.

In terminal A:

```bash
python3 docs/runbooks/startup-recovery-fixture.py mode "$recovery_dir" db-locked
python3 docs/runbooks/startup-recovery-fixture.py run "$recovery_dir"
```

Allow about five seconds. While waiting, Check again must read Checking and
prevent another check. Recovery must name SQLite's write check and the lock.
The window must stay responsive. Open app must be unavailable.

In terminal B, release only the fixture's lock:

```bash
recovery_dir="$(python3 docs/runbooks/startup-recovery-fixture.py locate)"
python3 docs/runbooks/startup-recovery-fixture.py mode "$recovery_dir" normal
```

This changes the fixture; it does not refresh the UI. In recovery, press
Check again. Its report must now identify successful music and SQLite checks.
Press Open app once. Music must open in the same window. A second window,
repeated startup, or needing to relaunch is wrong. Close the app using the
window manager's close control; terminal A must print exit code 0 with no panic.

```bash
python3 docs/runbooks/startup-recovery-fixture.py inspect "$recovery_dir"
```

After normal startup, existing workspace preference saves may change config
bytes. Inspect reports `normal_workspace_preferences_only` separately; every
non-workspace setting must still match. Recovery before normal startup must
preserve every config byte. The playlist count stays 1, migration records stay
the same, and no probe remains.

### 4a. Recheck Window-Manager Closing

This checks the close callback corrected after the operator's exit-101 panic.
With the rebuilt binary, open the healthy fixture:

```bash
python3 docs/runbooks/startup-recovery-fixture.py mode "$recovery_dir" normal
python3 docs/runbooks/startup-recovery-fixture.py run "$recovery_dir"
```

Wait for Music, then close using the same window-manager control that caused
the crash. Expect exit code 0 and no panic. Inspect afterward using the command
in section 4. Section 5 checks the same close callback while recovery is blocked.
Previously accepted storage and feedback checks need no repeat.

### 5. Read And Copy A Long Path

This proves that narrow recovery windows still expose the complete report.

```bash
python3 docs/runbooks/startup-recovery-fixture.py mode "$recovery_dir" long-path
python3 docs/runbooks/startup-recovery-fixture.py run "$recovery_dir"
```

Inspect normal and narrow widths. Details must wrap and scroll. Buttons must
remain reachable and wrap into rows. Copy report and paste it: the long path
must be complete. Clipping, an unreadable vertical button label, or inaccessible
actions fails this check. Close recovery using the window manager's close
control. Expect exit code 1 with no panic; the app never opened normal
operations. Inspect once more.

```bash
python3 docs/runbooks/startup-recovery-fixture.py inspect "$recovery_dir"
```

### Cleanup

Close the fixture app first. This removes only its verified directory and
stops only its lock process.

```bash
python3 docs/runbooks/startup-recovery-fixture.py cleanup "$recovery_dir"
```

Record results in task 002, its review checklist and the delivery row. Remove
its pending-human-check entry only after all sections, including 3a and 4a, pass. Later packets
own optional constructor isolation and the in-app correction editors.

## Task 003: Background Tools

Owner: [task 003](../tasks/adr-0066-task-003-runtime-failure-and-shell-availability.md).
Operator acceptance and fixture cleanup are complete - 2026-09-11. These
procedures remain for regression checks: failed background tools do not prevent
startup, and Check again repairs them in the same window.
Task 002's accepted core checks need no repeat.

Needs: a Linux desktop, Python 3.11 or later and the new debug binary. No real
external service is needed. The fixture deliberately uses an unreachable
MusicIndex endpoint and stubs control commands. An Index connection error after
runtime repair is expected; it must not be described as a runtime failure.

### 1. Open With Two Failed Tools

This proves that optional failures leave navigation and local reads usable.

In terminal A, from the repository root:

```bash
cargo build --quiet
recovery_dir="$(python3 docs/runbooks/startup-recovery-fixture.py setup)"
python3 docs/runbooks/startup-recovery-fixture.py verify "$recovery_dir"
python3 docs/runbooks/startup-recovery-fixture.py mode "$recovery_dir" runtime-and-cache-unavailable
python3 docs/runbooks/startup-recovery-fixture.py run "$recovery_dir"
```

Music must open. Visit Show, then Settings. The notice/report must identify
two separate failures: the background runtime and thumbnail cleanup.
Show must say **Show status unavailable** and **Not checked**; it must not claim
that playback is idle or invent event/service observations.
**Open report** leads to **Background tools** in Settings. **Copy report**
must paste the complete report, with UTC times and the thumbnail-cache path.
At normal and narrow widths, text must wrap and actions remain reachable.

Use the toolbar to search for `example`, then repeat by pressing Enter in the
search input. Local results may be empty. The Index result must explain that
the runtime is unavailable, with **Check again** still available in the notice.
Browse the existing **Startup fixture playlist**. Press Ctrl+R in Music, and
try the playback shortcut Ctrl+Alt+P (ADR 0067). Neither may panic or bypass the missing
runtime. Navigation, report copy and the repair buttons must still work.

If the window manager intercepts a shortcut, record the affected check as
unverified and continue the other checks. Do not count an intercepted key as an
app rejection. The earlier Super-key checks were intercepted; use the focused
[Ctrl shortcut check](../tasks/adr-0067-task-001-platform-shortcuts.md#operator-visual-check)
to verify keyboard delivery when regression testing; its operator acceptance is
complete and needs no repeat.

In Settings, press the runtime's **Check again** twice, waiting for each to
finish. The error should remain, but the completed check number must advance.
The thumbnail issue must remain. No permanent **Checking…** state is allowed.
This fixture returns its failure immediately, so **Checking…** may finish before
a frame renders. The increasing count and recorded completion time confirm the
attempt; seeing the intermediate label is not required for this case.

### 2. Repair Only The Runtime

This proves that repairing one tool does not erase the other tool's failure.

Leave the app open. In terminal B, recover and verify this fixture directory
(or use the exact directory printed by terminal A if several fixtures exist):

```bash
recovery_dir="$(python3 docs/runbooks/startup-recovery-fixture.py locate)"
python3 docs/runbooks/startup-recovery-fixture.py verify "$recovery_dir"
python3 docs/runbooks/startup-recovery-fixture.py mode "$recovery_dir" cache-worker-unavailable
```

Changing the fixture mode only changes what the next attempt will return.
Press the runtime's **Check again** in Settings. Its failure must clear;
thumbnail maintenance must remain failed. Stay in the same app window.
Return to Music. Runtime installation refreshes the library automatically; the
existing playlist must appear once. If the desktop forwards Ctrl+R, also check
that explicit refresh works. Otherwise keep that shortcut check unverified.
Online requests may report the fixture's connection
failure. Double-clicking Check again must not create duplicate rows or windows.

### 2a. Recheck Search Failure Readability

The operator's post-repair screenshot proved remote dispatch but exposed a
clipped technical error. This focused correction keeps the explanation readable
and puts full diagnostics behind Show details and Copy report.

Close the fixture app before relaunching the new debug binary. From the repository
root, use the existing verified fixture; do not create another one:

```bash
cargo build --quiet
python3 docs/runbooks/startup-recovery-fixture.py verify "$recovery_dir"
python3 docs/runbooks/startup-recovery-fixture.py mode "$recovery_dir" cache-worker-unavailable
python3 docs/runbooks/startup-recovery-fixture.py run "$recovery_dir"
```

Search for `runtime-retry`. At normal and narrow widths, the explanation must
wrap inside the result pane with Show details and Copy report reachable. It must
say that the app could not get MusicIndex results, explain that the local library
remains available, and suggest checking the connection/endpoint before repeating
the search. It must not blame an unavailable background runtime.

Open Show details. The report must wrap and scroll vertically when needed, with
no clipped edges. Copy report and paste into a text editor; it must include the
recorded UTC time, configured endpoint, and both feed/track failure details.
Hide/show must keep the same recorded time and copied report. Select the Library
filter: an empty local result must not display the remote failure or its report
buttons. Return to Music and confirm Startup fixture playlist appears once.

Keep this fixture open for thumbnail repair below. Final inspection/cleanup
remains in step 4. No real MusicIndex service is required.

### 3. Repair Thumbnail Cleanup

This proves that thumbnail maintenance can recover without replacing the app.

In terminal B:

```bash
python3 docs/runbooks/startup-recovery-fixture.py mode "$recovery_dir" normal
```

Press thumbnail maintenance's **Check again**. Its failure must clear. The
report must describe the completed cleanup scan. Copy the report again: its
times must be recorded UTC times, and starting the runtime must not claim that
an external service is reachable.

### 4. Start With Only Thumbnail Cleanup Failed

This checks the cache failure independently of runtime recovery.

Close the fixture app. It must exit with code 0. In terminal A:

```bash
python3 docs/runbooks/startup-recovery-fixture.py inspect "$recovery_dir"
python3 docs/runbooks/startup-recovery-fixture.py mode "$recovery_dir" cache-worker-unavailable
python3 docs/runbooks/startup-recovery-fixture.py run "$recovery_dir"
```

Music must open with only the thumbnail maintenance issue. The runtime must
be available. Navigate all three sections and open the report. Close the app;
it must exit with code 0. Inspect preservation, then remove the fixture:

```bash
python3 docs/runbooks/startup-recovery-fixture.py inspect "$recovery_dir"
python3 docs/runbooks/startup-recovery-fixture.py cleanup "$recovery_dir"
```

Both inspections must report preserved configuration, music and migration
records, one playlist, and no leftover probes. Normal workspace preferences
may have been saved. A crash, lost data, duplicate window, hidden repair action,
or one repair clearing the other issue fails this gate.

## Task 004: Optional Tool Isolation

Owner: [task 004](../tasks/adr-0066-task-004-optional-tool-isolation.md).
Operator acceptance is open. These checks cover the new optional-resource
boundaries; tasks 002/003 and ADR 0067 keep their accepted results.

Playback-dependent checks are paused following the operator's 2026-09-11
workflow correction. Music Play currently shares the Show playback session;
the required separate audition path and explicit Show cue loading are missing.
Do not use Music Play to satisfy a Show playback check. The producer screenshot
also reports an unresolved mpv IPC read error. Report and preservation checks
remain useful, but do not accept audio or metadata publication. See
[the recorded correction](../tasks/adr-0066-task-004-optional-tool-isolation.md#playback-workflow-correction--2026-09-11).

Needs: this checkout's debug binary, Python 3.11+, a Linux desktop, an installed
`mpv` binary, and working desktop audio for the producer-failure case. Start
with the volume low. The fixture supplies three quiet, 30-second WAV tracks,
one playlist, and all broken paths/configuration. External publisher/encoder
commands use failing local stubs, so their expected result is an attempted
command followed by the fixture's service-unavailable report. No real service,
relay or MusicIndex is needed. The real mpv binary is used only in the
producer-failure case, with sockets inside the verified fixture directory.
Agents may use seed, mode, inspect and CLI commands; only a person uses `run`.

### 1. Create The Fixture

Run from the repository root in one desktop terminal:

```bash
cargo build --quiet
optional_dir="$(python3 docs/runbooks/startup-recovery-fixture.py setup)"
python3 docs/runbooks/startup-recovery-fixture.py verify "$optional_dir"
```

An agent's `/tmp` path may not be visible in the desktop session. The commands
above create the fixture in the operator's environment. For an agent-prepared
fixture, transfer its archive through the shared checkout and extract it into
the operator's `/tmp`, preserving the archive's top-level directory name. The
manifest and configuration contain that absolute path. Run `verify` in the
desktop terminal before opening the transferred fixture.

Keep this terminal and variable. Close the fixture app before every mode change.
Keep the fixture through all five cases. Cleanup belongs only to step 6; do not
run it between cases.
After each run, the terminal must report exit code 0. Inspect after closing:
config, music, library, bindings and migration preservation must be true; no
music/producer/database probe may remain. Stop on a failed inspection and keep
the directory for diagnosis. Do not use Settings Save to repair these fixtures.

If a saved path no longer contains `fixture.json`, use `locate` to find another
verified fixture. If none exists, run `setup` again and assign its printed path
to `optional_dir`. Select the next outstanding case with `mode` before `run`.
Previously recorded report and inspection evidence remains valid; a replacement
fixture does not confirm any visual check that was left unconfirmed.

### 2. Fail Index And Player Preparation Together

```bash
python3 docs/runbooks/startup-recovery-fixture.py mode "$optional_dir" endpoint-and-player-unavailable
python3 docs/runbooks/startup-recovery-fixture.py run "$optional_dir"
```

Music must open normally. Open Background tools in Settings: the invalid
MusicIndex setting and failed playback preparation must remain separate.
Copy the report and paste into an editor; verify both subjects and their recorded
UTC times. Search for `a.wav` using the toolbar and Ctrl+F: the local track must
remain visible, with an Index failure explanation. Open Startup fixture playlist;
all three rows remain browsable. Play/Ctrl+Alt+P on Show must not start audio.
ADR 0066 task 007 now keeps meaningful actions enabled as repair routes; a
retained action and focused repair access replace the disabled-affordance check. Navigate back to Settings and confirm both reports remain.
A recovery-only window, missing local results, substituted player or cleared
report is wrong. Close the app.

```bash
python3 docs/runbooks/startup-recovery-fixture.py inspect "$optional_dir"
```

### 3. Preserve Invalid Presentation Settings

Accepted - 2026-09-11, including the duplicate-warning recheck, resizing/navigation
and post-run preservation inspection. Retained for regression; proceed to step 4
when continuing task 004's outstanding checks.

```bash
python3 docs/runbooks/startup-recovery-fixture.py mode "$optional_dir" presentation-invalid
python3 docs/runbooks/startup-recovery-fixture.py run "$optional_dir"
```

Inspect Music, Show and Settings at normal and narrow widths. Text and controls
must remain usable with the documented defaults. Settings must retain distinct
reports for theme, scale and content view mode. Resize and navigate several times,
then close. Silent correction, disappearing reports or unusable fallback layout
is wrong.

The 2026-09-11 screenshot exposed duplicate warnings in Show. With the rebuilt
binary, each configuration warning must appear once in the shared top notice;
the repeated red block below Idle must be absent. Open report must still expose
the complete Settings report. The operator accepted this recheck on 2026-09-11;
these instructions remain for regression and do not reopen the accepted case.

```bash
python3 docs/runbooks/startup-recovery-fixture.py inspect "$optional_dir"
```

For this case, `config_bytes_unchanged` must be true, not merely
`normal_workspace_preferences_only`.

### 4. Fail Producer Preparation, Then Publisher Selection

```bash
python3 docs/runbooks/startup-recovery-fixture.py mode "$optional_dir" producer-unavailable
python3 docs/runbooks/startup-recovery-fixture.py run "$optional_dir"
```

No audio starts when the app opens. Open Show without invoking Music Play.
The Source card and Settings must retain the drop-file preparation failure.
Publisher setup must remain independent: its watch must report the fixture's
external service failure rather than a producer configuration failure. If a
service action is available, invoke it and verify its result names that service.
A lost producer report or a publisher result replaced by the producer failure
is wrong. This only checks the reports. Audible playback, progress and
pause/resume remain unaccepted and paused until the Show cue workflow exists.
That check must load the playlist without starting audio, then start playback
from Show; Music audition must leave Show playback and publication unchanged.
Do not substitute the current playlist Play command. Close and inspect before
selecting the next case:

```bash
python3 docs/runbooks/startup-recovery-fixture.py inspect "$optional_dir"
python3 docs/runbooks/startup-recovery-fixture.py mode "$optional_dir" publisher-invalid
python3 docs/runbooks/startup-recovery-fixture.py run "$optional_dir"
```

Publisher host controls must be unavailable with a setup report.
The independent encoder watch must reach the fixture's `butt` stub and report
that encoder result. A publisher-host error must not replace the encoder result.
The producer-publication check remains open and paused with the playback check.
Once the Show cue workflow exists, its instructions must verify metadata
publication during Show playback despite the publisher-host error. This case
configures Null, so it cannot prove audible playback. The missing-value-routes
readiness note is expected for these fixture WAVs; Music audition must not
publish Show metadata. Do not invoke Music Play to create the metadata file.

Close the app and inspect from the original terminal:

```bash
python3 docs/runbooks/startup-recovery-fixture.py inspect "$optional_dir"
```

### 5. Contain A Partially Applied Path Repair

```bash
python3 docs/runbooks/startup-recovery-fixture.py mode "$optional_dir" partial-path-repair
python3 docs/runbooks/startup-recovery-fixture.py run "$optional_dir"
```

Normal Music must open with all three tracks and a persistent path-repair warning.
The report must explain that completed changes and remaining bindings were
retained. `a.wav` and `c.wav` remain usable; `b.wav` stays visible but its play
action must be unavailable. Inspect action availability without starting playback;
this check does not accept the current Music playback routing. Do not remove
tracks or run another repair.
A rollback claim, disappearing track, or executable absolute binding is wrong.
Close and inspect:

```bash
python3 docs/runbooks/startup-recovery-fixture.py inspect "$optional_dir"
```

`bindings` must be `["a.wav", "/old/music/b.wav", "c.wav"]`, and
`repair_not_attempted` must be false. All three tracks and playlist entries remain.

### 6. Record Results And Clean Up

Record each case, report-copy result, preservation inspection and observed
failures in task 004. Keep the paused audio/publication results explicitly
unaccepted. Its visual gate closes only after all five
cases pass. Leave any failed fixture intact until its evidence is recorded.
Once the app is closed and all inspections pass:

```bash
python3 docs/runbooks/startup-recovery-fixture.py cleanup "$optional_dir"
unset optional_dir
```

Cleanup removes only the verified fixture, including its WAVs, database,
path-failure trigger, blocker files, temporary mpv sockets and configuration.

## Task 005: Session Drain And Resumption

Owner: [task 005](../tasks/adr-0066-task-005-session-drain-and-resumption.md).
Operator V1–V3, preservation inspection and fixture cleanup accepted - 2026-09-11.
The [packet records the evidence](../tasks/adr-0066-task-005-session-drain-and-resumption.md#operator-evidence--2026-09-11).
These steps remain available for regression; no repeat is requested for that acceptance.

Needs: a Linux desktop, Python 3.11+, and this checkout's rebuilt debug binary.
No audio hardware, installed mpv, network service or running broadcast service
is needed. The fixture uses the explicit Null driver, local tracks and external
command stubs. This check does not accept task 004's deferred playback workflow.
Use a **new** fixture; retain task 004's separate fixture.

### 1. Prepare A Held Command

Close the normal app. In a desktop terminal:

```bash
cd /home/citizen/build/v4vmm
cargo build --locked --offline --quiet
session_fixture=$(python3 docs/runbooks/startup-recovery-fixture.py setup)
python3 docs/runbooks/startup-recovery-fixture.py verify "$session_fixture"
python3 docs/runbooks/startup-recovery-fixture.py mode "$session_fixture" session-held-command
python3 docs/runbooks/startup-recovery-fixture.py run "$session_fixture"
```

Keep that terminal open. In a second terminal, assign `session_fixture` to the
exact directory printed by verify, then run:

```bash
cd /home/citizen/build/v4vmm
python3 docs/runbooks/startup-recovery-fixture.py session-status "$session_fixture"
```

Expect `held: true` and one `command-held` observation with a generation number.
The admitted command holds an actual configured database reference but writes
no database records. It finishes when step 4 releases it, or after a ten-minute
fixture deadline. If no observation has appeared yet, wait and check again.
Complete the held-work and release checks within ten minutes of opening that
session. If V1 takes longer, use the deadline procedure in step 3; accepted V1
checks do not need to be repeated. The `held` flag describes the marker file,
which can remain present after the command's deadline expires.

### 2. V1 — Inspect The Session Action

In Music, open **Startup fixture playlist** so its paged actor is active.
Press Ctrl+Comma and choose Diagnostics. The App session section must explain
that End app session stops this app's work and built-in playback, discards
unsaved Settings edits and leaves the external services running. Record its
current session number. Inspect this section in Light and Dark, at normal and
roughly 560-pixel widths. Change themes in General without choosing Save or Use
Defaults; theme changes apply immediately and this check must preserve the
fixture configuration. Use Tab/Shift+Tab to reach its button. The explanation
and action must remain readable, unclipped and reachable by scrolling.

Choose **End app session**. Normal Music/Show/Settings content must be replaced
by the session report. The window must keep responding while the app waits.
This explicit action authorizes stopping built-in playback and recording any
existing playback session as stopped; it does not edit configuration or library
membership. The Null-driver fixture does not require an audible playback check.

### 3. V2 — Held Work Must Block Maintenance

After approximately five seconds, expect a report that maintenance is still
unavailable and that `FixtureSessionCommand` remains. **Retry drain** and
**Copy report** must be usable. **Open app** and core repair controls must not
be available yet. Retry once while the command is still held: the app must
remain in this waiting/failure state with a new recorded report entry.

Copy the report into a blank scratch editor. At normal and narrow widths,
confirm that the copied text includes the waiting work, session number and
recorded UTC times without clipping. Rendering or resizing must not change the
recorded times.

If recovery opens without a held-work failure, compare the recorded session
startup and drain times. A drain after the ten-minute fixture deadline does not
prove V2. While the hold marker is still present, choose **Open app**, confirm
the new generation's `command-held` observation with `session-status`, open the
fixture playlist and immediately repeat the held-work check. Retain the earlier
generation observations as history. If maintenance opens while the command is
still held within its deadline, keep the fixture and report the failure.

### 4. Release And Retry

In the second terminal:

```bash
python3 docs/runbooks/startup-recovery-fixture.py session-release "$session_fixture"
python3 docs/runbooks/startup-recovery-fixture.py session-status "$session_fixture"
```

Expect `held: false`; `command-released` appears when the worker observes the
release. Choose **Retry drain** in the app. Recovery must become available only
after the resource-release report. It checks the core resources without
starting a new normal session. Copy report must retain the earlier session
messages and their original times. The fixture status must now contain one
`maintenance` observation for the old generation.
Inspect the recovery report at normal and narrow widths before proceeding;
it uses the theme selected before ending the session.

### 5. V3 — Open One Fresh Session

Choose **Open app** after the core checks pass. Music must reappear once, with
one copy of the fixture playlist and its three tracks. Return to Settings →
Diagnostics. The current session number must be larger than the old one, and
the previous session report must retain its waiting, release and resumption
entries. Inspect the retained session report in Light and Dark at both widths,
using General's immediate theme choice without saving it. No line or action may
be inaccessible. Do not repeat unrelated accepted Settings field or audio checks.

In the second terminal:

```bash
python3 docs/runbooks/startup-recovery-fixture.py session-status "$session_fixture"
```

For the accepted held-work attempt, expect the old `command-held`/
`command-released`/`maintenance` generation followed by exactly one new `opened`
generation, with `held: false`. Earlier generations from an expired attempt
remain in the history. A duplicate opening after the tested drain or an old
generation appearing as the resumed session fails this check. Paste this
command's JSON output when reporting generation evidence.

### 6. Inspect And Clean Up

Quit the app with Ctrl+Q. In the original terminal:

```bash
python3 docs/runbooks/startup-recovery-fixture.py inspect "$session_fixture"
```

Every preservation flag must be true, with no residual music or database
probes. Configuration may differ only in the permitted workspace preferences.
The fixture must retain three tracks and playlist memberships, one playlist,
the original bindings and migrations 1–11. Record V1–V3 and inspection results
before cleanup. On failure, keep this fixture for diagnosis; release a held
command with step 4 before quitting, rather than killing its work.

After all results pass:

```bash
python3 docs/runbooks/startup-recovery-fixture.py cleanup "$session_fixture" &&
unset session_fixture
```

Close the scratch editor buffer and remove a separate scratch report only if
you saved one. Cleanup removes only this fixture's config, database, music,
markers and observation file; it changes no real service or desktop audio route.

## Task 006: Configuration Repair And Resumption

Operator acceptance is complete. V1–V3, preservation and fixture cleanup
accepted - 2026-09-11; V4–V6, preservation and fixture cleanup accepted -
2026-09-13. These procedures remain for regression checks; no repeat is requested.
Run these checks as your ordinary desktop user on Linux with Python
3.11+ and this checkout's debug binary. Permission cases require an unprivileged
user. No audio hardware, running publisher or reachable server is needed.
The fixture's endpoint is loopback port 9, its player is Null, and its service
commands are isolated stubs. Agents must not run the GUI.

Run commands from the repository root. Each case uses a fresh fixture. Keep a
failed fixture and its backups for diagnosis; do not change its mode while the
app is open. `repair-access` and `repair-conflict` are the named live changes.

### V1 — Correct Malformed TOML In Recovery

Create a broken document whose original bytes must survive its correction.

```bash
cargo build --quiet
repair_fixture=$(python3 docs/runbooks/startup-recovery-fixture.py setup)
python3 docs/runbooks/startup-recovery-fixture.py mode "$repair_fixture" repair-toml
python3 docs/runbooks/startup-recovery-fixture.py verify "$repair_fixture"
python3 docs/runbooks/startup-recovery-fixture.py run "$repair_fixture"
```

1. Choose **Edit configuration** in recovery. The report must name the file,
   resolved destination, and TOML line/column. The input must contain the actual
   document in a full-width, multiline viewport. Scroll within the input to
   reach its final lines. A tiny empty box, missing text or unreachable lines
   fails this check; do not continue to Save. This is the situational ADR 0066
   regression check for the collapsed editor observed on 2026-09-11.
   Remove the final `invalid = [` line and its comment. Editing alone
   must leave the recovery report and file unchanged.
2. Choose **Test draft and paths**, then **Save correction**. Each action must
   finish with a recorded UTC result. Save must name an owner-only backup.
   Repeated Save must be unavailable after success. Save must not open Music.
3. Choose **Check again**, then **Open app**. The same window must open Music
   with one fixture playlist and three tracks. Settings → Diagnostics must retain
   the correction report and backup path.
4. Inspect the new editor/report at normal and narrow widths and in Light/Dark
   using General's immediate theme choice without saving. Text, field actions
   and Save/Reload must remain reachable. Copy repair report into a scratch
   editor: paths and recorded times must match. **Copy draft (redacted)** copies
   the whole proposed document after its syntax is valid, with credential fields
   and URL credentials removed. A malformed draft stays in the editor and returns
   an explanation instead of unsafe clipboard contents.
5. Quit, then inspect and clean up using the final section below. Expect
   `original_backed_up: true` and the report's backup path in `backups`.

### V2 — Correct Both Core Paths And Change A Running Session's Folder

The fixture retains its prepared database and music but starts with two invalid
core values. It also creates `music-choice`, a copy of its fixture audio.

```bash
repair_fixture=$(python3 docs/runbooks/startup-recovery-fixture.py setup)
python3 docs/runbooks/startup-recovery-fixture.py mode "$repair_fixture" repair-paths
python3 docs/runbooks/startup-recovery-fixture.py verify "$repair_fixture"
printf '%s\n' "$repair_fixture/music-choice"
python3 docs/runbooks/startup-recovery-fixture.py run "$repair_fixture"
```

1. Choose **Edit configuration**. Select `music_dir` and enter the full Music
   path printed by verify, without quotes or a trailing line break. Enter each
   path as one line; a line break becomes part of the path.
   Select `db_path`; the unsaved music
   edit must remain in the draft. Enter a nonexistent file in the fixture's data
   directory. Test and Save must reject it without creating a database or backup.
2. Replace that draft value with the existing Database path printed by verify.
   Test, Save, Check again and Open app must succeed as separate actions.
3. In Settings → Library, the current music folder is a value with a repair
   route. Choose **Reload file (discard draft)** in Configuration repair, select
   `music_dir`, and enter the full `music-choice` path printed above. Choose
   **End session to edit core paths**. The running app must drain before recovery
   permits Save. The proposed folder must survive the transition in the editor.
4. After the old-session check finishes, Test and Save. Music must remain closed
   until you choose Check again and Open app. The Library group must then name
   `music-choice`. Diagnostics must retain the session and repair reports, with
   a larger session number. The playlist must still have three tracks once.
5. Quit and inspect. Original and selected audio must be preserved, as must
   database bindings, tracks, playlist memberships and migrations. Both saved
   originals must appear in `backups`; no music was moved by the app.

### V3 — Correct Two Independent Optional Fields

The fixture opens normally with invalid endpoint and converter values.

```bash
repair_fixture=$(python3 docs/runbooks/startup-recovery-fixture.py setup)
python3 docs/runbooks/startup-recovery-fixture.py mode "$repair_fixture" repair-optional
python3 docs/runbooks/startup-recovery-fixture.py run "$repair_fixture"
```

1. Open Settings → Library → Edit configuration. The same multiline viewport
   must show the selected field's value and remain usable at normal and narrow
   widths; a collapsed or empty input fails this ADR 0066 regression check.
   Correct only
   `musicindex_endpoint` to `http://127.0.0.1:9` and Save correction. The saved
   result must still name `flac_path`, and ordinary persistence must stay paused.
   The second field's value must remain `false` in the file.
2. Reload the file, select `flac_path`, clear its input, then Save correction.
   This removes the invalid optional setting. The fresh-read result must permit
   ordinary persistence. Both backup paths and the earlier result must remain.
3. In General choose Medium and Dark, then use the ordinary **Save** control.
   It must succeed. Return to Library: the endpoint control must retain the
   corrected loopback URL and the converter input must be empty. Optional tool
   reinitialization and original-operation retry belong to task 007; this check
   must not require a request or claim a successful server observation.
4. Quit and inspect. The only permitted non-layout changes are the corrected
   endpoint, removed converter value, and explicitly saved Medium/Dark values.

### V4 — Unreadable Configuration Has No Invented Editor

Mode 000 denies access to this fixture's configuration file.

```bash
repair_fixture=$(python3 docs/runbooks/startup-recovery-fixture.py setup)
python3 docs/runbooks/startup-recovery-fixture.py mode "$repair_fixture" repair-unreadable
python3 docs/runbooks/startup-recovery-fixture.py run "$repair_fixture"
```

Choose Edit configuration. Expect a named read failure and a retry action,
with no default-filled input or Save action. In a second terminal, locate and
verify the newest fixture, then restore only its permissions:

```bash
repair_fixture=$(python3 docs/runbooks/startup-recovery-fixture.py locate)
python3 docs/runbooks/startup-recovery-fixture.py verify "$repair_fixture"
python3 docs/runbooks/startup-recovery-fixture.py repair-access "$repair_fixture"
```

The verified path must match the fixture launched above. Edit configuration
must now load its original values. Before Open app, inspection must show
`original_preserved: true` and no backups; loading and checking in recovery must
not rewrite the document. Then Check again and Open app must work without a
configuration correction or Save.

Quit and inspect. Normal workspace persistence may now add layout preferences,
as in the earlier startup checks. In that case the inspector reports
`normal_workspace_preferences_only: true` and `config_preserved: true`, while
`original_preserved` and `config_bytes_unchanged` remain false. This allowance
requires the matching successful-exit record, a verified original case copy,
and unchanged values outside the workspace sections. Before normal resumption,
any byte change fails. No backup is expected because no Save was requested.

The situational ADR 0066 [fixture tests](test_startup_recovery_fixture.py) guard
this distinction, failed/mismatched exits, changed settings and correction cases
that still require original preservation. Run them with
`python3 -B docs/runbooks/test_startup_recovery_fixture.py`.

### V5 — Backup Failure Preserves Both Original And Draft

This malformed document is readable, but its containing directory denies writes.

```bash
repair_fixture=$(python3 docs/runbooks/startup-recovery-fixture.py setup)
python3 docs/runbooks/startup-recovery-fixture.py mode "$repair_fixture" repair-backup-failure
python3 docs/runbooks/startup-recovery-fixture.py run "$repair_fixture"
```

Load and correct the syntax as in V1. Save must report that it could not create
the backup, leave the draft editable, and preserve the malformed original.
In a second terminal restore directory access without editing the file:

```bash
repair_fixture=$(python3 docs/runbooks/startup-recovery-fixture.py locate)
python3 docs/runbooks/startup-recovery-fixture.py verify "$repair_fixture"
python3 docs/runbooks/startup-recovery-fixture.py repair-inspect "$repair_fixture"
python3 docs/runbooks/startup-recovery-fixture.py repair-access "$repair_fixture"
```

Verify the path matches. Save the retained draft again. It must now name a
backup; Check again and Open app must work. Quit and inspect once more.

### V6 — Concurrent Editor Conflict

Keep an endpoint correction draft while a second editor changes the source.

```bash
repair_fixture=$(python3 docs/runbooks/startup-recovery-fixture.py setup)
python3 docs/runbooks/startup-recovery-fixture.py mode "$repair_fixture" repair-conflict
python3 docs/runbooks/startup-recovery-fixture.py run "$repair_fixture"
```

In Settings → Library, load the file and draft endpoint `http://127.0.0.1:9`.
Do not save yet. In a second terminal:

```bash
repair_fixture=$(python3 docs/runbooks/startup-recovery-fixture.py locate)
python3 docs/runbooks/startup-recovery-fixture.py verify "$repair_fixture"
python3 docs/runbooks/startup-recovery-fixture.py repair-conflict "$repair_fixture"
```

Confirm the fixture path matches. Save correction must report the conflict,
retain the proposed endpoint, and leave the external value `99` unchanged.
Copy draft must retain the proposed document; no automatic merge is allowed.
Quit without reloading or saving again. Inspection must report
`external_revision_preserved: true`; a conflict found before preservation need
not create a backup.

### Inspect And Clean Up Each Task 006 Fixture

After quitting, use the same terminal variable for that case:

```bash
python3 docs/runbooks/startup-recovery-fixture.py repair-inspect "$repair_fixture"
```

Expect original/unedited-config/music/library/bindings/migration preservation,
owner-only backups when saved, one playlist with three tracks, and no candidate,
music-probe or database-probe leftovers. The inspector prints SHA-256 checksums
for every backup. Record the case, visible results, copied report and inspection
output. V4 alone can use the separately reported normal-workspace allowance
after a successful app exit; its recovery phase still requires unchanged bytes.
Preserve failed fixtures. After that case passes:

```bash
python3 docs/runbooks/startup-recovery-fixture.py cleanup "$repair_fixture"
unset repair_fixture
```

Cleanup removes only that verified fixture, including its alternate audio,
configuration backups and permission state. Close the scratch report buffer.
V1–V6 are accepted with preservation and cleanup confirmed. Task 006's acceptance
gate is closed; task 004 and inherited checks retain their separate gates.


## Task 007: Optional Tool Correction And Retry

Accepted — task 007 is complete. V1–V3 and preservation are accepted; the narrow
Library and Show card-overflow follow-ups are accepted with preservation and
cleanup. No startup fixtures remain in the checked temporary directories. The
packet records the correction of an unsupported extra gate inferred from a
port-only log. This procedure remains a regression check. It does not repeat
task 006's accepted editor checks or accept task 004's paused audio/publication
workflow.

Needs a Linux desktop, Python 3.11+, and this checkout's debug binary. No audio
hardware, installed mpv, reachable Index or real publisher is required. The
fixture runs a loopback Index server, rejects service mutations through local
stubs until explicitly released, and uses the Null player after correction.
Do not run the GUI as an agent. Use a fresh fixture for each V case. Run commands
from the repository root. Keep failed fixtures and their backups for diagnosis.

### Prepare Each Case

```bash
cargo build --quiet
retry_fixture=$(python3 docs/runbooks/startup-recovery-fixture.py setup)
python3 docs/runbooks/startup-recovery-fixture.py mode "$retry_fixture" retry-actions
python3 docs/runbooks/startup-recovery-fixture.py verify "$retry_fixture"
python3 docs/runbooks/startup-recovery-fixture.py retry-status "$retry_fixture"
python3 docs/runbooks/startup-recovery-fixture.py run "$retry_fixture"
```

Keep the printed directory and endpoint. While the app runs, use a second
terminal with the exact directory (replace the example):

```bash
retry_fixture=/tmp/v4vmm-startup-REPLACE_WITH_PRINTED_DIRECTORY
python3 docs/runbooks/startup-recovery-fixture.py retry-status "$retry_fixture"
```

The initial configuration has invalid `musicindex_endpoint`, `playback.driver`
and `flac_path` values, plus valid Local/default and Alternate/alternate
publisher hosts. The separate `flac_path` issue must survive these corrections.
The fixture records only HTTP GET requests and explicit service mutations;
passive service reads are not mutations. Empty Index results are intentional.

### V1 — Retry The Original Search

1. From Settings → Diagnostics, submit `first retained query` with the toolbar
   Search button. Music must become selected and show its search results. Return
   to Settings → Diagnostics, then use Ctrl+F, enter `second retained query`, and
   press Enter. Music must again become selected. Its breadcrumb must follow
   Music's navigation history, with no Settings ancestor. This is the visual
   regression check for the ADR 0060 search navigation correction. Both Index
   requests must fail locally. The shared tools notice must retain both queries;
   local library browsing must remain available.
   The search explanation must identify endpoint configuration/setup and direct
   the operator to Edit endpoint and `musicindex_endpoint`. It must not blame
   the background runtime. This is the regression check for the V1 screenshot's
   incorrect dependency explanation; the copied report must agree.
2. Choose **Edit endpoint** for the first query. Settings → Diagnostics must
   open the existing configuration editor at `musicindex_endpoint`. Set it to
   the endpoint printed by `retry-status`, as plain text without quotes.
   Choose **Test draft and paths**, then **Save correction**. The original
   backup path must remain in the editor report. Wait for the scoped tool check.
3. Run `retry-status` in the second terminal. `index_requests` and
   `service_commands` must still be empty. Save must not submit either query.
   The player and converter issues must remain. Returning to Music and browsing
   the fixture playlist must work while the editor draft/report is open.
4. In Settings → Diagnostics, scroll below the focused editor to **Background
   tools**. **Edit endpoint** opens the editor. **Check endpoint** validates the
   saved URL without sending a search. **Run search again** sends the retained
   query and requires a successful check. These effects must be visible beside
   the controls. In Music, **View search actions** opens these Settings controls
   without running a search. Choose **Run search again** for `first retained query`. Music must show that original query's empty
   Index result, even though the second query was the most recent selection.
   `retry-status` must contain search requests for the first query and none for
   the second. The search command may use more than one Index endpoint; inspect
   each request's query parameter. The completed query must offer only **Dismiss**
   in Music and Settings. Choose **Check endpoint** for the second pending query.
   The check must add no requests and must not restore repair/run controls for
   the completed first query.
5. Inspect the focused editor and retained-action report at normal/narrow widths
   and in Light/Dark using the immediate theme preview without saving. Actions,
   original subjects, inline explanations and result text must remain readable
   and reachable. A completed action must not look like a repair is needed. Copy
   the report: its subjects and recorded UTC times must match the app.
   Blocking manual guard (situational, ADR 0066): at about 560 pixels wide,
   inspect the converter row with two actions and the pending query with four.
   Buttons must move below the text when they cannot fit beside a readable text
   column, then wrap within the available width. One-character text columns,
   clipped explanations or unreachable buttons fail this check. Widen the window
   again; text and actions may share a line when they fit. Check the shared Music
   notices too. This guards the operator's narrow pending-row failure.

### V2 — Reject A Changed Playlist Subject

1. Prepare a fresh case and use a normal-width or maximized window. In Music,
   open the fixture playlist. Note the first track and choose **Repair playback**
   on that track's row below the playlist heading. The global Edit player settings
   notice does not retain a track action. The retained-action notice must name its
   track ID, playlist and original position. Choose **Edit player settings**; the
   editor must focus `playback.driver`.
2. Replace the entire focused `playback.driver` input with the four lowercase
   characters `null`, without quotes, a `value =` prefix or a newline. Before
   saving, return to Music and move the
   original first track down one position using the playlist's existing menu.
   Return to Settings; the unsaved player correction must remain. Choose Test
   draft and paths and require successful validation before Save correction.
   Require the editor report to say it saved the configuration correction
   and name the preserved backup. A validation-only result did not save the file;
   a failed Save leaves the correction unaccepted. The player check must finish
   without loading a track or creating a show session. The Index and converter
   issues must remain.
   The retained playback row must report a successful setup check and enable
   **Play original track**. If it stays disabled, choose **Check player** on that
   row and wait for its result. If still disabled, copy the Background tools
   report and keep the fixture: the changed-position rejection has not yet run.
3. Choose **Play original track** for the original playlist action. The report
   must reject the changed original position. Neither the new first track nor the old first
   track may start. A playback session or a silently substituted track fails
   this check. Restore the original playlist order before preservation inspection.
4. Click the original track's now-available Play action. This is a separate new
   action and may load the explicit Null player. It proves local command
   availability after repair; it proves nothing about audible playback or cue/
   audition separation. Quit before inspection and cleanup.

### V3 — Reject A Changed Publisher, Then Retry The Original

1. Prepare a fresh case. Open Show and choose **Start** on the publisher service
   for Local/default. The stub rejects it. The retained action must name Start,
   Local/default and the original event context. Its report must distinguish the
   command failure from the independent service observation.
2. Choose **Edit publisher settings** for that retained action. The shared editor
   must focus `broadcast.hosts`.
   Select the `broadcast.selected_host` field and change it to `Alternate`
   without quotes. Test and Save. The scoped check may read service state; it
   must not start a service. Run `retry-status`: the only mutation must still be
   the original failed Start for the default instance.
3. Choose **Start publisher** for the retained original action. It must reject the changed
   publisher context and add no service command. A Start for the alternate
   instance fails this check. Music/local browsing and independent setup tools
   must remain usable.
4. Choose **Edit publisher settings** for the original action again. Restore
   `broadcast.selected_host` to `Local`, then Test and Save. Permit the fixture's
   stub service commands from the second terminal:

```bash
python3 docs/runbooks/startup-recovery-fixture.py retry-release "$retry_fixture"
```

5. Choose **Check publisher**, then **Start publisher** on that original action.
   Check publisher must explain that it only reads service state. Only Start
   publisher may append another Start for the default instance. No command may
   target the alternate instance. The stub remains observably inactive; the app
   must report the successful command return separately from observed state.
   Repeated Save/check alone must never add service mutations. The unrelated
   Index/player/converter issues must remain.

### Narrow Library Follow-Up — ADR 0046

This is the remaining Library layout check from task 007, separate from accepted
V1–V3 behavior. Keep an existing follow-up fixture across correction builds; no
audio hardware or real service is needed. Quit and rerun it with the helper to
load a rebuilt normal binary. Use setup only if no follow-up fixture exists.

```bash
narrow_library_fixture=$(python3 docs/runbooks/startup-recovery-fixture.py setup)
python3 docs/runbooks/startup-recovery-fixture.py mode "$narrow_library_fixture" retry-actions
python3 docs/runbooks/startup-recovery-fixture.py run "$narrow_library_fixture"
```

1. In Music, open **Startup fixture playlist**. Resize the window to about
   463 pixels wide. The navigation pane must stack above the playlist detail;
   both must scroll independently. Scroll the detail to its title, playlist
   actions and all three track rows. Text must remain readable and row actions
   reachable. A one-character title column or content cut off to the right fails.
2. Drag the horizontal divider down to give navigation more height, then back
   up. Both panes must retain usable scrolling space; dragging must not select
   nearby text. Select an artist or album, then return to the playlist.
3. Widen the window. Drag the restored vertical divider to a comfortable sidebar
   width, narrow again, then widen. The preferred sidebar width must return when
   space permits; the stacked height preference must also survive the transition.
   At intermediate widths (about 570 pixels), track title/artist/duration must
   stay clear of Repair playback and the menu; actions wrap below when needed.
4. The recovery notice must use a bounded scroll area and leave room for the
   Library even with all three errors. Scroll to each issue and its repair/check
   controls. **View tools in Settings** must open Diagnostics without saving,
   checking or retrying; return to Music and retain the selected playlist.
   Repeat narrow/wide transitions in Light and Dark, and with a larger UI scale
   using preview only. The recovery notices must remain readable. Restore the
   original preview settings and playlist selection, then quit.
5. Inspect preservation:

```bash
python3 docs/runbooks/startup-recovery-fixture.py retry-inspect "$narrow_library_fixture"
```

Keep the fixture on any failure. After both visual checks and inspection pass:

```bash
python3 docs/runbooks/startup-recovery-fixture.py cleanup "$narrow_library_fixture"
test ! -e "$narrow_library_fixture"
```

Record visual acceptance, preservation and cleanup separately. This manual check
blocks task 007 closure until the operator inspects the correction.

### Show Card Overflow Follow-Up — ADR 0073

The Library and Show visual rechecks, follow-up preservation and cleanup are
accepted; directory absence is confirmed. ADR 0073 is Implemented. This section
remains a regression procedure. For a future regression, use a fixture from the
preceding Library procedure and leave its three configuration issues present. No audio hardware
or real service is required. Quit the app and relaunch from its existing terminal:

```bash
python3 docs/runbooks/startup-recovery-fixture.py run "$narrow_library_fixture"
```

1. Open Show with logs closed. Use a narrow, short window like the reported
   screenshot. Scroll over the cards to reach Source, Live Metadata and Stream,
   then back to Source. All three cards must remain selectable. A lower card
   clipped behind the transport or unreachable by scrolling fails.
2. Widen enough to open Live Metadata's Event Logs, then collapse the side panel
   and return to the narrow window. The log must retain its existing height
   priority; scroll the cards above it. Close the log and
   scroll all three cards again. Sidebar controls must remain independently
   reachable, and scrolling cards must not move the transport.
3. Widen and shorten the window, then repeat in Light/Dark and a larger UI
   scale preview. Restore the original theme/scale preview and Music selection.
   Quit the app and inspect the existing fixture:

```bash
python3 docs/runbooks/startup-recovery-fixture.py retry-inspect "$narrow_library_fixture"
```

Keep the fixture if any check fails. After visual acceptance and preservation
pass, clean it up and confirm absence:

```bash
python3 docs/runbooks/startup-recovery-fixture.py cleanup "$narrow_library_fixture"
test ! -e "$narrow_library_fixture"
```

This closes only the focused card-overflow gate. The completed shared-log packet
stays closed. Task 007 acceptance and fixture reconciliation are recorded in its packet.

### Preservation And Cleanup For Each Case

Quit the fixture app. Use its exact directory in the terminal that owns the
`retry_fixture` variable:

```bash
python3 docs/runbooks/startup-recovery-fixture.py retry-inspect "$retry_fixture"
```

Require original bytes preserved in the current document or an owner-only backup,
only the named endpoint/player/host edits and ordinary workspace preferences,
unchanged music/bindings/library/migrations, and no residual probe/candidate.
The inspector rejects unrelated configuration changes. On failure, keep the
fixture and paste the report. `configuration_checks` identifies each condition;
`unexpected_setting_paths` names changed fields without printing their values.
The endpoint check accepts surrounding whitespace and trailing slashes for the
exact fixture URL, matching the app's endpoint normalization. The separate format
flag can be true on a passing report. Changes to the scheme, host, port, path,
credentials, query or fragment still fail.
Do not reset the case, edit the evidence or run cleanup after a failed inspection.

Only after preservation passes, run cleanup:

```bash
python3 docs/runbooks/startup-recovery-fixture.py cleanup "$retry_fixture"
test ! -e "$retry_fixture"
```

Cleanup stops only the owned loopback server and removes its fixture directory. Record V1–V3,
preservation and cleanup separately; a backend fixture smoke check does not
close this operator gate.


## Task 008: Converter Verification And Setup

Accepted - 2026-09-17. V1–V3, Settings/core-recovery presentation, preservation
in both cases and fixture cleanup are accepted in the
[task packet](../tasks/adr-0066-task-008-converter-verification-and-setup.md#final-operator-acceptance-and-cleanup--2026-09-17).
This procedure remains a regression check for converter setup and version
verification; track conversion retry belongs to task 009.
Needs a Linux desktop, Python 3.11+,
`/bin/sh`, `/bin/sleep`, and this checkout's debug binary. No audio hardware,
installed FLAC/ffmpeg, external host, or failed system service is required.
The fixture supplies executable stubs, Null playback and service stubs. Its
invalid MusicIndex setting keeps unrelated configuration persistence paused.

### V1 — Missing Tools Become Available In The Same App

1. Create the isolated fixture and start it from a desktop terminal.

```bash
cargo build --quiet --bin v4vmm
converter_fixture=$(python3 docs/runbooks/startup-recovery-fixture.py setup)
python3 docs/runbooks/startup-recovery-fixture.py mode "$converter_fixture" converter-setup
python3 docs/runbooks/startup-recovery-fixture.py verify "$converter_fixture"
python3 docs/runbooks/startup-recovery-fixture.py run "$converter_fixture"
```

2. Open Settings → Library → Converter setup. This routes to the shared
   configuration editor with Converter setup selected. Leave the FLAC draft
   blank and press **Test converters**. Both executables must be reported
   missing on PATH, with recorded UTC times. Music and navigation stay usable.
   The installation guidance must name distribution package tools without an
   invented installation command. No Retry download control belongs here.

3. In a second desktop terminal, recover the exact directory printed above.
   `locate` returns the newest verified fixture; compare it with that directory
   before continuing if other fixtures exist.

```bash
converter_fixture=$(python3 docs/runbooks/startup-recovery-fixture.py locate)
python3 docs/runbooks/startup-recovery-fixture.py verify "$converter_fixture"
python3 docs/runbooks/startup-recovery-fixture.py converter-tools "$converter_fixture" working
python3 docs/runbooks/startup-recovery-fixture.py converter-status "$converter_fixture"
```

4. Without closing the app, press **Test converters** again. Both checks must
   succeed with exit 0 and fresh timestamps. The report identifies FLAC first
   and ffmpeg fallback. Old observations remain readable in the repair report.
   Copy the report and paste it into an editor; both executable sources and
   recorded times must be present. A stale missing result is wrong.

### V2 — Explicit Path And Guarded Save

1. Keep the same app open. Make only ffmpeg available; the status command prints
   a `missing_test_path` and a `configured_test_path` for the next steps.

```bash
python3 docs/runbooks/startup-recovery-fixture.py converter-tools "$converter_fixture" fallback
python3 docs/runbooks/startup-recovery-fixture.py converter-status "$converter_fixture"
```

2. Enter the printed `missing_test_path` in the FLAC draft. Editing alone must
   not add a probe record or change the saved file. Press **Test converters**:
   FLAC must fail at the configured path and ffmpeg must succeed on PATH.
   The report must describe the existing WAV download fallback, never claim
   all WAV downloads failed. **Save correction** must name the preserved
   original-file backup and must not add converter invocations to
   `converter-status`. The source path displayed above the input is explicitly
   the setting at editor load time; the result names the tested draft.

3. Choose **Reload file (discard draft)**, then select **Converter setup** if
   the editor selects the remaining MusicIndex issue. Enter the printed
   `configured_test_path`. Make the executables available and press Test again:

```bash
python3 docs/runbooks/startup-recovery-fixture.py converter-tools "$converter_fixture" working
```

4. FLAC must now succeed at the new configured path. Save correction must
   preserve the preceding configuration. The report must remain visible in
   place. Reload, select Converter setup, clear the FLAC draft, and save once
   more to restore the original unset path. Leave the unrelated MusicIndex
   issue unchanged. Select Converter setup again after any reload before the
   next test. Saving never downloads or converts a track.

### V3 — Bounded Failures And Shared Recovery Access

1. Keep the app open and switch the stubs to timeout mode. Press Test converters.
   Each probe has a five-second limit, so both checks together take about ten
   seconds. During the wait, navigate to another Settings group and back, resize,
   and close/reopen the editor. Completion must leave a closed editor closed.

```bash
python3 docs/runbooks/startup-recovery-fixture.py converter-tools "$converter_fixture" timeout
```

2. The report must name each timed-out executable and process cleanup. Test
   each remaining mode using the corresponding command below. Require distinct
   exit-7, permission-denied and output-limit reports. The failed executable's
   printed secret must not appear in the report, copied text or app stderr.
   Inspect the report and controls at normal and narrow window widths: clipped
   subject text, overlapping actions, or unreachable Test/Save controls are wrong.

```bash
python3 docs/runbooks/startup-recovery-fixture.py converter-tools "$converter_fixture" nonzero
```

```bash
python3 docs/runbooks/startup-recovery-fixture.py converter-tools "$converter_fixture" permission
```

```bash
python3 docs/runbooks/startup-recovery-fixture.py converter-tools "$converter_fixture" output-limit
```

3. Restore working stubs and press Test once more, then quit the app. Verify
   original bytes/backups, restored unset FLAC path, unchanged other settings,
   music/library/migrations, version-only invocations, reaped children and no
   candidates/probes. All eight converter checks and the shared preservation
   checks must pass. `config_bytes_unchanged` may be false after a guarded Save
   when the original bytes are preserved in a backup; this alone is not a failure.

```bash
python3 docs/runbooks/startup-recovery-fixture.py converter-tools "$converter_fixture" working
python3 docs/runbooks/startup-recovery-fixture.py converter-inspect "$converter_fixture"
```

4. Only after that preservation check passes, switch to core recovery. The
   fixture's empty music setting prevents core startup. Open Configuration
   repair → Edit configuration → Converter setup. Test once with both tools
   missing, then change to working in the second terminal and Test again.
   Both observed results must update in this recovery window without a normal
   app session. Do not save or correct the deliberately broken core setting.

```bash
python3 docs/runbooks/startup-recovery-fixture.py mode "$converter_fixture" converter-recovery
python3 docs/runbooks/startup-recovery-fixture.py run "$converter_fixture"
```

```bash
python3 docs/runbooks/startup-recovery-fixture.py converter-tools "$converter_fixture" working
```

### Preservation And Cleanup

Quit recovery (its failure exit is expected while the core setting is invalid).
Inspect before resetting or deleting anything. If either inspection fails,
retain the fixture and report its output; do not reset its case or evidence.

```bash
python3 docs/runbooks/startup-recovery-fixture.py converter-inspect "$converter_fixture"
```

After preservation passes, restore the fixture configuration and executable
search environment, then remove the fixture:

```bash
python3 docs/runbooks/startup-recovery-fixture.py mode "$converter_fixture" normal
python3 docs/runbooks/startup-recovery-fixture.py cleanup "$converter_fixture"
test ! -e "$converter_fixture"
```

The launcher changes PATH only for its child process; the desktop shell PATH
and real configuration are never changed. Record V1, V2, V3, Settings/recovery
presentation, preservation and cleanup separately. Mechanical checks do not
accept these observations.

## Task 009: Conversion Retry And Retained Input

Status: Accepted - 2026-09-17. V1–V3, normal/narrow presentation, configuration
restoration and preservation passed; fixture cleanup is confirmed. Retain this
procedure as a regression check. The
[packet records the evidence](../tasks/adr-0066-task-009-conversion-retry-and-retained-input.md#final-operator-acceptance-and-cleanup--2026-09-17).
Owner: [task 009](../tasks/adr-0066-task-009-conversion-retry-and-retained-input.md).

This checks the new retained conversion controls and their results in the existing
Music/Settings surfaces. Use a Linux desktop terminal, Python 3 and the normal
debug binary. The fixture supplies a loopback audio server, two conversion tracks,
private configuration/database/music and deterministic FLAC/ffmpeg executables.
It explicitly selects Null playback. No audio hardware, installed converter,
external service or real library is needed. Do not play these encoding stubs.
The fixture changes no real settings or service units.

### V1 — A Conversion Warning Keeps A Usable WAV

1. Create a fresh isolated fixture and open it. Keep this terminal open.

```bash
cd /home/citizen/build/v4vmm
cargo build --quiet --bin v4vmm
conversion_fixture="$(python3 docs/runbooks/startup-recovery-fixture.py setup)"
python3 docs/runbooks/startup-recovery-fixture.py mode "$conversion_fixture" conversion-retry
python3 docs/runbooks/startup-recovery-fixture.py run "$conversion_fixture"
```

2. In Music, open **Startup fixture playlist**, then click the **Conversion
   retry** title to open its details. Use **Download Track**. Both converters pass version checks but
   reject encoding and create failed partial output. The result must name
   **Conversion retry**, explain that the usable WAV is in the library, and offer
   **Edit converter setting**. A failed-download claim, missing repair route,
   or the fixture's secret sentinel in a report is wrong.
3. In a second terminal, locate this fixture and inspect the original request.
   Expect one `/conversion-4.wav` request, four file bindings and five playlist
   rows. No failed FLAC output should remain in staging.

```bash
cd /home/citizen/build/v4vmm
conversion_fixture="$(python3 docs/runbooks/startup-recovery-fixture.py locate)"
python3 docs/runbooks/startup-recovery-fixture.py conversion-status "$conversion_fixture"
```

### V2 — Setup Returns To The Original Track

1. Open **Edit converter setting** from that retained result. Change the fixture
   executables while the same app stays open:

```bash
python3 docs/runbooks/startup-recovery-fixture.py conversion-tools "$conversion_fixture" working
printf '%s/bin/flac\n' "$conversion_fixture"
```

2. Put the printed absolute path in the converter field. Use **Test converters**,
   then **Save correction**. Expect successful fresh version checks and a named
   configuration backup. Test and Save must not download or convert the track.
   Use `conversion-status` again: the request count and WAV binding stay unchanged.
3. Open **Background tools** in Settings. The retained **Conversion retry** result
   must still be present. Select **Check converter setting**, then **Retry original
   conversion**. Do not find the track again. While retry runs, repeated clicks
   must not start another operation. Expect a FLAC success report naming the
   original track and one updated binding. The Music row must update when it is
   mounted. Choosing a different track before retry must not change its subject.
4. Run the inspection command. Expect the same single `/conversion-4.wav` request,
   a `.flac` binding for track 4, four bindings total, and five playlist rows.

```bash
python3 docs/runbooks/startup-recovery-fixture.py conversion-status "$conversion_fixture"
```

5. Inspect the new result/actions at normal and narrow widths. Text must remain
   readable, actions reachable, and Copy report must include the original track,
   actual outcome, path and recorded UTC time. The completed action must offer
   Dismiss without another Retry. This checks the new conversion content only;
   previously accepted general editor/log behavior does not need another pass.

### V3 — Missing Input Requires Explicit Redownload; Fallback Is Successful

1. Restore failed encoding, then download **Conversion redownload** from the same
   playlist. Expect a usable-WAV warning for this second track.

```bash
python3 docs/runbooks/startup-recovery-fixture.py conversion-tools "$conversion_fixture" encode-failure
```

2. Move only that fixture track's retained WAV out of its expected location and
   enable working ffmpeg fallback. The helper preserves the moved bytes.

```bash
python3 docs/runbooks/startup-recovery-fixture.py conversion-remove-input "$conversion_fixture"
python3 docs/runbooks/startup-recovery-fixture.py conversion-tools "$conversion_fixture" fallback
```

3. In Background tools, **Check converter setting**, then **Retry original
   conversion** for **Conversion redownload**. The result must explain the missing
   original input and offer **Redownload original track**. It must not fetch yet.
   `conversion-status` must still show one `/conversion-5.wav` request.
4. Check the converter setting again and explicitly select **Redownload original
   track**. Expect ffmpeg fallback success, with FLAC rejection distinguished
   from failure of the whole download. Expect exactly two `/conversion-5.wav`
   requests, one `/conversion-4.wav` request, five bindings and five playlist rows.
   No unrelated track may be downloaded or added to the playlist.
5. Open **Settings → Library → Converter setup**. Use **Reload file (discard
   draft)** to edit the saved configuration, clear the configured FLAC path and
   **Save correction**, restoring the fixture's original unset/PATH setting.
   Saving must not repeat
   either conversion. Copy the report and confirm the secret sentinel is absent.
   Close the fixture app normally.

### Preservation And Cleanup

1. Inspect after closing the app. Every preservation flag must be true. This
   verifies original audio, configuration revisions/backups, migrations,
   original bindings, one binding per track, unchanged playlist count, exact
   enclosure requests, child cleanup and released staging. The original usable
   track 4 WAV is preserved on disk; its sole active binding points to FLAC.
   Retention belongs to one session and makes no crash/relaunch promise.

```bash
python3 docs/runbooks/startup-recovery-fixture.py conversion-inspect "$conversion_fixture"
```

2. Report V1–V3 and preservation results. Keep a failed fixture for diagnosis.
   After a passing inspection, remove the fixture and its owned server, then
   confirm the directory is absent. No real service or hardware cleanup is needed.

```bash
python3 docs/runbooks/startup-recovery-fixture.py cleanup "$conversion_fixture"
test ! -e "$conversion_fixture" && echo "Fixture removed"
```

## Task 010: Database Check And Backup

Status: accepted on 2026-09-17; V1–V3, Settings/recovery presentation, report copy,
responsiveness, preservation in both cases, normal-mode restoration and cleanup
are confirmed. Retained as a regression procedure; mechanical results alone do
not accept an operator check.

Purpose: inspect and back up disposable databases from Settings and core recovery.
Use a Linux desktop terminal, Python 3.11+ and this checkout's debug binary.
No audio hardware, installed converter or real external service is needed. The
fixture uses Null playback and failing service stubs. Its owned helper keeps a
committed row in WAL and a separate database exclusively locked. Keep that
helper running until the preservation inspection finishes.

### V1 — Settings And A New Backup

1. Create the fixture and open its normal app. Only the operator runs `run`.

```bash
cd /home/citizen/build/v4vmm
cargo build --locked --offline --quiet --bin v4vmm
fixture=$(python3 -B docs/runbooks/startup-recovery-fixture.py setup)
python3 -B docs/runbooks/startup-recovery-fixture.py mode "$fixture" database-tools
python3 -B docs/runbooks/startup-recovery-fixture.py run "$fixture"
```

2. Open **Settings → Diagnostics → Database tools**. Choose **Use configured
   database**, then **Check database**. Expect the configured fixture library
   path, separate Access/Integrity/Foreign keys/Schema results and recorded UTC.
   The check must say it did not initialize, migrate or repair the source.
   The read-only integrity result explains SQLite's CHECK-constraint limit;
   backup validation includes those constraints on its private candidate.
3. Enter the printed `normal_backup` path as **New backup file path**, then
   choose **Back up database**. Expect a verified snapshot at that exact path,
   with no foreign-key violations and a compatible schema. The explanation must
   exclude music and broadcaster token files. Navigation must remain usable.
4. Copy the database report and paste into a text editor. Confirm full paths,
   separate results and actual UTC times. At normal and narrow window widths,
   both path fields, actions and report remain reachable without overlapping.
   A success message before completion, truncated copied paths or an unexplained
   disabled control counts as wrong. Keep the app open for V2.

### V2 — WAL, Read-Only Source And No Overwrite

1. In a second terminal, recover the same fixture path and list its test paths.

```bash
cd /home/citizen/build/v4vmm
fixture=$(python3 -B docs/runbooks/startup-recovery-fixture.py locate)
python3 -B docs/runbooks/startup-recovery-fixture.py database-status "$fixture"
```

   Confirm `helper_running` and `exclusive_lock_blocks_reader` are both true.
   A helper process alone does not prove the exclusive lock is held.

2. In Database tools, enter the printed `wal` source and `wal_backup`
   destination. Check the source, then back it up while the helper remains
   running. Repeat with the `readonly` source and `readonly_backup` destination.
   Both backups must complete. A request to end the normal app session or make
   the read-only source writable is wrong.
3. Select the printed `occupied_destination` and attempt another backup.
   Expect refusal and a new-filename instruction, with no completed-backup
   claim for this attempt. Repeat with the already completed `wal_backup` path;
   it must also be refused. The earlier successful report may remain in history.
4. Close the app, then inspect the sources and completed snapshots.

```bash
python3 -B docs/runbooks/startup-recovery-fixture.py database-inspect "$fixture"
```

Every database preservation flag must be true, including
`committed_wal_row_in_backup`, `wal_bytes_preserved`, private backup permissions,
unchanged source files and no incomplete candidates. Shared library, music,
configuration and migration checks must also pass. The raw main database did
not contain the WAL marker when the helper committed it, so the snapshot row is
proof of WAL inclusion. Keep the fixture and helper for V3.

### V3 — Core Recovery And Distinct Failures

If an existing fixture reports `exclusive_lock_blocks_reader: false`, keep its
original WAL helper running. In a second terminal, restore only the lock:

```bash
cd /home/citizen/build/v4vmm
fixture=$(python3 -B docs/runbooks/startup-recovery-fixture.py locate)
python3 -B docs/runbooks/startup-recovery-fixture.py database-lock "$fixture"
```

Wait for **Fixture database lock held** and leave that terminal running. The
command checks the existing source hash, does not replace the baseline or
restart the WAL helper, and holds an exclusive transaction without writing data.
Normal fixture cleanup stops this additional helper. This correction is for
helpers started before the task 010 checksum/lock-order fix; accepted checks
need not be repeated. Recheck the locked source before timeout/cancellation.

1. Switch only after closing the normal fixture window. The new configured
   database has an invalid header; recovery must expose the same Database tools.

```bash
python3 -B docs/runbooks/startup-recovery-fixture.py mode "$fixture" database-recovery
python3 -B docs/runbooks/startup-recovery-fixture.py run "$fixture"
```

2. Choose **Use configured database**, then **Check database**. Expect the
   invalid-header path, a database-read failure and later checks marked as not
   checked. No empty replacement library may open. Select the `readonly` source
   manually and check it to establish that these tools work without the normal
   database or app runtime. Back up that source to the new absolute path
   `$fixture/database/recovery-backup.sqlite` (expand the fixture variable when
   entering it). Expect a verified backup from this same recovery window.
3. Use the paths printed by `database-status` to check these sources separately:

| Source | Expected report |
|---|---|
| `integrity` | Database access succeeds; integrity reports a damaged freelist. Source stays unchanged. |
| `foreign-key` | Integrity returns ok; foreign-key violations are reported separately. |
| `newer` | Integrity returns ok; migration 999 needs a compatible app, without calling unfamiliar schema corrupt. |
| `older` | A supported older ledger needs an explicit upgrade; Check does not apply one. |
| `locked` | A bounded Busy/Locked access result names the source and suggests closing the conflicting writer and checking again. |

4. For `locked`, use a new destination such as
   `$fixture/database/locked-backup.sqlite` (enter the expanded absolute path,
   not the shell variable). Choose **Back up database** and wait for its
   one-minute limit. Resize and interact with recovery while it runs. Expect a
   Busy/Locked failure, no completed snapshot and usable controls afterward.
   Repeat and choose **Cancel** while it runs. Expect a cancellation request,
   then a completion report; cancellation must not claim a verified backup.
5. Check normal/narrow recovery presentation and copy/paste its report. The
   selected source, failure, consequence and next action must remain readable;
   controls and the report must remain reachable. Close recovery using **Quit**.
   A nonzero app exit is expected because this case never resumed normal startup.

### Preservation And Cleanup

Inspect after closing the recovery window and before stopping the WAL helper.
Do not clean up a failed inspection; retain it for diagnosis.

```bash
python3 -B docs/runbooks/startup-recovery-fixture.py database-inspect "$fixture"
```

Record V1–V3, normal/narrow presentation, report-copy results and all preservation
flags in task 010. After they pass, restore normal configuration and remove only
this fixture. Cleanup terminates its owned WAL/lock helper and removes its
private databases, backups, configuration and stubs.

```bash
python3 -B docs/runbooks/startup-recovery-fixture.py mode "$fixture" normal
python3 -B docs/runbooks/startup-recovery-fixture.py cleanup "$fixture"
test ! -e "$fixture" && echo "Fixture removed"
```

Task 011 stays unstarted. These checks do not accept task 004 or inherited UI gates.
