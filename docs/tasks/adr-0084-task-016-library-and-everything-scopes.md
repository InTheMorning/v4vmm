# ADR 0084 Task 016: The Library And Everything Scopes

Status: Implemented - 2026-10-21, Parts A and B. Mechanical checks are Green. V1 to V8 are open, in the final walk of the run.

## Goal

The Releases list and the Artists list get one scope control with two states: "Library" and "Everything".
"Everything" adds the MusicIndex items to the Library items, in one name order. Each Library item keeps its state word.
Tracks has no scope control.

Part A replaces the stored contract with the Stophammer 0.12.0 contract and adds the decode, the requests and the scope control. It gives the Artists list its "Everything" scope.
Part B gives the Releases list its "Everything" scope, with selection and download for MusicIndex rows.

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md):
  - Decision 8: the two scopes. Decision 2: the destinations.
  - Decision 4: one page for each item, and the state word.
  - Decision 6: the singles groups. Decision 7: the two artist levels.
  - ADR 0084 Decision 7 amended on 2026-10-20: the key of a name is its sort key. A name that matches exactly one artist feed has no row of its own. Packet 017 gives the match function.
  - Decision 10: the selection model and the download queue.
- [ADR 0085](../adr/0085-artists-by-confirmation.md) Decision 2: the three confirmation levels. Decision 6: the artist feed of an album. A label feed is not an artist.
- [ADR 0062](../adr/0062-music-content-surface.md): paging. ADR 0084 replaces its three-state control.
- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) section 6: named request profiles, no detail read for each list row, and a defined refresh. [AGENTS.md](../../AGENTS.md) "Stored facts": replace the stored contract in the same change as the decode.
- [ADR 0040](../adr/README.md): background work runs on the async command runner. No screen calls `cx.spawn`.
- [Open Stophammer requests](../plans/v4vmm-open-requests.md): request 6, request 8 and "Release 0.12.0 - 2026-10-18". Stophammer ADR 0070 and ADR 0071 own the two name lists.
- The [phase plan](../plans/adr-0084-music-column-browser-phase-plan.md): the packet 016 row, and the open finding of 2026-10-18 on ADR 0070 §4 and ADR 0085 Decision 6.

## Recorded Facts - 2026-10-20

### Code facts

- `LibraryViewModel::releases_list` in `src/view_models/library.rs` builds the Releases entries from the Library tree only. It sorts each entry by `label().to_lowercase()`. `releases_list_with_selection` adds the checkbox facts.
- `destination_release_row` in `src/view_models/library.rs` gives the secondary text "artist · Kind". No Releases row shows a Library state word today.
- `LibraryViewModel::artists_list` groups each Library album by `AlbumNode::artist_identity`. `artist_list_row` gives a feed row "n releases" and a name row "n releases · by name" (`ARTIST_NAME_ROW_MARK`). It sorts by `label.to_lowercase()`.
- `DestinationListRowTarget` has `Release(ReleaseKey)`, `Artist(String)` and `Publisher(String)`. `LibraryApp::open_destination_row` in `src/library/app_impl.rs` maps them to navigation entries.
- `ArtistIdentity::resolve` in `src/view_models/artist_identity.rs` calls `select_artist_feed` in `src/view_models/named_feeds.rs`. The unverified rule reads `NamedFeed::confirmed`, which is `two_way_validated` of the link.
- `singles_groups` in `src/view_models/singles_group.rs` groups Library singles only. It borrows `ArtistNode` and `AlbumNode` from the `LibraryTree`. The guard `adr_0084_singles_group_r15_09_only_the_owner_groups_singles` keeps `ReleaseKind::Single` in this file and in `src/view_models/release_kind.rs`.
- `ReleaseKind::resolve` in `src/view_models/release_kind.rs` reads "unknown" as not stated, then uses the track count.
- `list_column_header_action` in `src/view_models/library.rs` gives "Select" and "Done" for Releases, Tracks and Needs attention. `LibraryApp::header_action_for_list_column` reads it, and `TopApp::render` in `src/app.rs` passes it to `content_list_header_action`.
- `render_frame_chrome` in `src/ui/composites/frame_shell.rs` has a filter row under the header (`frame_filter_row`). Today only the Tracks view-mode control uses it. `SegmentedControl` in `src/ui/composites/segmented_control.rs` is the shared segmented control.
- The guard `adr_0084_discover_r6_06_three_state_filter_control_is_deleted` forbids the names `LibraryFilterControlDisplay` and `library_filter_control` in `src`.
- `Feed` in `src/api.rs` decodes no `publisher_feed_guid`, no `named_feeds` and no `sort_name`. `src/api.rs` has no type for an artist name row, and `Client` has no name list method.
- `Client::fetch_recent_feeds` in `src/api.rs` sends `limit` and `cursor` only. `PAGE_LIMIT` is 20.
- `fetch_recent_feed_result_rows` in `src/application/queries/feed.rs` reads the full detail of each release on a Discover page. The overhaul plan records this as responsiveness item 10.
- `FetchRecentFeedsPage` runs through `present_command` from `LibraryApp::start_recent_music_load`. `RecentFeedsPageVm` in `src/view_models/recent_feeds.rs` has a `loading` flag and no request generation.
- `present_command` in `src/presentation/async_command_presenter.rs` drops a result only when the session generation changes. A page-level stale check is the caller's job, as `publisher_page_result_is_current` in `src/app/publisher_dispatch.rs` does.
- `src/application/request_profiles.rs` has four `RequestPathShape` values and eight registry profiles. The test `adr_0075_request_profile_registry_names_eight_profiles` counts them.
- `MUSICINDEX_CONTRACT_FIXTURE` in `tests/architecture_tests.rs` names `tests/fixtures/musicindex-openapi-0.11.0.json`. `CONTRACT_TYPE_MAP` maps each decoded type to its contract schema. AGENTS.md "Stored facts" names the same file.
- `LibraryViewModel::release_is_fully_downloaded` gives `true` for a key that the tree does not hold. `releases_selection_action_bar` thus counts a MusicIndex key as in the Library.
- `LibraryApp::run_releases_selection_action` downloads Library feeds only. `TopApp::download_detail_selection` in `src/app/detail_selection.rs` queues a MusicIndex release with `download_index_album_of_track`. `DownloadJob::IndexRelease` with `feed: None` reads the feed in its own queue slot (`src/library/downloads.rs`).
- `SelectionActionBarVm::from_counts` in `src/view_models/selection_action_bar.rs` gives each action its own count (packet 018 R18-06).
- `TopApp::open_publisher_page` takes a `PublisherPageContext`. A restore with no recorded context uses the Library query (`restore_publisher_page_for_nav`).
- `FrameNavigationEntry::IndexNameMatches(String)` opens the MusicIndex name match page (`src/app/name_match_dispatch.rs`). Packet 017 replaces it.
- `LibraryApp::show_destination_list` clears both selections and ends selection mode on each destination change.

