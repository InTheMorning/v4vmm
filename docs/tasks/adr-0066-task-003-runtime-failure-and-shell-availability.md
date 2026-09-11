# ADR 0066 Task 003: Runtime Failure And Shell Availability

Status: Complete - 2026-09-11; mechanical gate Green; all operator checks, final preservation inspection and fixture cleanup accepted.
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
- Cargo.toml — measured debug text-shaping cost during the Settings recheck
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
the remaining task 003 checks subsequently passed as recorded below.

### Operator Correction: Settings Responsiveness

The operator reported a brief stall on opening Settings during the Ctrl-shortcut
check. `render_settings` read SQLite and rebuilt the cached-file tree on every
render. This correction applies ADR 0040's existing background-command boundary
and ADR 0066's requirement that Settings remain accessible. It does not start
task 004 or change the app's minimum requirements.

`LoadCachedTracksTree` owns the read. `CachedFilesVm` owns the saved observation,
load state, coalescing and invalidation. The Settings renderer uses that snapshot
and the existing caption/label tokens. Settings entry, library mutations and
runtime recovery request refreshes through `present_command`. Query failures
cannot overwrite unrelated Settings action results. The old synchronous query
facade and the unused library-screen tree-builder wrapper are removed; the
application tree builder remains the shared owner.

Mechanical criteria and proof:

- `adr_0040_settings_cache_reads_leave_the_render_thread` guards the renderer,
  command dispatch and entry/event/runtime-repair wiring.
- `adr_0040_cached_tree_command_excludes_library_files` tests the actual SQLite
  query and tree, including a file moved into the library.
- `adr_0040_cache_refresh_coalesces_and_discards_invalidated_results` proves one
  pending read and one fresh read after an intervening invalidation.
- `adr_0066_unread_and_failed_cache_are_not_reported_as_empty` and
  `adr_0066_cache_failure_retains_last_observation` prove truthful states and
  retained observations. A failed read does not retry indefinitely.

Settings-correction verification is Green: 18 focused cache tests, all 230
architecture tests, cargo check, formatting, strict production Clippy and the
rebuilt debug binary. The agent did not launch the GUI.

Responsiveness/readability and both cached-file state checks passed on
2026-09-11. The steps below remain as regression instructions; do not repeat
them for acceptance. Reuse an isolated startup fixture and launch it with:

```bash
python3 docs/runbooks/startup-recovery-fixture.py run "$recovery_dir"
```

1. Accepted: Settings responds promptly and its text remains complete and readable.
   Do not repeat this check for probe removal alone.
2. Accepted: in the unavailable-runtime fixture, Cached files explained that
   the list could not be read because background tools were unavailable,
   pointed to Check again, and did not claim that the list was empty.
3. Accepted: with the app open, set the fixture to normal in the other terminal
   and choose Check again for Background runtime:

   ```bash
   python3 docs/runbooks/startup-recovery-fixture.py mode "$recovery_dir" normal
   ```

   Cached files updated in the mounted Settings pane after repair. This
   fixture has no cached tracks, so the completed read should say No cached
   files. Typing or switching tabs must not repeatedly start reads on rendering.
   No real external service is required. Keep previous accepted recovery checks
   closed; finish preservation inspection and cleanup using the shortcut packet.

### Settings Delay Evidence

A temporary debug probe measured selection, frame composition, layout,
prepaint, paint submission and the following frame callback. That last interval
includes presentation and waiting; it does not measure when pixels reached the
display. A temporary fixture command also sampled the verified app's main thread
with Linux perf. Both probes were removed after the operator accepted the fix.
The measurements and correction remain here as evidence.

Operator evidence — 2026-09-11 UTC:

| Request time | Selection handler | Frame composition | Layout computation | Through following frame callback |
|---|---:|---:|---:|---:|
| 01:28:01 | 0.052 ms | 0.347 ms | 489.369 ms | 547.349 ms |
| 01:28:03 | 0.031 ms | 0.333 ms | 479.124 ms | 533.052 ms |
| 01:28:04 | 0.035 ms | 0.351 ms | 496.977 ms | 536.999 ms |

### Debug Text Shaping Correction

Operator evidence — 2026-09-11 01:44 UTC: five further visits spent
487.687–507.121 ms between layout request completion and prepaint. The main-thread
profile contained about 1,000 samples with none lost. Its leading symbols were
font-table parsing and the unoptimized Option/Result/slice operations around it:
`ttf_parser::parser::FromData::parse` and
`ttf_parser::ggg::layout_table::Feature::parse` among them.

The pinned GPUI path is `LineLayoutCache::layout_wrapped_line` → `layout_line`
→ `CosmicTextSystemState::layout_line` → `ShapeLine::new`. GPUI caches line
layouts for the current and previous frame. Returning after another section has
drawn can require shaping Settings text again. Cosmic Text creates a Rustybuzz
`ShapePlan`, which parses the font's OpenType features. Cargo previously compiled
all three dependencies without optimization in the debug app.

