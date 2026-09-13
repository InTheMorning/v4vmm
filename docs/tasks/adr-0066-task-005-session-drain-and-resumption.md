# ADR 0066 Task 005: Session Drain And Resumption

Status: Complete - 2026-09-11; mechanical checks Green. Operator V1–V3,
preservation inspection and fixture cleanup accepted.

The operator explicitly requested task 005 after completing ADR 0069 task 001,
using task 004's delivered owners before task 004's remaining acceptance.
That scheduling exception closes none of task 004's checks. Task 006 is complete
with operator acceptance, preservation and fixture cleanup as of 2026-09-13;
task 007 follows in a fresh session.

## Goal And Scope

Stop the app's own work, release every configured database handle, and resume
one fresh session before live core correction or database maintenance can use
this transition. [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md),
the [phase plan](../plans/adr-0066-startup-recovery-phase-plan.md) and the
[review checklist](../reviews/adr-0066-startup-recovery-review-checklist.md)
own the decision, sequence and series coverage.

This packet adds the live End app session action in Settings → Diagnostics.
It explains that admitted work must finish, built-in playback stops and unsaved
Settings edits are discarded. Recovery opens after resource release; Open app
performs fresh core checks and preparation. Session reports survive the
transition and remain available in the reopened Diagnostics group.

No configuration format, configuration editor, database replacement, external
process lock or external service control is added. Existing playback state is
recorded as stopped when a playback session exists. Ending the app session
sends no stop command to the independent publisher or encoder. Task 004's
cue/audition and mpv IPC acceptance limitations remain separate.

## Implementation And Proof

The situational guard `adr_0066_core_maintenance_drains_the_session` in
[architecture_tests.rs](../../tests/architecture_tests.rs) owns this packet's
mechanism checks under ADR 0066 invariants 5–6. It replaces the packet's
implementation recipe and coding-model prompt. The binding ADR invariants,
acceptance requirements and operator procedure remain.

| Live owner | Responsibility |
|---|---|
| [session_lifecycle.rs](../../src/application/session_lifecycle.rs) | Atomic admission and tracked lifetimes; Running, Draining, Maintenance and Resuming; actual connection close and owned `MaintenanceSession` authority |
| [async_command_runner.rs](../../src/application/async_command_runner.rs) | Admission before the blocking pool; operation guard survives a dropped result receiver; typed rejection during drain |
| [session_transition.rs](../../src/presentation/session_transition.rs), [maintenance_executor.rs](../../src/presentation/maintenance_executor.rs) | Finite waits and runtime/resource teardown on the independent worker; abandoned preparation resources retire there too |
| [app/session.rs](../../src/app/session.rs), [app/startup.rs](../../src/app/startup.rs), [bootstrap.rs](../../src/app/bootstrap.rs) | Live teardown wiring, child unmount, retained reports, fresh preparation and one new generation |
| [session.rs](../../src/view_models/startup/session.rs), [startup.rs](../../src/view_models/startup.rs) | Typed actions, purpose, generation labels and recorded UTC reports through drain and recovery |
| [maintenance_forms.rs](../../src/ui/composites/maintenance_forms.rs), [app/settings.rs](../../src/app/settings.rs) | Shared token-based entry/report layout and thin Diagnostics wiring |
| [async_command_presenter.rs](../../src/presentation/async_command_presenter.rs), [startup_presenter.rs](../../src/presentation/startup_presenter.rs) | Retired-generation display rejection and single current preparation mount |
| [playback_owner.rs](../../src/playback_owner.rs), [mpv.rs](../../src/playback_driver/mpv.rs) | Checked shutdown and reaping of the app-owned child, removal of its now-playing file and existing playback-state stop |

The runtime bus and `RuntimeHost` carry the shared session. The live desktop
actor inventory is generic/paged-list `actor.rs`, `playback_polling.rs`,
`broadcast_readiness.rs`, `broadcast_service_watch.rs` and
`musicbrainz_feed_saga.rs`. Each acknowledges completion after releasing its
future's resources. `LibraryApp` releases its actor handles and runtime owner;
its paged actor owns a separate configured SQLite connection. Thumbnail cleanup
in `media/image_cache.rs` also holds an admitted-work guard until its worker
finishes. Runtime retry reuses the current session admission owner.

