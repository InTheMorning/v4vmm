# ADR 0059: Broadcast Control Surface

## Status

Accepted - 2026-09-09.

Implementation partial: tasks 001-016 complete; task 017 compact event controls,
badges, and diagnostics implementation and operator verification outstanding.
Show action feedback task 001 is complete, including operator visual acceptance.

Reconciled 2026-09-10: the operator confirmed action feedback task 001 tested and
passed. A7-A9 are closed; task 017 is ready to implement.

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
implementation and the new open visual gate; ADR 0063 owns the arrangement.

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
task 017 amendment above reopens it under ADR 0057.

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

### Stored Event Selection Is Explicit And Persistent

Amended 2026-09-09. The existing Resume decision already authorizes choosing
and checking a stored event. This section adds persistence, command-boundary
validation, and manual Check for Live and Dead as well as Unknown. It does not
change who registers events or who publishes metadata.

Clicking the selected event opens a bounded, scrollable list of locally stored
events, newest first. Each entry has its existing label, a creation date, a
distinguishing identifier, and its last known liveness state. Unnamed events
use the date and identifier as their display name. Short identifiers must be
disambiguated if they collide; selection always uses the full identity.

The list marks the selected entry. A successful publisher configuration read
may separately mark the entry configured on the selected host, instance, and
`drop_file_target`. That mark means configured, not proof that metadata is
currently flowing. Unknown or failed reads cannot supply an affirmative
configuration mark.

Selecting an entry updates the mounted view and requests a liveness check for
that entry, together with a fresh publisher configuration read. This includes
previously Live or Dead entries: the current Unknown-only manual-check
restriction needs an explicit extension. Opening the list alone makes no
network request for every entry.

The selected event survives refresh, navigation, and restart through a
database-scoped selection preference, owned by `src/db.rs` and added through a
migration. The preference records the full event ID and a selection revision.
An uninitialized preference may choose the newest row once and persist that
choice. If its saved entry has been
removed, retain that missing reference, show Event unavailable, and let the
operator choose another. No cascading deletion may silently select a different
identity. Loading or failing to read the registry is distinct from an empty
registry and cannot enable Create.

`EventRegistryCommand::execute` and `RefreshShowPage::execute` must resolve the
same persisted selection. Command eligibility uses that selected row's current
stored state and rejects a changed selection revision or missing selected row.
Check may operate on any existing selected Live, Dead, or Unknown row; Replace
still requires Dead. Create requires a successfully read empty registry, not
merely a missing or failed selection lookup. An unrelated newer row cannot
reject a valid command against the older saved selection.

Selection-dependent results belong to the persisted event identity, selection
revision, host, instance, target name, and request as applicable. A result may
update its own registry facts but cannot overwrite a different mounted selection.
If saving selection fails after registration succeeded, the created row and
token remain available in the refreshed list. Report the separate save failure;
it does not authorize another registration.

Successful Create or Replace adds and selects its new entry immediately, then
checks it as task 016 does today. Replace retains the old entry and token file.
Create/Replace keep their existing eligibility; creating additional
events while a live one is selected and adding a rename editor are outside
task 017.

### Attachment Names The Configured Target

Amended 2026-09-09 as an implementation correction under ADR 0057.
`EventTargetAttachmentDisplay::from_input` currently matches any target carrying
the event ID, while Attach writes `broadcast.drop_file_target`. A stale unused
target therefore reports Attached even when the configured target carries
another event. The readiness read, Attach, Detach, and picker association mark
must all use the same selected host, publisher instance, and normalized
configured target name. Match both target name and event ID. An empty target
name cannot pass.

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
first. The action makes that replacement clear before the press, including
which event is currently configured. The prior event and token file
remain in the local registry even after the publisher uses another event.

This configuration replacement and the following service restart are separate
operations. A successful write followed by a failed restart is a partial result
that must remain visible. Detach stays available in the overflow menu for an
event associated with the configured target, including a dead one, but it is
not a prerequisite for using another event. Arbitrary publisher-target
management is outside task 017.

### Event Actions Keep Independent Results

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

### Item Readiness Determines Section Readiness

Amended 2026-09-09. The view model owns these state kinds and labels; ADR 0063
owns their badge presentation. Ready refers to these prerequisites, not proof
of audio connectivity, feed publication, or listener delivery.

The three items are exactly Event, Producer, and Publisher, identified by role,
not by vector length. Once Live Metadata is mounted, missing Event input projects
an Unknown/Loading placeholder; a missing service observation projects that
role's Unknown/Status unavailable placeholder. Duplicate observations for a role
are invalid input for that role and also project Status unavailable. A missing
Publisher cannot be satisfied by two Producer observations.

Each item exposes a renderer-independent state kind and label. Using the existing
`ShowCardStateKind` vocabulary, the card state is `Ok` if and only if Event,
Producer, and Publisher each project `Ok`. ADR 0063 maps those kinds to shared badge tokens.
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

### Passive Checks Preserve Confirmation Until They Answer

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
command ownership and fresh-observation policy guarded by
[Show action feedback task 001](../tasks/show-action-feedback-task-001-command-state-and-result.md#verification),
rather than implementing a second transition policy.

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

Task 017 is unimplemented. Existing task 016 tests and guards verify the prior
shipped scope, not the new attachment definition, saved selection, or badge
contract. The [task 017 mechanical criteria and assertion inventory](../tasks/adr-0059-task-017-compact-event-controls-and-badges.md#acceptance-criteria)
name the cases and the exact existing tests to update in the implementation
commit. Each new behavior test is situational, ADR 0059. When a guard replaces
these implementation instructions, replace that prose with the named coverage
in the same change, retaining the incident and decision rationale.

The [task 017 operator visual check](../tasks/adr-0059-task-017-compact-event-controls-and-badges.md#operator-visual-check)
is the open situational ADR 0059 manual check for readiness feedback and usable
recovery. It also serves the separately identified ADR 0063 presentation checks.
Tests can prove state kinds and command effects but cannot establish that a
person can read the badges, find the controls, and follow the interaction.
The gate is listed in [pending human checks](../pending-human-checks.md) and
the delivery order; task 016's passed check does not close it.

## Follow-Up Work

- Implement [task 017](../tasks/adr-0059-task-017-compact-event-controls-and-badges.md)
  after its action-feedback prerequisite and obtain operator acceptance.

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
