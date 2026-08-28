# ADR 0011: Persist MusicIndex Feed and Track GUIDs in ID3 TXXX Frames

## Status

Accepted - 2026-04-21.

## Context

The RSS-to-ID3 staging flow already writes MusicIndex-derived metadata such as contributors, value routes, transcript references, and publisher fields into explicit ID3v2.4 targets. Operators also need stable MusicIndex feed and track identifiers embedded in downloaded files so those files can be matched back to source entities later.

## Decision

Add compare/staging rows for the feed and track GUID source facts and map them to dedicated ID3v2.4 TXXX descriptors:

- `RSS feed guid` -> `TXXX:MusicIndex Feed Guid`
- `RSS track guid` -> `TXXX:MusicIndex Track Guid`

These rows participate in the existing RSS-to-ID3 auto-staging path, so subscription downloads and explicit compare-table apply actions write the same canonical tags.

## Consequences

- Downloaded MP3 files retain stable MusicIndex source identifiers.
- GUID tagging stays behind the existing ID3 writer boundary from ADR 0008.
- The compare table exposes these identifiers as ordinary staged metadata rows instead of special-case write logic.
