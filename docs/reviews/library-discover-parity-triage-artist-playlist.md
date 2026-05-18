# Library / Index Artist + Playlist Detail Parity Triage

## Status

Triage - 2026-05-17.

---

# Artist detail

## Surfaces compared

- **Library:** `src/ui/shells/library/detail.rs`, `src/ui/shells/library/feed_list.rs`, `src/ui/shells/artist.rs`, `src/view_models/library.rs`, `src/view_models/artist_detail.rs`, `src/views.rs`, `src/sources.rs`, `src/library/app_impl.rs`, `src/db.rs`
- **Index:** `src/app.rs`, `src/app/search_dispatch.rs`, `src/ui/shells/search_results_inspector.rs`, `src/view_models/search_results/index_detail.rs`, `src/view_models/search_results/mod.rs`, `src/view_models/search_results/results.rs` — no dedicated Index artist detail surface; `IndexArtistDetail` renders a scoped Index feed result list, not an artist detail page.

## Surface inventory

| Field rendered          | Library shows? | Index shows? | Same composite? | Notes |
| ----------------------- | -------------- | ------------ | --------------- | ----- |
| Artist title / name | yes | no dedicated detail | no | Library title comes from `ArtistDetailPageVm`; Index artist nav scopes the search inspector to Index feeds. |
| Artist subtitle | no | no | no | Page contract supports it, but Library passes `None`. |
| Artist image | yes | no dedicated detail | no | Library loads `ArtistView::image_url` into the artist shell image slot. |
| Albums count | yes | no dedicated detail | no | Library fact row from local distinct feed count. |
| Tracks count | yes | no dedicated detail | no | Library fact row from local track count. |
| Downloaded count | yes, conditional | no | no | Library-only local file count. |
| Source subject count | yes, conditional | no | no | Library shows a count when multiple bound source subjects exist. |
| Sort name | yes, conditional | no | no | Library shows when exactly one bound source fact contributes it. |
| Area | yes, conditional | no | no | Library shows when exactly one bound source fact contributes it. |
| Active years | yes, conditional | no | no | Library renders `begin_year` / `end_year` as `Active`. |
| Website | yes, conditional | no | no | Library shows source fact website URL. |
| Aliases | yes, conditional | no | no | Library joins aliases into one detail row. |
| Linked feeds / releases list | yes | no dedicated detail | no | Library feed section lists local feeds under the artist. |
| Linked feed thumbnail | yes, conditional | no dedicated detail | no | Library feed rows use the feed/track image fallback. |
| Linked feed track count | yes | no dedicated detail | no | Library feed rows show per-feed track count. |
| Description / biography / annotation | no | no | n/a | Not present in `ArtistView` or Library artist detail rows. |
| Explicit state | no | no | n/a | No per-artist explicit field exists in `ArtistView`. |
| Contributor list | no | no | n/a | Artist-level contributor/person merge remains out of scope. |
| External identifiers (MBID, etc.) | no | no | n/a | Source ids are persisted for explicit artist facts but are not rendered on Library artist detail. |

## Gap analysis

### Field: artist header identity

- Library renderer: `src/ui/shells/library/detail.rs:59`, `src/ui/shells/library/feed_list.rs:32`, `src/ui/shells/artist.rs:61`
- Index renderer: no artist detail renderer; `src/app.rs:875` routes `IndexArtistDetail` through `render_search_results_inspector`, and `src/app.rs:890` scopes that inspector to `SearchResultsTab::Feeds`.
- Library VM source: `LibraryArtistDetailVm::page`, `src/view_models/library.rs:2488`; `ArtistDetailPageVm::title`, `src/view_models/artist_detail.rs:12`
- Index VM source: no `IndexDetailKind::Artist`; `src/view_models/search_results/index_detail.rs:20`
- Local persistence today: source-scoped artist facts include `name` and `image_url`, `src/db.rs:2595`; name-derived artists are `ArtistRef::LocalArtistName`, `src/views.rs:408`
- Hydration path: `LibraryApp::select_artist` builds `LibraryArtistDetail`, `src/library/app_impl.rs:1212`; local source hydration is `local_artist_view_from_tracks`, `src/sources.rs:130`
- Routing: loading-shape
- Rationale: Library has a true artist detail header backed by a Library artist page VM, while Index artist activation only changes the search-results inspector scope to Index feeds. Closing parity requires an Index artist detail loading/rendering shape before individual header fields can be compared.

