//! The actions of one release row (ADR 0083 Decisions 5 and 6, ADR 0084
//! Decision 10, task 031).
//!
//! A row of the Releases list, a release row of a feed page and a release
//! row of a name page get their "⋯" menu and their right-click menu from
//! the one function [`release_row_menu`]. So the three lists cannot show
//! different actions for the same release.
//!
//! [`ReleaseRowAvailability::of`] gives the availability of each action.
//! The menu and the counts of the selection action bar both use it, so a
//! menu item and a bar action agree for a selection of one release.
//!
//! This module holds no renderer type. The menu composite of task 006
//! renders the result through `row_menu_parts`.

#![warn(clippy::pedantic)]

use crate::view_models::album_page::AlbumPageAction;
use crate::view_models::detail_selection::LibraryReleaseFacts;
use crate::view_models::release_kind::ReleaseKind;
use crate::view_models::release_page::{
    TrackRowMenuAction, TrackRowMenuItemVm, TrackRowMenuVm, TrackRowPlaylistLevelVm,
};
use crate::view_models::workspace::ReleaseKey;

/// The tooltip of the "⋯" button of a release row.
const RELEASE_MENU_TRIGGER_TOOLTIP: &str = "More actions";

/// The labels of the "Add to playlist" level of a release row menu.
const RELEASE_ROW_PLAYLIST_LEVEL: TrackRowPlaylistLevelVm = TrackRowPlaylistLevelVm {
    back_text: "Back",
    back_a11y_label: "Back to release actions",
    new_playlist_a11y_label: "Create a new playlist",
    back_to_choices_a11y_label: "Back to playlist choices",
    create_a11y_label: "Create playlist and add release",
};

/// The Library state of one release row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReleaseRowState {
    /// The release is only in the Index. `job_active` is true while a job
    /// of its key waits or runs.
    Index { job_active: bool },
    /// The release is in the Library.
    Library(LibraryReleaseFacts),
}

impl ReleaseRowState {
    /// The state of a release with the Library facts `library`. `None`
    /// gives an Index release with `index_job_active`.
    #[must_use]
    pub(crate) const fn of(library: Option<LibraryReleaseFacts>, index_job_active: bool) -> Self {
        match library {
            Some(facts) => Self::Library(facts),
            None => Self::Index {
                job_active: index_job_active,
            },
        }
    }

    /// The Library feed id. `None` for an Index release.
    #[must_use]
    pub(crate) const fn feed_id(self) -> Option<i64> {
        match self {
            Self::Library(facts) => Some(facts.feed_id),
            Self::Index { .. } => None,
        }
    }
}

/// The availability of each action of one release (task 031, decisions
/// D4 and D5). An action that cannot act is absent.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ReleaseRowAvailability {
    /// The release is not fully in the Library, and no job or removal of
    /// it waits or runs.
    pub(crate) download: bool,
    /// The release is in the Library. A track that is not downloaded
    /// downloads first.
    pub(crate) add_to_playlist: bool,
    /// One or more tracks of the release are downloaded, and no job or
    /// removal of it waits or runs. The release page uses the same rule
    /// (operator answer of 2026-10-21).
    pub(crate) remove: bool,
}

impl ReleaseRowAvailability {
    /// The availability of the actions of a release in the state `state`.
    #[must_use]
    pub(crate) const fn of(state: ReleaseRowState) -> Self {
        match state {
            ReleaseRowState::Index { job_active } => Self {
                download: !job_active,
                add_to_playlist: false,
                remove: false,
            },
            ReleaseRowState::Library(facts) => Self {
                download: facts.downloadable(),
                add_to_playlist: true,
                remove: facts.removable(),
            },
        }
    }
}

/// The count of each release action for a selection. The selection
/// action bar reads these counts.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ReleaseActionCounts {
    pub(crate) download: usize,
    pub(crate) add_to_playlist: usize,
    pub(crate) remove: usize,
}

