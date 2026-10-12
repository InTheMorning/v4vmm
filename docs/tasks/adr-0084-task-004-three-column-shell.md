# ADR 0084 Task 004: The Three-Column Shell

Status: Implementation recorded - 2026-10-12. The mechanical checks are Green. The operator visual check V1 to V6 is open.

## Goal

Music shows three columns side by side: the sidebar, the list and the detail. The list stays in view when a page opens.
Back, Forward and the breadcrumb move to the chrome of the detail column.

This packet builds the columns only. The sidebar keeps its present content. Task 005 adds the destinations and the sidebar that collapses to icons. Task 006 adds Discover.

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md) Decision 1, the column table, and Decision 9: Back and Forward show in the chrome of the detail column.
- [ADR 0046](../adr/0046-workspace-frame-architecture.md) as ADR 0084 amends it: the Music frames are `SourceList`, `ContentList` and `Detail`. Invariant 5: the frame shell is shared.
- [ADR 0081](../adr/0081-remove-the-staged-frame-model.md) Decision 2: a slot is added when a screen fills it. This packet fills the `SourceList` and `Detail` slots.
- The durable set in [AGENTS.md](../../AGENTS.md): renderer portability, token discipline, typed action state, and "Current-view state must update in place".

## Recorded Facts - 2026-10-12

- `TopApp::visible_workspace_layout` in `src/app.rs` keeps only the `ContentList` frame for each mount. `WorkspaceLayout::default_layout` in `src/view_models/workspace/mod.rs` has `SourceList` (id 1), `ContentList` (id 2) and `Detail` (id 3).
- `WorkspaceShell::render` in `src/ui/shells/workspace.rs` lays out the content-list frame, and a split with the Queue frame when one exists. It has no layout for `SourceList` or `Detail`. `WorkspaceSlots::take` gives `None` for both since ADR 0081 task 001.
- The Music mount puts the whole `LibraryApp` entity in the content-list frame (`render_workspace_screen_mount`). `LibraryApp`'s `Render` (`src/library/app_impl.rs`) builds a two-pane `SplitPane`:
  - The leading pane holds the feed status and actions, Playlists, Saved Searches and the artist tree (`render_library_sidebar` in `src/ui/shells/library/sidebar.rs`).
  - The content pane holds the content list (`render_library_content_list`) when `self.detail` is `LibraryDetail::None`, else the detail page (`render_library_detail`). The list and the page never show together.
- The top app renders its own pages in the content-list slot: the Index release and track pages, the publisher page, the name-match page and the search results (`render_workspace_content` in `src/app.rs`).
- The one navigation stack is the `FrameNavigationState` of the content-list frame. Each caller reads it through `content_list_frame_id`. The content-list frame chrome shows Back, Forward and the breadcrumb (task 001: `content_list_history_controls_always_shown`, `should_render_breadcrumb`).
- `src/ui/layouts.rs` holds `CONTENT_PANE_MIN_WIDTH` (320 px), the split tokens and two collapse breakpoints. It has no column width token.
- These guards pin the present layout: `adr_0060_music_surface_is_dominant_content_without_operational_panes` (the filter in `visible_workspace_layout`, and the content-or-detail branch in `LibraryApp`) and `adr_0046_library_split_uses_measured_shared_geometry` (two `SplitPane::new(chrome.split_pane_id)` sites).
- The prototype canvas column browser uses a list column of 320 px and a flexible detail column.

## Design

### Frames and slots

- For the Music mount, `visible_workspace_layout` keeps three frames in this order: `SourceList`, `ContentList`, `Detail`. For the Settings mount, it keeps the content-list frame only, as today.
- The navigation stack stays on the content-list frame id, so no caller changes. The workspace shell gives that stack to the chrome of the `Detail` frame. The `ContentList` and `SourceList` frames show no Back, no Forward and no breadcrumb in Music.
- The `Detail` frame title is the page title of `content_list_frame_title` today. The `ContentList` frame title is "Library" until task 005 gives each destination its title. The `SourceList` frame shows no header.
- The content-list frame keeps the filter control and the view-mode control.

### Columns from one Library entity

`LibraryApp` stays one entity with one state. It gets three render functions, one for each column. A small view type, `LibraryColumnView { library: Entity<LibraryApp>, column: LibraryColumn }`, renders one column through `library.update(...)`. It observes the Library entity, so a Library notify renders each column again. The top app keeps the three views and puts them in the three slots.

