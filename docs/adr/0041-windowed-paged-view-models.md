# ADR 0041: Windowed Paged View-Models

## Status

Proposed — 2026-05-06.

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

1. **Eager identity index.** On first load the actor fetches
   `Vec<(Id, SortKey)>` via a dedicated cheap query
   (`fn ids_ordered_by(conn, sort) -> Vec<(Id, SortKey)>`).
   ~50–100 bytes per row. 100k rows ≈ 5–10 MB, acceptable.
2. **Lazy body cache.** Row bodies load in pages of N
   (default 64; configurable per actor). Each page is one
   `bodies_by_ids(conn, &[Id])` query bound to a page-shaped slice
   of the index.
3. **LRU eviction.** The cache is `LruCache<Id, Arc<Row>>` capped at
   `4 × visible_window_pages` by default. Pages outside the window
   evict automatically.
4. **Synchronous reads with placeholders.** `vm.row(i)` returns
   immediately:
   - `Either::Right(RowVm)` on cache hit;
   - `Either::Left(Placeholder { id, sort_key })` on miss, *and*
     enqueues a fetch of the missing page if not already loading.
   Screens render the placeholder via a `Skeleton` primitive
   (added in `ui/primitives/skeleton.rs`). When the page arrives the
   actor publishes a new snapshot version; the screen repaints next
   frame and renders the real row.
5. **Direction-aware prefetch.** The screen reports its visible
   range each frame. The actor diffs against the previous range to
   infer scroll direction and pre-loads the next page before it is
   asked for. On slow disks this is what hides latency.
6. **Event-driven invalidation.**
   `ApplicationEvent::TrackTagged { id }` invalidates one cache entry,
   not the list. `TrackAdded { id, sort_key }` inserts into the index
   in sort order; no SELECT. `TrackRemoved { id }` removes from index
   and cache. The actor subscribes to the relevant `ApplicationEvent`
   families and applies these directly.
7. **Sort and filter changes rebuild only the index.** Bodies stay
   cached because they are keyed by `Id`, not by position.

### `PagedListVm` API (sketch)

```rust
pub struct PagedListVm<Id, Row, Sort> { /* … */ }

impl<Id: Hash + Eq + Copy + Send + 'static,
     Row: Clone + Send + 'static,
     Sort: Ord + Clone + Send + 'static>
    PagedListVm<Id, Row, Sort>
{
    pub fn total(&self) -> usize;
    pub fn row(&mut self, index: usize) -> Either<Placeholder<Id, Sort>, Arc<Row>>;
    pub fn report_visible(&mut self, range: Range<usize>);
    pub fn invalidate(&mut self, id: Id);
    pub fn insert(&mut self, id: Id, sort_key: Sort);
    pub fn remove(&mut self, id: Id);
    pub fn snapshot(&self) -> watch::Receiver<Version>;
}
```

A `Version` counter on the snapshot lets screens detect changes
without inspecting the cache.

### DB query contract

For every paged collection, `db.rs` exposes two helpers:

```rust
fn ids_ordered_by(conn: &Connection, sort: Sort) -> Result<Vec<(Id, SortKey)>>;
fn bodies_by_ids(conn: &Connection, ids: &[Id]) -> Result<Vec<Row>>;
```

`bodies_by_ids` uses `WHERE id IN (?,?,…)` with chunking at ~500 ids
to stay under SQLite's variable limit. Existing single-`SELECT`
helpers (`cached_tracks`, etc.) remain for CLI / tests; they become
thin wrappers that read the index then call `bodies_by_ids` with all
ids.

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
- One mutation does not reload the list — `TrackTagged` invalidates
  one entry.
- The screen's `TableDelegate` shape is unchanged; only `rows_count()`
  and `render_td` indirect through `vm.row(i)`.

Negative:

- Two new query shapes per paged collection. Acceptable; mechanical.
- A `Skeleton` primitive must exist before any paged screen ships.
  Trivial.
- The `Either<Placeholder, Row>` return makes `render_td` slightly
  more verbose. Mitigated by a `match` helper in the screen.
- LRU + index together still cost memory; defaults must be tuned.

## Implementation notes

- Suggested defaults: `page_size = 64`, `lru_capacity = page_size *
  4`. Override per actor where measured behaviour demands it.
- `Sort` is generic so we can support multiple sort modes without
  losing static dispatch. Common bound: `Ord + Clone + Send + 'static`.
- Chunk size for `WHERE id IN (?,?,…)` is `500` (SQLite default
  `SQLITE_MAX_VARIABLE_NUMBER` is 999; 500 leaves room for other
  bound params).
- `report_visible` is the only place a screen mutates the actor each
  frame. Cheap: it's a tagged `Range<usize>` over a `mpsc`.
- Placeholders carry `sort_key` so the row can render *some*
  meaningful display (e.g. track title prefix) without waiting for
  the body. Index can be augmented with up to ~32 bytes per row of
  "skeleton-friendly" hints if measurement shows it improves UX.
- Tests: fake-clock + assert that `vm.row(i)` for an unloaded page
  produces exactly one fetch task; that scrolling forward triggers
  prefetch of `page+1`; that `invalidate(id)` does not refetch the
  whole page.
