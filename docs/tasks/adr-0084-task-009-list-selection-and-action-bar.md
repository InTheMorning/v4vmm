# ADR 0084 Task 009: Selection And The Action Bar In The List Column

Status: Implemented - 2026-10-18. Cases R9-01 to R9-10 pass. Mechanical checks are Green. The visual gate is open: an operator has not walked V1 to V7.

## Goal

The Releases list, the Tracks list and the Needs attention list share one selection model. A row shows a checkbox, and an action bar acts on the whole selection.
The keyboard moves through the rows, keeps the focused row in view, and selects rows.

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md) Decision 10: one shared selection model, the checkbox, the click and key rules, and the action bar with "Clear".
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 11: each track row keeps a place for audition. No control shows in it until [ADR 0068](../adr/0068-show-cue-and-audition-isolation.md), which is Proposed.
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 6: the checkbox of a singles group row selects each single of the group.
- The platform modifier: "Ctrl" in Decision 10 is Ctrl on Linux and Cmd on macOS. [ADR 0067](../adr/archive/0067-platform-shortcut-modifiers.md) decided it, and the guard `adr_0067_keyboard_and_menu_share_platform_modifier_resolution` enforces it. Each new binding goes through the shared platform adapter.
- Operator decisions of 2026-10-18:
  - Packet 009 covers the list column. Packet 018 covers the lists of the detail column.
  - The action bar offers "Download", "Add to playlist", "Remove" and "Clear". "Remove" opens one confirmation that lists each item.
- The durable set in [AGENTS.md](../../AGENTS.md): typed action state, renderer portability, token discipline, accessibility, and cautious destructive actions.

## Recorded Facts - 2026-10-18

- No list has a selection set. `LibraryViewModel` has `selected_id: Option<i64>` and `selected_playlist_id: Option<i64>` (`src/view_models/library.rs:2351`). They name the open page, not a row selection.
- `LibraryApp::move_up` and `move_down` (`src/library/app_impl.rs:1167` and `:1184`) call `focusable_items`, which returns an empty list. Thus Up and Down do nothing today. `select_id` only sets `selected_id`.
- `src/app/keyboard.rs` declares `MoveSelectionUp`, `MoveSelectionDown`, `ConfirmSelection` and `CancelActivePane`. `APP_KEY_BINDING_SPECS` binds Up, Down, Enter and Escape in the `ActivePane` key context. The guard `top_level_keyboard_shortcuts_route_through_key_binding_taxonomy` asserts these names.
- No row has a focus handle. One key context, `ActivePane`, wraps the workspace content (`src/app.rs`).
- The Releases list: `releases_list` gives `ReleasesListEntryVm` (`Release` or `SinglesGroup`), and `render_library_releases_list` in `src/ui/shells/library/destination_lists.rs` draws it with `ListRow`.
- The Tracks list and the Needs attention list: `ContentListPageVm` and `ContentListRowDisplay`, drawn by `render_content_list_row` in `src/ui/shells/library/content_list.rs` with `ListRow`. A row click calls `open_content_list_row`.
- No checkbox primitive and no bar composite exist in `src/ui/primitives` or `src/ui/composites`.
- Batch actions: `add_albums_to_playlist(feed_ids, playlist_id)` exists (`src/library/app_impl.rs:1770`). `download_feed`, `add_track_to_playlist` and the removal path (`LibraryRemovalIntent`, `LibraryRemovalPlan`) take one item.
- Scroll: the list column uses `music_column_scrollbar`, with no held `ScrollHandle`. Only the Tracks list holds one, `recent_music_scroll`, for its paging probe. No code keeps a row in view. The guards `adr_0084_scrollbars_r14_01` and `r14_03` limit where scroll code lives.
- The hover state `hovered_thumb_url` serves only the animated cover of a Releases row.
- No code contains "audition".

## Design

### The selection model

Add `src/view_models/list_selection.rs`. It is pure and has no renderer type.

