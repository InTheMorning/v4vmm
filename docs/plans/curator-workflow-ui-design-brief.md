# Curator Workflow UI Design Brief

## Status

Active product brief - 2026-09-07.

**Advisory, not binding.** ADR 0061 places every binding rule in an ADR. This
brief records product intent and reasoning. The decisions it argued for are
binding through ADR 0060 and the ADRs that follow it. Where this brief and an
ADR disagree, the ADR wins.

## Purpose

Record the product-owner intent for v4vmm's curator interface before more UI
packets are implemented. Future UI work must translate this brief into view
models, shared primitives, composites, tokens, screen wiring, and regression
guards. The product owner is not expected to specify layout, padding,
typography, component choice, breakpoint behavior, Apple HIG terminology, or
navigation mechanics.

This brief sits above bounded UI task packets. It does not replace ADRs,
schema plans, or implementation tasks. When a future change affects persistence,
service boundaries, public contracts, playback behavior, or workflow
architecture, create or amend the relevant ADR and task packets before changing
runtime code.

## Non-Goals

- No immediate runtime implementation.
- No new schema decision.
- No change to ADR 0059's current broadcast-control-surface contract.
- No requirement to reopen completed search, sidebar, frame, or shell
  restructuring work.
- No requirement for the product owner to choose concrete layout structures.

## Product Positioning

v4vmm is a Linux-first music curation and show-production app for Value4Value
music. It should feel like a calm creative/professional tool for curators, not
like a database console.

The product loop is:

```text
Discover -> Collect -> Maintain -> Prepare -> Broadcast or Record -> Package
```

The main product language should center on `Discover`, `Collection`, and
`Show`. Search is a finding and filtering tool inside those workflows, not the
whole product model.

## Primary User

The primary user is a V4V music curator with a music podcast. The current
target exemplar is Oystein Berge of the Mutton, Mead and Music podcast: a
Linux user who already works with tools such as Mixxx and butt.

The interface can assume that many curators understand music-production terms
like tracks, playlists, files, feeds, recording, and streaming. It should not
assume that new curators understand internal app architecture, storage states,
source-fact contracts, ID3 implementation details, or relay mechanics.

## Primary Task

The primary task is to discover music, take it into a trusted local collection,
keep that collection ready, and use it to prepare and run a show.

## Secondary Tasks

- Inspect metadata and provenance when something needs explanation.
- Fix collection problems before show time.
- Audition remote or downloaded tracks.
- Use either built-in `mpv` playback or Mixxx.
- Monitor live broadcast health.
- Package a recorded episode after the show.
- In future work, browse public playlist RSS feeds from other curators.
- In future work, tag or crate songs for later reuse.

## Core Workflow

1. The curator sees new releases and, later, online playlists from other
   curators.
2. The curator takes desirable tracks into the collection.
3. v4vmm downloads the audio, combines the actual file with artwork and
   metadata, and calls attention to ID3, file, download, or metadata problems.
4. The curator inspects or compares provenance only when needed.
5. The curator chooses a production mode.
6. v4vmm runs a mode-aware preflight check.
7. If the show is live, the curator updates the RSS feed for live broadcast and
   podpings only after the feed is valid.
8. During the show, v4vmm visually confirms service health and listener-facing
   now-playing metadata.
9. During non-song blocks, now-playing shows the episode's default metadata,
   value block, and artwork.
10. After the show stops, v4vmm shows the artifacts needed to package the
    recorded episode, including the butt recording and reconstructed value time
    split data.

## Production Modes

All production modes are first-class peers:

- Live show with built-in v4vmm/`mpv` playlists.
- Live show with Mixxx.
- Private or pre-recorded capture for later packaging.

The interface must not show all mode-specific controls as if they all matter at
once. Mode choice must shape readiness checks, primary actions, warnings,
monitoring, and post-show affordances.

### Built-In `mpv` Live Mode

- Built-in playlist readiness matters.
- Track order matters.
- Local playback controls are routine.
- Mixxx status is irrelevant unless the user also configures it as a source.

### Mixxx Live Mode

- Mixxx, `mixxx-now-playing`, `musicindex-live-publisher`, and v4vmm handoff
  health matter.
- Built-in playlist readiness is irrelevant.
- The curator likely stays in Mixxx and glances at v4vmm for confirmation.

### Private Or Pre-Recorded Capture Mode

- Public live event APIs, RSS live validity, and podping are not pre-show
  blockers.
- The app still needs song start/stop capture and metadata logging so the
  episode can be packaged later.
- This requires future `musicindex-live-publisher` support for a private,
  non-live mode that records song timing and metadata without public publishing.

## Collection Model

The current core collection interaction is correct and should be preserved:

```text
I browse. I see a song I like. I take it.
It is now in my collection.
I do not want it anymore. I dump it.
```

Taking a track into the collection should download it and call attention to
file, ID3, and metadata problems. Inspecting/comparing should reveal every
needed source fact without forcing all source detail into the primary list.

The collection should feel like the curator's trusted working inventory. A row,
tile, or detail surface should make these facts clear at a glance:

