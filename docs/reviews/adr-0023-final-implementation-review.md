# ADR 0023 Final Implementation Review

## Reviewed Artifact

ADR 0023 implementation and finalization through Task 011 on 2026-04-30:

- design tokens, primitives, composites, theme bridge, and scale bridge
- `library-token-intent`
- `search-inspector-token`
- final screen color/layout literal audit
- `adr-0023-task-006-shared-split-pane-shell`
- `adr-0023-task-007-release-detail-surface`
- `adr-0023-task-008-library-row-semantics`
- `adr-0023-task-009-command-intent-finish`
- `adr-0023-task-010-boundary-gates`
- `adr-0023-task-011-final-review`

## Result

Pass for ADR 0023 scope.

ADR 0023 is finalized as a design-system and GPUI-free view-model boundary
ADR. It does not claim the whole app is GPUI-independent and does not include a
broad CommandBus, QueryService, EventBus, or screen-directory split.

## Required Fixes

None.

## Verification

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --lib view_models::library`
- `cargo test --lib view_models::search`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`
- `cargo test`
- `cargo build`
- `git diff --check`

## Architectural Drift

No broad CommandBus, QueryService, EventBus, or screen-directory split was
introduced. The implementation stayed inside the ADR 0023 boundary:
view-models own pure labels/status classification; screens keep GPUI event
wiring and service dispatch. `tests/architecture_tests.rs` now enforces the
view-model import boundary so this does not rely only on review.

## Design-System Review

- Screen-level `rgb(...)` literals are removed from `app.rs`, `library.rs`,
  and `search.rs`.
- Screen-level numeric `px(...)` literals are removed from `app.rs`,
  `library.rs`, and `search.rs`.
- Fixed geometry that remains screen-visible is routed through named
  `theme::layout` or `theme::typography` constants.
- Discover and Library use the same `SplitPane` shell and pure resize-state
  contract.
- Discover feed detail and Library album detail now share
  `ReleaseDetailSurface`, `DetailHeader`, and `TrackRow`, so the same release
  has the same structural presentation across modes.
- Library album rows no longer show redundant per-row downloaded labels.
- Ghost action buttons now default to accent text instead of on-accent text,
  addressing low-contrast secondary controls on dark surfaces.
- New projection code remains GPUI-free.

## Residual Risk

- Manual visual smoke still matters before a release build: Discover resize,
  Library resize, Discover feed detail, Library album detail, rows with and
  without local files, and MusicBrainz/tag panels.
- The remaining hardcoded `Appearance::Dark` calls are app/bootstrap
  compatibility paths and are explicitly allowlisted in
  `tests/architecture_tests.rs`.
- `library.rs` and `search.rs` remain large GPUI adapters that dispatch
  services directly. Broader command/query/event architecture is deferred to a
  later ADR.

## Merge Recommendation

Mergeable. Verification commands listed above are green.
