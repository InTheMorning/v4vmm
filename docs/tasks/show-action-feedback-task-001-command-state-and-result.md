# Show Action Feedback Task 001: Command State And Result

Status: Implemented - 2026-09-10.
Mechanical acceptance Green. The operator confirmed the task tested and passed
on 2026-09-10, closing A7 (ADR 0065), A8 (ADR 0059), and A9 (ADR 0059/0063).
No visual acceptance gate remains open for this task.

## Goal

Make a pressed control report its own progress and its own result. A press holds
its state until the command answers, the controls stay where they are, and the
result of `Check all feeds` is readable.

## Why These Three Together

They are one defect in three places: **the surface that starts an action does
not own how that action reports.**

- A8, a watch sample repaints the state the operator just changed.
- A9, the stream action row is removed instead of disabled.
- A7, the feed check result is written where it does not fit.

Fixing one and not the others leaves the same class of surprise on the other two
controls.

## Implementation And Ownership

- `src/view_models/show.rs`: `ShowCommandState` retains a separate record for each
  service role and the stream. Each press has a monotonically increasing ID.
  `complete` accepts only its matching result; `observe` releases successful
  transitions on fresh readback. Every page projection applies
  `with_command_state`. The command controls remain unavailable throughout the
  transition, and `mark_stream_working` retains its action row.
- `src/app.rs` and `src/app/show.rs`: the app retains that state across queue,
  readiness, and event refreshes. `ShowCommandCompletion` timestamps return in
  the worker, including command failure. Both callbacks carry the command ID.
  Changing the watched configuration clears old command records without reusing
  IDs. A command failure releases the transition, projects the latest observed
  state, displays the error on Show, and requests a fresh read.
- `src/runtime/broadcast_service_watch.rs`: `read_started_at` is captured before
  the batch's first read. The existing `at` remains the batch completion time.
  Only the snapshot constructor and the two test snapshot initializers needed
  the added field; existing consumers retain their original facts. Polling and
  transport behavior are unchanged.
- `src/library/app_impl.rs`: the feed-check result is a full-width sibling below
  the action row. Its message and count projection remain owned by
  `src/view_models/library.rs`; no wording changed. Existing spacing and color
  tokens style the row. Service/stream controls retain their shared typed action
  renderers and token path.
- [Operator fixture](../runbooks/show-action-feedback-fixture.py): adds delayed
  simulated service/encoder replies using task 016's existing config isolation.
  The task 016 fixture and completed recovery walkthrough remain unchanged.

The incident was a command transition overwritten by a late watch batch. A
batch can finish after a command even when its reads began before that command
returned. Freshness therefore uses the read-start boundary, strictly after the
worker's completion stamp. Repeated projection or duplicate deliveries do not
count as new observations.

A fresh agreeing state releases the transition. Fresh Failed, NotInstalled, or
NotReachable service states release it immediately as settled answers. Reset
also accepts Active or Inactive. Three distinct fresh samples release any
remaining transition, even if they still disagree or report Starting, Stopping,
or Unknown. This is a sample bound, not a three-second timer; the watch must
supply those samples. Pending commands retain ownership until their matching
result arrives. Logs remain usable.

## Verification

All guards below are situational. Each has one owning ADR.

