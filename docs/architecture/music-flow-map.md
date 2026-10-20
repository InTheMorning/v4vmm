# Music Flow Map - 2026-10-20

## Status

Current - 2026-10-20, from commit `4456deb`. This map is advisory. It states no rule.
It is step 1 of [Phase 4A](../plans/design-and-cleanup-overhaul-plan.md#phase-4a-flow-and-information-architecture) of the overhaul plan.

A read-only agent wrote this map from the code at commit `4456deb`. It replaces the map of
2026-10-08, which an earlier agent wrote from commit `204382e`. The operator has not walked
this map yet. No finding below carries an operator check.

## Step-Count Method

Each count in a Job section is the number of clicks and key presses from a stated starting
point. A typed phrase counts as one step, whatever its length. A choice from an open popover
or menu counts as one more step. A wait for a background command counts as zero steps, because
it is a wait, not an action by the curator. A scroll counts as a step only when the target row
is not visible yet. Each case names its starting point and its assumptions, so the old count
and the new count compare the same situation.

## Structural Findings

1. **Two navigation stacks. Fixed by packet 001.** One `WorkspaceLayout` and one
   `FrameNavigationState`, per content frame, now carry every Music page change
   (`src/view_models/workspace/mod.rs`, `src/view_models/workspace/nav.rs`). `LibraryApp` sends
   each page change as a `LibraryAppEvent::OpenPage`, and `TopApp::open_music_page`
   (`src/app.rs`) pushes it onto that one stack. A guard in `tests/architecture_tests.rs`,
   `adr_0084_nav_library_screens_keep_no_navigation_stack`, blocks a Library screen module from
   holding `WorkspaceLayout`, `FrameNavigationState`, a breadcrumb type, `push_nav`, or
   `reset_nav`.
2. **The Library track row did nothing. Fixed by packet 003.** `open_content_list_row`
   (`src/library/app_impl.rs`) now calls `content_list_track_row_key` for a Library track row,
   and emits `LibraryAppEvent::OpenPage` with `FrameNavigationEntry::Track`. The row opens the
   track page, the same as an Index row opens the release page.
3. **No list has multiple selection. Changed.** Packet 009 gives the Releases, Tracks, and
   Needs attention lists a checkbox selection mode. It adds Ctrl+click, Shift+click, Ctrl+A,
   Space, and an action bar (`src/library/app_impl.rs`, `header_action_for_list_column`,
   `releases_selection_action_bar`, `run_releases_selection_action`). Packet 018 gives the same
   model to the detail column, for a release page, a singles group page, and a feed page
   (`src/app/detail_selection.rs`, `src/library/detail_selection.rs`). The playlist page's track
   list, the Discover tiles, and search results still have one click target only, pending
   packets 020, 012, and 010.
4. **Play is only on the playlist page. Open.** No change. [ADR 0068](../adr/0068-show-cue-and-audition-isolation.md)
   stays Proposed. ADR 0084 Decision 11 keeps a track row's audition position empty until it
   lands.
5. **Readiness opened only from Show. Fixed by packet 005.** "Needs attention" is now a sidebar
   destination (`MusicDestination::NeedsAttention`, `src/view_models/workspace/nav.rs`).
   `open_needs_attention_destination` (`src/library/app_impl.rs`) reaches the readiness report
   with no Show step. Show's own `open_broadcast_readiness_in_music`
   (`src/app/show.rs`) is now one entry point among several, not the only one.
6. **The publisher has no search entry. Open, no change.** Packet 010 is not written yet. It
   waits for Stophammer request 8. `FrameNavigationEntry::PublisherDetail` builds only from a
   destination-list row, a name link, or a breadcrumb or history restore
   (`src/library/app_impl.rs`, `open_destination_row`, `open_publisher_page`). No function in
   `src/app/search_dispatch.rs` builds a `PublisherDetail` entry.
7. **The track page breadcrumb was a history, not a location. Fixed by packet 001.** The track
   page keeps no breadcrumb of its own. The frame chrome shows the one location breadcrumb
   (`src/app/breadcrumb.rs`). `FrameLocation::with_page` (`src/view_models/workspace/nav.rs`)
   cuts the path at a page the path already holds, so a path never repeats a page.
8. **The Artists list already splits by identity, but only one side is new.** A row for a
   confirmed or unverified artist feed opens the feed-level page, which is the publisher page
   reused (`FrameNavigationEntry::PublisherDetail`, `artist_list_row` in
   `src/view_models/library.rs`). A row with no artist feed, marked "by name", still opens the
   pre-ADR-0084 Library artist page by name (`FrameNavigationEntry::ArtistDetail`, same
   function). ADR 0084 Decision 7's name-level page, which packet 017 owns, is not written.
9. **Bulk selection reaches only Library-known lists.** The action bars of packets 009 and 018
   give a batch download and a batch "Add to playlist". They cover the Releases list, a
   release's track list, a singles group's release list, and a feed page's release list. Each
   of these reads the Library tree only (`releases_list`, `src/view_models/library.rs`). Discover's tiles,
   search results, and the playlist page carry no selection model yet, so a never-subscribed
   Index item still downloads one at a time.
10. **A singles group page has its own one-click download, apart from the selection bar.** Its
    main action, "Download n singles", downloads every single of the group the Library does
    not yet have. One click runs it (`main_action` in `src/view_models/singles_group.rs`, and
    `download_singles` in `src/library/app_impl.rs`). This action needs no selection mode.
11. **A download from the Index now updates the open page in place.** `download_feed`
    (`src/library/app_impl.rs`, a Library-opened page) and `download_index_feed`
    (`src/app/search_dispatch.rs`, an Index-opened page) are still two functions. The Index one
    now calls `show_downloaded_index_feed`, which checks `release_download_updates_open_page`
    (`src/view_models/workspace/nav.rs`) before it changes anything. Both paths leave the
    curator on the same page, with no navigation to a different route.
12. **"+Playlist" and "Add to playlist" are still two labels for one action.** "+Playlist"
    marks an inline track row (`src/view_models/library.rs`, `src/app/search_dispatch.rs`).
    "Add to playlist" marks a page's own button
    (`src/ui/shells/library/track_detail_metadata.rs`, `src/ui/shells/library/feed_detail.rs`).
13. **`ArtistVm` and `render_artist_view` still have no caller.** No file outside
    `src/ui/shells/artist.rs` and its own tests calls `render_artist_view`. The live artist
    page renders through the separate `LibraryArtistDetailVm` and `render_artist_detail_shell`.
14. **The "Play" action of `TrackActionState` still has no confirmed render path.**
    `TrackActionState::actions` (`src/view_models/entity_detail.rs`) always appends an
    `EntityActionKind::Play` action. No file in `src/` matches a click handler for it. The one
    live row-action builder for an Index track row, `index_track_row_actions`
    (`src/app/search_dispatch.rs`), does not call `TrackActionState::actions` at all.
15. **Discover removes the old two-step "Browse".** Music's default navigation is
    `Destination(Discover)` with the child page `DiscoverView::NewReleases`
    (`default_navigation_child`, `src/view_models/workspace/mod.rs`). The curator reaches Index
    content with no search and no filter click, where the 2026-10-08 code needed a filter click
    first.
16. **Finding a known artist's new Index material by name, without search, is still not
    possible.** The Artists list reads the Library tree only. Discover's one view, "New
    releases", has no artist grouping and no filter. A name search still opens the name-match
    detour the 2026-10-08 map recorded (`FrameNavigationEntry::IndexNameMatches`), unchanged.
17. **The keyboard reaches every step of the new selection model.** The same key actions (Up,
    Down, Page Up, Page Down, Home, End, Space, Ctrl+A, Enter, Escape) dispatch to both the
    list column and the detail column from one place (`src/app/keyboard.rs`,
    `move_music_selection_focus` and its neighbors). Each column acts only while it holds the
    keyboard focus. Space and Ctrl+A also start selection mode, so a mouse click on "Select" is
    never required.
18. **Playback from a playlist still starts an RSS check with no extra click.** The
    `LibraryAppEvent::PlayPlaylistAt` handler in `src/app.rs` calls `check_playlist_rss` with
    `RssCheckTrigger::PlaybackStart` right after it starts playback.

## Job 1: Find New Music

- **Discover.** Music opens on the "Discover" destination, showing the "New releases" view:
  the Index releases, newest first, with no search and no filter step
  (`default_navigation_child` in `src/view_models/workspace/mod.rs`, and
  `render_discover_new_releases_detail` in `src/ui/shells/library/content_list.rs`). A row
  opens its release page in place.
- **Search.** Type in the toolbar search, then press Enter
  (`src/app/search_dispatch.rs`). Results show the filter All, Library, or Index, and the tabs
  Artists, Feeds, and Tracks, unchanged since 2026-10-08.
  - An Index artist row still opens the name-match page, not an artist page
    (`handle_index_artist_result_selected`, `open_name_match_page`,
    `src/app/search_dispatch.rs`).
  - A feed row opens the release page (`FrameNavigationEntry::Release`). A track row opens the
    track page (`FrameNavigationEntry::Track`). Both resolve to the same page shape as a
    Library row, by ADR 0084 Decision 4.
- **Artists and releases.** The "Releases" and "Artists" destinations list Library items only.
  A singles group row (two or more singles of one artist) opens its own group page in place
  (`click_singles_group_row`, `open_singles_group`, `src/library/app_impl.rs`).
- **Step count.** The case: Music is open, nothing is selected, and the curator opens one
  Index release from the default view, with no search. The step-count method is stated above.
  - 2026-10-08: 2 steps. Set the content-list filter to Index. Click a row to open it.
  - 2026-10-20: 1 step. Click a row in Discover's "New releases" view, already open.
- Dead ends:
  - The Index track page still has no "download one track" action and no "Add to playlist"
    action (`page_actions`, `src/view_models/track_detail.rs`: the Index branch returns an
    empty `plain` list). The track row of the release page still has both.
  - The name-match page still shows its "more tracks" count as text, with no load-more control
    (`src/view_models/name_match_page.rs`). `src/ui/shells/name_match_page.rs` renders that
    text. Packet 017, which owns Decision 7's name level, is not written.
- The operator must know the origin:
  - A name link on a release page opens the feed-level page, the publisher page reused, for a
    confirmed or unverified artist feed. For every other artist text, the same link opens the
    pre-ADR-0084 Library page by name (finding 8). The two pages look different. Only the
    feed-level page carries a confirmation level.
  - A Library artist search result and an Index artist search result still open two different
    page types (`ArtistDetail` and `IndexNameMatches`), unchanged since 2026-10-08.
  - The publisher page no longer has two layouts by origin. ADR 0085 Decision 4 gives it one
    section set and one action list, whatever the page's context.
  - Finding a specific known artist's new Index material, without typing a search, is still
    not possible (finding 16).

## Job 2: Take Music In

- **Download a release.** `download_feed` (`src/library/app_impl.rs`) downloads a release from
  a Library-opened page: the Releases list, a singles group page, or the feed page.
  `download_index_feed` (`src/app/search_dispatch.rs`) downloads a release from an Index-opened
  page. Both run `SubscribeFeed`. Both now leave the open page on the same page, because
  `show_downloaded_index_feed` checks `release_download_updates_open_page` before it changes
  anything (finding 11).
- **Download several releases or tracks at once.** On a release page, a singles group page, or
  a feed page, "Select" starts the selection mode of packet 018. Each checked row's release or
  track downloads in one "Download" click (`download_detail_selection`,
  `src/app/detail_selection.rs`). The Releases destination list has the same bar
  (`run_releases_selection_action`, `src/library/app_impl.rs`). The Tracks and Needs attention
  lists show no "Download" action, because every row there is already in the Library.
- **The singles group's own action.** A group page's main action, "Download n singles",
  downloads every single the Library does not yet have, in one click, with no selection step
  (finding 10).
- **Download a track.** From a track row of a release page, from the track page itself, or
  through the bulk actions above.
- **Confirm the tags.** A scan still runs after a download, a feed check, an update, a playlist
  check, and the end of playback (`scan_tag_updates`, `src/library/app_impl.rs`). When a file
  differs, the sidebar still shows "Update n files". Its dialog still lists each file and
  frame, and "Write Tags" still writes them.
- **MusicBrainz** still has two scopes: one track, from the track page or its menu
  (`toggle_musicbrainz_lookup`, `src/library/app_impl.rs`), and the downloaded tracks of a
  release, from the album page menu (`musicbrainz_feed`, same file). Unchanged since
  2026-10-08.
- **Step count.** The case: download 5 singles of one artist, already tracked by the Library,
  but not yet downloaded. The step-count method is stated above.
  - 2026-10-08: about 12 steps. Type the artist name, then press Enter. Open and download
    each of the 5 singles, one at a time, at 2 steps each.
  - 2026-10-20: 3 steps. Open "Releases" in the sidebar. Click the artist's singles group row.
    Click "Download 5 singles".
  - This count assumes the 5 singles are already subscribed, with no downloaded files, the
    case finding 9 records. A curator who must first find and subscribe 5 never-seen Index
    singles still repeats an open-and-download step for each one. Discover and search carry no
    selection model yet.
- Dead ends:
  - "Write Tags" writes files, and its button still has no destructive style
    (`src/ui/shells/tag_update_confirmation.rs`: `destructive: false`, confirmed by its own
    test). Unchanged since 2026-10-08.
  - The row action "repair a broadcast route" still repairs the payment routes of a track, not
    a file path (`RepairPaymentRoutesForTrack`, `repair_broadcast_routes_for_track`,
    `src/library/app_impl.rs`). Unchanged.

## Job 3: Build A Show Playlist

1. Create a playlist with `cmd-n` (`ctrl-n` on Linux), or the inline "+" control of the
   Playlists destination. The key opens the Playlists destination and an inline name field
   (`handle_new_playlist` in `src/app/keyboard.rs`, and `begin_new_playlist` and
   `create_playlist` in `src/library/app_impl.rs`).
2. Add a Library track or album with "Add to playlist", on the track page or the album page
   (`add_track_to_playlist`, `add_album_to_playlist`, `src/library/app_impl.rs`). Add several
   Library tracks or releases of a release at once by starting selection mode on the release
   page, checking each row, then "Add to playlist" once in the action bar
   (`add_detail_selection_to_playlist`, `src/app/detail_selection.rs`).
3. Add an Index track with the same "Add to playlist" control on an Index-opened track page.
   `SubscribeThenAppendToPlaylist` downloads it first, with no separate "Download and add"
   label. A Library track never downloads through this control. Unchanged since 2026-10-08.
4. Put the tracks in order with drag, or with "Move up" and "Move down"
   (`move_playlist_track`, `src/library/app_impl.rs`). Packet 009's selection model replaced
   `move_up` and `move_down` for the Releases and Tracks lists. The playlist page still uses
   its own reorder path, because packet 020 is not written.
5. Confirm the playlist with "Check RSS" (`check_playlist_rss`, `src/library/app_impl.rs`). The
   report shows stale feeds, changed fields, and tracks removed from their feed. Playback from
   the playlist still starts the same check with no extra click (finding 18).

- **Step count.** The case: add 3 tracks of one release to an existing playlist named "000".
  The step-count method is stated above.
  - By the unchanged per-track control, on 2026-10-08 and on 2026-10-20: 6 steps. Click "Add
    to playlist", then click "000" in the popover, for each of the 3 tracks.
  - By the new bulk path, only on 2026-10-20: 5 steps. Click "Select". Click the first
    track's row. Shift+click the third track's row. Click "Add to playlist", then "000".
  - The bulk path saves more steps as the track count grows past 3. It never costs more than
    the old per-track control.
- Dead ends:
  - "Delete playlist" still has no confirmation (`delete_playlist`,
    `src/library/app_impl.rs`). "Remove from all playlists" still has one
    (`request_remove_track_from_all_playlists`, same file). Unchanged since 2026-10-08.
  - The old failure to show a playlist again after Back from one of its track pages is fixed
    by finding 7's single location breadcrumb: a page the path already holds cuts the path at
    that page, so the playlist stays reachable.
- The operator must know the origin: an Index track still downloads when it goes into a
  playlist. A Library track still does not. The control still looks the same in both places.

## Job 4: Keep The Library Ready

1. "Check all feeds" in the sidebar still runs `CheckFeedsAndRepairRoutes`
   (`check_all_feeds`, `src/library/app_impl.rs`). When feeds changed, the button still becomes
   "Apply updates (n)" (`apply_all_feed_updates`, same file).
2. "Needs attention" is now a sidebar destination (finding 5). It opens the readiness list
   directly, through `open_needs_attention_destination`, with no Show step.
3. A readiness-list row can still repair payment routes
   (`repair_broadcast_routes_for_track`), or confirm a track that its feed removed
   (`confirm_removed_track`). A third action, removing the track from the Library, is also
   present.

- **Step count.** The case: run "Check all feeds", apply the updates it finds, open "Needs
  attention", and repair one track's payment route. The step-count method is stated above.
  - 2026-10-08: 6 steps. Click "Check all feeds", then "Apply updates (n)". Switch to the
    Show tab, open readiness from Show, open the row's action, then click "Repair broadcast
    route".
  - 2026-10-20: 5 steps. Click "Check all feeds", then "Apply updates (n)". Click "Needs
    attention" in the sidebar, open the row's action, then click "Repair payment route".
  - The step saved is the Show detour that finding 5 removed.

## Entry Points For Each Page

| Page | Navigation entry | How the operator reaches it |
|---|---|---|
| Discover, "New releases" | `DiscoverView(NewReleases)` | Music's default view, the "Discover" destination |
| Releases list | `Destination(Releases)` | Sidebar "Releases" |
| Artists list | `Destination(Artists)` | Sidebar "Artists" |
| Tracks list | `Destination(Tracks)` | Sidebar "Tracks" |
| Needs attention (readiness) list | `Destination(NeedsAttention)` | Sidebar "Needs attention", also from Show |
| Playlists list | `Destination(Playlists)` | Sidebar "Playlists" |
| Release page, Library or Index source | `Release(key)` | A Releases row, a singles group row, a "New releases" row, a search feed or track result, a name link |
| Track page, Library or Index source | `Track(key)` | A Tracks row, a release page's track row, a playlist track row, a search track result, a name-match row |
| Singles group page | `SinglesGroup(key)` | A singles group row's body, a single page's "All n singles" link |
| Library artist page by name | `ArtistDetail(name)` | An Artists row marked "by name", a Library artist search result, a name link with no confirmed feed |
| Feed-level artist page, the publisher page reused | `PublisherDetail(feed_guid)` | An Artists row with a feed identity, a name link to a confirmed or unverified artist feed, a breadcrumb restore |
| Playlist page | `PlaylistDetail(id)` | A Playlists row only |
| Search results | `Search(query)` | The toolbar search |
| Name-match page | `IndexNameMatches(name)` | An Index artist search result, a name link with no feed, from the Index side |
| Settings | `Settings` | The Settings tab |

## Sidebar, List, And Detail Columns

- Music shows three columns (ADR 0084 Decision 1): the sidebar, the list, and the detail
  column. Each one that scrolls shows its own scrollbar. At a narrow width the sidebar
  collapses to icons.
- The sidebar holds, from top to bottom:
  - the six destinations, in order: Discover, Releases, Artists, Tracks, Needs attention,
    Playlists
  - the feed status line, "Check all feeds" or "Apply updates (n)", and "Update n files"
  - "Playlists" with sort and "+", when the Playlists destination is open
- The list column shows the items of the open destination, or Discover's views. Only the
  Tracks destination keeps a Tiles-or-List view-mode control on the list column. Discover's
  "New releases" keeps its own view-mode control in its own header, moved there by Decision 3.
  The Releases, Artists, and Playlists lists read straight from the Library tree, with no
  filter control of their own.
- The detail column shows the open page: a release, a track, an artist, a playlist, or
  search results. Back, Forward, and the one location breadcrumb show in its chrome, and in
  the list column's chrome at the same time.

## Duplicates

- Two controls for one filter: fixed by packet 006. The content list's three-state cycling
  filter is deleted. Only the search-results chips remain.
- Two download functions for one release: changed. `download_feed` and `download_index_feed`
  still exist, but both now leave the curator on the same open page (finding 11).
- Two labels for one playlist control: open, unchanged. "Add to playlist" and "+Playlist" are
  still two labels for the same action, now split by context (finding 12).
- Two MusicBrainz scopes with similar labels: open, unchanged.
- Code that no path reaches: `ArtistVm` with `render_artist_view` (finding 13), and the "Play"
  action of `TrackActionState` (finding 14).
- A new duplicate: two artist page types by identity, not by origin. A confirmed-feed artist
  and a name-only artist still open two different pages (finding 8).

## Not Confirmed

- Resolved since 2026-10-08: which control calls `check_feed_on_view` on the album page. No
  control does. `select_album` (`src/library/app_impl.rs`) calls it automatically whenever a
  Library album page opens.
- Resolved since 2026-10-08: whether any path still pushes `QueueNowPlaying` in Music. No path
  found does. The entry exists only in the navigation enum, a default filler, and a breadcrumb
  label.
- The keyboard Tab order across the sidebar, the list column header, and the detail column
  needs a person at a keyboard. The session must use no mouse at all. Each control has a focus
  handle, but this map cannot walk their order from the code alone.
- The exact number of singles a real singles group shows, and the exact feed count a real
  "Check all feeds" run reports, are runtime facts. This map cannot confirm them from the code
  alone.
