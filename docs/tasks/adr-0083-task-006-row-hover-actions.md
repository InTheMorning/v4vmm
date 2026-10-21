# ADR 0083 Task 006: Track Row Hover Actions

Status: Implemented - 2026-10-20. Cases R83-6-01 to R83-6-15 pass. Mechanical checks are Green. V1 to V12 are open, in the final walk of the run.

## Goal

The shared track row shows its information at all times and shows most actions only on hover and on keyboard focus. The row follows the track row of Apple Music on macOS:

`[checkbox in Selecting] [#] [cover] Title [badge] / date ......... [download-state slot] [duration] [⋯ on hover and keyboard focus]`

The always-visible "Remove" text button and the "+ Playlist" button leave the row. A "⋯" menu and the same right-click menu hold the row actions. The bulk bar keeps the bulk actions.

## Authority

- [ADR 0083](../adr/0083-design-language.md) Decision 5: the action hierarchy. A destructive item is last, after a divider, in the danger color. Its label ends with "…", and it asks for confirmation.
- [ADR 0083](../adr/0083-design-language.md) Decision 6: a row shows its information and its state at all times. It shows its actions on hover and on keyboard focus. The right-click menu holds the same items as the "⋯" menu.
- [ADR 0083](../adr/0083-design-language.md) Decision 10: one icon set. An icon alone needs a tooltip and an accessibility label.
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 10: the selection model, the two modes, the action bar and the download queue.
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 11: each track row keeps a place for audition. No control shows in it.
- The archived [ADR 0044](../adr/archive/0044-playlist-drag-handle-reordering.md) gave the playlist row its "⋯" menu. This packet uses the same `ContextMenu` primitive.
- Packets [022](adr-0084-task-022-download-queue.md) and [025](adr-0084-task-025-playlist-selection.md): the queue phases, and the hidden row menu in selection mode.
- Operator decision of 2026-10-20 (binding):
  - Always visible: the number, the cover, the title, the date line, the state badges, the download-state slot and the duration.
  - The state shows as a badge after the title, for example "New". The `MusicBrainz` status becomes a small named icon. Its word is in a tooltip and in the accessibility label.
  - One fixed download-state slot shows before the duration. Each state of the slot has an accessibility label.
  - The duration is right-aligned. It is the last fixed column before the menu button.
  - The "⋯" menu and the right-click menu hold "Add to playlist", "Download", "Inspect sources", a divider and "Remove…".
  - Selection mode hides the "⋯" button and the context menu.
  - The `TrackRowColumn` tokens change to the new column set. The columns that do not exist any more are deleted.
- The durable set in [AGENTS.md](../../AGENTS.md): typed action state, renderer portability, token discipline, accessibility and cautious destructive actions.

## Recorded Facts - 2026-10-20

### The row and its users

- `TrackRow` in `src/ui/composites/track_row.rs` is the one track row composite. It sits above `ListRow`.
  - Its body holds the number, the cover, the title with the date line, and the duration cell.
  - After the body, it holds three fixed columns: `PrimaryAction`, `State` and `PlaylistAction`.
  - The builder methods are `primary_action`, `state_child` and `playlist_action`. The checkbox leads the row only when `RowSelectionVm::checkbox_visible` is true.
- `TrackRowColumn` in `src/ui/tokens.rs` has four variants: `Duration`, `PrimaryAction`, `State` and `PlaylistAction`. `ROW_ORDER` lists them. Each width has a text part on the TYPE scale and an inset part on the CHROME scale.
- `column_tests` in `src/ui/composites/track_row.rs` holds two tests on these columns:
  - `adr_0084_row_fix_empty_columns_keep_their_token_width`
  - `adr_0084_row_fix_column_widths_grow_with_the_scale`
- `TrackRow::from_vm` has one caller: `render_release_page_track_row` in `src/ui/shells/release_page.rs`. Two pages use that function:
  - The release page of each origin, through `render_release_page`.
  - The singles group page, through `render_single_row` in `src/ui/shells/library/singles_group.rs`.
- `render_release_page_track_row` makes these elements:
  - A text button from `ReleaseTrackPrimaryVm`, with the `RowAction` or the `DestructiveRowAction` style.
  - The `MusicBrainz` status as colored text. The color comes from `MbStatusKind` through `StatusRole`.
  - The state label as muted text.
  - An `AddToPlaylistPopover` with the trigger "+ Playlist".
- The playlist page rows do not use `TrackRow`. `render_playlist_track_row` in `src/ui/shells/playlist.rs` draws them, with a drag handle, "Play" and a "⋯" `ContextMenu`. Packet 025 hides that menu in selection mode.
- The Tracks list and the Needs attention list use `ListRow` rows, not `TrackRow`.

### The view model and the commands

- `ReleaseTrackRowVm` in `src/view_models/release_page.rs` has `primary: ReleaseTrackPrimaryVm`, `status: Option<ReleaseTrackStatusVm>`, `state_label` and `playlist_trigger_label`.
- `ReleaseTrackRowVm::library` and `library_single` read `LibraryTrackRowVm` (`src/view_models/library.rs`):
  - `primary_action_vm(work)` gives "Download", "Queued", "Downloading...", "Remove" or "Removing...".
  - `row_display()` gives the state label "New" when `is_in_library` is false.
  - `mb_status_text()` gives "MB: pending", "MB: looking up...", "MB: no missing fields", "MB: done" or "MB: skipped".
