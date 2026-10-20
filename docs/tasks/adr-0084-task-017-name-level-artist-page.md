# ADR 0084 Task 017: The Name Level Of The Artist Page

Status: Ready - 2026-10-20. The orchestrator wrote this packet on 2026-10-20. Decisions D1, D3, D5 and D7 remove or change something that the operator sees today. The orchestrator flags them for the operator before the implementation starts.

## Goal

The name level of the artist page replaces two old pages: the Library artist page by name and the Index name match page.
The new page shows "Releases that name <name>". It says that a name is not an identity, and it never merges with a feed page.

The page groups its releases by kind: "Albums and EPs", then "Singles". It shows the Library releases of the name and the MusicIndex releases of the name, each release one time. A release in the Library shows the word "In Library".
The Artists name row and the page count one set of releases with one function. The "Haleen · by name" row thus gives the same count as its page.

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md) Decision 7: the name level, the label "Releases that name <name>", the note that a name is not an identity, and no merge with a feed page. It replaces the Library artist page by name and the Index name match page.
- ADR 0084 Decision 4: one page for each item, and the Library is a state. Decision 6: "Albums and EPs" before "Singles". Decision 5: the kind of a release.
- ADR 0084 Decision 9: one navigation stack, and a breadcrumb that holds a page one time at most. Decision 10: the selection model of each list of releases.
- ADR 0084 Decision 12: the name level gives no image. Its art is the logo mosaic, or the entity color.
- [ADR 0085](../adr/0085-artists-by-confirmation.md) Decision 2: the level "Name only" is the name level. Decision 1: no interface text names a feed "publisher". Decision 6: the rule that selects the name level for a Library release.
- [ADR 0075](../adr/0075-metadata-contract.md) section 6: the stored contract changes in the same change as its decode.
- [ADR 0016](../adr/README.md): no schema change in this packet.
- The phase plan row of packet 017 and its note: "The 'Haleen · by name' row says 3 releases, and its page shows 6". The operator decided on 2026-10-18 that this defect waits for packet 017.
- Stophammer release 0.12.0 (Stophammer ADR 0071 section 4): `artist_key` on `GET /v1/feeds/by-name` gives the releases that the `release_count` of a `name` row counts.
- Stophammer ADR 0070 section 1: one sort key function, the normalization of Stophammer ADR 0049 section 7.

## Dependency On Packet 016

This packet does not wait for packet 016. The name level page is a page, not a list. ADR 0084 Decision 8 gives the scope control to the Releases list and the Artists list only.
The feed page of packet 008 already shows its MusicIndex releases with no scope control. The name level page does the same (decision D2).

Each of the two packets can come first. These rules apply to the one that comes second:

- The stored contract: the first packet replaces `musicindex-openapi-0.11.0.json` with the 0.12.0 contract. The second packet uses that file.
- The sort key: this packet adds `artist_name_key` in `src/view_models/artist_identity.rs`. Packet 016 uses it to merge the Library rows and the MusicIndex rows of the Artists list.
- The count: in the scope "Everything", packet 016 counts a name row as the "Releases" fact of this page counts it. That is the union of the Library set and the MusicIndex set, by feed GUID (decision D1).

## Recorded Facts - 2026-10-20

The code facts come from the working tree of 2026-10-20, after commit `d41d381`. Find each call site again before you change it.

### The Artists name row and its count

- `LibraryViewModel::artists_list` in `src/view_models/library.rs` groups each `(ArtistNode, AlbumNode)` pair of the tree by `album.artist_identity(&artist.name)`. That calls `ArtistIdentity::resolve` in `src/view_models/artist_identity.rs`.
- `artist_list_row` in `src/view_models/library.rs` gives the row. For `ArtistIdentity::Name(name)`, the secondary text is "n release(s)" and `ARTIST_ROW_SEPARATOR` and `ARTIST_NAME_ROW_MARK` ("by name"). The count is `members.len()`, the albums that resolve to that name identity.
- The row label is the identity text. The row target is `DestinationListRowTarget::Artist(name)`. Its art is `artist_row_art(None, members)`, a mosaic with no provided image.
- `ArtistIdentity::Name(String)` keeps the artist text as it is. Test `adr_0084_singles_group_r15_01_name_level_keeps_case` asserts that "Haleen" and "haleen" are two identities.
- The artist text of an `ArtistNode` comes from `build_tree` in `src/application/queries/library.rs`. It is `LibraryTrackRowVm::display_artist`: `album_artist_name`, else `artist_name`, else the fallback text "Unknown Artist".

### The old Library artist page by name

- `LibraryApp::open_destination_row` in `src/library/app_impl.rs` maps `DestinationListRowTarget::Artist(name)` to `FrameNavigationEntry::ArtistDetail(name)`.
- `LibraryApp::hydrate_detail_from_nav` calls `LibraryApp::select_artist(name)` for that entry.
- `select_artist` takes each album of the first `ArtistNode` whose `name` equals the text. It does not apply `ArtistIdentity`. Thus it also takes the albums that resolve to a feed identity. This is the cause of the "3 vs 6" defect.
- `select_artist` sets `LibraryDetail::Artist(Box<LibraryArtistDetail { name, view, tracks }>)` (`src/library.rs`). `view` comes from `ArtistView::from_local_rows` in `src/views.rs`.
- `render_library_detail` in `src/ui/shells/library/detail.rs` renders that state with `render_library_feed_list` in `src/ui/shells/library/feed_list.rs`. That screen uses `LibraryArtistDetailVm` (`src/view_models/library.rs`) and `render_artist_detail_shell` (`src/ui/shells/artist.rs`).
- `LibraryArtistDetailVm::page` gives the subtitle "Grouped by name" and the facts "Albums", "Tracks" and "Downloaded". A row click calls `LibraryApp::select_album_by_name`.
- `LibraryApp::render_library_detail_pane` in `src/library/app_impl.rs` loads the images of `LibraryArtistDetailVm::thumbnail_urls`.
- The Library album page link `AlbumNameLinkTarget::Artist(name)` emits `LibraryAppEvent::OpenArtistPage` (`LibraryApp::open_album_name_link`). `TopApp` handles it with `open_library_artist_page` and `open_library_artist_in_frame` in `src/app/search_dispatch.rs`. These push `FrameNavigationEntry::ArtistDetail`.
- `ReleasePageVm::name_links` in `src/view_models/entity_detail.rs` gives `AlbumNameLinkTarget::Artist(artist)` when `select_artist_feed` gives `ArtistFeed::None`.