### Live API facts

The packet author sent 12 read-only GET requests to `https://api.musicindex.org` on 2026-10-20, from 23:19 UTC.

- `GET /node/info` gives `git_revision` `056199b` and `built_at` `2026-10-18T23:49:01Z`. `GET /v1/node/info` gives `404`.
- `GET /openapi.json` gives `info.version` `0.12.0`, 100,712 bytes. This is the source of the new stored contract.
- The difference from the stored 0.11.0 contract:
  - New schema `NamedFeedEntry`: `publisher_feed_guid` (required), `remote_feed_title`, `album_names_rank`, `role` and `role_agreement`.
  - `ArtistNameRow` adds `agreed_roles`, `confirmed_release_artists`, `first_named_count`, `stated_rels` and `two_way_link_count`.
  - `FeedResponse` adds `first_named_count`, `named_feeds` and `publisher_feed_guid`.
  - No new path. The 0.11.0 contract already declares `/v1/feeds/by-name` and `/v1/artist-names`.
- `GET /v1/feeds/by-name` takes `cursor`, `limit`, `include` (`link_facts`, `named_feeds`), `medium` (default `music`), `from` and `artist_key`. `GET /v1/artist-names` takes `cursor`, `limit`, `include` (`link_facts`) and `from`. On both, `limit` is 1 to 200.
- `GET /v1/node/capabilities` lists `name_lists`, the entity type `publisher`, `include_params.feed_list` `["link_facts", "named_feeds"]` and `include_params.artist_list` `["link_facts"]`.
- `GET /v1/artist-names?include=link_facts&limit=200`, the first page:
  - 174 `publisher` rows and 26 `name` rows. `has_more` is true.
  - No publisher row has an agreed role. 169 publisher rows have a `first_named_count` above 0, and 5 have 0. These counts are the same as on 2026-10-18.
  - A `name` row gives `kind`, `name`, `sort_name`, `key`, `feed_guid` null, `image_url` null and `release_count`. Example: "$2 Holla", key "$2 holla", `release_count` 3.
- `GET /v1/feeds/by-name?include=named_feeds&limit=200`, the first page:
  - 200 music rows. 144 rows have `episode_count` 1. Each row gives `release_kind` "unknown".
  - 188 rows give `named_feeds`, each with one entry at rank 1. Each entry has `role` null and `role_agreement` null.
  - 186 rows give `publisher_feed_guid`, and it is the GUID of rank 1 on each of them.
  - The other 2 rows name a feed that the node does not hold. The entry has `remote_feed_title` null, and the row has `publisher_feed_guid` null. Examples: "#LIII4PRESIDENT" (LIIINEKELA) and "1 Take First Freestyle (No Edit)" (jSmovie).
- **Gap.** A `NamedFeedEntry` gives no `two_way_validated`. Stophammer ADR 0071 §3 gives one entry for each `music_to_publisher` row, so an entry can be a link that only the album states. The two entries with no held feed prove this. ADR 0085 Decision 6 needs the confirmation of the primary link when no link agrees on `artist`.
- Sir Libre Records (`34bf795f-f551-4db1-8877-ad19aa5f89c8`): `agreed_roles` `["label"]`, `two_way_link_count` 3, `first_named_count` 0. The `name` row "Sir Libre" has `release_count` 1.
- "Motive to Kill" names Motive to Kill at rank 1 (`artist`, `both`) and Sir Libre Records at rank 2 (`label`, `both`).
- TJ Wong (`a31f362c-d40b-5f56-ae2c-4ef9c548db9d`): `two_way_link_count` 0, `first_named_count` 0, `agreed_roles` empty. Its feed lists 6 releases of Timothy James. Each link has `two_way_validated` false and `album_names_as` null.

### Two rule differences between Stophammer and ADR 0085

Stophammer ADR 0070 §4 puts a release on the name level in two conditions. No item names a held feed, or each link agrees on a role set without `artist`. ADR 0085 Decision 6 uses the first named feed and its confirmation.

- **D-1, recorded on 2026-10-18.** An album names an agreed label first and a feed with no role second.
  - Stophammer: not on the name level. v4vmm: name level. Stophammer ADR 0071 declined item 6 of request 8.
  - On 2026-10-18, no album did this. On 2026-10-20, Sir Libre Records gives `first_named_count` 0, so it gives no such album.
  - The full check of the Checks section did not run on 2026-10-20.
- **D-2, found on 2026-10-20.** An album names a held feed first, through a link that only the album states, with no agreed role.
  - Stophammer: not on the name level, so no `name` row counts it. v4vmm: name level.
  - The feed row is in Artists only when other albums give it a `first_named_count` above 0.
  - A check needs the gap field above.

**The stored contract (orchestrator decision of 2026-10-20).** Packets 010, 016 and 017 each need the 0.12.0 contract. The first of the three that an implementer starts replaces `tests/fixtures/musicindex-openapi-0.11.0.json` with `musicindex-openapi-0.12.0.json`, with the guard constant and the `AGENTS.md` line. A later packet finds the 0.12.0 file and changes only its own decode.

## Design

### The scope

- `MusicListScope` has two values: `Library` and `Everything`. The default is `Library`. Releases and Artists each keep their own value. Tracks, Needs attention, Playlists and Discover have no scope.
- `MusicScopeControlDisplay` gives the control id, the selected value, and the two options with a visible label and an accessibility label. The view model gives it only for Releases and Artists.
- The control is a `SegmentedControl` with the `Filter` style. It sits at the leading edge of the frame filter row of the list column. "Select" stays in the header.
- A change of scope on Releases clears the Releases selection and ends selection mode.

### The MusicIndex requests

- `Client::fetch_feeds_by_name(limit, cursor, profile)` and `Client::fetch_artist_names(limit, cursor, profile)` in `src/api.rs`. The profile gives the `include` value.
- Two profiles in `src/application/request_profiles.rs`: `INDEX_RELEASE_NAME_LIST` (`RequestPathShape::FeedNameList`, `named_feeds`) and `INDEX_ARTIST_NAME_LIST` (`RequestPathShape::ArtistNameList`, `link_facts`).
- `NAME_LIST_PAGE_LIMIT` is 100, a named constant.
- Two commands in a new `src/application/queries/name_lists.rs`: `FetchReleaseNamePage` and `FetchArtistNamePage`. Each sends one request for each page and no detail request.
- The decode adds `ArtistNamesResponse`, `ArtistNameRow`, `FeedNamesResponse` and `NamedFeedEntry` to `src/api.rs`. `Feed` adds `publisher_feed_guid`, `named_feeds` and `sort_name`.

