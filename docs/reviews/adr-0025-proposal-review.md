# ADR 0025 Proposal Review

## Reviewed Artifact

- ADR: `docs/adr/0025-theme-icon-style-boundary.md` (Proposed - 2026-05-01)
- Phase plan: `docs/plans/adr-0025-visual-system-phase-plan.md`
- Tasks: `docs/tasks/adr-0025-task-001-theme-profile-gates.md` through `…task-006-retire-theme-shim.md`, including `…task-003b-button-style-sweep.md`
- Implementation review checklist: `docs/reviews/adr-0025-review-checklist.md` (kept separate; this document is a *proposal* review, the checklist remains the *post-implementation* review)

## Reviewer

- Reviewer: codex (proposal review)
- Date: 2026-05-01
- Status: Conditional accept

## Verdict

**Conditional accept.** Premises are accurate, direction is consistent with ADRs 0023/0024, phase decomposition is sound. One **P0** finding (control-style scope vs. existing `ui::primitives::Button`) and three **P1** findings should be resolved by amending the ADR text or task pack before Phase 1 implementation begins. The other findings are improvements that should be folded in but do not block start.

## Resolution Note

Updated 2026-05-01: ADR 0025 and its task packets were amended to address the
review findings:

- F1: `ControlStyle` is now the public role layer for
  `ui::primitives::Button`; direct `gpui_component::Button` styling is
  compatibility debt.
- F2: Task 001 now requires `install_theme(profile: ThemeProfile, scale, cx)`
  and updates the two bootstrap call sites.
- F3: Task 006 now owns `theme.rs` retirement with a measurable zero-call-site
  gate.
- F4: Task 001 now owns high-contrast contrast tests.
- F5-F9: entity roles, dead `theme::glyphs`, provenance/diff ownership,
  brand-color contrast, and `ControlStyle` admission criteria are now captured
  in the ADR, task packets, and review checklist.

Updated again 2026-05-01 after round 2:

- N1: Tasks 001-003 now run `cargo test` for new unit-test coverage.
- N2: Task 003 was split into boundary work and Task 003b for the screen button
  style sweep.
- N3: Direct `gpui_component::Button` compatibility debt now uses the
  `CONTROL-COMPAT(reason): ...` marker enforced by architecture tests.
- N4: Non-obvious `ControlStyle` roles now list current admission examples.
- N5: Task 003b requires a final inventory of direct button chains with
  migrated / compatibility-debt / one-off disposition.

## Strengths (no action needed)

- **Premise accuracy.** All eight claims in the ADR's "Context" section were verified against current code:
  - `tokens.rs` owns the eight semantic types: `SemanticColor`, `Spacing`, `Radius`, `FontSize`, `Size`, `Appearance`, `ScaleFactor`, `Environment` (`src/ui/tokens.rs:24-546`).
  - `theme_bridge::install_theme(appearance, scale, cx)` exists and overwrites ~60 fields on `gpui_component::Theme` (`src/ui/theme_bridge.rs:47`).
  - `theme.rs` is a dark-only shim with `color::*` (lines 21-103), `badges` (lines 105-140), and `glyphs` (lines 174-181) namespaces (`src/ui/theme.rs`).
  - `EntityKind` already exists as a typed 8-variant enum (`src/ui/composites/tag_badge.rs:22-32`).
  - Icons are scattered: inline SVG in `src/ui/mod.rs:22-35`, `src/search.rs:4782-4790`, `src/search.rs:4842`; Unicode glyphs in `composites/now_playing_bar.rs`, `composites/playlist_popover.rs`, `composites/section_header.rs`; legacy constants in `theme::glyphs`.
