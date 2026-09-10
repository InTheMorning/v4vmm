# ADR 0066: Startup Recovery Phase Plan

## Status

Implementation in progress - 2026-09-10.
ADR 0066 remains Accepted. Tasks 001 and 002 are complete. Task 002's
[operator check](../runbooks/startup-recovery-check.md#task-002-core-checks-and-reports)
passed and fixture cleanup is confirmed. Task 003 is next in a fresh session.
Tasks 003–013 have not started.

This plan executes [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md).
The [delivery order](broadcast-chain-delivery-order.md#current-delivery-order)
still owns cross-repository priority. Finish and verify this series before
changing the configuration format. Relay durability through adoption follows.
The real-show scheduling trigger still moves publisher show-log task 001 first.

## Goal

A broken core requirement leads to an understandable recovery screen and repair
tools. A broken optional tool leaves the app usable, explains what cannot work,
and offers a direct correction and retry route.

Normal operation requires only valid core configuration, usable music-file
storage, and verified SQLite. No network response is required to open the app.

## Non-Goals

No workspace key migration, automatic defaults-start, empty replacement database,
general corruption salvage, new remote service-state vocabulary, relay changes,
or download-format policy change. Do not implement ADR 0064 task 002's repair
history surface here. Log wrapping/following, local-time preferences, cross-repo
UTC corrections, narrow Show layout, and Clippy/decomposition debt stay separate.

## Assumptions And Current Boundaries

- The operator accepted ADR 0066 on 2026-09-10. Tooling is optional; a current
  constructor's expectation is not a product requirement.
- Configuration readers and savers are in config.rs. A strict compatibility
  reader remains temporarily while scoped consumers are migrated.
- The normal shell currently assumes a player and runner; both TopApp and
  LibraryApp fall back to an implicit runner when RuntimeHost is absent.
- Database connections include the main shared connection and the library
  paged actor's separately opened connection. Dropping command result receivers
  does not cancel blocking work.
- SQLite currently initializes/migrates through db.rs. Maintenance checks must
  not call that mutating entry point. Repair uses the existing MIGRATIONS and
  version-recording authority, including when operating on a candidate.
- Event reports already record UTC under ADR 0063. ADR 0066 extends that scoped
  precedent; this series does not choose cross-repo timestamp precision.
- These are staged slices. Task 002 proves core recovery; optional constructor
  failures remain assigned to 003/004 until those packets pass. The temporary
  coupling is a recorded incomplete implementation, never a new core rule.
- New files/types/guard names in packets are proposed implementation targets.
  They must acquire a live caller in their owning packet, not be parked as
  scaffolding for an unnamed later phase.

## Sequence And Stopping Points

Execute in this order, one packet per session. Each row stops after its own
mechanical gate and any applicable operator acceptance. Do not silently roll
an unwalked visual gate into a claim that the next dependency is complete.

| Packet | Usable result | Prerequisite | State |
|---|---|---|---|
| [001: Config Snapshot And Safe Persistence](../tasks/adr-0066-task-001-config-snapshot-and-safe-persistence.md) | Read configuration once, distinguish core errors from optional errors, and prevent ordinary saves from destroying a document that needs repair. | Accepted ADR | Complete - 2026-09-10; mechanical gate Green; no visual gate |
| [002: Core Checks And Startup Reports](../tasks/adr-0066-task-002-core-checks-and-startup-reports.md) | Show a useful recovery screen for broken core configuration, unusable music storage, or unusable SQLite, with safe checks and a single startup lifecycle. | 001 | Complete - 2026-09-10; mechanical gate Green; operator acceptance and fixture cleanup confirmed |
| [003: Runtime Failure And Shell Availability](../tasks/adr-0066-task-003-runtime-failure-and-shell-availability.md) | Keep navigation, reports and repair access working when the normal background runtime or optional thumbnail worker cannot start. | 002 | Not started |
| [004: Optional Tool Isolation](../tasks/adr-0066-task-004-optional-tool-isolation.md) | Open the app with valid core resources even when optional configuration or tool preparation fails, and limit only the operations that actually depend on each failure. | 003 | Not started |
| [005: Session Drain And Resumption](../tasks/adr-0066-task-005-session-drain-and-resumption.md) | Stop the app's own work, release every configured database handle, and resume one fresh session before any live core correction or database maintenance can use this transition. | 004 | Not started |
| [006: Configuration Repair And Resumption](../tasks/adr-0066-task-006-configuration-repair-and-resumption.md) | Repair configuration inside recovery or Settings, preserve the original file, and return to a freshly verified app session. | 005 | Not started |
| [007: Optional Tool Correction And Retry](../tasks/adr-0066-task-007-optional-tool-correction-and-retry.md) | Turn optional-tool failures into a direct Settings correction route and an explicit, freshly checked retry of the original action. | 006 | Not started |
| [008: Converter Verification And Setup](../tasks/adr-0066-task-008-converter-verification-and-setup.md) | Let the operator configure and freshly test FLAC/ffmpeg availability without restarting the app, while preserving the actual conversion fallback policy. | 007 | Not started |
| [009: Conversion Retry And Retained Input](../tasks/adr-0066-task-009-conversion-retry-and-retained-input.md) | Return from converter setup to the same track, reuse valid downloaded input where possible, and avoid duplicate library materialization. | 008 | Not started |
| [010: Database Check And Backup](../tasks/adr-0066-task-010-database-check-and-backup.md) | Offer database inspection and a verified SQLite backup from both Settings and core recovery, without requiring the normal app runtime. | 009 | Not started |
| [011: Database Maintenance And Preservation](../tasks/adr-0066-task-011-database-maintenance-and-preservation.md) | Obtain exclusive database maintenance access after draining the app, and preserve original database files without claiming an unverified copy is a backup. | 010 | Not started |
| [012: Database Restore](../tasks/adr-0066-task-012-database-restore.md) | Restore an explicitly chosen validated backup through the shared maintenance path, preserve the current database, and reopen only after verification. | 011 | Not started |
| [013: Interrupted Upgrade Repair](../tasks/adr-0066-task-013-interrupted-upgrade-repair.md) | Repair one recognized interrupted schema upgrade using the existing migration authority, then complete ADR 0066's implementation evidence. | 012 | Not started |

The sequence deliberately separates internal session draining (005) from
exclusive database access/preservation (011). Configuration correction needs
the former. Restore needs both. Converter verification (008) can be reviewed
without changing staging ownership; retry/materialization (009) then uses it.
Database inspection/backup (010) ships before any installation action.

Task 013 also reconciles the final evidence. It does not re-run accepted visual
checks by default or implement the next cross-repository phase.

## Owners And API Handoffs

These are planned internal API responsibilities, not an extra protocol or new
TOML format. Packets own the concrete procedures and assertions.

| Owner | Handoff |
|---|---|
| config.rs — 001/006 | ConfigSnapshot: original bytes, core values, scoped optional values/issues. Ordinary persistence guard and a separate explicit correction command. |
| startup.rs — 002/004 | CoreCheckOutcome, PreparedCore and scoped resource issues. Check-only versus explicit preparation. No GPUI types. |
| view_models/startup.rs — 002 onward | Safe recorded reports, issue collection, typed check/edit/maintenance/retry actions. |
| presentation/maintenance_executor.rs — 002 onward | One fallible background worker, one running operation and one bounded queued request. No normal runtime or main DB prerequisite. |
| application/async_command_runner.rs — 003/005 | Explicit unavailable runner, then atomic command admission and active-work tracking for drain. |
| application/session_lifecycle.rs — 005 | Running → Draining → Maintenance → Resuming. Own a MaintenanceSession only after tracked work and connections stop. |
| app/startup.rs, bootstrap.rs and startup_presenter.rs | Compose one recovery/normal lifecycle, route completions and exit outcome. Never perform domain I/O in a screen. |
| ui/composites/startup_report.rs and maintenance_forms.rs | Shared report, issue notice and tools in Settings/recovery; named tokens and accessible narrow layout. |
| application/capability_recovery.rs — 007 | Typed original-action identity/inputs and explicit verified retry; no captured UI closure. |
| audio_format.rs — 008 | Fresh bounded FLAC/ffmpeg observations. Existing fallback policy remains. |
| application/conversion_recovery.rs, track_compare.rs, subscribe_service.rs — 009 | One session owner for retained input; retry shares normal validation/materialization. |
| db.rs and db/maintenance.rs — 010–013 | Schema facts and migration authority remain in db.rs; maintenance owns inspection, snapshots, exclusive access, candidate validation and installation. |

Module exports are registered in their owning packet. Follow existing Rust
layering, private-by-default APIs, typed application errors and backend
anyhow context. Do not migrate unrelated code just because a new module exists.

### Schema And Installation Implications

No new durable application table or config key is planned. The rolled-back
startup probe leaves no schema. Session issues/retry state are not durable logs.

Task 010 enables the existing rusqlite version's backup feature. Task 012 installs
through SQLite's backup API into a maintenance-only connection, retaining the
configured database pathname/inode. No main-file or journal rename occurs beneath
unknown handles. Task 011 must prove exclusive access with real competing SQLite
connections/processes first. If damaged headers prevent safe access or SQLite
installation, retain the original and report the supported recovery limits;
there is no force-unlink fallback.

Task 013 recognizes one named interrupted condition: migration 11's compatible
table exists but its ledger entry was not recorded, after a valid 1–10 prefix.
It reuses the same migration registry and recording path. Broader damaged schemas
do not earn a generic Repair button. Older valid backups can be upgraded only
as an explicit candidate preparation through that same registry.

## Verification Strategy

The [review checklist](../reviews/adr-0066-startup-recovery-review-checklist.md)
maps every invariant and transferred ADR verification requirement to packets.
Each packet names proposed tests/guards, exact changed owners, regression
assertions, test commands and a distinct operator check.

Mechanical proof uses injected failures and temporary paths, never resource
exhaustion or the real operator database. Use genuine SQLite contention for
maintenance tests, committed WAL data for backup tests, and actual subprocess
stubs for converter tests. A green source assertion does not prove lifecycle
or storage semantics; pair it with behavioral tests.

The normal gate stays cargo check, format, tests and cargo clippy -- -D warnings.
Build the binary when a new operator fixture is ready. Do not broaden Clippy to
--all-targets as part of this series. Agents never run the app.

### Fixture Contract

Task 002 supplies the [fixture](../runbooks/startup-recovery-fixture.py) and
[operator runbook](../runbooks/startup-recovery-check.md#task-002-core-checks-and-reports).
The fixture owns setup, manifest verification/location, six core-failure modes,
isolated operator launch, preservation inspection and owned-process cleanup.
Its backend commands passed. Operators have verified the TOML report and
preservation, both music-path cases and the repeated-check feedback correction.
Locked-database feedback and responsiveness also passed. Core rechecks passed
after lock release, but window-manager closing
crashed. The queued-close correction passed its normal-window recheck with
exit 0, followed by normal-session preservation. Narrow long paths and blocked
recovery closing also passed. The final operator confirmation accepts TOML
screen/control/disclosure and Quit behavior, and same-window resumption after
lock release. All visual checks passed; fixture cleanup is confirmed. Passed cases
need no repeat.
Later packets extend these owners for their additional cases rather than
duplicating setup or schema construction.

Use Rust fixture support for current-schema construction and migration failure
seams; Python orchestrates files/stubs rather than duplicating db.rs schema.
Any GUI failure injection must be debug-only, explicitly activated for a verified
fixture, and absent/inert in release. Keep fixture hooks outside renderer logic.
Document their exact owner and compile guard in task 002/003.

The runbook must supply complete **unindented** command blocks with no Python
heredoc. Show one short purpose sentence before each check. State required OS/tool
conditions, what would be wrong, and cleanup. Do not print prospective commands
as though a not-yet-implemented fixture already exists.

## Risk Areas

| Risk | Owning proof |
|---|---|
| Default creation or autosave destroys the file being repaired | 001 byte comparisons; 006 backup/conflict tests |
| Secondary reader restores whole-config failure after startup succeeds | 004 command/query/CLI caller inventory |
| Runtime/player coupling creates a new minimum requirement | 003/004 absent-resource construction and paired independent-action tests |
| Worker or callback duplication after repair | 002 generation checks; 005 admission/drain/close/resume tests |
| Partial path repair is described as rolled back or used unsafely | 004 injected failure after a committed statement |
| Converter repair loses input or duplicates a library entry | 008 fresh probes; 009 owned retention/materialization tests |
| Copy omits WAL or maintenance races another process | 010 WAL snapshot tests; 011 external contention tests |
| Restore bypasses preservation or schema compatibility | 012 candidate/order/failure tests |
| Repair invents a second schema authority | 013 normal/fresh/migrated/interrupted ledger equivalence |

## Rollback Strategy

Revert a failing packet's code as a coherent change and leave its successors
pending. Preserve operator data and all recovery artifacts. A code rollback is
not authorization to undo migrations, delete journals, move music, or reset
configuration.

Config correction reports the exact preserved original. Database actions retain
original/candidate artifacts and report their meaning. An unsupported restore
does not quietly become a filesystem swap. Resumption always rechecks core
resources; no result claims that unrelated earlier startup mutations rolled back.

## Documentation And Acceptance

Task 001's [handoff table](../tasks/adr-0066-task-001-config-snapshot-and-safe-persistence.md#mechanism-handoff)
owns the transfer from ADR prose to implementation packets. Each packet removes
its duplicated procedures when guards land and records the actual guard symbol
plus verification artifact. Do not delete binding ADR invariants.

During implementation, update the active packet, this sequence, ADR partial
line, delivery row and AGENTS.md together. Add each runnable human gate to
[the pending index](../pending-human-checks.md). No gate is added merely because
this plan specifies a future test. Preserve all inherited gates.

Only all thirteen completed packets plus their actual operator acceptance permit
ADR 0066 to become Implemented, deferred item 6 to close, and the config-format
dependency to release. Tasks 001 and 002 are complete;
the series remains partial.
