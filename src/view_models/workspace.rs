//! Workspace frame state view-models.
//!
//! ADR 0046 introduces an application workspace that can describe frame
//! layout, focus, and per-frame navigation without depending on the rendering
//! layer. Screens and composites bind these plain Rust types to GPUI chrome in
//! later tasks.

#![warn(clippy::pedantic)]
#![expect(
    dead_code,
    reason = "ADR 0046 Task 001 lands workspace contracts before render wiring"
)]

use std::error::Error;
use std::fmt;

/// Stable identifier for a workspace frame.
///
/// Frame identifiers are opaque to callers. The workspace model only requires
/// equality and ordering within one layout snapshot; persistence can map these
/// numeric values into a stored layout in a later ADR 0046 task.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct WorkspaceFrameId(u64);

impl WorkspaceFrameId {
    /// Creates a workspace frame identifier.
    #[must_use]
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw identifier value.
    #[must_use]
    pub(crate) const fn value(self) -> u64 {
        self.0
    }
}

/// Structural role for a workspace frame.
///
/// The enum keeps frame identity typed instead of stringly-typed. Renderers can
/// map each variant to frame chrome and content without accepting unknown frame
/// kinds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorkspaceFrameKind {
    /// Library tree, playlists, saved searches, and settings entry points.
    SourceList,
    /// Selected library, search, or playlist results.
    ContentList,
    /// Track, feed, album, artist, or settings details.
    Detail,
    /// Queue, playback status, liveValue output, and output controls.
    QueueNowPlaying,
}

impl WorkspaceFrameKind {
    /// Returns the default title for this frame kind.
    #[must_use]
    pub(crate) const fn default_title(self) -> &'static str {
        match self {
            Self::SourceList => "Library",
            Self::ContentList => "Content",
            Self::Detail => "Detail",
            Self::QueueNowPlaying => "Queue",
        }
    }
}

/// Display-ready state for one workspace frame.
///
/// The frame carries only plain data. Focus is mirrored here so frame chrome can
/// render without recomputing layout state, while [`WorkspaceLayout`] owns the
/// invariant that at most one frame is focused.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkspaceFrameState {
    id: WorkspaceFrameId,
    kind: WorkspaceFrameKind,
    title: String,
    subtitle: Option<String>,
    status: Option<String>,
    focused: bool,
}

impl WorkspaceFrameState {
    /// Creates a frame with a caller-provided title.
    #[must_use]
    pub(crate) fn new(
        id: WorkspaceFrameId,
        kind: WorkspaceFrameKind,
        title: impl Into<String>,
    ) -> Self {
        Self {
            id,
            kind,
            title: title.into(),
            subtitle: None,
            status: None,
            focused: false,
        }
    }

    /// Creates a frame using the title associated with its kind.
    #[must_use]
    pub(crate) fn with_default_title(id: WorkspaceFrameId, kind: WorkspaceFrameKind) -> Self {
        Self::new(id, kind, kind.default_title())
    }

    /// Returns this frame's identifier.
    #[must_use]
    pub(crate) const fn id(&self) -> WorkspaceFrameId {
        self.id
    }

    /// Returns this frame's structural role.
    #[must_use]
    pub(crate) const fn kind(&self) -> WorkspaceFrameKind {
        self.kind
    }

    /// Returns this frame's title.
    #[must_use]
    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    /// Returns this frame's optional subtitle.
    #[must_use]
    pub(crate) fn subtitle(&self) -> Option<&str> {
        self.subtitle.as_deref()
    }

    /// Returns this frame's optional status text.
    #[must_use]
    pub(crate) fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    /// Returns whether this frame is currently focused.
    #[must_use]
    pub(crate) const fn is_focused(&self) -> bool {
        self.focused
    }

