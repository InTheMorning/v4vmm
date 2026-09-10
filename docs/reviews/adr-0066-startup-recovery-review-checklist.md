# ADR 0066: Startup Recovery Review Checklist

## Status And Scope

Task 001 reviewed - 2026-09-10; mechanical gate Green.
Implementation remains partial. Tasks 002–013 retain the requirements below.
Task 001 opens no operator visual gate.

Read the [ADR](../adr/0066-configuration-and-startup-failure-recovery.md),
[phase plan](../plans/adr-0066-startup-recovery-phase-plan.md), active packet and
its diff. The plan owns sequence; packets own procedures/tests; this checklist
owns coverage review. Existing human gates remain in
[pending human checks](../pending-human-checks.md).

## Invariant Coverage

| ADR 0066 invariant | Packet proof owners | Review question |
|---|---|---|
| 1. Core minimum only | 001–004 | Can valid music storage/SQLite still open the normal app when every external service is unreachable? Are music and SQLite usability actually tested? |
| 2. Typed scoped failures | 003/004/007 | Does each failed dependency limit only its dependent work, with no substituted server/player/path or fake healthy observation? |
| 3. Config preservation | 001/006 | Do first-run creation, all ordinary saves and explicit repair preserve the right original bytes? Are unrelated autosaves paused? |
| 4. One snapshot | 001/004 | Do startup and each command's dependent readers share the correct observation, including invalid sibling fields? |
| 5. Recovery and unavailable runtime | 002/003/005/006 | Can repair run without normal DB/runtime, with no ordinary dispatch/autosave in recovery or implicit runtime fallback? |
| 6. Data protection | 002/004/005/009–013 | Are probes rolled back, path bindings contained, sessions drained, candidates validated and originals preserved? |
| 7. Useful safe reports | 002 and every result-producing packet | Does each report name subject, operation, consequence, next action and recorded UTC, without credentials or fabricated success? |
| 8. Window failure fallback | 002 | Can stderr/exit handle no window without another window/runtime dependency? |
| 9. Correction and explicit retry | 006–009 | Is repair reachable, is the original subject retained and revalidated, and does Save alone do no original-operation work? |

## Transferred Verification Coverage

These rows replace the detailed implementation-verification table previously in
the ADR. They describe proof to obtain, not proof already obtained.

