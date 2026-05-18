# ADR 0052 / 0024-follow-up / 0054 Implementation Review

**Date:** 2026-05-18
**Reviewer:** Claude
**Scope:** Commits since `dfd2277` (prior review baseline) on `master`,
covering ADR 0052 (Library / Index data parity triage), the ADR 0024
loading-shape follow-up plan, and ADR 0054 (local metadata source-fact
persistence).

This review picks up where
`docs/reviews/adr-0047-0048-0049-implementation-review.md` left off and
audits whether the runtime work that converted deferred-work-index item #2
from triage to delivery respected existing architecture, governance, and
HIG rules.

## Commits reviewed

```
Phase 1 — Triage foundation (doc-only)
  7d4fd3a Add triage tasks for album, track, artist, and playlist detail parity

Phase 2 — ADR 0024 loading-shape follow-up (runtime + guards)
  6e61d4f feat: implement feed language loading and integration across models and views
  f9bff8d feat: implement rich Index track detail rendering and integration across components
  d7d0220 feat: add pub_date and explicit fields to TrackRow and related projections
  e8c1aaa feat: rename IndexArtistDetail to IndexArtistFeedScope and update related navigation logic
  8f701d2 feat: implement local playlist detail metadata rendering in PlaylistDetailVm
  de934bb feat: add final ADR 0024 loading-shape readiness architecture guard

Phase 3 — ADR 0054 local metadata source-fact persistence
  60f87fb feat: implement ADR 0054 for local metadata source-fact persistence
  668483e feat: add MusicIndex feed metadata ingestion and persistence logic
  0008155 feat: implement MusicIndex track metadata persistence and retrieval logic
  54e51b9 feat: implement local metadata source-fact hydration and integrate into feed read models
  46a7d8e feat: implement track metadata hydration and local fallback in feed service
  0bb37af feat: finalize ADR 0054 implementation and update review documentation
```

Thirteen commits, twelve hours wall-clock. The arc converted
deferred-architecture-work-index item #2 (Library / Index data parity) from
"needs triage" all the way through routing and runtime delivery:
loading-shape gaps were routed through the ADR 0024 follow-up plan and
executed as six bounded slices; persistence gaps were routed to ADR 0053
and delivered as ADR 0054.

## Method

- Per-commit diff scope and file-change inventory across all thirteen
  commits.
- Governance ADRs read or skimmed: 0023, 0024, 0025, 0028, 0033, 0034,
  0038, 0042, 0046, 0047, 0048, 0049, 0050, 0051, 0052, 0053, 0054.
- `tests/architecture_tests.rs` guard inventory (145 tests, including ten
  new guards added across this arc).
- UI inventory: every file under `src/ui/composites/` (26 composites) and
  `src/ui/shells/` (top-level + library/ + discover/ submodules), with
  public-render-fn enumeration and call-site counts.
- Token-discipline sweep across `src/ui/composites/` and `src/ui/shells/`
  for raw `px(N)`, `rgb()`, `rgba()`, `hsla()`, raw font sizes, and
  `Color::` literals bypassing `SemanticColor`.
- HIG check: SF Symbol usage, inline SVG/chevron literals, breadcrumb
  pattern conformance, modal/keyboard-nav patterns, "open-in-new-window"
  affordances.
- Verification reads of `src/ui/composites/breadcrumb_trail.rs`,
  `src/ui/composites/frame_shell.rs`, the three triage report files, ADR
  0052/0053/0054 status sections, and the deferred-architecture-work-index.

## Architecture and visual ownership recap

The repo enforces an eight-layer ownership model anchored in
ADRs 0023/0033/0038/0042:

1. **Tokens** — `src/ui/tokens.rs` (`Spacing`, `FontSize`, `SemanticColor`)
   and `src/ui/icons.rs` (`IconName`). Only legitimate source of
   dimensional, color, typography, and icon values.
2. **Primitives** — `src/ui/primitives/*` (Button, Label, Divider, Stack,
   Image, Popover, Surface, etc.). Token-driven, single-concept, no domain
   types.
3. **Composites** — `src/ui/composites/*`. ≥ 2 primitives, ≥ 2 call sites
   unless ADR-exempted.
4. **Shells** — `src/ui/shells/**`. Page/pane layout; single call sites
   allowed; may consume view-models.
5. **View-models** — `src/view_models/**`. GPUI-free snapshots,
   projections, local UI state, presentation contracts. After ADR 0050,
   the workspace and search_results VMs are decomposed into submodules.
