# Workspace Frame Architecture Plan

## Status

Draft - 2026-05-14.

This is a pre-ADR plan. It records the intended product direction before
creating the formal ADR and bounded task packets.

## Goal

Move the app from fixed Library/Search/Settings pages with tacked-on detail
navigation toward a flexible **Pro Workspace** model.

The default workspace should feel like a disciplined desktop music workbench:
stable source navigation, global search, inspectable content, and persistent
playback/queue status. It should stay visually restrained and HIG-aligned, but
it should be flexible enough to support multiple content frames and eventual
detachable/dockable queue or source panes.

## Non-Goals

- No immediate Foobar-style full visual customization.
- No detachable operating-system windows in the first implementation slice.
- No schema migration unless later queue/liveValue work proves it necessary.
- No bottom playback bar as the primary queue/liveValue home.
- No new local inspector "Back" buttons as a long-term navigation pattern.
- No redesign of playback engine semantics in this plan.

## Current State

- `TopApp` owns three primary tabs: Library, Search, and Settings.
- The app toolbar owns global search and a compact Now Playing frame.
- Library detail panes can show playlist, feed, album, and track inspectors.
- Opening a playlist track currently relies on detail-origin state and a
  track-inspector return control to get back to the playlist.
- ADR 0038 established shared shell/page-VM direction, and ADR 0043 added the
  top-toolbar/global-search shape.
- Richer playback work is now blocked on deciding where queue, liveValue
  output, playback status, and multi-source content viewing should live.

## Target State

Adopt a flexible split-view workspace made of first-class frames:

- `SourceList`: library tree, playlists, saved searches, and settings entry.
- `ContentList`: selected library/search/playlist results.
- `Detail`: track/feed/artist/album metadata and actions.
- `QueueNowPlaying`: active playlist/queue, playback status, liveValue output,
  and playback/output controls.

The main window starts with a sensible default layout:

- leading source/navigation frame
- center content/detail workspace
- trailing `QueueNowPlaying` inspector frame

Frames can be added and removed within the main window. Some frame metadata
can mark future detach/dock eligibility, but v1 keeps everything in-window.

## Product Decisions

- Primary model: **Pro Workspace**.
- Frame strategy: **flexible split view**, not fixed tabs only.
- Queue home: **trailing inspector frame** by default.
- Detach/dock: design-compatible but deferred.
- Visual tone: compact, utilitarian, Apple-HIG disciplined; not decorative or
  customization-first.
- Top toolbar: global command/search/status only. Detailed queue/liveValue
  controls move out of toolbar chrome and into the queue frame.
- Navigation: frame chrome owns Back/Forward/history. Entity inspectors should
  not gain more contextual escape buttons.

## Proposed Architecture

Add GPUI-free workspace model types before broad UI movement:

- `WorkspaceFrameId`
- `WorkspaceFrameKind`
- `WorkspaceFrameState`
- `WorkspaceLayout`
- `FrameNavigationState`

Add a shared frame shell/composite that owns:

- frame title and optional subtitle/status
- Back/Forward controls when history exists
- close/remove frame control where allowed
- frame-local action menu
- content slot supplied by existing page shells

Existing page VMs and shell helpers should continue rendering entity content.
They move inside workspace frames instead of becoming workspace/navigation
owners.

## Phases

### Phase 1 - ADR and Frame Contract

Create an ADR for workspace frame architecture. Define frame kinds, frame
navigation rules, and invariants that prevent inspectors from owning
cross-frame navigation.

Deliverables:

- `docs/adr/0046-workspace-frame-architecture.md` or next available ADR number
- `docs/plans/adr-0046-workspace-frame-architecture-phase-plan.md`
- first implementation task packet and review checklist

### Phase 2 - Workspace State Without Visual Overhaul

Introduce GPUI-free workspace layout and frame navigation state. Keep the
existing visible UI mostly intact, but route "opened from playlist" history
through frame navigation state instead of track-inspector controls.

Acceptance:

