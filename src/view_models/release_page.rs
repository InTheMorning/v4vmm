//! The release page of the Library and of the Index (ADR 0084 Decision 4,
//! packet 021).
//!
//! Both origins build the release page through this one view model. The
//! origin selects the data source and the commands, not the page. This
//! module holds no renderer type. `src/ui/shells/release_page.rs` renders
//! the result.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;

use crate::db::{self, TrackRow};
use crate::view_models::album_page::{
    album_page_actions, AlbumNameLinkTarget, AlbumNameLinkVm, AlbumPageAction,
    AlbumPageActionDisplay, AlbumPageOrigin,
};
use crate::view_models::detail_selection::{index_track_guid, DetailRowKey};
use crate::view_models::download_progress::FeedKey;
use crate::view_models::entity_detail::{
    EntityActionKind, EntitySurfaceContext, NamedFeedRowVm, ReleaseDetailPageVm, ReleaseDetailVm,
    ReleaseMembershipState,
};
use crate::view_models::library::{
    description_line_count, display_description_text, AlbumNode, BusyItems,
    DescriptionDisclosureDisplay, DescriptionState, LibraryAlbumDetailVm, LibraryTrackRowVm,
    LibraryViewModel, MbTrackStatus, TrackDragVm, TrackRowWork,
};
use crate::view_models::list_selection::RowSelectionVm;
use crate::view_models::release_kind::ReleaseKind;
use crate::view_models::singles_group::SinglesGroupPageRowVm;
use crate::view_models::track_detail::{TrackDetailSurfaceContext, TrackDetailVm, TrackRowVm};
use crate::view_models::workspace::{FrameNavigationEntry, ReleaseKey, TrackKey};
use crate::view_models::{playlist_option_displays, PlaylistOptionDisplayVm};
use crate::views::{FeedRef, FeedView, TrackView};

/// The action id prefix of a Library release page.
const LIBRARY_ACTION_ID_PREFIX: &str = "album-page";
/// The action id prefix of an Index release page.
const INDEX_ACTION_ID_PREFIX: &str = "index-album-page";
/// The menu item labels of a track row (ADR 0083 task 006, decision D2).
const TRACK_MENU_ADD_TO_PLAYLIST_LABEL: &str = "Add to playlist";
const TRACK_MENU_DOWNLOAD_LABEL: &str = "Download";
const TRACK_MENU_INSPECT_SOURCES_LABEL: &str = "Inspect sources";
const TRACK_MENU_REMOVE_LABEL: &str = "Remove\u{2026}";
/// The menu item labels of a playlist row and of a Needs attention row
/// (ADR 0083 task 007, decisions D5 to D7).
const TRACK_MENU_MOVE_UP_LABEL: &str = "Move up";
const TRACK_MENU_MOVE_DOWN_LABEL: &str = "Move down";
const TRACK_MENU_REMOVE_FROM_PLAYLIST_LABEL: &str = "Remove from playlist";
const TRACK_MENU_REMOVE_FROM_LIBRARY_LABEL: &str = "Remove from Library\u{2026}";
/// The tooltip of the "⋯" button of a track row.
const TRACK_MENU_TRIGGER_TOOLTIP: &str = "More actions";
/// The labels of the "Add to playlist" level of the track row menu.
const TRACK_MENU_BACK_LABEL: &str = "Back";
const TRACK_MENU_BACK_A11Y_LABEL: &str = "Back to track actions";
const TRACK_MENU_NEW_PLAYLIST_A11Y_LABEL: &str = "Create a new playlist";
const TRACK_MENU_BACK_TO_CHOICES_A11Y_LABEL: &str = "Back to playlist choices";
const TRACK_MENU_CREATE_A11Y_LABEL: &str = "Create playlist and add track";
/// The tooltips of the download-state slot (decision D6).
const DOWNLOAD_SLOT_DOWNLOAD_TOOLTIP: &str = "Download";
const DOWNLOAD_SLOT_QUEUED_TOOLTIP: &str = "Queued for download";
const DOWNLOAD_SLOT_DOWNLOADING_TOOLTIP: &str = "Downloading";
/// The badge words of a track row (decision D11).
const TRACK_BADGE_NEW_LABEL: &str = "New";
const TRACK_BADGE_REMOVING_LABEL: &str = "Removing";

/// The origin of a release page, with the facts that select its commands.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ReleasePageOrigin {
    /// A Library album.
    Library {
        /// The Library feed id. `None` for an album with no feed row.
        feed_id: Option<i64>,
        /// The Library membership of the album.
        membership: ReleaseMembershipState,
        /// A `MusicBrainz` lookup can start now.
        musicbrainz_available: bool,
    },
    /// An Index feed that the Library does not hold.
    Index {
        /// The feed GUID. `None` when the feed has none.
        feed_guid: Option<String>,
    },
}

/// The Library facts of one track row of a release page.
#[derive(Clone, Debug)]
pub(crate) struct LibraryTrackFacts {
    track: TrackRow,
    mb: Option<MbTrackStatus>,
    work: TrackRowWork,
}

/// The origin-neutral input of a release page.
#[derive(Clone, Debug)]
pub(crate) struct ReleasePageFacts {
    feed: FeedView,
    origin: ReleasePageOrigin,
    kind: ReleaseKind,
    /// The feed URL that "Copy feed URL" copies.
    release_feed_url: Option<String>,
    /// The Library track facts, in row order. Empty for the Index origin.
    library_tracks: Vec<LibraryTrackFacts>,
    release_key: Option<ReleaseKey>,
    name_links: Vec<AlbumNameLinkVm>,
    /// The description text and its disclosure state.
    description: Option<(String, DescriptionState)>,
    selections: Vec<RowSelectionVm>,
    playlists: Vec<PlaylistOptionDisplayVm>,
    /// The work in progress on each Index track row, in row order (ADR
    /// 0083 task 006, decision D8). Empty for the Library origin. A row
    /// with no entry is `Idle`.
    index_works: Vec<TrackRowWork>,
}

impl ReleasePageFacts {
    /// The facts of a Library album. The query layer already read each
    /// stored value and each named feed. This function sends no request.
    #[must_use]
    pub(crate) fn from_library_album(
        album: &AlbumNode,
        mb_status: &BTreeMap<i64, MbTrackStatus>,
        library_vm: &LibraryViewModel,
    ) -> Self {
        let feed = library_album_feed_view(album);
        let album_vm = LibraryAlbumDetailVm::new(&album.tracks, mb_status);
        // ADR 0084 Decision 10: the busy state comes from the download
        // queue and the removals in progress.
        let busy = library_vm.busy_items();
        // ADR 0084 Decisions 4 and 8 (operator decision of 2026-10-16): the
        // release page reads no list filter. It always shows each track of
        // the release, each with its own state word.
        let library_tracks = album
            .release_track_rows()
            .iter()
            .map(|track| LibraryTrackFacts {
                track: track.clone(),
                mb: mb_status.get(&track.id).cloned(),
                work: busy.track_row_work(album, track),
            })
            .collect();
        let release_key = album.release_key();
        // ADR 0084 Decision 6: a single in a group links to its group. The
        // group also holds the MusicIndex singles of its feed (packet 026
        // decision D14).
        let name_links = library_vm.release_page_name_links(
            ReleaseDetailVm::new(&feed, EntitySurfaceContext::Library).name_links(),
            release_key.as_ref(),
        );
        let description = album
            .feed_id
            .zip(LibraryViewModel::display_description_text(
                album.description.as_deref(),
            ))
            .map(|(feed_id, text)| {
                (
                    text.to_owned(),
                    library_vm.album_description_state(feed_id, Some(text)),
                )
            });
        Self {
            origin: ReleasePageOrigin::Library {
                feed_id: album.feed_id,
                membership: album_vm
                    .membership(busy.release_download_phase(album), busy.removing(album)),
                musicbrainz_available: !album_vm.has_active_musicbrainz(),
            },
            kind: album.release_kind(),
            release_feed_url: album.feed_url.clone(),
            library_tracks,
            release_key,
            name_links,
            description,
            selections: Vec::new(),
            playlists: playlist_option_displays(library_vm.playlists()),
            index_works: Vec::new(),
            feed,
        }
    }

    /// The facts of an Index feed. `description_state` is the disclosure
    /// state of the feed description. `None` gives the automatic state.
    #[must_use]
    pub(crate) fn from_index_feed(
        feed: FeedView,
        description_state: Option<DescriptionState>,
    ) -> Self {
        let description = display_description_text(feed.description.as_deref()).map(|text| {
            let state = description_state
                .unwrap_or_else(|| DescriptionState::project(description_line_count(Some(text))));
            (text.to_owned(), state)
        });
        Self {
            origin: ReleasePageOrigin::Index {
                feed_guid: index_feed_guid(&feed),
            },
            kind: ReleaseKind::resolve(feed.release_kind.as_deref(), feed.track_count()),
            release_feed_url: feed.feed_url.clone(),
            library_tracks: Vec::new(),
            release_key: feed
                .feed_guid
                .as_deref()
                .map(ReleaseKey::from_index_feed_guid),
            name_links: ReleaseDetailVm::new(&feed, EntitySurfaceContext::Library).name_links(),
            description,
            selections: Vec::new(),
            playlists: Vec::new(),
            index_works: Vec::new(),
            feed,
        }
    }

    /// Gives the name links to `links`, with the release key of the page.
    /// The Index host uses it to add the link to a singles group (ADR 0084
    /// Decision 6, packet 026 decision D14).
    #[must_use]
    pub(crate) fn map_name_links(
        mut self,
        links: impl FnOnce(Vec<AlbumNameLinkVm>, Option<&ReleaseKey>) -> Vec<AlbumNameLinkVm>,
    ) -> Self {
        let base = std::mem::take(&mut self.name_links);
        self.name_links = links(base, self.release_key.as_ref());
        self
    }

