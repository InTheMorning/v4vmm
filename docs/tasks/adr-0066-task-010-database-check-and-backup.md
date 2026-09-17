# ADR 0066 Task 010: Database Check And Backup

Status: Complete - 2026-09-17; mechanical checks Green; operator gate closed.
Operator V1–V3, Settings/recovery presentation, report copy, responsiveness and
preservation in both fixture cases are accepted. Normal-mode restoration and
fixture cleanup are confirmed. Task 011 completion and acceptance are recorded in
[its packet](adr-0066-task-011-database-maintenance-and-preservation.md).

## Goal

Offer database inspection and a verified SQLite backup from both Settings and core recovery, without requiring the normal app runtime.

## Read First And Dependency

Read [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md),
the [phase plan](../plans/adr-0066-startup-recovery-phase-plan.md), this whole packet, and the
[review checklist](../reviews/adr-0066-startup-recovery-review-checklist.md).
Execute after [task 009](adr-0066-task-009-conversion-retry-and-retained-input.md) in the phase plan's order.
Complete this packet in one session; do not start its successor.

## Files To Inspect

- src/db.rs — open_db, init_schema, MIGRATIONS, migrate_schema, schema_migrations
- Cargo.toml; Cargo.lock — rusqlite 0.38 with bundled, backup and hooks features
- src/application/commands/maintenance.rs; src/view_models/startup.rs
- src/presentation/maintenance_executor.rs; src/ui/composites/maintenance_forms.rs
- docs/adr/0016-schema-migration-discipline.md
- `tests/architecture_tests.rs`; `AGENTS.md`

## Changed Owners

- src/db/maintenance.rs; src/db.rs; src/db/startup.rs — inspection, snapshots and shared schema authority
- Cargo.toml — rusqlite backup/hooks features; Cargo.lock unchanged
- src/application/commands/maintenance.rs; src/view_models/startup.rs;
  src/view_models/startup/database.rs; src/view_models/settings.rs; src/view_models/log_view.rs
- src/ui/composites/maintenance_forms.rs; src/ui/composites/startup_report.rs;
  src/presentation/database_tools.rs; src/presentation/mod.rs;
  src/app/startup.rs; src/app.rs; src/app/settings.rs — shared tools route
- tests/architecture_tests.rs; src/startup/fixture.rs;
  docs/runbooks/test_startup_recovery_fixture.py; docs/runbooks/startup-recovery-fixture.py; docs/runbooks/startup-recovery-check.md
- This packet's Status/evidence, the phase plan and review checklist.
- ADR 0066's guard references/partial line and the delivery/deferred indexes as
  appropriate; `docs/pending-human-checks.md` when a runnable visual gate opens.
  Update `AGENTS.md` when the next executable packet changes.

## Do Not Touch

- Active database replacement, migration recipes, arbitrary corruption salvage
- A second schema registry or new durable maintenance tables
- Backup of music or broadcaster token files
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

The implementation recipe and coding prompt are retired in favor of the
situational guard `adr_0066_database_checks_and_snapshots_have_one_owner`
(ADR 0066 invariant 6 and ADR 0016) in `tests/architecture_tests.rs`.

- `db::inspect_schema` keeps the existing read contracts beside `MIGRATIONS`.
  Startup and maintenance share that authority. `db::maintenance::inspect`
  reports access, integrity, foreign keys and compatibility without preparation.
- `db::maintenance::backup` uses bounded `Backup::step` batches, cancellation,
  a named deadline and a private candidate. Validation, sync, no-clobber
  publication and owned cleanup stay in that backend owner.
- `DatabaseCommand` runs through the existing independent worker.
  `DatabaseVm` owns typed intent, availability, scope and recorded UTC reports.
  `DatabaseTools` retains the inputs and marshals results; `maintenance_forms`
  and the shared log frame own geometry and tokens. Settings Diagnostics and
  core recovery mount the same entity. No normal runtime/connection is required.
- Rust fixture support seeds current, older and newer schemas from the existing
  registry. Python owns the WAL/lock helper, source hashes and snapshot
  inspection, with no application DDL. The debug-only `database-backup`
  fixture command exercises the actual backend without opening a window.

Behavioral proof in `src/db/maintenance.rs`:

- `adr_0066_inspection_preserves_bytes_and_separates_schema_integrity_and_foreign_keys`
- `adr_0066_invalid_header_and_integrity_errors_never_become_verified_backups`
- `adr_0066_snapshot_includes_committed_wal_rows_with_source_open`
- `adr_0066_read_only_source_and_destination_aliases_are_safe`
- `adr_0066_busy_cancel_deadline_and_candidate_cleanup_are_bounded`
- `adr_0066_sql_progress_cancels_a_running_inspection`
- `adr_0066_late_cancel_io_failure_and_destination_race_never_publish_a_candidate`
- `adr_0066_backup_validation_checks_constraints_omitted_by_readonly_sqlite`

Command/VM proof in `src/view_models/startup/database.rs` covers independent
worker execution, the chosen source/destination, timestamps, cancellation,
unavailable execution and stale completions. `DatabasePreservationTests` in the
fixture tests reject missing snapshots and a stopped WAL helper and prevent
case reentry from reseeding or replacing preservation baselines.

