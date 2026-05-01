#![warn(clippy::pedantic)]
#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::doc_markdown,
    clippy::manual_let_else,
    clippy::map_unwrap_or,
    clippy::match_same_arms,
    clippy::needless_pass_by_value,
    clippy::redundant_closure_for_method_calls,
    clippy::single_match_else,
    clippy::struct_excessive_bools,
    clippy::too_many_lines,
    clippy::unused_self,
    reason = "legacy screen module is being migrated incrementally under ADR 0023"
)]

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use gpui::{
    div, prelude::*, px, AnyElement, ClickEvent, Context, Entity, FontWeight, Image,
    InteractiveElement, IntoElement, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Render,
    SharedString, Styled, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::menu::DropdownMenu as _;
use gpui_component::Size;

use crate::api::Track;
use crate::application::commands::download::{
    RemoveTrackFromLibrary, SetTrackLibraryMembership, SubscribeThenAppendToPlaylist,
    SubscribeTrack,
};
use crate::application::commands::feed::{
    ApplyFeedUpdates, CheckFeedStaleness, CheckSubscribedFeeds, UnsubscribeFeedById,
};
use crate::application::commands::metadata::{
    LookupMusicBrainzAlbumReleases, LookupMusicBrainzTrack, StageMusicBrainzCandidate,
    StageMusicBrainzTrack,
};
use crate::application::commands::playlist::{
    CreatePlaylist, DeletePlaylist, RemovePlaylistTrackAt, RenamePlaylist, ReorderPlaylistTrack,
};
use crate::application::{ApplicationServices, CommandContext};
use crate::audio_tags::write_id3v24_edits;
use crate::db::{self, TrackRow};
use crate::feed_service::{self, track_row_to_track_context, StagedMusicBrainzLookup};
use crate::library_service;
use crate::media::{image_from_bytes, ImageCache};
use crate::metadata::{
    aligned_compare_rows, auto_populated_pending_id3_edits, display_metadata_value,
    expand_woar_metadata_rows, expanded_metadata_display_value, id3_frame_base,
    metadata_field_is_expandable, musicbrainz_release_option_label, musicbrainz_release_summary,
    musicbrainz_subtitle, pending_id3_conflict_descriptions, pending_id3_edits_for_apply,
    summarize_contributor_value, track_metadata_rows, AlignedCompareRow, MetadataColumn,
    MetadataGridRow, MusicBrainzLookupResult, PendingId3Edit, TagCompareResult, TrackContext,
};
use crate::musicbrainz::{LookupMetadata, MusicBrainzCandidate};
use crate::presentation::GpuiCommandRunner;
use crate::subscribe_service::{self, SubscribeTrackRequest};
use crate::ui::composites::{
    action_button, DetailGrid, DetailHeader, DetailRow as CompositeDetailRow, DisclosureGroup,
    EntityKind, ListRow, ProvenanceRole, SplitPane, StatusRole, TagBadge, Thumbnail, ThumbnailSize,
    TrackRow as TrackRowComposite,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::{Button as UiButton, Image as ImagePrimitive, Label, MultilineText};
use crate::ui::sizable_bridge::SizableScaled;
use crate::ui::tokens::{FontSize, Radius, SemanticColor};
use crate::ui_entity::{render_release_detail_shell, ReleaseDetailSlots, TrackSectionSlot};
use crate::view_models::entity_detail::{
    EntityActionKind, EntityActionTarget, EntityActionTone, EntitySurfaceContext,
    MetadataPanelState, ReleaseDetailVm, TrackMetadataActionState,
};
use crate::view_models::library::{
    AlbumNode, ArtistNode, FeedUpdatePhase, LibraryAlbumDetailVm, LibraryArtistDetailVm,
    LibraryTrackActionVm, LibraryTrackRowVm, LibraryTree, LibraryViewModel, MbStatusKind,
    MbTrackStatus, PlaylistAppendIntent, PlaylistAppendOutcome, PlaylistDetailVm,
    TrackSubscribeOutcome,
};
use crate::view_models::track::TrackVm;
use crate::views::{FeedView, TrackRef};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum LibraryDetail {
    None,
    Artist(LibraryArtistDetail),
    Album(AlbumNode),
    Track(Box<InspectorFrame>),
    Playlist(PlaylistDetail),
}

#[derive(Clone, Debug)]
struct LibraryArtistDetail {
    name: String,
    tracks: Vec<TrackRow>,
}

#[derive(Clone, Debug)]
struct PlaylistDetail {
    playlist: db::Playlist,
    tracks: Vec<TrackRow>,
}

#[derive(Clone, Debug)]
pub enum LibraryAppEvent {
    PlayPlaylistAt {
        playlist_id: i64,
        playlist_position: i64,
    },
}

impl gpui::EventEmitter<LibraryAppEvent> for LibraryApp {}

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
    application_services: Arc<ApplicationServices>,
    command_runner: GpuiCommandRunner,
    cache: Arc<ImageCache>,
    musicindex_endpoint: String,
    /// Stateful screen view-model. Owns all pure UI state and loaded
    /// snapshots — tree, selection, expansion sets, sort orders,
    /// picker toggles, status, search query, playlists, MusicBrainz
    /// lookup state, feed-update workflow state. The fields kept on
    /// `LibraryApp` itself are GPUI-bound (Entity / Subscription),
    /// service handles, screen-only inspector state, or maps that
    /// still hold `Arc<gpui::Image>`. See ADR 0023.
    vm: LibraryViewModel,
    detail: LibraryDetail,
    thumbnails: BTreeMap<(String, bool), ThumbnailState>,
    search_input: Entity<InputState>,
    _search_sub: gpui::Subscription,
    new_playlist_input: Entity<InputState>,
}

use crate::ui::icons::render_rss_icon_link;
use crate::ui::layouts as layout;
use crate::ui::style::color;
use crate::ui::style::radius;
use crate::ui::style::spacing;
use crate::ui::style::typography;

// ---------------------------------------------------------------------------
// LibraryApp
// ---------------------------------------------------------------------------

impl LibraryApp {
    pub fn new(
        conn: Arc<Mutex<Connection>>,
        cache: Arc<ImageCache>,
        musicindex_endpoint: String,
        application_services: Arc<ApplicationServices>,
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
        let command_runner = GpuiCommandRunner::new(
            application_services.command_bus(),
            application_services.event_bus(),
        );
        let mut app = Self {
            conn,
            application_services,
            command_runner,
            cache,
            musicindex_endpoint,
            vm: LibraryViewModel::new(),
            detail: LibraryDetail::None,
            thumbnails: BTreeMap::new(),
            search_input,
            _search_sub: search_sub,
            new_playlist_input,
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
        self.vm
            .apply_search_query(self.search_input.read(cx).value().to_string());
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
        if items.is_empty() {
            return;
        }
        let current_idx = items
            .iter()
            .position(|&id| Some(id) == self.vm.selected_id());
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
        if items.is_empty() {
            return;
        }
        let current_idx = items
            .iter()
            .position(|&id| Some(id) == self.vm.selected_id());
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
        self.vm.select_library_item(id);
        // ... need to find the item to know what detail to show ...
    }

    fn focusable_items(&self) -> Vec<i64> {
        // Traverse filtered_tree based on expanded states
        // This is tricky because filtered_tree is computed in render.
        // I should probably compute it in a separate method.
        Vec::new()
    }

    pub fn set_musicindex_endpoint(&mut self, endpoint: String, cx: &mut Context<Self>) {
        self.musicindex_endpoint = endpoint;
        cx.notify();
    }

    fn reload(&mut self) {
        let conn = self.conn.lock().expect("lock db");
        match library_service::library_tracks(&conn) {
            Ok(rows) => {
                let count = rows.len();
                self.vm.replace_tree(build_tree(&rows, &conn));
                self.vm.finish_library_reload(count);
            }
            Err(err) => {
                self.vm.set_error_status(err);
            }
        }
        drop(conn);
        self.reload_playlists();
        self.vm.clear_library_selection();
        self.detail = LibraryDetail::None;
        self.vm.clear_mb_status();
    }

    fn reload_playlists(&mut self) {
        let conn = self.conn.lock().expect("lock db");
        match self.application_services.query_service().playlists(&conn) {
            Ok(list) => self.vm.replace_playlists(list),
            Err(err) => self.vm.fail_playlist_load(err),
        }
    }

    fn cycle_playlist_sort(&mut self, cx: &mut Context<Self>) {
        self.vm.cycle_playlist_sort();
        self.vm.sort_loaded_playlists();
        cx.notify();
    }

    fn select_playlist(&mut self, id: i64, cx: &mut Context<Self>) {
        self.vm.select_playlist(id);
        let conn = self.conn.lock().expect("lock db");
        let playlist = self.vm.playlist_by_id(id);
        let tracks = self
            .application_services
            .query_service()
            .playlist_tracks(&conn, id)
            .unwrap_or_default();
        drop(conn);
        if let Some(playlist) = playlist {
            self.detail = LibraryDetail::Playlist(PlaylistDetail {
                playlist,
                tracks: tracks.clone(),
            });
            self.vm.replace_playlist_tracks(tracks);
        }
        cx.notify();
    }

    fn create_playlist(&mut self, cx: &mut Context<Self>) {
        let name = self.new_playlist_input.read(cx).value().to_string();
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        let command = CreatePlaylist::new(Arc::clone(&self.conn), name.to_string());
        self.command_runner.run(
            command,
            CommandContext::next(),
            cx,
            |this, result, cx| {
                this.vm.close_creating_playlist();
                this.reload_playlists();
                this.select_playlist(result.playlist_id(), cx);
            },
            |this, err, _cx| this.vm.fail_playlist_create(err),
        );
    }

    #[allow(dead_code)]
    fn rename_playlist(&mut self, id: i64, new_name: String, cx: &mut Context<Self>) {
        let trimmed = new_name.trim();
        if trimmed.is_empty() {
            return;
        }
        let command = RenamePlaylist::new(Arc::clone(&self.conn), id, trimmed.to_string());
        self.command_runner.run(
            command,
            CommandContext::next(),
            cx,
            move |this, (), cx| {
                this.reload_playlists();
                if this.vm.is_playlist_selected(id) {
                    this.select_playlist(id, cx);
                }
            },
            |this, err, _cx| this.vm.fail_playlist_rename(err),
        );
    }

    fn delete_playlist(&mut self, id: i64, cx: &mut Context<Self>) {
        let command = DeletePlaylist::new(Arc::clone(&self.conn), id);
        self.command_runner.run(
            command,
            CommandContext::next(),
            cx,
            move |this, (), _cx| {
                if this.vm.clear_playlist_selection_if(id) {
                    this.detail = LibraryDetail::None;
                }
                this.reload_playlists();
            },
            |this, err, _cx| this.vm.fail_playlist_delete(err),
        );
    }

    fn remove_playlist_track_at(
        &mut self,
        playlist_id: i64,
        position: i64,
        cx: &mut Context<Self>,
    ) {
        let command = RemovePlaylistTrackAt::new(Arc::clone(&self.conn), playlist_id, position);
        self.command_runner.run(
            command,
            CommandContext::next(),
            cx,
            move |this, (), cx| {
                this.reload_playlists();
                if this.vm.is_playlist_selected(playlist_id) {
                    this.select_playlist(playlist_id, cx);
                }
            },
            |this, err, _cx| this.vm.fail_playlist_track_remove(err),
        );
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
        let command = ReorderPlaylistTrack::new(Arc::clone(&self.conn), playlist_id, from, to);
        self.command_runner.run(
            command,
            CommandContext::next(),
            cx,
            move |this, (), cx| {
                if this.vm.is_playlist_selected(playlist_id) {
                    this.select_playlist(playlist_id, cx);
                }
            },
            |this, err, _cx| this.vm.fail_playlist_track_reorder(err),
        );
    }

    fn add_track_to_playlist(&mut self, track_id: i64, playlist_id: i64, cx: &mut Context<Self>) {
        if let Some(intent) = self.vm.begin_playlist_append(playlist_id, vec![track_id]) {
            self.spawn_subscribe_then_append(intent, cx);
        }
    }

    fn add_album_to_playlist(&mut self, feed_id: i64, playlist_id: i64, cx: &mut Context<Self>) {
        let conn = self.conn.lock().expect("lock db");
        let tracks = match db::feed_tracks(&conn, feed_id) {
            Ok(t) => t,
            Err(err) => {
                self.vm.fail_album_tracks_load(err);
                cx.notify();
                return;
            }
        };
        drop(conn);
        let track_ids: Vec<i64> = tracks.iter().map(|t| t.id).collect();
        if track_ids.is_empty() {
            self.vm.set_album_has_no_tracks();
            cx.notify();
            return;
        }
        if let Some(intent) = self.vm.begin_playlist_append(playlist_id, track_ids) {
            self.spawn_subscribe_then_append(intent, cx);
        }
    }

    fn spawn_subscribe_then_append(
        &mut self,
        intent: PlaylistAppendIntent,
        cx: &mut Context<Self>,
    ) {
        let playlist_id = intent.playlist_id();
        let track_ids = intent.track_ids().to_vec();
        cx.notify();

        let command = SubscribeThenAppendToPlaylist::new(
            Arc::clone(&self.conn),
            self.application_services.download_manager(),
            playlist_id,
            track_ids,
        );
        self.command_runner.run(
            command,
            CommandContext::next(),
            cx,
            move |this, outcome, _cx| {
                this.vm.finish_playlist_append(
                    &intent,
                    PlaylistAppendOutcome::new(
                        outcome.appended(),
                        outcome.downloaded(),
                        outcome.failed().len(),
                    ),
                );
                this.reload_playlists();
            },
            |this, err, _cx| {
                this.vm.fail_playlist_append(err);
                this.reload_playlists();
            },
        );
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
        if self.vm.set_hovered_thumb_url(url) {
            cx.notify();
        }
    }

    fn select_album(&mut self, album: &AlbumNode, cx: &mut Context<Self>) {
        if let Some(feed_id) = album.feed_id {
            self.vm.select_library_item(feed_id);
        } else {
            self.vm.clear_library_selection();
        }
        self.detail = LibraryDetail::Album(album.clone());
        if let Some(feed_id) = album.feed_id {
            self.check_feed_on_view(feed_id, cx);
        }
    }

    fn select_artist(&mut self, name: &str, _cx: &mut Context<Self>) {
        // Find all tracks matching this artist name
        let mut tracks = Vec::new();
        for artist_node in &self.vm.tree().artists {
            if artist_node.name == name {
                for album in &artist_node.albums {
                    tracks.extend(album.tracks.clone());
                }
                break;
            }
        }
        self.vm.clear_library_selection();
        self.detail = LibraryDetail::Artist(LibraryArtistDetail {
            name: name.to_string(),
            tracks,
        });
    }

    fn check_feed_on_view(&mut self, feed_id: i64, cx: &mut Context<Self>) {
        if !self.vm.begin_feed_view_check(feed_id) {
            return;
        }
        let command = CheckFeedStaleness::new(
            Arc::clone(&self.conn),
            self.musicindex_endpoint.clone(),
            feed_id,
        );
        cx.notify();
        self.command_runner.run(
            command,
            CommandContext::next(),
            cx,
            move |this, outcome, _cx| {
                let checked_feed_id = outcome.feed_id();
                this.vm
                    .finish_feed_view_check(checked_feed_id, Ok(outcome.into_stale()));
            },
            move |this, error, _cx| {
                this.vm
                    .finish_feed_view_check(feed_id, Err(format!("{error:#}")));
            },
        );
    }

    fn check_all_feeds(&mut self, cx: &mut Context<Self>) {
        if self.vm.feed_update_state().phase != FeedUpdatePhase::Idle {
            return;
        }
        let feeds = {
            let conn = match self.conn.lock() {
                Ok(conn) => conn,
                Err(_) => {
                    self.vm.set_feed_check_error("database lock poisoned");
                    cx.notify();
                    return;
                }
            };
            match self
                .application_services
                .query_service()
                .subscribed_feeds_for_stale_check(&conn)
            {
                Ok(rows) => rows,
                Err(err) => {
                    self.vm.set_feed_check_error(format!("{err:#}"));
                    cx.notify();
                    return;
                }
            }
        };
        if feeds.is_empty() {
            self.vm.set_no_subscribed_feeds();
            cx.notify();
            return;
        }
        self.vm.begin_all_feed_check(feeds.len());
        cx.notify();

        let command = CheckSubscribedFeeds::new(
            Arc::clone(&self.conn),
            self.musicindex_endpoint.clone(),
            feeds,
        );
        self.command_runner.run(
            command,
            CommandContext::next(),
            cx,
            |this, outcome, _cx| {
                this.vm.finish_all_feed_check(outcome.into_stale());
            },
            |this, error, _cx| {
                this.vm.set_feed_check_error(format!("{error:#}"));
            },
        );
    }

    fn apply_all_feed_updates(&mut self, cx: &mut Context<Self>) {
        let Some(stale) = self.vm.begin_apply_feed_updates() else {
            return;
        };
        cx.notify();

        let command = ApplyFeedUpdates::new(
            Arc::clone(&self.conn),
            self.musicindex_endpoint.clone(),
            stale,
        );
        self.command_runner.run(
            command,
            CommandContext::next(),
            cx,
            |this, outcome, _cx| {
                this.vm
                    .finish_apply_feed_updates(outcome.message().to_string());
            },
            |this, error, _cx| {
                this.vm
                    .finish_apply_feed_updates(format!("Feed update error: {error:#}"));
            },
        );
    }

    fn select_track(&mut self, track: &TrackRow, cx: &mut Context<Self>) {
        self.vm.select_library_item(track.id);
        let image = track
            .track_image_href
            .as_deref()
            .or(track.album_image_href.as_deref())
            .and_then(|url| self.thumbnail_for_url(Some(url), true, cx));
        let mut frame = InspectorFrame::for_track(track.clone(), image);
        if let Some(lookup) = self.vm.staged_musicbrainz(track.id).cloned() {
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
                    .spawn(async move {
                        feed_service::fetch_library_track_context(&track, &musicindex_endpoint)
                    })
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
        self.vm.toggle_artist(name);
    }

    fn toggle_album(&mut self, artist: &str, album: &str) {
        self.vm.toggle_album(artist, album);
    }

    fn unsubscribe_feed(&mut self, feed_id: i64, cx: &mut Context<Self>) {
        let command = UnsubscribeFeedById::new(Arc::clone(&self.conn), feed_id);
        self.command_runner.run(
            command,
            CommandContext::next(),
            cx,
            |this, _result, _cx| this.reload(),
            |this, err, _cx| this.vm.set_error_status(err),
        );
    }

    fn remove_track(&mut self, track_id: i64, cx: &mut Context<Self>) {
        let command = RemoveTrackFromLibrary::new(Arc::clone(&self.conn), track_id);
        self.command_runner.run(
            command,
            CommandContext::next(),
            cx,
            |this, _result, _cx| this.reload(),
            |this, err, _cx| this.vm.set_error_status(err),
        );
    }

    fn subscribe_track(&mut self, track: TrackRow, cx: &mut Context<Self>) {
        if self.vm.has_busy_track() {
            return;
        }
        let track_id = track.id;
        self.vm.begin_busy_track(track_id, "Subscribing track...");
        cx.notify();

        let command = SubscribeTrack::new(
            Arc::clone(&self.conn),
            self.application_services.download_manager(),
            SubscribeTrackRequest::LibraryTrack {
                track: Box::new(track),
            },
            "Downloaded track",
        );
        self.command_runner.run(
            command,
            CommandContext::next(),
            cx,
            |this, outcome, _cx| {
                this.vm.finish_track_subscribe(TrackSubscribeOutcome::new(
                    outcome.path().to_string(),
                    outcome.format_warning().map(str::to_string),
                ));
                this.reload();
            },
            |this, error, _cx| this.vm.fail_track_subscribe(error),
        );
    }

    fn selected_track_frame_mut(&mut self) -> Option<&mut InspectorFrame> {
        match &mut self.detail {
            LibraryDetail::Track(frame) => Some(frame),
            LibraryDetail::None
            | LibraryDetail::Artist(_)
            | LibraryDetail::Album(_)
            | LibraryDetail::Playlist(_) => None,
        }
    }

    fn stage_musicbrainz_lookup_for_track(
        &mut self,
        track_id: i64,
        lookup: MusicBrainzLookupResult,
    ) {
        self.vm.stage_musicbrainz(track_id, lookup.clone());
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
                        subscribe_service::compare_downloaded_track_path(&path, &track_context)
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

        let command = SetTrackLibraryMembership::new(Arc::clone(&self.conn), track_id, subscribe);
        self.command_runner.run(
            command,
            CommandContext::next(),
            cx,
            move |this, result, _cx| {
                if let Some(frame) = this.selected_track_frame_mut() {
                    if frame.entity_id == track_id {
                        frame.subscription_busy = false;
                        frame.local_subscription = result.in_library();
                        frame.track.is_in_library = result.in_library();
                        frame.subscription_message = Some(result.message().into());
                    }
                }
            },
            move |this, err, _cx| {
                if let Some(frame) = this.selected_track_frame_mut() {
                    if frame.entity_id == track_id {
                        frame.subscription_busy = false;
                        let action = if subscribe {
                            "Subscribe"
                        } else {
                            "Unsubscribe"
                        };
                        frame.subscription_message = Some(format!("{action} error: {err:#}"));
                    }
                }
            },
        );
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
        cx.notify();

        self.command_runner.run(
            LookupMusicBrainzTrack::new(track),
            CommandContext::next(),
            cx,
            move |this, result, _cx| {
                if let Some(frame) = this.selected_track_frame_mut() {
                    if frame.entity_id == entity_id {
                        frame.musicbrainz_selected = 0;
                        frame.musicbrainz_lookup = LazyPanel::Loaded(result);
                    }
                }
            },
            move |this, error, _cx| {
                if let Some(frame) = this.selected_track_frame_mut() {
                    if frame.entity_id == entity_id {
                        frame.musicbrainz_lookup = LazyPanel::Empty(format!("Error: {error}"));
                    }
                }
            },
        );
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
        if !self.vm.begin_musicbrainz_track_lookup(track.id) {
            return;
        }
        cx.notify();

        let track_id = track.id;
        self.command_runner.run(
            StageMusicBrainzTrack::new(track),
            CommandContext::next(),
            cx,
            move |this, staged, _cx| {
                let n = staged.edit_count;
                this.stage_musicbrainz_lookup_for_track(track_id, staged.lookup);
                this.vm.finish_musicbrainz_track_lookup(track_id, n);
            },
            move |this, error, _cx| {
                this.vm.fail_musicbrainz_track_lookup(track_id, error);
            },
        );
    }

    fn musicbrainz_feed(&mut self, album: AlbumNode, cx: &mut Context<Self>) {
        let downloadable: Vec<TrackRow> = album
            .tracks
            .into_iter()
            .filter(|t| t.local_path.is_some())
            .collect();
        if !self
            .vm
            .begin_musicbrainz_album_lookup(downloadable.iter().map(|track| track.id))
        {
            cx.notify();
            return;
        }
        cx.notify();

        let conn = Arc::clone(&self.conn);
        let feed_id = album.feed_id.unwrap_or(0);
        let feed_title = Some(album.name.clone());
        let total_count = downloadable.len();
        let command_bus = self.application_services.command_bus();
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
                        let outcome = command_bus.execute(
                            LookupMusicBrainzAlbumReleases::new(meta_clone, 3),
                            &CommandContext::next(),
                        )?;
                        Ok::<_, crate::application::CommandError>(outcome.into_parts().0)
                    })
                    .await;

                let candidates = match release_candidates {
                    Ok(c) => c,
                    Err(err) => {
                        // Fall back to per-track recording search.
                        this.update(
                            cx,
                            move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                                this.vm.fail_musicbrainz_album_lookup_with_fallback(err);
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
                            this.vm.fallback_empty_musicbrainz_album_lookup();
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
                            this.vm.begin_musicbrainz_album_track_stage(
                                track_id,
                                progress,
                                total_count,
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
                            this.vm
                                .finish_musicbrainz_album_track_stage(track_id, status_clone);
                            cx.notify();
                        },
                    )
                    .ok();
                }

                this.update(
                    cx,
                    move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                        this.vm
                            .finish_musicbrainz_album_lookup(total_edits, processed);
                        cx.notify();
                    },
                )
                .ok();
            },
        )
        .detach();
    }
}

