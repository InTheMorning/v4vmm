# ADR 0084 Task 017: The Name Level Of The Artist Page

Status: Implemented - 2026-10-21. Cases R17-01 to R17-30 pass. Mechanical checks are Green. V1 to V11 are open, in the final walk of the run.

## Goal

The name level of the artist page replaces two old pages: the Library artist page by name and the Index name match page.
The new page shows "Releases that name <name>". It says that a name is not an identity, and it never merges with a feed page.

The page groups its releases by kind: "Albums and EPs", then "Singles". It shows the Library releases of the name and the MusicIndex releases of the name, each release one time. A release in the Library shows the word "In Library".

A name that matches exactly one artist feed has no name level page and no Artists row (ADR 0084 Decision 7, amended on 2026-10-20). The feed page holds its releases in a section "Matched by name", and each row there says that the relationship is inferred.
A name that two or more feeds match keeps its name level page. One pure function gives the match result of each name. The Artists rows, the feed page section, the name level page and the search rows all use it.

The Haleen feed page thus lists its feed releases, then the name-only releases of "Haleen" under "Matched by name". The Artists list has no "Haleen · by name" row. The Haleen feed row counts both sets. Each release shows on one artist page only, so the "3 vs 6" defect goes.

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md) Decision 7: the name level, the label "Releases that name <name>", the note that a name is not an identity, and no merge with a feed page. It replaces the Library artist page by name and the Index name match page.
  - The amendment of 2026-10-20 adds two rules. The key of a name is its sort key. A name that matches exactly one artist feed shows on that feed page in a section "Matched by name". It has no name level page and no row of its own.
- ADR 0084 Decision 4: one page for each item, and the Library is a state. Decision 6: "Albums and EPs" before "Singles". Decision 5: the kind of a release.
- ADR 0084 Decision 9: one navigation stack, and a breadcrumb that holds a page one time at most. Decision 10: the selection model of each list of releases.
- ADR 0084 Decision 12: the name level gives no image. Its art is the logo mosaic, or the entity color.
- [ADR 0085](../adr/0085-artists-by-confirmation.md) Decision 2: the level "Name only" is the name level. Decision 1: no interface text names a feed "publisher". Decision 6: the rule that selects the name level for a Library release.
- [ADR 0075](../adr/0075-metadata-contract.md) section 6: the stored contract changes in the same change as its decode.
- [ADR 0016](../adr/README.md): no schema change in this packet.
- The phase plan row of packet 017 and its note: "The 'Haleen · by name' row says 3 releases, and its page shows 6". The operator decided on 2026-10-18 that this defect waits for packet 017. The operator chose the fix on 2026-10-20: the feed page holds all (decision D1).
- Stophammer release 0.12.0 (Stophammer ADR 0071 section 4): `artist_key` on `GET /v1/feeds/by-name` gives the releases that the `release_count` of a `name` row counts.
- Stophammer ADR 0070 section 1: one sort key function, the normalization of Stophammer ADR 0049 section 7.

## Dependency On Packet 016

This packet does not wait for packet 016. The name level page is a page, not a list. ADR 0084 Decision 8 gives the scope control to the Releases list and the Artists list only.
The feed page of packet 008 already shows its MusicIndex releases with no scope control. The name level page does the same (decision D2).

Each of the two packets can come first. These rules apply to the one that comes second:

- The stored contract: the first packet replaces `musicindex-openapi-0.11.0.json` with the 0.12.0 contract. The second packet uses that file.
- The sort key: this packet adds `artist_name_key` in `src/view_models/artist_identity.rs`. Packet 016 uses it to merge the Library rows and the MusicIndex rows of the Artists list.
- The count: in the scope "Everything", packet 016 counts a name row as the "Releases" fact of this page counts it. That is the union of the Library set and the MusicIndex set, by feed GUID (decision D2).
- The match: this packet adds `NameFeedMatch` and `LibraryViewModel::name_feed_matches`. In both scopes, a name row whose key gives `MatchedFeed` gives no row, and its Library releases count in the Library feed row (decision D14). When packet 016 comes first, this packet applies the rule to the "Everything" merge and adds the case R16-21 of packet 016.

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

### The name match

Add `NameFeedMatch`, `NameFeedMatches` and the pure function `name_feed_matches` to `src/view_models/artist_identity.rs`. The input is one entry for each artist feed: its feed GUID and its key set. The output gives one `NameFeedMatch` for each key:

- `MatchedFeed(feed_guid)`: the key is in the key set of exactly one feed.
- `Ambiguous`: the key is in the key sets of two or more feeds.
- `NoFeed`: the key set of no feed holds the key.

`NameFeedMatches::get(key)` gives `NoFeed` for a key with no entry. The key of "Unknown Artist", the fallback text of `display_artist`, always gives `NoFeed`.

Add `LibraryViewModel::name_feed_matches(&self) -> NameFeedMatches` in `src/view_models/library.rs`. It builds the input from the Library tree at each call:

- The feeds are the feed identities that `ArtistIdentity::resolve` gives over the tree. A label feed thus never matches (ADR 0085 Decision 2).
- The key set of a feed holds `artist_name_key` of the feed title. The title comes from the `named_feeds` entry of the feed, as `artist_list_row` reads it.
- The key set also holds `artist_name_key` of `display_artist` of each Library release whose identity is that feed.

The match reads the Library tree only, in each scope and at each entry point (decision D14). A MusicIndex feed that the Library does not hold matches no name.

Add `LibraryViewModel::matched_name_keys(&self, feed_guid) -> Vec<ArtistNameKey>`: each key of the key set of the feed whose match is `MatchedFeed(feed_guid)`, in key order.

### One count function

Add `LibraryViewModel::name_level_members(&self, key: &str) -> Vec<NameLevelLibraryRelease>` in `src/view_models/library.rs`. It gives each album of the tree whose `artist_identity` is `ArtistIdentity::Name(key)`.

- `artists_list` and the name level page both use this one function. The row count and the "In Library" fact of the page are the length of its result.
- `artists_list` gives a name row only for a key whose match is `NoFeed` or `Ambiguous`.
- For a key whose match is `MatchedFeed(guid)`, `artists_list` gives no name row. The members of the key count in the feed row of `guid`. The feed row text stays "n release(s)". Its n is the count of the feed members plus the count of `name_level_members` of each key of `matched_name_keys(guid)`.
- The feed page section "Matched by name" uses the same two functions. The feed row count thus equals the Library rows of the feed members plus the Library rows of that section.
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

