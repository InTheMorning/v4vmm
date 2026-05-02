# One Owner Per Surface — Plan

## Status

Proposed - 2026-05-02. Extends ADR 0033 and complements
`post-adr-0033-ui-consolidation-plan.md`. Where the post-ADR plan focuses on
duplicated `render_*` helpers between Library and Search, this plan
generalizes the rule to *every* repeated UI thing and codifies the five
ownership invariants that gate all future UI work.

## Goal

Every repeated UI thing in the app — chrome, label, action, glyph, spacing,
color, fallback string — has exactly one owner. A "thing" repeats when it
appears in two or more screens, two or more composites, or both. Once it
repeats, it must move to the design system before another caller is added.

This plan is the pre-feature hardening gate for richer playlist, playback,
and Library/Discovery UI work. Do not add richer surface behavior through a
path that still violates these ownership rules. New feature work is safe only
when its surface already has a composite owner, a display contract, tokenized
chrome, and a regression guard.

## The five ownership rules

Each repeated thing must satisfy all five:

1. **One shared composite or primitive.** Lives under
   `src/ui/composites/` or `src/ui/primitives/`. No screen, presentation
   shell, or sibling composite reimplements its layout, chrome, or
   interaction surface.
2. **One view-model / display contract.** Display-ready data lives under
   `src/view_models/`, `src/views.rs`, or co-located with the consuming
   composite. The composite's signature accepts only the contract type;
   backend rows (`db::*`, `api::*`, `*_service::*`) are forbidden as inputs
   or callback parameter types.
3. **No screen-local fallback labels.** Coercions like
   `name.unwrap_or("Unknown Artist")`, `if title.is_empty() { "Untitled" }`,
   `feed_url.unwrap_or_default()` belong in the view-model that owns the
   field. Screens read the projected `display_*` accessor; they do not
   re-decide what an empty value means.
4. **No ad hoc sizes / colors / icons.** Spacing, radii, font sizes,
   weights, semantic colors, and SF-symbol-style glyphs come from
   `src/ui/tokens.rs` and the icon module. Raw `px(...)`, `rgb(...)`, inline
   SVG strings, and bare leading-glyph chars are forbidden outside the
   token/theme/icon layer.
5. **Architecture test prevents the regression.** The same change that
   consolidates a thing also adds (or strengthens) a named test in
   `tests/architecture_tests.rs` that fails if the duplication, fallback,
   or raw literal returns. No baseline grows; baselines only shrink.

A thing that satisfies rules 1–4 but not 5 is half-done. A thing whose test
exists with a non-zero baseline is technical debt with a documented
remediation owner.

## Apple HIG anchoring

These rules are not local taste. They make the app obey HIG principles
mechanically rather than by review:

- **Consistency** (`apple-hig/foundations/`): one composite per affordance
  means rows, popovers, headers, and action buttons present identically
  across surfaces. Cross-screen forks of the same control are the most
  common HIG-violation pattern in this codebase.
- **Hierarchy & predictability**
  (`apple-hig/summaries/typography-complete.md`,
  `apple-hig/summaries/layout-complete.md`): a single token vocabulary
  guarantees title/subtitle/metadata/state placement is identical between
  Library and Discovery. Ad hoc `px(12.0)` or `rgb(0x666666)` values silently
  re-introduce visual hierarchy drift.
- **Adaptivity** (`apple-hig/foundations/dark-mode.md`,
  `apple-hig/foundations/materials.md`): a screen that bypasses the theme
  bridge cannot adapt to dark mode, accent changes, or future material
  changes. Theme-aware behavior must live in primitives.
- **Direct manipulation & focus**
  (`apple-hig/components/buttons.md`, `apple-hig/components/popovers.md`):
  button role, popover anchoring, and command intent are properties of the
  composite, not the screen. Screens express *what* command runs; the
  composite owns *how* it presents.
- **Accessibility** (`apple-hig/summaries/accessibility-complete.md`):
  centralizing labels in view-models lets us audit empty-state strings,
  voiceover hints, and translatable copy in one place. Inline
  `unwrap_or("Untitled")` defeats this.

## Non-Goals

- No SwiftUI / AppKit port.
- No new visual redesign or HIG control. Pure consolidation.
- No backend, schema, service, or API changes.
- No ban on density. Dense desktop workflows stay; only their *ownership*
  changes.

## Surface inventory and current owners

Verified by grep on 2026-05-02. Every entry must converge to "one owner"
under this plan.

### Already one-owner (do not regress)

