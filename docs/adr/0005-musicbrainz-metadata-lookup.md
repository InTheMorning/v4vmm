# ADR 0005: MusicBrainz Metadata Lookup

## Status
Accepted

## Context
The MusicIndex tag comparison workflow can now download a track and read embedded MP3 tags. Operators also need a way to ask MusicBrainz for a likely recording match using the same metadata-driven approach documented for Picard's Lookup action.

Picard has two identification modes: Lookup and Scan. Lookup searches MusicBrainz from existing metadata such as title, artist, album, track number, duration, and ISRC. Scan fingerprints audio through Chromaprint and AcoustID. The first integration should avoid fingerprinting and avoid mutating local tags; it should only surface candidate MusicBrainz matches for operator review.

## Decision
Add a small `musicbrainz` module that performs metadata-based recording lookup against the MusicBrainz `/ws/2/recording` API. The search UI will expose a MusicBrainz action on track inspectors, reusing the existing lazy background task pattern.

The first pass will:
- Build a Lucene-style recording query from embedded tags, falling back to MusicIndex track fields.
- Request MusicBrainz recording search results with artist credits, releases, release groups, and media.
- Score returned candidates with Picard-inspired weighted fields for title, artist, album, track number, and duration.
- Render candidates for review without applying, saving, or rewriting any metadata.

## Consequences
- MusicBrainz lookup is isolated from the MusicIndex API client and from tag comparison.
- The UI can add fingerprint-based Scan later without conflating it with metadata Lookup.
- Network lookups depend on MusicBrainz availability and must use a project user agent.
