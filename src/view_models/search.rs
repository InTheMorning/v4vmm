//! Search screen view-model projections.
//!
//! These projections keep Discover/Search result display contracts out of
//! `search.rs`, while remaining GPUI-free. The screen owns event wiring,
//! thumbnails, focus, and selection; this module owns the text and image
//! fields that a result row needs to render.

#![warn(clippy::pedantic)]

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt::Write as _;

use crate::api::{self, Artist, EntityDetail, Feed, PaymentRoute, Publisher, Track};
use crate::db;
use crate::view_models::entity_detail::{
    EntityActionKind, EntityActionTarget, EntityActionVm, PlaylistActionState, ReleaseActionState,
    ReleaseMembershipState, TrackActionState, TrackMembershipState,
};
use crate::view_models::format::plural;
use crate::view_models::track::TrackVm;
use crate::view_models::{ActionStatusMessageDisplay, SplitPaneState};
use crate::views::{FeedRef, TrackRef};

const DEFAULT_SPLIT_PANE_WIDTH: f32 = 360.0;

/// Search result row data owned by the Discover screen.
#[derive(Clone, Debug)]
pub(crate) struct ResultRow {
    pub(crate) entity_type: String,
    pub(crate) entity_id: String,
    pub(crate) detail: Option<EntityDetail>,
}

impl ResultRow {
    #[must_use]
    pub(crate) fn new(
        entity_type: impl Into<String>,
        entity_id: impl Into<String>,
        detail: Option<EntityDetail>,
    ) -> Self {
        Self {
            entity_type: entity_type.into(),
            entity_id: entity_id.into(),
            detail,
        }
    }

    #[must_use]
    pub(crate) fn key(&self) -> String {
        entity_key(&self.entity_type, &self.entity_id)
    }

    #[must_use]
    pub(crate) fn display(&self) -> ResultRowDisplay {
        let mut display = ResultRowVm::new(&self.entity_id, self.detail.as_ref()).display();
        display.element_id = format!("result-item:{}:{}", self.entity_type, self.entity_id);
        display.kind_label.clone_from(&self.entity_type);
        display
    }

    #[must_use]
    pub(crate) fn render_item(&self) -> ResultRowRenderItem {
        ResultRowRenderItem {
            selection_key: self.key(),
            navigation_target: ResultNavigationTarget::from_row(self),
            display: self.display(),
        }
    }

    #[must_use]
    pub(crate) fn inspector_title(&self) -> String {
        let line1 = self.display().line1;
        if line1.is_empty() {
            self.entity_id.clone()
        } else {
            line1
        }
    }
}

fn entity_key(entity_type: &str, entity_id: &str) -> String {
    format!("{entity_type}:{entity_id}")
}

/// Display-ready text and media fields for one Discover result row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResultRowDisplay {
    pub(crate) element_id: String,
    pub(crate) kind_label: String,
    pub(crate) line1: String,
    pub(crate) line2: String,
    pub(crate) line3: String,
    pub(crate) image_url: Option<String>,
}

/// Complete render projection for one Discover result row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResultRowRenderItem {
    pub(crate) selection_key: String,
    pub(crate) navigation_target: ResultNavigationTarget,
    pub(crate) display: ResultRowDisplay,
}

/// Display-ready publisher link text and tooltip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PublisherLinkDisplay {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) target: String,
    pub(crate) tooltip: String,
}

impl PublisherLinkDisplay {
    #[must_use]
    pub(crate) fn new(publisher_text: impl Into<String>) -> Self {
        let title = publisher_text.into().trim().to_string();
        Self {
            id: format!("publisher-link:{title}"),
            target: title.clone(),
            tooltip: format!("Open publisher: {title}"),
            title,
        }
    }
}

/// Borrow-only projection for one recent-feed tile.
pub(crate) struct RecentFeedTileVm<'a> {
    feed: &'a Feed,
}

/// Display-ready content for one Discovery recent-feed tile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecentFeedTileDisplay {
    pub id: String,
    pub feed_list_tile_id: String,
    pub recent_tile_id: String,
    pub podroll_tile_id: String,
    pub title: String,
    pub a11y_label: String,
    pub subtitle: Option<String>,
    pub episode_note: Option<String>,
    pub image_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecentFeedTileOpenTarget {
    pub guid: String,
    pub title: String,
}

impl RecentFeedTileDisplay {
    #[must_use]
    pub fn open_target(&self) -> RecentFeedTileOpenTarget {
        RecentFeedTileOpenTarget {
            guid: self.id.clone(),
            title: self.title.clone(),
        }
    }

    #[must_use]
    pub fn take_recent_tile_id(&mut self) -> String {
        std::mem::take(&mut self.recent_tile_id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PodrollSectionDisplay {
    pub(crate) heading_label: &'static str,
    pub(crate) scroll_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SearchTypeFilterOptionDisplay {
    pub(crate) index: usize,
    pub(crate) label: &'static str,
    pub(crate) value: Option<&'static str>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SearchFeedListSectionDisplay {
    pub(crate) heading: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PaymentRouteGroupDisplay {
    pub(crate) heading: &'static str,
}

impl<'a> RecentFeedTileVm<'a> {
    #[must_use]
    pub(crate) const fn new(feed: &'a Feed) -> Self {
        Self { feed }
    }

    #[must_use]
    pub(crate) fn display(&self) -> RecentFeedTileDisplay {
        let id = self.feed.feed_guid.clone().unwrap_or_default();
        let title = feed_display_title(self.feed);
        RecentFeedTileDisplay {
            feed_list_tile_id: format!("feed-tile:{id}"),
            recent_tile_id: format!("recent-tile:{id}"),
            podroll_tile_id: format!("podroll-tile:{id}"),
            id,
            a11y_label: format!("Feed: {title}"),
            title,
            subtitle: nonempty_text(self.feed.release_artist.as_deref())
                .or_else(|| nonempty_text(self.feed.publisher_text.as_deref()))
                .map(str::to_string),
            episode_note: self
                .feed
                .episode_count
                .map(|count| format!("{count} tracks")),
            image_url: self.feed.image_url.clone(),
        }
    }
}

impl PodrollSectionDisplay {
    #[must_use]
    pub(crate) fn new(entity_id: &str) -> Self {
        Self {
            heading_label: "Podroll",
            scroll_id: format!("podroll-scroll:{entity_id}"),
        }
    }
}

/// Borrow-only projection for one Discover result row.
pub(crate) struct ResultRowVm<'a> {
    entity_id: &'a str,
    detail: Option<&'a EntityDetail>,
}

impl<'a> ResultRowVm<'a> {
    #[must_use]
    pub(crate) fn new(entity_id: &'a str, detail: Option<&'a EntityDetail>) -> Self {
        Self { entity_id, detail }
    }

    /// Project API/domain detail into the three-line list-row display used by
    /// Discover results.
    #[must_use]
    pub(crate) fn display(&self) -> ResultRowDisplay {
        match self.detail {
            Some(EntityDetail::Artist(artist)) => self.artist_display(artist),
            Some(EntityDetail::Feed(feed)) => feed_display(feed),
            Some(EntityDetail::Track(track)) => {
                let vm = TrackVm::new(track);
                let line1 = [Some(vm.title()), vm.duration_display()]
                    .into_iter()
                    .flatten()
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>()
                    .join(" – ");
                let feed_title = track.feed_title.clone().unwrap_or_default();
                let release_artist = track.release_artist.clone().unwrap_or_default();
                let line3 = if release_artist.is_empty() {
                    feed_title
                } else {
                    format!("{feed_title} by {release_artist}")
                };

                ResultRowDisplay {
                    element_id: String::new(),
                    kind_label: String::new(),
                    line1,
                    line2: track
                        .track_artist
                        .clone()
                        .unwrap_or_else(|| "Unknown".into()),
                    line3,
                    image_url: track.image_url.clone(),
                }
            }
            Some(EntityDetail::Publisher(publisher)) => publisher_display(publisher),
            Some(EntityDetail::Release(release)) => ResultRowDisplay {
                element_id: String::new(),
                kind_label: String::new(),
                line1: self.entity_id.to_string(),
                line2: String::new(),
                line3: String::new(),
                image_url: release.image_url.clone(),
            },
            Some(EntityDetail::Recording(recording)) => ResultRowDisplay {
                element_id: String::new(),
                kind_label: String::new(),
                line1: self.entity_id.to_string(),
                line2: String::new(),
                line3: String::new(),
                image_url: recording.image_url.clone(),
            },
            None => ResultRowDisplay {
                element_id: String::new(),
                kind_label: String::new(),
                line1: self.entity_id.to_string(),
                line2: String::new(),
                line3: String::new(),
                image_url: None,
            },
        }
    }

    fn artist_display(&self, artist: &Artist) -> ResultRowDisplay {
        let mut parts = Vec::new();
        if let Some(count) = artist.track_count {
            parts.push(count_label(count, "track"));
        }
        if let Some(count) = artist.feed_count {
            parts.push(count_label(count, "feed"));
        }
        let line3 = artist
            .area
            .clone()
            .or_else(|| artist_active_years(artist))
            .unwrap_or_default();

        ResultRowDisplay {
            element_id: String::new(),
            kind_label: String::new(),
            line1: artist
                .name
                .clone()
                .or_else(|| artist.artist_id.clone())
                .unwrap_or_else(|| self.entity_id.to_string()),
            line2: parts.join(" · "),
            line3,
            image_url: artist.image_url.clone(),
        }
    }
}

fn feed_display(feed: &Feed) -> ResultRowDisplay {
    let count = feed
        .episode_count
        .map_or_else(String::new, |count| format!("{count} tracks"));
    ResultRowDisplay {
        element_id: String::new(),
        kind_label: String::new(),
        line1: feed_display_title(feed),
        line2: nonempty_text(feed.release_artist.as_deref())
            .or_else(|| nonempty_text(feed.publisher_text.as_deref()))
            .map_or_else(|| "Unknown".into(), str::to_string),
        line3: count,
        image_url: feed.image_url.clone(),
    }
}

fn publisher_display(publisher: &Publisher) -> ResultRowDisplay {
    let mut parts = Vec::new();
    if let Some(count) = publisher.feed_count {
        parts.push(format!("{count} feeds"));
    }
    if let Some(count) = publisher.track_count {
        parts.push(format!("{count} tracks"));
    }
    ResultRowDisplay {
        element_id: String::new(),
        kind_label: String::new(),
        line1: publisher.publisher_text.clone().unwrap_or_default(),
        line2: parts.join(" · "),
        line3: String::new(),
        image_url: None,
    }
}

fn count_label(count: i32, noun: &str) -> String {
    format!("{count} {noun}{}", if count == 1 { "" } else { "s" })
}

#[must_use]
pub(crate) fn feed_display_title(feed: &Feed) -> String {
    nonempty_text(feed.title.as_deref())
        .or_else(|| nonempty_text(feed.name.as_deref()))
        .or_else(|| nonempty_text(feed.feed_guid.as_deref()))
        .map_or_else(|| "Untitled".into(), str::to_string)
}

fn artist_active_years(artist: &Artist) -> Option<String> {
    match (artist.begin_year, artist.end_year) {
        (Some(begin), Some(end)) => Some(format!("{begin}-{end}")),
        (Some(begin), None) => Some(format!("{begin}-")),
        (None, Some(end)) => Some(format!("until {end}")),
        (None, None) => None,
    }
}

#[must_use]
pub(crate) fn search_result_type_is_visible(entity_type: &str) -> bool {
    matches!(entity_type, "artist" | "feed" | "track")
}

#[must_use]
pub(crate) fn normalized_search_query(value: &str) -> Option<String> {
    let query = value.trim();
    if query.chars().any(char::is_alphanumeric) {
        Some(query.to_string())
    } else {
        None
    }
}

/// Borrow-only projection of a [`Publisher`] inspector panel.
///
/// Owns the title fallback (`"Unknown publisher"`), the feed-count and
/// track-count fallbacks (explicit count → collection length → 0),
/// the detail-grid composition, and the feed-list visibility flag.
/// The screen still owns rendering of the feed-list section itself.
pub(crate) struct PublisherInspectorVm<'a> {
    publisher: &'a Publisher,
}

impl<'a> PublisherInspectorVm<'a> {
    #[must_use]
    pub(crate) fn new(publisher: &'a Publisher) -> Self {
        Self { publisher }
    }

    /// Display title — `publisher_text` if present, else
    /// `"Unknown publisher"`.
    #[must_use]
    pub(crate) fn title(&self) -> String {
        self.publisher
            .publisher_text
            .clone()
            .unwrap_or_else(|| "Unknown publisher".to_string())
    }

    /// Number of feeds — `feed_count` if present, else the length of
    /// the embedded `feeds` list, else `0`. Always non-negative; a
    /// negative `feed_count` is clamped to zero so the display never
    /// shows a leading minus.
    #[must_use]
    pub(crate) fn feed_count(&self) -> i32 {
        Self::resolve_count(self.publisher.feed_count, self.publisher.feeds.as_deref())
    }

    /// Number of tracks — same fallback chain as [`Self::feed_count`].
    #[must_use]
    pub(crate) fn track_count(&self) -> i32 {
        Self::resolve_count(self.publisher.track_count, self.publisher.tracks.as_deref())
    }

    fn resolve_count<T>(explicit: Option<i32>, collection: Option<&[T]>) -> i32 {
        explicit
            .or_else(|| collection.map(|c| i32::try_from(c.len()).unwrap_or(i32::MAX)))
            .unwrap_or(0)
            .max(0)
    }

    /// Detail-grid rows in display order: `Feeds`, `Tracks`.
    #[must_use]
    pub(crate) fn detail_rows(&self) -> Vec<(String, String)> {
        vec![
            ("Feeds".to_string(), self.feed_count().to_string()),
            ("Tracks".to_string(), self.track_count().to_string()),
        ]
    }

    /// Owned copy of the embedded feed list, or an empty `Vec` when
    /// the publisher carries no `feeds` field.
    #[must_use]
    pub(crate) fn feeds(&self) -> Vec<Feed> {
        self.publisher.feeds.clone().unwrap_or_default()
    }

    /// `true` when the publisher carries at least one embedded feed —
    /// used by the screen to decide whether to render the feed-list
    /// section.
    #[must_use]
    pub(crate) fn has_feed_list(&self) -> bool {
        self.publisher
            .feeds
            .as_ref()
            .is_some_and(|feeds| !feeds.is_empty())
    }
}

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

/// Borrow-only projection of one [`api::PaymentRoute`] entry inside the
/// inspector's value-routes panel. Owns the `"Unnamed recipient"` /
/// `"route"` fallbacks, the fee-vs-split classification, and the
/// `"Fees"` / `"Recipients"` group bucket the screen used to inline.
pub(crate) struct PaymentRouteVm<'a> {
    route: &'a PaymentRoute,
}

impl<'a> PaymentRouteVm<'a> {
    #[must_use]
    pub(crate) fn new(route: &'a PaymentRoute) -> Self {
        Self { route }
    }

    #[must_use]
    pub(crate) fn recipient_name(&self) -> String {
        self.route
            .recipient_name
            .clone()
            .unwrap_or_else(|| "Unnamed recipient".to_string())
    }

    #[must_use]
    pub(crate) fn route_type(&self) -> String {
        self.route
            .route_type
            .clone()
            .unwrap_or_else(|| "route".to_string())
    }

    /// Primary one-line payment-route summary.
    #[must_use]
    pub(crate) fn summary(&self) -> String {
        let name = self.recipient_name();
        let route_type = self.route_type();
        let split = self.split();
        let kind_label = self.kind_label();
        format!("{name} ({route_type} · {split}% · {kind_label})")
    }

    /// Optional route address display, preserving empty strings when present.
    #[must_use]
    pub(crate) fn address(&self) -> Option<String> {
        self.route.address.clone()
    }

    /// Optional route custom fields, preserving empty values when present.
    #[must_use]
    pub(crate) fn custom_fields(&self) -> Option<String> {
        let mut parts = Vec::new();
        if let Some(key) = &self.route.custom_key {
            parts.push(format!("key {key}"));
        }
        if let Some(value) = &self.route.custom_value {
            parts.push(format!("value {value}"));
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" · "))
        }
    }

