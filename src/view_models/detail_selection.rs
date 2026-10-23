//! The selection of the Music detail column (ADR 0084 Decision 10, packet
//! 018).
//!
//! Five detail pages have a list that the curator can select: the track
//! rows of a release page, the rows of a singles group page, the release
//! rows of a feed page, the Discover "New releases" view (packet 020) and
//! the entries of a playlist page (packet 025).
//! The detail column has one selection and
//! one mode. They belong to one page entry. A move to another page clears
//! the selection and sets [`SelectionMode::Normal`].
//!
//! This module is pure and carries no renderer type. It uses the model of
//! `list_selection.rs` and the bar of `selection_action_bar.rs`. It does
//! not copy them.

#![warn(clippy::pedantic)]

use std::collections::{BTreeMap, BTreeSet};

use crate::db::{Playlist, PlaylistEdge, PlaylistEntryKey, TrackRow};
use crate::view_models::library::ContentListRowDisplay;
use crate::view_models::list_selection::{
    FocusStep, ListSelection, RowSelectionVm, SelectionMode, SelectionRow,
};
use crate::view_models::publisher_page::{PublisherPageSectionVm, TitleDisplay};
use crate::view_models::release_row_actions::{
    ReleaseActionCounts, ReleaseRowAvailability, ReleaseRowState,
};
use crate::view_models::search_results::{
    FeedResultDisplay, SearchLibraryLookup, TrackResultDisplay,
};
use crate::view_models::selection_action_bar::{
    PlaylistSelectionCounts, SelectionActionBarVm, SelectionActionCounts,
};
use crate::view_models::singles_group::SinglesGroupPageEntryVm;
use crate::view_models::window_groups::{
    ContentListItemVm, NewReleasesLayout, NewReleasesLayoutItem, WindowGroupTileVm,
};
use crate::view_models::workspace::{
    DiscoverView, FrameHeaderActionDisplay, FrameNavigationEntry, ReleaseKey, TrackKey,
};
use crate::views::{TrackRef, TrackView};

/// The key of one detail row. A release page row is a track. A singles
/// group page row and a feed page row are a release. A playlist page row
/// is an entry: one track can have two entries (packet 025).
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum DetailRowKey {
    /// A track row of a release page.
    Track(TrackKey),
    /// A release row of a singles group page or a feed page.
    Release(ReleaseKey),
    /// An entry row of a playlist page.
    PlaylistEntry(PlaylistEntryKey),
}

impl DetailRowKey {
    /// The row key of a Library detail row, from the page that its click
    /// opens: a track page for a release page row, and a release page for
    /// a singles group page row. `None` for each other page.
    #[must_use]
    pub(crate) fn for_open_entry(entry: &FrameNavigationEntry) -> Option<Self> {
        match entry {
            FrameNavigationEntry::Track(key) => Some(Self::Track(key.clone())),
            FrameNavigationEntry::Release(key) => Some(Self::Release(key.clone())),
            _ => None,
        }
    }
}

/// The kind of list that a detail page shows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DetailListKind {
    /// The track rows of a release page.
    Tracks,
    /// The release rows of a singles group page or a feed page.
    Releases,
    /// The entry rows of a playlist page (packet 025).
    PlaylistEntries,
}

impl DetailListKind {
    /// The list kind of the page `entry`. A release page gives tracks. A
    /// singles group page, a feed page and the Discover "New releases"
    /// view give releases. A playlist page gives its entries. Each other
    /// page gives `None`: it has no selection.
    #[must_use]
    pub(crate) const fn for_entry(entry: &FrameNavigationEntry) -> Option<Self> {
        match entry {
            FrameNavigationEntry::Release(_) => Some(Self::Tracks),
            FrameNavigationEntry::SinglesGroup(_)
            | FrameNavigationEntry::PublisherDetail(_)
            | FrameNavigationEntry::ArtistName(_)
            | FrameNavigationEntry::DiscoverView(DiscoverView::NewReleases)
            | FrameNavigationEntry::WindowGroup(_) => Some(Self::Releases),
            FrameNavigationEntry::PlaylistDetail(_) => Some(Self::PlaylistEntries),
            _ => None,
        }
    }

    const fn plural_noun(self) -> &'static str {
        match self {
            Self::Tracks | Self::PlaylistEntries => "tracks",
            Self::Releases => "releases",
        }
    }

    const fn item_noun(self) -> &'static str {
        match self {
            Self::Tracks | Self::PlaylistEntries => "track",
            Self::Releases => "release",
        }
    }
}

/// The kind of a click on a detail row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DetailRowClick {
    /// A click with no modifier key.
    Plain,
    /// Ctrl+click, or Cmd+click on macOS.
    Toggle,
    /// Shift+click.
    Extend,
}

/// The downloaded tracks of one Library release (ADR 0084 task 031).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReleaseDownloaded {
    /// No track of the release is downloaded. A release with no track
    /// has this state.
    NoTrack,
    /// One or more tracks are downloaded, but not each track.
    SomeTracks,
    /// Each track of the release is downloaded.
    EachTrack,
}

impl ReleaseDownloaded {
    /// The state of a release with the download fact `downloaded` of
    /// each track.
    #[must_use]
    pub(crate) fn of(downloaded: impl IntoIterator<Item = bool>) -> Self {
        let (mut any, mut all, mut count) = (false, true, 0_usize);
        for track in downloaded {
            any |= track;
            all &= track;
            count += 1;
        }
        match (count > 0 && all, any) {
            (true, _) => Self::EachTrack,
            (false, true) => Self::SomeTracks,
            (false, false) => Self::NoTrack,
        }
    }
}

/// The Library facts of one release.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LibraryReleaseFacts {
    /// The Library feed id.
    pub(crate) feed_id: i64,
    /// The downloaded tracks of the release.
    pub(crate) downloaded: ReleaseDownloaded,
    /// A download job of the release key is queued or in flight. "Remove"
    /// is then absent (orchestrator correction of 2026-10-20).
    pub(crate) download_active: bool,
    /// A removal of the release runs. "Download" and "Remove" are then
    /// absent (ADR 0084 task 031, decision D4).
    pub(crate) removing: bool,
}

impl LibraryReleaseFacts {
    /// Tells if each track of the release is downloaded.
    #[must_use]
    pub(crate) const fn fully_downloaded(self) -> bool {
        matches!(self.downloaded, ReleaseDownloaded::EachTrack)
    }

    /// Tells if one or more tracks of the release are downloaded. The
    /// release page shows "Remove <kind>…" for this state (ADR 0084 task
    /// 031, operator answer of 2026-10-21).
    #[must_use]
    pub(crate) const fn has_library_track(self) -> bool {
        !matches!(self.downloaded, ReleaseDownloaded::NoTrack)
    }

    /// Tells if "Remove" can act on the release: one or more tracks of
    /// the release are downloaded, and no job or removal of it waits or
    /// runs. The release page uses the same rule (ADR 0084 task 031,
    /// operator answer of 2026-10-21).
    #[must_use]
    pub(crate) const fn removable(self) -> bool {
        self.has_library_track() && !self.download_active && !self.removing
    }

    /// Tells if "Download" can act on the release: the release is not
    /// fully downloaded, and no job or removal of it waits or runs (ADR
    /// 0084 task 031, decision D4).
    #[must_use]
    pub(crate) const fn downloadable(self) -> bool {
        !self.fully_downloaded() && !self.download_active && !self.removing
    }
}

/// One track row of a release page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DetailTrackRow {
    /// The key of the row. `None` for an Index track with no track GUID:
    /// that row has no checkbox and no focus stop.
    pub(crate) key: Option<TrackKey>,
    /// The title that the checkbox label names.
    pub(crate) title: String,
    /// The Library track id. `None` for a track that is not in the
    /// Library. Only a Library track goes into a playlist.
    pub(crate) library_track_id: Option<i64>,
    /// The audio file of the track is downloaded.
    pub(crate) downloaded: bool,
    /// A download job of the release key of the track is queued or in
    /// flight. "Remove" is then absent (orchestrator correction of
    /// 2026-10-20).
    pub(crate) download_active: bool,
}

impl DetailTrackRow {
    /// Tells if "Remove" can act on the track.
    #[must_use]
    pub(crate) const fn removable(&self) -> bool {
        self.downloaded && !self.download_active
    }
}

/// One release row of a singles group page or a feed page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DetailReleaseRow {
    /// The key of the row.
    pub(crate) key: ReleaseKey,
    /// The title that the checkbox label names.
    pub(crate) title: String,
    /// The Library facts. `None` for a release that is only in the Index.
    pub(crate) library: Option<LibraryReleaseFacts>,
}

impl DetailReleaseRow {
    /// The state of the row for its actions (ADR 0084 task 031). The
    /// detail rows hold no job fact of an Index release.
    #[must_use]
    pub(crate) const fn state(&self) -> ReleaseRowState {
        ReleaseRowState::of(self.library, false)
    }
}

/// The plan of "Download n releases" of a detail bar (ADR 0084 Decision
/// 10). [`DetailPageRows::release_download_plan`] makes it.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct DetailReleaseDownloadPlan {
    /// The feed id of each Library release to download.
    pub(crate) library_feed_ids: Vec<i64>,
    /// The feed GUID and the title of each Index release to download.
    pub(crate) index_releases: Vec<(String, String)>,
}

/// One entry row of a playlist page (packet 025).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DetailPlaylistEntryRow {
    /// The key of the row: its position and its track id.
    pub(crate) key: PlaylistEntryKey,
    /// The track page that a plain click in `Normal` mode opens.
    pub(crate) track: TrackKey,
    /// The title that the checkbox label names.
    pub(crate) title: String,
    /// The audio file of the track is downloaded.
    pub(crate) downloaded: bool,
    /// A download job of the release key of the track is queued or in
    /// flight. "Remove from Library…" is then absent.
    pub(crate) download_active: bool,
}

impl DetailPlaylistEntryRow {
    /// Tells if "Remove from Library…" can act on the track.
    #[must_use]
    pub(crate) const fn removable(&self) -> bool {
        self.downloaded && !self.download_active
    }
}

