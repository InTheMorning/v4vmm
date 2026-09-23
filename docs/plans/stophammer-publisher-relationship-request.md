# Stophammer Publisher Relationship Request

## Status

Draft - 2026-09-23. The operator gave the decisions below on 2026-09-23.
This document is a request from the v4vmm client. Stophammer records its own decision in its own ADR.
It binds nothing until that Stophammer ADR is accepted.

The [research note](../notes/2026-09-23-publisher-feed-artist-research.md) holds the evidence.

## Purpose

Stophammer must report publisher relationships as facts that RSS states.
Each derived value must be a separate field that names its derivation.
Clients then build artist and label pages without a guess about the host.

## What RSS States

| Fact | RSS element | Reliability |
|---|---|---|
| The publisher of an album | Album `<podcast:publisher><podcast:remoteItem medium="publisher">` | Reliable on Wavlake and by specification |
| The identity of a publisher | Publisher feed `podcast:guid`, title, image, value block, `podcast:txt` | Reliable |
| The albums that a publisher lists | Publisher feed `<podcast:remoteItem medium="music">` | `feedUrl` is reliable. On Wavlake, `feedGuid` is the Wavlake URL identifier, not the album `podcast:guid` |
| The writer of an album feed | Album `itunes:owner` name | Reliable. Wavlake albums state "Wavlake" |
| The artist of an album | Album `itunes:author` | Reliable |
| The role of a listed album | Non-standard `rel` on `remoteItem` | Two known writers. The Podcast Namespace does not define it |

RSS does not state whether a publisher is an artist or a label, unless a non-standard `rel` states it.

## Current Stophammer Behavior

The references are to the local checkout at commit `2436e2a`.

1. Stophammer does not fetch the publisher feed that an album names. The live API returns HTTP `404` for two sampled Wavlake artist feeds.
2. `has_reciprocal_music_remote_item` in `src/api.rs` matches a back-link by `feedGuid` only. No Wavlake back-link can match.
3. For a Wavlake album, `publisher_repair_text` and the feed ingest write the fixed text "Wavlake".
   `publisher_repair_release_artist` copies the publisher feed title into `release_artist`.
4. Commits `984b78c` (2026-04-09) and `8dc3789` (2026-04-19) added the Wavlake rule. No Stophammer ADR records it.

## Operator Decisions

| # | Decision |
|---|---|
| 1 | Nobody contacts Wavlake. The back-link resolution for Wavlake is permanent |
| 2 | `release_artist` comes from `itunes:author` and `publisher_text` from `itunes:owner`. The publisher feed title becomes a separate derived field |
| 3 | Stophammer fetches each publisher feed that an album names, and resolves each listed album through its `feedUrl` |
| 4 | Stophammer stores raw `rel`, reports a role with its source, and reports a derived count of distinct artist names for each publisher |

## Requested Route

Each step is one Stophammer packet under one Stophammer ADR. Each step needs the steps before it.

1. **Decision record.** A Stophammer ADR states that publisher relationships are RSS facts. It supersedes the unrecorded Wavlake rule.
2. **Publisher feed ingest.** When an album names a publisher feed, Stophammer fetches that `feedUrl` and indexes the publisher feed.
   It uses the existing host pacing, the HTTP `429` backoff, and conditional GET.
3. **Back-link resolution.** For each album that a publisher feed lists:
   - When the listed `feedGuid` matches an indexed feed, the link is resolved by GUID.
   - Otherwise, Stophammer fetches the listed `feedUrl` and reads its `podcast:guid`.
   - Stophammer stores the declared values and the resolved GUID side by side, with the fetch time.
   - A link that it cannot resolve stays unresolved. Stophammer never resolves a link from the URL layout.
4. **Relationship facts.** For each album and publisher pair, the API reports:
   - whether the album names the publisher,
   - whether the publisher lists the album,
   - whether both are true,
   - how the publisher-side link was resolved: by GUID, by a `feedUrl` fetch, or unresolved.

   A publisher can list an album that names a different publisher. Stophammer keeps that link as "listed by", not as ownership.
5. **Text fields.** `release_artist` = `itunes:author`. `publisher_text` = `itunes:owner` name.
   A new derived field carries the publisher feed title and names its source. The host rule and the copy rule are deleted.
6. **Role.** Stophammer stores the raw `rel` value from the side that states it, marked as non-standard.
   It reports `role` and `role_source`. A stated `rel` gives `role_source = rel`. A missing `rel` gives `role = artist` and `role_source = default`.
7. **Artist count.** For each publisher, Stophammer reports the number of distinct `itunes:author` values across the albums that name it. The field is marked as derived.
   The Stophammer ADR states the normalization rule for this count.
8. **Corrective pass.** The `refresh` mode of Stophammer ADR 0047 runs the new ingest with Wavlake pacing.
   The pass reports the count of unresolved back-links, so that a silent Wavlake format change is visible.
9. **Tests from real feeds.** The Stophammer tests use these cases:
   - the DETOX Wavlake album and artist feed, with the wrong back-link GUID,
   - an artist-run RSS Blue or Fountain publisher feed,
   - the Sir Libre Records label feed with `rel="label"`,
   - the Jimmy V producer link to an album that names a different publisher,
   - an album with no publisher.

## Cost

These counts come from the local Stophammer database, dated 2026-04-19. The deployed data can differ.

| Work | Requests on the first pass |
|---|---:|
| Publisher feeds, all hosts | About 1,642 |
| Publisher feeds on Wavlake | About 1,520 |
| Wavlake album fetches for back-link resolution | About 6,652 |

At 2 seconds for each Wavlake request, the first Wavlake pass takes several hours.
Later passes send conditional requests for new or changed feeds only.

## Open Questions For The Stophammer ADR

- The normalization rule for the artist count. "feat." credits and spelling differences can make one artist count as more than one.
- The retention of a resolution after the album feed changes or disappears.
- The API names and ADR 0044 schemas for the new fields.

## What v4vmm Does After This Change

v4vmm keys an artist page on the publisher feed GUID.
It shows `role` and uses the artist count to select an artist page or a label page.
A separate v4vmm ADR replaces the dead artist-identifier binding of v4vmm ADR 0045.
