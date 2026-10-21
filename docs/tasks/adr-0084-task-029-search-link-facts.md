# ADR 0084 Task 029: The Link Facts Of A Feed Row In Search

Status: Ready - 2026-10-21. The operator answered decision D2 on 2026-10-21. The orchestrator decided D3 and D5 by their recommendations. Run it after packet 028 and before packet 026. It replaces the stored contract with 0.13.0.

## Goal

A feed row in the Artists tab of search tells an artist from a label, with no read for each row.
The `type=publisher` search request asks for the link facts of each row. A row then shows the badge "Artist" for a confirmed or an unverified artist.
A row shows the role words, for example "Label", for a feed with agreed roles without `artist`. Each other feed row shows "Feed", as today.

## Authority

- [ADR 0085](../adr/0085-artists-by-confirmation.md):
  - Decision 1: no interface text names a feed "publisher".
  - Decision 2: the three confirmation levels. The Artists list shows no level word.
  - Decision 7: the frame title of a feed page names its relation, for example "Artist" or "Label". An unknown rank never gives "Publisher".
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 2: search results in the list column. Decision 5: the kind badge of a release.
- [ADR 0075](../adr/0075-metadata-contract.md) section 6: named request profiles, no read for each list row, and the stored contract changes in the same change as the decode.
- Packet [010](adr-0084-task-010-search-results-list-column.md): decision D5 and the operator answer of 2026-10-20. Publisher rows stay "Feed" until Stophammer request 9 item C gives link facts on search rows.
- [Open Stophammer requests](../plans/v4vmm-open-requests.md): request 9 item 3, "Stophammer Answers - 2026-10-21" and "Release 0.13.0 - 2026-10-21". Stophammer ADR 0076 §3 owns the include.
- The open finding of 2026-10-21 in the [phase plan](../plans/adr-0084-music-column-browser-phase-plan.md): a feed row in search results shows no level word and no role word.

## Recorded Facts - 2026-10-21

### Code facts

The code facts come from the working tree of 2026-10-21. Packet 028 changes the search files at the same time. Find each call site again after its commit.

- `fetch_index_feed_page_result_rows` in `src/application/queries/search.rs` sends `GET /v1/search?type=publisher` through `Client::search(query, Some("publisher"), Some(PAGE_LIMIT), None)`. `PAGE_LIMIT` is 20.
- `Client::search` in `src/api.rs` sends `q`, `limit`, `type` and `cursor`. It sends no `include`, and it has no request profile.
- `SearchResult` in `src/api.rs` decodes no link fact.
- `index_feed_page_result_display` gives the row id `index-feed-page:<entity_id>`, the label, the accessibility label "Open the feed page <title>" and the image.
- `artist_row_vm` in `src/view_models/search_results/rows.rs` gives the badge of a row whose target is `SearchResultTarget::FeedPage`. The badge is `SearchRowBadge::Artist` when `SearchLibraryLookup::is_artist_feed` is true, else `SearchRowBadge::Feed`.
- `SearchRowBadge` is `Copy`. Its `label()` gives a `&'static str`. `entity_kind_for_badge` in `src/ui/shells/search_result_rows.rs` maps each badge to an `EntityKind`.
- A Library artist feed row (`artist-feed:<guid>`) and a MusicIndex row (`index-feed-page:<guid>`) both give `SearchResultTarget::FeedPage`. They merge by feed GUID, and the Library row stays.
- `classify_index_artist` in `src/view_models/artists_everything.rs` classifies a `publisher` row of `GET /v1/artist-names` by `agreed_roles` and `first_named_count` (packet 016 decision D11).
- `PublisherPageVm::frame_title` in `src/view_models/publisher_page.rs` gives the role words with `capitalize_first(&role_list_text(&words))` from `src/view_models/role_words.rs`. Example: "Label".
- `RequestPathShape` in `src/application/request_profiles.rs` has no shape for `/v1/search`. The registry test counts ten profiles.
- The stored contract is `tests/fixtures/musicindex-openapi-0.12.0.json`. Its `GET /v1/search` declares no `include`.

### Live API facts