    /// Split percentage; `0.0` when the route does not declare one.
    #[must_use]
    pub(crate) fn split(&self) -> f64 {
        self.route.split.unwrap_or_default()
    }

    #[must_use]
    pub(crate) fn is_fee(&self) -> bool {
        self.route.fee.unwrap_or_default()
    }

    /// `"fee"` when the route is marked as a fee, `"split"` otherwise.
    #[must_use]
    pub(crate) fn kind_label(&self) -> &'static str {
        if self.is_fee() {
            "fee"
        } else {
            "split"
        }
    }

    /// Group bucket key — `"Fees"` for fee routes, `"Recipients"`
    /// otherwise.
    #[must_use]
    pub(crate) fn group(&self) -> &'static str {
        if self.is_fee() {
            "Fees"
        } else {
            "Recipients"
        }
    }

    #[must_use]
    pub(crate) fn group_display(group: &'static str) -> PaymentRouteGroupDisplay {
        PaymentRouteGroupDisplay { heading: group }
    }
}

/// Source of the currently-pushed inspector frame. Used by the screen
/// to colour the back-button affordance and to decide which list the
/// "Back to results" target maps to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InspectorOrigin {
    Recents,
    Search,
}

/// Deferred inspector panel state.
///
/// This remains generic and GPUI-free so screens can use the same state
/// contract for contributors, value routes, `MusicBrainz`, podroll, and tag
/// comparison panels while keeping fetch/render wiring outside the VM layer.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) enum LazyPanel<T> {
    #[default]
    Hidden,
    Loading,
    Empty(String),
    Loaded(T),
}

/// Result of toggling a deferred collapsible inspector panel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LazyPanelToggle {
    Fetch,
    Toggled,
    Ignored,
}

impl LazyPanelToggle {
    #[must_use]
    pub(crate) fn should_fetch(self) -> bool {
        matches!(self, Self::Fetch)
    }

    #[must_use]
    pub(crate) fn should_notify(self) -> bool {
        matches!(self, Self::Fetch | Self::Toggled)
    }
}

impl<T> LazyPanel<T> {
    #[must_use]
    pub(crate) fn error(error: impl std::fmt::Display) -> Self {
        Self::Empty(format!("Error: {error}"))
    }

    pub(crate) fn begin_collapsible_toggle(
        &mut self,
        collapsed: &mut bool,
        force_toggle_only: bool,
    ) -> LazyPanelToggle {
        if force_toggle_only {
            *collapsed = !*collapsed;
            return LazyPanelToggle::Toggled;
        }

        match self {
            Self::Loaded(_) | Self::Empty(_) => {
                *collapsed = !*collapsed;
                LazyPanelToggle::Toggled
            }
            Self::Loading => LazyPanelToggle::Ignored,
            Self::Hidden => {
                *self = Self::Loading;
                *collapsed = false;
                LazyPanelToggle::Fetch
            }
        }
    }
}

impl<T> LazyPanel<Vec<T>> {
    pub(crate) fn from_items_result(
        result: Result<Vec<T>, impl std::fmt::Display>,
        empty_label: &str,
    ) -> Self {
        match result {
            Ok(items) if items.is_empty() => Self::Empty(empty_label.into()),
            Ok(items) => Self::Loaded(items),
            Err(error) => LazyPanel::error(error),
        }
    }
}

/// Stateful screen view-model for the Discover (search) tab.
///
/// Mirror of `view_models::library::LibraryViewModel`. Owns the
/// pure-data UI state for the search screen so `SearchApp` shrinks to
/// event wiring and `Render` glue. Per ADR 0023 this struct must
/// remain GPUI-free; fields requiring `gpui::Image`,
/// `gpui::FocusHandle`, or `Entity<_>` stay in `SearchApp`.
///
/// Subsequent commits will move snapshots (`results`,
/// `recent_feeds`, `playlists`).
#[expect(
    clippy::struct_excessive_bools,
    reason = "screen UI flags belong together; splitting them by lint count would obscure the model"
)]
#[derive(Clone, Debug)]
pub(crate) struct SearchViewModel {
    // Search filter / toggle.
    /// Current segmented filter index (`All`, `Artist`, `Feed`,
    /// `Track`). The screen owns the label/value tables; the VM owns
    /// the index and clamps it to the table length on `set_type_filter`.
    pub(crate) type_filter: usize,
    /// Whether the fuzzy-search toggle is on.
    pub(crate) fuzzy_search: bool,
    // Selection / inspector origin.
    /// Selection key — `"<entity_type>:<entity_id>"`. The screen
    /// resolves the key back to an `InspectorFrame` from its loaded
    /// rows.
    pub(crate) selected_key: Option<String>,
    /// Origin of the active inspector frame, if any.
    pub(crate) inspector_origin: Option<InspectorOrigin>,
    // Search-results pane state.
    pub(crate) loading: bool,
    pub(crate) status: String,
    pub(crate) cursor: Option<String>,
    pub(crate) has_more: bool,
    in_flight_tracks: HashSet<String>,
    // Recents pane state.
    pub(crate) recent_loading: bool,
    pub(crate) recent_status: String,
    pub(crate) recent_loaded_once: bool,
    pub(crate) recent_cursor: Option<String>,
    pub(crate) recent_has_more: bool,
    // Layout / drag state.
    split_pane: SplitPaneState,
    // Loaded snapshots — owned here so the screen can become a thin
    // Render impl. None of these carry GPUI types.
    pub(crate) results: Vec<ResultRow>,
    pub(crate) recent_feeds: Vec<Feed>,
    pub(crate) playlists: Vec<db::Playlist>,
}

/// Pure command intent for fetching the next recent-feed page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RecentFeedLoadIntent {
    cursor: Option<String>,
}

impl RecentFeedLoadIntent {
    #[must_use]
    pub(crate) fn into_cursor(self) -> Option<String> {
        self.cursor
    }
}

/// Pure command intent for fetching a Discover/Search result page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SearchLoadIntent {
    type_filter: usize,
    cursor: Option<String>,
    fuzzy: bool,
}

impl SearchLoadIntent {
    #[must_use]
    pub(crate) fn type_filter(&self) -> usize {
        self.type_filter
    }

    #[must_use]
    pub(crate) fn cursor(&self) -> Option<&str> {
        self.cursor.as_deref()
    }

    #[must_use]
    pub(crate) fn fuzzy(&self) -> bool {
        self.fuzzy
    }
}

/// One fetched Discover/Search result page.
#[derive(Clone, Debug)]
pub(crate) struct SearchBatch {
    pub(crate) rows: Vec<ResultRow>,
    pub(crate) has_more: bool,
    pub(crate) cursor: Option<String>,
}

/// Result-row identity needed by the screen to open the inspector.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ResultNavigationTarget {
    entity_type: String,
    entity_id: String,
    title: String,
}

impl ResultNavigationTarget {
    #[must_use]
    fn from_row(row: &ResultRow) -> Self {
        Self {
            entity_type: row.entity_type.clone(),
            entity_id: row.entity_id.clone(),
            title: row.inspector_title(),
        }
    }

    #[must_use]
    pub(crate) fn into_parts(self) -> (String, String, String) {
        (self.entity_type, self.entity_id, self.title)
    }
}

/// Status text plus severity for Discover/Search render paths.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SearchStatusSnapshot {
    pub(crate) text: String,
    pub(crate) display_text: String,
    pub(crate) is_error: bool,
}

impl SearchStatusSnapshot {
    #[must_use]
    fn from_text(text: &str) -> Self {
        let is_error = text.starts_with("Error:");
        Self {
            text: text.to_string(),
            display_text: if is_error {
                format!("\u{2717} {text}")
            } else {
                text.to_string()
            },
            is_error,
        }
    }

    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

/// Static labels and dynamic toggle label for the Discover results pane.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SearchPaneDisplay {
    pub(crate) split_pane_id: &'static str,
    pub(crate) resize_handle_id: &'static str,
    pub(crate) search_button_id: &'static str,
    pub(crate) fuzzy_toggle_id: &'static str,
    pub(crate) recents_button_id: &'static str,
    pub(crate) results_scroll_id: &'static str,
    pub(crate) load_more_button_id: &'static str,
    pub(crate) heading: &'static str,
    pub(crate) search_button_label: &'static str,
    pub(crate) fuzzy_toggle_label: &'static str,
    pub(crate) recents_button_label: &'static str,
    pub(crate) empty_icon: &'static str,
    pub(crate) empty_label: &'static str,
    pub(crate) load_more_label: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SearchInputDisplay {
    pub(crate) placeholder: &'static str,
}

impl SearchPaneDisplay {
    #[must_use]
    const fn new(fuzzy_search: bool) -> Self {
        Self {
            split_pane_id: "pane-container",
            resize_handle_id: "resize-handle",
            search_button_id: "search-btn",
            fuzzy_toggle_id: "fuzzy-toggle",
            recents_button_id: "show-recents",
            results_scroll_id: "results-scroll",
            load_more_button_id: "load-more",
            heading: "Search Index",
            search_button_label: "Search Index",
            fuzzy_toggle_label: if fuzzy_search {
                "Fuzzy: On"
            } else {
                "Fuzzy: Off"
            },
            recents_button_label: "Recent Feeds",
            empty_icon: "\u{1F50D}",
            empty_label: "No results",
            load_more_label: "Load more",
        }
    }
}

/// Pure render snapshot for the Discover/Search results pane.
#[expect(
    clippy::struct_excessive_bools,
    reason = "render snapshots intentionally group screen flags for one render pass"
)]
#[derive(Clone, Debug)]
pub(crate) struct SearchRenderSnapshot {
    pub(crate) status: SearchStatusSnapshot,
    pub(crate) pane_display: SearchPaneDisplay,
    pub(crate) rows: Vec<ResultRow>,
    pub(crate) selected_key: Option<String>,
    pub(crate) type_filter: usize,
    pub(crate) show_recents_root: bool,
    pub(crate) show_recents_command: bool,
    pub(crate) loading: bool,
    pub(crate) empty: bool,
    pub(crate) has_more: bool,
    pub(crate) fuzzy_search: bool,
}

/// Static labels for the recent-feeds root panel.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RecentFeedsDisplay {
    pub(crate) load_more_button_id: &'static str,
    pub(crate) heading: &'static str,
    pub(crate) empty_label: &'static str,
    pub(crate) load_more_label: &'static str,
}

impl RecentFeedsDisplay {
    const VALUE: Self = Self {
        load_more_button_id: "recent-load-more",
        heading: "Recent Feeds",
        empty_label: "No recent feeds",
        load_more_label: "Load more",
    };
}

/// Pure render snapshot for the recent-feeds root panel.
#[derive(Clone, Debug)]
pub(crate) struct RecentFeedsSnapshot {
    pub(crate) display: RecentFeedsDisplay,
    pub(crate) feeds: Vec<Feed>,
    pub(crate) status: String,
    pub(crate) has_more: bool,
    pub(crate) loading: bool,
    pub(crate) empty: bool,
}

/// Static labels for the Discover inspector shell.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InspectorChromeDisplay {
    pub(crate) back_button_id: &'static str,
    pub(crate) scroll_id: &'static str,
    pub(crate) back_label: &'static str,
    pub(crate) empty_icon: &'static str,
    pub(crate) empty_label: &'static str,
}

impl InspectorChromeDisplay {
    const VALUE: Self = Self {
        back_button_id: "inspector-back",
        scroll_id: "inspector-scroll",
        back_label: "\u{2190} Back",
        empty_icon: "\u{1F50D}",
        empty_label: "Select a result to inspect",
    };
}

/// Deferred inspector panel kinds that share [`LazyPanel`] state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DeferredPanelKind {
    Contributors,
    ValueRoutes,
}

/// Static labels for a deferred inspector panel.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DeferredPanelDisplay {
    pub(crate) section_id: &'static str,
    pub(crate) heading_label: &'static str,
    pub(crate) heading_a11y_label: &'static str,
    pub(crate) loading_label: &'static str,
    pub(crate) empty_label: &'static str,
}

/// Display-ready feed header text for the legacy Discover feed header.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SearchFeedHeaderDisplay {
    pub(crate) title: String,
    pub(crate) subtitle: Option<String>,
}

