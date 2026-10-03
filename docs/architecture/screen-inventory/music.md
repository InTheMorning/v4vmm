# Music Section Screen Inventory

Scope: the Music section of v4vmm. This covers the toolbar search and its
result tabs. It covers the Index feed and track pages, the Library, and the
Library album and track pages. It covers playlist pages, publisher pages,
artist and name-match pages, and the Music popups and dialogs. This is a
read-only inventory for Phase 2 of
`docs/plans/design-and-cleanup-overhaul-plan.md`. It states no rule and
changes no file.

Two structures hold most Music screens together:

- `LibraryApp` (struct in `src/library.rs`, render in `src/library/app_impl.rs`)
  is the one GPUI view for the Library side: Library list/tiles, the sidebar,
  and the Album/Track/Artist/Playlist detail panes.
- A separate set of dispatch files under `src/app/` (`search_dispatch.rs`,
  `name_match_dispatch.rs`, `publisher_dispatch.rs`) drives the remote-Index
  screens: Search Results, the Name-match page, Index feed/track pages, and
  the Publisher page. These mount into a shared "workspace" of frames
  (sidebar, content list, detail), defined in `src/view_models/workspace/`.

## Screen table

| Screen | Reached by | Shell file | View model | Key composites |
|---|---|---|---|---|
| Toolbar Search | The search input and Search button in the app toolbar, on every tab | `src/app/tab_bar.rs` | `src/view_models/app_toolbar.rs` (`AppToolbarVm`) | gpui_component `Input`, `Button` |
| Search Results (Feeds / Tracks / Artists) | A submitted toolbar query, or a restored `Search` breadcrumb | `src/ui/shells/search_results_inspector.rs`, `src/ui/shells/search_result_rows.rs` | `src/view_models/search_results/{mod,tabs,results,paged_tab,index_detail,empty_state,failure}.rs` | `SegmentedControl`, `FilterChipStrip`, `ListRow`, `Thumbnail`, `TagBadge` |
| Name-match page | An Artists-tab row for an Index name candidate (`index-artist:` id) | `src/ui/shells/name_match_page.rs` | `src/view_models/name_match_page.rs` | `SectionHeader`, `ListRow`, `Thumbnail`, `TagBadge` |
| Index Feed/Release page | A Feed row in Search Results, or a track's parent-feed link | `src/ui/shells/search_results_inspector.rs` + `src/ui/shells/entity.rs` | `src/view_models/entity_detail.rs` (`ReleaseDetailVm`) | `ReleaseDetailSurface`, `DetailHeader`, `DetailGrid`, `TrackRow`, `Thumbnail` |
| Index Track page | A Track row in Search Results, a Name-match row, or a track row on the Feed page | `src/ui/shells/search_results_inspector.rs` + `src/ui/shells/track.rs` | `src/view_models/track_detail.rs` (`TrackDetailVm`) | `TrackDetailSurface`, `TrackHeader`, `DetailGrid` |
| Library — List view | Open Music with no sidebar selection. Set the view control to "List" | `src/ui/shells/library/content_list.rs`, `src/library/app_impl.rs` | `src/view_models/library.rs` (`LibraryViewModel`) | `ListRow`, `Thumbnail`, `TagBadge`, `SkeletonTrackRow` |
| Library — Tiles view | Same entry. The view control is set to "Tiles" (the coded default) | `src/ui/shells/library/content_list.rs`, `src/library/app_impl.rs` | Same as List view | `TagBadge`, `Label`, image primitive, `Skeleton` |
| Library Album page | Click an album under an artist in the Library sidebar tree | `src/ui/shells/library/feed_detail.rs` | `LibraryAlbumDetailVm` (`src/view_models/library.rs`), `ReleaseDetailPageVm` | `ReleaseDetailSurface`, `DetailHeader`, `DetailGrid`, `TrackRow`, playlist popover |
| Library Track page | Click a track row on the Album page, a playlist, or search results | `src/ui/shells/library/track_detail.rs` + `track_detail_metadata*.rs` | `src/view_models/track_detail.rs`, `track_metadata_grid.rs`, `musicbrainz_panel.rs` | `TrackHeader`, `TrackDetailSurface`, `TrackMetadataGrid`, `FileHeader`, `MusicBrainzPanel` |
| Playlist page | Click a playlist row in the Library sidebar | `src/ui/shells/library/playlist_detail.rs` + `src/ui/shells/playlist.rs` | `playlist_detail.rs`, `paged_playlist_detail.rs`, `playlist_rss_check.rs` | `DetailHeader`, `DetailGrid`, `SkeletonTrackRow`, `ContextMenu` |
| Publisher page | "Open Publisher" on a Library track/album, or an Index album in Search Results | `src/ui/shells/publisher.rs` | `src/view_models/publisher_page.rs` | `DetailHeader`, `ListRow`, `Thumbnail`, `TagBadge`, `SectionHeader` |
| Artist page | A Library artist row, or a `library-artist:` row in the Artists tab | `src/ui/shells/library/feed_list.rs` + `src/ui/shells/artist.rs` | `LibraryArtistDetailVm` (`src/view_models/library.rs`), `ArtistDetailPageVm` | `DetailHeader`, `DetailGrid`, `Thumbnail` |
| Playlist popover | An "Add to playlist" / "+ Playlist" button on a track, album, or search row | `src/ui/composites/playlist_popover.rs` | `LibraryTrackPlaylistDisplay` / `LibraryAlbumPlaylistDisplay` (`library.rs`) | `Popover`, `Button`, `Divider`, `Input` |
| Confirmation dialogs | A "Remove" action, or "Remove from all playlists" on a stale playlist row | `src/ui/composites/confirmation_dialog.rs` + `library_removal_confirmation.rs` + `playlist_removal_confirmation.rs` | `src/view_models/library_removal.rs`, `playlist_rss_check.rs` | `Dialog`, `Button`, `Label` |
| Tag update popup | The "Update n files" button in the Library feed-update header row | `src/ui/shells/tag_update_confirmation.rs` | `src/view_models/tag_update.rs`, `application/queries/tag_update.rs` | `confirmation_dialog`, `Button`, `Label` |