| Surface | Owner | Test gate |
|---|---|---|
| Add-to-playlist popover | `src/ui/composites/playlist_popover.rs` (`AddToPlaylistPopover`) | `screens_do_not_grow_screen_local_playlist_popover_panels`, `library_release_detail_playlist_popovers_use_shared_composite`, `playlist_popover_calls_wire_create_mode` |
| Release detail surface | `src/ui/composites/release_detail_surface.rs` | `shared_top_level_ui_shells_do_not_import_screen_modules` |
| MusicBrainz panel chrome | `src/ui/composites/musicbrainz_panel.rs` | (consolidated per ADR 0033 follow-up) |
| Floating chrome (popovers, overlays, anchors, z-index) | `src/ui/primitives/popover.rs`, `src/ui/primitives/surface.rs` | `presentation_modules_do_not_hand_roll_floating_chrome` |
| Tokens (spacing, radius, font size, weight, color) | `src/ui/tokens.rs`, `src/ui/theme_bridge.rs`, `src/ui/theme_profiles.rs` | `screens_do_not_reintroduce_raw_color_or_numeric_px_literals`, `ui_components_do_not_bypass_theme_profile_resolution` |
| Status / provenance / layout style namespaces | removed | `ui_style_does_not_reintroduce_layout_namespace`, `ui_style_does_not_reintroduce_status_roles`, `ui_style_does_not_reintroduce_provenance_diff_roles` |
| Inline SVG glyph helpers | banned in screens | `screens_do_not_define_inline_icon_svg_helpers` |
| Raw leading button glyphs | banned | `ui_buttons_do_not_reintroduce_raw_leading_glyphs` |
| Direct `Component::Button` use in screens | baseline-zero | `screens_do_not_grow_unmarked_direct_component_button_usage` |
| Value-route recipient labels | hoisted to VM | `screens_do_not_inline_value_route_recipient_label_fallbacks` |
| Top-level UI module classification | covered | `top_level_gpui_modules_are_classified_as_screen_or_shared_ui` |

### Already scheduled (post-ADR 0033 plan, Workstream A)

`render_loading`, `render_action_row`, `render_track_metadata_grid`,
`render_file_header`, `render_track_header`, `render_musicbrainz_*` family.
Sequencing and reconciliation already specified there. This plan does not
re-specify them; it only adds the no-duplication test (Workstream B) under
rule 5.

### Newly identified — require this plan

These are *not* in the post-ADR 0033 plan. Each violates one or more of
rules 1–5 today.

#### Fallback labels embedded in screens (rule 3 violations)

| Site | Current code | Field | Target owner |
|---|---|---|---|
| `src/library.rs:164-165` | `.or_else(\|\| track.feed_title.clone()).unwrap_or_else(\|\| "Untitled".into())` | track display title with feed-title fallback | `TrackVm::display_title` (already proposed in post-ADR 0033 Workstream C.1; this plan binds it as *the* canonical owner — every consumer must call it) |
| `src/library.rs:1604-1605` | `.or_else(\|\| track.artist_name.clone()).unwrap_or_else(\|\| "Unknown Artist".to_string())` | artist display name | new `TrackVm::display_artist` |
| `src/library.rs:1609-1610` | `.unwrap_or_else(\|\| "Unknown Album".to_string())` | album display title | new `TrackVm::display_album` |
| `src/library.rs:2171` | `.unwrap_or("[untitled]")` | playlist row title | new `PlaylistVm::display_name` (or extend `PlaylistOption`) |
| `src/library.rs:2980` | `.unwrap_or("Tags")` | tag panel section title | move to `view_models/track_metadata_grid.rs` |
| `src/ui_track.rs:103` | guid presence branch | track identity affordance | hoist into `TrackVm::identity_state` |
| `src/library.rs:2384` | `feed_url.clone().unwrap_or_default()` | feed url display | `FeedVm::display_url`; empty must be expressed as `Option<String>` for the composite to render an empty-state, not coerced to `""` |

Rule 3 says: a screen never decides what an empty value *means*. The
view-model decides; the screen renders the projection.

#### Ad hoc icon / glyph carriers (rule 4 boundary check)

The icon module is the single owner of glyph identity. Any new
`text("▶")`, `text("…")`, `text("✓")`, `text("✗")`, or inline SVG string
in a screen or composite is a rule-4 violation. Audit on the same pass
that lands the no-duplication test (Workstream B). No new test required if
`ui_buttons_do_not_reintroduce_raw_leading_glyphs` and
`screens_do_not_define_inline_icon_svg_helpers` together already cover the
forbidden patterns; otherwise extend
`SCREEN_LOCAL_FLOATING_CHROME_FORBIDDEN_PATTERNS` analog for glyphs.

#### Display-contract gaps (rule 2)

Any composite under `src/ui/composites/` whose public signature accepts a
plain `String` or `&str` for a label that has policy attached
(fallbacks, truncation, casing) must take a view-model field instead. Audit
once with grep `pub fn .*: .*String` across `src/ui/composites/`; any
non-trivial label argument becomes a VM-owned field.

