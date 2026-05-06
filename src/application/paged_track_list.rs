//! `PagedTrackListActor` (ADR 0040 + 0041): the application-layer bridge
//! between `PagedListVm<i64, TrackRow>` and the synchronous `db` helpers.
//!
//! Behind the `async-runtime` Cargo feature.
//!
//! Lifecycle:
//!
//! - On startup the actor calls [`crate::db::track_ids_ordered_by`] to
//!   build the identity index, then publishes an initial `Snapshot`.
//! - UI sends [`PagedTrackListMsg::ReportVisible`] and
//!   [`PagedTrackListMsg::Read`]; both go through the wrapped
//!   `PagedListVm`. Any queued page requests are drained and serviced
//!   via [`crate::db::tracks_by_ids`].
//! - [`PagedTrackListMsg::Refresh`] re-reads the identity index (e.g.
//!   after large mutations).
//! - [`PagedTrackListMsg::Invalidate`] drops one cached body (e.g. on
//!   `TrackTagged`).
//!
//! The actor never imports `gpui*`. Screens own a
//! `watch::Receiver<Snapshot<PagedTrackListSnapshot>>` and a
//! `mpsc::Sender<PagedTrackListMsg>`; rendering happens in
//! `presentation::gpui_vm_bridge`.

#![warn(clippy::pedantic)]

use std::sync::{Arc, Mutex};

use crate::db::{self, TrackListing, TrackRow};
use crate::runtime::actor::{self, Actor, ActorHandle};
use crate::runtime::{PagedListVm, VmBus, VmEvent};
use rusqlite::Connection;

/// Inbox messages for [`PagedTrackListActor`].
#[derive(Debug)]
pub enum PagedTrackListMsg {
    /// Report the row range currently visible on screen. Drives
    /// direction-aware prefetch.
    ReportVisible(std::ops::Range<usize>),
    /// Touch row `index` to materialise it (and request its page on
    /// miss). The result becomes visible via the next snapshot.
    Read(usize),
    /// Drop a single cached row body; the next read will re-request
    /// the containing page.
    Invalidate(i64),
    /// Re-read the identity index from the database.
    Refresh,
}

/// Snapshot published by [`PagedTrackListActor`].
///
/// We hand the VM out by `Arc<Mutex<…>>` so the GPUI bridge can call
/// `vm.row(index)` from the render thread without copying every page.
/// Mutations are still serialised through the actor's inbox; the Mutex
/// only covers the single read-side `row` call.
pub type PagedTrackListSnapshot = Arc<Mutex<PagedListVm<i64, TrackRow>>>;

/// Long-running actor owning the windowed track list.
pub struct PagedTrackListActor {
    conn: Connection,
    listing: TrackListing,
    vm: PagedTrackListSnapshot,
}

impl PagedTrackListActor {
    /// Build (but don't yet spawn) the actor.
    ///
    /// # Errors
    ///
    /// Returns any error from the initial identity-index query.
    pub fn new(conn: Connection, listing: TrackListing) -> anyhow::Result<Self> {
        let index = db::track_ids_ordered_by(&conn, listing)?;
        let ids: Vec<i64> = index.into_iter().map(|(id, _)| id).collect();
        let vm = Arc::new(Mutex::new(PagedListVm::new(ids)));
        Ok(Self { conn, listing, vm })
    }

    /// Spawn the actor on the runtime; returns the inbox handle.
    ///
    /// # Panics
    ///
    /// Panics if the runtime has already been shut down.
    #[must_use]
    pub fn spawn(self, bus: VmBus) -> ActorHandle<PagedTrackListMsg, PagedTrackListSnapshot> {
        actor::spawn(self, bus)
    }

    fn drain_and_fulfill(&mut self) {
        let requests = {
            let mut vm = self.vm.lock().expect("paged track vm mutex poisoned");
            vm.drain_requests()
        };
        for request in requests {
            match db::tracks_by_ids(&self.conn, &request.ids) {
                Ok(rows) => {
                    let mut vm = self.vm.lock().expect("paged track vm mutex poisoned");
                    vm.fulfill_page(request.page, rows.into_iter().map(|r| (r.id, r)));
                }
                Err(err) => {
                    eprintln!(
                        "v4vmm::runtime: tracks_by_ids failed (page={}): {err}",
                        request.page
                    );
                }
            }
        }
    }

