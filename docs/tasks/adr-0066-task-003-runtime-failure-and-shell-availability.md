# ADR 0066 Task 003: Runtime Failure And Shell Availability

Status: Implementation recorded - 2026-09-10; only the intercepted Super-key operator check remains open; other operator checks and fixture cleanup passed.
Mechanical gate Green. Task 004 has not started.

## Goal

Keep navigation, reports and repair access working when the normal background runtime or optional thumbnail worker cannot start.

## Read First And Dependency

Read [ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md),
the [phase plan](../plans/adr-0066-startup-recovery-phase-plan.md), this whole packet, and the
[review checklist](../reviews/adr-0066-startup-recovery-review-checklist.md).
Execute after [task 002](adr-0066-task-002-core-checks-and-startup-reports.md) in the phase plan's order.
Complete this packet in one session; do not start its successor.

## Files To Inspect

- src/app.rs — TopApp::new, command_runner and runtime_host
- src/library.rs; src/library/app_impl.rs — constructors, command_runner and paged actors
- src/application/async_command_runner.rs — constructors and dispatch
- src/presentation/async_command_presenter.rs; src/presentation/runtime_host.rs; src/presentation/gpui_vm_bridge.rs
- src/app/show.rs; src/app/search_dispatch.rs; src/app/keyboard.rs
- src/media/image_cache.rs — with_capacity startup eviction
- src/runtime/mod.rs; src/presentation/mod.rs
- Task 002 startup/maintenance/report owners
- `tests/architecture_tests.rs`; `AGENTS.md`

## Changed Owners

- src/application/async_command_runner.rs; src/application/errors/command.rs; src/application/capability.rs; src/application/mod.rs
- src/presentation/async_command_presenter.rs; src/presentation/runtime_host.rs; src/presentation/gpui_vm_bridge.rs
- src/app.rs; src/app/bootstrap.rs; src/app/capabilities.rs; src/library/app_impl.rs
- src/discover/app_impl.rs — same unavailable-runner correction in the standalone search constructor
- src/app/show.rs; src/app/search_dispatch.rs — dependency dispatch only; keyboard routes stay on these shared commands
- src/view_models/startup.rs; src/view_models/startup/capabilities.rs; src/ui/composites/startup_report.rs
- src/view_models/show.rs — unavailable-query projection; no layout change
- src/view_models/search_results/{mod,empty_state,failure,tests}.rs; src/ui/shells/search_results_inspector.rs; src/ui/tokens.rs — operator correction for clipped search failures
- src/diagnostics.rs; src/lib.rs; src/startup.rs — shared report URL redaction
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

## Implementation And Proof

| Owner | Implemented boundary |
|---|---|
| `AsyncCommandRunner::unavailable`, `availability`, `dispatch` | Unavailable runners contain a typed `ExecutionUnavailable` and no Tokio handle. Dispatch returns `CommandError::Unavailable` without command execution or success events. Explicit healthy-handle constructors remain. |
| `CapabilityObservations`, `CapabilityReportVm` | Dependency-keyed observations retain actual UTC times and safe error categories. Runtime and thumbnail issues remain independent. Configure/Check/Copy actions have typed availability and accessibility labels. |
| `capability_report` in the shared startup-report composite | Music/Show get a concise notice; Settings gets the full issue report and Check again. Both use the same VM data. A completed retry retains its numbered UTC receipt. |
| `TopApp::check_capability`, `complete_capability`, `install_runtime` | Explicit checks run on `MaintenanceClient`. Generation admission precedes installation; a repaired host updates the existing child and starts guarded actor/bridge sets once. |
| `LibraryApp::install_runtime`, `maybe_start_musicbrainz_feed_saga` | Installs the explicit runner in the existing child, refreshes data while retaining detail, and starts one saga. Playlist actor ownership is unchanged. |
| `ImageCache::start_maintenance`, `check_maintenance` | Builder-based launch and actual pruning failures become scoped observations. Disk/hot cache and capacity behavior survive; checks name the cache path. |
| `RuntimeHost::for_config`, `bootstrap::cache_worker_for_config`, `fixture::injected_failure` | Production uses fallible factories. Debug failure injection requires an explicit verified fixture, matching configuration and running binary, and a fresh case read. |
| `SearchFailureDisplay`, `activate_failure_action`, shared `render_empty_state` | The operator-reported Index error has a bounded explanation and VM-owned Show details/Copy report actions. Reports retain recorded UTC and redacted diagnostic text; `diagnostics` owns URL handling. |

