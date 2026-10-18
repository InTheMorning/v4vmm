# ADR 0085: Artists By Confirmation, And No Publisher Word

## Status

Accepted - 2026-10-18. The operator asked for this decision, answered its open questions and accepted it on 2026-10-18.

It supersedes [ADR 0082](0082-publisher-roles-belong-to-each-album-link.md) Decisions 1, 2, 3 and 6.
ADR 0082 Decisions 4 and 5 stay.
It amends [ADR 0084](0084-music-column-browser.md) Decision 7: the publisher level becomes the feed level, and Decisions 5 and 6 of this ADR replace the "Publisher" link of the release page.

## Context

ADR 0077 keys an artist page on the feed GUID that an album names with `podcast:remoteItem medium="publisher"`. ADR 0082 groups the albums of that feed by `album_names_as` and by the agreement of the `rel` role of each link.

On 2026-10-18 the operator inspected ADR 0082 packet 002 on the HeyCitizen page. The operator found three problems:

- The word "publisher" has more than one meaning. A curator can read it as a record label, a podcast host or a company. The interface must not show it as the name of a feed.
- A small number of feeds state `rel`. On 2026-10-02 the MusicIndex site counted 6 feeds with a stated `rel` ([site ADR 0011](../../../musicindex/docs/adr/0011-search-site-reads-rel-as-pr-793-gives.md)). Thus almost all pages show "Role not stated", and the role structure is mostly empty.
- A "Publisher of" group with a role caption and a role line on each row states one fact three times.

The operator wants the interface to show artists, confirmed and unverified, until more feeds state `rel`. Labels and other roles appear when feeds state them.

[ADR 0075](0075-metadata-contract.md) Decision I stays: the app never derives a role from a count or a name match.

An album can name more than one feed. The orchestrator recorded these facts on 2026-10-18:

- Podcast-namespace PR #793, open on 2026-10-02, puts one `remoteItem` for each party in `<podcast:publisher>`, the primary party first. Example 2 of `../stophammer/docs/publisher-links-guide.md` names an artist feed and a label feed.
- Stophammer ADR 0069 §1a makes each item in `<podcast:publisher>` a publisher link, with `album_names_as` `publisher`.
- The Stophammer parser gives each channel `remoteItem` its document position (`extract_feed_remote_items` in `stophammer-parser/src/engine.rs`). The counter also counts other channel items, so the numbers keep the sequence but can have gaps.
- MusicIndex gives that sequence in `remote_items[].position`, with `source` `podcast_publisher`. Stophammer itself uses the lowest position as the primary party for `publisher_feed_title`.
- A MusicIndex publisher row has no position. v4vmm decodes `remote_items` with `position`, but it does not store the position of a named feed.
- v4vmm selects one named feed with `owned_publisher_feed_guid` in `src/db/publisher_relationships.rs`. It takes the most recent `observed_at`, so with two named feeds the result does not follow the RSS.

## Decision

### 1. The Interface Does Not Name A Feed "Publisher"

No label, heading, badge, breadcrumb, action label or accessibility label calls a feed a "publisher".

These keep the technical term:

- code identifiers, database columns and API field names
- the CLI JSON output
- the Inspect sources view, which shows each RSS value with its element name, for example `podcast:remoteItem medium="publisher"`

### 2. Artists Have Three Confirmation Levels

Each feed and each credit name in Artists has one level. The level comes from the link facts of Stophammer 0.7.0 only.

| Level | Rule |
|---|---|
| Artist | One link or more of the feed has `role_agreement` `both`, and its role set contains `artist` |
| Unverified artist | The feed is the primary named feed of one album or more (Decision 6), and no link of the feed has `role_agreement` `both` |
| Name only | An artist text that names no feed. This is the name level of ADR 0084 Decision 7 |

- A feed with agreed roles, none of which is `artist`, is not in Artists. Decision 4 shows it.
- A one-side `artist` role does not confirm an artist. The feed stays an unverified artist, and its page shows the stated role with its side.
- A feed that albums name only after their primary named feed, with no agreed role, is not in Artists. The release page shows it (Decision 6).
- A feed that no album names is not in Artists. Its page opens from "Shares albums with" and from search.
- The Artists list shows no level word. Only the artist page shows the level (Decision 3).
- "Unverified" tells that nothing confirms the role. The app uses "artist" as its default word for a named feed, not as a derived fact.

### 3. The Artist Page Groups Releases By Kind

The page of a feed has no type. It shows the level of Decision 2 in its header, in words.

- The releases that name the feed group by kind, as ADR 0084 Decision 6 gives: "Albums and EPs", then "Singles".
- A release with a link that has `role_agreement` `one_side` or `conflict` shows one role line, with the source texts of ADR 0082 Decision 3 ("stated by the album only", "not confirmed").
  In these texts, the page uses the feed title, not "this feed" and not "the publisher".
- The releases that the feed lists and that do not name the feed (`album_names_as` is null) come last, under "Listed by <feed title>, not named by the album".
- A release appears one time.

### 4. Other Roles Appear When A Feed States Them

