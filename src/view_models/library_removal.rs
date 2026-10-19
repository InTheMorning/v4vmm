//! Display and pending-state contract for local library removals.

#![warn(clippy::pedantic)]

use crate::application::library_removal::{
    LibraryRemovalImpact, LibraryRemovalPlan, LibraryRemovalTarget,
};
use crate::view_models::format::plural;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LibraryRemovalConfirmationDisplay {
    pub(crate) title: &'static str,
    pub(crate) message: String,
    pub(crate) cancel_button_id: &'static str,
    pub(crate) cancel_label: &'static str,
    pub(crate) cancel_a11y_label: &'static str,
    pub(crate) remove_button_id: &'static str,
    pub(crate) remove_label: &'static str,
    pub(crate) remove_a11y_label: &'static str,
}

/// When a removal asks the operator to confirm it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RemovalConfirmation {
    /// Ask only when a playlist refers to the removed tracks.
    WhenReferenced,
    /// Ask for each removal. A destructive menu item uses this (ADR 0083
    /// Decision 5).
    Always,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct LibraryRemovalConfirmationState {
    pending: Option<LibraryRemovalPlan>,
}

impl LibraryRemovalConfirmationState {
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self { pending: None }
    }

    #[must_use]
    pub(crate) fn confirm_or_defer(
        &mut self,
        plan: LibraryRemovalPlan,
        confirmation: RemovalConfirmation,
    ) -> bool {
        if confirmation == RemovalConfirmation::WhenReferenced && !plan.requires_confirmation() {
            return true;
        }
        self.pending = Some(plan);
        false
    }

    #[must_use]
    pub(crate) fn pending_display(&self) -> Option<LibraryRemovalConfirmationDisplay> {
        let plan = self.pending?;
        let message = removal_impact_message(plan.impact());
        match plan.impact() {
            LibraryRemovalImpact::Track { .. } => Some(LibraryRemovalConfirmationDisplay {
                title: "Remove Track from Library?",
                message,
                cancel_button_id: "library-removal-cancel",
                cancel_label: "Cancel",
                cancel_a11y_label: "Cancel removing track from library",
                remove_button_id: "library-removal-confirm",
                remove_label: "Remove",
                remove_a11y_label: "Remove track from library",
            }),
            LibraryRemovalImpact::Feed { .. } => Some(LibraryRemovalConfirmationDisplay {
                title: "Remove Album from Library?",
                message,
                cancel_button_id: "library-removal-cancel",
                cancel_label: "Cancel",
                cancel_a11y_label: "Cancel removing album from library",
                remove_button_id: "library-removal-confirm",
                remove_label: "Remove",
                remove_a11y_label: "Remove album from library",
            }),
        }
    }

    pub(crate) fn cancel(&mut self) {
        self.pending = None;
    }

    pub(crate) fn take_pending_target(&mut self) -> Option<LibraryRemovalTarget> {
        let target = self.pending.map(LibraryRemovalPlan::target)?;
        self.pending = None;
        Some(target)
    }
}

/// The impact text of one removal (ADR 0084 Decision 10 Design: "The
/// removal confirmation" reuses this for each item of a selection).
#[must_use]
pub(crate) fn removal_impact_message(impact: LibraryRemovalImpact) -> String {
    match impact {
        LibraryRemovalImpact::Track {
            playlist_reference_count,
        } => {
            if playlist_reference_count == 0 {
                "This track is in no playlist. Removing it takes it out of the library."
                    .to_owned()
            } else {
                let playlist_label = if playlist_reference_count == 1 {
                    "playlist"
                } else {
                    "playlists"
                };
                format!(
                    "This track is in {playlist_reference_count} {playlist_label}. Removing it from the library will make it unavailable for playlist playback."
                )
            }
        }
        LibraryRemovalImpact::Feed {
            playlist_track_count,
        } => match playlist_track_count {
            0 => "No track from this album is in a playlist. Removing the album takes its tracks out of the library."
                .to_owned(),
            1 => "1 track from this album is in playlists. Removing it from the library will make it unavailable for playlist playback."
                .to_owned(),
            count => format!(
                "{count} tracks from this album are in playlists. Removing them from the library will make them unavailable for playlist playback."
            ),
        },
    }
}

