# Publisher Feeds As Artist Identity: Research

## Status And Scope

Research note - 2026-09-23. This note is advisory. It states no rule.
A later decision record can cite it. [ADR 0076](../adr/0076-playlist-rss-check-for-stale-musicindex-records.md) is the related Proposed decision.

The note records how v4vmusic, the Podverse rewrite, and Wavlake treat publisher feeds as artist identity.
It also records how the loss of the Stophammer artist identifiers affects v4vmm.
The v4vmusic source code is not public. Its public change log is the only v4vmusic evidence.

## Sources

| Source | Revision or date | Access |
|---|---|---|
| [V4V Music change log](https://v4vmusic.com/pages/changes) | "Last updated September 23, 2026", version 0.17.5 | Public page, read on 2026-09-23 |
| [V4V Music about page](https://v4vmusic.com/pages/about) | Read on 2026-09-23 | Public page |
| [Podverse monorepo](https://github.com/podverse/podverse) | Commit `fcdd549`, 2026-09-19 | Shallow clone |
| [Wavlake artist feed, DETOX](https://wavlake.com/feed/artist/137aaa9c-75ff-4916-9f23-e02968b2d15e) | Read on 2026-09-23 | Public RSS |
| [Wavlake album feed, DETOX](https://wavlake.com/feed/cf3fb24c-582c-45dd-8ac7-bb41cdf4d41a) | Read on 2026-09-23 | Public RSS |
| `https://api.musicindex.org` | Read on 2026-09-23 | Read-only GET requests |
| Local Stophammer database `stophammer.db` | Dated 2026-04-19 | Read-only queries |
| Local Stophammer checkout | Commit `2436e2a` | Source and history |

## Publisher Links In The Index

These counts come from the local Stophammer database, dated 2026-04-19. The deployed data can differ.

| Measured item | Count |
|---|---:|
| Feeds | 7,632 |
| Feeds on Wavlake | 6,652 |
| Music feeds that declare a publisher feed | 7,139 of about 7,583 |
| Music feeds that declare more than one publisher feed | 0 |
| Distinct publisher feeds | 1,642 |
| Publisher feeds with one album | 818 |
| Publisher feeds with 20 or more albums | 73 |
| Largest album count for one publisher feed | 131 |

The local podping archive holds 23,877 feeds and no Wavlake feed. The archive can be partial.

## Wavlake Link Directions

The album-to-publisher link is correct. The DETOX album declares publisher `137aaa9c-75ff-4916-9f23-e02968b2d15e`.
The artist feed's own `podcast:guid` has the same value. Its `podcast:medium` is `publisher`.

The publisher-to-album link is not correct. The artist feed lists 30 albums.
Each `podcast:remoteItem` carries the Wavlake URL identifier as `feedGuid`, not the album's `podcast:guid`.

| Listed `feedGuid` | Album `podcast:guid` |
|---|---|
| `cf3fb24c-582c-45dd-8ac7-bb41cdf4d41a` (DETOX) | `e5ac2d62-2ce7-517a-8bfd-24bff61b2aa9` |
| `0ea057e2-fcf3-4e1d-881e-51a87b582b92` | `210f82e5-363f-5d77-94b5-c1d99d8d4807` |

A match by GUID therefore fails for each Wavlake back-link.
The MusicIndex relationship for the DETOX album reports `reciprocal_declared: false` and `two_way_validated: false`.

Wavlake serves feeds through a CDN with an `ETag` and `cache-control: max-age=43200`.
A conditional request returned `304` with `x-vercel-cache: HIT`.

## V4V Music

V4V Music runs its own RSS index. The change log names no MusicIndex use.

| Deployment | Change log entry, summarized |
|---|---|
| 2025-10-07 | Scheduled tasks refresh all feeds and discover new feeds from Podcast Index |
| 2025-10-23 | Podping through a livewire WebSocket, with a five-minute debounce for each feed |
| 2025-10-23 | Conditional GET with stored `etag` and `lastModified` values |
| 2025-11-09 | A song removed from RSS becomes INACTIVE and returns when it reappears. Before this change, a refresh destroyed user data |
| 2025-11-18 | Publisher system. A publisher feed is the artist page. One-way references are accepted for "Wavlake-style feeds" |
| 2025-11-18 | "NEVER use feedGuid from `<podcast:remoteItem>` RSS attributes (always wrong for Wavlake feeds)". "NEVER use URLs for identification" |
| 2025-11-18 | The app fetches each listed album's RSS for its canonical `podcast:guid`, and skips the album when that fetch fails |
| 2026-01-19 | Podping also triggers publisher feed updates |
| 2026-01-20 | The podping debounce is one minute |
| 2026-05-08 | A person is unique on name, href, group, and role |
| 2026-05-25 | A `rel` attribute on `podcast:remoteItem`: artist, label, network. A missing `rel` counts as artist. Two-way validation compares `rel` on both sides |
| 2026-05-26 | The publisher title comes from `itunes:author`, then `author` or `dc:creator`, before `<title>` |
| Undated entry near 2026-07 | An artist name opens the publisher through the album's `feedGuid`. "Real GUIDs required" |

The about page names "advanced rate limiting system to prevent HTTP 429 errors from RSS providers".

### V4V Music Public Publisher API

`GET https://v4vmusic.com/api/publishers`, read on 2026-09-23:

- The API reports 597 publishers. In the first 100, 75 are on Wavlake, 12 on Fountain, and 9 on RSS Blue.
- `availableRels` reports `artist` 596, `label` 1, and `producer` 1.
- Each publisher without `rel` also reports `"rels": ["artist"]`. The API does not separate an assumed role from a stated role.
- `authorName` is "Unknown Artist" for 97 of the first 100 publishers. Wavlake artist feeds carry no `itunes:author`.
- The first 100 publishers report 605 verified and 137 unverified albums.

Two publisher feeds write `rel`:

| Publisher feed | `rel` use |
|---|---|
| Sir Libre Records, `sirlibre.com` | `rel="label"` on 7 albums. The change log author signs as "Sir Libre" |
| Jimmy V, `music.jimmyv4v.com` | `rel="artist"` on his albums, and `rel="producer"` on a Wavlake album of another artist |

The Jimmy V producer link shows that a publisher feed can list an album that names a different publisher.

## The `rel` Attribute

The Podcast Namespace defines no `rel` attribute on `podcast:remoteItem`. It defines `rel` on `podcast:alternateEnclosure` with a different meaning.
The `podcast:publisher` specification calls a publisher a "parent publishing entity". It does not define an artist or a label.

MSP-2.0 writes `podcast:publisher` and `remoteItem medium="publisher"` without `rel`.
This is true for [ChadFarrow/MSP-2.0](https://github.com/ChadFarrow/MSP-2.0) at commit `183e424` (2026-08-30) and the Kolomona fork at commit `553fcc3` (2026-01-18).
The six largest non-Wavlake publisher feeds in the index and the DETOX Wavlake artist feed carry no `rel`.

## Podverse Rewrite

- A feed with medium `publisher` has the route kind `artist`. The type is `ChannelRouteKind = 'podcast' | 'album' | 'artist'`.
- The artist page lists the publisher feed's `podcast:remoteItem` entries. `PublisherFeedService` matches them to known albums by `podcast:guid` only.
- An entry with no match shows as "unadded" with a Podcast Index preview.
  This note did not run the code. With the Wavlake back-link values above, most Wavlake albums probably show as "unadded".
- Podping handles live items only (`runLiveItemListener.ts`). Add-by-RSS sends `If-None-Match`.

## Stophammer Artist Identifiers

Stophammer added artist credits to its public reads on 2026-03-12.
Commit `a16a720`, "Drop public artist_credit from source reads", removed them on 2026-04-08 with the artist routes.

The deployed API still sends artist text: `release_artist` and `release_artist_sort` on a feed, and `track_artist`, `track_artist_sort`, and `release_artist` on a track.
The v4vmm views show that text (`src/views.rs`).

The ADR 0045 artist binding in `src/identity_ingest.rs` reads `artist_credit`. It receives no artist identifier after 2026-04-08.

## Findings

1. V4V Music, the Podverse rewrite, and Wavlake all treat a publisher feed as an artist page.
2. The album-to-publisher link is the reliable direction on Wavlake.
3. A Wavlake publisher's album list needs a resolution through `feedUrl` to each album's `podcast:guid`. V4V Music does this for each listed album.
4. V4V Music keeps a track that RSS removes, and restores it when it returns. This note did not check Podverse for this behavior.
5. V4V Music uses conditional GET, podping, and a rate limiter against HTTP `429`.

## Limits

The V4V Music evidence is change log text, not code. It can differ from the deployed behavior.
The Podverse evidence is source code that this note did not run.
The Wavlake evidence is two albums and one artist feed.
