# Library / Index Track-Detail Parity Triage

## Status

Triage - 2026-05-17.

## Surfaces compared

- **Library:** `src/ui/shells/library/track_detail.rs`, `src/ui/shells/library/track_detail_metadata.rs`, `src/ui/shells/library/track_detail_metadata_grid.rs`, `src/ui/shells/library/track_detail_metadata_cells.rs`, `src/ui/shells/library/track_detail_metadata_values.rs`, `src/ui/composites/track_detail_surface.rs`, `src/ui/composites/track_header.rs`, `src/view_models/library.rs`, `src/view_models/track_detail.rs`, `src/views.rs`, `src/metadata.rs`, `src/view_models/track_metadata_grid.rs`
- **Index:** `src/view_models/search_results/index_detail.rs`, `src/view_models/search_results/mod.rs`, `src/view_models/search_results/results.rs`, `src/ui/shells/search_results_inspector.rs`, `src/app/search_dispatch.rs`, `src/api.rs`

## Surface inventory

| Field rendered          | Library shows? | Index shows? | Same composite? | Notes |
| ----------------------- | -------------- | ------------ | --------------- | ----- |
| Entity kind badge | yes @src/ui/composites/track_header.rs:61 | yes @src/ui/shells/search_results_inspector.rs:173 | no | Library uses track header; Index uses `TagBadge` in fallback detail. |
| Hero artwork / thumbnail | yes @src/ui/composites/track_header.rs:105 | yes @src/ui/shells/search_results_inspector.rs:211 | no | Index detail does not pass `thumbnail_href` into the detail thumbnail. |
| Title | yes @src/ui/composites/track_header.rs:80 | yes @src/ui/shells/search_results_inspector.rs:180 | no | Both display a title. |
| Artist / secondary text | yes @src/ui/composites/track_header.rs:87 | yes @src/ui/shells/search_results_inspector.rs:184 | no | Index secondary may combine track artist, release artist, and feed title. |
| Release / album context | yes @src/view_models/track_detail.rs:248 | no @src/ui/shells/search_results_inspector.rs:214 | no | Library summary row; Index can only include feed title in secondary text. |
| Track number | yes @src/view_models/track_detail.rs:253 | no @src/ui/shells/search_results_inspector.rs:214 | no | Library summary row and metadata row. |
| Duration | yes @src/view_models/track_detail.rs:259 | no @src/ui/shells/search_results_inspector.rs:214 | no | Library summary row and metadata row. |
| Release date | yes @src/view_models/track_detail.rs:265 | no @src/ui/shells/search_results_inspector.rs:214 | no | Library core VM has the slot, but local track projection sets pub date to `None`. |
| Publisher | yes @src/view_models/track_detail.rs:271 | no @src/ui/shells/search_results_inspector.rs:214 | no | Library summary row when populated. |
| Description | yes @src/ui/shells/library/track_detail.rs:108 | no @src/ui/shells/search_results_inspector.rs:214 | no | Library uses disclosure panel. |
| External website action | yes @src/view_models/track_detail.rs:287 | no @src/ui/shells/search_results_inspector.rs:214 | no | Library identity action only. |
| Copy Nostr action | yes @src/view_models/track_detail.rs:304 | no @src/ui/shells/search_results_inspector.rs:214 | no | Library identity action only. |
| Subscribe / remove action | yes @src/ui/shells/library/track_detail_metadata.rs:140 | no @src/ui/shells/search_results_inspector.rs:214 | no | Library-only command. |
| Add to playlist action | yes @src/ui/shells/library/track_detail_metadata.rs:154 | no @src/ui/shells/search_results_inspector.rs:214 | no | Library-only command. |
| Compare ID3 action | yes @src/ui/shells/library/track_detail_metadata.rs:176 | no @src/ui/shells/search_results_inspector.rs:214 | no | Library-only local-file command. |
| MusicBrainz lookup action | yes @src/ui/shells/library/track_detail_metadata.rs:199 | no @src/ui/shells/search_results_inspector.rs:214 | no | Library-only enrichment command. |
| Staged ID3 edits controls | yes @src/ui/shells/library/track_detail_metadata.rs:222 | no @src/ui/shells/search_results_inspector.rs:214 | no | Library-only edit state. |
| Index Source row | no @src/ui/composites/track_detail_surface.rs:173 | yes @src/ui/shells/search_results_inspector.rs:219 | no | Index-only fallback metadata row. |
| Index ID row | no @src/ui/composites/track_detail_surface.rs:173 | yes @src/ui/shells/search_results_inspector.rs:220 | no | Index-only fallback metadata row. |
| Metadata grid: Title | yes @src/metadata.rs:1001 | no @src/ui/shells/search_results_inspector.rs:214 | no | Library grid data row. |
| Metadata grid: Artist | yes @src/metadata.rs:1008 | no @src/ui/shells/search_results_inspector.rs:214 | no | Library grid data row. |
| Metadata grid: Album artist | yes @src/metadata.rs:1015 | no @src/ui/shells/search_results_inspector.rs:214 | no | Library grid data row. |
| Metadata grid: Album/Feed | yes @src/metadata.rs:1022 | no @src/ui/shells/search_results_inspector.rs:214 | no | Library grid data row. |
| Metadata grid: Track # | yes @src/metadata.rs:1033 | no @src/ui/shells/search_results_inspector.rs:214 | no | Library grid data row. |
| Metadata grid: Publisher | yes @src/metadata.rs:1040 | no @src/ui/shells/search_results_inspector.rs:214 | no | Library grid data row. |
| Metadata grid: RSS track guid | yes @src/metadata.rs:1047 | no @src/ui/shells/search_results_inspector.rs:214 | no | Library grid data row. |
| Metadata grid: RSS feed guid | yes @src/metadata.rs:1054 | no @src/ui/shells/search_results_inspector.rs:214 | no | Library grid data row. |
| Metadata grid: Nostr handle | yes @src/metadata.rs:1066 | no @src/ui/shells/search_results_inspector.rs:214 | no | Library grid data row. |
| Metadata grid: RSS feed nostr handle | yes @src/metadata.rs:1073 | no @src/ui/shells/search_results_inspector.rs:214 | no | Conditional Library grid data row. |
| Metadata grid: Website | yes @src/metadata.rs:1087 | no @src/ui/shells/search_results_inspector.rs:214 | no | Library grid data row. |
| Metadata grid: RSS feed website | yes @src/metadata.rs:1094 | no @src/ui/shells/search_results_inspector.rs:214 | no | Conditional Library grid data row. |
| Metadata grid: Release date | yes @src/metadata.rs:1108 | no @src/ui/shells/search_results_inspector.rs:214 | no | Required named field. |
| Metadata grid: Release year | yes @src/metadata.rs:1115 | no @src/ui/shells/search_results_inspector.rs:214 | no | Derived from release date. |
| Metadata grid: Duration | yes @src/metadata.rs:1122 | no @src/ui/shells/search_results_inspector.rs:214 | no | Library grid data row. |
| Metadata grid: Artwork | yes @src/metadata.rs:1129 | no @src/ui/shells/search_results_inspector.rs:214 | no | Expandable Library grid row. |
| Metadata grid: Transcript | yes @src/metadata.rs:1136 | no @src/ui/shells/search_results_inspector.rs:214 | no | Library grid data row. |
| Metadata grid: Transcript text | yes @src/metadata.rs:1143 | no @src/ui/shells/search_results_inspector.rs:214 | no | Library grid data row. |
| Metadata grid: Contributors | yes @src/metadata.rs:1150 | no @src/ui/shells/search_results_inspector.rs:214 | no | Required contributor identity field. |
| Metadata grid: Contributor detail rows | yes @src/metadata.rs:1160 | no @src/ui/shells/search_results_inspector.rs:214 | no | Dynamic per-track contributor rows. |
| Metadata grid: Value Routes | yes @src/metadata.rs:1179 | no @src/ui/shells/search_results_inspector.rs:214 | no | Expandable Library grid row. |
| Metadata grid: Value Route item label | yes @src/ui/shells/library/track_detail_metadata_values.rs:278 | no @src/ui/shells/search_results_inspector.rs:214 | no | Populated only when expanded. |
| Metadata grid: Value Route child fields | yes @src/ui/shells/library/track_detail_metadata_values.rs:342 | no @src/ui/shells/search_results_inspector.rs:214 | no | Populated only when route item expanded. |
| Metadata grid: RSS item pubdate | yes @src/metadata.rs:1189 | no @src/ui/shells/search_results_inspector.rs:214 | no | Track item date when distinct from release date. |
| Metadata grid: Description | yes @src/metadata.rs:1198 | no @src/ui/shells/search_results_inspector.rs:214 | no | Required named field. |
| Metadata grid: Tempo | yes @src/metadata.rs:1353 | no @src/ui/shells/search_results_inspector.rs:214 | no | ID3-only row when present. |
| Metadata grid: Unused ID3 frame rows | yes @src/metadata.rs:1941 | no @src/ui/shells/search_results_inspector.rs:214 | no | Dynamic ID3 compare rows. |
| Metadata grid: Used ID3 field rows | yes @src/metadata.rs:1955 | no @src/ui/shells/search_results_inspector.rs:214 | no | Dynamic ID3 compare rows. |
| Metadata grid: MusicBrainz recording | yes @src/metadata.rs:1710 | no @src/ui/shells/search_results_inspector.rs:214 | no | MusicBrainz-only row when lookup shown. |
| Metadata grid: MusicBrainz release | yes @src/metadata.rs:1715 | no @src/ui/shells/search_results_inspector.rs:214 | no | MusicBrainz-only row when lookup shown. |
| Metadata grid: MusicBrainz release group | yes @src/metadata.rs:1720 | no @src/ui/shells/search_results_inspector.rs:214 | no | MusicBrainz-only row when lookup shown. |
| Metadata grid: Release country | yes @src/metadata.rs:1725 | no @src/ui/shells/search_results_inspector.rs:214 | no | MusicBrainz-only row when lookup shown. |
| Metadata grid: Release status / explicit-like state | yes @src/metadata.rs:1730 | no @src/ui/shells/search_results_inspector.rs:214 | no | Required explicit-state audit: no track explicit row exists; only MusicBrainz release status exists. |
| Metadata grid: Release packaging | yes @src/metadata.rs:1735 | no @src/ui/shells/search_results_inspector.rs:214 | no | MusicBrainz-only row when lookup shown. |
| Metadata grid: Barcode | yes @src/metadata.rs:1740 | no @src/ui/shells/search_results_inspector.rs:214 | no | MusicBrainz-only row when lookup shown. |
| Metadata grid: Release note / annotation | yes @src/metadata.rs:1745 | no @src/ui/shells/search_results_inspector.rs:214 | no | Required annotation audit: MusicBrainz release disambiguation only. |
| Metadata grid: Release type | yes @src/metadata.rs:1750 | no @src/ui/shells/search_results_inspector.rs:214 | no | MusicBrainz-only row when lookup shown. |
| Metadata grid: Release secondary types | yes @src/metadata.rs:1755 | no @src/ui/shells/search_results_inspector.rs:214 | no | MusicBrainz-only row when lookup shown. |
| Metadata grid: Label | yes @src/metadata.rs:1760 | no @src/ui/shells/search_results_inspector.rs:214 | no | MusicBrainz-only row when lookup shown. |
| Metadata grid: Media | yes @src/metadata.rs:1761 | no @src/ui/shells/search_results_inspector.rs:214 | no | MusicBrainz-only row when lookup shown. |
| Metadata grid: Disc # | yes @src/metadata.rs:1762 | no @src/ui/shells/search_results_inspector.rs:214 | no | MusicBrainz-only row when lookup shown. |
| Metadata grid: Disc subtitle | yes @src/metadata.rs:1769 | no @src/ui/shells/search_results_inspector.rs:214 | no | MusicBrainz-only row when lookup shown. |
| Metadata grid: Track note | yes @src/metadata.rs:1774 | no @src/ui/shells/search_results_inspector.rs:214 | no | MusicBrainz-only row when lookup shown. |
| Metadata grid: ISRC | yes @src/metadata.rs:1779 | no @src/ui/shells/search_results_inspector.rs:214 | no | MusicBrainz-only row when lookup shown. |
| Language | no @src/metadata.rs:994 | no @src/ui/shells/search_results_inspector.rs:214 | no | Required named field; feed language exists, but no track-level field is rendered. |
| Explicit state | no @src/views.rs:649 | no @src/ui/shells/search_results_inspector.rs:214 | no | Required named field; local projection drops explicit state. |
| Lyrics | no @src/metadata.rs:1136 | no @src/ui/shells/search_results_inspector.rs:214 | no | Transcript rows exist; no lyrics row exists. |

