# ADR 0037 Task 001: Feed Identity Action Parity

## Goal

Render Library and Discover feed Website/Nostr/RSS identity actions from one
shared helper that consumes `ReleaseDetailPageVm.identity_actions`. To make
that possible, extend `EntityActionVm` with a `payload: Option<String>` field
so the helper has everything it needs from the VM alone.

## Files To Inspect

- `docs/adr/0037-same-entity-surface-parity.md`
- `docs/plans/adr-0037-same-entity-surface-parity-phase-plan.md`
- `src/ui_entity.rs`
- `src/ui_feed.rs`
- `src/library.rs`
- `src/view_models/entity_detail.rs`
- `src/ui/composites/identity_action.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/view_models/entity_detail.rs` — VM extension + identity-payload sites
  + new unit tests.
- `src/ui_entity.rs` — new `render_feed_identity_actions` helper.
- `src/ui_feed.rs` — call helper, delete local renderer.
- `src/library.rs` — call helper, delete local renderer.
- `tests/architecture_tests.rs` — new architecture guard.
- `docs/reviews/adr-0037-review-checklist.md` — fill in evidence.

## Do Not Touch

- Backend services
- Database schema
- RSS/ID3 parsing
- Playlist behavior
- Playback behavior
- Contributor identity row behavior
  (`library_contributor_identity_actions` and its callers stay as-is)

## Constraints

- Preserve Website open, Nostr copy, and RSS open behavior. The helper
  hardcodes these via `open::that` and `cx.write_to_clipboard`.
- Preserve Library and Discover context-specific primary actions
  (download/play/playlist chrome remains screen-bound).
- Do not introduce screen-local Website/Nostr/RSS button construction.
- Keep ElementId namespacing distinct per surface
  (`discover-feed-…` vs `library-feed-…`).

## Implementation Steps

### Step 1 — VM extension

Update `src/view_models/entity_detail.rs`:

```rust
pub struct EntityActionVm {
    pub kind: EntityActionKind,
    pub target: EntityActionTarget,
    pub label: String,
    pub enabled: bool,
    pub tone: EntityActionTone,
    pub payload: Option<String>, // NEW
}

impl EntityActionVm {
    pub fn new(/* unchanged signature */) -> Self {
        Self { /* …, */ payload: None }
    }

    #[must_use]
    pub fn with_payload(mut self, payload: impl Into<String>) -> Self {
        self.payload = Some(payload.into());
        self
    }
}
```

In `IdentityLinksVm::actions`, attach the URL/npub:

```rust
if let Some(url) = self.website_url() {
    actions.push(
        EntityActionVm::new(EntityActionKind::OpenWebsite, target.clone(),
                            "Website", EntityActionTone::Quiet)
            .with_payload(url),
    );
}
if let Some(npub) = self.nostr_npub() {
    actions.push(
        EntityActionVm::new(EntityActionKind::CopyNostr, target,
                            "Copy Nostr", EntityActionTone::Quiet)
            .with_payload(npub),
    );
}
```

In `ReleaseDetailVm::identity_actions`, attach `feed_url` to the RSS push:

```rust
if let Some(feed_url) = nonempty(self.view.feed_url.as_deref()) {
    actions.push(
        EntityActionVm::new(EntityActionKind::OpenRss, target,
                            "RSS", EntityActionTone::Quiet)
            .with_payload(feed_url),
    );
}
```

Audit every other `EntityActionVm::new` call site in the file. None of them
should set a payload. Existing `==` comparisons in tests must keep passing
because both sides will have `payload: None`.

### Step 2 — VM unit test

Add a test named `entity_action_vm_carries_identity_payload` in the existing
`tests` module of `src/view_models/entity_detail.rs`:

- Build a `FeedView` with website, nostr, and feed URL all populated.
- `ReleaseDetailVm::new(&view, …).identity_actions()` returns three actions
  whose payloads equal the source URL/npub strings exactly.
- Assert non-identity actions (`actions()` for the same view) keep
  `payload == None`.

### Step 3 — Shared helper

Add to `src/ui_entity.rs`:

```rust
#[must_use]
pub fn render_feed_identity_actions(
    page: &ReleaseDetailPageVm<'_>,
    id_prefix: &str,
) -> Vec<ReleaseSurfaceElement> {
    page.identity_actions
        .iter()
        .filter_map(|action| {
            let payload = action.payload.as_deref()?;
            let kind = match action.kind {
                EntityActionKind::OpenWebsite => IdentityActionKind::Website,
                EntityActionKind::CopyNostr   => IdentityActionKind::Nostr,
                EntityActionKind::OpenRss     => IdentityActionKind::Rss,
                _ => return None,
            };
            let id = SharedString::from(format!("{id_prefix}-{}:{payload}",
                kind_slug(kind)));
            let payload_for_click = payload.to_string();
            let button = identity_action_button(id, kind).on_click(
                move |_, _, cx| match kind {
                    IdentityActionKind::Website | IdentityActionKind::Rss => {
                        let _ = open::that(&payload_for_click);
                    }
                    IdentityActionKind::Nostr => {
                        cx.write_to_clipboard(
                            ClipboardItem::new_string(payload_for_click.clone()),
                        );
                    }
                },
            );
            Some(ReleaseSurfaceElement::from_element(button.into_any_element()))
        })
        .collect()
}
```