### The page state

- `IndexNameListPage<Row>` in a new `src/view_models/index_name_list.rs`: the loaded rows, the cursor, `has_more`, the load state and a request generation.
- `begin_load(append)` gives an intent with the cursor and the generation. A fresh load adds 1 to the generation.
- `finish_load(batch, generation)` and `fail_load(error, generation)` do nothing when the generation is old.
- The loaded pages stay until the app stops. No page expires. "Try again" after an error is the only new load of page 1.

### The sort key and the merge

- `name_sort_key(text)` in `src/view_models/index_name_list.rs` follows Stophammer ADR 0070 §1. It removes white space at the two ends, makes each internal run one space, and makes each character Unicode lowercase. Rust `String` order is code point order.
- The Library lists use this key in both scopes.
- Releases order: the sort key, then the feed GUID. A Library feed with no feed GUID comes after equal keys, by its feed id.
- Artists order: the sort key, then the kind (`name` before `publisher`, as Stophammer orders), then the key.
- **The watermark.** While `has_more` is true, the merged list shows only the rows whose order position is at or before the last loaded MusicIndex row. The other Library rows show when their page loads. When `has_more` is false, the list shows each row.
- The merge is a pure function. It runs at each render from the Library tree and the loaded pages. A download that adds a release to the Library thus changes its row in place.

### The Artists list in "Everything"

A MusicIndex `publisher` row is in the list by ADR 0085 Decision 2:

| Row facts | Result |
|---|---|
| `agreed_roles` holds `artist` (read by `role_contains_artist`) | Shown. Artist |
| `agreed_roles` is empty, and `first_named_count` is above 0 | Shown. Unverified artist |
| `agreed_roles` is not empty and holds no `artist` | Absent. A label or another role |
| `agreed_roles` is empty, and `first_named_count` is 0 or null | Absent. No confirmed link names the feed first |

- Each MusicIndex `name` row is shown, unless the match rule below removes it. It is the name level.
- A MusicIndex feed row and a Library feed row with the same feed GUID give one row. A MusicIndex name row and each Library name row with the same sort key give one row. The Library row keeps its label, its art and its target.
- The match rule (ADR 0084 Decision 7 amended on 2026-10-20, packet 017 decision D14): `LibraryViewModel::name_feed_matches` reads the Library tree only.
  - A Library or MusicIndex name row whose key gives `MatchedFeed(guid)` gives no row. Its Library releases count in the Library feed row of `guid`.
  - A MusicIndex name row whose key gives `NoFeed` or `Ambiguous` stays.
  - The match does not change with the loaded MusicIndex pages. A shown row thus never goes when a page loads (D7).
- No row shows a level word.

Row text in "Everything":

| Row | Secondary text | Opens |
|---|---|---|
| Library feed row | "3 in Library". The count includes the Library releases of each matched name | The feed page, as today |
| MusicIndex feed row | "Not in Library" | The feed page, with `PublisherPageContext::Index` |
| Library name row | "3 in Library · by name" | The Library artist page by name, as today |
| MusicIndex name row | "Not in Library · by name" | `FrameNavigationEntry::IndexNameMatches(name)`, until packet 017 |

In the "Library" scope, each row text stays as it is today.
A MusicIndex feed row shows its `image_url` as its provided image (ADR 0084 Decision 12). A MusicIndex name row shows the entity color.

### The Releases list in "Everything"

- A MusicIndex row and a Library release with the same feed GUID give one row: the Library row.
- A MusicIndex row shows the title, then "release_artist · Kind". The kind is `ReleaseKind::resolve(release_kind, episode_count)`. The thumbnail is `image_url`. It opens `ReleaseKey::FeedGuid`, as a Discover row does.
- A Library row adds its state word: "In Library", "3 of 7 tracks" or "Not downloaded". In the "Library" scope, the row text stays as it is today.
- The singles groups stay Library groups (decision D4). A MusicIndex single is a release row.
- Selection: a MusicIndex row has its checkbox, its focus stop and its range position, as each other row.
- The bar counts follow packet 018 R18-06. A MusicIndex release counts in "Download" only. A Library release counts in "Add to playlist" and "Remove".
- "Download" queues one `DownloadJob::IndexRelease` with `feed: None` for each MusicIndex release, and the present Library jobs for each Library release.

### The list states

- While page 1 loads, the list shows one loading row, "Loading MusicIndex releases" or "Loading MusicIndex artists", and no item row.
- After an error on page 1, the list shows the Library rows and one error row. Example: "MusicIndex did not answer the release list request. The list shows Library releases only." The detail names `GET /v1/feeds/by-name`. The row has a typed "Try again" action.
- After an error on a later page, the loaded rows stay, and the error row replaces "Load more".
- While `has_more` is true, the last row is "Load more", with the accessibility label "Load more MusicIndex releases" or "Load more MusicIndex artists".

## Orchestrator Decisions

FLAG marks a point that changes what the operator sees in a way that no ADR decides.

- **D1. The place of the control.** The control sits at the leading edge of the frame filter row of the list column, under the header. "Select" stays at the trailing edge of the header. The list column is narrow, and the header holds the title and "Select". FLAG.
- **D2. The labels.** "Library" and "Everything", from ADR 0084 Decision 8. The accessibility labels are "Show Library releases only", "Show Library and MusicIndex releases", "Show Library artists only" and "Show Library and MusicIndex artists".
- **D3. The scope persistence.** Releases and Artists each keep their scope in `LibraryApp` memory.
  - Each app start uses "Library". The config file does not store it.
  - An open of Music thus starts no name list request. FLAG, minor.
- **D4. MusicIndex singles form no group in this packet.** A MusicIndex single shows as a release row. A Library single keeps its Library group. The MusicIndex copy of a grouped Library single shows no second row. FLAG.
  - This changes the packet scope, and ADR 0084 Decision 6 asks for groups in the current scope. Two reasons:
  1. A `named_feeds` entry gives no `two_way_validated` (Live API facts). On 2026-10-20, each named feed on the first 200 releases has no agreed role. ADR 0085 Decision 6 thus needs the link confirmation, and the app cannot read it from the row. A group key from that row would make an artist from an album-only link, which ADR 0085 forbids.
  2. The release list pages in title order. The singles of one artist spread across the whole list, so a group from the loaded pages is not complete until the last page loads.
  A later packet, proposed as packet 026, adds MusicIndex singles to groups. It waits for the gap field and for a way to list the releases that name one feed (for example, a filter on `GET /v1/feeds/by-name`). The orchestrator bundles these into one Stophammer request, after the audit of the remaining packets.
