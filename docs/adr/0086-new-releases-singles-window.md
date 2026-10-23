# ADR 0086: New Releases Groups The Singles Of One Artist By A Window

## Status

Accepted - 2026-10-23. The operator asked for this proposal and answered its first questions on 2026-10-22. The operator answered the four open questions and accepted the ADR on 2026-10-23 (see "Operator Answers").
The [ADR 0086 phase plan](../plans/adr-0086-new-releases-window-groups-phase-plan.md) holds its packets.

It amends [ADR 0084](0084-music-column-browser.md) Decision 3: New releases and the saved searches of Discover show window groups. ADR 0084 Decision 6 stays for the Releases list.
It amends [ADR 0062](0062-music-content-surface.md): the New releases pager can load more pages before it shows a page. The recency order stays.

## Context

On 2026-10-22 the operator examined New releases in the Tiles mode. The artist jSmovie filled more than 16 tiles, one tile for each single. Each tile showed "Single New". The two albums of other artists were in the first row only by chance. A curator cannot see other new music past one active artist.

ADR 0084 Decision 6 groups the singles of one artist in the Releases list. That group holds each single of the artist in the current scope. It has no time limit, so it is not correct for a list of new music.

These facts control the design. The orchestrator recorded them on 2026-10-22:

- `GET /v1/feeds/recent` orders the feeds by `COALESCE(newest_item_at, -1) DESC, feed_guid DESC` (`recent_feed_rows` in `../stophammer/src/query.rs`). The tile order of New releases is thus the order of `newest_item_at`.
- The app requests 20 feeds for each page (`PAGE_LIMIT` in `src/api.rs`). A group can continue on a page that the app did not load.
- `fetch_recent_feed_result_rows` in `src/application/queries/feed.rs` reads the detail of each of the 20 feeds of a page, one after the other. A page takes about 400 ms (overhaul plan, responsiveness item 10).
- A Library single has no `newest_item_at`. The Decision 6 groups use the latest `pub_date` of its tracks as its date (`GroupMember::date` in `src/view_models/singles_group.rs`).
- `SinglesGroupKey` in `src/view_models/singles_group.rs` is the key of the Decision 6 groups. It includes the "Matched by name" rule of the ADR 0084 amendment of 2026-10-22.
- The state word of a release that the Library does not hold is "New". The state word of a Library release is "In library" (ADR 0084 Decision 8).

## Decision

### 1. A Window Group Holds The Singles Of One Artist In One Window

A window group is a set of singles with one `SinglesGroupKey`. The singles of the group have their release date in one window. The release date of a MusicIndex single is its `newest_item_at`. The release date of a Library single is the latest `pub_date` of its tracks, as in the Decision 6 groups. The window length is the setting of Decision 7, 14 days by default.

The app makes the groups of one key from newest to oldest:

1. The newest single that is in no group starts a group. Its release date is the anchor of the group.
2. The group takes each single of the key whose release date is at most one window length before the anchor.
3. The app repeats step 1 until each single of the key is in a group.

Example: jSmovie has singles on Oct 20, Oct 15, Oct 8 and Oct 1. With a window of 14 days, group 1 holds Oct 20, Oct 15 and Oct 8. The single of Oct 1 starts group 2.

- A group with one single is not a group. It shows as the single, with the "Single" kind.
- Albums and EPs never join a group. They do not start, stop or break a window.
- A single with no release date joins no group.
- An artist thus shows at most one group tile for each window. An album of the same artist can show near it.

### 2. A Group Tile Takes The Place Of Its Newest Single

The group tile shows at the position of its anchor single. The other singles of the group show no tile of their own.

### 3. The Group Tile Shows The Count, The State And The Window

The group tile is a collection. Its art is the art of the anchor single in the logo frame of ADR 0084 Decision 12.

- **Title.** The artist name of the key.
- **Second line.** The date span of the window, from the oldest single to the anchor, for example "Oct 8 to Oct 20".
- **Kind badge.** "n Singles", for example "9 Singles", in place of "Single".
- **State word.** "New" when one or more singles of the group are not in the Library. Then a count follows, for example "5 in library". When the Library holds each single, the state word is "In library".

The List mode shows the same groups with the same facts in each row.

### 4. The Group Opens A Window Group Page

A click on a group tile opens a window group page. That page is one page of ADR 0084 Decision 4, and it goes into the navigation stack of Decision 9. The breadcrumb shows it, for example "Discover › New releases › jSmovie · 9 singles".

- The page lists the singles of the window, newest first.
- Its main action is "Download n singles" when the Library does not hold each single. Else its main action is "Add to playlist".
- It links to the Decision 6 group page of the key, for example "All 23 singles".

In selection mode, the checkbox of a group tile selects each single of the window (ADR 0084 Decision 10).

### 5. The Pager Loads Until Each Window Closes, Up To A Cap

A window is closed when the app has a row whose release date is older than the start of the window. The recency order then shows that no other single of the window exists.

After a page arrives, the app loads one more page while a window of a loaded group is open. It loads at most 5 extra pages, 100 feeds, for one page request. At most 4 feed detail reads of a page run at the same time, as in ADR 0084 packet 026. The extra pages thus add less delay. The pager keeps its loading state during the extra loads, and the tiles show when the loads stop.

