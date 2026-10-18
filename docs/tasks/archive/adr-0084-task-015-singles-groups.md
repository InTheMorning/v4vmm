# ADR 0084 Task 015: The Singles Groups

Status: Complete - 2026-10-17. The mechanical checks are Green. The operator passed V1 to V11 on 2026-10-17.

## Goal

In the Releases list, two or more Library singles of one artist identity show as one group row. The row opens in place to show each single. A group page lists the singles, newest first, with one main action. A single page links to its group, for example "All 7 singles".

## Authority

- [ADR 0084](../../adr/0084-music-column-browser.md) Decision 6: the group row, its count and its Library state, the group page, its main action, the single page link, and the group key.
- [ADR 0084](../../adr/0084-music-column-browser.md) Decision 7: the artist identity. A release that names its publisher feed has the publisher level. Otherwise it has the name level. The operator decided on 2026-10-08: when the agreed role of the link does not include artist, the release has the name level.
- [ADR 0084](../../adr/0084-music-column-browser.md) Decision 5: the kind of a release (`ReleaseKind`, task 007).
- [ADR 0082](../../adr/0082-publisher-roles-belong-to-each-album-link.md) Decision 2: `role` is the role set of one link, joined by `", "`. `role_agreement` is `both`, `one_side`, `conflict` or null.
- The orchestrator split the old packet 007 on 2026-10-17. This packet owns Decision 6 in the Library scope. Packet 016 owns the scope "Everything".

## Recorded Facts - 2026-10-17

- `LibraryViewModel::releases_list` (`src/view_models/library.rs`) gives one `DestinationListRowVm` for each Library release, sorted by the lowercase release name. It reads the loaded tree and does no query. Its secondary text is "artist · kind".
- The Library tree groups releases by artist text: `LibraryTrackRowVm::display_artist` gives the album artist, else the track artist, else "Unknown Artist". This text is not an identity.
- `AlbumNode::publisher_feed_guid` holds the publisher feed GUID that the release names (`owned_publisher_feed_guid` in `src/db/publisher_relationships.rs`). The tree loader in `src/application/queries/library.rs` does not read `role` or `role_agreement` of that link. Both columns are in `feed_publisher_relationships` (migration 18).
- No code computes the artist identity of Decision 7. Packet 008 builds the artist page on the identity type that this packet adds.
- Each `TrackRow` (`src/db.rs`) carries `pub_date: Option<i64>` and `is_in_library: bool`.
- `FrameNavigationEntry` (`src/view_models/workspace/nav.rs`) has no group entry. `LibraryDetail` (`src/library.rs`) holds the Library page data of each page kind.
- `AlbumNameLinkTarget` and `AlbumNameLinkVm` (`src/view_models/album_page.rs`) carry the "Publisher" link of a release page. `ReleaseDetailVm::name_links` (`src/view_models/entity_detail.rs`) builds them.
- `LibraryApp::download_feed` and `LibraryApp::add_album_to_playlist` (`src/library/app_impl.rs`) act on one feed id. No batch action exists. Packet 009 adds the selection model.
- `DisclosureIndicator` (`src/ui/composites/disclosure_group.rs`) is the shared chevron. No list row uses it yet.

## Design

### The artist identity

Add `src/view_models/artist_identity.rs`:

```rust
/// The artist identity of a release (ADR 0084 Decision 7).
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) enum ArtistIdentity {
    /// The publisher feed that the release names, by its feed GUID.
    Publisher(String),
    /// The artist text of a release that has no publisher level.
    Name(String),
}

impl ArtistIdentity {
    /// The publisher level when the release names a publisher feed, unless
    /// the agreed role of that link does not include artist. Otherwise the
    /// name level of `artist_text`.
    pub(crate) fn resolve(
        publisher_feed_guid: Option<&str>,
        link_role: Option<&str>,
        role_agreement: Option<&str>,
        artist_text: &str,
    ) -> Self;
}
```