### The old Index name match page

- `FrameNavigationEntry::IndexNameMatches(String)` in `src/view_models/workspace/nav.rs`. Its `display_label` is the quoted name.
- `TopApp::open_name_match_page` and `restore_name_match_page_for_nav` in `src/app/name_match_dispatch.rs` run `FetchNameMatchTracks` (`src/application/queries/search.rs`). It sends `GET /v1/tracks?artist=<name>` through the request profile `INDEX_NAME_MATCH_TRACKS` (`src/application/request_profiles.rs`, path shape `TracksByArtistName`).
- `NameMatchPageVm` (`src/view_models/name_match_page.rs`) gives the title `Tracks matching "<name>"`, one track row for each track, and the empty text and the "more tracks" text. `render_name_match_page` (`src/ui/shells/name_match_page.rs`) draws it.
- `TopApp::render_workspace_slots` in `src/app.rs` mounts the page for `Some(FrameNavigationEntry::IndexNameMatches(_))` with `render_name_match_page_content`.
- Three entry points open it:
  - The Index search row of the Artists tab: `handle_index_artist_result_selected`, from the id prefix `index-artist:` in `src/app/search_dispatch.rs`.
  - The artist name link of an Index release page: `open_index_album_name_link` in `src/app/search_dispatch.rs`.
  - Back and Forward, through `restore_name_match_page_for_nav`.
- `IndexArtistCandidate::into_display` in `src/application/queries/search.rs` gives the search row: the label `Tracks matching "<name>"` and the secondary text "n feeds · m tracks". The counts come from the search hits. `merge_index_artist_candidates` merges the candidates by `to_lowercase()`.
- The Library search row: `LocalArtistResult::into_display` in `src/view_models/search_results/results.rs` gives the id `library-artist:<name>`, the label `<name>` and "n albums - m tracks". `TopApp` opens it with `open_library_artist_in_frame`.

### Navigation, titles and breadcrumbs

- `FrameNavigationEntry::ArtistDetail(String)` has the display label `<name>`, and `IndexNameMatches(String)` has `"<name>"` (`display_label` in `src/view_models/workspace/nav.rs`).
- `breadcrumb_entry_id` in `src/view_models/workspace/breadcrumb.rs` gives `artist-<slug>` and `index-name-matches-<slug>`.
- `TopApp::content_list_frame_title` in `src/app.rs` gives "Artist" for `ArtistDetail(_)` and "Tracks matching" for `IndexNameMatches(_)`.
- `DetailListKind::for_entry` in `src/view_models/detail_selection.rs` gives no selection for either entry. `TopApp::detail_page_rows` in `src/app/detail_selection.rs` gives the feed page rows through `feed_page_release_rows`.

### The feed page that the name level reuses

- `PublisherPageVm::sections` in `src/view_models/publisher_page.rs` gives `PublisherPageSectionVm` values with `PublisherPageAlbumVm` rows. The constants are `ALBUMS_AND_EPS_HEADING`, `SINGLES_HEADING`, `IN_LIBRARY_LABEL` and `LEVEL_LABEL` ("Confirmation").
- `render_section` and `render_album_row` in `src/ui/shells/publisher.rs` draw the sections and the rows. They are private functions of that screen.
- `PublisherPageVm::other_albums_status` gives the MusicIndex failure report. The Library rows stay in their sections.

### Dead code today

- `render_artist_view` in `src/ui/shells/artist.rs` and `ArtistVm` in `src/view_models/artist.rs` have no caller outside those two files.
- `Source::fetch_artist` in `src/sources.rs` has a production implementation and one test caller (`src/sources.rs` tests). No composition root calls it. `ApiSource` has no caller outside `src/sources.rs`.

### The guards that pin the old pages

Each name is a function or a constant in `tests/architecture_tests.rs`.

