# ADR 0037 Task 002 Review

## Reviewed Artifacts

- `docs/adr/0037-same-entity-surface-parity.md`
- `docs/plans/adr-0037-same-entity-surface-parity-phase-plan.md`
- `docs/tasks/adr-0037-task-002-track-header-action-parity.md`
- Diff for `src/view_models/track_detail.rs`, `src/ui_track.rs`,
  `src/search.rs`, `src/library.rs`, and `tests/architecture_tests.rs`
- Apple HIG references: `summaries/layout-complete.md`,
  `platforms/macos.md`

## Status

Implemented with automated evidence green - 2026-05-03. Visual smoke remains
pending.

## Findings

- No architectural blockers found in the implemented Task 002 diff.
- Visual evidence is still missing for track detail. Capture light and dark
  Library/Discover screenshots for the same normal track after full checks
  pass.

## Architectural Review

- `TrackDetailVm::identity_actions()` now projects Website and Nostr facts
  into `EntityActionVm` values with payloads, and emits no actions for a track
  without a stable track target.
- `ui_track::render_track_identity_actions(detail, id_prefix)` is the single
  track identity external-link renderer. It maps Website to open behavior and
  Nostr to clipboard-copy behavior from the VM payload.
- Discover track detail appends the shared identity elements into the existing
  external-link slot while keeping feed navigation and audio play in
  `render_track_header_subtitle`.
- Library track detail supplies the same shared identity strip through
  `TrackDetailSurface::external_links`; Library advanced metadata panels are
  untouched.
- The architecture guard requires Discover and Library track details to call
  the shared renderer and blocks the old Discover track Nostr path from
  returning.

## HIG Review

- The change strengthens hierarchy parity by placing the same identity actions
  in the same external-link region of `TrackDetailSurface` across Library and
  Discover.
- Contextual commands remain contextual: Discover's feed navigation and play
  controls are still screen-owned, and Library's advanced panels remain
  Library-only.
- The shared identity-action composite preserves recognizable labels and
  protocol icons rather than relying on color alone.

## Visual Evidence

Partial runtime check only. I launched the app on 2026-05-03 and selected the
Library track `MoeFactz`; the track detail surface rendered correctly, but the
local database had no `owner_kind='track'` identity link or ID rows, so no
structured Website/Nostr identity buttons were available to compare.

Required screenshots remain pending:

- `docs/reviews/screenshots/adr-0037-library-track-detail-light.png`
- `docs/reviews/screenshots/adr-0037-library-track-detail-dark.png`
- `docs/reviews/screenshots/adr-0037-discover-track-detail-light.png`
- `docs/reviews/screenshots/adr-0037-discover-track-detail-dark.png`

Use the same normal track in Library and Discover, with Website and Nostr
identity facts present.

## Verification

Green:

```bash
cargo fmt -- --check
cargo check
cargo test track_detail_identity_actions_carry_payloads
cargo test track_identity_links_use_shared_renderer
cargo test
cargo clippy -- -D warnings
git diff --check
```

## Merge Recommendation

Do not mark Task 002 visually complete until the track detail screenshots
confirm Library/Discover parity in light and dark themes.