- Sidebar column: the present leading pane, unchanged.
- List column: the present content list, always. It no longer gives its place to the detail page.
- Detail column: the Library page when `self.detail` is not `LibraryDetail::None`. Otherwise an empty state, "Select an item in the list.", from a view model.
- A top-app page (Index release, Index track, publisher, name match) goes in the `Detail` slot.
- Search results go in the list column, with their filter chips on the content-list frame, while the location starts from a search. A result opens its page in the detail column. The operator decided this on 2026-10-12 (ADR 0084 Decision 2, and ADR 0081 Decision 4: no Detail filter strip). Otherwise the list column keeps the Library list.

The two-pane `SplitPane` of `LibraryApp` goes. The sidebar width is a token. This packet adds no drag resize.

### Widths and the narrow layout

New tokens in `src/ui/layouts.rs`:

| Token | Value |
|---|---|
| `MUSIC_SIDEBAR_WIDTH` | 220 px |
| `MUSIC_LIST_COLUMN_WIDTH` | 320 px |
| `MUSIC_DETAIL_MIN_WIDTH` | 360 px |
| `MUSIC_SINGLE_COLUMN_BREAKPOINT` | 940 px |

A pure function in `src/view_models/` gives the visible columns for a workspace width and a page state:

- At `MUSIC_SINGLE_COLUMN_BREAKPOINT` or wider: the sidebar, the list and the detail.
- Narrower: the sidebar and one more column. It is the detail when the location has a page, else the list. Back returns to the list.

Task 005 adds the icon-only sidebar to the narrow layout. The values are the orchestrator's choice on 2026-10-12. The operator can change them in the visual check.

## Required Changes

1. Add the four tokens. Add the column function and its type in `src/view_models/`, with no renderer type.
2. `visible_workspace_layout` keeps the three frames for Music and one frame for Settings.
3. `WorkspaceShell` lays out the Music frames as one row: the sidebar at its token width, the list at its token width, and the detail with the remaining width. It hides a column that the column function hides. It gives the content-list navigation to the `Detail` frame chrome, with Back and Forward always shown (task 001) and the breadcrumb of the location path.
4. Add `WorkspaceSlots` builders for the `SourceList` and `Detail` slots, and fill them in the Music mount.
5. Add `LibraryColumn`, `LibraryColumnView` and the three column render functions. Delete the two-pane `SplitPane` of `LibraryApp` and each item that only it used.
6. Move each top-app page from the content-list slot to the `Detail` slot.
7. Add the empty-state text to a view model.
8. Update the two guards in Recorded Facts. Delete the text that pins the replaced layout. Add the guards of the mechanical criteria.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_columns_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| C4-01 | The visible layout for Music has `SourceList`, `ContentList` and `Detail`, in that order. The visible layout for Settings has the content-list frame only |
| C4-02 | The column function shows three columns at the breakpoint and wider. Narrower, it shows the sidebar and the detail when a page is open, and the sidebar and the list when no page is open |
| C4-03 | The `Detail` frame chrome display exposes Back and Forward, always shown, with typed availability from the one stack. The `ContentList` frame chrome in Music exposes neither |
| C4-04 | The breadcrumb display attaches to the `Detail` frame, not to the `ContentList` frame |
| C4-05 | The empty detail state text comes from a view model |
| C4-06 | A guard names ADR 0084 Decision 1: the Library render has no `SplitPane`, and the Music column widths come from the named tokens |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: Music shows the sidebar, the list and the detail side by side. With no page open, the detail shows "Select an item in the list."
- V2: a row in the list opens its page in the detail. The list stays in view with the same scroll position.
- V3: Back, Forward and the breadcrumb show above the detail only.
- V4: search results show in the list column. A search result, a publisher page and a name-match page show in the detail.
- V5: at a width under 940 px, the list or the page shows, not both. Back from a page shows the list.
- V6: Light and Dark, and a larger scale, show no clipped column, header or control.

## Exclusions

- No destinations, no icon-only sidebar and no Discover. Tasks 005 and 006 own them.
- No change to the sidebar content, the content list rows, the scope control or the search results.
- No drag resize of a column.
- No change to Settings or Show.
- No schema change.

## Files To Inspect