## Gap analysis

### Field: Library summary release / album context

- Library renderer: `src/ui/composites/track_detail_surface.rs:173`
- Index renderer: `src/ui/shells/search_results_inspector.rs:214`
- Library VM source: `TrackDetailVm::summary_rows`, `src/view_models/track_detail.rs:248`
- Index VM source: `IndexDetailDisplay::track`, `src/view_models/search_results/index_detail.rs:82`
- Local persistence today: `tracks.album_title`, `src/db.rs:2385`
- Hydration path: `TrackView::from_local_with_identity`, `src/views.rs:645`
- Routing: loading-shape
- Rationale: Local data is persisted and projected into Library detail, but the live Index detail display only carries title and secondary text from the search result row. If Index track detail should show release context as a field instead of lossy secondary text, the remote detail read model needs a richer track-detail shape.

### Field: Track number

- Library renderer: `src/ui/composites/track_detail_surface.rs:173`
- Index renderer: `src/ui/shells/search_results_inspector.rs:214`
- Library VM source: `TrackDetailVm::track_number_display`, `src/view_models/track_detail.rs:213`
- Index VM source: `TrackResultDisplay`, `src/view_models/search_results/results.rs:128`
- Local persistence today: `tracks.track_number`, `src/db.rs:2388`
- Hydration path: `TrackView::from_local_with_identity`, `src/views.rs:646`
- Routing: loading-shape
- Rationale: The local column and Library VM already support the field. Index search fetches full track detail before building the result row, but `index_track_display` drops `track_number`, so this is a read-model/loading-shape gap.

