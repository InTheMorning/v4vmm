# ADR 0025: Theme, Icon, and Style Boundary

## Status

Accepted - 2026-05-01. Implementation partial: the phase plan records
seven of eleven phases implemented by task packets. This ADR remains the live
theme, icon, and style boundary; changes affecting tokens, primitives,
composites, or theme contracts route through bounded ADR 0025 tasks per
`docs/plans/deferred-architecture-work-index.md` priority item 5.

## Context

ADR 0023 established the design-system foundation: semantic tokens,
primitives, composites, view-model projections, runtime scale, light/dark
palette tests, and architecture gates that prevent raw screen-level color and
numeric spacing literals. ADR 0024 then moved high-blast-radius workflows behind
the application boundary so GPUI screens are closer to thin presentation
adapters.

That work makes the app substantially easier to style, but it does not yet make
theme, icon, and control-style changes boring. The current state is mixed:

- `src/ui/tokens.rs` owns semantic colors, spacing, radius, typography, scale,
  `Appearance`, and `Environment`.
- `src/ui/theme_bridge.rs` correctly pushes token colors into
  `gpui_component`.
- `src/ui/theme.rs` has been removed. Remaining fixed geometry lives in
  `src/ui/layouts.rs`, and bridge-aware compatibility color roles live in
  `src/ui/style.rs` while legacy screens continue moving to direct tokens and
  primitives.
- Screens still contain many direct `gpui_component::Button::new(...)` call
  sites and per-call style chains.
- `src/ui/primitives/button.rs` already defines a token-native button
  primitive, but screens do not use it yet.
- RSS/Nostr/playback/status icons are still spread across helpers, inline SVG,
  string glyphs, and badge emoji rather than a single semantic icon boundary.
- Some visual decisions remain encoded by entity-type strings such as `"feed"`
  and `"track"` rather than typed visual roles.

The ideal architecture in `docs/architecture/architecture-diagrams.md` calls
for a design system of tokens, primitives, composites, and layouts where
screens compose existing pieces instead of owning visual rules. In that target,
theme changes flow through semantic aliases, icon changes flow through an icon
primitive, and control-style changes flow through named button/action variants.

The practical goal of this ADR is not a visual redesign. It is to finish the
visual-system boundary so the app can later support dark, light, high-contrast,
custom accent, and icon/style changes without touching `library.rs`,
`search.rs`, or `app.rs` for ordinary visual work.

## Decision

Treat theme, icons, and reusable control styles as a first-class design-system
boundary under `src/ui/`. Screens may choose semantic intent; they must not
choose palette values, inline icon SVG, string glyphs, or bespoke button chrome
for reusable patterns.

The target module shape is:

```text
src/ui/
  tokens.rs              existing semantic token base
  theme_bridge.rs        existing gpui-component bridge
  theme_profiles.rs      profile-specific semantic color resolution
  layouts.rs             fixed geometry for reusable UI shells
  style.rs               bridge-aware compatibility roles
  icons.rs               semantic icon catalog and Icon primitive facade
  control_styles.rs      role mapping for the native Button primitive
  primitives/
    button.rs            native button primitive consumed by control roles
    icon.rs              optional primitive if `icons.rs` grows too large
  composites/
    action_button.rs     migrated to control style roles
    tag_badge.rs         migrated to typed entity/status/provenance roles
```

The names above are normative unless implementation finds a clearer local
module split. The architecture boundary is more important than exact file
placement: semantic visual intent belongs in `ui/`, not in screens.

`src/theme_profile.rs` owns the GPUI-free persisted profile enum. UI bridge
code maps that profile to token appearances when installing a theme.

### Theme profile boundary

`tokens.rs` remains the lowest-level source of truth for semantic dimensions:
`SemanticColor`, `Spacing`, `Radius`, `FontSize`, `Size`, `Appearance`,
`ScaleFactor`, and `Environment`.

Add a theme-profile layer that defines complete, named visual profiles. The
profile type itself must stay GPUI-free so config and non-UI code can carry the
choice without importing the UI layer:

- `ThemeProfile::System`
- `ThemeProfile::Dark`
- `ThemeProfile::Light`
- `ThemeProfile::HighContrastDark`
- `ThemeProfile::HighContrastLight`

`System` follows GPUI's reported window appearance and resolves to the matching
Dark or Light profile at install time. The app observes window appearance
changes and reinstalls the profile when System is selected.

`ThemeProfile` must not be a bag of arbitrary user-provided hex strings in the
first slice. Custom accent color can be introduced later after the semantic
roles are complete and contrast tests can validate it. This keeps the first
work focused on replacing leaks rather than inventing a theme editor.

