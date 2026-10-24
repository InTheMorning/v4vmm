# ADR 0084 Task 026: The Singles Groups Of Index Releases In "Everything"

Status: Implemented - 2026-10-21, Parts A and B. The mechanical cases R26-01 to R26-26 pass. The operator passed V1 to V5 and V8 on 2026-10-23, apart from the "You Are Enough" row of V2. V2 (the "You Are Enough" row only), V6, V7, V9 and V10. V6 and V10 failed on 2026-10-23 (run record, question 2 and walk log) are open, in the final walk of the run of 2026-10-23. Packet 033 replaced the text "3 in Library" of V1 with the download state icon and its count on 2026-10-22.

## Goal

The Releases list has the scope "Everything". In it, the MusicIndex singles of one artist feed join one singles group with the Library singles of that feed.
The group row gives the full count of the singles of that feed, also of the singles on pages that did not load.
The group page lists the Library singles and the MusicIndex singles of the feed, newest first.

Part A gives the stored contract, the decode, the request for the releases that name one feed, and the groups in the Releases list.
Part B gives the group page, its main action and the link "All n singles" with the MusicIndex singles.

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md):
  - Decision 6: two or more singles of one artist in the current scope show as one group row. The group page lists the singles, newest first. The group key is the artist identity of Decision 7.
  - Decision 8: the two scopes.
  - Decision 4: one page for each item, and the state word.
  - Decision 10: the selection model and the download queue. Decision 12: the logo frame.
- [ADR 0085](../adr/0085-artists-by-confirmation.md) Decision 2 and Decision 6: the artist feed of a release is the first named feed with an agreed `artist` role, else the primary named feed with a confirmed link. A link that only the album states does not make an artist.
- [ADR 0075](../adr/0075-metadata-contract.md) section 6: named request profiles, no detail read for each list row, and the stored contract changes in the same change as the decode.
- [ADR 0040](../adr/README.md): background work runs on the async command runner. No screen calls `cx.spawn`.
- [Open Stophammer requests](../plans/v4vmm-open-requests.md): request 9, "Stophammer Answers - 2026-10-21" and "Release 0.13.0 - 2026-10-21". Stophammer ADR 0076 owns the three additions.
- The [phase plan](../plans/adr-0084-music-column-browser-phase-plan.md) row of packet 026.
- Archived packet [015](archive/adr-0084-task-015-singles-groups.md): the Library singles groups. Packet [016](adr-0084-task-016-library-and-everything-scopes.md) decision D4: a MusicIndex single is a release row until this packet.

## Recorded Facts - 2026-10-21

### Code facts

The code facts come from the working tree of 2026-10-21, after commit `c1c728a`. Find each call site again before you change it.

- `singles_groups` in `src/view_models/singles_group.rs` groups the Library singles only. `GroupSingle` borrows an `ArtistNode` and an `AlbumNode` of the `LibraryTree`.
- The guard `adr_0084_singles_group_r15_09_only_the_owner_groups_singles` keeps `ReleaseKind::Single` in `src/view_models/singles_group.rs` and `src/view_models/release_kind.rs`.
- `ArtistIdentity::resolve(named_feeds, artist_text)` in `src/view_models/artist_identity.rs` calls `select_artist_feed` in `src/view_models/named_feeds.rs`. The unverified rule reads `NamedFeed::confirmed`.
- `src/views.rs` sets `NamedFeed::confirmed` of a Library release from `two_way_validated == Some(true)`. It does not read `role_agreement` for this value.
- `NamedFeedEntry` in `src/api.rs` decodes `publisher_feed_guid`, `remote_feed_title`, `album_names_rank`, `role` and `role_agreement`. Its comment says that the entry gives no `two_way_validated`.
- `LibraryViewModel::releases_list` in `src/view_models/library.rs` builds the Library entries and the Library groups. In "Everything", it calls `releases_everything::everything_entries`.
- `everything_entries` in `src/view_models/releases_everything.rs` drops a MusicIndex row whose key is in `library_keys`. It applies the watermark of packet 016 decision D7.
- `ReleaseOrderKey::group` orders a group row by the sort key of its label.
- `SinglesGroupPageVm::from_tree` builds the group page from the Library tree only. Each row is the shared Library track row of the first track of a single (operator decision of 2026-10-17).
- `SinglesGroupAction::DownloadSingles(Vec<i64>)` holds Library feed ids only.
- `singles_group_name_link` gives "All n singles" only for a release in a Library group.
- `Client::fetch_recent_feeds` in `src/api.rs` sends `limit` and `cursor` only, with no request profile. `Client::fetch_feeds_by_name` sends `artist_key`, `limit`, `cursor` and the profile include.
- `src/application/request_profiles.rs` has eleven profiles after packet 029. The test `adr_0075_request_profile_registry_names_eleven_profiles` counts them.
- Packet 029 replaced the stored contract on 2026-10-21. `MUSICINDEX_CONTRACT_FIXTURE` in `tests/architecture_tests.rs` names `tests/fixtures/musicindex-openapi-0.13.0.json`. The test `adr_0084_scopes_r16_03_contract_fixture_is_0_13_0` asserts the version `0.13.0`. Thus this packet does not replace the contract (D1), and R26-01 is complete.
- `ReleasesDownloadPlan` and `loaded_index_release_title` in `src/view_models/releases_everything.rs` give the MusicIndex releases of the "Download" action and their titles.

### Live API facts

The packet author sent read-only GET requests to `https://api.musicindex.org` on 2026-10-21, from 16:17 UTC to 16:24 UTC.

- `GET /node/info` gave `git_revision` `b00f529` and `built_at` `2026-10-21T15:00:52Z`.
- `GET /openapi.json` gave `info.version` `0.13.0`, 104,418 bytes, sha256 `eae53d3836f5c68a1eff3a8ab1e0a205ba3b0cc6353a84aff41dea90f6a8df2d`. The scratchpad copy of the orchestrator has the same bytes.
- The difference from the stored 0.12.0 contract:
  - `NamedFeedEntry` adds `two_way_validated`, a required boolean.
  - `GET /v1/feeds/by-name` and `GET /v1/feeds/recent` add the query parameter `named_feed_guid`.
  - `GET /v1/search` adds `include`, with the one name `link_facts`. `SearchResponseItem` adds `two_way_link_count`, `stated_rels`, `agreed_roles`, `confirmed_release_artists` and `first_named_count`.
  - Changes that no packet reads: `TrackResponse` adds `local_value_time_splits`, `VtsResponse` adds four fields and makes `start_time` required, and the new schema `LocalVtsResponse`. The contract guard checks that each decoded field is declared, so these changes do not fail it.