### Field: Duration

- Library renderer: `src/ui/composites/track_detail_surface.rs:173`
- Index renderer: `src/ui/shells/search_results_inspector.rs:214`
- Library VM source: `TrackDetailVm::duration_display`, `src/view_models/track_detail.rs:218`
- Index VM source: `api::Track::duration_secs`, `src/api.rs:147`
- Local persistence today: `tracks.duration_seconds`, `src/db.rs:2389`
- Hydration path: `TrackView::from_local_with_identity`, `src/views.rs:648`
- Routing: loading-shape
- Rationale: Duration is stored locally and exists in the remote API type, but the Index detail VM does not retain it. The gap is between fetched detail and the rendered `IndexDetailDisplay`.

### Field: Release date

- Library renderer: `src/ui/composites/track_detail_surface.rs:173`
- Index renderer: `src/ui/shells/search_results_inspector.rs:214`
- Library VM source: `TrackDetailVm::release_date_display`, `src/view_models/track_detail.rs:223`
- Index VM source: `api::Track::pub_date`, `src/api.rs:148`
- Local persistence today: `tracks.pub_date`, `src/db.rs:2382`
- Hydration path: RSS subscribe persists item pubdate at `src/rss/subscribe.rs:244`; local `TrackRow` currently omits it at `src/db.rs:348`
- Routing: loading-shape
- Rationale: The schema stores track item pubdate and the API type has `pub_date`, but the local `TrackRow` SELECT/projection does not load it, and Index detail discards fetched `pub_date`. This is a loading-shape gap on both sides, not a missing column.

