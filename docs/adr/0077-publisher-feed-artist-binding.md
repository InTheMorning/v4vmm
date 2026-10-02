# ADR 0077: Publisher Feed Artist Binding

## Status

Accepted - 2026-09-24. The operator accepted this ADR on 2026-09-24.
The operator gave the direction of Decisions 1, 3 and 4 on 2026-09-23.
The [publisher relationship request](../plans/stophammer-publisher-relationship-request.md) records that direction.
It keys an artist page on the publisher feed GUID, shows `role`, and selects the page type with the artist count.
Decisions 2, 5, 6 and 7 and the text of each decision came with the draft of 2026-09-24.

The operator reviewed each proposal on 2026-09-24. "Accepted Refinements" records the results.
Implementation partial: packet 001 completes Decision 7 on 2026-09-24 with mechanical checks Green. Its visual gate is open and paused.
Packet 002 completes the storage of Decision 5 on 2026-09-24 with mechanical checks Green. Packets 003 and 004 remain.

This ADR supersedes [ADR 0045](archive/0045-track-artist-binding.md).

Amended 2026-09-24: [ADR 0078](0078-publisher-page-type-from-stated-role.md) supersedes Decision 4.
The page type comes from a stated role only. The label threshold proposal is removed.
[ADR 0079](0079-remove-musicindex-artist-subject-storage.md) closes the proposal for ADR 0029 artist facts. It deletes that storage.

Amended 2026-09-24: the operator accepted proposals as refinements. "Accepted Refinements" records them.


Amended 2026-10-02: [ADR 0082](0082-publisher-roles-belong-to-each-album-link.md) gives roles to each album link and no page type. Decision 2 stays for the publisher that an album names. A role never makes the album page show the publisher as its artist.
## Context

ADR 0045 bound a Library track to a MusicIndex artist identifier. `src/identity_ingest.rs` reads that identifier from `Track.artist_credit`.
Stophammer commit `a16a720` removed `artist_credit` and the artist routes on 2026-04-08.
Since then, the binding receives no identifier. The app keeps the bindings that it stored before that date, but it writes no new binding.

The app opens an artist page in three ways today:

- A Library artist view groups local tracks by artist name text (`ArtistRef::LocalArtistName`).
- An Index artist page sends `/v1/tracks?artist=<name>`, so it matches name text.
- An Index publisher page sends `/v1/publishers/{publisher_text}`.

Name text is not an identity. Two artists can have the same name, and one artist can have two spellings.

Stophammer ADR 0049 changed `publisher_text` to the `itunes:owner` name.
On 2026-09-24, `/v1/publishers` gave 7,352 feeds for "Wavlake". A page for that text shows the feed writer, not an artist or a label.