impl DeferredPanelDisplay {
    #[must_use]
    const fn for_kind(kind: DeferredPanelKind) -> Self {
        match kind {
            DeferredPanelKind::Contributors => Self {
                section_id: "section:contributors",
                heading_label: "Contributors",
                heading_a11y_label: "Toggle Contributors section",
                loading_label: "Loading contributors...",
                empty_label: "No contributors found",
            },
            DeferredPanelKind::ValueRoutes => Self {
                section_id: "section:value-routes",
                heading_label: "Value Routes",
                heading_a11y_label: "Toggle Value Routes section",
                loading_label: "Loading value routes...",
                empty_label: "No value routes found",
            },
        }
    }
}

/// Pure command intent for appending one or more tracks to a playlist.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlaylistAppendIntent {
    playlist_id: i64,
    track_ids: Vec<i64>,
    playlist_name: String,
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
    appended: usize,
    downloaded: usize,
    failed: usize,
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

const TYPE_FILTER_OPTIONS: [SearchTypeFilterOptionDisplay; 4] = [
    SearchTypeFilterOptionDisplay {
        index: 0,
        label: "All",
        value: None,
    },
    SearchTypeFilterOptionDisplay {
        index: 1,
        label: "Artist",
        value: Some("artist"),
    },
    SearchTypeFilterOptionDisplay {
        index: 2,
        label: "Feed",
        value: Some("feed"),
    },
    SearchTypeFilterOptionDisplay {
        index: 3,
        label: "Track",
        value: Some("track"),
    },
];
const TYPE_FILTER_LEN: usize = TYPE_FILTER_OPTIONS.len();

impl SearchViewModel {
    #[must_use]
    pub(crate) const fn search_input_display() -> SearchInputDisplay {
        SearchInputDisplay {
            placeholder: "Discover artists, feeds, and tracks...",
        }
    }