### Field: Publisher

- Library renderer: `src/ui/composites/track_detail_surface.rs:173`
- Index renderer: `src/ui/shells/search_results_inspector.rs:214`
- Library VM source: `TrackDetailVm::publisher_display`, `src/view_models/track_detail.rs:228`
- Index VM source: `api::Track::publisher_text`, `src/api.rs:159`
- Local persistence today: not persisted on `tracks`; feed fallback can live in `feeds.extra_json`, `src/db.rs:2369`
- Hydration path: remote context merge uses fetched track/feed detail, `src/feed_service.rs:81`
- Routing: persistence
- Rationale: Library can show publisher when its source context is fetched, but the local track row has no publisher column and `TrackView::from_local_with_identity` sets `publisher_text` to `None`. Persisting source facts or extending local loading is needed before same-view Library parity can be durable.

### Field: Description

- Library renderer: `src/ui/shells/library/track_detail.rs:108`
- Index renderer: `src/ui/shells/search_results_inspector.rs:214`
- Library VM source: `TrackDetailVm::description`, `src/view_models/track_detail.rs:237`
- Index VM source: `api::Track::description`, `src/api.rs:151`
- Local persistence today: no `tracks.description`; `feeds.description` exists at `src/db.rs:2361`
- Hydration path: RSS enrichment reads item description at `src/rss/enrich.rs:142` and applies it to API track context at `src/rss/enrich.rs:193`
- Routing: persistence
- Rationale: Track descriptions are visible in the Library surface only when the detail context is enriched from RSS or MusicIndex. They are not stored in the local tracks table or loaded into `TrackView::from_local_with_identity`, while Index detail drops the fetched API description.