    /// Sets the detail selection facts of the rows, in row order (ADR 0084
    /// Decision 10). A row with no selection fact gets
    /// [`RowSelectionVm::none`].
    #[must_use]
    pub(crate) fn with_row_selections(mut self, selections: Vec<RowSelectionVm>) -> Self {
        self.selections = selections;
        self
    }

    /// Sets the playlists that the playlist popovers show.
    #[must_use]
    pub(crate) fn with_playlists(mut self, playlists: &[db::Playlist]) -> Self {
        self.playlists = playlist_option_displays(playlists);
        self
    }

    /// Sets the work in progress on each Index track row from the queue
    /// state in `busy` (ADR 0083 task 006, decision D8). It changes
    /// nothing for the Library origin.
    #[must_use]
    pub(crate) fn with_index_track_work(mut self, busy: &BusyItems) -> Self {
        if !matches!(self.origin, ReleasePageOrigin::Index { .. }) {
            return self;
        }
        let key = FeedKey::new(
            index_feed_guid(&self.feed).as_deref(),
            self.feed.feed_url.as_deref(),
            None,
        );
        self.index_works = self
            .feed
            .tracks
            .iter()
            .map(|track| {
                let (Some(key), Some(guid)) = (key.as_ref(), index_track_guid(track)) else {
                    return TrackRowWork::Idle;
                };
                busy.index_track_work(key, guid.trim())
            })
            .collect();
        self
    }

    fn index_work(&self, index: usize) -> TrackRowWork {
        self.index_works.get(index).copied().unwrap_or_default()
    }

    fn selection(&self, index: usize) -> RowSelectionVm {
        self.selections
            .get(index)
            .cloned()
            .unwrap_or_else(RowSelectionVm::none)
    }
}

/// The `FeedView` of a Library album, from its stored facts.
fn library_album_feed_view(album: &AlbumNode) -> FeedView {
    let feed_row = db::FeedRow {
        id: album.feed_id.unwrap_or(0),
        feed_url: album.feed_url_for_detail(),
        feed_guid: album.feed_guid.clone(),
        title: Some(album.name.clone()),
        language: album.language.clone(),
        description: LibraryViewModel::display_description_text(album.description.as_deref())
            .map(str::to_owned),
        album_image_href: album.image_href.clone(),
        is_subscribed: false,
    };
    let track_views = album
        .tracks
        .clone()
        .into_iter()
        .map(TrackView::from_local)
        .collect();
    // ADR 0075 packet 020: the query layer projects the stored values of the
    // feed. An album without a feed row has only its columns.
    let mut feed_view = match album.stored_values.as_deref() {
        Some(values) => FeedView::from_local_with_facts(
            feed_row,
            track_views,
            album.identity_facts.clone(),
            values.clone(),
        ),
        None => {
            FeedView::from_local_with_identity(feed_row, track_views, album.identity_facts.clone())
        }
    };
    // ADR 0085 Decisions 5 and 6: the query layer already read the stored
    // named feeds of this album.
    feed_view.named_feeds.clone_from(&album.named_feeds);
    feed_view
}

/// One release or row command of a release page. The host entity maps
/// each command to its own data source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ReleasePageCommand {
    /// Download the release.
    DownloadRelease,
    /// Copy the feed URL of the release.
    CopyFeedUrl,
    /// Look up the missing `MusicBrainz` fields of the release.
    MusicBrainzLookup,
    /// Remove the release from the Library, after a confirmation.
    RemoveRelease,
    /// Add the release to the playlist with this id.
    AddReleaseToPlaylist(i64),
    /// Create a playlist with this name and add the release.
    CreatePlaylistForRelease(String),
    /// Open or close the description panel.
    ToggleDescription,
    /// Open the page of a name link.
    OpenNameLink(AlbumNameLinkTarget),
    /// Download the track of the row at this index (ADR 0083 task 006).
    DownloadTrack(usize),
    /// Remove the track of the row at this index, after a confirmation.
    RemoveTrack(usize),
    /// Open the track page of the row at this index with its "Inspect
    /// sources" disclosure open.
    InspectSources(usize),
    /// Add the track of the row at this index to the playlist with this id.
    AddTrackToPlaylist(usize, i64),
    /// Create a playlist with this name and add the track of the row.
    CreatePlaylistForTrack(usize, String),
    /// A click on the body of the row at this index (ADR 0084 packet 018).
    ClickRow(usize, RowClickModifiers),
    /// A click on the checkbox of the row at this index.
    ToggleRow(usize),
}

impl ReleasePageCommand {
    /// The command of an album page action. "Add to playlist" opens its
    /// popover, so it has no command.
    #[must_use]
    pub(crate) const fn for_action(action: AlbumPageAction) -> Option<Self> {
        match action {
            AlbumPageAction::DownloadAlbum => Some(Self::DownloadRelease),
            AlbumPageAction::CopyFeedUrl => Some(Self::CopyFeedUrl),
            AlbumPageAction::MusicBrainzLookup => Some(Self::MusicBrainzLookup),
            AlbumPageAction::RemoveAlbum => Some(Self::RemoveRelease),
            AlbumPageAction::AddToPlaylist => None,
        }
    }
}

/// The modifier keys of a row click, with no renderer type. The shell
/// copies them from the click event, and the host copies them back.
#[expect(
    clippy::struct_excessive_bools,
    reason = "each field is one modifier key of the click event"
)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct RowClickModifiers {
    pub(crate) control: bool,
    pub(crate) alt: bool,
    pub(crate) shift: bool,
    pub(crate) platform: bool,
    pub(crate) function: bool,
}

/// One action of a release page, with its id, labels and availability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReleasePageActionVm {
    pub(crate) action: AlbumPageAction,
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) a11y_label: String,
    pub(crate) available: bool,
    pub(crate) destructive: bool,
    /// The command of the action. `None` for "Add to playlist".
    pub(crate) command: Option<ReleasePageCommand>,
}

/// The action row of a release page (ADR 0083 Decision 5).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReleasePageActionsVm {
    /// The one filled button.
    pub(crate) filled: ReleasePageActionVm,
    /// The plain buttons.
    pub(crate) plain: Vec<ReleasePageActionVm>,
    /// The items of the "⋯" menu. A destructive item is last.
    pub(crate) menu: Vec<ReleasePageActionVm>,
    /// The element id of the "⋯" menu.
    pub(crate) menu_id: String,
    /// The element id of the release playlist popover. `None` when the
    /// release cannot go into a playlist.
    pub(crate) playlist_popover_id: Option<String>,
}

/// The element id of the description panel of a release page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ReleaseDescriptionId {
    /// The Library panel, by its Library feed id.
    Library(usize),
    /// The Index panel, by its feed GUID.
    Index(String),
}

/// The description panel of a release page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReleaseDescriptionVm {
    pub(crate) id: ReleaseDescriptionId,
    pub(crate) display: DescriptionDisclosureDisplay,
}

/// A badge after the title of a track row (ADR 0083 task 006, decision
/// D11).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TrackRowBadge {
    /// A Library track that is not downloaded.
    New,
    /// A removal of the track runs.
    Removing,
}

/// One badge of a track row, with its word.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TrackRowBadgeVm {
    pub(crate) badge: TrackRowBadge,
    pub(crate) label: &'static str,
}

/// The icon of the `MusicBrainz` status of a track row (decision D12).
/// Each status has its own icon.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MbStatusIcon {
    Pending,
    Processing,
    NoMissingFields,
    Done,
    Skipped,
}

/// The `MusicBrainz` status of a track row. The label is the tooltip and
/// the accessibility label of the icon.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TrackRowMbStatusVm {
    pub(crate) icon: MbStatusIcon,
    pub(crate) label: &'static str,
}

impl TrackRowMbStatusVm {
    /// The icon and the word of `status`.
    #[must_use]
    pub(crate) const fn of(status: &MbTrackStatus) -> Self {
        let (icon, label) = match status {
            MbTrackStatus::Pending => (MbStatusIcon::Pending, "MusicBrainz: pending"),
            MbTrackStatus::Processing => (MbStatusIcon::Processing, "MusicBrainz: looking up"),
            MbTrackStatus::Done(0) => (
                MbStatusIcon::NoMissingFields,
                "MusicBrainz: no missing fields",
            ),
            MbTrackStatus::Done(_) => (MbStatusIcon::Done, "MusicBrainz: done"),
            MbTrackStatus::Skipped => (MbStatusIcon::Skipped, "MusicBrainz: skipped"),
        };
        Self { icon, label }
    }
}

/// The state of the download-state slot of a track row (decision D6).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DownloadSlotState {
    /// The track is not downloaded and no job runs: a download button.
    Download,
    /// A download job waits in the queue.
    Queued,
    /// A download job runs.
    Downloading,
    /// The track is downloaded, or a removal runs: the slot is empty.
    Empty,
}

/// The download-state slot of a track row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DownloadSlotVm {
    pub(crate) state: DownloadSlotState,
    pub(crate) id: String,
    pub(crate) tooltip: Option<&'static str>,
    pub(crate) a11y_label: Option<String>,
    /// `true` only for [`DownloadSlotState::Download`].
    pub(crate) available: bool,
}

impl DownloadSlotVm {
    /// The slot of a row that is in the Library or not, with the work in
    /// progress on it. `title` names the track in the accessibility label.
    #[must_use]
    pub(crate) fn project(id: String, title: &str, in_library: bool, work: TrackRowWork) -> Self {
        let state = match (in_library, work) {
            (_, TrackRowWork::Queued) => DownloadSlotState::Queued,
            (_, TrackRowWork::Downloading) => DownloadSlotState::Downloading,
            (false, TrackRowWork::Idle) => DownloadSlotState::Download,
            (true, TrackRowWork::Idle) | (_, TrackRowWork::Removing) => DownloadSlotState::Empty,
        };
        let (tooltip, a11y_label) = match state {
            DownloadSlotState::Download => (
                Some(DOWNLOAD_SLOT_DOWNLOAD_TOOLTIP),
                Some(format!("Download {title}")),
            ),
            DownloadSlotState::Queued => (
                Some(DOWNLOAD_SLOT_QUEUED_TOOLTIP),
                Some(format!("{title} is queued for download")),
            ),
            DownloadSlotState::Downloading => (
                Some(DOWNLOAD_SLOT_DOWNLOADING_TOOLTIP),
                Some(format!("{title} is downloading")),
            ),
            DownloadSlotState::Empty => (None, None),
        };
        Self {
            state,
            id,
            tooltip,
            a11y_label,
            available: state == DownloadSlotState::Download,
        }
    }
}

