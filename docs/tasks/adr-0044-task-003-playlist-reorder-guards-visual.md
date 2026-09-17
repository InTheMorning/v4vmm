# ADR 0044 Task 003: Playlist Reorder Visual Readiness

Status: Accepted - 2026-09-10.
Implementation recorded; operator visual acceptance remains open.
The 2026-09-16 drag responsiveness follow-up is operator-accepted, including
reordering and horizontal out-of-bounds dragging after restart with the normal
desktop build. Post-check fixture preservation passes. Profile recovery located
GPUI's synchronous test drawing loop in the captured executable; the launcher
now rebuilds before launch. The broader Light/Dark playlist checks remain open.

## Scope After Reconciliation

Playlist reorder and mounted-row updates is the remaining acceptance scope.
The implementation and earlier mechanical evidence are recorded in the
[ADR 0044 review checklist](../reviews/adr-0044-review-checklist.md).
Its retirement table identifies replaced requirements before listing survivors.
This packet no longer instructs an agent to rebuild the earlier screen design.

The 2026-09-10 governance pass changes documentation only. It records no new
visual pass and does not erase the earlier incident evidence.

## Owners And Constraints

- [ADR 0044](../adr/0044-playlist-drag-handle-reordering.md) owns the surviving contract.
- The review checklist names the current shared owners and existing guards.
- ADRs 0047/0048/0060 own shared Music surfaces and frame navigation.
- An agent must not run the app. A person performs the checks below.
- Do not restore a retired screen, toolbar player, global scope enum, or
  inspector-local return control to satisfy an old instruction.
- A failure requires a bounded fix at its shared owner, relevant mechanical
  checks, and another operator inspection of the failed case.

## Acceptance Criteria

Mechanical: existing ownership guards cited in the review remain applicable.
Their recorded implementation results are historical evidence, not a claim
that a new suite was run during reconciliation.

Visual: complete the matching checklist rows in both Light and Dark using
the required populated fixture. An unavailable fixture leaves that row open.

## Operator Visual Check

### Drag Gesture Follow-Up — 2026-09-16

ADR 0066 V2 fixture `/tmp/v4vmm-startup-4yqtadz3` exposed a brief pause while
dragging; Move Up/Down used the same background reorder command without a pause.
The shared handle in `src/ui/shells/playlist.rs` allowed mouse-down to also start
Root text selection. On drop, text selection stayed active, and later pointer
movement selected unrelated text. The interaction test
`adr_0044_playlist_drag_does_not_start_text_selection` failed before the fix and
passes after the handle stops mouse-down propagation. It also checks that the
drop receives the original subject once, ends the drag, and permits a subsequent
ordinary text selection. This is a situational ADR 0044 regression guard.

The correction stays at the shared playlist handle. VM action state, tokens,
reorder dispatch and database behavior are unchanged. The test proves gesture
isolation; a person must still verify whether the reported pause is gone.

Mechanical verification is Green: formatting, compile, strict Clippy, debug
build, 1,410 unit tests and 251 architecture guards. Ten existing documentation
examples remain ignored. These results do not close the desktop visual gate.

Use the retained V2 fixture and the rebuilt binary in a desktop terminal:

```bash
python3 docs/runbooks/startup-recovery-fixture.py run /tmp/v4vmm-startup-4yqtadz3
```

1. Use a normal-width or maximized window and open Startup fixture playlist.
2. Drag a handle down, then back up. Require a responsive preview, the correct
   insertion edge and one committed move on release. An app pause fails.
3. Release a handle over its original row and outside valid rows. Require no
   move. After releasing, move the pointer over text; it must not start selecting.
4. Select text normally in a report and copy it. Menu Move Up/Down must still
   work. Repeat in Light/Dark using theme preview without saving.
