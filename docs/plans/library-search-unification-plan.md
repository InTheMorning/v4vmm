# Library + Search Unification Plan

## Status

Promoted - 2026-05-14. Pre-ADR concept doc promoted to
`docs/adr/0047-library-search-unification.md`. Builds on ADR 0046
(Workspace Frame Architecture) and the older entity-render
unification plan in `docs/plans/unify-discover-library-views.md`. The
seven open questions were resolved 2026-05-14 (see §Resolved
Decisions) and the resolutions are folded into the ADR and phase plan.

## Goal

Stop treating Library and Search as separate UI surfaces. There is one
content surface — the workspace `ContentList` frame — and one place
where search results land — the `Detail` (inspector) frame, opened with
a breadcrumb trail back to the originating query. The user reaches
search results by submitting a query from the global toolbar; they do
not enter a "Search tab" or get a left-pane search view.

Filter chips (`All`, `Library`, `Index`) replace the old search-scope
chips. They no longer change which screen renders — they live in each
content-showing frame's chrome and filter that frame's visible rows.

## Non-Goals

- No redesign of the global toolbar search input (per ADR 0043).
- No removal of the Library source list (`SourceList` frame); the
  library tree still owns sidebar navigation.
- No change to the playback engine or queue.
- No change to MusicBrainz lookup semantics.
- No changes to file ingestion, ID3 write boundary, or DB schema.

## Current State

- `src/search.rs` and `src/library.rs` render the same entity types
  with two parallel code paths (see
  `docs/plans/unify-discover-library-views.md` §Context).
- Global toolbar search lives in `src/app/tab_bar.rs` (ADR 0043) and
  feeds a Search tab whose body renders results in the left pane plus
  inspector.
- Three search scopes (`All`, `Library`, `Index` / `GlobalSearchScope`)
  switch which dataset the Search screen queries.
- ID3 compare and MusicBrainz panels are always present on the
  Library track inspector, even for non-downloaded items.
- Search feed and track inspectors already have collapsible
  Contributors and Value-route sections.
- Description on feed/track inspectors is always expanded.
- Library track inspector exposes extra fields (file path, format
  warnings, MusicBrainz status, ID3 frame groups) unconditionally for
  every selected track, downloaded or not.

## Target State

### One content surface

- `ContentList` frame is the single content surface. It renders the
  active source-list selection (a library node, a playlist, a saved
  search, etc.).
- A global search submit replaces the active `ContentList` content
  with a search-result content type whose origin (the query) is
  carried by frame navigation state.
- `SourceList` is unchanged: library tree, playlists, saved searches,
  settings entry. The user never sees a "Search" sidebar item; the
  toolbar input is the only search entry point.

### Search results land in the inspector frame with breadcrumbs

- Submitting a search opens or focuses the `Detail` frame and renders
  a `SearchResultsInspector` view inside it.
- The `Detail` frame chrome carries breadcrumbs: `Search › "<query>" ›
  <selected entity>`. Clicking a breadcrumb segment navigates back via
  the frame nav state (per ADR 0046).
- Drilling into a result (artist → feed → track) pushes entries onto
  the same frame's nav stack. Back chevron + breadcrumb both work.
- `SourceList` and `ContentList` selection is **not** disturbed by
  search. The user can still see their library context while
  inspecting search results.

### Per-frame filter chips replace scope chips

- Every content-showing frame (`ContentList`, `SearchResultsInspector`,
  artist view, feed view) owns its own filter chip strip: `All`,
  `Library`, `Index`.
- Chips no longer switch screen identity and are not global. Each
  frame independently filters its own rows by source membership.
- Chip semantics:
  - `All`: include every row from local DB + Musicindex.
  - `Library`: only rows where `is_in_library = 1`.
  - `Index`: only rows from Musicindex (federated discovery).
- Filter state lives on the frame VM; default is `All`. Two frames
  showing different content can carry different filter states.
- Narrow frame widths collapse the chip strip into a pull-down with
  the current selection as the trigger label.

### Search-results inspector is tabbed

- `SearchResultsInspector` renders three tabs: `Artists`, `Feeds`,
  `Tracks`. The active tab determines which result list is visible.
- Tab state persists for the lifetime of the inspector instance.
- Each tab honors the inspector's own filter chip state.
- Empty result set per tab renders an empty-state notice instead of
  hiding the tab.

### Saved searches