/// One action of the track row menu (decision D2). `MoveUp`, `MoveDown`
/// and `RemoveFromPlaylist` are items of a playlist row only (ADR 0083
/// task 007, decision D13).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TrackRowMenuAction {
    /// Opens the playlist level of the menu.
    AddToPlaylist,
    Download,
    InspectSources,
    MoveUp,
    MoveDown,
    /// Removes the entry from the open playlist. The track stays in the
    /// Library (task 007, decision D6).
    RemoveFromPlaylist,
    /// Removes the track from the Library, after a confirmation.
    Remove,
}

impl TrackRowMenuAction {
    /// The command of the action on the row at `index`. "Add to playlist"
    /// opens the playlist level of the menu, so it has no command.
    #[must_use]
    pub(crate) const fn command(self, index: usize) -> Option<ReleasePageCommand> {
        match self {
            Self::AddToPlaylist | Self::MoveUp | Self::MoveDown | Self::RemoveFromPlaylist => None,
            Self::Download => Some(ReleasePageCommand::DownloadTrack(index)),
            Self::InspectSources => Some(ReleasePageCommand::InspectSources(index)),
            Self::Remove => Some(ReleasePageCommand::RemoveTrack(index)),
        }
    }
}

/// One item of the track row menu.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TrackRowMenuItemVm {
    pub(crate) action: TrackRowMenuAction,
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) a11y_label: String,
    pub(crate) destructive: bool,
}

/// The labels of the "Add to playlist" level of the track row menu
/// (decision D3).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TrackRowPlaylistLevelVm {
    pub(crate) back_text: &'static str,
    pub(crate) back_a11y_label: &'static str,
    pub(crate) new_playlist_a11y_label: &'static str,
    pub(crate) back_to_choices_a11y_label: &'static str,
    pub(crate) create_a11y_label: &'static str,
}

const TRACK_ROW_PLAYLIST_LEVEL: TrackRowPlaylistLevelVm = TrackRowPlaylistLevelVm {
    back_text: TRACK_MENU_BACK_LABEL,
    back_a11y_label: TRACK_MENU_BACK_A11Y_LABEL,
    new_playlist_a11y_label: TRACK_MENU_NEW_PLAYLIST_A11Y_LABEL,
    back_to_choices_a11y_label: TRACK_MENU_BACK_TO_CHOICES_A11Y_LABEL,
    create_a11y_label: TRACK_MENU_CREATE_A11Y_LABEL,
};

/// The "⋯" menu and the right-click menu of a track row (ADR 0083
/// Decision 6). Both menus show these items.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TrackRowMenuVm {
    pub(crate) id: String,
    pub(crate) trigger_tooltip: &'static str,
    pub(crate) trigger_a11y_label: String,
    /// The items in the order of decision D2. A destructive item is last.
    pub(crate) items: Vec<TrackRowMenuItemVm>,
    pub(crate) playlist_level: TrackRowPlaylistLevelVm,
}

/// The label of the destructive "Remove" item of a track row menu.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TrackRowRemoveLabel {
    /// "Remove…", on a release page row and on a Tracks list row (task
    /// 006, decision D2).
    Remove,
    /// "Remove from Library…", on a playlist row and on a Needs attention
    /// row (task 007, decision D7).
    RemoveFromLibrary,
}

impl TrackRowRemoveLabel {
    const fn label(self) -> &'static str {
        match self {
            Self::Remove => TRACK_MENU_REMOVE_LABEL,
            Self::RemoveFromLibrary => TRACK_MENU_REMOVE_FROM_LIBRARY_LABEL,
        }
    }
}

/// The facts that select the menu items of a track row. An item with a
/// false fact is absent (decision D2).
#[expect(
    clippy::struct_excessive_bools,
    reason = "each field is one independent availability fact of the menu"
)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct TrackRowMenuFacts<'a> {
    pub(crate) row_id: &'a str,
    pub(crate) title: &'a str,
    pub(crate) add_to_playlist: bool,
    pub(crate) download: bool,
    pub(crate) inspect_sources: bool,
    pub(crate) move_up: bool,
    pub(crate) move_down: bool,
    pub(crate) remove_from_playlist: bool,
    /// The label of "Remove…". `None` gives no "Remove…" item.
    pub(crate) remove: Option<TrackRowRemoveLabel>,
    pub(crate) selecting: bool,
}

impl<'a> TrackRowMenuFacts<'a> {
    /// The facts of a row with no item. Each host sets the items that it
    /// gives.
    #[must_use]
    pub(crate) const fn new(row_id: &'a str, title: &'a str, selecting: bool) -> Self {
        Self {
            row_id,
            title,
            add_to_playlist: false,
            download: false,
            inspect_sources: false,
            move_up: false,
            move_down: false,
            remove_from_playlist: false,
            remove: None,
            selecting,
        }
    }
}

/// The availability of "Download" and "Remove…" of a Library track row
/// (task 006, decisions D2 and D5).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LibraryTrackMenuAvailability {
    /// The track is not downloaded, and no job or removal runs.
    pub(crate) download: bool,
    /// The track is downloaded, and no job or removal runs.
    pub(crate) remove: bool,
}

impl LibraryTrackMenuAvailability {
    /// The availability of the items of `track` with the work `work`.
    #[must_use]
    pub(crate) fn of(track: &TrackRow, work: TrackRowWork) -> Self {
        // The primary action of the track page selects "Remove…": it is an
        // enabled "Remove" only for a downloaded row with no work in
        // progress (ADR 0083 task 006, decision D5).
        let action = LibraryTrackRowVm::new(track, None).primary_action_vm(work);
        Self {
            download: !track.is_in_library && work == TrackRowWork::Idle,
            remove: action.kind == EntityActionKind::Remove && action.enabled,
        }
    }
}

impl TrackRowMenuVm {
    /// The menu of a row. `None` in selection mode (decision D9). An item
    /// that cannot act is absent (decision D2). The items keep the order
    /// of task 007 decision D5. The destructive item is last.
    #[must_use]
    pub(crate) fn project(facts: &TrackRowMenuFacts<'_>) -> Option<Self> {
        if facts.selecting {
            return None;
        }
        let id = format!("{}-menu", facts.row_id);
        let title = facts.title;
        let item = |action: TrackRowMenuAction, label: &'static str, a11y_label: String| {
            TrackRowMenuItemVm {
                action,
                id: format!("{id}:{action:?}"),
                label: label.to_owned(),
                a11y_label,
                destructive: action == TrackRowMenuAction::Remove,
            }
        };
        let mut items = Vec::new();
        if facts.add_to_playlist {
            items.push(item(
                TrackRowMenuAction::AddToPlaylist,
                TRACK_MENU_ADD_TO_PLAYLIST_LABEL,
                format!("Add {title} to a playlist"),
            ));
        }
        if facts.download {
            items.push(item(
                TrackRowMenuAction::Download,
                TRACK_MENU_DOWNLOAD_LABEL,
                format!("Download {title}"),
            ));
        }
        if facts.inspect_sources {
            items.push(item(
                TrackRowMenuAction::InspectSources,
                TRACK_MENU_INSPECT_SOURCES_LABEL,
                format!("Inspect the sources of {title}"),
            ));
        }
        if facts.move_up {
            items.push(item(
                TrackRowMenuAction::MoveUp,
                TRACK_MENU_MOVE_UP_LABEL,
                format!("Move {title} up"),
            ));
        }
        if facts.move_down {
            items.push(item(
                TrackRowMenuAction::MoveDown,
                TRACK_MENU_MOVE_DOWN_LABEL,
                format!("Move {title} down"),
            ));
        }
        if facts.remove_from_playlist {
            items.push(item(
                TrackRowMenuAction::RemoveFromPlaylist,
                TRACK_MENU_REMOVE_FROM_PLAYLIST_LABEL,
                format!("Remove {title} from this playlist"),
            ));
        }
        if let Some(remove) = facts.remove {
            items.push(item(
                TrackRowMenuAction::Remove,
                remove.label(),
                format!("Remove {title} from the Library\u{2026}"),
            ));
        }
        Some(Self {
            trigger_a11y_label: format!("More actions for {title}"),
            trigger_tooltip: TRACK_MENU_TRIGGER_TOOLTIP,
            items,
            playlist_level: TRACK_ROW_PLAYLIST_LEVEL,
            id,
        })
    }
}

/// The page that "Inspect sources" of a Library track row opens: the track
/// page of the track (ADR 0083 task 006, decision D4). A singles group row
/// opens the track page too, not the release page of its single.
#[must_use]
pub(crate) fn inspect_sources_entry(track: &TrackRow) -> FrameNavigationEntry {
    FrameNavigationEntry::Track(track.track_key())
}

/// The one track row of a release page and of a singles group page, for
/// each origin.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReleaseTrackRowVm {
    pub(crate) row_id: String,
    pub(crate) row: TrackRowVm,
    /// The badges after the title, in order.
    pub(crate) badges: Vec<TrackRowBadgeVm>,
    pub(crate) mb_status: Option<TrackRowMbStatusVm>,
    pub(crate) download: DownloadSlotVm,
    /// The row menu. `None` in selection mode.
    pub(crate) menu: Option<TrackRowMenuVm>,
    /// The page that "Inspect sources" opens. `None` when the row has no
    /// "Inspect sources".
    pub(crate) inspect_sources_entry: Option<FrameNavigationEntry>,
    /// The detail selection key of the row. `None` gives the row no click
    /// and no checkbox click.
    pub(crate) row_key: Option<DetailRowKey>,
    /// The page that a plain click in `Normal` mode opens.
    pub(crate) open_entry: Option<FrameNavigationEntry>,
    /// The breadcrumb label of the page that a click opens. `None` when
    /// the host reads the label from its own state.
    pub(crate) open_label: Option<String>,
    pub(crate) selection: RowSelectionVm,
    pub(crate) thumbnail_url: Option<String>,
    /// The drag fact of the row (ADR 0083 task 008, decision D2). An Index
    /// row has no drag (decision D3). In `Selecting`, only a checked row
    /// of a Library release page has a drag: it carries each checked track
    /// (ADR 0083 Decision 6, amended 2026-10-21).
    pub(crate) drag: Option<TrackDragVm>,
}

