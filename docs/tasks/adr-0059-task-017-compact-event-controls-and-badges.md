# ADR 0059 Task 017: Compact Event Controls And Badges

Status: Implemented - 2026-09-10.
Mechanical checks Green. Operator acceptance and fixture cleanup are complete
for ADR 0059 behavior and ADR 0063 presentation. The inspection record below
includes report wording and timestamps, saved selection, configured-target
readiness, per-item badges, copying, delayed log results, and preservation.
The operator confirmed all three registrations followed intentional Create/Replace
clicks, then confirmed the fixture app was closed, its relay stopped, and its
verified directory removed.

The narrow-window log-body limitation remains deferred in the linked layout
proposal. Card-title clipping, long log lines, per-log following/reading positions,
UTC consistency, and playback-bar observations remain documented follow-up work.
No acceptance check remains open for this packet.

## Goal

Make Event, Producer, and Publisher easy to inspect together: a compact event
picker and action row, one labeled state badge per item, and Event diagnostics
in the shared bottom pane. An older saved event must remain operable even when
a newer registry row exists. Only the configured publisher target can satisfy
attachment; passive checks retain confirmation until their result arrives.

## Dependencies

- [ADR 0059 task 016](adr-0059-task-016-event-row-in-live-metadata.md): complete.
- [ADR 0063 task 004](adr-0063-task-004-log-bottom-pane.md): complete.
- [Show action feedback task 001](show-action-feedback-task-001-command-state-and-result.md):
  complete on 2026-09-10, including operator visual acceptance. Consume its
  per-role command ownership, fresh-observation release,
  and bounded transition policy for service mutations and the publisher restart
  caused by Attach/Detach. It is a separate session and packet.

## Decision Owners