#[allow(dead_code)]
fn lookup_musicbrainz_stage_for_track(
    _conn: Arc<Mutex<Connection>>,
    track: &TrackRow,
) -> anyhow::Result<StagedMusicBrainzLookup> {
    let outcome = crate::application::CommandBus::new()
        .execute(
            StageMusicBrainzTrack::new(track.clone()),
            &CommandContext::next(),
        )
        .map_err(|error| anyhow::anyhow!("{error:#}"))?;
    Ok(outcome.into_parts().0)
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
    let outcome = crate::application::CommandBus::new()
        .execute(
            StageMusicBrainzCandidate::new(track.clone(), candidate.clone()),
            &CommandContext::next(),
        )
        .map_err(|error| anyhow::anyhow!("{error:#}"))?;
    Ok(outcome.into_parts().0)
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
                this.vm
                    .begin_musicbrainz_album_track_stage(track_id, progress, total_count);
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
                this.vm
                    .finish_musicbrainz_album_track_stage(track_id, status_clone);
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
            this.vm
                .finish_musicbrainz_album_lookup(total_edits, processed);
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

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

impl Render for LibraryApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status_text = self.vm.status().to_string();
        let status_color = if status_text.starts_with("Error:") {
            StatusRole::Danger.color(cx)
        } else {
            color::text_muted()
        };

        // Collect image URLs from tree, then fetch thumbnails (avoids borrow conflict).
        let urls: Vec<String> = {
            self.vm
                .tree()
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
        let hovered_url = self.vm.hovered_thumb_url().map(str::to_string);
        let mut album_thumbs: BTreeMap<String, Option<Arc<Image>>> = BTreeMap::new();
        for url in &urls {
            if !album_thumbs.contains_key(url.as_str()) {
                let animated = hovered_url.as_deref() == Some(url.as_str());
                let img = self.thumbnail_for_url(Some(url), animated, cx);
                album_thumbs.insert(url.clone(), img);
            }
        }

        let tree_projection = self.vm.tree_projection();
        let tree_items: Vec<AnyElement> = render_tree(
            &tree_projection.tree,
            &tree_projection.expanded_artists,
            &tree_projection.expanded_albums,
            self.vm.selected_id(),
            &album_thumbs,
            cx,
        );
        let filtered_empty = tree_projection.is_empty();

        let playlist_sidebar = self.vm.playlist_sidebar();
        let mut left_items: Vec<AnyElement> = Vec::new();

        left_items.push(
            div()
                .id("playlists-header")
                .px(spacing::SM)
                .py(spacing::XS)
                .rounded(spacing::XS)
                .cursor_pointer()
                .hover(|el| el.bg(color::bg_surface_hi()))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.vm.toggle_playlists_expanded();
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
                                .child(SharedString::from(playlist_sidebar.disclosure_glyph)),
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
                            UiButton::styled("playlists-sort", ControlStyle::ToolbarIcon)
                                .label(playlist_sidebar.sort_label)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.cycle_playlist_sort(cx);
                                })),
                        )
                        .child(
                            UiButton::styled("playlists-add", ControlStyle::ToolbarIcon)
                                .label("+")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.vm.toggle_creating_playlist();
                                    cx.notify();
                                })),
                        ),
                )
                .into_any_element(),
        );

        if playlist_sidebar.expanded {
            for row in &playlist_sidebar.rows {
                let playlist_id = row.id;
                left_items.push(
                    ListRow::compact(SharedString::from(row.element_id.clone()))
                        .selected(row.selected)
                        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                            this.select_playlist(playlist_id, cx);
                        }))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .pl(spacing::MD)
                                .child(
                                    Label::new(row.name.clone())
                                        .color(if row.selected {
                                            SemanticColor::Accent
                                        } else {
                                            SemanticColor::Label
                                        })
                                        .truncated(),
                                )
                                .child(
                                    Label::new(row.track_count_label.clone())
                                        .size(FontSize::Caption)
                                        .color(SemanticColor::TertiaryLabel),
                                ),
                        )
                        .into_any_element(),
                );
            }

            if playlist_sidebar.creating_playlist {
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
                                .scaled(Size::Small, cx),
                        )
                        .child(
                            UiButton::styled("playlist-add-btn", ControlStyle::Primary)
                                .label("Add")
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
            self.vm.busy_track(),
            self.vm.mb_status(),
            &album_thumbs,
            self.vm.playlists(),
            self.vm.album_feed_picker_open(),
            self.vm.album_track_picker_open(),
            cx,
        );

        let leading_pane = div()
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
                            .scaled(Size::Small, cx),
                    )
                    .child(
                        UiButton::styled("lib-search-btn", ControlStyle::Primary)
                            .label("Search")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.apply_search(cx);
                            })),
                    ),
            )
            .child({
                let feed_update = self.vm.feed_update_state();
                let has_stale = !feed_update.stale.is_empty();
                let phase = feed_update.phase.clone();
                let stale_count = feed_update.stale.len();
                let feed_status = feed_update.status_message.clone();
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
                        UiButton::styled("apply-feed-updates", ControlStyle::Primary)
                            .label(format!("Apply updates ({stale_count})"))
                            .disabled(phase != FeedUpdatePhase::Idle)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.apply_all_feed_updates(cx);
                            }))
                    } else {
                        UiButton::styled("check-all-feeds", ControlStyle::Secondary)
                            .label(if phase == FeedUpdatePhase::Checking {
                                "Checking..."
                            } else {
                                "Check all feeds"
                            })
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
                                filtered_empty && !self.vm.status().starts_with("Error:"),
                                |el| {
                                    el.child(
                                        div()
                                            .text_center()
                                            .p(spacing::XXL + spacing::LG)
                                            .text_color(color::text_muted())
                                            .child(
                                                div()
                                                    .mt(spacing::SM)
                                                    .child("No library tracks yet"),
                                            ),
                                    )
                                },
                            ),
                    ),
            )
            .into_any_element();

        let trailing_pane = div().child(detail_pane).into_any_element();
        let split_pane = SplitPane::new("library-pane-container")
            .resize_handle_id("library-resize-handle")
            .leading_width(px(self.vm.split_pane_width()))
            .leading_min_width(layout::INSPECTOR_MIN_WIDTH)
            .leading(leading_pane)
            .trailing(trailing_pane)
            .on_resize_move(cx.listener(|this, event: &MouseMoveEvent, _window, cx| {
                if this.vm.is_resizing() {
                    this.vm.resize_split_pane(
                        f32::from(event.position.x),
                        f32::from(layout::INSPECTOR_MIN_WIDTH),
                        f32::from(layout::INSPECTOR_MAX_WIDTH),
                    );
                    cx.notify();
                }
            }))
            .on_resize_end(cx.listener(|this, _: &MouseUpEvent, _window, cx| {
                if this.vm.is_resizing() {
                    this.vm.end_resize();
                    cx.notify();
                }
            }))
            .on_resize_start(cx.listener(|this, _: &MouseDownEvent, _window, cx| {
                this.vm.begin_resize();
                cx.notify();
            }));

        div()
            .size_full()
            .bg(color::bg_canvas())
            .text_color(color::text_primary())
            .text_sm()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(split_pane)
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
                    this.select_artist(&artist_name, cx);
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
                        .child(div().text_xs().text_color(color::text_muted()).child(
                            SharedString::from(format!(
                                "({album_count} album{})",
                                if album_count == 1 { "" } else { "s" }
                            )),
                        )),
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
                                    thumb_image.clone(),
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
                            .when(is_selected, |el| {
                                el.border_l_2().border_color(color::accent())
                            })
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
                                    .child(render_album_thumb(track_thumb_image.clone(), 24.0))
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

