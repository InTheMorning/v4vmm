# Music Flow Map - 2026-10-08

## Status

Current - 2026-10-08. This map is advisory. It states no rule.
It is step 1 of [Phase 4A](../plans/design-and-cleanup-overhaul-plan.md#phase-4a-flow-and-information-architecture) of the overhaul plan.

A read-only agent wrote it from the code at commit `204382e` and later. The orchestrator checked the main claims against the code. The operator has not confirmed it in a walkthrough yet.

## Structural Findings

1. **Two navigation stacks.** `TopApp.workspace_layout` in `src/app.rs` drives the Back and Forward buttons and the breadcrumb above the content frame. `LibraryApp.workspace_layout` in `src/library.rs` drives the Library page on screen.
   - A click in the sidebar tree, or on a track row of an album page or a playlist page, changes only the Library stack. Back and Forward do not record these steps.
   - Search, Index pages, publisher pages and name links change the top stack.
   - `hydrate_detail_from_nav` in `src/library/app_impl.rs` does not restore `PlaylistDetail`. Back to a playlist page can show a different page.
2. **A content list row always opens the Index album page.** `open_content_list_row` emits `OpenIndexFeedDetail` for each row, also for an album that is in the Library. The Library album page opens only from the sidebar tree, a search result of the Library, or a name link.
3. **No list has multiple selection.** Each list in Music has one click target for each row and one selected item at most. Each row action works on one row.
4. **Play is only on the playlist page.** Its button plays through the one shared playback session, the same session that the Show transport controls. No page has an audition route.
5. **Readiness opens only from Show.** `open_broadcast_readiness_in_music` in `src/app/show.rs` is the only entry to the list of tracks that need attention. No control in Music opens it.
6. **The publisher has no search entry.** A publisher page opens only from a name link on an album or a track that states a publisher relationship.

## Job 1: Find New Music

- **Search.** Type in the toolbar search, then press Enter. Search results show the filter All, Library or Index, and the tabs Artists, Feeds and Tracks.
  - An Index artist row opens the name match page, not an artist page.
  - A feed row opens the Index album page. A track row opens the Index track page.
- **Browse.** Open Music with no sidebar selection, and set the filter control to Index. The content list shows the recent MusicIndex feeds and loads more at the bottom.
- Dead ends:
  - The Index track page has no download of one track and no "Add to playlist". The track row on the Index album page has both.
  - The name match page shows its "more tracks" count as text, with no control to load more.
- The operator must know the origin:
  - An artist search result opens one of two page types, by its origin.
  - The publisher page has two layouts, "Library Albums" and "Other Albums", or "Owned Albums" and "Listed By", by the page that opened it.

## Job 2: Take Music In

- **Download an album** from the Library album page (`download_feed`) or the Index album page (`download_index_feed`). Both run `SubscribeFeed`. After a download from the Index, the page changes to the Library album page in place.
- **Download a track** from the track row of an album page, or from the Library track page.
- **Confirm the tags.** A scan runs after a download, a feed check, an update, a playlist check and the end of playback. When a file differs, the sidebar shows "Update n files". Its dialog lists each file and frame, and "Write Tags" writes them.
- **MusicBrainz** has two scopes: one track on the Library track page, and the downloaded tracks of an album in the album page menu.
- Dead ends:
  - "Write Tags" writes files, but its button does not have the destructive style.
  - The row action "repair a broadcast route" repairs the payment routes of a track, not a file path.

## Job 3: Build A Show Playlist

1. Create a playlist with "+" next to "Playlists" in the sidebar, or with `cmd-n`.
2. Add a Library track or album with "Add to playlist".
3. Add an Index track with the playlist control on a track row of the Index album page. This downloads the track. No path adds an Index track without a download.
4. Put the tracks in order with drag, or with "Move up" and "Move down".
5. Confirm the playlist with "Check RSS". The report shows stale feeds, changed fields and tracks removed from their feed. Playback from the playlist also starts a check.

- Dead ends:
  - "Delete playlist" has no confirmation. "Remove from all playlists" has one.
  - Back from a track page that opened from a playlist can fail to show the playlist again (finding 1). The breadcrumb on the track page works.
- The operator must know the origin: an Index track downloads when it goes into a playlist. A Library track does not. The control looks the same in both places.

## Job 4: Keep The Library Ready

1. "Check all feeds" in the sidebar runs `CheckFeedsAndRepairRoutes`. When feeds changed, the button becomes "Apply updates (n)".
2. A content list row of the readiness list can repair payment routes, or confirm a track that its feed removed.
3. The readiness list opens only from Show (finding 5).

## Entry Points For Each Page

| Page | Navigation entry | How the operator reaches it |
|---|---|---|
| Library root, the content list | `SourceList` | Music with no sidebar selection |
| Playlist page | `PlaylistDetail` | A sidebar playlist row only |
| Library track page | `TrackDetail` | A sidebar track row, an album or playlist track row, a Library search result |
| Library album page | `AlbumDetail` | A sidebar album row, a Library search result, an album name link, the end of an Index download |
| Library artist page | `ArtistDetail` | A sidebar artist row, a Library artist search result, an artist name link of a Library album |
| Search results | `Search` | The toolbar search |
| Name match page | `IndexNameMatches` | An Index artist search result, an artist name link of an Index album |
| Index album page | `IndexFeedDetail` | A feed search result, a content list row, an album name link of an Index track |
| Index track page | `IndexTrackDetail` | A track search result, a name match row, a track row of the Index album page |
| Publisher page | `PublisherDetail` | A publisher name link only |
| Readiness list | `ReadinessIssues` | Show only |

## Sidebar And Content List

- The sidebar holds, from top to bottom:
  - a status line
  - "Check all feeds" or "Apply updates (n)", and "Update n files"
  - the tag write report
  - "Playlists" with sort and "+"
  - "Saved Searches"
  - the tree of artists, albums and tracks
- The content list shows when the sidebar has no selection. One button cycles the filter through All, Library and Index. A second control selects Tiles or List.
  - All mixes the recent Index feeds with the stored Library rows. Library shows only stored rows. Index shows only remote rows and loads more pages.

## Duplicates

- Two navigation stacks for one content frame (finding 1).
- Two controls for one filter: a cycling button on the content list, and three chips on search results.
- Two download functions for one album, with two different results after the download.
- Two labels for one playlist control: "Add to playlist" and "+ Playlist".
- Two MusicBrainz scopes with similar labels.
- Code that no path reaches: `ArtistVm` with `render_artist_view`, and the "Play" action of `TrackActionState`.

## Not Confirmed

- Which control, if any, calls `check_feed_on_view` on the album page.
- Whether any path still pushes `QueueNowPlaying` in Music.