## Acceptance Criteria

Mechanical checks are Green. Visual criteria are separate below.

| ID | Required assertion | Actual proof |
|---|---|---|
| C1 | Racing dispatch enters tracked work or rejects; dropping the result receiver cannot permit early drain. | `adr_0066_admission_race_tracks_every_accepted_operation`; `adr_0066_dropped_receiver_keeps_blocking_command_counted_through_drain` |
| C2 | Held command, actor or configured connection blocks maintenance with a finite failure; failed work cannot authorize maintenance. | `adr_0066_held_connection_and_resource_owner_prevent_maintenance`; `adr_0066_actor_acknowledgement_follows_its_extra_connection_close`; `adr_0066_failed_worker_cannot_authorize_maintenance`; C1's held-command test |
| C3 | Actual resource release precedes maintenance; app-owned playback shutdown leaves independent processes alone. | `adr_0066_paged_actor_releases_its_exclusive_configured_connection`; `adr_0066_shutdown_reaps_only_the_app_owned_child_without_mpv`; `adr_0066_detached_resources_are_released_on_the_independent_worker`; lifecycle ownership tests and C5's external-command exclusion |
| C4 | Fresh checks precede one new session; stale completions cannot mount or change it; failed checks retain recovery and reports. | `adr_0066_drained_core_reopens_fresh_only_after_successful_checks`; `adr_0066_recovery_retains_session_report_and_rejects_old_mounts`; existing `adr_0066_only_the_current_preparation_mounts_a_normal_factory_once`; `adr_0066_pending_resource_release_keeps_worker_retryable` |
| C5 | The live core-maintenance entry uses the shared managed transition under invariants 5–6. | Situational `adr_0066_core_maintenance_drains_the_session`; later correction/install packets must consume this authority |

`adr_0066_session_actions_explain_playback_and_keep_retry_report_available`
checks the new view-model action contract and retained report. Shared button,
scrolling, token and renderer-boundary guards remain in the architecture suite.
Human inspection of the running layout and interaction passed as recorded below.

## Mechanical Evidence — 2026-09-11

Green:

```bash
cargo fmt --all -- --check
cargo check --locked --offline --quiet
cargo test --locked --offline --quiet
cargo clippy --locked --offline --quiet -- -D warnings
cargo build --locked --offline --quiet
```

The full suite passed 1,351 unit tests and 238 architecture tests; ten existing
documentation examples remain ignored. The socket fixtures required running
the suite outside the filesystem/network sandbox. Production Clippy is the
repository gate; no unrelated all-target cleanup is included.

The agent verified fixture setup, identity, held-command mode, session-status,
session-release, switching back to normal, preservation inspection and cleanup
using `/tmp/v4vmm-startup-k7ud7ktt`. Configuration and music bytes, three tracks,
three playlist memberships, one playlist, bindings and migrations 1–11 were
preserved, with no residual probes. The agent removed that temporary fixture.
No GUI was started and its observation list stayed empty; this is fixture-command
verification, not operator lifecycle acceptance.

The fixture's Rust hooks remain under `cfg(debug_assertions)` and require the
verified `V4VMM_STARTUP_FIXTURE` identity. The held command owns a configured
connection reference and is released through `session-release`, with a ten-minute
fixture deadline. `session-status` reads recorded generation observations;
`maintenance` is recorded only after the managed close succeeds.

## Operator Evidence — 2026-09-11

The operator walked V1–V3 using the debug build and the isolated fixture
`/tmp/v4vmm-startup-oaq_8a7x`, then supplied preservation and cleanup results.