### Dispatch And Actor Inventory

| Entry point | Dependency boundary and surviving work |
|---|---|
| `app/search_dispatch.rs` — toolbar button, Enter and saved-search routes | Local library search still uses its existing independent query path. Index requests, downloads and metadata commands reach `present_command` → `AsyncCommandRunner::dispatch`. |
| `app/keyboard.rs`, `app/playback_bar.rs` | Playback shortcuts call the same rejecting runner as mouse commands. Navigation, focus and selection remain usable. |
| `library/app_impl.rs` | All `present_command` calls share rejection and the persistent enabled repair route. Existing playlist reads and browsing remain independent; creating a playlist still needs its command runner. |
| `app/show.rs`, `ShowPageVm::with_execution_availability` | Command/query dispatch shares rejection. The readiness and service watchers return before spawning when the host is absent. Show says its status is unavailable instead of claiming idle playback; unobserved sections remain absent, with the shared notice offering repair. |
| `app.rs::maybe_start_playback_polling` | Checks the host before session loading or actor startup. Its existing live-driver and single-handle checks remain. |
| `LibraryApp` saga and paged playlist actor | No host means no actor. The saga's unavailable error identifies the same repair route; the existing actor-backed paging path is not replaced by blocking work. |
| TopApp/LibraryApp thumbnail requests | Hot-cache reads remain usable. Unavailable fetches do not enter Loading, so repairing the runtime allows the next fetch. |
| `discover/app_impl.rs` | The retained standalone search constructor also uses an explicit unavailable runner. The active Music/Show/Settings shell owns the repair report. |

`present_command` retains the typed rejection through each caller's error path.
The shared notice and Settings report keep Check again reachable even when that
caller's local status text is hidden. No blocked command is replayed by saving
settings or repairing the runtime; task 007 owns original-action return.

## Acceptance Criteria

### Operator Correction: Readable Search Failures

During the runtime-recovery check on 2026-09-10, the operator's `runtime-retry`
screenshot proved that the repaired runner attempted the offline MusicIndex
request, but its error was clipped at both edges. Correct this observed failure
under ADR 0066's scoped-error/report decisions and ADR 0063's column-text rule.
This does not start task 004 or change remote-search success/partial-result policy.

Implemented by `SearchFailureDisplay` and the shared `render_empty_state`, with
`adr_0066_search_failure_report_stays_readable_and_vm_owned` guarding their
boundary. A generic query error does not prove that MusicIndex is down.
`adr_0066_search_failure_reports_the_dependency_without_inventing_a_network_answer`
tests that distinction. `adr_0066_search_report_copy_is_complete_recorded_and_safe_before_disclosure`
tests recorded UTC, URL credential redaction and complete copy.
`adr_0066_search_failure_disclosure_respects_scope_and_resets_on_retry` tests
disclosure/reset and Library/Index separation. Existing navigation race and
remote-search guards remain unchanged. These references replace the correction's
implementation instructions.

