# ADR 0084 Task 001: One Navigation Stack And The Location Breadcrumb

Status: Complete - 2026-10-08. The mechanical checks are Green. The operator passed V1 to V5 on 2026-10-08.

## Goal

The Music content area has one navigation stack. Each page change goes into it: a sidebar click, a row, a link and a search. Back and Forward always show in the frame chrome of Music.

The breadcrumb shows the location, not the history. It shows each page one time at most.

This packet fixes findings 1 and 7 of the [Music flow map](../../architecture/music-flow-map.md).

## Authority

- [ADR 0084](../../adr/0084-music-column-browser.md) Decision 9 and its Verification section.
- [ADR 0046](../../adr/0046-workspace-frame-architecture.md) Architectural Invariant 2, as ADR 0084 amends it: the frame chrome owns Back, Forward and history.
- The durable set in [AGENTS.md](../../../AGENTS.md): renderer portability, typed action state, and "Current-view state must update in place".

## Recorded Facts - 2026-10-08

- `TopApp.workspace_layout` in `src/app.rs` drives the Back and Forward buttons and the breadcrumb above the content frame.
- `LibraryApp.workspace_layout` in `src/library.rs` is a second copy of the layout. Only the Library writes to it. `select_track` resets it, and `select_playlist` and `select_playlist_track` push into it.
- The sidebar calls `select_artist`, `select_album`, `select_track` and `select_playlist` directly. These calls do not change the top stack, so the chrome shows no Back, no Forward and no breadcrumb.
- `track_breadcrumb_display` in `src/library/app_impl.rs` projects the Library stack into a second breadcrumb on the track page. `select_frame_breadcrumb` handles it.
- `FrameNavigationState` in `src/view_models/workspace/nav.rs` keeps one list of entries. The breadcrumb shows that list, so the breadcrumb is the history.
- `hydrate_detail_from_nav` does not restore `PlaylistDetail`.
- `frame_shell` in `src/ui/composites/frame_shell.rs` hides Back and Forward when they are unavailable.

## Design

`FrameNavigationState` keeps a history of locations. A location is a path: the destination first, then the pages inside it, and the current page last.

| Step | New location | History |
|---|---|---|
| A sidebar click | The Library root, then the page | A new entry. Forward history clears |
| A row, a link, or a breadcrumb segment | The current path with the page added. When the path holds the page, the path stops at that page | A new entry. Forward history clears |
| A search | The search only | A new entry. A new query on the search page replaces the query in place |
| Back or Forward | The location of that history entry | Moves one entry |

The destination of a sidebar path is the Library root, with the label "Library". Packet 3 of Phase 4A replaces it with the destinations of Decision 2.

The Library keeps no navigation state. It asks the top app to open a page with an event. The top app changes the stack, then restores the page through the shared restore function that Back and Forward use.

## Required Changes

