//! Display and pending-state contract for local library removals.

#![warn(clippy::pedantic)]

use crate::application::library_removal::{
    LibraryRemovalImpact, LibraryRemovalPlan, LibraryRemovalTarget,
};

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
    pub(crate) fn confirm_or_defer(&mut self, plan: LibraryRemovalPlan) -> bool {
        if !plan.requires_confirmation() {
            return true;
        }
        self.pending = Some(plan);
        false
    }

    #[must_use]
    pub(crate) fn pending_display(&self) -> Option<LibraryRemovalConfirmationDisplay> {
        let plan = self.pending?;
        match plan.impact() {
            LibraryRemovalImpact::Track {
                playlist_reference_count,
            } => {
                let playlist_label = if playlist_reference_count == 1 {
                    "playlist"
                } else {
                    "playlists"
                };
                Some(LibraryRemovalConfirmationDisplay {
                    title: "Remove Track from Library?",
                    message: format!(
                        "This track is in {playlist_reference_count} {playlist_label}. Removing it from the library will make it unavailable for playlist playback."
                    ),
                    cancel_button_id: "library-removal-cancel",
                    cancel_label: "Cancel",
                    cancel_a11y_label: "Cancel removing track from library",
                    remove_button_id: "library-removal-confirm",
                    remove_label: "Remove",
                    remove_a11y_label: "Remove track from library",
                })
            }
            LibraryRemovalImpact::Feed {
                playlist_track_count,
            } => {
                let (track_label, verb, object_pronoun) = if playlist_track_count == 1 {
                    ("track", "is", "it")
                } else {
                    ("tracks", "are", "them")
                };
                Some(LibraryRemovalConfirmationDisplay {
                    title: "Remove Feed from Library?",
                    message: format!(
                        "{playlist_track_count} {track_label} from this feed {verb} in playlists. Removing {object_pronoun} from the library will make {object_pronoun} unavailable for playlist playback."
                    ),
                    cancel_button_id: "library-removal-cancel",
                    cancel_label: "Cancel",
                    cancel_a11y_label: "Cancel removing feed from library",
                    remove_button_id: "library-removal-confirm",
                    remove_label: "Remove",
                    remove_a11y_label: "Remove feed from library",
                })
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removal_confirmation_state_projects_hig_track_warning() {
        let mut state = LibraryRemovalConfirmationState::new();
        let plan = LibraryRemovalPlan::new(
            LibraryRemovalTarget::Track(7),
            LibraryRemovalImpact::Track {
                playlist_reference_count: 1,
            },
        );

        assert!(!state.confirm_or_defer(plan));
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

    #[test]
    fn removal_confirmation_state_returns_target_after_confirmation() {
        let mut state = LibraryRemovalConfirmationState::new();
        let plan = LibraryRemovalPlan::new(
            LibraryRemovalTarget::Feed(5),
            LibraryRemovalImpact::Feed {
                playlist_track_count: 2,
            },
        );

        assert!(!state.confirm_or_defer(plan));
        assert_eq!(
            state.take_pending_target(),
            Some(LibraryRemovalTarget::Feed(5))
        );
        assert!(state.pending_display().is_none());
    }
}