- `GET /v1/node/capabilities` gives `include_params.search` `["link_facts"]`.
- `GET /v1/feeds/recent` gives its rows in the order of `newest_item_at`, newest first. The contract calls this order "recent-source order". `limit` is 1 to 200. A `limit` of 500 gave 143 rows for a feed with 143 releases.
- `named_feed_guid` with `medium=publisher` gave `400` with `{"error":"named_feed_guid needs medium=music (ADR 0076 section 2)"}`. With `artist_key`, it gave `400` with `{"error":"named_feed_guid conflicts with artist_key (ADR 0076 section 2)"}`. A GUID that no release names gave `200` with an empty `data`.

The feeds of the operator's Library and of the walk:

| Feed | GUID | `GET /v1/feeds/recent?named_feed_guid=<guid>&include=named_feeds&limit=200` |
|---|---|---|
| Haleen, the Library artist feed (packet 008 record) | `64569161-a8f7-52e5-8aae-9e0385043149` | 56 rows, `has_more` false. 55 rows have `episode_count` 1, and "Damn Your Idols" has 11. Each entry of this feed is at rank 1 with `two_way_validated` true and no role |
| Haleen, a second feed with the same title (packet 028 D4) | `fe80a515-2826-4a46-aa37-b27f38d3c30f` | 39 rows. Each has `episode_count` 1. "You Are Enough" names the feed at rank 1 with `two_way_validated` false. The other 38 entries are true |
| Paul Birken | `7513ecb9-19e7-4df0-98e3-e13909e09533` | 143 rows, `has_more` false, 0.43 s, 168,032 bytes. Each entry is at rank 1 with `two_way_validated` true |
| Sir Libre Records | `34bf795f-f551-4db1-8877-ad19aa5f89c8` | 6 rows. Each entry is at rank 2. Three entries have `role` `label` and `role_agreement` `both`, and three have `role_agreement` `conflict`. Each of the six has `two_way_validated` true |

- The orchestrator asked if `7513ecb9-19e7-4df0-98e3-e13909e09533` is Sir Libre Records. It is not. It is the feed "Paul Birken". Sir Libre Records is `34bf795f-f551-4db1-8877-ad19aa5f89c8`.
- Each release row of these requests gives `release_kind` "unknown". `ReleaseKind::resolve` thus uses `episode_count`.
- The request for the Haleen feed took 0.12 s. The request for Rap Central (`a453c8d0-a2b1-45aa-bcbb-9200461e844c`) took 0.08 s. The same filter on `GET /v1/feeds/by-name` took about the same time.

The full release name list, read with 44 requests of `GET /v1/feeds/by-name?include=named_feeds&limit=200` from 16:21:56 UTC to 16:22:19 UTC:

- 8,798 music releases. 6,175 have `episode_count` 1. Each row gives `release_kind` "unknown". 757 rows give no `named_feeds` entry.
- By the rule of ADR 0085 Decision 6, 5,574 singles have an artist feed. They name 1,220 different feeds. 628 of these feeds have two or more singles, with 4,982 singles in all. The largest group has 101 singles.
- No entry has an agreed `artist` role. Thus each artist feed of a single is an unverified artist feed.
- 601 singles have no artist feed (the name level). They have 189 different name keys, and 82 keys have two or more singles.
- 8,048 entries: 176 have `two_way_validated` false, 4 have `role_agreement` `conflict`, 11 have `both` and 2 have `one_side`. Each of the 4 conflict entries has `two_way_validated` true. One conflict entry is at rank 1. 55 entries name a feed that the node does not hold.
- **D-2 count.** 121 releases name a held feed at rank 1 through a link that only the album states, with no agreed role. 98 of them are singles. Examples: "2 Die" names SOD. Many singles of "Under The Mulga Groove" name the feed of that title.
- In pages of 100 rows, each page names artist feeds that no earlier page named. For the first ten pages, the counts are 37, 28, 37, 35, 41, 42, 27, 30, 30 and 27. The mean over the 88 pages is 13.9.
- A group row in the order of its label: 128 of the 628 groups have a label that sorts before the last row of the page before the page of their first single. Such a group row thus shows above rows that already show.

`GET /v1/feeds/by-name?include=named_feeds&limit=100` gave page 1 in 0.17 s.

### Two rule differences that stay

- D-2 (packet 016): Stophammer ADR 0070 §4 does not put a D-2 release on the name level, and v4vmm does. Stophammer ADR 0076 §4 keeps this. Thus `artist_key` does not give the D-2 singles of a name.
- The conflict case: Stophammer ADR 0076 §1 calls a link confirmed when `two_way_validated` is true and `role_agreement` is not `conflict`. ADR 0085 Decision 2 calls a link confirmed when `two_way_validated` is true. On 2026-10-21, one entry at rank 1 had `conflict`.

**The stored contract (orchestrator decision of 2026-10-21).** Packets 026 and 029 each need the 0.13.0 contract. The first of the two that an implementer starts replaces the 0.12.0 file. The later packet finds the 0.13.0 file and changes only its own decode. The orchestrator proposes packet 029 first (see "Packet Order").

## Design

### The artist feed of a MusicIndex release

- `NamedFeedEntry` decodes `two_way_validated: Option<bool>`. The contract makes it required, and `Option` keeps a response of an older node decodable.
- Add `index_named_feeds(entries: &[NamedFeedEntry]) -> Vec<NamedFeed>` to `src/view_models/named_feeds.rs`. Each entry gives one `NamedFeed`:
  - `feed_guid` from `publisher_feed_guid`, `title` from `remote_feed_title`, `position` from `album_names_rank`.
  - `role` as it is, and `role_agreement` from `RoleAgreement::as_str`.
  - `confirmed` from `two_way_validated == Some(true)`, the same read as the Library in `src/views.rs` (decision D3).
  - `feed_rel`, `album_rel` and `image_url` are `None`.
  - The function sorts the result with `sort_named_feeds`.
- The artist identity of a MusicIndex release is `ArtistIdentity::resolve(&index_named_feeds(entries), release_artist)`. The Library and MusicIndex thus use one rule.

### The singles of one feed

- One request lists the releases that name one feed: `GET /v1/feeds/recent?named_feed_guid=<guid>&include=named_feeds&limit=200`.
  - Add `RequestPathShape::RecentFeedList` and the profile `INDEX_FEED_SINGLES` (`named_feeds`) to `src/application/request_profiles.rs`.
  - Add `Client::fetch_releases_naming(named_feed_guid, limit, cursor, profile)` to `src/api.rs`.
  - `FEED_SINGLES_PAGE_LIMIT` is 200, and `FEED_SINGLES_MAX_PAGES` is 5. These are named constants.
