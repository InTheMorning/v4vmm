# ADR 0084 Task 014: A Scrollbar On Each Music Column

Status: Ready - 2026-10-17.

## Goal

Each Music column that scrolls shows a vertical scrollbar. A person can drag it, so a column scrolls without a mouse wheel or a touchpad. The scrollbar covers no content. A drag to the end of "New releases" loads the next page, as a wheel scroll does.

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md) Decision 1, with its amendment of 2026-10-17: each column that can scroll shows a vertical scrollbar, with the shared scrollbar gutter.
- [ADR 0063](../adr/0063-show-dashboard-layout.md): the shared scrollbar gutter. `page_scroll_content` in `src/ui/composites/page_scroll_content.rs` gives it.
- [ADR 0062](../adr/0062-music-content-surface.md): the recency order and the paging of "New releases".
- The operator decided on 2026-10-16 to do this packet after packet 013 and before packet 007.

## Recorded Facts - 2026-10-17

- No Music column shows a scrollbar. The app binds no scroll key. The operator can scroll a Music column only with a mouse wheel or a touchpad.
- The app has one shared scrollbar pattern. Settings (`src/ui/composites/settings.rs`) and the maintenance page (`src/ui/composites/maintenance_page.rs`) use it: the scroll element has `.overflow_y_scroll().track_scroll(handle)`, its content is in `page_scroll_content(cx)`, and the parent has `.vertical_scrollbar(handle)`. `page_scroll_content` adds a right padding of `OVERLAY_SCROLLBAR_WIDTH` plus `Spacing::SM`, so the overlay scrollbar covers no content.
- `gpui_component::scroll::ScrollableElement` also gives `.overflow_y_scrollbar()`. It keeps the element as the scroll container, adds a vertical scrollbar, and keeps its own scroll state for each element id (`src/scroll/scrollable.rs` in `gpui-component` 0.6.1). Its default id is the call site, so a call site that renders more than one page needs `.id(..)` for each page.
- The content list (`src/ui/shells/library/content_list.rs`, both the rows and the tiles) keeps the tracked handle `LibraryApp::recent_music_scroll`. Back uses it to restore the scroll position of "New releases" (task 006 V3). `attach_content_list_auto_pagination` loads the next page only in an `on_scroll_wheel` listener. A drag of a scrollbar sends no wheel event.
- These are the scroll containers that a Music column renders. Each one calls `.overflow_y_scroll()` today:
  - sidebar: `render_sidebar_column` in `src/library/app_impl.rs` (`music-destinations-scroll`)
  - list column: `src/ui/shells/library/destination_lists.rs` (three containers), `src/ui/shells/library/content_list.rs` (two containers), and the search results in `src/ui/shells/search_results_inspector.rs`
  - detail column: `src/ui/composites/release_detail_surface.rs` (the release pages of both origins), `src/ui/shells/library/track_detail.rs`, the Index track page in `src/ui/shells/search_results_inspector.rs`, `src/ui/shells/library/feed_list.rs` (the artist page), `src/ui/shells/playlist.rs`, `src/ui/shells/publisher.rs` and `src/ui/shells/name_match_page.rs`
- `src/ui/shells/queue_now_playing.rs` is in Show, and `src/ui/composites/confirmation_dialog.rs` is a dialog. They are not Music columns.

## Design

### One way to make a Music column scroll

Add one shared helper in `src/ui/composites/` (for example `music_column_scroll.rs`). It owns the scrollbar choice for a Music column, so no shell picks its own:

- For a container with no tracked handle: the container uses `.overflow_y_scrollbar()` with an explicit, stable `.id(..)`, and its children are in `page_scroll_content(cx)`. A call site can render one page for many items. It then gives an id for each item, so two pages do not share one scroll position.
- For a container with a tracked handle (the content list): the Settings pattern. The container keeps `.overflow_y_scroll().track_scroll(handle)`, its children are in `page_scroll_content(cx)`, and the parent has `.vertical_scrollbar(handle)`.

The helper can be one function for each case, or one builder. It keeps the present flex rules. Each scrolling column is a flex column with `min_h_0`. Each row and the tile grid do not shrink (task 004 review).

Keep the present horizontal padding of each container. `page_scroll_content` adds only the gutter on the right.

### The paging of "New releases"

The next page loads when the scroll position comes near the end, whatever moved it: the wheel, the touchpad or a drag of the scrollbar. Move the near-end check out of the `on_scroll_wheel` listener into a check that runs after each scroll change. A `canvas` prepaint that reads `recent_music_scroll`, as `src/ui/composites/log_frame.rs` reads its handle, is one way. The check keeps its present guards. It sends no request while a page loads, and no request when the Index has no more pages. The stale-result checks of the `RecentMusic` source stay.