    /// Construct a view-model with `SearchApp::new` defaults: `All`
    /// filter, fuzzy search on, no selection, no inspector frame, no
    /// active operation, both panes idle.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            type_filter: 0,
            fuzzy_search: true,
            selected_key: None,
            inspector_origin: None,
            loading: false,
            status: String::new(),
            cursor: None,
            has_more: false,
            in_flight_tracks: HashSet::new(),
            recent_loading: false,
            recent_status: String::new(),
            recent_loaded_once: false,
            recent_cursor: None,
            recent_has_more: false,
            split_pane: SplitPaneState::new(DEFAULT_SPLIT_PANE_WIDTH),
            results: Vec::new(),
            recent_feeds: Vec::new(),
            playlists: Vec::new(),
        }
    }

    /// Set the segmented filter index. Out-of-range values are
    /// silently ignored — the segmented control owns the range.
    pub(crate) fn set_type_filter(&mut self, index: usize) {
        if index < TYPE_FILTER_LEN {
            self.type_filter = index;
        }
    }

    pub(crate) fn set_type_filter_if_changed(&mut self, index: usize) -> bool {
        let before = self.type_filter;
        self.set_type_filter(index);
        self.type_filter != before
    }

    /// Flip the fuzzy-search toggle.
    pub(crate) fn toggle_fuzzy_search(&mut self) {
        self.fuzzy_search = !self.fuzzy_search;
    }

    /// Set the `inspector_origin` to `Search` — call when pushing a
    /// frame from the main results list.
    pub(crate) fn mark_inspector_from_search(&mut self) {
        self.inspector_origin = Some(InspectorOrigin::Search);
    }

    /// Set the `inspector_origin` to `Recents` — call when pushing a
    /// frame from the "Recent feeds" panel.
    pub(crate) fn mark_inspector_from_recents(&mut self) {
        self.inspector_origin = Some(InspectorOrigin::Recents);
    }

    /// Drop the inspector-origin marker — call when popping the last
    /// frame off the inspector stack.
    pub(crate) fn clear_inspector_origin(&mut self) {
        self.inspector_origin = None;
    }

    /// Replace the selection key.
    pub(crate) fn select(&mut self, key: impl Into<String>) {
        self.selected_key = Some(key.into());
    }

    pub(crate) fn select_result(&mut self, entity_type: &str, entity_id: &str) {
        self.select(entity_key(entity_type, entity_id));
        self.mark_inspector_from_search();
    }

    pub(crate) fn select_recent_feed(&mut self, feed_guid: &str) {
        self.select(entity_key("feed", feed_guid));
        self.mark_inspector_from_recents();
    }

    /// Clear the selection.
    pub(crate) fn clear_selection(&mut self) {
        self.selected_key = None;
    }

    #[must_use]
    pub(crate) fn previous_result_target(&self) -> Option<ResultNavigationTarget> {
        if self.results.is_empty() {
            return None;
        }
        let next_idx = match self.selected_result_index() {
            Some(idx) if idx > 0 => idx - 1,
            _ => 0,
        };
        self.results
            .get(next_idx)
            .map(ResultNavigationTarget::from_row)
    }

    #[must_use]
    pub(crate) fn next_result_target(&self) -> Option<ResultNavigationTarget> {
        if self.results.is_empty() {
            return None;
        }
        let next_idx = match self.selected_result_index() {
            Some(idx) if idx + 1 < self.results.len() => idx + 1,
            Some(idx) => idx,
            None => 0,
        };
        self.results
            .get(next_idx)
            .map(ResultNavigationTarget::from_row)
    }

    fn selected_result_index(&self) -> Option<usize> {
        let current_key = self.selected_key.as_deref()?;
        self.results.iter().position(|row| row.key() == current_key)
    }

    #[must_use]
    pub(crate) fn render_snapshot(
        &self,
        inspector_stack_empty: bool,
        input_is_empty: bool,
    ) -> SearchRenderSnapshot {
        let empty = self.results.is_empty();
        let show_recents_root =
            inspector_stack_empty && self.inspector_origin.is_none() && empty && input_is_empty;
        SearchRenderSnapshot {
            status: SearchStatusSnapshot::from_text(&self.status),
            pane_display: SearchPaneDisplay::new(self.fuzzy_search),
            rows: self.results.clone(),
            selected_key: self.selected_key.clone(),
            type_filter: self.type_filter,
            show_recents_root,
            show_recents_command: inspector_stack_empty && !self.loading && !show_recents_root,
            loading: self.loading,
            empty,
            has_more: self.has_more,
            fuzzy_search: self.fuzzy_search,
        }
    }

    #[must_use]
    pub(crate) fn recent_feeds_snapshot(&self) -> RecentFeedsSnapshot {
        RecentFeedsSnapshot {
            display: RecentFeedsDisplay::VALUE,
            feeds: self.recent_feeds.clone(),
            status: self.recent_status.clone(),
            has_more: self.recent_has_more,
            loading: self.recent_loading,
            empty: self.recent_feeds.is_empty(),
        }
    }

    #[must_use]
    pub(crate) const fn recents_root_title() -> &'static str {
        RecentFeedsDisplay::VALUE.heading
    }

    #[must_use]
    pub(crate) fn inspector_title_display(
        show_recents_root: bool,
        frame_title: Option<&str>,
    ) -> String {
        match (show_recents_root, frame_title) {
            (true, None) => Self::recents_root_title().to_string(),
            (_, Some(title)) => title.to_string(),
            _ => String::new(),
        }
    }

    #[must_use]
    pub(crate) const fn inspector_chrome_display() -> InspectorChromeDisplay {
        InspectorChromeDisplay::VALUE
    }

    #[must_use]
    pub(crate) fn inspector_loading_message(title: &str) -> String {
        format!("Loading {title}...")
    }

    #[must_use]
    pub(crate) fn inspector_error_message(error: impl std::fmt::Display) -> String {
        format!("Error: {error}")
    }

    #[must_use]
    pub(crate) fn podroll_section_display(entity_id: &str) -> PodrollSectionDisplay {
        PodrollSectionDisplay::new(entity_id)
    }

    #[must_use]
    pub(crate) const fn deferred_panel_display(kind: DeferredPanelKind) -> DeferredPanelDisplay {
        DeferredPanelDisplay::for_kind(kind)
    }

    #[must_use]
    pub(crate) fn deferred_panel_empty_line(label: &str) -> String {
        label.to_string()
    }

    #[must_use]
    pub(crate) fn feed_header_display(
        title: &str,
        subtitle: Option<&str>,
    ) -> SearchFeedHeaderDisplay {
        SearchFeedHeaderDisplay {
            title: title.to_string(),
            subtitle: nonempty_text(subtitle).map(str::to_string),
        }
    }

    #[must_use]
    pub(crate) fn feed_inspector_tracks(feed: &Feed) -> Vec<Track> {
        feed.tracks.clone().unwrap_or_default()
    }

    #[must_use]
    pub(crate) const fn type_filter_options() -> &'static [SearchTypeFilterOptionDisplay] {
        &TYPE_FILTER_OPTIONS
    }

    #[must_use]
    pub(crate) fn type_filter_value(index: usize) -> Option<&'static str> {
        TYPE_FILTER_OPTIONS
            .get(index)
            .and_then(|option| option.value)
    }

    #[must_use]
    pub(crate) const fn feed_list_section_display() -> SearchFeedListSectionDisplay {
        SearchFeedListSectionDisplay { heading: "Feeds" }
    }

    /// Reset pure search state after the `MusicIndex` endpoint changes.
    pub(crate) fn reset_for_endpoint_change(&mut self) {
        self.results.clear();
        self.loading = false;
        self.status = "MusicIndex endpoint updated".into();
        self.cursor = None;
        self.has_more = false;
        self.clear_selection();
        self.clear_inspector_origin();
        self.recent_feeds.clear();
        self.recent_cursor = None;
        self.recent_has_more = false;
        self.recent_loaded_once = false;
        self.recent_status.clear();
    }

    /// Return Discover to its recent-feeds root presentation.
    ///
    /// The screen clears the GPUI input before calling this. The VM owns the
    /// pure pane state transition so the recents affordance cannot drift into
    /// renderer-only behavior.
    pub(crate) fn return_to_recent_feeds(&mut self) -> bool {
        self.loading = false;
        self.status.clear();
        self.cursor = None;
        self.has_more = false;
        self.results.clear();
        self.clear_selection();
        self.clear_inspector_origin();
        !self.recent_loaded_once && !self.recent_loading
    }

    #[must_use]
    pub(crate) fn begin_recent_feed_load(&mut self, append: bool) -> Option<RecentFeedLoadIntent> {
        if self.recent_loading {
            return None;
        }
        self.recent_loading = true;
        if !append {
            self.recent_feeds.clear();
            self.recent_cursor = None;
            self.recent_has_more = false;
        }
        self.recent_status = if append {
            "Loading more recent feeds...".into()
        } else {
            "Loading recent feeds...".into()
        };
        Some(RecentFeedLoadIntent {
            cursor: if append {
                self.recent_cursor.clone()
            } else {
                None
            },
        })
    }

    pub(crate) fn finish_recent_feed_load(&mut self, response: api::RecentFeedsResponse) {
        self.recent_loading = false;
        self.recent_loaded_once = true;
        self.recent_feeds.extend(response.data);
        self.recent_cursor = response.pagination.cursor;
        self.recent_has_more = response.pagination.has_more;
        self.recent_status.clear();
    }

    pub(crate) fn fail_recent_feed_load(&mut self, error: impl std::fmt::Display) {
        self.recent_loading = false;
        self.recent_loaded_once = true;
        self.recent_status = format!("Error: {error}");
    }

    #[must_use]
    pub(crate) fn begin_search_load(&mut self, append: bool) -> Option<SearchLoadIntent> {
        if self.loading {
            return None;
        }
        self.loading = true;
        self.status = if append {
            "Loading more...".into()
        } else {
            "Discovering...".into()
        };

        if !append {
            self.results.clear();
            self.cursor = None;
            self.has_more = false;
            self.clear_selection();
            self.clear_inspector_origin();
        }

        Some(SearchLoadIntent {
            type_filter: self.type_filter,
            cursor: if append { self.cursor.clone() } else { None },
            fuzzy: self.fuzzy_search,
        })
    }

    pub(crate) fn finish_search_load(&mut self, batch: SearchBatch, append: bool) {
        if !append && batch.rows.is_empty() {
            self.status.clear();
            self.results.clear();
            self.loading = false;
            self.has_more = false;
            self.cursor = None;
            return;
        }

        if append {
            let mut seen: HashSet<(String, String)> = self
                .results
                .iter()
                .map(|row| (row.entity_type.clone(), row.entity_id.clone()))
                .collect();
            for row in batch.rows {
                let key = (row.entity_type.clone(), row.entity_id.clone());
                if seen.insert(key) {
                    self.results.push(row);
                }
            }
        } else {
            self.results.extend(batch.rows);
        }
        self.cursor = batch.cursor;
        self.has_more = batch.has_more;
        self.loading = false;

        let total = self.results.len();
        self.status = format!(
            "{total} result{}{}",
            if total == 1 { "" } else { "s" },
            if self.has_more { "+" } else { "" }
        );
    }

    pub(crate) fn fail_search_load(&mut self, error: impl std::fmt::Display) {
        self.loading = false;
        self.status = format!("Error: {error}");
    }

    pub(crate) fn merge_artist_result_detail(&mut self, entity_id: &str, detail: &Artist) {
        for row in &mut self.results {
            if row.entity_type == "artist" && row.entity_id == entity_id {
                let Some(EntityDetail::Artist(artist)) = row.detail.as_mut() else {
                    continue;
                };
                artist.track_count = detail.track_count;
                artist.feed_count = detail.feed_count;
                if detail.image_url.is_some() {
                    artist.image_url.clone_from(&detail.image_url);
                }
            }
        }
    }

    pub(crate) fn replace_playlists(&mut self, playlists: Vec<db::Playlist>) {
        self.playlists = playlists;
    }

    #[must_use]
    pub(crate) fn playlists_snapshot(&self) -> Vec<db::Playlist> {
        self.playlists.clone()
    }

    pub(crate) fn fail_playlist_load(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Error loading playlists: {error:#}");
    }

    pub(crate) fn fail_feed_subscription(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Error subscribing feed: {error:#}");
    }

    pub(crate) fn fail_feed_tracks_load(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Error loading feed tracks: {error:#}");
    }

    pub(crate) fn set_feed_has_no_tracks(&mut self) {
        self.status = "Feed has no tracks".into();
    }

    pub(crate) fn fail_playlist_create(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Create playlist: {error:#}");
    }

    pub(crate) fn set_track_not_in_library(&mut self) {
        self.status = "Track not in local library".into();
    }

    #[must_use]
    pub(crate) fn begin_playlist_append(
        &mut self,
        playlist_id: i64,
        track_ids: Vec<i64>,
    ) -> Option<PlaylistAppendIntent> {
        if track_ids.is_empty() {
            return None;
        }
        let playlist_name = self
            .playlists
            .iter()
            .find(|playlist| playlist.id == playlist_id)
            .map(|playlist| playlist.name.clone())
            .unwrap_or_default();
        self.status = format!(
            "Downloading {} track{}...",
            track_ids.len(),
            if track_ids.len() == 1 { "" } else { "s" }
        );
        Some(PlaylistAppendIntent {
            playlist_id,
            track_ids,
            playlist_name,
        })
    }

    pub(crate) fn finish_playlist_append(
        &mut self,
        intent: &PlaylistAppendIntent,
        outcome: PlaylistAppendOutcome,
    ) {
        let mut message = format!(
            "Added {} of {} to {}",
            outcome.appended,
            intent.total_tracks(),
            intent.playlist_name()
        );
        if outcome.downloaded > 0 {
            write!(&mut message, " (downloaded {})", outcome.downloaded)
                .expect("writing to a String cannot fail");
        }
        if outcome.failed > 0 {
            write!(&mut message, "; {} failed", outcome.failed)
                .expect("writing to a String cannot fail");
        }
        self.status = message;
    }

    pub(crate) fn fail_playlist_append(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Error adding to playlist: {error:#}");
    }

    #[must_use]
    pub(crate) fn begin_track_operation(&mut self, key: impl Into<String>) -> bool {
        let key = key.into();
        if key.is_empty() {
            return false;
        }
        self.in_flight_tracks.insert(key)
    }

    #[must_use]
    pub(crate) fn is_track_operation_in_flight(&self, key: &str) -> bool {
        !key.is_empty() && self.in_flight_tracks.contains(key)
    }

    pub(crate) fn finish_track_download(&mut self, key: &str, message: impl Into<String>) {
        self.in_flight_tracks.remove(key);
        self.status = message.into();
    }

    pub(crate) fn fail_track_download(&mut self, key: &str, error: impl std::fmt::Display) {
        self.in_flight_tracks.remove(key);
        self.status = format!("Download error: {error:#}");
    }

    pub(crate) fn finish_track_remove(&mut self, key: &str, message: impl Into<String>) {
        self.in_flight_tracks.remove(key);
        self.status = message.into();
    }

    pub(crate) fn fail_track_remove(&mut self, key: &str, error: impl std::fmt::Display) {
        self.in_flight_tracks.remove(key);
        self.status = format!("Remove error: {error:#}");
    }

    #[must_use]
    pub(crate) fn is_resizing(&self) -> bool {
        self.split_pane.is_resizing()
    }

    #[must_use]
    pub(crate) fn split_pane_width(&self) -> f32 {
        self.split_pane.leading_width()
    }

    pub(crate) fn begin_resize(&mut self) {
        self.split_pane.begin_resize();
    }

    pub(crate) fn end_resize(&mut self) {
        self.split_pane.end_resize();
    }

    pub(crate) fn resize_split_pane(
        &mut self,
        requested_width: f32,
        min_width: f32,
        max_width: f32,
    ) {
        self.split_pane
            .resize_to(requested_width, min_width, max_width);
    }
}

/// This is pure projection over already-fetched rows. Network enrichment
/// remains in the screen-side query adapter until a broader command/query
/// layer exists.
#[must_use]
pub(crate) fn artist_rows_from_result_rows(
    rows: &[ResultRow],
    query: Option<&str>,
) -> Vec<ResultRow> {
    let mut artists = BTreeMap::<String, Artist>::new();

    for row in rows {
        match &row.detail {
            Some(EntityDetail::Artist(artist)) => {
                insert_artist_candidate(&mut artists, artist.clone(), query);
            }
            Some(EntityDetail::Feed(feed)) => {
                if let Some(name) = nonempty_text(feed.release_artist.as_deref()) {
                    insert_artist_candidate(
                        &mut artists,
                        Artist {
                            name: Some(name.to_string()),
                            feed_count: Some(1),
                            image_url: feed.image_url.clone(),
                            ..Artist::default()
                        },
                        query,
                    );
                }
            }
            Some(EntityDetail::Track(track)) => {
                insert_track_artist_candidates(&mut artists, track, query);
            }
            Some(
                EntityDetail::Release(_) | EntityDetail::Recording(_) | EntityDetail::Publisher(_),
            )
            | None => {}
        }
    }

    artists
        .into_values()
        .map(|artist| {
            let entity_id = artist
                .name
                .clone()
                .or_else(|| artist.artist_id.clone())
                .unwrap_or_default();
            ResultRow::new("artist", entity_id, Some(EntityDetail::Artist(artist)))
        })
        .collect()
}

fn insert_track_artist_candidates(
    artists: &mut BTreeMap<String, Artist>,
    track: &Track,
    query: Option<&str>,
) {
    let names: BTreeSet<&str> = [
        track.track_artist.as_deref(),
        track.release_artist.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect();
    for name in names {
        insert_artist_candidate(
            artists,
            Artist {
                name: Some(name.to_string()),
                track_count: Some(1),
                image_url: track.image_url.clone(),
                ..Artist::default()
            },
            query,
        );
    }
}

fn insert_artist_candidate(
    artists: &mut BTreeMap<String, Artist>,
    artist: Artist,
    query: Option<&str>,
) {
    let Some(name) = artist.name.clone().or_else(|| artist.artist_id.clone()) else {
        return;
    };
    let name = name.trim();
    if name.is_empty() || !artist_name_matches_query(name, query) {
        return;
    }

    let key = name.to_lowercase();
    if let Some(existing) = artists.get_mut(&key) {
        if existing.name.is_none() {
            existing.name = Some(name.to_string());
        }
        if existing.image_url.is_none() {
            existing.image_url = artist.image_url;
        }
        existing.feed_count = add_optional_counts(existing.feed_count, artist.feed_count);
        existing.track_count = add_optional_counts(existing.track_count, artist.track_count);
        return;
    }

    artists.insert(
        key,
        Artist {
            name: Some(name.to_string()),
            ..artist
        },
    );
}

fn artist_name_matches_query(name: &str, query: Option<&str>) -> bool {
    let Some(query) = query else {
        return true;
    };
    let normalized_name = name.to_lowercase();
    query
        .split_whitespace()
        .map(str::to_lowercase)
        .all(|term| normalized_name.contains(&term))
}

fn add_optional_counts(left: Option<i32>, right: Option<i32>) -> Option<i32> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left + right),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn nonempty_text(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| {
        !value.is_empty()
            && value
                .chars()
                .any(|ch| ch != '.' && ch != '\u{2026}' && !ch.is_whitespace())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Recording, Release};

    fn assert_width_eq(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < f32::EPSILON);
    }

    fn playlist(name: &str) -> db::Playlist {
        db::Playlist {
            id: 1,
            name: name.into(),
            description: None,
            track_count: 0,
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn artist_display_uses_counts_area_and_image() {
        let detail = EntityDetail::Artist(Artist {
            name: Some("The Artist".into()),
            track_count: Some(1),
            feed_count: Some(2),
            area: Some("Canada".into()),
            begin_year: Some(1999),
            image_url: Some("https://example.test/a.png".into()),
            ..Artist::default()
        });

        assert_eq!(
            ResultRowVm::new("artist-id", Some(&detail)).display(),
            ResultRowDisplay {
                element_id: String::new(),
                kind_label: String::new(),
                line1: "The Artist".into(),
                line2: "1 track · 2 feeds".into(),
                line3: "Canada".into(),
                image_url: Some("https://example.test/a.png".into()),
            }
        );
    }

    #[test]
    fn artist_display_falls_back_to_active_years_then_entity_id() {
        let detail = EntityDetail::Artist(Artist {
            begin_year: Some(2001),
            end_year: None,
            ..Artist::default()
        });

        let display = ResultRowVm::new("artist-id", Some(&detail)).display();
        assert_eq!(display.line1, "artist-id");
        assert_eq!(display.line3, "2001-");
    }

    #[test]
    fn feed_display_uses_title_fallbacks_and_episode_count() {
        let detail = EntityDetail::Feed(Feed {
            name: Some("Feed Name".into()),
            feed_guid: Some("feed-guid".into()),
            release_artist: Some("Release Artist".into()),
            episode_count: Some(12),
            image_url: Some("https://example.test/f.png".into()),
            ..Feed::default()
        });

        assert_eq!(
            ResultRowVm::new("feed-id", Some(&detail)).display(),
            ResultRowDisplay {
                element_id: String::new(),
                kind_label: String::new(),
                line1: "Feed Name".into(),
                line2: "Release Artist".into(),
                line3: "12 tracks".into(),
                image_url: Some("https://example.test/f.png".into()),
            }
        );
    }

    #[test]
    fn recent_feed_tile_vm_uses_current_recent_feed_response_labels() {
        let response: api::RecentFeedsResponse = serde_json::from_str(
            r#"{
                "data": [{
                    "feed_guid": "495c0d0b-f576-5d12-a76a-d806f2e19b7e",
                    "feed_url": "https://feeds.fountain.fm/ttc59BjLMAAPgxnP2fy2",
                    "title": "Is Anybody There?",
                    "raw_medium": "music",
                    "release_artist": "The Paisley Daze",
                    "release_artist_sort": null,
                    "release_date": 1777630024,
                    "release_kind": "unknown",
                    "description": null,
                    "image_url": "https://feeds.fountain.fm/cover.jpg",
                    "publisher_text": "The Paisley Daze",
                    "language": "en",
                    "explicit": false,
                    "episode_count": 1,
                    "newest_item_at": 1777630023,
                    "oldest_item_at": 1777630023,
                    "created_at": 1777650856,
                    "updated_at": 1777650856
                }],
                "pagination": {
                    "cursor": "next",
                    "has_more": true
                }
            }"#,
        )
        .expect("recent feeds response should deserialize");

        let feed = response.data.first().expect("fixture includes one feed");
        let vm = RecentFeedTileVm::new(feed);
        let display = vm.display();

        assert_eq!(display.id, "495c0d0b-f576-5d12-a76a-d806f2e19b7e");
        assert_eq!(
            display.feed_list_tile_id,
            "feed-tile:495c0d0b-f576-5d12-a76a-d806f2e19b7e"
        );
        assert_eq!(
            display.recent_tile_id,
            "recent-tile:495c0d0b-f576-5d12-a76a-d806f2e19b7e"
        );
        assert_eq!(
            display.podroll_tile_id,
            "podroll-tile:495c0d0b-f576-5d12-a76a-d806f2e19b7e"
        );
        assert_eq!(display.title, "Is Anybody There?");
        assert_eq!(display.subtitle.as_deref(), Some("The Paisley Daze"));
        assert_eq!(display.episode_note.as_deref(), Some("1 tracks"));
        assert_eq!(
            display.image_url.as_deref(),
            Some("https://feeds.fountain.fm/cover.jpg")
        );
    }

    #[test]
    fn recent_feed_tile_vm_falls_back_to_publisher_for_subtitle() {
        let feed = Feed {
            title: Some("Feed Title".into()),
            publisher_text: Some("Publisher".into()),
            ..Feed::default()
        };
        let vm = RecentFeedTileVm::new(&feed);
        let display = vm.display();

        assert_eq!(display.title, "Feed Title");
        assert_eq!(display.subtitle.as_deref(), Some("Publisher"));
    }

    #[test]
    fn recent_feed_tile_vm_projects_id_and_episode_note() {
        let feed = Feed {
            feed_guid: Some("feed-guid".into()),
            episode_count: Some(0),
            ..Feed::default()
        };
        let display = RecentFeedTileVm::new(&feed).display();

        assert_eq!(display.id, "feed-guid");
        assert_eq!(display.feed_list_tile_id, "feed-tile:feed-guid");
        assert_eq!(display.recent_tile_id, "recent-tile:feed-guid");
        assert_eq!(display.podroll_tile_id, "podroll-tile:feed-guid");
        assert_eq!(display.episode_note.as_deref(), Some("0 tracks"));

        let feed = Feed {
            feed_guid: None,
            episode_count: None,
            ..Feed::default()
        };
        let display = RecentFeedTileVm::new(&feed).display();

        assert_eq!(display.id, "");
        assert_eq!(display.feed_list_tile_id, "feed-tile:");
        assert_eq!(display.recent_tile_id, "recent-tile:");
        assert_eq!(display.podroll_tile_id, "podroll-tile:");
        assert_eq!(display.episode_note, None);
    }

    #[test]
    fn podroll_section_display_projects_heading_and_scroll_id() {
        assert_eq!(
            SearchViewModel::podroll_section_display("feed-1"),
            PodrollSectionDisplay {
                heading_label: "Podroll",
                scroll_id: "podroll-scroll:feed-1".into(),
            }
        );
    }

    #[test]
    fn recent_feed_tile_vm_does_not_emit_placeholder_ellipsis() {
        let feed = Feed {
            title: Some(" … ".into()),
            name: Some("...".into()),
            release_artist: Some("...".into()),
            publisher_text: Some("Publisher".into()),
            feed_guid: Some("feed-guid".into()),
            ..Feed::default()
        };
        let display = RecentFeedTileVm::new(&feed).display();

        assert_eq!(display.title, "feed-guid");
        assert_eq!(display.subtitle.as_deref(), Some("Publisher"));
        assert_ne!(display.title, "...");
        assert_ne!(display.subtitle.as_deref(), Some("..."));
    }

    #[test]
    fn track_display_uses_track_vm_title_duration_and_artist_fallback() {
        let detail = EntityDetail::Track(Track {
            name: Some("Track Name".into()),
            duration_secs: Some(65),
            feed_title: Some("Feed Title".into()),
            release_artist: Some("Release Artist".into()),
            image_url: Some("https://example.test/t.png".into()),
            ..Track::default()
        });

        assert_eq!(
            ResultRowVm::new("track-id", Some(&detail)).display(),
            ResultRowDisplay {
                element_id: String::new(),
                kind_label: String::new(),
                line1: "Track Name – 1:05".into(),
                line2: "Unknown".into(),
                line3: "Feed Title by Release Artist".into(),
                image_url: Some("https://example.test/t.png".into()),
            }
        );
    }

    #[test]
    fn publisher_display_keeps_no_image_contract() {
        let detail = EntityDetail::Publisher(Publisher {
            publisher_text: Some("Pub".into()),
            feed_count: Some(2),
            track_count: Some(3),
            ..Publisher::default()
        });

        assert_eq!(
            ResultRowVm::new("publisher-id", Some(&detail)).display(),
            ResultRowDisplay {
                element_id: String::new(),
                kind_label: String::new(),
                line1: "Pub".into(),
                line2: "2 feeds · 3 tracks".into(),
                line3: String::new(),
                image_url: None,
            }
        );
    }

    #[test]
    fn fallback_rows_preserve_release_and_recording_images() {
        let release = EntityDetail::Release(Release {
            image_url: Some("https://example.test/release.png".into()),
            ..Release::default()
        });
        let recording = EntityDetail::Recording(Recording {
            image_url: Some("https://example.test/recording.png".into()),
            ..Recording::default()
        });

        assert_eq!(
            ResultRowVm::new("release-id", Some(&release))
                .display()
                .image_url
                .as_deref(),
            Some("https://example.test/release.png")
        );
        assert_eq!(
            ResultRowVm::new("recording-id", Some(&recording))
                .display()
                .image_url
                .as_deref(),
            Some("https://example.test/recording.png")
        );
        assert_eq!(ResultRowVm::new("bare-id", None).display().line1, "bare-id");
    }

    #[test]
    fn visible_result_types_match_discover_scope() {
        assert!(search_result_type_is_visible("artist"));
        assert!(search_result_type_is_visible("feed"));
        assert!(search_result_type_is_visible("track"));
        assert!(!search_result_type_is_visible("publisher"));
    }

    #[test]
    fn artist_rows_are_derived_from_feed_and_track_details() {
        let rows = vec![
            ResultRow::new(
                "track",
                "track-1",
                Some(EntityDetail::Track(Track {
                    track_artist: Some("The Doerfels".into()),
                    release_artist: Some("The Doerfels".into()),
                    image_url: Some("https://example.test/track.png".into()),
                    ..Track::default()
                })),
            ),
            ResultRow::new(
                "feed",
                "feed-1",
                Some(EntityDetail::Feed(Feed {
                    release_artist: Some("The Doerfels".into()),
                    image_url: Some("https://example.test/feed.png".into()),
                    ..Feed::default()
                })),
            ),
            ResultRow::new(
                "artist",
                "other",
                Some(EntityDetail::Artist(Artist {
                    name: Some("Other Artist".into()),
                    ..Artist::default()
                })),
            ),
        ];

        let artist_rows = artist_rows_from_result_rows(&rows, Some("doerfels"));

        assert_eq!(artist_rows.len(), 1);
        assert_eq!(artist_rows[0].entity_type, "artist");
        assert_eq!(artist_rows[0].entity_id, "The Doerfels");
        let Some(EntityDetail::Artist(artist)) = &artist_rows[0].detail else {
            panic!("expected artist detail");
        };
        assert_eq!(artist.track_count, Some(1));
        assert_eq!(artist.feed_count, Some(1));
        assert_eq!(
            artist.image_url.as_deref(),
            Some("https://example.test/track.png")
        );
    }

    #[test]
    fn publisher_inspector_vm_falls_back_to_unknown_publisher_title() {
        let pub_ = Publisher::default();
        let vm = PublisherInspectorVm::new(&pub_);
        assert_eq!(vm.title(), "Unknown publisher");
    }

    #[test]
    fn publisher_inspector_vm_uses_publisher_text_when_present() {
        let pub_ = Publisher {
            publisher_text: Some("Acme Audio".into()),
            ..Publisher::default()
        };
        let vm = PublisherInspectorVm::new(&pub_);
        assert_eq!(vm.title(), "Acme Audio");
    }

    #[test]
    fn publisher_inspector_vm_prefers_explicit_counts_over_collection_length() {
        let pub_ = Publisher {
            feed_count: Some(7),
            track_count: Some(42),
            feeds: Some(vec![Feed::default()]),
            tracks: Some(vec![Track::default(), Track::default()]),
            ..Publisher::default()
        };
        let vm = PublisherInspectorVm::new(&pub_);
        assert_eq!(vm.feed_count(), 7);
        assert_eq!(vm.track_count(), 42);
    }

    #[test]
    fn publisher_inspector_vm_falls_back_to_collection_length_when_count_absent() {
        let pub_ = Publisher {
            feed_count: None,
            track_count: None,
            feeds: Some(vec![Feed::default(), Feed::default()]),
            tracks: Some(vec![Track::default(), Track::default(), Track::default()]),
            ..Publisher::default()
        };
        let vm = PublisherInspectorVm::new(&pub_);
        assert_eq!(vm.feed_count(), 2);
        assert_eq!(vm.track_count(), 3);
    }

    #[test]
    fn publisher_inspector_vm_falls_back_to_zero_when_neither_present() {
        let pub_ = Publisher::default();
        let vm = PublisherInspectorVm::new(&pub_);
        assert_eq!(vm.feed_count(), 0);
        assert_eq!(vm.track_count(), 0);
    }

    #[test]
    fn publisher_inspector_vm_detail_rows_render_in_feeds_then_tracks_order() {
        let pub_ = Publisher {
            feed_count: Some(3),
            track_count: Some(5),
            ..Publisher::default()
        };
        let vm = PublisherInspectorVm::new(&pub_);
        let rows = vm.detail_rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], ("Feeds".into(), "3".into()));
        assert_eq!(rows[1], ("Tracks".into(), "5".into()));
    }

    #[test]
    fn publisher_inspector_vm_has_feed_list_only_when_feeds_present() {
        let pub_ = Publisher::default();
        let vm = PublisherInspectorVm::new(&pub_);
        assert!(!vm.has_feed_list());
        let pub_ = Publisher {
            feeds: Some(vec![Feed::default()]),
            ..Publisher::default()
        };
        let vm = PublisherInspectorVm::new(&pub_);
        assert!(vm.has_feed_list());
    }

    #[test]
    fn action_row_vm_visibility_matches_feed_and_track_only() {
        assert!(ActionRowVm::new("feed", false, None, None).is_visible());
        assert!(ActionRowVm::new("track", false, None, None).is_visible());
        assert!(!ActionRowVm::new("artist", false, None, None).is_visible());
        assert!(!ActionRowVm::new("publisher", false, None, None).is_visible());
        assert!(!ActionRowVm::new("release", false, None, None).is_visible());
    }

    #[test]
    fn action_row_vm_busy_label_distinguishes_remove_vs_download() {
        // local_subscription = Some(true) → "Removing..."
        let vm = ActionRowVm::new("feed", true, Some(true), None);
        assert_eq!(vm.subscription_button_label(), "Removing...");
        // local_subscription = Some(false) → "Downloading..."
        let vm = ActionRowVm::new("feed", true, Some(false), None);
        assert_eq!(vm.subscription_button_label(), "Downloading...");
        // local_subscription = None → "Downloading..." (matches the
        // `unwrap_or(false)` in the legacy renderer).
        let vm = ActionRowVm::new("feed", true, None, None);
        assert_eq!(vm.subscription_button_label(), "Downloading...");
    }

    #[test]
    fn action_row_vm_idle_label_picks_noun_by_entity_type() {
        let vm = ActionRowVm::new("feed", false, Some(false), None);
        assert_eq!(vm.subscription_button_label(), "Download Feed");
        let vm = ActionRowVm::new("track", false, Some(false), None);
        assert_eq!(vm.subscription_button_label(), "Download Track");
        let vm = ActionRowVm::new("feed", false, Some(true), None);
        assert_eq!(vm.subscription_button_label(), "Remove Feed");
        let vm = ActionRowVm::new("track", false, Some(true), None);
        assert_eq!(vm.subscription_button_label(), "Remove Track");
    }

    #[test]
    fn action_row_vm_idle_label_treats_unknown_local_subscription_as_downloadable() {
        let vm = ActionRowVm::new("feed", false, None, None);
        assert_eq!(vm.subscription_button_label(), "Download Feed");
    }

    #[test]
    fn action_row_vm_add_to_playlist_label_uses_feed_noun() {
        let vm = ActionRowVm::new("feed", false, None, None);
        assert_eq!(vm.add_to_playlist_label(), "Add feed to playlist");
        let vm = ActionRowVm::new("track", false, None, None);
        assert_eq!(vm.add_to_playlist_label(), "Add to playlist");
    }

    #[test]
    fn action_row_vm_inspector_playlist_display_projects_id_and_label() {
        let vm = ActionRowVm::new("feed", false, None, None);
        assert_eq!(
            vm.inspector_playlist_display("feed-1", "Add feed to playlist ▾"),
            SearchInspectorPlaylistDisplay {
                popover_id: "inspector-add:feed-1".into(),
                trigger_label: "Add feed to playlist ▾".into(),
            }
        );

        let vm = ActionRowVm::new("track", false, None, None);
        assert_eq!(
            vm.inspector_playlist_display("track-1", "Add to playlist"),
            SearchInspectorPlaylistDisplay {
                popover_id: "inspector-add:track-1".into(),
                trigger_label: "Add to playlist".into(),
            }
        );
        assert_eq!(
            vm.inspector_playlist_display("track-1", "").trigger_label,
            "Add to playlist"
        );
    }

    #[test]
    fn action_row_vm_playlist_trigger_label_uses_release_action_when_available() {
        let vm = ActionRowVm::new("feed", false, None, None);
        let action = EntityActionVm::new(
            EntityActionKind::AddToPlaylist,
            EntityActionTarget::Feed(FeedRef::Musicindex("feed-1".into())),
            "Add release to playlist",
            crate::view_models::entity_detail::EntityActionTone::Secondary,
        );
        assert_eq!(
            vm.playlist_trigger_label(Some(&action)),
            "Add release to playlist"
        );
        assert_eq!(vm.playlist_trigger_label(None), "Add feed to playlist");

        let vm = ActionRowVm::new("track", false, None, None);
        assert_eq!(vm.playlist_trigger_label(Some(&action)), "Add to playlist");
    }

    #[test]
    fn action_row_vm_projects_subscription_message_display() {
        let vm = ActionRowVm::new("feed", false, None, Some("Subscribed!"));
        assert_eq!(
            vm.subscription_message_display(),
            Some(ActionStatusMessageDisplay::neutral("Subscribed!"))
        );
        let vm = ActionRowVm::new("feed", false, None, Some("error: bad request"));
        assert_eq!(
            vm.subscription_message_display(),
            Some(ActionStatusMessageDisplay::danger(
                "error: bad request",
                crate::view_models::ActionStatusMessageWidth::Status,
            ))
        );
        let vm = ActionRowVm::new("feed", false, None, Some("Error: bad request"));
        assert_eq!(
            vm.subscription_message_display(),
            Some(ActionStatusMessageDisplay::danger(
                "Error: bad request",
                crate::view_models::ActionStatusMessageWidth::Status,
            ))
        );
        let vm = ActionRowVm::new("feed", false, None, None);
        assert_eq!(vm.subscription_message_display(), None);
    }

    #[test]
    fn track_row_action_vm_key_prefers_enclosure_then_guid() {
        let track = Track {
            enclosure_url: Some("https://example.test/a.mp3".into()),
            track_guid: Some("guid".into()),
            ..Track::default()
        };
        let vm = TrackRowActionVm::new(&track, false, false);
        assert_eq!(vm.key(), "https://example.test/a.mp3");

        let track = Track {
            enclosure_url: None,
            track_guid: Some("guid".into()),
            ..Track::default()
        };
        let vm = TrackRowActionVm::new(&track, false, false);
        assert_eq!(vm.key(), "guid");
    }

    #[test]
    fn track_row_action_vm_labels_match_download_state() {
        let track = Track::default();
        let vm = TrackRowActionVm::new(&track, false, true);
        assert_eq!(vm.busy_tooltip(), "Downloading...");
        assert_eq!(vm.primary_action().label, "Downloading...");
        assert!(vm.is_in_flight());

        let vm = TrackRowActionVm::new(&track, true, true);
        assert_eq!(vm.busy_tooltip(), "Removing...");
        assert_eq!(vm.primary_action().label, "Removing...");
    }

    #[test]
    fn track_row_action_vm_download_display_projects_ids_and_tooltip() {
        let track = Track {
            enclosure_url: Some("https://example.test/a.mp3".into()),
            track_guid: Some("guid".into()),
            ..Track::default()
        };
        let vm = TrackRowActionVm::new(&track, false, true);
        assert_eq!(
            vm.download_display(),
            TrackRowDownloadDisplay {
                busy_indicator_id: "track-row-download-spin:https://example.test/a.mp3".into(),
                button_id: "track-row-download:https://example.test/a.mp3".into(),
                busy_tooltip: "Downloading...",
            }
        );

        let vm = TrackRowActionVm::new(&track, true, true);
        assert_eq!(vm.download_display().busy_tooltip, "Removing...");
    }

    #[test]
    fn track_row_action_vm_projects_shared_action_state() {
        let track = Track {
            track_guid: Some("track-guid".into()),
            enclosure_url: Some("https://example.test/track.mp3".into()),
            ..Track::default()
        };
        let remote = TrackRowActionVm::new(&track, false, false).primary_action();
        assert_eq!(remote.kind, EntityActionKind::Download);
        assert_eq!(remote.label, "Download");
        assert_eq!(
            remote.tone,
            crate::view_models::entity_detail::EntityActionTone::Secondary
        );
        assert!(remote.enabled);

        let removing = TrackRowActionVm::new(&track, true, true).primary_action();
        assert_eq!(removing.kind, EntityActionKind::Remove);
        assert_eq!(removing.label, "Removing...");
        assert_eq!(
            removing.tone,
            crate::view_models::entity_detail::EntityActionTone::DestructiveQuiet
        );
        assert!(!removing.enabled);
    }

    #[test]
    fn track_row_action_vm_disables_download_when_track_has_no_enclosure() {
        let track = Track {
            track_guid: Some("track-guid".into()),
            enclosure_url: None,
            ..Track::default()
        };
        let action = TrackRowActionVm::new(&track, false, false).primary_action();

        assert_eq!(action.kind, EntityActionKind::Download);
        assert!(!action.enabled);
    }

    #[test]
    fn artist_rows_merge_case_insensitive_counts() {
        let rows = vec![
            ResultRow::new(
                "feed",
                "feed-1",
                Some(EntityDetail::Feed(Feed {
                    release_artist: Some("Artist".into()),
                    ..Feed::default()
                })),
            ),
            ResultRow::new(
                "track",
                "track-1",
                Some(EntityDetail::Track(Track {
                    track_artist: Some("artist".into()),
                    ..Track::default()
                })),
            ),
        ];

        let artist_rows = artist_rows_from_result_rows(&rows, None);

        assert_eq!(artist_rows.len(), 1);
        let Some(EntityDetail::Artist(artist)) = &artist_rows[0].detail else {
            panic!("expected artist detail");
        };
        assert_eq!(artist.name.as_deref(), Some("Artist"));
        assert_eq!(artist.feed_count, Some(1));
        assert_eq!(artist.track_count, Some(1));
    }

    #[test]
    fn search_view_model_starts_with_all_filter_fuzzy_on_and_no_selection() {
        let vm = SearchViewModel::new();
        assert_eq!(vm.type_filter, 0);
        // Production default — `SearchApp::new` set fuzzy_search = true
        // and the VM mirrors that.
        assert!(vm.fuzzy_search);
        assert_eq!(vm.selected_key, None);
        assert_eq!(vm.inspector_origin, None);
    }

    #[test]
    fn search_view_model_starts_with_idle_panes_and_no_in_flight_tracks() {
        let vm = SearchViewModel::new();
        assert!(!vm.loading);
        assert!(vm.status.is_empty());
        assert_eq!(vm.cursor, None);
        assert!(!vm.has_more);
        assert!(vm.in_flight_tracks.is_empty());
        assert!(!vm.recent_loading);
        assert!(vm.recent_status.is_empty());
        assert!(!vm.recent_loaded_once);
        assert_eq!(vm.recent_cursor, None);
        assert!(!vm.recent_has_more);
        assert!(!vm.is_resizing());
        assert_width_eq(vm.split_pane_width(), DEFAULT_SPLIT_PANE_WIDTH);
    }

    #[test]
    fn track_inspector_header_vm_feed_link_url_falls_back_to_feed_guid() {
        let track = Track {
            feed_url: Some("https://example/x.rss".into()),
            feed_guid: Some("guid-1".into()),
            ..Track::default()
        };
        let vm = TrackInspectorHeaderVm::new(&track);
        assert_eq!(vm.feed_link_url().as_deref(), Some("https://example/x.rss"));

        let track = Track {
            feed_url: None,
            feed_guid: Some("guid-1".into()),
            ..Track::default()
        };
        let vm = TrackInspectorHeaderVm::new(&track);
        assert_eq!(vm.feed_link_url().as_deref(), Some("guid-1"));

        let track = Track::default();
        let vm = TrackInspectorHeaderVm::new(&track);
        assert_eq!(vm.feed_link_url(), None);
    }

    #[test]
    fn track_inspector_header_vm_feed_link_label_uses_feed_title_then_falls_back_to_guid() {
        let track = Track {
            feed_title: Some("Friendly Title".into()),
            feed_guid: Some("guid-1".into()),
            ..Track::default()
        };
        let vm = TrackInspectorHeaderVm::new(&track);
        assert_eq!(vm.feed_link_label("guid-1"), "Friendly Title");

        // Empty / whitespace-only feed_title falls back to the guid arg.
        let track = Track {
            feed_title: Some("   ".into()),
            feed_guid: Some("guid-1".into()),
            ..Track::default()
        };
        let vm = TrackInspectorHeaderVm::new(&track);
        assert_eq!(vm.feed_link_label("guid-1"), "guid-1");

        let track = Track {
            feed_title: None,
            ..Track::default()
        };
        let vm = TrackInspectorHeaderVm::new(&track);
        assert_eq!(vm.feed_link_label("fallback"), "fallback");
    }

    #[test]
    fn track_inspector_header_vm_projects_feed_link_display_contract() {
        let track = Track {
            feed_title: Some("Friendly Title".into()),
            feed_url: Some("https://example/x.rss".into()),
            feed_guid: Some("guid-1".into()),
            ..Track::default()
        };
        let vm = TrackInspectorHeaderVm::new(&track);
        assert_eq!(
            vm.feed_link_display(),
            Some(TrackFeedLinkDisplay {
                element_id: "track-feed-link:guid-1".into(),
                guid: "guid-1".into(),
                label: "Friendly Title".into(),
                url: Some("https://example/x.rss".into()),
                tooltip: "guid-1".into(),
            })
        );

        let track = Track {
            feed_title: Some("   ".into()),
            feed_url: None,
            feed_guid: Some("guid-1".into()),
            ..Track::default()
        };
        let vm = TrackInspectorHeaderVm::new(&track);
        assert_eq!(
            vm.feed_link_display(),
            Some(TrackFeedLinkDisplay {
                element_id: "track-feed-link:guid-1".into(),
                guid: "guid-1".into(),
                label: "guid-1".into(),
                url: Some("guid-1".into()),
                tooltip: "guid-1".into(),
            })
        );

        let track = Track {
            feed_guid: None,
            ..Track::default()
        };
        let vm = TrackInspectorHeaderVm::new(&track);
        assert_eq!(vm.feed_link_display(), None);
    }

    #[test]
    fn lazy_panel_collapsible_toggle_starts_fetch_toggles_and_ignores_loading() {
        let mut panel: LazyPanel<Vec<i32>> = LazyPanel::Hidden;
        let mut collapsed = true;

        let action = panel.begin_collapsible_toggle(&mut collapsed, false);
        assert_eq!(action, LazyPanelToggle::Fetch);
        assert!(action.should_fetch());
        assert!(action.should_notify());
        assert_eq!(panel, LazyPanel::Loading);
        assert!(!collapsed);

        let action = panel.begin_collapsible_toggle(&mut collapsed, false);
        assert_eq!(action, LazyPanelToggle::Ignored);
        assert!(!action.should_fetch());
        assert!(!action.should_notify());

        panel = LazyPanel::Loaded(vec![1]);
        let action = panel.begin_collapsible_toggle(&mut collapsed, false);
        assert_eq!(action, LazyPanelToggle::Toggled);
        assert!(!action.should_fetch());
        assert!(action.should_notify());
        assert!(collapsed);

        panel = LazyPanel::Empty("No items".into());
        let action = panel.begin_collapsible_toggle(&mut collapsed, false);
        assert_eq!(action, LazyPanelToggle::Toggled);
        assert!(!collapsed);
    }

    #[test]
    fn lazy_panel_force_toggle_only_never_starts_fetch() {
        let mut panel: LazyPanel<Vec<i32>> = LazyPanel::Hidden;
        let mut collapsed = true;

        let action = panel.begin_collapsible_toggle(&mut collapsed, true);

        assert_eq!(action, LazyPanelToggle::Toggled);
        assert_eq!(panel, LazyPanel::Hidden);
        assert!(!collapsed);
    }

    #[test]
    fn lazy_panel_from_items_result_maps_empty_loaded_and_error() {
        assert_eq!(
            LazyPanel::from_items_result(Result::<Vec<i32>, &str>::Ok(Vec::new()), "No rows"),
            LazyPanel::Empty("No rows".into())
        );
        assert_eq!(
            LazyPanel::from_items_result(Result::<Vec<i32>, &str>::Ok(vec![1, 2]), "No rows"),
            LazyPanel::Loaded(vec![1, 2])
        );
        assert_eq!(
            LazyPanel::from_items_result(Result::<Vec<i32>, &str>::Err("offline"), "No rows"),
            LazyPanel::Empty("Error: offline".into())
        );
    }

    #[test]
    fn lazy_panel_error_owns_error_prefix_display() {
        assert_eq!(
            LazyPanel::<Vec<i32>>::error("offline"),
            LazyPanel::Empty("Error: offline".into())
        );
    }

    #[test]
    fn payment_route_vm_falls_back_to_unnamed_recipient() {
        let r = api::PaymentRoute::default();
        let vm = PaymentRouteVm::new(&r);
        assert_eq!(vm.recipient_name(), "Unnamed recipient");
    }

    #[test]
    fn payment_route_vm_route_type_defaults_to_route() {
        let r = api::PaymentRoute::default();
        let vm = PaymentRouteVm::new(&r);
        assert_eq!(vm.route_type(), "route");
        let r = api::PaymentRoute {
            route_type: Some("lightning".into()),
            ..api::PaymentRoute::default()
        };
        let vm = PaymentRouteVm::new(&r);
        assert_eq!(vm.route_type(), "lightning");
    }

    #[test]
    fn payment_route_vm_projects_primary_summary() {
        let r = api::PaymentRoute::default();
        let vm = PaymentRouteVm::new(&r);
        assert_eq!(vm.summary(), "Unnamed recipient (route · 0% · split)");

        let r = api::PaymentRoute {
            recipient_name: Some("Alice".into()),
            route_type: Some("node".into()),
            split: Some(75.0),
            fee: Some(true),
            ..api::PaymentRoute::default()
        };
        let vm = PaymentRouteVm::new(&r);
        assert_eq!(vm.summary(), "Alice (node · 75% · fee)");
    }

    #[test]
    fn payment_route_vm_projects_address_without_coercing_presence() {
        let r = api::PaymentRoute {
            address: Some("lnbc1abc".into()),
            ..api::PaymentRoute::default()
        };
        let vm = PaymentRouteVm::new(&r);
        assert_eq!(vm.address().as_deref(), Some("lnbc1abc"));

        let r = api::PaymentRoute {
            address: Some(String::new()),
            ..api::PaymentRoute::default()
        };
        let vm = PaymentRouteVm::new(&r);
        assert_eq!(vm.address().as_deref(), Some(""));

        let r = api::PaymentRoute {
            address: None,
            ..api::PaymentRoute::default()
        };
        let vm = PaymentRouteVm::new(&r);
        assert_eq!(vm.address(), None);
    }

    #[test]
    fn payment_route_vm_projects_custom_fields_without_coercing_presence() {
        let r = api::PaymentRoute {
            custom_key: Some("pubkey".into()),
            custom_value: Some("abc".into()),
            ..api::PaymentRoute::default()
        };
        let vm = PaymentRouteVm::new(&r);
        assert_eq!(
            vm.custom_fields().as_deref(),
            Some("key pubkey · value abc")
        );

        let r = api::PaymentRoute {
            custom_key: Some(String::new()),
            custom_value: None,
            ..api::PaymentRoute::default()
        };
        let vm = PaymentRouteVm::new(&r);
        assert_eq!(vm.custom_fields().as_deref(), Some("key "));

        let r = api::PaymentRoute {
            custom_key: None,
            custom_value: None,
            ..api::PaymentRoute::default()
        };
        let vm = PaymentRouteVm::new(&r);
        assert_eq!(vm.custom_fields(), None);
    }

    #[test]
    fn payment_route_vm_classifies_fee_vs_split() {
        let r = api::PaymentRoute::default();
        let vm = PaymentRouteVm::new(&r);
        assert!(!vm.is_fee());
        assert_eq!(vm.kind_label(), "split");
        assert_eq!(vm.group(), "Recipients");
        assert_eq!(
            PaymentRouteVm::group_display(vm.group()),
            PaymentRouteGroupDisplay {
                heading: "Recipients"
            }
        );

        let r = api::PaymentRoute {
            fee: Some(true),
            ..api::PaymentRoute::default()
        };
        let vm = PaymentRouteVm::new(&r);
        assert!(vm.is_fee());
        assert_eq!(vm.kind_label(), "fee");
        assert_eq!(vm.group(), "Fees");
        assert_eq!(
            PaymentRouteVm::group_display(vm.group()),
            PaymentRouteGroupDisplay { heading: "Fees" }
        );
    }

    #[test]
    fn payment_route_vm_split_value_defaults_to_zero() {
        let r = api::PaymentRoute::default();
        let vm = PaymentRouteVm::new(&r);
        assert!((vm.split() - 0.0).abs() < f64::EPSILON);
        let r = api::PaymentRoute {
            split: Some(50.0),
            ..api::PaymentRoute::default()
        };
        let vm = PaymentRouteVm::new(&r);
        assert!((vm.split() - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn search_view_model_starts_with_empty_snapshots() {
        let vm = SearchViewModel::new();
        assert!(vm.results.is_empty());
        assert!(vm.recent_feeds.is_empty());
        assert!(vm.playlists.is_empty());
    }

    #[test]
    fn search_view_model_set_type_filter_updates_index_and_clears_when_unknown_type() {
        let mut vm = SearchViewModel::new();
        vm.set_type_filter(2);
        assert_eq!(vm.type_filter, 2);
        assert!(!vm.set_type_filter_if_changed(2));
        assert!(vm.set_type_filter_if_changed(3));
        assert_eq!(vm.type_filter, 3);
        // Out-of-range index stays at the prior value (caller is the
        // segmented control which knows its range).
        vm.set_type_filter(99);
        assert_eq!(vm.type_filter, 3);
        assert!(!vm.set_type_filter_if_changed(99));
        assert_eq!(vm.type_filter, 3);
    }

    #[test]
    fn search_type_filter_options_project_labels_and_values() {
        assert_eq!(
            SearchViewModel::type_filter_options(),
            [
                SearchTypeFilterOptionDisplay {
                    index: 0,
                    label: "All",
                    value: None,
                },
                SearchTypeFilterOptionDisplay {
                    index: 1,
                    label: "Artist",
                    value: Some("artist"),
                },
                SearchTypeFilterOptionDisplay {
                    index: 2,
                    label: "Feed",
                    value: Some("feed"),
                },
                SearchTypeFilterOptionDisplay {
                    index: 3,
                    label: "Track",
                    value: Some("track"),
                },
            ]
        );
        assert_eq!(SearchViewModel::type_filter_value(0), None);
        assert_eq!(SearchViewModel::type_filter_value(2), Some("feed"));
        assert_eq!(SearchViewModel::type_filter_value(99), None);
    }

    #[test]
    fn search_view_model_toggle_fuzzy_search_round_trip() {
        let mut vm = SearchViewModel::new();
        // Starts true (production default). Toggling once turns it off.
        vm.toggle_fuzzy_search();
        assert!(!vm.fuzzy_search);
        vm.toggle_fuzzy_search();
        assert!(vm.fuzzy_search);
    }

    #[test]
    fn search_view_model_inspector_origin_remembers_search_vs_recents() {
        let mut vm = SearchViewModel::new();
        vm.mark_inspector_from_search();
        assert_eq!(vm.inspector_origin, Some(InspectorOrigin::Search));
        vm.mark_inspector_from_recents();
        assert_eq!(vm.inspector_origin, Some(InspectorOrigin::Recents));
        vm.clear_inspector_origin();
        assert_eq!(vm.inspector_origin, None);
    }

    #[test]
    fn search_view_model_select_and_clear_selection() {
        let mut vm = SearchViewModel::new();
        vm.select("track:abc");
        assert_eq!(vm.selected_key.as_deref(), Some("track:abc"));
        vm.clear_selection();
        assert_eq!(vm.selected_key, None);
    }

    #[test]
    fn search_status_snapshot_prefixes_error_display() {
        let snapshot = SearchStatusSnapshot::from_text("Error: offline");
        assert_eq!(snapshot.text, "Error: offline");
        assert_eq!(snapshot.display_text, "\u{2717} Error: offline");
        assert!(snapshot.is_error);

        let snapshot = SearchStatusSnapshot::from_text("Ready");
        assert_eq!(snapshot.text, "Ready");
        assert_eq!(snapshot.display_text, "Ready");
        assert!(!snapshot.is_error);
    }

    #[test]
    fn search_input_display_projects_placeholder() {
        assert_eq!(
            SearchViewModel::search_input_display().placeholder,
            "Discover artists, feeds, and tracks..."
        );
    }

    #[test]
    fn search_render_snapshot_projects_result_pane_display_labels() {
        let mut vm = SearchViewModel::new();
        vm.status = "Error: offline".into();
        vm.loading = true;
        vm.has_more = true;
        vm.type_filter = 2;
        vm.fuzzy_search = false;
        vm.results.push(ResultRow::new("feed", "feed-1", None));
        vm.select_result("feed", "feed-1");

        let snapshot = vm.render_snapshot(true, true);

        assert_eq!(snapshot.status.text, "Error: offline");
        assert_eq!(snapshot.status.display_text, "\u{2717} Error: offline");
        assert!(snapshot.status.is_error);
        assert!(!snapshot.status.is_empty());
        assert_eq!(snapshot.pane_display.heading, "Search Index");
        assert_eq!(snapshot.pane_display.search_button_label, "Search Index");
        assert_eq!(snapshot.pane_display.fuzzy_toggle_label, "Fuzzy: Off");
        assert_eq!(snapshot.pane_display.empty_icon, "\u{1F50D}");
        assert_eq!(snapshot.pane_display.empty_label, "No results");
        assert_eq!(snapshot.pane_display.load_more_label, "Load more");
        assert_eq!(snapshot.rows.len(), 1);
        assert_eq!(snapshot.selected_key.as_deref(), Some("feed:feed-1"));
        assert_eq!(snapshot.type_filter, 2);
        assert!(!snapshot.show_recents_root);
        assert!(!snapshot.show_recents_command);
        assert!(snapshot.loading);
        assert!(!snapshot.empty);
        assert!(snapshot.has_more);
        assert!(!snapshot.fuzzy_search);

        let empty_snapshot = SearchViewModel::new().render_snapshot(true, true);
        assert!(empty_snapshot.show_recents_root);
        assert!(!empty_snapshot.show_recents_command);
        assert!(empty_snapshot.empty);
        assert!(empty_snapshot.status.is_empty());
        assert_eq!(empty_snapshot.status.display_text, "");
        assert_eq!(empty_snapshot.pane_display.split_pane_id, "pane-container");
        assert_eq!(
            empty_snapshot.pane_display.resize_handle_id,
            "resize-handle"
        );
        assert_eq!(empty_snapshot.pane_display.search_button_id, "search-btn");
        assert_eq!(empty_snapshot.pane_display.fuzzy_toggle_id, "fuzzy-toggle");
        assert_eq!(
            empty_snapshot.pane_display.results_scroll_id,
            "results-scroll"
        );
        assert_eq!(empty_snapshot.pane_display.load_more_button_id, "load-more");
        assert_eq!(empty_snapshot.pane_display.fuzzy_toggle_label, "Fuzzy: On");
    }

    #[test]
    fn search_render_snapshot_exposes_recent_feeds_command_after_search() {
        let mut vm = SearchViewModel::new();
        vm.results.push(ResultRow::new("feed", "feed-1", None));
        let snapshot = vm.render_snapshot(true, false);

        assert!(!snapshot.show_recents_root);
        assert!(snapshot.show_recents_command);
        assert_eq!(snapshot.pane_display.recents_button_id, "show-recents");
        assert_eq!(snapshot.pane_display.recents_button_label, "Recent Feeds");
    }

    #[test]
    fn recent_feeds_snapshot_projects_panel_display_labels() {
        let mut vm = SearchViewModel::new();
        vm.recent_feeds.push(Feed {
            feed_guid: Some("feed-1".into()),
            ..Feed::default()
        });
        vm.recent_status = "Loading recent feeds...".into();
        vm.recent_has_more = true;
        vm.recent_loading = true;

        let snapshot = vm.recent_feeds_snapshot();

        assert_eq!(snapshot.display.heading, "Recent Feeds");
        assert_eq!(snapshot.display.load_more_button_id, "recent-load-more");
        assert_eq!(snapshot.display.empty_label, "No recent feeds");
        assert_eq!(snapshot.display.load_more_label, "Load more");
        assert_eq!(snapshot.feeds.len(), 1);
        assert_eq!(snapshot.status, "Loading recent feeds...");
        assert!(snapshot.has_more);
        assert!(snapshot.loading);
        assert!(!snapshot.empty);
    }

    #[test]
    fn publisher_link_display_trims_title_and_tooltip() {
        let display = PublisherLinkDisplay::new("  Acme Audio  ");
        assert_eq!(display.id, "publisher-link:Acme Audio");
        assert_eq!(display.title, "Acme Audio");
        assert_eq!(display.target, "Acme Audio");
        assert_eq!(display.tooltip, "Open publisher: Acme Audio");
    }

    #[test]
    fn inspector_chrome_display_projects_back_and_empty_state() {
        let display = SearchViewModel::inspector_chrome_display();
        assert_eq!(display.back_button_id, "inspector-back");
        assert_eq!(display.scroll_id, "inspector-scroll");
        assert_eq!(display.back_label, "\u{2190} Back");
        assert_eq!(display.empty_icon, "\u{1F50D}");
        assert_eq!(display.empty_label, "Select a result to inspect");
    }

    #[test]
    fn inspector_status_messages_are_vm_owned() {
        assert_eq!(
            SearchViewModel::inspector_loading_message("Way to Go"),
            "Loading Way to Go..."
        );
        assert_eq!(
            SearchViewModel::inspector_error_message("offline"),
            "Error: offline"
        );
    }

    #[test]
    fn deferred_panel_display_projects_heading_and_loading_labels() {
        let contributors = SearchViewModel::deferred_panel_display(DeferredPanelKind::Contributors);
        assert_eq!(contributors.section_id, "section:contributors");
        assert_eq!(contributors.heading_label, "Contributors");
        assert_eq!(contributors.loading_label, "Loading contributors...");
        assert_eq!(contributors.empty_label, "No contributors found");

        let value_routes = SearchViewModel::deferred_panel_display(DeferredPanelKind::ValueRoutes);
        assert_eq!(value_routes.section_id, "section:value-routes");
        assert_eq!(value_routes.heading_label, "Value Routes");
        assert_eq!(value_routes.loading_label, "Loading value routes...");
        assert_eq!(value_routes.empty_label, "No value routes found");
    }

    #[test]
    fn deferred_panel_empty_line_projects_label() {
        assert_eq!(
            SearchViewModel::deferred_panel_empty_line("No value routes found"),
            "No value routes found"
        );
    }

    #[test]
    fn feed_header_display_filters_empty_subtitle() {
        assert_eq!(
            SearchViewModel::feed_header_display("Way to Go", Some("  Survival Guide  ")),
            SearchFeedHeaderDisplay {
                title: "Way to Go".into(),
                subtitle: Some("Survival Guide".into()),
            }
        );
        assert_eq!(
            SearchViewModel::feed_header_display("Way to Go", Some(" ... ")),
            SearchFeedHeaderDisplay {
                title: "Way to Go".into(),
                subtitle: None,
            }
        );
        assert_eq!(
            SearchViewModel::feed_header_display("Way to Go", None),
            SearchFeedHeaderDisplay {
                title: "Way to Go".into(),
                subtitle: None,
            }
        );
    }

    #[test]
    fn feed_inspector_tracks_defaults_missing_tracks_to_empty_list() {
        let feed = Feed::default();
        assert!(SearchViewModel::feed_inspector_tracks(&feed).is_empty());

        let feed = Feed {
            tracks: Some(vec![Track {
                title: Some("Track".into()),
                ..Track::default()
            }]),
            ..Feed::default()
        };
        assert_eq!(SearchViewModel::feed_inspector_tracks(&feed).len(), 1);
    }

    #[test]
    fn feed_list_section_display_projects_heading() {
        assert_eq!(
            SearchViewModel::feed_list_section_display(),
            SearchFeedListSectionDisplay { heading: "Feeds" }
        );
    }

    #[test]
    fn inspector_title_display_projects_recents_root_and_frame_title() {
        assert_eq!(
            SearchViewModel::inspector_title_display(true, None),
            "Recent Feeds"
        );
        assert_eq!(
            SearchViewModel::inspector_title_display(false, Some("Way to Go")),
            "Way to Go"
        );
        assert_eq!(SearchViewModel::inspector_title_display(false, None), "");
    }

    #[test]
    fn result_row_key_display_and_inspector_title_are_pure() {
        let row = ResultRow::new("feed", "feed-1", None);
        assert_eq!(row.key(), "feed:feed-1");
        let display = row.display();
        assert_eq!(display.element_id, "result-item:feed:feed-1");
        assert_eq!(display.kind_label, "feed");
        assert_eq!(display.line1, "feed-1");
        assert_eq!(row.inspector_title(), "feed-1");

        let item = row.render_item();
        assert_eq!(item.selection_key, "feed:feed-1");
        assert_eq!(item.display.element_id, "result-item:feed:feed-1");
        let (entity_type, entity_id, title) = item.navigation_target.into_parts();
        assert_eq!(
            (entity_type.as_str(), entity_id.as_str(), title.as_str()),
            ("feed", "feed-1", "feed-1")
        );
    }

    #[test]
    fn search_view_model_select_result_and_recent_feed_set_origin_and_key() {
        let mut vm = SearchViewModel::new();

        vm.select_result("track", "track-1");
        assert_eq!(vm.selected_key.as_deref(), Some("track:track-1"));
        assert_eq!(vm.inspector_origin, Some(InspectorOrigin::Search));

        vm.select_recent_feed("feed-1");
        assert_eq!(vm.selected_key.as_deref(), Some("feed:feed-1"));
        assert_eq!(vm.inspector_origin, Some(InspectorOrigin::Recents));
    }

    #[test]
    fn search_view_model_navigation_targets_follow_selection_and_clamp_edges() {
        let mut vm = SearchViewModel::new();
        vm.results = vec![
            ResultRow::new(
                "feed",
                "feed-1",
                Some(EntityDetail::Feed(Feed {
                    title: Some("First Feed".into()),
                    ..Feed::default()
                })),
            ),
            ResultRow::new(
                "track",
                "track-1",
                Some(EntityDetail::Track(Track {
                    name: Some("Second Track".into()),
                    ..Track::default()
                })),
            ),
        ];

        let (entity_type, entity_id, title) = vm
            .next_result_target()
            .expect("first result should be selected when no row is selected")
            .into_parts();
        assert_eq!(
            (entity_type.as_str(), entity_id.as_str(), title.as_str()),
            ("feed", "feed-1", "First Feed")
        );

        vm.select_result("track", "track-1");
        let (entity_type, entity_id, title) = vm
            .previous_result_target()
            .expect("previous result should move to first row")
            .into_parts();
        assert_eq!(
            (entity_type.as_str(), entity_id.as_str(), title.as_str()),
            ("feed", "feed-1", "First Feed")
        );

        let (entity_type, entity_id, title) = vm
            .next_result_target()
            .expect("next result should clamp at the final row")
            .into_parts();
        assert_eq!(
            (entity_type.as_str(), entity_id.as_str(), title.as_str()),
            ("track", "track-1", "Second Track")
        );
    }

    #[test]
    fn search_view_model_endpoint_reset_clears_snapshots_and_marks_status() {
        let mut vm = SearchViewModel::new();
        vm.results.push(ResultRow::new("feed", "f1", None));
        vm.loading = true;
        vm.status = "Searching...".into();
        vm.cursor = Some("cursor".into());
        vm.has_more = true;
        vm.select("feed:f1");
        vm.mark_inspector_from_search();
        vm.recent_feeds.push(Feed::default());
        vm.recent_cursor = Some("recent".into());
        vm.recent_has_more = true;
        vm.recent_loaded_once = true;
        vm.recent_status = "Loaded".into();

        vm.reset_for_endpoint_change();

        assert!(vm.results.is_empty());
        assert!(!vm.loading);
        assert_eq!(vm.status, "MusicIndex endpoint updated");
        assert_eq!(vm.cursor, None);
        assert!(!vm.has_more);
        assert_eq!(vm.selected_key, None);
        assert_eq!(vm.inspector_origin, None);
        assert!(vm.recent_feeds.is_empty());
        assert_eq!(vm.recent_cursor, None);
        assert!(!vm.recent_has_more);
        assert!(!vm.recent_loaded_once);
        assert!(vm.recent_status.is_empty());
    }

    #[test]
    fn search_view_model_return_to_recent_feeds_clears_search_pane() {
        let mut vm = SearchViewModel::new();
        vm.results.push(ResultRow::new("feed", "f1", None));
        vm.loading = true;
        vm.status = "Searching...".into();
        vm.cursor = Some("cursor".into());
        vm.has_more = true;
        vm.select("feed:f1");
        vm.mark_inspector_from_search();

        assert!(vm.return_to_recent_feeds());

        assert!(vm.results.is_empty());
        assert!(!vm.loading);
        assert!(vm.status.is_empty());
        assert_eq!(vm.cursor, None);
        assert!(!vm.has_more);
        assert_eq!(vm.selected_key, None);
        assert_eq!(vm.inspector_origin, None);

        vm.recent_loaded_once = true;
        assert!(!vm.return_to_recent_feeds());
    }

    #[test]
    fn search_view_model_recent_feed_load_intent_tracks_append_cursor() {
        let mut vm = SearchViewModel::new();
        vm.recent_feeds.push(Feed::default());
        vm.recent_cursor = Some("next".into());
        vm.recent_has_more = true;

        let intent = vm
            .begin_recent_feed_load(true)
            .expect("idle VM should begin recent load");

        assert_eq!(intent.into_cursor().as_deref(), Some("next"));
        assert!(vm.recent_loading);
        assert_eq!(vm.recent_status, "Loading more recent feeds...");
        assert_eq!(vm.recent_feeds.len(), 1);
        assert!(vm.begin_recent_feed_load(true).is_none());
    }

    #[test]
    fn search_view_model_recent_feed_fresh_load_resets_prior_page() {
        let mut vm = SearchViewModel::new();
        vm.recent_feeds.push(Feed::default());
        vm.recent_cursor = Some("next".into());
        vm.recent_has_more = true;

        let intent = vm
            .begin_recent_feed_load(false)
            .expect("idle VM should begin recent load");

        assert_eq!(intent.into_cursor(), None);
        assert!(vm.recent_feeds.is_empty());
        assert_eq!(vm.recent_cursor, None);
        assert!(!vm.recent_has_more);
        assert_eq!(vm.recent_status, "Loading recent feeds...");
    }

    #[test]
    fn search_view_model_recent_feed_finish_and_fail_update_state() {
        let mut vm = SearchViewModel::new();
        assert!(vm.begin_recent_feed_load(false).is_some());

        vm.finish_recent_feed_load(api::RecentFeedsResponse {
            data: vec![Feed::default()],
            pagination: api::Pagination {
                has_more: true,
                cursor: Some("next".into()),
            },
        });

        assert!(!vm.recent_loading);
        assert!(vm.recent_loaded_once);
        assert_eq!(vm.recent_feeds.len(), 1);
        assert_eq!(vm.recent_cursor.as_deref(), Some("next"));
        assert!(vm.recent_has_more);
        assert!(vm.recent_status.is_empty());

        assert!(vm.begin_recent_feed_load(true).is_some());
        vm.fail_recent_feed_load("offline");

        assert!(!vm.recent_loading);
        assert!(vm.recent_loaded_once);
        assert_eq!(vm.recent_status, "Error: offline");
    }

    #[test]
    fn normalized_search_query_rejects_non_search_terms() {
        assert_eq!(normalized_search_query("  feed  ").as_deref(), Some("feed"));
        assert_eq!(
            normalized_search_query("  c++ music  ").as_deref(),
            Some("c++ music")
        );
        assert_eq!(normalized_search_query(r"\"), None);
        assert_eq!(normalized_search_query("  ***  "), None);
        assert_eq!(normalized_search_query(" \n\t "), None);
    }

    #[test]
    fn search_view_model_begin_search_load_sets_status_and_intent() {
        let mut vm = SearchViewModel::new();
        vm.set_type_filter(2);
        vm.fuzzy_search = false;
        vm.cursor = Some("next".into());
        vm.results.push(ResultRow::new("feed", "old", None));
        vm.select("feed:old");
        vm.mark_inspector_from_search();

        let intent = vm
            .begin_search_load(false)
            .expect("idle VM should begin a fresh search");

        assert_eq!(intent.type_filter(), 2);
        assert_eq!(intent.cursor(), None);
        assert!(!intent.fuzzy());
        assert!(vm.loading);
        assert_eq!(vm.status, "Discovering...");
        assert!(vm.results.is_empty());
        assert_eq!(vm.cursor, None);
        assert_eq!(vm.selected_key, None);
        assert_eq!(vm.inspector_origin, None);
        assert!(vm.begin_search_load(false).is_none());
    }

    #[test]
    fn search_view_model_begin_search_append_preserves_existing_results() {
        let mut vm = SearchViewModel::new();
        vm.cursor = Some("next".into());
        vm.results.push(ResultRow::new("feed", "old", None));

        let intent = vm
            .begin_search_load(true)
            .expect("idle VM should begin an append search");

        assert_eq!(intent.cursor(), Some("next"));
        assert_eq!(intent.type_filter(), 0);
        assert!(intent.fuzzy());
        assert_eq!(vm.status, "Loading more...");
        assert_eq!(vm.results.len(), 1);
    }

    #[test]
    fn search_view_model_finish_search_load_formats_counts_and_cursor() {
        let mut vm = SearchViewModel::new();
        assert!(vm.begin_search_load(false).is_some());

        vm.finish_search_load(
            SearchBatch {
                rows: vec![ResultRow::new("feed", "f1", None)],
                has_more: true,
                cursor: Some("next".into()),
            },
            false,
        );

        assert!(!vm.loading);
        assert_eq!(vm.results.len(), 1);
        assert_eq!(vm.cursor.as_deref(), Some("next"));
        assert!(vm.has_more);
        assert_eq!(vm.status, "1 result+");
    }

    #[test]
    fn search_view_model_finish_search_append_dedupes_existing_rows() {
        let mut vm = SearchViewModel::new();
        vm.results.push(ResultRow::new("feed", "f1", None));
        assert!(vm.begin_search_load(true).is_some());

        vm.finish_search_load(
            SearchBatch {
                rows: vec![
                    ResultRow::new("feed", "f1", None),
                    ResultRow::new("track", "t1", None),
                ],
                has_more: false,
                cursor: None,
            },
            true,
        );

        assert_eq!(vm.results.len(), 2);
        assert_eq!(vm.status, "2 results");
    }

    #[test]
    fn search_view_model_finish_empty_fresh_search_clears_state() {
        let mut vm = SearchViewModel::new();
        assert!(vm.begin_search_load(false).is_some());

        vm.finish_search_load(
            SearchBatch {
                rows: Vec::new(),
                has_more: false,
                cursor: Some("ignored".into()),
            },
            false,
        );

        assert!(!vm.loading);
        assert!(vm.results.is_empty());
        assert_eq!(vm.cursor, None);
        assert!(!vm.has_more);
        assert!(vm.status.is_empty());
    }

    #[test]
    fn search_view_model_fail_search_load_sets_error_status() {
        let mut vm = SearchViewModel::new();
        assert!(vm.begin_search_load(false).is_some());

        vm.fail_search_load("offline");

        assert!(!vm.loading);
        assert_eq!(vm.status, "Error: offline");
    }

    #[test]
    fn search_view_model_merges_artist_result_detail_for_matching_result() {
        let mut vm = SearchViewModel::new();
        vm.results = vec![
            ResultRow::new(
                "artist",
                "artist-1",
                Some(EntityDetail::Artist(Artist {
                    name: Some("Artist".into()),
                    track_count: Some(1),
                    feed_count: Some(1),
                    image_url: Some("old.png".into()),
                    ..Artist::default()
                })),
            ),
            ResultRow::new(
                "artist",
                "artist-2",
                Some(EntityDetail::Artist(Artist {
                    track_count: Some(2),
                    feed_count: Some(2),
                    image_url: Some("keep.png".into()),
                    ..Artist::default()
                })),
            ),
        ];

        vm.merge_artist_result_detail(
            "artist-1",
            &Artist {
                track_count: Some(10),
                feed_count: Some(3),
                image_url: Some("new.png".into()),
                ..Artist::default()
            },
        );

        let Some(EntityDetail::Artist(artist)) = &vm.results[0].detail else {
            panic!("expected artist detail");
        };
        assert_eq!(artist.track_count, Some(10));
        assert_eq!(artist.feed_count, Some(3));
        assert_eq!(artist.image_url.as_deref(), Some("new.png"));

        let Some(EntityDetail::Artist(other_artist)) = &vm.results[1].detail else {
            panic!("expected artist detail");
        };
        assert_eq!(other_artist.track_count, Some(2));
        assert_eq!(other_artist.feed_count, Some(2));
        assert_eq!(other_artist.image_url.as_deref(), Some("keep.png"));
    }

    #[test]
    fn search_view_model_playlist_snapshot_and_failures_update_status() {
        let mut vm = SearchViewModel::new();
        let mut playlist = playlist("Focus");
        playlist.id = 12;

        vm.replace_playlists(vec![playlist]);
        assert_eq!(vm.playlists.len(), 1);

        vm.fail_playlist_load("db");
        assert_eq!(vm.status, "Error loading playlists: db");
        vm.fail_feed_subscription("offline");
        assert_eq!(vm.status, "Error subscribing feed: offline");
        vm.fail_feed_tracks_load("db");
        assert_eq!(vm.status, "Error loading feed tracks: db");
        vm.set_feed_has_no_tracks();
        assert_eq!(vm.status, "Feed has no tracks");
        vm.fail_playlist_create("exists");
        assert_eq!(vm.status, "Create playlist: exists");
        vm.set_track_not_in_library();
        assert_eq!(vm.status, "Track not in local library");
    }

    #[test]
    fn search_view_model_playlist_append_intent_and_finish_format_status() {
        let mut vm = SearchViewModel::new();
        let mut playlist = playlist("Focus");
        playlist.id = 12;
        vm.replace_playlists(vec![playlist]);

        let intent = vm
            .begin_playlist_append(12, vec![7, 8])
            .expect("non-empty track ids should build an append intent");

        assert_eq!(intent.playlist_id(), 12);
        assert_eq!(intent.track_ids(), &[7, 8]);
        assert_eq!(intent.total_tracks(), 2);
        assert_eq!(intent.playlist_name(), "Focus");
        assert_eq!(vm.status, "Downloading 2 tracks...");

        vm.finish_playlist_append(&intent, PlaylistAppendOutcome::new(1, 1, 1));
        assert_eq!(vm.status, "Added 1 of 2 to Focus (downloaded 1); 1 failed");
    }

    #[test]
    fn search_view_model_playlist_append_ignores_empty_and_formats_failure() {
        let mut vm = SearchViewModel::new();
        vm.status = "Ready".into();

        assert!(vm.begin_playlist_append(12, Vec::new()).is_none());
        assert_eq!(vm.status, "Ready");

        vm.fail_playlist_append("offline");
        assert_eq!(vm.status, "Error adding to playlist: offline");
    }

    #[test]
    fn search_view_model_track_operation_rejects_empty_and_duplicate_keys() {
        let mut vm = SearchViewModel::new();

        assert!(!vm.begin_track_operation(""));
        assert!(!vm.is_track_operation_in_flight(""));
        assert!(vm.begin_track_operation("track:1"));
        assert!(vm.is_track_operation_in_flight("track:1"));
        assert!(!vm.begin_track_operation("track:1"));
    }

    #[test]
    fn search_subscription_command_formats_begin_and_error_messages() {
        assert_eq!(
            SearchSubscriptionCommand::Download.begin_message(),
            "Downloading..."
        );
        assert_eq!(
            SearchSubscriptionCommand::track_download_success_message(),
            "Downloaded track"
        );
        assert_eq!(
            SearchSubscriptionCommand::Remove.begin_message(),
            "Removing..."
        );
        assert_eq!(
            SearchSubscriptionCommand::Download.error_message("offline"),
            "Download error: offline"
        );
        assert_eq!(
            SearchSubscriptionCommand::Remove.error_message("locked"),
            "Remove error: locked"
        );
        assert_eq!(
            SearchSubscriptionCommand::Download.success_message(0),
            "Downloaded track"
        );
        assert_eq!(
            SearchSubscriptionCommand::Download.success_message(2),
            "Downloaded track, applied 2 ID3 edits"
        );
        assert_eq!(
            SearchSubscriptionCommand::Remove.success_message(0),
            "Removed track"
        );
    }

    #[test]
    fn search_view_model_finishes_track_download_and_remove_operations() {
        let mut vm = SearchViewModel::new();

        assert!(vm.begin_track_operation("track:1"));
        vm.finish_track_download("track:1", "Downloaded");
        assert!(!vm.is_track_operation_in_flight("track:1"));
        assert_eq!(vm.status, "Downloaded");

        assert!(vm.begin_track_operation("track:2"));
        vm.finish_track_remove("track:2", "Removed");
        assert!(!vm.is_track_operation_in_flight("track:2"));
        assert_eq!(vm.status, "Removed");
    }

    #[test]
    fn search_view_model_fails_track_operations_with_contextual_status() {
        let mut vm = SearchViewModel::new();

        assert!(vm.begin_track_operation("track:1"));
        vm.fail_track_download("track:1", "offline");
        assert!(!vm.is_track_operation_in_flight("track:1"));
        assert_eq!(vm.status, "Download error: offline");

        assert!(vm.begin_track_operation("track:2"));
        vm.fail_track_remove("track:2", "locked");
        assert!(!vm.is_track_operation_in_flight("track:2"));
        assert_eq!(vm.status, "Remove error: locked");
    }

    #[test]
    fn search_view_model_tracks_resize_lifecycle() {
        let mut vm = SearchViewModel::new();

        assert!(!vm.is_resizing());
        vm.begin_resize();
        assert!(vm.is_resizing());
        vm.end_resize();
        assert!(!vm.is_resizing());
    }

    #[test]
    fn search_view_model_clamps_split_pane_width() {
        let mut vm = SearchViewModel::new();

        vm.resize_split_pane(120.0, 200.0, 800.0);
        assert_width_eq(vm.split_pane_width(), 200.0);

        vm.resize_split_pane(900.0, 200.0, 800.0);
        assert_width_eq(vm.split_pane_width(), 800.0);

        vm.resize_split_pane(420.0, 200.0, 800.0);
        assert_width_eq(vm.split_pane_width(), 420.0);
    }
}
