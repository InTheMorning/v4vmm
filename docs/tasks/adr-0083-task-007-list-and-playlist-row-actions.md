# ADR 0083 Task 007: List And Playlist Row Actions

Status: Implemented - 2026-10-20. Cases R83-7-01 to R83-7-14 pass. Mechanical checks are Green. V1 to V11 are open, in the final walk of the run.

## Goal

The row rule of task 006 applies to each other track row that does not use `TrackRow`. A row shows its information and its state at all times. It shows its actions on hover and on keyboard focus. The right-click menu holds the same items as the "⋯" menu.

This packet changes three lists:

- The playlist page rows.
- The Tracks list of the list column.
- The Needs attention list of the list column. Each row keeps its one main fix as a visible button (D10).

The release rows and the artist rows get no "⋯" menu. Decision D2 gives the reason.

## Authority

- [ADR 0083](../adr/0083-design-language.md) Decision 5: the action hierarchy. A destructive item is last, after a divider, in the danger color. Its label ends with "…", and it asks for confirmation.
- [ADR 0083](../adr/0083-design-language.md) Decision 6: a row shows its information and its state at all times. It shows its actions on hover and on keyboard focus. The right-click menu holds the same items as the "⋯" menu.
- [ADR 0083](../adr/0083-design-language.md) Decision 8: "Inspect sources" is one step away.
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 10: the selection model, the two modes and the action bar. No step needs a drag.
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 11: each track row keeps a place for audition. No control shows in it.
- [ADR 0076](../adr/0076-playlist-rss-check-for-stale-musicindex-records.md) Decision 7: the actions of a track with the "removed from feed" mark.
- The archived [ADR 0044](../adr/archive/0044-playlist-drag-handle-reordering.md): the drag handle and the menu items that replace the arrows.
- Packet [006](adr-0083-task-006-row-hover-actions.md): the menu composite `src/ui/composites/track_row_menu.rs`, `TrackRowMenuVm`, `menu_cell_visibility` and its decisions D2 to D5, D10 and D14.
- Packet [025](adr-0084-task-025-playlist-selection.md): the playlist selection mode. "Remove n from playlist" is not destructive and opens no confirmation.
- The durable set in [AGENTS.md](../../AGENTS.md): typed action state, renderer portability, token discipline, accessibility and cautious destructive actions.

## Recorded Facts - 2026-10-20

### The playlist page rows

- `render_playlist_track_row` in `src/ui/shells/playlist.rs` draws each playlist row. It does not use `TrackRow` or `ListRow`.
  - `render_playlist_leading_slot` draws the drag handle in `Normal` and the checkbox in `Selecting`.
  - `render_playlist_track_body` draws the position label, the cover, the title, the artist, the availability badge and the duration.
  - `render_playlist_track_controls` draws a "Play" icon button and a "⋯" `ContextMenu` with `ContextMenuScope::PlaylistTrack`.
  - `render_removed_from_feed` draws the error text and two buttons under a track with the "removed from feed" mark.
- `PlaylistTrackRowVm::controls_display` in `src/view_models/library.rs` gives three menu items:
  - "Move Up", with `disabled: !self.can_move_up()`.
  - "Move Down", with `disabled: !self.can_move_down()`.
  - "Remove", with the accessibility label "Remove track from playlist" and `destructive: true`. It opens no confirmation.
- `display_with_playback` changes `play_affordance` to `PlaylistPlayAffordance::RepairPlayback` when the player is not available. That affordance draws the text "Repair playback" in place of the Play icon.
- `PlaylistTrackRowDisplay::with_selection` sets `controls.reorder_visible` to false in `Selecting`. Then the handle, the drop target and the menu do not show (packet 025).
- `render_ready_playlist_row` in `src/ui/shells/library/playlist_detail.rs` wires the handlers:
  - The body click calls `click_playlist_entry`, then opens `FrameNavigationEntry::Track`.
  - "Play" emits `LibraryAppEvent::PlayPlaylistAt`.
  - "Move Up" and "Move Down" call `LibraryApp::move_playlist_track`.
  - "Remove" calls `LibraryApp::remove_playlist_track_at`.
- `playlist_reorder_display_contract_uses_drag_handle_and_menu_fallbacks` in `tests/architecture_tests.rs` asserts the text of the three menu items, also `label: "Remove"` and `destructive: true`.
- `adr_0084_playlist_selection_r25_11_selection_mode_row_accepts_no_drop` asserts `let actions_menu = reorder_visible.then(||` in `render_playlist_track_controls`.

