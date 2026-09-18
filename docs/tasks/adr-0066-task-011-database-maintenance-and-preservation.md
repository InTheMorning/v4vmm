# ADR 0066 Task 011: Database Maintenance And Preservation

Status: Complete - 2026-09-17; mechanical checks Green; operator gate closed.
V1–V3 behavior, manifest content, reopening, report retention, normal/narrow
presentation, both-fixture preservation and normal-mode restoration accepted.
Both fixture cleanups are confirmed. The subsequent
[task 012](adr-0066-task-012-database-restore.md) is also complete.

## Goal

Obtain exclusive database maintenance access after draining the app, and preserve original database files without claiming an unverified copy is a backup.

## Read First And Dependency

Read [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md),
the [phase plan](../plans/adr-0066-startup-recovery-phase-plan.md), this whole packet, and the
[review checklist](../reviews/adr-0066-startup-recovery-review-checklist.md).
Execute after [task 010](adr-0066-task-010-database-check-and-backup.md) in the phase plan's order.
Complete this packet in one session; do not start its successor.

## Files To Inspect

- Session drain and resumption packet — MaintenanceSession ownership
- src/db/maintenance.rs; src/db.rs; src/cli.rs — database open entry points
- src/library/app_impl.rs — paged actor's separate Connection
- src/application/commands/maintenance.rs; src/presentation/maintenance_executor.rs
- src/view_models/startup.rs; src/ui/composites/maintenance_forms.rs
- rusqlite backup/transaction behavior in the installed crate source
- `tests/architecture_tests.rs`; `AGENTS.md`

## Changed Owners

- `src/db/maintenance.rs` and new `src/db/maintenance/preservation.rs` — typed
  failures, exclusive connection, private copies and manifest.
- `src/application/commands/maintenance.rs`,
  `src/application/session_lifecycle.rs` — consumed drained-session authority,
  refused ordinary-command bypass, and empty cold-recovery drain.
- `src/view_models/startup/database.rs`, `src/presentation/database_tools.rs`,
  `src/app/startup.rs`, `src/ui/composites/maintenance_forms.rs` — shared typed
  actions, handoff, result report and fresh verification.
- `tests/architecture_tests.rs`, `docs/runbooks/startup-recovery-fixture.py`,
  `docs/runbooks/test_startup_recovery_fixture.py`, and the existing operator
  runbook — guards, real-process fixture, inspection and acceptance steps.
- This packet, review checklist, ADR/index, phase/delivery/deferred plans,
  pending-human index, `AGENTS.md` and the source map — current scope and gates.

## Do Not Touch

- File rename/unlink of the configured database, deletion of journals, or candidate installation
- Unsafe lock-file existence tests, process-list guessing, or forcibly killing external writers
- New migration/repair authority
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

The situational guard
`adr_0066_database_maintenance_requires_exclusive_access` in
[`tests/architecture_tests.rs`](../../tests/architecture_tests.rs) owns invariant
6's call-site, authority, connection-lifetime and no-replacement contract. It
replaces this packet's prospective mechanism instructions. Existing task 005
and 010 guards remain intact. No restore/install or new migration authority is
implemented.

### Mechanical Acceptance

| ID | Actual proof owner | Evidence |
|---|---|---|
| C1 | `adr_0066_real_readers_and_writers_bound_acquisition_without_copying` in the preservation module; task 005 drain tests | Separate bundled-SQLite processes hold read/write transactions in rollback and WAL modes; acquisition returns bounded Busy. A same-process second connection also blocks it. No copy destination is created while access is uncertain. |
| C2 | `adr_0066_exclusive_modes_exclude_processes_through_each_copy_and_release` | Separate processes attempt both reads and writes before copying, after every copied file, and after the manifest. They remain blocked until guard release, then regain access. WAL copies include committed data left in its journal by a process exit. |
| C3 | That mode test plus `adr_0066_preservation_failure_and_cancel_keep_partial_artifacts_and_originals` | Every copied source name/length/SHA-256 agrees with its manifest and bytes; private directory/file modes are checked. Destination failure and cancellation retain originals and report owned partial artifacts. Occupied destinations cannot be overwritten. |
| C4 | `adr_0066_lockable_damage_preserves_but_invalid_header_and_missing_sources_refuse`; VM tests `adr_0066_preservation_requires_drained_authority_and_keeps_report_through_resumption` and `adr_0066_preservation_cancel_and_invalid_source_keep_reports_and_reject_stale_results` | Lockable integrity damage is copied unchanged; invalid headers and missing files are refused. Real drain ownership, ordinary-command bypass rejection, recorded report retention, cancellation, generation rejection and fresh-session identity are tested. |
| C5 | `adr_0066_database_maintenance_requires_exclusive_access` | Only the drained-session command calls the backend acquisition entry. Copying requires its held guard. Empty recovery authority stays at the startup composition root. No file replacement, journal deletion, raw-copy fallback, creating open, or migration entry is available. |