### Field: local artist summary facts

- Library renderer: `src/ui/shells/artist.rs:45`, `src/ui/shells/artist.rs:69`
- Index renderer: not rendered as artist detail facts; `src/ui/shells/search_results_inspector.rs:327` renders result windows, not artist detail rows.
- Library VM source: `LibraryArtistDetailVm::detail_rows`, `src/view_models/library.rs:2449`; album, track, and downloaded counts at `src/view_models/library.rs:2422`, `src/view_models/library.rs:2432`, `src/view_models/library.rs:2438`
- Index VM source: artist search rows only expose `ArtistResultDisplay::secondary_text`, `src/view_models/search_results/results.rs:21`
- Local persistence today: local tracks provide `feed_id`, `duration_seconds`, and `local_path` context used for counts, `src/db.rs:21`
- Hydration path: `LibraryApp::select_artist` gathers matching local tracks, `src/library/app_impl.rs:1212`; `LocalSource::fetch_artist` filters local rows by artist names, `src/sources.rs:152`
- Routing: loading-shape
- Rationale: The data exists locally and Library renders it as detail facts. Index has only derived search-result secondary text, so this is primarily a missing Index artist-detail shape rather than a missing local persistence field.

### Field: source fact rows (sort name, area, active years, website, aliases)

- Library renderer: `src/ui/shells/artist.rs:45`, `src/ui/shells/artist.rs:69`
- Index renderer: not rendered; `src/view_models/search_results/index_detail.rs:22` has only `Feed` and `Track` detail kinds.
- Library VM source: `LibraryArtistDetailVm::push_artist_source_rows`, `src/view_models/library.rs:2465`; single-source fields are pushed at `src/view_models/library.rs:2476`
- Index VM source: no artist detail VM; Index search candidates contain only `name`, counts, and thumbnail, `src/app/search_dispatch.rs:1063`
- Local persistence today: `artist_source_facts.sort_name`, `image_url`, `website_url`, `aliases_json`, `area`, `begin_year`, and `end_year`, `src/db.rs:2595`
- Hydration path: source facts are collected for bound tracks, `src/sources.rs:245`; one fact enriches the `ArtistView`, `src/views.rs:449`
- Routing: loading-shape
- Rationale: Library can already render these fields when source bindings yield exactly one subject. Index lacks a dedicated artist detail page and its artist candidates collapse to result-row data, so parity needs a remote artist detail loading shape.

### Field: linked releases / feeds

- Library renderer: `src/ui/shells/library/feed_list.rs:42`; row title, thumbnail, and track count render at `src/ui/shells/library/feed_list.rs:81`
- Index renderer: `IndexArtistDetail` renders the generic search-results feed rows, `src/app.rs:890`; result rows render label, secondary text, origin, and kind at `src/ui/shells/search_results_inspector.rs:457`
- Library VM source: `LibraryArtistDetailVm::feed_summaries`, `src/view_models/library.rs:2503`; row display fields at `src/view_models/library.rs:2362`
- Index VM source: Index artist candidates are merged from feed/track searches, `src/app/search_dispatch.rs:1015`; candidate feed and track counts are computed at `src/app/search_dispatch.rs:1094`
- Local persistence today: local `tracks.feed_id`, `feed_title`, and `album_image_href`, `src/db.rs:21`
- Hydration path: Library collects tracks in `select_artist`, `src/library/app_impl.rs:1212`; Index artist activation pushes `FrameNavigationEntry::IndexArtistDetail`, `src/app/search_dispatch.rs:269`
- Routing: loading-shape
- Rationale: Both sides can show related feed-like rows, but only Library presents them inside an artist detail shell. The Index side currently uses a scoped result list, so the parity gap is the absence of an artist detail route/composite, not the absence of feed search rows.

### Field: description / biography / annotation

- Library renderer: not rendered; artist detail rows are only `detail_rows` and feed section, `src/ui/shells/artist.rs:45`
- Index renderer: not rendered; no artist detail renderer exists, `src/view_models/search_results/index_detail.rs:22`
- Library VM source: no `ArtistView` description field, `src/views.rs:81`
- Index VM source: `IndexArtistCandidate` has no description field, `src/app/search_dispatch.rs:1063`
- Local persistence today: not persisted in artist source facts; schema has no artist description column, `src/db.rs:2595`
- Hydration path: n/a; Library artist hydration applies scalar source facts only, `src/views.rs:457`
- Routing: persistence
- Rationale: The named deferred field is absent from both the local artist view and the source-fact schema. Adding it would require a source fact/persistence decision before renderer parity work.