    /// Returns this frame with subtitle text attached.
    #[must_use]
    pub(crate) fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Returns this frame with status text attached.
    #[must_use]
    pub(crate) fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
}

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
        };
        layout.sync_focus_flags();
        layout
    }

    /// Creates an empty workspace layout.
    #[must_use]
    pub(crate) const fn empty() -> Self {
        Self {
            frames: Vec::new(),
            focused_frame_id: None,
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
        };
        layout.ensure_unique_frame_ids()?;
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

    /// Returns the currently focused frame id.
    #[must_use]
    pub(crate) const fn focused_frame_id(&self) -> Option<WorkspaceFrameId> {
        self.focused_frame_id
    }

    /// Returns the currently focused frame.
    #[must_use]
    pub(crate) fn focused_frame(&self) -> Option<&WorkspaceFrameState> {
        let focused_id = self.focused_frame_id?;
        self.frames.iter().find(|frame| frame.id == focused_id)
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
        if !self.frames.iter().any(|frame| frame.id == id) {
            return Err(WorkspaceModelError::FrameNotFound(id));
        }
        self.focused_frame_id = Some(id);
        self.sync_focus_flags();
        Ok(())
    }

    /// Adds a frame to the end of the workspace.
    ///
    /// The first added frame becomes focused. Later additions preserve the
    /// existing focus until callers explicitly focus the new frame.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceModelError::DuplicateFrameId`] if the frame id already
    /// exists.
    pub(crate) fn add_frame(
        &mut self,
        frame: WorkspaceFrameState,
    ) -> Result<(), WorkspaceModelError> {
        let id = frame.id;
        if self.frames.iter().any(|existing| existing.id == id) {
            return Err(WorkspaceModelError::DuplicateFrameId(id));
        }
        self.frames.push(frame);
        if self.focused_frame_id.is_none() {
            self.focused_frame_id = Some(id);
        }
        self.sync_focus_flags();
        Ok(())
    }

    /// Removes a frame from the workspace.
    ///
    /// If the focused frame is removed, focus moves to the next frame at the
    /// same index or the final remaining frame.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceModelError::FrameNotFound`] when the frame does not
    /// exist.
    pub(crate) fn remove_frame(
        &mut self,
        id: WorkspaceFrameId,
    ) -> Result<WorkspaceFrameState, WorkspaceModelError> {
        let Some(position) = self.frames.iter().position(|frame| frame.id == id) else {
            return Err(WorkspaceModelError::FrameNotFound(id));
        };
        let removed = self.frames.remove(position);
        if self.focused_frame_id == Some(id) {
            self.focused_frame_id = self
                .frames
                .get(position)
                .or_else(|| self.frames.last())
                .map(WorkspaceFrameState::id);
        }
        self.sync_focus_flags();
        Ok(removed)
    }

    fn ensure_unique_frame_ids(&self) -> Result<(), WorkspaceModelError> {
        let mut seen = Vec::with_capacity(self.frames.len());
        for frame in &self.frames {
            if seen.contains(&frame.id) {
                return Err(WorkspaceModelError::DuplicateFrameId(frame.id));
            }
            seen.push(frame.id);
        }
        Ok(())
    }

    fn sync_focus_flags(&mut self) {
        for frame in &mut self.frames {
            frame.set_focused(Some(frame.id) == self.focused_frame_id);
        }
    }
}

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
    /// Queue and Now Playing frame root.
    QueueNowPlaying,
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
        let previous = std::mem::replace(&mut self.current, entry);
        self.back_stack.push(previous);
        self.forward_stack.clear();
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
}

#[cfg(test)]
mod tests {
    use super::{
        FrameNavigationEntry, FrameNavigationState, WorkspaceFrameId, WorkspaceFrameKind,
        WorkspaceFrameState, WorkspaceLayout, WorkspaceModelError,
    };

    fn frame(id: u64, kind: WorkspaceFrameKind) -> WorkspaceFrameState {
        WorkspaceFrameState::with_default_title(WorkspaceFrameId::new(id), kind)
    }