- `TrackRow::is_in_library` (`src/db.rs`) is true for a downloaded track. "Remove" shows only for such a track.
- `TrackRowWork` (`src/view_models/library.rs`) is `Idle`, `Queued`, `Downloading` or `Removing`. `BusyItems::track_row_work` gives it from `DownloadActivity` and from the removals in progress.
- `ReleaseTrackRowVm::index` gives each Index row the label "Download", always enabled. The Index row reads no queue state.
- `DownloadActivity` (`src/view_models/download_progress.rs`) holds Library track ids and feed keys. It holds no identity of an Index track job. `JobIdentity::IndexTrack` in `src/library/download_queue.rs` holds the feed key and an `IndexTrackIdentity`.
- `ReleasePageCommand` has `TrackPrimary(usize)`, `AddTrackToPlaylist(usize, i64)` and `CreatePlaylistForTrack(usize, String)`.
  - `LibraryApp::run_release_page_command` (`src/library/release_page.rs`) maps `TrackPrimary` to `remove_track` or to `download_library_tracks`.
  - `TopApp`, in `src/app/search_dispatch.rs`, maps the Index `TrackPrimary` to `download_index_track`.
- `LibraryApp::remove_track` uses `RemovalConfirmation::WhenReferenced`. `remove_track_after_confirmation` uses `RemovalConfirmation::Always`. The track page "Remove track…" uses the second one.

### The menus, the hover and the icons

- `ContextMenu` in `src/ui/primitives/context_menu.rs` is a "⋯" trigger with a popover. It keeps its open state in `window.use_keyed_state` with the key `"{id}-state"`. `build_menu_content` draws a divider before a destructive item.
- `PointerContextMenu` in the same file is a controlled menu at a pointer position. Its owner removes it on dismissal. Only `src/ui/composites/selectable_text.rs` uses it, through `on_mouse_down(MouseButton::Right, ...)`.
- The release page "⋯" menu (`render_release_page_menu`) and the track page "⋯" menu (`render_track_page_menu` in `src/ui/shells/library/track_detail_metadata.rs`) use `ContextMenu` with `trigger_label("")`.
- No file in `src/ui` uses `group_hover` today. GPUI 0.3.1 has `.group(name)`, `.group_hover(name, ...)`, `.invisible()` and `.visible()`.
  - A `Visibility::Hidden` element keeps its layout size. It paints no child and no hitbox.
  - It records its debug bounds before that check, so a layout test can read the bounds of a hidden cell.
- The keyboard focus of a row is a view-model fact: `RowSelectionVm::focused`. The detail column has one `FocusHandle`, `LibraryApp::detail_column_focus`.
- `IconName` in `src/ui/icons.rs` has `More` (Lucide `ellipsis`), but no cloud, clock or progress icon.
  - `component_icon` maps a name to the default bundle. The default bundle has `loader-circle`, `circle-check` and `circle-x`.
  - `extra_icon` maps a name to the complete catalog. `ExtraLucideIcons` embeds each extra icon.
  - `CloudDownload`, `Clock`, `Hourglass`, `ScanSearch` and `BadgeCheck` exist in the complete catalog.
- `Button` (`src/ui/primitives/button.rs`) has `leading_icon`, `tooltip` and `a11y_label`. `Tooltip` is in `src/ui/primitives/tooltip.rs`.
- The `AddToPlaylistPopover` (`src/ui/composites/playlist_popover.rs`) has a list mode and a create mode. Each mode is a private builder in that file.
- "Inspect sources" is a disclosure of the Library track page only (`render_library_track_detail_metadata`). `InspectorPanelKind::Sources` opens it. The Index track page has no "Inspect sources".

### Tests that pin the present row

- `src/view_models/release_page.rs` tests assert `primary`, `state_label` and `playlist_trigger_label`. One of them is `adr_0084_release_page_r21_03_library_row_matches_library_track_row_vm`.
- `library_track_row_vm_primary_action_follows_membership_and_busy_state` in `src/view_models/library.rs` asserts the primary action labels. The track page uses the same labels, so this test stays.
- `adr_0084_index_page_r13_06_track_row_click_and_description_toggle_are_wired` in `tests/architecture_tests.rs` requires `.on_click(` in `render_release_page_track_row`. It stays.

## Orchestrator Decisions

The orchestrator made these decisions on 2026-10-20, inside the operator decision of the same day. The operator can audit each one.

- **D1 Scope.** This packet changes `TrackRow` only: the release page of each origin and the singles group page. The playlist page rows and the `ListRow` track lists keep their present controls. A later packet, ADR 0083 task 007 (not written), owns them. The reasons:
  - The playlist row has "Play", a drag handle and its own menu. Its packet 025 walk is open.
  - The `ListRow` lists do not use `TrackRow`. A second row change makes this packet too large for one session.
- **D2 Menu order.** "Add to playlist", "Download", "Inspect sources", a divider, then "Remove…". An item that cannot act is absent. The label is "Inspect sources", the same as the disclosure on the track page.
- **D3 "Add to playlist".** The item opens a second level in the same menu surface. That level holds the playlist choices, "New playlist…" and "Back". It reuses the list mode and the create mode of `AddToPlaylistPopover`.
- **D4 "Inspect sources".** The item opens the track page of the row and opens its "Inspect sources" disclosure. It shows only on a Library row with a track key. The Index track page has no "Inspect sources", so the item is absent on an Index row.
- **D5 "Remove…".** The item always asks for confirmation through `remove_track_after_confirmation`. Its accessibility label names the track, for example "Remove Face Pale from the Library…". It is absent while the row is `Queued`, `Downloading` or `Removing` (packet 022, correction C3).
- **D6 Download slot.** The slot has five states:
  - Not downloaded and idle: an icon button with the `CloudDownload` icon. The tooltip is "Download". A click queues the track through packet 022.
  - Queued: the `Clock` icon. The tooltip is "Queued for download".
  - Downloading: the `LoaderCircle` icon, with no motion. The tooltip is "Downloading".
  - Downloaded, or removing: an empty slot.