The packet author sent read-only GET requests to `https://api.musicindex.org` on 2026-10-21, from 16:17 UTC to 16:24 UTC.

- `GET /node/info` gave `git_revision` `b00f529`. `GET /openapi.json` gave `info.version` `0.13.0`, 104,418 bytes, sha256 `eae53d3836f5c68a1eff3a8ab1e0a205ba3b0cc6353a84aff41dea90f6a8df2d`.
- The 0.13.0 contract declares `include` on `GET /v1/search`, with the one name `link_facts`. `SearchResponseItem` adds `two_way_link_count`, `stated_rels`, `agreed_roles`, `confirmed_release_artists` and `first_named_count`. Each is nullable.
- `GET /v1/node/capabilities` gives `include_params.search` `["link_facts"]`.
- Without the include, a `publisher` row gives only `entity_id`, `entity_type`, `episode_count`, `feed_image_url`, `quality_score`, `rank`, `release_artist`, `release_artist_source` and `title`.
- With `include=link_facts`, a `feed` row and a `track` row give no link fact. A search with no `type` gave no link fact on its rows.

`GET /v1/search?q=<q>&type=publisher&include=link_facts&limit=20`:

| Query | Row | `two_way_link_count` | `agreed_roles` | `first_named_count` | Class by ADR 0085 Decision 2 |
|---|---|---|---|---|---|
| `Libre` | Sir Libre Records (`34bf795f-f551-4db1-8877-ad19aa5f89c8`) | 3 | `["label"]` | 0 | Not an artist. Agreed role "label" |
| `Libre` | Kolomona Myer - AKA Sir Libre (`c8ec3dba-ed28-5662-9509-094d769f9597`) | 1 | `[]` | 1 | Unverified artist |
| `haleen` | Haleen (`fe80a515-2826-4a46-aa37-b27f38d3c30f`) | 38 | `[]` | 38 | Unverified artist |
| `haleen` | Haleen (`64569161-a8f7-52e5-8aae-9e0385043149`), the Library artist feed | 56 | `[]` | 56 | Unverified artist |
| `paul birken` | Paul Birken (`7513ecb9-19e7-4df0-98e3-e13909e09533`) | 143 | `[]` | 143 | Unverified artist |
| `heycitizen` | HeyCitizen (`8a106e41-c54d-451e-9f35-a8834b62fe0e`) | 2 | `["artist", "author", "host", "label", "producer"]` | 2 | Artist |
| `detox` | Official DETOX Music (`137aaa9c-75ff-4916-9f23-e02968b2d15e`) | 30 | `[]` | 30 | Unverified artist |
| `tj wong` | TJ Wong (`a31f362c-d40b-5f56-ae2c-4ef9c548db9d`) | 0 | `[]` | 0 | Not an artist. No confirmed link names it first |

- The five values of Sir Libre Records are the values of the same feed on `GET /v1/feeds/by-name?medium=publisher&include=link_facts` ("Release 0.13.0 - 2026-10-21").
- `GET /v1/search?q=a&type=publisher&include=link_facts&limit=100` gave 100 rows: 97 unverified artists and 3 rows with no confirmed link that names them first.
- The cost of the include, at 16:24 UTC:
  - `q=haleen`, `limit=20`: 0.055 s to 0.066 s with the include, and 0.045 s to 0.056 s without it.
  - `q=a`, `limit=20`: 0.074 s to 0.082 s with the include, and 0.060 s to 0.066 s without it.
  - `q=a`, `limit=100`: 1.20 s with the include, and 0.06 s without it. v4vmm sends `limit=20`.

**The stored contract (orchestrator decision of 2026-10-21).** Packets 026 and 029 each need the 0.13.0 contract. The first of the two that an implementer starts replaces the 0.12.0 file. The orchestrator proposes this packet first. Packet 026 then finds the 0.13.0 file and changes only its own decode.

## Design

### The request