/// The selectable rows of the open detail page, in their visible order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum DetailPageRows {
    /// The track rows of a release page.
    Tracks(Vec<DetailTrackRow>),
    /// The release rows of a singles group page or a feed page.
    Releases(Vec<DetailReleaseRow>),
    /// The release rows of the Discover "New releases" view (ADR 0086
    /// packet 004). `rows` holds each shown release and each member of
    /// each group, in item order. The action bar counts these rows.
    /// `tiles` holds one selection row for each shown item: a plain item
    /// has one member, and a group tile has each member of its window.
    GroupedReleases {
        rows: Vec<DetailReleaseRow>,
        tiles: Vec<SelectionRow<DetailRowKey, DetailRowKey>>,
    },
    /// The entry rows of the playlist `playlist_id` (packet 025). The rows
    /// come from the full list, so a row that waits is also here.
    PlaylistEntries {
        playlist_id: i64,
        rows: Vec<DetailPlaylistEntryRow>,
    },
}

impl DetailPageRows {
    /// The list kind of these rows.
    #[must_use]
    pub(crate) const fn kind(&self) -> DetailListKind {
        match self {
            Self::Tracks(_) => DetailListKind::Tracks,
            Self::Releases(_) | Self::GroupedReleases { .. } => DetailListKind::Releases,
            Self::PlaylistEntries { .. } => DetailListKind::PlaylistEntries,
        }
    }

    /// The key of each row that has one, in the visible order.
    #[must_use]
    pub(crate) fn members(&self) -> Vec<DetailRowKey> {
        match self {
            Self::Tracks(rows) => rows
                .iter()
                .filter_map(|row| row.key.clone().map(DetailRowKey::Track))
                .collect(),
            Self::Releases(rows) | Self::GroupedReleases { rows, .. } => rows
                .iter()
                .map(|row| DetailRowKey::Release(row.key.clone()))
                .collect(),
            Self::PlaylistEntries { rows, .. } => rows
                .iter()
                .map(|row| DetailRowKey::PlaylistEntry(row.key))
                .collect(),
        }
    }

    /// The rows for [`ListSelection`]. Each row has one member: its own
    /// key. A row with no key is not in this list. The Discover view gives
    /// its tiles: a group tile has each member of its window (ADR 0086
    /// packet 004).
    #[must_use]
    pub(crate) fn selection_rows(&self) -> Vec<SelectionRow<DetailRowKey, DetailRowKey>> {
        if let Self::GroupedReleases { tiles, .. } = self {
            return tiles.clone();
        }
        self.members()
            .into_iter()
            .map(SelectionRow::of_one)
            .collect()
    }

    /// Tells if one or more rows have a key.
    #[must_use]
    pub(crate) fn has_selectable_rows(&self) -> bool {
        match self {
            Self::Tracks(rows) => rows.iter().any(|row| row.key.is_some()),
            Self::Releases(rows) => !rows.is_empty(),
            Self::GroupedReleases { tiles, .. } => !tiles.is_empty(),
            Self::PlaylistEntries { rows, .. } => !rows.is_empty(),
        }
    }

    /// The title of the row of `key`.
    #[must_use]
    pub(crate) fn title_of(&self, key: &DetailRowKey) -> Option<&str> {
        match (self, key) {
            (Self::Tracks(rows), DetailRowKey::Track(key)) => rows
                .iter()
                .find(|row| row.key.as_ref() == Some(key))
                .map(|row| row.title.as_str()),
            (
                Self::Releases(rows) | Self::GroupedReleases { rows, .. },
                DetailRowKey::Release(key),
            ) => rows
                .iter()
                .find(|row| &row.key == key)
                .map(|row| row.title.as_str()),
            (Self::PlaylistEntries { .. }, DetailRowKey::PlaylistEntry(key)) => {
                self.playlist_entry(key).map(|row| row.title.as_str())
            }
            _ => None,
        }
    }

    /// The playlist entry row of `key`.
    #[must_use]
    pub(crate) fn playlist_entry(&self, key: &PlaylistEntryKey) -> Option<&DetailPlaylistEntryRow> {
        match self {
            Self::PlaylistEntries { rows, .. } => rows.iter().find(|row| &row.key == key),
            Self::Tracks(_) | Self::Releases(_) | Self::GroupedReleases { .. } => None,
        }
    }

    /// The selected playlist entry rows, in their visible order.
    #[must_use]
    pub(crate) fn selected_entries(
        &self,
        selection: &DetailSelection,
    ) -> Vec<&DetailPlaylistEntryRow> {
        match self {
            Self::PlaylistEntries { rows, .. } => rows
                .iter()
                .filter(|row| {
                    selection
                        .selection
                        .is_selected(&DetailRowKey::PlaylistEntry(row.key))
                })
                .collect(),
            Self::Tracks(_) | Self::Releases(_) | Self::GroupedReleases { .. } => Vec::new(),
        }
    }

    /// The choices of "Add to playlist". A playlist page leaves out its
    /// own playlist (packet 025). Each other page gives each playlist.
    #[must_use]
    pub(crate) fn add_to_playlist_choices(&self, playlists: &[Playlist]) -> Vec<Playlist> {
        let open = match self {
            Self::PlaylistEntries { playlist_id, .. } => Some(*playlist_id),
            Self::Tracks(_) | Self::Releases(_) | Self::GroupedReleases { .. } => None,
        };
        playlists
            .iter()
            .filter(|playlist| Some(playlist.id) != open)
            .cloned()
            .collect()
    }

    /// The selected track rows, in their visible order.
    #[must_use]
    pub(crate) fn selected_tracks(&self, selection: &DetailSelection) -> Vec<&DetailTrackRow> {
        match self {
            Self::Tracks(rows) => rows
                .iter()
                .filter(|row| {
                    row.key.as_ref().is_some_and(|key| {
                        selection
                            .selection
                            .is_selected(&DetailRowKey::Track(key.clone()))
                    })
                })
                .collect(),
            Self::Releases(_) | Self::GroupedReleases { .. } | Self::PlaylistEntries { .. } => {
                Vec::new()
            }
        }
    }

    /// The selected release rows, in their visible order.
    #[must_use]
    pub(crate) fn selected_releases(&self, selection: &DetailSelection) -> Vec<&DetailReleaseRow> {
        match self {
            Self::Releases(rows) | Self::GroupedReleases { rows, .. } => rows
                .iter()
                .filter(|row| {
                    selection
                        .selection
                        .is_selected(&DetailRowKey::Release(row.key.clone()))
                })
                .collect(),
            Self::Tracks(_) | Self::PlaylistEntries { .. } => Vec::new(),
        }
    }

    /// The plan of "Download n releases": each selected release that
    /// [`ReleaseRowAvailability::of`] lets "Download" act on. The bar
    /// count uses the same rule, so the plan holds one job for each
    /// counted release (ADR 0084 task 031). An Index release with a key
    /// that is not a feed GUID has no job.
    #[must_use]
    pub(crate) fn release_download_plan(
        &self,
        selection: &DetailSelection,
    ) -> DetailReleaseDownloadPlan {
        let mut plan = DetailReleaseDownloadPlan::default();
        for row in self.selected_releases(selection) {
            if !ReleaseRowAvailability::of(row.state()).download {
                continue;
            }
            match (row.library, &row.key) {
                (Some(facts), _) => plan.library_feed_ids.push(facts.feed_id),
                (None, ReleaseKey::FeedGuid(feed_guid)) => plan
                    .index_releases
                    .push((feed_guid.clone(), row.title.clone())),
                (None, ReleaseKey::LibraryFeed(_)) => {}
            }
        }
        plan
    }

    /// The action bar of the detail column, or `None` outside
    /// [`SelectionMode::Selecting`].
    ///
    /// Track rows: "Download" counts the tracks that are not downloaded,
    /// "Add to playlist" counts the Library tracks, and "Remove" counts
    /// the downloaded tracks. Release rows: the counts of
    /// [`ReleaseActionCounts::of`]. "Download" counts an Index release and
    /// a Library release that is not fully downloaded, with no job and no
    /// removal. "Add to playlist" counts each Library release. "Remove"
    /// counts each Library release with one or more downloaded tracks, with
    /// no job and no removal. Playlist entries: see
    /// [`Self::playlist_action_bar`].
    #[must_use]
    pub(crate) fn action_bar(&self, selection: &DetailSelection) -> Option<SelectionActionBarVm> {
        let counts = match self {
            Self::PlaylistEntries { rows, .. } => {
                return Self::playlist_action_bar(
                    rows.len(),
                    &self.selected_entries(selection),
                    selection.mode,
                );
            }
            Self::Tracks(_) => {
                let selected = self.selected_tracks(selection);
                SelectionActionCounts {
                    selected_count: selected.len(),
                    item_noun: DetailListKind::Tracks.item_noun(),
                    download_count: selected.iter().filter(|row| !row.downloaded).count(),
                    add_count: selected
                        .iter()
                        .filter(|row| row.library_track_id.is_some())
                        .count(),
                    remove_count: selected.iter().filter(|row| row.removable()).count(),
                }
            }
            // ADR 0084 task 031: the release row menu and this bar count
            // through the one availability rule. The detail rows hold no
            // job fact of an Index release.
            Self::Releases(_) | Self::GroupedReleases { .. } => {
                let selected = self.selected_releases(selection);
                let counts = ReleaseActionCounts::of(selected.iter().map(|row| row.state()));
                SelectionActionCounts {
                    selected_count: selected.len(),
                    item_noun: DetailListKind::Releases.item_noun(),
                    download_count: counts.download,
                    add_count: counts.add_to_playlist,
                    remove_count: counts.remove,
                }
            }
        };
        SelectionActionBarVm::from_counts(selection.mode, counts)
    }

    /// The bar of a playlist page (packet 025). "Download" and "Remove
    /// from Library…" count distinct tracks, because one track can have
    /// two entries. "Move to top" is absent when the selected entries are
    /// the first `n` entries of the `total`. "Move to bottom" is absent
    /// when they are the last `n` entries.
    fn playlist_action_bar(
        total: usize,
        selected: &[&DetailPlaylistEntryRow],
        mode: SelectionMode,
    ) -> Option<SelectionActionBarVm> {
        let distinct = |keep: fn(&DetailPlaylistEntryRow) -> bool| {
            selected
                .iter()
                .filter(|row| keep(row))
                .map(|row| row.key.track_id)
                .collect::<BTreeSet<_>>()
                .len()
        };
        let count = selected.len();
        let index = |row: &&DetailPlaylistEntryRow| usize::try_from(row.key.position).ok();
        SelectionActionBarVm::for_playlist(
            mode,
            PlaylistSelectionCounts {
                selected_count: count,
                download_count: distinct(|row| !row.downloaded),
                remove_count: distinct(DetailPlaylistEntryRow::removable),
                at_top: selected
                    .iter()
                    .all(|row| index(row).is_some_and(|index| index < count)),
                at_bottom: selected.iter().all(|row| {
                    index(row).is_some_and(|index| index >= total.saturating_sub(count))
                }),
            },
        )
    }
}