### Field: explicit state

- Library renderer: not rendered; artist detail shell renders header plus detail rows, `src/ui/shells/artist.rs:57`
- Index renderer: not rendered; no artist detail kind exists, `src/view_models/search_results/index_detail.rs:22`
- Library VM source: no `ArtistView::explicit`; `src/views.rs:81`
- Index VM source: Index artist candidates contain no explicit field, `src/app/search_dispatch.rs:1063`
- Local persistence today: not persisted in `artist_source_facts`, `src/db.rs:2595`
- Hydration path: n/a; `apply_single_artist_source_fact` applies image, sort name, area, years, website, aliases, and tags, `src/views.rs:457`
- Routing: intentional asymmetry
- Rationale: The current data model has explicit state at feed and track level, not artist level. Treating explicitness as an artist property would need product semantics first.

### Field: external identifiers

- Library renderer: not rendered; source fact rows stop at aliases in `push_artist_source_rows`, `src/view_models/library.rs:2476`
- Index renderer: not rendered; no artist detail renderer exists, `src/ui/shells/search_results_inspector.rs:155`
- Library VM source: `ArtistView::identity` can hold source ids, `src/views.rs:88`
- Index VM source: no artist detail VM; `IndexDetailDisplay` has only kind, id, title, secondary text, and optional feed detail, `src/view_models/search_results/index_detail.rs:40`
- Local persistence today: `artist_source_ids` table persists source ids, `src/db.rs:2629`
- Hydration path: `apply_single_artist_source_fact` maps source ids into `EntityIdentityLinks`, `src/views.rs:476`
- Routing: loading-shape
- Rationale: Local source ids can be hydrated for explicit artist facts, but neither Library nor Index artist detail renders them today. The first gap is display/loading shape, with identity-reconciliation risk only for name-derived multi-subject artists.

## Open questions

- Should `FrameNavigationEntry::IndexArtistDetail` become a real remote artist detail page, or is the current scoped Index feed result list the intended behavior? Evidence: `src/app.rs:875` renders `SearchResultsHeaderMode::Scoped` with `SearchResultsTab::Feeds`.
- How should name-derived Library artists with multiple bound source subjects expose source ids without implying a canonical artist merge? ADR 0045 forbids collapsing subjects into one canonical artist, `docs/adr/0045-track-artist-binding.md:42`.
- If artist contributors are desired, which future ADR owns durable person keys and merge policy? ADR 0029 explicitly defers global person identity, `docs/adr/0029-artist-person-identity-persistence.md:160`.

---

# Playlist detail

## Surfaces compared

- **Library:** `src/ui/shells/library/detail.rs`, `src/ui/shells/library/playlist_detail.rs`, `src/ui/shells/playlist.rs`, `src/view_models/library.rs`, `src/view_models/playlist_detail.rs`, `src/db.rs`, `src/library/app_impl.rs`
- **Index:** `src/view_models/search_results/index_detail.rs`, `src/view_models/search_results/mod.rs`, `src/ui/shells/search_results_inspector.rs`, `src/app/search_dispatch.rs` — no dedicated Index playlist result or detail surface.

## Surface inventory

