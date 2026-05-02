# ADR 0035 Task 005: Guards and Visual Gate

## Goal

Add the named ADR 0035 regression guards for track surface ownership and
complete screenshot-based visual smoke for Library and Discover rows,
inspector panes, and full-detail track surfaces.

## Files to Inspect

- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/adr/0035-track-surface-consolidation.md`
- `docs/plans/adr-0035-track-surface-consolidation-phase-plan.md`
- `tests/architecture_tests.rs`
- `src/library.rs`
- `src/search.rs`
- `src/ui/composites/track_detail_surface.rs`
- `src/ui/composites/track_inspector_pane.rs`
- `src/ui/composites/track_row.rs`
- `docs/reviews/adr-0035-review-checklist.md`

## Files Likely to Change

- `tests/architecture_tests.rs`
- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/adr/0035-track-surface-consolidation.md`
- `docs/reviews/adr-0035-review-checklist.md`

## Do Not Touch

- Runtime code unless a guard exposes a real remaining violation.
- Backend, schema, services, playlist behavior, playback behavior

## Constraints

- Tests must guard structure, not a single symptom.
- Do not use xdotool or coordinate automation. Ask for screenshots.
- Do not mark proceed until visual smoke passes or residual risk is explicit.
- Keep allowlists narrow and documented.
- Use the exact named tests from ADR 0035 unless a follow-up ADR renames them.
- Guards must cover row, inspector-pane, detail surface, labels, fallbacks,
  typed slots, and VM consumption.

## Implementation Steps

1. Add `screens_do_not_define_local_track_detail_surface_chrome`.
2. Add `screens_do_not_define_local_track_row_chrome`.
3. Add `screens_do_not_construct_track_inspector_pane_locally`.
4. Add `track_surface_consumers_use_track_detail_vm`.
5. Confirm the four guards already landed in earlier tasks remain at
   baseline zero:
   `screens_do_not_inline_unknown_artist_or_album_fallbacks`,
   `screens_do_not_inline_untitled_fallback`,
   `track_detail_labels_owns_canonical_field_labels` (Task 001), and
   `track_surface_slots_are_typed` (Task 002). Do not re-add.
6. Delete the legacy `TrackRow` (and any legacy `TrackInspectorPane`)
   constructors introduced as transitional adapters in Task 002. Tasks 003
   and 004 are required to have migrated every caller; if any caller still
   compiles against the legacy constructor, stop and route the residual
   caller through the VM constructor here rather than keeping the
   transitional API alive.
7. Resolve the `src/ui_track.rs` dual-ownership question. After Tasks 003
   and 004, `ui_track.rs` should no longer own track inspector chrome —
   that chrome lives in `TrackInspectorPane`. Either:
   - empty `ui_track.rs` to a thin re-export and remove it from
     `KNOWN_SHARED_UI_SHELL_FILES`; or
   - if the file still owns non-track-surface shell logic, document in its
     module-level doc comment exactly what remains and why it is not the
     track surface composite.
   Do not leave both `ui_track.rs` and `TrackInspectorPane` owning track
   inspector chrome.
8. Keep `TrackDetailSurface`, `TrackInspectorPane`, and `TrackRow`
   backend-free and screen-free under ADR 0033 shared-UI boundary guards.
9. Update ADR 0033 enforcing-test list to include all eight ADR 0035
   guards. (All eight are permanent per ADR 0035; no conditional.)
10. Run full checks.
11. Ask the user for screenshots only for the gates Tasks 003 and 004 did
    *not* already capture in their per-task visual smoke. The Task 005 gate
    is the side-by-side comparison:
    - Library full detail vs Discover full detail for the same track,
    - Library inspector pane vs Discover inspector pane for the same track,
    - Library detail with advanced panels open (if touched in Task 004),
    - Discover detail with lazy sections visible (if touched in Task 003).
    Confirm shared structure across surfaces; per-surface regressions
    should already have been resolved in Tasks 003/004.
12. Record pass/fail and readiness in the review checklist.

## Acceptance Criteria

- Architecture tests enforce one track row owner, one inspector-pane owner, one
  full-detail surface owner, one label/fallback owner, typed slots, and VM
  consumption.
- Legacy `TrackRow` (and any legacy `TrackInspectorPane`) constructors from
  Task 002 are deleted. Only the VM-driven constructors remain.
- `src/ui_track.rs` is no longer a dual owner of track inspector chrome —
  either emptied and removed from `KNOWN_SHARED_UI_SHELL_FILES`, or its
  remaining responsibilities are documented in its module doc.
- ADR 0033 "Enforcing tests" list includes all eight ADR 0035 guards.
- Full checks are green.
- User screenshots verify Library and Discover share recognizable track row,
  inspector, and detail structure while preserving surface-specific
  capabilities.
- Review checklist records `Proceed` only after visual smoke passes.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
cargo test
cargo clippy -- -D warnings
git diff --check
```

Note: `cargo test track_detail` (used in Tasks 001/003/004) is a substring
filter and will also match `track_detail_surface`, `track_detail_labels`,
`track_detail_load_state`, etc. That is intentional — every track-detail
test must pass — but reviewers who want only the VM unit tests should run
`cargo test --lib view_models::track_detail::`.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0035-track-surface-consolidation.md`
- `docs/plans/adr-0035-track-surface-consolidation-phase-plan.md`
- `docs/tasks/adr-0035-task-005-guards-and-visual-gate.md`
- `tests/architecture_tests.rs`
- `src/library.rs`
- `src/search.rs`
- `src/ui/composites/track_detail_surface.rs`

Goal:
- Add the named ADR 0035 ownership guards and complete visual readiness for
  track surface consolidation.

Constraints:
- No pointer automation.
- Tests guard structure, not a single visible symptom.
- Runtime code changes only if a real remaining violation is found.
- Use the ADR 0035 test names exactly.

Do not touch:
- Backend/schema/service files.
- Playlist/playback behavior.

Acceptance criteria:
- Architecture guards pass and cover row, inspector, detail, labels,
  fallbacks, typed slots, and VM consumption.
- Legacy transitional constructors are deleted.
- ADR 0033 lists all eight ADR 0035 guards.
- Full checks pass.
- Review checklist records screenshot-based gate status.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo test`
- `cargo clippy -- -D warnings`
- `git diff --check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- If visual smoke shows Library or Discover lost useful actions, row
  structure, inspector structure, labels, or sections, stop and create a
  targeted slot-fix task.
- If the guard requires a broad allowlist, stop and refine the migration.
