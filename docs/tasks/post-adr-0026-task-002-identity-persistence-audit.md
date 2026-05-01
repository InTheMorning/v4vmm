# Post-ADR 0026 Task 002: Identity Persistence Audit

## Status

Planned.

## Goal

Determine whether identity facts exposed by MusicIndex and modeled by ADR 0026
survive local Library workflows without loss.

## Read

- `docs/adr/0026-shared-entity-projection-layer.md`
- `docs/plans/post-adr-0026-follow-up-plan.md`
- `../musicindex/api.json`
- `src/views.rs`
- `src/db.rs`
- `src/feed_service.rs`
- `src/rss/subscribe.rs`
- `src/audio_tags.rs`
- `src/metadata.rs`
- `migrations/`

## Files Likely to Change

- `docs/reviews/post-adr-0026-task-002-identity-persistence-audit.md`
- A follow-up ADR under `docs/adr/` only if the audit proves a schema or
  persistence contract change is needed.

## Do Not Touch

- Do not change database schema in this task.
- Do not change import, subscription, metadata, or audio-tag write behavior.
- Do not add inference for missing identity data.
- Do not collapse distinct source facts into a single guessed canonical value.

## Constraints

- Preserve provenance-first behavior: document source facts and conflicts
  instead of normalizing them away.
- Treat remote MusicIndex identity fields, local database rows, audio tag
  fields, and projection structs as separate contracts.
- Audit at least contributor `href`, `img`, `npub`, `source_links`, and
  `source_ids`.

## Implementation Steps

1. Compare MusicIndex artist, item, and contributor identity fields from
   `../musicindex/api.json` with the ADR 0026 projection types.
2. Trace which fields are stored during feed subscription, feed updates,
   library import, metadata editing, and audio-tag persistence.
3. Produce a preservation matrix covering remote source, local storage,
   projection availability, and known loss points.
4. Recommend whether the next step is no-op, bounded implementation, or a
   schema/persistence ADR.

## Acceptance Criteria

- A preservation matrix exists under `docs/reviews/`.
- The matrix explicitly covers contributor `href`, `img`, `npub`,
  `source_links`, and `source_ids`.
- Any proposed schema work is backed by a concrete data-loss path.
- No code or schema changes are made as part of the audit.

## Test Commands

```bash
cargo fmt -- --check
cargo check
```

These commands are required only if the task unexpectedly changes code. A pure
documentation audit should state that no runtime tests were run.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0026-shared-entity-projection-layer.md`
- `docs/plans/post-adr-0026-follow-up-plan.md`
- `../musicindex/api.json`
- `src/views.rs`
- `src/db.rs`
- `src/feed_service.rs`
- `src/rss/subscribe.rs`
- `src/audio_tags.rs`
- `src/metadata.rs`
- `migrations/`

Goal:
- Produce an identity-fact preservation audit for local Library workflows.

Constraints:
- Do not change schema or runtime behavior.
- Do not infer or canonicalize missing facts.
- Keep source facts and provenance separate in the matrix.

Do not touch:
- Runtime Rust files
- migrations
- ADR status fields

Acceptance criteria:
- `docs/reviews/post-adr-0026-task-002-identity-persistence-audit.md` exists.
- The audit includes a preservation matrix.
- Proven data-loss gaps are separated from unknowns and no-op cases.
- Any recommended ADR scope is precise.

Test commands:
- `cargo fmt -- --check`
- `cargo check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- `../musicindex/api.json` is unavailable or contradicts the local API client
  types.
- The audit reveals a required schema migration.
- The current metadata paths discard source facts in a way that needs an ADR
  before implementation.
