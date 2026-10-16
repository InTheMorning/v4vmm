//! Workspace frame navigation state.
//!
//! This module owns the typed location and history model for one workspace
//! frame (ADR 0084 Decision 9). The parent `workspace` module re-exports the public(crate) types so
//! callers keep the same import path after decomposition.

#![warn(clippy::pedantic)]

use super::WorkspaceModelError;

/// A Music destination, in sidebar order (ADR 0084 Decision 2).
///
/// Each destination is a list root: the sidebar selects one, and the list
/// column fills with its items. A destination opens no page by itself.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MusicDestination {
    Discover,
    Releases,
    Artists,
    Tracks,
    NeedsAttention,
    Playlists,
}

impl MusicDestination {
    /// Every destination, in sidebar order (ADR 0084 Decision 2).
    pub(crate) const ALL: [Self; 6] = [
        Self::Discover,
        Self::Releases,
        Self::Artists,
        Self::Tracks,
        Self::NeedsAttention,
        Self::Playlists,
    ];

    /// The visible destination name.
    #[must_use]
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Discover => "Discover",
            Self::Releases => "Releases",
            Self::Artists => "Artists",
            Self::Tracks => "Tracks",
            Self::NeedsAttention => "Needs attention",
            Self::Playlists => "Playlists",
        }
    }

    /// The page this destination opens under it, the first time its root
    /// becomes the current location (ADR 0084 Decision 3). A sidebar
    /// click and the Music opening path both use this. Only Discover
    /// opens one: "New releases".
    #[must_use]
    pub(crate) const fn default_child_page(self) -> Option<FrameNavigationEntry> {
        match self {
            Self::Discover => Some(FrameNavigationEntry::DiscoverView(
                DiscoverView::NewReleases,
            )),
            Self::Releases
            | Self::Artists
            | Self::Tracks
            | Self::NeedsAttention
            | Self::Playlists => None,
        }
    }
}

/// A view of the Discover list column (ADR 0084 Decision 3). Discover is
/// the only destination with views: "New releases" is the first one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DiscoverView {
    NewReleases,
}

impl DiscoverView {
    /// Every Discover view, in list order (ADR 0084 Decision 3).
    pub(crate) const ALL: [Self; 1] = [Self::NewReleases];

    /// The visible view name.
    #[must_use]
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::NewReleases => "New releases",
        }
    }

    /// The view's list-row subtitle (ADR 0084 Decision 3).
    #[must_use]
    pub(crate) const fn subtitle(self) -> &'static str {
        match self {
            Self::NewReleases => "Newest first, from the Index",
        }
    }
}

/// Navigation target stored in a frame's history.
///
/// Entries are typed so frame history can distinguish entity detail,
/// workspace source, queue, and search destinations without parsing strings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FrameNavigationEntry {
    /// A Music destination, the root of a sidebar-started location (ADR
    /// 0084 Decisions 1 and 2). It replaces `SourceList` and
    /// `ReadinessIssues`.
    Destination(MusicDestination),
    /// A view of the Discover column, a page under the Discover
    /// destination (ADR 0084 Decision 3).
    DiscoverView(DiscoverView),
    /// Playlist detail by playlist id.
    PlaylistDetail(i64),
    /// A release, keyed by its identity, whatever its origin (ADR 0084
    /// Decision 4). The origin selects only the data source of the page,
    /// held outside this entry.
    Release(ReleaseKey),
    /// A track, keyed by its identity, whatever its origin (ADR 0084
    /// Decision 4). The origin selects only the data source of the page,
    /// held outside this entry.
    Track(TrackKey),
    /// Artist detail by display name.
    ArtistDetail(String),
    /// Publisher page by publisher feed GUID (ADR 0077 Decision 1). This
    /// entry never keys on name text or on `publisher_text`.
    PublisherDetail(String),
    /// Search results by submitted query.
    Search(String),
    /// Tracks `MusicIndex` gives for one name query (ADR 0077 packet 006,
    /// Accepted Refinement "Index artist page by name"). This entry carries
    /// no artist identity, no role, and no page type: it is a search
    /// result, never an artist page.
    IndexNameMatches(String),
    /// Application settings.
    Settings,
    /// Queue and Now Playing frame root.
    QueueNowPlaying,
}

