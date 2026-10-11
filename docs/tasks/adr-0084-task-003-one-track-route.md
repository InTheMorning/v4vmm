# ADR 0084 Task 003: One Route For Each Track

Status: Implementation recorded - 2026-10-11. The mechanical checks are Green. The operator visual check V1 to V5 is open.

## Goal

Each track has one navigation entry, keyed by its identity, whatever its origin. A Library track row in the content list opens its track page.
The origin selects only the data source of the page. A download changes the data source of the open page in place.

This packet fixes finding 2 of the [Music flow map](../architecture/music-flow-map.md).

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md) Decision 4 and its Verification section. A track is identified by its item GUID and its feed GUID.
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 9 and [task 001](archive/adr-0084-task-001-one-navigation-stack.md).
- [Task 002](archive/adr-0084-task-002-one-release-route.md): `ReleaseKey` and the resolver method.
- The durable set in [AGENTS.md](../../AGENTS.md).

## Recorded Facts - 2026-10-08

- `FrameNavigationEntry` has two track entries: `TrackDetail(i64)`, keyed by the Library track id, and `IndexTrackDetail { id, label }`. The Index `id` is `<feed_guid>:<item_guid>`, or `<item_guid>` alone when the result has no feed GUID (`split_index_track_target` in `src/app/search_dispatch.rs`).
- The track page has one view model for both origins: `TrackDetailVm` in `src/view_models/track_detail.rs`, with `TrackDetailSurfaceContext::Library` or `Discover`. Two assembly paths stay:
  - Library: `render_library_track_detail` in `src/ui/shells/library/track_detail.rs`, from `LibraryDetail::Track`.
  - Index: `index_track_detail_slots` and `render_index_track_detail`, from `TopApp.index_track_detail_state`.
- The body switch renders `IndexTrackDetail` only when a search or a name-match page is open. Otherwise the entry falls to the Library mount, and `hydrate_detail_from_nav` clears the Library detail. The operator then sees the content list, not the track.
- A Library content-list row is a track. Its row id is the Library track id as text. `open_content_list_row` in `src/library/app_impl.rs` finds only Index release rows, so a click on a Library track row does nothing.
- `download_index_track` refreshes the Library and changes no page. `download_index_album_of_track` downloads the release of an Index track.
- `db::TrackRow` has `item_guid: String` and `feed_guid: Option<String>`. `tracks` has `UNIQUE(feed_id, item_guid)`. `db::find_track_id` keys on the feed URL, not the feed GUID. No query finds a track by its feed GUID and item GUID.
- These places push or match a track entry:
  - the sidebar track rows, the album track rows (`feed_detail.rs`) and the playlist track rows (`playlist_detail.rs`)
  - the Library track search results, the Index track search results and the name-match rows
  - `hydrate_detail_from_nav`
- These guards name the two entries: `adr_0049_inspector_source_ownership_is_guarded`, `adr_0024_index_track_detail_uses_rich_track_view_path`, `adr_0024_loading_shape_readiness_gate_is_locked` and `nav_top_drives_content_list_body_switch`.

## Facts From Task 002 - 2026-10-11

[Task 002](archive/adr-0084-task-002-one-release-route.md) is complete. Use its names and its method:

- `ReleaseKey`, `ReleasePageSource`, `resolve_release_page_source` and `release_download_updates_open_page` are in `src/view_models/workspace/nav.rs`, with re-exports from `mod.rs`. Put `TrackKey`, `TrackPageSource` and the track resolver there too.
- `nav.rs` must hold no `#[cfg(test)]` block. The guard `adr_0046_forward_go_forward_and_on_forward_have_production_callers` forbids it. Put the tests in `src/view_models/workspace/tests.rs`, and the `TrackRow` key tests beside its constructor.
- `AlbumNode::release_key` in `src/view_models/library.rs` builds a release key. A `TrackRow` key uses the same rule for its release.
- The top app stores the release source in `TopApp.release_page_source` and sets it in `apply_release_page_source` (`src/app/search_dispatch.rs`). `restore_release_page_source_for_nav` restores it. Add the track equivalents with the same shape.
- `LibraryApp::release_label` labels a release by its key. Each breadcrumb segment uses its own key, never the source of the open page. Add `LibraryApp::track_label` by the same rule.
- `show_downloaded_index_feed` changes the open page only through `release_download_updates_open_page`. Each track download completion uses a track equivalent.
- The guard `adr_0084_release_one_route_replaces_the_two_entries` is the model for the R3-06 guard.

