# ADR 0023: Design System and View-Model Architecture

## Status

Accepted — design-system foundation and stateful screen VMs implemented for
both library and discover screens; token-literal sweep
(`audit-token-usage`) and a typed command-bus seam remain.

## Context

ADR 0015 established that workflow behavior belongs in non-UI service modules,
and ADR 0022 specifies how to extract the remaining domain logic out of
`library.rs` and `search.rs`. Those ADRs cover the *south side* of the GPUI
boundary. They do not address the *north side* — how the UI itself is
structured.

Before the work documented here, the UI had four problems:

1. **No design tokens.** Colors, spacing, radii, font sizes, and font weights
   were inlined as `rgb(0xffffff)`, `px(12.0)`, `px(124.0)`, etc., scattered
   across `ui_artist.rs`, `ui_feed.rs`, `ui_track.rs`, `ui_common.rs`,
   `library.rs`, `search.rs`, and `app.rs`. There was no way to change the
   visual language coherently. There was also no contrast verification: the
   light theme had no proof of WCAG compliance.
2. **No primitive layer.** Buttons, labels, popovers, dividers, and stack
   layouts were rebuilt with raw `div()` chains at every call site.
   `gpui_component` widgets were used directly with hardcoded
   `Size::Medium` / `Size::Large` choices that ignored user scaling
   preferences.
3. **No composite layer.** Repeated structures (track rows, detail headers,
   metadata grids, thumbnails, tag badges, segmented pickers) were
   reimplemented inline four to six times across the discover and library
   screens. `ui_common.rs` had become a 242-line dumping ground for shared
   render helpers, none of which composed cleanly.
4. **No view-model layer.** Display-ready strings (track titles with
   fallbacks, `M:SS` runtime, formatted dates, MusicBrainz status text,
   subtitle composition, detail-row entries) were built inside the render
   functions. The view code held both the projection logic and the GPUI
   element tree, and the projections were not unit-testable without a
   `Window` and an `App` cx.

The architecture diagrams in `docs/architecture-diagrams.md` describe the
target shape: a layered UI built from tokens → primitives → composites →
screens, with a view-model layer between domain services and screens. In the
ideal architecture, view-models hold read snapshots, local UI state, and
service-command intent while remaining GPUI-free.

PR #5 (`feat/design-tokens-and-primitives`, merged at f2548a0) implemented
the design-system foundation and the first projection-style view-models.
Follow-up local commits on `master` extended that work: `ui_common.rs` is
gone, `DisclosureGroup` and `ActionButton` shipped, `Environment` is the
default appearance/scale accessor for primitives and composites, and
`view_models::library` now owns the `MusicBrainz` row status type plus
artist-detail and playlist-detail projections.

The ideal architecture is still not complete. `library.rs` and `search.rs`
continue to own most screen state and still contain direct service dispatch,
raw layout literals, and several color literals. `search.rs` has no dedicated
screen view-model yet. The remaining migration is tracked in
`docs/remaining_plans.md`.

This ADR records the decision behind that work and the rules that govern
each layer going forward.

## Decision

Adopt a five-layer UI architecture, with strict import-direction rules and
a SwiftUI-inspired API at each layer.

```
screens/                        GPUI Render impls and event adapters
   ├─ bind to view_models/       GPUI-free snapshots, projections, commands
   │      ▲ read / dispatch through
   │      db / *_service / api   domain and service layer — no GPUI
   │
   └─ compose ui/composites/     multi-element components — local widget state
             ▲ compose
             ui/primitives/      single-purpose elements — token-driven
             ▲ read
             ui/tokens.rs + ui/theme.rs
                                  semantic tokens — single source of truth
             ▲ resolve
             gpui / gpui_component
```

### Layer 1 — Tokens (`src/ui/tokens.rs`)

Tokens are the single source of truth for visual values. The token surface is:

- `Spacing` — 4pt-grid scale.
- `Radius` — `SM`, `MD`, `LG`, `Full`.
- `FontSize` — semantic sizes.
- `Size` — height/width scale for primitives that take a size enum.
- `SemanticColor` — palette aliases (`text_primary`, `text_secondary`,
  `surface_raised`, `border_subtle`, etc.) resolved per appearance.
- `Appearance` — `Light` / `Dark`.
- `ScaleFactor` — runtime user preference, persisted in `Config.ui_scale`.
- `Environment` — SwiftUI-style typed bundle of `appearance + scale`,
  passed implicitly through render via a `gpui::Global`.

