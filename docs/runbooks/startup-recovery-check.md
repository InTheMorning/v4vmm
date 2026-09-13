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
all three rows remain browsable. Play actions must be unavailable; Ctrl+Alt+P on Show
must not start audio. Navigate back to Settings and confirm both reports remain.
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