- **D5. The merge key.** Releases merge by `ReleaseKey::FeedGuid`. A Library feed with no feed GUID never merges. Artists merge feed rows by feed GUID and name rows by sort key. The Library row wins, with its text, art and target (ADR 0084 Decisions 4 and 8).
- **D6. The order.** Name order by the Stophammer sort key, in both scopes. The operator chose name order on 2026-10-17 (request 6). No recency order applies to these two lists. In the "Library" scope, the order changes only for names with extra white space.
- **D7. Paging and the watermark.** 100 rows for each page, and a "Load more" row, as on Discover. A Library row after the last loaded MusicIndex row waits for its page. The rows thus never move after they show.
  - FLAG: in "Everything", a Library row near the end of the alphabet shows only after more pages load.
- **D8. The load states.** Page 1 shows a loading row and no item row. An error shows the Library rows, the report and "Try again" (AGENTS.md "Write reports for the operator").
- **D9. Freshness.** The loaded pages stay until the app stops, with no expiry. "Try again" is the only reload. This is the refresh rule that ADR 0075 section 6 asks for.
  - A later packet can add a reload. FLAG, minor.
- **D10. Stale results.** Each list keeps a request generation. A result of an older generation changes nothing.
- **D11. The artist classification.** The table of "The Artists list in Everything", from ADR 0085 Decision 2. A label feed such as Sir Libre Records is absent. No row shows a level word.
- **D12. The Artists row text in "Everything".** "3 in Library", "Not in Library" and the mark "by name". A MusicIndex feed row gives no release count, because `ArtistNameRow` gives none for a feed. A MusicIndex name row does not show its `release_count`. FLAG.
  - ADR 0084 Decision 7 amended on 2026-10-20: a name row whose key matches exactly one Library feed is absent. The count "n in Library" of that feed row includes the Library releases of the matched name. The `release_count` of a matched MusicIndex name row shows on no row.
- **D13. The Releases state word in "Everything" only.** Replaced on 2026-10-22 by packet 033 D2: each release row shows its download state icon in each scope (ADR 0083 Decision 6, amended on 2026-10-22). A Library row adds "In Library", "3 of 7 tracks" or "Not downloaded". A MusicIndex row adds no word. In the "Library" scope, each row is a Library item, and the word would repeat on each row, as ADR 0085 rejected for level words. FLAG.
- **D14. The targets of MusicIndex rows.** A feed row opens the feed page with the MusicIndex context. A name row opens the present MusicIndex name match page until packet 017 replaces it. FLAG: this page goes in packet 017.
- **D15. Selection and download.** MusicIndex rows in Releases are selectable. The bar counts follow packet 018 R18-06.
  - "Add to playlist" and "Remove" act on Library releases only. "Download" queues `IndexRelease` jobs.
  - Ctrl+A selects the shown rows only. A scope change clears the selection.
- **D16. No detail read for each row.** A name list page sends one request. It never reads a release detail for a row (ADR 0075 section 6, responsiveness item 10).
- **D17. The contract fixture.** Part A replaces `tests/fixtures/musicindex-openapi-0.11.0.json` with `tests/fixtures/musicindex-openapi-0.12.0.json`, from `GET /openapi.json`. The same change updates `MUSICINDEX_CONTRACT_FIXTURE`, `CONTRACT_TYPE_MAP` and the AGENTS.md "Stored facts" line.
- **D18. The two rule differences.** D-1 and D-2 stay as recorded.
  - Releases in "Everything" shows a D-1 or D-2 release as a release row. Artists in "Everything" can show no row for it, unless the Library holds it.
  - The Checks section gives the D-1 check after each Stophammer release. The D-2 check waits for the gap field.
  - FLAG: the Stophammer request of D4 is an operator decision.

## Required Changes

### Part A: the contract, the requests, the control and Artists

1. Replace the stored contract (D17). Delete the 0.11.0 file. Add the 0.12.0 file with the bytes of `GET /openapi.json`. Update the guard constant, its comment, `CONTRACT_TYPE_MAP` and AGENTS.md.
2. Add the decode types and the three `Feed` fields to `src/api.rs`. Add the two `Client` methods.
3. Add the two path shapes and the two profiles. Change the registry test to ten profiles.
4. Add `src/application/queries/name_lists.rs` with the two commands.
5. Add `src/view_models/index_name_list.rs`: `IndexNameListPage`, `name_sort_key`, the watermark, `MusicListScope` and `MusicScopeControlDisplay`.
6. Change `artists_list` to take the scope and the artist page. Add the classification and the merge. Use `name_sort_key` in both scopes.
7. Add a field to `DestinationListRowTarget::Publisher` for the page context, or a new variant. The MusicIndex name row target opens `IndexNameMatches`.
8. `LibraryApp`: keep the scope of each destination and the artist page. Start a page load through `present_command` when Artists shows "Everything" with no page. Handle "Load more" and "Try again".
9. `TopApp::render`: pass the scope control to the frame filter row of the list column for Releases and Artists. Add the slot and its handler to `src/ui/shells/workspace.rs` and `src/ui/composites/frame_shell.rs`.
10. Render the loading row, the error row and "Load more" in `src/ui/shells/library/destination_lists.rs`, with named tokens.

### Part B: Releases

