//! Paged-backed playlist detail view-model (ADR 0041 Phase E).
//!
//! Parallel/additive to [`super::library::PlaylistTrackRowVm`]: this
//! variant reads row bodies through a windowed [`PagedListVm`] managed
//! by an actor on the async runtime, rather than holding a
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
// (`ui/shells/library/playlist_detail.rs`) will consume these items in
// the follow-up slice. Suppressed here to keep the clippy gate green
// while the parallel/additive path is in flight.
#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use crate::db::{self, TrackRow};
use crate::runtime::paged_list_vm::{PagedListVm, RowSlot};

/// Backing handle the screen owner shares with the actor.
pub(crate) type PagedTrackListHandle = Arc<Mutex<PagedListVm<i64, TrackRow>>>;

/// Display-ready projection of a single paged row. Mirrors the shape
/// of [`super::library::PlaylistTrackRowVm`] for ready rows; pending
/// rows carry only their position so the screen can paint a skeleton.
#[derive(Clone, Debug)]
pub(crate) enum PagedPlaylistRow {
    /// Row body has not been fetched yet — render a skeleton placeholder.
    Pending { position: usize },
    /// Row body is cached and can be rendered.
    Ready {
        position: usize,
        last_position: usize,
        track: Arc<TrackRow>,
    },
}

impl PagedPlaylistRow {
    #[must_use]
    pub(crate) fn position(&self) -> usize {
        match self {
            PagedPlaylistRow::Pending { position } | PagedPlaylistRow::Ready { position, .. } => {
                *position
            }
        }
    }

    #[must_use]
    pub(crate) fn is_pending(&self) -> bool {
        matches!(self, PagedPlaylistRow::Pending { .. })
    }
}

/// Paged playlist detail VM. Cheap to construct each render.
pub(crate) struct PagedPlaylistDetailVm<'a> {
    playlist: &'a db::Playlist,
    backing: &'a PagedTrackListHandle,
}

impl<'a> PagedPlaylistDetailVm<'a> {
    pub(crate) fn new(playlist: &'a db::Playlist, backing: &'a PagedTrackListHandle) -> Self {
        Self { playlist, backing }
    }

    #[must_use]
    pub(crate) fn playlist_id(&self) -> i64 {
        self.playlist.id
    }

    #[must_use]
    pub(crate) fn title(&self) -> &str {
        &self.playlist.name
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
    /// Returns [`PagedPlaylistRow::Pending`] on cache miss.
    ///
    /// # Panics
    ///
    /// Panics if `position >= self.track_count()` (consistent with
    /// [`PagedListVm::peek_row`]) or if the backing mutex is poisoned.
    #[must_use]
    pub(crate) fn row(&self, position: usize) -> PagedPlaylistRow {
        let guard = self
            .backing
            .lock()
            .expect("paged track list mutex poisoned");
        let last_position = guard.total().saturating_sub(1);
        match guard.peek_row(position) {
            RowSlot::Ready(track) => PagedPlaylistRow::Ready {
                position,
                last_position,
                track,
            },
            RowSlot::Pending(_) => PagedPlaylistRow::Pending { position },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::TrackRow;
    use crate::runtime::paged_list_vm::PagedListVm;

    fn playlist(id: i64, name: &str) -> db::Playlist {
        db::Playlist {
            id,
            name: name.to_string(),
            description: None,
            track_count: 0,
            created_at: 0,
            updated_at: 0,
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
        let pl = playlist(1, "empty");
        let h = handle(Vec::new());
        let vm = PagedPlaylistDetailVm::new(&pl, &h);
        assert_eq!(vm.track_count(), 0);
        assert!(vm.is_empty());
        assert_eq!(vm.playlist_id(), 1);
        assert_eq!(vm.title(), "empty");
    }

    #[test]
    fn unfulfilled_row_is_pending() {
        let pl = playlist(7, "p");
        let h = handle(vec![10, 20, 30]);
        let vm = PagedPlaylistDetailVm::new(&pl, &h);
        assert_eq!(vm.track_count(), 3);

        let row = vm.row(1);
        assert!(row.is_pending());
        assert_eq!(row.position(), 1);
    }

    #[test]
    fn fulfilled_row_is_ready_with_correct_positions() {
        let pl = playlist(1, "p");
        let h = handle(vec![10, 20, 30]);

        // Simulate the actor's drain/fulfill cycle.
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

        let vm = PagedPlaylistDetailVm::new(&pl, &h);
        match vm.row(0) {
            PagedPlaylistRow::Ready {
                position,
                last_position,
                track,
            } => {
                assert_eq!(position, 0);
                assert_eq!(last_position, 2);
                assert_eq!(track.track_title.as_deref(), Some("a"));
            }
            PagedPlaylistRow::Pending { .. } => panic!("expected ready row"),
        }
        match vm.row(2) {
            PagedPlaylistRow::Ready {
                position,
                last_position,
                track,
            } => {
                assert_eq!(position, 2);
                assert_eq!(last_position, 2);
                assert_eq!(track.track_title.as_deref(), Some("c"));
            }
            PagedPlaylistRow::Pending { .. } => panic!("expected ready row"),
        }
    }

    #[test]
    fn render_path_does_not_queue_page_requests() {
        let pl = playlist(1, "p");
        let h = handle(vec![1, 2, 3, 4, 5]);
        let vm = PagedPlaylistDetailVm::new(&pl, &h);
        let _ = vm.row(0);
        let _ = vm.row(4);
        // peek_row must not have queued anything.
        assert!(h.lock().unwrap().drain_requests().is_empty());
    }

    #[test]
    fn mixed_pending_and_ready_in_partial_fulfillment() {
        // Mirrors the paged screen render: actor has fulfilled the first
        // page; later rows are still pending. The screen must see both
        // states without crashing or queueing requests on the render path.
        let pl = playlist(1, "p");
        let h = handle((0..200).collect());
        // Fulfill only positions 0..3 to simulate a partial first page.
        {
            let mut guard = h.lock().unwrap();
            let _ = guard.row(0);
            let _ = guard.drain_requests();
            guard.fulfill_page(
                0,
                (0..3i64).map(|id| (id, track(id, &format!("t{id}")))),
            );
        }

        let vm = PagedPlaylistDetailVm::new(&pl, &h);
        assert_eq!(vm.track_count(), 200);

        // Ready in the warm window.
        for pos in 0..3usize {
            let row = vm.row(pos);
            assert!(matches!(row, PagedPlaylistRow::Ready { .. }), "pos {pos}");
        }
        // Pending past the warm window.
        for pos in [50usize, 150, 199] {
            let row = vm.row(pos);
            assert!(
                matches!(row, PagedPlaylistRow::Pending { .. }),
                "pos {pos}"
            );
        }
        // Render path is read-only — no requests queued by peeking.
        assert!(h.lock().unwrap().drain_requests().is_empty());
    }
}
