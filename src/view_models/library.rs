//! Library screen view-models.
//!
//! Pure projections of [`db::TrackRow`] + library-screen-owned state
//! ([`MbTrackStatus`]) into the strings the library inspector and album
//! detail rows render. Same layer rules as [`super`]: no GPUI imports,
//! no service mutation; constructed fresh each render.
//!
//! The album detail track row was the first call site to migrate, so
//! its projection ([`LibraryTrackRowVm`]) lives here. Future entries
//! (artist node summary, playlist row, `MusicBrainz` panel header) will
//! join as `library.rs` is whittled down.
//!
//! Per ADR 0023, this layer must not import screen modules. The
//! per-track `MusicBrainz` lookup state therefore lives here as
//! [`MbTrackStatus`]; the library screen depends on the view-model for
//! the type, not the other way around.

#![warn(clippy::pedantic)]

use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;

use crate::db::{self, TrackRow};
use crate::feed_service;
use crate::metadata::MusicBrainzLookupResult;
use crate::view_models::entity_detail::{
    EntityActionTarget, EntityActionVm, PlaylistActionState, ReleaseActionState,
    ReleaseMembershipState, TrackActionState, TrackMembershipState,
};
use crate::view_models::format::{fmt_total_runtime_clock, plural};
use crate::view_models::SplitPaneState;
use crate::views::{FeedRef, FeedView, TrackRef};

const DEFAULT_SPLIT_PANE_WIDTH: f32 = 360.0;

/// Per-track `MusicBrainz` lookup state owned by the library screen and
/// projected into display by [`LibraryTrackRowVm`].
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) enum MbTrackStatus {
    Pending,
    Processing,
    Done(usize),
    Skipped(String),
}

/// Display-ready projection of a [`TrackRow`] in the library album
/// detail listing.
///
/// Borrow-only — constructed fresh each render and dropped before the
/// element tree is painted.
pub(crate) struct LibraryTrackRowVm<'a> {
    track: &'a TrackRow,
    mb: Option<&'a MbTrackStatus>,
}

/// Semantic colour bucket for the `MusicBrainz` status hint. The screen
/// maps each variant to a token at render time, keeping the VM free of
/// GPUI types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MbStatusKind {
    Success,
    Warning,
    Danger,
    Muted,
}

/// One artist node in the library sidebar tree. Owns a flat list of
/// album nodes; the screen handles expansion and rendering.
#[derive(Clone, Debug)]
pub(crate) struct ArtistNode {
    pub(crate) name: String,
    pub(crate) albums: Vec<AlbumNode>,
}

/// One album node — a feed in podcast terms — under an artist. Holds
/// the embedded track rows so the album-detail panel can render
/// without re-querying the DB.
#[derive(Clone, Debug)]
pub(crate) struct AlbumNode {
    pub(crate) name: String,
    pub(crate) feed_id: Option<i64>,
    pub(crate) feed_url: Option<String>,
    pub(crate) image_href: Option<String>,
    pub(crate) tracks: Vec<TrackRow>,
}

/// Top-level structure of the library sidebar — a list of artist
/// nodes, each with their albums.
#[derive(Clone, Debug, Default)]
pub(crate) struct LibraryTree {
    pub(crate) artists: Vec<ArtistNode>,
}

/// Pure data snapshots loaded by the library screen.
///
/// This groups DB-derived read models and in-flight metadata/feed state
/// behind the view-model boundary. It intentionally carries no GPUI
/// framework values; image caches and subscriptions stay in `LibraryApp`.
#[derive(Clone, Debug, Default)]
pub(crate) struct LibrarySnapshot {
    tree: LibraryTree,
    playlists: Vec<db::Playlist>,
    playlist_tracks: Vec<TrackRow>,
    mb_status: BTreeMap<i64, MbTrackStatus>,
    staged_musicbrainz: BTreeMap<i64, MusicBrainzLookupResult>,
    in_flight_feed_checks: HashSet<i64>,
    feed_update_state: FeedUpdateState,
}

/// Projected library tree plus the expansion state needed to render it.
#[derive(Clone, Debug)]
pub(crate) struct LibraryTreeProjection {
    pub(crate) tree: LibraryTree,
    pub(crate) expanded_artists: HashSet<String>,
    pub(crate) expanded_albums: HashSet<(String, String)>,
}

impl LibraryTreeProjection {
    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.tree.artists.is_empty()
    }
}

/// Display-ready projection for the playlist sidebar section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlaylistSidebarVm {
    pub(crate) expanded: bool,
    pub(crate) disclosure_glyph: &'static str,
    pub(crate) sort_label: &'static str,
    pub(crate) creating_playlist: bool,
    pub(crate) rows: Vec<PlaylistSidebarRowVm>,
}

/// One playlist row in the library sidebar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlaylistSidebarRowVm {
    pub(crate) id: i64,
    pub(crate) element_id: String,
    pub(crate) name: String,
    pub(crate) track_count_label: String,
    pub(crate) selected: bool,
}

/// Pure command intent for appending one or more library tracks to a playlist.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlaylistAppendIntent {
    playlist_id: i64,
    track_ids: Vec<i64>,
    playlist_name: String,
}

/// Pure result counts for a completed playlist append command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PlaylistAppendOutcome {
    appended: usize,
    downloaded: usize,
    failed: usize,
}

impl PlaylistAppendOutcome {
    #[must_use]
    pub(crate) fn new(appended: usize, downloaded: usize, failed: usize) -> Self {
        Self {
            appended,
            downloaded,
            failed,
        }
    }
}

/// Pure result data for a completed library track subscription.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TrackSubscribeOutcome {
    path_label: String,
    format_warning: Option<String>,
}

impl TrackSubscribeOutcome {
    #[must_use]
    pub(crate) fn new(path_label: impl Into<String>, format_warning: Option<String>) -> Self {
        Self {
            path_label: path_label.into(),
            format_warning,
        }
    }
}

/// Display/status projection for the library track action row.
///
/// The screen owns click handlers and panel rendering; this VM owns the
/// button labels and subscription-message classification.
pub(crate) struct LibraryTrackActionVm<'a> {
    subscription_busy: bool,
    local_subscription: bool,
    add_to_playlist_open: bool,
    subscription_message: Option<&'a str>,
}

impl<'a> LibraryTrackActionVm<'a> {
    #[must_use]
    pub(crate) fn new(
        subscription_busy: bool,
        local_subscription: bool,
        add_to_playlist_open: bool,
        subscription_message: Option<&'a str>,
    ) -> Self {
        Self {
            subscription_busy,
            local_subscription,
            add_to_playlist_open,
            subscription_message,
        }
    }

    #[must_use]
    pub(crate) fn subscription_button_label(&self) -> &'static str {
        match (self.subscription_busy, self.local_subscription) {
            (true, true) => "Unsubscribing...",
            (true, false) => "Subscribing...",
            (false, true) => "Unsubscribe Track",
            (false, false) => "Subscribe Track",
        }
    }

    #[must_use]
    pub(crate) fn add_to_playlist_label(&self) -> &'static str {
        if self.add_to_playlist_open {
            "Add to playlist ▴"
        } else {
            "Add to playlist ▾"
        }
    }

    #[must_use]
    pub(crate) fn subscription_message(&self) -> Option<&str> {
        self.subscription_message
    }

    #[must_use]
    pub(crate) fn message_is_error(&self) -> bool {
        self.subscription_message
            .is_some_and(|message| message.to_lowercase().contains("error"))
    }
}

impl PlaylistAppendIntent {
    #[must_use]
    pub(crate) fn playlist_id(&self) -> i64 {
        self.playlist_id
    }

    #[must_use]
    pub(crate) fn playlist_name(&self) -> &str {
        &self.playlist_name
    }

    #[must_use]
    pub(crate) fn total_tracks(&self) -> usize {
        self.track_ids.len()
    }

    #[must_use]
    pub(crate) fn track_ids(&self) -> &[i64] {
        &self.track_ids
    }
}

/// Snapshot of the multi-feed update workflow exposed by
/// `feed_service`. Owned by the library view-model; the screen reads
/// `phase` to decide whether to show progress vs. results.
#[derive(Clone, Debug, Default)]
pub struct FeedUpdateState {
    pub phase: FeedUpdatePhase,
    pub status_message: Option<String>,
    pub stale: Vec<feed_service::StaleFeed>,
}

/// Lifecycle of a feed-update workflow.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum FeedUpdatePhase {
    #[default]
    Idle,
    Checking,
    Applying,
}

/// Sort order for the library's playlist sidebar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum PlaylistSort {
    /// Alphabetical by playlist name.
    #[default]
    Name,
    /// Most-recently-updated playlists first.
    RecentlyUpdated,
    /// Largest playlists first by track count.
    TrackCount,
}

impl PlaylistSort {
    /// Cycle to the next sort order. Wraps `TrackCount` back to `Name`.
    #[must_use]
    pub(crate) fn next(self) -> Self {
        match self {
            PlaylistSort::Name => PlaylistSort::RecentlyUpdated,
            PlaylistSort::RecentlyUpdated => PlaylistSort::TrackCount,
            PlaylistSort::TrackCount => PlaylistSort::Name,
        }
    }

    /// Short button label for the sort cycler.
    #[must_use]
    pub(crate) fn label(self) -> &'static str {
        match self {
            PlaylistSort::Name => "A–Z",
            PlaylistSort::RecentlyUpdated => "Recent",
            PlaylistSort::TrackCount => "Size",
        }
    }
}

/// Stateful screen view-model for the library tab.
///
/// `LibraryViewModel` is the [SwiftUI `@Observable`-style](https://developer.apple.com/documentation/observation)
/// adapter between the GPUI `LibraryApp` and the rest of the layered
/// architecture: it owns the screen's *pure* UI state (selection,
/// expansion sets, sort orders, picker toggles) so the screen file
/// shrinks to event wiring and `Render` glue.
///
/// Per ADR 0023 this struct must remain GPUI-free. Anything that
/// requires `gpui::Image`, `gpui::Entity`, or other framework types
/// stays in `LibraryApp` for now.
///
/// Fields move from `LibraryApp` into this VM in phases as the legacy
/// renderer migrates to projection VMs.
#[derive(Clone, Debug)]
pub(crate) struct LibraryViewModel {
    // Loaded snapshots — owned here so the screen can become a thin
    // Render impl. None of these carry GPUI types.
    snapshot: LibrarySnapshot,
    // Sidebar expansion + sort.
    expanded_artists: HashSet<String>,
    expanded_albums: HashSet<(String, String)>,
    playlists_expanded: bool,
    playlist_sort: PlaylistSort,
    // Selection / focus.
    selected_id: Option<i64>,
    selected_playlist_id: Option<i64>,
    hovered_thumb_url: Option<String>,
    // Operation state.
    busy_track: Option<i64>,
    status: String,
    // Layout / drag state.
    split_pane: SplitPaneState,
    // Search + playlist creation.
    search_query: String,
    creating_playlist: bool,
    // Album-detail "Add to playlist" picker toggles.
    album_add_open_feed: bool,
    album_add_open_track: Option<i64>,
}

