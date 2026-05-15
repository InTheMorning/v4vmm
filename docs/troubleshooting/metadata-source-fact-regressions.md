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
  empty text as missing source text at that boundary. Placeholder-only HTML or
  entity wrappers such as `<p>...</p>`, `&hellip;`, `&#8230;`, and
  whitespace-only `<br>` payloads are the same class of source-fact absence.
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
- **Sanitize at every read boundary too, not only at the merge step.** Local
  DB rows can carry placeholder text persisted by an earlier ingest path. A
  reader that projects `TrackRow`/`Feed` into the display `Track`/`Feed`
  contract must collapse placeholder text to `None` so the metadata grid
  cannot render historical pollution as a current source fact.

## Symptom shape

The most visible failure mode is an inspector where the compact summary card
shows clean text (it consumes `frame.title` and similar pre-cleaned strings)
while the wide metadata grid below renders literal `...` across most or all
RSS-column rows. The contradiction is the tell: both surfaces draw from the
same `TrackContext.track`, so a one-sided regression points to a missing
sanitization boundary on the noisier surface.

## Current Guards

`src/feed_service.rs` rejects placeholder-only MusicIndex text while merging
library track detail, then enriches the merged context from RSS before handing
it to Library detail renderers. The regression test
`library_track_context_rejects_placeholder_source_text_at_boundary` locks the
merge boundary.

`src/subscribe_service.rs::track_row_to_api_track` and
`src/feed_service.rs::track_row_to_feed` strip placeholder text from local
DB rows before they reach the display contract. Identity columns
(`track_guid`, `feed_guid`, `item_guid`) pass through verbatim; only display
facts are sanitized. The regression test
`local_track_row_strips_placeholder_text_at_projection_boundary` locks the
read boundary.

`src/feed_service.rs::apply_feed_updates` and `src/rss/subscribe.rs` skip
`db::set_feed_description` when the MusicIndex feed description matches
placeholder detection, so the DB never persists `...` as a feed description
in the first place.

`src/metadata.rs::sanitize_track_context_source_text` is the shared projection
guard for `TrackContext` display/source facts. Search inspectors, subscribe
flows, local library reads, persistence handoff, and compare-file reads sanitize
before building detail or metadata rows. `aligned_compare_rows` also refills
stale placeholder source values from the current `TrackContext`, so a loaded
compare panel cannot keep showing `...` after the underlying source context has
been repaired.

`src/rss/helpers.rs::clean_text` and `src/views.rs` API-to-view projections
also call the same source-placeholder classifier. This is not renderer masking:
RSS parsing and API projection are read boundaries where transport payloads are
converted into source/display facts. They must reject placeholder-only markup or
entity values before UI view models receive them.

Migration `cleanup_placeholder_source_text` repairs databases that were already
polluted by placeholder-only text. It only nulls placeholder-only payloads in
nullable feed and track display/source text columns; real text containing
ellipsis punctuation is preserved. Migration
`cleanup_markup_placeholder_source_text` repeats the cleanup for databases that
already ran the first cleanup before HTML/entity placeholder detection existed.
