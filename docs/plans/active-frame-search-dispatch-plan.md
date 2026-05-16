# Active-Frame Search Dispatch

Status: Proposed - 2026-05-15.

Refines ADR 0047 search-submit behavior. Builds on ADR 0046 workspace
frame architecture. No new ADR required.

## Context

ADR 0046 + 0047 phases A–G shipped. `src/search.rs`, `src/search/`,
`src/ui/shells/discover/`, `src/ui/shells/feed.rs`,
`src/view_models/search.rs` are deleted (~4,500 LOC). All five gates
green.

Toolbar global search now routes submit through
`WorkspaceLayout::open_search_results_frame(query)`, which opens or
focuses a Detail frame whose `FrameNavigationEntry::Search(query)` is
rendered by `SearchResultsInspectorPageVm` +
`render_search_results_inspector`. Result tabs render empty because no
result loader is wired.

User reframed the model: **search is per-frame**. The toolbar input
dispatches the query to the *currently active* frame, which interprets
it in context. An explicit modifier opens a new Detail frame instead of
mutating the focused one.

This plan replaces "global search → new Detail frame" with
"active-frame search → frame-owned interpretation, with an opt-in
'open in new frame'."

## Goal

- Toolbar submit applies the query to whichever frame currently holds
  focus, using the page VM that owns that frame.
- Cmd/Ctrl+Enter (or a secondary toolbar button) opens a new Detail
  search-results frame instead of mutating the focused one.
- Placeholder text reflects the focused frame's scope so the user can
  see what the next submit will affect.

## Non-goals

- Result loading for `SearchResultsInspectorPageVm`. Empty inspector
  remains v1 behaviour until a separate plan wires
  `library_service::search_library_tracks` and `api::Client::search`.
- Saved searches end-to-end (no DB table, no save UI, no source-list
  render). Separate plan.
- Settings-field search. `ContentList` under Settings mount is a no-op
  for v1.
- Hotkey customisation. Cmd/Ctrl+Enter is the only modifier for v1.

## Current state

- Toolbar input always routes to `open_search_results_frame(query)`,
  which adds/focuses a Detail frame regardless of which frame had
  focus.
- `SearchResultsInspectorPageVm::new(query)` renders the inspector
  shell with empty tabs.
- Page VMs already own per-VM filter or query state in several cases
  (`ContentListPageVm` has a chip-based content filter;
  `SearchResultsInspectorPageVm` has `set_query`). No common dispatch
  contract reads from the workspace VM.

## Target state

### Per-frame search contract

Each `WorkspaceFrameKind` defines how it consumes a query when focused:

| Frame kind | When focused, submit means… |
|---|---|
| `SourceList` | filter sidebar items (artists, albums, playlists, saved searches) by name substring |
| `ContentList` (Library mount) | apply query as a track/feed substring filter on the visible library rows; works alongside the All/Library/Index chip strip |
| `ContentList` (Settings mount) | no-op for v1 (could be settings-field search later) |
| `Detail` with `Search(_)` | update the inspector query in place; re-run result loader |
| `Detail` with `TrackDetail(id)` / `FeedDetail(id)` / `AlbumDetail(id)` / `ArtistDetail(_)` / `PlaylistDetail(id)` | filter the tracks rendered inside the detail surface by query substring |
| `QueueNowPlaying` | filter queue rows by query substring |

The behaviour is owned by the frame's page VM
(`ContentListPageVm`, `SearchResultsInspectorPageVm`,
`QueueNowPlayingVm`, etc.). The toolbar is a dumb dispatcher.

### "Open in new frame" modifier

Two affordances open a new Detail frame instead of mutating the focused
one:

1. **Cmd/Ctrl + Enter** when the toolbar input is focused.
2. **A toolbar split-button**: primary "Search" submits to the active
   frame; a secondary chevron menu offers "Search in new frame…".

Behaviour: build a new Detail frame with
`FrameNavigationEntry::Search(query)`, focus it. Same path as today's
`open_search_results_frame`, just gated behind the modifier so it isn't
the default.

### Focus + toolbar coupling

Toolbar input placeholder changes with the focused frame so the user
sees what scope the query will apply to:

- SourceList focused → `"Filter sidebar…"`
- ContentList focused → `"Search library…"` (or `"Search settings…"`
  if Settings mount)
- Detail Search frame → `"Refine search…"`
- Detail entity frame → `"Filter tracks…"`
- QueueNowPlaying → `"Filter queue…"`

Empty-query submit clears the focused frame's filter (returns to its
default display).

## Required wiring

### View-model contracts

Add to `src/view_models/workspace.rs`:

