# ADR 0084 Task 036: A Singles Group Row At Its Artist Name In "Everything"

Status: Ready - 2026-10-24. The operator answered Q1 on 2026-10-24 (option A). Packet 037 starts after this packet.

## Goal

In the Releases list with the scope "Everything", a singles group row takes the place of its label, the artist name. This is the place that the row has in the scope "Library". Today the row in "Everything" takes the place of its first single in title order.

The packet also states how this rule agrees with packet 016 decision D7: a row does not move after it shows. Question Q1 holds that choice for the operator.

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md) Decision 6: the singles groups. Decision 8: the two scopes.
- [Packet 026](adr-0084-task-026-index-singles-groups.md) decision D5 and its "Operator Answers - 2026-10-24": option B. A group row in "Everything" takes the place of its artist name. A fix packet decides how this agrees with packet 016 decision D7.
- The [run record](../reviews/orchestrated-run-2026-10-23.md#operator-answers---2026-10-24) of 2026-10-23, "Operator Answers - 2026-10-24", answer 1.
- [Packet 016](adr-0084-task-016-library-and-everything-scopes.md) decisions D6 (the order) and D7 (the paging and the watermark).
- Packet 026 decisions D4 (when the app reads the singles of a feed), D10 (the rows of a page show after each of its requests has an answer) and D14 (a single page reads the singles of its feed).
- [Packet 032](adr-0084-task-032-singles-group-rows.md) decision D3: the matched singles do not move the row.
- [AGENTS.md](../../AGENTS.md): "Every fix gets a guard", "Delete dead code" and "Current-view state must update in place".

## Recorded Facts - 2026-10-24

### Code facts

The code facts come from the working tree of 2026-10-24, after commit `e476192`. Find each call site again before you change it.

- `src/view_models/library.rs:3664-3669`, in `LibraryViewModel::releases_list`: the order of a group row is `ReleaseOrderKey::group(&row.label, group.key())` in "Library". In "Everything", it is `ReleaseOrderKey::group_at(group.first_title_sort_key(), group.key())`.
- `src/view_models/releases_everything.rs:63-80`: `ReleaseOrderKey::group` calls `group_at` with `name_sort_key(label)`. `group_at` has no other production caller.
- `src/view_models/singles_group.rs:225-234`: `SinglesGroup::first_title_sort_key` gives the lowest sort key of `position_members`. `releases_list` is its only production caller.
- `src/view_models/singles_group.rs:240`: `position_members` also gives the art of the group row (line 303). It stays.
- `src/view_models/index_name_list.rs:126-128`: `watermark_shows` shows an entry when `has_more` is false. It also shows an entry whose order key is at or before the key of the last loaded `MusicIndex` row.
- `src/view_models/releases_everything.rs:215-255`: `everything_entries` applies `watermark_shows` to each entry, also to each group row (lines 249-254).
- The label of a group row is the row label of packet 026 decision D9: the Library label when the Library holds a member, else the `remote_feed_title` of the newest `MusicIndex` single, else its `release_artist`.
- A group of the feed G exists in "Everything" only when `IndexFeedSingles` holds a result for G. Three paths record that result:
  1. Page 1 reads each Library artist feed with one or more Library singles (packet 026 decision D4, case R26-08).
  2. Each page load reads each new artist feed of the singles on that page. `finish_release_page` in `src/view_models/index_feed_singles.rs:257-279` records the results.
  3. An open of a group page or of a single page reads G (`request_feed_singles` in `src/library/music_scopes.rs:248-286`, packet 026 decisions D12 and D14).
- By packet 026 decision D10, the rows of a page show only after each feed request of that page has an answer. Thus a `MusicIndex` single never changes from a release row to a group member.

### Tests that name the present rule

- `adr_0084_index_singles_r26_15_group_order_and_members` (`src/view_models/library.rs:12004`). With `fruit_page(true)`, it expects `["Haleen", "Banana", "Cherry"]`: the group at "Apple", its first single.
- `adr_0084_index_singles_r26_17_open_group_gives_members_newest_first` (`src/view_models/singles_group.rs:2538`) asserts `first_title_sort_key()` at line 2604.
- `adr_0084_singles_group_rows_matched_only_group_keeps_its_place` (`src/view_models/singles_group.rs:2754`) asserts only `first_title_sort_key()`.

### What the label order does to the list

These facts come from the code above and from the counts of packet 026. Nobody observed them in the running app.

- With the label order, the watermark applies to the sort key of the label. A known group row shows when the last loaded `MusicIndex` row sorts at or after its label.
- A group with a Library member is known after page 1 (path 1). Its row waits until the pages reach its label. It then shows in the new part of the list, below the rows that show. It never shows above them.
- A group with no Library member is known only after the page that names its feed for the first time loads (path 2). Its label can sort before the last row of the earlier pages. Then the row shows above rows that show already.
- Packet 026 counted this case on 2026-10-21: 128 of 628 groups. That count includes the groups with a Library member, which are never late. Thus 128 or fewer groups are late. No count separates the two kinds. On 88 pages, 128 late groups give a mean of less than 2 for each page.
- The present code has one path that already shows a group row above rows that show: path 3. A group page open can record a feed whose singles are on pages that did not load. The group row then shows at its first single, and that title can sort before the last loaded row.
- The group row "Haleen" has Library members. With the label order, it shows only when the pages reach "haleen". The walk of V1 thus needs many "Load more" clicks. Packet 037 records about 500 ms for each frame after several clicks (overhaul plan, responsiveness item 31).

## Design

1. In `LibraryViewModel::releases_list` (`src/view_models/library.rs:3611-3688`), give each group row `ReleaseOrderKey::group(&row.label, group.key())` in both scopes. Remove the `match scope` of lines 3664-3669.
2. In `src/view_models/releases_everything.rs`, move the body of `group_at` into `ReleaseOrderKey::group` and delete `group_at`.
3. In `src/view_models/singles_group.rs`, delete `first_title_sort_key`. Keep `position_members` for the art, and change its doc comment so that it names the art only.
4. `everything_entries` already applies the watermark to each group row. With option A of Q1, it needs no change.
5. Change the doc comments that state the present rule: `releases_list` (`src/view_models/library.rs:3607-3609`) and the doc comment of `group_at` (`src/view_models/releases_everything.rs:70-72`).

## Decisions

- **D1.** The order key of a group row is the sort key of its label in both scopes. The label is the row label of packet 026 decision D9.
- **D2.** A release row comes before a group row with the same sort key. `ReleaseOrderTie` gives this order, and it does not change.
- **D3.** The scope "Library" does not change (packet 026 decision D15).
- **D4.** The members of a group, their order, the count, the art and the group page do not change.
- **D5.** Packet 032 decision D3 stays for the art only. With the label order, no member gives the place of the row.

## Questions

- **Q1. How does a group row at its label agree with packet 016 decision D7?**

  Facts: a group with a Library member never shows above rows that show, because page 1 makes it known. A group with no Library member is known only when the page of its first single in title order loads. When its label sorts before the rows that show, no hold of the watermark can put it in the new part of the list. On 2026-10-21, this applied to 128 or fewer of 628 groups.

  - Option A: the watermark holds each group row until the pages reach its label. A group row that a later page finds after the pages passed its label shows at its label at once. Packet 016 decision D7 gets one exception for this row.
    - Each row below it moves down by one row. No release row goes, and no release row changes into a group member.
    - Packet 037 then keeps the rows in view at their place. A `ListState::splice` above the first visible item keeps that item at the top (gpui-pre 0.3.1 `src/elements/list.rs:536-548`).
    - Until packet 037, the rows in view can move down by one row for each late group.
  - Option B: the watermark holds a late group row until the last page loads (`has_more` false). No row moves.
    - But the members of the group give no release row (`hidden_keys`), so its singles show nowhere in the list until the last page. On 2026-10-21, the list had 88 pages.
    - To show them as release rows in that time breaks decision D7 in a different way: shown rows go into a group later.
  - Option C: the app finds each group before the pages reach its label. This needs the artist feeds with two or more singles, in name order. Stophammer gives no count of singles for each feed, and the kind rule belongs to v4vmm (packet 026 decision D4, option D). The app would need a new Stophammer request, or one singles request for each artist feed of the `GET /v1/artist-names` pages. On 2026-10-21, the singles named 1,220 artist feeds.
  - Option D: no hold for group rows. Each known group row shows at its label at once. Then each Library group row shows before the pages reach its label. Each later page adds rows above it, so the group rows move down at each "Load more".

  **Recommendation: Option A.** The operator wants the artist name to give the place. Option A keeps decision D7 for each release row and for each group row that is known early. The exception moves a row only when a page finds a new group, less than 2 times for each page on 2026-10-21. Packet 037 removes the visible move. Path 3 already gives the same exception today.

  With option A, Design step 4 needs no code. With option B, `everything_entries` needs a second watermark rule for a group row that is known after the pages passed its label. Options C and D change the scope of this packet. The orchestrator writes a new packet for them.

## Required Changes

1. `src/view_models/library.rs`: Design steps 1 and 5.
2. `src/view_models/releases_everything.rs`: Design steps 2 and 5.
3. `src/view_models/singles_group.rs`: Design step 3.
4. Tests:
   - Change `adr_0084_index_singles_r26_15_group_order_and_members` to the label order (case R36-01). Change its doc comment.
   - Remove the `first_title_sort_key()` assertion at `src/view_models/singles_group.rs:2604`.
   - Change `adr_0084_singles_group_rows_matched_only_group_keeps_its_place` so that it asserts the art of a group of matched singles alone, which `position_members` gives. Change its name to `..._keeps_its_art`.
   - Add each new test of "Mechanical Acceptance Criteria".
5. Documents, in the same change (AGENTS.md "Changing Direction Is One Change"):
   - Packet 016 decision D7: add the exception of the answer to Q1, with a link to this packet.
   - Packet 026: the Design line "The order position of a group row is the sort key of its first single in title order" and case R26-15 name this packet and the label order.
   - [Pending human checks](../pending-human-checks.md): add V1 to V3 of this packet. Packet 026 V1 stays open until the walk of this packet passes.

## Mechanical Acceptance Criteria

Each criterion here is mechanical. A unit test beside its owning code has the prefix `adr_0084_task_036_`. No test sends a request to `api.musicindex.org`.

| Case | Required proof |
|---|---|
| R36-01 | Mechanical, unit test in `src/view_models/library.rs`. Regression test: `adr_0084_task_036_group_row_takes_its_label_place`. With `index_singles_vm()`, `haleen_fills()` and `fruit_page(true)`, `releases_list` in "Everything" gives `["Banana", "Cherry"]`. With `fruit_page(false)`, it gives `["Banana", "Cherry", "Haleen", "Zebra"]`. Its doc comment names ADR 0084 Decision 6, packet 026 decision D5 (operator answer of 2026-10-24) and packet 016 decision D7. It is situational: delete it when ADR 0084 Decision 6 is superseded |
| R36-02 | Mechanical, unit test, same file. The order key of a group row in "Everything" equals `ReleaseOrderKey::group(label, key)`, the key of the same group in "Library" |
| R36-03 | Mechanical, unit test, same file. While `has_more` is true, a known group row whose label sorts after the last loaded `MusicIndex` row gives no entry. A later page that reaches its label gives the entry |
| R36-04 | Mechanical, unit test, same file. For option A of Q1: a constructed page 1 and page 2. Page 2 records a feed G whose label sorts before the last row of page 1. The entries after page 2 hold the group row at its label. Each entry of page 1 is still present, in the same order |
| R36-05 | Mechanical, unit test, same file. The same two pages: no release entry of page 1 is a member of a group after page 2 (packet 026 decision D10) |
| R36-06 | Mechanical, unit test, same file. A result of path 3 (`finish_feed_request`) can name a feed whose pages did not load. It gives the group row at its label, with the watermark of R36-03 |
| R36-07 | Mechanical, command. `rg "group_at\(\|first_title_sort_key" src` gives no result |
| R36-08 | Mechanical. Each test of packets 015, 016, 026 and 032 passes, with the changed tests of "Required Changes" item 4. `adr_0084_singles_group_r15_09_only_the_owner_groups_singles` passes |

## Visual Acceptance Criteria

Each criterion here is visual. Only a person can examine it. No test proves it.

- V1: packet 026 V1 again, in Releases with the scope "Everything".
  - The group row "Haleen" shows at the place of its name. The row above it sorts before "Haleen", and the row below it sorts after "Haleen".
  - The row shows "n singles", the download state icon with its count, for example "19 of 56", and "Artist feed", with n about 55.
  - No Haleen single of that feed shows as its own row.
- V2: packet 026 V4, changed by the answer to Q1. "Load more" adds rows at the end. With option A, a group row can also show above, at its name. No release row goes, and no release row changes into a group member.
- V3: Releases, "Library". Each group row shows at the place of its name, as before this packet.

## Exclusions

- No change to the scope "Library" (decision D3).
- No change to the requests, the fills, the group page or the link "All n singles".
- No change to the frame time of the list. Packet 037 owns it.
- No visual change of the rows. Phase 7 of the overhaul plan owns the visual fails of the walk.

## Files To Inspect

- [AGENTS.md](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/view_models/library.rs` (`releases_list`, `fruit_page`, `index_singles_vm`, `haleen_fills`, the R26 tests).
- `src/view_models/releases_everything.rs` (`ReleaseOrderKey`, `everything_entries`).
- `src/view_models/singles_group.rs` (`SinglesGroup`, `first_title_sort_key`, `position_members`, `singles_groups_everything`).
- `src/view_models/index_name_list.rs` (`watermark_shows`).
- `src/view_models/index_feed_singles.rs` (`finish_release_page`, `finish_feed_request`).
- `src/library/music_scopes.rs` (`request_feed_singles`).

## Checks

```bash
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo check --all-targets
touch src/main.rs && cargo build --bin v4vmm
```

## Rollback

Revert the commit. This packet adds no migration, no stored data and no configuration key.

## Operator Visual Check

This check needs a Linux desktop session and a connection to `https://api.musicindex.org`. It needs the Library groups of the Haleen artist feed `64569161`. Use the release binary, because the walk needs many "Load more" clicks, and the debug build is slower.

1. Build and start the release binary:

   ```bash
   cd ~/build/v4vmm
   cargo build --release
   ./target/release/v4vmm
   ```

2. Open Music, then Releases. Select the scope "Everything".
3. Click "Load more" at the end of the list until the list shows rows whose titles start with "H". Before packet 037, each frame can be slow after several clicks. That is item 31, not a defect of this packet.
4. V1: find the group row "Haleen".
   **Wrong if:** the row is not between rows that sort before and after "Haleen", or a Haleen single of that feed shows as its own row.
5. V2: scroll to the end and click "Load more" one more time.
   **Wrong if:** a release row that showed before is gone, or changes into a group member. With option A of Q1, a new group row above, at its name, is correct.
6. V3: select the scope "Library".
   **Wrong if:** a group row is not at the place of its name.

Cleanup: this check changes no stored data. The app starts with the scope "Library" at each start (packet 016 decision D3).

## Operator Answers - 2026-10-24

- Q1: option A. A group row that a later page finds after the pages passed its label shows at its label at once. This is the one exception to packet 016 decision D7. The operator chose it from this example: the loaded pages reach "M", and page 8 finds the group of "Aaron". The "Aaron" row shows under "A" at once.
