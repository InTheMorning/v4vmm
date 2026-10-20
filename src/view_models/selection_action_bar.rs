//! The action bar of a Releases or Tracks list selection (ADR 0084
//! Decision 10, amended 2026-10-19 for the selection mode).
//!
//! The bar shows only in [`SelectionMode::Selecting`].
//! [`SelectionActionBarVm::new`] gives `None` outside that mode, so a
//! caller renders nothing. In `Selecting`, with no selected row, the bar
//! gives "Select items" and no action. Each action is typed: an action
//! that cannot act is absent from [`SelectionActionBarVm::actions`],
//! never present and disabled.

#![warn(clippy::pedantic)]

use crate::view_models::format::plural;
use crate::view_models::list_selection::SelectionMode;

/// The ellipsis that marks an action that opens a further dialog (HIG
/// convention, also used by the Library's single-item removal actions).
const OPENS_DIALOG_MARK: char = '\u{2026}';

/// One action of the selection action bar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SelectionActionKind {
    Download,
    AddToPlaylist,
    Remove,
    /// "Remove n from playlist" of the playlist page (packet 025). The
    /// tracks stay in the Library.
    RemoveFromPlaylist,
    /// "Move to top" of the playlist page (packet 025).
    MoveToTop,
    /// "Move to bottom" of the playlist page (packet 025).
    MoveToBottom,
    Clear,
}

/// One typed action of the selection action bar: a label, an
/// accessibility label and the kind the screen dispatches on.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SelectionActionVm {
    pub(crate) kind: SelectionActionKind,
    pub(crate) element_id: &'static str,
    pub(crate) label: String,
    pub(crate) a11y_label: String,
}

/// The counts `SelectionActionBarVm::new` needs. A caller gathers these
/// from the selected keys and the already-loaded Library tree; this view
/// model reads no service and no database.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SelectionActionBarInput {
    /// The total number of selected items.
    pub(crate) selected_count: usize,
    /// The singular noun of one selected item, for example "track" or
    /// "release".
    pub(crate) item_noun: &'static str,
    /// The number of selected items that are not in the Library. Only
    /// these download.
    pub(crate) not_in_library_count: usize,
    /// The number of selected items that are in the Library. Only these
    /// remove.
    pub(crate) in_library_count: usize,
}

/// The count of each action of the bar (ADR 0084 packet 018). A caller
/// gathers these from the selected rows. The detail column gives a
/// separate count to each action: for example, "Add to playlist" counts
/// only the selected tracks that are in the Library.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SelectionActionCounts {
    /// The total number of selected items.
    pub(crate) selected_count: usize,
    /// The singular noun of one selected item, for example "track".
    pub(crate) item_noun: &'static str,
    /// The number of selected items that "Download" acts on.
    pub(crate) download_count: usize,
    /// The number of selected items that "Add to playlist" acts on.
    pub(crate) add_count: usize,
    /// The number of selected items that "Remove" acts on.
    pub(crate) remove_count: usize,
}

/// The counts and the edge facts of the playlist page bar (ADR 0084
/// Decision 10, packet 025). A caller gathers these from the selected
/// entries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PlaylistSelectionCounts {
    /// The number of selected entries. One track can have two entries.
    pub(crate) selected_count: usize,
    /// The number of distinct selected tracks that are not downloaded.
    pub(crate) download_count: usize,
    /// The number of distinct selected tracks that "Remove from
    /// Library…" acts on.
    pub(crate) remove_count: usize,
    /// The selected entries are the first entries of the playlist.
    pub(crate) at_top: bool,
    /// The selected entries are the last entries of the playlist.
    pub(crate) at_bottom: bool,
}

/// The action bar at the bottom of a list with a selection (ADR 0084
/// Decision 10).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SelectionActionBarVm {
    /// "7 tracks selected", "3 releases selected", "1 release selected".
    pub(crate) count_label: String,
    /// Each available action, in display order. "Clear" is always last.
    pub(crate) actions: Vec<SelectionActionVm>,
}

impl SelectionActionBarVm {
    /// Builds the bar, or gives `None` outside [`SelectionMode::Selecting`]
    /// (ADR 0084 Decision 10 Amendment, 2026-10-19). In `Selecting`, with
    /// no selected row, the bar reads "Select items" and carries no
    /// action. "Add to playlist" counts every selected item.
    #[must_use]
    pub(crate) fn new(mode: SelectionMode, input: SelectionActionBarInput) -> Option<Self> {
        Self::from_counts(
            mode,
            SelectionActionCounts {
                selected_count: input.selected_count,
                item_noun: input.item_noun,
                download_count: input.not_in_library_count,
                add_count: input.selected_count,
                remove_count: input.in_library_count,
            },
        )
    }

