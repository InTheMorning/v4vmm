//! Generic windowed/paged view-model container (ADR 0041).
//!
//! `PagedListVm<Id, Row>` is the **I/O-free** core of any list-shaped
//! VM whose payload may exceed ~10k rows. It owns:
//!
//! * an **eager identity index** (`Vec<Id>` sorted however the caller
//!   chose),
//! * a **lazy LRU body cache** (`LruCache<Id, Arc<Row>>`),
//! * a **pending-page set** so duplicate requests are coalesced,
//! * a **request queue** the surrounding actor drains and satisfies,
//!   and
//! * a **version counter** screens diff against to know when to
//!   re-render.
//!
//! Loading is deliberately **not** owned here. The actor wrapper turns
//! `drain_requests()` output into DB queries on the tokio blocking pool
//! and feeds results back through `fulfill_page`. That keeps this
//! module sync, pure, and trivially testable.
//!
//! ### Layer rules
//!
//! Lives under `src/runtime/`. No GPUI imports. Gated behind the
//! `async-runtime` Cargo feature only because it is part of the runtime
//! module tree; it has no runtime-only dependencies otherwise.

#![warn(clippy::pedantic)]

use std::collections::{HashSet, VecDeque};
use std::hash::Hash;
use std::num::NonZeroUsize;
use std::ops::Range;
use std::sync::Arc;

use lru::LruCache;

/// Default page size when not configured.
pub const DEFAULT_PAGE_SIZE: usize = 64;
/// Default cache capacity in pages (multiplied by `page_size`).
pub const DEFAULT_CACHE_PAGES: usize = 4;

/// Slot returned from [`PagedListVm::row`].
#[derive(Debug, Clone)]
pub enum RowSlot<Id, Row> {
    /// Cache miss; the page has been requested and the screen should
    /// render a placeholder.
    Pending(Placeholder<Id>),
    /// Cache hit; the row body is available.
    Ready(Arc<Row>),
}

/// Identity of a row whose body has not yet been loaded.
#[derive(Debug, Clone, Copy)]
pub struct Placeholder<Id> {
    /// Stable row identifier (same value as the index entry).
    pub id: Id,
    /// Position within the eager index.
    pub index: usize,
}

/// A batch of row ids the surrounding actor must fetch.
#[derive(Debug, Clone)]
pub struct PageRequest<Id> {
    /// 0-based page index (rows `[page * page_size, (page + 1) * page_size)`).
    pub page: usize,
    /// Row ids contained in the page (in index order).
    pub ids: Vec<Id>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    None,
    Forward,
    Backward,
}

/// I/O-free paged list view-model.
pub struct PagedListVm<Id, Row>
where
    Id: Eq + Hash + Copy,
{
    index: Vec<Id>,
    cache: LruCache<Id, Arc<Row>>,
    pending: HashSet<usize>,
    requests: VecDeque<PageRequest<Id>>,
    page_size: usize,
    last_visible: Option<Range<usize>>,
    direction: Direction,
    version: u64,
}

