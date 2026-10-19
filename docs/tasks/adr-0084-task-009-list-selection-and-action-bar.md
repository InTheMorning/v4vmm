# ADR 0084 Task 009: Selection And The Action Bar In The List Column

Status: Ready - 2026-10-18. Implementation has not started.

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

## Operator Visual Check

The implementer writes this section at completion. It gives numbered steps for V1 to V7.
V5 removes files from the Library. It names test tracks that the operator can download again, and gives the steps to download them again as its cleanup.
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
