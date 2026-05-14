# ADR 0044 Task 003: Playlist Reorder Guards and Visual Readiness

Status: Awaiting operator visual recheck after follow-up fixes on 2026-05-14.

## Goal

Add final guards, run verification, and record visual proof for playlist
drag-handle reordering.

## Files to Inspect

- `docs/adr/0044-playlist-drag-handle-reordering.md`
- `docs/plans/adr-0044-playlist-drag-handle-reordering-phase-plan.md`
- `docs/tasks/adr-0044-task-001-playlist-reorder-vm-contract.md`
- `docs/tasks/adr-0044-task-002-playlist-drag-shell.md`
- `docs/reviews/adr-0044-review-checklist.md`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `tests/architecture_tests.rs`
- `docs/reviews/adr-0044-review-checklist.md`
- Possibly minor fixes in playlist files touched by Tasks 001-002

## Do Not Touch

- Do not introduce new playlist behavior beyond guards/fixes needed for
  readiness.
- Do not alter database schema.
- Do not refactor unrelated Library or Search surfaces.

## Constraints

- Guards must map directly to ADR 0044 invariants.
- Visual proof must include light and dark themes.
- Visual proof must include a normal playlist row, an unavailable row,
  row Actions menu, and insertion-line feedback.
- Required checks must be green before recording `Proceed`.

## Implementation Steps

1. Done: add architecture guards that playlist rows do not render up/down
   arrow labels or arrow-specific ids.
2. Done: add guards that drag handle and menu fallback display come from
   `PlaylistTrackRowVm`.
3. Done: add guards that playlist shell uses the semantic icon catalog for the
   handle rather than raw glyph strings.
4. Done: run the required checks.
5. Pending: capture or review light/dark visual evidence after the
   2026-05-14 follow-up fixes.
6. Done: update the ADR 0044 review checklist with pass/fail, evidence, and
   merge recommendation.

## Acceptance Criteria

- [x] Architecture guards enforce the new handle/menu ownership contract.
- [x] Required checks are green.
- [ ] Visual proof confirms handle, menu fallback, unavailable row, and
  insertion line are legible in light and dark themes.
- [x] Review checklist records `Proceed` only if all gates pass.

## Verification

- Green: `cargo fmt -- --check`
- Green: `cargo check`
- Green: `cargo test`
- Green: `cargo clippy -- -D warnings`

## Visual Evidence Attempt

Initial visual proof was blocked because the local display could not be opened:

```text
DISPLAY=:0 wmctrl -l
Authorization required, but no authorization protocol specified
Cannot open display.
```

User screenshots and review on 2026-05-13 exposed readiness blockers:
playlist drops could leave playlist view without committing, playlist-origin
track removal did not return to the playlist or refresh unavailable rows in
place, and the Playlists `+` control shared the disclosure click target.

The fixes landed on 2026-05-13. The ADR 0044 review remains pending until
light and dark screenshots can verify the handle, Actions menu, unavailable
row, playlist-origin return path, successful in-place reorder drops, and
insertion-line feedback.

Follow-up user review on 2026-05-14 confirmed Back to Playlist and the
Playlists `+` target, but found that playlist reorder still needed a view
switch to refresh, the drop target still required an exact separator pixel,
and playlist-origin removal still did not refresh availability in place. The
follow-up fix refreshed the same-playlist paged listing and made ready rows
themselves drop targets with a stable top insertion border. Visual proof is
still required before marking this task or ADR 0044 complete.

A second 2026-05-14 user review confirmed reorder and removal commit, but
showed a placeholder/loading flash until mouse activity and ambiguous row
drop feedback. Fresh playlist actors now prime their first page from the
already loaded playlist rows before publishing, and row-level drag-over
feedback now keeps the cue to the insertion border rather than tinting the
whole destination row.

A third 2026-05-14 review confirmed the placeholder flash is gone and the
toolbar behavior is acceptable, but found a thin inactive insertion cue could
still appear where dropping does nothing. Drag-over feedback is now shown
only when `playlist_reorder_target` would produce a real move.

Operator follow-up clarified that row hover should quantize to the nearest
insertion point, always use the thick insertion cue, and only no-op for the
same place or outside the playlist. Row drop targets now quantize by drag
direction and use a tokenized thick insertion edge for active
feedback.

Follow-up correction: row hover now draws the cue on the actual insertion
edge. Downward drags draw the thick cue below the hovered row; upward drags
draw it above the hovered row.

HIG/architecture drift review on 2026-05-14 found the remaining separator
drop zones still overlapped the row-edge insertion mechanism, cross-playlist
drags lacked invalid-destination feedback, same-playlist reselects thrashed
the warm paged actor cache, and playlist return state was too narrow. The
separator drop zones were removed so row quantization is the only insertion
mechanism; invalid drags now use muted outlined row feedback with the system
no-drop cursor; same-playlist reselect sends `PagedTrackListMsg::Refresh` to
the existing actor; and track inspector return state now uses
`InspectorOrigin`.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test
cargo clippy -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0044-playlist-drag-handle-reordering.md`
- `docs/plans/adr-0044-playlist-drag-handle-reordering-phase-plan.md`
- `docs/reviews/adr-0044-review-checklist.md`
- `tests/architecture_tests.rs`

Goal:
- Add guards and visual-readiness evidence for playlist drag-handle
  reordering.

Constraints:
- Add no new feature behavior.
- Guards must map to explicit ADR 0044 invariants.
- Visual proof must cover light and dark themes.

Do not touch:
- Database schema
- Search/Discover UI
- Unrelated Library surfaces

Acceptance criteria:
- Guards enforce handle/menu ownership and no arrow controls.
- Required checks are green.
- Review checklist has a clear readiness decision.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test`
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- Visual proof shows ambiguous drop location or handle/body gesture
  conflict.
- Guards require broad fragile string baselines instead of direct
  ownership checks.
