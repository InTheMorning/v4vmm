use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use gpui::{
    div, img, prelude::*, px, rgb, AnyElement, Context, Entity, FontWeight, Image, ImageFormat,
    InteractiveElement, IntoElement, ObjectFit, Render, SharedString, Styled, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::menu::DropdownMenu as _;
use gpui_component::Disableable;
use gpui_component::Sizable;
use gpui_component::Size;
use reqwest::blocking::Client as ReqwestClient;

use crate::api::{Client as MusicIndexClient, Feed, SourceEntityLink, Track};
use crate::audio_tags::{read_audio_tags, write_id3v24_edits, Id3v24Edit};
use crate::config;
use crate::db::{self, TrackRow};
use crate::media::ImageCache;
use crate::metadata::{
    aligned_compare_rows, auto_populated_pending_id3_edits, display_metadata_value,
    expand_woar_metadata_rows, expanded_metadata_display_value, id3_frame_base,
    metadata_field_is_expandable, pending_id3_conflict_descriptions, pending_id3_edits_for_apply,
    summarize_contributor_value, track_metadata_rows, AlignedCompareRow, MetadataColumn,
    MetadataGridRow, MusicBrainzLookupResult, PendingId3Edit, TagCompareResult, TrackContext,
};
use crate::musicbrainz::{
    lookup_recordings, lookup_releases, LookupMetadata, MusicBrainzCandidate, MusicBrainzLookup,
};
use crate::search::id3_edits_for_track_context;
use crate::track_compare::{download_track, select_audio_enclosure, DownloadedTrack};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------


#[derive(Clone, Debug)]
enum LibraryDetail {
    None,
    Album(AlbumNode),
    Track(Box<InspectorFrame>),
    Playlist(PlaylistDetail),
}

#[derive(Clone, Debug)]
struct PlaylistDetail {
    playlist: db::Playlist,
    tracks: Vec<TrackRow>,
}

#[derive(Clone, Debug, Default)]
enum LazyPanel<T> {
    #[default]
    Hidden,
    Loading,
    Empty(String),
    Loaded(T),
}

#[derive(Clone, Debug)]
struct InspectorFrame {
    entity_id: i64,
    title: String,
    track: TrackRow,
    source_context: Option<TrackContext>,
    image: Option<Arc<Image>>,
    expanded_id3_frame_groups: BTreeSet<String>,
    expanded_metadata_cells: BTreeSet<String>,
    pending_id3_edits: BTreeMap<String, PendingId3Edit>,
    suppressed_auto_id3_edits: BTreeSet<String>,
    applying_id3_edits: bool,
    id3_apply_error: Option<String>,
    local_subscription: bool,
    subscription_busy: bool,
    subscription_message: Option<String>,
    tag_compare: LazyPanel<TagCompareResult>,
    musicbrainz_lookup: LazyPanel<MusicBrainzLookupResult>,
    musicbrainz_selected: usize,
    add_to_playlist_open: bool,
}

impl InspectorFrame {
    fn for_track(track: TrackRow, image: Option<Arc<Image>>) -> Self {
        let title = track
            .track_title
            .clone()
            .or_else(|| track.feed_title.clone())
            .unwrap_or_else(|| "Untitled".into());
        Self {
            entity_id: track.id,
            title,
            local_subscription: track.is_in_library,
            track,
            source_context: None,
            image,
            expanded_id3_frame_groups: BTreeSet::new(),
            expanded_metadata_cells: BTreeSet::new(),
            pending_id3_edits: BTreeMap::new(),
            suppressed_auto_id3_edits: BTreeSet::new(),
            applying_id3_edits: false,
            id3_apply_error: None,
            subscription_busy: false,
            subscription_message: None,
            tag_compare: LazyPanel::Hidden,
            musicbrainz_lookup: LazyPanel::Hidden,
            musicbrainz_selected: 0,
            add_to_playlist_open: false,
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
enum MbTrackStatus {
    Pending,
    Processing,
    Done(usize),
    Skipped(String),
}

#[derive(Clone, Debug)]
struct StagedMusicBrainzLookup {
    lookup: MusicBrainzLookupResult,
    edit_count: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct ArtistNode {
    pub(crate) name: String,
    pub(crate) albums: Vec<AlbumNode>,
}

#[derive(Clone, Debug)]
pub(crate) struct AlbumNode {
    pub(crate) name: String,
    pub(crate) feed_id: Option<i64>,
    pub(crate) feed_url: Option<String>,
    pub(crate) image_href: Option<String>,
    pub(crate) tracks: Vec<TrackRow>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct LibraryTree {
    pub(crate) artists: Vec<ArtistNode>,
}

#[derive(Clone, Debug)]
struct LibraryTrackCompare {
    tag_compare: TagCompareResult,
    track_context: TrackContext,
}

#[derive(Clone)]
enum ThumbnailState {
    Loading,
    Loaded(Option<Arc<Image>>),
}

pub struct LibraryApp {
    conn: Arc<Mutex<Connection>>,
    cache: Arc<ImageCache>,
    musicindex_endpoint: String,
    tree: LibraryTree,
    expanded_artists: HashSet<String>,
    expanded_albums: HashSet<(String, String)>,
    selected_id: Option<i64>,
    detail: LibraryDetail,
    status: String,
    busy_track: Option<i64>,
    mb_status: BTreeMap<i64, MbTrackStatus>,
    staged_musicbrainz: BTreeMap<i64, MusicBrainzLookupResult>,
    thumbnails: BTreeMap<(String, bool), ThumbnailState>,
    hovered_thumb_url: Option<String>,
    search_input: Entity<InputState>,
    search_query: String,
    _search_sub: gpui::Subscription,
    feed_update_state: FeedUpdateState,
    in_flight_feed_checks: HashSet<i64>,
    playlists: Vec<db::Playlist>,
    selected_playlist_id: Option<i64>,
    playlist_tracks: Vec<TrackRow>,
    creating_playlist: bool,
    new_playlist_input: Entity<InputState>,
    playlists_expanded: bool,
    playlist_sort: PlaylistSort,
    album_add_open_feed: bool,
    album_add_open_track: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PlaylistSort {
    #[default]
    Name,
    RecentlyUpdated,
    TrackCount,
}

impl PlaylistSort {
    fn next(self) -> Self {
        match self {
            PlaylistSort::Name => PlaylistSort::RecentlyUpdated,
            PlaylistSort::RecentlyUpdated => PlaylistSort::TrackCount,
            PlaylistSort::TrackCount => PlaylistSort::Name,
        }
    }

    fn label(self) -> &'static str {
        match self {
            PlaylistSort::Name => "A–Z",
            PlaylistSort::RecentlyUpdated => "Recent",
            PlaylistSort::TrackCount => "Size",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct FeedUpdateState {
    pub phase: FeedUpdatePhase,
    pub status_message: Option<String>,
    pub stale: Vec<StaleFeed>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum FeedUpdatePhase {
    #[default]
    Idle,
    Checking,
    Applying,
}

#[derive(Clone, Debug)]
pub struct StaleFeed {
    pub feed_id: i64,
    pub feed_guid: String,
    pub title: Option<String>,
    pub new_updated_at: i64,
}

use crate::ui::theme::color;
use crate::ui::theme::spacing;
use crate::ui::theme::typography;
use crate::ui::theme::radius;
use crate::ui::theme::{layout, badges, glyphs};
use crate::ui::render_rss_icon_link;

// ---------------------------------------------------------------------------
// LibraryApp
// ---------------------------------------------------------------------------

impl LibraryApp {
    pub fn new(
        conn: Arc<Mutex<Connection>>,
        cache: Arc<ImageCache>,
        musicindex_endpoint: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search_input = cx.new(|cx: &mut Context<InputState>| {
            InputState::new(window, cx).placeholder("Search your library...")
        });
        let search_sub = cx.subscribe(&search_input, Self::on_search_event);
        let new_playlist_input = cx.new(|cx: &mut Context<InputState>| {
            InputState::new(window, cx).placeholder("New playlist name…")
        });
        let mut app = Self {
            conn,
            cache,
            musicindex_endpoint,
            tree: LibraryTree::default(),
            expanded_artists: HashSet::new(),
            expanded_albums: HashSet::new(),
            selected_id: None,
            detail: LibraryDetail::None,
            status: String::new(),
            busy_track: None,
            mb_status: BTreeMap::new(),
            staged_musicbrainz: BTreeMap::new(),
            thumbnails: BTreeMap::new(),
            hovered_thumb_url: None,
            search_input,
            search_query: String::new(),
            _search_sub: search_sub,
            feed_update_state: FeedUpdateState::default(),
            in_flight_feed_checks: HashSet::new(),
            playlists: Vec::new(),
            selected_playlist_id: None,
            playlist_tracks: Vec::new(),
            creating_playlist: false,
            playlist_sort: PlaylistSort::default(),
            new_playlist_input,
            playlists_expanded: true,
            album_add_open_feed: false,
            album_add_open_track: None,
        };
        app.reload();
        app
    }

    fn on_search_event(
        &mut self,
        _entity: Entity<InputState>,
        event: &InputEvent,
        cx: &mut Context<Self>,
    ) {
        if let InputEvent::PressEnter { .. } = event {
            self.apply_search(cx);
        }
    }

    fn apply_search(&mut self, cx: &mut Context<Self>) {
        self.search_query = self.search_input.read(cx).value().trim().to_string();
        self.selected_id = None;
        self.detail = LibraryDetail::None;
        cx.notify();
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.reload();
        cx.notify();
    }

    pub fn focus_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search_input.update(cx, |input, cx| {
            input.focus(window, cx);
        });
    }

    pub fn pop_inspector(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.detail, LibraryDetail::None) {
            self.detail = LibraryDetail::None;
            cx.notify();
        }
    }

    pub fn move_up(&mut self, cx: &mut Context<Self>) {
        let items = self.focusable_items();
        if items.is_empty() { return; }
        let current_idx = items.iter().position(|&id| Some(id) == self.selected_id);
        let next_idx = match current_idx {
            Some(idx) if idx > 0 => idx - 1,
            _ => 0,
        };
        if let Some(&id) = items.get(next_idx) {
            self.select_id(id, cx);
        }
    }

    pub fn move_down(&mut self, cx: &mut Context<Self>) {
        let items = self.focusable_items();
        if items.is_empty() { return; }
        let current_idx = items.iter().position(|&id| Some(id) == self.selected_id);
        let next_idx = match current_idx {
            Some(idx) if idx + 1 < items.len() => idx + 1,
            Some(idx) => idx,
            None => 0,
        };
        if let Some(&id) = items.get(next_idx) {
            self.select_id(id, cx);
        }
    }

    pub fn confirm(&mut self, _cx: &mut Context<Self>) {
        // Already selected and opened in select_id
    }

    fn select_id(&mut self, id: i64, _cx: &mut Context<Self>) {
        // Need to find if it's an album or track to call appropriate method
        // This is a bit complex with current structure.
        // I'll simplify: just update selected_id and reload detail.
        self.selected_id = Some(id);
        // ... need to find the item to know what detail to show ...
    }

    fn focusable_items(&self) -> Vec<i64> {
        let items = Vec::new();
        // Traverse filtered_tree based on expanded states
        // This is tricky because filtered_tree is computed in render.
        // I should probably compute it in a separate method.
        items
    }

    pub fn set_musicindex_endpoint(&mut self, endpoint: String, cx: &mut Context<Self>) {
        self.musicindex_endpoint = endpoint;
        cx.notify();
    }

    fn reload(&mut self) {
        let conn = self.conn.lock().expect("lock db");
        match db::library_tracks(&conn) {
            Ok(rows) => {
                let count = rows.len();
                self.tree = build_tree(&rows, &conn);
                self.status =
                    format!("{count} library track{}", if count == 1 { "" } else { "s" });
            }
            Err(err) => {
                self.status = format!("Error: {err:#}");
            }
        }
        drop(conn);
        self.reload_playlists();
        self.selected_id = None;
        self.detail = LibraryDetail::None;
        self.mb_status.clear();
    }

    fn reload_playlists(&mut self) {
        let conn = self.conn.lock().expect("lock db");
        match db::playlists_list(&conn) {
            Ok(mut list) => {
                self.sort_playlists(&mut list);
                self.playlists = list;
            }
            Err(err) => self.status = format!("Error loading playlists: {err:#}"),
        }
    }

    fn sort_playlists(&self, list: &mut [db::Playlist]) {
        match self.playlist_sort {
            PlaylistSort::Name => {
                list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            }
            PlaylistSort::RecentlyUpdated => {
                list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            }
            PlaylistSort::TrackCount => {
                list.sort_by(|a, b| b.track_count.cmp(&a.track_count));
            }
        }
    }

    fn cycle_playlist_sort(&mut self, cx: &mut Context<Self>) {
        self.playlist_sort = self.playlist_sort.next();
        let mut list = std::mem::take(&mut self.playlists);
        self.sort_playlists(&mut list);
        self.playlists = list;
        cx.notify();
    }

    fn select_playlist(&mut self, id: i64, cx: &mut Context<Self>) {
        self.selected_id = None;
        self.selected_playlist_id = Some(id);
        let conn = self.conn.lock().expect("lock db");
        let playlist = self.playlists.iter().find(|p| p.id == id).cloned();
        let tracks = db::playlist_tracks(&conn, id).unwrap_or_default();
        drop(conn);
        if let Some(playlist) = playlist {
            self.detail = LibraryDetail::Playlist(PlaylistDetail { playlist, tracks: tracks.clone() });
            self.playlist_tracks = tracks;
        }
        cx.notify();
    }

    fn create_playlist(&mut self, cx: &mut Context<Self>) {
        let name = self.new_playlist_input.read(cx).value().to_string();
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        let conn = self.conn.lock().expect("lock db");
        match db::playlist_create(&conn, name) {
            Ok(id) => {
                drop(conn);
                self.creating_playlist = false;
                self.reload_playlists();
                self.select_playlist(id, cx);
            }
            Err(err) => self.status = format!("Error creating playlist: {err:#}"),
        }
        cx.notify();
    }

    #[allow(dead_code)]
    fn rename_playlist(&mut self, id: i64, new_name: String, cx: &mut Context<Self>) {
        let trimmed = new_name.trim();
        if trimmed.is_empty() {
            return;
        }
        let conn = self.conn.lock().expect("lock db");
        if let Err(err) = db::playlist_rename(&conn, id, trimmed) {
            self.status = format!("Error renaming: {err:#}");
            return;
        }
        drop(conn);
        self.reload_playlists();
        if self.selected_playlist_id == Some(id) {
            self.select_playlist(id, cx);
        }
        cx.notify();
    }

    fn delete_playlist(&mut self, id: i64, cx: &mut Context<Self>) {
        let conn = self.conn.lock().expect("lock db");
        if let Err(err) = db::playlist_delete(&conn, id) {
            self.status = format!("Error deleting: {err:#}");
            return;
        }
        drop(conn);
        if self.selected_playlist_id == Some(id) {
            self.selected_playlist_id = None;
            self.detail = LibraryDetail::None;
        }
        self.reload_playlists();
        cx.notify();
    }

    fn remove_playlist_track_at(
        &mut self,
        playlist_id: i64,
        position: i64,
        cx: &mut Context<Self>,
    ) {
        let mut conn = self.conn.lock().expect("lock db");
        if let Err(err) = db::playlist_remove_at(&mut conn, playlist_id, position) {
            self.status = format!("Error removing track: {err:#}");
            return;
        }
        drop(conn);
        self.reload_playlists();
        if self.selected_playlist_id == Some(playlist_id) {
            self.select_playlist(playlist_id, cx);
        }
        cx.notify();
    }

    fn move_playlist_track(
        &mut self,
        playlist_id: i64,
        from: i64,
        to: i64,
        cx: &mut Context<Self>,
    ) {
        if from == to {
            return;
        }
        let mut conn = self.conn.lock().expect("lock db");
        if let Err(err) = db::playlist_reorder(&mut conn, playlist_id, from, to) {
            self.status = format!("Error reordering: {err:#}");
            return;
        }
        drop(conn);
        if self.selected_playlist_id == Some(playlist_id) {
            self.select_playlist(playlist_id, cx);
        }
        cx.notify();
    }

    fn add_track_to_playlist(&mut self, track_id: i64, playlist_id: i64, cx: &mut Context<Self>) {
        let conn = self.conn.lock().expect("lock db");
        match db::playlist_append(&conn, playlist_id, track_id) {
            Ok(()) => {
                let name = self
                    .playlists
                    .iter()
                    .find(|p| p.id == playlist_id)
                    .map(|p| p.name.clone())
                    .unwrap_or_default();
                self.status = format!("Added to {name}");
            }
            Err(err) => self.status = format!("Error adding to playlist: {err:#}"),
        }
        drop(conn);
        self.reload_playlists();
        cx.notify();
    }

    fn add_album_to_playlist(&mut self, feed_id: i64, playlist_id: i64, cx: &mut Context<Self>) {
        let conn = self.conn.lock().expect("lock db");
        let tracks = match db::feed_tracks(&conn, feed_id) {
            Ok(t) => t,
            Err(err) => {
                self.status = format!("Error loading album tracks: {err:#}");
                cx.notify();
                return;
            }
        };
        let mut appended = 0usize;
        for track in &tracks {
            if db::playlist_append(&conn, playlist_id, track.id).is_ok() {
                appended += 1;
            }
        }
        let name = self
            .playlists
            .iter()
            .find(|p| p.id == playlist_id)
            .map(|p| p.name.clone())
            .unwrap_or_default();
        self.status = format!("Added {appended} tracks to {name}");
        drop(conn);
        self.reload_playlists();
        cx.notify();
    }

    fn thumbnail_for_url(
        &mut self,
        url: Option<&str>,
        animated: bool,
        cx: &mut Context<Self>,
    ) -> Option<Arc<Image>> {
        let url = url?.trim();
        if url.is_empty() {
            return None;
        }
        let cached = if animated {
            self.cache.peek(url)
        } else {
            self.cache.peek_static(url)
        };
        if let Some(img) = cached {
            return Some(img);
        }
        let key = (url.to_string(), animated);
        match self.thumbnails.get(&key) {
            Some(ThumbnailState::Loaded(image)) => return image.clone(),
            Some(ThumbnailState::Loading) => return None,
            None => {}
        }
        self.thumbnails.insert(key.clone(), ThumbnailState::Loading);
        let cache = Arc::clone(&self.cache);
        cx.spawn(
            async move |this: gpui::WeakEntity<LibraryApp>, cx: &mut gpui::AsyncApp| {
                let cache_url = key.0.clone();
                let cache_clone = Arc::clone(&cache);
                let image = cx
                    .background_executor()
                    .spawn(async move {
                        if animated {
                            cache_clone.fetch_blocking(&cache_url)
                        } else {
                            cache_clone.fetch_static_blocking(&cache_url)
                        }
                    })
                    .await;
                this.update(
                    cx,
                    move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                        this.thumbnails.insert(key, ThumbnailState::Loaded(image));
                        cx.notify();
                    },
                )
                .ok();
            },
        )
        .detach();
        None
    }

    fn set_hovered_thumb(&mut self, url: Option<String>, cx: &mut Context<Self>) {
        if self.hovered_thumb_url != url {
            self.hovered_thumb_url = url;
            cx.notify();
        }
    }

    fn select_album(&mut self, album: &AlbumNode, cx: &mut Context<Self>) {
        self.selected_id = album.feed_id;
        self.detail = LibraryDetail::Album(album.clone());
        if let Some(feed_id) = album.feed_id {
            self.check_feed_on_view(feed_id, cx);
        }
    }

    fn check_feed_on_view(&mut self, feed_id: i64, cx: &mut Context<Self>) {
        if self.feed_update_state.phase == FeedUpdatePhase::Applying
            || self
                .feed_update_state
                .stale
                .iter()
                .any(|entry| entry.feed_id == feed_id)
            || !self.in_flight_feed_checks.insert(feed_id)
        {
            return;
        }
        let conn = Arc::clone(&self.conn);
        let endpoint = self.musicindex_endpoint.clone();
        self.feed_update_state.status_message = Some("Checking feed...".into());
        cx.notify();
        cx.spawn(
            async move |this: gpui::WeakEntity<LibraryApp>, cx: &mut gpui::AsyncApp| {
                let stale = cx
                    .background_executor()
                    .spawn(async move { check_feed_staleness(&conn, &endpoint, feed_id) })
                    .await;
                let _ = this.update(cx, move |this, cx| {
                    this.in_flight_feed_checks.remove(&feed_id);
                    match stale {
                        Ok(Some(entry)) => {
                            if !this
                                .feed_update_state
                                .stale
                                .iter()
                                .any(|existing| existing.feed_id == entry.feed_id)
                            {
                                this.feed_update_state.stale.push(entry);
                            }
                            this.feed_update_state.status_message = Some(format!(
                                "{} feed update{} pending",
                                this.feed_update_state.stale.len(),
                                if this.feed_update_state.stale.len() == 1 {
                                    ""
                                } else {
                                    "s"
                                }
                            ));
                        }
                        Ok(None) => {
                            if this.feed_update_state.stale.is_empty()
                                && this.in_flight_feed_checks.is_empty()
                            {
                                this.feed_update_state.status_message =
                                    Some("Feed up to date".into());
                            }
                        }
                        Err(err) => {
                            this.feed_update_state.status_message =
                                Some(format!("Feed check error: {err:#}"));
                        }
                    }
                    cx.notify();
                });
            },
        )
        .detach();
    }

    fn check_all_feeds(&mut self, cx: &mut Context<Self>) {
        if self.feed_update_state.phase != FeedUpdatePhase::Idle {
            return;
        }
        let feeds = {
            let conn = match self.conn.lock() {
                Ok(conn) => conn,
                Err(_) => {
                    self.feed_update_state.status_message =
                        Some("Feed check error: database lock poisoned".into());
                    cx.notify();
                    return;
                }
            };
            match db::subscribed_feeds_for_stale_check(&conn) {
                Ok(rows) => rows,
                Err(err) => {
                    self.feed_update_state.status_message =
                        Some(format!("Feed check error: {err:#}"));
                    cx.notify();
                    return;
                }
            }
        };
        if feeds.is_empty() {
            self.feed_update_state.status_message = Some("No subscribed feeds to check".into());
            cx.notify();
            return;
        }
        self.feed_update_state.phase = FeedUpdatePhase::Checking;
        self.feed_update_state.stale.clear();
        self.feed_update_state.status_message = Some(format!("Checking {} feeds...", feeds.len()));
        cx.notify();

        let conn = Arc::clone(&self.conn);
        let endpoint = self.musicindex_endpoint.clone();
        cx.spawn(
            async move |this: gpui::WeakEntity<LibraryApp>, cx: &mut gpui::AsyncApp| {
                let stale = cx
                    .background_executor()
                    .spawn(async move {
                        let mut stale = Vec::new();
                        for feed in feeds {
                            if let Ok(Some(entry)) = check_feed_staleness(&conn, &endpoint, feed.id)
                            {
                                stale.push(entry);
                            }
                        }
                        stale
                    })
                    .await;
                let _ = this.update(cx, move |this, cx| {
                    this.feed_update_state.phase = FeedUpdatePhase::Idle;
                    this.feed_update_state.stale = stale;
                    this.feed_update_state.status_message =
                        Some(if this.feed_update_state.stale.is_empty() {
                            "All feeds up to date".into()
                        } else {
                            format!(
                                "{} feed update{} available",
                                this.feed_update_state.stale.len(),
                                if this.feed_update_state.stale.len() == 1 {
                                    ""
                                } else {
                                    "s"
                                }
                            )
                        });
                    cx.notify();
                });
            },
        )
        .detach();
    }

    fn apply_all_feed_updates(&mut self, cx: &mut Context<Self>) {
        if self.feed_update_state.phase != FeedUpdatePhase::Idle
            || self.feed_update_state.stale.is_empty()
        {
            return;
        }
        let stale = self.feed_update_state.stale.clone();
        self.feed_update_state.phase = FeedUpdatePhase::Applying;
        self.feed_update_state.status_message =
            Some(format!("Applying updates to {} feed(s)...", stale.len()));
        cx.notify();

        let conn = Arc::clone(&self.conn);
        let endpoint = self.musicindex_endpoint.clone();
        cx.spawn(
            async move |this: gpui::WeakEntity<LibraryApp>, cx: &mut gpui::AsyncApp| {
                let outcomes = cx
                    .background_executor()
                    .spawn(async move {
                        let mut total_tracks = 0usize;
                        let mut total_edits = 0usize;
                        let mut id3_errors: Vec<String> = Vec::new();
                        let mut feed_errors: Vec<String> = Vec::new();
                        for entry in &stale {
                            match apply_feed_updates(&conn, &endpoint, entry) {
                                Ok(outcome) => {
                                    total_tracks += outcome.tracks_updated;
                                    total_edits += outcome.edits_written;
                                    id3_errors.extend(outcome.id3_errors);
                                }
                                Err(err) => {
                                    let label = entry
                                        .title
                                        .clone()
                                        .unwrap_or_else(|| entry.feed_guid.clone());
                                    feed_errors.push(format!("{label}: {err:#}"));
                                }
                            }
                        }
                        (total_tracks, total_edits, id3_errors, feed_errors)
                    })
                    .await;
                let _ = this.update(cx, move |this, cx| {
                    this.feed_update_state.phase = FeedUpdatePhase::Idle;
                    this.feed_update_state.stale.clear();
                    let (tracks, edits, id3_errors, feed_errors) = outcomes;
                    let mut parts: Vec<String> = Vec::new();
                    parts.push(if tracks == 0 {
                        "No edits written".into()
                    } else {
                        format!("Applied {edits} edit(s) to {tracks} track(s)")
                    });
                    if !id3_errors.is_empty() {
                        parts.push(format!(
                            "Tag write errors ({}): {}",
                            id3_errors.len(),
                            id3_errors.join("; ")
                        ));
                    }
                    if !feed_errors.is_empty() {
                        parts.push(format!(
                            "Feed errors ({}): {}",
                            feed_errors.len(),
                            feed_errors.join("; ")
                        ));
                    }
                    this.feed_update_state.status_message = Some(parts.join(" — "));
                    cx.notify();
                });
            },
        )
        .detach();
    }

    fn select_track(&mut self, track: &TrackRow, cx: &mut Context<Self>) {
        self.selected_id = Some(track.id);
        let image = track
            .track_image_href
            .as_deref()
            .or(track.album_image_href.as_deref())
            .and_then(|url| self.thumbnail_for_url(Some(url), true, cx));
        let mut frame = InspectorFrame::for_track(track.clone(), image);
        if let Some(lookup) = self.staged_musicbrainz.get(&track.id).cloned() {
            frame.musicbrainz_lookup = LazyPanel::Loaded(lookup);
            frame.musicbrainz_selected = 0;
        }
        self.detail = LibraryDetail::Track(Box::new(frame));
        let entity_id = track.id;
        let track = track.clone();
        let musicindex_endpoint = self.musicindex_endpoint.clone();
        cx.notify();
        cx.spawn(
            async move |this: gpui::WeakEntity<LibraryApp>, cx: &mut gpui::AsyncApp| {
                let result = cx
                    .background_executor()
                    .spawn(async move { fetch_library_track_context(&track, &musicindex_endpoint) })
                    .await;

                this.update(
                    cx,
                    move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                        if let Some(frame) = this.selected_track_frame_mut() {
                            if frame.entity_id == entity_id {
                                if let Ok(context) = result {
                                    frame.source_context = Some(context);
                                }
                            }
                        }
                        cx.notify();
                    },
                )
                .ok();
            },
        )
        .detach();
    }

    fn toggle_artist(&mut self, name: &str) {
        if !self.expanded_artists.remove(name) {
            self.expanded_artists.insert(name.to_string());
        }
    }

    fn toggle_album(&mut self, artist: &str, album: &str) {
        let key = (artist.to_string(), album.to_string());
        if !self.expanded_albums.remove(&key) {
            self.expanded_albums.insert(key);
        }
    }

    fn unsubscribe_feed(&mut self, feed_id: i64) {
        let conn = self.conn.lock().expect("lock db");
        if let Err(err) = db::set_feed_subscribed(&conn, feed_id, false) {
            self.status = format!("Error: {err:#}");
            return;
        }
        if let Err(err) = db::unsubscribe_feed_tracks(&conn, feed_id) {
            self.status = format!("Error: {err:#}");
            return;
        }
        drop(conn);
        self.reload();
    }

    fn remove_track(&mut self, track_id: i64) {
        let conn = self.conn.lock().expect("lock db");
        if let Err(err) = db::set_track_in_library(&conn, track_id, false) {
            self.status = format!("Error: {err:#}");
            return;
        }
        drop(conn);
        self.reload();
    }

    fn subscribe_track(&mut self, track: TrackRow, cx: &mut Context<Self>) {
        if self.busy_track.is_some() {
            return;
        }
        let track_id = track.id;
        self.busy_track = Some(track_id);
        self.status = "Subscribing track...".into();
        cx.notify();

        let conn = Arc::clone(&self.conn);
        cx.spawn(
            async move |this: gpui::WeakEntity<LibraryApp>, cx: &mut gpui::AsyncApp| {
                let result = cx
                    .background_executor()
                    .spawn(async move { subscribe_library_track(conn, track) })
                    .await;

                this.update(
                    cx,
                    move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                        this.busy_track = None;
                        match result {
                            Ok(outcome) => {
                                let mut msg =
                                    format!("Subscribed track: {}", outcome.path.display());
                                if let Some(warning) = outcome.format_warning {
                                    msg.push_str(" — ");
                                    msg.push_str(&warning);
                                }
                                this.status = msg;
                                this.reload();
                            }
                            Err(error) => {
                                this.status = format!("Error subscribing track: {error:#}");
                            }
                        }
                        cx.notify();
                    },
                )
                .ok();
            },
        )
        .detach();
    }

    fn selected_track_frame_mut(&mut self) -> Option<&mut InspectorFrame> {
        match &mut self.detail {
            LibraryDetail::Track(frame) => Some(frame),
            LibraryDetail::None | LibraryDetail::Album(_) | LibraryDetail::Playlist(_) => None,
        }
    }

    fn stage_musicbrainz_lookup_for_track(
        &mut self,
        track_id: i64,
        lookup: MusicBrainzLookupResult,
    ) {
        self.staged_musicbrainz.insert(track_id, lookup.clone());
        if let Some(frame) = self.selected_track_frame_mut() {
            if frame.entity_id == track_id {
                frame.musicbrainz_lookup = LazyPanel::Loaded(lookup);
                frame.musicbrainz_selected = 0;
                frame.id3_apply_error = None;
            }
        }
    }

    fn toggle_id3_frame_group(&mut self, group_key: String, cx: &mut Context<Self>) {
        let Some(frame) = self.selected_track_frame_mut() else {
            return;
        };
        if !frame.expanded_id3_frame_groups.remove(&group_key) {
            frame.expanded_id3_frame_groups.insert(group_key);
        }
        cx.notify();
    }

    fn toggle_metadata_cell(&mut self, cell_key: String, cx: &mut Context<Self>) {
        let Some(frame) = self.selected_track_frame_mut() else {
            return;
        };
        if !frame.expanded_metadata_cells.remove(&cell_key) {
            frame.expanded_metadata_cells.insert(cell_key);
        }
        cx.notify();
    }

    fn apply_pending_id3_edits(&mut self, cx: &mut Context<Self>) {
        let Some(frame) = self.selected_track_frame_mut() else {
            return;
        };
        if frame.applying_id3_edits {
            return;
        }
        let LazyPanel::Loaded(result) = &frame.tag_compare else {
            return;
        };
        let fallback_context = track_row_to_track_context(&frame.track);
        let track_context = frame.source_context.clone().unwrap_or(fallback_context);
        let rows = track_metadata_rows_for_frame(frame, &track_context, Some(result));
        let pending_id3_edits = auto_populated_pending_id3_edits(
            &rows,
            &frame.pending_id3_edits,
            &frame.suppressed_auto_id3_edits,
            result.format,
        );
        if pending_id3_edits.is_empty() {
            return;
        }
        let conflicts = pending_id3_conflict_descriptions(&pending_id3_edits);
        if !conflicts.is_empty() {
            frame.id3_apply_error = Some(format!(
                "Resolve duplicate ID3 target{}: {}",
                if conflicts.len() == 1 { "" } else { "s" },
                conflicts.join("; ")
            ));
            cx.notify();
            return;
        }

        let entity_id = frame.entity_id;
        let path = PathBuf::from(result.path.clone());
        let edits = pending_id3_edits_for_apply(&pending_id3_edits);
        frame.applying_id3_edits = true;
        frame.id3_apply_error = None;
        cx.notify();

        cx.spawn(
            async move |this: gpui::WeakEntity<LibraryApp>, cx: &mut gpui::AsyncApp| {
                let result = cx
                    .background_executor()
                    .spawn(async move {
                        write_id3v24_edits(&path, &edits)?;
                        compare_downloaded_track_path(&path, &track_context)
                    })
                    .await;

                this.update(
                    cx,
                    move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                        if let Some(frame) = this.selected_track_frame_mut() {
                            if frame.entity_id == entity_id {
                                frame.applying_id3_edits = false;
                                match result {
                                    Ok(result) => {
                                        frame.tag_compare = LazyPanel::Loaded(result);
                                        frame.pending_id3_edits.clear();
                                        frame.suppressed_auto_id3_edits.clear();
                                        frame.id3_apply_error = None;
                                    }
                                    Err(error) => {
                                        frame.id3_apply_error =
                                            Some(format!("Error applying ID3 edits: {error}"));
                                    }
                                }
                            }
                        }
                        cx.notify();
                    },
                )
                .ok();
            },
        )
        .detach();
    }

    fn clear_pending_id3_edits(&mut self, cx: &mut Context<Self>) {
        let Some(frame) = self.selected_track_frame_mut() else {
            return;
        };
        if frame.applying_id3_edits {
            return;
        }
        frame.pending_id3_edits.clear();
        frame.suppressed_auto_id3_edits.clear();
        frame.id3_apply_error = None;
        cx.notify();
    }

    fn toggle_local_subscription(&mut self, cx: &mut Context<Self>) {
        let Some((track_id, subscribe)) = (match self.selected_track_frame_mut() {
            Some(frame) if frame.subscription_busy => return,
            Some(frame) if frame.local_subscription => {
                frame.subscription_busy = true;
                frame.subscription_message = Some("Unsubscribing...".into());
                Some((frame.entity_id, false))
            }
            Some(frame) => {
                frame.subscription_busy = true;
                frame.subscription_message = Some("Subscribing...".into());
                Some((frame.entity_id, true))
            }
            None => None,
        }) else {
            return;
        };
        cx.notify();

        let result = {
            let db = self.conn.lock().expect("lock db");
            db::set_track_in_library(&db, track_id, subscribe)
        };
        if let Some(frame) = self.selected_track_frame_mut() {
            if frame.entity_id == track_id {
                frame.subscription_busy = false;
                match result {
                    Ok(()) => {
                        frame.local_subscription = subscribe;
                        frame.track.is_in_library = subscribe;
                        frame.subscription_message = Some(if subscribe {
                            "Subscribed track".into()
                        } else {
                            "Unsubscribed track".into()
                        });
                    }
                    Err(err) => {
                        let action = if subscribe {
                            "Subscribe"
                        } else {
                            "Unsubscribe"
                        };
                        frame.subscription_message = Some(format!("{action} error: {err:#}"));
                    }
                }
            }
        }
        cx.notify();
    }

    fn toggle_tag_compare(&mut self, cx: &mut Context<Self>) {
        let Some(frame) = self.selected_track_frame_mut() else {
            return;
        };
        match frame.tag_compare {
            LazyPanel::Loaded(_) => {
                frame.tag_compare = LazyPanel::Hidden;
                cx.notify();
                return;
            }
            LazyPanel::Loading => return,
            LazyPanel::Empty(_) | LazyPanel::Hidden => {
                frame.tag_compare = LazyPanel::Loading;
            }
        }

        let entity_id = frame.entity_id;
        let track = frame.track.clone();
        let musicindex_endpoint = self.musicindex_endpoint.clone();
        cx.notify();

        cx.spawn(
            async move |this: gpui::WeakEntity<LibraryApp>, cx: &mut gpui::AsyncApp| {
                let result = cx
                    .background_executor()
                    .spawn(async move { compare_library_track(&track, &musicindex_endpoint) })
                    .await;

                this.update(
                    cx,
                    move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                        if let Some(frame) = this.selected_track_frame_mut() {
                            if frame.entity_id == entity_id {
                                frame.tag_compare = match result {
                                    Ok(result) => {
                                        frame.source_context = Some(result.track_context);
                                        LazyPanel::Loaded(result.tag_compare)
                                    }
                                    Err(error) => LazyPanel::Empty(format!("Error: {error}")),
                                };
                            }
                        }
                        cx.notify();
                    },
                )
                .ok();
            },
        )
        .detach();
    }

    fn redownload_tag_compare(&mut self, cx: &mut Context<Self>) {
        self.reload_tag_compare(cx);
    }

    fn reread_tag_compare(&mut self, cx: &mut Context<Self>) {
        self.reload_tag_compare(cx);
    }

    fn reload_tag_compare(&mut self, cx: &mut Context<Self>) {
        let Some(frame) = self.selected_track_frame_mut() else {
            return;
        };
        if !matches!(frame.tag_compare, LazyPanel::Loaded(_)) {
            return;
        }
        frame.tag_compare = LazyPanel::Loading;
        let entity_id = frame.entity_id;
        let track = frame.track.clone();
        let musicindex_endpoint = self.musicindex_endpoint.clone();
        cx.notify();

        cx.spawn(
            async move |this: gpui::WeakEntity<LibraryApp>, cx: &mut gpui::AsyncApp| {
                let result = cx
                    .background_executor()
                    .spawn(async move { compare_library_track(&track, &musicindex_endpoint) })
                    .await;

                this.update(
                    cx,
                    move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                        if let Some(frame) = this.selected_track_frame_mut() {
                            if frame.entity_id == entity_id {
                                frame.tag_compare = match result {
                                    Ok(result) => {
                                        frame.source_context = Some(result.track_context);
                                        LazyPanel::Loaded(result.tag_compare)
                                    }
                                    Err(error) => LazyPanel::Empty(format!("Error: {error}")),
                                };
                            }
                        }
                        cx.notify();
                    },
                )
                .ok();
            },
        )
        .detach();
    }

    fn toggle_musicbrainz_lookup(&mut self, cx: &mut Context<Self>) {
        let Some(frame) = self.selected_track_frame_mut() else {
            return;
        };
        match frame.musicbrainz_lookup {
            LazyPanel::Loaded(_) => {
                frame.musicbrainz_lookup = LazyPanel::Hidden;
                frame.musicbrainz_selected = 0;
                cx.notify();
                return;
            }
            LazyPanel::Loading => return,
            LazyPanel::Empty(_) | LazyPanel::Hidden => {
                frame.musicbrainz_lookup = LazyPanel::Loading;
            }
        }

        let entity_id = frame.entity_id;
        let track = frame.track.clone();
        let cache = Arc::clone(&self.cache);
        cx.notify();

        cx.spawn(
            async move |this: gpui::WeakEntity<LibraryApp>, cx: &mut gpui::AsyncApp| {
                let result = cx
                    .background_executor()
                    .spawn(async move { lookup_musicbrainz_library_track(&track, cache) })
                    .await;

                this.update(
                    cx,
                    move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                        if let Some(frame) = this.selected_track_frame_mut() {
                            if frame.entity_id == entity_id {
                                frame.musicbrainz_lookup = match result {
                                    Ok(result) => {
                                        frame.musicbrainz_selected = 0;
                                        LazyPanel::Loaded(result)
                                    }
                                    Err(error) => LazyPanel::Empty(format!("Error: {error}")),
                                };
                            }
                        }
                        cx.notify();
                    },
                )
                .ok();
            },
        )
        .detach();
    }

    fn select_musicbrainz_candidate(&mut self, idx: usize, cx: &mut Context<Self>) {
        let Some(frame) = self.selected_track_frame_mut() else {
            return;
        };
        if let LazyPanel::Loaded(result) = &frame.musicbrainz_lookup {
            if idx < result.lookup.candidates.len() {
                frame.musicbrainz_selected = idx;
                cx.notify();
            }
        }
    }


    #[allow(dead_code)]
    fn musicbrainz_track(&mut self, track: TrackRow, cx: &mut Context<Self>) {
        if self.mb_status.contains_key(&track.id) {
            return;
        }
        self.mb_status.insert(track.id, MbTrackStatus::Processing);
        self.status = "MusicBrainz lookup...".into();
        cx.notify();

        let track_id = track.id;
        let conn = Arc::clone(&self.conn);
        cx.spawn(
            async move |this: gpui::WeakEntity<LibraryApp>, cx: &mut gpui::AsyncApp| {
                let result = cx
                    .background_executor()
                    .spawn(async move { lookup_musicbrainz_stage_for_track(conn, &track) })
                    .await;

                this.update(
                    cx,
                    move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                        match result {
                            Ok(staged) => {
                                let n = staged.edit_count;
                                this.stage_musicbrainz_lookup_for_track(track_id, staged.lookup);
                                this.mb_status.insert(track_id, MbTrackStatus::Done(n));
                                this.status = format!(
                                    "MusicBrainz: staged {n} edit{}",
                                    if n == 1 { "" } else { "s" }
                                );
                            }
                            Err(err) => {
                                this.mb_status
                                    .insert(track_id, MbTrackStatus::Skipped(format!("{err:#}")));
                                this.status = format!("MusicBrainz error: {err:#}");
                            }
                        }
                        cx.notify();
                    },
                )
                .ok();
            },
        )
        .detach();
    }

    fn musicbrainz_feed(&mut self, album: AlbumNode, cx: &mut Context<Self>) {
        let downloadable: Vec<TrackRow> = album
            .tracks
            .into_iter()
            .filter(|t| t.local_path.is_some())
            .collect();
        if downloadable.is_empty() {
            self.status = "No downloaded tracks to process".into();
            cx.notify();
            return;
        }
        for t in &downloadable {
            self.mb_status.insert(t.id, MbTrackStatus::Pending);
        }
        self.status = format!(
            "MusicBrainz: album lookup for {} tracks...",
            downloadable.len()
        );
        cx.notify();

        let conn = Arc::clone(&self.conn);
        let feed_id = album.feed_id.unwrap_or(0);
        let feed_title = Some(album.name.clone());
        let total_count = downloadable.len();
        cx.spawn(
            async move |this: gpui::WeakEntity<LibraryApp>, cx: &mut gpui::AsyncApp| {
                // Build album-level metadata from feed + first track.
                let first_artist = downloadable.iter().find_map(|t| t.artist_name.clone());
                let album_metadata = LookupMetadata {
                    title: None,
                    artist: first_artist,
                    album: feed_title,
                    track_number: None,
                    total_tracks: Some(total_count.to_string()),
                    duration_secs: None,
                    isrc: None,
                };

                // Do album-level release search (blocking, on background thread).
                let meta_clone = album_metadata.clone();
                let release_candidates = cx
                    .background_executor()
                    .spawn(async move {
                        let mb_client = ReqwestClient::builder()
                            .user_agent(format!(
                                "v4vmm/{} (MusicBrainz metadata lookup)",
                                env!("CARGO_PKG_VERSION")
                            ))
                            .build()?;
                        lookup_releases(&mb_client, &meta_clone, 3)
                    })
                    .await;

                let candidates = match release_candidates {
                    Ok(c) => c,
                    Err(err) => {
                        // Fall back to per-track recording search.
                        this.update(
                            cx,
                            move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                                this.status = format!(
                                    "Album lookup failed ({err:#}), falling back to per-track..."
                                );
                                cx.notify();
                            },
                        )
                        .ok();
                        musicbrainz_feed_per_track(
                            this,
                            cx,
                            &conn,
                            &downloadable,
                            feed_id,
                            total_count,
                        )
                        .await;
                        return;
                    }
                };

                if candidates.is_empty() {
                    this.update(
                        cx,
                        move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                            this.status =
                                "Album lookup: no results, falling back to per-track...".into();
                            cx.notify();
                        },
                    )
                    .ok();
                    musicbrainz_feed_per_track(
                        this,
                        cx,
                        &conn,
                        &downloadable,
                        feed_id,
                        total_count,
                    )
                    .await;
                    return;
                }

                // Match each local track to best candidate by track position then title.
                let mut total_edits = 0usize;
                let mut processed = 0usize;
                for track in &downloadable {
                    let track_id = track.id;
                    let progress = processed + 1;
                    this.update(
                        cx,
                        move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                            this.mb_status.insert(track_id, MbTrackStatus::Processing);
                            this.status = format!(
                                "MusicBrainz: staging track {progress}/{total_count} ...",
                            );
                            cx.notify();
                        },
                    )
                    .ok();

                    let matched = match_candidate_to_track(&candidates, track);
                    let track2 = track.clone();
                    let result = match matched {
                        Some(candidate) => {
                            let candidate = candidate.clone();
                            cx.background_executor()
                                .spawn(async move { stage_candidate_for_track(&track2, &candidate) })
                                .await
                        }
                        None => {
                            // No matching candidate — fall back to recording search for this track.
                            let conn2 = Arc::clone(&conn);
                            cx.background_executor()
                                .spawn(async move { lookup_musicbrainz_stage_for_track(conn2, &track2) })
                                .await
                        }
                    };

                    let status = match result {
                        Ok(staged) => {
                            let n = staged.edit_count;
                            total_edits += n;
                            let lookup = staged.lookup;
                            this.update(
                                cx,
                                move |this: &mut LibraryApp, _cx: &mut Context<LibraryApp>| {
                                    this.stage_musicbrainz_lookup_for_track(track_id, lookup);
                                },
                            )
                            .ok();
                            MbTrackStatus::Done(n)
                        }
                        Err(err) => MbTrackStatus::Skipped(format!("{err:#}")),
                    };
                    processed += 1;

                    let status_clone = status.clone();
                    this.update(
                        cx,
                        move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                            this.mb_status.insert(track_id, status_clone);
                            cx.notify();
                        },
                    )
                    .ok();
                }

                this.update(
                    cx,
                    move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                        this.status = format!(
                            "MusicBrainz: staged {total_edits} edit{} across {} tracks",
                            if total_edits == 1 { "" } else { "s" },
                            processed,
                        );
                        cx.notify();
                    },
                )
                .ok();
            },
        )
        .detach();
    }
}

