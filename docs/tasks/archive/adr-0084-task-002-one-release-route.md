# ADR 0084 Task 002: One Route For Each Release

Status: Implementation recorded - 2026-10-08. The mechanical checks are Green. The operator visual check V1 to V5 is open.

## Goal

Each release has one navigation entry, keyed by its identity, whatever its origin. A release opened from the Library and the same release opened from the Index are the same page in the navigation stack and the breadcrumb.
The origin selects only the data source of the page. A download changes the data source of the open page in place, with no change to the navigation stack.

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md) Decision 4 and its Verification section. Decision 4 amends ADR 0049: the drill-down route for each origin goes.
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 9: one navigation stack ([task 001](archive/adr-0084-task-001-one-navigation-stack.md)).
- The durable set in [AGENTS.md](../../AGENTS.md): renderer portability, typed action state, and "Current-view state must update in place".

## Recorded Facts - 2026-10-08

- `FrameNavigationEntry` in `src/view_models/workspace/nav.rs` has two release entries: `AlbumDetail(i64)`, keyed by the Library feed id, and `IndexFeedDetail { id, label }`, keyed by the feed GUID with a display label.
- The album page has one view model for both origins: `album_page_actions` and `AlbumPageOrigin` in `src/view_models/album_page.rs`, and `ReleaseDetailVm` in `src/view_models/entity_detail.rs`. Two assembly paths stay:
  - Library: `render_library_feed_detail` in `src/ui/shells/library/feed_detail.rs`, from `LibraryDetail::Album` in `LibraryApp`.
  - Index: `render_index_feed_or_fallback_detail` and `index_feed_detail_slots` in `src/app/search_dispatch.rs`, from `TopApp.index_feed_detail_state`.
- The body switch in `render_workspace_content` in `src/app.rs` renders `IndexFeedDetail` from the fetch state when a search is active. Without a search, it renders the cached content-list row (`recent_music_index_feed_detail`), else the content list. `AlbumDetail` renders the Library mount.
- `restore_index_feed_detail_for_nav` returns early when no search is active, so Back to an Index release that opened from the content list does not fetch it again.
- `show_downloaded_index_feed` mounts the Library album and calls `nav.replace_current(FrameNavigationEntry::AlbumDetail(feed_id))`.
- A Library feed can have no feed GUID. `feeds.feed_guid` can be null, and `AlbumNode.feed_guid` is `Option<String>`. `db::find_feed_id_by_guid` maps a feed GUID to a Library feed id.
- These places push or match a release entry:
  - the sidebar album rows (`src/ui/shells/library/sidebar.rs`)
  - the album name link of a track page (`LibraryAppEvent::OpenAlbumPage`, `TrackNameLinkTarget`)
  - the Index content-list rows (`open_content_list_row`, `LibraryAppEvent::OpenIndexFeedDetail`)
  - the feed search results (`handle_index_feed_result_selected`)
  - `open_library_album_in_frame`.
- These guards name the two entries: `adr_0049_inspector_source_ownership_is_guarded`, `adr_0024_loading_shape_readiness_gate_is_locked` and `nav_top_drives_content_list_body_switch` in `tests/architecture_tests.rs`.

## Design

```rust
/// The identity of one release (ADR 0084 Decision 4).
pub(crate) enum ReleaseKey {
    /// The feed GUID. Each release with a feed GUID uses it, Library or Index.
    FeedGuid(String),
    /// A Library feed with no feed GUID. No Index release can match it.
    LibraryFeed(i64),
}

/// The data source of the open release page.
pub(crate) enum ReleasePageSource {
    Library { feed_id: i64 },
    Index { feed_guid: String },
}
```

- `FrameNavigationEntry::Release(ReleaseKey)` replaces `AlbumDetail(i64)` and `IndexFeedDetail { id, label }`. The entry holds no label, so two opens of one release are equal entries.
- One resolver gives the source: `LibraryFeed(id)` is the Library. `FeedGuid(guid)` is the Library when `db::find_feed_id_by_guid` finds a feed, else the Index. The resolver is a pure function in `src/view_models/` that takes the lookup result. The top app does the lookup.
- The top app keeps the source of the open release page. It sets the source when a release opens or restores, and after a download. The body switch reads the stored source. It does no database lookup during render.
- The breadcrumb label of a release is its title: the Library album name, the loaded Index title, or the label that the opening row gave. The top app keeps the labels that rows give, by feed GUID. Without a label, the breadcrumb shows "Release".
- The frame title of a release page is "Release" (ADR 0084 Decision 5 vocabulary).