```rust
pub(crate) enum FrameSearchScope {
    Sidebar,
    LibraryRows,
    SettingsRows, // no-op v1
    QueueRows,
    InspectorQuery,
    DetailTracks,
}

pub(crate) struct FrameSearchDescriptor {
    pub frame_id: WorkspaceFrameId,
    pub kind: WorkspaceFrameKind,
    pub nav: FrameNavigationEntry,
    pub scope: FrameSearchScope,
    pub placeholder: &'static str,
}

impl WorkspaceLayout {
    pub(crate) fn focused_search_descriptor(&self) -> Option<FrameSearchDescriptor> { … }
}
```

The descriptor is what the toolbar reads to set its placeholder and
route submit. It's a pure projection — GPUI-free.

### Page-VM filter methods

Each page VM that supports a search scope grows a typed mutator:

- `ContentListPageVm::set_text_filter(Option<String>)` —
  `src/view_models/library.rs`
- `SearchResultsInspectorPageVm::set_query(String)` already exists; add
  `clear_query()`.
- `QueueNowPlayingVm::set_text_filter(Option<String>)` —
  `src/view_models/queue_now_playing.rs`
- New `SourceListPageVm::set_text_filter(Option<String>)` if the
  sidebar filtering doesn't already live somewhere — verify before
  adding.

For Detail entity inspectors (TrackDetail / FeedDetail / AlbumDetail /
ArtistDetail / PlaylistDetail), add `set_text_filter(Option<String>)`
to the corresponding `*Vm` so the surface narrows its track rows.

### Dispatcher

`src/app.rs::submit_global_search`:

```rust
fn submit_global_search(&mut self, modifier: SubmitModifier, cx: &mut Context<Self>) {
    let query = self.global_search_input.read(cx).value().to_string();
    let descriptor = self.workspace_layout.focused_search_descriptor();
    match modifier {
        SubmitModifier::ActiveFrame => self.dispatch_active_frame_search(descriptor, query, cx),
        SubmitModifier::NewFrame => {
            let _ = self.workspace_layout.open_search_results_frame(query);
            cx.notify();
        }
    }
}
```

`dispatch_active_frame_search` matches on `descriptor.scope` and
forwards into the right page VM via the existing `library.update(...)`
/ `workspace_layout.frame_nav_mut(...)` paths. Empty query → clear
filter on the target VM.

### Toolbar shape

`src/view_models/app_toolbar.rs`:

- `AppToolbarVm` exposes `placeholder()` driven by the workspace
  `focused_search_descriptor()`.
- Add a `secondary_submit` display field: an icon button or a small
  menu that fires the "new frame" modifier.

`src/app.rs` toolbar render block (search for where `global_search` is
rendered):

- Bind placeholder to the descriptor.
- On `PressEnter`, read `cx.modifiers()` (or whatever GPUI exposes for
  key modifiers) — if Cmd/Ctrl is held, use
  `SubmitModifier::NewFrame`, else `SubmitModifier::ActiveFrame`.
- The secondary button always dispatches `NewFrame`.

### Search results result loading (separate plan)

The "Search in new frame" path opens a Detail frame with
`SearchResultsInspectorPageVm::new(query)` — same as today. **It still
won't show results** until a loader is wired (DB query via
`library_service::search_library_tracks` + optional
`api::Client::search` for Index results). That loader is a follow-up
task, not part of this plan:

- Phase α — wire `search_library_tracks` into Library origin tab.
- Phase β — wire `api::Client::search` into Index origin tab.
- Phase γ — async dispatch + skeleton rows during fetch.

Keep this plan scoped to the per-frame dispatcher. Empty inspector is
acceptable v1 — the user explicitly accepted this when scoping the
plan.

## Critical files

| File | Change |
| --- | --- |
| `src/view_models/workspace.rs` | add `FrameSearchScope`, `FrameSearchDescriptor`, `focused_search_descriptor()` |
| `src/view_models/library.rs` | `ContentListPageVm::set_text_filter`; thread into source-list filter projection |
| `src/view_models/queue_now_playing.rs` | `set_text_filter` |
| `src/view_models/search_results.rs` | `clear_query()` (`set_query` exists) |
| `src/view_models/{track_detail,feed,artist_detail,playlist_detail,paged_feed_detail,paged_playlist_detail}.rs` | `set_text_filter` on entity-detail VMs |
| `src/view_models/app_toolbar.rs` | placeholder + secondary submit display |
| `src/app.rs::submit_global_search` | dispatch by `FrameSearchScope`; modifier handling |
| `src/app.rs` toolbar render | bind placeholder, wire modifier, render secondary button |
| `tests/architecture_tests.rs` | guard: toolbar reads descriptor from workspace VM, never global state |

## Reusable utilities

- `WorkspaceLayout::open_search_results_frame(query)` — already
  implemented (added during ADR 0047 Phase F cleanup); reuse verbatim
  for the "new frame" path.