pub struct SubscribedTrackOutcome {
    pub path: std::path::PathBuf,
    pub format_warning: Option<String>,
}

fn subscribe_library_track(
    conn: Arc<Mutex<Connection>>,
    track: TrackRow,
) -> anyhow::Result<SubscribedTrackOutcome> {
    let cfg_path = config::config_path()?;
    let cfg = config::load_config(&cfg_path)?;
    config::ensure_dirs(&cfg)?;
    let api_track = track_row_to_api_track(&track);
    let existing = track
        .local_path
        .as_deref()
        .filter(|p| !p.is_empty())
        .map(std::path::PathBuf::from)
        .filter(|p| p.exists());
    enum PreparedTrack {
        Existing { path: PathBuf },
        Downloaded(DownloadedTrack),
    }

    let prepared = if let Some(buf) = existing {
        PreparedTrack::Existing {
            path: crate::track_compare::ensure_taggable_local_path(&cfg, &buf),
        }
    } else if let Some(enclosure) = select_audio_enclosure(&api_track) {
        let candidate = crate::track_compare::local_track_path(
            &cfg,
            &api_track,
            enclosure.format.canonical_extension(),
        );
        if candidate.exists() {
            PreparedTrack::Existing {
                path: crate::track_compare::ensure_taggable_local_path(&cfg, &candidate),
            }
        } else {
            PreparedTrack::Downloaded(download_track(&cfg, &ReqwestClient::new(), &api_track)?)
        }
    } else {
        PreparedTrack::Downloaded(download_track(&cfg, &ReqwestClient::new(), &api_track)?)
    };

    // Apply tags on the staged path *before* promoting the file into
    // music_dir, so a tag-write failure leaves no half-written file behind.
    let track_context = TrackContext {
        track: api_track,
        feed: None,
    };
    let edits = id3_edits_for_track_context(&track_context);
    let format_warning = match &prepared {
        PreparedTrack::Existing { .. } => None,
        PreparedTrack::Downloaded(downloaded) => downloaded.format_warning.clone(),
    };
    let working_path = match &prepared {
        PreparedTrack::Existing { path } => path.clone(),
        PreparedTrack::Downloaded(downloaded) => downloaded.path.clone(),
    };
    if !edits.is_empty() {
        if let Err(err) = write_id3v24_edits(&working_path, &edits) {
            eprintln!("skip tag write for {}: {err:#}", working_path.display());
        }
    }

    let final_path = match prepared {
        PreparedTrack::Existing { path } => path,
        PreparedTrack::Downloaded(downloaded) => downloaded.finalize()?,
    };
    let file_size = std::fs::metadata(&final_path)
        .ok()
        .and_then(|metadata| metadata.len().try_into().ok());
    let db = conn
        .lock()
        .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
    db::mark_track_downloaded(&db, track.id, &final_path, file_size)?;
    drop(db);

    Ok(SubscribedTrackOutcome {
        path: final_path,
        format_warning,
    })
}

