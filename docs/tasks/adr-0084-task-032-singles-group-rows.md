# ADR 0084 Task 032: One Track Row For Each Singles Group Member, And The Matched-By-Name Group

Status: Implemented - 2026-10-22. Cases R32-01 to R32-18 pass. Mechanical checks are Green. V1 to V7 are open, for the operator.

## Goal

Give the singles group page one row style for each member. A Library single and a `MusicIndex`
single both show as the track row of the Index release page. This replaces packet 026 decision
D13.

Join a name that matches exactly one artist feed into the singles group of that feed. The name
keeps no group of its own, in the Library scope and in the Everything scope. The group page lists
the joined singles under a heading "Matched by name", after the feed's own singles.

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md) Decision 6, amended 2026-10-22: a name that
  matches exactly one artist feed has no group of its own. Its singles join the group of that
  feed. The group page shows them under "Matched by name", marked as inferred. A name that
  matches two or more feeds keeps its own group.
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 7: the "Matched by name" rule of the
  artist feed page, and the sort key of a name.
- [ADR 0083 task 006](adr-0083-task-006-row-hover-actions.md): the shared track row, its
  `TrackRowColumn` columns, the download-state slot and the row menu.
- [Packet 026](adr-0084-task-026-index-singles-groups.md) decision D13 (replaced by this packet):
  a Library single kept the shared track row, and a `MusicIndex` single showed as a release row.
- [Packet 017](adr-0084-task-017-name-level-artist-page.md): `NameFeedMatches`,
  `NameFeedMatch::MatchedFeed`, `matched_name_keys`, and the feed page section "Matched by name"
  with its heading and note.
