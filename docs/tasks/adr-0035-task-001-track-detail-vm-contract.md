# ADR 0035 Task 001: Track Detail VM Contract

## Goal

Add a GPUI-free `TrackDetailVm` family that owns shared track display facts,
row projection, labels, fallback policy, typed slot descriptors, load state,
and summary row order for Library and Discover.

## Files to Inspect

- `docs/adr/0035-track-surface-consolidation.md`
- `docs/plans/adr-0035-track-surface-consolidation-phase-plan.md`
- `src/view_models/track.rs`
- `src/view_models/library.rs`
- `src/search.rs`
- `src/library.rs`
- `src/api.rs`

## Files Likely to Change

- `src/view_models/track_detail.rs`
- `src/view_models/mod.rs`
- `src/view_models/track.rs` only if shared helper reuse is needed
- `tests/architecture_tests.rs` for the VM/string ownership guards that
  land in this task (see Implementation Steps 13–14)
- `docs/reviews/adr-0035-review-checklist.md`

## Do Not Touch

- `src/library.rs`
- `src/search.rs`
- `src/ui/composites/`
- Backend, schema, services, playback, playlist behavior

## Constraints

- VM code must not import GPUI or shared UI modules.
- Do not invent metadata or infer facts not already present.
- Preserve existing title and artist fallback behavior from `TrackVm`.
- Field labels, fallback strings, and section titles belong here, not in
  screens.
- Preserve empty-vs-unknown distinctions with `Option<String>` where the
  composite should render nothing instead of a fallback.
- Keep UI construction out of the VM; action, link, contributor, value route,
  section, and advanced panel types are display contracts, not GPUI elements.
- Do not put resolved image handles in the VM. Artwork lookup is screen-owned
  and artwork display is composite-owned; the VM only owns artwork fallback
  facts such as display kind and accessibility text.
- `TrackRowVm` is a projection of `TrackDetailVm`, not a parallel fallback
  policy.

## Implementation Steps

1. Create `src/view_models/track_detail.rs`.
2. Define `TrackDetailSurfaceContext` with at least `Library` and `Discover`.
3. Define `TrackDetailVm` over source facts already exposed to the UI; do not
   import GPUI or service modules.
4. Define `TrackRowVm` as the row projection of `TrackDetailVm`.
5. Define `TrackDetailLabels` as the only owner of canonical field labels and
   section titles.
6. Define `TrackDetailLoadState` (`Loaded`, `Loading`, `Missing`,
   `Failed { reason }`).
7. Define typed display contract structs/enums for `TrackDetailSlots`,
   `ActionRowItem`, `ExternalLinkItem`, `ContributorItem`, `ValueRouteItem`,
   `TrackDetailSection`, and `TrackDetailAdvancedPanel`.
   `TrackDetailSlots` must not include GPUI image handles.
8. Project header facts: title, artist, kind label, release context, track
   number, duration, release date, publisher, description, and row trailing
   metadata.
9. Bind fallback accessors in one place: `display_title`, `display_artist`,
   `display_album`, `display_release_context`, `display_kind_badge`, and the
   summary section title.
10. Define summary row structs using plain strings and optional max-line
    policy.
11. Add unit tests for Library and Discover contexts, present/missing title,
    present/missing release/feed title, track number, duration, publisher,
    description, row projection, and fallback ownership.
12. Export the module from `src/view_models/mod.rs`.
13. Add `screens_do_not_inline_unknown_artist_or_album_fallbacks` and
    `screens_do_not_inline_untitled_fallback` to
    `tests/architecture_tests.rs`. Verified 2026-05-02: the literals
    `"Unknown Artist"`, `"Unknown Album"`, `"Untitled"`, and `"[untitled]"`
    are already absent from `library.rs`, `search.rs`, `ui_track.rs`, and
    `ui_entity.rs`, so both tests land at baseline zero. The matcher is a
    plain string-literal grep over `SCREEN_FILES`; allowlist test fixtures
    if needed.
14. Add `track_detail_labels_owns_canonical_field_labels` to
    `tests/architecture_tests.rs` with a *narrow* matcher. Bare grep for
    `"Album"`, `"Feed"`, `"Release"` in screen files would false-positive
    on log messages, error strings, match arms, and doc comments. Instead,
    scope the matcher to one of these forms:
    - literals appearing as the first argument to `Label::new(...)`,
      `text(...)`, `SectionHeader::new(...)`, or `DetailGrid` row label
      builders;
    - or, if the AST is too costly, a regex of the form
      `(Label::new|SectionHeader::new|\.label\(|\.title\()\s*\(\s*"(Album|Feed|Release|Tags)"`.
    Document the chosen matcher in the test's doc comment so reviewers know
    why it is narrow. Land at baseline zero by including any genuinely
    pre-existing match in `TRACK_DETAIL_LABEL_BASELINE_ALLOWLIST`; this
    plan expects zero entries, but allow the constant to exist for future
    edge cases.
15. Update the review checklist.

## Acceptance Criteria

- `TrackDetailVm` is GPUI-free.
- `TrackRowVm`, `TrackDetailLabels`, `TrackDetailLoadState`, and typed slot
  value types exist and are GPUI-free.
- Summary labels and fallback strings are generated by the VM, not screen
  code.
- Unit tests pin fallback behavior, label behavior, row projection, and
  empty-vs-unknown behavior.
- `screens_do_not_inline_unknown_artist_or_album_fallbacks`,
  `screens_do_not_inline_untitled_fallback`, and
  `track_detail_labels_owns_canonical_field_labels` pass at baseline zero.
- No UI rendering behavior changes yet.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test track_detail
cargo test --test architecture_tests
git diff --check
```

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0035-track-surface-consolidation.md`
- `docs/plans/adr-0035-track-surface-consolidation-phase-plan.md`
- `docs/tasks/adr-0035-task-001-track-detail-vm-contract.md`
- `src/view_models/track.rs`
- `src/search.rs`
- `src/library.rs`

Goal:
- Add a GPUI-free `TrackDetailVm` family for shared track display facts,
  labels, fallbacks, row projection, typed slot descriptors, and load state.

Constraints:
- No GPUI imports in view models.
- No screen migration in this task.
- Preserve existing fallback behavior.
- Do not invent metadata.
- Do not create untyped `AnyElement`/callback-bag slots in the VM contract.
- Do not add GPUI image types to the VM contract.

Do not touch:
- `src/library.rs`
- `src/search.rs`
- Backend/schema/service files.
- UI composite files.

Acceptance criteria:
- `TrackDetailVm`, `TrackRowVm`, `TrackDetailLabels`,
  `TrackDetailLoadState`, and typed slot value types exist and are exported.
- Unit tests cover fallback, label, row projection, and load-state behavior.
- Checks pass.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test track_detail`
- `cargo test --test architecture_tests`
- `git diff --check`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- If the VM needs facts not present on the existing display-fact sources
  (`api::Track`, `db::Track` projections already exposed via `views.rs` /
  existing VMs, or other projection helpers), stop and document the missing
  source fact instead of adding inference. Library and Discover both supply
  facts; do not assume `api::Track` is the only source.
- If Library-only DB state appears necessary and is not already projected,
  stop and split a Library adapter task rather than importing DB types into
  the VM.
- If the narrow label-matcher (Implementation Step 14) cannot land at
  baseline zero — i.e. a real `Label::new("Album")`-style call exists in
  screens today — stop and either add the offending site to
  `TRACK_DETAIL_LABEL_BASELINE_ALLOWLIST` with a one-line reason, or hoist
  the literal to `TrackDetailLabels` immediately. Do not loosen the matcher.
