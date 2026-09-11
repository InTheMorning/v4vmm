# ADR 0066: Startup Recovery Review Checklist

## Status And Scope

Tasks 001–003 are complete - 2026-09-11. Mechanical checks are Green;
operator acceptance, final preservation inspection and fixture cleanup are
confirmed. The keyboard checks use ADR 0067's accepted Ctrl bindings.
Settings responsiveness and cached-file recovery are accepted; temporary
diagnostics are removed. ADR 0066 remains partial. Task 004 implementation and
mechanical checks are complete; operator visual acceptance, preservation
inspection and fixture cleanup remain open. Tasks 005–013 have not started.

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
preservation checks and operator cleanup. Task 005 waits.
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