### Field: External website action

- Library renderer: `src/ui/shells/track.rs:63`
- Index renderer: `src/ui/shells/search_results_inspector.rs:214`
- Library VM source: `TrackDetailVm::identity_actions`, `src/view_models/track_detail.rs:281`
- Index VM source: `IndexDetailDisplay`, `src/view_models/search_results/index_detail.rs:42`
- Local persistence today: `entity_identity_links.url`, `src/db.rs:2534`
- Hydration path: `local_identity::track_facts`, `src/local_identity.rs:73`
- Routing: loading-shape
- Rationale: Local identity links are persisted and hydrated into Library `TrackView`, but `IndexDetailDisplay` has no identity-action list. A richer Index detail shape could surface remote source links without schema work.

### Field: Copy Nostr action

- Library renderer: `src/ui/shells/track.rs:63`
- Index renderer: `src/ui/shells/search_results_inspector.rs:214`
- Library VM source: `TrackDetailVm::identity_actions`, `src/view_models/track_detail.rs:304`
- Index VM source: `IndexDetailDisplay`, `src/view_models/search_results/index_detail.rs:42`
- Local persistence today: `entity_identity_ids.value`, `src/db.rs:2534`
- Hydration path: `local_identity::track_facts`, `src/local_identity.rs:73`
- Routing: loading-shape
- Rationale: Nostr identity is persisted as a source fact and projected into Library identity actions. The live Index detail shape has no identity facts or action slots, so this routes with the rest of the Index detail loading-shape work.

### Field: Library track command actions

- Library renderer: `src/ui/shells/library/track_detail_metadata.rs:140`
- Index renderer: `src/ui/shells/search_results_inspector.rs:214`
- Library VM source: `LibraryTrackActionVm::new`, `src/view_models/library.rs:839`
- Index VM source: `IndexDetailDisplay`, `src/view_models/search_results/index_detail.rs:42`
- Local persistence today: `tracks.is_in_library`, `src/db.rs:2396`; playlist membership uses `playlist_tracks`, `src/db.rs:2420`
- Hydration path: Library inspector frame owns local command state, `src/ui/shells/library/track_detail_metadata.rs:116`
- Routing: intentional asymmetry
- Rationale: Subscribe/remove, add-to-playlist, compare-ID3, MusicBrainz lookup, and staged edit controls are local-library commands. The Index track detail is a remote source drill-down and should not grow local mutation controls as a parity fix.

### Field: Index Source row

- Library renderer: `src/ui/composites/track_detail_surface.rs:173`
- Index renderer: `src/ui/shells/search_results_inspector.rs:219`
- Library VM source: `TrackDetailVm::summary_rows`, `src/view_models/track_detail.rs:246`
- Index VM source: literal `Index` row in renderer, `src/ui/shells/search_results_inspector.rs:219`
- Local persistence today: n/a; source is the remote search result origin, `src/view_models/search_results/results.rs:139`
- Hydration path: `index_track_display` creates rows with `SearchResultOrigin::Index`, `src/app/search_dispatch.rs:1311`
- Routing: intentional asymmetry
- Rationale: Index's source row is a drill-down provenance marker, not track metadata. Library surfaces provenance through Library chrome, identity actions, and metadata source columns instead.

### Field: Index ID row

- Library renderer: `src/ui/composites/track_detail_surface.rs:173`
- Index renderer: `src/ui/shells/search_results_inspector.rs:220`
- Library VM source: `TrackView::track_guid`, `src/views.rs:132`
- Index VM source: `IndexDetailDisplay::id`, `src/view_models/search_results/index_detail.rs:46`
- Local persistence today: `tracks.item_guid`, `src/db.rs:2378`
- Hydration path: `TrackView::from_local_with_identity`, `src/views.rs:639`
- Routing: intentional asymmetry
- Rationale: Library already exposes GUIDs inside the metadata grid, while Index exposes the activation/fallback id as a compact provenance row. The disagreement is presentation, not a missing source fact.

