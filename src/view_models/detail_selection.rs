//! The selection of the Music detail column (ADR 0084 Decision 10, packet
//! 018).
//!
//! Four detail pages have a list that the curator can select: the track
//! rows of a release page, the rows of a singles group page, the release
//! rows of a feed page and the Discover "New releases" view (packet 020).
//! The detail column has one selection and
//! one mode. They belong to one page entry. A move to another page clears
//! the selection and sets [`SelectionMode::Normal`].
//!
//! This module is pure and carries no renderer type. It uses the model of
//! `list_selection.rs` and the bar of `selection_action_bar.rs`. It does
//! not copy them.

#![warn(clippy::pedantic)]

use crate::db::TrackRow;
use crate::view_models::library::ContentListRowDisplay;
use crate::view_models::list_selection::{
    FocusStep, ListSelection, RowSelectionVm, SelectionMode, SelectionRow,
};
use crate::view_models::publisher_page::{PublisherPageSectionVm, TitleDisplay};
use crate::view_models::selection_action_bar::{SelectionActionBarVm, SelectionActionCounts};
use crate::view_models::singles_group::SinglesGroupPageRowVm;
use crate::view_models::workspace::{
    DiscoverView, FrameHeaderActionDisplay, FrameNavigationEntry, ReleaseKey, TrackKey,
};
use crate::views::{TrackRef, TrackView};

/// The key of one detail row. A release page row is a track. A singles
/// group page row and a feed page row are a release.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum DetailRowKey {
    /// A track row of a release page.
    Track(TrackKey),
    /// A release row of a singles group page or a feed page.
    Release(ReleaseKey),
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
}

impl DetailListKind {
    /// The list kind of the page `entry`. A release page gives tracks. A
    /// singles group page, a feed page and the Discover "New releases"
    /// view give releases. Each other page gives `None`: it has no
    /// selection. Packet 025 owns the playlist page.
    #[must_use]
    pub(crate) const fn for_entry(entry: &FrameNavigationEntry) -> Option<Self> {
        match entry {
            FrameNavigationEntry::Release(_) => Some(Self::Tracks),
            FrameNavigationEntry::SinglesGroup(_)
            | FrameNavigationEntry::PublisherDetail(_)
            | FrameNavigationEntry::DiscoverView(DiscoverView::NewReleases) => Some(Self::Releases),
            _ => None,
        }
    }

    const fn plural_noun(self) -> &'static str {
        match self {
            Self::Tracks => "tracks",
            Self::Releases => "releases",
        }
    }

    const fn item_noun(self) -> &'static str {
        match self {
            Self::Tracks => "track",
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

/// The Library facts of one release.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LibraryReleaseFacts {
    /// The Library feed id.
    pub(crate) feed_id: i64,
    /// Each track of the release is downloaded.
    pub(crate) fully_downloaded: bool,
    /// A download job of the release key is queued or in flight. "Remove"
    /// is then absent (orchestrator correction of 2026-10-20).
    pub(crate) download_active: bool,
}

impl LibraryReleaseFacts {
    /// Tells if "Remove" can act on the release.
    #[must_use]
    pub(crate) const fn removable(self) -> bool {
        self.fully_downloaded && !self.download_active
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

/// The selectable rows of the open detail page, in their visible order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum DetailPageRows {
    /// The track rows of a release page.
    Tracks(Vec<DetailTrackRow>),
    /// The release rows of a singles group page or a feed page.
    Releases(Vec<DetailReleaseRow>),
}

impl DetailPageRows {
    /// The list kind of these rows.
    #[must_use]
    pub(crate) const fn kind(&self) -> DetailListKind {
        match self {
            Self::Tracks(_) => DetailListKind::Tracks,
            Self::Releases(_) => DetailListKind::Releases,
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
            Self::Releases(rows) => rows
                .iter()
                .map(|row| DetailRowKey::Release(row.key.clone()))
                .collect(),
        }
    }