The old dark-only `theme::color::*` helper namespace is removed. New code must
use `tokens::color(cx, SemanticColor::...)`, profile-resolved roles,
primitives, or composites. Existing fixed-geometry and compatibility color
roles live in `ui::style` and resolve through the appearance installed by
`theme_bridge`.

`theme_bridge::install_theme` must take a `ThemeProfile`, not leave profile
resolution to callers:

```rust
pub fn install_theme(profile: ThemeProfile, scale: ScaleFactor, cx: &mut App)
```

`ui::theme_profiles` owns the mapping from `ThemeProfile` to `Appearance` and
profile-specific semantic color resolution. `theme_bridge` installs those
resolved colors into `gpui_component`. Current theme-install call sites must
pass `ThemeProfile::Dark` until config/profile selection exists. Keeping
`install_theme(appearance, scale, cx)` as the primary API is not allowed after
Task 001, because downstream icon and control code need one profile-driven
theme contract.

### Icon boundary

Introduce a semantic icon catalog. Screens request intent; the design system
chooses the concrete glyph/SVG/vector:

```rust
pub enum IconName {
    Rss,
    Nostr,
    Play,
    Pause,
    Stop,
    Previous,
    Next,
    Download,
    Remove,
    Check,
    Warning,
    Error,
    Info,
}
```

The list above is the initial migration set, not a complete catalog. It should
grow only when a migrated call site needs it. Icons must
take semantic color/size roles and must scale with the existing UI scale path.
Hardcoded icon colors are allowed only inside the icon catalog when they
represent an intentional brand or protocol identity, such as RSS orange or
Nostr purple, and must still pass a non-text contrast check for their usage.

Use Apple HIG as the shape guide: icons should align with text, scale with the
control or row they sit in, use a single consistent visual language, and avoid
using color as the only state indicator. SF Symbols names may be used as
semantic inspiration, but this Rust/GPUI app does not need to depend on the
Apple platform symbol runtime.

Inline SVG helpers and string glyph constants in screen modules are temporary.
They should move behind `ui::icons` before new iconography is added.

### Control-style boundary

ADR 0025 makes `ControlStyle` the public, screen-facing role layer for the
existing native `ui::primitives::Button`. It does not introduce a third button
vocabulary. `ui::primitives::Button` remains the concrete token-native
primitive; `ControlStyle` maps reusable product roles onto that primitive's
variant, size, icon, color, and focus behavior. Screens should migrate away from
direct `gpui_component::Button` styling for reusable action patterns.

Direct `gpui_component::Button` use may remain temporarily only where a
specific third-party widget capability has not yet been represented by the
native primitive. Each such call site must be treated as compatibility debt, not
as a parallel styling system.

Compatibility debt must use a concrete marker, not prose in a review comment.
Any remaining direct screen-level `gpui_component::Button` style chain must
have a preceding or same-line marker:

```rust
// CONTROL-COMPAT(reason): native Button cannot yet represent <capability>.
```

Architecture tests must fail on unmarked direct `gpui_component::Button`
references in screen files and list the file/line for every violation. This
keeps the exception count visible and prevents the compatibility path from
becoming a permanent second style system.

Introduce reusable control style roles for common interactive elements:

- `ControlStyle::Primary`
- `ControlStyle::Secondary`
- `ControlStyle::Ghost`
- `ControlStyle::Destructive`
- `ControlStyle::ToolbarIcon`
- `ControlStyle::RowAction`
- `ControlStyle::MetadataAction`
- `ControlStyle::Pill`

Initial non-obvious role admission examples:

- `ToolbarIcon`: playlist sort/add buttons in `src/library.rs:1724` and
  `src/library.rs:1734`.
- `RowAction`: playlist move/remove actions in `src/library.rs:2674` and
  `src/library.rs:2696`.
- `Pill`: fuzzy/type filter controls in `src/search.rs:1723` and
  `src/search.rs:2251`.
- `Ghost`: load-more/default/back actions in `src/search.rs:1780`,
  `src/search.rs:2362`, and `src/app.rs:563`.

The first implementation can expose these as constructors, builders, or
modifiers on the native primitive; it does not need a framework. The
requirement is that call sites say "this is a destructive row action" or "this
is a toolbar icon button" rather than repeating border, background, text color,
radius, font size, and compactness chains.

`ActionButton` should become a thin wrapper over `ControlStyle::MetadataAction`
instead of owning a parallel style. Direct
`Button::new(...).ghost().text_color(...).border_color(...)` chains in screens
should be migrated when the pattern is reusable. One-off layout wiring and
click handlers remain in screens.