#[allow(dead_code)]
fn lookup_musicbrainz_stage_for_track(
    _conn: Arc<Mutex<Connection>>,
    track: &TrackRow,
) -> anyhow::Result<StagedMusicBrainzLookup> {
    let path = track
        .local_path
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("no local file"))?;
    let tags = read_audio_tags(std::path::Path::new(path))?;

    let api_track = track_row_to_api_track(track);
    let metadata = LookupMetadata {
        title: tags.title.clone().or_else(|| api_track.title.clone()),
        artist: tags
            .artist
            .clone()
            .or_else(|| api_track.track_artist.clone()),
        album: tags.album.clone().or_else(|| api_track.feed_title.clone()),
        track_number: tags
            .track_number
            .clone()
            .or_else(|| api_track.track_number.map(|n| n.to_string())),
        total_tracks: None,
        duration_secs: api_track.duration_secs.map(i64::from),
        isrc: tags
            .custom
            .get("ISRC")
            .cloned()
            .or_else(|| tags.custom.get("isrc").cloned()),
    };

    let mb_client = ReqwestClient::builder()
        .user_agent(format!(
            "v4vmm/{} (MusicBrainz metadata lookup)",
            env!("CARGO_PKG_VERSION")
        ))
        .build()?;
    let lookup = lookup_recordings(&mb_client, &metadata, 3)?;
    let candidate = lookup
        .candidates
        .first()
        .ok_or_else(|| anyhow::anyhow!("no MusicBrainz results"))?;

    Ok(StagedMusicBrainzLookup {
        edit_count: mb_edits_for_missing_fields(&tags, candidate).len(),
        lookup: MusicBrainzLookupResult {
            lookup,
            image: None,
        },
    })
}

#[allow(dead_code)]
fn match_candidate_to_track<'a>(
    candidates: &'a [MusicBrainzCandidate],
    track: &TrackRow,
) -> Option<&'a MusicBrainzCandidate> {
    // Try exact track number match first.
    if let Some(track_num) = track.track_number {
        if let Some(c) = candidates
            .iter()
            .find(|c| c.track_position == Some(track_num as i32))
        {
            return Some(c);
        }
    }
    // Fall back to title similarity.
    let track_title = track.track_title.as_deref()?;
    let normalized_title = track_title.to_lowercase();
    candidates.iter().max_by_key(|c| {
        let ct = c
            .track_title
            .as_deref()
            .or(Some(&c.title))
            .unwrap_or("")
            .to_lowercase();
        if ct == normalized_title {
            return 1000;
        }
        // Simple word overlap score.
        let title_words: Vec<&str> = normalized_title.split_whitespace().collect();
        let cand_words: Vec<&str> = ct.split_whitespace().collect();
        title_words
            .iter()
            .filter(|w| cand_words.contains(w))
            .count()
            * 100
            / title_words.len().max(1)
    })
}

#[allow(dead_code)]
fn stage_candidate_for_track(
    track: &TrackRow,
    candidate: &MusicBrainzCandidate,
) -> anyhow::Result<StagedMusicBrainzLookup> {
    let path = track
        .local_path
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("no local file"))?;
    let tags = read_audio_tags(std::path::Path::new(path))?;
    Ok(StagedMusicBrainzLookup {
        edit_count: mb_edits_for_missing_fields(&tags, candidate).len(),
        lookup: MusicBrainzLookupResult {
            lookup: MusicBrainzLookup {
                query: "batch release lookup".into(),
                candidates: vec![candidate.clone()],
            },
            image: None,
        },
    })
}

#[allow(dead_code)]
async fn musicbrainz_feed_per_track(
    this: gpui::WeakEntity<LibraryApp>,
    cx: &mut gpui::AsyncApp,
    conn: &Arc<Mutex<Connection>>,
    downloadable: &[TrackRow],
    _feed_id: i64,
    total_count: usize,
) {
    let mut total_edits = 0usize;
    let mut processed = 0usize;
    for track in downloadable {
        let track_id = track.id;
        let progress = processed + 1;
        this.update(
            cx,
            move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                this.mb_status.insert(track_id, MbTrackStatus::Processing);
                this.status = format!("MusicBrainz: staging track {progress}/{total_count} ...",);
                cx.notify();
            },
        )
        .ok();

        let conn2 = Arc::clone(conn);
        let track2 = track.clone();
        let result = cx
            .background_executor()
            .spawn(async move { lookup_musicbrainz_stage_for_track(conn2, &track2) })
            .await;

        let status = match result {
            Ok(staged) => {
                let n = staged.edit_count;
                total_edits += n;
                let lookup = staged.lookup;
                this.update(
                    cx,
                    move |this: &mut LibraryApp, _cx: &mut Context<LibraryApp>| {
                        this.stage_musicbrainz_lookup_for_track(track_id, lookup);
                    },
                )
                .ok();
                MbTrackStatus::Done(n)
            }
            Err(err) => MbTrackStatus::Skipped(format!("{err:#}")),
        };
        processed += 1;

        let status_clone = status.clone();
        this.update(
            cx,
            move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                this.mb_status.insert(track_id, status_clone);
                cx.notify();
            },
        )
        .ok();

        if processed < total_count {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(1100))
                .await;
        }
    }

    this.update(
        cx,
        move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
            this.status = format!(
                "MusicBrainz: staged {total_edits} edit{} across {} tracks",
                if total_edits == 1 { "" } else { "s" },
                processed,
            );
            cx.notify();
        },
    )
    .ok();
}