#[expect(
    clippy::too_many_arguments,
    reason = "library detail rendering still passes explicit staged UI state"
)]
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

        LibraryDetail::Artist(artist) => {
            render_library_artist_detail(artist, album_thumbs, playlists, cx)
        }

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

fn render_library_artist_detail(
    detail: &LibraryArtistDetail,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    _playlists: &[db::Playlist],
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let vm = LibraryArtistDetailVm::new(&detail.name, &detail.tracks);

    let feed_rows: Vec<AnyElement> = vm
        .feed_summaries()
        .into_iter()
        .map(|summary| {
            let thumb_image = summary
                .thumb_url
                .as_ref()
                .and_then(|url| album_thumbs.get(url.as_str()))
                .and_then(|opt| opt.clone());
            let feed_name_for_click = summary.feed_name.clone();

            div()
                .id(SharedString::from(format!(
                    "artist-feed-{}",
                    summary.feed_name
                )))
                .flex()
                .flex_row()
                .items_center()
                .gap(spacing::SM)
                .px(spacing::SM)
                .py(spacing::XS)
                .rounded(radius::SM)
                .hover(|el| el.bg(color::bg_surface_hi()))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| {
                    let feed_name_to_match = feed_name_for_click.clone();
                    let tree_artists = this.vm.tree().artists.clone();
                    for artist_node in &tree_artists {
                        for album in &artist_node.albums {
                            if album.name == feed_name_to_match {
                                this.select_album(album, cx);
                                cx.notify();
                                return;
                            }
                        }
                    }
                }))
                .child(Thumbnail::new(EntityKind::Feed, ThumbnailSize::Sm).image(thumb_image))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(spacing::XXS)
                        .child(
                            div()
                                .text_size(typography::SIZE_MICRO)
                                .font_weight(FontWeight::MEDIUM)
                                .truncate()
                                .child(SharedString::from(summary.feed_name.clone())),
                        )
                        .child(div().text_xs().text_color(color::text_muted()).child(
                            SharedString::from(format!("{} tracks", summary.track_count)),
                        )),
                )
                .into_any_element()
        })
        .collect();

    div()
        .id("artist-detail-scroll")
        .size_full()
        .overflow_y_scroll()
        .p(spacing::LG)
        .flex()
        .flex_col()
        .gap(spacing::LG)
        .child(DetailHeader::new(
            EntityKind::Artist,
            vm.artist_name_or_unknown(),
        ))
        .child(DetailGrid::new(
            vm.detail_rows()
                .into_iter()
                .map(|(k, v)| CompositeDetailRow::text(k, v, 6))
                .collect::<Vec<_>>(),
        ))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(spacing::XXS)
                .children(feed_rows),
        )
        .into_any_element()
}