- [Agent rules](../../AGENTS.md), the [source map](../architecture/source-map.md), the [phase plan](../plans/adr-0084-music-column-browser-phase-plan.md) and the archived [task 003](archive/adr-0084-task-003-one-track-route.md).
- `src/app.rs` (`render_workspace_content`, `render_workspace_screen_mount`, `visible_workspace_layout`, `content_list_frame_title`).
- `src/ui/shells/workspace.rs`, `src/ui/composites/frame_shell.rs`, `src/view_models/workspace/chrome.rs`.
- `src/library.rs`, `src/library/app_impl.rs` (`impl Render for LibraryApp`).
- `src/ui/shells/library/content_list.rs`, `detail.rs`, `sidebar.rs`.
- `src/ui/layouts.rs`.
- `tests/architecture_tests.rs`.

## Checks

```bash
cargo test --lib adr_0084_columns_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the working tree. This packet adds no migration and no stored data. The saved layout format does not change.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md`, `docs/architecture/source-map.md` and `docs/plans/adr-0084-music-column-browser-phase-plan.md`
- This packet: `docs/tasks/adr-0084-task-004-three-column-shell.md`
- ADR 0084 Decisions 1 and 9, ADR 0046 Invariant 5 and ADR 0081 Decision 2
- Each file in "Files To Inspect"

