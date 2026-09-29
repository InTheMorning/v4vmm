//! Shared track surface display contract.
//!
//! This module owns the GPUI-free facts used by track rows, inspector panes,
//! and full-detail track surfaces. Screens resolve artwork and wire commands;
//! this module decides labels, fallbacks, row projection, and slot shape.

#![warn(clippy::pedantic)]

use crate::api::Feed;
use crate::metadata::{feed_nostr, feed_website};
use crate::view_models::entity_detail::{
    EntityActionKind, EntityActionTarget, EntityActionTone, EntityActionVm,
};
use crate::view_models::format::fmt_date;
use crate::view_models::track::fmt_dur;
use crate::view_models::track_metadata_grid::TrackMetadataGridVm;
use crate::views::{ArtistRef, FeedRef, TrackRef, TrackView};

const UNTITLED: &str = "Untitled";
const UNKNOWN_ARTIST: &str = "Unknown Artist";
const UNKNOWN_ALBUM: &str = "Unknown Album";
const TRACK_KIND: &str = "track";
/// The owner label of a track page's feed identity section when the feed
/// has no title of its own (ADR 0075 Decision B, packet 022).
const FEED_IDENTITY_FALLBACK_OWNER: &str = "Feed";

/// Surface requesting track display facts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrackDetailSurfaceContext {
    Library,
    Discover,
}

/// Loading lifecycle for a track surface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TrackDetailLoadState {
    Loaded,
    Loading,
    Missing,
    Failed { reason: String },
}

/// Canonical user-facing labels for track surfaces.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrackDetailLabels {
    context: TrackDetailSurfaceContext,
}

impl TrackDetailLabels {
    #[must_use]
    pub const fn new(context: TrackDetailSurfaceContext) -> Self {
        Self { context }
    }

    #[must_use]
    pub const fn release_label(self) -> &'static str {
        match self.context {
            TrackDetailSurfaceContext::Library | TrackDetailSurfaceContext::Discover => "Release",
        }
    }

    #[must_use]
    pub const fn artist_label(self) -> &'static str {
        "Artist"
    }

    #[must_use]
    pub const fn track_number_label(self) -> &'static str {
        "Track #"
    }

    #[must_use]
    pub const fn duration_label(self) -> &'static str {
        "Duration"
    }

    #[must_use]
    pub const fn release_date_label(self) -> &'static str {
        "Release Date"
    }

    #[must_use]
    pub const fn publisher_label(self) -> &'static str {
        "Publisher"
    }

    #[must_use]
    pub const fn description_label(self) -> &'static str {
        "Description"
    }

    #[must_use]
    pub const fn summary_section_title(self) -> &'static str {
        "Tags"
    }
}

/// Display-ready key/value row for summary metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrackDetailSummaryRow {
    pub label: String,
    pub value: String,
    pub max_lines: usize,
}

impl TrackDetailSummaryRow {
    #[must_use]
    pub fn new(label: impl Into<String>, value: impl Into<String>, max_lines: usize) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            max_lines,
        }
    }
}

/// Display-ready projection of one track detail surface.
#[derive(Debug)]
pub struct TrackDetailVm<'a> {
    track: &'a TrackView,
    context: TrackDetailSurfaceContext,
    override_title: Option<&'a str>,
    /// The publisher feed GUID of this track's album feed (ADR 0077
    /// Decision 2, packet 004 R4-03). The track stores no publisher value
    /// of its own; the caller supplies its album feed's own value.
    publisher_feed_guid: Option<&'a str>,
    /// The track's feed, supplied so [`Self::feed_identity_section`] can
    /// list the feed's own website and Nostr key apart from the track's
    /// header (ADR 0075 Decision B, packet 022). `None` when the caller's
    /// surface has no feed.
    feed: Option<&'a Feed>,
}

impl<'a> TrackDetailVm<'a> {
    #[must_use]
    pub const fn new(track: &'a TrackView, context: TrackDetailSurfaceContext) -> Self {
        Self {
            track,
            context,
            override_title: None,
            publisher_feed_guid: None,
            feed: None,
        }
    }