/// One item of a selection removal confirmation (ADR 0084 Decision 10).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SelectionRemovalItemDisplay {
    /// The item's title, for example a track title or a release name.
    pub(crate) title: String,
    /// The present impact text of the item's `LibraryRemovalPlan`.
    pub(crate) impact_message: String,
    /// Each playlist that refers to this item, A to Z. Empty when none
    /// does.
    pub(crate) referring_playlists: Vec<String>,
}

/// One item waiting for the operator's confirmation: its plan, to
/// execute, and its display, to show.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SelectionRemovalPendingItem {
    pub(crate) plan: LibraryRemovalPlan,
    pub(crate) display: SelectionRemovalItemDisplay,
}

impl SelectionRemovalPendingItem {
    #[must_use]
    pub(crate) fn new(
        plan: LibraryRemovalPlan,
        title: impl Into<String>,
        referring_playlists: Vec<String>,
    ) -> Self {
        Self {
            display: SelectionRemovalItemDisplay {
                title: title.into(),
                impact_message: removal_impact_message(plan.impact()),
                referring_playlists,
            },
            plan,
        }
    }
}

/// The confirmation of a whole-selection removal (ADR 0084 Decision 10
/// Design: "The removal confirmation"). It always confirms, so no caller
/// builds one through `RemovalConfirmation::WhenReferenced`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SelectionRemovalConfirmationDisplay {
    pub(crate) title: String,
    pub(crate) items: Vec<SelectionRemovalItemDisplay>,
    pub(crate) cancel_button_id: &'static str,
    pub(crate) cancel_label: &'static str,
    pub(crate) cancel_a11y_label: &'static str,
    pub(crate) remove_button_id: &'static str,
    pub(crate) remove_label: &'static str,
    pub(crate) remove_a11y_label: &'static str,
}

/// Pending state of a whole-selection removal confirmation (ADR 0084
/// Decision 10). One confirmation covers the whole selection, with
/// `RemovalConfirmation::Always`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct SelectionRemovalConfirmationState {
    pending: Vec<SelectionRemovalPendingItem>,
    /// The singular noun of one item, for example "track" or "release".
    item_noun: &'static str,
}

impl SelectionRemovalConfirmationState {
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self {
            pending: Vec::new(),
            item_noun: "item",
        }
    }

    /// Opens the confirmation for `items`. An empty `items` opens no
    /// confirmation, since "Remove" is absent from the action bar with
    /// no removable item.
    pub(crate) fn open(
        &mut self,
        item_noun: &'static str,
        items: Vec<SelectionRemovalPendingItem>,
    ) {
        self.item_noun = item_noun;
        self.pending = items;
    }

    #[must_use]
    pub(crate) fn pending_display(&self) -> Option<SelectionRemovalConfirmationDisplay> {
        if self.pending.is_empty() {
            return None;
        }
        let count = self.pending.len();
        let noun = format!("{}{}", self.item_noun, plural(count));
        Some(SelectionRemovalConfirmationDisplay {
            title: format!("Remove {count} {noun} from the Library?"),
            items: self
                .pending
                .iter()
                .map(|item| item.display.clone())
                .collect(),
            cancel_button_id: "library-selection-removal-cancel",
            cancel_label: "Cancel",
            cancel_a11y_label: "Cancel removing the selection from the library",
            remove_button_id: "library-selection-removal-confirm",
            remove_label: "Remove",
            remove_a11y_label: "Remove the selection from the library",
        })
    }

    pub(crate) fn cancel(&mut self) {
        self.pending.clear();
    }

    /// Takes the pending items for a removal: the target and the title of
    /// each one, and the item noun. A [`SelectionRemovalReport`] then
    /// follows each result.
    pub(crate) fn take_pending_items(
        &mut self,
    ) -> (Vec<(LibraryRemovalTarget, String)>, &'static str) {
        let items = self
            .pending
            .drain(..)
            .map(|item| (item.plan.target(), item.display.title))
            .collect();
        (items, self.item_noun)
    }
}