A new `ControlStyle` role is admitted only when at least two unrelated screens
or composites currently use the same visual/action pattern, or when a role
encodes a state or contrast requirement that a generic chain cannot express.
Single-screen styling belongs in the local composite until it proves reusable.

Task 003 owns the control-style boundary, `ActionButton` migration, pure role
mapping tests, and the `CONTROL-COMPAT` architecture-test mechanism. Task 003b
owns the screen-level chain sweep for reusable button/action patterns in
`app.rs`, `library.rs`, and `search.rs`. It may preserve direct
`gpui_component::Button` only for marked compatibility cases.

### Badge and entity-role boundary

Entity and status badges must use typed roles, not string-keyed color maps.
`TagBadge` / `EntityKind` are the preferred direction. The former
`theme::badges` compatibility shim has been removed.

The replacement should distinguish:

- entity roles: feed, track, artist, publisher, release, recording, playlist,
  generic
- status roles: success, warning, danger, info, pending, disabled
- provenance/diff roles: match, different, missing

Each role must define both visual color and non-color affordance where needed:
label, icon, shape, text, or another cue. Color alone is not sufficient for
state.

Provenance/diff display is owned by the typed badge/status role migration. It
may consume icons from `ui::icons`, but the semantic role belongs with the
badge/status visual-role layer so color, glyph/icon, label, and accessibility
text are resolved together.

General status message roles follow the same rule. `StatusRole` lives with the
typed visual roles exported by `ui::composites`, not in `ui::style`, so status
color and glyph semantics are resolved together.

The old `style::color::diff_*` compatibility helpers are removed after all
screen call sites route through `ProvenanceRole`; architecture tests keep the
screen baseline and `ui::style` helper count at zero.

### Runtime settings

The Settings tab may eventually expose appearance/profile and accent choices,
but not before the render paths consume semantic boundaries. Adding controls
too early would only make it possible to switch a partially themed app.

After the migration, settings should persist:

- `ThemeProfile`
- `Appearance` override or system-following mode, if supported
- optional accent color, only after contrast validation exists

Changing a visual setting must reinstall the theme through `theme_bridge`,
refresh windows, and avoid requiring screen-specific repaint code.

## Invariants

- Screens do not construct raw colors, inline icon SVG, or string glyphs for
  reusable visual roles.
- New screen code does not call `theme::color::*`, `theme::badges`, or
  `theme::glyphs`.
- Icons are requested by semantic `IconName`, not by copied SVG or ad-hoc text.
- Reusable buttons and actions use named control styles instead of per-call
  border/background/text-color chains.
- `ControlStyle` maps to the native `ui::primitives::Button`; direct
  `gpui_component::Button` styling in screens is compatibility debt.
- Entity and status badges use typed roles rather than string-keyed color maps.
- The visual system continues to follow the tokens -> primitives -> composites
  -> screens shape from ADR 0023.
- `src/ui/` may depend on GPUI and `gpui_component`; view-models,
  application, service, domain, and infrastructure layers must not depend on
  UI modules.
- Light and dark palettes stay first-class. Any new color role must be checked
  in both appearances.
- High-contrast profiles must improve contrast without changing workflow
  behavior, and must have contrast-matrix tests before they can be exposed.
- Color is not the sole indicator for destructive, success, warning, diff, or
  disabled states.
- Runtime visual changes flow through `theme_bridge` / `Environment` and do
  not require per-screen theme code.

## Non-goals

- This ADR does not redesign the app's product experience or information
  architecture.
- This ADR does not require replacing GPUI or `gpui_component`.
- This ADR does not require moving screens into a new `views/` directory.
- This ADR does not introduce a full end-user theme editor.
- This ADR does not require OS-level appearance detection in the first phase.
- This ADR does not require dependency on SF Symbols.
- This ADR does not change application commands, queries, events, services, or
  database schema.
- This ADR does not require every one-off button call to move at once. It does
  require Task 003 to migrate reusable button style-chain patterns in the main
  screen files or document compatibility exceptions.

## Alternatives considered

### Edit palette values now

Rejected as the first move. The token palette is already centralized enough to
make some color changes quickly, but style and icon leaks remain. Tweaking the
palette before removing those leaks would leave the app only partially
themeable and would not fix inconsistent Library/Discover affordances.

### Keep `theme.rs` as the permanent style API

Rejected. `theme.rs` was explicitly introduced as a compatibility shim during
ADR 0023. It resolves many values to dark appearance and encourages helper
names that are less semantic than `tokens`, profiles, primitives, and
composites. Keeping it permanent would preserve the current half-migrated
state.