#[expect(
    clippy::too_many_arguments,
    reason = "album detail rendering still threads explicit UI controls during rollout"
)]
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
    // Build FeedView from local album data via synthesized FeedRow
    let feed_row = db::FeedRow {
        id: album.feed_id.unwrap_or(0),
        feed_url: album.feed_url.clone().unwrap_or_default(),
        feed_guid: None,
        title: Some(album.name.clone()),
        description: None,
        album_image_href: album.image_href.clone(),
        is_subscribed: false, // Library view, not used
    };
    let feed_view = FeedView::from_local(feed_row, album.tracks.clone());

    let thumb_image = album
        .image_href
        .as_ref()
        .and_then(|url| album_thumbs.get(url.as_str()))
        .and_then(|opt| opt.clone());

    // Render track rows with library-specific affordances
    let track_rows: Vec<AnyElement> = album
        .tracks
        .iter()
        .map(|track| {
            render_library_track_row(
                track,
                mb_status,
                busy_track,
                add_open_track,
                album_thumbs,
                playlists,
                cx,
            )
        })
        .collect();

    let vm = LibraryAlbumDetailVm::new(&feed_view, &album.tracks, mb_status);

    // Library-specific action buttons
    let album_for_mb = album.clone();
    let feed_url = album.feed_url.clone();
    let feed_id = album.feed_id;
    let mut buttons = div().flex().flex_row().items_center().gap(spacing::SM);
    buttons = buttons.child(render_rss_icon_link(
        &format!("album-{}", album.feed_id.unwrap_or(0)),
        feed_url,
    ));
    if let Some(fid) = feed_id {
        let remove_action = vm.primary_action_vm(fid, false);
        let remove_label = remove_action.label;
        buttons = buttons.child(action_button(&remove_label, cx).on_click(cx.listener(
            move |this, _, _, cx| {
                this.unsubscribe_feed(fid, cx);
                cx.notify();
            },
        )));
    }
    buttons = buttons.child(
        action_button("MusicBrainz", cx)
            .disabled(vm.has_active_musicbrainz())
            .on_click(cx.listener(move |this, _, _, cx| {
                this.musicbrainz_feed(album_for_mb.clone(), cx);
            })),
    );
    if let Some(fid) = feed_id {
        let playlist_action = vm
            .playlist_action_vm(fid, add_open_feed)
            .expect("library feed playlist action should render for local feeds");
        let playlist_label = playlist_action.label;
        buttons = buttons.child(action_button(&playlist_label, cx).on_click(cx.listener(
            move |this, _, _, cx| {
                this.vm.toggle_album_feed_picker();
                let _ = fid;
                cx.notify();
            },
        )));
    }

    let feed_popup: Option<AnyElement> = if add_open_feed {
        feed_id.map(|fid| render_album_feed_add_panel(fid, playlists, cx))
    } else {
        None
    };

    let track_count = vm.track_count();
    let track_summary = vm.total_duration_label().map_or_else(
        || format!("{track_count} total"),
        |duration| format!("{track_count} total · {duration}"),
    );

    let projection = ReleaseDetailVm::new(&feed_view, EntitySurfaceContext::Library);
    let mut slots = ReleaseDetailSlots {
        header: Some(
            DetailHeader::new(EntityKind::Feed, vm.title())
                .subtitle(vm.artist())
                .image(thumb_image.clone())
                .into_any_element(),
        ),
        action_row: Some(buttons.into_any_element()),
        details: Some(
            DetailGrid::new(
                vm.detail_rows()
                    .into_iter()
                    .map(|(k, v)| CompositeDetailRow::text(k, v, 6))
                    .collect::<Vec<_>>(),
            )
            .into_any_element(),
        ),
        track_section: Some(TrackSectionSlot {
            summary: track_summary.into(),
            rows: track_rows,
        }),
        ..ReleaseDetailSlots::default()
    };
    if let Some(panel) = feed_popup {
        slots.panels.push(panel);
    }
    render_release_detail_shell("album-detail-scroll", &projection, slots)
}

