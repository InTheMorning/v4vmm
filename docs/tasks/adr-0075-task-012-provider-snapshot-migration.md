# ADR 0075 Task 012: Provider Snapshot Migration

Status: Implementation, technical review, and mechanical checks complete - 2026-09-20. The presentation gate remains open and paused.

Document preparation needs no visual check. The future repair-report and readiness check remains open and paused.

Technical review confirmed registry ownership, separate version targets, preservation before DDL, rollback verification, and receipt propagation through live callers.

## Goal And Scope

Add migration 12, `provider_metadata_snapshots`, through the existing migration registry.
Preserve existing records, interrupted migration-11 repair, and verified database restore.
Require a durable backup before upgrading an existing database through startup or the CLI.

The reviewed [storage schema](../schema/adr-0075-provider-snapshot-storage.md) owns the tables, constraints, and preservation contract.
This packet implements its migration section and cases S11-01 through S11-05.
It adds no field policy or provider observation writes.

## Read First

- [AGENTS.md](../../AGENTS.md) and [source map](../../.github/copilot-instructions.md).
- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md), [phase plan](../plans/adr-0075-metadata-contract-phase-plan.md), and [packet 011](archive/adr-0075-task-011-provider-snapshot-schema.md).
- [Storage schema](../schema/adr-0075-provider-snapshot-storage.md), especially migration, preservation, and implementation owners.
- [ADR 0016](../adr/archive/0016-schema-migration-discipline.md) and [ADR 0066 repair packet](archive/adr-0066-task-013-interrupted-upgrade-repair.md).
- Shared skills: `/home/citizen/.agents/skills/asd-ste100/SKILL.md`, `/home/citizen/.agents/skills/repo-docs-organizer/SKILL.md`, and `/home/citizen/.agents/skills/feature-orchestrator/SKILL.md`.
- Rust skill: `/home/citizen/.agents/skills/rust-skills/rust-dev/SKILL.md`.

## Inspected Owners And Permitted Changes

| Files | Required work |
| --- | --- |
| [src/db.rs](../../src/db.rs), [src/db/upgrades.rs](../../src/db/upgrades.rs) | Extend the registry and current-schema inspection. Bound migration execution by target version. Separate version-11 recognition from the latest schema. |
| `src/db/provider_snapshot_schema.rs` | New private module containing migration-12 DDL. The registry must call its apply function. |
| [src/db/startup.rs](../../src/db/startup.rs), [src/db/maintenance.rs](../../src/db/maintenance.rs), [preservation.rs](../../src/db/maintenance/preservation.rs) | Route preparation through guarded preservation and the existing verified SQLite snapshot builder. |
| `src/db/maintenance/upgrade.rs` | New private owner for guarded existing-database preparation and its typed result. Startup and CLI preparation must reach it. |
| [restore.rs](../../src/db/maintenance/restore.rs) | Carry the exact candidate target through validation, installation, and verification. Preserve rollback checks. |
| [src/startup.rs](../../src/startup.rs), [src/cli.rs](../../src/cli.rs), [bootstrap.rs](../../src/app/bootstrap.rs) | Connect guarded preparation to `StartupBackend::prepare`, `open_configured_db`, and `PreparedCore` consumption. Preserve preparation receipts. |
| [maintenance commands](../../src/application/commands/maintenance.rs), [database view model](../../src/view_models/startup/database.rs), [startup view model](../../src/view_models/startup.rs), [startup callbacks](../../src/app/startup.rs) | Carry the verified target and preparation result. Report paths and actual times. Admit a normal session only after current-schema preparation. |
| [startup fixture](../../src/startup/fixture.rs), [fixture script](../runbooks/startup-recovery-fixture.py), [fixture tests](../runbooks/test_startup_recovery_fixture.py) | Preserve supported interruption fixtures and their inspections after the latest version becomes 12. |
| `src/db/fixtures/adr-0075-schema-11.sql`, `src/db/fixtures/adr-0075-interrupted-11.sql` | New frozen test inputs from the version-11 implementation. Never use them to initialize an application database. |
| [architecture tests](../../tests/architecture_tests.rs) and unit tests beside these owners | Preserve ADR 0066 guards. Add migration-specific behavioral tests and an ADR 0075 ownership guard. |
| [Show test helper](../../src/app/show.rs), [broadcast registry test helper](../../src/broadcast/registry.rs) | Orchestrator-approved exceptions: extract the connection from `PreparedDatabase` in the two existing test helpers. Production behavior stays unchanged. |