### Build a generic CSS-like style engine

Rejected. The app is a Rust/GPUI desktop app with a small number of repeated
controls. A generic style engine would add indirection without solving the
current problem better than typed theme profiles, icons, and control roles.

### Add `ControlStyle` beside the native button primitive

Rejected. The repository already has a token-native
`ui::primitives::Button`. Adding an unrelated `ControlStyle` wrapper around
`gpui_component::Button` would create three button systems: the dormant native
primitive, direct third-party buttons in screens, and a new role vocabulary.
ADR 0025 instead makes control roles the public face of the native primitive.

### Expose arbitrary custom colors immediately

Rejected for now. Custom colors are useful, but they need role mapping and
contrast validation. Arbitrary hex input before role coverage would make it
easy to create unreadable states and hard to test the design system.

### Leave icons as inline SVG helpers

Rejected. Inline SVG is acceptable as an implementation detail inside the icon
catalog, but not as a screen-level pattern. Screen-level SVG and glyph strings
make icon changes broad and error-prone.

## Consequences

### Positive

- Theme changes become localized to token/profile definitions and the bridge.
- Icon replacement becomes a catalog change instead of a screen search.
- Button and action styling becomes consistent across Library, Discover,
  Settings, and playback surfaces.
- Architecture tests can prevent visual-system regressions the same way ADR
  0023/0024 prevent raw literals and direct workflow calls.
- High-contrast and light-mode work become practical because screens stop
  choosing dark-only helpers.
- The design system better matches the ideal architecture diagrams: tokens,
  primitives, composites, layouts, and thin screens.

### Negative

- The UI layer gains more named types before every visual benefit is visible.
- Some existing helpers will temporarily coexist with their replacements.
- Review discipline is needed so `ControlStyle` does not become a dumping
  ground for one-off styles.
- Icon fidelity may be imperfect at first if GPUI image/SVG handling limits
  what can be represented cleanly.

### Neutral

- No database migration is implied.
- No CLI behavior changes are implied.
- Existing view-model and application boundaries remain valid.
- The current dark appearance can remain the default while the boundary is
  migrated.

## Migration sequence

### Phase 1 - theme profile contract and gates

Add the named theme-profile type, document compatibility expectations for
`theme.rs`, and extend architecture tests so new screen code cannot add calls
to deprecated visual helpers. Change `theme_bridge::install_theme` to accept
`ThemeProfile`, update all theme-install call sites, add high-contrast
contrast tests, and ratchet deprecated helper usage. Do not migrate all call
sites in this phase.

### Phase 2 - icon catalog

Add semantic icons for RSS, Nostr, playback transport, download/remove,
playlist add, MusicBrainz, settings, search, and status indicators as call
sites require. Migrate duplicated inline SVG/glyph helpers into the catalog.

### Phase 3 - control styles

Map reusable control style roles onto `ui::primitives::Button`, migrate
`ActionButton`, add pure role mapping tests, and add the `CONTROL-COMPAT`
architecture-test mechanism.

### Phase 3b - screen button style sweep

Migrate repeated screen button patterns: metadata actions, row actions, toolbar
icon buttons, destructive buttons, and pill/toggle-like controls. Task 003b
owns the screen-level chain sweep in `app.rs`, `library.rs`, and `search.rs`,
with any remaining direct `gpui_component::Button` call sites marked using
`CONTROL-COMPAT(reason): ...`. Preserve current behavior and layout.

### Phase 4 - badge and entity-role migration

Replace remaining `theme::badges` and string-keyed entity visual lookups with
typed roles consumed by `TagBadge` or a successor badge primitive/composite.
Metadata provenance/diff display resolves through the same typed visual-role
boundary so color and glyph are not chosen independently in screen code.

### Phase 5 - runtime profile selection

After screens no longer rely on dark-only helpers for reusable visual roles,
persist a theme profile setting and route changes through `theme_bridge` /
`Environment`. Verify high-contrast profile tests before exposing high contrast
in settings, and expose only visually distinct profiles. Expose `System` only
after GPUI window appearance drives the installed profile.

### Phase 6 - retire compatibility shims

Remove or sharply narrow `theme.rs`. ADR 0025 removed it, moved remaining
fixed geometry to `ui::layouts`, and kept bridge-aware compatibility color
roles in `ui::style`. Architecture tests should fail if screens reintroduce
deprecated visual helpers or the old `ui::style::layout` namespace.

### Phase 9 - status role boundary