- **Reversible phases.** Each phase leaves the app working; the "compatibility shim coexists with replacement" pattern is explicit.
- **Prerequisites already in place.** Light + dark palettes are fully populated in `tokens.rs` (`dark_palette()` line 128, `light_palette()` line 188); `EntityKind` is typed; `theme_bridge` already centralizes theme installation. Phase 1 builds on real foundations, not greenfield.
- **Architecture-test ratchet exists.** `tests/architecture_tests.rs` already uses a screen-file allowlist + literal-pattern matcher (`SCREEN_FILES` line 65, `screens_do_not_reintroduce_raw_color_or_numeric_px_literals` line 124). Phase 1 gates fit this pattern with no new infrastructure.
- **Internal consistency.** ADR ↔ phase plan ↔ task files ↔ review checklist agree on module names (`theme_profile.rs`, `icons.rs`, `control_styles.rs`), invariants, and "Do Not Touch" lists. Each task has a tightly-scoped acceptance section and an escalation-trigger list.
- **Strong scope discipline.** "Non-goals" and per-task "Do Not Touch" sections prevent creep into application/services/db.

## P0 — Required before Phase 3

### F1 — `ControlStyle` does not address the existing `ui::primitives::Button`

The ADR says control styles "can expose these as helper functions, builders, or extensions around `gpui_component::Button`" (`docs/adr/0025-theme-icon-style-boundary.md:159-161`). But `src/ui/primitives/button.rs:1-90` already defines a **native, HIG-aligned button primitive** — `ButtonVariant::{Filled, Tinted, Plain, Destructive}`, `ButtonSize::{Sm, Md, Lg}` — explicitly built so "the token system fully owns the visual contract" (lines 4-6 of the file's docstring). Native means it is not a wrapper around `gpui_component::Button`.

The primitive is **not used by any screen**: `rg 'use crate::ui::primitives::button|primitives::button'` returns zero hits. Meanwhile `src/library.rs`, `src/search.rs`, `src/app.rs`, and `src/ui/composites/action_button.rs` all use `gpui_component::Button` directly.

The ADR proposes a third button vocabulary (`ControlStyle::{Primary, Secondary, Ghost, Destructive, ToolbarIcon, RowAction, MetadataAction, Pill}`) without addressing this. Implementing Phase 3 as written would leave the codebase with three concepts:

1. `ui::primitives::Button` — token-native, dormant, four variants
2. `gpui_component::Button` + ad-hoc style chains — what screens actually use, ~32 sites with ~200 chained methods
3. `ControlStyle` — new vocabulary built around #2

**Required:** the ADR must commit to one of:

- **(a)** `ControlStyle` *is* `ui::primitives::Button`'s public face. Variants map to control roles; screens migrate to it; the third-party `gpui_component::Button` is wrapped behind it and screens stop calling it directly. The dormant primitive becomes the consumed primitive.
- **(b)** `ui::primitives::Button` is deleted; `ControlStyle` wraps `gpui_component::Button`; the ADR explains why the in-house primitive was abandoned.
- **(c)** Both coexist with a documented split (e.g., primitive for purely-internal composites, control style for screen-facing reusable patterns). This is the weakest option because it preserves the divergence the ADR is trying to remove.

Until this is decided, Task 003 is under-specified and an implementer could plausibly produce any of the three outcomes.

## P1 — Required before Phase 1

### F2 — `install_theme` signature decision is left to the implementer

Task 001 says "Wire `theme_bridge::install_theme` through the new type if this can be done without behavior changes; otherwise add a documented adapter that preserves the current `Appearance` entry point" (`docs/tasks/adr-0025-task-001-theme-profile-gates.md`, step 3). This is a real fork that affects every downstream task — do icon/control modules read `Environment` (current global) or a new `ThemeProfile` global?

The current signature is `install_theme(appearance: Appearance, scale: ScaleFactor, cx: &mut App)` (`src/ui/theme_bridge.rs:47`); bootstrap calls it twice (`src/app/bootstrap.rs:37` and `:51`).

**Required:** the ADR (or Task 001) should commit to one path. Recommended: change the signature to `install_theme(profile: ThemeProfile, scale: ScaleFactor, cx: &mut App)`, give `ThemeProfile` an inherent method `appearance(&self) -> Appearance`, and update both bootstrap call sites. Two call sites is not "many screen render paths"; the escalation trigger doesn't fire.

### F3 — Phase 6 has no task file and no measurable retirement criterion

The phase plan describes Phase 6 ("retire compatibility shims," `docs/plans/adr-0025-visual-system-phase-plan.md:93-96`) but there is no `adr-0025-task-006-*.md`, and the ADR's Green Criteria is binary ("named theme-profile boundary exists," "no longer implemented as screen-level inline SVG," etc.). Without an objective retirement threshold, Phase 6 either ships partial or never ships.

**Required:** either add Task 006, or amend the ADR's Green Criteria with a measurable retirement gate. Suggested wording: "0 screen-file call sites of `theme::color::*`, `theme::badges`, or `theme::glyphs` for the duration of one full release cycle, after which `theme.rs` is reduced to layout constants only and the architecture gate forbidding the deprecated namespaces is unconditional (no allowlist)."

### F4 — High-contrast profile tests are constrained but unowned

Task 005 constraint: "Do not expose high contrast unless high-contrast profile tests exist." Task 001 adds `ThemeProfile::HighContrastDark` and `HighContrastLight` but does not create their contrast tests. No task in 001-005 does. The constraint is therefore unenforceable as planned.

**Required:** move high-contrast contrast-matrix test creation into Task 001 (the natural pairing — same place the profile type is added) or add an explicit Task 005a. The existing WCAG matrix tests in `src/ui/contrast.rs` are the obvious extension point.

## P2 — Worth fixing in the ADR

### F5 — Entity-role list is out of sync with `EntityKind`

ADR lines 178-179 list 6 entity roles: feed, track, artist, publisher, release, recording. The audited `EntityKind` enum (`src/ui/composites/tag_badge.rs:22-32`) has **8 variants** — adds `Playlist` and `Generic`. Either the ADR text is missing two or those two should be flagged for removal. Realign before Task 004 begins.

### F6 — Provenance/diff visualization is split across helpers but not assigned an owner

The ADR lists provenance roles `match`, `different`, `missing` under "Badge and entity-role boundary" (line 180-181). Today these render as *both* `theme::color::diff_*` (color) *and* `theme::glyphs::DIFF_*` (glyph: ✓ ≠ ∅). Diff display is therefore icon + color + label, not just badge.

Task 004 ("Typed Badge Role Migration") covers entity/status; provenance is named in the ADR but not explicitly placed in any task's scope. Specify whether diff roles are part of the badge composite, the icon catalog, or a separate diff-display primitive — and place them in the matching task (likely Task 002 + Task 004 together, since the diff render combines an icon and a role color).

### F7 — `theme::glyphs` has zero call sites

The audit found `theme::glyphs` is referenced 0 times outside its own definition. The ADR repeatedly cites it as a leak source, and Phase 2 implies migration. Recommend a one-liner in Task 001: delete `theme::glyphs` outright. It's already dead.

The Unicode glyph leakage that *does* exist sits in composites (`now_playing_bar`, `playlist_popover`, `section_header`) and `src/ui/primitives/button.rs:61`'s `leading_glyph` field — those are the actual Phase 2 migration targets.

### F8 — Brand-color contrast check missing from Task 002 acceptance

ADR lines 132-134 require brand/protocol icon colors (RSS orange, Nostr purple) to "still pass a non-text contrast check for their usage." Task 002's acceptance criteria do not include this check. Add it as an explicit acceptance criterion, with the existing contrast matrix in `src/ui/contrast.rs` as the verification path.

### F9 — `ControlStyle` admission criterion is unbounded

The ADR acknowledges "Review discipline is needed so `ControlStyle` does not become a dumping ground for one-off styles" (line 293-294). The review checklist mirrors the concern. Neither defines a concrete admission test.

Recommend adding to the ADR's "Control-style boundary" section: "A new `ControlStyle` role requires ≥2 unrelated screens currently using the matching style chain, **or** a state/contrast requirement that a generic chain cannot express." This makes the gate testable instead of subjective.

## P3 — Notes (not blockers)

- **F10** — Task 001 "Do Not Touch" mentions `src/app.rs` "except if module wiring requires a minimal import adjustment." Bootstrap wiring is in `src/app/bootstrap.rs`, not `src/app.rs`. Minor wording fix.
- **F11** — Task 005 "expose only tested profiles." Clarify whether `ThemeProfile::System` is exposed in Settings while it transparently resolves to Dark. Recommend no — exposing a no-op control is confusing.
- **F12** — ADR test-strategy clause "focused tests for control style role mapping if the implementation is pure enough to test without GPUI" hedges. Stronger: the role→token mapping should be implemented as a pure function so it *is* testable; that is a design constraint, not a hope.
- **F13** — `IconName` enum in the Decision text lists 17 variants speculatively. Consistent with the "grow only when needed" guidance elsewhere in the ADR — but the speculative enumeration mildly contradicts that guidance. Consider trimming the listed enum to the migration-set only and letting the catalog grow in code.

## Sizing reality-check

The audit confirms the leakage scale:

- ~32 direct `Button::new(` sites in screens (`library.rs`, `search.rs`, `app.rs`).
- ~200 chained style-method calls (`.ghost()`, `.text_color(`, `.border_color(`, `.bg(`, `.rounded(`).
- ~95 `theme::color::*` call sites, concentrated in `src/search.rs` and `src/library.rs`.
- 6 `theme::badges` call sites (`src/search.rs:3087, 3094`; `src/library.rs:3209-3210, 3354, 3361`).
- 0 `theme::glyphs` call sites (see F7).
- 3 inline-SVG icons (`src/search.rs:4782-4790, 4842`; `src/ui/mod.rs:22-35`).

Phases are sized realistically *if* Task 003 expands to absorb the ~200 chained methods. As written, Task 003 only migrates `ActionButton` plus "if low-risk" one row-action pattern. The bulk migration is implicit and unowned.

Tying this back to F1: once the button strategy is decided, Task 003 (or a Task 003b) should explicitly own the screen-level chain sweep with a concrete file list. `src/library.rs` and `src/search.rs` are the volume.

## Recommended ADR amendments before approval

1. Resolve **F1** in the ADR's "Control-style boundary" section by naming the relationship to `ui::primitives::Button`.
2. Resolve **F2** in Task 001 by committing to a single `install_theme` signature path.
3. Resolve **F3** by adding Task 006 or amending Green Criteria with a measurable retirement gate.
4. Resolve **F4** by moving high-contrast tests into Task 001's scope.
5. Apply **F5** and **F7** corrections to align ADR text with current code state.
6. Apply **F6**, **F8**, **F9** to close acceptance-criteria and admission-criteria gaps.

After these amendments the ADR is ready to begin Phase 1 implementation.

## Audit reproduction

The findings above are reproducible from the working tree at the time of review:

- `rg "theme::color" src/library.rs src/search.rs src/app.rs src/app/ src/ui_track.rs`
- `rg "theme::badges" src/`
- `rg "theme::glyphs" src/`
- `rg "Button::new\(" src/library.rs src/search.rs src/app.rs`
- `rg "use crate::ui::primitives::button|primitives::button" src/`
- `cargo test --test architecture_tests`

## Out of scope for this review

- Editing the ADR itself, the phase plan, the task files, or the implementation checklist (`docs/reviews/adr-0025-review-checklist.md`). Those are the author's call after reading this review.
- Any code changes under `src/`. The ADR is in Proposed status; nothing should ship until the findings above are accepted, deferred, or rejected.

---

## Round 2 Review (post-amendment, 2026-05-01)

The ADR, phase plan, all five original task packets, the new Task 006, and the implementation checklist were re-audited against the resolution note above.

### Verdict (round 2)

**Accept.** All thirteen findings (F1-F13) from round 1 are resolved. Five new minor observations below — all P2/P3, none block Phase 1 start.

### Round-1 findings — resolution verification

| Finding | Status | Verified at |
| --- | --- | --- |
| F1 — `ControlStyle` ↔ `ui::primitives::Button` | **Resolved** (option a — control roles are the public face of the native primitive) | ADR lines 161-166, 252-253; "Add `ControlStyle` beside the native button primitive" added to Alternatives Considered (lines 307-313); Task 003 constraint line 49-50, step 1 line 59. |
| F2 — `install_theme` signature | **Resolved** | ADR lines 106-118 commit to `install_theme(profile, scale, cx)`; Task 001 constraint line 50-51, steps 3-4, acceptance lines 76-77; Green Criteria line 426-427. |
| F3 — Phase 6 task + retirement gate | **Resolved** | Task 006 file added with measurable zero-call-site gate (constraints lines 49-51, acceptance lines 72-79); ADR Green Criteria lines 445-449; phase plan lines 110-113. |
| F4 — High-contrast contrast tests | **Resolved** | Task 001 constraint line 52, step 5, acceptance line 78; `src/ui/contrast.rs` added to Files Likely To Change. |
| F5 — Entity role list (8 variants) | **Resolved** | ADR line 214 lists feed/track/artist/publisher/release/recording/playlist/generic; Task 004 constraint line 44-45, acceptance line 67. |
| F6 — Provenance/diff ownership | **Resolved** | ADR lines 222-226 assign provenance/diff to badge migration; Task 004 constraint line 47-48, step 4, acceptance line 68-69. |
| F7 — Delete dead `theme::glyphs` | **Resolved** | ADR Context line 25-26; Task 001 constraint line 53, step 6, acceptance line 79; Green Criteria line 430. |
| F8 — Brand-color contrast in Task 002 | **Resolved** | Task 002 constraint line 46-47, step 5, acceptance line 67. |
| F9 — `ControlStyle` admission criterion | **Resolved** | ADR lines 196-199 add the ≥2-call-sites-or-state-rule admission test; Task 003 constraint lines 52-54; checklist lines 49-51. |
| F10 — Task 001 `src/app.rs` wording | **Resolved** | `src/app.rs` is now in Do Not Touch (line 39); bootstrap is in Files Likely To Change (line 30). |
| F11 — `ThemeProfile::System` no-op exposure | **Resolved** | Task 005 constraint line 41-42, acceptance line 62-63; checklist line 57-58. |
| F12 — Pure role-mapping function | **Resolved** | ADR test strategy lines 414-416; Task 003 step 2 line 60-61, acceptance line 78. |
| F13 — Trim speculative `IconName` | **Resolved** | List trimmed from 17 to 13 variants (lines 126-141), removing `AddToPlaylist`, `MusicBrainz`, `Search`, `Settings`. |

### Round-2 observations (new)

#### N1 — Task 001/002/003 test command lists omit `cargo test`

Tasks 001 (high-contrast contrast tests in `src/ui/contrast.rs`), 002 (brand-color contrast tests), and 003 (pure role-mapping tests) all *create new unit tests* but their Test Commands sections list only `cargo fmt --check`, `cargo check`, `cargo test --test architecture_tests`, and `cargo clippy …`. The new unit tests will not run automatically because integration-test invocation does not exercise `--lib`.

**Severity:** Low.
**Recommended fix:** add `cargo test` (or `cargo test --lib`) to the Test Commands section of Tasks 001, 002, and 003. Tasks 005 and 006 already list `cargo test` — match them for consistency.

#### N2 — Task 003 absorbs both boundary creation and the screen sweep

Task 003 now owns: `ControlStyle` boundary + role-mapping tests + `ActionButton` migration + the full screen-level chain sweep across `app.rs`, `library.rs`, `search.rs` (~200 chained method calls per the audit). The escalation trigger ("the style-chain sweep becomes too large to verify in one diff … split Task 003 into explicit file-scoped subtasks before editing," line 106-107) provides a relief valve, but pre-splitting would reduce the chance of a sprawling diff being pushed through.

**Severity:** Medium (workflow risk, not correctness risk).
**Recommended fix:** pre-split into Task 003a (boundary + ActionButton + tests, small) and Task 003b (screen sweep, file-scoped — one each for `app.rs`, `library.rs`, `search.rs` if the per-file diff stays large). The per-file split is natural because each screen file has a distinct vocabulary of buttons (search has download/import buttons; library has playlist/transport buttons; app has tab/settings buttons).

#### N3 — `gpui_component::Button` "compatibility debt" lacks an enforcement mechanism

The ADR (line 168-171) and Task 003 (acceptance line 79-81) repeatedly say remaining direct `gpui_component::Button` styling is "documented as compatibility debt" or "explicitly documented as compatibility exceptions." Neither names a concrete mechanism. Without one, the ratchet never tightens — anything can be marked "debt" and stay forever.

Two viable mechanisms:
- **(a)** A line-comment marker (`// CONTROL-COMPAT(reason): …`) plus an architecture-test that *counts* and *lists* every `gpui_component::Button` reference in `SCREEN_FILES`. Test fails if a new reference appears without the comment, and the count is logged so reviewers see the trend.
- **(b)** An explicit allowlist file (e.g., `tests/architecture_compat_buttons.txt`) with `path:line:reason` entries, each requiring a PR to add. New references without an allowlist entry fail the test.

**Severity:** Medium. Phase 6 acceptance line 67-68 only enforces unconditional rejection of `theme::*` namespaces — it does not address `gpui_component::Button` debt.
**Recommended fix:** Task 003 (or Task 006) defines one of (a)/(b) explicitly, with the architecture-test stub included in the same diff.

#### N4 — `ControlStyle::ToolbarIcon` and `ControlStyle::Pill` lack named example call sites

The ADR enumerates 8 ControlStyle roles. The admission rule is now binding (≥2 unrelated call sites or state/contrast rule). For each role, the ADR or Task 003 should name *at least one current call site* that justifies the role at admission time. Otherwise an implementer can add all 8 speculatively and admit each by retroactively migrating one call site, defeating the rule.

**Severity:** Low.
**Recommended fix:** in the ADR's Control-style boundary section or Task 003 step 1, list one current file:line example per role for the four non-obvious roles (`ToolbarIcon`, `RowAction`, `Pill`, `Ghost`). `Primary`/`Secondary`/`Destructive`/`MetadataAction` are obvious from `ActionButton` and standard CTA patterns.

#### N5 — Task 003 inventory step produces no checked-in artifact

Task 003 step 4: "Inventory direct styled `gpui_component::Button` chains in `app.rs`, `library.rs`, and `search.rs`." Good. But the inventory output is implicit — no acceptance criterion or expected-summary item asks for the inventory itself.

**Severity:** Low (process improvement).
**Recommended fix:** add an Expected Final Summary item: "5. inventory of direct `gpui_component::Button` chains found, with per-call disposition (migrated / compatibility-debt / one-off)." This gives reviewers a verifiable list without re-running ripgrep.

### Cross-task dependency note

Implicit dependencies are now visible only via escalation triggers:
- Task 004 escalation (line 93-94) flags Task 002 as a soft dependency for provenance icons.
- Task 006 constraint (line 48) explicitly depends on Phases 1-5.
- Task 005 implicitly needs render-path migration done before exposure.

These are adequate for an attentive implementer. A one-paragraph dependency summary in the phase plan would be a nicety but is not required.

### What's ready to ship

Phase 1 (Task 001) is unblocked. Recommended order of execution:
1. Apply N1 (add `cargo test` to Tasks 001-003) and N3 (compat-debt mechanism) before Phase 1 starts — both are doc-only edits.
2. Optionally apply N2 (pre-split Task 003) and N4 (example call sites) before Phase 3 starts.
3. N5 (inventory artifact) is a Task 003 expected-summary tweak, applicable any time before Task 003 begins.

After N1/N3 land, the ADR is fully unblocked.