1. Change `FrameNavigationState` into a history of locations, as the Design table gives. Add `start_path` for a step that starts a new path. A push of a page that the path holds cuts the path at that page.
2. Delete `pop_nav_until`. A breadcrumb selection is a push of a page that the path holds.
3. Delete `LibraryApp.workspace_layout`, `FrameHistoryMode`, `track_breadcrumb_display` and `select_frame_breadcrumb`. Delete the breadcrumb of the track page.
4. Add the event `LibraryAppEvent::OpenPage`, with the entry and a typed path start: a new path or the current path. The sidebar, the album track rows and the playlist track rows send it. The top app handles it.
5. `hydrate_detail_from_nav` restores `PlaylistDetail`.
6. The top app labels each breadcrumb segment with the names that the Library has: the playlist name, the track title, the album name and the artist name.
7. Back and Forward always show in the Music content frame. Each is unavailable when its history is empty. Other frames keep their present behavior.
8. The breadcrumb shows when the location path has more than one page.
9. "Open in Music" from the readiness card of Show starts a new path.
10. Add a situational guard for ADR 0084 Decision 9. Delete the guard requirements that assert the second stack, in the same change.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_nav_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| N1-01 | The navigation model gives a location path that holds each page one time at most, after a sequence that opens a page two times |
| N1-02 | A push of a page that the path holds cuts the path at that page, and Back then returns to the longer path |
| N1-03 | `start_path` gives a path with the destination and the page only, and Back returns to the earlier location |
| N1-04 | A push after a Back clears the forward history |
| N1-05 | A new search query on the search page replaces the query in place. A search from another page adds one history entry |
| N1-06 | The frame chrome display exposes Back and Forward as always shown for the Music content frame, each with its typed availability |
| N1-07 | The breadcrumb display projects the location path, not the history |
| N1-08 | A guard blocks navigation state and a breadcrumb in the Library screen modules, and names ADR 0084 Decision 9 |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: a sidebar click on a playlist shows Back, Forward and the breadcrumb "Library › playlist name".
- V2: a track row of that playlist adds the track to the breadcrumb. Back returns to the playlist page, with the same tracks.
- V3: the track page has no breadcrumb of its own.
- V4: a page that opens two times shows in the breadcrumb one time only.
- V5: Back and Forward show at normal and narrow widths, in Light and Dark, with no clipped icon.

## Exclusions

- No destinations sidebar, no Discover and no new columns. Packet 3 adds them.
- No change to the Index page routes. Packet 2 removes them.
- No new key binding. Task 015 owns the keys.
- No change to the persisted layout format.

## Files To Inspect

- [Agent rules](../../../AGENTS.md) and the [source map](../../architecture/source-map.md).
- `src/view_models/workspace/nav.rs`, `mod.rs`, `chrome.rs`, `breadcrumb.rs`, `tests.rs`.
- `src/ui/composites/frame_shell.rs`, `src/ui/shells/workspace.rs`.
- `src/app.rs`, `src/app/breadcrumb.rs`, `src/app/search_dispatch.rs`, `src/app/show.rs`.
- `src/library.rs`, `src/library/app_impl.rs`.
- `src/ui/shells/library/sidebar.rs`, `feed_detail.rs`, `playlist_detail.rs`, `track_detail.rs`, `detail.rs`.
- `tests/architecture_tests.rs`.

## Checks