| Field rendered          | Library shows? | Index shows? | Same composite? | Notes |
| ----------------------- | -------------- | ------------ | --------------- | ----- |
| Playlist title / name | yes | no dedicated detail | no | Library header title uses `PlaylistDetailHeaderDisplay`. |
| Track count | yes | no dedicated detail | no | Library detail grid always includes `Tracks`. |
| Total duration | yes, conditional | no | no | Library detail grid includes `Duration` when known. |
| Rename action | yes | no | no | Library-only local playlist control. |
| Delete action | yes | no | no | Library-only local playlist control. |
| Empty-state message | yes, conditional | no | no | Library shows an empty playlist message when no rows are present. |
| Track row position | yes | no | no | Library row body renders 1-indexed position. |
| Track row thumbnail | yes, conditional | no | no | Library uses track image then album image fallback. |
| Track row title | yes | no | no | Library row body renders title with fallback. |
| Track row artist | yes | no | no | Library row body renders artist with fallback. |
| Track availability | yes, conditional | no | no | Library rows can show `Unavailable`. |
| Track duration | yes, conditional | no | no | Library row body renders track duration. |
| Play action | yes | no | no | Library row control is enabled only for playable local files. |
| Reorder controls | yes | no | no | Library supports drag and menu reordering. |
| Remove action | yes | no | no | Library supports removing tracks from local playlists. |
| Release date | no | no | n/a | No playlist release-date field exists locally. |
| Created date | no | no | n/a | Persisted locally but not rendered. |
| Modified date | no | no | n/a | Persisted locally but not rendered. |
| Language | no | no | n/a | No playlist language field exists locally. |
| Explicit state | no | no | n/a | No playlist explicit field exists locally. |
| Description / annotation | no | no | n/a | Persisted locally but not rendered in playlist detail. |

## Gap analysis

### Field: playlist header and local actions

- Library renderer: `src/ui/shells/library/detail.rs:75`; shared playlist shell header and actions render at `src/ui/shells/playlist.rs:144` and `src/ui/shells/playlist.rs:223`
- Index renderer: not rendered; no `IndexDetailKind::Playlist`, `src/view_models/search_results/index_detail.rs:22`
- Library VM source: `PlaylistDetailVm::header_display`, `src/view_models/library.rs:2951`; `PlaylistDetailVm::actions_display`, `src/view_models/library.rs:3015`
- Index VM source: `IndexSearchResultRows` only has artists, feeds, and tracks, `src/view_models/search_results/index_detail.rs:11`
- Local persistence today: `playlists.name` is persisted, `src/db.rs:2417`
- Hydration path: `select_playlist_with_history` loads the selected playlist and tracks, `src/library/app_impl.rs:627`
- Routing: loading-shape
- Rationale: Library playlist detail is a local management surface with rename/delete controls. Index has neither playlist result rows nor a playlist detail VM, so parity would first need a remote playlist entity/loading shape.

### Field: playlist summary facts

- Library renderer: `src/ui/shells/playlist.rs:160`
- Index renderer: not rendered; `render_index_detail_display` is limited to feed/track fallback detail, `src/ui/shells/search_results_inspector.rs:155`
- Library VM source: `PlaylistDetailVm::detail_rows`, `src/view_models/library.rs:2993`
- Index VM source: no playlist branch in `IndexDetailDisplay`, `src/view_models/search_results/index_detail.rs:40`
- Local persistence today: `playlists_list` reads track count through `COUNT(pt.position)`, `src/db.rs:1732`; row duration comes from `TrackRow::duration_seconds`, `src/db.rs:32`
- Hydration path: playlist tracks are loaded through `query_service().playlist_tracks`, `src/library/app_impl.rs:641`
- Routing: loading-shape
- Rationale: Library can compute track count and total duration from local playlist membership. Index has no playlist surface or playlist membership model to map these facts onto.

### Field: playlist track list rows

- Library renderer: row shell renders position, title, artist, availability, thumbnail, duration, play, reorder, and remove controls at `src/ui/shells/playlist.rs:427`; row body text renders at `src/ui/shells/playlist.rs:662`
- Index renderer: not rendered; search result rows support artists, feeds, and tracks only, `src/ui/shells/search_results_inspector.rs:336`
- Library VM source: `PlaylistTrackRowDisplay`, `src/view_models/library.rs:2720`; row display projection at `src/view_models/library.rs:2903`
- Index VM source: no playlist row/result type; `SearchResultsInspectorPageVm` stores artists, feeds, and tracks, `src/view_models/search_results/mod.rs:44`
- Local persistence today: `playlist_tracks` stores playlist membership and position, `src/db.rs:2425`
- Hydration path: `db::playlist_tracks` joins `playlist_tracks` to `tracks` and orders by position, `src/db.rs:1811`
- Routing: loading-shape
- Rationale: The local playlist detail row model is rich and command-oriented. There is no Index playlist entity, detail loader, or row model to compare against.

### Field: created / modified dates

