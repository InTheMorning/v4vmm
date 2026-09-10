# ADR 0066 Task 012: Database Restore

Status: Ready after the preceding packet - 2026-09-10.
Implementation not started. Operator check specified below; not runnable or accepted yet.

## Goal

Restore an explicitly chosen validated backup through the shared maintenance path, preserve the current database, and reopen only after verification.

## Read First And Dependency

Read [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md),
the [phase plan](../plans/adr-0066-startup-recovery-phase-plan.md), this whole packet, and the
[review checklist](../reviews/adr-0066-startup-recovery-review-checklist.md).
Execute after [task 011](adr-0066-task-011-database-maintenance-and-preservation.md) in the phase plan's order.
Complete this packet in one session; do not start its successor.
Names marked **new**, including tests and guards, are implementation targets,
not claims that those files or symbols already exist. If a predecessor already
created a listed owner, extend that owner.

## Files To Inspect

- src/db/maintenance.rs — inspection, snapshot, exclusive access and preservation
- src/application/commands/maintenance.rs; src/view_models/startup.rs
- Session drain and resumption packet — connection closure and generation invalidation
- src/presentation/maintenance_executor.rs; src/app/startup.rs; src/ui/composites/maintenance_forms.rs
- rusqlite::backup::Backup — source/destination and transaction requirements
- `tests/architecture_tests.rs`; `AGENTS.md`

## Files Likely To Change

- src/db/maintenance.rs
- src/application/commands/maintenance.rs; src/view_models/startup.rs
- src/presentation/maintenance_executor.rs; src/app/startup.rs; src/ui/composites/maintenance_forms.rs
- tests/architecture_tests.rs; docs/runbooks/startup-recovery-fixture.py; docs/runbooks/startup-recovery-check.md
- This packet's Status/evidence, the phase plan and review checklist.
- ADR 0066's guard references/partial line and the delivery/deferred indexes as
  appropriate; `docs/pending-human-checks.md` when a runnable visual gate opens.
  Update `AGENTS.md` when the next executable packet changes.

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

## Implementation Steps

1. Copy the chosen backup through SQLite into a separate owned candidate and validate integrity, foreign keys and schema compatibility. Do not modify the chosen backup. This packet accepts the current supported schema; older supported backups need a named migration preparation through the existing registry before installation, supplied by 013. Newer/unknown schemas are rejected with a clear reason.

2. Present an explicit Restore action naming both backup and configured destination, the database-only scope, and the preservation location. Capture the candidate fingerprint and active config/session generation. A changed source/destination requires a fresh review; choosing a file is not approval to replace data.

3. Drain the current session and acquire 011's exclusive maintenance access. Create and validate a preservation snapshot of a readable current database and retain its associated preservation report/files; a lockable damaged source uses the labelled file-preservation path. If preservation or exclusive access fails, stop before installation.

4. Install the validated candidate through SQLite's backup API into the maintenance-only destination connection, retaining exclusive locking. Keep the configured database inode/path in place; do not rename/unlink the database or journals. This avoids a release-lock/rename window and lets SQLite own its transaction/journal rollback. Do not wrap Backup destination work in an already-active transaction; keep exclusive locking across the necessary transaction boundaries.

5. Step installation with bounded Busy/Locked handling. On error/drop before completion, finish the SQLite backup handle and verify the destination rollback result; retain candidate and preservation paths in the report. Never claim full rollback without that verification. If the destination is too damaged for SQLite to install into, report the limitation and keep the files; no force-unlink fallback is authorized.

6. After installation, verify integrity/schema and task 002's core read/write usability before releasing maintenance access. Reopen one normal session through the shared resumption owner. Failed verification keeps recovery open with the preserved original available for an explicit restore; do not silently switch to another database.

7. Expose the same tool in Settings and core recovery, with typed progress/result, Copy report and applicable next action. Add fixture modes for invalid/newer candidate, destination contention, preservation failure, failed installation and successful restore.

## Acceptance Criteria

Mechanical; assert at the named owner. Test/guard names below marked new must
be implemented by this packet.

| ID | Proof owner | Required assertion |
|---|---|---|
| C1 | candidate tests | Invalid, newer, aliased or changed backups/destinations cannot enter install. Chosen backup bytes remain unchanged. |
| C2 | command-order tests | Explicit Restore, drain, close, exclusive access, preservation, candidate install and verification occur in that order. Failed prerequisites perform no install. |
| C3 | SQLite install failure tests | Inject failure between backup steps; verify rollback where SQLite guarantees it and preserve original/candidate/report regardless. No live database or journal pathname is swapped. |
| C4 | success/resumption tests | Restored rows and migration ledger match the validated candidate; normal work resumes only after fresh checks, with one connection/actor generation. |
| C5 | new situational guard adr_0066_restore_uses_validated_maintenance_install | Installation consumes validated candidate and exclusive maintenance capability; no renderer/ordinary command can bypass preservation or explicit intent. |

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

1. Restore a fixture backup with a clearly different playlist/row count. Confirm the action names the correct backup and destination before activation.

2. Inspect the preserved original, perform Restore, and confirm the new data after the app resumes without manual relaunch. Check that music/token files were not touched.

3. Try invalid/newer candidates and an induced installation failure. Recovery must retain useful paths and offer a next action; it must not display a new empty library as success. Clean up.

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

The installed SQLite backup API cannot preserve the required exclusive/rollback behavior, or restoring a damaged destination would require unlinking it. Stop that path with a tested actionable report; a filesystem swap needs a separately reviewed protocol.
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
- Restore an explicitly chosen validated backup through the shared maintenance path, preserve the current database, and reopen only after verification.

Constraints:
- Follow this packet's Constraints and Implementation Steps.
- Preserve its data, dependency and prose-retirement contracts.
- One packet this session. Never run the app.

Do not touch:
- Automatic restore, pathname swapping under live connections, or new empty database fallback
- Unknown/corrupt schema salvage or repair SQL (013 owns the recognized recipe)
- Music relocation, token-file restore or dropping unrelated database records to pass checks

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

