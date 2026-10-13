# ADR 0084 Task 005: The Destinations Sidebar

Status: Implementation recorded - 2026-10-12. The mechanical checks are Green. The operator visual check V1 to V7 is open.

## Goal

The Music sidebar lists six destinations: Discover, Releases, Artists, Tracks, Needs attention and Playlists. A click on a destination fills the list column with its items, and a row opens its page in the detail column.
The sidebar collapses to icons. Needs attention shows its count, and the collapsed icon shows a mark. The sidebar tree of artists, albums and tracks goes.

This packet fixes finding 5 of the [Music flow map](../architecture/music-flow-map.md): Music reaches the tracks that need attention without Show.

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md) Decision 1 (the sidebar, its icons, tooltips, accessibility labels and attention mark) and Decision 2 (the destinations and their order).
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 9: a sidebar click starts a new path, with the destination first.
- The durable set in [AGENTS.md](../../AGENTS.md): token discipline, typed action state, never rely on color alone, and "Current-view state must update in place".

## Recorded Facts - 2026-10-12

- [Task 004](archive/adr-0084-task-004-three-column-shell.md) built the three columns. `LibraryApp::render_sidebar_column` in `src/library/app_impl.rs` renders the feed status and actions, Playlists (sort, "+", the inline new-playlist input), Saved Searches and the tree (`render_library_sidebar` in `src/ui/shells/library/sidebar.rs`).
- The list column always shows the content list (`render_library_content_list`). `ContentListPageVm` in `src/view_models/library.rs` has three sources: `Tree` (Library tracks), `RecentMusic` (recent Index releases, paged, ADR 0062) and `BroadcastReadiness` (the readiness list).
- The location root of a sidebar path is `FrameNavigationEntry::SourceList`, labeled "Library". The readiness list root is `FrameNavigationEntry::ReadinessIssues`. About 20 places use the two entries.
- The readiness list opens only from Show: `open_broadcast_readiness_in_music` in `src/app/show.rs`. `LibraryApp::refresh_current_broadcast_readiness_report` already builds the report from the Library connection and `music_dir`, with no Show dependency. `BroadcastReadinessReport::problem_count` gives the count.
- `IconName` in `src/ui/icons.rs` has `Warning` (Lucide `triangle-alert`). The Lucide catalog of `gpui_kit_assets` has `compass`, `disc`, `users`, `music` and `list-music` (ADR 0083 Decision 10).
- `Button` has `.tooltip(...)` and `.a11y_label(...)` (`src/ui/primitives/button.rs`).
- `FrameNavigationEntry::opens_detail_page` (task 004) names the list roots for the narrow layout.
- The prototype canvas shows the destinations with icons. Needs attention and Playlists show a count at the right. The collapsed Needs attention icon shows an orange mark. A collapse button is at the top, and the feed state with "Check all feeds" is at the bottom.

## Design

### Destinations and the navigation root

```rust
/// A Music destination (ADR 0084 Decision 2), in sidebar order.
pub(crate) enum MusicDestination { Discover, Releases, Artists, Tracks, NeedsAttention, Playlists }
```

- `FrameNavigationEntry::Destination(MusicDestination)` replaces `SourceList` and `ReadinessIssues`. Its label is the destination name. `opens_detail_page` is `false` for it.
- A sidebar click starts a new path with the destination only: `start_path(Destination(d), None)`. A row opens its page in the current path, for example "Releases › Selected Ambient Works".
- Music opens on `Destination(Discover)`. The readiness card of Show opens `Destination(NeedsAttention)`.
- The content-list frame title is the destination name. A search location keeps its present title.

### The list column for each destination

| Destination | List column | A row opens |
|---|---|---|
| Discover | The present recent Index releases (`RecentMusic`), with its view-mode control and its present filter control. Task 006 replaces it | `Release` |
| Releases | The Library releases, by name | `Release` |
| Artists | The Library artists, by name | `ArtistDetail` |
| Tracks | The Library tracks (`Tree`). No filter control | `Track` |
| Needs attention | The readiness list (`BroadcastReadiness`), with its row actions | `Track` |
| Playlists | The playlists, with sort, "+" and the inline new-playlist input. `cmd-n` still works | `PlaylistDetail` |