## Workstreams

### Workstream 0 — Stabilize the current canary surfaces

Before broad cleanup, fix the two surfaces that have already exposed drift:

1. **Discovery recent-feed tiles** must show title and artist/publisher from a
   view-model contract, never screen-local `"..."` placeholders. If tile
   chrome is repeated or policy-bearing, move it to a shared composite.
2. **Add-to-playlist popovers** must keep one shared owner and all call sites
   must wire create mode. Missing `+ New Playlist` is treated as a canary for
   shared-composite drift, not a standalone cosmetic bug.

These are first because they are visible, annoying, and diagnostic. A fix is
acceptable only if it removes a screen-local decision or strengthens a guard.

### Workstream 1 — Consolidate the duplicated render helpers

Already specified in `post-adr-0033-ui-consolidation-plan.md`. Run that plan
unchanged. This document subsumes its rule 5 test (no-duplication arch
test) into the broader rule-5 list below.

### Workstream 2 — Hoist screen-local fallback labels into view-models

For each row in the "Fallback labels" table above:

1. Add the `display_*` accessor to the named view-model. Plain `String`,
   GPUI-free, unit-tested with at least three cases: present, empty
   string, `None`.
2. Replace every screen call site with the accessor. Search for the
   literal fallback string (`"Untitled"`, `"Unknown Artist"`, `"Unknown
   Album"`, `"[untitled]"`, `"Tags"`) and confirm no other call site
   re-decides the same coercion.
3. Delete the inline coercion. The screen now reads VM output directly.
4. Add or extend an architecture test (see Workstream 4) that forbids the
   literal in `SCREEN_FILES`.