    fn refresh_index(&mut self) -> anyhow::Result<()> {
        let index = db::track_ids_ordered_by(&self.conn, self.listing)?;
        let ids: Vec<i64> = index.into_iter().map(|(id, _)| id).collect();
        let mut vm = self.vm.lock().expect("paged track vm mutex poisoned");
        vm.replace_index(ids);
        Ok(())
    }
}

impl Actor for PagedTrackListActor {
    type Message = PagedTrackListMsg;
    type State = PagedTrackListSnapshot;

    fn initial_state(&self) -> Self::State {
        Arc::clone(&self.vm)
    }

    fn handle(&mut self, msg: Self::Message, _bus: &VmBus) -> Option<Self::State> {
        let mut changed = false;
        match msg {
            PagedTrackListMsg::ReportVisible(range) => {
                let prev = {
                    let mut vm = self.vm.lock().expect("paged track vm mutex poisoned");
                    let v = vm.version();
                    vm.report_visible(range);
                    (v, vm.version())
                };
                changed = prev.0 != prev.1;
                self.drain_and_fulfill();
            }
            PagedTrackListMsg::Read(index) => {
                let total = self.vm.lock().expect("vm poisoned").total();
                if index < total {
                    let prev = {
                        let mut vm = self.vm.lock().expect("vm poisoned");
                        let v = vm.version();
                        let _ = vm.row(index);
                        (v, vm.version())
                    };
                    changed = prev.0 != prev.1;
                    self.drain_and_fulfill();
                }
            }
            PagedTrackListMsg::Invalidate(id) => {
                let prev = {
                    let mut vm = self.vm.lock().expect("vm poisoned");
                    let v = vm.version();
                    vm.invalidate(id);
                    (v, vm.version())
                };
                changed = prev.0 != prev.1;
            }
            PagedTrackListMsg::Refresh => {
                if let Err(err) = self.refresh_index() {
                    eprintln!("v4vmm::runtime: PagedTrackList refresh failed: {err}");
                } else {
                    changed = true;
                }
                self.drain_and_fulfill();
            }
        }

        if changed {
            Some(Arc::clone(&self.vm))
        } else {
            None
        }
    }

