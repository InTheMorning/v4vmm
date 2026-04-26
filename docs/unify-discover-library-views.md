# Unify Discover + Library entity views

## Context

Discover (`src/search.rs`) and Library (`src/library.rs`) render the same entity types — Artist, Feed/Album, Track — with two parallel sets of code:

- **Render helpers are duplicated**: `render_thumb`, `render_detail_header`, `render_detail_grid`, `metadata_action_button`, `section_heading` exist twice (search.rs:6363/6120/6259/6595/6543 vs library.rs:4981/4883/4933/5054). Drift is already happening (e.g. Library's `render_album_thumb` at 5247 has no Discover analogue).
- **Track row is forked**: `render_track_row` in search.rs:5586 takes `api::Track`; Library's row is inlined at library.rs:3055-3143 and takes `&db::TrackRow`. Different action buttons; different code paths to the same outcome.
- **Library lacks an Artist view entirely** — artists exist only as tree nodes (library.rs:2764), so cross-navigation from Library → Artist is impossible.
- **Feed-level download state is ad hoc**: library.rs:3169 counts `local_path.is_some()` but there is no aggregate "Download all / Remove all" toggle, and Discover only has subscribe/unsubscribe.
- **Subscription vs in-library are two flags** that the user wants collapsed: subscribed ⇔ at least one track in library. `db::reconcile_feed_subscription_by_url` (db.rs:450) already encodes this rule.

User goal: one set of Artist and Feed views shared by both tabs. The only Discover/Library divergence should live at the track-row level (Library shows compare + MusicBrainz buttons; Discover does not). Feed-level download state should aggregate per-track state and toggle subscription accordingly. Library data stays local-only — file tags + DB are sufficient, no online enrichment.

## Design

### Layer 1 — Source-agnostic view models  (`src/views.rs`, new)

```rust
pub struct ArtistView { id: ArtistRef, name, image_url, area, year_range, feed_count, track_count, links, ... }
pub struct FeedView   { id: FeedRef,   title, artist, image_url, release_date, language, episode_count, description, payment_routes, contributors, tracks: Vec<TrackView> }
pub struct TrackView  { id: TrackRef,  number, title, artist, album, duration, image_url, audio_url, mime, pub_date, contributors, payment_routes }

pub enum ArtistRef { Musicindex(String), LocalArtistName(String) }
pub enum FeedRef   { Musicindex(String /* guid */), LocalFeedId(i64) }
pub enum TrackRef  { Musicindex(String /* guid */), LocalTrackId(i64) }
```

Conversion lives next to the view models:
- `ArtistView::from_api(api::Artist)`, `::from_local_rows(&[db::TrackRow])`
- `FeedView::from_api(api::Feed)`, `::from_local(db::FeedRow, Vec<db::TrackRow>)`
- `TrackView::from_api(api::Track)`, `::from_local(db::TrackRow)`

Two existing helpers in library.rs already do most of this work and move under the new module: `track_row_to_api_track` (library.rs:2099), `track_row_to_feed` (library.rs:2128), `merge_track_context_from_detail` (library.rs:2208).

### Layer 2 — DAO trait  (`src/sources.rs`, new)

```rust
pub trait MetadataSource {
    fn fetch_artist(&self, r: &ArtistRef)                  -> Result<ArtistView>;
    fn fetch_feed  (&self, r: &FeedRef, mode: FetchMode)   -> Result<FeedView>;
    fn fetch_track (&self, r: &TrackRef)                   -> Result<TrackView>;
    fn list_feeds_for_artist(&self, r: &ArtistRef)         -> Result<Vec<FeedView>>;  // shallow (no track lists)
}
pub enum FetchMode { Shallow, WithTracks }
```

Impls:
- `ApiSource(api::Client)` wraps existing `Client::fetch_artist/fetch_feed/fetch_track/fetch_tracks_by_artist`.
- `LocalSource(Arc<Mutex<Connection>>)` wraps `db::feed_url_by_id`, `db::feed_tracks`, `db::library_tracks`, plus a new `db::artist_feeds_by_name(&str)` that GROUPs library_tracks by feed_id for a given album_artist_name.

Discover passes `ApiSource`; Library passes `LocalSource`. Both render through identical UI fns.

### Layer 3 — Shared UI helpers  (`src/ui_common.rs`, new)

Lift the duplicated helpers into one module. Keep the search.rs versions (slightly more polished) as canonical:

| Helper                    | Source of truth   | Library duplicate to delete |
|---------------------------|-------------------|------------------------------|
| `render_thumb`            | search.rs:6363    | library.rs:4981              |
| `render_detail_header`    | search.rs:6120    | library.rs:4883              |
| `render_detail_grid`      | search.rs:6259    | library.rs:4933              |
| `metadata_action_button`  | search.rs:6595    | library.rs:5054              |
| `section_heading`         | search.rs:6543    | (Library inlines)            |
| `truncated`/`optional_row`/`badge_text`/`type_color` | search.rs:6608/6620/6786 | (Library inlines) |
| `render_add_to_playlist_panel` (one parameterised version) | derived from current copies | library.rs:3265, 3306, 3837 |

### Layer 4 — Unified entity views  (`src/ui_artist.rs`, `src/ui_feed.rs`, new)

Single render fn per entity, parameterised on a context enum that controls action affordances:

```rust
pub enum ViewContext<'a> {
    Discover { source: &'a ApiSource },
    Library  { source: &'a LocalSource, mb_status: &'a BTreeMap<i64, MbTrackStatus> },
}
```

`render_artist_view(view, ctx, app, cx)` and `render_feed_view(view, ctx, app, cx)` produce identical layouts. The context only changes:
- Which `MetadataSource` is used for navigation clicks (open feed → fetch via the same source the parent came from).
- Whether to render the Library-only inspector affordances (compare panel link, MusicBrainz status badge) on each track row.

### Layer 5 — Unified track row  (`src/ui_track.rs`, new)

```rust
pub fn render_track_row(
    view: &TrackView,
    feed: Option<&FeedView>,
    mode: TrackRowMode,
    downloaded: bool,
    in_flight: bool,
    playlists: &[db::Playlist],
    app: &mut SearchApp,
    cx,
) -> AnyElement;

pub enum TrackRowMode {
    Discover,
    Library { local_path: Option<PathBuf>, mb_status: Option<MbTrackStatus> },
}
```

Always-shown buttons: download/remove toggle, +playlist popup, play. Library mode additionally renders the compare-tags button and MB status badge. Adding to a playlist auto-downloads if the track is not yet local (already implemented for Library at library.rs:584; lift the same call into Discover's add path).

### Layer 6 — Feed-level aggregate state  (`src/db.rs`)

Add:
```rust
pub struct FeedDownloadStatus { pub downloaded: usize, pub total: usize }
impl FeedDownloadStatus { pub fn is_complete(&self) -> bool { self.total > 0 && self.downloaded == self.total } }
pub fn feed_download_status(conn: &Connection, feed_id: i64) -> Result<FeedDownloadStatus>;
```

In `render_feed_view`, the action row gets one button:
- `is_complete()` → "Remove all" (calls existing `db::unsubscribe_feed_tracks`, which already flips every track's `is_in_library`).
- otherwise → "Download all" (iterates `feed_tracks`, kicks off `track_compare::download_track` for each not yet local; reuses Discover's existing per-track handler).

After every per-track download or removal, call `db::reconcile_feed_subscription_by_url` (db.rs:450) so `is_subscribed` tracks the aggregate. The legacy `Subscribe`/`Unsubscribe` buttons in both tabs are removed — subscription is now derived state.

## Files touched

| Stage | New                                              | Modified                                                                 |
|-------|--------------------------------------------------|--------------------------------------------------------------------------|
| 1     | `src/views.rs`                                   | re-export from `src/main.rs`                                             |
| 2     | `src/sources.rs`                                 | uses `api.rs`, `db.rs`                                                   |
| 3     | `src/ui_common.rs`                               | delete duplicates from `library.rs`; switch call sites                   |
| 4     | `src/ui_artist.rs`, `src/ui_feed.rs`             | `search.rs` (replace `render_artist_inspector` / `render_discover_feed_inspector` bodies); `library.rs` (delete `render_album_detail`, route to shared) |
| 5     | `src/ui_track.rs`                                | `search.rs` (drop `render_track_row`), `library.rs` (drop inline album-detail row) |
| 6     | —                                                | `db.rs` (`feed_download_status`), `search.rs` + `library.rs` (feed action row), removal of `Subscribe`/`Unsubscribe` buttons |

## Staged rollout

Each stage compiles, tests pass, and no current behaviour regresses.

### Stage 1 — View models + DAO trait (no UI change yet)
Add `views.rs` and `sources.rs`. Move `track_row_to_*` and `merge_track_context_from_detail` over. Search/Library still call the existing render code; new layer is dead but compiles. Unit-test `*::from_api` and `*::from_local` round-trips.

### Stage 2 — Lift shared UI helpers
Create `ui_common.rs`; switch both modules to import from it. Delete library.rs duplicates. Visual diff = none.

### Stage 3 — Unified Artist + Feed view fns (Discover only)
Implement `render_artist_view` and `render_feed_view`. Replace search.rs:3093 / search.rs:3224 bodies with one-line dispatches. Library still uses old code. Smoke: open an artist and a feed in Discover.

### Stage 4 — Unified track row (Discover only)
Replace search.rs:5586 body with a dispatch into `ui_track::render_track_row` in `Discover` mode. No behavioural change.

### Stage 5 — Add Library Artist view + reuse Feed view
Add `db::artist_feeds_by_name`. Library tree click on an artist node opens `render_artist_view` with `ViewContext::Library`. Replace `render_album_detail` (library.rs:3004) with `render_feed_view` in Library mode. Library track rows now show compare + MB badges via `TrackRowMode::Library`. The three duplicated playlist-add panels (library.rs:3265, 3306, 3837) collapse into one `render_add_to_playlist_panel` from `ui_common`.

### Stage 6 — Feed-level Download-all / Remove-all + derived subscription
Add `db::feed_download_status`. Render the new action button in the shared `render_feed_view`. After each per-track download/removal in either tab, call `db::reconcile_feed_subscription_by_url`. Remove the legacy Subscribe/Unsubscribe buttons in Discover (search.rs:3224 area) and Library (library.rs:3204). Add a confirm prompt before "Remove all" since it is destructive.

### Stage 7 — Cleanup
Delete now-unused: `render_album_detail` body, library's duplicated helpers, `render_track_inspector`-style dead fns flagged by the compiler.

## Verification

- `cargo check` and `cargo test --lib` pass after each stage.
- After Stage 3: open the Discover inspector for an artist and a feed — visually identical to today.
- After Stage 5: in Library, click an artist node → see new artist view listing their subscribed feeds; click a feed → see the unified feed view with compare + MB on each track row.
- After Stage 6: download every track in a feed → "Download all" flips to "Remove all"; remove one track → flips back to "Download all"; `is_subscribed` in DB tracks the aggregate. Confirm via `sqlite3 v4vmm.db "SELECT feed_url, is_subscribed FROM feeds"` while toggling.
- Manual cross-navigation smoke: Library artist → feed → track → back; Discover artist → feed → track → back. Same affordances except compare/MB badges only in Library.