- An empty or blank `publisher_feed_guid` counts as no publisher.
- The agreed role excludes artist when `role_agreement` is `both` and no item of `link_role`, split at `,` and trimmed, equals `artist` case-insensitively. A null `role` with `both` also excludes artist.
- Each other `role_agreement` value, for example `one_side`, `conflict` or null, keeps the publisher level.
- The name level keeps `artist_text` as it is. Two names that differ only in case are two identities. ADR 0084 Decision 6 says that two artists with the same name never share a group. A publisher identity and a name identity with the same text are different.

### Load the link role

- Add `owned_publisher_link(conn, feed_id) -> Result<Option<OwnedPublisherLink>>` in `src/db/publisher_relationships.rs`. It reads the same row as `owned_publisher_feed_guid`, and gives `publisher_feed_guid`, `role` and `role_agreement`. Use the same `WHERE` clause and the same order.
- The tree loader uses the new function in place of `owned_publisher_feed_guid`. Delete `owned_publisher_feed_guid` if no caller remains.
- Add `AlbumNode::publisher_link_role: Option<String>` and `AlbumNode::publisher_role_agreement: Option<String>`.
- Add `AlbumNode::artist_identity(&self, artist_text: &str) -> ArtistIdentity`. The caller gives the name of the `ArtistNode` that holds the album.

This adds a read of two columns that exist. It adds no schema change and no migration.

### The singles groups

Add one pure function in `src/view_models/library.rs` (or a new module `src/view_models/singles_group.rs`):

```rust
pub(crate) fn singles_groups(tree: &LibraryTree) -> Vec<SinglesGroup>;
```

- A single is an `AlbumNode` whose `release_kind()` is `ReleaseKind::Single` and that has a `release_key()`.
- The function groups the singles by `ArtistIdentity`. It keeps each group with two or more singles. One single makes no group.
- A `SinglesGroup` holds its `ArtistIdentity`, its display artist and its singles, newest first.
- The date of a single is the latest `pub_date` of its tracks. A single with no date comes after each single with a date. Equal dates sort by the lowercase release name.
- The display artist is the `ArtistNode` name of the newest single.
- A single is in the Library when each of its tracks has `is_in_library`. The group counts these singles.
- `releases_list`, the group page and the single page link each use this one function. No other code groups singles.

### The Releases list

- `releases_list` gives `Vec<ReleasesListEntryVm>`:
  - `Release(DestinationListRowVm)` for each release that is not in a group.
  - `SinglesGroup(SinglesGroupRowVm)` for each group.
- Sort the entries by one key: the lowercase label. The label of a release row is the release name. The label of a group row is the display artist.
- `SinglesGroupRowVm` carries:
  - `element_id`, from the group identity, with the publisher GUID or the name text in it.
  - `label`: the display artist, for example "Haleen".
  - `secondary_text`: "7 singles · 5 in Library · By name". When each single is in the Library, the text is "7 singles · In Library · By name". The last item is the identity level: "By name" or "Publisher feed". The separator comes from the view model.
  - `a11y_label`, for example "Haleen, 7 singles, 5 in Library, By name".
  - `thumbnail_href`: the image of the newest single.
  - `open: SinglesGroupKey`, the key of the group page.
  - `expanded: bool` and `disclosure_a11y_label`: "Show singles" or "Hide singles".
  - `singles: Vec<DestinationListRowVm>`: the release row of each single, newest first, when `expanded` is true. Empty when it is false.
- The group key is `SinglesGroupKey(ArtistIdentity)`. It holds no label.
- `LibraryApp` holds the expanded groups as a set of `SinglesGroupKey`. `releases_list` takes this set. The set is view state. It is not stored.

### The group row

- The chevron (`DisclosureIndicator`) toggles the row in place. It does not navigate.
- A click on the row body opens the group page.
- When the row is open, each single shows as an indented release row below it. A click opens the single page, as a release row does today.
- The row has no checkbox. Packet 009 adds the checkbox and the selection model.

