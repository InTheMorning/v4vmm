# ADR 0066 Task 010: Database Check And Backup

Status: Ready after the preceding packet - 2026-09-10.
Implementation not started. Operator check specified below; not runnable or accepted yet.

## Goal

Offer database inspection and a verified SQLite backup from both Settings and core recovery, without requiring the normal app runtime.

## Read First And Dependency

Read [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md),
the [phase plan](../plans/adr-0066-startup-recovery-phase-plan.md), this whole packet, and the
[review checklist](../reviews/adr-0066-startup-recovery-review-checklist.md).
Execute after [task 009](adr-0066-task-009-conversion-retry-and-retained-input.md) in the phase plan's order.
Complete this packet in one session; do not start its successor.
Names marked **new**, including tests and guards, are implementation targets,
not claims that those files or symbols already exist. If a predecessor already
created a listed owner, extend that owner.

## Files To Inspect

- src/db.rs — open_db, init_schema, MIGRATIONS, migrate_schema, schema_migrations
- Cargo.toml; Cargo.lock — rusqlite 0.38, currently bundled only
- src/application/commands/maintenance.rs; src/view_models/startup.rs
- src/presentation/maintenance_executor.rs; src/ui/composites/maintenance_forms.rs
- docs/adr/0016-schema-migration-discipline.md
- `tests/architecture_tests.rs`; `AGENTS.md`

## Files Likely To Change

- src/db/maintenance.rs (new); src/db.rs — module and schema-inspection helpers only
- Cargo.toml; Cargo.lock — rusqlite backup feature only if lockfile changes
- src/application/commands/maintenance.rs; src/view_models/startup.rs
- src/ui/composites/maintenance_forms.rs; src/app/startup.rs; src/app.rs — shared tools route only
- tests/architecture_tests.rs; docs/runbooks/startup-recovery-fixture.py; docs/runbooks/startup-recovery-check.md
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

## Implementation Steps

1. Extract database maintenance behind db::maintenance, retaining schema facts/registry in db.rs. Check opens an existing path with explicit non-creating flags; do not call open_db. Report access, PRAGMA integrity_check result, foreign-key violations and schema/ledger compatibility separately. An older supported schema needs an upgrade; a newer/unknown schema must not be called corrupt merely for being unfamiliar.

2. Enable rusqlite's backup feature. Back up with backup::Backup into an exclusively reserved new destination, stepping a bounded page batch and checking a named deadline/cancellation between steps. Do not use an unbounded run_to_completion loop. Snapshot source is SQLite's consistent view, including committed WAL data; a raw main-file copy is not a backup.

3. Reject destination aliases of the source, existing files and any source/journal path. Use restrictive permissions, finish/close the candidate, sync and validate integrity/schema before publishing its completed name without clobbering. A failed/cancelled backup is not labelled complete; clean only owned incomplete artifacts and report cleanup failures.

4. Keep backup usable while the normal app is open; it does not require the exclusive replacement lifecycle. Run it through the independent maintenance worker with no normal RuntimeHost or main-app connection prerequisite. Bounded Busy/Locked results name the database and suggest closing the conflicting writer and checking again.

5. Add Check database and Back up database to the same forms in Settings/recovery. State plainly that the backup covers database records, not music files or token files. Report the actual chosen path and recorded UTC result. Preservation copies for an unopenable database belong to 011, not a fallback that mislabels copied bytes as a valid backup.

6. Extend the fixture with committed WAL-only data, a read-only source, locked source, invalid header, integrity error and newer schema. Seed schema states through Rust/db test support; the Python fixture must not become a second copy of current application schema.

## Acceptance Criteria

Mechanical; assert at the named owner. Test/guard names below marked new must
be implemented by this packet.

| ID | Proof owner | Required assertion |
|---|---|---|
| C1 | db maintenance tests | Check reports access/integrity/schema separately and never initializes/migrates/repairs. Source contents and migration ledger do not change. |
| C2 | backup tests using WAL | Committed rows present only through WAL appear in the validated snapshot. A raw-main-file-copy implementation would fail this test. |
| C3 | filesystem/deadline tests | Existing/aliased destinations cannot be overwritten; Busy/Locked/cancel and I/O failure are bounded. No incomplete artifact is called a verified backup. |
| C4 | maintenance command/VM tests | Commands run without TopApp database or normal runtime; reports name source/destination, scope, actual time and next action. |
| C5 | new situational guard adr_0066_database_checks_and_snapshots_have_one_owner | Inspection avoids open_db mutation; verified backups use SQLite's snapshot API. Cite invariant 6 and ADR 0016. |

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

1. From Settings, check the disposable database and back it up to a new fixture path. Confirm the report names the destination and excludes music/token files.

2. Use fixture inspect to prove the snapshot contains its committed WAL-only row. Attempt an existing destination; it must be refused without changing that file.

3. Open database-failure recovery and use the same tools. Check must explain which test failed and offer the applicable next action. Clean up the fixture only.

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

Schema compatibility cannot be classified from existing schema/ledger facts, backup needs an unbounded lock wait, or a destination might alias the source. Do not bypass validation or fall back to copying a live main file.
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
- Offer database inspection and a verified SQLite backup from both Settings and core recovery, without requiring the normal app runtime.

Constraints:
- Follow this packet's Constraints and Implementation Steps.
- Preserve its data, dependency and prose-retirement contracts.
- One packet this session. Never run the app.

Do not touch:
- Active database replacement, migration recipes, arbitrary corruption salvage
- A second schema registry or new durable maintenance tables
- Backup of music or broadcaster token files

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

