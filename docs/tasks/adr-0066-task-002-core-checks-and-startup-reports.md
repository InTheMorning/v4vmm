# ADR 0066 Task 002: Core Checks And Startup Reports

Status: Complete - 2026-09-10.
Mechanical gate Green. All operator checks passed and fixture cleanup is
confirmed; evidence is recorded below. [Task 003](adr-0066-task-003-runtime-failure-and-shell-availability.md)
owns runtime/cache follow-through and its separate acceptance gate.

## Goal

Show a useful recovery screen for broken core configuration, unusable music storage, or unusable SQLite, with safe checks and a single startup lifecycle.

## Read First And Dependency

Read [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md),
the [phase plan](../plans/adr-0066-startup-recovery-phase-plan.md), this whole packet, and the
[review checklist](../reviews/adr-0066-startup-recovery-review-checklist.md).
Execute after [task 001](adr-0066-task-001-config-snapshot-and-safe-persistence.md) in the phase plan's order.
Complete this packet in one session; do not start its successor.

## Files To Inspect

- src/app/bootstrap.rs — run_app and all fallible startup/window calls
- src/main.rs; src/lib.rs; src/app.rs — run_app export and TopApp mount
- src/config.rs — task 001 snapshot and ensure_dirs
- src/db.rs — open_db, init_schema, migrate_schema, repair_local_file_paths, LocalPathRepairSkip
- src/presentation/mod.rs; src/ui/composites/mod.rs; src/view_models/mod.rs
- src/ui/primitives/button.rs; src/ui/tokens.rs; src/ui/control_styles.rs; src/ui/theme_bridge.rs
- docs/adr/0040-async-vm-runtime.md; docs/adr/0064-local-file-addressing.md
- `tests/architecture_tests.rs`; `AGENTS.md`

## Changed Owners

- src/startup.rs; src/startup/storage.rs; src/view_models/startup.rs (new)
- src/startup/fixture.rs (new) — debug-only fixture dispatch; backend schema construction remains in the db module
- src/presentation/maintenance_executor.rs (new); src/presentation/startup_presenter.rs (new)
- src/app/startup.rs (new); src/ui/composites/startup_report.rs (new)
- src/config.rs — storage setup split; src/db.rs; src/db/startup.rs (new) — check-only and preparation entry points
- src/app/bootstrap.rs; src/main.rs; src/app.rs — startup composition only
- src/lib.rs; src/view_models/mod.rs; src/presentation/mod.rs; src/ui/composites/mod.rs — module registration
- Existing named tokens and control styles are reused unchanged.
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

## Implementation And Proof

Procedures now live in the following owners and guards. The stage inventory
below retains the handoff to tasks 003/004; ADR decisions remain binding.

| Responsibility | Live owner |
|---|---|
| Core admission, independent field issues, fresh input comparison and explicit preparation | `StartupBackend`, `CoreCheckOutcome`, `PreparedCore` in [startup.rs](../../src/startup.rs) |
| Existing music-root verification and owned probes | `check_music`, `probe_with` in [storage.rs](../../src/startup/storage.rs); `ConfigSnapshot::created_defaults` limits first-run root creation |
| Check-only SQLite access, complete column contract, normal migration preparation and rolled-back main-database probe | `check_database`, `prepare_database`, `check_schema`, `probe_main_database` in [db/startup.rs](../../src/db/startup.rs), a child of the existing db owner |
| Download subtree preparation | `prepare_artists_directory` in [config.rs](../../src/config.rs); its failure becomes an optional notice |
| One independent worker and one admitted operation | `MaintenanceWorker`, `MaintenanceClient::submit` in [maintenance_executor.rs](../../src/presentation/maintenance_executor.rs); bootstrap joins outstanding work after the desktop loop exits |
| Completion, one normal mount and queued window-close shutdown | `present_startup`, `mount_current`, `window_disposition`, `quit_after_window_close` in [startup_presenter.rs](../../src/presentation/startup_presenter.rs) |
| Typed generation, availability, persistent check completion, UTC report and full-copy text | `StartupReportVm::feedback`, `StartupReportVm::complete`, `format_report` in [view_models/startup.rs](../../src/view_models/startup.rs) |
| Window wiring and reusable report geometry | [app/startup.rs](../../src/app/startup.rs), [bootstrap.rs](../../src/app/bootstrap.rs), [startup_report.rs](../../src/ui/composites/startup_report.rs) |
| Fixture schema construction and inspection | Debug-only [startup/fixture.rs](../../src/startup/fixture.rs); [Python orchestration](../runbooks/startup-recovery-fixture.py) never duplicates SQL schema |