### The Tracks list and the Needs attention list

- `render_content_list_row` in `src/ui/shells/library/content_list.rs` draws each row of both lists with `ListRow`. After the thumbnail and the text, it draws one `Secondary` button for each item of `row.actions`.
- `ContentListRowDisplay` (`src/view_models/library.rs`) has `actions: Vec<ContentListRowActionDisplay>` and `activation`. `from_kind` gives no action and `ContentListRowActivation::OpenContentRow`.
- A Tracks list row has no action. A click calls `LibraryApp::click_content_row`, which opens the track page in `Normal`.
- `ContentListRowDisplay::from_broadcast_readiness_track` makes a Needs attention row. It sets `activation` to `ContentListRowActivation::None`, so a click opens no page.
- `broadcast_readiness_row_actions` gives the actions of a Needs attention row:
  - `NoRouteTag`: "Fix routes" (`RepairBroadcastRoutes`). While the repair runs, the label is the working label, and the availability is `Working`.
  - `RemovedFromFeed`: "Confirm" (`ConfirmRemovedFromFeed`) and "Remove from library" (`RemoveFromLibrary`).
  - Each other state: no action.
- `LibraryApp::run_content_list_row_action` (`src/library/app_impl.rs`) maps `RemoveFromLibrary` to `remove_track`. That call uses `RemovalConfirmation::WhenReferenced`.
- `LibraryViewModel::content_list_rows_with_selection` fills `selection` only for a row where `content_list_track_row_key` gives a key. Only a Library track row gives a key.
- `BusyItems::track_row_work(album, track)` gives the `TrackRowWork` of a track. It needs the `AlbumNode` of the track.
- `LibraryApp` has two focus handles for the columns: `list_column_focus` and `detail_column_focus`.

### The rows with no action

- `render_destination_list_row` and `render_singles_group_rows` in `src/ui/shells/library/destination_lists.rs` draw the Releases and Artists rows. A click opens the page of the row. No row has an action button.
- `render_library_playlists_list` draws the Playlists rows. A click opens the playlist page. No row has an action button.
- `render_album_row` in `src/ui/shells/publisher.rs` draws the release rows of a feed page. A click opens the release page. The row shows the "In library" and "Not listed" badges. It has no action button.
- `render_release_page_menu` in `src/ui/shells/release_page.rs` holds the release actions of a release page.

## Orchestrator Decisions

The orchestrator made these decisions on 2026-10-20. The operator can audit each one. A decision with the mark **Operator-visible, no ADR decides it** changes what the operator sees in a way that no ADR decides. The operator must confirm it before the visual walk closes.

- **D1 Scope.** This packet changes the playlist page rows, the Tracks list rows and the Needs attention rows. It does not change `TrackRow`, which task 006 owns.
- **D2 No menu on release and artist rows.** On 2026-10-21, ADR 0084 packet 031 D1 replaced it for the rows of the Releases list, the feed page and the name page. They get one shared release row menu. The rest of D2 stays for the other rows. These rows get no "⋯" and no right-click menu:
  - The Releases rows and the singles group rows.
  - The Artists rows and the Playlists rows.
  - The feed page release rows and the Discover release rows.

  These are the reasons:

  - Each of these rows opens its page and has no row action today. Each row already shows its information and its state at all times.
  - The release page "⋯" menu and the action bar hold the release actions. A row menu adds a second path to the same actions with no new step.
  - A later packet can add a release row menu if the operator asks for it.
