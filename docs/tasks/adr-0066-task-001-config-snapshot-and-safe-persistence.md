# ADR 0066 Task 001: Config Snapshot And Safe Persistence

Status: Complete - 2026-09-10.
Mechanical gate Green. No new layout or operator visual gate.
Task 002 is next; it was not started in this session.

## Goal

Read configuration once, distinguish core errors from optional errors, and prevent ordinary saves from destroying a document that needs repair.

## Read First And Dependency

Read [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md),
the [phase plan](../plans/adr-0066-startup-recovery-phase-plan.md), this whole packet, and the
[review checklist](../reviews/adr-0066-startup-recovery-review-checklist.md).
This packet is complete. Its guard references below own the implemented
mechanics; the handoff table still assigns the remaining work.

## Files To Inspect

- src/config.rs — Config, config_path, load_config, load_musicindex_endpoint, all three save functions
- src/app.rs — save_settings and persist_workspace_layout
- src/app/resize.rs — pane preference saves
- src/cli.rs — configured readers and first-run output
- src/theme_profile.rs; src/config.rs — UiScale
- docs/adr/0010-musicindex-endpoint-setting.md; docs/adr/0046-workspace-frame-architecture.md; docs/adr/0051-workspace-pane-width-persistence.md
- `tests/architecture_tests.rs`; `AGENTS.md`

## Files Changed

- src/config.rs
- src/app.rs — persistence entry points only
- tests/architecture_tests.rs
- This packet, the phase plan, review checklist, ADR 0066 and its index,
  docs/README.md, delivery/deferred indexes, and AGENTS.md.
- Existing pending human checks are unchanged; this packet adds no visual gate.

## Do Not Touch

- src/db.rs and database schema
- Playback, download, broadcast transport, and renderer layout
- Workspace TOML key names; config-format migration
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

Implementation procedures are retired in favor of the following live owners
and guards. Binding decisions remain in ADR 0066.

| Responsibility | Implemented owner |
|---|---|
| One parsed document and independent field outcomes | `ConfigSnapshot`, `ConfigFieldIssue`, `ConfigIssueKind` in [config.rs](../../src/config.rs) |
| Existing readers and temporary strict behavior | `ConfigSnapshot::legacy_config`, `load_config`, `load_musicindex_endpoint` |
| First-run creation | `load_config_snapshot`, `load_snapshot_with_defaults`, `publish_default_config` |
| All ordinary saves | `read_config_for_save`, `write_existing_config`; `TopApp::save_settings` reloads through `ConfigSnapshot::read_existing` |
| Ownership/routing | Situational `adr_0066_config_creation_and_save_ownership` in [architecture_tests.rs](../../tests/architecture_tests.rs), ADR 0066 invariants 3–4 |

The old whole-Config/workspace/playback serde adapters were removed after tracing
all readers to the snapshot. `workspace_frame_phase_5_layout_persistence_contract`
and `workspace_pane_width_persistence_contract` now name the snapshot and legacy
layout adapter; their ADR 0046/0051 fallback and persistence rules remain.
`load_config_rejects_unknown_theme_profile` and
`load_config_rejects_unknown_playback_driver` retain strict rejection without
printing rejected values. The superseded ordinary-save recovery assertion is now
`save_workspace_layout_prefs_rejects_malformed_workspace_tables`.

Task 004 still owns the GUI's single startup snapshot and scoped command/query
consumers. The strict compatibility reader still rejects invalid operational
settings. Task 006 owns explicit correction with preservation and conflict
checks. Ordinary saves provide neither repair nor concurrent edit merging.
Startup recovery remains unimplemented until task 002 and its follow-through.

## Mechanical Evidence

