# Design-system + ideal-architecture migration

## Status snapshot

Branch: `feat/design-tokens-and-primitives` (local only — no push, no PR
without explicit direction).

**Completed:**
- Tokens layer (`tokens.rs`): `Spacing`, `Radius`, `FontSize`, `Size`,
  `SemanticColor`, `Appearance`, `ScaleFactor`. Each dimension exposes
  both a `const fn px()` and a runtime-scaled `.scaled(cx) -> Pixels`.
- WCAG matrix tests for Dark and Light palettes.
- `theme_bridge::install_theme(appearance, scale, cx)` — stashes scale
  as `gpui::Global` and calls `cx.refresh_windows()` so live changes
  paint immediately.
- `ScaleFactor` persisted in `Config.ui_scale` + Settings panel picker
  (now built on `SegmentedControl`).
- Primitives shipped: `Button`, `Surface`, `Label`
  (with chainable `.weight()` / `.size()` / `.truncated()`),
  `Divider`, `Popover` (HIG arrow droplet, dismissal, focus trap),
  `MultilineText` (`SwiftUI` `Text(...).lineLimit(n)` shape — replaces
  the deleted `ui::text` shim across `search.rs`, `library.rs`,
  `ui_track.rs`), **`VStack` / `HStack` / `ZStack` / `Spacer`**
  (SwiftUI-style spacing containers).
- Composites shipped: `Thumbnail`, `TagBadge`, `DetailHeader`,
  `DetailGrid`, **`SegmentedControl`**, **`ListRow`**, plus
  `playlist_popover` (the original composite).
- Screens partially migrated:
  - `ui_artist.rs` → uses `VStack` + `DetailHeader` + `DetailGrid`,
    bound to `ArtistVm`.
  - `ui_feed.rs` → uses `VStack`, bound to `FeedVm`.
  - `ui_track.rs` → discover row uses `ListRow::compact`.
  - `app.rs` → scale picker uses `SegmentedControl` (removed
    `rgb(0xffffff)` literal from screen code).
- `ui_common.rs` rewritten as a delegating shim — every existing call
  site of `render_thumb` / `render_detail_header` /
  `render_detail_grid` / `render_detail_grid_elements` now goes through
  the composites, so library/search become scale-aware indirectly.
- **Scale bridge (`ui::sizable_bridge::SizableScaled`)** — discrete
  step shift from `ScaleFactor` into `gpui_component::Size`. All ~40
  `.with_size(Size::*)` call sites in `library.rs`, `search.rs`,
  `app.rs` migrated to `.scaled(Size::*, cx)`. Settings UI scale now
  actually moves Buttons / Inputs / Sidebar / Sheet / Table widgets.
- **`theme::badges` is the single source of truth for entity-type
  styling** — artist and release special cases folded directly into
  `text_color` and `emoji`; the `ui_common::{type_color, badge_text,
  type_emoji}` wrappers are gone.
- **View-models layer scaffold (`src/view_models/`)** with the layer
  rules documented in `mod.rs`:
  - No GPUI imports, no service mutation, every projection unit-
    testable without a `Window` / `App`, borrow don't clone, one
    module per screen.
  - `view_models/format.rs` — `fmt_runtime`, `fmt_date` (lifted out
    of `search.rs`).
  - `view_models/artist.rs` — `ArtistVm` projects `ArtistView` +
    `&[Feed]` into title / subtitle / track-count label / detail
    rows. `ui_artist::render_artist_view` rewritten to consume it.
  - `view_models/library.rs` — `LibraryTrackRowVm` (album-detail
    row projection: number prefix, title fallback, `M:SS` suffix,
    composed label, MB status text + semantic-kind bucket). One
    call site migrated (`render_library_track_row`). Pattern proved
    out for further library slices.
  - `view_models/feed.rs` — `FeedVm` projects `FeedView` + `&[Track]`
    into title / artist label / publisher text / sorted tracks /
    runtime / detail entries / header-feed shim.
    `ui_feed::render_feed_view` rewritten to consume it.

