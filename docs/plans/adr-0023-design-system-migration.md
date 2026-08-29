# Design-system + ideal-architecture migration

## Current Snapshot

Verified 2026-04-30 after ADR 0023 finalization Tasks 006-010. This branch
should not be pushed or turned into a PR without explicit direction.

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
  `.with_size(...)` adapter in app code. Screen call sites use
  `.scaled(Size::*, cx)`.

The primitive/composite layer is also in place:

- Primitives: `Button`, `Surface`, `Label`, `Divider`, `Popover`,
  `MultilineText`, `Image`, `SectionHeader`, `VStack`, `HStack`, `ZStack`,
  and `Spacer`.
- Composites: `Thumbnail`, `TagBadge`, `DetailHeader`, `DetailGrid`,
  `ListRow`, `SegmentedControl`, `DisclosureGroup`, `ActionButton`,
  `SplitPane`, `ReleaseDetailSurface`, and `playlist_popover`.
- `ListRow` now owns selectable/focused/clickable row chrome for dense
  result lists, so screens do not hand-roll row background, focus ring, or
  click affordances.
- `ui_common.rs` has been removed. Its surviving responsibilities live in
  `ui/detail_row.rs`, `ui/composites/*`, and `view_models::format`.
- `theme::badges` remains the legacy badge compatibility surface. Newer
  badge rendering should prefer `TagBadge` / `EntityKind`.

The projection view-model layer is implemented for ADR 0023 scope:

- `view_models::artist::ArtistVm` backs `ui_artist.rs`.
- `view_models::feed::FeedVm` backs `ui_feed.rs`.
- `view_models::track::TrackVm` backs the discover row, several search
  track projections, and library track title default behavior.
- `view_models::search::{ResultRow, ResultRowVm}` backs Discover result-row
  data, display strings, image selection, visible-type filtering, and pure
  derived-artist aggregation.
- `view_models::search::SearchViewModel` now owns Discover/Search pure UI
  scalars and snapshots (`results`, `recent_feeds`, `playlists`). `search.rs`
  is still mid-migration while it owns service dispatch, inspector stack,
  GPUI handles, thumbnails, and layout pixels. Endpoint-reset and recent-feed
  loading transitions now go through VM methods and pure load intents. Main
  search loading/results now use the same VM-owned transition shape. Playlist
  snapshots, playlist append progress/completion, add-to-playlist preflight
  status text, track download/remove in-flight status transitions, and the
  split-pane resize lifecycle are also VM-owned. Result-row key/display title
  projection and keyboard navigation targets now live with the result snapshot
  in the VM layer, as does artist-result enrichment after inspector loads.
  Search and recent-feed render snapshots now group the remaining read-only
  render flags instead of having `search.rs` recompute them field by field.
  playlist popover rendering also reads playlists through a VM snapshot
  accessor. The reusable `LazyPanel<T>` state for deferred inspector panels
  now lives in `view_models::search`, along with the collapsible
  fetch/toggle transition used by contributors and value routes.
  GPUI-bound inspector frames remain in `search.rs`.
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
- The migrated `LibraryViewModel` state is private to the VM. `library.rs`
  now reads it through accessors/projections and writes it through typed
  transition methods.
- `PlaylistAppendIntent` / `PlaylistAppendOutcome` are the first small
  command-intent/result values for the library screen. The VM prepares the
  playlist id, track ids, playlist name, progress status, success summary,
  and failure text. The screen still performs the DB/service dispatch.
- `TrackSubscribeOutcome` moves library track subscribe completion state
  into the VM: busy-track clearing plus success/failure status text now live
  outside `library.rs`.
- `tests/architecture_tests.rs` enforces no GPUI/screen imports under
  `view_models`, no raw screen-level `rgb(...)` or numeric `px(...)` literals,
  and no unapproved hardcoded dark defaults in screen modules.
- Documentation is now organized by purpose: architecture diagrams and app
  overview under `docs/architecture/`, migration plans under `docs/plans/`,
  operator workflows under `docs/runbooks/`, storage notes under
  `docs/schema/`, research under `docs/research/`, task packets under
  `docs/tasks/`, reviews under `docs/reviews/`, and older roadmap notes under
  `docs/archive/`.

