# ADR 0038 Task 005: HIG Accessibility-Label Contract

## Status

Completed on 2026-05-04.

The interactive composite inventory now carries VM- or display-contract-sourced
accessibility labels:

- `ActionButtonDisplay` carries `a11y_label` from `EntityActionVm::a11y_label()`
  and `LibraryAlbumMusicBrainzActionVm::a11y_label`.
- `IdentityActionButtonDisplay` carries labels from `IdentityActionDisplay` and
  `ContributorIdentityActionDisplay`.
- `ActionRowDisplay`, `AddToPlaylistDisplay`, `PlaylistOptionDisplay`,
  `TrackRowDisplay`, `ListRow`, `RecentFeedTileDisplay`,
  `DisclosureGroupDisplay`, `SegmentDisplay`, `NowPlayingData`,
  `ReleaseDetailPageVm`/`ReleaseDetailSurface`, `TrackDetailSurface`, and
  `TrackRowVm` all expose explicit a11y-label fields.
- `interactive_composites_carry_accessibility_labels` enforces the expanded
  coverage list.

GPUI 0.2.x still has no final accessibility-label sink for these widgets. The
labels are contract data today and can be plumbed into the framework once GPUI
exposes the surface.

## Goal

Every composite that renders interactive chrome exposes a VM-sourced
accessibility label. Where the action is non-obvious, an accessibility
hint accompanies the label. Accessibility strings live in view-models,
never in screens.

## Inventory

Today: one `accessibility_label` method (`tag_badge.rs:166`). The rest
of the composite layer has none.

Composites that render interactive chrome (initial list; expand when
starting):

- `action_button` / `ActionRow`
- `identity_action_button`
- `playlist_popover` (`AddToPlaylistPopover`)
- `track_row` (`TrackRow`)
- `list_row` (`ListRow`)
- `recent_feed_tile`
- `disclosure_group`
- `segmented_control`
- `now_playing_bar`
- `release_detail_surface` action overlays
- `track_detail_surface` action overlays

Composites that are pure-text/pure-layout are exempt:
`detail_grid`, `detail_header`, `file_header` (verify), `multiline_text`
(primitive), `divider`, `loading`.

## Files Likely To Change

- `src/view_models/*.rs` — new `*_a11y_label` and (where appropriate)
  `*_a11y_hint` accessors.
- `src/ui/composites/*.rs` — accept the label/hint as typed parameters
  on existing display contracts.
- `src/ui/shells/*.rs` and `src/library.rs`/`src/search.rs` — pass the
  new fields through.
- `tests/architecture_tests.rs` — new guard
  `interactive_composites_carry_accessibility_labels` with an explicit
  composite list.

## Open Questions

1. **Static vs. dynamic labels.** Some labels are constant ("Add to
   playlist"); some include dynamic data ("Add 'Track Name' to
   playlist"). Decide per composite whether the VM produces a constant
   `&'static str` or a dynamic `String`. Default: dynamic, since the VM
   is already producing the visible label and constructing both
   together is cheap.
2. **Hint policy.** When does a composite need a hint? HIG: when the
   action is destructive, when it triggers navigation, or when the
   visible label alone is ambiguous. Document the rule in the
   composite's module-level doc.
3. **GPUI accessibility plumbing.** Verify GPUI's accessibility API
   surface. If the underlying widget set doesn't expose
   `accessibility_label` cleanly, document the gap and plumb what's
   available; do not block the task on framework limits.
4. **Non-text interactive elements.** Icon-only buttons and image
   thumbnails with click handlers must carry a label even though their
   visible text is empty. List them explicitly.

## Constraints

- This is purely additive — no existing composite signatures break
  silently. Add new fields with sensible defaults during migration if
  needed.
- VM unit tests cover a11y label generation alongside display label
  generation.
- Coordinate with Task 002: a11y label is part of the composite's
  display contract, not a separate parameter.

## Definition of Done

- Every composite in the inventory exposes an a11y label sourced from
  its VM.
- New guard `interactive_composites_carry_accessibility_labels` is
  green with an explicit composite list.
- A11y coverage table in the review checklist names every composite
  and its label/hint policy.
- Pointer to a child ADR for the dynamic-type ramp is added to ADR
  0038's "Follow-Up Work" once this task lands.

## When To Start

After Task 002 lands (composite contracts settled). Can run in parallel
with Task 004 (different concern; no overlap).