- Add `RequestPathShape::Search` and the profile `INDEX_PUBLISHER_SEARCH` (`link_facts`) to `src/application/request_profiles.rs`.
- `Client::search` takes an optional profile. With a profile, it sends `include` with the profile value. The `type=publisher` request of `fetch_index_feed_page_result_rows` uses the profile.
- The `type=feed` and `type=track` requests do not change. They send no include.
- The app does not read `GET /v1/node/capabilities` first. The node ignores an unknown include name, and the decode accepts a row with no link fact (decision D4).

### The decode

- `SearchResult` adds `agreed_roles: Option<Vec<String>>` and `first_named_count: Option<i64>` (decision D6).

### One classification for a feed

- Add `FeedLinkClass` and `FeedLinkClass::from_link_facts(agreed_roles, first_named_count)` to `src/view_models/artists_everything.rs`. It gives:
  - `Artist` when an agreed role is `artist` (read by `role_contains_artist`)
  - `OtherRoles(Vec<String>)` when the agreed roles are not empty and hold no `artist`
  - `UnverifiedArtist` when the agreed roles are empty and `first_named_count` is above 0
  - `NotNamedFirst` when the agreed roles are empty and `first_named_count` is 0 or null
- `classify_index_artist` uses this function for a `publisher` row. The Artists list and search thus use one rule.
- `ArtistResultDisplay` adds `link_class: Option<FeedLinkClass>`. `index_feed_page_result_display` sets it when the hit gives `agreed_roles` or `first_named_count`. Each other row has `None`.

### The badge

`artist_row_vm` gives the badge of a feed row by this sequence:

| Row facts | Badge |
|---|---|
| The Library Artists list has a feed level row of the GUID | "Artist" |
| `link_class` is `Artist` or `UnverifiedArtist` | "Artist" |
| `link_class` is `OtherRoles(words)` | The role words with a capital first letter, for example "Label" or "Label and producer" |
| `link_class` is `NotNamedFirst` | "Feed" |
| No `link_class` | "Feed", the rule of packet 010 |

- `SearchRowBadge` adds `Roles(String)`. It is thus not `Copy`, and `label()` gives a `&str`.
- The role text comes from `capitalize_first(&role_list_text(&role_words(...)))`, the path of `PublisherPageVm::frame_title`. The badge of a label thus equals the frame title of its feed page.
- `entity_kind_for_badge` maps `Roles` to `EntityKind::Feed`. The text tells the badges apart, so color is not the only difference.
- The accessibility label of a feed row adds the badge word, for example "Open the feed page Sir Libre Records, Label" (decision D7).
- No row shows a level word, a count or the word "publisher".
- The rows keep their order: the Library rows, then the MusicIndex rows in rank order.

## Orchestrator Decisions

"**Flag.**" marks a decision that the operator confirms before an implementer starts. Each one gives the options and the recommendation of the orchestrator.

- **D1. The contract fixture.** The first of packets 026 and 029 that runs replaces `tests/fixtures/musicindex-openapi-0.12.0.json` with `tests/fixtures/musicindex-openapi-0.13.0.json`. The new file holds the bytes of `GET /openapi.json`.
  - The same change updates `MUSICINDEX_CONTRACT_FIXTURE` and its comment, and the `CONTRACT_TYPE_MAP` comment.
  - It replaces the test `adr_0084_scopes_r16_03_contract_fixture_is_0_12_0` with a test that asserts `0.13.0`, maps the same four types, and finds no 0.12.0 file.
  - It updates the AGENTS.md "Stored facts" line.
- **D2. The badge words.** **Flag.**
  - Option A: "Artist" for a confirmed and an unverified artist, the role words for a feed with other agreed roles, and "Feed" for each other feed.
  - Option B: Option A, with "Unverified artist" for an unverified artist. On 2026-10-21, 97 of 100 rows of a broad search were unverified artists, so the word repeats on almost each row. The operator rejected a level word on each Artists row on 2026-10-18 (ADR 0085 Alternatives).
  - Option C: Option A, and no row in the Artists tab for a feed that is not an artist. Then a curator cannot find a label by its name, which is flow map finding 6.
  - **Recommendation: Option A.** It follows ADR 0085 Decision 2 and the frame titles of Decision 7.