| Check | Operator evidence | Result |
|---|---|---|
| V1: session entry | Music showed the three fixture tracks. The operator accepted the App session explanation, session number, keyboard focus and normal/narrow Light/Dark presentation without saving temporary theme choices. | Pass |
| V2: held work and retry | Session 2 began draining at 18:40:09 UTC. At 18:40:14 and again at 18:44:08, the report named `FixtureSessionCommand: 1` and kept maintenance unavailable. The operator accepted responsiveness, Retry drain, Copy report, resizing and unchanged recorded timestamps. | Pass |
| V2: release and recovery | After explicit fixture release, the report recorded resource release and successful core check 6 at 18:45:06 UTC. Both earlier failure entries remained in the copied report. The operator accepted recovery at normal and narrow widths. | Pass |
| V3: fresh session | The operator accepted one playlist with three tracks, a newer session number and the retained report in both themes and widths. Terminal JSON recorded session 2's `command-released` and `maintenance`, followed by one session 3 `opened`, with `held: false`. | Pass |
| Preservation | `config_preserved`, `music_preserved`, `migration_records_preserved`, `bindings_preserved`, `library_preserved` and `tool_blockers_preserved` were true. Three tracks, three memberships, one playlist, bindings `a.wav`, `b.wav`, `c.wav` and migrations 1–11 remained, with no database or residual music probes. | Pass |
| Cleanup | The operator confirmed `Removed fixture: /tmp/v4vmm-startup-oaq_8a7x`. | Pass |

Configuration bytes changed only in the permitted workspace preferences:
`config_bytes_unchanged: false`, `normal_workspace_preferences_only: true`.
The fixture inspection accepted that change.

Session 1's initial drain occurred at 18:34:10 UTC after its 18:20:50 startup,
beyond the fixture's ten-minute command deadline. That attempt did not establish
V2's held-work assertion. Open app started session 2 at 18:36:58, and only the
held-work check was repeated. The final terminal observation sequence was:

```text
1: command-held → maintenance
2: command-held → command-released → maintenance
3: opened
held: false
```

The additional initial generation records the expired attempt; it is not a
duplicate reopening of the accepted session 2 → 3 transition. The runbook now
explains how to repeat the held-work check after its fixture deadline without
repeating accepted V1 checks or miscounting generations.

## Review And Acceptance

The [task 005 review](../reviews/adr-0066-startup-recovery-review-checklist.md#task-005-review--2026-09-11)
records C1–C5 and the ownership inventory. No new architectural decision or
successor packet was needed. The explicit scheduling exception is the only
sequence deviation. The owned mpv shutdown change is limited to proving the
resource release required by this packet; it does not claim to fix task 004's
observed playback error.

The packet, ADR partial status, phase plan, delivery row, source map and
[pending-human index](../pending-human-checks.md) are reconciled. Operator
V1–V3, post-run preservation and fixture cleanup are accepted; this packet has
no remaining human gate. Task 004 and inherited checks retain their scope.

## Rollback

Revert this packet's code as a coherent change if its gate fails. Preserve
operator configuration, database backups and music, and keep unrelated changes.
Do not reverse migrations or delete recovery artifacts as a code rollback.
Leave dependent packets pending and record the observed limitation.

## Operator Visual Check

Accepted - 2026-09-11, including preservation and fixture cleanup. These steps
remain available for regression; no repeat is requested for this completion.

Follow [Task 005: Session Drain And Resumption](../runbooks/startup-recovery-check.md#task-005-session-drain-and-resumption)
for the complete commands, expected results and cleanup. Use a Linux desktop,
Python 3.11+ and a rebuilt debug binary. No audio hardware, installed mpv or
external service is needed. Create a new fixture and retain task 004's fixture.

1. Prepare the held command and launch from a desktop terminal:

```bash
cd /home/citizen/build/v4vmm
cargo build --locked --offline --quiet
session_fixture=$(python3 docs/runbooks/startup-recovery-fixture.py setup)
python3 docs/runbooks/startup-recovery-fixture.py verify "$session_fixture"
python3 docs/runbooks/startup-recovery-fixture.py mode "$session_fixture" session-held-command
python3 docs/runbooks/startup-recovery-fixture.py run "$session_fixture"
```

2. V1: open the fixture playlist, then Settings → Diagnostics. Inspect the
   explanation and keyboard access in both themes and normal/narrow widths
   without saving theme changes. End app session must keep the window responsive.
3. V2: the held command must cause a bounded failure with Retry drain and Copy
   report. Maintenance must not open early. Use `session-release` in a second
   terminal, then retry and inspect recovery plus the preserved report.
4. V3: Open app must show the preserved library once under one fresh generation.
   Compare `session-status` observations and inspect the retained report.
5. Quit, run `inspect`, record the results, then run `cleanup` as the runbook
   specifies. Keep a failed fixture for diagnosis. An unwalked step stays open.
