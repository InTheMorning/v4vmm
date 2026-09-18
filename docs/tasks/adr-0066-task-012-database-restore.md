# ADR 0066 Task 012: Database Restore

Status: Complete - 2026-09-17; mechanical checks Green. Operator V1–V3,
presentation and both preservation inspections accepted; fixture cleanup
confirmed. Task 013 is now implemented with its separate operator gate open.

## Goal

Restore an explicitly chosen validated backup through the shared maintenance path, preserve the current database, and reopen only after verification.

## Read First And Dependency

Read [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md),
the [phase plan](../plans/adr-0066-startup-recovery-phase-plan.md), this whole packet, and the
[review checklist](../reviews/adr-0066-startup-recovery-review-checklist.md).
Execute after [task 011](adr-0066-task-011-database-maintenance-and-preservation.md) in the phase plan's order.
Complete this packet in one session; do not start its successor.

## Files To Inspect

- src/db/maintenance.rs — inspection, snapshot, exclusive access and preservation
- src/application/commands/maintenance.rs; src/view_models/startup.rs
- Session drain and resumption packet — connection closure and generation invalidation
- src/presentation/maintenance_executor.rs; src/app/startup.rs; src/ui/composites/maintenance_forms.rs
- rusqlite::backup::Backup — source/destination and transaction requirements
- `tests/architecture_tests.rs`; `AGENTS.md`

## Changed Owners

- New `src/db/maintenance/restore.rs`; existing `src/db/maintenance.rs`,
  `src/db/maintenance/preservation.rs` and `src/db/startup.rs`.
- `src/application/commands/maintenance.rs`, `src/view_models/startup/database.rs`,
  `src/presentation/database_tools.rs`, `src/app/startup.rs` and
  `src/ui/composites/maintenance_forms.rs`.
- `src/startup/fixture.rs`, `tests/architecture_tests.rs`, existing fixture,
  fixture tests and operator runbook under `docs/runbooks/`.
- This packet, phase/delivery/deferred indexes, ADR status/index, review
  checklist, pending-human index, documentation index, source map and AGENTS.md.

## Do Not Touch

- Automatic restore, pathname swapping under live connections, or new empty database fallback
- Unknown/corrupt schema salvage or repair SQL (013 owns the recognized recipe)
- Music relocation, token-file restore or dropping unrelated database records to pass checks
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

The implementation recipe is retired into the following owners and regression
proofs. ADR 0066's binding decisions remain unchanged.

| Criterion | Owner and evidence |
|---|---|
| C1: validated, unchanged candidate and destination | `ValidatedRestore::review/revalidate` in `src/db/maintenance/restore.rs`; `adr_0066_restore_rejects_invalid_newer_older_and_aliased_candidates`, `adr_0066_restore_changed_files_or_records_require_new_review`. Review creates a private current-schema SQLite snapshot and binds file fingerprints, destination identity/records, configuration digest and session generation. |
| C2: explicit review and ordered prerequisites | `DatabaseCommand::execute_restore`, existing managed drain and `ValidatedRestore::install_with`; `adr_0066_restore_command_requires_review_drain_and_unchanged_configuration`, `adr_0066_restore_failed_preservation_never_installs`. Ordinary command execution refuses installation. |
| C3: failure, rollback and retained artifacts | `adr_0066_restore_interrupted_and_cancelled_steps_verify_sqlite_rollback` uses multi-step SQLite copies in rollback and WAL modes and real subprocess contention. Incomplete backup drop finishes SQLite before verification; a fresh bounded verification budget compares all schema/record fingerprints. Unknown rollback is reported as unknown. |
| C4: installed records and resumption | `adr_0066_restore_preserves_installs_and_verifies_without_replacing_inode`, `adr_0066_restore_verification_requires_candidate_records_and_usable_writes`; startup's existing check/open/mount generation owner admits one fresh session only after installation verification and fresh core checks. The configured database inode remains unchanged. |
| C5: ownership guard | Situational `adr_0066_restore_uses_validated_maintenance_install` in `tests/architecture_tests.rs`, ADR 0066 invariant 6. Existing exclusive-access and session-drain guards remain. |
| Reports and actions | `DatabaseVm`, `adr_0066_restore_review_is_explicit_bound_to_inputs_and_one_session`, `adr_0066_restore_reports_distinguish_rollback_and_failed_verification`; explicit destructive Restore requires a reviewed candidate, names both locations, and explains database-only scope. Reports retain recorded UTC and recovery paths. |
| Damaged original | `adr_0066_restore_lockable_damage_keeps_file_evidence` proves exact file preservation without calling the damaged copy a backup. Unsupported headers/exclusive access continue to refuse without unlinking. |
| Fixture inspection | `RestorePreservationTests` in the existing Python fixture test file rejects changed backups, replaced destination inode, damaged/missing preservation evidence and duplicate/missing session observations. |

Shared UI ownership: `src/view_models/startup/database.rs` owns facts, wording,
availability and destructive intent; `src/presentation/database_tools.rs` owns
input/worker marshalling; `src/ui/composites/maintenance_forms.rs` owns shared
geometry. Existing Spacing/FontSize tokens, ControlStyle::Destructive and the
shared log frame provide the token/verification path. `src/app/startup.rs`
wires the existing managed drain and fresh-session mount. No renderer performs
database I/O or schema decisions.

