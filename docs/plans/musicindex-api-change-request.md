# MusicIndex API Change Request

## Purpose

This document requests four changes to the MusicIndex API and its parser.
It comes from the v4vmm desktop client. Stophammer records its own decision.

Each change corrects a defect that affects every client, not only v4vmm.
Two changes are query-side only. One changes a parser rule. One is a release policy.

This request asks for no new provenance model and no new storage design.
An earlier and much longer request exists. This document replaces it as the request.
The [detailed audit](adr-0075-stophammer-decision-request.md) remains available as the annex.

## Evidence Limits

The inspected revision is commit `a220f44` in the local checkout.
The deployed revision is unverified. Confirm it before you act on this request.

The measured counts come from the local `stophammer.db`, dated 2026-04-19.
That file may not hold the deployed data. Treat each count as an order of magnitude.

| Measured item | Count |
|---|---:|
| Feeds | 7,632 |
| Tracks | 23,960 |
| Tracks with their own `image_url` | 1,781 |
| Feed `release_date` claims with path `feed.pub_date` | 7,414 |
| Feed `release_date` claims with path `oldest_item.pub_date` | 214 |

## Summary

| # | Change | Kind | Reingest |
|---|---|---|---|
| 1 | Return summary fields with search results | Query | None |
| 2 | Return track and feed artwork as separate fields | Query | None |
| 3 | Record which element supplied a feed publication date | Parser and ingest | Incremental |
| 4 | Never rename a response field without a version | Release policy | None |

## 1. Search Results Carry No Title

**What happens.** `SearchResponseItem` at `src/query.rs:306` returns `entity_type`,
`entity_id`, `rank`, `quality_score`, `feed_guid`, and `href`. It returns no title.

**What it costs any client.** A client cannot draw a result row from a search response.
It must request full detail for each hit. A page of 20 results costs 21 requests.
The v4vmm measurement recorded five requests for one feed and two tracks.

**Why the fix is small.** The handler at `src/query.rs:1703` already opens a reader
connection. For some track rows it already calls `db::get_track_by_guid` and then keeps
only the GUID and the href. The row it needs is already in hand.

**Request.** Return the fields that a result row needs, inside the loop that already
loads the row. A title is the minimum. A subtitle, an image URL, and a publication date
would remove almost all detail requests for list views.

Name the owner and the coverage of each summary field. State that a summary field is not
evidence of absence in full detail.

**Reingest.** None. The values are already stored.

## 2. Track Artwork Hides Its Owner

**What happens.** Five queries select `COALESCE(t.image_url, f.image_url)`.
They are at `src/query.rs:530`, `:549`, `:1967`, `:2044`, and `:2075`.
The response field `image_url` therefore holds either the track's image or the feed's
image. The response does not say which.

**What it costs any client.** A client cannot tell album art from track art.
It cannot show "this track has no art of its own". Matching URLs cannot recover the
difference, so no client can repair this locally.

In the measured database, 1,781 of 23,960 tracks have their own image. The other 22,179
tracks return feed artwork through a field that reads as the track's own image.

**Why the fix is small.** `tracks.image_url` and `feeds.image_url` are separate columns
already. See `migrations/0025_source_first_feed_track_fields.sql` and
`migrations/0032_feed_scoped_track_identity.sql`. The `COALESCE` happens at query time.

**Request.** Return the track image and the feed image as separate fields.
Keep the existing `image_url` field with its current behavior, so older clients continue
to work. Include a test where the track has no image, and a test where both owners assert
the same URL.

**Reingest.** None. Both values are already stored.

## 3. A Build Timestamp Can Become A Release Date

**What happens.** `stophammer-parser/src/profile.rs:180` carries the comment
"RSS2: lastBuildDate as pubDate fallback". The rule at line 184 reads `lastBuildDate`
into `FeedField::PubDate`.

`src/api.rs:2092` then computes `feed_data.pub_date.or(oldest_item_at)`.
Lines 2101 and 2103 label the resulting `release_date` claim `feed.pub_date` whenever
`pub_date` is present. The label does not record that `lastBuildDate` supplied the value.

**What it costs any client.** `lastBuildDate` is the time the feed file was generated.
It is not a publication date, and a generator can update it every hour. A client that
shows a release date can therefore show today's date for a ten-year-old album.
The claim states an extraction path that the value did not come from.

In the measured database, 7,414 feed `release_date` claims carry the `feed.pub_date`
path. The incorrect subset is the feeds with no `pubDate` element and a `lastBuildDate`
element. That subset is not recorded, so its size is unknown.

**Request.** Record which element supplied the value. Two options:

1. Keep the fallback and label the claim by its true source, such as
   `feed.last_build_date`.
2. Drop the fallback, and let a feed with no `pubDate` have no publication claim.

Either option is acceptable to this client. The first retains more data.
Do not keep a path label that names an element the parser did not read.

**Reingest.** Incremental, not a full pass. A parser change corrects a feed when that
feed is ingested again. The crawler and the podping listener already revisit feeds.
Only a feed whose content never changes needs `force_reingest`, because
`ContentHashVerifier` short-circuits an unchanged feed.

That forced pass can run as a background trickle over the affected feeds.
No client needs a synchronized reingest of all 7,632 feeds.

## 4. A Renamed Field Disappears Silently

**What happens.** Clients decode collection and scalar fields as optional, because the
contract marks them optional. A renamed field therefore decodes as absent.

**What it costs any client.** A rename produces no error, no warning, and no failed
request. The data stops arriving, and the client reports "not returned". The defect can
stay hidden for months.

**Request.** Treat a field rename as a breaking change that needs a version.
Keep the generated contract in `openapi.rs` matched to the deployed routes.
Do not hand-edit generated JSON as the source of the contract.

**Reingest.** None.

## What This Request Does Not Ask For

The v4vmm client models the owner, the source, and the observation time of every value
it holds. That model belongs to v4vmm. This request does not ask MusicIndex to adopt it.

These items are deliberately excluded:

- A full provenance record for each claim.
- A completeness signal for each collection. The client builds its own proof and treats
  MusicIndex collections as unverified.
- An owner marker on payment routes. This is deferred and is not urgent.
- Any change to payment behavior, broadcast operations, or ingestion policy.

## One Question For The Operator

What revision is deployed? Every statement above describes commit `a220f44` in a local
checkout. No live endpoint was inspected.

## Checks

The language check ran on this document. No Stophammer maintainer has read it.
No Stophammer decision exists for this request.
No agent wrote in the Stophammer checkout. The measured counts came from read-only
queries against a local database file.
