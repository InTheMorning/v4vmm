# ADR 0007: Metadata Compare Table Drag Copy

## Status

Accepted

## Context

The MusicIndex track inspector renders a three-way metadata comparison between RSS/MusicIndex, embedded ID3, and MusicBrainz. Operators need to treat this comparison as an editable table: dragging a value from an outer source column into the middle ID3 column should stage an ID3v2.4-formatted value for that row.

The current implementation is a visual grid, not a table model. It has aligned row data, but no stable row identity and no edit overlay. The project also currently reads ID3 tags but does not write them.

## Decision

Introduce a table-oriented UI layer with stable row IDs and an in-memory pending ID3 edit overlay. RSS and MusicBrainz cells are drag sources. ID3 cells are drop targets when a row has a known ID3 frame target. Dropping a source value onto an ID3 cell stages a pending edit for that row and renders the middle column from the pending value.

This ADR does not add file writes. Persisting staged edits to MP3 files requires a separate explicit apply action and a dedicated ID3v2.4 writer boundary.

## Consequences

- Drag/drop cannot mutate local audio files implicitly.
- The compare grid can show staged ID3 edits immediately.
- Row identity is separated from user-facing labels.
- ID3 formatting stays behind a helper that understands frame labels such as `TXXX:...`, `WXXX:...`, and `UFID:...`.
- A future writer can consume the same staged edit model.
