# ADR 0037: Same-Entity Surface Parity

## Status

Proposed - 2026-05-02.

## Context

ADR 0035 and ADR 0036 moved track, release, playlist-popover, and advanced
provenance grammar into shared view-models and composites. The remaining UI
risk is subtler: Library and Discover can still display the same feed or track
through the same shell while rebuilding small pieces of behavior chrome in
screen-local code.

Apple HIG layout guidance expects a consistent hierarchy and familiar
relationships between controls and content. In this app that means the same
entity must keep the same core identity, summary, actions, metadata order, and
track rows across Library and Discover. Context-specific capabilities are
allowed, but they must attach through named slots and shared contracts.

## Decision

Complete parity as bounded passes. The ADR closes only when both passes land.

1. **Pass 1 — Feed identity actions.** Website/Nostr/RSS rendering must move to
   one shared shell helper that consumes
   `ReleaseDetailPageVm.identity_actions`. Order, icon, label, button chrome,
   and click semantics have one owner. Library and Discover keep their own
   primary-action chrome (download/play/playlist), which remains screen-bound.
   To make the VM contract self-sufficient, `EntityActionVm` gains a
   `payload: Option<String>` field carrying the URL or npub the click target
   needs; the helper derives all clickable state from the VM alone.
2. **Pass 2 — Track header/action parity.** The same normal track must keep
   one header, summary, action-row, external-link, and lazy-section grammar
   across Library and Discover. Library-only advanced panels remain
   contextual. Pass 2 reuses the `EntityActionVm.payload` extension introduced
   in Pass 1.
3. Visual smoke remains screenshot-based. Both light and dark themes must be
   captured for any surface a pass changes; HIG dark-mode parity is a hard
   requirement, not optional.

## Invariants

- Same-entity normal surfaces must consume the same GPUI-free view-model
  contract before rendering.
- Screens may bind commands, images, and local state; they may not duplicate
  shared identity/action chrome.
- Context-specific capabilities must be visibly additive, not alternate page
  skeletons.
- Any parity fix must add or strengthen an architecture guard.

## Non-Goals

- No backend, schema, RSS, ID3, playlist, or playback semantics changes.
- No redesign of navigation.
- No attempt to make Library-only advanced provenance panels identical to
  Discover normal track detail.

## Alternatives Considered

- Accept the remaining duplication because the visuals are close. Rejected
  because this is how previous popover and recent-feed regressions returned.
- Move all click behavior into view-models. Rejected because command binding
  and local side effects remain UI responsibilities.
- Merge Library and Discover screens. Rejected as too broad for this pass.

## Consequences

- Shared entity shells get stricter ownership of repeated identity chrome.
- Screen code becomes smaller and more focused on command binding.
- `EntityActionVm` gains a payload field; this is the canonical place to
  attach URL/npub data, and Pass 2 consumes it without further VM churn.
- Richer playlist/playback work remains safer because the entity surface has
  fewer duplicate places to regress.
- Once the helper owns Nostr-copy click semantics, adding HIG-compliant copy
  feedback (toast or label transition) becomes a one-place change. Out of
  scope for ADR 0037; tracked as a follow-up.

## Enforcing Tests

- `release_feed_identity_actions_use_shared_renderer` blocks Library and
  Discover feed detail from constructing `IdentityActionKind::Rss` buttons
  outside `src/ui_entity.rs` and `src/ui/composites/`.
- `entity_action_vm_carries_identity_payload` (in
  `src/view_models/entity_detail.rs` tests) pins that
  `OpenWebsite`/`CopyNostr`/`OpenRss` VMs always carry their click payload.