    /// The rows for [`ListSelection`]. Each row has one member: its own
    /// key. A row with no key is not in this list.
    #[must_use]
    pub(crate) fn selection_rows(&self) -> Vec<SelectionRow<DetailRowKey, DetailRowKey>> {
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
            (Self::Releases(rows), DetailRowKey::Release(key)) => rows
                .iter()
                .find(|row| &row.key == key)
                .map(|row| row.title.as_str()),
            _ => None,
        }
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
            Self::Releases(_) => Vec::new(),
        }
    }

    /// The selected release rows, in their visible order.
    #[must_use]
    pub(crate) fn selected_releases(&self, selection: &DetailSelection) -> Vec<&DetailReleaseRow> {
        match self {
            Self::Releases(rows) => rows
                .iter()
                .filter(|row| {
                    selection
                        .selection
                        .is_selected(&DetailRowKey::Release(row.key.clone()))
                })
                .collect(),
            Self::Tracks(_) => Vec::new(),
        }
    }

    /// The action bar of the detail column, or `None` outside
    /// [`SelectionMode::Selecting`].
    ///
    /// Track rows: "Download" counts the tracks that are not downloaded,
    /// "Add to playlist" counts the Library tracks, and "Remove" counts
    /// the downloaded tracks. Release rows: "Download" counts an Index
    /// release and a Library release that is not fully downloaded. "Add
    /// to playlist" counts each Library release. "Remove" counts each
    /// Library release that is fully downloaded.
    #[must_use]
    pub(crate) fn action_bar(&self, selection: &DetailSelection) -> Option<SelectionActionBarVm> {
        let counts = match self {
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
            Self::Releases(_) => {
                let selected = self.selected_releases(selection);
                SelectionActionCounts {
                    selected_count: selected.len(),
                    item_noun: DetailListKind::Releases.item_noun(),
                    download_count: selected
                        .iter()
                        .filter(|row| row.library.is_none_or(|facts| !facts.fully_downloaded))
                        .count(),
                    add_count: selected.iter().filter(|row| row.library.is_some()).count(),
                    remove_count: selected
                        .iter()
                        .filter(|row| row.library.is_some_and(LibraryReleaseFacts::removable))
                        .count(),
                }
            }
        };
        SelectionActionBarVm::from_counts(selection.mode, counts)
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
            DetailPageRows::Releases(rows) => rows
                .iter()
                .map(|row| {
                    self.row_selection(Some(&DetailRowKey::Release(row.key.clone())), &row.title)
                })
                .collect(),
        }
    }

    /// A click on the body of the row `key`. Ctrl+click and Shift+click
    /// start [`SelectionMode::Selecting`] and change the selection. In
    /// `Selecting`, a plain click changes the selection of the row. Gives
    /// `true` when the caller must open the page of the row: a plain
    /// click in [`SelectionMode::Normal`].
    pub(crate) fn click(
        &mut self,
        key: &DetailRowKey,
        click: DetailRowClick,
        visible: &[SelectionRow<DetailRowKey, DetailRowKey>],
    ) -> bool {
        let row = SelectionRow::of_one(key.clone());
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
    let rows = rows.filter(|rows| rows.has_selectable_rows())?;
    let noun = rows.kind().plural_noun();
    let element_id = format!("music-detail-column-select-{noun}");
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

/// The rows of a singles group page. `facts` gives the Library facts of a
/// release key.
#[must_use]
pub(crate) fn singles_group_release_rows(
    rows: &[SinglesGroupPageRowVm],
    facts: impl Fn(&ReleaseKey) -> Option<LibraryReleaseFacts>,
) -> Vec<DetailReleaseRow> {
    rows.iter()
        .map(|row| DetailReleaseRow {
            key: row.key.clone(),
            title: track_row_title(row.track.track_title.as_deref(), Some(&row.track.item_guid)),
            library: facts(&row.key),
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

/// The key of a Discover row or tile (packet 020), from its `index-feed:`
/// row id. A row with no feed GUID has no key, so it has no checkbox and
/// no focus stop.
#[must_use]
pub(crate) fn discover_row_key(row: &ContentListRowDisplay) -> Option<DetailRowKey> {
    row.index_feed_guid()
        .map(|guid| DetailRowKey::Release(ReleaseKey::FeedGuid(guid.to_owned())))
}

/// The rows of the Discover "New releases" view (packet 020), in their
/// visible order. A row with no key is not in this list. `facts` gives the
/// Library facts of a release key, so the bar counts follow packet 018.
#[must_use]
pub(crate) fn discover_release_rows<'a>(
    rows: impl IntoIterator<Item = &'a ContentListRowDisplay>,
    facts: impl Fn(&ReleaseKey) -> Option<LibraryReleaseFacts>,
) -> Vec<DetailReleaseRow> {
    rows.into_iter()
        .filter_map(|row| match discover_row_key(row)? {
            DetailRowKey::Release(key) => Some(DetailReleaseRow {
                title: row.title().to_owned(),
                library: facts(&key),
                key,
            }),
            DetailRowKey::Track(_) => None,
        })
        .collect()
}

/// The Discover rows with the selection facts of the detail column
/// (packet 020). The row view and the tile view use these same rows. A row
/// with no key gets [`RowSelectionVm::none`].
#[must_use]
pub(crate) fn discover_rows_with_selection(
    rows: Vec<ContentListRowDisplay>,
    selection: &DetailSelection,
) -> Vec<ContentListRowDisplay> {
    rows.into_iter()
        .map(|mut row| {
            row.selection = selection.row_selection(discover_row_key(&row).as_ref(), row.title());
            row
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

    fn library_facts(feed_id: i64, fully_downloaded: bool) -> Option<LibraryReleaseFacts> {
        Some(LibraryReleaseFacts {
            feed_id,
            fully_downloaded,
            download_active: false,
        })
    }

    fn feed_row(guid: &str, title: &str) -> PublisherPageAlbumVm {
        PublisherPageAlbumVm {
            feed_guid: Some(guid.to_owned()),
            title: TitleDisplay::Stated(title.to_owned()),
            image_url: None,
            artist: None,
            not_listed: false,
            in_library: false,
            role_line: None,
            open: Some(ReleaseRowOpenAction {
                key: release_key(guid),
                a11y_label: format!("Open the release {title}"),
            }),
            selection: RowSelectionVm::none(),
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

        let group_rows = vec![SinglesGroupPageRowVm {
            track: library_track(5, "s1", "Single", true),
            date_text: None,
            work: crate::view_models::library::TrackRowWork::Idle,
            key: release_key("single-1"),
        }];
        let group = DetailPageRows::Releases(singles_group_release_rows(&group_rows, |_| None));
        let group_action = detail_header_action(Some(&group), SelectionMode::Normal)
            .expect("a group page has the action");
        assert_eq!(group_action.a11y_label, "Select releases");

        let feed = DetailPageRows::Releases(feed_page_release_rows(
            &[PublisherPageSectionVm {
                heading: "Singles".to_owned(),
                rows: vec![feed_row("r1", "Haleen")],
            }],
            |_| None,
        ));
        let feed_action = detail_header_action(Some(&feed), SelectionMode::Selecting)
            .expect("a feed page has the action");
        assert_eq!(feed_action.a11y_label, "Done selecting releases");

        for entry in [
            release_entry(),
            FrameNavigationEntry::SinglesGroup(SinglesGroupKey(ArtistIdentity::Name(
                "Haleen".to_owned(),
            ))),
            FrameNavigationEntry::PublisherDetail("feed-artist".to_owned()),
        ] {
            assert!(DetailListKind::for_entry(&entry).is_some());
        }
        let track_page =
            FrameNavigationEntry::Track(TrackKey::from_index_track_target(Some(FEED_GUID), "t1"));
        assert_eq!(DetailListKind::for_entry(&track_page), None);
        assert_eq!(
            DetailListKind::for_entry(&FrameNavigationEntry::PlaylistDetail(3)),
            None
        );
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
        }];
        let rows = DetailPageRows::Releases(feed_page_release_rows(&sections, |key| {
            (key == &release_key("library"))
                .then_some(library_facts(9, true))
                .flatten()
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
    fn discover_row(id: &str, title: &str) -> ContentListRowDisplay {
        ContentListRowDisplay::new(
            id,
            title,
            "Haleen",
            crate::view_models::library::ContentListRowSource::Index,
        )
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
        let releases = discover_release_rows(&rows, |_| None);
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].title, "Face Pale");

        let mut selection = DetailSelection::new();
        selection.bind(&discover_entry());
        selection.toggle_mode();
        let rows = discover_rows_with_selection(rows, &selection);
        assert!(rows[0].selection.checkbox_visible);
        assert!(!rows[1].selection.checkbox_visible);
        assert!(!rows[1].selection.focused);
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
        let rows = DetailPageRows::Releases(discover_release_rows(&rows, |key| {
            if key == &release_key("downloaded") {
                library_facts(9, true)
            } else if key == &release_key("partial") {
                library_facts(10, false)
            } else {
                None
            }
        }));
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
        assert!(discover_rows_with_selection(rows.clone(), &selection)
            .iter()
            .all(|row| !row.selection.checkbox_visible));

        selection.toggle_mode();
        let shown: Vec<bool> = discover_rows_with_selection(rows, &selection)
            .iter()
            .map(|row| row.selection.checkbox_visible)
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
        let rows = DetailPageRows::Releases(discover_release_rows(&rows, |_| None));
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
}
