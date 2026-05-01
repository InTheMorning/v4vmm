# Post-ADR 0026 Task 001: Visual Parity Smoke

## Status

Planned.

## Goal

Capture comparable visual evidence for the Discover and Library
release-detail surfaces so later work is driven by observed mismatches, not by
visual preference alone.

## Read

- `docs/adr/0026-shared-entity-projection-layer.md`
- `docs/plans/post-adr-0026-follow-up-plan.md`
- `docs/adr/0025-theme-and-style-boundary.md`
- `src/ui_entity.rs`
- `src/ui_feed.rs`
- `src/library.rs`
- `src/search.rs`
- `src/view_models/entity_detail.rs`

## Files Likely to Change

- `docs/reviews/post-adr-0026-task-001-visual-smoke-review.md`
- Screenshot artifacts referenced by the review, if the task runner stores
  them in the repository.

## Do Not Touch

- Do not change runtime UI code.
- Do not change projection structs or screen state.
- Do not create ADR 0027 from visual preference alone.
- Do not change database, service, download, playlist, or playback code.

## Constraints

- Compare the same release content in Discover and Library at the same
  viewport size.
- Include or create a scenario that exercises contributor image, website, and
  Nostr identity data when feasible.
- Classify each mismatch as one of:
  - styling or contrast mismatch
  - missing shared projection or action state
  - screen-owned service, fetch, or handler behavior
  - data preservation or artwork-source gap
- Keep GPUI, service, and image-cache behavior out of shared projection
  modules.

## Implementation Steps

1. Build or launch the app using the repository's normal smoke-test workflow.
2. Navigate to the same release in Discover and Library.
3. Capture same-viewport screenshots for both surfaces.
4. Compare sidebar behavior, density, metadata ordering, action prominence,
   track rows, contrast, and redundant state labels.
5. Write a review that links or names each screenshot artifact and assigns each
   mismatch to the follow-up track that owns it.

## Acceptance Criteria

- A visual smoke review exists under `docs/reviews/`.
- The review references Discover and Library screenshots captured at the same
  viewport.
- Every observed mismatch is classified and routed to a follow-up track.
- No new architecture ADR is proposed unless the review identifies a durable
  boundary change.

## Test Commands

```bash
cargo fmt -- --check
cargo check
```

Run broader verification only if the task changes runtime code, which should
be treated as a deviation requiring review.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0026-shared-entity-projection-layer.md`
- `docs/plans/post-adr-0026-follow-up-plan.md`
- `docs/adr/0025-theme-and-style-boundary.md`
- `src/ui_entity.rs`
- `src/ui_feed.rs`
- `src/library.rs`
- `src/search.rs`
- `src/view_models/entity_detail.rs`

Goal:
- Produce a screenshot-backed visual parity review for Discover and Library
  release-detail surfaces.

Constraints:
- Do not change runtime UI code.
- Compare the same content at the same viewport.
- Classify each mismatch by owning follow-up track.
- Do not create a new ADR unless a durable boundary change is identified.

Do not touch:
- `src/views.rs`
- `src/view_models/entity_detail.rs`
- database, service, download, playlist, or playback modules

Acceptance criteria:
- `docs/reviews/post-adr-0026-task-001-visual-smoke-review.md` exists.
- Discover and Library screenshot artifacts are referenced.
- Each mismatch is routed to a follow-up track.
- `cargo fmt -- --check` and `cargo check` pass if runtime code is touched.

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

- The app cannot be launched or navigated with the available smoke workflow.
- Comparable Discover and Library content cannot be prepared.
- The evidence suggests a projection, query, schema, or artwork contract
  change rather than styling cleanup.