- The singles of feed G are the rows of that list that are singles (`ReleaseKind::resolve(release_kind, episode_count)`) and whose identity is `ArtistIdentity::Publisher(G)`.
  - A row that names G at rank 2, for example a label, has another identity. It is not a single of G.
  - A row whose link to G is not confirmed has the name level. It is not a single of G (decision D7).
- The app reads each page of the list for G, up to 5 pages. Then the group count is complete. When the last page has `has_more` true, the group keeps the loaded singles and records that more exist (decision D11).
- `IndexFeedSingles` keeps the result of each feed G for the session: the rows, the load state and a request generation. The loaded results stay until the app stops, as the name list pages of packet 016 decision D9 do.

### When the app reads the singles of a feed

The app reads the singles of a feed G before the rows of the page that needs G show (decision D4):

1. When "Everything" loads page 1, the same command also reads each Library artist feed that has one or more Library singles.
2. When a page of `GET /v1/feeds/by-name` loads, the same command reads each artist feed of a single on that page that has no result yet.
3. At most four of these requests run at the same time (`FEED_SINGLES_PARALLEL`, decision D4).
4. The rows of the page show when each of its requests has an answer.

- The command reads no release detail. It sends one list request for each new feed, not for each row (ADR 0075 section 6).
- A failure of one request for a feed fails the page load (decision D10). The page shows the error row of packet 016. "Try again" sends the failed requests again, and does not send the requests that had an answer.

### The groups in "Everything"

- `singles_groups_everything(tree, fills, page)` in `src/view_models/singles_group.rs` gives the groups of the scope "Everything". `singles_groups` stays the function of the scope "Library".
- A group of feed G holds the Library singles whose identity is `Publisher(G)`. It also holds each MusicIndex single of G whose release key is not a Library release.
- A name identity group holds the Library singles only (decision D6).
- The Library decides a Library release (decision D8). A release in the Library has its Library identity. Its MusicIndex row is not a member of a group of another identity.
- A group exists when it has two or more members. A MusicIndex single of G in no group shows as a release row, as today.
- A MusicIndex single that is a member of a group gives no release row of its own.
- A member counts in the group also when its page of `GET /v1/feeds/by-name` did not load.

### The group row in "Everything"

- The order position of a group row is the sort key of its first single in title order (decision D5). The watermark of packet 016 decision D7 applies to that position. In "Library", the group row keeps the order of its label.
- The label is the Library label of packet 015 when the Library holds a member. Else it is the `remote_feed_title` of the G entry of the newest MusicIndex single. Else it is the `release_artist` of that single (decision D9).
- The second text:
  - with Library members: "55 singles · 3 in Library · Artist feed", or "55 singles · In Library · Artist feed" when each single is in the Library
  - with no Library member: "38 singles · Artist feed" (decision D9)
- The count is the number of members. "In Library" counts the members whose tracks are each in the Library, as in packet 015.
- The art is the logo frame of packet 011: the image of the artist feed when the Library knows it, else the mosaic of the newest four member images.
- The open group shows each member, newest first. A Library member keeps its Library row with its state word. A MusicIndex member uses `index_release_row`.
- The date of a Library member is the latest `pub_date` of its tracks. The date of a MusicIndex member is `newest_item_at`. A member with no date comes last. Equal dates sort by the sort key of the title.
- The group checkbox selects each member. `single_keys` holds the MusicIndex keys too. The bar counts follow packet 018 R18-06. "Download" queues one `DownloadJob::IndexRelease` with `feed: None` for each selected MusicIndex member.

### The group page (Part B)

- The page is not scoped (decision D12). `FrameNavigationEntry::SinglesGroup(SinglesGroupKey)` opens one page for each identity, from each scope and from each link.
- For a feed identity, the page reads the singles of G when `IndexFeedSingles` has no result. The request runs through `present_command`, with the generation check.
- While the request runs, the Library rows show, and one row says "Loading MusicIndex singles". After a failure, the Library rows stay, and an error row has "Try again".
- The rows come newest first, by the date rule of the group row.
  - A Library member keeps the shared Library track row (operator decision of 2026-10-17).
  - A MusicIndex member shows as a release row: the image, the title and the date. It has no state word, as on the feed page. A click opens its release page (decision D13).
- The subtitle is the second text of the group row in "Everything".
- The main action:
  - "Download n singles" when n members are not each in the Library. n counts the Library members that are not each in the Library and each MusicIndex member.
  - The action queues the present Library jobs and one `DownloadJob::IndexRelease` for each MusicIndex member, newest first.
  - Else "Add to playlist", as today.
- The empty state text becomes "This artist has fewer than two singles." (decision D12).

### The link "All n singles" (Part B)

- A release page of a single whose identity is `Publisher(G)` shows "All n singles" when the group of G has two or more members. This applies to a Library page and to an Index page.
- The release page reads the singles of G when `IndexFeedSingles` has no result (decision D14). It sends one request for each page open, and no request for a row.

## Orchestrator Decisions

"**Flag.**" marks a decision that the operator confirms before an implementer starts. Each one gives the options and the recommendation of the orchestrator.

- **D1. The contract fixture.** The first of packets 026 and 029 that runs replaces `tests/fixtures/musicindex-openapi-0.12.0.json` with `tests/fixtures/musicindex-openapi-0.13.0.json`. The new file holds the bytes of `GET /openapi.json`.
  - The same change updates `MUSICINDEX_CONTRACT_FIXTURE` and its comment, and the `CONTRACT_TYPE_MAP` comment.
  - It replaces the test `adr_0084_scopes_r16_03_contract_fixture_is_0_12_0` with a test that asserts `0.13.0`, maps the same four types, and finds no 0.12.0 file.
  - It updates the AGENTS.md "Stored facts" line.
- **D2. One request for the singles of each feed.** The app uses `named_feed_guid` on `GET /v1/feeds/recent`, because that route gives the rows newest first. The group page and ADR 0084 Decision 6 need that order.
- **D3. One confirmation rule.** A MusicIndex entry is confirmed when `two_way_validated` is true, as a Library link is. ADR 0085 Decision 2 states this rule.
  - The conflict case of Stophammer ADR 0076 §1 stays an open finding. On 2026-10-21 it changed one release.
