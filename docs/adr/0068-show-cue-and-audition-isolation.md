# ADR 0068: Show Cue And Audition Isolation

## Status

Proposed - 2026-09-11.

The operator requested the workflow separation during ADR 0066 task 004:
load a playlist into the Show cue, play it from Show, and use other playback
buttons for a separate audition audio path. The storage, routing and lifecycle
decisions below are proposed; implementation has not started. Writing this ADR
does not accept task 004's paused playback checks or change the delivery order.

Revised 2026-09-11: accepted [ADR 0069](0069-grouped-settings-and-selective-presets.md)
owns grouped Settings, app-selected Show/audition outputs, selective presets
and PulseAudio/JACK-first configuration. It resolves the app-versus-desktop-only
output-selection choice; this playback proposal remains otherwise Proposed
and its implementation remains deferred.

## Context

[ADR 0060](0060-workflow-surface-structure.md) places the queue and transport in
Show. It reserves Music playback for audition and forbids audition from setting
now-playing state. That separation is not implemented.

The current Music playlist action emits `LibraryAppEvent::PlayPlaylistAt`.
`TopApp::play_playlist_at` uses the same `PlaybackOwner` as Show transport.
The owner immediately loads the driver, updates the canonical playback session
and synchronizes drop-file metadata when a producer exists. The cuelist is then
projected from the active session's source playlist. There is no independent
loaded cue that the operator can inspect before starting audio.