- **D3 The playlist row parts.** These parts show at all times: the position label, the cover, the title, the artist, the availability badge, the "removed from feed" block and the duration. These parts show on hover and on keyboard focus: the drag handle, the Play icon button and "⋯". Each hidden part keeps its width.
- **D4 "Repair playback".** When the affordance is `RepairPlayback`, the "Repair playback" text button shows at all times. It tells the operator that playback does not work. The Play icon shows only on hover and on keyboard focus.
- **D5 The playlist row menu.** The items are, in this order: "Add to playlist", "Download", "Inspect sources", "Move up", "Move down", "Remove from playlist", a divider, then "Remove from Library…". An item that cannot act is absent, as task 006 D2 decides. The labels use sentence case, so "Move Up" becomes "Move up".
- **D6 "Remove from playlist".** This item is not destructive. It has no "…", no danger color and no confirmation. The track stays in the Library. Packet 025 made the same decision for "Remove n from playlist". The present item "Remove" loses its danger color.
- **D7 "Remove from Library…".** On a playlist row and on a Needs attention row, this item is destructive and last. It calls `remove_track_after_confirmation`, the same as task 006 D5. It shows only for a downloaded idle track. Its accessibility label names the track, for example "Remove Face Pale from the Library…".
- **D8 "Add to playlist" on a playlist row.** The item opens the second level of task 006 D3. The open playlist is not a choice, the same as the bar of packet 025.
- **D9 The Tracks list rows.** The row shows "⋯" on hover and on keyboard focus. It shows no other row control. The menu holds the items of task 006 D2: "Add to playlist", "Download", "Inspect sources", a divider and "Remove…". The rules of task 006 D2 to D5 apply.
- **D10 The Needs attention rows.** Operator decision of 2026-10-20. Each row keeps its one main fix as a visible button at all times. The fix is a state with its action, the same as "Update" in an Apple update list.
  - A `NoRouteTag` row shows the "Fix routes" button.
  - A `RemovedFromFeed` row shows the "Confirm" button.
  - A row in each other `BroadcastReadinessState` shows no fix button.
  - The "Remove from library" button leaves the row.
  - The row shows "⋯" on hover and on keyboard focus. The right-click menu holds the same items, as task 006 decides. The menu holds, in this order:
    - "Inspect sources", for each row.
    - A divider, then "Remove from Library…", for a `RemovedFromFeed` row.
  - The menu does not hold the fix. The fix button is the one path to the fix.
- **D11 The repair state.** While a route repair runs, the fix button of the row shows the working label "Repairing…" in place of "Fix routes". Its availability is `Working`, as today. The state label and the reason line show at all times, as today.
- **D12 Selection mode.** In `Selecting`, no row of the three lists shows "⋯" or attaches the right-click menu. The fix button of a Needs attention row does not show in `Selecting`, the same as each other row action. The playlist row keeps the packet 025 rule: the checkbox takes the place of the drag handle.
- **D13 One menu composite.** Each list uses `track_row_menu.rs` of task 006. Add the variants `MoveUp`, `MoveDown` and `RemoveFromPlaylist` to `TrackRowMenuAction`. Each host maps only the variants that its view model gives. The fix button is not a menu item, so it adds no menu variant.
- **D14 One hover rule.** Each list uses `menu_cell_visibility` of task 006 and the same GPUI group name. The keyboard focus comes from `RowSelectionVm::focused`.
- **D15 The return focus.** The right-click menu of a playlist row returns the focus to `detail_column_focus`. The right-click menu of a Tracks or Needs attention row returns the focus to `list_column_focus`.
- **D16 Keyboard.** No key opens a row menu in this packet, the same as task 006 D14.
- **D17 Drag.** This packet does not change the reorder drag. The drag handle only hides until hover or keyboard focus. Task 008 owns the drag of a track onto a playlist.

## Design

### The view model

- Give each playlist row a `menu: Option<TrackRowMenuVm>` in `PlaylistTrackControlsDisplay`. Delete `move_up_menu_item`, `move_down_menu_item` and `remove_menu_item`. Delete `PlaylistTrackMenuItemDisplay` if no other user remains.
- Give `PlaylistTrackControlsDisplay` a typed visibility for the play cell, for example `play_visibility: RowCellVisibility`. `RepairPlayback` gives always-visible. `Play` gives on-hover.
- `PlaylistTrackRowVm` builds the menu items from these facts: `is_in_library`, the `TrackRowWork` of the track, `can_move_up`, `can_move_down` and the track key. The host gives `TrackRowWork` from `BusyItems`.
- `with_selection` sets `menu` to `None` in `Selecting`. Keep `reorder_visible` for the handle and the drop target.
- Give `ContentListRowDisplay` a `menu: Option<TrackRowMenuVm>`. Replace `actions` with one field for the D10 fix button, for example `fix: Option<ContentListRowActionDisplay>`.
  - A Tracks list row gets the D9 menu when `content_list_track_row_key` gives a key. It gets no fix button.
  - A Needs attention row gets its fix button from `broadcast_readiness_row_actions`, and the D10 menu.
  - `broadcast_readiness_row_actions` gives at most one item: "Fix routes" or "Confirm". It gives no "Remove from library" item. Delete `ContentListRowActionKind::RemoveFromLibrary` when no caller remains.
  - `content_list_rows_with_selection` sets `menu` and `fix` to `None` in `Selecting`.
- The labels and the accessibility labels are named constants in the view model. The renderer makes no text.
- The names are a proposal. Keep the meaning if a name collides with a task 006 name.

### The commands

