//! Workspace frame state view-models.
//!
//! ADR 0046 introduces an application workspace that can describe frame
//! layout, focus, and per-frame navigation without depending on the rendering
//! layer. Screens and composites bind these plain Rust types to GPUI chrome in
//! later tasks.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

mod breadcrumb;
mod chrome;
mod columns;
mod frame;
mod nav;
#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) use breadcrumb::BreadcrumbTruncation;
pub(crate) use breadcrumb::{BreadcrumbDisplay, BreadcrumbSegment};
pub(crate) use chrome::{
    ContentFilter, ContentViewMode, ContentViewModeControlDisplay, FilterChipOption,
    FilterChipStripDisplay, FilterChipStripWidthClass, FrameChromeButtonDisplay,
    FrameChromeMenuItemDisplay, FrameShellDisplay,
};
pub(crate) use columns::{
    resolve_music_sidebar_state, toggle_music_sidebar_preference, MusicColumns,
    MusicSidebarPreference, MusicSidebarState,
};
pub(crate) use frame::{WorkspaceFrameId, WorkspaceFrameKind, WorkspaceFrameState};
pub(crate) use nav::{
    release_download_updates_open_page, resolve_release_page_source, resolve_track_page_source,
    track_download_updates_open_page, track_release_download_updates_open_page, DiscoverView,
    FrameNavigationEntry, FrameNavigationState, MusicDestination, NavigationPathStart, ReleaseKey,
    ReleasePageSource, TrackKey, TrackPageSource,
};

/// Workspace model mutation failure.
///
/// All fallible workspace operations use this error instead of panicking on
/// invalid frame identifiers, duplicate frames, or unavailable navigation
/// history.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum WorkspaceModelError {
    /// A requested frame identifier does not exist in the layout.
    FrameNotFound(WorkspaceFrameId),
    /// A new frame would duplicate an existing identifier.
    DuplicateFrameId(WorkspaceFrameId),
    /// The requested operation needs at least one frame.
    EmptyLayout,
    /// The frame has no back-history entry to select.
    CannotNavigateBack,
    /// The frame has no forward-history entry to select.
    CannotNavigateForward,
}

impl fmt::Display for WorkspaceModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FrameNotFound(id) => write!(f, "workspace frame {} was not found", id.value()),
            Self::DuplicateFrameId(id) => {
                write!(f, "workspace frame {} already exists", id.value())
            }
            Self::EmptyLayout => f.write_str("workspace layout contains no frames"),
            Self::CannotNavigateBack => f.write_str("workspace frame has no back history"),
            Self::CannotNavigateForward => f.write_str("workspace frame has no forward history"),
        }
    }
}

impl Error for WorkspaceModelError {}

/// Ordered workspace layout and focus state.
///
/// The layout owns frame ordering and focus invariants. Empty layouts are valid
/// as an intermediate model state, but focus operations on an empty layout
/// return [`WorkspaceModelError::EmptyLayout`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkspaceLayout {
    frames: Vec<WorkspaceFrameState>,
    focused_frame_id: Option<WorkspaceFrameId>,
    frame_navigation: BTreeMap<WorkspaceFrameId, FrameNavigationState>,
}

/// Serializable workspace layout configuration.
///
/// The DTO stays GPUI-free and stores only stable frame ordering plus the
/// focused frame id. Invalid or empty configs are ignored by
/// [`WorkspaceLayout::from_config`] in favor of the default ADR 0046 layout.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct WorkspaceLayoutConfig {
    /// Ordered frame configurations.
    pub(crate) frames: Vec<WorkspaceFrameConfig>,
    /// Focused frame id, when a persisted layout has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) focused_frame_id: Option<u64>,
}

/// Serializable workspace frame configuration.
///
/// Frame ids are persisted as numeric values while frame kinds use stable
/// snake-case strings through [`WorkspaceFrameKind`] serde annotations.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct WorkspaceFrameConfig {
    /// Stable frame identifier.
    pub(crate) id: u64,
    /// Structural role for this frame.
    pub(crate) kind: WorkspaceFrameKind,
}

impl Default for WorkspaceLayout {
    fn default() -> Self {
        Self::default_layout()
    }
}