Each dimension exposes both `const fn px()` for compile-time use inside
primitives and `.scaled(cx) -> Pixels` for runtime scaling at the screen
level.

A WCAG contrast matrix (`src/ui/contrast.rs`) gates the dark and light
palettes. `dark_palette_meets_wcag` and `light_palette_meets_wcag` tests
must pass; new semantic colors must declare their intended foreground/
background pairings and clear AA contrast at minimum.

### Layer 2 — Primitives (`src/ui/primitives/`)

Primitives are stateless, token-driven elements that own no domain data.
They expose a SwiftUI-style modifier API where each modifier returns
`Self`, and they read `Environment` (appearance, scale) from the cx by
default rather than taking explicit `Appearance::Dark` parameters.

Shipped primitives:

- `Button`
- `Surface`
- `Label` — chainable `.weight()`, `.size()`, `.truncated()`.
- `Divider`
- `Popover` — HIG-compliant arrow droplet, dismissal, focus trap.
- `MultilineText` — SwiftUI-style `Text(...).lineLimit(n)` shape.
- `Image` — replaces ad-hoc `artwork_img` helpers.
- `SectionHeader`
- `VStack`, `HStack`, `ZStack`, `Spacer` — SwiftUI-style spacing
  containers.

A primitive must not import `library`, `search`, `view_models`, or any
service module. Its only dependencies are `gpui`, `gpui_component`,
`tokens`, and `theme`.

### Layer 3 — Composites (`src/ui/composites/`)

Composites combine primitives into multi-element components. They may own
local widget state (popover open/closed, hover, segmented selection) but
must not own domain state and must not call services. They may take a
view-model or view-model projection by reference for display data.

Shipped composites:

- `Thumbnail`
- `TagBadge`
- `DetailHeader`
- `DetailGrid`
- `ListRow` — token-driven selectable/focused/clickable row chrome.
- `SegmentedControl`
- `DisclosureGroup`
- `ActionButton`
- `PlaylistPopover` (the original composite, predates this ADR)

Composites replace the equivalents in the now-removed `ui_common.rs`.
Layer rule: a composite may import primitives, tokens, theme, and (for
typed projection) view-models. It may not import screen modules or
service modules.

### Layer 4 — View-Models (`src/view_models/`)

A view-model is the GPUI-free adapter between domain/service state and a
screen. ADR 0023 permits two shapes, because the migration is incremental:

1. **Projection VMs** — borrow-only structs constructed during render to
   format already-loaded data into display-ready strings and semantic
   buckets. This is the shape used by `ArtistVm`, `FeedVm`, `TrackVm`,
   `LibraryTrackRowVm`, `LibraryArtistDetailVm`, and `PlaylistDetailVm`
   today.
2. **Screen VMs** — stateful models that own read snapshots, local UI state
   (selection, filters, what is expanded), and command intent for a screen.
   This is the target shape described in `docs/architecture-diagrams.md` and
   `docs/remaining_plans.md` for `LibraryViewModel` and `SearchViewModel`.

Rules — enforced by the module-level documentation in `view_models/mod.rs`
and by code review:

1. **No GPUI imports.** No `gpui::*`, no `gpui_component::*`. A VM that
   reaches for `SharedString`, `AnyElement`, `Window`, or `App` is wrong;
   expose plain Rust data and let the screen wrap it.
2. **No direct service mutation inside projection accessors.** Projection
   construction and accessors are pure. Screen VMs may expose typed command
   values or methods that return command intent; the screen or command
   adapter dispatches that intent against `*_service` modules.
3. **Every public projection is unit-testable without GPUI.** If a method
   needs a `cx`, it does not belong here.
4. **Borrow where practical, own where necessary.** Projection VMs should
   hold short-lived borrows of screen-owned data. Screen VMs may own
   snapshots and UI state so the GPUI screen can become a thin renderer.
5. **One module per screen.** Shared formatting helpers live in
   `view_models::format` or a future common module.
6. **No screen imports.** View-model modules may not import `library`,
   `search`, `app`, `ui_*`, or `ui`. Any screen-owned state needed by a VM
   must move to a domain-safe module or be represented by a VM-local type.

Shipped view-models:

- `view_models::format` — `fmt_runtime`, `fmt_date`.
- `view_models::artist::ArtistVm` — title / subtitle / track-count
  label / detail rows.