## Orchestration Artifacts

- `docs/tasks/adr-0023-task-001-doc-architecture-cleanup.md`
- `docs/tasks/adr-0023-task-002-top-app-token-composite-slice.md`
- `docs/tasks/adr-0023-task-003-library-token-intent-slice.md`
- `docs/tasks/adr-0023-task-004-search-inspector-token-slice.md`
- `docs/tasks/adr-0023-task-005-release-detail-parity.md`
- `docs/tasks/adr-0023-task-006-shared-split-pane-shell.md`
- `docs/tasks/adr-0023-task-007-release-detail-surface.md`
- `docs/tasks/adr-0023-task-008-library-row-semantics.md`
- `docs/tasks/adr-0023-task-009-command-intent-finish.md`
- `docs/tasks/adr-0023-task-010-boundary-gates.md`
- `docs/tasks/adr-0023-task-011-final-review.md`
- `docs/plans/adr-0023-finalization-plan.md`
- `docs/reviews/adr-0023-review-checklist.md`

## Completed And Deferred Work

### Track E — Finish Screen View-Models

- `library-view-model`: ADR 0023 scope complete. `LibraryViewModel` owns
  snapshots, selection, resize state, playlist append intent/results,
  subscribe outcomes, feed-update state, and `MusicBrainz` status transitions.
  Deferred: continue thinning service-dispatch setup from `library.rs` under a
  later command architecture ADR.
- `search-view-model`: ADR 0023 scope complete. `SearchViewModel` owns
  Discover/Search snapshots, selection, resize state, loading transitions,
  playlist append intent/results, track operation status, lazy-panel state, and
  inspector subscription command messages. Deferred: continue moving
  remaining inspector-panel transitions out of `search.rs` under a later ADR.
- `command-intent-types`: complete for ADR 0023. Narrow command/result values
  exist where they materially reduced screen status glue. Do not build a broad
  CommandBus without a separate ADR.
- `boundary-gates`: complete. `tests/architecture_tests.rs` enforces no GPUI
  imports under `view_models`, no screen-level raw `rgb(...)` / numeric
  `px(...)` literals, and no unapproved hardcoded dark defaults in screen
  modules.

Deferred detail:

- `library-view-model`: continue thinning `LibraryViewModel` now that
  `LibrarySnapshot` owns the pure snapshot fields and
  `LibraryTreeProjection` / `PlaylistSidebarVm` own sidebar projections.
  Next moves: continue introducing focused command-intent values where they
  remove service-dispatch setup and status formatting from `library.rs`.
- `search-view-model`: continue thinning `SearchViewModel` now that it owns
  Discover/Search pure UI state and snapshots. Next moves: replace direct
  `search.rs` field mutation with typed methods for remaining status
  formatting and inspector-panel transitions.
- `command-intent-types`: introduce small command enums or structs only where
  they remove direct service calls from screens. Do not build a broad
  CommandBus without a separate ADR.

### Track G — Thin The Screens

- `shared-split-pane-shell`: complete. Discover and Library now use the same
  split-pane shell and resize-state contract.
- `release-detail-surface`: complete. Discover feed detail and Library album
  detail share one structural surface with mode-specific slots.
- `screen-library-album`: move album-detail header rows, duration/downloaded
  counts, button labels, and add-to-playlist panel state into library
  view-model projections. ADR 0023 moved row labels and picker state. Further
  album-detail projection thinning is deferred.
- `library-row-semantics`: remove the redundant per-row `dl'd` marker from
  Library album rows. Membership is already represented by the `Remove`
  action. Aggregate downloaded counts can remain in detail grids until a
  product decision removes them. Completed 2026-04-30.
- `screen-library-playlists`: playlist sidebar rows now use `ListRow` /
  `Label`. Keep the `PlaylistDetailVm` path and replace the remaining
  playlist detail row actions with `ActionButton` where it preserves
  behavior. Deferred.
- `screen-library-metadata`: migrate the ID3 / MusicBrainz metadata panels
  out of inline string formatting and raw color literals. Deferred.