impl WorkspaceLayout {
    const SOURCE_LIST_ID: WorkspaceFrameId = WorkspaceFrameId::new(1);
    const CONTENT_LIST_ID: WorkspaceFrameId = WorkspaceFrameId::new(2);
    const DETAIL_ID: WorkspaceFrameId = WorkspaceFrameId::new(3);

    /// Creates the ADR 0060 default curation workspace layout.
    #[must_use]
    pub(crate) fn default_layout() -> Self {
        let frames = vec![
            WorkspaceFrameState::with_default_title(
                Self::SOURCE_LIST_ID,
                WorkspaceFrameKind::SourceList,
            ),
            WorkspaceFrameState::with_default_title(
                Self::CONTENT_LIST_ID,
                WorkspaceFrameKind::ContentList,
            ),
            WorkspaceFrameState::with_default_title(Self::DETAIL_ID, WorkspaceFrameKind::Detail),
        ];
        let mut layout = Self {
            frames,
            focused_frame_id: Some(Self::CONTENT_LIST_ID),
            frame_navigation: BTreeMap::new(),
        };
        layout.ensure_frame_navigation_entries();
        layout.sync_focus_flags();
        layout
    }

    /// Creates a workspace layout from ordered frames and optional focus.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceModelError::DuplicateFrameId`] if any frame id appears
    /// more than once. Returns [`WorkspaceModelError::FrameNotFound`] when a
    /// requested focused frame is not present. Returns
    /// [`WorkspaceModelError::EmptyLayout`] when a focused frame is requested for
    /// an empty layout.
    pub(crate) fn new(
        frames: Vec<WorkspaceFrameState>,
        focused_frame_id: Option<WorkspaceFrameId>,
    ) -> Result<Self, WorkspaceModelError> {
        let mut layout = Self {
            frames,
            focused_frame_id: None,
            frame_navigation: BTreeMap::new(),
        };
        layout.ensure_unique_frame_ids()?;
        layout.ensure_frame_navigation_entries();
        if let Some(id) = focused_frame_id {
            layout.focus_frame(id)?;
        } else {
            layout.focused_frame_id = layout.frames.first().map(WorkspaceFrameState::id);
            layout.sync_focus_flags();
        }
        Ok(layout)
    }

    /// Returns the ordered frame list.
    #[must_use]
    pub(crate) fn frames(&self) -> &[WorkspaceFrameState] {
        &self.frames
    }

    /// Returns the navigation state for a frame, when present.
    #[must_use]
    pub(crate) fn frame_nav(&self, id: WorkspaceFrameId) -> Option<&FrameNavigationState> {
        self.frame_navigation.get(&id)
    }

    /// Returns mutable navigation state for a frame, when present.
    pub(crate) fn frame_nav_mut(
        &mut self,
        id: WorkspaceFrameId,
    ) -> Option<&mut FrameNavigationState> {
        self.frame_navigation.get_mut(&id)
    }

    /// Resets a frame's navigation state.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceModelError::FrameNotFound`] when the frame does not
    /// exist in the layout.
    pub(crate) fn reset_nav(
        &mut self,
        id: WorkspaceFrameId,
        entry: FrameNavigationEntry,
    ) -> Result<(), WorkspaceModelError> {
        self.frame_nav_mut(id)
            .ok_or(WorkspaceModelError::FrameNotFound(id))?
            .reset(entry);
        Ok(())
    }

    /// Pushes navigation onto a frame's history.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceModelError::FrameNotFound`] when the frame does not
    /// exist in the layout.
    pub(crate) fn push_nav(
        &mut self,
        id: WorkspaceFrameId,
        entry: FrameNavigationEntry,
    ) -> Result<(), WorkspaceModelError> {
        self.frame_nav_mut(id)
            .ok_or(WorkspaceModelError::FrameNotFound(id))?
            .push(entry);
        Ok(())
    }