    #[test]
    fn default_layout_has_expected_workspace_shape() {
        let layout = WorkspaceLayout::default_layout();
        let kinds: Vec<_> = layout
            .frames()
            .iter()
            .map(WorkspaceFrameState::kind)
            .collect();

        assert_eq!(
            kinds,
            [
                WorkspaceFrameKind::SourceList,
                WorkspaceFrameKind::ContentList,
                WorkspaceFrameKind::Detail,
                WorkspaceFrameKind::QueueNowPlaying,
            ],
            "default workspace should expose the ADR 0046 frame order"
        );
        assert_eq!(
            layout.focused_frame().map(WorkspaceFrameState::kind),
            Some(WorkspaceFrameKind::ContentList),
            "default workspace should focus the primary content frame"
        );
        assert_eq!(
            layout
                .frames()
                .iter()
                .filter(|frame| frame.is_focused())
                .count(),
            1,
            "default workspace should mark exactly one focused frame"
        );
    }

    #[test]
    fn empty_layout_has_no_focus_and_rejects_focus_mutation() {
        let mut layout = WorkspaceLayout::empty();

        assert!(
            layout.frames().is_empty(),
            "empty layout should contain no frames"
        );
        assert_eq!(
            layout.focused_frame_id(),
            None,
            "empty layout should not carry a focused frame id"
        );
        assert_eq!(
            layout.focus_frame(WorkspaceFrameId::new(1)),
            Err(WorkspaceModelError::EmptyLayout),
            "empty layout should not accept focus mutation"
        );
    }

    #[test]
    fn single_frame_layout_marks_only_frame_focused() {
        let layout = WorkspaceLayout::new(
            vec![frame(10, WorkspaceFrameKind::Detail)],
            Some(WorkspaceFrameId::new(10)),
        )
        .expect("single-frame layout should be valid");

        assert_eq!(
            layout.focused_frame().map(WorkspaceFrameState::id),
            Some(WorkspaceFrameId::new(10)),
            "single-frame layout should focus its only frame"
        );
        assert!(
            layout.frames()[0].is_focused(),
            "single-frame layout should mirror focus into the frame state"
        );
    }

    #[test]
    fn single_frame_layout_without_requested_focus_focuses_only_frame() {
        let layout = WorkspaceLayout::new(vec![frame(10, WorkspaceFrameKind::Detail)], None)
            .expect("single-frame layout should be valid");

        assert_eq!(
            layout.focused_frame_id(),
            Some(WorkspaceFrameId::new(10)),
            "non-empty layouts should preserve a focus invariant"
        );
        assert!(
            layout.frames()[0].is_focused(),
            "implicit focus should mirror into the frame state"
        );
    }

    #[test]
    fn multi_frame_focus_moves_between_existing_frames() {
        let mut layout = WorkspaceLayout::new(
            vec![
                frame(1, WorkspaceFrameKind::SourceList),
                frame(2, WorkspaceFrameKind::ContentList),
                frame(3, WorkspaceFrameKind::Detail),
            ],
            Some(WorkspaceFrameId::new(1)),
        )
        .expect("multi-frame layout should be valid");

        layout
            .focus_frame(WorkspaceFrameId::new(3))
            .expect("existing frame should be focusable");

        assert_eq!(
            layout.focused_frame().map(WorkspaceFrameState::id),
            Some(WorkspaceFrameId::new(3)),
            "focus should move to the requested frame"
        );
        assert_eq!(
            layout
                .frames()
                .iter()
                .filter(|frame| frame.is_focused())
                .count(),
            1,
            "multi-frame layout should mark exactly one focused frame"
        );
    }