- Library renderer: not rendered; playlist shell detail grid consumes only `page.detail_rows()`, `src/ui/shells/playlist.rs:160`
- Index renderer: not rendered; no Index playlist detail kind, `src/view_models/search_results/index_detail.rs:22`
- Library VM source: `PlaylistDetailVm::detail_rows` only emits `Tracks` and optional `Duration`, `src/view_models/library.rs:2993`
- Index VM source: n/a; `IndexSearchResultRows` has no playlists collection, `src/view_models/search_results/index_detail.rs:11`
- Local persistence today: `playlists.created_at` and `playlists.updated_at`, `src/db.rs:2421`; `Playlist` struct exposes both fields, `src/db.rs:49`
- Hydration path: `playlists_list` reads created and updated timestamps, `src/db.rs:1735`; Library selection reuses the loaded playlist, `src/library/app_impl.rs:642`
- Routing: loading-shape
- Rationale: The data exists locally but the detail VM intentionally omits it today. If dates are desired, this is a VM/rendering shape gap for Library and still blocked by the absence of any Index playlist detail surface.

### Field: description / annotation

- Library renderer: not rendered; playlist shell renders header, detail grid, actions, then track rows, `src/ui/shells/playlist.rs:144`
- Index renderer: not rendered; no Index playlist detail branch exists, `src/ui/shells/search_results_inspector.rs:598`
- Library VM source: `PlaylistDetailVm` owns a `db::Playlist` but `detail_rows` omits `description`, `src/view_models/library.rs:2924`
- Index VM source: n/a; no playlist detail VM, `src/view_models/search_results/index_detail.rs:40`
- Local persistence today: `playlists.description`, `src/db.rs:2420`; `playlist_set_description` updates it, `src/db.rs:1787`
- Hydration path: `playlists_list` reads `p.description`, `src/db.rs:1735`
- Routing: loading-shape
- Rationale: Description is already persisted and hydrated into `db::Playlist`, but the Library detail VM and shell do not expose it. The Index side remains unavailable until a playlist detail route exists.

### Field: language

- Library renderer: not rendered; `src/ui/shells/playlist.rs:160`
- Index renderer: not rendered; no Index playlist result or detail surface, `src/view_models/search_results/index_detail.rs:11`
- Library VM source: no `Playlist::language`; `src/db.rs:44`
- Index VM source: n/a; Index detail display has no playlist payload, `src/view_models/search_results/index_detail.rs:40`
- Local persistence today: not persisted; `playlists` schema has name, description, created_at, and updated_at only, `src/db.rs:2417`
- Hydration path: n/a
- Routing: persistence
- Rationale: Playlist language cannot be rendered until it has a source and persistence contract. This is not merely a renderer omission.

### Field: explicit state

- Library renderer: not rendered; `src/ui/shells/playlist.rs:144`
- Index renderer: not rendered; no Index playlist detail kind, `src/view_models/search_results/index_detail.rs:22`
- Library VM source: no `Playlist::explicit`; `src/db.rs:44`
- Index VM source: n/a; no playlist branch in search results VM, `src/view_models/search_results/mod.rs:44`
- Local persistence today: not persisted in `playlists`, `src/db.rs:2417`
- Hydration path: n/a
- Routing: persistence
- Rationale: Explicit state exists for feeds and tracks elsewhere, but no playlist-level explicit semantics are modeled locally. This needs a persistence/product contract before surface parity work.

### Field: release date

- Library renderer: not rendered; `src/ui/shells/playlist.rs:160`
- Index renderer: not rendered; no Index playlist detail surface, `src/view_models/search_results/index_detail.rs:22`
- Library VM source: no `Playlist::release_date`; `src/db.rs:44`
- Index VM source: n/a; no playlist detail payload, `src/view_models/search_results/index_detail.rs:40`
- Local persistence today: not persisted; `playlists` has `created_at` and `updated_at` but no release date, `src/db.rs:2421`
- Hydration path: n/a
- Routing: intentional asymmetry
- Rationale: A local playlist is not currently modeled as a release-like entity. Treating it as having a release date would be a semantic change rather than a straightforward parity fix.

## Open questions

- Does MusicIndex have or plan playlist results/detail entities? The current app search fetches feed and track rows only, `src/app/search_dispatch.rs:1015`.
- Should Library playlist detail render local `created_at`, `updated_at`, and `description` now that those fields are persisted and hydrated, or are they intentionally sidebar/debug-only metadata?

---

## Out of scope (handled by sibling triage tasks)

- Album / release detail → Task 001
- Track detail → Task 002