---

## 1. Toolbar Search

1. **Name and reach.** The toolbar search control sits in the app toolbar on
   every tab. `TopApp::render` calls `render_tab_bar` once, apart from the
   tab switch. The operator types a query, then presses Enter or clicks
   Search.
2. **Owner files.** `src/app/tab_bar.rs` (`render_tab_bar`,
   `render_global_search`). View model: `src/view_models/app_toolbar.rs`
   (`AppToolbarVm`, `GlobalSearchDisplay`). Dispatch: `src/app.rs`
   (`submit_global_search`, `on_global_search_event`).
3. **Layout.** One toolbar row: app mark, three tab buttons (Music, Show,
   Settings), then a flexible center region with the search input and the
   Search button.
4. **Elements.** No title, no subtitle. The input shows placeholder text
   "Search Library and Index" with a search icon. The Search button shows
   the word "Search" at normal width, and shrinks to an icon with a tooltip
   below a width breakpoint. No artwork.
5. **Actions.** The Search button, or Enter in the input, calls
   `submit_global_search`, which opens Search Results for the trimmed
   query. An empty query opens nothing. No destructive action.
6. **Data.** The query text is the operator's own typed input, held in one
   field. The control shows no stored or remote data.
7. **Visual notes.** `ControlStyle::Primary` for the button, `ToolbarIcon`
   when compact. `Spacing::XS/SM/MD` for gaps. `SemanticColor::
   SecondarySystemBackground` and `Separator` for the bar.
8. **Redesign notes.**
   - The Search button's availability flag is hard-coded "Available." The
     operator sees no warning before a search runs and MusicIndex fails.
   - The placeholder text is the only hint that search covers both Library
     and Index content.
   - This is the one lasting entry point to both content sources. It could
     carry more visual weight.

## 2. Search Results (Feeds / Tracks / Artists tabs)

1. **Name and reach.** One screen with three tabs: Artists, Feeds, Tracks.
   A submitted toolbar query opens it in the Music content-list frame. A
   restored `Search` breadcrumb, or a Back/Forward move, reopens it.
2. **Owner files.** Shell: `src/ui/shells/search_results_inspector.rs`
   (`render_search_results_inspector`), row chrome in
   `src/ui/shells/search_result_rows.rs`. View models:
   `src/view_models/search_results/{mod,tabs,results,paged_tab,
   index_detail,empty_state,failure}.rs`. Frame chrome:
   `src/ui/shells/workspace.rs`, `src/ui/composites/filter_chip_strip.rs`.
   Query: `src/application/queries/search.rs` (`FetchIndexSearchResults`).
3. **Layout.** A breadcrumb ("Search: query") above the frame. A header
   row: the query text, the selected filter's name, and a three-way
   segmented control for Artists/Feeds/Tracks. A filter-chip row (All,
   Library, Index) above the header. Below, one scrolling row list, or an
   empty-state panel. A clicked row replaces this whole area with a detail
   page. There is no side-by-side list-plus-detail split.
4. **Elements.** Every row shares one shape: a 32 px thumbnail, a title
   line, a smaller secondary line, a source label ("In Library" in accent
   color, or "Index" in tertiary color), and an entity-type badge.
   - Artists tab: title is the artist name (library row), or the quoted
     phrase `Tracks matching "<name>"` (an Index name candidate).
   - Feeds tab: title is the feed title or its GUID. Secondary text joins
     release artist and episode count. Thumbnail is the feed's own image.
   - Tracks tab: title is the track title or its GUID. Secondary text
     joins track artist, release artist, and feed title. Thumbnail follows
     ADR 0075 Decision C (track image, then feed image).
   - Loading rows show a plain thumbnail and "Loading result." Empty state
     shows "No <tab> results" plus the query and filter. An error adds
     "Show details" and "Copy report" buttons.
5. **Actions.** The tab control switches Artists/Feeds/Tracks. The chip
   strip switches All/Library/Index. A clicked row opens: the Library's own
   track or album page (local id), an Index detail page (`index-track:` /
   `index-feed:` id, fetched only on open), or the Name-match page
   (`index-artist:` id). "Show details" and "Copy report" toggle and copy
   an error report. No destructive action.
6. **Data.** Library rows come from the local database. Index rows come
   live from the MusicIndex `/v1/search` endpoint. Each Index row carries
   only the search response's own summary fields. The search sends no
   per-row detail request (ADR 0075 packet 047). No RSS check, ID3 tag, or
   MusicBrainz record shows on this screen.
7. **Visual notes.** `LabelVariant::Headline`/`Caption` for the header.
   `FontSize::Micro` for rows. `SemanticColor::Accent` (library) and
   `TertiaryLabel` (Index) for the source label. Artwork stays at 32 px in
   every row. The list is dense, with `Spacing::XXS` row gaps.
8. **Redesign notes.**
   - Row artwork is fixed at 32 px and only grows to 80 px once a detail
     page opens. A musicindex.org-style redesign needs larger row artwork,
     or a grid.
   - A clicked row replaces the full list. Back is the only way back. No
     list-and-detail split exists here, unlike the Library's own
     sidebar-plus-content split.
   - The source label is the only visible sign of where a row comes from.
     No RSS, ID3, or MusicBrainz record shows until a detail page opens.

## 3. Name-match page

1. **Name and reach.** The operator clicks an Artists-tab row for an Index
   name candidate (`index-artist:` id). `open_name_match_page` pushes
   `FrameNavigationEntry::IndexNameMatches(name)` and starts the fetch. The
   breadcrumb is the quoted name. This page names no artist identity, no
   role, and no page type (ADR 0077 Decision 1): it is a search result.
2. **Owner files.** Shell: `src/ui/shells/name_match_page.rs`. View model:
   `src/view_models/name_match_page.rs` (`NameMatchPageVm`). Dispatch:
   `src/app/name_match_dispatch.rs`. Query:
   `src/application/queries/search.rs` (`FetchNameMatchTracks`). Reuses
   `render_result_row` from `src/ui/shells/search_result_rows.rs`.