- `screen-search-results`: Discover result rows now read labels, image URLs,
  visible type filtering, and derived artist rows through
  `view_models::search`, and render through `ListRow` / `Thumbnail` /
  `Label` / `TagBadge`. Next move section headers / empty states and
  result-row interaction command intent out of `search.rs`.
- `screen-search-inspector`: migrate track/feed/publisher inspector sections
  to projections and existing composites. Feed / track inspector identity
  labels now render through `TagBadge`. Keep direct GPUI event wiring in the
  screen until command intent is available. Deferred.

### Track D — Token And Literal Audit

- `audit-color-usage`: remove remaining screen-level `rgb(...)` literals in
  `app.rs`, `library.rs`, and `search.rs`. Use semantic tokens or a
  deliberately named compatibility helper in `theme.rs`. Completed
  2026-04-30: screen-level `rgb(...)` literals are gone. ID3 frame colours
  now resolve through named `theme::color` helpers.
- `audit-layout-usage`: reduce raw `px(...)` literals in screens. Preserve
  legitimate fixed geometry such as split-pane clamps and image pixel sizes,
  but document them or route them through `Size` / `Spacing` tokens when they
  are part of the design language. Completed 2026-04-30 for numeric screen
  literals: fixed geometry now uses named `theme::layout` and
  `theme::typography` constants.
- `theme-badge-migration`: replace remaining direct `theme::badges` calls in
  screens with `TagBadge` or `EntityKind` where the UI is rendering an entity
  badge rather than deriving compatibility color data. `search.rs` now uses
  `TagBadge` for Discover rows and feed / track inspector labels. The
  remaining Search usage is MusicBrainz release-picker button styling.
- `release-detail-parity`: align Discover feed detail and Library album detail
  by wiring both through shared header / row composites and fixing
  low-contrast ghost action defaults. Completed 2026-04-30.
- `release-detail-surface`: replace parallel Discover feed and Library album
  page skeletons with the shared `ReleaseDetailSurface` composite. Completed
  2026-04-30.

## Implementation Order

- [x] `docs-cleanup`: land the docs organization and update all stale references
   before implementation so future task packets point at stable paths.
- [x] `top-app-token-composite`: finish the app-level token sweep and bind the
   playback strip to `NowPlayingBar`, because this touches the root UI surface
   and establishes the remaining screen pattern.
- [x] `library-token-intent`: continue the library token audit and introduce
   narrow command-intent/result values only where they remove status-formatting
   or service-setup code from `library.rs`.
- [x] `search-inspector-token`: migrate the highest-count Discover inspector and
   metadata literal sites to semantic tokens/composites, backed by focused
   `SearchViewModel` tests for any moved transition logic.
- [x] `release-detail-parity`: make the same release render with the same
   detail skeleton in Discover and Library by using shared `DetailHeader` and
   `TrackRow` composites.
- [x] `shared-split-pane-shell`: make Discover and Library use the same
   resizable split-pane shell.
- [x] `release-detail-surface`: replace parallel feed/album page skeletons
   with one shared detail-surface contract.
- [x] `library-row-semantics`: remove redundant Library row downloaded labels
   and move remaining row semantics into projections.
- [x] `command-intent-finish`: finish narrow intent/result extraction where it
   materially reduces screen glue.
- [x] `boundary-gates`: add automated architecture tests for ADR 0023 import
   and token-literal boundaries.
- [x] `final-review`: reconcile ADR, plan, task, and review documents after
   implementation.

### Deferred Architecture Work

- A broad CommandBus / QueryService / EventBus architecture remains an ideal
  target in `docs/architecture/architecture-diagrams.md`. It is not part of
  this ADR 0023 cleanup unless a new ADR scopes it.
- Splitting `library.rs` or `search.rs` into directories remains out of
  scope for this plan. Do that only under a separate ADR.
- Service-side formatting in `metadata.rs` must not depend on
  `view_models`. Duplicate duration helpers may remain there until a
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
  `cargo fmt -- --check`. For Rust code changes, run the relevant test slice.
  Broaden to `cargo test` / `cargo clippy -- -D warnings` when shared behavior
  changes.
- No automatic push, PR, or local commit without explicit direction.