## Required Changes

1. Add `ReleaseKey` and `FrameNavigationEntry::Release(ReleaseKey)` in `src/view_models/workspace/nav.rs`. Delete `AlbumDetail` and `IndexFeedDetail`. Update `display_label` and the breadcrumb element id.
2. Add `ReleasePageSource` and the resolver in `src/view_models/`. Add `ReleaseKey` constructors for an `AlbumNode` and for an Index feed GUID.
3. Each place in Recorded Facts pushes `Release(key)`. A Library album with a feed GUID uses `FeedGuid`.
4. Merge `open_library_album_in_frame` and `push_index_feed_detail` into one top app function that pushes the entry, resolves the source, and restores the page.
5. `restore_content_list_nav_entry` restores a release through the resolver for each origin. Delete the "search is active" condition: Back to an Index release fetches it when no current result exists. Keep the stale-result check of `index_feed_detail_result_is_current`.
6. The body switch renders a release from the stored source. For the Index source, it renders the fetch state, and the cached content-list row while the fetch loads.
7. `show_downloaded_index_feed` changes the stored source to the Library and mounts the Library album. It changes no navigation entry.
8. Update the three guards in Recorded Facts. Delete the text that requires the two old entries. Add the situational guard of R2-06.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_release_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R2-01 | The `ReleaseKey` of a Library album with a feed GUID equals the `ReleaseKey` of an Index row with the same feed GUID |
| R2-02 | The `ReleaseKey` of a Library album with no feed GUID is `LibraryFeed` with its feed id |
| R2-03 | The resolver gives the Library source for `LibraryFeed`, and for `FeedGuid` when the lookup finds a feed. It gives the Index source when the lookup finds no feed |
| R2-04 | A navigation path that opens one release from the Library and then from the Index holds the release one time |
| R2-05 | The breadcrumb label of a release entry with no known title is "Release" |
| R2-06 | A guard blocks `AlbumDetail` and `IndexFeedDetail` in `src/`, blocks `replace_current` in `show_downloaded_index_feed`, and names ADR 0084 Decision 4 |
| R2-07 | A completed download changes only the open release page of the downloaded feed GUID. The orchestrator added this case in review |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: one release opened from the sidebar and from an Index content-list row shows the same page layout, with its Library state.
- V2: "Download album" on an Index release page changes the page to the Library state in place. Back goes to the page before the release, not to a second copy of it.
- V3: the breadcrumb shows the release title, one time.
- V4: Back to an Index release that opened from the content list shows that release, not the content list.
- V5: normal and narrow widths, Light and Dark.

## Exclusions

- No track route change. [Task 003](adr-0084-task-003-one-track-route.md) owns it.
- No merge of the two page assembly paths into one renderer. The routes change in this packet. The renderers stay.
- No release kind, no singles group and no scope control. Packet 005 owns them.
- No schema change and no new MusicIndex request.

## Files To Inspect

- [Agent rules](../../AGENTS.md), the [source map](../architecture/source-map.md) and the [phase plan](../plans/adr-0084-music-column-browser-phase-plan.md).
- `src/view_models/workspace/nav.rs`, `breadcrumb.rs`, `tests.rs`.
- `src/view_models/album_page.rs`, `src/view_models/entity_detail.rs`, `src/view_models/library.rs` (`AlbumNode`, `index_feed_selection`).
- `src/app.rs` (`render_workspace_content`, `content_list_frame_title`, `restore_content_list_nav_entry`), `src/app/breadcrumb.rs`, `src/app/search_dispatch.rs`.
- `src/library.rs`, `src/library/app_impl.rs` (`hydrate_detail_from_nav`, `navigation_label`, `open_content_list_row`, `open_track_name_link`).
- `src/ui/shells/library/sidebar.rs`.
- `src/db.rs` (`find_feed_id_by_guid`).
- `tests/architecture_tests.rs`.