```rust
/// The selection of one list (ADR 0084 Decision 10).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ListSelection<K: Clone + Eq + Ord> {
    selected: BTreeSet<K>,
    anchor: Option<K>,
    focus: Option<K>,
}
```

Each operation takes the visible row keys in their sequence, so the model knows no list type:

| Operation | Effect |
|---|---|
| `click(key)` | No change to the selection. The caller opens the page of the row. The focus moves to the row |
| `toggle(key)` | Ctrl+click and the checkbox. Adds or removes the row. The anchor and the focus move to it |
| `extend_to(key, visible)` | Shift+click. Selects each row from the anchor to `key`, in the visible sequence |
| `select_all(visible)` | Ctrl+A |
| `toggle_focused()` | Space |
| `move_focus(step, visible)` | Up, Down, Page Up, Page Down, Home and End. Returns the new focused key, so the caller keeps it in view |
| `clear()` | "Clear" and Escape |
| `retain_visible(visible)` | Drops each selected key that the list no longer shows, for example after a removal or a filter |

- A page step is 10 rows. A named constant holds it.
- The model gives `selected_count()`, `is_selected(key)`, `focused()` and `shows_checkboxes()`. `shows_checkboxes()` is true when a selection exists.

### The keys

- A Releases row has the key of its release (`ReleaseKey`). A singles group row stands for the keys of each single of the group. Its checkbox is selected when each single is selected, and `toggle` adds or removes all of them.
- A Tracks row and a Needs attention row have the key of the track (the present track row key of `content_list_track_row_key`).
- `LibraryApp` holds one `ListSelection` for the release list and one for the track lists. A change of destination clears both.

### The row

- `DestinationListRowVm`, `SinglesGroupRowVm` and `ContentListRowDisplay` get `selection: RowSelectionVm`: `checkbox_visible`, `checked`, `focused`, and the accessibility label of the checkbox, for example "Select Face Pale".
- `checkbox_visible` is true when a selection exists, or when the row has the keyboard focus. The screen also shows the checkbox while the pointer is on the row. Pointer hover is the screen's own state, so it needs no view model field.
- Add a checkbox primitive `src/ui/primitives/checkbox.rs`: a typed checked state, an accessibility label, a click handler, named tokens for its size, its border and its check mark icon. The check mark is a named icon, not a glyph string.
- `ListRow` gets an optional leading checkbox slot. A click on the checkbox calls `toggle` and does not open the page.
- `ListRow` also gets the audition slot of Decision 11 for a track row. This packet puts nothing in it, so no control shows and no space shows.

### The clicks

- A plain click opens the page of the row, as today, through `NavigationPathStart::ListRow`.
- Ctrl+click calls `toggle`. Shift+click calls `extend_to`. Neither opens a page.
- Read the modifiers from the GPUI click event through the shared platform adapter.

### The keys

- Up and Down call `move_focus(±1)`. Page Up and Page Down call `move_focus(±10)`. Home and End go to the first row and the last row.
- Space calls `toggle_focused`. Ctrl+A calls `select_all`. Enter opens the page of the focused row. Escape clears the selection, and keeps the present Escape behavior when no selection exists.
- Add each new action to `src/app/keyboard.rs` and to `APP_KEY_BINDING_SPECS`, in the `ActivePane` context.
- The new keys act only while the Music list column has the focus.
- After each focus move, the list scrolls so that the focused row is in view. Use a held `ScrollHandle` for the Releases list and for the track lists, through `music_column_tracked_scrollbar`. Update the r14 guards for the new file names, and keep their rules.
- Delete `move_up`, `move_down`, `focusable_items` and `select_id` when no caller remains.

### The action bar

Add `SelectionActionBarVm` in `src/view_models/` and a composite `src/ui/composites/selection_action_bar.rs` at the bottom of the list column.