Stophammer ADR 0049 is live on 2026-09-24. The [verification](../plans/stophammer-publisher-relationship-request.md#verification-against-the-deployed-api) records the fields.
An album feed with `include=publisher` gives one entry for each related publisher feed. The entry gives:

- `publisher_feed_guid`, the `podcast:guid` of the publisher feed,
- `music_names_publisher`, `true` when the album names this publisher in `podcast:publisher`,
- `publisher_lists_music`, `true` when the publisher feed lists the album,
- `publisher_link_resolution`, which is `guid`, `feed_url` or `unresolved`,
- `role` and `role_source`. A `role_source` of `default` means that no feed states a role.

A publisher feed also gives `distinct_release_artist_count` and `distinct_release_artists`.
Stophammer derives these values from the `itunes:author` text of the albums that name the publisher. A "feat." credit can count as a different artist.

The [research note](../notes/2026-09-23-publisher-feed-artist-research.md) records that V4V Music, the Podverse rewrite and Wavlake each use a publisher feed as the artist page.
The note also records that the album-to-publisher link is the reliable direction on Wavlake.

ADR 0075 Decision I makes RSS the only provenance.
The `podcast:publisher` element of an album channel is RSS that the channel states.
The link resolution, the default role and the artist count are values that MusicIndex derives.

## Decision

### 1. A Publisher Feed GUID Identifies An Artist Page

The identity of an artist page is the `podcast:guid` of a publisher feed.
The app gets that value from `publisher_feed_guid`. A new `ArtistRef` variant carries it.

Name text never becomes an artist identity. This ADR now owns the rule against name-only, fuzzy, filename and tag-only binding. That rule came from ADR 0045.

### 2. An Album Binds To The Publisher That It Names

The binding is between an album feed and a publisher feed. It is not between a track and an artist.
The app binds an album to a publisher only when `music_names_publisher` is `true`.

A publisher feed can list an album that names a different publisher. That relation is "listed by", not ownership.
The publisher page shows those albums in a separate group. The album page does not show that publisher as its artist.

A track opens the artist page through its album feed. This is navigation.
The app stores no publisher value on a track. ADR 0075 keeps this rule: a feed value never becomes a track value.

The track route `include=publisher` gave an empty array in each sample on 2026-09-24. The app does not read it.

### 3. The Page Shows The Role And Its Source

The page shows `role` with `role_source`.
When `role_source` is `default`, no feed states the role. The page must not show an assumed role as a stated fact.

When `role` is null, the two feeds state different roles. The page shows the two stated values.

### 4. The Page Type Comes From A Stated Role

Superseded by [ADR 0078](0078-publisher-page-type-from-stated-role.md) on 2026-09-24.
ADR 0078 owns the page type rule. The artist count never selects the type.

### 5. The Library Stores The Relationship As A Feed Fact

The Library stores each relationship entry of a feed as a MusicIndex observation under ADR 0075.
A new table holds the current relationship for each local feed. The ADR 0016 migration registry adds it.

The table keeps each field of the entry and the observation time.
It keeps `publisher_link_resolution` and `role_source`. The app does not combine them into one value.

### 6. Feed Owner Text Is Not An Artist

`publisher_text` is the `itunes:owner` name. The app shows it as the feed owner.
No artist page and no label page opens from `publisher_text` or from `/v1/publishers`.

### 7. The ADR 0045 Binding Is Deleted

The code that reads `Track.artist_credit` for a binding is deleted.
A migration removes the `track_artist_source_bindings` table. The app keeps no data that no entry point reads.

The guards that name ADR 0045 were deleted on 2026-09-24, when ADR 0045 was archived.
The screen check for `db::artist_source_fact(` stays until ADR 0079 deletes that storage.

## Accepted Refinements

The operator accepted each refinement below on 2026-09-24.

| Detail | Rule |
|---|---|
| Stated role values | Only `role = "label"` gives a label page. Each other stated value, such as `artist`, `producer` or `network`, gives an artist page. The page shows that stated value as text with its source |
| Role conflict | A null `role` with `role_source = "conflict"` gives an artist page. The page shows each stated value with its source. No side wins |
| Album without a publisher | The Library keeps its name-grouped artist view for that album. The page is labeled "Grouped by name". It has no identity, no role and no page type |
| Unresolved back-link | The binding stays when `music_names_publisher = true` and `publisher_lists_music = false` or `publisher_link_resolution = "unresolved"`. The album row on the publisher page is marked "Not listed by the publisher". The album page links to the publisher |
| Artist page title | The title is the `<title>` of the publisher feed. `distinct_release_artists` is never the title. A publisher feed without a title shows its GUID with a "No title" label |
| Index artist page by name | The name query `/v1/tracks?artist=<name>` becomes a search result, for example "Tracks matching \"DETOX\"". It has no artist identity, no role and no page type. Each result links to its album, and through the album to its publisher page |

## Alternatives Considered

- Keep the ADR 0045 binding and wait for Stophammer artist identifiers. Rejected: Stophammer removed them on purpose, and the publisher feed is the identity that RSS states.
- Bind a track to a publisher. Rejected: RSS states the publisher on the album channel, and a feed value never becomes a track value.
- Select the page type from `publisher_text`. Rejected: that text is the feed owner. For each Wavlake album it is "Wavlake".
- Use the listed direction as ownership. Rejected: a publisher can list an album that names a different publisher.
- Merge a name grouping with a publisher feed when the names agree. Rejected: that is a name-only match.

## Consequences

Positive:

- An artist page has an identity that RSS states and that other V4V clients also use.
- Two artists with the same name get two pages.
- A label with many artists gets a label page, not a page for one incorrect artist.
- The dead ADR 0045 binding, its table and its guards go away.

Negative:

- An album that names no publisher has no artist identity. It stays in a name grouping.
- A label that states no `rel` shows as an artist page. ADR 0078 records this consequence.
- Each Library feed needs one more MusicIndex response field and one more table.

## Invariants

- An artist identity comes only from `publisher_feed_guid`.
- A binding requires `music_names_publisher` to be `true`.
- A "listed by" relation never becomes ownership.
- An assumed role never shows as a stated role.
- The binding stays on the feed. No publisher value is stored on a track.
- `publisher_text` never opens an artist page or a label page.
- View models expose the page type and the role source. No renderer selects them.

## Acceptance Criteria

Mechanical:

- The API decode test for `PublisherResponse` covers each field of the live contract of 2026-09-24.
- The Library feed read model exposes the stored relationship with its resolution and role source.
- The artist page view model exposes the page type, the type source, the role and the role source.
- The view model exposes "listed by" albums separately from the albums that name the publisher.
- No code path builds an artist identity from name text or from `publisher_text`.
- No code reads `track_artist_source_bindings` or `Track.artist_credit`.

Visual, for the operator:

- An album page opens its artist page, in Light and Dark themes.
- A label page shows its artists and its albums.
- A default role does not look like a stated role.
- A "listed by" album looks different from an owned album.

## Relationship To Other Decisions

- Supersedes ADR 0045 from 2026-09-24.
- Uses ADR 0075 Decision I for provenance and ADR 0075 for observation storage.
- Uses ADR 0016 for the new table and for the removal of the old table.
- Depends on Stophammer ADR 0049. The live contract at `/openapi.json` is the specification.
- ADR 0076 does not compare publisher values. This ADR keeps that scope.
