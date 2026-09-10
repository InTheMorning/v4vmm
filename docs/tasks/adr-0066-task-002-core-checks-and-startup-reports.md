# ADR 0066 Task 002: Core Checks And Startup Reports

Status: Ready after the preceding packet - 2026-09-10.
Implementation not started. Operator check specified below; not runnable or accepted yet.

## Goal

Show a useful recovery screen for broken core configuration, unusable music storage, or unusable SQLite, with safe checks and a single startup lifecycle.

## Read First And Dependency

Read [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md),
the [phase plan](../plans/adr-0066-startup-recovery-phase-plan.md), this whole packet, and the
[review checklist](../reviews/adr-0066-startup-recovery-review-checklist.md).
Execute after [task 001](adr-0066-task-001-config-snapshot-and-safe-persistence.md) in the phase plan's order.
Complete this packet in one session; do not start its successor.
Names marked **new**, including tests and guards, are implementation targets,
not claims that those files or symbols already exist. If a predecessor already
created a listed owner, extend that owner.

## Files To Inspect

- src/app/bootstrap.rs — run_app and all fallible startup/window calls
- src/main.rs; src/lib.rs; src/app.rs — run_app export and TopApp mount
- src/config.rs — task 001 snapshot and ensure_dirs
- src/db.rs — open_db, init_schema, migrate_schema, repair_local_file_paths, LocalPathRepairSkip
- src/presentation/mod.rs; src/ui/composites/mod.rs; src/view_models/mod.rs
- src/ui/primitives/button.rs; src/ui/tokens.rs; src/ui/control_styles.rs; src/ui/theme_bridge.rs
- docs/adr/0040-async-vm-runtime.md; docs/adr/0064-local-file-addressing.md
- `tests/architecture_tests.rs`; `AGENTS.md`

## Files Likely To Change

- src/startup.rs (new); src/view_models/startup.rs (new)
- src/startup/fixture.rs (new) — debug-only fixture dispatch/failure seams; backend schema construction remains in db.rs
- src/presentation/maintenance_executor.rs (new); src/presentation/startup_presenter.rs (new)
- src/app/startup.rs (new); src/ui/composites/startup_report.rs (new)
- src/config.rs — storage setup split; src/db.rs — check-only and preparation entry points
- src/app/bootstrap.rs; src/main.rs; src/app.rs — startup composition only
- src/lib.rs; src/view_models/mod.rs; src/presentation/mod.rs; src/ui/composites/mod.rs — module registration
- src/ui/tokens.rs; src/ui/icons.rs — only needed named report roles
- tests/architecture_tests.rs
- docs/runbooks/startup-recovery-fixture.py (new); docs/runbooks/startup-recovery-check.md (new)
- This packet's Status/evidence, the phase plan and review checklist.
- ADR 0066's guard references/partial line and the delivery/deferred indexes as
  appropriate; `docs/pending-human-checks.md` when a runnable visual gate opens.
  Update `AGENTS.md` when the next executable packet changes.

## Do Not Touch

- Optional runtime/player policy changes assigned to 003/004
- Config editors, database restore, and conversion behavior
- ADR 0064 repair history surface; log navigation and Show layout
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

1. Add renderer-free CoreCheckOutcome, StartupIssue and PreparedCore owners in startup.rs. Issues carry stage, safe resource context, recorded SystemTime, consequence, and next action; convert transport/I/O errors before display. Config issues use task 001's snapshot. Keep ordinary bootstrap preparation and non-preparing recovery-check paths distinct.

2. Split ensure_dirs by purpose. An existing configuration's absent music root is an error; first-run setup may create its default root. Verify listing and an exclusively created unique probe: write bytes, read them back, remove the probe. Do not traverse audio content. Failure creating artists affects downloads that require it, not root validity. Database-parent creation belongs only to preparation of a legitimate new database.