impl<Id, Row> PagedListVm<Id, Row>
where
    Id: Eq + Hash + Copy,
{
    /// Construct a new VM from an eager identity index. Cache capacity
    /// defaults to [`DEFAULT_CACHE_PAGES`] × [`DEFAULT_PAGE_SIZE`].
    #[must_use]
    pub fn new(index: Vec<Id>) -> Self {
        Self::with_capacity(
            index,
            DEFAULT_PAGE_SIZE,
            DEFAULT_CACHE_PAGES * DEFAULT_PAGE_SIZE,
        )
    }

    /// Construct with explicit page size + cache capacity (number of
    /// row bodies, not pages).
    ///
    /// # Panics
    ///
    /// Panics if `page_size` is zero or `cache_capacity` is zero.
    #[must_use]
    pub fn with_capacity(index: Vec<Id>, page_size: usize, cache_capacity: usize) -> Self {
        let page_size = NonZeroUsize::new(page_size).expect("page_size > 0").get();
        let cache = LruCache::new(NonZeroUsize::new(cache_capacity).expect("cache > 0"));
        Self {
            index,
            cache,
            pending: HashSet::new(),
            requests: VecDeque::new(),
            page_size,
            last_visible: None,
            direction: Direction::None,
            version: 0,
        }
    }

    /// Total number of rows in the index.
    #[must_use]
    pub fn total(&self) -> usize {
        self.index.len()
    }

    /// Monotonic snapshot version. Bumped on any state change that
    /// affects rendered output.
    #[must_use]
    pub const fn version(&self) -> u64 {
        self.version
    }

    /// Configured page size.
    #[must_use]
    pub const fn page_size(&self) -> usize {
        self.page_size
    }

    /// Read the row at `index`. On cache miss, the page containing
    /// the row is requested (idempotent — duplicate requests coalesce).
    ///
    /// # Panics
    ///
    /// Panics if `index >= self.total()`.
    pub fn row(&mut self, index: usize) -> RowSlot<Id, Row> {
        let id = self.index[index];
        if let Some(row) = self.cache.get(&id) {
            return RowSlot::Ready(Arc::clone(row));
        }
        let page = index / self.page_size;
        self.request_page(page);
        RowSlot::Pending(Placeholder { id, index })
    }

    /// Read-only variant of [`Self::row`]. Returns the cached body if
    /// present without touching the LRU order or queuing a page
    /// request. Use this from the render path; the screen owner is
    /// responsible for separately driving prefetch through
    /// [`Self::report_visible`] on the actor inbox.
    ///
    /// # Panics
    ///
    /// Panics if `index >= self.total()`.
    #[must_use]
    pub fn peek_row(&self, index: usize) -> RowSlot<Id, Row> {
        let id = self.index[index];
        if let Some(row) = self.cache.peek(&id) {
            return RowSlot::Ready(Arc::clone(row));
        }
        RowSlot::Pending(Placeholder { id, index })
    }

    /// Inform the VM about the currently visible row range. Drives
    /// direction-aware prefetch.
    pub fn report_visible(&mut self, range: Range<usize>) {
        if range.is_empty() || self.index.is_empty() {
            self.last_visible = Some(range);
            return;
        }
        let start = range.start.min(self.index.len().saturating_sub(1));
        let end = range.end.min(self.index.len());
        let new = start..end;

        // Update direction.
        if let Some(prev) = self.last_visible.as_ref() {
            self.direction = match new.start.cmp(&prev.start) {
                std::cmp::Ordering::Greater => Direction::Forward,
                std::cmp::Ordering::Less => Direction::Backward,
                std::cmp::Ordering::Equal => self.direction,
            };
        }

        // Always request pages covering the visible range.
        let first_page = new.start / self.page_size;
        let last_page = new.end.saturating_sub(1) / self.page_size;
        for page in first_page..=last_page {
            self.request_page(page);
        }

        // Direction-aware prefetch: 1 page beyond the trailing edge.
        match self.direction {
            Direction::Forward => {
                self.request_page(last_page + 1);
            }
            Direction::Backward => {
                if first_page > 0 {
                    self.request_page(first_page - 1);
                }
            }
            Direction::None => {}
        }

        self.last_visible = Some(new);
    }

    /// Drain the queued page requests so the actor can issue the
    /// underlying `bodies_by_ids` query.
    pub fn drain_requests(&mut self) -> Vec<PageRequest<Id>> {
        self.requests.drain(..).collect()
    }

    /// Fulfill a previously requested page with the loaded row bodies.
    /// Bodies are inserted into the cache; the `Pending` mark is
    /// cleared. Bumps the version.
    pub fn fulfill_page(&mut self, page: usize, rows: impl IntoIterator<Item = (Id, Row)>) {
        let mut any = false;
        for (id, body) in rows {
            self.cache.put(id, Arc::new(body));
            any = true;
        }
        self.pending.remove(&page);
        if any {
            self.bump_version();
        }
    }

    /// Drop a single cache entry (e.g., on `TrackTagged`). Does not
    /// touch the index. Bumps the version only if something was evicted.
    pub fn invalidate(&mut self, id: Id) {
        if self.cache.pop(&id).is_some() {
            self.bump_version();
        }
    }

    /// Insert a new row at `position` in the index. The body, if
    /// supplied, is cached too. Bumps the version.
    ///
    /// # Panics
    ///
    /// Panics if `position > self.total()`.
    pub fn insert(&mut self, position: usize, id: Id, body: Option<Row>) {
        self.index.insert(position, id);
        if let Some(body) = body {
            self.cache.put(id, Arc::new(body));
        }
        // Pending pages keyed by index shift but membership is
        // idempotent — keeping the simple model. Worst case a page is
        // re-requested.
        self.pending.clear();
        self.requests.clear();
        self.last_visible = None;
        self.bump_version();
    }

    /// Remove a row by id. Bumps the version only if the id existed.
    pub fn remove(&mut self, id: Id) {
        if let Some(pos) = self.index.iter().position(|x| *x == id) {
            self.index.remove(pos);
            self.cache.pop(&id);
            self.pending.clear();
            self.requests.clear();
            self.last_visible = None;
            self.bump_version();
        }
    }

    /// Replace the identity index (e.g., on sort or filter change).
    /// The body cache is preserved because bodies are keyed by id.
    pub fn replace_index(&mut self, index: Vec<Id>) {
        self.index = index;
        self.pending.clear();
        self.requests.clear();
        self.last_visible = None;
        self.direction = Direction::None;
        self.bump_version();
    }

    fn request_page(&mut self, page: usize) {
        if self.index.is_empty() {
            return;
        }
        let start = page * self.page_size;
        if start >= self.index.len() {
            return;
        }
        if !self.pending.insert(page) {
            return;
        }
        let end = (start + self.page_size).min(self.index.len());
        // Skip ids already cached (incremental fill).
        let ids: Vec<Id> = self.index[start..end]
            .iter()
            .copied()
            .filter(|id| !self.cache.contains(id))
            .collect();
        if ids.is_empty() {
            // Already fully cached; clear the pending mark.
            self.pending.remove(&page);
            return;
        }
        self.requests.push_back(PageRequest { page, ids });
    }

    fn bump_version(&mut self) {
        self.version = self.version.wrapping_add(1);
    }
}