6. **Screens** — top-level GPUI entities, thin wiring under 300 LOC after
   shell extraction.
7. **Application layer** — `src/application/**`, typed
   `ApplicationCommand` / `CommandOutcome` / `ApplicationEvent` surface.
8. **Domain / Infrastructure** — services, database, HTTP clients,
   persistence, mutations.

Token enums in scope for this review:

- `SemanticColor` — `src/ui/tokens.rs:73`
- `Spacing` — `src/ui/tokens.rs:304`
- `FontSize` — `src/ui/tokens.rs:392`
- `IconName` — `src/ui/icons.rs:24`

Governance state at review time: ADRs 0042/0046/0047/0048/0049/0050/0052
are Accepted (some Implemented), ADRs 0025/0034/0038/0053 are Proposed,
ADR 0054 has Implemented status, and the workspace VM has the
submodule decomposition from ADR 0050 already in place.

## Findings — strengths

1. **Triage → routing → runtime executed cleanly end-to-end.** Deferred-work
   item #2 began as "needs triage after ADR 0028 contributor visibility";
   Phase 1 (`7d4fd3a`) shipped three parity triage reports under
   `docs/reviews/library-discover-parity-triage-{album,track,artist-playlist}.md`
   plus the consolidated synthesis at
   `docs/reviews/library-discover-parity-triage-synthesis.md`; Phase 2
   executed the six loading-shape slices routed by
   `docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md`;
   Phase 3 delivered the persistence gaps routed to ADR 0053 via the
   five-task ADR 0054 implementation. The same triage → routing → bounded
   slice pattern is reusable for the remaining deferred items.

2. **ADR 0050 module decomposition held under pressure.** `e8c1aaa` added
   the `IndexArtistDetail` → `IndexArtistFeedScope` rename plus
   breadcrumb/nav routing changes by touching
   `src/view_models/workspace/breadcrumb.rs` and
   `src/view_models/workspace/nav.rs` submodules rather than re-growing
   `workspace.rs`. `f9bff8d` added rich Index track detail through
   `src/view_models/search_results/index_detail.rs` rather than growing
   `search_results/results.rs`. The P1 file-size drift flagged in the
   prior review did not regress.

3. **Architecture-guard discipline per slice.** Every Phase 2 and Phase 3
   slice shipped its own behavior-pinning guard in
   `tests/architecture_tests.rs`:
   - `de934bb` — ADR 0024 loading-shape readiness guard.
   - `8f701d2` — playlist detail metadata path guard.
   - `d7d0220` — `local_track_pubdate_and_explicit_projection_path_is_guarded`.
   - `6e61d4f` — `local_feed_language_parity_is_loaded_through_read_model_path`.
   - `f9bff8d` — Index track detail VM contract guard.
   - `0bb37af` — ADR 0054 finalization guard.
   Each guard fails when its named invariant is broken; none are structural
   noise. This is the right cadence for ratcheting deferred work.

4. **Token discipline holds for new UI touches.** The recent edits to
   `src/ui/composites/frame_shell.rs`,
   `src/ui/composites/breadcrumb_trail.rs`,
   `src/ui/composites/filter_chip_strip.rs`,
   `src/ui/shells/search_results_inspector.rs`,
   `src/ui/shells/library/feed_detail.rs`, and the library detail surfaces
   introduced zero raw `rgb()`, raw font-size, or `Color::` bypasses. All
   color/spacing/typography values continue to flow through token enums.

5. **Breadcrumb composite remains HIG-compliant.**
   `src/ui/composites/breadcrumb_trail.rs:140-144` uses
   `IconName::ChevronRight` (SF Symbol) as separator,
   `ControlStyle::Ghost` for inactive segments,
   `SemanticColor::SecondaryLabel` for separator color, and single-click
   pop semantics. Full HIG path-bar conformance preserved.

6. **ADR 0054 follows ADR 0028's source-fact contract.** New MusicIndex
   feed and track metadata tables (added in `60f87fb`, populated in
   `668483e` and `0008155`, hydrated in `54e51b9` and `46a7d8e`) keep
   MusicIndex provenance distinct from RSS/feed provenance — no
   collapse of source-fact identity, no movement of MusicIndex API
   structs into shared projections, no name-derived inference. The
   ADR 0028 invariants from the prior parity review (contributor identity
   visibility) extend cleanly to the new metadata families.

