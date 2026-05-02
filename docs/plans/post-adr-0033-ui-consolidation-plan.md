# Post-ADR 0033 UI Consolidation Plan

## Status

Proposed - 2026-05-01.

## Goal

Take the clean ADR 0033 boundary (29 architecture tests passing, every
compatibility baseline at zero) and turn it into a clean **shape**: one
implementation per UI affordance, owned by the design-system layer
(`src/ui/composites/`, `src/ui/primitives/`), parameterized by view-model
output (`src/view_models/`), with a regression gate that prevents new
duplication from landing silently.

## Non-Goals

- No backend, schema, or service changes.
- No new HIG controls or visual redesign; consolidation only.
- No changes to the ADR 0031 release-detail contract work; those tasks
  proceed independently.
- No mass relocation of top-level UI shells (`src/ui_artist.rs`,
  `src/ui_entity.rs`, `src/ui_feed.rs`, `src/ui_track.rs`); see "Stretch
  scope" below.

## Compliance Survey (current state)

Verified by reading `tests/architecture_tests.rs` and grepping the screens.

### Clean

- **Backend boundary**: no `db::*` / `api::*` / `*_service` imports in
  `src/ui/primitives/` or `src/ui/composites/`. Test
  `shared_ui_components_do_not_import_backend_or_screen_layers` passes.
- **Callback hygiene**: no `Fn(db::...)` or `Fn(api::...)` callback
  signatures in shared UI. Test
  `shared_ui_callbacks_do_not_smuggle_backend_types` passes.
- **Floating chrome**: no hand-rolled popovers, overlays, `.absolute()`,
  `.fixed()`, `.z_index(...)`, or `SurfaceElevation::Floating` outside
  `src/ui/primitives/`. Test
  `presentation_modules_do_not_hand_roll_floating_chrome` passes.
- **Token discipline**: no raw `rgb(...)` or numeric `px(...)` literals in
  `SCREEN_FILES`. Test
  `screens_do_not_reintroduce_raw_color_or_numeric_px_literals` passes.
- **Compatibility baselines**: every entry in
  `DEPRECATED_VISUAL_HELPER_BASELINES`, `DIRECT_COMPONENT_BUTTON_BASELINES`,
  `PROVENANCE_DIFF_HELPER_BASELINES`, and
  `SCREEN_LOCAL_PLAYLIST_POPOVER_BASELINES` is at `max_count: 0` and
  satisfied. No legacy debt to retire.
- **Top-level module classification**: every `gpui`-importing top-level file
  appears in `SCREEN_FILES`, `PRESENTATION_GLUE_FILES`, or
  `KNOWN_SHARED_UI_SHELL_FILES`. Test
  `top_level_gpui_modules_are_classified_as_screen_or_shared_ui` passes.

### Not yet clean

- **Cross-screen render-helper duplication**: nine `render_*` helpers exist
  in both `src/library.rs` (4,172 lines) and `src/search.rs` (6,894 lines)
  with material divergences (different empty-state behavior, different
  spacing, different callback shapes). The MusicBrainz-panel family alone is
  four duplicated helpers carrying ~200 lines of forked layout.
- **No regression gate against duplication**: if a contributor copies a
  render helper from one screen to the other, no test fails today.
- **Two small screen-local presentation decisions**: `render_track_header`
  title coercion (`if frame.title.is_empty() { TrackVm::title() }`) and
  empty-name fallbacks at `library.rs:3923` / `search.rs:4071`.

## Assumptions

- ADR 0033 invariants stay in force. This plan strengthens them, not
  replaces them.
- View-model rules from `src/view_models/mod.rs` apply: no GPUI imports, no
  `SharedString` or `AnyElement`, plain `String` only, screens wrap into
  `SharedString` at render time.
- Shared cross-screen view-models (used by both Library and Search
  inspectors) are admissible as their own modules under `src/view_models/`,
  matching the precedent of `view_models/entity_detail.rs`.
- Screen-owned image resolution stays out of view-models; composites take
  pre-resolved `Option<Arc<Image>>` (or equivalent) as a separate parameter
  alongside the VM.

## Affected Modules

- `src/library.rs`, `src/search.rs` — primary cleanup targets.
- `src/view_models/` — new shared VMs for MusicBrainz panel, action row,
  track metadata grid, file header.
- `src/ui/composites/` — new composites for action row, track metadata grid,
  file header, track header, MusicBrainz panel.
- `src/ui/primitives/` — new `loading.rs` primitive.
- `src/view_models/track.rs` — new `display_title(override: Option<&str>)`
  method.
- `tests/architecture_tests.rs` — new no-duplication test plus baseline
  constant.
- `docs/adr/0033-hig-ui-architecture-governance.md` — extend "Enforcing
  tests" list when the new test lands.

