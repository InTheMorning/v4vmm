# ADR 0066 Task 003: Runtime Failure And Shell Availability

Status: Ready after the preceding packet - 2026-09-10.
Implementation not started. Operator check specified below; not runnable or accepted yet.

## Goal

Keep navigation, reports and repair access working when the normal background runtime or optional thumbnail worker cannot start.

## Read First And Dependency

Read [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md),
the [phase plan](../plans/adr-0066-startup-recovery-phase-plan.md), this whole packet, and the
[review checklist](../reviews/adr-0066-startup-recovery-review-checklist.md).
Execute after [task 002](adr-0066-task-002-core-checks-and-startup-reports.md) in the phase plan's order.
Complete this packet in one session; do not start its successor.
Names marked **new**, including tests and guards, are implementation targets,
not claims that those files or symbols already exist. If a predecessor already
created a listed owner, extend that owner.

## Files To Inspect

- src/app.rs — TopApp::new, command_runner and runtime_host
- src/library.rs; src/library/app_impl.rs — constructors, command_runner and paged actors
- src/application/async_command_runner.rs — constructors and dispatch
- src/presentation/async_command_presenter.rs; src/presentation/runtime_host.rs
- src/app/show.rs; src/app/search_dispatch.rs; src/app/keyboard.rs
- src/media/image_cache.rs — with_capacity startup eviction
- src/runtime/mod.rs; src/presentation/mod.rs
- Task 002 startup/maintenance/report owners
- `tests/architecture_tests.rs`; `AGENTS.md`

## Files Likely To Change

- src/application/async_command_runner.rs; src/application/errors/command.rs
- src/presentation/async_command_presenter.rs; src/presentation/runtime_host.rs
- src/app.rs; src/app/bootstrap.rs; src/library.rs; src/library/app_impl.rs
- src/app/show.rs; src/app/search_dispatch.rs; src/app/keyboard.rs — dependency dispatch only
- src/startup.rs; src/view_models/startup.rs; src/ui/composites/startup_report.rs
- src/media/image_cache.rs
- src/startup/fixture.rs; src/app/startup.rs — debug-only runtime/cache factory failure injection
- tests/architecture_tests.rs; docs/runbooks/startup-recovery-fixture.py; docs/runbooks/startup-recovery-check.md
- This packet's Status/evidence, the phase plan and review checklist.
- ADR 0066's guard references/partial line and the delivery/deferred indexes as
  appropriate; `docs/pending-human-checks.md` when a runnable visual gate opens.
  Update `AGENTS.md` when the next executable packet changes.

## Do Not Touch

- Tokio replacement, a second normal runtime, synchronous execution on the UI thread
- Playback semantics, config editing, SQLite replacement or Show layout
- Runtime actors' domain algorithms
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

1. Represent the GUI command runner as an explicitly available/unavailable runner, with a typed reason and no Handle in its unavailable state. Its dispatch must complete with a typed unavailable error without invoking CommandBus or emitting a success event. Keep explicit-runtime constructors; do not fall through to new/with_vm_bus, both of which use Handle::current.

2. Replace both None branches in TopApp::new and LibraryApp::new_with_content_view_mode. Construct the shell and report/maintenance routes without RuntimeHost. Skip runtime actors and sagas when unavailable; never replace them with blocking UI-thread work. Preserve existing successful handles and actor ownership.

3. Add a renderer-free capability issue collection keyed by dependency, with recorded observations and typed Configure/Check routes. Reuse the report composite for a concise normal-app notice and an issue list in Settings. Multiple failures remain independent; no one settings_status string or transient toast is the sole owner.

4. Project runtime availability at common dispatch/projection boundaries. Inventory all present_command call sites and direct runtime-host actor starts; library, search, playback and Show actions that need the runtime must carry an enabled repair/report route. Navigation, copying and independent maintenance remain usable. Verify toolbar and keyboard routes cannot bypass dispatch rejection. Do not blanket-disable local functionality that has an existing independent execution path.

5. Retry runtime creation through task 002's independent worker. Install one successful host/runner set, subscribe child bridges once, and start only permitted actors. A failed retry replaces the runtime issue, not unrelated issues. Saving settings does not retry automatically; task 007 adds original-action return.

6. Make ImageCache's eviction thread launch fallible using thread::Builder; continue available cache behavior after launch/prune failure and add a scoped issue. Do not propagate eviction failure as a core failure. Preserve independent cache capacity and thumbnail behavior.

7. Extend the fixture with injected runtime-unavailable and cache-worker-unavailable cases. Failure injection belongs to test/debug fixture seams; do not create undocumented release environment toggles or exhaust machine resources.

## Acceptance Criteria

Mechanical; assert at the named owner. Test/guard names below marked new must
be implemented by this packet.

| ID | Proof owner | Required assertion |
|---|---|---|
| C1 | runner and constructor tests | With no Tokio context and a failed RuntimeHost factory, unavailable runner construction/dispatch does not panic, execute a command, emit success or spawn a fallback runtime. |
| C2 | view-model/command tests | Navigation/report/maintenance actions remain enabled. Every runtime-dependent entry point, including toolbar and keyboard, rejects execution and offers a typed remedy. |
| C3 | issue-list tests | Simultaneous runtime/cache issues survive navigation; resolving one leaves the other. No adapter construction is projected as a successful remote observation. |
| C4 | retry lifecycle tests | A successful explicit runtime retry installs exactly one host/runner/bridge/actor set; repeated clicks and stale completions cannot duplicate it. |
| C5 | ImageCache tests with injected worker/prune failures | Failure is returned as a scoped issue while usable cache behavior remains. No panic or library reset occurs. |
| C6 | new situational guard adr_0066_missing_runtime_has_no_implicit_runner | Both GUI composition roots use explicit availability; missing runtime cannot reach Handle::current or UI-thread work. Preserve ADR 0040 runtime/GPUI separation guards. |

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

1. Launch the runtime-unavailable fixture. Navigate Music, Show and Settings. Open and copy the issue report. A dependent action must explain the limitation and offer Check again.

2. Allow the fixture runtime factory to succeed, then use Check again. The issue clears and the action becomes usable without duplicate rows or a second app window.

3. Launch the cache-worker-unavailable case. The app must remain usable and the issue must name thumbnail maintenance. Clean up the disposable fixture.

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

An independent repair operation still requires RuntimeHost, or preserving a supported local action requires executing I/O in a renderer. Name that dependency instead of adding an implicit runtime.
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
- Keep navigation, reports and repair access working when the normal background runtime or optional thumbnail worker cannot start.

Constraints:
- Follow this packet's Constraints and Implementation Steps.
- Preserve its data, dependency and prose-retirement contracts.
- One packet this session. Never run the app.

Do not touch:
- Tokio replacement, a second normal runtime, synchronous execution on the UI thread
- Playback semantics, config editing, SQLite replacement or Show layout
- Runtime actors' domain algorithms

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
