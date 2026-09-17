# ADR 0066 Task 007: Optional Tool Correction And Retry

Status: Complete — 2026-09-16. Mechanical checks Green. V1–V3 behavior,
presentation and preservation are operator-accepted. The narrow Library and
ADR 0073 Show card-overflow follow-ups are accepted, including preservation
and cleanup. V2/V3 fixture cleanup is confirmed. A read-only inventory found no
remaining startup fixture directories in `/tmp` or `/var/tmp`, including the
known first V1 path. The earlier port-only log does not establish a separate
fixture or acceptance gate; see the evidence reconciliation below. ADR 0073 is
Implemented. Task 004 retains its separate gate. Task 008 has not started.

## Goal And Scope

Optional-tool failures open the shared Settings correction tool and retain the
original action for explicit, freshly checked Retry. This packet follows the
accepted task 006 editor and the completed shared-log/text-selection packets.
[ADR 0066](../adr/0066-configuration-and-startup-failure-recovery.md), its
[phase plan](../plans/adr-0066-startup-recovery-phase-plan.md), and the
[review checklist](../reviews/adr-0066-startup-recovery-review-checklist.md)
own the decision, sequence and review.

## Implementation And Proof

`adr_0066_repair_routes_preserve_action_subject` in
`tests/architecture_tests.rs` is the situational ADR 0066 invariant 9 guard.
It replaces this packet's implementation recipe and coding prompt. It preserves
shared dispatch, command-boundary identity checks, inert Save and renderer-free
intent/availability ownership. Existing ADR 0059 command feedback, ADR 0066
optional-dependency and ADR 0069 Settings guards retain their requirements.

| Criterion | Actual proof |
|---|---|
| C1 — immutable inputs, enabled remedies, multiple subjects | `adr_0066_recovery_keeps_multiple_original_queries_and_save_never_retries`; `adr_0066_repair_routes_keep_subjects_actions_and_failed_checks_separate` |
| C2 — explicit consent, fresh check, command-boundary revalidation | `adr_0066_retry_rechecks_configuration_and_session_at_execution`; `adr_0066_runtime_verification_does_not_verify_an_optional_tool`; `adr_0066_retry_rejects_changed_publisher_and_removed_events`; `adr_0066_retry_rejects_removed_track_and_changed_playlist_position`; `adr_0066_target_retry_rejects_a_new_target_before_transport` |
| C3 — scoped preparation and preserved resources/issues | `adr_0066_endpoint_check_preserves_player_producer_and_other_issues`; `adr_0066_player_recovery_keeps_owner_and_does_not_materialize_a_session`; `adr_0066_producer_configuration_check_does_not_require_a_player`; `adr_0066_failed_check_does_not_resolve_an_issue_or_modify_a_core_path`; `adr_0066_failed_tool_check_disables_only_its_old_resource` |
| C4 — shared button, keyboard and toolbar routes | `adr_0066_repair_routes_preserve_action_subject` traces toolbar click and Enter to the same search adapter, keyboard/button playback to the existing owner and Show retry to the existing dispatchers; behavioral owner tests above check their shared admission; V1–V3 check desktop wiring |
| C5 — application/VM contracts and inert rendering | `adr_0066_repair_routes_preserve_action_subject`; `adr_0066_focused_repair_keeps_other_drafts_and_names_only_changed_tools`; `adr_0066_retained_inputs_and_revisions_do_not_leak_through_debug` |

### Live Owners

- `src/application/capability_recovery.rs`: typed original search, playlist/
  transport, publisher, event and encoder inputs; app-session/config generations;
  explicit Retry admission and execution-boundary revalidation. Playback and
  event commands repeat identity checks under their existing database locks.
  Attach/Detach also match the command payload to the retained target and host
  before any publisher transport call.
- `src/application/capability_recovery/setup.rs`: independent maintenance-worker
  checks and scoped resource preparation. The existing configuration write lease
  keeps in-app saves out of admitted setup/install and Retry. External edits
  still require a fresh snapshot at execution. Core path changes require the
  existing managed-session recovery route.
- `src/application/capability.rs`: dependency-to-field mapping, issue preservation
  and typed availability. Failed checks cannot continue using an old tool as
  though the new setup had succeeded. Other dependencies stay usable.
- `src/view_models/startup/capabilities.rs` and `correction.rs`: retained subjects,
  recorded results, focused fields, check/Retry availability and changed-tool
  inventory. `src/presentation/configuration_editor.rs` reuses the accepted
  guarded editor, retaining unsaved sibling drafts.
- `src/app/capabilities.rs`, existing search/playback/Show adapters and
  `src/app/startup.rs`: direct Settings routing, shared command dispatch and
  scoped installation. Runtime recovery keeps the existing runtime; changed
  publisher/encoder context refreshes the existing shared service watch with
  stale-result rejection. Save queues checks and never queues the original action.