`kind_slug` is a small private helper returning `"website" | "nostr" | "rss"`
to keep ElementIds stable and grep-friendly.

### Step 4 — Wire Discover

In `src/ui_feed.rs`:

- Delete `fn render_identity_actions`.
- In `render_feed_view`, set
  `identity_actions: render_feed_identity_actions(&page, "discover-feed")`.

### Step 5 — Wire Library

In `src/library.rs`:

- Delete `fn render_library_identity_actions`.
- In the corresponding feed-detail call site, set
  `identity_actions: render_feed_identity_actions(&page, "library-feed")`.
- Confirm `render_library_contributors_panel` and
  `library_contributor_identity_actions` are untouched.

### Step 6 — Architecture guard

Add `release_feed_identity_actions_use_shared_renderer` to
`tests/architecture_tests.rs`:

- Read sources for `src/ui_feed.rs` and `src/library.rs`. Assert neither
  contains the literal `IdentityActionKind::Rss`. (Website/Nostr literals
  are still allowed in `library.rs` because contributor rows use them.)
- Read source for `src/ui_entity.rs`. Assert it contains
  `fn render_feed_identity_actions`.

Use the same manifest-path + `read_source` helpers as the existing
ADR 0036 guard around line 1763.

### Step 7 — Checks

Run every command in the Test Commands list. All must pass with no
warnings. Then capture screenshots per the review checklist.

## Acceptance Criteria

- `EntityActionVm.payload` field exists and is `Some(…)` for the three
  identity kinds and `None` otherwise. New VM unit test covers both.
- Library and Discover feed detail both call
  `render_feed_identity_actions(&page, …)`. Local renderers are deleted.
- `release_feed_identity_actions_use_shared_renderer` is green and forbids
  `IdentityActionKind::Rss` in `src/ui_feed.rs` and `src/library.rs`.
- Website/Nostr/RSS click behavior is unchanged on both surfaces.
- Contributor identity rows are byte-identical (no incidental changes in
  `library.rs` outside the deleted feed renderer).
- `cargo fmt -- --check`, `cargo check`, `cargo test`,
  `cargo clippy -- -D warnings`, `git diff --check` all clean.
- Light and dark screenshots filed at:
  - `docs/reviews/screenshots/adr-0037-library-feed-identity-light.png`
  - `docs/reviews/screenshots/adr-0037-library-feed-identity-dark.png`
  - `docs/reviews/screenshots/adr-0037-discover-feed-identity-light.png`
  - `docs/reviews/screenshots/adr-0037-discover-feed-identity-dark.png`

## Test Commands

- `cargo fmt -- --check`
- `cargo check`
- `cargo test entity_action_vm_carries_identity_payload`
- `cargo test release_feed_identity_actions_use_shared_renderer`
- `cargo test`
- `cargo clippy -- -D warnings`
- `git diff --check`

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0037-same-entity-surface-parity.md`
- `docs/plans/adr-0037-same-entity-surface-parity-phase-plan.md`
- `docs/tasks/adr-0037-task-001-feed-identity-action-parity.md`
- `src/ui_entity.rs`
- `src/ui_feed.rs`
- `src/library.rs`
- `src/view_models/entity_detail.rs`
- `src/ui/composites/identity_action.rs`
- `tests/architecture_tests.rs`

Goal:
- Extend `EntityActionVm` with `payload: Option<String>` and a `with_payload`
  builder. Populate it for `OpenWebsite`, `CopyNostr`, and `OpenRss` only.
- Add `ui_entity::render_feed_identity_actions(page, id_prefix)`. Iterate
  `page.identity_actions`, map identity kinds, hardcode click semantics.
- Route Discover (`src/ui_feed.rs`) and Library (`src/library.rs`) feed
  detail through the helper. Delete the local renderers.
- Add the architecture guard
  `release_feed_identity_actions_use_shared_renderer` per Step 6.

Constraints:
- Preserve Website-open, Nostr-copy, RSS-open behavior exactly.
- Keep ElementId prefixes distinct per surface (`discover-feed-…` /
  `library-feed-…`).
- Do not touch backend, schema, RSS/ID3, playlist, playback, or contributor
  identity rows.
- Do not change any other `EntityActionVm` call site's payload (must remain
  `None`).

Acceptance criteria:
- Both feed detail paths use the shared helper.
- VM payload populated for the three identity kinds, `None` elsewhere.
- Architecture guard green.
- All checks green.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test entity_action_vm_carries_identity_payload`
- `cargo test release_feed_identity_actions_use_shared_renderer`
- `cargo test`
- `cargo clippy -- -D warnings`
- `git diff --check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