Goal:
- Music shows the three columns of "Design". Back, Forward and the breadcrumb show in the detail column chrome.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- View models carry no GPUI type. Each width comes from a named token.
- The Library keeps no navigation state. The guard `adr_0084_nav_library_screens_keep_no_navigation_stack` stays Green.
- Background work uses the ADR 0040 runtime. Never call `cx.spawn` from a screen.
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git add` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The sidebar content, the content list rows and the search result rows.
- The navigation model in `src/view_models/workspace/nav.rs`.
- The Settings and Show mounts.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case C4-01 to C4-06 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result" section and an "Operator Visual Check" section for V1 to V6. The check gives numbered steps, the expected result, an incorrect result, and the cleanup.

Test commands:
- Each command in "Checks".

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- `LibraryColumnView` cannot render a Library column through `Entity::update`, or a column does not render again after a Library notify.
- A Library listener or an input box stops working when it renders in a column view.
- A top-app page needs the list column, not the detail column.
- A change needs a file in "Do not touch".

## Implementation Result - 2026-10-12

Each Required Change is done.

### Files Changed

- `src/ui/layouts.rs`: four new tokens. `MUSIC_SIDEBAR_WIDTH` (220 px), `MUSIC_LIST_COLUMN_WIDTH`
  (320 px), `MUSIC_DETAIL_MIN_WIDTH` (360 px), `MUSIC_SINGLE_COLUMN_BREAKPOINT` (940 px). Deleted
  `SPLIT_STACKED_LEADING_FRACTION` and `SPLIT_STACKED_MIN_HEIGHT`, dead after the Library split
  removal.
- `src/view_models/workspace/columns.rs` (new): the pure enum `MusicColumns`, with `for_width`,
  `shows_list`, and `shows_detail`. It takes the width and the breakpoint in pixels as plain
  `f32`, so it holds no `gpui` type.
- `src/view_models/workspace/chrome.rs`: `FrameHistoryControls` gains the `Hidden` variant and
  `FrameShellDisplay::with_history_controls_hidden`. `back_shown`/`forward_shown` read the new
  variant.
- `src/view_models/workspace/mod.rs`: new `mod columns;` and the export `MusicColumns`.
- `src/view_models/workspace/tests.rs`: new tests for C4-02, C4-03, and C4-04, at the
  `FrameShellDisplay`/`MusicColumns` layer.
- `src/ui/shells/workspace.rs`: `WorkspaceSlots` gains `source_list_slot`, `detail_slot`,
  `detail_filter_chips`, `on_detail_filter_chips_select`, `detail_breadcrumb_labeler`,
  `on_detail_breadcrumb_select`, `on_detail_back_select`, and `on_detail_forward_select`. The
  removed `content_list_history_controls_always_shown` had no caller left after this change.
  The orchestrator later deleted `detail_filter_chips` and `on_detail_filter_chips_select` (Orchestrator Review item 1).
  - `WorkspaceShell::render` finds the Music three-column layout by a `SourceList` frame in the
  layout. It then projects the content-list frame's one navigation stack onto the Detail frame
  chrome. It shows Back and Forward always on Detail, and never on `SourceList`/`ContentList`.
  It lays the three columns out in one row, at their token widths, and hides a column that
  `MusicColumns` hides.
  - `should_render_breadcrumb` and `should_collapse_frame` take a new
  `music_three_column` flag.
  New tests cover C4-04 and the Detail width-collapse rule staying inert in Music.
- `src/library/column_view.rs` (new): `LibraryColumn` (`Sidebar`, `List`, `Detail`) and
  `LibraryColumnView`, a thin `Render` view that observes the Library entity and renders through
  `Entity::update`.
- `src/library.rs`: new `mod column_view;` and its export. Deleted the `split_pane_bounds` field,
  dead after the Library split removal.
- `src/library/app_impl.rs`: the former `impl Render for LibraryApp` body splits into
  `render_sidebar_column` (the present leading pane, unchanged), `render_list_column` (always the
  content list), `render_detail_column` (always `render_library_detail`, which gives the empty
  state itself when no page is open), and the dispatcher `render_column`. The two `SplitPane`
  sites, `on_pane_layout`, and the split-pane-only imports (`SplitPane`, `px`, `MouseDownEvent`,
  `MouseUpEvent`, `crate::ui::layouts as layout`) are deleted. The orchestrator deleted
  `impl Render for LibraryApp` in review (Orchestrator Review item 3).
- `src/view_models/library.rs`: deleted the `split_pane` field, `DEFAULT_SPLIT_PANE_WIDTH`, and
  the resize accessor methods (`is_resizing`, `split_pane_width`, `split_pane_height`,
  `resize_split_pane_height`, `begin_resize`, `end_resize`, `resize_split_pane`), each dead after
  the Library split removal. Deleted `LibraryChromeDisplay::split_pane_id` and `resize_handle_id`.
  `empty_detail_label` changes from "Select an item to view details" to "Select an item in the
  list.", and its test and the `chrome_display` test follow.
- `src/view_models/mod.rs`: deleted `SplitPaneState` and its two tests, dead after the Library
  split removal.
- `src/ui/composites/split_pane.rs`: deleted `SplitPane::fit_to`, `stacked_height`, `on_resize`,
  the `preferred_stacked_height` field, the `SplitPaneResize` enum, and the `FitTest` test
  (`adr_0046_split_fits_both_panes_and_restores_preferred_width`). After the Library split
  removal, nothing but that test called them. `show_log_pane.rs`'s own split, which uses `axis`,
  `on_layout`, and `on_resize_move`/`on_resize_end`/`on_resize_start`, is untouched.
- `src/app.rs`: `TopApp` gains `library_sidebar_view`, `library_list_view`, and
  `library_detail_view` (`Entity<LibraryColumnView>`), built in `new` right after the Library
  entity.
  - `visible_workspace_layout` uses the new `visible_music_layout` for the Music mount. It keeps
    `SourceList`, `ContentList` and `Detail`, in that order.
  - The list column title is "Library". The detail column title is the page title that
    `content_list_frame_title` gives.
  - In Music, `render_workspace_content` fills the detail slot with each top-app page: name match,
    publisher, and an Index release or track. After review, the search results fill the list
    column (Orchestrator Review item 1).
  - The sidebar and list slots fill with `library_sidebar_view` and `library_list_view`. The detail
    slot gets the same Back, Forward and breadcrumb handlers.
  - The new `library_or_screen_slot` helper gives the Library detail view in Music. New
  `#[cfg(test)] mod tests` covers C4-01.
- `tests/architecture_tests.rs`: deleted `adr_0046_library_split_uses_measured_shared_geometry`
  (it protected only the deleted Library split). Updated
  `adr_0060_music_surface_is_dominant_content_without_operational_panes` to check the three
  column functions in place of the deleted two-pane branch, naming ADR 0084 Decision 1. Updated
  `adr_0062_music_default_content_projects_recent_music_rows` the same way. Fixed the end markers
  of `adr_0075_feed_observation_roots_and_consumers_are_guarded` and
  `adr_0075_observation_writer_and_library_retention_have_one_owner`, which used the deleted
  `is_resizing(` as a text boundary. Added
  `adr_0084_columns_c4_06_music_columns_use_named_tokens_with_no_split_pane` for C4-06.

### Behavior Changed

