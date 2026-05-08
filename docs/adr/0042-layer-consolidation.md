# ADR 0042: Layer Consolidation — Primitive vs Composite vs Shell

## Status

Implemented — 2026-05-08.

Refines ADR 0023 (Design System), ADR 0033 (HIG UI Architecture
Governance), and ADR 0038 (Presentation Contract Enforcement) by
codifying when something belongs in `ui/primitives/`,
`ui/composites/`, or `ui/shells/`.

## Context

After ADRs 0023 → 0038 the UI layer has grown to:

- `ui/tokens.rs` + `ui/theme.rs` + `ui/contrast.rs` — design tokens.
- `ui/primitives/` — 11 files (Button, Label, Image, Surface,
  Divider, MultilineText, Stack, Popover, ContextMenu, Tooltip,
  SectionHeader, Loading).
- `ui/composites/` — 23 files including `disclosure_group`,
  `recent_feed_tile`, `release_detail_surface`, `track_inspector_pane`,
  `track_metadata_grid`, `track_detail_surface`, `now_playing_bar`,
  `playlist_popover`, `musicbrainz_panel`, `file_header`,
  `identity_action`, `tag_badge`, `thumbnail`, `action_button`,
  `action_row`, `list_row`, `track_row`, `detail_grid`, `detail_header`,
  `segmented_control`, `split_pane`.
- `ui/shells/` — 33 files split across `discover/`, `library/`,
  `artist`, `feed`, `playlist`, `track`, `entity`.
- Screens — `library.rs` (128 LOC), `search.rs` (238 LOC),
  `app.rs` (833 LOC), plus `app/` submodules.

The `composites/` directory now mixes two genuinely different things:

- **Reusable display fragments** with multiple call sites (Thumbnail,
  ListRow, DetailGrid, Button-extensions like `action_button`,
  TagBadge, SegmentedControl, DisclosureGroup, Popover).
- **Page-section blocks used in exactly one shell** (e.g.
  `release_detail_surface`, `track_inspector_pane`, `recent_feed_tile`,
  `now_playing_bar`, `track_detail_surface` in some configurations).

The latter group adds a layer hop without buying reuse. Each
"composite" file becomes a place to chase a definition that is only
called from one shell. ADR 0033 already names primitives, composites,
and shells but does not define when something *must* migrate between
them.

## Decision

Codify four layers with an enforceable single rule per layer:

### Primitive (`ui/primitives/*`)

- Renders a single visual concept (one element semantically).
- Token-driven; never holds domain vocabulary.
- `RenderOnce`, no internal state beyond display props.
- Rule: **no domain types in the public API.** No `TrackRow`,
  `FeedRow`, `EntityVm`, etc.

### Composite (`ui/composites/*`)

- Combines ≥ 2 primitives or ≥ 1 primitive + layout into a reusable
  display fragment.
- `RenderOnce`, stateless from the screen's perspective.
- Rule: **must have ≥ 2 distinct call sites.** A composite used in
  exactly one shell collapses into that shell.
- May accept domain-shaped *display data* (e.g. a `&str` title and
  a `SemanticColor`) but not raw domain types.

### Shell (`ui/shells/*`)

- A page or pane: stateful or single-call-site composition.
- May consume a view-model directly (`view_models::*`).
- May import primitives, composites, view-models, tokens, runtime
  handles. May NOT import `gpui_component` directly except through
  primitives/composites.
- Rule: **owns layout decisions for its page section.** Knows what
  a "track inspector" looks like. Composites do not.

### Screen (`src/{library,search,app}.rs` + `src/app/*`)

- Top-level GPUI entity for a tab.
- Wires shells, owns selected-entity state and command dispatch.
- Rule: **a screen must be < 300 LOC after migration.** Anything
  bigger means logic still lives there.

### Migration discipline

When adding new UI:

1. New display fragment used in one place ⇒ start in the consuming
   shell.
2. Same fragment needed in a second place ⇒ extract to a composite
   *at the second-use commit*, not preemptively.
3. A composite that loses its second call site collapses back into
   its remaining shell.

Audit cadence: every ADR closing a UI feature MUST run the
composite-call-site audit (see Implementation notes) and report any
single-use composites in its readiness gate.

### Naming hygiene

- Composites that overlap in role pick one name. Currently
  `track_row` and `list_row` overlap — `list_row` is the canonical
  name; track-specific behaviour is a configuration of `list_row`,
  not a separate composite.

## Consequences

Positive:

- Smaller `composites/` surface area (estimated 23 → ~16).
- Reading a shell becomes a local activity — no jumping into a
  composite that turns out to be one-shot wrapping.
- Future contributors get a clear rule: "did anyone else need this?
  No → it's a shell".
- Cuts duplicate-name confusion (track_row vs list_row).

Negative:

- One-time migration touches ~5–7 composite files plus their call
  sites. Mechanical.
- Some composite files have tests; tests must move with them into
  the shell module (`#[cfg(test)] mod tests` block at bottom of
  shell file).
- The "≥ 2 call sites" rule has a soft edge: a composite about to
  be reused in a planned-but-not-merged feature may be extracted in
  the same PR that introduces the second call site. Premature
  extraction is the failure mode this ADR prevents.

## Implementation result

- Composite call-site evidence is recorded in
  `docs/handoff/composite-audit.md`.
- The confirmed single-use composites were inlined:
  `recent_feed_tile` moved into `src/ui/shells/discover/recent.rs`,
  `track_inspector_pane` moved into
  `src/ui/shells/discover/track_inspector.rs`, and `now_playing_bar`
  moved into `src/app/playback_bar.rs`.
- The audit retained multi-use composites such as
  `release_detail_surface`, `track_detail_surface`, and
  `musicbrainz_panel` because they still have multiple real call sites.
- The `track_row` and `list_row` names remain intentionally separate:
  `track_row` is a domain composite backed by track-row view models,
  while `list_row` is the generic row primitive/composite shape it
  builds on.
- ADR 0033 and ADR 0038 reference this ADR for the composite-vs-shell
  rule, and `docs/architecture/architecture-current-snapshot.md`
  describes the post-ADR-0042 layer shape.