Move `StatusRole` out of `ui::style` and into typed visual roles so status
color and glyph semantics resolve together. Remove the old
`style::color::status_*` helpers and add an architecture gate against
reintroducing status roles in `ui::style`.

### Phase 10 - provenance helper retirement

Remove the final loose `style::color::diff_*` compatibility helpers and route
the remaining screen call site through `ProvenanceRole`. Tighten the
architecture baseline to zero and prevent `ui::style` from reintroducing diff
role helpers.

## Test strategy

- Keep `cargo test --test architecture_tests` as the main boundary gate.
- Extend architecture tests to reject new screen-level calls to deprecated
  visual helpers after each phase has a replacement.
- Keep existing WCAG contrast matrix tests for dark and light appearances.
- Add contrast tests for any high-contrast theme profiles before exposing them.
- High-contrast profiles must be visually distinct from the base Dark/Light
  palettes, not just aliases that reuse the same contrast matrix.
- Task 001 must add high-contrast profile coverage to the existing contrast
  matrix or an equivalent focused matrix.
- Add focused unit tests for icon role metadata where the icon catalog carries
  brand/status colors or labels.
- Add focused tests for control style role mapping if the implementation is
  pure enough to test without GPUI. The role-to-token mapping should be a pure
  function so this is practical.
- Tasks that add unit tests must run `cargo test` or `cargo test --lib`, not
  only `cargo test --test architecture_tests`.
- Run `cargo fmt -- --check`, `cargo check`,
  `cargo clippy --lib --tests -- -D warnings`, and relevant focused tests
  before accepting implementation commits.

## Green criteria

This ADR is fulfilled when:

- A named theme-profile boundary exists and is used by theme installation.
- `theme_bridge::install_theme` takes `ThemeProfile`, and the old
  appearance-only entry point is gone or private compatibility code.
- New screen code cannot add `theme::color::*`, `theme::badges`, or
  `theme::glyphs` call sites without failing architecture tests.
- `theme::glyphs` is deleted after its Library/Discover provenance and status
  call sites move behind icon/badge/status roles.
- Reusable icons are requested through semantic icon roles.
- RSS/Nostr/playback/download/remove/playlist/MusicBrainz/status icons are no
  longer implemented as screen-level inline SVG or glyph helpers.
- Reusable button/action styling flows through named control styles.
- `ControlStyle` maps to the native `ui::primitives::Button`.
- `ActionButton` is implemented in terms of the shared control-style boundary.
- Reusable screen-level button style chains in `app.rs`, `library.rs`, and
  `search.rs` have been migrated or explicitly documented as compatibility
  exceptions.
- Remaining direct screen-level `gpui_component::Button` compatibility
  exceptions use `CONTROL-COMPAT(reason): ...` and are enforced by
  architecture tests.
- Entity/status badges use typed roles rather than string-keyed color maps.
- Status message color and glyph semantics live in typed UI roles, not
  `ui::style`.
- Light and dark profiles pass the existing contrast tests.
- High-contrast profiles pass contrast tests and are visually distinct from
  base Dark/Light before being exposed as settings.
- Runtime profile changes reinstall the theme and refresh windows without
  screen-specific theme code.
- Layout constants used by screens and composites live in `ui::layouts`, not
  `ui::style::layout`.
- General status message roles live outside `ui::style`, and architecture
  tests prevent reintroducing `StatusRole` or `style::color::status_*`.
- Provenance/diff display has zero `style::color::diff_*` helpers or screen
  call sites; `ProvenanceRole` owns those semantics.
- Phase 6 retirement gate: migrated screen files have zero `theme::color::*`,
  `theme::badges`, and `theme::glyphs` call sites; `theme.rs` has been
  removed; and the architecture gate forbidding those deprecated namespaces is
  unconditional.
- The app preserves current behavior while reducing visual duplication.

## Follow-up work

- Decide whether custom accent color should be user-editable after role-level
  contrast validation exists.
- Decide whether layout shells such as inspector stack and scroll-list should
  join `SplitPane` as reusable layout composites after more screen code moves
  out of `library.rs` and `search.rs`.
- Revisit image/artwork treatment separately; album artwork is content, not a
  theme asset.

## References

- ADR 0023 - Design System and View-Model Architecture.
- ADR 0024 - Command/Query/Event Application Layer.
- `docs/architecture/architecture-diagrams.md` - ideal design-system and
  presentation architecture.
- Apple HIG color guidance: semantic colors, dark mode, contrast, high
  contrast, and not relying on color alone.
- Apple HIG icon/SF Symbols guidance: icons align with text, scale with type,
  use consistent rendering modes, and represent state with more than color.
