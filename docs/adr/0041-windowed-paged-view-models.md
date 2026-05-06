# ADR 0041: Windowed Paged View-Models

## Status

Accepted — 2026-05-06. Phase E in flight: `PagedListVm` shipped with
`PagedTrackListActor`, `Skeleton` primitive, runtime `VmBus`
subscription. The screen swap (`library-track-list-paged-vm`) is the
remaining work; the parallel-additive
`view_models::paged_playlist_detail::PagedPlaylistDetailVm` is in
place to make it a small slice.

Depends on ADR 0040 (Async VM Runtime) for the actor + snapshot layer
that owns the cache.

## Context

Both `feat/design-tokens-and-primitives` and `origin/master` materialise
list-shaped view-models eagerly:

- `view_models/library.rs` holds `Vec<TrackRow>`, `Vec<AlbumNode>`,
  `Vec<ArtistNode>`, `Vec<PlaylistSidebarRowVm>`, `Vec<TrackRow>` for
  playlist tracks.
- `application/queries/library.rs::cached_tracks(&conn) -> Vec<TrackRow>`
  returns the entire result set.
- Every list query family on master follows the same shape: one
  `SELECT`, one full `Vec<Row>`, hand it to the VM.

This is fine today and will not stay fine. Cold open scales linearly
with library size. A `TrackRow` carries title / artist / paths / MB
status / source URL — multi-kilobyte per row. A 50k-track library
round-trips ~50 MB on every reload. A single mutation
("a track was tagged") rebuilds the whole list because there is no
per-id invalidation hook. On a slow disk the UI is blocked behind the
SELECT before it can paint anything.

GPUI's `gpui_component::table::DataTable` already uses a windowed
`render_td(row, col)` callback, so the *render* cost is bounded — but
the VM cost is not. The entire dataset sits in RAM whether or not it
is on screen.

## Decision

Introduce a generic **`PagedListVm<Id, Row, Sort>`** owned by an actor
(per ADR 0040). Any list-shaped VM whose payload may exceed ~10k rows
MUST use it. The shape is:

```
identity index   (eager, cheap)        Vec<(Id, SortKey)>
   └──> total_count(), id(i), sort_key(i)
row body cache   (lazy, paged, LRU)    LruCache<Id, Arc<Row>>
   └──> row(i) → Either<Placeholder, RowVm>
```

### Rules

1. **Eager identity index.** On first load the actor fetches an
   ordered `Vec<Id>` via a dedicated cheap query
   (`fn track_ids_ordered_by(conn, listing) -> Vec<i64>`). ~8 bytes
   per row for `i64` ids; 100k rows ≈ 0.8 MB. The composite sort key
   is resolved inside the query.
2. **Lazy body cache.** Row bodies load in pages of N
   (default `DEFAULT_PAGE_SIZE = 64`; configurable per actor). Each
   page is one `bodies_by_ids(conn, &[Id])` query bound to a
   page-shaped slice of the index.
3. **LRU eviction.** The cache is `LruCache<Id, Arc<Row>>` capped at
   `DEFAULT_CACHE_PAGES * DEFAULT_PAGE_SIZE = 256` rows by default.
   Pages outside the hot window evict automatically.
4. **Synchronous reads with placeholders.** The VM exposes two
   variants:
   - `vm.row(i)` (mutating): returns `RowSlot::Ready(Arc<Row>)` on
     hit; on miss returns `RowSlot::Pending(Placeholder { id, index })`
     *and* enqueues a fetch of the missing page if not already
     loading. Used by the actor when handling a `Read` inbox message.
   - `vm.peek_row(i)` (read-only): same return shape but never
     enqueues, never bumps LRU, never bumps `version`. Used by the
     render path so screen redraws are pure projections.
   Screens render placeholders via the `Skeleton` primitive
   (`ui/primitives/skeleton.rs`). When a page arrives the actor
   bumps `version` and publishes a fresh snapshot; the GPUI bridge
   notifies the entity and the screen repaints.
5. **Direction-aware prefetch.** The screen reports its visible
   range each frame via `PagedTrackListMsg::ReportVisible`. The VM
   diffs against the previous range to infer scroll direction and
   pre-loads one page beyond the trailing edge. On a forward→backward
   flip the prefetch target switches to the page *behind* the
   visible range.
6. **Event-driven invalidation via `VmBus`.** The runtime broadcast
   bus (`runtime::VmBus` carrying `runtime::VmEvent`) wires
   `AsyncCommandRunner` outputs to actor inboxes. Mappings:
   - `VmEvent::TrackChanged { track_id }` → `vm.invalidate(id)`
     drops one cache entry; siblings stay hot. Next read for that id
     enqueues an *incremental* page request whose `ids` slice
     contains only the dropped id.
   - `VmEvent::InvalidateAll` → `actor.refresh_index()` rereads the
     identity index (e.g. for sort/filter changes or coarse
     `Library/Feed/Download/Metadata::Changed` events).
   - `VmEvent::PlaylistChanged { playlist_id }` /
     `VmEvent::FeedChanged { feed_id }` are routed to listings that
     depend on those scopes; the default `PagedTrackListActor` over
     `Library`/`Cached` listings ignores them.
   The actor inbox always wins under load: the spawn loop uses
   `tokio::select! { biased; ... }` with the inbox above the bus.
   On broadcast `Lagged`, the actor synthesises an `InvalidateAll`
   so caches drop conservatively.
7. **Sort and filter changes rebuild only the index.** Bodies stay
   cached because they are keyed by `Id`, not by position.
   `replace_index` clears `pending`/`requests`/`last_visible` and
   resets direction; the LRU survives.

### `PagedListVm` API (shipped)