`sections()` merges the two lists into rows, then puts each row in one section. Rules 1, 2, 5 and 6 are one shared function, `name_level_rows` in `src/view_models/name_level_page.rs`. The feed page section "Matched by name" uses it too.

1. Each Library release gives one row, with `in_library` true and the word "In Library".
2. Each MusicIndex release gives one row, unless one of these is true:
   - Its feed GUID is the feed GUID of a Library row of this page. The Library row stays, one time.
   - Its feed GUID is in `other_identity_feed_guids`. The Library rule of ADR 0085 Decision 6 decides the identity of a Library release, so the row is absent.
3. The kind of a Library row is `AlbumNode::release_kind`. The kind of a MusicIndex row is `ReleaseKind::resolve(None, episode_count)`. A null count counts as 0, as on the feed page.
4. "Albums and EPs" holds the rows of kind Album or EP. "Singles" holds the rows of kind Single. "Albums and EPs" comes first. A section with no row is absent.
5. Inside a section, the rows go newest first. The date of a Library row is the newest `pub_date` of its tracks. The date of a MusicIndex row is `newest_item_at`. A row with no date comes last. Equal dates sort by the lowercase title.
6. Each row is a `PublisherPageAlbumVm` with `artist: None`, `not_listed: false` and `role_line: None`. Its `open` action has the release key and the label "Open the release <title>". A Library release with no feed GUID uses its Library release key.

### The feed page section "Matched by name"

The feed page lists the releases of each matched name in one section (decision D1).

- Add `MatchedByNameFacts` to `src/view_models/publisher_page.rs`: `keys: Vec<ArtistNameKey>`, `library: Vec<NameLevelLibraryRelease>`, `index: NameLevelIndexState` and `other_identity_feed_guids`.
- Add `PublisherPageVm::with_matched_by_name(facts)`. `render_publisher_page_content` in `src/app/publisher_dispatch.rs` builds the facts at each render. The keys come from `matched_name_keys`, and `library` holds `name_level_members` of each key. A download or a removal thus changes the section in place.
- After the feed page loads in the Library context, the dispatch sends `FetchNameLevelReleases` for each key, in key order. `PublisherPageState` keeps the results with the page. The Index context sends no request, because the match reads the Library tree only.
- `sections()` adds the section after each other section. The heading is `MATCHED_BY_NAME_HEADING`, "Matched by name". The section has one list, newest first, with no split by kind.
- The rows come from `name_level_rows`. A row whose feed GUID is in another section of the page is absent, so each release shows one time.
- Each row has the detail line `names "<artist text>", no feed`, from `matched_by_name_line(text)`. The artist text is the raw text of the release: `display_artist` for a Library row, `release_artist` for a MusicIndex row.
- Rename the field `role_line` of `PublisherPageAlbumVm` to `detail_line`, because it now holds two kinds of line. The renderer draws it at the same place.
- `PublisherPageSectionVm` adds `note: Option<String>` and `status: Option<String>`. The note of this section is `MATCHED_BY_NAME_NOTE`: "These releases name no artist feed. The app relates them to this artist by the name only."
  The status is the loading text, the failure report or the "more releases" text of the MusicIndex part, as on the name level page. Each other section has `None` for both fields.
- The section shows when it has a row, or when its MusicIndex part is `Loading` or `Failed`. With `Failed`, the Library rows stay.

### The MusicIndex request

- Add `api::Client::fetch_feeds_by_name_with_profile(artist_key, limit, cursor, profile)` in `src/api.rs`. It sends `GET /v1/feeds/by-name` with `artist_key`, `limit` and `cursor`. It decodes `RecentFeedsResponse`, because the rows have the fields of `/v1/feeds/recent`.
- Add the request profile `INDEX_NAME_LEVEL_RELEASES` with a new path shape `FeedsByName` and no include. Delete `INDEX_NAME_MATCH_TRACKS` and the path shape `TracksByArtistName` when no caller stays.
- Add the command `FetchNameLevelReleases` in `src/application/queries/search.rs`, in place of `FetchNameMatchTracks`. It sends `limit=200`. It follows `pagination.cursor` while `has_more` is true, for 5 requests at most. After 5 requests with `has_more` true, it sets `more_available`.
- The command gives `Vec<NameLevelIndexRelease>`: the feed GUID, the title, the image URL, `release_artist`, `episode_count` and `newest_item_at`. A row with no feed GUID is dropped.
- The page sends no request when the key is `artist_name_key("Unknown Artist")`, the fallback text of `display_artist`. The `index` state is then `NotRequested`, and the page shows no MusicIndex text.
- The response is untrusted input. The command makes no other request for each row.

### The dispatch and the screen

- Replace `src/app/name_match_dispatch.rs` with `src/app/name_level_dispatch.rs`. It holds `NameLevelPageState { key, index: NameLevelIndexState }` on `TopApp`, in place of `name_match_page`.
- `open_name_level_page(key, start, cx)` reads `name_feed_matches` first. For `MatchedFeed(guid)`, it opens the feed page of `guid` with `PublisherPageContext::Library`. Else it pushes the entry through the one navigation stack.
- `restore_name_level_page_for_nav(entry, cx)` starts the request for a different key, and skips it for the mounted key.
- A Library change can make the key of a mounted or restored page match one feed. The page then shows `NameLevelPageVm::matched_feed_notice()`: "The releases of this name now show on the artist page <feed title>." It has an open action to that feed page, and the page shows no section (decision D16).
- A result for a key that is not the current key is dropped, as `name_match_page_result_is_current` does today.
- `render_name_level_page_content` reads `name_level_members` and the other-identity GUIDs from `LibraryApp` at each render. It builds `NameLevelPageVm` and calls the screen.
- Add `src/ui/shells/name_level_page.rs`. It composes the header (`DetailHeader` with `EntityKind::Artist`, the `XXl` collection art and the `Display` title size), the MusicIndex status, and the sections.
- Move `render_section` and `render_album_row` from `src/ui/shells/publisher.rs` to a new composite, `src/ui/composites/release_section_list.rs`. The feed page and the name level page call it. The composite also draws the section note and the section status, with named tokens.
- The feed page changes only by the section "Matched by name" and the rename of `detail_line`. Each other section, row and text stays.
- `LibraryApp::hydrate_detail_from_nav` clears the Library detail for `ArtistName`. `TopApp` renders the page, as it does the feed page.
- The detail column keeps its scrollbar through `music_column_tracked_scrollbar`.