- Music shows three columns side by side: the sidebar, the list, and the detail.
- The list column always shows the content list. It never gives its place to a page.
- The detail column shows the open page. With no page open, it shows "Select an item in the
  list."
- Back, Forward, and the breadcrumb show above the detail column only, with Back and Forward
  always shown. The sidebar and the list columns show neither.
- At a width under 940 px, the layout shows the sidebar and one more column. It shows the detail
  when a page is open, else the list. Back from a page returns to the list.
- The Library no longer carries a two-pane split or any resize state. The columns have no
  divider and no drag handle.

### Checks

| Command | Result |
|---|---|
| `cargo test --lib adr_0084_columns_` | Green, 9 tests |
| `cargo test` | Green after review: 1,869 unit tests and 307 guards |
| `cargo test --test architecture_tests` | Green after review: 307 tests |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

### Deviations From The Task

1. `impl Render for LibraryApp` stays. The packet asks to delete it if nothing mounts the entity
   directly. The existing guard `workspace_screen_mount_boundary_wraps_existing_screens_whole`
   (ADR 0046 Task 006a) requires the literal text
   `WorkspaceScreenMount::Music => self.library.clone().into_any_element()` in `src/app.rs`, which
   needs `LibraryApp: Render`. The orchestrator review reversed this deviation (item 3).
2. Deleted Library-split items beyond `src/library/app_impl.rs` and `src/view_models/library.rs`.
   These items had no caller after the split removal:
   - `SplitPane::fit_to`, `stacked_height`, `on_resize`, `preferred_stacked_height` and
     `SplitPaneResize` (`src/ui/composites/split_pane.rs`)
   - `SplitPaneState` (`src/view_models/mod.rs`)
   - the layout consts `SPLIT_STACKED_LEADING_FRACTION` and `SPLIT_STACKED_MIN_HEIGHT`

   AGENTS.md asks for dead code to be deleted, not parked. This packet deletes them
   and their tests, in the same change as the Library split.
3. Updated two guards the task does not name, beyond `adr_0060_...` and
   `adr_0046_library_split_uses_measured_shared_geometry`.
   `adr_0062_music_default_content_projects_recent_music_rows` and the `source_between` end
   markers in the two `adr_0075_...` guards broke as a direct, mechanical result of the Required
   Changes (the deleted two-pane branch text, and the deleted `is_resizing` function used as a
   text boundary). Fixing them is inside this packet's scope, since the break is a direct
   consequence of Required Change 5.
4. Added one guard the task does not name, `adr_0084_columns_c4_06_...`, so C4-06 has a mechanical
   proof beside the two updated guards.
5. `empty_detail_label` changes from "Select an item to view details" to "Select an item in the
   list.", to match the Design section and V1. The task names no exact string, but gives this
   text in Design and in V1's wording.
6. This packet does not edit `docs/pending-human-checks.md` or
   `docs/plans/broadcast-chain-delivery-order.md`. AGENTS.md asks for the open visual gate to go
   in both files. This task's own rules name only this packet file as an editable document, the
   same scope task 003 records in its own Deviation 5. The open V1 to V6 gate still needs a
   record in both files.

### Unresolved Concerns

- None open. The orchestrator corrected the ADR 0081 status line in review (Orchestrator Review item 5).

## Operator Visual Check

This check reads the app only. It writes no file. It needs a Linux desktop session, this
checkout, and a Library with one playlist, one album and one track, or more.

1. Close v4vmm. Make the binary.

   ```bash
   cd /home/citizen/build/v4vmm
   cargo build --bin v4vmm
   ```
2. Open the app.

   ```bash
   target/debug/v4vmm
   ```
3. Open Music.

**V1 - three columns, with the empty detail state**

4. Look at the Music window before any click.
   - Correct: three columns show side by side. The left column is the library tree, playlists,
     and saved searches. The middle column is the content list. The right column shows "Select
     an item in the list."
   - Incorrect: one or two columns show, or the right column is empty or shows the content list.

**V2 - a list row opens its page in the detail column, and the list stays in view**

5. Scroll the middle (list) column partway down. Click a track row.
   - Correct: the right column shows the track page. The middle column stays in view, at the same
     scroll position.
   - Incorrect: the middle column disappears, scrolls back to the top, or shows the track page
     itself.
6. Click a different track row further down the same list.
   - Correct: the right column changes to the new track. The middle column does not move.

**V3 - Back, Forward, and the breadcrumb show above the detail column only**

