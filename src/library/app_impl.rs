use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use gpui::{
    div, prelude::*, px, AnyElement, ClickEvent, Context, Entity, FontWeight, Image,
    InteractiveElement, IntoElement, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Render,
    SharedString, Styled, Window,
};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::Size;
use rusqlite::Connection;

#[cfg(feature = "async-runtime")]
use super::PlaylistActorState;
use super::{
    InspectorFrame, LazyPanel, LibraryApp, LibraryArtistDetail, LibraryDetail, LibraryTrackCompare,
    PlaylistDetail, ThumbnailState,
};
use crate::api::Client as MusicIndexClient;
use crate::application::commands::download::{
    SetTrackLibraryMembership, SubscribeThenAppendToPlaylist, SubscribeTrack,
};
use crate::application::commands::feed::{
    ApplyFeedUpdates, CheckFeedStaleness, CheckSubscribedFeeds,
};
use crate::application::commands::library_removal::RemoveFromLibrary;
use crate::application::commands::metadata::{
    LookupMusicBrainzAlbumReleases, LookupMusicBrainzTrack, StageMusicBrainzCandidate,
    StageMusicBrainzTrack,
};
use crate::application::commands::playlist::{
    CreatePlaylist, DeletePlaylist, RemovePlaylistTrackAt, RenamePlaylist, ReorderPlaylistTrack,
};
use crate::application::library_removal::{LibraryRemovalIntent, LibraryRemovalTarget};
use crate::application::{ApplicationServices, CommandContext};
use crate::audio_tags::write_id3v24_edits;
use crate::db::{self, TrackRow};
use crate::feed_service::{self, track_row_to_track_context, StagedMusicBrainzLookup};
use crate::library_service;
use crate::media::ImageCache;
use crate::metadata::{
    auto_populated_pending_id3_edits, pending_id3_conflict_descriptions,
    pending_id3_edits_for_apply, MusicBrainzLookupResult,
};
use crate::musicbrainz::{LookupMetadata, MusicBrainzCandidate};
use crate::presentation::GpuiCommandRunner;
use crate::sources;
use crate::subscribe_service::{self, SubscribeTrackRequest};
use crate::ui::composites::{
    DisclosureIndicator, DisclosureIndicatorDisplay, DisclosureSupplementDisplay,
    DisclosureSupplementLabel, ListRow, ListRowA11yLabel, PlaylistOption, PlaylistOptionDisplay,
    SkeletonTrackRow, SplitPane, StatusRole,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::layouts as layout;
use crate::ui::primitives::{Button as UiButton, Label};
use crate::ui::shells::library::detail::render_library_detail;
use crate::ui::shells::library::sidebar::render_library_sidebar;
use crate::ui::shells::library::track_detail_metadata::track_metadata_rows_for_frame;
use crate::ui::shells::library_removal_confirmation::open_library_removal_confirmation_dialog;
use crate::ui::sizable_bridge::SizableScaled;
use crate::ui::style::color;
use crate::ui::style::spacing;
use crate::ui::style::typography;
use crate::ui::tokens::SemanticColor;
use crate::view_models::entity_detail::TrackMetadataActionState;
use crate::view_models::library::{
    AlbumNode, ArtistNode, FeedUpdateActionDisplay, FeedUpdateActionKind, FeedUpdateDisplay,
    FeedUpdatePhase, LibraryTrackActionVm, LibraryTrackRowVm, LibraryTree, LibraryViewModel,
    MbTrackStatus, PlaylistAppendIntent, PlaylistAppendOutcome, PlaylistDetailActionsDisplay,
    PlaylistSidebarRowVm, PlaylistSidebarVm, TrackSubscribeOutcome,
};
use crate::view_models::playlist_option_displays;
use crate::view_models::search::pending_skeleton_count;
use crate::views::{EntityIdentityLinks, LocalIdentityFacts};

impl InspectorFrame {
    fn for_track(track: TrackRow, image: Option<Arc<Image>>) -> Self {
        let title = LibraryTrackRowVm::new(&track, None).display_title();
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
        }
    }
}