- **D4. When the app reads the singles of a feed.** **Flag.**
  - Option A: each page of the release list reads the singles of each new artist feed of its singles, before its rows show. At most four requests run at the same time. Page 1 also reads the Library artist feeds with singles.
    - Cost on 2026-10-21: 27 to 42 requests for each of the first ten pages of 100 rows, with a mean of 13.9 over all pages. One request took 0.08 s to 0.43 s. Page 1 can thus take 1 s to 4 s more than today.
  - Option B: groups only for the artist feeds that the Library holds. The app reads those feeds one time, with page 1. Each other MusicIndex single stays a release row. This is a part of ADR 0084 Decision 6 only, and needs an amendment of that decision.
  - Option C: groups from the loaded pages only, with no extra request. A group grows when a later page loads, and a release row can go into a group after it shows. This breaks packet 016 decision D7.
  - Option D: a new Stophammer request for a count of the singles that name each feed. Stophammer does not decide the release kind, so the count must use the kind rule of v4vmm. The orchestrator does not recommend it.
  - **Recommendation: Option A.** It gives correct groups and counts, and no row moves after it shows. The orchestrator adds the cost to the responsiveness list of the overhaul plan.
- **D5. The place of a group row in "Everything".** **Flag.**
  - Option A: the position of the first single of the group in title order. A group row then shows with the page of its first single, and it never shows above a row that already shows. In "Library", a group row keeps the order of its label.
  - Option B: the order of the label, as in "Library". On 2026-10-21, 128 of 628 groups would show after a later page loads, above rows that already show. A visible row thus moves down.
  - **Recommendation: Option A.** Packet 016 decision D7 says that no row moves after it shows. The label of the row still names the artist.
- **D6. A MusicIndex single at the name level stays a release row.** **Flag.**
  - Option A: the app forms no group of MusicIndex singles at the name level. A Library name group keeps its Library singles only.
  - Option B: name groups from `artist_key` on `GET /v1/feeds/by-name`. That route does not give the D-2 singles (98 on 2026-10-21), so such a group can miss singles that v4vmm puts on the name level.
  - **Recommendation: Option A.** On 2026-10-21, 601 of 6,175 singles had the name level.
- **D7. A single with a link that only the album states.** It has the name level, by ADR 0085 Decision 6. It is not a member of the group of the feed that it names. It shows as a release row.
  - Example: "You Are Enough" names the second Haleen feed (`fe80a515`) with `two_way_validated` false. It shows as a release row. Its release page shows that feed as an "Also named" row.
- **D8. The Library decides a Library release.** A release in the Library keeps its Library identity. Its MusicIndex row is not a member of another group. This is the rule of packet 016 decision D5 and packet 017 decision D9.
  - Thus a Library release with older named feed facts can show in a name group while MusicIndex names a feed for it. The Library refresh of the release corrects this.
- **D9. The text of a group row with MusicIndex members.** **Flag, minor.**
  - The label is the Library label when the Library holds a member, else the title of the artist feed.
  - The second text has no Library part when the Library holds no member: "38 singles · Artist feed". A MusicIndex row has no state word (packet 016 decision D13).
  - Alternative: "38 singles · Not in Library · Artist feed".
  - **Recommendation:** no Library part, as a MusicIndex release row.
- **D10. A failed request fails the page load.** The rows of a page show only after each request of that page has an answer. "Try again" sends only the failed requests. A row thus never changes from a release row to a group member.
- **D11. At most 5 pages of 200 rows for each feed.** On 2026-10-21, the largest feed had 143 releases. When more exist, the group keeps the loaded singles and the group page says that more exist. This is the limit of packet 017 decision D10.
- **D12. The group page is not scoped.** **Flag.**
  - The group page of a feed shows the Library singles and the MusicIndex singles of that feed, also when the operator opens it from the scope "Library".
  - Example: in "Library", the Haleen row says "3 singles · In Library · Artist feed". Its page then lists about 55 singles and offers "Download 52 singles".
  - ADR 0084 Decision 4 gives one page for each item, and Decision 8 gives a scope to the lists only. The feed page and the name level page already show their MusicIndex releases with no scope.
  - The empty state text "This artist has fewer than two singles in the Library." becomes "This artist has fewer than two singles."
  - Alternative: the page follows the scope of the list that opened it. This gives two pages for one key, which ADR 0084 Decision 4 does not permit.
  - **Recommendation:** the page is not scoped.
- **D13. The row of a MusicIndex single on the group page.** **Flag.** Replaced on 2026-10-22 by packet 032: each member of the group page uses one track row style (operator decision of 2026-10-22).
  - Option A: a Library single keeps the shared track row (operator decision of 2026-10-17). A MusicIndex single shows as a release row with its image, title and date.
  - Option B: each member shows as a release row, also a Library single. The track row and its toggle go from the group page.
  - Option C: each MusicIndex single shows as a track row. This needs a track read for each single, which ADR 0075 section 6 does not permit.
  - **Recommendation: Option A.** It keeps the decision of 2026-10-17 and sends no read for a row. The two row forms share one list and one date order.
- **D14. The link "All n singles" on each single page.** **Flag.**
  - A release page of a single with an artist feed reads the singles of that feed when the app has no result. The page then shows "All 55 singles". This adds one request to each open of such a page.
  - Alternative: the link uses only a result that the app already has, and sends no request. The link then shows or not by the history of the session.
  - **Recommendation:** read the singles of the feed, so the link does not depend on the history.
- **D15. The Library scope does not change.** In "Library", the groups, their order and their text stay as packet 015 gives them.

## Operator Answers - 2026-10-21

- D4: option A. Before a page shows, the app reads each new artist feed of the singles on that page, at most 4 requests at a time.
- D5: option A. A group row takes the place of its first single in title order.
- D12: the group page is not scoped. It lists the Library and MusicIndex singles of the feed in each scope.
- D13: option A. A Library single keeps its track row, and a MusicIndex single shows as a release row.
- The orchestrator decided D6, D9 and D14 by their recommendations, by the operator rule of 2026-10-20 ("decide but keep a record"). D6: a MusicIndex single at the name level stays a release row. D9: a group row with no Library member shows no Library part. D14: a single page reads the singles of its feed when the app has no result yet.


## Packet Order

The orchestrator proposes this order:

1. Packet 028, which is in progress. Packets 026 and 029 change `tests/architecture_tests.rs`, so they start after its commit.
2. Packet 029. It is small, and it replaces the stored contract (D1).
3. Packet 026 Part A, then Part B, each in its own session.

## Required Changes

### Part A: the contract, the decode, the request and the Releases list