Changes to other files require orchestrator review.
Do not create observation APIs, unused storage abstractions, renderer layouts, configuration formats, or another migration ledger writer.
Do not change RSS parsing, API transport, payment, playback, or broadcast behavior.
Do not modify upstream repositories, production databases, or shared status documents.

## Implementation Steps

### 1. Freeze Version-11 Evidence

1. Capture synthetic complete and interrupted version-11 databases before changing the registry.
2. Create the interruption through the existing `AfterApply` migration-11 failure boundary.
3. Export the two SQL test inputs with their source revision and fixture provenance recorded in comments.
4. Include representative records from every existing table, with stable row identities and an event-selection revision above zero.
5. Keep these inputs fixed after adding migration 12.

The frozen files provide independent regression evidence. The Rust registry remains the only executable migration authority.
Do not construct an older database by deleting ledger records or removing newer tables.

### 2. Separate Migration Targets

1. Add an explicit target to the normal registry executor and its existing failure seam.
2. Keep `record_migration` as the sole ledger writer.
3. Move selection-table creation out of `init_schema` into its existing migration-11 entry.
4. Build version-10 interruption fixtures and version-11 recognition expectations through that registry.
5. Keep migration-12 tables out of `init_schema`.
6. Update `recognize_migration_11` to use version-11 columns and schema objects, independent of `CURRENT_COLUMNS` for version 12.
7. Preserve its exact object checks, allowed historical enclosure-column order, ledger-prefix checks, and event-selection validation.
8. Reject partial migration-12 objects without ledger entry 12 as unsupported schema.

The current initializer contains earlier migration objects. Do not claim that the bounded builder reproduces every historical schema version.
Versions 10, 11, and 12 are the required construction targets here.

### 3. Guard Existing-Database Preparation

Follow the schema's [backup sequence](../schema/adr-0075-provider-snapshot-storage.md#backup-before-existing-database-upgrade).

1. Close compatibility probes before acquiring `ExclusiveDatabase`.
2. Recheck compatibility under exclusive access.
3. Preserve database and journal files in a new, private directory through the existing preservation owner.
4. Build a verified SQLite snapshot from the guarded connection, including committed WAL records.
5. Sync the snapshot and preservation directory before migration DDL.
6. Hold exclusive access through migration and verification.
7. Reopen and check the final database before returning a usable connection.

Use `build_snapshot` with the guarded connection. Do not call `backup` through another source connection while the guard is held.
Preserve the descriptor drop order in `ExclusiveDatabase`. Closing another descriptor for the same database can release its process lock.
Retain the existing bounded waits, cancellation checks, regular-file checks, and filesystem identity checks.

Both `db::open_db` and `db::startup::prepare_database` must reach this owner without recursion or an unguarded production path.
Fresh empty databases need no original backup. Current databases need no repeated migration or backup.
Supported older databases receive preservation before any remaining migration runs.
Interrupted migration 11 still requires the existing explicit repair operation.

Return a typed preparation receipt with the target, artifact paths, recorded times, and verification result.
On failure, retain completed artifact paths and the failed operation in the typed error.
Carry these facts through `PreparedCore` and the existing startup report owner.
The CLI must report the receipt on stderr and preserve its JSON stdout contract.
Do not discard receipts at a live caller or invent report times during rendering.

