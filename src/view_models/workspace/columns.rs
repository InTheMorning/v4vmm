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
    /// (`FrameNavigationEntry::Destination`).
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

/// The operator's Music sidebar collapse choice (ADR 0084 Decision 1).
///
/// The preference lives for the session only, never in `config.toml`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum MusicSidebarPreference {
    /// Collapse under the breakpoint, else expand.
    #[default]
    Auto,
    /// Always collapsed, whatever the width.
    Collapsed,
    /// Always expanded, whatever the width.
    Expanded,
}

/// The Music sidebar's resolved show state (ADR 0084 Decision 1).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MusicSidebarState {
    /// The six destinations, their labels, their counts, and the feed area.
    Expanded,
    /// Each destination as an icon only, with a tooltip.
    Collapsed,
}

impl MusicSidebarState {
    /// Toggles a resolved state to the opposite, explicit preference.
    #[must_use]
    pub(crate) const fn toggled(self) -> MusicSidebarPreference {
        match self {
            Self::Expanded => MusicSidebarPreference::Collapsed,
            Self::Collapsed => MusicSidebarPreference::Expanded,
        }
    }
}

/// Gives the Music sidebar's resolved show state (ADR 0084 Decision 1).
///
/// `width_px` is the current workspace width. `breakpoint_px` is the named
/// token `MUSIC_SIDEBAR_COLLAPSE_BREAKPOINT`. `Auto` collapses under the
/// breakpoint; `Collapsed` and `Expanded` hold at every width, so the
/// operator can change the sidebar regardless of the window size.
#[must_use]
pub(crate) fn resolve_music_sidebar_state(
    preference: MusicSidebarPreference,
    width_px: f32,
    breakpoint_px: f32,
) -> MusicSidebarState {
    match preference {
        MusicSidebarPreference::Collapsed => MusicSidebarState::Collapsed,
        MusicSidebarPreference::Expanded => MusicSidebarState::Expanded,
        MusicSidebarPreference::Auto => {
            if width_px < breakpoint_px {
                MusicSidebarState::Collapsed
            } else {
                MusicSidebarState::Expanded
            }
        }
    }
}

/// Gives the preference after a press of the collapse button (ADR 0084
/// Decision 1).
///
/// The press moves the sidebar to the opposite of its resolved state. When
/// that state is the one that the width rule gives, the preference returns
/// to `Auto`, so the sidebar follows the width again. Otherwise the press
/// sets an explicit preference that holds at each width.
#[must_use]
pub(crate) fn toggle_music_sidebar_preference(
    preference: MusicSidebarPreference,
    width_px: f32,
    breakpoint_px: f32,
) -> MusicSidebarPreference {
    let next = resolve_music_sidebar_state(preference, width_px, breakpoint_px).toggled();
    let width_rule =
        resolve_music_sidebar_state(MusicSidebarPreference::Auto, width_px, breakpoint_px);
    if resolve_music_sidebar_state(next, width_px, breakpoint_px) == width_rule {
        MusicSidebarPreference::Auto
    } else {
        next
    }
}

#[cfg(test)]
mod tests {
    use super::{resolve_music_sidebar_state, MusicSidebarPreference, MusicSidebarState};

    /// D5-05: `Auto` collapses under the breakpoint and expands at or above
    /// it. `Collapsed` and `Expanded` hold at both sides of the breakpoint.
    #[test]
    fn adr_0084_destinations_d5_05_sidebar_state_resolves_from_preference_and_width() {
        assert_eq!(
            resolve_music_sidebar_state(MusicSidebarPreference::Auto, 900.0, 1100.0),
            MusicSidebarState::Collapsed
        );
        assert_eq!(
            resolve_music_sidebar_state(MusicSidebarPreference::Auto, 1100.0, 1100.0),
            MusicSidebarState::Expanded
        );
        assert_eq!(
            resolve_music_sidebar_state(MusicSidebarPreference::Auto, 1400.0, 1100.0),
            MusicSidebarState::Expanded
        );
        assert_eq!(
            resolve_music_sidebar_state(MusicSidebarPreference::Collapsed, 1400.0, 1100.0),
            MusicSidebarState::Collapsed
        );
        assert_eq!(
            resolve_music_sidebar_state(MusicSidebarPreference::Expanded, 900.0, 1100.0),
            MusicSidebarState::Expanded
        );
    }

    /// D5-07: the collapse button's next preference is the opposite of the
    /// resolved state, so one click always moves the sidebar.
    #[test]
    fn adr_0084_destinations_d5_07_toggled_state_gives_the_opposite_preference() {
        assert_eq!(
            MusicSidebarState::Expanded.toggled(),
            MusicSidebarPreference::Collapsed
        );
        assert_eq!(
            MusicSidebarState::Collapsed.toggled(),
            MusicSidebarPreference::Expanded
        );
    }

    /// A collapse and an expand at a wide width return the preference to
    /// `Auto`, so the sidebar collapses again under the breakpoint. The
    /// operator found on 2026-10-16 that the sidebar no longer followed the
    /// width after steps 10 to 12 of the task 005 visual check.
    #[test]
    fn adr_0084_destinations_toggle_back_to_the_width_rule_restores_auto() {
        use super::toggle_music_sidebar_preference;
        let collapsed =
            toggle_music_sidebar_preference(MusicSidebarPreference::Auto, 1400.0, 1100.0);
        assert_eq!(collapsed, MusicSidebarPreference::Collapsed);
        let restored = toggle_music_sidebar_preference(collapsed, 1400.0, 1100.0);
        assert_eq!(restored, MusicSidebarPreference::Auto);
        assert_eq!(
            resolve_music_sidebar_state(restored, 900.0, 1100.0),
            MusicSidebarState::Collapsed,
            "after the restore, a narrow width collapses the sidebar again"
        );
        let expanded_narrow =
            toggle_music_sidebar_preference(MusicSidebarPreference::Auto, 900.0, 1100.0);
        assert_eq!(
            expanded_narrow,
            MusicSidebarPreference::Expanded,
            "an expand under the breakpoint holds"
        );
    }
}