- Saved searches appear as `SourceList` entries beneath playlists.
- Activating a saved search opens it in a `Detail` frame as a
  `SearchResultsInspector`, just like an ad-hoc query, with the
  saved query carried in frame nav state.
- Same breadcrumb chrome and tabbed layout as ad-hoc searches.

### Empty filter state

- When the active filter selects zero rows (e.g., `Library` with no
  local tracks matching), the frame renders an explicit empty-state
  notice that names the active filter and offers to clear it.

### Compare ID3 and MusicBrainz controls follow download state

- Track inspector still surfaces "Compare ID3" and "MusicBrainz"
  controls.
- Both controls render disabled (greyed) when the selected track is
  not downloaded (`local_path.is_none()`).
- Disabled state honors HIG: dimmed glyph, no click handler, tooltip
  text "Download track to enable" (or equivalent).
- This unifies behavior across Library- and Search-origin track
  inspectors: previously Library always showed them enabled (even when
  files were missing) and Search omitted them.

### Library-extra fields render on demand only

- Track inspector renders a compact metadata core by default:
  title, artist, album, duration, release date, contributors, value
  routes, description, image.
- The expanded Library-only fields (local file path, ingest
  timestamps, ID3 frame groups, format warnings, MusicBrainz match
  detail) render only after the user clicks "Compare ID3" or
  "MusicBrainz" on a downloaded item.
- Clicking either control reveals its panel inline and unlocks the
  corresponding extra-field group. Closing the panel returns the
  inspector to the compact view.
- Non-downloaded items never reveal the extra-field groups, even if
  the user attempts to expand them (the controls are disabled).

### Collapsible Description

- Description section on feed and track inspectors gains a disclosure
  control matching the existing Contributors and Value-route sections.
- Default state: **collapsed** when the description body exceeds five
  rendered lines; **expanded** when five lines or fewer. Threshold
  measured against the rendered line count at the current frame
  width, not raw character length.
- Collapse state persists per inspector instance.

## Mapping to Workspace Frames (ADR 0046)

| Surface | Frame kind | Owner |
| --- | --- | --- |
| Library tree, playlists, saved searches | `SourceList` | source-list shell |
| Active source content (library rows, playlist rows) | `ContentList` | content-list shell |
| Entity inspector (artist / feed / track / search results) | `Detail` | inspector shell |
| Queue + transport + liveValue | `QueueNowPlaying` | queue shell |

Search results are a `Detail` content type. They reuse the same frame
chrome (back/forward, close, menu) as any other inspector view.

## Apple HIG Considerations

- **Breadcrumbs**: HIG path-bar pattern. Truncate middle segments
  with an ellipsis at narrow widths; leftmost (origin) and rightmost
  (current) segments remain visible.
- **Filter chips**: render as a small segmented control inside each
  content-showing frame's chrome, not on the global toolbar.
  Single-select. Selected chip uses accent fill; unselected chips use
  the chrome surface tone. Narrow frame widths collapse the strip
  into a pull-down whose label shows the active filter.
- **Tabbed inspector**: HIG segmented-tab or pill-tab pattern at the
  top of the `SearchResultsInspector` body. Single-select. Tab labels
  are static; counts (e.g., `Artists (12)`) may be appended.
- **Disabled controls**: dimmed glyph + label, no click handler,
  tooltip explains why (HIG: disabled controls should communicate
  their state).
- **Disclosure controls**: chevron points right when collapsed, down
  when expanded (matching existing Contributors / Value-route
  sections).
- **Discoverability**: the previous search-scope chips do not
  disappear from the UI; they relocate from a global toolbar control
  to per-frame chrome where their filtering effect is visibly local.
- **Empty state**: HIG content-unavailable view inside the frame —
  symbol, short title, secondary description, optional "Clear
  filter" button.

## Architectural Sketch

### View models

- Add `ContentFilter` enum (`All`, `Library`, `Index`) once; every
  content-showing frame VM owns its own `filter_state: ContentFilter`
  field. No global filter state.
- Add `FilterChipStripDisplay { id, options, selected,
  narrow_collapse_to_pulldown: bool }` as the GPUI-free display
  contract consumed by frame chrome (lives next to `FrameShellDisplay`
  in `src/view_models/workspace.rs` or its own module under
  `src/view_models/`).
- Add `SearchResultsInspectorPageVm` with display contract: query,
  active tab (`Artists` | `Feeds` | `Tracks`), per-tab result lists,
  per-tab paged windowing per ADR 0041, breadcrumb stack, filter
  state, empty-state messaging.