- **D7 No progress ring.** The queue records no fraction for one job, so a ring has no true value. The progress item of packet 022 shows the batch.
- **D8 The Index row phase.** `DownloadActivity` gets the Index track jobs, by feed key and track GUID. The Index row reads its own phase, then the phase of a release job of its key. A queued job of another track of the release does not change the row.
- **D9 Selection mode.** `Selecting` removes the "⋯" button and the right-click menu. The download slot and its button stay, as "Play" stays on the playlist row. The menu column keeps its width.
- **D10 Hover.** The row is a GPUI group. The menu cell is `invisible()` and becomes visible on group hover. It is also visible when the row has the keyboard focus or when its menu is open. A hidden cell keeps its width.
- **D11 Badges.** "New" shows as a quiet badge after the title on a Library row that is not downloaded. "Removing" shows as a badge while a removal runs. An Index row shows no "New" badge, because each row of an Index page is not in the Library.
- **D12 `MusicBrainz` icons.** Each status has its own icon. The word is in the tooltip and in the accessibility label:
  - Pending: `Hourglass`, "MusicBrainz: pending".
  - Processing: `ScanSearch`, "MusicBrainz: looking up".
  - Done with no missing field: `CircleCheck`, "MusicBrainz: no missing fields".
  - Done with fields: `BadgeCheck`, "MusicBrainz: done".
  - Skipped: `CircleX`, "MusicBrainz: skipped".
- **D13 Columns.** `TrackRowColumn` becomes `DownloadState`, `Duration` and `Menu`, in this order. `DownloadState` and `Menu` have the width of `Size::PointerHitTarget` on the CHROME scale. `Duration` keeps 44 px of text at `FontSize::Micro`. `PrimaryAction`, `State` and `PlaylistAction` are deleted.
- **D14 Keyboard.** The keyboard focus shows the "⋯" button. No key opens the menu in this packet. ADR 0083 Decision 6 makes a shortcut optional.
- **D15 Drag to a playlist.** ADR 0083 Decision 6 also asks for a drag of a track onto a sidebar playlist. This packet does not add it. A later packet, ADR 0083 task 008 (not written), owns it.

## Design

### The view model

Change `ReleaseTrackRowVm` in `src/view_models/release_page.rs`. Delete `primary`, `status`, `state_label` and `playlist_trigger_label`. Add these typed fields:

```rust
pub(crate) enum TrackRowBadge { New, Removing }
pub(crate) struct TrackRowBadgeVm { pub(crate) badge: TrackRowBadge, pub(crate) label: &'static str }

pub(crate) enum MbStatusIcon { Pending, Processing, NoMissingFields, Done, Skipped }
pub(crate) struct TrackRowMbStatusVm { pub(crate) icon: MbStatusIcon, pub(crate) label: &'static str }

pub(crate) enum DownloadSlotState { Download, Queued, Downloading, Empty }
pub(crate) struct DownloadSlotVm {
    pub(crate) state: DownloadSlotState,
    pub(crate) id: String,
    pub(crate) tooltip: Option<&'static str>,
    pub(crate) a11y_label: Option<String>,
    pub(crate) available: bool, // true only for `Download`
}

pub(crate) enum TrackRowMenuAction { AddToPlaylist, Download, InspectSources, Remove }
pub(crate) struct TrackRowMenuItemVm {
    pub(crate) action: TrackRowMenuAction,
    pub(crate) id: String,
    pub(crate) label: &'static str,
    pub(crate) a11y_label: String,
    pub(crate) destructive: bool,
}
pub(crate) struct TrackRowMenuVm {
    pub(crate) id: String,
    pub(crate) trigger_a11y_label: String,
    pub(crate) items: Vec<TrackRowMenuItemVm>,
}
```

- `badges: Vec<TrackRowBadgeVm>`, `mb_status: Option<TrackRowMbStatusVm>`, `download: DownloadSlotVm` and `menu: Option<TrackRowMenuVm>`.
- `menu` is `None` when `selection.checkbox_visible` is true. The same fact removes the right-click menu.
- The labels are named constants in the view model. The renderer makes no text.
- The names in the code block are a proposal. Keep the meaning if a name collides.

### The Index row phase

- Add `index_tracks` and `in_flight_index_tracks` to `DownloadActivity`, as sets of `(FeedKey, String)`. The string is the track GUID.
- `DownloadQueue::activity` fills them from each `JobIdentity::IndexTrack` with `IndexTrackIdentity::Guid`. Add `DownloadActivity::index_track_phase(key, guid)`.
- `ReleaseTrackRowVm::index` takes a `TrackRowWork`. The host gives the phase of the Index track job, then `release_phase` of the feed key.
- An Index row is never downloaded. Its slot is `Download`, `Queued` or `Downloading`.

### The commands

- Delete `ReleasePageCommand::TrackPrimary`. Add `DownloadTrack(usize)`, `RemoveTrack(usize)` and `InspectSources(usize)`. Keep `AddTrackToPlaylist` and `CreatePlaylistForTrack`.
- `LibraryApp::run_release_page_command`:
  - `DownloadTrack` calls `download_library_tracks`.
  - `RemoveTrack` calls `remove_track_after_confirmation`.
  - `InspectSources` opens `FrameNavigationEntry::Track` of the row track, then expands `InspectorPanelKind::Sources` of that track frame.