- `view_models::feed::FeedVm` — title / artist label / publisher text /
  sorted tracks / runtime / detail entries.
- `view_models::track::TrackVm` — title with fallback, runtime suffix,
  composed labels, identity helpers.
- `view_models::search::{ResultRow, ResultRowVm}` — Discover result-row
  data and projection (visible-type filtering, derived artist aggregation,
  three-line display text, and image URL selection).
- `view_models::search::SearchViewModel` — stateful Discover/Search
  screen VM owning pure UI scalars and snapshots (`results`,
  `recent_feeds`, `playlists`), while `search.rs` still owns GPUI-bound
  handles, service dispatch, and several direct VM field transitions during
  the migration. Endpoint-reset and recent-feed loading transitions now
  route through VM methods and pure command intent; main search
  loading/result application and playlist append state now follow the same
  pattern.
- `view_models::library::{LibraryViewModel, LibrarySnapshot}` — stateful
  library screen VM owning pure read snapshots (`tree`, playlists, playlist
  tracks), staged `MusicBrainz` lookups, per-track MB status, and feed-update
  workflow state.
- `view_models::library::LibraryTreeProjection` — filtered tree plus render
  expansion state for the library sidebar.
- `view_models::library::PlaylistSidebarVm` — playlist sidebar header and
  row projection, including disclosure state, sort label, create-input
  visibility, row labels, and selected row state.
- `view_models::library::LibraryViewModel` selection / picker methods —
  typed state transitions for library item selection, playlist selection,
  playlist creation visibility, and album add-to-playlist pickers.
- `view_models::library::LibraryViewModel` operation-state methods —
  typed transitions for status text, busy track, hovered thumbnail URL,
  library reload summaries, playlist CRUD failures, and album-track-load
  empty/error states.
- `view_models::library::{PlaylistAppendIntent, PlaylistAppendOutcome}` —
  pure command intent and result-count data for playlist append operations;
  `LibraryViewModel` prepares the intent and owns the progress, success,
  and failure status text while `library.rs` dispatches the service call.
- `view_models::library::TrackSubscribeOutcome` — pure result data for
  library track subscription completion; `LibraryViewModel` clears busy
  state and owns the success / failure status text.
- `view_models::library::MbTrackStatus` — screen-independent
  `MusicBrainz` lookup state used by the library screen.
- `view_models::library::LibraryTrackRowVm` — album-detail row
  projection (number prefix, title fallback, `M:SS` suffix, MB status
  semantic-kind bucket).
- `view_models::library::LibraryArtistDetailVm` — artist detail
  projection (artist fallback, album / track / downloaded counts, feed
  summaries).
- `view_models::library::PlaylistDetailVm` and
  `PlaylistTrackRowVm` — playlist detail projection (duration roll-up,
  empty-state text, per-row labels, thumbnails, play/move affordance
  enablement).

### Layer 5 — Screens

Screens compose composites and primitives, bind them to view-models, and
forward user interactions as command intent. In the final shape, screens do
not own workflow state, do not call services directly, and do not build
display strings inline.

After PR #5 and follow-up local commits, `ui_artist.rs`, `ui_feed.rs`, and
the discover row of `ui_track.rs` are bound to projection VMs.
`library.rs` uses `LibraryTrackRowVm`, `LibraryArtistDetailVm`,
`LibraryAlbumDetailVm`, `PlaylistDetailVm`, `PlaylistTrackRowVm`, and
`TrackVm` across its detail panels, and routes its pure library
snapshots through `LibraryViewModel` / `LibrarySnapshot` methods
instead of mutating those maps and vectors directly. Its library-tree
filtering / expansion projection lives in `LibraryTreeProjection`, and
its playlist sidebar rows render from `PlaylistSidebarVm` through
`ListRow` / `Label`. Library selection, album add-to-playlist picker
toggles, status / busy-track / hovered-thumbnail updates, and command
intent for playlist append / track subscribe are all mediated by
`LibraryViewModel` methods and value types
(`PlaylistAppendIntent`, `PlaylistAppendOutcome`,
`TrackSubscribeOutcome`) rather than direct screen mutation.