    /// Opens a list column row under the destination of a frame's current
    /// location (ADR 0084 Decision 9).
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceModelError::FrameNotFound`] when the frame does not
    /// exist in the layout.
    pub(crate) fn open_nav_list_row(
        &mut self,
        id: WorkspaceFrameId,
        entry: FrameNavigationEntry,
    ) -> Result<(), WorkspaceModelError> {
        self.frame_nav_mut(id)
            .ok_or(WorkspaceModelError::FrameNotFound(id))?
            .open_list_row(entry);
        Ok(())
    }

    /// Starts a new location path on a frame (ADR 0084 Decision 9).
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceModelError::FrameNotFound`] when the frame does not
    /// exist in the layout.
    pub(crate) fn start_nav_path(
        &mut self,
        id: WorkspaceFrameId,
        destination: FrameNavigationEntry,
        page: Option<FrameNavigationEntry>,
    ) -> Result<(), WorkspaceModelError> {
        self.frame_nav_mut(id)
            .ok_or(WorkspaceModelError::FrameNotFound(id))?
            .start_path(destination, page);
        Ok(())
    }

    /// Pops a frame's back-history entry and returns the new current entry.
    pub(crate) fn pop_nav(&mut self, id: WorkspaceFrameId) -> Option<FrameNavigationEntry> {
        self.frame_navigation.get_mut(&id)?.go_back().ok().cloned()
    }

    /// Pops a frame's forward-history entry and returns the new current entry.
    pub(crate) fn pop_nav_forward(&mut self, id: WorkspaceFrameId) -> Option<FrameNavigationEntry> {
        self.frame_navigation
            .get_mut(&id)?
            .go_forward()
            .ok()
            .cloned()
    }

    /// Replaces a frame's full navigation state.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceModelError::FrameNotFound`] when the frame does not
    /// exist in the layout.
    pub(crate) fn replace_nav(
        &mut self,
        id: WorkspaceFrameId,
        nav: FrameNavigationState,
    ) -> Result<(), WorkspaceModelError> {
        let slot = self
            .frame_navigation
            .get_mut(&id)
            .ok_or(WorkspaceModelError::FrameNotFound(id))?;
        *slot = nav;
        Ok(())
    }

    /// Opens search results in the `ContentList` frame's nav stack.
    ///
    /// A search from a non-search page starts a new location path with
    /// [`FrameNavigationEntry::Search`], as one history entry, so the user can
    /// navigate back to their previous content. A search submitted on the
    /// search page replaces its query in place. The `ContentList` frame
    /// must exist in a valid layout; it is not auto-created.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceModelError::FrameNotFound`] if the `ContentList` frame
    /// does not exist.
    pub(crate) fn open_search_results_in_content_list(
        &mut self,
        query: impl Into<String>,
    ) -> Result<WorkspaceFrameId, WorkspaceModelError> {
        let query = query.into();
        let content_list_frame_id = self
            .frames
            .iter()
            .find(|frame| frame.kind() == WorkspaceFrameKind::ContentList)
            .map(WorkspaceFrameState::id)
            .ok_or(WorkspaceModelError::FrameNotFound(Self::CONTENT_LIST_ID))?;

        let nav = self
            .frame_nav_mut(content_list_frame_id)
            .ok_or(WorkspaceModelError::FrameNotFound(content_list_frame_id))?;
        nav.replace_active_search_or_push(FrameNavigationEntry::Search(query));
        self.focus_frame(content_list_frame_id)?;
        Ok(content_list_frame_id)
    }

    /// Returns the currently focused frame id.
    #[must_use]
    pub(crate) const fn focused_frame_id(&self) -> Option<WorkspaceFrameId> {
        self.focused_frame_id
    }

    /// Focuses an existing frame.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceModelError::EmptyLayout`] when the layout has no
    /// frames. Returns [`WorkspaceModelError::FrameNotFound`] when the requested
    /// frame does not exist.
    pub(crate) fn focus_frame(&mut self, id: WorkspaceFrameId) -> Result<(), WorkspaceModelError> {
        if self.frames.is_empty() {
            return Err(WorkspaceModelError::EmptyLayout);
        }
        if !self.frames.iter().any(|frame| frame.id() == id) {
            return Err(WorkspaceModelError::FrameNotFound(id));
        }
        self.focused_frame_id = Some(id);
        self.sync_focus_flags();
        Ok(())
    }

    /// Sets the title of the frame with id `id` (ADR 0084 task 015,
    /// operator decision 2026-10-17: the Detail frame title follows the
    /// open page for a singles group).
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceModelError::FrameNotFound`] when the frame does not
    /// exist in the layout.
    pub(crate) fn set_frame_title(
        &mut self,
        id: WorkspaceFrameId,
        title: impl Into<String>,
    ) -> Result<(), WorkspaceModelError> {
        self.frames
            .iter_mut()
            .find(|frame| frame.id() == id)
            .ok_or(WorkspaceModelError::FrameNotFound(id))?
            .set_title(title);
        Ok(())
    }

    /// Converts this layout to a serializable configuration DTO.
    #[must_use]
    pub(crate) fn to_config(&self) -> WorkspaceLayoutConfig {
        WorkspaceLayoutConfig {
            frames: self
                .frames
                .iter()
                .map(|frame| WorkspaceFrameConfig {
                    id: frame.id().value(),
                    kind: frame.kind(),
                })
                .collect(),
            focused_frame_id: self.focused_frame_id.map(WorkspaceFrameId::value),
        }
    }

    /// Creates a workspace layout from an optional configuration DTO.
    ///
    /// Missing, empty, duplicate, or otherwise invalid configs fall back to the
    /// ADR 0046 default layout.
    #[must_use]
    pub(crate) fn from_config(config: Option<&WorkspaceLayoutConfig>) -> Self {
        let Some(config) = config else {
            return Self::default_layout();
        };
        if config.frames.is_empty() {
            return Self::default_layout();
        }

        let frames: Vec<_> = config
            .frames
            .iter()
            .map(|frame| {
                WorkspaceFrameState::with_default_title(WorkspaceFrameId::new(frame.id), frame.kind)
            })
            .collect();
        let focused_frame_id = config.focused_frame_id.map(WorkspaceFrameId::new);

        Self::new(frames, focused_frame_id).unwrap_or_else(|_| Self::default_layout())
    }

    fn ensure_unique_frame_ids(&self) -> Result<(), WorkspaceModelError> {
        let mut seen = Vec::with_capacity(self.frames.len());
        for frame in &self.frames {
            if seen.contains(&frame.id()) {
                return Err(WorkspaceModelError::DuplicateFrameId(frame.id()));
            }
            seen.push(frame.id());
        }
        Ok(())
    }

    fn sync_focus_flags(&mut self) {
        for frame in &mut self.frames {
            frame.set_focused(Some(frame.id()) == self.focused_frame_id);
        }
    }

    fn ensure_frame_navigation_entries(&mut self) {
        for frame in &self.frames {
            self.frame_navigation.entry(frame.id()).or_insert_with(|| {
                let root = default_navigation_entry(frame.kind());
                match default_navigation_child(frame.kind()) {
                    Some(page) => FrameNavigationState::new_with_page(root, page),
                    None => FrameNavigationState::new(root),
                }
            });
        }
    }
}

fn default_navigation_entry(kind: WorkspaceFrameKind) -> FrameNavigationEntry {
    match kind {
        // ADR 0084 Decision 3: Music opens on Discover.
        WorkspaceFrameKind::SourceList | WorkspaceFrameKind::ContentList => {
            FrameNavigationEntry::Destination(MusicDestination::Discover)
        }
        WorkspaceFrameKind::Detail => {
            FrameNavigationEntry::Track(TrackKey::from_index_track_target(None, String::new()))
        }
        WorkspaceFrameKind::QueueNowPlaying => FrameNavigationEntry::QueueNowPlaying,
    }
}

/// The page under the Music default location, when its destination opens
/// one beneath it (ADR 0084 Decision 3: Music opens on Discover, then
/// "New releases").
fn default_navigation_child(kind: WorkspaceFrameKind) -> Option<FrameNavigationEntry> {
    match kind {
        WorkspaceFrameKind::SourceList | WorkspaceFrameKind::ContentList => {
            MusicDestination::Discover.default_child_page()
        }
        WorkspaceFrameKind::Detail | WorkspaceFrameKind::QueueNowPlaying => None,
    }
}