## Checks

```bash
cargo test --lib adr_0084_release_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md`, `docs/architecture/source-map.md` and `docs/plans/adr-0084-music-column-browser-phase-plan.md`
- This packet: `docs/tasks/adr-0084-task-002-one-release-route.md`
- ADR 0084 Decisions 4 and 9
- Each file in "Files To Inspect"

Goal:
- Replace `AlbumDetail` and `IndexFeedDetail` with one `Release(ReleaseKey)` navigation entry and one data-source resolver, as "Design" and "Required Changes" give.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- View models carry no GPUI type. The resolver is a pure function. The top app does the database lookup, never the render path.
- Keep each stale-result check of the Index fetch.
- Background work uses the ADR 0040 runtime. Never call `cx.spawn` from a screen.
- Never run `git checkout`, `git restore`, `git stash`, `git reset` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- `TrackDetail`, `IndexTrackDetail` and the track pages. Task 003 owns them.
- The database schema and the migration registry.
- Any ADR.
- Each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R2-01 to R2-06 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result" section and an "Operator Visual Check" section for V1 to V5. The check gives numbered steps, the expected result, an incorrect result, and the cleanup.

Test commands:
- Each command in "Checks".

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- A release opens from a place that has neither a feed GUID nor a Library feed id.
- The Index page needs data that only the old `label` field carried.
- A guard asserts an ADR 0049 rule that ADR 0084 Decision 4 does not replace.
- A change needs a file in "Do not touch".

## Implementation Result - 2026-10-08

Each Required Change is done.

### Files Changed

- `src/view_models/workspace/nav.rs`: `FrameNavigationEntry::Release(ReleaseKey)` replaces
  `AlbumDetail(i64)` and `IndexFeedDetail { id, label }`. New types: the `ReleaseKey` enum
  (`FeedGuid`, `LibraryFeed`) and its `from_index_feed_guid` constructor, the `ReleasePageSource`
  enum (`Library`, `Index`), and the pure function `resolve_release_page_source`.
  `display_label` gives the text "Release" for a `Release` entry with no known title.
- `src/view_models/workspace/breadcrumb.rs`: the breadcrumb element id of a `Release` entry
  carries its `ReleaseKey`. A new function, `release_key_id`, builds that id.
- `src/view_models/workspace/mod.rs`: re-exports `ReleaseKey`, `ReleasePageSource` and
  `resolve_release_page_source`.
- `src/view_models/workspace/tests.rs`: each test that built or read an `AlbumDetail` or
  `IndexFeedDetail` entry builds and reads a `Release(ReleaseKey)` entry. New tests cover
  R2-01 to R2-05. The R2-01 to R2-03 key and resolver tests sit in this file, not in `nav.rs`.
  An existing ADR 0046 Task 015 guard forbids a `#[cfg(test)]` block in `nav.rs`.
- `src/view_models/library.rs`: a new function, `AlbumNode::release_key`, gives the `ReleaseKey`
  of an album. It uses the feed GUID when the album has one, else the Library feed id, else
  `None`. A new test module covers R2-01 and R2-02 from the album side.
- `src/app.rs`: two new fields on `TopApp`.
  - `release_page_source` and `release_page_labels` are the two new fields.
  - `content_list_frame_title` and the `render_workspace_content` body switch read a release
    page from the stored `release_page_source`, with no database query.
  - The Index branch shows the cached content-list row while the operator's own fetch is
    loading.
  - `restore_content_list_nav_entry` calls `restore_release_page_source_for_nav`. It no longer
    calls `restore_index_feed_detail_for_nav`.
