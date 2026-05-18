# ADR 0047 Task 017: Final Guards and Visual Readiness

Status: Implemented - 2026-05-18.

## Goal

Add final architecture guards spanning Phases B-F and record visual
proof in `docs/reviews/adr-0047-review-checklist.md` covering every
new contract in light and dark themes.

## Files to Inspect

- `docs/plans/library-search-unification-plan.md`
- `docs/plans/adr-0047-library-search-unification-phase-plan.md`
- All tasks 001-016
- `tests/architecture_tests.rs`
- `docs/reviews/adr-0044-review-checklist.md` (checklist precedent)

## Files Likely to Change

- `tests/architecture_tests.rs`
- `docs/reviews/adr-0047-review-checklist.md` (new)
- Possibly minor fixes uncovered by visual review

## Do Not Touch

- New feature behavior beyond guards / fixes needed for readiness
- DB schema
- Playback, mpv driver

## Constraints

- Guards must map directly to ADR 0047 invariants.
- Visual proof must include light and dark themes.
- Required checks must be green before recording `Proceed`.

## Visual Proof Inventory

Capture screenshots or video evidence for:

1. Default workspace (SourceList + ContentList + Detail +
   QueueNowPlaying), light + dark.
2. ContentList frame with filter chip strip visible, light + dark.
3. ContentList frame at narrow width with filter chip strip
   collapsed to pull-down, light + dark.
4. ContentList frame with `Library` filter active and zero rows
   (empty-state), light + dark.
5. Library track inspector compact view (no expanded panels),
   light + dark.
6. Library track inspector with Compare ID3 expanded for a
   downloaded track, light + dark.
7. Compare ID3 + MusicBrainz disabled on an undownloaded track with
   tooltip visible, light + dark.
8. Description collapsed (long body), light + dark.
9. Description expanded (long body, user-toggled), light + dark.
10. Description default expanded (short body), light + dark.
11. Search submit producing a `Detail` frame with breadcrumb chrome,
    light + dark.
12. Search-results inspector — Artists tab, Feeds tab, Tracks tab —
    each tab populated, light + dark.
13. Search-results inspector empty state for a filter that yields
    zero rows, light + dark.
14. Saved search opened from source list, light + dark.
15. Breadcrumb middle-ellipsis truncation at narrow width.

## Implementation Steps

1. Audit all task acceptance criteria for guards that have not yet
   landed. Add any missing guards.
2. Create `docs/reviews/adr-0047-review-checklist.md` with sections:
   Overview, Required Checks, Visual Proof, Merge Recommendation.
3. Capture visual evidence per the inventory.
4. Record pass/fail per gate. Mark `Proceed` only if all pass.

## Acceptance Criteria

- [x] All Phase B-F invariants have at least one architecture guard.
- [x] Required checks (fmt, check, test, clippy) are green.
- [x] Visual proof captured in light and dark themes or superseded by ADR 0048
      ContentList-frame visual proof.
- [x] Review checklist records a clear `Proceed` / `Block` decision.

## Completion Notes

ADR 0047's Detail-frame search visual checklist is historical after ADR 0048.
The active search surface now lives in the ContentList frame, and the operator
closed the remaining visual smoke items during the 2026-05-18 completion pass.

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
- `docs/plans/library-search-unification-plan.md`
- `docs/plans/adr-0047-library-search-unification-phase-plan.md`
- Tasks 001-016 in `docs/tasks/`
- `tests/architecture_tests.rs`
- `docs/reviews/adr-0044-review-checklist.md`

Goal:
- Final architecture guards + visual-readiness checklist for ADR
  0047.

Constraints:
- Guards map directly to invariants.
- Visual proof covers light and dark.
- Required checks green before `Proceed`.

Do not touch:
- Production behavior beyond guards
- DB schema
- Playback

Acceptance criteria:
- Guards cover all invariants.
- Checklist records readiness decision with visual proof.

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

- Visual proof reveals HIG drift or invariant violation not covered
  by an earlier task (escalate; do not paper over with a guard).