- The playlist host maps `AddToPlaylist` to `add_track_to_playlist`, `Download` to `download_library_tracks` and `InspectSources` to the task 006 route.
  It maps `MoveUp` and `MoveDown` to `move_playlist_track`, `RemoveFromPlaylist` to `remove_playlist_track_at` and `Remove` to `remove_track_after_confirmation`.
- The list column host maps the D9 items the same as the release page host of task 006.
- The list column host maps the `Remove` menu item of a Needs attention row to `remove_track_after_confirmation`.
- The fix button keeps the present path: `run_content_list_row_action` maps `RepairBroadcastRoutes` and `ConfirmRemovedFromFeed` as today.

### The renderers

- `render_playlist_track_row` gets the hover group. The drag handle cell, the Play cell and the menu cell are `invisible()` and become visible on group hover. `menu_cell_visibility` keeps them visible on keyboard focus or while the menu is open.
- `render_playlist_track_controls` draws the menu through `track_row_menu.rs`. Delete `render_playlist_track_actions_menu` and `menu_item` when no caller remains.
- The playlist row attaches the right-click menu only when `menu` is `Some`.
- `ListRow` in `src/ui/composites/list_row.rs` gets one optional menu cell. When the cell is present, the row root gets the hover group, and the cell is the last fixed column with the width of `Size::PointerHitTarget`. With no menu cell, `ListRow` does not change.
- `render_content_list_row` draws the fix button at all times when `fix` is `Some`, as one `Secondary` button before the menu cell. It draws no other action button. It gives the menu to `ListRow` and attaches the right-click menu only when `menu` is `Some`.

## Required Changes

1. Add the D13 variants to `TrackRowMenuAction` and their labels.
2. Change the playlist row view model to D3 to D8. Update `controls_display`, `with_selection` and their tests.
3. Change `ContentListRowDisplay` to the menu of D9 and D10, and to the one D10 fix button. Change `broadcast_readiness_row_actions` to give at most one fix item. Update `content_list_rows_with_selection` and the Needs attention tests.
4. Add the menu cell and the hover group to `ListRow`.
5. Change `render_playlist_track_row`, `render_playlist_track_controls` and `render_content_list_row` to the hover rule and the menu composite. `render_content_list_row` keeps the D10 fix button visible.
6. Change the two hosts to the commands of "The commands".
7. Update the guards in "Recorded Facts" with the same meaning. Add the tests and the guards of this packet.

## Mechanical Acceptance Criteria

Use the prefix `adr_0083_list_row_actions_` for unit tests beside the owning code. Each new guard cites ADR 0083 Decision 6 and is situational.