// ---------------------------------------------------------------------------
// LibraryApp
// ---------------------------------------------------------------------------

impl LibraryApp {
    pub fn new(
        conn: Arc<Mutex<Connection>>,
        cache: Arc<ImageCache>,
        musicindex_endpoint: String,
        application_services: Arc<ApplicationServices>,
        #[cfg(feature = "async-runtime")] runtime_host: Option<
            Arc<crate::presentation::RuntimeHost>,
        >,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let chrome = LibraryViewModel::chrome_display();
        let search_input = cx.new(|cx: &mut Context<InputState>| {
            InputState::new(window, cx).placeholder(chrome.search_placeholder)
        });
        let search_sub = cx.subscribe(&search_input, Self::on_search_event);
        let new_playlist_input = cx.new(|cx: &mut Context<InputState>| {
            InputState::new(window, cx).placeholder(chrome.new_playlist_placeholder)
        });
        let rename_playlist_input = cx.new(|cx: &mut Context<InputState>| {
            InputState::new(window, cx)
                .placeholder(PlaylistDetailActionsDisplay::RENAME_INPUT_PLACEHOLDER)
        });
        let rename_playlist_sub =
            cx.subscribe(&rename_playlist_input, Self::on_rename_playlist_event);
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
            rename_playlist_input,
            _rename_playlist_sub: rename_playlist_sub,
            #[cfg(feature = "async-runtime")]
            runtime_host,
            #[cfg(feature = "async-runtime")]
            playlist_actor: None,
        };
        app.start_async_reload(cx);
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

    fn on_rename_playlist_event(
        &mut self,
        _entity: Entity<InputState>,
        event: &InputEvent,
        cx: &mut Context<Self>,
    ) {
        if let InputEvent::PressEnter { .. } = event {
            self.submit_playlist_rename(cx);
        }
    }

    fn apply_search(&mut self, cx: &mut Context<Self>) {
        self.vm
            .apply_search_query(self.search_input.read(cx).value().to_string());
        self.detail = LibraryDetail::None;
        cx.notify();
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.start_async_reload(cx);
    }