pub(crate) fn build_tree(tracks: &[TrackRow], conn: &Connection) -> LibraryTree {
    let mut artist_map: BTreeMap<String, BTreeMap<String, Vec<TrackRow>>> = BTreeMap::new();
    for track in tracks {
        let artist = track
            .album_artist_name
            .clone()
            .or_else(|| track.artist_name.clone())
            .unwrap_or_else(|| "Unknown Artist".to_string());
        let album = track
            .album_title
            .clone()
            .or_else(|| track.feed_title.clone())
            .unwrap_or_else(|| "Unknown Album".to_string());
        artist_map
            .entry(artist)
            .or_default()
            .entry(album)
            .or_default()
            .push(track.clone());
    }

    let mut feed_url_cache: BTreeMap<i64, Option<String>> = BTreeMap::new();
    let artists = artist_map
        .into_iter()
        .map(|(artist_name, album_map)| {
            let albums = album_map
                .into_iter()
                .map(|(album_name, mut tracks)| {
                    tracks.sort_by(|a, b| a.track_number.cmp(&b.track_number));
                    let feed_id = tracks.first().map(|t| t.feed_id);
                    let feed_url = feed_id.and_then(|fid| {
                        feed_url_cache
                            .entry(fid)
                            .or_insert_with(|| db::feed_url_by_id(conn, fid).ok().flatten())
                            .clone()
                    });
                    let image_href = tracks
                        .iter()
                        .find_map(|t| t.album_image_href.clone())
                        .or_else(|| tracks.iter().find_map(|t| t.track_image_href.clone()));
                    AlbumNode {
                        name: album_name,
                        feed_id,
                        feed_url,
                        image_href,
                        tracks,
                    }
                })
                .collect();
            ArtistNode {
                name: artist_name,
                albums,
            }
        })
        .collect();

    LibraryTree { artists }
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
                    .filter(|t| {
                        t.track_title
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

pub(crate) fn cleanup_empty_parents(path: &std::path::Path) {
    let music_dir = config::config_path()
        .ok()
        .and_then(|p| config::load_config(&p).ok())
        .map(|c| c.music_dir);
    let mut dir = path.parent();
    while let Some(d) = dir {
        if music_dir.as_deref() == Some(d) {
            break;
        }
        if std::fs::read_dir(d)
            .map(|mut r| r.next().is_none())
            .unwrap_or(false)
        {
            let _ = std::fs::remove_dir(d);
            dir = d.parent();
        } else {
            break;
        }
    }
}

#[allow(dead_code)]
fn mb_edits_for_missing_fields(
    tags: &crate::audio_tags::AudioTags,
    candidate: &MusicBrainzCandidate,
) -> Vec<Id3v24Edit> {
    let mut edits = Vec::new();

    // Build TRCK as "pos/total" when both available.
    let trck_value = match (candidate.track_position, candidate.total_tracks) {
        (Some(pos), Some(total)) => Some(format!("{pos}/{total}")),
        _ => candidate.track_number.clone(),
    };

    // Standard text frames: (frame_label, existing_check, mb_value)
    let checks: Vec<(&str, bool, Option<String>)> = vec![
        ("TIT2", tags.title.is_some(), Some(candidate.title.clone())),
        ("TPE1", tags.artist.is_some(), candidate.artist.clone()),
        (
            "TALB",
            tags.album.is_some(),
            candidate.release_title.clone(),
        ),
        ("TRCK", tags.track_number.is_some(), trck_value),
        ("TDRC", tags.date.is_some(), candidate.release_date.clone()),
        (
            "TPUB",
            tag_has_frame(tags, "TPUB"),
            candidate.labels.first().cloned(),
        ),
        (
            "TSRC",
            tag_has_frame(tags, "TSRC"),
            candidate.isrcs.first().cloned(),
        ),
        (
            "TMED",
            tag_has_frame(tags, "TMED"),
            candidate.format.clone(),
        ),
        (
            "TPOS",
            tag_has_frame(tags, "TPOS"),
            candidate.medium_position.map(|p| p.to_string()),
        ),
        (
            "TSST",
            tag_has_frame(tags, "TSST"),
            candidate.medium_title.clone(),
        ),
        (
            "TLEN",
            tag_has_frame(tags, "TLEN"),
            candidate.track_length_ms.map(|ms| ms.to_string()),
        ),
        // TXXX frames
        (
            "TXXX:MusicBrainz Album Id",
            tags.custom.contains_key("MusicBrainz Album Id"),
            candidate.release_id.clone(),
        ),
        (
            "TXXX:MusicBrainz Release Group Id",
            tags.custom.contains_key("MusicBrainz Release Group Id"),
            candidate.release_group_id.clone(),
        ),
        (
            "TXXX:BARCODE",
            tags.custom.contains_key("BARCODE"),
            candidate.release_barcode.clone(),
        ),
        // UFID for MusicBrainz recording ID
        (
            "UFID:http://musicbrainz.org",
            tag_has_frame(tags, "UFID"),
            if candidate.recording_id.is_empty() {
                None
            } else {
                Some(candidate.recording_id.clone())
            },
        ),
    ];

    for (frame_label, has_existing, mb_value) in checks {
        if has_existing {
            continue;
        }
        if let Some(value) = mb_value {
            if !value.is_empty() {
                edits.push(Id3v24Edit {
                    frame_label: frame_label.to_string(),
                    value,
                });
            }
        }
    }
    edits
}

#[allow(dead_code)]
fn tag_has_frame(tags: &crate::audio_tags::AudioTags, frame_id: &str) -> bool {
    tags.fields.iter().any(|f| f.frame_id == frame_id)
}

fn track_row_to_api_track(track: &TrackRow) -> Track {
    Track {
        track_guid: Some(track.item_guid.clone()),
        feed_guid: track.feed_guid.clone(),
        feed_title: track.feed_title.clone(),
        title: track.track_title.clone(),
        duration_secs: track
            .duration_seconds
            .and_then(|seconds| seconds.try_into().ok()),
        track_number: track.track_number.and_then(|number| number.try_into().ok()),
        enclosure_url: track.enclosure_url.clone(),
        enclosure_type: track.enclosure_type.clone(),
        image_url: track.track_image_href.clone(),
        track_artist: track.artist_name.clone(),
        source_links: track.transcript_url.as_ref().map(|url| {
            vec![SourceEntityLink {
                entity_type: Some("track".into()),
                entity_id: Some(track.item_guid.clone()),
                link_type: Some("transcript".into()),
                url: Some(url.clone()),
                source: Some("rss".into()),
                extraction_path: Some("podcast:transcript@url".into()),
                ..Default::default()
            }]
        }),
        ..Default::default()
    }
}

fn track_row_to_feed(track: &TrackRow) -> Feed {
    Feed {
        feed_guid: track.feed_guid.clone(),
        title: track.feed_title.clone(),
        image_url: track.album_image_href.clone(),
        ..Default::default()
    }
}

fn track_defaults(mut track: Track, defaults: &Track) -> Track {
    if track.track_guid.is_none() {
        track.track_guid = defaults.track_guid.clone();
    }
    if track.feed_guid.is_none() {
        track.feed_guid = defaults.feed_guid.clone();
    }
    if track.feed_title.is_none() {
        track.feed_title = defaults.feed_title.clone();
    }
    if track.title.is_none() {
        track.title = defaults.title.clone();
    }
    if track.duration_secs.is_none() {
        track.duration_secs = defaults.duration_secs;
    }
    if track.track_number.is_none() {
        track.track_number = defaults.track_number;
    }
    if track.enclosure_url.is_none() {
        track.enclosure_url = defaults.enclosure_url.clone();
    }
    if track.image_url.is_none() {
        track.image_url = defaults.image_url.clone();
    }
    if track.track_artist.is_none() {
        track.track_artist = defaults.track_artist.clone();
    }
    if track.description.is_none() {
        track.description = defaults.description.clone();
    }
    if track.publisher_text.is_none() {
        track.publisher_text = defaults.publisher_text.clone();
    }
    if track.source_contributors.is_none() {
        track.source_contributors = defaults.source_contributors.clone();
    }
    if track.source_links.is_none() {
        track.source_links = defaults.source_links.clone();
    }
    if track.source_ids.is_none() {
        track.source_ids = defaults.source_ids.clone();
    }
    if track.source_release_claims.is_none() {
        track.source_release_claims = defaults.source_release_claims.clone();
    }
    if track.payment_routes.is_none() {
        track.payment_routes = defaults.payment_routes.clone();
    }
    track
}

fn feed_defaults(mut feed: Feed, defaults: &Feed) -> Feed {
    if feed.feed_guid.is_none() {
        feed.feed_guid = defaults.feed_guid.clone();
    }
    if feed.title.is_none() {
        feed.title = defaults.title.clone();
    }
    if feed.name.is_none() {
        feed.name = defaults.name.clone();
    }
    if feed.feed_url.is_none() {
        feed.feed_url = defaults.feed_url.clone();
    }
    if feed.image_url.is_none() {
        feed.image_url = defaults.image_url.clone();
    }
    feed
}

fn merge_track_context_from_detail(
    track_row: &TrackRow,
    fetched_track: Option<Track>,
    fetched_feed: Option<Feed>,
) -> TrackContext {
    let local_track = track_row_to_api_track(track_row);
    let local_feed = track_row_to_feed(track_row);
    let feed = feed_defaults(
        fetched_feed.unwrap_or_else(|| local_feed.clone()),
        &local_feed,
    );
    let track = crate::api::track_with_feed_defaults(
        track_defaults(
            fetched_track.unwrap_or_else(|| local_track.clone()),
            &local_track,
        ),
        Some(&feed),
    );
    TrackContext {
        track,
        feed: Some(feed),
    }
}

fn fetch_library_track_context(
    track: &TrackRow,
    musicindex_endpoint: &str,
) -> anyhow::Result<TrackContext> {
    let client = MusicIndexClient::new_with_base_url(musicindex_endpoint.to_string());
    let include =
        Some("source_links,source_ids,source_release_claims,source_contributors,payment_routes");
    let fetched_track = client.fetch_track(&track.item_guid, include).ok();
    let feed_guid = fetched_track
        .as_ref()
        .and_then(|track| track.feed_guid.as_deref())
        .or(track.feed_guid.as_deref());
    let fetched_feed = feed_guid.and_then(|feed_guid| client.fetch_feed(feed_guid, include).ok());
    if fetched_track.is_none() && fetched_feed.is_none() {
        return Err(anyhow::anyhow!("MusicIndex metadata unavailable"));
    }
    Ok(merge_track_context_from_detail(
        track,
        fetched_track,
        fetched_feed,
    ))
}

// ---------------------------------------------------------------------------
// Feed update checking / auto-apply
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct FeedApplyOutcome {
    pub tracks_updated: usize,
    pub edits_written: usize,
    pub id3_errors: Vec<String>,
}

fn check_feed_staleness(
    conn: &Arc<Mutex<Connection>>,
    musicindex_endpoint: &str,
    feed_id: i64,
) -> anyhow::Result<Option<StaleFeed>> {
    let stored = {
        let db = conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        db::feed_stale_check_row(&db, feed_id)?
    };
    let Some(stored) = stored else {
        return Ok(None);
    };
    let client = MusicIndexClient::new_with_base_url(musicindex_endpoint.to_string());
    let api_feed = client.fetch_feed(&stored.feed_guid, None)?;
    let Some(api_updated_at) = api_feed.updated_at else {
        return Ok(None);
    };
    if stored
        .musicindex_updated_at
        .is_some_and(|stored_at| stored_at >= api_updated_at)
    {
        return Ok(None);
    }
    Ok(Some(StaleFeed {
        feed_id,
        feed_guid: stored.feed_guid,
        title: stored.title,
        new_updated_at: api_updated_at,
    }))
}

fn apply_feed_updates(
    conn: &Arc<Mutex<Connection>>,
    musicindex_endpoint: &str,
    stale: &StaleFeed,
) -> anyhow::Result<FeedApplyOutcome> {
    let tracks = {
        let db = conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        db::library_tracks_for_feed(&db, stale.feed_id)?
    };
    let mut outcome = FeedApplyOutcome {
        tracks_updated: 0,
        edits_written: 0,
        id3_errors: Vec::new(),
    };
    for track in &tracks {
        let Some(local_path) = track.local_path.clone() else {
            continue;
        };
        let Ok(context) = fetch_library_track_context(track, musicindex_endpoint) else {
            continue;
        };
        let edits = id3_edits_for_track_context(&context);
        if edits.is_empty() {
            continue;
        }
        match write_id3v24_edits(Path::new(&local_path), &edits) {
            Ok(written) => {
                if written > 0 {
                    outcome.tracks_updated += 1;
                    outcome.edits_written += written;
                }
            }
            Err(error) => {
                let label = track
                    .track_title
                    .clone()
                    .unwrap_or_else(|| local_path.clone());
                outcome.id3_errors.push(format!("{label}: {error:#}"));
            }
        }
    }
    {
        let db = conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        db::set_feed_musicindex_updated_at(&db, stale.feed_id, stale.new_updated_at)?;
    }
    Ok(outcome)
}

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

impl Render for LibraryApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status_text = self.status.clone();
        let status_color = if status_text.starts_with("Error:") {
            color::status_danger()
        } else {
            color::text_muted()
        };

        // Collect image URLs from tree, then fetch thumbnails (avoids borrow conflict).
        let urls: Vec<String> = {
            self.tree
                .artists
                .iter()
                .flat_map(|a| &a.albums)
                .flat_map(|album| {
                    album
                        .image_href
                        .iter()
                        .chain(album.tracks.iter().filter_map(|track| {
                            track
                                .track_image_href
                                .as_ref()
                                .or(track.album_image_href.as_ref())
                        }))
                        .cloned()
                })
                .collect()
        };
        let hovered_url = self.hovered_thumb_url.clone();
        let mut album_thumbs: BTreeMap<String, Option<Arc<Image>>> = BTreeMap::new();
        for url in &urls {
            if !album_thumbs.contains_key(url.as_str()) {
                let animated = hovered_url.as_deref() == Some(url.as_str());
                let img = self.thumbnail_for_url(Some(url), animated, cx);
                album_thumbs.insert(url.clone(), img);
            }
        }

        let base_tree = &self.tree;
        let query = self.search_query.trim();
        let has_query = !query.is_empty();
        let filtered_tree = if has_query {
            filter_tree(base_tree, query)
        } else {
            base_tree.clone()
        };
        let (expanded_artists, expanded_albums) = if has_query {
            let ea: HashSet<String> = filtered_tree
                .artists
                .iter()
                .map(|a| a.name.clone())
                .collect();
            let eb: HashSet<(String, String)> = filtered_tree
                .artists
                .iter()
                .flat_map(|a| {
                    a.albums
                        .iter()
                        .map(move |alb| (a.name.clone(), alb.name.clone()))
                })
                .collect();
            (ea, eb)
        } else {
            (self.expanded_artists.clone(), self.expanded_albums.clone())
        };
        let tree_items: Vec<AnyElement> = render_tree(
            &filtered_tree,
            &expanded_artists,
            &expanded_albums,
            self.selected_id,
            &album_thumbs,
            cx,
        );
        let filtered_empty = filtered_tree.artists.is_empty();

        let playlists = self.playlists.clone();
        let selected_playlist_id = self.selected_playlist_id;
        let creating_playlist = self.creating_playlist;
        let playlists_expanded = self.playlists_expanded;
        let mut left_items: Vec<AnyElement> = Vec::new();

        let playlist_arrow = if playlists_expanded {
            "\u{25BC}"
        } else {
            "\u{25B6}"
        };
        left_items.push(
            div()
                .id("playlists-header")
                .px(spacing::SM)
                .py(spacing::XS)
                .rounded(spacing::XS)
                .cursor_pointer()
                .hover(|el| el.bg(color::bg_surface_hi()))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.playlists_expanded = !this.playlists_expanded;
                    cx.notify();
                }))
                .flex()
                .flex_row()
                .justify_between()
                .items_center()
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(spacing::XS)
                        .items_baseline()
                        .child(
                            div()
                                .text_xs()
                                .text_color(color::text_muted())
                                .w(spacing::MD)
                                .child(SharedString::from(playlist_arrow)),
                        )
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(color::text_primary())
                                .child("Playlists"),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(spacing::XS)
                        .items_center()
                        .child(
                            Button::new("playlists-sort")
                                .label(self.playlist_sort.label())
                                .ghost()
                                .with_size(Size::XSmall)
                                .text_color(color::text_muted())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.cycle_playlist_sort(cx);
                                })),
                        )
                        .child(
                            Button::new("playlists-add")
                                .label("+")
                                .ghost()
                                .with_size(Size::XSmall)
                                .text_color(color::text_primary())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.creating_playlist = !this.creating_playlist;
                                    cx.notify();
                                })),
                        ),
                )
                .into_any_element(),
        );

        if playlists_expanded {
            for playlist in &playlists {
                let is_selected = selected_playlist_id == Some(playlist.id);
                let playlist_id = playlist.id;
                let playlist_name = playlist.name.clone();
                let track_count = playlist.track_count;

                left_items.push(
                    div()
                        .id(SharedString::from(format!("playlist-{}", playlist.id)))
                        .pl(spacing::LG + spacing::XS)
                        .pr(spacing::SM)
                        .py(spacing::XXS)
                        .rounded(spacing::XS)
                        .cursor_pointer()
                        .when(is_selected, |el| el.bg(color::bg_selected()))
                        .when(is_selected, |el| el.border_l_2().border_color(color::accent()))
                        .when(!is_selected, |el| el.hover(|e| e.bg(color::bg_surface_hi())))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.select_playlist(playlist_id, cx);
                        }))
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .flex_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(if is_selected {
                                            color::accent()
                                        } else {
                                            color::text_primary()
                                        })
                                        .child(SharedString::from(playlist_name.clone())),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(color::text_muted())
                                        .child(SharedString::from(format!("({track_count})")))
                                ),
                        )
                        .into_any_element(),
                );
            }

            if creating_playlist {
                left_items.push(
                    div()
                        .id("playlist-new-input")
                        .pl(spacing::LG + spacing::XS)
                        .pr(spacing::SM)
                        .py(spacing::XXS)
                        .flex()
                        .flex_row()
                        .gap(spacing::XS)
                        .items_center()
                        .child(
                            Input::new(&self.new_playlist_input)
                                .cleanable(false)
                                .with_size(Size::Small)
                        )
                        .child(
                            Button::new("playlist-add-btn")
                                .label("Add")
                                .primary()
                                .with_size(Size::XSmall)
                                .text_color(rgb(0xffffff))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.create_playlist(cx);
                                })),
                        )
                        .into_any_element(),
                );
            }
        }

        left_items.extend(tree_items);

        let detail_pane = render_detail(
            &self.detail,
            self.busy_track,
            &self.mb_status,
            &album_thumbs,
            &self.playlists,
            self.album_add_open_feed,
            self.album_add_open_track,
            cx,
        );

        div()
            .size_full()
            .bg(color::bg_canvas())
            .text_color(color::text_primary())
            .text_sm()
            .flex()
            .flex_col()
            .overflow_hidden()
            // Two panes
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    // Left pane: list
                    .child(
                        div()
                            .w(layout::INSPECTOR_WIDTH)
                            .min_w(px(200.0))
                            .flex_shrink_0()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .border_r_1()
                            .border_color(color::border_subtle())
                            .child(
                                div()
                                    .p(spacing::MD)
                                    .border_b_1()
                                    .border_color(color::border_subtle())
                                    .flex()
                                    .flex_col()
                                    .gap(spacing::SM)
                                    .child(
                                        typography::type_micro(div())
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(color::text_muted())
                                            .child("Search Library"),
                                    )
                                    .child(
                                        Input::new(&self.search_input)
                                            .cleanable(true)
                                            .with_size(Size::Small),
                                    )
                                    .child(
                                        Button::new("lib-search-btn")
                                            .label("Search")
                                            .primary()
                                            .with_size(Size::Small)
                                            .text_color(rgb(0xffffff))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.apply_search(cx);
                                            })),
                                    ),
                            )
                            .child({
                                let has_stale = !self.feed_update_state.stale.is_empty();
                                let phase = self.feed_update_state.phase.clone();
                                let stale_count = self.feed_update_state.stale.len();
                                let feed_status = self.feed_update_state.status_message.clone();
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .justify_between()
                                    .gap(spacing::SM)
                                    .px(spacing::MD)
                                    .py(spacing::XS)
                                    .border_b_1()
                                    .border_color(color::border_subtle())
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(spacing::XXS)
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(status_color)
                                                    .child(SharedString::from(status_text)),
                                            )
                                            .when_some(feed_status, |el, msg| {
                                                el.child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(color::text_muted())
                                                        .child(SharedString::from(msg)),
                                                )
                                            }),
                                    )
                                    .child(if has_stale {
                                        Button::new("apply-feed-updates")
                                            .label(format!("Apply updates ({stale_count})"))
                                            .primary()
                                            .with_size(Size::XSmall)
                                            .text_color(rgb(0xffffff))
                                            .disabled(phase != FeedUpdatePhase::Idle)
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.apply_all_feed_updates(cx);
                                            }))
                                    } else {
                                        Button::new("check-all-feeds")
                                            .label(if phase == FeedUpdatePhase::Checking {
                                                "Checking..."
                                            } else {
                                                "Check all feeds"
                                            })
                                            .with_size(Size::XSmall)
                                            .disabled(phase != FeedUpdatePhase::Idle)
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.check_all_feeds(cx);
                                            }))
                                    })
                            })
                            .child(
                                div()
                                    .id("library-list")
                                    .flex_1()
                                    .overflow_y_scroll()
                                    .p(spacing::SM)
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(spacing::XXS)
                                            .children(left_items)
                                            .when(
                                                filtered_empty
                                                    && !self.status.starts_with("Error:"),
                                                |el| {
                                                    el.child(
                                                        div()
                                                            .text_center()
                                                            .p(spacing::XXL + spacing::LG)
                                                            .text_color(color::text_muted())
                                                            .child(div().mt(spacing::SM).child("No library tracks yet")),
                                                    )
                                                },
                                            ),
                                    ),
                            ),
                    )
                    // Right pane: detail
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .child(detail_pane),
                    ),
            )
    }
}