### Each entry point

| Entry point | Today | After this packet |
|---|---|---|
| Artists name row | `ArtistDetail(name)`, old Library page | `ArtistName(key)`. A matched name has no row |
| Artist name link of a Library release page | `OpenArtistPage`, old Library page | `ArtistName(key)` in the current path, or the matched feed page |
| Artist name link of an Index release page | `IndexNameMatches(name)` | `ArtistName(key)` in the current path, or the matched feed page |
| Search row `index-artist:<name>` | `IndexNameMatches(name)` | `ArtistName(key)`, as "The search rows" gives |
| Search row `library-artist:<name>` | `ArtistDetail(name)`, old Library page | `ArtistName(key)`, as "The search rows" gives |
| Back and Forward | `select_artist`, `restore_name_match_page_for_nav` | `restore_name_level_page_for_nav` |

- Delete `LibraryAppEvent::OpenArtistPage`. `LibraryApp::open_album_name_link` emits `LibraryAppEvent::OpenPage` with the new entry.
- Each entry point reads `name_feed_matches` for its key. `MatchedFeed(guid)` opens the feed page of `guid` with `PublisherPageContext::Library`. `NoFeed` and `Ambiguous` open `ArtistName(key)`.

### The search rows

The search gives no empty name level page (decisions D6 and D7).

- Each search artist row reads the match of its key.
- `MatchedFeed(guid)`: the name row is absent. One feed row shows in its place. Its label is the feed title, and its accessibility label is "Open the artist page <feed title>". It opens the feed page of `guid` with `PublisherPageContext::Library`. Two names that match one feed give one feed row.
- `NoFeed` or `Ambiguous`: the Library row shows when `name_level_members(key)` is not empty. The MusicIndex row shows when the MusicIndex check finds a `name` row with the key.
- The MusicIndex check sends `GET /v1/artist-names?from=<key>&limit=1` for each MusicIndex candidate key that is not `MatchedFeed`. The row shows when the first row has `kind` `name` and the same `key`.
  On 2026-10-20, `from=haleen` gave the `name` row first, before the `publisher` rows of the same key. A failed check shows the row. The page then shows its failure report.
- The check uses the request profile `INDEX_ARTIST_NAME_CHECK` with the path shape of `/v1/artist-names` and no include. When packet 016 added `Client::fetch_artist_names` and its path shape, add a `from` argument and use them. Else add them.
- A name row has the label `NameLevelPageVm::title_text_for_name(name)` and an equal accessibility label. It shows no count (decision D7).

### Selection

- `DetailListKind::for_entry` gives `Releases` for `ArtistName(_)`.
- `TopApp::detail_page_rows` gives the rows of `NameLevelPageVm::sections` through `feed_page_release_rows`, as for the feed page.
- The "Select" button, the action bar and "Download n releases" thus work on the name level page, with no new selection code.

### The guard

Add one situational guard to `tests/architecture_tests.rs`: `adr_0084_name_level_r17_one_count_function`. It cites ADR 0084 Decision 7. It earns a test: the "3 vs 6" defect of 2026-10-18.

- The body of `artists_list` calls `name_level_members(` and `name_feed_matches(`, or a function that it calls does.
- `src/app/name_level_dispatch.rs` and `src/app/publisher_dispatch.rs` call `name_level_members(` and `name_feed_matches(` or `matched_name_keys(`.
- No file in `src/` contains `fn select_artist(`, `LibraryArtistDetailVm`, `FrameNavigationEntry::ArtistDetail`, `IndexNameMatches` or `FetchNameMatchTracks`.

Its failure message names the owner and the fix:

```text
ADR 0084 §7 One Artist Page With Two Levels: <file> counts or lists the name level apart from `name_level_members` and `name_feed_matches`.
The Artists rows, the name level page and the feed page section count one set. Call `LibraryViewModel::name_level_members` and `LibraryViewModel::name_feed_matches` in each.
```

## Orchestrator Decisions

The orchestrator made these decisions on 2026-10-20. "Flag" marks a decision that removes or changes something that the operator sees today. The operator made D1, D6 and D7 on 2026-10-20.

- **D1. The feed page holds all. Operator decision of 2026-10-20.** ADR 0084 Decision 7, as amended on 2026-10-20, records the rule.
  - A name that matches exactly one artist feed has no name level page and no Artists row. The feed page lists its feed releases, then the section "Matched by name" with the name-only releases. Each row there says `names "<name>", no feed`.
  - The Artists feed row counts the feed releases and the matched releases. A name that two or more feeds match keeps its name level page and its row.
  - The name level page, a name row and the "In Library" fact use `name_level_members`. `name_feed_matches` gives the match for the Artists rows, the feed page section, the name level page and the search rows.
  - The "3 vs 6" defect goes: each release shows on one artist page only, and the row count and the page agree.
  - **Flag:** the "Haleen · by name" row goes, and the Haleen feed row count rises by the name-only releases. The phase plan records 6 releases on the old page and 3 on the row.
- **D2. MusicIndex releases always show.** The page shows the MusicIndex releases of `artist_key` with no "Everything" scope, as the feed page does. Packet 016 is not a prerequisite. The page can thus show more rows than the row count. The header gives "In Library" and "Releases" as two facts.
- **D3. The name key is the Stophammer sort key.** The operator confirmed D3 on 2026-10-20, and ADR 0084 Decision 7 records it. The name identity uses `artist_name_key`: white space collapsed and Unicode lowercase. The label is the raw text that the most releases state.
  - **Flag:** two Artists rows or two singles groups that differ only in case or in white space become one. The test R15-01 asserted the old rule, and this packet replaces it.
- **D4. The level word replaces "Grouped by name".** The header shows "Confirmation: Name only" (ADR 0085 Decision 2) and the note "A name is not an identity. These releases state this artist text and name no artist feed."
  - **Flag:** the text "Grouped by name" and the facts "Albums", "Tracks" and "Downloaded" go. The page shows releases, not tracks.
- **D5. Titles.** The page title is `Releases that name <label>`, with no quotation marks, as ADR 0084 Decision 7 writes it. The frame title is "Artist name". The breadcrumb label is `<label> · by name`, the mark of the Artists row.
  - **Flag:** the frame title "Artist" of the old Library page and "Tracks matching" of the old Index page go. The quoted breadcrumb `"<name>"` goes.
