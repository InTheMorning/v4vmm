# ADR 0026 Task 001: Identity Facts

## Goal

Extend the API and source-normalized view fact layer so MusicIndex identity
data for artists, feeds/items/tracks, and contributors is deserialized,
copied into local source-fact structs, preserved, and available to later
shared projections.

## Read

- `docs/adr/0026-shared-entity-projection-layer.md`
- `docs/plans/adr-0026-shared-entity-projection-phase-plan.md`
- `../musicindex/api.json`
- `src/api.rs`
- `src/views.rs`
- `src/search.rs` identity helpers around feed/track Nostr rendering
- `src/feed_service.rs` source fact defaults

## Files Likely To Change

- `src/api.rs`
- `src/views.rs`

## Do Not Touch

- Do not migrate Discover or Library rendering in this task.
- Do not introduce `src/ui_entity.rs` in this task unless needed for compile
  scaffolding.
- Do not add database migrations.
- Do not move service calls or screen handlers.
- Do not change playlist, download, MusicBrainz, or playback behavior.

## Constraints

- Keep the fact layer GPUI-free.
- Preserve raw `source_links` and `source_ids`; convenience identity fields
  must not replace provenance.
- Do not expose concrete `api::SourceEntityLink` or `api::SourceEntityId`
  values as public fields on shared view fact types. Convert them into local
  GPUI-free `IdentityLinkFact` and `IdentityIdFact` structs.
- Do not infer identity from names, titles, publisher text, filenames, or fuzzy
  matching.
- Prefer small helper functions with focused unit tests over screen-level
  extraction logic.
- Follow existing Rust style and keep clippy pedantic clean for touched code.

## Implementation Steps

1. Inspect `../musicindex/api.json` examples and current `src/api.rs` structs
   for identity-bearing fields.
2. Extend `api::Contributor` to deserialize contributor identity fields,
   including `href`, `img`, and `npub`.
3. Add local source fact types, such as `IdentityLinkFact` and
   `IdentityIdFact`, to `src/views.rs`.
4. Add `ContributorView`, `EntityIdentityLinks`, and an `ArtworkRef` or
   equivalent plain-data artwork source type to `src/views.rs`.
5. Add conversion helpers from `api::SourceEntityLink` and
   `api::SourceEntityId` into the local source fact structs.
6. Convert feed/track contributor collections in shared view facts from
   `api::Contributor` to `ContributorView`.
7. Add conservative extraction helpers for Nostr, website, and image
   convenience fields from local source facts.
8. Add `identity: EntityIdentityLinks` to `ArtistView`, `FeedView`, and
   `TrackView`.
9. Update `from_api` and `from_local` constructors. Local constructors should
   populate only facts truly available locally and leave unknown identity empty.
10. Add unit tests covering:
   - contributor `href` / `img` / `npub` preservation
   - `source_ids` `nostr_npub` extraction
   - `source_links` website extraction
   - raw source fact preservation after convenience fields are populated
   - shared view facts do not expose `api::SourceEntityLink` or
     `api::SourceEntityId` as public identity fields
   - feed/track view contributor collections use `ContributorView`

## Acceptance Criteria

- New MusicIndex identity fields deserialize without dropping unknown or raw
  source facts.
- `ArtistView`, `FeedView`, `TrackView`, and `ContributorView` expose identity
  facts without importing GPUI.
- Shared view fact fields use local source fact types rather than public
  `api::*` source row fields.
- Feed and track view contributor collections use `ContributorView`, not
  `api::Contributor`.
- Existing Discover and Library behavior is unchanged.
- Tests cover positive and missing-field cases.
- `cargo fmt -- --check`, `cargo check`, targeted tests, architecture tests,
  and clippy pass.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test views::tests
cargo test api::tests
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0026-shared-entity-projection-layer.md`
- `docs/plans/adr-0026-shared-entity-projection-phase-plan.md`
- `docs/tasks/adr-0026-task-001-identity-facts.md`
- `../musicindex/api.json`
- `src/api.rs`
- `src/views.rs`
- relevant identity helpers in `src/search.rs`

Goal:
- Preserve MusicIndex identity facts in API and view fact types.

Constraints:
- No GPUI imports in `src/views.rs`.
- No rendering migration.
- No database migration.
- No service calls in identity extraction helpers.
- Raw `source_links` and `source_ids` must remain available.
- Shared view facts must not expose `api::SourceEntityLink` or
  `api::SourceEntityId` as public identity fields.

Do not touch:
- `src/library.rs`
- `src/ui_feed.rs`
- `src/ui_track.rs`
- playlist/download/MusicBrainz/playback behavior

Acceptance criteria:
- Contributor `href`, `img`, and `npub` deserialize and project into
  `ContributorView`.
- `EntityIdentityLinks` exists and is used by artist/feed/track views.
- `IdentityLinkFact` and `IdentityIdFact` or equivalent local source fact
  types preserve raw source facts.
- Feed and track view contributor collections use `ContributorView`.
- Unit tests cover identity extraction and raw fact preservation.
- Verification commands pass.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test views::tests`
- `cargo test api::tests`
- `cargo test --test architecture_tests`
- `cargo clippy --lib --tests -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- The MusicIndex API uses identity field names not represented in
  `../musicindex/api.json`.
- A database migration appears necessary.
- Existing local data cannot represent identity facts needed by Library.
- Identity extraction would require fuzzy matching or inferred artist identity.
