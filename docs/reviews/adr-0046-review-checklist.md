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

Status: Planning packet complete - 2026-05-14.

Readiness decision: **Proceed to Task 001 only**.

Do not begin QueueNowPlaying implementation until Phase 2 and Phase 3 frame
navigation/chrome work is complete and visually checked.

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
- [ ] Phase 3 implementation complete.
- [ ] Phase 4 implementation complete.
- [ ] Full final gate green.

## Required Fixes

- None before starting Task 001.

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

## Visual Readiness Checklist

- Task 007 local visual smoke was attempted on 2026-05-14 but GPUI could not
  initialize the X11/GPU context in this environment. Manual visual review is
  still required before Phase 3 is closed.
- User screenshot on 2026-05-14 showed the first Task 007 pass rendered false
  SourceList/Detail placeholders and squeezed Library into the Content frame.
  Follow-up patch makes the transitional workspace two-frame: active whole
  screen plus Queue placeholder. Manual confirmation is still required.
- Follow-up screenshots on 2026-05-14 showed disabled frame Back/Forward chrome,
  unfiltered Library rows under Search type chips, raw local-track deferred
  panel 404s, and no search-row library membership status. Follow-up patch
  hides inert frame navigation, uses a Search breadcrumb root in the inspector,
  filters Library and Index sections through the selected result type, resolves
  local deferred panels to empty states, and adds row membership status. Manual
  confirmation is still required.
- Follow-up report on 2026-05-14 showed scroll panes receiving wheel events
  without visible scrolling, plus Recent Feeds becoming inaccessible after
  search. Follow-up patch makes the workspace mount/frame-shell scroll chain
  explicitly bounded, restores the Recent Feeds command outside the empty root,
  and adds AGENTS/architecture/test guards for the regression ratchet. Manual
  confirmation is still required.
- [ ] Light and dark default workspace show the expected frame hierarchy
      without overlap.
- [ ] Narrow width collapses optional frames before hiding global search or
      primary navigation.
- [ ] Frame chrome Back/Forward uses symbols, not text-only buttons.
- [ ] Track inspector contains only track actions, no playlist-return button.
- [ ] Queue frame can collapse and restore while toolbar Now Playing remains
      readable.
- [ ] Search remains dispatchable and scope remains reachable at compact width.
- [ ] SourceList selection remains visibly persistent.
- [ ] UI remains dense, quiet, and utilitarian; no hero panels, decorative
      cards, or branding-forward chrome.

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

Proceed with Task 001. Re-review after each task before handing the next task
to a lower-context implementation model.
