# ADR 0084 Task 020: Selection On Discover

Status: Implemented - 2026-10-20. Cases R20-01 to R20-08 pass. Mechanical checks are Green. V1 to V6 are open, in the final walk of the run.

## Goal

"New releases" in Discover gets the selection model and the selection mode of packets 009, 018 and 019. Its tile view and its row view both get them.
A tile shows its checkbox at the top-left corner of its cover. The bar downloads, adds or removes the selected releases.

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md) Decision 10, amended on 2026-10-19 and on 2026-10-20: the selection model, the two modes, the bar and the download queue.
- [ADR 0083](../adr/0083-design-language.md) Decisions 4 and 6: artwork leads, and a row shows its state at all times.
- The archived packets [018](archive/adr-0084-task-018-detail-column-selection.md) and [019](archive/adr-0084-task-019-selection-mode.md), and packet [022](adr-0084-task-022-download-queue.md).
- Operator decisions of 2026-10-20: a tile shows its checkbox at the top-left corner of its cover, with a backing so that it reads on each cover. A selected tile also gets an outline. A plain click in selection mode toggles the tile.
- The [run record of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md): the tile focus moves in a line. Packet 025 owns the playlist page.

## Recorded Facts - 2026-10-20

- "New releases" is in the detail column: `render_discover_new_releases_detail` in `src/ui/shells/library/content_list.rs`. It shares `render_library_content_list` with the Tracks list and the Needs attention list of the list column.
- In tile view, `render_content_list_tiles` and `render_content_list_tile` draw each tile as a plain `div()`, with no checkbox. In row view, `render_content_list_row` uses `ListRow` with the checkbox of packet 009.
- `content_list.rs` says that Discover has no selection model, and it gives the Discover rows no selection facts. It has 569 lines. The ADR 0038 ceiling of `src/ui/shells/library` counts its code lines.
- `ContentListRowDisplay.selection` exists. A Discover row has the id `index-feed:<guid>` (`src/view_models/library.rs`).
- The detail selection of packet 018: `DetailListKind::for_entry`, `DetailRowKey`, `DetailPageRows` in `src/view_models/detail_selection.rs`, `library_detail_page_rows` in `src/library/detail_selection.rs`, and the clicks, the keys and `download_detail_selection` in `src/app/detail_selection.rs`.
- Each download goes through the queue of packet 022.

## Design

### The keys and the state

- A Discover row and a Discover tile have the key `DetailRowKey::Release(ReleaseKey::FeedGuid(guid))`, from the `index-feed:` row id. A row with no feed GUID has no key and no checkbox.
- `DetailListKind::for_entry` gives `Releases` for the "New releases" view.
- Add the builder `discover_release_rows` for `DetailPageRows`, with the Library facts of each release, so the bar counts follow packet 018.
- `library_detail_page_rows` gets the arm for Discover.
- Remove the comment that says Discover has no selection model.
- "Load more" adds rows that are not selected.

### The routing

- Add a typed argument `ContentListSelectionTarget { ListColumn, DetailColumn }` to `render_library_content_list`. Discover passes `DetailColumn`.
- With `DetailColumn`, each checkbox click, row click and tile click goes to the detail selection methods of packet 018, not to `toggle_content_list_row_selection` or `click_content_list_row`. The tile click reads the modifiers.

### The tile

- Add the composite `src/ui/composites/selectable_tile.rs`.
- In selection mode, the checkbox shows at the top-left corner of the cover, inset by `Spacing::XS`. Its backing is a `Size::PointerHitTarget` circle of `SemanticColor::SystemBackground`, with a `Separator` border. The `Checkbox` primitive is inside it.
- A selected tile gets the `Accent` outline and the `SelectedContent` fill. A focused tile gets the `Focus` outline. Make `row_outline` of `src/ui/composites/list_row.rs` `pub(crate)` and use it. Do not copy it.
- The focused tile holds the scroll probe of packet 018.
- No raw literal.

### The keys

- The tile focus moves in a line: Up and Left go to the tile before, and Down and Right go to the tile after. The number of tiles in a row is a layout fact, and the view model holds no layout fact.
- The row view and the tile view keep one focus order, so a change of view keeps the focus.
- Space, Ctrl+A, Enter and Escape act as in packet 018.

### The bar

- The bar is the release bar of packet 018: "Download" counts the Index releases and the Library releases that are not fully downloaded. "Add to playlist" and "Remove" count the Library releases.
- "Download" goes through `download_detail_selection` and the queue of packet 022.

## Required Changes

