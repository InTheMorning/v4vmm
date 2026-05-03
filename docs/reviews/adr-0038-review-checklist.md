# ADR 0038 Review Checklist

## Reviewed Artifacts

- `docs/adr/0038-presentation-contract-enforcement.md`
- `docs/plans/adr-0038-presentation-contract-enforcement-phase-plan.md`
- `docs/tasks/adr-0038-task-001-layer-relocation.md`
- `docs/tasks/adr-0038-task-002-composite-display-contract-audit.md`
- `docs/tasks/adr-0038-task-003-library-search-vm-consolidation.md` (stub)
- `docs/tasks/adr-0038-task-004-dark-mode-parity-audit.md` (stub)
- `docs/tasks/adr-0038-task-005-accessibility-label-contract.md` (stub)
- `docs/tasks/adr-0038-task-006-page-vm-generalization.md` (stub)
- `docs/tasks/adr-0038-task-007-screen-decomposition.md` (stub)
- `docs/tasks/adr-0038-task-008-final-sweep-and-readiness-gate.md` (stub)

## Gate Status

Status: Task 003 (Library/Search VM Consolidation) is in progress.
Task 002 migrated `TrackRow`, `DetailHeader`, `DetailGrid`,
`ReleaseDetailSurface::track_section`, `AddToPlaylistPopover`, and
`TrackMetadataGrid` cells off public loose string builders. The
`ActionRow`, `DisclosureGroup`, `SegmentedControl`, `TagBadge`, and
`action_button`
generic-control sub-slices have also moved status messages, disclosure
labels, segment labels, badge labels, and metadata-action labels to
display contracts, tightening the composite display-contract guard.
Task 003 has started by moving the Discover track-inspector feed-link
label and URL fallback into `TrackFeedLinkDisplay`, then moving
payment-route address and custom-field presence into `PaymentRouteVm`.

## Required Questions For Every UI Change

- Which architectural layer does this change live in (1–8)?
- What repeated presentation concept is changing?
- Who owns it: primitive, composite, shell, or VM?
- What display contract carries title, subtitle, metadata, state,
  availability, fallback labels, and command intent?
- For interactive composites: what accessibility label and hint?
- For visible changes: light + dark visual smoke?
- Which screen-local pattern is being deleted or prevented?
- Which architecture test, unit test, visual smoke, or baseline
  reduction blocks the regression from returning?
- If Library and Discover differ, is the difference a named additive
  context policy rather than a forked page skeleton?

## Contract Matrix

| Contract | Pass Condition | Common Failure |
|---|---|---|
| Shared owner | Repeated chrome lives in `src/ui/primitives` or `src/ui/composites` | Copying a row/header/action helper into another screen |
| Display contract | GPUI-free VM or co-located display struct owns labels and state | Screen calls `unwrap_or("Untitled")` or formats availability locally |
| Token/icon discipline | Named tokens and icon catalog are used | Raw `px`, `rgb`, glyph strings, inline SVGs outside allowed layers |
| Additive context policy | Context-specific actions attach through slots | Library/Discover rebuild different page skeletons for the same entity |
| Regression guard | Guard lands with the consolidation | Visual patch lands without test or visual-smoke evidence |
| Page VM | Entity detail pages render through shell helper + `<Entity>DetailPageVm` | Screen assembles a page from individual VM accessors |
| Layer architecture | Imports respect layer order; shells live under `src/ui/shells/` | Top-level `src/ui_*.rs` shell file added; layer skipped |
| Theme adaptivity | Colors resolve through `theme_bridge`; light + dark verified | Raw `rgb(0x…)` outside the token layer |
| Accessibility contract | Interactive composites carry VM-sourced a11y labels | Icon-only button without label; screen-side a11y string |

## Task Results

| # | Task | Status | Required Evidence |
|---|---|---|---|
| 1 | Layer Relocation                              | Implemented with visual-proof caveat | Files moved under `src/ui/shells/`; `KNOWN_SHARED_UI_SHELL_FILES` removed; `top_level_shells_live_under_src_ui_shells` green. Visual proof remains open; no provisional screenshot artifacts are retained. |
| 2 | Composite Display-Contract Audit              | Implemented | `TrackRow` row number/title/duration now enter through `TrackRowVm` or `SharedTrackRowVm`; `DetailHeader` title/subtitle/data rows now enter through `DetailHeaderDisplay`; `DetailGrid` key/value rows now enter through `DetailElementRow` or `DetailTextRow`; release track sections now enter through `ReleaseTrackSectionDisplay`; playlist popovers now enter through `AddToPlaylistDisplay` and `PlaylistOptionDisplay`; metadata grid cells now enter through `TrackMetadataGroupDisplay`, `TrackMetadataFieldDisplay`, `TrackMetadataFrameDisplay`, `TrackMetadataTagDisplay`, and `TrackMetadataTextDisplay`; action-row status messages now enter through `ActionRowMessageDisplay`; disclosure headings now enter through `DisclosureGroupDisplay`; segmented options now enter through `SegmentDisplay`; badge labels now enter through `TagBadgeDisplay`; metadata-action labels now enter through `ActionButtonDisplay`; guard renamed/tightened to scan multi-line signatures; allowlist shrank by the old `TrackRow`, `DetailHeader`, `DetailGrid`, release track-section, playlist popover, metadata-grid cell, action-row message, disclosure-group, segmented-control, tag-badge, and action-button string builders |
| 3 | Library/Search VM Consolidation               | In progress - first three Discover slices implemented | `TrackFeedLinkDisplay` now carries Discover track-inspector feed guid, label, and URL; `TrackInspectorHeaderVm::feed_link_display()` owns `feed_title -> guid` and `feed_url -> feed_guid`; `PaymentRouteVm::address()` owns optional payment-route address display without coercing empty strings; `PaymentRouteVm::custom_fields()` owns optional payment-route `key ...` / `value ...` display without coercing empty strings; `src/search.rs` no longer reconstructs the feed-link label fallback, payment-route address presence, or payment-route custom-field formatting; `view_models_own_display_fallbacks_for_library_and_search` added/tightened |
| 4 | HIG Dark-Mode Parity Audit                    | Stub           | `style.rs` resolution; light + dark screenshots per surface |
| 5 | HIG Accessibility-Label Contract              | Stub           | A11y labels per interactive composite; new guard; coverage table |
| 6 | PageVm Generalization                         | Stub           | `<Entity>DetailPageVm` per surface; shell helpers; new guard |
| 7 | Screen Decomposition                          | Stub           | `library.rs`/`search.rs` ≤ 500 LOC; per-surface files under `src/ui/shells/{library,discover}/`; new guards |
| 8 | Final Sweep + Readiness Gate                  | Stub           | All baselines zero; full visual smoke; gate decision recorded |

