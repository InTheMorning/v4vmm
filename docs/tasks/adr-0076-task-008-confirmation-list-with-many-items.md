# ADR 0076 Task 008: Confirmation List With Many Items

Status: Implemented - 2026-10-03. Mechanical checks Green. Its visual gate is open.

## Goal

The confirmation popup shows each item when the item list is taller than its scrolling column.

## Authority

- [ADR 0076](../adr/0076-playlist-rss-check-for-stale-musicindex-records.md) Decision 8: the "Update n files" popup lists each file with its frames.
- [ADR 0063](../adr/0063-show-dashboard-layout.md): stacked column text does not call `truncate()`.
- The durable set in [AGENTS.md](../../AGENTS.md): no isolated visual tweaks. Fix the shared composite.

## Recorded Facts - 2026-10-03

- The operator opened "Update 34 files?" on the real Library. The popup showed its title, its message, an empty area and its buttons. It showed no file.
- `confirmation_items` in `src/ui/composites/confirmation_dialog.rs` builds a column with `max_h(Size::ColumnRegular)` and `overflow_y_scroll()`.
  Each item is a flex column child with `overflow_hidden()` and no `flex_shrink_0()`.
- A flex child with hidden overflow has a minimum height of zero. When the items are taller than the column, the layout shrinks each item to zero height.
- The ADR 0076 packet 004 fixture had two or three files. They fit in the column, so the defect did not show.
- The same composite lists the items of each other confirmation, for example the ADR 0044 removal confirmations.

## Required Changes

1. Each item in `confirmation_items` keeps its own height. The column scrolls when the items are taller than the column.
2. Keep `overflow_hidden()` and no truncation on stacked text (ADR 0063).
3. Change no view model and no screen. The fix is in the shared composite.

## Mechanical Acceptance Criteria

Use the prefix `adr_0076_confirmation_list_` for tests beside the owning code.

| Case | Required proof |
|---|---|
| R76-8-01 | A GPUI layout test renders the confirmation dialog with 40 items. Each item has a height above zero, and the first item has the same height as in a dialog with one item. Use `debug_selector` and `debug_bounds`, as the test in `src/ui/composites/settings.rs` does |
| R76-8-02 | The item column has a height no larger than its `max_h` with 40 items |
| R76-8-03 | A dialog with no item shows no item column (the present behavior stays) |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: "Update n files" with more files than the column holds shows each file with its title, album and frames. The list scrolls.
- V2: a removal confirmation with one item still shows that item, at normal and narrow widths, in Light and Dark themes.

## Exclusions

- No change to the tag scan or to the tag writer. [ADR 0080 packet 003](adr-0080-task-003-vorbis-date-shares-one-key.md) owns the FLAC date defect.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- `src/ui/composites/confirmation_dialog.rs`, `src/ui/shells/tag_update_confirmation.rs`.
- `src/ui/composites/settings.rs`: the layout test that uses `debug_bounds`.
- `docs/troubleshooting/column-text-truncation.md`.

## Checks

```bash
cargo test --lib adr_0076_confirmation_list_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Implementation Result - 2026-10-03

### Files

Changed file: `src/ui/composites/confirmation_dialog.rs`. No other file changed.

- `confirmation_items`: each item's `div` now carries `flex_shrink_0()`. A doc comment states
  the reason (ADR 0076 Decision 8).
- Each item and the item column now carry a `debug_selector`. GPUI makes `debug_selector` a
  no-op outside a test build, so this adds no runtime behavior. The new tests use it.
- Its test module: two new `#[gpui::test]` layout tests,
  `adr_0076_confirmation_list_keeps_item_height_with_many_items` and
  `adr_0076_confirmation_list_with_no_item_shows_no_column`, with their fixtures
  (`sample_items`, `empty_display`, `leaked_selector`, and two test-only render types).

### Tests

- `cargo test --lib adr_0076_confirmation_list_`: 2 passed (R76-8-01, R76-8-02, and R76-8-03).
- `cargo test --lib`: 1787 passed, 0 failed.
- `cargo test`: all passed, including the 1787 library tests, 289 architecture tests, and the
  doc tests (ignored, as the crate marks them).
