# ADR 0066 Task 004: Optional Tool Isolation

Status: Implemented - 2026-09-11; mechanical gate Green. The presentation case,
including the duplicate-warning recheck and preservation inspection, is accepted.
Producer-case preservation is Green; its playback acceptance remains open.
The remaining operator checks and final fixture cleanup are open. Playback-dependent
checks are paused after the operator identified the missing Show cue/audition
separation; the producer screenshot also contains an unresolved mpv IPC error.
Task 005 is not started and waits for this gate.

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

## Implementation And Proof

The three situational guards in [architecture_tests.rs](../../tests/architecture_tests.rs)
replace this packet's implementation procedure:

- `adr_0066_optional_dependencies_are_scoped`
- `adr_0066_repair_failure_preserves_bindings`
- `adr_0066_whole_config_adapter_callers_are_explicit`

The existing ADR 0046/0051 persistence guards now follow scoped decoding and
independent presentation fallback. The ADR 0066 save guard still requires the
config writer's fresh snapshot and saveability check. Runtime retry still installs
one host; player polling begins after a successful playback command, so opening
the app or recovering the runtime cannot launch/ping mpv. Playback session data,
queue ordering and driver timing remain owned by their existing modules.

| Criterion | Actual behavioral proof |
|---|---|
| C1 | `app::bootstrap::tests::adr_0066_normal_factory_scopes_each_optional_group_and_keeps_config_bytes`; `startup::tests::adr_0066_failed_player_and_producer_preserve_independent_resources` |
| C2 | `application::capability::tests::adr_0066_optional_dependency_pairs_preserve_independent_actions`; `app::show::tests::adr_0066_broadcast_observers_and_commands_are_independent`; `view_models::show::tests::adr_0066_show_projects_paired_availability_and_retains_setup_reports`; `view_models::library::tests::playlist_track_row_vm_can_play_follows_local_path`; existing runtime/maintenance tests and guarded keyboard/toolbar routes |
| C3 | `cli::tests::adr_0066_cli_shares_one_snapshot_and_requires_only_used_fields`; `rss::subscribe::tests::adr_0066_known_rss_import_skips_invalid_index_and_retains_local_identity`; `api::tests::adr_0066_invalid_endpoint_rejects_requests_before_transport`; `broadcast::registry::tests::forget_event_removes_row_and_token_file_without_relay_call`; `track_compare::tests::adr_0066_invalid_converter_preserves_downloads_and_retains_wav` |
| C4 | `startup::tests::adr_0066_partial_repair_keeps_committed_and_unvalidated_bindings`; `startup::tests::adr_0066_repair_failure_rechecks_core_storage_and_schema` |
| C5 | Full suite retains explicit/default Null, lazy mpv, absent producer, both ADR 0064 repair skips, relative-path safety and task 017 event tests |
| C6/C7 | Three guards above, normal factory failure tests and the production caller inventory below |

### Production Caller Inventory

`Config`, `BroadcastConfig`, `ConfigSnapshot::legacy_config`, `load_config` and
`ensure_dirs` are removed. There are no whole-config adapter callers, including
indirect callers or test-only exceptions. The unused `discover::run_search_app`
composition root is removed.

| Reader / caller | Fields consumed |
|---|---|
| `StartupBackend::execute` → `prepare_normal` → `mount_normal` | One admitted snapshot; core paths required; every optional field remains scoped |
| `CliConfig::snapshot` and CLI helpers | One cached snapshot per invocation; database, music destination, endpoint, player or host only as the operation requires |
| `EventRegistryCommand::execute` | Create/Replace read the current endpoint; Check reads the selected event's saved endpoint |
| `subscribe_track`, `subscribe_feed`, `download_and_compare_track`, `lookup_musicbrainz_track` | Download destination and deferred converter result |
| `feed_service` file operations; `library_service::subscribe_then_append_to_playlist`; `broadcast_readiness_report` | Music path only |
| `rss::subscribe_feed` | Provided RSS URL and optional typed Index endpoint; invalid Index skips enrichment |
| `LibraryApp::spawn_playlist_actor` | Uses the already admitted connection's database path; no global configuration reread |
| Configuration save functions | Fresh snapshot to protect all original bytes from an ordinary save while any issue survives |

`MusicIndexEndpoint` reaches the API URL builder, so a direct command cannot
bypass endpoint validation. Publisher, producer and encoder preparation are
independent. Failed player preparation leaves no owner. Configuration issues,
preparation failures and incomplete path repair retain separate recorded reports.
Shared Show/playlist view models project action availability; existing shell
geometry and tokens render it. Presentation defaults apply per invalid field,
while ordinary configuration persistence stays paused.

### Verification Evidence — 2026-09-11

Repository gate: `cargo fmt -- --check`, `cargo check --quiet`, `cargo test --quiet`,
`cargo clippy --quiet -- -D warnings`, and `cargo build --quiet`: Green.
The test suite needs local HTTP/Unix sockets outside the agent sandbox; assertions
were not relaxed for that restriction. Test output is in
`/tmp/adr0066-task004-tests-unrestricted.log` for this session.