### The group page

- Add `FrameNavigationEntry::SinglesGroup(SinglesGroupKey)`. Its fallback label is "Singles". `opens_detail_page` is true.
- Add one `LibraryDetail` variant for the group page. The page reads the current tree at each render through `singles_groups`, so a download or a removal updates it in place.
- Add `SinglesGroupPageVm` in `src/view_models/`, built from one `SinglesGroup`:
  - `title`: "Singles by Haleen". The breadcrumb label is the same text.
  - `subtitle`: "7 singles · 5 in Library", the same text as the row.
  - `identity_note`: at the name level, "Grouped by the artist name. A name is not an identity." At the publisher level, "Grouped by the publisher feed that each single names."
  - `rows`: one row for each single, newest first: the release name, the date text when one exists, the state word "In Library" or "Not downloaded", and the release key.
  - `main_action`: one typed action with availability, a label and an accessibility label.
    - When one or more singles are not in the Library: "Download 2 singles", accessibility label "Download the 2 singles that are not in the Library". The count is the number of singles not in the Library. One single gives "Download 1 single".
    - Else: "Add to playlist", accessibility label "Add the 7 singles to a playlist".
- When no group has the key, for example after a removal leaves one single, the page shows an empty state: "This artist has fewer than two singles in the Library." It has no action.
- "Download n singles" calls `download_feed` for each single that is not in the Library, newest first.
- "Add to playlist" opens the shared playlist popover. The choice adds each single, newest first, through `add_album_to_playlist`. "Create playlist" adds them through the same path.
- A row click opens the single page.

### The single page link

- Add `AlbumNameLinkTarget::SinglesGroup(SinglesGroupKey)`.
- `ReleaseDetailVm::name_links`, or the Library release page link builder, adds the link "All 7 singles" after the existing links. It adds the link only when the release is a single in a Library group. The accessibility label is "Open all 7 singles by Haleen".
- The link shows on a release page of each origin when the release key is in a Library group. An Index release that is not in the Library gets no link. Packet 016 adds the Index singles.
- A click pushes `FrameNavigationEntry::SinglesGroup` into the current path.

## Required Changes