    #[must_use]
    pub const fn with_override_title(mut self, title: Option<&'a str>) -> Self {
        self.override_title = title;
        self
    }

    /// Sets the publisher feed GUID of this track's album feed (R4-03). The
    /// caller reads it from the album feed, never from name text.
    #[must_use]
    pub const fn with_publisher_feed_guid(mut self, publisher_feed_guid: Option<&'a str>) -> Self {
        self.publisher_feed_guid = publisher_feed_guid;
        self
    }

    /// Sets the track's feed (R22-03, ADR 0075 Decision B). The track's own
    /// header never reads this; only [`Self::feed_identity_section`] does.
    #[must_use]
    pub const fn with_feed_identity(mut self, feed: Option<&'a Feed>) -> Self {
        self.feed = feed;
        self
    }

    #[must_use]
    pub const fn page(self) -> TrackDetailPageVm<'a> {
        TrackDetailPageVm { detail: self }
    }

    #[must_use]
    pub const fn track(&self) -> &'a TrackView {
        self.track
    }

    #[must_use]
    pub const fn context(&self) -> TrackDetailSurfaceContext {
        self.context
    }

    #[must_use]
    pub const fn labels(&self) -> TrackDetailLabels {
        TrackDetailLabels::new(self.context)
    }

    #[must_use]
    pub fn row(&self) -> TrackRowVm {
        TrackRowVm::from_detail(self)
    }

    #[must_use]
    pub fn display_title(&self) -> String {
        self.override_title
            .and_then(nonempty)
            .map(str::to_owned)
            .or_else(|| {
                self.track
                    .title
                    .as_deref()
                    .and_then(nonempty)
                    .map(str::to_owned)
            })
            .or_else(|| {
                self.track
                    .track_guid
                    .as_deref()
                    .and_then(nonempty)
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| UNTITLED.to_string())
    }

    #[must_use]
    pub fn display_artist(&self) -> String {
        self.track
            .artist
            .as_deref()
            .and_then(nonempty)
            .map_or_else(|| UNKNOWN_ARTIST.to_string(), str::to_owned)
    }

    #[must_use]
    pub fn display_album(&self) -> String {
        self.track
            .album
            .as_deref()
            .and_then(nonempty)
            .or_else(|| self.track.feed_title.as_deref().and_then(nonempty))
            .map_or_else(|| UNKNOWN_ALBUM.to_string(), str::to_owned)
    }

    #[must_use]
    pub fn display_release_context(&self) -> String {
        self.display_album()
    }

    #[must_use]
    pub const fn display_kind_badge(&self) -> &'static str {
        TRACK_KIND
    }

    #[must_use]
    pub fn track_number_display(&self) -> Option<String> {
        self.track.track_number.map(|number| number.to_string())
    }

    #[must_use]
    pub fn duration_display(&self) -> Option<String> {
        self.track.duration_secs.map(fmt_dur)
    }

    #[must_use]
    pub fn release_date_display(&self) -> Option<String> {
        self.track.pub_date.and_then(fmt_date)
    }

    #[must_use]
    pub fn publisher_display(&self) -> Option<String> {
        self.track
            .publisher_text
            .as_deref()
            .and_then(nonempty)
            .map(str::to_owned)
    }

    #[must_use]
    pub fn description(&self) -> Option<String> {
        self.track
            .description
            .as_deref()
            .and_then(nonempty)
            .map(str::to_owned)
    }

    #[must_use]
    pub fn summary_rows(&self) -> Vec<TrackDetailSummaryRow> {
        let labels = self.labels();
        let mut rows = vec![TrackDetailSummaryRow::new(
            labels.release_label(),
            self.display_release_context(),
            3,
        )];
        push_optional(
            &mut rows,
            labels.track_number_label(),
            self.track_number_display(),
            1,
        );
        push_optional(
            &mut rows,
            labels.duration_label(),
            self.duration_display(),
            1,
        );
        push_optional(
            &mut rows,
            labels.release_date_label(),
            self.release_date_display(),
            1,
        );
        if self.track.explicit == Some(true) {
            rows.push(TrackDetailSummaryRow::new("Explicit", "Yes", 1));
        }
        push_optional(
            &mut rows,
            labels.publisher_label(),
            self.publisher_display(),
            3,
        );
        rows
    }

    #[must_use]
    pub fn identity_actions(&self) -> Vec<EntityActionVm> {
        let Some(target) = self.track.id.clone().map(EntityActionTarget::Track) else {
            return Vec::new();
        };

        let mut actions = Vec::new();
        if let Some(url) = self
            .track
            .identity
            .website_url
            .as_deref()
            .and_then(nonempty)
        {
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
        if let Some(npub) = self.track.identity.nostr_npub.as_deref().and_then(nonempty) {
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

    /// R4-03 (ADR 0077 packet 004): the "open publisher" action of this
    /// track's album feed. `None` when the album names no publisher. The
    /// track stores no publisher value of its own.
    #[must_use]
    pub fn publisher_action(&self) -> Option<EntityActionVm> {
        let publisher_feed_guid = self.publisher_feed_guid?.to_owned();
        Some(EntityActionVm::new(
            EntityActionKind::OpenPublisher,
            EntityActionTarget::Artist(ArtistRef::PublisherFeed(publisher_feed_guid)),
            "Open publisher",
            EntityActionTone::Quiet,
        ))
    }

    #[must_use]
    pub const fn identity_action_prefix(&self) -> &'static str {
        match self.context {
            TrackDetailSurfaceContext::Discover => "discover-track",
            TrackDetailSurfaceContext::Library => "library-track",
        }
    }

    /// R22-02/R22-03 (ADR 0075 Decision B): the feed identity section, apart
    /// from the track's own header. `None` when this surface has no feed, or
    /// when the feed has no website and no Nostr key. The feed title is the
    /// section's owner, and each action label and accessibility text names
    /// the feed.
    #[must_use]
    pub fn feed_identity_section(&self) -> Option<FeedIdentitySectionVm> {
        let feed = self.feed?;
        let website = feed_website(feed).filter(|url| nonempty(url).is_some());
        let nostr = feed_nostr(feed).filter(|npub| nonempty(npub).is_some());
        if website.is_none() && nostr.is_none() {
            return None;
        }
        let target = feed
            .feed_guid
            .clone()
            .map(FeedRef::Musicindex)
            .map(EntityActionTarget::Feed)
            .or_else(|| self.track.id.clone().map(EntityActionTarget::Track))?;
        let owner_label = feed
            .title
            .as_deref()
            .and_then(nonempty)
            .unwrap_or(FEED_IDENTITY_FALLBACK_OWNER)
            .to_string();

        let mut actions = Vec::new();
        if let Some(url) = website {
            actions.push(
                EntityActionVm::new(
                    EntityActionKind::OpenWebsite,
                    target.clone(),
                    format!("{owner_label} website"),
                    EntityActionTone::Quiet,
                )
                .with_payload(url)
                .with_identity_a11y_label(format!("Open {owner_label} website")),
            );
        }
        if let Some(npub) = nostr {
            actions.push(
                EntityActionVm::new(
                    EntityActionKind::CopyNostr,
                    target,
                    format!("{owner_label} Nostr key"),
                    EntityActionTone::Quiet,
                )
                .with_payload(npub)
                .with_identity_a11y_label(format!("Copy {owner_label} Nostr key")),
            );
        }
        // `website` or `nostr` holds a value here (the guard above returns
        // early when both are `None`), so `actions` always holds at least
        // one entry.
        Some(FeedIdentitySectionVm {
            owner_label,
            actions,
        })
    }

    /// The identity-action id prefix of the feed identity section. Apart
    /// from [`Self::identity_action_prefix`] so a feed action's element id
    /// cannot collide with the track's own header action.
    #[must_use]
    pub const fn feed_identity_action_prefix(&self) -> &'static str {
        match self.context {
            TrackDetailSurfaceContext::Discover => "discover-track-feed",
            TrackDetailSurfaceContext::Library => "library-track-feed",
        }
    }

    #[must_use]
    pub const fn primary_actions_a11y_label(&self) -> &'static str {
        match self.context {
            TrackDetailSurfaceContext::Discover => "Discover track actions",
            TrackDetailSurfaceContext::Library => "Library track actions",
        }
    }
}

