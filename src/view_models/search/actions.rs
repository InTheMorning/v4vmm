//! Inspector action-row and playlist append projections.

#![warn(clippy::pedantic)]

use crate::view_models::entity_detail::{
    EntityActionTarget, EntityActionVm, PlaylistActionState, ReleaseActionState,
    ReleaseMembershipState,
};
use crate::view_models::format::plural;
use crate::view_models::ActionStatusMessageDisplay;
use crate::views::FeedRef;

/// Borrow-only projection over the per-entity action-row state owned by
/// the search inspector. Owns:
/// * the visibility rule (only `feed` and `track` carry an action row);
/// * the four-way subscription button label (busy × subscribed);
/// * release action labels for feed subscription and playlist affordances;
/// * the message-is-error classification used to pick the status colour.
///
/// The screen still owns click handlers and rendering;
/// the VM owns the strings and the boolean classifications.
pub(crate) struct ActionRowVm<'a> {
    entity_type: &'a str,
    subscription_busy: bool,
    local_subscription: Option<bool>,
    subscription_message: Option<&'a str>,
}

/// Pure command label/message semantics for inspector subscription actions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SearchSubscriptionCommand {
    Download,
    Remove,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SearchInspectorPlaylistDisplay {
    pub(crate) popover_id: String,
    pub(crate) trigger_label: String,
}

impl<'a> ActionRowVm<'a> {
    #[must_use]
    pub(crate) fn new(
        entity_type: &'a str,
        subscription_busy: bool,
        local_subscription: Option<bool>,
        subscription_message: Option<&'a str>,
    ) -> Self {
        Self {
            entity_type,
            subscription_busy,
            local_subscription,
            subscription_message,
        }
    }

    /// `true` when an action row should render for this entity type.
    /// Only `feed` and `track` ever do.
    #[must_use]
    pub(crate) fn is_visible(&self) -> bool {
        matches!(self.entity_type, "feed" | "track")
    }

    /// Subscription button label. Distinguishes the busy and idle
    /// states, and routes idle by `entity_type` for the `Feed`/`Track`
    /// noun.
    #[must_use]
    pub(crate) fn subscription_button_label(&self) -> String {
        if self.entity_type == "feed" {
            return self
                .release_primary_action(EntityActionTarget::Feed(
                    FeedRef::Musicindex(String::new()),
                ))
                .label;
        }

        let subscribed = self.local_subscription.unwrap_or(false);
        if self.subscription_busy {
            return if subscribed {
                "Removing...".into()
            } else {
                "Downloading...".into()
            };
        }
        let noun = if self.entity_type == "feed" {
            "Feed"
        } else {
            "Track"
        };
        if subscribed {
            format!("Remove {noun}")
        } else {
            format!("Download {noun}")
        }
    }

    /// Label for the playlist popover trigger. Feeds get the
    /// `Add feed to playlist` form so the operator knows the whole album
    /// will be added.
    #[must_use]
    pub(crate) fn add_to_playlist_label(&self) -> &'static str {
        if self.entity_type == "feed" {
            "Add feed to playlist"
        } else {
            "Add to playlist"
        }
    }

    #[must_use]
    pub(crate) fn inspector_playlist_display(
        &self,
        entity_id: &str,
        trigger_label: impl Into<String>,
    ) -> SearchInspectorPlaylistDisplay {
        let trigger_label = trigger_label.into();
        let trigger_label = if trigger_label.is_empty() {
            self.add_to_playlist_label().to_string()
        } else {
            trigger_label
        };
        SearchInspectorPlaylistDisplay {
            popover_id: format!("inspector-add:{entity_id}"),
            trigger_label,
        }
    }

    #[must_use]
    pub(crate) fn playlist_trigger_label(
        &self,
        release_playlist_action: Option<&EntityActionVm>,
    ) -> String {
        if self.entity_type == "feed" {
            release_playlist_action.map_or_else(
                || self.add_to_playlist_label().to_string(),
                |action| action.label.clone(),
            )
        } else {
            self.add_to_playlist_label().to_string()
        }
    }

    #[must_use]
    pub(crate) fn release_primary_action(&self, target: EntityActionTarget) -> EntityActionVm {
        self.release_action_state(PlaylistActionState::Hidden)
            .primary_action(target)
    }

    #[must_use]
    pub(crate) fn release_playlist_action(
        &self,
        target: EntityActionTarget,
    ) -> Option<EntityActionVm> {
        self.release_action_state(PlaylistActionState::Closed)
            .playlist_action(target)
    }

    #[must_use]
    fn release_action_state(&self, playlist: PlaylistActionState) -> ReleaseActionState {
        let membership = if self.subscription_busy {
            if self.local_subscription.unwrap_or(false) {
                ReleaseMembershipState::Removing
            } else {
                ReleaseMembershipState::Downloading
            }
        } else if self.local_subscription.unwrap_or(false) {
            ReleaseMembershipState::InLibrary
        } else {
            ReleaseMembershipState::RemoteOnly
        };

        ReleaseActionState::new(membership, playlist)
    }

    #[must_use]
    pub(crate) fn subscription_message_display(&self) -> Option<ActionStatusMessageDisplay> {
        ActionStatusMessageDisplay::subscription(self.subscription_message)
    }

    #[expect(
        clippy::unused_self,
        reason = "kept as an instance method so action-row labels travel with the VM contract"
    )]
    #[must_use]
    pub(crate) const fn action_row_a11y_label(&self) -> &'static str {
        "Inspector actions"
    }
}

impl SearchSubscriptionCommand {
    #[must_use]
    pub(crate) fn begin_message(self) -> &'static str {
        match self {
            Self::Download => "Downloading...",
            Self::Remove => "Removing...",
        }
    }

    #[must_use]
    pub(crate) const fn track_download_success_message() -> &'static str {
        "Downloaded track"
    }

    #[must_use]
    pub(crate) fn error_message(self, error: impl std::fmt::Display) -> String {
        match self {
            Self::Download => format!("Download error: {error:#}"),
            Self::Remove => format!("Remove error: {error:#}"),
        }
    }

    #[must_use]
    pub(crate) fn success_message(self, applied_edits: usize) -> String {
        match self {
            Self::Download => {
                if applied_edits == 0 {
                    Self::track_download_success_message().into()
                } else {
                    format!(
                        "Downloaded track, applied {applied_edits} ID3 edit{}",
                        plural(applied_edits)
                    )
                }
            }
            Self::Remove => "Removed track".into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlaylistAppendIntent {
    pub(super) playlist_id: i64,
    pub(super) track_ids: Vec<i64>,
    pub(super) playlist_name: String,
}

impl PlaylistAppendIntent {
    #[must_use]
    pub(crate) fn playlist_id(&self) -> i64 {
        self.playlist_id
    }

    #[must_use]
    pub(crate) fn track_ids(&self) -> &[i64] {
        &self.track_ids
    }

    #[must_use]
    pub(crate) fn total_tracks(&self) -> usize {
        self.track_ids.len()
    }

    #[must_use]
    pub(crate) fn playlist_name(&self) -> &str {
        &self.playlist_name
    }
}

/// Pure result counts for a completed playlist append command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PlaylistAppendOutcome {
    pub(super) appended: usize,
    pub(super) downloaded: usize,
    pub(super) failed: usize,
}

impl PlaylistAppendOutcome {
    #[must_use]
    pub(crate) fn new(appended: usize, downloaded: usize, failed: usize) -> Self {
        Self {
            appended,
            downloaded,
            failed,
        }
    }
}