## Design

```rust
/// The identity of one track (ADR 0084 Decision 4).
pub(crate) struct TrackKey {
    /// The release of the track. `None` only for an Index result with no
    /// feed GUID.
    release: Option<ReleaseKey>,
    /// The RSS item GUID.
    item_guid: String,
}

/// The data source of the open track page.
pub(crate) enum TrackPageSource {
    Library { track_id: i64 },
    Index { target: String },
}
```

- `FrameNavigationEntry::Track(TrackKey)` replaces `TrackDetail(i64)` and `IndexTrackDetail { id, label }`.
- The resolver gives the Library source when the Library holds the track: by feed GUID and item GUID for `FeedGuid`, and by feed id and item GUID for `LibraryFeed`. Otherwise it gives the Index source. A key with no release is always the Index source.
- The top app keeps the source of the open track page, with the method of task 002 for releases.
- The breadcrumb label is the track title: the Library title, the loaded Index title, or the label that the opening row gave. Without a label, the breadcrumb shows "Track".

## Required Changes

1. Add `TrackKey` and `FrameNavigationEntry::Track(TrackKey)`. Delete `TrackDetail` and `IndexTrackDetail`. Update `display_label` and the breadcrumb element id.
2. Add `TrackPageSource` and the resolver in `src/view_models/`. Add `TrackKey` constructors for a `TrackRow` and for an Index track target.
3. Add two read queries in `src/db.rs`. One finds a Library track id by feed GUID and item GUID. The other finds it by feed id and item GUID. Add no migration.
4. Each place in Recorded Facts pushes `Track(key)`.
5. A Library track row in the content list opens its track page in the current path. The content-list view model gives the open target of each row: a `ReleaseKey` for a release row and a `TrackKey` for a track row.
6. `restore_content_list_nav_entry` restores a track through the resolver. Delete the "search or name-match page is open" condition of the body switch. Keep the stale-result check of `index_track_detail_result_is_current`, and keep the name-match cache.
7. After `download_index_track` and `download_index_album_of_track` complete, the top app resolves the source of the open track page again. The page changes to the Library state in place. No navigation entry changes.
8. Update the guards in Recorded Facts. Delete the text that requires the two old entries. Extend the task 002 guard to the track entries.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_track_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R3-01 | The `TrackKey` of a Library track with a feed GUID equals the `TrackKey` of an Index target with the same feed GUID and item GUID |
| R3-02 | The `TrackKey` of a Library track in a feed with no feed GUID carries `ReleaseKey::LibraryFeed` |
| R3-03 | The resolver gives the Library source when the lookup finds the track, and the Index source when it does not. A key with no release gives the Index source |
| R3-04 | The content-list view model gives a `TrackKey` open target for a Library track row and a `ReleaseKey` open target for an Index release row |
| R3-05 | The two new read queries find a track by feed GUID and item GUID, and by feed id and item GUID, in an in-memory database |
| R3-06 | A guard blocks `TrackDetail(` and `IndexTrackDetail` in `src/`, blocks a navigation change in the two download completions, and names ADR 0084 Decision 4 |
| R3-07 | A release row never gives a track key, even when its id equals a Library track id. The orchestrator added this case to the R3-04 test in review |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: a click on a Library track row in the content list opens its track page.
- V2: one track opened from a playlist and from an Index search shows the same page layout, with its Library state.
- V3: "Download album" on an Index track page changes the page to the Library state in place. Back goes to the page before the track.
- V4: Back to an Index track page after the search closes shows that track, not the content list.
- V5: normal and narrow widths, Light and Dark.

