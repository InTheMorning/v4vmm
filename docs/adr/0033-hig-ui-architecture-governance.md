# ADR 0033: HIG UI Architecture Governance

## Status

Implemented - 2026-05-02.

## Context

ADR 0032 fixed the playlist popover regression by moving repeated popover
chrome into a shared composite. The deeper risk remains broader than playlist
menus: screens can still regress by reintroducing backend-shaped data into
shared UI, rebuilding floating panels locally, or copying visual code instead
of extending the design system.

The Apple HIG guidance used for this decision is platform-agnostic in spirit:
interfaces should be consistent, direct, focused, adaptive, and predictable.
For this macOS-style GPUI app, that means:

- popovers are transient, anchored, compact surfaces for a few related tasks;
- buttons communicate role and state through consistent style/content/role;
- layout adapts through reusable spacing, typography, and surface tokens;
- macOS workflows should preserve information density without hiding commands
  behind unnecessary modality.

These principles cannot depend on taste during review. They need mechanical
repo gates that make architectural drift fail early.

## Decision

Codify a strict UI architecture governance boundary, anchored to concrete
directories and enforced by named tests in `tests/architecture_tests.rs`:

- Backend/service/database/API layers own facts, persistence, network work,
  filesystem work, and mutations.
- View models (`src/view_models/`) and shared view facts (`src/views.rs`) own
  GPUI-free presentation contracts, labels, availability, command intents, and
  display-ready facts.
- Tokens (`src/ui/tokens.rs`) and theme bridges (`src/ui/theme_bridge.rs`,
  `src/ui/theme_profiles.rs`) are the only places that may carry raw `rgb(...)`
  values or numeric `px(...)` literals; everything else consumes named
  `SemanticColor`, `Spacing`, `Radius`, `FontSize`, and `Weight` tokens.
- Shared UI primitives (`src/ui/primitives/`) and composites
  (`src/ui/composites/`) own HIG-style chrome: buttons, rows, surfaces,
  popovers, spacing, typography, color roles, and floating-panel mechanics.
  Primitives are the only modules permitted to call raw floating-chrome APIs
  (`gpui_component::popover`, `SurfaceElevation::Floating`, `.absolute()`,
  `.fixed()`, `.z_index(...)`); composites compose primitives; screens compose
  composites and primitives.
- Screen modules own wiring only: event callbacks, command dispatch, image
  resolution, selected entity state, and composition of shared components.
- Shared UI components must accept display-ready data and callbacks, where
  "display-ready" means a type defined in `src/view_models/`, `src/views.rs`,
  or co-located with the component under `src/ui/composites/` or
  `src/ui/primitives/`. Backend row types (`crate::db::*`, `crate::api::*`)
  and service objects are forbidden as shared-UI inputs, including when
  smuggled through callback signatures (e.g. `Fn(db::Track)`).
- Presentation modules must not hand-roll floating chrome. If a screen needs a
  popover, overlay, anchored menu, or floating panel, it must use or extend a
  shared primitive/composite.

### Human-interface structure bar

User-visible UI work is acceptable only when it improves or preserves the
structure that produced the interface. A local patch that merely hides a
visible symptom is rejected unless it is the smallest step toward a stronger
shared component, view-model contract, token role, or regression guard.

Observed defects are diagnostic signals, not structural causes. For example,
a missing `+ New Playlist` command is a symptom that playlist popover
ownership or call-site wiring has drifted; the structural issue is duplicated
popover chrome and incomplete use of the shared composite contract. Tests may
assert the visible affordance only as a canary for that ownership boundary.

Every UI change must name at least one structural contract it strengthens:

- HIG-style hierarchy and disclosure: title, subtitle, metadata, state, and
  actions have predictable placement, weight, and visibility.
- Shared ownership: repeated chrome, popovers, rows, buttons, menus, and
  presentation mechanics move to `src/ui/primitives` or `src/ui/composites`
  before being copied across screens.
- View-model contract: fallback labels, display strings, availability,
  command intents, and derived presentation facts live in GPUI-free
  `src/view_models` or `src/views.rs`.
- Token and component discipline: spacing, sizing, color, typography, icons,
  and action roles use named tokens/components rather than raw literals,
  glyph strings, or ad hoc wrappers.
- Regression guard: the same change adds or strengthens an architecture test,
  unit test, visual smoke, or baseline reduction that blocks the regression
  class.
- Visual proof: layout, hierarchy, and presentation fixes are inspected in
  the running UI or a captured screenshot before being described as fixed.

If a UI change cannot satisfy one of these contracts, it must be reframed as a
structural UI task before implementation.

For the playlist popover family, `AddToPlaylistPopover` (in
`src/ui/composites/playlist_popover.rs`) accepts `PlaylistOption`, a
display-ready type co-located with the composite, instead of `db::Playlist`.
This makes the composite's input explicitly display-ready and lets
architecture tests forbid backend imports in shared UI components.

### Enforcing tests

The following tests in `tests/architecture_tests.rs` are the mechanical gates
behind this ADR. Renaming or removing any of them requires a follow-up ADR
update so the contract and its enforcement do not drift apart:

