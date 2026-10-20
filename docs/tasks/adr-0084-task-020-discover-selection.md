# ADR 0084 Task 020: Selection On Discover

Status: Ready - 2026-10-20.

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