impl FrameNavigationEntry {
    /// Returns whether this entry opens a page in the Music detail column
    /// (ADR 0084 Decisions 1 and 2). A list root (a destination, a search)
    /// fills the list column and opens no page.
    #[must_use]
    pub(crate) const fn opens_detail_page(&self) -> bool {
        !matches!(
            self,
            Self::Destination(_) | Self::Search(_) | Self::Settings
        )
    }

    /// Returns the default visible label for this navigation entry.
    #[must_use]
    pub(crate) fn display_label(&self) -> String {
        match self {
            Self::Destination(destination) => destination.label().to_string(),
            Self::DiscoverView(view) => view.label().to_string(),
            Self::PlaylistDetail(id) => format!("Playlist {id}"),
            // The top app gives the real title from the resolved data
            // source. This fallback shows only before that title is known.
            Self::Release(_) => "Release".to_string(),
            // The top app gives the real title from the resolved data
            // source. This fallback shows only before that title is known.
            Self::Track(_) => "Track".to_string(),
            Self::ArtistDetail(name) => name.clone(),
            // ADR 0077 packet 006: the breadcrumb shows the quoted name.
            // This entry names no artist, so its label never looks like one.
            Self::IndexNameMatches(name) => format!("\"{name}\""),
            // The publisher page view model gives the real title. This
            // fallback shows only when no caller supplies that title.
            Self::PublisherDetail(publisher_feed_guid) => publisher_feed_guid.clone(),
            Self::Search(query) => {
                if query.trim().is_empty() {
                    "Search".to_string()
                } else {
                    format!("Search: {query}")
                }
            }
            Self::Settings => "Settings".to_string(),
            Self::QueueNowPlaying => "Queue".to_string(),
        }
    }
}

/// Where a page change puts its page (ADR 0084 Decision 9).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NavigationPathStart {
    /// A sidebar click: a new path starts at the destination.
    NewPath,
    /// A row or a link: the page goes into the current path.
    CurrentPath,
}

/// The identity of one release, whatever its origin (ADR 0084 Decision 4).
///
/// A release opened from the Library and the same release opened from the
/// Index build this same key, so they push the same navigation entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ReleaseKey {
    /// The feed GUID. Each release with a feed GUID uses it, whatever its
    /// origin.
    FeedGuid(String),
    /// A Library feed with no feed GUID. No Index release can match it.
    LibraryFeed(i64),
}

impl ReleaseKey {
    /// The release key of an Index row, keyed by its feed GUID.
    #[must_use]
    pub(crate) fn from_index_feed_guid(feed_guid: impl Into<String>) -> Self {
        Self::FeedGuid(feed_guid.into())
    }
}

/// The data source that fills an open release page (ADR 0084 Decision 4).
///
/// The origin of a release selects this source, never its route or its
/// layout. The top app looks this up once and keeps it; the render path
/// reads the stored value and makes no lookup of its own.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ReleasePageSource {
    /// The release is a Library feed.
    Library {
        /// The Library feed id.
        feed_id: i64,
    },
    /// The release is known to the Index only.
    Index {
        /// The feed GUID.
        feed_guid: String,
    },
}

/// Resolves the data source of a release (ADR 0084 Decision 4).
///
/// `library_feed_id` is the Library feed id a caller already looked up for
/// a [`ReleaseKey::FeedGuid`] key, through `db::find_feed_id_by_guid`. A
/// [`ReleaseKey::LibraryFeed`] key needs no lookup: it names its feed id
/// directly, so the Library source always carries it. A [`ReleaseKey::FeedGuid`]
/// key resolves to the Library source when the lookup finds a feed, else to
/// the Index source.
#[must_use]
pub(crate) fn resolve_release_page_source(
    key: ReleaseKey,
    library_feed_id: Option<i64>,
) -> ReleasePageSource {
    match key {
        ReleaseKey::LibraryFeed(feed_id) => ReleasePageSource::Library { feed_id },
        ReleaseKey::FeedGuid(feed_guid) => match library_feed_id {
            Some(feed_id) => ReleasePageSource::Library { feed_id },
            None => ReleasePageSource::Index { feed_guid },
        },
    }
}