## Affected Pairs (Workstream A targets)

All pairs verified by grep.

| Pair                          | library.rs | search.rs | Target |
|-------------------------------|-----------:|----------:|--------|
| `render_loading`              | 4090       | 5264      | Primitive `src/ui/primitives/loading.rs`. Takes message string only. |
| `render_action_row`           | 3026       | 2699      | Composite `src/ui/composites/action_row.rs`, driven by view-model action lists. |
| `render_track_metadata_grid`  | 3416       | 3305      | Composite fed by `TrackMetadataGridVm`. |
| `render_file_header`          | 3174       | 3796      | Composite fed by `FileHeaderVm` projected from `TagCompareResult`. |
| `render_track_header`         | 3009       | 4967      | Move to shared UI; fold title coercion into `TrackVm::display_title`. |
| `render_musicbrainz_panel`    | 3247       | 3116      | Composite `src/ui/composites/musicbrainz_panel.rs` taking `MusicBrainzPanelVm` (subsumes the next three). |
| `render_musicbrainz_lookup`   | 3256       | 3125      | (Inside the MusicBrainz composite.) |
| `render_musicbrainz_header`   | 3273       | 3143      | (Inside the MusicBrainz composite.) |
| `render_musicbrainz_title_bar`| 3319       | 3189      | (Inside the MusicBrainz composite.) |

### Reconciling the MusicBrainz divergences

The two MusicBrainz implementations are forked code with deliberate
differences. The consolidated composite picks the union behavior because
"Library and Discovery share the same skeleton" (ADR 0033 invariant):

| Aspect                       | library.rs | search.rs | Canonical |
|------------------------------|------------|-----------|-----------|
| Empty-state                  | muted line only | title-bar + muted line | search (more informative) |
| Title bar `selected` param   | non-`Option` | `Option<&MusicBrainzCandidate>` | search (superset) |
| Title bar disabled-when-empty| no fallback | `disabled(true)` | search |
| Trigger `px` spacing         | `XS` | `SM` | `SM` (matches design-system mid-tier default) |
| Trigger `mb` spacing         | `XS` | `SM` | `SM` |
| Selection callback           | typed `Context<LibraryApp>` | typed `Context<SearchApp>` | screen-agnostic `Fn(usize, &mut Window, &mut App) + 'static`, each screen wraps its own typed `select_musicbrainz_candidate` |

## Workstream A: Consolidate the duplicated screen render helpers

### Pattern (applies to every pair)

1. Read both implementations side by side; note any genuine surface-specific
   logic (action availability, click dispatch). Surface-specific logic stays
   in the screen as a callback or projection input.
2. Add the view-model that decides display content under `src/view_models/`.
   Plain `String` only, GPUI-free, unit-tested.
3. Add the composite under `src/ui/composites/` (or primitive under
   `src/ui/primitives/`) that takes the view-model and the surface-specific
   callbacks. Reuses existing primitives for layout.
4. Replace both screen functions with calls into the composite. Delete the
   originals.
5. Re-export the composite from `src/ui/composites/mod.rs` (or
   `src/ui/primitives/mod.rs`).

### Recommended sequence (smallest first to validate the pattern)

1. **Task #6**: `render_loading` → primitive. ~20 lines, no VM needed. Pure
   pattern validator.
2. **Task #7**: no-duplication architecture test. Catches any regression
   from this point onward.
3. **Task #4**: `render_file_header` → composite + `FileHeaderVm`.
4. **Task #5**: `render_track_header` → composite + `TrackVm::display_title`
   (Workstream C.1 lands as part of this).
5. **Task #2**: `render_action_row` → composite.
6. **Task #3**: `render_track_metadata_grid` → composite + VM.
7. **Task #1**: MusicBrainz family → composite + `MusicBrainzPanelVm`. Most
   complex; do last with the established pattern.
8. **Task #8**: audit empty-name fallbacks at `library.rs:3923` and
   `search.rs:4071`; hoist into the relevant view-model only if the
   coercion truly repeats across screens.

This sequencing trades total task count for risk: any pattern revision
(callback shape, where shared VMs live, image-handoff convention) is
discovered on a 20-line refactor rather than a 300-line one.

## Workstream B: Add a no-duplication architecture test

After Workstream A, add a test in `tests/architecture_tests.rs` that fails
if any `^fn render_[a-z_]+(` definition appears in more than one file in
`SCREEN_FILES`. Sketch:

- Walk every file in `SCREEN_FILES`.
- For each `^fn render_[a-z_]+(` definition, record `(name, file)`.
- Group by name; fail if any name appears in ≥2 distinct files.
- Allow an explicit `RENDER_HELPER_DUPLICATION_BASELINE` constant for
  documented exceptions, defaulting empty.