1. Add `ArtistIdentity` and its tests.
2. Add `owned_publisher_link`, the two `AlbumNode` fields and `AlbumNode::artist_identity`.
3. Add `singles_groups`, `SinglesGroup` and `SinglesGroupKey`.
4. Change `releases_list` to give `ReleasesListEntryVm`. Change its renderer in `src/ui/shells/library/destination_lists.rs` to draw the group row with the chevron and the open singles.
5. Add the navigation entry, the `LibraryDetail` variant, `SinglesGroupPageVm`, its renderer and its actions.
6. Add the single page link and its click.
7. Add the guard of R15-09. Update each test that asserts the old `releases_list` type.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_singles_group_` for tests beside the owning code.

| Case | Required proof |
|---|---|
| R15-01 | `ArtistIdentity::resolve`: a publisher GUID with a null agreement, `one_side` or `conflict` gives `Publisher`. `both` with "artist, producer" gives `Publisher`. `both` with "label" gives `Name`. `both` with a null role gives `Name`. No GUID or a blank GUID gives `Name` |
| R15-02 | `owned_publisher_link` reads `role` and `role_agreement` of the same row that `owned_publisher_feed_guid` read, in a database test |
| R15-03 | `singles_groups`: two singles of one name give one group. One single gives no group. Two singles of "Haleen" and "haleen" give no group. A single with a publisher identity and a single with the same name text give no group. An album and an EP never join a group |
| R15-04 | `singles_groups` orders the singles newest first, with a single with no date last |
| R15-05 | `releases_list` gives one group entry and no release entry for each single in a group, with the label "Haleen" and the secondary text "3 singles · 2 in Library". Each single in the Library gives "3 singles · In Library" |
| R15-06 | `releases_list` with the group key in the expanded set gives the singles rows newest first and `disclosure_a11y_label` "Hide singles". Without the key, it gives no singles rows and "Show singles" |
| R15-07 | `SinglesGroupPageVm`: one single not in the Library gives "Download 1 single". Two give "Download 2 singles". None gives "Add to playlist". Each action is available and has its accessibility label. Each level gives its identity note, and the subtitle names the level |
| R15-08 | The release page name links of a single in a group of 7 include "All 7 singles" with the group key. A single in no group and an album give no such link |
| R15-09 | A guard names ADR 0084 Decision 6: no code outside the owner of `singles_groups` groups releases by `ReleaseKind::Single` |
| R15-10 | `FrameNavigationEntry::SinglesGroup` opens a detail page, and its fallback label is "Singles" |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: the Releases list shows one group row for an artist with two or more Library singles. The row shows the artist, the count and the Library state.
- V2: the chevron opens the row in place and shows each single, newest first. The chevron closes it again. Neither click changes the page in the detail column.
- V3: a click on the group row body opens the group page. The breadcrumb shows "Singles by" and the artist.
- V4: the group page lists the singles newest first, with a state word on each. Its main action is "Add to playlist" when each single is in the Library.
- V5: a single page in a group shows "All n singles", and the link opens the group page.
- V6: a single that is the only single of its artist shows as a normal release row.
- V7: Light and Dark, and a larger scale, show no clipped row, chevron, link or action.

## Exclusions

- No scope control and no Index singles. Packet 016 owns them.
- No checkbox and no selection. Packet 009 owns them.
- No artist page change. The order "Albums and EPs" before "Singles" on the artist page waits for packet 008.
- No schema change and no migration.
- No new MusicIndex request.
- No change to an internal name, such as `AlbumNode` or `DownloadAlbum`.

## Files To Inspect

- [Agent rules](../../../AGENTS.md), the [source map](../../architecture/source-map.md) and the [phase plan](../../plans/adr-0084-music-column-browser-phase-plan.md).
- `src/view_models/release_kind.rs`, `src/view_models/library.rs` (`AlbumNode`, `ArtistNode`, `LibraryTree`, `releases_list`, `DestinationListRowVm`, `DestinationListRowTarget`).
- `src/application/queries/library.rs` (the tree loader), `src/db/publisher_relationships.rs`, `src/db.rs` (`TrackRow`).
- `src/view_models/workspace/nav.rs`, `src/view_models/workspace/breadcrumb.rs`, `src/app.rs` (`content_list_frame_title` and the detail render switch), `src/app/breadcrumb.rs`.
- `src/library.rs` (`LibraryDetail`), `src/library/app_impl.rs` (`open_destination_row`, `open_page`, `download_feed`, `add_album_to_playlist`, `create_playlist_and_add_album`, `open_album_name_link`).
- `src/view_models/album_page.rs`, `src/view_models/entity_detail.rs` (`name_links`), `src/app/search_dispatch.rs` (`open_index_album_name_link`).
- `src/ui/shells/library/destination_lists.rs`, `src/ui/shells/library/feed_detail.rs`, `src/ui/shells/entity.rs` (`render_album_name_links`), `src/ui/composites/disclosure_group.rs`, `src/ui/composites/playlist_popover.rs`.
- `tests/architecture_tests.rs`.

## Checks

```bash
cargo test adr_0084_singles_group_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

