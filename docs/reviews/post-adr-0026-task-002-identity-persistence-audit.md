# Post-ADR 0026 Task 002 Identity Persistence Audit

## Result

Pass with schema follow-up required if offline/local identity preservation is a
product requirement - 2026-05-01.

## Scope

- Reviewed `../musicindex/api.json`.
- Audited API types, local view facts, database rows, RSS subscription paths,
  metadata rows, and audio-tag write paths.
- Focused on contributor `href`, `img`, `npub`, `source_links`, and
  `source_ids`.

## Preservation Matrix

| Fact | Remote MusicIndex API | Local DB / Library | ID3 / Audio Tags | ADR 0026 Projections |
|---|---|---|---|---|
| Contributor `href` | Present on `api::Contributor`. | Not durably persisted from MusicIndex contributor rows. RSS `podcast:person` attrs may remain in raw `people_json`, but Library projections do not surface them. | Not written; contributor ID3 rows use name/role values. | Supported by `ContributorView.href`. |
| Contributor `img` | Present on `api::Contributor`. | Same as `href`: possibly retained only as raw RSS attrs, not projected into Library rows. | Not written; audio tags preserve embedded artwork, not contributor images. | Supported by `ContributorView.image_url`. |
| Contributor `npub` | Present on `api::Contributor`. | Not durably persisted from MusicIndex contributor rows. Feed/track Nostr enrichment is in-memory unless converted into existing tag workflows. | Track/feed Nostr can be written as `TXXX:RSS Nostr Handle`; contributor-level `npub` is not written. | Supported by `ContributorView.nostr_npub`. |
| `source_links` | Present on API feed and track detail. | No durable local structure for raw source links. Local rows keep scalar link/image fields and some extra JSON, but not full source-link provenance. | Website/transcript rows can be generated from fetched context, but raw link provenance is not preserved in tags. | Supported by `EntityIdentityLinks::source_links` when API facts are loaded. |
| `source_ids` | Present on API feed and track detail. | No durable local structure for raw source IDs. Local projections drop raw IDs. | Selected Nostr/GUID values may be written, but raw source ID provenance is not preserved. | Supported by `EntityIdentityLinks::source_ids` when API facts are loaded. |

## Concrete Data-Loss Paths

- `FeedView::from_api` and `TrackView::from_api` can preserve source facts,
  but `FeedView::from_local` and `TrackView::from_local` rebuild local identity
  primarily from scalar image fields and do not surface contributors.
- SQLite has scalar feed/track metadata, value JSON, `people_json`, and
  `extra_json`, but no normalized MusicIndex source-fact storage for
  `source_links`, `source_ids`, or contributor identity.
- `TrackRow` does not carry raw person JSON, so even RSS-retained contributor
  attrs cannot currently reach ADR 0026 local projections.
- ID3 persistence is intentionally lossy for provenance. It writes selected
  user-facing metadata rows, not the raw MusicIndex source-fact graph.

## Recommendation

Create a future schema/persistence ADR if the intended contract is:

- Library-local detail can render MusicIndex identity facts while offline.
- Local rows preserve source facts and provenance, not only convenience fields.
- Contributor identity facts survive subscription/import/update workflows.

A smaller bounded task can add missing API include parameters where remote
detail fetches under-request `source_links` or `source_ids`; that does not need
a schema ADR by itself.

## Verification

- Documentation audit only.
- No runtime code changed.
- No tests were run.
