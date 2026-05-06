# v4vmm — Performance and Maturity Report

## TL;DR

1. **SQLite is run on near-default settings.** Only `foreign_keys=ON` is configured; no WAL, no `synchronous=NORMAL`, no `mmap_size`, no `cache_size`. Single biggest one-day win in the codebase.
2. **All DB access serialises through one `Arc<Mutex<Connection>>`.** Reads block writes and vice-versa; an in-flight metadata write can stall a UI list refresh.
3. **No list virtualization anywhere.** `render_tree`, the album track list, the playlist detail and the Discover results render every row every frame. Falls over at 5–10k tracks.
4. **One `reqwest::blocking::Client` per call site, not per process.** Nine separate `Client::new()` sites means TLS handshakes are paid per operation; the connection pool inside each client is wasted.
5. **MusicBrainz rate-limit compliance is wishful.** Two `sleep(1100ms)` calls inside two different code paths, no global token bucket — under bulk operations the app can exceed the public 1 req/s limit.

## Methodology

Static analysis only. No profiler trace, no benchmark, no flame graph. Sources:

- `src/library.rs` (5,455 LOC), `src/search.rs` (7,915 LOC), `src/db.rs` (1,004 LOC), `src/api.rs` (637 LOC), `src/musicbrainz.rs` (1,124 LOC), plus `src/media/`, `src/audio_tags.rs`, `src/track_compare.rs`, `src/rss/`.
- `cx.notify()` is called from **140 sites** across `src/`. That alone justifies looking at the render path.
- `cargo check` and `cargo test --lib` pass on `master` (86 tests).

What's *not* in scope here: UX redesign, packaging/distribution, audio playback latency, any work blocked on the deferred trait-based unification from `docs/unify-discover-library-views.md`.

## Findings

### 3.1 UI render path

- **`LibraryApp::render` rebuilds the filtered tree on every frame** while a search query is active (`src/library.rs:2430`). `filter_tree` (`src/library.rs:1959`) clones every matching `ArtistNode`, `AlbumNode`, and `TrackRow`. For a 5k-track library that's roughly 1.5 MB of allocation per frame during typing. **Severity: High** at >2k tracks.
- **No list virtualization.** `render_tree` (`src/library.rs:2775`), `render_album_detail` track list (`src/library.rs:3022`), `render_playlist_detail` (`src/library.rs:~3601`), and Discover's results list all do `.children(items)` against the full vector. GPUI re-creates every element every frame. **Severity: High** at >2k visible rows; will become the dominant cost before any DB issue does.
- **`cx.notify()` fan-out is high.** 140 call sites; pointer-drag handlers (`src/search.rs:1736`) call `notify` per pixel of mouse-move when resizing the divider, triggering a full search-results rebuild each step. **Severity: Medium**.
- **Closures clone `TrackRow`/`AlbumNode` per row.** `render_tree`'s album-click handler clones the whole `AlbumNode` into the closure (`src/library.rs:2847`); per-track closures in the playlist detail clone the row. With 5k tracks fully expanded that's 5k clones per frame on top of the work in (1). **Severity: Medium**.
- **Synchronous DB calls inside event handlers.** `LibraryApp::reload` (`src/library.rs:~407`), `select_playlist` (`src/library.rs:~458`), `unsubscribe_feed` (`src/library.rs:~971`) all `conn.lock()` and run a query on the UI thread. Fine today; will visibly hitch once `Mutex<Connection>` contention shows up. **Severity: Medium**.

### 3.2 Database + local storage