Cite the test in ADR 0033's "Enforcing tests" list.

## Workstream C: Tighten view-model coverage on remaining screen-local decisions

1. `render_track_header` title coercion: `if frame.title.is_empty() { TrackVm::title() }
   else { frame.title }`. Move to `TrackVm::display_title(override_title: Option<&str>)`
   so Library, Search, and feed surfaces share one path. Lands as part of
   Task #5.
2. Empty-name fallbacks at `library.rs:3923` and `search.rs:4071`. Hoist
   only if the coercion repeats across screens; otherwise leave as
   wiring-level code. Task #8.

## Workstream D (stretch scope): Relocate top-level UI shells

`src/ui_artist.rs`, `src/ui_entity.rs`, `src/ui_feed.rs`, `src/ui_track.rs`,
and `src/ui_context.rs` are shared UI but live at `src/*.rs`. The
architecture tests carve them out via `KNOWN_SHARED_UI_SHELL_FILES` and
`PRESENTATION_GLUE_FILES`. Moving them under `src/ui/shells/` (or merging
their content into `src/ui/composites/`) would let directory-scoped tests
cover them automatically and shrink the allowlists toward zero.

Trade-off: a wide rename touching every importer. Worth doing only after
Workstream A drains material from the screens; until then the importers
churn twice. Not a prerequisite for any other workstream.

## Risk Areas

- **Reconciling forked behavior is a UX choice, not a mechanical
  refactor.** Each duplicated pair will differ in at least one detail
  (spacing, empty state, conditional logic). Document the chosen canonical
  in the composite's module-level doc-comment so future contributors do not
  re-fork.
- **`view_models/mod.rs` forbids `SharedString` and `AnyElement`.** New VMs
  must expose plain `String`; the composite wraps. Easy to violate
  accidentally — caught only by review and `cargo clippy`.
- **Image handoff convention not yet established.** Composites take
  `Option<Arc<Image>>` as a separate parameter alongside the VM (per
  ADR 0033 invariant: image-cache lookup is screen-owned). The first
  composite that needs an image (`MusicBrainzPanelVm`, `FileHeaderVm`)
  pins the convention.
- **Selection callbacks must be screen-agnostic closures.** Current code
  uses `cx.weak_entity()` to dispatch on a typed `LibraryApp` /
  `SearchApp`. The composite cannot know either type; each screen wraps
  its own typed update inside an `impl Fn(usize, &mut Window, &mut App) +
  'static` closure.

## Existing Utilities to Reuse (do not re-implement)

- View-model entry points: `TrackVm`, `LibraryTrackRowVm`, `ReleaseDetailVm`,
  `FeedVm`, `ArtistVm` already exist in `src/view_models/`.
- Tokens: `SemanticColor`, `Spacing`, `Radius`, `FontSize`, `Weight` in
  `src/ui/tokens.rs`. Compatibility wrapper `crate::ui::style::color`
  already in use across screens.
- Composites already shipped: `DetailGrid`, `DetailHeader`, `DetailRow`,
  `EntityKind`, `ListRow`, `ReleaseDetailSurface`, `Thumbnail`,
  `AddToPlaylistPopover`, `TrackRow`, `TagBadge`, `IdentityAction`,
  `SegmentedControl`, `SplitPane`, `NowPlayingBar`, `DisclosureGroup`,
  `ActionButton`. New composites should compose these, not bypass them.
- Primitives: `Button`, `Divider`, `Image`, `Label`, `MultilineText`,
  `Popover`, `SectionHeader`, `Stack`, `Surface`. New floating chrome must
  extend `Popover` / `Surface` rather than inline GPUI APIs.
- Shared MusicBrainz formatting helpers already in `src/metadata.rs`:
  `musicbrainz_release_summary`, `musicbrainz_release_option_label`,
  `musicbrainz_subtitle`. The new VM consumes these; do not duplicate.

## Verification

- `cargo test --test architecture_tests` — must remain green at every task
  boundary; the new no-duplication test (Workstream B) is added once the
  consolidations are in place.
- `cargo test` — full suite, including any new VM tests added under
  `src/view_models/*`.
- `cargo clippy -- -D warnings` and `cargo fmt -- --check`.
- Manual visual smoke for both Library and Discovery on the ADR 0031 Task
  004 fixture set (release with website + Nostr, release with empty
  description, release with zero tracks, release with 100+ tracks),
  augmented by a track inspector with an open MusicBrainz panel both with
  and without a candidate match.
- Diff `library.rs` and `search.rs` line counts before and after each
  workstream. Target after Workstream A: ≥1,500 lines removed in total.

## Schema/API Implications

None.

## Out of Scope

- Backend, schema, or service changes.
- ADR 0031 release-detail contract work (proceeds independently).
- New HIG controls or visual redesign.