1. When this packet runs before packet 029, replace the stored contract (D1). Else use the 0.13.0 file that packet 029 added.
2. Decode `NamedFeedEntry::two_way_validated`. Change the comment that says that the entry gives no `two_way_validated`.
3. Add `index_named_feeds` to `src/view_models/named_feeds.rs`.
4. Add `RequestPathShape::RecentFeedList`, the profile `INDEX_FEED_SINGLES` and `Client::fetch_releases_naming`. Add one to the count of the registry test.
5. Add the command that reads the singles of a set of feeds, at most four at the same time. Change `FetchReleaseNamePage` to read the new feeds of its page in the same command.
6. Add `IndexFeedSingles` with its generation, and keep it in `LibraryApp`.
7. Add `singles_groups_everything` and the members of a group with two origins to `src/view_models/singles_group.rs`.
8. Change `releases_list` in "Everything" to use the new groups, their order position, their text and their members. Change `everything_entries` to drop each MusicIndex row that is a group member.
9. Change the selection of a group row and `ReleasesDownloadPlan` to include the MusicIndex members, with their titles from `IndexFeedSingles`.
10. Change the error row of a failed page to name the failed request, for example `GET /v1/feeds/recent?named_feed_guid=…`. "Try again" sends only the failed requests.

### Part B: the group page and the link

11. Change `SinglesGroupPageVm` to take the result of the feed and to give the MusicIndex rows, the loading row and the error row.
12. Start the request of the group page through `present_command` when the app has no result for the feed.
13. Render the MusicIndex rows of the group page with named tokens, in the screen that renders the page today.
14. Change `SinglesGroupAction::DownloadSingles` to hold the Library feed ids and the MusicIndex feed GUIDs with titles. Queue the MusicIndex jobs through the download queue.
15. Change `singles_group_name_link` and its callers to use the groups with MusicIndex members. Start the request of the release page when the app has no result for the feed.
16. Change the empty state text (D12).

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_index_singles_` for unit tests beside the owning code. Each test uses a recorded row of 2026-10-21 or a constructed value. No test sends a request to `api.musicindex.org`. A request test uses a local fixture server, as the `Fixture` of `src/application/queries/feed.rs` does.

| Case | Part | Required proof |
|---|---|---|
| R26-01 | A | The contract test asserts `0.13.0`, maps the four name list types, and finds no 0.12.0 file. The contract field guard passes. Packet 029 can own this case when it runs first |
| R26-02 | A | `NamedFeedEntry` decodes `two_way_validated` true and false from recorded entries. An entry with no such field decodes as `None` |
| R26-03 | A | `index_named_feeds` gives the rank as the position, the role, the agreement and `confirmed` from `two_way_validated`. It gives the RSS order |
| R26-04 | A | `ArtistIdentity::resolve` with `index_named_feeds` gives `Publisher(64569161…)` for a recorded Haleen single. It gives the name level for "You Are Enough" of `fe80a515`, and for a release with an empty `named_feeds` |
| R26-05 | A | A rank 2 entry with `label` and `both`, as on the recorded Sir Libre Records rows, gives no group of Sir Libre Records |
| R26-06 | A | The singles command sends `GET /v1/feeds/recent` with `named_feed_guid`, `include=named_feeds` and `limit=200`. It follows the cursor up to 5 pages. It sends no detail request |
| R26-07 | A | A page command sends one request for each new artist feed of its singles. It sends no request for a feed that has a result. It never runs more than four requests at the same time |
| R26-08 | A | Page 1 in "Everything" also reads each Library artist feed with one or more Library singles |
| R26-09 | A | The view model gives no rows of a page while one of its requests runs. A failure of one request gives the error row, and "Try again" plans only the failed requests |
| R26-10 | A | `singles_groups_everything` gives one group for a Library single and a MusicIndex single of one feed. Its count is the size of the union by release key |
| R26-11 | A | A group counts a member whose name list page did not load. A MusicIndex single that is a member gives no release row |
| R26-12 | A | One MusicIndex single of a feed with no other single gives a release row and no group |
| R26-13 | A | A name level MusicIndex single gives a release row, also when a Library name group has the same key (D6) |
| R26-14 | A | A Library release keeps its Library identity, and its MusicIndex row is not a member of another group (D8) |
| R26-15 | A | The order position of a group row in "Everything" is the sort key of its first single. In "Library", it is the sort key of its label. The watermark applies |
| R26-16 | A | The group row text follows D9: "55 singles · 3 in Library · Artist feed" and "38 singles · Artist feed" |
| R26-17 | A | The open group gives its members newest first by the date rule. A MusicIndex member uses the MusicIndex release row |
| R26-18 | A | The group checkbox selects each member. The bar counts follow R18-06. The download plan holds each selected MusicIndex member with its title, with no duplicate |
| R26-19 | A | The scope "Library" gives the same entries as before this packet. The tests of packets 015 and 016 pass |
| R26-20 | B | The group page of a feed gives the Library rows and the MusicIndex rows, newest first, from each scope |
| R26-21 | B | The group page gives the loading row while its request runs, and the error row with "Try again" after a failure. The Library rows stay in both states |
| R26-22 | B | The main action gives "Download n singles" with n from the D12 example. Its plan holds the Library feed ids and the MusicIndex feed GUIDs, newest first |
| R26-23 | B | A single page of a feed group of two or more members gives "All n singles". The page plans one request when the app has no result |
| R26-24 | B | A result of an older generation changes nothing on the group page and on the release page |
| R26-25 | B | The empty state text is "This artist has fewer than two singles." |
| R26-26 | A, B | The guard `adr_0084_singles_group_r15_09_only_the_owner_groups_singles` passes. Each new use of `ReleaseKind::Single` is in its owner file |

## Visual Acceptance Criteria

These are for the operator, in a walk after each part. No test proves them.

- V1 (Part A): Releases, "Everything". A group row "Haleen" shows "n singles · 3 in Library · Artist feed", with n about 55. Packet 033 replaced "3 in Library" with the download state icon and its count, for example "19 of 56", on 2026-10-22. No Haleen single of that feed shows as its own row.
- V2 (Part A): a second group row "Haleen" shows "38 singles · Artist feed". "You Are Enough" of that feed shows as its own row.
- V3 (Part A): a group row "Paul Birken" shows its singles. Sir Libre Records has no group row.
- V4 (Part A): "Load more" adds rows at the end. No row that showed before moves, and no row changes into a group member.
- V5 (Part A): open the Haleen group row with its chevron. The singles show newest first. The Library singles show their state words.
- V6 (Part A): select the Haleen group row and click "Download". The progress item shows the MusicIndex singles. After each download, its row changes in place.
- V7 (Part B): open the Haleen group page from "Library" and from "Everything". The two pages are the same. The MusicIndex singles show after the Library singles load, newest first.
- V8 (Part B): an Index single page of Paul Birken shows "All n singles". The link opens the group page.
- V9 (Part B): with the network off, the group page shows the Library singles and the error row with "Try again".
- V10 (Part A and B): Light and Dark themes, at normal and narrow width. No row text and no action is clipped.

The ADR 0076 visual criterion "a group of Index singles holds each single of the feed on its first page" is V1 and V7.

## Exclusions

- Name level groups of MusicIndex singles (D6).
- The conflict case of D3, and the D-2 rule.
- A Labels destination and the "As <roles>" sections.
- A change of the scope "Library" (D15).
- A reload or an expiry of the loaded results (packet 016 decision D9).
- The search rows: packet 029.
- No schema change and no stored MusicIndex row.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/api.rs` (`NamedFeedEntry`, `Feed`, `Client::fetch_recent_feeds`, `Client::fetch_feeds_by_name`, `name_list_params`).
- `src/application/request_profiles.rs`, `src/application/queries/name_lists.rs`, `src/application/queries/feed.rs` (the `Fixture`).
- `src/view_models/named_feeds.rs`, `src/view_models/artist_identity.rs`, `src/view_models/release_kind.rs`, `src/views.rs` (the Library `NamedFeed`).
- `src/view_models/singles_group.rs`, `src/view_models/releases_everything.rs`, `src/view_models/index_name_list.rs`, `src/view_models/library.rs` (`releases_list`, `releases_selection_rows`, `releases_download_plan`).
- `src/view_models/selection_action_bar.rs`, `src/view_models/collection_art.rs`, `src/view_models/release_page.rs`, `src/view_models/entity_detail.rs`.
- `src/library.rs`, `src/library/music_scopes.rs`, `src/library/app_impl.rs`, `src/library/downloads.rs`.
- `src/ui/shells/library/destination_lists.rs`, `src/ui/shells/library/index_list_status.rs`, and the renderer of the group page (find it from `SinglesGroupPageVm`).
- `tests/architecture_tests.rs`: the contract guard, `adr_0084_scopes_r16_03_contract_fixture_is_0_12_0` and `adr_0084_singles_group_r15_09_only_the_owner_groups_singles`.
- Archived packet 015, packets 016, 017 and 022.