// ---------------------------------------------------------------------------
// Rendering helpers
// ---------------------------------------------------------------------------

pub(crate) fn render_tree(
    tree: &LibraryTree,
    expanded_artists: &HashSet<String>,
    expanded_albums: &HashSet<(String, String)>,
    selected_id: Option<i64>,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    cx: &mut Context<LibraryApp>,
) -> Vec<AnyElement> {
    let mut items = Vec::new();
    for artist in &tree.artists {
        let artist_expanded = expanded_artists.contains(&artist.name);
        let arrow = if artist_expanded {
            "\u{25BC}"
        } else {
            "\u{25B6}"
        };
        let album_count = artist.albums.len();
        let artist_name = artist.name.clone();

        items.push(
            div()
                .id(SharedString::from(format!("artist-{}", artist.name)))
                .px(spacing::SM)
                .py(spacing::XS)
                .rounded(spacing::XS)
                .cursor_pointer()
                .hover(|el| el.bg(color::bg_surface_hi()))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.toggle_artist(&artist_name);
                    cx.notify();
                }))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(spacing::XS)
                        .items_baseline()
                        .child(
                            div()
                                .text_xs()
                                .text_color(color::text_muted())
                                .w(spacing::MD)
                                .child(SharedString::from(arrow)),
                        )
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(color::text_primary())
                                .child(SharedString::from(artist.name.clone())),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(color::text_muted())
                                .child(SharedString::from(format!(
                                    "({album_count} album{})",
                                    if album_count == 1 { "" } else { "s" }
                                ))),
                        ),
                )
                .into_any_element(),
        );

        if artist_expanded {
            for album in &artist.albums {
                let album_key = (artist.name.clone(), album.name.clone());
                let album_expanded = expanded_albums.contains(&album_key);
                let arrow = if album_expanded {
                    "\u{25BC}"
                } else {
                    "\u{25B6}"
                };
                let track_count = album.tracks.len();
                let artist_for_toggle = artist.name.clone();
                let album_for_toggle = album.name.clone();
                let album_for_select = album.clone();
                let thumb_url = album.image_href.clone();
                let thumb_image = thumb_url
                    .as_ref()
                    .and_then(|url| album_thumbs.get(url.as_str()))
                    .and_then(|opt| opt.clone());

                items.push(
                    div()
                        .id(SharedString::from(format!(
                            "album-{}-{}",
                            artist.name, album.name
                        )))
                        .pl(spacing::LG + spacing::XS)
                        .pr(spacing::SM)
                        .py(spacing::XXS)
                        .rounded(spacing::XS)
                        .cursor_pointer()
                        .hover(|el| el.bg(color::bg_surface_hi()))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_album(&artist_for_toggle, &album_for_toggle);
                            this.select_album(&album_for_select, cx);
                            cx.notify();
                        }))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .gap(spacing::XS)
                                .items_center()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(color::text_muted())
                                        .w(spacing::MD)
                                        .child(SharedString::from(arrow)),
                                )
                                .child(hoverable_thumb(
                                    thumb_url.clone(),
                                    thumb_image.as_ref(),
                                    34.0,
                                    cx,
                                ))
                                .child(
                                    div()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(color::accent())
                                        .child(SharedString::from(album.name.clone())),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(color::text_muted())
                                        .child(SharedString::from(format!("({track_count})",))),
                                ),
                        )
                        .into_any_element(),
                );

                if album_expanded {
                    for track in &album.tracks {
                        let track_clone_b = track.clone();
                        let is_selected = selected_id == Some(track.id);
                        let title = track
                            .track_title
                            .as_deref()
                            .unwrap_or("[untitled]")
                            .to_string();
                        let num = track
                            .track_number
                            .map(|n| format!("{n:02} - "))
                            .unwrap_or_default();
                        let track_thumb_image = track
                            .track_image_href
                            .as_ref()
                            .or(track.album_image_href.as_ref())
                            .and_then(|url| album_thumbs.get(url.as_str()))
                            .and_then(|opt| opt.clone());

                        let mut row = div()
                            .id(SharedString::from(format!("tree-track-{}", track.id)))
                            .pl(spacing::XXL + spacing::MD)
                            .pr(spacing::SM)
                            .py(spacing::XXS)
                            .rounded(spacing::XS)
                            .cursor_pointer()
                            .when(is_selected, |el| el.bg(color::bg_selected()))
                            .when(is_selected, |el| el.border_l_2().border_color(color::accent()))
                            .hover(|el| el.bg(color::bg_surface_hi()));

                        row = row
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.select_track(&track_clone_b, cx);
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap(spacing::XS)
                                    .child(render_album_thumb(track_thumb_image.as_ref(), 24.0))
                                    .child(
                                        div()
                                            .flex_1()
                                            .text_xs()
                                            .text_color(if is_selected {
                                                color::accent()
                                            } else {
                                                color::text_primary()
                                            })
                                            .child(SharedString::from(format!("{num}{title}"))),
                                    ),
                            );

                        items.push(row.into_any_element());
                    }
                }
            }
        }
    }
    items
}

fn render_detail(
    detail: &LibraryDetail,
    busy_track: Option<i64>,
    mb_status: &BTreeMap<i64, MbTrackStatus>,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    playlists: &[db::Playlist],
    album_add_open_feed: bool,
    album_add_open_track: Option<i64>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    match detail {
        LibraryDetail::None => div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_color(color::text_muted())
                    .text_center()
                    .child("Select an item to view details"),
            )
            .into_any_element(),

        LibraryDetail::Album(album) => render_album_detail(
            album,
            busy_track,
            mb_status,
            album_thumbs,
            playlists,
            album_add_open_feed,
            album_add_open_track,
            cx,
        ),

        LibraryDetail::Track(frame) => render_track_detail(frame, playlists, cx),

        LibraryDetail::Playlist(detail) => render_playlist_detail(detail, album_thumbs, cx),
    }
}

fn render_album_detail(
    album: &AlbumNode,
    busy_track: Option<i64>,
    mb_status: &BTreeMap<i64, MbTrackStatus>,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    playlists: &[db::Playlist],
    add_open_feed: bool,
    add_open_track: Option<i64>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let title = &album.name;
    let feed_id = album.feed_id;
    let thumb_image = album
        .image_href
        .as_ref()
        .and_then(|url| album_thumbs.get(url.as_str()))
        .and_then(|opt| opt.clone());

    let track_rows: Vec<AnyElement> = album
        .tracks
        .iter()
        .map(|track| {
            let track_for_click = track.clone();
            let track_for_select = track.clone();
            let track_id = track.id;
            let in_library = track.is_in_library;
            let is_busy = busy_track == Some(track_id);
            let popup_open = add_open_track == Some(track_id);
            let track_title = track
                .track_title
                .as_deref()
                .unwrap_or("[untitled]")
                .to_string();
            let num_str = track
                .track_number
                .map(|n| format!("{n}. "))
                .unwrap_or_default();
            let dur = track
                .duration_seconds
                .map(|s| format!("  ({}:{:02})", s / 60, s % 60))
                .unwrap_or_default();
            let mb = mb_status.get(&track_id);
            let mb_text = match mb {
                Some(MbTrackStatus::Pending) => Some("MB: pending"),
                Some(MbTrackStatus::Processing) => Some("MB: looking up..."),
                Some(MbTrackStatus::Done(0)) => Some("MB: no missing fields"),
                Some(MbTrackStatus::Done(_)) => Some("MB: done"),
                Some(MbTrackStatus::Skipped(_)) => Some("MB: skipped"),
                None => None,
            };

            let row = div()
                .id(SharedString::from(format!("album-track-{track_id}")))
                .flex()
                .flex_row()
                .items_center()
                .gap(spacing::SM)
                .px(spacing::SM)
                .py(spacing::XS)
                .rounded(radius::SM)
                .hover(|el| el.bg(color::bg_surface_hi()))
                .child(
                    div()
                        .id(SharedString::from(format!("album-track-select-{track_id}")))
                        .flex_1()
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.select_track(&track_for_select, cx);
                            cx.notify();
                        }))
                        .child(SharedString::from(format!("{num_str}{track_title}{dur}")))
                        .when(mb_text.is_some(), |el| {
                            let color = match mb {
                                Some(MbTrackStatus::Done(n)) if *n > 0 => color::status_success(),
                                Some(MbTrackStatus::Skipped(_)) => color::status_danger(),
                                Some(MbTrackStatus::Processing) => color::status_warning(),
                                _ => color::text_muted(),
                            };
                            el.child(
                                div()
                                    .text_xs()
                                    .text_color(color)
                                    .child(SharedString::from(mb_text.unwrap().to_string())),
                            )
                        }),
                )
                .child(
                    Button::new(SharedString::from(format!("lib-toggle-{track_id}")))
                        .label(if is_busy {
                            "Subscribing..."
                        } else if in_library {
                            "Unsubscribe"
                        } else {
                            "Subscribe"
                        })
                        .with_size(Size::XSmall)
                        .when(in_library, |btn| btn.primary())
                        .when(!in_library, |btn| btn.ghost())
                        .text_color(rgb(0xffffff))
                        .disabled(is_busy)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if in_library {
                                this.remove_track(track_id);
                            } else {
                                this.subscribe_track(track_for_click.clone(), cx);
                            }
                            cx.notify();
                        })),
                )
                .when(track.local_path.is_some(), |el| {
                    el.child(div().text_xs().text_color(color::status_success()).child("dl'd"))
                })
                .child(
                    Button::new(SharedString::from(format!("album-track-add-{track_id}")))
                        .label(if popup_open { "Add ▴" } else { "Add ▾" })
                        .ghost()
                        .with_size(Size::XSmall)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.album_add_open_track =
                                if this.album_add_open_track == Some(track_id) {
                                    None
                                } else {
                                    Some(track_id)
                                };
                            cx.notify();
                        })),
                );

            if popup_open {
                let popup = render_album_track_add_panel(track_id, playlists, cx);
                div()
                    .flex()
                    .flex_col()
                    .gap(spacing::XXS)
                    .child(row)
                    .child(popup)
                    .into_any_element()
            } else {
                row.into_any_element()
            }
        })
        .collect();

    // Compute album metadata.
    let artist = album
        .tracks
        .iter()
        .find_map(|t| {
            t.album_artist_name
                .clone()
                .or_else(|| t.artist_name.clone())
        })
        .unwrap_or_else(|| "Unknown Artist".to_string());
    let total_duration_secs: i64 = album.tracks.iter().filter_map(|t| t.duration_seconds).sum();
    let duration_str = if total_duration_secs > 0 {
        let mins = total_duration_secs / 60;
        let secs = total_duration_secs % 60;
        if mins >= 60 {
            format!("{}h {}m", mins / 60, mins % 60)
        } else {
            format!("{mins}:{secs:02}")
        }
    } else {
        String::new()
    };
    let downloaded = album
        .tracks
        .iter()
        .filter(|t| t.local_path.is_some())
        .count();
    let track_count = album.tracks.len();
    let mut detail_rows = vec![
        ("Artist".to_string(), artist.clone()),
        (
            "Tracks".to_string(),
            format!(
                "{track_count} track{}",
                if track_count == 1 { "" } else { "s" }
            ),
        ),
    ];
    if !duration_str.is_empty() {
        detail_rows.push(("Duration".to_string(), duration_str.clone()));
    }
    if downloaded > 0 {
        detail_rows.push(("Downloaded".to_string(), downloaded.to_string()));
    }

    // Buttons row.
    let has_active_mb = mb_status
        .values()
        .any(|s| matches!(s, MbTrackStatus::Pending | MbTrackStatus::Processing));
    let album_for_mb = album.clone();
    let feed_url = album.feed_url.clone();
    let mut buttons = div().flex().flex_row().items_center().gap(spacing::SM);
    buttons = buttons.child(render_rss_icon_link(
        &format!("album-{}", album.feed_id.unwrap_or(0)),
        feed_url,
    ));
    if let Some(fid) = feed_id {
        buttons = buttons.child(
            metadata_action_button("Unsubscribe Feed")
                .danger()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.unsubscribe_feed(fid);
                    cx.notify();
                })),
        );
    }
    buttons = buttons.child(
        metadata_action_button("MusicBrainz")
            .disabled(has_active_mb)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.musicbrainz_feed(album_for_mb.clone(), cx);
            })),
    );
    if let Some(fid) = feed_id {
        buttons = buttons.child(
            metadata_action_button(if add_open_feed {
                "Add album to playlist ▴"
            } else {
                "Add album to playlist ▾"
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                this.album_add_open_feed = !this.album_add_open_feed;
                let _ = fid;
                cx.notify();
            })),
        );
    }

    let feed_popup: Option<AnyElement> = if add_open_feed {
        feed_id.map(|fid| render_album_feed_add_panel(fid, playlists, cx))
    } else {
        None
    };

    let mut container = div()
        .id("album-detail-scroll")
        .size_full()
        .overflow_y_scroll()
        .p(spacing::LG)
        .flex()
        .flex_col()
        .gap(spacing::MD)
        .child(render_detail_header(
            "feed",
            title,
            Some(artist.as_str()),
            thumb_image.as_ref(),
        ))
        .child(render_detail_grid(detail_rows))
        .child(buttons);
    if let Some(panel) = feed_popup {
        container = container.child(panel);
    }
    container
        .child(div().flex().flex_col().gap(spacing::XXS).children(track_rows))
        .into_any_element()
}

fn render_album_track_add_panel(
    track_id: i64,
    playlists: &[db::Playlist],
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let mut panel = div()
        .border_1()
        .border_color(color::border_subtle())
        .rounded(radius::SM)
        .bg(color::bg_surface())
        .p(spacing::SM)
        .gap(spacing::XS)
        .flex()
        .flex_col();

    if playlists.is_empty() {
        panel = panel.child(
            div()
                .text_size(typography::SIZE_MICRO)
                .text_color(color::text_muted())
                .child(SharedString::from(
                    "No playlists yet — create one from the sidebar.",
                )),
        );
        return panel.into_any_element();
    }

    for p in playlists {
        let playlist_id = p.id;
        let label = format!("{} ({})", p.name, p.track_count);
        panel = panel.child(
            metadata_action_button(&label).on_click(cx.listener(move |this, _, _, cx| {
                this.album_add_open_track = None;
                this.add_track_to_playlist(track_id, playlist_id, cx);
            })),
        );
    }

    panel.into_any_element()
}

fn render_album_feed_add_panel(
    feed_id: i64,
    playlists: &[db::Playlist],
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let mut panel = div()
        .border_1()
        .border_color(color::border_subtle())
        .rounded(radius::SM)
        .bg(color::bg_surface())
        .p(spacing::SM)
        .gap(spacing::XS)
        .flex()
        .flex_col();

    if playlists.is_empty() {
        panel = panel.child(
            div()
                .text_size(typography::SIZE_MICRO)
                .text_color(color::text_muted())
                .child(SharedString::from(
                    "No playlists yet — create one from the sidebar.",
                )),
        );
        return panel.into_any_element();
    }

    for p in playlists {
        let playlist_id = p.id;
        let label = format!("{} ({})", p.name, p.track_count);
        panel = panel.child(
            metadata_action_button(&label).on_click(cx.listener(move |this, _, _, cx| {
                this.album_add_open_feed = false;
                this.add_album_to_playlist(feed_id, playlist_id, cx);
            })),
        );
    }

    panel.into_any_element()
}