- Add `inspector_expanded_panels: BTreeSet<InspectorPanelKind>` on
  the track-inspector VM where `InspectorPanelKind ∈ { CompareId3,
  MusicBrainz }`. Library-extra field render guards on membership.
- Add `description_state: DescriptionState` enum
  (`AutoCollapsed` | `AutoExpanded` | `UserCollapsed` | `UserExpanded`)
  to feed and track inspector VMs. Auto-state derives from
  rendered-line count threshold (5 lines).
- Track-inspector VM exposes `compare_id3_enabled` and
  `musicbrainz_enabled` as functions of `is_downloaded`.
- Add `SavedSearchEntry { id, query, label }` to source-list VM.

### Shells

- Retire the standalone Search screen. Its inspector helpers feed
  the unified content/detail shells.
- `ContentList` shell consumes a filter-aware page VM that already
  knows how to project library rows; add a Musicindex-row projection
  pathway behind the `Index` / `All` filters.
- `Detail` shell renders the new `SearchResultsInspector` view in
  addition to existing artist/feed/track inspectors. Breadcrumb chrome
  lives in the frame shell composite (ADR 0046 task 006); breadcrumb
  data comes from frame nav state.

### Frame nav state

- Search submit pushes a `Search(query)` entry onto the active
  `Detail` frame's nav stack.
- Drilling into a result pushes the entity (artist/feed/track) entry.
- Back/forward and breadcrumbs both consume the same nav state.

### Filter wiring

- Filter chip dispatches a `SetFrameFilter(frame_id, ContentFilter)`
  command targeted at a specific frame VM. The frame VM rebuilds
  visible rows from cached query results without refetching when
  possible.
- No global filter store — each frame is the source of truth for its
  own filter state.

### Tabbed inspector wiring

- `SearchResultsInspector` tab change dispatches
  `SetSearchResultsTab(frame_id, tab)`. Tab and filter state are
  independent; switching tabs preserves filter.
- Per-tab paged windows can be active simultaneously (ADR 0041).

### Saved-search wiring

- Source-list click on a saved search dispatches
  `OpenSavedSearch(saved_search_id)`, which opens or focuses a
  `Detail` frame configured with `SearchResultsInspectorPageVm`
  seeded by the saved query.

### Library-extra panel guards

- Track inspector renders `compare_id3_panel` and `musicbrainz_panel`
  only when `inspector_expanded_panels` contains the kind AND the
  track is downloaded. Clicking the (enabled) control toggles
  membership.
- Extra-field render helpers in `track_detail_metadata.rs` accept the
  expanded-panels set and gate their output.

## Backward Compatibility

- DB schema unchanged.
- Existing Library / Search VM tests retire alongside their screens.
- Toolbar search-submit handler now opens a `Detail` frame instead of
  switching tabs.

## Risks

- **Discoverability**: users accustomed to the Search tab may not
  realize results moved to the inspector. Mitigation: animated
  open-from-toolbar transition; breadcrumb shows the query
  prominently; settings tip on first use.
- **Filter ambiguity**: `All`/`Library`/`Index` filtering applies to
  both `ContentList` and search-results inspector. Filter state must
  be visibly consistent in both surfaces to avoid confusion.
- **Inspector crowding**: a tall search-results inspector may push
  artist/feed/track detail off-screen. Mitigation: paged result list
  per ADR 0041; expandable result groups.
- **Disabled-control confusion**: greyed Compare ID3 / MusicBrainz
  on non-downloaded items must communicate the gating clearly.
  Mitigation: HIG-compliant disabled state + tooltip.

## Test Strategy

- Architecture tests:
  - No `src/search.rs` screen module after retirement.
  - Toolbar search-submit dispatches into `Detail` frame, not a tab
    swap.
  - Filter chips render inside each content-showing frame's chrome,
    not on the global toolbar.
  - No global `ContentFilter` store; filter state lives on frame VMs.
  - `SearchResultsInspector` renders the three-tab contract; tab and
    filter state are independent.
  - Track inspector "Compare ID3" / "MusicBrainz" controls are
    disabled when `is_downloaded = false`.
  - Library-extra fields render only when the corresponding panel is
    expanded.
  - Saved searches in `SourceList` dispatch `OpenSavedSearch`, not a
    legacy search-tab swap.