Run `cargo build --bin v4vmm` last, after every test run. A test run can leave a GPUI test-support binary at `target/debug/v4vmm` (AGENTS.md, "Build, Test, Lint").

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md`, `docs/architecture/source-map.md` and `docs/plans/adr-0084-music-column-browser-phase-plan.md`
- This packet: `docs/tasks/adr-0084-task-015-singles-groups.md`
- ADR 0084 Decisions 4 to 7, and ADR 0082 Decision 2
- Each file in "Files To Inspect"

Goal:
- The artist identity, the singles groups, the group row, the group page and the single page link, as "Design" gives.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- View models carry no GPUI type. Each label, count text, separator and accessibility label is a view-model value. The renderer decides no text and no availability.
- Each size, space, color and icon comes from a named token. Column text does not call `truncate()` (ADR 0063).
- A navigation entry and a group key hold no label.
- Never call `cx.spawn` from a screen.
- Do not rename an internal type, function or action variant.
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git add` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The scroll helpers of task 014 and the description display of task 013.
- The Discover views, the search results view and its chips.
- The artist page, the publisher page and the name match page.
- The Settings and Show screens.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R15-01 to R15-10 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result" section and an "Operator Visual Check" section for V1 to V7. The check gives numbered steps, the expected result, an incorrect result, and the cleanup. It tells the operator how to find an artist with two or more Library singles. If the operator's Library has none, it gives the steps to download two singles of one artist from Discover, and the cleanup that removes them.

Test commands:
- Each command in "Checks".

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- The tree loader cannot give the role of the link without a new table or a schema change.
- The group page needs Index data.
- An action that the group page needs has no existing per-release command in `LibraryApp`.
- A change needs a file in "Do not touch".
- A guard of an earlier packet asserts the old `releases_list` type and names a decision that this packet does not amend.

## Implementation Result - 2026-10-17

Each Required Change is done. No stop trigger applied.

### Files Changed

- `src/view_models/artist_identity.rs` (new file): `ArtistIdentity` and `ArtistIdentity::resolve`, with the R15-01 tests.
- `src/view_models/singles_group.rs` (new file): `SinglesGroupKey`, `SinglesGroup`, `singles_groups`, `SinglesGroupRowVm`, `SinglesGroupPageVm`, the main action, `singles_group_name_link` and `release_page_name_links`, with the R15-03, R15-04, R15-07 and R15-08 tests.
- `src/view_models/mod.rs`: registers the two new modules.
- `src/db/publisher_relationships.rs`: adds `OwnedPublisherLink` and `owned_publisher_link`, with the R15-02 test. `owned_publisher_feed_guid` stays, because `src/sources.rs` still calls it.
- `src/application/queries/library.rs`: the tree loader reads `owned_publisher_link` and fills the two new `AlbumNode` fields.
- `src/view_models/library.rs`: adds `AlbumNode::publisher_link_role`, `AlbumNode::publisher_role_agreement` and `AlbumNode::artist_identity`. Adds `ReleasesListEntryVm` and `destination_release_row`. `releases_list` takes the expanded set and gives `Vec<ReleasesListEntryVm>`. Adds `release_page_name_links` and `singles_group_page`. Updates the two old `releases_list` tests and adds the R15-05 and R15-06 tests.
- `src/view_models/album_page.rs`: adds `AlbumNameLinkTarget::SinglesGroup`.
- `src/view_models/workspace/nav.rs`, `breadcrumb.rs` and `tests.rs`: add `FrameNavigationEntry::SinglesGroup`, its fallback label "Singles", its breadcrumb id and the R15-10 test.
- `src/library.rs`: adds `LibraryDetail::SinglesGroup`, the expanded set and the download queue of `LibraryApp`.
- `src/library/app_impl.rs`: opens and toggles a group, and mounts the group page from the navigation entry. It also gives the breadcrumb label, opens the group from a single page link, and runs the main action.
- `src/ui/shells/library/destination_lists.rs`: adds `render_library_releases_list`, the group row with the chevron, and the indented single rows.
- `src/ui/shells/library/singles_group.rs` (new file) and `mod.rs`: the group page renderer.
- `src/ui/shells/library/detail.rs`: renders the group page.
- `src/ui/shells/library/feed_detail.rs` and `src/app/search_dispatch.rs`: add the group link to a release page, and open the group page from the link.
- `src/app.rs` and `src/app/breadcrumb.rs`: the frame title "Singles" and the breadcrumb label from the Library.
- `tests/architecture_tests.rs`: adds the R15-09 guard `adr_0084_singles_group_r15_09_only_the_owner_groups_singles`.

