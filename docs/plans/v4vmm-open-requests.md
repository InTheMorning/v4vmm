# v4vmm Open Requests To Stophammer

## Status

Open - 2026-09-25. This document consolidates each open request from the v4vmm client.
The Stophammer repository and the v4vmm repository each hold a copy at `docs/plans/v4vmm-open-requests.md`. Update both copies together.

This document binds nothing in Stophammer. Stophammer records each decision in its own ADR.
The v4vmm operator reviewed each item on 2026-09-24 and 2026-09-25.

## Evidence Base

- Live API: read-only GET requests to `https://api.musicindex.org` on 2026-09-25, and the contract at `/openapi.json`.
  The contract changed on 2026-09-25. The deploy of that day added `/v1/copies`, `/v1/feeds/{guid}/copies`, `/v1/feeds/{guid}/route-history`, `/v1/guid-changes` and `/v1/blocks`, and removed `/v1/proofs/challenge` and `/v1/proofs/assert`.
- Stophammer source: the local checkout at commit `64052ea`, read on 2026-09-25. The deployed revision is unconfirmed. Request 2 asks for a way to confirm it.
- v4vmm documents: `docs/plans/musicindex-api-change-request.md`, `docs/plans/stophammer-publisher-relationship-request.md`, and the album summary request that this document replaces.

## Requests

| # | Request | Kind | Priority |
|---|---|---|---|
| 1 | Album summary fields in each publisher relationship entry | API field addition | Wanted |
| 2 | A deployed revision that a client can read | API field addition | Wanted |
| 3 | Capabilities that match the contract for track includes | Contract correction | Small |
| 4 | Field renames are breaking changes | Release policy | Wanted |

### 1. Album Summary Fields In Each Publisher Relationship Entry

**What happens.** `GET /v1/feeds/{publisher_guid}?include=publisher` returns one `publisher_to_music` entry for each album that the publisher feed lists.
Each entry gives `music_feed_guid`, `music_feed_url`, the link state and the role. No entry gives an album title or an image.

**What it costs.** A client that shows the albums of a publisher sends one feed request for each album.
On 2026-09-24, one publisher listed 9 albums. The local database of 2026-04-19 had a publisher with 131 albums, which costs 132 sequential requests.

**Request.** Add these fields to each `PublisherResponse` entry, for the album that `music_feed_guid` names:

| Field | Value |
|---|---|
| `music_feed_title` | The `<title>` of the album feed. Null when the node has not indexed the album |
| `music_feed_image_url` | The channel image URL of the album feed. Null when the album states none |
| `music_release_artist` | The `release_artist` of the album |
| `music_release_artist_source` | The `release_artist_source` of the album |

Each value is a value that the album channel states, or its existing source label. None of them is new derived data.

**What v4vmm does with it.** The v4vmm publisher page sends one request for each publisher. v4vmm packets 0077-003 and 0077-004 wait for these fields.

### 2. A Deployed Revision That A Client Can Read

**What happens.** `/openapi.json` gives `info.version` from `CARGO_PKG_VERSION` (`src/openapi.rs:796`), which stays `0.1.0` across deployments.
`/node/info` returns only `node_pubkey`. `/v1/node/capabilities` returns `api_version: "v1"`.

**What it costs.** A client cannot tell which revision serves a response. Each v4vmm verification records "deployed revision unconfirmed".

**Request.** Report the source revision and the build time of the running node, for example in `/node/info`: `git_revision` and `built_at`.

### 3. Capabilities That Match The Contract For Track Includes

**What happens.** On 2026-09-25, `/v1/node/capabilities` lists these track includes: `payment_routes`, `value_time_splits`, `source_links`, `source_ids`, `source_contributors`, `source_release_claims`, `source_enclosures`, `source_transcripts`.
The contract for `/v1/tracks/{guid}` also lists `remote_items` and `publisher`, and the live route accepts both.

**Request.** Make the capabilities list equal to the contract, or state which of the two a client must read.

### 4. Field Renames Are Breaking Changes

This is change 4 of the v4vmm API change request of 2026-09-22. No external check can prove a release policy, so it stays open.

**What happens.** Clients decode fields as optional, because the contract marks them optional. A renamed field decodes as absent, with no error.

**Request.** Treat a field rename or removal as a breaking change that needs a version. Keep the generated contract matched to the deployed routes.
A Stophammer decision record that states this policy closes the request.

## Open Question