### Field: Metadata grid core source rows

- Library renderer: `src/ui/shells/library/track_detail_metadata_grid.rs:75`
- Index renderer: `src/ui/shells/search_results_inspector.rs:214`
- Library VM source: `track_metadata_rows`, `src/metadata.rs:994`
- Index VM source: `IndexDetailDisplay`, `src/view_models/search_results/index_detail.rs:42`
- Local persistence today: `tracks.track_title`, `tracks.artist_name`, `tracks.album_artist_name`, `tracks.album_title`, `tracks.track_number`, `tracks.duration_seconds`, `tracks.track_image_href`, `src/db.rs:2383`
- Hydration path: `track_row_from_sql`, `src/db.rs:930`
- Routing: loading-shape
- Rationale: Title, artist, album artist, album/feed, track number, duration, artwork, RSS track guid, and RSS feed guid are persisted and already projected into the Library metadata grid. Index has fetched track detail available in `index_track_display` but keeps only label, secondary text, and thumbnail.

### Field: Metadata grid release date and RSS item pubdate

- Library renderer: `src/ui/shells/library/track_detail_metadata_cells.rs:58`
- Index renderer: `src/ui/shells/search_results_inspector.rs:214`
- Library VM source: `musicindex_release_date`, `src/metadata.rs:365`
- Index VM source: `api::Track::pub_date`, `src/api.rs:148`
- Local persistence today: `tracks.pub_date`, `src/db.rs:2382`
- Hydration path: RSS subscribe writes `pub_date`, `src/rss/subscribe.rs:244`
- Routing: loading-shape
- Rationale: The database has item-level pubdate, and the metadata grid knows how to separate release date from RSS item pubdate. The local row projection and Index detail shape both fail to surface the stored/fetched value.

### Field: Metadata grid description

- Library renderer: `src/ui/shells/library/track_detail_metadata_cells.rs:58`
- Index renderer: `src/ui/shells/search_results_inspector.rs:214`
- Library VM source: `source_value_for_metadata_field`, `src/metadata.rs:1903`
- Index VM source: `api::Track::description`, `src/api.rs:151`
- Local persistence today: no `tracks.description`; `feeds.description` exists at `src/db.rs:2361`
- Hydration path: RSS enrichment applies item description to fetched track context, `src/rss/enrich.rs:194`
- Routing: persistence
- Rationale: Description is not stored as a track source fact locally even though the metadata grid can render it from enriched context. A downstream fix needs persistence before local Library can show the same value without refetching.

### Field: Metadata grid language

- Library renderer: not rendered; `track_metadata_rows` starts at `src/metadata.rs:994`
- Index renderer: not rendered; `src/ui/shells/search_results_inspector.rs:214`
- Library VM source: no `TrackView::language`; `src/views.rs:130`
- Index VM source: no `api::Track::language`; `src/api.rs:140`
- Local persistence today: `feeds.language`, `src/db.rs:2360`; no track-level language column
- Hydration path: feed schema only, `src/db.rs:2354`
- Routing: persistence
- Rationale: The named deferred field is absent from both track detail surfaces and the track API/view types. Because only feed-level language is persisted, track-level language parity requires a source-fact decision rather than just a renderer change.

### Field: Metadata grid explicit state

- Library renderer: not rendered; `track_metadata_rows` has no explicit row at `src/metadata.rs:994`
- Index renderer: not rendered; `src/ui/shells/search_results_inspector.rs:214`
- Library VM source: `TrackView::explicit`, `src/views.rs:143`
- Index VM source: `api::Track::explicit`, `src/api.rs:150`
- Local persistence today: `tracks.itunes_explicit`, `src/db.rs:2391`
- Hydration path: RSS subscribe reads and writes `itunes_explicit`, `src/rss/subscribe.rs:216`
- Routing: loading-shape
- Rationale: The value is persisted and exists in API/view types, but `TrackView::from_local_with_identity` sets `explicit` to `None`, and no Library or Index track detail renderer consumes it. This is a read-model gap.

### Field: Metadata grid lyrics / annotation

