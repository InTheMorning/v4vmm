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

/// Content source filter for a frame-local content surface.
///
/// Invalid filter states are unrepresentable. Renderers map these variants to
/// frame-local controls, while workspace view-models pass the selected value
/// through without importing UI framework types.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ContentFilter {
    /// Show content from every available source.
    #[default]
    All,
    /// Show content already present in the local library.
    Library,
    /// Show content available from the remote index.
    Index,
}

impl ContentFilter {
    /// Returns the visible label for this filter.
    #[must_use]
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Library => "Library",
            Self::Index => "Index",
        }
    }
}

/// Display contract for one frame-local filter chip option.
///
/// The value carries the typed filter command. Labels stay static so callers
/// can reuse the standard All / Library / Index set without allocating or
/// hand-rolling duplicate option lists.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FilterChipOption {
    /// Filter selected when this option is activated.
    pub(crate) value: ContentFilter,
    /// Visible chip label.
    pub(crate) label: &'static str,
    /// Accessibility label for assistive technologies and tooltips.
    pub(crate) a11y_label: &'static str,
    /// Whether this option should render unavailable.
    pub(crate) disabled: bool,
}

/// Display contract for a frame-local filter chip strip.
///
/// The display carries all data needed by frame chrome to render either an
/// expanded chip strip or a narrow pull-down. It contains no GPUI values and
/// leaves command dispatch to the renderer that consumes the selected
/// [`ContentFilter`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FilterChipStripDisplay {
    /// Stable element identifier for the filter strip.
    pub(crate) id: String,
    /// Ordered filter options.
    pub(crate) options: Vec<FilterChipOption>,
    /// Currently selected filter.
    pub(crate) selected: ContentFilter,
    /// Whether narrow frame chrome should collapse chips into a pull-down.
    pub(crate) narrow_collapse_to_pulldown: bool,
}

impl FilterChipStripDisplay {
    /// Creates the default content-list filter strip display.
    #[must_use]
    pub(crate) fn default_for_content_list(
        selected: ContentFilter,
        narrow_collapse_to_pulldown: bool,
    ) -> Self {
        Self::with_standard_options(
            "workspace-content-list-filter",
            selected,
            narrow_collapse_to_pulldown,
        )
    }

    /// Creates the default search-inspector filter strip display.
    #[must_use]
    pub(crate) fn default_for_search_inspector(
        selected: ContentFilter,
        narrow_collapse_to_pulldown: bool,
    ) -> Self {
        Self::with_standard_options(
            "workspace-search-inspector-filter",
            selected,
            narrow_collapse_to_pulldown,
        )
    }

    fn with_standard_options(
        id: impl Into<String>,
        selected: ContentFilter,
        narrow_collapse_to_pulldown: bool,
    ) -> Self {
        Self {
            id: id.into(),
            options: Self::standard_options(),
            selected,
            narrow_collapse_to_pulldown,
        }
    }

    fn standard_options() -> Vec<FilterChipOption> {
        vec![
            FilterChipOption {
                value: ContentFilter::All,
                label: "All",
                a11y_label: "Show library and index content",
                disabled: false,
            },
            FilterChipOption {
                value: ContentFilter::Library,
                label: "Library",
                a11y_label: "Show library content only",
                disabled: false,
            },
            FilterChipOption {
                value: ContentFilter::Index,
                label: "Index",
                a11y_label: "Show index content only",
                disabled: false,
            },
        ]
    }
}

/// Display contract for one frame-chrome button.
///
/// The shell renderer owns visual treatment and icon selection. This contract
/// only supplies a stable command id, accessibility label, and availability
/// state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FrameChromeButtonDisplay {
    /// Stable element or command identifier for the button.
    pub(crate) id: String,
    /// Accessibility label for assistive technologies and tooltips.
    pub(crate) a11y_label: &'static str,
    /// Whether the command should render unavailable.
    pub(crate) disabled: bool,
}

impl FrameChromeButtonDisplay {
    /// Creates a display contract for a frame-chrome button.
    #[must_use]
    pub(crate) fn new(id: impl Into<String>, a11y_label: &'static str, disabled: bool) -> Self {
        Self {
            id: id.into(),
            a11y_label,
            disabled,
        }
    }
}