Each row uses `ListRow` and the status line of task 004. A view model gives each list. The view model gives the rows and their labels. The renderer decides nothing.

### The sidebar

- A collapse button at the top, then the six destinations, then the feed area at the bottom: the status line, "Check all feeds" or "Apply updates (n)", "Update n files" and the tag write report.
- Saved Searches stay below the destinations until task 006 moves them to Discover. The collapsed sidebar does not show them.
- Each destination is a button with a named icon, its label, and a count when it has one. Needs attention shows its problem count. Playlists shows the playlist count.
- The selected destination is the root of the current location.
- New icons: `Discover` (compass), `Release` (disc), `Artist` (users), `Track` (music), `Playlist` (list-music). Needs attention uses `Warning`.

### The collapsed sidebar

- A pure function gives the sidebar state from a preference and the workspace width. The preference is `Auto`, `Collapsed` or `Expanded`. `Auto` collapses under `MUSIC_SIDEBAR_COLLAPSE_BREAKPOINT`. The collapse button sets `Collapsed` or `Expanded`, so the operator can change the sidebar at any width. The preference lives for the session only.
- Collapsed, the sidebar is `MUSIC_SIDEBAR_COLLAPSED_WIDTH` wide. Each destination shows its icon only, with a tooltip and an accessibility label. The feed area does not show.
- When Needs attention has a count, the collapsed icon shows a mark. The tooltip and the accessibility label give the count, for example "Needs attention, 2 tracks". The mark never relies on color alone.

New tokens: `MUSIC_SIDEBAR_COLLAPSED_WIDTH` (56 px) and `MUSIC_SIDEBAR_COLLAPSE_BREAKPOINT` (1100 px). The values are the orchestrator's choice on 2026-10-12. The operator can change them in the visual check.

### The Needs attention count

The Library computes the readiness report on load, after a feed check or a repair, and after a download or a removal. It stores the count in the view model. The render path makes no database query.

## Required Changes

1. Add `MusicDestination`, its labels and its icons. Add `FrameNavigationEntry::Destination`. Delete `SourceList` and `ReadinessIssues`, and change each use.
2. Add the destination list view models and their renderers, as the table gives. The list column renders the list of the current root destination, or the search results while the location starts from a search.
3. Rebuild `render_sidebar_column` as "The sidebar" gives. Delete the tree: `render_library_sidebar`, the tree expansion state and each item that only the tree used.
4. Add the sidebar preference, the pure sidebar state function and the collapse button with its typed action, label and tooltip. Give the sidebar width to the workspace shell from that state.
5. Compute and store the Needs attention count. Show it in the expanded and the collapsed sidebar.
6. Change "Open in Music" in Show to open `Destination(NeedsAttention)`.
7. Update the guards that name the replaced entries or the tree. Delete the text that asserts a replaced rule. Add the guards of the mechanical criteria.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_destinations_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| D5-01 | The view model gives the six destinations in the order of Decision 2. Each has a label, a named icon, a tooltip and an accessibility label |
| D5-02 | A sidebar click gives the path with the destination only. A row of a destination list gives the path with the destination and the page |
| D5-03 | Music opens on `Destination(Discover)`. The Show readiness entry opens `Destination(NeedsAttention)` |
| D5-04 | Each destination list view model gives its rows and the entry that each row opens |
| D5-05 | The sidebar state function gives Expanded and Collapsed for `Auto` at each side of the breakpoint, and keeps `Collapsed` and `Expanded` at each width |
| D5-06 | With a Needs attention count, the collapsed destination exposes a mark, and its tooltip and its accessibility label give the count |
| D5-07 | The collapse button has a typed action, an accessibility label and a tooltip, each from the view model |
| D5-08 | A guard names ADR 0084 Decision 2: no `SourceList`, no `ReadinessIssues` and no `render_library_sidebar` in `src/` |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: the sidebar shows the six destinations in order, each with its icon. Discover is selected when Music opens.
- V2: each destination fills the list column with its items, and a row opens its page in the detail column. The breadcrumb starts with the destination name.
- V3: Needs attention shows its count, and opens its list with no visit to Show.
- V4: the collapse button collapses the sidebar to icons and expands it again. Each icon shows its tooltip. The Needs attention mark shows when the count is not zero.
- V5: under 1100 px the sidebar collapses, and the button still expands it.
- V6: Playlists creates, renames and opens a playlist as before, with "+" and with `cmd-n`.
- V7: Light and Dark, and a larger scale, show no clipped label, icon or count.