- Shared Settings, capability-report and playlist chrome own geometry and
  accessibility; Settings VM owns focused-repair ordering. Named existing
  tokens/control styles and the accepted log frame remain the presentation path.
  No second editor or retry dispatch pipeline was added.

### Verification — 2026-09-15

Green: `cargo fmt -- --check`, `cargo check --quiet`,
`cargo test --quiet`, `cargo clippy --quiet -- -D warnings`,
`cargo build --quiet`, and
`python3 -B docs/runbooks/test_startup_recovery_fixture.py`.
The fixture regression suite has nine tests. The full Rust suite and guard
counts are recorded in the review checklist. Existing HTTP fixture tests need
local socket access; the sandboxed attempt was rerun with that permission.

Backend-only fixture smoke: Green. The loopback server recorded original-query
requests, passive service reads produced no mutations, rejected and released
stub commands recorded their original unit, preservation inspection passed,
changing case restored the original stub, and both agent-created fixture
directories were removed. The GUI was not run. Backend smoke does not accept
V1–V3 or the operator's fixture cleanup.

### Boundaries

No architectural deviation or configuration/schema format change. The packet
extends the live split owners created by predecessors, including playback
commands and the shared playlist button for subject identity and accessibility.

An unknown original publisher/encoder context cannot be inferred after repair:
Retry explains the rejection and requires a new action. Changing a loaded
player or its publication target requires ending the app session; setup does
not interrupt it. A check deferred by an active Show command reports that fact
and leaves its **Check** control available after completion. The converter field is
validated here; executable verification and retained-download retry remain
008/009. Audible playback, cue/audition separation and the observed mpv IPC
issue remain under task 004/ADR 0068.

## V1 Screenshot Follow-Up

The operator's screenshot shows separate retained entries for `first retained
query` and `second retained query`, recorded at 2026-09-16 00:43:00 UTC and
00:43:30 UTC. The second query remains selected and both Repair actions are
visible. This supports the multiple-subject checkpoint only.

The search explanation incorrectly described every unavailable dependency as
a missing background runtime. `src/view_models/search_results/failure.rs` now
uses the typed dependency to distinguish endpoint setup from runtime failure.
`adr_0066_search_endpoint_failure_does_not_blame_the_runtime` checks the visible
explanation, repair instruction and copied report for both endpoint dependency
forms, while preserving the runtime-specific explanation. The later screenshot
shows the corrected endpoint explanation. The operator accepted the focused
editor route. The copied editor report records validation at 2026-09-16
00:58:19 UTC and Save at 00:58:27 UTC. It names the original backup
`.v4vmm-config-3893835-0.backup` in fixture `/tmp/v4vmm-startup-o77im_r8`.
The next screenshots show the original first query completed at 01:00:33 UTC,
with the second query still pending and the player/converter issues retained.
The request log records only the first query's feed and track requests at
01:00:33.378979 UTC and 01:00:33.379740 UTC, after the recorded Save. It contains
no request for the second query and no service mutation. This supports inert
Save and explicit original-query Retry. Preservation is accepted below. Repeated
checks, width/theme/copy checks and cleanup were not accepted at this checkpoint;
subsequent acceptance is recorded below. V2–V3 have not run.

### V1 Preservation Inspector Correction

The operator's inspection of `/tmp/v4vmm-startup-o77im_r8` reports
`original_preserved: true`, `owner_only_backups: true`, and
`unedited_values_preserved: false`. The expanded report isolates this to the
endpoint spelling: it differs only by surrounding whitespace or trailing
slashes. Every other configuration check passes, no unexpected setting path
is reported, and no candidate remains.

`retry-inspect` now reports each configuration check, unexpected field paths and
residual candidates without printing configuration values. Its endpoint check
accepts surrounding whitespace and trailing slashes for the fixture's exact URL.
This matches `normalize_musicindex_endpoint` in `src/config.rs`; focused
correction retains the entered text in the saved document. A different scheme,
host, port, path, credential, query or fragment still fails. Nested comparisons
preserve TOML types, so Python's `false == 0` behavior cannot conceal an unrelated
change. Fourteen Python regression tests are Green, including format-equivalent
URLs, rejected target changes, unchanged evidence after inspection, private
backup requirements, private-value exclusion, exact field paths and residual
candidates. No app change or fixture edit was needed.

The operator's full rerun is Green. Original bytes, owner-only backup permissions,
all unedited settings, music, library, bindings and migration records are
preserved. The fixture still has three tracks, three playlist entries and one
playlist, with bindings `a.wav`, `b.wav`, `c.wav` and migrations 1–11. No music
probe, database probe or candidate remains. `config_bytes_unchanged: false` is
expected after the explicit endpoint correction; `config_preserved: true`
confirms the original backup and permitted edits. Preservation for this fixture
is accepted. Its cleanup is unconfirmed. The revised presentation and V2–V3
remain open.

### V1 Control Clarity Follow-Up

The operator reported that Repair action, Check again and Retry did not explain
their effects. The shared VM now names the setting and operation: Edit endpoint,
Check endpoint and Run search again. Visible help distinguishes opening Settings,
checking saved setup and running the original command. Music's ready action uses
View search actions to open Settings without executing it. Other retained actions
use their own tool and command names. The shared composite renders the VM's help
with existing caption/color/spacing tokens. It owns no state decisions.

`RecoveryResult` distinguishes successful completion from failure. A completed
row offers only Dismiss on both surfaces. Later setup checks cannot restore its
run controls or replace its completion time. A new user action can still fail
and start a new recovery attempt. The successful search/playback/Show callbacks
record completion through the application owner.

Behavioral guards are `adr_0066_search_controls_explain_edit_check_and_execution`,
`adr_0066_service_controls_distinguish_observation_from_mutation` and
`adr_0066_completed_action_cannot_be_rearmed_by_a_setup_check` and
`adr_0066_failed_event_check_keeps_its_recovery_controls`. The situational
architecture guard `adr_0066_recovery_controls_explain_effect_and_completion`
preserves the VM/composite/adapter boundary. The operator accepted Music's revised
failed-search notices: Edit endpoint and its visible Settings explanation replace
Repair action. The operator also accepted Settings' Edit endpoint, Check endpoint
and Run search again explanations. The fresh fixture at
`http://127.0.0.1:44601` reports empty Index and service-command logs after Save
and explicit Check endpoint. This accepts the wording and passive pre-retry
checkpoints. Later acceptance of ready/completed Music rows, completed-state
retention and width/theme checks is recorded below. Earlier screenshot evidence
stays recorded.

