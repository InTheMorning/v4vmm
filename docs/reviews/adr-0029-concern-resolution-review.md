# ADR 0029 Concern Resolution Review

## Result

Pass - 2026-05-01.

## Scope

- `docs/adr/0029-artist-person-identity-persistence.md`
- `docs/plans/adr-0029-artist-person-identity-persistence-phase-plan.md`
- `docs/plans/deferred-architecture-work-index.md`
- `docs/reviews/adr-0029-review-checklist.md`
- `docs/tasks/adr-0029-task-002-artist-source-schema.md`
- `docs/reviews/adr-0029-task-002-review.md`
- `src/db.rs` as implemented by `ce9cc6b`

## Concerns Addressed

| Concern | Resolution |
|---|---|
| ADR scope mixed artist and person identity. | ADR title, context, decision, plan, and index now state artist identity only; person identity is deferred. |
| Person rows risk global identity without durable keys. | ADR keeps MusicIndex contributors and RSS `podcast:person` rows owner-scoped under ADR 0028 until durable person ids and merge policy exist. |
| Storage shape was underspecified. | ADR now names `artist_source_facts`, `artist_source_links`, and `artist_source_ids`, including JSON-array treatment for aliases/tags and relational treatment for links/ids. |
| Lifecycle and cascade behavior were unclear. | ADR and task docs now state artist source facts are source-keyed and do not cascade from feed or track deletion. |
| Local track-to-artist binding was implied too early. | ADR now explicitly says this ADR does not add a `tracks` binding column and defers name-derived Library artist hydration to a follow-up ADR. |
| Conflict policy referenced a nonexistent source-priority rule. | ADR now defers cross-source display priority and tie-breaking until a future binding ADR. |
| Done criteria were too broad. | ADR now defines done as local explicit-source artist lookup rendering known scalar artist facts from local data, while name-derived artists remain unchanged. |
| Future reviews could miss the narrowed scope. | ADR 0029 review checklist now checks person deferral, non-cascade behavior, and no track binding. |

## Code Alignment

The existing Task 002 implementation already matches the clarified ADR:

- `artist_source_facts` has no feed/track foreign key, so feed unsubscription
  does not cascade artist subjects.
- Replacement is keyed by explicit `(source, source_artist_id)`.
- There is no global person table.
- There is no local `tracks` artist-subject binding.
- Source links/ids are artist-specific child rows.

Because the implementation already matched the corrected direction, no
`git reset` was needed.

## Verification

Green on 2026-05-01:

- `cargo fmt -- --check`
- `cargo check`