The SQLite probe uses a savepoint inside an outer transaction. Rolling back the
outer transaction preserves the isolated database's file bytes; releasing a
rolled-back root savepoint alone did not. The failure tests enforce this.
`CURRENT_COLUMNS` is a read-compatibility contract, not another schema writer;
its coverage test compares it with the normal migration registry's actual schema.

The compatibility `ensure_dirs` wrapper remains for existing non-startup callers.
Normal startup now splits music/root, artists and new database-parent handling.
Strict optional configuration, the second endpoint read, path-repair errors,
player/producer constructors and runtime/thumbnail failures retain their explicit
003/004 handoff. This packet does not claim optional-tool recovery is complete.

## Mechanical Evidence

Green - 2026-09-10:

- `cargo fmt -- --check`, `cargo check --quiet`, `cargo clippy --quiet -- -D warnings`
- `cargo test --quiet`: 1,299 unit tests and 224 architecture guards pass;
  10 existing documentation examples remain ignored.
- `cargo build --quiet`
- Fixture setup, verify/locate, all six modes, inspect and cleanup. Every case
  preserved expected config/music bytes and migration records with no residual
  probe. The GUI launcher was not run.

The full suite required local socket access outside the sandbox. Its existing
HTTP/mpv socket tests failed or waited inside the restricted sandbox; the
socket-enabled rerun passed. No code or gate was weakened for that restriction.

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

Tests are situational ADR 0066 guards. All names below exist in their named
owner; copied report text and UI layout still need the human check.

| ID | Mechanical evidence |
|---|---|
| C1 | storage tests `adr_0066_music_root_is_not_recreated_for_existing_configuration`, `adr_0066_music_file_and_probe_failures_preserve_audio`, `adr_0066_music_permissions_identify_list_and_write_failures`; startup test `adr_0066_artists_failure_and_optional_errors_are_scoped` |
| C2 | db/startup tests `adr_0066_database_checks_leave_schema_rows_and_file_unchanged`, `adr_0066_missing_database_check_does_not_create_or_upgrade`, `adr_0066_corrupt_wrong_schema_and_migration_gaps_are_named`, `adr_0066_main_database_lock_is_bounded_and_reported`, `adr_0066_read_only_configured_database_rejects_the_write_probe`, `adr_0066_failed_probe_rolls_back_its_main_table`, `adr_0066_schema_read_contract_covers_the_current_registry` |
| C3 | startup tests `adr_0066_checks_do_not_prepare_and_changed_inputs_require_new_consent`, `adr_0066_core_failure_produces_no_prepared_connection_and_preserves_bytes`; presentation test `adr_0066_only_the_current_preparation_mounts_a_normal_factory_once` |
| C4 | VM tests `adr_0066_report_uses_recorded_utc_and_keeps_full_path_and_action`, `adr_0066_parse_report_omits_source_excerpts_and_debug_bytes`, `adr_0066_endpoint_credentials_are_removed_before_debug_or_reporting` |
| C4a | VM tests `adr_0066_identical_rechecks_keep_distinct_visible_completions`, `adr_0066_worker_failure_does_not_claim_a_completed_check`; the existing `adr_0066_recorded_report_context` guard keeps completion feedback outside disclosure and scrolling, with VM-owned recorded time |
| C5 | worker tests `adr_0066_worker_start_failure_is_fallible_without_a_runtime`, `adr_0066_worker_rejects_duplicates_and_finishes_after_receiver_closes`; VM tests `adr_0066_stale_closed_and_repeated_results_cannot_mount`, `adr_0066_worker_failure_leaves_copy_and_quit_available`; presenter test `adr_0066_window_failure_and_closed_window_exit_but_activation_is_nonfatal` |
| C5a | Situational architecture guard `adr_0066_window_manager_close_queues_quit` keeps Quit on the foreground queue after the platform close callback; it preserves worker joining and the existing startup-dependent exit result. Actual window-manager closes require the runbook's operator checks. |
| C6 | `adr_0066_core_recovery_ownership` and `adr_0066_recorded_report_context` in [architecture_tests.rs](../../tests/architecture_tests.rs) |
| C7 | Existing workspace persistence, cx.spawn and macOS menu guards remain Green. The screen inventory adds app/startup.rs. The metadata storage guard recognizes db/startup.rs as part of the existing db module; it still rejects raw table access outside that owner. |