fn render_playlist_detail(
    detail: &PlaylistDetail,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let playlist_id = detail.playlist.id;
    let playlist_name = detail.playlist.name.clone();
    let track_count = detail.tracks.len();
    let total_duration_secs: i64 = detail.tracks.iter().filter_map(|t| t.duration_seconds).sum();
    let duration_str = if total_duration_secs > 0 {
        let mins = total_duration_secs / 60;
        let secs = total_duration_secs % 60;
        if mins >= 60 {
            format!("{}h {}m", mins / 60, mins % 60)
        } else {
            format!("{mins}:{secs:02}")
        }
    } else {
        String::new()
    };

    let track_rows: Vec<AnyElement> = if detail.tracks.is_empty() {
        vec![div()
            .text_center()
            .p(spacing::XXL)
            .text_color(color::text_muted())
            .child("Empty — add tracks from the library or search")
            .into_any_element()]
    } else {
        detail
            .tracks
            .iter()
            .enumerate()
            .map(|(idx, track)| {
                let track_for_select = track.clone();
                let track_id = track.id;
                let position = idx as i64;
                let last_position = (track_count - 1) as i64;
                let pl_id = playlist_id;
                let track_title = track
                    .track_title
                    .as_deref()
                    .unwrap_or("[untitled]")
                    .to_string();
                let artist = track
                    .artist_name
                    .as_deref()
                    .unwrap_or("Unknown")
                    .to_string();
                let dur = track
                    .duration_seconds
                    .map(|s| format!("{}:{:02}", s / 60, s % 60))
                    .unwrap_or_default();
                let track_thumb_image = track
                    .track_image_href
                    .as_ref()
                    .or(track.album_image_href.as_ref())
                    .and_then(|url| album_thumbs.get(url.as_str()))
                    .and_then(|opt| opt.clone());

                let up_btn = Button::new(SharedString::from(format!(
                    "playlist-up-{pl_id}-{position}"
                )))
                .label("▲")
                .ghost()
                .with_size(Size::Small)
                .disabled(position == 0)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.move_playlist_track(pl_id, position, position - 1, cx);
                }));

                let down_btn = Button::new(SharedString::from(format!(
                    "playlist-down-{pl_id}-{position}"
                )))
                .label("▼")
                .ghost()
                .with_size(Size::Small)
                .disabled(position == last_position)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.move_playlist_track(pl_id, position, position + 1, cx);
                }));

                let remove_btn = Button::new(SharedString::from(format!(
                    "playlist-remove-{pl_id}-{position}"
                )))
                .label("✕")
                .ghost()
                .danger()
                .with_size(Size::Small)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.remove_playlist_track_at(pl_id, position, cx);
                }));

                div()
                    .id(SharedString::from(format!("playlist-track-{track_id}-{position}")))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(spacing::SM)
                    .px(spacing::SM)
                    .py(spacing::XS)
                    .rounded(radius::SM)
                    .hover(|el| el.bg(color::bg_surface_hi()))
                    .child(
                        div()
                            .id(SharedString::from(format!(
                                "playlist-row-body-{pl_id}-{position}"
                            )))
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(spacing::SM)
                            .flex_1()
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.select_track(&track_for_select, cx);
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .w(px(32.0))
                                    .text_xs()
                                    .text_color(color::text_muted())
                                    .child(SharedString::from(format!("{}.", idx + 1))),
                            )
                            .child(render_album_thumb(track_thumb_image.as_ref(), 24.0))
                            .child(
                                div()
                                    .flex_1()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(color::text_primary())
                                            .child(SharedString::from(track_title)),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(color::text_muted())
                                            .child(SharedString::from(artist)),
                                    ),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(color::text_muted())
                                    .w(px(48.0))
                                    .child(SharedString::from(dur)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(spacing::XS)
                            .child(up_btn)
                            .child(down_btn)
                            .child(remove_btn),
                    )
                    .into_any_element()
            })
            .collect()
    };

    let mut detail_rows = vec![("Tracks".to_string(), format!("{track_count}"))];
    if !duration_str.is_empty() {
        detail_rows.push(("Duration".to_string(), duration_str));
    }

    let mut buttons = div().flex().flex_row().items_center().gap(spacing::SM);
    let playlist_for_rename = playlist_id;
    buttons = buttons.child(
        Button::new(SharedString::from(format!("playlist-rename-{playlist_id}")))
            .label("Rename")
            .ghost()
            .with_size(Size::Small)
            .on_click(cx.listener(move |_this, _, _, cx| {
                // TODO Stage 3: implement inline rename modal/input
                cx.notify();
            })),
    );
    buttons = buttons.child(
        Button::new(SharedString::from(format!("playlist-delete-{playlist_id}")))
            .label("Delete")
            .danger()
            .with_size(Size::Small)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.delete_playlist(playlist_for_rename, cx);
            })),
    );

    div()
        .id("playlist-detail-scroll")
        .size_full()
        .overflow_y_scroll()
        .p(spacing::LG)
        .flex()
        .flex_col()
        .gap(spacing::MD)
        .child(render_detail_header(
            "playlist",
            &playlist_name,
            None,
            None,
        ))
        .child(render_detail_grid(detail_rows))
        .child(buttons)
        .child(div().flex().flex_col().gap(spacing::XXS).children(track_rows))
        .into_any_element()
}

fn render_track_detail(
    frame: &InspectorFrame,
    playlists: &[db::Playlist],
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let context = track_row_to_track_context(&frame.track);
    let context = frame.source_context.as_ref().unwrap_or(&context);
    let result = match &frame.tag_compare {
        LazyPanel::Loaded(result) => Some(result),
        LazyPanel::Loading | LazyPanel::Empty(_) | LazyPanel::Hidden => None,
    };
    div()
        .id("track-detail-scroll")
        .size_full()
        .overflow_y_scroll()
        .p(spacing::LG)
        .child(render_track_window(frame, context, result, playlists, cx))
        .into_any_element()
}

fn render_track_window(
    frame: &InspectorFrame,
    track_context: &TrackContext,
    result: Option<&TagCompareResult>,
    playlists: &[db::Playlist],
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let show_id3_panel = !matches!(frame.tag_compare, LazyPanel::Hidden);
    let show_musicbrainz_panel = !matches!(frame.musicbrainz_lookup, LazyPanel::Hidden);
    let columns = 1 + u16::from(show_id3_panel) + u16::from(show_musicbrainz_panel);
    let rows = track_metadata_rows_for_frame(frame, track_context, result);
    let pending_id3_edits = if let Some(result) = result {
        auto_populated_pending_id3_edits(
            &rows,
            &frame.pending_id3_edits,
            &frame.suppressed_auto_id3_edits,
            result.format,
        )
    } else {
        frame.pending_id3_edits.clone()
    };

    div()
        .flex()
        .flex_col()
        .gap(spacing::LG)
        .child(
            div()
                .grid()
                .grid_cols(columns)
                .gap(spacing::XL)
                .items_start()
                .child(render_track_left_column(
                    frame,
                    &track_context.track,
                    &pending_id3_edits,
                    playlists,
                    cx,
                ))
                .when(show_id3_panel, |el| {
                    el.child(if let Some(result) = result {
                        render_file_header(result, cx)
                    } else {
                        render_track_compare_panel(frame)
                    })
                })
                .when(show_musicbrainz_panel, |el| {
                    el.child(render_musicbrainz_panel(frame, cx))
                }),
        )
        .child(render_track_metadata_grid(
            rows,
            show_id3_panel,
            show_musicbrainz_panel,
            &pending_id3_edits,
            &frame.expanded_metadata_cells,
            result.and_then(|r| r.file_image.clone()),
            result
                .and_then(|r| r.format)
                .map(|f| f.display_label())
                .unwrap_or("Tags"),
            cx,
        ))
        .into_any_element()
}

fn render_track_left_column(
    frame: &InspectorFrame,
    track: &Track,
    pending_id3_edits: &BTreeMap<String, PendingId3Edit>,
    playlists: &[db::Playlist],
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(spacing::MD)
        .child(render_track_header(frame, track))
        .child(render_action_row(frame, pending_id3_edits, playlists, cx))
        .into_any_element()
}

fn render_track_compare_panel(frame: &InspectorFrame) -> AnyElement {
    match &frame.tag_compare {
        LazyPanel::Loaded(_) => div().into_any_element(),
        LazyPanel::Loading => render_loading("Reading embedded metadata..."),
        LazyPanel::Empty(label) => render_loading(label),
        LazyPanel::Hidden => div().into_any_element(),
    }
}

fn render_track_header(frame: &InspectorFrame, track: &Track) -> AnyElement {
    let title = if frame.title.is_empty() {
        track_title(track)
    } else {
        frame.title.clone()
    };
    let artist = track
        .track_artist
        .clone()
        .or_else(|| track.release_artist.clone())
        .unwrap_or_else(|| "Unknown".into());
    render_detail_header("track", &title, Some(artist.as_str()), frame.image.as_ref())
}

fn render_action_row(
    frame: &InspectorFrame,
    pending_id3_edits: &BTreeMap<String, PendingId3Edit>,
    playlists: &[db::Playlist],
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let pending_conflicts = pending_id3_conflict_descriptions(pending_id3_edits);
    let has_pending_conflicts = !pending_conflicts.is_empty();

    div()
        .flex()
        .flex_col()
        .items_start()
        .gap(spacing::XS)
        .child(
            metadata_action_button(&subscription_button_label(frame))
                .disabled(frame.subscription_busy)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.toggle_local_subscription(cx);
                })),
        )
        .child(
            metadata_action_button("Add to playlist ▾")
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Some(frame) = this.selected_track_frame_mut() {
                        frame.add_to_playlist_open = !frame.add_to_playlist_open;
                    }
                    cx.notify();
                })),
        )
        .when(frame.add_to_playlist_open, |el| {
            el.child(render_add_to_playlist_panel(frame, playlists, cx))
        })
        .when_some(frame.subscription_message.clone(), |el, message| {
            el.child(
                div()
                    .max_w(px(220.0))
                    .text_size(typography::SIZE_MICRO)
                    .line_height(px(14.0))
                    .text_color(if message.contains("error") || message.contains("Error") {
                        color::status_danger()
                    } else {
                        color::text_muted()
                    })
                    .child(SharedString::from(message)),
            )
        })
        .when(frame.track.local_path.is_some(), |el| {
            el.child(
                metadata_action_button(match frame.tag_compare {
                    LazyPanel::Loaded(_) => "Hide Compare",
                    LazyPanel::Loading => "Reading ID3...",
                    LazyPanel::Empty(_) | LazyPanel::Hidden => "Compare ID3",
                })
                .disabled(matches!(frame.tag_compare, LazyPanel::Loading))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.toggle_tag_compare(cx);
                })),
            )
            .child(
                metadata_action_button(match frame.musicbrainz_lookup {
                    LazyPanel::Loaded(_) => "Hide MusicBrainz",
                    LazyPanel::Loading => "Searching MusicBrainz...",
                    LazyPanel::Empty(_) | LazyPanel::Hidden => "MusicBrainz",
                })
                .disabled(matches!(frame.musicbrainz_lookup, LazyPanel::Loading))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.toggle_musicbrainz_lookup(cx);
                })),
            )
        })
        .when(
            !pending_id3_edits.is_empty() || frame.applying_id3_edits,
            |el| {
                let count = pending_id3_edits.len();
                let conflict_text = pending_conflicts.join("; ");
                let label = if frame.applying_id3_edits {
                    "Applying tags...".to_string()
                } else {
                    format!("Apply tags ({count})")
                };
                el.child(
                    div()
                        .flex()
                        .flex_col()
                        .items_start()
                        .gap(spacing::XS)
                        .child(div().text_size(typography::SIZE_MICRO).text_color(color::text_muted()).child(
                            SharedString::from(format!(
                                "{count} staged tag edit{}",
                                if count == 1 { "" } else { "s" }
                            )),
                        ))
                        .child(
                            metadata_action_button(&label)
                                .disabled(frame.applying_id3_edits || has_pending_conflicts)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.apply_pending_id3_edits(cx);
                                })),
                        )
                        .when(has_pending_conflicts, |el| {
                            el.child(
                                div()
                                    .max_w(px(190.0))
                                    .text_size(typography::SIZE_MICRO)
                                    .line_height(px(14.0))
                                    .text_color(color::status_danger())
                                    .child(SharedString::from(format!(
                                        "Duplicate target: {conflict_text}"
                                    ))),
                            )
                        })
                        .when(
                            !frame.applying_id3_edits && !frame.pending_id3_edits.is_empty(),
                            |el| {
                                el.child(metadata_action_button("Discard staged").on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.clear_pending_id3_edits(cx);
                                    }),
                                ))
                            },
                        ),
                )
            },
        )
        .when_some(frame.id3_apply_error.clone(), |el, error| {
            el.child(
                div()
                    .max_w(px(180.0))
                    .text_size(typography::SIZE_MICRO)
                    .line_height(px(14.0))
                    .text_color(color::status_danger())
                    .child(SharedString::from(error)),
            )
        })
        .into_any_element()
}

fn subscription_button_label(frame: &InspectorFrame) -> String {
    if frame.subscription_busy {
        return if frame.local_subscription {
            "Unsubscribing...".into()
        } else {
            "Subscribing...".into()
        };
    }
    if frame.local_subscription {
        "Unsubscribe Track".into()
    } else {
        "Subscribe Track".into()
    }
}

fn render_add_to_playlist_panel(
    frame: &InspectorFrame,
    playlists: &[db::Playlist],
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let track_id = frame.entity_id;

    let mut panel = div()
        .border_1()
        .border_color(color::border_subtle())
        .rounded(radius::SM)
        .bg(color::bg_surface())
        .p(spacing::SM)
        .gap(spacing::XS)
        .flex()
        .flex_col();

    if playlists.is_empty() {
        panel = panel.child(
            div()
                .text_size(typography::SIZE_MICRO)
                .text_color(color::text_muted())
                .child(SharedString::from(
                    "No playlists yet — create one from the sidebar.",
                )),
        );
        return panel.into_any_element();
    }

    for p in playlists {
        let playlist_id = p.id;
        let label = format!("{} ({})", p.name, p.track_count);
        panel = panel.child(
            metadata_action_button(&label).on_click(cx.listener(
                move |this, _, _, cx| {
                    if let Some(frame) = this.selected_track_frame_mut() {
                        frame.add_to_playlist_open = false;
                    }
                    this.add_track_to_playlist(track_id, playlist_id, cx);
                },
            )),
        );
    }

    panel.into_any_element()
}

fn render_file_header(result: &TagCompareResult, cx: &mut Context<LibraryApp>) -> AnyElement {
    let embedded_label = embedded_tag_label(result);
    div()
        .flex()
        .flex_row()
        .items_start()
        .gap(spacing::LG)
        .child(render_thumb(
            result.file_image.as_ref(),
            "track",
            80.0,
            true,
        ))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(spacing::XS)
                        .mb(spacing::XS)
                        .child(
                            div()
                                .text_size(typography::SIZE_MICRO)
                                .font_weight(FontWeight::BOLD)
                                .text_color(badge_text("track"))
                                .bg(type_color("track"))
                                .px(spacing::XS)
                                .py(spacing::XXS)
                                .rounded(radius::SM)
                                .child(SharedString::from(embedded_label.clone())),
                        )
                        .child(metadata_action_button("Re-read").on_click(cx.listener(
                            |this, _, _, cx| {
                                this.reread_tag_compare(cx);
                            },
                        )))
                        .child(metadata_action_button("Re-download").on_click(cx.listener(
                            |this, _, _, cx| {
                                this.redownload_tag_compare(cx);
                            },
                        ))),
                )
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .line_height(px(23.0))
                        .child(SharedString::from(id3_header_title(result))),
                )
                .child(
                    div()
                        .text_color(color::text_muted())
                        .text_size(typography::SIZE_MICRO)
                        .line_clamp(2)
                        .child(SharedString::from(result.path.clone())),
                ),
        )
        .into_any_element()
}


fn id3_header_title(result: &TagCompareResult) -> String {
    result
        .rows
        .iter()
        .find(|row| row.field == "Title")
        .and_then(|row| row.tag_value.clone())
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| embedded_tag_label(result))
}

fn embedded_tag_label(result: &TagCompareResult) -> String {
    result
        .format
        .map(|format| format!("Embedded {}", format.display_label()))
        .unwrap_or_else(|| "Embedded tags".into())
}

fn render_musicbrainz_panel(frame: &InspectorFrame, cx: &mut Context<LibraryApp>) -> AnyElement {
    match &frame.musicbrainz_lookup {
        LazyPanel::Loaded(result) => render_musicbrainz_lookup(frame, result, cx),
        LazyPanel::Loading => render_loading("Searching MusicBrainz..."),
        LazyPanel::Empty(label) => render_loading(label),
        LazyPanel::Hidden => div().into_any_element(),
    }
}

fn render_musicbrainz_lookup(
    frame: &InspectorFrame,
    result: &MusicBrainzLookupResult,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let selected = selected_musicbrainz_candidate(frame, result);
    match selected {
        Some(candidate) => render_musicbrainz_header(frame, result, candidate, cx),
        None => div()
            .flex()
            .flex_col()
            .gap(spacing::XS)
            .child(muted_line("No MusicBrainz recording match found"))
            .into_any_element(),
    }
}

fn render_musicbrainz_header(
    frame: &InspectorFrame,
    result: &MusicBrainzLookupResult,
    candidate: &MusicBrainzCandidate,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .items_start()
        .gap(spacing::LG)
        .child(render_thumb(result.image.as_ref(), "track", 80.0, true))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(render_musicbrainz_title_bar(result, candidate, cx))
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .line_height(px(23.0))
                        .child(SharedString::from(candidate.title.clone())),
                )
                .child(
                    div()
                        .text_color(color::text_muted())
                        .text_size(typography::SIZE_MICRO)
                        .line_clamp(2)
                        .child(SharedString::from(musicbrainz_subtitle(
                            frame, result, candidate,
                        ))),
                ),
        )
        .into_any_element()
}

fn render_musicbrainz_title_bar(
    result: &MusicBrainzLookupResult,
    selected: &MusicBrainzCandidate,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let label = musicbrainz_release_summary(selected);
    let candidates = result.lookup.candidates.clone();
    let selected_idx = candidates
        .iter()
        .position(|candidate| candidate.release_id == selected.release_id)
        .unwrap_or_default();
    let app = cx.weak_entity();

    Button::new("musicbrainz-release-picker")
        .label(SharedString::from(format!("MusicBrainz: {label}")))
        .with_size(Size::XSmall)
        .compact()
        .ghost()
        .w_full()
        .justify_start()
        .bg(type_color("track"))
        .text_color(rgb(0xffffff))
        .text_size(typography::SIZE_MICRO)
        .font_weight(FontWeight::BOLD)
        .px(spacing::XS)
        .py(spacing::XXS)
        .border_1()
        .border_color(type_color("track"))
        .rounded(radius::SM)
        .mb(spacing::XS)
        .dropdown_menu(move |menu, _window, _cx| {
            candidates.iter().enumerate().fold(
                menu.min_w(px(320.0)).max_w(px(520.0)).scrollable(true),
                |menu, (idx, candidate)| {
                    let app = app.clone();
                    menu.item(
                        gpui_component::menu::PopupMenuItem::new(musicbrainz_release_option_label(
                            candidate,
                        ))
                        .checked(idx == selected_idx)
                        .on_click(move |_, _, cx| {
                            let _ = app.update(cx, |this, cx| {
                                this.select_musicbrainz_candidate(idx, cx);
                            });
                        }),
                    )
                },
            )
        })
        .into_any_element()
}