- `shared_ui_components_do_not_import_backend_or_screen_layers`
- `shared_ui_callbacks_do_not_smuggle_backend_types`
- `presentation_modules_do_not_hand_roll_floating_chrome`
- `top_level_gpui_modules_are_classified_as_screen_or_shared_ui`
- `screens_do_not_reintroduce_raw_color_or_numeric_px_literals`
- `ui_components_do_not_bypass_theme_profile_resolution`
- `ui_style_does_not_reintroduce_layout_namespace`
- `ui_style_does_not_reintroduce_status_roles`
- `ui_style_does_not_reintroduce_provenance_diff_roles`
- `screens_do_not_define_inline_icon_svg_helpers`
- `ui_buttons_do_not_reintroduce_raw_leading_glyphs`
- `screens_do_not_grow_unmarked_direct_component_button_usage`
- `screens_do_not_grow_screen_local_playlist_popover_panels`
- `library_release_detail_playlist_popovers_use_shared_composite`
- `playlist_popover_calls_wire_create_mode`
- `discovery_recent_tiles_use_shared_composite`
- `screens_do_not_inline_unknown_artist_or_album_fallbacks`
- `screens_do_not_inline_untitled_fallbacks`
- `screens_do_not_coerce_empty_feed_url_to_empty_string`
- `composite_loose_string_display_apis_are_allowlisted`
- `screens_do_not_duplicate_render_helpers_without_baseline`
- `screens_do_not_inline_value_route_recipient_label_fallbacks`
- `shared_top_level_ui_shells_do_not_import_screen_modules`

The tests scope themselves by directory (`src/ui/primitives`,
`src/ui/composites`) where possible, so adding a new shared component is
auto-covered. Where a test scopes by file allowlist (`SCREEN_FILES`,
`PRESENTATION_GLUE_FILES` in `tests/architecture_tests.rs`), any new
top-level screen or presentation-glue module must be added to those lists in
the same change that introduces the module; otherwise it is a silent escape
hatch from this ADR.

The forbidden-pattern lists in those tests
(`SHARED_UI_BACKEND_FORBIDDEN_PATTERNS`,
`SCREEN_LOCAL_FLOATING_CHROME_FORBIDDEN_PATTERNS`) are not exhaustive. They
must be extended whenever a new screen-local chrome pattern or backend-leak
pattern is discovered in review.

The render-helper duplication baseline is temporary consolidation debt, not a
permission slip for new copy-paste. Any new duplicate `render_*` helper across
screen files must first become a shared primitive/composite or receive a
follow-up ADR/task note explaining why it cannot yet be consolidated.

## Invariants

- `src/ui/primitives` and `src/ui/composites` stay backend-free and
  screen-free.
- Floating chrome implementation belongs under `src/ui/primitives` or
  `src/ui/composites`, not screen modules. Primitives are the only modules
  permitted to call raw floating-chrome APIs.
- Raw `rgb(...)` values and numeric `px(...)` literals appear only in
  `src/ui/tokens.rs` and theme bridges; everywhere else uses named tokens.
- Screens may pass callbacks and already-prepared display data into shared UI;
  they may not pass service/query objects or backend row types into shared UI,
  including via callback parameter types.
- Display-ready inputs to shared UI come from `src/view_models/`, `src/views.rs`,
  or types co-located with the consuming primitive/composite.
- New repeated UI affordances must become primitives/composites before they
  appear in more than one screen.
- User-visible UI fixes must name the structural contract they improve. One-off
  symptom patches are rejected unless they move the app toward a shared
  primitive/composite, view-model projection, named token/component, or
  regression guard.
- New top-level screen or presentation-glue modules are added to `SCREEN_FILES`
  and `PRESENTATION_GLUE_FILES` in `tests/architecture_tests.rs` in the same
  change that introduces them.
- Visual changes that touch popovers, overlays, action rows, or release detail
  layout require architecture tests and visual smoke.
- A user-visible layout or presentation fix is not complete without visual
  evidence from the affected surface, or an explicit residual-risk note that
  visual verification remains undone.
- Existing ADR 0023, 0025, 0031, and 0032 boundaries remain in force.

## Non-Goals

- No rewrite to SwiftUI/AppKit.
- No schema or service redesign.
- No attempt to make GPUI itself mimic every platform-specific Apple control.
- No mass cleanup of every legacy screen helper in this change.
- No ban on dense desktop workflows; density is acceptable when structure,
  hierarchy, and action roles remain clear.

## Alternatives Considered

- Rely on visual review only. Rejected because the playlist popover regression
  showed that visual regressions can come from architectural drift.
- Let shared UI import `db` rows as convenient data carriers. Rejected because
  it couples chrome to persistence shape and makes UI components harder to
  reuse safely.
- Ban all screen-level layout code. Rejected as too broad; screens still need
  composition glue, but reusable chrome and floating mechanics must live in the
  design system.

## Consequences

- `shared_ui_components_do_not_import_backend_or_screen_layers` fails if any
  file under `src/ui/primitives` or `src/ui/composites` imports backend,
  service, API, database, or screen modules.
- `presentation_modules_do_not_hand_roll_floating_chrome` fails if any file in
  `PRESENTATION_GLUE_FILES` uses raw floating-chrome APIs such as component
  popovers, fixed/absolute overlays, `z_index`, or `SurfaceElevation::Floating`.
- `screens_do_not_reintroduce_raw_color_or_numeric_px_literals` fails if any
  file in `SCREEN_FILES` carries raw `rgb(...)` or numeric `px(...)` literals
  outside the token/theme layer.
- `AddToPlaylistPopover` consumes a display-ready `PlaylistOption` defined in
  `src/ui/composites/playlist_popover.rs`.
- Future HIG-aligned UI work should strengthen these gates by moving repeated
  affordances into shared primitives/composites, extending the forbidden-pattern
  lists when new screen-local chrome shapes are discovered, and lowering any
  remaining compatibility baselines (e.g. `DEPRECATED_VISUAL_HELPER_BASELINES`,
  `DIRECT_COMPONENT_BUTTON_BASELINES`,
  `SCREEN_LOCAL_PLAYLIST_POPOVER_BASELINES`) toward zero.
- Reviews must reject "looks fixed" UI diffs that do not improve hierarchy,
  shared ownership, view-model projection, token/component discipline, or
  regression protection.