### Behavior Changed

- In the Releases list, two or more Library singles of one artist identity show as one group row. The row shows the display artist, the count and the Library state. A single that is the only single of its identity stays a normal release row.
- The chevron of a group row opens and closes the row in place. An open row shows each single below it, indented and newest first.
- A click on the group row body opens the group page "Singles by" and the artist. The page lists the singles newest first, with a state word on each, and has one main action.
- A single page in a Library group shows the link "All n singles". The link opens the group page in the current path.

### Checks

| Command | Result |
|---|---|
| `cargo test adr_0084_singles_group_` | Green: 13 unit tests, 1 guard |
| `cargo test` | Green: 1,895 unit tests, 316 guards, 10 ignored doc tests |
| `cargo test --test architecture_tests` | Green, 316 tests |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

### Deviations From The Task

1. `download_feed` does not start while another download runs. Thus a loop that calls it for each single starts only the first download. `LibraryApp` now holds a queue of feed ids. "Download n singles" puts the feed ids in the queue, newest first. When one download ends, with success or with failure, the next one starts through `download_feed`.
2. The playlist commands run at the same time, so one append for each single can add the singles in a random order. `add_album_to_playlist` now calls the new `add_albums_to_playlist` with one feed id. The group page calls `add_albums_to_playlist` with each feed id, newest first, and this gives one append in that order. `create_playlist_and_add_album` calls the new `create_playlist_and_add_albums` the same way. Thus the album page and the group page use the same path.
3. `ArtistIdentity` and `SinglesGroupKey` are `pub`, not `pub(crate)`, and their modules are `pub mod`. `AlbumNameLinkTarget` is a `pub` enum, and with a `pub(crate)` key the lint `private_interfaces` fails `cargo check`.
4. The two new `AlbumNode` fields made `LibraryDetail::Album` too large for the lint `clippy::large_enum_variant`. The enum now has `#[expect(clippy::large_enum_variant, reason = ...)]`. A change to `Box<AlbumNode>` would touch each user of `LibraryDetail::Album`.
5. The R15-08 test proves the link at `release_page_name_links`, the link builder that the Library release page and the Index release page both call. `ReleaseDetailVm::name_links` does not change.
6. This packet does not record the open V1 to V7 gate in `docs/pending-human-checks.md` or in `docs/plans/broadcast-chain-delivery-order.md`. This packet names only its own file as an editable document.

### Unresolved Concerns

- The two files of Deviation 6 still need the open V1 to V7 gate.
- No person walked V1 to V7. A green mechanical suite gives no evidence about them.
- `src/ui/shells/library/destination_lists.rs` has 497 code lines. The guard `surface_modules_under_500_loc` (ADR 0038 Task 007) stops at 500. The next change to this file must move code to a sibling file.
- The Releases list and the breadcrumb compute `singles_groups` again at each render. The computation reads only the loaded tree. If a large Library makes the list slow, add the concern to the overhaul plan.

## Review Fix - 2026-10-17

The singles download queue started only at the end of a feed download, so a track download that ran at the click stopped the queue. Now each end of a busy track or a busy feed in `src/library/app_impl.rs` starts the next queued download.

Operator finding, 2026-10-17: the main button of the group page showed in the center, below the header. The page now puts the button in the shared `ActionRow`, as a release page does, so it aligns left. `SinglesGroupActionVm::row_a11y_label` gives the label of the row.

Operator finding, 2026-10-17: the Releases list showed two "Haleen" rows that looked the same. The operator's Library query showed the cause. Three Haleen singles name the publisher feed `64569161-a8f7-52e5-8aae-9e0385043149` with no stated role, and the other singles name no publisher feed. Decision 7 keeps these two identities apart, so the two groups are correct. Each group row and each group page now names its identity level: "By name" or "Publisher feed". The publisher-level page also has a note.

## Operator Visual Check

