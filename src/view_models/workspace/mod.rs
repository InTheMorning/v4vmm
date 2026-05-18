//! Workspace frame state view-models.
//!
//! ADR 0046 introduces an application workspace that can describe frame
//! layout, focus, and per-frame navigation without depending on the rendering
//! layer. Screens and composites bind these plain Rust types to GPUI chrome in
//! later tasks.

#![warn(clippy::pedantic)]
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "workspace contracts land before every frame action is wired"
    )
)]

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

mod breadcrumb;
mod chrome;
mod frame;
mod nav;
#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) use breadcrumb::BreadcrumbTruncation;
pub(crate) use breadcrumb::{BreadcrumbDisplay, BreadcrumbSegment};
pub(crate) use chrome::{
    ContentFilter, FilterChipOption, FilterChipStripDisplay, FrameChromeButtonDisplay,
    FrameChromeMenuItemDisplay, FrameShellDisplay,
};
pub(crate) use frame::{
    FrameDetachEligibility, FrameDockTarget, FrameSearchDescriptor, FrameSearchScope,
    WorkspaceFrameId, WorkspaceFrameKind, WorkspaceFrameState,
};
pub(crate) use nav::{FrameNavigationEntry, FrameNavigationState};

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
    /// The requested frame removal would leave the workspace empty.
    LastFrameRemoval,
    /// The requested operation needs at least one frame.
    EmptyLayout,
    /// The frame has no back-history entry to select.
    CannotNavigateBack,
    /// The frame has no forward-history entry to select.
    CannotNavigateForward,
    /// The detach request is valid but windowing support is deferred.
    DetachDeferred(WorkspaceFrameId),
    /// The dock request is valid but windowing support is deferred.
    DockDeferred {
        /// Frame that requested docking.
        frame_id: WorkspaceFrameId,
        /// Requested dock lane.
        target: FrameDockTarget,
    },
    /// The frame is anchored and cannot detach or dock.
    NotDetachable(WorkspaceFrameId),
}

