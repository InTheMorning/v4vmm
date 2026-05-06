# ADR 0025 Task 003b Review: Screen Button Style Sweep

## Reviewed Scope

- `src/app.rs`
- `src/library.rs`
- `src/search.rs`
- `tests/architecture_tests.rs`

## Verdict

Pass.

Task 003b can be treated as complete. The next ADR 0025 implementation packet
is Task 004, typed badge roles.

## Required Fixes

None.

## Inventory And Disposition

Migrated to `ControlStyle`:

- `src/app.rs`: settings save, settings defaults, cached-track delete, delete
  all cached.
- `src/library.rs`: playlist sort/add toolbar controls, new playlist add,
  library search, apply feed updates, check all feeds, album-track
  subscribe/remove, album-track add-to-playlist, playlist move up/down/remove,
  playlist play, playlist rename, playlist delete.
- `src/search.rs`: fuzzy toggle, load more, type filter pills, inspector back,
  recent-feed load more.

Compatibility debt with `CONTROL-COMPAT(reason): ...`:

- `src/search.rs`: search submit button because native `Button` does not yet
  expose loading state.
- `src/search.rs`: MusicBrainz release picker because native `Button` does not
  yet expose dropdown menu, full-width alignment, and custom badge fill
  styling.
- `src/search.rs`: play icon button and track download/remove icon button
  because native `Button` does not yet expose tooltip plus fixed square
  icon-button geometry.
- `src/library.rs`: MusicBrainz release picker because native `Button` does not
  yet expose dropdown menu, full-width alignment, and custom badge fill
  styling.

One-off:

- None left unmarked.

## Architectural Review

- Reusable screen-level button style chains now go through
  `UiButton::styled(..., ControlStyle::...)`.
- Remaining direct `gpui_component::Button` call sites are explicitly marked
  compatibility debt.
- Architecture tests now allow zero unmarked direct component-button call
  sites in screen files.
- Workflow logic, command/query behavior, labels, disabled states, and click
  handlers were preserved.

## Tests Run

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo test`
- `cargo clippy --lib --tests -- -D warnings`

## Residual Risk

The native `Button` still lacks loading, dropdown-menu trigger, tooltip, and
fixed square icon-button affordances. Those are now explicit compatibility
gaps for later primitive/control-style work instead of hidden screen styling.
