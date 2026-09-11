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
Operator acceptance is open. These checks prove that failed background tools
do not prevent startup, and that Check again repairs them in the same window.
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
to complete the remaining keyboard acceptance without repeating accepted checks.

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
