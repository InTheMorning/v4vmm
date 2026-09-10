# ADR 0066 Task 001: Config Snapshot And Safe Persistence

Status: Ready - 2026-09-10.
Implementation not started. No new layout is specified.

## Goal

Read configuration once, distinguish core errors from optional errors, and prevent ordinary saves from destroying a document that needs repair.

## Read First And Dependency

Read [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md),
the [phase plan](../plans/adr-0066-startup-recovery-phase-plan.md), this whole packet, and the
[review checklist](../reviews/adr-0066-startup-recovery-review-checklist.md).
This is the next implementation packet.
Complete this packet in one session; do not start its successor.
Names marked **new**, including tests and guards, are implementation targets,
not claims that those files or symbols already exist. If a predecessor already
created a listed owner, extend that owner.

## Files To Inspect

- src/config.rs — Config, config_path, load_config, load_musicindex_endpoint, all three save functions
- src/app.rs — save_settings and persist_workspace_layout
- src/app/resize.rs — pane preference saves
- src/cli.rs — configured readers and first-run output
- src/theme_profile.rs; src/config.rs — UiScale
- docs/adr/0010-musicindex-endpoint-setting.md; docs/adr/0046-workspace-frame-architecture.md; docs/adr/0051-workspace-pane-width-persistence.md
- `tests/architecture_tests.rs`; `AGENTS.md`

## Files Likely To Change

- src/config.rs
- src/app.rs — persistence entry points only
- src/app/resize.rs — persistence error handling only
- tests/architecture_tests.rs
- This packet's Status/evidence, the phase plan and review checklist.
- ADR 0066's guard references/partial line and the delivery/deferred indexes as
  appropriate; `docs/pending-human-checks.md` when a runnable visual gate opens.
  Update `AGENTS.md` when the next executable packet changes.

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

## Implementation Steps

1. Introduce a ConfigSnapshot in config.rs containing the original bytes, core paths, independently decoded optional groups, and typed field issues. Use one TOML parse. Distinguish missing fields, explicit invalid fields, and a malformed whole table. Decode host selection, local producer, and encoder independently inside a readable broadcast table. An invalid sibling must not invalidate valid fields. Preserve current defaults for omitted optional settings.

   Keep individual field validation outcomes in the snapshot even when a core
   field is invalid. GUI startup aggregates both core requirements. A CLI
   operation requests only its own fields: an endpoint-only command does not
   need a valid music path, and a database-only query does not probe music
   storage. Invalid whole-document TOML still prevents reliable extraction.

2. Keep existing load_config callers compiling through a strict compatibility adapter over that snapshot; it must not silently manufacture an operational endpoint, player, or host for an invalid value. Task 004 moves the scoped callers off this adapter. load_musicindex_endpoint must use the same parsing policy and require only its own optional value from the parsed document. Task 004 removes the GUI's second startup read and command-specific rereads.

   Preserve the compatibility reader's existing malformed-layout fallback and
   its load_config_ignores_malformed_workspace_layout/_prefs tests. The snapshot
   records those issues even when that reader returns a layout default, so saves
   still refuse to overwrite the invalid values. The compatibility reader's
   existing rejection of unknown theme/player values stays until callers move
   to scoped access. Do not make it silently supply an operational substitute.

3. Separate config path resolution from directory creation. Inspect the directory entry with symlink_metadata: only NotFound on an absent entry permits first-run creation. A dangling symlink, directory, unreadable file, invalid UTF-8, or bad TOML is an error. Write and sync a complete validated sibling temporary file, then publish without replacement using a same-filesystem hard link. On AlreadyExists, read the winner. Remove only the owned temporary file; report any cleanup failure. First-run notices use stderr, never successful CLI JSON stdout.

4. Give all ordinary save paths a common fresh-read guard. Missing/unreadable/invalid TOML or invalid core fields reject the save. Optional errors reject autosaves and unrelated setting saves. A focused explicit correction may change only its declared fields, preserve unedited TOML values, validate those fields and the core, and retain other issues. Task 006 supplies the backup/conflict-protected correction command; do not expose an unprotected repair shortcut here.

5. Do not let save_settings immediately perform an unrelated layout save after an optional validation failure. Resume persistence only after a clean fresh validation. Normal database edits remain independent. Preserve current successful serialization behavior; no comment-preservation or concurrent merge feature.

6. Update superseded config tests by name: load_config_rejects_unknown_theme_profile and load_config_rejects_unknown_playback_driver keep asserting the strict adapter errors and gain snapshot tests proving scoped issues. save_workspace_layout_prefs_recovers_malformed_workspace_tables must instead assert ordinary save refusal and unchanged bytes; explicit correction is covered in task 006.

7. Review the mechanism handoff below against the ADR and every successor packet. Verify each has its own prose-retirement criterion. Record task 001's own guard references when complete; later guards are later packets' obligations.

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

Mechanical; assert at the named owner. Test/guard names below marked new must
be implemented by this packet.

| ID | Proof owner | Required assertion |
|---|---|---|
| C1 | config snapshot tests | Invalid UTF-8/TOML and wrong/missing core paths fail core decoding. Each optional field/table fails only its own group; valid siblings survive. Missing endpoint defaults; wrong type does not. |
| C2 | filesystem tests in config.rs | Absent first run succeeds; concurrent creator wins without overwrite; dangling symlink and permission failures never create defaults; failed publication/cleanup names residual paths. Existing bytes remain unchanged. |
| C3 | save tests for all three entry points | Missing/broken core and optional-error autosaves preserve bytes. Unrelated saves fail. Valid existing settings/layout saves retain their previous values and behavior. |
| C4 | snapshot test with injected reader | Change the backing file after one read; all fields in the returned snapshot still describe the first bytes. |
| C5 | new situational guard adr_0066_config_creation_and_save_ownership | Only first-run loading creates an absent config; all ordinary save paths use the guarded owner. Cite ADR 0066 invariants 3–4. |
| C6 | documentation review | The handoff assigns every mechanism to a named packet, preserves all accepted decisions, and leaves no successor without a prose-retirement criterion. |

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
```

New source assertions must preserve the rules of any guard they replace and name
ADR 0066 as their situational owner. Do not widen the lint gate or add a second
integration-test file.

## Operator Visual Check

No new visual gate is planned. The implementation report must say this explicitly.
If the implementation changes presentation, supply a focused manual check and
track it before acceptance; do not infer visual proof from backend tests.

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

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md`
- `docs/adr/0066-configuration-and-startup-failure-recovery.md`
- `docs/plans/adr-0066-startup-recovery-phase-plan.md`
- This packet in full, including Files To Inspect, implementation steps and criteria.

Goal:
- Read configuration once, distinguish core errors from optional errors, and prevent ordinary saves from destroying a document that needs repair.

Constraints:
- Follow this packet's Constraints and Implementation Steps.
- Preserve its data, dependency and prose-retirement contracts.
- One packet this session. Never run the app.

Do not touch:
- src/db.rs and database schema
- Playback, download, broadcast transport, and renderer layout
- Workspace TOML key names; config-format migration

Acceptance criteria:
- Prove every mechanical row in this packet at its named owner.
- Record actual guard references and keep trackers consistent.
- Do not claim a visual change or visual acceptance from these backend checks.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
