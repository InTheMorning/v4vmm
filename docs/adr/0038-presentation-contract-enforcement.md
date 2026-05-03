# ADR 0038: Presentation Contract Enforcement

## Status

Proposed - 2026-05-03. Supersedes
`docs/plans/one-owner-per-surface-plan.md` and
`docs/plans/post-adr-0033-ui-consolidation-plan.md`. Both are retained as
historical artifacts; their invariants and surface inventory are absorbed
into this ADR and its phase plan.

## Context

ADR 0023 through ADR 0037 produced the design-system layers (tokens,
primitives, composites), the GPUI-free view-model layer, and a growing set
of architecture tests. They also produced two "ideal architecture" planning
documents — `one-owner-per-surface-plan.md` and
`post-adr-0033-ui-consolidation-plan.md` — that ran in parallel and now
overlap. The state today:

- Composites and primitives are in good shape: ~30 architecture guards
  enforce backend-boundary, callback-hygiene, floating-chrome, token, and
  fallback-string rules. Most of the relevant guard baselines are at zero.
- Top-level shells (`src/ui_artist.rs`, `src/ui_entity.rs`,
  `src/ui_feed.rs`, `src/ui_track.rs`) are shared GPUI shells but live next
  to screens at `src/*.rs`, so directory-scoped tests need the
  `KNOWN_SHARED_UI_SHELL_FILES` allowlist (which omits `ui_feed.rs` and
  `ui_track.rs`, classifying them as "presentation glue").
- Screen monoliths persist: `src/library.rs` is 3,907 LOC,
  `src/search.rs` is 6,445 LOC, `src/view_models/library.rs` is 2,832 LOC,
  `src/view_models/search.rs` is 2,878 LOC. Roughly 16k LOC concentrated in
  four files.
- Apple HIG concerns are referenced in plans but not operationalized.
  Accessibility-label coverage is one method (`tag_badge.rs:166`).
  `src/ui/style.rs` still contains raw `rgb(0x…)` literals.
  Dark-mode parity is partial; dynamic-type tolerance is undefined.
- ADR 0037 produced `ReleaseDetailPageVm` + `EntityActionVm.payload` as the
  "page-level VM contract consumed by a single shell helper" pattern. That
  pattern is the right shape for every entity surface but is currently
  applied only to feed and track detail.

The lesson from 0023–0037 is that visual consistency follows from
mechanical ownership rules, not from review. ADR 0038 generalizes the
pattern across the app, names the architectural layers explicitly, and
makes Apple HIG foundations a first-class invariant rather than a footnote.

## Decision

Adopt the following as app-wide architectural invariants. Every UI change
must name which contract or invariant it strengthens, or be reframed
before implementation.

### Architectural Invariants

These are organized in three groups: presentation contracts (per-concept
ownership), layer architecture (where code lives), and HIG foundations
(what the UI must guarantee for users).

#### Presentation contracts

A presentation concept is "repeated" when it appears in two or more
screens, two or more composites, or one screen and one composite. Examples:
headers, rows, action strips, popovers, fallback labels, metadata
sections, empty states, icon treatments, spacing, color roles, button
tones, external-link chrome.

Every repeated concept must satisfy:

1. **Shared owner.** Repeated chrome lives in `src/ui/primitives` or
   `src/ui/composites`; screens only compose and wire callbacks.
2. **GPUI-free display contract.** Display strings, fallback labels,
   availability, command intents, and derived presentation facts live in
   `src/view_models`, `src/views.rs`, or a co-located display struct
   consumed by a composite. Public composite signatures do not accept
   policy-bearing `String`/`&str`; they accept VM fields or display
   structs.
3. **Token and icon discipline.** Spacing, sizing, color, typography,
   radius, and icon identity use named tokens and components rather than
   raw literals, glyph strings, or one-off wrappers.
4. **Additive context behavior.** Library, Discover, playlist, and playback
   surfaces may expose different commands. Those commands attach through
   named slots on the shared surface; they do not fork the page skeleton.
5. **Regression guard.** The change that consolidates a concept adds or
   tightens an architecture test, VM unit test, visual-smoke requirement,
   or baseline reduction. Baselines may shrink; they may not grow.

#### Page-level invariant

