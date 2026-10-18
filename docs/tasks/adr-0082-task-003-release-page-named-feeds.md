# ADR 0082 Task 003: The Named Feeds Of A Release

Status: Ready - 2026-10-18. The orchestrator rewrote this packet for [ADR 0085](../adr/0085-artists-by-confirmation.md) Decisions 5 and 6 on 2026-10-18. Implementation has not started.
This packet adds migration 19. The visual gate opens when the implementation is complete.

## Goal

The app keeps the named feeds of each album in their RSS sequence. It selects the artist feed of the album by ADR 0085 Decision 6.
A release page names its artist, with a link to the page of the artist feed. It shows a role row only for a feed with a different role, and an "Also named" row for each other named feed.
No release page and no track page shows the word "Publisher".

## Authority

- [ADR 0085](../adr/0085-artists-by-confirmation.md) Decision 5: the release page names its artist, and shows a role row only for a different role.
- [ADR 0085](../adr/0085-artists-by-confirmation.md) Decision 6: the named feeds keep the RSS sequence. The first named feed with an agreed `artist` role is the artist feed. An observation time never selects a named feed.
- [ADR 0085](../adr/0085-artists-by-confirmation.md) Decision 3: the role words come from the feeds, in alphabetical sequence. This packet adds their text owner, and packet 008 uses it.
- [ADR 0085](../adr/0085-artists-by-confirmation.md) Non-Goals: the position of each named feed is the only storage change. It uses the migration registry of [ADR 0016](../adr/0016-schema-migration-registry.md).
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 7: the artist name opens the feed level of the artist feed. When the release has no artist feed, it opens the name level.
- [ADR 0077](../adr/0077-publisher-feed-artist-binding.md) Decision 2: an album binds to the feed that it names. Decision 6: `publisher_text` is the feed owner, and no link takes its label or target from it.
- The durable set in [AGENTS.md](../../AGENTS.md): renderer portability, element hierarchy, typed action state and token discipline.

## Recorded Facts - 2026-10-18