This check needs a Linux desktop session, this checkout, and a reachable MusicIndex endpoint. Steps
13 to 17 download two singles only when your Library has no artist with two or more singles.

1. Close v4vmm. Build the binary.

   ```bash
   cd /home/citizen/build/v4vmm
   cargo build --bin v4vmm
   ```
2. Open the app.

   ```bash
   target/debug/v4vmm
   ```
3. Click Music, then click Releases in the sidebar.
4. Find a row with a chevron on its left and the text "n singles" below the artist name. This row is
   a singles group. If no row has a chevron, do steps 13 to 17 first, then continue at step 5.

**V1 - one group row for an artist with two or more Library singles**

5. Look at the group row.
   - Correct: the row shows the artist name, then a text such as "3 singles · 2 in Library · By
     name" or "3 singles · In Library · Publisher feed". No single of that artist shows as a separate
     release row. One artist can have two rows, one "By name" and one "Publisher feed", when only
     some of the singles name the publisher feed.
   - Incorrect: the singles show as separate rows, the row has no count, no Library state or no
     level, or two rows of one artist show the same level.

**V2 - the chevron opens and closes the row in place**

6. Note the page in the detail column. Click the chevron of the group row.
   - Correct: each single shows below the row, indented, newest first. The chevron points down. The
     detail column shows the same page as before.
   - Incorrect: no single shows, the order is not newest first, or the detail column changes.
7. Click the chevron again.
   - Correct: the singles go away and the chevron points right. The detail column does not change.
   - Incorrect: the singles stay, or the detail column changes.

**V3 - the row body opens the group page**

8. Click the artist name of the group row.
   - Correct: the detail column shows the group page. The breadcrumb shows "Singles by" and the
     artist name.
   - Incorrect: the row only opens or closes, or the breadcrumb shows "Singles" alone.

**V4 - the group page**

9. Look at the group page.
   - Correct: the singles show newest first. Each row shows "In Library" or "Not downloaded". When
     each single is in the Library, the main button is "Add to playlist". Else the button is
     "Download n singles". A group by name, not by publisher feed, also shows "Grouped by the artist
     name. A name is not an identity."
   - Incorrect: the order is not newest first, a row has no state word, or the button does not
     agree with the state words.
   - Correct: the main button aligns left, below the header text, as on a release page.
   - Incorrect: the button shows in the center of the column.
10. If the button is "Add to playlist", click it.
    - Correct: the shared playlist popover opens. Press Escape to close it without a choice.
    - Incorrect: no popover opens.

**V5 - the single page links to its group**

11. Click one single row on the group page.
    - Correct: the single page opens. The links below its title include "All n singles". A click
      on that link opens the group page again.
    - Incorrect: the link is absent, or it opens another page.

**V6 - a single with no other single of its artist**

12. Go back to Releases. Find a release row whose text ends with "· Single" and that has no
    chevron.
    - Correct: that release shows as a normal release row, with the artist and "Single".
    - Incorrect: that release shows as a group row with "1 singles".

**Setup when no group exists (steps 13 to 17)**

13. Click the toolbar search, type the name of an artist that has two or more single releases on
    MusicIndex, and press Enter.
14. Open the first single of that artist from the results. Click "Download single". Wait until
    its tracks show "In Library".
15. Go back to the results. Open a second single of the same artist and click "Download single".
    Wait until its tracks show "In Library".
16. Click Releases in the sidebar.
    - Correct: one group row for that artist, with "2 singles · In Library".
    - Incorrect: two separate release rows. If the two singles name different publisher feeds, or
      show two different artist texts, they are two identities. Pick another artist.
17. Write down the names of the two singles for the cleanup. Continue at step 5.

**V7 - Light, Dark and a larger scale**

18. Open Settings, then General (`Ctrl+Comma`). Select Dark, then Light, then a larger scale.
    Return to Music after each change. Look at the group row open and closed, the group page and
    the "All n singles" link.
    - Correct: no row, chevron, link or button is clipped, and each text is readable.
    - Incorrect: a part is clipped, or a text overlaps another element.