- **D6. The name match page goes, and search opens the name level. Operator decision of 2026-10-20.** The search row "Releases that name <name>" opens the name level page of its name.
  - When the name has no name-only release, the name row is absent. When the name matches exactly one feed, the name row is absent, and the feed row of that artist shows in its place. The search never opens an empty page.
  - **Flag:** the list of tracks whose artist text matches a name goes. A track search keeps the tracks. Packet 010 adds the search entry of each other feed page (flow map finding 6).
- **D7. The search artist rows show no count. Operator decision of 2026-10-20.** Their label is the page title, `Releases that name <name>`, and their accessibility label is equal. A search hit count does not count the releases of the name level, so the row shows none. D6 gives when the row is absent.
  - **Flag:** the texts "n feeds · m tracks" and "n albums - m tracks" go from the Artists tab of search. The text `Tracks matching "<name>"` goes.
  A Library row and a MusicIndex row of one name can both show. Each opens the same page. Packet 010 owns the search results view and merges them.
- **D8. One section pair only.** The name level page has "Albums and EPs" and "Singles" only. A name level release names no artist feed, so no "As <roles>", "Role not confirmed" or "Listed by" section applies.
- **D9. The Library decides a Library release.** A MusicIndex row of a release that the Library holds under another identity is absent. Stophammer ADR 0070 section 4 and ADR 0085 Decision 6 differ for one case only. The phase plan open finding of 2026-10-18 records that case.
- **D10. At most 5 requests of 200 rows.** The page lists the first 1,000 MusicIndex releases and says when more exist. On 2026-10-20, the largest recorded name row had a `release_count` far below this limit. This packet adds no "Load more".
- **D11. Selection is on.** The page has the detail selection of packet 018 through the feed page rows. No new selection code.
- **D12. No dependency on packet 016.** See "Dependency On Packet 016". The match rule of D14 applies in both scopes, whichever packet comes first.
- **D13. The dead artist code goes.** `src/ui/shells/artist.rs`, `src/view_models/artist.rs` and `src/view_models/artist_detail.rs` lose their last caller and go. `Source::fetch_artist` and `ArtistRef::LocalArtistName` go when no composition root reaches them. The rest of `src/sources.rs` stays, and the result records it as an open finding.
- **D14. The match reads the Library tree only.** `name_feed_matches` uses the Library artist feeds, in each scope and at each entry point. Thus the scope, the load order of MusicIndex pages and the search never change the result of a name.
  - The MusicIndex name rows of packet 016 follow the same rule. In "Everything", a MusicIndex name row whose key gives `MatchedFeed` merges into the Library feed row, because that feed row is in the same list. Any other MusicIndex name row stays.
  - A MusicIndex name row can thus stay beside a MusicIndex feed row of the same name when the Library holds no feed of that name. The result records this as an open finding for a later packet.
  - A feed page in the Index context has no section "Matched by name".
- **D15. One check request for each MusicIndex search name.** The search cannot know from its hits if a name has a name-only release. It sends one `GET /v1/artist-names?from=<key>&limit=1` for each MusicIndex candidate key that is not `MatchedFeed`.
  - The search response gives a small number of candidate keys. The check reads no release detail.
  - **Flag:** a search sends more MusicIndex requests than today.
- **D16. A matched key on a mounted page.** A download can make the key of an open name level page match one feed. The page then shows a notice and an open action to that feed page, not an empty page and not a navigation change.

## Required Changes

1. Add `artist_name_key`, `ArtistNameKey` and the label rule. Change `ArtistIdentity::resolve`, the Artists row label and the search candidate merge to use them.
2. Add `name_level_members` and use it in `artists_list`. Add `NameFeedMatch`, `NameFeedMatches`, `name_feed_matches` and `matched_name_keys`. Remove the matched name rows from `artists_list`, and add their counts to the feed rows.
3. Replace the stored contract with the 0.12.0 contract, unless packet 016 did it first. Name the file `musicindex-openapi-0.12.0.json`, delete the 0.11.0 file and change each test that reads the file name. Change the contract line of `AGENTS.md` in the same change.
4. Add the client method, the request profile and `FetchNameLevelReleases`. Delete `FetchNameMatchTracks`, `INDEX_NAME_MATCH_TRACKS` and `fetch_tracks_by_artist_with_profile` when no caller stays.
5. Add `FrameNavigationEntry::ArtistName`. Delete `ArtistDetail` and `IndexNameMatches`, with their labels, ids, frame titles and body switch arms.
6. Add `src/view_models/name_level_page.rs`, `src/app/name_level_dispatch.rs`, `src/ui/shells/name_level_page.rs` and `src/ui/composites/release_section_list.rs`.
7. Change each entry point of "Each entry point". Delete `LibraryAppEvent::OpenArtistPage`, `open_library_artist_page` and `open_library_artist_in_frame`.
8. Delete the replaced pages and their dead code: `LibraryDetail::Artist`, `LibraryArtistDetail`, `select_artist`, `select_album_by_name` when no caller stays, `LibraryArtistDetailVm`, `ArtistFeedSummaryVm`, `ArtistFeedSummaryDisplay`, `src/ui/shells/library/feed_list.rs`, `src/app/name_match_dispatch.rs`, `src/view_models/name_match_page.rs`, `src/ui/shells/name_match_page.rs`, and the files of D13.
9. Add the selection of "Selection". The rows of the section "Matched by name" are feed page rows, so `feed_page_release_rows` gives them with no new code.
10. Add the feed page section of "The feed page section 'Matched by name'": `MatchedByNameFacts`, `with_matched_by_name`, the section, the note, the status, the rename of `role_line` to `detail_line`, and the MusicIndex requests of `src/app/publisher_dispatch.rs`.
11. Add the search rows of "The search rows": the match of each row, the feed row in place of a matched name, the MusicIndex check and its request profile.
12. Change each guard in "The guards that pin the old pages":
    - Delete `adr_0077_name_matches_index_row_opens_tracks_matching_not_artist_page` and its entry in `adr_0024_loading_shape_readiness_gate_is_locked`. ADR 0084 Decision 7 supersedes the rule of ADR 0077 packet 006 that it guards.
    - Replace `restore_name_match_page_for_nav`, `IndexNameMatches(` and `ArtistDetail(_)` with the new names in the other guards.
    - Remove the deleted files from `SCREEN_FILES`, `LIBRARY_SCREEN_SURFACE_FILES`, `entity_detail_pages_render_through_shell_helper_and_page_vm`, `shared_top_level_ui_shells_do_not_import_screen_modules` and `adr_0084_scrollbars_r14_01_music_columns_use_the_shared_helper`. Add the new files where the old ones were.
    - Move the end marker of `adr_0075_library_observation_callers_and_consumers_are_guarded` to the function that follows `hydrate_album_identity_on_view`.
    - Delete each unit test of the deleted code. Replace R15-01 with R17-02.
