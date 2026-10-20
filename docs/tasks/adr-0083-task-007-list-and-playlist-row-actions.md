# ADR 0083 Task 007: List And Playlist Row Actions

Status: Ready - 2026-10-20. It starts after task 006 is implemented.

## Goal

The row rule of task 006 applies to each other track row that does not use `TrackRow`. A row shows its information and its state at all times. It shows its actions on hover and on keyboard focus. The right-click menu holds the same items as the "⋯" menu.

This packet changes three lists:

- The playlist page rows.
- The Tracks list of the list column.
- The Needs attention list of the list column.

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
- **D2 No menu on release and artist rows.** These rows get no "⋯" and no right-click menu:
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
- **D10 The Needs attention rows.** **Operator-visible, no ADR decides it.** The "Fix routes", "Confirm" and "Remove from library" buttons leave the row. The row shows "⋯" on hover and on keyboard focus. The menu holds, in this order:
  - "Fix routes", for a `NoRouteTag` row that is not in repair.
  - "Confirm", for a `RemovedFromFeed` row.
  - "Inspect sources", for each row.
  - A divider, then "Remove from Library…", for a `RemovedFromFeed` row.
  - The reason: ADR 0083 Decision 6 puts row actions behind hover. The list column is narrow, and one hover control keeps each row's text width stable. The operator can ask for a "Fix routes" button on hover in a later packet.
- **D11 The repair state.** While a route repair runs, "Fix routes" is absent from the menu. The state label "Repairing…" and the reason line show at all times, as today.
- **D12 Selection mode.** In `Selecting`, no row of the three lists shows "⋯" or attaches the right-click menu. The playlist row keeps the packet 025 rule: the checkbox takes the place of the drag handle.
- **D13 One menu composite.** Each list uses `track_row_menu.rs` of task 006. Add the variants `MoveUp`, `MoveDown`, `RemoveFromPlaylist`, `FixRoutes` and `ConfirmRemovedFromFeed` to `TrackRowMenuAction`. Each host maps only the variants that its view model gives.
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
- Give `ContentListRowDisplay` a `menu: Option<TrackRowMenuVm>`. Delete `actions` and `ContentListRowActionDisplay` when no renderer reads them. Keep `ContentListRowActionKind` only if the menu host uses it to dispatch.
  - A Tracks list row gets the D9 menu when `content_list_track_row_key` gives a key.
  - A Needs attention row gets the D10 menu from `broadcast_readiness_row_actions`.
  - `content_list_rows_with_selection` sets `menu` to `None` in `Selecting`.
- The labels and the accessibility labels are named constants in the view model. The renderer makes no text.
- The names are a proposal. Keep the meaning if a name collides with a task 006 name.

### The commands

- The playlist host maps `AddToPlaylist` to `add_track_to_playlist`, `Download` to `download_library_tracks` and `InspectSources` to the task 006 route.
  It maps `MoveUp` and `MoveDown` to `move_playlist_track`, `RemoveFromPlaylist` to `remove_playlist_track_at` and `Remove` to `remove_track_after_confirmation`.
- The list column host maps the D9 items the same as the release page host of task 006.
- The list column host maps `FixRoutes` to `repair_broadcast_routes_for_track`, `ConfirmRemovedFromFeed` to `confirm_removed_track` and `Remove` to `remove_track_after_confirmation`.

### The renderers

- `render_playlist_track_row` gets the hover group. The drag handle cell, the Play cell and the menu cell are `invisible()` and become visible on group hover. `menu_cell_visibility` keeps them visible on keyboard focus or while the menu is open.
- `render_playlist_track_controls` draws the menu through `track_row_menu.rs`. Delete `render_playlist_track_actions_menu` and `menu_item` when no caller remains.
- The playlist row attaches the right-click menu only when `menu` is `Some`.
- `ListRow` in `src/ui/composites/list_row.rs` gets one optional menu cell. When the cell is present, the row root gets the hover group, and the cell is the last fixed column with the width of `Size::PointerHitTarget`. With no menu cell, `ListRow` does not change.
- `render_content_list_row` draws no action button. It gives the menu to `ListRow` and attaches the right-click menu only when `menu` is `Some`.

## Required Changes

1. Add the D13 variants to `TrackRowMenuAction` and their labels.
2. Change the playlist row view model to D3 to D8. Update `controls_display`, `with_selection` and their tests.
3. Change `ContentListRowDisplay` to the menu of D9 and D10. Update `content_list_rows_with_selection` and the Needs attention tests.
4. Add the menu cell and the hover group to `ListRow`.
5. Change `render_playlist_track_row`, `render_playlist_track_controls` and `render_content_list_row` to the hover rule and the menu composite.
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
| R83-7-09 | A Needs attention row gives the D10 items for each `BroadcastReadinessState`. A `NoRouteTag` row in repair gives no "Fix routes". Each row gives "Inspect sources" |
| R83-7-10 | The `Remove` command of a Needs attention row reaches `remove_track_after_confirmation` |
| R83-7-11 | In `Selecting`, each Tracks and Needs attention row has `menu` `None` |
| R83-7-12 | A GPUI layout test of `ListRow` with a menu cell: the cell has its token width when it is hidden, visible or empty. Each cell starts at the same x position on each row |
| R83-7-13 | A source guard: `render_content_list_row` makes no `UiButton` for a row action. `render_playlist_track_controls` and `render_content_list_row` build the menu through `track_row_menu.rs`. The right-click handler attaches only when the menu is `Some` |
| R83-7-14 | `playlist_reorder_display_contract_uses_drag_handle_and_menu_fallbacks` and `adr_0084_playlist_selection_r25_11_selection_mode_row_accepts_no_drop` keep their meaning with the new fields. Each packet 006, 019 and 025 test still passes |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: on a playlist page, the pointer on a row shows the drag handle, the Play icon and "⋯". The pointer off the row hides them. No other row part moves.
- V2: the keyboard focus on a playlist row shows the same three controls on that row only.
- V3: with no player, the "Repair playback" text shows on each row at all times.
- V4: a right-click on a playlist row opens a menu with the same items, the same order and the same divider as "⋯". Only "Remove from Library…" has the danger color.
- V5: "Move up", "Move down" and "Remove from playlist" change the page in place, with no navigation.
- V6: in the Tracks list, the pointer on a row shows "⋯" only. The menu and the right-click menu hold the same items.
- V7: in the Needs attention list, no row shows a button. "⋯" on hover holds "Fix routes" or "Confirm", "Inspect sources" and "Remove from Library…" when they apply.
- V8: "Fix routes" from the menu shows "Repairing…" on the row in place. After the repair, the row updates or leaves the list with no navigation.
- V9: "Inspect sources" opens the track page with the "Inspect sources" disclosure open.
- V10: "Select" hides "⋯" and the right-click menu in each of the three lists. "Done" shows them again.
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
- Make each change in "Required Changes": the menu actions, the playlist row, the list rows, the `ListRow` menu cell and the hosts.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Follow each decision in "Orchestrator Decisions". Do not change a decision.
- The view model gives each label, accessibility label, visibility and availability. The renderer only composes.
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
