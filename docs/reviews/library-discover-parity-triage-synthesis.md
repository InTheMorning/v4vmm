# Library / Index Data Parity Triage Synthesis

## Status

Complete - 2026-05-17.

## Inputs Reviewed

- `docs/adr/0052-library-index-data-parity-triage.md`
- `docs/plans/library-discover-parity-triage-plan.md`
- `docs/reviews/library-discover-parity-triage-album.md`
- `docs/reviews/library-discover-parity-triage-track.md`
- `docs/reviews/library-discover-parity-triage-artist-playlist.md`

## Result

Pass with routing split.

The triage completed its documentation-only scope and produced enough
file:line evidence to close the deferred-work index's **triage** requirement
for Library / Discover data parity.

Runtime parity is not complete. The next implementation work is split:

- Loading-shape gaps route to
  `docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md`.
- Persistence / source-fact gaps route to
  `docs/adr/0053-local-detail-source-fact-parity.md`.
- Intentional asymmetries do not get implementation packets.
- Open questions must be resolved before downstream task packets are written.

## Required Fixes Before Runtime Work

- Create one task packet per downstream slice from the ADR 0024 follow-up plan.
- Resolve source-fact semantics in ADR 0053 before adding columns, migrations,
  or local durable metadata fields.
- Resolve whether `IndexArtistDetail` is a true remote detail page or a scoped
  result-list state.
- Resolve whether Index track drill-down should use the shared track detail
  surface or remain compact provenance.

## Architectural Drift

None found in the triage artifacts. The reports keep `src/discover/` as
reference-only and do not propose renderer-side inference.

## Missing Tests

No Rust tests are required for documentation-only triage. Future runtime
packets need projection/unit tests and architecture guards per the follow-up
plan.

## Merge Recommendation

Merge the triage docs. Do not implement parity fixes from this worktree without
new task packets.