/// Returns whether a completed download of the release `downloaded_guid`
/// changes the open page (ADR 0084 Decision 4).
///
/// Only the open release page of that feed GUID changes its data source in
/// place. A download that completes after the operator opened another page
/// changes no page.
#[must_use]
pub(crate) fn release_download_updates_open_page(
    current: Option<&FrameNavigationEntry>,
    downloaded_guid: Option<&str>,
) -> bool {
    match (current, downloaded_guid) {
        (Some(FrameNavigationEntry::Release(ReleaseKey::FeedGuid(open_guid))), Some(guid)) => {
            open_guid == guid
        }
        _ => false,
    }
}

/// The identity of one track, whatever its origin (ADR 0084 Decision 4).
///
/// A track opened from the Library and the same track opened from the
/// Index build this same key, so they push the same navigation entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TrackKey {
    /// The release of the track. `None` only for an Index result with no
    /// feed GUID.
    pub(crate) release: Option<ReleaseKey>,
    /// The RSS item GUID.
    pub(crate) item_guid: String,
}

impl TrackKey {
    /// The track key of an Index track target (ADR 0084 Decision 4). A
    /// target with a feed GUID carries that release, whatever its origin.
    /// A target with no feed GUID carries no release, so it always
    /// resolves to the Index.
    #[must_use]
    pub(crate) fn from_index_track_target(
        feed_guid: Option<&str>,
        item_guid: impl Into<String>,
    ) -> Self {
        Self {
            release: feed_guid.map(ReleaseKey::from_index_feed_guid),
            item_guid: item_guid.into(),
        }
    }

    /// The Index activation target of this key: `<feed_guid>:<item_guid>`
    /// when the key has a feed GUID, or the bare item GUID otherwise.
    #[must_use]
    pub(crate) fn index_target(&self) -> String {
        match &self.release {
            Some(ReleaseKey::FeedGuid(feed_guid)) => format!("{feed_guid}:{}", self.item_guid),
            Some(ReleaseKey::LibraryFeed(_)) | None => self.item_guid.clone(),
        }
    }
}

/// The data source that fills an open track page (ADR 0084 Decision 4).
///
/// The origin of a track selects this source, never its route or its
/// layout. The top app looks this up once and keeps it; the render path
/// reads the stored value and makes no lookup of its own.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum TrackPageSource {
    /// The track is a Library track.
    Library {
        /// The Library track id.
        track_id: i64,
    },
    /// The track is known to the Index only.
    Index {
        /// The Index activation target.
        target: String,
    },
}

/// Resolves the data source of a track (ADR 0084 Decision 4).
///
/// `library_track_id` is the Library track id a caller already looked up
/// for a key with a release, through the two read queries of Required
/// Change 3. A key with no release needs no lookup: it always resolves to
/// the Index, because the Library has no track with an unknown release.
#[must_use]
pub(crate) fn resolve_track_page_source(
    key: &TrackKey,
    library_track_id: Option<i64>,
) -> TrackPageSource {
    if key.release.is_none() {
        return TrackPageSource::Index {
            target: key.item_guid.clone(),
        };
    }
    match library_track_id {
        Some(track_id) => TrackPageSource::Library { track_id },
        None => TrackPageSource::Index {
            target: key.index_target(),
        },
    }
}

/// Returns whether a completed single-track download changes the open page
/// (ADR 0084 Decision 4).
///
/// Only the open track page of that exact track changes its data source in
/// place. A download that completes after the operator opened another page
/// changes no page.
#[must_use]
pub(crate) fn track_download_updates_open_page(
    current: Option<&FrameNavigationEntry>,
    downloaded_key: Option<&TrackKey>,
) -> bool {
    match (current, downloaded_key) {
        (Some(FrameNavigationEntry::Track(open_key)), Some(key)) => open_key == key,
        _ => false,
    }
}