/// The selection of the detail column, with its mode and its page entry.
///
/// `LibraryApp` holds one of these. The detail selection and the two list
/// column selections are independent.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct DetailSelection {
    entry: Option<FrameNavigationEntry>,
    selection: ListSelection<DetailRowKey>,
    mode: SelectionMode,
}

impl DetailSelection {
    /// An empty selection in [`SelectionMode::Normal`], with no page.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// A page change: clears the selection and the focus, sets
    /// [`SelectionMode::Normal`], and binds the selection to `entry`.
    pub(crate) fn reset(&mut self, entry: Option<FrameNavigationEntry>) {
        *self = Self {
            entry,
            ..Self::default()
        };
    }

    /// Binds the selection to the open page `entry`. A selection of
    /// another entry is cleared and set to [`SelectionMode::Normal`].
    /// Gives `true` when the entry changed.
    pub(crate) fn bind(&mut self, entry: &FrameNavigationEntry) -> bool {
        if self.entry.as_ref() == Some(entry) {
            return false;
        }
        self.reset(Some(entry.clone()));
        true
    }

    /// The page entry of this selection.
    #[must_use]
    pub(crate) fn entry(&self) -> Option<&FrameNavigationEntry> {
        self.entry.as_ref()
    }

    /// The mode of the detail column.
    #[must_use]
    pub(crate) const fn mode(&self) -> SelectionMode {
        self.mode
    }

    /// The number of selected rows.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn selected_count(&self) -> usize {
        self.selection.selected_count()
    }

    /// The row with the keyboard focus.
    #[must_use]
    pub(crate) fn focused(&self) -> Option<&DetailRowKey> {
        self.selection.focused()
    }

    /// The selection facts of a row with `key`, named `title`. A row with
    /// no key gets [`RowSelectionVm::none`]: no checkbox and no focus.
    #[must_use]
    pub(crate) fn row_selection(&self, key: Option<&DetailRowKey>, title: &str) -> RowSelectionVm {
        let Some(key) = key else {
            return RowSelectionVm::none();
        };
        RowSelectionVm::for_row(
            title,
            self.selection.is_selected(key),
            self.selection.focused() == Some(key),
            self.mode.is_selecting(),
        )
    }

    /// The selection facts of each row of `rows`, in their visible order.
    #[must_use]
    pub(crate) fn row_selections(&self, rows: &DetailPageRows) -> Vec<RowSelectionVm> {
        match rows {
            DetailPageRows::Tracks(rows) => rows
                .iter()
                .map(|row| {
                    let key = row.key.clone().map(DetailRowKey::Track);
                    self.row_selection(key.as_ref(), &row.title)
                })
                .collect(),
            DetailPageRows::Releases(rows) | DetailPageRows::GroupedReleases { rows, .. } => rows
                .iter()
                .map(|row| {
                    self.row_selection(Some(&DetailRowKey::Release(row.key.clone())), &row.title)
                })
                .collect(),
            DetailPageRows::PlaylistEntries { rows, .. } => rows
                .iter()
                .map(|row| {
                    self.row_selection(Some(&DetailRowKey::PlaylistEntry(row.key)), &row.title)
                })
                .collect(),
        }
    }

    /// A click on the body of the row `key`. Ctrl+click and Shift+click
    /// start [`SelectionMode::Selecting`] and change the selection. In
    /// `Selecting`, a plain click changes the selection of the row. Gives
    /// `true` when the caller must open the page of the row: a plain
    /// click in [`SelectionMode::Normal`].
    ///
    /// The row of `key` in `visible` gives the members, so a click on a
    /// group tile acts on each member of its window (ADR 0086 packet 004
    /// decision D12). A key that `visible` does not hold is a row of one
    /// member.
    pub(crate) fn click(
        &mut self,
        key: &DetailRowKey,
        click: DetailRowClick,
        visible: &[SelectionRow<DetailRowKey, DetailRowKey>],
    ) -> bool {
        let row = visible
            .iter()
            .find(|row| &row.id == key)
            .cloned()
            .unwrap_or_else(|| SelectionRow::of_one(key.clone()));
        match click {
            DetailRowClick::Toggle => {
                self.mode = SelectionMode::Selecting;
                self.selection.toggle(&row);
                false
            }
            DetailRowClick::Extend => {
                self.mode = SelectionMode::Selecting;
                self.selection.extend_to(&row, visible);
                false
            }
            DetailRowClick::Plain if self.mode.is_selecting() => {
                self.selection.toggle(&row);
                false
            }
            DetailRowClick::Plain => {
                self.selection.click(key.clone());
                true
            }
        }
    }

    /// A click on the checkbox of the row `key`. It starts
    /// [`SelectionMode::Selecting`] and changes the selection of the row.
    pub(crate) fn toggle_row(&mut self, key: &DetailRowKey) {
        self.mode = SelectionMode::Selecting;
        self.selection.toggle(&SelectionRow::of_one(key.clone()));
    }

    /// The checkbox of a group tile (ADR 0086 packet 004 decision D12): it
    /// starts [`SelectionMode::Selecting`]. When each member of `row` is
    /// selected, it clears each member. Else it selects each member.
    pub(crate) fn toggle_selection_row(&mut self, row: &SelectionRow<DetailRowKey, DetailRowKey>) {
        self.mode = SelectionMode::Selecting;
        self.selection.toggle(row);
    }

    /// The selection facts of a group tile, named `title` (ADR 0086 packet
    /// 004 decision D12). The checkbox is checked when each member is
    /// selected, and mixed when some members are selected. A mixed tile
    /// has no selected mark.
    #[must_use]
    pub(crate) fn tile_selection(
        &self,
        row: &SelectionRow<DetailRowKey, DetailRowKey>,
        title: &str,
    ) -> RowSelectionVm {
        RowSelectionVm::for_group_row(
            title,
            self.selection.row_check_state(&row.members),
            self.selection.focused() == Some(&row.id),
            self.mode.is_selecting(),
        )
    }

    /// Space: starts [`SelectionMode::Selecting`] and changes the
    /// selection of the row with the keyboard focus.
    pub(crate) fn toggle_focused(&mut self, visible: &[SelectionRow<DetailRowKey, DetailRowKey>]) {
        self.mode = SelectionMode::Selecting;
        self.selection.toggle_focused(visible);
    }

    /// Ctrl+A: starts [`SelectionMode::Selecting`] and selects each row.
    pub(crate) fn select_all(&mut self, visible: &[SelectionRow<DetailRowKey, DetailRowKey>]) {
        self.mode = SelectionMode::Selecting;
        self.selection.select_all(visible);
    }

    /// Up, Down, Page Up, Page Down, Home and End: moves the row focus.
    /// Gives the key of the row that gets the focus.
    pub(crate) fn move_focus(
        &mut self,
        step: FocusStep,
        visible: &[SelectionRow<DetailRowKey, DetailRowKey>],
    ) -> Option<DetailRowKey> {
        self.selection.move_focus(step, visible)
    }

    /// Escape: in [`SelectionMode::Selecting`], sets
    /// [`SelectionMode::Normal`] and clears the selection. Gives `false`
    /// in `Normal`, so the caller keeps the Back action of Escape.
    pub(crate) fn end_selecting(&mut self) -> bool {
        if !self.mode.is_selecting() {
            return false;
        }
        self.mode = SelectionMode::Normal;
        self.selection.clear();
        true
    }

    /// The header button: "Select" starts [`SelectionMode::Selecting`].
    /// "Done" sets [`SelectionMode::Normal`] and clears the selection.
    pub(crate) fn toggle_mode(&mut self) {
        if !self.end_selecting() {
            self.mode = SelectionMode::Selecting;
        }
    }

    /// "Clear" of the bar: clears the rows and keeps the mode.
    pub(crate) fn clear_rows(&mut self) {
        self.selection.clear();
    }

    /// Drops each selected key that `visible_members` no longer names,
    /// for example after a removal.
    pub(crate) fn retain_visible(&mut self, visible_members: &[DetailRowKey]) {
        self.selection.retain_visible(visible_members);
    }

    /// The selected playlist entries, in position order (packet 025).
    #[must_use]
    pub(crate) fn selected_playlist_entries(&self) -> Vec<PlaylistEntryKey> {
        let mut entries: Vec<PlaylistEntryKey> = self
            .selection
            .selected_keys()
            .filter_map(|key| match key {
                DetailRowKey::PlaylistEntry(entry) => Some(*entry),
                DetailRowKey::Track(_) | DetailRowKey::Release(_) => None,
            })
            .collect();
        entries.sort_unstable();
        entries
    }

    /// After "Move to top" or "Move to bottom" of a playlist of `total`
    /// entries (packet 025): each selected key, the anchor and the focus
    /// get the new position of their entry. The track id of each key
    /// stays, so the selection follows the moved entries.
    pub(crate) fn rekey_after_move(&mut self, edge: PlaylistEdge, total: usize) {
        let moved: Vec<i64> = self
            .selected_playlist_entries()
            .iter()
            .map(|entry| entry.position)
            .collect();
        let count = i64::try_from(moved.len()).unwrap_or(i64::MAX);
        let total = i64::try_from(total).unwrap_or(i64::MAX);
        self.selection.rekey(|key| match key {
            DetailRowKey::PlaylistEntry(entry) => {
                let before = moved.partition_point(|position| *position < entry.position);
                let before = i64::try_from(before).unwrap_or(i64::MAX);
                let position = match (moved.binary_search(&entry.position).is_ok(), edge) {
                    (true, PlaylistEdge::Top) => before,
                    (true, PlaylistEdge::Bottom) => total - count + before,
                    (false, PlaylistEdge::Top) => entry.position + count - before,
                    (false, PlaylistEdge::Bottom) => entry.position - before,
                };
                DetailRowKey::PlaylistEntry(PlaylistEntryKey {
                    position,
                    track_id: entry.track_id,
                })
            }
            other => other.clone(),
        });
    }
}