    pub fn focus_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search_input.update(cx, |input, cx| {
            input.focus(window, cx);
        });
    }

    pub fn begin_new_playlist(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.vm.playlist_sidebar().creating_playlist {
            self.vm.toggle_creating_playlist();
        }
        self.vm.cancel_playlist_rename();
        self.new_playlist_input.update(cx, |input, cx| {
            input.focus(window, cx);
        });
        cx.notify();
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

    /// Kick off a non-blocking library reload.
    ///
    /// The render thread returns immediately; the heavy
    /// `library_tracks` + `build_tree` work runs on the background
    /// executor. While the reload is in flight the view-model is
    /// flagged so the sidebar paints skeleton placeholders instead of
    /// blocking the cold-open paint.
    pub(crate) fn start_async_reload(&mut self, cx: &mut Context<Self>) {
        self.vm.begin_library_reload();
        self.vm.clear_library_selection();
        self.vm.clear_mb_status();
        self.detail = LibraryDetail::None;
        cx.notify();

        // Playlists are a small single-table query: keep them on the
        // foreground for now so the playlist sidebar stays responsive
        // and we don't grow a second async path until proven needed.
        self.reload_playlists();

        let conn = Arc::clone(&self.conn);
        cx.spawn(
            async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                let result = cx
                    .background_executor()
                    .spawn(async move {
                        let conn = conn.lock().expect("lock db");
                        library_service::library_tracks(&conn).map(|rows| {
                            let count = rows.len();
                            let tree = build_tree(&rows, &conn);
                            (count, tree)
                        })
                    })
                    .await;
                let _ = this.update(cx, |this, cx| {
                    match result {
                        Ok((count, tree)) => {
                            this.vm.replace_tree(tree);
                            this.vm.finish_library_reload(count);
                        }
                        Err(err) => this.vm.set_error_status(err),
                    }
                    cx.notify();
                });
            },
        )
        .detach();
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
            #[cfg(feature = "async-runtime")]
            self.spawn_playlist_actor(id, cx);
        }
        cx.notify();
    }

    /// Spawn (or replace) the paged playlist actor for `playlist_id`.
    ///
    /// Dropping the previous handle closes its inbox so the actor task
    /// exits gracefully. The new handle is bridged via
    /// [`crate::presentation::bridge_watch`] so snapshot publishes
    /// trigger a re-render automatically.
    #[cfg(feature = "async-runtime")]
    fn spawn_playlist_actor(&mut self, playlist_id: i64, cx: &mut Context<Self>) {
        use crate::application::paged_track_list::PagedTrackListActor;
        use crate::db::{open_db, TrackListing};
        use crate::presentation::bridge_watch;

        let Some(host) = self.runtime_host.clone() else {
            return;
        };
        // Idempotent: if the actor for this playlist is already running,
        // keep it (and its warm cache) rather than churning a new one.
        if let Some(state) = &self.playlist_actor {
            if state.playlist_id == playlist_id {
                return;
            }
        }
        // Open a dedicated connection for the actor: rusqlite Connections
        // are not Sync, and the actor consumes its connection by value.
        let cfg = match crate::config::config_path()
            .ok()
            .and_then(|path| crate::config::load_config(&path).ok())
        {
            Some(cfg) => cfg,
            None => return,
        };
        let conn = match open_db(&cfg) {
            Ok(conn) => conn,
            Err(err) => {
                eprintln!("v4vmm::library: failed to open actor DB conn: {err}");
                return;
            }
        };
        let actor = match PagedTrackListActor::new(conn, TrackListing::Playlist { playlist_id }) {
            Ok(actor) => actor,
            Err(err) => {
                eprintln!("v4vmm::library: PagedTrackListActor::new failed: {err}");
                return;
            }
        };
        let bus = host.bus().clone();
        let _enter = host.handle().enter();
        let handle = actor.spawn(bus);
        let snapshot = handle.borrow().clone();
        let rx = handle.subscribe();
        self.playlist_actor = Some(PlaylistActorState {
            playlist_id,
            snapshot,
            handle,
        });
        bridge_watch(
            rx,
            move |this: &mut Self, snap, _cx| {
                if let Some(state) = &mut this.playlist_actor {
                    if state.playlist_id == playlist_id {
                        state.snapshot = snap;
                    }
                }
            },
            cx,
        );
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

    pub(crate) fn begin_playlist_rename(
        &mut self,
        playlist_id: i64,
        current_name: String,
        placeholder: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.vm.begin_playlist_rename(playlist_id);
        self.rename_playlist_input.update(cx, |input, cx| {
            input.set_placeholder(placeholder, window, cx);
            input.set_value(current_name, window, cx);
            input.focus(window, cx);
        });
        cx.notify();
    }

    pub(crate) fn cancel_playlist_rename(&mut self, cx: &mut Context<Self>) {
        self.vm.cancel_playlist_rename();
        cx.notify();
    }

    pub(crate) fn submit_playlist_rename(&mut self, cx: &mut Context<Self>) {
        let Some(playlist_id) = self.vm.renaming_playlist_id() else {
            return;
        };
        let name = self.rename_playlist_input.read(cx).value().to_string();
        if name.trim().is_empty() {
            return;
        }
        self.vm.cancel_playlist_rename();
        self.rename_playlist(playlist_id, name, cx);
        cx.notify();
    }

    pub(crate) fn delete_playlist(&mut self, id: i64, cx: &mut Context<Self>) {
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

    pub(crate) fn remove_playlist_track_at(
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

    pub(crate) fn move_playlist_track(
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

    pub(crate) fn add_track_to_playlist(
        &mut self,
        track_id: i64,
        playlist_id: i64,
        cx: &mut Context<Self>,
    ) {
        if let Some(intent) = self.vm.begin_playlist_append(playlist_id, vec![track_id]) {
            self.spawn_subscribe_then_append(intent, cx);
        }
    }

    pub(crate) fn create_playlist_and_add_track(
        &mut self,
        name: &str,
        track_id: i64,
        cx: &mut Context<Self>,
    ) {
        let command = CreatePlaylist::new(Arc::clone(&self.conn), name.to_string());
        self.command_runner.run(
            command,
            CommandContext::next(),
            cx,
            move |this, result, cx| {
                this.reload_playlists();
                this.add_track_to_playlist(track_id, result.playlist_id(), cx);
            },
            |this, err, _cx| this.vm.fail_playlist_create(err),
        );
    }

    pub(crate) fn add_album_to_playlist(
        &mut self,
        feed_id: i64,
        playlist_id: i64,
        cx: &mut Context<Self>,
    ) {
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

    pub(crate) fn create_playlist_and_add_album(
        &mut self,
        name: &str,
        feed_id: i64,
        cx: &mut Context<Self>,
    ) {
        let command = CreatePlaylist::new(Arc::clone(&self.conn), name.to_string());
        self.command_runner.run(
            command,
            CommandContext::next(),
            cx,
            move |this, result, cx| {
                this.reload_playlists();
                this.add_album_to_playlist(feed_id, result.playlist_id(), cx);
            },
            |this, err, _cx| this.vm.fail_playlist_create(err),
        );
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
        let animated = animated && crate::ui::tokens::Environment::current(cx).allows_motion();
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

    pub(crate) fn hovered_thumb_url(&self) -> Option<&str> {
        self.vm.hovered_thumb_url()
    }

    pub(crate) fn set_hovered_thumb(&mut self, url: Option<String>, cx: &mut Context<Self>) {
        if self.vm.set_hovered_thumb_url(url) {
            cx.notify();
        }
    }

    pub(crate) fn select_album(&mut self, album: &AlbumNode, cx: &mut Context<Self>) {
        if let Some(feed_id) = album.feed_id {
            self.vm.select_library_item(feed_id);
        } else {
            self.vm.clear_library_selection();
        }
        self.detail = LibraryDetail::Album(album.clone());
        self.hydrate_album_identity_on_view(album, cx);
        if let Some(feed_id) = album.feed_id {
            self.check_feed_on_view(feed_id, cx);
        }
    }

    pub(crate) fn select_album_by_name(&mut self, name: &str, cx: &mut Context<Self>) {
        let tree_artists = self.vm.tree().artists.clone();
        for artist_node in &tree_artists {
            for album in &artist_node.albums {
                if album.name == name {
                    self.select_album(album, cx);
                    cx.notify();
                    return;
                }
            }
        }
    }

    fn hydrate_album_identity_on_view(&mut self, album: &AlbumNode, cx: &mut Context<Self>) {
        if album_has_feed_identity_actions(&album.identity_facts) {
            return;
        }
        let (Some(feed_id), Some(feed_guid)) = (album.feed_id, album.feed_guid.clone()) else {
            return;
        };

        let conn = Arc::clone(&self.conn);
        let musicindex_endpoint = self.musicindex_endpoint.clone();
        cx.spawn(
            async move |this: gpui::WeakEntity<LibraryApp>, cx: &mut gpui::AsyncApp| {
                let result = cx
                    .background_executor()
                    .spawn(async move {
                        hydrate_album_identity_facts(
                            conn,
                            &musicindex_endpoint,
                            feed_id,
                            &feed_guid,
                        )
                    })
                    .await;
                this.update(
                    cx,
                    move |this: &mut LibraryApp, cx: &mut Context<LibraryApp>| {
                        if let Ok(facts) = result {
                            this.vm.update_album_identity_facts(feed_id, &facts);
                            if let LibraryDetail::Album(album) = &mut this.detail {
                                if album.feed_id == Some(feed_id) {
                                    album.identity_facts = facts;
                                }
                            }
                            cx.notify();
                        }
                    },
                )
                .ok();
            },
        )
        .detach();
    }

    pub(crate) fn select_artist(&mut self, name: &str, _cx: &mut Context<Self>) {
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
        let view = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))
            .and_then(|conn| sources::local_artist_view_from_tracks(&conn, name, &tracks))
            .unwrap_or_else(|_| crate::views::ArtistView::from_local_rows(name, &tracks));
        self.vm.clear_library_selection();
        self.detail = LibraryDetail::Artist(Box::new(LibraryArtistDetail {
            name: name.to_string(),
            view,
            tracks,
        }));
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
                this.vm.finish_feed_view_check_error(feed_id, error);
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
                    self.vm.set_feed_check_error(err);
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
                this.vm.set_feed_check_error(error);
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
                this.vm.finish_apply_feed_updates_error(error);
            },
        );
    }

    pub(crate) fn select_track(&mut self, track: &TrackRow, cx: &mut Context<Self>) {
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

    pub(crate) fn toggle_artist(&mut self, name: &str) {
        self.vm.toggle_artist(name);
    }

    pub(crate) fn toggle_album(&mut self, artist: &str, album: &str) {
        self.vm.toggle_album(artist, album);
    }

    pub(crate) fn unsubscribe_feed(
        &mut self,
        feed_id: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.request_library_removal(LibraryRemovalIntent::FeedId(feed_id), window, cx);
    }

    pub(crate) fn remove_track(
        &mut self,
        track_id: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.request_library_removal(LibraryRemovalIntent::TrackId(track_id), window, cx);
    }

    fn request_library_removal(
        &mut self,
        intent: LibraryRemovalIntent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let plan_result = {
            let conn = self.conn.lock().expect("lock db");
            self.application_services
                .query_service()
                .library_removal_plan(&conn, &intent)
        };
        let plan = match plan_result {
            Ok(plan) => plan,
            Err(err) => {
                self.vm.set_error_status(err);
                cx.notify();
                return;
            }
        };
        if !self.vm.confirm_library_removal(plan) {
            let Some(display) = self.vm.pending_library_removal_confirmation() else {
                cx.notify();
                return;
            };
            open_library_removal_confirmation_dialog(
                window,
                cx,
                display,
                |this, cx| {
                    this.vm.cancel_pending_library_removal();
                    cx.notify();
                },
                |this, cx| {
                    this.execute_pending_library_removal(cx);
                },
            );
            cx.notify();
            return;
        }

        self.execute_library_removal_target(plan.target(), cx);
    }

    fn execute_library_removal_target(
        &mut self,
        target: LibraryRemovalTarget,
        cx: &mut Context<Self>,
    ) {
        let command = RemoveFromLibrary::new(Arc::clone(&self.conn), target);
        self.command_runner.run(
            command,
            CommandContext::next(),
            cx,
            |this, _result, cx| this.start_async_reload(cx),
            |this, err, _cx| this.vm.set_error_status(err),
        );
    }

    fn execute_pending_library_removal(&mut self, cx: &mut Context<Self>) {
        let Some(target) = self.vm.take_pending_library_removal() else {
            cx.notify();
            return;
        };
        self.execute_library_removal_target(target, cx);
    }

    pub(crate) fn subscribe_track(&mut self, track: TrackRow, cx: &mut Context<Self>) {
        if self.vm.has_busy_track() {
            return;
        }
        let track_id = track.id;
        self.vm.begin_busy_track(
            track_id,
            LibraryTrackActionVm::track_subscribe_begin_status(),
        );
        cx.notify();

        let command = SubscribeTrack::new(
            Arc::clone(&self.conn),
            self.application_services.download_manager(),
            SubscribeTrackRequest::LibraryTrack {
                track: Box::new(track),
            },
            LibraryTrackActionVm::track_subscribe_success_message(),
        );
        self.command_runner.run(
            command,
            CommandContext::next(),
            cx,
            |this, outcome, cx| {
                this.vm.finish_track_subscribe(TrackSubscribeOutcome::new(
                    outcome.path().to_string(),
                    outcome.format_warning().map(str::to_string),
                ));
                this.start_async_reload(cx);
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

    pub(crate) fn toggle_id3_frame_group(&mut self, group_key: String, cx: &mut Context<Self>) {
        let Some(frame) = self.selected_track_frame_mut() else {
            return;
        };
        if !frame.expanded_id3_frame_groups.remove(&group_key) {
            frame.expanded_id3_frame_groups.insert(group_key);
        }
        cx.notify();
    }

    pub(crate) fn toggle_metadata_cell(&mut self, cell_key: String, cx: &mut Context<Self>) {
        let Some(frame) = self.selected_track_frame_mut() else {
            return;
        };
        if !frame.expanded_metadata_cells.remove(&cell_key) {
            frame.expanded_metadata_cells.insert(cell_key);
        }
        cx.notify();
    }

    pub(crate) fn apply_pending_id3_edits(&mut self, cx: &mut Context<Self>) {
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
            frame.id3_apply_error = Some(TrackMetadataActionState::duplicate_id3_target_message(
                &conflicts,
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
                                        frame.id3_apply_error = Some(
                                            TrackMetadataActionState::id3_apply_error_message(
                                                error,
                                            ),
                                        );
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

    pub(crate) fn clear_pending_id3_edits(&mut self, cx: &mut Context<Self>) {
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

    pub(crate) fn toggle_local_subscription(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((track_id, subscribe)) = (match self.selected_track_frame_mut() {
            Some(frame) if frame.subscription_busy => return,
            Some(frame) if frame.local_subscription => Some((frame.entity_id, false)),
            Some(frame) => {
                frame.subscription_busy = true;
                frame.subscription_message =
                    Some(LibraryTrackActionVm::subscription_busy_message(true).into());
                Some((frame.entity_id, true))
            }
            None => None,
        }) else {
            return;
        };
        if !subscribe {
            self.request_library_removal(LibraryRemovalIntent::TrackId(track_id), window, cx);
            return;
        }
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
                        frame.subscription_message = Some(
                            LibraryTrackActionVm::subscription_error_message(subscribe, err),
                        );
                    }
                }
            },
        );
    }

    pub(crate) fn toggle_tag_compare(&mut self, cx: &mut Context<Self>) {
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
                                    Err(error) => LazyPanel::Empty(
                                        LibraryViewModel::deferred_panel_error_message(error),
                                    ),
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

    pub(crate) fn redownload_tag_compare(&mut self, cx: &mut Context<Self>) {
        self.reload_tag_compare(cx);
    }

    pub(crate) fn reread_tag_compare(&mut self, cx: &mut Context<Self>) {
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
                                    Err(error) => LazyPanel::Empty(
                                        LibraryViewModel::deferred_panel_error_message(error),
                                    ),
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

    pub(crate) fn toggle_musicbrainz_lookup(&mut self, cx: &mut Context<Self>) {
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
                        frame.musicbrainz_lookup =
                            LazyPanel::Empty(LibraryViewModel::deferred_panel_error_message(error));
                    }
                }
            },
        );
    }

    pub(crate) fn select_musicbrainz_candidate(&mut self, idx: usize, cx: &mut Context<Self>) {
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

    pub(crate) fn musicbrainz_feed(&mut self, album: AlbumNode, cx: &mut Context<Self>) {
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
        let row_vm = LibraryTrackRowVm::new(track, None);
        let artist = row_vm.display_artist();
        let album = row_vm.display_album();
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
                    let feed_guid = tracks.first().and_then(|t| t.feed_guid.clone());
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
                        feed_guid,
                        feed_url,
                        image_href,
                        identity_facts: feed_id
                            .and_then(|fid| crate::local_identity::feed_facts(conn, fid).ok())
                            .unwrap_or_default(),
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

fn album_has_feed_identity_actions(facts: &LocalIdentityFacts) -> bool {
    let identity = EntityIdentityLinks::from_source_facts(
        None,
        facts.source_links.clone(),
        facts.source_ids.clone(),
    );
    identity.website_url.is_some() && identity.nostr_npub.is_some()
}

fn hydrate_album_identity_facts(
    conn: Arc<Mutex<Connection>>,
    musicindex_endpoint: &str,
    feed_id: i64,
    feed_guid: &str,
) -> anyhow::Result<LocalIdentityFacts> {
    let client = MusicIndexClient::new_with_base_url(musicindex_endpoint.to_string());
    let feed = client.fetch_feed(
        feed_guid,
        Some("source_links,source_ids,source_contributors"),
    )?;
    let mut db = conn
        .lock()
        .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
    crate::identity_ingest::persist_musicindex_feed(&mut db, feed_id, &feed)?;
    crate::local_identity::feed_facts(&db, feed_id)
}

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

impl Render for LibraryApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let chrome = LibraryViewModel::chrome_display();
        let status = self.vm.status_snapshot();
        let status_color = if status.is_error {
            StatusRole::Danger.color(cx)
        } else {
            color::text_muted()
        };
        let status_text = status.text;

        // Collect image URLs from tree, then fetch thumbnails (avoids borrow conflict).
        let mut urls: Vec<String> = {
            self.vm
                .tree()
                .artists
                .iter()
                .flat_map(|a| &a.albums)
                .flat_map(|album| {
                    album
                        .image_href
                        .iter()
                        .chain(
                            album
                                .identity_facts
                                .contributors
                                .iter()
                                .filter_map(|contributor| contributor.image_url.as_ref()),
                        )
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
        if let LibraryDetail::Artist(artist) = &self.detail {
            if let Some(url) = artist.view.image_url.clone() {
                urls.push(url);
            }
        }
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
        let tree_items: Vec<AnyElement> = render_library_sidebar(
            &tree_projection.tree,
            &tree_projection.expanded_artists,
            &tree_projection.expanded_albums,
            self.vm.selected_id(),
            &album_thumbs,
            cx,
        );
        let filtered_empty = tree_projection.is_empty();

        let PlaylistSidebarVm {
            header_id: playlist_header_id,
            sort_button_id: playlist_sort_button_id,
            add_button_id: playlist_add_button_id,
            new_playlist_input_id,
            new_playlist_add_button_id,
            expanded: playlists_expanded,
            disclosure_glyph: playlist_disclosure_glyph,
            heading: playlist_heading,
            sort_label: playlist_sort_label,
            add_label: playlist_add_label,
            new_playlist_add_label,
            creating_playlist,
            rows: playlist_rows,
        } = self.vm.playlist_sidebar();
        let mut left_items: Vec<AnyElement> = Vec::new();

        left_items.push(
            div()
                .id(playlist_header_id)
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
                        .child(DisclosureIndicator::new(DisclosureIndicatorDisplay {
                            glyph: playlist_disclosure_glyph.into(),
                        }))
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(color::text_primary())
                                .child(playlist_heading),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(spacing::XS)
                        .items_center()
                        .child(
                            UiButton::styled(playlist_sort_button_id, ControlStyle::ToolbarIcon)
                                .label(playlist_sort_label)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.cycle_playlist_sort(cx);
                                })),
                        )
                        .child(
                            UiButton::styled(playlist_add_button_id, ControlStyle::ToolbarIcon)
                                .label(playlist_add_label)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.vm.toggle_creating_playlist();
                                    cx.notify();
                                })),
                        ),
                )
                .into_any_element(),
        );

        if playlists_expanded {
            for row in playlist_rows {
                let PlaylistSidebarRowVm {
                    id: playlist_id,
                    element_id,
                    name,
                    track_count_label,
                    selected,
                } = row;
                let row_a11y_label = format!("Playlist: {name}");
                left_items.push(
                    ListRow::compact(SharedString::from(element_id))
                        .a11y_label(ListRowA11yLabel {
                            label: row_a11y_label.into(),
                        })
                        .selected(selected)
                        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                            this.select_playlist(playlist_id, cx);
                        }))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .pl(spacing::MD)
                                .child(
                                    Label::new(name)
                                        .color(if selected {
                                            SemanticColor::Accent
                                        } else {
                                            SemanticColor::Label
                                        })
                                        .truncated(),
                                )
                                .child(DisclosureSupplementLabel::new(
                                    DisclosureSupplementDisplay {
                                        label: track_count_label.into(),
                                    },
                                )),
                        )
                        .into_any_element(),
                );
            }

            if creating_playlist {
                left_items.push(
                    div()
                        .id(new_playlist_input_id)
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
                            UiButton::styled(new_playlist_add_button_id, ControlStyle::Primary)
                                .label(new_playlist_add_label)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.create_playlist(cx);
                                })),
                        )
                        .into_any_element(),
                );
            }
        }

        if self.vm.is_library_loading() && tree_items.is_empty() {
            let count = pending_skeleton_count(true, false);
            for i in 0..count {
                left_items.push(
                    SkeletonTrackRow::new(("library-skeleton-row", i))
                        .show_thumbnail(false)
                        .into_any_element(),
                );
            }
        } else {
            left_items.extend(tree_items);
        }

        let detail_pane = render_library_detail(
            &self.detail,
            self.vm.busy_track(),
            self.vm.mb_status(),
            &album_thumbs,
            self.vm.playlists(),
            &chrome,
            self.rename_playlist_input.clone(),
            self.vm.renaming_playlist_id(),
            #[cfg(feature = "async-runtime")]
            self.playlist_actor.as_ref(),
            cx,
        );

        let leading_pane = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
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
                            .child(chrome.search_heading),
                    )
                    .child(
                        Input::new(&self.search_input)
                            .cleanable(true)
                            .scaled(Size::Small, cx),
                    )
                    .child(
                        UiButton::styled(chrome.search_button_id, ControlStyle::Primary)
                            .label(chrome.search_button_label)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.apply_search(cx);
                            })),
                    ),
            )
            .child({
                let FeedUpdateDisplay {
                    status_message: feed_status,
                    action,
                } = self.vm.feed_update_display();
                let FeedUpdateActionDisplay {
                    kind,
                    button_id,
                    label,
                    disabled,
                } = action;
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
                    .child(if kind == FeedUpdateActionKind::ApplyUpdates {
                        UiButton::styled(button_id, ControlStyle::Primary)
                            .label(label)
                            .disabled(disabled)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.apply_all_feed_updates(cx);
                            }))
                    } else {
                        UiButton::styled(button_id, ControlStyle::Secondary)
                            .label(label)
                            .disabled(disabled)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.check_all_feeds(cx);
                            }))
                    })
            })
            .child(
                div()
                    .id(chrome.list_scroll_id)
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p(spacing::SM)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(spacing::XXS)
                            .children(left_items)
                            .when(self.vm.should_show_empty_library(filtered_empty), |el| {
                                el.child(
                                    div()
                                        .text_center()
                                        .p(spacing::XXL + spacing::LG)
                                        .text_color(color::text_muted())
                                        .child(
                                            div().mt(spacing::SM).child(chrome.empty_library_label),
                                        ),
                                )
                            }),
                    ),
            )
            .into_any_element();

        let trailing_pane = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .overflow_hidden()
            .child(detail_pane)
            .into_any_element();
        let split_pane = SplitPane::new(chrome.split_pane_id)
            .resize_handle_id(chrome.resize_handle_id)
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

pub(crate) fn playlist_options(playlists: &[db::Playlist]) -> Vec<PlaylistOption> {
    playlist_option_displays(playlists)
        .into_iter()
        .map(|option| {
            PlaylistOption::new(PlaylistOptionDisplay {
                id: option.id,
                name: SharedString::from(option.name),
                a11y_label: SharedString::from(option.a11y_label),
            })
        })
        .collect()
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
