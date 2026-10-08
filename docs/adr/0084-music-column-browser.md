# ADR 0084: Music Is A Column Browser

## Status

Proposed - 2026-10-08. The operator made each choice below on 2026-10-08, in Phase 4A of the [overhaul plan](../plans/design-and-cleanup-overhaul-plan.md#phase-4a-flow-and-information-architecture). The choices come from the [Music flow map](../architecture/music-flow-map.md) and a private prototype canvas.

The first draft left three points open. The operator decided them on the same day:

- the rule for each release kind (Decision 5)
- the artist link when the publisher is a label (Decision 7)
- the place of saved searches (Decisions 2 and 3)

When the operator accepts it, it amends these decisions:

- [ADR 0046](0046-workspace-frame-architecture.md): the Music frames are the three columns of Decision 1, with one navigation stack (Decision 9).
- [ADR 0047](0047-library-search-unification.md) items 1, 5 and 9: the source list holds destinations, the scope control has two states, and saved searches move (Decision 2).
- [ADR 0049](0049-inspector-source-ownership.md): the Library source tree and the drill-down route for each origin go. Each item has one page (Decision 4).
- [ADR 0060](0060-workflow-surface-structure.md) Vocabulary: the curator sees "Release" (Decision 5).
- [ADR 0062](0062-music-content-surface.md): the scope control has two states, and the source tree goes. Recency order and paging stay.
- [ADR 0082](0082-publisher-roles-belong-to-each-album-link.md) Decision 2: each group of a publisher page divides its releases by kind (Decision 6).
- [ADR 0083](0083-design-language.md): Decision 9 is superseded by Decision 3 of this ADR. Decisions 4, 5 and 6 gain the additions of Decisions 12, 5 and 10.

## Context

The operator found the Music flow scattered and hard to learn on 2026-10-08. The [Music flow map](../architecture/music-flow-map.md) records seven structural findings in the code:

1. Two navigation stacks drive one content area. A path that starts in the sidebar shows no Back, no Forward and no breadcrumb.
2. A Library track row in the content list does nothing when clicked. An Index row opens the Index album page.
3. No list has multiple selection.
4. Play exists only on the playlist page, through the one Show session.
5. The list of tracks that need attention opens only from Show.
6. A publisher page opens only from a name link.
7. The track page breadcrumb records history, so it repeats pages.

The same album has a Library page and an Index page with different routes. "Artist" has three page types: the Library artist page by name, the Index name match page, and the publisher page.

The operator set four curator jobs for the structure: find new music, take music in, build a show playlist, and keep the Library ready. The operator added two constraints. Multiple selection with one download action is essential. An audition player with its own audio route comes later ([ADR 0068](0068-show-cue-and-audition-isolation.md)).

The operator compared three clickable structures and chose the column browser. The operator then asked for the changes that Decisions 1, 3, 5, 6, 8 and 12 record.

## Decision

### 1. Music Is Three Columns

The Music section shows three columns:

| Column | Holds | ADR 0046 frame |
|---|---|---|
| Sidebar | The destinations of Decision 2 | `SourceList` |
| List | The items of the selected destination, or the views of Discover | `ContentList` |
| Detail | The page of the selected item | `Detail` |

One shared shell owns the three columns. At a narrow width the sidebar collapses to icons. Each icon has a tooltip and an accessibility label. A destination with items that need attention shows a mark on its icon. Its tooltip and its accessibility label give the count, so the mark never relies on color alone. The operator can collapse and expand the sidebar at any width.

### 2. The Destinations

The sidebar lists these destinations, in this order:

1. **Discover** (Decision 3).
2. **Releases** (Decisions 5 and 6).
3. **Artists** (Decision 7).
4. **Tracks**: the tracks of the Library.
5. **Needs attention**: the tracks that a show cannot use as they are, with a count. Music reaches this list without Show (flow map finding 5).
6. **Playlists**.

Search stays a toolbar command ([ADR 0060](0060-workflow-surface-structure.md)). Its results fill the list column, and a result opens in the detail column.

The feed state and "Check all feeds" stay at the bottom of the sidebar until the status bar of [ADR 0083](0083-design-language.md) Decision 7 takes them.

Saved searches are views in the Discover column (Decision 3). The operator decided this on 2026-10-08.

### 3. Music Opens On Discover

Discover is the first destination and the default when Music opens. Its list column holds views. The first view is "New releases": the Index releases, newest first, as tiles or rows, with the recency order and the paging of [ADR 0062](0062-music-content-surface.md).

Each saved search is a view after "New releases", for example "Saved: bluegrass". A saved search runs its query against the Library and the Index.

A later decision adds more views to the Discover column, for example shared musicL playlists and genre tags.

This supersedes ADR 0083 Decision 9: Music opens on Discover, not on a home. "A tile is a release" stays.

### 4. One Page For Each Item, And The Library Is A State

Each release, track, artist and playlist has one page, whatever its origin. The origin selects the data source of the page, not its route or its layout.

| Item | Page identity |
|---|---|
| Release | Its feed GUID |
| Track | Its item GUID and its feed GUID |
| Artist | Decision 7 |
| Playlist | Its local id |

A page shows its Library state with a word, for example "In Library", "3 of 7 tracks" or "Not downloaded". Its actions follow that state ([ADR 0083](0083-design-language.md) Decision 5).

This replaces the drill-down route for each origin of ADR 0049. A click on a Library row and a click on an Index row open the same page.

### 5. A Release Has A Kind

The curator sees "Release" for a feed of music, in place of "Album" and "Feed". Many releases are singles. Internal names do not change ([ADR 0060](0060-workflow-surface-structure.md) Vocabulary).

Each release shows its kind: Album, EP or Single. Labels that name a release use its kind, for example "Download single".

The operator decided this rule on 2026-10-08: the kind comes from the value that the feed states, when it names album, EP or single. Without a stated value, a release with one track is a single, and any other release is an album. EP shows only when the feed states it.

### 6. Singles Of One Artist Show As A Group

In the Releases list, two or more singles of one artist in the current scope show as one group row. The row shows the artist, the count and the Library state, for example "Haleen · 7 singles · 5 in Library".

- The row opens in place to show each single. Its checkbox selects each single of the group.
- The group page lists the singles, newest first. Its main action is "Download n singles" when some are not in the Library, else "Add to playlist".
- A single page links to its group, for example "All 7 singles".
- On an artist page, inside each group of [ADR 0082](0082-publisher-roles-belong-to-each-album-link.md) Decision 2, "Albums and EPs" come before "Singles".

The group key is the artist identity of Decision 7. Two artists with the same name never share a group.

### 7. One Artist Page With Two Levels

An artist page has one layout and one of two identities. The page says which identity it has.

- **Publisher level.** The page of a publisher feed, keyed by its feed GUID ([ADR 0077](0077-publisher-feed-artist-binding.md) Decision 1). It shows the feed title, and the groups and roles of [ADR 0082](0082-publisher-roles-belong-to-each-album-link.md).
- **Name level.** A labeled grouping of the releases that name an artist text and name no publisher feed, for example "Releases that name Ledbetter". The page says that a name is not an identity, and it never merges with a publisher page.

The artist name on a release page opens the publisher level when the release names its publisher feed ([ADR 0077](0077-publisher-feed-artist-binding.md) Decision 2). Otherwise it opens the name level. The "Publisher" link stays ([ADR 0082](0082-publisher-roles-belong-to-each-album-link.md) Decision 6).

The operator decided on 2026-10-08: when the agreed role of the link does not include artist, for example a label, the artist name opens the name level, and only the "Publisher" link opens the publisher page.

This replaces the Library artist page by name and the Index name match page.

### 8. Two Scopes

Releases and Artists have one scope control with two states: "Library" and "Everything". "Everything" adds the Index items, and each Library item keeps its state word. Tracks has no scope control and shows the Library only. Discover and search show everything, with the state word on each Library item.

This replaces the three-state control of [ADR 0062](0062-music-content-surface.md). No control shows Index items only.

### 9. One Navigation Stack And A Location Breadcrumb

The Music content area has one navigation stack. Each page change goes into it: a sidebar click, a row, a link and a search. Back and Forward always show in the chrome of the detail column.

The breadcrumb shows the location, not the history: the destination, then the path inside it, for example "Playlists › 000 › Tiddies In My Face (Single)".

- A page shows in the breadcrumb one time at most. A move to a page that is already in the path cuts the path at that page.
- A sidebar click starts a new path.

No screen keeps a second navigation stack or a breadcrumb of its own. This completes [ADR 0046](0046-workspace-frame-architecture.md) Invariant 2 for Music.

### 10. Multiple Selection Is One Shared Behavior

Each list of releases and tracks has the same selection model:

- A row shows a checkbox when the pointer is on it, when it has keyboard focus, or when a selection exists.
- Ctrl+click adds a row. Shift+click adds a range. Ctrl+A selects the list. Space changes the row with keyboard focus.
- A plain click opens the page of the row.
- With a selection, an action bar at the bottom of the list shows the count, the actions for the whole selection and "Clear". An example is "Download 7 tracks".

No step needs a drag or a scroll wheel. The selection and the availability of each bar action are view-model state.

### 11. Each Track Row Keeps A Place For Audition

Each track row and each release page keep a position for an audition control. The audition player of [ADR 0068](0068-show-cue-and-audition-isolation.md) fills it. Until then, no control shows in that position: a control that cannot act is absent.

### 12. The Logo Frames A Collection

As on `../musicindex/search.html`, the MusicIndex logo shape frames the art of a collection. A collection is a singles group, an artist page or a playlist.

- The frame holds a mosaic of up to four covers. Two or three covers repeat to fill four cells, as `search.html` does.
- With no cover, the frame shows the entity color.
- A collection with one cover shows that cover as a square. A release always shows its cover as a square.

The logo shape is a named token. The technique is a packet's choice, for example a mosaic image that the image cache composes and masks off the UI thread. The view model carries no renderer type.

## Consequences

Positive:

- One route reaches each item, so the Library state never decides where the operator must look.
- Back, Forward and the breadcrumb record each step.
- Multiple selection makes a batch download one action.
- Discover gives the first curator job a place to browse.

Negative:

- The change is large. The sidebar tree, the Index page routes, the name match page, the Library artist page by name and the second navigation stack go.
- The guards of the decisions that this ADR amends change with the packets. A guard that asserts a replaced rule is deleted with that rule, in the same change.
- Three columns need width. The collapsed sidebar recovers part of it.

## Alternatives Considered

- **A destinations sidebar with one content pane, as Apple Music.** It needs less width. Rejected by the operator, who preferred to keep the list in view.
- **A Music home first.** It shows upkeep and new music on the first screen. Rejected: Discover takes the browse job, and Needs attention takes the upkeep job.
- **Library and Index as two linked places.** Less change, but the operator still chooses a place first. Rejected.
- **Artists and publishers as two destinations.** Clear for the data, but two pages answer one question for most releases. Rejected.
- **A breadcrumb of history.** It repeats pages and does the work of Back. Rejected.

## Verification

Mechanical criteria, each at the layer that owns it:

- The Music view model exposes the destinations of Decision 2 in that order, and Discover is the default.
- A release opened from a Library row and from an Index row builds the same page view model, keyed by its feed GUID.
- The Releases list view model gives one group row for two or more singles of one artist identity, and no group for one single.
- The artist page view model gives the publisher level only for a publisher feed GUID, and labels the name level.
- The navigation model has one stack for the Music content area. A breadcrumb holds each page one time at most.
- The selection model supports add, range, all and clear, and gives the bar actions with typed availability.
- The kind of a release follows Decision 5.
- A guard blocks a second navigation stack in a Music screen.

Visual criteria, for the operator:

- The three columns, the collapsed sidebar, Discover, the singles group, the two artist levels and the logo frame, in Light and Dark, at normal and narrow width.