| Owner | Mechanical proof |
|---|---|
| ADR 0059 | `show_command_transition_requires_fresh_read_and_matching_direction`: Start/Stop ignore old state and pre-completion reads, even if a batch finishes later or agrees; in-flight commands keep controls unavailable. |
| ADR 0059 | `show_command_failures_and_old_results_keep_role_ownership_separate`: failure restores observed state; role ownership, Reset, stale completions, and host changes remain separate. |
| ADR 0059 | `show_command_fresh_settled_failure_releases_immediately`: failed starts and missing/unreachable units do not wait for agreement. |
| ADR 0059 | `show_command_transition_is_bounded_and_duplicate_reads_do_not_count`: three distinct fresh samples release disagreeing and transitional states. |
| ADR 0059 | `show_command_stream_keeps_controls_and_uses_the_service_release_policy`: both stream operations retain typed controls and use the same completion/freshness/failure/bounded-release policy. |
| ADR 0059 | `show_watch_read_start_precedes_all_reads_and_end_follows_them` in the watch module: the two timestamps bracket every service and encoder read. |
| ADR 0059 | `adr_0059_show_command_feedback_survives_all_reprojections` and `adr_0059_stream_working_retains_typed_actions` in architecture tests: mounted projection and command callbacks consume the retained state; stream actions stay present. |
| ADR 0065 | `adr_0065_feed_check_result_has_its_own_full_width_row` in architecture tests: the result follows the buttons, uses the sidebar width, and does not truncate. Existing `feed_check_route_repair_status_reports_counts_separately` retains count/message coverage. |

The `show_command_*` tests live in `src/view_models/show.rs`. The named tests
and guards replace the packet's implementation instructions and superseded
backlog prescriptions in the same change.

Mechanical checks on 2026-09-10: Green.

```bash
cargo check --quiet
cargo fmt -- --check
cargo test show --lib --quiet
cargo test --quiet
cargo clippy --quiet -- -D warnings
cargo build --quiet
```

Full test results: 1,243 unit tests and 216 architecture tests passed; ten
existing documentation examples remain ignored. The isolated fixture's service
and encoder commands, failure/stuck modes, unsupported-unit rejection, and
marker verification also passed without running the desktop app.

## Operator Visual Check

Passed by the operator on 2026-09-10. These steps remain a manual regression
check for future changes.

Purpose: check service progress, stable Stream controls, and readable feed-check
results. The compact Event layout belongs to task 017.

A fixture mode controls how later commands behave. Changing the mode does not
change the service's current state.

| Mode | What the fake command does |
|---|---|
| `normal` | Succeeds and changes the state. |
| `crash` | Start succeeds, but the service reports Failed. |
| `fail` | Returns a command error and leaves the state unchanged. |
| `stuck` | Pretends to succeed and leaves the state unchanged. |

Requires a Linux desktop session and Python 3.11+. No broadcast hardware,
installed publisher, running relay, actual encoder, or real unit changes are
needed. The generated systemctl and encoder stubs are scoped to this app launch.
Allow about 20 seconds for three delayed fresh samples in the stuck-state test.

1. In a desktop terminal, build and create a new isolated fixture. Keep this
   terminal open so `feedback_dir` remains available:

   ```bash
   cd /home/citizen/build/v4vmm
   cargo build
   feedback_dir=$(mktemp -d /tmp/v4vmm-feedback.XXXXXX)
   python3 docs/runbooks/show-action-feedback-fixture.py setup "$feedback_dir"
   python3 docs/runbooks/show-action-feedback-fixture.py verify "$feedback_dir"
   env XDG_CONFIG_HOME="$feedback_dir/config" PATH="$feedback_dir/bin:$PATH" \
     ./target/debug/v4vmm
   ```

   Confirm Show's Source is **Task 016 fixture**, the library is empty, and
   Settings uses **http://127.0.0.1:17863**. If any differs, close that window
   and correct the fixture launch. This procedure does not create an event.
   Close the app once; its isolated database now exists.

2. Prepare the long feed result and relaunch in the same terminal:

   ```bash
   python3 docs/runbooks/show-action-feedback-fixture.py seed-feeds "$feedback_dir"
   env XDG_CONFIG_HOME="$feedback_dir/config" PATH="$feedback_dir/bin:$PATH" \
     ./target/debug/v4vmm &
   feedback_app_pid=$!
   ```

   In Music, press **Check all feeds**. The fixture has one undownloaded track
   and no subscribed feeds, so no network query or tag write is needed. Expect
   the whole message beneath the button: **Checked 0 feeds; all feeds up to
   date; repaired 0 route tags; 0 tracks need publisher routes; 1 failed**.
   Resize down to the smallest allowed window and sidebar width. Wrapping is
   expected; clipped counts, an ellipsis, or result text beside the button fails
   A7. The missing download is intentional and needs no repair.