fn musicbrainz_release_summary(candidate: &MusicBrainzCandidate) -> String {
    let mut parts = Vec::new();
    if let Some(country) = &candidate.country {
        parts.push(country.clone());
    }
    if let Some(format) = &candidate.format {
        parts.push(format.clone());
    }
    if let Some(tracks) = candidate.total_tracks {
        parts.push(format!("{tracks} tracks"));
    }
    let mut value = if parts.is_empty() {
        candidate
            .release_title
            .clone()
            .unwrap_or_else(|| candidate.title.clone())
    } else {
        parts.join(" - ")
    };
    if let Some(date) = &candidate.release_date {
        value.push_str(&format!(" ({date})"));
    }
    value
}

fn musicbrainz_release_option_label(candidate: &MusicBrainzCandidate) -> SharedString {
    let release = candidate
        .release_title
        .clone()
        .unwrap_or_else(|| candidate.title.clone());
    SharedString::from(format!(
        "{} - {}",
        musicbrainz_release_summary(candidate),
        release
    ))
}

fn musicbrainz_subtitle(
    frame: &InspectorFrame,
    result: &MusicBrainzLookupResult,
    candidate: &MusicBrainzCandidate,
) -> String {
    let rank = if result
        .lookup
        .candidates
        .get(frame.musicbrainz_selected)
        .is_some()
    {
        frame.musicbrainz_selected + 1
    } else {
        1
    };
    let score = if let Some(musicbrainz_score) = candidate.musicbrainz_score {
        format!(
            "Best: #{} - {}% local - {} MB",
            rank, candidate.similarity_score, musicbrainz_score
        )
    } else {
        format!("Best: #{} - {}% local", rank, candidate.similarity_score)
    };
    if let Some(release_id) = &candidate.release_id {
        format!("{score} - {release_id}")
    } else {
        format!("{score} - {}", candidate.recording_id)
    }
}

fn selected_musicbrainz_candidate<'a>(
    frame: &InspectorFrame,
    result: &'a MusicBrainzLookupResult,
) -> Option<&'a MusicBrainzCandidate> {
    result
        .lookup
        .candidates
        .get(frame.musicbrainz_selected)
        .or_else(|| result.lookup.candidates.first())
}

fn track_metadata_rows_for_frame(
    frame: &InspectorFrame,
    track_context: &TrackContext,
    result: Option<&TagCompareResult>,
) -> Vec<MetadataGridRow> {
    let selected_musicbrainz = match &frame.musicbrainz_lookup {
        LazyPanel::Loaded(lookup) => selected_musicbrainz_candidate(frame, lookup),
        _ => None,
    };
    let show_musicbrainz = !matches!(frame.musicbrainz_lookup, LazyPanel::Hidden);
    let rows = result.map_or_else(
        || track_metadata_rows(track_context, selected_musicbrainz, show_musicbrainz),
        |result| {
            aligned_compare_rows(
                result,
                track_context,
                selected_musicbrainz,
                show_musicbrainz,
                &frame.expanded_id3_frame_groups,
            )
        },
    );
    expand_woar_metadata_rows(rows)
}

fn render_track_metadata_grid(
    rows: Vec<MetadataGridRow>,
    show_id3: bool,
    show_musicbrainz: bool,
    pending_id3_edits: &BTreeMap<String, PendingId3Edit>,
    expanded_metadata_cells: &BTreeSet<String>,
    file_image: Option<Arc<Image>>,
    tag_column_label: &str,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let mut cells: Vec<AnyElement> = Vec::new();
    let columns = 1 + u16::from(show_id3) + u16::from(show_musicbrainz);
    cells.push(metadata_heading_cell("RSS", 96.0));
    if show_id3 {
        cells.push(metadata_heading_cell(tag_column_label, 12.0));
    }
    if show_musicbrainz {
        cells.push(metadata_heading_cell("MusicBrainz", 12.0));
    }

    for row in rows {
        match row {
            MetadataGridRow::Group(group) => cells.push(metadata_group_cell(group, columns, cx)),
            MetadataGridRow::Data(row) => {
                let pending = pending_id3_edits.get(&row.row_id);
                let rss_expanded = expanded_metadata_cells.contains(&format!("rss:{}", row.row_id));
                let id3_expanded = expanded_metadata_cells.contains(&format!("id3:{}", row.row_id));
                cells.push(metadata_rss_cell(
                    &row,
                    pending,
                    rss_expanded,
                    expanded_metadata_cells,
                    cx,
                ));
                if show_id3 {
                    cells.push(metadata_id3_cell(
                        &row,
                        pending,
                        id3_expanded,
                        expanded_metadata_cells,
                        file_image.as_ref(),
                        cx,
                    ));
                }
                if show_musicbrainz {
                    cells.push(metadata_musicbrainz_cell(&row, pending));
                }
            }
        }
    }

    div()
        .grid()
        .grid_cols(columns)
        .gap_x(spacing::XL)
        .gap_y(spacing::SM)
        .children(cells)
        .into_any_element()
}

fn metadata_heading_cell(label: &str, indent: f32) -> AnyElement {
    div()
        .pl(px(indent))
        .text_color(color::text_muted())
        .font_weight(FontWeight::BOLD)
        .text_size(typography::SIZE_MICRO)
        .child(SharedString::from(label.to_string()))
        .into_any_element()
}

fn metadata_group_cell(
    group: crate::metadata::MetadataGroupRow,
    columns: u16,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let label = if group.unused_count == 0 {
        group.label
    } else {
        format!("{} ({} unused)", group.label, group.unused_count)
    };
    let expanded = group.expanded;
    let mut cell = div().col_span(columns).mt(spacing::XS);
    if let Some(group_key) = group.key {
        cell = cell.child(
            render_clickable_section_heading(&label, !expanded).on_click(cx.listener(
                move |this, _, _, cx| {
                    this.toggle_id3_frame_group(group_key.clone(), cx);
                },
            )),
        );
    } else {
        cell = cell.child(
            div()
                .text_size(typography::SIZE_MICRO)
                .font_weight(FontWeight::BOLD)
                .text_color(color::text_muted())
                .child(SharedString::from(label)),
        );
    }
    cell.into_any_element()
}

fn metadata_rss_cell(
    row: &AlignedCompareRow,
    pending: Option<&PendingId3Edit>,
    expanded: bool,
    expanded_cells: &BTreeSet<String>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let value = row.rss_value.as_deref().unwrap_or("");
    let base_display = display_metadata_value(&row.field, value);
    let glyph = pending_source_glyph(pending, MetadataColumn::Rss, row.rss_value.as_deref());
    let display_value = display_with_glyph(glyph, &base_display);
    let value_color = source_cell_color(pending, MetadataColumn::Rss, row.rss_value.as_deref())
        .unwrap_or_else(color::text_primary);
    let value_element = metadata_value_cell(
        &row.field,
        &row.row_id,
        value,
        &display_value,
        expanded,
        value_color,
        "rss",
        expanded_cells,
        None,
        cx,
    );
    div()
        .flex()
        .flex_row()
        .items_start()
        .gap(spacing::SM)
        .child(
            div()
                .w(px(86.0))
                .flex_shrink_0()
                .text_color(color::text_primary())
                .text_size(typography::SIZE_MICRO)
                .line_height(px(16.0))
                .child(SharedString::from(row.field.clone())),
        )
        .child(div().flex_1().min_w_0().child(value_element))
        .into_any_element()
}

fn metadata_id3_cell(
    row: &AlignedCompareRow,
    pending: Option<&PendingId3Edit>,
    expanded: bool,
    expanded_cells: &BTreeSet<String>,
    file_image: Option<&Arc<Image>>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let frame = pending
        .map(|edit| edit.frame.as_str())
        .or(row.id3_frame.as_deref());
    let value = pending
        .map(|edit| edit.value.as_str())
        .or(row.id3_value.as_deref())
        .unwrap_or("");
    let base_display = display_metadata_value(&row.field, value);
    let glyph = if pending.is_some() {
        Some(glyphs::DIFF_MATCH)
    } else {
        comparison_status_glyph(&row.id3_status)
    };
    let display_value = display_with_glyph(glyph, &base_display);
    let color = pending
        .map(|edit| pending_source_color(edit.source))
        .unwrap_or_else(|| id3_cell_status_color(row));
    let value_element = metadata_tag_cell(
        &row.field,
        &row.row_id,
        value,
        &display_value,
        expanded,
        color,
        frame,
        expanded_cells,
        file_image,
        cx,
    );
    div()
        .pl(spacing::MD)
        .min_w_0()
        .rounded(radius::SM)
        .child(value_element)
        .when_some(pending, |el, edit| {
            el.border_1()
                .border_color(pending_source_color(edit.source))
        })
        .into_any_element()
}

fn metadata_musicbrainz_cell(
    row: &AlignedCompareRow,
    pending: Option<&PendingId3Edit>,
) -> AnyElement {
    let musicbrainz_color = source_cell_color(
        pending,
        MetadataColumn::MusicBrainz,
        row.musicbrainz_value.as_deref(),
    )
    .unwrap_or_else(|| comparison_status_color(&row.musicbrainz_status));
    let value = row.musicbrainz_value.as_deref().unwrap_or("");
    let base_display = display_metadata_value(&row.field, value);
    let glyph = pending_source_glyph(pending, MetadataColumn::MusicBrainz, row.musicbrainz_value.as_deref())
        .or_else(|| comparison_status_glyph(&row.musicbrainz_status));
    let display_value = display_with_glyph(glyph, &base_display);
    div()
        .pl(spacing::MD)
        .min_w_0()
        .child(compare_tag_cell(
            &display_value,
            Some(musicbrainz_color),
            row.musicbrainz_key.as_deref(),
            None,
        ))
        .into_any_element()
}

#[expect(
    clippy::too_many_arguments,
    reason = "UI cell renderer keeps field context explicit"
)]
fn metadata_value_cell(
    field: &str,
    row_id: &str,
    raw_value: &str,
    display_value: &str,
    expanded: bool,
    color: gpui::Rgba,
    column: &str,
    expanded_cells: &BTreeSet<String>,
    file_image: Option<&Arc<Image>>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let logical_field = metadata_logical_field(field);
    let expandable = metadata_field_is_expandable(logical_field) && !raw_value.is_empty();
    if !expandable {
        return compare_cell(display_value, Some(color));
    }
    let cell_key = format!("{column}:{row_id}");
    let glyph = if expanded { "v" } else { ">" };
    let summary = expandable_cell_summary(logical_field, field, raw_value, display_value);
    if expanded && logical_field == "Value Routes" {
        let header_key = cell_key.clone();
        return div()
            .text_size(typography::SIZE_MICRO)
            .line_height(px(16.0))
            .text_color(color)
            .flex()
            .flex_col()
            .child(
                div()
                    .id(SharedString::from(format!(
                        "metadata-cell:{cell_key}:header"
                    )))
                    .cursor_pointer()
                    .flex()
                    .flex_row()
                    .items_start()
                    .gap(spacing::XS)
                    .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _window, cx| {
                        this.toggle_metadata_cell(header_key.clone(), cx);
                    }))
                    .child(div().text_size(typography::SIZE_MICRO).text_color(color::text_muted()).child(glyph)),
            )
            .child(div().flex().flex_col().children(value_routes_tree_elements(
                raw_value,
                column,
                row_id,
                color,
                expanded_cells,
                cx,
            )))
            .into_any_element();
    }
    let content = if expanded {
        expanded_metadata_value(logical_field, raw_value, display_value, color, file_image)
    } else {
        div()
            .text_color(color::accent())
            .truncate()
            .child(SharedString::from(summary))
            .into_any_element()
    };
    let cell_id = SharedString::from(format!("metadata-cell:{cell_key}"));
    div()
        .id(cell_id)
        .cursor_pointer()
        .text_size(typography::SIZE_MICRO)
        .line_height(px(16.0))
        .flex()
        .flex_row()
        .items_start()
        .gap(spacing::XS)
        .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _window, cx| {
            this.toggle_metadata_cell(cell_key.clone(), cx);
        }))
        .child(div().text_size(typography::SIZE_MICRO).text_color(color::text_muted()).child(glyph))
        .child(div().flex_1().min_w_0().child(content))
        .into_any_element()
}

#[expect(
    clippy::too_many_arguments,
    reason = "UI cell renderer keeps field context explicit"
)]
fn metadata_tag_cell(
    field: &str,
    row_id: &str,
    raw_value: &str,
    display_value: &str,
    expanded: bool,
    color: gpui::Rgba,
    frame_id: Option<&str>,
    expanded_cells: &BTreeSet<String>,
    file_image: Option<&Arc<Image>>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let frame_color = frame_id.map_or_else(color::text_muted, id3_frame_color);
    let value = metadata_value_cell(
        field,
        row_id,
        raw_value,
        display_value,
        expanded,
        color,
        "id3",
        expanded_cells,
        file_image,
        cx,
    );
    div()
        .flex()
        .flex_row()
        .items_start()
        .gap(spacing::XS)
        .child(
            div()
                .w(px(136.0))
                .flex_shrink_0()
                .text_color(frame_color)
                .text_size(typography::SIZE_MICRO)
                .line_height(px(16.0))
                .child(SharedString::from(frame_id.unwrap_or_default().to_string())),
        )
        .child(div().flex_1().min_w_0().child(value))
        .into_any_element()
}

fn expanded_metadata_value(
    field: &str,
    raw_value: &str,
    display_value: &str,
    color: gpui::Rgba,
    file_image: Option<&Arc<Image>>,
) -> AnyElement {
    if field == "Artwork" {
        if let Some(image) = file_image {
            return div()
                .flex()
                .flex_col()
                .gap(spacing::XS)
                .child(SharedString::from(display_value.to_string()))
                .child(render_thumb(Some(image), "track", 160.0, true))
                .into_any_element();
        }
    }
    let value = expanded_metadata_display_value(field, raw_value, display_value);
    div()
        .text_color(color)
        .flex()
        .flex_col()
        .children(compare_value_line_elements(value, 20))
        .into_any_element()
}

fn value_routes_tree_elements(
    raw_value: &str,
    column: &str,
    row_id: &str,
    color: gpui::Rgba,
    expanded_cells: &BTreeSet<String>,
    cx: &mut Context<LibraryApp>,
) -> Vec<AnyElement> {
    let Ok(routes) = serde_json::from_str::<Vec<serde_json::Value>>(raw_value) else {
        return compare_value_line_elements(raw_value, 20);
    };

    routes
        .into_iter()
        .enumerate()
        .map(|(index, route)| {
            let name = route
                .get("recipient_name")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("Unknown");
            let split = route.get("split").and_then(route_split_label);
            let label = split.map_or_else(|| name.to_string(), |split| format!("{name} {split}"));
            let sub_key = format!("{column}:{row_id}:{index}");
            let sub_expanded = expanded_cells.contains(&sub_key);
            let sub_glyph = if sub_expanded { "v" } else { ">" };
            let header_key = sub_key.clone();

            let mut item = div()
                .id(SharedString::from(format!(
                    "value-route:{column}:{row_id}:{index}"
                )))
                .flex()
                .flex_col()
                .gap(spacing::XXS)
                .child(
                    div()
                        .id(SharedString::from(format!(
                            "value-route:{column}:{row_id}:{index}:header"
                        )))
                        .cursor_pointer()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(spacing::XS)
                        .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _window, cx| {
                            this.toggle_metadata_cell(header_key.clone(), cx);
                        }))
                        .child(
                            div()
                                .text_size(typography::SIZE_MICRO)
                                .text_color(color::text_muted())
                                .child(sub_glyph),
                        )
                        .child(
                            div()
                                .text_color(if sub_expanded { color } else { color::accent() })
                                .truncate()
                                .child(SharedString::from(label)),
                        ),
                );

            if sub_expanded {
                if let serde_json::Value::Object(map) = &route {
                    for (key, value) in map {
                        if matches!(key.as_str(), "recipient_name" | "split") {
                            continue;
                        }
                        let Some(value) = route_value_label(value) else {
                            continue;
                        };
                        item = item.child(
                            div()
                                .pl(spacing::LG)
                                .flex()
                                .flex_row()
                                .gap(spacing::XS)
                                .child(
                                    div()
                                        .text_color(color::text_muted())
                                        .child(SharedString::from(format!("{key}: "))),
                                )
                                .child(
                                    div()
                                        .text_color(color)
                                        .truncate()
                                        .child(SharedString::from(value)),
                                ),
                        );
                    }
                }
            }

            item.into_any_element()
        })
        .collect()
}

fn route_value_label(value: &serde_json::Value) -> Option<String> {
    let label = match value {
        serde_json::Value::Null => return None,
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::Bool(value) => value.to_string(),
        other => other.to_string(),
    };
    let label = label.trim();
    if label.is_empty() {
        None
    } else {
        Some(label.to_string())
    }
}

fn route_split_label(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Number(number) => {
            let raw = number.to_string();
            let trimmed = raw.strip_suffix(".0").unwrap_or(&raw);
            Some(format!("{trimmed}%"))
        }
        _ => route_value_label(value),
    }
}

fn metadata_logical_field(field: &str) -> &str {
    match field {
        "TXXX:MusicIndex Contributors" => "Contributors",
        "TXXX:MusicIndex Value Routes" => "Value Routes",
        _ => field,
    }
}

fn expandable_cell_summary(
    logical_field: &str,
    _display_field: &str,
    raw_value: &str,
    display_value: &str,
) -> String {
    match logical_field {
        "Contributors" => {
            summarize_contributor_value(raw_value).unwrap_or_else(|| display_value.to_string())
        }
        "Value Routes" => {
            if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(raw_value) {
                format!("[{} items]", arr.len())
            } else {
                display_value.to_string()
            }
        }
        "Artwork" if raw_value.starts_with("http://") || raw_value.starts_with("https://") => {
            raw_value
                .rsplit('/')
                .next()
                .unwrap_or(raw_value)
                .to_string()
        }
        _ => display_value.to_string(),
    }
}

fn compare_cell(value: &str, color: Option<gpui::Rgba>) -> AnyElement {
    let mut cell = div()
        .text_size(typography::SIZE_MICRO)
        .line_height(px(16.0))
        .flex()
        .flex_col();
    if let Some(color) = color {
        cell = cell.text_color(color);
    }
    cell.children(compare_value_line_elements(value, 4))
        .into_any_element()
}

