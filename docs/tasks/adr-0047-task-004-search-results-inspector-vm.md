# ADR 0047 Task 004: Search Results Inspector VM

Status: Proposed - 2026-05-14.

## Goal

Define `SearchResultsInspectorPageVm` with a tabbed contract
(`Artists` / `Feeds` / `Tracks`), per-tab paged windows (per
ADR 0041), and a per-frame filter slot (`ContentFilter`). GPUI-free
contract.

## Files to Inspect

- `docs/plans/library-search-unification-plan.md`
- `docs/plans/adr-0047-library-search-unification-phase-plan.md`
- `docs/adr/0041-windowed-paged-view-models.md`
- `src/view_models/workspace.rs`
- `src/view_models/library.rs` (page-VM precedent)
- `src/view_models/search.rs` if present (legacy contract)
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/view_models/search_results.rs` (new, GPUI-free)
- `src/view_models/mod.rs` (module declaration)
- `tests/architecture_tests.rs`

## Do Not Touch

- Backend HTTP / Musicindex / db
- Any `src/ui/*`
- Playback engine

## Constraints

- Zero `gpui::*` imports.
- `M-CANONICAL-DOCS` on every public type.
- Tabbed contract:
  ```rust
  enum SearchResultsTab { Artists, Feeds, Tracks }
  struct SearchResultsInspectorPageVm {
      query: String,
      tab: SearchResultsTab,
      filter: ContentFilter,
      artists: PagedListVm<ArtistResultDisplay>,
      feeds:   PagedListVm<FeedResultDisplay>,
      tracks:  PagedListVm<TrackResultDisplay>,
      empty_state: Option<EmptyStateDisplay>,
  }
  ```
- `ArtistResultDisplay`, `FeedResultDisplay`, `TrackResultDisplay` are
  GPUI-free display structs (id, label, secondary text, optional
  thumbnail-href, a11y label).
- `EmptyStateDisplay { title, secondary, clear_filter_action_id }` —
  HIG content-unavailable view (lives next to the page VM or in
  workspace.rs).
- Builder pattern when constructor params reach four or more.
- Tab and filter state are independent; switching tabs preserves
  filter.

## Implementation Steps

1. Add `src/view_models/search_results.rs` and declare it.
2. Define `SearchResultsTab` enum.
3. Define the three result display structs.
4. Define `EmptyStateDisplay` (or reuse if one exists).
5. Define `SearchResultsInspectorPageVm` with tabbed paged contract.
6. Add helpers: `set_tab(tab)`, `set_filter(filter)`, `is_empty(tab,
   filter) -> bool`.
7. Unit tests covering: tab/filter independence; per-tab paged
   windows operate independently; empty state surfaces when active
   tab + filter combination yields zero rows.
8. Architecture guard asserting the module is GPUI-free and contains
   the listed types.

## Acceptance Criteria

- [ ] Module compiles, public types documented.
- [ ] Tab and filter state independent; tab switch preserves filter.
- [ ] Per-tab paged windows compile (use ADR 0041 machinery).
- [ ] Unit tests cover tab/filter independence + empty state.
- [ ] No `gpui` imports.
- [ ] Architecture guard records the contract.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test search_results
cargo test --test architecture_tests
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/plans/library-search-unification-plan.md`
- `docs/plans/adr-0047-library-search-unification-phase-plan.md`
- `docs/adr/0041-windowed-paged-view-models.md`
- `src/view_models/workspace.rs`
- `src/view_models/library.rs`
- `tests/architecture_tests.rs`

Goal:
- Add `src/view_models/search_results.rs` with
  `SearchResultsInspectorPageVm` (tabbed, paged-per-tab, GPUI-free).

Constraints:
- Three tabs: Artists, Feeds, Tracks.
- Per-tab paged windows via ADR 0041 machinery.
- Tab and filter state independent.

Do not touch:
- Backend / db / Musicindex
- `src/ui/*`
- Playback

Acceptance criteria:
- Module compiles with docs and unit tests.
- Per-tab paged windows operate independently.
- Empty state contract present.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test search_results`
- `cargo test --test architecture_tests`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- ADR 0041 windowed paged VM cannot host three concurrent windows
  without refactor (escalate first).
- Tab and filter state require coupling to compile (signals contract
  design needs adjustment).
