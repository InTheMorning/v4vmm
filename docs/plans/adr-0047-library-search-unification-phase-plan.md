# ADR 0047 Phase Plan: Library + Search Unification

Status: Proposed - 2026-05-14.

Companion to `docs/adr/0047-library-search-unification.md` and
`docs/plans/library-search-unification-plan.md` (pre-ADR concept
artifact). Builds on ADR 0046 (Workspace Frame Architecture).

## ADR 0046 Conflict Resolutions

No blocking conflicts. Three integration points handled in this plan:

1. **Frame chrome ownership (ADR 0046 invariant 5).** Breadcrumbs and
   filter chips render through `frame_shell` composite only; this
   plan extends `FrameShellDisplay` rather than adding a sibling
   chrome composite.
2. **Transitional whole-screen mount (ADR 0046 task 007).** Phase F
   retires `src/search.rs` and unbundles screens into real
   Source/Content/Detail frame slots. `WORKSPACE_RENDER_ENABLED`
   toggle retires when Phase F completes.
3. **Single-frame nav state (ADR 0046 phase 2).** `FrameNavigationState`
   ownership generalizes from `LibraryApp` to the workspace VM keyed
   by `WorkspaceFrameId`. This work is best sequenced after ADR 0046
   phase 5 task 012 (frame add/remove + multi-frame nav state).

## Phase A - Concept Ratification

Goal: record ADR 0047 and ratify this phase plan before implementation
begins.

Status: Completed - 2026-05-14.

Deliverables:

- `docs/adr/0047-library-search-unification.md`
- `docs/plans/adr-0047-library-search-unification-phase-plan.md`
  (this file)
- First implementation task packet (Task 001)

Acceptance: completed. ADR records invariants, resolved decisions,
conflict handling, and consequences. Phase plan enumerates tasks per
phase.

Risks: none until implementation begins.

## Phase B - View-Model Groundwork

Goal: introduce GPUI-free VM contracts that downstream phases consume.
No visible UI change.

Status: Implemented - 2026-05-14.

Tasks:

- `adr-0047-task-001-content-filter-vm`
- `adr-0047-task-002-inspector-panel-state-vm`
- `adr-0047-task-003-description-collapse-vm`
- `adr-0047-task-004-search-results-inspector-vm`
- `adr-0047-task-005-saved-search-vm`

Deliverables:

- `ContentFilter` enum + `FilterChipStripDisplay`
- `InspectorPanelKind` + `inspector_expanded_panels` set +
  `compare_id3_enabled` / `musicbrainz_enabled` predicates
- `DescriptionState` enum + 5-line threshold projector
- `SearchResultsInspectorPageVm` with tabbed contract + paged tabs
- `SavedSearchEntry` on source-list VM

Acceptance: VMs compile, document, and unit-test. No screen modified.

Risks: paged-tab windowing complexity. Mitigation: reuse ADR 0041
windowed VM machinery.

## Phase C - Inspector Rewiring

Goal: apply the new inspector VMs to the existing track inspector
without changing search/library navigation.

Status: Implemented - 2026-05-14. Awaiting operator visual
confirmation before Phase D.

Tasks:

- `adr-0047-task-006-disable-compare-musicbrainz-on-undownloaded`
- `adr-0047-task-007-gate-library-extra-fields-behind-expanded-panels`
- `adr-0047-task-008-description-disclosure`

Deliverables:

- Compare ID3 + MusicBrainz controls render disabled (HIG-dimmed +
  tooltip) when `is_downloaded = false`
- Library-extra fields (file path, ID3 frame groups, format warnings,
  MB match detail) render only when the corresponding panel is
  expanded for downloaded items
- Description section gains disclosure with auto-collapse threshold
- Architecture guards for each invariant

Acceptance: Library track inspector compact by default; expansion is
explicit; non-downloaded items cannot reveal extra-field groups.

Risks: visible regressions in Library workflow. Mitigation: keep
existing fields visible to expanded state; default to expanded for
backward parity until D ships filter chips.

## Phase D - Per-Frame Filter Chips

Status: Implemented for Phase E sequencing - Task 010 Library-backed visible
wiring and Task 011 toolbar scope retirement are implemented and visually
confirmed - 2026-05-15.

Goal: relocate the All/Library/Index control from toolbar scope chips
to per-frame chrome chips.

Tasks:

- `adr-0047-task-009-filter-chip-strip-composite`
- `adr-0047-task-010a-content-list-page-vm-ownership`
- `adr-0047-task-010-wire-filter-chips-into-content-list-frame`
- `adr-0047-task-011-retire-global-search-scope`

Deliverables:

- Filter chip strip composite reading `FilterChipStripDisplay`, with
  narrow-width pull-down collapse
- `FrameShellDisplay` extended with optional filter-chip slot
- A GPUI-free `ContentListPageVm` owns per-frame filter state,
  source-aware row projection, empty-filter state, and chip-strip
  display data before UI wiring