| Requirement | Concrete packet |
|---|---|
| Every startup stage has a disposition, including nonfatal activation and programmer invariants | [002](../tasks/adr-0066-task-002-core-checks-and-startup-reports.md#startup-stage-inventory), with 003/004 follow-through |
| Read/list/write/read-back/remove music probe; absent mount and download-only subtree distinguished | [002](../tasks/adr-0066-task-002-core-checks-and-startup-reports.md) |
| Configured SQLite read/write/schema checks, bounded locks, fresh DB preparation | [002](../tasks/adr-0066-task-002-core-checks-and-startup-reports.md) |
| Invalid core versus each invalid optional field/table and valid siblings | [001](../tasks/adr-0066-task-001-config-snapshot-and-safe-persistence.md), [004](../tasks/adr-0066-task-004-optional-tool-isolation.md) |
| Byte preservation across failure, ordinary saves, first-run race/symlink/cleanup | [001](../tasks/adr-0066-task-001-config-snapshot-and-safe-persistence.md) |
| Explicit backup-protected config repair, editor conflicts, fresh validation before autosave | [006](../tasks/adr-0066-task-006-configuration-repair-and-resumption.md) |
| Snapshot remains coherent when file changes after read | [001](../tasks/adr-0066-task-001-config-snapshot-and-safe-persistence.md), [004](../tasks/adr-0066-task-004-optional-tool-isolation.md) |
| Independent work survives optional failure; CLI/keyboard/toolbar guards cannot bypass scope | [003](../tasks/adr-0066-task-003-runtime-failure-and-shell-availability.md), [004](../tasks/adr-0066-task-004-optional-tool-isolation.md), [007](../tasks/adr-0066-task-007-optional-tool-correction-and-retry.md) |
| Missing converter becomes available in same process; real fallback/warning outcome | [008](../tasks/adr-0066-task-008-converter-verification-and-setup.md), [009](../tasks/adr-0066-task-009-conversion-retry-and-retained-input.md) |
| Same-track repair/retry, staging validation/cleanup, no duplicate materialization | [009](../tasks/adr-0066-task-009-conversion-retry-and-retained-input.md) |
| Database tools without TopApp/runtime, check without migration, committed WAL backup | [010](../tasks/adr-0066-task-010-database-check-and-backup.md) |
| Internal drain, extra connections, external writers, original/journal preservation | [005](../tasks/adr-0066-task-005-session-drain-and-resumption.md), [011](../tasks/adr-0066-task-011-database-maintenance-and-preservation.md) |
| Invalid candidate, failed install, validation before resumption | [012](../tasks/adr-0066-task-012-database-restore.md) |
| Same registry and ledger on fresh/migrated/interrupted upgrade; unsupported corruption refused | [013](../tasks/adr-0066-task-013-interrupted-upgrade-repair.md) |
| Existing Null/absent producer/lazy mpv/presentation fallback and ADR 0064 skips survive | [001](../tasks/adr-0066-task-001-config-snapshot-and-safe-persistence.md), [004](../tasks/adr-0066-task-004-optional-tool-isolation.md) |
| Partial repair error preserves completed changes and contains unvalidated bindings | [004](../tasks/adr-0066-task-004-optional-tool-isolation.md) |
| Window close/failure, single-flight checks, stale callbacks and one resumption | [002](../tasks/adr-0066-task-002-core-checks-and-startup-reports.md), [005](../tasks/adr-0066-task-005-session-drain-and-resumption.md), [006](../tasks/adr-0066-task-006-configuration-repair-and-resumption.md) |
| Safe full-copy reports, multi-issue notice and recorded UTC | [002](../tasks/adr-0066-task-002-core-checks-and-startup-reports.md), [003](../tasks/adr-0066-task-003-runtime-failure-and-shell-availability.md), then each tool's VM tests |
| Shared owner/token/VM paths and original architecture guard intent survive | Every packet; actual guard names recorded in its completion evidence |

## Packet Review Procedure

1. Check the file inventory and live caller. Reject uncalled support modules,
   unrelated cleanup, new config keys or unplanned schema changes.
2. Read the actual enum/error/resource types and caller dependencies. Check
   command and query execution as well as displayed availability.
3. Inspect behavior tests at the failure boundary. Require actual WAL/SQLite
   contention and converter process tests where specified, not only source
   assertions or mocks of the claimed behavior.
4. Verify original data before/after failures, including partial operations.
   Reject claims that a failed startup proves rollback.
5. Trace drain and resumption through all connection/actor owners. Dropped
   callbacks and a zero UI activity indicator are not completion evidence.
6. Check report purpose and privacy. A user should identify what acted on
   which resource, what happened, what remains unknown, and what to do next.
7. Read existing guards that change. Preserve their still-live rules; retire
   only superseded claims and name the deciding ADR. Keep the original
   workspace persistence, runtime separation, and task 017 recovery rules.
8. Remove duplicated procedure prose when actual guards land. Task 001 reviews
   the handoff; each successor owns its own retirement. Leave binding ADR
   decisions/invariants intact.
9. Check test commands and outcomes. The lint gate is cargo clippy -- -D warnings.
   No agent GUI run or widened all-targets cleanup belongs in acceptance.
10. Reconcile packet Status, plan row, ADR partial line, delivery row, AGENTS.md
    and any pending operator gate. Do not manufacture a gate for future tooling.

## Operator Acceptance Recording

Each UI packet supplies its own fixture/runbook section before its human check
opens. Record the date, built revision, exact section/case, operator result and
any remaining concern in that packet. A screenshot verifies only the visible
claim; use fixture inspection for bytes, row counts, migration records and
process activity.

Do not re-open ADR 0059 task 017's accepted gate or repeat already accepted
recovery layout merely because another maintenance command is added. Add a
regression check only when an affected owner changed or a concern remains.
Never close a visual gate from mechanical results.

No ADR 0066 operator result is claimed. Task 001 requires no visual check.

## Task 001 Review — 2026-09-10

Scope: [config snapshot and safe persistence](../tasks/adr-0066-task-001-config-snapshot-and-safe-persistence.md#implementation-and-proof).
Green: formatting, cargo check, full cargo test and strict production Clippy.
The suite passed 1,275 unit tests and 221 architecture tests; 10 existing
documentation examples are ignored. No app launch or visual acceptance.

- C1–C5: snapshot, sibling-field, safe-diagnostic, filesystem-failure and
  all-three-save tests are linked in the packet. The injected reader proves one
  observation despite a backing-file change. Real first-run competitors share a
  publication barrier; permission tests include both injected denial and Unix
  file permissions. Byte assertions cover refused ordinary saves.
- C6: reviewed the ADR and all thirteen packet owners against the handoff.
  Every remaining mechanism has a named owner. Tasks 002–013 each retain the
  documentation-proof criterion and explicit prose-retirement constraint.
  Task 013 retains ADR 0016's shared migration registry/ledger, including
  candidate-copy repair; no second schema-mutation authority is introduced.
- The packet's implemented steps and coding prompt were retired in favor of
  live symbols and passing guards. ADR decisions/invariants remain unchanged.
  The existing ADR 0046/0051 guards were updated to the new decoding owner;
  their layout fallback and persistence rules remain guarded.
- Invariants 3–4 have task 001 proof, with explicit correction and complete
  caller migration still assigned to 006 and 004. Core probes, recovery UI,
  optional-tool isolation and database tools remain later work.
- Packet, phase plan, ADR partial line/index, delivery/deferred indexes,
  docs index and AGENTS.md agree: 001 complete; 002 next; series Accepted.
  Pending human checks are unchanged. No deviation from the bounded scope.

## Final Series Review

ADR 0066 remains Accepted until all thirteen packets and their operator gates
are complete. Confirm the following before changing it to Implemented:

- All invariants and transferred requirements above have actual evidence.
- No expected startup failure remains as an unclassified panic; programmer
  invariants remain distinct.
- Invalid optional settings do not reappear as core failure through a secondary
  reader, and a missing runtime cannot construct an implicit runner.
- Repair and retry are useful in both normal Settings and core recovery.
- Migration repair uses the normal registry and recording path.
- No unsafe pathname replacement or empty database fallback exists.
- All remaining limitations are stated in reports and completion evidence.
- Deferred item 6 and the config-format dependency are reconciled; unrelated
  inherited human gates remain accurately indexed.
- The next session is selected from the canonical delivery order.

## Review Result Format

Report pass/fail, required fixes, optional improvements, whether this packet is
ready to merge, and whether its successor needs a bounded amendment. Distinguish
a mechanical pass from pending operator acceptance.