Documentation proof: implemented steps and the coding prompt are retired in
favor of these owners/tests. The stage table keeps every unimplemented boundary
assigned. Packet, plan, ADR partial line, delivery row and pending human checks
agree that operator acceptance is recorded and its visual gate is closed.
Fixture cleanup is confirmed; task 002 is complete.

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

Accepted - 2026-09-10. All sections of [Task 002: Core checks and reports](../runbooks/startup-recovery-check.md#task-002-core-checks-and-reports)
passed, including repeated-check feedback and window-manager closing. The
runbook remains for regression checks and fixture cleanup. Passed cases need
no repeat.

### Operator Evidence — 2026-09-10

Fixture: `/tmp/v4vmm-startup-2527c3uu`. Build revision was not supplied.
The operator supplied the invalid-TOML report recorded at 21:05:04 UTC and
the fixture's inspection result.

- Report text: verified. It identifies the configuration path, line 7,
  column 12, the parsing failure, the consequence and the next action.
  UTC timestamps survived the supplied text.
- Preservation: verified. Config bytes, music and migration records are
  unchanged; the playlist count is 1, migration versions are 1–11, and no
  music or database probes remain.
- The operator's final confirmation also accepts the TOML screen: Music and
  Show are absent, Open app is unavailable, disclosure works, and Quit exits
  with a nonzero code.

The earlier fixture's manifest was subsequently missing. The operator verified
a fresh fixture at `/tmp/v4vmm-startup-ibmtye2t`, using this checkout's
`target/debug/v4vmm`.

- Missing music directory: visual/recheck pass confirmed by the operator.
  Recovery names the missing folder; Check again retains the error and Open
  app remains unavailable.
- Missing-folder preservation: pass confirmed by the operator after Quit and
  inspection. Preservation results are true and no music or database probes
  remain. This case is complete.
- File-instead-of-directory: visual and preservation pass confirmed by the
  operator. The app names the wrong path type, keeps Open app unavailable,
  preserves fixture data and leaves no probes.
- The operator reported that Check again gives no perceptible acknowledgement
  when a fast check returns the same failure. The VM now retains a numbered,
  UTC completion line above disclosure and scrolling; the active check button
  says Checking while work runs. Copy report includes that same feedback.
  The receipt records completion once; rendering never advances its time.
  See C4a and the [focused recheck](../runbooks/startup-recovery-check.md#3a-confirm-repeated-checks).
  The operator confirmed the recheck: each completed check advances its number,
  feedback stays visible with details hidden, and the copied report includes
  the completion line. The accepted path/data cases remain passed.
- Direct path editing remains assigned to task 006. The operator also requested
  creating a new setup; that separate workflow needs definition before it can
  change the current preservation policy. The note is recorded in
  [task 006](adr-0066-task-006-configuration-repair-and-resumption.md#operator-notes--2026-09-10).
- Locked database: pass confirmed by the operator. Checking appears during
  the bounded wait, recovery identifies the lock, the window stays responsive
  and Open app remains unavailable.
- After releasing the lock, the operator supplied the report at 21:42:38 UTC
  confirming music and SQLite checks passed. Closing through the window manager
  then panicked in GPUI's X11 client with `RefCell already borrowed`, exit 101.
  Bootstrap had called Quit inside the platform close callback. The presenter
  now queues Quit on the foreground executor; C5a guards this boundary.
  The operator confirmed the normal-window recheck: Music opens and the same
  window-manager close exits with code 0 and no panic.
- Normal-session preservation: pass confirmed by the operator. Configuration,
  music and migration preservation checks are true with no residual probes.
- Long paths and blocked recovery closing: pass confirmed by the operator.
  The supplied report at 21:58:04 UTC contains the complete long path and check
  completion. Normal/narrow text wrapping, scrolling, reachable buttons and
  copied text passed. Window-manager closing exits with code 1 and no panic;
  preservation inspection passes with no residual probes.
- The operator's final confirmation accepts same-window resumption after the
  database lock was released: Check again passes and Open app mounts Music in
  the existing window. All operator acceptance checks are closed.
- Fixture cleanup: the operator confirmed completion on 2026-09-10 after
  receiving the fixture helper's cleanup command. Task 002 is complete.

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