- The `TopApp` Index route maps `DownloadTrack` to `download_index_track`. It ignores `RemoveTrack` and `InspectSources`, because the view model gives no such item on an Index row.

### The composite

- `TrackRow` keeps the number, the cover and the title column. The title line holds the title, then each badge, then the `MusicBrainz` icon. The date line stays under the title.
- After the body, `TrackRow` holds the fixed columns of `TrackRowColumn::ROW_ORDER`: `DownloadState`, `Duration` and `Menu`. The duration is right-aligned. Each empty or hidden cell keeps its token width.
- The `TrackRow` builders become `badges`, `mb_status`, `download_slot`, `menu` and the handlers `on_download` and `on_menu_action`. Delete `primary_action`, `state_child` and `playlist_action`.
- Add `src/ui/composites/track_row_menu.rs`. It owns these parts:
  - One item builder from `TrackRowMenuVm`, for the "⋯" trigger and for the right-click menu.
  - The "⋯" trigger: `IconName::More`, no text, the tooltip "More actions" and the accessibility label from the view model.
  - The second level of "Add to playlist", with the playlist choices, "New playlist…" and "Back".
  - The pointer position of the right-click menu, in keyed state of the row id.
- Make the list mode and the create mode builders of `src/ui/composites/playlist_popover.rs` `pub(crate)`. Do not copy them.
- Add `pub(crate) fn ContextMenu::is_open(id, window, cx)` in `src/ui/primitives/context_menu.rs`, or one equal helper. The row uses it to keep the trigger visible while the menu is open.
- The row root gets `.group(TRACK_ROW_GROUP)`. The menu cell is `invisible()` with `.group_hover(TRACK_ROW_GROUP, |s| s.visible())`. It is visible when `selection.focused` is true or when its menu is open.
- Put that rule in one pure function, for example `menu_cell_visibility(focused, open) -> MenuCellVisibility`.
- The right-click handler attaches only when `menu` is `Some`. It records the position and opens `PointerContextMenu`. Its return focus is `LibraryApp::detail_column_focus`, which the host gives through `ReleasePageImages` or a new field.
- The download slot draws an icon button for `Download` and an icon with a tooltip for `Queued` and `Downloading`. It draws nothing for `Empty`. The button click stops propagation, so it does not open the row page.
- The badge uses one named badge style with a text word. Use an existing primitive if one fits. Otherwise add a neutral row badge beside `render_state_badge` in `src/ui/primitives/status_badge.rs`.
- The `MusicBrainz` icon uses a named color role and its own icon shape. The tooltip and the accessibility label hold the word.

### The icons

Add these names to `IconName` in `src/ui/icons.rs`:

| `IconName` | Lucide icon | Source |
|---|---|---|
| `CloudDownload` | `cloud-download` | `extra_icon` and `ExtraLucideIcons` |
| `Queued` | `clock` | `extra_icon` and `ExtraLucideIcons` |
| `InProgress` | `loader-circle` | `component_icon` |
| `MbPending` | `hourglass` | `extra_icon` and `ExtraLucideIcons` |
| `MbLookingUp` | `scan-search` | `extra_icon` and `ExtraLucideIcons` |
| `MbNoMissingFields` | `circle-check` | `component_icon` |
| `MbDone` | `badge-check` | `extra_icon` and `ExtraLucideIcons` |
| `MbSkipped` | `circle-x` | `component_icon` |

The names are a proposal. Use the semantic name, not the shape name, if the icon guard asks for it.

## Required Changes

1. Change `TrackRowColumn` in `src/ui/tokens.rs` to `DownloadState`, `Duration` and `Menu`. Delete the other variants. Replace the two column tests.
2. Add the icon names and the extra icons.
3. Add the Index track sets and `index_track_phase` to `DownloadActivity`. Fill them in `DownloadQueue::activity`.
4. Change `ReleaseTrackRowVm` and its constructors to the fields of "The view model". Delete `ReleaseTrackPrimaryVm` and `ReleaseTrackStatusVm`.
5. Change `ReleasePageCommand` and both host routes.
6. Add `track_row_menu.rs`. Change `TrackRow` to the new columns, the badges, the hover rule and the right-click menu.
7. Change `render_release_page_track_row` and `render_single_row` to the new builders. Give the return focus to the row.
8. Update the tests that pin the present row with the same meaning. Add the tests and guards of this packet.

## Mechanical Acceptance Criteria

Use the prefix `adr_0083_row_actions_` for unit tests beside the owning code. Each new guard cites ADR 0083 Decision 6 and is situational.