7. **Commit sizing within budget for runtime work.** Largest runtime
   commit is `f9bff8d` at 619+/69− LOC across eight files (Index track
   detail). Every other runtime commit is ≤ 700 LOC. The only commits
   above 1000 LOC are `60f87fb` (ADR 0054 spec + initial schema/DB
   helpers, 1067+) and `7d4fd3a` (triage foundation, 2108+); both are
   doc-heavy ADR/triage commits — structural by design, not bundled
   runtime drift like `bee1ac2` was in the prior arc.

8. **Doc governance closeout complete.** ADRs 0052/0053/0054 written;
   triage synthesis and follow-up plans landed; review checklist at
   `docs/reviews/library-discover-parity-triage-review-checklist.md`
   ratifies the workflow; ADR 0052 cross-references all artifacts; the
   parent triage plan (`docs/plans/library-discover-parity-triage-plan.md`)
   carries a "Completed - 2026-05-17" status with pointers to every
   downstream artifact.

## Findings — drift and concerns

### P2: pre-existing skeleton `px()` literals slip past the token-discipline guard

Files:

- `src/ui/composites/skeleton_inspector.rs:72,75,76`
- `src/ui/composites/skeleton_feed_tile.rs:66`
- `src/ui/composites/skeleton_track_row.rs:55,69,91`

Seven raw `px(N)` literals for placeholder block dimensions. These predate
this arc — they were not introduced by Phase 1/2/3 — but the composite
inventory pass uncovered them. The active guard
`screens_do_not_reintroduce_raw_color_or_numeric_px_literals` walks
`src/ui/shells/` and screen-mounted modules, not `src/ui/composites/`, so
these slip through.

ADR 0034 explicitly tightens the rule to apply to "shared UI primitives
and composites" but the enforcement is currently shell-only. Either define
a placeholder-block size token (e.g., extend `Size` or add a
`SkeletonBlock` size enum), refactor the seven hits, then extend the guard
to cover composites.

### P2: three single-call-site composites outside the ADR 0048 exemption

ADR 0042 requires composites to have ≥ 2 distinct call sites unless an
ADR documents an exemption. Current single-site composites:

