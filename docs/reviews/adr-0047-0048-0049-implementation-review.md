# ADR 0047 / 0048 / 0049 — implementation review

- **Date:** 2026-05-16
- **Reviewer:** Claude (Opus 4.7)
- **Scope:** the eight commits between `0e8c732` and `bee1ac2` on `master` that landed the library/search unification (ADR 0047), the content-list-frame breadcrumb search (ADR 0048), and the inspector source ownership model (ADR 0049).

Commits reviewed (oldest → newest):

```
0e8c732 feat: workspace navigation ownership (ADR 0047)
f225faf feat: frame shell breadcrumb + search results inspector shell
4e91e38 feat: saved searches in library app
4e17477 feat: mpv driver remediation plan + composite call-site audit (docs only)
64e24cb feat: text filtering across view models
9cca749 feat: active frame search dispatch (NewFrame / ActiveFrame modifiers)
19515b6 docs: plan content-frame search ownership (ADR 0048, 0049)
bee1ac2 feat: unify content-frame search and feed ownership
```

## Method

- Read each commit's diff scope and per-file deltas via `git show`.
- Pulled governance source-of-truth from ADRs 0023 (design system + view models), 0025 (theme/icon/style boundary), 0033 (HIG UI architecture governance), 0034 (scale-aware tokens), 0038 (presentation contract enforcement), 0042 (layer consolidation), 0046 (workspace frame architecture), 0047, 0048, and 0049.
- Inventoried `src/ui/composites/` (27 composites) and the workspace/search-results shell stack; counted call sites for the major composites.
- Skimmed `tests/architecture_tests.rs` (~125 guards) to understand what classes of drift the suite catches.
- Verified the contested or non-obvious claims with direct reads of `src/app.rs` (tab switching, render dispatch), `src/ui/shells/search_results_inspector.rs` (empty-state path, dead-code markers), `src/ui/composites/breadcrumb_trail.rs` (glyph reuse), and the `discover/` import graph.

## Architecture & visual ownership recap

The codebase enforces an eight-layer model (per ADRs 0042, 0038): screens → ui shells → ui composites → ui primitives → ui foundations → view models → application → backend services. Going down the stack, GPUI disappears at the view-model boundary, services disappear at the composite boundary, and domain vocabulary disappears at the primitive boundary. Composites earn their module by having ≥2 distinct call sites; otherwise they collapse into the consuming shell (ADR 0042).

Visual ownership (ADRs 0023, 0025, 0033, 0034):

- Spacing, typography, color, and iconography flow through `Spacing`, `FontSize`, `SemanticColor`, and `IconName` tokens. User-facing dimensions resolve through `.scaled(cx)`.
- A theme bridge resolves `Appearance × ScaleFactor`. Profiles (Dark, Light, HighContrastDark, HighContrastLight, System) are named and palette-checked against WCAG.
- Frame chrome (title, back, forward, breadcrumb, filter chip strip, menu, close) lives in `src/ui/composites/frame_shell.rs`. Screens consume it through `FrameShellDisplay`/`FrameShellSlots`; hand-rolled floating chrome is forbidden by an architecture guard.
- Native `gpui_component::Button` is allowed in screens/composites only with a `// CONTROL-COMPAT(reason)` comment.

Frame ownership (ADRs 0046, 0048, 0049):

- `WorkspaceLayout`, `WorkspaceFrameId`, `FrameNavigationState`, and `FrameNavigationEntry` live in `src/view_models/workspace.rs` and are GPUI-free.
- Toolbar search pushes `FrameNavigationEntry::Search(query)` onto the ContentList frame's nav stack. No Detail frame is spawned.
- ContentList body switches on its nav top: `SourceList` → library tree / `Settings` → settings body / `Search(q)` → search-results inspector / `*Detail(id)` → entity inspector.
- The ContentList VM (not the Library source tree) owns the All / Library / Index source filter.

Inspector ownership (ADR 0049):

- ContentList detail VM carries origin + membership for mixed-origin rows; the source tree never filters by All/Library/Index.
- Removed-but-known-by-Index rows remain visible under All/Index with a Download action.
- Uncached Index activations push their own `IndexFeedDetail` / `IndexTrackDetail` nav entries and render through the same breadcrumb-backed body.

Mechanical enforcement: `tests/architecture_tests.rs` contains roughly 125 guards spanning layer boundaries, token usage, floating-chrome bans, frame-shape, breadcrumb sync, and view-model contract leaks.

## Findings — strengths

