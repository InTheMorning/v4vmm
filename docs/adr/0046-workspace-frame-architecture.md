# ADR 0046: Workspace Frame Architecture

## Status

Proposed - 2026-05-14. Supersedes
`docs/plans/workspace-frame-architecture-plan.md`, which is retained as a
historical pre-ADR artifact.

## Context

ADR 0038 established shared shell/page-VM ownership for presentation
contracts. ADR 0043 added the top-toolbar global search. Recent playlist
work (ADR 0044) exposed a structural weakness: detail/inspector surfaces
were carrying cross-frame navigation state (`InspectorFrame.origin` /
`InspectorOrigin::Playlist` plus a "Back to Playlist" inspector control)
because the app shell had no first-class frame model that could own history.

The product also wants to evolve beyond fixed Library/Search/Settings
tabs toward a Pro Workspace: stable source navigation, global search,
inspectable content, a persistent QueueNowPlaying surface, and headroom
for additional content/detail frames. The plan calls for a flexible
split view with model-level support for future detach/dock without
shipping multi-window in the first slice.

Today:

- `TopApp` owns three primary tabs (Library, Search, Settings).
- The app toolbar owns global search and a compact Now Playing card.
- Playback/queue/liveValue controls are scattered across toolbar chrome,
  track inspector, and library detail surfaces.
- Track inspectors carry origin state and inline back buttons.
- There is no shared concept of a workspace frame with title, history,
  close, and action-menu chrome.

Without a frame model, every new surface invents its own navigation
shape. With a frame model, navigation, history, collapse, and detach
eligibility become invariants enforced by mechanical rules.

## Decision

Adopt a workspace-frame architecture with the following invariants and
deliverables. Implementation proceeds in additive phases.

### Architectural Invariants

1. **Workspace model is GPUI-free.** `WorkspaceFrameId`,
   `WorkspaceFrameKind`, `WorkspaceFrameState`, `WorkspaceLayout`, and
   `FrameNavigationState` live in `src/view_models/workspace.rs`. No
   `gpui::*` imports in that module.
2. **Frame chrome owns navigation.** Back, forward, close, and history
   live in the frame shell composite. Entity inspectors do not own
   cross-frame navigation; "Back to Playlist" and equivalent
   inspector-local return controls are removed.
3. **Top toolbar stays global.** Toolbar carries global command,
   search, and compact status only. Detailed queue, liveValue output,
   and transport controls move into the QueueNowPlaying frame.
4. **Frame kinds are an enum, not strings.** `WorkspaceFrameKind`
   covers `SourceList`, `ContentList`, `Detail`, and `QueueNowPlaying`.
   Invalid kinds are unrepresentable.
5. **Frame composite is shared.** The frame shell lives in
   `src/ui/composites/frame_shell.rs` and consumes a `FrameShellDisplay`
   contract. Screens do not hand-roll frame chrome.
6. **Page VMs render inside frames.** Existing Library, Search, and
   Settings page VMs and shell helpers move inside frames without
   becoming frame owners.
7. **Layout persistence is additive.** Workspace layout serializes to
   `config.toml`; old configs still load with a default layout.
8. **Detach/dock is model-only in v1.** Detach/dock metadata exists on
   `WorkspaceFrameKind`; commands return a deferred-error variant. No
   second OS window in this ADR.

### Frame Kinds and Default Layout

- `SourceList`: library tree, playlists, saved searches, settings entry.
- `ContentList`: selected library/search/playlist results.
- `Detail`: track/feed/artist/album metadata and actions.
- `QueueNowPlaying`: queue, playback status, liveValue output, and
  playback/output controls.

Default workspace:

- Leading: `SourceList`.
- Center: `ContentList` plus `Detail`.
- Trailing: `QueueNowPlaying`.

Narrow widths collapse optional frames before primary nav/search.

### Resolved Open Questions

The pre-ADR plan recorded five open questions. ADR 0046 resolves them
as follows; rationale captured for downstream review:

1. **Settings becomes a source-list item.** Top-level Settings tab
   retires once `SourceList` ships. Settings has the same shape as
   playlists/saved searches: an entry that opens a `Detail` frame.