    #[test]
    fn invalid_layout_mutations_return_errors() {
        let mut layout = WorkspaceLayout::new(
            vec![frame(1, WorkspaceFrameKind::SourceList)],
            Some(WorkspaceFrameId::new(1)),
        )
        .expect("initial layout should be valid");

        assert_eq!(
            layout.focus_frame(WorkspaceFrameId::new(99)),
            Err(WorkspaceModelError::FrameNotFound(WorkspaceFrameId::new(
                99
            ))),
            "focusing a missing frame should return an error"
        );
        assert_eq!(
            layout.add_frame(frame(1, WorkspaceFrameKind::Detail)),
            Err(WorkspaceModelError::DuplicateFrameId(
                WorkspaceFrameId::new(1)
            )),
            "adding a duplicate frame should return an error"
        );
        assert_eq!(
            layout.remove_frame(WorkspaceFrameId::new(99)),
            Err(WorkspaceModelError::FrameNotFound(WorkspaceFrameId::new(
                99
            ))),
            "removing a missing frame should return an error"
        );
    }

    #[test]
    fn duplicate_frames_are_rejected_at_construction() {
        assert_eq!(
            WorkspaceLayout::new(
                vec![
                    frame(1, WorkspaceFrameKind::SourceList),
                    frame(1, WorkspaceFrameKind::Detail),
                ],
                Some(WorkspaceFrameId::new(1)),
            ),
            Err(WorkspaceModelError::DuplicateFrameId(
                WorkspaceFrameId::new(1)
            )),
            "constructor should reject duplicate frame ids"
        );
    }

    #[test]
    fn removing_focused_frame_preserves_focus_when_possible() {
        let mut layout = WorkspaceLayout::new(
            vec![
                frame(1, WorkspaceFrameKind::SourceList),
                frame(2, WorkspaceFrameKind::ContentList),
                frame(3, WorkspaceFrameKind::Detail),
            ],
            Some(WorkspaceFrameId::new(2)),
        )
        .expect("multi-frame layout should be valid");

        let removed = layout
            .remove_frame(WorkspaceFrameId::new(2))
            .expect("focused frame should be removable");

        assert_eq!(
            removed.id(),
            WorkspaceFrameId::new(2),
            "remove_frame should return the removed frame"
        );
        assert_eq!(
            layout.focused_frame_id(),
            Some(WorkspaceFrameId::new(3)),
            "focus should move to the next frame after removing the focused frame"
        );
        assert_eq!(
            layout
                .frames()
                .iter()
                .filter(|frame| frame.is_focused())
                .count(),
            1,
            "layout should still mark exactly one focused frame"
        );
    }

    #[test]
    fn navigation_back_forward_boundaries_return_errors() {
        let mut nav = FrameNavigationState::new(FrameNavigationEntry::SourceList);

        assert!(
            !nav.can_go_back(),
            "new navigation state should not have back history"
        );
        assert!(
            !nav.can_go_forward(),
            "new navigation state should not have forward history"
        );
        assert_eq!(
            nav.go_back(),
            Err(WorkspaceModelError::CannotNavigateBack),
            "back at the first entry should return an error"
        );
        assert_eq!(
            nav.go_forward(),
            Err(WorkspaceModelError::CannotNavigateForward),
            "forward without forward history should return an error"
        );
    }

    #[test]
    fn navigation_push_pop_round_trip() {
        let mut nav = FrameNavigationState::new(FrameNavigationEntry::PlaylistDetail(7));
        nav.push(FrameNavigationEntry::TrackDetail(42));

        assert_eq!(
            nav.current(),
            &FrameNavigationEntry::TrackDetail(42),
            "push should update the current entry"
        );
        assert!(nav.can_go_back(), "push should create back history");
        assert!(!nav.can_go_forward(), "push should clear forward history");

        assert_eq!(
            nav.go_back().cloned(),
            Ok(FrameNavigationEntry::PlaylistDetail(7)),
            "go_back should restore the previous entry"
        );
        assert!(
            nav.can_go_forward(),
            "go_back should create forward history"
        );

        assert_eq!(
            nav.go_forward().cloned(),
            Ok(FrameNavigationEntry::TrackDetail(42)),
            "go_forward should restore the pushed entry"
        );
        assert!(
            !nav.can_go_forward(),
            "round-trip should consume forward history"
        );
    }
}
