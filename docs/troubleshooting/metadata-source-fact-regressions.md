# Metadata Source-Fact Regressions

## Purpose

Prevent repeat fixes that hide bad metadata at render time instead of correcting
the source-fact boundary.

## Prohibited Fix

Do not patch Library/Search renderers, composites, or display view-models to
treat non-empty strings like `...` as absent. That masks provenance bugs,
discards possible source data, and can hide the difference between MusicIndex,
RSS, ID3, and MusicBrainz facts.

## Required Mitigation

- Confirm which boundary admitted the bad value: MusicIndex API fetch, RSS
  hydration, local DB persistence, ID3 read, or MusicBrainz lookup.
- Correct the earliest boundary that can distinguish placeholder transport data
  from real source facts.
- Treat `...`, Unicode ellipsis-only values, multiline ellipsis payloads, and
  empty text as missing source text at that boundary.
- Prefer feed-scoped track fetches when both feed and track identifiers are
  available. Unscoped track GUID lookup can be ambiguous and can hydrate an
  inspector with the wrong or incomplete source facts.
- Preserve real RSS item/feed values by re-reading RSS when MusicIndex detail
  data is incomplete. This includes core visible facts such as title, artist,
  album/feed title, track number, release date, duration, artwork, and
  description, not only auxiliary links.
- Invalidate loaded compare panels and reload source context after
  download/remove actions. Metadata panels must never keep a pre-change
  `TrackContext` after the local library state changes.
- Keep display code simple: render the source-fact state it receives.
- Add a unit or architecture test that proves placeholder-looking transport
  values cannot override real local/RSS facts.

## Current Guard

`src/feed_service.rs` rejects placeholder-only MusicIndex text while merging
library track detail, then enriches the merged context from RSS before handing
it to Library detail renderers. The regression test
`library_track_context_rejects_placeholder_source_text_at_boundary` locks this
behavior.
