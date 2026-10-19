# ADR 0084 Task 008: The Feed Page And Artists By Confirmation

Status: Implemented - 2026-10-18. Cases R85-8-01 to R85-8-16 pass. Mechanical checks are Green. The visual gate is open. No operator walked V1 to V9 yet.

## Goal

The page of a feed shows its confirmation level in words. Its releases go in one section for each relation, and no interface text calls a feed a "publisher".
The Artists list holds the confirmed artists, the unverified artists and the names, with no level word on a row.

## Authority

- [ADR 0085](../adr/0085-artists-by-confirmation.md) Decision 1: no interface text names a feed "publisher".
- [ADR 0085](../adr/0085-artists-by-confirmation.md) Decision 2: the three levels. The Artists list shows no level word. Only the artist page shows the level.
- [ADR 0085](../adr/0085-artists-by-confirmation.md) Decision 3: the sections, their sequence and their row text.
- [ADR 0085](../adr/0085-artists-by-confirmation.md) Decision 7: the frame title, and the one use of "Publisher".
- [ADR 0085](../adr/0085-artists-by-confirmation.md) Decision 4, and [ADR 0082](../adr/0082-publisher-roles-belong-to-each-album-link.md) Decisions 4 and 5: the header roles and "Shares albums with" stay.
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 7: the feed level of the artist page. Decision 4: one page for each item, and the Library is a state. Decision 6: "Albums and EPs" before "Singles". Decision 5: the kind of a release.
- [ADR 0075](../adr/0075-metadata-contract.md) section 6: the stored contract changes in the same change as its decode.
- Operator decisions of 2026-10-18:
  - The old packet 008 is two packets. This packet builds the feed level and the Artists list. Packet 017 builds the name level, and replaces the Library artist page by name and the Index name match page.
  - Packet 010 owns flow map finding 6, the search entry of a feed page.
  - This packet also changes the three Music strings of Required Change 1.
  - The packet uses the facts of request 7. The app sends no request for each album of a feed page.

## Recorded Facts - 2026-10-18

- `PublisherPageVm` in `src/view_models/publisher_page.rs` exposes the groups of ADR 0082 packet 002: `AlbumNameGroup` ("Publisher of", "Listed, not named"), `RoleSubgroupCaption`, the row role text of `AlbumRoleDisplay`, `library_scopes()` ("Library Albums", "Other Albums"), `header_facts()`, `shares_albums_with()`, and the derived artist counts.
- The Index feed page reads one feed response with `publisher` entries (`fetch_index_publisher_page_albums` in `src/application/queries/feed.rs`). Each album row comes from a `publisher_to_music` entry. The Library page uses the same view model with the Library context.
- The stored contract is `tests/fixtures/musicindex-openapi-0.7.0.json`. Its `PublisherResponse` gives no rank and no track count.
- Stophammer release 0.11.0 is deployed. On 2026-10-18, `GET /node/info` gave `git_revision` `99e90f7`, and `/openapi.json` gave `info.version` `0.11.0`.
  Each `PublisherResponse` gives `album_names_rank` (integer or null, from 1) and `remote_episode_count` (integer or null).
