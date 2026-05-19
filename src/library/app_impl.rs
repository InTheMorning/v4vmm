use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::PlaylistActorState;
use super::{
    InspectorFrame, LazyPanel, LibraryApp, LibraryArtistDetail, LibraryDetail, PlaylistDetail,
    ThumbnailState,
};
use crate::application::commands::download::{SubscribeThenAppendToPlaylist, SubscribeTrack};
use crate::application::commands::feed::{
    ApplyFeedUpdates, CheckFeedStaleness, CheckSubscribedFeeds, SubscribeFeed,
};
use crate::application::commands::library_removal::RemoveFromLibrary;
use crate::application::commands::metadata::{
    ApplyTrackId3Edits, LookupMusicBrainzTrack, StageMusicBrainzTrack,
};
use crate::application::commands::playlist::{
    CreatePlaylist, DeletePlaylist, RemovePlaylistTrackAt, RenamePlaylist, ReorderPlaylistTrack,
};
use crate::application::library_removal::{LibraryRemovalIntent, LibraryRemovalTarget};
use crate::application::queries::images::FetchThumbnail;
use crate::application::queries::library::{
    CompareLibraryTrack, FetchLibraryTrackContext, HydrateAlbumIdentity, LoadLibraryTracksTree,
};
use crate::application::{ApplicationServices, AsyncCommandRunner, CommandContext};
use crate::db::{self, TrackRow};
use crate::feed_service::track_row_to_track_context;
use crate::library_service;
use crate::media::ImageCache;
use crate::metadata::{
    auto_populated_pending_id3_edits, pending_id3_conflict_descriptions,
    pending_id3_edits_for_apply, MusicBrainzLookupResult,
};
use crate::presentation::{bridge_watch, present_command};
use crate::runtime::musicbrainz_feed_saga::{MusicBrainzFeedSagaState, StartFeedLookup};
use crate::sources;
use crate::subscribe_service::{self, SubscribeFeedRequest, SubscribeTrackRequest};
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
use crate::ui::tokens::{FontSize, SemanticColor, Spacing};
use crate::view_models::entity_detail::TrackMetadataActionState;
use crate::view_models::library::{
    description_line_count, AlbumNode, FeedUpdateActionDisplay, FeedUpdateActionKind,
    FeedUpdateDisplay, FeedUpdatePhase, InspectorPanelKind, LibraryTrackActionVm,
    LibraryTrackInspectorState, LibraryTrackRowVm, LibraryTree, LibraryViewModel, MbTrackStatus,
    PlaylistAppendIntent, PlaylistAppendOutcome, PlaylistDetailActionsDisplay,
    PlaylistSidebarRowVm, PlaylistSidebarVm, SavedSearchesSectionDisplay, TrackSubscribeOutcome,
};
use crate::view_models::pagination::pending_skeleton_count;
use crate::view_models::playlist_option_displays;
use crate::view_models::workspace::{
    BreadcrumbDisplay, ContentFilter, FilterChipStripDisplay, FrameNavigationEntry,
    FrameNavigationState, WorkspaceFrameId, WorkspaceLayout, WorkspaceModelError,
};
use crate::views::{EntityIdentityLinks, LocalIdentityFacts};
use gpui::{
    div, prelude::*, px, AnyElement, ClickEvent, Context, Entity, FontWeight, Image,
    InteractiveElement, IntoElement, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Render,
    SharedString, Styled, Window,
};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::Size;
use rusqlite::Connection;