Restore retains its candidate and original artifacts for recovery. The selected
backup must be a standalone snapshot in rollback-journal mode, with no
sidecars or hard-link aliases;
live databases first use task 010's verified backup. Older schemas require task
013's later explicit migration preparation. Review rejects changed database
records, including work completed during drain: review again in recovery.
SQLite installation owns transaction rollback; the app never swaps or unlinks
the destination or its journals. This follows the installed rusqlite 0.38
backup handle's Drop implementation and [SQLite's backup contract](https://www.sqlite.org/c3ref/backup_finish.html).

The debug-only `startup::fixture::interrupt_database_restore` activates only for
a verified fixture/config/binary identity in a restore case. Release builds use
no interruption. The new fixture seed reuses the Rust schema/backup authority;
Python orchestrates files, locks, inspection and cleanup.

### Mechanical Evidence — 2026-09-17

- `cargo check`, `cargo fmt -- --check`, `cargo clippy -- -D warnings`: Green.
- Full suite: 1,466 unit tests and 258 architecture guards Green; ten existing
  documentation examples remain ignored. Sandbox socket restrictions required
  an unrestricted rerun, which is Green.
- Focused restore suite: eleven owner tests and the restore guard Green.
- Python fixture suite: 35 tests Green.
- Normal `cargo build --bin v4vmm` after tests: Green.
- Both restore fixture cases: backend-only setup, current-schema seed, actual
  writer contention/release, failure controls, unchanged chosen backup and
  rejection of an unperformed restore by the inspector Green. Smoke fixtures
  `/tmp/v4vmm-startup-mqgl152m` and `/tmp/v4vmm-startup-5ob_wy8t` were removed.
  This smoke did not launch a desktop or claim successful operator acceptance.
- Changed-document local links/anchors and `git diff --check`: Green.

Logs: `/tmp/v4vmm-0066-012-tests-unrestricted.log`,
`/tmp/v4vmm-0066-012-final-focused.log`, `/tmp/v4vmm-0066-012-fixture-tests.log`,
`/tmp/v4vmm-0066-012-smoke.log`, `/tmp/v4vmm-0066-012-build.log`.
Mechanical results do not accept the visual gate.

### Operator Evidence — 2026-09-17

The operator walked the desktop procedure and accepted the following results.
This date uses America/Montreal; copied reports retain their recorded UTC,
including the recovery review after midnight on 2026-09-18 UTC.

| Check | Observed result and acceptance |
|---|---|
| V1: normal session | Fixture `/tmp/v4vmm-startup-nzvj4k26`: review named the chosen backup, configured database, preservation directory, private candidate and database-only scope. Editing the preservation path disabled Restore. Explicit installation reopened Music in the same window with both fixture playlists; operator accepted. |
| V1: retained report and presentation | Settings and recovery controls/reports passed normal/narrow inspection. Copied history retained full paths and UTC. The successful installation entry at `2026-09-17 23:44:47 UTC` names the file manifest and verified snapshot of the previous database, then confirms installation and verification. |
| V3: rejected prerequisites | Invalid header refused at `23:33:07 UTC`; newer schema refused at `23:34:26 UTC`. A real writer prevented exclusive access at `23:35:44 UTC`; the candidate remained available and installation did not begin. The occupied preservation path refused installation at `23:41:04 UTC` without overwriting existing files. These times are on 2026-09-17 UTC. |
| V3: interrupted installation | At `2026-09-17 23:42:18 UTC`, the report named preserved original files and a verified snapshot, reported interruption between SQLite page batches, and confirmed the destination schema and records matched their pre-installation fingerprint after rollback. Recovery remained open; operator accepted. |
| V2: damaged original | Fixture `/tmp/v4vmm-startup-vbz7ithx`, confirmed as `database-restore-recovery`: operator accepted review, chosen-backup input invalidation, explicit restore, same-window reopening and both playlists. The damaged original remained labelled as a file copy without a verified-snapshot claim. |
| Preservation | After closing each app, the operator reported the normal fixture's `restore-inspect` passed and the recovery fixture's flags were all true. The inspectors cover backup bytes, original files/manifests, snapshot eligibility, configured inode, restored records/ledger, music, token sentinel, configuration and session observations. |
| Cleanup | The operator confirmed cleanup of `/tmp/v4vmm-startup-nzvj4k26`, `/tmp/v4vmm-startup-vbz7ithx` and the additional `/tmp/v4vmm-startup-eloo_1ay` fixture. |

The first V2 review exposed a fixture-path mismatch: the running app used
`vbz7ithx`, while the supplied backup/preservation paths used `eloo_1ay`.
The review correctly named the configured destination. After the fixture mode
was confirmed, both inputs were corrected to `vbz7ithx` and reviewed again for
the accepted restore. The regression procedure now explicitly checks that all
fixture paths match the running app before installation.

These confirmations close only task 012. Task 004 and the inherited human
checks retain their separate gates. ADR 0066 remains Accepted and partial;
task 013 is unstarted.

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

**Accepted — 2026-09-17**: [task 012 operator procedure](../runbooks/startup-recovery-check.md#task-012-database-restore)
remains a regression check. Evidence is recorded above.

- V1: Settings review, explicit Restore, automatic same-window resumption,
  distinct restored playlists, retained Copy report and normal/narrow controls.
- V2: core recovery with lockable damage, labelled original-file preservation,
  successful restore and fresh-session resumption.
- V3: invalid/newer candidates, real destination contention, occupied
  preservation location and interrupted installation with verified rollback.
- Both fixtures: preservation inspector, configuration/music/token protection,
  retained recovery artifacts, configured inode and cleanup.

The runbook contains exact setup, failure control, launch, inspection and cleanup
commands. Agents do not launch the desktop. All checks above and cleanup are
accepted for this packet; its entry is removed from the pending-human index.

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

The installed SQLite backup API cannot preserve the required exclusive/rollback behavior, or restoring a damaged destination would require unlinking it. Stop that path with a tested actionable report; a filesystem swap needs a separately reviewed protocol.
Routine placement inside the named owner is authorized. If a boundary needs a
new architectural decision, name the conflict and proposed bounded correction
before widening this packet.