## Checks

```bash
cargo test --lib adr_0084_index_singles_
cargo test --lib adr_0084_singles_group_
cargo test --lib adr_0084_scopes_
cargo test --lib adr_0075_request_profile_
cargo test --lib adr_0084_download_queue_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo check --all-targets
touch src/main.rs && cargo build --bin v4vmm
```

## Rollback

Revert the commit of each part, Part B first. When Part A replaced the stored contract, its revert restores the 0.12.0 file. This packet adds no migration and no stored data.

## Size

Large. Part A changes about 14 files, with about 1,000 lines with tests. Part B changes about 8 files, with about 600 lines with tests. Each part runs in its own session. Each part leaves no code that no composition root reaches.

## Implementation Result, Part A - 2026-10-21

Part A makes Required Changes 2 to 10. Packet 029 made Required Change 1 (D1), so R26-01 is complete.

### What changed

- `src/api.rs`: `NamedFeedEntry` decodes `two_way_validated: Option<bool>`. `Client::fetch_releases_naming` sends `GET /v1/feeds/recent` with `named_feed_guid`, `limit`, the cursor and the profile include.
- `src/application/request_profiles.rs`: `RequestPathShape::RecentFeedList` and `INDEX_FEED_SINGLES` (`named_feeds`). The registry test is now `adr_0075_request_profile_registry_names_twelve_profiles`.
- `src/view_models/named_feeds.rs`: `index_named_feeds`.
- `src/view_models/singles_group.rs`: `index_release_identity`, `index_single_artist_feed`, `index_singles_of_feed`, `library_single_artist_feeds` and `singles_groups_everything`. `SinglesGroup` has `index_singles`, `members()` and `first_title_sort_key()`. The group row gives the `MusicIndex` members with `index_release_row`.
- `src/view_models/index_feed_singles.rs` (new): `IndexFeedSingles` with its generation, `ReleaseNamePageLoad` and `finish_release_page`.
- `src/view_models/index_name_list.rs`: a page can keep its rows apart after a failed feed request (`fail_pending`, `begin_pending_retry`). The error row names the failed request.
- `src/application/queries/name_lists.rs`: `FetchReleaseNamePage` reads the singles of each new artist feed of its page, and of each Library artist feed with singles on page 1. `fetch_feed_singles` reads at most 5 pages of 200 rows. At most 4 requests run at the same time, on scoped threads inside the command.
- `src/view_models/releases_everything.rs`, `src/view_models/library.rs`: "Everything" uses the new groups, the order position of the first single, and drops each `MusicIndex` row that is a group member. The download plan takes a title from `IndexFeedSingles` when no loaded page holds the release.
- `src/library.rs`, `src/library/music_scopes.rs`, `src/library/app_impl.rs`: `LibraryApp` keeps `IndexFeedSingles`. "Try again" of a page with waiting rows sends only the failed feed requests.

### Proof

| Case | Test |
|---|---|
| R26-01 | Packet 029: `adr_0084_scopes_r16_03_contract_fixture_is_0_13_0` and the contract field guard |
| R26-02 | `adr_0084_index_singles_r26_02_named_feed_entry_decodes_two_way_validated` |
| R26-03 | `adr_0084_index_singles_r26_03_index_named_feeds` |
| R26-04 | `adr_0084_index_singles_r26_04_identity_of_index_rows` |
| R26-05 | `adr_0084_index_singles_r26_05_a_label_at_rank_2_makes_no_group`, and the rank 2 row of the R26-07 test |
| R26-06 | `adr_0084_index_singles_r26_06_feed_singles_profile`, `adr_0084_index_singles_r26_06_feed_singles_follow_the_cursor_up_to_five_pages` |
| R26-07 | `adr_0084_index_singles_r26_07_page_reads_each_new_feed_four_at_a_time` |
| R26-08 | `adr_0084_index_singles_r26_08_library_feeds_of_singles`, and the Library feed of the R26-07 test |
| R26-09 | `adr_0084_index_singles_r26_09_a_failed_feed_request_fails_the_page`, `adr_0084_index_singles_r26_09_two_failures_name_the_first`, `adr_0084_index_singles_r26_09_retry_sends_only_the_failed_requests` |
| R26-10, R26-14 | `adr_0084_index_singles_r26_10_library_and_index_singles_make_one_group` |
| R26-11, R26-15 | `adr_0084_index_singles_r26_15_group_order_and_members` |
| R26-12, R26-13 | `adr_0084_index_singles_r26_12_one_single_and_name_level_make_no_index_group` |
| R26-16 | `adr_0084_index_singles_r26_16_group_text_with_no_library_single`, and the texts of the R26-10 and R26-15 tests |
| R26-17 | `adr_0084_index_singles_r26_17_open_group_gives_members_newest_first` |
| R26-18 | `adr_0084_index_singles_r26_18_group_selection_and_download_plan` |
| R26-19 | `adr_0084_index_singles_r26_19_library_groups_have_no_index_single`, the Library part of the R26-15 test, and the tests of packets 015 and 016 |
| R26-26 (A) | `adr_0084_singles_group_r15_09_only_the_owner_groups_singles`. Each new use of `ReleaseKind::Single` is in `src/view_models/singles_group.rs` |

