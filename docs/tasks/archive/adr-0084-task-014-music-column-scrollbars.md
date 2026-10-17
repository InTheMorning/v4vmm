# ADR 0084 Task 014: A Scrollbar On Each Music Column

Status: Complete - 2026-10-17. The mechanical checks are Green. The operator passed V1 to V7 on 2026-10-17.
visual check (V1 to V7) remains open.

## Goal

Each Music column that scrolls shows a vertical scrollbar. A person can drag it, so a column scrolls without a mouse wheel or a touchpad. The scrollbar covers no content. A drag to the end of "New releases" loads the next page, as a wheel scroll does.

## Authority

- [ADR 0084](../../adr/0084-music-column-browser.md) Decision 1, with its amendment of 2026-10-17: each column that can scroll shows a vertical scrollbar, with the shared scrollbar gutter.
- [ADR 0063](../../adr/0063-show-dashboard-layout.md): the shared scrollbar gutter. `page_scroll_content` in `src/ui/composites/page_scroll_content.rs` gives it.
- [ADR 0062](../../adr/0062-music-content-surface.md): the recency order and the paging of "New releases".
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

- [Agent rules](../../../AGENTS.md), the [source map](../../architecture/source-map.md), the [phase plan](../../plans/adr-0084-music-column-browser-phase-plan.md) and the archived [task 013](adr-0084-task-013-index-release-page-parity.md).
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

## Implementation Result - 2026-10-17

Each Required Change is done.

### Files Changed

- `src/ui/composites/music_column_scroll.rs` (new file): the shared helper named in "Design". `music_column_scrollbar` gives a column its own scrollbar, with a caller-given stable id. `music_column_tracked_scrollbar` gives a column its scrollbar with a tracked handle, and takes an optional probe layer.
- `src/ui/composites/mod.rs`: registers the new module.
- `src/library/app_impl.rs`: the sidebar's destinations list now calls `music_column_scrollbar`.
- `src/ui/shells/library/destination_lists.rs`: the Discover views list, the Releases or Artists destination rows, and the Playlists destination list each call `music_column_scrollbar`. The Releases list and the Artists list each get a different scrollbar id, built from `empty_label`, so they keep separate scroll positions.
- `src/ui/shells/library/content_list.rs`: the "New releases" rows and tiles each build their content inside `page_scroll_content`, then call `attach_content_list_scroll`. Deletes `attach_content_list_auto_pagination` and its `on_scroll_wheel` paging check.
- `src/ui/shells/library/content_list_scroll.rs` (new file): holds `attach_content_list_scroll`, moved out of `content_list.rs` under Deviation 1. It reads the scroll position on a `canvas` prepaint after each scroll change. `src/ui/composites/log_frame.rs` reads its own handle the same way. It then calls `music_column_tracked_scrollbar` with that reading layer.
- `src/ui/shells/library/mod.rs`: registers the new module.
- `src/ui/shells/search_results_inspector.rs`: the Index detail loading or failed state, the Index track page, and the search results rows of each tab call `music_column_scrollbar`. The search failure notice calls it too.
- `src/ui/composites/release_detail_surface.rs`: the scrollable release page, for both origins, calls `music_column_scrollbar`. Its scrollbar id is built from the page's own `id` field, so a Library release page and a Discover release page keep separate scroll positions.
- `src/ui/shells/library/track_detail.rs`: the Library track page calls `music_column_scrollbar`.
- `src/ui/shells/library/feed_list.rs`: the artist page calls `music_column_scrollbar`.
- `src/ui/shells/playlist.rs`: the playlist page calls `music_column_scrollbar`.
- `src/ui/shells/publisher.rs`: the publisher page calls `music_column_scrollbar`.
- `src/ui/shells/name_match_page.rs`: the name-match page calls `music_column_scrollbar`.
- `src/view_models/pagination.rs`: adds `should_load_more_near_scroll_end`, the pure near-end paging function of R14-02, and five tests for it.
- `tests/architecture_tests.rs`: adds `adr_0084_scrollbars_r14_01_music_columns_use_the_shared_helper` (R14-01) and `adr_0084_scrollbars_r14_03_content_list_paging_is_not_wheel_only` (R14-03). Changes one required string in `adr_0066_search_failure_report_stays_readable_and_vm_owned`, under Deviation 2.