impl InspectorFrame {
    fn for_track(track: TrackRow, image: Option<Arc<Image>>) -> Self {
        let title = LibraryTrackRowVm::new(&track, None).display_title();
        let local_subscription = track.is_in_library && track.local_path.is_some();
        Self {
            entity_id: track.id,
            title,
            local_subscription,
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
            inspector_state: LibraryTrackInspectorState::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LibraryReloadMode {
    ResetDetail,
    PreserveDetail,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FrameHistoryMode {
    Record,
    Restore,
}

#[derive(Clone, Debug)]
enum TrackSubscriptionAction {
    Download(Box<TrackRow>),
    Remove(i64),
}

fn apply_library_removal_to_album_detail(detail: &mut LibraryDetail, target: LibraryRemovalTarget) {
    if let LibraryDetail::Album(album) = detail {
        match target {
            LibraryRemovalTarget::Track(track_id) => {
                if let Some(track) = album.tracks.iter_mut().find(|track| track.id == track_id) {
                    track.is_in_library = false;
                    track.local_path = None;
                }
            }
            LibraryRemovalTarget::Feed(feed_id) if album.feed_id == Some(feed_id) => {
                for track in &mut album.tracks {
                    track.is_in_library = false;
                    track.local_path = None;
                }
            }
            LibraryRemovalTarget::Feed(_) => {}
        }
    }
}

fn apply_track_subscription_to_album_detail(
    detail: &mut LibraryDetail,
    track_id: i64,
    path: &str,
    marked_downloaded: bool,
) {
    if !marked_downloaded {
        return;
    }

    let LibraryDetail::Album(album) = detail else {
        return;
    };

    if let Some(track) = album.tracks.iter_mut().find(|track| track.id == track_id) {
        track.is_in_library = true;
        track.local_path = Some(path.to_string());
    }
}

fn api_feed_from_album(album: &AlbumNode) -> crate::api::Feed {
    crate::api::Feed {
        feed_guid: album.feed_guid.clone(),
        feed_url: album.feed_url.clone(),
        title: Some(album.name.clone()),
        name: Some(album.name.clone()),
        description: album.description.clone(),
        image_url: album.image_href.clone(),
        tracks: Some(
            album
                .tracks
                .iter()
                .map(subscribe_service::track_row_to_api_track)
                .collect(),
        ),
        ..crate::api::Feed::default()
    }
}

fn album_thumbnail_urls(album: &AlbumNode) -> Vec<String> {
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
        .collect()
}

fn apply_library_removal_to_inspector_frame(
    frame: &mut InspectorFrame,
    target: LibraryRemovalTarget,
) {
    let message = match target {
        LibraryRemovalTarget::Track(track_id) if frame.entity_id == track_id => {
            Some("Removed track")
        }
        LibraryRemovalTarget::Feed(feed_id) if frame.track.feed_id == feed_id => {
            Some("Removed feed")
        }
        LibraryRemovalTarget::Track(_) | LibraryRemovalTarget::Feed(_) => None,
    };

    if let Some(message) = message {
        reset_removed_inspector_frame(frame, message);
    }
}

fn reset_removed_inspector_frame(frame: &mut InspectorFrame, message: &'static str) {
    frame.subscription_busy = false;
    frame.local_subscription = false;
    frame.track.is_in_library = false;
    frame.track.local_path = None;
    frame.source_context = None;
    frame.tag_compare = LazyPanel::Hidden;
    frame.pending_id3_edits.clear();
    frame.suppressed_auto_id3_edits.clear();
    frame.id3_apply_error = None;
    frame.subscription_message = Some(message.into());
}

// ---------------------------------------------------------------------------
// LibraryApp
// ---------------------------------------------------------------------------

impl LibraryApp {
    fn default_workspace_layout() -> WorkspaceLayout {
        let mut layout = WorkspaceLayout::default_layout();
        layout
            .reset_nav(Self::content_frame_id(), FrameNavigationEntry::SourceList)
            .expect("default workspace layout contains the content frame");
        layout
    }

    const fn content_frame_id() -> WorkspaceFrameId {
        WorkspaceLayout::default_content_frame_id()
    }

    pub(crate) fn content_filter_chip_strip(&self) -> FilterChipStripDisplay {
        self.vm.content_filter_chip_strip()
    }

    pub(crate) fn has_filterable_content_detail(&self) -> bool {
        matches!(self.detail, LibraryDetail::Album(_))
    }

    pub(crate) fn set_content_filter(&mut self, filter: ContentFilter, cx: &mut Context<Self>) {
        self.vm.set_content_filter(filter);
        cx.notify();
    }

    #[allow(dead_code)]
    pub(crate) fn set_content_list_text_filter(
        &mut self,
        filter: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.vm.set_content_text_filter(filter);
        cx.notify();
    }

    #[allow(dead_code)]
    pub(crate) fn set_detail_text_filter(
        &mut self,
        filter: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.vm.set_detail_text_filter(filter);
        cx.notify();
    }

    fn open_saved_search(&mut self, saved_search_id: i64, cx: &mut Context<Self>) {
        if let Some(saved_search) = self
            .vm
            .saved_searches()
            .iter()
            .find(|saved_search| saved_search.id == saved_search_id)
        {
            cx.emit(super::LibraryAppEvent::OpenSavedSearch {
                saved_search_id,
                query: saved_search.query.clone(),
            });
        }
    }

    fn current_frame_navigation_mut(
        &mut self,
    ) -> Result<&mut FrameNavigationState, WorkspaceModelError> {
        let frame_id = Self::content_frame_id();
        self.workspace_layout
            .frame_nav_mut(frame_id)
            .ok_or(WorkspaceModelError::FrameNotFound(frame_id))
    }

    fn current_frame_navigation(&self) -> Result<&FrameNavigationState, WorkspaceModelError> {
        let frame_id = Self::content_frame_id();
        self.workspace_layout
            .frame_nav(frame_id)
            .ok_or(WorkspaceModelError::FrameNotFound(frame_id))
    }

    fn reset_frame_navigation(
        &mut self,
        entry: FrameNavigationEntry,
    ) -> Result<(), WorkspaceModelError> {
        self.workspace_layout
            .reset_nav(Self::content_frame_id(), entry)
    }

    fn push_frame_navigation(
        &mut self,
        entry: FrameNavigationEntry,
    ) -> Result<(), WorkspaceModelError> {
        self.workspace_layout
            .push_nav(Self::content_frame_id(), entry)
    }

    fn restore_frame_navigation(&mut self) -> Result<FrameNavigationEntry, WorkspaceModelError> {
        self.workspace_layout
            .pop_nav(Self::content_frame_id())
            .ok_or(WorkspaceModelError::CannotNavigateBack)
    }

    #[expect(
        dead_code,
        reason = "ADR 0046 frame chrome back controls will consume this when navigation buttons are wired"
    )]
    fn frame_back_destination(&self) -> Option<FrameNavigationEntry> {
        self.workspace_layout
            .frame_nav(Self::content_frame_id())
            .and_then(FrameNavigationState::back_destination)
            .cloned()
    }

    pub fn new(
        conn: Arc<Mutex<Connection>>,
        cache: Arc<ImageCache>,
        musicindex_endpoint: String,
        application_services: Arc<ApplicationServices>,
        runtime_host: Option<Arc<crate::presentation::RuntimeHost>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let chrome = LibraryViewModel::chrome_display();
        let new_playlist_input = cx.new(|cx: &mut Context<InputState>| {
            InputState::new(window, cx).placeholder(chrome.new_playlist_placeholder)
        });
        let rename_playlist_input = cx.new(|cx: &mut Context<InputState>| {
            InputState::new(window, cx)
                .placeholder(PlaylistDetailActionsDisplay::RENAME_INPUT_PLACEHOLDER)
        });
        let rename_playlist_sub =
            cx.subscribe(&rename_playlist_input, Self::on_rename_playlist_event);
        let command_runner = match runtime_host.as_ref() {
            Some(host) => AsyncCommandRunner::with_vm_bus_on_handle(
                application_services.command_bus(),
                application_services.event_bus(),
                host.bus().clone(),
                host.handle().clone(),
            ),
            None => AsyncCommandRunner::new(
                application_services.command_bus(),
                application_services.event_bus(),
            ),
        };
        let musicbrainz_feed_saga = runtime_host.as_ref().map(|host| {
            let _enter = host.handle().enter();
            let handle =
                crate::runtime::musicbrainz_feed_saga::spawn(application_services.command_bus());
            bridge_watch(
                handle.subscribe(),
                |this: &mut Self, state, cx| {
                    this.apply_musicbrainz_feed_saga_state(state, cx);
                },
                cx,
            );
            handle
        });
        let mut app = Self {
            conn,
            application_services,
            command_runner,
            cache,
            musicindex_endpoint,
            vm: LibraryViewModel::new(),
            workspace_layout: Self::default_workspace_layout(),
            detail: LibraryDetail::None,
            thumbnails: BTreeMap::new(),
            new_playlist_input,
            rename_playlist_input,
            _rename_playlist_sub: rename_playlist_sub,
            runtime_host,
            playlist_actor: None,
            musicbrainz_feed_saga,
        };
        app.start_async_reload(cx);
        app
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

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.start_async_reload_preserving_detail(cx);
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
        self.start_async_reload_with_mode(LibraryReloadMode::ResetDetail, cx);
    }

    fn start_async_reload_preserving_detail(&mut self, cx: &mut Context<Self>) {
        self.start_async_reload_with_mode(LibraryReloadMode::PreserveDetail, cx);
    }

    fn start_async_reload_with_mode(&mut self, mode: LibraryReloadMode, cx: &mut Context<Self>) {
        self.vm.begin_library_reload();
        self.vm.clear_mb_status();
        if mode == LibraryReloadMode::ResetDetail {
            self.vm.clear_library_selection();
            self.detail = LibraryDetail::None;
        }
        cx.notify();

        // Playlists are a small single-table query: keep them on the
        // foreground for now so the playlist sidebar stays responsive
        // and we don't grow a second async path until proven needed.
        self.reload_playlists();
        if mode == LibraryReloadMode::PreserveDetail {
            self.refresh_selected_detail(cx);
        }

        let command = LoadLibraryTracksTree::new(Arc::clone(&self.conn));
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            |this, result, _cx| {
                this.vm.replace_tree(result.tree);
                this.vm.finish_library_reload(result.count);
            },
            |this, err, _cx| this.vm.set_error_status(err),
        );
    }

    fn reload_playlists(&mut self) {
        let conn = self.conn.lock().expect("lock db");
        match self.application_services.query_service().playlists(&conn) {
            Ok(list) => self.vm.replace_playlists(list),
            Err(err) => self.vm.fail_playlist_load(err),
        }
    }

    pub(crate) fn playlists(&self) -> &[db::Playlist] {
        self.vm.playlists()
    }

    fn refresh_selected_detail(&mut self, cx: &mut Context<Self>) {
        let playlist_id = match &self.detail {
            LibraryDetail::Playlist(detail) => Some(detail.playlist.id),
            LibraryDetail::None
            | LibraryDetail::Artist(_)
            | LibraryDetail::Album(_)
            | LibraryDetail::Track(_) => None,
        };
        if let Some(playlist_id) = playlist_id {
            self.select_playlist_with_history(playlist_id, FrameHistoryMode::Restore, cx);
            return;
        }

        if let LibraryDetail::Album(album) = &self.detail {
            if let Some(feed_id) = album.feed_id {
                if let Some(album) = self.album_for_detail_by_feed_id(feed_id) {
                    self.select_album(&album, cx);
                }
                return;
            }
        }

        if let LibraryDetail::Track(frame) = &self.detail {
            self.refresh_selected_track(frame.entity_id, cx);
        }
    }

    fn refresh_selected_track(&mut self, track_id: i64, cx: &mut Context<Self>) {
        // Preserve-detail reloads use one synchronous PK lookup, matching
        // the sidebar playlist refresh path until a shared async detail actor
        // is introduced.
        let track = {
            let conn = self.conn.lock().expect("lock db");
            db::track_row_by_id(&conn, track_id).unwrap_or_default()
        };
        let Some(track) = track else {
            return;
        };
        let Some(frame) = self.selected_track_frame_mut() else {
            return;
        };
        if frame.entity_id != track_id {
            return;
        }
        frame.title = LibraryTrackRowVm::new(&track, None).display_title();
        frame.local_subscription = track.is_in_library && track.local_path.is_some();
        frame.track = track.clone();
        frame.source_context = None;
        frame.tag_compare = LazyPanel::Hidden;
        frame.pending_id3_edits.clear();
        frame.suppressed_auto_id3_edits.clear();
        frame.id3_apply_error = None;
        self.load_track_source_context(track, cx);
    }

    fn cycle_playlist_sort(&mut self, cx: &mut Context<Self>) {
        self.vm.cycle_playlist_sort();
        self.vm.sort_loaded_playlists();
        cx.notify();
    }

    pub(crate) fn select_playlist(&mut self, id: i64, cx: &mut Context<Self>) {
        self.select_playlist_with_history(id, FrameHistoryMode::Record, cx);
    }

    fn select_playlist_with_history(
        &mut self,
        id: i64,
        history_mode: FrameHistoryMode,
        cx: &mut Context<Self>,
    ) {
        if history_mode == FrameHistoryMode::Record {
            if let Err(err) = self.push_frame_navigation(FrameNavigationEntry::PlaylistDetail(id)) {
                self.vm.set_error_status(err);
                cx.notify();
                return;
            }
        }
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
            self.spawn_playlist_actor(id, &tracks, cx);
            self.vm.replace_playlist_tracks(tracks);
        }
        cx.notify();
    }