3. In Show → Live Metadata, press Producer **Stop**. Expect Working with the
   controls retained and unavailable, followed by Inactive. Start it again:
   expect Working followed by Active. Repeat for Publisher. Press a command on
   the other role while the first is Working; neither may clear the other's
   transition. Navigate to Music and back during a command. A brief return to
   the old state before the new state, an enabled duplicate press, or a row
   that changes height fails A8.

4. Stop Producer, wait for Inactive, then set the simulated failed-start mode:

   ```bash
   python3 docs/runbooks/show-action-feedback-fixture.py mode "$feedback_dir" crash
   ```

   Press Producer Start. The command succeeds but the observed unit has failed.
   Expect Failed on fresh readback, never indefinite Working. Then test an
   explicit command error:

   ```bash
   python3 docs/runbooks/show-action-feedback-fixture.py mode "$feedback_dir" fail
   ```

   Press Reset. After the brief Working state, expect the observed Failed state,
   a visible fixture-command error on Show, and a usable Reset control. Restore
   normal mode, press Reset, then Start and wait for Active:

   ```bash
   python3 docs/runbooks/show-action-feedback-fixture.py mode "$feedback_dir" normal
   ```

5. Check that the UI stops waiting when a service ignores a command.
   First restore normal behavior:

   ```bash
   python3 docs/runbooks/show-action-feedback-fixture.py mode "$feedback_dir" normal
   ```

   Producer must read Active before this check. If it reads Failed, press Reset
   and wait for Inactive. If it reads Inactive, press Start and wait for Active.
   Then freeze that Active state:

   ```bash
   python3 docs/runbooks/show-action-feedback-fixture.py mode "$feedback_dir" stuck
   ```

   Press Stop. Expect Working, then Active again after three fresh status
   batches. Stop must become available again. The fake service deliberately
   ignores Stop; the UI must report that result instead of waiting forever.
   Reset in `stuck` mode also leaves the old state untouched.
   Restore normal behavior afterward:

   ```bash
   python3 docs/runbooks/show-action-feedback-fixture.py mode "$feedback_dir" normal
   ```

6. Check that Stream controls stay visible and recover from command errors.
   Open Stream. Wait for Disconnected, then press Connect. Wait for Connected,
   then press Disconnect. Both buttons must stay visible and unavailable during
   Working. The row must keep its height.

   Make the next command fail:

   ```bash
   python3 docs/runbooks/show-action-feedback-fixture.py mode "$feedback_dir" fail
   ```

   Press the available Stream action. Expect a command error on Show. The
   stream must keep its previous state. Both buttons must stay visible, and the
   applicable button must become available again. A disappearing row or a
   permanently disabled control fails A9. Restore normal behavior:

   ```bash
   python3 docs/runbooks/show-action-feedback-fixture.py mode "$feedback_dir" normal
   ```

7. Close only the fixture app, wait for its process to exit, and remove only the
   marked temporary directory:

   ```bash
   wait "$feedback_app_pid"
   python3 - "$feedback_dir" <<'PY_CLEANUP'
   from pathlib import Path
   import runpy
   import shutil
   import sys
   fixture = runpy.run_path('docs/runbooks/show-action-feedback-fixture.py')
   root = fixture['BASE']['fixture_root'](sys.argv[1])
   fixture['verify'](root)
   if root.parent != Path('/tmp') or not root.name.startswith('v4vmm-feedback.'):
       raise SystemExit('Cleanup requires the generated feedback directory')
   shutil.rmtree(root)
   PY_CLEANUP
   unset feedback_dir feedback_app_pid
   ```

The operator pass is recorded in this task and the delivery order. Its pending
human-check entry is closed. Task 017 is ready for the next session.

## Deviations

The packet's original file list omitted the retained app-state field and test
snapshot initializers; those additive edits are included. A separate operator
fixture was added so delays and failure states are reproducible without touching
real services. No transport, watch interval, queue, card ordering, or feed-result
wording changed.
