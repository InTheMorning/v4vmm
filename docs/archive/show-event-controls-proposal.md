# Show Event Controls Proposal

> Archived 2026-09-09 after operator approval. Current decisions live in
> [ADR 0059](../adr/0059-broadcast-control-surface.md) and
> [ADR 0063](../adr/0063-show-dashboard-layout.md); executable work and the open
> acceptance gate live in [task 017](../tasks/adr-0059-task-017-compact-event-controls-and-badges.md).
> The proposal below records the reviewed draft and no longer binds current work.

Status: Archived - 2026-09-09. Proposal accepted; not an implementation record.

Advisory design draft for review. After reviewing the mechanics, the operator
chose to retain the terminology and requested an individual state badge for
Event, Producer, and Publisher. This document does not amend a binding ADR or
authorize implementation.

Retirement: on acceptance, move the decisions into the ADR 0059/0063 amendments
and executable work into a task packet, then move this file to
`docs/archive/show-event-controls-proposal.md` with an archive notice and links
to its replacements. Remove its Current plans entry and update incoming links
in the same change. If rejected, delete this draft and its index entry. It does
not remain a second current authority after the decision.

## Problem And Goal

After accepting the event recovery behavior in
[ADR 0059 task 016](../tasks/adr-0059-task-016-event-row-in-live-metadata.md),
the operator found the event item too tall. Repeated identity and state text,
full paths, the feed tag, and command results push Producer and Publisher much
further down the Live Metadata side panel.

The proposed event item answers three questions at a glance: which event is
selected, what needs attention, and what action is useful next. Configuration
and diagnostics remain available on demand in the existing bottom pane.
Each of the three items has its own labeled, colored badge. All three green
means the Live Metadata card also has a green Ready badge.

## Current State

- [The registry](../../src/broadcast/registry.rs) already stores and lists
  events, including old dead entries. The picker needs no second event store.
- [The Show application owner](../../src/app/show.rs) selects the newest
  registry entry on refresh. There is no explicit event picker or saved choice.
- [The detail composite](../../src/ui/composites/show_detail_panel.rs) renders
  identity, state, target, actions, command feedback, ID, endpoint, token path,
  feed tag, and hints above the two services.
- [The bottom pane](../../src/ui/composites/show_log_pane.rs) already provides
  resizable, selectable text with keyboard and right-click copying. Its current
  source model is specific to Producer and Publisher logs.
- Attachment projection currently matches any target carrying the event ID
  (`src/view_models/show.rs:1926`), while Attach writes the configured
  `drop_file_target` (`src/app/show.rs:1184`). This mismatch is a correctness
  defect to fix before deriving the proposed badges.

## Proposed Layout

Normal event content has three rows: heading with its state badge, picker, and
actions. A condition that needs explanation may add one short line. The badge
replaces the repeated State heading and state paragraph. Full errors never
expand this item into a diagnostic transcript. Producer and Publisher likewise
place their state badge beside their heading, retaining their unit detail and
controls below it.

```text
Live Metadata

Event                    [Not attached]
[ Sep 9, 20:30 · …a71c              ▾ ]
[ Attach ]   [ Logs ]   [ ⋯ ]

──────────────────────────────────────
Producer                       [Active]
…existing unit detail and controls…

──────────────────────────────────────
Publisher                      [Active]
…existing unit detail and controls…
```