**Remaining (in priority order):**

* `view-model-library`, `view-model-search` — follow the
  `ArtistVm` / `FeedVm` / `TrackVm` pattern. Library and search are
  larger and entangled with screen state; tackle alongside the
  `screen-library` / `screen-search` extractions.
* `disclosure-group-composite` — fold the
  `div().id(...).cursor_pointer().child(SectionHeader.disclosure())`
  wrapper into a SwiftUI-style `DisclosureGroup` composite that
  owns header + content + on_toggle.
* `env-bundle` / `env-component-defaults` — Introduce
  `Environment { appearance, scale }` in `tokens.rs` as the single
  SwiftUI-style accessor and drop the hardcoded
  `appearance: Appearance::Dark` defaults from every primitive.
* `screen-library`, `screen-search` — bind the giant screens to
  their view-models + composites; where the bulk of remaining
  hardcoded `px(N)` literals live (~120 call sites).
* `audit-token-usage` — final grep for raw `rgb(…)` /
  `px(<number>)` outside `tokens.rs` / `theme.rs` / primitives /
  composites.
* **Cross-module duplication noted, deferred:** `track_title` and
  `fmt_dur` still exist (slightly different) in `library.rs` and
  `metadata.rs`. Once `view-model-library` lands they should
  collapse onto `view_models::track::TrackVm` /
  `view_models::track::fmt_dur` too. The `metadata.rs` copy is
  service-side and may stay if the service module shouldn't depend
  on `view_models`.

## Direction (user, this turn)

> Adhere more strongly to the ideal architecture. Instead of monkey-
> patching components, re-implement them keeping their spirit, or
> adjust them in a new definition, then rebuild the views with those
> for coherence. Address the backend/view-model approach first.

Layered architecture in force:

```
db / *_service / api  (domain, no GPUI)            ← already exists
        ▲ read / write
        │
view_models/                                       ← NEW LAYER
  - own UI state (selection, filters, "what's showing")
  - subscribe to service changes
  - expose display-ready data + commands to the view
        ▲ observe / dispatch
        │
ui/primitives/        (Button, Surface, Popover…)  ← shipped
ui/composites/        (PlaylistPopover, …)         ← shipped (1)
        ▲ bind
        │
screens/  (library / search / settings …)         ← thin
```

## Plan

### Track E — view-models scaffold (do first)

* `view-models-scaffold` — Create `src/view_models/mod.rs` with the
  pattern doc + one tiny example. Document the observation pattern
  (`cx.subscribe` to service events / `cx.observe` on shared entities).
* `view-model-track` — Extract `TrackInspectorViewModel` from
  `ui_track.rs` as the reference (smallest touch surface).
* `view-model-feed` — Same for `ui_feed.rs`.
* `view-model-artist` — Same for `ui_artist.rs`.
* `view-model-library` — `LibraryViewModel` for `library.rs`.
* `view-model-search` — `SearchViewModel` for `search.rs`.

### Track F — re-implement composites cleanly

Built on primitives + tokens, no monkey-patching of gpui-component.

* `composite-list-row` — `ListRow` (used by playlist tracks, cached
  files list, search results).
* `composite-detail-header` — replaces `ui_common::render_detail_header`.
* `composite-detail-grid` — replaces `render_detail_grid` /
  `render_detail_grid_elements` (currently `px(124)`, `px(17)`).
* `composite-thumbnail` — replaces `render_thumb` (hardcoded radii).
* `composite-tag-badge` — replaces inline badge `div`s.
* `composite-segmented-control` — UI scale picker is the first user.

### Track G — bind screens to view-models + composites

Each screen becomes a thin `Render` impl: `Entity<XxxViewModel>`,
composites only, forwards events to VM commands.

* `screen-ui-track`
* `screen-ui-feed`
* `screen-ui-artist`
* `screen-ui-common` — refactor helpers into composites instead.
* `screen-library` (large)
* `screen-search` (largest)

