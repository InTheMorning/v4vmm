# ADR 0066 Task 005: Session Drain And Resumption

Status: Ready after the preceding packet - 2026-09-10.
Implementation not started. Operator check specified below; not runnable or accepted yet.

## Goal

Stop the app's own work, release every configured database handle, and resume one fresh session before any live core correction or database maintenance can use this transition.

## Read First And Dependency

Read [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md),
the [phase plan](../plans/adr-0066-startup-recovery-phase-plan.md), this whole packet, and the
[review checklist](../reviews/adr-0066-startup-recovery-review-checklist.md).
Execute after [task 004](adr-0066-task-004-optional-tool-isolation.md) in the phase plan's order.
Complete this packet in one session; do not start its successor.
Names marked **new**, including tests and guards, are implementation targets,
not claims that those files or symbols already exist. If a predecessor already
created a listed owner, extend that owner.

## Files To Inspect

- src/app.rs — shutdown and runtime/playback ownership
- src/app/bootstrap.rs; src/app/startup.rs — recovery/normal root lifecycle
- src/application/async_command_runner.rs — dispatch receiver is not cancellation
- src/library.rs; src/library/app_impl.rs — actor and separate configured Connection
- src/runtime/playback_polling.rs; src/runtime/broadcast_readiness.rs; src/runtime/broadcast_service_watch.rs
- src/runtime/actor.rs; src/runtime/paged_list_vm.rs; src/runtime/musicbrainz_feed_saga.rs; src/application/paged_track_list.rs — remaining actor/connection lifetime owners
- src/presentation/runtime_host.rs; src/presentation/maintenance_executor.rs
- src/application/ports/download_manager.rs; src/playback_owner.rs
- `tests/architecture_tests.rs`; `AGENTS.md`

## Files Likely To Change

- src/application/session_lifecycle.rs (new); src/application/mod.rs
- src/application/async_command_runner.rs — admission and active-operation tracking
- src/app/startup.rs; src/app.rs; src/library.rs; src/library/app_impl.rs — teardown/resume adapters only
- src/presentation/runtime_host.rs; src/presentation/startup_presenter.rs
- src/runtime/playback_polling.rs; src/runtime/broadcast_readiness.rs; src/runtime/broadcast_service_watch.rs — stop acknowledgement only where missing
- src/runtime/actor.rs; src/runtime/paged_list_vm.rs; src/runtime/musicbrainz_feed_saga.rs; src/application/paged_track_list.rs — tracked shutdown/connection release only
- src/view_models/startup.rs; src/ui/composites/maintenance_forms.rs (new) — session action shell
- src/ui/composites/mod.rs; tests/architecture_tests.rs
- docs/runbooks/startup-recovery-fixture.py; docs/runbooks/startup-recovery-check.md
- This packet's Status/evidence, the phase plan and review checklist.
- ADR 0066's guard references/partial line and the delivery/deferred indexes as
  appropriate; `docs/pending-human-checks.md` when a runnable visual gate opens.
  Update `AGENTS.md` when the next executable packet changes.

## Do Not Touch

- SQLite file replacement/external-process locking (011–012)
- Changes to remote services, broadcast timing, or automatic kill of external tools
- Replacing configuration or implementing its editors (006)
- Other repositories or unrelated open human checks.
- Unrelated Clippy debt; `--all-targets` is not this repository's lint gate.

## Constraints

- Preserve ADR 0066's minimum: valid core configuration, usable music storage
  and working SQLite. Optional services/tools never become core requirements.
- Keep typed facts/availability/intent in backend/application/view-model owners.
  Shared composites own geometry and named tokens. Screens only wire them.
- Every new module is called by the packet's live workflow. No parked scaffolding,
  fake healthy values or screen-only execution checks.
- Preserve original data and safe diagnostics; no tokens or credential-bearing
  excerpts in reports, clipboard or Debug output.
- Delete duplicated mechanism prose when its guard lands. Replace it with the
  actual guard symbol and verification artifact in this packet; keep the ADR's
  binding decision/invariants. Task 001 owns the series handoff review.

## Implementation Steps

1. Define session states Running, Draining, Maintenance and Resuming, with a monotonically changing generation. MaintenanceSession is an owned capability returned only after normal work stops and configured database handles close. A boolean on a renderer is not proof of this transition.

2. Make command admission atomic with an active-operation counter/guard in AsyncCommandRunner. Block new dependent work after draining starts; await all previously admitted blocking commands. Dropping a oneshot receiver does not stop spawn_blocking. A named finite drain timeout reports which work remains and does not authorize maintenance.

3. Stop actor watches and paged lists, await acknowledgements, and release their database clones. Include LibraryApp's extra open_db connection in its paged actor setup. Stop/release the built-in playback owner and owned child process resources through existing shutdown paths. Do not stop the independent publisher/encoder chain.

4. Unmount normal child views and release their handles after work completes, retaining the session/recovery report outside TopApp. Use actual close/drop ownership outcomes, not Arc::strong_count alone. An outstanding handle/command leaves the transition incomplete with a report and a retry route.

   Await actor/worker shutdown and drop any blocking runtime resources through
   the presentation maintenance path. Never wait for them in a renderer or
   freeze the window while waiting for admission counters to reach zero.