/// The result of one whole-selection removal (ADR 0084 Decision 10). Each
/// item completes or fails apart from the others. When the last item
/// returns, the report names each failed item and its error.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct SelectionRemovalReport {
    waiting: Vec<(LibraryRemovalTarget, String)>,
    failures: Vec<(String, String)>,
    total: usize,
    item_noun: &'static str,
}

impl SelectionRemovalReport {
    /// Starts a report for `items`, each a target and its title.
    #[must_use]
    pub(crate) fn start(
        items: Vec<(LibraryRemovalTarget, String)>,
        item_noun: &'static str,
    ) -> Self {
        Self {
            total: items.len(),
            waiting: items,
            failures: Vec::new(),
            item_noun,
        }
    }

    /// Tells if this report waits for `target`.
    #[must_use]
    pub(crate) fn waits_for(&self, target: LibraryRemovalTarget) -> bool {
        self.waiting.iter().any(|(item, _)| *item == target)
    }

    /// Records a completed removal of `target`.
    pub(crate) fn record_done(&mut self, target: LibraryRemovalTarget) {
        self.waiting.retain(|(item, _)| *item != target);
    }

    /// Records a failed removal of `target` with its error text.
    pub(crate) fn record_failure(
        &mut self,
        target: LibraryRemovalTarget,
        error: impl Into<String>,
    ) {
        if let Some(index) = self.waiting.iter().position(|(item, _)| *item == target) {
            let (_, title) = self.waiting.remove(index);
            self.failures.push((title, error.into()));
        }
    }

    /// Tells if each item returned.
    #[must_use]
    pub(crate) fn finished(&self) -> bool {
        self.waiting.is_empty()
    }