| Case | Required proof |
|---|---|
| R83-7-01 | The playlist row `menu` gives the items in the order of D5. A downloaded idle track in the middle of a playlist gives "Add to playlist", "Inspect sources", "Move up", "Move down", "Remove from playlist" and "Remove from Library…" |
| R83-7-02 | The first playlist row gives no "Move up". The last playlist row gives no "Move down". A not-downloaded track gives "Download" and no "Remove from Library…" |
| R83-7-03 | "Remove from playlist" has `destructive` false and no "…". "Remove from Library…" is last, has `destructive` true, ends with "…" and has an accessibility label that names the track |
| R83-7-04 | "Remove from Library…" and "Download" are absent for `Queued`, `Downloading` and `Removing` |
| R83-7-05 | The "Add to playlist" choices of a playlist row do not hold the open playlist |
| R83-7-06 | `play_visibility` is always-visible for `RepairPlayback` and on-hover for `Play` |
| R83-7-07 | In `Selecting`, the playlist row `menu` is `None` and `reorder_visible` is false. In `Normal`, `menu` is `Some` |
| R83-7-08 | A Tracks list row with a track key gives the items of task 006 D2 for each pair of `is_in_library` and `TrackRowWork`. A row with no track key gives no menu |
| R83-7-09 | For each `BroadcastReadinessState`, a Needs attention row gives the D10 fix button and the D10 menu items. `NoRouteTag` gives the "Fix routes" fix, and `RemovedFromFeed` gives the "Confirm" fix. Each other state gives no fix. The menu never holds a fix item. Each row menu gives "Inspect sources". Only a `RemovedFromFeed` row menu gives "Remove from Library…" |
| R83-7-10 | The `Remove` menu item of a Needs attention row reaches `remove_track_after_confirmation`. The fix buttons reach `run_content_list_row_action` with `RepairBroadcastRoutes` and `ConfirmRemovedFromFeed` |
| R83-7-11 | In `Selecting`, each Tracks and Needs attention row has `menu` `None` and `fix` `None` |
| R83-7-12 | A GPUI layout test of `ListRow` with a menu cell: the cell has its token width when it is hidden, visible or empty. Each cell starts at the same x position on each row |
| R83-7-13 | A source guard: `render_content_list_row` makes one `UiButton` only, from `fix`, and no other `UiButton` for a row action. `render_playlist_track_controls` and `render_content_list_row` build the menu through `track_row_menu.rs`. The right-click handler attaches only when the menu is `Some` |
| R83-7-14 | `playlist_reorder_display_contract_uses_drag_handle_and_menu_fallbacks` and `adr_0084_playlist_selection_r25_11_selection_mode_row_accepts_no_drop` keep their meaning with the new fields. Each packet 006, 019 and 025 test still passes |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: on a playlist page, the pointer on a row shows the drag handle, the Play icon and "⋯". The pointer off the row hides them. No other row part moves.
- V2: the keyboard focus on a playlist row shows the same three controls on that row only.
- V3: with no player, the "Repair playback" text shows on each row at all times.
- V4: a right-click on a playlist row opens a menu with the same items, the same order and the same divider as "⋯". Only "Remove from Library…" has the danger color.
- V5: "Move up", "Move down" and "Remove from playlist" change the page in place, with no navigation.
- V6: in the Tracks list, the pointer on a row shows "⋯" only. The menu and the right-click menu hold the same items.
- V7: in the Needs attention list, each route problem row shows "Fix routes", and each removed track row shows "Confirm", with no hover. "⋯" on hover holds "Inspect sources", and "Remove from Library…" for a removed track. The menu holds no fix.
- V8: "Fix routes" shows "Repairing…" in its place on the row. After the repair, the row updates or leaves the list with no navigation.
- V9: "Inspect sources" opens the track page with the "Inspect sources" disclosure open.
- V10: "Select" hides "⋯", the right-click menu and the Needs attention fix buttons in each of the three lists. "Done" shows them again.
- V11: Light and Dark themes, a narrow width and the larger type sizes. No text is clipped, and no column overlaps another.

## Exclusions

- `TrackRow`, which task 006 owns.
- A row menu on the release, artist and playlist list rows, and on the feed page release rows (D2).
- The search result rows of `src/ui/shells/search_result_rows.rs`.
- A drag of a track onto a playlist. ADR 0083 task 008 owns it.
- A key that opens a row menu (D16).
- A download-state slot on the playlist rows and the list rows.
- The audition place of ADR 0084 Decision 11. It stays empty.
- No change to the selection model, the action bar, the reorder drag or the download queue rules.
- No schema change.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- Packet 006 and the code that it adds: `src/ui/composites/track_row_menu.rs`, `src/ui/composites/track_row.rs`.
- `src/ui/shells/playlist.rs`, `src/ui/shells/library/playlist_detail.rs`.
- `src/ui/shells/library/content_list.rs`, `src/ui/composites/list_row.rs`.
- `src/ui/shells/library/destination_lists.rs`, `src/ui/shells/publisher.rs`: read only, to confirm D2.
- `src/ui/primitives/context_menu.rs`, `src/ui/composites/playlist_popover.rs`.
- `src/view_models/library.rs` (`PlaylistTrackRowVm`, `PlaylistTrackControlsDisplay`, `ContentListRowDisplay`, `broadcast_readiness_row_actions`, `BusyItems`).
- `src/library/app_impl.rs` (`run_content_list_row_action`, `remove_track_after_confirmation`, `move_playlist_track`, `remove_playlist_track_at`), `src/library/detail_selection.rs`.
- `tests/architecture_tests.rs`: the guards in "Recorded Facts".

## Checks

