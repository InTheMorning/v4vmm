//! Shared entity-detail projections.
//!
//! This module formats source-normalized [`crate::views`] facts into plain
//! display data. Screens bind the resulting values to GPUI controls elsewhere.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;

use crate::view_models::format::fmt_runtime;
use crate::view_models::track::fmt_dur;
use crate::views::{
    ArtistRef, ArtworkRef, ContributorView, EntityIdentityLinks, FeedRef, FeedView, TrackRef,
    TrackView,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntitySurfaceKind {
    Artist,
    Feed,
    Track,
    Contributor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntitySurfaceContext {
    Discover,
    Library,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EntityActionKind {
    Download,
    Remove,
    AddToPlaylist,
    Play,
    CompareMetadata,
    OpenMusicBrainz,
    OpenWebsite,
    CopyNostr,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EntityActionTarget {
    Artist(ArtistRef),
    Feed(FeedRef),
    Track(TrackRef),
    Contributor(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntityActionTone {
    Primary,
    Secondary,
    Quiet,
    DestructiveQuiet,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrackMembershipState {
    RemoteOnly,
    Downloading,
    InLibrary,
    Removing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlaylistActionState {
    Hidden,
    Closed,
    Open,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrackActionState {
    pub membership: TrackMembershipState,
    pub playlist: PlaylistActionState,
    pub download_available: bool,
}

impl TrackActionState {
    #[must_use]
    pub const fn new(membership: TrackMembershipState, playlist: PlaylistActionState) -> Self {
        Self {
            membership,
            playlist,
            download_available: true,
        }
    }

    #[must_use]
    pub const fn for_context(context: EntitySurfaceContext) -> Self {
        match context {
            EntitySurfaceContext::Discover => Self::new(
                TrackMembershipState::RemoteOnly,
                PlaylistActionState::Closed,
            ),
            EntitySurfaceContext::Library => {
                Self::new(TrackMembershipState::InLibrary, PlaylistActionState::Closed)
            }
        }
    }

    #[must_use]
    pub const fn with_download_available(mut self, is_available: bool) -> Self {
        self.download_available = is_available;
        self
    }

    #[must_use]
    pub fn primary_action(&self, target: EntityActionTarget) -> EntityActionVm {
        let action = match self.membership {
            TrackMembershipState::RemoteOnly => EntityActionVm::new(
                EntityActionKind::Download,
                target,
                "Download",
                EntityActionTone::Secondary,
            ),
            TrackMembershipState::Downloading => EntityActionVm::new(
                EntityActionKind::Download,
                target,
                "Downloading...",
                EntityActionTone::Secondary,
            )
            .disabled(),
            TrackMembershipState::InLibrary => EntityActionVm::new(
                EntityActionKind::Remove,
                target,
                "Remove",
                EntityActionTone::DestructiveQuiet,
            ),
            TrackMembershipState::Removing => EntityActionVm::new(
                EntityActionKind::Remove,
                target,
                "Removing...",
                EntityActionTone::DestructiveQuiet,
            )
            .disabled(),
        };

        if self.membership == TrackMembershipState::RemoteOnly && !self.download_available {
            action.disabled()
        } else {
            action
        }
    }

    #[must_use]
    pub fn playlist_action(&self, target: EntityActionTarget) -> Option<EntityActionVm> {
        match self.playlist {
            PlaylistActionState::Hidden => None,
            PlaylistActionState::Closed => Some(EntityActionVm::new(
                EntityActionKind::AddToPlaylist,
                target,
                "+ Playlist",
                EntityActionTone::Quiet,
            )),
            PlaylistActionState::Open => Some(EntityActionVm::new(
                EntityActionKind::AddToPlaylist,
                target,
                "+ Playlist ▴",
                EntityActionTone::Quiet,
            )),
        }
    }

    #[must_use]
    pub fn actions(&self, target: EntityActionTarget) -> Vec<EntityActionVm> {
        let primary = self.primary_action(target.clone());
        let mut actions = vec![primary];
        if let Some(action) = self.playlist_action(target.clone()) {
            actions.push(action);
        }
        actions.push(EntityActionVm::new(
            EntityActionKind::Play,
            target,
            "Play",
            EntityActionTone::Quiet,
        ));
        actions
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityActionVm {
    pub kind: EntityActionKind,
    pub target: EntityActionTarget,
    pub label: String,
    pub enabled: bool,
    pub tone: EntityActionTone,
}

impl EntityActionVm {
    #[must_use]
    pub fn new(
        kind: EntityActionKind,
        target: EntityActionTarget,
        label: impl Into<String>,
        tone: EntityActionTone,
    ) -> Self {
        Self {
            kind,
            target,
            label: label.into(),
            enabled: true,
            tone,
        }
    }

    #[must_use]
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityDetailRow {
    pub key: &'static str,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityHeaderVm<'a> {
    pub kind: EntitySurfaceKind,
    pub title: String,
    pub subtitle: Option<String>,
    pub artwork: Option<&'a ArtworkRef>,
    pub identity: IdentityLinksVm<'a>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdentityLinksVm<'a> {
    identity: &'a EntityIdentityLinks,
}

impl<'a> IdentityLinksVm<'a> {
    #[must_use]
    pub const fn new(identity: &'a EntityIdentityLinks) -> Self {
        Self { identity }
    }

    #[must_use]
    pub fn has_any(&self) -> bool {
        self.nostr_npub().is_some() || self.website_url().is_some()
    }

    #[must_use]
    pub fn nostr_npub(&self) -> Option<&'a str> {
        self.identity.nostr_npub.as_deref()
    }

    #[must_use]
    pub fn website_url(&self) -> Option<&'a str> {
        self.identity.website_url.as_deref()
    }

    #[must_use]
    pub fn actions(&self, target: EntityActionTarget) -> Vec<EntityActionVm> {
        let mut actions = Vec::new();
        if self.website_url().is_some() {
            actions.push(EntityActionVm::new(
                EntityActionKind::OpenWebsite,
                target.clone(),
                "Website",
                EntityActionTone::Quiet,
            ));
        }
        if self.nostr_npub().is_some() {
            actions.push(EntityActionVm::new(
                EntityActionKind::CopyNostr,
                target,
                "Copy Nostr",
                EntityActionTone::Quiet,
            ));
        }
        actions
    }
}

pub struct ReleaseDetailVm<'a> {
    view: &'a FeedView,
    context: EntitySurfaceContext,
}

impl<'a> ReleaseDetailVm<'a> {
    #[must_use]
    pub const fn new(view: &'a FeedView, context: EntitySurfaceContext) -> Self {
        Self { view, context }
    }

    #[must_use]
    pub fn header(&self) -> EntityHeaderVm<'a> {
        EntityHeaderVm {
            kind: EntitySurfaceKind::Feed,
            title: self.title(),
            subtitle: self.view.artist.clone(),
            artwork: self.view.artwork.as_ref(),
            identity: IdentityLinksVm::new(&self.view.identity),
        }
    }

    #[must_use]
    pub fn title(&self) -> String {
        self.view
            .title
            .clone()
            .unwrap_or_else(|| "Unknown Feed".to_string())
    }

    #[must_use]
    pub fn detail_rows(&self) -> Vec<EntityDetailRow> {
        let mut rows = Vec::with_capacity(5);
        rows.push(EntityDetailRow {
            key: "Release Kind",
            value: self
                .view
                .release_kind
                .clone()
                .unwrap_or_else(|| "Unknown".to_string()),
        });
        if let Some(publisher) = nonempty(self.view.publisher_text.as_deref()) {
            rows.push(EntityDetailRow {
                key: "Publisher",
                value: publisher.to_string(),
            });
        }
        if let Some(language) = nonempty(self.view.language.as_deref()) {
            rows.push(EntityDetailRow {
                key: "Language",
                value: language.to_string(),
            });
        }
        if self.view.explicit == Some(true) {
            rows.push(EntityDetailRow {
                key: "Explicit",
                value: "Yes".to_string(),
            });
        }
        if let Some(count) = self.view.episode_count {
            rows.push(EntityDetailRow {
                key: "Tracks",
                value: count.to_string(),
            });
        }
        rows
    }

    #[must_use]
    pub fn identity_actions(&self) -> Vec<EntityActionVm> {
        self.view.id.clone().map_or_else(Vec::new, |id| {
            IdentityLinksVm::new(&self.view.identity).actions(EntityActionTarget::Feed(id))
        })
    }

    #[must_use]
    pub fn contributors(&self) -> ContributorListVm<'a> {
        ContributorListVm::new(&self.view.contributors)
    }

    #[must_use]
    pub fn track_list(&self) -> TrackListVm<'a> {
        TrackListVm::new(&self.view.tracks, self.context)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContributorRowVm<'a> {
    contributor: &'a ContributorView,
}

impl<'a> ContributorRowVm<'a> {
    #[must_use]
    pub const fn new(contributor: &'a ContributorView) -> Self {
        Self { contributor }
    }

    #[must_use]
    pub fn display_name(&self) -> String {
        self.contributor
            .name
            .clone()
            .unwrap_or_else(|| "Unknown".to_string())
    }

    #[must_use]
    pub fn role_suffix(&self) -> String {
        self.contributor
            .role
            .as_ref()
            .map_or_else(String::new, |role| format!(" ({role})"))
    }

    #[must_use]
    pub fn full_label(&self) -> String {
        format!("{}{}", self.display_name(), self.role_suffix())
    }

    #[must_use]
    pub fn href(&self) -> Option<&'a str> {
        self.contributor.href.as_deref()
    }

    #[must_use]
    pub fn image_url(&self) -> Option<&'a str> {
        self.contributor.image_url.as_deref()
    }

    #[must_use]
    pub fn nostr_npub(&self) -> Option<&'a str> {
        self.contributor.nostr_npub.as_deref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContributorGroupVm<'a> {
    pub group: Option<String>,
    pub contributors: Vec<ContributorRowVm<'a>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContributorListVm<'a> {
    contributors: &'a [ContributorView],
}

impl<'a> ContributorListVm<'a> {
    #[must_use]
    pub const fn new(contributors: &'a [ContributorView]) -> Self {
        Self { contributors }
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.contributors.is_empty()
    }

    #[must_use]
    pub fn groups(&self) -> Vec<ContributorGroupVm<'a>> {
        let mut grouped = BTreeMap::<Option<String>, Vec<ContributorRowVm<'a>>>::new();
        for contributor in self.contributors {
            grouped
                .entry(nonempty(contributor.group_name.as_deref()).map(str::to_string))
                .or_default()
                .push(ContributorRowVm::new(contributor));
        }
        grouped
            .into_iter()
            .map(|(group, contributors)| ContributorGroupVm {
                group,
                contributors,
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TrackListVm<'a> {
    tracks: &'a [TrackView],
    context: EntitySurfaceContext,
}

impl<'a> TrackListVm<'a> {
    #[must_use]
    pub const fn new(tracks: &'a [TrackView], context: EntitySurfaceContext) -> Self {
        Self { tracks, context }
    }

    #[must_use]
    pub fn rows(&self) -> Vec<SharedTrackRowVm<'a>> {
        self.tracks
            .iter()
            .map(|track| SharedTrackRowVm::new(track, self.context))
            .collect()
    }

    #[must_use]
    pub fn summary(&self) -> String {
        let count = self.tracks.len();
        let total: i32 = self
            .tracks
            .iter()
            .filter_map(|track| track.duration_secs)
            .sum();
        if total > 0 {
            format!("{count} total · {}", fmt_runtime(total))
        } else {
            format!("{count} total")
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SharedTrackRowVm<'a> {
    track: &'a TrackView,
    context: EntitySurfaceContext,
}

impl<'a> SharedTrackRowVm<'a> {
    #[must_use]
    pub const fn new(track: &'a TrackView, context: EntitySurfaceContext) -> Self {
        Self { track, context }
    }

    #[must_use]
    pub fn title(&self) -> String {
        self.track
            .title
            .clone()
            .or_else(|| self.track.track_guid.clone())
            .unwrap_or_else(|| "Untitled".to_string())
    }

    #[must_use]
    pub fn number_label(&self) -> String {
        self.track
            .track_number
            .map_or_else(|| "·".to_string(), |number| number.to_string())
    }

    #[must_use]
    pub fn duration_display(&self) -> Option<String> {
        self.track.duration_secs.map(fmt_dur)
    }

    #[must_use]
    pub fn actions(&self) -> Vec<EntityActionVm> {
        self.actions_with_state(TrackActionState::for_context(self.context))
    }

    #[must_use]
    pub fn actions_with_state(&self, state: TrackActionState) -> Vec<EntityActionVm> {
        let Some(id) = self.track.id.clone() else {
            return Vec::new();
        };
        state.actions(EntityActionTarget::Track(id))
    }
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::{EntityIdentityLinks, IdentityIdFact, IdentityLinkFact};

    fn feed_view() -> FeedView {
        FeedView {
            id: Some(FeedRef::Musicindex("feed-guid".into())),
            title: Some("Release".into()),
            artist: Some("Artist".into()),
            release_kind: Some("album".into()),
            publisher_text: Some("Publisher".into()),
            language: Some("en".into()),
            explicit: Some(true),
            episode_count: Some(2),
            identity: EntityIdentityLinks::from_source_facts(
                Some("https://example.test/art.jpg".into()),
                vec![IdentityLinkFact {
                    link_type: Some("website".into()),
                    url: Some("https://example.test".into()),
                    ..IdentityLinkFact::default()
                }],
                vec![IdentityIdFact {
                    scheme: Some("nostr_npub".into()),
                    value: Some("npub1artist".into()),
                    ..IdentityIdFact::default()
                }],
            ),
            contributors: vec![
                ContributorView {
                    name: Some("Alice".into()),
                    role: Some("vocals".into()),
                    group_name: Some("Band".into()),
                    href: Some("https://example.test/alice".into()),
                    image_url: Some("https://example.test/alice.jpg".into()),
                    nostr_npub: Some("npub1alice".into()),
                },
                ContributorView {
                    name: Some("Bob".into()),
                    role: Some("drums".into()),
                    group_name: Some("Band".into()),
                    ..ContributorView::default()
                },
            ],
            tracks: vec![
                TrackView {
                    id: Some(TrackRef::Musicindex("track-1".into())),
                    track_guid: Some("track-1".into()),
                    title: Some("One".into()),
                    track_number: Some(1),
                    duration_secs: Some(65),
                    ..TrackView::default()
                },
                TrackView {
                    id: Some(TrackRef::Musicindex("track-2".into())),
                    track_guid: Some("track-2".into()),
                    title: Some("Two".into()),
                    track_number: Some(2),
                    duration_secs: Some(120),
                    ..TrackView::default()
                },
            ],
            ..FeedView::default()
        }
    }

    #[test]
    fn release_header_projects_title_subtitle_and_identity() {
        let feed = feed_view();
        let vm = ReleaseDetailVm::new(&feed, EntitySurfaceContext::Discover);
        let header = vm.header();

        assert_eq!(header.kind, EntitySurfaceKind::Feed);
        assert_eq!(header.title, "Release");
        assert_eq!(header.subtitle.as_deref(), Some("Artist"));
        assert!(header.identity.has_any());
    }

    #[test]
    fn release_detail_rows_filter_missing_values() {
        let feed = feed_view();
        let rows = ReleaseDetailVm::new(&feed, EntitySurfaceContext::Discover).detail_rows();

        assert_eq!(
            rows.iter().map(|row| row.key).collect::<Vec<_>>(),
            vec![
                "Release Kind",
                "Publisher",
                "Language",
                "Explicit",
                "Tracks"
            ]
        );
    }

    #[test]
    fn identity_actions_include_website_and_nostr() {
        let feed = feed_view();
        let actions =
            ReleaseDetailVm::new(&feed, EntitySurfaceContext::Discover).identity_actions();

        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].kind, EntityActionKind::OpenWebsite);
        assert_eq!(actions[1].kind, EntityActionKind::CopyNostr);
        assert!(actions.iter().all(|action| action.enabled));
    }

    #[test]
    fn contributor_list_groups_by_group_name() {
        let feed = feed_view();
        let groups = ReleaseDetailVm::new(&feed, EntitySurfaceContext::Discover)
            .contributors()
            .groups();

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].group.as_deref(), Some("Band"));
        assert_eq!(groups[0].contributors.len(), 2);
        assert_eq!(groups[0].contributors[0].full_label(), "Alice (vocals)");
        assert_eq!(
            groups[0].contributors[0].href(),
            Some("https://example.test/alice")
        );
        assert_eq!(
            groups[0].contributors[0].image_url(),
            Some("https://example.test/alice.jpg")
        );
        assert_eq!(groups[0].contributors[0].nostr_npub(), Some("npub1alice"));
    }

    #[test]
    fn track_list_summary_rolls_up_count_and_duration() {
        let feed = feed_view();
        let track_list = ReleaseDetailVm::new(&feed, EntitySurfaceContext::Discover).track_list();

        assert_eq!(track_list.summary(), "2 total · 3 min");
        assert_eq!(
            track_list.rows()[0].duration_display().as_deref(),
            Some("1:05")
        );
    }

    #[test]
    fn track_actions_change_by_context_without_changing_layout_contract() {
        let feed = feed_view();
        let discover_actions = ReleaseDetailVm::new(&feed, EntitySurfaceContext::Discover)
            .track_list()
            .rows()[0]
            .actions();
        let library_actions = ReleaseDetailVm::new(&feed, EntitySurfaceContext::Library)
            .track_list()
            .rows()[0]
            .actions();

        assert_eq!(discover_actions[0].kind, EntityActionKind::Download);
        assert_eq!(discover_actions[0].tone, EntityActionTone::Secondary);
        assert_eq!(library_actions[0].kind, EntityActionKind::Remove);
        assert_eq!(library_actions[0].tone, EntityActionTone::DestructiveQuiet);
        assert_eq!(discover_actions[1].kind, EntityActionKind::AddToPlaylist);
        assert_eq!(library_actions[1].kind, EntityActionKind::AddToPlaylist);
        assert_eq!(discover_actions[1].label, "+ Playlist");
        assert_eq!(library_actions[1].label, "+ Playlist");
    }

    #[test]
    fn track_action_state_projects_busy_and_disabled_membership_actions() {
        let target = EntityActionTarget::Track(TrackRef::Musicindex("track-1".into()));
        let remote_unavailable = TrackActionState::new(
            TrackMembershipState::RemoteOnly,
            PlaylistActionState::Hidden,
        )
        .with_download_available(false)
        .primary_action(target.clone());

        assert_eq!(remote_unavailable.kind, EntityActionKind::Download);
        assert_eq!(remote_unavailable.label, "Download");
        assert!(!remote_unavailable.enabled);

        let downloading = TrackActionState::new(
            TrackMembershipState::Downloading,
            PlaylistActionState::Hidden,
        )
        .primary_action(target.clone());
        assert_eq!(downloading.kind, EntityActionKind::Download);
        assert_eq!(downloading.label, "Downloading...");
        assert!(!downloading.enabled);

        let removing =
            TrackActionState::new(TrackMembershipState::Removing, PlaylistActionState::Hidden)
                .primary_action(target);
        assert_eq!(removing.kind, EntityActionKind::Remove);
        assert_eq!(removing.label, "Removing...");
        assert_eq!(removing.tone, EntityActionTone::DestructiveQuiet);
        assert!(!removing.enabled);
    }

    #[test]
    fn track_action_state_projects_playlist_open_state() {
        let target = EntityActionTarget::Track(TrackRef::Musicindex("track-1".into()));
        let closed =
            TrackActionState::new(TrackMembershipState::InLibrary, PlaylistActionState::Closed)
                .playlist_action(target.clone())
                .expect("closed playlist action should render");
        let open =
            TrackActionState::new(TrackMembershipState::InLibrary, PlaylistActionState::Open)
                .playlist_action(target)
                .expect("open playlist action should render");

        assert_eq!(closed.label, "+ Playlist");
        assert_eq!(open.label, "+ Playlist ▴");
        assert_eq!(closed.tone, EntityActionTone::Quiet);
    }

    #[test]
    fn empty_identity_has_no_actions() {
        let identity = EntityIdentityLinks::default();
        let vm = IdentityLinksVm::new(&identity);

        assert!(!vm.has_any());
        assert!(vm
            .actions(EntityActionTarget::Contributor("Alice".into()))
            .is_empty());
    }
}