impl LibraryViewModel {
    /// Construct a view-model with the legacy `LibraryApp::new`
    /// defaults: empty tree + snapshots, empty expansion sets,
    /// playlists sidebar already expanded, sort by name, no selection,
    /// no operation in flight.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            snapshot: LibrarySnapshot::default(),
            expanded_artists: HashSet::new(),
            expanded_albums: HashSet::new(),
            playlists_expanded: true,
            playlist_sort: PlaylistSort::default(),
            selected_id: None,
            selected_playlist_id: None,
            hovered_thumb_url: None,
            busy_track: None,
            status: String::new(),
            split_pane: SplitPaneState::new(DEFAULT_SPLIT_PANE_WIDTH),
            search_query: String::new(),
            creating_playlist: false,
            album_add_open_feed: false,
            album_add_open_track: None,
        }
    }

    #[must_use]
    pub(crate) fn tree(&self) -> &LibraryTree {
        &self.snapshot.tree
    }

    pub(crate) fn replace_tree(&mut self, tree: LibraryTree) {
        self.snapshot.tree = tree;
    }

    pub(crate) fn finish_library_reload(&mut self, track_count: usize) {
        self.status = format!("{track_count} library track{}", plural(track_count));
    }

    #[must_use]
    pub(crate) fn tree_projection(&self) -> LibraryTreeProjection {
        let query = self.search_query.trim();
        if query.is_empty() {
            return LibraryTreeProjection {
                tree: self.snapshot.tree.clone(),
                expanded_artists: self.expanded_artists.clone(),
                expanded_albums: self.expanded_albums.clone(),
            };
        }

        let tree = filter_tree(&self.snapshot.tree, query);
        let expanded_artists = tree
            .artists
            .iter()
            .map(|artist| artist.name.clone())
            .collect();
        let expanded_albums = tree
            .artists
            .iter()
            .flat_map(|artist| {
                artist
                    .albums
                    .iter()
                    .map(move |album| (artist.name.clone(), album.name.clone()))
            })
            .collect();

        LibraryTreeProjection {
            tree,
            expanded_artists,
            expanded_albums,
        }
    }

    #[must_use]
    pub(crate) fn playlists(&self) -> &[db::Playlist] {
        &self.snapshot.playlists
    }

    pub(crate) fn replace_playlists(&mut self, mut playlists: Vec<db::Playlist>) {
        Self::sort_playlists_by(self.playlist_sort, &mut playlists);
        self.snapshot.playlists = playlists;
    }

    pub(crate) fn fail_playlist_load(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Error loading playlists: {error:#}");
    }

    pub(crate) fn sort_loaded_playlists(&mut self) {
        Self::sort_playlists_by(self.playlist_sort, &mut self.snapshot.playlists);
    }

    #[must_use]
    pub(crate) fn playlist_by_id(&self, playlist_id: i64) -> Option<db::Playlist> {
        self.snapshot
            .playlists
            .iter()
            .find(|playlist| playlist.id == playlist_id)
            .cloned()
    }

    #[must_use]
    pub(crate) fn playlist_sidebar(&self) -> PlaylistSidebarVm {
        PlaylistSidebarVm {
            expanded: self.playlists_expanded,
            disclosure_glyph: if self.playlists_expanded {
                "\u{25BC}"
            } else {
                "\u{25B6}"
            },
            sort_label: self.playlist_sort_label(),
            creating_playlist: self.creating_playlist,
            rows: self
                .snapshot
                .playlists
                .iter()
                .map(|playlist| PlaylistSidebarRowVm {
                    id: playlist.id,
                    element_id: format!("playlist-{}", playlist.id),
                    name: playlist.name.clone(),
                    track_count_label: format!("({})", playlist.track_count),
                    selected: self.selected_playlist_id == Some(playlist.id),
                })
                .collect(),
        }
    }

    pub(crate) fn replace_playlist_tracks(&mut self, tracks: Vec<TrackRow>) {
        self.snapshot.playlist_tracks = tracks;
    }

    pub(crate) fn fail_playlist_create(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Error creating playlist: {error:#}");
    }

    pub(crate) fn fail_playlist_rename(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Error renaming: {error:#}");
    }

    pub(crate) fn fail_playlist_delete(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Error deleting: {error:#}");
    }

    pub(crate) fn fail_playlist_track_remove(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Error removing track: {error:#}");
    }

    pub(crate) fn fail_playlist_track_reorder(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Error reordering: {error:#}");
    }

    pub(crate) fn fail_album_tracks_load(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Error loading album tracks: {error:#}");
    }

    pub(crate) fn set_album_has_no_tracks(&mut self) {
        self.status = "Album has no tracks".into();
    }

    #[must_use]
    pub(crate) fn begin_playlist_append(
        &mut self,
        playlist_id: i64,
        track_ids: Vec<i64>,
    ) -> Option<PlaylistAppendIntent> {
        if track_ids.is_empty() {
            return None;
        }
        let playlist_name = self
            .playlist_by_id(playlist_id)
            .map(|playlist| playlist.name)
            .unwrap_or_default();
        self.status = format!(
            "Downloading {} track{}...",
            track_ids.len(),
            plural(track_ids.len())
        );
        Some(PlaylistAppendIntent {
            playlist_id,
            track_ids,
            playlist_name,
        })
    }

    pub(crate) fn finish_playlist_append(
        &mut self,
        intent: &PlaylistAppendIntent,
        outcome: PlaylistAppendOutcome,
    ) {
        let mut message = format!(
            "Added {} of {} to {}",
            outcome.appended,
            intent.total_tracks(),
            intent.playlist_name()
        );
        if outcome.downloaded > 0 {
            write!(&mut message, " (downloaded {})", outcome.downloaded)
                .expect("writing to a String cannot fail");
        }
        if outcome.failed > 0 {
            write!(&mut message, "; {} failed", outcome.failed)
                .expect("writing to a String cannot fail");
        }
        self.status = message;
    }

    pub(crate) fn fail_playlist_append(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Error adding to playlist: {error:#}");
    }

    #[must_use]
    pub(crate) fn selected_id(&self) -> Option<i64> {
        self.selected_id
    }

    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "screen migration uses playlist selection helpers before this accessor"
        )
    )]
    pub(crate) fn selected_playlist_id(&self) -> Option<i64> {
        self.selected_playlist_id
    }

    #[must_use]
    pub(crate) fn hovered_thumb_url(&self) -> Option<&str> {
        self.hovered_thumb_url.as_deref()
    }

    pub(crate) fn set_hovered_thumb_url(&mut self, url: Option<String>) -> bool {
        if self.hovered_thumb_url == url {
            return false;
        }
        self.hovered_thumb_url = url;
        true
    }

    #[must_use]
    pub(crate) fn busy_track(&self) -> Option<i64> {
        self.busy_track
    }

    #[must_use]
    pub(crate) fn has_busy_track(&self) -> bool {
        self.busy_track.is_some()
    }

    pub(crate) fn begin_busy_track(&mut self, track_id: i64, status: impl Into<String>) {
        self.busy_track = Some(track_id);
        self.status = status.into();
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "kept as a focused escape hatch for future cancellable operations"
        )
    )]
    pub(crate) fn clear_busy_track(&mut self) {
        self.busy_track = None;
    }

    pub(crate) fn finish_track_subscribe(&mut self, outcome: TrackSubscribeOutcome) {
        self.busy_track = None;
        let mut message = format!("Subscribed track: {}", outcome.path_label);
        if let Some(warning) = outcome.format_warning {
            message.push_str(" — ");
            message.push_str(&warning);
        }
        self.status = message;
    }

    pub(crate) fn fail_track_subscribe(&mut self, error: impl std::fmt::Display) {
        self.busy_track = None;
        self.status = format!("Error subscribing track: {error:#}");
    }

    #[must_use]
    pub(crate) fn status(&self) -> &str {
        &self.status
    }

    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "kept as a focused state accessor for search-field migration tests"
        )
    )]
    pub(crate) fn search_query(&self) -> &str {
        &self.search_query
    }

    pub(crate) fn set_error_status(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Error: {error:#}");
    }

    #[must_use]
    pub(crate) fn is_resizing(&self) -> bool {
        self.split_pane.is_resizing()
    }

    #[must_use]
    pub(crate) fn split_pane_width(&self) -> f32 {
        self.split_pane.leading_width()
    }

    pub(crate) fn begin_resize(&mut self) {
        self.split_pane.begin_resize();
    }

    pub(crate) fn end_resize(&mut self) {
        self.split_pane.end_resize();
    }

    pub(crate) fn resize_split_pane(
        &mut self,
        requested_width: f32,
        min_width: f32,
        max_width: f32,
    ) {
        self.split_pane
            .resize_to(requested_width, min_width, max_width);
    }

    pub(crate) fn select_library_item(&mut self, id: i64) {
        self.selected_id = Some(id);
        self.selected_playlist_id = None;
    }

    pub(crate) fn clear_library_selection(&mut self) {
        self.selected_id = None;
    }

    pub(crate) fn select_playlist(&mut self, playlist_id: i64) {
        self.selected_id = None;
        self.selected_playlist_id = Some(playlist_id);
    }

    pub(crate) fn clear_playlist_selection_if(&mut self, playlist_id: i64) -> bool {
        if self.selected_playlist_id == Some(playlist_id) {
            self.selected_playlist_id = None;
            return true;
        }
        false
    }

    #[must_use]
    pub(crate) fn is_playlist_selected(&self, playlist_id: i64) -> bool {
        self.selected_playlist_id == Some(playlist_id)
    }

    #[must_use]
    pub(crate) fn album_feed_picker_open(&self) -> bool {
        self.album_add_open_feed
    }

    #[must_use]
    pub(crate) fn album_track_picker_open(&self) -> Option<i64> {
        self.album_add_open_track
    }

    pub(crate) fn toggle_album_feed_picker(&mut self) {
        self.album_add_open_feed = !self.album_add_open_feed;
    }

    pub(crate) fn close_album_feed_picker(&mut self) {
        self.album_add_open_feed = false;
    }

    pub(crate) fn toggle_album_track_picker(&mut self, track_id: i64) {
        self.album_add_open_track = if self.album_add_open_track == Some(track_id) {
            None
        } else {
            Some(track_id)
        };
    }

    pub(crate) fn close_album_track_picker(&mut self) {
        self.album_add_open_track = None;
    }

    #[must_use]
    pub(crate) fn mb_status(&self) -> &BTreeMap<i64, MbTrackStatus> {
        &self.snapshot.mb_status
    }

    #[must_use]
    pub(crate) fn has_mb_status(&self, track_id: i64) -> bool {
        self.snapshot.mb_status.contains_key(&track_id)
    }

    pub(crate) fn set_mb_status(&mut self, track_id: i64, status: MbTrackStatus) {
        self.snapshot.mb_status.insert(track_id, status);
    }

    pub(crate) fn mark_musicbrainz_pending(&mut self, track_ids: impl IntoIterator<Item = i64>) {
        for track_id in track_ids {
            self.set_mb_status(track_id, MbTrackStatus::Pending);
        }
    }

    pub(crate) fn clear_mb_status(&mut self) {
        self.snapshot.mb_status.clear();
    }

    pub(crate) fn begin_musicbrainz_track_lookup(&mut self, track_id: i64) -> bool {
        if self.has_mb_status(track_id) {
            return false;
        }
        self.set_mb_status(track_id, MbTrackStatus::Processing);
        self.status = "MusicBrainz lookup...".into();
        true
    }

    pub(crate) fn finish_musicbrainz_track_lookup(&mut self, track_id: i64, edit_count: usize) {
        self.set_mb_status(track_id, MbTrackStatus::Done(edit_count));
        self.status = format!(
            "MusicBrainz: staged {edit_count} edit{}",
            plural(edit_count)
        );
    }

    pub(crate) fn fail_musicbrainz_track_lookup(
        &mut self,
        track_id: i64,
        error: impl std::fmt::Display,
    ) {
        self.set_mb_status(track_id, MbTrackStatus::Skipped(format!("{error:#}")));
        self.status = format!("MusicBrainz error: {error:#}");
    }

    pub(crate) fn begin_musicbrainz_album_lookup(
        &mut self,
        track_ids: impl IntoIterator<Item = i64>,
    ) -> bool {
        let track_ids: Vec<i64> = track_ids.into_iter().collect();
        if track_ids.is_empty() {
            self.status = "No downloaded tracks to process".into();
            return false;
        }
        self.mark_musicbrainz_pending(track_ids.iter().copied());
        self.status = format!(
            "MusicBrainz: album lookup for {} tracks...",
            track_ids.len()
        );
        true
    }

    pub(crate) fn fail_musicbrainz_album_lookup_with_fallback(
        &mut self,
        error: impl std::fmt::Display,
    ) {
        self.status = format!("Album lookup failed ({error:#}), falling back to per-track...");
    }

    pub(crate) fn fallback_empty_musicbrainz_album_lookup(&mut self) {
        self.status = "Album lookup: no results, falling back to per-track...".into();
    }

    pub(crate) fn begin_musicbrainz_album_track_stage(
        &mut self,
        track_id: i64,
        progress: usize,
        total_count: usize,
    ) {
        self.set_mb_status(track_id, MbTrackStatus::Processing);
        self.status = format!("MusicBrainz: staging track {progress}/{total_count} ...");
    }

    pub(crate) fn finish_musicbrainz_album_track_stage(
        &mut self,
        track_id: i64,
        status: MbTrackStatus,
    ) {
        self.set_mb_status(track_id, status);
    }

    pub(crate) fn finish_musicbrainz_album_lookup(&mut self, total_edits: usize, processed: usize) {
        self.status = format!(
            "MusicBrainz: staged {total_edits} edit{} across {processed} tracks",
            plural(total_edits)
        );
    }

    #[must_use]
    pub(crate) fn staged_musicbrainz(&self, track_id: i64) -> Option<&MusicBrainzLookupResult> {
        self.snapshot.staged_musicbrainz.get(&track_id)
    }

    pub(crate) fn stage_musicbrainz(&mut self, track_id: i64, lookup: MusicBrainzLookupResult) {
        self.snapshot.staged_musicbrainz.insert(track_id, lookup);
    }

    #[must_use]
    pub(crate) fn feed_update_state(&self) -> &FeedUpdateState {
        &self.snapshot.feed_update_state
    }

    pub(crate) fn begin_feed_view_check(&mut self, feed_id: i64) -> bool {
        if self.snapshot.feed_update_state.phase == FeedUpdatePhase::Applying
            || self
                .snapshot
                .feed_update_state
                .stale
                .iter()
                .any(|entry| entry.feed_id == feed_id)
            || !self.snapshot.in_flight_feed_checks.insert(feed_id)
        {
            return false;
        }
        self.snapshot.feed_update_state.status_message = Some("Checking feed...".into());
        true
    }

    pub(crate) fn finish_feed_view_check(
        &mut self,
        feed_id: i64,
        result: Result<Option<feed_service::StaleFeed>, String>,
    ) {
        self.snapshot.in_flight_feed_checks.remove(&feed_id);
        match result {
            Ok(Some(entry)) => {
                if !self
                    .snapshot
                    .feed_update_state
                    .stale
                    .iter()
                    .any(|existing| existing.feed_id == entry.feed_id)
                {
                    self.snapshot.feed_update_state.stale.push(entry);
                }
                self.snapshot.feed_update_state.status_message = Some(
                    Self::pending_feed_update_label(self.snapshot.feed_update_state.stale.len()),
                );
            }
            Ok(None) => {
                if self.snapshot.feed_update_state.stale.is_empty()
                    && self.snapshot.in_flight_feed_checks.is_empty()
                {
                    self.snapshot.feed_update_state.status_message = Some("Feed up to date".into());
                }
            }
            Err(err) => {
                self.snapshot.feed_update_state.status_message =
                    Some(format!("Feed check error: {err}"));
            }
        }
    }

    pub(crate) fn set_feed_check_error(&mut self, message: impl Into<String>) {
        self.snapshot.feed_update_state.status_message =
            Some(format!("Feed check error: {}", message.into()));
    }

    pub(crate) fn set_no_subscribed_feeds(&mut self) {
        self.snapshot.feed_update_state.status_message =
            Some("No subscribed feeds to check".into());
    }

    pub(crate) fn begin_all_feed_check(&mut self, feed_count: usize) {
        self.snapshot.feed_update_state.phase = FeedUpdatePhase::Checking;
        self.snapshot.feed_update_state.stale.clear();
        self.snapshot.feed_update_state.status_message =
            Some(format!("Checking {feed_count} feeds..."));
    }

    pub(crate) fn finish_all_feed_check(&mut self, stale: Vec<feed_service::StaleFeed>) {
        self.snapshot.feed_update_state.phase = FeedUpdatePhase::Idle;
        self.snapshot.feed_update_state.stale = stale;
        self.snapshot.feed_update_state.status_message =
            Some(if self.snapshot.feed_update_state.stale.is_empty() {
                "All feeds up to date".into()
            } else {
                format!(
                    "{} feed update{} available",
                    self.snapshot.feed_update_state.stale.len(),
                    plural(self.snapshot.feed_update_state.stale.len())
                )
            });
    }

    pub(crate) fn begin_apply_feed_updates(&mut self) -> Option<Vec<feed_service::StaleFeed>> {
        if self.snapshot.feed_update_state.phase != FeedUpdatePhase::Idle
            || self.snapshot.feed_update_state.stale.is_empty()
        {
            return None;
        }
        let stale = self.snapshot.feed_update_state.stale.clone();
        self.snapshot.feed_update_state.phase = FeedUpdatePhase::Applying;
        self.snapshot.feed_update_state.status_message =
            Some(format!("Applying updates to {} feed(s)...", stale.len()));
        Some(stale)
    }

    pub(crate) fn finish_apply_feed_updates(&mut self, message: String) {
        self.snapshot.feed_update_state.phase = FeedUpdatePhase::Idle;
        self.snapshot.feed_update_state.stale.clear();
        self.snapshot.feed_update_state.status_message = Some(message);
    }

    pub(crate) fn apply_search_query(&mut self, query: impl Into<String>) {
        self.search_query = query.into().trim().to_string();
        self.selected_id = None;
    }

    pub(crate) fn toggle_creating_playlist(&mut self) {
        self.creating_playlist = !self.creating_playlist;
    }

    pub(crate) fn close_creating_playlist(&mut self) {
        self.creating_playlist = false;
    }

    /// Toggle the expansion state of an artist node by name.
    pub(crate) fn toggle_artist(&mut self, name: &str) {
        if !self.expanded_artists.remove(name) {
            self.expanded_artists.insert(name.to_string());
        }
    }

    /// Toggle the expansion state of an album node, keyed on the
    /// `(artist, album)` pair to keep two albums with the same name
    /// from collapsing onto each other.
    pub(crate) fn toggle_album(&mut self, artist: &str, album: &str) {
        let key = (artist.to_string(), album.to_string());
        if !self.expanded_albums.remove(&key) {
            self.expanded_albums.insert(key);
        }
    }

    /// Flip the playlists sidebar expansion flag.
    pub(crate) fn toggle_playlists_expanded(&mut self) {
        self.playlists_expanded = !self.playlists_expanded;
    }

    /// Advance the playlist sort order one step.
    pub(crate) fn cycle_playlist_sort(&mut self) {
        self.playlist_sort = self.playlist_sort.next();
    }

    fn sort_playlists_by(sort: PlaylistSort, playlists: &mut [db::Playlist]) {
        match sort {
            PlaylistSort::Name => {
                playlists.sort_by_key(|playlist| playlist.name.to_lowercase());
            }
            PlaylistSort::RecentlyUpdated => {
                playlists.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            }
            PlaylistSort::TrackCount => {
                playlists.sort_by(|a, b| b.track_count.cmp(&a.track_count));
            }
        }
    }

    fn pending_feed_update_label(count: usize) -> String {
        format!("{count} feed update{} pending", plural(count))
    }

    // Both lookups are exercised by the unit tests below but not yet
    // by the legacy renderer (which still uses `LibraryTreeProjection`).
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "kept as focused expansion predicates for future tree rendering"
        )
    )]
    #[must_use]
    pub(crate) fn is_artist_expanded(&self, name: &str) -> bool {
        self.expanded_artists.contains(name)
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "kept as focused expansion predicates for future tree rendering"
        )
    )]
    #[must_use]
    pub(crate) fn is_album_expanded(&self, artist: &str, album: &str) -> bool {
        self.expanded_albums
            .contains(&(artist.to_string(), album.to_string()))
    }

    #[must_use]
    pub(crate) fn playlist_sort_label(&self) -> &'static str {
        self.playlist_sort.label()
    }
}