- `SCREEN_FILES` names `src/app/name_match_dispatch.rs`. `LIBRARY_SCREEN_SURFACE_FILES` names `src/ui/shells/library/feed_list.rs`.
- `adr_0046_forward_restores_pages_through_the_shared_back_restore_path` requires `restore_name_match_page_for_nav`.
- `adr_0066_search_failure_report_stays_readable_and_vm_owned` has a comment on the name match page. Its count of `.on_failure_action(` stays 1.
- `adr_0049_inspector_source_ownership_is_guarded` requires `FrameNavigationEntry::IndexNameMatches(`.
- `adr_0024_loading_shape_readiness_gate_is_locked` requires one guard `adr_0077_name_matches_index_row_opens_tracks_matching_not_artist_page`.
- `adr_0077_name_matches_index_row_opens_tracks_matching_not_artist_page` pins the name match route, its frame title and its mount.
- `entity_detail_pages_render_through_shell_helper_and_page_vm` names "Library artist detail" (`feed_list.rs`), "Discover artist detail" (`artist.rs`) and `ArtistDetailPageVm`.
- `shared_top_level_ui_shells_do_not_import_screen_modules` names `src/ui/shells/artist.rs`.
- `nav_top_drives_content_list_body_switch` requires `FrameNavigationEntry::ArtistDetail(_)` and `FrameNavigationEntry::IndexNameMatches(_)`.
- `adr_0075_library_observation_callers_and_consumers_are_guarded` uses `pub(crate) fn select_artist(` as the end marker of a source range.
- `adr_0084_scrollbars_r14_01_music_columns_use_the_shared_helper` names `feed_list.rs` and `name_match_page.rs`.
- These unit tests pin the old pages:
  - the `adr_0077_name_matches_` tests in `src/application/queries/search.rs`
  - the tests in `src/view_models/name_match_page.rs` and `src/app/name_match_dispatch.rs`
  - the `LibraryArtistDetailVm` tests in `src/view_models/library.rs`
  - the `ArtistDetail` and `IndexNameMatches` cases in `src/view_models/workspace/tests.rs`
  - the `INDEX_NAME_MATCH_TRACKS` test in `src/api.rs`

### MusicIndex, read on 2026-10-20 at 23:20 UTC

- `GET /node/info` gave `git_revision` `056199b`. `/openapi.json` gave `info.version` `0.12.0`.
- The stored contract is `tests/fixtures/musicindex-openapi-0.11.0.json`. Its `/v1/feeds/by-name` has no `artist_key` and no `include`. The client has no method for `/v1/feeds/by-name` or `/v1/artist-names`.
- The 0.12.0 contract gives `artist_key` on `/v1/feeds/by-name`: "Keeps only the music releases that the `name` row of `GET /v1/artist-names` with this `key` counts in `release_count` (ADR 0071 section 4). Needs `medium=music`." `limit` has the maximum 200.
- `GET /v1/feeds/by-name?artist_key=sir%20seat%20sitter` gave 4 rows, `has_more` false: "Gay and Long", "Gone" and "That Duck" with `episode_count` 1, and "Waitin' on Alex" with `episode_count` 17. Each row gave `release_artist` "Sir Seat Sitter" and a null `publisher_feed_guid`.
- `GET /v1/artist-names?from=haleen&limit=6` gave a `name` row "Haleen" with `key` `haleen` and `release_count` 1. It also gave two `publisher` rows "Haleen", with the feed GUIDs `64569161-a8f7-52e5-8aae-9e0385043149` and `fe80a515-2826-4a46-aa37-b27f38d3c30f`.
- `GET /v1/feeds/by-name?artist_key=haleen&include=named_feeds` gave 1 row, "Haleen - Music", `episode_count` 1, with an empty `named_feeds`.
- An `artist_key` with no `name` row gave an empty `data` and `has_more` false. `medium=all` with `artist_key` gave HTTP 400.
- Stophammer `sort_key` (`src/model.rs` in the Stophammer checkout at `1e42509`) is `text.split_whitespace().collect::<Vec<&str>>().join(" ").to_lowercase()`. Its tests give "the  best\tsong " to "the best song", "Élan" to "élan", and "The Beatles" to "the beatles".

**The stored contract (orchestrator decision of 2026-10-20).** Packets 010, 016 and 017 each need the 0.12.0 contract. The first of the three that an implementer starts replaces `tests/fixtures/musicindex-openapi-0.11.0.json` with `musicindex-openapi-0.12.0.json`, with the guard constant and the `AGENTS.md` line. A later packet finds the 0.12.0 file and changes only its own decode.

## Design

### The name key

Add `artist_name_key(text: &str) -> String` to `src/view_models/artist_identity.rs`. It is the Stophammer `sort_key`: split on white space, join with one space, then Unicode lowercase.

- `ArtistIdentity::resolve` gives `ArtistIdentity::Name(artist_name_key(artist_text))`. Two texts that differ only in case or in white space are one identity (decision D3).
- Each label of a name identity comes from its members, not from the key. The label is the raw artist text that the most members state. With a tie, it is the first raw text in code point order. This is the rule of Stophammer ADR 0070 section 3.
- The singles group keeps its present label rule: `display_artist` of its newest single.
- `merge_index_artist_candidates` in `src/application/queries/search.rs` uses `artist_name_key` in place of `to_lowercase()`.

### One count function

Add `LibraryViewModel::name_level_members(&self, key: &str) -> Vec<NameLevelLibraryRelease>` in `src/view_models/library.rs`. It gives each album of the tree whose `artist_identity` is `ArtistIdentity::Name(key)`.

- `artists_list` and the name level page both use this one function. The row count and the "In Library" fact of the page are the length of its result.
- The Artists name row keeps its text form: "n release(s) · by name". Its label follows "The name key". Its target becomes `DestinationListRowTarget::ArtistName(ArtistNameKey)`.
- `NameLevelLibraryRelease` holds the facts that a row needs. These are the release key, the feed GUID, the title and the image URL.
  They also include the raw artist text, the kind (`AlbumNode::release_kind`), the newest track date and the state "in Library".
- The function reads the tree at each call. A download or a removal thus changes the page and the row in place, as the singles group page does.

### The navigation entry

Replace `FrameNavigationEntry::ArtistDetail(String)` and `FrameNavigationEntry::IndexNameMatches(String)` with one entry `FrameNavigationEntry::ArtistName(ArtistNameKey)`.