- **PRAGMA setup is bare.** `src/db.rs:723` only sets `foreign_keys=ON`. Missing: `journal_mode=WAL` (writers don't block readers), `synchronous=NORMAL` (writes commit ~10× faster with negligible durability cost on a desktop app), `temp_store=MEMORY`, `mmap_size`, a sized `cache_size`. **Severity: High**, **effort: ~10 lines**.
- **One `Mutex<Connection>` for the whole app** (`src/app.rs:64`, used by every caller). Even with WAL enabled, rusqlite's single connection serialises reads. A read-only pool (e.g. `r2d2_sqlite` with a writer + N readers, or just a hand-rolled `Vec<Connection>` behind a `Mutex<VecDeque>`) is the right shape. **Severity: High**.
- **N+1 in `build_tree`** (`src/library.rs:1930–1935`): for each unique `feed_id` in the result set, `db::feed_url_by_id` runs a separate `SELECT`. The fix is one line — `library_tracks` (`src/db.rs:203`) already joins `feeds f` but its SELECT list doesn't include `f.feed_url`. Add the column, populate `TrackRow::feed_url`, delete the lookup loop. **Severity: Medium → trivial fix**.
- **Missing index: `tracks.enclosure_url`.** Queried at least at `src/db.rs:391, 406` for download-completion lookups. At 10k tracks each lookup is a full scan. `tracks.item_guid` is *covered* by the `UNIQUE(feed_id, item_guid)` index but only when `feed_id` is in the WHERE clause — confirm that's true at every call site, otherwise add a standalone index. The new artist-detail view (Stage 5 of the unification plan) groups by `album_artist_name`; an index there will matter as the library grows. **Severity: Medium**.
- **Image cache is bounded** (`src/media/image_cache.rs:28, 47–62`): `hot_capacity` LRU + `max_disk_bytes` with periodic eviction. The earlier audit's "unbounded growth" claim was wrong. The remaining concern is that eviction spawns a new OS thread every 64 writes (line ~62) instead of using a long-lived worker; under heavy art ingest you get thread-spawn churn. **Severity: Low**.
- **Audio-tag reads have no cache.** `read_audio_tags` (`src/audio_tags.rs:56`) is called from at least four UI paths (`src/library.rs:1715, 1807`, plus search inspector and metadata grid). Each call re-parses the full ID3/FLAC tag block. Caching by `(path, mtime)` would eliminate redundant reads when a user clicks back and forth between detail panels. **Severity: Medium**.

### 3.3 Network + concurrency

- **Nine `reqwest::blocking::Client::new()` sites** (`src/api.rs:327, 334, 578`, `src/library.rs:1660, 1663`, `src/search.rs:2566, 2701, 6462`, `src/rss/enrich.rs:17, 97`, `src/rss/subscribe.rs:19`). Each builds its own pool; per-call clients defeat keep-alive. A 100-track album download = 100 cold TLS handshakes. **Severity: High**, fix is a process-wide `OnceCell<Client>`.
- **No global MusicBrainz rate limiter.** `src/musicbrainz.rs:239` and `src/musicbrainz.rs:334` each `sleep(1100)` between requests inside their own loop, but two parallel background tasks doing MB lookups will both serially-pace themselves and still exceed 1 req/s when interleaved. A `tokio::sync::Semaphore` or a small token-bucket guarding every call site is the right shape. **Severity: Medium** (correctness as well as perf — the public MB API will start 503-ing).
- **No retry/backoff.** Every HTTP error path (`src/api.rs:516`, `src/musicbrainz.rs:193`, `src/track_compare.rs:306`, `src/rss/enrich.rs:97`) propagates immediately. A single transient 502 aborts a 50-track bulk metadata operation. **Severity: High** for usability under flaky networks.
- **No request deduplication for API or MB lookups.** Download in-flight tracking exists (`src/search.rs:196, 271`); there's no equivalent for `fetch_artist`, `fetch_feed`, or per-track MB lookups. Double-clicking an artist fires two requests. **Severity: Low** today, **Medium** at scale.
- **Cancellation is cosmetic.** Background tasks hold a `WeakEntity<SearchApp>`; if the entity drops, the UI callback is skipped — but the HTTP request still completes. Power users navigating quickly accumulate orphaned in-flight work. **Severity: Low**, but it's a maturity tell.
- **Downloads stream to disk** via `std::io::copy` (`src/track_compare.rs`). Not buffered in memory. **No progress reporting**, though — UI shows nothing until the file is fully written. **Severity: Low** for performance, **Medium** for perceived maturity.

### 3.4 Cross-cutting

- **No structured logging surface.** No `tracing`/`log` setup that surfaces to the user; failures show up as `eprintln!` or are swallowed. Hard to triage user reports without it. **Severity: Medium**.
- **No metrics / instrumentation.** Even an in-memory ring buffer of the last N HTTP timings would make future perf work data-driven instead of vibes-driven. **Severity: Low**.
- **No telemetry around bulk operations.** "Subscribe feed" and "Download all" silently spawn many tasks; if one fails, the user has no audit trail. **Severity: Medium**.

## Maturity gaps (separate from raw perf)

- **List virtualization** — already covered above; calling it out separately because at 10k+ tracks it stops being a slowdown and becomes a memory exhaustion bug.
- **Progress UI** for downloads, RSS subscribes, MusicBrainz batch lookups.
- **Retry-with-backoff** on transient HTTP failures, not just hard-fail.
- **Real cancellation** (carry an `AtomicBool` / `tokio::sync::Notify` into the HTTP task).
- **MB rate-limit compliance under load** — token bucket shared across all call sites.
- **Observability** — `tracing` with a level filter, optional log file.
- **Bulk operation feedback** — a notification or toast when a 50-track import finishes, with a count of failures.

## Recommended sequencing

### Tier 1 — biggest ROI, each landable in a day

1. SQLite PRAGMAs: `journal_mode=WAL`, `synchronous=NORMAL`, `temp_store=MEMORY`, `mmap_size=268435456`, `cache_size=-65536`. One block in `db::open`. Nothing else changes.
2. Add `tracks.enclosure_url` and `tracks.album_artist_name` indexes. Two `CREATE INDEX IF NOT EXISTS` lines + a schema-version bump.
3. Kill the `build_tree` N+1: add `f.feed_url` to the `library_tracks` SELECT, populate `TrackRow::feed_url`, delete the `feed_url_cache` block in `build_tree` (`src/library.rs:1921–1935`).
4. Move to a single shared `reqwest::blocking::Client` (`OnceCell<Client>` in a small `http` module). Replace all nine `Client::new()` sites.

### Tier 2 — correctness + scale

5. Replace `Arc<Mutex<Connection>>` with a small read pool (one writer + N readers). With WAL on, readers will run concurrently. Threading audit at every former `.lock()` site to make sure no caller assumed exclusive access.
6. List virtualization for `render_tree`, `render_album_detail` track list, `render_playlist_detail`, and Discover results. GPUI's `uniform_list` is the obvious building block; if its API doesn't fit, a simple "render only rows in `[scroll_top, scroll_top + viewport_height]`" wrapper is enough.
7. Global MusicBrainz token-bucket rate limiter (1 token/s, capacity 1). Wrap every call site.
8. Retry-with-backoff on transient HTTP failures (4 retries, exponential, only on 5xx and connection errors).
9. Request deduplication for `fetch_artist`/`fetch_feed`/MB lookups — a `Mutex<HashMap<Key, Shared<Future>>>` pattern.

### Tier 3 — polish

10. Download progress UI (already streams, just pipe `Read::read` chunk sizes through a `mpsc` to the entity).
11. Real cancellation: each spawned task gets an `Arc<AtomicBool>`; the HTTP body read loop checks it.
12. `(path, mtime)` cache for `read_audio_tags`.
13. `tracing` setup with env-driven level filter and an optional rolling log file.
14. Move `ImageCache` eviction to a long-lived worker thread; drop the per-64-write spawn.

## Out of scope

- Anything blocked on the deferred `InspectorHost` trait from `docs/unify-discover-library-views.md` Stage 5.
- UX redesign — this report is about *how the existing UI behaves*, not *what the UI should be*.
- Audio playback / decoding pipeline — not inspected.
- Cross-platform packaging, signing, auto-update.
