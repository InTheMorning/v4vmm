# ADR 0047 Task 010a: ContentList Page VM Ownership

Status: Implemented - 2026-05-15.

## Goal

Create the GPUI-free `ContentListPageVm` ownership contract that Task
010 needs before filter chips can be wired into frame chrome. This task
does not render chips and does not change the transitional
Library/Search/Settings whole-screen mount.

## Files to Inspect

- `docs/adr/0047-library-search-unification.md`
- `docs/plans/adr-0047-library-search-unification-phase-plan.md`
- `docs/tasks/adr-0047-task-001-content-filter-vm.md`
- `docs/tasks/adr-0047-task-004-search-results-inspector-vm.md`
- `docs/tasks/adr-0047-task-009-filter-chip-strip-composite.md`
- `docs/tasks/adr-0047-task-010-wire-filter-chips-into-content-list-frame.md`
- `src/view_models/workspace.rs`
- `src/view_models/library.rs`
- `src/view_models/search_results.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/view_models/library.rs`
- `tests/architecture_tests.rs`
- `docs/reviews/adr-0047-review-checklist.md`

## Do Not Touch

- `src/ui/*`
- `src/app.rs`
- Backend, MusicIndex, db, playback
- Toolbar global search or `GlobalSearchScope`

## Constraints

- Model only. No frame shell wiring and no visible filter chips.
- `ContentListPageVm` is GPUI-free and lives in the view-model layer.
- The VM owns `filter_state: ContentFilter` and exposes
  `set_filter(ContentFilter)`.
- Row projection consumes the current filter state. Do not create a
  global filter store.
- Rows carry source/provenance sufficient for `ContentFilter::Library`
  and `ContentFilter::Index` to produce deterministic visible rows.
- The VM exposes empty-filter state for zero visible rows.
- The VM exposes `filter_chip_strip() -> FilterChipStripDisplay`.
- Reuse `ContentFilter` and `FilterChipStripDisplay` from
  `src/view_models/workspace.rs`; do not define a second filter enum.

## Implementation Steps

1. Add a `ContentListPageVm` contract in `src/view_models/library.rs`.
2. Add row display/source types only as needed to distinguish Library
   rows from Index rows in a GPUI-free way.
3. Add `filter_state`, `filter()`, and `set_filter(ContentFilter)`.
4. Add filtered row projection that applies `All`, `Library`, and
   `Index` consistently to cached rows.
5. Add an empty-state projection for filters that hide every row.
6. Add `filter_chip_strip() -> FilterChipStripDisplay` using the
   content-list constructor from `src/view_models/workspace.rs`.
7. Add unit tests covering default filter, filter mutation, row
   projection for each source, empty-filter state, and chip-strip
   selected-state passthrough.
8. Add an architecture guard proving that `ContentListPageVm` exists
   before Task 010 is allowed to wire chips into `FrameShellDisplay`.

## Acceptance Criteria

- [x] `ContentListPageVm` exists and is GPUI-free.
- [x] VM owns `filter_state` and `set_filter(ContentFilter)`.
- [x] Row projection changes with `All`, `Library`, and `Index`.
- [x] Empty-filter state is modeled in the VM.
- [x] `filter_chip_strip()` returns `FilterChipStripDisplay` with the
  current selected filter.
- [x] Architecture guard records the page-VM ownership contract.
- [x] No UI, app, backend, db, or playback files changed.

## Implementation Notes

- `src/view_models/library.rs` now defines `ContentListPageVm`,
  `ContentListRowDisplay`, `ContentListRowSource`, and
  `ContentListEmptyStateDisplay`.
- `ContentListPageVm` owns frame-local `filter_state`, filters cached
  rows through `ContentFilter`, and exposes `filter_chip_strip()` using
  `FilterChipStripDisplay::default_for_content_list`.
- `TrackRow.is_in_library` is the Task 010a membership discriminator:
  local rows project to `Library`, non-local cached rows project to
  `Index`. Future source/provenance refinement needs a separate task if
  MusicIndex/local merge semantics require more than membership.
- `tests/architecture_tests.rs` records that this task is VM-only and
  must not wire `ContentListPageVm` into `src/ui/*` or `src/app.rs`.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test library
cargo test --test architecture_tests
cargo clippy -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0047-library-search-unification.md`
- `docs/tasks/adr-0047-task-001-content-filter-vm.md`
- `docs/tasks/adr-0047-task-004-search-results-inspector-vm.md`
- `docs/tasks/adr-0047-task-009-filter-chip-strip-composite.md`
- `docs/tasks/adr-0047-task-010-wire-filter-chips-into-content-list-frame.md`
- `src/view_models/workspace.rs`
- `src/view_models/library.rs`
- `src/view_models/search_results.rs`
- `tests/architecture_tests.rs`

Goal:
- Add the GPUI-free `ContentListPageVm` ownership contract required
  before Task 010 can wire filter chips into frame chrome.

Constraints:
- Model only. Do not render chips and do not edit `src/ui/*`.
- Reuse `ContentFilter` and `FilterChipStripDisplay`; do not define a
  second filter type.
- The VM owns per-frame filter state and row projection.
- Do not touch backend, db, playback, toolbar global search, or app
  window/frame wiring.

Do not touch:
- `src/ui/*`
- `src/app.rs`
- Backend, MusicIndex, db, playback
- Toolbar global search

Acceptance criteria:
- `ContentListPageVm` compiles, is GPUI-free, and owns
  `filter_state`.
- `set_filter(ContentFilter)` updates row projection.
- Empty-filter state and `filter_chip_strip()` are covered by tests.
- Architecture guard proves Task 010 has a real page VM to consume.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test library`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Row provenance cannot be represented without backend or persistence
  changes.
- `ContentListPageVm` cannot filter rows without coupling to GPUI
  renderers.
- Implementing this task appears to require changes to
  `src/ui/shells/workspace.rs` or `src/app.rs`.
