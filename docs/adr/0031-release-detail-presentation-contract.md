# ADR 0031: Release Detail Presentation Contract

## Status

Implemented - 2026-05-02.

## Context

ADR 0030 fixed several Discovery and Library correctness issues, but visual
smoke exposed a deeper composition problem: Library and Discovery can now render
the same release through a shared shell while still producing visibly different
results, each independently wrong.

The current failure is not primarily a Rust, GPUI, or metadata problem. The
screens still let source data and screen-local slots decide too much of the
visible page structure:

- One surface can be sparse, with weak hierarchy and a track list that feels
  disconnected from the release.
- Another surface can be overly dense, with raw website and Nostr values leaking
  into the hero area.
- The description can appear in more than one place.
- Library and Discovery can diverge by omission or by local override instead of
  by a deliberate surface policy.

The result is a UI that has shared plumbing but no shared product contract.

## Decision

Introduce a canonical release-detail presentation contract between source
metadata, view-model data, and GPUI rendering.

Library and Discovery should both project feed, album, and release-like data
into the same presentation shape before rendering. The renderer should render
that shape. It should not decide which raw metadata fields deserve hero
placement.

The contract lives in the GPUI-free projection layer,
`src/view_models/entity_detail.rs`, and should remain independent from command
dispatch, image-cache lookup, popovers, database reads, and service calls.

Proposed shape:

```rust
pub struct ReleaseDetailPageVm<'a> {
    pub hero: ReleaseHeroVm<'a>,
    pub primary_actions: Vec<ReleaseActionVm<'a>>,
    pub identity_actions: Vec<ReleaseIdentityActionVm<'a>>,
    pub summary_facts: Vec<ReleaseFactVm<'a>>,
    pub panels: Vec<ReleasePanelVm<'a>>,
    pub tracks: ReleaseTrackSectionVm<'a>,
}
```

The exact type names may change to match the existing view-model vocabulary,
but the contract must preserve these zones:

1. Hero.
2. Actions.
3. Summary facts.
4. Optional panels.
5. Tracks.

## Presentation Rules

### Hero

The hero is for human-readable identity and a small set of orienting visuals
only:

- Artwork.
- Entity kind badge.
- Title.
- Creator, publisher, or artist subtitle.
- Optional short human-readable supporting line.

The hero must not contain:

- Raw URLs.
- Raw `npub` or Nostr identifiers.
- Long GUIDs.
- Multi-line descriptions.
- Metadata-table rows.

### Actions

Actions must be visually separate from facts.

Primary actions should be limited to the actions the user can reasonably take
on the release in the current surface. Library and Discovery may differ here,
but the difference must come from action projection policy, not from a different
page skeleton.

Identity actions, such as Website, Nostr, RSS, and similar outbound affordances,
should render as compact actions. Their raw values should not render in the
hero.

### Summary Facts

Summary facts are compact, ordered, and capped.

Recommended order:

1. Release kind.
2. Release date.
3. Tracks.
4. Duration.
5. Language or explicitness only when relevant.

The contract should cap the visible summary facts to the small set that
helps orient the user. Additional source facts belong in panels or metadata
inspection, not the first viewport.

### Panels

Panels are for longer or secondary content:

- Description.
- Identity details.
- Provenance/source facts.
- Contributor summaries.
- Metadata comparison details.

The description must appear in exactly one place. If it appears as a panel, it
must not also appear in the hero or summary facts.

Identity details may expose full URLs, Nostr IDs, GUIDs, and source facts, but
they should be demoted below the summary rather than competing with the title.

### Tracks

The track section must use one dense, stable layout across Library and
Discovery:

- Consistent number column.
- Consistent artwork/thumb behavior.
- Title and secondary metadata aligned predictably.
- Duration and row actions placed consistently.
- Empty and loading states handled by the same section contract.

Surface-specific track actions are allowed, but the row skeleton should not
change by surface.

## Surface Policy

The presentation contract should accept an explicit surface context, reusing
`EntitySurfaceContext` where possible.

Discovery:

- Shows subscription/download/listening actions relevant to remote content.
- Hides Library-only compare actions.
- May show Website/Nostr/RSS identity actions.
- Should not show raw local-file metadata panels unless a local file exists and
  the surface explicitly opts into that panel.

Library:

- Shows local management, playlist, MusicBrainz, and compare actions when
  allowed by existing action-state policy.
- Uses the same hero, summary facts, panels, and track section skeleton as
  Discovery.
- May show richer local metadata panels below the first viewport.

## Invariants

- Source facts remain preserved. The contract decides only presentation
  placement.
- View models stay GPUI-free.
- Screen modules continue to own command dispatch, async service calls,
  popovers, subscriptions, image-cache lookup, and database access.
- No schema migration is part of this ADR.
- No source-fact inference is introduced.
- Long machine identifiers are never first-viewport hero content.
- A release description appears exactly once.
- Library and Discovery share the same page skeleton.
- No nested vertical scroll views are introduced. One vertical scroll view per
  detail surface, as defined by ADR 0030.

## Alternatives Considered

- Keep patching `DetailHeader` and release slots. Rejected because slots alone
  do not prevent raw metadata from leaking into inappropriate zones.
- Add more screen-local conditionals. Rejected because it preserves divergent
  Library and Discovery behavior.
- Create a purely visual GPUI composite contract. Rejected because the decision
  about what belongs in each zone should happen before rendering and should be
  testable without GPUI.
- Hide more fields in the current renderer. Rejected because this would fix one
  screenshot while leaving the underlying composition contract implicit.

## Consequences

- The next implementation should move release-detail composition into a typed
  projection before rendering.
