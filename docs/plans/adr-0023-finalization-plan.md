# ADR 0023 Finalization Plan

## Goal

Finish ADR 0023 honestly: the UI should have a coherent design-system shape,
shared shell behavior between Discover and Library, and enforceable
GPUI-free view-model boundaries. This plan does not claim the whole app is
GPUI-independent; it makes the non-GPUI application state and projection
boundary real enough that later ADRs can move command/query/event
architecture without fighting screen-specific UI drift.

## Non-goals

- Do not introduce a broad `CommandBus`, `QueryService`, or `EventBus`.
- Do not split `library.rs` or `search.rs` into directories.
- Do not change database schema or feed subscription semantics.
- Do not redesign the product visually beyond making equivalent entity
  surfaces behave consistently.
- Do not move GPUI out of presentation components; ADR 0023 only requires
  `view_models/*`, domain, and service layers to stay GPUI-free.

## Current State

- Tokens, primitives, composites, `Environment`, scale bridging, contrast
  tests, and screen-level literal audits are in place.
- `LibraryViewModel` and `SearchViewModel` own many pure snapshots and local
  transitions.
- Discover has a resizable split between results and inspector. Library still
  has a fixed two-pane layout.
- Discover feed detail and Library album detail now share `DetailHeader` and
  `TrackRow`, but they do not yet share a detail shell or layout contract.
- Library album rows still expose redundant downloaded state in the row even
  though the `Remove` action already implies library membership.
- The old `docs/reviews/adr-0023-final-implementation-review.md` is too
  optimistic and is superseded by this finalization plan.

## Target State

- Discover and Library both use the same resizable split-pane layout primitive
  or composite, with pure resize state outside GPUI event wiring.
- Equivalent release/feed/album detail surfaces share the same structural
  contract: header, action bar, detail grid, section header, rows, and
  optional mode-specific panels.
- Redundant Library-only downloaded labels are removed from rows; downloaded
  counts remain only in aggregate detail rows where they add information.
- Remaining status formatting and service-dispatch setup in screens is moved
  into narrow command-intent/result values where that reduces screen logic.
- Import-boundary and token-literal expectations are enforced by tests, not
  just by code review.
- ADR 0023, the migration plan, tasks, and review checklist all describe the
  same state.

## Assumptions

- Library should gain Discover-style sidebar/detail resizing.
- Library-specific actions such as `MusicBrainz`, tag compare, and remove
  should remain available, but as mode-specific trailing controls inside the
  shared skeleton.
- "Perfectly themable" means no screen-owned visual literals or hardcoded dark
  defaults in render paths; it does not mean every color in `theme.rs` has been
  removed.
- "Non-GPUI-dependent" means view-models, domain, and services compile without
  GPUI imports. Presentation components remain GPUI by design.

## Affected Modules

- `src/library.rs`
- `src/search.rs`
- `src/view_models/library.rs`
- `src/view_models/search.rs`
- `src/view_models/mod.rs`
- `src/ui/composites/*`
- Possible new `src/ui/layouts/*` or `src/ui/composites/split_pane.rs`
- `src/ui/theme.rs`
- `docs/adr/0023-design-system-and-view-models.md`
- `docs/plans/adr-0023-design-system-migration.md`
- `docs/reviews/adr-0023-review-checklist.md`

## Proposed Sequence

1. `adr-0023-task-006-shared-split-pane-shell`: introduce a shared resizable
   split-pane component and pure resize state, then wire both Discover and
   Library through it.
2. `adr-0023-task-007-release-detail-surface`: introduce a shared release
   detail surface contract so feed/album detail structure is identical across
   modes.
3. `adr-0023-task-008-library-row-semantics`: remove redundant Library row
   downloaded labels and move remaining album-row labels/actions into
   projections.
4. `adr-0023-task-009-command-intent-finish`: add narrow command intents for
   the remaining high-noise screen workflows, without building a broad bus.
5. `adr-0023-task-010-boundary-gates`: add automated architecture tests for
   GPUI-free view-models, no screen-level raw color/layout literals, and no
   hardcoded dark render defaults.
6. `adr-0023-task-011-final-review`: reconcile ADR status, migration docs, and
   visual/manual review notes after implementation is green.

## Schema/API Implications

None. This plan is presentation and application-boundary work only.

## Risk Areas

- Shared split-pane wiring touches both root screen layouts and can break
  focus, scrolling, resize gestures, or inspector widths.
- Shared release detail structure can accidentally erase Library-only actions
  or Discover-only playlist/download behavior.
- Moving command-intent setup can change status messages or error propagation.
- Architecture tests that scan source files can be brittle if they do not
  allow intentional literals inside token/theme/component layers.

## Test Strategy

- Run `cargo fmt -- --check` and `cargo check` after every task.
- Run focused VM tests after any view-model edits:
  `cargo test --lib view_models::library` and/or
  `cargo test --lib view_models::search`.
- Run `cargo clippy --lib --tests -- -D warnings` after shared component or
  architecture-test work.
- Run full `cargo test` and `cargo build` before marking ADR 0023 finalized.
- Perform manual UI smoke for:
  Discover resize, Library resize, Library album detail, Discover feed detail,
  track rows with and without local files, and MusicBrainz/tag panels.

## Rollback Strategy

Each task must be independently revertible. Shared split-pane work should land
before release-detail unification so a broken layout can be reverted without
discarding later command-intent or boundary-test changes. Do not combine more
than one task packet in a single commit unless explicitly directed.

## Open Questions

- Should Library persist its split-pane width independently from Discover, or
  should both use one shared preference?
- Should aggregate "Downloaded" counts remain in Library album/artist detail
  grids, or should local membership be represented only through actions?
- Should ADR 0023 stop at narrow command intents, or should a new ADR
  immediately follow for CommandBus / QueryService / EventBus?