impl ReleaseTrackRowVm {
    /// The row of a Library track on a release page. A click opens the
    /// track page. The row shows the date of the track.
    #[must_use]
    pub(crate) fn library(
        track: &TrackRow,
        mb: Option<&MbTrackStatus>,
        work: TrackRowWork,
        selection: RowSelectionVm,
    ) -> Self {
        let open_entry = FrameNavigationEntry::Track(track.track_key());
        Self::library_with_entry(track, mb, work, selection, open_entry)
    }

    /// The row of a single on a singles group page (ADR 0084 task 015,
    /// operator decision of 2026-10-17). A click opens the release page of
    /// the single. The row shows the date of the single. The row shows no
    /// track number: the number of the track in its single tells
    /// nothing (operator finding of 2026-10-20). The cover leads the row.
    #[must_use]
    pub(crate) fn library_single(
        single: &SinglesGroupPageRowVm,
        mb: Option<&MbTrackStatus>,
        selection: RowSelectionVm,
    ) -> Self {
        let open_entry = FrameNavigationEntry::Release(single.key.clone());
        let mut row =
            Self::library_with_entry(&single.track, mb, single.work, selection, open_entry);
        row.row.date_text.clone_from(&single.date_text);
        row.row.number = None;
        row
    }

    fn library_with_entry(
        track: &TrackRow,
        mb: Option<&MbTrackStatus>,
        work: TrackRowWork,
        selection: RowSelectionVm,
        open_entry: FrameNavigationEntry,
    ) -> Self {
        let vm = LibraryTrackRowVm::new(track, mb);
        let display = vm.row_display();
        let availability = LibraryTrackMenuAvailability::of(track, work);
        let track_view = TrackView::from_local(track.clone());
        let row = TrackDetailVm::new(&track_view, TrackDetailSurfaceContext::Library).row();
        let download = DownloadSlotVm::project(
            display.toggle_button_id,
            &row.title,
            track.is_in_library,
            work,
        );
        let mut badges = Vec::new();
        if display.state_label.is_some() {
            badges.push(TrackRowBadgeVm {
                badge: TrackRowBadge::New,
                label: TRACK_BADGE_NEW_LABEL,
            });
        }
        if work == TrackRowWork::Removing {
            badges.push(TrackRowBadgeVm {
                badge: TrackRowBadge::Removing,
                label: TRACK_BADGE_REMOVING_LABEL,
            });
        }
        // ADR 0083 task 006, decision D5: "Remove…" is absent while a job
        // or a removal of the row runs.
        let menu = TrackRowMenuVm::project(&TrackRowMenuFacts {
            add_to_playlist: true,
            download: download.available,
            inspect_sources: true,
            remove: availability.remove.then_some(TrackRowRemoveLabel::Remove),
            ..TrackRowMenuFacts::new(&display.row_id, &row.title, selection.checkbox_visible)
        });
        // ADR 0084 task 031: in `Selecting`, `release_page_vm` gives a
        // checked row the drag of each checked track. A singles group row
        // is a release, so it starts no drag in `Selecting` (decision D14).
        let drag = (!selection.checkbox_visible).then(|| TrackDragVm::one(track.id, &row.title));
        Self {
            row_id: display.row_id,
            badges,
            mb_status: vm.mb_status().map(TrackRowMbStatusVm::of),
            download,
            menu,
            inspect_sources_entry: Some(inspect_sources_entry(track)),
            row,
            row_key: DetailRowKey::for_open_entry(&open_entry),
            open_entry: Some(open_entry),
            open_label: None,
            selection,
            thumbnail_url: track
                .track_image_href
                .clone()
                .or_else(|| track.album_image_href.clone()),
            drag,
        }
    }

    /// The row of an Index track. The title follows the title rule of
    /// `TrackDetailVm::display_title`: the title is trimmed, and a title of
    /// only spaces shows the fallback (orchestrator decision of
    /// 2026-10-20). A track with no track GUID gets no key and no click.
    /// `work` is the phase of the Index track job, then the phase of a
    /// release job of the key (ADR 0083 task 006, decision D8).
    #[must_use]
    pub(crate) fn index(
        feed: &FeedView,
        index: usize,
        track: &TrackView,
        selection: RowSelectionVm,
        work: TrackRowWork,
    ) -> Self {
        let track_guid = index_track_guid(track);
        let id_key = track_guid.clone().unwrap_or_else(|| index.to_string());
        let feed_guid = index_feed_guid(feed);
        let key = index_track_row_click_key(feed_guid.as_deref(), track_guid);
        // The search path uses the same label fallback: the track GUID.
        let open_label = key
            .as_ref()
            .map(|key| track.title.clone().unwrap_or_else(|| key.item_guid.clone()));
        let row_id = format!("entity-track:{index}");
        let row = TrackDetailVm::new(track, TrackDetailSurfaceContext::Discover).row();
        // An Index row is never downloaded. The Index track page has no
        // "Inspect sources" (decision D4), and an Index row has no badge
        // (decision D11).
        let download = DownloadSlotVm::project(
            format!("index-track-download:{id_key}"),
            &row.title,
            false,
            work,
        );
        let menu = TrackRowMenuVm::project(&TrackRowMenuFacts {
            add_to_playlist: true,
            download: download.available,
            ..TrackRowMenuFacts::new(&row_id, &row.title, selection.checkbox_visible)
        });
        Self {
            row_id,
            row,
            badges: Vec::new(),
            mb_status: None,
            download,
            menu,
            inspect_sources_entry: None,
            row_key: key.clone().map(DetailRowKey::Track),
            open_entry: key.map(FrameNavigationEntry::Track),
            open_label,
            selection,
            thumbnail_url: index_track_row_artwork_url(feed, track).map(str::to_owned),
            drag: None,
        }
    }
}

/// The release page: the shared header and the parts that differ by
/// origin.
pub(crate) struct ReleasePageVm<'a> {
    pub(crate) detail: ReleaseDetailVm<'a>,
    pub(crate) page: ReleaseDetailPageVm<'a>,
    pub(crate) name_links: Vec<AlbumNameLinkVm>,
    pub(crate) named_feed_rows: Vec<NamedFeedRowVm>,
    /// The action row. Each action id starts with the action id prefix
    /// of the origin.
    pub(crate) actions: ReleasePageActionsVm,
    pub(crate) description: Option<ReleaseDescriptionVm>,
    /// The page shows the contributors panel. Only the Library origin
    /// shows it today (orchestrator decision of 2026-10-20).
    pub(crate) contributors_panel: bool,
    pub(crate) rows: Vec<ReleaseTrackRowVm>,
    pub(crate) playlists: Vec<PlaylistOptionDisplayVm>,
}

/// Builds the release page of `input`.
#[must_use]
pub(crate) fn release_page_vm(input: &ReleasePageFacts) -> ReleasePageVm<'_> {
    let detail = ReleaseDetailVm::new(&input.feed, EntitySurfaceContext::Library);
    let page = detail.page();
    let named_feed_rows = detail.named_feed_rows();
    let rows = match &input.origin {
        ReleasePageOrigin::Library { .. } => {
            let mut rows: Vec<ReleaseTrackRowVm> = input
                .library_tracks
                .iter()
                .enumerate()
                .map(|(index, track)| {
                    ReleaseTrackRowVm::library(
                        &track.track,
                        track.mb.as_ref(),
                        track.work,
                        input.selection(index),
                    )
                })
                .collect();
            apply_selection_drags(&mut rows, &input.library_tracks);
            rows
        }
        ReleasePageOrigin::Index { .. } => input
            .feed
            .tracks
            .iter()
            .enumerate()
            .map(|(index, track)| {
                ReleaseTrackRowVm::index(
                    &input.feed,
                    index,
                    track,
                    input.selection(index),
                    input.index_work(index),
                )
            })
            .collect(),
    };
    ReleasePageVm {
        detail,
        page,
        name_links: input.name_links.clone(),
        named_feed_rows,
        actions: release_page_actions(input),
        description: release_page_description(input),
        contributors_panel: matches!(input.origin, ReleasePageOrigin::Library { .. }),
        rows,
        playlists: input.playlists.clone(),
    }
}

/// The drag of each checked row of a Library release page in `Selecting`
/// (ADR 0083 Decision 6, amended 2026-10-21). A checked row carries each
/// checked track, in list order. A row that is not checked starts no drag.
/// A row in `Normal` keeps its drag of one track.
fn apply_selection_drags(rows: &mut [ReleaseTrackRowVm], tracks: &[LibraryTrackFacts]) {
    if !rows.iter().any(|row| row.selection.checkbox_visible) {
        return;
    }
    let checked: Vec<i64> = rows
        .iter()
        .zip(tracks)
        .filter(|(row, _)| row.selection.is_marked())
        .map(|(_, track)| track.track.id)
        .collect();
    for row in rows.iter_mut() {
        row.drag = TrackDragVm::selection(row.selection.is_marked(), &checked);
    }
}