When the cap stops the loads, each open group shows a "+" after its count, for example "13+ Singles". Its count grows in place when the next page arrives, and the "+" goes when its window closes. A single tile becomes a group tile in place when a later page adds a single to its window. The mounted view updates in place, as the current design philosophy requires.

### 6. Saved Searches Group By The Same Window

The saved search views of Discover make window groups with the same key and window as New releases. The app makes the windows from the release date of Decision 1, not from the relevance order.

A relevance order never closes a window. A saved search thus groups only its loaded rows. It loads no extra page for a window. Each group of a saved search that a later page can change shows the "+" of Decision 5. The operator decided this on 2026-10-23.

The group tile takes the position of its anchor single in the relevance order. The relevance order of each other tile does not change.

### 7. The Window Length Is A Setting

Settings has a value for the window length in days. The value is 1 to 60. The default is 14. No value turns the grouping off.

- The value is in `config.toml` with `#[serde(default)]`, so an older `config.toml` keeps loading.
- A change of the value makes the groups again in the mounted view. The curator does not move away and back.
- The Settings row uses the shared guarded editor of [ADR 0069](0069-grouped-settings-and-selective-presets.md) task 002.
- **Configuration exception.** ADR 0066 holds back each configuration-format change until its task 004 passes. The operator made one exception on 2026-10-23: this ADR adds one key with `#[serde(default)]`. The exception releases no other key, preset or format change.

## Invariants

- A single is in at most one window group.
- Each single of a window group has the same `SinglesGroupKey`.
- No two singles of one window group are more than one window length apart.
- Albums and EPs are never in a window group.
- The Releases list keeps the Decision 6 groups of ADR 0084, with no window.

## Non-Goals

- This ADR does not change the Releases list, the artist pages or the Decision 6 group page.
- This ADR does not request a grouped query from Stophammer.
- This ADR does not group albums or EPs.
- This ADR does not change the recency order of ADR 0062.

## Consequences

- An active artist takes one tile for each window, so the first screen of New releases shows more artists.
- The first tiles of New releases can show later, because the pager loads extra pages. The cap limits the delay to 5 more requests for each page.
- An open group can show a count with "+". The count can grow while the curator scrolls.
- A tile can change from a single to a group in place when a later page arrives.
- The app adds one page kind: the window group page.
- The app adds one Settings value.

## Alternatives Considered

- **Fixed calendar periods.** Two-week periods that start on a fixed date. Rejected: two singles two days apart can fall into two periods.
- **Only the newest group of each artist.** Rejected: older singles of an active artist do not show in New releases.
- **The count grows in place, with no extra loads.** Rejected by the operator on 2026-10-22: the count must be final when the tile shows. Only the cap of Decision 5 can stop the loads first.
- **Stophammer groups the singles.** Rejected: the feature then waits for a Stophammer release.
- **Library and new singles in two groups.** Rejected: the artist then shows two times in one window.
- **The group tile takes the place of its best-ranked single in a saved search.** Rejected by the operator on 2026-10-22: the anchor single sets the place in each view.
- **A fixed window length.** Rejected by the operator on 2026-10-22: the curator sets it.

## Operator Answers - 2026-10-23

1. **Saved search paging.** A saved search groups only its loaded rows. An open group shows the "+" of Decision 5 (Decision 6).
2. **Library rows in a saved search.** A Library single uses the latest `pub_date` of its tracks as its release date (Decision 1).
3. **Setting range.** 1 to 60 days, with no "off" value (Decision 7).
4. **The extra page cap.** The cap stays at 5 pages. Before the pager loads extra pages, the feed detail reads of a page run at most 4 at the same time (Decision 5).
5. **Configuration key.** One additive key is an exception to the ADR 0066 gate (Decision 7).

## Verification

### Mechanical Criteria

A unit test in the owner view model proves each criterion.

- The New releases page view model gives one group row for two or more singles of one `SinglesGroupKey` in one window. It gives no group row for one single.
- The view model makes the windows by the chain of Decision 1. Singles on Oct 20, Oct 15, Oct 8 and Oct 1 with a window of 14 days give one group of three and one single.
- The view model never puts an album or an EP in a group, and never puts a single with no `newest_item_at` in a group.
- The view model puts a group row at the position of its anchor single, in New releases and in a saved search.
- The group row exposes the artist name, the date span, the count, the state word and the Library count of Decision 3.
- The pager view model requests one more page while a window is open, and stops after 5 extra pages.
- The query of a New releases page runs at most 4 feed detail reads at the same time. It keeps the row order of the list response.
- A Library single takes the latest `pub_date` of its tracks as its release date.
- A saved search view model loads no extra page for a window, and an open group exposes the open state.
- After the cap, the group row exposes the open state, and the count changes when the next page arrives.
- The window group page view model lists the singles newest first. It gives the main action of Decision 4 and the link to the Decision 6 group.
- A config with no window value loads with 14 days. The Settings editor accepts 1 to 60, and rejects 0 and 61. A change of the value makes the groups again in the mounted page view model.

### Visual Criteria

A person inspects each criterion in the running app.

- New releases in Tiles and in List mode, with an artist that has many singles, such as jSmovie. Examine Light and Dark, at normal and narrow width.
- The group tile: the logo frame, the title, the date span, the "n Singles" badge and the state word.
- The "+" of an open group, and its change when the next page loads.
- The loading state while the pager loads extra pages.
- The window group page, its breadcrumb and the link to the full group.
- The Settings row of the window length, and the change of New releases after a change of the value.
