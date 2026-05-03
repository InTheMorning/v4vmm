# ADR 0038 Presentation Contract Enforcement Phase Plan

## Goal

Enforce presentation contracts, layer architecture, and HIG foundations
across the app so visual consistency is produced by structure rather than
review. Phases are ordered by **blast radius, highest first**: structural
moves that change file layout and contracts come before per-surface
cleanup, because every cleanup task consumes the structure laid down
above it.

This plan absorbs `docs/plans/one-owner-per-surface-plan.md` and
`docs/plans/post-adr-0033-ui-consolidation-plan.md`. Those documents are
superseded; their surface inventory and Workstreams are merged here.

## Non-Goals

- No backend, schema, RSS, ID3, playlist, playback, or service behavior
  changes.
- No SwiftUI/AppKit migration.
- No broad redesign or palette change.
- No completion of every surface in one pass.
- No screenshot-only acceptance without an accompanying contract or
  guard.
- No dynamic-type ramp design (deferred to a child ADR after Task 005).

## Current State

- ~30 architecture guards already cover backend boundary, callback
  hygiene, floating chrome, tokens, fallback strings, and helper
  duplication. Most baselines are at zero.
- ADR 0037 produced `ReleaseDetailPageVm` + `EntityActionVm.payload` as
  the page-VM/shell-helper pattern for feed and track detail surfaces.
- Top-level shells live next to screens at `src/*.rs`. The
  `KNOWN_SHARED_UI_SHELL_FILES` allowlist covers
  `src/ui_artist.rs` and `src/ui_entity.rs` only; `src/ui_feed.rs` and
  `src/ui_track.rs` are misclassified as presentation glue.
- Screen monoliths: `src/library.rs` 3,907 LOC, `src/search.rs` 6,445
  LOC. View-model monoliths: `src/view_models/library.rs` 2,832 LOC,
  `src/view_models/search.rs` 2,878 LOC.
- HIG: `src/ui/style.rs` contains raw `rgb(0x…)` literals (lines
  105–114). Accessibility-label coverage is one method
  (`tag_badge.rs:166`). Dark-mode infrastructure exists (`theme_bridge`,
  `theme_profiles`) but parity is not audited.
- One genuine render-helper duplicate: `render_track_row` in both
  `src/ui_track.rs:93` and `src/search.rs:4428`.
- 9 import sites across the codebase reference the four shell shell
  modules — relocation is mechanical.

## Target Architecture

```
src/
├── application/                application layer (commands, queries)
├── view_models/                GPUI-free display contracts
│   ├── entity_detail.rs        ReleaseDetailPageVm + EntityActionVm
│   ├── track_detail.rs         TrackDetailPageVm
│   ├── artist_detail.rs        ArtistDetailPageVm (new)
│   ├── playlist_detail.rs      PlaylistDetailPageVm (new)
│   ├── library/                Library-specific VM submodules
│   └── discover/               Discover-specific VM submodules
├── ui/
│   ├── tokens.rs / theme_*     foundations
│   ├── primitives/             layer 5
│   ├── composites/             layer 6 (display-contract enforced)
│   └── shells/                 layer 7
│       ├── artist.rs           (was src/ui_artist.rs)
│       ├── entity.rs           (was src/ui_entity.rs)
│       ├── feed.rs             (was src/ui_feed.rs)
│       ├── track.rs            (was src/ui_track.rs)
│       ├── library/            decomposed library screens
│       └── discover/           decomposed discover screens
├── library.rs                  thin entry; routes to ui/shells/library/
├── search.rs                   thin entry; routes to ui/shells/discover/
└── app/                        composition root, event wiring
```

The `library.rs`/`search.rs` monoliths shrink to thin entry modules.
Their interior splits into per-surface files under
`src/ui/shells/{library,discover}/`. Each per-surface file is bound by a
soft 500-LOC ceiling.

## Phases (blast radius descending)

### Phase 1 — Layer Relocation (highest blast radius, mechanical)

Move shells under `src/ui/shells/`. Establish the layer 7 directory.
Remove the `KNOWN_SHARED_UI_SHELL_FILES` allowlist hack. Single-commit
structural change that simplifies every downstream test.

Deliverables:
- `src/ui/shells/{artist,entity,feed,track}.rs` (relocated).
- `src/lib.rs` re-exports updated.
- `src/ui/mod.rs` adds `pub mod shells;`.
- All 9 import sites updated.
- `tests/architecture_tests.rs`: drop `KNOWN_SHARED_UI_SHELL_FILES`, add
  `top_level_shells_live_under_src_ui_shells`.
- Visual smoke: existing screenshots remain valid; no UI behavior change.

### Phase 2 — Composite Display-Contract Audit

Every public composite signature accepts a VM field, a display struct, or
a pure passthrough — never a policy-bearing `String`/`&str`.

