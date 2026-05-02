# One Owner Per Surface Task 001: Recents Surface Ownership

## Goal

Make Discovery recent-feed tiles structurally safe: visible title and
artist/publisher labels come from one view-model contract, and any repeated
tile chrome moves to one shared composite instead of living as screen-local
render code.

## Context

The visible `...` recent-feed regression is a canary. The structural issue is
not the exact placeholder text; it is that a user-facing tile can lose its
display contract or fallback policy without a guard failing.

## Files to Inspect

- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/plans/one-owner-per-surface-plan.md`
- `docs/tasks/adr-0030-task-002-recents-labels.md`
- `docs/reviews/adr-0030-task-002-review.md`
- `src/search.rs`
- `src/view_models/search.rs`
- `src/ui/composites/mod.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/search.rs`
- `src/view_models/search.rs`
- `src/ui/composites/recent_feed_tile.rs` if tile chrome repeats or carries
  policy
- `src/ui/composites/mod.rs` if a new composite is added
- `tests/architecture_tests.rs`
- `docs/reviews/one-owner-per-surface-review-checklist.md`

## Do Not Touch

- Backend API response shape unless a deserialization test proves the UI lacks
  source facts.
- Playlist, playback, metadata write, MusicBrainz lookup, or release-detail
  behavior.
- Theme palettes or unrelated tile layout.

## Constraints

- The fix must strengthen at least one HI structure contract from ADR 0033:
  shared ownership, view-model display contract, token/component discipline,
  regression guard, or visual proof.
- Do not hard-code better-looking placeholders in `src/search.rs`.
- Do not invent metadata. Labels must come from source fields or an existing
  view-model fallback policy.
- If a shared composite is added, it must be backend-free and accept
  display-ready inputs only.
- The tile must not render bare `...` for title or subtitle when a source
  title, artist, or publisher exists.

## Implementation Steps

1. Read `RecentFeedTileVm` and `render_recent_feeds_tiles` side by side.
2. Add or strengthen unit tests proving current `/v1/feeds/recent`-shaped
   responses hydrate title and artist/publisher labels.
3. If `render_recent_feeds_tiles` owns repeated tile chrome or policy-bearing
   label fallback, extract `RecentFeedTile` under `src/ui/composites/`.
4. Route the screen through the VM/composite contract. The screen should wire
   click behavior and image handles only.
5. Add an architecture or focused unit test that fails if recent tiles can use
   a screen-local `...` label path again.
6. Capture visual smoke for Discovery recents and record the result in the
   review checklist.

## Acceptance Criteria

- Discovery recent-feed tiles show stable title and artist/publisher labels
  from `RecentFeedTileVm` or a named display contract.
- No screen-local `...` fallback remains for recent tile title or subtitle.
- Any repeated tile chrome has one owner under `src/ui/composites/`.
- Tests prove the regression class cannot return silently.
- The implementation updates the review checklist with pass/fail status and
  visual-smoke notes.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
cargo test recent_feed_tile
git diff --check
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/plans/one-owner-per-surface-plan.md`
- `docs/tasks/one-owner-per-surface-task-001-recents-surface-ownership.md`
- `src/search.rs`
- `src/view_models/search.rs`
- `tests/architecture_tests.rs`

Goal:
- Make Discovery recent-feed tiles structurally safe so labels come from one
  VM/display contract and cannot regress to screen-local `...` placeholders.

Constraints:
- No backend/API redesign unless a source-fact deserialization test proves it
  is required.
- No screen-local placeholder patching.
- Shared UI stays backend-free and screen-free.
- Use existing tokens, buttons, labels, images, and composites.

Do not touch:
- Playlist behavior.
- Playback behavior.
- Metadata write paths.
- Theme palettes.

Acceptance criteria:
- Recent tiles render title and artist/publisher labels through a VM/composite
  contract.
- No screen-local recent-tile `...` fallback remains.
- A test or architecture guard fails if the regression returns.
- Visual smoke is recorded in the review checklist.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo test recent_feed_tile`
- `git diff --check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- If `/v1/feeds/recent` no longer provides title or artist/publisher source
  facts, stop and create a backend/API task rather than inventing labels.
- If fixing recents requires changing shared card/grid primitives used by
  other screens, stop and split the primitive work into its own task.