13. Add the guard of "The guard".
14. Change these documents in the same change:
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
| R17-03 | The Haleen tree: one `ArtistNode` "Haleen" with 3 albums that name no feed and 3 albums with a confirmed artist feed titled "Haleen". `name_level_members("haleen")` gives 3. `name_feed_matches` gives `MatchedFeed` for "haleen". `artists_list` gives no "Haleen · by name" row, and the Haleen feed row gives "6 releases" |
| R17-04 | A test tree has four names, and one of them is `Ambiguous`. For each name row of `artists_list`, the row count equals the "In Library" fact of its page. No name row has a `MatchedFeed` key |
| R17-05 | The label of a name identity is the raw text that the most members state. A tie gives the first raw text in code point order |
| R17-06 | `sections()` gives "Albums and EPs" before "Singles". A Library single and a MusicIndex row with `episode_count` 1 go in "Singles". A null count gives an Album. An empty section is absent. Rows go newest first, and a row with no date comes last |
| R17-07 | A MusicIndex row with the feed GUID of a Library row appears one time, with "In Library". A MusicIndex row whose feed GUID is in `other_identity_feed_guids` is absent. A MusicIndex-only row has `in_library` false |
| R17-08 | `title_text()` gives "Releases that name Haleen". `header_facts()` gives "Confirmation" "Name only", the note, and "In Library". "Releases" shows only for `Loaded`. `frame_title()` gives "Artist name". `display_label` of the entry gives "Haleen · by name" |
| R17-09 | `Loading` gives the loading text. `Failed` gives the report before the detail, and the Library rows stay in their sections. `Loaded` with `more_available` gives the "more releases" text. `Loaded` with no row and no Library row gives the empty text |
| R17-10 | `FetchNameLevelReleases` for "sir seat sitter" sends one `GET /v1/feeds/by-name?artist_key=sir%20seat%20sitter&limit=200` with no `include`. The recorded response of 2026-10-20 gives 4 releases: 3 singles and "Waitin' on Alex" with 17 tracks |
| R17-11 | With `has_more` true on each response, the command sends 5 requests and sets `more_available`. The key of "Unknown Artist" sends no request |
| R17-12 | A result for a key that is not the current key is dropped. A restore for the mounted key sends no request |
| R17-13 | `ArtistNameKey` values with the key "haleen" and the labels "Haleen" and "HALEEN" are equal. A path that opens both holds one breadcrumb item |
| R17-14 | Each entry point of "Each entry point" gives `FrameNavigationEntry::ArtistName` with the key of its text for `NoFeed` and `Ambiguous`. It gives the feed page of the feed for `MatchedFeed`. A test covers the Artists row target, `ReleasePageVm::name_links` with `ArtistFeed::None`, and the two search row ids |
| R17-15 | A search artist row of a `NoFeed` name with Library members has the label "Releases that name Survival Guide", an equal accessibility label, and no secondary text |
| R17-16 | `DetailListKind::for_entry(&ArtistName(_))` gives `Releases`. The detail page rows of the page give one row for each release with a release key |
| R17-17 | `collection_art()` gives the mosaic of the newest four covers with `CollectionKind::Artist`, a single cover as a square, and the entity color with no cover. It never uses a provided image |
| R17-18 | A tree change that adds a Library album of the name gives "In Library" n + 1. It also gives an "In Library" row on the next `NameLevelPageVm`, with no new request |
| R17-19 | The stored contract `musicindex-openapi-0.12.0.json` has `artist_key` on `/v1/feeds/by-name`. No test reads the 0.11.0 file |
| R17-20 | The guard `adr_0084_name_level_r17_one_count_function` passes. A copy of `artists_list` that counts members with no call to `name_level_members` fails it. A copy that gives a name row with no call to `name_feed_matches` fails it |
| R17-21 | With no matched name, the feed page gives the same sections, rows and texts as before the move of the row renderer. The present `adr_0085_feed_page_` tests pass with the rename of `role_line` to `detail_line` as their only change |
| R17-22 | `name_feed_matches`, exactly one: a key equal to the key of one feed title gives `MatchedFeed` of that feed. A key equal to the key of the artist text of a release of one feed gives the same. "HALEEN " matches the feed title "Haleen" |
| R17-23 | `name_feed_matches`, two or more: a key in the key sets of two feeds gives `Ambiguous`. The name row stays with its count. Neither feed row counts its releases, and neither feed page has a section "Matched by name" |
| R17-24 | `name_feed_matches`, none: a key in no key set gives `NoFeed`, and so does a key with no entry. The key of "Unknown Artist" gives `NoFeed` when a feed has that title. A label feed of ADR 0085 Decision 2 matches no key |
| R17-25 | The feed page section: with the R17-03 tree, `sections()` puts the 3 feed albums in "Albums and EPs" or "Singles". "Matched by name" comes last with the 3 name-only albums. The section has the note. Each row has the detail line `names "Haleen", no feed`, "In Library" and an open action |
| R17-26 | The feed page section, MusicIndex part: a MusicIndex row of a matched key with a new feed GUID is in the section. A row whose feed GUID is in another section is absent. `Loading` and `Failed` give the status, and the Library rows stay. With no Library row and `Loaded` with no row, the section is absent |
| R17-27 | The Artists feed row count is the count of the feed members plus the count of `name_level_members` of each key of `matched_name_keys`. It equals the Library rows of the feed members plus the Library rows of the section "Matched by name" |
| R17-28 | The search rows: a `MatchedFeed` name gives no name row. It gives one feed row with the feed title, its accessibility label and the feed page target. Two names that match one feed give one feed row |
| R17-29 | The search rows: a `NoFeed` name with no Library member gives no Library row. A MusicIndex candidate sends one `GET /v1/artist-names?from=<key>&limit=1`. A first row `name` with the same key shows the row. A first row of another key or `kind` hides it. A failed check shows it. A `MatchedFeed` candidate sends no check |
| R17-30 | A tree change makes the key of the mounted page `MatchedFeed`. Then `matched_feed_notice()` gives the feed title and its open action, and `sections()` gives no section |