The next screenshot records a failed first-query retry at 2026-09-16
02:43:15 UTC against `http://127.0.0.1:39385/v1/search`, while the current fixture
status names port 44601 and has no requests. At 02:44:10 UTC, Check endpoint
returns both unfinished actions to checked status. This is failure recovery,
not evidence that a completed action was rearmed. Returning to Music when Run
search again dispatches is the expected results route. The editor value and
loaded configuration path were not supplied, so the mismatch's cause is not
established.

The later fixture log records the first query's feed and track requests on port
44601 at 2026-09-16 10:50:24.357604 UTC and 10:50:24.358403 UTC. No second-query
request or service mutation appears. This confirms that the latest attempt
reached the current fixture with the original query. After the requested Check
endpoint step, the operator supplied the same two request records and no service
commands. The passive-check request-count checkpoint is Green. Completed-row
presentation and retention were not yet accepted at this checkpoint; the later
operator acceptance is recorded below.

### V1 Narrow-Width Follow-Up

The operator's approximately 560-pixel Settings screenshot shows the pending
query compressed into a one-character column beside four action buttons. The
converter's two buttons also leave too little space for its explanation. The
completed first query is readable and offers only Dismiss, confirming that
Settings completion checkpoint. Subsequent Music completion, theme/width and
report-copy acceptance is recorded below.

The shared `capability_report` composite previously gave text a zero flex basis
and placed every button beside it as an independent sibling. The correction
gives the text `Size::ColumnRegular` as its preferred basis and groups actions
in their own bounded, wrapping container. The outer row can move that group
below the text, and the group can wrap its buttons. Both Music and Settings use
this owner. Labels, help, completion and availability remain in
`CapabilityReportVm`; typography, color and spacing keep their existing tokens.