/// The header action of the detail column: "Select" in
/// [`SelectionMode::Normal`] and "Done" in [`SelectionMode::Selecting`].
/// `None` when the page has no selectable row, for example a track page or
/// a release with no track.
#[must_use]
pub(crate) fn detail_header_action(
    rows: Option<&DetailPageRows>,
    mode: SelectionMode,
) -> Option<FrameHeaderActionDisplay> {
    header_action_with_id_prefix("music-detail-column-select", rows, mode)
}

/// The "Select" and "Done" button of the search list in the list column
/// header (packet 010 "The selection"), by the rule of
/// [`detail_header_action`]. The Artists tab gives no rows, so it gives no
/// button.
#[must_use]
pub(crate) fn search_header_action(
    rows: Option<&DetailPageRows>,
    mode: SelectionMode,
) -> Option<FrameHeaderActionDisplay> {
    header_action_with_id_prefix("music-search-list-select", rows, mode)
}

fn header_action_with_id_prefix(
    id_prefix: &str,
    rows: Option<&DetailPageRows>,
    mode: SelectionMode,
) -> Option<FrameHeaderActionDisplay> {
    let rows = rows.filter(|rows| rows.has_selectable_rows())?;
    let noun = rows.kind().plural_noun();
    let element_id = format!("{id_prefix}-{noun}");
    Some(match mode {
        SelectionMode::Normal => {
            FrameHeaderActionDisplay::new(element_id, "Select", format!("Select {noun}"))
        }
        SelectionMode::Selecting => {
            FrameHeaderActionDisplay::new(element_id, "Done", format!("Done selecting {noun}"))
        }
    })
}

/// The title of a track row: its title, else its GUID, else "Untitled".
/// The Library row and the Index row of one track use this one rule, so
/// they give the same checkbox label.
#[must_use]
fn track_row_title(title: Option<&str>, guid: Option<&str>) -> String {
    title
        .or(guid)
        .map_or_else(|| "Untitled".to_owned(), str::to_owned)
}

/// The track GUID of an Index track: its stated GUID, else its `MusicIndex`
/// reference.
#[must_use]
pub(crate) fn index_track_guid(track: &TrackView) -> Option<String> {
    track.track_guid.clone().or_else(|| match &track.id {
        Some(TrackRef::Musicindex(track_guid)) => Some(track_guid.clone()),
        Some(TrackRef::LocalTrackId(_)) | None => None,
    })
}

/// The rows of a Library release page, from its tracks.
/// `download_active` is true while a download job of the release key is
/// queued or in flight.
#[must_use]
pub(crate) fn library_release_track_rows(
    tracks: &[TrackRow],
    download_active: bool,
) -> Vec<DetailTrackRow> {
    tracks
        .iter()
        .map(|track| DetailTrackRow {
            key: Some(track.track_key()),
            title: track_row_title(track.track_title.as_deref(), Some(&track.item_guid)),
            library_track_id: Some(track.id),
            downloaded: track.is_in_library,
            download_active,
        })
        .collect()
}

/// The rows of an Index release page, from the tracks of its feed. A
/// track with no track GUID has no key.
#[must_use]
pub(crate) fn index_release_track_rows(
    feed_guid: Option<&str>,
    tracks: &[TrackView],
) -> Vec<DetailTrackRow> {
    tracks
        .iter()
        .map(|track| {
            let guid = index_track_guid(track);
            DetailTrackRow {
                key: guid
                    .clone()
                    .map(|guid| TrackKey::from_index_track_target(feed_guid, guid)),
                title: track_row_title(track.title.as_deref(), guid.as_deref()),
                library_track_id: None,
                downloaded: false,
                download_active: false,
            }
        })
        .collect()
}

/// The rows of a singles group page: the Library rows and the `MusicIndex`
/// rows, in page order (packet 026 decision D13). `facts` gives the
/// Library facts of a release key.
#[must_use]
pub(crate) fn singles_group_release_rows(
    rows: &[SinglesGroupPageEntryVm],
    facts: impl Fn(&ReleaseKey) -> Option<LibraryReleaseFacts>,
) -> Vec<DetailReleaseRow> {
    rows.iter()
        .map(|row| DetailReleaseRow {
            key: row.key().clone(),
            title: match row {
                SinglesGroupPageEntryVm::Library(row) => {
                    track_row_title(row.track.track_title.as_deref(), Some(&row.track.item_guid))
                }
                SinglesGroupPageEntryVm::Index(row) => row.title.clone(),
            },
            library: facts(row.key()),
        })
        .collect()
}

/// The rows of a feed page, in the order of its sections. A row with no
/// open action has no key, so it is not in this list. `facts` gives the
/// Library facts of a release key.
#[must_use]
pub(crate) fn feed_page_release_rows(
    sections: &[PublisherPageSectionVm],
    facts: impl Fn(&ReleaseKey) -> Option<LibraryReleaseFacts>,
) -> Vec<DetailReleaseRow> {
    sections
        .iter()
        .flat_map(|section| section.rows.iter())
        .filter_map(|row| {
            let open = row.open.as_ref()?;
            let title = match &row.title {
                TitleDisplay::Stated(text) | TitleDisplay::Missing(text) => text.clone(),
            };
            Some(DetailReleaseRow {
                key: open.key.clone(),
                title,
                library: facts(&open.key),
            })
        })
        .collect()
}

/// The rows of a playlist page (packet 025), from the full list of its
/// entries in the visible order of `PlaylistDetailVm::visible_tracks`.
/// `download_active` tells if a download job of the release key of a
/// track is queued or in flight.
#[must_use]
pub(crate) fn playlist_entry_rows(
    entries: &[(usize, &TrackRow)],
    download_active: impl Fn(&TrackRow) -> bool,
) -> Vec<DetailPlaylistEntryRow> {
    entries
        .iter()
        .map(|(position, track)| DetailPlaylistEntryRow {
            key: PlaylistEntryKey {
                position: i64::try_from(*position).unwrap_or(i64::MAX),
                track_id: track.id,
            },
            track: track.track_key(),
            title: track_row_title(track.track_title.as_deref(), Some(&track.item_guid)),
            downloaded: track.is_in_library,
            download_active: download_active(track),
        })
        .collect()
}

/// The release rows of the search results (ADR 0084 Decision 10, packet
/// 010), from the merged rows in their visible order. A row with no key is
/// not in this list. `lookup` gives the Library facts, so the bar counts
/// follow packet 018.
#[must_use]
pub(crate) fn search_release_rows(
    rows: &[FeedResultDisplay],
    lookup: &dyn SearchLibraryLookup,
) -> Vec<DetailReleaseRow> {
    rows.iter()
        .filter_map(|row| {
            let key = row.key.clone()?;
            Some(DetailReleaseRow {
                title: row.label.clone(),
                library: lookup.release(&key).map(|release| release.facts),
                key,
            })
        })
        .collect()
}

/// The track rows of the search results (ADR 0084 Decision 10, packet
/// 010), from the merged rows in their visible order. An Index track with
/// no feed GUID has no key, so it has no checkbox.
#[must_use]
pub(crate) fn search_track_rows(
    rows: &[TrackResultDisplay],
    lookup: &dyn SearchLibraryLookup,
) -> Vec<DetailTrackRow> {
    rows.iter()
        .map(|row| {
            let library = row.key.as_ref().and_then(|key| lookup.track(key));
            DetailTrackRow {
                key: row.key.clone(),
                title: row.label.clone(),
                library_track_id: library.as_ref().map(|track| track.track_id),
                downloaded: library.as_ref().is_some_and(|track| track.downloaded),
                download_active: library.is_some_and(|track| track.download_active),
            }
        })
        .collect()
}

/// The key of a Discover row or tile (packet 020), from its `index-feed:`
/// row id. A row with no feed GUID has no key, so it has no checkbox and
/// no focus stop.
#[must_use]
pub(crate) fn discover_row_key(row: &ContentListRowDisplay) -> Option<DetailRowKey> {
    row.index_feed_guid()
        .map(|guid| DetailRowKey::Release(ReleaseKey::FeedGuid(guid.to_owned())))
}

/// The selection row of the group tile `tile`: the anchor key is its
/// identity, and each member of its window is a member (ADR 0086 packet
/// 004). `members` gives the row of each member, newest first.
fn window_group_selection_row(
    tile: &WindowGroupTileVm,
    members: &[&ContentListRowDisplay],
) -> SelectionRow<DetailRowKey, DetailRowKey> {
    SelectionRow::new(
        DetailRowKey::Release(tile.page_key.anchor.clone()),
        members
            .iter()
            .filter_map(|row| discover_row_key(row))
            .collect(),
    )
}

/// The rows of each member of the window of `tile`, newest first.
fn window_group_member_rows<'r>(
    layout: &NewReleasesLayout,
    tile: &WindowGroupTileVm,
    rows_by_id: &BTreeMap<&str, &'r ContentListRowDisplay>,
) -> Vec<&'r ContentListRowDisplay> {
    layout
        .window_member_row_ids(&tile.page_key)
        .into_iter()
        .filter_map(|row_id| rows_by_id.get(row_id).copied())
        .collect()
}