Deliverables:
- Per-composite doc-comment naming the contract type (e.g. `TrackHeader`
  takes `&TrackHeaderVm`, not `(title: String, subtitle: String)`).
- Co-located display structs where a full VM is overkill (e.g.
  `ActionRowItem`).
- New guard:
  `composite_signatures_take_display_contracts_not_loose_strings` with a
  per-composite allowlist for legitimate passthrough.
- Migration of any caller using the old loose API.

### Phase 3 — Library/Search VM Consolidation

Hoist all fallback policy from `library.rs` and `search.rs` into
`view_models/library.rs` and `view_models/search.rs` (and possibly new
submodules under `view_models/library/` / `view_models/discover/`).
Screens read `display_*` accessors; screens never decide what an empty
value means.

Targets (verified by grep on 2026-05-02):
- `Untitled` / `[untitled]` — `TrackVm::display_title`,
  `PlaylistVm::display_name`.
- `Unknown Artist` — `TrackVm::display_artist`.
- `Unknown Album` — `TrackVm::display_album`.
- `feed_url.unwrap_or_default()` — `FeedVm::display_url ->
  Option<String>` (preserve empty-vs-unknown distinction).
- `Tags` section title — `view_models/track_metadata_grid.rs`.
- Other fallbacks discovered during the pass.

Deliverables:
- VM accessors with unit tests (present / empty / `None`).
- Screen call-site sweep.
- New guard: `view_models_own_display_fallbacks_for_library_and_search`.

### Phase 4 — HIG Foundations: Dark-Mode Parity Audit

Verify every composite resolves through `theme_bridge`. Remove raw
`rgb(0x…)` literals outside the token layer (notably
`src/ui/style.rs:105-114`). Visual-smoke baseline for both themes
across all main surfaces.

Deliverables:
- `src/ui/style.rs` cleaned or absorbed into tokens.
- Visual smoke: light + dark for Library list, Library inspector,
  Discover list, Discover inspector, release detail, track detail,
  playlist popover, now-playing bar.
- Existing `screens_do_not_reintroduce_raw_color_or_numeric_px_literals`
  guard tightened to also cover `src/ui/style.rs` (or `style.rs` is
  removed).

### Phase 5 — HIG Foundations: Accessibility-Label Contract

Every composite that renders interactive chrome exposes an
`accessibility_label` (and, where the action is non-obvious, an
`accessibility_hint`). Strings come from VMs, not screens. Pure-text
primitives are exempt.

Deliverables:
- VM fields `display_*_a11y_label` for any composite-bound action.
- Composite signatures accept the a11y label as a typed field.
- New guard:
  `interactive_composites_carry_accessibility_labels` listing the
  composites required to expose the label.
- Coverage table in the review checklist.
- Pointer to a child ADR for the dynamic-type ramp; opened after this
  phase lands.

### Phase 6 — PageVm Generalization

Apply the ADR 0037 page-VM + shell-helper pattern to every entity
detail surface that does not yet have it.

Deliverables:
- `ArtistDetailPageVm` (`view_models/artist_detail.rs` or extension to
  `view_models/artist.rs`) consumed by `ui::shells::artist`.
- `PlaylistDetailPageVm` (new `view_models/playlist_detail.rs`) consumed
  by a new shell helper.
- `TrackDetailPageVm` if not already factored — the existing
  `TrackDetailVm` may already be the contract; verify.
- Search-result page surfaces (e.g. recent-feed tile rows) use the same
  `EntityActionVm.payload` pattern for any clickable identity action.
- New guard:
  `entity_detail_pages_render_through_shell_helper_and_page_vm`.

### Phase 7 — Screen Decomposition

Split `library.rs` and `search.rs` along surface lines under
`src/ui/shells/library/` and `src/ui/shells/discover/`. Each per-surface
file ≤ 500 LOC.

Suggested initial decomposition (refine when starting):

```
src/ui/shells/library/
├── mod.rs
├── sidebar.rs
├── feed_list.rs
├── feed_detail.rs
├── track_detail.rs
├── playlist_detail.rs
└── now_playing.rs

src/ui/shells/discover/
├── mod.rs
├── recent.rs
├── result_list.rs
├── feed_inspector.rs
├── track_inspector.rs
└── search_input.rs
```

`src/library.rs` and `src/search.rs` shrink to thin entry modules that
own selected-entity state and delegate rendering to the shell modules.

Deliverables:
- Per-surface files under `src/ui/shells/{library,discover}/`.
- `library.rs` and `search.rs` reduced to ≤ 500 LOC each (target;
  enforced by a guard if practical).
- New guards:
  `library_screen_modules_are_decomposed_under_src_ui_shells_library`
  and the discover analog.
- Visual smoke: confirm every surface still renders correctly.

### Phase 8 — Final Sweep + Readiness Gate

Eliminate residual fallbacks, retire the `render_track_row` duplicate,
confirm baselines are all at zero. Produce the readiness gate.