`Cargo.toml` now optimizes `ttf-parser`, `rustybuzz`, and `cosmic-text` in the
development profile. Optimizing their callers also covers generic parsing code.
Cargo's build records confirm level 3 for these libraries and level 0 for
v4vmm, with debug symbols, assertions and overflow checks retained in each.
Recovery-fixture injection remains available. Dependency versions, app fonts,
layout, and release-profile settings are unchanged. Optimized dependency builds
cost more compilation time, and stepping through their code may be less direct.

A CPU-only comparison shaped the same report and Settings labels ten times,
using Liberation Sans from the agent's installed fonts. After the first pass,
the unoptimized runs took 37.339–37.834 ms; the optimized runs took
1.349–1.427 ms. The complete glyph-layout debug output had the same fingerprint
on every pass before and after (`d7266c32cf0e71ee`). This verifies the local
speedup and output parity. The operator's desktop measurements follow below.
No GUI was launched by the agent.

Mechanical verification of the rebuilt debug profile is Green: cargo check,
formatting, strict production Clippy, debug build, all 1,322 unit tests and all
230 architecture tests. Ten existing doctests remain ignored. The initial
sandboxed suite could not complete its socket fixtures; the complete rerun with
local sockets permitted passed. Operator acceptance is recorded separately below.

Operator timings after the correction — 2026-09-11 UTC:

| Request time | Selection handler | Frame composition | Layout computation | Through following frame callback |
|---|---:|---:|---:|---:|
| 02:00:18 | 0.053 ms | 0.358 ms | 24.371 ms | 50.274 ms |
| 02:00:19 | 0.058 ms | 0.361 ms | 22.405 ms | 37.045 ms |
| 02:00:20 | 0.054 ms | 0.393 ms | 22.224 ms | 39.676 ms |
| 02:00:22 | 0.049 ms | 0.414 ms | 22.163 ms | 40.273 ms |

The reported half-second layout interval is absent on all four visits. Layout
now takes 22–24 ms, about twenty times faster on the operator's desktop.
The operator then explicitly confirmed responsiveness and complete, readable
text. This closes the Settings speed/readability gate. It does not accept the
remaining shortcut or cached-file state checks.

**Situational manual regression guard — ADR 0066 task 003:** after changing the
text dependency profiles or upgrading the text stack, use a normal `cargo build`
and the existing startup fixture with its Background tools report visible.
Alternate Music and Settings, waiting for each draw. Settings must respond to a
single request without the reported half-second pause; reports, inputs and
cached-file states must remain complete and readable. First-entry and repeat
visits both count. A visible pause or missed input fails the check. The guard
needs no external service and shares the fixture's normal preservation
inspection and cleanup.

**Retirement complete — 2026-09-11:** this packet removed the temporary
`settings_timing.rs` module, its TopApp state and frame wrapper, the fixture's
timing-environment exception, and its `profile-settings` command. The optimized
text dependencies, cache-query guards and manual regression check remain.
Existing profile captures stay in the operator's fixture until normal cleanup.
Removal verification is Green: cargo check, formatting, strict production
Clippy, debug build, four shortcut unit tests and all 230 architecture checks.
The fixture's help lists no profiling command, and its environment check
confirms that the retired timing flag is stripped. The deleted probe's own
report-format test retired with it; no new runtime instrumentation was added.

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

Complete - 2026-09-11. Refresh/playback keyboard rejection and both cached-file
state checks passed. Quit returned exit code 0; final preservation inspection
and fixture cleanup passed. The procedures in this packet and the
[shortcut packet](adr-0067-task-001-platform-shortcuts.md#operator-visual-check)
remain for regression checks. No acceptance check remains open for this packet.

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

At the end of the 2026-09-10 run, refresh/playback keyboard rejection remained
unverified because the window manager intercepted Super.

### Operator Evidence — 2026-09-11

After relaunching the current fixture in `runtime-and-cache-unavailable` mode,
the operator pressed Ctrl+R once in Music. Refresh reported unavailable
background tools, finished loading and left navigation working. The operator
confirmed pass. One Ctrl+Alt+P press in Music then produced a Settings result
explaining that playback failed because background tools were unavailable.
Navigation and repair controls stayed usable; the operator confirmed pass.
Keyboard rejection is accepted. In Settings, Cached files then explained that
background tools were unavailable and pointed to Check again in Background
tools, without claiming an empty list. The operator confirmed pass. With the
app still open, the operator set the fixture to normal, chose Check again for
Background runtime, and confirmed that Cached files changed to No cached files
without leaving Settings. Recovery in place passed. Ctrl+Q then closed the app
and the fixture terminal reported exit code 0; the operator confirmed pass.
Final inspection in normal mode passed. Configuration changes are confined to
workspace preferences (`config_bytes_unchanged: false`,
`normal_workspace_preferences_only: true`); config and music are preserved.
Migration versions 1–11 are preserved, one playlist remains, database probes
are zero and residual music probes are empty. The operator then confirmed the
cleanup command's Removed fixture message. All acceptance and cleanup gates
are closed. The accepted desktop build used revision 6c63451 with the temporary
timing/profiling code removed; the retirement build and checks were Green before
the final operator sequence. Task 004 is next and has not started.

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
