# ADR 0066 Task 011: Database Maintenance And Preservation

Status: Ready after the preceding packet - 2026-09-10.
Implementation not started. Operator check specified below; not runnable or accepted yet.

## Goal

Obtain exclusive database maintenance access after draining the app, and preserve original database files without claiming an unverified copy is a backup.

## Read First And Dependency

Read [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md),
the [phase plan](../plans/adr-0066-startup-recovery-phase-plan.md), this whole packet, and the
[review checklist](../reviews/adr-0066-startup-recovery-review-checklist.md).
Execute after [task 010](adr-0066-task-010-database-check-and-backup.md) in the phase plan's order.
Complete this packet in one session; do not start its successor.
Names marked **new**, including tests and guards, are implementation targets,
not claims that those files or symbols already exist. If a predecessor already
created a listed owner, extend that owner.

## Files To Inspect

- Session drain and resumption packet — MaintenanceSession ownership
- src/db/maintenance.rs; src/db.rs; src/cli.rs — database open entry points
- src/library/app_impl.rs — paged actor's separate Connection
- src/application/commands/maintenance.rs; src/presentation/maintenance_executor.rs
- src/view_models/startup.rs; src/ui/composites/maintenance_forms.rs
- rusqlite backup/transaction behavior in the installed crate source
- `tests/architecture_tests.rs`; `AGENTS.md`

## Files Likely To Change

- src/db/maintenance.rs
- src/application/commands/maintenance.rs; src/presentation/maintenance_executor.rs
- src/view_models/startup.rs; src/ui/composites/maintenance_forms.rs
- src/app/startup.rs — maintenance-session handoff only
- tests/architecture_tests.rs; docs/runbooks/startup-recovery-fixture.py; docs/runbooks/startup-recovery-check.md
- This packet's Status/evidence, the phase plan and review checklist.
- ADR 0066's guard references/partial line and the delivery/deferred indexes as
  appropriate; `docs/pending-human-checks.md` when a runnable visual gate opens.
  Update `AGENTS.md` when the next executable packet changes.

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

## Implementation Steps

1. Consume the drained MaintenanceSession supplied by the session packet; obtain a fresh maintenance-only SQLite connection with non-creating flags and a named five-second contention deadline. Closing one Arc or dropping a command receiver is not evidence that writers stopped.

2. Use SQLite exclusive locking mode and an explicit lock-acquiring transaction to establish maintenance access, retaining the exclusive connection across preparation/preservation. Cover rollback-journal and WAL behavior. If full access cannot be established, return Busy/Unsupported with the reason before copying or changing files. Do not infer exclusive access from an advisory sidecar lock or a successful read.

3. Preserve the main file and associated -wal, -shm and -journal files that exist under the held exclusive access into a new owner-only directory. Record exact source names, lengths and checksums in a manifest, with the observed SQLite mode and UTC time. Never replay or delete a journal manually. If obtaining a usable lock requires SQLite recovery/checkpoint work, report that operation; do not describe the copy as byte-identical to before that work.

4. If a damaged database can still provide safe exclusive access but cannot make a verified snapshot, expose Preserve database files. Label its result a preservation copy, not a verified restorable backup. If damage prevents establishing safe access at all, leave the original untouched and explain that in-app preservation cannot proceed safely; Check again and restore guidance remain available. No raw-copy bypass for that case.

5. Release maintenance access and resume through fresh core verification, keeping the result report. Failed drain/access/copy must not swap any file or claim normal work resumed. A Cancel before copying releases the guard; an in-progress copy must reach a known completed/failed state before session resumption.

6. Add a real second-process SQLite contention fixture. Tests must exercise a separate connection/process during lock acquisition and while maintenance is held; mutex-only mocks do not prove this contract. Preserve files only in a state the backend can prove stable.

7. Record the maintenance access protocol and verified SQLite behavior in this packet's completion evidence for task 012. If the installed SQLite cannot hold the required protection for a supported mode, do not ship an unsafe fallback.

## Acceptance Criteria

Mechanical; assert at the named owner. Test/guard names below marked new must
be implemented by this packet.

| ID | Proof owner | Required assertion |
|---|---|---|
| C1 | two-connection and subprocess tests | A real external writer/reader that prevents exclusive maintenance yields bounded Busy. No preservation runs while access is uncertain; normal app connections have already been closed. |
| C2 | journal-mode tests | Rollback-journal and WAL cases establish and retain the required protection or return a precise unsupported result before touching files. Releasing the guard restores normal access. |
| C3 | preservation tests | The manifest matches all copied database/journal files. Destination failure retains originals and reports partial owned artifacts. A preservation copy is never labelled verified backup. |
| C4 | lifecycle/VM tests | A damaged un-lockable source yields an actionable refusal; copy cancellation/failure cannot resume a stale session or overwrite an unrelated report. |
| C5 | new situational guard adr_0066_database_maintenance_requires_exclusive_access | Only a drained session plus a backend exclusive-access guard can enter preservation or later install. Cite invariant 6. |

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

1. Open Database tools while the fixture holds an external SQLite writer. Attempt preservation; the app must explain the busy database and remain responsive.

2. Release the fixture writer and retry. Inspect the preservation manifest and original paths. The result must say preservation copy, not verified backup.

3. Use the fixture's lockable damaged database, then its invalid-header case. The first may preserve files; the second must explain any access limitation without pretending repair succeeded. Clean up.

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

SQLite cannot establish the promised exclusive protection in a supported journal mode, or a preservation path would require filesystem replacement beneath unknown handles. Report the tested mode and API result; do not improvise OS locks or a raw-copy fallback.
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
- Obtain exclusive database maintenance access after draining the app, and preserve original database files without claiming an unverified copy is a backup.

Constraints:
- Follow this packet's Constraints and Implementation Steps.
- Preserve its data, dependency and prose-retirement contracts.
- One packet this session. Never run the app.

Do not touch:
- File rename/unlink of the configured database, deletion of journals, or candidate installation
- Unsafe lock-file existence tests, process-list guessing, or forcibly killing external writers
- New migration/repair authority

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