5. Restore the original playlist order. Quit, then run `retry-inspect` for this
   fixture. Keep it on failure; clean up only after acceptance using task 007's
   [preservation/cleanup procedure](../runbooks/startup-recovery-check.md#preservation-and-cleanup-for-each-case).

Needs a Linux desktop and this disposable fixture; no audio hardware or real
service is involved. V2 preservation before the drag correction passed. Recheck
after interaction testing. The narrow Library layout defect remains separate.

### Drag Start Pause Diagnostic — 2026-09-16

The operator rechecked the correction and reports an immediate 4–5 second pause
on pressing the handle, with the hand cursor unchanged until the move occurs.
The selection regression remains guarded, but it did not fix this pause. The
handle press only stops event propagation, and the drag constructor creates its
preview; neither calls the database. Reorder dispatch begins at drop. These
source observations do not identify the native stall, so capture its stack before
another behavioral correction.

1. Keep `/tmp/v4vmm-startup-4yqtadz3` open at the populated playlist. If it is
   closed, use the fixture run command above. This diagnostic needs a Linux
   desktop and GDB. The existing desktop ptrace restriction requires `sudo` to
   attach; no kernel setting change is needed.
2. In a second terminal, run the following. After the password prompt completes,
   the three-second delay lets you return to the app and press/drag the handle.
   GDB briefly stops the process, records stacks without arguments or local
   variable values, then detaches. It disables debug-file downloads and does not
   write a core dump.

   ```bash
   sudo -v
   sleep 3
   sudo -n gdb --batch --nx \
     -iex 'set debuginfod enabled off' \
     -iex 'set print frame-arguments none' \
     -iex 'set print entry-values no' \
     -p "$(cat /tmp/v4vmm-startup-4yqtadz3/app.pid)" \
     -ex 'set pagination off' \
     -ex 'thread apply all bt 40' \
     -ex detach \
     > /tmp/v4vmm-startup-4yqtadz3/playlist-drag-stacks.txt 2>&1
   cat /tmp/v4vmm-startup-4yqtadz3/playlist-drag-stacks.txt
   ```

   Share the output and whether the pause occurs on every attempt or only the
   first after launch. An attachment failure is useful error evidence; do not
   change system settings to bypass it. A stack from an idle event loop may have
   missed the pause and does not locate the stall. Debugger-added stop time is
   not a measurement of the app's original delay.
3. Restore the original playlist order using the menu, then quit and inspect
   with the existing `retry-inspect` command. Keep the fixture and capture while
   diagnosis is open. Its normal fixture cleanup removes the capture after
   acceptance. No new production or fixture setting is required.

### Native Layout Capture And Next Diagnostic

The supplied `playlist-drag-stacks.txt` attached to PID 41408 and detached
successfully. Its main thread (thread 1) is computing Taffy block/flex layout,
including nested flex-base and cross-size measurements. All 40 captured main
frames remain inside layout; the trace stops before the caller that initiated
layout. At that instant the main thread is doing computation, not waiting on a
database lock, socket or clipboard response. One stack does not establish where
all 4–5 seconds went or distinguish one expensive frame from repeated redraws.

A temporary simulated-window probe used the shared three-track playlist shell,
split pane and Root. Initial/press/move/release work took about 3–4 ms. Adding the
production workspace frame and two configuration notices at 1400×850 took about
11–13 ms per dispatch or forced redraw. Neither reproduced the pause. These are
mock-platform measurements, not desktop acceptance. The probe was removed; no
additional production change is justified by it. The original capture remains
in the worktree as operator evidence, outside the commit.

Collect a 12-second main-thread CPU profile in the same desktop session:

1. Leave the fixture app open at the playlist. In a second terminal, start this
   block, then return to the app and drag down/up repeatedly during the recording.
   It needs Linux `perf` and the existing `sudo` profiling permission. It samples
   only the app's main thread and uses the software CPU clock. Keep the running
   binary unchanged until the report has been generated.

   ```bash
   sudo perf record -e cpu-clock:u -F 99 --call-graph dwarf,65528 \
     -t "$(cat /tmp/v4vmm-startup-4yqtadz3/app.pid)" \
     -o /tmp/v4vmm-startup-4yqtadz3/playlist-drag-perf.data -- sleep 12
   sudo perf report --stdio --stdio-color never --no-children \
     --call-graph graph,1,caller --percent-limit 1 \
     -i /tmp/v4vmm-startup-4yqtadz3/playlist-drag-perf.data \
     > /tmp/v4vmm-startup-4yqtadz3/playlist-drag-profile.txt
   cp /tmp/v4vmm-startup-4yqtadz3/playlist-drag-profile.txt ./playlist-drag-profile.txt
   ```

2. Share `playlist-drag-profile.txt`, whether one or several pauses occurred,
   and any recording error. A useful profile has samples with resolved app,
   GPUI or Taffy symbols. A recording failure does not justify changing kernel
   permissions. The agent checked the available profiler options; its sandbox
   does not expose the named software event, so it did not validate recording.
3. Restore playlist order using the menu. Quit and run `retry-inspect`. Keep the
   fixture, raw profile and text capture for diagnosis. Normal fixture cleanup
   removes its captures after acceptance; remove the copied worktree reports
   when they are no longer needed, without committing them.

### CPU Report Receipt

The supplied `playlist-drag-profile.txt` contains 846 CPU-clock samples with zero
lost samples and an approximate event count of 8,545,454,460 ns (8.55 seconds of
main-thread CPU time during the recording). Flex layout accounts for 21.16% of
self samples and block layout's inner computation for 6.38%; additional layout
helpers appear throughout the report. The operator reports a longer hang when
dragging horizontally outside the valid area as well as the reorder pause.

Perf reported an `addr2line` failure for the app's cached debug ELF. Its build ID,
`3354b84cf53a7885cc0bd19ade7e4d5e4ac26a0d`, matches `target/debug/v4vmm` in the
worktree. App function names are resolved, but most caller chains are missing.
This is an analysis limitation, not evidence that the app emitted that error.
The text report establishes CPU-heavy layout but did not identify its triggering
owner. The operator subsequently supplied the raw recording for offline recovery.

### Recovered Caller And Desktop Build Guard

Offline reconstruction of the recorded registers and stack recovered complete
callers in early samples. Whole-window layout at `Window::draw_roots` line 3293
runs from `App::flush_effects` line 1722 after pointer input and a background
service snapshot. In the pinned GPUI source, that synchronous draw loop is
compiled only for tests, `test-support`, or `bench-support`. Normal desktop
updates schedule frames instead. The profile is evidence of a test-enabled
desktop binary; it is not a drag-preview or database stack.

The build sequence reproduced the issue without starting the app:

1. `cargo build --locked --offline --quiet --bin v4vmm` produced build ID
   `8295ff34c03a88bdfe1fb8410a36934bb9b18ebb`, without GPUI test-platform symbols.
2. `cargo test --locked --offline --quiet --test architecture_tests` passed 251
   guards and replaced `target/debug/v4vmm` with build ID
   `3354b84cf53a7885cc0bd19ade7e4d5e4ac26a0d`, exactly matching the capture. That
   binary includes GPUI test-platform symbols and the captured test draw loop.
3. A final normal desktop build restores the non-test executable.

Synchronous drawing on each update can accumulate pointer events and explains
why prolonged movement can worsen the delay. Desktop verification is still
required to establish that this build correction removes the observed pause.
An expanded mock Library probe remained responsive and was removed. No speculative
layout change remains.

The shared startup-fixture launcher now builds the normal executable before
applying the fixture environment or opening the GUI. Its situational ADR 0066
tests require build-before-launch, developer build environment, isolated app
environment, and no launch or exit-record mutation after a failed build. These
tests are Green (16 fixture tests). The agent instructions also require a normal
desktop build after tests for operator handoff.

Operator recheck, using Cargo, cached dependencies and the same Linux desktop:

1. Quit the existing fixture app, then run:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py run /tmp/v4vmm-startup-4yqtadz3
   ```

2. Open Startup fixture playlist at normal width. Press a handle, drag down and
   up, and drag horizontally outside valid rows before release. Require immediate
   response, correct insertion and one move only on a valid drop. Any seconds-long
   pause or delayed movement fails. Menu Move Up/Down and normal text selection
   must still work. Repeat in Light/Dark without saving theme changes.
3. Restore the original order, quit, and inspect:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py retry-inspect /tmp/v4vmm-startup-4yqtadz3
   ```

Keep the fixture and original recording until diagnosis and preservation are
complete. The generated worktree copy is diagnostic evidence and is not for
commit. Normal task 007 cleanup removes the fixture after its remaining checks
are accepted. No additional recording is needed for this recheck.

### Desktop Responsiveness Acceptance — 2026-09-16

The operator reports **pass** for the instructed normal-build restart, playlist
reordering and horizontal out-of-bounds drag recheck. This accepts the reported
drag pause correction. The accompanying inspection of
`/tmp/v4vmm-startup-4yqtadz3` passes original/unedited configuration preservation,
owner-only backup permissions and every named configuration check. Backup
`.v4vmm-config-4147752-0.backup` remains. Music, library, bindings, tool blockers
and migration records 1–11 are preserved: one playlist, three tracks and three
playlist tracks, with a.wav/b.wav/c.wav bindings. No unexpected setting paths,
candidate files, music probes or database probes remain.

The pass covers the latest instructed responsiveness check. It does not supply
separate Light/Dark, unavailable-row or removal evidence for the broader
inherited playlist gate. Task 007's subsequent ordinary Null-player Play check
also passes, as does final preservation inspection. The operator confirms task
007's fixture cleanup and directory-absence check on 2026-09-16. The fixture path
in the diagnostic procedures above is now historical; use a fresh fixture for
remaining checks. No new profiling is needed.

### Remaining Inherited Checks

Follow [the current playlist reorder and mounted-row updates procedure](../runbooks/inherited-ui-checks.md#playlist-reordering--adr-0044-task-003),
including preparation and cleanup. Record the fixture, entry route, theme,
result, and any screenshots in the review checklist. Close only this packet's
criteria, even when one walkthrough also supplies another packet's evidence.

## Closure

After operator acceptance and cleanup, update this Status, the review checklist,
the owning ADR when all its gates are closed, and the delivery row. Remove the
corresponding entry from pending human checks in the same change.

No runtime work or visual acceptance is claimed by this reconciliation.