/// Display contract for one frame-chrome menu item.
///
/// Menu items use semantic ids and static labels. The shared frame shell owns
/// rendering details, including any icon catalog mapping.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FrameChromeMenuItemDisplay {
    /// Stable element or command identifier for the menu item.
    pub(crate) id: String,
    /// Visible menu label.
    pub(crate) label: &'static str,
    /// Accessibility label for the menu item.
    pub(crate) a11y_label: &'static str,
    /// Whether the command should render unavailable.
    pub(crate) disabled: bool,
}

/// Display contract consumed by the shared workspace frame shell.
///
/// This projection keeps frame chrome data in a GPUI-free view model. Screens
/// render content into the named slot while the shared shell renders title,
/// navigation, close, and action-menu affordances consistently.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FrameShellDisplay {
    /// Frame identifier represented by this shell.
    pub(crate) frame_id: WorkspaceFrameId,
    /// Primary frame title.
    pub(crate) title: String,
    /// Optional secondary frame context.
    pub(crate) subtitle: Option<String>,
    /// Optional trailing status text.
    pub(crate) status: Option<String>,
    /// Back navigation command display.
    pub(crate) back: FrameChromeButtonDisplay,
    /// Forward navigation command display.
    pub(crate) forward: FrameChromeButtonDisplay,
    /// Optional close command display.
    pub(crate) close: Option<FrameChromeButtonDisplay>,
    /// Additional frame action menu items.
    pub(crate) action_menu_items: Vec<FrameChromeMenuItemDisplay>,
    /// Stable content slot identifier for mounting frame body content.
    pub(crate) content_slot_id: String,
}

impl FrameShellDisplay {
    const BACK_A11Y_LABEL: &'static str = "Navigate back in frame";
    const FORWARD_A11Y_LABEL: &'static str = "Navigate forward in frame";
    const CLOSE_A11Y_LABEL: &'static str = "Close frame";

    /// Projects frame state and navigation into shell chrome display data.
    #[must_use]
    pub(crate) fn from_frame(
        frame: &WorkspaceFrameState,
        nav: &FrameNavigationState,
        allow_close: bool,
    ) -> Self {
        let frame_id = frame.id();
        Self {
            frame_id,
            title: frame.title().to_string(),
            subtitle: frame.subtitle().map(ToOwned::to_owned),
            status: frame.status().map(ToOwned::to_owned),
            back: FrameChromeButtonDisplay::new(
                format!("workspace-frame-{}-back", frame_id.value()),
                Self::BACK_A11Y_LABEL,
                !nav.can_go_back(),
            ),
            forward: FrameChromeButtonDisplay::new(
                format!("workspace-frame-{}-forward", frame_id.value()),
                Self::FORWARD_A11Y_LABEL,
                !nav.can_go_forward(),
            ),
            close: allow_close.then(|| {
                FrameChromeButtonDisplay::new(
                    format!("workspace-frame-{}-close", frame_id.value()),
                    Self::CLOSE_A11Y_LABEL,
                    false,
                )
            }),
            action_menu_items: Vec::new(),
            content_slot_id: format!("workspace-frame-{}-content", frame_id.value()),
        }
    }
}

/// Breadcrumb path truncation policy.
///
/// Breadcrumb renderers keep the origin and current segment visible when space
/// is constrained, collapsing the middle of longer paths.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BreadcrumbTruncation {
    /// Collapse middle segments when a renderer needs to abbreviate the path.
    MiddleEllipsis,
}

/// Display contract for one breadcrumb segment.
///
/// Current segments are labels only. Non-current segments carry the typed frame
/// destination so renderers can dispatch navigation without parsing strings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BreadcrumbSegment {
    /// Stable segment identifier.
    pub(crate) id: String,
    /// Visible segment label.
    pub(crate) label: String,
    /// Accessibility label for the segment.
    pub(crate) a11y_label: String,
    /// Whether this segment represents the current frame destination.
    pub(crate) is_current: bool,
    /// Target selected when activating a non-current segment.
    pub(crate) target: Option<FrameNavigationEntry>,
}