3. Add database checks that open the configured main database with explicit flags, a named five-second busy deadline, schema/read verification, and a rolled-back main-database write probe. Use a SAVEPOINT and a uniquely named ordinary table created/inserted/read/dropped inside that savepoint, then roll back; TEMP/in-memory writes do not count. Confirm no retained schema/data change. Recovery checks never call init_schema, migrate_schema, open_db, or path repair. Report a supported older schema as needing explicit preparation/upgrade, not as corruption; task 013 adds interrupted-upgrade repair.

4. Initial preparation may retain the existing first-run schema/migration policy. A recovery Check again never repeats that preparation. Open app consumes a successful check generation, rereads/revalidates if inputs changed, performs final preparation once, and mounts normal operations once. Unprepared first-run resources are reported as needing setup until explicit Open app performs it. Do not make repeated checking create directories/databases or apply repair statements.

5. Use one independent maintenance worker with fallible std::thread::Builder creation, a bounded queue of one pending request, and one running operation. Reject duplicate requests with typed availability. Backend work never runs in GPUI callbacks. Completion is marshalled through presentation; no RuntimeHost or main Connection is required. A worker-start failure leaves Copy report/Quit and the failure visible. Drop stale check results using a generation; do not drop a worker handle and pretend its ongoing I/O stopped.

6. Build StartupReportVm and the shared startup_report composite: short subject/cause/action, optional safe details, Copy report, Check again, Open app when checks permit it, and Quit. Use installed pre-config Dark/Medium theme, named tokens, typed availability and accessibility labels. Details wrap, scroll and copy in full. Task 006 adds correction editors to this same owner; this packet's inspection does not claim those later tools are complete.

   Build parse diagnostics from safe error category, field name and location;
   never forward TOML's source excerpt or rejected value wholesale. Redact URL
   credentials before formatting optional-service errors. Reuse the same safe
   report text for UI, clipboard and stderr. Clipboard dispatch is not proof
   of a successful paste: GPUI's write API supplies no such acknowledgement.

7. Wire run_app/main to typed core outcomes. No normal library/show actors or autosaves mount on core failure. Do not catch programmer panics. Classify every bootstrap boundary using the stage inventory below; runtime/driver replacements remain explicitly assigned to 003/004 and are not new core requirements. Return an unsuccessful exit if recovery closes before normal startup; normal close after Open app uses ordinary success semantics.

8. Handle open_window failure once: print the safe report and window error to stderr; do not recursively open an error window. A failed activation of an existing window is nonfatal; a closed window ends startup. Preserve the config-parent, fixed ApplicationServices wiring, and unique-frame programmer assertions. Test adapters rather than trying to exhaust OS resources or launch GPUI in an agent session.

9. Implement the isolated fixture/runbook contract from the phase plan for core cases. Its validate/inspect commands must prove fixture identity, config byte preservation, probe cleanup, database contents and migration records without launching the app.

## Startup Stage Inventory

This table owns the procedure inventory moved from ADR 0066. Symbols take
precedence over shifting line numbers.

| Existing boundary in run_app | Disposition | Completion owner |
|---|---|---|
| config_path resolution | Recovery with unresolved-path wording; environment correction may require relaunch | 002 |
| Config parent/default creation | Only first run; failed publication names path and residual artifact | 001/002 |
| load_config read/TOML/core validation | Core recovery, existing bytes preserved | 001/002 |
| load_musicindex_endpoint second read | Removed; one snapshot and scoped MusicIndex issue | 004 |
| Theme/scale/layout decoding | Documented default, visible issue, persistence paused | 001/004 |
| ensure_dirs | Core root/database parent distinguished from artists subtree | 002 |
| open_db connection/pragmas/schema/migrations/probe | Core recovery, named substep; no reset | 002 |
| repair_local_file_paths error | Recheck core usability; contain unverified bindings without claiming rollback | 004 |
| LocalPathRepairSkip | MusicFolderMissing blocks core, NothingResolved alone does not; preserve bindings | 002/004 |
| ConfiguredPlaybackDriver::from_config | Scoped issue; no failed-mpv-to-Null substitution or startup ping | 004 |
| BroadcastConfig/DropFileProducer construction | Independent publisher host, producer, encoder issues | 004 |
| RuntimeHost::new | Shell opens; dependent work unavailable; no implicit runner | 003 |
| ImageCache startup eviction | Nonfatal scoped worker/prune issue | 003 |
| config path parent expectation | Programmer invariant, remains explicit | 002 |
| open_window | Safe stderr report and unsuccessful exit if no window | 002 |
| window update/activation | Existing window: report and continue; gone window: shutdown | 002 |