11. Add `FetchReleaseNamePage` wiring to `LibraryApp`, as step 8.
12. Change `releases_list` and `releases_list_with_selection` to take the scope and the release page. Add the merge, the watermark and the state word.
13. Keep `singles_groups` a Library function (D4). Remove a MusicIndex row whose feed GUID is a grouped Library single.
14. Change the Releases bar counts to the R18-06 rule. A key that the tree does not hold counts as a MusicIndex release.
15. Add a pure plan of the "Download" action: the Library feed ids and the MusicIndex feed GUIDs with titles. `run_releases_selection_action` queues the MusicIndex jobs through the download queue.
16. A scope change on Releases clears the selection and ends selection mode.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_scopes_` for unit tests beside the owning code. Each test uses a recorded row of 2026-10-20 or a constructed value. No test sends a request to `api.musicindex.org`. A request test uses a local fixture server, as the `Fixture` of `src/application/queries/feed.rs` does.

| Case | Part | Required proof |
|---|---|---|
| R16-01 | A | `ArtistNameRow` decodes the recorded "0$" publisher row and the "$2 Holla" name row with each field. The Sir Libre Records row decodes `agreed_roles` `["label"]` and `first_named_count` 0 |
| R16-02 | A | `Feed` decodes a recorded by-name row with `named_feeds`, `publisher_feed_guid` and `sort_name`. A `NamedFeedEntry` with `remote_feed_title` null decodes |
| R16-03 | A | The contract guard names the 0.12.0 file and maps each new decoded type. The 0.11.0 file is absent. The guard passes |
| R16-04 | A | The two profiles name their path shape and include list. The registry test counts ten profiles |
| R16-05 | A | Each command sends one request for a page, with `limit=100`, the profile include and the cursor. It sends no other request |
| R16-06 | A | `name_sort_key` gives "$2 holla" for "$2 Holla" and "the ohm" for "  The   Ohm ". It keeps "The" and the punctuation |
| R16-07 | A | The page state ignores a finish and a failure of an older generation. An append keeps the rows. `has_more` and the cursor follow the batch |
| R16-08 | A | The artist classification gives each row of the D11 table. No row text holds "Artist", "Unverified" or "publisher" |
| R16-09 | A | The Artists merge gives one row for a Library feed and a MusicIndex row of the same GUID. It also gives one row for name rows with the same sort key. The order follows D6. The watermark follows D7 |
| R16-10 | A | The Artists row text and targets follow the D12 and D14 tables in "Everything". In "Library", each row equals the present row |
| R16-11 | A | The scope control display exists for Releases and Artists only, with the D2 labels. The default is `Library`. No name in the r6_06 guard list is used |
| R16-12 | A | Page 1 in flight gives the loading state and no item row. A page 1 error gives the Library rows and the error row with "Try again" available |
| R16-13 | B | The Releases merge gives one Library row for a Library release and a MusicIndex row of the same GUID. A `LibraryFeed` key never merges. The order and the watermark follow D6 and D7 |
| R16-14 | B | A Library row in "Everything" gives "In Library", "3 of 7 tracks" or "Not downloaded". A MusicIndex row gives "release_artist · Kind" with no state word |
| R16-15 | B | A MusicIndex single is a release row. The MusicIndex copy of a grouped Library single gives no second row. The packet 015 cases pass |
| R16-16 | B | The Releases bar counts a MusicIndex release in "Download" only, and a Library release in "Add to playlist" and "Remove" |
| R16-17 | B | The "Download" plan gives the Library feed ids and the MusicIndex feed GUIDs with their titles, with no duplicate |
| R16-18 | B | A scope change clears the selection and ends selection mode. Ctrl+A selects the shown rows only |
| R16-19 | B | The same loaded page with a new tree that holds the release gives a Library row, with no new request |
| R16-20 | B | Each case of packets 009, 015, 018, 019 and 022 still passes |
| R16-21 | A | In "Everything", a Library name row and a MusicIndex name row whose key gives `MatchedFeed` give no row. The Library feed row count includes the matched Library releases. A `NoFeed` or `Ambiguous` MusicIndex name row stays. When packet 017 comes second, packet 017 adds this case |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: Releases and Artists show "Library" and "Everything" under the list header. Tracks, Needs attention, Playlists and Discover show no scope control.
- V2: Artists in "Everything" shows MusicIndex artists in name order. Sir Libre Records is absent. Each Library artist shows "n in Library". When packet 017 is complete, no "Haleen · by name" row shows (ADR 0084 Decision 7 amended on 2026-10-20).
- V3: a MusicIndex feed row opens its feed page. A MusicIndex name row opens the name match page.
- V4: Releases in "Everything" shows MusicIndex releases with "artist · Kind". Library releases show their state word. One release shows one time.
- V5: "Load more" adds rows at the end. No row moves after it shows.
- V6: select two MusicIndex releases and click "Download". The progress item shows them. After the download, each row changes to its Library row in place.
- V7: with the network off, "Everything" shows the Library rows and the error report with "Try again".
- V8: Light and Dark themes, at normal and narrow width. The control and the row text are not clipped.

## Exclusions

- The name level page: packet 017.
- Search, and the search entry of a feed page: packet 010.
- MusicIndex singles in groups: proposed packet 026 (D4).
- A jump to a letter with `from`.
- A reload or an expiry of the loaded pages (D9).
- A scope on Tracks, Needs attention, Playlists or Discover.
- A Labels destination (ADR 0085 Non-Goals).
- A change to the D-1 and D-2 rules.
- No schema change and no stored MusicIndex row.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/api.rs`, `src/application/request_profiles.rs`, `src/application/queries/feed.rs`, `src/application/queries/search.rs`.
- `src/view_models/library.rs` (`releases_list`, `releases_list_with_selection`, `destination_release_row`, `artist_list_row`, `artists_list`, `list_column_header_action`, `releases_selection_action_bar`).
- `src/view_models/singles_group.rs`, `src/view_models/artist_identity.rs`, `src/view_models/named_feeds.rs`, `src/view_models/role_words.rs`, `src/view_models/release_kind.rs`, `src/view_models/recent_feeds.rs`.
- `src/view_models/selection_action_bar.rs`, `src/view_models/detail_selection.rs`, `src/view_models/workspace/chrome.rs`, `src/view_models/workspace/nav.rs`.
- `src/library.rs`, `src/library/app_impl.rs`, `src/library/downloads.rs`.
- `src/app.rs` (the list column slots), `src/app/publisher_dispatch.rs`, `src/app/name_match_dispatch.rs`, `src/app/detail_selection.rs`.
- `src/ui/shells/library/destination_lists.rs`, `src/ui/shells/workspace.rs`, `src/ui/composites/frame_shell.rs`, `src/ui/composites/segmented_control.rs`.
- `tests/architecture_tests.rs`: the contract guard, `adr_0084_discover_r6_06_three_state_filter_control_is_deleted` and `adr_0084_singles_group_r15_09_only_the_owner_groups_singles`.
- The archived packets 015, 018, 019 and 020, and packet 022.

## Checks