    /// Builds the bar from a separate count for each action (ADR 0084
    /// packet 018: the detail column). An action with a count of 0 is
    /// absent. "Clear" shows with each selection.
    #[must_use]
    pub(crate) fn from_counts(mode: SelectionMode, counts: SelectionActionCounts) -> Option<Self> {
        if !mode.is_selecting() {
            return None;
        }
        if counts.selected_count == 0 {
            return Some(Self {
                count_label: "Select items".to_owned(),
                actions: Vec::new(),
            });
        }
        let noun_for = |count: usize| format!("{}{}", counts.item_noun, plural(count));
        let mut actions = Vec::new();
        if counts.download_count > 0 {
            let label = format!(
                "Download {} {}",
                counts.download_count,
                noun_for(counts.download_count)
            );
            actions.push(SelectionActionVm {
                kind: SelectionActionKind::Download,
                element_id: "music-selection-download",
                a11y_label: label.clone(),
                label,
            });
        }
        if counts.add_count > 0 {
            let label = format!(
                "Add {} {} to playlist",
                counts.add_count,
                noun_for(counts.add_count)
            );
            actions.push(SelectionActionVm {
                kind: SelectionActionKind::AddToPlaylist,
                element_id: "music-selection-add-to-playlist",
                a11y_label: label.clone(),
                label,
            });
        }
        if counts.remove_count > 0 {
            let label = format!(
                "Remove {} {}{OPENS_DIALOG_MARK}",
                counts.remove_count,
                noun_for(counts.remove_count)
            );
            actions.push(SelectionActionVm {
                kind: SelectionActionKind::Remove,
                element_id: "music-selection-remove",
                a11y_label: label.clone(),
                label,
            });
        }
        actions.push(SelectionActionVm {
            kind: SelectionActionKind::Clear,
            element_id: "music-selection-clear",
            a11y_label: "Clear the selection".to_owned(),
            label: "Clear".to_owned(),
        });
        Some(Self {
            count_label: format!(
                "{} {} selected",
                counts.selected_count,
                noun_for(counts.selected_count)
            ),
            actions,
        })
    }

    /// Builds the bar of the playlist page (ADR 0084 Decision 10, packet
    /// 025), or gives `None` outside [`SelectionMode::Selecting`]. The
    /// order is "Remove n from playlist", "Download n tracks", "Add n to
    /// playlist", "Move to top", "Move to bottom", "Remove from
    /// Library…" and "Clear". An action that cannot act is absent.
    #[must_use]
    pub(crate) fn for_playlist(
        mode: SelectionMode,
        counts: PlaylistSelectionCounts,
    ) -> Option<Self> {
        if !mode.is_selecting() {
            return None;
        }
        let selected = counts.selected_count;
        if selected == 0 {
            return Some(Self {
                count_label: "Select items".to_owned(),
                actions: Vec::new(),
            });
        }
        let tracks = |count: usize| format!("{count} track{}", plural(count));
        let action = |kind, element_id, label: String, a11y_label: String| SelectionActionVm {
            kind,
            element_id,
            label,
            a11y_label,
        };
        let mut actions = vec![action(
            SelectionActionKind::RemoveFromPlaylist,
            "music-selection-remove-from-playlist",
            format!("Remove {selected} from playlist"),
            format!("Remove {} from playlist", tracks(selected)),
        )];
        if counts.download_count > 0 {
            let label = format!("Download {}", tracks(counts.download_count));
            actions.push(action(
                SelectionActionKind::Download,
                "music-selection-download",
                label.clone(),
                label,
            ));
        }
        actions.push(action(
            SelectionActionKind::AddToPlaylist,
            "music-selection-add-to-playlist",
            format!("Add {selected} to playlist"),
            format!("Add {} to playlist", tracks(selected)),
        ));
        if !counts.at_top {
            actions.push(action(
                SelectionActionKind::MoveToTop,
                "music-selection-move-to-top",
                "Move to top".to_owned(),
                format!("Move {} to top", tracks(selected)),
            ));
        }
        if !counts.at_bottom {
            actions.push(action(
                SelectionActionKind::MoveToBottom,
                "music-selection-move-to-bottom",
                "Move to bottom".to_owned(),
                format!("Move {} to bottom", tracks(selected)),
            ));
        }
        if counts.remove_count > 0 {
            actions.push(action(
                SelectionActionKind::Remove,
                "music-selection-remove",
                format!("Remove from Library{OPENS_DIALOG_MARK}"),
                format!(
                    "Remove {} from Library{OPENS_DIALOG_MARK}",
                    tracks(counts.remove_count)
                ),
            ));
        }
        actions.push(action(
            SelectionActionKind::Clear,
            "music-selection-clear",
            "Clear".to_owned(),
            "Clear the selection".to_owned(),
        ));
        Some(Self {
            count_label: format!("{} selected", tracks(selected)),
            actions,
        })
    }