Sequence: do `display_title` first (already partially scoped by post-ADR
0033 Task #5), then artist, album, playlist, tag, feed-url — smallest
blast-radius first.

### Workstream 3 — Audit shared-UI display contracts

One pass across `src/ui/composites/*.rs`:

1. Grep `pub fn .*-> .*Self` and `pub fn .*: \(impl Into<\)?String` on each
   composite struct.
2. For each `String` / `&str` parameter, classify: pure passthrough (OK),
   policy-bearing (move to VM field).
3. Where a composite already takes a VM (`PlaylistOption`,
   `MusicBrainzPanelVm`), it is fine. Where it takes loose strings with
   policy, define a co-located display struct (e.g. `ActionRowItem` next
   to `action_row.rs`) and migrate callers.

Output: one short note per composite in its module-level doc comment
naming its display contract type and where the type is defined.

### Workstream 4 — Add and tighten architecture tests (rule 5)

Add the following named tests to `tests/architecture_tests.rs`. Each must
land in the same change that consolidates the corresponding surface;
none may grow a non-zero baseline.

| Test | Fails when |
|---|---|
| `screens_do_not_define_duplicate_render_helpers` | Any `^fn render_[a-z_]+(` name appears in ≥2 files in `SCREEN_FILES` (sketch already in post-ADR 0033 Workstream B). |
| `screens_do_not_inline_unknown_artist_or_album_fallbacks` | Any file in `SCREEN_FILES` contains the string literal `"Unknown Artist"` or `"Unknown Album"` outside a test or VM call site. |
| `screens_do_not_inline_untitled_fallback` | Any file in `SCREEN_FILES` contains `"Untitled"` or `"[untitled]"` outside a test. |
| `screens_do_not_coerce_empty_feed_url_to_empty_string` | `feed_url.*unwrap_or_default` (or equivalent grep) appears in `SCREEN_FILES`. Hoist to `FeedVm::display_url -> Option<String>`. |
| `composite_signatures_take_display_contracts_not_loose_strings` | Per-composite allowlist; any new `pub fn` taking `String` for a policy-bearing field outside the allowlist fails. Land with Workstream 3. |
| `screens_do_not_inline_glyph_strings` | Extend forbidden-pattern list with bare unicode-glyph string literals seen in audit. (Skip if existing glyph tests already cover.) |

Every new test added under this plan is appended to the "Enforcing tests"
list in ADR 0033 in the same change.

### Workstream 5 — Stretch: shrink the screen allowlists

`SCREEN_FILES`, `PRESENTATION_GLUE_FILES`, and
`KNOWN_SHARED_UI_SHELL_FILES` carve specific top-level files out of
directory-scoped tests. Once a top-level shell (`src/ui_artist.rs`,
`src/ui_entity.rs`, `src/ui_feed.rs`, `src/ui_track.rs`) is fully VM-and-
composite driven, relocate it under `src/ui/shells/` so directory-scoped
tests cover it automatically and the allowlist entry can be removed.
Identical to post-ADR 0033 Workstream D; included here for completeness.

## Execution Sequence

Run these as separate commits. Each task is small enough for a lower-context
coding model to implement without rethinking the architecture, and each review
must reject symptom-only fixes.

1. [Task 001: Recents surface ownership](../tasks/one-owner-per-surface-task-001-recents-surface-ownership.md)
   - Current canary: Discovery recents rendering `...` instead of meaningful
     title/artist/publisher labels.
   - Outcome: labels and tile structure owned by VM/composite, with visual
     smoke and a regression guard.
2. [Task 002: Fallback display accessors](../tasks/one-owner-per-surface-task-002-fallback-display-accessors.md)
   - Current debt: screen-local `Untitled`, `Unknown Artist`,
     `Unknown Album`, empty feed URL, and related coercions.
   - Outcome: display policy lives in view-models and architecture tests
     forbid reintroducing the screen literals/coercions.
3. [Task 003: Composite display-contract audit](../tasks/one-owner-per-surface-task-003-composite-display-contract-audit.md)
   - Current debt: public composite signatures can still accept loose
     strings for policy-bearing labels.
   - Outcome: composites document their display contracts and policy-bearing
     labels use a VM or co-located display struct.
4. [Task 004: Feature-readiness gate](../tasks/one-owner-per-surface-task-004-feature-readiness-gate.md)
   - Current risk: new playlist/playback UI can land on surfaces that still
     allow duplication.
   - Outcome: ADR 0033 enforcing-test list is current, all new gates pass,
     and the review checklist says whether richer feature work can proceed.

## Readiness Gate for New Playlist/Playback Features

Richer playlist/playback features may start only after Task 004 records
"Proceed" in `docs/reviews/one-owner-per-surface-review-checklist.md`.
Proceed means:

- no known screen-local playlist popover or recent-tile fallback path remains;
- no open task requires adding a baseline above zero;
- each target feature surface names its owner composite/primitive before code
  is written;
- each target feature surface names its view-model/display contract before
  code is written;
- visual smoke has covered Library, Discovery recents, release detail, track
  detail, playlist popover, and now-playing/action chrome.

Until then, UI work is limited to structural hardening from this plan or
already-scoped ADR 0033 consolidation tasks.

## Files Touched

- `src/view_models/track.rs` — new `display_title`, `display_artist`,
  `display_album` accessors.
- `src/view_models/feed.rs` — new `display_url`.
- `src/view_models/` — possibly new `playlist.rs` if `PlaylistOption` does
  not already cover row display.
- `src/view_models/track_metadata_grid.rs` — owns the `"Tags"` fallback.
- `src/ui/composites/` — composites converged on display contracts; per-
  composite doc-comment names the contract type.
- `src/library.rs`, `src/search.rs`, `src/ui_*.rs` — call sites updated;
  inline coercions deleted.
- `tests/architecture_tests.rs` — new tests per Workstream 4.
- `docs/adr/0033-hig-ui-architecture-governance.md` — extend "Enforcing
  tests" list as each new test lands.

## Verification

Per change:

- `cargo test --test architecture_tests` green; new test(s) included.
- `cargo test` full suite green; new VM unit tests included.
- `cargo clippy -- -D warnings` and `cargo fmt -- --check`.
- Visual smoke on the affected surface (Library list, Library inspector,
  Discovery list, Discovery inspector, release detail, playlist popover).
- Grep proof: the literal string or coercion the change removed has zero
  remaining occurrences in `SCREEN_FILES`.

Per workstream completion:

- Diff `library.rs` and `search.rs` line counts before/after; expect
  monotonic shrinkage as VM accessors land.
- New tests' baselines stay at zero.

## Risk Areas

- **Fallback strings carry policy.** `"Unknown Artist"` is not a synonym
  for `""`. Hoisting must preserve the empty-vs-unknown distinction; some
  VMs will need to expose `Option<String>` rather than `String` so the
  composite can choose between empty-state UI and labeled fallback. Decide
  per accessor; document in the VM doc-comment.
- **Allowlist tests are sharp tools.** A new screen file added without
  updating `SCREEN_FILES` silently escapes every directory-scoped test.
  The ADR 0033 invariant already requires this; reviewers must enforce.
- **Reconciling forked behavior is a UX choice.** Same risk as post-ADR
  0033 plan; canonical chosen behavior is documented in the composite's
  module-level doc.
- **Test sprawl.** Each new arch test is mechanical but cumulative cost
  matters. Prefer one test that walks `SCREEN_FILES` for a forbidden-
  pattern list (already the precedent) over many single-pattern tests.

## Out of Scope

- Backend, schema, API, or service redesign.
- ADR 0031 release-detail contract work (independent track).
- New HIG controls.
- Theme palette redesign.
- Accessibility audit beyond "labels live in view-models so they can be
  audited later".