## Visual Acceptance Criteria

These are for the operator. No test proves them. Each check runs in the Light theme and in the Dark theme.

- V1: the Artists list shows no "Haleen · by name" row. The Haleen feed row shows "n releases", and n is the count of its feed releases plus its name-only releases in the Library. The feed page lists its feed releases, then the section "Matched by name" last.
  - The section has the note that the app relates these releases to the artist by the name only. Each row shows `names "Haleen", no feed` and "In Library". Each release shows one time on the page.
  - A "Haleen · by name" row, a name-only release that shows on no page, or a row of the section that looks like a confirmed release is wrong. This carries §12 V1 for a matched name.
- V2: open a name row of the Artists list, for example "<name> · by name". The page title reads "Releases that name <name>". The header shows "Confirmation: Name only" and the note that a name is not an identity. The frame title is "Artist name". The breadcrumb reads "Artists › <name> · by name".
  - This carries §21 V3 of the overhaul plan list. ADR 0085 Decision 2 retires its text "Grouped by name": the level word of the name level is "Name only".
- V3: the page shows "Albums and EPs" before "Singles". Each Library release shows "In Library". Each release shows one time. The page shows no aliases, no area, no active years and no source subjects. The row count of V2 equals the "In Library" fact.
  - A missing Library release, an empty page for a name with Library releases, or an error report is wrong. This carries §12 V1. ADR 0084 Decision 7 retires its track list: the name level lists releases.
- V4: from an Index release page that names no artist feed and no matched name, the artist name opens the name level page. The page shows the MusicIndex releases of the name. A release row opens its release page, and Back returns to the name level page. From a release that names "Haleen" and no feed, the artist name opens the Haleen feed page.
- V5: a search for a name with a name-only release and no matched feed shows the row "Releases that name <name>", with no count. The row opens the name level page. A release opens its release page.
  - This carries §23 V2. ADR 0084 Decision 7 retires its "Tracks matching" page and its track list. It also changes the row text of §23 V1, which is in the final walk of the run of 2026-10-20.
- V6: a search for "Haleen" shows no "Releases that name Haleen" row. It shows the Haleen feed row, which opens the Haleen feed page. A search for a name with no name-only release shows no name row, and no search opens an empty page.
  - This replaces §23 V3. Operator decision D6 of 2026-10-20 retires its empty text.
- V7: with MusicIndex unreachable, the page shows the report first, and the Library releases stay in their sections. While MusicIndex answers, the page shows the loading text. The section "Matched by name" of a feed page does the same.
- V8: "Select", two releases, then "Download 2 releases" on the action bar. Each downloaded release then shows "In Library" in place, and the "In Library" count rises, with no navigation.
- V9: the header art is the logo mosaic of up to four covers. A name with one cover shows that cover as a square. A name with no cover shows the artist color.
- V10: normal and narrow widths, and the larger type sizes. Each element is in its place, with no clipped text. The section headings are stronger than the row text. The note and the detail line of "Matched by name" are weaker than the row title. This carries §21 V4 for the two pages and §23 V4 for the search row.
- V11: a name that two Library artist feeds match keeps its name row "<name> · by name" and its name level page. Neither feed page shows its releases under "Matched by name". The implementer names such a name of the operator Library in the Operator Visual Check, or records that the Library holds none.

The implementer changes the overhaul plan list in the same change:

- §12: "Carried by ADR 0084 packet 017 V1 and V3. Retired: the track list of V1 (ADR 0084 Decision 7)."
- §21: "Carried by ADR 0084 packet 017 V2 and V10. Retired: the text 'Grouped by name' of V3 (ADR 0085 Decision 2)."
- §23: "Carried by ADR 0084 packet 017 V5, V6 and V10. Retired: the 'Tracks matching' page of V2 and the row text of V1 (ADR 0084 Decision 7), and the empty text of V3 (packet 017 decision D6, operator decision of 2026-10-20)." V1 keeps its rule that the row does not look like an artist identity.

## Exclusions

- No scope control and no MusicIndex rows in the Artists list. Packet 016 owns them.
- No search entry of a feed page, no merge of the search rows and no new search view. Packet 010 owns them.
- No change to the feed page apart from the section "Matched by name" and the rename of `detail_line`.
- No artist name link on the track page. The track page has none today.
- No "Load more" on the name level page.
- No schema change and no migration.
- No new MusicIndex endpoint. The packet uses `artist_key` on `/v1/feeds/by-name` and `from` on `/v1/artist-names` of release 0.12.0 only.
- No match against a MusicIndex feed that the Library does not hold (decision D14).
- No change to `src/sources.rs` apart from D13.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/view_models/artist_identity.rs`, `src/view_models/library.rs` (`artists_list`, `artist_list_row`, `LibraryArtistDetailVm`), `src/view_models/singles_group.rs`.
- `src/view_models/publisher_page.rs`, `src/ui/shells/publisher.rs`, `src/app/publisher_dispatch.rs`: the feed page that this page reuses. This packet edits all three for the section "Matched by name".
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

## Operator Answers - 2026-10-20

- D1: replaced. The operator chose "the feed page holds all". A name that matches exactly one artist feed shows on that feed page under "Matched by name", marked as inferred. It has no name level page and no Artists row. ADR 0084 Decision 7, amended on 2026-10-20, records the rule.
- D3: confirmed. ADR 0084 Decision 7 records the name key.
- D6: confirmed with a change. The search row "Releases that name <name>" opens the name level page. When the name has no name-only release, or when it matches exactly one feed, the name row is absent. For a matched name, the feed row of that artist shows in its place. No search opens an empty page.
- D7: confirmed with the change of D6.

## Orchestrator Review - 2026-10-21

Pass. The code needs no fix.

- The orchestrator made Required Change 14: the phase plan row, the ADR 0085 status line, the overhaul plan items §12, §21 and §23, the open gates and the source map.
- The search check of decision D15 is responsiveness item 17 of the overhaul plan.
- `src/sources.rs` had no production caller after this packet. The orchestrator deletes it in a separate change.
- A key that a Library change matches after a feed page loads gets its MusicIndex request only at the next load of the page. Its Library rows show at once. The orchestrator accepts this.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0084-task-017-name-level-artist-page.md`
- ADR 0084 Decisions 4, 6, 7, 9, 10 and 12, and ADR 0085 Decisions 1, 2 and 6. Read Decision 7 with its amendment of 2026-10-20: the name key and the rule "Matched by name".
- The archived packet `docs/tasks/archive/adr-0084-task-008-feed-page-and-artists.md`
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes". These are the name key, the name match, the count function, the entry, the page and the request.
- They also include the feed page section, the entry points, the search rows, the deletions, the guards and the documents.

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
- The feed page behavior, its sections and its texts, apart from the section "Matched by name" and the rename of `detail_line`.
- The Artists list scope and the search results view. Packets 016 and 010 own them. This packet changes only the artist rows of "The search rows". When packet 016 is complete, it also changes the name rows of D14 in its merge.
- The database schema and the migration registry.
- Any ADR text apart from the status line of ADR 0085.