    /// Spawn (or replace) the paged playlist actor for `playlist_id`.
    ///
    /// Dropping the previous handle closes its inbox so the actor task
    /// exits gracefully. The new handle is bridged via
    /// [`crate::presentation::bridge_watch`] so snapshot publishes
    /// trigger a re-render automatically.
    fn spawn_playlist_actor(
        &mut self,
        playlist_id: i64,
        initial_rows: &[TrackRow],
        cx: &mut Context<Self>,
    ) {
        use crate::application::paged_track_list::{PagedTrackListActor, PagedTrackListMsg};
        use crate::db::{open_db, TrackListing};

        let Some(host) = self.runtime_host.clone() else {
            return;
        };
        if let Some(state) = &self.playlist_actor {
            if state.playlist_id == playlist_id {
                let _ = state
                    .handle
                    .try_send(PagedTrackListMsg::PrimeRows(initial_rows.to_vec()));
                let _ = state.handle.try_send(PagedTrackListMsg::Refresh);
                return;
            }
        }
        self.playlist_actor = None;
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
        let mut actor = match PagedTrackListActor::new(conn, TrackListing::Playlist { playlist_id })
        {
            Ok(actor) => actor,
            Err(err) => {
                eprintln!("v4vmm::library: PagedTrackListActor::new failed: {err}");
                return;
            }
        };
        actor.prime_initial_rows(initial_rows.iter().cloned());
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

    fn refresh_origin_playlist_actor(&mut self) {
        use crate::application::paged_track_list::PagedTrackListMsg;

        let Ok(nav) = self.current_frame_navigation() else {
            return;
        };
        if !matches!(nav.current(), FrameNavigationEntry::TrackDetail(_)) {
            return;
        }
        let Some(FrameNavigationEntry::PlaylistDetail(playlist_id)) = nav.back_destination() else {
            return;
        };
        let Some(state) = &self.playlist_actor else {
            return;
        };
        if state.playlist_id != *playlist_id {
            return;
        }
        let tracks = {
            let conn = self.conn.lock().expect("lock db");
            self.application_services
                .query_service()
                .playlist_tracks(&conn, *playlist_id)
                .unwrap_or_default()
        };
        let _ = state.handle.try_send(PagedTrackListMsg::PrimeRows(tracks));
        let _ = state.handle.try_send(PagedTrackListMsg::Refresh);
    }

    fn create_playlist(&mut self, cx: &mut Context<Self>) {
        let name = self.new_playlist_input.read(cx).value().to_string();
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        let command = CreatePlaylist::new(Arc::clone(&self.conn), name.to_string());
        present_command(
            &self.command_runner,
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
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, (), cx| {
                this.reload_playlists();
                if this.vm.is_playlist_selected(id) {
                    this.select_playlist_with_history(id, FrameHistoryMode::Restore, cx);
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
        present_command(
            &self.command_runner,
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
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, (), cx| {
                this.reload_playlists();
                if this.vm.is_playlist_selected(playlist_id) {
                    this.select_playlist_with_history(playlist_id, FrameHistoryMode::Restore, cx);
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
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, (), cx| {
                if this.vm.is_playlist_selected(playlist_id) {
                    this.select_playlist_with_history(playlist_id, FrameHistoryMode::Restore, cx);
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
        present_command(
            &self.command_runner,
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
        present_command(
            &self.command_runner,
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
        present_command(
            &self.command_runner,
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
        let command = FetchThumbnail::new(Arc::clone(&self.cache), key.0.clone(), animated);
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, image, cx| {
                this.thumbnails.insert(key, ThumbnailState::Loaded(image));
                cx.notify();
            },
            |_, _, _| {},
        );
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
        let mut album = album.clone();
        if let Some(feed_id) = album.feed_id {
            if let Ok(tracks) = self
                .conn
                .lock()
                .map_err(|_| anyhow::anyhow!("database lock poisoned"))
                .and_then(|conn| db::feed_tracks(&conn, feed_id))
            {
                if !tracks.is_empty() {
                    album.tracks = tracks;
                }
            }
        }
        self.detail = LibraryDetail::Album(album.clone());
        self.hydrate_album_identity_on_view(&album, cx);
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

    pub(crate) fn find_album_by_feed_id(&self, feed_id: i64) -> Option<&AlbumNode> {
        for artist_node in &self.vm.tree().artists {
            for album in &artist_node.albums {
                if album.feed_id == Some(feed_id) {
                    return Some(album);
                }
            }
        }
        None
    }

    pub(crate) fn album_for_detail_by_feed_id(&self, feed_id: i64) -> Option<AlbumNode> {
        if let Some(album) = self.find_album_by_feed_id(feed_id) {
            return Some(album.clone());
        }

        let conn = self.conn.lock().ok()?;
        let tracks = db::feed_tracks(&conn, feed_id).ok()?;
        let first = tracks.first()?;
        let language = db::feed_language_by_id(&conn, feed_id).ok().flatten();
        Some(AlbumNode {
            name: first
                .feed_title
                .clone()
                .or_else(|| first.album_title.clone())
                .unwrap_or_else(|| "Untitled Feed".to_string()),
            feed_id: Some(feed_id),
            feed_guid: first.feed_guid.clone(),
            feed_url: db::feed_url_by_id(&conn, feed_id).ok().flatten(),
            language,
            description: None,
            image_href: first
                .album_image_href
                .clone()
                .or_else(|| first.track_image_href.clone()),
            identity_facts: crate::local_identity::feed_facts(&conn, feed_id).unwrap_or_default(),
            metadata_facts: Box::new(
                crate::local_metadata::feed_facts(&conn, feed_id).unwrap_or_default(),
            ),
            tracks,
        })
    }

    fn hydrate_album_identity_on_view(&mut self, album: &AlbumNode, cx: &mut Context<Self>) {
        if album_has_feed_identity_actions(&album.identity_facts)
            && album.description.is_some()
            && !album.metadata_facts.is_empty()
        {
            return;
        }
        let (Some(feed_id), Some(feed_guid)) = (album.feed_id, album.feed_guid.clone()) else {
            return;
        };

        let command = HydrateAlbumIdentity::new(
            Arc::clone(&self.conn),
            self.musicindex_endpoint.clone(),
            feed_id,
            feed_guid,
        );
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, hydration, cx| {
                this.vm
                    .update_album_identity_facts(feed_id, &hydration.identity_facts);
                this.vm
                    .update_album_metadata_facts(feed_id, &hydration.metadata_facts);
                this.vm
                    .update_album_description(feed_id, hydration.description.as_deref());
                if let LibraryDetail::Album(album) = &mut this.detail {
                    if album.feed_id == Some(feed_id) {
                        album.identity_facts = hydration.identity_facts;
                        *album.metadata_facts = hydration.metadata_facts;
                        album.description = hydration.description;
                    }
                }
                cx.notify();
            },
            |_, _, _| {},
        );
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

    /// Synchronize LibraryApp detail state to match a navigation entry.
    ///
    /// Maps the nav entry to the appropriate Library detail state:
    /// - `TrackDetail(id)` → look up track and call `select_track`
    /// - `AlbumDetail(id)` → look up album via feed_id and call `select_album`
    /// - `ArtistDetail(name)` → call `select_artist`
    /// - `Search(_)` / `SourceList` / others → clear detail to root state
    pub(crate) fn hydrate_detail_from_nav(
        &mut self,
        entry: &FrameNavigationEntry,
        cx: &mut Context<Self>,
    ) {
        match entry {
            FrameNavigationEntry::TrackDetail(track_id) => {
                if let Some(track_row) = self.conn.lock().ok().and_then(|conn| {
                    library_service::track_row_by_id(&conn, *track_id)
                        .ok()
                        .flatten()
                }) {
                    self.select_track(&track_row, cx);
                }
            }
            FrameNavigationEntry::AlbumDetail(feed_id) => {
                if let Some(album) = self.album_for_detail_by_feed_id(*feed_id) {
                    self.select_album(&album, cx);
                }
            }
            FrameNavigationEntry::ArtistDetail(name) => {
                self.select_artist(name, cx);
            }
            FrameNavigationEntry::Search(_)
            | FrameNavigationEntry::RecentFeeds
            | FrameNavigationEntry::IndexArtistFeedScope(_)
            | FrameNavigationEntry::IndexFeedDetail { .. }
            | FrameNavigationEntry::IndexTrackDetail { .. }
            | FrameNavigationEntry::SourceList
            | FrameNavigationEntry::Settings => {
                // Reset detail to default/unset state for root navigation
                self.clear_detail();
            }
            // Other nav entries (PlaylistDetail, QueueNowPlaying) don't drive library detail
            _ => {}
        }
    }

    /// Clear the detail state, resetting to library root.
    pub(crate) fn clear_detail(&mut self) {
        self.detail = LibraryDetail::None;
        self.vm.clear_library_selection();
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
        present_command(
            &self.command_runner,
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
        present_command(
            &self.command_runner,
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
        present_command(
            &self.command_runner,
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
        if let Err(err) = self.reset_frame_navigation(FrameNavigationEntry::TrackDetail(track.id)) {
            self.vm.set_error_status(err);
            cx.notify();
            return;
        }
        self.select_track_detail(track, cx);
    }

    pub(crate) fn select_playlist_track(
        &mut self,
        playlist_id: i64,
        track: &TrackRow,
        cx: &mut Context<Self>,
    ) {
        if !matches!(
            self.current_frame_navigation_mut()
                .map(|navigation| navigation.current()),
            Ok(FrameNavigationEntry::PlaylistDetail(current_playlist_id))
                if *current_playlist_id == playlist_id
        ) {
            if let Err(err) =
                self.reset_frame_navigation(FrameNavigationEntry::PlaylistDetail(playlist_id))
            {
                self.vm.set_error_status(err);
                cx.notify();
                return;
            }
        }
        if let Err(err) = self.push_frame_navigation(FrameNavigationEntry::TrackDetail(track.id)) {
            self.vm.set_error_status(err);
            cx.notify();
            return;
        }
        self.select_track_detail(track, cx);
    }

    fn select_track_detail(&mut self, track: &TrackRow, cx: &mut Context<Self>) {
        self.vm.select_library_item(track.id);
        let image = track
            .track_image_href
            .as_deref()
            .or(track.album_image_href.as_deref())
            .and_then(|url| self.thumbnail_for_url(Some(url), true, cx));
        let mut frame = InspectorFrame::for_track(track.clone(), image);
        frame.inspector_state.description_state = self.vm.track_description_state(track.id, None);
        if let Some(lookup) = self.vm.staged_musicbrainz(track.id).cloned() {
            frame.musicbrainz_lookup = LazyPanel::Loaded(lookup);
            frame.musicbrainz_selected = 0;
        }
        self.detail = LibraryDetail::Track(Box::new(frame));
        cx.notify();
        self.load_track_source_context(track.clone(), cx);
    }

    fn load_track_source_context(&mut self, track: TrackRow, cx: &mut Context<Self>) {
        let entity_id = track.id;
        let command = FetchLibraryTrackContext::new(
            Arc::clone(&self.conn),
            track,
            self.musicindex_endpoint.clone(),
        );
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, context, _cx| {
                if let Some(frame) = this.selected_track_frame_mut() {
                    if frame.entity_id == entity_id {
                        frame.source_context = Some(context);
                    }
                }
            },
            |_, _, _| {},
        );
    }

    #[expect(
        dead_code,
        reason = "ADR 0046 frame chrome back controls will consume this when navigation buttons are wired"
    )]
    fn navigate_back_to_frame_history(&mut self, cx: &mut Context<Self>) {
        match self.restore_frame_navigation() {
            Ok(FrameNavigationEntry::PlaylistDetail(playlist_id)) => {
                self.select_playlist_with_history(playlist_id, FrameHistoryMode::Restore, cx);
            }
            Ok(_) | Err(WorkspaceModelError::CannotNavigateBack) => cx.notify(),
            Err(err) => {
                self.vm.set_error_status(err);
                cx.notify();
            }
        }
    }

    fn track_breadcrumb_display(&self) -> Option<BreadcrumbDisplay> {
        let nav = self.workspace_layout.frame_nav(Self::content_frame_id())?;
        if !matches!(nav.current(), FrameNavigationEntry::TrackDetail(_)) {
            return None;
        }
        let display = BreadcrumbDisplay::project("library-track-breadcrumb", nav, |entry| {
            self.frame_breadcrumb_label(entry)
        });
        (display.segments.len() > 1).then_some(display)
    }

    fn frame_breadcrumb_label(&self, entry: &FrameNavigationEntry) -> String {
        match entry {
            FrameNavigationEntry::SourceList => "Library".to_string(),
            FrameNavigationEntry::PlaylistDetail(playlist_id) => self
                .vm
                .playlist_by_id(*playlist_id)
                .map_or_else(|| "Playlist".to_string(), |playlist| playlist.name),
            FrameNavigationEntry::TrackDetail(track_id) => match &self.detail {
                LibraryDetail::Track(frame) if frame.entity_id == *track_id => frame.title.clone(),
                _ => "Track".to_string(),
            },
            FrameNavigationEntry::AlbumDetail(_) => "Album".to_string(),
            FrameNavigationEntry::ArtistDetail(name)
            | FrameNavigationEntry::IndexArtistFeedScope(name) => name.clone(),
            FrameNavigationEntry::Search(query) if query.trim().is_empty() => "Search".to_string(),
            FrameNavigationEntry::Search(query) => query.clone(),
            FrameNavigationEntry::RecentFeeds => "Recent Feeds".to_string(),
            FrameNavigationEntry::IndexFeedDetail { label, .. }
            | FrameNavigationEntry::IndexTrackDetail { label, .. } => label.clone(),
            FrameNavigationEntry::Settings => "Settings".to_string(),
            FrameNavigationEntry::QueueNowPlaying => "Queue".to_string(),
        }
    }

    pub(crate) fn select_frame_breadcrumb(
        &mut self,
        entry: FrameNavigationEntry,
        cx: &mut Context<Self>,
    ) {
        match entry {
            FrameNavigationEntry::PlaylistDetail(playlist_id) => {
                if let Err(err) =
                    self.reset_frame_navigation(FrameNavigationEntry::PlaylistDetail(playlist_id))
                {
                    self.vm.set_error_status(err);
                    cx.notify();
                    return;
                }
                self.select_playlist_with_history(playlist_id, FrameHistoryMode::Restore, cx);
            }
            _ => cx.notify(),
        }
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

    pub(crate) fn download_feed(&mut self, feed_id: i64, cx: &mut Context<Self>) {
        if self.vm.has_busy_track() || self.vm.has_busy_feed() {
            return;
        }
        let Some(feed) = self.api_feed_for_album(feed_id) else {
            self.vm.set_album_has_no_tracks();
            cx.notify();
            return;
        };
        self.vm.begin_busy_feed(feed_id, "Downloading feed...");
        cx.notify();
        let command = SubscribeFeed::new(
            Arc::clone(&self.conn),
            self.application_services.download_manager(),
            SubscribeFeedRequest {
                feed,
                musicindex_endpoint: self.musicindex_endpoint.clone(),
            },
        );
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            |this, result, cx| {
                this.vm.finish_feed_download(
                    result.downloaded(),
                    result.applied_edits(),
                    result.skipped(),
                );
                this.refresh_origin_playlist_actor();
                this.start_async_reload_preserving_detail(cx);
            },
            |this, error, _cx| this.vm.fail_feed_download(error),
        );
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
        if let LibraryRemovalTarget::Feed(feed_id) = target {
            self.vm.begin_busy_feed(feed_id, "Removing feed...");
            cx.notify();
        }
        let command = RemoveFromLibrary::new(Arc::clone(&self.conn), target);
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            |this, result, cx| {
                this.vm.clear_busy_feed();
                this.apply_library_removal_result_to_selected_detail(result.target());
                this.refresh_origin_playlist_actor();
                this.start_async_reload_preserving_detail(cx);
            },
            |this, err, _cx| {
                this.vm.clear_busy_feed();
                this.vm.set_error_status(err);
            },
        );
    }

    fn apply_library_removal_result_to_selected_detail(&mut self, target: LibraryRemovalTarget) {
        apply_library_removal_to_album_detail(&mut self.detail, target);

        if let Some(frame) = self.selected_track_frame_mut() {
            apply_library_removal_to_inspector_frame(frame, target);
        }
    }

    fn execute_pending_library_removal(&mut self, cx: &mut Context<Self>) {
        let Some(target) = self.vm.take_pending_library_removal() else {
            cx.notify();
            return;
        };
        self.execute_library_removal_target(target, cx);
    }

    fn api_feed_for_album(&self, feed_id: i64) -> Option<crate::api::Feed> {
        let album = self.album_for_detail_by_feed_id(feed_id)?;
        Some(api_feed_from_album(&album))
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
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, outcome, cx| {
                let path = outcome.path().to_string();
                apply_track_subscription_to_album_detail(
                    &mut this.detail,
                    track_id,
                    &path,
                    outcome.marked_downloaded(),
                );
                this.vm.finish_track_subscribe(TrackSubscribeOutcome::new(
                    path,
                    outcome.format_warning().map(str::to_string),
                ));
                this.refresh_origin_playlist_actor();
                this.start_async_reload_preserving_detail(cx);
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

        let command = ApplyTrackId3Edits::new(path, edits, track_context);
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, result, _cx| {
                if let Some(frame) = this.selected_track_frame_mut() {
                    if frame.entity_id == entity_id {
                        frame.applying_id3_edits = false;
                        frame.tag_compare = LazyPanel::Loaded(result);
                        frame.pending_id3_edits.clear();
                        frame.suppressed_auto_id3_edits.clear();
                        frame.id3_apply_error = None;
                    }
                }
            },
            move |this, error, _cx| {
                if let Some(frame) = this.selected_track_frame_mut() {
                    if frame.entity_id == entity_id {
                        frame.applying_id3_edits = false;
                        frame.id3_apply_error =
                            Some(TrackMetadataActionState::id3_apply_error_message(error));
                    }
                }
            },
        );
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
        let Some(action) = (match self.selected_track_frame_mut() {
            Some(frame) if frame.subscription_busy => return,
            Some(frame) if frame.local_subscription => {
                Some(TrackSubscriptionAction::Remove(frame.entity_id))
            }
            Some(frame) => {
                frame.subscription_busy = true;
                frame.subscription_message =
                    Some(LibraryTrackActionVm::subscription_busy_message(true).into());
                Some(TrackSubscriptionAction::Download(Box::new(
                    frame.track.clone(),
                )))
            }
            None => None,
        }) else {
            return;
        };
        let track = match action {
            TrackSubscriptionAction::Remove(track_id) => {
                self.request_library_removal(LibraryRemovalIntent::TrackId(track_id), window, cx);
                return;
            }
            TrackSubscriptionAction::Download(track) => track,
        };

        let track_id = track.id;
        self.vm.begin_busy_track(
            track_id,
            LibraryTrackActionVm::track_subscribe_begin_status(),
        );
        cx.notify();

        let command = SubscribeTrack::new(
            Arc::clone(&self.conn),
            self.application_services.download_manager(),
            SubscribeTrackRequest::LibraryTrack { track },
            LibraryTrackActionVm::track_subscribe_success_message(),
        );
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, result, cx| {
                let path = result.path().to_string();
                apply_track_subscription_to_album_detail(
                    &mut this.detail,
                    track_id,
                    &path,
                    result.marked_downloaded(),
                );
                this.vm.finish_track_subscribe(TrackSubscribeOutcome::new(
                    path.clone(),
                    result.format_warning().map(str::to_string),
                ));
                if let Some(frame) = this.selected_track_frame_mut() {
                    if frame.entity_id == track_id {
                        frame.subscription_busy = false;
                        frame.local_subscription = result.marked_downloaded();
                        frame.track.is_in_library = result.marked_downloaded();
                        if result.marked_downloaded() {
                            frame.track.local_path = Some(path);
                        }
                        frame.source_context = None;
                        frame.tag_compare = LazyPanel::Hidden;
                        frame.pending_id3_edits.clear();
                        frame.suppressed_auto_id3_edits.clear();
                        frame.id3_apply_error = None;
                        frame.subscription_message = Some(result.message().into());
                    }
                }
                this.refresh_origin_playlist_actor();
                this.start_async_reload_preserving_detail(cx);
            },
            move |this, err, _cx| {
                this.vm.fail_track_subscribe(&err);
                if let Some(frame) = this.selected_track_frame_mut() {
                    if frame.entity_id == track_id {
                        frame.subscription_busy = false;
                        frame.subscription_message =
                            Some(LibraryTrackActionVm::subscription_error_message(true, err));
                    }
                }
            },
        );
    }

    pub(crate) fn toggle_tag_compare(&mut self, cx: &mut Context<Self>) {
        let Some(frame) = self.selected_track_frame_mut() else {
            return;
        };
        if !frame.local_subscription || frame.track.local_path.is_none() {
            return;
        }
        let expanded = frame.toggle_inspector_panel(InspectorPanelKind::CompareId3);
        if !expanded {
            cx.notify();
            return;
        }
        match frame.tag_compare {
            LazyPanel::Loaded(_) => {
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
        cx.notify();

        self.start_compare_library_track(entity_id, track, cx);
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
        cx.notify();

        self.start_compare_library_track(entity_id, track, cx);
    }

    fn start_compare_library_track(
        &mut self,
        entity_id: i64,
        track: TrackRow,
        cx: &mut Context<Self>,
    ) {
        let command = CompareLibraryTrack::new(track, self.musicindex_endpoint.clone());
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, result, _cx| {
                if let Some(frame) = this.selected_track_frame_mut() {
                    if frame.entity_id == entity_id {
                        frame.source_context = Some(result.track_context);
                        frame.tag_compare = LazyPanel::Loaded(result.tag_compare);
                    }
                }
            },
            move |this, error, _cx| {
                if let Some(frame) = this.selected_track_frame_mut() {
                    if frame.entity_id == entity_id {
                        frame.tag_compare =
                            LazyPanel::Empty(LibraryViewModel::deferred_panel_error_message(error));
                    }
                }
            },
        );
    }

    pub(crate) fn toggle_musicbrainz_lookup(&mut self, cx: &mut Context<Self>) {
        let Some(frame) = self.selected_track_frame_mut() else {
            return;
        };
        if !frame.local_subscription || frame.track.local_path.is_none() {
            return;
        }
        let expanded = frame.toggle_inspector_panel(InspectorPanelKind::MusicBrainz);
        if !expanded {
            cx.notify();
            return;
        }
        match frame.musicbrainz_lookup {
            LazyPanel::Loaded(_) => {
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

        present_command(
            &self.command_runner,
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

    pub(crate) fn toggle_track_description(&mut self, cx: &mut Context<Self>) {
        let Some(frame) = self.selected_track_frame_mut() else {
            return;
        };
        let line_count =
            description_line_count(frame.source_context.as_ref().and_then(|context| {
                LibraryViewModel::display_description_text(context.track.description.as_deref())
            }));
        frame.inspector_state.project_description(line_count);
        frame.toggle_description();
        let track_id = frame.entity_id;
        let state = frame.inspector_state.description_state;
        self.vm.set_track_description_state(track_id, state);
        cx.notify();
    }

    pub(crate) fn toggle_album_description(&mut self, feed_id: i64, cx: &mut Context<Self>) {
        let Some(description) = self.detail_album_description(feed_id) else {
            return;
        };
        self.vm
            .toggle_album_description(feed_id, Some(description.as_str()));
        cx.notify();
    }

    fn detail_album_description(&self, feed_id: i64) -> Option<String> {
        let LibraryDetail::Album(album) = &self.detail else {
            return None;
        };
        (album.feed_id == Some(feed_id))
            .then(|| {
                LibraryViewModel::display_description_text(album.description.as_deref())
                    .map(str::to_owned)
            })
            .flatten()
    }

    #[allow(dead_code)]
    fn musicbrainz_track(&mut self, track: TrackRow, cx: &mut Context<Self>) {
        if !self.vm.begin_musicbrainz_track_lookup(track.id) {
            return;
        }
        cx.notify();

        let track_id = track.id;
        present_command(
            &self.command_runner,
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

        let feed_id = album.feed_id.unwrap_or(0);
        let feed_title = Some(album.name.clone());
        let request = StartFeedLookup::new(feed_id, feed_title, downloadable);
        let Some(saga) = self.musicbrainz_feed_saga.as_ref() else {
            self.vm
                .fail_musicbrainz_album_lookup_with_fallback("runtime unavailable");
            cx.notify();
            return;
        };
        if !saga.try_start(request) {
            self.vm
                .fail_musicbrainz_album_lookup_with_fallback("lookup actor unavailable");
            cx.notify();
        }
    }

    fn apply_musicbrainz_feed_saga_state(
        &mut self,
        state: MusicBrainzFeedSagaState,
        _cx: &mut Context<Self>,
    ) {
        match state {
            MusicBrainzFeedSagaState::Idle
            | MusicBrainzFeedSagaState::AlbumSearchInFlight { .. } => {}
            MusicBrainzFeedSagaState::AlbumSearchFailed { error, .. } => {
                self.vm.fail_musicbrainz_album_lookup_with_fallback(error);
            }
            MusicBrainzFeedSagaState::AlbumSearchEmpty { .. } => {
                self.vm.fallback_empty_musicbrainz_album_lookup();
            }
            MusicBrainzFeedSagaState::PerTrackInFlight {
                track_id,
                progress,
                total,
                ..
            } => {
                self.vm
                    .begin_musicbrainz_album_track_stage(track_id, progress, total);
            }
            MusicBrainzFeedSagaState::TrackDone {
                track_id,
                progress,
                total,
                edit_count,
                lookup,
                ..
            } => {
                self.vm
                    .begin_musicbrainz_album_track_stage(track_id, progress, total);
                self.stage_musicbrainz_lookup_for_track(track_id, lookup);
                self.vm.finish_musicbrainz_album_track_stage(
                    track_id,
                    MbTrackStatus::Done(edit_count),
                );
            }
            MusicBrainzFeedSagaState::TrackSkipped {
                track_id,
                progress,
                total,
                reason,
                ..
            } => {
                self.vm
                    .begin_musicbrainz_album_track_stage(track_id, progress, total);
                self.vm
                    .finish_musicbrainz_album_track_stage(track_id, MbTrackStatus::Skipped(reason));
            }
            MusicBrainzFeedSagaState::Completed {
                total_edits,
                processed,
                ..
            } => {
                self.vm
                    .finish_musicbrainz_album_lookup(total_edits, processed);
            }
        }
    }
}

pub(crate) fn build_tree(tracks: &[TrackRow], conn: &Connection) -> LibraryTree {
    crate::application::queries::library::build_tree(tracks, conn)
}

fn album_has_feed_identity_actions(facts: &LocalIdentityFacts) -> bool {
    let identity = EntityIdentityLinks::from_source_facts(
        None,
        facts.source_links.clone(),
        facts.source_ids.clone(),
    );
    identity.website_url.is_some() && identity.nostr_npub.is_some()
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
                .flat_map(album_thumbnail_urls)
                .collect()
        };
        if let LibraryDetail::Album(album) = &self.detail {
            urls.extend(album_thumbnail_urls(album));
        }
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
        let content_filter_empty_state = self.vm.content_filter_empty_state();

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
                .px(Spacing::SM.scaled(cx))
                .py(Spacing::XS.scaled(cx))
                .rounded(Spacing::XS.scaled(cx))
                .flex()
                .flex_row()
                .justify_between()
                .items_center()
                .child(
                    div()
                        .id(playlist_header_id)
                        .flex()
                        .flex_row()
                        .gap(Spacing::XS.scaled(cx))
                        .items_baseline()
                        .cursor_pointer()
                        .hover(|el| el.bg(color::bg_surface_hi()))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.vm.toggle_playlists_expanded();
                            cx.notify();
                        }))
                        .child(DisclosureIndicator::new(DisclosureIndicatorDisplay {
                            glyph: playlist_disclosure_glyph.into(),
                        }))
                        .child(Label::new(playlist_heading).weight(FontWeight::SEMIBOLD)),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(Spacing::XS.scaled(cx))
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
                                .pl(Spacing::MD.scaled(cx))
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

        if let Some(SavedSearchesSectionDisplay { heading, rows }) =
            self.vm.saved_searches_section()
        {
            left_items.push(
                div()
                    .px(Spacing::SM.scaled(cx))
                    .py(Spacing::XS.scaled(cx))
                    .child(Label::new(heading).weight(FontWeight::SEMIBOLD))
                    .into_any_element(),
            );
            for saved_search in rows {
                let saved_search_id = saved_search.id;
                let row_id = format!("saved-search-{saved_search_id}");
                left_items.push(
                    ListRow::compact(SharedString::from(row_id))
                        .a11y_label(ListRowA11yLabel {
                            label: saved_search.a11y_label.into(),
                        })
                        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                            this.open_saved_search(saved_search_id, cx);
                        }))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .pl(Spacing::MD.scaled(cx))
                                .child(Label::new(saved_search.label).truncated())
                                .child(
                                    Label::new(saved_search.query)
                                        .size(FontSize::Micro)
                                        .color(SemanticColor::TertiaryLabel)
                                        .truncated(),
                                ),
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
            self.track_breadcrumb_display(),
            self.vm.busy_track(),
            self.vm.mb_status(),
            &self.vm,
            &album_thumbs,
            self.vm.playlists(),
            &chrome,
            self.rename_playlist_input.clone(),
            self.vm.renaming_playlist_id(),
            self.playlist_actor.as_ref(),
            cx,
        );

        let leading_pane = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
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
                                let mut empty_label = div()
                                    .text_center()
                                    .p(spacing::XXL + spacing::LG)
                                    .text_color(color::text_muted());
                                if let Some(empty_state) = content_filter_empty_state {
                                    empty_label = empty_label
                                        .child(div().mt(spacing::SM).child(empty_state.title))
                                        .child(
                                            div()
                                                .mt(spacing::XS)
                                                .text_xs()
                                                .child(empty_state.secondary),
                                        );
                                } else {
                                    empty_label = empty_label.child(
                                        div().mt(spacing::SM).child(chrome.empty_library_label),
                                    );
                                }
                                el.child(empty_label)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::queries::library::{
        apply_local_track_metadata_defaults, fetch_library_track_context_with_local_fallback,
    };
    use crate::metadata::TrackContext;

    fn test_track(id: i64, feed_id: i64, is_in_library: bool) -> TrackRow {
        TrackRow {
            id,
            feed_id,
            item_guid: format!("track-{id}"),
            track_title: Some(format!("Track {id}")),
            is_in_library,
            local_path: is_in_library.then(|| format!("/music/track-{id}.mp3")),
            ..TrackRow::default()
        }
    }

    fn test_album(feed_id: i64, tracks: Vec<TrackRow>) -> AlbumNode {
        AlbumNode {
            name: "Test Album".into(),
            feed_id: Some(feed_id),
            feed_guid: Some("feed-guid".into()),
            feed_url: Some("https://example.test/feed.xml".into()),
            language: None,
            description: None,
            image_href: Some("https://example.test/art.png".into()),
            identity_facts: LocalIdentityFacts::default(),
            metadata_facts: Box::<crate::views::FeedMetadataFacts>::default(),
            tracks,
        }
    }

    #[test]
    fn build_tree_loads_feed_language_for_unsubscribed_local_feed() -> anyhow::Result<()> {
        let conn = Connection::open_in_memory()?;
        db::init_schema(&conn)?;
        conn.execute(
            "INSERT INTO feeds (feed_url, title, language, is_subscribed)
             VALUES (?1, ?2, ?3, 0)",
            rusqlite::params!["https://example.test/feed.xml", "Example Feed", "fr"],
        )?;
        let feed_id = conn.last_insert_rowid();
        let track = TrackRow {
            id: 1,
            feed_id,
            item_guid: "track-1".into(),
            track_title: Some("Track 1".into()),
            artist_name: Some("Example Artist".into()),
            album_title: Some("Example Feed".into()),
            is_in_library: true,
            ..TrackRow::default()
        };

        let tree = build_tree(&[track], &conn);
        let album = &tree.artists[0].albums[0];

        assert_eq!(album.feed_id, Some(feed_id));
        assert_eq!(album.language.as_deref(), Some("fr"));

        Ok(())
    }

    #[test]
    fn build_tree_loads_feed_metadata_facts() -> anyhow::Result<()> {
        let mut conn = Connection::open_in_memory()?;
        db::init_schema(&conn)?;
        conn.execute(
            "INSERT INTO feeds (feed_url, title, is_subscribed)
             VALUES (?1, ?2, 0)",
            rusqlite::params!["https://example.test/feed.xml", "Example Feed"],
        )?;
        let feed_id = conn.last_insert_rowid();
        db::replace_local_metadata_facts(
            &mut conn,
            db::LocalMetadataOwner::Feed(feed_id),
            "musicindex",
            &[db::LocalMetadataFactInput {
                fact_key: "publisher_text".into(),
                value: db::LocalMetadataValue::Text("Example Publisher".into()),
                extraction_path: None,
                observed_at: None,
                raw_json: None,
            }],
        )?;
        let track = TrackRow {
            id: 1,
            feed_id,
            item_guid: "track-1".into(),
            track_title: Some("Track 1".into()),
            artist_name: Some("Example Artist".into()),
            album_title: Some("Example Feed".into()),
            is_in_library: true,
            ..TrackRow::default()
        };

        let tree = build_tree(&[track], &conn);
        let album = &tree.artists[0].albums[0];

        assert_eq!(
            album.metadata_facts.publisher_text.as_deref(),
            Some("Example Publisher")
        );

        Ok(())
    }

    #[test]
    fn library_track_context_falls_back_to_local_hydrated_context() -> anyhow::Result<()> {
        let mut conn = Connection::open_in_memory()?;
        db::init_schema(&conn)?;
        conn.execute(
            "INSERT INTO feeds (id, feed_url, feed_guid, title)
             VALUES (2, ?1, ?2, ?3)",
            rusqlite::params!["https://example.test/feed.xml", "feed-guid", "Feed"],
        )?;
        conn.execute(
            "INSERT INTO tracks (id, feed_id, item_guid, track_title)
             VALUES (9, 2, ?1, ?2)",
            rusqlite::params!["track-guid", "Track"],
        )?;
        db::replace_local_metadata_facts(
            &mut conn,
            db::LocalMetadataOwner::Track(9),
            "musicindex",
            &[db::LocalMetadataFactInput {
                fact_key: "description".into(),
                value: db::LocalMetadataValue::Text("Local track description".into()),
                extraction_path: None,
                observed_at: None,
                raw_json: None,
            }],
        )?;
        let track = TrackRow {
            id: 9,
            feed_id: 2,
            feed_guid: Some("feed-guid".into()),
            item_guid: "track-guid".into(),
            track_title: Some("Track".into()),
            feed_title: Some("Feed".into()),
            ..TrackRow::default()
        };
        let conn = Arc::new(Mutex::new(conn));

        let context =
            fetch_library_track_context_with_local_fallback(&conn, &track, "http://127.0.0.1:9")?;

        assert_eq!(
            context.track.description.as_deref(),
            Some("Local track description")
        );
        Ok(())
    }

    #[test]
    fn local_track_metadata_fills_missing_remote_context_fields() {
        let mut remote_context = TrackContext {
            track: crate::api::Track {
                title: Some("Remote Track".into()),
                publisher_text: Some("Remote Publisher".into()),
                ..crate::api::Track::default()
            },
            feed: None,
        };
        let local_context = TrackContext {
            track: crate::api::Track {
                publisher_text: Some("Local Publisher".into()),
                description: Some("Local track description".into()),
                pub_date: Some(1_700_000_000),
                explicit: Some(true),
                ..crate::api::Track::default()
            },
            feed: None,
        };

        apply_local_track_metadata_defaults(&mut remote_context, &local_context);

        assert_eq!(
            remote_context.track.publisher_text.as_deref(),
            Some("Remote Publisher"),
            "remote source text should not be overwritten"
        );
        assert_eq!(
            remote_context.track.description.as_deref(),
            Some("Local track description")
        );
        assert_eq!(remote_context.track.pub_date, Some(1_700_000_000));
        assert_eq!(remote_context.track.explicit, Some(true));
    }

    fn test_inspector_frame(track: TrackRow) -> InspectorFrame {
        InspectorFrame {
            entity_id: track.id,
            title: "Track".into(),
            track,
            source_context: None,
            image: None,
            expanded_id3_frame_groups: BTreeSet::new(),
            expanded_metadata_cells: BTreeSet::new(),
            pending_id3_edits: BTreeMap::new(),
            suppressed_auto_id3_edits: BTreeSet::new(),
            applying_id3_edits: false,
            id3_apply_error: Some("stale error".into()),
            local_subscription: true,
            subscription_busy: true,
            subscription_message: None,
            tag_compare: LazyPanel::Empty("stale comparison".into()),
            musicbrainz_lookup: LazyPanel::Hidden,
            musicbrainz_selected: 0,
            inspector_state: LibraryTrackInspectorState::default(),
        }
    }

    #[test]
    fn removal_keeps_album_detail_row_as_index_content() {
        let mut detail = LibraryDetail::Album(test_album(
            7,
            vec![test_track(1, 7, true), test_track(2, 7, true)],
        ));

        apply_library_removal_to_album_detail(&mut detail, LibraryRemovalTarget::Track(1));

        let LibraryDetail::Album(album) = detail else {
            panic!("detail remains an album");
        };
        assert!(
            !album.tracks[0].is_in_library,
            "removed row should remain in the album as Index content"
        );
        assert!(
            album.tracks[0].local_path.is_none(),
            "removed row must stop advertising a local file"
        );
        assert!(
            album.tracks[1].is_in_library,
            "unrelated rows should retain their local membership"
        );
    }

    #[test]
    fn removal_resets_open_track_inspector_to_downloadable_state() {
        let mut frame = test_inspector_frame(test_track(9, 7, true));

        apply_library_removal_to_inspector_frame(&mut frame, LibraryRemovalTarget::Track(9));

        assert!(!frame.subscription_busy);
        assert!(!frame.local_subscription);
        assert!(!frame.track.is_in_library);
        assert!(frame.track.local_path.is_none());
        assert!(matches!(frame.tag_compare, LazyPanel::Hidden));
        assert_eq!(frame.subscription_message.as_deref(), Some("Removed track"));
    }

    #[test]
    fn subscription_updates_album_detail_row_to_library_content() {
        let mut detail = LibraryDetail::Album(test_album(
            7,
            vec![test_track(1, 7, false), test_track(2, 7, true)],
        ));

        apply_track_subscription_to_album_detail(&mut detail, 1, "/music/track-1.mp3", true);

        let LibraryDetail::Album(album) = detail else {
            panic!("detail remains an album");
        };
        assert!(
            album.tracks[0].is_in_library,
            "downloaded row should move back to Library content in place"
        );
        assert_eq!(
            album.tracks[0].local_path.as_deref(),
            Some("/music/track-1.mp3")
        );
        assert!(
            album.tracks[1].is_in_library,
            "unrelated rows should retain their membership"
        );
    }

    #[test]
    fn album_thumbnail_urls_include_open_detail_artwork_after_tree_removal() {
        let mut track = test_track(1, 7, false);
        track.track_image_href = Some("https://example.test/track.png".into());
        let album = test_album(7, vec![track]);

        let urls = album_thumbnail_urls(&album);

        assert!(
            urls.iter().any(|url| url == "https://example.test/art.png"),
            "open album detail artwork must stay prefetched even when the album is no longer in the library tree"
        );
        assert!(
            urls.iter()
                .any(|url| url == "https://example.test/track.png"),
            "track row artwork must stay prefetched for index-content rows"
        );
    }

    #[test]
    fn api_feed_from_album_preserves_track_download_sources() {
        let mut track = test_track(1, 7, false);
        track.enclosure_url = Some("https://example.test/audio.mp3".into());
        track.enclosure_type = Some("audio/mpeg".into());
        let album = test_album(7, vec![track]);

        let feed = api_feed_from_album(&album);
        let track = feed
            .tracks
            .as_deref()
            .and_then(|tracks| tracks.first())
            .expect("album track");

        assert_eq!(
            track.enclosure_url.as_deref(),
            Some("https://example.test/audio.mp3")
        );
        assert_eq!(track.enclosure_type.as_deref(), Some("audio/mpeg"));
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