`search.rs` mirrors this shape via `SearchViewModel`. The screen owns
only GPUI-bound fields — `Entity<InputState>`, `gpui::Subscription`,
`FocusHandle`, `Pixels`, the inspector stack with `Arc<Image>`, the
thumbnails map, and service handles — while `SearchViewModel` owns
selection (`selected_key`, `inspector_origin`), filter state
(`type_filter`, `fuzzy_search`), both panes' loading / status /
cursor / `has_more` flags, track download/remove in-flight
transitions, drag-resize lifecycle, result-row identity / keyboard
navigation targets, render snapshots, artist-result enrichment, and the
loaded snapshots (`results`, `recent_feeds`, `playlists`). The reusable
`LazyPanel<T>` inspector panel state also lives in the view-model layer,
while GPUI-bound inspector frames remain screen-owned. Discover result rows render through `ListRow`,
`Thumbnail`, `Label`, and `TagBadge` instead of raw row / badge
layout. Inspector projection logic lives in `PublisherInspectorVm`,
`ActionRowVm`, `TrackInspectorHeaderVm`, `ContributorVm`, and
`PaymentRouteVm`; the screen consumes them and only owns the GPUI
element tree and event wiring.

`library.rs` and `search.rs` remain large. They still call services
directly (no command bus seam yet) and contain raw `px(...)` and
`rgb(...)` literals. The audit sweep is tracked separately as
`audit-token-usage` in `docs/remaining_plans.md`.

### Cross-cutting bridges

- `ui::theme_bridge::install_theme(appearance, scale, cx)` — installs
  `Environment` as a `gpui::Global` and triggers `cx.refresh_windows()`
  so live theme/scale changes paint immediately.
- `ui::sizable_bridge::SizableScaled` — discrete step shift from
  `ScaleFactor` into `gpui_component::Size`. All ~40 `.with_size(Size::*)`
  call sites in `library.rs`, `search.rs`, and `app.rs` migrated to
  `.scaled(Size::*, cx)` so the user's UI scale picker actually moves
  third-party widgets.
- `theme::badges` — legacy compatibility surface for string-keyed badge
  styling. New entity badges use `TagBadge` / `EntityKind`; the previous
  `ui_common::{type_color, badge_text, type_emoji}` wrappers are gone.

## Consequences

### Positive

- A single source of truth exists for color, spacing, radius, typography,
  and scale. Theme changes propagate by editing tokens; no screen-level
  literal sweeps required.
- Light theme is now first-class. WCAG matrix tests prevent silent
  contrast regressions on either palette.
- Runtime UI scale works end-to-end. The settings picker drives both
  primitive sizing and `gpui_component` widget sizing, and live changes
  repaint immediately via `theme_bridge`.
- Composites eliminate the four-to-six-fold inline duplication of detail
  headers, metadata grids, thumbnails, and badges. Bug fixes apply once.
- View-models make display-ready strings unit-testable without GPUI
  scaffolding. The `ArtistVm` test pattern is reproducible across the
  remaining screens.
- The smaller screen helpers are meaningfully thinner. `library.rs` and
  `search.rs` still carry hardcoded literals and inline state/service glue,
  but the remaining work is now described as a screen-VM migration rather
  than a design-system invention task.
- The SwiftUI-shaped API (`Label` modifiers, `VStack`/`HStack`,
  `Environment`, `DisclosureGroup`) reduces cognitive load for callers
  and gives migration of further screens a clear template.

### Negative

- The UI now spans three new directory trees (`ui/primitives/`,
  `ui/composites/`, `view_models/`) plus the `tokens` / `theme` /
  `theme_bridge` / `sizable_bridge` files. New contributors have more
  surface to learn before adding a screen.
- Some source-of-truth pairs still exist during migration. `fmt_dur` lives
  both in `metadata.rs` and in `view_models::track`; related title fallback
  behavior exists in multiple projections. They should collapse where the
  target layer dependency allows it. Service-side formatting must not depend
  on `view_models`.
- Both `library.rs` and `search.rs` still dispatch service calls
  directly. `LibraryViewModel` and `SearchViewModel` own the pure
  snapshot data and pane state, Search also owns track-operation
  in-flight/status transitions, and library has typed command-intent
  values (`PlaylistAppendIntent`, `TrackSubscribeOutcome`,
  `PlaylistAppendOutcome`), but a full command-bus seam — typed
  commands replacing every direct `&mut self` service call — is still
  future work and will likely need its own ADR.
- Several `gpui_component` widgets do not expose appearance through
  `Environment` and require explicit `.appearance()` modifier calls.
  This is acceptable but inconsistent with the rest of the primitive
  layer.
- Some legacy screen code still installs or assumes dark appearance at the
  app boundary. Primitive and composite render paths now resolve default
  appearance through `Appearance::current(cx)`.