```bash
cargo test --lib adr_0084_nav_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Implementation Result - 2026-10-08

Each Required Change is done.

### Files Changed

- `src/view_models/workspace/nav.rs`: `FrameNavigationState` keeps a history of locations. A new `FrameLocation` holds the path. New items: `start_path`, `has_location_path` and the enum `NavigationPathStart`. `back_destination` and `has_history` are deleted, because no production path calls them.
- `src/view_models/workspace/mod.rs`: new `start_nav_path`. `pop_nav_until` is deleted. `default_content_frame_id` is test-only, because the Library was its one production caller.
- `src/view_models/workspace/chrome.rs`: new enum `FrameHistoryControls`, and the methods `with_history_controls_always_shown`, `back_shown` and `forward_shown`.
- `src/ui/composites/frame_shell.rs`: Back and Forward show from `back_shown` and `forward_shown`.
- `src/ui/shells/workspace.rs`: a new slot shows Back and Forward at all times on the content-list frame. The breadcrumb shows when the location path has more than one page.
- `src/app.rs`: new `open_music_page`, which handles `LibraryAppEvent::OpenPage`. The Music mount sets the new slot.
- `src/app/breadcrumb.rs`: a breadcrumb selection is a push, and it restores the page through `restore_content_list_nav_entry`. The Library gives the labels of playlist, track and album pages.
- `src/app/show.rs`: the readiness list from Show starts a new path.
- `src/library.rs`, `src/library/app_impl.rs`: the Library stack, `FrameHistoryMode`, `select_playlist_track`, `track_breadcrumb_display`, `select_frame_breadcrumb` and `frame_breadcrumb_label` are deleted. New: `open_page`, `navigation_label`, and the `PlaylistDetail` arm of `hydrate_detail_from_nav`.
- `src/ui/shells/library/sidebar.rs`, `feed_detail.rs`, `playlist_detail.rs`: each click sends `OpenPage`.
- `src/ui/shells/library/track_detail.rs`, `detail.rs`: the track page breadcrumb is deleted.
- `src/view_models/workspace/tests.rs`: seven `adr_0084_nav_` tests. The tests of `pop_nav_until`, `has_history` and `back_destination` are deleted with those methods.
- `tests/architecture_tests.rs`: the new situational guard `adr_0084_nav_library_screens_keep_no_navigation_stack`. Nine guards change. Each one asserted the second stack or the track page breadcrumb, which ADR 0084 Decision 9 replaces.

### Behavior Changed

- A sidebar click on an artist, an album, a track or a playlist adds one history entry. Back and Forward record it.
- Back to a playlist page shows that playlist again.
- The breadcrumb shows the location. A page that opens two times shows in it one time.
- The track page has no breadcrumb of its own.
- Back and Forward always show in Music. An unavailable control shows as disabled.
- A search from a detail page of an earlier search starts a new path. Back returns to that detail page. Before this change, the new search removed the earlier search from the history.
- After a download on a track page, the mounted playlist actor refreshes its rows. Before this change, it refreshed only when the track page opened from that playlist.

### Checks

| Command | Result |
|---|---|
| `cargo test --lib adr_0084_nav_` | Green, 7 tests |
| `cargo test` | Green, 1,857 unit tests and 304 guards |
| `cargo test --test architecture_tests` | Green, 304 tests |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

A probe line with `WorkspaceLayout` in `src/library.rs` made the new guard fail with its ADR 0084 message. The probe line is removed.

### Deviations From The Task

1. An album in the sidebar without a feed id has no navigation entry. It mounts as before and adds no history entry. Packet 2 gives each release one page by its feed GUID.

## Operator Visual Check

This check reads the app only. It writes no tag and no file. It needs a Linux desktop session, this checkout, and a Library with one playlist of two or more tracks.

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

**V1 - a sidebar click shows Back, Forward and the breadcrumb**

4. Look at the content frame header before a click.
   - Correct: Back and Forward show, each as unavailable.
   - Incorrect: no Back or no Forward shows.
5. Click a playlist in the sidebar.
   - Correct: Back is available. The breadcrumb shows "Library › playlist name".
   - Incorrect: Back is unavailable, or no breadcrumb shows.

**V2 - a track row adds to the path, and Back returns to the playlist**

6. Click a track row of the playlist.
   - Correct: the breadcrumb shows "Library › playlist name › track title".
7. Click Back.
   - Correct: the playlist page shows again, with the same tracks.
   - Incorrect: a different page shows, or the page is empty.
8. Click Forward.
   - Correct: the track page shows again.

**V3 - the track page has no breadcrumb of its own**

9. On the track page, look below the frame header.
   - Correct: one breadcrumb only, in the frame chrome.
   - Incorrect: a second breadcrumb shows above the track title.

**V4 - a page shows in the breadcrumb one time**

10. On the track page, click the album name link. Then click the same track in the album track list.
    - Correct: the breadcrumb shows "Library › playlist name › track title". The album is not in it.
    - Incorrect: the playlist or the track shows two times.
11. Click "Library" in the breadcrumb, then click Back.
    - Correct: the track page shows again.

**V5 - widths, themes and scale**

12. Resize the window to its usual width, then to its narrowest width. Open Settings -> General (`Ctrl+Comma`). Select Light, then Dark, then a larger scale. Go back to Music after each change.
    - Correct: Back, Forward and the breadcrumb stay in view, with no clipped icon. An unavailable control looks disabled.
    - Incorrect: a control is clipped, overlaps a different control, or disappears.
13. Close the app.

**Clean-up**

None. This check reads your library. It creates no fixture, and it writes no file.
