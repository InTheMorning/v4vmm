# Event Recovery Visual Check

Status: Passed - 2026-09-09. Operator acceptance met for
[ADR 0059 task 016](../tasks/adr-0059-task-016-event-row-in-live-metadata.md).

The operator confirmed all broadcast event recovery tests pass, including
Create, Replace, stored Unknown and retry, explicit Attach, feed-tag Copy,
service readiness, resizing, and registry/token preservation and command
separation. This procedure remains the situational ADR 0059 manual regression
check for future changes; no acceptance check remains open for task 016.

## Prerequisites

Use a Linux desktop session, Python 3.11 or newer, and two terminals. Port `17863` must be
free. Build the current checkout with `cargo build`.

The [fixture](broadcast-event-recovery-fixture.py) supplies a local relay,
a reachable publisher with an initially empty target list, and two simulated
running services. It creates an isolated app database and token directory.
No installed publisher, encoder, audio hardware, or real service changes are
needed. Its `systemctl` and publisher executables affect only the app process
launched with the environment below. Do not substitute your normal app config.

## Current Acceptance Walkthrough

Task 016's operator acceptance remains complete. For the new compact layout,
use [task 017's walkthrough](../tasks/adr-0059-task-017-compact-event-controls-and-badges.md#operator-visual-check).
It covers the picker, item badges, and Event Logs as well as recovery.
Event Logs now names the action, event, response, and saved result in timestamped
sentences. It omits idle operations. Action times remain unchanged when Logs
reopens; these session results do not survive an app restart.

The shorter recovery sequence below remains available for regression checks.
It uses the current compact controls: event paths and results are in **Logs**;
**Check again**, **Detach**, and **Copy feed tag** are under **More**.

## Fixture Modes

Run these in terminal B with the same `task016_dir`. They change fixture
responses or configuration. They do not refresh the app by themselves.

| Command after `python3 docs/runbooks/broadcast-event-recovery-fixture.py` | Purpose | Then do this in the app |
|---|---|---|
| `mode "$task016_dir" live` | The relay returns metadata for the checked event after its first check. | Press Retry check, or More → Check again. |
| `mode "$task016_dir" dead` | The next check reports the event gone. | Press Retry check, or More → Check again. |
| `mode "$task016_dir" fail` | The relay returns HTTP 503 for the checked event. | Press Retry check, or More → Check again. |
| `create-mode "$task016_dir" fail` | Registration fails without allocating an ID. | Press Create or Replace when available. |
| `create-mode "$task016_dir" live` | Registration works again. | Press Create or Replace when available. |
| `producer-state "$task016_dir" inactive` | Stop the simulated Producer. | Wait for its badge, then Start. |
| `publisher-state "$task016_dir" failed` | Fail the simulated Publisher. | Wait for its badge, then Reset and Start. |
| `journal-mode "$task016_dir" slow-fail` | Service-log reads fail after five seconds. | Open Publisher Logs, then Event Logs while the read is pending; the late failure must leave Event Logs visible. |
| `journal-mode "$task016_dir" slow` | Service-log reads succeed after five seconds. | Exercise repeated Logs presses and switching services as described in task 017 step 9. |
| `journal-mode "$task016_dir" normal` | New service-log reads return immediately again. | Open a service's Logs to read its fixture journal. |
| `target-scope "$task016_dir"` | Prepare task 017 step 6: default uses event 1; unused uses your selected replacement event. The command checks isolation and reads saved event/token paths. | Select the replacement before running it; then More → Check again must show Not attached. |

Both service-state commands accept `active`, `inactive`, or `failed`.
Every event's first check deliberately fails, even in `live` mode. Later checks
use the selected mode. Checks take two seconds so their progress is visible.
`journalctl` is simulated too; service Logs identify the requested unit.
Journal mode defaults to normal in existing and fresh fixture directories.
No setup, app restart, or relay restart is needed to change it. Each request
keeps the journal mode it read when it started; changing the mode affects new
requests. Task 017's [delayed service-log checks](../tasks/adr-0059-task-017-compact-event-controls-and-badges.md#delayed-service-log-checks)
cover switching, closing, and repeated reads. Restore journal mode normal
after those checks. These modes do not change the event relay or service states.
Target add/remove affect only the named target and preserve unrelated entries.

Non-GUI regression checks:

```bash
python3 docs/runbooks/test_broadcast_event_recovery_fixture.py
```

## Operator Visual Check

1. In terminal A, prepare the fixture and keep its relay running:

   ```bash
   cd /home/citizen/build/v4vmm
   cargo build
   task016_dir=$(mktemp -d /tmp/v4vmm-task016.XXXXXX)
   python3 docs/runbooks/broadcast-event-recovery-fixture.py setup "$task016_dir"
   python3 docs/runbooks/broadcast-event-recovery-fixture.py serve "$task016_dir"
   ```

   Setup prints the directory. Shell variables do not carry from terminal A
   into terminal B. In terminal B, recover the existing directory before
   launching the app:

   ```bash
   cd /home/citizen/build/v4vmm
   if task016_dir=$(python3 docs/runbooks/broadcast-event-recovery-fixture.py locate) &&
      python3 docs/runbooks/broadcast-event-recovery-fixture.py verify "$task016_dir"; then
     env XDG_CONFIG_HOME="$task016_dir/config" PATH="$task016_dir/bin:$PATH" \
       ./target/debug/v4vmm &
     task016_app_pid=$!
   fi
   ```

   Keep terminal A running throughout this check. Restarting the fixture relay
   resets its identifier counter; start a fresh fixture directory if it exits.
   If more than one fixture exists, `locate` lists them and stops. Use the exact
   directory printed by the running relay, then run `verify` before executing
   the `env` launch command with that directory assigned to `task016_dir`.

2. Before pressing an event or service action, confirm the Source card says
   **Task 016 fixture**, the new fixture library is empty, and Settings shows
   the endpoint **http://127.0.0.1:17863**. A Source named **Local**, existing
   library tracks, or **https://api.musicindex.org** means this is a different
   app configuration. Close that window and launch using the verified command
   in step 1. Changing a shell variable does not change a running app's config.

   Open **Show**, then **Live Metadata**. Expect exactly three cards: Source,
   Live Metadata, Stream. Their heights match. The Live Metadata card says
   `Event: No event` and `Live Metadata: not ready` even though both fixture
   services are running. In the detail, Event precedes Producer and Publisher.
   Create is the primary action; there is no selected event to check or replace. A separate Event
   card, a ready card, or services preceding Event is wrong.

3. Press **Create** once. The new `fixture-event-1` appears in the picker. Open Event Logs to see
   its full ID and token path. The first check takes two seconds, then fails
   deliberately. Registration remains successful, the stored liveness stays Unknown and its badge says Check failed,
   and **Retry check** becomes available. Create, Replace, and Attach remain
   unavailable. Press Retry check while the fixture still fails: progress must
   be visible, duplicate commands unavailable, then retry available again.
   Losing the identity/path, reporting registration failure, offering Attach,
   or requiring navigation to see results is wrong.

4. Close only the app and relaunch it with the same terminal B command from
   step 1. Show must retain `fixture-event-1` and its token path. The app checks the restored selection. Expect the deliberate failure and
   Retry check again. The check creates no event.

5. In terminal B, change the relay answer, then press **Retry check** in Show:

   ```bash
   python3 docs/runbooks/broadcast-event-recovery-fixture.py mode "$task016_dir" dead
   ```

   Expect Dead in the same row and Replace available, with Create unavailable.
   The card remains not ready. Press **Replace** once. Expect a new
   `fixture-event-2` and new token path in Logs immediately, followed by the deliberate
   initial check failure. Registration still succeeds; Retry check returns.
   Replacement must not attach the new event.

6. Restore a successful relay answer, then press **Retry check**:

   ```bash
   python3 docs/runbooks/broadcast-event-recovery-fixture.py mode "$task016_dir" live
   ```

   Expect `fixture-event-2` to become Live in place, with the same token path.
   The target read confirms it is not attached. Attach becomes available; the
   card still says not ready. Press **Attach** explicitly. After target refresh,
   expect the event attached to `default`, then the card ready with both services
   Active. Choose More → Copy feed tag and paste into a text editor: expect exactly
   `<podcast:liveValue uri="fixture-event-2" protocol="socket.io"/>`.

7. Exercise service readiness with the live, attached event:

   ```bash
   python3 docs/runbooks/broadcast-event-recovery-fixture.py producer-state "$task016_dir" inactive
   ```

   Within the next service observation, the card must name `Producer: Inactive`
   and say not ready. Start is available in the Producer row. Press it to
   restore the simulated service, then repeat with a failure:

   ```bash
   python3 docs/runbooks/broadcast-event-recovery-fixture.py producer-state "$task016_dir" failed
   ```

   Expect the card's failed state and `Producer: Failed`. Press Reset, then Start, on the simulated Producer to return to ready. The card must keep its height through
   all transitions. Resize the window across one, two, and three columns; open
   and close the side panel. All cards remain visible, detail scrolls, and
   transport stays accessible. Unreadable column text or a growing card is wrong.

8. Check preservation and command separation in terminal B:

   ```bash
   env XDG_CONFIG_HOME="$task016_dir/config" PATH="$task016_dir/bin:$PATH" \
     ./target/debug/v4vmm broadcast events list --json
   cat "$task016_dir/calls.jsonl"
   ```

   Expect two registry entries: `fixture-event-1` remains Dead and
   `fixture-event-2` is Live. Both token files remain present. The fixture log
   has exactly two `register` entries, checks keep their respective event IDs,
   and `target add` occurs only after your explicit Attach in step 6. It must
   contain no token text. This fixture verifies mouse interaction and display;
   it does not establish connectivity to a production relay or publisher.

## Recover A Missing Directory Variable

If a later command reports an empty or invalid fixture directory, recover it
in that terminal and repeat the requested operation:

```bash
task016_dir=$(python3 docs/runbooks/broadcast-event-recovery-fixture.py locate) &&
  python3 docs/runbooks/broadcast-event-recovery-fixture.py mode "$task016_dir" dead
```

This reuses the existing fixture. Leave the relay and app running so their
event identities and progress survive. `locate` refuses to choose between
multiple fixtures; in that case use the path printed in terminal A explicitly.
If no fixture exists, complete setup in step 1 first.

## Recover A Window Using The Wrong Configuration

The 2026-09-09 screenshot showed `https://api.musicindex.org`, Source `Local`,
and a token path containing the literal `REPLACE_WITH_PRINTED_SUFFIX` from the
first walkthrough. That launch created a default config, which selected the
production endpoint and normal database. Changing the fixture relay cannot
change that event's status. Recovering `task016_dir` in a terminal does not
reconfigure the already-running app.

Close that app window and keep the actual fixture relay running. Recover the
fixture directory with `locate`, then reset its response for the first check:

```bash
task016_dir=$(python3 docs/runbooks/broadcast-event-recovery-fixture.py locate) &&
  python3 docs/runbooks/broadcast-event-recovery-fixture.py mode "$task016_dir" fail
```

Execute the verified launch block in step 1. Repeat the configuration checks in step 2
before continuing the fixture scenario. Keep the mistakenly created placeholder
directory: it contains a production event's token file, and its registry entry
may refer to that path. The fixture cleanup below applies only to a prepared
fixture directory with the marker file.

## Fixture Directory Regression Check

Situational, ADR 0059. Added after the 2026-09-09 operator check encountered
the generic `Not a task 016 fixture directory` error. Run this check whenever
fixture directory handling changes. Empty input must not resolve to the current
directory, invalid paths must identify themselves, and discovery must refuse
both absence and ambiguity. The 2026-09-09 wrong-window follow-up also requires
launch verification to reject production endpoints, shared database paths,
wrong hosts, and missing command stubs. The check uses temporary files and cleans them up;
it launches neither the desktop app nor a relay.

```bash
python3 - <<'PY'
from pathlib import Path
import contextlib
import io
import runpy
import subprocess
import sys
import tempfile

script = Path("docs/runbooks/broadcast-event-recovery-fixture.py").resolve()
fixture = runpy.run_path(str(script))
locate = fixture["locate_fixture"]
with tempfile.TemporaryDirectory() as directory:
    root = Path(directory)
    with contextlib.redirect_stdout(io.StringIO()):
        fixture["setup"](root)
    fixture["verify_fixture"](root)
    result = subprocess.run([sys.executable, str(script), "verify", str(root)],
                            capture_output=True, text=True, check=True)
    assert "Fixture verified" in result.stdout
    config_path = root / "config/v4vmm/config.toml"
    original = config_path.read_text()
    for before, after in [("http://127.0.0.1:17863", "https://api.musicindex.org"),
                          (str(root / "app.sqlite"), "/tmp/normal-app.sqlite"),
                          ("Task 016 fixture", "Local")]:
        config_path.write_text(original.replace(before, after))
        result = subprocess.run([sys.executable, str(script), "verify", str(root)],
                                capture_output=True, text=True)
        assert result.returncode != 0 and "config mismatch" in result.stderr, result.stderr
    config_path.write_text(original)
    stub = root / "bin/systemctl"
    stub.chmod(0o600)
    result = subprocess.run([sys.executable, str(script), "verify", str(root)],
                            capture_output=True, text=True)
    assert result.returncode != 0 and "stub" in result.stderr, result.stderr
    stub.chmod(0o700)
    fixture["verify_fixture"](root)
    for value, expected in [("", "directory is empty"),
                            (str(root / "missing"), str(root / "missing"))]:
        result = subprocess.run([sys.executable, str(script), "mode", value, "dead"],
                                cwd=root, capture_output=True, text=True)
        assert result.returncode != 0 and expected in result.stderr, result.stderr
        assert (root / "mode").read_text() == "fail"
    for count in range(3):
        if count:
            candidate = root / f"v4vmm-task016.{count}"
            candidate.mkdir()
            (candidate / "task-016-fixture").touch()
        if count == 1:
            assert locate(root) == candidate
        else:
            try:
                locate(root)
            except SystemExit as error:
                assert ("No task" if count == 0 else "Multiple task") in str(error)
            else:
                raise AssertionError("Fixture discovery must refuse absence and ambiguity")
    subprocess.run([sys.executable, str(script), "mode", str(root), "dead"], check=True)
    assert (root / "mode").read_text() == "dead"
print("Green: fixture directory regression check")
PY
```

## Cleanup

Close the fixture app. Stop the relay with Ctrl+C in terminal A. In terminal B,
remove only the isolated directory after confirming its printed path:

```bash
printf '%s\n' "$task016_dir"
test -f "$task016_dir/task-016-fixture" && rm -r -- "$task016_dir"
unset task016_dir task016_app_pid
```

No real service needs restoring. The environment overrides applied only to
individual commands. Acceptance is recorded in task 016 and the delivery order;
its entry has been removed from [pending human checks](../pending-human-checks.md).
