//! Workspace frame navigation state.
//!
//! This module owns the typed breadcrumb/history model for one workspace
//! frame. The parent `workspace` module re-exports the public(crate) types so
//! callers keep the same import path after decomposition.

#![warn(clippy::pedantic)]
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "workspace contracts land before every frame action is wired"
    )
)]

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
    /// Search results by submitted query.
    Search(String),
    /// Remote Index artist drill-down by display name.
    IndexArtistDetail(String),
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
            Self::ArtistDetail(name) | Self::IndexArtistDetail(name) => name.clone(),
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
            Self::Settings => "Settings".to_string(),
            Self::QueueNowPlaying => "Queue".to_string(),
        }
    }
}

/// Per-frame back/forward navigation history.
///
/// The current entry is always present; callers create a history state from
/// the frame's initial destination and then push new destinations as the frame
/// drills into content.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FrameNavigationState {
    back_stack: Vec<FrameNavigationEntry>,
    current: FrameNavigationEntry,
    forward_stack: Vec<FrameNavigationEntry>,
}

impl FrameNavigationState {
    /// Creates navigation history at an initial entry.
    #[must_use]
    pub(crate) const fn new(current: FrameNavigationEntry) -> Self {
        Self {
            back_stack: Vec::new(),
            current,
            forward_stack: Vec::new(),
        }
    }

    /// Returns the current navigation entry.
    #[must_use]
    pub(crate) const fn current(&self) -> &FrameNavigationEntry {
        &self.current
    }

    /// Returns the destination that a back action would select.
    #[must_use]
    pub(crate) fn back_destination(&self) -> Option<&FrameNavigationEntry> {
        self.back_stack.last()
    }

    /// Returns the active search query for a search flow or one of its descendants.
    ///
    /// Search-result drill-down entries remain part of the search flow, so the
    /// owning app can keep the inspector VM alive while breadcrumbs navigate
    /// below a submitted query.
    #[must_use]
    pub(crate) fn active_search_query(&self) -> Option<&str> {
        match &self.current {
            FrameNavigationEntry::Search(query) => Some(query.as_str()),
            _ => self.back_stack.iter().rev().find_map(|entry| match entry {
                FrameNavigationEntry::Search(query) => Some(query.as_str()),
                _ => None,
            }),
        }
    }

    /// Returns the visible navigation path from root history through current.
    #[must_use]
    pub(crate) fn path_entries(&self) -> Vec<FrameNavigationEntry> {
        self.back_stack
            .iter()
            .chain(std::iter::once(&self.current))
            .cloned()
            .collect()
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

    /// Pushes a new current entry and clears forward history.
    pub(crate) fn push(&mut self, entry: FrameNavigationEntry) {
        if self.current == entry {
            return;
        }
        let previous = std::mem::replace(&mut self.current, entry);
        self.back_stack.push(previous);
        self.forward_stack.clear();
    }

    /// Replaces current navigation and clears history.
    pub(crate) fn reset(&mut self, entry: FrameNavigationEntry) {
        self.back_stack.clear();
        self.current = entry;
        self.forward_stack.clear();
    }

    /// Replaces only the current navigation entry.
    ///
    /// Back history stays intact so transient destinations such as updated
    /// search queries do not erase the path back to the previous content.
    pub(crate) fn replace_current(&mut self, entry: FrameNavigationEntry) {
        if self.current == entry {
            return;
        }
        self.current = entry;
        self.forward_stack.clear();
    }

    /// Replaces the active search flow or pushes a new search entry.
    ///
    /// If the frame is already at `Search(_)`, this replaces the current query.
    /// If a search entry exists in the back stack, descendants of that search
    /// are discarded and the new query becomes current. Otherwise this behaves
    /// like [`Self::push`].
    pub(crate) fn replace_active_search_or_push(&mut self, entry: FrameNavigationEntry) {
        debug_assert!(
            matches!(entry, FrameNavigationEntry::Search(_)),
            "replace_active_search_or_push only accepts search entries"
        );

        if matches!(self.current, FrameNavigationEntry::Search(_)) {
            self.replace_current(entry);
            return;
        }

        if let Some(index) = self
            .back_stack
            .iter()
            .rposition(|candidate| matches!(candidate, FrameNavigationEntry::Search(_)))
        {
            self.back_stack.truncate(index);
            self.current = entry;
            self.forward_stack.clear();
            return;
        }

        self.push(entry);
    }

    /// Moves back one navigation entry.
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
        Ok(&self.current)
    }

    /// Moves forward one navigation entry.
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
        Ok(&self.current)
    }

    /// Returns whether the navigation state has more than the root entry.
    ///
    /// Returns true when a back action would be valid (i.e., the `back_stack`
    /// is non-empty), indicating the user can meaningfully press back or a
    /// breadcrumb-segment button.
    #[must_use]
    pub(crate) fn has_history(&self) -> bool {
        !self.back_stack.is_empty()
    }
}