The gate of 2026-10-21: `cargo test` gave 2,195 unit tests and 360 architecture tests, all Green. `cargo fmt -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo check --all-targets` and `cargo build --bin v4vmm` are Green.

### Deviations

- `singles_groups_everything` has the parameters `tree` and `fills`, with no `page`. The `MusicIndex` members of a group come from the loaded feed results only.
- Decision D11: the command reads 5 pages at most. Part A does not keep the fact that more rows exist, because no Part A surface shows it. Part B adds this fact for the group page.
- Two or more feed requests of one page can fail. The error row then names the first failed request and gives the count of the other failed requests.

### Live measurement - 2026-10-21

The implementer sent read-only GET requests to `https://api.musicindex.org` from 17:52:52 UTC to 17:52:57 UTC. At most 4 feed requests ran at the same time:

- Page 1: the name list request took 1.06 s. The 37 feed requests took 2.36 s.
- Page 2: the name list request took 0.20 s. The 28 feed requests took 1.06 s.

These times agree with the estimate of decision D4. The requests for the Library artist feeds on page 1 are not in this measurement.

### What Part B still needs

- Required Changes 11 to 16 and the cases R26-20 to R26-25.
- The group page reads `singles_groups` only. Until Part B, a group row of "Everything" with no Library single opens the empty state, and a group page shows the Library singles only.
- The link "All n singles" uses the Library groups only.
- `IndexFeedSingles` has a generation for each loaded feed. Part B adds the load state of a running request and of a failed request. The group page and the release page use these states. Part B also adds the fact that more rows exist (D11).

## Orchestrator Review, Part A - 2026-10-21

Pass, with two corrections that the orchestrator made.

1. Each page command now reads the Library artist feeds with singles, not only page 1. Before, page 1 could start before the Library loaded, or a download could add a Library single later. That single then showed as a row, and a later page moved it into a group (D10). The command skips each loaded feed, so the app still reads each feed one time. The guard `adr_0084_index_singles_each_page_reads_the_library_feeds` pins this.
2. `finish_release_page` now records no feed singles when the page generation is old. The test `adr_0084_index_singles_old_page_result_records_no_singles` proves it. The unused `append` value of `begin_pending_retry` is deleted.

Part B gave the group page its MusicIndex members on 2026-10-21, so a group with no Library single opens a full page.

## Implementation Result, Part B - 2026-10-21

Part B makes Required Changes 11 to 16.

### What changed

- `src/view_models/index_feed_singles.rs`: `FeedReleaseList` holds the rows and the D11 fact `more`. `IndexFeedSingles` keeps the state of the request of a page for each feed: running or failed. `state`, `needs_request`, `begin_feed_request` and `finish_feed_request` give and change this state. A recorded result ends the request state, so a later answer of that request changes nothing.
- `src/application/queries/name_lists.rs`: `fetch_feed_singles` records `more` when the fifth page has `has_more` true. The new command `FetchFeedSingles` reads the singles of one feed.
- `src/view_models/singles_group.rs`:
  - `SinglesGroupPageVm::from_tree` takes `IndexFeedSingles` and reads the groups of "Everything" in each scope (D12).
  - Each row is a `SinglesGroupPageEntryVm`: a Library row keeps the shared track row, and a `MusicIndex` row is `SinglesGroupIndexRowVm` with the image, the title and the date (D13).
  - The page gives `index_status`, the loading row or the error row with "Try again", and `more_note` (D11). While the request runs or after it failed, a feed with one Library single shows that row and not the empty state.
  - `SinglesGroupAction::DownloadSingles` holds a `ReleasesDownloadPlan`: the Library feed ids and the `MusicIndex` feed GUIDs with titles, newest first.
  - The empty state text is "This artist has fewer than two singles." (D12).
  - `single_page_feed_to_read`, `library_page_feed_to_read`, `index_page_feed_to_read` and `group_page_feed_to_read` tell which feed a page must read.
- `src/view_models/library.rs`: `LibraryViewModel` keeps `IndexFeedSingles`. `singles_group_page` and `release_page_name_links` read the groups of "Everything" (D12, D14). `BusyItems::index_release_downloading` tells if a `MusicIndex` single is in the download queue.
- `src/library/music_scopes.rs`: `request_singles_group_feed`, `retry_singles_group_feed`, `request_library_single_feed` and `request_index_single_feed` send `FetchFeedSingles` through `present_command`, with the generation check.
- `src/library/app_impl.rs`: each open of a group page and each open of a Library release page starts the request when the app has no result. `src/app/search_dispatch.rs` starts it when an Index release page has its feed.
- `src/library/downloads.rs`: `download_release_plan` queues a plan. The Releases action bar and the group page use it.
- `src/ui/shells/library/singles_group.rs`: the `MusicIndex` rows, with their selection checkbox, and the status row of `index_list_status.rs`. `checkbox_state_for` of `src/ui/composites/release_section_list.rs` is now shared.
- `src/view_models/detail_selection.rs`, `src/library/release_page.rs`: the detail selection holds the `MusicIndex` rows of the group page as release rows.

### Proof

| Case | Test |
|---|---|
| R26-20 | `adr_0084_index_singles_r26_20_group_page_gives_both_origins_newest_first` |
| R26-21 | `adr_0084_index_singles_r26_21_loading_and_error_rows_keep_the_library_rows`, `adr_0084_index_singles_page_command_reads_one_feed` |
| R26-22 | `adr_0084_index_singles_r26_22_main_action_downloads_both_origins` |
| R26-23 | `adr_0084_index_singles_r26_23_single_page_links_to_its_feed_group`, `adr_0084_index_singles_page_command_reads_one_feed` |
| R26-24 | `adr_0084_index_singles_r26_24_old_answers_change_nothing` |
| R26-25 | `adr_0084_index_singles_r26_25_empty_state_text` |
| D11 | `adr_0084_index_singles_d11_group_page_says_more_exist`, and the `more` assertion of `adr_0084_index_singles_r26_06_feed_singles_follow_the_cursor_up_to_five_pages` |
| R26-26 (B) | `adr_0084_singles_group_r15_09_only_the_owner_groups_singles`. Each new use of `ReleaseKind::Single` is in `src/view_models/singles_group.rs` |