| Case | Required proof |
|---|---|
| R83-6-01 | `ReleaseTrackRowVm.menu` gives the items in the order of D2. A downloaded idle Library row gives "Add to playlist", "Inspect sources" and "Remove…". A not-downloaded idle Library row gives "Add to playlist", "Download" and "Inspect sources". An Index row gives "Add to playlist" and "Download" |
| R83-6-02 | "Remove…" is the last item, `destructive` is true, the label ends with "…", and the accessibility label names the track. It is absent for `Queued`, `Downloading` and `Removing` |
| R83-6-03 | "Download" is absent for a downloaded row and for `Queued`, `Downloading` and `Removing` |
| R83-6-04 | "Inspect sources" shows only on a Library row with a track key, also on a singles group row. Its command targets `FrameNavigationEntry::Track` of the row track |
| R83-6-05 | `DownloadSlotVm` gives `Download`, `Queued`, `Downloading` or `Empty` for each pair of `is_in_library` and `TrackRowWork`. Each non-empty state has a tooltip and an accessibility label. `available` is true only for `Download` |
| R83-6-06 | `DownloadActivity::index_track_phase` gives `Queued` and `InFlight` for an Index track job. An Index row reads its own phase, then the release phase. A job of another track of the release leaves the row at `Download` |
| R83-6-07 | In `Selecting`, `menu` is `None` and `download` does not change. In `Normal`, `menu` is `Some` |
| R83-6-08 | `badges` holds "New" for a not-downloaded Library row and "Removing" during a removal. An Index row has no badge |
| R83-6-09 | Each `MbTrackStatus` gives a different `MbStatusIcon` and a label that starts with "MusicBrainz: ". No two states share one icon |
| R83-6-10 | A GPUI layout test of `TrackRow`: each `TrackRowColumn` cell has its token width when it is empty, filled or hidden, and in `Selecting`. Each cell starts at the same x position on each row. `Duration` is before `Menu` |
| R83-6-11 | `TrackRowColumn::ROW_ORDER` is `[DownloadState, Duration, Menu]`. Each width does not get smaller at a larger `ScaleFactor` |
| R83-6-12 | `menu_cell_visibility` gives always-visible for a focused row or an open menu, and on-hover for the other cases |
| R83-6-13 | A source guard: `track_row_menu.rs` builds the "⋯" items and the right-click items with one function. `TrackRow` attaches the right-click handler only when the menu is `Some` |
| R83-6-14 | A source guard: `render_release_page_track_row` makes no `AddToPlaylistPopover` and no text "Remove" button. `ReleaseTrackPrimaryVm` and `ReleasePageCommand::TrackPrimary` are absent from `src/` |
| R83-6-15 | Each packet 018, 021, 022 and 025 test still passes. The tests of "Tests that pin the present row" keep their meaning with the new fields |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: on a release page, the pointer on a row shows "⋯". The pointer off the row hides it. No other row control moves.
- V2: the keyboard focus on a row (Up and Down in the detail column) shows "⋯" on that row only.
- V3: a right-click on a row opens a menu with the same items, the same order, the same divider and the same danger color as "⋯".
- V4: the "⋯" menu stays open when the pointer moves from the row into the menu.
- V5: "Add to playlist" opens the playlist choices. A choice adds the track. "New playlist…" makes a playlist and adds the track.
- V6: the download slot shows the download icon on a track that is not downloaded. After a click, it shows the queued icon, then the downloading icon, then nothing. Each icon has a tooltip.
- V7: "Remove…" asks for confirmation. After the confirmation, the row shows the "New" badge and the download icon with no navigation.
- V8: the "New" badge shows after the title. Each `MusicBrainz` icon shows its word in a tooltip.
- V9: the download slot, the duration and "⋯" start at the same x position on each row. Check a release page with track numbers, a singles group page with no numbers and an Index release page.
- V10: "Select" hides "⋯" and the right-click menu. The checkbox shows. "Done" shows them again.
- V11: "Inspect sources" opens the track page with the "Inspect sources" disclosure open.
- V12: Light and Dark themes, a narrow width and the larger type sizes. No text is clipped, and no column overlaps another. Each state has a word or a tooltip, not only a color.

## Exclusions

- The playlist page rows and the `ListRow` track lists (Tracks, Needs attention). ADR 0083 task 007, not written, owns them (D1).
- A drag of a track onto a sidebar playlist. ADR 0083 task 008, not written, owns it (D15).
- A key that opens the row menu (D14).
- A progress ring and the motion of the downloading icon (D7).
- The audition place of ADR 0084 Decision 11. It stays empty.
- No change to the selection model, the action bar, the download queue rules or the release page header.
- No schema change.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/ui/composites/track_row.rs`, `src/ui/composites/list_row.rs`, `src/ui/composites/playlist_popover.rs`.
- `src/ui/primitives/context_menu.rs`, `src/ui/primitives/button.rs`, `src/ui/primitives/tooltip.rs`, `src/ui/primitives/status_badge.rs`.
- `src/ui/tokens.rs` (`TrackRowColumn`, `Size::PointerHitTarget`), `src/ui/icons.rs`.
- `src/ui/shells/release_page.rs`, `src/ui/shells/library/singles_group.rs`, `src/ui/shells/library/track_detail_metadata.rs`.
- `src/ui/composites/selectable_text.rs`: the one present right-click menu.
- `src/view_models/release_page.rs`, `src/view_models/library.rs` (`LibraryTrackRowVm`, `TrackRowWork`, `BusyItems`), `src/view_models/download_progress.rs`.
- `src/library/release_page.rs`, `src/library/app_impl.rs` (`remove_track_after_confirmation`, `toggle_inspect_sources`), `src/library/download_queue.rs`, `src/library/detail_selection.rs`.
- `src/app/search_dispatch.rs` (`render_index_release_page` and the Index route).
- `tests/architecture_tests.rs`: the guards in "Recorded Facts".

## Checks

```bash
cargo test --lib adr_0083_row_actions_
cargo test --lib adr_0084_release_page_
cargo test --lib adr_0084_download_queue_
cargo test --lib adr_0084_detail_selection_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the commit. This packet adds no migration and no stored data. V5 and V7 change a playlist and the Library. Use a test playlist and a release that you can download again.

## Implementation Result - 2026-10-20

### Files Changed

- `src/ui/tokens.rs`: `TrackRowColumn` has the variants `DownloadState`, `Duration` and `Menu`, in this order.
  - `DownloadState` and `Menu` have the width of `Size::PointerHitTarget` on the CHROME scale. `Duration` keeps 44 px of text at `FontSize::Micro`.
  - `PrimaryAction`, `State`, `PlaylistAction`, `text_px`, `font_size` and `inset` are deleted.