The [visual recheck](../runbooks/startup-recovery-check.md#2a-recheck-search-failure-readability)
passed for normal/narrow widths, expanded details, complete copy, Library-filter
separation and playlist uniqueness. This focused correction is operator-accepted;
the remaining task 003 checks stay open.

### Original Packet Criteria

Mechanical; asserted at the named owner. The proof inventory below links the implemented checks.

| ID | Proof owner | Required assertion |
|---|---|---|
| C1 | runner and constructor tests | With no Tokio context and a failed RuntimeHost factory, unavailable runner construction/dispatch does not panic, execute a command, emit success or spawn a fallback runtime. |
| C2 | view-model/command tests | Navigation/report/maintenance actions remain enabled. Every runtime-dependent entry point, including toolbar and keyboard, rejects execution and offers a typed remedy. |
| C3 | issue-list tests | Simultaneous runtime/cache issues survive navigation; resolving one leaves the other. No adapter construction is projected as a successful remote observation. |
| C4 | retry lifecycle tests | A successful explicit runtime retry installs exactly one host/runner/bridge/actor set; repeated clicks and stale completions cannot duplicate it. |
| C5 | ImageCache tests with injected worker/prune failures | Failure is returned as a scoped issue while usable cache behavior remains. No panic or library reset occurs. |
| C6 | situational guard adr_0066_missing_runtime_has_no_implicit_runner | Both GUI composition roots use explicit availability; missing runtime cannot reach Handle::current or UI-thread work. Preserve ADR 0040 runtime/GPUI separation guards. |

### Mechanical Evidence

Green - 2026-09-10: formatting, cargo check, strict production Clippy, build,
1,314 unit tests and 227 architecture tests, including the search-error correction.
Ten existing documentation examples
remain ignored. The full suite used local socket fixtures outside the sandbox.
Fixture setup/verification, all three new failure modes, normal mode, preservation
inspection and cleanup are Green. Configuration, music and migration records
remained intact with no residual probes. The agent did not launch the GUI.

| Criteria | Implemented tests/guards |
|---|---|
| C1 | `adr_0066_unavailable_runner_rejects_commands_without_execution_or_events`; `adr_0066_failed_runtime_factory_has_no_fallback` |
| C2 | `adr_0066_blocked_mutation_keeps_existing_local_queries_usable`; `adr_0066_independent_issues_keep_typed_remedies_and_recorded_times`; `adr_0066_unavailable_show_query_does_not_claim_idle_playback`; `adr_0066_missing_runtime_has_no_implicit_runner` inventories keyboard, toolbar and command routes |
| C3 | `adr_0066_observations_are_independent_without_a_runtime`; `adr_0066_independent_issues_keep_typed_remedies_and_recorded_times` |
| C4 | `adr_0066_retry_generation_admits_one_install_and_rejects_stale_results`; `adr_0066_independent_worker_restores_an_explicit_usable_runner`; `adr_0066_repeated_tool_failures_keep_distinct_recorded_feedback`; `adr_0066_runtime_retry_keeps_one_host_and_independent_reports` guards the actual installation/subscription callers |
| C5 | `adr_0066_failed_cache_worker_preserves_disk_hot_and_capacity_behavior`; `adr_0066_cache_prune_failure_reports_the_path_and_keeps_cached_images` |
| C6 / fixture | `adr_0066_missing_runtime_has_no_implicit_runner`; `adr_0066_runtime_retry_keeps_one_host_and_independent_reports`; `adr_0066_fixture_failures_require_identity_and_follow_fresh_case` |

The situational guards live in [architecture_tests.rs](../../tests/architecture_tests.rs).
Behavioral tests live beside the owners in the implementation table. Existing
runtime/GPUI, startup, workspace and Show guards remain required.
`adr_0060_live_status_and_show_share_cached_projection` now checks the typed
unavailable initial projection under ADR 0066; its cached projector and
invalidation assertions remain. Implemented
procedures and the coding prompt are retired in favor of these references.
ADR decisions/invariants remain binding. Task 004 still owns optional config,
player/producer isolation, secondary reads and path-repair failure policy.

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

Only refresh/playback keyboard rejection remains open, on a desktop that forwards
Super+R and Super+Alt+P to the app. Use the unavailable-runtime case in
[Task 003: Background tools](../runbooks/startup-recovery-check.md#task-003-background-tools).
All other operator checks, preservation inspection and fixture cleanup passed;
they need no repeat. The evidence below records the keyboard limitation without
claiming desktop key delivery from mechanical dispatch tests.

### Operator Evidence — 2026-09-10

The operator reported **pass** after opening the isolated fixture in
`runtime-and-cache-unavailable` mode and visiting Music, Show and Settings.
Startup/navigation and two separate failures with reachable Check again controls
are accepted.

Repeated-check feedback passed: the operator confirmed increasing check counts
and updated timestamps. Attempts completed immediately, with no visible
Checking state. The fixture returns its injected failure immediately; a visible
intermediate frame is not required for this case. Completion feedback is retained.

Full report copy passed. The supplied report identifies fixture
`/tmp/v4vmm-startup-clzmrchl`, check 12 completed at 22:59:59 UTC, the runtime
failure recorded at that time, and the separate thumbnail-worker failure at
22:59:22 UTC. It includes the affected and available work, recovery actions,
and the complete thumbnail-cache path. No report content is truncated.

Narrow-width readability and unavailable Show wording passed. The operator
confirmed that the Settings report wraps, its buttons remain reachable, and
Show says **Show status unavailable** and **Not checked** while the runtime
is unavailable.

Search dispatch passed with both the toolbar Search button and Enter in the
search field. The operator confirmed the unavailable-runtime explanation and
continued access to navigation and Check again.

Existing playlist browsing passed. The operator's window manager uses Super as
its modifier and intercepted Super+R and Super+Alt+P. Those shortcut checks are
unverified, not passed; the app did not receive the keys. Continue independent
repair checks without changing window-manager bindings. The mechanical dispatch
guards remain Green, but do not prove desktop key delivery.

Independent runtime recovery passed. After switching the fixture to
`cache-worker-unavailable`, the operator pressed the runtime's Check again and
confirmed that its failure cleared, thumbnail cleanup remained failed, and the
same app window stayed open.

The `runtime-retry` screenshot proves post-repair remote dispatch: the report
names the attempted `http://127.0.0.1:9/v1/search` requests and only thumbnail
maintenance remains failed. The operator rejected the clipped diagnostic line.
The readable-search-failure correction's initial layout recheck passed: after
relaunching the rebuilt fixture, the operator confirmed the explanation wraps
inside the pane at normal/narrow widths and Show details/Copy report remain
reachable. Expanded diagnostics and copy also passed. The operator supplied the
complete report recorded at 23:39:54 UTC: it names MusicIndex, explains the
failure and next action, identifies `http://127.0.0.1:9/`, and includes both
feed and track request errors. Hide/show preserves the timestamp and report.
The subsequent two screenshots verify Library-filter separation and playlist
uniqueness: local search shows its normal empty state without the MusicIndex
error/report buttons, and the playlist list contains one Startup fixture playlist.
The thumbnail-maintenance notice remains a separate shell issue. The focused
search-error correction is now operator-accepted.

Thumbnail repair passed in the existing window. After selecting fixture mode
`normal` and Check again, the operator's Settings screenshot records the
completed thumbnail cleanup scan at 23:50:24 UTC and its full cache path. The
runtime's earlier startup result remains at 23:39:34 UTC and explicitly makes
no claim about external-service reachability. Both failure rows cleared.

Success-report copy and the post-repair preservation inspection passed. The
copied report retains both successful results at 23:39:34 and 23:50:24 UTC.
The `normal` inspection reports configuration and music preserved, one playlist,
migration versions 1–11 preserved, no residual music probes and zero database
probes. Changed config bytes are confined to normal workspace preferences.
The supplied inspection output does not include that app exit code.

Fresh startup in `cache-worker-unavailable` mode passed. Only thumbnail
maintenance failed; navigation through Music, Show and Settings remained usable.
The operator confirmed clean shutdown with exit code 0.

Final preservation inspection passed in `cache-worker-unavailable` mode.
Configuration and music are preserved; changed config bytes are confined to
normal workspace preferences. One playlist and migration versions 1–11 remain,
with migration records preserved, no residual music probes and zero database probes.

The operator confirmed fixture cleanup after the final preservation inspection.

Still open: refresh/playback keyboard rejection on a desktop that forwards the keys.

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
