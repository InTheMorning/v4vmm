# ADR 0059: Broadcast Control Surface

## Status

Implemented - 2026-09-10.

Tasks 001-017 are complete, including compact event controls, badges,
diagnostics, operator acceptance, and fixture cleanup.
Show action feedback task 001 is complete, including operator visual acceptance.

Reconciled 2026-09-10: the operator confirmed action feedback task 001 tested and
passed. A7-A9 are closed. Task 017's operator acceptance also passed, including
preservation and confirmation that all three registrations were intentional.
The operator confirmed fixture cleanup, closing task 017's final gate and
returning this ADR to Implemented. The packet retains the narrow-window
limitation as deferred work; no operator acceptance check remains open.

Amended 2026-09-10: [Show action feedback task 001](../tasks/show-action-feedback-task-001-command-state-and-result.md)
corrects stale service-state flashes and disappearing stream controls. Its
[verification inventory](../tasks/show-action-feedback-task-001-command-state-and-result.md#verification)
names the situational guards for command ownership and read-start freshness;
operator acceptance passed on 2026-09-10.

Amended 2026-09-09: the operator approved stored-event selection, compact
per-item status, and on-demand diagnostics after finding the accepted event row
too tall. This amendment also corrects attachment matching against an unused
target, extends Check to known event states, and keeps passive checks from
invalidating confirmed readiness merely by starting.
[Task 017](../tasks/adr-0059-task-017-compact-event-controls-and-badges.md) owns
implementation and the acceptance walkthrough; ADR 0063 owns the arrangement.

Clarified 2026-09-09 after packet review: configured-target attachment and
passive-check readiness are explicit invariants. Task 017 owns the database
execution sequence and source locations; this record owns their contract.

Tasks 001-015 are verified by the
[implementation review](../reviews/adr-0059-implementation-review.md).
[Task 016](../tasks/adr-0059-task-016-event-row-in-live-metadata.md) records Green
mechanical checks and operator acceptance of all event recovery checks in
[the fixture walkthrough](../runbooks/broadcast-event-recovery-check.md).

Reconciled 2026-09-09: the operator confirmed all broadcast event recovery tests
pass, closing the final gate opened by the amendment that moves Event into
Live Metadata. That verification returned the status to `Implemented`; the later
task 017 amendment above reopened it under ADR 0057 until its 2026-09-10 closure.

Amended 2026-09-09: event setup includes explicit Create, Replace, and retryable
Check actions. A dead entry stays selected, and a failed check after successful
registration must not strand the operator or discard the new identity. Task 016
specifies the action states and recovery tests. The same-day implementation
replaces the now-guarded event ownership, action, and readiness instructions
below with their guard references, following ADR 0061.

Amended 2026-09-06: the `Event` section must show the ready-to-paste
`podcast:liveValue` tag with a copy action. Listener apps discover a live event
only through that tag in the RSS feed of the show. Without the tag, the whole
chain reports success and no listener receives anything.

Amended 2026-09-06: the broadcast surface gains a fourth section, `Stream`, for
the stream encoder. `butt` has a control interface with a status request, a
connect and disconnect pair, and a network address option, so the section can
be built before any remote playback work. The three-section decision below
becomes four. Nothing else changes.

Amended 2026-09-06: the relay has a second death mode that the first draft did
not record. It removes an event after an idle TTL, and the default is 24 hours
(`splitkit`, `src/lib.rs`). The decisions below do not change, because the
liveness test already treats a `404` as a dead event for both modes.

Supersedes ADR 0018 and ADR 0019. This ADR carries the live decision for the
relay client surface. The work follows
`docs/plans/adr-0059-broadcast-control-surface-phase-plan.md`.

Amended 2026-09-10: the operator requires Event reports to name the event and
relay, explain the response and its consequence, and timestamp recorded
actions. Reports distinguish an HTTP error response from no response and from
a failure to save an otherwise valid answer. Task 017 owns this correction.

## Context

ADR 0018 and ADR 0019 put a live-publish client in this app. `src/api.rs`
creates live items and sends metadata. `src/cli.rs` gives the `v4vmm liveitem`
commands. No operator used this path in a real show.

The path sends the wrong body shape. The relay accepts two body forms. A body
with exactly the keys `event_id` and `metadata` is a wrapped body. Any other
body is a direct live value payload. This app sends the wrapped form.

Listener apps read `remoteValue` and expect the direct form. Payment splits are
in `value.destinations` in the direct form. The relay does not reject the
wrapped form. The failure is therefore silent.

`musicindex-live-publisher` sends the direct form. It is a headless service in
a separate repository. It reads a drop file, transforms the data, and sends the
result to the relay.

The drop-file contract is `musicindex.nowplaying/1`. ADR 0002 of that
repository defines the contract.

This app already starts the payment chain. `src/metadata.rs` writes
`TXXX:MusicIndex Value Routes` and the GUID tags into each download.

`mixxx-now-playing` reads the same tags from the audio file. The publisher then
sends those routes to listeners. The route data comes from this app, but at
download time only.

The operator wants more than one music player. `mpv` is the built-in player for
podcast work. Mixxx is the player for a DJ set.

Liquidsoap or Mixxx AutoDJ can hold the stream when the operator sleeps. The
publisher can run on a different machine than this app.

These facts about the relay control this design:

- The relay keeps state in memory only. A restart of the relay process discards
  live items, tokens, and snapshots.
- The relay removes an event after an idle TTL. The default is 24 hours. An
  event therefore dies without a restart.
- The relay has no route to list live items. It has no route to delete one.
- The relay returns the broadcaster token one time only. It keeps a hash of the
  token and does not return the token again.

## Decision

### This App Is A Control Surface

The broadcast chain must operate when this app is closed. This app shows state
and sends commands. It is not a part of the chain.

`mpv` is the one exception. `mpv` runs inside this app, so the `mpv` source
stops when the app stops. This limit is correct for desk work and is not a
defect.

### Sources Are Selectable

A source is a component that reports the current track. A source has a name, a
kind, a host, and an observation method. The kinds at this time are `mpv`,
`mixxx`, and `external`. Liquidsoap arrives later as one more kind.

The app must not put the name `mixxx` in shared logic. Only the source adapter
knows the kind.

### Observation Has Two Layers

Layer one is listener truth. The relay holds the payload that listener apps
receive. The app reads that payload with the relay client that it has today.

This layer operates for a local publisher and for a remote publisher, because
the relay is a network service.

Layer two is rig health. Unit state, drop-file presence, and log text tell the
operator if the local equipment operates correctly. This layer needs access to
the machine that runs the publisher.

The panel shows both layers together. A difference between the two layers is
the defect that the operator must see.

### This App Owns The Event Registry

The relay has no list route and no delete route. This app therefore keeps the
list of events that the operator created.

- `Create` calls the relay and stores the result.
- `Forget` removes the local record. The relay discards its own record on the
  next restart.
- `Resume` selects a stored event and then tests it. A `404` response shows
  that the event is dead.

A dead event must not cause an automatic replacement. A new event has a new
identifier, and listeners must then tune again. The operator makes that choice.

Amended 2026-09-09: registry command ownership and separation from publisher
mutation are enforced by
`adr_0059_event_row_precedes_services_and_registry_actions_do_not_attach`
(situational, ADR 0059) in `tests/architecture_tests.rs`.

The `show_event_*` tests in `src/app/show.rs` and `src/view_models/show.rs`
enforce registration/check transitions, typed action availability, preserved
identity and token files, retention of the old dead entry, and mounted-row
projection. They cover failed reads, retry Live, retry 404, repeated failure,
failed status storage, and obsolete refreshes.

A dead entry stays in the registry, so an operator needs Replace as well as
Create. Registration and checking have separate results because a failed query
does not undo the identity the relay just issued. Attachment is a separate
operator decision because it changes publisher configuration.
[Task 016](../tasks/adr-0059-task-016-event-row-in-live-metadata.md) records the
implementation and completed operator verification.

### Event Reports Name The Action And Outcome

Amended 2026-09-10. Event reports identify the event, relay, publisher target,
or stored choice involved in each reported action. A result explains what the
app requested, what answered, and what the app saved or could not establish.
An HTTP error proves that a response arrived. It does not prove that the event
exists or that the publisher is sending metadata. A missing response proves
neither. A failed local write must not be reported as an unreachable relay.

`event_report_check_failures_use_typed_response_facts` and
`event_report_success_names_the_answer_and_saved_state` in
`src/view_models/show/event_report.rs` enforce the response distinctions and
their consequences. Registry and application tests verify those facts against
HTTP responses, disconnects, and failed database writes. These are situational
ADR 0059 guards. ADR 0063 owns recorded-time and plain-text presentation.
This tightens task 017's existing independent-result contract and changes no
event or publisher command semantics.

### Stored Event Selection Is Explicit And Persistent

Amended 2026-09-09. The existing Resume decision already authorizes choosing
and checking a stored event. This section adds persistence, command-boundary
validation, and manual Check for Live and Dead as well as Unknown. It does not
change who registers events or who publishes metadata.

Selection is now enforced by `broadcast_selection_is_persistent_and_revisioned`
in `src/db.rs` and `compact_event_saved_choice_drives_commands_and_refresh`,
`compact_event_missing_choice_and_registry_errors_are_explicit`, and
`compact_event_registration_survives_selection_save_failure` in `src/app/show.rs`
(situational, ADR 0059). They cover persistence, missing references, command
revalidation, known-state checks, and separate registration/storage results.

The bounded picker and distinguishing identities are covered by
`compact_event_logs_picker_and_diagnostics_are_identity_scoped` in
`src/view_models/show.rs` (situational, ADR 0063). Resume changes the app's
choice. Publisher configuration remains a separate operator decision.

### Attachment Names The Configured Target

Corrected 2026-09-10 under the 2026-09-09 amendment (ADR 0057).
The old event-ID-only lookup could report Attached from a stale unused target
while the configured target carried another event. That false confirmation
would defeat the purpose of the badge.

`compact_event_configured_target_and_remote_hint` and
`compact_event_context_revision_and_configured_detach_are_scoped` enforce the
configured-name-plus-ID correction, including empty names and duplicate
associations (situational, ADR 0059). The visible replacement notice names the
currently configured event. Configuration is not proof of delivery.

`adr_0059_packet_014_attach_event_replaces_target_and_restarts_publisher` retains
the target service's explicit replacement/restart contract. The fixture's
`test_named_target_replacement_and_removal_preserve_other_associations` guards
unrelated targets. `adr_0059_event_target_commands_share_publisher_ownership`
guards partial restart failure and readback through the existing service-command
owner. These guards are situational, ADR 0059. Selecting or checking an event
has no publisher mutation capability; registry rows and token files are retained.

### Event Actions Keep Independent Results

The six `show_event_*recovery*` cases named in
[task 017's assertion inventory](../tasks/adr-0059-task-017-compact-event-controls-and-badges.md#existing-assertions-to-update-in-the-implementation-change)
retain the original registration/check separation and preservation proof.
`show_event_unknown_check_retry_and_working_actions_are_typed`,
`compact_event_passive_checks_and_mutations_have_distinct_readiness`, and
`compact_event_registration_survives_selection_save_failure` cover the extended
action states (situational, ADR 0059).

A failed read does not undo a successful registration. A failed restart does
not undo a successful configuration write. These are separate outcomes in Event
Logs, with one short explanation in the compact item. Logs and Copy remain
usable during work and failure. Creating another event while a live selection
exists, renaming events, and UI Forget remain outside this amendment.

### Item Readiness Determines Section Readiness

Implemented by `compact_event_badge_tables_and_card_equivalence`,
`show_event_readiness_table_covers_liveness_and_every_attachment_result`, and
`show_event_readiness_table_covers_every_service_state` in `src/view_models/show.rs`
(situational, ADR 0059). They hold the complete kind/label mappings, role-identity
rules, and aggregate readiness contract. `event_badge`, `service_badge`, and
`live_metadata_card_state` are the projection owners; ADR 0063 owns presentation.

Ready describes Event/Producer/Publisher prerequisites, not proof of audio
connectivity, RSS publication, or listener delivery. Event says Attached and
services say Active; Ready is reserved for the card. Each item's badge exposes
its own failure, even when an earlier item determines the card's state.

### Passive Checks Preserve Confirmation Until They Answer

A question has not changed the confirmed facts. Starting a passive check must
not imply that a healthy chain stopped. A failed answer does change what is
known, even if the database retains historical liveness.

`compact_event_passive_checks_and_mutations_have_distinct_readiness` enforces
retained confirmation while pending and immediate application of independent
read failures. `compact_event_context_revision_and_configured_detach_are_scoped`
protects context changes. `adr_0059_event_target_commands_share_publisher_ownership`
reuses the bounded fresh-observation policy from
[Show action feedback task 001](../tasks/show-action-feedback-task-001-command-state-and-result.md#verification).
All are situational ADR 0059 guards, cited by the invariants below.

### Tokens Are Files

Nobody can replace a broadcaster token. The app writes each token to its own
file with mode `0600`. The app stores the file path in the database. The app
does not store the token in the database.

This matches how `musicindex-live-publisher` reads tokens today. It also lets
the operator copy one token to another machine.

### The Publisher Owns Its Configuration

`musicindex-live-publisher` owns its configuration file and its setup rules.
This app reads that file to show its content. This app does not write it.

All changes go through the publisher command-line tools. This app runs
`musicindex-live-publisher provision` and `setup-mixxx-musicindex`, and reads
the exit code and the output.

### Remote Control Uses SSH First

For a remote host, this app runs `systemctl --user` through `ssh`. The
publisher repository needs no change for this step.

A control API in the publisher is deferred. The liquidsoap work needs a larger
remote surface than start and stop. That surface is easier to design when those
requirements exist.

### The Publish Path Is Removed

Phase 1 removes `publish_live_metadata`, `publish_live_metadata_with_token`,
and the `v4vmm liveitem publish` commands. They send the wrong shape and have
no user.

`create_live_item`, `fetch_live_metadata_optional`, and `health` remain. Event
registration and listener truth need them.

### Lists From The Start

Events, sources, and hosts are lists in the data model and in the service
layer. The first user interface shows one selection at a time. Support for more
than one stream must not need a data model change.

### The Broadcast Sections Live In Show

Amended 2026-09-08 by ADR 0060, which removed the `Broadcast` frame. The four
broadcast sections mount inside the `Show` screen. Amended 2026-09-09 to three,
in this order: `Source`, `Live Metadata`, and `Stream`. `Event` became the first
row of `Live Metadata`.

Amended 2026-09-08. The section that holds the two services is named
`Live Metadata`, not `Publisher`. It holds the metadata producer and the
metadata publisher, so the section name states what the two services do. The
services keep the names `Producer` and `Publisher` inside it.

Event's nested ownership and detail order are enforced by
`adr_0059_event_row_precedes_services_and_registry_actions_do_not_attach`
(situational, ADR 0059). The `show_event_readiness_table_*` view-model tests
cover the combined event, attachment, and service state.

An event is the identity the services publish to. Without a live, attached
event, two running services still publish nothing. A failed attachment query
provides no evidence of absence, and a stopped service does no work. Chain
order puts the earliest problem before the components that depend on it.

A section is an optional field on `ShowPageVm` and a group of callbacks on
`ShowSlots`. An absent section renders nothing. It does not render as
unavailable.

`Event` identifies the live item and makes the exact RSS tag available through
its diagnostics and Copy feed tag action, as arranged by ADR 0063:

```xml
<podcast:liveValue uri="EVENT_ID" protocol="socket.io"/>
```

The operator copies that tag into the feed of the show. This app does not write
the feed. Feed publication is future work, recorded in
`docs/research/broadcast-recording-and-feed-publishing.md`.

`Stream` shows the encoder that feeds the listeners, which is `butt` today.
`butt` accepts control on the command line and over a network address, so a
local encoder and a remote encoder use one code path and neither needs `ssh`.
The section reports the connection state and the recording state, and offers
connect and disconnect.

The app does not send a song title to the encoder. The producer already writes
the text file that the encoder reads.

The queue keeps its name and its meaning. It shows local playback in this app.
ADR 0060 moved it into `Show` beside these sections. The queue and the
broadcast sections stay separate in code and in the interface.

## Invariants

- The broadcast chain operates when this app is closed, for every source except
  `mpv`.
- No component writes the configuration file of `musicindex-live-publisher`
  except the publisher tools.
- Broadcaster tokens are files with mode `0600`. No token is in the database.
- The app reports a dead event to the operator. The app does not replace it
  automatically.
- Only the configured target on the selected host and publisher instance can
  satisfy attachment. Its normalized name must be nonempty and its event ID
  must match the selected event; an unused target cannot satisfy attachment.
- A passive check retains the prior readiness confirmation for the same event
  and publisher context while pending. Its result changes readiness when it
  arrives, not when the check starts; each failed read invalidates its own
  confirmation even when another read remains pending or succeeds.
- The app sends no metadata to the relay. The publisher is the only sender.
- Source kind names appear in source adapters only.
- Encoder commands run only in the broadcast service layer, never in a screen.
- The app never sends a song title to the encoder.
- Relay clients come from `src/http_client.rs`, as ADR 0058 requires.
- A runtime actor runs all work that blocks, as ADR 0040 requires.

## Alternatives Considered

### Always Select The Newest Stored Event

Rejected by the 2026-09-09 amendment. Resume already permits an older entry;
refreshing or validating against the newest row defeats that choice. Persist
the chosen identity and validate commands against it.

### Clear Readiness When A Passive Check Starts

Rejected by the 2026-09-09 amendment. A request has not changed the confirmed
facts. Keep its activity separate, then apply its success or failure. A failed
result still invalidates readiness immediately.

### Keep The Built-In Publish Path

Rejected. The path sends a body that listener apps cannot read for payment
splits, and no show used it. A second sender also gives two owners for one
event, with no lock between them.

### Add `musicindex-live-publisher` As A Library Dependency

Rejected. The publisher makes its own HTTP client that blocks. It owns its own
timeout constants. ADR 0058 forbids both in this app. The publisher is also a
separate package that can be absent or remote. A library dependency cannot show
that condition.

### Write The Publisher Configuration Directly

Rejected. The publisher setup script holds marker checks, backup steps, and
placeholder tests. A second writer would copy that logic and then drift from
it.

### Add A Control API To The Publisher Now

Rejected for this stage, but not rejected forever. An endpoint that starts and
stops services is a remote-command surface and needs authentication, a local
default bind address, and review. SSH gives the same result now with no new
surface. The liquidsoap work is the correct moment for that API.

### Make This App A Drop-File Producer For Every Source

Rejected. Mixxx and remote hosts already have producers. Only the `mpv` source
needs a producer in this app.

## Consequences

Positive:

- The app shows what listeners receive, for a local publisher and a remote
  publisher, with the client that already exists.
- Removal of the publish path deletes the only untested network write.
- The event registry keeps tokens that the relay cannot return again.
- One source model holds `mpv`, Mixxx, and liquidsoap.
- The three panel sections give the later interface work three separate owners.

The 2026-09-09 amendment makes each prerequisite inspectable independently and
prevents an unused target from satisfying readiness. Selection now needs a
migration and revision ownership; a missing reference must remain visible.
Passive checks retain last-confirmed readiness until they answer, which is a
continuity choice rather than proof of continuous delivery. Publisher changes
and the following restart remain separate outcomes.

Negative and risks:

- This app becomes the keeper of secrets that nobody can replace. If the
  operator loses a token file, that event dies.
- A relay restart kills every event. The operator must then create new events
  and tell listeners.
- SSH control needs key access to the remote host. An operator without keys
  cannot use the buttons.
- A publisher unit that fails five times in 300 seconds stays in the `failed`
  state. The panel must show that state, or `Start` appears to do nothing.
- The `mpv` producer adds a second writer of drop files in this project.

## Amendment Verification

Task 017 is built. Its [verification inventory](../tasks/adr-0059-task-017-compact-event-controls-and-badges.md#verification)
records the mechanical checks and the named regression owners. Guard references
above replace the implementation instructions they enforce, retaining the
incident and decision rationale under ADR 0061.

The [task 017 operator visual check](../tasks/adr-0059-task-017-compact-event-controls-and-badges.md#operator-visual-check)
is the passed situational ADR 0059 manual check for readiness feedback and usable
recovery. It also serves the separately identified ADR 0063 presentation checks.
Tests can prove state kinds and command effects but cannot establish that a
person can read the badges, find the controls, and follow the interaction.
The operator confirmed fixture cleanup on 2026-09-10. The completed gate is
removed from [pending human checks](../pending-human-checks.md) and recorded
in the delivery order. Task 017 retains its operator evidence and the deferred
layout/log observations.

## Follow-Up Work

- Layout, log readability, following, and timestamp consistency remain
  separate follow-ups recorded in [task 017](../tasks/adr-0059-task-017-compact-event-controls-and-badges.md#operator-visual-check)
  and its linked plans; they are not open acceptance gates on this amendment.

- `splitkit`: add long-lived live items. Weekly shows and permanent stations
  need an event that survives a relay restart.
- `musicindex-live-publisher`: add a remote control API when liquidsoap
  control starts.
- Liquidsoap source support, after the control API exists.
- Icecast state, if the operator needs it next to the publisher state.

## References

- ADR 0014 - PlaybackSession authoritative state
- ADR 0016 - Schema migration discipline
- ADR 0017 - CLI debug contracts
- ADR 0040 - Async view-model runtime
- ADR 0046 - Workspace frame architecture
- ADR 0057 - ADR status vocabulary and amendment policy
- ADR 0058 - Outbound HTTP client policy
- `musicindex-live-publisher` ADR 0002 - Now-playing drop-file contract
- `docs/plans/adr-0059-broadcast-control-surface-phase-plan.md`
