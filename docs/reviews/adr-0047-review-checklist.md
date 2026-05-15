# ADR 0047 Review Checklist

## Reviewed Artifacts

- `docs/adr/0047-library-search-unification.md`
- `docs/plans/adr-0047-library-search-unification-phase-plan.md`
- `docs/tasks/adr-0047-task-001-content-filter-vm.md`
- `docs/tasks/adr-0047-task-002-inspector-panel-state-vm.md`
- `docs/tasks/adr-0047-task-003-description-collapse-vm.md`
- `docs/tasks/adr-0047-task-004-search-results-inspector-vm.md`
- `docs/tasks/adr-0047-task-005-saved-search-vm.md`
- `docs/tasks/adr-0047-task-006-disable-compare-musicbrainz-on-undownloaded.md`
- `docs/tasks/adr-0047-task-007-gate-library-extra-fields-behind-expanded-panels.md`
- `docs/tasks/adr-0047-task-008-description-disclosure.md`
- `docs/tasks/adr-0047-task-009-filter-chip-strip-composite.md`
- `docs/tasks/adr-0047-task-010-wire-filter-chips-into-content-list-frame.md`
- `docs/tasks/adr-0047-task-010a-content-list-page-vm-ownership.md`
- `src/library.rs`
- `src/library/app_impl.rs`
- `src/ui/composites/disclosure_group.rs`
- `src/ui/composites/filter_chip_strip.rs`
- `src/ui/composites/frame_shell.rs`
- `src/ui/shells/library/feed_detail.rs`
- `src/ui/shells/library/track_detail.rs`
- `src/ui/shells/library/track_detail_metadata.rs`
- `src/view_models/workspace.rs`
- `src/view_models/library.rs`
- `src/view_models/search_results.rs`
- `tests/architecture_tests.rs`

## Gate Status

Status: Phase D Task 010 Library-backed frame wiring and Task 011 toolbar
scope retirement are implemented and user-visually confirmed - 2026-05-15.

Readiness decision: **Task 010 is implemented only through the GPUI-free
`ContentListPageVm`. Do not extend chips to Search/Settings transitional mounts
or create a global/renderer-local filter store.**

Phase B was view-model-only. Phase C touched inspector rendering, so it still
requires operator visual confirmation. The operator resumed ADR 0047 completion
on 2026-05-15; Task 009 is limited to shared frame-chrome filter chip structure
and does not wire real filtering or remove `GlobalSearchScope`. Task 011 is the
bounded Phase D cleanup for the duplicate toolbar controls observed after Task
010, not a search-routing redesign; it is now implemented.

## Required Checks

- [x] Task 001: `ContentFilter`, `FilterChipOption`, and
      `FilterChipStripDisplay` live in the workspace VM and are GPUI-free.
- [x] Task 002: track-inspector panel state and Compare ID3 /
      MusicBrainz availability predicates live in the library VM.
- [x] Task 003: description disclosure state and 5-line auto-collapse
      threshold are modeled in the library VM.
- [x] Task 004: `SearchResultsInspectorPageVm` exists as a GPUI-free,
      tabbed, paged contract and reuses the workspace `ContentFilter`.
- [x] Task 005: saved-search display state is exposed from the source-list
      VM layer without persistence or schema changes.
- [x] Task 006: Compare ID3 and MusicBrainz controls remain visible but
      disabled with tooltip copy when a library track lacks a local file.
- [x] Task 007: Library-only metadata panels are driven by
      `LibraryTrackInspectorState` and remain hidden until explicitly
      expanded for downloaded tracks.
- [x] Task 008: feed and track descriptions render through shared
      disclosure chrome backed by `DescriptionState`.
- [x] Phase C follow-up: track description toggles persist per track id,
      and feed updates persist MusicIndex feed descriptions into local
      feed rows instead of relying on renderer placeholder inference.
- [x] Phase C follow-up: visible membership controls use Download/Remove
      vocabulary rather than subscribe/unsubscribe wording.
- [x] Architecture guard locks Phase B placement, GPUI-free boundaries,
      and shared `ContentFilter` ownership.
- [x] Architecture guard locks Phase C inspector-state ownership,
      disabled metadata actions, and shared disclosure rendering.
- [x] No backend, DB schema, or playback files changed.
- [x] Task 009: shared `filter_chip_strip` composite exists and reuses
      existing segmented-control/context-menu primitives.
- [x] Task 009: `FrameShellDisplay` carries optional frame-local filter
      display and `frame_shell` renders it only when present.
- [x] Task 009 architecture guard green.
- [x] Task 009 visual proof deferred to Task 010 visible-frame wiring.
- [x] ADR 46 Phase 4 visual gate cleared for Task 010 sequencing.
- [x] ADR 46 Task 012 persistence gate cleared for Task 010 sequencing.
- [x] Task 010 escalation recorded: current `ContentList` frame wraps whole
      Library/Search/Settings screens and initially could not host per-frame
      filter state.