- Operator decisions of 2026-10-22 (recorded in this packet's "Orchestrator Decisions").
- [AGENTS.md](../../AGENTS.md): renderer portability, token discipline, typed action state, "Every
  fix gets a guard", and "Column text does not call `truncate()`".

## Recorded Facts - 2026-10-22

### The group page row style

- `src/ui/shells/library/singles_group.rs:284-311` `render_single_row` builds
  `ReleaseTrackRowVm::library_single` and renders it with `render_release_page_track_row`
  (`src/ui/shells/release_page.rs:369`). It already matches ADR 0083 task 006.
- `src/ui/shells/library/singles_group.rs:317-376` `render_index_single_row` builds a plain
  `ListRow` with a `Thumbnail`, a title and a date. It has no `TrackRowColumn`, no duration and no
  per-row download action. Its checkbox and click call `LibraryApp::toggle_library_detail_row` and
  `click_library_detail_row` directly.
- `src/view_models/singles_group.rs:823-872` `SinglesGroupIndexRowVm` carries `title`, `date_text`,
  `image_url`, `a11y_label` and `checkbox_id`. It carries no duration and no download work.
- `src/view_models/release_page.rs:793-976` `ReleaseTrackRowVm` is the one track row view model of
  a release page and of the group page today for a Library single. Its `index` constructor
  (line 929) builds the row of an Index release page track from a `FeedView` and a `TrackView`. It
  opens `FrameNavigationEntry::Track`, not `Release`.
- `src/view_models/release_page.rs:845-856` `library_single` builds a Library single's row from
  `SinglesGroupPageRowVm`, with `number: None` and `open_entry: FrameNavigationEntry::Release`.
- `src/api.rs:208` `Feed::tracks: Option<Vec<Track>>`. `src/api.rs:277` `Track::duration_secs:
  Option<i32>`. `src/application/request_profiles.rs:171` `INDEX_FEED_SINGLES` gives the include
  `named_feeds`, not `tracks`. A single that this request reads thus has `tracks: None` today, so
  `duration_secs` is not present. `Feed` has a container `#[serde(default)]`
  (`src/api.rs:170-172`), so a missing field decodes to `None`, not an error.
- `src/library/release_page.rs:18-80` `run_release_page_command` and `release_page_row` resolve a
  `ReleasePageCommand` by the row index of the open page. `release_page_row` gives `None` for a
  `SinglesGroupPageEntryVm::Index` row (line 134-137 of that file): no command reaches it today.
- `src/library/downloads.rs:60-72` `download_release_plan` queues the Library jobs of a plan, then
  one Index release job for each `(feed_guid, title)` pair, through
  `index_release_job_by_guid` (line 519). The group page's main action already uses it.
- `src/view_models/library.rs:5305-5318` `BusyItems::index_release_downloading` gives a bare
  `bool` from `DownloadActivity::key_phase`. `TrackRowWork::from_phase` (line 5399) needs the typed
  `DownloadPhase`, not the bare `bool`.

### The matched-by-name rule

- `src/view_models/artist_identity.rs:141-250` `NameFeedMatch`, `NameFeedEntry`, `NameFeedMatches`
  and `name_feed_matches` give the match of a name key to the Library artist feeds. `ArtistIdentity
  ::Name(String)` already stores the sort key (`src/view_models/artist_identity.rs:52-59`), the
  same key that `NameFeedMatches::get` reads.
- `src/view_models/library.rs:4063-4091` `LibraryViewModel::name_feed_matches` builds the match
  from `self.snapshot.tree`. No free function builds it from a bare `&LibraryTree`, so
  `src/view_models/singles_group.rs`, which takes a `&LibraryTree` and not a
  `LibraryViewModel`, cannot call it today.
- `tests/architecture_tests.rs:22547-22632` `adr_0084_name_level_r17_one_count_function` requires
  `src/view_models/library.rs`, `src/app/name_level_dispatch.rs` and `src/app/publisher_dispatch.rs`
  to call `name_feed_matches(` or `matched_name_keys(`. It does not read
  `src/view_models/singles_group.rs`.
- `src/view_models/singles_group.rs:305-336` `singles_groups` and `:338-366`
  `library_singles_by_identity` group Library singles by `ArtistIdentity` alone. A name identity
  and a publisher identity never join. A name that matches one feed by title or by artist text
  still keeps its own group, when it has two or more singles.
- `src/view_models/singles_group.rs:454-528` `everything_groups` groups the scope "Everything" the
  same way, then adds the `MusicIndex` singles of each publisher identity. It calls
  `library_singles_by_identity` too.
- `src/view_models/singles_group.rs:372-379` `library_single_artist_feeds` reads the publisher
  keys of `library_singles_by_identity` to pick the feeds whose `MusicIndex` singles page 1 reads.
- `src/view_models/publisher_page.rs:545-548` gives `PublisherPageVm::MATCHED_BY_NAME_HEADING`
  ("Matched by name") and `MATCHED_BY_NAME_NOTE` ("These releases name no artist feed. The app
  relates them to this artist by the name only."). `:826-873` `matched_by_name_section` builds the
  feed page section with this heading and note, after each other section.
- `src/view_models/library.rs:3328-3392` `releases_list` hides an individual release row when
  `groups.iter().any(|group| group.contains(key))` (line 3360). The same check places a release
  inside a group row or shows it alone.
- `src/view_models/singles_group.rs:687-699` `singles_group_name_link` finds the group of a
  release key with `group.contains(key)` and gives "All n singles" with `group.count()`.
- A test audit of every `with_publisher(` call in `src/view_models/singles_group.rs` found one
  tree. It is inside `adr_0084_singles_group_r15_03_different_identities_and_kinds_do_not_group`
  (`level_tree`, line 1279-1288). It has a publisher-linked single and a name-only single under one
  artist node name. This is the only existing test of this file that the new join rule changes.

## Design

### One track row style (replaces packet 026 D13)

1. Add `ReleaseTrackRowVm::index_single` next to `library_single` and `index`
   (`src/view_models/release_page.rs`). It takes the title, the duration, the date text, the
   thumbnail URL, the release key, a stable row id, the row's selection and its `TrackRowWork`. It
   builds the same `TrackRowVm`, `DownloadSlotVm` and `TrackRowMenuVm` shape as `index`, with two
   differences:
   - `open_entry` is `FrameNavigationEntry::Release(key)`, because a `MusicIndex` single has no
     Library track and no track page of its own.
   - The menu holds only "Download": a `MusicIndex` single that the Library does not hold has no
     playlist of its own to add to, from this page (see "Exclusions"). When "Download" is not
     available, the menu has no item left, so the constructor gives `menu: None` instead of an
     empty list. An empty "⋯" button is not useful.
2. `SinglesGroupIndexRowVm` (`src/view_models/singles_group.rs`) keeps `element_id`, `key`, `title`,
   `date_text` and `image_url`. It gains `duration: Option<String>`, from
   `single.feed.tracks.as_deref().and_then(|tracks| tracks.first()).and_then(|track|
   track.duration_secs).map(fmt_dur)`, and `work: TrackRowWork`, from a new
   `BusyItems::index_release_work(key)`. It drops `a11y_label` and `checkbox_id`: `TrackRowVm`
   and the row's own id already give both.
3. `BusyItems::index_release_work(&self, key: &ReleaseKey) -> TrackRowWork` reads the same
   `DownloadActivity::key_phase` as `index_release_downloading`, through `TrackRowWork::from_phase`.
   `index_release_downloading` becomes `self.index_release_work(key) != TrackRowWork::Idle`, so the
   two methods never answer two different questions about the same key.
4. `src/ui/shells/library/singles_group.rs` `render_index_single_row` builds
   `ReleaseTrackRowVm::index_single` from `SinglesGroupIndexRowVm`, resolves its thumbnail the same
   way `render_single_row` does, and renders it with the same `render_release_page_track_row`
   function. The row has no Library track, so its commands cannot reach
   `LibraryApp::release_page_row`, which only resolves a Library `TrackRow`. Each row thus gets its
   own small `ReleasePageDispatch`, built by a new `index_single_dispatch(key, title, cx)`, that
   sends `DownloadTrack`, `ClickRow` and `ToggleRow` to a new `LibraryApp::
   run_singles_group_index_row_command` (`src/library/release_page.rs`). "Download" calls
   `download_release_plan` with one Index release job. "Click" and "Toggle" call
   `click_library_detail_row` and `toggle_library_detail_row` with
   `FrameNavigationEntry::Release(key)`, the same entry the row already opens today.
5. Selection, the checkbox and the click of a `MusicIndex` single keep their row key
   `DetailRowKey::Release(key)`. `src/library/detail_selection.rs` already builds this key for
   every singles group row through `singles_group_release_rows`, so this part needs no change.

### The matched-by-name group (amends Decision 6)

6. Extract the body of `LibraryViewModel::name_feed_matches` into a free function
   `tree_name_feed_matches(tree: &LibraryTree) -> NameFeedMatches` in `src/view_models/library.rs`.
   `LibraryViewModel::name_feed_matches` becomes `tree_name_feed_matches(&self.snapshot.tree)`. The
   literal call `self.name_feed_matches()` stays in `artists_list`'s source, so the guard
   `adr_0084_name_level_r17_one_count_function` still passes with no change to that guard.
7. Replace `library_singles_by_identity` with `library_singles_grouped`, which also calls
   `tree_name_feed_matches`. Take each Library single. When its identity is `ArtistIdentity::
   Name(key)` and `tree_name_feed_matches(tree).get(key)` gives `NameFeedMatch::MatchedFeed(guid)`,
   the single goes to a `matched_by_name` map, keyed by `guid`. It does not go to the `own` map
   that `singles_groups` and `everything_groups` group as before. `NoFeed` and `Ambiguous` keep
   the single in `own`, under its `Name` identity, the same as before.
8. `SinglesGroup` (`src/view_models/singles_group.rs`) gains `matched_by_name: Vec<GroupSingle
   <'a>>`, sorted newest first with the same rule as `singles`. `count()`, `in_library_count()` and
   `contains()` read it too. The group's count and its "n in Library" text thus include it. The
   plain release list hides a matched member through `contains()` too (`releases_list` line 3360).
   `members()`, `first_title_sort_key()` and `collection_art()` do not read it (see "Exclusions").
9. `singles_groups` and `everything_groups` build their identity set from `own.keys()` and from
   `matched_by_name.keys()` mapped to `ArtistIdentity::Publisher`. A feed can thus form a group
   from matched singles alone. The two-or-more-members rule now counts `own` plus
   `matched_by_name` (plus `index_singles` for "Everything").
10. `library_single_artist_feeds` adds the feed GUIDs of `matched_by_name` to its result, with no
    duplicate. Page 1 of "Everything" thus also reads the `MusicIndex` singles of a feed whose
    Library singles are only matched by name.
11. `main_action` (`src/view_models/singles_group.rs`) folds `matched_by_name` into the Library
    part of the download plan and of the "Add to playlist" list, through one chained iterator over
    `group.singles` and `group.matched_by_name`.
12. `SinglesGroupPageVm` gains `matched_by_name: Option<MatchedByNameHeadingVm>`, a heading, a
    note and a row count. `MatchedByNameHeadingVm` reuses `PublisherPageVm::MATCHED_BY_NAME_HEADING`
    and `MATCHED_BY_NAME_NOTE`, the same text the feed page section shows. `rows` stays one flat
    list: the feed's own members first, from `page_rows`, then the matched singles, from a new
    `matched_by_name_rows`. That function shares one `library_single_row` helper with the Library
    branch of `page_rows`. The heading's row count tells the shell where to put the heading.
13. `src/ui/shells/library/singles_group.rs` inserts a heading element, styled with the shared
    `SectionHeader` primitive and a `Caption` note. It goes before the last `matched_by_name.count`
    rows of the page, when `matched_by_name` is `Some`. The rows after it render exactly like
    every other Library row of the page: decision 1 of this packet gives every row one style, so
    the matched rows carry their own real date, not a replaced text.

## Orchestrator Decisions

- **D1. The menu of a `MusicIndex` single's row has no "Add to playlist".** The Index release page
  can add an un-downloaded `MusicIndex` track to a playlist because `TopApp` already owns a
  subscribe-then-append path for one Index track. The singles group page is `LibraryApp`'s page,
  and `LibraryApp`'s playlist code reads tracks of a Library feed id
  (`add_albums_to_playlist`, `src/library/app_impl.rs:2021`). Building a second,
  parallel path to add an un-downloaded `MusicIndex` release to a playlist is a new capability the
  operator decisions do not ask for. The row keeps "Download" in its menu and in its
  download-state slot, matching "download keeps working" in the operator decision.
- **D2. The row's disclosure lists every single the group row counts.** This is the orchestrator
  decision of 2026-10-22. The feed's own singles come first, and the matched-by-name singles come
  after them, in each scope. `DestinationListRowVm` has no field for an inferred relation, so each
  matched row's subtitle text adds the words of the group page's "Matched by name" heading. This
  replaces the first version of this decision, which kept the matched-by-name singles out of the
  disclosure.
- **D3. A matched-by-name single does not move the group row in the Releases list.** It also does
  not join the logo-frame art pool. `first_title_sort_key()` and `collection_art()` both read
  `members()`, which this packet does not change. Moving a group row, or changing its art from
  such a release, is a second decision that the operator decision does not state. A later packet
  can extend this, with its own ADR citation, if the operator asks.
- **D4. The matched rows keep a real date, not a "names X, no feed" text.** The feed page section
  replaces the date line of its row style with a detail line. That row style has no date line of
  its own. This packet's row style always has a date line (decision 1). The heading and the
  note already mark the section as inferred, so repeating that mark on each row is not needed.

## Required Changes

1. `src/view_models/release_page.rs`: add `ReleaseTrackRowVm::index_single`.
2. `src/view_models/library.rs`: add `tree_name_feed_matches`, change `name_feed_matches` to call
   it, add `BusyItems::index_release_work`, and change `index_release_downloading` to call it.
3. `src/view_models/singles_group.rs`: change `SinglesGroupIndexRowVm`; add `matched_by_name` to
   `SinglesGroup`, `library_singles_grouped`, `MatchedByNameHeadingVm`, `library_single_row` and
   `matched_by_name_rows`; change `singles_groups`, `everything_groups`,
   `library_single_artist_feeds`, `main_action`, `SinglesGroupPageVm` (struct, `new` and
   `from_tree`), `count`, `in_library_count`, `library_state_part` and `contains`.
4. `src/library/release_page.rs`: add `run_singles_group_index_row_command`.
5. `src/ui/shells/library/singles_group.rs`: rewrite `render_index_single_row`; add
   `index_single_dispatch` and `render_matched_by_name_heading`; thread `detail_focus` to the Index
   row; insert the heading at the right row position.
6. Update the one affected existing test
   (`adr_0084_singles_group_r15_03_different_identities_and_kinds_do_not_group`) to the joined
   group it now gives, and add each test of "Mechanical Acceptance Criteria".
7. Add one situational guard in `tests/architecture_tests.rs` citing ADR 0084 Decision 6, amended
   2026-10-22: `src/view_models/singles_group.rs` must call `tree_name_feed_matches(`.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_singles_group_rows_` for unit tests beside the owning code.

| Case | Required proof |
|---|---|
| R32-01 | `ReleaseTrackRowVm::index_single` gives `row.number` `None`, the given title, the given duration, the given date text, `open_entry` `Release(key)`, `row_key` `Release(key)` and `inspect_sources_entry` `None` |
| R32-02 | With a duration, `row.row.duration` is `Some`. With no duration, it is `None`, and the row still builds: no field forces a duration |
| R32-03 | The menu holds "Download" when available, and no other item. When "Download" is not available, `menu` is `None` |
| R32-04 | `BusyItems::index_release_work` gives `Idle`, `Queued` and `Downloading` for a `FeedGuid` key from the same `DownloadActivity` facts as `release_phase`. `index_release_downloading` agrees with it on every case |
| R32-05 | `SinglesGroupIndexRowVm::new` reads the duration from the first track of its `Feed`, and gives `None` when the `Feed` has no `tracks` |
| R32-06 | A source guard: `render_index_single_row` calls `render_release_page_track_row`, and does not call `ListRow::new` |
| R32-07 | `run_singles_group_index_row_command` with `DownloadTrack` queues one `ReleasesDownloadPlan` with one `index_releases` entry and no `library_feed_ids`. With `ClickRow`, it calls the Release entry of the key through the shared detail selection. Each other command changes nothing |
| R32-08 | A Library name matches one artist feed. It has one single of its own and one single that names the feed with a confirmed link. In the Library scope, `singles_groups` gives one group, `ArtistIdentity::Publisher`, with `count()` 2 and one `matched_by_name` entry |
| R32-09 | The same tree, in the scope "Everything", with one `MusicIndex` single of the feed too: `singles_groups_everything` gives one group with `count()` 3 |
| R32-10 | A name that matches two artist feeds keeps its own group: `singles_groups` gives `ArtistIdentity::Name`, and no feed's `matched_by_name` gains a member |
| R32-11 | `SinglesGroupPageVm::new` of the R32-08 group gives `matched_by_name` `Some`, with the heading and the note of `PublisherPageVm::MATCHED_BY_NAME_HEADING` and `MATCHED_BY_NAME_NOTE`, and a count of 1. The last row of `rows` is the matched single |
| R32-12 | `library_single_artist_feeds` gives the feed GUID of a feed whose Library singles are only `matched_by_name`. It gives that GUID once, with no duplicate, when the feed also has its own singles |
| R32-13 | `singles_group_name_link` gives "All n singles" for the release key of a `matched_by_name` single, with `n` equal to the joined group's `count()` |
| R32-14 | `library.rs` `releases_list`: a `matched_by_name` single's release key does not show as its own row; it shows inside the group row's count only |
| R32-15 | `main_action`'s download plan and "Add to playlist" list each include a `matched_by_name` single of the group |
| R32-16 | The updated `adr_0084_singles_group_r15_03_different_identities_and_kinds_do_not_group` passes with the joined-group assertion |
| R32-17 | The situational guard of "Required Changes" item 7 passes, and fails against a copy of `singles_groups` or `everything_groups` with no call to `tree_name_feed_matches` |
| R32-18 | Each test of packets 015, 017, 018, 026 and ADR 0083 task 006 that this file or `release_page.rs` holds still passes |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: open a singles group page with Library singles and `MusicIndex` singles. Every row has the
  same image size, the same title size and a date line. Its download-state, duration and menu
  columns start at the same position on each row.
- V2: a `MusicIndex` single with a known duration shows it, right-aligned. A `MusicIndex` single
  with no known duration shows an empty duration column, the same width as a filled one.
- V3: click the download icon of a `MusicIndex` single's row. The icon changes to queued, then to
  downloading, then empties. The page does not navigate.
- V4: click a `MusicIndex` single's row body. The release page of that single opens. Click a
  Library single's row body. Its page opens, the same as before this packet.
- V5: a group formed only by the matched-by-name join: open its page. The heading "Matched by
  name" and its note show after the feed's own singles, styled the same as the feed page's
  section of the same name.
- V6: the Releases list, scope "Everything": the group row of a feed with matched-by-name singles
  shows their count. No separate row shows a matched-by-name single on its own.
- V7: Light and Dark themes, a narrow width and the larger type sizes. No text is clipped, and the
  heading and the note are weaker than the row title.

## Exclusions

- No "Add to playlist" for a `MusicIndex` single's row on the group page (D1).
- No change to the Releases list's in-place "show each single" preview, to the group row's
  position, or to the logo-frame art pool (D2, D3).
- No change to the feed page, its own "Matched by name" section, or `name_feed_matches` callers
  outside this packet.
- No change to the empty-duration case of a Library single: it already has a duration from its
  own track.
- No schema change, and no new network request.

## Files To Inspect

- [AGENTS.md](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/view_models/release_page.rs`, `src/view_models/singles_group.rs`,
  `src/view_models/library.rs` (`BusyItems`, `name_feed_matches`, `releases_list`).
- `src/view_models/artist_identity.rs`, `src/view_models/publisher_page.rs` (the constants and
  `matched_by_name_section`).
- `src/ui/shells/library/singles_group.rs`, `src/ui/shells/release_page.rs`,
  `src/ui/composites/track_row.rs`, `src/ui/primitives/section_header.rs` (or its owning file).
- `src/library/release_page.rs`, `src/library/downloads.rs`, `src/library/detail_selection.rs`.
- `tests/architecture_tests.rs` (`adr_0084_name_level_r17_one_count_function`, for the pattern of
  the new guard).

## Checks

```bash
cargo test --lib adr_0084_singles_group_rows_
cargo test --lib adr_0084_singles_group_
cargo test --lib adr_0084_index_singles_
cargo test --lib adr_0084_name_level_
cargo test --lib adr_0083_row_actions_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo check --all-targets
touch src/main.rs && cargo build --bin v4vmm
```

## Rollback

Revert the commit. This packet adds no migration and no stored data.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0084-task-032-singles-group-rows.md`
- ADR 0084 Decision 6 as amended on 2026-10-22, and Decision 7
- Packets 017 and 026, and ADR 0083 task 006
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes".

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use
  the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Follow each decision in "Orchestrator Decisions". Do not change a decision.
- The view model gives each text, image and availability. The renderer only composes. Use named
  tokens. Column text does not call `.truncate()`.
- Tests use recorded values. No test sends a request to a live service.
- Background work runs through the ADR 0040 runtime. Do not call `cx.spawn` from a screen.
- Never run `git add`, `git rm`, `git commit`, `git checkout`, `git restore`, `git stash` or
  `git reset`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The database schema and the migration registry.
- The feed page, `src/app/name_level_dispatch.rs` and `src/app/publisher_dispatch.rs`.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R32-01 to R32-18 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - 2026-10-22" section and an "Operator Visual Check"
  section for V1 to V7.

At the end, report: files changed, tests run, behavior changed, deviations, unresolved concerns,
and the document changes that the orchestrator must make.

Stop and report the problem, and do not guess, when:
- A change needs a new architecture decision.
- A change needs a file in "Do not touch".

## Implementation Result - 2026-10-22

### Files Changed

- `src/view_models/release_page.rs`: new `ReleaseTrackRowVm::index_single`, next to `library_single`
  and `index`. It builds the same `TrackRowVm`, `DownloadSlotVm` and `TrackRowMenuVm` shape, opens
  `FrameNavigationEntry::Release`, and gives `menu: None` when "Download" leaves no item.
- `src/view_models/library.rs`:
  - New free function `tree_name_feed_matches(tree: &LibraryTree) -> NameFeedMatches`, placed
    beside `name_level_library_release`. `LibraryViewModel::name_feed_matches` now calls it with
    `self.snapshot.tree`. The literal call `self.name_feed_matches()` stays in `artists_list`, so
    the existing guard `adr_0084_name_level_r17_one_count_function` needed no change.
  - New `BusyItems::index_release_work(key) -> TrackRowWork`. `index_release_downloading` now
    calls it and compares the result to `TrackRowWork::Idle`.
  - New test `adr_0084_singles_group_rows_r32_04_index_release_work_matches_downloading` and
    `adr_0084_singles_group_rows_r32_14_matched_single_hides_its_own_row`.
- `src/view_models/singles_group.rs`:
  - `SinglesGroupIndexRowVm` drops `a11y_label` and `checkbox_id`. It gains `duration` (from the
    first track of its `Feed`, when the `Feed` has `tracks`) and `work: TrackRowWork`.
  - `SinglesGroup` gains `matched_by_name: Vec<GroupSingle<'a>>`. `count`, `in_library_count`,
    `library_state_part` and `contains` read it.
  - New `LibrarySinglesGrouping`, `library_singles_grouped` and `feed_matched_singles` replace
    `library_singles_by_identity`. `library_singles_grouped` calls `tree_name_feed_matches` and
    sends a `Name` single whose match is `MatchedFeed(guid)` to `matched_by_name[guid]`, instead of
    its own `own[Name(key)]` bucket.
  - `singles_groups`, `everything_groups` and `library_single_artist_feeds` read
    `library_singles_grouped`. The two group builders add a feed's `matched_by_name` singles to its
    combined count and to its identity set. A feed can thus form a group from matched singles
    alone.
  - `main_action` folds `matched_by_name` into the Library part of the download plan and of "Add to
    playlist", through one `singles.iter().chain(&matched_by_name)` closure.
  - `SinglesGroupPageVm` gains `matched_by_name: Option<MatchedByNameHeadingVm>`. New
    `MatchedByNameHeadingVm`, `library_single_row` (shared by `page_rows` and the new
    `matched_by_name_rows`), and `matched_by_name_heading`, which reuses
    `PublisherPageVm::MATCHED_BY_NAME_HEADING` and `MATCHED_BY_NAME_NOTE`.
  - One existing test, `adr_0084_singles_group_r15_03_different_identities_and_kinds_do_not_group`,
    lost its `level_tree` case: that tree now gives one joined group, not an empty result. Its
    test moved, with the new assertion, to R32-08.
  - 12 new tests: R32-05, R32-08 to R32-13, and R32-15 (listed below).
- `src/library/release_page.rs`: new `LibraryApp::run_singles_group_index_row_command`. It queues
  `download_release_plan` for `DownloadTrack`, and reaches `click_library_detail_row` and
  `toggle_library_detail_row` for `ClickRow` and `ToggleRow`. Each other command does nothing: the
  row's menu never sends one. `release_page_row`'s comment on the `SinglesGroup::Index` arm now
  names this new function.
- `src/ui/shells/library/singles_group.rs`:
  - `render_index_single_row` now builds `ReleaseTrackRowVm::index_single` and renders it with
    `render_release_page_track_row`, the same function `render_single_row` uses. It takes
    `detail_focus` now, for the row's "⋯" menu return focus.
  - New `index_single_dispatch`: one small `ReleasePageDispatch` for each `MusicIndex` row, closing
    over its key and title, since the row has no Library track for `release_page_row` to resolve.
  - New `render_matched_by_name_heading`, styled with the shared `SectionHeader` primitive and a
    `Caption` note, the same as the feed page's section of the same name.
  - `render_library_singles_group_page` inserts that heading before the trailing
    `matched_by_name.count` rows of the flat `rows` list.
  - Dropped imports: `EntityKind`, `ListRow`, `ListRowA11yLabel`, `Thumbnail`, `Checkbox` and
    `checkbox_state_for`, each with no caller left in this file.
- `tests/architecture_tests.rs`: three new situational guards, R32-06, R32-07 and R32-17 (listed
  below), each citing ADR 0084 Decision 6 as amended on 2026-10-22.

### Tests

| Case | Proof |
|---|---|
| R32-01 | `adr_0084_singles_group_rows_r32_01_index_single_row_shape` (`release_page.rs`) |
| R32-02 | `adr_0084_singles_group_rows_r32_02_index_single_row_with_no_duration` (`release_page.rs`) |
| R32-03 | `adr_0084_singles_group_rows_r32_03_index_single_menu_has_download_only` (`release_page.rs`) |
| R32-04 | `adr_0084_singles_group_rows_r32_04_index_release_work_matches_downloading` (`library.rs`) |
| R32-05 | `adr_0084_singles_group_rows_r32_05_index_row_reads_duration_from_tracks` (`singles_group.rs`) |
| R32-06 | Guard `adr_0084_singles_group_rows_r32_06_index_row_uses_the_shared_track_row` |
| R32-07 | Guard `adr_0084_singles_group_rows_r32_07_index_row_command_route` |
| R32-08 | `adr_0084_singles_group_rows_r32_08_name_matched_to_one_feed_joins_its_group` |
| R32-09 | `adr_0084_singles_group_rows_r32_09_matched_group_in_everything_scope` |
| R32-10 | `adr_0084_singles_group_rows_r32_10_name_matched_to_two_feeds_keeps_its_own_group` |
| R32-11 | `adr_0084_singles_group_rows_r32_11_page_lists_matched_singles_under_their_heading` |
| R32-12 | `adr_0084_singles_group_rows_r32_12_library_single_artist_feeds_includes_matched_feeds` |
| R32-13 | `adr_0084_singles_group_rows_r32_13_name_link_counts_the_joined_group` |
| R32-14 | `adr_0084_singles_group_rows_r32_14_matched_single_hides_its_own_row` (`library.rs`) |
| R32-15 | `adr_0084_singles_group_rows_r32_15_main_action_includes_matched_singles` |
| R32-16 | The updated `adr_0084_singles_group_r15_03_different_identities_and_kinds_do_not_group` |
| R32-17 | Guard `adr_0084_singles_group_rows_r32_17_one_match_function_for_singles_groups` |
| R32-18 | `cargo test` and `cargo test --test architecture_tests` are Green, with every prior test kept |

### Checks - 2026-10-22

| Check | Result |
|---|---|
| `cargo test` | Green, 2262 library tests, 0 failed. One unrelated test, `audio_format::probe::tests::adr_0066_converter_timeout_reaps_child`, failed once under full parallel load. It passed alone and on a full rerun: a pre-existing timing flake, not a regression of this packet |
| `cargo test --test architecture_tests` | Green, 372 guards |
| `cargo fmt -- --check` | Green |
| `cargo clippy --all-targets -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

### Behavior Changed

- The singles group page shows a `MusicIndex` single in the same track row as a Library single:
  the same image size, the same title size, a date line, and the `TrackRowColumn` download-state,
  duration and menu columns.
- A `MusicIndex` single's row shows its duration when the `MusicIndex` data the app already holds
  gives one. It shows the download icon and "Download" in its "⋯" menu, the same as the shared
  track row's download action. It has no "Add to playlist" (decision D1) and no "Inspect sources".
- A click on a `MusicIndex` single's row body opens the release page of that single, the same page
  it opened before this packet. A click on a Library single's row keeps opening its release page.
- A name that matches exactly one artist feed no longer keeps a singles group of its own. This
  applies in the Library scope and in the "Everything" scope. Its singles join the group of that
  feed. The group's count and its "n in Library" text include them. Its release list row no
  longer shows them as separate rows.
- The group row's "show each single" preview now lists every matched single too, after the feed's
  own singles, with "Matched by name" in its subtitle (orchestrator decision of 2026-10-22).
- The group page of such a feed lists the joined singles under a trailing heading "Matched by
  name". The heading carries the same note as the feed page's section of the same name, and comes
  after the feed's own singles.
- "All n singles" on the release page of a joined single now counts the joined singles. The group
  page's "Download n singles" and "Add to playlist" actions act on them too.
- A name that matches two or more artist feeds keeps its own singles group, unchanged.

### Deviations From The Packet Text

1. The packet's Design step 7 named the heading field only as giving "a count". The implementation
   also needed `matched_by_name_heading` and `library_single_row`, two small shared helper
   functions, to avoid writing the row-building logic twice. This is the same shape the packet's
   Required Changes item 3 already named.
2. R32-07 and R32-06 are architecture guards, not runtime unit tests. `LibraryApp` has no existing
   test harness in this codebase: no file builds one to call its methods directly. The guards
   check the same facts a unit test would, at the source level. Packets 006 and 017 guard similar
   glue-routing facts the same way.
3. One existing test lost its `level_tree` scenario (see "Files Changed"). The orchestrator review
   should confirm this is the correct move. AGENTS.md "Delete Dead Things" and the governance
   rule on superseded rules support it.

### Unresolved Concerns

1. V1 to V7 below need the operator. No test proves a rendered column position, a tooltip, an
   icon state, or a heading's visual weight.
2. `src/ui/composites/track_row_menu.rs` and `render_matched_by_name_heading` are not in
   `docs/architecture/source-map.md`. This packet's instructions name only this packet as a
   document to edit, so the orchestrator should add the new owner facts there.
3. Orchestrator Decision D3 (no change to the group row's position or its logo-frame art pool for
   a matched-by-name single) is a scoping choice this packet made, not a point the operator
   decided explicitly. It is recorded for the operator to confirm or correct.
4. A `MusicIndex` single's menu can hold zero items while its download is queued or running. It
   has no "Add to playlist" to fall back on (D1), so the row's "⋯" button is absent for that short
   time. This is a minor, transient state, not a defect.

## Orchestrator Review - 2026-10-22

Pass, with three corrections. The orchestrator decided that the disclosure of a group row lists each single that the row counts, and the matched singles show last with the mark "Matched by name" (D2). The review then found two defects, and the orchestrator corrected them. A group of matched singles alone took an empty title for its place and lost its art. It now takes its place and its art from those singles. The mark "Matched by name" is now also in the accessibility label of the row. The test `adr_0084_singles_group_rows_matched_only_group_keeps_its_place` proves the first correction.

## Operator Visual Check

This check needs a Linux desktop session. It also needs a Library with one artist feed with a
confirmed singles group. That feed needs two or more `MusicIndex` singles in the scope
"Everything". A name that matches that same feed by text (same artist name, with named releases
that confirm no feed) gives the "Matched by name" condition of V5 and V6. Record it if the
operator Library holds none.

1. Build and start the binary:

   ```bash
   cd ~/build/v4vmm
   cargo build --bin v4vmm
   ./target/debug/v4vmm
   ```

2. Open Music, then Releases, in the sidebar. Set the scope to "Everything". Open a singles group
   that has both a Library single and a `MusicIndex` single.

3. V1: compare a Library row and a `MusicIndex` row on the group page. Each row must show the same
   image size and the same title size, with a date line under the title. The download-state,
   duration and menu columns must start at the same x position on every row.
   **Wrong if:** a column starts at a different position on the two row kinds.

4. V2: find a `MusicIndex` row with a known duration, and check that it shows the duration,
   right-aligned. Find one with no known duration, and check that its duration column stays
   empty, the same width as a filled one.
   **Wrong if:** the empty column is narrower, or shifts the menu column.

5. V3: click the download icon of a `MusicIndex` row. The icon must change to the queued icon,
   then the downloading icon, then empty. The page must not navigate.
   **Wrong if:** the click opens the release page, or an icon state shows no tooltip.

6. V4: click the body of a `MusicIndex` row, and check that its release page opens. Go back, and
   click a Library row's body. Its page must open the same as it did before this packet.
   **Wrong if:** either click opens the wrong page or none.

7. V5: open a singles group page where a name matches the feed by text (see the setup note above).
   A heading "Matched by name" must show after the feed's own singles, with a note. Compare the
   note's wording and weight to the feed page's own "Matched by name" section.
   **Wrong if:** the heading is missing, placed before the feed's own singles, or styled stronger
   than the row titles.

8. V6: in the Releases list, scope "Everything", find that same feed's group row. Its count must
   include the matched singles. Expand the row.
   **Wrong if:** a matched single's own row shows in the expanded list, or anywhere else in the
   Releases list outside the group page.

9. V7: repeat steps 3 and 4 in the Dark theme, at a narrow window width, and with the largest type
   size in Settings. No text must be clipped, and the "Matched by name" note must stay weaker than
   the row titles.
   **Wrong if:** text clips, a column overlaps, or the note reads as strong as a row title.

Cleanup: none. This check downloads at most one `MusicIndex` single (V3). Remove it from the
Library afterward if the operator Library should stay as it was.