## Exclusions

- No Discover views and no "New releases" page. Task 006 owns them.
- No release kind, no scope control and no singles groups. Packet 007 owns them.
- No change to the artist page. Packet 008 owns it.
- No change to the search results view.
- No schema change.

## Files To Inspect

- [Agent rules](../../AGENTS.md), the [source map](../architecture/source-map.md), the [phase plan](../plans/adr-0084-music-column-browser-phase-plan.md) and the archived [task 004](archive/adr-0084-task-004-three-column-shell.md), with its Orchestrator Review.
- `src/view_models/workspace/nav.rs`, `columns.rs`, `breadcrumb.rs`, `tests.rs`.
- `src/app.rs`, `src/app/show.rs`, `src/app/breadcrumb.rs`.
- `src/library.rs`, `src/library/app_impl.rs`, `src/library/column_view.rs`.
- `src/view_models/library.rs`, `src/ui/shells/library/sidebar.rs`, `content_list.rs`.
- `src/ui/shells/workspace.rs`, `src/ui/layouts.rs`, `src/ui/icons.rs`, `src/ui/primitives/button.rs`.
- `src/application/queries/broadcast.rs`.
- `tests/architecture_tests.rs`.

## Checks

```bash
cargo test --lib adr_0084_destinations_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md`, `docs/architecture/source-map.md` and `docs/plans/adr-0084-music-column-browser-phase-plan.md`
- This packet: `docs/tasks/adr-0084-task-005-destinations-sidebar.md`
- ADR 0084 Decisions 1, 2 and 9
- The archived task 004 packet, with its Orchestrator Review
- Each file in "Files To Inspect"