Additional backend tests:
`adr_0066_sqlite_journal_recovery_is_observed_before_preservation` creates a real
hot rollback journal in a subprocess, confirms SQLite recovered it during
access, and verifies that the manifest/copy describe the recovered bytes.
`adr_0066_preservation_refuses_hardlink_aliases_and_nonregular_journals` rejects
unsupported aliases and a symlink journal without reading the unrelated target.

The fixture extends Rust's existing `database-seed` states; Python adds no schema.
`database-maintenance` opens Settings and `database-maintenance-recovery` starts
with an invalid configured header. The new writer holds `BEGIN IMMEDIATE` in a
separate process, and `maintenance-release` stops only that owned process.
`maintenance-inspect` verifies manifests and bytes without opening the copies
with SQLite. Python regressions cover actual writer contention/release, changed
copies/originals, empty manifests, refused-destination artifacts and immutable
fixture baselines.

### Verified Access Protocol And Task 012 Handoff

Verified against this lockfile's rusqlite 0.38.0 and bundled SQLite 3.51.1 on
Linux. `ExclusiveDatabase::acquire` opens an existing file without CREATE or URI
interpretation, sets `main.locking_mode=EXCLUSIVE` before database access, then
acquires `BEGIN EXCLUSIVE` within `EXCLUSIVE_ACCESS_DEADLINE` (five seconds).
It rolls back the empty transaction while retaining the exclusive connection.
The process tests above prove exclusion during this autocommit interval for both
rollback journals and WAL. Task 012 must retain this same guard when using
SQLite's backup API; that installation is not part of task 011.

