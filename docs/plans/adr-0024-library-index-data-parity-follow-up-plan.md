# ADR 0024 Library / Index Data Parity Follow-Up Plan

## Status

Implemented - 2026-05-18. Produced by ADR 0052 triage and completed through
the six loading-shape/runtime slices ending in readiness guard commit
`de934bb`.

## Goal

Route every Library / Index visible-field parity gap found by
`docs/plans/library-discover-parity-triage-plan.md` into the correct
downstream owner without changing runtime code during triage.

This plan owns **loading-shape** follow-up work: fields that are already
persisted or fetched but are not surfaced through the local query, VM, or live
Index detail shape.

Source-fact / persistence gaps route to
`docs/adr/0053-local-detail-source-fact-parity.md`.

## Non-Goals

- No implementation in this plan commit.
- No schema migration.
- No new renderer-only fallback labels.
- No artist/person identity reconciliation.
- No revival of `src/discover/`.

## Assumptions

- The live Index detail path is `SearchResultsInspector` +
  `IndexDetailDisplay`, not the parked Discover module.
- ADR 0049 owns inspector-source identity and same-view mutation behavior.
- ADR 0024 owns local read-model and application query shape.
- ADR 0053 or a successor source-fact ADR must land before any field that lacks
  local source persistence is made a parity requirement.

## Affected Modules For Future Work

- `src/application/queries/**`
- `src/db.rs`
- `src/library/app_impl.rs`
- `src/view_models/library.rs`
- `src/view_models/search_results/index_detail.rs`
- `src/view_models/search_results/results.rs`
- `src/app/search_dispatch.rs`
- `src/ui/shells/search_results_inspector.rs`
- Shared entity/track/artist/playlist shells only after VM contracts exist.

## Triage Inputs

- `docs/reviews/library-discover-parity-triage-album.md`
- `docs/reviews/library-discover-parity-triage-track.md`
- `docs/reviews/library-discover-parity-triage-artist-playlist.md`

## Routing Summary

| Bucket | Count | Downstream owner |
| ------ | ----- | ---------------- |
| Loading-shape | 25 | ADR 0024 follow-up tasks from this plan |
| Persistence / source facts | 12 | ADR 0053 source-fact parity |
| Intentional asymmetry | 10 | Documented; no implementation packet |
| Open questions | 13 | Resolve before implementation packets that depend on them |

## Gap Routing Matrix

### Album / Release Detail

| Gap | Triage route | Downstream route |
| --- | ------------ | ---------------- |
| Publisher | persistence | ADR 0053 feed/release source facts |
| Release kind | persistence | ADR 0053 feed/release source facts; decide `podcast_medium` mapping |
| Release date | persistence | ADR 0053 feed/release source facts |
| Explicit state | persistence | ADR 0053 feed/release source facts |
| Language | loading-shape | ADR 0024 feed query / `FeedRow` / `AlbumNode` projection |
| Fallback source metadata | intentional asymmetry | Keep Index-only fallback provenance chrome |

### Track Detail

| Gap | Triage route | Downstream route |
| --- | ------------ | ---------------- |
| Library summary release / album context | loading-shape | Rich Index track-detail VM shape |
| Track number | loading-shape | Preserve fetched/local track number in detail VMs |
| Duration | loading-shape | Preserve fetched/local duration in detail VMs |
| Release date | loading-shape | Load `tracks.pub_date` and preserve `api::Track::pub_date` |
| Publisher | persistence | ADR 0053 track source facts |
| Description | persistence | ADR 0053 track source facts |
| External website action | loading-shape | Index detail identity/action slot after VM support |
| Copy Nostr action | loading-shape | Index detail identity/action slot after VM support |
| Library track command actions | intentional asymmetry | Keep local-only command surface |
| Index Source row | intentional asymmetry | Keep remote provenance row |
| Index ID row | intentional asymmetry | Keep compact Index provenance row |
| Metadata grid core source rows | loading-shape | Shared track detail / metadata projection decision |
| Metadata grid release date and RSS item pubdate | loading-shape | Load local pubdate and preserve Index pubdate |
| Metadata grid description | persistence | ADR 0053 track source facts |
| Metadata grid language | persistence | ADR 0053 language scope decision |
| Metadata grid explicit state | loading-shape | Surface existing `tracks.itunes_explicit` / `api::Track::explicit` |
| Metadata grid lyrics / annotation | persistence | ADR 0053 track annotation source facts |
| Metadata grid contributor identity | loading-shape | Preserve contributors in Index detail shape |
| Metadata grid website / Nostr source facts | loading-shape | Preserve source links/ids in Index detail shape |
| Metadata grid transcript fields | loading-shape | Preserve transcript source links in detail shape |
| Metadata grid value routes | loading-shape | Preserve payment routes if product scope includes them |
| Metadata grid ID3 compare rows | intentional asymmetry | Keep local-file inspection only |
| Metadata grid MusicBrainz lookup rows | intentional asymmetry | Keep local metadata-lookup diagnostics only |
| Metadata grid release status / explicit-like state | intentional asymmetry | Do not treat MusicBrainz status as explicit state |
| Metadata grid MusicBrainz release note / annotation | intentional asymmetry | Do not treat MusicBrainz disambiguation as track annotation |
| Hero artwork parity | loading-shape | Add thumbnail to `IndexDetailDisplay` track shape |

### Artist Detail

