# Stophammer Publisher Album Summary Request

## Status

Draft - 2026-09-24. The orchestrator recommended this request on 2026-09-24, when the operator asked which route is better. The operator has not sent it.
This document is a request from the v4vmm client. Stophammer records its own decision.
It binds nothing in Stophammer.

## Purpose

The v4vmm publisher page shows the albums of one publisher feed.
Today, that page must send one request for each album to get a title and an image.
This request asks for a summary of each album in the publisher relationship entry, so that one request is sufficient.

It also asks two questions. The v4vmm packets need the answers before they read the relationship as complete.

## Evidence - 2026-09-24

Read-only GET requests to `https://api.musicindex.org`:

- `GET /v1/feeds/bcbe7207-9338-474e-ba18-09e6b1b69979?include=publisher` returns 9 entries with `direction = "publisher_to_music"`.
- Each entry gives `music_feed_guid`, `music_feed_url`, the link state and the role. No entry gives an album title or an image.
- The local Stophammer database of 2026-04-19 has a publisher feed with 131 albums. That page would need 132 requests.

The [publisher relationship request](stophammer-publisher-relationship-request.md#verification-against-the-deployed-api) records the rest of the verification.

## Requested Change

Add these fields to each `PublisherResponse` entry, for the album that `music_feed_guid` names:

| Field | Value |
|---|---|
| `music_feed_title` | The `<title>` of the album feed. Null when Stophammer has not indexed the album |
| `music_feed_image_url` | The channel image URL of the album feed. Null when the album states none |
| `music_release_artist` | The `release_artist` of the album, with `music_release_artist_source` |

Each field is a value that the album channel states. None of them is derived.

## Questions

1. **Does the publisher feed list an album that names it but that it does not list?**
   On a publisher feed, does `include=publisher` return an entry for an album with `music_names_publisher = true` and `publisher_lists_music = false`?
   v4vmm marks such an album "Not listed by the publisher". It must find the album on the publisher page.
2. **Is the `publisher` collection complete for one feed?**
   Does one response hold every entry, with no pagination and no limit?
   v4vmm deletes a stored relationship only when a response proves completeness. ADR 0075 packet 013 requires that proof.

## What v4vmm Does With The Answer

- With the summary fields, packet 003 sends one request for each Index publisher page.
- With answer 1, packet 003 reads "listed by" and "not listed" albums from one list.
- With answer 2, a subsequent packet registers the completeness contract, and stored relationships can be replaced.
- If Stophammer rejects the summary fields, the operator selects a new route for packet 003.