SQLite's [locking-mode contract](https://www.sqlite.org/pragma.html#pragma_locking_mode)
and [exclusive WAL access](https://www.sqlite.org/wal.html#use_of_wal_without_shared_memory)
explain the protocol; the installed rusqlite transaction/backup source was also
reviewed. The manifest reports the observed mode: a newly opened connection
observes DELETE after a prior connection used TRUNCATE or PERSIST, whereas WAL
persists. Leftover journal files are still included when present.

The guard holds every raw source descriptor until its SQLite connection closes.
This field order is guarded and tested after each copied file, because closing a
raw descriptor early can release POSIX locks on the same inode. Do not insert a
source `fs::read`, independently opened checksum reader, or early file close
inside this interval. Output-file reads are safe and verify copied checksums.

Copying is bounded and cancellable between chunks. The command consumes one
`MaintenanceSession`, closes its access guard before returning that authority,
and retains partial artifacts on failure. The root starts fresh core checks;
only explicit Open app can mount a new normal session. Worker admission failure
returns the retained authority. The shared database report survives resumption.

SQLite itself can recover journals during access and checkpoint/clean up on
close. The manifest records UTC acquisition/copy times and file-metadata changes
observed across acquisition; it never asserts pre-access byte identity. A
preservation copy has not passed backup validation. Hard-link aliases,
non-regular/symlink journal files, unsupported modes and unreadable headers get
an actionable refusal, without a raw-copy bypass.

### Presentation And Verification Paths

- View model: `DatabaseVm`, including typed EndSession/Preserve/Cancel actions,
  accessibility labels, blocked availability and recorded reports.
- Shared owner: `maintenance_forms::database_tools`, composed unchanged by the
  existing Settings and recovery hosts. The input/help text, wrapped action row
  and `LogSource::Database` report use the existing shared controls/log frame.
- Tokens: existing `Spacing`, `FontSize`, `ControlStyle::Secondary`, semantic
  colors and input scaling. No new literal sizes, colors or glyphs.
- Mechanical results: **Green** — check, format, full tests, production Clippy,
  and normal desktop build. Full Rust suite: 1,455 unit tests and 257 architecture
  guards; ten existing documentation examples remain ignored. Python fixture
  suite: 31 tests. Test log: `/tmp/v4vmm-0066-011-tests-unrestricted.log`.
  The sandbox denied existing socket fixtures; the unrestricted rerun passed.
- Backend fixture smoke: both new cases passed setup, real writer contention,
  release, source/shared-state inspection, normal restoration and owned cleanup
  without launching the app. This checks fixture mechanics only. Local links in
  the changed documentation resolve, including their heading anchors.
- Human verification: [task 011 procedure](../runbooks/startup-recovery-check.md#task-011-database-maintenance-and-preservation),
  V1–V3 plus both inspections, normal restoration and cleanup are accepted.
  No agent launched the desktop app.

Scope choice: Settings explicitly ends the app session before a second,
separate preservation action. This reuses the accepted task 005 transition and
keeps the destination and report mounted. Backend implementation is a live child
module of the existing maintenance owner. No new ADR, schema or configuration
format was needed. Mechanical and operator acceptance are complete; this
session stops at task 011.

## Operator Evidence — 2026-09-17

The operator supplied the V1 result recorded at **20:39:30 UTC**, for source
`/tmp/v4vmm-startup-f_7zad8y/database/locked.sqlite` and destination
`/tmp/v4vmm-startup-f_7zad8y/database/busy-copy`. The report identifies exclusive
access as Busy, states that no preservation copy completed or database was
installed/replaced, and retains fresh verification before normal resumption.
The Busy-refusal report is accepted.

The next supplied report records cancellation at **20:43:45 UTC**, using the
same source and destination. Acquisition ended with operator cancellation, no
completed copy or database installation/replacement, and normal work still
stopped. The pasted report retains the earlier Busy attempt. The cancellation
report and retention of both report entries are accepted.

The operator then supplied successful fixture verification and writer-release
output: `writer_running` and `exclusive_access_blocked` are both false. At
**20:45:47 UTC**, the app reports one preserved database file in `locked-copy`,
with a manifest path, exclusive access recorded at that time and journal mode
`delete`. It explicitly distinguishes preservation from a verified restorable
backup, describes SQLite journal handling, releases access and leaves normal
work stopped. The full pasted report retains all three attempts.

The operator's accompanying **pass** accepts this release/copy report and the
requested retained-path and responsiveness checks from V1. No separate elapsed
duration was supplied.

The operator supplied the `locked-copy/manifest.json` contents: preservation-only
kind, exact source and copied names, a 278,528-byte file with SHA-256, acquisition
at `2026-09-17T20:45:47.208855679+00:00` and copy time at
`2026-09-17T20:45:47.222246953+00:00`, SQLite 3.51.1, journal mode `delete`, and
no observed changes during acquisition. The manifest content check is accepted;
checksum and permission verification passed in the later fixture inspection.

At **20:48:25 UTC**, the app checked `database/integrity.sqlite` and reported one
integrity error, no foreign-key violations and a compatible schema, with no
initialization/migration/repair. At **20:48:29 UTC**, preservation of that source
to `database/damaged-copy` completed with one file and a manifest. The report
retains the damage finding, makes no repair or verified-backup claim and leaves
normal work stopped. The lockable-damage report check is accepted.

At **20:50:27 UTC**, the app reports preservation of the configured
`data/library.sqlite` to `database/normal-copy`: one file, a manifest, exclusive
access in journal mode `delete`, no verified-backup claim and access released
before resumption. The operator **confirmed** fresh Check again followed by
explicit Open app, Music and Settings in the same window, the complete report
retained after reopening Database tools, and readable paths/reports with reachable
controls at normal and narrow widths. V1–V2 behavior and presentation are accepted.

The operator supplied the first fixture's `maintenance-inspect` output: all
**25 maintenance flags are Green**, including released writer, unchanged source
files, absent refused copies, correct manifests, exact copy inventories, private
permissions, lengths and checksums for locked/damaged/normal copies. Shared
inspection confirms configuration, music, migration records 1–11, bindings and
library contents preserved, with no database or music probes. Configuration
bytes differ only through accepted normal workspace preferences. First-fixture
preservation is accepted.

For recovery fixture `/tmp/v4vmm-startup-ar_glv2s`, the app selected the configured
`database/invalid-header.sqlite` at **20:56:46 UTC**. At **20:56:51 UTC**, attempted
preservation to `database/invalid-copy` refused invalid database contents during
exclusive-access acquisition. The report states no completed copy or database
installation/replacement, directs the operator to retain originals and inspect a
known backup, and leaves normal work stopped. The operator's **pass** accepts
this refusal, unavailable Open app and normal/narrow recovery presentation.
The operator's subsequent fixture-inspection pass confirms absence of the
refused copy.

At **20:59:04 UTC**, the recovery window reports preservation of `locked.sqlite`
to `locked-copy`; at **20:59:39 UTC**, it reports preservation of `integrity.sqlite`
to `damaged-copy`. Each reports one file and a manifest, exclusive access in
journal mode `delete`, the SQLite journal-handling explanation, and no verified
backup or repair claim. Both release access and leave normal work stopped. The
earlier invalid-header refusal remains in the supplied report. V3 copy reporting
is accepted.

The operator then reported **pass** for recovery-fixture `maintenance-inspect`,
restoration with `mode normal`, and the following shared `inspect`. Recovery
preservation and normal-mode restoration are accepted from that confirmation;
raw inspection output was not supplied.

The first fixture's final shared inspection reports `case: normal`, unchanged
configuration bytes, preserved music, migration records 1–11, bindings, library
contents and tool blockers, with no probes. Its normal-mode restoration is
accepted. `normal_workspace_preferences_only: false` is expected when the
configuration bytes are unchanged. Artifact
evidence comes from the operator; the desktop fixtures were not accessible from
the agent environment.

### Final Acceptance And Cleanup

The operator **confirmed** both cleanup commands removed
`/tmp/v4vmm-startup-f_7zad8y` and `/tmp/v4vmm-startup-ar_glv2s`, after both apps
closed and preservation/restoration passed. Task 011 is complete on 2026-09-17:
mechanical checks Green; V1–V3, Settings/recovery presentation, report retention,
both preservation inspections, normal-mode restoration and cleanup accepted.
No task 011 human check remains open. Task 004 and inherited checks are separate;
ADR 0066 remains Accepted and partial. Task 012 requires a fresh session.

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

Follow [task 011 in the startup recovery runbook](../runbooks/startup-recovery-check.md#task-011-database-maintenance-and-preservation).
It supplies exact commands, desktop requirements, expected/wrong results and
cleanup. The operator gate is closed; this procedure remains a regression check.

1. V1: enter Settings, drain the normal app, attempt preservation against the
   owned external writer, check responsiveness and cancel a second attempt.
2. V2: release the writer, preserve healthy and lockable damaged sources, inspect
   manifests, preserve the configured library, then explicitly reopen after
   fresh checks. Confirm report retention and normal/narrow presentation.
3. V3: use cold recovery with an invalid header; confirm actionable refusal and
   unchanged originals. Preserve the other sources through those same tools.
4. Close the app, pass both fixture inspections, restore normal mode and remove
   both fixtures. Record actual operator evidence before closing the gate.

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

SQLite cannot establish the promised exclusive protection in a supported journal mode, or a preservation path would require filesystem replacement beneath unknown handles. Report the tested mode and API result; do not improvise OS locks or a raw-copy fallback.
Routine placement inside the named owner is authorized. If a boundary needs a
new architectural decision, name the conflict and proposed bounded correction
before widening this packet.
