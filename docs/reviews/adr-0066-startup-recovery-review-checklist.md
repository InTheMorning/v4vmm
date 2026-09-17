# ADR 0066: Startup Recovery Review Checklist

## Status And Scope

Tasks 001–003 are complete - 2026-09-11. Mechanical checks are Green;
operator acceptance, final preservation inspection and fixture cleanup are
confirmed. The keyboard checks use ADR 0067's accepted Ctrl bindings.
Settings responsiveness and cached-file recovery are accepted; temporary
diagnostics are removed. ADR 0066 remains partial. Task 005 is complete with
mechanical checks Green, operator V1–V3 and preservation accepted, and fixture
cleanup confirmed. Task 004's implementation and mechanical checks are complete;
its presentation case and producer preservation are accepted. Its remaining
operator checks and fixture cleanup stay open; playback checks are deferred.
Task 006 is complete on 2026-09-13 with mechanical checks Green, operator
V1–V6 and preservation accepted, and fixture cleanup confirmed. Task 007 is
complete on 2026-09-16 with mechanical checks Green; V1–V3 and Library/Show
follow-ups are accepted, preservation passed and no startup fixtures remain in
the checked temporary directories. The final reconciliation below corrects an
unsupported extra gate inferred from a port-only log. Task 008 is complete on
2026-09-17 with mechanical checks Green; operator V1–V3, Settings/core-recovery
presentation, preservation in both cases and fixture cleanup are accepted.
Tasks 009–013 have not started.

Read the [ADR](../adr/0066-configuration-and-startup-failure-recovery.md),
[phase plan](../plans/adr-0066-startup-recovery-phase-plan.md), active packet and
its diff. The plan owns sequence; packets own procedures/tests; this checklist
owns coverage review. Existing human gates remain in
[pending human checks](../pending-human-checks.md).

## Invariant Coverage

| ADR 0066 invariant | Packet proof owners | Review question |
|---|---|---|
| 1. Core minimum only | 001–004 | Can valid music storage/SQLite still open the normal app when every external service is unreachable? Are music and SQLite usability actually tested? |
| 2. Typed scoped failures | 003/004/007 | Does each failed dependency limit only its dependent work, with no substituted server/player/path or fake healthy observation? |
| 3. Config preservation | 001/006 | Do first-run creation, all ordinary saves and explicit repair preserve the right original bytes? Are unrelated autosaves paused? |
| 4. One snapshot | 001/004 | Do startup and each command's dependent readers share the correct observation, including invalid sibling fields? |
| 5. Recovery and unavailable runtime | 002/003/005/006 | Can repair run without normal DB/runtime, with no ordinary dispatch/autosave in recovery or implicit runtime fallback? |
| 6. Data protection | 002/004/005/009–013 | Are probes rolled back, path bindings contained, sessions drained, candidates validated and originals preserved? |
| 7. Useful safe reports | 002 and every result-producing packet | Does each report name subject, operation, consequence, next action and recorded UTC, without credentials or fabricated success? |
| 8. Window failure fallback | 002 | Can stderr/exit handle no window without another window/runtime dependency? |
| 9. Correction and explicit retry | 006–009 | Is repair reachable, is the original subject retained and revalidated, and does Save alone do no original-operation work? |

## Transferred Verification Coverage

These rows replace the detailed implementation-verification table previously in
the ADR. They describe proof to obtain, not proof already obtained.

