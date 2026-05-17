//! Workspace frame identity and display models.

#![warn(clippy::pedantic)]
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "workspace contracts land before every frame action is wired"
    )
)]

use serde::{Deserialize, Serialize};

use super::nav::FrameNavigationEntry;

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
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
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

/// Detach availability for a workspace frame kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FrameDetachEligibility {
    /// The frame can request detach once window support exists.
    Detachable,
    /// The frame is anchored in the workspace and cannot detach.
    NotDetachable,
}

/// Dock lane for a workspace frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FrameDockTarget {
    /// Dock the frame into the leading workspace lane.
    Leading,
    /// Dock the frame into the center workspace lane.
    Center,
    /// Dock the frame into the trailing workspace lane.
    Trailing,
}

/// Search interpretation for the currently focused workspace frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FrameSearchScope {
    /// Filter source-list rows.
    Sidebar,
    /// Search or filter library/content rows.
    LibraryRows,
    /// Search or filter settings rows.
    SettingsRows,
    /// Filter queue rows.
    QueueRows,
    /// Refine a search-results inspector query.
    InspectorQuery,
    /// Filter track rows in an entity detail inspector.
    DetailTracks,
}

/// GPUI-free search descriptor projected from the focused workspace frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FrameSearchDescriptor {
    /// Focused frame identifier.
    pub(crate) frame_id: WorkspaceFrameId,
    /// Focused frame kind.
    pub(crate) kind: WorkspaceFrameKind,
    /// Focused frame's current navigation entry.
    pub(crate) nav: FrameNavigationEntry,
    /// Frame-local destination for submitted search text.
    pub(crate) scope: FrameSearchScope,
    /// Toolbar placeholder for the focused frame.
    pub(crate) placeholder: &'static str,
}

impl FrameDockTarget {
    /// Returns the stable lane label used in diagnostics.
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Leading => "leading",
            Self::Center => "center",
            Self::Trailing => "trailing",
        }
    }
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

    /// Returns whether frames of this kind can request detach.
    #[must_use]
    pub(crate) const fn detach_eligibility(self) -> FrameDetachEligibility {
        match self {
            Self::SourceList => FrameDetachEligibility::NotDetachable,
            Self::ContentList | Self::Detail | Self::QueueNowPlaying => {
                FrameDetachEligibility::Detachable
            }
        }
    }
}

/// Display-ready state for one workspace frame.
///
/// The frame carries only plain data. Focus is mirrored here so frame chrome can
/// render without recomputing layout state, while [`super::WorkspaceLayout`]
/// owns the invariant that at most one frame is focused.
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

    /// Updates the focus flag used by the workspace layout.
    pub(super) fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
}