- **D3. The Library decides first.** **Flag, minor.** A feed that the Library Artists list holds keeps "Artist", whatever the link facts say. The Library row wins the merge, and its feed page opens in the Library context.
  - Alternative: the link facts decide each MusicIndex row.
  - **Recommendation:** the Library decides first. The badge then agrees with the Artists list of the Library.
- **D4. No capability read.** The app sends the include with each `type=publisher` request. A node with no such include gives rows with no link fact, and those rows show "Feed", as today.
- **D5. The order of the rows does not change.** **Flag, minor.**
  - Option A: the rank order of MusicIndex. A label row stays at its rank, and its badge tells it apart.
  - Option B: the artist rows first, then the other feed rows.
  - **Recommendation: Option A.** The ADR 0076 visual criterion asks that search show a label apart from the artists, and the badge does this.
- **D6. Decode two fields.** The class reads `agreed_roles` and `first_named_count` only. The other three fields are not decoded, because no code reads them.
- **D7. The accessibility label names the badge.** A screen reader then tells "Label" or "Artist" with the row.
- **D8. The saved search page.** The saved search page of packet 012 uses the same row view model. It thus shows the same badges with no other change.
- **D9. The cost.** The include goes on the `type=publisher` request only. On 2026-10-21, it added about 0.01 s to 0.02 s at `limit=20`.

## Operator Answers - 2026-10-21

- D2: option A. The badge is "Artist" for a feed that albums name through a confirmed link, the role word for a feed with an agreed role, for example "Label", else "Feed".
- The orchestrator decided D3 and D5 by their recommendations, by the operator rule of 2026-10-20 ("decide but keep a record"). D3: a feed of the Library Artists list keeps "Artist". D5: the order of the rows does not change.


## Required Changes

1. When this packet runs before packet 026, replace the stored contract (D1). Else use the 0.13.0 file that packet 026 added.
2. Decode the two fields of `SearchResult`.
3. Add `RequestPathShape::Search` and `INDEX_PUBLISHER_SEARCH`. Add one to the count of the registry test.
4. Add the profile argument to `Client::search`. The `type=publisher` request uses the profile.
5. Add `FeedLinkClass` and use it in `classify_index_artist`.
6. Add `link_class` to `ArtistResultDisplay`, and set it in `index_feed_page_result_display`.
7. Add `SearchRowBadge::Roles` and the badge sequence to `artist_row_vm`. Add the badge word to the accessibility label of a feed row.
8. Map `Roles` in `entity_kind_for_badge`.
9. Change the comments that say that a search row gives no link fact.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_search_link_facts_` for unit tests beside the owning code. Each test uses a recorded row of 2026-10-21 or a constructed value. No test sends a request to `api.musicindex.org`. A request test uses a local fixture server.

| Case | Required proof |
|---|---|
| R29-01 | The contract test asserts `0.13.0`, maps the four name list types, and finds no 0.12.0 file. The contract field guard passes. Packet 026 can own this case when it runs first |
| R29-02 | `SearchResult` decodes `agreed_roles` and `first_named_count` from the recorded Sir Libre Records row and the recorded Kolomona Myer row. A row with no link fact decodes with `None` |
| R29-03 | The `type=publisher` request sends `include=link_facts`. The `type=feed` and `type=track` requests send no `include` |
| R29-04 | `INDEX_PUBLISHER_SEARCH` names `RequestPathShape::Search` and `link_facts`. The registry test counts one more profile |
| R29-05 | `FeedLinkClass::from_link_facts` gives each row of the table of "One classification for a feed". The recorded rows of the Live API facts give their class |
| R29-06 | `classify_index_artist` gives the same results as before. The packet 016 test `adr_0084_scopes_r16_08_classification` passes |
| R29-07 | The badge sequence gives "Label" for Sir Libre Records, "Artist" for Kolomona Myer, HeyCitizen and each Haleen row, and "Feed" for TJ Wong |
| R29-08 | A feed that the Library Artists list holds gives "Artist" also with the link facts of a label (D3). A row with no link fact gives "Feed" |
| R29-09 | The accessibility label of a feed row names its badge word. No label, badge or accessibility label of the view model contains "Publisher", "publisher" or "Unverified" |
| R29-10 | Each test of packets 010, 012 and 028 still passes |

## Visual Acceptance Criteria

These are for the operator, in a walk after this packet. No test proves them.

- V1: search "Libre", Artists tab. Sir Libre Records shows the badge "Label". "Kolomona Myer - AKA Sir Libre" shows "Artist". A click on Sir Libre Records opens its feed page with the frame title "Label".
- V2: search "haleen", Artists tab. Each of the two "Haleen" rows shows "Artist".
- V3: search "TJ Wong", Artists tab. The row shows "Feed".
- V4: search "HeyCitizen", Artists tab. The row shows "Artist".
- V5: open a saved search that finds Sir Libre Records. The row shows "Label".
- V6: Light and Dark themes, at normal and narrow width. A role badge is not clipped, and the title keeps its width.

The ADR 0076 visual criterion "the search shows a label feed apart from the artist feeds" is V1.

## Exclusions

- No level word and no count on a search row (D2).
- No change to the Releases tab and the Tracks tab.
- No change to the order of the rows (D5).
- The two "Haleen" rows show the same title and the same badge. Their images tell them apart (packet 028 D4). This packet does not change that.
- No change to the query text (Stophammer request 10).
- No schema change and no stored MusicIndex row.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/api.rs` (`SearchResult`, `Client::search`), `src/application/request_profiles.rs`.
- `src/application/queries/search.rs` (`fetch_index_feed_page_result_rows`, `index_feed_page_result_display`, the request tests of packet 010).
- `src/view_models/search_results/results.rs` (`ArtistResultDisplay`), `src/view_models/search_results/rows.rs` (`SearchRowBadge`, `artist_row_vm`), `src/view_models/search_results/tests.rs`.
- `src/view_models/artists_everything.rs`, `src/view_models/role_words.rs`, `src/view_models/publisher_page.rs` (`frame_title`).
- `src/ui/shells/search_result_rows.rs` (`entity_kind_for_badge`).
- `tests/architecture_tests.rs`: the contract guard and `adr_0084_scopes_r16_03_contract_fixture_is_0_12_0`.
- Packets 010, 012, 016 and 028.