- `src/ui/icons.rs`: new `IconName` values `CloudDownload`, `Queued`, `InProgress`, `MbPending`, `MbLookingUp`, `MbNoMissingFields`, `MbDone` and `MbSkipped`.
  - Five icons come from the complete catalog. `ExtraLucideIcons` embeds them. Three come from the default bundle. The icon tests list each new name.
- `src/view_models/download_progress.rs`: new `DownloadActivity` sets `index_tracks` and `in_flight_index_tracks`, and new `index_track_phase`.
- `src/library/download_queue.rs`: `DownloadQueue::activity` fills the two new sets from each `JobIdentity::IndexTrack` with `IndexTrackIdentity::Guid`. New test of the Index track phase.
- `src/view_models/library.rs`:
  - New `BusyItems::index_track_work` and `LibraryTrackRowVm::mb_status`.
  - `MbStatusKind`, `mb_status_text`, `mb_status_kind` and `LibraryTrackRowVm::playlist_display` are deleted with their three tests. No caller was left.
- `src/view_models/release_page.rs`:
  - `ReleaseTrackRowVm` has the fields `badges`, `mb_status`, `download`, `menu` and `inspect_sources_entry`.
  - New types `TrackRowBadge`, `TrackRowBadgeVm`, `MbStatusIcon`, `TrackRowMbStatusVm`, `DownloadSlotState`, `DownloadSlotVm`, `TrackRowMenuAction`, `TrackRowMenuItemVm`, `TrackRowPlaylistLevelVm` and `TrackRowMenuVm`. Each label is a named constant.
  - `ReleaseTrackPrimaryVm`, `ReleaseTrackStatusVm` and the `primary`, `status`, `state_label`, `playlist_popover_id` and `playlist_trigger_label` fields are deleted.
  - `ReleasePageCommand::TrackPrimary` is deleted. New commands are `DownloadTrack`, `RemoveTrack` and `InspectSources`.
  - New `ReleasePageFacts::with_index_track_work`, new `inspect_sources_entry`, and a new `work` argument of `ReleaseTrackRowVm::index`.
  - Tests R21-03, R21-04 and R21-06 use the new fields with the same meaning. New tests R83-6-01 to R83-6-09.
- `src/ui/primitives/context_menu.rs`:
  - A menu item can have a second level (`ContextMenuItem::level`). The "⋯" menu and the pointer menu show that level in the same surface.
  - New `ContextMenu::trigger_tooltip` and `ContextMenu::is_open`.
- `src/ui/primitives/mod.rs`: exports `MenuLevelControls`.
- `src/ui/primitives/status_badge.rs`: new `render_row_badge`, a neutral badge with a word.
- `src/ui/composites/playlist_popover.rs`: `build_list_mode` and `build_create_mode` are `pub(crate)`.
  They take `PlaylistChoiceControls` in place of the private popover state, so the row menu uses the same two modes.
- `src/ui/composites/track_row_menu.rs` (new):
  - `track_row_menu_items` builds the items of both menus. The "Add to playlist" level has "Back", the playlist choices and "New Playlist".
  - `track_row_pointer_menu`, `TrackRowMenuCell`, `TRACK_ROW_GROUP` and the pure `menu_cell_visibility`. Test R83-6-12.
- `src/ui/composites/mod.rs`: registers the new module and exports `TrackRowMenuHandlers`.
- `src/ui/composites/track_row.rs`:
  - The builders are `badges`, `mb_status`, `download_slot`, `on_download`, `menu` and `on_menu_action`. `primary_action`, `state_child` and `playlist_action` are deleted.
  - The title line holds the title, the badges and the `MusicBrainz` icon. The trailing columns are the download slot, the duration and the menu cell.
  - The row root is the GPUI group `track-row`. The row root attaches the right-click handler only when the row has a menu.
  - The download and the menu cells have the minimum height of a hit target, so each row has the same height in each state.
  - Tests R83-6-10 and R83-6-11 replace the two old column tests.
- `src/ui/shells/release_page.rs`:
  - `render_release_page_track_row` builds the row from the new fields and takes the return focus of the right-click menu.
  - `ReleasePageImages` has the new field `row_menu_focus`.
- `src/ui/shells/library/detail.rs`, `feed_detail.rs` and `singles_group.rs`: give `LibraryApp::detail_column_focus` to the rows.
- `src/library.rs` and `src/library/app_impl.rs`:
  - New field `pending_inspect_sources`, and new `request_inspect_sources` and `busy_items`.
  - `select_track_detail` opens the "Inspect sources" disclosure of the requested track when its page mounts.
- `src/library/release_page.rs`: `DownloadTrack` calls `download_library_tracks`. `RemoveTrack` calls `remove_track_after_confirmation`. `InspectSources` asks for the disclosure and opens the track page.
- `src/app/search_dispatch.rs`: the Index release page reads the phase of each row from `LibraryApp::busy_items` and gives the return focus. `DownloadTrack` calls `download_index_track`. The route ignores `RemoveTrack` and `InspectSources`.
- `tests/architecture_tests.rs`:
  - New guards R83-6-13 and R83-6-14. Each one is situational and cites ADR 0083 Decision 6.
  - The ADR 0063 pointer menu guard reads the new call `build_menu_level(` and the shared `build_menu_content(` call. See deviation 4.

### Behavior Changed