- [ADR 0059: stored selection](../adr/0059-broadcast-control-surface.md#stored-event-selection-is-explicit-and-persistent),
  [configured target](../adr/0059-broadcast-control-surface.md#attachment-names-the-configured-target),
  [actions](../adr/0059-broadcast-control-surface.md#event-actions-keep-independent-results),
  [state mappings](../adr/0059-broadcast-control-surface.md#item-readiness-determines-section-readiness),
  and [passive checks](../adr/0059-broadcast-control-surface.md#passive-checks-preserve-confirmation-until-they-answer).
- [ADR 0063: compact items](../adr/0063-show-dashboard-layout.md#live-metadata-has-three-compact-items),
  [shared badges](../adr/0063-show-dashboard-layout.md#item-badges-share-the-card-badge-presentation),
  and [Event diagnostics](../adr/0063-show-dashboard-layout.md#event-diagnostics-reuses-the-bottom-pane).

These sections own the contract. The archived proposal is historical and is
not required reading. This packet owns implementation steps and verification.

## Files To Inspect And Likely To Change

Source locations throughout this packet were inspected on 2026-09-09; use the
named symbols if line numbers move.

- `src/db.rs`: migration registry, fresh schema, event queries, new selection preference.
- `src/app/show.rs`: command-boundary selection validation, target identity,
  command/result ownership, refresh, and initial checks.
- `src/app.rs`: retained input and selection/diagnostic state as needed.
- `src/view_models/show.rs`: renderer-free item kinds, action state, selection,
  confirmation, progress, and bottom-pane source state. Retain `event_hint`
  (`:2078`) as the compact Event item's visible short explanation.
- `src/view_models/show/event_report.rs`: concrete report sentences, frozen
  action/result times, captured identities, and omission of idle actions.
- `src/api.rs` and `src/broadcast/registry.rs`: preserve typed HTTP response
  and local-save failure facts for the report without changing request behavior.
- `src/ui/composites/show_detail_panel.rs`, `show_card.rs`, `show_log_pane.rs`,
  and `src/ui/shells/show.rs`: layout and callback slots.
- `src/ui/primitives/status_badge.rs`, `context_menu.rs`, `button.rs`, and
  `src/ui/icons.rs`: shared badge and bounded two-line menu controls.
- Shared popover and selectable-text owners under `src/ui/`:
  reuse them; extract card badge presentation if there is no shared owner yet.
- `src/broadcast/registry.rs` and `src/broadcast/publisher_targets.rs`: inspect
  the existing registry and target APIs. Change only if integration requires a
  narrow service-boundary adjustment; do not change their transport semantics.
- `tests/architecture_tests.rs`: named assertions below.
- `docs/runbooks/broadcast-event-recovery-fixture.py`: extend the isolated fixture
  as specified below; it never launches the app.
- `docs/runbooks/test_broadcast_event_recovery_fixture.py`: non-GUI fixture regressions.
- `docs/runbooks/broadcast-event-recovery-check.md`: update the walkthrough for
  the extended fixture while preserving the completed task 016 recovery checks.
- ADRs 0059/0063, this packet, delivery order, pending human checks, and affected
  operations/fixture instructions: reconcile with the completed implementation.

## Do Not Touch

- Source, Stream, Music, queue, audio, feed publication, and relay lifecycle.
- Service-watch sampling or command-transition policy; use the dependency.
- The external publisher's configuration file directly or its CLI contract.
- Existing broadcaster token contents, paths, or permissions when selecting or
  checking an event; no automatic Forget or cleanup of old entries.
- Dashboard card count, common height, or two-line summary contract.

No event rename editor, UI Forget action, Create-another action for a live
selection, separate diagnostics pane, or persistent command audit log is added.

## Implementation Steps

The 2026-09-10 report correction records each operation's latest result and UTC
time at the accepted application callback. The view model renders those frozen
facts. Store only results for actual operations in the existing session state.
Do not parse HTTP status from error strings, infer a remote failure from a local
write failure, invent timestamps while rendering, or add persistent history.

1. Add a database-scoped selection preference through the migration registry,
   including fresh-database initialization. Store one full event ID and a
   monotonically increasing selection revision. An absent preference is
   uninitialized; initialize it from the newest row once if one exists. Preserve
   a reference whose event was later removed so the UI can report it missing.
   Do not cascade deletion into a fallback choice. Keep this separate from the
   event registry, which remains the only event store.
2. Use that preference for both `RefreshShowPage::execute` projection and
   `EventRegistryCommand::execute`. Under the database mutex, re-read the
   selection/revision and fetch the chosen row by ID. Revalidate request identity
   and current action eligibility at the command boundary.
   Replace both `selected_event_input(&conn)` lookups in `src/app/show.rs`: the
   command-boundary lookup/comparison at `:722` and the projection lookup at
   `:886`. Check accepts an existing selected Live/Dead/Unknown event; Replace
   requires stored Dead; Create requires a successfully read empty registry.
   A newer unrelated row
   must not invalidate the chosen event's command.
3. Persist a picker choice before dispatching its reads. Persist a newly
   registered choice before its initial check. Preserve registration success,
   registry row, and token if saving that choice fails; refresh the list and
   report the separate save failure. Selection/context revisions and request
   ownership prevent obsolete results from replacing mounted state.
4. Correct attachment projection and Detach lookup to match the normalized
   configured target name and event ID, scoped to selected host and instance.
   `EventTargetAttachmentDisplay::from_input` (`src/view_models/show.rs:1926`)
   currently matches any target; `event_section_input` (`src/app/show.rs:1184`)
   supplies the configured target name for Attach. Make their scope agree.
   An event found only on an unused target is Not attached. Use the configured
   name for mutations; Attach continues to call `target add --replace`. The
   picker association mark follows the same scope, independently of selection.
5. Project exactly the three role slots and the complete ADR 0059 kind/label
   tables. Missing/duplicate service observations cannot produce Ok. Derive
   the card's kind from the same facts; only its label is Ready. Event's Ok
   label is Attached and each service's Ok label is Active.
6. Separate passive progress from readiness. A pending recheck preserves current
   confirmed facts for the same context and adds Checking activity. Initial
   checks cannot establish Ok. Apply each completed liveness/target read
   immediately: one failure is not concealed by another pending or successful
   read. Failed status storage is also a failed verification.
7. Integrate fact-changing event/service operations with command ownership.
   Attach/Detach reserve the Publisher role because they restart it; feed their
   completion through the same fresh-observation policy as direct service
   commands. Preserve each role's independent outcome. Release failures through
   readback and the existing bounded policy, never an indefinite Working state.
8. Add the bounded stored-event picker, compact Event item, shared labeled
   badges, and contextual actions through their view-model and shared UI owners.
   Keep busy controls mounted; Logs and copying remain usable. Preserve full
   identity in diagnostics and disambiguate any shortened picker IDs. Do not
   call `truncate()` on stacked column text.
   Preserve `event_hint` as the permitted short explanation beneath the compact
   controls, including when the badge is Attached. Its existing condition is a
   selected event with `remote_host && token_file_missing`. This local file
   observation does not establish whether the publisher host has its token,
   so it adds no badge-table row and does not invalidate confirmed attachment.
   Keep the hint visible without opening Logs; full token paths stay in Logs.
9. Generalize the bottom-pane source model from service role to service/Event
   diagnostics. Source identity includes event/context and request as relevant.
   Project the full snapshot and latest per-operation session results. Switch
   title/content together, invalidate selection when source text changes, and
   retain the existing keyboard/right-click Copy behavior. Event selection must
   not steal an open service journal pane. Full feed tag plus Copy remain
   available in diagnostics, with Copy also in the Event overflow menu.
10. Extend the fixture and update the named tests/guards below. Keep the six
    recovery tests and their preservation assertions. Replace obsolete
    Unknown-only Check assertions in this implementation commit, retaining all
    unrelated rules in composite guards.
11. Run the mechanical checks. Replace ADR prose now enforced by guards with
    their named coverage, retaining rationale and incidents. Update the actual
    fixture/operations instructions and the check below if integration changes
    command details. Keep the visual gate open in all three trackers until the
    operator completes it.

## Fixture Work Required Before Operator Acceptance

The current fixture supplies the local relay, first-check failure, two-second
GET latency, Producer state, and basic target commands. It is not sufficient
for all task 017 checks yet. Extend the existing script in this packet:

- `target add --replace` changes only the named target and `target remove`
  removes only the named target. Preserve unrelated entries. Log the target
  name and event ID, never token contents. Add a fixture regression proving
  both operations with two targets and two entries carrying the same event ID.
- Add `create-mode DIRECTORY live|fail`, default live. Fail returns HTTP 503
  for registration before issuing a new event ID/token. Existing GET `mode`
  keeps its independent behavior. The relay counter is unchanged on failed
  creation.
- Add `publisher-state DIRECTORY active|inactive|failed`, default active.
  Publisher Start/Stop/Restart/Reset acts on that fixture state, just as Producer
  actions affect Producer state. Neither touches a real unit.
- Add a `journalctl` stub producing deterministic multiline text identifying
  the requested service, and include it in fixture verification. Service Logs
  in this check must read that stub rather than the desktop's real journal.
  `journal-mode DIRECTORY normal|slow|slow-fail` makes service reads immediate,
  successful after five seconds, or failed after five seconds. Each request
  captures its mode before waiting. Record request and result without token
  contents. Existing fixtures default to normal and need no new setup.
- Keep setup/locate/verify isolation, the existing task-016 marker/prefix,
  and current mode commands compatible. Preserve two-second GET latency for
  visible passive-check progress. Test invalid paths and inputs without
  launching the GUI; the previous directory-isolation regression stays valid.

The new mode commands below are implemented. The relay mode and creation mode
are independent; changing either only changes the next fixture answer.

## Acceptance Criteria

- View-model tests assert the complete item-kind/label tables in ADR 0059 and that
  `card.state == Ok` if and only if the Event, Producer, and Publisher item
  state kinds are each `Ok`. Missing or duplicated roles cannot satisfy that
  expression. A Live event found only on an unused target projects Attention,
  not Ok; Detach is unavailable for that unrelated association. Test the same
  event on both an unused target and the configured target and prove that only
  the configured target controls readiness and Detach. Empty and whitespace-only
  configured names project Attention / Target not set with Attach unavailable,
  even for a Live event found on another target (situational, ADR 0059).
- View-model tests retain the `event_hint` condition: a selected event with a
  remote host and a missing local token file exposes the existing short hint;
  a present local file, local host, or absent event does not expose that hint.
  With a Live event on the configured remote target and both services Active,
  the local-missing hint remains present alongside Event Ok / Attached and card
  Ok / Ready (situational, ADR 0059). A shared-presentation guard proves the
  compact Event item renders the hint in its permitted short explanation without
  requiring Logs (situational, ADR 0063).
- View-model tests distinguish passive requests from mutations: a pending
  recheck retains prior Ok item/card kinds and adds separate Checking activity;
  a failed response makes Event Unknown. Initial checks cannot establish Ok.
  When liveness and target reads overlap, a failure takes effect immediately
  and the other read's pending or successful state cannot conceal it.
  Mutations project the affected item as non-Ok until resolved. They also prove
  summary fields exclude full paths, feed XML, and raw command-error bodies,
  while diagnostics retains those fields and independent outcomes (situational,
  ADR 0059).
- Report tests cover UTC timestamps with seconds, unchanged text on repeated
  projection, captured event identity, independent creation/selection/check
  results, no idle rows, and unchanged feed-tag text. Typed facts distinguish
  no HTTP response, HTTP 503, unreadable metadata, HTTP 404, and failure to save
  a valid relay answer (situational, ADRs 0059/0063).
- Application tests cover selection across refresh/restart, missing saved
  entries, failed registry reads, Create/Replace list updates, liveness checks
  of previously known events, busy controls, and rejection of obsolete results.
  They prove selection/checking never mutate publisher configuration and
  replacement preserves old records/tokens. With A selected and a newer B in
  the registry, Check and an eligible Replace operate on A; a revision change
  before execution rejects the stale request. Attachment/Detach operate on the
  configured target, including when another target carries the same ID
  (situational, ADR 0059).
- Bottom-pane tests cover event/service source switching, current-request
  ownership, selection invalidation, exact feed-tag copying, and exclusion of
  token contents (situational, ADR 0063).
- A shared-presentation guard covers reuse of badge geometry and token mapping
  by the three items and the card; the view model supplies text and state
  rather than renderer-specific colors (situational, ADR 0063).

### Existing Assertions To Update In The Implementation Change

These assertions change with the amended decision in the same implementation
commit. Replace obsolete assertions, not their still-valid
recovery coverage.

| Location and symbol | Required revision |
|---|---|
| `src/app/show.rs:1576`, within `show_event_recovery_sequence` at `:1459` | Replace the Check-enabled-only-for-Unknown assertion at lines 1576-1578 with enabled Check for each existing, idle selected Live/Dead/Unknown event. Keep Attach/Replace eligibility, exact request counts, same-ID retries, and token/config preservation assertions. |
| `src/view_models/show.rs:3946`, `show_event_unknown_check_retry_and_working_actions_are_typed` | Extend the case set to Live and Dead checks. Retain serialization, typed availability, Unknown retry, and separate registration/check outcomes. Add the passive-check activity and mutation-invalidates-readiness cases. |
| `src/view_models/show.rs:3766`, `show_event_readiness_table_covers_liveness_and_every_attachment_result` | Replace the single-available-registry-action assumption: Dead permits Replace and Check, Live permits Check. Add configured-name-plus-ID matching and the complete Event badge mapping. |
| `src/view_models/show.rs:3886`, `show_event_readiness_table_covers_every_service_state` | Assert both item-kind/label mappings and aggregate card kinds for both roles, missing/duplicate roles, and service transition ownership. Preserve current non-Ok card precedence. |
| `src/view_models/show.rs:3286`, `event_section_hints_when_remote_token_file_is_missing` | Retain the visible hint and cover its condition's negative cases plus a confirmed Live/configured/Active chain. Local file absence alone does not prove a missing token on the publisher host. |
| `src/app/show.rs:1686`, `show_event_refresh_cannot_clear_progress_or_failure_feedback` | Extend refresh rejection to saved selection revisions and host/instance/target context while retaining feedback for the same current request. |
| `tests/architecture_tests.rs:15310`, `adr_0059_event_row_precedes_services_and_registry_actions_do_not_attach` | Update feed-tag copying assertions to its new shared presentation owner. Keep Event-before-services, no Event card, registry/publisher mutation separation, and projection-before-initial-check assertions. |
| `tests/architecture_tests.rs:15142`, `adr_0063_logs_use_an_independent_bottom_pane_and_current_request` | Generalize service-only title/source and request assertions for Event diagnostics. Keep the shared splitter, selectable text, current-request gating, panel independence, and transport placement. |

The recovery helper above drives these six tests in `src/app/show.rs`; all six
remain and run against the revised helper (situational, ADR 0059):

- `:1611` `show_event_create_initial_failure_then_retry_live`
- `:1615` `show_event_replace_initial_failure_then_retry_live`
- `:1619` `show_event_create_failed_retry_remains_retryable`
- `:1623` `show_event_replace_failed_retry_remains_retryable`
- `:1627` `show_event_create_retry_404_stores_dead_before_offering_replace`
- `:1631` `show_event_replace_retry_404_stores_dead_before_offering_replace`

Also retain `show_event_status_write_failure_retains_unknown_and_retry`
(`src/app/show.rs:1636`). Extend the service log tests
`show_logs_name_each_unit_and_survive_card_selection_and_panel_close` (`:3534`),
`show_logs_cycle_same_unit_and_switch_other_unit` (`:3575`), and
`show_logs_discard_old_and_duplicate_results_after_close_and_reprojection`
(`:3610`) in `src/view_models/show.rs` to cover Event as a source (situational,
ADR 0063). Preserve the existing text selection and copy-menu guards at
`tests/architecture_tests.rs:15231` and `:15260`.

Configured-target attachment and passive-check readiness guards cite
[ADR 0059 Invariants](../adr/0059-broadcast-control-surface.md#invariants).
Every guard keeps one owning ADR and its class. Retire an obsolete assertion
when its replacement becomes binding; delete a whole test or guard only when
every rule it enforces is retired. Keep the queue separation, three-card, and
two-summary-line assertions. Remove prose replaced by the new guards in the
same implementation change, and link the owners to that coverage.

## Test Commands

```bash
cargo fmt -- --check
cargo check --quiet
cargo test --quiet
cargo clippy --quiet -- -D warnings
cargo build --quiet
```

Run the fixture's documented non-GUI regression checks too. A unit or
architecture test cannot meet the visual criteria below. Never launch the app
from an agent session.

## Verification

Mechanical checks on 2026-09-10: Green. `cargo check`, `cargo fmt -- --check`,
`cargo test` (1,252 unit tests and 218 architecture guards), strict Clippy, and
`cargo build`. No app or headless display was launched.

Checking-layout correction on 2026-09-10: Green for `cargo check`, formatting,
all 219 architecture guards, strict Clippy, and `cargo build`. Its new guard
failed against the previous layout before the correction. Operator retest
passed later on 2026-09-10: the operator confirmed Checking is okay after
rebuilding. This visual evidence, rather than the guard, accepts the correction.

Event-report correction on 2026-09-10: Green. The full suite passed with
1,257 unit tests and 220 architecture guards; 10 doctests remain ignored.
Formatting, `cargo check`, `cargo clippy -- -D warnings`, and `cargo build`
passed. All eight registry tests passed again after preserving the HTTP error
cause chain. The operator supplied the revised report and confirmed the retest
passed on 2026-09-10: reopening Logs preserved the recorded timestamps.

The isolated fixture has eight non-GUI regression tests:

```bash
python3 docs/runbooks/test_broadcast_event_recovery_fixture.py
```

Step 6 setup correction on 2026-09-10: Green for all six fixture tests. The
target-scope command replaces the indentation-sensitive pasted Python block.
Its two new tests cover command dispatch, use of the saved choice rather than
the newest row, preserved registry/config/token files, a distinct fixture-setup
trace entry, and rejection of incomplete or non-isolated setup before target
changes. This mechanical result does not close step 6's operator check.

Delayed service-log fixture on 2026-09-10: Green for all eight fixture tests.
The two additional situational ADR 0063 tests verify that pending reads keep
their captured outcome when the mode changes, that existing executable
wrappers report the delayed failure, and that normal mode restores immediate
reads. The operator checks remain open until walked in the app.

Coverage added (all situational):

- ADR 0059: `broadcast_selection_is_persistent_and_revisioned` covers fresh and
  migrated schema, reopen, missing references, and monotonic revisions.
- ADR 0059: `compact_event_saved_choice_drives_commands_and_refresh`,
  `compact_event_registration_survives_selection_save_failure`,
  `compact_event_missing_choice_and_registry_errors_are_explicit`, and
  `compact_event_context_revision_and_configured_detach_are_scoped` cover the
  application boundary and preserved registry/token ownership.
- ADR 0059: `compact_event_configured_target_and_remote_hint`,
  `compact_event_passive_checks_and_mutations_have_distinct_readiness`, and
  `compact_event_badge_tables_and_card_equivalence` cover item states and
  role-based card readiness. Existing recovery/readiness tests were updated.
- ADR 0059: `adr_0059_event_target_commands_share_publisher_ownership` retains
  the prerequisite's bounded fresh-observation policy for target restarts.
- ADR 0063: `compact_event_logs_picker_and_diagnostics_are_identity_scoped`
  and `adr_0063_compact_items_share_badges_and_event_log_disclosure` cover
  disclosure, shared badge presentation, picker identities, and log sources.
  Existing card, splitter, selection, and Copy guards remain binding.
- ADR 0063: `adr_0063_item_activity_keeps_its_width_and_single_line` guards
  the activity layout correction found during step 3. It fails against the
  original header, which allowed Checking to shrink and wrap letter by letter.
- ADRs 0059/0063: the four `event_report_*` tests in
  `src/view_models/show/event_report.rs` cover report wording, times, subjects,
  response facts, target names, and result consequences.
  `adr_0063_event_reports_use_recorded_times_and_plain_text` guards projection.
  `check_event_retains_http_status_without_changing_saved_facts` and the
  extended disconnect, recovery, and status-write-failure tests verify the
  response facts through real requests and database operations.

Implementation uses migration 11, the existing command runner, and the shared
context menu/button owners. Named target mutations preserve unrelated entries.
There are no changes to audio, relay lifecycle, or publisher CLI semantics.
The fixture's task-016 prefix remains for compatibility. Initial checks after
a selection/context change require a fresh liveness answer; pending rechecks
within an already confirmed context keep that confirmation.

The new visual gate is still open. Task 016 and action feedback task 001 retain
their completed acceptance records.

## Expected Final Report

Report changed files, behavior, mechanical checks, deviations, and remaining
acceptance work. Finish with Operator visual check, including numbered desktop
commands, expected and wrong results, prerequisites, and cleanup. Do not mark
an ADR Implemented while this packet retains an open gate.

## Escalation Triggers

- The action-feedback prerequisite has not shipped: leave this packet waiting;
  do not fold its implementation into task 017.
- The named target cannot be distinguished from unused targets with the current
  publisher response: document the observed contract before changing it.
- A selection migration cannot preserve missing references or existing rows.
- A layout would require a fourth card, a third summary line, or a separate
  diagnostic pane. Revisit the owning decision rather than bypassing it.

## Operator Visual Check

Gate: **Passed - 2026-09-10; operator acceptance and fixture cleanup complete.** The commands
below are for a person in a Linux desktop session after task 017 is built.
Python 3.11+, two terminals, and free port 17863 are needed. No real publisher,
encoder, audio hardware, or system-service changes are needed. Use a fresh
fixture and keep its relay alive throughout; its existing task016 directory
name and marker are intentionally retained for locate/verify compatibility.

Inspection progress - 2026-09-10:

| Check | Evidence and current state |
|---|---|
| Setup and layout | Passed. The operator confirmed the isolated Source and compact items at narrow and normal widths. |
| Creation failure | Passed. No event was selected, the inline failure stayed short, and Logs retained the error without moving services. |
| Checking activity | Passed after correction. The operator confirmed Checking stays horizontal. |
| Dead event | Passed. Dead and Replace appeared, while the card stayed Not ready despite two Active services. |
| Registration and check results | The operator supplied successful registration/selection and an HTTP 503 check failure for fixture-event-3. The corrected report wording and timestamp stability passed retest. The supplied report identified the publisher target, relay response, event, and retained Unknown status; reopening Logs preserved 15:30:30 and 15:30:32 UTC. |
| Live event without attachment | Passed. After mode live and Retry check for fixture-event-3, Event showed Not attached, both services showed Active, and the card stayed Not ready. |
| Attach and card readiness | Passed. The supplied report confirmed target default uses fixture-event-3 and the restart request was accepted at 15:36:07 UTC. The operator then confirmed green Attached/Active/Active item badges and a green Ready card. |
| Feed-tag Copy | Passed. The operator pasted and confirmed identical exact tags from the Event menu and Event Logs: `<podcast:liveValue uri="fixture-event-3" protocol="socket.io"/>`. |
| Picker retention and older-event selection | Passed. The supplied report saved fixture-event-1 as selected at 15:40:24 UTC, read target default still using fixture-event-3 at 15:40:25 UTC, and confirmed event 1 Live at 15:40:27 UTC. |
| Repeat check and saved selection | Passed. The operator confirmed Check again succeeds for fixture-event-1 and that event 1 remains selected through the resulting refresh, Music/Show navigation, and an app restart with the fixture relay kept running. |
| Step 5 configuration preservation | Accepted with operator clarification. Initial selection/check readback kept target default on event 3. The later target file named event 1 after an intentional Attach. The operator confirmed another intentional Attach restored event 3. These actions explain the differing file/readback states. |
| Target-change investigation | Resolved. The next restart screenshot retained event 1 selected, read target default using event 3 at 15:57:12 UTC, and reported HTTP 503 for event 1 at 15:57:14 UTC while keeping its saved Live status. Fixture calls.jsonl lines 35, 42, and 50 record target add for events 3, 1, and 3 respectively; the operator confirmed pressing Attach for event 1 and then event 3. No unintended target mutation was established. |
| Unused-target readiness | Passed. After target-scope setup and Check again with event 3 selected, default used event 1 and unused used event 3. The operator confirmed Not attached, Not ready, and Detach unavailable. |
| Attach preserves unrelated targets | Passed. The supplied targets.json retained unused and default, both using fixture-event-3 with its token path. |
| Detach preserves unrelated targets | Passed. The supplied targets.json contains only unused, still using fixture-event-3 with its token path. The operator then confirmed Event Not attached and the card Not ready. |
| Attachment restoration and successful passive check | Passed. The operator restored Attached/Active/Active and Ready, then confirmed Attached and Ready stayed green throughout More → Check again, with separate Checking activity, and remained green on success. |
| Failed passive check | Passed. The operator confirmed Attached and Ready stayed green while the request was pending, then changed to Check failed and Not ready when the relay returned HTTP 503. Event Logs explained the response. |
| Relay-response recovery | Passed. After restoring mode live, the operator confirmed Retry check alone returned fixture-event-3 to Attached, both services Active, and the card Ready. |
| Failed configuration read | Passed. The operator's screenshot shows fixture-event-3 with Target read failed, the card Not ready despite two Active services, Retry config, no Attach, and disabled Detach. Event Logs says the app could not read target default and cannot confirm which event it uses, followed by the JSON parse error. |
| Configuration-read recovery | Passed. The operator restored targets.saved.json to targets.json and confirmed Retry config alone returned the existing attachment to Attached/Active/Active and Ready. |
| Stopped Producer | Passed. The operator confirmed producer-state inactive changes Producer to Inactive and the card to Not ready while Event remains Attached and Publisher Active. |
| Producer recovery and isolated Publisher failure | Passed. After the initial screenshot showed both Producer Inactive and Publisher Failed, the operator started Producer and confirmed Producer Active, Event Attached, Publisher Failed, and the card Not ready with Publisher Failed as its first summary line. |
| Publisher recovery | Passed. The operator confirmed Reset reaches Inactive with Start available, then Start shows progress and returns Publisher to Active and the card Ready while Event stays Attached and Producer Active. |
| Log-source switching | Passed. The operator confirmed Event Logs identifies fixture-event-3 and its details, Producer Logs identifies mixxx-now-playing.service, and Publisher Logs identifies musicindex-live-publisher@task016-fixture.service, all in the shared bottom pane with matching titles and contents. |
| Selected-text copying | Passed. The operator selected a complete Publisher-log line and confirmed Ctrl+C and right-click Copy both pasted exactly that line into an editor. |
| Pane resizing and navigation | The operator passed resizing, close/reopen, and Source/Stream/Live Metadata navigation. Cards and compact detail controls remain usable. The supplied narrow screenshot shows only a log header with no visible log text; this limitation is recorded separately in A13 and is not accepted as narrow log readability. |
| Event-log identity and text selection | Passed. With Event Logs open, the operator selected text and chose fixture-event-1. The title and saved-event details changed together, and the old text selection cleared. |
| Service-log ownership after an event change | Passed. After selecting fixture-event-3 with the relay in mode fail, the operator's screenshot shows Check failed while the bottom pane retains musicindex-live-publisher@task016-fixture.service and its three fixture journal lines. |
| Closing Event Logs during a check | Passed. The operator closed the pane while Retry check showed Checking, then confirmed the failed reply left it closed. |
| Late service failure after switching to Event Logs | Passed. With journal-mode slow-fail, the operator opened Publisher Logs, switched to Event Logs while Reading logs appeared, and confirmed the Event title and details remained after the delayed failure. |
| Closing during a service read | Passed. With journal-mode slow-fail, the operator closed Publisher Logs with × while Reading logs appeared and confirmed the pane remained closed after the delayed failure. |
| Repeated service reads | Passed. With journal-mode slow, the operator opened, closed, and reopened Publisher Logs, then selected Producer Logs before the second read completed. The Producer title and journal remained after the delayed replies. Step 9 is complete with the recorded narrow-window limitation deferred. |
| Registry and token preservation | Passed. The operator's report lists fixture-event-3, fixture-event-2, and fixture-event-1, each with token present. The trace contains one register entry for each ID (lines 2, 17, and 24). No stored event or token file is missing. |
| Target command review | Passed. Trace lines 35/42/50 show the previously confirmed intentional default changes 3 → 1 → 3. Line 63 records the target-scope setup separately. Lines 66/70/73 match the guided default Attach, Detach, and reattach checks. |
| Registration action review | Passed. The operator confirmed all three registrations followed intentional Create/Replace clicks. This resolves the extra registration relative to the fresh two-event walkthrough. |
| Fixture cleanup | Complete. The operator confirmed the fixture app was closed, its relay stopped, and its verified marked directory removed. Step 10 and this packet are complete. |

The operator deferred card-title clipping to
[polish item A10](../plans/hig-product-polish-backlog.md#a10---narrow-show-card-titles-clip-abruptly)
and long-line log readability to
[polish item A11](../plans/hig-product-polish-backlog.md#a11---long-log-lines-are-hard-to-inspect).

The operator also found the selected event and publisher destination difficult
to distinguish in the compact panel. The supplied Attached badge agrees with
the target file, but the closed panel does not name the destination under
Publisher. A proposed short destination line needs a presentation decision;
no new layout has been implemented or visually accepted.

The operator's additional log requirements are recorded in
[A12: per-log following and reading positions](../plans/hig-product-polish-backlog.md#a12---follow-latest-logs-and-remember-each-reading-position)
and the [UTC timestamp follow-up](../plans/broadcast-chain-delivery-order.md#consistent-utc-log-timestamps).
These notes preserve the requested future behavior without counting it as
implemented or changing the current packet's acceptance scope.

The [narrow-layout proposal](../plans/show-narrow-layout-proposal.md) records
the later screenshot's missing log viewport, compact-card and side-panel-width
options, and the request to remove dead playback controls. Their future
implementation requires its own decision and visual checks.

### Report Wording Retest

Passed - 2026-09-10. The operator supplied the revised report and confirmed
that closing and reopening Logs left the recorded timestamps unchanged.

Purpose: identify the event and relay, explain the response and saved state,
and show when each action's result was recorded. Keep the existing fixture.

1. Close only the test app. Leave terminal A's relay running.
2. In terminal B, make the next event check fail, rebuild, and relaunch:

   ```bash
   cd /home/citizen/build/v4vmm
   if python3 docs/runbooks/broadcast-event-recovery-fixture.py verify "$task016_dir" &&
      python3 docs/runbooks/broadcast-event-recovery-fixture.py mode "$task016_dir" fail &&
      cargo build; then
     env XDG_CONFIG_HOME="$task016_dir/config" PATH="$task016_dir/bin:$PATH" \
       ./target/debug/v4vmm &
     task016_app_pid=$!
   fi
   ```

3. Open Show → Live Metadata. Press Retry check after the initial check ends.
   Open Event Logs. Each action result has a date, time with seconds, and UTC.
   The report names the selected event and relay, says the relay answered HTTP
   503, and says which saved event state the app kept. Technical details follow
   that explanation. A claim that the relay never answered is wrong.
4. Close and reopen Logs without another check. Existing result times stay
   unchanged. Actions that did not run produce no rows. A restart does not
   invent a registration action. Keep the fixture running for the remaining
   checks; step 10 supplies cleanup.

### Setup And Event Recovery

Each fixture mode command below runs in terminal B. It changes the next answer;
it does not refresh the app. The app action beside it requests that answer.

The event IDs below describe a fresh run. If your replacement has a different
ID, use its actual picker ID in the later checks. The target-scope script reads
the saved selection directly. The accepted guided session reached
fixture-event-3. The operator confirmed all three registrations followed
intentional Create/Replace clicks; step 10 records their preservation.


1. **Use a safe test environment.** In your desktop terminal A, prepare a fresh
   fixture and start its relay. Create it in that terminal; do not reuse a
   temporary path supplied by an agent session. Agent-side verification does
   not establish that your desktop terminal can access that directory.

   ```bash
   cd /home/citizen/build/v4vmm &&
   cargo build &&
   task016_dir=$(mktemp -d /tmp/v4vmm-task016.XXXXXX) &&
   python3 docs/runbooks/broadcast-event-recovery-fixture.py setup "$task016_dir" &&
   python3 docs/runbooks/broadcast-event-recovery-fixture.py verify "$task016_dir" &&
   python3 docs/runbooks/broadcast-event-recovery-fixture.py serve "$task016_dir"
   ```

   Keep terminal A running. In terminal B, locate and verify it before launch:

   ```bash
   cd /home/citizen/build/v4vmm
   if task016_dir=$(python3 docs/runbooks/broadcast-event-recovery-fixture.py locate) &&
      python3 docs/runbooks/broadcast-event-recovery-fixture.py verify "$task016_dir"; then
     env XDG_CONFIG_HOME="$task016_dir/config" PATH="$task016_dir/bin:$PATH" \
       ./target/debug/v4vmm &
     task016_app_pid=$!
   fi
   ```

   If locate reports more than one fixture, assign the exact path printed by
   the running relay and verify it before launching. Never substitute a literal
   placeholder or normal app configuration. Confirm Source says Task 016 fixture,
   the library is empty, and the endpoint is http://127.0.0.1:17863. If not,
   close that window and correct the verified launch before pressing actions.

2. **Check the cleaner layout.** Open Show, then Live Metadata. Confirm Event/Producer/Publisher each has a
   labeled badge. Event reads No event and stays non-green even with two Active
   service badges.
   Event has no inline token path or XML. At normal size and the smallest
   supported window size, both service headings, badges, and controls remain
   reachable without scrolling through event diagnostics. A growing card or
   diagnostics pushing the services down is wrong.

3. **Check that failures stay compact.** Set a registration failure in terminal B:

   ```bash
   python3 docs/runbooks/broadcast-event-recovery-fixture.py create-mode "$task016_dir" fail
   ```

   Press Create. Expect No event with brief creation failure and usable Logs.
   Open Logs: expect the failure details and no invented event identity. Restore
   creation, then press Create once:

   ```bash
   python3 docs/runbooks/broadcast-event-recovery-fixture.py create-mode "$task016_dir" live
   ```

   Expect fixture-event-1 selected immediately, followed by the deliberate
   initial check failure. Registration success and check failure stay separate.
   Retry check is available; the event and card remain non-green. Inspect the
   full ID/token path in Logs. Duplicate commands during checking are disabled.
   Inspect the pending activity at narrow and normal widths: Checking stays
   on one horizontal line beside the badge, and the header height stays stable.
   If retesting the layout correction with an existing event, keep the relay
   and fixture directory, rebuild and relaunch only the app using step 1's
   verified terminal B launch, then press Retry check in fail mode. Do not
   Create or Replace solely to repeat this layout check. Inspect registration
   success and check failure in Logs before closing the old app; those session
   results are cleared on restart. Its stored event ID and token path persist.
   The revised report uses timestamped sentences: the app created the named
   event and saved your choice; the relay then answered HTTP 503 for that event.
   Each sentence states its own outcome. Idle operations have no rows.

4. **Check recovery in the new controls.** Set the relay to Dead, then press Retry check:

   ```bash
   python3 docs/runbooks/broadcast-event-recovery-fixture.py mode "$task016_dir" dead
   ```

   Expect a Dead Event badge and Replace. Replace once: fixture-event-2 appears
   in the picker while fixture-event-1 remains stored. The initial check fails;
   set mode live and press Retry check:

   ```bash
   python3 docs/runbooks/broadcast-event-recovery-fixture.py mode "$task016_dir" live
   ```

   Expect Not attached, with two Active service badges and a Not ready card.
   Press Attach. After configuration readback and fresh service observation,
   expect Attached/Active/Active and the card's green Ready badge. The card alone
   says Ready. Copy feed tag from the menu and from diagnostics into a text
   editor; each must yield exactly
   `<podcast:liveValue uri="fixture-event-2" protocol="socket.io"/>`.

### Saved Selection, Target Scope, And Passive Checks

5. **Check that your choice sticks.** Choose the older fixture-event-1. Expect its own check and the notice that
   the configured publisher target still names fixture-event-2. Check again
   must work despite the newer registry row. Refresh, navigate away/back, close
   only the app, and relaunch with the terminal B launch command from step 1.
   The same older entry stays selected. Inspect fixture targets to confirm
   selection/checking did not change them:

   ```bash
   cat "$task016_dir/targets.json"
   ```

   Returning to the newest row or rejecting a command with "Selected event
   changed; refresh Show" solely because it is older is wrong. Select
   fixture-event-2 again before the next step.

6. **Check that unused targets cannot turn Event green.** Set mode live in
   terminal B, select the replacement event in the app, and wait for its check
   to finish. In the current guided session the replacement is fixture-event-3.

   ```bash
   python3 docs/runbooks/broadcast-event-recovery-fixture.py mode "$task016_dir" live
   ```

   Then prepare the two fixture targets in terminal B. This command verifies
   isolation and reads the saved choice and token paths without changing the
   registry or token files. It assigns default to fixture-event-1 and unused
   to the selected replacement event:

   ```bash
   python3 docs/runbooks/broadcast-event-recovery-fixture.py target-scope "$task016_dir"
   ```

   It prints both target assignments and records a seed target scope entry in
   calls.jsonl, distinct from an app Attach command. If an earlier pasted block
   failed on indentation, run this command with the existing fixture.

   Choose **More → Check again** to refresh liveness and configuration. Although unused
   carries the selected ID, expect Not attached; Detach must be unavailable.
   Press Attach: default changes to the selected event and unused remains. Inspect
   targets.json. Now choose **More → Detach**: only default is removed; unused remains and
   Event becomes Not attached. A green Event based on unused, deleting unused,
   or wiping both targets is wrong. Attach again to restore the configured chain.

7. **Check that passive checks do not flicker.** With Attached/Active/Active confirmed, choose **More → Check again**. During its two-second
   request, Attached and the card's Ready stay green with separate Checking
   activity. Success keeps them green. Change mode to fail, then Check again:

   ```bash
   python3 docs/runbooks/broadcast-event-recovery-fixture.py mode "$task016_dir" fail
   ```

   During the request the confirmed badges stay; when failure answers, Event
   becomes Check failed and the card Not ready. Logs shows the error. Restore
   mode live:

   ```bash
   python3 docs/runbooks/broadcast-event-recovery-fixture.py mode "$task016_dir" live
   ```

   Press Retry check to recover without registration or target mutation.
   A card flicker solely because a passive check started, or a green card after
   its failed response, is wrong.

   Test a failed configuration read. Run this block in terminal B, then choose
   **More → Check again** in the app:

   ```bash
   cp "$task016_dir/targets.json" "$task016_dir/targets.saved.json"
   printf 'invalid fixture json\n' > "$task016_dir/targets.json"
   ```

   Expect Target read failed, Not ready, and no Attach; successful liveness
   cannot hide the failed configuration read. Restore targets and use the
   configuration-read retry offered by the item. Run the command, then press
   **Retry config**:

   ```bash
   cp "$task016_dir/targets.saved.json" "$task016_dir/targets.json"
   ```

### Services, Diagnostics, And Cleanup

8. **Check each badge separately.** In terminal B, stop the simulated Producer:

   ```bash
   python3 docs/runbooks/broadcast-event-recovery-fixture.py producer-state "$task016_dir" inactive
   ```

   Expect Producer Inactive, Event Attached, Publisher Active, and card Not ready.
   Press Producer Start and wait for its fresh Active observation. Then:

   ```bash
   python3 docs/runbooks/broadcast-event-recovery-fixture.py publisher-state "$task016_dir" failed
   ```

   Expect Publisher Failed and card Not ready. Press **Reset**, then **Start**,
   and confirm recovery. During mutations, the affected badge reports
   progress; unrelated items keep their states. Readable labels must carry the
   meaning even without identifying colors.

9. **Check on-demand diagnostics.** Open Event Logs, then Producer Logs, then
   Publisher Logs. Titles and contents
   must match; service text must identify the fixture journal. Resize the bottom
   pane, select text, use Ctrl+C and right-click Copy, and paste into an editor.
   Close/reopen the pane and switch cards. Event diagnostics must not expand its
   side-panel item; transport remains reachable. Changing Event selection while
   Event Logs is open changes title/snapshot together and clears old selection.
   Changing it while service Logs is open must not steal that service pane.
   Repeated reads, late failures, or a close during a read must not overwrite a
   different source. No token content may appear in any text or clipboard result.
   Use [Delayed Service Log Checks](#delayed-service-log-checks) below to keep
   service reads pending long enough to inspect these cases.

10. **Check preservation, then clean up.** Inspect the stored events and commands:

    ```bash
    env XDG_CONFIG_HOME="$task016_dir/config" PATH="$task016_dir/bin:$PATH" \
      ./target/debug/v4vmm broadcast events list --json
    cat "$task016_dir/calls.jsonl"
    ```

    Expect all created event records and token files retained. A fresh run has
    exactly two successful registrations. Each must correspond to one explicit
    Create/Replace action; investigate any extra registration before passing.
    Repeated checks use the selected ID. Target mutations
    occur only after explicit Attach/Detach. The log includes no token content.
    This is fixture acceptance, not proof of production connectivity.

    For a concise token-presence report, run the following in terminal B.
    It reads each stored token path and checks whether its file exists; it
    does not read or print token contents. Compare the event IDs with the
    successful registrations in the second command's output. Each successful
    registration must correspond to an intentional Create or Replace press.

```bash
env XDG_CONFIG_HOME="$task016_dir/config" PATH="$task016_dir/bin:$PATH" ./target/debug/v4vmm broadcast events list --json |
python3 -c 'import json, pathlib, sys; events = json.load(sys.stdin); [print(e["event_id"] + ": token " + ("present" if pathlib.Path(e["token_path"]).is_file() else "MISSING")) for e in events]'
```

```bash
rg '"operation": "(register|target add|target remove|seed target scope)"' "$task016_dir/calls.jsonl"
```

Close the fixture app, stop the relay in terminal A with Ctrl+C, then in
terminal B remove only the verified marked fixture directory:

```bash
printf '%s\n' "$task016_dir"
test -f "$task016_dir/task-016-fixture" && rm -r -- "$task016_dir"
unset task016_dir task016_app_pid
```

Preserve any earlier directory named with the literal
`REPLACE_WITH_PRINTED_SUFFIX`; a real registry may still reference a token
there. No real service/configuration restore is needed for this isolated run.

Closure recorded - 2026-09-10. This packet and the delivery row are Implemented,
the pending-human-checks entry is removed, and ADRs 0059/0063 are reconciled to
Implemented. Task 016's historical acceptance record is unchanged.

### Delayed Service Log Checks

These complete the remaining service-read cases in step 9. Keep the existing
fixture app and relay running. No rebuild or restart is needed: the existing
fake journal command loads the updated script for each request. These modes
affect only fixture service logs; event replies and service states keep their
own settings. If Reading logs was not visible before switching or closing,
repeat that case; an already completed request cannot prove late-result handling.

1. **A late service failure must not replace Event Logs.** Set the new journal
   mode in terminal B:

```bash
python3 docs/runbooks/broadcast-event-recovery-fixture.py journal-mode "$task016_dir" slow-fail
```

Open Publisher Logs. While it says Reading logs, open Event Logs. Wait six
seconds. The pane must retain the Event title and event details. A Publisher
error replacing them is wrong.

2. **A late service failure must not reopen a closed pane.** Keep slow-fail.
   Open Publisher Logs, then close the pane with × while Reading logs appears.
   Wait six seconds. The pane must stay closed.

3. **Repeated reads must leave the most recently chosen log visible.** Set
   delayed successful reads in terminal B:

```bash
python3 docs/runbooks/broadcast-event-recovery-fixture.py journal-mode "$task016_dir" slow
```

Open Publisher Logs. While Reading logs appears, press Publisher Logs again
to close it, then once more to start another read. Open Producer Logs before
that read finishes. Wait for the Producer journal, then another six seconds.
Its title and text must name mixxx-now-playing.service throughout the completed
display; neither Publisher reply may replace them.

Restore immediate service reads before preservation and cleanup in step 10:

```bash
python3 docs/runbooks/broadcast-event-recovery-fixture.py journal-mode "$task016_dir" normal
```