| Gap | Triage route | Downstream route |
| --- | ------------ | ---------------- |
| Artist header identity | loading-shape | Decide/build dedicated Index artist detail page |
| Local artist summary facts | loading-shape | Dedicated Index artist detail shape or documented no-goal |
| Source fact rows: sort name, area, active years, website, aliases | loading-shape | Dedicated Index artist detail shape after identity constraints |
| Linked releases / feeds | loading-shape | Replace scoped result list only if dedicated detail is approved |
| Description / biography / annotation | persistence | ADR 0053 artist source facts |
| Explicit state | intentional asymmetry | Keep explicitness feed/track-scoped unless product semantics change |
| External identifiers | loading-shape | Render hydrated ids only without implying canonical merge |

### Playlist Detail

| Gap | Triage route | Downstream route |
| --- | ------------ | ---------------- |
| Playlist header and local actions | loading-shape | No Index packet until MusicIndex playlist entity exists |
| Playlist summary facts | loading-shape | Local VM surfacing only; Index blocked on playlist entity |
| Playlist track list rows | loading-shape | No Index packet until MusicIndex playlist entity exists |
| Created / modified dates | loading-shape | Optional local playlist detail VM rows |
| Description / annotation | loading-shape | Optional local playlist detail VM row |
| Language | persistence | ADR 0053 playlist product/source semantics |
| Explicit state | persistence | ADR 0053 playlist product/source semantics |
| Release date | intentional asymmetry | Local playlists are not release-like entities today |

## Proposed Sequence

1. **Feed loading-shape slice.**
   Load persisted `feeds.language` through the local query/read model into
   `AlbumNode` and `FeedView::from_local_with_identity`. Guard that Library
   album detail can render the shared language summary fact without touching
   renderer conditionals.
2. **Track detail loading-shape slice.**
   Replace compact Index track fallback with a richer GPUI-free detail VM shape
   that preserves fetched title, artist, release context, thumbnail, track
   number, duration, pubdate, explicit state, identity facts, contributors,
   transcript links, and value routes when product scope allows them. Keep
   local-only commands and ID3/MusicBrainz diagnostics out of Index.
3. **Track local projection slice.**
   Load persisted `tracks.pub_date` and `tracks.itunes_explicit` into local
   track projections and metadata rows. Do not add track description,
   publisher, language, or annotation until ADR 0053 resolves persistence.
4. **Artist detail decision slice.**
   Decide whether `IndexArtistDetail` is a real remote artist detail page or a
   scoped feed-result list. If it becomes a detail page, introduce an
   origin-aware artist detail VM that does not collapse name-derived local
   artists or imply person identity reconciliation.
5. **Playlist local-detail slice.**
   If product wants local playlist metadata visible, surface already-persisted
   `created_at`, `updated_at`, and `description` through `PlaylistDetailVm`.
   Do not invent Index playlist parity unless MusicIndex playlist entities
   exist.
6. **Readiness gate.**
   Add architecture guards for: no renderer-only field inference, no Discover
   dependency, no Index ID parsed as local ID, and every surfaced parity field
   arriving through a VM/query contract.

Each slice needs its own task packet before implementation.

## Implemented Runtime Slices

- `6e61d4f` - feed language loading and integration.
- `f9bff8d` - rich Index track detail rendering and integration.
- `d7d0220` - track `pub_date` and explicit projection.
- `e8c1aaa` - Index artist route decision: `IndexArtistFeedScope`.
- `8f701d2` - local playlist detail metadata rendering.
- `de934bb` - final ADR 0024 loading-shape readiness guard.

The persistence/source-fact gaps remain governed by ADR 0053/0054 or future
source-fact ADRs; they are not active ADR 0024 loading-shape work.

## Open Questions Before Implementation

- Should local album release date be derived from `tracks.pub_date`, or only
  from a true feed/release source fact?
- Should `feeds.podcast_medium` map to MusicIndex `release_kind`, or should
  MusicIndex release kind be persisted separately?
- Should Index track drill-down use the shared track detail surface, or remain
  a compact provenance detail?
- Is track-level description a durable local source fact, or only fetched
  enrichment context?
- Is language feed/release-level only, or is track-level language required?
- Should existing RSS `itunes_explicit` be surfaced directly, or normalized
  into source-fact tables first?
- Are payment/value routes in Index track parity scope?
- Should per-track contributor identity be visible on Index detail now, or
  deferred to person-identity work?
- Should `IndexArtistDetail` become a true artist detail page?
- How should name-derived local artists expose external ids without implying a
  canonical merge?
- Which future ADR owns artist contributors and durable person keys?
- Does MusicIndex have or plan playlist result/detail entities?
- Should Library playlist detail show local `created_at`, `updated_at`, and
  `description`?

## Test Strategy For Future Packets

- Unit tests for each VM projection that gains a field.
- Architecture tests that block GPUI renderers from inventing missing fields.
- Architecture tests that keep `src/discover/` out of live parity work.
- Existing `cargo fmt -- --check`, `cargo check --quiet`,
  `cargo test --lib --quiet`, `cargo test --test architecture_tests --quiet`,
  and `cargo clippy --quiet -- -D warnings` for implementation slices.
- Manual visual smoke remains operator-run only.

## Rollback Strategy

Each future implementation slice must be independently revertible. Loading
fields into VMs should be additive. If a surfaced field proves semantically
wrong, revert the slice that introduced that field and leave the source-fact
ADR unresolved rather than hiding the field in a renderer.

## References

- ADR 0024 - Command, query, and event application layer
- ADR 0049 - Inspector source ownership
- ADR 0052 - Library / Index data parity triage
- ADR 0053 - Local detail source-fact parity
- `docs/reviews/library-discover-parity-triage-album.md`
- `docs/reviews/library-discover-parity-triage-track.md`
- `docs/reviews/library-discover-parity-triage-artist-playlist.md`