/// The rows of the Discover "New releases" view (packet 020, ADR 0086
/// packet 004), in item order.
///
/// - `rows` of the result holds one release row for each shown release
///   and for each member of each group. A row with no key is not in it.
/// - `tiles` of the result holds one selection row for each shown item
///   with a key. A group tile holds each member of its window.
///
/// When `layout` has no item, each row is a plain item. `facts` gives the
/// Library facts of a release key, so the bar counts follow packet 018.
#[must_use]
pub(crate) fn discover_grouped_release_rows(
    layout: &NewReleasesLayout,
    rows: &[&ContentListRowDisplay],
    facts: impl Fn(&ReleaseKey) -> Option<LibraryReleaseFacts>,
) -> DetailPageRows {
    let release_row = |row: &ContentListRowDisplay| match discover_row_key(row)? {
        DetailRowKey::Release(key) => Some(DetailReleaseRow {
            title: row.title().to_owned(),
            library: facts(&key),
            key,
        }),
        DetailRowKey::Track(_) | DetailRowKey::PlaylistEntry(_) => None,
    };
    let mut release_rows = Vec::new();
    let mut tiles = Vec::new();
    if layout.items().is_empty() {
        for row in rows {
            if let Some(release) = release_row(row) {
                tiles.push(SelectionRow::of_one(DetailRowKey::Release(
                    release.key.clone(),
                )));
                release_rows.push(release);
            }
        }
        return DetailPageRows::GroupedReleases {
            rows: release_rows,
            tiles,
        };
    }
    let rows_by_id: BTreeMap<&str, &ContentListRowDisplay> =
        rows.iter().map(|row| (row.id.as_str(), *row)).collect();
    for item in layout.items() {
        match item {
            NewReleasesLayoutItem::Release { row_id } => {
                if let Some(release) = rows_by_id
                    .get(row_id.as_str())
                    .and_then(|row| release_row(row))
                {
                    tiles.push(SelectionRow::of_one(DetailRowKey::Release(
                        release.key.clone(),
                    )));
                    release_rows.push(release);
                }
            }
            NewReleasesLayoutItem::WindowGroup(tile) => {
                let members = window_group_member_rows(layout, tile, &rows_by_id);
                tiles.push(window_group_selection_row(tile, &members));
                release_rows.extend(members.into_iter().filter_map(release_row));
            }
        }
    }
    DetailPageRows::GroupedReleases {
        rows: release_rows,
        tiles,
    }
}

/// The items of the Discover "New releases" view with the selection facts
/// of the detail column (packet 020, ADR 0086 packets 002 and 004). The
/// row view and the tile view use these same items.
///
/// The items follow `layout` in both modes (packet 004 decision D11). A
/// plain item gives its row. A group item gives its group tile, with the
/// selection facts of [`DetailSelection::tile_selection`]: in
/// `SelectionMode::Selecting`, the tile shows a checkbox that can be
/// checked, mixed or clear. When `layout` has no item, for example when
/// the content list shows no New releases rows, each row gives its row.
///
/// A row with no key gets [`RowSelectionVm::none`].
#[must_use]
pub(crate) fn discover_items_with_selection(
    rows: Vec<ContentListRowDisplay>,
    layout: &NewReleasesLayout,
    selection: &DetailSelection,
) -> Vec<ContentListItemVm> {
    let with_selection = |mut row: ContentListRowDisplay| {
        row.selection = selection.row_selection(discover_row_key(&row).as_ref(), row.title());
        ContentListItemVm::Row(row)
    };
    if layout.items().is_empty() {
        return rows.into_iter().map(with_selection).collect();
    }
    let rows_by_id: BTreeMap<&str, &ContentListRowDisplay> =
        rows.iter().map(|row| (row.id.as_str(), row)).collect();
    let tile_rows: Vec<Option<SelectionRow<DetailRowKey, DetailRowKey>>> = layout
        .items()
        .iter()
        .map(|item| match item {
            NewReleasesLayoutItem::WindowGroup(tile) => Some(window_group_selection_row(
                tile,
                &window_group_member_rows(layout, tile, &rows_by_id),
            )),
            NewReleasesLayoutItem::Release { .. } => None,
        })
        .collect();
    let mut rows_by_id: BTreeMap<String, ContentListRowDisplay> =
        rows.into_iter().map(|row| (row.id.clone(), row)).collect();
    layout
        .items()
        .iter()
        .zip(tile_rows)
        .filter_map(|(item, tile_row)| match (item, tile_row) {
            (NewReleasesLayoutItem::Release { row_id }, _) => {
                rows_by_id.remove(row_id).map(with_selection)
            }
            (NewReleasesLayoutItem::WindowGroup(tile), tile_row) => {
                let mut tile = tile.as_ref().clone();
                tile.selection = tile_row.map_or_else(RowSelectionVm::none, |row| {
                    selection.tile_selection(&row, &tile.title)
                });
                Some(ContentListItemVm::WindowGroup(Box::new(tile)))
            }
        })
        .collect()
}

/// Left or Right on the Discover tiles (packet 020). The tile focus moves
/// in a line: the number of tiles in a row is a layout fact, and the view
/// model holds no layout fact. Up and Down use [`FocusStep::Up`] and
/// [`FocusStep::Down`] directly.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TileArrow {
    Left,
    Right,
}

