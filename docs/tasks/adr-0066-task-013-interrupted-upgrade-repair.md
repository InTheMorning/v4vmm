# ADR 0066 Task 013: Interrupted Upgrade Repair

Status: Ready after the preceding packet - 2026-09-10.
Implementation not started. Operator check specified below; not runnable or accepted yet.

## Goal

Repair one recognized interrupted schema upgrade using the existing migration authority, then complete ADR 0066's implementation evidence.

## Read First And Dependency

Read [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md),
the [phase plan](../plans/adr-0066-startup-recovery-phase-plan.md), this whole packet, and the
[review checklist](../reviews/adr-0066-startup-recovery-review-checklist.md).
Execute after [task 012](adr-0066-task-012-database-restore.md) in the phase plan's order.
Complete this packet in one session; do not start its successor.
Names marked **new**, including tests and guards, are implementation targets,
not claims that those files or symbols already exist. If a predecessor already
created a listed owner, extend that owner.

## Files To Inspect

- src/db.rs — MIGRATIONS, migrate_schema, record_migration, migration_applied, create_broadcast_event_selection_table
- src/db/maintenance.rs — validated candidate and install path
- src/application/commands/maintenance.rs; src/view_models/startup.rs; src/ui/composites/maintenance_forms.rs
- docs/adr/0016-schema-migration-discipline.md
- docs/reviews/adr-0066-startup-recovery-review-checklist.md
- `tests/architecture_tests.rs`; `AGENTS.md`

## Files Likely To Change

- src/db.rs — migration inspection/reuse and precise recognized condition only
- src/db/maintenance.rs
- src/application/commands/maintenance.rs; src/view_models/startup.rs; src/ui/composites/maintenance_forms.rs
- tests/architecture_tests.rs; docs/runbooks/startup-recovery-fixture.py; docs/runbooks/startup-recovery-check.md
- ADR 0066 and its plan, checklist, packet states, delivery/deferred indexes and AGENTS.md — evidence-based closure only
- This packet's Status/evidence, the phase plan and review checklist.
- ADR 0066's guard references/partial line and the delivery/deferred indexes as
  appropriate; `docs/pending-human-checks.md` when a runnable visual gate opens.
  Update `AGENTS.md` when the next executable packet changes.

## Do Not Touch

- Independent repair SQL registry, fabricated applied versions, or general corruption salvage
- Retroactive rewriting of an applied migration, schema reset, or deleting user rows to pass checks
- Workspace format changes, relay work or unrelated open visual gates
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

1. Name the first supported recipe narrowly: complete migration 11, broadcast_event_selection, after its table creation succeeded but its ledger row was not recorded. Require a valid schema through versions 1–10, their expected names, a compatible existing broadcast_event_selection table, no unknown later migration, and a passing integrity check. A merely missing ledger row in an arbitrary database is not enough.

2. Recognize the condition through read-only schema/ledger inspection. Report the missing migration version/name and the proposed operation. Offer Repair interrupted upgrade only for this proven condition; unsupported/inconsistent schemas offer preservation/restore/check guidance instead of guessed SQL.

3. Preserve the original under exclusive maintenance, create a candidate using the existing snapshot path, and call the SAME migrate_schema/MIGRATIONS/record_migration path used for normal preparation against that candidate. No repair-only SQL or ledger insert. If more robust migration atomicity is necessary, change the shared registry execution path and prove normal and repair behavior together; do not create a parallel engine.

4. Validate candidate schema/ledger, integrity, foreign keys and preserved event selections/rows, then use task 012's controlled installation. Report what migration was recorded and what data was preserved. Failed recognition/apply/install leaves useful preservation paths and recovery available.

5. For otherwise valid older backups, prepare a candidate through the same normal migration registry after an explicit Upgrade backup action, then return it to task 012's validation/review. Do not silently migrate the operator's chosen backup or broaden the interrupted-repair recognizer to arbitrary partial schema states.

6. Extend the fixture with a migration-11 interruption produced by a test-only failure seam between apply and record_migration. Do not duplicate migration SQL in Python. Test interruption before apply, after apply, after recording, and during candidate installation.

7. Review the series checklist and all packet evidence. Replace duplicated recipe/procedure prose with actual guard names and the fixture/runbook anchor. Mark ADR 0066 Implemented only when all packets and their operator gates have passed; otherwise keep Accepted with an accurate partial line. Reconcile deferred item 6 and release the item 7 dependency only at that completion point. Do not run relay or another implementation phase in this session.

## Acceptance Criteria

Mechanical; assert at the named owner. Test/guard names below marked new must
be implemented by this packet.

| ID | Proof owner | Required assertion |
|---|---|---|
| C1 | migration recognizer tests | Only the named supported interrupted state offers repair. Missing/wrong columns, unrelated missing versions, unknown later versions and corruption are rejected without schema mutation. |
| C2 | fresh/legacy/interrupted migration tests | Normal preparation and candidate repair use the same version/name records. Existing test_migrations_record_versions_on_fresh_schema and test_migrations_update_legacy_schema remain; interrupted apply/record boundaries preserve data and do not double-apply work. |
| C3 | repair/installation tests | Original preservation precedes candidate mutation; repaired candidate passes integrity/schema checks; failed install retains recovery data and cannot falsely resume normal work. |
| C4 | older-backup tests | Explicit candidate upgrade preserves the chosen source and uses the normal registry; only a validated result reaches Restore review. |
| C5 | new situational guard adr_0066_upgrade_repair_uses_normal_migration_authority | Repair calls shared registry execution/version recording and shared maintenance installation. Cite ADR 0016 and ADR 0066 invariant 6. |
| C6 | documentation/evidence review | Every invariant has named proof, every implemented visual change has recorded operator acceptance or an open tracked gate, and all status/index records agree. No unrun gate is closed by mechanical tests. |

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

1. Open the interrupted-upgrade fixture in recovery. The report must name migration 11 and explain what Repair interrupted upgrade will do.

2. Run the repair. Inspect the preserved original, repaired ledger and event selection. Open app and verify the fixture's library and saved event still exist.

3. Try the unsupported partial-schema fixture. It must explain why this recipe does not apply and offer preservation/restore guidance. Clean up; record only this packet's new inspection, without rerunning already accepted layouts.

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

Migration 11 is not safely identifiable/replayable under the actual schema, the recipe needs a second mutation/version authority, or closure evidence is missing. Narrow/refuse the recipe or keep the relevant gate open; never mark the ADR Implemented to finish the packet.
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
- Repair one recognized interrupted schema upgrade using the existing migration authority, then complete ADR 0066's implementation evidence.

Constraints:
- Follow this packet's Constraints and Implementation Steps.
- Preserve its data, dependency and prose-retirement contracts.
- One packet this session. Never run the app.

Do not touch:
- Independent repair SQL registry, fabricated applied versions, or general corruption salvage
- Retroactive rewriting of an applied migration, schema reset, or deleting user rows to pass checks
- Workspace format changes, relay work or unrelated open visual gates

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

