# ADR 0027 Task 005: Final Visual Smoke

## Status

Implemented.

## Goal

Capture final visual evidence for ADR 0027 by comparing the same release in
Library and Discover after shared entity action state, metadata action state,
and destructive row treatment are implemented.

## Read

- `docs/adr/0027-shared-entity-action-state.md`
- `docs/plans/adr-0027-shared-entity-action-state-phase-plan.md`
- `docs/tasks/adr-0027-task-001-track-row-action-state.md`
- `docs/tasks/adr-0027-task-002-release-action-state.md`
- `docs/tasks/adr-0027-task-003-metadata-action-state.md`
- `docs/tasks/adr-0027-task-004-destructive-row-control.md`
- `src/view_models/library.rs`
- `src/search.rs`
- `src/library.rs`

## Files Changed

- `src/view_models/library.rs`
- `src/view_models/search.rs`
- `src/search.rs`
- `docs/adr/0027-shared-entity-action-state.md`
- `docs/plans/adr-0027-shared-entity-action-state-phase-plan.md`
- `docs/tasks/adr-0027-task-005-final-visual-smoke.md`
- `docs/reviews/adr-0027-task-005-final-visual-smoke-review.md`

## Do Not Touch

- Do not change database schema or migrations.
- Do not change command handlers, service behavior, or network behavior.
- Do not write smoke data into the user's real library.
- Do not commit screenshot binaries.

## Constraints

- Use a copied config, database, and thumbnail cache under `/tmp`.
- Compare the same release at the same viewport.
- If visual smoke finds an ADR 0027 target-state miss, fix that bounded issue
  before recording the pass.
- Keep final screenshots local and reference their paths in the review.

## Implementation Summary

- Launched a rebuilt app against copied config/data under
  `/tmp/v4vmm-adr27-1777665559`.
- Captured Library and Discover screenshots for `The Heycitizen Experience`.
- Removed the redundant Library album `Downloaded` detail row because Library
  membership is now represented by release and row removal actions.
- Updated Discover track-row membership controls to render descriptor labels
  through native row-control styles instead of icon-only download/remove
  controls.
- Wrote the final visual smoke review.

## Acceptance Criteria

- [x] Library and Discover screenshots compare the same release.
- [x] Release-level membership and playlist actions use shared labels.
- [x] Row membership actions use shared labels and quiet destructive treatment.
- [x] Library release detail no longer shows redundant downloaded count.
- [x] Screenshot artifacts are referenced in a review under `docs/reviews/`.
- [x] Required verification commands pass.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test album_detail_vm_omits_downloaded_count_when_membership_actions_cover_state
cargo test track_row_action_vm_labels_match_download_state
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
cargo test
```

## Escalation Triggers

- Comparable Library and Discover release screenshots cannot be captured.
- A remaining mismatch requires command, service, schema, or query changes.
- Visual fixes require new design tokens rather than existing ADR 0025 roles.