- `ArtistNameKey` (in `src/view_models/artist_identity.rs`) holds `key` and `label`. The label is the text that the operator clicked.
  `PartialEq`, `Eq`, `Hash`, `PartialOrd` and `Ord` use `key` only. Thus "Haleen" and "haleen" open one page, and the breadcrumb holds it one time.
- `display_label` gives `<label> · by name`, with the same separator and mark constants as the Artists row. Move these two constants to `src/view_models/artist_identity.rs` so that the row and the entry use one source.
- `breadcrumb_entry_id` gives `artist-name-<slug of key>`.
- `opens_detail_page` is true for the entry.
- `content_list_frame_title` gives the frame title "Artist name" (decision D5). It is the constant `NameLevelPageVm::FRAME_TITLE`.

### The page view model

Add `src/view_models/name_level_page.rs` with `NameLevelPageFacts` and `NameLevelPageVm`. The view model carries no renderer type.

`NameLevelPageFacts` holds:

- `key: ArtistNameKey`
- `library: Vec<NameLevelLibraryRelease>`, from `name_level_members` at each render
- `index: NameLevelIndexState`: `NotRequested`, `Loading`, `Loaded { releases, more_available }` or `Failed { detail }`
- `other_identity_feed_guids: BTreeSet<String>`: each feed GUID of a Library release whose identity is not this name

`NameLevelPageVm` gives these facts. Each text is a named constant.

| Method | Value |
|---|---|
| `title_text()` | `Releases that name <label>`. The label follows "The name key" over the Library and MusicIndex rows. With no row, it is the label of the entry |
| `title_text_for_name(name)` | The same text for a search row, before a page loads |
| `header_facts()` | "Confirmation": "Name only". "About this page": "A name is not an identity. These releases state this artist text and name no artist feed." "In Library": the count of `library`. "Releases": the count of distinct rows, only when `index` is `Loaded` |
| `sections()` | `Vec<PublisherPageSectionVm>`, as in "The sections" |
| `index_status()` | The loading text, the failure report, or the "more releases" text |
| `collection_art()` | `CollectionArtVm::resolve(None, covers, CollectionKind::Artist)`. The covers are the images of the newest four rows. The name level gives no provided image (ADR 0084 Decision 12) |
| `frame_title()` | "Artist name" |
| `empty_message()` | "No release in the Library or in MusicIndex names <label> as its artist and names no artist feed." It shows when each section is empty and `index` is `Loaded` |

- The level word is the ADR 0085 Decision 2 word "Name only", with the feed page label "Confirmation". This replaces the subtitle "Grouped by name" (decision D4).
- "In Library" and "Releases" are two facts, never one ratio. The page can show more rows than the Artists row count. Each extra row is a MusicIndex release with no state word, as on the feed page.
- The loading text is "Loading the releases of this name from MusicIndex." The failure report is "The app could not load the releases of this name from MusicIndex. The releases in the Library stay on this page." The technical detail comes after the report.
- The "more releases" text is "MusicIndex has more releases for this name. This page shows the first 1,000."

### The sections

`sections()` merges the two lists into rows, then puts each row in one section.

1. Each Library release gives one row, with `in_library` true and the word "In Library".
2. Each MusicIndex release gives one row, unless one of these is true:
   - Its feed GUID is the feed GUID of a Library row of this page. The Library row stays, one time.
   - Its feed GUID is in `other_identity_feed_guids`. The Library rule of ADR 0085 Decision 6 decides the identity of a Library release, so the row is absent.
3. The kind of a Library row is `AlbumNode::release_kind`. The kind of a MusicIndex row is `ReleaseKind::resolve(None, episode_count)`. A null count counts as 0, as on the feed page.
4. "Albums and EPs" holds the rows of kind Album or EP. "Singles" holds the rows of kind Single. "Albums and EPs" comes first. A section with no row is absent.
5. Inside a section, the rows go newest first. The date of a Library row is the newest `pub_date` of its tracks. The date of a MusicIndex row is `newest_item_at`. A row with no date comes last. Equal dates sort by the lowercase title.
6. Each row is a `PublisherPageAlbumVm` with `artist: None`, `not_listed: false` and `role_line: None`. Its `open` action has the release key and the label "Open the release <title>". A Library release with no feed GUID uses its Library release key.

### The MusicIndex request

- Add `api::Client::fetch_feeds_by_name_with_profile(artist_key, limit, cursor, profile)` in `src/api.rs`. It sends `GET /v1/feeds/by-name` with `artist_key`, `limit` and `cursor`. It decodes `RecentFeedsResponse`, because the rows have the fields of `/v1/feeds/recent`.
- Add the request profile `INDEX_NAME_LEVEL_RELEASES` with a new path shape `FeedsByName` and no include. Delete `INDEX_NAME_MATCH_TRACKS` and the path shape `TracksByArtistName` when no caller stays.
- Add the command `FetchNameLevelReleases` in `src/application/queries/search.rs`, in place of `FetchNameMatchTracks`. It sends `limit=200`. It follows `pagination.cursor` while `has_more` is true, for 5 requests at most. After 5 requests with `has_more` true, it sets `more_available`.
- The command gives `Vec<NameLevelIndexRelease>`: the feed GUID, the title, the image URL, `release_artist`, `episode_count` and `newest_item_at`. A row with no feed GUID is dropped.
- The page sends no request when the key is `artist_name_key("Unknown Artist")`, the fallback text of `display_artist`. The `index` state is then `NotRequested`, and the page shows no MusicIndex text.
- The response is untrusted input. The command makes no other request for each row.