Acceptance criteria:
- Each case R17-01 to R17-30 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - <date>" section and an "Operator Visual Check" section for V1 to V11, with numbered steps, the commands, what is wrong, and the cleanup.

Test commands:
- The commands in "Checks", with `cargo fmt -- --check` after the last edit and `cargo build --bin v4vmm` last.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- One name level page, one feed page section or one Artists row needs a second match rule apart from `name_feed_matches`.
- The live contract or a recorded response does not agree with "Recorded Facts". An example is a `name` row whose `release_count` differs from the row count of `artist_key`.
- `name_level_members` cannot serve both the Artists row and the page without a second grouping rule.
- The feed page row renderer cannot move to a composite without a change to the feed page apart from the section "Matched by name".
- A change needs a file in "Do not touch".

## Implementation Result - 2026-10-21

The implementer made the code changes of "Required Changes" 1 to 13 on 2026-10-21. The gate of `AGENTS.md` is Green. The visual gate stays open.

### Changes

- The name key: `ArtistIdentity::resolve` gives `Name(name_sort_key(text))`. `name_sort_key` of packet 016 in `src/view_models/index_name_list.rs` is the one sort key function. The packet name `artist_name_key` is not added.
- `src/view_models/artist_identity.rs` holds `ArtistNameKey`, `name_label`, `NameFeedMatch`, `NameFeedEntry`, `NameFeedMatches`, `name_feed_matches` and `ArtistNameDestination`. It also holds the moved constants `ARTIST_NAME_ROW_MARK`, `ARTIST_ROW_SEPARATOR` and the new `UNKNOWN_ARTIST_TEXT`.
- `LibraryViewModel` adds `name_level_members_by_key`, `name_level_members`, `name_feed_matches`, `matched_name_keys` and `other_identity_feed_guids`. `artists_list` gives no row for a `MatchedFeed` name. The feed row counts the matched releases.
- `everything_rows` of packet 016 drops a `MusicIndex` name row whose key gives `MatchedFeed` (R16-21).
- `DestinationListRowTarget::ArtistName(ArtistNameKey)` replaces `Artist(String)` and `IndexName(String)`.
- `FrameNavigationEntry::ArtistName(ArtistNameKey)` replaces `ArtistDetail` and `IndexNameMatches`. The breadcrumb id is `artist-name-<slug>`, and the frame title is "Artist name".
- New files: `src/view_models/name_level_page.rs`, `src/app/name_level_dispatch.rs`, `src/ui/shells/name_level_page.rs` and `src/ui/composites/release_section_list.rs`. The feed page and the name level page draw their sections through the composite.
- The feed page has the section "Matched by name" in the Library context. `PublisherPageAlbumVm::role_line` is now `detail_line`. `PublisherPageSectionVm` adds `note` and `status`.
- `FetchNameLevelReleases` sends `GET /v1/feeds/by-name` with `artist_key` and `limit=200`, for 5 requests at most.
- The search artist rows read the Library match. A `MatchedFeed` name gives one feed row (`artist-feed:<guid>`). A name row shows only with a name level member or a `MusicIndex` `name` row. A name row has no count.
- Selection: `DetailListKind::for_entry` gives `Releases` for `ArtistName(_)`, and `TopApp::detail_page_rows` gives the name level rows through `feed_page_release_rows`.
- The new guard `adr_0084_name_level_r17_one_count_function` and the test `adr_0084_name_level_r17_19_contract_declares_the_name_level_parameters` are in `tests/architecture_tests.rs`.

### Deleted

- `LibraryDetail::Artist`, `LibraryArtistDetail`, `select_artist`, `select_album_by_name`, `LibraryArtistDetailVm`, `ArtistFeedSummaryVm`, `ArtistFeedSummaryDisplay` and `LibraryAppEvent::OpenArtistPage`.
- `open_library_artist_page`, `open_library_artist_in_frame` and `handle_index_artist_result_selected`.
- The files `src/ui/shells/library/feed_list.rs`, `src/ui/shells/artist.rs`, `src/view_models/artist_detail.rs`, `src/app/name_match_dispatch.rs`, `src/view_models/name_match_page.rs` and `src/ui/shells/name_match_page.rs`.
- `FetchNameMatchTracks`, `INDEX_NAME_MATCH_TRACKS`, `RequestPathShape::TracksByArtistName` and `fetch_tracks_by_artist_with_profile`.
- D13: `MetadataSource::fetch_artist`, `MetadataSource::list_feeds_for_artist`, `ArtistRef::LocalArtistName`, `ArtistView` and `ArtistView::from_local_rows`.
- The name match cache of the Index track page: `name_match_page_track_row`, `IndexDetailDisplay::track`, `IndexDetailDisplay::track_or_fallback`, `TrackResultDisplay::remote_track`, `with_remote_track` and `ArtistResultDisplay::with_secondary_text`.
- The guard `adr_0077_name_matches_index_row_opens_tracks_matching_not_artist_page` and its entry in `adr_0024_loading_shape_readiness_gate_is_locked`. The guards that named the old pages now name the new pages.

### Tests

- `cargo test`: 2,139 unit tests and 356 guards pass.
- `cargo test --test architecture_tests`, `cargo fmt -- --check`, `cargo clippy --all-targets -- -D warnings` and `cargo check --all-targets` are Green.
- `cargo build --bin v4vmm` ran last.
- Each case R17-01 to R17-30 has a test with the prefix `adr_0084_name_level_r17_`. R17-21 is the present `adr_0085_feed_page_` tests, with the field rename as the only change. R17-20 is part of the guard. R16-21 is `adr_0084_scopes_r16_21_matched_name_rows_in_everything`.