## Checks

```bash
cargo test --lib adr_0084_search_link_facts_
cargo test --lib adr_0084_search_list_
cargo test --lib adr_0084_saved_search_
cargo test --lib adr_0084_scopes_
cargo test --lib adr_0075_request_profile_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo check --all-targets
touch src/main.rs && cargo build --bin v4vmm
```

## Rollback

Revert the commit. When this packet replaced the stored contract, the revert restores the 0.12.0 file. This packet adds no migration and no stored data.

## Size

Small. About 9 files, with about 350 lines with tests. One session.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0084-task-029-search-link-facts.md`, with the operator answers to the flagged decisions
- ADR 0085 Decisions 1, 2 and 7, and packet 010 decision D5
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes".

Constraints:
- Packet 028 must be committed before you start.
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Follow each Orchestrator Decision and each operator answer. Do not decide a flagged point again.
- The view model gives each text, badge and accessibility label. The screen only composes. Use named tokens.
- Send no request for a row. Read no feed detail for a search row.
- Tests use recorded rows. No test sends a request to `api.musicindex.org`.
- When you replace the stored contract, get it with one GET request to `https://api.musicindex.org/openapi.json`. Send no other request.
- Never run `git add`, `git rm`, `git commit`, `git checkout`, `git restore`, `git stash` or `git reset`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The Releases tab, the Tracks tab and the row order.
- The Artists list rules of packet 016, apart from the shared class function.
- The database schema and the migration registry.
- Any ADR. Each document other than this packet and the one AGENTS.md "Stored facts" line.

Acceptance criteria:
- Each case R29-01 to R29-10 has a passing test or guard.
- Each command in "Checks" is Green.
- The packet has an "Implementation Result" section with the date and an "Operator Visual Check" section for V1 to V6.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
6. the document changes that the orchestrator must make

Stop and report the problem, and do not guess, when:
- The contract from `GET /openapi.json` differs from the Live API facts of this packet.
- A row needs a second request to show its badge.
- A change needs a file in "Do not touch".