7. Look at the header above each of the three columns.
   - Correct: the left and middle columns show no Back, no Forward, and no breadcrumb. The right
     column shows Back and Forward, each as available or disabled, and the breadcrumb once more
     than one page is open.
   - Incorrect: Back, Forward, or the breadcrumb show above the middle column, or do not show
     above the right column.
8. Click a playlist in the left column, then a track in that playlist.
   - Correct: the right column's breadcrumb reads "Library › playlist name › track title". Back
     returns to the playlist page in the right column, with the middle column unchanged.

**V4 - search results show in the list column, and their pages show in the detail column**

9. Use the toolbar search for a track or release not in your Library.
   - Correct: the search results, with their tabs and their All, Library and Index chips, show in
     the middle column. The right column shows "Select an item in the list."
   - Incorrect: the search results show in the right column, or a control in the middle column is
     clipped so that you cannot use it.
10. Open one search result, then open its artist or publisher name link.
    - Correct: the track or release page, then the publisher or name-match page, each show in the
      right column in turn. The middle column keeps the search results.
11. Click Back until the search results have no open page, then click a Library row in the sidebar.
    - Correct: the middle column shows the Library list again.

**V5 - a narrow width shows the sidebar and one more column**

12. Resize the window to under 940 px wide, with no page open.
    - Correct: the window shows the sidebar and the list column only. The detail column is gone.
13. Click a track row.
    - Correct: the window shows the sidebar and the detail column only, with the open track. The
      list column is gone.
14. Click Back in the detail column.
    - Correct: the window returns to the sidebar and the list column.
15. Widen the window back past 940 px.
    - Correct: all three columns show again.

**V6 - Light, Dark, and a larger scale show no clipped column, header, or control**

16. Open Settings -> General (`Ctrl+Comma`). Select Light, then Dark, then a larger scale. Return
    to Music after each change, with a page open in the detail column.
    - Correct: all three columns, their headers, and their controls stay fully visible, with no
      clipped text, icon, or control, in every combination.
    - Incorrect: a column, a header, or a control is clipped, cut off, or overlaps another.
17. Close the app.

**Clean-up**

None. This check reads your Library. It creates no fixture, and it writes no file.

## Orchestrator Review - 2026-10-12

The orchestrator reviewed the diff against ADR 0084, ADR 0081, this packet and the [review checklist](../reviews/adr-0084-review-checklist.md). The column layout, the column views and the detail chrome keep to the design.

Five points were corrected in review:

1. The search results filled the detail column. Their filter chips moved to the `Detail` frame chrome through two new builders, `detail_filter_chips` and `on_detail_filter_chips_select`.

   ADR 0081 Decision 4 requires a new ADR for a Detail filter strip. The new names also passed the ADR 0081 guard, which looks for `detail_filter_chip_strip`. The orchestrator's design was the cause.

   The operator decided on 2026-10-12 that the search results go in the list column now (ADR 0084 Decision 2). The new helper `with_search_results_list` fills the list column while the location starts from a search. The two builders are deleted, and the ADR 0081 guard now blocks their names.
2. The narrow layout counted a search as an open page, so it hid the search results. The new `FrameNavigationEntry::opens_detail_page` names the list roots, and the test `adr_0084_columns_list_roots_open_no_detail_page` proves it.
3. `impl Render for LibraryApp` and a whole-Library fallback stayed only because two guards named the literal `WorkspaceScreenMount::Music => self.library.clone().into_any_element()`. ADR 0084 Decision 1 replaces that whole-screen Music mount. Both are deleted. A non-Music path gets empty slots. The two guards now require the column views, and keep their Show and Settings requirements.
4. `adr_0062_music_default_content_query_stays_off_render_path` used `impl Render for LibraryApp` as a start marker. It now reads from `render_sidebar_column`.
5. The status line of ADR 0081 broke `adr_0057_status_headers_are_canonical`. The orchestrator wrote that line when it closed ADR 0081 task 001, after its last test run. The line now follows the ADR 0057 format. This packet did not cause it.

After the corrections, each check is Green: 1,869 unit tests, 307 guards, `fmt`, `clippy -D warnings`, `check --all-targets` with no warning, and `cargo build --bin v4vmm`.

Merge recommendation: pass. Task 005 adds the destinations and uses `opens_detail_page` for its destination roots.
