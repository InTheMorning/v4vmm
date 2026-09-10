# ADR 0066 Task 006: Configuration Repair And Resumption

Status: Ready after the preceding packet - 2026-09-10.
Implementation not started. Operator check specified below; not runnable or accepted yet.

## Goal

Repair configuration inside recovery or Settings, preserve the original file, and return to a freshly verified app session.

## Read First And Dependency

Read [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md),
the [phase plan](../plans/adr-0066-startup-recovery-phase-plan.md), this whole packet, and the
[review checklist](../reviews/adr-0066-startup-recovery-review-checklist.md).
Execute after [task 005](adr-0066-task-005-session-drain-and-resumption.md) in the phase plan's order.
Complete this packet in one session; do not start its successor.
Names marked **new**, including tests and guards, are implementation targets,
not claims that those files or symbols already exist. If a predecessor already
created a listed owner, extend that owner.

## Files To Inspect

- src/config.rs — snapshot, guarded writers and field validation
- src/startup.rs; src/view_models/startup.rs; src/app/startup.rs
- src/presentation/maintenance_executor.rs; src/presentation/startup_presenter.rs
- src/ui/composites/startup_report.rs; src/app.rs — render_settings/save_settings
- Session drain and resumption packet — managed transition out of normal work
- docs/troubleshooting/column-text-truncation.md
- `tests/architecture_tests.rs`; `AGENTS.md`

## Files Likely To Change

- src/config.rs — explicit correction backend
- src/application/commands/maintenance.rs (new); src/application/commands/mod.rs
- src/view_models/startup.rs; src/ui/composites/maintenance_forms.rs
- src/app/startup.rs; src/app.rs — shared editor composition and Settings route only
- src/presentation/startup_presenter.rs; src/ui/composites/mod.rs
- tests/architecture_tests.rs; docs/runbooks/startup-recovery-fixture.py; docs/runbooks/startup-recovery-check.md
- This packet's Status/evidence, the phase plan and review checklist.
- ADR 0066's guard references/partial line and the delivery/deferred indexes as
  appropriate; `docs/pending-human-checks.md` when a runnable visual gate opens.
  Update `AGENTS.md` when the next executable packet changes.

## Do Not Touch

- Database content replacement or music-file moves
- Automatic repair/default reset, configuration key renaming, comment-preserving merge
- New editors independently duplicated in Settings and recovery
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

1. Add explicit document/field correction commands separate from ordinary saves. Retain original bytes and a file identity/content revision at edit start. For readable invalid TOML, offer a raw editor with error location. For readable valid TOML, offer focused fields. An unreadable document has a report and Check again; never seed its editor with defaults.

2. Validate proposed TOML/core and each edited optional field before any write. On conflict with a changed/deleted/replaced file, return a conflict and retain the proposed draft for copy/reload; never merge automatically. Save the exact original bytes to an exclusively created, separately named backup with owner-only permissions, sync it, then publish a complete sibling candidate. Recheck the source revision immediately before replacement. Backup failure prevents replacement; report paths for any residual owned artifacts. Do not echo raw rejected values or source excerpts into diagnostics.

3. For a symlink config, show both link and resolved destination, retain that identity, and correct the same target only if neither identity nor content changed. Preserve the link. Refuse dangling/changing links with a named next action. Ordinary autosaves remain guarded; this explicit operation never turns into a generic force-write.

4. Add music-folder and database-path tools that test the proposed existing location before save. Recovery first-run setup remains a separate explicit preparation flow; choosing another database must not create an empty replacement. Do not move music. A core path correction uses the managed session transition from the drain packet; it never retargets a live Connection or player.

5. Share one view model and maintenance_forms composite between Settings and core recovery. Editing/selecting alone does not save; Save alone does not retry. Show original backup path and result after an explicit save. Core edits lead to Check again and Open app; optional edits leave remaining issues visible and are reinitialized by task 007.

6. Use worker generations to reject old validation results and typed single-flight save/check actions. Preserve reports across managed resumption. Fresh clean config validation permits autosaves again; another optional issue keeps them paused.

7. Extend fixture/runbook for malformed TOML corrected in the app, invalid core paths, unreadable document, failed backup, a concurrent editor and multiple invalid optional fields.

## Acceptance Criteria

Mechanical; assert at the named owner. Test/guard names below marked new must
be implemented by this packet.

| ID | Proof owner | Required assertion |
|---|---|---|
| C1 | config correction tests | Rejected draft, failed backup, unreadable source, concurrent changed/deleted source and symlink change cannot overwrite original data. Successful correction preserves the exact original in a named backup and unedited TOML values. |
| C2 | VM/command tests | Editing is inert; explicit Save validates and writes once; Save never repeats the original action. Other optional issues survive a focused correction; autosaves resume only after a clean fresh read. |
| C3 | path-tool tests | Proposed existing music/database paths are tested; neither music relocation nor empty replacement database occurs. An invalid path cannot become active. |
| C4 | lifecycle tests | Core edits drain the previous session, close old handles, check fresh resources and mount one new session only on Open app. Stale completions do not reopen old paths. |
| C5 | new situational guard adr_0066_shared_guarded_config_repair | Settings and core recovery use the same correction commands and composites; ordinary save guards cannot be bypassed. Cite invariants 3–6 and 9. |

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

1. In invalid-toml recovery, correct the fixture in the editor and save. The app must name the preserved original. Check again, then Open app; no terminal edit or relaunch should be needed.

2. In Settings, correct one of two bad optional fields. Confirm the other issue remains and its value is preserved. Correct it and verify ordinary persistence resumes.

3. While an editor is open, use the fixture's conflict command. Save must report that the file changed and preserve both the external file and your draft. Inspect backups/checksums and clean up.

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

A focused save cannot preserve unedited values, a path edit would require silently migrating files, or a core correction would reopen beneath existing handles. Keep the draft and report the named conflict.
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
- Repair configuration inside recovery or Settings, preserve the original file, and return to a freshly verified app session.

Constraints:
- Follow this packet's Constraints and Implementation Steps.
- Preserve its data, dependency and prose-retirement contracts.
- One packet this session. Never run the app.

Do not touch:
- Database content replacement or music-file moves
- Automatic repair/default reset, configuration key renaming, comment-preserving merge
- New editors independently duplicated in Settings and recovery

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