### 4. Apply Migration 12 Atomically

1. Add the exact tables and constraints from the reviewed schema.
2. Insert only the `metadata_generation` singleton with generation zero.
3. Leave all other new tables empty. Do not infer providers or owners from legacy records.
4. Capture the unchanged-record baseline after version 11 and before migration 12.
5. Apply migration-12 DDL and its ledger entry in one SQLite transaction.
6. Keep `BeforeApply`, `AfterApply`, and `AfterRecord` failure boundaries inside that transaction before commit.
7. Verify the target schema, integrity, foreign keys, and unchanged legacy records before commit.
8. Run the final readiness check after commit and reopen.

Use an explicit version-11 table manifest for the legacy record digest.
Preserve each row identity, stored value, and old ledger record. Compare the expected schema change separately.
Do not omit all metadata tables or all ledger records to force a preservation comparison to pass.
Earlier migrations retain their existing transformations.

The startup write probe creates its own transaction. Do not nest that probe inside the migration transaction.
After a precommit failure, verify rollback against the old schema, ledger, and legacy record digest.
A postcommit verification failure must retain recovery and artifact paths. It must not claim that rollback occurred.

### 5. Preserve Repair And Restore Compatibility

1. Give `ValidatedRestore` and its result an explicit verified schema target.
2. Make `migrate_candidate` accept that target through the same migration registry.
3. Repair interrupted migration 11 only through target 11.
4. Preserve the existing repair digest, which excludes only the missing migration-11 ledger record.
5. Make `verify_installed` validate the requested target without treating version 11 as current readiness.
6. Run separately backed-up preparation to 12 before admitting a normal session.
7. Make explicit older-backup preparation produce a private target-12 candidate without changing the selected backup.
8. Preserve restore rollback checks and complete version-12 data during current-backup restore.

The current repair report promises a fresh session after any verified installation.
The callback in `src/app/startup.rs` also treats every `InstallState::Verified` as sufficient for resumption.
Update both consumers to preserve the distinction between repaired version 11 and ready version 12.
The existing resumption flow can perform fresh preparation. It must retain the repair report and the separate migration-12 receipt.
Do not admit a normal session directly from version-11 verification.

### 6. Preserve Fixture And Guard Meaning

1. Construct the debug upgrade fixture through version 10 before seeding its interruption scenario.
2. Replace `interrupt_fixture` setup that removes the selection table and ledger entry.
3. Update fixture inspection to recognize repaired version 11 and prepared version 12 separately.
4. Verify unchanged legacy rows, expected new tables, backup bytes, and files outside the database.
5. Update existing ADR 0066 guards for the bounded registry call without weakening their authority or preservation requirements.

Keep fixture setup behind the existing verified temporary-root boundary.
Use frozen inputs for old-schema regression tests and registry-generated inputs for failure-boundary tests.
Do not add a shared test helper module or another integration test file.

## Mechanical Acceptance Criteria

Each case needs a behavioral test beside its owning code. Use the `adr_0075_migration_` test prefix.