- `src/ui/composites/breadcrumb_trail.rs` — visible call site is
  `src/ui/shells/library/track_detail.rs:136`. `frame_shell.rs` renders
  breadcrumbs as part of frame chrome but does so via inline call path
  rather than going through `BreadcrumbTrail`. Either route frame_shell's
  breadcrumb rendering through this composite (preferred — that is what
  ADR 0048's frame_shell exemption presupposes) or document a fresh
  exemption.
- `src/ui/composites/release_detail_surface.rs` — one call site at
  `src/ui/shells/entity.rs:74`. Was once shared with the parked Discover
  surface; current state is single-site.
- `src/ui/composites/musicbrainz_panel.rs` — one call site.
- `src/ui/composites/skeleton_feed_tile.rs` — one call site.

ADR 0048 documented a single exemption (`frame_shell`). These four merit
the same conscious choice: inline if truly single-use, find/add the
second caller, or extend the exemption list. Today they sit in an
unaddressed middle ground.

### P2: ADR 0053 status reconciliation needed

ADR 0053 ("Local detail source-fact parity") sits at status "Proposed"
while ADR 0054 has shipped the persistence work for what reads as the
same scope (release date, language, explicit state, description) at the
source-fact layer. The relationship between the two ADRs is not stated
explicitly in either document. A reader landing on ADR 0053 cannot tell
whether it is:

- subsumed by ADR 0054 (then mark Superseded with a pointer);
- the parent contract that ADR 0054 implements (then mark Accepted and
  cross-reference 0054 as the implementation);
- or carrying remaining scope ADR 0054 deliberately did not cover (then
  list that residual scope in the Status block).

The triage synthesis (`docs/reviews/library-discover-parity-triage-synthesis.md`)
implies the second reading, but ADR 0053 itself does not say so.

### P3: `src/ui/shells/playlist.rs` is 831 LOC

Soft budget for shells is 800 LOC. Predates this arc; no new growth from
Phase 1/2/3. Eligible for an ADR 0050-style decomposition when the next
playlist-touching packet starts. Low priority — not blocking anything.

### P3: deferred-work-index entry needs status refresh

`docs/plans/deferred-architecture-work-index.md` was last updated
2026-05-08. The plan file `library-discover-parity-triage-plan.md` was
edited 2026-05-17 to mark the triage as "Completed", but the index
itself may still list item #2 under "Priority order" rather than "Recently
Resolved". The runtime fixes (Phase 2 + Phase 3) shipped, so item #2 is
not just routed — it is delivered. Either move the entry to "Recently
Resolved" with the right closure line, or document the residual scope
clearly if any remains.

## Recommended remediations

| # | Action | Touch points | Shape |
|---|--------|--------------|-------|
| R1 | ADR 0053 status reconciliation | `docs/adr/0053-local-detail-source-fact-parity.md` Status block + cross-ref to ADR 0054 | trivial doc edit |
| R2 | Skeleton block-dimension token + refactor seven px() hits | `src/ui/tokens.rs` (new size variants or `SkeletonBlock` enum), `src/ui/composites/skeleton_inspector.rs`, `skeleton_feed_tile.rs`, `skeleton_track_row.rs` | small task |
| R3 | Composite call-site audit + ADR 0042 reconciliation | short verification reads of `frame_shell.rs`, `entity.rs`, `release_detail_surface.rs`, `musicbrainz_panel.rs`, `skeleton_feed_tile.rs`; either inline single-use composites, route through them, or document an exemption list extension in an ADR addendum | small task |
| R4 | Token-discipline guard extension to composites | `tests/architecture_tests.rs` — extend `screens_do_not_reintroduce_raw_color_or_numeric_px_literals` (or sibling) to walk `src/ui/composites/` once R2 lands | trivial task |
| R5 | Deferred-work-index item #2 status update | `docs/plans/deferred-architecture-work-index.md` | trivial doc edit |

No new ADR is required by this arc. R3 may upgrade ADR 0042's call-site
rule with an explicit exemption list rather than re-litigating per
composite; that decision belongs to whoever picks up R3.

Suggested execution order: R1 and R5 first (cheap doc edits unblock
reader confusion and close the deferred-work entry), then R2 (skeleton
tokens), then R4 (guard extension once R2 has reduced the failure
surface to zero), then R3 (broader composite audit, which may produce its
own follow-up packet).

## Verification path for any follow-up

Standard five-gate cargo block:

```bash
cargo fmt -- --check
cargo build 2>&1 | tail -5
cargo test --lib 2>&1 | tail -5
cargo test --test architecture_tests 2>&1 | tail -5
cargo clippy -- -D warnings 2>&1 | tail -5
```

Plus an operator-visible UI skim:

- Library album / release detail surface shows the four named parity
  fields (release date, language, explicit state, description) where the
  triage marked them populated.
- Library track detail metadata grid surfaces the new `pub_date` and
  `explicit` cells.
- Index track detail renders the rich layout from `f9bff8d` with no raw
  numeric/color leaks.
- Breadcrumb chrome remains visually unchanged after any
  `frame_shell.rs` / `breadcrumb_trail.rs` reroute under R3.

## Out of scope

- Async-runtime audit (deferred-work item #3, ADR 0040 follow-up).
- Remote-only Discover read thinning (deferred-work item #4).
- Person/global identity persistence (deferred-work item #1, ADR 0029
  territory).
- Accessibility audit beyond existing
  `interactive_composites_carry_accessibility_labels` guard.
- Playback-driver supervision (deferred-work item #6).
- `RemoteDetailThumbnailState` lifecycle (carried over from prior review).

## Bottom line

The arc converted a deferred-work item from triage through runtime under
existing governance with no P1 drift. Workspace and search_results VMs
held their post-ADR-0050 shape; every runtime slice landed with its own
behavior-pinning guard; token discipline and HIG conformance for new UI
edits is intact. Remaining concerns are small mop-ups — skeleton block
tokens, single-call-site composite audit, ADR 0053 status note,
deferred-work-index update — none of which block picking up the next
deferred item.

## References

- ADR 0024 — application command/query/event layer
- ADR 0028 — local identity source-fact persistence
- ADR 0042 — primitive/composite/shell layer consolidation
- ADR 0050 — post-ADR-0048 module decomposition
- ADR 0052 — Library / Index data parity triage
- ADR 0053 — local detail source-fact parity
- ADR 0054 — local metadata source-fact persistence
- `docs/plans/library-discover-parity-triage-plan.md`
- `docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md`
- `docs/reviews/library-discover-parity-triage-synthesis.md`
- `docs/reviews/library-discover-parity-triage-review-checklist.md`
- `docs/reviews/adr-0047-0048-0049-implementation-review.md` (prior baseline)
- `docs/plans/deferred-architecture-work-index.md`
- `src/ui/composites/breadcrumb_trail.rs`
- `src/ui/composites/frame_shell.rs`
- `src/ui/composites/skeleton_inspector.rs`
- `src/ui/composites/skeleton_feed_tile.rs`
- `src/ui/composites/skeleton_track_row.rs`
- `tests/architecture_tests.rs`
