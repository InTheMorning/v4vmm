# ADR 0037 Task 002: Track External-Link Parity

## Status

Implemented in code. Automated evidence is green; track-detail visual smoke
remains pending.

## Goal

Make the same normal track expose track identity external links from one shared
VM-backed renderer across Library and Discover. The existing
`TrackDetailSurface` already owns header and summary grammar; this task removes
the remaining screen-local Nostr/Website identity-link strip drift.

## Inventory Findings

- `src/ui/composites/track_detail_surface.rs` owns the track header, summary,
  description, section, and advanced-panel layout.
- `src/search.rs` builds Discover track external chrome in
  `render_track_header_subtitle`: feed navigation, audio play, and Nostr copy.
- `src/library.rs` renders Library track detail through `TrackDetailSurface`
  but does not supply a matching track identity external-link strip.
- `TrackDetailVm` does not yet expose track identity actions as
  `Vec<EntityActionVm>` with payloads.

## Files To Inspect

- `docs/adr/0037-same-entity-surface-parity.md`
- `docs/plans/adr-0037-same-entity-surface-parity-phase-plan.md`
- `src/ui_track.rs`
- `src/search.rs`
- `src/library.rs`
- `src/view_models/track_detail.rs`
- `src/view_models/entity_detail.rs`
- `src/ui/composites/identity_action.rs`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/view_models/track_detail.rs` — add track identity action projection and
  VM tests.
- `src/ui_track.rs` — add shared renderer for track identity external actions.
- `src/search.rs` — call the shared renderer and remove screen-local track
  Nostr button construction from the track detail strip.
- `src/library.rs` — call the shared renderer for track detail external links.
- `tests/architecture_tests.rs` — add guard against screen-local track Nostr
  identity button construction.
- `docs/reviews/adr-0037-review-checklist.md` — update Task 002 evidence.
- `docs/reviews/adr-0037-task-002-review.md` — implementation review.

## Do Not Touch

- Backend services
- Database schema
- RSS/ID3 parsing or metadata comparison logic
- Playback driver semantics
- Playlist behavior
- Library-only advanced metadata panels and MusicBrainz compare panels
- Contributor identity rows

## Constraints

- Preserve click behavior for identity links:
  - Website opens with `open::that(payload)`.
  - Nostr copies `payload` to the clipboard.
- Preserve Discover-only feed navigation and audio play controls. They remain
  screen-bound because they dispatch `SearchApp` navigation/play behavior, not
  identity external-link behavior.
- Preserve Library-only advanced metadata panels as additive panels.
- Use `EntityActionVm.payload` for clickable identity actions.
- Keep ElementId namespaces distinct per surface:
  `discover-track-...` and `library-track-...`.

## Implementation Steps

1. In `src/view_models/track_detail.rs`, import
   `EntityActionKind`, `EntityActionTarget`, `EntityActionTone`, and
   `EntityActionVm`.
2. Add `TrackDetailVm::identity_actions(&self) -> Vec<EntityActionVm>`.
   It should:
   - build a target from `TrackView.id` when present;
   - add `OpenWebsite` with label `Website` and payload from
     `track.identity.website_url`;
   - add `CopyNostr` with label `Copy Nostr` and payload from
     `track.identity.nostr_npub`;
   - return an empty vector when there is no track id.
3. Add VM tests proving Website/Nostr payloads are present and that an
   unidentified track emits no identity actions.
4. In `src/ui_track.rs`, add:

   ```rust
   #[must_use]
   pub(crate) fn render_track_identity_actions(
       detail: &TrackDetailVm<'_>,
       id_prefix: &str,
   ) -> Vec<TrackSurfaceElement>;
   ```

   Map `OpenWebsite` to `IdentityActionKind::Website` and `CopyNostr` to
   `IdentityActionKind::Nostr`. Skip other action kinds. Click behavior is
   hardcoded from the payload.
5. In Discover track detail (`src/search.rs`), append the shared identity
   action elements to the existing external-link slot and remove the Nostr
   argument/path from `render_track_header_subtitle`.
6. In Library track detail (`src/library.rs`), pass
   `render_track_identity_actions(&detail_vm, "library-track")` into
   `TrackDetailSurface::external_links`.
7. Add architecture guard `track_identity_links_use_shared_renderer`:
   - `src/search.rs` and `src/library.rs` must not contain
     `IdentityActionKind::Nostr` in track-detail code paths.
   - `src/ui_track.rs` must define `fn render_track_identity_actions`.
8. Update the ADR 0037 review checklist and add a task review.

## Acceptance Criteria

- `TrackDetailVm::identity_actions()` returns Website/Nostr actions with
  payloads for tracks whose identity facts contain those values.
- Discover and Library track detail both render track identity external links
  through `render_track_identity_actions`.
- Discover still keeps feed navigation and audio play behavior.
- Library advanced panels remain untouched.
- The architecture guard is green.
- `cargo fmt -- --check`, `cargo check`, targeted tests, `cargo test`,
  `cargo clippy -- -D warnings`, and `git diff --check` are green.
- Light and dark screenshots are reviewed for Library and Discover track detail.

## Test Commands

- `cargo fmt -- --check`
- `cargo check`
- `cargo test track_detail_identity_actions_carry_payloads`
- `cargo test track_identity_links_use_shared_renderer`
- `cargo test`
- `cargo clippy -- -D warnings`
- `git diff --check`

## Escalation Triggers

- If track Website/Nostr facts are unavailable in Library because local
  hydration is missing, stop and create a follow-up hydration task rather than
  inferring metadata in the renderer.
- If Discover feed navigation or audio play must move into a shared renderer,
  stop and update the ADR first because those actions are command-bound, not
  identity external-link actions.
- If Library-only advanced panels need structural changes, stop and split that
  into a separate task.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0037-same-entity-surface-parity.md`
- `docs/plans/adr-0037-same-entity-surface-parity-phase-plan.md`
- `docs/tasks/adr-0037-task-002-track-header-action-parity.md`
- `src/ui_track.rs`
- `src/search.rs`
- `src/library.rs`
- `src/view_models/track_detail.rs`
- `src/view_models/entity_detail.rs`
- `src/ui/composites/identity_action.rs`
- `tests/architecture_tests.rs`

Goal:
- Add `TrackDetailVm::identity_actions()` returning Website/Nostr
  `EntityActionVm`s with payloads.
- Add `ui_track::render_track_identity_actions(detail, id_prefix)`.
- Route Discover and Library track detail through the helper for Website/Nostr
  identity external links.
- Preserve Discover feed navigation/audio play and Library advanced panels.

Constraints:
- Do not redesign track detail.
- Do not touch backend, DB schema, RSS/ID3 parsing, playback, playlist, or
  metadata comparison semantics.
- Do not move SearchApp feed navigation or audio play into the identity helper.
- Keep ElementId prefixes distinct per surface.

Do not touch:
- Backend services
- Database schema
- RSS/ID3 parsing
- Playback driver
- Playlist behavior
- Library-only advanced metadata panels
- Contributor identity rows

Acceptance criteria:
- `TrackDetailVm::identity_actions()` carries Website/Nostr payloads.
- Discover and Library track detail call
  `render_track_identity_actions(&detail_vm, ...)`.
- `track_identity_links_use_shared_renderer` is green.
- Required cargo checks pass.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test track_detail_identity_actions_carry_payloads`
- `cargo test track_identity_links_use_shared_renderer`
- `cargo test`
- `cargo clippy -- -D warnings`
- `git diff --check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