fn filter_tree(tree: &LibraryTree, query: &str) -> LibraryTree {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return tree.clone();
    }
    let mut artists = Vec::new();
    for artist in &tree.artists {
        let artist_match = artist.name.to_lowercase().contains(&q);
        let mut albums = Vec::new();
        for album in &artist.albums {
            let album_match = album.name.to_lowercase().contains(&q);
            let keep_all = artist_match || album_match;
            let tracks: Vec<TrackRow> = if keep_all {
                album.tracks.clone()
            } else {
                album
                    .tracks
                    .iter()
                    .filter(|track| {
                        track
                            .track_title
                            .as_deref()
                            .unwrap_or("")
                            .to_lowercase()
                            .contains(&q)
                    })
                    .cloned()
                    .collect()
            };
            if keep_all || !tracks.is_empty() {
                albums.push(AlbumNode {
                    name: album.name.clone(),
                    feed_id: album.feed_id,
                    feed_url: album.feed_url.clone(),
                    image_href: album.image_href.clone(),
                    tracks,
                });
            }
        }
        if !albums.is_empty() {
            artists.push(ArtistNode {
                name: artist.name.clone(),
                albums,
            });
        }
    }
    LibraryTree { artists }
}

impl Default for LibraryViewModel {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> LibraryTrackRowVm<'a> {
    #[must_use]
    pub(crate) fn new(track: &'a TrackRow, mb: Option<&'a MbTrackStatus>) -> Self {
        Self { track, mb }
    }

    /// Display title — the row's `track_title`, or `"[untitled]"` if
    /// absent. Matches the legacy `library::render_library_track_row`
    /// fallback exactly.
    #[must_use]
    pub(crate) fn title(&self) -> String {
        self.track
            .track_title
            .as_deref()
            .unwrap_or("[untitled]")
            .to_string()
    }

    /// Leading `"{n}. "` segment, empty when there is no track number.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn number_prefix(&self) -> String {
        self.track
            .track_number
            .map(|n| format!("{n}. "))
            .unwrap_or_default()
    }

    /// Leading track-number label for the shared [`TrackRow`] composite.
    #[must_use]
    pub(crate) fn number_label(&self) -> String {
        self.track
            .track_number
            .map_or_else(|| "·".into(), |n| n.to_string())
    }

    /// Trailing `"  (M:SS)"` segment, empty when there is no
    /// duration.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn duration_suffix(&self) -> String {
        self.track
            .duration_seconds
            .map(|s| format!("  ({}:{:02})", s / 60, s % 60))
            .unwrap_or_default()
    }

    /// Duration label for the shared [`TrackRow`] composite.
    #[must_use]
    pub(crate) fn duration_display(&self) -> Option<String> {
        self.track
            .duration_seconds
            .map(|seconds| format!("{}:{:02}", seconds / 60, seconds % 60))
    }

    /// Add-to-playlist action label for the row.
    #[must_use]
    pub(crate) fn playlist_action_label(&self, is_open: bool) -> &'static str {
        match self
            .track_action_state(false, is_open)
            .playlist_action(EntityActionTarget::Track(self.track_ref()))
            .map(|action| action.label)
            .as_deref()
        {
            Some("+ Playlist ▴") => "+ Playlist ▴",
            _ => "+ Playlist",
        }
    }

    #[must_use]
    pub(crate) fn primary_action_vm(&self, is_busy: bool) -> EntityActionVm {
        self.track_action_state(is_busy, false)
            .primary_action(EntityActionTarget::Track(self.track_ref()))
    }

    #[must_use]
    pub(crate) fn track_action_state(
        &self,
        is_busy: bool,
        playlist_open: bool,
    ) -> TrackActionState {
        let membership = match (self.track.is_in_library, is_busy) {
            (true, true) => TrackMembershipState::Removing,
            (true, false) => TrackMembershipState::InLibrary,
            (false, true) => TrackMembershipState::Downloading,
            (false, false) => TrackMembershipState::RemoteOnly,
        };
        let playlist = if playlist_open {
            PlaylistActionState::Open
        } else {
            PlaylistActionState::Closed
        };
        TrackActionState::new(membership, playlist)
    }

    #[must_use]
    fn track_ref(&self) -> TrackRef {
        TrackRef::LocalTrackId(self.track.id)
    }

    /// Concatenated single-line label: `"{n}. {title}  (M:SS)"`.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn full_label(&self) -> String {
        format!(
            "{}{}{}",
            self.number_prefix(),
            self.title(),
            self.duration_suffix()
        )
    }

    /// Human-readable `MusicBrainz` status hint, or `None` when no
    /// lookup has been started for this track.
    #[must_use]
    pub(crate) fn mb_status_text(&self) -> Option<&'static str> {
        match self.mb? {
            MbTrackStatus::Pending => Some("MB: pending"),
            MbTrackStatus::Processing => Some("MB: looking up..."),
            MbTrackStatus::Done(0) => Some("MB: no missing fields"),
            MbTrackStatus::Done(_) => Some("MB: done"),
            MbTrackStatus::Skipped(_) => Some("MB: skipped"),
        }
    }

    /// Semantic colour bucket for the status hint (or `None` when
    /// there is no hint).
    #[must_use]
    pub(crate) fn mb_status_kind(&self) -> Option<MbStatusKind> {
        match self.mb? {
            MbTrackStatus::Done(n) if *n > 0 => Some(MbStatusKind::Success),
            MbTrackStatus::Skipped(_) => Some(MbStatusKind::Danger),
            MbTrackStatus::Processing => Some(MbStatusKind::Warning),
            _ => Some(MbStatusKind::Muted),
        }
    }
}

