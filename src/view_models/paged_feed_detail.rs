//! Paged-backed feed detail view-model (ADR 0041 Phase E).
//!
//! Mirror of [`super::paged_playlist_detail`] for feed-scoped track
//! listings. Reads row bodies through a windowed [`PagedListVm`]
//! managed by an actor on the async runtime, rather than holding a
//! `Vec<TrackRow>`. The screen layer is responsible for sending
//! [`crate::application::paged_track_list`] inbox messages
//! (`ReportVisible`, `Read`) so prefetch keeps up with scroll; this VM
//! is **read-only** and never mutates the underlying [`PagedListVm`].
//!
//! Same layer rules as the rest of `view_models/`: no GPUI, no service
//! mutation, constructed fresh each render. Behind `async-runtime`.

#![cfg(feature = "async-runtime")]
#![warn(clippy::pedantic)]
// Scaffold for `library-track-list-paged-vm`: the screen layer
// (`ui/shells/library/feed_detail.rs`) will consume these items in
// the follow-up slice. Suppressed here to keep the clippy gate green
// while the parallel/additive path is in flight.
#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use crate::db::{self, TrackRow};
use crate::runtime::paged_list_vm::{PagedListVm, RowSlot};

/// Backing handle the screen owner shares with the actor.
pub(crate) type PagedTrackListHandle = Arc<Mutex<PagedListVm<i64, TrackRow>>>;

/// Display-ready projection of a single paged row. Pending rows carry
/// only their position so the screen can paint a skeleton.
#[derive(Clone, Debug)]
pub(crate) enum PagedFeedRow {
    /// Row body has not been fetched yet — render a skeleton placeholder.
    Pending { position: usize },
    /// Row body is cached and can be rendered.
    Ready {
        position: usize,
        last_position: usize,
        track: Arc<TrackRow>,
    },
}

impl PagedFeedRow {
    #[must_use]
    pub(crate) fn position(&self) -> usize {
        match self {
            PagedFeedRow::Pending { position } | PagedFeedRow::Ready { position, .. } => *position,
        }
    }

    #[must_use]
    pub(crate) fn is_pending(&self) -> bool {
        matches!(self, PagedFeedRow::Pending { .. })
    }
}

/// Paged feed detail VM. Cheap to construct each render.
pub(crate) struct PagedFeedDetailVm<'a> {
    feed: &'a db::FeedRow,
    backing: &'a PagedTrackListHandle,
}

impl<'a> PagedFeedDetailVm<'a> {
    pub(crate) fn new(feed: &'a db::FeedRow, backing: &'a PagedTrackListHandle) -> Self {
        Self { feed, backing }
    }

    #[must_use]
    pub(crate) fn feed_id(&self) -> i64 {
        self.feed.id
    }

    #[must_use]
    pub(crate) fn title(&self) -> &str {
        self.feed.title.as_deref().unwrap_or("")
    }

    /// Total row count from the eager identity index.
    ///
    /// # Panics
    ///
    /// Panics if the backing mutex has been poisoned by a previous
    /// thread crash. Callers run on the GPUI thread; a poisoned VM
    /// indicates an unrecoverable runtime bug.
    #[must_use]
    pub(crate) fn track_count(&self) -> usize {
        self.backing
            .lock()
            .expect("paged track list mutex poisoned")
            .total()
    }

    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.track_count() == 0
    }

    /// Read the row at `position` without mutating the backing VM.
    /// Returns [`PagedFeedRow::Pending`] on cache miss.
    ///
    /// # Panics
    ///
    /// Panics if `position >= self.track_count()` (consistent with
    /// [`PagedListVm::peek_row`]) or if the backing mutex is poisoned.
    #[must_use]
    pub(crate) fn row(&self, position: usize) -> PagedFeedRow {
        let guard = self
            .backing
            .lock()
            .expect("paged track list mutex poisoned");
        let last_position = guard.total().saturating_sub(1);
        match guard.peek_row(position) {
            RowSlot::Ready(track) => PagedFeedRow::Ready {
                position,
                last_position,
                track,
            },
            RowSlot::Pending(_) => PagedFeedRow::Pending { position },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::TrackRow;
    use crate::runtime::paged_list_vm::PagedListVm;

    fn feed(id: i64, title: &str) -> db::FeedRow {
        db::FeedRow {
            id,
            feed_url: format!("https://example.test/{id}.xml"),
            feed_guid: None,
            title: Some(title.to_string()),
            description: None,
            album_image_href: None,
            is_subscribed: true,
        }
    }

    fn track(id: i64, title: &str) -> TrackRow {
        TrackRow {
            id,
            track_title: Some(title.to_string()),
            ..Default::default()
        }
    }

    fn handle(ids: Vec<i64>) -> PagedTrackListHandle {
        Arc::new(Mutex::new(PagedListVm::new(ids)))
    }

    #[test]
    fn empty_backing_reports_zero_count() {
        let f = feed(1, "empty");
        let h = handle(Vec::new());
        let vm = PagedFeedDetailVm::new(&f, &h);
        assert_eq!(vm.track_count(), 0);
        assert!(vm.is_empty());
        assert_eq!(vm.feed_id(), 1);
        assert_eq!(vm.title(), "empty");
    }

    #[test]
    fn missing_feed_title_renders_as_empty_string() {
        let mut f = feed(2, "ignored");
        f.title = None;
        let h = handle(Vec::new());
        let vm = PagedFeedDetailVm::new(&f, &h);
        assert_eq!(vm.title(), "");
    }

    #[test]
    fn unfulfilled_row_is_pending() {
        let f = feed(7, "f");
        let h = handle(vec![10, 20, 30]);
        let vm = PagedFeedDetailVm::new(&f, &h);
        assert_eq!(vm.track_count(), 3);

        let row = vm.row(1);
        assert!(row.is_pending());
        assert_eq!(row.position(), 1);
    }

    #[test]
    fn fulfilled_row_is_ready_with_correct_positions() {
        let f = feed(1, "f");
        let h = handle(vec![10, 20, 30]);

        {
            let mut guard = h.lock().unwrap();
            let _ = guard.row(0);
            let _ = guard.drain_requests();
            guard.fulfill_page(
                0,
                vec![
                    (10, track(10, "a")),
                    (20, track(20, "b")),
                    (30, track(30, "c")),
                ],
            );
        }

        let vm = PagedFeedDetailVm::new(&f, &h);
        match vm.row(0) {
            PagedFeedRow::Ready {
                position,
                last_position,
                track,
            } => {
                assert_eq!(position, 0);
                assert_eq!(last_position, 2);
                assert_eq!(track.track_title.as_deref(), Some("a"));
            }
            PagedFeedRow::Pending { .. } => panic!("expected ready row"),
        }
        match vm.row(2) {
            PagedFeedRow::Ready {
                position,
                last_position,
                track,
            } => {
                assert_eq!(position, 2);
                assert_eq!(last_position, 2);
                assert_eq!(track.track_title.as_deref(), Some("c"));
            }
            PagedFeedRow::Pending { .. } => panic!("expected ready row"),
        }
    }

    #[test]
    fn render_path_does_not_queue_page_requests() {
        let f = feed(1, "f");
        let h = handle(vec![1, 2, 3, 4, 5]);
        let vm = PagedFeedDetailVm::new(&f, &h);
        let _ = vm.row(0);
        let _ = vm.row(4);
        assert!(h.lock().unwrap().drain_requests().is_empty());
    }
}
