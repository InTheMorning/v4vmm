# Active-Frame Search Dispatch Task 003: Entity Detail Text Filters

Status: Implemented - 2026-05-16.

## Goal

Add GPUI-free text filter mutators to entity-detail page view-models that render
track rows, so a later active-frame dispatcher can narrow the focused detail
surface without renderer-local query state.

## Files to Inspect

- `docs/plans/active-frame-search-dispatch-plan.md`
- `docs/tasks/active-frame-search-dispatch-task-001-workspace-descriptor.md`
- `src/view_models/track_detail.rs`
- `src/view_models/feed.rs`
- `src/view_models/artist_detail.rs`
- `src/view_models/playlist_detail.rs`
- `src/view_models/paged_feed_detail.rs`
- `src/view_models/paged_playlist_detail.rs`
- `src/view_models/library.rs`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `src/view_models/track_detail.rs`
- `src/view_models/feed.rs`
- `src/view_models/artist_detail.rs`
- `src/view_models/playlist_detail.rs`
- `src/view_models/paged_feed_detail.rs`
- `src/view_models/paged_playlist_detail.rs`
- `src/view_models/library.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/app.rs`
- `src/app/tab_bar.rs`
- `src/ui/*`
- `src/view_models/workspace.rs`
- `src/view_models/search_results.rs`
- `src/view_models/queue_now_playing.rs`
- Backend, database, playback, and network modules

## Constraints

- VM only. Do not wire dispatch from toolbar or frame chrome.
- Only VMs that already own or project track rows should get text filtering.
- Do not invent source facts or infer metadata validity from placeholder-looking
  text.
- Empty or whitespace-only input clears the text filter.
- Filtering must narrow displayed track rows by display-owned text only.
- If a named file has no track-list VM in the current code, document that in the
  final report instead of creating a fake VM.

## Implementation Steps

1. Inventory the named entity-detail VM files and identify which structs own
   track rows.
2. Add `set_text_filter(Option<String>)` and `text_filter()` to the applicable
   page/detail VMs.
3. Preserve original row collections so clearing the filter restores the full
   list.
4. Apply filtering only in VM projection methods that already return row
   displays.
5. Add focused unit tests for each changed VM.
6. Add or strengthen an architecture guard proving entity-detail text filters
   live in view-models, not renderers.

## Acceptance Criteria

- [x] Applicable detail VMs expose typed text-filter mutators.
- [x] Clearing a text filter restores the original track row projection.
- [x] Whitespace-only filters clear state.
- [x] VMs without track rows are explicitly left unchanged.
- [x] No app, backend, database, playback, or search-results files change.

## Implementation Notes

- `FeedVm` now owns optional text filtering for its sorted track projection.
- `PlaylistDetailVm` and `PlaylistDetailPageVm` now expose text-filter
  mutators while preserving original playlist positions for commands.
- `TrackDetailVm` / `TrackDetailPageVm`, `ArtistDetailPageVm`,
  `LibraryAlbumDetailVm`, `PagedFeedDetailVm`, and `PagedPlaylistDetailVm` were
  intentionally left unchanged because they do not currently own an eager
  track-row projection that can be filtered without renderer, paging actor, or
  backend coupling.
- `LibraryArtistDetailVm` was left unchanged because it projects feed summaries
  rather than a track-row list in the current detail surface.
- The only UI file touched in Phase 1 was a compile-only
  `QueueNowPlayingPageVm` destructuring update after queue VM state gained
  private fields; no toolbar or detail search dispatch wiring was added.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test track_detail
cargo test feed
cargo test artist_detail
cargo test playlist_detail
cargo test paged_feed_detail
cargo test paged_playlist_detail
cargo test library
cargo test --test architecture_tests
cargo clippy -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/plans/active-frame-search-dispatch-plan.md`
- `docs/tasks/active-frame-search-dispatch-task-001-workspace-descriptor.md`
- `src/view_models/track_detail.rs`
- `src/view_models/feed.rs`
- `src/view_models/artist_detail.rs`
- `src/view_models/playlist_detail.rs`
- `src/view_models/paged_feed_detail.rs`
- `src/view_models/paged_playlist_detail.rs`
- `src/view_models/library.rs`
- `tests/architecture_tests.rs`

Goal:
- Add GPUI-free text filter mutators to applicable entity-detail VMs that
  already own track row projections.

Constraints:
- VM only. Do not wire toolbar submit or render controls.
- Do not edit workspace descriptor code; another task owns it.
- Do not add metadata inference or source-fact filtering.
- Empty or whitespace-only input clears the filter.
- Leave files unchanged when they do not own track rows.

Do not touch:
- `src/app.rs`
- `src/app/tab_bar.rs`
- `src/ui/*`
- `src/view_models/workspace.rs`
- `src/view_models/search_results.rs`
- `src/view_models/queue_now_playing.rs`
- Backend, database, playback, and network modules

Acceptance criteria:
- Applicable entity detail VMs expose `set_text_filter(Option<String>)`.
- Tests prove set and clear behavior.
- Original row data is preserved.
- Any intentionally unchanged file is reported with the reason.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test track_detail`
- `cargo test feed`
- `cargo test artist_detail`
- `cargo test playlist_detail`
- `cargo test paged_feed_detail`
- `cargo test paged_playlist_detail`
- `cargo test library`
- `cargo test --test architecture_tests`
- `cargo clippy -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- A detail surface renders track rows only in GPUI renderer code.
- Filtering would require DB queries or source-fact mutation.
- Any change appears to require `src/app.rs` or `src/ui/*`.
