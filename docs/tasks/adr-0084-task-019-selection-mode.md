# ADR 0084 Task 019: Selection Mode With A Select Button

Status: Implemented - 2026-10-19. Cases R19-01 to R19-07 pass. Mechanical checks are Green. The visual gate is open.

## Goal

The list column has two modes. In normal mode, rows show no checkbox and keep no space for one. A "Select" button starts selection mode, where each row shows its checkbox and a click selects.
"Done" and Escape end the mode. This replaces the checkbox visibility rule of packet 009.

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md) Decision 10, amended on 2026-10-19: normal mode, selection mode, the "Select" and "Done" button, and the action bar with "Select items".
- [Packet 009](archive/adr-0084-task-009-list-selection-and-action-bar.md): the selection model, the rows with members, the checkbox, the keys and the action bar. This packet keeps each of them.
- The operator decided the design on 2026-10-19, on the model of Apple Photos, Files and Mail. Ctrl+click, Shift+click, Ctrl+A and Space also start selection mode.
- The durable set in [AGENTS.md](../../AGENTS.md): typed action state, renderer portability and token discipline.

## Recorded Facts - 2026-10-19

- `ListRow` in `src/ui/composites/list_row.rs` has a checkbox slot and `checkbox_force_visible`. A checkbox that is not forced is invisible, keeps its space, and shows with `group_hover`.
- `LibraryApp` asks `shows_checkboxes()` of `releases_selection` and `track_selection` (`src/library/app_impl.rs:3569` and `:3575`). `RowSelectionVm::checkbox_visible` comes from the focus and the selection.
- The click handlers `click_release_row`, `click_singles_group_row` and the track row click open a page on a plain click.
- The list column title, for example "Releases", comes from the frame shell of the list column (`content_list_frame_title` in `src/app.rs`, `FrameShellDisplay` in `src/view_models/workspace/`).
- The action bar shows only while a selection exists (`SelectionActionBarVm`).
- The open check of 2026-10-19 in `docs/pending-human-checks.md` covers the checkbox contrast and the focus outline. Its fix stays.

## Design

### The mode

- Add `SelectionMode::{Normal, Selecting}` to the selection state of each list, beside its `ListSelection`. The two lists of the list column keep their own mode, as they keep their own selection.
- "Select" sets `Selecting`. "Done", Escape, a change of destination and "Clear" with no row left set `Normal` and clear the selection. "Clear" with rows selected clears the rows and keeps the mode.
- Ctrl+click, Shift+click, Ctrl+A and Space set `Selecting` and then act as today.
- A removal that leaves no row keeps the mode.

### The rows

- In `Normal`, `RowSelectionVm::checkbox_visible` is false, and the row keeps no space for the checkbox. Delete the `group_hover` reveal and `checkbox_force_visible` when no caller remains.
- In `Selecting`, each row of the list shows its checkbox.
- In `Selecting`, a plain click on a row calls `toggle` for that row. It opens no page. A click on a group row's chevron still opens or closes the group.
- In `Normal`, a plain click opens the page, as today.
- The keyboard focus outline shows in both modes.

### The button

- `LibraryViewModel` gives a typed header action for the list column of Releases, Tracks and Needs attention: label "Select" in `Normal`, "Done" in `Selecting`, an accessibility label ("Select releases", "Done selecting releases") and its availability. The action is absent when the list has no row.
- The frame shell of the list column shows the action at the trailing end of its header, through a typed slot. The shell decides no label.
- Artists, Discover and Playlists show no button.

### The action bar

- In `Selecting`, the bar shows always. With no selected row, its text is "Select items" and it shows no action.
- With selected rows, the bar is as in packet 009.

## Required Changes

