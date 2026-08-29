# Inspector Source Ownership Phase Plan

## Status

Implemented - 2026-05-18. Runtime ownership fixes, architecture guards, and
operator visual smoke are complete under ADR 0049.

## Goal

Fix the ContentList ownership regressions introduced during ADR 0048:

- Index-only search rows drill down into remote-backed detail instead of local-ID
  parsing.
- The ContentList filter controls the visible inspector/detail surface, not the
  Library source tree.
- Removing a track from a library album updates the mounted inspector in place.
  removed tracks remain visible as Index-origin rows under `All`/`Index` with a
  `Download` action when MusicIndex identity is available.
- Settings tab visits preserve the prior Library/Search ContentList state.

## Non-Goals

- No schema migration.
- No Discover module deletion.
- No global filter store.
- No change to playlist, playback, ID3 compare, or MusicBrainz behavior except
  preserving current inspector state after membership mutations.

## Assumptions

- Local tracks already carry enough MusicIndex/RSS identity facts to request
  remote feed/track detail in common album cases.
- `ContentFilter` remains the shared enum in `src/view_models/workspace.rs`.
- `SearchResultsInspectorPageVm` already owns search-result filter state and is
  the model for inspector-owned filters.
- Same-view mutation refresh must happen in the command-success callback, not
  only after navigation.

## Affected Modules

- `src/app.rs`: ContentList filter dispatch and search-result activation.
- `src/library/app_impl.rs`: album/track membership mutation success, detail
  hydration, and inspector filter ownership.
- `src/view_models/library.rs`: Library source-tree and detail filter state.
- `src/view_models/search_results.rs`: remote result activation contracts if
  row IDs need stronger origin typing.
- `src/view_models/entity_detail.rs`: membership/action projections if
  remote-only rows need additional state.
- `src/ui/shells/workspace.rs` and inspector shells: binding only. No ownership
  inference in renderers.
- `tests/architecture_tests.rs`: ownership guards.

## Target State

### Source Tree

The left Library source tree shows local Library navigation. It is never filtered
by `All / Library / Index`. Membership mutations may remove items from the tree
because the local library changed, but content filters do not.

### ContentList Inspector

Each visible ContentList detail/search surface owns its filter state. Search
results use `SearchResultsInspectorPageVm`. Library-backed album/artist/track
details must expose an equivalent filter-owned contract for their rows and
actions.

### Remote Drill-Down

Search result row IDs preserve origin:

- `library-*` rows activate local Library detail.
- `index-*` artist rows push `IndexArtistDetail`, show Index feeds for that
  artist, and expose a breadcrumb back to the originating search result.
- `index-*` feed rows push `IndexFeedDetail` using `feed_guid` and render an
  Index detail surface even when no cached local detail exists.
- `index-*` track rows push `IndexTrackDetail` using `track_guid` and optional
  feed scope, and render an Index detail surface even when no cached local
  detail exists.

Remote drill-down must not produce "invalid local id" status for valid Index
rows. Uncached Index feed/track activation is a breadcrumb-driven detail path,
not transient feedback.

### Remove / Download Mutation

On successful remove/download:

- Update command status.
- Invalidate or prime the currently mounted detail VM.
- Recompute membership action state.
- Keep Index-origin rows visible under `All`/`Index` when remote identity exists.
- Show `Download` for remote-only rows and `Remove` for in-library rows.

## Proposed Sequence

1. **Planning and guards.** Land ADR 0049, this phase plan, one bounded task
   packet, and architecture guard expectations.
2. **Filter dispatch fix.** Rewire ContentList filter selection so source-tree
   rows are not filtered. Add a regression guard that `TopApp::set_frame_filter`
   does not call `LibraryApp::set_content_filter` for the source tree.
3. **Remote drill-down.** Teach search result activation to route `index-*`
   rows through remote identity and render a remote-backed detail state.
4. **Album mutation refresh.** On track removal/download success, update the
   mounted album inspector in place and project removed tracks as Index-origin
   rows when possible.
5. **Smoke follow-up fixes.** Restore Library/Search state after Settings,
   expose filter chrome for selected Library album details, add
   `IndexArtistDetail`, `IndexFeedDetail`, and `IndexTrackDetail` navigation
   entries, and ensure each Index drill-down surface keeps a breadcrumb back to
   the search result.
6. **Verification.** Run focused VM/architecture tests, full `cargo check`,
   `cargo clippy -- -D warnings`, `cargo fmt -- --check`, and `cargo test`.
   Capture visual proof when GPUI can launch.

## Schema / API Implications

No schema change is planned. Remote drill-down uses existing `api::Client`
detail methods and existing persisted identity facts when available.

## Risk Areas

- Accidentally moving remote fetch logic into renderers.
- Filtering the source tree while trying to filter the inspector.
- Rebuilding album details from local-only queries after removal.
- Stale async Index completions overwriting a newer ContentList nav state.
- Breaking local Library drill-down while adding remote drill-down.

## Test Strategy

- Unit tests for detail/filter VMs: `All`, `Library`, and `Index` row sets.
- Unit tests for mutation projection: local row becomes remote-only/downloadable
  after removal when remote identity exists.
- Unit tests for mutation projection: downloaded row becomes Library content in
  the mounted album detail without navigation.
- Architecture guards:
  - Source tree is not filtered by ContentList filter selection.
  - Remote result activation does not parse `index-*` IDs as local IDs.
  - Mutation success path calls same-view detail refresh/prime logic.
- Existing focused tests:
  - `cargo test --lib search_results --features async-runtime`
  - `cargo test --test architecture_tests adr_0048 --features async-runtime`
- Final gates:
  - `cargo fmt -- --check`
  - `cargo check`
  - `cargo clippy -- -D warnings`
  - `cargo test`

## Rollback Strategy

If mixed-origin album detail proves too large for this pass, keep the filter
dispatch fix and remote search drill-down guard, then explicitly disable
Index-origin album rows behind a VM-owned empty state. Do not restore tree-owned
filtering or local-ID parsing for remote rows.

## Open Questions

- Which existing remote detail shell should render `IndexArtistDetail`,
  `IndexFeedDetail`, and `IndexTrackDetail` after the Discover module is
  retired?
- Do album rows have enough persisted MusicIndex identity in every local removal
  case, or do some rows need a remote-unavailable empty state?
