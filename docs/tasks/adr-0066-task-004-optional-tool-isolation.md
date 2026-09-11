# ADR 0066 Task 004: Optional Tool Isolation

Status: Ready - 2026-09-11; task 003 prerequisite complete, including operator acceptance and fixture cleanup.
Implementation not started. Operator check specified below; not runnable or accepted yet.

## Goal

Open the app with valid core resources even when optional configuration or tool preparation fails, and limit only the operations that actually depend on each failure.

## Read First And Dependency

Read [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md),
the [phase plan](../plans/adr-0066-startup-recovery-phase-plan.md), this whole packet, and the
[review checklist](../reviews/adr-0066-startup-recovery-review-checklist.md).
Execute after [task 003](adr-0066-task-003-runtime-failure-and-shell-availability.md) in the phase plan's order.
Complete this packet in one session; do not start its successor.
Names marked **new**, including tests and guards, are implementation targets,
not claims that those files or symbols already exist. If a predecessor already
created a listed owner, extend that owner.

## Files To Inspect

- src/config.rs — ConfigSnapshot and strict compatibility adapter from 001
- src/app/bootstrap.rs; src/app.rs — endpoint, playback_owner, broadcast config
- src/library.rs; src/library/app_impl.rs — endpoint and paged actor creation
- src/app/search_dispatch.rs; src/app/queue_now_playing.rs; src/app/playback_bar.rs; src/app/show.rs; src/app/keyboard.rs
- src/application/commands/search.rs; src/application/queries/broadcast.rs; src/application/commands/playback.rs
- src/cli.rs; src/feed_service.rs; src/subscribe_service.rs; src/library_service.rs
- src/rss/subscribe.rs — RSS ingestion and optional MusicIndex enrichment
- src/playback_driver/mod.rs; src/playback_owner.rs; src/broadcast/producer.rs
- src/db.rs; src/library_path.rs; src/view_models/show.rs; src/view_models/app_toolbar.rs
- `tests/architecture_tests.rs`; `AGENTS.md`

## Files Likely To Change

- src/config.rs; src/startup.rs; src/view_models/startup.rs
- src/app/bootstrap.rs; src/app.rs; src/library.rs; src/library/app_impl.rs
- src/app/search_dispatch.rs; src/app/queue_now_playing.rs; src/app/playback_bar.rs; src/app/show.rs; src/app/keyboard.rs — resource availability/wiring only
- src/view_models/app_toolbar.rs; src/view_models/show.rs; src/view_models/library.rs — typed dependency projections only
- src/application/commands/search.rs; src/application/queries/broadcast.rs; src/application/commands/playback.rs — boundary guards only
- src/cli.rs; src/feed_service.rs; src/subscribe_service.rs; src/library_service.rs — scoped config readers only
- src/rss/subscribe.rs — preserve known-RSS ingestion when optional Index enrichment is unavailable
- src/playback_driver/mod.rs; src/playback_owner.rs; src/broadcast/producer.rs — preparation/availability adapters only
- src/db.rs; src/library_path.rs — failed-repair containment only
- tests/architecture_tests.rs; docs/runbooks/startup-recovery-fixture.py; docs/runbooks/startup-recovery-check.md
- src/broadcast/registry.rs — separate local registry operations from endpoint-dependent operations
- This packet's Status/evidence, the phase plan and review checklist.
- ADR 0066's guard references/partial line and the delivery/deferred indexes as
  appropriate; `docs/pending-human-checks.md` when a runnable visual gate opens.
  Update `AGENTS.md` when the next executable packet changes.

## Do Not Touch

- New service-state vocabulary, seventh ServiceState or remote protocol changes
- ADR 0064 repair-history surface, config-format migration or decoder changes beyond scoped adapters
- Playback timing, queue semantics, broad show.rs decomposition
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

1. Consume task 001's one snapshot throughout startup. Remove the second load_musicindex_endpoint read. Carry capability values as typed available/unavailable resources; never populate a valid-looking endpoint, host or player from an invalid configured value. Missing optional values keep documented defaults. Presentation defaults remain usable with persistent issues and autosaves paused.

2. Keep playback_owner absent when driver setup fails, carrying its configured choice and safe issue. Check the owner before constructing playback commands, polling, queue actions and shutdown cleanup. Explicit/default Null is still a real configured driver; failed mpv never becomes Null. Do not launch or ping mpv during startup.

3. Prepare publisher host selection, DropFileProducer and encoder independently. A bad producer directory/target must not block audio playback or external publisher controls. A bad host list must not block a valid local producer/encoder. Deliberately absent drop_directory is not a failure. Preserve task 017's event selection, attachment and badge rules.

4. Implement the dependency inventory below at both query and command boundaries. Route setup issues to task 003's notice and Settings list, plus affected feature status. Availability means a dependency can be attempted; URL syntax and adapter construction never imply remote reachability. Optional outages never promote themselves to core recovery.

5. Replace operation-time whole-Config reads in the named service/CLI owners with scoped snapshot access. Each CLI invocation shares a snapshot across its helpers; GUI commands receive current scoped values or a fresh scoped read at their boundary. Preserve direct-RSS and local registry list/forget operations without requiring MusicIndex solely because BroadcastRegistry::new takes an endpoint. Split local and network-dependent registry construction/use as needed; include src/broadcast/registry.rs only for that adapter change.