/// Display contract for a frame breadcrumb path.
///
/// The display is GPUI-free and projects from [`FrameNavigationState`]. Shells
/// or transitional surfaces can consume it without owning navigation policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BreadcrumbDisplay {
    /// Stable breadcrumb element identifier.
    pub(crate) id: String,
    /// Ordered breadcrumb segments from origin through current destination.
    pub(crate) segments: Vec<BreadcrumbSegment>,
    /// Renderer truncation guidance.
    pub(crate) truncation: BreadcrumbTruncation,
}

impl BreadcrumbDisplay {
    /// Projects a frame navigation state into display-ready breadcrumbs.
    #[must_use]
    pub(crate) fn project(
        id: impl Into<String>,
        nav: &FrameNavigationState,
        mut label_for: impl FnMut(&FrameNavigationEntry) -> String,
    ) -> Self {
        let mut entries: Vec<&FrameNavigationEntry> = nav.back_stack.iter().collect();
        entries.push(&nav.current);
        let last_index = entries.len().saturating_sub(1);
        let segments = entries
            .into_iter()
            .enumerate()
            .map(|(index, entry)| {
                let label = label_for(entry);
                let is_current = index == last_index;
                BreadcrumbSegment {
                    id: breadcrumb_entry_id(entry),
                    a11y_label: if is_current {
                        format!("Current location: {label}")
                    } else {
                        format!("Go to {label}")
                    },
                    label,
                    is_current,
                    target: (!is_current).then(|| entry.clone()),
                }
            })
            .collect();

        Self {
            id: id.into(),
            segments,
            truncation: BreadcrumbTruncation::MiddleEllipsis,
        }
    }
}

fn breadcrumb_entry_id(entry: &FrameNavigationEntry) -> String {
    match entry {
        FrameNavigationEntry::SourceList => "source-list".to_string(),
        FrameNavigationEntry::PlaylistDetail(id) => format!("playlist-{id}"),
        FrameNavigationEntry::TrackDetail(id) => format!("track-{id}"),
        FrameNavigationEntry::AlbumDetail(id) => format!("album-{id}"),
        FrameNavigationEntry::ArtistDetail(name) => format!("artist-{}", slug_id(name)),
        FrameNavigationEntry::Search(query) => format!("search-{}", slug_id(query)),
        FrameNavigationEntry::QueueNowPlaying => "queue-now-playing".to_string(),
    }
}