Green - 2026-09-10:

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test --quiet`: 1,275 unit tests and 221 architecture tests passed;
  10 documentation examples remain ignored by the existing suite.
- `cargo clippy --quiet -- -D warnings`

The focused configuration suite contains 49 passing tests. `src/app/resize.rs`
already delegates to the common save owner and reports failures; it needed no
edit. No database, playback, transport or renderer implementation changed.

## Mechanism Handoff

Packet authoring has moved the startup-stage procedures and detailed verification
out of the ADR. This table owns the remaining implementation handoff. It is not
a second copy of those instructions.

| Mechanism previously described in the ADR | Packet owner |
|---|---|
| Snapshot, first-run publication, ordinary persistence guard | [001](adr-0066-task-001-config-snapshot-and-safe-persistence.md) |
| Music/SQLite checks, startup-stage classification, safe UTC reports, window lifecycle and independent worker | [002](adr-0066-task-002-core-checks-and-startup-reports.md) |
| Missing runtime, dispatch guard, optional thumbnail worker, common issue notice | [003](adr-0066-task-003-runtime-failure-and-shell-availability.md) |
| Optional config consumers, driver/producer/encoder construction, command/query dependencies, failed path-repair containment | [004](adr-0066-task-004-optional-tool-isolation.md) |
| Internal command/actor drain, connection closure, managed resumption | [005](adr-0066-task-005-session-drain-and-resumption.md) |
| Configuration preservation, explicit correction, editor conflict, Check again/Open app | [006](adr-0066-task-006-configuration-repair-and-resumption.md) |
| Capability reinitialization, original-action retention and explicit retry | [007](adr-0066-task-007-optional-tool-correction-and-retry.md) |
| Fresh FLAC/ffmpeg probes and converter setup | [008](adr-0066-task-008-converter-verification-and-setup.md) |
| Staging ownership, conversion outcome and same-track retry | [009](adr-0066-task-009-conversion-retry-and-retained-input.md) |
| Inspection and verified SQLite snapshots | [010](adr-0066-task-010-database-check-and-backup.md) |
| Exclusive database access and preservation copies after 005's session drain | [011](adr-0066-task-011-database-maintenance-and-preservation.md) |
| Candidate validation and controlled installation | [012](adr-0066-task-012-database-restore.md) |
| Recognized upgrade recipe, shared migration ledger and final ADR reconciliation | [013](adr-0066-task-013-interrupted-upgrade-repair.md) |

Binding product contracts remain in the ADR. Do not delete an invariant merely
because a test now proves it. Delete duplicated procedure prose when the named
guard lands, and keep its symbol plus the verification artifact in that packet.
Task 001 can close after its own work and this handoff review; it does not wait
for tasks 002–013.

## Acceptance Criteria

Mechanical evidence lives in [config.rs tests](../../src/config.rs) unless
specified otherwise. The table records the actual proof for each criterion.

| ID | Passing evidence |
|---|---|
| C1 | `adr_0066_snapshot_defaults_and_required_fields_are_distinct`, `adr_0066_optional_fields_fail_without_poisoning_core_paths`, `adr_0066_malformed_optional_tables_keep_other_groups_available`, `adr_0066_readable_tables_preserve_valid_sibling_fields`, `adr_0066_endpoint_reader_requires_only_its_own_field`; safe errors: `adr_0066_config_diagnostics_exclude_rejected_values_and_source_bytes` |
| C2 | `adr_0066_first_run_publishes_only_a_valid_complete_document`, `adr_0066_first_run_reads_the_competing_creators_document`, `adr_0066_concurrent_first_run_writers_do_not_clobber`, `adr_0066_symlinks_and_unreadable_entries_never_invoke_defaults`, `adr_0066_existing_bad_bytes_and_read_denial_do_not_create_defaults`, `adr_0066_permission_denial_preserves_config_across_load_and_saves`, `adr_0066_failed_default_writes_and_publication_clean_owned_temporaries`, `adr_0066_failed_default_cleanup_reports_the_remaining_path` |
| C3 | `adr_0066_all_ordinary_saves_preserve_broken_documents`, `adr_0066_saves_never_recreate_a_missing_document`, `adr_0066_saves_resume_after_a_fresh_valid_document`, `save_workspace_layout_prefs_rejects_malformed_workspace_tables`; existing settings/layout round-trip tests stay Green |
| C4 | `adr_0066_snapshot_keeps_one_read_even_if_the_file_changes` |
| C5 | `adr_0066_config_creation_and_save_ownership` in [architecture_tests.rs](../../tests/architecture_tests.rs), situational ADR 0066 invariants 3–4 |
| C6 | [Task 001 handoff review](../reviews/adr-0066-startup-recovery-review-checklist.md#task-001-review--2026-09-10): every mechanism has a packet owner; 002–013 each retain their own prose-retirement criterion |

## Test Commands

Run the focused owner tests while editing, then the repository gate once:

```bash
cargo fmt -- --check
cargo check --quiet
cargo test --quiet
cargo clippy --quiet -- -D warnings
```

New source assertions must preserve the rules of any guard they replace and name
ADR 0066 as their situational owner. Do not widen the lint gate or add a second
integration-test file.

## Operator Visual Check

No new operator visual check applies: this packet changes backend decoding,
file persistence and safe diagnostics, with no renderer, view-model or layout
change. The app was not launched. Existing human checks remain open unchanged.

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

A caller requires silently defaulting an invalid optional group, a save cannot preserve unedited values, or first-run publication cannot distinguish an absent entry from an unreadable one. Name the failing contract; do not weaken it.
Routine placement inside the named owner is authorized. If a boundary needs a
new architectural decision, name the conflict and proposed bounded correction
before widening this packet.