- `owned_publisher_feed_guid` and `owned_publisher_link` in `src/db/publisher_relationships.rs` read one `music_to_publisher` row with `music_names_publisher = 1`. They sort by `observed_at DESC`. Thus with two named feeds, the result does not follow the RSS.
- Callers of these two functions: `src/sources.rs:132` (the Library `FeedView`), `src/application/queries/library.rs:404` (the Library tree) and `src/library/app_impl.rs:2007`.
- `owned_publisher_feed_guid_from_relationships` in `src/views.rs` gives the first response entry with `music_names_publisher` true. It does not read a position. `FeedView` and the track view both use it (`src/views.rs:469` and `src/views.rs:660`).
- `api::RemoteItem` (`src/api.rs:420`) decodes `position`, `medium`, `remote_feed_guid`, `remote_feed_url` and `source`. The album request includes `remote_items` (`FEED_INCLUDE` in `src/application/commands/feed.rs:991`). No code stores a position.
- `api::PublisherRelationship` decodes `remote_feed_title` for each entry. The table `feed_publisher_relationships` has no column for it. Its `publisher_feed_title` column gets one value for each album response: Stophammer gives the title of the primary party. Thus a stored title is correct only for the first named feed.
- The table also stores `role`, `role_agreement`, `album_names_as`, `publisher_rel` and `music_rel` for each row (migration 18, `src/db/publisher_link_facts.rs`).
- `ArtistIdentity::resolve` (`src/view_models/artist_identity.rs`) takes one GUID, one role and one agreement. It gives the name level when both sides agree on a role set without `artist` (operator decision of 2026-10-08). The singles groups use it through `AlbumNode::artist_identity`.
- `ReleaseDetailVm::name_links` (`src/view_models/entity_detail.rs:880`) gives the artist link (`AlbumNameLinkTarget::Artist`, by name) and a "Publisher" link (`PUBLISHER_LINK_LABEL`, accessibility label "Open the publisher of this album").
- `TrackDetailVm::name_links` (`src/view_models/track_detail.rs:405`) gives the album link and a "Publisher" link (accessibility label "Open the publisher of this track"). The track page has no artist link.
- `set_music_to_publisher_remote` (`src/db/publisher_relationships.rs`) writes the one `podcast:publisher` remote item of a checked RSS document (ADR 0076 packet 002).
- Schema version 18 is current.
- Stophammer release 0.11.0, deployed on 2026-10-18, gives `album_names_rank` on each publisher entry, in the two directions. Packet 008 decodes it. This packet keeps the position of `remote_items`, as ADR 0085 states.
- The release record of the [open requests](../plans/v4vmm-open-requests.md#release-0110---2026-10-18) says that 6 albums name two feeds. Each one names the artist first and Sir Libre Records second. These albums are the candidates for V1.

## Design

### Read of Decision 6 for an agreed role without `artist`

Decision 6 says that the primary named feed is the artist feed when no named feed has an agreed `artist` role.
Decision 2 says that a feed with an agreed role set without `artist` is not in Artists. The operator decision of 2026-10-08 gives such a release the name level.
This packet applies the three together:

1. The artist feed is the first named feed whose link has `role_agreement` `both` and a role set that contains `artist`. The release has a **confirmed** artist feed.
2. Else, the primary named feed is the artist feed when its link does not have `role_agreement` `both`. The release has an **unverified** artist feed.
3. Else, the release has no artist feed. Its artist name opens the name level.

The packet adds no other rule. A count, a name match and an observation time never select the artist feed.
The operator accepted this read on 2026-10-18.

### The position of each named feed

- Migration 19 adds `named_feed_position INTEGER NULL` to `feed_publisher_relationships`. Existing rows get null. Add it through the ADR 0016 registry, with its schema contract and its upgrade fixtures. The schema version becomes 19.
- The album hydration writes the position of each `music_to_publisher` row. The position is the `position` of the `remote_items` entry with `source` `podcast_publisher` and the same `remote_feed_guid`. No match gives null.
- The RSS check path (`set_music_to_publisher_remote`) keeps the position of the row that it changes. A new row that it writes gets null. The next hydration writes the position.

### The named feeds

Add `src/view_models/named_feeds.rs`. It is pure and has no renderer type.

```rust
/// One feed that an album names, in its RSS sequence (ADR 0085 Decision 6).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NamedFeed {
    pub(crate) feed_guid: String,
    /// The title of the feed, when the app knows it.
    pub(crate) title: Option<String>,
    pub(crate) position: Option<i64>,
    /// The role set of the link, joined by `", "`.
    pub(crate) role: Option<String>,
    /// `both`, `one_side`, `conflict`, or `None`.
    pub(crate) role_agreement: Option<String>,
    /// The raw `rel` of the feed side, and of the album side.
    pub(crate) feed_rel: Option<String>,
    pub(crate) album_rel: Option<String>,
}

/// The artist feed of an album (ADR 0085 Decision 6, read above).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ArtistFeed {
    Confirmed(usize),
    Unverified(usize),
    None,
}

pub(crate) fn sort_named_feeds(feeds: &mut [NamedFeed]);
pub(crate) fn select_artist_feed(feeds: &[NamedFeed]) -> ArtistFeed;
```

- `sort_named_feeds` sorts by position. A null position comes after each known position. Equal positions sort by `feed_guid`. No other value changes the sequence.
- The index of `ArtistFeed` points into the sorted list.
- The Library builds the list with a new reader `named_feeds(conn, feed_id)` in `src/db/publisher_relationships.rs`. It reads each `music_to_publisher` row with `music_names_publisher = 1`, in the sequence of `sort_named_feeds`.
- The Library title of the first named feed is the stored `publisher_feed_title`. Each other named feed gets the title of its Library feed, when the Library has a feed with that GUID. Else its title is null.
- The Index builds the list from the response: each `music_to_publisher` entry with `music_names_publisher` true. The title is `remote_feed_title`. The position comes from `remote_items`, by the rule of the hydration.
- Delete `owned_publisher_feed_guid`, `owned_publisher_link`, `OwnedPublisherLink` and `owned_publisher_feed_guid_from_relationships`. Each caller uses the named feeds.

### The artist identity

- `ArtistIdentity::resolve` takes the named feeds and the artist text. A confirmed or unverified artist feed gives `ArtistIdentity::Publisher(feed_guid)`. No artist feed gives `ArtistIdentity::Name(artist_text)`.
- `AlbumNode` holds `named_feeds: Vec<NamedFeed>` in place of `publisher_feed_guid`, `publisher_link_role` and `publisher_role_agreement`.
- Update the R15-01 tests for the new input. Keep each case that they prove. The singles groups keep their behavior for an album with one named feed.

### The role words

Add `src/view_models/role_words.rs`. Packet 008 uses it for the "As <roles>" headings and the "Role not confirmed" rows.

- `role_words(role_set)` splits at `,`, trims each item and drops each empty item. It keeps the case of the feed. It removes an exact duplicate. It sorts by the lowercase word, then by the word.
- `role_list_text(words)` joins the words: "label", "label and producer", "distributor, label and producer".
- `capitalize_first(text)` writes the first letter with a capital letter: "Label and producer".
- `stated_sides_text(feed_title, feed_rel, album_rel)` gives the text of each side, by title. The side that states a role comes first. When both sides state a role, the feed side comes first. A side with no role gives "no role". Examples: "Sir Libre Records: label · the album: recordLabel" and "The album: label · Sir Libre Records: no role".
- "label" and "recordLabel" are two different words. The module keeps no list of role words.
- The separator " · " and the word "the album" are named constants in this module.

### The release page

`ReleaseDetailVm` exposes the named feed facts of Decisions 5 and 6. The Library page and the Index page use the same model.

- The artist link keeps the artist name as its label. Its target is the page of the artist feed, when one exists. Else its target is the name level, as today.
- With an unverified artist feed, the view model exposes the note "Unverified artist" with the artist link. The accessibility label of the link is "Open the artist page of <name>, unverified artist".
- With a confirmed artist feed, the link has no note. The accessibility label is "Open the artist page of <name>".
- Each other named feed gives one row, in the named feed sequence:
  - Agreed role set (`role_agreement` `both`): a role row. The label is the role list text with a capital letter, for example "Label" or "Label and producer". The value is the feed title. The row opens the page of the feed.
  - `one_side` or `conflict`: a "Role not confirmed" row. The value is the feed title. The detail is the stated sides text. The row opens the page of the feed.
  - No stated agreement: an "Also named" row with the feed title. The row opens the page of the feed.
- A release with no artist feed shows its first named feed by the same rule. For example, an agreed `label` link gives "Label: Sir Libre Records".
- The artist feed itself gives no role row, also when its agreed role set has other roles than `artist`.
- A feed with no title shows the feed GUID through the present `TitleDisplay` rule. The view model invents no title.
- Delete the "Publisher" link, `PUBLISHER_LINK_LABEL` and the accessibility label "Open the publisher of this album".
- The "Feed owner" row of `publisher_text` stays. It opens nothing.
- The view model owns each label as a named constant. The screen composes the rows with the present header row composite and the present link style.

### The track page

- Delete the "Publisher" link and the accessibility label "Open the publisher of this track".
- When the release of the track has an artist feed, the track page has one link to its page. The label is the feed title. The accessibility label is "Open the artist page of <title>".
  With an unverified artist feed, the link has the note "Unverified artist".
- The track page shows no role row and no "Also named" row. The release page shows them.
- This applies Decisions 1 and 6 to the track page. ADR 0085 names the release page only. The operator accepted this change on 2026-10-18.

## Required Changes

1. Add migration 19, its schema contract and its upgrade fixtures. The schema version becomes 19.
2. Write the position in the hydration path. Keep it in the RSS check path.
3. Add `named_feeds.rs`, `named_feeds(conn, feed_id)` and the Index builder. Delete the four functions and the type that the design names.
4. Change `ArtistIdentity::resolve` and `AlbumNode` to use the named feeds.
5. Add `role_words.rs`.
6. Change the release page view model and its screen: the artist link target and note, the role rows, the "Role not confirmed" rows and the "Also named" rows. Delete the "Publisher" link.
7. Change the track page view model and its screen: delete the "Publisher" link, and add the artist feed link.
8. Update each test that asserts the "Publisher" link or the observation time sequence.
9. Rewrite each source comment that describes the deleted functions, or delete it.

## Mechanical Acceptance Criteria

Use the prefix `adr_0085_named_feeds_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R85-3-01 | `sort_named_feeds` gives position sequence. A null position comes after each known position. Equal positions sort by feed GUID |
| R85-3-02 | `select_artist_feed`: a first feed with no role and a second feed with agreed `artist` give `Confirmed(1)` |
| R85-3-03 | `select_artist_feed`: no agreed `artist`, and a primary feed with no agreed role, give `Unverified(0)`. A primary feed with a one-side `artist` role also gives `Unverified(0)` |
| R85-3-04 | `select_artist_feed`: a primary feed with agreed `label`, and no other agreed `artist`, give `None` |
| R85-3-05 | Two named feeds, with a newer `observed_at` on the second feed, still give the first feed. This is the test of the ADR 0085 Verification section |
| R85-3-06 | Migration 19 adds `named_feed_position`, keeps each row and gives null. A version 18 fixture migrates to version 19 |
| R85-3-07 | The hydration stores the position of each row from `remote_items` with `source` `podcast_publisher`. An entry with no match stores null |
| R85-3-08 | The RSS check path keeps the position of the row that it changes |
| R85-3-09 | The Index `FeedView` gives the named feeds in `remote_items` position sequence, not in response sequence |
| R85-3-10 | `role_words` keeps the feed case, removes an exact duplicate and sorts. "label" and "recordLabel" stay two words. `role_list_text` gives "label", "label and producer" and "distributor, label and producer" |
| R85-3-11 | `stated_sides_text` gives "Sir Libre Records: label · the album: recordLabel" and "The album: label · Sir Libre Records: no role" |
| R85-3-12 | The release view model with an agreed `label` link gives the role row "Label: Sir Libre Records" with an open action. An artist feed with agreed `artist, producer` gives no role row |
| R85-3-13 | The release view model gives a "Role not confirmed" row for a `conflict` link, and an "Also named" row for a link with no agreement |
| R85-3-14 | The release view model gives the note "Unverified artist" for an unverified artist feed, and no note for a confirmed one. The artist link opens the feed page. A release with no artist feed opens the name level |
| R85-3-15 | No release name link and no track name link has the label "Publisher" or an accessibility label with the word "publisher" |
| R85-3-16 | The track view model gives one artist feed link with the feed title, and no role row |
| R85-3-17 | `ArtistIdentity::resolve` gives `Publisher(guid)` of the second feed for R85-3-02, and the name level for R85-3-04 |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: a release that names its artist first and Sir Libre Records second, with an agreed `label` link, shows "Label: Sir Libre Records". The row opens the Sir Libre Records page. The page shows no word "Publisher".
- V2: a HeyCitizen release shows the artist name with no note. The name opens the HeyCitizen page. The page shows no role row.
- V3: an Official DETOX Music release shows the artist name with the note "Unverified artist". The name opens the DETOX page.
- V4: a track page of a HeyCitizen track shows one link with the feed title, and no "Publisher" link. The link opens the HeyCitizen page. This carries §20 V2 of the [overhaul plan list](../plans/design-and-cleanup-overhaul-plan.md#phase-4-publisher-and-artist-pages).
- V5: a release page shows `publisher_text` as "Feed owner" text that opens nothing. This carries §21 V1 of the same list.
- V6: after migration 19, a Library release page shows the same artist link as before the migration.
- V7: normal and narrow widths, Light and Dark themes, and the larger type sizes show each row in its place, with no clipped text. "Unverified artist" reads as text, not through color alone.

## Exclusions

- No change to the feed page. Packet 008 of the ADR 0084 plan owns it. It uses `role_words.rs`.
- No storage of the title of each named feed. ADR 0085 permits only the position. The Recorded Facts give the effect.
- No Artists list change. Packet 008 owns it.
- No new MusicIndex request.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/db/publisher_relationships.rs`, `src/db/publisher_link_facts.rs`, `src/db.rs` (the migration registry) and `src/db/upgrades.rs`.
- `src/views.rs` (`FeedView`, the track view and `owned_publisher_feed_guid_from_relationships`), `src/sources.rs`.
- `src/application/queries/library.rs`, `src/library/app_impl.rs`, `src/application/commands/feed.rs` (the hydration).
- `src/rss/check_apply.rs` (the RSS check write).
- `src/view_models/artist_identity.rs`, `src/view_models/library.rs` (`AlbumNode`), `src/view_models/singles_group.rs`.
- `src/view_models/entity_detail.rs`, `src/view_models/album_page.rs`, `src/view_models/track_detail.rs` and their screens under `src/ui/shells/library/`.

## Checks

```bash
cargo test --lib adr_0085_named_feeds_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Migration 19 adds one nullable column. The operator makes a SQLite backup before the first run of the new build.
To roll back, revert the commit and restore the backup. A version 18 build cannot open a version 19 database.

## Operator Visual Check

The implementer writes this section at completion. It gives numbered steps for V1 to V7.
It states the needed state, what counts as wrong, and the cleanup. Step 1 is the SQLite backup before the first run.
Do not delete `/tmp/v4vmm-governance.ie6k8TQf`. Color alone is not a valid difference.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0082-task-003-release-page-named-feeds.md`
- ADR 0085, ADR 0084 Decision 7, ADR 0077 Decisions 2 and 6, and ADR 0016
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the position of each named feed, the artist feed selection, the role words and the release and track page rows.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- The view model gives each label, row and action availability. The screen only composes. Use the scaled tokens of ADR 0039 and existing shared composites.
- Tests use recorded values. No test sends a request.
- Never run `git checkout`, `git restore`, `git stash`, `git reset` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The feed page (`src/view_models/publisher_page.rs` and `src/ui/shells/publisher.rs`).
- The Artists list.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R85-3-01 to R85-3-17 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 to V7.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- The release page has no place for the role rows in its present element hierarchy.
- The hydration path has no access to `remote_items` where it writes the relationship rows.
- The migration needs a change to a table other than `feed_publisher_relationships`.
- A change needs a file in "Do not touch".