| Case | Required proof |
| --- | --- |
| M12-01 | S11-01 passes for populated frozen version-11 input. All old records remain equal. Only the new schema, ledger entry, and generation singleton appear. |
| M12-02 | S11-02 passes for frozen and registry-generated interruptions. Unknown objects, invalid selection state, and partial migration 12 remain rejected without writes. |
| M12-03 | S11-03 proves target-11 repair, retained repair evidence, separate target-12 preservation, and readiness only after target 12. |
| M12-04 | S11-04 covers all three failure boundaries and reopening after an interrupted transaction. Neither partial tables nor ledger entry 12 survive. |
| M12-05 | S11-05 covers preservation, snapshot, sync, and cancellation failures before DDL. DELETE and WAL cases preserve all committed records. |
| M12-06 | Both real preparation entry points use the guarded owner. A competing writer cannot enter between backup and commit. |
| M12-07 | Fresh creation reaches version 12. Repeated preparation of a current database creates no backup, duplicate ledger row, or metadata record. |
| M12-08 | Current-backup restore preserves seeded version-12 bodies, selected absence, and discrepancy history. Older-backup preparation leaves the selected backup unchanged. |
| M12-09 | Precommit failure reports verified rollback only after comparison. Postcommit verification failure retains recovery and reports no rollback claim. |
| M12-10 | Typed startup and CLI results retain backup paths and actual times. Reports and resumption distinguish repaired version 11 from ready version 12. |
| M12-11 | Fixture inspection preserves playlist order, source claims, local paths, event-selection revision, configuration, audio files, and secret-file bytes. |
| M12-12 | DDL tests enforce the reviewed keys and constraints, including generic selected-field keys and separate discrepancy resource pairs. |

Use direct SQL fixture rows for M12-08 and M12-12. Do not implement future observation writers to seed these tests.
An interrupted-transaction test may use a temporary test process. It must not launch the desktop app.

## Test Commands

Run focused tests during implementation. Run the full suite once after integration.

```bash
cargo test --locked --offline adr_0075_migration_
cargo test --locked --offline adr_0066_upgrade_
cargo test --locked --offline adr_0066_restore_
python3 -m unittest discover -s docs/runbooks -p test_startup_recovery_fixture.py
cargo test --locked --offline
cargo check --locked --offline
cargo clippy --locked --offline -- -D warnings
cargo fmt -- --check
cargo build --locked --offline --bin v4vmm
```

The full suite includes `tests/architecture_tests.rs`. Rebuild the normal binary after tests before any operator handoff.
Do not run the app or change a production database.

## Rollback And Dependencies

Before commit, use the verified transaction rollback described above.
After a completed migration, use the verified original backup with a compatible version-11 binary and the existing reviewed restore procedure.
Preserve the version-12 database before that restore. A code revert alone does not downgrade a database.
Never reverse migration 12 by deleting its ledger record or dropping its tables.

The current binary's older-backup preparation upgrades to 12. It is not a downgrade command.

