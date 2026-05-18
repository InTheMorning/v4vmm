# ADR 0024 Library / Index Data Parity Task 002 — Index Track Detail Shape

## Goal

Replace the sparse Index track fallback detail with the shared track detail
surface fed by a GPUI-free `TrackView`.

This task implements the second loading-shape slice from
`docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md` for Index
track drill-down only.

## Files To Inspect

- `docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md`
- `docs/reviews/library-discover-parity-triage-track.md`
- `src/app/search_dispatch.rs`
- `src/view_models/search_results/results.rs`
- `src/view_models/search_results/index_detail.rs`
- `src/view_models/search_results/tests.rs`
- `src/ui/shells/search_results_inspector.rs`
- `src/ui/shells/track.rs`
- `src/ui/composites/track_detail_surface.rs`
- `src/view_models/track_detail.rs`
- `src/views.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/app/search_dispatch.rs`
- `src/view_models/search_results/results.rs`
- `src/view_models/search_results/index_detail.rs`
- `src/view_models/search_results/tests.rs`
- `src/ui/shells/search_results_inspector.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/discover/**`
- `src/ui/shells/library/**`
- `src/view_models/library.rs`
- `src/db.rs`
- SQLite schema / migrations
- ADR 0053 source-fact design

## Constraints

- Use the existing shared track detail VM/shell path:
  `TrackView` -> `TrackDetailVm` -> `TrackDetailPageVm` ->
  `build_track_detail_surface`.
- Keep this Index-only. Do not change Library track detail behavior.
- Do not add local source-fact persistence, schema columns, or migrations.
- Do not parse Index IDs as local IDs.
- Do not resurrect the parked Discover module.
- Do not add renderer-only field inference. Fields must come from the fetched
  `api::Track` through `TrackView::from_api`.
- Keep local-only controls out of Index detail: no Remove, Download,
  MusicBrainz, ID3 compare, staged edit, or playlist mutation controls in this
  task.
- No new `#[allow(...)]`; use `#[expect(...)]` only if an intentional lint
  suppression is truly required.

## Implementation Steps

1. Extend `TrackResultDisplay` to optionally carry a rich remote
   `TrackView`.
2. In `index_track_display`, when `EntityDetail::Track(track)` is available,
   build `TrackView::from_api(track.clone())` and attach it to the result row.
   Preserve the existing label, secondary text, and thumbnail behavior.
3. Extend `IndexDetailDisplay` to optionally carry the rich remote
   `TrackView`.
4. Update `IndexDetailDisplay::track` to copy the remote track view from the
   selected result row.
5. In `render_index_detail_display`, render a rich Index track detail when the
   track view exists:
   - create `TrackDetailVm::new(track, TrackDetailSurfaceContext::Discover)`
     and `.page()`;
   - provide identity actions through `render_track_page_identity_actions`;
   - render through `build_track_detail_surface`;
   - keep the scroll container and frame behavior consistent with the existing
     sparse Index detail page.
6. Keep the sparse fallback for missing remote track detail.
7. Add unit tests proving the rich remote track view is attached to result rows
   and propagated into `IndexDetailDisplay`.
8. Add or strengthen an architecture guard that prevents Index track detail
   from staying on the sparse `Source` / `ID` fallback path when a rich track
   view is present.

## Acceptance Criteria

- Activating an Index track with fetched detail renders the shared track detail
  surface, not the sparse source/id-only fallback.
- The rendered detail receives the fetched title, artist/release context,
  track number, duration, pubdate, explicit state, identity facts,
  contributors, transcript URL, and value routes through `TrackView::from_api`.
- Sparse source/id fallback still exists for missing remote track detail.
- No Library, Discover, schema, or local persistence behavior changes.
- No local-only command controls appear on Index track detail.
- Regression coverage pins the rich-track VM path and architecture ownership.

## Test Commands

```bash
cargo fmt -- --check
cargo check --quiet
cargo test --lib --quiet
cargo test --test architecture_tests --quiet
cargo clippy --quiet -- -D warnings
```

## Expected Final Summary Format

1. Files changed.
2. Tests run.
3. Behavior changed.
4. Deviations from task.
5. Unresolved concerns.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/tasks/adr-0024-library-index-data-parity-task-002-index-track-detail.md`
- `docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md`
- `docs/reviews/library-discover-parity-triage-track.md`
- `src/app/search_dispatch.rs`
- `src/view_models/search_results/results.rs`
- `src/view_models/search_results/index_detail.rs`
- `src/view_models/search_results/tests.rs`
- `src/ui/shells/search_results_inspector.rs`
- `src/ui/shells/track.rs`
- `src/view_models/track_detail.rs`
- `src/views.rs`
- `tests/architecture_tests.rs`

Goal:
- Carry fetched Index track detail as a `TrackView` through search result rows
  into `IndexDetailDisplay`, and render it with the shared track detail
  surface.

Constraints:
- Use `TrackView::from_api`, `TrackDetailVm`, and
  `track::build_track_detail_surface`.
- Keep sparse source/id fallback only for missing remote track detail.
- Do not touch Library detail, Discover modules, schema, migrations, or ADR
  0053 source-fact persistence.
- Do not add local-only command controls to Index track detail.
- Do not infer fields in renderers.
- No new `#[allow(...)]`.
- You are not alone in the codebase. Do not revert unrelated edits.

Do not touch:
- `src/discover/**`
- `src/ui/shells/library/**`
- `src/view_models/library.rs`
- `src/db.rs`
- SQLite schema / migrations
- ADR 0053 source-fact design

Acceptance criteria:
- `TrackResultDisplay` can carry a remote `TrackView`.
- `index_track_display` attaches the fetched `TrackView`.
- `IndexDetailDisplay::track` preserves that rich track view.
- `render_index_detail_display` renders rich Index track details through the
  shared track detail surface when present.
- Unit and architecture coverage pin the path.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- The Index track fetch path no longer has access to `api::Track` detail.
- Rendering the shared track surface requires Library-only command state.
- The implementation needs schema, source-fact persistence, or local DB
  changes.
- The shared track surface cannot render without GPUI image-cache state.
