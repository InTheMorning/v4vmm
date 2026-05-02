# One Owner Per Surface Task 002: Fallback Display Accessors

## Goal

Move repeated screen-local fallback labels and empty-value coercions into
view-model display accessors, then add guards that prevent the literals and
coercions from returning in screens.

## Files to Inspect

- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/plans/one-owner-per-surface-plan.md`
- `src/library.rs`
- `src/search.rs`
- `src/ui_track.rs`
- `src/view_models/track.rs`
- `src/view_models/library.rs`
- `src/view_models/search.rs`
- `src/view_models/feed.rs` if present
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/view_models/track.rs`
- `src/view_models/feed.rs` or the existing feed VM owner
- `src/view_models/library.rs` or a new `src/view_models/playlist.rs`
- `src/library.rs`
- `src/search.rs`
- `src/ui_track.rs`
- `tests/architecture_tests.rs`
- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/reviews/one-owner-per-surface-review-checklist.md`

## Do Not Touch

- Backend schema, API, database, ingest, or MusicBrainz behavior.
- Broad screen layout or visual redesign.
- Existing fallback meaning without documenting the owner in the VM.

## Constraints

- Screens do not decide what an empty title, artist, album, playlist name, or
  feed URL means.
- View models stay GPUI-free and expose plain `String` or `Option<String>`.
- Use `Option<String>` where absence should render an empty state instead of
  a replacement label.
- Add unit tests for present, empty-string, and `None` cases for each new
  accessor.
- Add or extend architecture tests so screen files cannot reintroduce the
  removed fallback literals or `feed_url.unwrap_or_default` pattern.

## Implementation Steps

1. Start with `TrackVm::display_title`; reconcile with the existing
   `TrackVm::title()` behavior and the post-ADR 0033 track-header task.
2. Add `display_artist` and `display_album` to the track VM owner.
3. Add the playlist display-name owner or extend the existing playlist display
   contract used by `AddToPlaylistPopover` if appropriate.
4. Add the feed URL display owner as `Option<String>` if an empty UI state is
   semantically different from an empty string.
5. Replace the call sites listed in the plan and delete inline coercions.
6. Add architecture tests for forbidden screen fallback literals/coercions.
7. Update ADR 0033's enforcing-test list with the new test names.

## Acceptance Criteria

- `src/library.rs`, `src/search.rs`, and `src/ui_track.rs` do not contain the
  removed fallback policy for title, artist, album, playlist name, tag label,
  or feed URL.
- The fallback policy has one VM owner with focused unit tests.
- New architecture tests fail if screen-local fallback literals/coercions come
  back.
- ADR 0033 lists the new tests.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
cargo test display_
git diff --check
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/plans/one-owner-per-surface-plan.md`
- `docs/tasks/one-owner-per-surface-task-002-fallback-display-accessors.md`
- `src/library.rs`
- `src/search.rs`
- `src/ui_track.rs`
- `src/view_models/track.rs`
- `tests/architecture_tests.rs`

Goal:
- Hoist screen-local fallback labels and empty-value coercions into
  view-model accessors, with tests and architecture guards.

Constraints:
- Keep VMs GPUI-free.
- Do not invent metadata.
- Do not broaden layout changes.
- Each removed fallback must have one named owner.

Do not touch:
- Backend, API, schema, ingest, or playback code.
- Theme palettes.
- Unrelated screen helpers.

Acceptance criteria:
- Screen fallback literals/coercions are removed.
- VM accessors are unit-tested.
- Architecture tests guard the removed patterns.
- ADR 0033 enforcing-test list is updated.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --test architecture_tests`
- `cargo test display_`
- `git diff --check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- If a fallback has conflicting meanings across surfaces, stop and document
  the conflict instead of choosing silently.
- If a removed literal is still needed in a VM test, keep it scoped to the VM
  test and explain why the architecture test excludes it.
