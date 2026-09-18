# ADR 0066 Task 013: Interrupted Upgrade Repair

Status: Complete — 2026-09-18. Mechanical checks Green.
Operator V1–V3 behavior and the [ADR 0074 presentation correction](../adr/0074-repair-and-diagnostics-pages.md)
are accepted. Settings and recovery passed normal/narrow and short-height
checks, including report copy, scrolling and page navigation. Settings passed
XL scale in both themes and preference restoration. The operator reported both
final preservation inspections as passed and confirmed cleanup of both fixtures.
ADR 0066 remains Accepted. Task 004 retains its independent gate.

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

## Files Changed

- Schema and migration: `src/db.rs`, new `src/db/upgrades.rs`,
  `src/db/startup.rs`, `src/db/maintenance.rs`, `src/db/maintenance/restore.rs`.
- Commands and presentation: `src/application/commands/maintenance.rs`,
  `src/view_models/startup/database.rs`, `src/presentation/database_tools.rs`,
  `src/ui/composites/maintenance_forms.rs`, `src/app/startup.rs`.
- Verification and fixtures: `tests/architecture_tests.rs`,
  `src/startup/fixture.rs`, `docs/runbooks/startup-recovery-fixture.py`,
  `docs/runbooks/test_startup_recovery_fixture.py`.
- Documentation: this packet, task 001's handoff, task 012's successor status,
  the operator runbook, review checklist, ADR 0066 and its index, phase plan,
  delivery/deferred indexes, pending-human index, docs index, `AGENTS.md` and
  `.github/copilot-instructions.md`.

No documentation files or folders were created or moved. Existing root
`AGENTS.md` remains the workflow owner; other canonical root documents remain
in place. Local links and heading anchors in changed documentation are Green.

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

## Implementation And Proof

`db::inspect_schema` distinguishes `InterruptedUpgrade` from an ordinary older
schema. `db::upgrades::recognize_migration_11` compares the existing schema with
the normal authority's in-memory schema; startup keeps this state in recovery.
`DatabaseVm` offers repair only after a successful read-only check and session
drain. The command rechecks the configured source and session, then obtains
`ExclusiveDatabase` before recognition and preservation.

`repair_interrupted_upgrade` preserves files and a verified snapshot before
`migrate_candidate` calls the shared `migrate_schema` / `MIGRATIONS` /
`record_migration` path. `database_digest` proves every existing row, event
selection, schema object and prior ledger record survives, excluding only the
new migration-11 record. `ValidatedRestore::install_prepared` is the same
controlled SQLite installation and verification owner as task 012. There is no
parallel repair SQL or ledger writer.

`ValidatedRestore::upgrade_backup` prepares only a private candidate. Its
explicit VM action returns to the existing session/configuration-bound Restore
review; it does not replace the configured database or migrate the chosen file.

UI ownership stays in `view_models/startup/database.rs`,
`presentation/database_tools.rs` and the existing shared `maintenance_forms`
composite. Geometry uses its existing named spacing, typography and control
styles. `app/startup.rs` routes the existing drain, independent worker and fresh
resumption lifecycle. No configuration or durable schema format changes.

### Mechanical Acceptance