/// The action row of a release page. Each action id starts with the
/// action id prefix of the origin.
fn release_page_actions(input: &ReleasePageFacts) -> ReleasePageActionsVm {
    let (action_id_prefix, album_origin, has_target, menu_suffix, playlist_popover_id) =
        match &input.origin {
            ReleasePageOrigin::Library {
                feed_id,
                membership,
                musicbrainz_available,
            } => (
                LIBRARY_ACTION_ID_PREFIX,
                AlbumPageOrigin::Library {
                    membership: *membership,
                    musicbrainz_available: *musicbrainz_available,
                },
                feed_id.is_some(),
                feed_id.unwrap_or_default().to_string(),
                feed_id.map(|feed_id| format!("album-feed-add:{feed_id}")),
            ),
            ReleasePageOrigin::Index { feed_guid } => (
                INDEX_ACTION_ID_PREFIX,
                AlbumPageOrigin::Index,
                true,
                feed_guid.clone().unwrap_or_else(|| "unknown".to_owned()),
                None,
            ),
        };
    let album_actions = album_page_actions(album_origin, input.release_feed_url.as_deref());
    let action_vm = |display: AlbumPageActionDisplay, place: &str| {
        let action = display.action;
        ReleasePageActionVm {
            action,
            id: format!("{action_id_prefix}-{place}:{action:?}"),
            label: action.label(input.kind),
            a11y_label: action.a11y_label(input.kind),
            available: display.available && has_target,
            destructive: action.is_destructive(),
            command: ReleasePageCommand::for_action(action),
        }
    };
    ReleasePageActionsVm {
        filled: action_vm(album_actions.filled, "action"),
        plain: album_actions
            .plain
            .iter()
            .map(|display| action_vm(*display, "action"))
            .collect(),
        menu: album_actions
            .menu
            .iter()
            .map(|display| action_vm(*display, "menu"))
            .collect(),
        menu_id: format!("{action_id_prefix}-menu-{menu_suffix}"),
        playlist_popover_id,
    }
}

/// The description panel of a release page. `None` when the release has
/// no description, or when a Library album has no feed row.
fn release_page_description(input: &ReleasePageFacts) -> Option<ReleaseDescriptionVm> {
    let (text, state) = input.description.as_ref()?;
    let id = match &input.origin {
        ReleasePageOrigin::Library { feed_id, .. } => {
            ReleaseDescriptionId::Library(usize::try_from((*feed_id)?).unwrap_or_default())
        }
        ReleasePageOrigin::Index { feed_guid } => {
            ReleaseDescriptionId::Index(feed_guid.clone().unwrap_or_else(|| "unknown".to_owned()))
        }
    };
    Some(ReleaseDescriptionVm {
        id,
        display: DescriptionDisclosureDisplay::project(text, *state),
    })
}

/// The feed GUID of an Index feed: its own GUID, or the GUID of its Index
/// reference.
#[must_use]
pub(crate) fn index_feed_guid(feed: &FeedView) -> Option<String> {
    feed.feed_guid.clone().or_else(|| match &feed.id {
        Some(FeedRef::Musicindex(feed_guid)) => Some(feed_guid.clone()),
        Some(FeedRef::LocalFeedId(_)) | None => None,
    })
}

/// The click key of an Index track row (ADR 0084 Decision 4, "The Index
/// track rows"). `None` when the track has no track GUID, so the row
/// gets no click.
#[must_use]
pub(crate) fn index_track_row_click_key(
    feed_guid: Option<&str>,
    track_guid: Option<String>,
) -> Option<TrackKey> {
    Some(TrackKey::from_index_track_target(feed_guid, track_guid?))
}

/// The artwork URL of an Index feed: the feed image, or else the first
/// track image.
#[must_use]
pub(crate) fn index_feed_artwork_url(feed: &FeedView) -> Option<&str> {
    non_empty_str(feed.image_url.as_deref()).or_else(|| {
        feed.tracks
            .iter()
            .find_map(|track| non_empty_str(track.image_url.as_deref()))
    })
}

/// The artwork URL of an Index track row: the track image (ADR 0075
/// Decision C, packet 048), or else the feed artwork.
#[must_use]
pub(crate) fn index_track_row_artwork_url<'a>(
    feed: &'a FeedView,
    track: &'a TrackView,
) -> Option<&'a str> {
    track
        .display_artwork_url()
        .or_else(|| index_feed_artwork_url(feed))
}

