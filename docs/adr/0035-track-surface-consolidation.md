# ADR 0035: Track Surface Consolidation

## Status

Proposed - 2026-05-02. Amended 2026-05-02 to widen scope per
`docs/plans/one-owner-per-surface-plan.md`: track row, inspector pane,
artwork handoff, label policy, and named architecture tests are now in
scope so the contract migrates once instead of twice.

## Context

Library and Discover both present tracks, but they currently express
different Human Interface structure for the same entity:

- Library track detail is metadata-heavy, narrow, and ID3-oriented.
- Discover track detail is compact, search-oriented, and link/action
  oriented.
- Library and Discover each render the track twice — once as a row in a
  list and once as a detail surface in an inspector pane — and the four
  resulting code paths have drifted in spacing, labels, action placement,
  and empty-state behavior.
- The same concepts use different labels (`Album/Feed` vs `Release`) and
  different action placement.
- Existing shared pieces (`TrackHeader`, `TrackRow`, `ActionRow`,
  `AddToPlaylistPopover`, `TrackMetadataGrid`, `MusicBrainzPanel`,
  `FileHeader`) are re-composed differently per screen, so the visible
  surface still drifts.
- Inline fallback coercions like `unwrap_or("Unknown Artist")`,
  `unwrap_or("Unknown Album")`, and `unwrap_or("[untitled]")` exist in
  `src/library.rs` (lines 164, 1604–1605, 1609–1610, 2171, 2980), so
  "what an empty value means" is decided per call site.

This is the same class of problem addressed by ADR 0033 and ADR 0034:
visible UI drift is a symptom of duplicated surface ownership. The fix is
not to make Library look like Discover or Discover look like Library. The
fix is one shared track surface — list row, inspector pane, and
full-screen detail — driven by one display contract, with explicit typed
slots for surface-specific capabilities.

Apple HIG layout, accessibility, and consistency guidance
(`apple-hig/foundations/`, `apple-hig/summaries/layout-complete.md`,
`apple-hig/summaries/typography-complete.md`,
`apple-hig/summaries/accessibility-complete.md`,
`apple-hig/components/buttons.md`) expects related items grouped
predictably, essential information easy to find, controls and content
visually distinct, and layout adaptive while remaining recognizably
consistent across surfaces. For this app, a track must have one
recognizable structure everywhere: artwork, kind badge, title, artist,
release context, primary actions, summary metadata, optional detail
sections, and optional advanced panels. The same core structure appears in
a list row at smaller scale and in an inspector pane at intermediate
scale.

## Decision

Create one shared track surface contract and exactly one composite owner
per layout: row, inspector pane, and full detail. One GPUI-free display
contract family drives all three layouts. Bind all label policy, fallback
strings, and slot definitions to the contract so screens cannot re-decide
them.

### Files

