# ADR 0038 Task 002: Composite Display-Contract Audit

## Status

In progress - first five slices implemented on 2026-05-03.

## Goal

Every public composite signature accepts a view-model field, a co-located
display struct, or a pure passthrough. No composite accepts a
policy-bearing `String`/`&str`/`SharedString` for a label, fallback, or
state value.

## Structural Contract

- Layer: 6 (`src/ui/composites`) with callers in layer 7 shells and layer
  8 screens.
- Presentation owner: each repeated composite owns layout and chrome;
  display policy lives in a GPUI-free VM or a co-located display contract.
- HIG foundation strengthened: predictable hierarchy. Row title, metadata,
  state, and actions enter shared chrome through named contracts rather
  than ad hoc screen strings.
- Regression guard:
  `composite_signatures_take_display_contracts_not_loose_strings`.

## Classification Rule

A composite parameter is **policy-bearing** if it has, or invites callers
to make, a fallback rule, truncation rule, casing rule, format rule,
availability label, state label, or command label that two screens could
plausibly implement differently.

A **pure passthrough** is a value whose display policy has already been
decided by a VM, display struct, enum, command outcome, or caller-owned
generic option list. Passthrough APIs must be listed in the architecture
test allowlist with a one-line justification.

## Migration Order

1. `TrackRow`
   - Replace public loose builders for row number/title/duration with a
     VM-backed row contract.
   - Keep thumbnail, click handler, and trailing actions as presentation
     slots because they are not fallback policy.
   - Shrink the string API allowlist by removing the old `TrackRow`
     builder allowances.
2. `DetailHeader`
   - Audit `new`, `subtitle`, and `data_row`.
   - Introduce `DetailHeaderDisplay` / `DetailHeaderDataRow`.
   - Remove the loose title/subtitle/data-row builder allowances.
3. `DetailGrid`
   - Audit `DetailRow::new` and `DetailRow::text`.
   - Introduce `DetailElementRow` / `DetailTextRow`.
   - Remove loose key/value row constructor allowances.
4. `ReleaseDetailSurface`
   - Audit `track_section` title/summary.
   - Introduce `ReleaseTrackSectionDisplay`.
   - Move the section title to `TrackListVm`.
5. `PlaylistPopover`
   - Audit trigger label and playlist option labels.
   - Introduce `AddToPlaylistDisplay` and `PlaylistOptionDisplay`.
   - Remove the trigger-label builder and loose playlist-option name
     constructor allowances.
6. `TrackMetadataGrid` cells
   - Audit group/field/tag/text labels.
   - Keep allowed only where `TrackMetadataGridVm` or metadata row VMs own
     the source-specific strings.
7. Generic control composites
   - Audit `ActionRow`, `DisclosureGroup`, `SegmentedControl`,
     `TagBadge`, and `action_button`.
   - Keep only explicit passthrough allowances; no wildcard exceptions.

## Files Inspected

- `src/ui/composites/*.rs`
- `src/ui/shells/entity.rs`
- `src/ui/shells/track.rs`
- `src/view_models/entity_detail.rs`
- `src/view_models/track_detail.rs`
- `tests/architecture_tests.rs`

## Files Changed In Current Slices

- `src/ui/composites/track_row.rs`
- `src/ui/composites/detail_header.rs`
- `src/ui/composites/detail_grid.rs`
- `src/ui/composites/mod.rs`
- `src/ui/composites/track_detail_surface.rs`
- `src/ui/composites/release_detail_surface.rs`
- `src/ui/composites/playlist_popover.rs`
- `src/ui/shells/artist.rs`
- `src/ui/shells/entity.rs`
- `src/ui/shells/track.rs`
- `src/library.rs`
- `src/search.rs`
- `src/view_models/entity_detail.rs`
- `src/view_models/artist.rs`
- `tests/architecture_tests.rs`
- `docs/tasks/adr-0038-task-002-composite-display-contract-audit.md`
- `docs/reviews/adr-0038-review-checklist.md`

## Do Not Touch

- `render_feed_identity_actions` and the ADR 0037 identity-action atom.
- Backend, playlist service, RSS, ID3, database, and playback behavior.
- Visual palette or layout density except as required by the contract.