### The dispatch and the screen

- Replace `src/app/name_match_dispatch.rs` with `src/app/name_level_dispatch.rs`. It holds `NameLevelPageState { key, index: NameLevelIndexState }` on `TopApp`, in place of `name_match_page`.
- `open_name_level_page(key, start, cx)` pushes the entry through the one navigation stack. `restore_name_level_page_for_nav(entry, cx)` starts the request for a different key, and skips it for the mounted key.
- A result for a key that is not the current key is dropped, as `name_match_page_result_is_current` does today.
- `render_name_level_page_content` reads `name_level_members` and the other-identity GUIDs from `LibraryApp` at each render. It builds `NameLevelPageVm` and calls the screen.
- Add `src/ui/shells/name_level_page.rs`. It composes the header (`DetailHeader` with `EntityKind::Artist`, the `XXl` collection art and the `Display` title size), the MusicIndex status, and the sections.
- Move `render_section` and `render_album_row` from `src/ui/shells/publisher.rs` to a new composite, `src/ui/composites/release_section_list.rs`. The feed page and the name level page call it. The feed page does not change.
- `LibraryApp::hydrate_detail_from_nav` clears the Library detail for `ArtistName`. `TopApp` renders the page, as it does the feed page.
- The detail column keeps its scrollbar through `music_column_tracked_scrollbar`.

### Each entry point

| Entry point | Today | After this packet |
|---|---|---|
| Artists name row | `ArtistDetail(name)`, old Library page | `ArtistName(key)` |
| Artist name link of a Library release page | `OpenArtistPage`, old Library page | `ArtistName(key)` in the current path |
| Artist name link of an Index release page | `IndexNameMatches(name)` | `ArtistName(key)` in the current path |
| Search row `index-artist:<name>` | `IndexNameMatches(name)` | `ArtistName(key)` |
| Search row `library-artist:<name>` | `ArtistDetail(name)`, old Library page | `ArtistName(key)` |
| Back and Forward | `select_artist`, `restore_name_match_page_for_nav` | `restore_name_level_page_for_nav` |

- Delete `LibraryAppEvent::OpenArtistPage`. `LibraryApp::open_album_name_link` emits `LibraryAppEvent::OpenPage` with the new entry.
- Each search artist row gets the label `NameLevelPageVm::title_text_for_name(name)` and an equal accessibility label. It shows no count (decision D7).

### Selection

- `DetailListKind::for_entry` gives `Releases` for `ArtistName(_)`.
- `TopApp::detail_page_rows` gives the rows of `NameLevelPageVm::sections` through `feed_page_release_rows`, as for the feed page.
- The "Select" button, the action bar and "Download n releases" thus work on the name level page, with no new selection code.

### The guard

Add one situational guard to `tests/architecture_tests.rs`: `adr_0084_name_level_r17_one_count_function`. It cites ADR 0084 Decision 7. It earns a test: the "3 vs 6" defect of 2026-10-18.

- The body of `artists_list` calls `name_level_members(`, or a function that it calls does.
- `src/app/name_level_dispatch.rs` calls `name_level_members(`.
- No file in `src/` contains `fn select_artist(`, `LibraryArtistDetailVm`, `FrameNavigationEntry::ArtistDetail`, `IndexNameMatches` or `FetchNameMatchTracks`.

Its failure message names the owner and the fix:

```text
ADR 0084 §7 One Artist Page With Two Levels: <file> counts or lists the name level apart from `name_level_members`.
The Artists name row and the name level page count one set. Call `LibraryViewModel::name_level_members` in both.
```

## Orchestrator Decisions

The orchestrator made these decisions on 2026-10-20. "Flag" marks a decision that removes or changes something that the operator sees today.

- **D1. One count function.** The Artists name row and the "In Library" fact of the page use `name_level_members`. The page lists only the releases whose Library identity is the name.
  - **Flag:** the Haleen name level page loses the releases that resolve to a Haleen artist feed. They stay on the Haleen feed page. The phase plan records 6 releases on the old page and 3 on the row.
- **D2. MusicIndex releases always show.** The page shows the MusicIndex releases of `artist_key` with no "Everything" scope, as the feed page does. Packet 016 is not a prerequisite. The page can thus show more rows than the row count. The header gives "In Library" and "Releases" as two facts.
- **D3. The name key is the Stophammer sort key.** The name identity uses `artist_name_key`: white space collapsed and Unicode lowercase. The label is the raw text that the most releases state.
  - **Flag:** two Artists rows or two singles groups that differ only in case or in white space become one. The test R15-01 asserted the old rule, and this packet replaces it.
  ADR 0084 Decision 7 says "artist text" and states no case rule. The orchestrator reads this decision as inside Decision 7. The orchestrator asks the operator to confirm it, or to request an amendment line in ADR 0084 Decision 7, before the implementation starts.
- **D4. The level word replaces "Grouped by name".** The header shows "Confirmation: Name only" (ADR 0085 Decision 2) and the note "A name is not an identity. These releases state this artist text and name no artist feed."
  - **Flag:** the text "Grouped by name" and the facts "Albums", "Tracks" and "Downloaded" go. The page shows releases, not tracks.
- **D5. Titles.** The page title is `Releases that name <label>`, with no quotation marks, as ADR 0084 Decision 7 writes it. The frame title is "Artist name". The breadcrumb label is `<label> · by name`, the mark of the Artists row.
  - **Flag:** the frame title "Artist" of the old Library page and "Tracks matching" of the old Index page go. The quoted breadcrumb `"<name>"` goes.
