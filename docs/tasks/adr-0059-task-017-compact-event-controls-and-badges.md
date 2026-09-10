# ADR 0059 Task 017: Compact Event Controls And Badges

Status: Accepted - 2026-09-09.
Implementation not started. Mechanical acceptance outstanding; operator visual
acceptance open for ADR 0059 behavior and ADR 0063 presentation.
Scheduling: ready for the next session. Show action feedback task 001 passed
operator acceptance on 2026-09-10; its prerequisite is complete.

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
- `src/ui/composites/show_detail_panel.rs`, `show_card.rs`, `show_log_pane.rs`,
  and `src/ui/shells/show.rs`: layout and callback slots.
- Shared badge/menu/popover and selectable-text owners under `src/ui/`:
  reuse them; extract card badge presentation if there is no shared owner yet.
- `src/broadcast/registry.rs` and `src/broadcast/publisher_targets.rs`: inspect
  the existing registry and target APIs. Change only if integration requires a
  narrow service-boundary adjustment; do not change their transport semantics.
- `tests/architecture_tests.rs`: named assertions below.
- `docs/runbooks/broadcast-event-recovery-fixture.py`: extend the isolated fixture
  as specified below; it never launches the app.
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
- Keep setup/locate/verify isolation, the existing task-016 marker/prefix,
  and current mode commands compatible. Preserve two-second GET latency for
  visible passive-check progress. Test invalid paths and inputs without
  launching the GUI; the previous directory-isolation regression stays valid.

The two new mode commands below are this implementation's fixture contract;
they are not claimed to exist before task 017 is implemented.

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

Gate: **Open; awaiting implementation and fixture extensions.** The commands
below are for a person in a Linux desktop session after task 017 is built.
Python 3.11+, two terminals, and free port 17863 are needed. No real publisher,
encoder, audio hardware, or system-service changes are needed. Use a fresh
fixture and keep its relay alive throughout; its existing task016 directory
name and marker are intentionally retained for locate/verify compatibility.

### Setup And Event Recovery

1. In terminal A, prepare a fresh isolated fixture and start its relay:

   ```bash
   cd /home/citizen/build/v4vmm
   cargo build
   task016_dir=$(mktemp -d /tmp/v4vmm-task016.XXXXXX)
   python3 docs/runbooks/broadcast-event-recovery-fixture.py setup "$task016_dir"
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

2. Open Show, then Live Metadata. Confirm Event/Producer/Publisher each has a
   labeled badge. Event reads No event and stays non-green even with two Active
   service badges.
   Event has no inline token path or XML. At normal size and the smallest
   supported window size, both service headings, badges, and controls remain
   reachable without scrolling through event diagnostics. A growing card or
   diagnostics pushing the services down is wrong.

3. Before creation, set a registration failure in terminal B:

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

4. Set the relay to Dead, then press Retry check:

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

5. Choose the older fixture-event-1. Expect its own check and the notice that
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

6. In terminal B, seed the exact target-scope regression. This block verifies
   isolation and reads token paths, never token contents:

   ```bash
   python3 - "$task016_dir" <<'PY_TARGETS'
   import json
   from pathlib import Path
   import runpy
   import sqlite3
   import sys

   fixture = runpy.run_path('docs/runbooks/broadcast-event-recovery-fixture.py')
   root = fixture['fixture_root'](sys.argv[1])
   fixture['verify_fixture'](root)
   with sqlite3.connect(root / 'app.sqlite', timeout=5) as conn:
       events = dict(conn.execute('SELECT event_id, token_path FROM broadcast_events'))
   assert {'fixture-event-1', 'fixture-event-2'} <= events.keys()
   def target(name, event_id):
       return {'name': name, 'event_id': event_id,
               'token_file': events[event_id], 'stream_delay_secs': 0.0}
   payload = {'targets': [target('default', 'fixture-event-1'),
                          target('unused', 'fixture-event-2')]}
   temp = root / 'targets.pending.json'
   temp.write_text(json.dumps(payload))
   temp.replace(root / 'targets.json')
   PY_TARGETS
   ```

   Press Check again to refresh liveness and configuration. Although unused
   carries the selected ID, expect Not attached; Detach must be unavailable.
   Press Attach: default changes to fixture-event-2 and unused remains. Inspect
   targets.json. Now press Detach: only default is removed; unused remains and
   Event becomes Not attached. A green Event based on unused, deleting unused,
   or wiping both targets is wrong. Attach again to restore the configured chain.

7. With Attached/Active/Active confirmed, press Check again. During its two-second
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

   Save the target fixture, induce a failed target-list parse, and recheck:

   ```bash
   cp "$task016_dir/targets.json" "$task016_dir/targets.saved.json"
   printf 'invalid fixture json\n' > "$task016_dir/targets.json"
   ```

   Expect Target read failed, Not ready, and no Attach; successful liveness
   cannot hide the failed configuration read. Restore targets and use the
   configuration-read retry offered by the item:

   ```bash
   cp "$task016_dir/targets.saved.json" "$task016_dir/targets.json"
   ```

### Services, Diagnostics, And Cleanup

8. Exercise each service's non-green state independently in terminal B:

   ```bash
   python3 docs/runbooks/broadcast-event-recovery-fixture.py producer-state "$task016_dir" inactive
   ```

   Expect Producer Inactive, Event Attached, Publisher Active, and card Not ready.
   Press Producer Start and wait for its fresh Active observation. Then:

   ```bash
   python3 docs/runbooks/broadcast-event-recovery-fixture.py publisher-state "$task016_dir" failed
   ```

   Expect Publisher Failed and card Not ready. Reset/Start from its controls as
   offered and confirm recovery. During mutations, the affected badge reports
   progress; unrelated items keep their states. Readable labels must carry the
   meaning even without identifying colors.

9. Open Event Logs, then Producer Logs, then Publisher Logs. Titles and contents
   must match; service text must identify the fixture journal. Resize the bottom
   pane, select text, use Ctrl+C and right-click Copy, and paste into an editor.
   Close/reopen the pane and switch cards. Event diagnostics must not expand its
   side-panel item; transport remains reachable. Changing Event selection while
   Event Logs is open changes title/snapshot together and clears old selection.
   Changing it while service Logs is open must not steal that service pane.
   Repeated reads, late failures, or a close during a read must not overwrite a
   different source. No token content may appear in any text or clipboard result.

10. Inspect preservation and command separation:

    ```bash
    env XDG_CONFIG_HOME="$task016_dir/config" PATH="$task016_dir/bin:$PATH" \
      ./target/debug/v4vmm broadcast events list --json
    cat "$task016_dir/calls.jsonl"
    ```

    Expect both event records and token files retained. Successful registrations
    number exactly two; repeated checks use the selected ID. Target mutations
    occur only after explicit Attach/Detach. The log includes no token content.
    This is fixture acceptance, not proof of production connectivity.

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

On acceptance, record this gate Passed in this packet and the delivery row,
remove its pending-human-checks entry, and reconcile both ADR statuses against
all their gates. Do not change task 016's historical acceptance record.