5. Add a Settings maintenance action that explicitly ends the current app session and opens the existing recovery/tools surface. Its purpose and effect on built-in playback must be stated before execution. This is the live caller for the new lifecycle; task 006 uses it for core config correction and tasks 011–012 use it for database preservation/restore.

6. Open app performs a fresh core check/preparation through 002 and installs one new session generation. Ignore stale callbacks for display; do not interpret ignored callbacks as cancelled I/O. Reattach only the new actor/command resources. A failed preparation stays in recovery with previous reports.

7. Provide test seams for a held command, held actor connection, drain failure and stale completion. Extend the fixture with an observable session generation and controlled held command; release it through the fixture instead of manually killing work.

## Acceptance Criteria

Mechanical; assert at the named owner. Test/guard names below marked new must
be implemented by this packet.

| ID | Proof owner | Required assertion |
|---|---|---|
| C1 | admission/drain tests | Racing dispatch either enters the tracked prior generation or is rejected; no untracked writer enters after drain begins. A dropped result receiver still keeps its work counted. |
| C2 | lifecycle tests | Held command/actor/extra connection blocks MaintenanceSession with a bounded report. No new session or maintenance capability is produced early. |
| C3 | resource tests | Successful drain closes all known configured connections and owned playback resources. External publisher/encoder services receive no stop command. |
| C4 | resumption tests | One fresh generation is mounted after valid checks; old callbacks cannot change its state. Failed checks leave recovery, not a partly reopened TopApp. |
| C5 | new situational guard adr_0066_core_maintenance_drains_the_session | Core reconfiguration and later database installation can consume only the shared managed transition. Cite invariants 5–6. |

Documentation proof: remove this packet's duplicate mechanism prose as its guards
land; record actual symbols and fixture/runbook anchors. Keep its Status,
the plan, ADR partial line, delivery row and pending-human-check index truthful.
An unwalked visual check cannot pass through a green mechanical suite.

## Test Commands

Run the focused owner tests while editing, then the repository gate once:

```bash
cargo fmt -- --check
cargo check --quiet
cargo test --quiet
cargo clippy --quiet -- -D warnings
cargo build --quiet
```

New source assertions must preserve the rules of any guard they replace and name
ADR 0066 as their situational owner. Do not widen the lint gate or add a second
integration-test file.

## Operator Visual Check

The implementation must extend `docs/runbooks/startup-recovery-fixture.py` and
`docs/runbooks/startup-recovery-check.md` with a section for this packet.
Those files are implementation deliverables, not commands available at packet
authoring time. Follow the phase plan's fixture contract. Supply unindented
copyable commands, named fixture state, purpose, expected result and cleanup.
Never ask an operator to repeat an already accepted check without a changed
owner or an unresolved failure.

1. Open Settings in the fixture and choose its session-maintenance action. Confirm the app explains that its own active work/playback must stop and that the external broadcast chain continues.

2. Hold a fixture command, request maintenance, and confirm it reports waiting/failure without entering maintenance early. Release the command and retry.

3. Use Open app from maintenance. Confirm the library reappears once, with one fresh session generation and no duplicate activity. Clean up.

Do not run the app as an agent. When the binary/fixture are ready, open this
packet's gate in its Status, the delivery row and pending-human-check index.
Record the operator's actual result before closing it.

## Rollback

Revert this packet's code as a coherent change if its gate fails; preserve all
operator configuration, database backups and music. Do not reverse migrations
or delete recovery artifacts as a code rollback. Leave dependent packets pending
and document any observable intermediate limitation.

## Expected Final Report Format

1. Files changed
2. Tests run (Green, or the exact failure)
3. Behavior changed
4. Deviations from task
5. Unresolved concerns
6. Operator visual check, or the explicit reason no new visual check applies

## Escalation Triggers

An actor/command cannot acknowledge completion or a configured Connection has no tracked owner. Name and fix that owner before permitting maintenance; do not use a timeout as evidence of safety.
Routine placement inside the named owner is authorized. If a boundary needs a
new architectural decision, name the conflict and proposed bounded correction
before widening this packet.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md`
- `docs/adr/0066-configuration-and-startup-failure-recovery.md`
- `docs/plans/adr-0066-startup-recovery-phase-plan.md`
- This packet in full, including Files To Inspect, implementation steps and criteria.

Goal:
- Stop the app's own work, release every configured database handle, and resume one fresh session before any live core correction or database maintenance can use this transition.

Constraints:
- Follow this packet's Constraints and Implementation Steps.
- Preserve its data, dependency and prose-retirement contracts.
- One packet this session. Never run the app.

Do not touch:
- SQLite file replacement/external-process locking (011–012)
- Changes to remote services, broadcast timing, or automatic kill of external tools
- Replacing configuration or implementing its editors (006)

Acceptance criteria:
- Prove every mechanical row in this packet at its named owner.
- Record actual guard references and keep trackers consistent.
- Supply the specified fixture/runbook check; keep its human gate open until walked.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test --quiet`
- `cargo clippy --quiet -- -D warnings`
- `cargo build --quiet`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
