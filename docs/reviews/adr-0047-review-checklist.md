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
- `src/library.rs`
- `src/library/app_impl.rs`
- `src/ui/composites/disclosure_group.rs`
- `src/ui/shells/library/feed_detail.rs`
- `src/ui/shells/library/track_detail.rs`
- `src/ui/shells/library/track_detail_metadata.rs`
- `src/view_models/workspace.rs`
- `src/view_models/library.rs`
- `src/view_models/search_results.rs`
- `tests/architecture_tests.rs`

## Gate Status

Status: Phase C implemented - 2026-05-14; awaiting operator visual
confirmation.

Readiness decision: **Stop before Phase D until visual confirmation is
complete**.

Phase B was view-model-only. Phase C touches inspector rendering, so it
requires operator visual confirmation before filter-chip relocation
begins.

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

## Required Fixes

- None.

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
- [ ] Phase C compact track inspector visual confirmation.
- [ ] Phase C disabled Compare ID3 / MusicBrainz visual confirmation.
- [ ] Phase C description collapsed and expanded visual confirmation.
- [ ] Phase C feed description disclosure visual confirmation.

## Merge Recommendation

Do not proceed to Phase D until Phase C visual confirmation passes.
