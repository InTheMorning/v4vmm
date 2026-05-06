# ADR 0036 Task 003: Advanced Provenance Panel Consistency

## Goal

Give Library advanced compare, MusicBrainz, staged-tag, and provenance panels a
consistent panel grammar while preserving their dense source-specific content.

## Scope

This task must run after Tasks 001 and 002 are green. It should identify
repeated advanced panel layout and label policy, then move it to dedicated
panel VMs/composites.

## Acceptance Criteria

- Advanced panel layout is owned by shared panel composites, not screens.
- Source-specific labels remain allowed in provenance contracts.
- Normal track/feed summary labels stay owned by ADR 0035/0036 VMs.
- User screenshots verify advanced Library panels.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0036-feed-visual-and-provenance-surface-consistency.md`
- `docs/plans/adr-0036-feed-visual-and-provenance-consistency-phase-plan.md`
- `docs/tasks/adr-0036-task-003-advanced-provenance-panel-consistency.md`
- `src/library.rs`
- `src/metadata.rs`
- `src/ui/composites/musicbrainz_panel.rs`
- `src/view_models/musicbrainz_panel.rs`
- `tests/architecture_tests.rs`

Goal:
- Consolidate advanced Library provenance panel grammar without changing
  metadata behavior.

Constraints:
- Preserve MusicBrainz, compare, staged tag, and provenance workflows.
- Do not hide source-specific labels behind generic normal-detail labels.

Do not touch:
- Backend, schema, ID3/RSS parsing semantics, playlist semantics, playback
  semantics.

Acceptance criteria:
- Advanced panel repeated layout has one owner.
- Tests prevent screen-local panel grammar drift.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test`
- `cargo clippy -- -D warnings`
- `git diff --check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