- The bar shows only when a selection exists. Its count text names the item kind: "7 tracks selected", "3 releases selected", "1 release selected".
- Each action is a typed action with a label, an accessibility label and an availability. An action that cannot act is absent.

| Action | Label example | Shows when | Effect |
|---|---|---|---|
| Download | "Download 5 tracks" | One or more selected items are not in the Library. The count names only those | Downloads each of them, newest first, through the present download path |
| Add to playlist | "Add 7 tracks to playlist" | Always with a selection | Opens the present playlist popover. The choice adds each item: `add_albums_to_playlist` for releases, `add_track_to_playlist` for each track |
| Remove | "Remove 7 tracks…" | One or more selected items are in the Library | Opens one confirmation (below) |
| Clear | "Clear" | Always with a selection | Clears the selection |

- After an action, the selection stays, so the operator can do a second action. A removal drops the removed items through `retain_visible`.
- The list updates in place after each action (AGENTS.md: current-view state updates in place).

### The removal confirmation

- One confirmation for the whole selection, with `RemovalConfirmation::Always`. Its title names the count, for example "Remove 7 tracks from the Library?".
- It lists each item by its title, and names each playlist that refers to an item. It reuses the present impact text of `LibraryRemovalPlan` for each item.
- "Remove" removes each item through the present removal path, one `LibraryRemovalIntent` for each. "Cancel" changes nothing.
- When one removal fails, the others continue. The report names each failed item and its error.

## Required Changes

1. Add `list_selection.rs` and its tests.
2. Add the checkbox primitive and the `ListRow` checkbox slot and audition slot.
3. Add `RowSelectionVm` to the three row view models, and the two `ListSelection` values to `LibraryApp`.
4. Wire the clicks and the keys. Delete the dead selection code.
5. Add the held scroll handles and keep the focused row in view.
6. Add `SelectionActionBarVm`, its composite and its four actions.
7. Add the removal confirmation for a selection.
8. Update each guard that names the deleted code, and add the guards of the criteria below.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_selection_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R9-01 | `toggle`, `extend_to`, `select_all`, `toggle_focused` and `clear` give the selection of Decision 10 on a recorded list of 6 keys. `extend_to` follows the visible sequence |
| R9-02 | `move_focus` gives the next key for ±1, ±10, Home and End, and stops at the first and the last row |
| R9-03 | `retain_visible` drops each key that the list no longer shows |
| R9-04 | A singles group row is checked when each of its singles is selected. `toggle` on it adds or removes each single |
| R9-05 | `RowSelectionVm` shows the checkbox for a row with the focus, and for each row while a selection exists. It has the label "Select <title>" |
| R9-06 | `SelectionActionBarVm` gives the count text, and the four actions with their labels. "Download" counts only the items not in the Library, and is absent when each item is in the Library |
| R9-07 | "Remove" is absent when no selected item is in the Library. The confirmation lists each item and each referring playlist |
| R9-08 | A change of destination clears the two selections |
| R9-09 | A guard finds no `move_up`, `move_down` or `focusable_items` in `src/library/`. It cites ADR 0084 Decision 10 |
| R9-10 | The keyboard guard still finds each action name. The new actions are in `APP_KEY_BINDING_SPECS` |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: in Releases, a pointer on a row shows its checkbox. A click on the checkbox selects the row and opens no page. Then each row shows its checkbox, and the action bar shows "1 release selected".
- V2: Ctrl+click adds a row, Shift+click adds a range, and Ctrl+A selects the list. A plain click still opens the page.
- V3: in Tracks, Up and Down move the focus, and the list scrolls to keep the focused row in view. Page Up, Page Down, Home and End work. Space selects the focused row. Enter opens it. Escape clears the selection.
- V4: "Download n tracks", "Add n tracks to playlist" and "Clear" act on the selection, and the list updates in place.
- V5: "Remove n tracks…" opens one confirmation that lists each track and each playlist that refers to one. "Cancel" changes nothing. "Remove" removes each track, and the list updates in place. Use test tracks that you can download again.
- V6: a singles group row selects each of its singles.
- V7: check normal and narrow widths, Light and Dark themes, and the larger type sizes. The checkbox, the row text and the bar stay in place, with no clipped text. The checked state reads without color alone.