impl ReleaseActionCounts {
    /// The counts of the releases in `states`. Each release counts in each
    /// action that [`ReleaseRowAvailability::of`] gives it.
    #[must_use]
    pub(crate) fn of(states: impl IntoIterator<Item = ReleaseRowState>) -> Self {
        states.into_iter().map(ReleaseRowAvailability::of).fold(
            Self::default(),
            |counts, available| Self {
                download: counts.download + usize::from(available.download),
                add_to_playlist: counts.add_to_playlist + usize::from(available.add_to_playlist),
                remove: counts.remove + usize::from(available.remove),
            },
        )
    }
}

/// The release that a row menu acts on.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReleaseRowTarget {
    pub(crate) key: ReleaseKey,
    /// The title that a download job shows until its feed loads.
    pub(crate) title: String,
    /// The Library feed id. `None` for an Index release.
    pub(crate) feed_id: Option<i64>,
}

impl ReleaseRowTarget {
    /// The feed GUID of an Index release. `None` for a Library release.
    #[must_use]
    pub(crate) fn index_feed_guid(&self) -> Option<&str> {
        match (&self.key, self.feed_id) {
            (ReleaseKey::FeedGuid(feed_guid), None) => Some(feed_guid),
            _ => None,
        }
    }
}

/// One command of a release row menu. The host maps each command to the
/// path of the matching action of the selection action bar.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ReleaseRowCommand {
    /// "Download <kind>": queue one job for the release.
    Download,
    /// "Add to playlist", then the playlist with this id.
    AddToPlaylist(i64),
    /// "Add to playlist", then "New Playlist" with this name.
    CreatePlaylist(String),
    /// "Remove <kind>…": the removal with its confirmation.
    Remove,
}

impl ReleaseRowCommand {
    /// The command of a menu item. "Add to playlist" opens the playlist
    /// level, so it has no command.
    #[must_use]
    pub(crate) const fn of_menu_action(action: TrackRowMenuAction) -> Option<Self> {
        match action {
            TrackRowMenuAction::Download => Some(Self::Download),
            TrackRowMenuAction::Remove => Some(Self::Remove),
            TrackRowMenuAction::AddToPlaylist
            | TrackRowMenuAction::InspectSources
            | TrackRowMenuAction::MoveUp
            | TrackRowMenuAction::MoveDown
            | TrackRowMenuAction::RemoveFromPlaylist => None,
        }
    }
}

/// The "⋯" menu and the right-click menu of one release row, with the
/// release that it acts on.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReleaseRowMenuVm {
    /// The items. The menu composite of task 006 reads this type.
    pub(crate) menu: TrackRowMenuVm,
    pub(crate) target: ReleaseRowTarget,
}

/// The facts of one release row that select its menu items.
#[derive(Clone, Debug)]
pub(crate) struct ReleaseRowActionFacts<'a> {
    /// The element id of the row. The menu id starts with it.
    pub(crate) row_id: &'a str,
    pub(crate) key: &'a ReleaseKey,
    pub(crate) title: &'a str,
    pub(crate) kind: ReleaseKind,
    pub(crate) state: ReleaseRowState,
    /// The list is in selection mode.
    pub(crate) selecting: bool,
}