Task 002 does not claim the optional-tool paths assigned to 003/004 are fixed.
Keep ADR 0066 Accepted and record this partial boundary in the completion report.

## Acceptance Criteria

Mechanical; assert at the named owner. Test/guard names below marked new must
be implemented by this packet.

| ID | Proof owner | Required assertion |
|---|---|---|
| C1 | startup.rs/config.rs filesystem tests | Absent configured root, read-only/unlistable root, file-as-directory, probe write/read-back/cleanup failure produce the correct typed operation/path. Existing audio bytes remain unchanged. Artists-only failure is scoped. |
| C2 | db.rs tests | Fresh preparation and existing database work. Corrupt/read-only/locked/wrong-schema/probe failures identify their substep; lock handling is bounded and no probe changes persist. |
| C3 | injected startup factory tests | A core failure prevents all normal-app factories. Check again does not migrate, repair, or create resources. Explicit Open app prepares once; obsolete results cannot mount an app. |
| C4 | StartupReportVm tests | Reports carry actual recorded UTC, subject, path or unresolved-path wording, cause, consequence, next action and complete copied text. TOML source excerpts, credentials and token contents never reach display, stderr or clipboard. |
| C5 | presentation adapter tests | Worker creation failure, duplicate checks, closed completion target, window failure, activation failure and quit have defined outcomes; no duplicate actor/resource construction or implicit normal runtime. |
| C6 | new situational guards adr_0066_core_recovery_ownership and adr_0066_recorded_report_context | Core recovery excludes ordinary dispatch/autosave; maintenance I/O stays in backend/presentation owners; reports have one safe formatter. Cite invariants 1, 5, 7–8. |
| C7 | existing architecture guards | Preserve workspace_frame_phase_5_layout_persistence_contract, workspace_pane_width_persistence_contract, cx_spawn_is_restricted_to_presentation_runtime_and_bootstrap, and macos_app_menu_bootstrap_exposes_standard_app_commands; update location assertions only. |

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

1. Launch the fixture's invalid-toml case. Confirm the file path, line/column, consequence and next action make sense. Copy the report and compare complete text. The normal Music/Show shell must not mount.

2. Launch music-missing, music-file and db-locked cases separately. The subject must change to the failing storage operation. Release the fixture lock, use Check again, then Open app. It must open once without relaunch.

3. Inspect a long path at normal and narrow widths. Summary and actions must stay reachable; details must wrap and copy completely. Close an unresolved recovery and inspect the nonzero exit printed by the fixture launcher.

4. Run fixture inspect to compare config checksums, music bytes, migration records and probe cleanup. Remove only this fixture using its cleanup command.

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

A core check requires migration or destructive repair, the report requires the normal database/runtime, or a window-system API cannot express the specified lifecycle. Preserve the named invariant and report the exact boundary.
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
- Show a useful recovery screen for broken core configuration, unusable music storage, or unusable SQLite, with safe checks and a single startup lifecycle.

Constraints:
- Follow this packet's Constraints and Implementation Steps.
- Preserve its data, dependency and prose-retirement contracts.
- One packet this session. Never run the app.

Do not touch:
- Optional runtime/player policy changes assigned to 003/004
- Config editors, database restore, and conversion behavior
- ADR 0064 repair history surface; log navigation and Show layout

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