3. **Layout.** One scrolling column: a section header with the page title,
   the track list, and an optional "more tracks" line. No side list, no
   side detail pane.
4. **Elements.** Title: `Tracks matching "<name>"`. Each row reuses the
   Search Results track-row shape (32 px thumbnail, title, track
   artist/feed title line, "Index" label, Track badge). A tertiary-color
   line states when MusicIndex reports more tracks than shown. Loading,
   empty, and error states each show one centered status line.
5. **Actions.** A clicked track row opens the Index Track page for that
   id. No filter, no tab, no destructive action.
6. **Data.** One live request: `GET /v1/tracks?artist=<name>` through the
   `INDEX_NAME_MATCH_TRACKS` request profile. Every row is a remote
   MusicIndex track. No local library data, RSS record, ID3 tag, or
   MusicBrainz record shows here.
7. **Visual notes.** Same row shape, text size, and color as the Search
   Results Tracks tab, through the shared `render_result_row` function.
8. **Redesign notes.**
   - This page reuses the Search Results track row in full, so a row
     redesign there applies here with no extra work.
   - "More tracks" is a plain text line, not a control a person can
     activate to load more.
   - By design this page shows no artist page. Keep that distinction
     clear in the redesign, since its look is near an Artist page.

## 4. Index Feed/Release page

1. **Name and reach.** Shows one remote MusicIndex feed (an album-like
   release). Reached from a Feed row in Search Results, or a track's
   parent-feed link. `FrameNavigationEntry::IndexFeedDetail{id,label}`
   keeps it on the frame's Back/Forward stack and breadcrumb.
2. **Owner files.** Screen: `src/ui/shells/search_results_inspector.rs`
   (`render_index_feed_detail`). Shared shell: `src/ui/shells/entity.rs`
   (`render_release_detail_shell`). Dispatch:
   `src/app/search_dispatch.rs` (`index_feed_detail_slots`). View model:
   `src/view_models/entity_detail.rs` (`ReleaseDetailVm`, `ReleaseHeroVm`,
   `ReleasePanelVm`, `TrackListVm`). Composites: `ReleaseDetailSurface`,
   `DetailHeader`, `DetailGrid`, `TrackRow`, `identity_action.rs`.
3. **Layout.** One scrolling column: header (thumbnail, badge, title,
   subtitle, feed-owner row), an action row (primary buttons left,
   identity buttons right), a summary-facts grid, a Description panel and
   an Identity panel (both always expanded), then a track list. No side
   pane, no footer.
4. **Elements.** Title: feed title or "Unknown Feed." Artwork: one 80 px
   thumbnail from the feed's own RSS channel image, with a fallback to the
   first track's image. Summary rows: Release Kind, Published (with its
   source, "RSS" or "MusicIndex"), Tracks, Duration, Language, Explicit.
   Identity rows: Website, Nostr, Feed URL, GUID. Track rows: number, 32 px
   thumbnail, title, duration, two action buttons.
5. **Actions.** "Download Feed" subscribes the whole feed (not
   destructive). "MusicBrainz" is permanently disabled, a stub. "Add feed
   to playlist" opens the playlist popover. "Open publisher" shows only
   when the feed states a publisher relationship. Identity row: open
   website or RSS link, or copy the Nostr key. Per track: Download and
   "+ Playlist."
6. **Data.** A live fetch on open (`FetchIndexFeedDetail`, no local
   database read). The response becomes a `FeedView`, which records
   whether a date came from RSS or MusicIndex. No RSS/ID3/MusicBrainz
   compare grid appears. The Identity panel and the "(source)" text after
   Published are the only provenance hints. Value4Value payment routes
   decode but never render.
7. **Visual notes.** `FontSize::Title2` for the title, `Micro` for most
   body text. `SemanticColor::Label/SecondaryLabel/TertiaryLabel`,
   `Spacing::XS/SM/MD/LG`. The Description panel starts expanded, never
   collapsed. Loading and failure states show a generic skeleton with a
   plain text line, no retry button.
8. **Redesign notes.**
   - Artwork tops out at 80 px even in the hero header.
   - The view model builds a "Remove Feed" destructive action and an
     accessibility label for the action group, and the screen discards
     both without using them.
   - Value4Value payment routes are fetched but never shown, on the one
     page a curator uses to judge a release.

## 5. Index Track page

1. **Name and reach.** Shows one remote MusicIndex track. Reached from a
   Track row in Search Results, a Name-match row, or a track row on the
   Feed page. `FrameNavigationEntry::IndexTrackDetail{id,label}` keeps it
   on the frame's Back/Forward stack.
2. **Owner files.** Screen: `src/ui/shells/search_results_inspector.rs`
   (`render_index_track_detail`). Shared shell: `src/ui/shells/track.rs`
   (`build_track_detail_surface`). Dispatch:
   `src/app/search_dispatch.rs` (`index_track_detail_slots`). View model:
   `src/view_models/track_detail.rs` (`TrackDetailVm`). Composites:
   `TrackDetailSurface`, `TrackHeader`, `DetailGrid`.
3. **Layout.** One scrolling column: header (80 px thumbnail, badge,
   title, artist), an external-links row shown only if a link exists (no
   primary-action row at all), a summary grid, and an optional Description
   box. No track list, no side pane, no footer.
4. **Elements.** Title: track title, else its GUID, else "Untitled."
   Artwork: one 80 px thumbnail, tried in order from the track's own
   image, the feed's image, then a legacy image field. Summary rows:
   Release, Track #, Duration, Release Date, feed publication date (a
   fallback only), Explicit, Publisher. Loading, missing, and failed
   states each collapse to one plain text line, with no artwork or badge.
5. **Actions.** Only two: open the website link, or copy the Nostr key.
   No Download, Play, or Add-to-playlist action anywhere on this page. No
   destructive action.