- A track row on a release page of each origin and on a singles group page shows no "Remove" text button and no "+ Playlist" button.
- The row always shows the number, the cover, the title, the badges and the `MusicBrainz` icon. It also always shows the date line, the download slot and the duration.
- The download slot shows a download button on a track that is not downloaded. It shows the queued icon or the downloading icon while a job waits or runs. It is empty for a downloaded track and during a removal.
  - Each state has a tooltip. A click on the button queues the track and does not open the row page.
- An Index row reads the phase of its own Index track job, then the phase of a release job of its key.
- The "New" badge shows after the title of a Library track that is not downloaded. The "Removing" badge shows during a removal. An Index row shows no badge.
- Each `MusicBrainz` status shows its own icon. The word is in the tooltip.
- The "⋯" button shows on hover of the row, on the keyboard focus of the row and while its menu is open. A hidden button keeps its width.
- The "⋯" menu and a right-click on the row show "Add to playlist", "Download", "Inspect sources", a divider and "Remove…". An item that cannot act is absent.
  - "Add to playlist" shows the playlist choices in the same menu surface.
  - "Remove…" always asks for confirmation.
  - "Inspect sources" opens the track page with the "Inspect sources" disclosure open.
- In selection mode, the row shows no "⋯" button and no right-click menu. The download slot stays.
- Each track row is the same height in each state. Each row has a download cell and a menu cell with the height of a hit target.

### Checks - 2026-10-20

| Check | Result |
|---|---|
| `cargo test --lib adr_0083_row_actions_` | Green, 13 tests |
| `cargo test --lib adr_0084_release_page_` | Green, 8 tests |
| `cargo test --lib adr_0084_download_queue_` | Green, 21 tests |
| `cargo test --lib adr_0084_detail_selection_` | Green, 9 tests |
| `cargo test` | Green, 2064 unit tests and 344 guards. Ten documentation examples stay ignored |
| `cargo test --test architecture_tests` | Green, 344 guards |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo clippy --all-targets -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

No application launch and no production-data change occurred.

### Deviations From The Task

1. The track page mounts after the navigation event of `open_page`, not in the same call. Thus `InspectSources` records the track id in `pending_inspect_sources`. `select_track_detail` opens the disclosure when the page of that track mounts.
2. The "⋯" menu and the right-click menu need a second level. `ContextMenu` and `PointerContextMenu` had no such level, so `ContextMenuItem::level` is new.
   The row menu state (the pointer position and the create mode) is keyed state of the row in `track_row_menu.rs`.
3. `ContextMenu::is_open` reads the keyed state of the menu at its element position. Thus the menu cell is its own component, `TrackRowMenuCell`. It reads the open state and draws the menu at the same position.
4. The ADR 0063 guard `adr_0063_log_copy_menu_preserves_selection_and_uses_typed_actions` required the literal text `build_menu_content(&dismiss, self.items, cx)`.
   The pointer menu now calls `build_menu_level`, and that function calls `build_menu_content`. The guard now requires the two new calls. Its meaning does not change: the pointer menu uses the shared item rows.
5. The R83-6-10 layout test compares the focused row with the cell positions moved by 1 px. The focus outline of `ListRow` is 2 px, and the plain border is 1 px. This packet does not change `ListRow`.
6. The duration cell is a sibling of the body now, not a child of the body. The width and the order of each column follow D13.
7. The packet lists "New playlist…" in the playlist level. The shared list mode shows its own label "New Playlist", because this packet reuses that mode and does not copy it.
8. `mb_status_text`, `mb_status_kind`, `MbStatusKind` and `LibraryTrackRowVm::playlist_display` had no caller after the change. They are deleted with their tests.
9. The download and the menu cells have the minimum height of a hit target (44 px). A row with a download button was taller than a row with no button. With this height, each row has the same height.

### Unresolved Concerns

1. V1 to V12 below need the operator. No test proves a rendered hover, a tooltip, a menu position or the danger color.
2. The download button and the "⋯" button are wider than the 28 px column at the medium scale. The column clips the hover fill of the button at its two sides. V12 examines this.
3. A track row is 10 px taller than the 44 px floor. The cause is the two cells with the height of a hit target in each row. Before this packet, a row with a text button had the same height.
4. The new file `src/ui/composites/track_row_menu.rs` is not in `docs/architecture/source-map.md`. The visual checks V1 to V12 are not in `docs/pending-human-checks.md`. The task instructions named only this packet as a document to edit.
5. A right-click on a row in selection mode does nothing. A right-click on a row with no handler, for example a test row, also does nothing.

## Operator Visual Check

This check needs a Linux desktop session and this checkout. It also needs these items:

- A Library release with three or more tracks, partly downloaded, and with a `MusicBrainz` status on one track.
- An Index release that is not in the Library, with two or more tracks.
- A Library singles group with two or more singles.
- A test playlist.

V5 changes a playlist. V7 removes an audio file. Use a test playlist and a track that you can download again.

1. Build the app: `cargo build --bin v4vmm`. Then start `target/debug/v4vmm` from a desktop session.
2. Open Music, then Releases, in the sidebar. Open the partly downloaded Library release.

### V1: The hover

3. Move the pointer onto a track row. A "⋯" button must show at the right end of the row.
4. Move the pointer off the row. The "⋯" button must go.

**Wrong if:** the duration or the download slot moves when "⋯" shows or goes.

### V2: The keyboard focus

5. Click an empty area of the detail column. Push Down and Up.
6. The "⋯" button must show on the focused row only.

**Wrong if:** "⋯" shows on a row that has no focus and no pointer.

### V3: The right-click menu