### Track D — final audit

* `audit-token-usage` — grep for raw `rgb()` / `rgba()` / hex literals
  and `px(<number>)` outside `tokens.rs` / `theme.rs`.

## Conventions in force

* Every new file: `#![warn(clippy::pedantic)]`, exceptions documented
  with `#[expect(reason = …)]`.
* Both Light and Dark palettes pass WCAG (`dark_palette_meets_wcag`,
  `light_palette_meets_wcag`).
* All sizing in screen / composite code goes through `.scaled(cx)`.
  Primitive-internal constants may use `.px()` only when consumed at
  construction time outside a render path.
* `cargo fmt && cargo clippy --lib --tests && cargo test --lib` clean
  before any todo is marked done.
* No automatic push or PR. Commits made locally only.

## Out of scope (this branch)

* Visual redesign of any screen beyond what falls out of using the
  primitives correctly.
* Light-theme colour tweaks beyond keeping the WCAG matrix passing.
* New features in services / db / playback.

---

## Session & Checkpoint Paths

Paths containing session artifacts, checkpoints, and the working plan used during this migration:

- Session folder (current): /Users/dbrasdasilva/.copilot/session-state/9f02ebb8-7837-472c-81d4-d6a57673ceec/
  - plan.md — current human-readable plan for this session
  - checkpoints/ — numbered checkpoint folders; index.md lists each checkpoint and summary
  - files/ — persistent session artifacts created during work
- Global session-state parent: /Users/dbrasdasilva/.copilot/session-state/ (contains all sessions)
- Repo-local plan (this file): /Users/dbrasdasilva/dev/vcs-codebases/github.com/InTheMorning/v4vmm/remaining_plans.md
- Repo root (code + git history): /Users/dbrasdasilva/dev/vcs-codebases/github.com/InTheMorning/v4vmm/

## How to interpret these artifacts (for other automated agents)

1. Checkpoints first: read checkpoints/index.md inside the session folder to obtain a chronological list of changes, motivations, and file diffs for each checkpoint. Use it as the primary narrative.

2. Read plan.md: it is the authoritative human plan for current work. It lists completed steps, remaining todos, and the intended order. Use it to decide next tasks.

3. Use SQL `todos` table as the canonical work queue. Query ready todos, mark `in_progress` before starting, and `done` when verified. Example SQL:
   - SELECT id, status FROM todos WHERE status = 'pending';
   - UPDATE todos SET status = 'in_progress' WHERE id = 'foo';
   - UPDATE todos SET status = 'done' WHERE id = 'foo';

4. Code-change rules:
   - Make surgical, minimal changes that fully address the todo.
   - Adhere to repo conventions (see plan "Conventions in force").
   - Add `#![warn(clippy::pedantic)]` to new files and document exceptions with `#[expect(...)]`.
   - Always run: cargo fmt && cargo clippy --lib --tests && cargo test --lib
   - Do not push or open PRs; commit locally only. Do NOT add any `Co-authored-by:` AI trailers.

5. Key files of interest (quick map):
   - tokens & theme: src/ui/tokens.rs, src/ui/theme_bridge.rs
   - primitives: src/ui/primitives/*.rs
   - composites: src/ui/composites/*.rs
   - view-model layer: src/view_models/*.rs
   - large screens: src/library.rs, src/search.rs

6. Verification & handoff:
   - After implementing a todo, run the build/test/lint pipeline and capture results.
   - Update the `todos` table accordingly and append a short summary to plan.md (or checkpoints/) describing what changed and why.
   - If the change is nontrivial, create a checkpoint entry under checkpoints/ with a brief rationale and files changed.

7. If uncertain about behavior or encountering design choices, prefer the "View-model first" approach: extract a small VM, unit-test it, then wire the screen to it. Ask human only for major UI/UX policy deviations.

8. For cross-agent coordination: prefer idempotent operations, document assumptions in plan.md, and leave explicit follow-ups as SQL todos.


-- End of appended guidance --
