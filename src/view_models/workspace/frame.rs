//! Workspace frame identity and display models.

#![warn(clippy::pedantic)]

use serde::{Deserialize, Serialize};

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

/// ADR 0046 Task 014 keeps detach and dock model-only: `src/ui/` and
/// `src/app.rs` must not wire a detach or dock command yet. This type, and
/// `WorkspaceFrameKind::detach_eligibility` below, have no caller outside
/// their own dedicated tests until a later task lifts that guard, so they
/// are compiled only for tests.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FrameDetachEligibility {
    /// The frame can request detach once window support exists.
    Detachable,
    /// The frame is anchored in the workspace and cannot detach.
    NotDetachable,
}

/// Dock lane for a workspace frame. See `FrameDetachEligibility` above for
/// why this is test-only.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FrameDockTarget {
    /// Dock the frame into the leading workspace lane.
    Leading,
    /// Dock the frame into the center workspace lane.
    Center,
    /// Dock the frame into the trailing workspace lane.
    Trailing,
}

#[cfg(test)]
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

#[cfg(test)]
impl WorkspaceFrameKind {
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

    /// Updates the focus flag used by the workspace layout.
    pub(super) fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
}

/// `with_subtitle` and `with_status` have no caller outside their own
/// dedicated tests: no frame is built with a subtitle or a status line
/// today. They stay here, compiled only for tests.
#[cfg(test)]
impl WorkspaceFrameState {
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
}