Goal:
- The sidebar, the destination lists and the collapsed sidebar of "Design".

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- View models carry no GPUI type. Each width, icon and color comes from a named token.
- The Library keeps no navigation state. It sends `LibraryAppEvent::OpenPage`. The top app owns the stack.
- The render path makes no database query.
- Keep each scrolling column a flex column with `min_h_0`. A row in a scrolling list keeps `flex_shrink_0` (task 004 review, items 8 and 9).
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git add` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The search results view and its rows.
- The release, track, artist and playlist pages.
- The Settings and Show screens, except "Open in Music".
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case D5-01 to D5-08 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result" section and an "Operator Visual Check" section for V1 to V7. The check gives numbered steps, the expected result, an incorrect result, and the cleanup.

Test commands:
- Each command in "Checks".

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- The Needs attention count needs a database query in the render path.
- A Lucide icon of "Design" is not in the catalog.
- A guard asserts a rule of an ADR that this packet does not change.
- A change needs a file in "Do not touch".

## Implementation Result - 2026-10-12

Each Required Change is done.

### Files Changed

- `src/view_models/workspace/nav.rs`: new `MusicDestination` enum, in sidebar order, with
  `ALL` and `label()`. `FrameNavigationEntry::Destination(MusicDestination)` replaces
  `SourceList` and `ReadinessIssues`. `opens_detail_page` and `display_label` read the new
  variant. New `FrameNavigationState::root()` gives the first ancestor, or the current page
  when the path has none, for a frame title to read.
- `src/view_models/workspace/columns.rs`: new `MusicSidebarPreference` (`Auto`, `Collapsed`,
  `Expanded`), `MusicSidebarState` (`Expanded`, `Collapsed`) with `toggled()`, and the pure
  function `resolve_music_sidebar_state(preference, width, breakpoint)`. New tests for D5-05
  and D5-07.
- `src/view_models/workspace/mod.rs`: exports the new types. `default_navigation_entry` gives
  `Destination(MusicDestination::Discover)` for the `SourceList` and `ContentList` frames.
- `src/view_models/workspace/breadcrumb.rs`: new `destination_id()` gives each destination its
  breadcrumb slug, for example `"destination-discover"`.
- `src/view_models/workspace/tests.rs`: each `SourceList` fixture becomes
  `Destination(MusicDestination::Discover)`. New tests for D5-02 and D5-03, and the
  `ReadinessIssues` test now covers `Destination(NeedsAttention)`.
- `src/ui/icons.rs`: five new `IconName` variants, `Discover`, `Release`, `Artist`, `Track`,
  and `Playlist`. Each resolves through the Lucide catalog (`Compass`, `Disc`, `Users`,
  `Music`, `ListMusic`), added to the `icon_assets!` macro call. Updated tests.
- `src/ui/layouts.rs`: two new tokens, `MUSIC_SIDEBAR_COLLAPSED_WIDTH` (56 px) and
  `MUSIC_SIDEBAR_COLLAPSE_BREAKPOINT` (1100 px).
- `src/view_models/library.rs`: the largest change.
  - New types: `MusicDestinationIcon`, `DestinationRowVm`, `SidebarCollapseButtonVm`,
    `DestinationListRowTarget`, `DestinationListRowVm`.
  - New `LibraryViewModel` fields: `sidebar_preference`, `active_destination` (defaults to
    `Discover`), `needs_attention_count`.
  - New methods: `show_tracks_destination`, `sidebar_preference`/`set_sidebar_preference`,
    `active_destination`/`set_active_destination`, `show_needs_attention_report`,
    `refresh_needs_attention_report`, `destination_rows`, `destination_row`,
    `destination_count`, `sidebar_collapse_button`, `releases_list`, `artists_list`.
  - Deleted the Library tree projection and its display types: `LibraryTreeProjection`,
    `ArtistNode`/`AlbumNode` tree display, `LibraryArtistTreeDisplay`,
    `LibraryAlbumTreeDisplay`, and `LibraryTreeTrackDisplay`, with `tree_projection`,
    `tree_number_prefix`, and `filter_tree`.
  - Deleted the tree toggle state: `expanded_artists`, `expanded_albums`, `toggle_artist`,
    `toggle_album`, `is_artist_expanded`, and `is_album_expanded`.
  - Deleted the search-query filter path: `search_query`, `set_source_text_filter`, and
    `apply_search_query`.
  - Deleted the content-list empty state: `content_filter_empty_state`,
    `should_show_empty_library`, and `ContentListPageVm::empty_state`. Each item had no caller
    left after the tree removal.
  - The hover-thumbnail display struct lost its last caller when the tree left, for a short
    time. It came back as a thin method pair, when the destination lists needed the same hover
    behavior.
  - `PlaylistSidebarVm` loses its `expanded` field. The Playlists list has no disclosure toggle.
  - `LibraryChromeDisplay` loses `list_scroll_id` and `empty_library_label`. Their one caller
    was the deleted tree sidebar.
  - Many new tests with the `adr_0084_destinations_d5_` prefix, for D5-01, D5-04, D5-06, and
    D5-07.
- `src/library/app_impl.rs`: the second-largest change.
  - `render_sidebar_column` is rewritten: the collapse button, the six destination rows, then,
    when expanded, Saved Searches and the feed area, in a scrolling flex column.
  - `render_list_column` matches on `active_destination()`. `Discover`, `Tracks`, and
    `NeedsAttention` keep the present content-list path. `Releases` and `Artists` call the new
    view-model list methods and `render_library_destination_rows`. `Playlists` calls
    `render_library_playlists_list`.
  - `hydrate_detail_from_nav` opens a destination's list and clears the detail column for
    `FrameNavigationEntry::Destination`.
  - New `show_destination_list`, `fetch_broadcast_readiness_report`,
    `open_needs_attention_destination`, `sidebar_state`, `sidebar_preference`,
    `active_destination`, `toggle_sidebar_collapse`, `open_destination_row`, and
    `toggle_creating_playlist`.
  - `refresh_current_broadcast_readiness_report` now always runs the readiness query. This keeps
    the Needs attention count fresh. Row replacement stays with
    `refresh_needs_attention_report`, which replaces rows only while that list is open. The
    query now also runs on load: `start_async_reload_with_mode`'s success path calls
    `refresh_current_broadcast_readiness_report` after each reload finishes.
  - `cycle_playlist_sort` and `create_playlist` become `pub(crate)`, for the new
    `destination_lists.rs` module to call.
  - `render_library_sidebar` and the tree toggle wrappers are deleted.
- `src/ui/shells/library/sidebar.rs`: rewritten. It renders the collapse button and each
  destination row only. `icon_name` maps a `MusicDestinationIcon` to a named `IconName`. The
  attention mark is a colored dot beside the icon, and never the only signal: the row's
  tooltip and accessibility label already give the count in text.
- `src/ui/shells/library/destination_lists.rs` (new file): renders the Releases, Artists, and
  Playlists lists. `render_library_destination_rows` renders a release or artist row list, with
  an empty message when a list has no rows. `render_library_playlists_list` moves the sidebar's
  sort button, add button, inline new-playlist input, and playlist rows here, unchanged in
  behavior.
- `src/ui/shells/library/mod.rs`: adds `pub mod destination_lists;`.
- `src/ui/shells/workspace.rs`: `WorkspaceSlots` gains `source_list_sidebar_preference`.
  `WorkspaceShell::render` resolves the sidebar state from the preference and the workspace
  width, and sizes the sidebar column at `MUSIC_SIDEBAR_WIDTH` or
  `MUSIC_SIDEBAR_COLLAPSED_WIDTH`.
- `src/app.rs`: new `music_list_frame_title`, which reads a frame's navigation root and gives
  the destination's label, or "Search Results" for a search. `open_music_page` and `select_tab`
  read `Destination` in place of `SourceList`/`ReadinessIssues`. `render_workspace_content`
  passes the sidebar preference to the workspace shell. It shows the view-mode control for
  `Discover`, `Tracks`, and `NeedsAttention`. It shows the library filter control for
  `Discover` and `NeedsAttention` only.
- `src/app/show.rs`: `open_broadcast_readiness_in_music` opens
  `Destination(MusicDestination::NeedsAttention)`. `music_readiness_list_is_current` checks the
  same variant.
- `src/ui/composites/breadcrumb_trail.rs`: test fixtures use
  `Destination(MusicDestination::Discover)` in place of `SourceList`.
- `tests/architecture_tests.rs`: updated each guard that named `SourceList`, `ReadinessIssues`,
  the deleted tree, or the deleted sidebar text markers
  (`adr_0061_root_content_frame_header_omits_redundant_section_title`,
  `adr_0065_feed_check_result_has_its_own_full_width_row`,
  `adr_0071_selection_and_primary_stay_in_shared_owners`,
  `adr_0084_columns_c4_06_music_columns_use_named_tokens_with_no_split_pane`,
  `adr_0084_columns_music_column_wrappers_are_flex_columns`,
  `nav_top_drives_content_list_body_switch`, `playlist_rows_scale_through_design_tokens`,
  `playlist_refresh_and_frame_navigation_preserve_context`,
  `adr_0047_task_010a_content_list_page_vm_owns_filter_projection`,
  `adr_0047_task_010_content_list_library_filter_is_frame_local`). New
  `adr_0084_destinations_d5_03_show_opens_needs_attention_destination` and
  `adr_0084_destinations_d5_08_source_list_and_readiness_issues_stay_deleted`, which names ADR
  0084 Decision 2 and blocks `SourceList`, `ReadinessIssues`, and `render_library_sidebar` from
  returning to `src/`.

### Behavior Changed

- The Music sidebar shows six destinations, Discover, Releases, Artists, Tracks, Needs
  attention, and Playlists, each with a named icon. The Library tree of artists, albums, and
  tracks is gone.
- A destination click starts a new path at that destination. The list column fills with that
  destination's items. A row opens its page in the detail column, on the current path.
- The sidebar collapses to icons, by a width under 1100 px or by the collapse button. Each icon
  keeps its tooltip and its accessibility label. The preference lives for the session.
- Needs attention shows its problem count, in the sidebar and on the collapsed icon's mark. The
  count stays current after a feed check, a repair, a download, a removal, or a library
  reload, with no visit to Show.
- Playlists keeps its sort button, its "+" button, its inline new-playlist input, and `cmd-n`,
  now in the list column, not in the sidebar.
- Releases and Artists each show their own name-sorted list in the list column. A release row
  opens the release page. An artist row opens the artist page.
- Show's "Open in Music" opens the Needs attention destination directly.

### Checks

| Command | Result |
|---|---|
| `cargo test --lib adr_0084_destinations_` | Green, 10 tests |
| `cargo test` | Green: 1,866 unit tests and 311 guards |
| `cargo test --test architecture_tests` | Green, 311 tests |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

### Deviations From The Task

1. `refresh_current_broadcast_readiness_report` now runs its query on every reload, not only
   while the Needs attention list is open. "Recorded Facts" and "The Needs attention count"
   ask for a current count. It must stay current after a load, a feed check, a repair, a
   download, or a removal. The gate for this moved from the query itself to the
   row-replacement step inside `refresh_needs_attention_report`. The task's wording does not
   name this split, but the Design section needs the resulting behavior.
2. Deleted several items beyond the ones Required Change 3 names, each with no caller left
   after the tree and search-query removal: `content_filter_empty_state`,
   `should_show_empty_library`, `ContentListPageVm::empty_state`, `search_query`,
   `set_source_text_filter`, `apply_search_query`, `LibraryChromeDisplay::list_scroll_id`, and
   `LibraryChromeDisplay::empty_library_label`. AGENTS.md asks for dead code to be deleted, not
   parked, in the same change as the cause.
3. Updated guards the task does not name, beyond the ones in Required Change 7:
   `adr_0047_task_010a_content_list_page_vm_owns_filter_projection`,
   `adr_0047_task_010_content_list_library_filter_is_frame_local`,
   `adr_0071_selection_and_primary_stay_in_shared_owners`,
   `adr_0084_columns_c4_06_music_columns_use_named_tokens_with_no_split_pane`, and
   `adr_0084_columns_music_column_wrappers_are_flex_columns`. Each broke as a direct, mechanical
   result of the Required Changes (a deleted method, a deleted file, or a new `sidebar_width`
   expression in place of the fixed `MUSIC_SIDEBAR_WIDTH` token). Fixing them stays inside this
   packet's scope, since each break is a direct consequence of a Required Change.
4. This packet does not edit `docs/pending-human-checks.md` or
   `docs/plans/broadcast-chain-delivery-order.md`. AGENTS.md asks for the open visual gate to go
   in both files. This task's own rules name only this packet file as an editable document.
   Task 004 recorded the same scope limit, in its own Deviation 6. The open V1 to V7 gate
   still needs a record in both files.

### Unresolved Concerns

- The two files named in Deviation 4 still need the open V1 to V7 gate recorded, outside this
  packet's own editable scope.
- I ran `cargo clippy --all-targets -- -D warnings` beyond the packet's own check, as an extra
  diligence pass. It surfaces pre-existing clippy findings in test code this packet did not
  touch, in files such as `log_view.rs`, `playlist_detail.rs`, `publisher_page.rs`, and
  `track_detail.rs`. These findings exist before this packet's changes and are outside its
  scope. The packet's own named check, `cargo clippy -- -D warnings`, stays Green.

## Operator Visual Check

This check reads the app and writes two playlists. The clean-up removes them. It needs a Linux
desktop session and this checkout. It needs a Library with one release, one artist,
and one track. It needs the Needs attention report to show one problem track or more. Run a
feed check or a repair first if Needs attention shows no count.

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

**V1 - six destinations, in order, with Discover selected**

4. Look at the sidebar before any click.
   - Correct: the sidebar lists Discover, Releases, Artists, Tracks, Needs attention, and
     Playlists, in that order, each with its own icon. Discover shows as selected.
   - Incorrect: a destination is missing, out of order, or shown with no icon. No destination
     shows as selected.

**V2 - each destination fills the list column, and a row opens its page**

5. Click Releases.
   - Correct: the list column shows releases by name. The breadcrumb above the detail column
     starts with "Releases".
   - Incorrect: the list column keeps showing the Discover content, or the breadcrumb does not
     start with "Releases".
6. Click a release row.
   - Correct: the detail column shows that release's page. The list column stays in view.
7. Click Artists, then an artist row.
   - Correct: the list column shows artists by name, then the artist's page opens in the
     detail column. The breadcrumb starts with "Artists".
8. Click Tracks.
   - Correct: the list column shows the full library track list. The breadcrumb starts with
     "Tracks".

**V3 - Needs attention opens with no visit to Show**

9. Click Needs attention in the sidebar.
   - Correct: the list column shows the readiness report rows with no stop at Show. The
     sidebar count on Needs attention matches the number of rows shown. The breadcrumb starts
     with "Needs attention".
   - Incorrect: the app requires a visit to Show first, or the sidebar count does not match
     the row count.

**V4 - the collapse button, tooltips, and the attention mark**

10. Click the collapse button at the top of the sidebar.
    - Correct: the sidebar narrows to icons only. Saved Searches and the feed area disappear.
      Hovering each icon shows its tooltip, with its label, and its count for Needs attention
      and Playlists.
    - Incorrect: a destination loses its tooltip, or the sidebar keeps its labels.
11. With Needs attention showing a count above zero, look at its collapsed icon.
    - Correct: the icon shows a small mark. The tooltip text still states the count in words,
      for example "Needs attention, 2 tracks".
    - Incorrect: the mark is the only signal of a problem, with no matching tooltip or
      accessibility text.
12. Click the collapse button again.
    - Correct: the sidebar returns to its full width, with labels and the feed area back.

**V5 - a width under 1100 px collapses the sidebar**

13. Resize the window to under 1100 px wide.
    - Correct: the sidebar collapses to icons on its own. The collapse button still expands it
      at this width.
14. Widen the window back past 1100 px.
    - Correct: the sidebar returns to its expanded state, unless you had set Collapsed by hand
      in step 10.

**V6 - Playlists creates, renames, and opens a playlist**

15. Click Playlists in the sidebar.
    - Correct: the list column shows the sort button, the "+" button, and each playlist row,
      each as before this change.
16. Click "+". Type a name, for example "V1 to V7 check", and press Enter, or click its Add
    button. Press `ctrl-n` (`cmd-n` on macOS) and create a second playlist the same way.
    - Correct: each new playlist appears in the list, and opens in the detail column.
17. Click a playlist row to open its page. Rename it from its page, as before this change.
    - Correct: the rename takes effect, and the list column shows the new name.

**V7 - Light, Dark, and a larger scale show no clipped label, icon, or count**

18. Open Settings -> General (`Ctrl+Comma`). Select Light, then Dark, then a larger scale.
    Return to Music after each change, with the sidebar expanded and a destination's list
    open.
    - Correct: each destination's icon, label, and count stay fully visible, with no clipped
      text or icon, in every combination.
    - Incorrect: a label, an icon, or a count is clipped, cut off, or overlaps another.
19. Close the app.

**Clean-up**

Delete each playlist step 16 created ("V1 to V7 check" and the `ctrl-n` playlist). Open each
playlist's page and use its delete action, as for any other playlist. This check creates no
other file and changes no other stored data.

## Orchestrator Review - 2026-10-12

The orchestrator reviewed the diff against ADR 0084, this packet and the [review checklist](../reviews/adr-0084-review-checklist.md), and ran each check. Each check is Green: 1,866 unit tests, 311 guards, `fmt`, `clippy -D warnings`, `check --all-targets` with no warning, and `cargo build --bin v4vmm`.

- The destination model, the root entry, the lists, the collapsed sidebar and the count keep to the design. No item is renamed to pass a guard.
- The new lists use `ListRow` in flex columns with `min_h_0`, so the scroll rules of the task 004 review hold.
- The deleted Library search filter served only the tree. No other path called it.
- Deviation 1 is accepted: the readiness report now runs after each Library event that can change it, so that the count stays current. The query runs on the UI thread, as it did before for the open list. The overhaul plan records it as a responsiveness concern.
- The visual check had two small errors: the playlist count of the intro, and the new-playlist key on Linux. The orchestrator corrected them.

Merge recommendation: pass. Task 006 moves Saved Searches and Discover into the Discover views.