The "Load more" button stays.

### What stays

- Back to "New releases" restores its scroll position (task 006 V3).
- The scroll position of a column does not move when a row changes in place.
- The detail column header (Back, Forward, the title and the breadcrumb) stays outside the scroll container.

## Required Changes

1. Add the shared helper, as "One way to make a Music column scroll" gives.
2. Change each Music scroll container in "Recorded Facts" to the helper. Give each container a stable id.
3. Change the content list paging, as "The paging of "New releases"" gives.
4. Add the guard and the tests of the mechanical criteria.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_scrollbars_` for tests.

| Case | Required proof |
|---|---|
| R14-01 | A guard names ADR 0084 Decision 1 (amended 2026-10-17): each file in the Music container list of "Recorded Facts" has no bare `.overflow_y_scroll()` outside the shared helper. A container uses the helper. The failure message names the ADR and the helper |
| R14-02 | The near-end paging decision is a pure function of the scroll offset, the viewport height, the content height, `has_more` and `is_loading`. Near the end with more pages, it loads. At the top, while a page loads, or with no more pages, it does not load |
| R14-03 | A guard: `attach_content_list_auto_pagination`, or its replacement, does not depend on `on_scroll_wheel` alone for the paging decision |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: the sidebar, the list column and the detail column each show a vertical scrollbar when their content is longer than the column. A column with short content shows no scrollbar, or an empty track.
- V2: a drag of each scrollbar scrolls its column. The mouse wheel and the touchpad still scroll.
- V3: a drag of the "New releases" scrollbar to the end loads the next page.
- V4: the scrollbar covers no text, no tile, no row action and no "Show more".
- V5: Back to "New releases" returns to the same scroll position.
- V6: two release pages, opened one after the other, do not share one scroll position. The second page opens at its top.
- V7: Light and Dark, and a larger scale, show the scrollbars with no clipped content.

## Exclusions

- No keyboard scroll and no keyboard selection. Packet 009 owns them.
- No virtual list.
- No horizontal scrollbar.
- No change to Settings, Show or the log frames.
- No schema change.

## Files To Inspect

- [Agent rules](../../AGENTS.md), the [source map](../architecture/source-map.md), the [phase plan](../plans/adr-0084-music-column-browser-phase-plan.md) and the archived [task 013](archive/adr-0084-task-013-index-release-page-parity.md).
- `src/ui/composites/page_scroll_content.rs`, `src/ui/composites/settings.rs`, `src/ui/composites/maintenance_page.rs`, `src/ui/composites/log_frame.rs`, `src/ui/layouts.rs`.
- `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-component-0.6.1/src/scroll/scrollable.rs`.
- Each file in the Music container list of "Recorded Facts".
- `src/library.rs` (`recent_music_scroll`), `src/ui/shells/workspace.rs`.
- `tests/architecture_tests.rs`.

## Checks

```bash
cargo test adr_0084_scrollbars_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

Run `cargo build --bin v4vmm` last, after every test run. A test run can leave a GPUI test-support binary at `target/debug/v4vmm` (AGENTS.md, "Build, Test, Lint").

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md`, `docs/architecture/source-map.md` and `docs/plans/adr-0084-music-column-browser-phase-plan.md`
- This packet: `docs/tasks/adr-0084-task-014-music-column-scrollbars.md`
- ADR 0084 Decision 1 with its amendment of 2026-10-17, and the "Shared Log Frames And Following" section of ADR 0063
- Each file in "Files To Inspect"

Goal:
- A vertical scrollbar on each Music column that scrolls, from one shared helper, as "Design" gives.
- Paging of "New releases" that a scrollbar drag also starts.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Use named tokens. No raw size or color in a renderer.
- Keep each scrolling column a flex column with `min_h_0`, and each row and tile grid non-shrinking.
- Keep the tracked handle `recent_music_scroll` and the Back restore of "New releases".
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git add` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- Settings, Show, the log frames and the confirmation dialog.
- The navigation model in `src/view_models/workspace/`.
- The page content: rows, tiles, facts, actions and descriptions.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R14-01 to R14-03 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result" section and an "Operator Visual Check" section for V1 to V7. The check gives numbered steps, the expected result, an incorrect result, and the cleanup. Each step that needs scrolling uses a drag of the scrollbar.

Test commands:
- Each command in "Checks".

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- A Music scroll container is inside another scroll container, so two scrollbars would show for one column.
- `overflow_y_scrollbar()` breaks the flex layout of a column, for example the column stops scrolling or loses its height.
- The paging check cannot read the scroll position after a scrollbar drag.
- A change needs a file in "Do not touch".
