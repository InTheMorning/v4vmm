# ADR 0036 Feed, Visual, and Provenance Consistency Phase Plan

## Goal

Make the same feed/track recognizable across Library and Discover, then make
the surrounding visual system and advanced provenance panels consistent enough
to support richer playlist and playback work safely.

## Non-Goals

- No backend, schema, service, or API changes.
- No metadata inference changes.
- No pointer-driven visual automation.
- No new feature work while the current pass is not green.

## Assumptions

- ADR 0035 track consolidation remains in the worktree.
- `ReleaseDetailVm` is the canonical feed/release display contract.
- `AddToPlaylistPopover` remains the only playlist popover owner.
- Screens can still own click handlers and resolved images.

## Affected Modules

- `src/view_models/entity_detail.rs`
- `src/ui_entity.rs`
- `src/ui/composites/release_detail_surface.rs`
- `src/library.rs`
- `src/ui_feed.rs`
- `tests/architecture_tests.rs`
- Follow-up passes will also touch shared primitives/tokens and advanced
  metadata panel modules.

## Proposed Sequence

1. Feed surface consolidation:
   - Add typed release surface slot wrappers.
   - Route release shell behavior slots through typed elements.
   - Add architecture guards for typed slots and VM consumption.
   - Verify Library and Discover feed detail still use the same shell.
2. Visual system enforcement:
   - Audit shared primitives/composites for raw sizes, colors, icon glyphs,
     row heights, and popover padding.
   - Move any repeated value into a named token or primitive.
   - Tighten architecture tests so screens cannot compensate locally.
3. Advanced provenance panel consistency:
   - Inventory MusicBrainz, compare, staged tag, and provenance panels.
   - Extract repeated panel grammar and label policy.
   - Keep source-specific facts in panel VMs, not screen code.

## Schema/API Implications

None.

## Risk Areas

- Over-typing behavior slots can make command wiring cumbersome. Keep wrappers
  thin and only encode boundaries that prevent drift.
- Visual-system changes can unintentionally change density. Require screenshots
  from the user after each visual pass.
- Advanced metadata panels contain legitimate source-specific labels. Guards
  must separate provenance labels from normal detail labels.

## Test Strategy

- `cargo fmt -- --check`
- `cargo check`
- `cargo test`
- `cargo clippy -- -D warnings`
- `git diff --check`
- User-provided screenshots for Library feed, Discover feed, Library track,
  Discover track, playlist popovers, player/header, and advanced panels.

## Rollback Strategy

Each pass is isolated. Revert the pass-specific files and guard additions if a
surface cannot be made green without broad redesign.
