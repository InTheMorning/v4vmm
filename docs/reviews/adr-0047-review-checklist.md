# ADR 0047 Review Checklist

## Reviewed Artifacts

- `docs/adr/0047-library-search-unification.md`
- `docs/plans/adr-0047-library-search-unification-phase-plan.md`
- `docs/tasks/adr-0047-task-001-content-filter-vm.md`
- `docs/tasks/adr-0047-task-002-inspector-panel-state-vm.md`
- `docs/tasks/adr-0047-task-003-description-collapse-vm.md`
- `docs/tasks/adr-0047-task-004-search-results-inspector-vm.md`
- `docs/tasks/adr-0047-task-005-saved-search-vm.md`
- `src/view_models/workspace.rs`
- `src/view_models/library.rs`
- `src/view_models/search_results.rs`
- `tests/architecture_tests.rs`

## Gate Status

Status: Phase B implemented - 2026-05-14.

Readiness decision: **Proceed to Phase C Task 006 after a fresh
handoff**.

Phase B is view-model-only. It intentionally has no visual confirmation
checkpoint because no renderer, screen, or UI shell changed.

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
- [x] Architecture guard locks Phase B placement, GPUI-free boundaries,
      and shared `ContentFilter` ownership.
- [x] No `src/ui/*`, screen, backend, DB, or playback files changed.

## Required Fixes

- None.

## Optional Improvements

- Revisit whether `SearchResultsPagedTab` should keep separate
  per-filter windows or collapse to one window plus filter counts when
  the loader is wired in Phase E.
- Replace text-line description estimates with renderer-provided line
  counts when Phase C renders disclosure controls.

## Architectural Drift Watchlist

- Do not introduce a second `ContentFilter` outside
  `src/view_models/workspace.rs`.
- Do not wire saved searches to persistence in Phase B/C; persistence
  belongs to a later explicit task.
- Do not add UI for Compare ID3, MusicBrainz, or description disclosure
  without routing through the new VM state.
- Do not move search-result routing into `src/search.rs`; ADR 0047
  retires that path after the shared inspector is ready.

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

## Merge Recommendation

Proceed with Phase C in a fresh implementation pass. Stop for visual
confirmation after Phase C touches inspector rendering.
