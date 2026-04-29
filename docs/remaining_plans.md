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
- `ListRow` now owns selectable/focused/clickable row chrome for dense
  result lists, so screens do not hand-roll row background, focus ring, or
  click affordances.
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
- `view_models::library::{LibraryViewModel, LibrarySnapshot}` owns the
  library screen's pure read snapshots and workflow state: tree, playlists,
  playlist tracks, `mb_status`, staged `MusicBrainz` lookups,
  in-flight feed checks, and feed-update state. `library.rs` now uses VM
  methods for those transitions instead of direct map/vector mutation.
- `view_models::library::LibraryTreeProjection` owns library sidebar tree
  filtering and the "expand all matches while searching" expansion state.
- `view_models::library::PlaylistSidebarVm` owns playlist sidebar header and
  row projection. The playlist sidebar now renders rows through `ListRow` /
  `Label` instead of raw row chrome.
- `LibraryViewModel` now owns selection and album add-to-playlist picker
  transitions through methods, reducing direct screen mutation of those
  fields.
- `LibraryViewModel` also owns status text, busy track, and hovered
  thumbnail transitions through accessors / mutators. It now also owns
  library reload summaries, playlist CRUD failure text, and album-track-load
  empty/error state.
- The migrated `LibraryViewModel` state is private to the VM; `library.rs`
  now reads it through accessors/projections and writes it through typed
  transition methods.
- `PlaylistAppendIntent` / `PlaylistAppendOutcome` are the first small
  command-intent/result values for the library screen. The VM prepares the
  playlist id, track ids, playlist name, progress status, success summary,
  and failure text; the screen still performs the DB/service dispatch.
- `TrackSubscribeOutcome` moves library track subscribe completion state
  into the VM: busy-track clearing plus success/failure status text now live
  outside `library.rs`.

## Remaining Work

### Track E — Finish Screen View-Models

- `library-view-model`: continue thinning `LibraryViewModel` now that
  `LibrarySnapshot` owns the pure snapshot fields and
  `LibraryTreeProjection` / `PlaylistSidebarVm` own sidebar projections.
  Next moves: continue introducing focused command-intent values where they
  remove service-dispatch setup and status formatting from `library.rs`.
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
- `screen-library-playlists`: playlist sidebar rows now use `ListRow` /
  `Label`; keep the `PlaylistDetailVm` path and replace the remaining
  playlist detail row actions with `ActionButton` where it preserves
  behavior.
- `screen-library-metadata`: migrate the ID3 / MusicBrainz metadata panels
  out of inline string formatting and raw color literals.
- `screen-search-results`: Discover result rows now read labels, image URLs,
  visible type filtering, and derived artist rows through
  `view_models::search`, and render through `ListRow` / `Thumbnail` /
  `Label` / `TagBadge`; next move section headers / empty states and
  result-row interaction command intent out of `search.rs`.
- `screen-search-inspector`: migrate track/feed/publisher inspector sections
  to projections and existing composites. Feed / track inspector identity
  labels now render through `TagBadge`; keep direct GPUI event wiring in the
  screen until command intent is available.

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
  badge rather than deriving compatibility color data. `search.rs` now uses
  `TagBadge` for Discover rows and feed / track inspector labels; the
  remaining Search usage is MusicBrainz release-picker button styling.

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