19. Close the app.

**Clean-up**

- Set the theme and the scale of V7 back to your usual values.
- If you did steps 13 to 17, remove the two singles. Open the page of each single that you wrote
  down in step 17. Open the "⋯" menu, click "Remove single…", and confirm.
- The open or closed state of a group row is not stored. It needs no cleanup.

## Orchestrator Review - 2026-10-17

The orchestrator reviewed the diff against ADR 0084 Decisions 6 and 7, this packet and the [review checklist](../../reviews/adr-0084-review-checklist.md). The orchestrator also ran each check.

- `ArtistIdentity`, `singles_groups`, the group row, the group page and the single page link follow "Design". One function groups the singles, and the R15-09 guard keeps it the only one.
- The group key and the navigation entry hold no label. The view models hold no GPUI type. The renderers use the existing label and token patterns.
- Deviation 1, the download queue, is necessary, because `download_feed` starts one download at a time. The first version started the queue only at the end of a feed download. The orchestrator returned it to the implementer, and the "Review Fix" section records the correction.
- Deviation 2 keeps the playlist order. The existing one-album functions now call the new functions, so each caller keeps its behavior.
- Deviations 3 and 4 prevent compiler and clippy failures. They change no behavior.
- `src/ui/shells/library/destination_lists.rs` has 497 code lines. The next change to it must move code to a sibling file.
- The orchestrator recorded the open gate in the pending checks, the delivery order, the phase plan and `AGENTS.md`.

Merge recommendation: pass. The visual gate V1 to V7 stays open. Packet 016 waits for Stophammer request 6. Packet 008 comes next, after the visual check of this packet, and it uses `ArtistIdentity`.

## Operator Check - 2026-10-17

The operator walked V1 to V4 and reported two findings: the main button showed in the center, and two "Haleen" rows looked the same. The "Review Fix" section records the fix of each one. The operator then passed V5 to V7.

After the operator passed V5 to V7, the operator reported two more findings on 2026-10-17:

- Each single row on the group page had no release art, unlike the release rows of the app.
- The detail column header showed "Singles". The operator decided that this header shows the page title, for example "Singles by Haleen". Each other page keeps its type label.

V8 and V9 check the fixes:

1. Open the new binary: `target/debug/v4vmm`. Open Music, then Releases, then a singles group.
2. **V8:** look at the rows of the group page.
   - Correct: each single row shows its release art on the left, at the same size as a Releases row.
   - Incorrect: a row has no image, or the image has a different size.
3. **V9:** look at the header of the detail column.
   - Correct: it says "Singles by" and the artist name, the same text as the page title.
   - Incorrect: it says "Singles" alone.

The operator made four more decisions on 2026-10-17. Each single row on the group page is the
Library track row of a release page. `render_library_track_row` in
`src/ui/shells/library/feed_detail.rs` draws it with the track of the single. The row shows the art,
the title, the date, the duration and the same actions. A click on the row opens the release page of
the single.

The shared `TrackRow` composite shows an optional date text as a second line below the title. The
view model gives this text through `fmt_date`. Thus the Library and the Index release pages also
show a date below each track title that has one. The group page row shows the date of its single.
It shows no state word, because the toggle action shows the state.

V10 and V11 check this change:

4. **V10:** open Music, then Releases, then a singles group. Look at the rows of the group page.
   - Correct: each row looks like a track row of a release page. It shows the art, the title, the
     date below the title, the duration and the actions. A click on the row body opens the single
     page.
   - Incorrect: a row looks different from a release page track row. Or a row has no date when its
     single has one. Or a click opens a track page.
5. **V11:** open a release page from Releases. Then open an Index release page from the toolbar
   search.
   - Correct: a date shows below each track title that has one. A track with no date shows only
     its title.
   - Incorrect: a track with a date shows no date, or an empty second line shows.

The operator passed V10 and V11 on 2026-10-17.
