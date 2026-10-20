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
use crate::view_models::entity_detail::{
    EntityActionTone, EntitySurfaceContext, NamedFeedRowVm, ReleaseDetailPageVm, ReleaseDetailVm,
    ReleaseMembershipState,
};
use crate::view_models::library::{
    description_line_count, display_description_text, AlbumNode, DescriptionDisclosureDisplay,
    DescriptionState, LibraryAlbumDetailVm, LibraryTrackRowVm, LibraryViewModel, MbStatusKind,
    MbTrackStatus, TrackRowWork,
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
/// The label of the primary action of an Index track row.
const INDEX_TRACK_PRIMARY_LABEL: &str = "Download";
/// The trigger label of the playlist popover of a track row.
const TRACK_PLAYLIST_TRIGGER_LABEL: &str = "+ Playlist";

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
        // ADR 0084 Decision 6: a single in a Library group links to its group.
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
            feed,
        }
    }

    /// Gives the name links to `links`, with the release key of the page.
    /// The Index host uses it to add the link to a Library singles group
    /// (ADR 0084 Decision 6).
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
    /// The primary action of the track row at this index.
    TrackPrimary(usize),
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

/// The primary action of a track row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReleaseTrackPrimaryVm {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) enabled: bool,
    /// The action removes the track. The renderer shows the destructive
    /// row style.
    pub(crate) destructive: bool,
}

/// The `MusicBrainz` status of a track row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ReleaseTrackStatusVm {
    pub(crate) text: &'static str,
    pub(crate) kind: Option<MbStatusKind>,
}