## Exclusions

- No selection in the detail column lists: the release page track rows, the singles group page, the feed page, the playlist page and the search results. Packet 018 owns them, and the keyboard scroll of the detail column.
- No audition control. ADR 0068 owns it.
- No selection in the Artists list. Decision 10 names lists of releases and tracks.
- No schema change.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/view_models/library.rs` (`releases_list`, `ContentListPageVm`, the row view models), `src/library/app_impl.rs` (the row open functions, `move_up`, `move_down`, the download, playlist and removal paths, `render_list_column`).
- `src/ui/shells/library/destination_lists.rs`, `src/ui/shells/library/content_list.rs`, `src/ui/composites/list_row.rs`, `src/ui/composites/music_column_scroll.rs`, `src/ui/shells/library/content_list_scroll.rs`.
- `src/app/keyboard.rs`, `src/app.rs` (the key context).
- `src/application/library_removal.rs`, `src/view_models/library_removal.rs`.
- `src/view_models/singles_group.rs`.
- `tests/architecture_tests.rs`: the keyboard guard, the r14 scroll guards, `adr_0084_columns_list_rows_do_not_shrink_in_a_scrolling_column`.

## Checks

```bash
cargo test --lib adr_0084_selection_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the commit. This packet adds no migration and no stored data. A removal in V5 deletes audio files. Use tracks that you can download again.

## Implementation Result - 2026-10-18

### Files Changed

- `src/view_models/list_selection.rs` (new): `ListSelection<K>`, `FocusStep`, `RowSelectionVm`, and the R9-01 to R9-05 tests.
- `src/view_models/selection_action_bar.rs` (new): `SelectionActionBarVm`, `SelectionActionKind`, and the R9-06 and R9-07 tests.
- `src/ui/primitives/checkbox.rs` (new): the `Checkbox` primitive.
- `src/ui/composites/selection_action_bar.rs` (new): the `SelectionActionBar` composite.
- `src/ui/shells/library/destination_list_scroll.rs` (new): the Releases list's two scroll containers, split out of `destination_lists.rs` for the ADR 0038 line ceiling.
- `src/view_models/workspace/nav.rs`: `ReleaseKey` and `TrackKey` gain `Ord` and `PartialOrd`, for the `BTreeSet` of a `ListSelection`.
- `src/view_models/singles_group.rs`: `SinglesGroupRowVm` gains `single_keys` and `selection`.
- `src/view_models/library.rs`: `DestinationListRowVm` and `ContentListRowDisplay` gain `selection`. New methods: `releases_list_with_selection`, `content_list_rows_with_selection`, `releases_selection_action_bar`, `content_list_selection_action_bar`, `releases_selected_feed_ids`, `releases_removable_feed_ids`, `releases_downloadable_feed_ids`, `content_list_selected_tracks`. Deleted the dead accessor `selected_id()`.
- `src/view_models/library_removal.rs`: the shared function `removal_impact_message`, and the new `SelectionRemovalConfirmationState`, `SelectionRemovalPendingItem` and `SelectionRemovalItemDisplay` for a whole-selection removal.
- `src/application/library_removal.rs`: the new function `removal_referring_playlist_names`.
- `src/db.rs` and `src/library_service.rs`: the new read-only queries `playlist_names_referencing_track` and `playlist_names_referencing_feed`.
- `src/ui/shells/library_removal_confirmation.rs`: `open_selection_removal_confirmation_dialog`, built on the existing shared `confirmation_dialog` composite.
- `src/library.rs`: new `LibraryApp` fields `releases_selection`, `track_selection`, `releases_list_scroll`, `list_column_focus`, `selection_removal`.
- `src/library/app_impl.rs`: the click, checkbox, keyboard and action bar methods of ADR 0084 Decision 10. Deleted the dead `move_up`, `move_down`, `confirm`, `select_id` and `focusable_items`.
- `src/ui/shells/library/destination_lists.rs` and `src/ui/shells/library/content_list.rs`: the checkbox slot, the pointer-hover reveal, and the Ctrl and Shift click handling of each row.
- `src/ui/composites/list_row.rs`: the `checkbox` and `audition` slots, and the pointer-hover reveal of a checkbox that is not forced visible.
- `src/app/keyboard.rs` and `src/app.rs`: the six new selection actions, and `primary_modifier_held`, the shared Ctrl or Cmd click reader.
- `src/ui/tokens.rs`: the new size token `CheckboxIndicator`.
- `tests/architecture_tests.rs`: the R9-08, R9-09 and R9-10 guards. It also updates the keyboard taxonomy guard and the r14-01 file group for the new scroll file.