    /// The report text when each item returned and one or more failed.
    /// `None` while items wait, and when each item was removed: a success
    /// stays silent, as a single removal does.
    #[must_use]
    pub(crate) fn failure_text(&self) -> Option<String> {
        if !self.finished() || self.failures.is_empty() {
            return None;
        }
        let removed = self.total - self.failures.len();
        let plural = if self.total == 1 { "" } else { "s" };
        let failed: Vec<String> = self
            .failures
            .iter()
            .map(|(title, error)| format!("{title} ({error})"))
            .collect();
        Some(format!(
            "The app removed {removed} of {} {}{plural}. It could not remove: {}.",
            self.total,
            self.item_noun,
            failed.join(", ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ADR 0084 Decision 10, orchestrator review of task 009: a removal of
    /// many items names each failed item and its error, after the last
    /// item returns. A batch with no failure gives no text.
    #[test]
    fn adr_0084_selection_removal_report_names_each_failed_item() {
        let mut report = SelectionRemovalReport::start(
            vec![
                (LibraryRemovalTarget::Track(1), "Face Pale".to_owned()),
                (LibraryRemovalTarget::Track(2), "Whispers".to_owned()),
                (LibraryRemovalTarget::Track(3), "Motive to Kill".to_owned()),
            ],
            "track",
        );
        report.record_failure(LibraryRemovalTarget::Track(1), "file is locked");
        assert_eq!(report.failure_text(), None, "items still wait");
        report.record_done(LibraryRemovalTarget::Track(2));
        report.record_failure(LibraryRemovalTarget::Track(3), "database is busy");
        assert!(report.finished());
        assert_eq!(
            report.failure_text().as_deref(),
            Some("The app removed 1 of 3 tracks. It could not remove: Face Pale (file is locked), Motive to Kill (database is busy).")
        );

        let mut clean = SelectionRemovalReport::start(
            vec![(
                LibraryRemovalTarget::Feed(9),
                "Biological Weapon".to_owned(),
            )],
            "release",
        );
        assert!(clean.waits_for(LibraryRemovalTarget::Feed(9)));
        clean.record_done(LibraryRemovalTarget::Feed(9));
        assert_eq!(clean.failure_text(), None);
    }

    #[test]
    fn removal_confirmation_state_projects_hig_track_warning() {
        let mut state = LibraryRemovalConfirmationState::new();
        let plan = LibraryRemovalPlan::new(
            LibraryRemovalTarget::Track(7),
            LibraryRemovalImpact::Track {
                playlist_reference_count: 1,
            },
        );

        assert!(!state.confirm_or_defer(plan, RemovalConfirmation::WhenReferenced));
        let display = state
            .pending_display()
            .expect("playlist-referenced removal should require confirmation");

        assert_eq!(display.title, "Remove Track from Library?");
        assert_eq!(
            display.message,
            "This track is in 1 playlist. Removing it from the library will make it unavailable for playlist playback."
        );
        assert_eq!(display.cancel_label, "Cancel");
        assert_eq!(
            display.cancel_a11y_label,
            "Cancel removing track from library"
        );
        assert_eq!(display.remove_label, "Remove");
        assert_eq!(display.remove_a11y_label, "Remove track from library");
    }

    /// ADR 0083 Decision 5: a destructive menu item asks for confirmation
    /// also when no playlist refers to the track.
    #[test]
    fn always_confirmation_defers_an_unreferenced_track_removal() {
        let mut state = LibraryRemovalConfirmationState::new();
        let plan = LibraryRemovalPlan::new(
            LibraryRemovalTarget::Track(7),
            LibraryRemovalImpact::Track {
                playlist_reference_count: 0,
            },
        );

        assert!(state.confirm_or_defer(plan, RemovalConfirmation::WhenReferenced));
        assert!(!state.confirm_or_defer(plan, RemovalConfirmation::Always));
        let display = state
            .pending_display()
            .expect("an always-confirmed removal waits for the operator");
        assert_eq!(display.title, "Remove Track from Library?");
        assert_eq!(
            display.message,
            "This track is in no playlist. Removing it takes it out of the library."
        );
    }

    /// R83-52 (ADR 0083 task 005): "Remove album…" asks for each plan,
    /// also when no playlist refers to a track of the album.
    #[test]
    fn always_confirmation_defers_each_album_removal() {
        for (count, message) in [
            (
                0,
                "No track from this album is in a playlist. Removing the album takes its tracks out of the library.",
            ),
            (
                1,
                "1 track from this album is in playlists. Removing it from the library will make it unavailable for playlist playback.",
            ),
            (
                3,
                "3 tracks from this album are in playlists. Removing them from the library will make them unavailable for playlist playback.",
            ),
        ] {
            let mut state = LibraryRemovalConfirmationState::new();
            let plan = LibraryRemovalPlan::new(
                LibraryRemovalTarget::Feed(5),
                LibraryRemovalImpact::Feed {
                    playlist_track_count: count,
                },
            );

            assert!(!state.confirm_or_defer(plan, RemovalConfirmation::Always));
            let display = state
                .pending_display()
                .expect("an album removal waits for the operator");
            assert_eq!(display.title, "Remove Album from Library?");
            assert_eq!(display.message, message);
        }
    }

    #[test]
    fn removal_confirmation_state_returns_target_after_confirmation() {
        let mut state = LibraryRemovalConfirmationState::new();
        let plan = LibraryRemovalPlan::new(
            LibraryRemovalTarget::Feed(5),
            LibraryRemovalImpact::Feed {
                playlist_track_count: 2,
            },
        );

        assert!(!state.confirm_or_defer(plan, RemovalConfirmation::WhenReferenced));
        assert_eq!(
            state.take_pending_target(),
            Some(LibraryRemovalTarget::Feed(5))
        );
        assert!(state.pending_display().is_none());
    }

    /// R9-07: the selection removal confirmation lists each item by its
    /// title, and names each playlist that refers to it.
    #[test]
    fn adr_0084_selection_r9_07_confirmation_lists_each_item_and_playlist() {
        let mut state = SelectionRemovalConfirmationState::new();
        let items = vec![
            SelectionRemovalPendingItem::new(
                LibraryRemovalPlan::new(
                    LibraryRemovalTarget::Track(7),
                    LibraryRemovalImpact::Track {
                        playlist_reference_count: 2,
                    },
                ),
                "Face Pale",
                vec!["Late Show".to_owned(), "Warmup".to_owned()],
            ),
            SelectionRemovalPendingItem::new(
                LibraryRemovalPlan::new(
                    LibraryRemovalTarget::Track(8),
                    LibraryRemovalImpact::Track {
                        playlist_reference_count: 0,
                    },
                ),
                "Hollow",
                Vec::new(),
            ),
        ];

        state.open("track", items);
        let display = state
            .pending_display()
            .expect("a non-empty selection waits for the operator");

        assert_eq!(display.title, "Remove 2 tracks from the Library?");
        assert_eq!(display.items.len(), 2);
        assert_eq!(display.items[0].title, "Face Pale");
        assert_eq!(
            display.items[0].referring_playlists,
            vec!["Late Show".to_owned(), "Warmup".to_owned()]
        );
        assert_eq!(display.items[1].title, "Hollow");
        assert!(display.items[1].referring_playlists.is_empty());
    }

    /// R9-07: "Remove" is absent when no selected item is in the
    /// Library, so the confirmation opens with no items and shows
    /// nothing.
    #[test]
    fn adr_0084_selection_r9_07_no_confirmation_with_no_items() {
        let mut state = SelectionRemovalConfirmationState::new();
        state.open("track", Vec::new());
        assert!(state.pending_display().is_none());
    }

    /// "Cancel" changes nothing: it leaves the pending items and lets the
    /// screen cancel them without removing anything.
    #[test]
    fn selection_removal_confirmation_cancel_clears_pending_items() {
        let mut state = SelectionRemovalConfirmationState::new();
        state.open(
            "release",
            vec![SelectionRemovalPendingItem::new(
                LibraryRemovalPlan::new(
                    LibraryRemovalTarget::Feed(9),
                    LibraryRemovalImpact::Feed {
                        playlist_track_count: 0,
                    },
                ),
                "Haleen",
                Vec::new(),
            )],
        );

        state.cancel();

        assert!(state.pending_display().is_none());
        assert!(state.take_pending_items().0.is_empty());
    }

    #[test]
    fn selection_removal_confirmation_takes_each_pending_target() {
        let mut state = SelectionRemovalConfirmationState::new();
        state.open(
            "track",
            vec![
                SelectionRemovalPendingItem::new(
                    LibraryRemovalPlan::new(
                        LibraryRemovalTarget::Track(1),
                        LibraryRemovalImpact::Track {
                            playlist_reference_count: 0,
                        },
                    ),
                    "A",
                    Vec::new(),
                ),
                SelectionRemovalPendingItem::new(
                    LibraryRemovalPlan::new(
                        LibraryRemovalTarget::Track(2),
                        LibraryRemovalImpact::Track {
                            playlist_reference_count: 0,
                        },
                    ),
                    "B",
                    Vec::new(),
                ),
            ],
        );

        let (items, item_noun) = state.take_pending_items();

        assert_eq!(
            items,
            vec![
                (LibraryRemovalTarget::Track(1), "A".to_owned()),
                (LibraryRemovalTarget::Track(2), "B".to_owned())
            ]
        );
        assert_eq!(item_noun, "track");
        assert!(state.pending_display().is_none());
    }
}
