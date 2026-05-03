# ADR 0037 Review Checklist

## Reviewed Artifacts

- `docs/adr/0037-same-entity-surface-parity.md`
- `docs/plans/adr-0037-same-entity-surface-parity-phase-plan.md`
- `docs/tasks/adr-0037-task-001-feed-identity-action-parity.md`
- `docs/tasks/adr-0037-task-002-track-header-action-parity.md` (Pass 2 stub)

## Gate Status

Status: Task 001 in progress. Task 002 not yet started.

## Structural Review Questions

### Pass 1 (Task 001)

- Do Library and Discover feed detail render identity actions from
  `ui_entity::render_feed_identity_actions(page, id_prefix)`?
- Does the helper consume `ReleaseDetailPageVm.identity_actions` only —
  no reach back into `FeedView.identity.*` or `feed_url`?
- Does `EntityActionVm` carry `payload: Option<String>`?
  - Populated for `OpenWebsite`, `CopyNostr`, `OpenRss`.
  - `None` for every other action kind.
- Do `IdentityLinksVm::actions` and `ReleaseDetailVm::identity_actions`
  populate the payload?
- Are Website-open, Nostr-copy, and RSS-open click behaviors preserved?
- Are ElementId prefixes distinct per surface
  (`discover-feed-…` vs `library-feed-…`)?
- Are contributor identity rows untouched
  (`library_contributor_identity_actions` byte-identical)?
- Did the task add `release_feed_identity_actions_use_shared_renderer`?
  Does it forbid `IdentityActionKind::Rss` in `src/ui_feed.rs` and
  `src/library.rs` and require `fn render_feed_identity_actions` in
  `src/ui_entity.rs`?
- Was the helper's `EntityActionKind` match exhaustive enough that
  unexpected kinds (Play, Download, …) are skipped, not panicked?

### Cross-cutting

- Did visual smoke use user-provided screenshots at the pinned paths?
- Are both light and dark theme screenshots present (HIG dark-mode parity)?

## Task Results

| Task | Status | Required Evidence | Notes |
|---|---|---|---|
| Task 001 feed identity action parity | In progress | VM payload field + tests, shared helper, architecture guard, checks, four screenshots | Implementation pending |
| Task 002 track header/action parity   | Not started | TBD when Task 001 lands | Reuses `EntityActionVm.payload` |

## Visual Smoke

Required screenshot paths (capture both themes):

| Surface | Light | Dark |
|---|---|---|
| Library feed detail   | `docs/reviews/screenshots/adr-0037-library-feed-identity-light.png` | `docs/reviews/screenshots/adr-0037-library-feed-identity-dark.png` |
| Discover feed detail  | `docs/reviews/screenshots/adr-0037-discover-feed-identity-light.png` | `docs/reviews/screenshots/adr-0037-discover-feed-identity-dark.png` |

Capture conditions:
- Project's standard dev window size (no manual resize).
- Theme toggled via the app's theme control; use the project default theme
  variant for each side.
- Feed must have all three identity sources populated (website, nostr, RSS)
  so the full button row renders. If no real feed in fixtures has all
  three, document the substitute.

## Automated Checks

- `cargo fmt -- --check`: pending
- `cargo check`: pending
- `cargo test entity_action_vm_carries_identity_payload`: pending
- `cargo test release_feed_identity_actions_use_shared_renderer`: pending
- `cargo test`: pending
- `cargo clippy -- -D warnings`: pending
- `git diff --check`: pending