1. Add `SelectionMode` and its transitions, with tests.
2. Change `RowSelectionVm` and the row renderers for the two modes. Delete the hover reveal.
3. Change the row click handlers for `Selecting`.
4. Add the header action, its typed slot in the frame shell, and its click.
5. Change `SelectionActionBarVm` for "Select items".
6. Update each guard and test that names the hover reveal.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_selection_mode_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R19-01 | "Select" gives `Selecting`. "Done", Escape and a change of destination give `Normal` and clear the selection |
| R19-02 | Ctrl+click, Shift+click, Ctrl+A and Space give `Selecting` |
| R19-03 | In `Normal`, no row view model shows a checkbox. In `Selecting`, each row view model shows one |
| R19-04 | In `Selecting`, a plain click toggles the row and opens no page. In `Normal`, it opens the page |
| R19-05 | The header action reads "Select" in `Normal` and "Done" in `Selecting`, with an accessibility label. It is absent for an empty list and for Artists |
| R19-06 | In `Selecting` with no selected row, the bar gives "Select items" and no action |
| R19-07 | "Clear" with rows selected clears them and keeps `Selecting` |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: in Releases, rows show no checkbox and no empty space. The header shows "Select" at its right.
- V2: "Select" shows a checkbox on each row and the bar "Select items". A click on a row selects it and opens no page. The header reads "Done".
- V3: "Done" hides the checkboxes, clears the selection and hides the bar. Escape does the same.
- V4: in normal mode, Ctrl+click on a row starts selection mode with that row selected.
- V5: the same in Tracks and in Needs attention. Artists shows no "Select".
- V6: check normal and narrow widths, Light and Dark themes, and the larger type sizes. The button, the checkboxes and the bar stay in place, with no clipped text.

## Exclusions