- `src/view_models/track_detail.rs` owns:
  - `TrackDetailVm` — display-ready facts for a single track.
  - `TrackRowVm` — projection of `TrackDetailVm` to row-shaped data
    (title, subtitle, trailing metadata, primary action). Same contract,
    smaller surface; the row is an intentional subset, not a parallel
    type.
  - `TrackDetailSlots` — GPUI-free typed enumeration of optional
    non-artwork slot categories the detail surface accepts (see "Slot
    taxonomy" below). Not a free-form callback bag.
  - `TrackDetailLabels` — a frozen vocabulary of canonical field labels
    (`release_label()`, `artist_label()`, `summary_section_title()`,
    etc.). Library and Discover read; neither overrides.
  - Fallback policy: `display_title`, `display_artist`, `display_album`,
    `display_release_context`, `display_kind_badge`. Each accessor
    returns `String` after applying the canonical fallback decided in
    one place. Empty-vs-unknown distinction is preserved by exposing
    `Option<String>` where the composite needs to render an empty-state
    rather than a labeled fallback.
  - Artwork fallback policy facts such as the display kind badge and any
    fallback accessibility text. The VM does not own resolved image
    handles.

- `src/ui/composites/track_detail_surface.rs` owns the detail layout:
  artwork slot, header (kind badge, title, artist, release context),
  primary action slot, summary detail grid, description slot,
  collapsible/lazy section slot, and advanced panel slot.

- `src/ui/composites/track_row.rs` owns the row layout. Already exists;
  this ADR binds it to consume `TrackRowVm` exclusively and forbids
  screen-side construction of row chrome from raw `db::Track` rows.

- `src/ui/composites/track_inspector_pane.rs` is a thin composite that
  composes `TrackDetailSurface` inside the inspector frame used by both
  Library and Discover. It is the *same* composite ownership as the full
  detail view; only the surrounding pane chrome differs.

### Slot taxonomy

`TrackDetailSlots` exposes named, typed slots. Empty is a first-class
value; the composite renders nothing, not a screen-decided placeholder.

| Slot | Type | Owner of empty-state |
|---|---|---|
| `artwork` | UI-layer composite input: `ResolvedArtwork` wrapping `Option<Arc<Image>>`, resolved by the screen | Composite renders the kind-badge fallback |
| `primary_actions` | `Vec<ActionRowItem>` (display contract from `view_models/`) | Composite renders nothing if empty |
| `summary_metadata` | `TrackMetadataGridVm` | Composite renders the section header only when at least one row is populated |
| `description` | `Option<String>` | Composite renders nothing for `None`; renders MultilineText for `Some` |
| `sections` | `Vec<TrackDetailSection>` (collapsible/lazy) | Composite renders the section list; per-section empty handled by the section VM |
| `advanced_panels` | `Vec<TrackDetailAdvancedPanel>` (e.g. ID3 compare, MusicBrainz lookup) | Composite renders the panel header; the panel VM owns its empty-state |
| `back_navigation` | `Option<NavigationContext>` | Composite renders the back affordance only when present |
| `external_links` | `Vec<ExternalLinkItem>` (feed, audio, Nostr, RSS) | Composite renders the links group only when non-empty |
| `contributors` | `Vec<ContributorItem>` | Composite renders the contributors group only when non-empty |
| `value_routes` | `Vec<ValueRouteItem>` | Composite renders the value-route group only when non-empty |

Library and Discover each populate the subset of slots they support.
Neither rebuilds chrome around a slot. Artwork is named in this taxonomy
because the surface owns its presentation, but the resolved image handle is
not part of the GPUI-free VM slot contract.

### Label policy

`TrackDetailLabels` is the single source of canonical user-facing labels.
Where Library said "Album/Feed" and Discover said "Release," the VM
chooses one wording and both screens use it. Differences in capability
(e.g. Library shows ID3 compare; Discover does not) are expressed as
present-or-absent slots, not as different labels for the same concept.

Fallback policy bindings (cross-references
`docs/plans/one-owner-per-surface-plan.md` Workstream 2):

- `TrackDetailVm::display_title` is the only owner of the `"Untitled"`
  fallback. Removes the inline fallback at `src/library.rs:164-165`.
- `TrackDetailVm::display_artist` is the only owner of the `"Unknown
  Artist"` fallback. Removes the inline fallback at
  `src/library.rs:1604-1605`.
- `TrackDetailVm::display_album` is the only owner of the `"Unknown
  Album"` fallback. Removes the inline fallback at
  `src/library.rs:1609-1610`.
- `TrackDetailLabels::summary_section_title()` is the only owner of the
  `"Tags"` fallback at `src/library.rs:2980`.
- `PlaylistVm::display_name` (covered separately by the one-owner plan)
  removes the `"[untitled]"` fallback at `src/library.rs:2171`.

### Artwork handoff

Image-cache lookup is screen-owned per ADR 0033. Artwork display is
composite-owned per this ADR. The handoff is fixed: screens resolve to
`Option<Arc<Image>>` and pass it as a UI-layer artwork input alongside
the `TrackDetailVm`. The composite never reaches into a service or cache,
and the GPUI-free VM never imports the image type.

### Empty and loading states

`TrackDetailSurface` accepts a `TrackDetailLoadState` enum (`Loaded`,
`Loading`, `Missing`, `Failed { reason }`). Loading and missing renders
are owned by the composite; screens dispatch the load and pass state.
Library and Discover may not draw their own loading skeletons for a
track surface.

## Invariants

- The track surface — list row, inspector pane, and full detail — has
  exactly one composite owner per layout (`TrackRow`,
  `TrackInspectorPane`, `TrackDetailSurface`). All three consume the
  same `TrackDetailVm` family and the same `TrackDetailLabels`
  vocabulary.
- Track surface display strings, fallback labels, and section titles
  have exactly one VM owner. Screens never decide what an empty title,
  artist, album, or section means.
- Library and Discover pass typed slot values into the shared surface;
  they do not rebuild header, summary, action, link, contributor, or
  value-route chrome locally.
- Advanced Library capabilities (ID3 compare, staged tag edits,
  MusicBrainz lookup) remain available as named entries in the
  `advanced_panels` slot.
- Discover capabilities (search-origin back navigation, feed links,
  audio play links, Nostr/RSS links, contributors, value routes) remain
  available as the `back_navigation`, `external_links`, `contributors`,
  and `value_routes` slots.
- `AddToPlaylistPopover` remains the only add-to-playlist popover owner.
- Image-cache lookup stays screen-owned; artwork passes into the
  composite as a UI-layer artwork input.
- Loading and empty rendering for a track surface are composite-owned;
  screens dispatch the load and pass a `TrackDetailLoadState`.
- Any new track UI must strengthen an ADR 0033/0034/0035 guard: a new
  shared composite, a new VM accessor, a new token role, a new slot, or
  a new architecture test.
- Visual smoke uses user-provided screenshots, not pointer automation.

## Non-Goals

- No backend, schema, service, or API changes.
- No metadata inference changes.
- No redesign of the whole Library or Discover screen.
- No removal of Library's advanced metadata workflows.
- No attempt to replace the inspector pane's surrounding shell (sidebar,
  filter bar, toolbar). Only the track surface inside the pane is owned
  by this ADR.