- the actual audio file exists;
- title, artist, and artwork are suitable for listener presentation;
- upstream source data changed or remains current;
- a problem exists before show preparation begins;
- the track has never been played, or was played recently;
- the track is collected, cached-only, downloaded, missing, stale, or
  problematic.

Source facts and provenance remain important, but they belong in an inspection
layer unless a conflict affects readiness.

## Search And Discovery

The app should not force curators to reason about separate worlds called
catalog search, library search, downloaded files, and queues. The UI should
make scope visible and answer curator questions:

- Can I use this track?
- Do I already have it?
- Is it downloaded?
- Is it ready for broadcast or recording?
- Is it in the current show?
- What will listeners see?

A scope control such as `Only show my collection` can be useful, but the design
principle is broader: search should preserve the user's intent while making
local/remote/collection state obvious.

## Auditioning

Auditioning should be a lightweight utility, not a full DJ deck.

The first useful version is a simple playback affordance near a track, plus a
progress indicator. It should support checking remote or downloaded audio when
possible. It does not need waveform display, BPM, key, moodbars, crates,
headphone cueing, or routing controls.

Audition playback and show playback are separate concepts:

- Audition playback is local preview for choosing or checking music.
- Show playback and now-playing state affect listener-facing metadata and may
  affect live publishing.

If the app is currently broadcasting, audition should warn strongly and require
an explicit override such as `I know what I am doing`. For now, the user owns
audio routing.

## Preflight Readiness

Preflight readiness is mode-aware. A condition can be a hard blocker in one
mode and irrelevant in another.

Hard blockers include the following when applicable:

- RSS feed invalid.
- Event ID missing or not configured.
- butt not running when needed.
- Mixxx not running when Mixxx mode is selected.
- Relay unavailable.
- No playlist loaded when built-in `mpv` playlist mode is selected.
- Missing artwork when it affects listener presentation.
- Bad or incomplete song metadata when it affects listener presentation.
- Library not fully updated.
- `musicindex-live-publisher` not authenticated or not healthy.
- Recording not started when recording is part of the selected workflow.

The primary start/podping action must be unavailable when relevant hard
blockers exist, and the UI must name the blocker in curator-facing language.

## Live Console

During a live show, v4vmm should function as a command center and confidence
display. It may run on the primary monitor or on a second monitor. A Mixxx user
may interact with Mixxx most of the time and glance at v4vmm for confirmation.

The live surface should keep these visible:

- listener-facing now-playing preview with artwork, show or episode title,
  artist, song title, and split summary;
- essential service health: butt, Mixxx or built-in source, relay, RSS validity,
  and LiveValue freshness;
- recording status, recording timer, and listener count when available;
- current and recent song history;
- upcoming queue when built-in `mpv` mode is selected;
- a calm live heading such as `MusicShow episode 12 live`.

The raw event ID is mainly pre-show and post-show utility for RSS work. It does
not need permanent live-screen dominance.

When no song is playing, now-playing should intentionally show episode default
metadata, value block, and artwork. This is a real state, not an error state.

## Recovery And Diagnostics

When something breaks, the user should intuitively find a recovery surface that
does not require hunting through general settings. The recovery UI should stay
compact but obvious.

The recovery surface should:

- appear or become prominent only when a problem is detected;
- explain the failing state in curator-facing language;
- offer quick buttons for low-risk fixes;
- provide `More info` paths for detailed status, logs, and deeper inspection;
- keep now-playing confidence visible where possible.

## Post-Show Packaging

After the episode stops, v4vmm should show the information and artifacts needed
to package the recorded episode:

- butt's recorded MP3;
- captured song starts and stops;
- reconstructed value time split blocks;
- RSS episode material for a properly formatted music-show episode.

This is future architecture work after the broadcast control surface. The
timeline owner is expected to be `musicindex-live-publisher`, because the chain
must operate when v4vmm is closed.

## Information Hierarchy

### Persistent Information

- Collection readiness.
- Local file availability.
- Metadata and artwork confidence.
- Update/freshness status.
- Problem status.
- Recent usage or never-played status.
- Live service health when a show is active.
- Listener-facing now-playing preview when a show is active.
- Recording timer, recording status, and listener count when available.

### Contextual Information

- RSS, MusicIndex, ID3, and MusicBrainz source facts.
- Detailed relay, publisher, stream, and RSS diagnostics.
- Event ID copy/paste details.
- Cache location and purge policy.
- Show history detail.
- Tag/crate management detail.

### Background Or Deferred Information

- Playlist export, pending more research.
- Advanced audio routing.
- Deep logs unless a problem exists or the user asks for details.

## Action Hierarchy

### Primary Actions

- Take track into collection.
- Dump track from collection.
- Audition track.
- Start preflight.
- Podping/start live show when preflight is valid.
- Start, stop, resume, or pause songs in built-in player mode.
- Open detected problems.

### Secondary Actions

- Compare metadata/provenance.
- Retry failed downloads.
- Refresh/update from source.
- View show/song usage history.
- Add or remove tags/crates in future work.
- Inspect detailed logs or service state.