/// Display-ready projection of a feed-row inside the library artist
/// detail. The screen looks up the actual thumbnail image by `thumb_url`
/// and wires the click handler by `feed_id`; the VM only carries plain
/// data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ArtistFeedSummaryVm {
    pub(crate) feed_id: i64,
    pub(crate) feed_name: String,
    pub(crate) thumb_url: Option<String>,
    pub(crate) track_count: usize,
}

/// Display-ready projection of a library artist detail panel.
///
/// Borrow-only — constructed fresh each render and dropped before the
/// element tree is painted. The VM groups tracks by feed and applies the
/// "Untitled Feed" / "Unknown" fallbacks the legacy renderer used.
pub(crate) struct LibraryArtistDetailVm<'a> {
    name: &'a str,
    tracks: &'a [TrackRow],
}

impl<'a> LibraryArtistDetailVm<'a> {
    #[must_use]
    pub(crate) fn new(name: &'a str, tracks: &'a [TrackRow]) -> Self {
        Self { name, tracks }
    }

    /// Artist name with the legacy `"Unknown"` fallback applied when
    /// empty.
    #[must_use]
    pub(crate) fn artist_name_or_unknown(&self) -> String {
        if self.name.is_empty() {
            "Unknown".to_string()
        } else {
            self.name.to_string()
        }
    }

    /// Number of distinct feeds (== albums) under this artist.
    #[must_use]
    pub(crate) fn album_count(&self) -> usize {
        let mut feeds = std::collections::BTreeSet::new();
        for t in self.tracks {
            feeds.insert(t.feed_id);
        }
        feeds.len()
    }

    /// Total track count.
    #[must_use]
    pub(crate) fn track_count(&self) -> usize {
        self.tracks.len()
    }

    /// Number of tracks that have been downloaded to disk.
    #[must_use]
    pub(crate) fn downloaded_count(&self) -> usize {
        self.tracks
            .iter()
            .filter(|t| t.local_path.is_some())
            .count()
    }

    /// Detail-grid rows in display order: `Albums`, `Tracks` (with
    /// pluralised count), and `Downloaded` (only when at least one track
    /// is downloaded).
    #[must_use]
    pub(crate) fn detail_rows(&self) -> Vec<(String, String)> {
        let mut rows = vec![
            ("Albums".to_string(), self.album_count().to_string()),
            (
                "Tracks".to_string(),
                format!("{} track{}", self.track_count(), plural(self.track_count())),
            ),
        ];
        let downloaded = self.downloaded_count();
        if downloaded > 0 {
            rows.push(("Downloaded".to_string(), downloaded.to_string()));
        }
        rows
    }

    /// One [`ArtistFeedSummaryVm`] per distinct feed, ordered by
    /// `feed_id` (matches `BTreeMap` iteration of the legacy renderer).
    #[must_use]
    pub(crate) fn feed_summaries(&self) -> Vec<ArtistFeedSummaryVm> {
        let mut feed_map: BTreeMap<i64, (Option<String>, Vec<&TrackRow>)> = BTreeMap::new();
        for track in self.tracks {
            feed_map
                .entry(track.feed_id)
                .or_insert_with(|| (track.feed_title.clone(), Vec::new()))
                .1
                .push(track);
        }
        feed_map
            .into_iter()
            .map(|(feed_id, (feed_title, tracks))| {
                let feed_name = feed_title.unwrap_or_else(|| "Untitled Feed".to_string());
                let first = tracks.first();
                let thumb_url = first.and_then(|t| {
                    t.album_image_href
                        .clone()
                        .or_else(|| t.track_image_href.clone())
                });
                ArtistFeedSummaryVm {
                    feed_id,
                    feed_name,
                    thumb_url,
                    track_count: tracks.len(),
                }
            })
            .collect()
    }
}

/// Display-ready projection of a library album detail panel.
///
/// Borrow-only — constructed fresh each render and dropped before the
/// element tree is painted. The VM owns the title/artist fallbacks,
/// detail-row composition, total-runtime roll-up, and the
/// `MusicBrainz` activity flag the action button needs to disable
/// itself while a lookup is in flight.
pub(crate) struct LibraryAlbumDetailVm<'a> {
    feed_view: &'a FeedView,
    tracks: &'a [TrackRow],
    mb_status: &'a BTreeMap<i64, MbTrackStatus>,
}

impl<'a> LibraryAlbumDetailVm<'a> {
    #[must_use]
    pub(crate) fn new(
        feed_view: &'a FeedView,
        tracks: &'a [TrackRow],
        mb_status: &'a BTreeMap<i64, MbTrackStatus>,
    ) -> Self {
        Self {
            feed_view,
            tracks,
            mb_status,
        }
    }

    /// Album title with the legacy `"Untitled"` fallback.
    #[must_use]
    pub(crate) fn title(&self) -> String {
        self.feed_view
            .title
            .clone()
            .unwrap_or_else(|| "Untitled".to_string())
    }

    /// Artist with the legacy `"Unknown Artist"` fallback. The detail
    /// header subtitle and the `Artist` detail-row both display this.
    #[must_use]
    pub(crate) fn artist(&self) -> String {
        self.feed_view
            .artist
            .clone()
            .unwrap_or_else(|| "Unknown Artist".to_string())
    }

    #[must_use]
    pub(crate) fn track_count(&self) -> usize {
        self.tracks.len()
    }

    /// Sum of all track durations in seconds.
    #[must_use]
    pub(crate) fn total_duration_seconds(&self) -> i64 {
        self.tracks.iter().filter_map(|t| t.duration_seconds).sum()
    }

    /// Clock-style total runtime label, or `None` when no track has a
    /// known duration. See [`fmt_total_runtime_clock`].
    #[must_use]
    pub(crate) fn total_duration_label(&self) -> Option<String> {
        fmt_total_runtime_clock(self.total_duration_seconds())
    }

    /// Number of tracks downloaded to disk.
    #[must_use]
    pub(crate) fn downloaded_count(&self) -> usize {
        self.tracks
            .iter()
            .filter(|t| t.local_path.is_some())
            .count()
    }

    /// Detail-grid rows in display order: `Artist`, `Tracks` (with
    /// pluralised count), `Duration` (only when total > 0), and
    /// `Downloaded` (only when at least one track is downloaded).
    #[must_use]
    pub(crate) fn detail_rows(&self) -> Vec<(String, String)> {
        let track_count = self.track_count();
        let mut rows = vec![
            ("Artist".to_string(), self.artist()),
            (
                "Tracks".to_string(),
                format!("{track_count} track{}", plural(track_count)),
            ),
        ];
        if let Some(label) = self.total_duration_label() {
            rows.push(("Duration".to_string(), label));
        }
        let downloaded = self.downloaded_count();
        if downloaded > 0 {
            rows.push(("Downloaded".to_string(), downloaded.to_string()));
        }
        rows
    }

    /// `true` when any track has an in-flight `MusicBrainz` lookup —
    /// used by the screen to disable the `MusicBrainz` action button.
    #[must_use]
    pub(crate) fn has_active_musicbrainz(&self) -> bool {
        self.mb_status
            .values()
            .any(|s| matches!(s, MbTrackStatus::Pending | MbTrackStatus::Processing))
    }

    #[must_use]
    pub(crate) fn primary_action_vm(&self, feed_id: i64, is_busy: bool) -> EntityActionVm {
        self.release_action_state(is_busy, PlaylistActionState::Hidden)
            .primary_action(EntityActionTarget::Feed(FeedRef::LocalFeedId(feed_id)))
    }

    #[must_use]
    pub(crate) fn playlist_action_vm(&self, feed_id: i64, open: bool) -> Option<EntityActionVm> {
        self.release_action_state(
            false,
            if open {
                PlaylistActionState::Open
            } else {
                PlaylistActionState::Closed
            },
        )
        .playlist_action(EntityActionTarget::Feed(FeedRef::LocalFeedId(feed_id)))
    }

    #[must_use]
    #[expect(
        clippy::unused_self,
        reason = "kept on the album VM while Library action wiring is migrated"
    )]
    fn release_action_state(
        &self,
        is_busy: bool,
        playlist: PlaylistActionState,
    ) -> ReleaseActionState {
        let membership = if is_busy {
            ReleaseMembershipState::Removing
        } else {
            ReleaseMembershipState::InLibrary
        };

        ReleaseActionState::new(membership, playlist)
    }
}

/// Display-ready projection of a single row inside a playlist detail
/// listing. The screen owns the click handlers and button rendering;
/// the VM owns text fallbacks, duration formatting, and the
/// move-up/move-down enable rules.
pub(crate) struct PlaylistTrackRowVm<'a> {
    track: &'a TrackRow,
    position: usize,
    last_position: usize,
}

impl PlaylistTrackRowVm<'_> {
    #[must_use]
    pub(crate) fn track(&self) -> &TrackRow {
        self.track
    }

    #[must_use]
    pub(crate) fn track_id(&self) -> i64 {
        self.track.id
    }

    #[must_use]
    pub(crate) fn position(&self) -> usize {
        self.position
    }

    /// `"{n}."` where `n` is the 1-indexed position.
    #[must_use]
    pub(crate) fn position_label(&self) -> String {
        format!("{}.", self.position + 1)
    }

    /// Title with the legacy `"[untitled]"` fallback.
    #[must_use]
    pub(crate) fn title(&self) -> String {
        self.track
            .track_title
            .as_deref()
            .unwrap_or("[untitled]")
            .to_string()
    }

    /// Artist with the legacy `"Unknown"` fallback.
    #[must_use]
    pub(crate) fn artist(&self) -> String {
        self.track
            .artist_name
            .as_deref()
            .unwrap_or("Unknown")
            .to_string()
    }

    /// `"M:SS"` formatted duration, or `""` when the track has none.
    #[must_use]
    pub(crate) fn duration_label(&self) -> String {
        self.track
            .duration_seconds
            .map(|s| format!("{}:{:02}", s / 60, s % 60))
            .unwrap_or_default()
    }

    /// Preferred thumbnail URL — `track_image_href` first, then
    /// `album_image_href`. Matches the legacy renderer's lookup order.
    #[must_use]
    pub(crate) fn thumb_url(&self) -> Option<&str> {
        self.track
            .track_image_href
            .as_deref()
            .or(self.track.album_image_href.as_deref())
    }

    /// `true` when the track has a local file and can be played.
    #[must_use]
    pub(crate) fn can_play(&self) -> bool {
        self.track.local_path.is_some()
    }

    #[must_use]
    pub(crate) fn can_move_up(&self) -> bool {
        self.position > 0
    }

    #[must_use]
    pub(crate) fn can_move_down(&self) -> bool {
        self.position < self.last_position
    }
}

/// Display-ready projection of a playlist detail panel.
///
/// Borrow-only — constructed fresh each render and dropped before the
/// element tree is painted. The VM owns the duration roll-up,
/// detail-row composition, and per-track projections.
pub(crate) struct PlaylistDetailVm<'a> {
    playlist: &'a db::Playlist,
    tracks: &'a [TrackRow],
}

impl<'a> PlaylistDetailVm<'a> {
    #[must_use]
    pub(crate) fn new(playlist: &'a db::Playlist, tracks: &'a [TrackRow]) -> Self {
        Self { playlist, tracks }
    }

    #[must_use]
    pub(crate) fn playlist_id(&self) -> i64 {
        self.playlist.id
    }

    #[must_use]
    pub(crate) fn name(&self) -> &str {
        &self.playlist.name
    }

    #[must_use]
    pub(crate) fn track_count(&self) -> usize {
        self.tracks.len()
    }

    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    /// Sum of all track durations in seconds.
    #[must_use]
    pub(crate) fn total_duration_seconds(&self) -> i64 {
        self.tracks.iter().filter_map(|t| t.duration_seconds).sum()
    }