/// Returns whether a completed album download changes the open track page
/// (ADR 0084 Decision 4).
///
/// "Download album" on a track page downloads every track of the release,
/// not the one open track by its own identity. Only the open track page of
/// that release changes its data source in place.
#[must_use]
pub(crate) fn track_release_download_updates_open_page(
    current: Option<&FrameNavigationEntry>,
    downloaded_guid: Option<&str>,
) -> bool {
    match (current, downloaded_guid) {
        (Some(FrameNavigationEntry::Track(key)), Some(guid)) => {
            matches!(&key.release, Some(ReleaseKey::FeedGuid(open_guid)) if open_guid == guid)
        }
        _ => false,
    }
}

/// One location of a frame: the destination, then the pages inside it, and
/// the current page last (ADR 0084 Decision 9).
///
/// A location holds each page one time at most. The breadcrumb projects a
/// location, never the history.
#[derive(Clone, Debug, Eq, PartialEq)]
struct FrameLocation {
    ancestors: Vec<FrameNavigationEntry>,
    page: FrameNavigationEntry,
}

impl FrameLocation {
    const fn new(page: FrameNavigationEntry) -> Self {
        Self {
            ancestors: Vec::new(),
            page,
        }
    }

    fn entries(&self) -> impl Iterator<Item = &FrameNavigationEntry> {
        self.ancestors.iter().chain(std::iter::once(&self.page))
    }

    /// Returns this location with `entry` as its page. When the path holds
    /// `entry`, the path stops at it. Otherwise `entry` goes after the page.
    fn with_page(&self, entry: FrameNavigationEntry) -> Self {
        if let Some(index) = self.ancestors.iter().position(|item| *item == entry) {
            let mut ancestors = self.ancestors.clone();
            ancestors.truncate(index);
            return Self {
                ancestors,
                page: entry,
            };
        }
        let mut ancestors = self.ancestors.clone();
        ancestors.push(self.page.clone());
        Self {
            ancestors,
            page: entry,
        }
    }
}

/// Per-frame back/forward navigation history (ADR 0084 Decision 9).
///
/// The history holds locations. Back and Forward move through the history.
/// The breadcrumb shows only the current location, so it never repeats a
/// page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FrameNavigationState {
    back_stack: Vec<FrameLocation>,
    current: FrameLocation,
    forward_stack: Vec<FrameLocation>,
}

impl FrameNavigationState {
    /// Creates navigation history at an initial entry.
    #[must_use]
    pub(crate) const fn new(current: FrameNavigationEntry) -> Self {
        Self {
            back_stack: Vec::new(),
            current: FrameLocation::new(current),
            forward_stack: Vec::new(),
        }
    }

    /// Creates navigation history at an initial location path:
    /// `destination` then `page`, with no back or forward history (ADR
    /// 0084 Decision 3: Music opens on Discover, then "New releases").
    #[must_use]
    pub(crate) fn new_with_page(
        destination: FrameNavigationEntry,
        page: FrameNavigationEntry,
    ) -> Self {
        Self {
            back_stack: Vec::new(),
            current: FrameLocation::new(destination).with_page(page),
            forward_stack: Vec::new(),
        }
    }

    /// Returns the current page.
    #[must_use]
    pub(crate) const fn current(&self) -> &FrameNavigationEntry {
        &self.current.page
    }

    /// Returns the search query of the current location, when the location
    /// starts from a search.
    ///
    /// Search-result drill-down pages stay in the search location, so the
    /// owning app can keep the inspector VM alive while breadcrumbs navigate
    /// below a submitted query.
    #[must_use]
    pub(crate) fn active_search_query(&self) -> Option<&str> {
        self.current.entries().find_map(|entry| match entry {
            FrameNavigationEntry::Search(query) => Some(query.as_str()),
            _ => None,
        })
    }

    /// Returns the current location path, from its destination through the
    /// current page.
    #[must_use]
    pub(crate) fn path_entries(&self) -> Vec<FrameNavigationEntry> {
        self.current.entries().cloned().collect()
    }

    /// Returns the root entry of the current location (ADR 0084 Decision 2):
    /// the destination or the search that the current path started from.
    /// A sidebar click or a search replaces this; a row or a link never
    /// does.
    #[must_use]
    pub(crate) fn root(&self) -> &FrameNavigationEntry {
        self.current.ancestors.first().unwrap_or(&self.current.page)
    }