1. **Frame architecture compliance.** `render_workspace_content` (`src/app.rs:1744-1984`) does an exhaustive match on the ContentList nav top to pick the body. The Detail frame is no longer auto-spawned by toolbar search. New guards lock this: `global_search_routes_to_content_list`, `nav_top_drives_content_list_body_switch`, `breadcrumb_pop_syncs_library_detail`, `search_results_detail_syncs_with_search_nav_flow`.

2. **Tab consolidation matches ADR 0048.** `AppTab` is now `Library | Settings` only; the Search tab and the `WorkspaceScreenMount::Search` variant are retired. Switching to Settings stashes the current ContentList nav into `TopApp::last_library_content_nav` and resets the nav to `FrameNavigationEntry::Settings` (`src/app.rs:463-475`). Switching back to Library restores the stashed nav (`src/app.rs:478-489`), then hydrates `LibraryApp` detail from whatever nav entry is current. Both paths call `sync_search_results_detail_with_nav` after the nav change.

3. **Visual reuse is healthy.** The new `src/ui/shells/search_results_inspector.rs` composes existing composites: `SegmentedControl` for the tab switcher (Artists / Feeds / Tracks), `ListRow` + `Thumbnail` + `TagBadge` for result rows, and `EmptyStateDisplay` (imported from the VM) for empty bodies via `render_empty_state` (`src/ui/shells/search_results_inspector.rs:424-465`). No row, tab, or empty-state UX was hand-rolled. Subagent inventory across `src/ui/composites/` shows broad reuse: `Thumbnail` 15 call sites, `TagBadge` 8, `DisclosureGroup` 8, `TrackRow` 10, `SegmentedControl` 5.

4. **Breadcrumb HIG correctness.** The breadcrumb separator is `Icon::new(IconName::ChevronRight)` (`src/ui/composites/breadcrumb_trail.rs:141`) — a proper SF Symbol reuse, not an emoji or a hand-typed glyph. Inactive segments render through `ControlStyle::Ghost`; the current segment renders as static text. Foreground colors come from `SemanticColor` and resolve correctly under both `Appearance::Dark` and `Appearance::Light`. Font size is `FontSize::Micro.scaled(cx)`.

5. **Token discipline holds.** Verified reads of `frame_shell.rs`, `breadcrumb_trail.rs`, `split_pane.rs`, and the new search-results inspector found no raw `rgb()` or `px(N)` literals. `SplitPane` exposes `leading_width` / `leading_min_width` parameters and is supplied with `layout::CONTENT_PANE_DEFAULT_WIDTH` and `layout::CONTENT_PANE_MIN_WIDTH` from the shell side. Frame chrome padding uses `Spacing::{XS, SM, MD, LG}` exclusively.

6. **Inspector source ownership (ADR 0049) implemented.** `IndexSearchResultRows`, `IndexDetailKind`, and `IndexDetailDisplay` were added to `src/view_models/search_results.rs`. Each row carries origin + membership state. `SearchResultsPagedTab<Row>` gained `ready_library`, `replace_index_rows`, and `empty_state_for_scope` to project mixed-origin results into the three (All / Library / Index) scopes without touching the SourceList tree's data.

7. **Resize is fluid.** `SplitPane` (`src/ui/composites/split_pane.rs`) exposes `on_resize_start` / `_move` / `_end` accepting GPUI mouse events. `WorkspaceSlots` wires these handlers into `TopApp::begin_content_pane_resize` / `resize_content_pane` / `end_content_pane_resize` (`src/app.rs:1956-1978`). The divider's hover affordance shifts from `SemanticColor::border_subtle` to `accent` — the HIG-style drag cue, not a static grab line.

8. **Doc governance closeout complete.** ADR 0048 (`docs/adr/0048-content-list-frame-breadcrumb-search.md`) and ADR 0049 (`docs/adr/0049-inspector-source-ownership.md`) are now in place. ADR 0047 carries a forward-pointer to ADR 0048. The superseded `active-frame-search-dispatch-plan.md` is marked `Superseded 2026-05-16`. Review checklists exist for active-frame-search-dispatch (Superseded), ADR 0047, and inspector-source-ownership.

## Findings — drift and concerns

### P1 — file-size drift in `app.rs`, `workspace.rs` VM, and `search_results.rs` VM

| File | LOC | Note |
|---|---:|---|
| `src/app.rs` | 2,922 | `bee1ac2` alone added ~2,042 lines |
| `src/view_models/workspace.rs` | 2,904 | Frame state + nav state + breadcrumb projection + search-results helpers + text-filter state all colocated |
| `src/view_models/search_results.rs` | 1,408 | `bee1ac2` added 781: `IndexSearchResultRows`, `IndexDetailKind`, `IndexDetailDisplay`, paged-tab scope methods |