fn render_library_track_row(
    track: &TrackRow,
    mb_status: &BTreeMap<i64, MbTrackStatus>,
    busy_track: Option<i64>,
    add_open_track: Option<i64>,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    playlists: &[db::Playlist],
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let track_for_click = track.clone();
    let track_for_select = track.clone();
    let track_id = track.id;
    let is_busy = busy_track == Some(track_id);
    let popup_open = add_open_track == Some(track_id);
    let vm = LibraryTrackRowVm::new(track, mb_status.get(&track_id));
    let primary_action = vm.primary_action_vm(is_busy);
    let in_library = primary_action.kind == EntityActionKind::Remove;
    let mb_text = vm.mb_status_text();
    let mb_kind = vm.mb_status_kind();
    let thumbnail = track
        .track_image_href
        .as_ref()
        .or(track.album_image_href.as_ref())
        .and_then(|url| album_thumbs.get(url.as_str()))
        .and_then(|opt| opt.clone());
    let primary_style = match primary_action.tone {
        EntityActionTone::DestructiveQuiet => ControlStyle::DestructiveRowAction,
        _ => ControlStyle::RowAction,
    };

    let toggle_button = UiButton::styled(
        SharedString::from(format!("lib-toggle-{track_id}")),
        primary_style,
    )
    .label(primary_action.label.clone())
    .disabled(!primary_action.enabled)
    .on_click(cx.listener(move |this, _, _, cx| {
        if in_library {
            this.remove_track(track_id, cx);
        } else {
            this.subscribe_track(track_for_click.clone(), cx);
        }
        cx.notify();
    }));

    let mut row = TrackRowComposite::new(SharedString::from(format!("album-track-{track_id}")))
        .number(vm.number_label())
        .title(vm.title())
        .duration(vm.duration_display())
        .thumbnail(thumbnail)
        .on_click(cx.listener(move |this, _, _, cx| {
            this.select_track(&track_for_select, cx);
            cx.notify();
        }))
        .trailing_child(toggle_button);

    if let Some(text) = mb_text {
        let status_color = match mb_kind {
            Some(MbStatusKind::Success) => StatusRole::Success.color(cx),
            Some(MbStatusKind::Danger) => StatusRole::Danger.color(cx),
            Some(MbStatusKind::Warning) => StatusRole::Warning.color(cx),
            _ => color::text_muted(),
        };
        row = row.trailing_child(
            div()
                .text_size(typography::SIZE_MICRO)
                .text_color(status_color)
                .child(SharedString::from(text.to_string())),
        );
    }

    row = row.trailing_child(
        UiButton::styled(
            SharedString::from(format!("album-track-add-{track_id}")),
            ControlStyle::RowAction,
        )
        .label(vm.playlist_action_label(popup_open))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.vm.toggle_album_track_picker(track_id);
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
        panel = panel.child(action_button(&label, cx).on_click(cx.listener(
            move |this, _, _, cx| {
                this.vm.close_album_track_picker();
                this.add_track_to_playlist(track_id, playlist_id, cx);
            },
        )));
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
        panel = panel.child(action_button(&label, cx).on_click(cx.listener(
            move |this, _, _, cx| {
                this.vm.close_album_feed_picker();
                this.add_album_to_playlist(feed_id, playlist_id, cx);
            },
        )));
    }

    panel.into_any_element()
}

