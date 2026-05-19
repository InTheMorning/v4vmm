#![warn(clippy::pedantic)]
#![expect(
    clippy::doc_markdown,
    clippy::manual_let_else,
    clippy::needless_pass_by_value,
    clippy::single_match_else,
    clippy::too_many_lines,
    clippy::unused_self,
    reason = "legacy screen module is being migrated incrementally under ADR 0023"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use gpui::{Entity, Image};
use gpui_component::input::InputState;

use crate::application::paged_track_list::{PagedTrackListMsg, PagedTrackListSnapshot};
use crate::application::{ApplicationServices, AsyncCommandRunner};
use crate::db::{self, TrackRow};
use crate::media::ImageCache;
use crate::metadata::{MusicBrainzLookupResult, PendingId3Edit, TagCompareResult, TrackContext};
use crate::presentation::RuntimeHost;
use crate::runtime::actor::ActorHandle;
use crate::runtime::musicbrainz_feed_saga::MusicBrainzFeedSagaHandle;
use crate::view_models::library::{
    description_line_count, AlbumNode, InspectorPanelKind, LibraryTrackInspectorDisplay,
    LibraryTrackInspectorState, LibraryViewModel,
};
use crate::view_models::workspace::WorkspaceLayout;
use crate::views::ArtistView;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub(crate) enum LibraryDetail {
    None,
    Artist(Box<LibraryArtistDetail>),
    Album(AlbumNode),
    Track(Box<InspectorFrame>),
    Playlist(PlaylistDetail),
}

#[derive(Clone, Debug)]
pub(crate) struct LibraryArtistDetail {
    pub(crate) name: String,
    pub(crate) view: ArtistView,
    pub(crate) tracks: Vec<TrackRow>,
}

#[derive(Clone, Debug)]
pub(crate) struct PlaylistDetail {
    pub(crate) playlist: db::Playlist,
    pub(crate) tracks: Vec<TrackRow>,
}

#[derive(Clone, Debug)]
pub enum LibraryAppEvent {
    PlayPlaylistAt {
        playlist_id: i64,
        playlist_position: i64,
    },
    OpenSavedSearch {
        saved_search_id: i64,
        query: String,
    },
}

impl gpui::EventEmitter<LibraryAppEvent> for LibraryApp {}

#[derive(Clone, Debug, Default)]
pub(crate) enum LazyPanel<T> {
    #[default]
    Hidden,
    Loading,
    Empty(String),
    Loaded(T),
}

#[derive(Clone, Debug)]
pub(crate) struct InspectorFrame {
    pub(crate) entity_id: i64,
    pub(crate) title: String,
    pub(crate) track: TrackRow,
    pub(crate) source_context: Option<TrackContext>,
    pub(crate) image: Option<Arc<Image>>,
    pub(crate) expanded_id3_frame_groups: BTreeSet<String>,
    pub(crate) expanded_metadata_cells: BTreeSet<String>,
    pub(crate) pending_id3_edits: BTreeMap<String, PendingId3Edit>,
    pub(crate) suppressed_auto_id3_edits: BTreeSet<String>,
    pub(crate) applying_id3_edits: bool,
    pub(crate) id3_apply_error: Option<String>,
    pub(crate) local_subscription: bool,
    pub(crate) subscription_busy: bool,
    pub(crate) subscription_message: Option<String>,
    pub(crate) tag_compare: LazyPanel<TagCompareResult>,
    pub(crate) musicbrainz_lookup: LazyPanel<MusicBrainzLookupResult>,
    pub(crate) musicbrainz_selected: usize,
    pub(crate) inspector_state: LibraryTrackInspectorState,
}

impl InspectorFrame {
    pub(crate) fn inspector_display(
        &self,
        description: Option<&str>,
    ) -> LibraryTrackInspectorDisplay {
        let description = LibraryViewModel::display_description_text(description);
        let mut display = self
            .inspector_state
            .display(self.local_subscription && self.track.local_path.is_some());
        display.description_state = display
            .description_state
            .project_sticky(description_line_count(description));
        display
    }

    pub(crate) fn toggle_inspector_panel(&mut self, kind: InspectorPanelKind) -> bool {
        self.inspector_state.toggle_panel(kind);
        self.inspector_state.is_panel_expanded(kind)
    }

    pub(crate) fn toggle_description(&mut self) {
        self.inspector_state.toggle_description();
    }
}

#[derive(Clone)]
enum ThumbnailState {
    Loading,
    Loaded(Option<Arc<Image>>),
}

pub struct LibraryApp {
    conn: Arc<Mutex<Connection>>,
    application_services: Arc<ApplicationServices>,
    command_runner: AsyncCommandRunner,
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
    workspace_layout: WorkspaceLayout,
    detail: LibraryDetail,
    thumbnails: BTreeMap<(String, bool), ThumbnailState>,
    new_playlist_input: Entity<InputState>,
    rename_playlist_input: Entity<InputState>,
    _rename_playlist_sub: gpui::Subscription,
    /// Optional async runtime host (ADR 0040). When present, screens
    /// can spawn paged-track-list actors that publish snapshots back
    /// via `presentation::bridge_watch`.
    pub(crate) runtime_host: Option<Arc<RuntimeHost>>,
    /// Currently spawned paged playlist actor for the active
    /// `LibraryDetail::Playlist`. Replaced on each `select_playlist`;
    /// dropping the previous handle closes the actor's inbox so it
    /// exits gracefully.
    pub(crate) playlist_actor: Option<PlaylistActorState>,
    /// Feed-level MusicBrainz lookup saga actor. Dropping the handle
    /// closes the inbox and lets the runtime task exit.
    musicbrainz_feed_saga: Option<MusicBrainzFeedSagaHandle>,
}

pub(crate) struct PlaylistActorState {
    pub(crate) playlist_id: i64,
    pub(crate) snapshot: PagedTrackListSnapshot,
    #[allow(dead_code)]
    pub(crate) handle: ActorHandle<PagedTrackListMsg, PagedTrackListSnapshot>,
}

mod app_impl;

pub(crate) use app_impl::{build_tree, playlist_options};