    /// Returns whether the current location has more than one page.
    #[must_use]
    pub(crate) fn has_location_path(&self) -> bool {
        !self.current.ancestors.is_empty()
    }

    /// Returns whether a back-history entry is available.
    #[must_use]
    pub(crate) fn can_go_back(&self) -> bool {
        !self.back_stack.is_empty()
    }

    /// Returns whether a forward-history entry is available.
    #[must_use]
    pub(crate) fn can_go_forward(&self) -> bool {
        !self.forward_stack.is_empty()
    }

    fn move_to(&mut self, location: FrameLocation) {
        if self.current == location {
            return;
        }
        let previous = std::mem::replace(&mut self.current, location);
        self.back_stack.push(previous);
        self.forward_stack.clear();
    }

    /// Opens a page in the current location and clears forward history.
    ///
    /// When the location path holds `entry`, the path stops at that page.
    /// Otherwise the page goes after the current page. Each change is one
    /// history entry.
    pub(crate) fn push(&mut self, entry: FrameNavigationEntry) {
        if self.current.page == entry {
            return;
        }
        let location = self.current.with_page(entry);
        self.move_to(location);
    }

    /// Starts a new location path at `destination`, with `page` inside it
    /// when given. A sidebar click and a search start a new path.
    pub(crate) fn start_path(
        &mut self,
        destination: FrameNavigationEntry,
        page: Option<FrameNavigationEntry>,
    ) {
        let root = FrameLocation::new(destination);
        let location = match page {
            Some(page) => root.with_page(page),
            None => root,
        };
        self.move_to(location);
    }

    /// Replaces current navigation and clears history.
    pub(crate) fn reset(&mut self, entry: FrameNavigationEntry) {
        self.back_stack.clear();
        self.current = FrameLocation::new(entry);
        self.forward_stack.clear();
    }

    /// Replaces only the current page.
    ///
    /// Back history stays intact so transient destinations such as updated
    /// search queries do not erase the path back to the previous content.
    pub(crate) fn replace_current(&mut self, entry: FrameNavigationEntry) {
        if self.current.page == entry {
            return;
        }
        if let Some(index) = self
            .current
            .ancestors
            .iter()
            .position(|item| *item == entry)
        {
            self.current.ancestors.truncate(index);
        }
        self.current.page = entry;
        self.forward_stack.clear();
    }

    /// Replaces the query of the search page, or starts a new search path.
    ///
    /// If the current page is `Search(_)`, this replaces the query in place.
    /// Otherwise the search starts a new location path, as one history
    /// entry.
    pub(crate) fn replace_active_search_or_push(&mut self, entry: FrameNavigationEntry) {
        debug_assert!(
            matches!(entry, FrameNavigationEntry::Search(_)),
            "replace_active_search_or_push only accepts search entries"
        );

        if matches!(self.current.page, FrameNavigationEntry::Search(_)) {
            self.replace_current(entry);
            return;
        }

        self.start_path(entry, None);
    }

    /// Moves back one history entry and returns the page of that location.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceModelError::CannotNavigateBack`] when there is no
    /// back-history entry.
    pub(crate) fn go_back(&mut self) -> Result<&FrameNavigationEntry, WorkspaceModelError> {
        let previous = self
            .back_stack
            .pop()
            .ok_or(WorkspaceModelError::CannotNavigateBack)?;
        let current = std::mem::replace(&mut self.current, previous);
        self.forward_stack.push(current);
        Ok(&self.current.page)
    }

    /// Moves forward one history entry and returns the page of that location.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceModelError::CannotNavigateForward`] when there is no
    /// forward-history entry.
    pub(crate) fn go_forward(&mut self) -> Result<&FrameNavigationEntry, WorkspaceModelError> {
        let next = self
            .forward_stack
            .pop()
            .ok_or(WorkspaceModelError::CannotNavigateForward)?;
        let current = std::mem::replace(&mut self.current, next);
        self.back_stack.push(current);
        Ok(&self.current.page)
    }
}