- [x] Task 010a implemented: `ContentListPageVm` owns per-frame filter state,
      source-aware row projection, empty-filter state, and chip-strip display
      before frame-shell chip wiring resumes.
- [x] Task 010 implemented for the Library-backed `ContentList` frame:
      `LibraryViewModel` owns `ContentListPageVm`, workspace shell accepts a
      typed filter-chip slot/callback, and `TopApp::set_frame_filter` dispatches
      by visible frame id.

## Required Fixes

- Keep the Task 011 architecture guard in place: toolbar source scope controls
  and `GlobalSearchScope` must not return.
- Keep Search and Settings free of content-list chips until Phase E gives them
  real frame-local content/search-result owners.

## Prohibited Regression

- Never treat placeholder-looking metadata such as `...` or `...`
  repeated across lines as absent in Library/Search renderers,
  composites, or view-model display helpers. That was the first
  attempted fix for the Strange Love description bug, and it is
  explicitly prohibited.
- Correct source-data defects at the source boundary instead: RSS and
  MusicIndex feed/item descriptions must be preserved and refreshed
  into local rows or source-fact state. Display code may trim empty
  whitespace; it must not infer that non-empty source text is invalid.
- Visible membership controls must not use subscribe/unsubscribe
  vocabulary. Feeds and tracks use the app-level action language:
  `Download Feed`, `Remove Feed`, `Download Track`, and `Remove Track`.
  When feed removal leaves a remote feed detail visible, the primary
  action must telegraph the new state as `Download Feed`.

## Optional Improvements

- Revisit whether `SearchResultsPagedTab` should keep separate
  per-filter windows or collapse to one window plus filter counts when
  the loader is wired in Phase E.
- Replace text-line description estimates with renderer-provided line
  counts if a future GPUI text-measurement API makes that practical.

## Architectural Drift Watchlist

- Do not introduce a second `ContentFilter` outside
  `src/view_models/workspace.rs`.
- Do not wire saved searches to persistence in Phase B/C; persistence
  belongs to a later explicit task.
- Do not add UI for Compare ID3, MusicBrainz, or description disclosure
  without routing through the new VM state.
- Do not move search-result routing into `src/search.rs`; ADR 0047
  retires that path after the shared inspector is ready.
- Do not reintroduce renderer/view-model placeholder inference for
  descriptions. If a description looks wrong, fix hydration or
  persistence.
- Do not reintroduce subscribe/unsubscribe button labels in Library,
  Search, shared entity shells, or action-row view models.

## Test Gates

Green on 2026-05-14:

```bash
cargo fmt -- --check
cargo check
cargo test
cargo clippy -- -D warnings
```

## Visual Readiness Checklist

- [x] Phase B has no visual confirmation point; no UI rendering changed.
- User visual pass on 2026-05-15 found the current narrow Search inspector
  layout still has crowded text actions. That is not Task 009 filter-chip proof;
  keep Phase D visual confirmation open until Task 010 wires the strip into a
  visible frame and the action controls have an explicit icon-button task or
  ADR.
- [ ] Phase C compact track inspector visual confirmation.
- [ ] Phase C disabled Compare ID3 / MusicBrainz visual confirmation.
- [ ] Phase C description collapsed and expanded visual confirmation.
- [ ] Phase C feed description disclosure visual confirmation.
- [x] Phase D Task 010 Library-backed filter chip strip visual confirmation.
- [ ] Phase D Task 010 filter chip narrow menu visual confirmation.
- [x] Phase D Task 011 toolbar scope-control removal visual confirmation.
- 2026-05-15 lower-context review found Task 010 and later tasks still
  incomplete. Task 009 is structurally present, but the transitional workspace
  does not mount the filter strip in normal app flow until Task 010. Do not
  treat the deferred visual proof as completion of Phase D.
- 2026-05-15 Task 010 implementation wires the Library-backed `ContentList`
  through `ContentListPageVm`, so the chips mutate visible Library tree rows
  and expose the VM-owned empty-filter state. User visual confirmation found the
  planned follow-up problem: toolbar `GlobalSearchScope` controls still render
  beside the per-frame chips. Search/Settings remain transitional whole-screen
  mounts and intentionally do not receive chips yet.
- 2026-05-15 Task 011 implementation removes the duplicate toolbar
  `All / Library / Index` controls and deletes `GlobalSearchScope`.
  User visual proof confirmed the toolbar keeps the search field and submit
  button, the Library frame-local filter remains in frame chrome, and global
  search still submits into Search results.

## Merge Recommendation

Task 011 passes review. Phase E remains the next architectural step for routing
global search into the shared search-results inspector; do not reuse the retired
toolbar source-scope model.