- **D6. The name match page goes, and search opens the name level.** Each search artist row, Library and MusicIndex, opens the name level page of its name. A name with no release on the name level shows the empty text of the page.
  - **Flag:** the list of tracks whose artist text matches a name goes. A track search keeps the tracks. A name whose releases all have an artist feed now opens an empty name level page. Packet 010 adds the search entry of a feed page (flow map finding 6).
- **D7. The search artist rows show no count.** Their label is the page title, `Releases that name <name>`, and their accessibility label is equal. A search hit count does not count the releases of the name level, so the row shows none.
  - **Flag:** the texts "n feeds · m tracks" and "n albums - m tracks" go from the Artists tab of search. The text `Tracks matching "<name>"` goes.
  A Library row and a MusicIndex row of one name can both show. Each opens the same page. Packet 010 owns the search results view and merges them.
- **D8. One section pair only.** The name level page has "Albums and EPs" and "Singles" only. A name level release names no artist feed, so no "As <roles>", "Role not confirmed" or "Listed by" section applies.
- **D9. The Library decides a Library release.** A MusicIndex row of a release that the Library holds under another identity is absent. Stophammer ADR 0070 section 4 and ADR 0085 Decision 6 differ for one case only. The phase plan open finding of 2026-10-18 records that case.
- **D10. At most 5 requests of 200 rows.** The page lists the first 1,000 MusicIndex releases and says when more exist. On 2026-10-20, the largest recorded name row had a `release_count` far below this limit. This packet adds no "Load more".
- **D11. Selection is on.** The page has the detail selection of packet 018 through the feed page rows. No new selection code.
- **D12. No dependency on packet 016.** See "Dependency On Packet 016".
- **D13. The dead artist code goes.** `src/ui/shells/artist.rs`, `src/view_models/artist.rs` and `src/view_models/artist_detail.rs` lose their last caller and go. `Source::fetch_artist` and `ArtistRef::LocalArtistName` go when no composition root reaches them. The rest of `src/sources.rs` stays, and the result records it as an open finding.

## Required Changes

1. Add `artist_name_key`, `ArtistNameKey` and the label rule. Change `ArtistIdentity::resolve`, the Artists row label and the search candidate merge to use them.
2. Add `name_level_members` and use it in `artists_list`.
3. Replace the stored contract with the 0.12.0 contract, unless packet 016 did it first. Name the file `musicindex-openapi-0.12.0.json`, delete the 0.11.0 file and change each test that reads the file name. Change the contract line of `AGENTS.md` in the same change.
4. Add the client method, the request profile and `FetchNameLevelReleases`. Delete `FetchNameMatchTracks`, `INDEX_NAME_MATCH_TRACKS` and `fetch_tracks_by_artist_with_profile` when no caller stays.
5. Add `FrameNavigationEntry::ArtistName`. Delete `ArtistDetail` and `IndexNameMatches`, with their labels, ids, frame titles and body switch arms.
6. Add `src/view_models/name_level_page.rs`, `src/app/name_level_dispatch.rs`, `src/ui/shells/name_level_page.rs` and `src/ui/composites/release_section_list.rs`.
7. Change each entry point of "Each entry point". Delete `LibraryAppEvent::OpenArtistPage`, `open_library_artist_page` and `open_library_artist_in_frame`.
8. Delete the replaced pages and their dead code: `LibraryDetail::Artist`, `LibraryArtistDetail`, `select_artist`, `select_album_by_name` when no caller stays, `LibraryArtistDetailVm`, `ArtistFeedSummaryVm`, `ArtistFeedSummaryDisplay`, `src/ui/shells/library/feed_list.rs`, `src/app/name_match_dispatch.rs`, `src/view_models/name_match_page.rs`, `src/ui/shells/name_match_page.rs`, and the files of D13.
9. Add the selection of "Selection".
10. Change each guard in "The guards that pin the old pages":
    - Delete `adr_0077_name_matches_index_row_opens_tracks_matching_not_artist_page` and its entry in `adr_0024_loading_shape_readiness_gate_is_locked`. ADR 0084 Decision 7 supersedes the rule of ADR 0077 packet 006 that it guards.
    - Replace `restore_name_match_page_for_nav`, `IndexNameMatches(` and `ArtistDetail(_)` with the new names in the other guards.
    - Remove the deleted files from `SCREEN_FILES`, `LIBRARY_SCREEN_SURFACE_FILES`, `entity_detail_pages_render_through_shell_helper_and_page_vm`, `shared_top_level_ui_shells_do_not_import_screen_modules` and `adr_0084_scrollbars_r14_01_music_columns_use_the_shared_helper`. Add the new files where the old ones were.
    - Move the end marker of `adr_0075_library_observation_callers_and_consumers_are_guarded` to the function that follows `hydrate_album_identity_on_view`.
    - Delete each unit test of the deleted code. Replace R15-01 with R17-02.