impl fmt::Display for WorkspaceModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FrameNotFound(id) => write!(f, "workspace frame {} was not found", id.value()),
            Self::DuplicateFrameId(id) => {
                write!(f, "workspace frame {} already exists", id.value())
            }
            Self::LastFrameRemoval => f.write_str("cannot remove the last workspace frame"),
            Self::EmptyLayout => f.write_str("workspace layout contains no frames"),
            Self::CannotNavigateBack => f.write_str("workspace frame has no back history"),
            Self::CannotNavigateForward => f.write_str("workspace frame has no forward history"),
            Self::DetachDeferred(id) => write!(
                f,
                "workspace frame {} detach is deferred until windowing support exists",
                id.value()
            ),
            Self::DockDeferred { frame_id, target } => write!(
                f,
                "workspace frame {} dock to {} is deferred until windowing support exists",
                frame_id.value(),
                target.label()
            ),
            Self::NotDetachable(id) => {
                write!(f, "workspace frame {} is not detachable", id.value())
            }
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
    const QUEUE_NOW_PLAYING_ID: WorkspaceFrameId = WorkspaceFrameId::new(4);

    /// Returns the default primary content frame identifier.
    #[must_use]
    pub(crate) const fn default_content_frame_id() -> WorkspaceFrameId {
        Self::CONTENT_LIST_ID
    }

    /// Returns the default detail frame identifier.
    #[must_use]
    pub(crate) const fn default_detail_frame_id() -> WorkspaceFrameId {
        Self::DETAIL_ID
    }

    /// Creates the ADR 0046 default workspace layout.
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
            WorkspaceFrameState::with_default_title(
                Self::QUEUE_NOW_PLAYING_ID,
                WorkspaceFrameKind::QueueNowPlaying,
            ),
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

    /// Creates an empty workspace layout.
    #[must_use]
    pub(crate) fn empty() -> Self {
        Self {
            frames: Vec::new(),
            focused_frame_id: None,
            frame_navigation: BTreeMap::new(),
        }
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

    /// Pops a frame's back-history entry and returns the new current entry.
    pub(crate) fn pop_nav(&mut self, id: WorkspaceFrameId) -> Option<FrameNavigationEntry> {
        self.frame_navigation.get_mut(&id)?.go_back().ok().cloned()
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
    /// A search from a non-search destination pushes
    /// [`FrameNavigationEntry::Search`] so the user can navigate back to their
    /// previous content. A search submitted while an existing search flow is
    /// active replaces that search entry and discards its descendants, preserving
    /// earlier history without stacking query crumbs. The `ContentList` frame
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

    /// Pops the named frame's nav stack until the given entry is on top.
    ///
    /// If the entry is already at the top, this is a no-op. If the entry does
    /// not exist in the back-history, returns [`WorkspaceModelError::FrameNotFound`].
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceModelError::FrameNotFound`] if the frame or the target
    /// entry does not exist in the frame's history.
    pub(crate) fn pop_nav_until(
        &mut self,
        frame_id: WorkspaceFrameId,
        target_entry: &FrameNavigationEntry,
    ) -> Result<(), WorkspaceModelError> {
        let nav_state = self
            .frame_nav_mut(frame_id)
            .ok_or(WorkspaceModelError::FrameNotFound(frame_id))?;

        // If already at target, no-op
        if nav_state.current() == target_entry {
            return Ok(());
        }

        let path = nav_state.path_entries();
        let Some(found_at) = path.iter().position(|entry| entry == target_entry) else {
            return Err(WorkspaceModelError::FrameNotFound(frame_id));
        };
        let back_stack_len = path.len().saturating_sub(1);

        // Pop until the target is on top: we need to pop (back_stack_len - found_at) times
        for _ in 0..(back_stack_len - found_at) {
            nav_state.go_back()?;
        }

        Ok(())
    }

    /// Returns the currently focused frame id.
    #[must_use]
    pub(crate) const fn focused_frame_id(&self) -> Option<WorkspaceFrameId> {
        self.focused_frame_id
    }

    /// Returns the currently focused frame.
    #[must_use]
    pub(crate) fn focused_frame(&self) -> Option<&WorkspaceFrameState> {
        let focused_id = self.focused_frame_id?;
        self.frames.iter().find(|frame| frame.id() == focused_id)
    }

    /// Projects the focused frame into a toolbar search descriptor.
    #[must_use]
    pub(crate) fn focused_search_descriptor(&self) -> Option<FrameSearchDescriptor> {
        let frame = self.focused_frame()?;
        let nav = self.frame_nav(frame.id())?.current().clone();
        let (scope, placeholder) = match (frame.kind(), &nav) {
            (WorkspaceFrameKind::SourceList, _) => (FrameSearchScope::Sidebar, "Filter sidebar..."),
            (WorkspaceFrameKind::ContentList, FrameNavigationEntry::Settings) => {
                (FrameSearchScope::SettingsRows, "Search settings...")
            }
            (
                WorkspaceFrameKind::ContentList,
                FrameNavigationEntry::SourceList
                | FrameNavigationEntry::Search(_)
                | FrameNavigationEntry::RecentFeeds,
            ) => (FrameSearchScope::LibraryRows, "Search library..."),
            (WorkspaceFrameKind::ContentList, _) => {
                (FrameSearchScope::DetailTracks, "Filter tracks...")
            }
            (WorkspaceFrameKind::Detail, FrameNavigationEntry::Search(_)) => {
                (FrameSearchScope::InspectorQuery, "Refine search...")
            }
            (WorkspaceFrameKind::Detail, _) => (FrameSearchScope::DetailTracks, "Filter tracks..."),
            (WorkspaceFrameKind::QueueNowPlaying, _) => {
                (FrameSearchScope::QueueRows, "Filter queue...")
            }
        };

        Some(FrameSearchDescriptor {
            frame_id: frame.id(),
            kind: frame.kind(),
            nav,
            scope,
            placeholder,
        })
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

    /// Adds a frame kind to the end of the workspace.
    ///
    /// The new frame receives the next stable frame id and becomes focused.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceModelError::DuplicateFrameId`] if the generated frame
    /// id already exists.
    pub(crate) fn add_frame(
        &mut self,
        kind: WorkspaceFrameKind,
    ) -> Result<WorkspaceFrameId, WorkspaceModelError> {
        let id = self.next_frame_id();
        self.add_frame_state(WorkspaceFrameState::with_default_title(id, kind))?;
        self.focus_frame(id)?;
        Ok(id)
    }

    /// Adds a caller-provided frame state to the end of the workspace.
    ///
    /// Construction and tests use this helper when explicit ids are required.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceModelError::DuplicateFrameId`] if the frame id already
    /// exists.
    fn add_frame_state(&mut self, frame: WorkspaceFrameState) -> Result<(), WorkspaceModelError> {
        let id = frame.id();
        if self.frames.iter().any(|existing| existing.id() == id) {
            return Err(WorkspaceModelError::DuplicateFrameId(id));
        }
        let kind = frame.kind();
        self.frames.push(frame);
        self.frame_navigation
            .entry(id)
            .or_insert_with(|| FrameNavigationState::new(default_navigation_entry(kind)));
        if self.focused_frame_id.is_none() {
            self.focused_frame_id = Some(id);
        }
        self.sync_focus_flags();
        Ok(())
    }

    /// Removes a frame from the workspace.
    ///
    /// If the focused frame is removed, focus moves to the left sibling, or the
    /// first remaining frame when there is no left sibling.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceModelError::FrameNotFound`] when the frame does not
    /// exist. Returns [`WorkspaceModelError::LastFrameRemoval`] when the
    /// requested removal would leave the workspace empty.
    pub(crate) fn remove_frame(&mut self, id: WorkspaceFrameId) -> Result<(), WorkspaceModelError> {
        let Some(position) = self.frames.iter().position(|frame| frame.id() == id) else {
            return Err(WorkspaceModelError::FrameNotFound(id));
        };
        if self.frames.len() == 1 {
            return Err(WorkspaceModelError::LastFrameRemoval);
        }
        self.frames.remove(position);
        self.frame_navigation.remove(&id);
        if self.focused_frame_id == Some(id) {
            let next_focus_index = position.saturating_sub(1);
            self.focused_frame_id = self
                .frames
                .get(next_focus_index)
                .map(WorkspaceFrameState::id);
        }
        self.sync_focus_flags();
        Ok(())
    }

    /// Requests that an eligible frame detach into a separate surface.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceModelError::FrameNotFound`] when the frame does not
    /// exist. Returns [`WorkspaceModelError::NotDetachable`] when the frame is
    /// anchored. Returns [`WorkspaceModelError::DetachDeferred`] for eligible
    /// frames until a future windowing ADR implements actual detach behavior.
    pub(crate) fn request_detach(&self, id: WorkspaceFrameId) -> Result<(), WorkspaceModelError> {
        match self.frame_detach_eligibility(id)? {
            FrameDetachEligibility::Detachable => Err(WorkspaceModelError::DetachDeferred(id)),
            FrameDetachEligibility::NotDetachable => Err(WorkspaceModelError::NotDetachable(id)),
        }
    }

    /// Requests that an eligible frame dock into a workspace lane.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceModelError::FrameNotFound`] when the frame does not
    /// exist. Returns [`WorkspaceModelError::NotDetachable`] when the frame is
    /// anchored. Returns [`WorkspaceModelError::DockDeferred`] for eligible
    /// frames until a future windowing ADR implements actual dock behavior.
    pub(crate) fn request_dock(
        &self,
        id: WorkspaceFrameId,
        target: FrameDockTarget,
    ) -> Result<(), WorkspaceModelError> {
        match self.frame_detach_eligibility(id)? {
            FrameDetachEligibility::Detachable => Err(WorkspaceModelError::DockDeferred {
                frame_id: id,
                target,
            }),
            FrameDetachEligibility::NotDetachable => Err(WorkspaceModelError::NotDetachable(id)),
        }
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

    fn frame_detach_eligibility(
        &self,
        id: WorkspaceFrameId,
    ) -> Result<FrameDetachEligibility, WorkspaceModelError> {
        self.frames
            .iter()
            .find(|frame| frame.id() == id)
            .map(|frame| frame.kind().detach_eligibility())
            .ok_or(WorkspaceModelError::FrameNotFound(id))
    }

    fn next_frame_id(&self) -> WorkspaceFrameId {
        let next = self
            .frames
            .iter()
            .map(|frame| frame.id().value())
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        WorkspaceFrameId::new(next)
    }

    fn sync_focus_flags(&mut self) {
        for frame in &mut self.frames {
            frame.set_focused(Some(frame.id()) == self.focused_frame_id);
        }
    }

    fn ensure_frame_navigation_entries(&mut self) {
        for frame in &self.frames {
            self.frame_navigation.entry(frame.id()).or_insert_with(|| {
                FrameNavigationState::new(default_navigation_entry(frame.kind()))
            });
        }
    }
}

fn default_navigation_entry(kind: WorkspaceFrameKind) -> FrameNavigationEntry {
    match kind {
        WorkspaceFrameKind::SourceList | WorkspaceFrameKind::ContentList => {
            FrameNavigationEntry::SourceList
        }
        WorkspaceFrameKind::Detail => FrameNavigationEntry::TrackDetail(0),
        WorkspaceFrameKind::QueueNowPlaying => FrameNavigationEntry::QueueNowPlaying,
    }
}