Implementation details resolved in this packet: rusqlite's `hooks` feature is
also enabled to interrupt SQL checks at the cancellation/deadline boundary.
`Cargo.lock` is unchanged. The bundled SQLite omits CHECK constraints when it
loads schema read-only. Check reports that limit; private candidate validation
uses a write-capable, query-only connection after close/sync and includes CHECK
constraints. No source writes are needed. Existing/unknown schemas are never
migrated by these commands. OS file I/O still depends on the filesystem returning;
the named deadline bounds SQLite work and lock retries, not a stalled kernel.

## Acceptance Criteria

Mechanical; actual proof owners and behavioral tests are listed above.

| ID | Proof owner | Required assertion |
|---|---|---|
| C1 | db maintenance tests | Check reports access/integrity/schema separately and never initializes/migrates/repairs. Source contents and migration ledger do not change. |
| C2 | backup tests using WAL | Committed rows present only through WAL appear in the validated snapshot. A raw-main-file-copy implementation would fail this test. |
| C3 | filesystem/deadline tests | Existing/aliased destinations cannot be overwritten; Busy/Locked/cancel and I/O failure are bounded. No incomplete artifact is called a verified backup. |
| C4 | maintenance command/VM tests | Commands run without TopApp database or normal runtime; reports name source/destination, scope, actual time and next action. |
| C5 | situational guard adr_0066_database_checks_and_snapshots_have_one_owner | Inspection avoids open_db mutation; verified backups use SQLite's snapshot API. Cite invariant 6 and ADR 0016. |

Documentation proof: remove this packet's duplicate mechanism prose as its guards
land; record actual symbols and fixture/runbook anchors. Keep its Status,
the plan, ADR partial line, delivery row and pending-human-check index truthful.
An unwalked visual check cannot pass through a green mechanical suite.

## Test Commands

Repository verification commands:

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

## Verification — 2026-09-17

Green:

- `cargo fmt -- --check`, `cargo check --locked --offline --quiet` and
  `cargo clippy --locked --offline --quiet -- -D warnings`.
- Full unit run: 1,445 passed. The final focused `database_` run passed 9 unit
  tests, including the subsequently added fixture-schema test, and the new
  architecture guard. The complete unit inventory now contains 1,446 tests.
- All 256 architecture guards passed. The initial new guard compared formatted
  source literally; its corrected whitespace-normalized assertion passes.
- Eight database-maintenance behavioral tests passed; command/VM tests passed.
  Final review replaced a timing-sensitive one-microsecond test with a
  deterministic expired deadline and the production SQL progress handler.
- All 26 startup fixture tests passed, including two real-process lock
  regressions added during acceptance. Existing Rust local-socket tests needed
  the unrestricted runner after sandbox denial; assertions were unchanged.
- Documentation tests retain 10 existing ignored examples.
- Normal `v4vmm` desktop binary rebuilt after tests. No desktop app launch.
- Backend smoke in `/tmp/v4vmm-startup-xync7scx`: normal/WAL/read-only/recovery
  snapshots, source/WAL bytes, original configuration/library/music/ledger,
  private permissions and incomplete-artifact checks passed in both modes.
  Its owned helper stopped and the entire smoke fixture was removed. This is
  mechanical evidence only; it accepts none of operator V1–V3.
- Changed-document local file links and `git diff --check`: Green. No documents
  moved, no documentation folders created, and the canonical AGENTS.md remains
  at the repository root. The packet, runbook, review, source map and current
  indexes were updated in place.

Deviation: `hooks` is enabled alongside the planned `backup` feature to bound
SQL inspection. The SQLite read-only CHECK-constraint limit and candidate
validation correction are documented above and covered by a regression test.
No active database replacement, migration recipe, configuration format or
successor packet was implemented. Operator acceptance is recorded below.

## Operator Evidence — 2026-09-17

Accepted in `/tmp/v4vmm-startup-h1c__y5h`. The following UTC times come from the
operator's copied reports; no time is inferred for presentation or cleanup.
Database source/backup basenames below are under that fixture's `database/`
directory, except the configured normal library at `data/library.sqlite`.