Packet 013 owns atomic provider replacement and complete-empty collections.
Packet 014 owns durable raw observations and reachable evidence writes.
The operator deleted packet 015 on 2026-09-21. No packet owns the combined isolation, restart, rollback, and superseded-response tests.
The [plan](../plans/adr-0075-metadata-contract-phase-plan.md#unassigned-work) lists them as unassigned work.
Passing this packet does not complete schema cases S11-06 through S11-23.

## Escalation Triggers

Return to the orchestrator if the inspected registry has already advanced beyond version 11.
Also return if the work requires a different preservation engine, another configuration format, broader presentation changes, or new field policy.
Do not change an accepted constraint to make a test pass.

## Implementation Report - 2026-09-20

Migration 12 creates the fifteen reviewed metadata tables through the existing registry.
Only the generation singleton receives a row. Its generation is zero.
The migration preserves all nineteen legacy tables, including row identities and old migration records.
No field policy, observation writer, or display selection was added.

Startup and CLI preparation share the guarded preparation owner.
Existing databases receive file preservation and a verified SQLite snapshot before migration.
The guard excludes competing writers through verification. Snapshot tests include committed WAL records.
Typed receipts retain artifact paths, actual recorded times, and the preparation result.

Startup reports retain these receipts through repeated checks, session drain, and resumption without duplicate history.
The CLI writes receipts to stderr and keeps command JSON on stdout.

Migration-12 DDL and its ledger entry commit in one transaction.
All three failure boundaries run before commit. Tests also terminate a disposable process during that transaction.
Rollback reports require an equal schema, ledger, record digest, and database identity.
Postcommit verification failures retain recovery evidence without a rollback claim.

Interrupted migration 11 uses explicit repair to target 11.
Normal admission requires separate preserved preparation to target 12.
Restore review verifies the exact completed private candidate before installation.
An extra current-schema column fails review while source and destination bytes remain unchanged.
Current restore preserves binary bodies, selected absence, and discrepancy history.

### Files And Scope

New source files are `src/db/provider_snapshot_schema.rs` and `src/db/maintenance/upgrade.rs`.
The two frozen SQL inputs are new files in `src/db/fixtures/`.
All other source, test, and fixture changes use the permitted owners listed above.
The two approved test-helper exceptions only extract `PreparedDatabase.connection`.
This packet is the only documentation file changed by the implementation agent.
The orchestrator owns shared status and policy documents.

No documentation file moved. No new documentation folder or root document was created.

The frozen SQL inputs were captured before the registry changed.
Their source revision is `a12521e7510b3d05cd4fc097a370f1f965145aad`.
Capture used synthetic databases and the existing migration-11 `AfterApply` boundary.
Each input contains records from all legacy tables and event-selection revision 7.

The frozen files remain independent test evidence. Production initialization uses only the registry.
Older fixture construction no longer removes ledger records or tables from a current database.

### Mechanical Evidence

| Criteria | Behavioral evidence |
| --- | --- |
| M12-01 | Frozen version-11 records and identities remain equal. Only the expected schema, ledger row, and generation singleton appear. |
| M12-02 | Frozen and generated interruptions remain recognized. Unknown objects, invalid selection state, and partial migration 12 remain rejected. |
| M12-03 | Repair verifies target 11. A separate preserved preparation verifies target 12 before normal admission. |
| M12-04 | Each registry boundary rolls back. Process termination leaves no partial migration after reopening. |
| M12-05 | Preservation, snapshot, sync, and cancellation failures stop before DDL. DELETE and WAL records remain equal. |
| M12-06 | Both preparation entry points preserve records. A separate process cannot write while the migration guard holds access. |
| M12-07 | New databases reach 12. Repeated preparation creates no backup, duplicate ledger row, or metadata record. |
| M12-08 | Restore preserves seeded metadata evidence. Older-backup preparation leaves the selected backup unchanged. Exact candidate mismatch fails before installation. |
| M12-09 | Precommit reports distinguish verified rollback from postcommit verification failure. Both retain completed artifact paths. |
| M12-10 | Typed receipts retain paths and actual times. Three drain/resume cycles preserve one initial receipt. |
| M12-11 | Fixture checks preserve playlist sequence, source claims, local paths, selection revision, configuration, audio, and secret bytes. |
| M12-12 | Direct SQL tests enforce columns, keys, JSON checks, enums, ranges, and restricted deletion. |

| Command | Result |
| --- | --- |
| `cargo test --locked --offline adr_0075_migration_` | Green: 22 unit tests and one architecture guard. |
| `cargo test --locked --offline adr_0066_upgrade_` | Green: 10 unit tests and one architecture guard. |
| `cargo test --locked --offline adr_0066_restore_` | Green: 11 unit tests and one architecture guard. |
| `python3 -B -m unittest discover -s docs/runbooks -p test_startup_recovery_fixture.py` | Green: 42 tests. |
| `cargo test --locked --offline --quiet` | Green: 1,593 unit tests and 265 architecture tests. Ten documentation tests remain ignored. |
| `cargo check --locked --offline` | Green. |
| `cargo clippy --locked --offline -- -D warnings` | Green. |
| `cargo fmt -- --check` | Green. |
| `cargo build --locked --offline --bin v4vmm` | Green after tests. |
| Isolated CLI `playlists list --json` | Green: JSON stdout, receipt stderr, preserved version-11 snapshot, target 12, and no repeated backup. |
| Markdown link checker | Green: 30 local links. |
| Shared STE checker | Paragraph defects corrected. Dictionary findings remain for technical terms and inherited prose. |

The initial sandboxed suite could not bind local fixture sockets.
The complete unrestricted suite passed after the fixture corrections and both review corrections.
No desktop app, production database, or external service was used.
Final technical review passed after both corrections. No required code fix remains in this packet.
The presentation gate remains open and paused.

Successful final checks returned terminal output. The earlier `/tmp/adr0075-task012-full-tests.log` records an interrupted run.

## Operator Visual Check

This future check remains open and paused. Document preparation does not require it.
Do not request or run a visual batch now.

After mechanical review, the operator needs a desktop session and a temporary recovery fixture.
No audio hardware, publisher, encoder, or external service is required.

1. Create the supported interruption in a temporary fixture from the normal binary.

   ```bash
   cd /home/citizen/build/v4vmm
   cargo build --locked --offline --quiet --bin v4vmm
   ADR0075_MIGRATION_FIXTURE=$(python3 docs/runbooks/startup-recovery-fixture.py setup)
   python3 docs/runbooks/startup-recovery-fixture.py mode "$ADR0075_MIGRATION_FIXTURE" upgrade-interrupted
   python3 docs/runbooks/startup-recovery-fixture.py upgrade-interrupt "$ADR0075_MIGRATION_FIXTURE"
   python3 docs/runbooks/startup-recovery-fixture.py upgrade-status "$ADR0075_MIGRATION_FIXTURE"
   python3 docs/runbooks/startup-recovery-fixture.py run "$ADR0075_MIGRATION_FIXTURE"
   ```

2. Open Database tools. Select the configured database and check it.
3. Enter the printed `upgrade/failed-preservation` destination. Repair the interrupted upgrade.
4. Confirm that the injected failure retains recovery and reports verified rollback with the preserved artifact paths.
5. Remove only the fixture interruption from a second terminal with the same fixture variable.

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py upgrade-ready "$ADR0075_MIGRATION_FIXTURE"
   ```

6. Check the database again. Enter `upgrade/repaired-preservation` and repair the interrupted upgrade.
7. Confirm that the repair report names version 11, preserved records, backup paths, and recorded UTC times.
8. Confirm that current readiness follows the separately backed-up migration to 12, with both reports retained.
9. If recovery requires **Open app**, use that action to complete preparation.
10. Copy the reports. Confirm complete paths and the distinction between repair and current readiness.
11. Inspect the changed report and readiness text at normal and narrow widths in Light and Dark.
12. Treat clipped text, lost reports, false rollback claims, or early readiness as failures.
13. Review the printed `upgrade/older-backup.sqlite` through Database tools. Use `upgrade/backup-restore-preservation` for preservation.
14. Choose **Upgrade backup**. Confirm a separate version-12 candidate and leave **Restore database** unclicked.
15. Close the app. Verify preservation before fixture cleanup.

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py upgrade-inspect "$ADR0075_MIGRATION_FIXTURE"
   ```

16. Keep a failing fixture for diagnosis. Clean up only after every preservation flag passes.

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py cleanup "$ADR0075_MIGRATION_FIXTURE"
   ```

The orchestrator owns shared gate records. Mechanical results cannot close this presentation gate.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:

- This packet, its Read First inputs, and each inspected owner before editing it.

Goal:

- Add migration 12 with guarded preservation, atomic application, and version-aware repair and restore.

Constraints:

- Implement the six steps and all mechanical acceptance cases.
- Keep the migration registry authoritative and the future presentation gate open.
- Preserve every existing record and receipt. Leave new observation tables empty except for the generation singleton.

Do not touch:

- Production data, upstream repositories, configuration formats, field policies, renderers, or shared status documents.
- Later packets' storage writers, provider requests, comparison logic, or display selection.

Acceptance criteria:

- M12-01 through M12-12 pass. Report the paused operator check separately.

Test commands:

- Run the commands in Test Commands. Check changed documentation with the shared STE and Markdown link checkers.

At the end, report:

1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
