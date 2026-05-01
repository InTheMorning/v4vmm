# Post-ADR 0026 Follow-Up Plan

## Status

Active follow-up index -- 2026-05-01. Tasks 001-003 applied. Current deferred
work is indexed in `docs/plans/deferred-architecture-work-index.md`.

## Goal

Keep ADR 0026 closed while organizing the remaining work needed to make the
shared entity projection architecture visually consistent, easier to theme, and
less coupled to GPUI screen state.

## Baseline

- ADR 0026 is implemented.
- Discover feed detail and Library album detail both route through
  `render_release_detail_shell`.
- Contributor panels store local `ContributorView` rows and render images,
  website actions, and Nostr actions through shared projections and
  `identity_action_button`.
- `src/views.rs` and `src/view_models/entity_detail.rs` remain GPUI-free.
- Screens still own command dispatch, popover state, image-cache resolution,
  and service calls.
- Screenshot smoke coverage now exists as a local manual review for one shared
  release-detail fixture.

## Planning Rule

Do not reopen ADR 0026 for this work. ADR 0026 established the projection and
slot-shell contract. Follow-up work should become either a bounded task or a
new ADR only when it changes a durable boundary.

Create a new ADR only when the work changes one of these contracts:

- projection inputs or action descriptors
- application query/service ownership
- database schema
- image-cache or artwork resolution
- public API/source-fact preservation

Use task files for visual verification, audits, and bounded implementation that
does not change those contracts.

## Default Execution Order

Track 1 is mandatory first. After Track 1, the visual smoke review may demote,
skip, or reorder later tracks according to the triage outcomes below.

1. Visual parity and screenshot smoke.
2. Shared action-state modeling, if visual evidence shows projection-state
   gaps.
3. ADR 0024 query/service thinning.
4. Identity persistence audit.
5. Artwork source expansion.

## Track 1 — Visual Parity and Screenshot Smoke

Priority: P0.

First artifact:

- `docs/tasks/post-adr-0026-task-001-visual-smoke.md`

Purpose:

- Verify that the same release content feels like the same surface in Discover
  and Library.
- Capture concrete visual evidence before changing more architecture.

Scope:

- Capture Discover and Library release-detail screenshots at the same viewport.
- Include a fixture or scenario with contributor image, website, and Nostr
  identity data.
- Compare sidebar behavior, density, metadata ordering, action prominence,
  track rows, contrast, and redundant state labels.
- Record whether each mismatch is a styling issue, missing projection state, or
  screen-owned behavior issue.

Acceptance gate:

- A review file identifies each mismatch and assigns it to a follow-up track.
- No architecture ADR is created from visual preference alone; require a
  durable boundary change.

## Track 2 — Shared Action-State Modeling

Priority: P1.

ADR:

- `docs/adr/0027-shared-entity-action-state.md`

Created because Track 1 showed the shared projection lacks state needed to
render consistent rows/actions.

Candidate scope:

- MusicBrainz status inputs.
- Compare/provenance status inputs.
- In-flight download/removal state.
- Playlist add/popover state, only if it can be modeled without leaking GPUI
  state into projections.

Design constraint:

- Add narrow GPUI-free input structs to `view_models::entity_detail`.
- Do not let projections read `SearchApp`, `LibraryApp`, `Context`, `Window`,
  services, or database rows directly.
- Keep handlers and command dispatch in screen adapters or ADR 0024
  presentation/application boundaries.

Acceptance gate:

- Projection tests cover every new state input.
- Any new architecture-test expectations are named in the task and included in
  the global verification checklist.

## Track 3 — ADR 0024 Query and Service Thinning

Priority: P1 after Track 2, or P2 if Track 1 shows no major screen-coupling
problem.

Candidate plan, to be created only if Track 1 routes work here:

- `docs/plans/adr-0024-query-service-thinning-plan.md`

Purpose:

- Move more screen-owned fetch/service work behind application queries and
  command/query boundaries while preserving the ADR 0026 projection contract.

Candidate scope:

- Feed/detail loading inputs consumed by `FeedView`, `TrackView`, and
  `ContributorView`.
