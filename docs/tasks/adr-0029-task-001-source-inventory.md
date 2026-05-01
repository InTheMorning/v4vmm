# ADR 0029 Task 001: Source Inventory

## Status

Ready.

## Goal

Inventory artist/person identity facts across MusicIndex, RSS, local SQLite,
and existing projections before designing ADR 0029 schema.

## Read

- `docs/adr/0029-artist-person-identity-persistence.md`
- `docs/plans/adr-0029-artist-person-identity-persistence-phase-plan.md`
- `../musicindex/api.json`
- `src/api.rs`
- `src/views.rs`
- `src/sources.rs`
- `src/local_identity.rs`
- `src/db.rs`
- `src/identity_ingest.rs`
- `src/rss/subscribe.rs`
- `src/view_models/artist.rs`
- `docs/reviews/post-adr-0026-task-002-identity-persistence-audit.md`

## Files Likely To Change

- `docs/reviews/adr-0029-task-001-source-inventory-review.md`
- `docs/tasks/adr-0029-task-001-source-inventory.md`
- `docs/plans/adr-0029-artist-person-identity-persistence-phase-plan.md`

## Do Not Touch

- Do not change runtime Rust files.
- Do not add migrations.
- Do not change MusicIndex, RSS, Library, Discover, or metadata behavior.
- Do not infer identity matches.

## Constraints

- Separate explicit source ids from display names.
- Treat contributor position as source-order only, not durable identity.
- Record unknowns rather than filling them with assumptions.
- Recommend schema only after listing source fields and loss points.

## Implementation Steps

1. Compare MusicIndex artist/person/contributor fields in
   `../musicindex/api.json` with `src/api.rs`.
2. Trace local artist derivation in `src/sources.rs`, `src/views.rs`, and
   `src/view_models/artist.rs`.
3. Trace RSS person attributes in `src/rss/subscribe.rs`.
4. Produce a preservation matrix for image, website, Nostr, aliases, area,
   active years, source ids, and source links.
5. Recommend whether ADR 0029 should use one source-subject schema or separate
   artist/person schemas.

## Acceptance Criteria

- [ ] Review file exists under `docs/reviews/`.
- [ ] Preservation matrix covers MusicIndex artist, MusicIndex contributor, RSS
  person, local Library artist rows, and `ArtistView`.
- [ ] Explicit durable keys are separated from display-only names.
- [ ] Recommended schema direction is stated with risks.
- [ ] No runtime behavior changes.

## Test Commands

Documentation-only task. If runtime files change unexpectedly, run:

```bash
cargo fmt -- --check
cargo check
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0029-artist-person-identity-persistence.md`
- `docs/plans/adr-0029-artist-person-identity-persistence-phase-plan.md`
- `docs/tasks/adr-0029-task-001-source-inventory.md`
- `../musicindex/api.json`
- `src/api.rs`
- `src/views.rs`
- `src/sources.rs`
- `src/local_identity.rs`
- `src/db.rs`
- `src/identity_ingest.rs`
- `src/rss/subscribe.rs`
- `src/view_models/artist.rs`
- `docs/reviews/post-adr-0026-task-002-identity-persistence-audit.md`

Goal:
- Produce a source inventory and schema-direction recommendation for ADR 0029.

Constraints:
- Documentation only.
- No identity inference.
- Separate explicit durable keys from display-only names.
- Record unknowns.

Do not touch:
- Runtime Rust files
- migrations
- MusicIndex/RSS behavior
- Library/Discover UI behavior

Acceptance criteria:
- `docs/reviews/adr-0029-task-001-source-inventory-review.md` exists.
- Matrix covers MusicIndex artist, MusicIndex contributor, RSS person, local
  Library artist rows, and `ArtistView`.
- Schema direction and risks are stated.
- No runtime behavior changes.

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

- `../musicindex/api.json` is unavailable.
- MusicIndex has no explicit artist/person identity keys.
- The inventory reveals a runtime data-loss bug unrelated to ADR 0029.