6. **Data.** A live fetch on open (`FetchIndexTrackDetail`, scoped by feed
   GUID when known), no local database read. No RSS/ID3/MusicBrainz
   compare grid: the Library's metadata-grid view model is never built
   here. Payment routes decode but never render.
7. **Visual notes.** Same token family as the Feed page. Artwork capped at
   80 px. Nearly every line below the header uses `FontSize::Micro`. A
   track with no description leaves a silent gap, with no "No
   description" label.
8. **Redesign notes.**
   - This page is visibly sparser than the Feed page: with no website or
     Nostr key, it shows no action at all.
   - The view model carries a feed-identity section and a publisher
     action. This screen requests neither one. The Library Track page
     renders both correctly for the same kind of data.
   - The track's own image and the feed's image are separate facts in the
     data. The page shows only one resolved picture. That distinction
     never reaches the operator.

## 6. Library — List view

1. **Name and reach.** This screen is the Music section with no sidebar
   item selected, and its view-mode control set to List. The operator
   opens the app, where Music is the default tab, and picks List.
   Clicking a sidebar artist, album, track, or playlist replaces this
   screen with a detail pane.
2. **Owner files.** Struct: `src/library.rs` (`LibraryApp`,
   `LibraryDetail`). Render and actions: `src/library/app_impl.rs`
   (`render`, `check_all_feeds`, `open_tag_update_popup`,
   `scan_tag_updates`). Rows: `src/ui/shells/library/content_list.rs`.
   Sidebar: `src/ui/shells/library/sidebar.rs`. Frame chrome (filter and
   view-mode controls): `src/ui/composites/frame_shell.rs`. View models:
   `src/view_models/library.rs` (`LibraryViewModel`, `ContentListPageVm`),
   `src/view_models/workspace/chrome.rs` (`ContentFilter`,
   `ContentViewMode`).
