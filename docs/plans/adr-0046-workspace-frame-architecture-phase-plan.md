# ADR 0046 Phase Plan: Workspace Frame Architecture

Status: Draft - 2026-05-14.

Companion to `docs/adr/0046-workspace-frame-architecture.md`. Each phase
is additive; the prior visible UI keeps rendering until the
workspace-frame path is validated.

## Phase 1 - ADR and Frame Contract

Goal: ratify ADR 0046, this phase plan, and the first task packet.

Deliverables:

- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/plans/adr-0046-workspace-frame-architecture-phase-plan.md`
- First implementation task packet (Task 001)

Acceptance:

- ADR records invariants, frame kinds, resolved open questions, and
  consequences.
- Phase plan enumerates tasks per phase.

Risks: none until implementation begins.

Verification: docs land and downstream tasks reference the ADR
correctly.

## Phase 2 - Workspace State Without Visual Overhaul

Goal: introduce GPUI-free workspace state and route playlist-origin
navigation through frame history. Visible UI mostly unchanged.

Tasks:

- `adr-0046-task-001-workspace-model-types`
- `adr-0046-task-002-frame-history-vm`
- `adr-0046-task-003-retire-inspector-back-button`
- `adr-0046-task-004-phase-2-architecture-guards`

Deliverables:

- `src/view_models/workspace.rs` with workspace types and unit tests
- Frame history wired under the existing UI
- Inspector "Back to Playlist" removed; `InspectorFrame.origin` /
  `InspectorOrigin` retired from playlist-return navigation
- Architecture guards for GPUI-free frame state and no inspector-owned
  back controls

Acceptance:

- Track inspector content owns no cross-frame navigation.
- Frame navigation state represents returning to the originating
  playlist or other origin.
- Library/Search workflows render as before.

Risks: subtle behavior gaps if inspector-local refresh path is removed
together with the back button. Mitigation: keep refresh-preserving path
in `LibraryApp`; only the visible inspector button goes away.

Verification: `cargo test workspace`, `cargo test --test
architecture_tests`, no new visible UI.

## Phase 3 - Frame Shell and Flexible Split View

Goal: render the main app body as a workspace layout using a shared
frame shell composite.

Tasks:

- `adr-0046-task-005-frame-shell-display-vm`
- `adr-0046-task-006-frame-shell-composite`
- `adr-0046-task-006a-screen-mount-boundaries`
- `adr-0046-task-007-workspace-layout-render`
- `adr-0046-task-008-narrow-width-collapse-and-visual`

Deliverables:

- `FrameShellDisplay` VM with back/forward/close/menu display fields
- `src/ui/composites/frame_shell.rs` rendering the display contract
- A bounded screen-mount decision for existing Library/Search split panes
- `src/ui/shells/workspace.rs` rendering `WorkspaceLayout`
- Narrow-width collapse rules + breakpoints
- Visual proof for default and narrow layouts (light + dark)

Acceptance:

- Frame chrome owns title, history, close, and menu controls.
- Library/Search/Settings content renders inside frames.
- Narrow window collapses optional frames before primary
  nav/search.

Risks: regressing existing screen layouts. Mitigation: keep prior tab
rendering reachable behind a feature flag or fallback until the
workspace render is verified.

Verification: `cargo test --test architecture_tests`, manual L/D visual
review captured in the review checklist.

## Phase 4 - QueueNowPlaying Frame

Goal: move queue/liveValue/transport into a dedicated frame; reduce
toolbar Now Playing card to compact status and minimal transport.

Tasks:

- `adr-0046-task-009-queue-now-playing-page-vm`
- `adr-0046-task-010-queue-now-playing-frame-shell`
- `adr-0046-task-011-phase-4-guards-and-visual`

Deliverables:

- `QueueNowPlayingPageVm` display contract
- `src/ui/shells/queue_now_playing.rs` rendering the frame
- Toolbar Now Playing reduced to status + minimal transport
- Architecture guards: liveValue and queue controls do not render
  through the toolbar
- Visual proof for queue-frame expanded and collapsed (L + D)

Acceptance:

- Queue list, transport, liveValue picker, and volume live in the
  QueueNowPlaying frame.
- Toolbar Now Playing remains visible as compact status.
- Collapsing the queue frame preserves global playback status.

Risks: playback semantics regressions. Mitigation: do not modify
playback engine or mpv driver in this phase.

Verification: `cargo test --test architecture_tests`, L/D visual review.

## Phase 5 - Multi-Frame Expansion

Goal: persist frame layout state and prepare add/remove operations without
exposing fake additional frames before content/detail frame owners exist.

Tasks:

- `adr-0046-task-012-frame-add-remove-and-persistence`
- `adr-0046-task-013-multi-frame-commands-ux`

Deliverables:

- Workspace VM add/remove operations with focus invariants
- Layout persistence in `config.toml`
- Frame-chrome context menu and keybinding remain deferred until a real
  `ContentList`/`Detail` page VM can own non-duplicated content
- Architecture guard preventing transitional whole-screen frame duplication

Acceptance:

- Users are not offered a second frame that only duplicates the default
  Library/Search/Settings mount.
- Frame removal leaves a valid focused frame.
- Layout persistence does not require schema migration.

Risks: config-shape churn. Mitigation: workspace layout serializes
additively; missing fields fall back to default.

Verification: unit tests for add/remove/focus, L/D visual review of
multi-frame layout.

## Phase 6 - Detach/Dock Readiness

Status: Implemented model-only - 2026-05-15.

Goal: add detach/dock metadata and commands to the workspace model
without shipping separate OS windows.

Tasks:

- `adr-0046-task-014-detach-dock-metadata`

Deliverables:

- Per-frame detach/dock eligibility metadata
- `request_detach` / `request_dock` commands returning a deferred-error
  variant
- Architecture guards that no `src/ui/*` references detach commands
- Unit tests covering eligibility per frame kind

Acceptance:

- Frame model expresses detach/dock intent.
- No UI promises detach/dock before implementation exists.

Risks: language overpromise. Mitigation: deferred-error variant + UI
exclusion guard.

Verification: `cargo test workspace`, `cargo test --test
architecture_tests`.

## Cross-Phase Verification

Every phase runs:

- `cargo fmt -- --check`
- `cargo check`
- `cargo test`
- `cargo clippy -- -D warnings`

Architecture-test guards land alongside the structural change they
protect. Visual proof and phase readiness are captured in
`docs/reviews/adr-0046-review-checklist.md`, created before Phase 2
implementation begins and updated through each phase.