    /// `"M:SS"` for short playlists, `"Hh Mm"` once total runtime
    /// crosses an hour, or `None` when the total is zero (no track
    /// has a known duration). Matches the legacy renderer exactly.
    #[must_use]
    pub(crate) fn total_duration_label(&self) -> Option<String> {
        fmt_total_runtime_clock(self.total_duration_seconds())
    }

    /// Detail-grid rows in display order: `Tracks` always, plus
    /// `Duration` when there is a non-zero total runtime.
    #[must_use]
    pub(crate) fn detail_rows(&self) -> Vec<(String, String)> {
        let mut rows = vec![("Tracks".to_string(), self.track_count().to_string())];
        if let Some(label) = self.total_duration_label() {
            rows.push(("Duration".to_string(), label));
        }
        rows
    }

    /// Empty-state message rendered in place of the track list.
    #[expect(
        clippy::unused_self,
        reason = "kept as a method for API symmetry with the other accessors"
    )]
    #[must_use]
    pub(crate) fn empty_message(&self) -> &'static str {
        "Empty — add tracks from the library or search"
    }

    /// One [`PlaylistTrackRowVm`] per track, in stored order. Returns
    /// an empty vec when the playlist has no tracks (callers can use
    /// [`Self::is_empty`] to branch on the empty-state message).
    #[must_use]
    pub(crate) fn track_rows(&self) -> Vec<PlaylistTrackRowVm<'a>> {
        let last_position = self.tracks.len().saturating_sub(1);
        self.tracks
            .iter()
            .enumerate()
            .map(|(position, track)| PlaylistTrackRowVm {
                track,
                position,
                last_position,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_width_eq(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < f32::EPSILON);
    }

    fn row() -> TrackRow {
        TrackRow {
            id: 0,
            feed_id: 0,
            feed_guid: None,
            item_guid: String::new(),
            track_title: None,
            artist_name: None,
            album_title: None,
            album_artist_name: None,
            track_number: None,
            disc_number: None,
            duration_seconds: None,
            enclosure_url: None,
            enclosure_type: None,
            track_image_href: None,
            is_in_library: false,
            feed_title: None,
            album_image_href: None,
            local_path: None,
            transcript_url: None,
        }
    }

    #[test]
    fn title_falls_back_to_untitled_marker() {
        assert_eq!(LibraryTrackRowVm::new(&row(), None).title(), "[untitled]");
        let mut r = row();
        r.track_title = Some("Hello".into());
        assert_eq!(LibraryTrackRowVm::new(&r, None).title(), "Hello");
    }

    #[test]
    fn number_prefix_renders_only_when_present() {
        assert_eq!(LibraryTrackRowVm::new(&row(), None).number_prefix(), "");
        let mut r = row();
        r.track_number = Some(7);
        assert_eq!(LibraryTrackRowVm::new(&r, None).number_prefix(), "7. ");
        assert_eq!(LibraryTrackRowVm::new(&r, None).number_label(), "7");
        assert_eq!(LibraryTrackRowVm::new(&row(), None).number_label(), "·");
    }

    #[test]
    fn duration_suffix_pads_seconds_below_ten() {
        let mut r = row();
        r.duration_seconds = Some(65);
        assert_eq!(
            LibraryTrackRowVm::new(&r, None).duration_suffix(),
            "  (1:05)"
        );
        assert_eq!(
            LibraryTrackRowVm::new(&r, None)
                .duration_display()
                .as_deref(),
            Some("1:05")
        );
    }

    #[test]
    fn duration_suffix_is_empty_when_absent() {
        assert_eq!(LibraryTrackRowVm::new(&row(), None).duration_suffix(), "");
    }

    #[test]
    fn library_track_row_vm_primary_action_follows_membership_and_busy_state() {
        let mut r = row();
        let vm = LibraryTrackRowVm::new(&r, None);
        let action = vm.primary_action_vm(false);
        assert_eq!(
            action.kind,
            crate::view_models::entity_detail::EntityActionKind::Download
        );
        assert_eq!(action.label, "Download");
        assert!(action.enabled);

        let action = vm.primary_action_vm(true);
        assert_eq!(action.label, "Downloading...");
        assert!(!action.enabled);

        r.is_in_library = true;
        let vm = LibraryTrackRowVm::new(&r, None);
        let action = vm.primary_action_vm(false);
        assert_eq!(
            action.kind,
            crate::view_models::entity_detail::EntityActionKind::Remove
        );
        assert_eq!(action.label, "Remove");
        assert!(action.enabled);

        let action = vm.primary_action_vm(true);
        assert_eq!(action.label, "Removing...");
        assert!(!action.enabled);
    }

    #[test]
    fn library_track_row_vm_projects_shared_primary_action() {
        let mut r = row();
        r.id = 42;
        let vm = LibraryTrackRowVm::new(&r, None);
        let action = vm.primary_action_vm(false);
        assert_eq!(
            action.target,
            EntityActionTarget::Track(TrackRef::LocalTrackId(42))
        );
        assert_eq!(
            action.kind,
            crate::view_models::entity_detail::EntityActionKind::Download
        );
        assert_eq!(
            action.tone,
            crate::view_models::entity_detail::EntityActionTone::Secondary
        );

        r.is_in_library = true;
        let vm = LibraryTrackRowVm::new(&r, None);
        let action = vm.primary_action_vm(false);
        assert_eq!(
            action.kind,
            crate::view_models::entity_detail::EntityActionKind::Remove
        );
        assert_eq!(
            action.tone,
            crate::view_models::entity_detail::EntityActionTone::DestructiveQuiet
        );
        assert!(action.enabled);

        let action = vm.primary_action_vm(true);
        assert_eq!(action.label, "Removing...");
        assert!(!action.enabled);
    }

    #[test]
    fn library_track_row_vm_playlist_action_label_reflects_open_state() {
        let r = row();
        let vm = LibraryTrackRowVm::new(&r, None);
        assert_eq!(vm.playlist_action_label(false), "+ Playlist");
        assert_eq!(vm.playlist_action_label(true), "+ Playlist ▴");
    }

    #[test]
    fn library_track_row_vm_local_path_does_not_change_row_action_text() {
        let mut r = row();
        r.local_path = Some("/music/track.mp3".into());
        let vm = LibraryTrackRowVm::new(&r, None);
        assert_eq!(vm.primary_action_vm(false).label, "Download");
        assert_eq!(vm.playlist_action_label(false), "+ Playlist");
    }

    #[test]
    fn full_label_concatenates_segments() {
        let mut r = row();
        r.track_number = Some(3);
        r.track_title = Some("Track Three".into());
        r.duration_seconds = Some(245);
        assert_eq!(
            LibraryTrackRowVm::new(&r, None).full_label(),
            "3. Track Three  (4:05)"
        );
    }

    #[test]
    fn mb_status_text_distinguishes_done_zero_and_done_nonzero() {
        let r = row();
        let pending = MbTrackStatus::Pending;
        let processing = MbTrackStatus::Processing;
        let done_zero = MbTrackStatus::Done(0);
        let done_some = MbTrackStatus::Done(2);
        let skipped = MbTrackStatus::Skipped("bad".into());

        assert_eq!(LibraryTrackRowVm::new(&r, None).mb_status_text(), None);
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&pending)).mb_status_text(),
            Some("MB: pending")
        );
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&processing)).mb_status_text(),
            Some("MB: looking up...")
        );
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&done_zero)).mb_status_text(),
            Some("MB: no missing fields")
        );
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&done_some)).mb_status_text(),
            Some("MB: done")
        );
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&skipped)).mb_status_text(),
            Some("MB: skipped")
        );
    }

    #[test]
    fn mb_status_kind_routes_done_zero_to_muted_not_success() {
        let r = row();
        let done_zero = MbTrackStatus::Done(0);
        let done_some = MbTrackStatus::Done(3);
        let processing = MbTrackStatus::Processing;
        let skipped = MbTrackStatus::Skipped("nope".into());
        let pending = MbTrackStatus::Pending;

        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&done_zero)).mb_status_kind(),
            Some(MbStatusKind::Muted)
        );
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&done_some)).mb_status_kind(),
            Some(MbStatusKind::Success)
        );
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&processing)).mb_status_kind(),
            Some(MbStatusKind::Warning)
        );
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&skipped)).mb_status_kind(),
            Some(MbStatusKind::Danger)
        );
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&pending)).mb_status_kind(),
            Some(MbStatusKind::Muted)
        );
        assert_eq!(LibraryTrackRowVm::new(&r, None).mb_status_kind(), None);
    }

    fn track_for_feed(feed_id: i64, feed_title: Option<&str>) -> TrackRow {
        let mut r = row();
        r.feed_id = feed_id;
        r.feed_title = feed_title.map(str::to_string);
        r
    }

    #[test]
    fn artist_detail_vm_falls_back_to_unknown_for_empty_name() {
        let vm = LibraryArtistDetailVm::new("", &[]);
        assert_eq!(vm.artist_name_or_unknown(), "Unknown");
        let vm = LibraryArtistDetailVm::new("Aphex", &[]);
        assert_eq!(vm.artist_name_or_unknown(), "Aphex");
    }

    #[test]
    fn artist_detail_vm_counts_distinct_feeds_as_albums() {
        let tracks = vec![
            track_for_feed(1, Some("A")),
            track_for_feed(1, Some("A")),
            track_for_feed(2, Some("B")),
        ];
        let vm = LibraryArtistDetailVm::new("Artist", &tracks);
        assert_eq!(vm.album_count(), 2);
        assert_eq!(vm.track_count(), 3);
    }

    #[test]
    fn artist_detail_vm_omits_downloaded_row_when_zero() {
        let tracks = vec![track_for_feed(1, Some("A"))];
        let vm = LibraryArtistDetailVm::new("Artist", &tracks);
        let rows = vm.detail_rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], ("Albums".into(), "1".into()));
        assert_eq!(rows[1], ("Tracks".into(), "1 track".into()));
    }

    #[test]
    fn artist_detail_vm_pluralises_track_count_above_one() {
        let tracks = [track_for_feed(1, Some("A")), track_for_feed(1, Some("A"))];
        let vm = LibraryArtistDetailVm::new("Artist", &tracks);
        let rows = vm.detail_rows();
        assert_eq!(rows[1], ("Tracks".into(), "2 tracks".into()));
    }

    #[test]
    fn artist_detail_vm_includes_downloaded_row_when_any_local_path_present() {
        let mut t1 = track_for_feed(1, Some("A"));
        t1.local_path = Some("/x".into());
        let t2 = track_for_feed(1, Some("A"));
        let tracks = [t1, t2];
        let vm = LibraryArtistDetailVm::new("Artist", &tracks);
        let rows = vm.detail_rows();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[2], ("Downloaded".into(), "1".into()));
    }

    #[test]
    fn artist_detail_vm_feed_summaries_apply_untitled_fallback_and_track_counts() {
        let mut t1 = track_for_feed(1, None);
        t1.album_image_href = Some("img-1".into());
        let t2 = track_for_feed(1, None);
        let t3 = track_for_feed(2, Some("Real"));
        let tracks = [t1, t2, t3];
        let vm = LibraryArtistDetailVm::new("Artist", &tracks);
        let summaries = vm.feed_summaries();
        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].feed_id, 1);
        assert_eq!(summaries[0].feed_name, "Untitled Feed");
        assert_eq!(summaries[0].thumb_url.as_deref(), Some("img-1"));
        assert_eq!(summaries[0].track_count, 2);
        assert_eq!(summaries[1].feed_id, 2);
        assert_eq!(summaries[1].feed_name, "Real");
        assert_eq!(summaries[1].track_count, 1);
    }

    #[test]
    fn artist_detail_vm_thumb_url_falls_back_to_track_image_href() {
        let mut t = track_for_feed(1, Some("A"));
        t.track_image_href = Some("track-img".into());
        let tracks = [t];
        let vm = LibraryArtistDetailVm::new("Artist", &tracks);
        let summaries = vm.feed_summaries();
        assert_eq!(summaries[0].thumb_url.as_deref(), Some("track-img"));
    }

    fn playlist(name: &str) -> db::Playlist {
        db::Playlist {
            id: 1,
            name: name.into(),
            description: None,
            track_count: 0,
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn playlist_detail_vm_reports_empty_state() {
        let pl = playlist("Mix");
        let vm = PlaylistDetailVm::new(&pl, &[]);
        assert!(vm.is_empty());
        assert_eq!(vm.track_count(), 0);
        assert_eq!(vm.total_duration_seconds(), 0);
        assert_eq!(vm.total_duration_label(), None);
        assert_eq!(vm.detail_rows(), vec![("Tracks".into(), "0".into())]);
    }

    #[test]
    fn playlist_detail_vm_total_duration_uses_minutes_below_an_hour() {
        let pl = playlist("Mix");
        let mut t1 = row();
        t1.duration_seconds = Some(125); // 2:05
        let mut t2 = row();
        t2.duration_seconds = Some(180); // 3:00
        let tracks = [t1, t2];
        let vm = PlaylistDetailVm::new(&pl, &tracks);
        assert_eq!(vm.total_duration_seconds(), 305);
        assert_eq!(vm.total_duration_label().as_deref(), Some("5:05"));
    }

    #[test]
    fn playlist_detail_vm_total_duration_switches_to_hours_after_60_minutes() {
        let pl = playlist("Mix");
        let mut t = row();
        // 1h 23m == 4980 sec
        t.duration_seconds = Some(4980);
        let tracks = [t];
        let vm = PlaylistDetailVm::new(&pl, &tracks);
        assert_eq!(vm.total_duration_label().as_deref(), Some("1h 23m"));
    }

    #[test]
    fn playlist_detail_vm_total_duration_is_none_when_no_track_has_seconds() {
        let pl = playlist("Mix");
        let tracks = [row(), row()];
        let vm = PlaylistDetailVm::new(&pl, &tracks);
        assert_eq!(vm.total_duration_label(), None);
        assert_eq!(vm.detail_rows().len(), 1);
    }

    #[test]
    fn playlist_detail_vm_detail_rows_include_duration_when_known() {
        let pl = playlist("Mix");
        let mut t = row();
        t.duration_seconds = Some(60);
        let tracks = [t];
        let vm = PlaylistDetailVm::new(&pl, &tracks);
        let rows = vm.detail_rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], ("Tracks".into(), "1".into()));
        assert_eq!(rows[1], ("Duration".into(), "1:00".into()));
    }

    #[test]
    fn playlist_track_row_vm_applies_title_and_artist_fallbacks() {
        let pl = playlist("Mix");
        let tracks = [row()];
        let vm = PlaylistDetailVm::new(&pl, &tracks);
        let rows = vm.track_rows();
        assert_eq!(rows[0].title(), "[untitled]");
        assert_eq!(rows[0].artist(), "Unknown");
        assert_eq!(rows[0].duration_label(), "");
        assert_eq!(rows[0].position_label(), "1.");
    }

    #[test]
    fn playlist_track_row_vm_can_play_follows_local_path() {
        let pl = playlist("Mix");
        let mut t = row();
        t.local_path = Some("/x".into());
        let tracks = [t, row()];
        let vm = PlaylistDetailVm::new(&pl, &tracks);
        let rows = vm.track_rows();
        assert!(rows[0].can_play());
        assert!(!rows[1].can_play());
    }

    #[test]
    fn playlist_track_row_vm_move_enable_rules_at_boundaries() {
        let pl = playlist("Mix");
        let tracks = [row(), row(), row()];
        let vm = PlaylistDetailVm::new(&pl, &tracks);
        let rows = vm.track_rows();
        assert!(!rows[0].can_move_up());
        assert!(rows[0].can_move_down());
        assert!(rows[1].can_move_up());
        assert!(rows[1].can_move_down());
        assert!(rows[2].can_move_up());
        assert!(!rows[2].can_move_down());
    }

    #[test]
    fn playlist_track_row_vm_thumb_prefers_track_image_then_album_image() {
        let pl = playlist("Mix");
        let mut t1 = row();
        t1.track_image_href = Some("track".into());
        t1.album_image_href = Some("album".into());
        let mut t2 = row();
        t2.album_image_href = Some("album-only".into());
        let tracks = [t1, t2];
        let vm = PlaylistDetailVm::new(&pl, &tracks);
        let rows = vm.track_rows();
        assert_eq!(rows[0].thumb_url(), Some("track"));
        assert_eq!(rows[1].thumb_url(), Some("album-only"));
    }

    fn feed_view_with(title: Option<&str>, artist: Option<&str>) -> FeedView {
        FeedView {
            title: title.map(str::to_string),
            artist: artist.map(str::to_string),
            ..FeedView::default()
        }
    }

    fn library_tree() -> LibraryTree {
        let mut rhubarb = row();
        rhubarb.id = 1;
        rhubarb.track_title = Some("Rhubarb".into());
        let mut cliffs = row();
        cliffs.id = 2;
        cliffs.track_title = Some("Cliffs".into());
        let mut windowlicker = row();
        windowlicker.id = 3;
        windowlicker.track_title = Some("Windowlicker".into());

        LibraryTree {
            artists: vec![
                ArtistNode {
                    name: "Aphex Twin".into(),
                    albums: vec![
                        AlbumNode {
                            name: "Selected Ambient Works".into(),
                            feed_id: Some(10),
                            feed_url: Some("https://example.test/saw.xml".into()),
                            image_href: Some("saw.jpg".into()),
                            tracks: vec![rhubarb, cliffs],
                        },
                        AlbumNode {
                            name: "Windowlicker".into(),
                            feed_id: Some(20),
                            feed_url: None,
                            image_href: None,
                            tracks: vec![windowlicker],
                        },
                    ],
                },
                ArtistNode {
                    name: "Autechre".into(),
                    albums: vec![AlbumNode {
                        name: "Tri Repetae".into(),
                        feed_id: Some(30),
                        feed_url: None,
                        image_href: None,
                        tracks: vec![row()],
                    }],
                },
            ],
        }
    }

    #[test]
    fn album_detail_vm_falls_back_to_untitled_and_unknown_artist() {
        let view = FeedView::default();
        let mb = BTreeMap::new();
        let vm = LibraryAlbumDetailVm::new(&view, &[], &mb);
        assert_eq!(vm.title(), "Untitled");
        assert_eq!(vm.artist(), "Unknown Artist");
    }

    #[test]
    fn album_detail_vm_uses_provided_title_and_artist_when_present() {
        let view = feed_view_with(Some("Selected Ambient Works"), Some("Aphex Twin"));
        let mb = BTreeMap::new();
        let vm = LibraryAlbumDetailVm::new(&view, &[], &mb);
        assert_eq!(vm.title(), "Selected Ambient Works");
        assert_eq!(vm.artist(), "Aphex Twin");
    }

    #[test]
    fn album_detail_vm_detail_rows_minimum_set_is_artist_and_tracks() {
        let view = feed_view_with(None, Some("A"));
        let mb = BTreeMap::new();
        let vm = LibraryAlbumDetailVm::new(&view, &[], &mb);
        let rows = vm.detail_rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], ("Artist".into(), "A".into()));
        assert_eq!(rows[1], ("Tracks".into(), "0 tracks".into()));
    }

    #[test]
    fn album_detail_vm_pluralises_tracks_count() {
        let view = feed_view_with(None, Some("A"));
        let mb = BTreeMap::new();
        let tracks = [row()];
        let vm = LibraryAlbumDetailVm::new(&view, &tracks, &mb);
        let rows = vm.detail_rows();
        assert_eq!(rows[1], ("Tracks".into(), "1 track".into()));
    }

    #[test]
    fn album_detail_vm_includes_duration_when_known() {
        let view = feed_view_with(None, None);
        let mb = BTreeMap::new();
        let mut t = row();
        t.duration_seconds = Some(125);
        let tracks = [t];
        let vm = LibraryAlbumDetailVm::new(&view, &tracks, &mb);
        let rows = vm.detail_rows();
        assert!(rows.iter().any(|(k, v)| k == "Duration" && v == "2:05"));
    }

    #[test]
    fn album_detail_vm_includes_downloaded_count_when_any_local_path_present() {
        let view = feed_view_with(None, None);
        let mb = BTreeMap::new();
        let mut t = row();
        t.local_path = Some("/x".into());
        let tracks = [t, row()];
        let vm = LibraryAlbumDetailVm::new(&view, &tracks, &mb);
        let rows = vm.detail_rows();
        assert!(rows.iter().any(|(k, v)| k == "Downloaded" && v == "1"));
    }

    #[test]
    fn album_detail_vm_omits_duration_and_downloaded_rows_when_zero() {
        let view = feed_view_with(None, None);
        let mb = BTreeMap::new();
        let tracks = [row(), row()];
        let vm = LibraryAlbumDetailVm::new(&view, &tracks, &mb);
        let rows = vm.detail_rows();
        assert!(!rows.iter().any(|(k, _)| k == "Duration"));
        assert!(!rows.iter().any(|(k, _)| k == "Downloaded"));
    }

    #[test]
    fn album_detail_vm_has_active_musicbrainz_when_any_track_pending_or_processing() {
        let view = feed_view_with(None, None);
        let mut tracks = [row(), row(), row()];
        tracks[0].id = 10;
        tracks[1].id = 20;
        tracks[2].id = 30;
        let mut mb: BTreeMap<i64, MbTrackStatus> = BTreeMap::new();
        mb.insert(10, MbTrackStatus::Done(2));
        let vm = LibraryAlbumDetailVm::new(&view, &tracks, &mb);
        assert!(!vm.has_active_musicbrainz());
        mb.insert(20, MbTrackStatus::Pending);
        let vm = LibraryAlbumDetailVm::new(&view, &tracks, &mb);
        assert!(vm.has_active_musicbrainz());
        mb.insert(20, MbTrackStatus::Processing);
        let vm = LibraryAlbumDetailVm::new(&view, &tracks, &mb);
        assert!(vm.has_active_musicbrainz());
        mb.insert(20, MbTrackStatus::Skipped("err".into()));
        let vm = LibraryAlbumDetailVm::new(&view, &tracks, &mb);
        assert!(!vm.has_active_musicbrainz());
    }

    #[test]
    fn library_view_model_starts_with_playlists_expanded_and_default_sort() {
        let vm = LibraryViewModel::new();
        assert!(vm.playlists_expanded);
        assert_eq!(vm.playlist_sort, PlaylistSort::Name);
        assert!(vm.expanded_artists.is_empty());
        assert!(vm.expanded_albums.is_empty());
    }

    #[test]
    fn library_view_model_starts_with_no_selection_or_operation_in_flight() {
        let vm = LibraryViewModel::new();
        assert_eq!(vm.selected_id, None);
        assert_eq!(vm.selected_playlist_id, None);
        assert_eq!(vm.hovered_thumb_url, None);
        assert_eq!(vm.busy_track, None);
        assert!(vm.status.is_empty());
        assert!(vm.search_query.is_empty());
        assert!(!vm.creating_playlist);
        assert!(!vm.album_add_open_feed);
        assert_eq!(vm.album_add_open_track, None);
        assert!(!vm.is_resizing());
        assert_width_eq(vm.split_pane_width(), DEFAULT_SPLIT_PANE_WIDTH);
    }

    #[test]
    fn library_view_model_tracks_resize_lifecycle_and_width() {
        let mut vm = LibraryViewModel::new();

        vm.begin_resize();
        assert!(vm.is_resizing());

        vm.resize_split_pane(120.0, 200.0, 800.0);
        assert_width_eq(vm.split_pane_width(), 200.0);

        vm.resize_split_pane(900.0, 200.0, 800.0);
        assert_width_eq(vm.split_pane_width(), 800.0);

        vm.resize_split_pane(420.0, 200.0, 800.0);
        assert_width_eq(vm.split_pane_width(), 420.0);

        vm.end_resize();
        assert!(!vm.is_resizing());
    }

    #[test]
    fn library_view_model_starts_with_empty_snapshots_and_idle_feed_update() {
        let vm = LibraryViewModel::new();
        assert!(vm.tree().artists.is_empty());
        assert!(vm.playlists().is_empty());
        assert!(vm.snapshot.playlist_tracks.is_empty());
        assert!(vm.mb_status().is_empty());
        assert!(vm.snapshot.staged_musicbrainz.is_empty());
        assert!(vm.snapshot.in_flight_feed_checks.is_empty());
        assert_eq!(vm.feed_update_state().phase, FeedUpdatePhase::Idle);
        assert!(vm.feed_update_state().status_message.is_none());
        assert!(vm.feed_update_state().stale.is_empty());
    }

    #[test]
    fn library_view_model_tree_projection_uses_saved_expansion_without_query() {
        let mut vm = LibraryViewModel::new();
        vm.replace_tree(library_tree());
        vm.toggle_artist("Aphex Twin");
        vm.toggle_album("Aphex Twin", "Selected Ambient Works");

        let projection = vm.tree_projection();

        assert_eq!(projection.tree.artists.len(), 2);
        assert!(projection.expanded_artists.contains("Aphex Twin"));
        assert!(projection
            .expanded_albums
            .contains(&("Aphex Twin".into(), "Selected Ambient Works".into())));
    }

    #[test]
    fn library_view_model_tree_projection_filters_tracks_and_expands_matches() {
        let mut vm = LibraryViewModel::new();
        vm.replace_tree(library_tree());
        vm.apply_search_query("  cliff  ");

        let projection = vm.tree_projection();

        assert_eq!(projection.tree.artists.len(), 1);
        assert_eq!(projection.tree.artists[0].name, "Aphex Twin");
        assert_eq!(projection.tree.artists[0].albums.len(), 1);
        assert_eq!(
            projection.tree.artists[0].albums[0].tracks[0]
                .track_title
                .as_deref(),
            Some("Cliffs")
        );
        assert!(projection.expanded_artists.contains("Aphex Twin"));
        assert!(projection
            .expanded_albums
            .contains(&("Aphex Twin".into(), "Selected Ambient Works".into())));
    }

    #[test]
    fn library_view_model_tree_projection_album_match_keeps_all_album_tracks() {
        let mut vm = LibraryViewModel::new();
        vm.replace_tree(library_tree());
        vm.apply_search_query("ambient");

        let projection = vm.tree_projection();

        assert_eq!(projection.tree.artists.len(), 1);
        assert_eq!(projection.tree.artists[0].albums.len(), 1);
        assert_eq!(projection.tree.artists[0].albums[0].tracks.len(), 2);
    }

    #[test]
    fn library_view_model_apply_search_query_clears_track_selection() {
        let mut vm = LibraryViewModel::new();
        vm.select_library_item(99);

        vm.apply_search_query("aphex");

        assert_eq!(vm.selected_id(), None);
        assert_eq!(vm.search_query(), "aphex");
    }

    #[test]
    fn library_view_model_toggle_artist_round_trip() {
        let mut vm = LibraryViewModel::new();
        assert!(!vm.is_artist_expanded("Aphex"));
        vm.toggle_artist("Aphex");
        assert!(vm.is_artist_expanded("Aphex"));
        vm.toggle_artist("Aphex");
        assert!(!vm.is_artist_expanded("Aphex"));
    }

    #[test]
    fn library_view_model_toggle_album_keys_on_artist_and_album() {
        let mut vm = LibraryViewModel::new();
        vm.toggle_album("Aphex", "SAW II");
        assert!(vm.is_album_expanded("Aphex", "SAW II"));
        // Different (artist, album) combination is independent.
        assert!(!vm.is_album_expanded("Aphex", "Drukqs"));
        assert!(!vm.is_album_expanded("Other", "SAW II"));
        vm.toggle_album("Aphex", "SAW II");
        assert!(!vm.is_album_expanded("Aphex", "SAW II"));
    }

    #[test]
    fn library_view_model_toggle_playlists_expanded_flips_flag() {
        let mut vm = LibraryViewModel::new();
        assert!(vm.playlist_sidebar().expanded);
        vm.toggle_playlists_expanded();
        assert!(!vm.playlist_sidebar().expanded);
        vm.toggle_playlists_expanded();
        assert!(vm.playlist_sidebar().expanded);
    }

    #[test]
    fn library_view_model_cycle_playlist_sort_advances_through_three_states() {
        let mut vm = LibraryViewModel::new();
        assert_eq!(vm.playlist_sort_label(), "A–Z");
        vm.cycle_playlist_sort();
        assert_eq!(vm.playlist_sort_label(), "Recent");
        vm.cycle_playlist_sort();
        assert_eq!(vm.playlist_sort_label(), "Size");
        vm.cycle_playlist_sort();
        assert_eq!(vm.playlist_sort_label(), "A–Z");
    }

    #[test]
    fn library_view_model_playlist_sort_label_reflects_active_sort() {
        let mut vm = LibraryViewModel::new();
        assert_eq!(vm.playlist_sort_label(), "A–Z");
        vm.cycle_playlist_sort();
        assert_eq!(vm.playlist_sort_label(), "Recent");
        vm.cycle_playlist_sort();
        assert_eq!(vm.playlist_sort_label(), "Size");
    }

    #[test]
    fn library_view_model_replaces_and_sorts_playlists_by_active_sort() {
        let mut vm = LibraryViewModel::new();
        let mut alpha = playlist("Alpha");
        alpha.id = 10;
        alpha.track_count = 3;
        alpha.updated_at = 1;
        let mut zed = playlist("zed");
        zed.id = 20;
        zed.track_count = 9;
        zed.updated_at = 5;

        vm.replace_playlists(vec![zed.clone(), alpha.clone()]);
        assert_eq!(vm.playlists()[0].id, alpha.id);
        assert_eq!(
            vm.playlist_by_id(zed.id).map(|playlist| playlist.name),
            Some("zed".into())
        );

        vm.cycle_playlist_sort();
        vm.sort_loaded_playlists();
        assert_eq!(vm.playlists()[0].id, zed.id);

        vm.cycle_playlist_sort();
        vm.sort_loaded_playlists();
        assert_eq!(vm.playlists()[0].track_count, 9);
    }

    #[test]
    fn library_view_model_playlist_sidebar_projects_rows_and_header_state() {
        let mut vm = LibraryViewModel::new();
        let mut alpha = playlist("Alpha");
        alpha.id = 10;
        alpha.track_count = 3;
        let mut zed = playlist("zed");
        zed.id = 20;
        zed.track_count = 9;
        vm.select_playlist(20);
        vm.toggle_creating_playlist();

        vm.replace_playlists(vec![zed, alpha]);
        let sidebar = vm.playlist_sidebar();

        assert!(sidebar.expanded);
        assert_eq!(sidebar.disclosure_glyph, "\u{25BC}");
        assert_eq!(sidebar.sort_label, "A–Z");
        assert!(sidebar.creating_playlist);
        assert_eq!(sidebar.rows.len(), 2);
        assert_eq!(sidebar.rows[0].id, 10);
        assert_eq!(sidebar.rows[0].element_id, "playlist-10");
        assert_eq!(sidebar.rows[0].name, "Alpha");
        assert_eq!(sidebar.rows[0].track_count_label, "(3)");
        assert!(!sidebar.rows[0].selected);
        assert!(sidebar.rows[1].selected);
    }

    #[test]
    fn library_view_model_playlist_sidebar_reflects_collapsed_state() {
        let mut vm = LibraryViewModel::new();
        vm.toggle_playlists_expanded();

        let sidebar = vm.playlist_sidebar();

        assert!(!sidebar.expanded);
        assert_eq!(sidebar.disclosure_glyph, "\u{25B6}");
    }

    #[test]
    fn library_view_model_toggle_creating_playlist_flips_flag() {
        let mut vm = LibraryViewModel::new();
        assert!(!vm.playlist_sidebar().creating_playlist);
        vm.toggle_creating_playlist();
        assert!(vm.playlist_sidebar().creating_playlist);
        vm.close_creating_playlist();
        assert!(!vm.playlist_sidebar().creating_playlist);
        vm.toggle_creating_playlist();
        assert!(vm.playlist_sidebar().creating_playlist);
        vm.toggle_creating_playlist();
        assert!(!vm.playlist_sidebar().creating_playlist);
    }

    #[test]
    fn library_view_model_tracks_playlist_snapshot_replacement() {
        let mut vm = LibraryViewModel::new();
        let mut track = row();
        track.id = 42;

        vm.replace_playlist_tracks(vec![track]);

        assert_eq!(vm.snapshot.playlist_tracks.len(), 1);
        assert_eq!(vm.snapshot.playlist_tracks[0].id, 42);
    }

    #[test]
    fn library_view_model_library_reload_status_pluralizes_track_count() {
        let mut vm = LibraryViewModel::new();

        vm.finish_library_reload(1);
        assert_eq!(vm.status(), "1 library track");

        vm.finish_library_reload(2);
        assert_eq!(vm.status(), "2 library tracks");
    }

    #[test]
    fn library_view_model_playlist_failures_format_status_text() {
        let mut vm = LibraryViewModel::new();

        vm.fail_playlist_load("db");
        assert_eq!(vm.status(), "Error loading playlists: db");
        vm.fail_playlist_create("exists");
        assert_eq!(vm.status(), "Error creating playlist: exists");
        vm.fail_playlist_rename("bad");
        assert_eq!(vm.status(), "Error renaming: bad");
        vm.fail_playlist_delete("missing");
        assert_eq!(vm.status(), "Error deleting: missing");
        vm.fail_playlist_track_remove("locked");
        assert_eq!(vm.status(), "Error removing track: locked");
        vm.fail_playlist_track_reorder("position");
        assert_eq!(vm.status(), "Error reordering: position");
    }

    #[test]
    fn library_view_model_album_track_load_status_helpers_format_text() {
        let mut vm = LibraryViewModel::new();

        vm.fail_album_tracks_load("db");
        assert_eq!(vm.status(), "Error loading album tracks: db");
        vm.set_album_has_no_tracks();
        assert_eq!(vm.status(), "Album has no tracks");
    }

    #[test]
    fn library_view_model_playlist_append_intent_sets_status_and_playlist_name() {
        let mut vm = LibraryViewModel::new();
        let mut playlist = playlist("Focus");
        playlist.id = 12;
        vm.replace_playlists(vec![playlist]);

        let intent = vm
            .begin_playlist_append(12, vec![7, 8])
            .expect("non-empty track ids should build an append intent");

        assert_eq!(intent.playlist_id(), 12);
        assert_eq!(intent.playlist_name(), "Focus");
        assert_eq!(intent.total_tracks(), 2);
        assert_eq!(intent.track_ids(), &[7, 8]);
        assert_eq!(vm.status(), "Downloading 2 tracks...");
    }

    #[test]
    fn library_view_model_playlist_append_ignores_empty_track_ids() {
        let mut vm = LibraryViewModel::new();
        vm.finish_library_reload(1);

        assert!(vm.begin_playlist_append(12, Vec::new()).is_none());
        assert_eq!(vm.status(), "1 library track");
    }

    #[test]
    fn library_view_model_playlist_append_finish_formats_counts() {
        let mut vm = LibraryViewModel::new();
        let mut playlist = playlist("Focus");
        playlist.id = 12;
        vm.replace_playlists(vec![playlist]);
        let intent = vm
            .begin_playlist_append(12, vec![7, 8, 9])
            .expect("non-empty track ids should build an append intent");

        vm.finish_playlist_append(&intent, PlaylistAppendOutcome::new(2, 1, 1));

        assert_eq!(
            vm.status(),
            "Added 2 of 3 to Focus (downloaded 1); 1 failed"
        );
    }

    #[test]
    fn library_view_model_playlist_append_finish_omits_zero_optional_counts() {
        let mut vm = LibraryViewModel::new();
        let mut playlist = playlist("Focus");
        playlist.id = 12;
        vm.replace_playlists(vec![playlist]);
        let intent = vm
            .begin_playlist_append(12, vec![7])
            .expect("non-empty track ids should build an append intent");

        vm.finish_playlist_append(&intent, PlaylistAppendOutcome::new(1, 0, 0));

        assert_eq!(vm.status(), "Added 1 of 1 to Focus");
    }

    #[test]
    fn library_view_model_playlist_append_failure_sets_error_status() {
        let mut vm = LibraryViewModel::new();

        vm.fail_playlist_append("offline");

        assert_eq!(vm.status(), "Error adding to playlist: offline");
    }

    #[test]
    fn library_view_model_selection_methods_keep_sidebar_and_tree_exclusive() {
        let mut vm = LibraryViewModel::new();
        vm.select_playlist(7);
        assert_eq!(vm.selected_id(), None);
        assert_eq!(vm.selected_playlist_id(), Some(7));
        assert!(vm.is_playlist_selected(7));

        vm.select_library_item(42);
        assert_eq!(vm.selected_id(), Some(42));
        assert_eq!(vm.selected_playlist_id(), None);

        vm.clear_library_selection();
        assert_eq!(vm.selected_id(), None);
    }

    #[test]
    fn library_view_model_clears_matching_playlist_selection_only() {
        let mut vm = LibraryViewModel::new();
        vm.select_playlist(7);

        assert!(!vm.clear_playlist_selection_if(8));
        assert_eq!(vm.selected_playlist_id(), Some(7));
        assert!(vm.clear_playlist_selection_if(7));
        assert_eq!(vm.selected_playlist_id(), None);
    }

    #[test]
    fn library_view_model_album_feed_picker_toggles_and_closes() {
        let mut vm = LibraryViewModel::new();
        assert!(!vm.album_feed_picker_open());
        vm.toggle_album_feed_picker();
        assert!(vm.album_feed_picker_open());
        vm.close_album_feed_picker();
        assert!(!vm.album_feed_picker_open());
    }

    #[test]
    fn library_view_model_album_track_picker_toggles_by_track_id() {
        let mut vm = LibraryViewModel::new();
        assert_eq!(vm.album_track_picker_open(), None);
        vm.toggle_album_track_picker(10);
        assert_eq!(vm.album_track_picker_open(), Some(10));
        vm.toggle_album_track_picker(10);
        assert_eq!(vm.album_track_picker_open(), None);
        vm.toggle_album_track_picker(20);
        assert_eq!(vm.album_track_picker_open(), Some(20));
        vm.close_album_track_picker();
        assert_eq!(vm.album_track_picker_open(), None);
    }

    #[test]
    fn library_view_model_hovered_thumb_reports_only_changes() {
        let mut vm = LibraryViewModel::new();
        assert_eq!(vm.hovered_thumb_url(), None);
        assert!(vm.set_hovered_thumb_url(Some("img".into())));
        assert_eq!(vm.hovered_thumb_url(), Some("img"));
        assert!(!vm.set_hovered_thumb_url(Some("img".into())));
        assert!(vm.set_hovered_thumb_url(None));
        assert_eq!(vm.hovered_thumb_url(), None);
    }

    #[test]
    fn library_view_model_busy_track_and_status_transition_together() {
        let mut vm = LibraryViewModel::new();
        assert!(!vm.has_busy_track());
        assert_eq!(vm.busy_track(), None);

        vm.begin_busy_track(42, "Subscribing track...");

        assert!(vm.has_busy_track());
        assert_eq!(vm.busy_track(), Some(42));
        assert_eq!(vm.status(), "Subscribing track...");

        vm.clear_busy_track();
        assert_eq!(vm.busy_track(), None);
    }

    #[test]
    fn library_track_action_vm_formats_subscription_labels() {
        assert_eq!(
            LibraryTrackActionVm::new(false, false, false, None).subscription_button_label(),
            "Subscribe Track"
        );
        assert_eq!(
            LibraryTrackActionVm::new(false, true, false, None).subscription_button_label(),
            "Unsubscribe Track"
        );
        assert_eq!(
            LibraryTrackActionVm::new(true, false, false, None).subscription_button_label(),
            "Subscribing..."
        );
        assert_eq!(
            LibraryTrackActionVm::new(true, true, false, None).subscription_button_label(),
            "Unsubscribing..."
        );
    }

    #[test]
    fn library_track_action_vm_formats_playlist_label_and_message_status() {
        let closed = LibraryTrackActionVm::new(false, false, false, Some("Subscribed"));
        assert_eq!(closed.add_to_playlist_label(), "Add to playlist ▾");
        assert_eq!(closed.subscription_message(), Some("Subscribed"));
        assert!(!closed.message_is_error());

        let open = LibraryTrackActionVm::new(false, false, true, Some("Error: offline"));
        assert_eq!(open.add_to_playlist_label(), "Add to playlist ▴");
        assert!(open.message_is_error());
    }

    #[test]
    fn library_view_model_track_subscribe_finish_clears_busy_and_formats_warning() {
        let mut vm = LibraryViewModel::new();
        vm.begin_busy_track(42, "Subscribing track...");

        vm.finish_track_subscribe(TrackSubscribeOutcome::new(
            "/tmp/song.mp3",
            Some("converted from WAV".into()),
        ));

        assert_eq!(vm.busy_track(), None);
        assert_eq!(
            vm.status(),
            "Subscribed track: /tmp/song.mp3 — converted from WAV"
        );
    }

    #[test]
    fn library_view_model_track_subscribe_failure_clears_busy_and_sets_error() {
        let mut vm = LibraryViewModel::new();
        vm.begin_busy_track(42, "Subscribing track...");

        vm.fail_track_subscribe("offline");

        assert_eq!(vm.busy_track(), None);
        assert_eq!(vm.status(), "Error subscribing track: offline");
    }

    #[test]
    fn library_view_model_status_helper_sets_error_text() {
        let mut vm = LibraryViewModel::new();
        vm.set_error_status("broken");
        assert_eq!(vm.status(), "Error: broken");
    }

    #[test]
    fn library_view_model_tracks_musicbrainz_status_and_staged_lookup() {
        let mut vm = LibraryViewModel::new();
        vm.set_mb_status(7, MbTrackStatus::Processing);
        assert!(vm.has_mb_status(7));
        assert!(matches!(
            vm.mb_status().get(&7),
            Some(MbTrackStatus::Processing)
        ));

        vm.mark_musicbrainz_pending([8, 9]);
        assert!(matches!(
            vm.mb_status().get(&8),
            Some(MbTrackStatus::Pending)
        ));
        assert!(matches!(
            vm.mb_status().get(&9),
            Some(MbTrackStatus::Pending)
        ));

        let lookup = MusicBrainzLookupResult {
            lookup: crate::musicbrainz::MusicBrainzLookup {
                query: "track".into(),
                candidates: Vec::new(),
            },
            image: None,
        };
        vm.stage_musicbrainz(7, lookup);
        assert_eq!(
            vm.staged_musicbrainz(7)
                .map(|lookup| lookup.lookup.query.as_str()),
            Some("track")
        );

        vm.clear_mb_status();
        assert!(vm.mb_status().is_empty());
    }

    #[test]
    fn library_view_model_musicbrainz_track_lookup_transitions_are_pure() {
        let mut vm = LibraryViewModel::new();
        assert!(vm.begin_musicbrainz_track_lookup(7));
        assert!(!vm.begin_musicbrainz_track_lookup(7));
        assert_eq!(vm.status(), "MusicBrainz lookup...");
        assert!(matches!(
            vm.mb_status().get(&7),
            Some(MbTrackStatus::Processing)
        ));

        vm.finish_musicbrainz_track_lookup(7, 1);
        assert_eq!(vm.status(), "MusicBrainz: staged 1 edit");
        assert!(matches!(
            vm.mb_status().get(&7),
            Some(MbTrackStatus::Done(1))
        ));

        vm.fail_musicbrainz_track_lookup(8, "offline");
        assert_eq!(vm.status(), "MusicBrainz error: offline");
        assert!(matches!(
            vm.mb_status().get(&8),
            Some(MbTrackStatus::Skipped(message)) if message == "offline"
        ));
    }

    #[test]
    fn library_view_model_musicbrainz_album_lookup_transitions_are_pure() {
        let mut vm = LibraryViewModel::new();
        assert!(!vm.begin_musicbrainz_album_lookup([]));
        assert_eq!(vm.status(), "No downloaded tracks to process");

        assert!(vm.begin_musicbrainz_album_lookup([7, 8]));
        assert_eq!(vm.status(), "MusicBrainz: album lookup for 2 tracks...");
        assert!(matches!(
            vm.mb_status().get(&7),
            Some(MbTrackStatus::Pending)
        ));
        assert!(matches!(
            vm.mb_status().get(&8),
            Some(MbTrackStatus::Pending)
        ));

        vm.fail_musicbrainz_album_lookup_with_fallback("offline");
        assert_eq!(
            vm.status(),
            "Album lookup failed (offline), falling back to per-track..."
        );
        vm.fallback_empty_musicbrainz_album_lookup();
        assert_eq!(
            vm.status(),
            "Album lookup: no results, falling back to per-track..."
        );

        vm.begin_musicbrainz_album_track_stage(7, 1, 2);
        assert_eq!(vm.status(), "MusicBrainz: staging track 1/2 ...");
        assert!(matches!(
            vm.mb_status().get(&7),
            Some(MbTrackStatus::Processing)
        ));
        vm.finish_musicbrainz_album_track_stage(7, MbTrackStatus::Done(2));
        assert!(matches!(
            vm.mb_status().get(&7),
            Some(MbTrackStatus::Done(2))
        ));

        vm.finish_musicbrainz_album_lookup(1, 2);
        assert_eq!(vm.status(), "MusicBrainz: staged 1 edit across 2 tracks");
        vm.finish_musicbrainz_album_lookup(3, 2);
        assert_eq!(vm.status(), "MusicBrainz: staged 3 edits across 2 tracks");
    }

    #[test]
    fn library_view_model_single_feed_check_dedupes_and_records_stale() {
        let mut vm = LibraryViewModel::new();
        assert!(vm.begin_feed_view_check(42));
        assert!(!vm.begin_feed_view_check(42));
        assert_eq!(
            vm.feed_update_state().status_message.as_deref(),
            Some("Checking feed...")
        );

        vm.finish_feed_view_check(
            42,
            Ok(Some(feed_service::StaleFeed {
                feed_id: 42,
                feed_guid: "feed-guid".into(),
                title: Some("Feed".into()),
                new_updated_at: 100,
            })),
        );

        assert_eq!(vm.feed_update_state().stale.len(), 1);
        assert_eq!(
            vm.feed_update_state().status_message.as_deref(),
            Some("1 feed update pending")
        );
    }

    #[test]
    fn library_view_model_bulk_feed_check_and_apply_transitions_are_pure() {
        let mut vm = LibraryViewModel::new();
        vm.begin_all_feed_check(2);
        assert_eq!(vm.feed_update_state().phase, FeedUpdatePhase::Checking);
        assert_eq!(
            vm.feed_update_state().status_message.as_deref(),
            Some("Checking 2 feeds...")
        );

        vm.finish_all_feed_check(vec![feed_service::StaleFeed {
            feed_id: 1,
            feed_guid: "one".into(),
            title: None,
            new_updated_at: 9,
        }]);
        assert_eq!(vm.feed_update_state().phase, FeedUpdatePhase::Idle);
        assert_eq!(
            vm.feed_update_state().status_message.as_deref(),
            Some("1 feed update available")
        );

        let stale = vm
            .begin_apply_feed_updates()
            .expect("stale feeds should apply");
        assert_eq!(stale.len(), 1);
        assert_eq!(vm.feed_update_state().phase, FeedUpdatePhase::Applying);
        vm.finish_apply_feed_updates("Done".into());
        assert_eq!(vm.feed_update_state().phase, FeedUpdatePhase::Idle);
        assert!(vm.feed_update_state().stale.is_empty());
        assert_eq!(
            vm.feed_update_state().status_message.as_deref(),
            Some("Done")
        );
    }

    #[test]
    fn album_detail_vm_playlist_action_label_flips_arrow_glyph_when_open() {
        let view = feed_view_with(None, None);
        let mb = BTreeMap::new();
        let vm = LibraryAlbumDetailVm::new(&view, &[], &mb);
        let closed = vm
            .playlist_action_vm(7, false)
            .expect("closed playlist action should render");
        let open = vm
            .playlist_action_vm(7, true)
            .expect("open playlist action should render");

        assert_eq!(closed.label, "Add feed to playlist ▾");
        assert_eq!(open.label, "Add feed to playlist ▴");
    }

    #[test]
    fn album_detail_vm_release_actions_use_shared_feed_vocabulary() {
        let view = feed_view_with(None, None);
        let mb = BTreeMap::new();
        let vm = LibraryAlbumDetailVm::new(&view, &[], &mb);
        let primary = vm.primary_action_vm(7, false);
        let busy = vm.primary_action_vm(7, true);
        let playlist = vm
            .playlist_action_vm(7, true)
            .expect("playlist action should render");

        assert_eq!(primary.label, "Remove Feed");
        assert!(primary.enabled);
        assert_eq!(busy.label, "Removing...");
        assert!(!busy.enabled);
        assert_eq!(playlist.label, "Add feed to playlist ▴");
    }
}