2. **Search becomes a global command opening a Search frame.** Toolbar
   search (per ADR 0043) stays as the always-visible entry point; the
   submit action opens a `ContentList` frame configured for search
   results.
3. **First-slice QueueNowPlaying control set.** Queue list, transport
   (play/pause, skip previous, skip next), liveValue device picker,
   volume slider. Further playback semantics deferred.
4. **Layout persistence in `config.toml`.** Workspace layout serializes
   to `config.toml`. Database-backed preferences revisited only if
   layout shape grows beyond what TOML expresses cleanly.
5. **Open-second-frame commands require real frame content owners.**
   The workspace model may carry add/remove operations before every
   frame kind is mounted, but the visible frame-chrome command and
   keybinding stay deferred while `ContentList` is still a transitional
   whole-screen Library/Search/Settings wrapper. Toolbar menu deferred.

### Apple HIG Alignment

- Frame chrome uses a standard back chevron glyph, not a "Back" text
  label (per HIG toolbars and navigation guidance).
- Close button uses a standard close symbol; placement consistent with
  macOS frame conventions.
- Toolbar primary actions (search submit) stay visible at all window
  sizes; collapse rule applies to frames, not to global commands.
- Disclosure controls in `SourceList` follow ADR 0033 click-target
  rules: chevron + label only, not the surrounding row chrome.
- Drag-and-drop into frames follows ADR 0044 conventions: insertion
  cues, invalid-destination muted feedback, same-container move
  semantics.

### Rust Discipline

- `M-CANONICAL-DOCS` on every new public type in `workspace.rs`.
- Public VM types use concrete shapes, no smart pointers.
- Builder pattern when display types reach four or more parameters.
- Fallible workspace operations return `Result`, never panic on bad
  input.
- Frame-kind state modeled as an enum with typed variants per
  Rust 2024 idioms.

## Consequences

Positive:

- Inspectors stop carrying navigation state.
- Queue, liveValue, and transport controls have a persistent home.
- Workspace layout becomes the unit of persistence; future frame work
  (additional content, secondary detail) plugs into the model without
  rewriting screens.
- Architecture tests gain a clear surface to guard: frame model
  GPUI-free, frame chrome shared, toolbar global-only.

Negative / risks:

- Six-phase rollout is large. Mitigation: each phase is additive; the
  old tab rendering coexists until the workspace path is validated.
- Frame history may overlap with existing inspector origin state.
  Mitigation: ADR 0046 retires inspector-owned origin navigation
  (`InspectorFrame.origin` / `InspectorOrigin`) in Phase 2.
- Detach/dock metadata can overpromise. Mitigation: commands return a
  deferred-error variant; no UI exposes detach in v1.
- QueueNowPlaying frame can crowd the workspace at narrow widths.
  Mitigation: explicit collapse breakpoints with HIG-compliant fallback
  access via toolbar menu or keybinding.

## Alternatives Considered

- **Stay with fixed tabs.** Rejected. Cannot host a persistent
  QueueNowPlaying surface and continues to push navigation state into
  inspectors.
- **Foobar-style full customization.** Rejected as v1. Out of scope per
  plan Non-Goals; revisit after the disciplined default ships.
- **Bottom playback bar.** Rejected as the queue/liveValue home; bottom
  bar can stay as compact transport status only.
- **OS-window detach now.** Rejected. Requires a separate ADR on window
  semantics; model-only readiness lets the visible UX ship first.

## References

- ADR 0023 — design system and view models
- ADR 0033 — HIG UI architecture governance
- ADR 0034 — scale-aware UI tokens and controls
- ADR 0037 — same-entity surface parity
- ADR 0038 — presentation contract enforcement
- ADR 0040 — async VM runtime
- ADR 0041 — windowed paged view models
- ADR 0042 — layer consolidation
- ADR 0043 — top toolbar global search
- ADR 0044 — playlist drag-handle reordering
- `docs/plans/workspace-frame-architecture-plan.md` — pre-ADR plan
- `docs/plans/adr-0046-workspace-frame-architecture-phase-plan.md`