11. Add the guard of "The guard".
12. Change these documents in the same change:
    - The phase plan row of packet 017.
    - The status line of ADR 0085: the name level is built.
    - The §12, §21 and §23 items of the overhaul plan list, as "Visual Acceptance Criteria" gives.
    - `docs/pending-human-checks.md` and `docs/plans/broadcast-chain-delivery-order.md`: the open gate of this packet.
    - `docs/architecture/source-map.md`: one row for the name level page.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_name_level_` for unit tests beside the owning code. Each test uses recorded values. No test sends a request to `api.musicindex.org`. A query test uses a local recorded-response server, as the tests in `src/application/queries/search.rs` do.

| Case | Required proof |
|---|---|
| R17-01 | `artist_name_key` gives "the best song" for "the  best\tsong ", "élan" for "Élan", "jimmy v" for "  Jimmy   V " and "the beatles" for "The Beatles" |
| R17-02 | `ArtistIdentity::resolve(&[], "Haleen")` equals `ArtistIdentity::resolve(&[], " haleen ")`. A name identity and a feed identity with the same text stay two identities |
| R17-03 | The Haleen tree: one `ArtistNode` "Haleen" with 3 albums that name no feed and 3 albums with a confirmed artist feed. `name_level_members("haleen")` gives 3. The `artists_list` name row gives "3 releases · by name". `NameLevelPageVm` gives "In Library" "3". No row of the 3 feed albums is in `sections()` |
| R17-04 | For each name row of `artists_list` on a test tree with four names, the row count equals the "In Library" fact of its page |
| R17-05 | The label of a name identity is the raw text that the most members state. A tie gives the first raw text in code point order |
| R17-06 | `sections()` gives "Albums and EPs" before "Singles". A Library single and a MusicIndex row with `episode_count` 1 go in "Singles". A null count gives an Album. An empty section is absent. Rows go newest first, and a row with no date comes last |
| R17-07 | A MusicIndex row with the feed GUID of a Library row appears one time, with "In Library". A MusicIndex row whose feed GUID is in `other_identity_feed_guids` is absent. A MusicIndex-only row has `in_library` false |
| R17-08 | `title_text()` gives "Releases that name Haleen". `header_facts()` gives "Confirmation" "Name only", the note, and "In Library". "Releases" shows only for `Loaded`. `frame_title()` gives "Artist name". `display_label` of the entry gives "Haleen · by name" |
| R17-09 | `Loading` gives the loading text. `Failed` gives the report before the detail, and the Library rows stay in their sections. `Loaded` with `more_available` gives the "more releases" text. `Loaded` with no row and no Library row gives the empty text |
| R17-10 | `FetchNameLevelReleases` for "sir seat sitter" sends one `GET /v1/feeds/by-name?artist_key=sir%20seat%20sitter&limit=200` with no `include`. The recorded response of 2026-10-20 gives 4 releases: 3 singles and "Waitin' on Alex" with 17 tracks |
| R17-11 | With `has_more` true on each response, the command sends 5 requests and sets `more_available`. The key of "Unknown Artist" sends no request |
| R17-12 | A result for a key that is not the current key is dropped. A restore for the mounted key sends no request |
| R17-13 | `ArtistNameKey` values with the key "haleen" and the labels "Haleen" and "HALEEN" are equal. A path that opens both holds one breadcrumb item |
| R17-14 | Each entry point of "Each entry point" gives `FrameNavigationEntry::ArtistName` with the key of its text. A test covers the Artists row target, `ReleasePageVm::name_links` with `ArtistFeed::None`, and the two search row ids |
| R17-15 | A search artist row has the label "Releases that name Survival Guide", an equal accessibility label, and no secondary text |
| R17-16 | `DetailListKind::for_entry(&ArtistName(_))` gives `Releases`. The detail page rows of the page give one row for each release with a release key |
| R17-17 | `collection_art()` gives the mosaic of the newest four covers with `CollectionKind::Artist`, a single cover as a square, and the entity color with no cover. It never uses a provided image |
| R17-18 | A tree change that adds a Library album of the name gives "In Library" n + 1. It also gives an "In Library" row on the next `NameLevelPageVm`, with no new request |
| R17-19 | The stored contract `musicindex-openapi-0.12.0.json` has `artist_key` on `/v1/feeds/by-name`. No test reads the 0.11.0 file |
| R17-20 | The guard `adr_0084_name_level_r17_one_count_function` passes. A copy of `artists_list` that counts members with no call to `name_level_members` fails it |
| R17-21 | The feed page gives the same sections, rows and texts as before the move of the row renderer. The present `adr_0085_feed_page_` tests pass with no change |

## Visual Acceptance Criteria

These are for the operator. No test proves them. Each check runs in the Light theme and in the Dark theme.

- V1: the Artists list shows "Haleen" with "n releases · by name". Its page shows "In Library: n" with the same n. No release of the Haleen artist feed page shows on this page. The feed row "Haleen" stays a separate row.
- V2: the page title reads "Releases that name Haleen". The header shows "Confirmation: Name only" and the note that a name is not an identity. The frame title is "Artist name". The breadcrumb reads "Artists › Haleen · by name".
  - This carries §21 V3 of the overhaul plan list. ADR 0085 Decision 2 retires its text "Grouped by name": the level word of the name level is "Name only".
- V3: the page shows "Albums and EPs" before "Singles". Each Library release shows "In Library". Each release shows one time. The page shows no aliases, no area, no active years and no source subjects.
  - A missing Library release, an empty page for a name with Library releases, or an error report is wrong. This carries §12 V1. ADR 0084 Decision 7 retires its track list: the name level lists releases.
- V4: from an Index release page that names no artist feed, the artist name opens the name level page. The page shows the MusicIndex releases of the name. A release row opens its release page, and Back returns to the name level page.
- V5: a search for "Survival Guide" shows the row "Releases that name Survival Guide", with no count. The row opens the name level page. A release opens its release page.
  - This carries §23 V2. ADR 0084 Decision 7 retires its "Tracks matching" page and its track list. It also changes the row text of §23 V1, which is in the final walk of the run of 2026-10-20.
- V6: a search for a name with no release on the name level opens the page with its empty text. This carries §23 V3.
- V7: with MusicIndex unreachable, the page shows the report first, and the Library releases stay in their sections. While MusicIndex answers, the page shows the loading text.
- V8: "Select", two releases, then "Download 2 releases" on the action bar. Each downloaded release then shows "In Library" in place, and the "In Library" count rises, with no navigation.
- V9: the header art is the logo mosaic of up to four covers. A name with one cover shows that cover as a square. A name with no cover shows the artist color.
- V10: normal and narrow widths, and the larger type sizes. Each element is in its place, with no clipped text. The section headings are stronger than the row text. This carries §21 V4 for the page and §23 V4 for the search row.

The implementer changes the overhaul plan list in the same change:

- §12: "Carried by ADR 0084 packet 017 V3. Retired: the track list of V1 (ADR 0084 Decision 7)."
- §21: "Carried by ADR 0084 packet 017 V2 and V10. Retired: the text 'Grouped by name' of V3 (ADR 0085 Decision 2)."
- §23: "Carried by ADR 0084 packet 017 V5, V6 and V10. Retired: the 'Tracks matching' page of V2 and the row text of V1 (ADR 0084 Decision 7)." V1 keeps its rule that the row does not look like an artist identity.

## Exclusions

- No scope control and no MusicIndex rows in the Artists list. Packet 016 owns them.
- No search entry of a feed page, no merge of the search rows and no new search view. Packet 010 owns them.
- No change to the feed page, its sections or its texts.
- No artist name link on the track page. The track page has none today.
- No "Load more" on the name level page.
- No schema change and no migration.
- No new MusicIndex endpoint. The packet uses `artist_key` of release 0.12.0 only.
- No change to `src/sources.rs` apart from D13.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/view_models/artist_identity.rs`, `src/view_models/library.rs` (`artists_list`, `artist_list_row`, `LibraryArtistDetailVm`), `src/view_models/singles_group.rs`.
- `src/view_models/publisher_page.rs`, `src/ui/shells/publisher.rs`, `src/app/publisher_dispatch.rs`: the feed page that this page reuses.
- `src/view_models/name_match_page.rs`, `src/ui/shells/name_match_page.rs`, `src/app/name_match_dispatch.rs`: the replaced Index page.
- `src/library.rs`, `src/library/app_impl.rs` (`select_artist`, `hydrate_detail_from_nav`, `open_destination_row`, `open_album_name_link`), `src/ui/shells/library/detail.rs`, `src/ui/shells/library/feed_list.rs`.
- `src/app.rs` (`content_list_frame_title`, the body switch), `src/app/search_dispatch.rs` (the search row handlers, `open_index_album_name_link`, `open_library_artist_in_frame`).
- `src/view_models/workspace/nav.rs`, `src/view_models/workspace/breadcrumb.rs`, `src/view_models/workspace/tests.rs`.
- `src/view_models/detail_selection.rs`, `src/app/detail_selection.rs`.
- `src/application/queries/search.rs`, `src/application/request_profiles.rs`, `src/api.rs`, `tests/fixtures/`.
- `src/view_models/search_results/results.rs`, `src/view_models/entity_detail.rs` (`name_links`), `src/view_models/collection_art.rs`.
- `src/ui/shells/artist.rs`, `src/view_models/artist.rs`, `src/view_models/artist_detail.rs`, `src/views.rs` (`ArtistView::from_local_rows`), `src/sources.rs`.
- `tests/architecture_tests.rs`: each guard in "The guards that pin the old pages".