During the producer-failure fixture check, the operator rejected instructions
to play a track in Music and then open Show. The screenshot also contains an
mpv IPC read error. [Task 004](../tasks/adr-0066-task-004-optional-tool-isolation.md#playback-workflow-correction--2026-09-11)
records both findings separately. Preservation passed; playback did not.

A second instance of the current driver is insufficient by itself. Its socket
path includes only the application process ID, so two instances in the same
runtime directory would collide. Separate playback also requires separate
commands, state, audio routing and failure handling.

## Decision

### Two Playback Purposes

| Purpose | Operator action | Audio | State and publication |
|---|---|---|---|
| Prepare Show | Load a playlist into the Show cue | None | Stores cue order and selection; no now-playing update |
| Run Show | Start and control the loaded cue from Show | Program output | Updates the Show playback session; only this path may produce local Show metadata |
| Audition | Audition one track from Music | Monitor output | Updates audition state only; never changes the Show cue, session, event, producer or recording |

Loading a cue, auditioning, and starting Show are distinct typed commands.
Neither the visible section nor a boolean supplied to a generic Play command
decides whether the command publishes metadata.

### The Show Cue Exists Before Playback

Provide an explicitly named `Load into Show cue` action for a playlist. It
copies the playlist's ordered track references into one persisted Show cue.
It selects the first entry and updates the mounted Show view without starting
the driver, activating a broadcast, selecting a publisher event or writing a
now-playing payload. Loading does not require an available audio device.

The cue is a snapshot of membership and order, not a live alias of a playlist.
Each entry has its own identity, separate from track identity, and retains its
source playlist/position as provenance. Repeated occurrences of a track remain
distinct. Editing, reordering or deleting the source playlist does not change
the loaded cue. Titles, source metadata and file readiness continue to come
from their existing owners; this snapshot does not create a second metadata
truth or copy the audio files.

Store the cue, its ordered entries and its selected entry through the database
migration registry. Do not encode a loaded cue as a paused or playing session.
A failed load transaction retains the previous cue intact. Empty playlists
cannot replace a cue. An unavailable track stays visible with a typed reason;
loading must not silently discard it. Library removal must account for cue
references, and an externally missing file must leave a diagnosable entry.

Loading into an empty cue and replacing a populated cue have distinct labels.
Replacement and clearing are unavailable while Show is playing or paused;
the operator stops Show first. The first implementation loads or replaces a
whole playlist. Appending, reordering individual cue entries, multiple stored
shows and scheduling are separate follow-ups.

### Show Owns Its Transport

Show renders the loaded cue even when no track has played. A loaded cue alone
does not make a show active. Existing independently observed broadcast activity
continues to determine the live status strip under ADRs 0059/0060.

Show Play starts the selected cue entry; Pause/Resume acts on that same entry.
Stop ends local audio and local metadata publication, resets the selected entry's
playback position, and retains the cue. Selecting another entry while stopped
changes only the selection. Track selection does not implicitly start audio.

Next/Previous while stopped selects the adjacent entry without playing it.
While playing it loads the adjacent entry; while paused it selects and prepares
the adjacent entry while preserving pause, without an audible burst. These
commands follow cue-entry identities, not the source playlist's current rows.
They are unavailable at the corresponding boundary. Unavailable entries are
not silently skipped: the command reports the target and leaves the current
state intact. The operator may stop and select another entry explicitly.

EOF advances once to the next cue entry. At the final entry it stops without
looping and retains the cue. If the next entry cannot play, Show stops and
reports that entry; it does not silently jump ahead. Driver observations carry
the owner generation and active cue-entry identity so a late EOF or command
reply cannot advance a replacement track or a different cue.

The Show view model exposes distinct empty, loaded/stopped, starting, playing,
paused and failed states, with action availability and failure details. A
successful command admission is not proof that audio started. Publication
failure remains a separate fact from audio state, as required by ADR 0066.

### Audition Is One Independent Track

Track playback affordances in Music become clearly labeled audition actions,
including playlist rows and track details. They address one track at a time.
Starting another audition replaces only the current audition. EOF stops that
audition; it never advances a playlist or the Show cue.

Audition has its own idle, starting, playing, paused and failed state, position,
and Start/Pause/Resume/Seek/Stop commands. It has no Show session ID, cue cursor,
publisher credentials, drop-file producer or recording capability. Its owner
may resolve a local track through existing read services; it cannot write Show
playback state. A different session ID on the existing producer-capable owner
does not establish this boundary.

Audition remains an inline Music utility with progress and a reachable stop
action. If navigation within Music removes the originating row, a compact
active-audition control remains within Music. Leaving Music stops audition;
Show playback continues through section navigation. Audition does not activate
the Show status strip. It can run while a Show or external broadcast is active
when the configured monitor path is available.

The first implementation auditions validated local files under ADR 0064.
Remote-only tracks retain their acquisition actions and expose why local
audition is unavailable. Direct enclosure streaming, caching policy, waveforms,
mixing and audition playlists are outside this decision.

### Independent Audio Outputs And Driver Instances

ADR 0069 selects the monitor output in v4vmm. Show
and audition have independent output settings and separately identifiable
audio streams. Keep the existing Show playback settings compatible; add scoped
audition settings without rewriting an older configuration on startup.

Audition requires an explicitly configured monitor destination. It does not
inherit the Show destination or silently fall back to the system default when
its destination is missing. Show may retain its explicitly selected default
output behavior for compatibility. A single physical interface may provide
distinct program and monitor destinations; two physical devices are not a
requirement. The implementation must use device identities supported by the
audio backend rather than unstable list positions. Changing one destination
does not restart or reroute the other player.

ADR 0069 supplies the accepted output-selection contract, with PulseAudio and
JACK first and native PipeWire later. It does not introduce a mixing console
or operating-system patchbay. App-level separation cannot prove that an external loopback or encoder
excludes the monitor signal. Operator verification must listen to and inspect
the actual program capture path. A confirmation dialog is not evidence of
audio isolation.

Reuse the `PlaybackDriver` abstraction and mpv adapter with separate driver
instances. Each instance owns its child, IPC connection, response state and
socket identity. Socket names include playback purpose and an instance nonce
in addition to the application process identity. Commands and observations are
serialized within each owner; neither player waits on the other player's IPC
lock. Construction stays lazy: opening the app, loading a cue or viewing output
settings must not start audio. Any device-discovery process must finish without
loading media and clean up its own resources.

Missing or invalid audition configuration disables audition only. A Show-player
failure leaves audition usable when its own resources work. A shared missing
binary or audio service may affect both, but each report names the affected
purpose. A producer or publisher failure does not disable either audio path.
Never substitute Null to disguise a configured mpv failure. Null remains an
explicit simulation option, and its status never claims audible playback.

### Publication And Authoritative State

`PlaybackSession` remains the authoritative local Show now-playing state under
ADR 0014. The cue stores intended order; the Show owner reconciles actual driver
observations into the session. Audition observations use a separate read model
and never produce `NowPlayingUpdate`.

Only the Show owner receives the local drop-file publication capability.
Cue loading and all audition operations, including failure and teardown, must
leave existing Show metadata untouched. Show pause/stop/EOF retain the existing
producer policy. An audio failure must not leave an advancing, apparently healthy
local now-playing report. A publisher or metadata-write failure must not be
reported as failed audio or trigger a different playback source.

External sources such as Mixxx retain their own audio and metadata ownership.
Neither loading a local cue nor auditioning selects the local source, changes
an event, starts/stops a publisher or encoder, or claims an external broadcast
is healthy. This decision adds no relay protocol or recording engine.

### Commands, Keys And Layer Ownership

Use distinct application command families for cue preparation, Show transport
and audition. Shared view models project each family's availability and labels.
Screens compose existing chrome, use named tokens and dispatch those commands;
they do not infer playback purpose or choose an output device during rendering.

Existing Play/Pause and Next/Previous menu and keyboard actions become explicitly
named Show actions and require the Show section. Preserve ADR 0067's platform
modifiers and key combinations. From Music or Settings those actions cannot
start or manipulate Show, and do not fall back to audition. Music's focused
audition control has its own accessible action. Search submission, text editing
and bare navigation keys retain their existing scope.

The application/runtime layer owns both players, their command admission and
their independent watches. A blocked audition command cannot stall the Show
watch. Both participate in app shutdown and ADR 0066's session drain contract;
late completions from retired generations cannot restore audio, change the
cue or clear another generation's metadata. Stopping app-owned players does
not stop the independent broadcast chain.

### Persistence, Restart And CLI Compatibility

Persist the Show cue and selection. On reopen, retain the stored track identity,
position and observation time for inspection, and reconcile the session to
stopped before projecting current playback. Audio stays stopped until an
explicit Show action starts playback. This adds no play-history subsystem.
Never reconstruct a cue by guessing that
an old default playback session represents a planned show. Preserve existing
playlists, session records, bindings and migration history during migration.
Audition state is transient and is not resumed after restart or session recovery.

The one-shot CLI remains a session simulator under ADRs 0020/0021; it is not
an audition path or RPC to the desktop. Its existing JSON contracts remain.
Add exclusive writer admission between a live desktop Show owner and legacy
session-mutating CLI commands against the same database/session. A conflicting
CLI mutation fails explicitly; it cannot become audio by being discovered on
a later poll. Read-only `now-playing --json` remains available. Isolated fixture
databases continue to support simulation without an audio player. The storage
packet must prove lease release on normal exit and process failure, rather than
using a persistent boolean as evidence that a desktop owner is alive.

## Relationship To Existing Decisions

If accepted, this ADR changes these bounded contracts; it does not supersede
the referenced ADRs in full:

- **0014:** authoritative Show now-playing remains; cue preparation and audition
  are expressly outside that state.
- **0020:** session-only CLI simulation remains, with writer exclusion while
  the desktop owns the same session. GUI advancement follows the loaded cue.
- **0021:** replace the single default-session driver lifecycle with independent
  Show/audition instances; replace automatic audio restoration on restart with
  explicit Show start. Keep the driver boundary and IPC hygiene requirements.
- **0060:** implements its Show/curation separation. The obsolete reserved
  audition ADR number is replaced by this record.
- **0063:** retain Show's Cuelist container and transport placement; extend their
  view-model contract to represent a cue before playback. This does not accept
  the separately scheduled narrow-layout work.
- **0066:** scope failures per playback purpose and include both owners in
  lifecycle accounting. Its startup/preservation guarantees remain.
- **0067:** retain key delivery and modifiers; narrow transport availability
  to Show through typed dispatch. Existing keyboard acceptance does not prove
  the new command routing.

The curator design brief is advisory. Its earlier suggestion to warn before
audition during a broadcast does not supply the separate path required here.
Direct remote audition remains deferred under ADR 0021.

## Invariants

1. Cue loading performs no driver load, Show-session activation or publication.
2. Source-playlist edits cannot silently change a loaded cue's order or cursor.
3. Only explicit Show commands can start and control desktop program playback.
4. Audition cannot mutate Show state or publish, clear or replace Show metadata.
5. Each player owns a distinct driver instance, IPC identity and output route.
6. Optional failure and stale completion are contained to their owning purpose.
7. Restart and recovery restore no audio automatically.
8. Intended selection, observed playback and publication results remain distinct.

## Alternatives Considered

- **Rename Music Play to Audition but retain the owner.** Reject: it still
  changes Show state and can publish metadata.
- **Load the playlist as a paused session.** Reject: preparing a show would
  acquire playback/now-playing semantics before the operator starts it.
- **Keep the cue as a live playlist reference.** Reject: curation edits would
  alter the running show's order and invalidate its cursor.
- **Use one player and restore Show after each audition.** Reject: audition
  would interrupt program audio and entangle failure recovery.
- **Run two players into the same implicit default output.** Reject: process
  isolation alone does not supply independently chosen program/monitor paths.
- **Let desktop audio tools alone own routing.** Not selected: ADR 0069 accepts
  app-selected, recallable destinations. Desktop routing remains complementary
  and still needs operator verification of the actual program capture path.
- **Add a DJ mixer, remote preview and multiple-show scheduler together.** Reject:
  those features are not required to establish the two playback purposes.

## Consequences

The operator can prepare and inspect a show before playing it, and audition
without changing what listeners see. A persisted cue adds a storage lifecycle
and must participate in library-removal checks. Two players add output setup,
resource use and teardown work. Requiring a monitor destination adds initial
setup to audition; a distinct default stream would be simpler but would leave
output selection entirely to desktop routing tools.

This changes inherited GUI playlist playback and keyboard behavior. Existing
tests that equate Music Play with Show playback must be replaced at their
owning layers, not preserved by routing audition through the old owner.

## Verification

These are required proofs for implementation, not tests claimed to exist.

| ID | Mechanical proof owner | Required assertion |
|---|---|---|
| M1 | Cue storage/command tests | Load and replacement are atomic, silent and independent of a player; failed/empty load preserves the old cue; duplicate occurrences have distinct IDs. |
| M2 | Cue query/removal tests | Loaded order survives playlist edits/deletion and restart; missing files remain visible; library removal accounts for cue references. |
| M3 | Show owner tests | Start/pause/stop/select/skip obey the cue; paused skip emits no audible load; final EOF stops; unavailable next entry is reported; duplicate/stale EOF cannot advance twice. |
| M4 | Audition owner tests with recording fakes | Start/replace/seek/pause/EOF/error/stop perform zero Show writes or producer calls; independently driven Show progression continues, including its own metadata updates. |
| M5 | Driver/runtime tests | Concurrent instances have different children/sockets and independent replies; one timeout, shutdown or route failure cannot control or block the other. |
| M6 | Configuration and output-adapter tests | Old config remains readable; invalid output affects only its purpose; no missing-route fallback or failure-to-Null substitution; explicit simulation is labeled. |
| M7 | View-model/dispatch tests | Cue preparation, Show transport and audition have distinct intents and accessible labels; Music/Settings shortcuts cannot mutate Show; state updates reach mounted views. |
| M8 | Startup/lifecycle tests | Cue survives restart with audio stopped; audition does not resume; both owners drain and release resources; stale work cannot republish or relaunch. |
| M9 | CLI/owner admission tests | Simulation works alone; same-session CLI mutations fail during desktop ownership; read-only JSON works; process failure releases ownership. |
| M10 | Migration/preservation tests | Existing playlists, tracks, bindings, sessions and migration records survive; legacy session data does not implicitly create an active Show. |

Situational guards in `tests/architecture_tests.rs` must cite ADR 0068 and
protect the live command routes and the absence of Show publication capability
from audition. Behavioral tests prove effects; source guards alone cannot prove
audio isolation. Replace procedure prose with actual test/guard references when
implementation lands, following ADR 0061.

### Operator Visual And Audio Check

Open; not runnable from this draft. No application behavior changed. The
implementation packet must supply fixture setup/run/inspect/cleanup commands,
an output-selection procedure, and a controllable program capture path before
requesting this check. It needs a Linux desktop, mpv, local fixture tracks with
distinguishable audio, and separately routable program/monitor destinations.

1. Load a playlist in Music. Inspect its cue in Show before starting playback.
   Audio or new Show metadata during loading is wrong.
2. Start the cue from Show. Inspect cue order, current-entry identity, transport,
   progress and metadata while using pause, stop and track advancement.
3. While Show plays, audition a different fixture track in Music. Listen at
   both destinations and inspect program capture and Show metadata. Only the
   monitor path may contain audition; Show must keep its track and position.
4. Replace, pause and stop audition; leave Music; disconnect its output and
   exercise a controlled player failure. Verify Show continues. Repeat the
   independent-failure check with Show unavailable and audition available.
5. Edit the source playlist, restart the app and exercise section shortcuts.
   The stored cue must remain intact, startup must remain silent, and Music
   controls must never start Show. Inspect at normal and narrow widths.
6. Run preservation inspection after closing the app. Remove fixture-owned
   audio routes, capture files, child processes and sockets through the
   packet's cleanup commands; restore any desktop routing changed for the check.

Keep the gate in the implementation packet, delivery index and pending-human
index until a person supplies these observations and preservation/cleanup results.

## Follow-Up Work

Before implementation, accept or revise this proposal and allocate bounded
packets in the [delivery order](../plans/broadcast-chain-delivery-order.md).
Packet boundaries should cover cue persistence and Show commands, independent
audition/output ownership, then UI integration and operator verification. Each
packet must be useful and reviewable on its own; no parallel legacy GUI route
may ship under an Audition label while still controlling Show.

Account for both owners when ADR 0066 task 005 introduces managed session drain;
this ADR does not require that unstarted packet as a prerequisite for drafting
or storage work. Scheduling must explicitly reconcile task 004's paused playback
criteria instead of silently moving or closing them. Report/preservation checks
may continue independently. The observed mpv IPC error needs its own diagnosis
and regression proof before audible playback acceptance; this draft claims no fix.

## References

- [ADR 0059: Broadcast control surface](0059-broadcast-control-surface.md)
- [ADR 0064: Local file addressing](0064-local-file-addressing.md)
- [ADR 0067: Platform shortcuts](0067-platform-shortcut-modifiers.md)
- [Curator workflow design brief](../plans/curator-workflow-ui-design-brief.md#auditioning)
- [Pending human checks](../pending-human-checks.md#6-optional-tool-isolation--adr-0066-task-004)
