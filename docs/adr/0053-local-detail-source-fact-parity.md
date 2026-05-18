# ADR 0053: Local detail source-fact parity

## Status

Proposed - 2026-05-17. Drafted by ADR 0052 triage.

## Context

ADR 0052 triaged Library / Index detail parity after ADR 0048 and ADR 0049.
The triage found two different classes of gaps:

- Loading-shape gaps, where a field is already persisted or fetched but is not
  projected into the active detail VM.
- Source-fact gaps, where Library parity would require storing a field that is
  not currently durable local data.

ADR 0028 solved identity source facts for links, ids, and contributors. It did
not define durable local source facts for release metadata, track descriptions,
publisher text, lyrics/annotations, artist biography, or playlist-level
language / explicit semantics.

## Decision

Do not close persistence gaps with renderer fallbacks, local inference, or
temporary fetch-only state. Any Library field that should remain visible after
navigation, restart, or offline use must first have a source-fact persistence
contract.

This ADR draft reserves the source-fact parity route for the gaps identified
by ADR 0052:

- Feed/release publisher.
- Feed/release kind.
- Feed/release date.
- Feed/release explicit state.
- Track publisher.
- Track description.
- Track language, if product scope requires track-level language.
- Track lyrics / annotation, if product scope requires them.
- Artist description / biography / annotation.
- Playlist language and playlist explicit state, if playlists become
  source-like entities rather than local-only lists.

Before implementation, a follow-up task must decide whether these fields belong
in existing entity tables, source-fact tables, or a new release/track metadata
fact table. The implementation must preserve provenance and must not collapse
remote MusicIndex facts, RSS facts, tag facts, and user-authored local facts
into one untraceable scalar.

## Invariants

- Non-empty source metadata is not hidden or reinterpreted in renderers.
- No Library detail field is made durable by deriving it from a weaker local
  value unless the ADR explicitly permits that derivation.
- MusicIndex `release_kind` is not assumed to equal RSS `podcast_medium`
  without an explicit mapping decision.
- Track description is distinct from feed description.
- Track annotation/lyrics are distinct from transcript URL and MusicBrainz
  release disambiguation text.
- Artist biography does not imply artist/person identity reconciliation.
- Playlist language and explicit state require product semantics before schema
  work because local playlists are currently local management objects.

## Non-Goals

- No schema migration in this ADR draft.
- No renderer changes.
- No remote Index fetch behavior changes.
- No artist/person merge policy.
- No local playlist publication model.

## Alternatives Considered

- **Reuse `extra_json` for every missing field.** Rejected as the default route.
  It can preserve raw payloads, but it does not create an auditable typed
  source-fact contract by itself.
- **Derive album release date from track pubdates.** Deferred. This may be a
  useful display policy, but it is not equivalent to a source-provided
  feed/release date unless explicitly approved.
- **Map `podcast_medium` to release kind automatically.** Deferred. The fields
  may overlap, but the mapping needs a documented compatibility rule.
- **Treat playlist language/explicit state like feed metadata.** Rejected for
  now. Local playlists are not modeled as published release/feed entities.

## Consequences

Positive:

- Follow-up work cannot smuggle source inference into renderer conditionals.
- Missing-field fixes remain provenance-first and survive navigation/restart.
- ADR 0028 identity facts stay scoped to identity rather than becoming a bag
  for unrelated metadata.

Negative / risks:

- User-visible parity remains incomplete until source-fact design lands.
- Some gaps may resolve as intentional non-goals after product semantics are
  clarified.

## Follow-Up Work

- Draft a concrete source-fact schema / migration ADR if any of the reserved
  fields become required product behavior.
- Decide `podcast_medium` versus MusicIndex `release_kind` mapping.
- Decide feed/release date derivation policy.
- Decide track language and track annotation product scope.
- Decide whether playlists ever carry source-like metadata.

## References

- ADR 0028 - Local identity source-fact persistence
- ADR 0052 - Library / Index data parity triage
- `docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md`
- `docs/reviews/library-discover-parity-triage-album.md`
- `docs/reviews/library-discover-parity-triage-track.md`
- `docs/reviews/library-discover-parity-triage-artist-playlist.md`