### Deviations

- The caller of this session permitted edits to this packet only. Required Change 14 is not done. These documents do not show this packet yet: the phase plan row, the ADR 0085 status line, the overhaul plan items §12, §21 and §23, `docs/pending-human-checks.md`, `docs/plans/broadcast-chain-delivery-order.md` and `docs/architecture/source-map.md`.
- The name level request uses the client method of packet 016, `fetch_feeds_by_name`, with a new `artist_key` argument. It uses the path shape `FeedNameList` and decodes `FeedNamesResponse`. The search name check uses `fetch_artist_names` with a new `from` argument. No new path shape `FeedsByName` exists.
- The one grouping of the name level is `name_level_members_by_key`. `artists_list` calls it one time for all keys. `name_level_members(key)` reads it. The guard requires this shape, in place of a call of `name_level_members(` in `artists_list`.
- `PublisherPageSectionVm::note` is `Option<&'static str>`. `PublisherPageSectionVm::status` is `Option<ReleaseListStatus>`, not `Option<String>`, so the failure report and its detail stay two texts.
- `NameLevelPageVm` has no `frame_title()` method, because no caller used it. `NameLevelPageVm::FRAME_TITLE` gives "Artist name".
- `ContentListRowKind::Release` now holds a `Box<FeedResultDisplay>`. The removal of `remote_track` made the track variant small, and clippy then reported a large size difference. The guard `adr_0062_mixed_entity_row_contract_is_kind_backed` names the boxed variant.
- The recorded response of R17-10 has placeholder feed GUIDs. The packet did not record the GUIDs.

### Open Findings

- `src/sources.rs` keeps `MetadataSource`, `ApiSource` and `LocalSource` with no caller outside the file (decision D13).
- A Library change after a feed page loads can add a matched key. That key then has no `MusicIndex` request until the page loads again. The section shows its Library rows.
- A Library search row and a `MusicIndex` search row of one feed can both show. Packet 010 owns the merge of the search rows.
- The search sends one `GET /v1/artist-names` check for each `MusicIndex` candidate name, one after the other (decision D15). A slow node makes the artist rows of a search slower. This is a responsiveness concern for the overhaul plan list.
- A `MusicIndex` name row can stay beside a `MusicIndex` feed row of the same name when the Library holds no feed of that name (decision D14).
- The agent session could not read the operator Library. V11 asks the operator to find a name that two artist feeds match.

## Operator Visual Check

Do each check in the Light theme and in the Dark theme. The checks need network access to `https://api.musicindex.org`. V8 needs two releases of one name that are not in the Library.

1. Build and open the app:

   ```bash
   cd /home/citizen/build/v4vmm
   cargo build --release
   ./target/release/v4vmm
   ```

2. V1: open Music, then Artists. Find the row "Haleen".
   - The list must show no row "Haleen · by name". The feed row "Haleen" must show "n releases". The n is the count of its feed releases and its name-only releases in the Library.
   - Open the row. The feed page must list its feed releases, then the section "Matched by name" last. The section must show the note that the app relates these releases to the artist by the name only.
   - Each row of that section must show `names "Haleen", no feed` and "In Library". Each release must show one time.
   - Wrong: a "Haleen · by name" row, a name-only release that shows on no page, or a row of the section that looks like a confirmed release.
3. V2 and V3: in Artists, open a row "<name> · by name".
   - The page title must read "Releases that name <name>". The header must show "Confirmation: Name only" and the note that a name is not an identity. The frame title must read "Artist name". The breadcrumb must read "Artists › <name> · by name".
   - The page must show "Albums and EPs" before "Singles". Each Library release must show "In Library", and each release must show one time. The page must show no aliases, no area, no active years and no source subjects.
   - The "In Library" count must equal the count of the Artists row.
   - Wrong: a missing Library release, an empty page for a name with Library releases, or an error report.
4. V4: search for a release that names no artist feed and no matched name. Open the release from the search results, then click its artist name.
   - The name level page must open and show the `MusicIndex` releases of the name. A release row must open its release page, and Back must return to the name level page.
   - Open a release that names "Haleen" and no feed, then click "Haleen". The Haleen feed page must open.
5. V5: search for a name with a name-only release and no matched feed.
   - The Artists tab must show the row "Releases that name <name>" with no count. The row must open the name level page, and a release row must open its release page.
6. V6: search for "Haleen".
   - The Artists tab must show no row "Releases that name Haleen". It must show the row "Haleen", and that row must open the Haleen feed page.
   - Search for a name with no name-only release. No name row may show, and no search may open an empty page.
7. V7: close the app. In `~/.config/v4vmm/config.toml`, set `musicindex_endpoint = "http://127.0.0.1:9"`. Open the app again with `./target/release/v4vmm`.
   - Open a name level page. The page must show the report first, and the Library releases must stay in their sections.
   - Open the Haleen feed page. The section "Matched by name" must show the same report, and its Library rows must stay.
   - Close the app and set `musicindex_endpoint = "https://api.musicindex.org"` again. Open the app. While `MusicIndex` answers, the name level page must show the loading text.
8. V8: open a name level page with two releases that are not in the Library. Click "Select", select the two releases, then click "Download 2 releases" on the action bar.
   - Each downloaded release must then show "In Library" in place. The "In Library" count must rise, and the app must not change the page.
9. V9: look at the header art of three name level pages.
   - A name with two or more covers must show the logo mosaic of four covers or fewer. A name with one cover must show that cover as a square. A name with no cover must show the artist color.
10. V10: do steps 2 to 6 at the normal window width, at a narrow width and at the larger type sizes.
    - Each element must stay in its place, with no clipped text. The section headings must be stronger than the row text.
    - The note and the detail line of "Matched by name" must be weaker than the row title.
11. V11: in Artists, find a row "<name> · by name" whose name is also the title of two artist feed rows. The agent session could not read the operator Library, so it names no such name.
    - The row and its name level page must stay. Neither feed page may show the releases of that name under "Matched by name".
    - If the Library holds no such name, record that in `docs/pending-human-checks.md`.

Cleanup:

- If V7 changed `~/.config/v4vmm/config.toml`, set `musicindex_endpoint = "https://api.musicindex.org"` again.
- V8 added two releases to the Library. To remove them, open the name level page, click "Select", select the two releases, then click "Remove" on the action bar.