The blocking manual guard is the explicit two-/four-button narrow-width check
in [V1 step 5](../runbooks/startup-recovery-check.md#v1--retry-the-original-search).
It is situational under ADR 0066. Mechanical checks cannot prove the corrected
layout. The operator accepted the corrected narrow Background tools rows on
2026-09-16: converter and pending-query text wrap readably, and their two-/four-
button groups move below the text as needed. The operator subsequently reported
pass for Settings Diagnostics and Music notices at normal/narrow widths in
Light/Dark, readable text, reachable buttons and the report-copy comparison of
query names and UTC times. This is operator confirmation; no copied report
artifact accompanied that pass. These V1 presentation checks are accepted.
Follow-up mechanical checks on 2026-09-16 are Green: formatting, compilation,
strict Clippy, build, 1,409 unit tests and 250 architecture guards. The full
suite passed with local socket access; the sandboxed IPC test that stalled was
stopped.

### V1 Search Navigation Follow-Up

The operator's next screenshot shows toolbar search results while Settings stays
selected and Settings remains in the breadcrumb. The shared search helper opened
the workspace frame without selecting its Music section. This failed ADR 0060's
existing section ownership and prevented the requested Diagnostics inspection.

The helper now uses the existing Music section transition before opening a
nonempty query from another section. This restores Music's saved navigation
before the search replaces or extends its history. Search button, Enter and
saved searches share that helper. Window-aware event subscriptions provide the
existing transition with its focus context. Empty input returns before any
section change; searching within Music retains its existing behavior. The
configuration editor remains mounted with its draft.

The situational ADR 0060 guard
`adr_0060_search_uses_music_section_navigation` protects the shared route,
empty-input ordering and use of section navigation before frame navigation.
V1 step 1 now starts each toolbar submission from Settings and requires Music
selection plus Music history in the breadcrumb. The operator accepted button and
Enter navigation on 2026-09-16. The subsequent narrow Background tools row
recheck is also operator-accepted.
Mechanical checks are Green: formatting, compilation, strict Clippy, build,
1,409 unit tests and 251 architecture guards. The suite used local socket access
for its existing HTTP/IPC tests.

### V1 Behavior And Presentation Acceptance — 2026-09-16

The operator reported pass for Music's View search actions opening Settings
without running a search, explicit Run search again using the first retained
query, and the completed Music notice offering only Dismiss after Check endpoint
for the second query. The checkpoint also required requests only for the first
query and no service mutations. No new request-log artifact accompanied this
confirmation; the earlier supplied request logs remain the recorded evidence.
Together with the accepted navigation, layout, theme and report-copy checks,
this closes V1 behavior and presentation. The operator subsequently reported
pass for the current `layout_fixture` preservation inspection, accepting original
configuration preservation, private backups, unchanged music/library/bindings
and no residual probes. No new inspection JSON or absolute fixture path was
supplied. This fixture's preservation is accepted by operator confirmation;
cleanup was not confirmed at this checkpoint. The final evidence reconciliation
below corrects the inferred additional fixture gate.

### V2 Entry Observation — 2026-09-16

The operator's 463-pixel-wide screenshot shows Startup fixture playlist selected
with the expected converter, endpoint and player configuration issues. Recovery
notices wrap readably. The Library sidebar leaves too little width for playlist
details: its heading wraps one character per line, and the track actions are not
visible in the supplied viewport. This is an open Library layout defect, distinct
from the accepted V1 recovery-notice layout. The Library composition in
`src/library/app_impl.rs` uses the shared `SplitPane` owner; playlist hierarchy
and row actions belong to `src/ui/shells/playlist.rs`.

Continue the V2 behavior check at normal window width. Repair playback belongs
to the first track's row and retains its track/playlist/position. The global
Edit player settings notice opens configuration without retaining that subject.
The screenshot did not establish editor focus, changed-position rejection or
prior fixture cleanup. After maximizing, the operator accepted the first track's
Repair playback route and Edit player settings focusing `playback.driver`.
Changed-position rejection and prior fixture cleanup remain open. The narrow
Library defect requires a separate correction and operator
recheck; maximizing is only a way to continue the behavior check.

### V2 Draft Retention And Passive Correction — 2026-09-16

The operator entered `null` in the focused player draft, moved the original first
playlist track down one position, and returned to Settings. The unsaved draft
survived. Test draft and paths, Save correction and the player setup check passed
without loading a track or starting playback. The endpoint and converter issues
remained. The operator initially accepted this checkpoint; the later failed
player check below reopens player-correction acceptance. Changed-position rejection
has not yet been exercised. Keep the original track in its new position for that
check, then restore the original order before preservation inspection.

The next operator report states that Play original track is greyed out. The
changed-position rejection has therefore not run. `CapabilityReportVm::action`
requires a checked, idle retained action and no active tool check to enable Retry;
playlist identity is validated later at the command boundary. The disabled state
does not establish subject rejection or identify why setup is not ready. Inspect
the retained row's message and run its Check player action, then capture the
Background tools report if Retry remains disabled. Preserve the moved track and
current fixture while diagnosing this checkpoint.

### V2 Saved Player Validation Failure — 2026-09-16

The supplied Background tools report identifies the V2 fixture as
`/tmp/v4vmm-startup-4yqtadz3`. Built-in playback check 1 at 16:54:21 UTC failed:
App could not validate playback settings. The original action retained at
16:27:46 UTC still names playlist 1, position 0 and track 1, with no successful
setup check. Endpoint and converter issues remain. This is a saved-configuration
validation failure before player preparation or retry, not evidence of
changed-position rejection. Reopen the earlier player-correction acceptance;
draft retention and focused-editor acceptance remain valid.

The subsequent repair report confirms the editor loaded that same configuration
at 16:28:09 UTC. Save at 16:46:41 UTC and Validate at 16:46:44 UTC both rejected
the proposed `playback.driver`: expected "null" or "mpv". No correction was saved
by that attempt, explaining why the later setup check still failed. The exact
draft contents have not been supplied, so the invalid spelling and its cause
remain unknown. The fixture is not visible in the agent filesystem.

Replace the entire focused driver input with the four lowercase characters
`null`, without quotes, a `value =` assignment or a newline. Require successful
validation, then a successful Save report naming its backup, before checking the
player again. Keep the moved track and the current fixture for V2 rejection.

### V2 Player Setup And Retry Readiness — 2026-09-16

The next supplied Background tools report records check 3 at 21:13:38 UTC:
App verified Built-in playback configuration and local setup and refreshed the
tool without retrying an original action. The retained action still names
playlist 1, original position 0 and track 1, and explicitly reports Play original
track available in Settings. The driver configuration issue is cleared; endpoint
and converter issues remain. Saved player setup, isolation and retry readiness
are accepted. The prior disabled state is resolved. The replacement Save report
and backup path were not supplied; preservation inspection remains separate.
The subsequent changed-position rejection is accepted below. Preserve the
current fixture for the remaining explicit-play and preservation checks.

### V2 Changed-Position Rejection — 2026-09-16

The operator supplied the Background tools report and a matching Settings
screenshot for 21:16:07 UTC. Retry of playlist 1, original position 0 and track 1
was refused because the original subject or position changed. This is the
expected command-boundary rejection after moving the first track down one row.
The report still identifies the original action; it does not substitute the new
first track. The screenshot shows Play original track disabled after the failed
attempt, with Edit player settings, Check player and Dismiss retained. Endpoint
and converter issues remain. Changed-position rejection is accepted.

Restore the original playlist order, then test the original track's ordinary
Play action with the prepared Null player. This new action and the fixture's
cleanup remain open; preservation is accepted below. The narrow Library layout
defect is unchanged.

### V2 Preservation And Drag Observation — 2026-09-16

The supplied inspection for `/tmp/v4vmm-startup-4yqtadz3` passes original and
unedited-value preservation, owner-only backup permissions and every named
configuration check. Backup `.v4vmm-config-4147752-0.backup` preserves the original.
There are no unexpected setting paths, candidate files, music probes or database
probes. Music, library, bindings, tool blockers and migrations 1–11 are preserved:
one playlist, three tracks, three playlist tracks and bindings a.wav/b.wav/c.wav.
Changed configuration bytes are expected after the player correction. V2
preservation is accepted; cleanup and explicit Null-player Play confirmation are
still open.

The operator also reports a brief pause when dragging the playlist handle;
Move Up/Down menu actions do not pause. Both routes use the same background
reorder command. The shared handle has a reproduced text-selection gesture
conflict and a bounded correction under
[ADR 0044 task 003](adr-0044-task-003-playlist-reorder-guards-visual.md).
The freeze's timing and correction still need desktop inspection. Keep the
fixture for that recheck; this does not close the inherited playlist gate.
The follow-up's formatting, compile, strict Clippy, debug build, 1,410 unit tests
and 251 architecture guards are Green; ten existing documentation examples remain
ignored.

The subsequent desktop drag recheck fails: an immediate 4–5 second pause with a
hand cursor remains before the move. The supplied native stack shows main-thread
layout work; a populated mock probe did not reproduce the pause. The supplied
846-sample CPU report confirms heavy layout work and a longer horizontal
out-of-bounds pause is reported. Offline recovery from the supplied raw recording
identifies GPUI's synchronous test drawing loop in the desktop executable.
Running the architecture tests reproduced the exact captured binary; a normal
desktop build excludes that loop. The fixture launcher now rebuilds before
opening the app, guarded by 16 fixture tests; 251 architecture guards also pass.
The operator subsequently reports **pass** for the normal-build restart,
reordering and horizontal out-of-bounds drag check. The repeated inspection
passes all configuration, owner-only backup, music, library, binding, tool-blocker
and migration checks, with no candidates or probes. The drag responsiveness
correction and post-check preservation are accepted. Keep this V2 fixture for
the ordinary Null-player Play check; cleanup is not yet recorded. ADR 0044's
broader theme-specific checks retain their separate scope.

### V2 Ordinary Play Acceptance — 2026-09-16

The operator reports **pass** for reopening `/tmp/v4vmm-startup-4yqtadz3` at
normal width, restoring the original playlist order and choosing Play on the
original first track's row. The explicit action loads the intended track without
a setup error or substitution using the silent Null player. Endpoint and
converter notices remain. This completes V2's behavior checks. Earlier fixture
preservation passes remain recorded. The subsequently supplied final inspection
also passes, and cleanup is confirmed below. No audible-playback, task 004 or ADR 0068 acceptance
is inferred. The narrow Library layout defect and V3 remain open.

### V2 Final Preservation Acceptance — 2026-09-16

The supplied inspection after ordinary Play accepts preservation for
`/tmp/v4vmm-startup-4yqtadz3`: original bytes, unedited values, owner-only backup
permissions and all named configuration checks pass. The original backup remains
`.v4vmm-config-4147752-0.backup`. Music, library, bindings, tool blockers and
migration records 1–11 are preserved, with one playlist, three tracks and three
playlist tracks. No unexpected setting paths, candidate files or music/database
probes remain. Changed configuration bytes are expected after the saved player
correction. The operator subsequently confirms cleanup below.

### V2 Fixture Cleanup — 2026-09-16

The operator reports **pass** for the cleanup command and directory-absence
check for `/tmp/v4vmm-startup-4yqtadz3`. V2 behavior, final preservation and
fixture cleanup are accepted. This confirmation covers only that fixture;
earlier V1 fixture cleanup remains unconfirmed. V3 begins below. The narrow
Library layout defect and ADR 0044's separate inherited checks remain open.

### V3 Failed Publisher Start — 2026-09-16

The supplied Show screenshot records `Start Publisher on Local (default),
original event none` at 23:05:03 UTC. The app retains that original action and
offers Edit publisher settings, explaining that opening Settings does not run
the action. The command result reports the fixture's rejected systemctl Start
with exit 1, separately from the Publisher's Inactive observation for
`musicindex-live-publisher@default.service`. No show is active. The unrelated
endpoint, converter and player configuration issues remain visible.

This accepts V3's initial failed-command and retained-action presentation.
The service-command record has not yet been supplied. The subsequent editor
report identifies the fixture below. Command-log evidence is still needed to
verify that Save adds no service mutation. Changed-host rejection,
original-host retry, final preservation and cleanup remain open.

### V3 Configuration Save — 2026-09-16

The supplied report identifies `/tmp/v4vmm-startup-x1nt5sbv` as the current V3
fixture. The editor loaded its configuration at 23:11:08 UTC, validated the
draft at 23:12:09 UTC and saved the correction at 23:12:13 UTC. Save reports
the original backup at
`config/v4vmm/.v4vmm-config-77402-0.backup` beneath this fixture and states that
it did not retry an operation. Endpoint, converter and player issues remain;
ordinary persistence remains paused.

This accepts draft validation and the reported configuration Save after the
instructed host edit. The report does not include the selected host value,
publisher setup result or service-command record. Those later observations
must establish retry readiness and passive Save before the changed-host retry
check is accepted. Final backup permissions and preservation remain subject to
the fixture inspector; cleanup is not yet due.

### V3 Passive Setup And Retry Readiness — 2026-09-16

The supplied Background tools report records Publisher host check 2 at
23:18:47 UTC. It read the selected publisher's service state, refreshed its
setup and retried no original action. The retained subject remains Start
Publisher on Local/default, original event none, with Start publisher available
in Settings. Endpoint, converter and player configuration issues remain.

The accompanying `retry-status` record for endpoint `http://127.0.0.1:36691`
has service commands disabled, no Index requests and exactly one service command:
`--user start musicindex-live-publisher@default.service`. This is the initial
failed Start already observed; Save and Check added no command. Passive
Save/Check and retry readiness are accepted. The subsequent changed-host retry
result is recorded below.

### V3 Changed Publisher Rejection — 2026-09-16

At 23:21:50 UTC, the retained Start Publisher on Local/default, original event
none, rejected the changed or unavailable original target and directed the
operator to check the intended target before starting a new action. The
accompanying `retry-status` record for `/tmp/v4vmm-startup-x1nt5sbv` still contains
exactly one service command: the initial
`--user start musicindex-live-publisher@default.service`. Service commands remain
disabled and there are no Index requests. No additional command or alternate
target command was sent. Endpoint, converter and player issues remain.

This accepts V3's changed-host rejection. Restoring Local, checking it without a
service mutation, explicitly retrying the original Start, unrelated-control
availability, final preservation and cleanup still need acceptance.

### V3 Original Publisher Setup — 2026-09-16

Following the instruction to restore Local, Publisher host check 4 at
23:29:02 UTC reports a service-state read and tool refresh with no Start, restart
or original-action retry. The retained Start Publisher on Local/default,
original event none, is ready in Settings. Endpoint, converter and player issues
remain, with ordinary persistence paused. This accepts the reported setup and
readiness. Two subsequent identical fixture records for endpoint
`http://127.0.0.1:36691` show `service_commands_enabled: true`, no Index requests
and exactly the initial `--user start musicindex-live-publisher@default.service`.
This accepts passive Save/Check after restoration and confirms release of the
fixture's service stub. The subsequent original Start acceptance is recorded below.

### V3 Original Publisher Retry Acceptance — 2026-09-16

The operator reported pass for the explicit Start on the retained Local/default,
original event none, action. The specified check required exactly two Start
records for `musicindex-live-publisher@default.service` (initial failure and
explicit retry), no alternate-instance command and no Index request. It also
required successful command completion reported separately from the fixture's
Inactive observation, usable local Music browsing and retained unrelated
configuration issues. These checks are accepted by operator confirmation; no new
report, command-log artifact or retry timestamp accompanied the pass.

Subsequent independent setup-tool access and preservation acceptance are recorded
below.

### V3 Final Preservation And Independent Tool Access — 2026-09-16

The operator confirms that Edit converter setting still opens the editor focused
on `flac_path` and can be closed without changes. This accepts independent
setup-tool access and completes V3's behavior checks.

The final inspection for `/tmp/v4vmm-startup-x1nt5sbv` passes original-byte
preservation, unedited values, owner-only backups and every named configuration
check. Backups `.v4vmm-config-77402-0.backup` and
`.v4vmm-config-77402-2.backup` remain. The final configuration differs only by
ordinary workspace preferences; no unexpected setting paths or residual
candidates remain. Music, library, bindings, tool blockers and migration records
1–11 are preserved. Counts remain one playlist, three tracks and three playlist
tracks, with a.wav/b.wav/c.wav bindings and no music or database probes.

V3 behavior and final preservation are accepted. Subsequent fixture cleanup is
confirmed below. Earlier V1 preservation/cleanup gaps and the narrow Library
layout correction remain separate.

### V3 Fixture Cleanup — 2026-09-16

The operator confirms successful cleanup and the directory-absence check for
`/tmp/v4vmm-startup-x1nt5sbv`. V3 behavior, final preservation and cleanup are
complete. This confirmation does not cover the earlier V1 fixtures or the narrow
Library layout defect.

### Narrow Library Viewport Correction — 2026-09-16

The shared `SplitPane` owner now fits both Library branches to the measured
viewport under ADR 0046. A side-by-side split reserves the content minimum width
and clamps the displayed sidebar width. When both minimum widths cannot fit,
navigation stacks above content with independent scrolling and a resizable divider.
The Library view model retains the preferred horizontal width, so widening
restores it. The screen only supplies measured GPUI bounds and callbacks; it does
not own the stacking calculation. Named layout tokens supply the minimum widths
and the initial navigation height share. Width and height preferences are retained
separately. Selection and command semantics are unchanged.

The situational renderer test
`adr_0046_split_fits_both_panes_and_restores_preferred_width` covers the reported
463-pixel width, short viewport, larger scale, both pane bounds, width restoration
and resize-handle behavior. The architecture guard
`adr_0046_library_split_uses_measured_shared_geometry` covers both Library branches.
The [focused operator procedure](../runbooks/startup-recovery-check.md#narrow-library-follow-up--adr-0046)
blocks acceptance until normal/narrow widths, both themes and larger scale are
inspected. It keeps the existing follow-up fixture across correction builds and
records preservation and cleanup separately;
accepted V1–V3 behavior is not reopened.

Verification: Green — five shared split-pane tests, all 252 architecture guards,
`cargo check`, formatting, required strict Clippy and the normal desktop build.
An additional `cargo clippy --all-targets -- -D warnings` run found 41 existing
test-target lint errors outside this correction; it is not claimed Green.
The normal binary was rebuilt after architecture tests for the operator handoff.

The subsequent narrow screenshot confirms stacked navigation/detail and a
readable Startup fixture playlist title, replacing the one-character column.
The three unrelated recovery notices remain readable. The navigation viewport
is very short below its feed toolbar; the playlist navigation row and track
actions are outside the shown scroll positions. Independent scrolling and
readable/selectable navigation rows are not yet confirmed. Width restoration,
theme/scale checks and fresh-fixture preservation/cleanup remain open.

### Narrow Library Scrolling And Intermediate-Width Follow-Up

The operator confirms both panes scroll and supplies a narrow screenshot showing
all three track rows and Repair playback actions. The fixed stacked divider and
unbounded recovery notices still leave navigation cramped. A wider screenshot
shows track identity/duration overlapping Repair playback. These failures keep
the visual gate open.

The shared split now permits vertical resizing using coordinates relative to
its actual viewport, clamps both panes to usable heights, and retains height
independently of the preferred sidebar width in the view model. The shared
playlist shell wraps controls below identity when necessary and clips long
column text without `truncate()`. The normal-shell capability report has a
bounded scroll viewport, a view-model count summary and a typed passive
**View tools in Settings** action. Settings keeps the full report. No error,
retained action or repair control is removed, and no configuration format or
command behavior changes. Operator acceptance and fixture preservation/cleanup
remain open.

Verification: Green — 1,415 unit tests, 253 architecture guards, `cargo check`,
formatting and required strict Clippy. Geometry tests exercise both divider axes
with a nonzero viewport origin, playlist row widths from 240 to 900 pixels, and
workspace height with three setup failures. View-model tests cover independent
width/height preferences and notice counts; the architecture guard keeps the
Settings route passive. The broader all-target lint limitation recorded above
remains outside this correction. These checks do not close visual acceptance.
The normal desktop binary was rebuilt after the final tests: Green. Reuse the
existing narrow Library fixture for the operator recheck; keep it on any failure.

### Library Acceptance And Show Card Overflow Follow-Up

The operator reports pass for the Library recheck: vertical resizing, independent
scrolling, row readability at intermediate widths, retained pane preferences,
bounded notices, passive Settings navigation, themes and larger scale preview.
No preservation or cleanup result accompanied this confirmation.

The accompanying Show screenshot exposes clipped cards with logs closed, three
setup issues and a wrapped configuration-save failure. `ShowLogPane` only mounted
the scrolling card viewport in its open-log branch. [ADR 0073](../adr/0073-show-card-overflow-scrolling.md)
records the correction before implementation: reuse that shared viewport in both
branches. Existing card geometry, view-model state, log priority, sidebar and
transport ownership remain unchanged. The focused Show visual gate and the
existing follow-up fixture's preservation/cleanup remain open. The completed
shared-log packet stays closed, and task 008 has not started.

Show follow-up verification: Green — 1,416 unit tests, 253 architecture guards,
`cargo check`, formatting and required strict Clippy. The ADR 0073 renderer
regression scrolls to the last card with logs closed and open at two short
heights, verifies the bounded viewport, and leaves transport geometry unchanged.
Documentation links and whitespace checks are Green. Visual acceptance and
fixture preservation/cleanup remain open.
Normal desktop rebuild after the final regression suite: Green. Reuse the
existing follow-up fixture for the Show check.

### Show Card Overflow Acceptance

The operator reports pass for the focused Show recheck: with the fixture issues
present, all three cards remain reachable and selectable with logs closed, open
and closed again; card scrolling leaves the transport fixed. The requested
Light/Dark and larger-scale preview checks are accepted by the same confirmation.
No new screenshots, fixture preservation results or cleanup confirmation
accompanied this pass. The Show visual gate is closed. Keep the existing
follow-up fixture until preservation inspection passes, then confirm cleanup.

### Library And Show Follow-Up Preservation

The supplied `retry-inspect` output passes preservation. Configuration bytes
are unchanged and the original remains in place; no backup was needed. Every
configuration check passes, unedited values and optional-tool blockers remain
intact, and there are no unexpected setting paths or residual candidates.

Music, library records and bindings are preserved: one playlist, three tracks,
three playlist bindings (`a.wav`, `b.wav`, `c.wav`), and migration versions 1–11.
No database or music probes remain. `normal_workspace_preferences_only: false`
does not indicate a failure because `config_bytes_unchanged: true` confirms no
configuration change. The existing follow-up fixture is ready for cleanup;
directory absence has not been confirmed. Earlier V1 fixture gaps remain separate.

### Library And Show Follow-Up Cleanup Confirmed

The operator confirms that cleanup succeeded for `narrow_library_fixture` and
the directory-absence check passed. The Library/Show follow-up's mechanical,
visual, preservation and cleanup gates are closed. ADR 0073 is Implemented.
This confirmation covers the follow-up fixture. Earlier V1 evidence is
reconciled below. Task 008 has not started.

### Earlier V1 Evidence Reconciliation

The operator cannot reconstruct the earlier V1 inspection/cleanup history from
memory. A read-only filesystem inventory found no `v4vmm-startup-*` directories
in `/tmp` or `/var/tmp`; `/tmp/v4vmm-startup-o77im_r8` is absent. No fixture was
created, changed or deleted during this inventory. Preservation for the first
V1 run and the later `layout_fixture` is already operator-accepted.

`http://127.0.0.1:44601` identifies a temporary local Index server in the supplied
request log. The log does not identify its fixture directory or establish that
it was separate from the later accepted `layout_fixture`. The agent incorrectly
promoted this port-only reference into a distinct missing preservation gate.
That inferred gate is withdrawn. No historical inspection result or cleanup
command is inferred from directory absence.

The required V1–V3 behavior and preservation checks are accepted. V2/V3 and the
Library/Show follow-up have confirmed cleanup; the inventory finds no remaining
earlier startup fixtures. Task 007 is complete. No replacement test or waiver is
required for the unidentified port-only reference. Task 004's remaining checks
and tasks 008–013 stay separate.

## Operator Visual Check

Accepted: V1–V3 and the narrow Library/Show follow-ups. No additional operator
check is required to correct this acceptance record.

[Task 007's operator procedure](../runbooks/startup-recovery-check.md#task-007-optional-tool-correction-and-retry)
remains a regression check with setup, inspection and cleanup commands. It needs
a Linux desktop, Python 3.11+ and the debug binary; network/service state is
isolated and the player uses Null. This packet's closure is recorded in the
[delivery order](../plans/broadcast-chain-delivery-order.md). Unrelated open gates
remain in the [pending-human index](../pending-human-checks.md).

## Rollback

Revert this packet's code together if its gate fails. Preserve configuration,
backups, database and music. Do not reverse migrations or delete recovery
artifacts as rollback. Leave later packets pending and record the failure.