- Library renderer: not rendered; transcript rows are rendered at `src/metadata.rs:1136`
- Index renderer: not rendered; `src/ui/shells/search_results_inspector.rs:214`
- Library VM source: no `TrackView::lyrics` or annotation field, `src/views.rs:130`
- Index VM source: no `api::Track::lyrics` or annotation field, `src/api.rs:140`
- Local persistence today: not persisted
- Hydration path: n/a
- Routing: persistence
- Rationale: The task's named lyrics/annotation field is not the same as transcript or MusicBrainz disambiguation. There is no current track-level local or remote VM field for lyrics/annotation, so this routes to source-fact persistence/design before UI parity.

### Field: Metadata grid contributor identity

- Library renderer: `src/ui/shells/library/track_detail_metadata_cells.rs:58`
- Index renderer: `src/ui/shells/search_results_inspector.rs:214`
- Library VM source: `track_metadata_rows`, `src/metadata.rs:1150`
- Index VM source: `api::Track::source_contributors`, `src/api.rs:162`
- Local persistence today: `entity_contributors` with `owner_kind = 'track'`, `src/db.rs:2534`
- Hydration path: RSS track contributor persistence, `src/rss/subscribe.rs:379`
- Routing: loading-shape
- Rationale: Per-track contributors are persisted and hydrated for Library identity facts, and the remote API type carries contributors. Index detail drops this detail into a compact result row, so the parity gap is in the detail loading shape.

### Field: Metadata grid website / Nostr source facts

- Library renderer: `src/ui/shells/library/track_detail_metadata_cells.rs:58`
- Index renderer: `src/ui/shells/search_results_inspector.rs:214`
- Library VM source: `source_value_for_metadata_field`, `src/metadata.rs:1892`
- Index VM source: `api::Track::source_links` and `api::Track::source_ids`, `src/api.rs:164`
- Local persistence today: identity source fact tables, `src/db.rs:2534`
- Hydration path: `local_identity::track_facts`, `src/local_identity.rs:73`
- Routing: loading-shape
- Rationale: These source facts already have persistence and Library metadata-grid renderers. Index fetch currently requests no include for track detail and then drops any facts it has, so this belongs to Index detail loading-shape work.

### Field: Metadata grid transcript fields

- Library renderer: `src/ui/shells/library/track_detail_metadata_cells.rs:58`
- Index renderer: `src/ui/shells/search_results_inspector.rs:214`
- Library VM source: `track_metadata_rows`, `src/metadata.rs:1136`
- Index VM source: source links on `api::Track`, `src/api.rs:164`
- Local persistence today: transcript URL in track `extra_json`, `src/db.rs:2397`
- Hydration path: RSS subscribe stores transcript info in `extra_json`, `src/rss/subscribe.rs:223`
- Routing: loading-shape
- Rationale: Transcript URLs are captured locally and rendered by the Library metadata grid. Index track detail does not carry source links or transcript facts into `IndexDetailDisplay`.

### Field: Metadata grid value routes

- Library renderer: `src/ui/shells/library/track_detail_metadata_values.rs:56`
- Index renderer: `src/ui/shells/search_results_inspector.rs:214`
- Library VM source: `track_metadata_rows`, `src/metadata.rs:1179`
- Index VM source: `api::Track::payment_routes`, `src/api.rs:169`
- Local persistence today: `tracks.item_value_json`, `src/db.rs:2395`
- Hydration path: RSS subscribe writes item value JSON, `src/rss/subscribe.rs:222`
- Routing: loading-shape
- Rationale: The Library grid has expandable value-route rendering, including child fields. The local schema stores item value data and the API type has payment routes; the missing piece is detail-shape projection on the Index side and durable local projection into `TrackView`.

### Field: Metadata grid ID3 compare rows

- Library renderer: `src/ui/shells/library/track_detail_metadata_grid.rs:84`
- Index renderer: `src/ui/shells/search_results_inspector.rs:214`
- Library VM source: `aligned_compare_rows`, `src/metadata.rs:1310`
- Index VM source: `IndexDetailDisplay`, `src/view_models/search_results/index_detail.rs:42`
- Local persistence today: local file/tag state is not represented as remote Index data; local file rows live in `local_files`, `src/db.rs:2401`
- Hydration path: `TagCompareResult` loaded in Library inspector frame, `src/ui/shells/library/track_detail.rs:40`
- Routing: intentional asymmetry
- Rationale: ID3 rows, unused frames, frame labels, and staged edit comparison status are local-file inspection concerns. They should not be expected on a remote Index-source detail surface.