- No SwiftUI / AppKit port.

## Alternatives Considered

- Make Library visually match Discover. Rejected because Library has
  advanced metadata workflows that Discover does not need.
- Make Discover visually match Library. Rejected because Discover should
  stay focused and search-oriented.
- Continue sharing only `TrackHeader` and `TrackRow`. Rejected because
  the drift lives in the surrounding detail surface composition and
  label policy.
- Add more screen-local helper functions. Rejected because helper forks
  are the regression path ADR 0033 was created to stop.
- Land the detail surface first and consolidate `TrackRow` and the
  inspector pane in a follow-up ADR. Rejected because that schedule
  forces label policy and artwork handoff to be re-decided per
  migration; doing them once is cheaper than twice.
- Free-form `Vec<AnyElement>` slots. Rejected because typed slots are
  what makes architecture tests possible; an `AnyElement` slot is a
  callback bag that smuggles screen logic back into the composite.

## Enforcing tests

The following named tests in `tests/architecture_tests.rs` are the
mechanical gates behind this ADR. Renaming or removing any of them
requires a follow-up ADR update.

- `screens_do_not_define_local_track_detail_surface_chrome` — fails when
  any file in `SCREEN_FILES` constructs track-detail header, summary,
  action, link, contributor, or value-route layout outside
  `TrackDetailSurface`. Forbidden patterns include screen-local
  `render_track_detail_*` helpers.
- `screens_do_not_define_local_track_row_chrome` — fails when any file
  in `SCREEN_FILES` defines a `render_track_row*` helper or constructs
  row chrome outside `TrackRow`.
- `screens_do_not_construct_track_inspector_pane_locally` — fails when
  any file outside `src/ui/composites/track_inspector_pane.rs` composes
  inspector-pane track-surface chrome.
- `track_surface_consumers_use_track_detail_vm` — fails when a call site
  to `TrackDetailSurface`, `TrackInspectorPane`, or `TrackRow` is
  constructed with a backend row type or a hand-rolled struct rather
  than the `TrackDetailVm` family.
- `screens_do_not_inline_unknown_artist_or_album_fallbacks` — fails when
  the literals `"Unknown Artist"` or `"Unknown Album"` appear in
  `SCREEN_FILES`. Shared with `one-owner-per-surface-plan.md`
  Workstream 4.
- `screens_do_not_inline_untitled_fallback` — fails when `"Untitled"`
  or `"[untitled]"` appears in `SCREEN_FILES`. Shared with the
  one-owner plan.
- `track_detail_labels_owns_canonical_field_labels` — fails when the
  literal label strings owned by `TrackDetailLabels` (e.g. `"Release"`,
  `"Album"`, `"Feed"`, `"Tags"`) appear in `SCREEN_FILES` or in any
  composite outside `track_detail_surface.rs`,
  `track_inspector_pane.rs`, and `track_row.rs`.
- `track_surface_slots_are_typed` — fails when any
  `TrackDetailSurface`, `TrackInspectorPane`, or `TrackRow` builder
  exposes a slot setter typed as `AnyElement` or
  `impl IntoElement` rather than the typed slot value
  (`ActionRowItem`, `ExternalLinkItem`, `ContributorItem`,
  `ValueRouteItem`, `TrackDetailSection`,
  `TrackDetailAdvancedPanel`).

These extend ADR 0033's "Enforcing tests" list and follow the same rules:
no test grows a non-zero baseline; baselines only shrink.

## Consequences

- Track detail and row changes will touch `src/library.rs`,
  `src/search.rs`, `src/ui_track.rs`, and `src/ui_entity.rs`, but only to
  route through the shared VM/composite and provide slots.
- Some field labels will change or converge. Any label change is traced
  to `TrackDetailLabels`, not a screen-local preference.
- Inline fallback coercions named in the "Label policy" section above
  are deleted as the corresponding VM accessors land.
- The inspector pane track surface in Library and Discover renders from
  the same composite. Any divergence after this ADR is a bug, not a
  configuration choice.
- Loading and empty states become consistent across surfaces; screens no
  longer hand-roll skeletons.
- A final visual gate compares Library full detail, Library inspector
  pane, Discover full detail, and Discover inspector pane for the same
  track. Side-by-side screenshots confirm shared structure while each
  surface preserves its useful capabilities (Library shows advanced
  panels; Discover shows external links and contributors).
- Future track-surface UI work is delayed if it depends on chrome,
  labels, or slot shapes that have not yet been moved into the shared
  contract.