## Visual Smoke Ledger

To be filled per task. All entries require both themes. File path
convention: `docs/reviews/screenshots/adr-0038-{surface}-{theme}.png`.

| Surface | Light | Dark | Fixture | Status |
|---|---|---|---|---|
| Library list                | TBD | TBD | TBD | Pending Task 004 |
| Library inspector           | TBD | TBD | TBD | Pending Task 004 |
| Library track detail        | TBD | TBD | TBD | Pending Task 004 |
| Library feed detail         | TBD | TBD | TBD | Pending Task 004 |
| Discover list               | TBD | TBD | TBD | Pending Task 004 |
| Discover inspector          | TBD | TBD | TBD | Pending Task 004 |
| Discover track detail       | TBD | TBD | TBD | Pending Task 004 |
| Discover feed detail        | TBD | TBD | TBD | Pending Task 004 |
| Playlist popover            | TBD | TBD | TBD | Pending Task 004 |
| Now-playing bar             | TBD | TBD | TBD | Pending Task 004 |
| Recent feed tiles           | TBD | TBD | TBD | Pending Task 004 |
| Search results              | TBD | TBD | TBD | Pending Task 004 |

Task 001 relocation smoke (light theme only, per task packet):

| Surface | Screenshot | Status |
|---|---|---|
| Library shell | None retained | Needs deterministic/manual recapture before counting as acceptance evidence |
| Discover shell | None retained | Needs deterministic/manual recapture before counting as acceptance evidence |

Visual-proof caveat, 2026-05-03: coordinate-driven X11 captures were
discarded and are not retained in the repository. Automated gates are
green, but visual acceptance for the relocation remains open until a
reviewer captures or verifies Library and Discover through a
deterministic/manual process.

## Accessibility Coverage Ledger

To be filled by Task 005. Composites that render interactive chrome:

| Composite | A11y label source | A11y hint? | Guard entry | Status |
|---|---|---|---|---|
| `action_button`              | TBD | TBD | TBD | Pending |
| `ActionRow`                  | TBD | TBD | TBD | Pending |
| `identity_action_button`     | TBD | TBD | TBD | Pending |
| `AddToPlaylistPopover`       | TBD | TBD | TBD | Pending |
| `TrackRow`                   | TBD | TBD | TBD | Pending |
| `ListRow`                    | TBD | TBD | TBD | Pending |
| `RecentFeedTile`             | TBD | TBD | TBD | Pending |
| `DisclosureGroup`            | TBD | TBD | TBD | Pending |
| `SegmentedControl`           | TBD | TBD | TBD | Pending |
| `NowPlayingBar`              | TBD | TBD | TBD | Pending |
| Release detail action overlays | TBD | TBD | TBD | Pending |
| Track detail action overlays  | TBD | TBD | TBD | Pending |

Pure-text/pure-layout composites are exempt:
`detail_grid`, `detail_header`, `multiline_text`, `divider`, `loading`.

## Automated Checks

For Task 001:

- `cargo fmt -- --check`
- `cargo check`
- `cargo test top_level_shells_live_under_src_ui_shells`
- `cargo test`
- `cargo clippy -- -D warnings`
- `git diff --check`

For each subsequent task: re-run the full set, plus the task's
targeted guard test.

For Task 002 current slices:

- `cargo test composite_signatures_take_display_contracts_not_loose_strings`

For Task 003 current slices:

- `cargo test track_inspector_header_vm_projects_feed_link_display_contract`
- `cargo test payment_route_vm_projects_address_without_coercing_presence`
- `cargo test payment_route_vm_projects_custom_fields_without_coercing_presence`
- `cargo test view_models_own_display_fallbacks_for_library_and_search`

## Readiness Gate (filled by Task 008)

| Question | Answer | Evidence |
|---|---|---|
| All architecture guards green, baselines zero?           | TBD | TBD |
| Every entity detail page on PageVm + shell helper?       | TBD | TBD |
| Every interactive composite carries a11y label?           | TBD | TBD |
| Light + dark visual smoke covers every main surface?     | TBD | TBD |
| `library.rs` and `search.rs` are thin entries?           | TBD | TBD |
| Zero remaining screen-local fallback strings?            | TBD | TBD |
| Deferred-architecture-work index reconciled?             | TBD | TBD |

Decision: TBD (`Proceed` or `Defer`).