ADR 0042 specifically targeted shrinking top-level orchestration. These three files are now the three largest in the UI tree and have been growing in lockstep with each ADR in this region. Future work on the same surfaces will compound the size pressure.

Material added to `src/app.rs` in this arc: `render_workspace_content` body switch, `handle_search_result_selected`, `handle_content_list_breadcrumb_select`, `sync_search_results_detail_with_nav`, `start_index_search_for_query`, `RemoteDetailThumbnailState`, the fluid-resize handler trio, and the Library/Settings tab switching logic.

Suggested split (deferred — not in scope here):

- `src/app/search_dispatch.rs` for `submit_global_search*`, `handle_search_result_selected`, `start_index_search_for_query`, `sync_search_results_detail_with_nav`.
- `src/app/breadcrumb.rs` for `handle_content_list_breadcrumb_select` and breadcrumb labelers.
- `src/app/resize.rs` for pane resize handlers + state.
- `src/view_models/workspace/` directory: `frame_state.rs`, `nav_state.rs`, `breadcrumb.rs`.
- `src/view_models/search_results/` directory: `paged_tab.rs`, `index_detail.rs`, `empty_state.rs`.

### P1 — `discover/` is parked code, ~5,000 LOC, no exit plan

`src/discover.rs` (263 LOC) + `src/discover/app_impl.rs` (2,728 LOC) + `src/ui/shells/discover/*` (~15 files) survive the rename from `search` → `discover` performed in `bee1ac2`. The decision to rename rather than delete was deliberate: it preserves capability for a possible future re-activation.

Import graph today: no path from `src/app.rs` or `src/library/` reaches these shells. They are only imported by other discover shells, by `src/ui/shells/feed.rs`, and by `src/ui/shells/track.rs` — none of which are rendered from the composition root.

Risk:

- A future architecture audit will read this as dead code and propose deletion, losing whatever institutional knowledge sat in those shells.
- It will rot under refactor pressure (compile-fail under tokens/layer changes) and be deleted in haste.

Recommended follow-up: add `docs/notes/2026-05-discover-module-parked.md` documenting (a) why the module was preserved, (b) which capability it represents, (c) when it returns to the visible UI, (d) the conditions under which deleting it becomes acceptable. Add an architecture test pinning the module's `pub(crate)` surface so future deletions are conscious choices, not drift.

### P2 — text-filter helper duplication across VMs

`64e24cb` added text filtering to five VMs (queue, library content list, playlist, feed, search-results). Verified duplication:

- `normalize_text_filter` is defined as a file-local free fn in **both** `src/view_models/queue_now_playing.rs:431` and `src/view_models/library.rs:2147`.
- Per-VM matchers: `track_matches_text_filter` (`feed.rs:154`), `track_row_matches_text_filter` (`library.rs:2153`), `queue_row_matches_text_filter` (`queue_now_playing.rs:437`), `matches_text_filter` (`library.rs:692`). Each searches a different subset of fields (3–7) with no shared baseline of which fields count as searchable.

Recommended follow-up: extract `src/view_models/text_filter.rs`:

- `pub(crate) fn normalize(filter: Option<String>) -> Option<String>` — one definition.
- A small `pub(crate) trait Searchable { fn searchable_fields(&self) -> impl Iterator<Item = &str>; fn matches(&self, needle: &str) -> bool { /* default impl walks fields */ } }`.
- Each VM either implements the trait on its row type or calls `normalize` + `contains_normalized(haystack, needle)`.

Add an arch guard: a file-local `fn normalize_text_filter` outside `view_models::text_filter` is forbidden.

### P2 — single 2,042-line `bee1ac2` commit

The "unify content-frame search and feed ownership" commit bundles a module rename (`search` → `discover`), Search-tab removal, ContentList-frame search integration, Index detail VM introduction, async index search wiring, fluid resize wiring, and ~200 LOC of test changes.

Rollback granularity is now coarse: pulling any one concern (e.g., async index search if it ships broken) means reverting the lot.

Recommended follow-up: future ADR closeouts that span this many surfaces should land as a stacked PR or sequential commits on the same branch. Each concern has its own arch-test cluster and could have been a discrete commit.

### P2 — two render entry points for the same inspector shell

`render_search_results_inspector` (tabbed) at `src/ui/shells/search_results_inspector.rs:89` and `render_search_results_inspector_scoped` (single-tab, used for Index detail bodies) at `:106` both delegate to `render_search_results_inspector_with_scope` with a `SearchResultsHeaderMode` enum. The public surface is two thin wrappers around one private fn.