- No selection mode in the detail column lists or the Discover tiles. Packet 018 owns them, with this model.
- No change to the selection model, the members, the keys or the bar actions of packet 009.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/view_models/list_selection.rs`, `src/view_models/selection_action_bar.rs`, `src/view_models/library.rs`.
- `src/library/app_impl.rs`, `src/library.rs`.
- `src/ui/composites/list_row.rs`, `src/ui/shells/library/destination_lists.rs`, `src/ui/shells/library/content_list.rs`.
- `src/app.rs` (`content_list_frame_title`), `src/view_models/workspace/` (`FrameShellDisplay`), the frame shell renderer.
- `tests/architecture_tests.rs`: the selection guards of packet 009.

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

Revert the commit. This packet adds no migration and no stored data.

## Implementation Result - 2026-10-19

### Files Changed

- `src/view_models/list_selection.rs`: new `SelectionMode` enum (`Normal`, `Selecting`) and its `is_selecting` method.
  `RowSelectionVm::for_row` and `for_group_row` take `selecting: bool` in place of `shows_checkboxes: bool`, and set `checkbox_visible` from it alone. Deleted the dead `ListSelection::shows_checkboxes`.
  Amended the R9-05 test for the new rule, and added `adr_0084_selection_mode_r19_03_checkbox_visible_follows_the_mode`.
- `src/view_models/selection_action_bar.rs`: `SelectionActionBarVm::new` takes a leading `SelectionMode`. It gives `None` outside `Selecting`, and gives "Select items" with no action when `Selecting` holds no selected row.
  Updated each R9-06 and R9-07 test's call shape, and added `adr_0084_selection_mode_r19_01_no_bar_outside_selecting_mode` and `adr_0084_selection_mode_r19_06_select_items_with_no_selected_row`.
- `src/view_models/library.rs`: new free function `list_column_header_action(destination, mode, list_has_rows)`, with its own test `adr_0084_selection_mode_r19_05_header_action_reads_select_and_done`.
  `releases_list_with_selection`, `content_list_rows_with_selection`, `releases_selection_action_bar` and `content_list_selection_action_bar` each take the list's `SelectionMode`.
- `src/view_models/workspace/chrome.rs` and `mod.rs`: new `FrameHeaderActionDisplay` (an id, a label, and an accessibility label), a `header_action` field and `with_header_action` builder on `FrameShellDisplay`, and `header_action.is_some()` added to `header_visible`.
- `src/ui/composites/frame_shell.rs`: `FrameShellSlots` gains `on_header_action`. `render_chrome` renders the header action at the trailing end of the header, through the new `header_action_button` function. The shell supplies no label of its own.
- `src/ui/shells/workspace.rs`: `WorkspaceSlots` gains `content_list_header_action` and `on_content_list_header_action`, wired to the `ContentList` frame only, the same pattern as `content_list_view_mode_control`.
- `src/app.rs`: new `TopApp::toggle_list_column_selection_mode`, and the wiring that reads `LibraryApp::header_action_for_list_column` and supplies it, and its click handler, to the Music content-list frame.
- `src/library.rs`: new `LibraryApp` fields `releases_selection_mode` and `track_selection_mode`.
- `src/library/app_impl.rs`: `click_release_row`, `click_singles_group_row` and `click_content_list_row` start `Selecting` on Ctrl+click and Shift+click. A plain click toggles the row only in `Selecting`, and opens the page only in `Normal`.
  `toggle_release_row_selection`, `toggle_release_group_selection` and `toggle_content_list_row_selection` (the checkbox clicks), `toggle_focused_selection` (Space) and `select_all_visible` (Ctrl+A) each start `Selecting`.
  `clear_focused_selection` (Escape) now gates on `is_selecting()`, and sets `Normal` when it clears. `show_destination_list` resets both modes to `Normal`.
  New `header_action_for_list_column` and `toggle_list_column_selection_mode` (the "Select" and "Done" button). `releases_selection_action_bar` and `content_list_selection_action_bar` pass their list's mode through. `render_list_column` passes each list's mode to `releases_list_with_selection` and `content_list_rows_with_selection`.
- `src/ui/composites/list_row.rs`: deleted `checkbox_force_visible` and the `group_hover` reveal. `ListRow::checkbox` takes one argument, the checkbox element, and the row renders it only when the caller gives one.
- `src/ui/composites/selection_action_bar.rs`: updated its module comment for the new showing rule. No code change: the composite already renders an empty `actions` list with only the count text.
- `src/ui/shells/library/destination_lists.rs` and `src/ui/shells/library/content_list.rs`: each checkbox call now reads `if selection.checkbox_visible { ... }`, and no longer passes a second argument to `ListRow::checkbox`.
- `tests/architecture_tests.rs`: four new guards, `adr_0084_selection_mode_r19_01_select_and_done_set_the_mode`, `r19_02_modifiers_and_space_start_selecting`, `r19_04_plain_click_follows_the_mode` and `r19_07_clear_keeps_selecting_mode`. Each reads the real source of `src/library/app_impl.rs` and checks for the required statements, the same method as the R9-08 and R9-09 guards.

### Behavior Changed

- The Releases, Tracks and Needs attention lists each have a `Normal` mode and a `Selecting` mode, held beside each list's own selection.
- In `Normal`, no row shows a checkbox and no row keeps space for one. A plain click opens the row's page.
- In `Selecting`, each row shows its checkbox. A plain click toggles the row and opens no page. A click on a group row's chevron still opens or closes the group.
- The list column header shows "Select" in `Normal` and "Done" in `Selecting`, with its own accessibility label ("Select releases", "Done selecting releases", and the same pattern for "tracks"). It is absent for Artists, Discover, Playlists, and an empty list.
- "Select" starts `Selecting` with no change to the selection. "Done" ends it and clears the selection.
- Ctrl+click, Shift+click, Ctrl+A, Space and the row's own checkbox each start `Selecting`, the same as before this packet, plus the mode change.
- Escape ends `Selecting` and clears the selection. Outside `Selecting`, Escape keeps its present Back action.
- A change of destination ends `Selecting` for both lists, the same change that already clears their selection.
- The action bar shows whenever its list is in `Selecting`. With no selected row, it reads "Select items" and shows no action. With one or more selected rows, it is unchanged from packet 009.
- "Clear" still only clears the selected rows. It does not end `Selecting`.
- A removal that leaves no row does not end `Selecting` either: no removal code touches the mode.

### Checks - 2026-10-19

| Check | Result |
|---|---|
| `cargo test --lib adr_0084_selection_` | Green, 27 tests |
| `cargo test` | Green, 1976 unit tests and 327 guards. Ten documentation examples stay ignored |
| `cargo test --test architecture_tests` | Green, 327 guards |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

No application launch and no production-data change occurred.

### Deviations From The Task

1. R9-05 of the archived packet 009 stated the checkbox visibility rule. This packet's Goal names that rule as replaced. A row's own keyboard focus no longer shows its checkbox by itself. The test keeps its name and its call shape. Its assertions now match the new rule, with a new amendment note in its doc comment. This matches the packet's own Goal, not an unplanned conflict.
2. R19-01, R19-02, R19-04 and R19-07 name facts about `LibraryApp` methods. Each method needs a `Context` and a `Window`. The R9-08 and R9-09 guards set the precedent for this: a `tests/architecture_tests.rs` guard reads the real source of `src/library/app_impl.rs` for each fact. This project keeps no GPUI test harness for `LibraryApp`, so no unit test proves these facts instead.
3. The Design section names a fourth Normal-setting trigger: "'Clear' with no row left". This packet does not add that trigger. R19-01 names only three triggers: "Done", Escape and a change of destination. R19-07 states that "Clear" keeps `Selecting` instead. The "Clear" action bar button already only cleared the selection before this packet, with no mode field to touch. This packet changes nothing in its handler: the R19-07 guard confirms this by checking that the handler never writes `SelectionMode::Normal`.
4. "A removal that leaves no row keeps the mode" needed no code change: no removal method in this codebase ever read or wrote a selection mode field before or after this packet.
5. The checkbox's own accessibility label (for example "Select Face Pale") still reads the same in both modes: only `checkbox_visible` depends on the mode. The label itself was never part of the superseded rule.

### Unresolved Concerns

1. V1 to V6 below need the operator. No test proves a rendered checkbox, a rendered button label at its actual screen position, or a layout at a narrow width.
2. [Pending human check 5](../pending-human-checks.md#5-checkbox-and-keyboard-focus-contrast---adr-0084-decision-10) names the checkbox border and the keyboard focus outline contrast. It says to walk that check with this packet. This implementer changed no color, no border and no outline logic. The task instructions for this packet named only this one document to edit. The operator check below folds that check's two contrast points into its V1 and V3 steps. A later session should retire or merge pending check 5 against this packet's own visual gate, when an operator walks both checks.
3. The unresolved concerns of packet 009's Implementation Result and Correction Result stay open. This packet changes none of them.

## Operator Visual Check

This check needs a Linux desktop session and this checkout. It also needs a Library with one Releases singles group, six or more Library tracks, and one item in Needs attention. If Needs attention is empty, skip its step in V5. Record that gap, and continue with the rest of the check.

1. Build and start the app: `cargo build --bin v4vmm` then run the built `target/debug/v4vmm` binary from a desktop session.
2. Open Music, then Releases, in the sidebar.

### V1: Normal mode shows no checkbox

3. Look at the Releases list with no row selected. Look for no checkbox on any row, and no empty space where a checkbox could sit.
4. Look at the list column header. Look for a "Select" button at its right (trailing) end.
5. Press Down to give the list column the keyboard focus, then press Down twice more. Look for the clear focus outline on the focused row. Check that this outline is easy to see, in both Dark and Light (folds in pending check 5's focus-outline point).

**Wrong if:** a row shows a checkbox or reserves checkbox space in this mode. Also wrong: no "Select" button shows. Also wrong: the focus outline is hard to see in one theme.

### V2: "Select" starts selection mode

6. Click "Select". Look for a checkbox on every row, and the action bar reading "Select items" with no button.
7. Click a release row's body, away from its checkbox and its thumbnail. Look for that row's checkbox becoming checked, the action bar changing to "1 release selected", and no page opening.
8. Look at the checked row's own checkbox. Check that its border and its fill are easy to see against the row, in both Dark and Light (folds in pending check 5's checkbox-border point).
9. Look at the list column header. Look for it now reading "Done".

**Wrong if:** a click on a row opens its page and does not toggle it. Also wrong: the action bar does not show "Select items" before a row is checked. Also wrong: the header does not change to "Done". Also wrong: the checkbox is hard to see in one theme.

### V3: "Done" and Escape end selection mode

10. With the row of V2 still checked, click "Done". Look for every checkbox disappearing, the action bar disappearing, and the header reading "Select" again.
11. Click "Select" again, then check a row. Give the list column the keyboard focus and press Escape. Look for the same result as step 10: the checkboxes, the bar and "Select" all go back to the normal-mode state.
12. With no selection active, give the list column the keyboard focus and press Escape again. Look for the app's present Back action, not a selection-mode message.

**Wrong if:** "Done" or Escape leaves a checkbox checked or visible, or leaves the action bar showing, or leaves the header reading "Done". Also wrong: Escape with nothing selected does not fall back to Back.

### V4: Ctrl+click starts selection mode from Normal mode

13. In Normal mode, hold Ctrl (Cmd on macOS) and click a release row. Look for selection mode starting directly: every row shows a checkbox, the clicked row is checked, and the header reads "Done".
14. Click "Done" to return to Normal mode.

**Wrong if:** Ctrl+click opens the row's page, or does not start selection mode, or checks the wrong row.

### V5: Tracks and Needs attention, and no button for Artists

15. Open the Tracks destination. Repeat V1 step 4 (the "Select" button) and V2 steps 6 and 7 (click "Select", then a row). Look for the same behavior as Releases.
16. Open the Needs attention destination, if it holds a row. Repeat the same two checks.
17. Open the Artists destination. Look for no "Select" button in its header.

**Wrong if:** Tracks or Needs attention does not show the button or does not toggle a row in selection mode. Also wrong: Artists shows a "Select" button.

### V6: Widths, themes, and type sizes

18. In Releases, click "Select". Resize the window to a narrow width, then back to normal width. Look for the "Select"/"Done" button, each checkbox, and the action bar staying in their place, with no clipped text.
19. Switch between Light and Dark themes, in Settings. Look for the button, the checkboxes, and the bar staying clear and in place in both themes.
20. Raise the system or app type size to a larger setting, if available. Look for the button label and the row text still fitting, with no overlap.

**Wrong if:** a narrower or wider column clips the button, a checkbox, or the bar text. Also wrong: a theme or a larger type size hides or overlaps any of the three.

**Cleanup:** press Escape, or click "Done", to leave selection mode and clear any checked row. This check removes no file and changes no stored record.

Do not delete `/tmp/v4vmm-governance.ie6k8TQf`. Color alone is not a valid difference.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0084-task-019-selection-mode.md`
- ADR 0084 Decision 10, and the archived packet 009
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the two modes, the rows, the clicks, the header button and the bar text.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- The view model gives each label, mode and availability. The screen only composes. Use named tokens.
- Tests use recorded values.
- Never run `git checkout`, `git restore`, `git stash`, `git reset` or `git commit`. Do not stage files.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The detail column lists and the Discover tiles.
- The database schema.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R19-01 to R19-07 has a passing test, and each case of packet 009 still passes.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 to V6.

Test commands:
- The commands in "Checks", with `cargo fmt -- --check` after the last edit and `cargo build --bin v4vmm` last.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- The frame shell of the list column has no place for a trailing header action, and a new slot would change another frame.
- A packet 009 case fails in a way that its meaning, not only its call shape, must change.
- A change needs a file in "Do not touch".

## Orchestrator Review - 2026-10-19

Pass. The code needs no fix.

- `cargo fmt -- --check`, `cargo clippy -- -D warnings` and `cargo check --all-targets` are Green. `cargo test` gives 1976 library tests and 327 guards, all passing.
- `list_column_header_action` gives the label and the accessibility label. The frame shell draws a typed `Ghost` button from them, and decides no text.
- The deviations agree with the packet. R9-05 of packet 009 changes as the Goal of this packet states. "Clear" shows only with selected rows, so the design sentence about "Clear" with no row left has no case.
- R19-01, R19-02, R19-04 and R19-07 are source guards, because no test builds `LibraryApp` with a GPUI context. Packet 009 uses the same method for R9-08 and R9-09.
- The orchestrator replaced the open contrast check of 2026-10-19 with the check of this packet. V1 step 5 and V2 step 8 hold its two points.

