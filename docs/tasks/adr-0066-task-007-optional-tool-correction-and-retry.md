# ADR 0066 Task 007: Optional Tool Correction And Retry

Status: Ready after the preceding packet - 2026-09-10.
Implementation not started. Operator check specified below; not runnable or accepted yet.

## Goal

Turn optional-tool failures into a direct Settings correction route and an explicit, freshly checked retry of the original action.

## Read First And Dependency

Read [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md),
the [phase plan](../plans/adr-0066-startup-recovery-phase-plan.md), this whole packet, and the
[review checklist](../reviews/adr-0066-startup-recovery-review-checklist.md).
Execute after [task 006](adr-0066-task-006-configuration-repair-and-resumption.md) in the phase plan's order.
Complete this packet in one session; do not start its successor.
Names marked **new**, including tests and guards, are implementation targets,
not claims that those files or symbols already exist. If a predecessor already
created a listed owner, extend that owner.

## Files To Inspect

- src/view_models/startup.rs; src/application/commands/maintenance.rs; src/config.rs
- src/app.rs; src/app/show.rs; src/app/search_dispatch.rs; src/app/keyboard.rs
- src/library.rs; src/library/app_impl.rs
- src/view_models/show.rs; src/view_models/app_toolbar.rs; src/view_models/library.rs
- src/playback_driver/mod.rs; src/playback_owner.rs; src/broadcast/producer.rs
- src/presentation/runtime_host.rs; src/presentation/startup_presenter.rs
- `tests/architecture_tests.rs`; `AGENTS.md`

## Files Likely To Change

- src/view_models/startup.rs; src/application/commands/maintenance.rs
- src/application/capability_recovery.rs (new); src/application/mod.rs
- src/app.rs; src/app/show.rs; src/app/search_dispatch.rs; src/app/keyboard.rs; src/library/app_impl.rs — remediation routing only
- src/view_models/show.rs; src/view_models/app_toolbar.rs; src/view_models/library.rs — remediation intents only
- src/ui/composites/maintenance_forms.rs; src/presentation/startup_presenter.rs
- src/startup.rs; src/playback_owner.rs — targeted reinitialization only
- tests/architecture_tests.rs; docs/runbooks/startup-recovery-fixture.py; docs/runbooks/startup-recovery-check.md
- This packet's Status/evidence, the phase plan and review checklist.
- ADR 0066's guard references/partial line and the delivery/deferred indexes as
  appropriate; `docs/pending-human-checks.md` when a runnable visual gate opens.
  Update `AGENTS.md` when the next executable packet changes.

## Do Not Touch

- Converter process behavior and staging retention (008/009)
- External service auto-restarts, package installation, or a new service-state taxonomy
- Core database replacement; duplicated Settings tool surfaces
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

1. Define a renderer-free RecoveryIntent containing action kind, dependency, original subject identifiers and immutable inputs, plus its originating session/config generation. Keep payloads typed; do not store an arbitrary command closure. For broadcast actions retain event identity and publisher context; for searches retain query/scope; playback retains session/track. These are session objects, not new DB tables.

2. Map dependency issues to focused tools: endpoint URL, playback settings/runtime directory, publisher host/selection, local producer directory/target, encoder fields, presentation values, or explicit runtime Check again. Use task 006's guarded editor for supported config fields; a focused TOML block editor may cover structured host/encoder settings. Do not invent TOML keys.

3. Use the same typed route for buttons, keyboard and toolbar. Opening setup retains the intended subject even if the current UI selection changes. A blocked execution does no work. Keep the operation report with the action's actual consequence, rather than labelling a warning as total failure.

4. Save changes explicitly, then reinitialize only the corrected capability. Probe configuration/setup without claiming remote success; remote Check uses the existing observation path. Keep persistent issues until the relevant fresh result resolves them. Failed retries must not erase other issues.

5. Offer explicit Retry after verification, or a separately labelled Save and retry. Revalidate original subject existence, current availability, session/config generation, and publisher target/event safety at command execution. If the original operation no longer makes sense, report that fact; never retarget a newer selection. Saving alone cannot publish, download, attach or restart anything.

6. Update the named action adapters instead of adding a parallel dispatch pipeline. Preserve ADR 0059 command-boundary event selection/target revalidation and task 017 passive-check readiness semantics. Runtime recovery reuses 003, with no second background runtime.

## Acceptance Criteria

Mechanical; assert at the named owner. Test/guard names below marked new must
be implemented by this packet.

| ID | Proof owner | Required assertion |
|---|---|---|
| C1 | RecoveryIntent/VM tests | Each scoped failure has an enabled correction/check intent, original subject and inputs. Multiple pending subjects cannot overwrite one another silently. |
| C2 | command tests | Save has no original-operation side effect. Explicit Retry requires a fresh successful check and revalidates identity/context; removed tracks/events and changed publisher contexts are rejected without retargeting. |
| C3 | adapter tests | Only the corrected capability is reinitialized; unrelated player, publisher and runtime owners retain their identity. Failed checks preserve the issue and truthful observation state. |
| C4 | entry-point tests | Toolbar, keyboard and button routes produce the same remediation and retry behavior. |
| C5 | new situational guard adr_0066_repair_routes_preserve_action_subject | Application/VM owners carry intent and availability; renderers cannot replay a saved action or invent enablement. Cite invariant 9. |

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

1. Attempt an Index request with the fixture's bad endpoint. Open its repair tool, correct the URL and save. Nothing should rerun until Retry; Retry must use the original search.

2. Repeat with failed playback setup and one publisher operation. Change the selection while repairing. The app must identify the original action and reject an obsolete subject rather than act on the new selection.

3. Confirm unrelated controls remain available while setup is open and no external service restarts merely because Settings was saved. Clean up.

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

An operation needs new durable retry storage or cannot name/revalidate its original subject. Do not replace the typed intent with a captured UI closure or silently replay a command.
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
- Turn optional-tool failures into a direct Settings correction route and an explicit, freshly checked retry of the original action.

Constraints:
- Follow this packet's Constraints and Implementation Steps.
- Preserve its data, dependency and prose-retirement contracts.
- One packet this session. Never run the app.

Do not touch:
- Converter process behavior and staging retention (008/009)
- External service auto-restarts, package installation, or a new service-state taxonomy
- Core database replacement; duplicated Settings tool surfaces

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

