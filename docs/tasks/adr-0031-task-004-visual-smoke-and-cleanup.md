# ADR 0031 Task 004: Visual Smoke and Cleanup

## Status

Completed with residual fixture gaps - 2026-05-02.

## Goal

Verify ADR 0031 visually on the representative fixture list, document the
smoke results, confirm screen-owned behavior still works, and remove obsolete
local composition paths only after the contract path is active.

## Files To Inspect

- `docs/adr/0031-release-detail-presentation-contract.md`
- `docs/plans/adr-0031-release-detail-presentation-contract-phase-plan.md`
- `docs/tasks/adr-0031-task-001-contract-types-and-projection-tests.md`
- `docs/tasks/adr-0031-task-002-renderer-adoption.md`
- `docs/tasks/adr-0031-task-003-track-section-parity.md`
- `src/view_models/entity_detail.rs`
- `src/ui_entity.rs`
- `src/ui_feed.rs`
- `src/library.rs`
- `src/search.rs`

## Files Likely To Change

- `docs/reviews/adr-0031-visual-smoke.md`
- `src/ui_entity.rs`
- `src/ui_feed.rs`
- `src/library.rs`
- `src/search.rs`

## Do Not Touch

- `src/db.rs`
- `migrations/`
- service modules
- unrelated UI surfaces

## Constraints

- Cleanup follows verification; do not remove fallbacks before screenshots and
  tests prove the contract path is active.
- Screenshots or screenshot paths must be referenced from the review document.
- Keep ADR 0031 non-goals intact.
- Exercise every fixture listed in ADR 0031 on Library and Discovery where
  applicable.
- Regression-check Library compare, download, playlist add, MusicBrainz lookup,
  and playback from the new contract path.

## Implementation Steps

1. Run the ADR fixture list in representative Library and Discovery
   release-detail views:
   - release with Website, Nostr, and a multi-paragraph description
   - release with an empty description
   - release with zero tracks
   - release with 100+ tracks
   - release with only podcast/RSS identity
   - Library release with full local-file metadata
2. Capture screenshots for initial release detail, action/fact areas, and track
   section start.
3. Verify description appears once and raw identity values are demoted.
4. Regression-check Library compare, download, playlist add, MusicBrainz
   lookup, and playback from the new contract path.
5. Remove obsolete local composition paths that are no longer used.
6. Write `docs/reviews/adr-0031-visual-smoke.md` with pass/fail notes,
   screenshot references, and residual risks.

## Acceptance Criteria

- [x] First viewport has a clear title, creator, restrained actions, compact
  facts, and visible start of the track section when content exists.
- [x] Description appears once.
- [x] Raw identity values are available only in demoted panels or copy/open
  actions.
- [x] Library compare, download, playlist add, MusicBrainz lookup, and playback
  remain screen-owned from the new contract path; MusicBrainz and playlist
  paths were visually exercised, while compare/playback were code-path
  reviewed.
- [x] Dead `ReleaseDetailSlots` fields, helpers, and screen-local conditionals
  superseded by the contract are removed.
- [x] Screenshots are attached or referenced from a review document.
- [x] Cleanup does not change service or data semantics.

## Implementation Summary

Task 004 ran an isolated visual smoke pass against the current binary using a
copied config/database under `/tmp/v4vmm-adr31-smoke` and user-attached
screenshots. The smoke covered Library baseline release detail, Library
playlist overlays, Library MusicBrainz lookup state, Discover same-release
detail with Website/Nostr/RSS identities, Discover one-track detail, and
Discover recent feeds.

The pass is documented in `docs/reviews/adr-0031-visual-smoke.md`. Residual
fixture gaps remain for zero-track, 100+ track, and proven multi-paragraph
description releases because those fixtures were not available in the smoke
set.

Cleanup removed the stale release-specific `EntityHeaderVm`,
`ReleaseDetailVm::header`, `ReleaseDetailVm::detail_rows`, and
`header_data_rows` path from `src/view_models/entity_detail.rs`. That old path
could still project description and raw identity rows into a pre-contract
header shape even though runtime rendering now uses `ReleaseDetailPageVm`.

Verification completed:

- `cargo fmt -- --check`
- `cargo check`
- `cargo test view_models::entity_detail`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`
- `git diff --check`

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test view_models::entity_detail
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0031-release-detail-presentation-contract.md`
- `docs/tasks/adr-0031-task-004-visual-smoke-and-cleanup.md`
- `src/view_models/entity_detail.rs`
- `src/ui_entity.rs`
- `src/ui_feed.rs`
- `src/library.rs`
- `src/search.rs`

Goal:
- Verify ADR 0031 visually and perform bounded cleanup.

Constraints:
- Document screenshots or screenshot paths in `docs/reviews`.
- Exercise the ADR fixture list.
- Regression-check Library compare, download, playlist add, MusicBrainz lookup,
  and playback from the new contract path.
- Remove only obsolete composition code proven unused by the contract path.
- Do not change service, database, playback, playlist, download, or
  subscription behavior.

Do not touch:
- `src/db.rs`
- `migrations/`
- service modules

Acceptance criteria:
- Visual smoke review exists.
- Description appears once.
- Raw identity values are demoted out of the hero.
- Behavior regressions listed in ADR 0031 are checked.
- Final checks are green.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test view_models::entity_detail`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