**The retention of a publisher resolution.** When an album feed changes or disappears, what happens to its stored back-link resolution and its `publisher_link_observed_at`?
The v4vmm publisher relationship request asked this for the Stophammer ADR. ADR 0049 does not state it.

## Stophammer Answers - 2026-09-25

Stophammer examined each request against the live API and the source at commit `64052ea`.
The work plan in `docs/plans/client-requests-work-plan.md` of the Stophammer repository gives the sequence of the work.
These answers are advisory. The ADR that each answer names is the owner of the rule. The Stophammer operator approved the recommended work on 2026-09-25.

| # | Answer | Work plan item |
|---|---|---|
| 1 | Confirmed and recommended. musicindex.org request 1 asks for the same fields on `remote_items` entries. One rule covers the two requests. The fields are added fields, so they stay in `v1` (ADR 0044 §4). ADR 0059, Accepted on 2026-09-25, owns the rule. The names are `remote_feed_title`, `remote_feed_image_url`, `remote_release_artist` and `remote_release_artist_source`, not `music_*`. | 5 |
| 2 | Confirmed and recommended. `/node/info` can give `git_revision` and `built_at`. The fields are added fields | 4 |
| 3 | Confirmed as a defect. `handle_capabilities` in `src/query.rs` does not list `remote_items` and `publisher` for tracks. The fix makes the capabilities route give the list that the track route accepts. Also, the track route ignores an include name that it does not know, and gives no error | 1 |
| 4 | Answered by ADR 0044 §4, Accepted: "A field rename or a field removal in a `v1` response is a breaking change. It needs a new path version." ADR 0044 applies to the fields of a response that a client decodes without a credential. It gives no rule for the removal of a route. The two proof routes were write routes that needed a credential. On 2026-09-25 the Stophammer operator decided to keep ADR 0044 as it is. ADR 0044 §4 closes this request | 7 |

**The open question.** Stophammer keeps no publisher resolution. `music_to_publisher_facts` in `src/query.rs` calculates `publisher_link_resolution` and `publisher_link_observed_at` again at each read. When an album feed is deleted, the next read gives the link as `unresolved`. When an album feed changes, the next read gives the resolution of the changed feed. Work plan item 6 adds this statement to ADR 0049.

## Deploy Of 2026-09-26

Stophammer deployed commit `607bb3a` on 2026-09-26 at 03:29 UTC.

- Request 2 is complete. `GET /node/info` gives `git_revision` and
  `built_at`. A node that is not built with `deploy.sh` gives null in each.
- Request 3 is complete. `/v1/node/capabilities` lists `remote_items` and
  `publisher` for tracks. The route and the include handling now read one
  list.
- The open question is answered in ADR 0049 section 3. A test proves that the
  publisher read gives `unresolved` after an album feed is deleted.

## Deferred, Not Requested Now

**A reverse album list.** The publisher view lists only the albums that the publisher feed lists (`load_publisher` in `src/query.rs`).
An album that names a publisher without a listing by it does not appear. Stophammer has a reverse query internally (`get_publisher_album_release_artists` in `src/db.rs`).

The v4vmm operator decided on 2026-09-25 not to request this list now, because Stophammer is correcting the relationship data.
`/v1/publisher-links/stats` shows the correction: unresolved links fell from 1,159 on 2026-09-24 to 156 on 2026-09-25.

## Answered, No Action

These items were open in earlier v4vmm documents. The Stophammer source or the live API answers each one.

| Item | Answer | Evidence |
|---|---|---|
| Is the `publisher` collection complete in one response? | Yes. The query has no limit and no paging | `load_publisher`, `get_feed_remote_items_for_feed` in `src/db.rs` |
| Why is `include=publisher` on a track empty? | The track view reads the item's own remote items. It is empty when the item states no publisher | `load_track_publisher`, ADR 0038 |
| The normalization rule of the artist count | Trim, collapse white space, Unicode lowercase. "feat." is not split | ADR 0049 §7 |
| `Feed.name`, `Track.name`, `Track.feed_url` | No longer in the responses. v4vmm deletes its readers | Live responses, 2026-09-25 |
| `Track.artist_credit` | Removed on purpose on 2026-04-08 | Commit `a16a720` |
| Search summary fields, separate track artwork, the publication-date label | Live since 2026-09-23 | v4vmm API change request, changes 1 to 3 |
| The `last_build_date` claim | No Stophammer action. v4vmm writes its own field rule | v4vmm |
| The publisher relationship fields | Live since 2026-09-24 | ADR 0049 |
