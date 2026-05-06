# ADR 0023 Review Checklist

## Architecture

- `view_models/*` has no imports from `gpui`, `gpui_component`, screen modules,
  or `ui`.
- Primitives do not import composites, screens, services, or view-models.
- Composites do not call services or own domain state.
- Screens remain the only layer wiring GPUI events to service calls until a
  separate command-bus ADR exists.
- Discover and Library use the same split-pane shell and resize semantics.
- Shared entity/release surfaces own layout order; screens only provide
  mode-specific actions, panels, and event handlers.

## Design System

- New screen colors use `SemanticColor` through tokens or an explicitly named
  compatibility helper.
- New spacing, radius, type, and control sizes use tokens unless the value is
  fixed content geometry.
- Entity badges use `TagBadge` / `EntityKind` when rendering entity identity.
- Equivalent release/feed/album detail screens use the same structural
  components across Discover and Library.
- Library rows do not show redundant state labels when an existing action
  already communicates that state.
- Components remain dense and utilitarian; no decorative marketing layouts.

## Tests

- New view-model projections and transitions have unit tests.
- Architecture-boundary tests enforce no GPUI imports under `view_models`.
- Architecture-boundary tests enforce no raw screen-level color/layout
  literals or hardcoded dark render defaults.
- `cargo fmt -- --check` is green.
- `cargo check` is green.
- `cargo clippy --lib --tests -- -D warnings` is green for Rust changes.
- Broaden to `cargo test` when behavior crosses service or shared model
  boundaries.
- Manual visual smoke covers Discover resize, Library resize, Discover feed
  detail, and Library album detail before ADR 0023 is marked finalized.

## Documentation

- ADR 0023 and `docs/plans/adr-0023-design-system-migration.md` reflect the
  current code status after each slice.
- Task packet status is updated when a slice lands.
- New docs live under the purpose-based folders listed in `docs/README.md`.
