# ADR 0062 Task 002: Music Opens On Recent Music

Status: Ready - 2026-09-07. Do after task 001.

## Goal

Make the `Music` content region render rows instead of the navigation tree, in
feed publish date order, paged. This is the task that answers both operator
complaints.

## Files To Inspect

- `docs/adr/0062-music-content-surface.md`
- `docs/adr/0041-windowed-paged-view-models.md`
- `docs/tasks/adr-0062-task-001-mixed-entity-row-contract.md`
- `src/view_models/recent_feeds.rs`, for `RecentFeedsPageVm` and its cursor
  paging
- `src/view_models/library.rs`, for `ContentListPageVm` and
  `content_list_rows_from_tree`
- `src/ui/shells/workspace.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/view_models/library.rs`
- `src/library/app_impl.rs`
- `src/app.rs`
- `src/ui/shells/workspace.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/view_models/recent_feeds.rs` internals. This task consumes its query and
  its paging. Task 005 moves ownership.
- The `Recent Feeds` toolbar command. Task 005 removes it.
- The filter control. Task 003 replaces it.
- The source tree itself. It keeps its own region.

## Constraints

- **Reuse the paging that exists.** `RecentFeedsPageVm` already has cursor
  paging, `begin_load(append)`, `finish_load`, and `has_more()`. Do not write a
  second pager. ADR 0041 governs windowed paged view models.
- **All music is what scrolling reveals.** There is no separate everything
  query and no unbounded load at startup. Newest first, then pages.
- The content region renders rows. The navigation tree keeps its own region and
  the breadcrumb stays.
- `content_list_rows_from_tree` stays for the tree-derived case. This task adds
  the recency source beside it. A guard requires that function today, so
  removing it breaks an unrelated guard.
- Loading, empty, and failed states are view-model facts with their own
  displays. A renderer never composes an error string.
- The query blocks. It runs in the runtime, never on the render path.

## Implementation Steps

1. Add a recency row source that projects the index query into task 001 rows.
2. Give `ContentListPageVm` a way to be populated from the recency source as
   well as from the tree, without a second page model.
3. Wire the default `Music` state to the recency source, so a fresh section with
   no search and no selection shows rows.
4. Render `visible_rows()` in the content region. Keep the source tree in its
   own region.
5. Wire load-more to the existing cursor paging.
6. Add loading, empty, and failed displays for the content region.
7. Add guards, situational, citing ADR 0062:
   - the content region renders rows and not the tree
   - no second pager exists beside `RecentFeedsPageVm`
   - no blocking query runs on the render path
8. Add view-model tests: first page, appended page, no more pages, empty
   result, and failed load.

## Acceptance Criteria

Mechanical:

- The default `Music` state projects rows from the recency source, ordered
  newest first.
- Load-more appends through the existing cursor paging, and no second pager
  exists.
- Loading, empty, and failed states are view-model displays.
- A guard proves no blocking query runs on the render path.
- `content_list_rows_from_tree` still exists and its guard still passes.

Visual proof, operator only:

- `Music` opens on music rather than a tree.
- The content region is filled at a normal window width.
- Scrolling loads more rows without a visible stall.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test library --lib --quiet`
- `cargo test recent_feeds --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`
- Do not run the app. Write the operator visual check instead, as AGENTS.md
  requires.

## Expected Final Report Format

1. Files changed
2. Tests run
3. Behavior changed
4. Screenshots captured, or the visual gate reported open
5. Deviations from task
6. Unresolved concerns

## Escalation Triggers

- `RecentFeedsPageVm` cannot be consumed without moving it. Report it rather
  than duplicating its paging. Task 005 owns the move.
- The index query returns feeds only, and artist or track rows need a second
  call. Report the shape before adding a request per row.
- Populating `ContentListPageVm` from two sources needs a change to the guard
  that requires `replace_rows(content_list_rows_from_tree(&tree))`.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0062-music-content-surface.md`, `docs/adr/0041-windowed-paged-view-models.md`
- `src/view_models/recent_feeds.rs`, `src/view_models/library.rs`,
  `src/ui/shells/workspace.rs`

Goal:
- The `Music` content region renders rows in feed publish date order, paged,
  instead of the navigation tree.

Constraints:
- Reuse `RecentFeedsPageVm` cursor paging. Do not write a second pager.
- No unbounded load. Newest first, then pages.
- The tree keeps its own region. The breadcrumb stays.
- Keep `content_list_rows_from_tree`. A guard requires it.
- No blocking query on the render path.

Do not touch:
- `recent_feeds.rs` internals, the Recent Feeds command, the filter control

Acceptance criteria are split into mechanical and visual. Report the visual
gate as open if no window can be opened.

Test commands:
- `cargo fmt -- --check`
- `cargo test library --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. screenshots captured or the visual gate reported open
5. deviations from task
6. unresolved concerns