### Infrequent Or Configuration Actions

- Configure event ID.
- Configure storage/cache policy.
- Force redownload.
- Force purge.
- Clear cached files.
- Configure deeper publisher, butt, or Mixxx details.
- Export playlists after research decides the product contract.

### Risky Actions

- Podping/start live broadcast.
- Stop live broadcast.
- Audition while broadcasting.
- Force purge or delete local files.
- Clear cache.
- Publish public live metadata.

Risky actions must be visually separated from routine actions. Destructive file
operations require stronger caution than removing a track from the collection.

## Screen And Input Context

- Normal curation uses a large or maximized window.
- v4vmm may run on a primary monitor or a second monitor.
- During a live Mixxx workflow, the curator may stay in Mixxx and glance at
  v4vmm with little interaction.
- Interaction is mouse/trackpad-first.
- Start with only common and intuitive keyboard shortcuts, such as search
  focus, until real workflow demand proves more are needed.
- High contrast and size toggles remain required.
- The UI must not rely on color alone.
- Tiny click targets should be avoided.
- Layout density should eventually be adjustable.

## Interface Implications

- Use workflow-oriented top-level structure: `Discover`, `Collection`, and
  `Show`.
- Keep search as a scoped find/filter affordance inside workflows.
- Keep collection health visible without making source details permanent
  clutter.
- Use progressive disclosure for provenance, logs, cache policy, and advanced
  diagnostics.
- Use mode-aware readiness projections instead of screen-local conditional
  checks.
- Use consistent presentation for collected, downloaded, cached-only, missing,
  stale, conflicted, and recently played states.
- Keep audition playback visually distinct from show playback.
- Make the live surface highly scannable from a distance.
- Make the recovery affordance contextual and quick to reach.
- Treat Apple HIG as structural guidance: predictable hierarchy, clear state,
  progressive disclosure, comfortable density, accessibility, and cautious
  destructive actions.

## Ownership Requirements For Future UI Work

Future UI packets that use this brief must satisfy the existing UI ownership
gate:

- Curator-facing labels, readiness states, command availability, and action
  intent belong in GPUI-free view models or `src/views.rs`.
- Button/action vocabulary, disabled states, destructive roles, and
  accessibility labels belong in typed action-state contracts before rendering.
- Reusable rows, buttons, popovers, panels, recovery surfaces, and interaction
  geometry belong in `src/ui/primitives` or `src/ui/composites`.
- Tokens own colors, spacing, typography, status roles, and material roles.
- Screens compose views, resolve images, wire callbacks, manage focus and
  selection, and dispatch commands.
- User-visible layout or workflow changes need tests, architecture guards, or
  visual smoke evidence before they are called complete.

## Documentation Impact And Required Follow-Ups

Immediate documentation changes for this brief:

- Add this file under `docs/plans/` because it is a product/UI feature design
  and governance source, not an ADR.
- Add this file to `docs/README.md` so agents find it from the docs index.
- Cross-reference this file from `docs/plans/hig-product-polish-backlog.md` so
  future HIG polish work does not become disconnected cosmetic work.

Required follow-up documentation before implementation:

- Task packets that touch curator-facing UI must list this file under
  `Files to inspect` and include design-brief compatibility in acceptance
  criteria.
- Production-mode readiness that changes runtime behavior must amend or extend
  ADR 0059, or create a new ADR if it exceeds the broadcast-control-surface
  decision.
- Collection/cache separation needs a future ADR because it affects storage,
  file layout, deletion behavior, and cache policy.
- Show/song usage statistics need a future ADR or schema plan before database
  changes.
- Tags/crates need a future ADR or schema plan before persistence or workflow
  implementation.
- Built-in audition playback should route through the existing playback
  architecture docs and needs an ADR only if it changes playback-driver
  ownership, now-playing semantics, or live-safety behavior.
- Private/non-live capture mode needs documentation in
  `musicindex-live-publisher` first, then corresponding v4vmm ADR/task updates
  after the contract exists.
- Post-show packaging remains future architecture work after ADR 0059. It
  should build on the existing broadcast recording and feed-publishing research.

## Open Questions

- What exact labels should replace internal terms for collection freshness and
  unreviewed releases?
- Which collection/cache policy should be default: immediate purge on dump,
  grace-period cache, size-limited cache, age-limited cache, or a combination?
- What is the minimum useful show-history statistic set: last played, play
  count, show list, or time-window warnings?
- Which parts of private capture belong in `musicindex-live-publisher` versus
  v4vmm?
- Which visual smoke fixtures should become canonical for `Discover`,
  `Collection`, and `Show`?

## References

- `AGENTS.md`
- `docs/README.md`
- `docs/architecture/ui-backend-boundary.md`
- `docs/architecture/ui-regression-ratchet.md`
- `docs/plans/hig-product-polish-backlog.md`
- `docs/adr/0059-broadcast-control-surface.md`
- `docs/plans/adr-0059-broadcast-control-surface-phase-plan.md`
- `docs/research/broadcast-recording-and-feed-publishing.md`