## First-Slice Implementation Notes

- `TrackRow` now exposes row construction through `TrackRowVm` or
  `SharedTrackRowVm`, not public `.number()`, `.title()`, or
  `.duration()` loose string builders.
- `TrackRow` retains additive slots for thumbnail, click handling, and
  trailing action elements.
- The architecture guard now scans multi-line public function signatures
  and is named to match ADR 0038:
  `composite_signatures_take_display_contracts_not_loose_strings`.
- The explicit allowlist remains for genuine passthrough APIs and shrank
  by the three former `TrackRow` string builders.

## Second-Slice Implementation Notes

- `DetailHeader` now accepts `DetailHeaderDisplay`.
- Header title, subtitle, and metadata rows now enter through
  `DetailHeaderDisplay` / `DetailHeaderDataRow`, not public loose string
  builders on the composite.
- `library.rs`, `search.rs`, `ui::shells::artist`, and
  `ui::shells::entity` construct the display contract from existing
  VM-projected values.
- The explicit allowlist shrank by the three former `DetailHeader`
  string builders.

## Third-Slice Implementation Notes

- `DetailGrid` now uses `DetailElementRow` and `DetailTextRow` display
  contracts for rich and plain key/value rows.
- `DetailRow::new` and `DetailRow::text` no longer expose loose
  key/value string-like parameters.
- Library, Discover, artist shell, entity shell, and track detail surface
  callers construct `DetailTextRow` from existing VM-owned facts.
- The explicit allowlist shrank by the two former `DetailGrid` row
  constructor allowances.

## Fourth-Slice Implementation Notes

- `ReleaseDetailSurface::track_section` now accepts
  `ReleaseTrackSectionDisplay` instead of loose title/summary string
  parameters.
- The release track-section title now comes from `TrackListVm::title()`;
  the summary remains `TrackListVm::summary()`.
- `ui::shells::entity` constructs the section display contract from
  `ReleaseDetailPageVm`.
- The explicit allowlist shrank by the former
  `ReleaseDetailSurface::track_section` string API allowance.

## Fifth-Slice Implementation Notes

- `AddToPlaylistPopover` now accepts `AddToPlaylistDisplay` instead of
  loose id/playlists inputs plus a `.trigger_label(...)` override.
- `PlaylistOption::new` now accepts `PlaylistOptionDisplay`, so playlist
  option names are explicit display-contract fields.
- Library, Discover, and Discover track-shell callers construct the
  popover display contract from existing VM/action labels and playlist
  snapshots.
- The explicit allowlist shrank by the former `PlaylistOption::new`,
  `AddToPlaylistPopover::new`, and `.trigger_label(...)` string APIs.
  The `on_create` callback allowance remains because it carries a new
  playlist name payload, not display copy.

## Acceptance Criteria

- `cargo test composite_signatures_take_display_contracts_not_loose_strings`
  is green.
- `TrackRow` public signatures no longer accept row number, title, or
  duration as loose strings.
- `src/ui/shells/entity.rs` consumes the shared row VM instead of
  assembling the row display fields.
- `DetailHeader` public signatures no longer accept title, subtitle, or
  metadata-row labels/values as loose string parameters.
- `DetailGrid` row public signatures no longer accept key/value text as
  loose parameters.
- `ReleaseDetailSurface::track_section` no longer accepts title/summary
  text as loose parameters.
- `AddToPlaylistPopover` public display inputs no longer accept trigger
  labels or playlist option names as loose parameters.
- Full project gates are green before the slice is called complete.
- No screenshots are required for this slice because the row rendering
  behavior is unchanged; visual proof is deferred until a visible Task 004
  or Task 005 surface change and should use reviewer-guided navigation.

## Test Commands

```sh
cargo fmt -- --check
cargo check
cargo test composite_signatures_take_display_contracts_not_loose_strings
cargo test
cargo clippy -- -D warnings
git diff --check
```

## Expected Final Report

- Name the composite migrated.
- Name the guard tightened.
- Report automated gate status.
- Explicitly say whether visual evidence was needed and, if not, why.