- `album_names_rank` is null when `album_names_as` is null (Stophammer ADR 0069 §3a). `remote_episode_count` is null when the node holds no feed for the entry (ADR 0059 §1).
- Stophammer declined `remote_release_kind`. Each feed stores `release_kind` `unknown`, because RSS states no album, EP or single kind.
- The release record of the [open requests](../plans/v4vmm-open-requests.md#release-0110---2026-10-18) gives two examples. The DETOX feed `137aaa9c-75ff-4916-9f23-e02968b2d15e` gives rank 1 on each of its 30 rows.
  6 albums name two feeds. Each one gives rank 1 to the artist and rank 2 to Sir Libre Records.
- `LibraryViewModel::artists_list` (`src/view_models/library.rs:2737`) gives one row for each `ArtistNode` of the tree, by artist text. Its secondary text is "n albums". Each row opens `DestinationListRowTarget::Artist(name)`.
- The screen is `src/ui/shells/publisher.rs`. Its header uses `EntityKind::Publisher` (label "publisher", monogram "PB"). `src/ui/composites/thumbnail.rs:189` and `src/ui/contrast.rs` also use the publisher entity kind.
- `FrameNavigationEntry::PublisherDetail` has the frame title "Publisher" (`src/app.rs:1020`). `open_publisher_page` writes "Failed to navigate to publisher: …" (`src/app/publisher_dispatch.rs:87`).
- Music strings that name a publisher, outside the feed page:
  - `src/view_models/singles_group.rs`: "Publisher feed" and "Grouped by the publisher feed that each single names."
  - `src/view_models/track_detail.rs:93`: the label "Publisher" of the `publisher_text` row. The release page names the same value "Feed owner".
  - `src/view_models/playlist_rss_check.rs:500` and `:591`: the field "Publisher" and "Publisher feed {} at {url}".
  - `src/view_models/library.rs:72`, `:3608` and `:3641`: "Publisher must add payment routes." and the summary "n track(s) needs publisher routes".
- The Show section and the startup reports use "publisher" for the `musicindex-live-publisher` broadcast service. That word names a service, not a feed. ADR 0085 Decision 1 does not apply to it.
- ADR 0082 packet 003 adds `named_feeds.rs`, `role_words.rs`, migration 19 and the artist feed selection. This packet uses them.
- The operator found two gaps on 2026-10-18, in the check of ADR 0082 packet 003:
  - A release row of the feed page opens nothing. `render_album_row` in `src/ui/shells/publisher.rs` gives the row no `on_click`. `PublisherPageAlbumVm` has `feed_guid`, but no open action. ADR 0077 packet 004 recorded "Row navigation stays a follow-up", and no packet took it.
  - The header shows the monogram "PB", because `render_header` gives `DetailHeader` no image. `PublisherPageFacts` has no image of the feed itself. The Index path (`publisher_page_facts_from_feed` in `src/application/queries/feed.rs`) does not read `api::Feed::image_url`.
- One release route exists: `ReleaseKey::from_index_feed_guid` and `open_release_in_frame` in `src/app/search_dispatch.rs` (ADR 0084 Decision 4). The resolver opens the Library source when the Library holds the release.

## Design

### The new facts

- Decode `album_names_rank` and `remote_episode_count` on `api::PublisherRelationship`, as optional fields.
- Replace the stored contract with the `/openapi.json` of release 0.11.0, in the same change as the decode (ADR 0075 section 6). Name the file `musicindex-openapi-0.11.0.json`, and delete the 0.7.0 file. Change each test that reads the file name.
  Packet 016 also needs a newer contract. When packet 016 replaced it first, use that file.
- `PublisherPageAlbumFact` gets `named_rank: Option<i64>`, `track_count: Option<i64>` and `two_way_validated: Option<bool>`.
- An Index row takes them from `album_names_rank`, `remote_episode_count` and `two_way_validated`.
- A Library row takes them from the Library: the rank of the feed in the named feeds of the album (packet 003), and the track count of the feed.
- Stop and report when the decoded values do not agree with the release record. Examples: a DETOX row with a rank other than 1, or a rank on a row with a null `album_names_as`.

### The level

`PublisherPageVm::level()` gives one of three values. It reads only `album_names_as`, `role_agreement`, `role` and `named_rank`.

| Value | Rule | Header text |
|---|---|---|
| `Artist` | A row has `album_names_as` `publisher`, `role_agreement` `both`, and a role set that contains `artist` | "Artist" |
| `UnverifiedArtist` | No row has `role_agreement` `both`. A row with `two_way_validated` true has `album_names_as` `publisher` and `named_rank` 1 | "Unverified artist" |
| `None` | Each other page | No level fact |

- A role set contains `artist` when an item of `role_words` is `artist`, read without case. This is the same test as `ArtistIdentity`.
- A null `named_rank` is not rank 1. The page invents no rank.
- The header shows the level as a header fact with a named label. Color is never the only difference.

### The sections

`PublisherPageVm::sections()` gives the sections of ADR 0085 Decision 3, in this sequence. A section with no row is absent. Each release appears one time.

| Section | Rows |
|---|---|
| "Albums and EPs" | `album_names_as` `publisher`, with `role_agreement` `both` and a role set that contains `artist`, or with no `role_agreement`. Kind Album or EP |
| "Singles" | The same rows, with kind Single |
| "As <roles>" | `album_names_as` `publisher`, `role_agreement` `both`, and a role set without `artist`. One section for each different role word list |
| "Role not confirmed" | `album_names_as` `publisher`, and `role_agreement` `one_side` or `conflict` |
| "Listed by <feed title> only" | `album_names_as` null. An unknown `album_names_as` value also goes here |

- The kind comes from `ReleaseKind::resolve(None, track_count)`. The feeds state no kind, so the track count decides. A null track count counts as 0, and ADR 0084 Decision 5 then gives an Album.
- The "As <roles>" heading is "As " and `role_list_text` of `role_words`, for example "As label" or "As label and producer". The "As <roles>" sections sort by their heading text.
- A row in "Role not confirmed" exposes a detail line: `stated_sides_text(feed title, publisher_rel, music_rel)`. Example: "Sir Libre Records: label · the album: recordLabel".
- A row in another section exposes no role line.
- The "Listed by" heading uses the feed title. With no title, it is "Listed by this feed only".
- Inside a section, the rows keep the present sequence of the view model.
- Delete `AlbumNameGroup`, `RoleSubgroupCaption`, `AlbumRoleSubgroupVm`, `AlbumNameGroupVm` and the row role text of `AlbumRoleDisplay` that no section uses. Delete each test that tests only them. Keep `TitleDisplay`, `CoCreditedFeedVm` and the derived artist counts.

### One page shape

- The Library context and the Index context give the same sections (ADR 0084 Decision 4). Delete `library_scopes()`, `PublisherPageScopeVm`, "Library Albums" and "Other Albums".
- The Library context joins the Library rows and the other rows into one row list. A release that is in the two lists appears one time, with the Library facts.
- Each row in the Library keeps the mark "In Library".
- When MusicIndex fails, the page shows the report first. The Library rows stay in their sections.

### The words

Change each string below. Each new string is a named constant in its view model.

| Owner | Present text | New text |
|---|---|---|
| `publisher_page.rs` | "Not listed by the publisher" | "Not listed by this feed" |
| `publisher_page.rs` | "The app could not load the other albums of this publisher from MusicIndex. …" | "The app could not load the other releases of this feed from MusicIndex. The releases in the Library stay on this page." |
| `publisher_page.rs` | "No publisher page is open." | "No feed page is open." |
| `src/app.rs` | frame title "Publisher" for each feed page | The title of "The frame title" |
| `publisher_dispatch.rs` | "Failed to navigate to publisher: …" | "The app could not open the feed page: …" |
| `singles_group.rs` | "Publisher feed" | "Artist feed" |
| `singles_group.rs` | "Grouped by the publisher feed that each single names." | "Grouped by the artist feed of each single." |
| `track_detail.rs` | "Publisher" | "Feed owner" |
| `playlist_rss_check.rs` | "Publisher" | "podcast:publisher" |
| `playlist_rss_check.rs` | "Publisher feed {} at {url}" | "podcast:publisher feed {} at {url}" |
| `library.rs` | "Publisher must add payment routes." | "The feed owner must add payment routes." |
| `library.rs` | "Publisher must add payment routes for {title}" | "The feed owner must add payment routes for {title}" |
| `library.rs` | "n track(s) needs publisher routes" | "n track(s) needs payment routes from the feed owner" |

- The RSS check report names the element `podcast:publisher`. ADR 0085 Decision 1 permits an RSS element name.
- The feed page header uses `EntityKind::Artist` for the levels `Artist` and `UnverifiedArtist`, and `EntityKind::Feed` for `None`. Delete `EntityKind::Publisher` and its color token when no caller remains. Else record each caller.
- Code identifiers, routes, element ids and the CLI JSON keep their names.

### The frame title

[ADR 0085](../adr/0085-artists-by-confirmation.md) Decision 7, decided by the operator on 2026-10-18, gives the frame title of a feed page.

- `PublisherPageVm::frame_title()` gives it, from the facts of the page, in the sequence of the Decision 7 table:
  - "Artist" for the levels `Artist` and `UnverifiedArtist`.
  - With no artist level and one agreed role set or more: `capitalize_first(role_list_text(...))` of the agreed role words, as in the header roles. Example: "Label".
  - With no agreed role, no side that states `artist`, and a known rank that is not 1 on each named row: "Publisher".
  - Each other page: "Feed".
- A side states `artist` when an item of `role_words(publisher_rel)` or `role_words(music_rel)` is `artist`, read without case.
- A null `named_rank` on a named row gives "Feed". It never gives "Publisher".
- `content_list_frame_title` in `src/app.rs` uses this title for a loaded page. Before the facts load, it uses "Feed".
- The orchestrator has not confirmed which screen draws this title. The implementer records it in the result.
- "Publisher" is one named constant in `publisher_page.rs`. No other interface text uses the word.

### The Artists list

`LibraryViewModel::artists_list` gives one row for each artist identity of the Library tree.

- A feed row for each feed with the level `Artist` or `UnverifiedArtist`, by the rule of the level over the stored links of the Library. Its label is the feed title. It opens the feed page in the Library context.
- A name row for each artist text of the albums whose `ArtistIdentity` is `Name`. It opens the present Library artist page by name. Packet 017 replaces that page.
- An album whose artist feed has no row goes to the name row of its artist text.
- A feed with only agreed roles without `artist`, for example a label, has no row.
- No row shows a level word. The secondary text is "1 release" or "n releases".
- A feed row and a name row with the same label are two rows. The element id of a feed row comes from its feed GUID.
- Sort the rows by the lowercase label.

### The release rows and the header image

- Each release row exposes a typed open action, with an accessibility label such as "Open the release Face Pale". The target is `ReleaseKey::from_index_feed_guid(feed_guid)`. The one release route then selects the Library source or the Index source (ADR 0084 Decision 4).
- A row with no feed GUID has an unavailable action. The view model decides it, not the screen.
- `publisher_dispatch.rs` passes an open-release callback to the screen, beside the present open-feed callback of "Shares albums with". It opens the route in the current frame, as a release row of a list does.
- `PublisherPageFacts` gets `feed_image_url: Option<String>`. The Index path reads `api::Feed::image_url`. The Library path reads the stored image of the feed when the Library holds the feed, else the Index value.
- The header shows that image through the present image path of the artist header (`src/ui/shells/artist.rs`). With no image, it keeps the monogram of its entity kind.

### The guard

Add a situational guard to `tests/architecture_tests.rs`. It cites ADR 0085 Decision 1.

- It scans the string literals of `src/view_models/`, `src/ui/`, `src/views.rs`, `src/app.rs` and `src/app/`, outside `#[cfg(test)]` modules and comments.
- It fails on the word "publisher" in any case.
- It does not fail on the element name `podcast:publisher`, or on a snake_case or kebab-case identifier such as `music_to_publisher` or `publisher-page`.
- It permits the one frame title constant of ADR 0085 Decision 7, by its name.
- It skips a named list of files that use the word for the broadcast publisher service: the Show and startup view models and screens, and `src/app/show.rs`. Each entry of the list gives this reason.
- Its failure message names the file, the line and the fix:

```text
ADR 0085 §1 The Interface Does Not Name A Feed "Publisher": <file>:<line> names a feed "publisher".
Use "feed", "artist" or the RSS element name. Only the Decision 7 frame title says "Publisher". A service name goes in the broadcast file list of this guard.
```

## Required Changes

1. Change each string of "The words", and the entity kind of the feed page header.
2. Replace the stored contract, and decode the two facts of release 0.11.0.
3. Add the level, the sections and the one page shape to `PublisherPageVm`. Delete the groups, the subgroups and the scopes.
4. Change `src/ui/shells/publisher.rs` to compose the level fact and the sections. Use the present section header composite and its levels.
5. Change `artists_list` to give the rows of "The Artists list".
6. Add the guard. Delete each guard text of ADR 0082 Decisions 1 to 3 that this packet replaces.
7. Add the open action of each release row, its callback, and the header image.

## Mechanical Acceptance Criteria

Use the prefix `adr_0085_feed_page_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R85-8-01 | `level()` gives `Artist` for a row with `both` and `artist, producer`. A one-side `artist` row with rank 1 gives `UnverifiedArtist`. Only agreed `label` rows give `None` |
| R85-8-02 | `level()` gives `None` for a feed with rank 2 and no agreed row. A row with a null rank, and a rank 1 row that is not confirmed, also give `None` |
| R85-8-03 | The Sir Libre Records facts of 2026-10-18 give "As label", "Role not confirmed" and "Listed by Sir Libre Records only", in that sequence, with 3, 3 and 1 rows. No level fact |
| R85-8-04 | Agreed sets `label` and `label, producer` give two sections, "As label" and "As label and producer". `label` and `recordLabel` give two sections |
| R85-8-05 | A discography with two albums and one single gives "Albums and EPs" with 2 rows, then "Singles" with 1 row. An Index row with `remote_episode_count` 1 is a Single, and a null count gives an Album |
| R85-8-06 | A "Role not confirmed" row exposes the `stated_sides_text` detail. A row in each other section exposes no role line |
| R85-8-07 | An empty section is absent, and each release appears one time |
| R85-8-08 | The Library context and the Index context give the same sections for the same facts. A release in the Library list and the other list appears one time, with "In Library" |
| R85-8-09 | A MusicIndex failure keeps the Library rows in their sections and gives the report |
| R85-8-10 | `artists_list` gives a feed row for an `Artist` feed and for an `UnverifiedArtist` feed, and a name row for a name identity. An agreed `label` feed has no row. No row text contains a level word |
| R85-8-11 | Each new string of "The words" comes from its named constant |
| R85-8-12 | The decode reads `album_names_rank` and `remote_episode_count` from a recorded response, and the stored 0.11.0 contract has them |
| R85-8-13 | The guard of ADR 0085 Decision 1 passes, and a test string with the word "Publisher" in a scanned file fails it |
| R85-8-14 | `frame_title()` gives "Artist" for the HeyCitizen and DETOX facts, and "Label" for the Sir Libre Records facts. A feed with rank 2 rows and no stated role gives "Publisher". A null rank gives "Feed", and so does a rank 2 row with a one-side `artist` role |
| R85-8-15 | Each release row with a feed GUID exposes an available open action with the release key of that GUID and an accessibility label. A row with no feed GUID exposes an unavailable action |
| R85-8-16 | The Index facts carry `image_url` of the feed response as `feed_image_url`, and the view model exposes it for the header |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: the HeyCitizen page has the frame title "Artist". It shows "Artist" and its five roles in the header. Its releases show under "Albums and EPs" and "Singles". No text on the page says "publisher". The operator also finds a feed with only rank 2 links and no role, and checks that its frame title is "Publisher".
- V2: the Official DETOX Music page shows "Unverified artist". Its releases show under "Albums and EPs" and "Singles".
- V3: the Sir Libre Records page has the frame title "Label", and shows no level word. It shows "As label", "Role not confirmed" and "Listed by Sir Libre Records only". Each "Role not confirmed" row tells who stated which role.
  A release that names the feed, but that the feed does not list, shows "Not listed by this feed". This carries the "Not listed" part of §20 V4.
- V4: the Artists list shows HeyCitizen and DETOX by feed title, when the Library holds their releases, with no level word. Sir Libre Records is not in the list. A feed row opens the feed page.
- V5: with MusicIndex unreachable, a Library feed page shows the report first, and its Library releases stay in their sections. This carries the failure part of §20 V3 of the [overhaul plan list](../plans/design-and-cleanup-overhaul-plan.md#phase-4-publisher-and-artist-pages).
- V6: the DETOX page shows "Artists that name this feed: 1" with the name, and the unconfirmed fact with 0. A feed whose listed releases do not name it shows 0 and the unconfirmed names. The two labels read clearly. This carries §28 V1 to V3.
- V7: the track page labels the feed owner "Feed owner". The RSS check report names the field "podcast:publisher". The readiness list says "The feed owner must add payment routes."
- V8: normal and narrow widths, Light and Dark themes, and the larger type sizes show each element in its place, with no clipped text. The section headers are stronger than the row text. This carries §20 V5 and §28 V4.
- V9: a click on a release row of a feed page opens that release page, from the Library and from the Index. Back returns to the feed page. The HeyCitizen header shows the feed image, not "PB". Found by the operator on 2026-10-18.

ADR 0085 Decision 3 retires the stated and assumed role text of §20 V4. ADR 0084 Decision 4 retires the "two groups" part of §20 V3.

## Exclusions

- No name level page. Packet 017 owns it.
- No search entry. Packet 010 owns flow map finding 6.
- No scope control "Everything". Packet 016 owns it.
- No release page and no track page change apart from the string of the track page. ADR 0082 packet 003 owns them.
- No migration. Packet 003 adds migration 19.
- No request for each album of a feed page.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/view_models/publisher_page.rs`, `src/ui/shells/publisher.rs`, `src/app/publisher_dispatch.rs`, `src/app.rs`.
- `src/application/queries/feed.rs` and `src/application/queries/library.rs`: the page facts.
- `src/api.rs` (`PublisherRelationship`) and `tests/fixtures/`.
- `src/view_models/named_feeds.rs`, `src/view_models/role_words.rs` and `src/view_models/artist_identity.rs`, after packet 003.
- `src/view_models/library.rs` (`artists_list`), `src/view_models/singles_group.rs`, `src/view_models/track_detail.rs`, `src/view_models/playlist_rss_check.rs`.
- `src/ui/composites/tag_badge.rs`, `src/ui/composites/thumbnail.rs`, `src/ui/contrast.rs`.
- `tests/architecture_tests.rs`: the ADR 0077 and ADR 0082 feed page guards.

## Checks

```bash
cargo test --lib adr_0085_feed_page_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the commit. This packet adds no migration and no stored data.

## Operator Visual Check

Walk V1 to V9 in a desktop session. GPUI needs a real display, so an agent
cannot run this app. Do not delete `/tmp/v4vmm-governance.ie6k8TQf`. Its
cleanup is still unconfirmed. Color alone is not a valid difference.

### Step 1: build and start the app

1. A prior `cargo test` run can leave a GPUI test-support binary at
   `target/debug/v4vmm`. Run `cargo build --bin v4vmm` again before you
   start the app.
2. Start `target/debug/v4vmm`.
3. Open Settings. Check that the MusicIndex endpoint field holds a working
   endpoint. Note its value. V5 needs it for its cleanup.
4. Open the Music section.

### V1: HeyCitizen shows "Artist", five roles, and two discography sections

1. Open a HeyCitizen release, from Music > Releases or from a search for a
   HeyCitizen release. Click its artist name "HeyCitizen". Search does not
   find a feed page itself (packet 010 owns that entry).
2. Check the frame title of the page. It must read "Artist".
3. Check the header. It must show a "Confirmation" fact with the value
   "Artist". It must also show a "Roles" fact with five role words.
4. Check the releases. They must show under "Albums and EPs", then under
   "Singles". Each section shows only when it has a release.
5. Read every label and row on the page. No text must name "publisher", in
   any case.
6. Optional: when you know a feed that albums name only second, with no
   stated role on either side, open its page. Its frame title must read
   "Publisher". No recorded feed fits on 2026-10-18. The test R85-8-14
   proves the rule.
7. Wrong: the frame title of step 2, or of step 6 when you did it, does not
   match.
8. Wrong: a label or a row on the HeyCitizen page names "publisher".
9. Cleanup: use Back, or the breadcrumb, to return to the release.

### V2: Official DETOX Music shows "Unverified artist"

1. Open the page for the feed GUID
   `137aaa9c-75ff-4916-9f23-e02968b2d15e` (Official DETOX Music), from the
   Index or the Library.
2. Check the header. It must show the "Confirmation" fact with the value
   "Unverified artist".
3. Check the releases. They must show under "Albums and EPs", then under
   "Singles". Each section shows only when it has a release.
4. Wrong: the header shows "Artist", or shows no "Confirmation" fact.
5. Wrong: the releases show under another heading.
6. Cleanup: use Back, or the breadcrumb, to return to the page you opened
   in step 1.

### V3: Sir Libre Records shows "Label", its three sections, and the "Not listed" mark

1. Open the Sir Libre Records page (feed GUID
   `34bf795f-f551-4db1-8877-ad19aa5f89c8`).
2. Check the frame title of the page. It must read "Label".
3. Check the header. It must show no "Confirmation" fact.
4. Check the sections, in this order: "As label", "Role not confirmed",
   then "Listed by Sir Libre Records only". On 2026-10-18 they held 3, 3
   and 1 rows.
5. Open each row of "Role not confirmed". Check that each one shows a
   detail line. The line must name which feed stated which role, for
   example "Sir Libre Records: label · the album: recordLabel".
6. Find a release that names Sir Libre Records, in "Role not confirmed" or
   "As label". Check that Sir Libre Records does not list that release
   back. Check that row carries the mark "Not listed by this feed".
7. Wrong: the frame title is not "Label".
8. Wrong: the header shows a "Confirmation" fact.
9. Wrong: a "Role not confirmed" row shows no detail line.
10. Wrong: the mark of step 6 reads other than "Not listed by this feed".
11. Cleanup: use Back, or the breadcrumb, to return to the page you opened
    in step 1.

### V4: the Artists list shows feed rows with no level word

1. In Music, open the Artists destination.
2. Find the rows for HeyCitizen and for Official DETOX Music, if the
   Library holds their releases. Check each row shows the feed title,
   with no level word ("Artist", "Unverified artist" or "Confirmation")
   in its text.
3. Check that Sir Libre Records has no row in this list.
4. Click the HeyCitizen row.
5. Check that the app opens the HeyCitizen feed page.
6. Wrong: a row shows a level word.
7. Wrong: Sir Libre Records has a row.
8. Wrong: the click opens the wrong page, or opens no page.
9. Cleanup: use Back, or the breadcrumb, to return to the Artists
   destination.

### V5: a MusicIndex failure keeps the Library rows, with the report

1. Open Settings. Change the MusicIndex endpoint field to an address that
   gives no answer, for example `http://127.0.0.1:1`. Save Settings.
2. Open a feed page of a feed whose releases the Library holds.
3. Check the page shows this report, before its sections: "The app could
   not load the other releases of this feed from MusicIndex. The releases
   in the Library stay on this page."
4. Check the Library releases still show in their sections, below the
   report.
5. Wrong: the page shows no report.
6. Wrong: the Library releases do not show.
7. Cleanup: open Settings. Set the MusicIndex endpoint field back to the
   value from step 3 of the setup. Save Settings.

### V6: the artist-count facts read clearly

1. Open the Official DETOX Music page (V2).
2. Check the header shows a fact. Its label must read "Artists that name
   this feed". Its value must read "1 — Derived from album credits", with
   the artist's name in parentheses.
3. Check the header shows a second fact. Its label must read "Artists
   this feed lists without a link back". Its value must read "0 —
   Derived from album credits".
4. Check that the two labels read clearly. Check that each one is easy to
   tell apart from the other.
5. Find a feed whose listed releases do not name it back. Open its page.
   Check its first fact reads "0 — Derived from album credits", and its
   second fact names the unconfirmed artists.
6. Wrong: either label is unclear, or the two facts are hard to tell
   apart.
7. Cleanup: use Back, or the breadcrumb, to return to the page you opened
   in step 1.

### V7: "Feed owner", "podcast:publisher", and the payment-routes message

1. Open a track whose feed states an `itunes:owner` name.
2. Check the track page labels that row "Feed owner", not "Publisher".
3. Open a playlist whose RSS check report shows a difference on the
   `podcast:publisher` remote item. Check the report names the field
   "podcast:publisher". When no report has such a difference, skip this
   step.
4. Open a readiness row for a track with no upstream payment routes.
   Check it reads "The feed owner must add payment routes."
5. Wrong: any of the three labels of this check still says "Publisher"
   alone.
6. Cleanup: none.

### V8: normal and narrow widths, Light and Dark themes, and larger type

1. Open the HeyCitizen page (V1) and the Sir Libre Records page (V3).
2. Resize the window to its normal width, then to a narrow width.
3. Repeat step 2 in Light theme, then in Dark theme (Settings >
   Appearance).
4. Repeat step 2 at a larger system type size.
5. At each width, theme and type size, check the "Confirmation" fact, the
   "Roles" fact, each section heading and each row. Each one must stay in
   its own place, with no clipped text.
6. Check that each section heading reads stronger, bolder or larger, than
   its row text.
7. Wrong: a label or a heading is cut off mid-word.
8. Wrong: two elements overlap.
9. Wrong: a heading does not read stronger than its rows.
10. Cleanup: restore the window to its earlier size and theme.

### V9: a release row opens its release, and the header shows the feed image

1. Open a feed page from the Library context, for example from the
   Artists list of Music.
2. Open a feed page from the Index context, for example from a Discover
   search.
3. On each page, click a release row under "Albums and EPs" or under
   "Singles".
4. Check that the app opens that release's page.
5. Use Back, or the breadcrumb, to return.
6. Check that the app returns to the feed page you opened.
7. Open the HeyCitizen page. Check that its header shows the feed's own
   image, not a two-letter monogram.
8. Wrong: a release row click opens the wrong page, or opens no page.
9. Wrong: Back does not return to the feed page.
10. Wrong: the header still shows a monogram.
11. Cleanup: none.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0084-task-008-feed-page-and-artists.md`
- ADR 0085, ADR 0084 Decisions 4 to 7, and ADR 0082 Decisions 4 and 5
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the words, the new facts, the level, the sections, one page shape, the Artists list and the guard.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- The view model gives each label, section, text and action availability. The screen only composes. Use the scaled tokens of ADR 0039 and existing shared composites.
- Tests use recorded JSON. No test sends a request.
- Never run `git checkout`, `git restore`, `git stash`, `git reset` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The release page and the track page, apart from the "Feed owner" label.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R85-8-01 to R85-8-16 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 to V9.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- ADR 0082 packet 003 is not complete.
- The decoded values do not agree with the release record of 0.11.0.
- The Library rows and the other rows cannot join without a new query.
- A change needs a file in "Do not touch".

## Implementation Result - 2026-10-18

### Files Changed

New file:

- `tests/fixtures/musicindex-openapi-0.11.0.json` — the stored MusicIndex contract, read
  from the live `/openapi.json` of `https://api.musicindex.org` on 2026-10-18
  (`info.version` `0.11.0`). Deleted `tests/fixtures/musicindex-openapi-0.7.0.json`.

Changed files:

- `src/api.rs` — `PublisherRelationship` decodes `album_names_rank` and
  `remote_episode_count`. New test module `adr_0085_feed_page` proves the decode from a
  recorded DETOX row.
- `src/view_models/publisher_page.rs` — rewritten. `PublisherPageVm` gives `level()`,
  `frame_title()`, `header_facts()` (with the new "Confirmation" fact) and `sections()`
  in place of the former groups, subgroups and scopes. `PublisherPageAlbumFact` carries
  `named_rank`, `track_count` and `two_way_validated`. `PublisherPageFacts` carries
  `feed_image_url`. `PublisherPageAlbumVm` carries an `open: Option<ReleaseRowOpenAction>`.
  Deleted `AlbumNameGroup`, `RoleSubgroupCaption`, `AlbumRoleSubgroupVm`,
  `AlbumNameGroupVm`, `PublisherPageScopeVm`, `library_scopes()` and the row role text of
  `AlbumRoleDisplay`.
- `src/ui/shells/publisher.rs` — rewritten. `render_sections` and `render_section`
  compose the new sections, with the present `SectionHeader` composite. `render_header`
  shows the feed image and picks `EntityKind::Artist` or `EntityKind::Feed` from
  `vm.level()`. `render_album_row` wires the release row's open action and its role
  line.
- `src/app/publisher_dispatch.rs` — `PublisherPageState::Loaded` carries
  `facts: Box<PublisherPageFacts>` (clippy's `large_enum_variant` lint, once the struct
  grew). Builds the header image and the open-release callback, and passes them to
  `render_publisher_page`. Added `publisher_page_frame_title()`. Changed the failure
  text to "The app could not open the feed page: …".
- `src/app.rs` — `content_list_frame_title` gives "Feed" for a `PublisherDetail` frame
  before its facts load. `render_workspace_content` sets the loaded title from
  `publisher_page_frame_title()`, the same pattern it already uses for the SinglesGroup
  title.
- `src/app/search_dispatch.rs` — `open_release_in_frame` is `pub(super)`, so
  `publisher_dispatch.rs` can call the one release route.
- `src/application/queries/feed.rs` — the Index query reads `album_names_rank`,
  `remote_episode_count` and the feed's own `image_url` into the new facts.
- `src/application/queries/library.rs` — the Library query reads the named-feed rank
  and the track count from the Library. It reads the feed's own stored image, and
  falls back to the remote value.
- `src/db/publisher_relationships.rs` — `LocalPublisherAlbum` carries
  `two_way_validated`, read from the stored `feed_publisher_relationships` row.
- `src/view_models/role_words.rs` — added `role_contains_artist`, moved here from
  `named_feeds.rs` so `publisher_page.rs` and `library.rs` can both call it.
- `src/view_models/named_feeds.rs` — calls the moved `role_contains_artist`.
- `src/view_models/release_kind.rs` — added `ReleaseKind::is_single()`.
- `src/view_models/library.rs` — `artists_list()` gives a feed row for each confirmed or
  unverified artist feed, by the rule of "The Artists list". It gives a name row for
  each name identity. Three "publisher" strings changed to "the feed owner" text.
- `src/view_models/singles_group.rs`, `src/view_models/track_detail.rs`,
  `src/view_models/playlist_rss_check.rs` — the word changes of "The words".
- `src/ui/composites/tag_badge.rs`, `src/ui/composites/thumbnail.rs`,
  `src/ui/contrast.rs`, `src/ui/tokens.rs`, `src/ui/theme_profiles.rs` — deleted
  `EntityKind::Publisher` and `SemanticColor::EntityPublisher`, with no caller left.
- `src/ui/shells/library/destination_lists.rs`, `src/library/app_impl.rs` — route a
  `DestinationListRowTarget::Publisher` row to `EntityKind::Artist` and to the feed page.
- `tests/architecture_tests.rs` — the stored-contract filename constant now names
  `0.11.0`. Added the ADR 0085 Decision 1 guard
  (`adr_0085_feed_page_guard_no_interface_text_names_a_feed_publisher`) and its two
  companion tests. Rewrote the ADR 0077 screen-labels guard for the new view model API.

### Behavior Changed

- A feed page header shows a "Confirmation" fact, "Artist" or "Unverified artist", by
  ADR 0085 Decision 2. A page with neither level shows no such fact.
- A feed page's releases show under five sections at most, in the Decision 3 order.
  Each section shows only when it holds a row.
- The frame title of a feed page is "Artist", the agreed role words, "Publisher", or
  "Feed", by the Decision 7 rule. `src/app.rs` draws it.
- A release row on a feed page opens that release. It was dead before this packet.
- A feed page header shows the feed's own image, when either side states one. It showed
  only a monogram before this packet.
- The Artists list gives one row for each confirmed or unverified artist feed, by feed
  title, and one row for each name identity. Neither row names its level.
- "Publisher" text is gone from the track page's feed-owner label, the singles group
  note, the RSS check report, and the readiness list. It is gone from the feed page
  too, apart from the one named "Publisher" frame title.
- `EntityKind::Publisher` and its color token are gone.
- The stored MusicIndex contract is release 0.11.0. The decode reads its two new
  fields.

### Checks Run

| Check | Result |
|---|---|
| `cargo test --lib adr_0085_feed_page_` | Green. 17 passed. |
| `cargo test` | Green. 1942 lib tests, 320 architecture tests, 10 ignored doc tests. |
| `cargo test --test architecture_tests` | Green. 320 passed. |
| `cargo fmt -- --check` | Green. |
| `cargo clippy -- -D warnings` | Green. |
| `cargo check --all-targets` | Green, no warning. |
| `cargo build --bin v4vmm` | Green. |

### Deviations From The Task

- I added `ReleaseKind::is_single()` to `release_kind.rs`. The packet did not name it.
  `sections()` first wrote the literal text `ReleaseKind::Single`. The existing ADR
  0084 guard R15-09 forbids that text outside `release_kind.rs` and `singles_group.rs`,
  "so two groupings cannot disagree." The new method is the fix the guard asks for.
- I boxed `PublisherPageState::Loaded.facts` as `Box<PublisherPageFacts>`. The struct
  grew past clippy's `large_enum_variant` threshold once it carried the new per-album
  fields and `feed_image_url`.
- I added `two_way_validated` to `LocalPublisherAlbum`. The Design section named this
  field only for the Index row. The Library row needs it too, so `level()` and
  `frame_title()` give the same result from either context, as ADR 0084 Decision 4
  asks.
- I changed `open_release_in_frame` in `search_dispatch.rs` from private to
  `pub(super)`. This lets `publisher_dispatch.rs`, a sibling module, call the one
  release route for a feed page's release rows.
- The packet left open which screen draws the frame title. `src/app.rs` draws it, in
  `render_workspace_content`, the same way it already draws the SinglesGroup title.
  `PublisherPageVm::frame_title()` gives the text. The screen composes no text of its
  own.
- I wrote the Operator Visual Check steps in the numbered, "Wrong:" and "Cleanup:"
  format. The archived ADR 0082 packet 003 already uses that same format.

### Unresolved Concerns

- V1 asks the operator to find a feed with only rank 2 links and no stated role, to
  check its frame title reads "Publisher". The recorded facts name no such feed by
  GUID. The operator must find one, as the packet's own acceptance text asks.
- The accessibility label and the exact layout of the role line, the "Not listed by
  this feed" mark, and the open-release action are my own choice. The packet named the
  label, the value and the detail text, not the accessibility string or the layout. V3,
  V8 and V9 should confirm this choice reads well.
- I did not update `docs/pending-human-checks.md` or
  `docs/plans/broadcast-chain-delivery-order.md`. My task instructions name only this
  packet file as an allowed document edit. The orchestrator should add this packet's
  open gate to both files.
- `AGENTS.md` still names `tests/fixtures/musicindex-openapi-0.7.0.json` as the stored
  contract. My task instructions name only this packet file as an allowed document
  edit. The orchestrator should change that line to `musicindex-openapi-0.11.0.json`.

## Orchestrator Review - 2026-10-18

Pass. The code needs no fix. The orchestrator corrected the visual check and the records.

- `cargo fmt -- --check`, `cargo clippy -- -D warnings` and `cargo check --all-targets` are Green. `cargo test` gives 1942 library tests and 320 guards, all passing.
- `level()`, `frame_title()` and `sections()` follow ADR 0085 Decisions 2, 3 and 7. The guard of Decision 1 skips only the named broadcast service files and the one frame title constant.
- V1 opened the feed page from search, but search does not find a feed page (packet 010). V1 and V2 required "Singles" for feeds that can have no single. V1 required a "Publisher" feed that no recorded fact names, so that step is optional now. V7 step 3 named the owner field, not the `podcast:publisher` remote item.
- The orchestrator changed the stored contract fact of `AGENTS.md` to `musicindex-openapi-0.11.0.json`.

Recorded concern, not a defect: `artists_list` keys each row on the artist identity of each album (ADR 0085 Decision 6). In one rare case this differs from Decision 2. Example: a feed is the confirmed first feed of one album. It also has an agreed role without `artist` on a second album. It then gets a feed row. Decision 2 gives it no level.

## Operator Correction - 2026-10-18

The operator walked part of the check and found two defects. The orchestrator fixed each one. V4 and V9 stay open for a new walk.

- A release row of the DETOX page opened "Through The Looking Glass" with "GET failed with HTTP 404". The DETOX feed lists each of its 30 albums under a GUID that MusicIndex does not hold. Stophammer resolved each link by its feed URL (`publisher_link_resolution` `feed_url`) and gives the album in `music_feed_guid`.
  `publisher_page_facts_from_feed` in `src/application/queries/feed.rs` now takes `music_feed_guid` first, then `remote_feed_guid`. The "In Library" mark uses the same GUID. The test `adr_0085_feed_page_row_uses_the_resolved_album_guid` holds the recorded DETOX values.
- The Artists list showed two rows "Haleen" with the same text. Three releases name the Haleen feed `64569161-a8f7-52e5-8aae-9e0385043149`, and the other releases name no feed. The two identities are correct (ADR 0084 Decision 7).
  The operator decided on 2026-10-18 that only a name row carries the mark "by name", for example "3 releases · by name". It names the source of the group, not a level. The R85-8-10 and D5-04 tests assert it.