Deliverables:
- `render_track_row` consolidated; the duplicate at
  `src/search.rs:4428` is removed and the shared
  `src/ui/shells/track.rs::render_track_row` is the single owner.
- Final fallback-string sweep using grep evidence.
- Visual-smoke summary table covering every main surface in light + dark.
- Accessibility-label coverage report.
- Readiness gate decision in the review checklist: "Proceed" or
  "Defer" for richer playlist/playback work.

## Task Sequence

| # | Task | File |
|---|---|---|
| 1 | Layer Relocation                              | `docs/tasks/adr-0038-task-001-layer-relocation.md` |
| 2 | Composite Display-Contract Audit              | `docs/tasks/adr-0038-task-002-composite-display-contract-audit.md` |
| 3 | Library/Search VM Consolidation               | `docs/tasks/adr-0038-task-003-library-search-vm-consolidation.md` |
| 4 | Dark-Mode Parity Audit                        | `docs/tasks/adr-0038-task-004-dark-mode-parity-audit.md` |
| 5 | Accessibility-Label Contract                  | `docs/tasks/adr-0038-task-005-accessibility-label-contract.md` |
| 6 | PageVm Generalization                         | `docs/tasks/adr-0038-task-006-page-vm-generalization.md` |
| 7 | Screen Decomposition                          | `docs/tasks/adr-0038-task-007-screen-decomposition.md` |
| 8 | Final Sweep + Readiness Gate                  | `docs/tasks/adr-0038-task-008-final-sweep-and-readiness-gate.md` |

Tasks 4 and 5 may run in parallel after Task 2 lands; they touch
different concerns (color vs. label). Task 3 may split into 003a
(Library) / 003b (Discover) once a fallback inventory is in hand.
Task 7 may split per-surface (sidebar / list / detail / popover).

Only Task 001 is fully specified now. Later tasks are stubs and should
be expanded from the structural shape laid down by their predecessors —
not redesigned from scratch.

## HIG Basis

- Apple HIG `foundations/dark-mode.md`, `foundations/materials.md` —
  dark-mode parity is required, not optional. Phase 4.
- Apple HIG `summaries/accessibility-complete.md` — accessibility
  labels and hints are properties of controls, not screens. Phase 5.
- Apple HIG `summaries/layout-complete.md` — predictable hierarchy and
  flexible layout. Enforced via tokens (existing) plus shell pattern
  (Phase 6).
- Apple HIG `summaries/typography-complete.md` — text scale tolerance
  is required. Existing scale-token discipline holds; full dynamic-type
  ramp is deferred to a child ADR after Phase 5.
- Apple HIG `components/buttons.md`, `components/popovers.md` — control
  role and command intent live with the composite. Existing composite
  layer enforces this; tightened by Phase 2.

## Risks

- **Layer relocation breaks something subtle.** Mitigation: 9 import
  sites is small; verify with `cargo check` and visual smoke. Revert is
  a single `git revert` of the move commit.
- **Composite contract changes ripple widely.** Mitigation: per-composite
  audit is incremental; one composite at a time, each with its own
  caller migration. Don't bundle.
- **VM consolidation is a 5,700-LOC touch surface.**
  Mitigation: split Task 003 by surface; extract one fallback at a time
  with a unit test before deletion.
- **Screen decomposition can churn if started too early.**
  Mitigation: it is Phase 7, after VMs and shells are clean. Decomposing
  before VM consolidation just relocates the mess.
- **Accessibility-label contract is a new surface area.**
  Mitigation: start with composites that already have interactive
  chrome; don't try to retrofit every primitive at once. Phase 5 names a
  bounded composite list.
- **Dark-mode parity audit may surface design issues, not just bugs.**
  Mitigation: Phase 4 is an audit + raw-rgb cleanup pass. Real palette
  decisions land in a follow-up under ADR 0034 or a child ADR.

## Test Strategy

Per implementation task:
- `cargo fmt -- --check`
- `cargo check`
- targeted VM or architecture tests
- `cargo test`
- `cargo clippy -- -D warnings`
- `git diff --check`
- visual smoke for affected surfaces (light + dark) when user-visible

Per phase completion:
- All baselines remain at zero (or have shrunk).
- New guards added in the same change as the consolidation.
- Review checklist updated with task results, guard names, and visual
  smoke ledger entries.

## Rollback Strategy

Every phase is independently revertible:
- Phase 1 (relocation): revert the move commit.
- Phase 2 (composite contracts): revert per-composite; the audit is
  incremental.
- Phase 3 (VM consolidation): revert per-fallback; VM accessors are
  additive.
- Phase 4 (dark-mode audit): revert per cleanup commit.
- Phase 5 (a11y): a11y additions are purely additive.
- Phase 6 (PageVm): each PageVm migration is a single composite + one
  shell helper.
- Phase 7 (decomposition): revert the per-surface move commit.
- Phase 8 (sweep): individual changes are tiny.