fn slug_id(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "root".to_string()
    } else {
        trimmed.to_string()
    }
}

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

    /// Returns the default primary content frame identifier.
    #[must_use]
    pub(crate) const fn default_content_frame_id() -> WorkspaceFrameId {
        Self::CONTENT_LIST_ID
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

    /// Returns the destination that a back action would select.
    #[must_use]
    pub(crate) fn back_destination(&self) -> Option<&FrameNavigationEntry> {
        self.back_stack.last()
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
        BreadcrumbDisplay, BreadcrumbTruncation, ContentFilter, FilterChipStripDisplay,
        FrameNavigationEntry, FrameNavigationState, FrameShellDisplay, WorkspaceFrameId,
        WorkspaceFrameKind, WorkspaceFrameState, WorkspaceLayout, WorkspaceModelError,
    };

    fn frame(id: u64, kind: WorkspaceFrameKind) -> WorkspaceFrameState {
        WorkspaceFrameState::with_default_title(WorkspaceFrameId::new(id), kind)
    }

    #[test]
    fn filter_chip_strip_defaults_use_standard_option_order() {
        let content_list =
            FilterChipStripDisplay::default_for_content_list(ContentFilter::All, true);
        let search_inspector =
            FilterChipStripDisplay::default_for_search_inspector(ContentFilter::All, true);

        let content_values: Vec<_> = content_list
            .options
            .iter()
            .map(|option| option.value)
            .collect();
        let search_values: Vec<_> = search_inspector
            .options
            .iter()
            .map(|option| option.value)
            .collect();

        assert_eq!(
            content_values,
            [
                ContentFilter::All,
                ContentFilter::Library,
                ContentFilter::Index
            ],
            "content-list filters should keep the ADR 0047 option order"
        );
        assert_eq!(
            search_values,
            [
                ContentFilter::All,
                ContentFilter::Library,
                ContentFilter::Index
            ],
            "search-inspector filters should keep the ADR 0047 option order"
        );
    }

    #[test]
    fn filter_chip_strip_defaults_round_trip_selected_filter() {
        let content_list =
            FilterChipStripDisplay::default_for_content_list(ContentFilter::Library, true);
        let search_inspector =
            FilterChipStripDisplay::default_for_search_inspector(ContentFilter::Index, true);

        assert_eq!(
            content_list.selected,
            ContentFilter::Library,
            "content-list filter display should preserve the selected filter"
        );
        assert_eq!(
            search_inspector.selected,
            ContentFilter::Index,
            "search-inspector filter display should preserve the selected filter"
        );
    }

    #[test]
    fn filter_chip_strip_defaults_pass_through_narrow_collapse() {
        let expanded = FilterChipStripDisplay::default_for_content_list(ContentFilter::All, false);
        let collapsed =
            FilterChipStripDisplay::default_for_search_inspector(ContentFilter::All, true);

        assert!(
            !expanded.narrow_collapse_to_pulldown,
            "content-list filter display should preserve expanded narrow-mode preference"
        );
        assert!(
            collapsed.narrow_collapse_to_pulldown,
            "search-inspector filter display should preserve collapsed narrow-mode preference"
        );
    }

    #[test]
    fn frame_shell_display_disables_empty_history_navigation() {
        let frame = frame(7, WorkspaceFrameKind::Detail);
        let nav = FrameNavigationState::new(FrameNavigationEntry::TrackDetail(42));

        let display = FrameShellDisplay::from_frame(&frame, &nav, true);

        assert!(
            display.back.disabled,
            "empty back history should disable frame Back"
        );
        assert!(
            display.forward.disabled,
            "empty forward history should disable frame Forward"
        );
        assert_eq!(
            display.back.id, "workspace-frame-7-back",
            "back id should be stable and frame-scoped"
        );
        assert_eq!(
            display.forward.id, "workspace-frame-7-forward",
            "forward id should be stable and frame-scoped"
        );
    }

    #[test]
    fn frame_shell_display_uses_history_for_navigation_availability() {
        let frame = frame(7, WorkspaceFrameKind::Detail);
        let mut nav = FrameNavigationState::new(FrameNavigationEntry::PlaylistDetail(1));

        nav.push(FrameNavigationEntry::TrackDetail(42));
        let with_back = FrameShellDisplay::from_frame(&frame, &nav, true);

        assert!(
            !with_back.back.disabled,
            "back history should enable frame Back"
        );
        assert!(
            with_back.forward.disabled,
            "pushing a new entry should leave frame Forward disabled"
        );

        nav.go_back()
            .expect("pushed navigation should allow going back");
        let with_forward = FrameShellDisplay::from_frame(&frame, &nav, true);

        assert!(
            with_forward.back.disabled,
            "after returning to the first entry, frame Back should be disabled"
        );
        assert!(
            !with_forward.forward.disabled,
            "back navigation should enable frame Forward"
        );
    }

    #[test]
    fn frame_shell_display_hides_close_when_not_allowed() {
        let frame = frame(7, WorkspaceFrameKind::Detail);
        let nav = FrameNavigationState::new(FrameNavigationEntry::TrackDetail(42));

        let fixed_frame = FrameShellDisplay::from_frame(&frame, &nav, false);
        let closable_frame = FrameShellDisplay::from_frame(&frame, &nav, true);

        assert_eq!(
            fixed_frame.close, None,
            "non-closable frames should not expose a close command"
        );
        assert_eq!(
            closable_frame.close.map(|close| close.id),
            Some("workspace-frame-7-close".to_string()),
            "closable frames should expose a frame-scoped close command"
        );
    }

    #[test]
    fn frame_shell_display_passes_through_header_text_and_slot_id() {
        let frame = WorkspaceFrameState::new(
            WorkspaceFrameId::new(7),
            WorkspaceFrameKind::ContentList,
            "Playlist",
        )
        .with_subtitle("Seven tracks")
        .with_status("Ready");
        let nav = FrameNavigationState::new(FrameNavigationEntry::PlaylistDetail(1));

        let display = FrameShellDisplay::from_frame(&frame, &nav, true);

        assert_eq!(display.frame_id, WorkspaceFrameId::new(7));
        assert_eq!(display.title, "Playlist");
        assert_eq!(
            display.subtitle,
            Some("Seven tracks".to_string()),
            "subtitle should pass through from frame state"
        );
        assert_eq!(
            display.status,
            Some("Ready".to_string()),
            "status should pass through from frame state"
        );
        assert_eq!(
            display.content_slot_id, "workspace-frame-7-content",
            "content slot id should be stable and frame-scoped"
        );
        assert!(
            display.action_menu_items.is_empty(),
            "Task 005 defines the menu item contract without adding default actions"
        );
    }

    #[test]
    fn breadcrumb_display_projects_navigation_path() {
        let mut nav = FrameNavigationState::new(FrameNavigationEntry::PlaylistDetail(7));
        nav.push(FrameNavigationEntry::TrackDetail(42));

        let display =
            BreadcrumbDisplay::project("library-track-breadcrumb", &nav, |entry| match entry {
                FrameNavigationEntry::PlaylistDetail(_) => "My Playlist".to_string(),
                FrameNavigationEntry::TrackDetail(_) => "Lantern Tide".to_string(),
                _ => "Library".to_string(),
            });

        assert_eq!(display.id, "library-track-breadcrumb");
        assert_eq!(display.truncation, BreadcrumbTruncation::MiddleEllipsis);
        assert_eq!(display.segments.len(), 2);
        assert_eq!(display.segments[0].label, "My Playlist");
        assert_eq!(
            display.segments[0].target,
            Some(FrameNavigationEntry::PlaylistDetail(7))
        );
        assert!(
            !display.segments[0].is_current,
            "playlist segment should be a selectable parent"
        );
        assert_eq!(display.segments[1].label, "Lantern Tide");
        assert_eq!(display.segments[1].target, None);
        assert!(
            display.segments[1].is_current,
            "track segment should be the current location"
        );
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

    #[test]
    fn navigation_push_current_entry_is_noop() {
        let mut nav = FrameNavigationState::new(FrameNavigationEntry::PlaylistDetail(7));

        nav.push(FrameNavigationEntry::PlaylistDetail(7));

        assert_eq!(
            nav.current(),
            &FrameNavigationEntry::PlaylistDetail(7),
            "same-entry push should preserve the current destination"
        );
        assert!(
            !nav.can_go_back(),
            "same-entry push should not create synthetic back history"
        );
        assert!(
            !nav.can_go_forward(),
            "same-entry push should not create synthetic forward history"
        );
    }

    #[test]
    fn navigation_back_destination_peeks_without_mutating() {
        let mut nav = FrameNavigationState::new(FrameNavigationEntry::PlaylistDetail(7));

        assert_eq!(
            nav.back_destination(),
            None,
            "initial navigation should not have a back destination"
        );

        nav.push(FrameNavigationEntry::TrackDetail(42));

        assert_eq!(
            nav.back_destination(),
            Some(&FrameNavigationEntry::PlaylistDetail(7)),
            "back destination should expose the previous entry"
        );
        assert_eq!(
            nav.current(),
            &FrameNavigationEntry::TrackDetail(42),
            "peeking should not change current navigation"
        );
    }

    #[test]
    fn navigation_reset_clears_back_and_forward_history() {
        let mut nav = FrameNavigationState::new(FrameNavigationEntry::PlaylistDetail(7));
        nav.push(FrameNavigationEntry::TrackDetail(42));
        nav.go_back()
            .expect("pushed navigation should allow going back");

        nav.reset(FrameNavigationEntry::TrackDetail(99));

        assert_eq!(
            nav.current(),
            &FrameNavigationEntry::TrackDetail(99),
            "reset should replace the current entry"
        );
        assert!(!nav.can_go_back(), "reset should clear back history");
        assert!(!nav.can_go_forward(), "reset should clear forward history");
    }
}