## Exclusions

- No merge of the two page assembly paths into one renderer.
- No change to the name-match page and its rows, except the entry that a row pushes.
- No schema change and no new MusicIndex request.

## Files To Inspect

- [Agent rules](../../AGENTS.md), the [source map](../architecture/source-map.md), the [phase plan](../plans/adr-0084-music-column-browser-phase-plan.md) and the Implementation Result of [task 002](archive/adr-0084-task-002-one-release-route.md).
- `src/view_models/workspace/nav.rs`, `breadcrumb.rs`, `tests.rs`.
- `src/view_models/track_detail.rs`, `src/view_models/library.rs` (`ContentListRowDisplay`, `index_feed_selection`), `src/view_models/name_match_page.rs`.
- `src/app.rs`, `src/app/breadcrumb.rs`, `src/app/search_dispatch.rs`, `src/app/name_match_dispatch.rs`.
- `src/library.rs`, `src/library/app_impl.rs`.
- `src/ui/shells/library/sidebar.rs`, `feed_detail.rs`, `playlist_detail.rs`, `content_list.rs`.
- `src/db.rs` (`find_track_id`, `track_row_by_id`).
- `tests/architecture_tests.rs`.

## Checks

```bash
cargo test --lib adr_0084_track_
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
- This packet: `docs/tasks/adr-0084-task-003-one-track-route.md`
- The Implementation Result of `docs/tasks/archive/adr-0084-task-002-one-release-route.md`
- ADR 0084 Decisions 4 and 9
- Each file in "Files To Inspect"

Goal:
- Replace `TrackDetail` and `IndexTrackDetail` with one `Track(TrackKey)` navigation entry and one data-source resolver, as "Design" and "Required Changes" give. A Library track row in the content list opens its track page.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Use the `ReleaseKey` and the resolver method of task 002. Do not make a second method.
- View models carry no GPUI type. The resolver is a pure function. The top app does the database lookup, never the render path.
- Keep each stale-result check of the Index fetch, and the name-match cache.
- Never run `git checkout`, `git restore`, `git stash`, `git reset` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The release route of task 002, except to share its key and its method.
- The database schema and the migration registry.
- Any ADR.
- Each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R3-01 to R3-06 has a passing test.
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
- A track opens from a place that has no item GUID.
- Two Library tracks match one key.
- The Index page needs data that only the old `label` field carried.
- A change needs a file in "Do not touch".

## Implementation Result - 2026-10-11

Each Required Change is done.

### Files Changed

- `src/view_models/workspace/nav.rs`: `FrameNavigationEntry::Track(TrackKey)` replaces
  `TrackDetail(i64)` and `IndexTrackDetail { id, label }`. New types: the `TrackKey` struct
  (`release`, `item_guid`) with its `from_index_track_target` constructor and its
  `index_target` method, the `TrackPageSource` enum (`Library`, `Index`), and the pure
  functions `resolve_track_page_source`, `track_download_updates_open_page`, and
  `track_release_download_updates_open_page`. `display_label` gives the text "Track" for a
  `Track` entry with no known title.
- `src/view_models/workspace/breadcrumb.rs`: the breadcrumb element id of a `Track` entry
  carries its `TrackKey`. A new function, `track_key_id`, builds that id.
- `src/view_models/workspace/mod.rs`: re-exports `TrackKey`, `TrackPageSource`,
  `resolve_track_page_source`, `track_download_updates_open_page`, and
  `track_release_download_updates_open_page`. `default_navigation_entry` builds its placeholder
  track entry with `TrackKey::from_index_track_target`.
- `src/view_models/workspace/tests.rs`: each test that built or read a `TrackDetail` or
  `IndexTrackDetail` entry now builds and reads a `Track(TrackKey)` entry, through a new local
  helper, `track(id)`. New tests cover R3-01, R3-02, R3-03, and the two pure download-update
  functions of R3-06.
- `src/view_models/library.rs`: a new method, `TrackRow::track_key`, gives the `TrackKey` of a
  track row. It uses the feed GUID when the track's feed has one, else the Library feed id. A
  new test module, `track_row_track_key_tests`, covers R3-01 and R3-02 from the track side. A
  new method, `LibraryViewModel::content_list_track_row_key`, gives the `TrackKey` of a
  content-list track row, by its Library track id, through a new free function,
  `track_row_key_in_tree`. A new test, `adr_0084_track_r3_04_content_list_track_row_key_is_typed_by_row_kind`,
  covers R3-04.
- `src/db.rs`: two new read queries, `find_track_id_by_feed_guid_and_item_guid` and
  `find_track_id_by_feed_id_and_item_guid`. A new in-memory database test,
  `adr_0084_track_r3_05_finds_a_track_by_feed_guid_or_feed_id_and_item_guid`, covers R3-05.
- `src/library/app_impl.rs`: `hydrate_detail_from_nav` no longer handles a track entry. The top
  app's resolved source owns that mount, the same way it already owns a release. `navigation_label`
  drops its `TrackDetail` case.
  - A new method, `track_label`, gives the Library track title of a
  track key, the same way `release_label` gives a release title. The removed helper
  `find_track_by_id` had only one caller, now gone. This packet deletes it.
  - `open_content_list_row` adds a branch: a Library track row emits `LibraryAppEvent::OpenPage`
  with a `Track` entry and `NavigationPathStart::CurrentPath`.
- `src/app.rs`: two new fields on `TopApp`.
  - `track_page_source` and `track_page_labels` sit adjacent to their release equivalents.
  - `content_list_frame_title` and the `render_workspace_content` body switch read a track page
    from the stored `track_page_source`, with no database query. The "search or name-match page
    is open" condition is gone: a track page now always renders from its stored source, the same
    way a release page already did.
  - `restore_content_list_nav_entry` calls `restore_track_page_source_for_nav` in place of
    `restore_index_track_detail_for_nav`.
- `src/app/breadcrumb.rs`: a new function, `track_navigation_label`, gives the breadcrumb label
  of a `Track` entry. It reads the Library track title first, then the loaded Index title, then
  the label the opening row gave.
- `src/app/search_dispatch.rs`: the functions that open a track page.
  - `open_track_in_frame` replaces `push_index_track_detail`. It pushes the entry, stores the
    opening label, and resolves the data source. It does not store the label for a
    `LibraryFeed`-origin key, because that key's bare item GUID could collide with an unrelated
    feed's track.
  - A new function, `apply_track_page_source`, resolves the data source and mounts it. The top
    app runs the two new `db` read queries. The render path runs no query.
  - For the Library source, `apply_track_page_source` asks the Library to mount the track row.
  - For the Index source, it clears the Library detail. It skips its own fetch when the
    name-match cache already carries this track. It also skips the fetch when the mounted fetch
    already agrees with the activation target.
  - `restore_track_page_source_for_nav` calls `apply_track_page_source` for a restored `Track`
    entry. For each other entry, it clears the stored source. It replaces
    `restore_index_track_detail_for_nav`.
  - Two new read-only functions give data to these paths: `index_track_loaded_title` and
    `track_opening_label`.
  - `handle_index_track_result_selected` now reads its label from the name-match row too, when
    no search result gives one, then calls `open_track_in_frame`.
  - `handle_search_result_selected`'s Library track branch now calls `open_track_in_frame` in
    place of a direct `select_track` and `push_nav` call.
  - `download_index_track` resolves the open track page again after its download completes,
    through `track_download_updates_open_page`.
  - `show_downloaded_index_feed` also resolves an open track page of the downloaded release,
    through `track_release_download_updates_open_page`. This is the shared completion that
    "Download album" from a track page reaches, the same way it already was for the release
    page.
- `src/ui/shells/library/sidebar.rs`, `feed_detail.rs`, `playlist_detail.rs`: each track row
  opens `FrameNavigationEntry::Track`, built from `TrackRow::track_key`, in place of
  `TrackDetail(track_id)`.
- `src/ui/shells/workspace.rs`: `navigation_entry_for` builds its placeholder track entry with
  `TrackKey::from_index_track_target`, the same way `view_models/workspace/mod.rs` does.
- `src/ui/composites/breadcrumb_trail.rs`: the test fixture builds a `Track` entry in place of a
  `TrackDetail` entry.
- `tests/architecture_tests.rs`: one new situational guard,
  `adr_0084_track_one_route_replaces_the_two_entries` (R3-06). Six existing guards change:
  `adr_0049_inspector_source_ownership_is_guarded`,
  `adr_0024_index_track_detail_uses_rich_track_view_path`,
  `adr_0024_loading_shape_readiness_gate_is_locked`, `nav_top_drives_content_list_body_switch`,
  `adr_0046_forward_restores_pages_through_the_shared_back_restore_path`, and
  `playlist_refresh_and_frame_navigation_preserve_context`. Each changed check names ADR 0084
  Decision 4 in its replacing comment.

### Behavior Changed

- The sidebar, an album track row, a playlist track row, and a content-list track row each open
  a track through the same `Track` entry. A Library search result, an Index search result, and a
  name-match row do this too. The opened track shares one page in the breadcrumb and in the
  history.
- A click on a Library track row in the content list opens its track page. Before this packet,
  that click did nothing.
- A track page renders from its stored data source at all times, not only while a search or a
  name-match page is open. Back to an Index track page, after the search that opened it closes,
  shows that track, not the content list.
- "Download album" on an Index track page changes the page to the Library state, in the same
  place, when the download completes. It adds no new navigation entry. A completed single-track
  download does the same for the track it downloaded.
- Back, Forward, and the breadcrumb select resolve a track through the same function an initial
  open uses. The breadcrumb and the frame title show the word "Track" in place of a bare numeric
  id or the stored row label.

### Checks

| Command | Result |
|---|---|
| `cargo test --lib adr_0084_track_` | Green, 9 tests |
| `cargo test` | Green, 1,875 unit tests and 306 guards |
| `cargo test --test architecture_tests` | Green, 306 tests |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

### Deviations From The Task

1. `TrackKey` and `TrackPageSource` carry `pub(crate)` fields. `TrackKey` has `release` and
   `item_guid`. `TrackPageSource` has `track_id` and `target`. `app/breadcrumb.rs` and
   `app/search_dispatch.rs` read these fields directly. `ReleaseKey`'s own variant data is
   crate-visible the same way. This packet keeps the two keys the same, and adds no getter
   methods.
2. `TrackRow::track_key` sits in `src/view_models/library.rs`, as an `impl TrackRow` block
   beside `AlbumNode::release_key`, not in `nav.rs`. `TrackKey::from_index_track_target` stays in
   `nav.rs`, with no dependency on `db::TrackRow`. This keeps `nav.rs` free of a database
   dependency, the same reason task 002 kept the album-side release key builder out of `nav.rs`.
3. `show_downloaded_index_feed` (`src/app/search_dispatch.rs`) gains a track branch.
   `download_index_album_of_track` has no completion closure of its own. It fetches the album,
   then calls `download_index_feed`, and that function's own completion is
   `show_downloaded_index_feed`. The task named `download_index_album_of_track` as one of the
   "two download completions". This packet treats the completion it reaches as that one
   completion, the same function task 002 already guards for the release page.
4. `resolve_track_page_source` takes `&TrackKey`, not an owned `TrackKey`, unlike
   `resolve_release_page_source`. The caller needs the key again after the call. It pushes the
   navigation entry with it, and stores its opening label with it. A borrow avoids a second
   clone at each call site.
5. This packet does not edit `docs/pending-human-checks.md` or
   `docs/plans/broadcast-chain-delivery-order.md`. AGENTS.md asks for the open visual gate to go
   in both files. This task's own rules name only this packet file as an editable document. The
   open V1 to V5 gate still needs a record in both files.

### Unresolved Concerns

- None found in this packet's scope.

## Operator Visual Check

This check reads and changes your Library. It needs these things: a Linux desktop session, this
checkout, and a configured MusicIndex endpoint. It also needs a Library track in a release
MusicIndex also indexes, and that track in the content list. It also needs a release from
MusicIndex that is not in your Library.

1. Close v4vmm. Make the binary.

   ```bash
   cd /home/citizen/build/v4vmm
   cargo build --bin v4vmm
   ```
2. Open the app.

   ```bash
   target/debug/v4vmm
   ```
3. Open Music.

**V1 - a content-list track row opens its track page**

4. Look at the main content list, not a search result list. Click a track row in it.
   - Correct: the track page opens, with its Library actions and its title.
   - Incorrect: nothing happens, or the content list stays open.

**V2 - one track page for both origins**

5. Click a playlist in the sidebar that holds the same track, then click the track row.
   - Correct: the track page opens, with its Library state.
6. Use the toolbar search to find the same track by name. Open its Index result.
   - Correct: the same page layout opens, still in its Library state.
   - Incorrect: a different layout shows, or the page offers a download action for a track that
     is already in your Library.
6a. On the track page, click the release name. On the release page, click a different track.
   - Correct: the breadcrumb shows the title of each track, each one time.
   - Incorrect: two track segments show the same title.

**V3 - a download moves the track page to the Library state in place**

7. Search for a track whose release is not in your Library. Open its Index result.
   - Correct: the page shows its Index state, with a "Download album" action.
8. Click "Download album". Wait for the download to complete.
   - Correct: the same page updates in place to show the Library state. No navigation happens,
     and no second page appears.
9. Click Back.
   - Correct: the page before the track shows, for example the search results.
   - Incorrect: the track page shows again, or the content area is empty.

**V4 - Back to an Index track page after the search closes**

10. Search for a track that is not in your Library. Open its Index result.
11. Close the search, for example by opening a different page from the sidebar, then click Back
    to the track.
    - Correct: the track page shows again, with its MusicIndex detail, not the content list.
    - Incorrect: the content list shows, not the track.

**V5 - widths, themes and scale**

12. Resize the window to its usual width, then to its narrowest width. Open Settings -> General
    (`Ctrl+Comma`). Select Light, then Dark, then a larger scale. Reopen a track page after each
    change.
    - Correct: the track page, its image, its actions and the breadcrumb lay out with no clipped
      text and no clipped control.
    - Incorrect: a control is clipped, overlaps a different control, or disappears.
13. Close the app.

**Clean-up**

Step 8 downloaded one release into your Library. Keep it, or open its page, choose "Remove
album…" from its "⋯" menu, and complete the removal.

## Orchestrator Review - 2026-10-11

The orchestrator reviewed the diff against ADR 0084, this packet and the [review checklist](../reviews/adr-0084-review-checklist.md). The implementation keeps to the design and to the task 002 method. It does not repeat the two task 002 defects. Each breadcrumb segment uses its own key. Each download completion changes only its own open page, through a tested pure function. Each guard section fails when its function is missing.

One weak point was corrected in review. `content_list_track_row_key` parsed any row id as a Library track id. Today each release row id has the `index-feed:` prefix, so no defect shows. A later release row with a numeric id would open a track page. The new `ContentListPageVm::is_library_track_row` limits the key to Library track rows, and the R3-04 test now proves it (R3-07).

Two visual steps were corrected. Step 5 opens the playlist before its track row. The new step 6a checks the breadcrumb with two tracks in the path.

After the correction, each check is Green: 1,875 unit tests, 306 guards, `fmt`, `clippy -D warnings`, `check --all-targets` with no warning, and `cargo build --bin v4vmm`.

Merge recommendation: pass. This packet fixes flow map finding 2. Packet 004 of the phase plan needs ADR 0081 task 001 first.