| ID | Actual proof |
|---|---|
| C1 | `adr_0066_upgrade_recognizer_rejects_inconsistent_schema_without_writes`; `adr_0066_upgrade_integrity_damage_never_authorizes_repair`; `adr_0066_upgrade_schema_recognizes_legacy_column_order_without_ignoring_sql_values`; schema/ledger checks in `db::inspect_schema`; `adr_0066_upgrade_actions_require_fresh_recognition_and_explicit_candidate_preparation` |
| C2 | `adr_0066_upgrade_boundaries_share_normal_registry_and_preserve_selection`; existing `test_migrations_record_versions_on_fresh_schema` and `test_migrations_update_legacy_schema` |
| C3 | `adr_0066_upgrade_repair_preserves_before_install_and_verifies_failure_rollback`; `adr_0066_upgrade_failed_preservation_and_unsupported_schema_do_not_mutate`; `adr_0066_upgrade_command_requires_configured_source_and_drained_session`; existing task 012 process-contention/install tests |
| C4 | `adr_0066_upgrade_backup_requires_explicit_candidate_and_preserves_chosen_source`; `adr_0066_upgrade_backup_returns_to_review_without_installing` |
| C5 | Situational `adr_0066_upgrade_repair_uses_normal_migration_authority` in `tests/architecture_tests.rs`, citing ADR 0016 and ADR 0066 invariant 6; task 012's guard retains its preservation/installation rules while allowing the explicitly gated candidate migration |
| C6 | [Series evidence reconciliation](../reviews/adr-0066-startup-recovery-review-checklist.md#task-013-review--2026-09-17); existing completed packets stay accepted; task 013 is complete and task 004 retains its independent gate |

The implementation procedure and coding prompt are retired in favor of these
symbols and the [task 013 operator procedure](../runbooks/startup-recovery-check.md#task-013-interrupted-upgrade-repair).
`db::upgrades::interrupt_fixture` is compiled only for tests/debug builds;
`startup::fixture::seed_upgrade_checks` constructs the interruption through the
shared executor's apply/record failure seam. Python orchestrates the isolated
files, unsupported ledger case and preservation inspection without migration
DDL. The existing verified-debug-fixture installation hook is inert in release.

### Mechanical Evidence — 2026-09-17

Green: `cargo fmt -- --check`, `cargo check --quiet`, `cargo test --quiet`,
`cargo clippy --quiet -- -D warnings`, `cargo build --quiet --bin v4vmm`.
The full final suite passes **1,476 unit tests and 259 architecture guards**;
ten existing documentation examples remain ignored. All **39 Python fixture
tests** pass. The normal desktop binary was rebuilt after the final tests.
Task proof-symbol references, changed documentation links and `git diff --check`
are Green. Three existing loopback tests were sandbox-denied during a focused
run; the approved unrestricted rerun and full final suite passed.

Backend fixture smoke passed setup, Rust seam construction, saved selection,
older backup state, read-only inspection, missing-evidence refusal, interruption
controls, prevention of reseeding, and cleanup for both new modes. Its private
fixtures `/tmp/v4vmm-startup-doce9a9r` and `/tmp/v4vmm-startup-90f3ub4x` were
removed. This did not launch a desktop or establish visual acceptance.

No architectural deviation. The new schema-read helper stays under `db`;
normal migration atomicity is unchanged because migration 11 is idempotent and
all repair mutation occurs in a preserved candidate. The recognizer deliberately
refuses schema definitions outside the authority-generated contract, including
extra objects and inconsistent constraints. General corruption salvage remains
unsupported. No unresolved mechanical failure remains.

Operator V1–V3, presentation and preservation acceptance are recorded below.
Fixture cleanup is confirmed. Task 004's separate gate prevents full
ADR 0066 closure.

## Operator Evidence — 2026-09-18 UTC

Fixture: `/tmp/v4vmm-startup-hyy6ux18`. Evidence in this section comes from the
operator's copied reports and confirmation. The operator reported the first
artifact inspection as passed; its raw output was not supplied.

- Startup check 1 at 00:42:38 UTC and check 2 at 00:45:53 UTC identify the
  unrecorded migration 11 `broadcast_event_selection`. Both reports keep normal
  library/show operations closed. The operator confirmed Open app remains
  unavailable after Check again.
- Database checks at 00:46:15 and 00:47:34 UTC report read-only access, integrity
  `ok` with the documented read-only CHECK-constraint limit, no foreign-key
  violations, the compatible selection table and the valid 1–10 migration
  prefix. Neither check initialized, migrated or repaired the database.
- The repair result at 00:49:33 UTC records exclusive access at 00:49:32 UTC,
  one preserved database/journal file and
  `upgrade/failed-preservation/manifest.json`. It names the verified original
  snapshot at
  `upgrade/failed-preservation/.v4vmm-database-814801-0/candidate.sqlite`.
  The injected page-batch installation failure reports verified SQLite rollback:
  destination schema and records match their pre-installation fingerprint.
  The report states that recovery remains open.
- A fresh database check at 00:52:55 UTC still recognized the interrupted
  migration. The successful repair at 00:53:30 UTC recorded migration 11 through
  the normal registry and reported preservation of all existing rows, event
  selections, schema objects and prior migration records. It recorded exclusive
  access at 00:53:30 UTC, one preserved database/journal file and
  `upgrade/repaired-preservation/manifest.json`. The verified original snapshot
  is `upgrade/repaired-preservation/.v4vmm-database-814801-2/candidate.sqlite`.
  Installation verification covered schema, records and a rolled-back write
  probe.
- The operator accepted automatic Music resumption in the same window, the
  retained startup fixture playlist and tracks, and **Upgrade fixture saved
  event** in Show. Both repair reports remained available in Settings >
  Diagnostics > Database tools, with readable reports and reachable controls
  at normal and narrow window widths.
- V2's restore review at 01:08:18 UTC refused `upgrade/older-backup.sqlite`
  because it required explicit migration preparation; no database was installed.
  At 12:33:30 UTC, **Upgrade backup** reported normal migration preparation on
  a separate candidate, followed by successful validation and an explicit
  Restore review. The review names the original backup, configured database,
  `upgrade/backup-restore-preservation` and the validated candidate at
  `upgrade/.v4vmm-database-814801-5/candidate.sqlite`. It reports that neither
  the chosen backup nor the configured database was replaced.
- The operator reported a pass for the unchanged library and saved event after
  preparation and for `upgrade-inspect` after closing the app. Restore remained
  a separate, unexecuted action in this procedure.
- V3 uses `/tmp/v4vmm-startup-t7hwzh63`. Configuration selection at 12:38:27
  UTC named that fixture's database. Database checks at 12:38:30 and 12:38:33
  UTC reported read-only access, integrity `ok`, no foreign-key violations and
  an unrecognized schema or migration ledger. Both reports gave guidance to
  retain the original and use a compatible app or inspect a known backup;
  neither check initialized, migrated or repaired the database.

V1–V2 behavior and the first preservation inspection remain accepted. The
operator rejected the combined repair/diagnostics presentation after the V3
report checks. Earlier Settings presentation acceptance does not accept the
replacement pages. Later V3 control and replacement-page acceptance are recorded
below. Final preservation inspections are accepted and cleanup is confirmed.

## Presentation Correction — ADR 0074

The operator reported oversized text, logs that capture page scrolling, and
poor separation between commands, instructions and reports. The operator
requested pages and a separate action column on 2026-09-18. Acceptance paused at
that point. Later results are recorded below.

[ADR 0074](../adr/0074-repair-and-diagnostics-pages.md) records the correction.
Diagnostics selects one tool page. Recovery selects Startup, Configuration or
Database. Database tasks have separate pages. Instructions and Report occupy
separate views. The shared composite places actions beside the content at normal
widths and above the content at narrow widths. Reports retain the existing log
owner, selection, copying and reading state. Instructions use the scaled Body
token. Changed instructions and database reports use short, explicit sentences
under the asd-ste100 skill. The official word dictionary was not checked.

The operator then accepted the page choices and text size. A subsequent
382-pixel-high Settings screenshot showed the task below several navigation
rows. The follow-up combines the Settings header, uses menus for tool and task
selection, and places one Show report/Show instructions button inside the
content container beside the task selector. Every Settings group retains the
Settings title. Narrow
action bands reserve one quarter of the available body height, with enough
space for one complete button. Startup commands belong to the Startup page.
The new renderer check mounts the complete Settings and recovery compositions at short
height, normal/narrow widths and larger scale. Operator results are recorded below.

The operator provisionally accepted the secondary pager, then requested the
shared Settings title and one view toggle inside the content container. The
operator accepted the restored title, button placement and switching in both
directions. The subsequent Database Check walkthrough passed: the report opens,
instructions and input fields remain reachable in a short window, report text
scrolls and selects, and the copied database report is complete. Database task
navigation, relevant fields, retained destination input and report reading
position also passed. The operator used the unused destination text
`/tmp/v4vmm-startup-hyy6ux18/upgrade/ui-input-check.sqlite` without running a
database command. Configuration draft retention across pages, Escape focus on
Close editor, keyboard close and reopening with the draft retained also passed.
The operator reloaded the file to discard the test draft without saving.
These corrections remain within this packet.

The theme and scale check exposed an empty Background tools page. The full page
used the compact notice's failure-only rows. The view model now keeps nine tool
entries visible when no failure is recorded. Failures and retained actions come
first. Existing edit/check actions retain their availability rules. Opening the
page creates no observation and runs no check. The compact notice still omits
idle tools. The operator accepted the correction at normal and narrow widths:
tool names, instructions and buttons appear, both areas scroll to their final
entries, and the list remains after switching to Report and back. Tool checks
remained unclicked.

The operator accepted all five Diagnostics pages at normal and narrow widths,
including approximately 380-pixel height, with XL scale in Light and Dark
themes. Menu mouse selection, keyboard selection and Escape passed. The
operator restored the original theme and scale. Save and maintenance actions
remained unclicked.

The operator reported walkthrough steps 1–5 as passed, then requested the route
to Startup. The subsequent screenshot shows normal Settings and the repaired
fixture's `hyy6ux18` database path. The agent incorrectly assumed the recovery
window was active. The earlier Settings acceptance remains valid. The latest
pass cannot yet establish recovery presentation or unsupported-fixture action
acceptance. The earlier unsupported-schema reports remain recorded.

The next screenshot confirms recovery in `/tmp/v4vmm-startup-t7hwzh63`.
Startup check 1 completed at 2026-09-18 15:05:34 UTC. Its observations are
recorded at 15:05:33 UTC. The report identifies the unsupported database schema
or migration record and states that the app did not change it or open normal
library/show operations. Open app is visibly disabled. This establishes the
initial recovery state, not the result of an explicit Check again.
The operator then accepted the explicit Check again: check 2 completed, the
report still identified the unsupported schema, and Open app remained disabled.
No completion timestamp was supplied for check 2.

The operator then accepted the remaining checks in the confirmed recovery
window. Startup, Configuration and Database passed normal/narrow and
approximately 380-pixel-height inspection. Instructions/report switching,
scrolling and button access passed. Check database reported the unsupported
schema, the copied report was complete, and Repair interrupted upgrade was
absent. V3 behavior and the ADR 0074 visual gate are accepted.

The operator then reported both final `upgrade-inspect` commands as passed for
`/tmp/v4vmm-startup-hyy6ux18` and `/tmp/v4vmm-startup-t7hwzh63`. This confirmation
accepts all preservation flags in both inspections. The raw JSON outputs were
not supplied. The agent did not run these inspections in the desktop session.
The operator subsequently confirmed that both cleanup commands printed
`Removed fixture`. Both isolated fixtures and their preservation copies are
removed. Task 013 has no open gate. ADR 0074 is Implemented. Task 004 retains its
independent gate, so ADR 0066 stays Accepted and configuration-format work stays gated.

Owners: `src/view_models/maintenance.rs`, `src/view_models/settings.rs`,
`src/view_models/startup/database.rs`, `src/view_models/startup/capabilities.rs` and
`src/ui/composites/maintenance_page.rs`. Existing Settings/recovery screens,
maintenance forms and presenters compose these owners. Database commands,
installation and preservation remain with their existing owners.

Mechanical verification is Green — 2026-09-18: 1,484 unit tests and 259
architecture guards pass. Ten pre-existing doctests remain ignored. Formatting,
`cargo check`, strict Clippy and the normal desktop build pass. The desktop
binary was rebuilt after the final tests. The full suite used local socket
access after the sandbox blocked existing socket tests.

Named guards:

- `adr_0074_background_tools_remain_visible_without_failures`: full-page tools
  before and after a recorded success, no invented observations and an empty
  compact notice when no failure or retained action exists.
- `adr_0074_background_tools_keep_failure_routes_and_check_gates`: specific
  configuration routes, no duplicate tool checks and existing busy/unavailable
  check states.
- `adr_0074_reports_do_not_capture_instruction_scrolling`: normal/narrow bounds,
  wheel access to the last instruction, independent report scrolling and retained
  instruction position after using the single toggle in both directions.
- `adr_0074_short_windows_reserve_space_for_task_content`: complete Settings
  and recovery navigation at 382 pixels high, normal/narrow widths and normal/XL
  scale, including Instructions and Report, the Settings title and a toggle
  contained within the content column.
- `adr_0074_page_menu_supports_keyboard_and_pointer`: menu opening, Escape,
  keyboard selection, pointer selection and one callback per selection.
- `adr_0074_diagnostics_selects_one_page_without_edit_effects`: one mounted tool,
  retained view selection and navigation without edit effects.
- `adr_0074_database_navigation_retains_inputs_without_starting_work`: retained
  paths/reports, unchanged command generation and repair admission.
- `adr_0074_repair_failure_reports_only_verified_recovery_steps`: repair retry
  guidance requires verified rollback and never promises to open an interrupted
  database.
- `adr_0074_reports_are_separate_from_instruction_scrolling`: shared viewport,
  typography and non-scrolling report ancestors.

The short-window follow-up retains the existing focus, editor Escape,
navigation ownership and recorded-feedback guards. Their source boundaries now
match the shared focus setup, menu header and Startup feedback placement.

Local Markdown links, anchors and `git diff --check` are Green.
The [focused operator procedure](../runbooks/startup-recovery-check.md#repair-and-diagnostics-pages--adr-0074)
remains a regression procedure. The two acceptance fixtures are removed.
The visual and preservation gates are accepted and cleanup is confirmed.

Documentation inventory: created this correction's ADR under `docs/adr`.
Updated this packet, the operator procedure, decision indexes, source map,
acceptance indexes and current-work guidance. No documents moved. No folders
were created. Root `AGENTS.md` remains the existing workflow owner.

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

Follow [task 013 V1–V3](../runbooks/startup-recovery-check.md#task-013-interrupted-upgrade-repair).
The runbook supplies complete commands, fixture states, expected results and
cleanup. V1 covers recognition, failed installation and explicit successful
repair with preserved selection; V2 covers older-backup candidate preparation;
V3 covers unsupported-schema refusal. V1–V3, normal/narrow presentation, complete
report copy, same-window resumption and both final preservation inspections are
accepted. Fixture cleanup is confirmed. No accepted visual or
behavior check needs repeating.

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