Recommended follow-up: collapse to a single public `render_search_results_inspector(vm, slots, header_mode, cx)` taking `SearchResultsHeaderMode`. Updates `src/app.rs:45-46,1794,1817`.

### P3 — `frame_shell` composite has 1 call site

ADR 0042 requires composites to have ≥2 distinct call sites. `frame_shell.rs` (438 LOC, the largest composite in the tree) is only used by `WorkspaceShell::render`.

This is borderline rather than a violation: the composite is the canonical home of frame chrome under ADR 0033, and centralizing title + back + breadcrumb + filter chip strip + close + menu earns its module even at one caller. A future ADR-0042-style audit will, however, propose collapsing it without that context.

Recommended follow-up: add a module-level doc comment in `frame_shell.rs` documenting the single-call-site exception with a pointer to ADR 0033, or mention the exception explicitly in ADR 0048.

### P3 — `content_pane_width` not persisted

The plan called out v1 ships in-memory only. After a relaunch, the user re-drags the divider. This is a known gap, not a regression.

Recommended follow-up: persist to `config.toml` alongside other workspace prefs (a `[workspace]` section or new `WorkspaceLayoutPrefs` table). Out of scope for this arc; queue for the next workspace ADR.

## Recommended remediations

| ID | Action | Touch points | Shape |
|---|---|---|---|
| R1 | Split `src/app.rs` into `src/app/{search_dispatch,breadcrumb,resize}.rs` | `src/app.rs`, `src/app/mod.rs`, arch guards | Multi-stage |
| R2 | Split `src/view_models/workspace.rs` into a submodule directory | `src/view_models/workspace/{mod,frame_state,nav_state,breadcrumb}.rs`, tests | Multi-stage |
| R3 | Split `src/view_models/search_results.rs` into a submodule directory | `src/view_models/search_results/{mod,paged_tab,index_detail,empty_state}.rs`, tests | Multi-stage |
| R4 | Extract `src/view_models/text_filter.rs`; arch guard against per-VM `normalize_text_filter` | 5 VMs + new module + 1 arch test | Small |
| R5 | Document `discover/` parked status in `docs/notes/`; arch test pinning surface | `docs/notes/`, `tests/architecture_tests.rs` | Small |
| R6 | Collapse `render_search_results_inspector` + `_scoped` to one public fn with `SearchResultsHeaderMode` | `src/ui/shells/search_results_inspector.rs`, `src/app.rs:45-46,1794,1817` | Small |
| R7 | Document `frame_shell` single-call-site exception in module doc | `src/ui/composites/frame_shell.rs` | Trivial |
| R8 | Persist `content_pane_width` to `config.toml` | new prefs slot + load/save path | Follow-up ADR |

R1–R5 are file-organization debt. They should land as one consolidation phase (its own ADR) rather than dribbled out per future feature ADR. R6–R7 are quick wins. R8 is a future ADR.

## Verification path for any follow-up

The 5-gate must pass after each remediation:

```bash
cargo fmt -- --check
cargo build 2>&1 | tail -5
cargo test --lib 2>&1 | tail -5
cargo test --test architecture_tests 2>&1 | tail -5
cargo clippy -- -D warnings 2>&1 | tail -5
```

Operator-visible verification (no behavioral change expected from R1–R7):

- App launches; workspace renders ContentList + Queue with a draggable divider.
- Toolbar search pushes onto ContentList nav; breadcrumb shows `Library › Search: …`.
- Result row drill-down pushes the correct `*Detail` nav entry; the back chevron and breadcrumb segments pop correctly.
- Library tab restores the prior search nav; Settings tab stashes and restores.

## Out of scope

- Async-runtime audit of `start_index_search_for_query` (separate review).
- `RemoteDetailThumbnailState` lifecycle / race-condition review.
- Accessibility audit (separate apple-hig-flavored pass on a11y labels, focus rings, keyboard nav).
- The `mpv-playback-driver` remediation plan added in `4e17477` (unrelated to this UI surface).

## Bottom line

The eight commits land the ADR-0047 / 0048 / 0049 ContentList-frame search consolidation cleanly. Visual reuse is intact (no row, tab, empty-state, or chrome was reinvented). Token discipline holds. HIG-relevant choices — SF Symbol chevron, segmented control for tabs, fluid drag with a subtle hover cue, a sparse top-level tab set — are correct. The drift is structural, not stylistic: `app.rs`, the workspace VM, and the search-results VM are now the three largest UI-tree files in the repo, and the `discover/` rename left ~5,000 LOC of parked code with no sunset. None of that is urgent; it is the right next consolidation pass.