- Unit tests:
  - Filter chip changes update visible rows on the content VM
    without refetching.
  - Breadcrumb projection from frame nav state covers single-entity,
    multi-entity, and deep-drill cases.
  - Inspector-expanded-panel toggling preserves selection.
- Visual smoke (light + dark):
  - Library row selection while a search result inspector is open.
  - Search submit → breadcrumb appears → drill into a result.
  - Filter chips applied to library content and to search results.
  - Disabled Compare ID3 / MusicBrainz on a non-downloaded track.
  - Description collapse/expand on feed and track inspectors.

## Phases

### Phase A - Concept ratification

- Completed 2026-05-14: promoted this doc to
  `docs/adr/0047-library-search-unification.md`.
- Completed 2026-05-14: resolved Open Questions.
- Completed 2026-05-14: authored phase plan + implementation task
  packets.

### Phase B - View-model groundwork

- Add `ContentFilter` enum + filter state on content VM.
- Add `SearchResultsInspectorPageVm`.
- Add `inspector_expanded_panels` and `description_collapsed` to
  inspector VMs.
- Add `compare_id3_enabled` / `musicbrainz_enabled` predicates.

### Phase C - Inspector rewiring

- Render `Compare ID3` / `MusicBrainz` as disabled when not
  downloaded.
- Gate Library-extra fields behind expanded-panel membership.
- Add disclosure to Description section.

### Phase D - Per-frame filter chips

- Add `ContentFilter` enum and `FilterChipStripDisplay` contract.
- Add `filter_state` to every content-showing frame VM.
- Render the chip strip inside frame chrome via a new shared
  composite consumed by `frame_shell` (or a sibling composite).
- Implement narrow-width pull-down collapse.
- Wire `SetFrameFilter` command per frame.
- Retire `GlobalSearchScope`.

### Phase E - Search-results inspector (tabbed)

- Render search submit into a `Detail` frame.
- Implement tabbed inspector (`Artists` | `Feeds` | `Tracks`).
- Implement breadcrumb chrome (middle-ellipsis truncation) on the
  frame shell.
- Wire drill-down through frame nav state.
- Surface saved searches as `SourceList` entries that open the same
  inspector via `OpenSavedSearch`.

### Phase F - Retire Search screen

- Delete `src/search.rs` screen module after all callers route
  through frame-based content/detail.
- Update architecture guards.

### Phase G - Cleanup + visual proof

- Architecture guards for every Phase B-F invariant.
- Visual proof checklist (light + dark) per ADR 0044 / 0046 pattern.

## Resolved Decisions

The original seven Open Questions are resolved as follows. Resolutions
are folded into the §Target State, §Apple HIG Considerations, and
§Architectural Sketch sections above.

1. **Filter persistence — per frame.** Each content-showing frame
   owns its own filter chip strip and `filter_state`. No global
   filter store. Two frames may carry different filter states.
2. **Search-results inspector layout — tabbed.** Three tabs:
   `Artists`, `Feeds`, `Tracks`. Tab state independent of filter
   state; both persist for the inspector instance lifetime.
3. **Description collapse default — auto-collapse at >5 lines.**
   Default collapsed when rendered description exceeds five lines at
   current frame width; default expanded otherwise. User toggle
   overrides the auto-default for the inspector instance.
4. **Breadcrumb truncation — middle ellipsis.** Leftmost (origin)
   and rightmost (current) breadcrumb segments stay visible; middle
   segments collapse to `…` at narrow widths.
5. **Saved searches — yes.** Surface as `SourceList` entries beneath
   playlists; activation opens the same tabbed
   `SearchResultsInspector` as an ad-hoc query.
6. **Empty-filter behavior — empty-state notice.** Frame renders an
   HIG content-unavailable view naming the active filter and offering
   "Clear filter".
7. **Filter chip width — narrow collapse to pull-down.** Chip strip
   collapses to a single pull-down whose trigger label shows the
   active filter when frame width is below a breakpoint.

## References

- ADR 0033 — HIG UI architecture governance
- ADR 0038 — presentation contract enforcement
- ADR 0040 — async VM runtime
- ADR 0041 — windowed paged view models
- ADR 0043 — top toolbar global search
- ADR 0046 — workspace frame architecture
- `docs/plans/unify-discover-library-views.md` — entity-render
  unification (companion plan)
- `docs/plans/discovery-library-ui-fixes.md` — earlier UI parity work