fn non_empty_str(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::queries::stored_values::{FeedStoredValues, Owned, ValueOwner};
    use crate::view_models::named_feeds::NamedFeed;
    use crate::views::{FeedMetadataFacts, LocalIdentityFacts};

    fn library_track(id: i64, guid: &str, title: &str, in_library: bool) -> TrackRow {
        TrackRow {
            id,
            feed_id: 7,
            feed_guid: Some("feed-guid-7".to_owned()),
            item_guid: guid.to_owned(),
            track_title: Some(title.to_owned()),
            track_number: Some(id),
            duration_seconds: Some(65),
            pub_date: Some(1_712_275_200),
            album_image_href: Some("https://example.test/album.jpg".to_owned()),
            is_in_library: in_library,
            ..TrackRow::default()
        }
    }

    /// A recorded album with stored values and named feeds.
    fn recorded_album() -> AlbumNode {
        AlbumNode {
            name: "Recorded Album".to_owned(),
            feed_id: Some(7),
            feed_guid: Some("feed-guid-7".to_owned()),
            feed_url: Some("https://example.test/feed.xml".to_owned()),
            language: Some("en".to_owned()),
            description: Some("  A short description.  ".to_owned()),
            image_href: Some("https://example.test/album.jpg".to_owned()),
            named_feeds: vec![NamedFeed {
                feed_guid: "artist-guid".to_owned(),
                title: Some("Recorded Artist".to_owned()),
                position: Some(0),
                role: None,
                role_agreement: None,
                feed_rel: None,
                album_rel: None,
                confirmed: true,
                image_url: None,
            }],
            identity_facts: LocalIdentityFacts::default(),
            metadata_facts: Box::new(FeedMetadataFacts::default()),
            stored_values: Some(Box::new(FeedStoredValues {
                title: Owned {
                    value: Some("Stored Title".to_owned()),
                    owner: ValueOwner::Channel,
                },
                description: Owned {
                    value: Some("A short description.".to_owned()),
                    owner: ValueOwner::Channel,
                },
                album_artist: Owned {
                    value: Some("Recorded Artist".to_owned()),
                    owner: ValueOwner::Channel,
                },
                ..FeedStoredValues::default()
            })),
            tracks: vec![
                library_track(1, "track-guid-1", "First", true),
                library_track(2, "track-guid-2", "Second", false),
            ],
        }
    }

    fn library_facts(album: &AlbumNode) -> ReleasePageFacts {
        ReleasePageFacts::from_library_album(album, &BTreeMap::new(), &LibraryViewModel::default())
    }

    fn index_track(title: Option<&str>, guid: Option<&str>) -> TrackView {
        TrackView {
            title: title.map(str::to_owned),
            track_guid: guid.map(str::to_owned),
            ..TrackView::default()
        }
    }

    fn index_feed(tracks: Vec<TrackView>) -> FeedView {
        FeedView {
            feed_guid: Some("index-feed".to_owned()),
            title: Some("Index Release".to_owned()),
            tracks,
            ..FeedView::default()
        }
    }

    /// R21-01: a Library album and the same feed as an Index `FeedView`
    /// give an equal page and equal row displays. Only the actions and the
    /// description id differ.
    #[test]
    fn adr_0084_release_page_r21_01_both_origins_give_one_page() {
        let album = recorded_album();
        let library = library_facts(&album);
        let index = ReleasePageFacts::from_index_feed(library_album_feed_view(&album), None);
        let library_vm = release_page_vm(&library);
        let index_vm = release_page_vm(&index);

        assert_eq!(
            format!("{:?}", library_vm.page),
            format!("{:?}", index_vm.page)
        );
        let rows = |vm: &ReleasePageVm<'_>| {
            vm.rows
                .iter()
                .map(|row| (row.row.clone(), row.selection.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(rows(&library_vm), rows(&index_vm));
        assert_eq!(library_vm.rows.len(), 2);

        assert!(library_vm
            .actions
            .filled
            .id
            .starts_with("album-page-action:"));
        assert!(index_vm
            .actions
            .filled
            .id
            .starts_with("index-album-page-action:"));
        assert_eq!(library_vm.actions.menu_id, "album-page-menu-7");
        assert_eq!(
            index_vm.actions.menu_id,
            "index-album-page-menu-feed-guid-7"
        );
        assert_eq!(
            library_vm.description.as_ref().map(|panel| &panel.id),
            Some(&ReleaseDescriptionId::Library(7))
        );
        assert_eq!(
            index_vm.description.as_ref().map(|panel| &panel.id),
            Some(&ReleaseDescriptionId::Index("feed-guid-7".to_owned()))
        );
        assert_eq!(
            library_vm.description.map(|panel| panel.display),
            index_vm.description.map(|panel| panel.display)
        );
    }

    /// R21-02: `from_library_album` gives the `FeedView` that the Library
    /// release page gave before packet 021, for a recorded album with
    /// stored values and named feeds.
    #[test]
    fn adr_0084_release_page_r21_02_library_album_gives_the_same_feed_view() {
        let album = recorded_album();
        let feed_row = db::FeedRow {
            id: 7,
            feed_url: "https://example.test/feed.xml".to_owned(),
            feed_guid: Some("feed-guid-7".to_owned()),
            title: Some("Recorded Album".to_owned()),
            language: Some("en".to_owned()),
            description: Some("A short description.".to_owned()),
            album_image_href: Some("https://example.test/album.jpg".to_owned()),
            is_subscribed: false,
        };
        let mut expected = FeedView::from_local_with_facts(
            feed_row,
            album
                .tracks
                .iter()
                .cloned()
                .map(TrackView::from_local)
                .collect(),
            album.identity_facts.clone(),
            album
                .stored_values
                .as_deref()
                .cloned()
                .expect("stored values"),
        );
        expected.named_feeds.clone_from(&album.named_feeds);

        let facts = library_facts(&album);

        assert_eq!(format!("{:?}", facts.feed), format!("{expected:?}"));
        assert_eq!(facts.feed.title.as_deref(), Some("Stored Title"));
        assert_eq!(facts.feed.named_feeds.len(), 1);
    }

    /// R21-03: for each `TrackMembershipState` and each `MusicBrainz`
    /// status, a Library row has the download slot, the `MusicBrainz`
    /// status and the "New" badge of `LibraryTrackRowVm` (ADR 0083 task
    /// 006 moves the primary action into the slot and the menu).
    #[test]
    fn adr_0084_release_page_r21_03_library_row_matches_library_track_row_vm() {
        let statuses = [
            None,
            Some(MbTrackStatus::Pending),
            Some(MbTrackStatus::Processing),
            Some(MbTrackStatus::Done(0)),
            Some(MbTrackStatus::Done(3)),
            Some(MbTrackStatus::Skipped),
        ];
        // (in the Library, work) gives InLibrary, Removing, Queued,
        // RemoteOnly and Downloading.
        let cases = [
            (true, TrackRowWork::Idle),
            (true, TrackRowWork::Removing),
            (true, TrackRowWork::Queued),
            (false, TrackRowWork::Idle),
            (false, TrackRowWork::Queued),
            (false, TrackRowWork::Downloading),
        ];
        for (in_library, work) in cases {
            for mb in &statuses {
                let track = library_track(4, "guid-4", "Song", in_library);
                let expected = LibraryTrackRowVm::new(&track, mb.as_ref());
                let action = expected.primary_action_vm(work);
                let row =
                    ReleaseTrackRowVm::library(&track, mb.as_ref(), work, RowSelectionVm::none());

                // The slot offers "Download" exactly when the primary
                // action of the track page is an enabled "Download".
                assert_eq!(
                    row.download.available,
                    action.enabled && action.label == "Download"
                );
                let menu = row.menu.as_ref().expect("a menu in Normal mode");
                let has_remove = menu
                    .items
                    .iter()
                    .any(|item| item.action == TrackRowMenuAction::Remove);
                assert_eq!(has_remove, action.enabled && action.label == "Remove");
                assert_eq!(row.download.id, "lib-toggle-4");
                assert_eq!(row.row_id, "album-track-4");
                assert_eq!(
                    row.mb_status.is_some(),
                    mb.is_some(),
                    "{in_library} {work:?} {mb:?}"
                );
                assert_eq!(
                    row.badges
                        .iter()
                        .any(|badge| badge.badge == TrackRowBadge::New),
                    expected.row_display().state_label.is_some()
                );
                assert_eq!(menu.id, "album-track-4-menu");
            }
        }
    }

    /// R21-04: an Index row offers "Download", has no status and no badge,
    /// and keeps the element ids of the Index page before packet 021.
    #[test]
    fn adr_0084_release_page_r21_04_index_row_keeps_its_ids_and_download() {
        let feed = index_feed(vec![
            index_track(Some("No GUID"), None),
            index_track(Some("Song"), Some("guid-b")),
        ]);
        let vm_facts = ReleasePageFacts::from_index_feed(feed, None);
        let vm = release_page_vm(&vm_facts);

        let with_guid = &vm.rows[1];
        assert_eq!(with_guid.download.state, DownloadSlotState::Download);
        assert!(with_guid.download.available);
        assert_eq!(with_guid.mb_status, None);
        assert!(with_guid.badges.is_empty());
        assert_eq!(with_guid.row_id, "entity-track:1");
        assert_eq!(with_guid.download.id, "index-track-download:guid-b");

        let without_guid = &vm.rows[0];
        assert_eq!(without_guid.row_id, "entity-track:0");
        assert_eq!(without_guid.download.id, "index-track-download:0");
        assert!(!vm.actions.filled.id.is_empty());
        assert_eq!(
            vm.actions.filled.id,
            "index-album-page-action:DownloadAlbum"
        );
    }

    /// R21-05: an Index row with no track GUID has no key and no open
    /// entry. A row with a track GUID opens its track page.
    #[test]
    fn adr_0084_release_page_r21_05_index_row_without_guid_has_no_key() {
        let feed = index_feed(vec![
            index_track(Some("No GUID"), None),
            index_track(Some("Song"), Some("guid-b")),
        ]);
        let without = ReleaseTrackRowVm::index(
            &feed,
            0,
            &feed.tracks[0],
            RowSelectionVm::none(),
            TrackRowWork::Idle,
        );
        assert_eq!(without.row_key, None);
        assert_eq!(without.open_entry, None);
        assert_eq!(without.open_label, None);

        let with = ReleaseTrackRowVm::index(
            &feed,
            1,
            &feed.tracks[1],
            RowSelectionVm::none(),
            TrackRowWork::Idle,
        );
        let key = TrackKey::from_index_track_target(Some("index-feed"), "guid-b");
        assert_eq!(with.row_key, Some(DetailRowKey::Track(key.clone())));
        assert_eq!(with.open_entry, Some(FrameNavigationEntry::Track(key)));
        assert_eq!(with.open_label.as_deref(), Some("Song"));
    }

    /// R21-06: a singles group row opens the release of its single and
    /// shows the date text of the single.
    #[test]
    fn adr_0084_release_page_r21_06_single_row_opens_its_release() {
        let key = ReleaseKey::FeedGuid("single-guid".to_owned());
        let single = SinglesGroupPageRowVm {
            track: library_track(9, "guid-9", "Single", true),
            date_text: Some("May 1, 2025".to_owned()),
            work: TrackRowWork::Idle,
            key: key.clone(),
        };

        let row = ReleaseTrackRowVm::library_single(&single, None, RowSelectionVm::none());

        assert_eq!(
            row.open_entry,
            Some(FrameNavigationEntry::Release(key.clone()))
        );
        assert_eq!(row.row_key, Some(DetailRowKey::Release(key)));
        assert_eq!(row.row.date_text.as_deref(), Some("May 1, 2025"));
        assert_eq!(row.row_id, "album-track-9");
        assert_eq!(row.download.id, "lib-toggle-9");
    }

    /// Operator finding of 2026-10-20: a single row on a singles group
    /// page has no track number. A Library row and an Index row on a
    /// release page keep their track number.
    #[test]
    fn adr_0084_row_fix_single_row_has_no_track_number() {
        let track = library_track(9, "guid-9", "Single", true);
        let single = SinglesGroupPageRowVm {
            track: track.clone(),
            date_text: None,
            work: TrackRowWork::Idle,
            key: ReleaseKey::FeedGuid("single-guid".to_owned()),
        };

        let single_row = ReleaseTrackRowVm::library_single(&single, None, RowSelectionVm::none());
        assert_eq!(single_row.row.number, None);

        let release_row =
            ReleaseTrackRowVm::library(&track, None, TrackRowWork::Idle, RowSelectionVm::none());
        assert_eq!(release_row.row.number.as_deref(), Some("9"));

        let index_track = TrackView {
            track_number: Some(3),
            ..index_track(Some("Song"), Some("item-3"))
        };
        let feed = index_feed(vec![index_track.clone()]);
        let index_row = ReleaseTrackRowVm::index(
            &feed,
            0,
            &index_track,
            RowSelectionVm::none(),
            TrackRowWork::Idle,
        );
        assert_eq!(index_row.row.number.as_deref(), Some("3"));
    }

    /// R21-07:`contributors_panel` is true for the Library origin and
    /// false for the Index origin.
    #[test]
    fn adr_0084_release_page_r21_07_contributors_panel_is_library_only() {
        let album = recorded_album();
        let library = library_facts(&album);
        let index = ReleasePageFacts::from_index_feed(library_album_feed_view(&album), None);

        assert!(release_page_vm(&library).contributors_panel);
        assert!(!release_page_vm(&index).contributors_panel);
        assert!(matches!(library.origin, ReleasePageOrigin::Library { .. }));
    }

    /// R21-08: an Index title with spaces before and after it is trimmed.
    /// A title of only spaces gives the fallback.
    #[test]
    fn adr_0084_release_page_r21_08_index_title_follows_the_track_title_rule() {
        let feed = index_feed(vec![
            index_track(Some("  Padded Song  "), Some("guid-a")),
            index_track(Some("   "), Some("guid-b")),
            index_track(Some("   "), None),
        ]);
        let facts = ReleasePageFacts::from_index_feed(feed, None);
        let vm = release_page_vm(&facts);

        assert_eq!(vm.rows[0].row.title, "Padded Song");
        assert_eq!(vm.rows[0].row.a11y_label, "Padded Song");
        assert_eq!(vm.rows[1].row.title, "guid-b");
        assert_eq!(vm.rows[2].row.title, "Untitled");
    }
    /// R13-05 (ADR 0084 task 013): the Index track row click builds the
    /// key `TrackKey::from_index_track_target`, through a pure function.
    /// A track with no GUID gives no key.
    #[test]
    fn adr_0084_index_page_r13_05_track_row_click_key_needs_a_track_guid() {
        assert_eq!(
            index_track_row_click_key(Some("feed-1"), Some("track-1".to_string())),
            Some(TrackKey::from_index_track_target(Some("feed-1"), "track-1"))
        );
        assert_eq!(
            index_track_row_click_key(None, Some("track-1".to_string())),
            Some(TrackKey::from_index_track_target(None, "track-1"))
        );
        assert_eq!(index_track_row_click_key(Some("feed-1"), None), None);
        assert_eq!(index_track_row_click_key(None, None), None);
    }
    #[test]
    fn index_feed_artwork_url_prefers_feed_image() {
        let feed = FeedView {
            image_url: Some("https://example.test/feed.jpg".to_string()),
            tracks: vec![TrackView {
                image_url: Some("https://example.test/track.jpg".to_string()),
                ..TrackView::default()
            }],
            ..FeedView::default()
        };

        assert_eq!(
            index_feed_artwork_url(&feed),
            Some("https://example.test/feed.jpg")
        );
    }

    #[test]
    fn index_feed_artwork_url_falls_back_to_track_image() {
        let feed = FeedView {
            tracks: vec![
                TrackView {
                    image_url: Some("   ".to_string()),
                    ..TrackView::default()
                },
                TrackView {
                    image_url: Some("https://example.test/track.jpg".to_string()),
                    ..TrackView::default()
                },
            ],
            ..FeedView::default()
        };

        assert_eq!(
            index_feed_artwork_url(&feed),
            Some("https://example.test/track.jpg")
        );
    }

    #[test]
    fn index_track_row_artwork_url_falls_back_to_feed_image() {
        let feed = FeedView {
            image_url: Some("https://example.test/feed.jpg".to_string()),
            ..FeedView::default()
        };
        let track = TrackView {
            image_url: None,
            ..TrackView::default()
        };

        assert_eq!(
            index_track_row_artwork_url(&feed, &track),
            Some("https://example.test/feed.jpg")
        );
    }

    #[test]
    fn index_track_row_artwork_url_prefers_track_image() {
        let feed = FeedView {
            image_url: Some("https://example.test/feed.jpg".to_string()),
            ..FeedView::default()
        };
        let track = TrackView {
            image_url: Some("https://example.test/track.jpg".to_string()),
            ..TrackView::default()
        };

        assert_eq!(
            index_track_row_artwork_url(&feed, &track),
            Some("https://example.test/track.jpg")
        );
    }

    fn selecting() -> RowSelectionVm {
        RowSelectionVm {
            checkbox_visible: true,
            ..RowSelectionVm::none()
        }
    }

    fn menu_actions(row: &ReleaseTrackRowVm) -> Vec<TrackRowMenuAction> {
        row.menu
            .as_ref()
            .map(|menu| menu.items.iter().map(|item| item.action).collect())
            .unwrap_or_default()
    }

    fn library_row(in_library: bool, work: TrackRowWork) -> ReleaseTrackRowVm {
        let track = library_track(4, "guid-4", "Face Pale", in_library);
        ReleaseTrackRowVm::library(&track, None, work, RowSelectionVm::none())
    }

    const ALL_WORK: [TrackRowWork; 4] = [
        TrackRowWork::Idle,
        TrackRowWork::Queued,
        TrackRowWork::Downloading,
        TrackRowWork::Removing,
    ];

    /// R83-6-01 (ADR 0083 Decision 6): the menu gives its items in the
    /// order of decision D2, for a downloaded row, a row that is not
    /// downloaded and an Index row.
    #[test]
    fn adr_0083_row_actions_r83_6_01_menu_items_follow_the_decided_order() {
        use TrackRowMenuAction::{AddToPlaylist, Download, InspectSources, Remove};
        assert_eq!(
            menu_actions(&library_row(true, TrackRowWork::Idle)),
            vec![AddToPlaylist, InspectSources, Remove]
        );
        assert_eq!(
            menu_actions(&library_row(false, TrackRowWork::Idle)),
            vec![AddToPlaylist, Download, InspectSources]
        );
        let feed = index_feed(vec![index_track(Some("Song"), Some("guid-b"))]);
        let index_row = ReleaseTrackRowVm::index(
            &feed,
            0,
            &feed.tracks[0],
            RowSelectionVm::none(),
            TrackRowWork::Idle,
        );
        assert_eq!(menu_actions(&index_row), vec![AddToPlaylist, Download]);
        let menu = index_row.menu.expect("a menu");
        assert_eq!(menu.trigger_tooltip, "More actions");
        assert_eq!(menu.trigger_a11y_label, "More actions for Song");
        let labels: Vec<_> = menu.items.iter().map(|item| item.label.as_str()).collect();
        assert_eq!(labels, vec!["Add to playlist", "Download"]);
    }

    /// R83-6-02 (ADR 0083 Decision 5): "Remove…" is last and destructive,
    /// its label ends with "…", and its accessibility label names the
    /// track. It is absent while a job or a removal of the row runs.
    #[test]
    fn adr_0083_row_actions_r83_6_02_remove_is_last_destructive_and_confirms() {
        let row = library_row(true, TrackRowWork::Idle);
        let menu = row.menu.expect("a menu");
        let remove = menu.items.last().expect("an item");
        assert_eq!(remove.action, TrackRowMenuAction::Remove);
        assert!(remove.destructive);
        assert!(remove.label.ends_with('\u{2026}'));
        assert_eq!(
            remove.a11y_label,
            "Remove Face Pale from the Library\u{2026}"
        );
        assert_eq!(menu.items.iter().filter(|item| item.destructive).count(), 1);
        assert_eq!(
            TrackRowMenuAction::Remove.command(2),
            Some(ReleasePageCommand::RemoveTrack(2))
        );
        for work in [
            TrackRowWork::Queued,
            TrackRowWork::Downloading,
            TrackRowWork::Removing,
        ] {
            assert!(
                !menu_actions(&library_row(true, work)).contains(&TrackRowMenuAction::Remove),
                "{work:?}"
            );
        }
    }

    /// R83-6-03: "Download" is absent for a downloaded row, and while a
    /// job or a removal of the row runs.
    #[test]
    fn adr_0083_row_actions_r83_6_03_download_only_for_an_idle_remote_row() {
        for in_library in [true, false] {
            for work in ALL_WORK {
                let has_download = menu_actions(&library_row(in_library, work))
                    .contains(&TrackRowMenuAction::Download);
                assert_eq!(
                    has_download,
                    !in_library && work == TrackRowWork::Idle,
                    "{in_library} {work:?}"
                );
            }
        }
        assert_eq!(
            TrackRowMenuAction::Download.command(1),
            Some(ReleasePageCommand::DownloadTrack(1))
        );
        assert_eq!(TrackRowMenuAction::AddToPlaylist.command(1), None);
    }

    /// R83-6-04: "Inspect sources" shows only on a Library row with a
    /// track key, also on a singles group row. Its entry is the track page
    /// of the row track, not the release page of the single.
    #[test]
    fn adr_0083_row_actions_r83_6_04_inspect_sources_targets_the_track_page() {
        let track = library_track(9, "guid-9", "Single", true);
        let single = SinglesGroupPageRowVm {
            track: track.clone(),
            date_text: None,
            work: TrackRowWork::Idle,
            key: ReleaseKey::FeedGuid("single-guid".to_owned()),
        };
        let row = ReleaseTrackRowVm::library_single(&single, None, RowSelectionVm::none());
        assert!(menu_actions(&row).contains(&TrackRowMenuAction::InspectSources));
        assert_eq!(
            row.inspect_sources_entry,
            Some(FrameNavigationEntry::Track(track.track_key()))
        );
        assert_eq!(
            inspect_sources_entry(&track),
            FrameNavigationEntry::Track(track.track_key())
        );
        assert_eq!(
            TrackRowMenuAction::InspectSources.command(3),
            Some(ReleasePageCommand::InspectSources(3))
        );

        let feed = index_feed(vec![index_track(Some("Song"), Some("guid-b"))]);
        let index_row = ReleaseTrackRowVm::index(
            &feed,
            0,
            &feed.tracks[0],
            RowSelectionVm::none(),
            TrackRowWork::Idle,
        );
        assert!(!menu_actions(&index_row).contains(&TrackRowMenuAction::InspectSources));
        assert_eq!(index_row.inspect_sources_entry, None);
    }

    /// R83-6-05 (decision D6): the slot state for each pair of
    /// `is_in_library` and `TrackRowWork`. Each state that is not empty has
    /// a tooltip and an accessibility label. Only `Download` can act.
    #[test]
    fn adr_0083_row_actions_r83_6_05_download_slot_follows_membership_and_work() {
        for in_library in [true, false] {
            for work in ALL_WORK {
                let slot = library_row(in_library, work).download;
                let expected = match (in_library, work) {
                    (_, TrackRowWork::Queued) => DownloadSlotState::Queued,
                    (_, TrackRowWork::Downloading) => DownloadSlotState::Downloading,
                    (false, TrackRowWork::Idle) => DownloadSlotState::Download,
                    (true, TrackRowWork::Idle) | (_, TrackRowWork::Removing) => {
                        DownloadSlotState::Empty
                    }
                };
                assert_eq!(slot.state, expected, "{in_library} {work:?}");
                let filled = expected != DownloadSlotState::Empty;
                assert_eq!(slot.tooltip.is_some(), filled);
                assert_eq!(slot.a11y_label.is_some(), filled);
                assert_eq!(slot.available, expected == DownloadSlotState::Download);
            }
        }
        let slot = library_row(false, TrackRowWork::Queued).download;
        assert_eq!(slot.tooltip, Some("Queued for download"));
        assert_eq!(
            slot.a11y_label.as_deref(),
            Some("Face Pale is queued for download")
        );
        let slot = library_row(false, TrackRowWork::Downloading).download;
        assert_eq!(slot.tooltip, Some("Downloading"));
        let slot = library_row(false, TrackRowWork::Idle).download;
        assert_eq!(slot.tooltip, Some("Download"));
        assert_eq!(slot.a11y_label.as_deref(), Some("Download Face Pale"));
    }

    /// R83-6-06 (decision D8): an Index row reads the phase of its own
    /// job, then the phase of a release job of its key. A job of another
    /// track of the release leaves the row at `Download`.
    #[test]
    fn adr_0083_row_actions_r83_6_06_index_row_reads_its_own_phase() {
        use crate::view_models::download_progress::DownloadActivity;
        use std::collections::BTreeSet;

        let key = FeedKey::Guid("index-feed".to_owned());
        let feed = index_feed(vec![
            index_track(Some("A"), Some("guid-a")),
            index_track(Some("B"), Some(" guid-b ")),
            index_track(Some("C"), None),
        ]);
        let states = |activity: DownloadActivity| {
            let busy = BusyItems::new(activity, BTreeSet::new());
            let facts =
                ReleasePageFacts::from_index_feed(feed.clone(), None).with_index_track_work(&busy);
            release_page_vm(&facts)
                .rows
                .iter()
                .map(|row| row.download.state)
                .collect::<Vec<_>>()
        };

        let own_job = DownloadActivity {
            index_tracks: BTreeSet::from([(key.clone(), "guid-a".to_owned())]),
            ..DownloadActivity::default()
        };
        assert_eq!(
            states(own_job),
            vec![
                DownloadSlotState::Queued,
                DownloadSlotState::Download,
                DownloadSlotState::Download,
            ]
        );

        let running = DownloadActivity {
            index_tracks: BTreeSet::from([(key.clone(), "guid-b".to_owned())]),
            in_flight_index_tracks: BTreeSet::from([(key.clone(), "guid-b".to_owned())]),
            ..DownloadActivity::default()
        };
        assert_eq!(
            states(running),
            vec![
                DownloadSlotState::Download,
                DownloadSlotState::Downloading,
                DownloadSlotState::Download,
            ]
        );

        let release_job = DownloadActivity {
            keys: BTreeSet::from([key.clone()]),
            release_keys: BTreeSet::from([key.clone()]),
            ..DownloadActivity::default()
        };
        assert_eq!(
            states(release_job),
            vec![
                DownloadSlotState::Queued,
                DownloadSlotState::Queued,
                DownloadSlotState::Download,
            ]
        );
    }

    /// R83-6-07 (decision D9): in `Selecting`, the row has no menu and the
    /// same download slot. In `Normal`, the row has a menu.
    #[test]
    fn adr_0083_row_actions_r83_6_07_selection_mode_removes_the_menu() {
        let track = library_track(4, "guid-4", "Face Pale", false);
        let normal =
            ReleaseTrackRowVm::library(&track, None, TrackRowWork::Idle, RowSelectionVm::none());
        let in_selection =
            ReleaseTrackRowVm::library(&track, None, TrackRowWork::Idle, selecting());
        assert!(normal.menu.is_some());
        assert_eq!(in_selection.menu, None);
        assert_eq!(in_selection.download, normal.download);

        let feed = index_feed(vec![index_track(Some("Song"), Some("guid-b"))]);
        let index_row =
            ReleaseTrackRowVm::index(&feed, 0, &feed.tracks[0], selecting(), TrackRowWork::Idle);
        assert_eq!(index_row.menu, None);
        assert_eq!(index_row.download.state, DownloadSlotState::Download);
    }

    /// R83-6-08 (decision D11): "New" on a Library row that is not
    /// downloaded, "Removing" while a removal runs, and no badge on an
    /// Index row.
    #[test]
    fn adr_0083_row_actions_r83_6_08_badges_name_the_row_state() {
        let badges = |row: &ReleaseTrackRowVm| {
            row.badges
                .iter()
                .map(|badge| (badge.badge, badge.label))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            badges(&library_row(false, TrackRowWork::Idle)),
            vec![(TrackRowBadge::New, "New")]
        );
        assert!(badges(&library_row(true, TrackRowWork::Idle)).is_empty());
        assert_eq!(
            badges(&library_row(true, TrackRowWork::Removing)),
            vec![(TrackRowBadge::Removing, "Removing")]
        );
        let feed = index_feed(vec![index_track(Some("Song"), Some("guid-b"))]);
        let index_row = ReleaseTrackRowVm::index(
            &feed,
            0,
            &feed.tracks[0],
            RowSelectionVm::none(),
            TrackRowWork::Idle,
        );
        assert!(index_row.badges.is_empty());
    }

    /// R83-6-09 (decision D12): each `MusicBrainz` status has its own icon
    /// and a label that starts with "`MusicBrainz`: ".
    #[test]
    fn adr_0083_row_actions_r83_6_09_each_musicbrainz_status_has_its_own_icon() {
        let statuses = [
            MbTrackStatus::Pending,
            MbTrackStatus::Processing,
            MbTrackStatus::Done(0),
            MbTrackStatus::Done(3),
            MbTrackStatus::Skipped,
        ];
        let projected: Vec<_> = statuses.iter().map(TrackRowMbStatusVm::of).collect();
        for status in &projected {
            assert!(status.label.starts_with("MusicBrainz: "), "{status:?}");
        }
        for (index, status) in projected.iter().enumerate() {
            for other in &projected[index + 1..] {
                assert_ne!(status.icon, other.icon);
                assert_ne!(status.label, other.label);
            }
        }
        let track = library_track(4, "guid-4", "Face Pale", true);
        let row = ReleaseTrackRowVm::library(
            &track,
            Some(&MbTrackStatus::Skipped),
            TrackRowWork::Idle,
            RowSelectionVm::none(),
        );
        assert_eq!(
            row.mb_status,
            Some(TrackRowMbStatusVm {
                icon: MbStatusIcon::Skipped,
                label: "MusicBrainz: skipped",
            })
        );
    }

    /// R83-8-01 (ADR 0083 task 008, decisions D2 and D3): a Library row
    /// in `Normal` gives its drag fact with the track id and the title. An
    /// Index row gives none. The row builder gives no drag in `Selecting`:
    /// `release_page_vm` sets the drag of a selection (ADR 0084 task 031,
    /// R31-09).
    #[test]
    fn adr_0083_track_drag_r83_8_01_release_row_drag() {
        use crate::view_models::library::TrackDragVm;
        let expected = Some(TrackDragVm::one(4, "Face Pale"));
        for in_library in [true, false] {
            assert_eq!(
                library_row(in_library, TrackRowWork::Idle).drag,
                expected,
                "in_library {in_library}"
            );
        }
        let track = library_track(4, "guid-4", "Face Pale", true);
        let selected = ReleaseTrackRowVm::library(&track, None, TrackRowWork::Idle, selecting());
        assert_eq!(selected.drag, None);

        let single = SinglesGroupPageRowVm {
            track: library_track(9, "guid-9", "Single", true),
            date_text: None,
            work: TrackRowWork::Idle,
            key: ReleaseKey::FeedGuid("single-guid".to_owned()),
        };
        let single_row = ReleaseTrackRowVm::library_single(&single, None, RowSelectionVm::none());
        assert_eq!(single_row.drag.map(|drag| drag.track_ids), Some(vec![9]));
        let single_selected = ReleaseTrackRowVm::library_single(&single, None, selecting());
        assert_eq!(single_selected.drag, None);

        let feed = index_feed(vec![index_track(Some("Song"), Some("guid-b"))]);
        for selection in [RowSelectionVm::none(), selecting()] {
            let index_row =
                ReleaseTrackRowVm::index(&feed, 0, &feed.tracks[0], selection, TrackRowWork::Idle);
            assert_eq!(index_row.drag, None);
        }
    }

    /// A Library album of four tracks, in track order.
    fn four_track_album() -> AlbumNode {
        AlbumNode {
            tracks: vec![
                library_track(1, "g-1", "One", true),
                library_track(2, "g-2", "Two", true),
                library_track(3, "g-3", "Three", false),
                library_track(4, "g-4", "Four", true),
            ],
            ..recorded_album()
        }
    }

    fn checked(title: &str, checked: bool) -> RowSelectionVm {
        RowSelectionVm::for_row(title, checked, false, true)
    }

    /// R31-08 (ADR 0084 task 031, decision D9): in `Normal`, each row of a
    /// Library release page drags its one track, and the preview shows its
    /// title.
    #[test]
    fn adr_0083_selection_drag_r31_08_normal_drag_is_one_track() {
        let album = four_track_album();
        let facts = library_facts(&album);
        let vm = release_page_vm(&facts);
        let drags: Vec<_> = vm.rows.iter().map(|row| row.drag.clone()).collect();
        assert_eq!(
            drags,
            vec![
                Some(TrackDragVm::one(1, "One")),
                Some(TrackDragVm::one(2, "Two")),
                Some(TrackDragVm::one(3, "Three")),
                Some(TrackDragVm::one(4, "Four")),
            ]
        );
    }

    /// R31-09 (ADR 0083 Decision 6, amended 2026-10-21): in `Selecting`, a
    /// checked row carries each checked Library track, in list order, with
    /// the label "3 tracks". A row that is not checked gives `None`. One
    /// checked track gives "1 track".
    #[test]
    fn adr_0083_selection_drag_r31_09_checked_rows_carry_the_selection() {
        let album = four_track_album();
        let facts = library_facts(&album).with_row_selections(vec![
            checked("One", false),
            checked("Two", true),
            checked("Three", true),
            checked("Four", true),
        ]);
        let vm = release_page_vm(&facts);
        let expected = TrackDragVm {
            track_ids: vec![2, 3, 4],
            label: "3 tracks".to_owned(),
        };
        assert_eq!(vm.rows[0].drag, None, "a row that is not checked");
        for row in &vm.rows[1..] {
            assert_eq!(row.drag.as_ref(), Some(&expected), "{}", row.row_id);
        }

        let one = library_facts(&album).with_row_selections(vec![
            checked("One", false),
            checked("Two", false),
            checked("Three", true),
            checked("Four", false),
        ]);
        let vm = release_page_vm(&one);
        assert_eq!(
            vm.rows[2].drag,
            Some(TrackDragVm {
                track_ids: vec![3],
                label: "1 track".to_owned(),
            })
        );
        assert!(vm
            .rows
            .iter()
            .enumerate()
            .all(|(index, row)| index == 2 || row.drag.is_none()));

        let none = library_facts(&album).with_row_selections(vec![checked("One", false); 4]);
        assert!(release_page_vm(&none)
            .rows
            .iter()
            .all(|row| row.drag.is_none()));
    }

    /// R31-13 (task 031, decision D11): on an Index release page, a checked
    /// row in `Selecting` starts no drag.
    #[test]
    fn adr_0083_selection_drag_r31_13_index_page_starts_no_drag() {
        let feed = index_feed(vec![
            index_track(Some("Song"), Some("guid-a")),
            index_track(Some("Other"), Some("guid-b")),
        ]);
        let facts = ReleasePageFacts::from_index_feed(feed, None)
            .with_row_selections(vec![checked("Song", true), checked("Other", true)]);
        assert!(release_page_vm(&facts)
            .rows
            .iter()
            .all(|row| row.drag.is_none()));
    }
}