6. Handle repair_local_file_paths errors after possible partial updates. Recheck core database/music usability. If either fails, enter recovery. Otherwise report uncertainty and let already valid relative bindings work; remaining absolute/unvalidated bindings cannot drive file operations. Retain LibraryRelativePath validation and ADR 0064 skip/binding contracts. Do not call repair again from Check again or create a repair history UI here.

7. Update source assertions that assumed mandatory resources while keeping their rule. Existing driver-null/default/absent-producer, path-repair skip, task 017 configured-target and passive-check tests remain. Inventory production config/runner/playback callers with rg before marking this packet complete; do not leave one secondary reader turning an unrelated optional error back into a global failure. Apply C7 to ConfigSnapshot::legacy_config and the load_config wrapper; migrating most callers is not completion.

## Command And Query Dependency Inventory

| Boundary | Required fact | Independent work to preserve |
|---|---|---|
| Toolbar search / search commands / Index detail requests | Valid MusicIndex endpoint and runtime; remote result is a separate observation | Local library and explicit RSS URL operations |
| Local registry list/forget/selection | Core database, token-path rules for forget | No remote endpoint needed just to read stored events |
| Event Create/Check | Create needs current endpoint; Check uses selected event's saved endpoint, plus execution runtime | Local selection and saved reports |
| Publisher service/target queries and mutations | Selected publisher host and configured target where the command uses it | Local playback/producer and independent encoder |
| Encoder commands | Valid encoder settings/transport and execution runtime | Publisher and music work |
| Built-in playback/polling | Prepared configured playback owner and runtime for async work | External broadcast controls and library browsing |
| Drop-file publication | Valid producer settings; publication I/O has its own result | Built-in audio playback |
| Feed/subscription/library services | Core paths plus only the optional features actually invoked | Known RSS operations without unrelated Index refresh |
| Downloads beneath artists | Usable destination subtree; converter need is decided by actual format policy | Existing valid tracks elsewhere |
| Theme/scale/layout projection and saves | Presentation fallback can render; persistence requires clean config or explicit correction | Database-only operations |
| File actions after failed path repair | Validated relative binding under verified music root | Independent validated bindings |

The common runtime guard from 003 applies only to entries that use that runner.
A MusicIndex outage is not a reason to disable already available RSS inputs.

## Acceptance Criteria

Mechanical; assert at the named owner. Test/guard names below marked new must
be implemented by this packet.

| ID | Proof owner | Required assertion |
|---|---|---|
| C1 | startup factory tests | Each invalid optional group and each preparation failure still mounts normal operation with valid core resources. Malformed whole tables and invalid sibling fields have the specified scope. |
| C2 | paired VM and dispatch tests | MusicIndex failure/local library, bad host/valid producer, bad producer/audio playback, bad player/external control and unavailable runtime/maintenance demonstrate independent availability. Keyboard and toolbar cannot bypass it. |
| C3 | CLI/service tests | An invalid unused core or optional field does not fail an independent CLI operation. Known-RSS operations do not require unrelated Index enrichment. Local registry list/forget do not require an endpoint. JSON success stays on stdout; errors/notices use stderr. |
| C4 | failure-injected path repair tests | An error after an applied statement preserves that change and every remaining binding; valid independent files remain usable and unvalidated legacy paths cannot execute. Loss of core usability enters recovery. No rollback claim is emitted. |
| C5 | existing regression tests | Explicit/default Null, lazy mpv, absent producer, both LocalPathRepairSkip results, LibraryRelativePath safety and task 017 selection/attachment/readiness remain correct. |
| C6 | new situational guards adr_0066_optional_dependencies_are_scoped and adr_0066_repair_failure_preserves_bindings | No silent resource substitution, optional-to-core promotion or bypass around scoped validation; preserve ADR 0064 guard intent. |
| C7 | caller inventory, startup failure tests and new situational guard adr_0066_whole_config_adapter_callers_are_explicit | After migration, ConfigSnapshot::legacy_config has no callers and is deleted with any unused wrapper, or every remaining caller (including indirect load_config callers) is named by symbol with its reason for requiring the whole configuration. The guard enforces that exact inventory and rejects additional callers; test-only uses cannot justify retaining a production adapter. In every case, prepare_normal stops using the strict adapter and its optional-configuration expect. Invalid optional fields produce scoped issues and do not panic during startup. |

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

1. Launch the fixture with a bad endpoint and a failed playback runtime directory. Confirm the normal app opens, both issues remain visible, and local library work succeeds.

2. Use malformed optional theme/layout values and resize/navigate. The app uses its documented presentation default; fixture inspect must show the original values unchanged.

3. Fail only producer setup, then only publisher host selection. Verify the unaffected controls and statuses still work. Run the fixture's partial-path-repair case and inspect the warning without losing valid tracks. Clean up.

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

A real command dependency is missing from this inventory, a strict whole-Config reader cannot be replaced without changing a service contract, or a repair error cannot be contained without erasing bindings. Report the exact owner and add no substitute server/driver.
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
- Open the app with valid core resources even when optional configuration or tool preparation fails, and limit only the operations that actually depend on each failure.

Constraints:
- Follow this packet's Constraints and Implementation Steps.
- Preserve its data, dependency and prose-retirement contracts.
- One packet this session. Never run the app.

Do not touch:
- New service-state vocabulary, seventh ServiceState or remote protocol changes
- ADR 0064 repair-history surface, config-format migration or decoder changes beyond scoped adapters
- Playback timing, queue semantics, broad show.rs decomposition

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