The gate of 2026-10-21: `cargo test` gave 2,204 unit tests and 361 architecture tests, all Green. `cargo fmt -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo check --all-targets` and `cargo build --bin v4vmm` are Green.

### Deviations

- `IndexFeedSingles` moved from `LibraryApp` to `LibraryViewModel`. The Library release page and the group page get their facts from `LibraryViewModel`, so this change adds no parameter to the renderers. `LibraryApp` still changes it only through `present_command` results.
- The status row of the group page uses `IndexListStatusRowVm` and its shared renderer. The page gives no "Load more" row.
- A feed that the app did not read yet shows the loading row. The page sends its request when it opens.
- The main action stays available while the request runs. Its count then holds the Library singles only, and it changes in place when the result arrives.
- A failed request of a single page gives no error row. The page shows no "All n singles" link, and the next open of the page sends the request again.

## Orchestrator Review, Part B - 2026-10-21

Pass, with corrections that the orchestrator had made.

1. A Library single page sends its feed request only when the page opens. A refresh of the open page and the end of a download use `refresh_album_detail`, which sends no request. The guard `adr_0084_index_singles_one_singles_request_for_each_page_open` pins this.
2. A Releases page command does not read a feed whose page request runs (`loaded_or_running_feeds`). A unit test proves it.
3. The status row of the group page has no "Load more" action. `IndexListActions::load_more` is optional, and no closure does nothing.
4. The D11 note names the 1,000 newest releases. A unit test ties that number to `FEED_SINGLES_MAX_PAGES` and `FEED_SINGLES_PAGE_LIMIT`.
5. The view model gives the checkbox id of a MusicIndex row.

## Operator Visual Check

Walk this check in a desktop session after the commit of Part A. The machine must reach `https://api.musicindex.org`.

1. Run `cargo build --release`, then run `target/release/v4vmm`.
2. Open Music, then Releases. Select the scope "Everything".
3. V1: find the group row "Haleen". It shows "n singles · 3 in Library · Artist feed", with n about 55. No Haleen single of that feed shows as its own row. A Haleen single row of that feed is wrong.
4. V2: find the second group row "Haleen". It shows "38 singles · Artist feed". "You Are Enough" shows as its own row. A Library part in the text of this row is wrong.
5. V3: find the group row "Paul Birken". Sir Libre Records has no group row.
6. V4: scroll to the end and click "Load more". The new rows show at the end. A row that showed before and moves, or that goes into a group, is wrong.
7. V5: click the chevron of the Haleen group row. The singles show newest first. Each Library single shows its state word.
8. V6: click "Select", select the Haleen group row, and click "Download". The progress item shows the MusicIndex singles. After each download, its row changes in place.
9. V10: do steps 3 to 7 in the Light theme and in the Dark theme. Do them at normal width and at narrow width. Clipped row text or a clipped action is wrong.

Part B (V7 to V10). Walk these steps after the commit of Part B, in a new start of the app. The machine must reach `https://api.musicindex.org`, except in step 14.

10. V7: open Music, then Releases, with the scope "Library". Click the group row "Haleen", not its chevron.
    - A loading row "Loading MusicIndex singles" shows below the Library singles.
    - Then the MusicIndex singles show, newest first, in one list with the Library singles.
    - A Library single keeps its track row. A MusicIndex single shows its image, its title and its date, with no state word.
    - The subtitle shows about 55 singles, and the main action is "Download n singles". Note the rows.
11. V7: select the scope "Everything" and click the group row "Haleen" again. The page is the same as in step 10. A different count or different rows is wrong.
12. V7: click a MusicIndex single row. Its release page opens. Go back.
13. V8: in "Everything", open the chevron of the group row "Paul Birken". Click one single.
    - Its Index release page shows the link "All n singles". A single page with no link is wrong.
    - Click the link. The group page "Singles by Paul Birken" opens.
14. V9: stop the app. Run `nmcli networking off`, or remove the network cable. Start `target/release/v4vmm`.
    - Open Releases with the scope "Library", and click the group row "Haleen". The Library singles show.
    - An error row shows below them with "Try again". Its detail names `GET /v1/feeds/recent?named_feed_guid=64569161-a8f7-52e5-8aae-9e0385043149`.
    - An empty page or no "Try again" is wrong.
    - Run `nmcli networking on`, and click "Try again". The MusicIndex singles show.
15. V10: do steps 10 to 13 in the Light theme and in the Dark theme. Do them at normal width and at narrow width. Clipped row text, a clipped date or a clipped action is wrong.

Cleanup: the downloads of step 8 add releases to the Library. Remove them from the Library when you do not want to keep them. After step 14, make sure that `nmcli networking on` ran.

## Prompt for lower-context coding model

You are implementing one bounded part of a larger plan.

Implement only the part that the orchestrator names: Part A or Part B. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0084-task-026-index-singles-groups.md`, with the operator answers to the flagged decisions
- ADR 0084 Decisions 4, 6, 8, 10 and 12, and ADR 0085 Decisions 2 and 6
- Each file in "Files To Inspect"

Goal:
- Part A: make Required Changes 1 to 10.
- Part B: make Required Changes 11 to 16. Part A must be complete.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Follow each Orchestrator Decision and each operator answer. Do not decide a flagged point again.
- The view model gives each text, count, state and availability. The screen only composes. Use named tokens.
- Keep each use of `ReleaseKind::Single` in `src/view_models/singles_group.rs` or `src/view_models/release_kind.rs`.
- Never call `cx.spawn` from a screen. Use `present_command`.
- Send one list request for each feed, never one request for each row. Read no release detail for a row.
- Tests use recorded rows. No test sends a request to `api.musicindex.org`.
- When you replace the stored contract, get it with one GET request to `https://api.musicindex.org/openapi.json`. Send no other request.
- Never run `git add`, `git rm`, `git commit`, `git checkout`, `git restore`, `git stash` or `git reset`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The rules of the scope "Library", the Artists list, and the feed page.
- The download queue rules of packet 022.
- The database schema and the migration registry.
- Any ADR. Each document other than this packet and the one AGENTS.md "Stored facts" line.

Acceptance criteria:
- Each case of the part has a passing test or guard.
- Each command in "Checks" is Green.
- The packet has an "Implementation Result" section for the part, with the date, and an "Operator Visual Check" section for its V items.

Test commands:
- The commands in "Checks", with `cargo fmt -- --check` after the last edit and `cargo build --bin v4vmm` last.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
6. the document changes that the orchestrator must make

Stop and report the problem, and do not guess, when:
- The contract from `GET /openapi.json` differs from the Live API facts of this packet.
- A row needs a detail request to show its text or its group.
- The command runner cannot run four requests at the same time inside one command.
- A change needs a file in "Do not touch".
