# ADR 0032 UI Backend Boundary Phase Plan

## Goal

Make UI/backend boundaries and shared UI chrome ownership enforceable enough
that screens cannot quietly reintroduce divergent release-detail popovers.

## Non-Goals

- No service, database, or schema changes.
- No navigation redesign.
- No replacement of GPUI or the existing design system.
- No broad cleanup of unrelated legacy panels.

## Assumptions

- ADR 0031's release-detail contract remains the release page source of truth.
- Screen modules still own command dispatch and callbacks.
- Shared primitives/composites own popover chrome.

## Affected Modules

- `src/library.rs`
- `src/ui/composites/playlist_popover.rs`
- `src/ui/primitives/popover.rs`
- `src/view_models/library.rs`
- `tests/architecture_tests.rs`
- `docs/architecture/ui-backend-boundary.md`

## Proposed Sequence

### Phase 1 - Playlist Popover Contract Repair

Status: Completed.

Task: `docs/tasks/adr-0032-task-001-playlist-popover-contract.md`

Migrate Library release and track add-to-playlist affordances to the canonical
`AddToPlaylistPopover` composite, remove stale Library view-model popover-open
state, and document the boundary rule.

### Phase 2 - Architecture-Test Enforcement

Status: Completed.

Add architecture tests that prevent new screen-local playlist popover panels
and make task packets include UI/backend boundary checks.

Task: `docs/tasks/adr-0032-task-002-architecture-test-enforcement.md`

### Phase 3 - Inspector Playlist Popover Migration

Status: Completed.

Migrate the remaining Library/Discover inspector playlist panels and the stale
Discover row popup wrapper to `AddToPlaylistPopover`, remove screen-owned
popover-open state, and tighten the architecture-test baseline to zero.

Task: `docs/tasks/adr-0032-task-003-inspector-popover-migration.md`

## Risk Areas

- Accidentally changing playlist command semantics while fixing chrome.
- Moving command dispatch into shared UI composites.
- Keeping stale view-model state for visual popover open/closed chrome.
- Reintroducing raw full-width panels as row children.
- Disabling inspector playlist actions differently while moving from
  screen-local buttons to the shared composite.

## Test Strategy

- `cargo fmt -- --check`
- `cargo check`
- `cargo test view_models::library`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`
- Manual visual smoke for Library album and track playlist popovers.

## Rollback Strategy

Phase 1 can be reverted by restoring the Library screen-local panels, but that
also restores the full-width popover regression. Prefer fixing the shared
popover composite over reverting to raw panels.
