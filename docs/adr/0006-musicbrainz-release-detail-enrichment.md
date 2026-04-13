# ADR 0006: MusicBrainz Release Detail Enrichment

## Status
Accepted

## Context
ADR 0005 added metadata-based MusicBrainz recording lookup for the MusicIndex comparison workflow. That first pass intentionally stored a compact candidate model from recording search results. Operators now need more MusicBrainz provenance fields beside RSS and embedded metadata, such as label, barcode, release status, packaging, release-group type, ISRC, and URL relationships.

Picard's lookup flow treats recording search as the match step and then loads richer release data for the selected release/recording pair. The v4vmm UI still must avoid mutating local tags or inferring missing facts.

## Decision
After recording-search candidates are scored and truncated, v4vmm will fetch MusicBrainz release detail for candidates that have a release id. Release-detail fetches are cached by release id during a lookup. If a detail fetch fails, the candidate remains usable with the search-level fields.

The enriched candidate model may surface additional MusicBrainz source facts in the comparison UI, but those facts remain read-only and explicitly MusicBrainz-sourced.

## Consequences
- MusicBrainz lookup makes up to one additional release request per displayed release candidate.
- The UI can show richer MusicBrainz facts without adding inference or tag-writing behavior.
- Partial MusicBrainz failures degrade to the prior compact candidate data instead of failing the whole lookup.
