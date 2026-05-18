//! Track row and track inspector display projections.

#![warn(clippy::pedantic)]

use crate::api::Track;
use crate::view_models::entity_detail::{
    EntityActionKind, EntityActionTarget, EntityActionVm, PlaylistActionState, TrackActionState,
    TrackMembershipState,
};
use crate::views::TrackRef;

use super::common::nonempty_text;

/// Display and identity projection for per-track row actions in Discover.
///
/// The screen owns GPUI buttons and service dispatch. This VM owns the stable
/// row key plus the download/remove labels and tooltips used by those buttons.
pub(crate) struct TrackRowActionVm<'a> {
    track: &'a Track,
    is_downloaded: bool,
    is_in_flight: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TrackRowDownloadDisplay {
    pub(crate) busy_indicator_id: String,
    pub(crate) button_id: String,
    pub(crate) busy_tooltip: &'static str,
}

impl<'a> TrackRowActionVm<'a> {
    #[must_use]
    pub(crate) fn new(track: &'a Track, is_downloaded: bool, is_in_flight: bool) -> Self {
        Self {
            track,
            is_downloaded,
            is_in_flight,
        }
    }

    #[must_use]
    pub(crate) fn key(&self) -> String {
        self.track
            .enclosure_url
            .clone()
            .or_else(|| self.track.track_guid.clone())
            .unwrap_or_default()
    }

    #[must_use]
    pub(crate) fn busy_tooltip(&self) -> &'static str {
        match self.primary_action().kind {
            EntityActionKind::Remove => "Removing...",
            _ => "Downloading...",
        }
    }

    #[must_use]
    pub(crate) fn is_in_flight(&self) -> bool {
        self.is_in_flight
    }

    #[must_use]
    pub(crate) fn primary_action(&self) -> EntityActionVm {
        self.action_state()
            .primary_action(EntityActionTarget::Track(self.track_ref()))
    }

    #[must_use]
    pub(crate) fn download_display(&self) -> TrackRowDownloadDisplay {
        let key = self.key();
        TrackRowDownloadDisplay {
            busy_indicator_id: format!("track-row-download-spin:{key}"),
            button_id: format!("track-row-download:{key}"),
            busy_tooltip: self.busy_tooltip(),
        }
    }

    #[must_use]
    pub(crate) fn action_state(&self) -> TrackActionState {
        let membership = match (self.is_downloaded, self.is_in_flight) {
            (true, true) => TrackMembershipState::Removing,
            (true, false) => TrackMembershipState::InLibrary,
            (false, true) => TrackMembershipState::Downloading,
            (false, false) => TrackMembershipState::RemoteOnly,
        };
        TrackActionState::new(membership, PlaylistActionState::Closed)
            .with_download_available(self.track.enclosure_url.is_some())
    }

    #[must_use]
    fn track_ref(&self) -> TrackRef {
        TrackRef::Musicindex(
            self.track
                .track_guid
                .clone()
                .or_else(|| self.track.enclosure_url.clone())
                .unwrap_or_default(),
        )
    }
}

/// Borrow-only projection over the discover track-inspector header.
/// Owns the feed-link URL fallback (`feed_url` -> `feed_guid`) and the
/// feed-link label fallback (`feed_title` -> caller-provided guid).
pub(crate) struct TrackInspectorHeaderVm<'a> {
    track: &'a Track,
}

/// Display-ready feed link for the Discover track inspector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TrackFeedLinkDisplay {
    pub(crate) element_id: String,
    pub(crate) guid: String,
    pub(crate) label: String,
    pub(crate) url: Option<String>,
    pub(crate) tooltip: String,
}

impl<'a> TrackInspectorHeaderVm<'a> {
    #[must_use]
    pub(crate) fn new(track: &'a Track) -> Self {
        Self { track }
    }

    /// Complete feed-link display contract for the track inspector.
    #[must_use]
    pub(crate) fn feed_link_display(&self) -> Option<TrackFeedLinkDisplay> {
        let guid = nonempty_text(self.track.feed_guid.as_deref())?.to_string();
        Some(TrackFeedLinkDisplay {
            element_id: format!("track-feed-link:{guid}"),
            label: self.feed_link_label(&guid),
            url: self.feed_link_url(),
            tooltip: guid.clone(),
            guid,
        })
    }

    /// URL the feed link should target — `feed_url` first, else
    /// `feed_guid` (used as a stand-in identifier when no URL is
    /// known).
    #[must_use]
    pub(crate) fn feed_link_url(&self) -> Option<String> {
        self.track
            .feed_url
            .clone()
            .or_else(|| self.track.feed_guid.clone())
    }

    /// Visible label for the feed link — trimmed `feed_title` if
    /// non-empty, otherwise the supplied `guid_fallback` (typically
    /// the row's `feed_guid`).
    #[must_use]
    pub(crate) fn feed_link_label(&self, guid_fallback: &str) -> String {
        self.track
            .feed_title
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map_or_else(|| guid_fallback.to_string(), str::to_string)
    }
}