### Neutral

- No schema changes. No service-layer changes. No on-disk format changes.
- No public CLI surface changes.
- This ADR is independent of ADR 0022 and does not block it. The two
  ADRs operate on opposite sides of the GPUI boundary and can be
  executed in any order.

## Layer Rules (enforcement)

The following import rules are normative. Violations should be caught
in code review.

| Layer | May import | Must not import |
|---|---|---|
| `ui/tokens.rs` | `gpui` (for `Pixels`, `Hsla`) | services, view_models, screens |
| `ui/theme.rs` | `tokens` | services, view_models, screens |
| `ui/primitives/*` | `gpui`, `gpui_component`, `tokens`, `theme` | services, view_models, screens, composites |
| `ui/composites/*` | primitives, `tokens`, `theme`, `view_models` (for typed projections) | services, screens |
| `view_models/*` | `db`, `api`, `metadata`, `track_compare`, `views`, command-intent types, `std` | `gpui`, `gpui_component`, screens, `ui` |
| screens | everything below | other screens (where avoidable) |

A composite needing service data must take a view-model by reference,
not call the service itself.

A view-model needing display formatting must do it with `String` /
`Cow<'static, str>`, not `SharedString`.

A primitive needing a color must take a `SemanticColor` from tokens, not
an `Hsla` literal.

## Out of Scope

- Visual redesign of any screen beyond what falls out of correct
  primitive use.
- Light-theme color palette rebalancing beyond keeping the WCAG matrix
  passing.
- New features in services, db, or playback.
- Splitting `library.rs` or `search.rs` into multiple files. That is a
  separate ADR if it happens at all.
- Replacing `gpui_component`. The `sizable_bridge` is the agreed seam.

## Green Criteria

This ADR is fulfilled when the following are true. Some of these are
already true at merge of PR #5; others are explicitly tracked in
`docs/remaining_plans.md`.

- [ ] `tokens.rs` / `theme.rs` / `theme_bridge.rs` are the only places with
      raw color construction. Current screen literals are removed or routed
      through semantic tokens.
- [x] WCAG matrix tests pass for both `Appearance::Dark` and
      `Appearance::Light`.
- [x] Primitives, composites, and view-models exist as separate
      directories with the rules in `view_models/mod.rs` documented.
- [x] `ui_common.rs` is removed; its responsibilities split between
      `ui::detail_row`, `ui::composites::*`, and `view_models::format`.
- [x] Projection VMs exist with unit tests that pin display invariants:
      `ArtistVm`, `FeedVm`, `TrackVm`,
      `ResultRowVm`, `PublisherInspectorVm`, `ActionRowVm`,
      `ContributorVm`, `PaymentRouteVm`, `TrackInspectorHeaderVm`,
      `LibraryTrackRowVm`, `LibraryArtistDetailVm`,
      `LibraryAlbumDetailVm`, `PlaylistDetailVm`, `PlaylistTrackRowVm`,
      `PlaylistSidebarVm`, `ArtistFeedSummaryVm`,
      `LibraryTreeProjection`.
- [x] User-driven `ScaleFactor` flows through both primitives and
      `gpui_component` widgets.
- [x] `view-model-library` and `view-model-search` extracted; library
      and search screens bound to stateful `LibraryViewModel` /
      `SearchViewModel` for selection, snapshot ownership,
      pane state, and command-intent values
      (`PlaylistAppendIntent`, `TrackSubscribeOutcome`,
      `PlaylistAppendOutcome`).
- [x] `view_models/*` has no imports from screen modules, including
      `library`, `search`, `app`, and `ui_*`.
- [x] No primitive/composite render defaults hardcode `Appearance::Dark`;
      they read from `Appearance::current(cx)` / `Environment`.
- [ ] Final audit (`audit-token-usage`): zero `rgb()` / `px(<number>)`
      literals in screen modules outside `tokens.rs`, `theme.rs`,
      primitives, and composites.

## References

- ADR 0013 — Shared Discover Track Row Module (predecessor; first shared
  render helper).
- ADR 0015 — Non-UI Service Boundaries (south-side companion).
- ADR 0022 — UI-Agnostic Core Extraction (south-side companion).
- `docs/architecture-diagrams.md` — current and target architecture
  diagrams.
- `docs/remaining_plans.md` — outstanding migration work (Tracks E, G, D).
- PR #5 (commit f2548a0) — implementation.