### Behavior Changed

- The Releases list, the Tracks list and the Needs attention list share one selection model (ADR 0084 Decision 10).
- A row's checkbox shows when a selection exists, when the row has the keyboard focus, or when the pointer is on the row.
- Ctrl+click (Cmd+click on macOS) toggles a row. Shift+click extends the selection. A plain click still opens the row's page.
- Up, Down, Page Up, Page Down, Home and End move the keyboard focus and keep the focused row in view. Space toggles the focused row. Ctrl+A (Cmd+A) selects every visible row. Escape clears the selection first, then falls back to the present Back action.
- A singles group row's checkbox selects or clears every single of the group.
- An action bar shows at the bottom of the list with a selection: "Download", "Add to playlist", "Remove" and "Clear", each shown only when it can act.
- "Remove" opens one confirmation that lists each item, its impact text, and each playlist that refers to it. "Remove" removes each item. One failure does not stop the rest.
- A change of destination clears both selections. A removal or a reload drops a removed item from the selection it was in.

### Checks - 2026-10-18

| Check | Result |
|---|---|
| `cargo test --lib adr_0084_selection_` | Green, 13 tests |
| `cargo test` | Green, 1961 unit tests and 323 guards. Ten documentation examples stay ignored |
| `cargo test --test architecture_tests` | Green, 323 guards |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo check --all-targets` | Green |
| `cargo build --bin v4vmm` | Green |

No application launch and no production-data change occurred.

### Deviations From The Task

1. An expanded single row under an open group keeps `RowSelectionVm::none()`. Only the group row's own checkbox selects or clears the group. This matches R9-04's wording, which names only the group row's checkbox.
2. The Needs attention list's row body still accepts no click, the same as before this packet. Ctrl+click and Shift+click act only on a row that already opens a page. The checkbox still toggles a Needs attention row, with no row click needed.
3. "Download" is never present on the Tracks or the Needs attention action bar. Only a Library track row resolves a key for `content_list_track_row_key`, so a track in either list is always already in the Library.
4. "Download n releases" reuses the existing queued download path, `download_singles`, which the singles group page's main action already used. The queue runs one download at a time, as before.
5. A removal that fails during a batch of several does not stop the rest, but the status line shows only the most recent error. Each item's own earlier state, for example a busy feed mark, still clears correctly. A combined report of each failed item and its own error text is not built. See "Unresolved Concerns".
6. `scroll_to_item` finds a row by its index in the scrolled element's direct children. `render_content_list_rows` and the new tracked Releases container now give the scrolled element its id and padding directly. Neither one uses a wrapping div, so each visible row is a direct child.

### Unresolved Concerns

1. V3's keyboard scroll-to-item behavior, and V1, V2, V6 and V7's visual checks, need the operator. No test proves a GPUI scroll position or a rendered color.
2. The removal of several items in one batch reports only the last error when more than one fails. A future packet could give `SelectionRemovalConfirmationState` a named error per item, if the operator meets this case in practice.
3. The sort order of "Download n releases" uses each release's newest track `pub_date`. A release with no dated track sorts last, the same rule as the existing singles group order.

## Operator Visual Check

This check needs a Linux desktop session and this checkout. It also needs a Library with one Releases singles group, six or more Library tracks, and one feed with more than one track. In that feed, not every track is downloaded.

**V5 removes files from the Library.** Pick a small, low-value track or release for V5, one you do not mind removing. The cleanup step tells you how to download it again.

1. Build and start the app: `cargo build --bin v4vmm` then run the built `target/debug/v4vmm` binary from a desktop session.
2. Open Music, then Releases, in the sidebar.

### V1: Releases checkbox and the action bar

3. Move the pointer over a release row with no selection active. Look for its checkbox at the left of the row. Move the pointer off the row. The checkbox goes away.
4. Click the checkbox of one release row. Look for the checkbox filled with a check mark, and for no page opening. An action bar must appear at the bottom of the list, with the text "1 release selected".
5. Look at every other row in the list. Each one now shows its own checkbox, filled only for the one you picked.

**Wrong if:** a click on the checkbox opens the release page, or no action bar appears, or another row's checkbox looks filled.

### V2: Ctrl+click, Shift+click, Ctrl+A, and a plain click

6. With the one release still selected, hold Ctrl (Cmd on macOS) and click a second row. Look for two checked rows and the count text "2 releases selected".
7. Hold Shift and click a row several rows below the last one you clicked. Look for every row between the two clicks becoming checked.
8. Press Ctrl+A (Cmd+A on macOS). Look for every row in the list becoming checked.
9. Click "Clear" in the action bar. Look for every checkbox going back to unchecked and the action bar going away.
10. Click a release row with no modifier key held. Look for the release page opening, with no checkbox involved.

**Wrong if:** Ctrl+click opens a page, or Shift+click selects the wrong range. Also wrong: Ctrl+A misses a row. Also wrong: a plain click selects a row and does not open its page.

### V3: Tracks keyboard focus, scroll, and Escape

11. Open the Tracks destination in the sidebar. Click once on a track row, then press Escape, so the list column itself holds the keyboard focus.
12. Press Down several times. Look for the keyboard focus moving one row down each time, and the list scrolling so the focused row stays in view.
13. Press Page Down. Look for the focus jumping about ten rows down. Press Home. Look for the focus jumping to the first row, with the list scrolled to the top.
14. Press End. Look for the focus jumping to the last row, with the list scrolled to the bottom.
15. Press Space. Look for the focused row's checkbox becoming checked and an action bar appearing.
16. Press Enter. Look for the focused row's page opening.
17. Return to the Tracks list, select two or more rows, then press Escape. Look for the selection clearing, with no change to the open page.

**Wrong if:** Down or Page Down does nothing, or the list does not scroll to the focused row. Also wrong: Space does not check the row, or Enter opens the wrong row. Also wrong: Escape, with no selection, does not give its present Back action.

### V4: Download, Add to playlist, and Clear update the list in place

18. In Releases, select one release that is not fully downloaded. Look for a "Download 1 release" button in the action bar.
19. Click "Download 1 release". Look for the download starting, with no navigation away from the Releases list.
20. Open the "Add to playlist" popover of the action bar and choose a playlist. Look for the tracks of the selection appending to that playlist, with the Releases list staying in place and the selection staying checked.
21. Click "Clear". Look for the selection clearing with no other change.

**Wrong if:** the list column changes to a different page during an action, or the selection clears after Download or Add to playlist.

### V5: Remove opens a confirmation, and removes each item

22. In Tracks, select two or three tracks that you can download again later. Note their titles.
23. Click "Remove n tracks…" in the action bar. Look for one dialog that lists each track's title, and names each playlist that refers to a track, when one does.
24. Click "Cancel". Look for no track removed and the selection unchanged.
25. Click "Remove n tracks…" again, then click "Remove". Look for each track leaving the Tracks list, and the list updating in place with no navigation.

**Cleanup:** download the removed tracks again. Open the release page of each removed track (through Releases or a search), and use its existing download action.

### V6: A singles group row's checkbox selects every single

26. In Releases, find a singles group row, for example "Artist name · n singles". Click its checkbox. Look for the action bar showing "n releases selected".
27. Click the chevron to open the group. Look for each single of the group, with no checkbox of its own on an individual single row.
28. Click the group's checkbox again. Look for "n releases selected" going back to no selection.

**Wrong if:** the group's checkbox selects only the group row's own key, and not every single of the group.

### V7: Widths, themes, and type sizes

29. Resize the window to a narrow width, then back to normal width. Look for the checkbox, the row text and the action bar staying in their place, with no clipped text.
30. Switch between Light and Dark themes, in Settings. Look for the checked state staying visible and clear in both themes.
31. Raise the system or app type size to a larger setting, if available. Look for the checkbox and the row text still fitting, with no overlap.
32. Confirm the checked state reads from the check mark and the fill, not from color alone. Try this with a color-blindness simulation, if one is available.

**Wrong if:** a column narrower or wider than normal clips the checkbox or the row text. Also wrong: the checked state is visible only through its color.

Do not delete `/tmp/v4vmm-governance.ie6k8TQf`. Color alone is not a valid difference.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0084-task-009-list-selection-and-action-bar.md`
- ADR 0084 Decisions 6, 9, 10 and 11, and the archived ADR 0067 for the platform modifier
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the selection model, the checkbox, the clicks and the keys, the scroll, the action bar and the removal confirmation.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- The view model gives each label, count, availability and checked state. The screen only composes, and keeps only the pointer hover state. Use named tokens and named icons.
- Tests use recorded values. No test sends a request or deletes a file.
- Never run `git checkout`, `git restore`, `git stash`, `git reset` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The detail column lists named in "Exclusions".
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R9-01 to R9-10 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 to V7.

