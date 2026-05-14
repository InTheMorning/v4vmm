# ADR 0046 Task 001: Workspace Model Types

Status: Proposed - 2026-05-14.

## Goal

Introduce GPUI-free workspace model types so later frame-shell and
navigation work has a typed contract to render. No visible UI change.

## Files to Inspect

- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/plans/adr-0046-workspace-frame-architecture-phase-plan.md`
- `src/view_models/mod.rs`
- `src/view_models/library.rs` (for VM doc/style precedent)
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/view_models/workspace.rs` (new)
- `src/view_models/mod.rs` (module declaration)
- `tests/architecture_tests.rs`

## Do Not Touch

- Any `src/ui/*` rendering
- `src/library*`, `src/search*`, `src/app*`
- `src/db.rs`, playback engine, mpv driver

## Constraints

- No `gpui::*` import anywhere in `src/view_models/workspace.rs`.
- Use enums to make invalid states unrepresentable.
- `M-CANONICAL-DOCS` on every public type and function.
- Derive `Debug`, `Clone`, `Eq`, `PartialEq` where reasonable. No
  smart pointers in public type fields.
- Builder pattern only if any type reaches four or more parameters.

## Implementation Steps

1. Add `src/view_models/workspace.rs` and declare it in
   `src/view_models/mod.rs`.
2. Define `WorkspaceFrameId(u64)` newtype with constructor + accessor.
3. Define `WorkspaceFrameKind` enum: `SourceList`, `ContentList`,
   `Detail`, `QueueNowPlaying`.
4. Define `WorkspaceFrameState` carrying `id: WorkspaceFrameId`,
   `kind: WorkspaceFrameKind`, `title: String`, optional
   `subtitle: String`, optional `status: String`, focused flag.
5. Define `WorkspaceLayout` carrying ordered frames (`Vec<
   WorkspaceFrameState>`) and the focused frame id. Provide
   constructors for the default layout and operations to mutate
   layout (returning `Result`).
6. Define `FrameNavigationState` per-frame: forward stack, back stack,
   current entry. `FrameNavigationEntry` enum covers
   `PlaylistDetail(playlist_id)`, `TrackDetail(track_id)`,
   `AlbumDetail(album_id)`, `ArtistDetail(name)`,
   `Search(query)`, plus `SourceList`, `QueueNowPlaying` markers.
7. Implement push/pop/focus operations as pure methods on the types.
8. Unit tests in `mod tests` for: default layout shape, push/pop
   round-trip, back/forward disabled at boundaries, focus invariants,
   and invalid mutation returning `Err`.
9. Architecture guard: assert `src/view_models/workspace.rs` contains
   no `use gpui` / `gpui::` strings.

## Acceptance Criteria

- [ ] `src/view_models/workspace.rs` exists and compiles.
- [ ] All public types document summary, behavior, errors, panics where
  applicable.
- [ ] Unit tests cover empty layout, single-frame layout, multi-frame
  layout, back/forward boundary states, and focus invariants.
- [ ] Architecture guard confirms no `gpui` imports in
  `src/view_models/workspace.rs`.
- [ ] No `src/ui/*` or screen module changed.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test workspace
cargo test --test architecture_tests
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0046-workspace-frame-architecture.md`
- `docs/plans/adr-0046-workspace-frame-architecture-phase-plan.md`
- `src/view_models/mod.rs`
- `src/view_models/library.rs` (style precedent)
- `tests/architecture_tests.rs`

Goal:
- Add GPUI-free workspace model types in
  `src/view_models/workspace.rs`: `WorkspaceFrameId`,
  `WorkspaceFrameKind`, `WorkspaceFrameState`, `WorkspaceLayout`,
  `FrameNavigationState`, `FrameNavigationEntry`.

Constraints:
- No `gpui::*` imports.
- Enums model state; invalid states unrepresentable.
- Public types carry summary + applicable section docs.
- Fallible operations return `Result`, never panic on bad input.
- No smart pointers in public type fields.

Do not touch:
- Any `src/ui/*`
- `src/library*`, `src/search*`, `src/app*`
- `src/db.rs`, playback engine

Acceptance criteria:
- Module exists with documented public types.
- Unit tests cover layout, navigation, and focus invariants.
- Architecture guard asserts no `gpui` references in
  `src/view_models/workspace.rs`.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test workspace`
- `cargo test --test architecture_tests`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Workspace types require GPUI imports to compile (signals a
  layering error to escalate before adding `use gpui`).
- Existing `src/view_models/mod.rs` cannot host a new module without a
  broader refactor.