    /// Translate runtime invalidations into local cache mutations.
    ///
    /// * [`VmEvent::TrackChanged`] — drop the single cached row body
    ///   so the next read re-fetches the page.
    /// * [`VmEvent::InvalidateAll`] — re-read the identity index from
    ///   the database (cached page bodies for surviving ids are kept
    ///   by [`PagedListVm::replace_index`]).
    /// * Other variants are ignored: `FeedChanged` and `PlaylistChanged`
    ///   never affect a `TrackListing::Library` / `Cached` ordering by
    ///   themselves; if they ever should, callers can send a
    ///   [`PagedTrackListMsg::Refresh`] explicitly.
    fn handle_event(&mut self, event: VmEvent, _bus: &VmBus) -> Option<Self::State> {
        let changed = match event {
            VmEvent::TrackChanged { track_id } => {
                let mut vm = self.vm.lock().expect("paged track vm mutex poisoned");
                let v0 = vm.version();
                vm.invalidate(track_id);
                vm.version() != v0
            }
            VmEvent::InvalidateAll => {
                if let Err(err) = self.refresh_index() {
                    eprintln!("v4vmm::runtime: PagedTrackList InvalidateAll refresh failed: {err}");
                    return None;
                }
                true
            }
            VmEvent::FeedChanged { .. } | VmEvent::PlaylistChanged { .. } => false,
        };

        if changed {
            Some(Arc::clone(&self.vm))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{init_schema, migrate_schema};
    use crate::runtime::RowSlot;

    fn open_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        init_schema(&conn).unwrap();
        migrate_schema(&conn).unwrap();
        conn.execute(
            "INSERT INTO feeds (feed_url, title) VALUES ('http://t', 'Feed')",
            [],
        )
        .unwrap();
        let feed_id: i64 = conn.last_insert_rowid();
        for i in 0..200 {
            conn.execute(
                "INSERT INTO tracks (feed_id, item_guid, track_title, track_number, is_in_library)
                 VALUES (?1, ?2, ?3, ?4, 1)",
                rusqlite::params![feed_id, format!("g{i}"), format!("T{i:03}"), i],
            )
            .unwrap();
        }
        conn
    }

    #[test]
    fn new_builds_index_from_db() {
        let conn = open_db();
        let actor = PagedTrackListActor::new(conn, TrackListing::Library).unwrap();
        assert_eq!(actor.vm.lock().unwrap().total(), 200);
    }

    #[test]
    fn read_misses_then_drain_fulfills_page() {
        let conn = open_db();
        let mut actor = PagedTrackListActor::new(conn, TrackListing::Library).unwrap();

        let initial = match actor.vm.lock().unwrap().row(5) {
            RowSlot::Pending(_) => true,
            RowSlot::Ready(_) => false,
        };
        assert!(initial, "fresh actor should miss the cache");

        actor.drain_and_fulfill();

        match actor.vm.lock().unwrap().row(5) {
            RowSlot::Ready(_) => {}
            RowSlot::Pending(_) => panic!("after drain the page must be ready"),
        };
    }

    #[test]
    fn refresh_picks_up_new_rows() {
        let conn = open_db();
        let mut actor = PagedTrackListActor::new(conn, TrackListing::Library).unwrap();
        actor
            .conn
            .execute(
                "INSERT INTO tracks (feed_id, item_guid, track_title, track_number, is_in_library)
             VALUES (1, 'new', 'NEW', 999, 1)",
                [],
            )
            .unwrap();

        actor.refresh_index().unwrap();
        assert_eq!(actor.vm.lock().unwrap().total(), 201);
    }

    #[test]
    fn invalidate_drops_cached_body() {
        let conn = open_db();
        let mut actor = PagedTrackListActor::new(conn, TrackListing::Library).unwrap();

        // Read row 0 and let the actor's drain helper fulfil the page.
        actor.vm.lock().unwrap().row(0);
        actor.drain_and_fulfill();
        assert!(matches!(actor.vm.lock().unwrap().row(0), RowSlot::Ready(_)));

        // The id of the row we just cached is the first index entry.
        // (Tests below reach into VM internals via the public surface only.)
        let target_id = match actor.vm.lock().unwrap().row(0) {
            RowSlot::Ready(row) => row.id,
            RowSlot::Pending(_) => panic!("row 0 must be cached at this point"),
        };

        let mut vm = actor.vm.lock().unwrap();
        let v0 = vm.version();
        vm.invalidate(target_id);
        assert!(vm.version() > v0, "invalidate must bump version");
    }

    #[test]
    fn handle_event_track_changed_invalidates_single_id() {
        let conn = open_db();
        let mut actor = PagedTrackListActor::new(conn, TrackListing::Library).unwrap();
        actor.vm.lock().unwrap().row(0);
        actor.drain_and_fulfill();
        let target_id = match actor.vm.lock().unwrap().row(0) {
            RowSlot::Ready(row) => row.id,
            RowSlot::Pending(_) => panic!("row must be cached"),
        };
        let v0 = actor.vm.lock().unwrap().version();

        let bus = VmBus::new();
        let snapshot = actor.handle_event(
            VmEvent::TrackChanged {
                track_id: target_id,
            },
            &bus,
        );
        assert!(snapshot.is_some(), "track-change must publish a snapshot");
        assert!(actor.vm.lock().unwrap().version() > v0);
    }

    #[test]
    fn handle_event_invalidate_all_refreshes_index() {
        let conn = open_db();
        let mut actor = PagedTrackListActor::new(conn, TrackListing::Library).unwrap();
        let before = actor.vm.lock().unwrap().total();

        actor
            .conn
            .execute(
                "INSERT INTO tracks (feed_id, item_guid, track_title, track_number, is_in_library)
                 VALUES (1, 'extra', 'EXTRA', 9999, 1)",
                [],
            )
            .unwrap();

        let bus = VmBus::new();
        let snapshot = actor.handle_event(VmEvent::InvalidateAll, &bus);
        assert!(snapshot.is_some());
        assert_eq!(actor.vm.lock().unwrap().total(), before + 1);
    }

    #[test]
    fn handle_event_unrelated_variants_are_noop() {
        let conn = open_db();
        let mut actor = PagedTrackListActor::new(conn, TrackListing::Library).unwrap();
        let bus = VmBus::new();
        let v0 = actor.vm.lock().unwrap().version();
        assert!(actor
            .handle_event(VmEvent::FeedChanged { feed_id: 1 }, &bus)
            .is_none());
        assert!(actor
            .handle_event(VmEvent::PlaylistChanged { playlist_id: 1 }, &bus)
            .is_none());
        assert_eq!(actor.vm.lock().unwrap().version(), v0);
    }
}