```bash
cargo test --lib adr_0083_list_row_actions_
cargo test --lib adr_0083_row_actions_
cargo test --lib adr_0084_playlist_selection_
cargo test --lib adr_0084_detail_selection_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the commit. This packet adds no migration and no stored data. V5, V8 and the removal items change a playlist and the Library. Use a test playlist and a track that you can download again.

## Operator Answers - 2026-10-20

- D10: the operator decided that each Needs attention row keeps its main fix as a visible button.

## Orchestrator Review - 2026-10-20

Pass, after one fix. The orchestrator made the fix in the same change.

- `ContextMenuScope::PlaylistTrack` had no production user after this task. The orchestrator deleted it, and changed its two tests and the shared context menu guard (AGENTS.md: delete dead code).
- The deviations agree with the packet. The menu cell is 44 px, the hit area of a `Button`.
- A Needs attention row whose track is not in the Library tree gets no menu. An Index row in the Tracks list gets an empty menu cell. The orchestrator accepts both, because each row has no action that can act.
- The orchestrator added `content_list_row_actions.rs` to the source map.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0083-task-007-list-and-playlist-row-actions.md`
- Packet 006: `docs/tasks/adr-0083-task-006-row-hover-actions.md`, and its "Implementation Result" section
- ADR 0083 Decisions 5, 6 and 8, and ADR 0084 Decisions 10 and 11
- Packet 025
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the menu actions, the playlist row, the list rows, the `ListRow` menu cell, the Needs attention fix button and the hosts.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Follow each decision in "Orchestrator Decisions". Do not change a decision.
- The view model gives each label, accessibility label, visibility and availability. The renderer only composes.
- A Needs attention row keeps its one fix button visible at all times, outside the menu (D10).
- Use named tokens and named icons. Never show a state by color only.
- Tests use recorded values. No test sends a request or writes a file.
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git add` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- `TrackRow` and `track_row_menu.rs`, except to add the D13 variants and to make a builder `pub(crate)`.
- The reorder drag and its drop target in `src/ui/shells/playlist.rs`, except to hide the handle until hover.
- The release, artist and playlist list rows, and `src/ui/shells/publisher.rs`.
- The selection model, the action bar and the download queue rules.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R83-7-01 to R83-7-14 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - 2026-10-20" section and an "Operator Visual Check" section for V1 to V11.

Test commands:
- The commands in "Checks", with `cargo fmt -- --check` after the last edit and `cargo build --bin v4vmm` last.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- Task 006 is not implemented, or `track_row_menu.rs` does not exist.
- A playlist row or a list row cannot use `menu_cell_visibility` and the group hover of task 006.
- An `invisible()` drag handle cannot start a drag after hover makes it visible.
- `TrackRowWork` of a Tracks list row needs an `AlbumNode` that the view model cannot find.
- A change needs a file in "Do not touch".

## Implementation Result - 2026-10-20

### Files Changed

- `src/view_models/release_page.rs`:
  - `TrackRowMenuAction` has the new variants `MoveUp`, `MoveDown` and `RemoveFromPlaylist` (D13). Their labels are named constants.
  - New `TrackRowRemoveLabel` selects "Remove…" or "Remove from Library…".
  - `TrackRowMenuFacts` and `TrackRowMenuVm::project` are `pub(crate)`. Each item has a fact. The order is the order of D5.
  - New `LibraryTrackMenuAvailability` gives the "Download" and "Remove…" facts of a Library track. The release page rows use it too.
- `src/view_models/library.rs`:
  - `PlaylistTrackControlsDisplay` has `menu` and `play_visibility`. `actions_menu_id`, `actions_menu_a11y_label` and the three menu item fields are deleted. `PlaylistTrackMenuItemDisplay` is deleted.
  - New `RowCellVisibility`, `PlaylistTrackRowVm::with_work` and `playlist_row_add_choices`.
  - `with_selection` sets `menu` to `None` in `Selecting`.
  - `ContentListRowDisplay` has `fix`, `menu` and `readiness_state` in place of `actions`.
  - `broadcast_readiness_row_actions` gives at most one fix. `ContentListRowActionKind::RemoveFromLibrary` is deleted.
  - New `content_list_track_menu`, `LibraryViewModel::library_track_works` and `LibraryViewModel::library_track_key`.
  - `content_list_rows_with_selection` fills `menu`. It sets `menu` and `fix` to `None` in `Selecting`.
  - The working label of "Fix routes" is "Repairing…" (D11).
  - New tests R83-7-01 to R83-7-09 and R83-7-11. The playlist row and readiness tests use the new fields with the same meaning.
- `src/ui/composites/list_row.rs`:
  - `ListRow::menu_cell` adds one menu cell. The cell is the last fixed column, with the width of `TrackRowColumn::Menu` (44 px at the medium scale). With a menu cell, the row root is the hover group.
  - New `RowMenu`, `RowMenuRoot`, `row_menu_parts` and `menu_cell`. The playlist row uses them too.
  - New GPUI layout test R83-7-12.
- `src/ui/shells/playlist.rs`:
  - New component `PlaylistTrackRowElement`. It reads the keyed menu state at render time.
  - The row root is the hover group. The drag handle cell and the Play cell use the hover rule.
  - The menu cell is a `TrackRowMenuCell`. `render_playlist_track_actions_menu` and `menu_item` are deleted.
  - `PlaylistTrackRowSlot` has `menu` in place of `on_move_up` and `on_move_down`.
  - New GPUI test: the hidden drag handle starts a drag when the pointer is on it.
- `src/ui/shells/library/playlist_detail.rs`: builds the row menu with the playlist choices, the work of each track and the detail column focus.
- `src/ui/shells/library/detail.rs`: gives the playlists, the track works and the detail column focus to the playlist page.
- `src/ui/shells/library/content_list.rs`: draws the fix button and gives the menu to the `ListRow` menu cell.
- `src/ui/shells/library/content_list_row_actions.rs` (new): `ContentListMenuContext`, `content_list_row_menu` and the fix button renderer `render_content_list_row_action`.
- `src/ui/shells/library/mod.rs`: registers the new module.
- `src/library/app_impl.rs`:
  - New `run_content_list_menu_action`, `run_playlist_row_menu_action`, `run_track_row_menu_action`, `add_content_row_to_playlist` and `create_playlist_for_content_row`.
  - `run_content_list_row_action` takes no window. `remove_track` is deleted, because no caller was left.
  - The list column gives the playlists and `list_column_focus` to the content list.
- `tests/architecture_tests.rs`:
  - New situational guards R83-7-10 and R83-7-13. Each one cites ADR 0083 Decision 6.
  - `playlist_reorder_display_contract_uses_drag_handle_and_menu_fallbacks`, `adr_0084_playlist_selection_r25_11_selection_mode_row_accepts_no_drop`, `adr_0076_route_readiness_removal_actions_reuse_existing_flows` and `adr_0065_readiness_rows_keep_state_labels_separate_from_actions` use the new fields with the same meaning.

### Behavior Changed

- A playlist row always shows the position, the cover, the title, the artist, the availability badge, the "removed from feed" block and the duration.
- The drag handle, the Play icon and "⋯" of a playlist row show on a hover and on the keyboard focus of the row. A hidden cell keeps its width.
- "Repair playback" shows at all times when the player is not available.
- The playlist row menu holds "Add to playlist", "Download", "Inspect sources", "Move up", "Move down" and "Remove from playlist". Then it holds a divider and "Remove from Library…". An item that cannot act is absent.
  - "Remove from playlist" has no danger color and opens no confirmation.
  - "Remove from Library…" always asks for confirmation.
  - The "Add to playlist" choices do not hold the open playlist.
- A right-click on a playlist row opens the same menu. Its dismissal gives the focus to the detail column.
- A Tracks list row shows "⋯" on a hover and on the keyboard focus. The menu holds "Add to playlist", "Download", "Inspect sources", a divider and "Remove…".
- A Needs attention row shows its one fix button at all times: "Fix routes" or "Confirm". The "Remove from library" button is gone.
  - "⋯" holds "Inspect sources". A removed track also has a divider and "Remove from Library…".
  - During a route repair, the fix button shows "Repairing…".
- A right-click on a Tracks or Needs attention row opens the same menu. Its dismissal gives the focus to the list column.
- In selection mode, no row of the three lists shows "⋯" or a right-click menu. A Needs attention row shows no fix button.

### Checks - 2026-10-20

| Check | Result |
|---|---|
| `cargo test --lib adr_0083_list_row_actions_` | Green, 12 tests |
| `cargo test --lib adr_0083_row_actions_` | Green, 13 tests |
| `cargo test --lib adr_0084_playlist_selection_` | Green, 12 tests |
| `cargo test --lib adr_0084_detail_selection_` | Green, 9 tests |
| `cargo test` | Green, 2094 unit tests and 350 guards. Ten documentation examples stay ignored |
| `cargo test --test architecture_tests` | Green, 350 guards |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo clippy --all-targets -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

No application launch and no production-data change occurred.

### Deviations From The Task

1. The packet gives the menu cell the width of `Size::PointerHitTarget`. The cell has the width of `TrackRowColumn::Menu`, which is `Size::MinHitTarget` (44 px). The orchestrator review of task 006 set this width, so no hover fill is cut.
2. The right-click menu needs keyed state, and keyed state needs the window. Thus the playlist row is the component `PlaylistTrackRowElement`, and `render_playlist_track_row` takes the menu parts.
3. `content_list.rs` exceeded the 500-line ceiling of the screen surface guard. Thus the fix button renderer and the menu wiring are in the new module `content_list_row_actions.rs`. The ADR 0065 guard reads the fix button renderer from that module.
4. `PlaylistTrackRowVm` gets the work through the new builder `with_work`. The host reads the work of each track from `library_track_works`, one tree pass for each render.
5. The R83-7-10 command proof is a source guard, because the commands need a `LibraryApp` with a database. The view model tests prove the items, and the guard proves the mapping.
6. The present working label "Fixing..." changed to "Repairing…" (D11).
7. A tile of the content list keeps the fix button and gets no menu. The Needs attention list always uses the list mode, and a Tracks row has no fix.

### Unresolved Concerns

1. V1 to V11 need the operator. No test proves a rendered hover, a menu position or the danger color.
2. `ContextMenuScope::PlaylistTrack` has no production user now. Only its own tests use it. This packet does not change `context_menu.rs`.
3. The new file `src/ui/shells/library/content_list_row_actions.rs` is not in `docs/architecture/source-map.md`. V1 to V11 are not in `docs/pending-human-checks.md`. The task instructions named only this packet as a document to edit.
4. The GPUI test of the hidden drag handle also passes with no hover step. The test harness moves the pointer before a mouse press. Thus the test proves that a drag starts from the handle, but not that the handle is hidden before a hover. V1 examines that.
5. A Needs attention row whose track is not in the Library tree gets no menu. A Tracks list row of an Index track gets an empty menu cell.

## Operator Visual Check

This check needs a Linux desktop session and this checkout. It also needs these items:

- A test playlist with three or more tracks. One track must be downloaded, and one track must not be downloaded.
- A second playlist.
- One Needs attention row with missing routes, if the Library has one. One Needs attention row of a track removed from its feed, if the Library has one.

V5 changes a playlist. "Remove from Library…" removes an audio file. Use a test playlist and a track that you can download again.

1. Build the app: `cargo build --bin v4vmm`. Then start `target/debug/v4vmm` from a desktop session.
2. V1: open Music, then open the test playlist. Move the pointer onto a row.
   - Correct: the drag handle, the Play icon and "⋯" show on that row. They hide when the pointer leaves the row.
   - Wrong: a control shows on a row with no pointer, or a part of the row moves when the controls show or hide.
3. V2: click a playlist row, then press the Up arrow and the Down arrow.
   - Correct: the focused row shows the drag handle, the Play icon and "⋯". The other rows do not.
4. V3: stop the player so that playback is not available. For example, stop the mpv service that the app uses. Open the playlist again.
   - Correct: each row shows "Repair playback" at all times, with no hover.
5. V4: click "⋯" on the downloaded middle track, then right-click the same row.
   - Correct: each menu shows "Add to playlist", "Inspect sources", "Move up", "Move down" and "Remove from playlist". Then each menu shows a divider and "Remove from Library…".
   - Correct: only "Remove from Library…" has the danger color. The first row has no "Move up", and the last row has no "Move down".
   - Correct: a track that is not downloaded shows "Download" and no "Remove from Library…".
   - Correct: "Add to playlist" shows the second playlist and not the open playlist.
6. V5: select "Move down", then "Move up", then "Remove from playlist" on one row.
   - Correct: the page changes in place. No navigation occurs, and no confirmation opens.
7. V6: open the Tracks list. Move the pointer onto a row, then right-click the row.
   - Correct: only "⋯" shows on hover. The "⋯" menu and the right-click menu show the same items.
8. V7: open Needs attention.
   - Correct: a missing routes row shows "Fix routes", and a removed track row shows "Confirm", with no hover.
   - Correct: "⋯" shows "Inspect sources". For a removed track, it also shows a divider and "Remove from Library…". No menu holds "Fix routes" or "Confirm".
9. V8: click "Fix routes" on one row.
   - Correct: the button shows "Repairing…" in its place. After the repair, the row changes or leaves the list, with no navigation.
10. V9: select "Inspect sources" in a playlist row menu and in a Tracks row menu.
    - Correct: the track page opens with the "Inspect sources" disclosure open.
11. V10: click "Select" in the playlist, the Tracks list and Needs attention. Then click "Done".
    - Correct: "Select" hides "⋯", the right-click menu and the fix buttons. "Done" shows them again.
12. V11: change to the Light theme and to the Dark theme. Make the window narrow. Set a larger type size in Settings.
    - Wrong: clipped text, or one column on top of another column.

### Cleanup

1. Put back the removed playlist entries with "Add to playlist".
2. Download again each track that you removed from the Library.
3. Start the player service again, if you stopped it in step 4.
4. Set the theme, the window width and the type size back to your values.