- `SetFrameFilter(frame_id, ContentFilter)`-style dispatch wired into
  the Library-backed `ContentList` frame VM without introducing a
  global filter store
- `GlobalSearchScope` and the toolbar segmented control removed
- Architecture guards: no toolbar filter control; no global filter
  store

Current acceptance: the Library-backed `ContentList` frame renders its
own chip strip, filter changes apply only to that frame, empty filter
results use VM-owned copy, and the toolbar no longer duplicates the
per-frame source filter. No explicit narrow-menu proof was requested before
Phase E began; narrow pull-down visual proof remains in the Phase G visual
inventory.

Risks: HIG drift in narrow-mode pull-down. Mitigation: reuse existing
pull-down primitive; visual proof in Phase G.

## Phase E - Search-Results Inspector and Breadcrumbs

Status: In progress - Task 012 frame-navigation ownership landed
2026-05-15. Task 013 is next and owns frame-shell breadcrumb rendering.

Goal: route global search submit into a `Detail` frame rendering the
tabbed `SearchResultsInspector` with breadcrumb chrome.

Tasks:

- `adr-0047-task-012-frame-breadcrumb-vm` - implemented 2026-05-15
- `adr-0047-task-013-frame-shell-breadcrumb-render`
- `adr-0047-task-014-search-results-inspector-shell`
- `adr-0047-task-015-search-submit-and-saved-search-commands`

Deliverables:

- Per-frame breadcrumb projection from generalized
  `FrameNavigationState` (workspace-VM-owned, multi-frame keyed)
- `frame_shell` composite renders breadcrumbs with middle-ellipsis
  truncation; back chevron remains
- Tabbed search-results inspector shell rendering Artists / Feeds /
  Tracks tabs through existing track/feed/artist composites
- `SubmitGlobalSearch` opens or focuses a `Detail` frame; saved-search
  activation dispatches `OpenSavedSearch(id)`

Acceptance: search submit produces an inspector with breadcrumbs;
drilling pushes nav state; saved searches in source list open the
same inspector.

Risks: nav state generalization cascades into Library callers.
Mitigation: ship the workspace-VM ownership move as a discrete prior
step inside task 012; do not change observable behavior until task
015 wires the new command.

## Phase F - Retire `src/search.rs`

Goal: delete the standalone Search screen module; route all
artist/feed/track render through shared composites consumed by
content-list and inspector shells.

Tasks:

- `adr-0047-task-016-retire-search-screen-module`

Deliverables:

- `src/search.rs` deleted; shared composites (`ui_artist`, `ui_feed`,
  `ui_track`) host all entity render
- Library and search-result inspectors call the same composites
- Feed and track membership controls are a single shared
  Download/Remove action vocabulary. UI labels must not expose
  subscribe/unsubscribe wording; feed removal must leave any visible
  remote feed detail showing the `Download Feed` action state.
- `WORKSPACE_RENDER_ENABLED` toggle removed; workspace render is the
  only path
- Architecture guards: search.rs absent; no `crate::search::*` import

Acceptance: app builds and runs without `src/search.rs`; all entity
inspectors render identically across library and search origins.

Risks: hidden call-site coupling. Mitigation: search-screen retirement
is the last code-deletion step; preceding phases keep parallel paths
alive.

## Phase G - Guards and Visual Proof

Goal: lock invariants and capture L/D visual proof.

Tasks:

- `adr-0047-task-017-final-guards-and-visual-readiness`

Deliverables:

- Architecture guards spanning all Phase B-F invariants
- `docs/reviews/adr-0047-review-checklist.md` with pass/fail per gate
  and visual proof for: default layout, search submit, drilled
  search result, library track inspector compact, library track
  inspector with Compare ID3 expanded, Compare ID3 disabled on
  undownloaded track, Description collapsed, Description expanded,
  filter chip strip, filter chip pull-down at narrow width, tabbed
  search inspector, saved search opened from source list — all in
  light and dark themes.

Acceptance: review checklist records `Proceed`.

## Cross-Phase Verification

Every phase runs:

- `cargo fmt -- --check`
- `cargo check`
- `cargo test`
- `cargo clippy -- -D warnings`

Architecture-test guards land alongside the structural change they
protect.

## Suggested Execution Order

1. Tasks 001-005 in parallel (Phase B VM contracts).
2. Tasks 006-008 sequentially (Phase C inspector rewire).
3. Tasks 009-011 sequentially (Phase D filter chips).
4. Tasks 012-015 sequentially (Phase E search results + breadcrumbs).
5. Task 016 (Phase F retire search.rs).
6. Task 017 (Phase G guards + visual proof).

Phase B can begin before ADR 0046 Phase 4 lands. Phase E should wait
for ADR 0046 phase 5 task 012 (multi-frame nav state) to avoid
double-refactor of nav-state ownership.
