# ADR 0037 Review Checklist

## Reviewed Artifacts

- `docs/adr/0037-same-entity-surface-parity.md`
- `docs/plans/adr-0037-same-entity-surface-parity-phase-plan.md`
- `docs/tasks/adr-0037-task-001-feed-identity-action-parity.md`
- `docs/tasks/adr-0037-task-002-track-header-action-parity.md`

## Gate Status

Status: Task 001 implemented with automated evidence green. Visual smoke found
a Library hydration blocker; follow-up fix implemented and awaiting screenshot
re-check. Task 002 implemented with automated evidence green; track-detail
visual smoke pending.

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
- Do track identity actions preserve Discover's feed navigation/audio play
  controls and Library's advanced panels as contextual, screen-bound actions?

## Task Results

| Task | Status | Required Evidence | Notes |
|---|---|---|---|
| Task 001 feed identity action parity | Follow-up fix implemented; visual re-check pending | VM payload field + tests, shared helper, architecture guard, checks, four screenshots | Shared feed identity renderer landed; user screenshots showed Discover identity facts missing from Library; Library album selection now hydrates missing feed source facts by feed GUID |
| Task 002 track header/action parity   | Implemented; visual smoke pending | Track VM payload actions, shared helper, Discover/Library route-through, architecture guard, four screenshots | Reuses `EntityActionVm.payload`; Discover feed navigation/audio play and Library advanced panels remain screen-bound |

## Visual Smoke

Required screenshot paths (capture both themes):

| Surface | Light | Dark |
|---|---|---|
| Library feed detail   | `docs/reviews/screenshots/adr-0037-library-feed-identity-light.png` | `docs/reviews/screenshots/adr-0037-library-feed-identity-dark.png` |
| Discover feed detail  | `docs/reviews/screenshots/adr-0037-discover-feed-identity-light.png` | `docs/reviews/screenshots/adr-0037-discover-feed-identity-dark.png` |
| Library track detail  | `docs/reviews/screenshots/adr-0037-library-track-detail-light.png` | `docs/reviews/screenshots/adr-0037-library-track-detail-dark.png` |
| Discover track detail | `docs/reviews/screenshots/adr-0037-discover-track-detail-light.png` | `docs/reviews/screenshots/adr-0037-discover-track-detail-dark.png` |

Capture conditions:
- Project's standard dev window size (no manual resize).
- Theme toggled via the app's theme control; use the project default theme
  variant for each side.
- Feed must have all three identity sources populated (website, nostr, RSS)
  so the full button row renders. If no real feed in fixtures has all
  three, document the substitute.

Received visual evidence:
- User-provided screenshots in chat on 2026-05-02 cover Discover dark,
  Library dark, Discover light, and Library light for `Way to Go`.
- Discover dark/light show `Website` and `RSS` feed identity actions.
- Library dark/light show `RSS` only for the same feed.
- No screenshot shows a Nostr identity action for this fixture.
- Follow-up user screenshots in chat on 2026-05-02 cover Discover dark,
  Library dark, Discover light, and Library light for
  `The Heycitizen Experience`.
- Follow-up Discover screenshots show `Website`, `Nostr`, and `RSS`; follow-up
  Library screenshots show `RSS` only.
- Follow-up fix: Library album nodes now retain `feed_guid`, selected Library
  albums with incomplete feed identity facts fetch MusicIndex feed source facts,
  persist them through `identity_ingest::persist_musicindex_feed`, and update
  the open album detail snapshot.
- Result: visual gate needs one more Library/Discover screenshot pass for
  `The Heycitizen Experience` after the hydration task runs on selection.
- Task 002 track-detail screenshots have not yet been captured. They need a
  normal track with Website and Nostr identity facts so Library and Discover
  can be compared in both themes.
- Attempted Task 002 visual smoke on 2026-05-03 with the local Library track
  `MoeFactz`; the track detail rendered, but the local SQLite data had no
  `owner_kind='track'` identity link or ID rows, so there were no structured
  Website/Nostr buttons to compare.

## Automated Checks

- `cargo fmt -- --check`: Green
- `cargo check`: Green
- `cargo test entity_action_vm_carries_identity_payload`: Green
- `cargo test release_feed_identity_actions_use_shared_renderer`: Green
- `cargo test track_detail_identity_actions_carry_payloads`: Green
- `cargo test track_identity_links_use_shared_renderer`: Green
- `cargo test`: Green
- `cargo clippy -- -D warnings`: Green
- `git diff --check`: Green