/// Page-level projection for a track detail surface.
#[derive(Debug)]
pub struct TrackDetailPageVm<'a> {
    detail: TrackDetailVm<'a>,
}

impl<'a> TrackDetailPageVm<'a> {
    #[must_use]
    pub const fn detail(&self) -> &TrackDetailVm<'a> {
        &self.detail
    }

    #[must_use]
    pub fn row(&self) -> TrackRowVm {
        self.detail.row()
    }

    #[must_use]
    pub fn identity_actions(&self) -> Vec<EntityActionVm> {
        self.detail.identity_actions()
    }

    #[must_use]
    pub const fn identity_action_prefix(&self) -> &'static str {
        self.detail.identity_action_prefix()
    }

    #[must_use]
    pub fn feed_identity_section(&self) -> Option<FeedIdentitySectionVm> {
        self.detail.feed_identity_section()
    }

    #[must_use]
    pub const fn feed_identity_action_prefix(&self) -> &'static str {
        self.detail.feed_identity_action_prefix()
    }
}

/// A track page's feed identity section, apart from the track's own header
/// (ADR 0075 Decision B, packet 022 R22-03). `owner_label` is the feed's
/// title, or a generic fallback when the feed has none. Each entry in
/// `actions` already names `owner_label` in its label and its
/// accessibility text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeedIdentitySectionVm {
    pub owner_label: String,
    pub actions: Vec<EntityActionVm>,
}