    /// The action of `kind`, when the bar carries one.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn action(&self, kind: SelectionActionKind) -> Option<&SelectionActionVm> {
        self.actions.iter().find(|action| action.kind == kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R9-06: `SelectionActionBarVm` gives the count text, and the four
    /// actions with their labels. "Download" counts only the items not
    /// in the Library.
    #[test]
    fn adr_0084_selection_r9_06_action_bar_gives_count_and_labels() {
        let bar = SelectionActionBarVm::new(
            SelectionMode::Selecting,
            SelectionActionBarInput {
                selected_count: 7,
                item_noun: "track",
                not_in_library_count: 5,
                in_library_count: 2,
            },
        )
        .expect("a selection of 7 gives a bar");

        assert_eq!(bar.count_label, "7 tracks selected");
        assert_eq!(
            bar.action(SelectionActionKind::Download)
                .expect("some tracks are not in the Library")
                .label,
            "Download 5 tracks"
        );
        assert_eq!(
            bar.action(SelectionActionKind::AddToPlaylist)
                .expect("always present with a selection")
                .label,
            "Add 7 tracks to playlist"
        );
        assert_eq!(
            bar.action(SelectionActionKind::Remove)
                .expect("some tracks are in the Library")
                .label,
            "Remove 2 tracks\u{2026}"
        );
        assert_eq!(
            bar.action(SelectionActionKind::Clear)
                .expect("always present with a selection")
                .label,
            "Clear"
        );
    }

    /// R9-06: a single release gives the singular count text, and
    /// "Download" is absent when each selected item is in the Library.
    #[test]
    fn adr_0084_selection_r9_06_download_absent_when_all_in_library() {
        let bar = SelectionActionBarVm::new(
            SelectionMode::Selecting,
            SelectionActionBarInput {
                selected_count: 1,
                item_noun: "release",
                not_in_library_count: 0,
                in_library_count: 1,
            },
        )
        .expect("a selection of 1 gives a bar");

        assert_eq!(bar.count_label, "1 release selected");
        assert!(bar.action(SelectionActionKind::Download).is_none());
        assert!(bar.action(SelectionActionKind::Remove).is_some());
    }

    /// R9-07: "Remove" is absent when no selected item is in the
    /// Library.
    #[test]
    fn adr_0084_selection_r9_07_remove_absent_when_none_in_library() {
        let bar = SelectionActionBarVm::new(
            SelectionMode::Selecting,
            SelectionActionBarInput {
                selected_count: 3,
                item_noun: "track",
                not_in_library_count: 3,
                in_library_count: 0,
            },
        )
        .expect("a selection of 3 gives a bar");

        assert!(bar.action(SelectionActionKind::Remove).is_none());
        assert!(bar.action(SelectionActionKind::Download).is_some());
    }

    /// ADR 0084 Decision 10 Amendment, 2026-10-19: the bar is absent
    /// outside `Selecting`, whatever the selected count. This replaces
    /// packet 009's "the bar shows only while a selection exists".
    #[test]
    fn adr_0084_selection_mode_r19_01_no_bar_outside_selecting_mode() {
        let bar = SelectionActionBarVm::new(
            SelectionMode::Normal,
            SelectionActionBarInput {
                selected_count: 0,
                item_noun: "track",
                not_in_library_count: 0,
                in_library_count: 0,
            },
        );
        assert!(bar.is_none());
    }

    /// R19-06: in `Selecting` with no selected row, the bar gives "Select
    /// items" and no action.
    #[test]
    fn adr_0084_selection_mode_r19_06_select_items_with_no_selected_row() {
        let bar = SelectionActionBarVm::new(
            SelectionMode::Selecting,
            SelectionActionBarInput {
                selected_count: 0,
                item_noun: "track",
                not_in_library_count: 0,
                in_library_count: 0,
            },
        )
        .expect("Selecting shows a bar even with no selected row");

        assert_eq!(bar.count_label, "Select items");
        assert!(bar.actions.is_empty());
    }
}