7. Click "⋯" on a downloaded row and read the items. Push Escape.
8. Right-click the same row. The menu must open at the pointer.
9. Compare the two menus. Each one must show "Add to playlist", "Inspect sources", a divider and "Remove…", in this order. "Remove…" must be in the danger color in each menu.

**Wrong if:** the two menus show different items, a different order or a different color.

### V4: The open menu

10. Click "⋯" on a row. Move the pointer from the row into the menu.
11. The menu must stay open, and "⋯" must stay visible.

**Wrong if:** the menu closes or "⋯" goes when the pointer leaves the row.

### V5: Add to playlist

12. Open "⋯" and click "Add to playlist". The playlist choices must show in the same menu, with "Back" at the top.
13. Click the test playlist. Open the test playlist page. The track must be in it.
14. Do step 12 again on another row. Click "New Playlist", type a name, and click "Create & Add". A new playlist with the track must show in the sidebar.

**Wrong if:** a second menu opens, or the track is not in the playlist.

### V6: The download slot

15. Find a track that is not downloaded. Its slot must show the download icon. Hold the pointer on the icon. The tooltip "Download" must show.
16. Click the icon. The slot must show the queued icon, then the downloading icon, then nothing. The tooltips must read "Queued for download" and "Downloading".
17. The track page must not open after the click.

**Wrong if:** the click opens the track page, or a state has no tooltip.

### V7: Remove

18. Open "⋯" on a downloaded row and click "Remove…". A confirmation must open.
19. Confirm. The row must show the "New" badge and the download icon. The page must stay open.

**Wrong if:** the track goes with no confirmation, or the row changes only after a move to another page.

### V8: The badges and the `MusicBrainz` icon

20. On a track that is not downloaded, look for the "New" badge after the title.
21. Hold the pointer on the `MusicBrainz` icon of the track with a status. A tooltip must show the word, for example "MusicBrainz: done".

**Wrong if:** a badge or an icon has no word, or the badge covers the title.

### V9: The column positions

22. On the release page, compare the rows. The download slot, the duration and "⋯" must start at the same x position on each row.
23. Do the same on the singles group page, where no row has a number. Do the same on the Index release page.

**Wrong if:** a column starts at a different position on one row.

### V10: Selection mode

24. Click "Select" in the detail column header. Each row must show its checkbox. No row must show "⋯" on hover. A right-click must open no menu.
25. Click "Done". The "⋯" button on hover and the right-click menu must come back.

**Wrong if:** "⋯" or the right-click menu shows in selection mode.

### V11: Inspect sources

26. Open "⋯" on a Library row and click "Inspect sources". The track page must open with the "Inspect sources" disclosure open.
27. On the singles group page, do the same. The track page of the single must open, not its release page.
28. On the Index release page, open "⋯". The menu must not show "Inspect sources".

**Wrong if:** the page opens with the disclosure closed, or a different page opens.

### V12: Widths, themes and type sizes

29. Do steps 3, 15 and 22 in the Light theme and in the Dark theme.
30. Make the window narrow. Then set the largest type size in Settings and do the steps again.
31. Look at the hover fill of the download button and of "⋯".

**Wrong if:** text is clipped, a column overlaps another column, or a state shows only a color. Also wrong: the clipped hover fill of a button looks broken.

### Cleanup

32. Delete the new playlist of step 14. Download the removed track of step 19 again if you need it.

## Orchestrator Review - 2026-10-20

Pass, after one fix. The orchestrator made the fix in the same change.

- The download column and the menu column were 28 px (`Size::PointerHitTarget`), but each `Button` has a hit area of at least 44 px (`Size::MinHitTarget`). The column thus cut the hover fill of each icon button. Both columns now use `Size::MinHitTarget`, which changes decision D13. The test `adr_0083_row_actions_r83_6_11_row_order_and_scale` holds the widths.
- The deviations agree with the packet. "Inspect sources" opens the track page and then its disclosure, through `pending_inspect_sources`.
- `cargo fmt -- --check` and `cargo clippy --all-targets -- -D warnings` are Green. `cargo test` gives 2064 library tests and 344 guards, all passing.
- The orchestrator added `track_row_menu.rs` to the source map.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0083-task-006-row-hover-actions.md`
- ADR 0083 Decisions 5, 6 and 10, and ADR 0084 Decisions 10 and 11
- Packets 022 and 025, and the archived packet 018
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the columns, the icons, the Index phase, the view model, the commands, the menu and the row.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Follow each decision in "Orchestrator Decisions". Do not change a decision.
- The view model gives each label, tooltip, accessibility label and availability. The renderer only composes.
- Use named tokens and named icons. Never show a state by color only.
- Tests use recorded values. No test sends a request or writes a file.
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git add` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The playlist page rows in `src/ui/shells/playlist.rs` and `src/ui/shells/library/playlist_detail.rs`.
- `ListRow` and the Tracks and Needs attention lists, except to read them.
- The selection model, the action bar and the download queue rules.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R83-6-01 to R83-6-15 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - 2026-10-20" section and an "Operator Visual Check" section for V1 to V12.

Test commands:
- The commands in "Checks", with `cargo fmt -- --check` after the last edit and `cargo build --bin v4vmm` last.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- `group_hover` cannot make an `invisible()` cell visible.
- The "⋯" menu closes when the pointer leaves the row, and `ContextMenu::is_open` cannot keep it open.
- The track frame does not exist after the navigation of `InspectSources`, so "Inspect sources" cannot open.
- The Index row phase needs a `crate::library` import in a view model.
- The return focus of the right-click menu cannot reach `detail_column_focus`.
- A change needs a file in "Do not touch".