- Library album/detail query shape.
- Contributor, value-route, and metadata lazy-panel loading.
- Presentation bridge behavior for updating GPUI entities after async work.

Design constraint:

- Loaded facts may come from application queries or existing screen load paths,
  but shared UI/projection modules must not fetch.
- This track belongs under ADR 0024. Do not fold service ownership changes into
  ADR 0026.

Acceptance gate:

- Screens lose direct service/database calls only for the workflows covered by
  the task.
- Any new architecture tests name the migrated workflow and prevent regression.

## Track 4 — Local Identity Persistence and Schema Audit

Priority: P2.

First artifact:

- `docs/tasks/post-adr-0026-task-002-identity-persistence-audit.md`

Likely ADR:

- Only if the audit proves a schema or persistence contract change is needed.

Purpose:

- Determine whether Library-local data preserves the identity facts now exposed
  by MusicIndex and modeled by ADR 0026.

Scope:

- Audit local feed, track, artist, contributor, and source fact persistence.
- Identify facts lost when remote MusicIndex entities become local Library
  rows.
- Document whether contributor `href`, `img`, `npub`, `source_links`, and
  `source_ids` survive local workflows.

Acceptance gate:

- Produce an explicit preservation matrix.
- Propose a schema ADR only for proven data-loss gaps.

## Track 5 — Artwork Source Expansion

Priority: P2.

Likely task first:

- `docs/tasks/post-adr-0026-task-003-artwork-source-expansion.md`

Likely ADR:

- Only if cache, DB, or public artwork contracts change.

Purpose:

- Make non-URL `ArtworkRef` variants useful in real rendering paths.

Candidate scope:

- `ArtworkRef::CacheKey`
- `ArtworkRef::LocalPath`
- `ArtworkRef::EmbeddedBytesKey`
- Screen/image-cache adapter behavior for each variant.

Design constraint:

- Shared projections keep returning plain artwork references.
- GPUI image handles remain screen/adapter-owned.

Acceptance gate:

- Each supported artwork source has a rendering adapter path and unit or smoke
  coverage.
- Unsupported variants remain explicit rather than silently falling back.

## Triage Outcomes From Track 1

The visual smoke review routed work as follows:

- Styling or contrast mismatch: create a bounded ADR 0025 task.
- Missing projection/action state: ADR 0027 is proposed.
- Screen-owned service/fetch behavior blocks consistency: create the ADR 0024
  query/service thinning plan.
- Remote identity facts are lost in local Library data: create an identity
  persistence/schema ADR.
- Artwork cannot render from a non-URL source: defer until a producer/resolver
  contract exists.

Track 4 and Track 5 are both P2 and may proceed in parallel if they touch
separate files and neither requires a schema/cache contract decision first.

## Applied Outcomes

- Task 001 visual smoke found that ADR 0026 shell parity is working, but row
  actions and detail state still differ enough to justify ADR 0027 shared
  entity action state. `docs/adr/0027-shared-entity-action-state.md` is now
  proposed with a phase plan, review checklist, and Task 001 packet.
- Task 002 identity audit found concrete local data-loss paths for contributor
  identity and source-fact provenance. ADR 0028 now proposes additive local
  source-fact persistence so Library can hydrate known identity facts while
  offline.
- Task 003 artwork audit found that `ArtworkRef::Url` is the only constructed
  and supported variant. Non-URL artwork variants should stay unsupported until
  a future ADR defines resolver and storage ownership.
- Post-ADR 0028 Task 001 completed Library contributor identity visibility.
- ADR 0029 now proposes the artist/person identity persistence follow-up.

## Verification

The global verification checklist is canonical for implementation tasks:

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
```

Run broader `cargo test` before marking a track complete. Visual tasks should
also attach or reference screenshots for the compared states.

## Non-Goals

- Do not redesign the app visually without screenshot evidence.
- Do not make one large ADR for all deferred items.
- Do not move network, database, or service calls into shared projection or
  shared UI modules.
- Do not claim the app is fully GPUI-independent; GPUI is thinner, but screens
  still own presentation adapters and handlers.