```bash
cargo test --lib adr_0084_scopes_
cargo test --lib adr_0075_request_profile_
cargo test --lib adr_0084_singles_group_
cargo test --lib adr_0084_selection_
cargo test --lib adr_0084_download_queue_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

### The check after each Stophammer release

A person or an agent does this check after each Stophammer release. It sends only read-only GET requests.

1. Read `GET /node/info` and `GET /openapi.json`. Record `git_revision` and `info.version`.
2. When the version changed, replace the stored contract in the same change as the decode changes (AGENTS.md "Stored facts").
3. Read `NamedFeedEntry` in the contract. When it gives `two_way_validated`, tell the orchestrator. Packet 026 and the D-2 check can then start.
4. Read each page of `GET /v1/artist-names?include=link_facts&limit=200`. Keep each `publisher` row with an `agreed_roles` that is not empty and holds no `artist`.
5. For each kept feed with a `first_named_count` above 0, read `GET /v1/feeds/{guid}?include=publisher`. Keep each album with `album_names_rank` 1 and `role_agreement` `both`.
6. For each kept album, read `GET /v1/feeds/{album guid}?include=publisher`. A D-1 album names another feed at a higher rank with `role` null.
7. Record the date, the revision and each D-1 album in the phase plan open finding. When the count is above 0, tell the orchestrator.

## Rollback

Revert the commit of each part, Part B first. The revert of Part A restores the 0.11.0 contract file. This packet adds no migration and no stored data.

## Size

Large. Part A changes about 15 files, with about 1,100 lines with tests. Part B changes about 8 files, with about 800 lines with tests. Each part runs in its own session. Each part leaves no code that no composition root reaches.

## Operator Answers - 2026-10-20

- D4: the operator agreed on 2026-10-20 that Index singles form no group in this packet. Packet 026 waits for Stophammer request 9.
- D12: confirmed as written.
- D13: confirmed: the state word shows in "Everything" only, except a partial state such as "3 of 7 tracks", which also shows in "Library".
- D18: the orchestrator drafts the bundled Stophammer request 9. The operator reviews it before Stophammer reads it.
- D14: packet 017 replaces the name match page. If packet 017 comes first, an Index name row opens the name level page.

## Orchestrator Review Of Part A - 2026-10-21

Pass. The code needs no fix.

- The release list request moves to Part B, because Part A has no caller for it and the guards forbid a `dead_code` attribute.
- The Releases list shows the scope control before Part B makes "Everything" act. AGENTS.md says that a control that cannot act is absent. Part B starts at once after this commit, so the orchestrator accepts the short gap. No operator walk runs on this commit alone.
- The orchestrator added the new files to the source map.

## Orchestrator Review Of Part B - 2026-10-21

Pass. The code needs no fix.

- The deviations agree with the packet. "Add to playlist" now counts Library releases only, the rule of packet 018 R18-06.
- The repeated merge in one render is responsiveness item 16 of the overhaul plan.
- A Library reload while page 1 loads clears the Releases selection. The orchestrator accepts this, because no item row shows at that time.

## Prompt for lower-context coding model

You are implementing one bounded part of a larger plan.

Implement only the part that the orchestrator names: Part A or Part B. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0084-task-016-library-and-everything-scopes.md`
- ADR 0084 Decisions 4, 6, 7, 8 and 10, and ADR 0085 Decisions 2 and 6
- Each file in "Files To Inspect"

Goal:
- Part A: make Required Changes 1 to 10.
- Part B: make Required Changes 11 to 16. Part A must be complete.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Follow each Orchestrator Decision. Do not decide a flagged point again.
- The view model gives each text, count, state and availability. The screen only composes. Use named tokens.
- Never call `cx.spawn` from a screen. Use `present_command`.
- A list page sends one request. It reads no release detail for a row.
- Tests use recorded rows. No test sends a request to `api.musicindex.org`.
- Get the 0.12.0 contract with one GET request to `https://api.musicindex.org/openapi.json`. Send no other request.
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git add` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- `singles_groups` grouping rules, the download queue rules, and the release page assembly.
- The database schema and the migration registry.
- Any ADR. Each document other than this packet and the one AGENTS.md "Stored facts" line.

Acceptance criteria:
- Each case of the part has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result" section for the part, with the date, and an "Operator Visual Check" section for its V items. Part A owns V1 to V3, V7 and V8 for Artists. Part B owns V4 to V8 for Releases.

Test commands:
- The commands in "Checks", with `cargo fmt -- --check` after the last edit and `cargo build --bin v4vmm` last.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- The 0.12.0 contract from `GET /openapi.json` differs from the Live API facts of this packet.
- A MusicIndex row needs a detail request to show its text.
- The frame filter row cannot hold the scope control without a change to another destination.
- A change needs a file in "Do not touch".

## Implementation Result - 2026-10-21 (Part A)

Part A is complete, with two deviations. The section "Deviations" gives them.

### The stored contract

- The implementer sent one read-only GET request to `https://api.musicindex.org/openapi.json` on 2026-10-21 at 04:05 UTC.
- The response gave `info.version` `0.12.0` and 100,712 bytes. Its difference from the 0.11.0 contract is the difference of the Live API facts.
- `tests/fixtures/musicindex-openapi-0.12.0.json` holds those bytes. The 0.11.0 file is deleted.
- `MUSICINDEX_CONTRACT_FIXTURE`, the guard comment, the failure text and the AGENTS.md "Stored facts" line name the 0.12.0 file.
- The test `adr_0085_feed_page_contract_fixture_is_0_11_0` is replaced by `adr_0084_scopes_r16_03_contract_fixture_is_0_12_0`.

### Files changed

- `src/api.rs`: `FeedNamesResponse`, `ArtistNamesResponse`, `ArtistNameKind`, `ArtistNameRow` and `NamedFeedEntry`. `Feed` adds `sort_name`, `publisher_feed_guid` and `named_feeds`. `Client::fetch_artist_names` and `name_list_params`.
- `src/application/request_profiles.rs`: `RequestPathShape::ArtistNameList` and `INDEX_ARTIST_NAME_LIST`. The registry test counts nine profiles.
- `src/application/queries/name_lists.rs` (new): `NAME_LIST_PAGE_LIMIT` (100) and `FetchArtistNamePage`.
- `src/view_models/index_name_list.rs` (new): `MusicListScope`, `MusicScopeControlDisplay`, `name_sort_key`, `watermark_shows`, `IndexNameListPage`, `IndexNameListBatch` and `IndexListStatusRowVm`.
- `src/view_models/artists_everything.rs` (new): the classification of decision D11, the MusicIndex rows of decisions D12 and D14, and the merge of decisions D5 to D8.
- `src/view_models/library.rs`: `artists_list(scope, page)` and `artists_list_status`. The Library rows use `name_sort_key` in both scopes. `DestinationListRowTarget` adds `IndexPublisher` and `IndexName`.
- `src/library.rs`, `src/library/app_impl.rs` and `src/library/music_scopes.rs` (new): the two scopes, the artist page, the request through `present_command`, "Load more" and "Try again". The event `OpenIndexPublisherPage` opens a MusicIndex feed row.
- `src/app.rs` and `src/app/publisher_dispatch.rs`: the scope control slot, and the feed page with `PublisherPageContext::Index`.
- `src/view_models/workspace/chrome.rs`, `src/ui/shells/workspace.rs` and `src/ui/composites/frame_shell.rs`: the scope control at the leading edge of the frame filter row.
- `src/ui/shells/library/destination_lists.rs`: the status row of the Artists list. The file has 458 code lines.
- `src/ui/shells/library/views_list.rs` (new): the Discover views list, moved out of `destination_lists.rs` for the ADR 0038 ceiling.
- `src/ui/shells/library/index_list_status.rs` (new): the loading row, the error row and "Load more".
- `tests/architecture_tests.rs`: the contract guard of R16-03.
- `src/view_models/singles_group.rs`: one test calls `artists_list` with the scope "Library".

