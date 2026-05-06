# ADR 0037 Task 001 Review

## Reviewed Artifacts

- `docs/adr/0037-same-entity-surface-parity.md`
- `docs/plans/adr-0037-same-entity-surface-parity-phase-plan.md`
- `docs/tasks/adr-0037-task-001-feed-identity-action-parity.md`
- Diff for `src/view_models/entity_detail.rs`, `src/ui_entity.rs`,
  `src/ui_feed.rs`, `src/library.rs`, and `tests/architecture_tests.rs`
- Apple HIG references: `summaries/layout-complete.md`,
  `summaries/accessibility-complete.md`, `platforms/macos.md`

## Status

Automated pass with Library hydration follow-up implemented - 2026-05-02.

## Findings

- Visual smoke found a blocker: user-provided screenshots for `Way to Go` show
  Discover rendering `Website` and `RSS` identity actions, while Library
  renders only `RSS` for the same feed. No screenshot shows the requested
  Nostr action.
- Follow-up screenshots for `The Heycitizen Experience` confirm the blocker:
  Discover renders `Website`, `Nostr`, and `RSS`, while Library renders only
  `RSS`.
- Follow-up fix implemented: Library album nodes retain `feed_guid`, and
  selecting a Library album with incomplete feed identity facts hydrates and
  persists MusicIndex feed source facts before updating the selected album
  detail.

## Architectural Review

- `EntityActionVm` now carries `payload: Option<String>` and keeps the default
  payload absent for non-identity actions.
- Website, Nostr, and RSS feed identity actions are populated from the
  GPUI-free release-detail projection before rendering.
- Library and Discover feed detail now call
  `render_feed_identity_actions(&page, ...)`, so same-entity identity action
  order, label, icon, button chrome, and click semantics have one owner.
- Library contributor identity rows remain screen-bound and unchanged, matching
  the task carve-out.
- The architecture guard blocks feed RSS button construction from returning to
  `src/ui_feed.rs` or `src/library.rs`.

## HIG Review

- The change supports HIG-style hierarchy by keeping the same feed identity
  actions in the same relationship to the release-detail shell across Library
  and Discover.
- The shared identity-action composite preserves text labels plus protocol
  icons where available, so the row is not color-only.
- macOS-specific primary actions remain screen-owned, preserving contextual
  command affordances without forking the repeated identity chrome.

## Visual Evidence

- User-provided screenshots in chat on 2026-05-02 cover Discover dark,
  Library dark, Discover light, and Library light for `Way to Go`.
- The screenshots were reviewed in-thread rather than committed at the pinned
  file paths.
- Discover dark/light show `Website` and `RSS` identity actions.
- Library dark/light show `RSS` only.
- Follow-up screenshots for `The Heycitizen Experience` show Discover
  dark/light with `Website`, `Nostr`, and `RSS`; Library dark/light with
  `RSS` only.
- This proves the shared renderer is visible in both themes, but it also
  demonstrates that same-entity identity action parity is blocked by source
  fact availability or Library hydration.
- The Library hydration follow-up addresses that blocker in code. A new
  screenshot pass is still needed after selecting the Library album long enough
  for the background hydration to complete.

## Required Fixes

- Re-run visual smoke for `The Heycitizen Experience` after the Library
  hydration task completes on album selection. Expected result: Library and
  Discover both show `Website`, `Nostr`, and `RSS`.

## Verification

Green:

```bash
cargo fmt -- --check
cargo check
cargo test entity_action_vm_carries_identity_payload
cargo test release_feed_identity_actions_use_shared_renderer
cargo test library_view_model_updates_album_identity_facts_by_feed_id
cargo test
cargo clippy -- -D warnings
git diff --check
```

## Merge Recommendation

Do not mark Task 001 fully complete until the follow-up visual pass confirms
Library hydrates the same feed identity actions as Discover. Automated and
architectural gates are clean.