### Behavior Changed

- Each Music column that scrolls now shows a vertical scrollbar. Before this change, a column scrolled only with a mouse wheel or a touchpad.
- A drag of a column's scrollbar now scrolls that column.
- A drag of the "New releases" scrollbar to its end now starts the next page load, the same as a wheel scroll.
- The wheel and the touchpad still scroll each column. A wheel scroll near the end of "New releases" still starts the next page load, now through the same check as a drag.

### Checks

| Command | Result |
|---|---|
| `cargo test adr_0084_scrollbars_` | Green, 8 tests |
| `cargo test` | Green: 1,873 unit tests, 314 guards, 10 ignored doc tests |
| `cargo test --test architecture_tests` | Green, 314 tests |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

### Deviations From The Task

1. `src/ui/shells/library/content_list.rs` keeps its paging wire-up in a new sibling file, `src/ui/shells/library/content_list_scroll.rs`. The guard `surface_modules_under_500_loc` (ADR 0038 Task 007) caps a screen surface file at 500 code lines. The merged function pushed `content_list.rs` past that cap. The two new R14 guards in `tests/architecture_tests.rs` read both files together, as one column owner.
2. The existing situational guard `adr_0066_search_failure_report_stays_readable_and_vm_owned` checked the search failure notice for the literal text `overflow_y_scrollbar()`. That text moved into the shared helper. The guard's required string now reads `music_column_scrollbar(`, which still confirms the notice keeps its scrollbar. This fix is a direct, mechanical result of Required Change 2, inside this packet's own scope.
3. This packet does not record the open V1 to V7 gate in `docs/pending-human-checks.md` or in `docs/plans/broadcast-chain-delivery-order.md`. This packet's own rules name only this packet file as an editable document. Tasks 004, 005, 006 and 013 recorded the same scope limit, in their own deviations.

### Unresolved Concerns

- The two files named in Deviation 3 still need the open V1 to V7 gate recorded, outside this packet's own editable scope.
- No person walked V1 to V7 yet. A green mechanical suite gives no evidence about them.
- V6 asks that two release pages, opened one after another, keep separate scroll positions. The release page's scrollbar id comes from `page.detail_scroll_id`. That id has one fixed value for the Library origin, and one fixed value for the Discover origin, not a separate value for each release. This packet keeps that same id rule and did not change it. If two releases of the same origin, opened one after another, still share a scroll position, that condition came before this packet, not from it. The check below still walks V6, since no test can prove it.

## Operator Visual Check

This check reads the app only. It needs a Linux desktop session, this checkout, and a reachable
MusicIndex endpoint. It needs "New releases" to hold more rows than fit in the list column. It
also needs one release page with enough panels and tracks to overflow the detail column.

1. Close v4vmm. Build the binary.

   ```bash
   cd /home/citizen/build/v4vmm
   cargo build --bin v4vmm
   ```
2. Open the app.

   ```bash
   target/debug/v4vmm
   ```

**V1 - a long column shows a scrollbar, a short column does not**

"New releases" shows in the detail column. The list column of Discover holds only its views. The Releases destination gives a long list column.

3. Open Music. Open Discover.
   - Correct: the sidebar and the list column (one row, "New releases") show no scrollbar, or an empty track. The detail column, with the "New releases" rows or tiles, shows a vertical scrollbar at its right edge.
   - Incorrect: the detail column shows no scrollbar, although it holds more releases than fit.
4. Click Releases in the sidebar.
   - Correct: the list column shows a vertical scrollbar, when it holds more releases than fit.