- `src/app/search_dispatch.rs`: the functions that open a release page.
  - `open_release_in_frame` replaces `push_index_feed_detail` and the Index half of
    `open_library_album_in_frame`.
  - `open_library_release_in_frame` replaces the Library half of `open_library_album_in_frame`.
  - A new function, `apply_release_page_source`, resolves the data source and mounts it. The
    top app runs the `db::find_feed_id_by_guid` lookup. The render path runs no lookup.
  - For the Library source, `apply_release_page_source` asks the Library to select the album.
  - For the Index source, it clears the Library detail. It starts its own fetch when the
    mounted fetch does not agree with the feed GUID.
  - `restore_release_page_source_for_nav` calls `apply_release_page_source` for a restored
    `Release` entry. For each other entry, it clears the stored source.
  - Three new read-only functions give data to these paths: `index_feed_detail_is_pending`,
    `index_feed_loaded_title`, and `release_opening_label`.
  - `show_downloaded_index_feed` changes `release_page_source` to the Library source and mounts
    the album. It leaves the navigation entry unchanged.
- `src/app/breadcrumb.rs`: a new function, `release_navigation_label`, gives the breadcrumb
  label of a `Release` entry. It reads the stored source first, then the loaded Index title,
  then the label the opening row gave.
- `src/library/app_impl.rs`: `hydrate_detail_from_nav` no longer handles a release entry. The
  top app's resolved source owns that mount. `navigation_label` gives the Library album name for
  a `Release(LibraryFeed(feed_id))` entry.
- `src/ui/shells/library/sidebar.rs`: an album row opens `FrameNavigationEntry::Release`, built
  from `AlbumNode::release_key`. An album with no feed GUID and no feed id keeps calling
  `select_album` directly, with no navigation entry, as before this packet.
- `tests/architecture_tests.rs`: one new situational guard,
  `adr_0084_release_one_route_replaces_the_two_entries` (R2-06). Five existing guards change:
  `adr_0046_forward_go_forward_and_on_forward_have_production_callers`,
  `adr_0046_forward_restores_pages_through_the_shared_back_restore_path`,
  `adr_0049_inspector_source_ownership_is_guarded`,
  `adr_0024_loading_shape_readiness_gate_is_locked`, and
  `nav_top_drives_content_list_body_switch`. Each changed check names ADR 0084 Decision 4 in its
  replacing comment.

### Behavior Changed

- The Library sidebar, an Index row, an Index search result, and a content-list row each open
  the same release through the same `Release` entry. The opened release shares one page in the
  breadcrumb and in the history.
- "Download album" on an Index release page changes the page to the Library state, in the same
  place. It adds no new navigation entry. Back goes to the page before the release.
- Back, Forward, and the breadcrumb select resolve a release through the same function an
  initial open uses. An Index release always starts its own fetch, not only when a search is
  active. A release opened from a content-list row also starts the operator's own MusicIndex
  fetch. The cached content-list row shows immediately. The fetch result replaces that row
  when it differs.
- The breadcrumb and the frame title show the word "Release" in place of "Album" or "Feed".

### Checks

| Command | Result |
|---|---|
| `cargo test --lib adr_0084_release_` | Green, 8 tests |
| `cargo test` | Green, 1,865 unit tests and 305 guards |
| `cargo test --test architecture_tests` | Green, 305 tests |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

### Deviations From The Task

1. `ReleaseKey`, `ReleasePageSource`, and `resolve_release_page_source` sit in
   `src/view_models/workspace/nav.rs`, re-exported from `workspace/mod.rs`. This packet adds no
   separate file for them. They are tightly linked to `ReleaseKey`'s owning entry, `Release`.
   The "Files To Inspect" list already named `nav.rs` for this change.
2. The R2-01 to R2-03 key and resolver tests sit in `src/view_models/workspace/tests.rs`, and in
   `src/view_models/library.rs` with the album-side constructor. They do not sit in `nav.rs`.
   An existing guard, `adr_0046_forward_go_forward_and_on_forward_have_production_callers`,
   forbids a `#[cfg(test)]` block in `nav.rs`. The first draft of this work put the tests there
   and broke that guard. Moving the tests to the named navigation-model test file, and to
   `library.rs`, keeps each guard Green.
3. This packet does not edit `docs/pending-human-checks.md` or
   `docs/plans/broadcast-chain-delivery-order.md`. AGENTS.md asks for the open visual gate to go
   in both files. This task's own rules name only this packet file as an editable document. The
   open V1 to V5 gate still needs a record in both files.