3. **Layout.** Frame header: back/forward, title, menu. Filter row: a
   Library/Index cycling button, then the Tiles/List toggle. Below, a
   resizable split: sidebar (playlist rows, then the artist/album/track
   tree) on the left. On the right, a feed-update toolbar row ("Check all
   feeds" or "Apply updates," "Update n files," a report line), then the
   scrolling row list.
4. **Elements.** Each row: a 32 px thumbnail, a title/secondary-text
   block, a library-membership label ("In library" or "Not in library"),
   an optional state label ("New release"), action buttons, and a trailing
   entity-type badge ("Release" or "Track"). Loading rows show a skeleton.
   Sidebar rows show a disclosure arrow, then artist, album (34 px
   thumbnail), and track (24 px thumbnail) levels, with the selected track
   marked by an accent border.
5. **Actions.** Row actions: repair a broadcast route, confirm a track
   removed from its feed, or remove the track from the library
   (destructive, opens a confirmation dialog). Clicking a row opens the
   Index feed detail page, not a local detail pane. "Check all feeds"
   checks subscribed feeds for staleness and repairs routes. Its button
   becomes "Apply updates (n)" when stale feeds exist. "Update n files"
   opens the tag-update confirmation (writes audio tags on confirm).
   Sidebar: sort-cycle and add-playlist buttons.
6. **Data.** The sidebar tree and row data are stored database values. The
   default content list is a live MusicIndex recency query (ADR 0062).
   "Check all feeds" triggers a live RSS check. No ID3 or MusicBrainz
   inspection surface appears here. That lives on the Track detail pane.
7. **Visual notes.** `SemanticColor::{TertiaryLabel,Accent,Danger,
   SystemFill,Separator,SelectedContent,Focus}`, `FontSize::Micro`,
   `Spacing::XXS` to `XXL`. Artwork is small everywhere (32 px or less).
   Density is compact. Empty and error states are a centered message.
   Loading shows skeleton rows.
8. **Redesign notes.**
   - Artwork is small everywhere in List view, against the
     "browsable music player" goal.
   - The Library/Index filter is one cycling button, not a 3-way chip
     control, and may be less discoverable.
   - Clicking a row always opens the Index feed detail, even for an
     already-local track. There is no direct path from a row to the
     local Album/Track pane the sidebar shows.

## 7. Library — Tiles view

1. **Name and reach.** The same Music section and content-list frame, with
   the view-mode control set to "Tiles," the coded default. A fresh
   session opens here.
2. **Owner files.** Identical to List view, except rows render through
   `render_content_list_tiles`/`render_content_list_tile` in
   `src/ui/shells/library/content_list.rs`. Sidebar, "Check all feeds,"
   "Update n files," and frame chrome files are the same as List view.
3. **Layout.** Same frame header and filter row as List view. The right
   pane stacks the same feed-update toolbar row, then a wrapping grid of
   tiles, not a vertical row list. Each tile stacks artwork, title,
   secondary text, then a badge row.
4. **Elements.** Each tile is a fixed 176 px column: artwork at 152 px
   (an image, or a bordered placeholder square), a title, an optional
   secondary line, and a badge row (entity-type badge, library-membership
   label, optional state label). Loading tiles show a skeleton block. Tile
   artwork draws directly, not through the shared Thumbnail composite used
   by rows and the sidebar.
5. **Actions.** Same typed actions as List view (repair route, confirm
   removal, remove from library/destructive). Tile click opens the Index
   feed detail, same as a List row. "Check all feeds" and "Update n files"
   behave the same, since the toolbar sits above the grid.
6. **Data.** Same sourcing as List view. Tile artwork resolves from the
   track's own image, then the album image, in that order, cached locally
   by URL (an RSS item image falling back to the channel image).
7. **Visual notes.** Same token set as List view, plus a 176 px tile width
   and a 152 px artwork size unique to this view. Artwork is the dominant
   part of a tile, a sharp contrast with List view's 32 px thumbnails.
   Empty, loading, and error states are identical to List view.
8. **Redesign notes.**
   - Tiles view already gives artwork real size and is the closer
     starting point for the "browsable music player" goal.
   - Tile artwork bypasses the shared Thumbnail composite, so a future
     artwork change has two call sites to update, not one.
   - No "Inspect" disclosure exists on tiles or rows for RSS/ID3 values.
     That only exists on the Track page, so the Library's own list and
     tile screens do not yet meet the plan's inspection-access goal.

## 8. Library Album page

1. **Name and reach.** The `LibraryDetail::Album(AlbumNode)` variant.
   Reached by expanding an artist in the Library sidebar tree and clicking
   one of its album rows.
2. **Owner files.** Shell: `src/ui/shells/library/feed_detail.rs` (named
   for its history. It renders the album/release detail). View models:
   `LibraryAlbumDetailVm`/`AlbumNode` (`src/view_models/library.rs`),
   `ReleaseDetailVm`/`ReleaseHeroVm` (`src/view_models/entity_detail.rs`).
   Shared layout: `render_release_detail_shell`
   (`src/ui/shells/entity.rs`). Composites: `ReleaseDetailSurface`,
   `DetailHeader`, `DetailGrid`, `TrackRow`, the playlist popover.
3. **Layout.** Header (80 px artwork, title, subtitle). Action row:
   primary action (Download/Remove) plus identity links. Summary-facts
   grid. Panels: a collapsed-by-default description, then a contributors
   panel if present. A track-section title and count line, then track
   rows.
4. **Elements.** Title: album/feed name. Artwork: 80 px, from the feed's
   image (RSS or MusicIndex, never ID3). Summary rows come from stored
   values (ADR 0075 packet 020). A track row shows a "New" label when it
   is indexed but not in the library. It shows a short MusicBrainz-status
   label when a match state exists. Track rows: number, 32 px thumbnail,
   title, duration, a trailing action cluster.
5. **Actions.** "Download Feed" or "Remove" (destructive tone, depending
   on membership). "MusicBrainz" runs an album-wide lookup across all
   tracks, separate from the per-track panel on the Track page. "+
   Playlist" adds the album or opens the create-playlist flow. "Open
   publisher" shows only when the album states a publisher relationship.
   Per track: a download/remove toggle (destructive when removing), and
   its own "+ Playlist." Clicking a row body opens the Track page.
6. **Data.** Stored values for the feed, plus local identity and metadata
   facts. No live fetch on open. The publisher relationship is a stored
   value, not a live request. This screen shows no raw-provenance or
   compare view of its own. ID3 and MusicBrainz data for one track show
   only on that track's own page.
7. **Visual notes.** `color::text_muted()/text_primary()`, status-role
   colors for MusicBrainz state text, `spacing::SM/XL/LG`. Artwork is
   large (80 px) only in the header. Every track row stays at 32 px, the
   app-wide row size. No album-wide loading state is shown. A per-track
   busy flag disables that row's button during a write.
8. **Redesign notes.**
   - Track-row artwork is 32 px even on an album page. A browsable-player
     feel likely wants larger art here too, not only in the header.
   - The album page shows no inspection or provenance data at all. Adding
     one would need a new slot in the shared release-detail shell.
   - The album-wide MusicBrainz lookup and the single-track MusicBrainz
     panel are different actions at different scopes. Keep them visually
     distinct in the redesign.

## 9. Library Track page (metadata compare grid and MusicBrainz panel)

1. **Name and reach.** The `LibraryDetail::Track(Box<InspectorFrame>)`
   variant. Reached by clicking a track row on the Album page, in a
   playlist, or from search, which pushes a Back/Forward entry.
2. **Owner files.** Shells, all in `src/ui/shells/library/`:
   `track_detail.rs` (header, summary), `track_detail_metadata.rs` (action
   row, panels), `track_detail_metadata_grid.rs` (row assembly),
   `track_detail_metadata_cells.rs` (cell rendering),
   `track_detail_metadata_values.rs` (expandable values). View models:
   `src/view_models/track_detail.rs`, `track_metadata_grid.rs`,
   `musicbrainz_panel.rs`, and `InspectorFrame`/`LibraryTrackInspectorState`
   in `src/library.rs`/`src/view_models/library.rs`. Composites:
   `TrackHeader`, `TrackDetailSurface`, `TrackMetadataGrid`,
   `MusicBrainzPanel`, `FileHeader`, `DisclosureGroup`, `TagBadge`. Pure
   data: `src/track_compare.rs`, `src/metadata.rs`.
3. **Layout.** A breadcrumb, then `TrackHeader` (80 px artwork, badge,
   title, artist). A primary action row (subscription toggle, "+
   Playlist," "Compare ID3," "MusicBrainz," plus staged-edit controls).
   External identity links, then the feed's own identity links kept apart
   (ADR 0075 Decision B). A summary grid, then a collapsible description.
   Below that is a row of one to three columns: the track summary, an
   optional ID3 panel, and an optional MusicBrainz panel. Below the row
   sits the full metadata comparison grid, with one row group for each
   category and a maximum of three side-by-side value cells.
4. **Elements.** Title/artist in large type. Artwork: 80 px in the header
   and both panels, from the track's own image falling back to the album
   image. Embedded ID3 artwork only shows inside an expanded "Artwork"
   grid cell, read straight from the file. A format badge (FLAC/MP3) shows
   on the file panel. Grid group headers can be a disclosure the operator
   expands or collapses. Grid data rows: a label plus a maximum of three
   value cells, each independently expandable.
5. **Actions.** Subscription toggle (Download/Remove Track, destructive
   when removing). "+ Playlist." "Compare ID3" and "MusicBrainz" buttons
   are disabled until the track is downloaded locally. Each loads its
   panel on click. Inside the ID3 panel: Re-read and Redownload. Inside
   the MusicBrainz panel: pick a release candidate. Staged ID3 edits show
   "Apply tags (n)" and "Discard staged." "Open publisher," same rule as
   the Album page.
6. **Data.** The two detail panels start hidden. Each loads only on an
   explicit click, through a four-state `Hidden/Loading/Empty/Loaded`
   panel. It is one grid, not separate panels, for the row comparison.
   Each row carries an RSS value, an ID3 value, and a MusicBrainz value
   together, each marked Match, Different, or Missing.

   The grid's column count depends only on whether the track is
   downloaded. It does not depend on whether its panel load is complete.
   So a column can show empty cells before the operator presses its load
   button.

   The RSS value is a stored database value, or the last RSS check
   (MusicIndex is a cache of RSS, ADR 0075 Decision I). The ID3 value is
   a direct read of the local file's tags. The MusicBrainz value is a
   live lookup, cached after it loads. This is the one Library screen
   where RSS, ID3, and MusicBrainz show as three distinct columns, kept
   apart and not merged.
7. **Visual notes.** A Match/Different/Missing color role applies to every
   cell's text across all three columns. ID3 frame labels carry a
   version-specific color. Expanded cells show full text or, for
   "Artwork," the embedded image at 80 px. Grid text is uniformly the
   app's smallest size, with no visual hierarchy between field categories
   beyond a bold group heading.
8. **Redesign notes.**
   - The compare grid is already gated behind two explicit buttons and a
     loading state. This is a strong model for the operator's planned
     "Inspect" disclosure. Today it is not set apart from the ordinary
     track view. It is one long scrolling stack.
   - Every grid row renders at the smallest text size, with no visual
     split between a casual read and a deep inspection read.
   - A reserved, still-empty grid column (downloaded but not yet loaded)
     could read as broken. The redesign should gate the column on the
     loaded state, and not only on "downloaded."

## 10. Playlist page (with the RSS check report)

1. **Name and reach.** The operator clicks a playlist row in the Library
   sidebar tree, which sets `LibraryDetail::Playlist(...)`. No keyboard
   shortcut exists for this path.
2. **Owner files.** Adapter: `src/ui/shells/library/playlist_detail.rs`.
   Shared shell: `src/ui/shells/playlist.rs` (owns the rendering). View
   models: `src/view_models/playlist_detail.rs`,
   `paged_playlist_detail.rs`, `playlist_rss_check.rs`. Storage:
   `src/db/rss_check_runs.rs`, `rss_field_holds.rs`.
3. **Layout.** `DetailHeader` (playlist icon, title). A `DetailGrid` (track
   count, created/modified dates, description). An action row (Rename,
   Delete, Check RSS). The RSS check report, shown only after a run
   exists. Then a scrollable track list. Each row shows a drag handle,
   position, 24 px thumbnail, title and artist, an availability label,
   duration, a Play button, and an overflow menu. An empty-state message
   shows when there are no tracks.
4. **Elements.** Title: the playlist's own name. Grid values are stored,
   not fetched live. Track thumbnails come from the track's own image,
   falling back to the album image. A track with no image shows a
   hard-coded music-note glyph, not a token-driven icon. RSS report rows:
   feed name plus a colored result. A difference row states the field, a
   plain change sentence, and the check time. A stale-feed row links to
   podping.me and offers "Copy feed URL."
5. **Actions.** Rename (inline editor). Delete — no confirmation dialog at
   all. Check RSS (starts a live check). Per row: Play, Move up/down,
   Remove, drag-reorder. "Download" on an added-track row. "Remove from
   playlist" and "Remove from all playlists" (the latter destructive, with
   a confirmation dialog) on a removed-from-feed row.
6. **Data.** Playlist and track facts are stored database values. The RSS
   check report runs live, per click, through a runtime actor, and is
   stored in `rss_check_runs`/`rss_field_holds`. The report's difference
   list is the raw provenance panel for this screen: field name, old
   stored value, new RSS value, and the check time. No MusicBrainz data
   appears here.
7. **Visual notes.** This shell styles most text with the older
   `style::color::*` functions, not the `Label` primitive or
   `SemanticColor` enum used elsewhere in the app. The RSS report is a
   flat stack of caption-size text lines, with no card or grid chrome.
   Artwork only appears as 24 px row thumbnails.
8. **Redesign notes.**
   - Artwork is small (24 px) and absent everywhere else on the page.
   - "Delete playlist" has no confirmation, while a smaller action
     (remove one track from every playlist) does. An inconsistency in
     destructive-action care.
   - The RSS report reads as a developer-facing diagnostic list. It is
     good inspection data to keep reachable, but it likely needs a
     disclosure in the redesign. This screen also uses an older styling
     path than the dialog composites, a token-discipline gap to close.

## 11. Publisher page

1. **Name and reach.** Keyed by the publisher feed GUID, never name text
   (ADR 0077 Decision 1). Reached by an "Open Publisher" button on a
   Library track or album page, or on an Index album in Search Results.
2. **Owner files.** Shell: `src/ui/shells/publisher.rs`. Dispatch:
   `src/app/publisher_dispatch.rs`. View model:
   `src/view_models/publisher_page.rs` (`PublisherPageVm`). Queries:
   `FetchLibraryPublisherPage`, `FetchIndexPublisherPage`. Storage:
   `src/db/publisher_relationships.rs`, `publisher_link_facts.rs`.
3. **Layout.** One scrolling column. A `DetailHeader` whose facts sit
   inside its own data rows (no sibling grid, unlike the Artist page). An
   optional notice when a live request for other albums fails. Two album
   groups with section headers: "Library Albums"/"Other Albums" (Library
   context), or "Owned Albums"/"Listed By" (Index context).
4. **Elements.** Title: the publisher feed's own stated title, or its GUID
   with a "No title" label. Header facts: Type (Artist or Label, ADR
   0078), artist names that state or do not state this feed. No artwork
   on the header itself. Its image slot always shows a placeholder. Album
   rows: a 32 px thumbnail, a title, an optional artist name with its
   source in parentheses, an optional role line, and tag badges ("Not
   listed by the publisher," "In Library").
5. **Actions.** This page exposes no button of its own. "Open Publisher"
   is the action of the calling screen. No destructive action.
6. **Data.** Library albums come from a stored relationship table, an
   observation of a past MusicIndex response. Albums outside the library
   come from a live request that can fail. In Index context, every album
   fact is read live. Fields for a stated role and a derived role are
   kept apart and never merged (ADR 0077 Decision 5). No ID3 or
   MusicBrainz data appears. No compare grid. The only provenance text is
   the "(source)" suffix after an artist name.
7. **Visual notes.** Consistent `SemanticColor`/`Spacing`/`Radius` token
   use. Artwork only appears as 32 px row thumbnails. The header carries
   none. Loading, empty, and failed states each show a centered message.
8. **Redesign notes.**
   - The header never shows artwork, though every album row below it
     carries a thumbnail. A gap against an artwork-forward redesign.
   - All inspection and provenance detail sits in small caption text
     beside each row. There is no expandable panel to preserve.
   - The shared `DetailHeader` composite is used differently here
     (data rows inside the header) than on the Artist page (a sibling
     grid). A header redesign must settle this before reuse.

## 12. Artist page

1. **Name and reach.** Keyed by artist name text, a name-grouped fallback
   for albums naming no publisher. Reached from an artist row in the
   Library sidebar tree, or a `library-artist:` row in the Search Results
   Artists tab. An Index name candidate opens the Name-match page instead,
   never this screen.
2. **Owner files.** Dispatcher: `src/ui/shells/library/detail.rs`. Shell:
   `src/ui/shells/library/feed_list.rs`, calling the shared
   `render_artist_detail_shell` (`src/ui/shells/artist.rs`). View model:
   `LibraryArtistDetailVm` (`src/view_models/library.rs`),
   `ArtistDetailPageVm` (`src/view_models/artist_detail.rs`). A second,
   unreachable pair (`ArtistVm`/`render_artist_view`) exists with no live
   caller. See Redesign notes.
3. **Layout.** One scrolling column. A `DetailHeader` (thumbnail, artist
   name, subtitle "Grouped by name"). A separate `DetailGrid` below it for
   metadata. A plain vertical list of feed rows. No toolbar, no side pane.
4. **Elements.** Title: the artist name, or "Unknown." Metadata rows:
   Albums, Tracks, Downloaded counts (shown only when above zero).
   Artwork: one 80 px header image from the first local track's album
   image for this artist name. Each feed row: a 32 px thumbnail, a title,
   a track-count line. No state badges.
5. **Actions.** Clicking a feed row opens that album. No other action, no
   destructive action.
6. **Data.** Already-loaded local track rows only. No live MusicIndex
   request, RSS check, or MusicBrainz lookup here. Richer fields (sort
   name, area, active years, aliases) stay empty: no current source
   supplies them (ADR 0079). No provenance or inspection panel exists.
   That detail lives only on the Track and Album pages this page links to.
7. **Visual notes.** Mixes two token-access styles: the tokens.rs enum
   path in one file, the style.rs free-function path in another. Artwork
   plays a small role: one 80 px header image, then 32 px row icons. No
   loading, empty, or error state, since the page renders from
   already-loaded data.
8. **Redesign notes.**
   - A second, dead artist view/shell pair exists in the code, with
     richer fields that cannot be populated under the current data model.
   - `DetailHeader` is used two different ways across this page and the
     Publisher page. Reconcile before a shared header redesign.
   - Artwork plays only a small, supporting role. There is no existing
     large-artwork surface to build an artist page redesign from.

## 13. Playlist popover

1. **Name and reach.** The "Add to playlist" (also labeled "+ Playlist")
   trigger button on a track action row, an album action row, or a search
   result row. Not present on the Playlist page's own rows. Click outside,
   or Escape, closes it without committing.
2. **Owner files.** `src/ui/composites/playlist_popover.rs`
   (`AddToPlaylistPopover`), re-exported by `src/ui/playlist_popover.rs`.
   Inputs built in `src/view_models/library.rs`
   (`LibraryTrackPlaylistDisplay`, `LibraryAlbumPlaylistDisplay`).
3. **Layout.** A small trigger button opens a floating panel below it,
   with internal scrolling. Two modes: a list of existing playlists plus
   "New Playlist," or a create form (name input, "Create & Add").
4. **Elements.** List mode: one full-width button per playlist, or "No
   playlists yet." Create mode: a name input and a filled "Create & Add"
   button. No artwork, no metadata, no subtitle anywhere in this popover.
5. **Actions.** Select a playlist (adds and closes). "New Playlist"
   switches to create mode. "Create & Add" validates a non-empty name,
   creates the playlist, adds the item, and closes. "Back" returns to the
   list. No destructive action.
6. **Data.** The playlist list is a stored database value, passed in
   already built by the caller. The popover itself makes no query.
7. **Visual notes.** Plain text list, standard button styling, low
   density, full-width rows.
8. **Redesign notes.**
   - The trigger label differs across call sites ("Add to playlist" vs
     "+ Playlist") for the same action.
   - No artwork or visual identity per playlist row. A musicindex.org-
     style redesign would likely want a thumbnail or color per row.
   - This composite is shared by four screens, so a visual change here
     reaches all of them at once.

## 14. Confirmation dialogs

1. **Name and reach.** The shared `confirmation_dialog` primitive, used
   for: removing a feed or track from the library, and "Remove from all
   playlists" on a stale playlist row. Deleting a playlist itself opens no
   dialog at all (see Playlist page, item 5).
2. **Owner files.** Primitive: `src/ui/composites/confirmation_dialog.rs`.
   Presenters: `src/ui/shells/library_removal_confirmation.rs`,
   `playlist_removal_confirmation.rs`. View models:
   `src/view_models/library_removal.rs`, and a helper in
   `playlist_rss_check.rs`.
3. **Layout.** A modal dialog, layered over the window: a message, an
   optional scrolling list of affected items, then a right-aligned
   Cancel/Confirm row. No header image or icon. Escape and Cancel are the
   only ways out besides Confirm.
4. **Elements.** A title, a message body, an optional item list (each item
   with a title, an optional subtitle, an optional colored status mark,
   and optional detail lines), and Cancel/Confirm buttons, where Confirm
   can be styled as destructive.
5. **Actions.** Cancel (closes, no effect). Confirm (closes, runs the
   caller's action). Library removal always marks Confirm as destructive.
   "Remove from all playlists" also marks Confirm as destructive and lists
   each affected playlist by name.
6. **Data.** Both presenters receive fully built display text from their
   view model. The dialog itself runs no query. No raw provenance or
   inspection data appears in either dialog.
7. **Visual notes.** Uses the `SemanticColor` enum and the `Label`/
   `Button` primitives consistently, more disciplined than the Playlist
   page's own styling. A status mark always carries text, not color
   alone. No artwork anywhere.
8. **Redesign notes.**
   - This primitive is already well tokenized. A good model for
     restyling the Playlist page's own RSS report.
   - The missing confirmation on "Delete playlist" is a real gap against
     the app's own cautious-destructive-action rule.
   - No item in scope carries artwork. An artwork-forward confirmation
     item would need a new image slot.

## 15. Tag update popup ("Update n files")

1. **Name and reach.** The "Update n files" button sits in the Library's
   feed-update header row, beside "Check all feeds"/"Apply Updates." It
   shows only when a background scan finds a file. That file's tags must
   differ from the stored metadata.
2. **Owner files.** Presenter: `src/ui/shells/tag_update_confirmation.rs`
   (reuses `confirmation_dialog`). View model:
   `src/view_models/tag_update.rs`. Scan: `application/queries/
   tag_update.rs`. Write: `application/commands/tag_update.rs`
   (re-checks playback right before writing, so a file the show starts
   playing after the scan is still skipped).
3. **Layout.** A button in the Library header row (its label becomes
   "Updating n files" and disables mid-write). Clicking it opens the
   shared modal dialog: a message, a scrollable item list, then
   Cancel/"Write Tags" buttons. After a confirm, a report block renders
   below the header row, outside the dialog.
4. **Elements.** Each popup item: track title, album subtitle, and one
   line per ID3 frame to write, formatted as the new value alongside the
   old file value. An excluded item gets a colored mark ("In use by the
   show," "Cannot read"). The post-write report colors each line by
   outcome (written, not written, failed). No artwork.
5. **Actions.** Cancel (writes nothing). "Write Tags" runs the write per
   file, skipping any file now in use or unreadable, and reports each
   file's own outcome. One failure does not stop the others. This button
   is coded as non-destructive, despite writing to disk.
6. **Data.** This compares the database's own stored-value projection
   against the embedded ID3 tag read straight from the file. No live
   MusicIndex or RSS call runs during this scan.

   The popup's item list is itself the raw compare data for this screen,
   a frame-by-frame list of the expected value against the file value. It
   shows as plain text in the generic dialog, not in its own panel.
7. **Visual notes.** Reuses the confirmation dialog's token styling
   (warning/danger labels). The button and report sit in a dense toolbar
   strip, at the app's smallest text size.
8. **Redesign notes.**
   - A real, file-mutating, irreversible write is marked non-destructive
     in code. Confirm this choice still holds once destructive styling is
     redrawn.
   - This is the one screen where raw ID3 inspection data is already
     operator-facing. Keep it reachable even as the primary flow changes.
   - The button and report live in the global Library header, not on any
     one entity's screen, so a header redesign could hide them by
     accident.

---

## Cross-screen observations

**Artwork size varies by screen with no stated rule.** Rows show 32 px
everywhere (Search Results, Library List/Tiles, Album/Track rows,
Playlist rows at 24 px). Detail headers cap at 80 px (Index Feed/Track,
Album, Track). Library Tiles reach 152 px. The Publisher-page header
shows no artwork at all, though every row below it carries a thumbnail.

The stated redesign goal is purposeful, prominent artwork like
musicindex.org. This spread is the single largest gap between the
current code and that goal.

**The same type of data renders through different idioms.** Provenance
shows as a three-column Match/Different/Missing grid (Library Track
page), a "(source)" text suffix (Index Feed page, Publisher page), or a
source label in two colors ("In Library"/"Index," Search Results). On
the Index Track page and the Library Album page, it does not show at
all. A redesign that picks one inspection idiom should expect to touch
every one of these screens.

**Two different layout shapes share one section.** The Library screens
use a persistent sidebar-plus-content split (`SplitPane`). The search
flow (Search Results, Name-match, Index Feed/Track pages) is a single
full-width pane whose content is swapped by navigation, with no list-plus-
detail split and no sidebar. This is a structural difference a visual
token change alone cannot fix.

**Destructive-action discipline is inconsistent.** "Delete playlist" has
no confirmation dialog, while the smaller action "Remove from all
playlists" does. The tag-update "Write Tags" button is coded
non-destructive although it is an irreversible disk write. Removing a
track from the library is destructive and confirmed. These three related
actions disagree on what counts as destructive.

**Token-access style is mixed.** Most screens read colors and spacing
through the `SemanticColor`/`FontSize`/`Spacing` enums in
`src/ui/tokens.rs`. The Playlist page and part of the Artist page instead
call older free functions in `src/ui/style.rs` (`color::text_muted()`,
`spacing::SM`). The Playlist page also hard-codes a music-note glyph
character for a missing thumbnail, which the app's own token-discipline
rule forbids.

**One feature exists twice at different scopes.** A MusicBrainz lookup
exists as an album-wide batch action (Library Album page) and as a
single-track panel (Library Track page). They are easy to confuse without
a visual distinction. The Index Feed page's own "MusicBrainz" button is a
permanently disabled stub.

**Dead or discarded code sits beside live screens.** An unreachable
second Artist view/shell pair (`ArtistVm`/`render_artist_view`) exists
with no call site. The Index Feed and Index Track detail surfaces each
compute an action-group accessibility label and a destructive "Remove"
action, then discard both without rendering them.

**A feature gap splits the two Track screens.** The Library Track page
shows a feed-identity panel and a publisher link. The Index Track page's
view model carries the same two facts, but the screen never requests
them, so they never render there.

**Value4Value payment-route data is fetched but never shown.** The Index
Feed page and the Index Track page each decode payment-route fields into
their view data. No Music screen in this inventory displays them.

Screens inventoried: 15.