- track inspector content does not own "Back to Playlist"
- frame/navigation state can represent returning to the originating playlist
- existing Library/Search workflows still render as before

### Phase 3 - Frame Shell and Flexible Split View

Add the shared frame shell and render the main app body as a workspace layout.
Start with the current default arrangement so the visual change is controlled.

Acceptance:

- frame chrome owns title, history, and close/menu controls
- Library/Search/Settings content render inside frames or frame-backed regions
- narrow window behavior collapses optional frames before hiding primary
  navigation/search commands

### Phase 4 - QueueNowPlaying Frame

Move detailed playback/queue/liveValue controls into the trailing
`QueueNowPlaying` frame. Keep toolbar Now Playing compact as status and
transport only.

Acceptance:

- queue list and playback status have their own persistent space
- liveValue output controls live with queue/playback, not in track inspectors
  or global toolbar overflow
- queue frame can collapse without losing global playback status

### Phase 5 - Multi-Frame Expansion

Allow opening additional content/detail frames inside the main workspace. Add
frame add/remove controls and persistence for the default workspace layout.

Acceptance:

- users can view more than one source/detail context without losing current
  queue state
- frame removal leaves a valid focused frame
- layout persistence does not require schema migration unless config storage
  proves insufficient

### Phase 6 - Detach/Dock Readiness

Add metadata and commands that describe which frames may detach later. Do not
implement separate windows until a follow-up ADR decides the OS-window
behavior.

Acceptance:

- frame model can express detachable/dockable intent
- no UI promises detachable windows before implementation exists

## Affected Modules

- `src/app.rs` and `src/app/tab_bar.rs` for app-shell routing and toolbar
  boundaries.
- `src/ui/shells/*` and `src/ui/composites/*` for frame shell and content
  placement.
- `src/view_models/*` for GPUI-free frame display contracts.
- `src/library/app_impl.rs` and `src/search/app_impl.rs` for opening content
  through workspace navigation rather than inspector-local return controls.
- `tests/architecture_tests.rs` for regression guards.

## Risks

- A flexible workspace can become scattered if frame rules are too loose.
- Retrofitting frame history may accidentally duplicate existing inspector
  origin state.
- Queue/liveValue can crowd the UI if it stays coupled to toolbar controls.
- Detach/dock language can overpromise before actual multi-window support
  exists.
- Large visual changes can invalidate previous ADR 0038/0043 architectural
  cleanup if frame shells do not reuse existing page VMs and composites.

## Test Strategy

- Architecture tests:
  - detail/inspector renderers must not contain playlist-specific Back buttons
  - frame chrome owns Back/Forward controls
  - top toolbar remains global command/search/status only
  - queue/liveValue controls render through the `QueueNowPlaying` frame path
  - frame display contracts stay GPUI-free
- Unit tests:
  - frame history push/pop behavior
  - removing a frame preserves a valid layout and focus target
  - opening a playlist track records origin in frame navigation state
- Visual smoke:
  - dark and light default workspace
  - narrow-width collapse behavior
  - SourceList + ContentList + Detail + QueueNowPlaying visible together
  - queue frame collapsed and restored

## Rollback Strategy

Keep the first implementation slices additive:

- Introduce workspace state and frame shell behind current app rendering.
- Do not delete existing tab/detail rendering until the frame-backed path is
  verified.
- Keep toolbar global search and compact Now Playing stable while the body
  layout changes.
- If the frame shell regresses navigation or layout, revert the render switch
  while leaving GPUI-free model tests in place for a corrected follow-up.

## Open Questions

- Should Settings remain a top-level tab or become a source-list item/frame?
- Should Search remain a top-level navigation item after source-list frames
  exist, or should it become a global command that opens a Search frame?
- What is the minimum useful queue/liveValue control set for the first
  `QueueNowPlaying` frame slice?
- Should workspace layout persistence live in `config.toml`, app state, or a
  later database-backed preference table?
- What exact command opens a second content frame: context menu, toolbar menu,
  keyboard shortcut, or all of these?