Test commands:
- The commands in "Checks", with `cargo fmt -- --check` after the last edit and `cargo build --bin v4vmm` last.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- GPUI gives no way to read the platform modifier of a click.
- The present removal path cannot remove one item of many without a change to its plan type.
- A held scroll handle breaks the r14 guards in a way that their rule does not permit.
- A change needs a file in "Do not touch".

## Orchestrator Review - 2026-10-18

Pass, after one fix. The orchestrator made the fix in the same change.

- The packet asks that a removal report names each failed item and its error. The implementation showed only the last error, because each failure replaced the status text of the one before.
  `SelectionRemovalReport` in `src/view_models/library_removal.rs` now records each result. When the last item returns, the status reads, for example, "The app removed 1 of 3 tracks. It could not remove: Face Pale (file is locked), Motive to Kill (database is busy)." A batch with no failure stays silent, as a single removal does. The test `adr_0084_selection_removal_report_names_each_failed_item` guards it.
  The orchestrator deleted `take_pending_targets`, because `take_pending_items` replaced it.
- `primary_modifier_held` in `src/app/keyboard.rs` uses the same platform test as the key registry (`cfg!(target_os = "macos")`), in the same module. The click and the key binding thus agree, as the archived ADR 0067 requires.
- The other deviations agree with the packet or with the data. A Tracks row is always a downloaded Library track, so "Download" does not show on the track lists. An expanded single has no checkbox of its own, and the group checkbox selects each single (R9-04).
- `cargo fmt -- --check`, `cargo clippy -- -D warnings` and `cargo check --all-targets` are Green. `cargo test` gives 1962 library tests and 323 guards, all passing.
- Not part of this packet: `cargo clippy --all-targets -- -D warnings` reports errors in files that this packet does not touch, for example `src/startup.rs`. The project gate of `AGENTS.md` is `cargo clippy -- -D warnings`.

