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
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use gpui::{Entity, FocusHandle, Image, ScrollHandle};
use gpui_component::input::InputState;

use crate::api::{ArtistNameRow, Feed};
use crate::application::paged_track_list::{PagedTrackListMsg, PagedTrackListSnapshot};
use crate::application::{ApplicationServices, AsyncCommandRunner};
use crate::db::{self, TrackRow};
use crate::media::ImageCache;
use crate::metadata::{MusicBrainzLookupResult, PendingId3Edit, TagCompareResult, TrackContext};
use crate::presentation::collection_art::{
    CollectionArtHost, CollectionArtLoads, CollectionArtParts,
};
use crate::presentation::RuntimeHost;
use crate::runtime::actor::ActorHandle;
use crate::runtime::musicbrainz_feed_saga::MusicBrainzFeedSagaHandle;
use crate::ui::composites::music_column_scroll::ScrollReveal;
use crate::view_models::detail_selection::DetailSelection;
use crate::view_models::index_name_list::{IndexNameListPage, MusicListScope};
use crate::view_models::library::{
    description_line_count, AlbumNode, InspectorPanelKind, LibraryTrackInspectorDisplay,
    LibraryTrackInspectorState, LibraryViewModel, ReleasesRowId,
};
use crate::view_models::library_removal::{
    SelectionRemovalConfirmationState, SelectionRemovalReport,
};
use crate::view_models::list_selection::{ListSelection, SelectionMode};
use crate::view_models::recent_feeds::RecentFeedsPageVm;
use crate::view_models::singles_group::SinglesGroupKey;
use crate::view_models::workspace::{
    ContentViewMode, DiscoverView, FrameNavigationEntry, NavigationPathStart, ReleaseKey, TrackKey,
};
use crate::views::ArtistView;
use download_queue::DownloadQueue;

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
    /// A view of the Discover column is open in the detail column (ADR
    /// 0084 Decision 3).
    Discover(DiscoverView),
    /// The page of one Library singles group (ADR 0084 Decision 6). The
    /// page reads the current tree at each render, so a download or a
    /// removal updates it in place.
    SinglesGroup(SinglesGroupKey),
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
pub(crate) enum LibraryAppEvent {
    PlayPlaylistAt {
        track_id: i64,
        playlist_id: i64,
        playlist_position: i64,
    },
    /// Requests a content view-mode change, from the Discover detail
    /// header (ADR 0084 Decision 3). The top app owns the mutation and
    /// its config persistence (ADR 0062).
    SetContentViewMode(ContentViewMode),
    OpenIndexFeedDetail {
        feed_guid: String,
        label: String,
    },
    /// Opens the publisher page of a Library album or track (ADR 0077
    /// packet 004). `publisher_feed_guid` is the value that the album's
    /// stored owned relationship names, never name text.
    OpenPublisherPage {
        publisher_feed_guid: String,
    },
    /// Opens the feed page of a MusicIndex feed row of the Artists list,
    /// with the MusicIndex context, directly under its destination (ADR
    /// 0084 packet 016 decision D14).
    OpenIndexPublisherPage {
        publisher_feed_guid: String,
    },
    /// Opens the Library album page of a feed id, from the album name link
    /// of a track page (ADR 0083 Decision 5).
    OpenAlbumPage {
        feed_id: i64,
    },
    /// Opens the Library artist page of a name, from the artist name link
    /// of an album page (ADR 0083 task 005).
    OpenArtistPage {
        name: String,
    },
    /// Opens a Library page in the one Music navigation stack (ADR 0084
    /// Decision 9). The Library keeps no navigation state: the top app
    /// changes the stack, then restores the page.
    OpenPage {
        entry: FrameNavigationEntry,
        start: NavigationPathStart,
    },
    /// An Index job of the download queue completed (ADR 0084 Decision
    /// 10, packet 022). The top app updates its open pages in place and
    /// starts the tag scan of a successful download.
    IndexDownloadFinished {
        job: DownloadJob,
        outcome: DownloadOutcome,
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

/// The main color of one cover (ADR 0083 task 005).
#[derive(Clone, Copy)]
enum CoverColorState {
    Loading,
    Loaded(Option<crate::media::cover_color::CoverColor>),
}

impl CollectionArtHost for LibraryApp {
    fn collection_art_parts(&mut self) -> CollectionArtParts<'_> {
        CollectionArtParts {
            loads: &mut self.collection_art,
            cache: &self.cache,
            runner: &self.command_runner,
        }
    }
}

pub struct LibraryApp {
    conn: Arc<Mutex<Connection>>,
    application_services: Arc<ApplicationServices>,
    command_runner: AsyncCommandRunner,
    cache: Arc<ImageCache>,
    musicindex_endpoint: crate::config::MusicIndexEndpoint,
    music_dir: PathBuf,
    pub(crate) playback_availability:
        Result<(), crate::application::capability::ExecutionUnavailable>,
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
    cover_colors: BTreeMap<String, CoverColorState>,
    /// The load state of each collection art (ADR 0084 Decision 12).
    collection_art: CollectionArtLoads,
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
    /// Playlist RSS check actor (ADR 0076 Decision 2). Dropping the handle
    /// closes the inbox and lets the runtime task exit.
    playlist_rss_check: Option<crate::runtime::PlaylistRssCheckHandle>,
    /// Latest RSS check snapshot. `None` while no check actor runs.
    playlist_rss_snapshot: Option<crate::runtime::PlaylistRssCheckSnapshot>,
    /// Tag update actor (ADR 0076 Decision 8). Dropping the handle closes
    /// the inbox and lets the runtime task exit.
    tag_update: Option<crate::runtime::TagUpdateHandle>,
    /// Latest tag update snapshot. `None` while no tag update actor runs.
    tag_update_snapshot: Option<crate::runtime::TagUpdateSnapshot>,
    /// `true` when the actor did not accept the latest confirm.
    tag_update_request_failed: bool,
    /// Existing Recent Feeds pager consumed by the default Music content region.
    recent_music_page: RecentFeedsPageVm,
    /// Scroll state for default Music recency pagination.
    recent_music_scroll: ScrollHandle,
    /// The scope of the Releases list and of the Artists list (ADR 0084
    /// packet 016 decision D3). Each app start uses "Library". The config
    /// file does not store it.
    releases_scope: MusicListScope,
    artists_scope: MusicListScope,
    /// The loaded MusicIndex artist name pages of the Artists scope
    /// "Everything" (packet 016 decision D9). They stay until the app
    /// stops.
    artist_name_page: IndexNameListPage<ArtistNameRow>,
    /// The loaded MusicIndex release name pages of the Releases scope
    /// "Everything" (packet 016 decision D9).
    release_name_page: IndexNameListPage<Feed>,
    /// The singles groups whose Releases list rows show their singles
    /// (ADR 0084 Decision 6). This is view state. It is not stored.
    expanded_singles_groups: BTreeSet<SinglesGroupKey>,
    /// The one download queue of the Library and the Index (ADR 0084
    /// Decision 10, packet 022). At most three downloads run at the same
    /// time, and never two of one release.
    download_queue: DownloadQueue,
    /// The Releases list selection (ADR 0084 Decision 10). A change of
    /// destination clears it. Required correction item 2, 2026-10-18:
    /// the focus and the anchor hold a [`ReleasesRowId`], a release row
    /// or a group row, distinct from the [`ReleaseKey`] members that the
    /// selected set holds.
    releases_selection: ListSelection<ReleaseKey, ReleasesRowId>,
    /// The Releases list's mode (ADR 0084 Decision 10 Amendment,
    /// 2026-10-19): `Normal` or `Selecting`, beside `releases_selection`.
    /// A change of destination resets it to `Normal`.
    releases_selection_mode: SelectionMode,
    /// The Tracks and Needs attention lists' shared selection (ADR 0084
    /// Decision 10). A change of destination clears it.
    track_selection: ListSelection<TrackKey>,
    /// The Tracks and Needs attention lists' shared mode (ADR 0084
    /// Decision 10 Amendment, 2026-10-19), beside `track_selection`. A
    /// change of destination resets it to `Normal`.
    track_selection_mode: SelectionMode,
    /// The Releases list's held scroll handle (ADR 0084 Decision 10), so
    /// a keyboard focus move keeps the focused row in view.
    releases_list_scroll: ScrollHandle,
    /// The Music list column's own focus handle (ADR 0084 Decision 10).
    /// The selection keyboard actions act only while it, or a row inside
    /// it, carries the keyboard focus.
    list_column_focus: FocusHandle,
    /// The pending whole-selection removal confirmation (ADR 0084
    /// Decision 10).
    selection_removal: SelectionRemovalConfirmationState,
    /// The result of the whole-selection removal that runs, when one runs
    /// (ADR 0084 Decision 10).
    selection_removal_report: Option<SelectionRemovalReport>,
    /// The detail column selection, its mode and its page entry (ADR 0084
    /// Decision 10, packet 018). A page change clears it.
    detail_selection: DetailSelection,
    /// The Music detail column's own focus handle (packet 018). The
    /// detail selection keys act only while it carries the focus.
    detail_column_focus: FocusHandle,
    /// The Library track whose track page opens its "Inspect sources"
    /// disclosure when it mounts (ADR 0083 task 006, decision D4).
    pending_inspect_sources: Option<i64>,
    /// The spring-load timer of the sidebar row "Playlists" (ADR 0083
    /// task 008, decision D1).
    spring_load: crate::view_models::library::SpringLoadVm,
    /// The held scroll handle of each Music detail page (packet 018), so
    /// a key can scroll the page or keep the focused row in view.
    detail_column_scroll: ScrollHandle,
    /// The request to keep the focused detail row in view (packet 018).
    detail_scroll_reveal: ScrollReveal,
    /// The request to keep the focused Discover row or tile in view
    /// (packet 020). The Discover view scrolls with `recent_music_scroll`.
    discover_scroll_reveal: ScrollReveal,
}

pub(crate) struct PlaylistActorState {
    pub(crate) playlist_id: i64,
    pub(crate) snapshot: PagedTrackListSnapshot,
    pub(crate) handle: ActorHandle<PagedTrackListMsg, PagedTrackListSnapshot>,
}

mod app_impl;
mod column_view;
mod detail_selection;
mod download_queue;
mod downloads;
mod music_scopes;
mod release_page;
mod track_page;

pub(crate) use app_impl::playlist_options;
pub(crate) use column_view::{DownloadStatusView, LibraryColumn, LibraryColumnView};
pub(crate) use download_queue::{DownloadJob, DownloadOutcome};
pub(crate) use downloads::{
    api_track_from_view, index_release_job, index_release_job_by_guid, index_track_job,
};