fn render_playlist_detail(
    detail: &PlaylistDetail,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let vm = PlaylistDetailVm::new(&detail.playlist, &detail.tracks);
    let playlist_id = vm.playlist_id();
    let playlist_name = vm.name().to_string();

    let track_rows: Vec<AnyElement> = if vm.is_empty() {
        vec![div()
            .text_center()
            .p(spacing::XXL)
            .text_color(color::text_muted())
            .child(vm.empty_message())
            .into_any_element()]
    } else {
        vm.track_rows()
            .into_iter()
            .map(|row| {
                let track_for_select = row.track().clone();
                let track_id = row.track_id();
                let position = row.position() as i64;
                let pl_id = playlist_id;
                let can_play = row.can_play();
                let track_title = row.title();
                let artist = row.artist();
                let dur = row.duration_label();
                let position_label = row.position_label();
                let track_thumb_image = row
                    .thumb_url()
                    .and_then(|url| album_thumbs.get(url))
                    .and_then(|opt| opt.clone());

                let up_btn = UiButton::styled(
                    SharedString::from(format!("playlist-up-{pl_id}-{position}")),
                    ControlStyle::RowAction,
                )
                .label("▲")
                .disabled(!row.can_move_up())
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.move_playlist_track(pl_id, position, position - 1, cx);
                }));

                let down_btn = UiButton::styled(
                    SharedString::from(format!("playlist-down-{pl_id}-{position}")),
                    ControlStyle::RowAction,
                )
                .label("▼")
                .disabled(!row.can_move_down())
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.move_playlist_track(pl_id, position, position + 1, cx);
                }));

                let remove_btn = UiButton::styled(
                    SharedString::from(format!("playlist-remove-{pl_id}-{position}")),
                    ControlStyle::Destructive,
                )
                .label("✕")
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.remove_playlist_track_at(pl_id, position, cx);
                }));

                let play_btn = UiButton::styled(
                    SharedString::from(format!("playlist-play-{pl_id}-{position}")),
                    ControlStyle::RowAction,
                )
                .label("▶")
                .disabled(!can_play)
                .on_click(cx.listener(move |_this, _, _, cx| {
                    cx.emit(LibraryAppEvent::PlayPlaylistAt {
                        playlist_id: pl_id,
                        playlist_position: position,
                    });
                }));

                div()
                    .id(SharedString::from(format!(
                        "playlist-track-{track_id}-{position}"
                    )))
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
                                    .w(layout::PLAYLIST_THUMB_SLOT)
                                    .text_xs()
                                    .text_color(color::text_muted())
                                    .child(SharedString::from(position_label)),
                            )
                            .child(render_album_thumb(track_thumb_image.clone(), 24.0))
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
                                    .w(layout::PLAYLIST_TITLE_OFFSET)
                                    .child(SharedString::from(dur)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(spacing::XS)
                            .child(play_btn)
                            .child(up_btn)
                            .child(down_btn)
                            .child(remove_btn),
                    )
                    .into_any_element()
            })
            .collect()
    };

    let detail_rows = vm.detail_rows();

    let mut buttons = div().flex().flex_row().items_center().gap(spacing::SM);
    let playlist_for_rename = playlist_id;
    buttons = buttons.child(
        UiButton::styled(
            SharedString::from(format!("playlist-rename-{playlist_id}")),
            ControlStyle::Ghost,
        )
        .label("Rename")
        .on_click(cx.listener(move |_this, _, _, cx| {
            // TODO Stage 3: implement inline rename modal/input
            cx.notify();
        })),
    );
    buttons = buttons.child(
        UiButton::styled(
            SharedString::from(format!("playlist-delete-{playlist_id}")),
            ControlStyle::Destructive,
        )
        .label("Delete")
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
        .child(DetailHeader::new(EntityKind::Playlist, &playlist_name))
        .child(DetailGrid::new(
            detail_rows
                .into_iter()
                .map(|(k, v)| CompositeDetailRow::text(k, v, 6))
                .collect::<Vec<_>>(),
        ))
        .child(buttons)
        .child(
            div()
                .flex()
                .flex_col()
                .gap(spacing::XXS)
                .children(track_rows),
        )
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
    let metadata_state = track_metadata_action_state(frame);
    let show_id3_panel = metadata_state.show_compare_panel();
    let show_musicbrainz_panel = metadata_state.show_musicbrainz_panel();
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
            result.and_then(|r| {
                r.file_image
                    .as_ref()
                    .map(|img| image_from_bytes(img.clone()))
            }),
            result
                .and_then(|r| r.format)
                .map(|f| f.display_label())
                .unwrap_or("Tags"),
            cx,
        ))
        .into_any_element()
}

fn track_metadata_action_state(frame: &InspectorFrame) -> TrackMetadataActionState {
    TrackMetadataActionState::new(
        metadata_panel_state(&frame.tag_compare),
        metadata_panel_state(&frame.musicbrainz_lookup),
        frame.track.local_path.is_some(),
    )
}

fn metadata_panel_state<T>(panel: &LazyPanel<T>) -> MetadataPanelState {
    match panel {
        LazyPanel::Hidden => MetadataPanelState::Hidden,
        LazyPanel::Loading => MetadataPanelState::Loading,
        LazyPanel::Loaded(_) => MetadataPanelState::Loaded,
        LazyPanel::Empty(_) => MetadataPanelState::Empty,
    }
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
        TrackVm::new(track).title()
    } else {
        frame.title.clone()
    };
    let artist = track
        .track_artist
        .clone()
        .or_else(|| track.release_artist.clone())
        .unwrap_or_else(|| "Unknown".into());
    DetailHeader::new(EntityKind::Track, title)
        .subtitle(artist.as_str().to_string())
        .image(frame.image.clone())
        .into_any_element()
}