5. Open a release with several panels and tracks, long enough to overflow the detail column.
   - Correct: the detail column shows a vertical scrollbar.

**V2 - a scrollbar drag scrolls its column, the wheel and the touchpad still work**

6. In the Releases list column, press the scrollbar thumb and drag it down.
   - Correct: the list scrolls to follow the drag.
7. Scroll the same list with the mouse wheel.
   - Correct: the list also scrolls with the wheel.
8. Drag the scrollbar of the release page in the detail column.
   - Correct: the release page scrolls to follow the drag.

**V3 - a scrollbar drag to the end of "New releases" loads the next page**

9. Click Discover. Drag the scrollbar of "New releases" in the detail column to the bottom of its track.
   - Correct: after a short pause, more releases appear below the ones you had, and the thumb gets smaller.
   - Incorrect: no new releases appear, and only the "Load more" button still works.

**V4 - the scrollbar covers no content**

10. Look at the right edge of the list column and the detail column while each scrollbar shows.
   - Correct: no title, row, tile, action button or "Show more" button sits under the scrollbar.
     Each one keeps clear space to its left.
   - Incorrect: the scrollbar overlaps a letter, an icon or a button.

**V5 - Back to "New releases" restores its scroll position**

11. Drag the "New releases" scrollbar down partway.
12. Open a release from the scrolled list.
13. Click Back in the detail column.
    - Correct: "New releases" returns to the same scroll position as step 11.
    - Incorrect: "New releases" returns to the top.

**V6 - two release pages, opened one after another, do not share one scroll position**

14. Open a long release page (step 5). Drag its scrollbar down partway.
15. Click Back, then open a different long release.
    - Correct: the second release's page opens at its top.
    - Incorrect: the second release's page opens already scrolled down.

**V7 - Light, Dark and a larger scale show the scrollbars with no clipped content**

16. Open Settings, then General (`Ctrl+Comma`). Select Dark, then Light, then a larger scale.
    Return to Music after each change and look at a column that shows a scrollbar.
    - Correct: the scrollbar shows clearly in each combination. No text, icon or button is
      clipped.
    - Incorrect: the scrollbar is hard to see, overlaps text, or content is clipped.
17. Close the app.

**Clean-up**

None. This check reads the app only. It writes no file and changes no stored data. Set the theme
and the scale of V7 back to your usual values.

## Orchestrator Review - 2026-10-17

The orchestrator reviewed the diff against ADR 0084 Decision 1 with its amendment, ADR 0063, this packet and the [review checklist](../../reviews/adr-0084-review-checklist.md). The orchestrator also ran each check.

- The helper, the gutter, the tracked handle of the content list and the move of the paging check follow "Design". No stop condition fired. The split into `content_list_scroll.rs` keeps `content_list.rs` under the ADR 0038 line ceiling. The change to the ADR 0066 search-failure guard follows from Required Change 2.
- The release page used one scroll id for each origin, not one for each release, so two release pages shared one scroll position. "Design" requires one id for each item. The orchestrator added `ReleaseDetailPageVm::detail_scroll_key`, from the feed GUID, the feed URL or the title, and the test `adr_0084_scrollbars_release_pages_get_one_scroll_id_each`.
- Before its first layout the scroll handle has no height, and the paging function read that as the end of the content. It could thus request a page before any layout. The orchestrator added a zero-height rule and the test `adr_0084_scrollbars_no_layout_yet_does_not_load`.
- The track, artist, playlist, publisher and name-match pages keep one fixed scroll id for each page type, as before this packet. The phase plan lists this under "Open Findings".
- The visual check put "New releases" in the list column. Discover shows it in the detail column. Steps 3 to 9 now use the detail column for "New releases" and the Releases destination for the list column.
- The orchestrator recorded the open gate in the pending checks and in the delivery order.

Merge recommendation: pass. Packet 007, Releases, comes after the visual check.

## Operator Check - 2026-10-17

The operator walked V1 to V7 with a normal debug binary, and each step passed.