### Field: Metadata grid MusicBrainz lookup rows

- Library renderer: `src/ui/shells/library/track_detail_metadata_grid.rs:94`
- Index renderer: `src/ui/shells/search_results_inspector.rs:214`
- Library VM source: `musicbrainz_remainder_rows`, `src/metadata.rs:1693`
- Index VM source: `IndexDetailDisplay`, `src/view_models/search_results/index_detail.rs:42`
- Local persistence today: not persisted as local track facts unless later applied as ID3/source facts
- Hydration path: `library_musicbrainz_panel`, `src/ui/shells/library/track_detail_metadata.rs:359`
- Routing: intentional asymmetry
- Rationale: MusicBrainz recording/release/release-group IDs, release country/status/packaging/barcode/type/media/disc/ISRC rows are lookup-candidate diagnostics for local metadata editing. They are intentionally not part of the MusicIndex-source track detail contract.

### Field: Metadata grid release status / explicit-like state

- Library renderer: `src/metadata.rs:1730`
- Index renderer: `src/ui/shells/search_results_inspector.rs:214`
- Library VM source: `MusicBrainzCandidate::release_status`, `src/metadata.rs:1733`
- Index VM source: `api::Track::explicit`, `src/api.rs:150`
- Local persistence today: `tracks.itunes_explicit`, `src/db.rs:2391`; MusicBrainz release status not persisted
- Hydration path: explicit via RSS subscribe, `src/rss/subscribe.rs:216`; MusicBrainz release status via lookup panel, `src/ui/shells/library/track_detail_metadata.rs:359`
- Routing: intentional asymmetry
- Rationale: MusicBrainz release status is not the requested track explicit state. It appears only as lookup context in the Library metadata grid, so it should not be treated as Index explicit-state parity.

### Field: Metadata grid MusicBrainz release note / annotation

- Library renderer: `src/metadata.rs:1745`
- Index renderer: `src/ui/shells/search_results_inspector.rs:214`
- Library VM source: `MusicBrainzCandidate::release_disambiguation`, `src/metadata.rs:1748`
- Index VM source: no annotation field in `api::Track`, `src/api.rs:140`
- Local persistence today: not persisted
- Hydration path: MusicBrainz lookup panel, `src/ui/shells/library/track_detail_metadata.rs:359`
- Routing: intentional asymmetry
- Rationale: The visible Library row is a MusicBrainz candidate disambiguation note, not a persisted track annotation. Treating it as a track annotation parity gap would cross source boundaries.

### Field: Hero artwork parity

- Library renderer: `src/ui/composites/track_header.rs:105`
- Index renderer: `src/ui/shells/search_results_inspector.rs:211`
- Library VM source: `TrackView::image_url`, `src/views.rs:145`
- Index VM source: `TrackResultDisplay::thumbnail_href`, `src/view_models/search_results/results.rs:135`
- Local persistence today: `tracks.track_image_href`, `src/db.rs:2392`
- Hydration path: `index_track_display` sets thumbnail href, `src/app/search_dispatch.rs:1324`
- Routing: loading-shape
- Rationale: Index track result rows can carry thumbnails, but `render_index_detail_display` renders an empty `Thumbnail` instead of consuming `thumbnail_href` because `IndexDetailDisplay` has no thumbnail field. This is a narrow detail VM shape gap.

## Open questions

- Should Index track drill-down be upgraded from compact fallback detail to the shared `TrackDetailSurface` contract, or should it remain intentionally sparse and only list provenance?
- Should track-level description be persisted as a source fact separate from feed description, given Library currently shows it only through fetched/enriched context?
- Should language be treated as feed/release-level only for this product, or is track-level language required for parity?
- Should explicit state use the existing RSS `itunes_explicit` column as the local source fact, or should it be normalized into a source-fact table before surfacing?
- Should payment/value routes be in the parity scope for Index track detail, or remain a Library metadata-editing surface only?
- Should per-track contributor identity be visible on Index track detail now that ADR-0028 contributor persistence exists locally, or deferred to the sibling artist/playlist/contributor synthesis?

## Out of scope (handled by sibling triage tasks)

- Album / release detail fields → Task 001
- Artist + Playlist detail fields → Task 003