impl TileArrow {
    /// Left goes to the tile before, the same as Up. Right goes to the
    /// tile after, the same as Down.
    #[must_use]
    pub(crate) const fn focus_step(self) -> FocusStep {
        match self {
            Self::Left => FocusStep::Up,
            Self::Right => FocusStep::Down,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_models::artist_identity::ArtistIdentity;
    use crate::view_models::list_selection::RowCheckState;
    use crate::view_models::publisher_page::{PublisherPageAlbumVm, ReleaseRowOpenAction};
    use crate::view_models::selection_action_bar::SelectionActionKind;
    use crate::view_models::singles_group::SinglesGroupKey;

    const FEED_GUID: &str = "feed-face-pale";

    fn library_track(id: i64, item_guid: &str, title: &str, downloaded: bool) -> TrackRow {
        TrackRow {
            id,
            feed_id: 7,
            feed_guid: Some(FEED_GUID.to_owned()),
            item_guid: item_guid.to_owned(),
            track_title: Some(title.to_owned()),
            is_in_library: downloaded,
            ..TrackRow::default()
        }
    }

    fn index_track(guid: Option<&str>, title: &str) -> TrackView {
        TrackView {
            track_guid: guid.map(str::to_owned),
            feed_guid: Some(FEED_GUID.to_owned()),
            title: Some(title.to_owned()),
            ..TrackView::default()
        }
    }

    fn release_entry() -> FrameNavigationEntry {
        FrameNavigationEntry::Release(ReleaseKey::FeedGuid(FEED_GUID.to_owned()))
    }

    fn track_key(guid: &str) -> DetailRowKey {
        DetailRowKey::Track(TrackKey::from_index_track_target(Some(FEED_GUID), guid))
    }

    fn release_key(guid: &str) -> ReleaseKey {
        ReleaseKey::FeedGuid(guid.to_owned())
    }

    fn library_facts(feed_id: i64, fully_downloaded: bool) -> LibraryReleaseFacts {
        LibraryReleaseFacts {
            feed_id,
            downloaded: if fully_downloaded {
                ReleaseDownloaded::EachTrack
            } else {
                ReleaseDownloaded::NoTrack
            },
            download_active: false,
            removing: false,
        }
    }

    fn feed_row(guid: &str, title: &str) -> PublisherPageAlbumVm {
        PublisherPageAlbumVm {
            feed_guid: Some(guid.to_owned()),
            title: TitleDisplay::Stated(title.to_owned()),
            image_url: None,
            artist: None,
            not_listed: false,
            in_library: false,
            download_state:
                crate::view_models::release_download_state::ReleaseDownloadStateVm::in_library(
                    false,
                ),
            detail_line: None,
            open: Some(ReleaseRowOpenAction {
                key: release_key(guid),
                a11y_label: format!("Open the release {title}"),
            }),
            selection: RowSelectionVm::none(),
            kind: crate::view_models::release_kind::ReleaseKind::Album,
            menu: None,
        }
    }

    /// R18-01: the detail selection of one page entry is empty and in
    /// `Normal` for another entry. A page change clears it.
    #[test]
    fn adr_0084_detail_selection_r18_01_a_page_change_clears_the_selection() {
        let mut selection = DetailSelection::new();
        let page_a = release_entry();
        let page_b = FrameNavigationEntry::PublisherDetail("feed-artist".to_owned());

        assert!(selection.bind(&page_a));
        selection.toggle_row(&track_key("t1"));
        assert_eq!(selection.mode(), SelectionMode::Selecting);
        assert_eq!(selection.selected_count(), 1);
        assert!(
            !selection.bind(&page_a),
            "the same page keeps its selection"
        );
        assert_eq!(selection.selected_count(), 1);

        assert!(selection.bind(&page_b));
        assert_eq!(selection.mode(), SelectionMode::Normal);
        assert_eq!(selection.selected_count(), 0);
        assert_eq!(selection.focused(), None);
        assert_eq!(selection.entry(), Some(&page_b));

        // Back to page A is a page change too: the selection stays empty.
        selection.toggle_row(&DetailRowKey::Release(release_key("r1")));
        selection.reset(Some(page_a.clone()));
        assert_eq!(selection.mode(), SelectionMode::Normal);
        assert_eq!(selection.selected_count(), 0);
        assert_eq!(selection.entry(), Some(&page_a));
    }

    /// R18-02: a Library release page and an Index release page of one
    /// release give the same visible rows and the same `RowSelectionVm`
    /// for each track. A track with no key has no checkbox.
    #[test]
    fn adr_0084_detail_selection_r18_02_library_and_index_rows_agree() {
        let library = DetailPageRows::Tracks(library_release_track_rows(
            &[
                library_track(1, "t1", "Face Pale", true),
                library_track(2, "t2", "Haleen", false),
            ],
            false,
        ));
        let index = DetailPageRows::Tracks(index_release_track_rows(
            Some(FEED_GUID),
            &[
                index_track(Some("t1"), "Face Pale"),
                index_track(Some("t2"), "Haleen"),
            ],
        ));
        assert_eq!(library.selection_rows(), index.selection_rows());

        let mut selection = DetailSelection::new();
        selection.bind(&release_entry());
        selection.toggle_row(&track_key("t2"));
        assert_eq!(
            selection.row_selections(&library),
            selection.row_selections(&index)
        );
        let vms = selection.row_selections(&library);
        assert_eq!(vms[1].check_state, RowCheckState::Checked);
        assert_eq!(vms[1].checkbox_a11y_label, "Select Haleen");

        let no_key = DetailPageRows::Tracks(index_release_track_rows(
            Some(FEED_GUID),
            &[index_track(None, "No GUID")],
        ));
        assert!(no_key.selection_rows().is_empty());
        let vm = &selection.row_selections(&no_key)[0];
        assert!(!vm.checkbox_visible);
        assert!(!vm.focused);
    }

    /// R18-03: in `Normal`, no detail row view model shows a checkbox. In
    /// `Selecting`, each row with a key shows one.
    #[test]
    fn adr_0084_detail_selection_r18_03_checkbox_follows_the_mode() {
        let rows = DetailPageRows::Tracks(index_release_track_rows(
            Some(FEED_GUID),
            &[
                index_track(Some("t1"), "Face Pale"),
                index_track(None, "No GUID"),
            ],
        ));
        let mut selection = DetailSelection::new();
        selection.bind(&release_entry());
        assert!(selection
            .row_selections(&rows)
            .iter()
            .all(|vm| !vm.checkbox_visible));

        selection.toggle_mode();
        let vms = selection.row_selections(&rows);
        assert!(vms[0].checkbox_visible);
        assert!(
            !vms[1].checkbox_visible,
            "a row with no key has no checkbox"
        );

        let releases = DetailPageRows::Releases(feed_page_release_rows(
            &[PublisherPageSectionVm {
                heading: "Albums and EPs".to_owned(),
                rows: vec![feed_row("r1", "Haleen")],
                note: None,
                status: None,
            }],
            |_| None,
        ));
        assert!(selection.row_selections(&releases)[0].checkbox_visible);
    }

    /// R18-03: a plain click opens the page in `Normal` and changes the
    /// selection in `Selecting`. Ctrl+click and Shift+click start
    /// `Selecting`.
    #[test]
    fn adr_0084_detail_selection_r18_03_clicks_follow_the_mode() {
        let rows = DetailPageRows::Tracks(library_release_track_rows(
            &[
                library_track(1, "t1", "Face Pale", true),
                library_track(2, "t2", "Haleen", true),
                library_track(3, "t3", "Rain", true),
            ],
            false,
        ));
        let visible = rows.selection_rows();
        let mut selection = DetailSelection::new();
        selection.bind(&release_entry());

        assert!(selection.click(&track_key("t1"), DetailRowClick::Plain, &visible));
        assert_eq!(selection.selected_count(), 0);

        assert!(!selection.click(&track_key("t1"), DetailRowClick::Toggle, &visible));
        assert_eq!(selection.mode(), SelectionMode::Selecting);
        assert!(!selection.click(&track_key("t3"), DetailRowClick::Extend, &visible));
        assert_eq!(selection.selected_count(), 3);

        assert!(!selection.click(&track_key("t2"), DetailRowClick::Plain, &visible));
        assert_eq!(selection.selected_count(), 2);

        selection.clear_rows();
        assert_eq!(selection.mode(), SelectionMode::Selecting);
        assert!(selection.end_selecting());
        assert_eq!(selection.mode(), SelectionMode::Normal);
        assert!(!selection.end_selecting(), "Escape in Normal keeps Back");

        selection.move_focus(FocusStep::Home, &visible);
        selection.select_all(&visible);
        assert_eq!(selection.mode(), SelectionMode::Selecting);
        assert_eq!(selection.selected_count(), 3);
        selection.toggle_focused(&visible);
        assert_eq!(selection.selected_count(), 2);
    }

    /// R18-04: the detail header action reads "Select" and "Done" with an
    /// accessibility label on the three pages. It is absent on a track
    /// page and on a release with no track.
    #[test]
    fn adr_0084_detail_selection_r18_04_header_action_reads_select_and_done() {
        let tracks = DetailPageRows::Tracks(library_release_track_rows(
            &[library_track(1, "t1", "Face Pale", true)],
            false,
        ));
        let select = detail_header_action(Some(&tracks), SelectionMode::Normal)
            .expect("a release page with a track has the action");
        assert_eq!(select.label, "Select");
        assert_eq!(select.a11y_label, "Select tracks");
        let done = detail_header_action(Some(&tracks), SelectionMode::Selecting)
            .expect("a release page with a track has the action");
        assert_eq!(done.label, "Done");
        assert_eq!(done.a11y_label, "Done selecting tracks");

        let group_rows = vec![SinglesGroupPageEntryVm::Library(Box::new(
            crate::view_models::singles_group::SinglesGroupPageRowVm {
                track: library_track(5, "s1", "Single", true),
                date_text: None,
                work: crate::view_models::library::TrackRowWork::Idle,
                key: release_key("single-1"),
            },
        ))];
        let group = DetailPageRows::Releases(singles_group_release_rows(&group_rows, |_| None));
        let group_action = detail_header_action(Some(&group), SelectionMode::Normal)
            .expect("a group page has the action");
        assert_eq!(group_action.a11y_label, "Select releases");

        let feed = DetailPageRows::Releases(feed_page_release_rows(
            &[PublisherPageSectionVm {
                heading: "Singles".to_owned(),
                rows: vec![feed_row("r1", "Haleen")],
                note: None,
                status: None,
            }],
            |_| None,
        ));
        let feed_action = detail_header_action(Some(&feed), SelectionMode::Selecting)
            .expect("a feed page has the action");
        assert_eq!(feed_action.a11y_label, "Done selecting releases");

        for entry in [
            release_entry(),
            FrameNavigationEntry::SinglesGroup(SinglesGroupKey(ArtistIdentity::Name(
                "haleen".to_owned(),
            ))),
            FrameNavigationEntry::PublisherDetail("feed-artist".to_owned()),
            // Packet 025 gave the playlist page its selection.
            FrameNavigationEntry::PlaylistDetail(3),
        ] {
            assert!(DetailListKind::for_entry(&entry).is_some());
        }
        let track_page =
            FrameNavigationEntry::Track(TrackKey::from_index_track_target(Some(FEED_GUID), "t1"));
        assert_eq!(DetailListKind::for_entry(&track_page), None);
        assert!(detail_header_action(None, SelectionMode::Normal).is_none());
        let empty = DetailPageRows::Tracks(Vec::new());
        assert!(detail_header_action(Some(&empty), SelectionMode::Normal).is_none());
    }

    /// R18-05: the track bar. "Download" counts only the tracks that are
    /// not downloaded, "Add to playlist" only the Library tracks, and
    /// "Remove" only the downloaded tracks. Each is absent with a count of
    /// 0.
    #[test]
    fn adr_0084_detail_selection_r18_05_track_bar_counts() {
        let mut library_rows = library_release_track_rows(
            &[
                library_track(1, "t1", "Face Pale", true),
                library_track(2, "t2", "Haleen", false),
            ],
            false,
        );
        library_rows.extend(index_release_track_rows(
            Some(FEED_GUID),
            &[index_track(Some("t3"), "Rain")],
        ));
        let rows = DetailPageRows::Tracks(library_rows);
        let visible = rows.selection_rows();
        let mut selection = DetailSelection::new();
        selection.bind(&release_entry());
        selection.select_all(&visible);

        let bar = rows.action_bar(&selection).expect("Selecting shows a bar");
        assert_eq!(bar.count_label, "3 tracks selected");
        let label = |kind| bar.action(kind).map(|action| action.label.clone());
        assert_eq!(
            label(SelectionActionKind::Download).as_deref(),
            Some("Download 2 tracks")
        );
        assert_eq!(
            label(SelectionActionKind::AddToPlaylist).as_deref(),
            Some("Add 2 tracks to playlist")
        );
        assert_eq!(
            label(SelectionActionKind::Remove).as_deref(),
            Some("Remove 1 track\u{2026}")
        );

        selection.clear_rows();
        selection.toggle_row(&track_key("t3"));
        let index_only = rows.action_bar(&selection).expect("Selecting shows a bar");
        assert!(index_only.action(SelectionActionKind::Download).is_some());
        assert!(index_only
            .action(SelectionActionKind::AddToPlaylist)
            .is_none());
        assert!(index_only.action(SelectionActionKind::Remove).is_none());

        selection.clear_rows();
        selection.toggle_row(&track_key("t1"));
        let downloaded_only = rows.action_bar(&selection).expect("Selecting shows a bar");
        assert!(downloaded_only
            .action(SelectionActionKind::Download)
            .is_none());
        assert!(downloaded_only
            .action(SelectionActionKind::AddToPlaylist)
            .is_some());
        assert!(downloaded_only
            .action(SelectionActionKind::Remove)
            .is_some());

        assert!(selection.end_selecting());
        assert!(rows.action_bar(&selection).is_none());
    }

    /// R18-06: the release bar of the feed page counts an Index release in
    /// "Download" only, and a Library release in "Add to playlist" and
    /// "Remove".
    #[test]
    fn adr_0084_detail_selection_r18_06_release_bar_counts() {
        let sections = [PublisherPageSectionVm {
            heading: "Albums and EPs".to_owned(),
            rows: vec![
                feed_row("index-only", "Rain"),
                feed_row("library", "Haleen"),
            ],
            note: None,
            status: None,
        }];
        let rows = DetailPageRows::Releases(feed_page_release_rows(&sections, |key| {
            (key == &release_key("library")).then_some(library_facts(9, true))
        }));
        let visible = rows.selection_rows();
        let mut selection = DetailSelection::new();
        selection.bind(&FrameNavigationEntry::PublisherDetail(
            "feed-artist".to_owned(),
        ));
        selection.select_all(&visible);

        let bar = rows.action_bar(&selection).expect("Selecting shows a bar");
        let label = |kind| bar.action(kind).map(|action| action.label.clone());
        assert_eq!(
            label(SelectionActionKind::Download).as_deref(),
            Some("Download 1 release")
        );
        assert_eq!(
            label(SelectionActionKind::AddToPlaylist).as_deref(),
            Some("Add 1 release to playlist")
        );
        assert_eq!(
            label(SelectionActionKind::Remove).as_deref(),
            Some("Remove 1 release\u{2026}")
        );

        selection.clear_rows();
        selection.toggle_row(&DetailRowKey::Release(release_key("index-only")));
        let index_only = rows.action_bar(&selection).expect("Selecting shows a bar");
        assert!(index_only.action(SelectionActionKind::Download).is_some());
        assert!(index_only
            .action(SelectionActionKind::AddToPlaylist)
            .is_none());
        assert!(index_only.action(SelectionActionKind::Remove).is_none());
    }

    /// ADR 0084 task 031, operator answer of 2026-10-21: the downloaded
    /// tracks of a release select "Remove". A release with no track has
    /// no downloaded track.
    #[test]
    fn adr_0084_release_row_actions_release_downloaded_state() {
        assert_eq!(ReleaseDownloaded::of([]), ReleaseDownloaded::NoTrack);
        assert_eq!(
            ReleaseDownloaded::of([false, false]),
            ReleaseDownloaded::NoTrack
        );
        assert_eq!(
            ReleaseDownloaded::of([false, true]),
            ReleaseDownloaded::SomeTracks
        );
        assert_eq!(
            ReleaseDownloaded::of([true, true]),
            ReleaseDownloaded::EachTrack
        );
    }

    /// Review correction B of 2026-10-21 (ADR 0084 task 031): the plan of
    /// "Download n releases" of a detail bar holds one job for each
    /// release that the bar counts. A release that is fully downloaded,
    /// with a job, or in a removal gets no job. The detail rows hold no
    /// job fact of an Index release.
    #[test]
    fn adr_0084_release_row_actions_detail_download_plan_matches_the_bar_count() {
        let facts = |feed_id, fully_downloaded, download_active, removing| LibraryReleaseFacts {
            feed_id,
            downloaded: if fully_downloaded {
                ReleaseDownloaded::EachTrack
            } else {
                ReleaseDownloaded::SomeTracks
            },
            download_active,
            removing,
        };
        let rows = DetailPageRows::Releases(vec![
            DetailReleaseRow {
                key: release_key("normal"),
                title: "Normal".to_owned(),
                library: Some(facts(1, false, false, false)),
            },
            DetailReleaseRow {
                key: release_key("full"),
                title: "Full".to_owned(),
                library: Some(facts(2, true, false, false)),
            },
            DetailReleaseRow {
                key: release_key("job"),
                title: "Job".to_owned(),
                library: Some(facts(3, false, true, false)),
            },
            DetailReleaseRow {
                key: release_key("removing"),
                title: "Removing".to_owned(),
                library: Some(facts(4, false, false, true)),
            },
            DetailReleaseRow {
                key: release_key("index"),
                title: "Index".to_owned(),
                library: None,
            },
        ]);
        let mut selection = DetailSelection::new();
        selection.bind(&FrameNavigationEntry::PublisherDetail(
            "feed-artist".to_owned(),
        ));
        selection.select_all(&rows.selection_rows());

        let plan = rows.release_download_plan(&selection);
        assert_eq!(plan.library_feed_ids, [1]);
        assert_eq!(
            plan.index_releases,
            [("index".to_owned(), "Index".to_owned())]
        );
        let bar = rows.action_bar(&selection).expect("Selecting shows a bar");
        let download = bar
            .action(SelectionActionKind::Download)
            .expect("two releases can download");
        assert_eq!(download.label, "Download 2 releases");
        assert_eq!(
            plan.library_feed_ids.len() + plan.index_releases.len(),
            2,
            "the plan size equals the bar count"
        );
    }

    fn discover_row(id: &str, title: &str) -> ContentListRowDisplay {
        ContentListRowDisplay::new(
            id,
            title,
            "Haleen",
            crate::view_models::library::ContentListRowSource::Index,
        )
    }

    /// The selection facts of each item, in item order.
    fn item_selections(items: &[ContentListItemVm]) -> Vec<RowSelectionVm> {
        items
            .iter()
            .map(|item| match item {
                ContentListItemVm::Row(row) => row.selection.clone(),
                ContentListItemVm::WindowGroup(tile) => tile.selection.clone(),
            })
            .collect()
    }

    /// The rows of the Discover view of `rows`, with no window group.
    fn discover_rows(
        rows: &[ContentListRowDisplay],
        facts: impl Fn(&ReleaseKey) -> Option<LibraryReleaseFacts>,
    ) -> DetailPageRows {
        let rows: Vec<&ContentListRowDisplay> = rows.iter().collect();
        discover_grouped_release_rows(&NewReleasesLayout::default(), &rows, facts)
    }

    fn discover_entry() -> FrameNavigationEntry {
        FrameNavigationEntry::DiscoverView(DiscoverView::NewReleases)
    }

    /// R20-01: the Discover "New releases" view gives `Releases`.
    #[test]
    fn adr_0084_discover_selection_r20_01_new_releases_gives_releases() {
        assert_eq!(
            DetailListKind::for_entry(&discover_entry()),
            Some(DetailListKind::Releases)
        );
    }

    /// R20-02: only an `index-feed:` row gives a key. A row with no feed
    /// GUID has no key and no checkbox.
    #[test]
    fn adr_0084_discover_selection_r20_02_only_an_index_feed_row_gives_a_key() {
        assert_eq!(
            discover_row_key(&discover_row("index-feed:face-pale", "Face Pale")),
            Some(DetailRowKey::Release(release_key("face-pale")))
        );
        for id in ["index-feed:", "42", "library-feed:7"] {
            assert_eq!(discover_row_key(&discover_row(id, "No GUID")), None, "{id}");
        }

        let rows = vec![
            discover_row("index-feed:face-pale", "Face Pale"),
            discover_row("index-feed:", "No GUID"),
        ];
        let releases = discover_rows(&rows, |_| None);
        assert_eq!(releases.members().len(), 1);
        assert_eq!(releases.selection_rows().len(), 1);
        assert_eq!(
            releases.title_of(&DetailRowKey::Release(release_key("face-pale"))),
            Some("Face Pale")
        );

        let mut selection = DetailSelection::new();
        selection.bind(&discover_entry());
        selection.toggle_mode();
        let rows = item_selections(&discover_items_with_selection(
            rows,
            &NewReleasesLayout::default(),
            &selection,
        ));
        assert!(rows[0].checkbox_visible);
        assert!(!rows[1].checkbox_visible);
        assert!(!rows[1].focused);
    }

    /// R20-03: the Discover bar counts follow packet 018 R18-06. "Download"
    /// counts the Index releases and the Library releases that are not
    /// fully downloaded. "Add to playlist" and "Remove" count the Library
    /// releases.
    #[test]
    fn adr_0084_discover_selection_r20_03_bar_counts_follow_packet_018() {
        let rows = [
            discover_row("index-feed:index-only", "Rain"),
            discover_row("index-feed:downloaded", "Haleen"),
            discover_row("index-feed:partial", "Face Pale"),
        ];
        let rows = discover_rows(&rows, |key| {
            if key == &release_key("downloaded") {
                Some(library_facts(9, true))
            } else if key == &release_key("partial") {
                Some(library_facts(10, false))
            } else {
                None
            }
        });
        let mut selection = DetailSelection::new();
        selection.bind(&discover_entry());
        selection.select_all(&rows.selection_rows());

        let bar = rows.action_bar(&selection).expect("Selecting shows a bar");
        assert_eq!(bar.count_label, "3 releases selected");
        let label = |kind| bar.action(kind).map(|action| action.label.clone());
        assert_eq!(
            label(SelectionActionKind::Download).as_deref(),
            Some("Download 2 releases")
        );
        assert_eq!(
            label(SelectionActionKind::AddToPlaylist).as_deref(),
            Some("Add 2 releases to playlist")
        );
        assert_eq!(
            label(SelectionActionKind::Remove).as_deref(),
            Some("Remove 1 release\u{2026}")
        );
    }

    /// R20-04: in `Normal`, no Discover row or tile view model shows a
    /// checkbox. In `Selecting`, each row with a key shows one. The row
    /// view and the tile view render these same rows.
    #[test]
    fn adr_0084_discover_selection_r20_04_checkbox_follows_the_mode() {
        let rows = vec![
            discover_row("index-feed:a", "Rain"),
            discover_row("index-feed:b", "Haleen"),
            discover_row("index-feed:", "No GUID"),
        ];
        let mut selection = DetailSelection::new();
        selection.bind(&discover_entry());
        let layout = NewReleasesLayout::default();
        assert!(item_selections(&discover_items_with_selection(
            rows.clone(),
            &layout,
            &selection
        ))
        .iter()
        .all(|row| !row.checkbox_visible));

        selection.toggle_mode();
        let shown: Vec<bool> =
            item_selections(&discover_items_with_selection(rows, &layout, &selection))
                .iter()
                .map(|row| row.checkbox_visible)
                .collect();
        assert_eq!(shown, [true, true, false]);
    }

    /// R20-05: the tile focus moves in a line for Up, Down, Left and Right,
    /// and stops at the first and the last tile.
    #[test]
    fn adr_0084_discover_selection_r20_05_tile_focus_moves_in_a_line() {
        let rows = [
            discover_row("index-feed:a", "A"),
            discover_row("index-feed:b", "B"),
            discover_row("index-feed:c", "C"),
        ];
        let rows = discover_rows(&rows, |_| None);
        let visible = rows.selection_rows();
        let key = |guid: &str| Some(DetailRowKey::Release(release_key(guid)));
        let mut selection = DetailSelection::new();
        selection.bind(&discover_entry());

        let mut step = |step: FocusStep| selection.move_focus(step, &visible);
        assert_eq!(step(TileArrow::Right.focus_step()), key("a"));
        assert_eq!(step(TileArrow::Right.focus_step()), key("b"));
        assert_eq!(step(FocusStep::Down), key("c"));
        assert_eq!(
            step(TileArrow::Right.focus_step()),
            key("c"),
            "stops at the last tile"
        );
        assert_eq!(step(FocusStep::Down), key("c"), "stops at the last tile");
        assert_eq!(step(TileArrow::Left.focus_step()), key("b"));
        assert_eq!(step(FocusStep::Up), key("a"));
        assert_eq!(
            step(TileArrow::Left.focus_step()),
            key("a"),
            "stops at the first tile"
        );
        assert_eq!(step(FocusStep::Up), key("a"), "stops at the first tile");
    }

    const PLAYLIST_ID: i64 = 25;

    /// A recorded playlist that holds "Haleen" (track 2) two times:
    /// positions 0 to 4 hold tracks 1, 2, 3, 2 and 4. Tracks 1 and 2 are
    /// downloaded.
    fn playlist_tracks() -> Vec<TrackRow> {
        vec![
            library_track(1, "t1", "Face Pale", true),
            library_track(2, "t2", "Haleen", true),
            library_track(3, "t3", "Glass", false),
            library_track(2, "t2", "Haleen", true),
            library_track(4, "t4", "Ember", false),
        ]
    }

    fn playlist_rows(tracks: &[TrackRow]) -> DetailPageRows {
        let entries: Vec<(usize, &TrackRow)> = tracks.iter().enumerate().collect();
        DetailPageRows::PlaylistEntries {
            playlist_id: PLAYLIST_ID,
            rows: playlist_entry_rows(&entries, |_| false),
        }
    }

    fn entry(position: i64, track_id: i64) -> DetailRowKey {
        DetailRowKey::PlaylistEntry(PlaylistEntryKey { position, track_id })
    }

    fn playlist_entry_page() -> FrameNavigationEntry {
        FrameNavigationEntry::PlaylistDetail(PLAYLIST_ID)
    }

    /// R25-01: a playlist page gives `PlaylistEntries`, with the noun
    /// "track".
    #[test]
    fn adr_0084_playlist_selection_r25_01_playlist_page_gives_entries() {
        assert_eq!(
            DetailListKind::for_entry(&playlist_entry_page()),
            Some(DetailListKind::PlaylistEntries)
        );
        let tracks = playlist_tracks();
        let rows = playlist_rows(&tracks);
        assert_eq!(rows.kind(), DetailListKind::PlaylistEntries);
        let header = detail_header_action(Some(&rows), SelectionMode::Normal)
            .expect("a playlist with entries has a Select button");
        assert_eq!(header.label, "Select");
        assert_eq!(header.a11y_label, "Select tracks");
    }

    /// R25-02: two entries of one track have two keys. A toggle of one does
    /// not select the other.
    #[test]
    fn adr_0084_playlist_selection_r25_02_two_entries_have_two_keys() {
        let tracks = playlist_tracks();
        let rows = playlist_rows(&tracks);
        assert_eq!(rows.members()[1], entry(1, 2));
        assert_eq!(rows.members()[3], entry(3, 2));

        let mut selection = DetailSelection::new();
        selection.bind(&playlist_entry_page());
        selection.toggle_row(&entry(3, 2));
        let checked: Vec<bool> = selection
            .row_selections(&rows)
            .iter()
            .map(|row| row.check_state.is_checked())
            .collect();
        assert_eq!(checked, [false, false, false, true, false]);
        assert_eq!(selection.selected_playlist_entries().len(), 1);
    }

    /// R25-03: after a reload, `retain_visible` drops a key whose position
    /// now holds a different track.
    #[test]
    fn adr_0084_playlist_selection_r25_03_retain_drops_a_changed_position() {
        let tracks = playlist_tracks();
        let mut selection = DetailSelection::new();
        selection.bind(&playlist_entry_page());
        selection.toggle_row(&entry(1, 2));
        selection.toggle_row(&entry(4, 4));

        // Another action removed position 1. Position 1 now holds track 3.
        let mut reloaded = tracks.clone();
        reloaded.remove(1);
        selection.retain_visible(&playlist_rows(&reloaded).members());
        assert_eq!(
            selection.selected_playlist_entries(),
            Vec::<PlaylistEntryKey>::new()
        );

        selection.toggle_row(&entry(0, 1));
        selection.retain_visible(&playlist_rows(&tracks).members());
        assert_eq!(selection.selected_count(), 1);
    }

    fn bar_labels(rows: &DetailPageRows, selection: &DetailSelection) -> Vec<String> {
        rows.action_bar(selection)
            .expect("Selecting shows the bar")
            .actions
            .into_iter()
            .map(|action| action.label)
            .collect()
    }

    /// R25-04: the bar gives the six actions and "Clear" in the order of
    /// the packet table. "Download" and "Remove from Library…" count
    /// distinct tracks.
    #[test]
    fn adr_0084_playlist_selection_r25_04_bar_order_and_distinct_counts() {
        let tracks = playlist_tracks();
        let rows = playlist_rows(&tracks);
        let mut selection = DetailSelection::new();
        selection.bind(&playlist_entry_page());
        for key in [entry(1, 2), entry(2, 3), entry(3, 2)] {
            selection.toggle_row(&key);
        }

        assert_eq!(
            bar_labels(&rows, &selection),
            [
                "Remove 3 from playlist",
                "Download 1 track",
                "Add 3 to playlist",
                "Move to top",
                "Move to bottom",
                "Remove from Library\u{2026}",
                "Clear",
            ]
        );
        let bar = rows
            .action_bar(&selection)
            .expect("Selecting shows the bar");
        let remove = bar
            .action(SelectionActionKind::Remove)
            .expect("track 2 is downloaded");
        assert_eq!(remove.a11y_label, "Remove 1 track from Library\u{2026}");
        assert_eq!(bar.count_label, "3 tracks selected");
    }

    /// R25-05: "Move to top" is absent when the selection is already at
    /// the top. "Move to bottom" is absent when it is already at the
    /// bottom.
    #[test]
    fn adr_0084_playlist_selection_r25_05_move_absent_at_its_edge() {
        let tracks = playlist_tracks();
        let rows = playlist_rows(&tracks);
        let mut selection = DetailSelection::new();
        selection.bind(&playlist_entry_page());
        selection.toggle_row(&entry(1, 2));
        selection.toggle_row(&entry(0, 1));
        let labels = bar_labels(&rows, &selection);
        assert!(!labels.iter().any(|label| label == "Move to top"));
        assert!(labels.iter().any(|label| label == "Move to bottom"));

        selection.clear_rows();
        selection.toggle_row(&entry(4, 4));
        selection.toggle_row(&entry(3, 2));
        let labels = bar_labels(&rows, &selection);
        assert!(labels.iter().any(|label| label == "Move to top"));
        assert!(!labels.iter().any(|label| label == "Move to bottom"));

        selection.select_all(&rows.selection_rows());
        let labels = bar_labels(&rows, &selection);
        assert!(!labels.iter().any(|label| label.starts_with("Move")));
    }

    /// R25-08: `rekey_after_move` gives the new keys and the new focus.
    /// The order a, b, c, b, d with positions 1 and 3 selected gives
    /// b, b, a, c, d after "Move to top".
    #[test]
    fn adr_0084_playlist_selection_r25_08_rekey_follows_the_moved_entries() {
        let mut selection = DetailSelection::new();
        selection.bind(&playlist_entry_page());
        selection.toggle_row(&entry(1, 2));
        selection.toggle_row(&entry(3, 2));
        let visible = playlist_rows(&playlist_tracks()).selection_rows();
        // The focus is on the unselected entry at position 2 (track 3).
        selection.move_focus(FocusStep::Up, &visible);

        selection.rekey_after_move(PlaylistEdge::Top, 5);
        assert_eq!(
            selection.selected_playlist_entries(),
            [
                PlaylistEntryKey {
                    position: 0,
                    track_id: 2
                },
                PlaylistEntryKey {
                    position: 1,
                    track_id: 2
                },
            ]
        );
        assert_eq!(selection.focused(), Some(&entry(3, 3)));

        // From b, b, a, c, d: the focus moves to the selected entry at
        // position 1, then "Move to bottom" gives a, c, d, b, b.
        selection.toggle_row(&entry(1, 2));
        selection.toggle_row(&entry(1, 2));
        selection.rekey_after_move(PlaylistEdge::Bottom, 5);
        assert_eq!(
            selection.selected_playlist_entries(),
            [
                PlaylistEntryKey {
                    position: 3,
                    track_id: 2
                },
                PlaylistEntryKey {
                    position: 4,
                    track_id: 2
                },
            ]
        );
        assert_eq!(selection.focused(), Some(&entry(4, 2)));
    }

    /// R25-10: the "Add to playlist" choices of a playlist page leave out
    /// the open playlist. Each other page gives each playlist.
    #[test]
    fn adr_0084_playlist_selection_r25_10_choices_leave_out_the_open_playlist() {
        let playlist = |id: i64, name: &str| Playlist {
            id,
            name: name.to_owned(),
            description: None,
            track_count: 0,
            created_at: 0,
            updated_at: 0,
        };
        let playlists = [playlist(PLAYLIST_ID, "Open"), playlist(9, "Other")];
        let tracks = playlist_tracks();
        let ids = |choices: Vec<Playlist>| -> Vec<i64> {
            choices.into_iter().map(|playlist| playlist.id).collect()
        };
        assert_eq!(
            ids(playlist_rows(&tracks).add_to_playlist_choices(&playlists)),
            [9]
        );
        let release = DetailPageRows::Tracks(Vec::new());
        assert_eq!(
            ids(release.add_to_playlist_choices(&playlists)),
            [PLAYLIST_ID, 9]
        );
    }

    /// R17-16 (ADR 0084 Decision 7, packet 017): the name level page has
    /// the release selection, and its detail page rows give one row for
    /// each release with a release key.
    #[test]
    fn adr_0084_name_level_r17_16_name_level_page_has_release_rows() {
        use crate::view_models::artist_identity::ArtistNameKey;
        use crate::view_models::name_level_page::{
            NameLevelIndexRelease, NameLevelIndexState, NameLevelLibraryRelease,
            NameLevelPageFacts, NameLevelPageVm,
        };
        use crate::view_models::release_kind::ReleaseKind;

        let entry = FrameNavigationEntry::ArtistName(ArtistNameKey::from_text("Haleen"));
        assert_eq!(
            DetailListKind::for_entry(&entry),
            Some(DetailListKind::Releases)
        );

        let vm = NameLevelPageVm::new(NameLevelPageFacts {
            key: ArtistNameKey::from_text("Haleen"),
            library: vec![
                NameLevelLibraryRelease {
                    release_key: Some(ReleaseKey::LibraryFeed(4)),
                    feed_guid: None,
                    title: "No GUID".to_owned(),
                    image_url: None,
                    artist_text: "Haleen".to_owned(),
                    kind: ReleaseKind::Album,
                    newest_date: Some(2),
                    in_library: true,
                },
                NameLevelLibraryRelease {
                    release_key: None,
                    feed_guid: None,
                    title: "No key".to_owned(),
                    image_url: None,
                    artist_text: "Haleen".to_owned(),
                    kind: ReleaseKind::Single,
                    newest_date: Some(1),
                    in_library: true,
                },
            ],
            index: NameLevelIndexState::Loaded {
                releases: vec![NameLevelIndexRelease {
                    feed_guid: "index-guid".to_owned(),
                    title: Some("Index".to_owned()),
                    episode_count: Some(1),
                    ..NameLevelIndexRelease::default()
                }],
                more_available: false,
            },
            other_identity_feed_guids: std::collections::BTreeSet::new(),
            matched_feed: None,
        });
        let rows = feed_page_release_rows(&vm.sections(), |_| None);
        let keys: Vec<ReleaseKey> = rows.into_iter().map(|row| row.key).collect();
        assert_eq!(
            keys,
            [
                ReleaseKey::LibraryFeed(4),
                ReleaseKey::from_index_feed_guid("index-guid")
            ],
            "a release with no release key gives no selection row"
        );
    }
}
