# Inspector Source Ownership Task 001

## Status

Implemented - 2026-05-18.

## Goal

Fix ContentList ownership regressions without redesigning the workspace:

- Content filters target the current inspector/detail, not the Library source
  tree.
- Index search rows drill down through remote identity.
- Removing a track from an album refreshes the mounted inspector and changes the
  removed row to a downloadable Index-origin row when possible.
- Downloading that row changes it back to Library-origin action state without
  requiring navigation.
- Settings tab visits do not reset the current Library/Search ContentList
  navigation state.
- Index artist/feed/track clicks push explicit breadcrumb-backed Index detail
  navigation entries instead of silently updating a Settings-only status string.

## Files To Inspect

- `src/app.rs`
- `src/library/app_impl.rs`
- `src/view_models/library.rs`
- `src/view_models/search_results.rs`
- `src/view_models/entity_detail.rs`
- `src/view_models/workspace.rs`
- `src/ui/shells/workspace.rs`
- `tests/architecture_tests.rs`
- `docs/adr/0049-inspector-source-ownership.md`
- `docs/plans/inspector-source-ownership-phase-plan.md`

## Files Likely To Change

- `src/app.rs`
- `src/library/app_impl.rs`
- `src/view_models/library.rs`
- `src/view_models/search_results.rs`
- `src/view_models/entity_detail.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- Database migrations.
- Playlist/playback/ID3 compare behavior.
- Discover module deletion.
- Workspace tab topology (`Library`, `Settings` only).
- Raw renderer-only fixes that hide stale membership state.

## Constraints

- Do not let renderers infer membership or source origin.
- Do not parse `index-*` IDs as SQLite IDs.
- Do not use the Library source tree as the owner of `All / Library / Index`
  filtering.
- Same-view mutation must update the mounted inspector before navigation.
- Preserve source facts; do not drop remote rows just because local membership
  changed.

## Implementation Steps

1. Add or strengthen architecture guards for the three ownership rules.
2. Rewire `TopApp::set_frame_filter` so ContentList filter selection delegates
   to the active inspector/detail VM instead of filtering the Library tree.
3. Make Index search-result activation route by origin and remote identity.
4. Update album remove/download success callbacks to refresh or prime the
   currently mounted detail.
5. Extend the relevant Library/detail VM so removed local tracks can remain as
   Index-origin rows with `Download` under `All`/`Index`.
6. Preserve Library/Search ContentList state when Settings is opened and closed.
7. Add `IndexArtistDetail`, `IndexFeedDetail`, and `IndexTrackDetail`
   navigation entries. Index artist activation shows Index feeds with a
   breadcrumb back to the search result; uncached Index feed/track activation
   pushes an Index detail entry and renders an Index detail surface.
8. Run focused tests, then full project gates.

## Acceptance Criteria

- [x] Selecting `Library / Index / All` in ContentList does not hide/show rows in
  the left Library tree.
- [x] Index-only search results drill down without "invalid id" status.
- [x] Removing a track from a library album changes the mounted row action from
  `Remove` to `Download` when remote identity is available.
- [x] Downloading that row changes the mounted row action back from `Download` to
  `Remove`.
- [x] Navigating away and back to the album does not permanently hide remote-known
  removed tracks under `All`/`Index`.
- [x] Switching to Settings and back to Library restores the previous Library/Search
  state.
- [x] Clicking an uncached Index feed/track pushes `IndexFeedDetail` or
  `IndexTrackDetail` and renders an Index detail surface with breadcrumb back to
  the search result.
- [x] Clicking an Index artist pushes `IndexArtistDetail` and shows Index feeds
  with breadcrumb back to the search result.
- [x] Architecture tests guard against reintroducing the broken ownership.

Implementation notes: `IndexArtistDetail` was renamed to
`IndexArtistFeedScope` during follow-up because the implemented UI is a scoped
Index feed list, not a full remote artist detail page.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo clippy -- -D warnings
cargo test --lib search_results --features async-runtime
cargo test --test architecture_tests --features async-runtime
cargo test
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0049-inspector-source-ownership.md`
- `docs/plans/inspector-source-ownership-phase-plan.md`
- `src/app.rs`
- `src/library/app_impl.rs`
- `src/view_models/library.rs`
- `src/view_models/search_results.rs`
- `src/view_models/entity_detail.rs`
- `tests/architecture_tests.rs`

Goal:
- Fix ContentList inspector source ownership so filters target the inspector,
  Index rows drill down by remote identity, and album remove/download mutations
  update the current inspector in place.

Constraints:
- Do not redesign workspace frames.
- Do not move ownership into GPUI renderers.
- Do not parse `index-*` IDs as local database IDs.
- Preserve Library source tree as local navigation only.
- Preserve source facts and remote rows.

Do not touch:
- Migrations.
- Playlist/playback/ID3 compare behavior.
- Discover module deletion.
- Tab topology.

Acceptance criteria:
- Content filters do not mutate the Library source tree.
- Index-only search results drill down through remote identity.
- Index artist/feed/track activation pushes `IndexArtistDetail`,
  `IndexFeedDetail`, or `IndexTrackDetail`; uncached feed/track activation
  renders an Index detail surface.
- Removed album tracks become remote-only/downloadable where Index identity is
  available.
- Same-view inspector state updates on mutation success.
- Focused and full tests pass.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo clippy -- -D warnings`
- `cargo test --lib search_results --features async-runtime`
- `cargo test --test architecture_tests --features async-runtime`
- `cargo test`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- A schema migration appears necessary.
- Remote drill-down cannot be represented by existing or new
  `IndexArtistDetail`, `IndexFeedDetail`, and `IndexTrackDetail` detail
  surfaces.
- Persisted identity facts are missing for a common removed-track path.
- GPUI visual verification is still blocked by local X11/GPU initialization.
