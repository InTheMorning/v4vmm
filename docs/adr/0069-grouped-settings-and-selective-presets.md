# ADR 0069: Grouped Settings And Selective Presets

## Status

Accepted - 2026-09-11.

The operator accepted grouped Settings, selectable live metadata resources,
independent audio outputs and console-style selective preset save/recall.
Implementation has not started. The [phase plan](../plans/adr-0069-settings-presets-phase-plan.md)
and [first packet](../tasks/adr-0069-task-001-grouped-settings-foundation.md)
separate the existing-field foundation from guarded editing, new configuration
and playback integration. ADR 0068 remains Proposed; accepting this Settings
decision does not accept its entire playback design or resume playback work.

## Context

Settings currently combines appearance, library paths, the MusicIndex endpoint,
conversion tools, cached files and background reports in one scrolling form.
`render_settings` and `TopApp::save_settings` in `src/app.rs` compose that form
and couple ordinary persistence to updates of runtime owners.

Show already lets the operator select an Event and operate a Producer and
Publisher. Selecting their configured resources needs a durable Settings home.
The operator wants Mixxx and built-in operating modes, separately selectable
Show and Music audition outputs, and favorite setups assembled from saved
presets. Copying a whole configuration file cannot express selective recall
without also changing unrelated machine settings or active show state.

[ADR 0066](0066-configuration-and-startup-failure-recovery.md) owns scoped
validation, preservation, shared Settings/recovery repair and explicit retry.
[ADR 0068](0068-show-cue-and-audition-isolation.md) proposes independent Show
and audition owners. This decision supplies their Settings and preset contract;
it does not implement either lifecycle.

## Decision

### Grouped Settings Within The Existing App Section

Keep Music, Show and Settings as the three app sections. Settings has its own
labeled group tabs, a persistent indication of the selected group and a bounded
scroll area for the group's content. Remember the selected group within the
session. Changing groups or leaving Settings retains the working draft.

| Group | Configuration or maintenance owned here |
|---|---|
| General | Operating mode, theme and interface scale |
| Live Metadata | Producer resource, publisher host/instance and their connection settings |
| Audio | Independent Show playback and Music audition backend/output settings |
| Library | Music storage, MusicIndex endpoint and download/conversion tools |
| Diagnostics | Configuration issues, tool checks, cached-file inspection and maintenance reports |

Preset selection, Save Preset and Recall belong above the group tabs when
preset commands exist. They act across groups, with an explicit component mask.
Do not publish empty future tabs or nonfunctional preset controls. The first
packet groups the existing General, Library and Diagnostics content; later
packets add Live Metadata, Audio and preset controls with their live owners.
Once delivered, a group's navigation remains stable when a resource is missing.

Group navigation, labels, field state, notices and action availability come
from renderer-independent view models. Shared form and navigation geometry
uses existing primitives/composites and named tokens. The Settings screen
only composes content, owns GPUI input/focus wiring and dispatches commands.
Long reports belong in Diagnostics; Open report selects that group directly.
Validation near an edited field must not require a trip to Diagnostics.

### Settings Configures Resources; Show Operates Them

An operating mode is a choice of intended workflow, initially Mixxx or built-in.
It proposes compatible producer, publisher and audio defaults in the draft.
It is not a continuously enforced master switch. Existing explicit choices
are not overwritten by changing mode. Where they conflict with the proposed
mode, explain the conflict and let the operator change the selected components.
Never expand a recall mask silently to make a combination work.

The producer describes the source of live metadata: an external producer
service or the built-in Show producer. The publisher identifies a host and
publisher instance consuming that producer's output. Audio routing is a
different component. Configuring Mixxx mode does not give v4vmm ownership of
Mixxx's audio engine; v4vmm continues to own its audition output.

Settings editors and any Show quick selectors share resource identities,
validation and configuration commands. Event selection stays in Show, using
the existing registry. Start, Stop and Attach remain explicit Show operations.
Saving a different publisher selection does not stop the previously selected
external service, start the new service or attach the selected Event.
External services must remain independent of this app's lifetime.

### Draft, Saved Configuration And Running Resources Are Distinct

Represent three different facts: the draft being edited, the last successfully
saved configuration, and the configuration a running resource has applied.
Do not label a recalled or merely saved setup as active on the running chain.

Recall changes the draft only. Show the selected changes and validate the
combined draft before saving. Failed validation or persistence retains the
draft. Cancel restores the last saved draft; appearance preview, if offered,
must be reversible. Saving an invalid optional component cannot damage valid
sibling settings or turn an unavailable tool into a startup requirement.

Configuration writes use ADR 0066's shared guarded editor: original revision
and identity, validation, preserved original, atomic publication and refusal
to overwrite an externally changed file. Settings and recovery must not grow
separate writers. Ordinary saves remain paused where ADR 0066 requires it.
Saving alone never starts/restarts a service, retries a failed operation,
attaches an Event, begins playback or reroutes active audio.

An explicit apply action checks the current resource generation and names its
impact. Changes requiring an idle player or managed session transition wait
for that state. A stale apply completion cannot replace a newer selection.
Failure leaves the previous running resource state accurately reported and
the new saved choice available for an explicit retry. Do not claim a distributed
transaction across independently managed services. Appearance and other safe
local preferences may apply without a service transition.