impl<Id, Row> std::fmt::Debug for PagedListVm<Id, Row>
where
    Id: Eq + Hash + Copy + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PagedListVm")
            .field("total", &self.index.len())
            .field("page_size", &self.page_size)
            .field("cache_len", &self.cache.len())
            .field("pending_pages", &self.pending.len())
            .field("queued_requests", &self.requests.len())
            .field("last_visible", &self.last_visible)
            .field("direction", &self.direction)
            .field("version", &self.version)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vm(n: usize, page_size: usize, cache: usize) -> PagedListVm<u64, String> {
        let ids: Vec<u64> = (0..n as u64).collect();
        PagedListVm::with_capacity(ids, page_size, cache)
    }

    #[test]
    fn new_vm_starts_empty_and_zero_versioned() {
        let vm: PagedListVm<u64, String> = PagedListVm::new(Vec::new());
        assert_eq!(vm.total(), 0);
        assert_eq!(vm.version(), 0);
    }

    #[test]
    fn row_miss_emits_placeholder_and_queues_one_page_request() {
        let mut vm = vm(200, 50, 200);
        let slot = vm.row(0);
        assert!(matches!(slot, RowSlot::Pending(_)));
        let reqs = vm.drain_requests();
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].page, 0);
        assert_eq!(reqs[0].ids.len(), 50);
    }

    #[test]
    fn duplicate_misses_in_same_page_coalesce() {
        let mut vm = vm(200, 50, 200);
        let _ = vm.row(0);
        let _ = vm.row(1);
        let _ = vm.row(49);
        let reqs = vm.drain_requests();
        assert_eq!(reqs.len(), 1);
    }

    #[test]
    fn fulfill_page_makes_subsequent_reads_ready_and_bumps_version() {
        let mut vm = vm(100, 10, 100);
        let _ = vm.row(0);
        let _ = vm.drain_requests();
        let v0 = vm.version();
        vm.fulfill_page(0, (0..10u64).map(|i| (i, format!("row{i}"))));
        assert!(vm.version() > v0);
        match vm.row(3) {
            RowSlot::Ready(s) => assert_eq!(&*s, "row3"),
            RowSlot::Pending(_) => panic!("expected ready"),
        }
    }

    #[test]
    fn report_visible_forward_prefetches_next_page() {
        let mut vm = vm(500, 50, 500);
        vm.report_visible(0..50);
        let _ = vm.drain_requests(); // first page only, no direction yet
        vm.report_visible(40..90);
        let reqs = vm.drain_requests();
        let pages: Vec<usize> = reqs.iter().map(|r| r.page).collect();
        assert!(pages.contains(&1), "page 1 covers visible end at 90");
        assert!(pages.contains(&2), "page 2 prefetched in forward direction");
    }

    #[test]
    fn report_visible_backward_prefetches_previous_page() {
        let mut vm = vm(500, 50, 500);
        vm.report_visible(200..250);
        let _ = vm.drain_requests();
        vm.report_visible(100..150);
        let reqs = vm.drain_requests();
        let pages: Vec<usize> = reqs.iter().map(|r| r.page).collect();
        assert!(pages.contains(&2), "page 2 covers visible 100..150");
        assert!(
            pages.contains(&1),
            "page 1 prefetched in backward direction"
        );
    }

    #[test]
    fn lru_evicts_when_capacity_reached() {
        let mut vm = vm(200, 10, 10);
        // Cache holds 10 rows max. Load page 0 (rows 0..10).
        let _ = vm.row(0);
        let _ = vm.drain_requests();
        vm.fulfill_page(0, (0..10u64).map(|i| (i, format!("a{i}"))));
        // Load page 5 (rows 50..60), evicts page 0 entirely.
        let _ = vm.row(50);
        let _ = vm.drain_requests();
        vm.fulfill_page(5, (50..60u64).map(|i| (i, format!("b{i}"))));
        // Reading row 0 again must miss.
        match vm.row(0) {
            RowSlot::Pending(_) => {}
            RowSlot::Ready(_) => panic!("expected eviction"),
        }
    }

    #[test]
    fn invalidate_drops_only_one_entry_and_bumps_version() {
        let mut vm = vm(50, 10, 50);
        let _ = vm.row(0);
        let _ = vm.drain_requests();
        vm.fulfill_page(0, (0..10u64).map(|i| (i, format!("x{i}"))));
        let v = vm.version();
        vm.invalidate(3);
        assert!(vm.version() > v);
        // Other rows remain cached.
        assert!(matches!(vm.row(0), RowSlot::Ready(_)));
        // The invalidated row is now pending.
        assert!(matches!(vm.row(3), RowSlot::Pending(_)));
    }

    #[test]
    fn invalidate_unknown_id_is_noop() {
        let mut vm = vm(10, 5, 10);
        let v = vm.version();
        vm.invalidate(999);
        assert_eq!(vm.version(), v);
    }

    #[test]
    fn insert_grows_index_and_bumps_version() {
        let mut vm: PagedListVm<u64, String> = PagedListVm::new(vec![1, 2, 3]);
        let v = vm.version();
        vm.insert(1, 99, Some("inserted".into()));
        assert_eq!(vm.total(), 4);
        assert!(vm.version() > v);
        match vm.row(1) {
            RowSlot::Ready(s) => assert_eq!(&*s, "inserted"),
            RowSlot::Pending(_) => panic!("body should have been cached"),
        }
    }

    #[test]
    fn remove_shrinks_index_and_bumps_version() {
        let mut vm: PagedListVm<u64, String> = PagedListVm::new(vec![1, 2, 3]);
        let v = vm.version();
        vm.remove(2);
        assert_eq!(vm.total(), 2);
        assert!(vm.version() > v);
    }

    #[test]
    fn replace_index_preserves_cached_bodies() {
        let mut vm = vm(20, 5, 20);
        let _ = vm.row(0);
        let _ = vm.drain_requests();
        vm.fulfill_page(0, (0..5u64).map(|i| (i, format!("k{i}"))));
        // New index reorders (puts 3 first).
        vm.replace_index(vec![3, 0, 1, 2, 4]);
        match vm.row(0) {
            RowSlot::Ready(s) => assert_eq!(&*s, "k3"),
            RowSlot::Pending(_) => panic!("body for id 3 should still be cached"),
        }
    }

    #[test]
    fn peek_row_does_not_queue_requests_or_change_version() {
        let mut vm = vm(100, 10, 100);
        let v0 = vm.version();
        let slot = vm.peek_row(0);
        assert!(matches!(slot, RowSlot::Pending(_)));
        assert!(
            vm.drain_requests().is_empty(),
            "peek_row must not queue page requests"
        );
        assert_eq!(vm.version(), v0, "peek_row must not bump version");
    }

    #[test]
    fn peek_row_returns_cached_body_when_present() {
        let mut vm = vm(100, 10, 100);
        let _ = vm.row(0);
        let _ = vm.drain_requests();
        vm.fulfill_page(0, (0..10u64).map(|i| (i, format!("row{i}"))));
        match vm.peek_row(3) {
            RowSlot::Ready(s) => assert_eq!(&*s, "row3"),
            RowSlot::Pending(_) => panic!("expected cache hit"),
        }
    }
}