fn render_action_row(
    frame: &InspectorFrame,
    pending_id3_edits: &BTreeMap<String, PendingId3Edit>,
    playlists: &[db::Playlist],
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let pending_conflicts = pending_id3_conflict_descriptions(pending_id3_edits);
    let has_pending_conflicts = !pending_conflicts.is_empty();
    let metadata_state = track_metadata_action_state(frame);
    let metadata_target = EntityActionTarget::Track(TrackRef::LocalTrackId(frame.entity_id));
    let compare_action = metadata_state.compare_action(metadata_target.clone());
    let musicbrainz_action = metadata_state.musicbrainz_action(metadata_target);
    let action_vm = LibraryTrackActionVm::new(
        frame.subscription_busy,
        frame.local_subscription,
        frame.add_to_playlist_open,
        frame.subscription_message.as_deref(),
    );
    let subscription_message = action_vm.subscription_message().map(str::to_owned);
    let subscription_message_is_error = action_vm.message_is_error();

    div()
        .flex()
        .flex_col()
        .items_start()
        .gap(spacing::XS)
        .child(
            action_button(action_vm.subscription_button_label(), cx)
                .disabled(frame.subscription_busy)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.toggle_local_subscription(cx);
                })),
        )
        .child(
            action_button(action_vm.add_to_playlist_label(), cx).on_click(cx.listener(
                |this, _, _, cx| {
                    if let Some(frame) = this.selected_track_frame_mut() {
                        frame.add_to_playlist_open = !frame.add_to_playlist_open;
                    }
                    cx.notify();
                },
            )),
        )
        .when(frame.add_to_playlist_open, |el| {
            el.child(render_add_to_playlist_panel(frame, playlists, cx))
        })
        .when_some(subscription_message, |el, message| {
            el.child(
                div()
                    .max_w(layout::STATUS_MESSAGE_WIDTH)
                    .text_size(typography::SIZE_MICRO)
                    .line_height(typography::LINE_TIGHT)
                    .text_color(if subscription_message_is_error {
                        StatusRole::Danger.color(cx)
                    } else {
                        color::text_muted()
                    })
                    .child(SharedString::from(message)),
            )
        })
        .when_some(compare_action, |el, action| {
            el.child(
                action_button(&action.label, cx)
                    .disabled(!action.enabled)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_tag_compare(cx);
                    })),
            )
        })
        .when_some(musicbrainz_action, |el, action| {
            el.child(
                action_button(&action.label, cx)
                    .disabled(!action.enabled)
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
                        .child(
                            div()
                                .text_size(typography::SIZE_MICRO)
                                .text_color(color::text_muted())
                                .child(SharedString::from(format!(
                                    "{count} staged tag edit{}",
                                    if count == 1 { "" } else { "s" }
                                ))),
                        )
                        .child(
                            action_button(&label, cx)
                                .disabled(frame.applying_id3_edits || has_pending_conflicts)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.apply_pending_id3_edits(cx);
                                })),
                        )
                        .when(has_pending_conflicts, |el| {
                            el.child(
                                div()
                                    .max_w(layout::CONFLICT_MESSAGE_WIDTH)
                                    .text_size(typography::SIZE_MICRO)
                                    .line_height(typography::LINE_TIGHT)
                                    .text_color(StatusRole::Danger.color(cx))
                                    .child(SharedString::from(format!(
                                        "Duplicate target: {conflict_text}"
                                    ))),
                            )
                        })
                        .when(
                            !frame.applying_id3_edits && !frame.pending_id3_edits.is_empty(),
                            |el| {
                                el.child(action_button("Discard staged", cx).on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.clear_pending_id3_edits(cx);
                                    },
                                )))
                            },
                        ),
                )
            },
        )
        .when_some(frame.id3_apply_error.clone(), |el, error| {
            el.child(
                div()
                    .max_w(layout::ACTION_MESSAGE_WIDTH)
                    .text_size(typography::SIZE_MICRO)
                    .line_height(typography::LINE_TIGHT)
                    .text_color(StatusRole::Danger.color(cx))
                    .child(SharedString::from(error)),
            )
        })
        .into_any_element()
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
        panel = panel.child(action_button(&label, cx).on_click(cx.listener(
            move |this, _, _, cx| {
                if let Some(frame) = this.selected_track_frame_mut() {
                    frame.add_to_playlist_open = false;
                }
                this.add_track_to_playlist(track_id, playlist_id, cx);
            },
        )));
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
        .child(
            Thumbnail::new(EntityKind::Track, ThumbnailSize::Lg).image(
                result
                    .file_image
                    .as_ref()
                    .map(|img| image_from_bytes(img.clone())),
            ),
        )
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
                        .child(TagBadge::new(EntityKind::Track).label(embedded_label.clone()))
                        .child(action_button("Re-read", cx).on_click(cx.listener(
                            |this, _, _, cx| {
                                this.reread_tag_compare(cx);
                            },
                        )))
                        .child(action_button("Re-download", cx).on_click(cx.listener(
                            |this, _, _, cx| {
                                this.redownload_tag_compare(cx);
                            },
                        ))),
                )
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .line_height(typography::LINE_HEADER)
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
        .child(
            Thumbnail::new(EntityKind::Track, ThumbnailSize::Lg).image(
                result
                    .image
                    .as_ref()
                    .map(|img| image_from_bytes(img.clone())),
            ),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(render_musicbrainz_title_bar(result, candidate, cx))
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .line_height(typography::LINE_HEADER)
                        .child(SharedString::from(candidate.title.clone())),
                )
                .child(
                    div()
                        .text_color(color::text_muted())
                        .text_size(typography::SIZE_MICRO)
                        .line_clamp(2)
                        .child(SharedString::from(musicbrainz_subtitle(
                            frame.musicbrainz_selected,
                            result,
                            candidate,
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
    let badge_fill = EntityKind::Track.fill_color(cx);
    let badge_text = EntityKind::Track.on_fill_color(cx);
    let candidates = result.lookup.candidates.clone();
    let selected_idx = candidates
        .iter()
        .position(|candidate| candidate.release_id == selected.release_id)
        .unwrap_or_default();
    let app = cx.weak_entity();

    // CONTROL-COMPAT(reason): native Button does not yet expose dropdown_menu, full-width alignment, and custom badge fill styling.
    Button::new("musicbrainz-release-picker")
        .label(SharedString::from(format!("MusicBrainz: {label}")))
        .scaled(Size::XSmall, cx)
        .compact()
        .ghost()
        .w_full()
        .justify_start()
        .bg(badge_fill)
        .text_color(badge_text)
        .text_size(typography::SIZE_MICRO)
        .font_weight(FontWeight::BOLD)
        .px(spacing::XS)
        .py(spacing::XXS)
        .border_1()
        .border_color(badge_fill)
        .rounded(radius::SM)
        .mb(spacing::XS)
        .dropdown_menu(move |menu, _window, _cx| {
            candidates.iter().enumerate().fold(
                menu.min_w(layout::MENU_MIN_WIDTH)
                    .max_w(layout::MENU_MAX_WIDTH)
                    .scrollable(true),
                |menu, (idx, candidate)| {
                    let app = app.clone();
                    menu.item(
                        gpui_component::menu::PopupMenuItem::new(SharedString::from(
                            musicbrainz_release_option_label(candidate),
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

#[expect(
    clippy::too_many_arguments,
    reason = "metadata grid needs explicit column state and edit state inputs"
)]
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
                    cells.push(metadata_musicbrainz_cell(&row, pending, cx));
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
        let id = SharedString::from(format!("section:id3-frame-group:{group_key}"));
        cell = cell.child(
            DisclosureGroup::new(id, label.clone())
                .collapsed(!expanded)
                .on_toggle(cx.listener(move |this, _, _, cx| {
                    this.toggle_id3_frame_group(group_key.clone(), cx);
                })),
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
    let source_role = pending_source_role(pending, MetadataColumn::Rss, row.rss_value.as_deref());
    let glyph = source_role.map(ProvenanceRole::glyph);
    let display_value = display_with_glyph(glyph, &base_display);
    let value_color = source_role
        .map(|role| role.color(cx))
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
                .w(layout::COMPACT_COLUMN_WIDTH)
                .flex_shrink_0()
                .text_color(color::text_primary())
                .text_size(typography::SIZE_MICRO)
                .line_height(typography::LINE_BODY)
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
        Some(ProvenanceRole::Match.glyph())
    } else {
        comparison_status_glyph(&row.id3_status)
    };
    let display_value = display_with_glyph(glyph, &base_display);
    let color = pending
        .map(|edit| pending_source_color(edit.source, cx))
        .unwrap_or_else(|| id3_cell_status_color(row, cx));
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
                .border_color(pending_source_color(edit.source, cx))
        })
        .into_any_element()
}

fn metadata_musicbrainz_cell(
    row: &AlignedCompareRow,
    pending: Option<&PendingId3Edit>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let source_role = pending_source_role(
        pending,
        MetadataColumn::MusicBrainz,
        row.musicbrainz_value.as_deref(),
    );
    let musicbrainz_color = source_role
        .map(|role| role.color(cx))
        .unwrap_or_else(|| comparison_status_color(&row.musicbrainz_status, cx));
    let value = row.musicbrainz_value.as_deref().unwrap_or("");
    let base_display = display_metadata_value(&row.field, value);
    let glyph = source_role.map_or_else(
        || comparison_status_glyph(&row.musicbrainz_status),
        |role| Some(role.glyph()),
    );
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
            .line_height(typography::LINE_BODY)
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
                    .child(
                        div()
                            .text_size(typography::SIZE_MICRO)
                            .text_color(color::text_muted())
                            .child(glyph),
                    ),
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
        .line_height(typography::LINE_BODY)
        .flex()
        .flex_row()
        .items_start()
        .gap(spacing::XS)
        .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _window, cx| {
            this.toggle_metadata_cell(cell_key.clone(), cx);
        }))
        .child(
            div()
                .text_size(typography::SIZE_MICRO)
                .text_color(color::text_muted())
                .child(glyph),
        )
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
                .w(layout::METADATA_LABEL_WIDTH)
                .flex_shrink_0()
                .text_color(frame_color)
                .text_size(typography::SIZE_MICRO)
                .line_height(typography::LINE_BODY)
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
                .child(
                    Thumbnail::new(EntityKind::Track, ThumbnailSize::Lg).image(Some(image.clone())),
                )
                .into_any_element();
        }
    }
    let value = expanded_metadata_display_value(field, raw_value, display_value);
    MultilineText::new(value.to_string())
        .max_lines(20)
        .color_raw(color)
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
        return vec![MultilineText::new(raw_value.to_string())
            .max_lines(20)
            .color_raw(color)
            .into_any_element()];
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
    let mut cell = MultilineText::new(value.to_string())
        .max_lines(4)
        .size(FontSize::Micro)
        .line_height(typography::LINE_BODY);
    if let Some(color) = color {
        cell = cell.color_raw(color);
    }
    cell.into_any_element()
}

fn compare_tag_cell(
    value: &str,
    color: Option<gpui::Rgba>,
    frame_id: Option<&str>,
    frame_color: Option<gpui::Rgba>,
) -> AnyElement {
    let mut body = MultilineText::new(value.to_string())
        .max_lines(4)
        .size(FontSize::Micro)
        .line_height(typography::LINE_BODY);
    if let Some(color) = color {
        body = body.color_raw(color);
    }
    div()
        .flex()
        .flex_row()
        .items_start()
        .gap(spacing::XS)
        .child(
            div()
                .w(layout::METADATA_LABEL_WIDTH)
                .flex_shrink_0()
                .text_color(frame_color.unwrap_or_else(color::text_muted))
                .text_size(typography::SIZE_MICRO)
                .line_height(typography::LINE_BODY)
                .child(SharedString::from(frame_id.unwrap_or_default().to_string())),
        )
        .child(div().flex_1().min_w_0().child(body))
        .into_any_element()
}

fn id3_frame_color(frame_id: &str) -> gpui::Rgba {
    match id3_frame_base(frame_id) {
        "SYLT" | "USLT" | "APIC" => color::id3_frame_v24_only(),
        "TXXX" | "WXXX" | "UFID" => color::id3_frame_v22(),
        _ => color::accent(),
    }
}

fn comparison_status_color(
    status: &crate::track_compare::ComparisonStatus,
    cx: &mut Context<LibraryApp>,
) -> gpui::Rgba {
    comparison_status_role(status).map_or_else(color::text_muted, |role| role.color(cx))
}

fn comparison_status_glyph(
    status: &crate::track_compare::ComparisonStatus,
) -> Option<&'static str> {
    comparison_status_role(status).map(ProvenanceRole::glyph)
}

fn comparison_status_role(
    status: &crate::track_compare::ComparisonStatus,
) -> Option<ProvenanceRole> {
    match status {
        crate::track_compare::ComparisonStatus::Match => Some(ProvenanceRole::Match),
        crate::track_compare::ComparisonStatus::Different => Some(ProvenanceRole::Different),
        crate::track_compare::ComparisonStatus::MissingSource
        | crate::track_compare::ComparisonStatus::MissingTag => Some(ProvenanceRole::Missing),
        crate::track_compare::ComparisonStatus::MissingBoth => None,
    }
}

fn pending_source_role(
    pending: Option<&PendingId3Edit>,
    column: MetadataColumn,
    cell_value: Option<&str>,
) -> Option<ProvenanceRole> {
    let edit = pending?;
    if edit.source != column {
        return None;
    }
    let cell_value = cell_value.map(str::trim).filter(|v| !v.is_empty())?;
    if cell_value == edit.value.trim() {
        Some(ProvenanceRole::Match)
    } else {
        Some(ProvenanceRole::Different)
    }
}

fn display_with_glyph(glyph: Option<&str>, value: &str) -> String {
    match glyph {
        Some(g) if !value.is_empty() => format!("{g} {value}"),
        Some(g) => g.to_string(),
        None => value.to_string(),
    }
}

fn id3_cell_status_color(row: &AlignedCompareRow, cx: &mut Context<LibraryApp>) -> gpui::Rgba {
    if row.id3_value.is_some() && row.rss_value.is_none() && row.musicbrainz_value.is_none() {
        return color::text_primary();
    }
    comparison_status_color(&row.id3_status, cx)
}

fn pending_source_color(source: MetadataColumn, cx: &mut Context<LibraryApp>) -> gpui::Rgba {
    match source {
        MetadataColumn::Rss | MetadataColumn::MusicBrainz => ProvenanceRole::Match.color(cx),
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

fn muted_line(value: &str) -> AnyElement {
    div()
        .text_color(color::text_muted())
        .text_size(typography::SIZE_MICRO)
        .child(SharedString::from(value.to_string()))
        .into_any_element()
}

fn compare_library_track(
    track: &TrackRow,
    musicindex_endpoint: &str,
) -> anyhow::Result<LibraryTrackCompare> {
    let path = track
        .local_path
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("library track has no local file"))?;
    let context = feed_service::fetch_library_track_context(track, musicindex_endpoint)
        .unwrap_or_else(|_| track_row_to_track_context(track));
    let tag_compare = subscribe_service::compare_downloaded_track_path(Path::new(path), &context)?;
    Ok(LibraryTrackCompare {
        tag_compare,
        track_context: context,
    })
}

fn hoverable_thumb(
    url: Option<String>,
    image: Option<Arc<Image>>,
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
            if this.vm.hovered_thumb_url() != Some(enter_url.as_str()) {
                this.set_hovered_thumb(Some(enter_url.clone()), cx);
            }
        }))
        .on_hover(cx.listener(move |this, entered: &bool, _, cx| {
            if !*entered && this.vm.hovered_thumb_url() == Some(leave_url.as_str()) {
                this.set_hovered_thumb(None, cx);
            }
        }))
        .child(inner)
        .into_any_element()
}

pub(crate) fn render_album_thumb(image: Option<Arc<Image>>, size: f32) -> AnyElement {
    if let Some(img_data) = image {
        ImagePrimitive::new(img_data)
            .dimension(px(size))
            .radius(Radius::SM)
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
            .text_size(layout::ACTION_ICON_INNER_SIZE)
            .flex_shrink_0()
            .child("\u{1F3B5}")
            .into_any_element()
    }
}
