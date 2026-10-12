//! Music column visibility (ADR 0084 Decision 1).
//!
//! Music shows three columns: the sidebar, the list and the detail. At a
//! narrow width, one of the list and the detail columns gives way to the
//! other. This module holds that rule as a pure function. It carries no
//! renderer type. The caller supplies the workspace width and the
//! single-column breakpoint, both in pixels, so this module does not import
//! `gpui`.

#![warn(clippy::pedantic)]

/// The Music columns visible for one workspace width (ADR 0084 Decision 1).
///
/// The sidebar always shows. This enum names only how the list and the
/// detail columns divide the rest of the width.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MusicColumns {
    /// The sidebar, the list and the detail columns all show.
    Three,
    /// A narrow width with no open page: the sidebar and the list show.
    SidebarList,
    /// A narrow width with an open page: the sidebar and the detail show.
    SidebarDetail,
}

impl MusicColumns {
    /// Gives the visible Music columns for a workspace width.
    ///
    /// `width_px` is the current workspace width. `breakpoint_px` is the
    /// single-column breakpoint (the named token
    /// `MUSIC_SINGLE_COLUMN_BREAKPOINT`). `page_open` is `true` when the
    /// location's current entry is a page, not a destination root
    /// (`FrameNavigationEntry::SourceList`).
    #[must_use]
    pub(crate) fn for_width(width_px: f32, breakpoint_px: f32, page_open: bool) -> Self {
        if width_px >= breakpoint_px {
            Self::Three
        } else if page_open {
            Self::SidebarDetail
        } else {
            Self::SidebarList
        }
    }

    /// Returns whether the list column shows.
    #[must_use]
    pub(crate) const fn shows_list(self) -> bool {
        matches!(self, Self::Three | Self::SidebarList)
    }

    /// Returns whether the detail column shows.
    #[must_use]
    pub(crate) const fn shows_detail(self) -> bool {
        matches!(self, Self::Three | Self::SidebarDetail)
    }
}