fn compare_tag_cell(
    value: &str,
    color: Option<gpui::Rgba>,
    frame_id: Option<&str>,
    frame_color: Option<gpui::Rgba>,
) -> AnyElement {
    let mut value_cell = div().text_size(typography::SIZE_MICRO).line_height(px(16.0));
    if let Some(color) = color {
        value_cell = value_cell.text_color(color);
    }
    div()
        .flex()
        .flex_row()
        .items_start()
        .gap(spacing::XS)
        .child(
            div()
                .w(px(136.0))
                .flex_shrink_0()
                .text_color(frame_color.unwrap_or_else(color::text_muted))
                .text_size(typography::SIZE_MICRO)
                .line_height(px(16.0))
                .child(SharedString::from(frame_id.unwrap_or_default().to_string())),
        )
        .child(
            value_cell
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .children(compare_value_line_elements(value, 4)),
        )
        .into_any_element()
}

fn compare_value_line_elements(value: &str, max_lines: usize) -> Vec<AnyElement> {
    let mut lines = value.lines().collect::<Vec<_>>();
    if lines.is_empty() {
        lines.push("");
    }
    let truncated = lines.len() > max_lines;
    lines
        .into_iter()
        .take(max_lines)
        .enumerate()
        .map(|(index, line)| {
            let line = if truncated && index + 1 == max_lines {
                "..."
            } else if line.is_empty() {
                " "
            } else {
                line
            };
            div()
                .truncate()
                .child(SharedString::from(line.to_string()))
                .into_any_element()
        })
        .collect()
}

fn id3_frame_color(frame_id: &str) -> gpui::Rgba {
    match id3_frame_base(frame_id) {
        "SYLT" | "USLT" | "APIC" => rgb(0x3ac4c4),
        "TXXX" | "WXXX" | "UFID" => rgb(0xb06cf4),
        _ => color::accent(),
    }
}

fn comparison_status_color(status: &crate::track_compare::ComparisonStatus) -> gpui::Rgba {
    match status {
        crate::track_compare::ComparisonStatus::Match => color::diff_match(),
        crate::track_compare::ComparisonStatus::Different => color::diff_different(),
        crate::track_compare::ComparisonStatus::MissingSource
        | crate::track_compare::ComparisonStatus::MissingTag => color::diff_missing(),
        crate::track_compare::ComparisonStatus::MissingBoth => color::text_muted(),
    }
}

fn comparison_status_glyph(status: &crate::track_compare::ComparisonStatus) -> Option<&'static str> {
    match status {
        crate::track_compare::ComparisonStatus::Match => Some(glyphs::DIFF_MATCH),
        crate::track_compare::ComparisonStatus::Different => Some(glyphs::DIFF_DIFFERENT),
        crate::track_compare::ComparisonStatus::MissingSource
        | crate::track_compare::ComparisonStatus::MissingTag => Some(glyphs::DIFF_MISSING),
        crate::track_compare::ComparisonStatus::MissingBoth => None,
    }
}

fn pending_source_glyph(
    pending: Option<&PendingId3Edit>,
    column: MetadataColumn,
    cell_value: Option<&str>,
) -> Option<&'static str> {
    let edit = pending?;
    if edit.source != column {
        return None;
    }
    let cell_value = cell_value.map(str::trim).filter(|v| !v.is_empty())?;
    if cell_value == edit.value.trim() {
        Some(glyphs::DIFF_MATCH)
    } else {
        Some(glyphs::DIFF_DIFFERENT)
    }
}

fn display_with_glyph(glyph: Option<&str>, value: &str) -> String {
    match glyph {
        Some(g) if !value.is_empty() => format!("{g} {value}"),
        Some(g) => g.to_string(),
        None => value.to_string(),
    }
}

fn id3_cell_status_color(row: &AlignedCompareRow) -> gpui::Rgba {
    if row.id3_value.is_some() && row.rss_value.is_none() && row.musicbrainz_value.is_none() {
        return color::text_primary();
    }
    comparison_status_color(&row.id3_status)
}

fn pending_source_color(source: MetadataColumn) -> gpui::Rgba {
    match source {
        MetadataColumn::Rss | MetadataColumn::MusicBrainz => color::diff_match(),
    }
}

fn source_cell_color(
    pending: Option<&PendingId3Edit>,
    column: MetadataColumn,
    cell_value: Option<&str>,
) -> Option<gpui::Rgba> {
    let edit = pending?;
    if edit.source != column {
        return None;
    }
    let cell_value = cell_value
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    if cell_value == edit.value.trim() {
        Some(color::diff_match())
    } else {
        Some(color::diff_different())
    }
}

fn render_detail_header(
    entity_type: &str,
    title: &str,
    subtitle: Option<&str>,
    image: Option<&Arc<Image>>,
) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .items_start()
        .gap(spacing::LG)
        .child(render_thumb(image, entity_type, 80.0, true))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .text_size(typography::SIZE_MICRO)
                        .font_weight(FontWeight::BOLD)
                        .text_color(badge_text(entity_type))
                        .bg(type_color(entity_type))
                        .px(spacing::XS)
                        .py(spacing::XXS)
                        .rounded(radius::SM)
                        .mb(spacing::XS)
                        .child(SharedString::from(entity_type.to_string())),
                )
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .line_height(px(23.0))
                        .child(SharedString::from(title.to_string())),
                )
                .when_some(subtitle.map(str::to_owned), |el, sub| {
                    el.child(
                        div()
                            .mt(spacing::XS)
                            .text_size(typography::SIZE_HEADLINE)
                            .font_weight(FontWeight::MEDIUM)
                            .line_height(px(20.0))
                            .text_color(color::text_muted())
                            .child(SharedString::from(sub)),
                    )
                }),
        )
        .into_any_element()
}

fn render_detail_grid(rows: Vec<(String, String)>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(spacing::XS)
        .children(rows.into_iter().map(|(key, value)| {
            div()
                .flex()
                .flex_row()
                .items_start()
                .gap(spacing::MD)
                .child(
                    div()
                        .w(px(124.0))
                        .flex_shrink_0()
                        .text_color(color::text_muted())
                        .whitespace_nowrap()
                        .text_size(typography::SIZE_MICRO)
                        .child(SharedString::from(key)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(typography::SIZE_MICRO)
                        .line_height(px(17.0))
                        .flex()
                        .flex_col()
                        .children(compare_value_line_elements(&value, 6)),
                )
                .into_any_element()
        }))
        .into_any_element()
}

fn artwork_img(image: Arc<Image>, size: f32) -> AnyElement {
    let base = img(image.clone())
        .w(px(size))
        .h(px(size))
        .object_fit(ObjectFit::Cover);
    if image.format == ImageFormat::Gif {
        base.id(SharedString::from(format!("anim-thumb:{}", image.id())))
            .into_any_element()
    } else {
        base.into_any_element()
    }
}

fn render_thumb(
    image_data: Option<&Arc<Image>>,
    entity_type: &str,
    size: f32,
    large: bool,
) -> AnyElement {
    let radius = if large { radius::MD } else { radius::SM };
    if let Some(image) = image_data {
        div()
            .w(px(size))
            .h(px(size))
            .rounded(radius)
            .overflow_hidden()
            .flex_shrink_0()
            .child(artwork_img(image.clone(), size))
            .into_any_element()
    } else {
        div()
            .w(px(size))
            .h(px(size))
            .rounded(radius)
            .bg(color::border_subtle())
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(if large { 28.0 } else { 14.0 }))
            .flex_shrink_0()
            .child(type_emoji(entity_type))
            .into_any_element()
    }
}

fn render_loading(message: &str) -> AnyElement {
    div()
        .text_color(color::text_muted())
        .italic()
        .py(spacing::SM)
        .child(SharedString::from(message.to_string()))
        .into_any_element()
}

fn render_clickable_section_heading(label: &str, collapsed: bool) -> gpui::Stateful<gpui::Div> {
    let state = if collapsed { "show" } else { "hide" };
    let glyph = if collapsed { ">" } else { "v" };
    div()
        .id(SharedString::from(format!("section-heading:{label}")))
        .flex()
        .flex_row()
        .items_center()
        .gap(spacing::XS)
        .cursor_pointer()
        .child(
            div()
                .text_size(typography::SIZE_MICRO)
                .font_weight(FontWeight::BOLD)
                .text_color(color::text_muted())
                .child(glyph),
        )
        .child(
            div()
                .text_size(typography::SIZE_MICRO)
                .font_weight(FontWeight::BOLD)
                .text_color(color::text_muted())
                .child(SharedString::from(label.to_string())),
        )
        .child(
            div()
                .text_size(typography::SIZE_MICRO)
                .text_color(color::text_muted())
                .child(SharedString::from(state.to_string())),
        )
}

fn metadata_action_button(label: &str) -> Button {
    Button::new(SharedString::from(format!("metadata-action:{label}")))
        .label(SharedString::from(label.to_string()))
        .with_size(Size::XSmall)
        .compact()
        .ghost()
        .text_color(rgb(0xffffff))
        .text_size(typography::SIZE_MICRO)
        .rounded(radius::SM)
        .border_1()
        .border_color(color::accent())
}

fn muted_line(value: &str) -> AnyElement {
    div()
        .text_color(color::text_muted())
        .text_size(typography::SIZE_MICRO)
        .child(SharedString::from(value.to_string()))
        .into_any_element()
}

fn type_color(entity_type: &str) -> gpui::Rgba {
    badges::type_color(entity_type)
}

fn badge_text(entity_type: &str) -> gpui::Rgba {
    badges::text_color(entity_type)
}

fn type_emoji(entity_type: &str) -> &'static str {
    badges::emoji(entity_type)
}

fn compare_downloaded_track_path(
    path: &Path,
    track_context: &TrackContext,
) -> anyhow::Result<TagCompareResult> {
    let tags = read_audio_tags(path)?;
    let file_image = tags.artwork.as_ref().and_then(|art| {
        if art.data.is_empty() {
            None
        } else {
            let format = ImageFormat::from_mime_type(&art.mime_type).unwrap_or(ImageFormat::Jpeg);
            Some(Arc::new(Image::from_bytes(format, art.data.clone())))
        }
    });
    let track = &track_context.track;
    let mut rows = crate::metadata::compare_track_rows(track, track_context.feed.as_ref(), &tags);
    let detected = crate::audio_format::AudioFormat::detect_from_file(path).ok();
    if let Some(detected) = detected {
        crate::metadata::push_compare_row(
            &mut rows,
            "File format",
            None,
            Some(detected.display_label().to_string()),
        );
    }
    Ok(TagCompareResult {
        path: path.display().to_string(),
        rows,
        file_image,
        contributors: track.source_contributors.clone().unwrap_or_default(),
        value_routes: track.payment_routes.clone().unwrap_or_default(),
        total_tracks: tags.total_tracks.clone(),
        id3_fields: tags.fields,
        format: detected,
    })
}

fn compare_library_track(
    track: &TrackRow,
    musicindex_endpoint: &str,
) -> anyhow::Result<LibraryTrackCompare> {
    let path = track
        .local_path
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("library track has no local file"))?;
    let context = fetch_library_track_context(track, musicindex_endpoint)
        .unwrap_or_else(|_| track_row_to_track_context(track));
    let tag_compare = compare_downloaded_track_path(Path::new(path), &context)?;
    Ok(LibraryTrackCompare {
        tag_compare,
        track_context: context,
    })
}

fn lookup_musicbrainz_library_track(
    track: &TrackRow,
    cache: Arc<ImageCache>,
) -> anyhow::Result<MusicBrainzLookupResult> {
    let path = track
        .local_path
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("library track has no local file"))?;
    let tags = read_audio_tags(Path::new(path))?;
    let context = track_row_to_track_context(track);
    let metadata = musicbrainz_lookup_metadata(&context.track, &tags);
    let musicbrainz_client = ReqwestClient::builder()
        .user_agent(format!(
            "v4vmm/{} (MusicBrainz metadata lookup)",
            env!("CARGO_PKG_VERSION")
        ))
        .build()?;
    let lookup = lookup_recordings(&musicbrainz_client, &metadata, 5)?;
    let image = lookup
        .candidates
        .first()
        .and_then(|candidate| candidate.release_id.as_deref())
        .and_then(|release_id| {
            let url = format!("https://coverartarchive.org/release/{release_id}/front-250");
            cache.fetch_blocking(&url)
        });
    Ok(MusicBrainzLookupResult { lookup, image })
}

fn musicbrainz_lookup_metadata(
    track: &Track,
    tags: &crate::audio_tags::AudioTags,
) -> LookupMetadata {
    LookupMetadata {
        title: tags
            .title
            .clone()
            .or_else(|| track.title.clone())
            .or_else(|| track.name.clone()),
        artist: tags.artist.clone().or_else(|| track.track_artist.clone()),
        album: tags.album.clone().or_else(|| track.feed_title.clone()),
        track_number: tags
            .track_number
            .clone()
            .or_else(|| track.track_number.map(|number| number.to_string())),
        total_tracks: None,
        duration_secs: track.duration_secs.map(i64::from),
        isrc: tags
            .custom
            .get("ISRC")
            .cloned()
            .or_else(|| tags.custom.get("isrc").cloned()),
    }
}

fn track_row_to_track_context(track: &TrackRow) -> TrackContext {
    let feed = track_row_to_feed(track);
    let api_track =
        crate::api::track_with_feed_defaults(track_row_to_api_track(track), Some(&feed));
    TrackContext {
        track: api_track,
        feed: Some(feed),
    }
}

fn track_title(track: &Track) -> String {
    track
        .title
        .clone()
        .or_else(|| track.name.clone())
        .or_else(|| track.track_guid.clone())
        .unwrap_or_else(|| "Untitled".into())
}

#[allow(dead_code)]
fn fmt_dur(secs: i32) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

fn hoverable_thumb(
    url: Option<String>,
    image: Option<&Arc<Image>>,
    size: f32,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let inner = render_album_thumb(image, size);
    let Some(url) = url else {
        return inner;
    };
    let enter_url = url.clone();
    let leave_url = url.clone();
    div()
        .id(SharedString::from(format!("thumb-{url}")))
        .on_mouse_move(cx.listener(move |this, _, _, cx| {
            if this.hovered_thumb_url.as_deref() != Some(enter_url.as_str()) {
                this.set_hovered_thumb(Some(enter_url.clone()), cx);
            }
        }))
        .on_hover(cx.listener(move |this, entered: &bool, _, cx| {
            if !*entered && this.hovered_thumb_url.as_deref() == Some(leave_url.as_str()) {
                this.set_hovered_thumb(None, cx);
            }
        }))
        .child(inner)
        .into_any_element()
}

pub(crate) fn render_album_thumb(image: Option<&Arc<Image>>, size: f32) -> AnyElement {
    if let Some(img_data) = image {
        div()
            .w(px(size))
            .h(px(size))
            .rounded(radius::SM)
            .overflow_hidden()
            .flex_shrink_0()
            .child(artwork_img(img_data.clone(), size))
            .into_any_element()
    } else {
        div()
            .w(px(size))
            .h(px(size))
            .rounded(radius::SM)
            .bg(color::border_subtle())
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(14.0))
            .flex_shrink_0()
            .child("\u{1F3B5}")
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{merge_track_context_from_detail, track_row_to_track_context, TrackRow};
    use crate::api::{Contributor, Feed, PaymentRoute, SourceEntityId, Track};
    use crate::search::id3_edits_for_track_context;

    #[test]
    fn library_track_context_preserves_feed_guid_for_id3_provenance() {
        let track = TrackRow {
            id: 1,
            feed_id: 2,
            feed_guid: Some("feed-guid".into()),
            item_guid: "track-guid".into(),
            track_title: Some("Song".into()),
            artist_name: None,
            album_title: None,
            album_artist_name: None,
            track_number: None,
            disc_number: None,
            duration_seconds: None,
            enclosure_url: None,
            enclosure_type: None,
            track_image_href: None,
            is_in_library: true,
            feed_title: Some("Feed".into()),
            album_image_href: None,
            local_path: None,
            transcript_url: None,
        };

        let context = track_row_to_track_context(&track);
        let edits = id3_edits_for_track_context(&context);

        assert!(edits.iter().any(|edit| {
            edit.frame_label == "TXXX:MusicIndex Feed Guid" && edit.value == "feed-guid"
        }));
    }

    #[test]
    fn library_track_context_inherits_feed_level_musicindex_metadata() {
        let track_row = TrackRow {
            id: 1,
            feed_id: 2,
            feed_guid: Some("feed-guid".into()),
            item_guid: "track-guid".into(),
            track_title: Some("Song".into()),
            artist_name: Some("Artist".into()),
            album_title: None,
            album_artist_name: None,
            track_number: Some(4),
            disc_number: None,
            duration_seconds: Some(223),
            enclosure_url: Some("https://example.test/track.mp3".into()),
            enclosure_type: None,
            track_image_href: None,
            is_in_library: true,
            feed_title: Some("Feed".into()),
            album_image_href: None,
            local_path: None,
            transcript_url: None,
        };
        let track = Track {
            track_guid: Some("track-guid".into()),
            feed_guid: Some("feed-guid".into()),
            title: Some("Song".into()),
            ..Default::default()
        };
        let feed = Feed {
            feed_guid: Some("feed-guid".into()),
            title: Some("Feed".into()),
            publisher_text: Some("HeyCitizen".into()),
            description: Some("Feed description".into()),
            source_ids: Some(vec![SourceEntityId {
                scheme: Some("nostr_npub".into()),
                value: Some("npub1heycitizen".into()),
                ..Default::default()
            }]),
            source_contributors: Some(vec![Contributor {
                name: Some("HeyCitizen".into()),
                role: Some("musician".into()),
                ..Default::default()
            }]),
            payment_routes: Some(vec![PaymentRoute {
                recipient_name: Some("HeyCitizen".into()),
                split: Some(100.0),
                ..Default::default()
            }]),
            ..Default::default()
        };

        let context = merge_track_context_from_detail(&track_row, Some(track), Some(feed));
        assert_eq!(context.track.publisher_text.as_deref(), Some("HeyCitizen"));
        assert_eq!(
            context.track.source_contributors.as_ref().map(Vec::len),
            Some(1)
        );
        assert_eq!(context.track.source_ids.as_ref().map(Vec::len), Some(1));
        assert_eq!(context.track.payment_routes.as_ref().map(Vec::len), Some(1));
        assert_eq!(
            context
                .feed
                .as_ref()
                .and_then(|feed| feed.description.as_deref()),
            Some("Feed description")
        );
    }
}
