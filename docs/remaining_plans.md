# Design-system + ideal-architecture migration

## Current Snapshot

Branch: `master`, based on local HEAD `f238eae` (`feat(view-models): project
playlist detail and per-track row`) with additional uncommitted refactor work.
This branch is ahead of `origin/master` and should not be pushed or turned
into a PR without explicit direction.

The ADR 0023 design-system foundation is in place:

- `src/ui/tokens.rs` defines `Spacing`, `Radius`, `FontSize`, `Size`,
  `SemanticColor`, `Appearance`, `ScaleFactor`, and `Environment`.
- `theme_bridge::install_theme(appearance, scale, cx)` installs the
  bundled `Environment`, mirrors `ScaleFactor` for legacy callers, and
  refreshes windows so live scale/theme changes repaint.
- Dark and Light palette contrast tests exist in `src/ui/contrast.rs`.
- `Config.ui_scale` is persisted and the Settings scale picker is built on
  `SegmentedControl`.
- `ui::sizable_bridge::SizableScaled` is the only remaining
  `.with_size(...)` adapter in app code; screen call sites use
  `.scaled(Size::*, cx)`.

The primitive/composite layer is also in place:

- Primitives: `Button`, `Surface`, `Label`, `Divider`, `Popover`,
  `MultilineText`, `Image`, `SectionHeader`, `VStack`, `HStack`, `ZStack`,
  and `Spacer`.
- Composites: `Thumbnail`, `TagBadge`, `DetailHeader`, `DetailGrid`,
  `ListRow`, `SegmentedControl`, `DisclosureGroup`, `ActionButton`, and
  `playlist_popover`.
- `ui_common.rs` has been removed. Its surviving responsibilities live in
  `ui/detail_row.rs`, `ui/composites/*`, and `view_models::format`.
- `theme::badges` remains the legacy badge compatibility surface; newer
  badge rendering should prefer `TagBadge` / `EntityKind`.

The projection view-model layer is partially migrated:

- `view_models::artist::ArtistVm` backs `ui_artist.rs`.
- `view_models::feed::FeedVm` backs `ui_feed.rs`.
- `view_models::track::TrackVm` backs the discover row, several search
  track projections, and library track title fallback.
- `view_models::search::{ResultRow, ResultRowVm}` backs Discover result-row
  data, display strings, image selection, visible-type filtering, and pure
  derived-artist aggregation.
- `view_models::library::MbTrackStatus` is now screen-independent.
- `view_models::library::LibraryTrackRowVm` backs album detail track rows.
- `view_models::library::LibraryArtistDetailVm` backs library artist detail.
- `view_models::library::PlaylistDetailVm` and `PlaylistTrackRowVm` back
  playlist detail.

## Remaining Work

### Track E — Finish Screen View-Models

- `library-view-model`: create a stateful `LibraryViewModel` that owns the
  library screen's read snapshot, selection, filters, expanded sections,
  playlist detail state, and `BTreeMap<i64, MbTrackStatus>`. Keep service
  mutation out of accessors; expose typed command intent for the screen to
  dispatch.
- `search-view-model`: create `SearchViewModel` for discover/search state,
  result grouping, inspector frame state, recent-feed state, and display-ready
  result rows. `ResultRow` / `ResultRowVm` now own the former `result_lines` /
  `result_image_url` projection, visible type filtering, and pure derived
  artist aggregation; continue with track/feed headers, inspector sections,
  and async command dispatch.
- `command-intent-types`: introduce small command enums or structs only where
  they remove direct service calls from screens. Do not build a broad
  CommandBus without a separate ADR.

### Track G — Thin The Screens

- `screen-library-album`: move album-detail header rows, duration/downloaded
  counts, button labels, and add-to-playlist panel state into library
  view-model projections.
- `screen-library-playlists`: keep the `PlaylistDetailVm` path, then replace
  the remaining raw playlist row layout with `ListRow` / `ActionButton`
  where it preserves behavior.
- `screen-library-metadata`: migrate the ID3 / MusicBrainz metadata panels
  out of inline string formatting and raw color literals.
- `screen-search-results`: Discover result rows now read labels, image URLs,
  visible type filtering, and derived artist rows through
  `view_models::search`; next move section headers / empty states and
  result-row interaction command intent out of `search.rs`.
- `screen-search-inspector`: migrate track/feed/publisher inspector sections
  to projections and existing composites; keep direct GPUI event wiring in
  the screen until command intent is available.

### Track D — Token And Literal Audit

- `audit-color-usage`: remove remaining screen-level `rgb(...)` literals in
  `app.rs`, `library.rs`, and `search.rs`. Use semantic tokens or a
  deliberately named compatibility helper in `theme.rs`.
- `audit-layout-usage`: reduce raw `px(...)` literals in screens. Preserve
  legitimate fixed geometry such as split-pane clamps and image pixel sizes,
  but document them or route them through `Size` / `Spacing` tokens when they
  are part of the design language.
- `theme-badge-migration`: replace remaining direct `theme::badges` calls in
  screens with `TagBadge` or `EntityKind` where the UI is rendering an entity
  badge rather than deriving compatibility color data.

### Deferred Architecture Work

- A broad CommandBus / QueryService / EventBus architecture remains an ideal
  target in `docs/architecture-diagrams.md`; it is not part of this ADR 0023
  cleanup unless a new ADR scopes it.
- Splitting `library.rs` or `search.rs` into directories remains out of
  scope for this plan. Do that only under a separate ADR.
- Service-side formatting in `metadata.rs` must not depend on
  `view_models`; duplicate duration helpers may remain there until a
  domain-safe formatting module exists.

## Conventions In Force

- Follow ADR 0023 layer rules: no GPUI imports under `view_models/`, no
  service calls from primitives/composites, and no screen imports from
  `view_models/`.
- Projection constructors and accessors are pure and unit-testable without
  `Window` / `App`.
- New view-model code should borrow screen-owned data when practical and own
  state only for stateful screen VMs.
- New public display contracts need focused unit tests in the same module.
- Verify documentation-only changes with `cargo check` and
  `cargo fmt -- --check`. For Rust code changes, run the relevant test slice;
  broaden to `cargo test` / `cargo clippy -- -D warnings` when shared behavior
  changes.
- No automatic push, PR, or local commit without explicit direction.