| Check | Recorded UTC | Accepted result |
|---|---|---|
| V1 configured source | 18:29:00; 18:29:05 | Configuration lookup selects the existing normal library; read-only access, integrity, foreign keys and current schema pass without initialization, migration or repair. |
| V1 normal backup | 18:39:44 | Verified `normal-backup.sqlite`; candidate integrity includes CHECK constraints, foreign keys pass and schema is current. |
| V2 WAL source/backup | 18:44:35; 18:44:57 | Source check and verified `wal-backup.sqlite` pass while the WAL helper remains running. |
| V2 read-only source/backup | 18:46:30; 18:49:59 | Verified `readonly-backup.sqlite` and separate read-only source check pass. |
| V2 existing destinations | 18:48:35; 18:49:12 | `occupied.sqlite` and the completed WAL backup are refused; new-filename guidance replaces any completed-backup claim for these attempts. |
| V3 configured invalid header | 18:54:14; 18:55:38 | `invalid-header.sqlite` fails access; dependent checks are explicitly not checked; original-file retention guidance is clear. |
| V3 valid source/backup | 18:58:00; 18:58:59 | Recovery creates verified `recovery-backup.sqlite` from `readonly.sqlite`, and its separate source check passes without the normal app runtime. |
| V3 integrity damage | 18:59:59 | `integrity.sqlite` passes access and reports one integrity error separately; raw database text is omitted, foreign keys/schema pass and originals are retained. |
| V3 foreign keys | 19:00:59 | `foreign-key.sqlite` passes integrity and reports one foreign-key violation separately; it cannot be labelled a verified backup. |
| V3 newer schema | 19:02:02 | `newer.sqlite` passes integrity; migration 999 needs a compatible app and is not called corruption. |
| V3 older schema | 19:03:09 | `older.sqlite` passes integrity; its 10-of-11 ledger needs an explicit upgrade that Check does not apply. |
| V3 corrected lock check | 19:10:40 | `locked.sqlite` reports bounded Busy/Locked access, conflicting-writer guidance and skipped dependent checks. |
| V3 backup timeout | 19:12:58 | Snapshot copying to `locked-backup.sqlite` ends with bounded Busy/Locked failure and no completed backup. |
| V3 cancellation | 19:20:46 | The report explicitly confirms operator cancellation before completion and claims no completed backup. |

The operator accepted navigation to Music and back, retained paths/report,
normal/narrow Settings and recovery presentation, reachable fields/actions/report
without overlap, readable failures/next actions, full report copy and window
responsiveness during contention. Reports retain actual UTC, source/destination
paths and backup scope: committed WAL data is included; music/token files are
excluded; the app's library selection is not changed.

### Fixture Lock Correction

The initial locked-source check at 19:03:48 UTC unexpectedly passed and is not
lock-failure evidence. A separate-process reproduction identified a fixture bug:
checksum reads closed a raw descriptor after `BEGIN EXCLUSIVE`, releasing the
helper's POSIX lock. SQLite documents this
[descriptor-close behavior](https://www.sqlite.org/howtocorrupt.html#_posix_advisory_locks_canceled_by_a_separate_thread_doing_close_).
The fixture now completes checksum reads before taking the lock. Status and
preservation inspection probe an actual external reader instead of relying only
on helper liveness. The `database-lock` command restored the existing fixture's
lock without restarting WAL or replacing its baseline. Ordinary fixture cleanup
stops that additional helper.

`DatabaseLockTests` adds two ADR 0066 real-process regressions: checksum
completion retains the lock; restoring the old fixture's lost lock preserves
WAL bytes/baseline and cleanup stops both helpers. All 26 Python fixture tests
and the normal desktop build are Green. Application code did not change during
this correction. Additional reports at 19:14:44 and 19:16:33 UTC were lock
timeouts, not cancellation evidence; the 19:20:46 UTC result accepts cancellation.
The transient cancellation-request message is separate from copied completion
history. No application cancellation defect was established.

### Preservation And Cleanup

The normal `database-tools` inspection passed all 22 database flags then present:
three validated/private snapshots, committed WAL-row inclusion, original
source/WAL bytes and read-only permissions, and no incomplete candidates or
failed backup publication. Shared music, ledger 1–11, bindings and library
counts were preserved with no probes. Configuration changes were limited to
permitted normal workspace preferences; `config_preserved` was true.

The final `database-recovery` inspection passed all 28 database flags, including
the added external-reader lock probe and all four validated/private snapshots.
Source/WAL bytes and read-only permissions remained unchanged, and no incomplete
candidate or failed backup was published. Shared inspection confirmed unchanged
configuration bytes, preserved music, migration records 1–11, bindings, three
tracks, three playlist tracks and one playlist, with no music/database probes.

The operator then restored normal fixture configuration and confirmed:

```text
Fixture files are restored.
Removed fixture: /tmp/v4vmm-startup-h1c__y5h
Fixture removed
```

Cleanup stops both owned helpers before removing the fixture. Normal-mode
restoration and fixture removal are confirmed. Task 010's gate is closed;
task 004 and inherited checks remain separate. Task 011 had not started at this
acceptance; its later completion is recorded in its own packet.

## Operator Visual Check

Accepted: V1–V3, normal/narrow Settings and recovery presentation, navigation,
report copy, responsiveness, preservation in both cases, restoration and cleanup.
The [task 010 procedure](../runbooks/startup-recovery-check.md#task-010-database-check-and-backup)
remains a regression check with terminal commands, fixture requirements,
expected results and cleanup. No task 010 operator check remains open.

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

Schema compatibility cannot be classified from existing schema/ledger facts, backup needs an unbounded lock wait, or a destination might alias the source. Do not bypass validation or fall back to copying a live main file.
Routine placement inside the named owner is authorized. If a boundary needs a
new architectural decision, name the conflict and proposed bounded correction
before widening this packet.