### Tests

| Case | Proof |
|---|---|
| R16-01 | `api::adr_0084_scopes_tests::adr_0084_scopes_r16_01_artist_name_rows_decode` |
| R16-02 | `api::adr_0084_scopes_tests::adr_0084_scopes_r16_02_feed_name_rows_decode` |
| R16-03 | `adr_0084_scopes_r16_03_contract_fixture_is_0_12_0` and `adr_0075_task_051_contract_field_guard_passes_on_present_code` |
| R16-04 | `adr_0084_scopes_r16_04_name_list_profiles` and `adr_0075_request_profile_registry_names_nine_profiles`, for the artist profile only |
| R16-05 | `adr_0084_scopes_r16_05_artist_page_sends_one_request`, for the artist command only |
| R16-06 | `adr_0084_scopes_r16_06_name_sort_key` |
| R16-07 | `adr_0084_scopes_r16_07_append_keeps_rows` and `adr_0084_scopes_r16_07_old_generation_changes_nothing` |
| R16-08 | `adr_0084_scopes_r16_08_classification` |
| R16-09 | `adr_0084_scopes_r16_09_merge_order_and_watermark` and `adr_0084_scopes_r16_09_watermark` |
| R16-10 | `adr_0084_scopes_r16_10_index_row_text_and_targets` and `adr_0084_scopes_r16_10_artists_rows_in_each_scope`. The present Artists tests pass in the scope "Library" |
| R16-11 | `adr_0084_scopes_r16_11_scope_control`. The guard `adr_0084_discover_r6_06_three_state_filter_control_is_deleted` passes |
| R16-12 | `adr_0084_scopes_r16_12_status_rows` and `adr_0084_scopes_r16_12_first_page_states` |
| R16-21 | Not in Part A. Packet 017 adds it, because packet 016 came first |

The "0$" row of R16-01 is a constructed row with each contract field. The packet does not record its values.

### Deviations

1. **The release name list request moves to Part B.**
   - Part A has no caller of `FetchReleaseNamePage`, `Client::fetch_feeds_by_name`, `RequestPathShape::FeedNameList` or `INDEX_RELEASE_NAME_LIST`.
   - The guard `adr_0060_discover_surface_stays_deleted` forbids each `dead_code` lint attribute. AGENTS.md "Delete dead code" forbids parked code.
   - Part B adds the four items with step 11. Then the registry test counts ten profiles, and R16-04 and R16-05 get their release cases.
   - Part A keeps the decode of the release rows: `FeedNamesResponse`, `NamedFeedEntry` and the three `Feed` fields.
2. **The Releases scope control has no effect in Part A.** The control shows for Releases, and it keeps its value. The Releases list shows the Library rows in each scope until Part B.

### Notes

- A MusicIndex name row opens `FrameNavigationEntry::IndexNameMatches(name)` with the `name` text of the row.
- The loading row and the error row of page 1 show before the item rows. "Load more" and the error row of a later page show after them.
- A scope change on Artists starts page 1 only when no request was sent. After an error, "Try again" is the only new request.
- `docs/architecture/source-map.md` does not name the five new files. This packet may not change that file.

## Implementation Result - 2026-10-21 (Part B)

Part B is complete. Required Changes 11 to 16 and the four items that Part A moved to Part B are in the code. The section "Deviations" gives the small differences.

### Files changed

- `src/api.rs`: `Client::fetch_feeds_by_name`. It sends `GET /v1/feeds/by-name` with `limit`, the cursor and the profile include.
- `src/application/request_profiles.rs`: `RequestPathShape::FeedNameList` and `INDEX_RELEASE_NAME_LIST` (`named_feeds`). The registry test is now `adr_0075_request_profile_registry_names_ten_profiles`.
- `src/application/queries/name_lists.rs`: `FetchReleaseNamePage`. It sends one request for each page and reads no release detail.
- `src/view_models/releases_everything.rs` (new): the order key, the state word, the `MusicIndex` release row, the merge with the watermark, the scope change of the selection and `ReleasesDownloadPlan`.
- `src/view_models/index_name_list.rs`: `IndexNameList::Releases`, with the texts of the loading row, the error row and "Load more".
- `src/view_models/library.rs`:
  - `releases_list` and `releases_list_with_selection` take the scope and the release page. `releases_list_status` gives the status row.
  - `library_release_row` adds the state word. `releases_selection_rows` gives the selection rows of the shown entries.
  - `releases_selection_action_bar` uses `SelectionActionBarVm::from_counts`. `releases_download_plan` gives the "Download" plan.
  - `release_is_fully_downloaded` gives `false` for a key that the tree does not hold (Required Change 14).
- `src/library.rs`: the field `release_name_page`.
- `src/library/music_scopes.rs`: the Releases scope, "Load more", "Try again" and one shared request function for the two name lists.
- `src/library/app_impl.rs`: the "Download" action queues the `MusicIndex` jobs. The Releases list gets its status row. A focus move counts a status row at the list start.
- `src/ui/shells/library/destination_lists.rs`: the status row of the Releases list.

### Tests

| Case | Proof |
|---|---|
| R16-04 | `adr_0084_scopes_r16_04_name_list_profiles` and `adr_0075_request_profile_registry_names_ten_profiles` |
| R16-05 | `adr_0084_scopes_r16_05_release_page_sends_one_request` and `adr_0084_scopes_r16_05_artist_page_sends_one_request` |
| R16-12 | `adr_0084_scopes_r16_12_release_status_rows`, for the Releases texts |
| R16-13 | `adr_0084_scopes_r16_13_merge_order_and_watermark` and `adr_0084_scopes_r16_13_first_page_in_flight_shows_no_row` |
| R16-14 | `adr_0084_scopes_r16_14_index_row_text`, `adr_0084_scopes_r16_14_state_words` and `adr_0084_scopes_r16_14_library_row_state_words` |
| R16-15 | `adr_0084_scopes_r16_15_index_single_is_a_release_row`. The `adr_0084_singles_group_` tests pass |
| R16-16 | `adr_0084_scopes_r16_16_bar_counts` |
| R16-17 | `adr_0084_scopes_r16_17_download_plan`. It also proves Required Change 14 |
| R16-18 | `adr_0084_scopes_r16_18_scope_change_resets_selection` and `adr_0084_scopes_r16_18_select_all_selects_shown_rows_only` |
| R16-19 | `adr_0084_scopes_r16_19_download_changes_the_row_in_place` |
| R16-20 | The full `cargo test` run passes, with the tests of packets 009, 015, 018, 019 and 022 |