## Checks

```bash
cargo test --lib adr_0084_name_level_
cargo test --lib adr_0085_feed_page_
cargo test --lib adr_0084_singles_group_
cargo test --lib adr_0084_detail_selection_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the commit. This packet adds no migration and no stored data. A revert also restores the 0.11.0 contract file, unless packet 016 replaced it first.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0084-task-017-name-level-artist-page.md`
- ADR 0084 Decisions 4, 6, 7, 9, 10 and 12, and ADR 0085 Decisions 1, 2 and 6
- The archived packet `docs/tasks/archive/adr-0084-task-008-feed-page-and-artists.md`
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the name key, the count function, the entry, the page, the request, the entry points, the deletions, the guards and the documents.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- The view model gives each text, count, state and availability. The screen only composes. Use named tokens, named icons and named constants.
- No interface text names a feed "publisher" (ADR 0085 Decision 1). Code identifiers keep their names.
- Column text does not call `truncate()` (ADR 0063).
- Tests use recorded values. No test sends a request to `api.musicindex.org`.
- Delete each replaced page, each dead function and each guard of a replaced rule in the same change.
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git add` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The feed page behavior, its sections and its texts.
- The Artists list scope and the search results view. Packets 016 and 010 own them.
- The database schema and the migration registry.
- Any ADR text apart from the status line of ADR 0085.

Acceptance criteria:
- Each case R17-01 to R17-21 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - <date>" section and an "Operator Visual Check" section for V1 to V10, with numbered steps, the commands, what is wrong, and the cleanup.

Test commands:
- The commands in "Checks", with `cargo fmt -- --check` after the last edit and `cargo build --bin v4vmm` last.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- The operator did not confirm decision D3, and the orchestrator did not tell you to continue.
- The live contract or a recorded response does not agree with "Recorded Facts". An example is a `name` row whose `release_count` differs from the row count of `artist_key`.
- `name_level_members` cannot serve both the Artists row and the page without a second grouping rule.
- The feed page row renderer cannot move to a composite without a change to the feed page.
- A change needs a file in "Do not touch".