This is the target editor contract. Task 001 only moves existing controls and
preserves their current Save, Use Defaults and appearance-preview behavior.
It does not claim to deliver the new transaction or Cancel behavior. Those
arrive in the guarded-editor phase before new selectors or preset recall ship.

### Selective Preset Save And Recall

Presets are named, versioned snapshots of selected configuration components.
They have stable identifiers independent of display names. They do not inherit
live values from other presets, and recalling them does not bind the active
setup to future edits of their source files.

Initial selectable components are Operating mode, Producer, Publisher, Show
audio, Audition audio and Appearance. A component is a coherent unit of
configuration. For example, an audio component carries its backend and that
backend's destination/channel options together. Producer/Publisher are separate
checkboxes, but their combined connection must still validate.

On Save Preset, checkboxes select what the preset contains. Capture validated
values from the reviewed draft without applying them to running resources.
On Recall, checkboxes select which contained components to copy into the draft.
The interface distinguishes a component absent from the preset from one that
is present but unchecked. Neither state means clear or reset to defaults.

Recall several presets in sequence to assemble a setup. Each recall replaces
only its selected components. A later recall wins for an overlapping selected
component; every other draft value is retained. Show the resulting differences
and identify which preset supplied each recalled component while editing.
Manual changes mark that component as edited. A combined setup is Custom until
saved as a new preset; it is not misleadingly named after the last recalled part.

No preset changes cue contents, playback position, service running state,
observed health, library/database paths or library records. Event selection
is excluded from the initial mask. A future explicit Event component would be
unchecked by default, resolve an existing registry identity and still require
Attach as a separate action. Tokens are never copied into presets; protected
credential files remain the authority for any referenced resource.

### Configuration And Preset Storage

`config.toml` remains the saved active configuration. Named preset documents
live separately under the app's configuration directory and reuse the same
typed component definitions and validators. Do not add a second configuration
language or encode arbitrary TOML key patches as presets.

The persistence packet must fix the versioned on-disk schema and document its
compatibility tests before writing it. File identity is based on the preset ID,
not a user-entered name used as a path. Explicit overwrite preserves the prior
document and rejects concurrent changes. Reject unsupported schema versions
without modifying the preset or applying a partially decoded selection.
An unreadable preset disables that recall, not normal app startup.

Retain older configuration keys and documented defaults. Missing new optional
fields do not trigger a startup rewrite. Unedited settings survive focused
writes. Grouping the UI does not require renaming TOML sections, merging the
workspace sections or migrating a user's existing configuration.

### Audio Backend Contract

Show audio and Audition audio each own a backend selection and destination.
Support PulseAudio and JACK first, with native PipeWire as a later adapter.
Use a common output-purpose model with backend-specific options: JACK ports
and connections must not be represented as PulseAudio sink identifiers.
Enumerate capabilities of the installed backend; configuration validity and
current device availability are separate facts.

Preserve an unavailable destination's identity and explain what is missing.
Never silently route audition to Show or substitute the system default when a
selected destination disappears. App-selected audition routing is the accepted
Settings direction. It does not by itself establish independent audio owners.
Audio selectors must wait for ADR 0068's owner/route isolation and verification;
relabeling the current shared player is insufficient.

mpv documents PulseAudio, JACK and PipeWire outputs and separate JACK port
options. This establishes a possible adapter route, not support verified in
the operator's installed build. See the [mpv audio output manual](https://mpv.io/manual/stable/#audio-output-drivers).

## Invariants

1. Settings group navigation preserves edits and invokes no resource operation.
2. Mode defaults and recall never silently overwrite unselected components.
3. Recall affects only the draft; saved and running resource facts remain distinct.
4. Settings and recovery share guarded configuration validation and persistence.
5. Presets are snapshots; absent/unchecked components preserve existing values.
6. Session contents, library identity, secrets and observed health are not presets.
7. Show and audition retain independent output configuration and failure scope.
8. New controls have live typed owners, shared UI paths and named proof before shipping.

## Non-Goals

No playback implementation, Mixxx audio-engine control, native PipeWire adapter,
system-wide JACK/PulseAudio configuration, general patch-bay editor, plugin
settings framework, arbitrary field-level masks, preset inheritance, cloud
sync, preset import/export or automatic service activation. No wholesale TOML
migration or library relocation. No existing operator gate is closed here.

## Alternatives Considered

- One growing Settings form: rejected because unrelated configuration and
  reports obscure each other as capabilities grow.
- Whole-config presets: rejected because choosing an audio setup should not
  retarget the library or overwrite unrelated preferences.
- Live inheritance between presets: rejected because later preset edits would
  alter assembled setups without an explicit recall.
- Immediate operational recall: rejected because loading a favorite setup must
  not attach a different Event or interrupt a running show.
- Desktop routing alone: useful alongside app controls, but does not meet the
  accepted requirement to select and recall audition destinations in Settings.

## Consequences And Follow-Up

The app gains one organized configuration surface and repeatable partial
setups. It also needs draft/change tracking, component compatibility checks,
versioned preset persistence and truthful saved-versus-running state.

The [delivery order](../plans/broadcast-chain-delivery-order.md) prioritizes
the existing-field foundation while playback stays deferred. ADR 0066 remains
the prerequisite for guarded editing and configuration-format changes; this
decision neither releases that gate nor accepts its paused playback evidence.
Further packets follow the [phase plan](../plans/adr-0069-settings-presets-phase-plan.md)
and [review checklist](../reviews/adr-0069-settings-presets-review-checklist.md).