/// Row-shaped projection of [`TrackDetailVm`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrackRowVm {
    pub element_key: String,
    pub number: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub duration: Option<String>,
    pub a11y_label: String,
}

impl TrackRowVm {
    #[must_use]
    pub fn from_detail(detail: &TrackDetailVm<'_>) -> Self {
        let title = detail.display_title();
        let duration = detail.duration_display();
        Self {
            element_key: track_element_key(detail.track),
            number: detail
                .track
                .track_number
                .map_or_else(|| "\u{00B7}".to_string(), |number| number.to_string()),
            a11y_label: track_row_a11y_label(&title, duration.as_deref()),
            title,
            subtitle: Some(detail.display_artist()).filter(|artist| artist != UNKNOWN_ARTIST),
            duration,
        }
    }

    #[must_use]
    pub fn a11y_label(&self) -> String {
        if self.a11y_label.is_empty() {
            track_row_a11y_label(&self.title, self.duration.as_deref())
        } else {
            self.a11y_label.clone()
        }
    }
}

fn track_row_a11y_label(title: &str, duration: Option<&str>) -> String {
    duration.map_or_else(
        || title.to_string(),
        |duration| format!("{title}, {duration}"),
    )
}

/// Typed non-artwork slots accepted by the shared track detail surface.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrackDetailSlots {
    pub primary_actions: Vec<ActionRowItem>,
    pub summary_metadata: Option<TrackMetadataGridVm>,
    pub sections: Vec<TrackDetailSection>,
    pub advanced_panels: Vec<TrackDetailAdvancedPanel>,
    pub back_navigation: Option<NavigationContext>,
    pub external_links: Vec<ExternalLinkItem>,
    pub contributors: Vec<ContributorItem>,
    pub value_routes: Vec<ValueRouteItem>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionRowItem {
    pub id: String,
    pub label: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalLinkItem {
    pub label: String,
    pub url: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContributorItem {
    pub label: String,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueRouteItem {
    pub recipient: String,
    pub split: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrackDetailSection {
    pub id: String,
    pub label: String,
    pub empty_label: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrackDetailAdvancedPanel {
    pub id: String,
    pub label: String,
    pub empty_label: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NavigationContext {
    pub label: String,
}

fn push_optional(
    rows: &mut Vec<TrackDetailSummaryRow>,
    label: &str,
    value: Option<String>,
    max_lines: usize,
) {
    if let Some(value) = value.and_then(|value| nonempty(&value).map(str::to_owned)) {
        rows.push(TrackDetailSummaryRow::new(label, value, max_lines));
    }
}

fn track_element_key(track: &TrackView) -> String {
    match &track.id {
        Some(TrackRef::Musicindex(id)) => format!("musicindex:{id}"),
        Some(TrackRef::LocalTrackId(id)) => format!("local:{id}"),
        None => track.track_guid.as_deref().and_then(nonempty).map_or_else(
            || "track:unknown".to_string(),
            |guid| format!("guid:{guid}"),
        ),
    }
}

fn nonempty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{SourceEntityId, SourceEntityLink};
    use crate::views::{EntityIdentityLinks, IdentityIdFact, IdentityLinkFact, TrackRef};

    fn track() -> TrackView {
        TrackView {
            id: Some(TrackRef::Musicindex("t1".to_string())),
            track_guid: Some("guid-1".to_string()),
            feed_title: Some("Release title".to_string()),
            title: Some("Track title".to_string()),
            artist: Some("Artist".to_string()),
            album: Some("Album".to_string()),
            track_number: Some(7),
            duration_secs: Some(125),
            pub_date: Some(1_712_275_200),
            publisher_text: Some("Publisher".to_string()),
            description: Some("Description".to_string()),
            ..TrackView::default()
        }
    }

    fn track_with_identity() -> TrackView {
        TrackView {
            identity: EntityIdentityLinks::from_source_facts(
                None,
                vec![IdentityLinkFact {
                    link_type: Some("website".into()),
                    url: Some("https://example.test/track".into()),
                    ..IdentityLinkFact::default()
                }],
                vec![IdentityIdFact {
                    scheme: Some("nostr_npub".into()),
                    value: Some("npub1track".into()),
                    ..IdentityIdFact::default()
                }],
            ),
            ..track()
        }
    }

    #[test]
    fn display_title_prefers_nonempty_override() {
        let track = track();
        let vm = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Discover)
            .with_override_title(Some("Override"));

        assert_eq!(vm.display_title(), "Override");
    }

    #[test]
    fn display_title_falls_back_to_track_guid_then_untitled() {
        let mut track = track();
        track.title = None;
        let vm = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Library);
        assert_eq!(vm.display_title(), "guid-1");

        track.track_guid = None;
        let vm = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Library);
        assert_eq!(vm.display_title(), "Untitled");
    }

    #[test]
    fn display_artist_and_album_have_canonical_fallbacks() {
        let mut track = track();
        track.artist = None;
        track.album = None;
        track.feed_title = None;
        let vm = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Library);

        assert_eq!(vm.display_artist(), "Unknown Artist");
        assert_eq!(vm.display_album(), "Unknown Album");
    }

    #[test]
    fn summary_rows_use_canonical_label_order() {
        let mut track = track();
        track.explicit = Some(true);
        let vm = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Discover);
        let rows = vm.summary_rows();

        assert_eq!(
            rows.iter()
                .map(|row| (row.label.as_str(), row.value.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("Release", "Album"),
                ("Track #", "7"),
                ("Duration", "2:05"),
                ("Release Date", "Apr 5, 2024"),
                ("Explicit", "Yes"),
                ("Publisher", "Publisher"),
            ]
        );
    }

    #[test]
    fn summary_rows_omit_non_explicit_state() {
        let mut track = track();
        track.explicit = Some(false);
        let vm = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Discover);

        assert!(
            vm.summary_rows()
                .iter()
                .all(|row| row.label.as_str() != "Explicit"),
            "explicit summary row should only render true state"
        );
    }

    #[test]
    fn row_projection_is_subset_of_detail_contract() {
        let track = track();
        let vm = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Discover);
        let row = vm.row();

        assert_eq!(row.element_key, "musicindex:t1");
        assert_eq!(row.number, "7");
        assert_eq!(row.title, "Track title");
        assert_eq!(row.subtitle.as_deref(), Some("Artist"));
        assert_eq!(row.duration.as_deref(), Some("2:05"));
    }

    #[test]
    fn track_detail_identity_actions_carry_payloads() {
        let track = track_with_identity();
        let actions =
            TrackDetailVm::new(&track, TrackDetailSurfaceContext::Discover).identity_actions();

        assert_eq!(
            actions
                .iter()
                .map(|action| (action.kind.clone(), action.payload.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                (
                    EntityActionKind::OpenWebsite,
                    Some("https://example.test/track")
                ),
                (EntityActionKind::CopyNostr, Some("npub1track")),
            ]
        );
    }

    #[test]
    fn track_detail_identity_action_display_projects_ids() {
        let track = track_with_identity();
        let vm = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Discover);
        let actions = vm.identity_actions();

        assert_eq!(
            actions
                .iter()
                .filter_map(|action| action.identity_display(vm.identity_action_prefix()))
                .map(|display| display.id)
                .collect::<Vec<_>>(),
            vec![
                "discover-track-website:https://example.test/track".to_string(),
                "discover-track-nostr:npub1track".to_string(),
            ]
        );
    }

    #[test]
    fn track_detail_identity_action_prefix_projects_context() {
        let track = track_with_identity();
        let discover = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Discover);
        let library = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Library);

        assert_eq!(discover.identity_action_prefix(), "discover-track");
        assert_eq!(library.identity_action_prefix(), "library-track");
        assert_eq!(
            library
                .identity_actions()
                .iter()
                .filter_map(|action| action.identity_display(library.identity_action_prefix()))
                .map(|display| display.id)
                .collect::<Vec<_>>(),
            vec![
                "library-track-website:https://example.test/track".to_string(),
                "library-track-nostr:npub1track".to_string(),
            ]
        );
    }

    #[test]
    fn track_detail_identity_actions_require_track_target() {
        let mut track = track_with_identity();
        track.id = None;
        let actions =
            TrackDetailVm::new(&track, TrackDetailSurfaceContext::Library).identity_actions();

        assert!(actions.is_empty());
    }

    fn feed_with_identity() -> Feed {
        Feed {
            feed_guid: Some("feed-guid".into()),
            title: Some("MoeFactz".into()),
            source_links: Some(vec![SourceEntityLink {
                link_type: Some("website".into()),
                url: Some("https://example.test/feed".into()),
                ..Default::default()
            }]),
            source_ids: Some(vec![SourceEntityId {
                scheme: Some("nostr_npub".into()),
                value: Some("npub1feed".into()),
                ..Default::default()
            }]),
            ..Default::default()
        }
    }

    /// R22-02 (ADR 0075 Decision B): a track with no own identities exposes
    /// no identity action in its header, even when its feed has a website
    /// and a Nostr key.
    #[test]
    fn adr_0075_track_header_r22_02_header_hides_feed_identity() {
        let track = track();
        let feed = feed_with_identity();
        let vm = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Discover)
            .with_feed_identity(Some(&feed));

        assert!(vm.identity_actions().is_empty());
    }

    /// R22-03: the feed identity section names the feed as the owner, and
    /// each action label and accessibility text names the feed.
    #[test]
    fn adr_0075_track_header_r22_03_feed_identity_section_names_owner() {
        let track = track();
        let feed = feed_with_identity();
        let vm = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Discover)
            .with_feed_identity(Some(&feed));

        let section = vm
            .feed_identity_section()
            .expect("a feed with a website and a Nostr key exposes a section");
        assert_eq!(section.owner_label, "MoeFactz");
        assert_eq!(
            section
                .actions
                .iter()
                .map(|action| action.label.as_str())
                .collect::<Vec<_>>(),
            vec!["MoeFactz website", "MoeFactz Nostr key"]
        );
        for action in &section.actions {
            let display = action
                .identity_display(vm.feed_identity_action_prefix())
                .expect("a website or Nostr action always projects a display");
            assert!(
                display.a11y_label.contains("MoeFactz"),
                "accessibility text must name the feed, got {}",
                display.a11y_label
            );
        }
    }

    /// R22-04: a track with its own website and Nostr key shows them in its
    /// header; the feed section still shows the feed's own values apart
    /// from them.
    #[test]
    fn adr_0075_track_header_r22_04_own_and_feed_identities_stay_apart() {
        let track = track_with_identity();
        let feed = feed_with_identity();
        let vm = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Discover)
            .with_feed_identity(Some(&feed));

        assert_eq!(
            vm.identity_actions()
                .iter()
                .map(|action| action.payload.clone())
                .collect::<Vec<_>>(),
            vec![
                Some("https://example.test/track".to_string()),
                Some("npub1track".to_string()),
            ]
        );
        let section = vm
            .feed_identity_section()
            .expect("the feed keeps its own identity apart from the track's");
        assert_eq!(
            section
                .actions
                .iter()
                .map(|action| action.payload.clone())
                .collect::<Vec<_>>(),
            vec![
                Some("https://example.test/feed".to_string()),
                Some("npub1feed".to_string()),
            ]
        );
    }

    /// R22-05: a track without its own description exposes no description,
    /// even when its feed has one.
    #[test]
    fn adr_0075_track_header_r22_05_no_feed_description_fallback() {
        let mut track = track();
        track.description = None;
        let mut feed = feed_with_identity();
        feed.description = Some("Feed description".into());
        let vm = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Discover)
            .with_feed_identity(Some(&feed));

        assert!(vm.description().is_none());
    }

    /// A feed with no website and no Nostr key exposes no identity section.
    #[test]
    fn adr_0075_track_header_feed_identity_section_absent_without_feed_identity() {
        let track = track();
        let feed = Feed {
            feed_guid: Some("feed-guid".into()),
            title: Some("MoeFactz".into()),
            ..Default::default()
        };
        let vm = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Discover)
            .with_feed_identity(Some(&feed));

        assert!(vm.feed_identity_section().is_none());
    }

    /// A track page with no feed in its context shows no feed identity
    /// section.
    #[test]
    fn adr_0075_track_header_feed_identity_section_absent_without_feed() {
        let track = track();
        let vm = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Discover);

        assert!(vm.feed_identity_section().is_none());
    }

    #[test]
    fn page_vm_wraps_track_detail_contract() {
        let track = track_with_identity();
        let page = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Library)
            .with_override_title(Some("Page title"))
            .page();

        assert_eq!(page.detail().display_title(), "Page title");
        assert_eq!(page.identity_action_prefix(), "library-track");
        assert_eq!(page.row().title, "Page title");
        assert_eq!(page.identity_actions().len(), 2);
    }

    #[test]
    fn load_state_can_represent_failure() {
        let state = TrackDetailLoadState::Failed {
            reason: "missing".to_string(),
        };

        assert_eq!(
            state,
            TrackDetailLoadState::Failed {
                reason: "missing".to_string()
            }
        );
    }

    /// R4-03 (ADR 0077 packet 004): a track exposes the "open publisher"
    /// action of its album feed. No value comes from the track itself: the
    /// same track with no publisher feed GUID supplied exposes no action.
    #[test]
    fn adr_0077_publisher_navigation_track_exposes_its_album_feed_publisher_action() {
        let track = track_with_identity();

        let with_publisher = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Library)
            .with_publisher_feed_guid(Some("publisher-guid"))
            .publisher_action()
            .expect("a track with an album feed publisher exposes an action");
        assert_eq!(with_publisher.kind, EntityActionKind::OpenPublisher);
        assert_eq!(
            with_publisher.target,
            EntityActionTarget::Artist(ArtistRef::PublisherFeed("publisher-guid".into()))
        );
        assert!(with_publisher.enabled);

        let without_publisher =
            TrackDetailVm::new(&track, TrackDetailSurfaceContext::Library).publisher_action();
        assert_eq!(
            without_publisher, None,
            "no track row stores a publisher value: the track alone gives no action"
        );
    }
}