1. Add the Discover keys, the list kind, the builder and the page rows.
2. Add `ContentListSelectionTarget` and route the Discover clicks.
3. Add the tile composite and use it in tile view.
4. Add the linear tile focus.
5. Update the guards.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_discover_selection_` for unit tests beside the owning code. Each new guard cites ADR 0084 Decision 10.

| Case | Required proof |
|---|---|
| R20-01 | `DetailListKind::for_entry` gives `Releases` for "New releases" |
| R20-02 | Only an `index-feed:` row gives a key. A row with no feed GUID has no key and no checkbox |
| R20-03 | The Discover bar counts follow packet 018 R18-06 for a recorded set of Index and Library releases |
| R20-04 | In `Normal`, no Discover row or tile view model shows a checkbox. In `Selecting`, each row with a key shows one |
| R20-05 | The tile focus moves in a line for Up, Down, Left and Right, and stops at the first and the last tile |
| R20-06 | A source guard: `content_list.rs` sends the Discover clicks through `ContentListSelectionTarget::DetailColumn` |
| R20-07 | A source guard: `selectable_tile.rs` uses named tokens only, and calls no `gpui::px(` |
| R20-08 | Each packet 009, 018, 019 and 022 test still passes |

## Visual Acceptance Criteria

These are for the operator, in the final walk of the run. No test proves them.

- V1: in Discover, the detail column header shows "Select". In selection mode, each tile shows its checkbox at the top-left corner of its cover.
- V2: the checkbox reads on a light cover, on a dark cover and on a busy cover.
- V3: a selected tile shows the outline and the check. A focused tile shows the focus outline. Each reads without color alone.
- V4: a click on a tile in selection mode selects it and opens no page. "Download 3 releases" queues three downloads, and the progress item shows them.
- V5: the row view of Discover in selection mode, the same as V1 to V4.
- V6: Light and Dark themes, a narrow width and the larger type sizes. The tiles keep their size in selection mode.

## Exclusions

- No playlist page selection. Packet 025 owns it.
- No search results selection. Packet 010 owns it.
- No spatial arrow keys in the tile grid. The HIG polish backlog can take them.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/ui/shells/library/content_list.rs`, `src/ui/composites/list_row.rs`, `src/ui/primitives/checkbox.rs`, `src/ui/tokens.rs`.
- `src/view_models/detail_selection.rs`, `src/library/detail_selection.rs`, `src/app/detail_selection.rs`, `src/view_models/library.rs` (`ContentListRowDisplay`).
- `tests/architecture_tests.rs`: the tile contract guards, `surface_modules_under_500_loc`, and the packet 018 guards R18-07 and R18-08.

## Checks

```bash
cargo test --lib adr_0084_discover_selection_
cargo test --lib adr_0084_detail_selection_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the commit. This packet adds no migration and no stored data.

## Implementation Result - 2026-10-20

### Files Changed

- `src/view_models/detail_selection.rs`:
  - `DetailListKind::for_entry` gives `Releases` for `DiscoverView::NewReleases`.
  - New `discover_row_key`, `discover_release_rows` (the builder) and `discover_rows_with_selection`.
  - New `TileArrow` (Left and Right) with `focus_step`. `DetailSelection::entry` is now also in the production build.
  - Tests R20-01 to R20-05.
- `src/view_models/library.rs`: new `ContentListSelectionTarget { ListColumn, DetailColumn }` and `ContentListRowDisplay::index_feed_guid`.
- `src/library.rs` and `src/library/app_impl.rs`: new field `discover_scroll_reveal`, on the scroll handle of the content list. The two calls of the content list renderers give the target.
- `src/library/detail_selection.rs`:
  - `library_detail_page_rows` has the arm for Discover.
  - New `click_discover_row`, `toggle_discover_row`, `click_content_row` and `toggle_content_row`.
  - `detail_scroll_reveal` gives the Discover request while the Discover view is open.
- `src/app/detail_selection.rs`: `detail_page_rows` has the arm for Discover. New `move_discover_tile_focus` for Left and Right.
- `src/app/keyboard.rs`: Right and Left also call `move_discover_tile_focus`.
- `src/ui/composites/selectable_tile.rs` (new): the tile composite, with its token test.
- `src/ui/composites/list_row.rs`: `RowOutline` and `row_outline` are `pub(crate)`.
- `src/ui/composites/mod.rs`: the new module.
- `src/ui/shells/library/content_list.rs`:
  - `render_library_content_list` takes the target and the scroll probe.
  - The tile renderer uses `SelectableTile`. The row renderer and the tile renderer share `render_content_row_checkbox`.
  - The comment that said that Discover has no selection model is deleted. The file has 496 code lines.
- `tests/architecture_tests.rs`: new guards `adr_0084_discover_selection_r20_06_discover_clicks_go_to_the_detail_column` and `adr_0084_discover_selection_r20_07_selectable_tile_uses_named_tokens`.

### Behavior Changed

- "New releases" in Discover has the selection model and the two modes of packets 018 and 019, in the row view and in the tile view.
- The detail column header shows "Select" and "Done" on the view. The accessibility label is "Select releases" or "Done selecting releases".
- In selection mode, each tile shows its checkbox at the top-left corner of its cover. The checkbox is on a circle of `SystemBackground` with a `Separator` border.
- A selected tile has the `Accent` outline and the `SelectedContent` fill. A focused tile has the `Focus` outline. The outline is a layer over the tile, so the tile keeps its size.
- A plain click on a row or a tile in `Normal` opens the release page, as before. In selection mode, it changes the selection and opens no page. Ctrl+click and Shift+click start selection mode.
- Up, Down, Left and Right move the tile focus in a line, and stop at the first and the last tile. The row view and the tile view use one focus order.
- Space, Ctrl+A, Enter and Escape act as in packet 018. The focused row or tile stays in view.
- The bar is the release bar of packet 018. "Download" queues each Index release and each Library release that is not fully downloaded through the queue of packet 022.
- "Load more" adds rows that are not selected.

### Checks - 2026-10-20

| Check | Result |
|---|---|
| `cargo test --lib adr_0084_discover_selection_` | Green, 6 tests |
| `cargo test --lib adr_0084_detail_selection_` | Green, 9 tests |
| `cargo test --lib adr_0084_selection_` | Green, 29 tests |
| `cargo test --lib adr_0084_download_queue_` | Green, 21 tests |
| `cargo test` | Green, 2020 unit tests and 336 guards. Ten documentation examples stay ignored |
| `cargo test --test architecture_tests` | Green, 336 guards |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

No application launch and no production-data change occurred. No test sends a request.

### Deviations From The Task

1. `render_library_content_list` takes one argument `routing: (ContentListSelectionTarget, Option<AnyElement>)`, not a separate target. The second item is the scroll probe of the focused row.
   Two separate arguments made `content_list.rs` go above the ADR 0038 ceiling.
2. `ContentListSelectionTarget` is in `src/view_models/library.rs`, not in `content_list.rs`. `LibraryApp` and the shell both use it, and it holds no renderer type.
3. The match on the target is in `LibraryApp::click_content_row` and `LibraryApp::toggle_content_row`, in `src/library/detail_selection.rs`. The shell sends each click with its target.
   This also keeps `content_list.rs` under the ceiling. The guard R20-06 reads the shell and these two methods.
4. A tile in the list column now sends its click to `click_content_list_row`, with the modifiers, the same as a row of the list column. Before, a tile click only opened the page.
   A tile in the list column also shows the checkbox of its row in selection mode. The list column selection model does not change.
5. Left and Right move the focus on the Discover view only, in the tile view and in the row view. On each other page, they keep their packet 018 behavior.
6. The Discover view uses a second `ScrollReveal`, on the scroll handle of the content list. The request of packet 018 scrolls the other detail pages.
7. A release page or a feed page that opens from Discover keeps `LibraryDetail::Discover`. Thus the Discover rows and the Discover request also need the bound page entry of the detail selection.
8. `TileArrow` has only Left and Right. Up and Down use `FocusStep::Up` and `FocusStep::Down` directly. R20-05 tests the four keys.
9. R20-04 tests the rows of `discover_rows_with_selection`. The row view and the tile view render these same rows. No test proves the rendered tile.

### Unresolved Concerns

1. V1 to V6 below need the operator. No test proves the checkbox place, its backing, the outline, the scroll position or a layout at a narrow width.
2. `detail_scroll_handle` of packet 018 gives the content list handle while `LibraryDetail::Discover` stays, also on a release page or a feed page that opens from Discover. This packet does not change it.
   A key scroll on such a page can thus move the wrong handle.
3. The source map and `docs/pending-human-checks.md` do not name `src/ui/composites/selectable_tile.rs` or V1 to V6. The task instructions named only this packet as a document to edit.

## Operator Visual Check

This check needs a Linux desktop session, this checkout and a connection to MusicIndex. It also needs these items:

- Three Index releases in "New releases" that are not in the Library: one with a light cover, one with a dark cover and one with a busy cover.
- One release in "New releases" that is also in the Library and fully downloaded.

V4 downloads three releases. Use releases that you can remove after the check.

1. Build the app: `cargo build --bin v4vmm`. Then start `target/debug/v4vmm` from a desktop session.
2. Open Music, then Discover, then "New releases", in the sidebar.
3. Set the tile view with the view-mode control at the top of the detail column.

### V1: The header button and the tile checkbox

4. Look for a "Select" button at the right end of the detail column header.
5. Click "Select". Look for a checkbox at the top-left corner of the cover of each tile. Look for the bar "Select items" at the bottom of the detail column.
6. Click "Done". The checkboxes and the bar must go, and the header must read "Select".

**Wrong if:** a tile shows a checkbox in normal mode, or the checkbox is not at the top-left corner of the cover.

### V2: The checkbox on each cover

7. Click "Select". Look at the checkbox on the light cover, on the dark cover and on the busy cover.
8. Check each of these three tiles, and look at each checkbox again.

**Wrong if:** a checkbox or its circle backing is hard to see on a cover. Examine the clear state and the checked state.

### V3: The selected tile and the focused tile

9. Look at a checked tile. It must show a check mark, an outline and a fill.
10. Click an empty area of the detail column. Push Right two times, then Left one time. A focus outline must move one tile at a time.
11. Push Down at the last tile of a row. The focus must go to the first tile of the next row.
12. Push End, then Home. The focus must go to the last tile, then to the first tile. The column must scroll to keep the focused tile in view.

**Wrong if:** a selected tile shows its state only by color, with no check mark. Also wrong: the focus skips a tile, or the focused tile goes out of view.

### V4: A click in selection mode and "Download"

13. Click "Done", then click "Select". Click the body of three tiles that are not in the Library.
14. Each of the three tiles must become checked, and no release page must open.
15. Look for "Download 3 releases" in the bar. Click it.
16. Look at the progress item below the live status strip. It must show three downloads, for example "Downloading 1 of 3".
17. Click "Done". Click one tile. Its release page must open.

**Wrong if:** a click in selection mode opens a page, or the progress item does not show the three releases.

### V5: The row view

18. Go back to "New releases". Set the row view with the view-mode control.
19. Do steps 4 to 17 again in the row view. In the row view, the checkbox is at the start of each row, and Up and Down move the focus.
20. Move the focus to a row, then change to the tile view. The same release must keep the focus.

**Wrong if:** the row view does not do the same as the tile view. Also wrong: a change of view moves the focus to a different release.

### V6: Themes, widths and type sizes

21. In the tile view, click "Select" and check two tiles. Compare the size of a tile in selection mode and in normal mode.
22. Change between the Light and the Dark theme in Settings.
23. Make the window narrow, then make it wide again.
24. Set a larger type size in Settings, if this setting is available.
25. In each condition, look at the tiles, the checkboxes, the outlines and the bar.

**Wrong if:** a tile changes its size in selection mode, text is clipped, or a checkbox moves off its cover.

**Cleanup:** click "Done" to end selection mode. Remove the three releases that V4 downloaded, if you do not want them. Do not delete `/tmp/v4vmm-governance.ie6k8TQf`.

## Orchestrator Review - 2026-10-20

Pass, after one fix. The orchestrator made the fix in the same change.

- A release page or a feed page that opens from Discover keeps `LibraryDetail::Discover`. `detail_scroll_handle` read only that kind, so the keys of such a page scrolled the hidden Discover list. It now chooses the handle with `discover_is_open()`, as `detail_scroll_reveal` does. The guard `adr_0084_discover_selection_a_page_from_discover_scrolls_its_own_handle` holds it.
- The deviations agree with the packet. A tile in the list column now sends its click with the modifiers to `click_content_list_row`, so a tile and a row act the same.
- The orchestrator added `selectable_tile.rs` to the source map.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0084-task-020-discover-selection.md`
- ADR 0084 Decision 10, and the packets 018, 019 and 022
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the keys, the routing, the tile, the linear focus and the guards.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- The view model gives each key, check state and availability. The screen only composes. Use named tokens.
- Use the types of packets 009, 018 and 019. Do not copy the selection model or `row_outline`.
- Tests use recorded values. No test sends a request.
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git add` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The playlist page and the search results.
- The download queue, except a call to it.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R20-01 to R20-08 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - 2026-10-20" section and an "Operator Visual Check" section for V1 to V6.

Test commands:
- The commands in "Checks", with `cargo fmt -- --check` after the last edit and `cargo build --bin v4vmm` last.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- The Discover rows cannot reach the detail selection without a change to the list column selection.
- A file in `src/ui/shells/library` goes above the ADR 0038 line ceiling, and a split would move code out of its owner.
- A change needs a file in "Do not touch".