| Requirement | Concrete packet |
|---|---|
| Every startup stage has a disposition, including nonfatal activation and programmer invariants | [002](../tasks/adr-0066-task-002-core-checks-and-startup-reports.md#startup-stage-inventory), with 003/004 follow-through |
| Read/list/write/read-back/remove music probe; absent mount and download-only subtree distinguished | [002](../tasks/adr-0066-task-002-core-checks-and-startup-reports.md) |
| Configured SQLite read/write/schema checks, bounded locks, fresh DB preparation | [002](../tasks/adr-0066-task-002-core-checks-and-startup-reports.md) |
| Invalid core versus each invalid optional field/table and valid siblings | [001](../tasks/adr-0066-task-001-config-snapshot-and-safe-persistence.md), [004](../tasks/adr-0066-task-004-optional-tool-isolation.md) |
| Byte preservation across failure, ordinary saves, first-run race/symlink/cleanup | [001](../tasks/adr-0066-task-001-config-snapshot-and-safe-persistence.md) |
| Explicit backup-protected config repair, editor conflicts, fresh validation before autosave | [006](../tasks/adr-0066-task-006-configuration-repair-and-resumption.md) |
| Snapshot remains coherent when file changes after read | [001](../tasks/adr-0066-task-001-config-snapshot-and-safe-persistence.md), [004](../tasks/adr-0066-task-004-optional-tool-isolation.md) |
| Independent work survives optional failure; CLI/keyboard/toolbar guards cannot bypass scope | [003](../tasks/adr-0066-task-003-runtime-failure-and-shell-availability.md), [004](../tasks/adr-0066-task-004-optional-tool-isolation.md), [007](../tasks/adr-0066-task-007-optional-tool-correction-and-retry.md) |
| Missing converter becomes available in same process; real fallback/warning outcome | [008](../tasks/adr-0066-task-008-converter-verification-and-setup.md), [009](../tasks/adr-0066-task-009-conversion-retry-and-retained-input.md) |
| Same-track repair/retry, staging validation/cleanup, no duplicate materialization | [009](../tasks/adr-0066-task-009-conversion-retry-and-retained-input.md) |
| Database tools without TopApp/runtime, check without migration, committed WAL backup | [010](../tasks/adr-0066-task-010-database-check-and-backup.md) |
| Internal drain, extra connections, external writers, original/journal preservation | [005](../tasks/adr-0066-task-005-session-drain-and-resumption.md), [011](../tasks/adr-0066-task-011-database-maintenance-and-preservation.md) |
| Invalid candidate, failed install, validation before resumption | [012](../tasks/adr-0066-task-012-database-restore.md) |
| Same registry and ledger on fresh/migrated/interrupted upgrade; unsupported corruption refused | [013](../tasks/adr-0066-task-013-interrupted-upgrade-repair.md) |
| Existing Null/absent producer/lazy mpv/presentation fallback and ADR 0064 skips survive | [001](../tasks/adr-0066-task-001-config-snapshot-and-safe-persistence.md), [004](../tasks/adr-0066-task-004-optional-tool-isolation.md) |
| Partial repair error preserves completed changes and contains unvalidated bindings | [004](../tasks/adr-0066-task-004-optional-tool-isolation.md) |
| Window close/failure, single-flight checks, stale callbacks and one resumption | [002](../tasks/adr-0066-task-002-core-checks-and-startup-reports.md), [005](../tasks/adr-0066-task-005-session-drain-and-resumption.md), [006](../tasks/adr-0066-task-006-configuration-repair-and-resumption.md) |
| Safe full-copy reports, multi-issue notice and recorded UTC | [002](../tasks/adr-0066-task-002-core-checks-and-startup-reports.md), [003](../tasks/adr-0066-task-003-runtime-failure-and-shell-availability.md), then each tool's VM tests |
| Shared owner/token/VM paths and original architecture guard intent survive | Every packet; actual guard names recorded in its completion evidence |

## Packet Review Procedure

1. Check the file inventory and live caller. Reject uncalled support modules,
   unrelated cleanup, new config keys or unplanned schema changes.
2. Read the actual enum/error/resource types and caller dependencies. Check
   command and query execution as well as displayed availability.
3. Inspect behavior tests at the failure boundary. Require actual WAL/SQLite
   contention and converter process tests where specified, not only source
   assertions or mocks of the claimed behavior.
4. Verify original data before/after failures, including partial operations.
   Reject claims that a failed startup proves rollback.
5. Trace drain and resumption through all connection/actor owners. Dropped
   callbacks and a zero UI activity indicator are not completion evidence.
6. Check report purpose and privacy. A user should identify what acted on
   which resource, what happened, what remains unknown, and what to do next.
7. Read existing guards that change. Preserve their still-live rules; retire
   only superseded claims and name the deciding ADR. Keep the original
   workspace persistence, runtime separation, and task 017 recovery rules.
8. Remove duplicated procedure prose when actual guards land. Task 001 reviews
   the handoff; each successor owns its own retirement. Leave binding ADR
   decisions/invariants intact.
9. Check test commands and outcomes. The lint gate is cargo clippy -- -D warnings.
   No agent GUI run or widened all-targets cleanup belongs in acceptance.
10. Reconcile packet Status, plan row, ADR partial line, delivery row, AGENTS.md
    and any pending operator gate. Do not manufacture a gate for future tooling.

## Operator Acceptance Recording

Each UI packet supplies its own fixture/runbook section before its human check
opens. Record the date, built revision, exact section/case, operator result and
any remaining concern in that packet. A screenshot verifies only the visible
claim; use fixture inspection for bytes, row counts, migration records and
process activity.

Do not re-open ADR 0059 task 017's accepted gate or repeat already accepted
recovery layout merely because another maintenance command is added. Add a
regression check only when an affected owner changed or a concern remains.
Never close a visual gate from mechanical results.

Task 002 has full operator acceptance recorded below. Task 001 requires no
visual check.

## Task 001 Review — 2026-09-10

Scope: [config snapshot and safe persistence](../tasks/adr-0066-task-001-config-snapshot-and-safe-persistence.md#implementation-and-proof).
Green: formatting, cargo check, full cargo test and strict production Clippy.
The suite passed 1,275 unit tests and 221 architecture tests; 10 existing
documentation examples are ignored. No app launch or visual acceptance.

- C1–C5: snapshot, sibling-field, safe-diagnostic, filesystem-failure and
  all-three-save tests are linked in the packet. The injected reader proves one
  observation despite a backing-file change. Real first-run competitors share a
  publication barrier; permission tests include both injected denial and Unix
  file permissions. Byte assertions cover refused ordinary saves.
- C6: reviewed the ADR and all thirteen packet owners against the handoff.
  Every remaining mechanism has a named owner. Tasks 002–013 each retain the
  documentation-proof criterion and explicit prose-retirement constraint.
  Task 013 retains ADR 0016's shared migration registry/ledger, including
  candidate-copy repair; no second schema-mutation authority is introduced.
- The packet's implemented steps and coding prompt were retired in favor of
  live symbols and passing guards. ADR decisions/invariants remain unchanged.
  The existing ADR 0046/0051 guards were updated to the new decoding owner;
  their layout fallback and persistence rules remain guarded.
- Invariants 3–4 have task 001 proof, with explicit correction and complete
  caller migration still assigned to 006 and 004. Core probes, recovery UI,
  optional-tool isolation and database tools remain later work.
- Packet, phase plan, ADR partial line/index, delivery/deferred indexes,
  docs index and AGENTS.md agree: 001 complete; 002 next; series Accepted.
  Pending human checks are unchanged. No deviation from the bounded scope.

## Task 002 Review — 2026-09-10

Scope: [core checks and startup reports](../tasks/adr-0066-task-002-core-checks-and-startup-reports.md#implementation-and-proof).
Green: formatting, cargo check, strict production Clippy, build, 1,299 unit
tests and 224 architecture guards. Ten existing documentation examples are
ignored. The full suite required local sockets outside the sandbox; no source
or assertion changed to accommodate that restriction.

- C1/C2: actual music probes, permission cases, SQLite locks and rollback
  tests cover usable storage. Existing files and migration records survive
  check-only and failed checks. Missing storage is not recreated for existing
  configuration. Missing/older databases need preparation; Check again does
  not initialize or migrate them. Artists-subtree failures remain notices.
- C3/C5: the independent worker admits one operation. Closed receivers do not
  abandon its work. Bootstrap joins it after the desktop loop exits. Generation
  and normal-factory tests reject stale, closed and repeated completions.
  No normal TopApp, runtime or ordinary autosave exists in core recovery.
- C4/C6: recorded UTC, complete paths, full-copy text and credential omission
  are tested in the view model. Shared report geometry uses named tokens and
  typed actions; the screen only dispatches and mounts. Window failure uses
  stderr and exit rather than another recovery window.
- The SQLite implementation lives in db/startup.rs, under the existing db
  owner. Its column contract is checked against the normal migration registry;
  it adds no second schema writer. The existing metadata-table ownership guard
  now recognizes this child module and still rejects other owners.
- Existing workspace persistence, presentation/runtime separation, macOS menu
  and Show recovery guards remain Green. The screen inventory includes the new
  recovery screen. The packet's implemented procedures were replaced with
  actual guard references; the ADR's decision and invariants are unchanged.
- Fixture setup, locate/verify, all six modes, inspection and cleanup passed.
  Rust owns fixture schema construction; Python owns isolated files, stubs and
  the lock process. The seed/inspect dispatcher is debug-only. No GUI was run.
- Optional constructors, the second endpoint read and path-repair error policy
  retain their 003/004 handoff. Editors, session drain and database tools remain
  later packets. This is not full implementation of ADR 0066.

Result: mechanical gate Green; [operator acceptance](../runbooks/startup-recovery-check.md#task-002-core-checks-and-reports)
recorded - 2026-09-10. All visual checks passed and fixture cleanup is confirmed.
Packet, plan, ADR/index, delivery/deferred indexes, docs index and AGENTS.md
agree. The pending index no longer lists this accepted gate. Task 003 is next
in a fresh session; inherited checks and task 017's acceptance are unchanged.

Operator evidence received - 2026-09-10: the invalid-TOML report and preservation
inspection pass their text/data checks. The report names its path and parse
location with recorded UTC; original config/music and migration records remain
intact, with no probes. See the packet's
[operator evidence](../tasks/adr-0066-task-002-core-checks-and-startup-reports.md#operator-evidence--2026-09-10)
for fixture identity and acceptance details. The operator confirmed the missing-folder error, its
persistence after Check again, and unavailable Open app on a fresh verified
fixture. The operator then confirmed preservation and no residual probes after
Quit and inspection. Both missing-folder and file-path cases are now complete,
including preservation. The Check again feedback correction is operator-accepted:
the VM retains a numbered UTC completion outside disclosure
and scrolling. Tests cover identical failures in the same second, stale results,
copy text and worker failure. The [focused recheck](../runbooks/startup-recovery-check.md#3a-confirm-repeated-checks)
passed for advancing check numbers, visible completion with details hidden and
copied feedback. The locked-database case also passed: visible Checking during
the wait, a named lock failure, responsive window and unavailable Open app.
After lock release, the operator's report confirms both core checks passed.
The operator then reported a window-manager close panic in GPUI 0.2.2's
X11Client::with_common (exit 101). X11Client::handle_event holds its mutable
client borrow while calling should_close; bootstrap's inline App::quit tried
to borrow it again. App::defer also flushes inside that callback's App update.
The presenter now queues Quit on the foreground executor, after native event
dispatch. `adr_0066_window_manager_close_queues_quit` guards the call boundary
and retained worker/exit wiring. No GPUI dependency change or panic suppression
was introduced. The operator confirmed the normal-window recheck: Music opens
and closing through the window manager exits with code 0 and no panic.
Normal-session preservation also passed, including config/music/migrations and
no residual probes. The operator also passed normal/narrow long-path readability,
complete copied text, blocked recovery window-manager closing with exit 1 and
no panic, and final preservation. The supplied report is timestamped 21:58:04
UTC. The final operator confirmation accepts the TOML screen/control/disclosure
and nonzero Quit exit, and same-window resumption after releasing the database
lock. All operator checks are accepted. The operator confirmed fixture cleanup
on 2026-09-10; task 002 is complete. Task 006 retains
direct path correction and records the separate
new-setup design question.

## Task 003 Review — 2026-09-10

Implementation revision: `beb2ed4` contains task 003, including its scoped
runtime/cache recovery and search-error reporting correction. Its commit subject
names only the correction; this record identifies the full packet for traceability.

Scope: [runtime failure and shell availability](../tasks/adr-0066-task-003-runtime-failure-and-shell-availability.md#implementation-and-proof).
Green: formatting, cargo check, strict production Clippy, build, 1,314 unit
tests and 227 architecture tests, including the search-error correction. Ten existing documentation examples remain
ignored. The full suite used local socket fixtures outside the sandbox.

- C1/C2/C6: failed runtime construction leaves an explicit unavailable runner.
  Dispatch returns a typed error before execution or success events. The guard
  inventories toolbar, keyboard and shared command paths; existing independent
  library queries still work. No implicit runtime or UI-thread fallback was
  added. The retained standalone search constructor has the same correction.
- C3: the observation collection retains independent runtime/cache results.
  The VM supplies recorded UTC reports and typed Configure/Check/Copy actions.
  Music/Show notices and Settings use one shared composite. A restored runtime
  does not claim that an external service answered.
- C4: the independent maintenance worker creates a usable runtime without a
  caller Tokio context. VM tests reject duplicate and stale completions; the
  architecture guard checks generation admission before host installation and
  the existing single-host/actor/bridge boundaries. Repeated identical failures
  retain distinct numbered completion receipts.
- C5: injected worker and actual prune failures preserve usable disk/hot-cache
  behavior and capacity. Reports name the cache path. Thumbnail fetches that
  cannot dispatch do not become stuck in Loading.
- The Show VM now distinguishes an unavailable query from confirmed idle
  playback. The ADR 0060 cached-projection guard replaces its old idle initializer
  assertion with this typed projection; its cached projector and invalidation
  assertions remain. All runtime/GPUI, startup and workspace guards remain Green.
- Fixture setup, verification, runtime-only, cache-only and combined failure
  modes, normal mode, inspection and cleanup are Green. Configuration, music
  and migration records remained intact, with no residual probes. Injection is
  debug-only and requires a matching fixture/configuration/running binary.
  The agent did not launch the GUI.
- Packet procedures and the coding prompt were retired for actual proof
  references. ADR 0066 remains Accepted. Optional configuration, player/producer
  isolation, secondary reads and path-repair policy remain task 004; no later
  packet was started.

Result: mechanical gate Green; [operator acceptance](../runbooks/startup-recovery-check.md#task-003-background-tools)
open. Packet, phase plan, ADR/index, delivery/deferred indexes, docs index,
AGENTS.md and pending checks agree. Task 002's accepted core checks and inherited
gates remain unchanged. No deviation requiring a new architectural decision.

Operator evidence - 2026-09-10: startup/navigation with simultaneous runtime and
thumbnail failures passed. Both failures and their Check again controls were
reachable in Settings. Repeated-check feedback also passed: counts increased and
timestamps updated after immediate failures. Checking did not remain visible;
this immediate fixture does not require an intermediate rendered frame. Full
report copy passed: the supplied check-12 report records distinct runtime/cache
failures, UTC times, affected work, recovery actions and the complete cache path
for fixture `/tmp/v4vmm-startup-clzmrchl`. Narrow Settings report wrapping and
reachable controls passed, as did Show's unavailable/Not checked wording.
Search-button and Enter dispatch also passed, with a clear runtime failure and
usable navigation/repair controls. The packet records this limited acceptance;
local browsing subsequently passed. The window manager intercepted the Super
refresh/playback shortcuts, so their operator check remains unverified. Mechanical
dispatch coverage is separate from desktop key delivery. Independent runtime
recovery passed in the same window while thumbnail cleanup remained failed.
The subsequent screenshot proves post-repair remote dispatch but fails search
error readability: the full technical line clips at both edges. Playlist
uniqueness, thumbnail repair and preservation checks remain open; no window-manager
reconfiguration is required to continue them.

The focused search-error correction uses the existing shared search-results
shell, `MultilineText` wrapping, a named NoticeWidth token, and VM-owned diagnostic
disclosure/copy actions. `src/diagnostics.rs` shares the former startup URL
redactor; the ADR 0056 guard caught URL parsing in the VM, and moving it to that
helper preserves the guard unchanged. The packet links the three new behavioral
tests and `adr_0066_search_failure_report_stays_readable_and_vm_owned`.
The [focused visual recheck](../runbooks/startup-recovery-check.md#2a-recheck-search-failure-readability)
has an initial layout pass: the operator confirmed explanation wrapping and
reachable Show details/Copy report at normal and narrow widths after relaunch.
Expanded diagnostics and copy also passed, including stable time/report after
hide/show. The supplied 23:39:54 UTC report contains the endpoint and both
feed/track errors. The subsequent screenshots verify Library-filter separation
and exactly one Startup fixture playlist. The focused correction is fully
operator-accepted. Thumbnail repair subsequently passed in the existing window:
the screenshot records cleanup completion at 23:50:24 UTC, the cache path, and
the retained runtime startup result at 23:39:34 UTC without an external-service
health claim. The copied success report and `normal` preservation inspection
also passed: only workspace preferences changed, config/music/migrations are
preserved, one playlist remains and no probes remain. Fresh cache-only startup
subsequently passed: only thumbnail maintenance failed, all three sections
remained navigable, and the operator confirmed exit code 0 on shutdown.
Final preservation inspection also passed in `cache-worker-unavailable` mode:
config/music/migration records are preserved, only workspace preferences changed,
one playlist and migration versions 1–11 remain, and no probes remain.
The operator confirmed fixture cleanup. Only the intercepted Super shortcuts
remain unverified; other operator checks are accepted.
The operator subsequently requested standard Linux Ctrl bindings; the focused
[ADR 0067 check](../tasks/adr-0067-task-001-platform-shortcuts.md#operator-visual-check)
owns actual refresh/playback key delivery and rejection acceptance.
No operator approval is inferred from passing mechanical tests.

## Settings Responsiveness Acceptance — 2026-09-11

The operator confirmed responsive Settings navigation and complete, readable
text after four measured visits at 02:00:18–02:00:22 UTC. Layout took
22.163–24.371 ms and the following frame callback arrived after 37.045–50.274 ms.
The temporary timing module/frame wrapper and fixture profiling command are
removed; optimized debug text dependencies and the packet's manual regression
check remain. This closes the speed/readability gate. First-press Ctrl+2/3/1
section navigation subsequently passed before any in-app click and after
leaving the Settings endpoint input. Ctrl+Comma opened Settings and Ctrl+F
moved focus from its endpoint input to toolbar search; both passed. Toolbar text
selection, copy/cut/paste, undo and word movement passed without navigation or
playback. After a fresh unavailable-runtime launch, Ctrl+R reported unavailable
background tools, finished loading and preserved navigation. Refresh rejection
passed. Ctrl+Alt+P subsequently reported playback failure due to unavailable
background tools in Settings, preserving navigation and repair; the operator
confirmed pass. The cached-file unavailable message then passed: it explained
the failed read, pointed to Background tools and did not claim an empty list.
The operator then changed the fixture to normal, retried Background runtime and
confirmed No cached files appeared without leaving Settings. Cache recovery
passed. Ctrl+Q then closed the app with fixture exit code 0; the operator
confirmed pass. Final normal-mode inspection passed: config and music are
preserved, only workspace preferences changed, migration versions 1–11 remain,
one playlist remains and no database/music probes remain. The operator then
confirmed the cleanup command's Removed fixture message. Tasks 0066/003 and
0067/001 are complete; their pending-human-check entry is removed. Task 004 is
ready for a new session and has not started.

## Final Series Review

ADR 0066 remains Accepted until all thirteen packets and their operator gates
are complete. Confirm the following before changing it to Implemented:

- All invariants and transferred requirements above have actual evidence.
- No expected startup failure remains as an unclassified panic; programmer
  invariants remain distinct.
- Invalid optional settings do not reappear as core failure through a secondary
  reader, and a missing runtime cannot construct an implicit runner.
- Repair and retry are useful in both normal Settings and core recovery.
- Migration repair uses the normal registry and recording path.
- No unsafe pathname replacement or empty database fallback exists.
- All remaining limitations are stated in reports and completion evidence.
- Deferred item 6 and the config-format dependency are reconciled; unrelated
  inherited human gates remain accurately indexed.
- The next session is selected from the canonical delivery order.

## Review Result Format

Report pass/fail, required fixes, optional improvements, whether this packet is
ready to merge, and whether its successor needs a bounded amendment. Distinguish
a mechanical pass from pending operator acceptance.


## Task 004 Review — 2026-09-11

[Implementation evidence and caller inventory](../tasks/adr-0066-task-004-optional-tool-isolation.md#implementation-and-proof)
cover C1–C7. The strict Config and BroadcastConfig adapters are gone. The empty
whole-config caller inventory is guarded. Optional resources and presentation
siblings remain independent; prepared-player availability reaches Show and
playlist displays as well as keyboard/command dispatch. Polling starts after a
successful playback command, preserving lazy startup without editing queue data.

The existing persistence and runtime guards follow the new owners. A GPUI
re-entrancy guard caught a draft playlist projection; final wiring passes typed
availability through the display input. Full tests and required build/lint/format
checks are Green. Backend fixture checks retained config/audio, all tracks and
playlist entries, migration versions and the first committed path update.

The presentation case has operator acceptance and Green preservation inspection.
[Task 004's runbook](../runbooks/startup-recovery-check.md#task-004-optional-tool-isolation)
still owns the first case's remaining visual confirmation, producer failure
with audible playback, publisher isolation, partial path repair, their remaining
preservation checks and operator cleanup. The operator subsequently authorized
task 005 before these remaining checks; that exception accepts none of them.
Tasks 001–003, ADR 0067 and the five inherited check groups retain their prior scope.

The operator's presentation screenshot exposed configuration warnings duplicated
between the shared notice and Show status. The correction projects residual
startup status through `normal_startup_status`; configuration notices retain
their capability-report owner. Factory tests retain all three reports, preserve
a separate download-directory notice and verify unchanged config/blocker bytes.
The live call is covered by `adr_0066_optional_dependencies_are_scoped`.
The operator supplied the presentation inspection and stated "pass" on
2026-09-11, accepting the screenshot correction and the requested resize/navigation
checks. The presentation recheck is closed in task 004 and removed from the
pending index. The packet's remaining checks stay open.

The operator's subsequent producer screenshot contains an mpv IPC read error;
audio and preservation are not accepted from that image. The operator also
rejected the runbook's Music Play route: a playlist must load into the Show cue
before Show starts it, and other playback buttons must audition independently.
The current commands share one playback owner and can publish drop-file metadata
from Music Play. The runbook now pauses its playback-dependent portions pending
that workflow decision and implementation. C2/C5's mechanical isolation proofs
do not establish cue/audition separation. Keep these checks open as recorded in
[task 004](../tasks/adr-0066-task-004-optional-tool-isolation.md#playback-workflow-correction--2026-09-11);
the accepted presentation case is unaffected.

The operator then supplied the `producer-unavailable` inspection. Preservation
is Green: only permitted workspace preferences changed in configuration; music,
bindings, library data, migrations and the producer blocker remain preserved,
with no residual probes. This closes that case's preservation check only.
Playback acceptance remains paused and final fixture cleanup remains open. See
[the recorded inspection](../tasks/adr-0066-task-004-optional-tool-isolation.md#producer-preservation-inspection--2026-09-11).

## Task 005 Review — 2026-09-11

[Implementation and proof](../tasks/adr-0066-task-005-session-drain-and-resumption.md#implementation-and-proof)
cover C1–C5. The live Diagnostics action uses typed view-model state and shared
maintenance forms. The command runner, five live desktop actor paths, thumbnail
worker and runtime share a session owner. Configured connection release is
proved both through actual ownership transfer/close and the paged actor's
separate SQLite connection test. A dropped command receiver remains tracked.

Teardown and abandoned preparation resources are released by the independent
maintenance worker. The owned-child test verifies that shutdown reaps only the
app's child; the transition sends no external service stop command. A real core
prepare/drain/recheck/resume test verifies a larger generation, failed-check
recovery and configuration preservation. Existing presenter and view-model
tests cover single mounting and stale-result rejection.

`adr_0066_core_maintenance_drains_the_session` guards the live ownership route
under ADR 0066 invariants 5–6. The packet's implementation recipe and coding
prompt are replaced by the actual proof inventory. Full tests, production
Clippy, check, formatting and build are Green. The fixture's held/released
marker, status, preservation and cleanup commands passed without starting GPUI.

The [operator check](../runbooks/startup-recovery-check.md#task-005-session-drain-and-resumption)
passed V1–V3, final preservation inspection and fixture cleanup on 2026-09-11. The
operator's explicit scheduling exception lets task 005 use task 004's delivered
owners; it does not accept task 004's paused playback workflow or remaining
checks. Task 006's separate configuration editor and acceptance are recorded
below. No configuration editor, database installation or configuration-format
change is included in task 005.

The [operator evidence](../tasks/adr-0066-task-005-session-drain-and-resumption.md#operator-evidence--2026-09-11)
records fixture `/tmp/v4vmm-startup-oaq_8a7x`. Session 1 exceeded the fixture's
ten-minute hold deadline before drain, so only the held-work check was repeated.
Session 2 retained `FixtureSessionCommand: 1` through failures recorded at
18:40:14 and 18:44:08 UTC, then reached recovery at 18:45:06 after release.
The copied report retained both failures and the original timestamps. Terminal
JSON confirmed `command-held → command-released → maintenance` for session 2,
followed by exactly one session 3 `opened`, with `held: false`.

The operator accepted the library, reports, keyboard access and applicable
theme/width checks. Preservation inspection retained music, bindings, library
and migrations 1–11 with no residual probes; only permitted workspace preferences
changed in configuration. The operator confirmed the fixture's removal. This
closes task 005's gate and removes its entry from the pending-human index.

## Task 006 Review — 2026-09-11

Mechanical review: Green. [Implementation and proof](../tasks/adr-0066-task-006-configuration-repair-and-resumption.md#implementation-and-proof)
map C1–C5 to the live correction backend, commands, shared VM/composite/input
entity and existing session lifecycle. Original-byte backups, source/link/target
revision conflicts, invalid siblings, absent/unprepared path rejection and
current-generation save/check admission have behavioral proof. The extended
bootstrap test closes old handles, corrects both core paths, rejects stale Open
consent and prepares a fresh session on the selected resources.

`adr_0066_shared_guarded_config_repair` owns the call boundary under invariants
3–6 and 9. The original persistence guard remains. Ordinary Settings saves reject
music-root changes; the existing Settings width/default guards retain their
optional-field rules and follow the new core maintenance route. No normal
connection or player is retargeted to a newly saved core path.
`ConfigWriteLease` excludes overlapping ordinary/correction writers for the same
resolved destination without waiting for a writer; a cross-thread test proves
rejection, independent destinations and release. Settings exposes typed busy
state, and its existing appearance preview setters follow saved field values.

Green: formatting, check, strict production Clippy, build, 1,361 unit tests and
239 architecture tests. Ten existing documentation examples remain ignored.
The six fixture modes, access restoration, conflict command, preservation
inspection and owned cleanup passed without opening GPUI. The fixture's access
restoration avoids changing the file revision when only directory permission
repair is needed. Existing docs were updated; none were created or moved, and
canonical root instruction files stayed in place. Changed-document relative
links were checked.

The packet's implementation recipe and coding prompt were replaced by actual
proof references. Optional reinitialization and original-action retry remain
task 007; complex focused fields use a TOML value assignment. Malformed draft
copy returns an explanation until syntax allows credential redaction. These
limits are stated in the packet and operator procedure.

Task 006 acceptance is complete on 2026-09-13.
[Operator V1–V6](../runbooks/startup-recovery-check.md#task-006-configuration-repair-and-resumption),
preservation inspections and fixture cleanup are accepted. The operator's
viewport correction is covered by the shared geometry owner and the runbook's
normal/narrow-width regression check. The V4 fixture inspector correction has
six situational ADR 0066 tests, including rejection of unrelated configuration
changes and preservation failures in other cases. That new Python test file
lives beside the existing fixture helper; no Markdown file or documentation
folder was created or moved.

The packet, phase plan, ADR, delivery/deferred indexes and AGENTS.md record
completion. The closed task 006 section is removed from the pending-human index.
Task 007 implementation and acceptance are recorded below. No new architectural
decision or cross-repository change was needed; task 004 and inherited checks
retain their previous scope.

Closure checks - 2026-09-13: Green. All 239 architecture tests and six fixture
regression tests pass. The changed-document file links and whitespace checks
are Green. The task 006 gate is closed in every current status reference;
ADR 0066 remains Accepted for the remaining packets and task 004 acceptance.


## Task 007 Review — 2026-09-15

Mechanical review: Green. The [proof inventory](../tasks/adr-0066-task-007-optional-tool-correction-and-retry.md#implementation-and-proof)
maps C1–C5 to live owners and actual tests. Retained actions keep their original
queries, track/playlist/queue, event selection and publisher/encoder context.
Save invalidates prior verification and queues only changed-tool setup. Retry
requires fresh verification and checks the current configuration, app session
and original subject at command execution; playback/event checks repeat under
the command's existing database lock. Attach/Detach payloads must match the
retained event, target, transport and publisher instance before external I/O;
`adr_0066_target_retry_rejects_a_new_target_before_transport` guards changed
subjects. Changed/removed subjects are rejected.

The situational invariant 9 guard
`adr_0066_repair_routes_preserve_action_subject` replaces the packet's duplicated
recipe. Entry-point guards trace actual toolbar, Enter, playback and Show wiring
to the shared owners; behavior tests exercise those owners. Renderers do not
replay actions or derive availability. The shared Settings/editor, report frame,
control styles and tokens own the new presentation path. Its desktop proof is
V1–V3 and remains open.

Targeted preparation preserves unrelated resource handles and observations.
Configuration admission excludes competing in-app saves through setup/install
and Retry. A missing player does not prevent checking the independent producer
configuration. Loaded players refuse replacement; busy Show tools report a
pending manual check, and failed setup cannot leave an old tool marked ready.
The existing runtime is reused. Publisher/encoder changes restart only their
existing shared observation watch, with stale callback rejection. No service is
started by Save or Check, and no core database is replaced.

Green: repository formatting, check, strict production Clippy, build, 1,409 unit
tests, 251 architecture guards, and fourteen Python fixture regression tests. Existing
HTTP tests require local socket access and passed with it. The fixture's CLI
smoke verified its loopback request records, passive service reads, explicit
stub mutations, preservation, case restoration and owned cleanup without GPUI.
The operator still needs to inspect their own running fixtures and accept
preservation/cleanup. Ten existing documentation examples remain ignored.

No new architectural decision or cross-repository change. Converter execution,
retained-download retry and database maintenance remain in 008–013. Unknown
original publisher/encoder context requires a new action after repair; Retry
never guesses it. Task 004's paused playback and inherited checks stay open.
Current status references, the delivery order and pending-human index agree.
Existing documentation was updated in place; no Markdown file was created or
moved. The canonical root instructions remain in place.


### V1 Dependency Explanation Follow-Up

The operator screenshot records both original queries separately, with the
second query selected. Its search explanation exposed a dependency mismatch:
all unavailable errors were presented as background-runtime failures. The
search failure VM now distinguishes endpoint setup and runtime failure;
`adr_0066_search_endpoint_failure_does_not_blame_the_runtime` covers visible and
copied text plus the retained runtime explanation. Mechanical checks are Green.
The [packet follow-up](../tasks/adr-0066-task-007-optional-tool-correction-and-retry.md#v1-screenshot-follow-up)
records the evidence and scoped recheck. V1 Save/Retry and the remaining operator
gate stay open.


### V1 Control Clarity And Completion Follow-Up

The operator's next screenshots show the first query completed at 2026-09-16
01:00:33 UTC and the second query still pending. The copied correction report
records Save at 00:58:27 UTC with an original-byte backup and only the unrelated
converter/player issues remaining. The operator reported unclear Repair action,
Check again and Retry controls across Music and Settings.

The shared VM now supplies tool/operation names and visible effect descriptions.
The shared composite renders those descriptions with named caption/color/spacing
tokens. A ready Music entry opens its action controls in Settings. Successful
completion has a typed result and offers only Dismiss on both surfaces. Later
checks preserve completion and its recorded time. A failed event check remains
unsuccessful even when its command transport returns a typed result.

New behavioral tests cover edit/check/run meanings, service observation versus
mutation, completed-action admission, and failed-event completion. The new
situational ADR 0066 guard
`adr_0066_recovery_controls_explain_effect_and_completion` checks shared ownership,
non-executing Settings navigation and completion recording in the existing
adapters. Formatting, check, Clippy, build, 1,409 unit tests and 250 architecture
guards are Green. The GUI was not run. Subsequent wording/help, width/theme and
report-copy, Music ready-action navigation and completion acceptance is recorded
below. The operator's request history confirms
only the first query's two requests after Save and no service mutations.
Remaining preservation, cleanup and V2–V3 need acceptance. Earlier screenshot
evidence is retained.

The operator's expanded V1 preservation report isolates the failure to endpoint
whitespace/trailing slashes. Original bytes, private backup permissions and all
other configuration checks pass, with no unexpected field paths or candidates.
The inspector now accepts this spelling of the exact fixture URL, matching
`normalize_musicindex_endpoint`. It still rejects different schemes, hosts,
ports, paths, credentials, queries and fragments. It reports checks, field paths
and candidates without printing configuration values or changing evidence.
Type-aware comparisons also reject boolean/integer substitutions in unedited
values. Fourteen Python regression tests are Green. The full operator rerun for
`/tmp/v4vmm-startup-o77im_r8` is Green: private original backup, unedited settings,
music, library, bindings and migrations are preserved, with no residual probes
or candidates. This fixture's preservation is accepted. Cleanup is unconfirmed.
V1 presentation is accepted below; V2–V3 remain open. The
runbook separates inspection from cleanup so a failed inspection does not lead
directly to fixture deletion.

### V1 Narrow-Width Follow-Up — 2026-09-16

The operator's narrow Settings screenshot confirms the completed query's
Dismiss-only state but fails the pending-row layout. Four action buttons leave
the query timestamp one character wide; two converter buttons also crowd their
explanation. The shared `capability_report` owner now gives text an existing
`Size::ColumnRegular` preferred basis and puts actions in a bounded wrapping
group. The same owner renders Music and Settings. VM state, labels, controls
and token roles remain shared. V1 step 5 is the blocking situational ADR 0066
manual guard for two-/four-button rows at about 560 pixels, normal widths and
both themes. Formatting, compilation, strict Clippy, build, 1,409 unit tests and
250 architecture guards are Green. Existing HTTP/IPC tests passed with local
socket access; the stalled sandboxed attempt was stopped. The operator accepted
the corrected narrow Background tools converter and pending-query rows on
2026-09-16. The operator subsequently reported pass for Settings Diagnostics and
Music notices at normal/narrow widths in Light/Dark, readable text, reachable
buttons and report-copy matching of query names and UTC times. No copied report
artifact accompanied the confirmation. These V1 presentation checks are accepted;
Music's ready-action navigation and completed-state acceptance follows below.

### V1 Search Navigation Follow-Up — 2026-09-16

The operator showed search results with Settings still selected and present in
the breadcrumb. The shared search helper now selects Music through the existing
section transition before changing the frame's search history. Button, Enter
and saved searches reuse that helper with a window context. Empty input leaves
the section unchanged. Settings draft ownership is unchanged. The situational
ADR 0060 guard `adr_0060_search_uses_music_section_navigation` protects that route
and ordering. V1 now checks button and Enter submissions from Settings; Music
selection and breadcrumb history must agree with the content. The operator
accepted this navigation check on 2026-09-16 and subsequently accepted the narrow
Background tools row recheck.
Formatting, compilation, strict Clippy, build, 1,409 unit tests and 251
architecture guards are Green. Existing HTTP/IPC tests ran with local socket
access. No GUI was run by the agent.

### V1 Behavior And Presentation Acceptance — 2026-09-16

The operator accepted View search actions opening Settings without running a
search, explicit retry of the first query, and Music's completed notice retaining
only Dismiss after checking the second query. The checkpoint included requests
only for the first query and no service mutations. This is operator confirmation;
no new request-log artifact was supplied. Earlier supplied request logs remain
recorded above. V1 behavior and presentation are accepted. The operator then
reported pass for the current `layout_fixture` preservation inspection. This
accepts original configuration preservation, private backups, unchanged
music/library/bindings and no residual probes by operator confirmation; no new
JSON or absolute fixture path was supplied. Cleanup and V2–V3 remained open at
this checkpoint. The final reconciliation below corrects the inferred extra
V1 fixture gate.

### V2 Entry Observation — 2026-09-16

The operator reached the fixture playlist at 463 pixels wide. The recovery
notices wrap, but Library's sidebar leaves the playlist heading one character
wide and hides track actions from the supplied viewport. The Library split and
playlist detail layout need correction and visual recheck; the accepted V1
notice layout does not cover this surface. No source correction is claimed.
After maximizing, the operator accepted the first track's Repair playback
action, then Edit player settings focusing `playback.driver`. This accepts V2's
entry route on 2026-09-16. The top-level Edit player settings notice does not
create that retained track action. Changed-position rejection, the narrow Library
layout correction and prior fixture cleanup remain open.

### V2 Draft Retention And Passive Correction — 2026-09-16

The operator accepted the focused `null` player draft surviving navigation to
Music, moving the original first track down one position and returning to
Settings. Validation, Save correction and the player check passed without loading
a track or starting playback. Endpoint and converter issues remained. This
accepts draft retention and passive player correction. Rejection of the retained
action's changed position, subsequent explicit Null-player action, preservation
and cleanup remain open.

### V2 Disabled Retry Observation — 2026-09-16

The operator reported Play original track greyed out after the correction
checkpoint. No changed-position rejection is accepted. Source inspection shows
Retry availability depends on a checked, idle retained action and no active tool
check; the playlist subject is validated at command execution. The disabled
button alone does not establish the cause. The next diagnostic is the retained
row's message and explicit Check player, followed by its report if readiness
does not recover. No source correction or verified root cause is claimed.

### V2 Saved Player Validation Failure — 2026-09-16

The operator supplied check 1's report for `/tmp/v4vmm-startup-4yqtadz3`:
at 16:54:21 UTC, playback setup rejected the saved playback settings. The action
retained at 16:27:46 UTC still names playlist 1, position 0, track 1 and requires
a successful setup check. This reopens the earlier player-correction acceptance;
focused editing and draft retention remain accepted. Retry has not executed.
The subsequent repair report confirms the same configuration loaded at 16:28:09
UTC, then Save at 16:46:41 UTC and Validate at 16:46:44 UTC rejected the proposed
driver. That attempt did not save a correction. The exact draft text remains
unknown; no source defect or fix is inferred. Require successful validation and
a Save report naming the backup, then successful player setup, before testing
the changed position. The fixture is not visible in the agent filesystem. Keep
it and the moved track.

### V2 Player Setup And Retry Readiness — 2026-09-16

The supplied check 3 report at 21:13:38 UTC confirms that Built-in playback
configuration and local setup passed and the tool was refreshed without retry.
The retained action still names playlist 1, position 0 and track 1; Play original
track is available in Settings. The driver issue is cleared, while endpoint and
converter issues remain. This accepts saved player setup, isolation and retry
readiness and resolves the disabled-retry checkpoint. The replacement Save
report/backup path was not supplied; fixture preservation remains open.
The subsequent changed-position rejection is accepted below; a new explicit
Null-player action still needs operator evidence.

### V2 Changed-Position Rejection — 2026-09-16

The supplied report and matching Settings screenshot record rejection at
21:16:07 UTC for playlist 1, original position 0 and track 1 after the track was
moved down. The command refused the changed subject without substituting the new
first track. Play original track is disabled after the failed attempt; Edit,
Check player and Dismiss remain available. Endpoint and converter issues remain.
This accepts the changed-position rejection. Restoring order, testing a new
ordinary Play with the Null player, preservation and cleanup remain open, as
does the separately recorded narrow Library layout defect.

### V2 Preservation And Drag Observation — 2026-09-16

The supplied fixture inspection accepts preservation for
`/tmp/v4vmm-startup-4yqtadz3`: original bytes in private backup
`.v4vmm-config-4147752-0.backup`, unedited values, all named configuration checks,
music, library, bindings, tool blockers and migrations 1–11 pass. Counts remain
one playlist, three tracks and three playlist tracks; bindings remain
a.wav/b.wav/c.wav. No unexpected settings, residual candidates, music probes or
database probes remain. Explicit Null-player Play confirmation and cleanup are
still open.

The operator reports a brief pause for drag-handle reordering while menu moves
work. The shared gesture conflict, correction and operator recheck belong to
[ADR 0044's review](adr-0044-review-checklist.md). Neither this preservation pass
nor the interaction test closes that inherited visual gate.

### V2 Drag Follow-Up Acceptance — 2026-09-16

The operator accepts the normal-build restart, playlist reordering and horizontal
out-of-bounds drag recheck after the fixture launcher correction. The accompanying
inspection repeats the preservation pass for `/tmp/v4vmm-startup-4yqtadz3`:
original and unedited configuration, private backup, every named configuration
check, music, library, bindings, tool blockers and migrations 1–11 pass. Counts
remain one playlist, three tracks and three playlist tracks; no candidates or
probes remain. This accepts the reported drag pause correction and post-check
preservation. Ordinary Null-player Play, fixture cleanup, the narrow Library
layout defect and V3 remain open. ADR 0044's separate theme-specific checks are
not inferred from this responsiveness pass.

### V2 Ordinary Play Acceptance — 2026-09-16

The operator accepts the separate new Play action on the original first track's
playlist row in `/tmp/v4vmm-startup-4yqtadz3`, after restoring playlist order.
The intended track loads without setup failure or substitution under the silent
Null player; endpoint and converter notices remain. V2 behavior is accepted.
The subsequently supplied preservation inspection after this final action passes;
fixture cleanup is subsequently confirmed below.
V3 and the narrow Library layout correction stay open. This check supplies no
audible-playback evidence for task 004 or cue/audition acceptance under ADR 0068.

### V2 Final Preservation Acceptance — 2026-09-16

The final inspection for `/tmp/v4vmm-startup-4yqtadz3` accepts original and
unedited configuration preservation, owner-only backup permissions and every
named configuration check. Backup `.v4vmm-config-4147752-0.backup` remains.
Music, library, bindings, tool blockers and migration records 1–11 pass. Counts
remain one playlist, three tracks and three playlist tracks; a.wav/b.wav/c.wav
bindings remain. No unexpected setting paths, residual candidates or probes
remain. Cleanup is subsequently confirmed below; no V3 acceptance is inferred.

### V2 Fixture Cleanup — 2026-09-16

The operator accepts the cleanup command's removal result and directory-absence
check for `/tmp/v4vmm-startup-4yqtadz3`. V2 behavior, final preservation and
cleanup are complete. Earlier V1 fixture cleanup is not covered by this pass.
V3, the narrow Library layout defect and the separate inherited gates remain open.

### V3 Failed Publisher Start — 2026-09-16

The supplied Show screenshot accepts the initial failed Start presentation:
at 23:05:03 UTC the retained action names Publisher, Local/default and original
event none. Its Edit publisher settings explanation states that the action is
not run by opening Settings. The fixture's systemctl exit-1 rejection appears
separately from the Publisher's Inactive observation for
`musicindex-live-publisher@default.service`. No show is active; endpoint,
converter and player issues remain visible. The subsequent report identifies
the fresh fixture below; the command-log baseline is still missing. Passive Save, changed-host
rejection, original-host retry, preservation and cleanup remain open.

### V3 Configuration Save — 2026-09-16

The configuration report identifies fixture `/tmp/v4vmm-startup-x1nt5sbv` and
records load at 23:11:08 UTC, successful draft validation at 23:12:09 UTC and
Save at 23:12:13 UTC. Save reports original backup
`.v4vmm-config-77402-0.backup` in the fixture configuration directory. Endpoint,
converter and player issues remain, with ordinary persistence paused. This
accepts validation and reported Save. The selected-host value, publisher setup
result and command log were not supplied, so passive Save and retry readiness
still need evidence. Backup-permission/preservation inspection and cleanup remain open.

### V3 Passive Setup And Retry Readiness — 2026-09-16

Publisher host check 2 at 23:18:47 UTC accepts saved setup and reports that only
service state was read. Start Publisher on Local/default, original event none,
remains retained and ready in Settings. The supplied fixture record contains
exactly the original `--user start musicindex-live-publisher@default.service`,
no Index requests, and `service_commands_enabled: false`. Save and Check
therefore added no service mutation. Passive setup and retry readiness are
accepted. Endpoint, converter and player issues remain. The subsequent
changed-host rejection is recorded below.

### V3 Changed Publisher Rejection — 2026-09-16

The supplied report at 23:21:50 UTC rejects the retained Start Publisher on
Local/default, original event none, because its original target is unavailable
or changed. The fixture record still contains only the initial
`--user start musicindex-live-publisher@default.service`, no Index requests, and
`service_commands_enabled: false`. No additional service command was sent and
none targeted the alternate instance. Changed-host rejection is accepted.
Unrelated endpoint, converter and player issues remain. Original-host retry,
unrelated-control availability, final preservation and cleanup remain open.

### V3 Original Publisher Setup — 2026-09-16

Publisher host check 4 at 23:29:02 UTC reports a service-state read, tool refresh
and no original-action retry following the instruction to restore Local. The
retained Start Publisher on Local/default, original event none, reports setup
passed and Start available. Endpoint, converter and player issues remain.
Two subsequent identical fixture records show service commands enabled, no Index
requests and only the initial default Start. Passive setup and readiness after
restoration are accepted. Subsequent original Start acceptance is recorded below.

### V3 Original Publisher Retry Acceptance — 2026-09-16

The operator reports pass for the retained Local/default original Start check:
exactly two default-instance Start records, no alternate-instance command or
Index request, successful command completion separate from the fixture's
Inactive observation, usable local Music browsing and retained unrelated
configuration issues. Acceptance is by operator confirmation; no new report,
command-log artifact or retry timestamp accompanied it. Subsequent independent
setup-tool access and preservation acceptance are recorded below.

### V3 Final Preservation And Independent Tool Access — 2026-09-16

The operator confirms that Edit converter setting opens the focused `flac_path`
editor and closes without changes. V3 behavior is accepted. The final inspection
for `/tmp/v4vmm-startup-x1nt5sbv` passes original and unedited configuration
preservation, every named configuration check, and owner-only backups
`.v4vmm-config-77402-0.backup` and `.v4vmm-config-77402-2.backup`. Only ordinary
workspace preferences differ in the final configuration. Music, library,
bindings, tool blockers and migration records 1–11 remain preserved, with one
playlist, three tracks, three playlist tracks and a.wav/b.wav/c.wav bindings.
No unexpected setting paths, residual candidates or music/database probes remain.
Final V3 preservation is accepted; cleanup is confirmed below. Earlier V1 gaps and the
narrow Library layout correction are not closed by this inspection.

### V3 Fixture Cleanup — 2026-09-16

The operator confirms the cleanup command and directory-absence check for
`/tmp/v4vmm-startup-x1nt5sbv`. V3 behavior, preservation and cleanup are complete.
Earlier V1 preservation/cleanup gaps and the narrow Library layout defect remain
open.

### Narrow Library Viewport Correction — 2026-09-16

ADR 0046 records the shared split-pane correction: reserve content width when
panes fit side by side, otherwise stack navigation above content with independent
scrolling and a resizable divider. Library supplies its allocated bounds and retains
width and height preferences independently in its view model. Named layout tokens own geometry.
The shared renderer test checks viewport/scale transitions, both pane bounds,
width restoration and divider behavior; the architecture guard covers the recent
content and selected-detail call sites. The dedicated task 007 runbook check
keeps visual acceptance and fresh-fixture preservation/cleanup open. Earlier
V1 fixture gaps remain; accepted V1–V3 behavior stays closed.

Verification is Green for five shared split-pane tests, all 252 architecture
guards, `cargo check`, formatting, required `cargo clippy -- -D warnings` and
normal desktop build. Extra all-target Clippy finds 41 pre-existing test-target
lints outside this correction (including float comparisons and test-module
ordering); that broader check is not Green. The normal binary was rebuilt after
architecture tests. No GUI was launched by the agent.

The next operator screenshot confirms stacked panes and a readable playlist
title at narrow width. It does not yet prove usable navigation or track actions:
the navigation viewport is very short, and the required rows/actions are outside
the shown scroll positions. Independent scrolling, readable/selectable rows,
width restoration, theme/scale checks and preservation/cleanup remain open.

The next screenshots and operator report accept independent scrolling and show
all three repair actions at narrow width. They reject usability of the fixed
stacked divider and show track text/actions overlapping at intermediate width.
The follow-up makes vertical resizing use measured viewport coordinates with a
separate VM height preference, wraps playlist controls at the shared shell, and
bounds normal-shell recovery notices while retaining all rows and a typed passive
Settings route. The visual recheck and fixture preservation/cleanup remain open.

Follow-up verification: Green — 1,415 unit tests, 253 architecture guards,
`cargo check`, formatting and required strict Clippy. The shared geometry tests
cover offset divider coordinates, narrow/intermediate playlist row bounds and
workspace height beneath multiple failures. View-model tests cover independent
pane preferences and issue/action counts; the Settings navigation guard forbids
checks, saves and retries in that route. The previously recorded extra all-target
lint limitation is unchanged. Operator visual acceptance remains open.
Normal desktop rebuild after the final regression run: Green. The operator
recheck reuses the existing narrow Library fixture.

### Library Pass And Show Overflow Follow-Up

The operator confirms pass for the requested Library recheck, including divider
resizing, readable rows at intermediate widths, bounded notices and Settings
navigation. The requested theme/scale and preference checks are accepted by that
confirmation; no additional screenshots or preservation results were supplied
for those steps. The follow-up fixture stays open.

The accompanying Show screenshot shows lower cards clipped with logs closed.
The shared log composite only created its scrolling card viewport in the
open-log branch. ADR 0073 records the narrow correction before implementation:
mount the shared viewport in both branches and keep log priority, sidebar,
transport, card geometry and command semantics unchanged. The focused
[Show check](../runbooks/startup-recovery-check.md#show-card-overflow-follow-up--adr-0073)
and follow-up fixture preservation/cleanup remain open. Prior V1–V3 and shared-log
acceptance stay closed for their accepted scope.

Show overflow mechanical verification: Green — 1,416 unit tests, 253 architecture
guards, `cargo check`, formatting and required strict Clippy. The new ADR 0073
renderer test reaches the last card with logs closed and open at two available
heights and confirms the transport stays fixed. Documentation links and
whitespace checks are Green. The focused operator gate remains open.
The normal desktop binary was rebuilt after the final tests: Green.

### Show Overflow Visual Acceptance

The operator confirms pass for the requested Show recheck: all three cards are
reachable/selectable with the fixture errors present, logs closed/open/closed
again, and the transport fixed while cards scroll. The requested theme/scale
checks are accepted by that confirmation. No additional screenshots,
preservation outputs or cleanup confirmation were supplied. Show and Library
visual gates are closed. Follow-up fixture preservation/cleanup and the earlier
V1 preservation/cleanup gaps remain open.

### Library And Show Follow-Up Preservation Accepted

The operator supplied passing `retry-inspect` output. Original configuration
bytes and unedited values are unchanged; no backup was created or needed.
All configuration checks pass. Music, optional-tool blockers, library records,
bindings and migrations 1–11 are preserved: one playlist, three tracks and
three playlist tracks with `a.wav`, `b.wav`, `c.wav`. No unexpected setting
paths, candidates, music probes or database probes remain. The false
workspace-preferences-only flag is immaterial because configuration bytes are
unchanged. Cleanup and directory absence are still unconfirmed. Earlier V1
preservation/cleanup gaps remain independently open.

### Library And Show Follow-Up Cleanup Confirmed

The operator confirms successful cleanup and directory absence for
`narrow_library_fixture`. The Library/Show follow-up is complete, including
visual acceptance and preservation. ADR 0073 is Implemented. Earlier V1 evidence
is reconciled below; task 008 has not started.

### Earlier V1 Evidence Reconciliation

The operator cannot reconstruct the earlier fixture history. Read-only enumeration
found no startup fixture directories in `/tmp` or `/var/tmp`; the known first
V1 directory `/tmp/v4vmm-startup-o77im_r8` is absent. This inventory performed
no cleanup. The first V1 and later layout preservation passes remain accepted.

The request log using port 44601 does not identify a fixture directory or prove
that it was separate from the later accepted layout fixture. The agent's
additional preservation gate relied on that unsupported inference and is
withdrawn. Historical identity and helper invocation details remain unknown;
no missing inspection is marked passed. The actual V1–V3 and follow-up acceptance
requirements are met, and no startup fixtures remain in the checked locations.
Task 007 is complete. Task 004's separate checks stay open; task 008 has not started.


## Task 008 Review — 2026-09-16

Scope: [converter verification and setup](../tasks/adr-0066-task-008-converter-verification-and-setup.md#implementation-and-proof).
Green: formatting, cargo check, 1,423 unit tests, 254 architecture guards,
production Clippy, normal desktop build and 18 fixture tests. Ten existing
documentation examples are ignored. The packet records the socket-restricted
attempt, transient executable-busy test observation and superseded guard
correction. No app was launched.

- C1/C2: actual subprocess tests establish same-process PATH refresh, configured
  path changes, FLAC-first encoding and permitted ffmpeg fallback. They cover
  missing, permission, exit, timeout/reaping and bounded-output outcomes.
  The version checker keeps no raw process output. Typed executable/source,
  exit facts and recorded UTC reach the VM's safe report.
- C3: the existing correction command, draft/revision owner and independent
  worker now admit TestConverter separately from Save. Tests prove no edit/Test
  writes, no Save probes, backup bytes, unchanged invalid siblings, stale result
  rejection, conflict preservation and fresh testing after Save. The existing
  task 007 entry route focuses this shared Settings/core-recovery editor.
- C4: `adr_0066_converter_checks_are_refreshable` is situational to ADR 0066.
  `settings_form_inputs_fill_scaled_frame_width` retains the ADR 0069 endpoint
  width proof and moves its replaced converter assertion to the shared editor
  under ADR 0066. Existing correction, runtime, token and presentation guards
  remain Green. No new configuration key or renderer-side process probe exists.
- The bounded process helper is a live audio-format child module. VM wording
  is a live startup child module. Library's converter entry uses the existing
  correction route, with geometry and tokens in the shared maintenance form.
  The narrow reused-WAV correction removes automatic PATH FLAC substitution;
  its existing FLAC prerequisite remains. No staging/download materialization,
  package automation, retained-track Retry or successor implementation was added.
- Backend fixture smoke passed both cases, seven controlled modes, strict
  preservation, PATH restoration and cleanup. The fixture permits no installed
  PATH converter to leak into missing-tool observations. Its inspector rejects
  non-version invocations, unreaped recorded children, unrelated settings and
  an un-restored configured FLAC path. This is mechanical evidence only.
- Implemented mechanism steps and the coding prompt were retired from task 008;
  the packet names actual tests/guards and runbook anchors. ADR decisions remain
  binding. Task 007 remains complete. Task 008's runnable V1–V3,
  Settings/core-recovery presentation, operator preservation and cleanup were
  held open for the operator walkthrough; final acceptance is recorded below.

Result: mechanical gate Green. Operator acceptance and cleanup completed on
2026-09-17, as recorded below. Task 009 has not started; no phase is chained into
this session. Dated operator notes record intermediate states; the final
acceptance closes their temporary pending gates.


### Task 008 Operator Follow-Up — 2026-09-17

Fixture `/tmp/v4vmm-startup-2qpie3af`: the supplied repair report records both
PATH converters missing at 10:49:27 and 10:55:46 UTC, then succeeding with exit 0
at 10:57:12 UTC after working stubs were enabled in the same app. Earlier results
remain readable; fallback wording and supplied copy content are correct.
The operator's subsequent "pass" confirms Settings responsiveness, visible
installation guidance and the requested unchanged configuration/probe-log hashes
after editing the explicit missing FLAC path without Test or Save.

V1 is accepted; V2's edit-only check passes. V2's remaining Test/Save/changed-path
checks, V3, remaining Settings/core-recovery presentation, preservation and
cleanup stay open. The packet, delivery row, phase plan and pending index retain
those gates. Task 009 has not started.


### Task 008 V2 Explicit Path And Fallback — 2026-09-17

The operator's supplied report records FLAC missing at the configured fixture
path `/tmp/v4vmm-startup-2qpie3af/missing-flac` and PATH ffmpeg succeeding with
exit 0, both at 11:02:45 UTC. The report preserves the existing WAV download
fallback and rejects silent PATH FLAC substitution. This V2 Test passes.
Guarded Save/backup/no-probe verification, changed-path Test/Save and restoration
remain open, along with V3, remaining presentation, preservation and cleanup.


### Task 008 V2 Guarded Save — 2026-09-17

The operator supplied the Save report recorded at 11:05:06 UTC and matching
before/after hashes. The fixture's `.v4vmm-config-237986-0.backup` matches the
configuration before Save, and the converter invocation log is unchanged.
The packet records both full SHA-256 values. Save reports no operation retry,
and the unrelated invalid MusicIndex setting and persistence pause remain.
This accepts V2's original-byte backup and no-probe Save checks. Changed-path
Test/Save and restoration, V3, remaining presentation and final fixture
preservation/cleanup remain open.


### Task 008 V2 Changed Configured Path — 2026-09-17

The operator's supplied report records reload at 11:22:12 UTC and successful
FLAC/ffmpeg checks at 11:22:55 UTC. FLAC uses the new configured executable
`/tmp/v4vmm-startup-2qpie3af/bin/flac`; ffmpeg uses PATH. Both exited with code 0,
with the earlier missing-path report retained. The changed-path Test passes.
Saving/reloading the working path, its preceding-revision backup and restoration
remain open, followed by V3, remaining presentation and preservation/cleanup.


### Task 008 V2 Working Path Save/Reload — 2026-09-17

The supplied repair report records Save at 11:24:50 UTC with backup
`.v4vmm-config-237986-2.backup` in the same fixture, then reload at 11:25:06 UTC.
The operator's "pass" confirms the requested preceding-configuration/backup hash
match, retention of the earlier backup and display of the saved working FLAC
path after Reload. Changed-path Save/reload and preceding-revision preservation
pass. Unset-path restoration remains the final V2 step; V3, remaining
presentation and final fixture preservation/cleanup stay open.


### Task 008 V2 Restoration And Acceptance — 2026-09-17

The operator's "pass" confirms clearing and saving the FLAC path, reloading,
and seeing both the blank field and the unset/PATH description while leaving
the unrelated MusicIndex setting unchanged. Unset-path restoration passes;
V1–V2 are accepted. V3, remaining Settings/core-recovery presentation and final
preservation/cleanup remain open. Continue in the same fixture for bounded
failure checks. Task 009 has not started.


### Task 008 V3 Timeout Report — 2026-09-17

The supplied report records the unset-path restoration Save/reload at
11:28:08/11:28:12 UTC, preserving `.v4vmm-config-237986-4.backup`. It then names
PATH FLAC and ffmpeg timeout results at 11:31:15 and 11:31:20 UTC respectively,
with a five-second limit and termination/reaping explanation. Timeout reporting
and the supplied copy pass. Timeout interaction confirmation followed with the
exit-7 check below. Child cleanup awaits fixture inspection.

### Task 008 V3 Exit-7 Report And Interaction Confirmation — 2026-09-17

The supplied report records PATH FLAC and ffmpeg version checks failing with
exit 7 at 13:16:21 UTC. The scoped WAV warning and version-only explanation
remain accurate, and the copy retains earlier observations without fixture
process output. The operator confirmed timeout navigation/resize responsiveness
and completion with the editor closed until manually reopened. The operator
also confirmed the fixture sentinel was absent from the app report, copied
report and terminal running the app. Those checks pass.

Permission/output-limit cases, remaining Settings/core-recovery presentation,
working-mode recovery and preservation/cleanup remain open. Task 009 has not
started.

### Task 008 V3 Permission-Denied Report — 2026-09-17

The supplied report records permission to execute denied for PATH FLAC and
ffmpeg at 13:22:50 UTC, distinct from the earlier missing-executable, timeout
and exit-7 results. The scoped WAV warning and version-only explanation remain
accurate. Permission-denied reporting passes. The output-limit case, remaining
Settings/core-recovery presentation, working-mode recovery and
preservation/cleanup remain open.

### Task 008 V3 Output Limit And Settings Presentation — 2026-09-17

The operator reported pass for both converters reaching the 16 KiB output limit,
reporting process cleanup and finishing without hanging. The operator also
accepted normal/narrow Settings presentation: readable report text, no action
overlap and reachable Test/Save controls. No separate observation timestamp
was supplied. Working-mode recovery after the failures, core-recovery
access/presentation, preservation inspection in both cases and fixture cleanup
remain open. Task 009 has not started.

### Task 008 Converter-Setup Preservation — 2026-09-17

The operator supplied the closed-app inspection for
`/tmp/v4vmm-startup-2qpie3af`. All eight converter checks passed, including
preserved original bytes and unedited values, restored unset FLAC path,
version-only invocations, reaped children, owner-only backups, no candidates
and an intact original case copy. Shared checks also passed for configuration,
music, migration versions 1–11, the one-playlist/three-track library, unchanged
bindings and absence of music/database probes. Changed configuration bytes are
consistent with the accepted guarded Saves; preservation passed separately.

Converter-setup preservation is accepted. Working-mode version invocations
were recorded at 13:28:37 UTC, but this output does not record their exit
results. Final success-report confirmation, core-recovery access/presentation,
core-recovery preservation and fixture cleanup remain open.

### Task 008 Core-Recovery Access And Missing Converters — 2026-09-17

The supplied recovery report records configuration loading at 13:33:09 UTC,
with the deliberate empty `music_dir` and invalid `musicindex_endpoint` issues.
Both PATH converters report not found at 13:33:43 UTC, with the scoped WAV
warning and version-only explanation. Core-recovery access and missing-converter
reporting pass. Fresh working results in that same recovery window,
normal/narrow recovery presentation, core-recovery preservation and fixture
cleanup remain open. Final converter-setup working-result confirmation also
remains pending.

### Task 008 Fresh Working Results In Core Recovery — 2026-09-17

The supplied recovery report retains earlier missing-converter observations
and records both PATH FLAC and ffmpeg version checks succeeding with exit 0 at
13:35:03 UTC. FLAC-first guidance, the existing ffmpeg fallback and the
version-only explanation are accurate. Fresh working results in the same
recovery window pass. Normal/narrow recovery presentation, core-recovery
preservation, fixture cleanup and final converter-setup working-result
confirmation remain open.

### Task 008 Core-Recovery Preservation — 2026-09-17

The operator supplied the closed-app `converter-recovery` inspection for
`/tmp/v4vmm-startup-2qpie3af`. All eight converter checks passed. Shared checks
confirm unchanged configuration bytes, preserved music and migration versions
1–11, unchanged one-playlist/three-track library and bindings, and no residual
music or database probes. Recorded converter children were reaped. Preservation
is accepted in both fixture cases. Final converter-setup success-report and
normal/narrow recovery presentation confirmations, and fixture cleanup remain
open. Task 009 has not started.

### Task 008 Final Acceptance And Cleanup — 2026-09-17

The operator supplied normal-mode restoration and removal output for
`/tmp/v4vmm-startup-2qpie3af`, then reported "all passed" for fixture removal,
normal/narrow recovery presentation and the final Settings Test showing exit 0
for both converters after the failure cases. This completes V3's remaining
confirmations and cleanup. V1–V3, Settings/core-recovery presentation and
preservation in both cases are accepted. Task 008 is complete; no acceptance
checks remain for this packet. Task 004 and inherited gates remain separate.
Task 009 has not started and requires a fresh session.