### Unresolved Concerns

- None found in this packet's scope.

## Operator Visual Check

This check reads and changes your Library. It needs these things: a Linux desktop session, this
checkout, and a configured MusicIndex endpoint. It also needs a Library with one release that
MusicIndex also indexes, and one release from MusicIndex that is not in your Library.

1. Close v4vmm. Make the binary:

   ```bash
   cd /home/citizen/build/v4vmm
   cargo build --bin v4vmm
   ```
2. Open the app:

   ```bash
   target/debug/v4vmm
   ```
3. Open Music.

**V1 - one release page for both origins**

4. In the sidebar, click an album that is in your Library and that MusicIndex also indexes.
   - Correct: the release page opens, with its Library actions (for example "Add to playlist")
     and its tracks.
5. Use the toolbar search to find the same release by name. Open its Index result.
   - Correct: the same page layout opens, still in its Library state. It offers no
     "Download album" action.
   - Incorrect: a different layout shows, or the page offers "Download album" for a release
     that is already in your Library.

**V2 - a download moves the page to the Library state in place**

6. Search for a release that is not in your Library. Open its Index result.
   - Correct: the page shows "Download album" as its filled action.
7. Click "Download album". Wait for the download to complete.
   - Correct: the same page updates in place to show the Library actions. No navigation
     happens, and no second page flashes into view.
8. Click Back.
   - Correct: the page before the release shows, for example the search results.
   - Incorrect: the release page shows again, or the content area is empty.

**V3 - the breadcrumb shows the release title one time**

9. Look at the breadcrumb above the open release page.
   - Correct: the breadcrumb shows the release title, for example "Library › Selected Ambient
     Works", one time.
   - Incorrect: the breadcrumb shows "Release" after the page finishes loading, or shows the
     release two times.
10. On the release page, open a track. On the track page, click the publisher name, then open a
    different release from the publisher page.
    - Correct: the breadcrumb shows the title of each release, each one time.
    - Incorrect: two release segments show the same title.

**V4 - Back to an Index release from the content list shows that release**

11. In the content list, open an Index release row that is not a search result. An example row
    is "Recent" or a different browse list.
12. Click Back, then click Forward.
    - Correct: Forward shows that same release again, with its MusicIndex detail, not an empty
      content list.
    - Incorrect: Forward shows the content list with no release, or shows a different release.

**V5 - widths, themes and scale**

13. Resize the window to its usual width, then to its narrowest width. Open Settings -> General
    (`Ctrl+Comma`). Select Light, then Dark, then a larger scale. Reopen a release page after
    each change.
    - Correct: the release page, its image, its actions and the breadcrumb lay out with no
      clipped text and no clipped control.
    - Incorrect: a control is clipped, overlaps a different control, or disappears.
14. Close the app.

**Clean-up**

Step 7 downloaded one release into your Library. Keep it, or open its page, choose
"Remove album…" from its "⋯" menu, and complete the removal.

## Orchestrator Review - 2026-10-08

The orchestrator reviewed the diff against ADR 0084, this packet and the [review checklist](../reviews/adr-0084-review-checklist.md). The implementation keeps to the design. Two defects were found and corrected in review:

1. `release_navigation_label` labeled each release segment from the source of the open page. In a path with two releases, both segments showed one title. Each segment now uses its own key: the new `LibraryApp::release_label` gives the Library album name by key, then the loaded Index title, then the opening label.
2. `show_downloaded_index_feed` mounted the downloaded album and set the page source for each completion. A download that completed after the operator opened another page replaced that page. The new pure function `release_download_updates_open_page` limits the change to the open release page of the downloaded feed GUID. Test R2-07 proves it, and the R2-06 guard now requires the call. The guard also fails when `show_downloaded_index_feed` is missing.

After the corrections, each check is Green: 9 `adr_0084_release_` tests, 1,866 unit tests, 305 guards, `fmt`, `clippy -D warnings`, `check --all-targets` with no warning, and `cargo build --bin v4vmm`.

Merge recommendation: pass. Task 003 needs no change, except that it uses `release_label` and `release_download_updates_open_page` as the model for its track equivalents.