- `cargo test --test architecture_tests`: 289 passed, 0 failed.
- `cargo fmt -- --check`: Green, no difference.
- `cargo clippy -- -D warnings`: Green, no warning.
- `cargo check --all-targets`: Green, no warning.
- `cargo build --bin v4vmm`: Green.
- Check: with `flex_shrink_0()` removed, `adr_0076_confirmation_list_keeps_item_height_with_many_items`
  fails with "item 0 has zero height with 40 items". This confirms the test proves the fix.

### Behavior

Each item in a confirmation popup's item column keeps its own height. When the items are taller
than the column's `max_h`, the column scrolls to show every item. No item shrinks to zero height. A popup with one item, or with no item, renders the same as before this change.

### Deviations

None. The fix stays inside `confirmation_items`, and the tests stay inside its test module, as
"Required Changes" states. `adr_0076_confirmation_list_with_no_item_shows_no_column` renders the
real `confirmation_dialog` function inside a `gpui_component::Root`, the pattern
`src/ui/composites/selectable_text.rs` already uses for a test window. This proves R76-8-03
through the real "no item" branch. It does not copy the condition.

### Concerns

None found for this packet's required changes. The visual gate stays open until an operator
completes V1 and V2 below.

## Operator Visual Check

This check only opens a popup and clicks its Cancel button. It writes no tag, and it removes no
track from the library.

Needs: a Linux desktop session, and a Library with more files pending a tag update than its
popup column shows at once. The Recorded Facts above name 34 as an observed count. A fresh scan
after a feed check, a download, or a tag apply also produces one. V2 needs one track already in
the local library.

**Setup**

1. Build and open the desktop binary:

   ```bash
   cargo build --bin v4vmm && target/debug/v4vmm
   ```

   Run this command first. A prior `cargo test` run can leave a GPUI test-support binary at
   `target/debug/v4vmm`. This step is the only one that starts the app.

**V1 — "Update n files" scrolls through every file**

2. Open Music's Library. Wait for its "Update n files" button to name a count taller than one
   screen of the popup. Select the button.
3. Read the popup.
   - This result is wrong: an empty area below the message, with no file.
   - This result is wrong: the list stops partway through the named files.
   - This result is wrong: a file row with no title.
4. Scroll the list with the mouse wheel or a trackpad gesture. Confirm the first and the last
   named file both become visible, each with its title, its album, and its frames.
5. Select Cancel. No file changes.

**V2 — a one-item removal confirmation still shows its item**

6. Find a track already in the local library. Open its row actions and select "Remove from
   Library". The popup opens with exactly one item.
7. Read the item.
   - This result is wrong: the item area is empty.
   - This result is wrong: the item's title or its album is missing.
8. Select Cancel. No track is removed.

**V3 — normal and narrow widths, Light and Dark themes**

9. Repeat step 6 through step 7 at the normal window width, then at a narrow width (about 560
   pixels, as in the search toolbar check).
10. Repeat step 9 in Light theme, then in Dark theme (Settings → General).
    - This result is wrong: the one item clips its title or its album at either width or in
      either theme.
    - Color alone is not a valid difference between a correct result and an incorrect one.

**Cleanup**

11. Close the app window. Step 5 and step 8 each end in Cancel, so the library, its tracks, and
    its audio files are unchanged. No step undoes state.
    Do not delete `/tmp/v4vmm-governance.ie6k8TQf`. It belongs to a different, still-open check.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
- This packet: `docs/tasks/adr-0076-task-008-confirmation-list-with-many-items.md`
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": each confirmation item keeps its height, and the column scrolls.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Use named tokens. Do not call `truncate()` on stacked text.
- Never run `git checkout`, `git restore`, `git stash`, `git reset` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The tag scan, the tag writer and each view model.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R76-8-01 to R76-8-03 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 and V2.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- A layout test cannot measure the item height with the present GPUI test support.
- The fix needs a change outside `src/ui/composites/confirmation_dialog.rs` and its tests.