6. **Page VM and shell helper.** Every entity detail page renders through
   a shell helper that consumes a single `<Entity>DetailPageVm` value
   (e.g. `ReleaseDetailPageVm`, `TrackDetailPageVm`,
   `ArtistDetailPageVm`, `PlaylistDetailPageVm`). Screens supply hero
   images and command callbacks; they do not assemble the page from
   individual VM accessors. ADR 0037 introduced this pattern for feed and
   track surfaces; ADR 0038 makes it the structural shape for every
   entity surface.

#### Layer architecture

The app has eight layers. Each layer may import only from layers strictly
below it. New code must name its layer; tests enforce the boundary.

```
8. screens                 (src/app/, src/library.rs, src/search.rs)
7. ui shells               (src/ui/shells/)
6. ui composites           (src/ui/composites/)
5. ui primitives           (src/ui/primitives/)
4. ui foundations          (src/ui/tokens.rs, theme_*, icons, layouts)
3. view models             (src/view_models/, src/views.rs)
2. application             (src/application/)
1. backend / services      (src/db.rs, src/api.rs, src/*_service.rs, src/rss/)
```

Layers 4–6 are GPUI-aware but domain-agnostic. Layer 3 is GPUI-free.
Layer 7 (shells) is GPUI-aware top-level layout that consumes view-models
and composites only — it does not import from screens, services, or
backend. Layer 8 is screen wiring: command dispatch, selected-entity
state, image resolution.

`src/ui_artist.rs`, `src/ui_entity.rs`, `src/ui_feed.rs`, and
`src/ui_track.rs` are layer-7 modules and must move under
`src/ui/shells/` so directory-scoped tests cover them automatically. The
`KNOWN_SHARED_UI_SHELL_FILES` allowlist is removed in the same change.

#### HIG foundations

Apple HIG-compliant UI is treated as a structural property, not a polish
pass. Every composite and shell must satisfy:

7. **Theme adaptivity.** All colors resolve through `theme_bridge` or
   `theme_profiles`. No raw `rgb(...)`/`Rgba` outside the token layer.
   Light and dark themes are first-class; visual smoke captures both for
   any user-visible change.
8. **Accessibility contract.** Every composite that renders interactive
   chrome (button, popover trigger, list row with selection, action
   strip) exposes a VM-sourced accessibility label and, where the action
   is non-obvious, an accessibility hint. Accessibility strings are not
   computed in screens; they live in the VM next to the display label
   they describe. Composites without interactive chrome are exempt;
   pure-text primitives are exempt.
9. **Dynamic-type tolerance.** Layout uses `.scaled(cx)` token reflow.
   Hard-coded line counts, fixed-pixel widths, and clipped text are not
   acceptable except behind a token. A child ADR governs the full
   dynamic-type story (text scale ramps, max scale, truncation policy);
   ADR 0038 only requires that the existing scale-token discipline holds
   and is mechanically enforced.

### Enforcement

ADR 0038 is enforced in layers:

- `tests/architecture_tests.rs` is the mechanical gate. New top-level
  GPUI modules must be classified in the same change. Allowlists may
  shrink; they may not grow without an explicit follow-up entry.
- Each task under this ADR names the surface owner, the display
  contract, the forbidden screen-local pattern, the layer affected, and
  the visual-smoke surface before code edits.
- Visual smoke is evidence for HIG foundations — light + dark, both
  themes — not a substitute for a contract.
- Symptom-only visual fixes are not accepted unless they are the smallest
  step in a contract-strengthening task.

## Non-Goals

- No SwiftUI, AppKit, or framework migration.
- No backend, schema, RSS, ID3, playlist, playback, or service redesign.
- No broad visual redesign or palette change.
- No reduction in desktop density for its own sake; HIG macOS guidance
  permits dense workflows.
- No attempt to make context-specific workflows identical;
  context-specific commands remain additive.
- No full dynamic-type policy in this ADR; deferred to a child ADR.

## Alternatives Considered

- **Continue fixing inconsistencies as individual bugs.** Rejected — the
  failure mode that ADRs 0031–0037 were meant to prevent.
- **Freeze all UI work until everything is fixed.** Rejected — blocks
  useful structural improvements and encourages oversized patches.
- **One mega-composite for every page.** Rejected — the app needs typed
  slot composition, not a single inflexible layout owner.