The example describes hierarchy, not pixel sizes or final copy. Named tokens
and the shared control owners determine geometry. Long names must not turn
into tall paragraphs; the picker and diagnostics provide the full identity.
Column text continues to follow
[ADR 0063](../adr/0063-show-dashboard-layout.md#column-text-does-not-truncate).

### Three Item Badges And One Card Badge

The three items are exactly Event, Producer, and Publisher, identified by role,
not by vector length. Once Live Metadata is mounted, missing Event input projects
an Unknown/Loading placeholder; a missing service observation projects that
role's Unknown/Status unavailable placeholder. Duplicate observations for a role
are invalid input for that role and also project Status unavailable. A missing
Publisher cannot be satisfied by two Producer observations.

Each item exposes a renderer-independent state kind and label. Using the existing
`ShowCardStateKind` vocabulary, the card state is `Ok` if and only if Event,
Producer, and Publisher each project `Ok`. `Ok` maps to the shared Success
tokens, `Attention` to Warning, `Failed` to Danger, and `Unknown` to Info.
Only the card uses the label Ready. Event's successful label is Attached;
both successful service labels are Active.

Event's badge requires both confirmed Live liveness and a confirmed association
on the configured target. The target is the nonempty, normalized
`broadcast.drop_file_target` on the selected host and publisher instance.
`EventTargetAttachmentDisplay::from_input` must match both that target name and
the selected event ID. An unused target carrying this ID cannot satisfy it.
Detach and the picker association mark use this same target identity; they must
not remove or mark whichever target an event-ID-only search happens to find.

Event badge mapping, evaluated top to bottom. The command rows describe display
overlays, not invented variants of `EventState` or `EventTargetAttachmentState`.
Passive progress preserves existing facts only until a newer result for those
facts arrives; it cannot mask a failure or change reported by a concurrent read.

| Input or overlay | State kind | Badge label |
|---|---|---|
| Create / Replace in progress | Unknown | Creating / Replacing, respectively |
| Attach / Detach in progress, including readback pending | Unknown | Attaching / Detaching, respectively |
| Registry / Event input still loading | Unknown | Loading |
| Registry read failed | Unknown | Events unavailable |
| Saved selection references a missing registry entry | Attention | Event unavailable |
| Loaded registry with no event (`EventState::None`) | Attention | No event |
| Passive recheck pending after a completed result for the same read and context | Preserve current fact-derived kind | Preserve current fact-derived label; separate activity text Checking |
| Initial check pending with no completed result for that read | Unknown | Checking |
| Latest liveness check failed, including failed status storage | Unknown | Check failed |
| `EventState::Dead` | Failed | Dead |
| `EventState::Unknown` | Unknown | Unknown |
| Live, configured target name is empty | Attention | Target not set |
| Live, target read Unknown | Unknown | Target unknown |
| Live, target read CommandsUnavailable | Unknown | Commands unavailable |
| Live, target read NotReachable | Unknown | Not reachable |
| Live, target read Failed | Unknown | Target read failed |
| Live, successful read finds no configured target or that target names another event | Attention | Not attached |
| Live, successful read finds the configured target naming the selected event | Ok | Attached |

Every service state below applies independently to both Producer and Publisher:

| `PublisherServiceStateDisplay` or missing input | State kind | Badge label |
|---|---|---|
| Active | Ok | Active |
| Inactive | Attention | Inactive |
| Starting | Attention | Starting |
| Stopping | Attention | Stopping |
| Failed, with any reason | Failed | Failed |
| NotInstalled | Attention | Not installed |
| NotReachable | Attention | Not reachable |
| Unknown | Unknown | Unknown |
| Working | Unknown | Working |
| Missing or duplicate observation for the role | Unknown | Status unavailable |

Failure details remain in diagnostics with a short explanation if needed. A
failed Create leaves No event plus creation-failure feedback; a failed Replace
that created nothing leaves the former event's state plus replacement-failure
feedback. Neither outcome invents a new relay state.

For a non-Ok card, preserve the existing ordering: Event's non-Ok kind wins;
otherwise any Failed service makes the card Failed; otherwise a non-Ok service
makes it Attention. The label is Not ready. The first summary line names the
earliest non-Ok role in Event, Producer, Publisher order, even when a later
service determines the card's failure kind. The second summary line states
section readiness. Individual badges expose failures further down the chain.

### Passive Checks And Changes Have Different Effects

A passive liveness or target-list recheck cannot change the event or publisher
configuration. While it is pending, retain the last settled badge and its
confirmation for the same event, host, instance, and configured target. Show
Checking as separate item activity, including in accessibility text. A healthy
Attached/Active/Active chain remains Ok/Ready during the request. Do not replace
its Event badge with a non-Ok Checking badge just to report activity.

If no confirmation exists yet, the check cannot create an Ok state. On success,
replace the old facts with the result. On 404, project Dead once stored. On
transport, target-read, or status-storage failure, project the appropriate
Unknown badge and Not ready card; retained database liveness is only historical
evidence. Thus failure changes readiness when it answers, not when the operator
asks. Selecting another event or changing host/instance/target invalidates prior
confirmation instead of carrying an unrelated Attached badge across the change.
When liveness and target-list reads overlap, apply each completed result at
once. The other pending read cannot hold the item at its former Ok kind after
one of them has failed. Conversely, one success cannot clear the other's
failure. Diagnostics retains both results separately.

Fact-changing operations invalidate the affected confirmation immediately:
Create/Replace and Attach/Detach affect Event; service Start/Stop/Reset/Restart
affect that service. Attach/Detach also restart Publisher, so Publisher projects
Working until a fresh observation resolves that transition. An unrelated service
retains its own state. Failed mutations trigger readback and expose partial
results; an old observation cannot release the transition. This uses the
command ownership and fresh-observation contract of the action-feedback packet
linked below, rather than implementing a second transition policy.

Green confirms the Live Metadata prerequisites. It does not claim that the
audio stream is connected, the feed tag has been published, or a listener is
receiving data. All badges use shared badge geometry and named semantic tokens,
including the overall card, with their labels and accessibility text supplied
by the view model. No service gets a screen-local color rule.

### Event Picker

Clicking the selected event opens a bounded, scrollable list of locally stored
events, newest first. Each entry has its existing label, a creation date, a
distinguishing identifier, and its last known liveness state. Unnamed events
use the date and identifier as their display name. Short identifiers must be
disambiguated if they collide; selection always uses the full identity.

The list marks the selected entry. A successful publisher configuration read
may separately mark the entry configured on the selected host, instance, and
`drop_file_target`. That mark means configured, not proof that metadata is
currently flowing.
Unknown or failed reads cannot supply an affirmative configuration mark.

Selecting an entry updates the mounted view and requests a liveness check for
that entry, together with a fresh publisher configuration read. This includes
previously Live or Dead entries: the current Unknown-only manual-check
restriction needs an explicit extension. Opening the list alone makes no
network request for every entry.

The choice survives refresh, navigation, and restart through a database-scoped
selection preference, owned by `src/db.rs` and added through a migration. Store
the full event ID and a selection revision. An uninitialized preference may
choose the newest row once and persist that choice. If its saved entry has been
removed, retain that missing reference, show Event unavailable, and let the
operator choose another. No cascading deletion may silently select a different
identity. Loading or failing to read the registry is distinct from an empty
registry and cannot enable Create.

`EventRegistryCommand::execute` currently calls `selected_event_input(&conn)`
and compares the newest row to its captured selection at
`src/app/show.rs:722`. Replace that command-boundary lookup, not just the UI
projection. Under the same database mutex used for execution, read the saved
selection/revision, compare it with the request, then fetch that selected row by
full ID and derive action eligibility from its current stored state. Check may
operate on any existing selected Live, Dead, or Unknown row; Replace still
requires Dead. Create requires a successfully read empty registry, not merely
a missing or failed selection lookup. An unrelated newer row cannot reject a
valid command against the older saved selection.

All selection changes update the preference before issuing selection-dependent
reads. Create/Replace persist the new selection before scheduling their initial
check. If saving selection fails after registration succeeded, keep the created
row and token, report that separate failure, and offer that row in the refreshed
list; do not register again. Results carry the event ID, selection revision,
host, instance, target name, and request ownership as applicable. A result may
update its own registry facts but cannot overwrite a different mounted selection.

Successful Create or Replace adds and selects its new entry immediately, then
checks it as task 016 does today. Replace retains the old entry and token file.
The proposal keeps existing Create/Replace eligibility; creating additional
events while a live one is selected and adding a rename editor are outside
this change.

### Selection And Publishing Are Separate

Choosing an event changes the app's selection and performs reads. It does not
change publisher configuration, restart services, or start a show. The explicit
publisher action remains a separate press with its own progress and result.

If the configured publisher target names a different event, show that difference
briefly beside the selected event's status. Use wording such as
`Publisher configured for …b82d`, not `Publishing to …b82d`: configuration does
not establish successful delivery. The card describes readiness for the
selected event and cannot claim it is ready because another event is configured.

Attach already invokes `target add --replace`, as specified by
[task 014](../tasks/adr-0059-task-014-attach-event-to-publisher-target.md) and
enforced by `adr_0059_packet_014_attach_event_replaces_target_and_restarts_publisher`
in the publisher-target service tests (situational, ADR 0059). An existing
association at that name is replaced; the operator does not need to detach it
first. The proposed action should make that replacement clear before the press,
including which event is currently configured. The prior event and token file
remain in the local registry even after the publisher uses another event.

This configuration replacement and the following service restart are separate
operations. A successful write followed by a failed restart is a partial result
that must remain visible. Detach stays available in the overflow menu for an
event associated with the configured target, including a dead one, but it is
not a prerequisite for using another event. Arbitrary publisher-target
management is outside this proposal.

### Actions And Feedback

The action area keeps a stable place for the next useful action, Logs, and the
overflow menu. The view model supplies presence, availability, progress, and
accessibility labels. A command in progress keeps its control mounted and
unavailable; a watch refresh cannot erase the command's feedback.

The picker and competing event commands are unavailable while registration,
checking, or a publisher change is in progress. Logs and copying stay usable.
Results still carry their original identity and request ownership so that a
host change or delayed refresh cannot apply them to a different selection.

| Condition, in priority order | Primary action or visible result |
|---|---|
| An event registry or publisher-change command is running | Keep the initiating action in its progress state |
| Registry is loading or its read failed | Loading or Events unavailable; retry the failed read, no Create |
| Registry loaded and genuinely empty | Create |
| Saved selection is missing and registry contains other events | Event unavailable; choose an entry in the picker |
| Liveness is unknown, or its latest check failed | Check / Retry check |
| Selected event is confirmed Dead | Replace, with Check again in the menu |
| Publisher configuration could not be read | Retry that configuration read; no Attach |
| Event is confirmed Live, intended target belongs to another event, existing command prerequisites pass | Attach; make the replacement effect clear |
| Event is confirmed Live, target is free, existing command prerequisites pass | Attach |
| Event is Live and attached to the configured target | No setup prompt; retain the concise status |

The overflow menu holds Copy feed tag, Check again, and Detach when applicable.
Check again reads liveness for the selected ID and never registers another
event. Copy feed tag also remains available beside the full tag in diagnostics.
Logs remains available during failures and commands, including failed creation
when there is no event yet. Registry deletion controls are outside this change.

Keep registration and checking outcomes independent. For example, a successful
creation followed by a failed check gives a short `Created · Check failed`
message, retains the new selection, and offers Retry check. Full causes belong
in Logs. Publisher changes also report failure locally; if configuration was
changed but the service restart failed, show that partial result and read the
configuration back instead of claiming that nothing changed.

The existing readiness table still governs the chain: no event, a dead or
unverified event, an unconfirmed configuration, or an inactive/failed service
cannot produce a ready section. A failed recheck preserves stored liveness but
marks it as unverified in the current display; it must not present an old Live
result as a fresh confirmation. Producer and Publisher retain their own states
and recovery actions below the compact event item.

### Event Diagnostics In The Bottom Pane

Logs opens the shared bottom pane with an event-specific title and content.
It uses the existing close, resize, selection, and Copy behavior. Opening it
does not expand the event item or cover the service controls in the side panel.
Only one source occupies the bottom pane at a time.

The event content includes:

- A configuration snapshot: full event ID, label, creation time, relay endpoint,
  token file path, exact feed tag, last successful check time, and observed
  publisher association with host/instance context.
- Separate results for registration, liveness checks, publisher configuration
  reads, and publisher changes, including progress and complete safe failure
  details. Token contents never enter the display or clipboard.

This is event diagnostics assembled from registry facts and application command
results, not a service journal. The initial scope retains the latest result
per operation and event for the current app session. Reopening the app restores
the registry snapshot; it does not pretend to restore a historical command log.
A persistent audit log is outside this proposal.

While Event diagnostics is open, changing the event updates its title and
snapshot together and clears text selection. Late results stay associated with
their original event, host, instance, and request; they cannot overwrite the
new source. Choosing Producer or Publisher Logs switches to that service's
existing log view. Changing the event does not steal an open service log pane.

## Retained Terminology

The operator reviewed these mechanics and is no longer seeking a terminology
rewrite. Retain the existing action names. The individual badges supply the
additional visual confirmation that prompted this proposal's refinement.

| Current term | Actual meaning to communicate |
|---|---|
| Event | A relay identity that listener apps can follow through the feed tag |
| Select / Resume | Choose a stored identity to inspect and check in this app |
| Attach | Configure the named publisher destination with this identity and token file path, replacing its previous association if present, then restart the publisher service |
| Detach | Remove that publisher configuration entry, then restart its service; keep the event in the registry |
| Live | The last successful relay read found the event; this does not prove broadcasting is underway |
| Replace | Register a new identity after confirmed event death; listener references need the new identity |
| Logs | Open the event's configuration snapshot and diagnostic results |

The combined Event badge uses Attached when both liveness and attachment pass;
Live remains the name of the individual relay fact in the picker and
diagnostics. Short explanations state consequences where the operator makes
the choice, particularly when Attach replaces a previous association. Backend
command names can remain in diagnostics.

## Ownership And Delivery

### Existing Authority And New Decisions

[ADR 0059](../adr/0059-broadcast-control-surface.md#this-app-owns-the-event-registry)
already decides that Resume selects a stored event and tests it. The picker
delivers that existing workflow; a list of stored events and selecting an older
entry need no new architectural permission. Registration/checking remaining
separate from publisher mutation, retained dead entries, and token storage are
also existing decisions.

The amendments add this specific set:

- ADR 0059: persisted database-scoped selection and command-boundary
  revalidation against that choice; manual Check for selected Live and Dead
  entries as well as Unknown; exact configured-target attachment semantics;
  per-item state kinds and their card aggregation; passive-check continuity
  versus fact-changing operations and failed verification.
- ADR 0063: a compact Event item and per-item badge placement through shared
  presentation; Event as a source in the existing bottom pane, including
  snapshot/result ownership and the relocation of verbose event fields.

The configured-target definition corrects what implementation got wrong: the
readiness read and publisher mutation have different target scopes today.
Record it as that correction under
[ADR 0057's amendment policy](../adr/0057-adr-status-vocabulary-and-amendment-policy.md),
with the inspection date and the stale-unused-target failure case. The other
changes complete or refine existing registry, summary/detail, and bottom-pane
decisions. They do not permit automatic replacement, app-owned publishing,
token disclosure, or a fourth dashboard card. No new ADR is proposed for this
set; any later change to an existing prohibition requires separate review under
ADR 0057.

### Acceptance Reconciliation

Accepting the amendments opens new implementation and operator gates. In the
same change:

1. Change ADR 0059 from Implemented to `Accepted - YYYY-MM-DD`, using the
   amendment date. Add a separate line: `Implementation partial: tasks 001-016
   complete; compact event controls implementation and operator verification
   outstanding.` Add the dated amendment reason and link the new task packet.
   Task 016 remains accepted evidence for its shipped scope; its completed
   acceptance is not reused to satisfy the new gate.
2. Keep ADR 0063 Accepted, add its dated amendment reason and a partial
   implementation line identifying its completed dashboard/log work and the
   new badges/Event diagnostics work outstanding. Update the ADR index entries
   to agree with both records.
3. Create the bounded implementation packet with its visual gate explicitly
   open in `Status:`. Add its row to the
   [delivery order](../plans/broadcast-chain-delivery-order.md) and its executable
   operator check to [pending human checks](../pending-human-checks.md). The
   check names fixture commands, expected/wrong outcomes, prerequisites, and
   cleanup. All three artifacts refer to the same gate.
4. Transfer the draft's decision content to the owners and retire this proposal
   as stated in Status, updating the docs index and links in the same change.

After implementation, mechanical success alone leaves these gates open. Record
operator acceptance in the new packet and delivery row, remove the pending
check, and promote an ADR to Implemented only when every gate it owns is closed.
The current edit is still a proposal revision: it does not change those ADR
statuses or claim an executable check for unimplemented controls.

Likely implementation owners are the Show view model for picker/diagnostic
state, item/card badge readiness, and typed actions; the Show application owner
for selection persistence and commands; the existing registry/database for
event facts; and the shared badge, detail, menu, selectable-text, and bottom-pane
owners for presentation. Extract the existing card badge presentation to a
shared owner if necessary instead of copying its geometry or token mapping.
The implementation packet specifies the selection-preference migration and its
revision handling within the database ownership decided above. No second event
registry is needed.

The implementation packet depends on the service command-ownership and fresh
observation portion of the existing
[action feedback work](../tasks/show-action-feedback-task-001-command-state-and-result.md).
Schedule it accordingly in the delivery order; do not duplicate that work or
absorb its unrelated Stream/library changes. Source, Stream, queue, relay
lifecycle, and audio behavior are outside this proposal.

## Alternatives Considered

| Alternative | Assessment |
|---|---|
| Keep only the card badge | Leaves the operator reading each item's paragraphs to identify why the card is Not ready; rejected after the operator requested per-item confirmation. |
| Expand the Event item inline for diagnostics | Restores the vertical growth that pushed Producer and Publisher out of easy reach; rejected. |
| Add a separate event diagnostics pane | Adds another panel owner, resize interaction, and copy surface when the existing bottom pane already serves this task; rejected. |
| Reuse the bottom pane and put compact badges on all three items | Chosen. Preserves one diagnostic surface and keeps the three prerequisites visible together. |

## Consequences

The operator can see both the earliest unmet prerequisite and failures in later
items without opening diagnostics. Exact configured-target matching prevents a
stale unused target from falsely satisfying Event. Explicit saved selection
makes older registry entries usable while retaining publisher changes as
separate actions.

Shared diagnostics displays one source at a time, so event diagnostics and a
service journal cannot be compared side by side. Full paths and error bodies
take an extra press to inspect. The app must maintain selection revisions and
source/request ownership, and the migration must preserve a missing saved
reference without silently choosing another event.

A passive request retains last-confirmed readiness until it answers. That is a
deliberate continuity choice, not proof of continuous delivery. A failed result
then makes readiness unknown; no timeout or failure is treated as success.
Configuration change and service restart still have separate failure outcomes.
The revised operator fixture must exercise both these transitions.

## Verification And Rollback Plan

Proposed mechanical verification, each at its owning layer:

- View-model tests assert the complete item-kind/label tables and that
  `card.state == Ok` if and only if the Event, Producer, and Publisher item
  state kinds are each `Ok`. Missing or duplicated roles cannot satisfy that
  expression. A Live event found only on an unused target projects Attention,
  not Ok; Detach is unavailable for that unrelated association. Test the same
  event on both an unused target and the configured target and prove that only
  the configured target controls readiness and Detach (situational, ADR 0059).
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

These are source locations inspected on 2026-09-09; use the named symbols if
line numbers move. Their behavior changes with the amended decision, in the
same implementation commit. Replace obsolete assertions, not their still-valid
recovery coverage.

| Location and symbol | Required revision |
|---|---|
| `src/app/show.rs:1576`, within `show_event_recovery_sequence` at `:1459` | Replace the Check-enabled-only-for-Unknown assertion at lines 1576-1578 with enabled Check for each existing, idle selected Live/Dead/Unknown event. Keep Attach/Replace eligibility, exact request counts, same-ID retries, and token/config preservation assertions. |
| `src/view_models/show.rs:3946`, `show_event_unknown_check_retry_and_working_actions_are_typed` | Extend the case set to Live and Dead checks. Retain serialization, typed availability, Unknown retry, and separate registration/check outcomes. Add the passive-check activity and mutation-invalidates-readiness cases. |
| `src/view_models/show.rs:3766`, `show_event_readiness_table_covers_liveness_and_every_attachment_result` | Replace the single-available-registry-action assumption: Dead permits Replace and Check, Live permits Check. Add configured-name-plus-ID matching and the complete Event badge mapping. |
| `src/view_models/show.rs:3886`, `show_event_readiness_table_covers_every_service_state` | Assert both item-kind/label mappings and aggregate card kinds for both roles, missing/duplicate roles, and service transition ownership. Preserve current non-Ok card precedence. |
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

Every guard keeps one owning ADR and its class. Retire an obsolete assertion
when its replacement becomes binding; delete a whole test or guard only when
every rule it enforces is retired. Keep the queue separation, three-card, and
two-summary-line assertions. Remove prose replaced by the new guards in the
same implementation change, and link the owners to that coverage.

The future implementation runs the repository build, test, formatting, and
strict clippy checks. Visual acceptance has its own gate in the implementation
packet, delivery order, and pending-human-checks index; this unimplemented
draft creates no executable check or claim of visual acceptance.

Rollback would restore the earlier presentation and command wiring while
preserving registry entries and token files. Any selection preference added
for this feature should be safe for the older presentation to ignore.

## Operator Visual Check

Prospective checks for the implementation packet, not steps to run against
this draft:

1. Use the [isolated recovery fixture](../runbooks/broadcast-event-recovery-check.md)
   to reach no event, failed initial check, Dead, Replace, Live, and configured
   states. At the supported minimum window size and normal desktop size,
   confirm Event stays compact and both service headings, states, and controls
   are reachable without scrolling past event diagnostics. Long paths or
   errors pushing the services down are wrong.
2. Use the two entries created by that walkthrough to inspect the picker.
   Select the older entry, refresh, navigate away/back, and restart the app.
   Confirm selection persists, states identify what was checked, and choosing
   an entry does not change the publisher's configured event. Test an occupied
   target: the explicit Attach action replaces its association without a prior
   Detach, while preserving the former event and token file in the registry.
   Put the selected event only on an unused target while the configured target
   carries the other event: expect Not attached. Detach must not offer to
   remove the unused association. Put the selected ID on both targets and
   confirm Detach removes only the configured target's association.
3. Open Event, Producer, and Publisher Logs in turn. Confirm source titles and
   content agree, text remains selectable, right-click Copy works, feed-tag
   copying is exact, and delayed results do not display under another source.
   Check a failed creation with no selected event as well.
4. Confirm all three item badges have readable labels and consistent placement.
   A Live but unattached event stays non-green while Active services are green.
   Attach it: all three become green and the card reads Ready in green. Use the
   fixture's inactive and failed Producer modes to confirm only that service's
   badge changes, while the card becomes Not ready. On a healthy chain, press
   Check again: Attached and the card's Ready remain green with separate
   Checking activity until a response arrives. Initial checks stay non-green;
   a failed recheck becomes Check failed and Not ready. A flicker to Not ready
   solely because a passive request began is wrong. A green card while any
   item is non-green is also wrong. State must remain understandable from the
   text without identifying its color.
5. Close the fixture app, stop its relay, and use the linked walkthrough's
   marked-directory cleanup. The future packet must extend that fixture and
   supply exact commands for any additional target replacement, latency, or
   failure states; this draft does not claim those scenarios are already supported.

The fixture needs a Linux desktop, Python 3.11 or newer, and free port 17863.
Its publisher and services are simulated; no real broadcast rig is needed.