/// The one track row of a release page and of a singles group page, for
/// each origin.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReleaseTrackRowVm {
    pub(crate) row_id: String,
    pub(crate) row: TrackRowVm,
    pub(crate) primary: ReleaseTrackPrimaryVm,
    pub(crate) status: Option<ReleaseTrackStatusVm>,
    pub(crate) state_label: Option<&'static str>,
    pub(crate) playlist_popover_id: String,
    pub(crate) playlist_trigger_label: &'static str,
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
    /// the single. The row shows the date of the single.
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
        let action = vm.primary_action_vm(work);
        let display = vm.row_display();
        let track_view = TrackView::from_local(track.clone());
        let row = TrackDetailVm::new(&track_view, TrackDetailSurfaceContext::Library).row();
        let playlist = vm.playlist_display();
        Self {
            row_id: display.row_id,
            row,
            primary: ReleaseTrackPrimaryVm {
                id: display.toggle_button_id,
                label: action.label,
                enabled: action.enabled,
                destructive: action.tone == EntityActionTone::DestructiveQuiet,
            },
            status: vm.mb_status_text().map(|text| ReleaseTrackStatusVm {
                text,
                kind: vm.mb_status_kind(),
            }),
            state_label: display.state_label,
            playlist_popover_id: playlist.popover_id,
            playlist_trigger_label: playlist.trigger_label,
            row_key: DetailRowKey::for_open_entry(&open_entry),
            open_entry: Some(open_entry),
            open_label: None,
            selection,
            thumbnail_url: track
                .track_image_href
                .clone()
                .or_else(|| track.album_image_href.clone()),
        }
    }

    /// The row of an Index track. The title follows the title rule of
    /// `TrackDetailVm::display_title`: the title is trimmed, and a title of
    /// only spaces shows the fallback (orchestrator decision of
    /// 2026-10-20). A track with no track GUID gets no key and no click.
    #[must_use]
    pub(crate) fn index(
        feed: &FeedView,
        index: usize,
        track: &TrackView,
        selection: RowSelectionVm,
    ) -> Self {
        let track_guid = index_track_guid(track);
        let id_key = track_guid.clone().unwrap_or_else(|| index.to_string());
        let feed_guid = index_feed_guid(feed);
        let key = index_track_row_click_key(feed_guid.as_deref(), track_guid);
        // The search path uses the same label fallback: the track GUID.
        let open_label = key
            .as_ref()
            .map(|key| track.title.clone().unwrap_or_else(|| key.item_guid.clone()));
        Self {
            row_id: format!("entity-track:{index}"),
            row: TrackDetailVm::new(track, TrackDetailSurfaceContext::Discover).row(),
            primary: ReleaseTrackPrimaryVm {
                id: format!("index-track-download:{id_key}"),
                label: INDEX_TRACK_PRIMARY_LABEL.to_owned(),
                enabled: true,
                destructive: false,
            },
            status: None,
            state_label: None,
            playlist_popover_id: format!("index-track-add:{id_key}"),
            playlist_trigger_label: TRACK_PLAYLIST_TRIGGER_LABEL,
            row_key: key.clone().map(DetailRowKey::Track),
            open_entry: key.map(FrameNavigationEntry::Track),
            open_label,
            selection,
            thumbnail_url: index_track_row_artwork_url(feed, track).map(str::to_owned),
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
        ReleasePageOrigin::Library { .. } => input
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
            .collect(),
        ReleasePageOrigin::Index { .. } => input
            .feed
            .tracks
            .iter()
            .enumerate()
            .map(|(index, track)| {
                ReleaseTrackRowVm::index(&input.feed, index, track, input.selection(index))
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
    /// status, a Library row has the primary action, the status and the
    /// state label of `LibraryTrackRowVm`.
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

                assert_eq!(row.primary.label, action.label);
                assert_eq!(row.primary.enabled, action.enabled);
                assert_eq!(
                    row.primary.destructive,
                    action.tone == EntityActionTone::DestructiveQuiet
                );
                assert_eq!(row.primary.id, "lib-toggle-4");
                assert_eq!(row.row_id, "album-track-4");
                assert_eq!(
                    row.status.map(|status| status.text),
                    expected.mb_status_text()
                );
                assert_eq!(
                    row.status.and_then(|status| status.kind),
                    expected.mb_status_kind()
                );
                assert_eq!(row.state_label, expected.row_display().state_label);
                assert_eq!(row.playlist_popover_id, "album-track-add:4");
            }
        }
    }

    /// R21-04: an Index row has the primary label "Download", no status,
    /// no state label, and the element ids of the Index page before packet
    /// 021.
    #[test]
    fn adr_0084_release_page_r21_04_index_row_keeps_its_ids_and_download() {
        let feed = index_feed(vec![
            index_track(Some("No GUID"), None),
            index_track(Some("Song"), Some("guid-b")),
        ]);
        let vm_facts = ReleasePageFacts::from_index_feed(feed, None);
        let vm = release_page_vm(&vm_facts);

        let with_guid = &vm.rows[1];
        assert_eq!(with_guid.primary.label, "Download");
        assert!(with_guid.primary.enabled);
        assert!(!with_guid.primary.destructive);
        assert_eq!(with_guid.status, None);
        assert_eq!(with_guid.state_label, None);
        assert_eq!(with_guid.row_id, "entity-track:1");
        assert_eq!(with_guid.primary.id, "index-track-download:guid-b");
        assert_eq!(with_guid.playlist_popover_id, "index-track-add:guid-b");
        assert_eq!(with_guid.playlist_trigger_label, "+ Playlist");

        let without_guid = &vm.rows[0];
        assert_eq!(without_guid.row_id, "entity-track:0");
        assert_eq!(without_guid.primary.id, "index-track-download:0");
        assert_eq!(without_guid.playlist_popover_id, "index-track-add:0");
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
        let without = ReleaseTrackRowVm::index(&feed, 0, &feed.tracks[0], RowSelectionVm::none());
        assert_eq!(without.row_key, None);
        assert_eq!(without.open_entry, None);
        assert_eq!(without.open_label, None);

        let with = ReleaseTrackRowVm::index(&feed, 1, &feed.tracks[1], RowSelectionVm::none());
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
        assert_eq!(row.primary.id, "lib-toggle-9");
    }

    /// R21-07: `contributors_panel` is true for the Library origin and
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
}
