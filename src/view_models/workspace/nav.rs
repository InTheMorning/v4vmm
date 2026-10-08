//! Workspace frame navigation state.
//!
//! This module owns the typed location and history model for one workspace
//! frame (ADR 0084 Decision 9). The parent `workspace` module re-exports the public(crate) types so
//! callers keep the same import path after decomposition.

#![warn(clippy::pedantic)]

use super::WorkspaceModelError;

/// Navigation target stored in a frame's history.
///
/// Entries are typed so frame history can distinguish entity detail,
/// workspace source, queue, and search destinations without parsing strings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FrameNavigationEntry {
    /// Source-list frame root.
    SourceList,
    /// Playlist detail by playlist id.
    PlaylistDetail(i64),
    /// Track detail by track id.
    TrackDetail(i64),
    /// Album detail by album id.
    AlbumDetail(i64),
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
    /// Remote Index feed drill-down.
    IndexFeedDetail {
        /// Stable remote feed id.
        id: String,
        /// Display label captured from the selected result row.
        label: String,
    },
    /// Remote Index track drill-down.
    IndexTrackDetail {
        /// Stable remote track activation id.
        id: String,
        /// Display label captured from the selected result row.
        label: String,
    },
    /// Music content list filtered to tracks that need broadcast readiness work.
    ReadinessIssues,
    /// Application settings.
    Settings,
    /// Queue and Now Playing frame root.
    QueueNowPlaying,
}

impl FrameNavigationEntry {
    /// Returns the default visible label for this navigation entry.
    #[must_use]
    pub(crate) fn display_label(&self) -> String {
        match self {
            Self::SourceList => "Library".to_string(),
            Self::PlaylistDetail(id) => format!("Playlist {id}"),
            Self::TrackDetail(id) => format!("Track {id}"),
            Self::AlbumDetail(id) => format!("Album {id}"),
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
            Self::IndexFeedDetail { label, .. } | Self::IndexTrackDetail { label, .. } => {
                label.clone()
            }
            Self::ReadinessIssues => "Broadcast Readiness".to_string(),
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