```rust
pub enum RowSlot<Id, Row> {
    Pending(Placeholder<Id>),
    Ready(Arc<Row>),
}

pub struct Placeholder<Id> { pub id: Id, pub index: usize }

pub struct PageRequest<Id> { pub page: usize, pub ids: Vec<Id> }

pub struct PagedListVm<Id, Row>
where Id: Eq + Hash + Copy { /* … */ }

impl<Id: Eq + Hash + Copy, Row> PagedListVm<Id, Row> {
    pub fn new(index: Vec<Id>) -> Self;
    pub fn with_capacity(index: Vec<Id>, page_size: usize, cache: usize) -> Self;

    pub fn total(&self) -> usize;
    pub const fn version(&self) -> u64;
    pub const fn page_size(&self) -> usize;

    // Mutating reads (actor side).
    pub fn row(&mut self, index: usize) -> RowSlot<Id, Row>;
    pub fn report_visible(&mut self, range: Range<usize>);
    pub fn drain_requests(&mut self) -> Vec<PageRequest<Id>>;
    pub fn fulfill_page(&mut self, page: usize, rows: impl IntoIterator<Item = (Id, Row)>);

    // Read-only (render side).
    pub fn peek_row(&self, index: usize) -> RowSlot<Id, Row>;

    // Event-driven mutations.
    pub fn invalidate(&mut self, id: Id);
    pub fn insert(&mut self, position: usize, id: Id, body: Option<Row>);
    pub fn remove(&mut self, id: Id);
    pub fn replace_index(&mut self, index: Vec<Id>);
}
```

The monotonically increasing `version()` is the snapshot signal. The
actor wraps the VM in an `ActorHandle` whose `tokio::sync::watch`
channel emits `PagedTrackListSnapshot { version, total }` so the
GPUI bridge (`presentation::gpui_vm_bridge::bridge_watch`) can
trigger `cx.notify()` only when something observable changed.

The `Sort` generic from the original sketch was dropped: sort order
is resolved inside the db helper that produces `Vec<Id>`. Bodies
remain `Id`-keyed, so the LRU survives `replace_index`.

### DB query contract

For every paged collection, `db.rs` exposes two helpers:

```rust
fn track_ids_ordered_by(conn: &Connection, listing: TrackListing) -> Result<Vec<i64>>;
fn tracks_by_ids(conn: &Connection, ids: &[i64]) -> Result<Vec<TrackRow>>;
```

`tracks_by_ids` uses `WHERE id IN (?,?,…)` chunked at 500 ids per
SQL call to stay under SQLite's `SQLITE_MAX_VARIABLE_NUMBER`
(default 999). Existing single-`SELECT` helpers (`cached_tracks`,
etc.) remain for CLI / tests; they stay as-is rather than wrapping
the new path because the CLI does not benefit from windowing.

### Skeleton primitive

`ui/primitives/skeleton.rs` adds a low-contrast, scale-aware row that
renders the same height as a real row. No animation in v1 (avoids a
permanent foreground tick). Visual: muted background + a pair of
short bars sized via the row's expected `Label` widths.

## Consequences

Positive:

- Cold open is one `COUNT(*)` + one cheap id query. UI paints in
  milliseconds regardless of library size.
- Smooth scroll on slow disks because adjacent pages prefetch.
- Bounded memory: 100k-track library uses the same cache footprint
  as 1k.
- One mutation does not reload the list — `VmEvent::TrackChanged{id}`
  invalidates one entry; the next read on that row triggers an
  incremental SQL fetch whose `IN (?,?,…)` slice contains only the
  dropped id.
- The screen's `TableDelegate` shape is unchanged; only `rows_count()`
  and `render_td` indirect through `vm.peek_row(i)`.

Negative:

- Two new query shapes per paged collection. Acceptable; mechanical.
- A `Skeleton` primitive must exist before any paged screen ships
  (shipped at `ui/primitives/skeleton.rs`).
- The `RowSlot::{Pending, Ready}` return makes `render_td` slightly
  more verbose. Mitigated by a `match` helper in the screen.
- LRU + index together still cost memory; defaults
  (`DEFAULT_CACHE_PAGES * DEFAULT_PAGE_SIZE = 256` rows) must be
  tuned per measured workload.

## Implementation notes

- Defaults: `DEFAULT_PAGE_SIZE = 64`,
  `DEFAULT_CACHE_PAGES = 4` (cache holds 256 rows). Override per
  actor where measured behaviour demands it.
- Sort order is resolved inside the db helper, not the VM. Switching
  sort modes calls `replace_index(new_ids)` and keeps the body
  cache hot.
- Chunk size for `WHERE id IN (?,?,…)` is `500` (SQLite default
  `SQLITE_MAX_VARIABLE_NUMBER` is 999; 500 leaves room for other
  bound params).
- `report_visible` is the only place a screen mutates the actor each
  frame. Cheap: it's a tagged `Range<usize>` over an `mpsc`. Render
  itself uses `peek_row` and never holds the actor inbox.
- Placeholders carry the stable `id` and the eager `index`. The
  screen renders a `Skeleton` row (no shimmer per ADR 0034 / Apple
  HIG analogue of `.redacted(.placeholder)`); when the page fulfils,
  the same id transitions to `Ready(Arc<Row>)` under the same
  position.
- `tokio::select!` on the actor's spawn loop is `biased`. Inbox
  handling wins under load; the bus is consulted second. On a
  broadcast `Lagged` the actor calls `handle_event(InvalidateAll)`;
  on `Closed` it resubscribes (VmBus owners outlive actors in
  practice).
- Tests covering the contract live in `runtime::paged_list_vm`:
  direction flip, mid-page invalidation (incremental fill),
  placeholder→ready transition, full-page invalidation,
  empty/overshoot visible ranges, LRU eviction, and `peek_row`
  read-only invariants.