- `WorkspaceLayout::frame_nav` / `frame_nav_mut` / `focused_frame_id`
  / `focused_frame` — already in place.
- `FrameNavigationEntry` variants — already cover the entity-detail
  kinds we need to switch on.
- `library_service::search_library_tracks` — DB-backed substring
  search; reuse for Library and detail-track filters.

## Phases

1. **VM contracts.** Land `FrameSearchScope`/`FrameSearchDescriptor` +
   `focused_search_descriptor()`. Add `set_text_filter` to the page
   VMs that need it. No UI change; covered by unit tests on the
   workspace VM and each page VM.
2. **Toolbar bind.** Make the toolbar placeholder and submit modifier
   read the descriptor and dispatch. Implement
   `dispatch_active_frame_search` in `app.rs`. Wire Cmd/Ctrl+Enter
   modifier.
3. **Secondary submit button.** Add the "open in new frame" affordance
   to the toolbar shell. Visual smoke once the operator runs the
   build.
4. **Architecture guard.** Forbid global search input from directly
   mutating page-VM state outside the descriptor path.

## Risks

- **Focus ambiguity.** GPUI's "focused frame" notion is internal to the
  workspace layout. If the toolbar input itself is focused, the
  *previous* focused frame is what should govern submit semantics.
  Mitigation: track last-focused frame separately from the toolbar
  input's focus.
- **Empty-query semantics.** Empty submit clearing the filter could
  surprise users who expect Enter to be a no-op. Mitigation: only
  clear when the input is non-empty-then-emptied; treat first empty
  submit as no-op.
- **Modifier discovery.** Cmd/Ctrl+Enter is invisible. Mitigation: the
  secondary toolbar button surfaces the "open in new frame" path
  visually so the keyboard modifier is an optional accelerator, not
  the only way.
- **Descriptor drift.** The `FrameSearchDescriptor` couples the
  workspace VM to specific page-VM scopes. Mitigation: keep the
  descriptor as an opaque `(scope, placeholder)` projection; the
  dispatcher does the matching, not the workspace VM.

## Test strategy

- Unit tests on `WorkspaceLayout::focused_search_descriptor()` covering
  each frame kind + nav-entry combination, including empty-layout and
  no-focus cases.
- Unit tests on each page VM's `set_text_filter(None|Some)` for
  default-state and filtered-state transitions.
- Architecture guard test in `tests/architecture_tests.rs` asserting
  `app.rs::submit_global_search` only mutates page VMs through the
  descriptor-driven dispatcher (no direct
  `library.update(|l, _| l.set_filter(...))` calls bypassing the
  descriptor).
- Cargo gate sweep after each phase: `cargo fmt --check`, `cargo
  build`, `cargo test --lib`, `cargo test --test architecture_tests`,
  `cargo clippy -- -D warnings`.

## Rollback strategy

Each phase is additive. To roll back:

- Revert the dispatcher change in `app.rs::submit_global_search` to
  the current always-new-frame path.
- Leave `FrameSearchScope` / `FrameSearchDescriptor` in place if
  Phase 1 landed alone — they're inert without the dispatcher reading
  them.
- Remove the secondary submit button render block in `app.rs`.
- Keep the Cmd/Ctrl+Enter modifier wiring disabled but in place if
  Phase 2 partially shipped.

No DB schema, no config schema, no service or playback path touched.

## Open questions

- Should empty-query submit be a no-op or clear-filter? Default in
  this plan is clear-filter on second submit only.
- Should the secondary "open in new frame" button live in the toolbar
  permanently, or only when the focused frame has a non-trivial
  filter scope?
- Should `SourceList` frame sidebar filtering be added in v1 or
  deferred? Plan currently lists it but `SourceListPageVm` may not
  exist yet.

## Verification

In-sandbox gates (after each phase):

```bash
cargo fmt -- --check
cargo build 2>&1 | tail -5
cargo test --lib 2>&1 | tail -5
cargo test --test architecture_tests 2>&1 | tail -5
cargo clippy -- -D warnings 2>&1 | tail -5
```

End-to-end (operator, post-phase-3):

1. Focus Library content frame. Type `heycitizen` in toolbar. Submit.
   Library rows narrow to the matching tracks/feeds.
2. Cmd/Ctrl+Enter the same query. A new Detail frame opens showing
   the `SearchResultsInspector` (empty until result-loader plan
   ships). Focus moves to the new frame.
3. Focus the Queue frame. Type `palm`. Queue rows narrow.
4. Click a feed in the library tree → Detail frame mounts. Type a
   track-title substring in the toolbar. The feed-detail tracks
   narrow.
5. Empty the toolbar and re-submit → focused frame's filter clears.

Sandbox cannot run the GUI; the five cargo gates are the in-sandbox
proof and the operator handles visual confirmation.