/// The menu of one release row (task 031). `None` in selection mode
/// (decision D7) and for a row with no available action. The items are
/// "Download <kind>", "Add to playlist" and "Remove <kind>…", in this
/// order. An item that cannot act is absent. The destructive item is
/// last.
#[must_use]
pub(crate) fn release_row_menu(facts: &ReleaseRowActionFacts<'_>) -> Option<ReleaseRowMenuVm> {
    if facts.selecting {
        return None;
    }
    let available = ReleaseRowAvailability::of(facts.state);
    let id = format!("{}-menu", facts.row_id);
    let noun = facts.kind.noun();
    let title = facts.title;
    let item = |action: TrackRowMenuAction, label: AlbumPageAction, a11y_label: String| {
        TrackRowMenuItemVm {
            action,
            id: format!("{id}:{action:?}"),
            label: label.label(facts.kind),
            a11y_label,
            destructive: label.is_destructive(),
        }
    };
    let mut items = Vec::new();
    if available.download {
        items.push(item(
            TrackRowMenuAction::Download,
            AlbumPageAction::DownloadAlbum,
            format!("Download the {noun} {title}"),
        ));
    }
    if available.add_to_playlist {
        items.push(item(
            TrackRowMenuAction::AddToPlaylist,
            AlbumPageAction::AddToPlaylist,
            format!("Add the {noun} {title} to a playlist"),
        ));
    }
    if available.remove {
        items.push(item(
            TrackRowMenuAction::Remove,
            AlbumPageAction::RemoveAlbum,
            format!("Remove the {noun} {title} from the Library\u{2026}"),
        ));
    }
    if items.is_empty() {
        return None;
    }
    Some(ReleaseRowMenuVm {
        menu: TrackRowMenuVm {
            trigger_tooltip: RELEASE_MENU_TRIGGER_TOOLTIP,
            trigger_a11y_label: format!("More actions for {title}"),
            items,
            playlist_level: RELEASE_ROW_PLAYLIST_LEVEL,
            id,
        },
        target: ReleaseRowTarget {
            key: facts.key.clone(),
            title: title.to_owned(),
            feed_id: facts.state.feed_id(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_models::detail_selection::{
        DetailPageRows, DetailReleaseRow, DetailRowKey, DetailSelection, ReleaseDownloaded,
    };
    use crate::view_models::selection_action_bar::SelectionActionKind;
    use crate::view_models::workspace::FrameNavigationEntry;

    fn key() -> ReleaseKey {
        ReleaseKey::FeedGuid("guid-1".to_owned())
    }

    /// The downloaded tracks of a Library release.
    #[derive(Clone, Copy, Debug)]
    enum Tracks {
        /// No track is downloaded.
        None,
        /// One or more tracks are downloaded, but not each track.
        Partly,
        /// Each track is downloaded.
        Fully,
    }

    const TRACKS: [Tracks; 3] = [Tracks::None, Tracks::Partly, Tracks::Fully];

    fn library(tracks: Tracks, download_active: bool, removing: bool) -> ReleaseRowState {
        ReleaseRowState::Library(LibraryReleaseFacts {
            feed_id: 7,
            downloaded: match tracks {
                Tracks::None => ReleaseDownloaded::NoTrack,
                Tracks::Partly => ReleaseDownloaded::SomeTracks,
                Tracks::Fully => ReleaseDownloaded::EachTrack,
            },
            download_active,
            removing,
        })
    }

    fn menu(
        state: ReleaseRowState,
        kind: ReleaseKind,
        selecting: bool,
    ) -> Option<ReleaseRowMenuVm> {
        let key = key();
        release_row_menu(&ReleaseRowActionFacts {
            row_id: "row",
            key: &key,
            title: "Face Pale",
            kind,
            state,
            selecting,
        })
    }

    fn actions(state: ReleaseRowState) -> Vec<TrackRowMenuAction> {
        menu(state, ReleaseKind::Album, false)
            .map(|menu| menu.menu.items.iter().map(|item| item.action).collect())
            .unwrap_or_default()
    }

    /// The bar of a detail selection of the one release in `state`.
    fn bar_kinds(state: ReleaseRowState) -> Vec<SelectionActionKind> {
        let ReleaseRowState::Library(facts) = state else {
            panic!("the detail bar reads the Library facts only");
        };
        let rows = DetailPageRows::Releases(vec![DetailReleaseRow {
            key: key(),
            title: "Face Pale".to_owned(),
            library: Some(facts),
        }]);
        let mut selection = DetailSelection::default();
        let entry = FrameNavigationEntry::PublisherDetail("feed".to_owned());
        selection.reset(Some(entry));
        selection.toggle_mode();
        selection.toggle_row(&DetailRowKey::Release(key()));
        rows.action_bar(&selection)
            .expect("the bar shows in Selecting")
            .actions
            .iter()
            .map(|action| action.kind)
            .filter(|kind| *kind != SelectionActionKind::Clear)
            .collect()
    }

    fn bar_kind_of(action: TrackRowMenuAction) -> SelectionActionKind {
        match action {
            TrackRowMenuAction::Download => SelectionActionKind::Download,
            TrackRowMenuAction::AddToPlaylist => SelectionActionKind::AddToPlaylist,
            TrackRowMenuAction::Remove => SelectionActionKind::Remove,
            other => panic!("a release row has no {other:?}"),
        }
    }

    /// R31-01 (ADR 0084 Decision 10, ADR 0083 Decision 5): a Library
    /// album that is not fully downloaded, with one downloaded track and no
    /// job, gives "Download album", "Add to playlist", "Remove album…", in
    /// this order. "Remove album…" is destructive and last. A downloaded
    /// album gives "Add to playlist" and "Remove album…" (operator answer
    /// of 2026-10-21: the release page rule).
    #[test]
    fn adr_0084_release_row_actions_r31_01_items_and_order() {
        let partly = menu(
            library(Tracks::Partly, false, false),
            ReleaseKind::Album,
            false,
        )
        .expect("a Library album has a menu");
        let labels: Vec<&str> = partly
            .menu
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect();
        assert_eq!(
            labels,
            ["Download album", "Add to playlist", "Remove album\u{2026}"]
        );
        assert_eq!(partly.target.feed_id, Some(7));
        let last = partly.menu.items.last().expect("an item");
        assert_eq!(last.action, TrackRowMenuAction::Remove);
        assert!(last.destructive);
        assert!(partly.menu.items[..partly.menu.items.len() - 1]
            .iter()
            .all(|item| !item.destructive));

        let full = menu(
            library(Tracks::Fully, false, false),
            ReleaseKind::Album,
            false,
        )
        .expect("a downloaded album has a menu");
        let labels: Vec<&str> = full
            .menu
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect();
        assert_eq!(labels, ["Add to playlist", "Remove album\u{2026}"]);
        assert_eq!(full.menu.id, "row-menu");
        assert_eq!(full.menu.items[0].id, "row-menu:AddToPlaylist");
        assert_eq!(full.menu.trigger_a11y_label, "More actions for Face Pale");
    }

    /// R31-02 (task 031, decision D4): no "Download <kind>" for a release
    /// fully in the Library, or with a waiting or running job. No "Remove
    /// <kind>…" for a release with no downloaded track, with a job, or
    /// during a removal.
    #[test]
    fn adr_0084_release_row_actions_r31_02_absent_items() {
        use TrackRowMenuAction::{AddToPlaylist, Download, Remove};
        assert_eq!(
            actions(library(Tracks::None, false, false)),
            [Download, AddToPlaylist]
        );
        assert_eq!(
            actions(library(Tracks::Fully, false, false)),
            [AddToPlaylist, Remove]
        );
        for tracks in TRACKS {
            assert_eq!(actions(library(tracks, true, false)), [AddToPlaylist]);
            assert_eq!(actions(library(tracks, false, true)), [AddToPlaylist]);
            assert_eq!(actions(library(tracks, true, true)), [AddToPlaylist]);
        }
        assert_eq!(
            actions(ReleaseRowState::Index { job_active: false }),
            [Download]
        );
        assert!(actions(ReleaseRowState::Index { job_active: true }).is_empty());
        assert!(
            menu(
                ReleaseRowState::Index { job_active: true },
                ReleaseKind::Album,
                false
            )
            .is_none(),
            "a row with no available action shows no menu"
        );
    }

    /// R31-03 (ADR 0084 Decision 10): for each Library case of R31-01 and
    /// R31-02, the menu and the bar of a selection of that one release
    /// give the same "Download", "Add to playlist" and "Remove".
    #[test]
    fn adr_0084_release_row_actions_r31_03_menu_and_bar_agree() {
        for tracks in TRACKS {
            for job in [false, true] {
                for removing in [false, true] {
                    let state = library(tracks, job, removing);
                    let menu_kinds: Vec<_> = actions(state).into_iter().map(bar_kind_of).collect();
                    let mut bar = bar_kinds(state);
                    // The bar order is "Download", "Add", "Remove". The
                    // menu has the same order.
                    bar.retain(|kind| {
                        matches!(
                            kind,
                            SelectionActionKind::Download
                                | SelectionActionKind::AddToPlaylist
                                | SelectionActionKind::Remove
                        )
                    });
                    assert_eq!(
                        menu_kinds, bar,
                        "tracks {tracks:?}, job {job}, removing {removing}"
                    );
                }
            }
        }
    }

    /// R31-03: the counts of the bar and the menu use the one
    /// availability rule, also for an Index release.
    #[test]
    fn adr_0084_release_row_actions_r31_03_counts_use_the_one_rule() {
        let states = [
            library(Tracks::None, false, false),
            library(Tracks::Partly, false, false),
            library(Tracks::Fully, false, false),
            library(Tracks::Fully, true, false),
            library(Tracks::Partly, false, true),
            ReleaseRowState::Index { job_active: false },
            ReleaseRowState::Index { job_active: true },
        ];
        let counts = ReleaseActionCounts::of(states);
        let menus: Vec<_> = states.iter().map(|state| actions(*state)).collect();
        let count = |action| menus.iter().filter(|items| items.contains(&action)).count();
        assert_eq!(counts.download, count(TrackRowMenuAction::Download));
        assert_eq!(
            counts.add_to_playlist,
            count(TrackRowMenuAction::AddToPlaylist)
        );
        assert_eq!(counts.remove, count(TrackRowMenuAction::Remove));
        assert_eq!(
            counts,
            ReleaseActionCounts {
                download: 3,
                add_to_playlist: 5,
                remove: 2,
            }
        );
    }

    /// R31-04 (task 031, decision D5): an Index release gives "Download
    /// <kind>". A single gives "Download single". The target holds the
    /// feed GUID for the Index download path.
    #[test]
    fn adr_0084_release_row_actions_r31_04_index_release() {
        let index = ReleaseRowState::Index { job_active: false };
        for (kind, label) in [
            (ReleaseKind::Album, "Download album"),
            (ReleaseKind::Ep, "Download EP"),
            (ReleaseKind::resolve(None, 1), "Download single"),
        ] {
            let menu = menu(index, kind, false).expect("an Index release has a menu");
            assert_eq!(menu.menu.items[0].label, label);
            assert_eq!(menu.target.index_feed_guid(), Some("guid-1"));
            assert_eq!(menu.target.feed_id, None);
        }
        let single = menu(
            library(Tracks::Fully, false, false),
            ReleaseKind::resolve(None, 1),
            false,
        )
        .expect("a menu");
        assert_eq!(single.menu.items[1].label, "Remove single\u{2026}");
        assert_eq!(single.target.index_feed_guid(), None);
    }

    /// R31-05 (task 031, decision D7): in `Selecting`, the menu is `None`.
    #[test]
    fn adr_0084_release_row_actions_r31_05_selecting_has_no_menu() {
        for state in [
            library(Tracks::Partly, false, false),
            library(Tracks::Fully, false, false),
            ReleaseRowState::Index { job_active: false },
        ] {
            assert!(menu(state, ReleaseKind::Album, true).is_none());
        }
    }

    /// Each menu command maps to one command of the host. "Add to
    /// playlist" opens the playlist level.
    #[test]
    fn adr_0084_release_row_actions_menu_commands() {
        assert_eq!(
            ReleaseRowCommand::of_menu_action(TrackRowMenuAction::Download),
            Some(ReleaseRowCommand::Download)
        );
        assert_eq!(
            ReleaseRowCommand::of_menu_action(TrackRowMenuAction::Remove),
            Some(ReleaseRowCommand::Remove)
        );
        assert_eq!(
            ReleaseRowCommand::of_menu_action(TrackRowMenuAction::AddToPlaylist),
            None
        );
    }
}