The commands of the section "Checks" are Green. `cargo clippy --all-targets -- -D warnings` and `cargo check --all-targets` are Green. `cargo build --bin v4vmm` ran last.

### Deviations

1. A `MusicIndex` row with no feed GUID gives no row. The app cannot open, select or download it.
2. A `MusicIndex` row with no title shows "Untitled release".
3. The single rows of an open singles group also show the state word in "Everything". They use the same row function as each other Library release row.
4. The selection rows of the Releases list moved from `LibraryApp` to `releases_selection_rows` in the view model, so that a test can prove R16-18.

### Notes

- "Add to playlist" now counts only the selected Library releases. Before Part B, it counted each selected release.
- A `MusicIndex` row has the element id of a Library row with the same key. The row thus keeps its identity when a download adds it to the Library.
- The Releases list computes the merge for each call of `releases_entries`. One render calls it more than one time, for the header action and for the rows.
- A Library reload while page 1 loads clears the Releases selection, because the list then shows no item row.
- `docs/architecture/source-map.md` does not name `src/view_models/releases_everything.rs`. This packet may not change that file.

## Operator Visual Check

Part A owns V1 to V3, V7 and V8 for the Artists list. The gate stays open until a person does these steps.

The check needs a desktop session, network access to `https://api.musicindex.org` and a Library with at least one artist.

1. Build the binary: `cargo build --bin v4vmm`.
2. Start the app: `target/debug/v4vmm`. Open Music.
3. V1: click each sidebar destination.
   - Releases and Artists show "Library" and "Everything" under the list header. "Library" is selected. "Select" stays in the header of Releases.
   - Discover, Tracks, Needs attention and Playlists show no scope control.
   - Wrong: a scope control on another destination, or a control that covers the header.
4. V1 for Releases: click "Everything" on Releases. The list does not change in Part A. Click "Library" again.
5. V2: open Artists and click "Everything".
   - One row "Loading MusicIndex artists" shows, and no item row. Then the rows show in name order.
   - Each Library artist shows "n in Library". A Library name row shows "n in Library · by name".
   - A MusicIndex artist shows "Not in Library". A MusicIndex name row shows "Not in Library · by name".
   - Sir Libre Records is absent. No row shows "Artist", "Unverified" or "publisher".
   - Wrong: one artist on two rows, or a Library row that shows "Not in Library".
6. V2: scroll to the end of the list and click "Load more". The new rows come after the last row. No row that showed before moves.
7. V3: click a MusicIndex feed row. Its feed page opens in the detail column. Click a MusicIndex name row. The MusicIndex name match page opens.
8. V2 note: the "Haleen · by name" row goes in packet 017. Do not record it as a failure of this packet.
9. V8: do steps 3 and 5 again in the Light theme and in the Dark theme. Make the window narrow until the list column is at its minimum width. The control and the row text must not be clipped.
10. V7: close the app. Turn the network off: `nmcli networking off`.
11. V7: start the app with `target/debug/v4vmm`. Open Artists and click "Everything".
    - The error row shows first: "MusicIndex did not answer the artist list request. The list shows Library artists only." Its detail names `GET /v1/artist-names`.
    - The Library rows show under it. The row has "Try again".
12. V7: turn the network on with `nmcli networking on`. Click "Try again". The MusicIndex rows show.

Cleanup:

1. Close the app.
2. Make sure that the network is on: `nmcli networking on`.
3. The app stores no scope in the config file, so no other cleanup is necessary.

### Part B: the Releases list

Part B owns V4 to V8 for the Releases list. The gate stays open until a person does these steps.

The check needs a desktop session, network access to `https://api.musicindex.org` and a Library with one or more releases. One Library release with only some tracks downloaded helps the check of the partial state.

1. Build the binary: `cargo build --bin v4vmm`.
2. Start the app: `target/debug/v4vmm`. Open Music, then open Releases.
3. D13 in "Library": a release with only some tracks downloaded shows, for example, "Haleen · Album · 3 of 7 tracks". Each other Library row shows no state word.
4. V4: click "Everything".
   - One row "Loading MusicIndex releases" shows, and no item row. Then the rows show in name order.
   - A MusicIndex release shows "artist · Kind", for example "LIIINEKELA · Single", with no state word.
   - Each Library release adds "In Library", "3 of 7 tracks" or "Not downloaded".
   - A Library singles group stays one group row. A MusicIndex single shows as a release row.
   - Wrong: one release on two rows, a group single that shows a second time as a MusicIndex row, or a Library row with no state word.
5. Click a MusicIndex release row. Its release page opens in the detail column.
6. V5: scroll to the end of the list and click "Load more". The new rows come after the last row. No row that showed before moves.
7. V6: click "Select". Select two MusicIndex releases and one Library release that is fully downloaded.
   - The bar shows "Download 2 releases", "Add 1 release to playlist" and "Remove 1 release…".
   - Wrong: a MusicIndex release in the count of "Add to playlist" or "Remove".
8. V6: click "Download 2 releases".
   - The download progress item shows the two releases.
   - After each download, its row changes to the Library row in place, with "In Library". The row does not move.
9. Click "Library". The selection clears, and selection mode ends.
10. V8: do steps 4 and 7 again in the Light theme and in the Dark theme. Make the window narrow until the list column is at its minimum width. The control and the row text must not be clipped.
11. V7: close the app. Turn the network off: `nmcli networking off`.
12. V7: start the app with `target/debug/v4vmm`. Open Releases and click "Everything".
    - The error row shows first: "MusicIndex did not answer the release list request. The list shows Library releases only." Its detail names `GET /v1/feeds/by-name`.
    - The Library rows show under it, with their state words. The row has "Try again".
13. V7: turn the network on with `nmcli networking on`. Click "Try again". The MusicIndex rows show.

Cleanup for Part B:

1. Close the app.
2. Make sure that the network is on: `nmcli networking on`.
3. Step 8 added two releases to the Library. To remove them, open Releases, click "Select", select the two releases and click "Remove 2 releases…". You can also keep them.
4. The app stores no scope in the config file, so no other cleanup is necessary.