- **Keep `one-owner-per-surface-plan.md` and the post-0033 plan as
  separate live documents.** Rejected — they overlap and drift; one ADR
  with one phase plan is the pattern that worked for 0035–0037.
- **Full screen monolith split before VM consolidation.** Rejected —
  splitting `library.rs`/`search.rs` before fallback policy moves out of
  them just relocates the mess. Sequence is VM consolidation first, then
  decomposition.

## Consequences

- Future UI work starts by naming the layer, the presentation owner,
  the display contract, and the HIG foundation it touches.
- `library.rs`, `search.rs`, `view_models/library.rs`, and
  `view_models/search.rs` will shrink and decompose as policy moves into
  smaller VMs and into composites. Target file layout is named in the
  phase plan; per-surface migration tasks are sequenced.
- `EntityActionVm.payload` (ADR 0037) becomes the canonical pattern for
  any clickable VM action. Pages adopt `<Entity>DetailPageVm` as their
  contract.
- Architecture tests sharpen: new guards land for composite display
  contracts, accessibility labels on interactive composites, and layer
  classification. Some tasks are mostly guard work before visible
  changes ship.
- Visual smoke captures both light and dark per surface. A11y label
  coverage becomes inspectable from VMs.
- Plan churn drops: one ADR + one phase plan + N task files. The two
  superseded plans are headered as historical and not maintained.

## Superseded Plans

- `docs/plans/one-owner-per-surface-plan.md` (2026-05-02). Surface
  inventory, fallback table, and Workstreams 0–5 are absorbed.
- `docs/plans/post-adr-0033-ui-consolidation-plan.md` (2026-05-01).
  Render-helper consolidation and shell relocation are absorbed.

Both files carry a "Superseded by ADR 0038" header. Their bodies remain
as historical context.

## Follow-Up Work

- Implement
  `docs/plans/adr-0038-presentation-contract-enforcement-phase-plan.md`.
- Tasks are sequenced by blast radius (highest first):
  1. `docs/tasks/adr-0038-task-001-layer-relocation.md`
  2. `docs/tasks/adr-0038-task-002-composite-display-contract-audit.md`
  3. `docs/tasks/adr-0038-task-003-library-search-vm-consolidation.md`
  4. `docs/tasks/adr-0038-task-004-dark-mode-parity-audit.md`
  5. `docs/tasks/adr-0038-task-005-accessibility-label-contract.md`
  6. `docs/tasks/adr-0038-task-006-page-vm-generalization.md`
  7. `docs/tasks/adr-0038-task-007-screen-decomposition.md`
  8. `docs/tasks/adr-0038-task-008-final-sweep-and-readiness-gate.md`
- A child ADR will cover dynamic-type policy (text scale ramps, max
  scale, truncation rules). To be opened after Task 005 lands.

## Enforcing Tests (current and planned)

Current (already in `tests/architecture_tests.rs`):

- `screens_do_not_inline_unknown_artist_or_album_fallbacks`
- `screens_do_not_inline_untitled_fallback`
- `screens_do_not_coerce_empty_feed_url_to_empty_string`
- `screens_do_not_duplicate_render_helpers_without_baseline`
- `screens_do_not_define_inline_icon_svg_helpers`
- `screens_do_not_reintroduce_raw_color_or_numeric_px_literals`
- `ui_buttons_do_not_reintroduce_raw_leading_glyphs`
- `ui_components_do_not_bypass_theme_profile_resolution`
- `top_level_gpui_modules_are_classified_as_screen_or_shared_ui`
- `shared_ui_components_do_not_import_backend_or_screen_layers`
- `shared_ui_callbacks_do_not_smuggle_backend_types`
- `presentation_modules_do_not_hand_roll_floating_chrome`
- (~20 others; see file)

Planned under ADR 0038:

- `top_level_shells_live_under_src_ui_shells` — replaces
  `KNOWN_SHARED_UI_SHELL_FILES` allowlist after Task 001.
- `composite_signatures_take_display_contracts_not_loose_strings` —
  Task 002.
- `view_models_own_display_fallbacks_for_library_and_search` — Task 003.
- `interactive_composites_carry_accessibility_labels` — Task 005.
- `entity_detail_pages_render_through_shell_helper_and_page_vm` —
  Task 006.
- `library_screen_modules_are_decomposed_under_src_ui_shells_library` —
  Task 007 (and the discover analog).
