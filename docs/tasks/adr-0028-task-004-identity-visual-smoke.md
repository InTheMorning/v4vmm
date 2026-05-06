# ADR 0028 Task 004: Identity Visual Smoke

## Status

Implemented.

## Goal

Capture visual evidence that Library can render persisted ADR 0028 identity
source facts through the same GPUI-free projections as Discover when those
facts are locally available.

## Read

- `docs/adr/0028-local-identity-source-fact-persistence.md`
- `docs/plans/adr-0028-local-identity-source-fact-persistence-phase-plan.md`
- `docs/tasks/adr-0028-task-001-schema-and-db-helpers.md`
- `docs/tasks/adr-0028-task-002-ingest-persistence.md`
- `docs/tasks/adr-0028-task-003-local-view-hydration.md`
- `docs/reviews/post-adr-0026-task-001-visual-smoke-review.md`
- `docs/reviews/adr-0027-task-005-final-visual-smoke-review.md`
- `src/views.rs`
- `src/sources.rs`
- `src/search.rs`
- `src/library.rs`

## Files Likely to Change

- `docs/tasks/adr-0028-task-004-identity-visual-smoke.md`
- `docs/reviews/adr-0028-task-004-identity-visual-smoke-review.md`
- `docs/plans/adr-0028-local-identity-source-fact-persistence-phase-plan.md`
- Narrow UI display files only if smoke exposes a regression caused by ADR
  0028 hydration.

## Do Not Touch

- Do not change database schema or migrations.
- Do not change MusicIndex, RSS, subscription, download, playlist, or playback
  behavior.
- Do not write smoke data into the user's real library.
- Do not commit screenshot binaries.
- Do not redesign Library or Discover visuals.

## Constraints

- Use a copied config, database, and thumbnail cache under `/tmp`.
- Compare the same release in Library and Discover at the same viewport.
- Prefer a fixture with feed, track, and contributor identity facts. If the
  available fixture lacks contributor identity facts, record the gap instead of
  inferring identities.
- Verify identity affordances remain readable in the active dark profile:
  visible labels, distinguishable controls, and no icon-only state that depends
  on color alone.
- Keep GPUI, SQLite, service, and network dependencies out of shared projection
  modules.

## Implementation Steps

1. Build the app so the smoke pass uses the current binary.
2. Launch the app with isolated XDG config/data directories under `/tmp`.
3. Navigate to the same release in Library and Discover.
4. Capture same-viewport screenshots for both surfaces.
5. Compare visible identity affordances: source links, identity icons, website /
   Nostr actions, contributor rows, metadata density, and contrast.
6. Apply only bounded display fixes if ADR 0028 introduced a visual regression.
7. Write a review that references screenshot artifacts and records remaining
   gaps.

## Implementation Summary

- Launched the rebuilt app against copied config/data under
  `/tmp/v4vmm-adr28-identity-smoke`.
- Seeded only the copied database with explicit `adr-0028-smoke-fixture`
  source facts for `The Heycitizen Experience` so local Library hydration could
  be observed without writing to the user's real library.
- Wired Library album snapshots to carry persisted feed identity facts into
  `FeedView::from_local_with_identity`.
- Rendered shared Website/Nostr identity actions in both Library and Discover
  release-detail action slots.
- Captured Library and Discover release screenshots and wrote the visual smoke
  review.

## Acceptance Criteria

- [x] Library and Discover screenshots compare the same release.
- [x] The review records whether persisted identity links and ids render from
  local Library data.
- [x] The review records whether contributor identity facts are covered by the
  available fixture or remain a fixture gap.
- [x] Any remaining mismatch is classified as source-data availability,
  screen-owned behavior, or a new follow-up.
- [x] Screenshot artifacts are referenced in a review under `docs/reviews/`.
- [x] Required verification commands pass.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test identity_actions_are_shared_across_surface_contexts
cargo test sources::tests::local_source_fetch_feed_hydrates_feed_and_track_identity_facts
cargo test views::tests::from_local_feed_hydrates_identity_facts_and_contributors
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
cargo test
```

Run targeted tests for any runtime code changed by this task.

## Escalation Triggers

- Comparable Library and Discover release screenshots cannot be captured.
- The local fixture cannot represent any persisted ADR 0028 identity facts.
- A discovered mismatch requires schema, ingest, service, or projection changes
  instead of bounded display cleanup.
