# ADR 0046 Review Checklist

## Reviewed Artifacts

- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/plans/adr-0046-workspace-frame-architecture-phase-plan.md`
- `docs/plans/workspace-frame-architecture-plan.md`
- `docs/tasks/adr-0046-task-001-workspace-model-types.md`
- `docs/tasks/adr-0046-task-002-frame-history-vm.md`
- `docs/tasks/adr-0046-task-003-retire-inspector-back-button.md`
- `docs/tasks/adr-0046-task-004-phase-2-architecture-guards.md`
- `docs/tasks/adr-0046-task-005-frame-shell-display-vm.md`
- `docs/tasks/adr-0046-task-006-frame-shell-composite.md`
- `docs/tasks/adr-0046-task-006a-screen-mount-boundaries.md`
- `docs/tasks/adr-0046-task-007-workspace-layout-render.md`
- `docs/tasks/adr-0046-task-008-narrow-width-collapse-and-visual.md`
- `docs/tasks/adr-0046-task-009-queue-now-playing-page-vm.md`
- `docs/tasks/adr-0046-task-010-queue-now-playing-frame-shell.md`
- `docs/tasks/adr-0046-task-011-phase-4-guards-and-visual.md`
- `docs/tasks/adr-0046-task-012-frame-add-remove-and-persistence.md`
- `docs/tasks/adr-0046-task-013-multi-frame-commands-ux.md`
- `docs/tasks/adr-0046-task-014-detach-dock-metadata.md`

## Gate Status

Status: Phase 5 Task 013 deferral confirmed; visible commands remain deferred
until real per-frame content owners exist. Phase 6 Task 014 detach/dock metadata
implemented model-only - 2026-05-15.

Readiness decision: **Close Phase 5 as a command-deferral slice, not as
user-visible multi-frame commands. Keep add/remove model support from Task 012,
but hide open/close frame UI until ADR 0047/frame-navigation work creates
non-duplicated content frames.**

Phase 6 model-only detach/dock metadata may proceed while Task 013 visible
commands remain deferred because it does not expose UI, keybindings, OS-window
primitives, or duplicate whole-screen content frames.

QueueNowPlaying implementation began after Phase 2 and Phase 3
frame-navigation/chrome work was complete and user-confirmed.

## Required Checks

- [x] ADR records context, decision, alternatives, consequences, invariants,
      and non-goals.
- [x] Phase plan exists and sequences the work into bounded phases.
- [x] Every task packet includes a lower-context prompt.
- [x] Every task packet includes escalation triggers.
- [x] Task paths refer to the real view-model module entrypoint,
      `src/view_models/mod.rs`.
- [x] Task wording reflects current `InspectorFrame.origin` /
      `InspectorOrigin` code instead of the older origin-field name.
- [x] Phase 3 includes a screen-mount boundary before workspace rendering.
- [x] Review checklist exists before Phase 2 implementation.
- [x] Task 001 workspace model types implemented and focused gates are green.
- [x] Task 002 frame history view model implemented and focused gates are
      green.
- [x] Task 003 inspector-local Back control and origin state retired.
- [x] Task 004 Phase 2 architecture guards implemented and focused gates are
      green.
- [x] Phase 2 implementation complete.
- [x] Task 005 frame shell display view model implemented and focused gates are
      green.
- [x] Task 006 frame shell composite implemented and focused gates are green.
- [x] Task 006a screen mount boundary implemented and focused gates are green.
- [x] Task 007 workspace layout render implemented and focused gates are green.
- [x] Phase 3 implementation complete.
- [x] Task 009 QueueNowPlaying page VM implemented and focused gates are green.
- [x] Task 010 QueueNowPlaying frame shell implemented and focused gates are
      green.
- [x] Task 011 Phase 4 guards implemented and automated gates are green.
- [x] Phase 4 implementation complete.
- [x] Full automated gate green.
- [x] Task 008 narrow-width collapse implementation added named workspace
      breakpoints and runtime optional-frame collapse.
- [x] Task 008 architecture guards integrated by parent task owner.
- [x] Task 008 visual proof complete.
- [x] Phase 4 manual visual proof complete.
- [x] Task 012 start gate cleared.
- [x] Task 012 add/remove operations implemented with `Result` errors.
- [x] Task 012 focus invariants and last-frame removal guard covered by unit
      tests.
- [x] Task 012 `workspace_layout` config persistence implemented with malformed
      config fallback.
- [x] Task 012 startup/save/shutdown persistence wiring recorded by architecture
      guard.
- [x] Task 012 full automated gate green.
- [x] Task 013 frame action menu ids and labels are not exposed while
      `ContentList` is a transitional whole-screen mount.
- [x] Task 013 context-menu and keybinding commands are deferred instead of
      routing to duplicate active Library/Search/Settings frames.
- [x] Task 013 keybindings for unavailable workspace-frame actions are absent.
- [x] Task 013 focus indicator uses `SemanticColor::Focus`.
- [x] Task 013 deferral architecture guard green.
- [x] Task 013 duplicate-frame regression visually confirmed absent by the
      operator on 2026-05-15.
- [x] Task 014 detach/dock eligibility metadata implemented in
      `src/view_models/workspace.rs`.
- [x] Task 014 `request_detach` / `request_dock` return deferred model errors
      for detachable frames and `NotDetachable` for anchored frames.
- [x] Task 014 model-only architecture guard confirms no `src/ui/*` or
      `src/app.rs` wiring references the detach/dock surface.

## Required Fixes

- None for the ADR 0046 Task 013 deferral or Task 014 model-only slice.

## Optional Improvements

- Add sketches or screenshots to this checklist during Phase 3 visual review.
- Revisit whether Settings remains in toolbar navigation after the SourceList
  frame ships.

## Architectural Drift Watchlist

- Do not split Library/Search internals during Task 007 unless a later task
  explicitly owns that work.
- Do not reintroduce inspector-local Back controls.
- Do not move queue/liveValue controls into toolbar overflow.
- Do not expose detach/dock UI before a follow-up windowing ADR.
- Do not let lower-context tasks redesign frame kinds.
- Do not expand Task 013 into breadcrumb, move/resize, or independent
  per-frame Library/Search/Settings routing work; keep that for ADR 0047/
  frame-navigation tasks.
- Do not expose "Open New Frame" or workspace-frame keybindings while the only
  possible result is a duplicate whole-screen Library/Search/Settings frame.

## Visual Readiness Checklist

- Task 007 local visual smoke was attempted on 2026-05-14 but GPUI could not
  initialize the X11/GPU context in this environment. Later operator screenshots
  and confirmations below resolved the Phase 3 visual gate.
- User screenshot on 2026-05-14 showed the first Task 007 pass rendered false
  SourceList/Detail placeholders and squeezed Library into the Content frame.
  Follow-up patch makes the transitional workspace two-frame: active whole
  screen plus Queue placeholder. Later operator confirmation below resolved this
  layout issue.
- Follow-up screenshots on 2026-05-14 showed disabled frame Back/Forward chrome,
  unfiltered Library rows under Search type chips, raw local-track deferred
  panel 404s, and no search-row library membership status. Follow-up patch
  hides inert frame navigation, uses a Search breadcrumb root in the inspector,
  filters Library and Index sections through the selected result type, resolves
  local deferred panels to empty states, and adds row membership status. User
  confirmation on 2026-05-14 resolved the regression set.
- Follow-up report on 2026-05-14 showed scroll panes receiving wheel events
  without visible scrolling, plus Recent Feeds becoming inaccessible after
  search. Follow-up patch makes the workspace mount/frame-shell scroll chain
  explicitly bounded, restores the Recent Feeds command outside the empty root,
  and adds architecture/test guards for the regression ratchet. User confirmed
  on 2026-05-14 that scrolling works across the affected panes, Recent Feeds is
  reachable after search, search filters apply to visible result sections, and
  raw 404 panel errors are gone from normal inspector display.
- User visual pass on 2026-05-15 confirmed the normal-width frame hierarchy and
  light/dark rendering. The same pass found compact search submit was no longer
  directly visible and playlist rows did not fully track UI scale. Follow-up
  patch restores a direct compact search icon submit, routes playlist row and
  thumbnail geometry through scale-aware layout tokens, and adds architecture
  guards. User confirmed the compact search affordance on 2026-05-15.
- Follow-up visual pass on 2026-05-15 showed the compact search submit icon was
  visible but optically too small, and Settings text fields collapsed instead
  of filling the frame at XL scale. Follow-up patch routes the search affordance
  through the bundled vector search icon and gives Settings form inputs a
  full-width, scale-aware column contract. User confirmed both fixes on
  2026-05-15.
- [x] Light and dark default workspace show the expected frame hierarchy
      without overlap.
- [x] Narrow width collapses optional frames before hiding global search or
      primary navigation. Implementation now omits QueueNowPlaying below
      `WORKSPACE_QUEUE_COLLAPSE_BREAKPOINT` and Detail below
      `WORKSPACE_SECONDARY_DETAIL_COLLAPSE_BREAKPOINT`.
- [x] Frame chrome Back/Forward uses symbols, not text-only buttons.
- [x] Track inspector contains only track actions, no playlist-return button.
- [x] Queue frame can collapse and restore while toolbar Now Playing remains
      readable.
- [x] Queue frame shows multi-track queue rows, current-track emphasis,
      previous/play-next transport controls, liveValue output menu, and volume
      slider.
- [x] Global toolbar Now Playing is compact: status/title plus play/pause only,
      with detailed queue/output controls absent.
- [x] liveValue output picker communicates unavailable routing without offering
      an active no-op command.
- [x] Search remains dispatchable and scope remains reachable at compact width.
      Implementation now keeps a direct search icon button beside the compact
      scope menu; user confirmed compact visibility on 2026-05-15.
- [x] SourceList selection remains visibly persistent.
- [x] UI remains dense, quiet, and utilitarian; no hero panels, decorative
      cards, or branding-forward chrome.
- User visual confirmation on 2026-05-15 provided default and narrow Library
  screenshots showing persistent source selection, track detail in the content
  frame, queue frame expansion, and queue collapse before toolbar nav/search
  disappears. Earlier 2026-05-15 confirmation covered light/dark rendering and
  compact search/icon fixes.
- 2026-05-15 lower-context review found Task 012, Task 013, and Task 014 still
  proposed. Task 012 was then implemented: `WorkspaceLayout` exposes
  `add_frame(kind) -> Result<WorkspaceFrameId, ...>`, protects last-frame
  removal, preserves focus invariants, and persists `workspace_layout` through
  `config.toml` with malformed-config fallback. Task 013 visible commands were
  later deferred; Task 014 is complete as a model-only Phase 6 slice.
- 2026-05-15 Task 013 first implementation exposed shared frame action menu ids
  and keybindings. User feedback showed the menu first opened an unusable
  placeholder frame, then a misleading duplicate Library frame, while
  `ctrl-shift-n` still did not trigger useful behavior on Linux. The corrected
  Task 013 stance is to defer the user-visible open/close commands and project
  persisted extra content frames out of the transitional layout. Breadcrumbs,
  move/resize, and independent per-frame Library/Search/Settings routing remain
  non-goals for Task 013 and belong to later ADR 0047/frame-navigation work.
- 2026-05-15 local visual smoke was attempted with `cargo run` and
  `LIBGL_ALWAYS_SOFTWARE=1 cargo run`; both failed before opening a GPUI window
  due to X11/GPU initialization errors. Operator visual confirmation supplied
  the Task 013 duplicate-frame proof afterward.
- [ ] Task 013 default layout visual confirmation.
- [x] Task 013 confirms no duplicate second Library/Search/Settings frame is
      reachable from frame chrome or keybindings.
- [ ] Task 013 focused-frame indicator visual confirmation.
- [ ] Task 013 light/dark visual confirmation.

## Test Gates

Each implementation phase must run:

```bash
cargo fmt -- --check
cargo check
cargo test
cargo clippy -- -D warnings
```

Focused task gates may run narrower commands while the task is in progress,
but phase readiness requires the full gate above.

## Merge Recommendation

Task 013 may close as a visually confirmed command-deferral slice, and Task 014
model-only detach/dock metadata may merge after automated gates. User-visible
multi-frame open/close commands remain deferred until a later ADR 0047 task
provides real per-frame content owners.
