# ADR 0037 Same-Entity Surface Parity Phase Plan

## Goal

Make the same feed or normal track recognizable across Library and Discover by
removing remaining screen-local identity/action chrome.

This plan covers **Pass 1 of 2** (feed identity actions). Pass 2 (track
header/action parity) is sequenced separately as Task 002 and reuses the
`EntityActionVm.payload` extension landed here.

## Non-Goals

- No backend, schema, metadata inference, playlist, or playback behavior
  changes.
- No broad visual redesign.
- No removal of Library-only advanced metadata workflows.
- No change to identity-action click semantics (still
  `open::that` for Website/RSS, clipboard write for Nostr). HIG-compliant copy
  feedback is a follow-up, not part of this pass.

## Current State

- Release and track surfaces route through shared composites
  (`ui::composites::ReleaseSurfaceElement`, `render_release_detail_shell`).
- `ReleaseDetailVm::identity_actions()` already produces a context-invariant
  `Vec<EntityActionVm>` for Website/Nostr/RSS, and a unit test pins parity
  across `EntitySurfaceContext::Discover` and
  `EntitySurfaceContext::Library`.
- However, `EntityActionVm` carries `kind`, `target`, `label`, `tone`,
  `enabled` — **not** the URL or npub the click handler needs. So a screen
  cannot today render a clickable identity row from `ReleaseDetailPageVm`
  alone; it still reaches into `FeedView.identity.*` and `FeedView.feed_url`.
- Discover (`src/ui_feed.rs:65-114`) and Library
  (`src/library.rs:2473-2520`) each iterate those source fields and build
  near-identical `Vec<ReleaseSurfaceElement>` lists. Iteration logic and
  click bindings are duplicated; only the `ElementId` prefix differs.

## Target State

- `EntityActionVm` carries an `Option<String>` payload populated for the
  three identity kinds (`OpenWebsite`, `CopyNostr`, `OpenRss`).
- `ui_entity::render_feed_identity_actions(page, id_prefix)` is the single
  renderer for feed identity actions; it consumes `page.identity_actions`
  and the payload, hardcodes the click semantics, and returns
  `Vec<ReleaseSurfaceElement>`.
- Discover feed detail and Library feed detail call the helper. Their local
  identity-action renderers are removed.
- One architecture test forbids `IdentityActionKind::Rss` button construction
  outside `src/ui_entity.rs` and `src/ui/composites/`.
- Both light and dark theme screenshots verify Library and Discover feed
  detail.

## Affected Modules

- `src/view_models/entity_detail.rs` — `EntityActionVm` field + builder,
  `IdentityLinksVm::actions` and the RSS push site populate payload, new
  unit tests.
- `src/ui_entity.rs` — new `render_feed_identity_actions` helper,
  `EntityActionKind → IdentityActionKind` mapping.
- `src/ui_feed.rs` — replace `render_identity_actions` with helper call.
- `src/library.rs` — replace `render_library_identity_actions` with helper
  call. Contributor identity rows untouched.
- `tests/architecture_tests.rs` — new
  `release_feed_identity_actions_use_shared_renderer` guard.
- `docs/adr/0037-…`, `docs/reviews/adr-0037-review-checklist.md`,
  `docs/tasks/adr-0037-task-001-…` — update to reflect landed contract.

## Helper Signature (pinned)

```rust
// src/ui_entity.rs
#[must_use]
pub fn render_feed_identity_actions(
    page: &ReleaseDetailPageVm<'_>,
    id_prefix: &str,
) -> Vec<ReleaseSurfaceElement>;
```

- Iterates `page.identity_actions` in order.
- Maps `EntityActionKind::OpenWebsite → IdentityActionKind::Website`,
  `CopyNostr → Nostr`, `OpenRss → Rss`. Other kinds are ignored (they
  are not expected in `identity_actions` today; ignoring keeps the helper
  forward-compatible).
- Builds `ElementId`s as `format!("{id_prefix}-{kind}:{payload}")` so screens
  keep namespace separation (`discover-feed`, `library-feed`).
- Click semantics are hardcoded:
  - `OpenWebsite` and `OpenRss` → `open::that(payload)`
  - `CopyNostr` → `cx.write_to_clipboard(ClipboardItem::new_string(payload))`
- Skips any action whose payload is `None` (defensive; should not occur if
  the VM is correct).

## VM Extension (pinned)

```rust
// src/view_models/entity_detail.rs
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityActionVm {
    pub kind: EntityActionKind,
    pub target: EntityActionTarget,
    pub label: String,
    pub enabled: bool,
    pub tone: EntityActionTone,
    pub payload: Option<String>, // NEW
}

impl EntityActionVm {
    pub fn new(/* unchanged */) -> Self { /* payload: None */ }
    pub fn with_payload(mut self, payload: impl Into<String>) -> Self { … }
}
```

- `IdentityLinksVm::actions(target)` calls `with_payload(url)` /
  `with_payload(npub)` on the Website/Nostr pushes.
- `ReleaseDetailVm::identity_actions()` calls `with_payload(feed_url)` on the
  RSS push.
- All other action constructors keep `payload: None`.

## Sequence

1. Extend `EntityActionVm` with `payload: Option<String>` + `with_payload`.
   Update the three identity push sites. Add VM unit tests asserting payload
   presence/absence.
2. Add `ui_entity::render_feed_identity_actions(page, id_prefix)`.
3. Route Discover feed detail through the helper; delete
   `render_identity_actions` from `ui_feed.rs`.
4. Route Library feed detail through the helper; delete
   `render_library_identity_actions` from `library.rs`.
5. Add the architecture guard.
6. Run all checks. Capture light + dark screenshots for Library and Discover
   feed detail.

## Schema/API Implications

- `EntityActionVm` gains a public field. This is a VM contract change. All
  internal call sites and tests are updated in this pass; no external
  consumer exists.
- No DB schema, RSS, ID3, or playback-contract changes.

## Risks

- Click bindings must keep Website-open, Nostr-copy, RSS-open behavior
  intact. Helper hardcodes them; verified by screenshot evidence (visible
  buttons) and by manual click test on at least one feed per surface.
- Contributor identity rows in Library use the same atom but are out of
  scope; their renderer must remain unchanged.
- `EntityActionVm` is a public struct in `view_models::entity_detail`. Adding
  a field is a minor breaking change for any test/consumer that constructs
  one with a struct literal. Audit before landing.

## Test Strategy

- `cargo fmt -- --check`
- `cargo check`
- `cargo test entity_action_vm_carries_identity_payload`
- `cargo test release_feed_identity_actions_use_shared_renderer`
- `cargo test`
- `cargo clippy -- -D warnings`
- `git diff --check`
- Visual: light + dark screenshots for Library feed detail and Discover feed
  detail (four images), filed at the paths in the review checklist.

## Rollback Strategy

Revert the helper, the VM payload field, and restore the prior Library and
Discover identity-action renderers. No persisted data is affected. The VM
extension is the only cross-cutting change; if Pass 2 has not yet started,
reverting it is local.
