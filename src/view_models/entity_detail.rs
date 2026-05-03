//! Shared entity-detail projections.
//!
//! This module formats source-normalized [`crate::views`] facts into plain
//! display data. Screens bind the resulting values to GPUI controls elsewhere.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;

use crate::view_models::format::{fmt_date, fmt_runtime};
use crate::view_models::track::fmt_dur;
use crate::view_models::{ActionStatusMessageDisplay, ActionStatusMessageWidth};
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
    OpenRss,
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
pub enum IdentityActionDisplayKind {
    Website,
    Nostr,
    Rss,
}

impl IdentityActionDisplayKind {
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Website => "website",
            Self::Nostr => "nostr",
            Self::Rss => "rss",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityActionDisplay {
    pub id: String,
    pub kind: IdentityActionDisplayKind,
    pub payload: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrackMembershipState {
    RemoteOnly,
    Downloading,
    InLibrary,
    Removing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseMembershipState {
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
pub enum MetadataPanelState {
    Hidden,
    Loading,
    Loaded,
    Empty,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrackMetadataActionState {
    pub context: EntitySurfaceContext,
    pub compare: MetadataPanelState,
    pub musicbrainz: MetadataPanelState,
    pub has_local_file: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagedId3EditsDisplay {
    pub message: ActionStatusMessageDisplay,
    pub apply_label: String,
    pub apply_enabled: bool,
    pub conflict_message: Option<ActionStatusMessageDisplay>,
    pub discard_label: &'static str,
    pub show_discard: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MetadataFileActionsDisplay {
    pub reread_label: &'static str,
    pub redownload_label: &'static str,
}

impl TrackMetadataActionState {
    #[must_use]
    pub const fn new(
        context: EntitySurfaceContext,
        compare: MetadataPanelState,
        musicbrainz: MetadataPanelState,
        has_local_file: bool,
    ) -> Self {
        Self {
            context,
            compare,
            musicbrainz,
            has_local_file,
        }
    }

    #[must_use]
    pub const fn show_compare_panel(&self) -> bool {
        !matches!(self.compare, MetadataPanelState::Hidden)
    }

    #[must_use]
    pub const fn show_musicbrainz_panel(&self) -> bool {
        !matches!(self.musicbrainz, MetadataPanelState::Hidden)
    }

    #[must_use]
    pub const fn compare_panel_loading_message() -> &'static str {
        "Reading embedded metadata..."
    }

    #[must_use]
    pub const fn musicbrainz_panel_loading_message() -> &'static str {
        "Searching MusicBrainz..."
    }

    #[must_use]
    pub const fn file_actions_display() -> MetadataFileActionsDisplay {
        MetadataFileActionsDisplay {
            reread_label: "Re-read",
            redownload_label: "Re-download",
        }
    }

    #[must_use]
    pub fn duplicate_id3_target_message(conflicts: &[String]) -> String {
        format!(
            "Resolve duplicate ID3 target{}: {}",
            if conflicts.len() == 1 { "" } else { "s" },
            conflicts.join("; ")
        )
    }

    #[must_use]
    pub fn id3_apply_error_message(error: impl std::fmt::Display) -> String {
        format!("Error applying ID3 edits: {error}")
    }

    #[must_use]
    pub fn compare_action(&self, target: EntityActionTarget) -> Option<EntityActionVm> {
        if self.context != EntitySurfaceContext::Library || !self.has_local_file {
            return None;
        }

        let action = match self.compare {
            MetadataPanelState::Loaded => EntityActionVm::new(
                EntityActionKind::CompareMetadata,
                target,
                "Hide Compare",
                EntityActionTone::Quiet,
            ),
            MetadataPanelState::Loading => EntityActionVm::new(
                EntityActionKind::CompareMetadata,
                target,
                "Reading ID3...",
                EntityActionTone::Quiet,
            )
            .disabled(),
            MetadataPanelState::Empty | MetadataPanelState::Hidden => EntityActionVm::new(
                EntityActionKind::CompareMetadata,
                target,
                "Compare ID3",
                EntityActionTone::Quiet,
            ),
        };
        Some(action)
    }

    #[must_use]
    pub fn musicbrainz_action(&self, target: EntityActionTarget) -> Option<EntityActionVm> {
        if self.context != EntitySurfaceContext::Library || !self.has_local_file {
            return None;
        }

        let action = match self.musicbrainz {
            MetadataPanelState::Loaded => EntityActionVm::new(
                EntityActionKind::OpenMusicBrainz,
                target,
                "Hide MusicBrainz",
                EntityActionTone::Quiet,
            ),
            MetadataPanelState::Loading => EntityActionVm::new(
                EntityActionKind::OpenMusicBrainz,
                target,
                "Searching MusicBrainz...",
                EntityActionTone::Quiet,
            )
            .disabled(),
            MetadataPanelState::Empty | MetadataPanelState::Hidden => EntityActionVm::new(
                EntityActionKind::OpenMusicBrainz,
                target,
                "MusicBrainz",
                EntityActionTone::Quiet,
            ),
        };
        Some(action)
    }

    #[must_use]
    pub fn actions(&self, target: EntityActionTarget) -> Vec<EntityActionVm> {
        let mut actions = Vec::new();
        if let Some(action) = self.compare_action(target.clone()) {
            actions.push(action);
        }
        if let Some(action) = self.musicbrainz_action(target) {
            actions.push(action);
        }
        actions
    }

    #[must_use]
    pub fn staged_id3_edits_display(
        &self,
        count: usize,
        applying: bool,
        conflict_text: Option<&str>,
    ) -> Option<StagedId3EditsDisplay> {
        if self.context != EntitySurfaceContext::Library || (count == 0 && !applying) {
            return None;
        }

        let has_conflicts = conflict_text.is_some_and(|text| !text.trim().is_empty());
        Some(StagedId3EditsDisplay {
            message: ActionStatusMessageDisplay::neutral(format!(
                "{count} staged tag edit{}",
                if count == 1 { "" } else { "s" }
            )),
            apply_label: if applying {
                "Applying tags...".into()
            } else {
                format!("Apply tags ({count})")
            },
            apply_enabled: !applying && !has_conflicts,
            conflict_message: conflict_text
                .filter(|text| !text.trim().is_empty())
                .map(|text| {
                    ActionStatusMessageDisplay::danger(
                        format!("Duplicate target: {text}"),
                        ActionStatusMessageWidth::Conflict,
                    )
                }),
            discard_label: "Discard staged",
            show_discard: !applying && count > 0,
        })
    }

    #[must_use]
    pub fn id3_apply_error_display(message: impl Into<String>) -> ActionStatusMessageDisplay {
        ActionStatusMessageDisplay::danger(message, ActionStatusMessageWidth::Action)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReleaseActionState {
    pub membership: ReleaseMembershipState,
    pub playlist: PlaylistActionState,
}

impl ReleaseActionState {
    #[must_use]
    pub const fn new(membership: ReleaseMembershipState, playlist: PlaylistActionState) -> Self {
        Self {
            membership,
            playlist,
        }
    }

    #[must_use]
    pub const fn for_context(context: EntitySurfaceContext) -> Self {
        match context {
            EntitySurfaceContext::Discover => Self::new(
                ReleaseMembershipState::RemoteOnly,
                PlaylistActionState::Closed,
            ),
            EntitySurfaceContext::Library => Self::new(
                ReleaseMembershipState::InLibrary,
                PlaylistActionState::Closed,
            ),
        }
    }

    #[must_use]
    pub fn primary_action(&self, target: EntityActionTarget) -> EntityActionVm {
        match self.membership {
            ReleaseMembershipState::RemoteOnly => EntityActionVm::new(
                EntityActionKind::Download,
                target,
                "Download Feed",
                EntityActionTone::Secondary,
            ),
            ReleaseMembershipState::Downloading => EntityActionVm::new(
                EntityActionKind::Download,
                target,
                "Downloading...",
                EntityActionTone::Secondary,
            )
            .disabled(),
            ReleaseMembershipState::InLibrary => EntityActionVm::new(
                EntityActionKind::Remove,
                target,
                "Remove Feed",
                EntityActionTone::DestructiveQuiet,
            ),
            ReleaseMembershipState::Removing => EntityActionVm::new(
                EntityActionKind::Remove,
                target,
                "Removing...",
                EntityActionTone::DestructiveQuiet,
            )
            .disabled(),
        }
    }

    #[must_use]
    pub fn playlist_action(&self, target: EntityActionTarget) -> Option<EntityActionVm> {
        match self.playlist {
            PlaylistActionState::Hidden => None,
            PlaylistActionState::Closed => Some(EntityActionVm::new(
                EntityActionKind::AddToPlaylist,
                target,
                "Add feed to playlist ▾",
                EntityActionTone::Quiet,
            )),
            PlaylistActionState::Open => Some(EntityActionVm::new(
                EntityActionKind::AddToPlaylist,
                target,
                "Add feed to playlist ▴",
                EntityActionTone::Quiet,
            )),
        }
    }

    #[must_use]
    pub fn actions(&self, target: EntityActionTarget) -> Vec<EntityActionVm> {
        let primary = self.primary_action(target.clone());
        let mut actions = vec![primary];
        if let Some(action) = self.playlist_action(target) {
            actions.push(action);
        }
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
    pub payload: Option<String>,
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
            payload: None,
        }
    }

    #[must_use]
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    #[must_use]
    pub fn with_payload(mut self, payload: impl Into<String>) -> Self {
        self.payload = Some(payload.into());
        self
    }

    #[must_use]
    pub fn identity_display(&self, id_prefix: &str) -> Option<IdentityActionDisplay> {
        let payload = self.payload.as_ref()?;
        let kind = match self.kind {
            EntityActionKind::OpenWebsite => IdentityActionDisplayKind::Website,
            EntityActionKind::CopyNostr => IdentityActionDisplayKind::Nostr,
            EntityActionKind::OpenRss => IdentityActionDisplayKind::Rss,
            EntityActionKind::Download
            | EntityActionKind::Remove
            | EntityActionKind::AddToPlaylist
            | EntityActionKind::Play
            | EntityActionKind::CompareMetadata
            | EntityActionKind::OpenMusicBrainz => return None,
        };
        Some(IdentityActionDisplay {
            id: format!("{id_prefix}-{}:{payload}", kind.slug()),
            kind,
            payload: payload.clone(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityDetailRow {
    pub key: &'static str,
    pub value: String,
}

#[derive(Clone, Debug)]
pub struct ReleaseDetailPageVm<'a> {
    pub hero: ReleaseHeroVm<'a>,
    pub primary_actions: Vec<EntityActionVm>,
    pub identity_actions: Vec<EntityActionVm>,
    pub summary_facts: Vec<ReleaseFactVm>,
    pub panels: Vec<ReleasePanelVm>,
    pub tracks: TrackListVm<'a>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReleaseHeroVm<'a> {
    pub kind: EntitySurfaceKind,
    pub artwork: Option<&'a ArtworkRef>,
    pub title: &'a str,
    pub subtitle: Option<&'a str>,
    pub supporting_line: Option<&'a str>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseFactVm {
    pub key: &'static str,
    pub value: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleasePanelKind {
    Description,
    Identity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleasePanelVm {
    pub kind: ReleasePanelKind,
    pub title: &'static str,
    pub body: Option<String>,
    pub rows: Vec<EntityDetailRow>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContributorPanelDisplay {
    pub id: &'static str,
    pub title: &'static str,
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
        if let Some(url) = self.website_url() {
            actions.push(
                EntityActionVm::new(
                    EntityActionKind::OpenWebsite,
                    target.clone(),
                    "Website",
                    EntityActionTone::Quiet,
                )
                .with_payload(url),
            );
        }
        if let Some(npub) = self.nostr_npub() {
            actions.push(
                EntityActionVm::new(
                    EntityActionKind::CopyNostr,
                    target,
                    "Copy Nostr",
                    EntityActionTone::Quiet,
                )
                .with_payload(npub),
            );
        }
        actions
    }
}

pub struct ReleaseDetailVm<'a> {
    view: &'a FeedView,
    context: EntitySurfaceContext,
}

const MAX_RELEASE_SUMMARY_FACTS: usize = 5;

impl<'a> ReleaseDetailVm<'a> {
    #[must_use]
    pub const fn new(view: &'a FeedView, context: EntitySurfaceContext) -> Self {
        Self { view, context }
    }

    #[must_use]
    pub fn page(&self) -> ReleaseDetailPageVm<'a> {
        ReleaseDetailPageVm {
            hero: self.hero(),
            primary_actions: self.actions(),
            identity_actions: self.identity_actions(),
            summary_facts: self.summary_facts(),
            panels: self.panels(),
            tracks: self.track_list(),
        }
    }

    #[must_use]
    pub fn hero(&self) -> ReleaseHeroVm<'a> {
        ReleaseHeroVm {
            kind: EntitySurfaceKind::Feed,
            artwork: self.view.artwork.as_ref(),
            title: self
                .view
                .title
                .as_deref()
                .and_then(hero_text)
                .unwrap_or("Unknown Feed"),
            subtitle: self.view.artist.as_deref().and_then(hero_text),
            supporting_line: self.view.publisher_text.as_deref().and_then(hero_text),
        }
    }

    #[must_use]
    pub fn summary_facts(&self) -> Vec<ReleaseFactVm> {
        let mut facts = Vec::with_capacity(MAX_RELEASE_SUMMARY_FACTS);
        facts.push(ReleaseFactVm {
            key: "Release Kind",
            value: self
                .view
                .release_kind
                .clone()
                .unwrap_or_else(|| "Unknown".to_string()),
        });
        if let Some(date) = self.view.release_date.and_then(fmt_date) {
            facts.push(ReleaseFactVm {
                key: "Release Date",
                value: date,
            });
        }
        if let Some(count) = self.view.episode_count {
            facts.push(ReleaseFactVm {
                key: "Tracks",
                value: count.to_string(),
            });
        }
        if let Some(duration) = self.total_duration_fact() {
            facts.push(ReleaseFactVm {
                key: "Duration",
                value: duration,
            });
        }
        if let Some(language) = nonempty(self.view.language.as_deref()) {
            facts.push(ReleaseFactVm {
                key: "Language",
                value: language.to_string(),
            });
        }
        if self.view.explicit == Some(true) {
            facts.push(ReleaseFactVm {
                key: "Explicit",
                value: "Yes".to_string(),
            });
        }
        facts.truncate(MAX_RELEASE_SUMMARY_FACTS);
        facts
    }

    #[must_use]
    pub fn panels(&self) -> Vec<ReleasePanelVm> {
        let mut panels = Vec::with_capacity(2);
        if let Some(description) = nonempty(self.view.description.as_deref()) {
            panels.push(ReleasePanelVm {
                kind: ReleasePanelKind::Description,
                title: "Description",
                body: Some(description.to_string()),
                rows: Vec::new(),
            });
        }

        let identity_rows = self.identity_panel_rows();
        if !identity_rows.is_empty() {
            panels.push(ReleasePanelVm {
                kind: ReleasePanelKind::Identity,
                title: "Identity",
                body: None,
                rows: identity_rows,
            });
        }
        panels
    }

    #[must_use]
    pub fn title(&self) -> String {
        self.view
            .title
            .clone()
            .unwrap_or_else(|| "Unknown Feed".to_string())
    }

    #[must_use]
    pub fn identity_actions(&self) -> Vec<EntityActionVm> {
        let Some(id) = self.view.id.clone() else {
            return Vec::new();
        };

        let target = EntityActionTarget::Feed(id);
        let mut actions = IdentityLinksVm::new(&self.view.identity).actions(target.clone());
        if let Some(feed_url) = nonempty(self.view.feed_url.as_deref()) {
            actions.push(
                EntityActionVm::new(
                    EntityActionKind::OpenRss,
                    target,
                    "RSS",
                    EntityActionTone::Quiet,
                )
                .with_payload(feed_url),
            );
        }
        actions
    }

    #[must_use]
    pub fn actions(&self) -> Vec<EntityActionVm> {
        self.actions_with_state(ReleaseActionState::for_context(self.context))
    }

    #[must_use]
    pub fn actions_with_state(&self, state: ReleaseActionState) -> Vec<EntityActionVm> {
        self.view
            .id
            .clone()
            .map_or_else(Vec::new, |id| state.actions(EntityActionTarget::Feed(id)))
    }

    #[must_use]
    pub fn contributors(&self) -> ContributorListVm<'a> {
        ContributorListVm::new(&self.view.contributors)
    }

    #[must_use]
    pub const fn contributor_panel_display(&self) -> ContributorPanelDisplay {
        ContributorPanelDisplay {
            id: match self.context {
                EntitySurfaceContext::Discover => "discover-contributors",
                EntitySurfaceContext::Library => "library-contributors",
            },
            title: "Contributors",
        }
    }

    #[must_use]
    pub fn track_list(&self) -> TrackListVm<'a> {
        TrackListVm::new(&self.view.tracks, self.context)
    }

    #[must_use]
    fn total_duration_fact(&self) -> Option<String> {
        let total = self
            .view
            .tracks
            .iter()
            .filter_map(|track| track.duration_secs)
            .sum::<i32>();
        (total > 0).then(|| fmt_runtime(total))
    }

    #[must_use]
    fn identity_panel_rows(&self) -> Vec<EntityDetailRow> {
        let mut rows = Vec::with_capacity(4);
        if let Some(website) = nonempty(self.view.identity.website_url.as_deref()) {
            rows.push(EntityDetailRow {
                key: "Website",
                value: website.to_string(),
            });
        }
        if let Some(npub) = nonempty(self.view.identity.nostr_npub.as_deref()) {
            rows.push(EntityDetailRow {
                key: "Nostr",
                value: npub.to_string(),
            });
        }
        if let Some(feed_url) = nonempty(self.view.feed_url.as_deref()) {
            rows.push(EntityDetailRow {
                key: "Feed URL",
                value: feed_url.to_string(),
            });
        }
        if let Some(guid) = nonempty(self.view.feed_guid.as_deref()) {
            rows.push(EntityDetailRow {
                key: "GUID",
                value: guid.to_string(),
            });
        }
        rows
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContributorRowVm<'a> {
    contributor: &'a ContributorView,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContributorIdentityActionKind {
    Website,
    Nostr,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContributorIdentityActionDisplay {
    pub id: String,
    pub kind: ContributorIdentityActionKind,
    pub target: String,
}

impl<'a> ContributorRowVm<'a> {
    #[must_use]
    pub const fn new(contributor: &'a ContributorView) -> Self {
        Self { contributor }
    }

    #[must_use]
    pub fn display_name(&self) -> String {
        nonempty(self.contributor.name.as_deref())
            .map_or_else(|| "Unknown".to_string(), str::to_string)
    }

    #[must_use]
    pub fn role_suffix(&self) -> String {
        self.role_label()
            .map_or_else(String::new, |role| format!(" ({role})"))
    }

    #[must_use]
    pub fn role_label(&self) -> Option<String> {
        nonempty(self.contributor.role.as_deref()).map(str::to_string)
    }

    #[must_use]
    pub fn full_label(&self) -> String {
        format!("{}{}", self.display_name(), self.role_suffix())
    }

    #[must_use]
    pub fn identity_actions(&self, id_prefix: &str) -> Vec<ContributorIdentityActionDisplay> {
        let label = self.full_label();
        let mut actions = Vec::new();
        if let Some(href) = self.href() {
            actions.push(ContributorIdentityActionDisplay {
                id: format!("{id_prefix}-website:{label}:{href}"),
                kind: ContributorIdentityActionKind::Website,
                target: href.to_string(),
            });
        }
        if let Some(npub) = self.nostr_npub() {
            actions.push(ContributorIdentityActionDisplay {
                id: format!("{id_prefix}-nostr:{label}:{npub}"),
                kind: ContributorIdentityActionKind::Nostr,
                target: npub.to_string(),
            });
        }
        actions
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
    pub fn people(&self) -> Vec<ContributorPersonVm<'a>> {
        let mut grouped = BTreeMap::<String, Vec<ContributorRowVm<'a>>>::new();
        for contributor in self.contributors {
            let row = ContributorRowVm::new(contributor);
            grouped.entry(row.display_name()).or_default().push(row);
        }
        grouped
            .into_iter()
            .map(|(name, contributors)| ContributorPersonVm { name, contributors })
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContributorPersonVm<'a> {
    name: String,
    contributors: Vec<ContributorRowVm<'a>>,
}

impl<'a> ContributorPersonVm<'a> {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn primary(&self) -> Option<&ContributorRowVm<'a>> {
        self.contributors.first()
    }

    #[must_use]
    pub fn roles(&self) -> Vec<String> {
        let mut roles = Vec::new();
        for contributor in &self.contributors {
            let role = contributor
                .role_label()
                .unwrap_or_else(|| "Contributor".to_string());
            if !roles.contains(&role) {
                roles.push(role);
            }
        }
        roles
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
    pub const fn title(&self) -> &'static str {
        "Tracks"
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

fn hero_text(value: &str) -> Option<&str> {
    let value = nonempty(Some(value))?;
    (!is_machine_text(value)).then_some(value)
}

fn is_machine_text(value: &str) -> bool {
    value.contains('\n')
        || value.contains('\r')
        || is_raw_url(value)
        || value.to_ascii_lowercase().starts_with("npub1")
        || is_long_machine_identifier(value)
}

fn is_raw_url(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("://") || lower.starts_with("www.")
}

fn is_long_machine_identifier(value: &str) -> bool {
    value.len() >= 32
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::{EntityIdentityLinks, IdentityIdFact, IdentityLinkFact};

    fn feed_view() -> FeedView {
        FeedView {
            id: Some(FeedRef::Musicindex("feed-guid".into())),
            title: Some("Release".into()),
            feed_url: Some("https://feeds.example.test/rss.xml".into()),
            artist: Some("Artist".into()),
            release_kind: Some("album".into()),
            publisher_text: Some("Publisher".into()),
            description: Some("Release description".into()),
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
    fn release_page_contract_exposes_canonical_zones() {
        let feed = feed_view();
        let page = ReleaseDetailVm::new(&feed, EntitySurfaceContext::Discover).page();

        assert_eq!(page.hero.kind, EntitySurfaceKind::Feed);
        assert_eq!(page.hero.title, "Release");
        assert_eq!(page.primary_actions.len(), 2);
        assert_eq!(page.identity_actions.len(), 3);
        assert!(!page.summary_facts.is_empty());
        assert!(!page.panels.is_empty());
        assert_eq!(page.tracks.rows().len(), 2);
    }

    #[test]
    fn release_page_hero_excludes_machine_values_and_descriptions() {
        let mut feed = feed_view();
        let long_guid = "0123456789abcdef0123456789abcdef".to_string();
        feed.feed_guid = Some(long_guid.clone());
        feed.feed_url = Some("https://feeds.example.test/rss.xml".into());
        feed.title = Some("https://example.test/release".into());
        feed.artist = Some("npub1artistmachinevalue".into());
        feed.publisher_text = Some(long_guid.clone());
        feed.description = Some("First line\nSecond line".into());

        let page = ReleaseDetailVm::new(&feed, EntitySurfaceContext::Discover).page();
        let hero_text = [
            Some(page.hero.title),
            page.hero.subtitle,
            page.hero.supporting_line,
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");

        assert_eq!(page.hero.title, "Unknown Feed");
        assert!(!hero_text.contains("https://"));
        assert!(!hero_text.contains("npub1"));
        assert!(!hero_text.contains(&long_guid));
        assert!(!hero_text.contains("First line"));
        assert!(!hero_text.contains('\n'));
        assert!(page.panels.iter().any(|panel| {
            panel.kind == ReleasePanelKind::Description
                && panel.body.as_deref() == Some("First line\nSecond line")
        }));
        assert!(page.panels.iter().any(|panel| {
            panel.kind == ReleasePanelKind::Identity
                && panel.rows.iter().any(|row| row.value == long_guid)
        }));
    }

    #[test]
    fn release_page_summary_facts_are_ordered_and_capped() {
        let mut feed = feed_view();
        feed.release_date = Some(1_712_275_200);

        let facts = ReleaseDetailVm::new(&feed, EntitySurfaceContext::Discover).summary_facts();

        assert_eq!(facts.len(), 5);
        assert_eq!(
            facts
                .iter()
                .map(|fact| (fact.key, fact.value.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("Release Kind", "album"),
                ("Release Date", "Apr 5, 2024"),
                ("Tracks", "2"),
                ("Duration", "3 min"),
                ("Language", "en"),
            ]
        );
    }

    #[test]
    fn release_page_places_description_in_one_panel_only() {
        let feed = feed_view();
        let page = ReleaseDetailVm::new(&feed, EntitySurfaceContext::Discover).page();
        let description_panels = page
            .panels
            .iter()
            .filter(|panel| panel.kind == ReleasePanelKind::Description)
            .collect::<Vec<_>>();

        assert_eq!(description_panels.len(), 1);
        assert_eq!(
            description_panels[0].body.as_deref(),
            Some("Release description")
        );
        assert_ne!(page.hero.title, "Release description");
        assert_ne!(page.hero.subtitle, Some("Release description"));
        assert_ne!(page.hero.supporting_line, Some("Release description"));
        assert!(page
            .summary_facts
            .iter()
            .all(|fact| fact.value != "Release description"));
    }

    #[test]
    fn release_page_structural_zones_match_across_surfaces() {
        let feed = feed_view();
        let discover = ReleaseDetailVm::new(&feed, EntitySurfaceContext::Discover).page();
        let library = ReleaseDetailVm::new(&feed, EntitySurfaceContext::Library).page();

        assert_eq!(discover.hero.kind, library.hero.kind);
        assert_eq!(discover.hero.artwork, library.hero.artwork);
        assert_eq!(discover.hero.title, library.hero.title);
        assert_eq!(discover.hero.subtitle, library.hero.subtitle);
        assert_eq!(discover.hero.supporting_line, library.hero.supporting_line);
        assert_eq!(
            discover.primary_actions.len(),
            library.primary_actions.len()
        );
        assert_eq!(discover.identity_actions, library.identity_actions);
        assert_eq!(discover.summary_facts, library.summary_facts);
        assert_eq!(
            discover
                .panels
                .iter()
                .map(|panel| panel.kind)
                .collect::<Vec<_>>(),
            library
                .panels
                .iter()
                .map(|panel| panel.kind)
                .collect::<Vec<_>>()
        );
        assert_eq!(discover.tracks.summary(), library.tracks.summary());
        assert_eq!(discover.tracks.rows().len(), library.tracks.rows().len());
    }

    #[test]
    fn identity_actions_include_website_nostr_and_rss() {
        let feed = feed_view();
        let actions =
            ReleaseDetailVm::new(&feed, EntitySurfaceContext::Discover).identity_actions();

        assert_eq!(actions.len(), 3);
        assert_eq!(actions[0].kind, EntityActionKind::OpenWebsite);
        assert_eq!(actions[1].kind, EntityActionKind::CopyNostr);
        assert_eq!(actions[2].kind, EntityActionKind::OpenRss);
        assert!(actions.iter().all(|action| action.enabled));
    }

    #[test]
    fn identity_action_display_projects_id_kind_and_payload() {
        let feed = feed_view();
        let actions =
            ReleaseDetailVm::new(&feed, EntitySurfaceContext::Discover).identity_actions();

        assert_eq!(
            actions
                .iter()
                .filter_map(|action| action.identity_display("library-feed"))
                .collect::<Vec<_>>(),
            vec![
                IdentityActionDisplay {
                    id: "library-feed-website:https://example.test".to_string(),
                    kind: IdentityActionDisplayKind::Website,
                    payload: "https://example.test".to_string(),
                },
                IdentityActionDisplay {
                    id: "library-feed-nostr:npub1artist".to_string(),
                    kind: IdentityActionDisplayKind::Nostr,
                    payload: "npub1artist".to_string(),
                },
                IdentityActionDisplay {
                    id: "library-feed-rss:https://feeds.example.test/rss.xml".to_string(),
                    kind: IdentityActionDisplayKind::Rss,
                    payload: "https://feeds.example.test/rss.xml".to_string(),
                },
            ]
        );

        let download = EntityActionVm::new(
            EntityActionKind::Download,
            EntityActionTarget::Feed(FeedRef::Musicindex("feed-guid".into())),
            "Download",
            EntityActionTone::Primary,
        )
        .with_payload("https://example.test/audio.mp3");
        assert_eq!(download.identity_display("library-feed"), None);
        assert_eq!(
            actions[0]
                .clone()
                .disabled()
                .identity_display("library-feed"),
            actions[0].identity_display("library-feed")
        );
    }

    #[test]
    fn entity_action_vm_carries_identity_payload() {
        let feed = feed_view();
        let projection = ReleaseDetailVm::new(&feed, EntitySurfaceContext::Discover);
        let identity_actions = projection.identity_actions();

        assert_eq!(
            identity_actions
                .iter()
                .map(|action| (action.kind.clone(), action.payload.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                (EntityActionKind::OpenWebsite, Some("https://example.test")),
                (EntityActionKind::CopyNostr, Some("npub1artist")),
                (
                    EntityActionKind::OpenRss,
                    Some("https://feeds.example.test/rss.xml")
                ),
            ]
        );

        assert!(projection
            .actions()
            .iter()
            .all(|action| action.payload.is_none()));
    }

    #[test]
    fn identity_actions_are_shared_across_surface_contexts() {
        let feed = feed_view();
        let discover_actions =
            ReleaseDetailVm::new(&feed, EntitySurfaceContext::Discover).identity_actions();
        let library_actions =
            ReleaseDetailVm::new(&feed, EntitySurfaceContext::Library).identity_actions();

        assert_eq!(discover_actions, library_actions);
    }

    #[test]
    fn contributor_panel_display_projects_surface_chrome() {
        let feed = feed_view();

        assert_eq!(
            ReleaseDetailVm::new(&feed, EntitySurfaceContext::Library).contributor_panel_display(),
            ContributorPanelDisplay {
                id: "library-contributors",
                title: "Contributors",
            }
        );
        assert_eq!(
            ReleaseDetailVm::new(&feed, EntitySurfaceContext::Discover).contributor_panel_display(),
            ContributorPanelDisplay {
                id: "discover-contributors",
                title: "Contributors",
            }
        );
    }

    #[test]
    fn contributor_list_groups_roles_by_person() {
        let feed = feed_view();
        let people = ReleaseDetailVm::new(&feed, EntitySurfaceContext::Discover)
            .contributors()
            .people();

        assert_eq!(people.len(), 2);
        assert_eq!(people[0].name(), "Alice");
        assert_eq!(people[0].roles(), vec!["vocals".to_string()]);
        let alice = people[0]
            .primary()
            .expect("person group should expose a primary contributor");
        assert_eq!(alice.href(), Some("https://example.test/alice"));
        assert_eq!(alice.image_url(), Some("https://example.test/alice.jpg"));
        assert_eq!(alice.nostr_npub(), Some("npub1alice"));
        assert_eq!(people[1].name(), "Bob");
        assert_eq!(people[1].roles(), vec!["drums".to_string()]);
    }

    #[test]
    fn contributor_identity_actions_project_ids_kinds_and_targets() {
        let feed = feed_view();
        let alice = ReleaseDetailVm::new(&feed, EntitySurfaceContext::Discover)
            .contributors()
            .people()
            .into_iter()
            .next()
            .and_then(|person| person.primary().cloned())
            .expect("person group should expose a primary contributor");

        assert_eq!(
            alice.identity_actions("contributor"),
            vec![
                ContributorIdentityActionDisplay {
                    id: "contributor-website:Alice (vocals):https://example.test/alice".to_string(),
                    kind: ContributorIdentityActionKind::Website,
                    target: "https://example.test/alice".to_string(),
                },
                ContributorIdentityActionDisplay {
                    id: "contributor-nostr:Alice (vocals):npub1alice".to_string(),
                    kind: ContributorIdentityActionKind::Nostr,
                    target: "npub1alice".to_string(),
                },
            ]
        );
        assert_eq!(
            alice
                .identity_actions("library-contributor")
                .into_iter()
                .map(|action| action.id)
                .collect::<Vec<_>>(),
            vec![
                "library-contributor-website:Alice (vocals):https://example.test/alice".to_string(),
                "library-contributor-nostr:Alice (vocals):npub1alice".to_string(),
            ]
        );
    }

    #[test]
    fn contributor_identity_actions_omit_absent_targets() {
        let contributor = ContributorView {
            name: Some("Alice".into()),
            role: Some("vocals".into()),
            ..ContributorView::default()
        };
        let vm = ContributorRowVm::new(&contributor);

        assert!(vm.identity_actions("contributor").is_empty());
    }

    #[test]
    fn contributor_list_combines_multiple_roles_for_same_person() {
        let contributors = [
            ContributorView {
                name: Some("Alice".into()),
                role: Some("vocals".into()),
                ..ContributorView::default()
            },
            ContributorView {
                name: Some("Alice".into()),
                role: Some("producer".into()),
                ..ContributorView::default()
            },
            ContributorView {
                name: Some("Alice".into()),
                role: Some("vocals".into()),
                ..ContributorView::default()
            },
        ];
        let people = ContributorListVm::new(&contributors).people();

        assert_eq!(people.len(), 1);
        assert_eq!(people[0].name(), "Alice");
        assert_eq!(
            people[0].roles(),
            vec!["vocals".to_string(), "producer".to_string()]
        );
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
    fn release_actions_change_by_context_without_changing_layout_contract() {
        let feed = feed_view();
        let discover_actions =
            ReleaseDetailVm::new(&feed, EntitySurfaceContext::Discover).actions();
        let library_actions = ReleaseDetailVm::new(&feed, EntitySurfaceContext::Library).actions();

        assert_eq!(discover_actions[0].kind, EntityActionKind::Download);
        assert_eq!(discover_actions[0].label, "Download Feed");
        assert_eq!(discover_actions[0].tone, EntityActionTone::Secondary);
        assert_eq!(library_actions[0].kind, EntityActionKind::Remove);
        assert_eq!(library_actions[0].label, "Remove Feed");
        assert_eq!(library_actions[0].tone, EntityActionTone::DestructiveQuiet);
        assert_eq!(discover_actions[1].kind, EntityActionKind::AddToPlaylist);
        assert_eq!(library_actions[1].kind, EntityActionKind::AddToPlaylist);
        assert_eq!(discover_actions[1].label, "Add feed to playlist ▾");
        assert_eq!(library_actions[1].label, "Add feed to playlist ▾");
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
    fn release_action_state_projects_busy_and_playlist_open_state() {
        let target = EntityActionTarget::Feed(FeedRef::Musicindex("feed-1".into()));
        let downloading = ReleaseActionState::new(
            ReleaseMembershipState::Downloading,
            PlaylistActionState::Hidden,
        )
        .primary_action(target.clone());
        let removing = ReleaseActionState::new(
            ReleaseMembershipState::Removing,
            PlaylistActionState::Hidden,
        )
        .primary_action(target.clone());
        let open =
            ReleaseActionState::new(ReleaseMembershipState::InLibrary, PlaylistActionState::Open)
                .playlist_action(target)
                .expect("open playlist action should render");

        assert_eq!(downloading.kind, EntityActionKind::Download);
        assert_eq!(downloading.label, "Downloading...");
        assert!(!downloading.enabled);
        assert_eq!(removing.kind, EntityActionKind::Remove);
        assert_eq!(removing.label, "Removing...");
        assert_eq!(removing.tone, EntityActionTone::DestructiveQuiet);
        assert!(!removing.enabled);
        assert_eq!(open.label, "Add feed to playlist ▴");
        assert_eq!(open.tone, EntityActionTone::Quiet);
    }

    #[test]
    fn track_metadata_action_state_projects_compare_and_musicbrainz_actions() {
        let target = EntityActionTarget::Track(TrackRef::Musicindex("track-1".into()));
        let state = TrackMetadataActionState::new(
            EntitySurfaceContext::Library,
            MetadataPanelState::Hidden,
            MetadataPanelState::Loading,
            true,
        );
        let compare = state
            .compare_action(target.clone())
            .expect("compare action should render for local files");
        let musicbrainz = state
            .musicbrainz_action(target.clone())
            .expect("musicbrainz action should render for local files");
        let loaded = TrackMetadataActionState::new(
            EntitySurfaceContext::Library,
            MetadataPanelState::Loaded,
            MetadataPanelState::Loaded,
            true,
        )
        .actions(target);

        assert_eq!(compare.kind, EntityActionKind::CompareMetadata);
        assert_eq!(compare.label, "Compare ID3");
        assert!(compare.enabled);
        assert_eq!(musicbrainz.kind, EntityActionKind::OpenMusicBrainz);
        assert_eq!(musicbrainz.label, "Searching MusicBrainz...");
        assert!(!musicbrainz.enabled);
        assert_eq!(loaded[0].label, "Hide Compare");
        assert_eq!(loaded[1].label, "Hide MusicBrainz");
    }

    #[test]
    fn track_metadata_action_state_projects_loading_and_staged_id3_display() {
        assert_eq!(
            TrackMetadataActionState::compare_panel_loading_message(),
            "Reading embedded metadata..."
        );
        assert_eq!(
            TrackMetadataActionState::musicbrainz_panel_loading_message(),
            "Searching MusicBrainz..."
        );

        let state = TrackMetadataActionState::new(
            EntitySurfaceContext::Library,
            MetadataPanelState::Hidden,
            MetadataPanelState::Hidden,
            true,
        );
        assert_eq!(state.staged_id3_edits_display(0, false, None), None);
        assert_eq!(
            state.staged_id3_edits_display(2, false, Some("TIT2")),
            Some(StagedId3EditsDisplay {
                message: ActionStatusMessageDisplay::neutral("2 staged tag edits"),
                apply_label: "Apply tags (2)".into(),
                apply_enabled: false,
                conflict_message: Some(ActionStatusMessageDisplay::danger(
                    "Duplicate target: TIT2",
                    ActionStatusMessageWidth::Conflict,
                )),
                discard_label: "Discard staged",
                show_discard: true,
            })
        );
        assert_eq!(
            state.staged_id3_edits_display(1, true, None),
            Some(StagedId3EditsDisplay {
                message: ActionStatusMessageDisplay::neutral("1 staged tag edit"),
                apply_label: "Applying tags...".into(),
                apply_enabled: false,
                conflict_message: None,
                discard_label: "Discard staged",
                show_discard: false,
            })
        );
    }

    #[test]
    fn track_metadata_action_state_projects_file_actions_and_id3_errors() {
        assert_eq!(
            TrackMetadataActionState::file_actions_display(),
            MetadataFileActionsDisplay {
                reread_label: "Re-read",
                redownload_label: "Re-download",
            }
        );
        assert_eq!(
            TrackMetadataActionState::duplicate_id3_target_message(&["TIT2".into()]),
            "Resolve duplicate ID3 target: TIT2"
        );
        assert_eq!(
            TrackMetadataActionState::duplicate_id3_target_message(&["TIT2".into(), "TPE1".into()]),
            "Resolve duplicate ID3 targets: TIT2; TPE1"
        );
        assert_eq!(
            TrackMetadataActionState::id3_apply_error_message("offline"),
            "Error applying ID3 edits: offline"
        );
        assert_eq!(
            TrackMetadataActionState::id3_apply_error_display("Error applying ID3 edits: offline"),
            ActionStatusMessageDisplay::danger(
                "Error applying ID3 edits: offline",
                ActionStatusMessageWidth::Action,
            )
        );
    }

    #[test]
    fn track_metadata_action_state_projects_panel_visibility_and_local_file_gate() {
        let target = EntityActionTarget::Track(TrackRef::Musicindex("track-1".into()));
        let no_file = TrackMetadataActionState::new(
            EntitySurfaceContext::Library,
            MetadataPanelState::Loaded,
            MetadataPanelState::Loaded,
            false,
        );
        let hidden = TrackMetadataActionState::new(
            EntitySurfaceContext::Library,
            MetadataPanelState::Hidden,
            MetadataPanelState::Hidden,
            true,
        );

        assert!(no_file.compare_action(target.clone()).is_none());
        assert!(no_file.musicbrainz_action(target).is_none());
        assert!(no_file.show_compare_panel());
        assert!(no_file.show_musicbrainz_panel());
        assert!(!hidden.show_compare_panel());
        assert!(!hidden.show_musicbrainz_panel());
    }

    #[test]
    fn track_metadata_action_state_hides_compare_and_musicbrainz_actions_in_discover() {
        let target = EntityActionTarget::Track(TrackRef::Musicindex("track-1".into()));
        let state = TrackMetadataActionState::new(
            EntitySurfaceContext::Discover,
            MetadataPanelState::Hidden,
            MetadataPanelState::Hidden,
            true,
        );

        assert!(state.compare_action(target.clone()).is_none());
        assert!(state.musicbrainz_action(target.clone()).is_none());
        assert!(state.actions(target).is_empty());
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