- When a link of the feed has an agreed role set without `artist`, its releases show under a "<roles> of" heading, for example "Label of". The role words come from the feed. The app keeps no list of role words.
- The header lists the agreed roles, as ADR 0082 Decision 4 gives.
- "Shares albums with" stays as ADR 0082 Decision 5 gives. Each row shows the feed title, its roles and the count of shared albums, with no source text.
- A Labels destination is a subsequent decision. The operator makes it when many feeds have a label role.
  MusicIndex 0.7.0 gives no count of those feeds. A request to Stophammer is a candidate, and this ADR does not make it.

### 5. The Release Page Names Its Artist, And A Role Row Only For Another Role

- The artist name on a release page opens the artist page of the artist feed of Decision 6, as ADR 0084 Decision 7 gives.
- The release page has no "Publisher" row.
- When the agreed role set of the link does not contain `artist`, the release page shows one role row with the source text.
  An example is "Label: Sir Libre Records". The row opens the page of that feed.

### 6. An Album Can Name More Than One Feed

The named feeds of an album keep the sequence of the RSS: the `position` of each item with `source` `podcast_publisher`. The first one is the primary named feed.

- The artist feed of the album is the first named feed with an agreed role set that contains `artist`. When no named feed has one, it is the primary named feed, and the release page says that the artist is unverified.
- Each other named feed with a stated role shows as a role row of Decision 5, for example "Label: Sir Libre Records".
- Each other named feed with no stated role shows as an "Also named" row with its feed title. The row opens the page of that feed.
- An observation time never selects a named feed.

## Invariants

- No interface text names a feed "publisher".
- A level comes from `role_agreement`, `role` and `album_names_as` only. A count, a name match or a credit text never selects it.
- Each level shows in words. Color is never the only difference.
- A release appears one time on one page.
- A stated role that one side gives stays visible, with its side.
- The named feeds of an album keep their RSS sequence.

## Non-Goals

- No Labels destination.
- No new MusicIndex request.
- No storage change apart from the position of each named feed. That change uses the migration registry of ADR 0016.
- No change to the name level of ADR 0084 Decision 7, apart from its level word.

## Consequences

- Almost all artist pages say "Unverified artist" until more feeds state `rel`. The Artists list shows no level, so the list does not repeat the word.
- A feed such as HeyCitizen shows as "Artist", with its five agreed roles in the header.
- A label feed such as Sir Libre Records is not in Artists. Its releases reach it through the role row of Decision 5.
- ADR 0082 packet 002 keeps its decode, its storage, its header roles and "Shares albums with". Its groups, subgroups and row role text are replaced. Its visual checks V1, V2 and V4 retired on 2026-10-18. V3 stays.
- ADR 0082 packet 003 is rewritten for Decision 5 before it starts.
- ADR 0084 packet 008 builds Decisions 2 to 4.
- v4vmm stores the position of each named feed, from `remote_items`. The schema version goes from 18 to 19. `owned_publisher_feed_guid` uses the artist feed of Decision 6 in place of the most recent `observed_at`.
- Until PR #793 is merged, the namespace allows one item in `<podcast:publisher>`. Albums with two named feeds are thus rare today.

## Alternatives Considered

- A page type from roles and from name matches, as the MusicIndex site does (site ADRs 0007 and 0010). Rejected: a name match derives a role (ADR 0075 Decision I), and a feed with five roles gets one layout.
- Keep the word "publisher", with "<roles> of" headings for each relation. The orchestrator proposed it on 2026-10-18. Rejected by the operator: the word has more than one meaning, and the role headings are mostly empty while a small number of feeds state `rel`.
- A level word on each Artists list row. Rejected by the operator on 2026-10-18: the word repeats on almost all rows.
- Two sections in the Artists list, "Artists" and "Unverified artists". Rejected by the operator on 2026-10-18, in favor of the level on the artist page only.
- The source text "albums give" on "Shares albums with" rows, as the MusicIndex site writes. Rejected by the operator on 2026-10-18: the row shows the roles only.
- Each named feed with no stated role as an "Unverified artist". Rejected by the operator on 2026-10-18: a label that an album names second would then show as an artist.
  Example 3 of the guide, two artists with no `rel`, thus shows one artist in Artists. The second shows on the release page as "Also named".

## Verification

Mechanical, phrased at the owning layer:

- A guard finds no interface string constant in `src/view_models` or `src/ui` that names a feed "publisher". The guard cites this ADR.
- The artists view model gives the level of Decision 2 for each `role_agreement` and `album_names_as` combination. Its list rows expose no level text.
- The artist page view model groups releases by kind, puts the listed releases last and puts each release one time.
- The release page view model gives a role row only when the agreed role set does not contain `artist`.

Visual, for the operator:

- HeyCitizen shows "Artist" and its five roles. DETOX shows "Unverified artist". No screen shows the word "publisher".
- A label feed shows its "Label of" heading, and its release shows the "Label:" row.
- The release view model gives the named feeds in their position sequence, and selects the artist feed by the rule of Decision 6.
- A test with two named feeds and a newer observation of the second feed still selects the first feed.
