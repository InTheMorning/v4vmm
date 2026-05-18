# Library / Index data parity triage review checklist

## Reviewed Artifacts

- `docs/adr/0052-library-index-data-parity-triage.md`
- `docs/plans/library-discover-parity-triage-plan.md`
- `docs/tasks/library-discover-parity-triage-task-001-album-detail.md`
- `docs/tasks/library-discover-parity-triage-task-002-track-detail.md`
- `docs/tasks/library-discover-parity-triage-task-003-artist-playlist-detail.md`
- `docs/reviews/library-discover-parity-triage-album.md`
- `docs/reviews/library-discover-parity-triage-track.md`
- `docs/reviews/library-discover-parity-triage-artist-playlist.md`
- `docs/reviews/library-discover-parity-triage-synthesis.md`
- `docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md`
- `docs/adr/0053-local-detail-source-fact-parity.md`

## Required Checks

- Triage remains documentation-only; no `src/` files, migrations, or schema
  files change.
- Reports compare Library detail surfaces against live Index detail surfaces,
  not the parked `src/discover/` module.
- Every report includes file:line evidence for rendered fields, VM source,
  persistence status, and hydration path.
- Every named deferred field appears in the relevant inventory: release date,
  language, explicit state, description / annotation, and contributor identity.
- Every gap is routed to loading-shape, persistence, intentional asymmetry, or
  an explicit open question.
- Loading-shape gaps route to ADR 0024 follow-up work.
- Persistence gaps route to source-fact ADR work.
- Artist/person identity reconciliation gaps route to ADR 0029 or future
  person-identity work, not to ad hoc matching.
- Intentional asymmetries cite the relevant ADR or invariant.
- The synthesis artifact names every gap from the three reports.
- `docs/plans/deferred-architecture-work-index.md` is updated only after the
  synthesis artifact exists.

## Verification

- `git status --short`
- `git diff --check`

No cargo gate is required for documentation-only triage.

## Merge Recommendation

Merge only if the reports and synthesis provide complete routing evidence and
the deferred-work index no longer points at an unresolved triage gap.