Backend fixture verification seeded three real WAVs and one playlist, exercised
all five optional cases without opening the GUI, and parsed JSON stdout from
playlist list, library tracks and local event list with invalid Index/player
settings. A real SQLite trigger rejected the second path update; inspection
confirmed `["a.wav", "/old/music/b.wav", "c.wav"]`, three tracks, three playlist
entries, unchanged config/music bytes, unchanged migration records and no probes.
The agent's disposable fixture was inspected and cleaned up. This proves backend
preservation only; the operator gate below remains open.

### Bounded Placement Changes

The typed endpoint also changes the API client and its query/command request
adapters, plus the audio-materialization config type in `track_compare.rs`.
These are necessary consumers of the removed strict adapter. Playlist shell
wiring passes typed availability through its existing view-model display; the
screen never rereads its own GPUI entity during rendering. No new service state,
protocol, schema migration, configuration format or correction/retry workflow
was added. Task 005 and all later packets remain unstarted.

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

Open. Follow [Task 004: Optional Tool Isolation](../runbooks/startup-recovery-check.md#task-004-optional-tool-isolation)
using [startup-recovery-fixture.py](../runbooks/startup-recovery-fixture.py).
The runbook supplies unindented commands, five named cases, expected/wrong
results, actual-audio prerequisites, report-copy checks and cleanup.
Its playback portions are paused as recorded below; do not use Music Play as a
substitute for loading a Show cue and starting it from Show.

The shared owners are `application::capability`, `view_models::startup::capabilities`,
`view_models::show`, `view_models::library` and `view_models::app_toolbar`.
The existing capability notice/Settings composites, Show cards and playlist row
controls retain their existing token paths and geometry. Mechanical proofs are
listed above; only a person can accept their running presentation.

Record actual observations here before closing this gate. Keep this Status,
the phase plan, delivery row and pending-check index open until all five cases,
post-run preservation inspection and operator fixture cleanup pass.

### Operator Evidence — 2026-09-11

The operator supplied the Background tools report from
`/tmp/v4vmm-startup-6rsrpoyp`. Its four observations are recorded at
`2026-09-11 12:06:06 UTC`:

- App started its background runtime, explicitly without claiming external
  service reachability.
- App completed the thumbnail cleanup scan and named the fixture cache directory.
- App could not prepare built-in playback; its report retained the failure,
  configuration location and correction instructions.
- App rejected `musicindex_endpoint`, retained that issue separately, and
  reported that ordinary configuration saves were paused.

The supplied text matches the expected report content for the paired
Index/player case.

The operator then supplied the `endpoint-and-player-unavailable` fixture
inspection. Preservation is Green: configuration bytes, music, tool blocker,
bindings and migration records are unchanged. The database retains one playlist,
three tracks and three playlist entries; bindings remain `a.wav`, `b.wav` and
`c.wav`. Music/producer probes are absent and the database probe count is zero.
The inspection output has no timestamp; none is inferred from the earlier report.

The remaining first-case visual checks are unconfirmed: normal navigation,
local search results, all three playlist rows, unavailable Play, rejected
Ctrl+Alt+P on Show and retained reports after navigation. Those checks remain
open alongside the producer,
publisher and partial-path-repair cases. No first-case acceptance or operator
fixture cleanup is recorded; presentation acceptance is recorded below.

The operator subsequently reported that the original fixture's `fixture.json`
was missing. Fixture discovery found no verified replacement. The cause of the
missing fixture is not established. The agent created
`/tmp/v4vmm-startup-yz5928gu`, selected `presentation-invalid`, and verified its
baseline preservation without opening the app. The earlier report and inspection
evidence remains recorded. This replacement is retained for the operator's
remaining checks. The runbook now explicitly keeps cleanup after all five cases.

The operator requested transfer from the agent sandbox into the desktop's
`/tmp`. The agent verified the replacement and packaged it in the shared checkout
as `target/startup-recovery-handoff.tar.gz`; archive comparison against the
source fixture is Green. Extracting into `/tmp` with its original directory name
preserves the absolute paths in the manifest and configuration. Desktop-side
verification and the presentation check remain outstanding. Agent-side fixture
discovery alone does not establish what exists in the operator's filesystem.

### Presentation Screenshot Correction — 2026-09-11

The operator's screenshot shows the Show section mounted with separate theme,
scale and content-view warnings in the shared notice. It also exposes the same
three warnings repeated as red Show status text below Idle. It does not establish
narrow-window behavior, persistence after navigation or post-run preservation.

`view_models::startup::normal_startup_status` now projects only residual startup
notices into normal status. Optional configuration notices retain their shared
capability-report owner. `mount_normal` uses that projection; Show command-result
messages keep their existing route. The change adds no renderer geometry or
token adjustment.

`app::bootstrap::tests::adr_0066_optional_notices_have_one_normal_report_owner`
proves that all three configuration reports remain available exactly once, a
separate download-directory notice still reaches normal status, and configuration
and blocker bytes remain unchanged. The existing optional factory test also
checks that configuration notices do not become normal status text.
`adr_0066_optional_dependencies_are_scoped` guards the live projection call.

Correction verification is Green: 71 focused ADR 0066 unit tests, the existing
`show_page_carries_a_command_message` regression, all 233 architecture checks,
`cargo fmt -- --check`, `cargo check --quiet`, `cargo clippy --quiet -- -D warnings`
and `cargo build --quiet`. Session logs use the
`/tmp/adr0066-task004-report-` prefix. The rebuilt debug binary is ready; no
agent GUI run or operator acceptance is claimed.

The operator subsequently supplied Music and Settings screenshots for the
transferred `presentation-invalid` fixture. Music shows all three tracks
(`a.wav`, `b.wav`, `c.wav`), the three-entry fixture playlist, and the three
shared warnings. Its recent-music request reports connection refused at
`http://127.0.0.1:9`, the fixture's intentionally unreachable Index endpoint.
Settings shows all three presentation reports with correction instructions,
configuration locations and observations recorded at `2026-09-11 12:56:34 UTC`.
The timestamp belongs to those report observations, not to screenshot capture.
The report paragraphs wrap within the displayed Settings column.

The operator then supplied a `presentation-invalid` inspection and stated
"pass". Presentation acceptance is met on 2026-09-11, including the requested
Show duplicate-warning recheck, resizing/navigation and retained reports.
Preservation is Green: configuration bytes, music, bindings, library entries,
migration records and tool blockers are preserved; no probes remain. The fixture
retains three tracks, three playlist entries and one playlist. The inspection has
no timestamp; no completion time is inferred from earlier report observations.

This closes the presentation case only. The first case's remaining visual
confirmation, producer failure with audible playback, publisher isolation,
partial path repair and final fixture cleanup remain open. Task 005 stays pending.

### Playback Workflow Correction — 2026-09-11

The operator's producer-case screenshot retains the drop-file preparation
warning in the shared notice and Source card. Show displays `a.wav` as Playing,
with all three fixture tracks in its cuelist. It also reports
`Playback error: read mpv IPC message: Resource temporarily unavailable (os error 11)`.
The image does not establish successful audio or preservation. No observation
time is inferred from its elapsed playback display. The IPC error's cause is
unresolved; it is not evidence that producer preparation blocked the player.

The operator clarified the required workflow: load a playlist into the Show
playback cue, then play it from Show. Other playback buttons audition through a
separate audio path. The runbook's instruction to play in Music and then open
Show was incorrect and is withdrawn. This producer case is incomplete.

The current route confirms the gap: `LibraryAppEvent::PlayPlaylistAt` reaches
`TopApp::play_playlist_at`, which uses the same playback owner as Show transport.
`PlaybackOwner::play_playlist_at` loads the driver immediately, updates the
canonical session and synchronizes drop-file metadata when a producer exists.
This route does not provide an independent audition path or a load-cue-only
action. ADR 0060 already distinguishes audition from queuing; the
[proposed ADR 0068](../adr/0068-show-cue-and-audition-isolation.md)
now records the requested cue/audition separation, with implementation pending.
Task 004's C2/C5 proofs cover dependency
isolation and inherited driver behavior, not acceptance of that product workflow.

Playback-dependent producer and publisher checks remain open and paused. Before
resuming them, the playback decision and its implementation must establish
explicit cue loading without starting audio, Show-controlled cue playback, and
audition that does not alter the Show cue/session or publish Show metadata.
Their own tests and operator check must cover that separation. Queue semantics
and playback timing are outside this packet; no playback redesign or IPC fix
is included in this documentation correction. Report-only checks may continue,
but cannot close the outstanding audio/publication criteria. The accepted
presentation case remains closed.

The operator requested ADR 0068 after the producer preservation inspection.
The draft is Proposed; it supplies no playback acceptance or IPC fix and does
not start a new implementation packet. Scheduling and task 004's remaining
criteria still require explicit reconciliation in the delivery order.

### Producer Preservation Inspection — 2026-09-11

The operator subsequently supplied the `producer-unavailable` inspection.
Preservation is Green. `config_bytes_unchanged` is false, but
`normal_workspace_preferences_only` and `config_preserved` are true. This case
has valid configuration and a resource-preparation failure, so ordinary workspace
preferences may save. The inspector confirms that parsed configuration outside
`workspace` and `workspace_layout` is unchanged after a recorded clean app exit.
Exact-byte preservation remains required for the invalid-configuration cases.

Music, migration records, relative bindings, library entries and the producer
blocker are preserved. The database retains migration versions 1–11, one playlist,
three tracks and three playlist entries; bindings remain `a.wav`, `b.wav` and
`c.wav`. No music/producer probes remain and the database probe count is zero.
The supplied inspection contains no timestamp; no observation time is inferred.

This accepts producer-case preservation only. The playback workflow gap and mpv
IPC error remain unresolved; audio/publication acceptance remains paused. Final
fixture cleanup and the packet's other outstanding checks remain open.

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