- Existing `ReleaseDetailVm`, `ReleaseDetailSlots`, and `ReleaseDetailSurface`
  should be adapted if they can express the contract cleanly.
- If the existing names become misleading, rename or wrap them in a bounded
  task rather than creating a parallel release-detail system.
- Tests should assert projection output: hero content, summary ordering,
  description placement, hidden raw IDs, and surface-specific actions.
- Visual smoke remains required because automated tests cannot fully verify the
  composition quality.

## Non-Goals

- No navigation redesign.
- No database or API shape change.
- No metadata persistence change.
- No new identity-ingest behavior.
- No change to MusicBrainz, download, playlist, playback, or subscription
  semantics.
- No generalized dashboard/card redesign outside release-like detail pages.

## Implementation Plan

Tasks must be executed in order. Task 002 depends on Task 001. Task 003 covers
only the visual row template that Task 002 deliberately leaves abstract. If
Task 002 absorbs row-template parity in practice, Task 003 collapses into it.

### Task 001: Contract Types, Action Projection, and Projection Tests

Add the canonical presentation contract in `src/view_models/entity_detail.rs`,
either by adapting the existing `ReleaseDetailVm` or by introducing
`ReleaseDetailPageVm` alongside it with a clear migration path off
`ReleaseDetailVm`. Implement identity-action projection (Website, Nostr, RSS,
and similar outbound affordances) from source facts in the same task, since
the primary/identity action split is part of the contract.

Acceptance criteria:

- `ReleaseDetailPageVm` (or the renamed equivalent) accepts `&FeedView` and
  `EntitySurfaceContext`, mirroring existing `ReleaseDetailVm::new`.
- Projection tests prove that hero text excludes raw URLs, `npub` values, long
  GUIDs, and multi-line descriptions.
- Summary facts render in the documented order (kind, date, tracks, duration,
  language/explicitness when relevant) and the visible list is capped at five.
- Description appears in a panel and not in the hero or summary facts.
- Website, Nostr, and RSS affordances land in `identity_actions` and never in
  `primary_actions`, asserted per surface.
- Discovery and Library produce the same structural zones for equivalent
  release data. Surface-specific differences appear only in action lists.
- All projection tests are GPUI-free.

### Task 002: Renderer Adoption and Slot Retirement

Update the shared release detail shell (`render_release_detail_shell` in
`src/ui_entity.rs`) to consume the contract directly and migrate every caller
(`src/ui_feed.rs` and any Library equivalent) in lock-step. The shell
signature changes from `ReleaseDetailSlots` to `&ReleaseDetailPageVm`. Treat
this as a breaking change and update all construction sites in the same task.
Retire or narrow `ReleaseDetailSlots` so it cannot reintroduce the
screen-local-decision failure mode described in the Context section.

Acceptance criteria:

- Library album/feed and Discovery feed details render through the same page
  contract.
- The shell reads only from the contract. It does not access `FeedView` or
  source-fact fields directly.
- `ReleaseDetailSlots` is either deleted or reduced to slots that cannot carry
  hero, description, or summary content.
- Existing action handlers remain screen-owned (callbacks injected via the
  contract, dispatch resolved by the screen).
- The single-vertical-scroll invariant from ADR 0030 holds.

### Task 003: Track Row Visual Template

Normalize the visual row template of the track section across Library and
Discovery. Scope is limited to row geometry. The section structure itself is
expected to already come from Task 002.

Acceptance criteria:

- Row column order is fixed: number, artwork/thumb, title and secondary
  metadata, duration, surface action slot.
- Number column width and row height are constants in one place, applied to
  both surfaces.
- The surface action slot lives at one named position on the row. Surfaces
  populate it but cannot reorder it.
- Empty and loading states are owned by `ReleaseTrackSectionVm` (or the
  renamed equivalent) and rendered by one shared component on both surfaces.

### Task 004: Visual Smoke, Regression Pass, and Cleanup

Run manual smoke against an enumerated set of representative releases and
confirm that screen-owned behavior still works after the contract migration.
Cleanup means removing slot fields, helpers, or screen-local conditionals that
the new contract makes dead.

Smoke fixture list (each must be exercised on Library and Discovery where
applicable):

- Release with Website, Nostr, and a multi-paragraph description.
- Release with an empty description.
- Release with zero tracks.
- Release with 100+ tracks.
- Release with only podcast/RSS identity.
- Library release with full local-file metadata.

Acceptance criteria:

- First viewport for every fixture has a clear title, creator, restrained
  primary actions, compact summary facts, and a visible start of the track
  section.
- Description appears exactly once on every fixture.
- Raw identity values appear only in demoted panels or in copy/open actions.
- Regression check: Library compare, download, playlist add, MusicBrainz
  lookup, and playback still trigger from the new contract path.
- Dead `ReleaseDetailSlots` fields, helpers, and screen-local conditionals
  superseded by the contract are removed.
- Screenshots for each fixture are attached or referenced from a review
  document.

## Review Checklist

- Does the hero contain only the documented identity and orienting visuals
  (artwork, kind badge, title, subtitle, optional supporting line)?
- Are raw URLs, `npub` values, GUIDs, and long machine IDs absent from the hero?
- Are primary actions and identity actions visually distinct, with Website,
  Nostr, and RSS rendered as identity actions?
- Are identity-detail panels demoted below the summary rather than competing
  with the title?
- Does the description render exactly once?
- Are summary facts capped at five and in the documented order?
- Do Library and Discovery share the same skeleton?
- Are differences limited to surface policy and action availability?
- Are command dispatch and services still screen-owned?
- Are projection tests GPUI-free?
- Does manual smoke show the track section starting in the first viewport when
  content exists?
