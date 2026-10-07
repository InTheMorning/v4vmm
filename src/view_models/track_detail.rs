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
use crate::views::{FeedRef, TrackRef, TrackView};

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

    /// ADR 0075 packet 050, operator decision D50-1: the label of the
    /// track's feed publication-date fallback row.
    #[must_use]
    pub const fn feed_publication_date_label(self) -> &'static str {
        "Feed publication date"
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
    /// The Library state that selects the page actions (ADR 0083 Decision
    /// 5). `None` for a track that is not in the Library.
    library_state: Option<LibraryTrackActionState>,
    /// The Library feed id of the track's album, for the album name link.
    album_feed_id: Option<i64>,
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
            library_state: None,
            album_feed_id: None,
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

    /// Supplies the Library feed id of the track's album, so the album name
    /// links to the Library album page.
    #[must_use]
    pub const fn with_album_feed_id(mut self, album_feed_id: i64) -> Self {
        self.album_feed_id = Some(album_feed_id);
        self
    }

    /// Supplies the Library state of the track. A track without this state
    /// is not in the Library, and gets the actions of an Index track.
    #[must_use]
    pub const fn with_library_state(mut self, state: LibraryTrackActionState) -> Self {
        self.library_state = Some(state);
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

    /// The feed's own publication date, shown apart from the track's own
    /// date (ADR 0075 packet 050, Track publication-date fallback). `None`
    /// when the track has a date of its own: this row is a fallback only,
    /// and it creates no track-owned date assertion. `None` also when this
    /// surface has no feed, or when the feed has no publication date.
    #[must_use]
    pub fn feed_publication_date_display(&self) -> Option<String> {
        if self.track.pub_date.is_some() {
            return None;
        }
        let (date, source) = crate::metadata::feed_publication_pubdate(self.feed?)?;
        Some(format!("{date} ({source})"))
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
        push_optional(
            &mut rows,
            labels.feed_publication_date_label(),
            self.feed_publication_date_display(),
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

    /// The name links under the title (ADR 0083 Decision 5: navigation is a
    /// link on the text). The album name links to its album page. The
    /// publisher name links to the publisher page of the album feed (ADR
    /// 0077 packet 004). A name with no page target gives no link.
    #[must_use]
    pub fn name_links(&self) -> Vec<TrackNameLinkVm> {
        let mut links = Vec::new();
        let album_target = match self.context {
            TrackDetailSurfaceContext::Library => {
                self.album_feed_id.map(TrackNameLinkTarget::LibraryAlbum)
            }
            TrackDetailSurfaceContext::Discover => self
                .track
                .feed_guid
                .as_deref()
                .and_then(nonempty)
                .map(|feed_guid| TrackNameLinkTarget::IndexAlbum {
                    feed_guid: feed_guid.to_owned(),
                }),
        };
        if let Some(target) = album_target {
            let label = self.display_album();
            links.push(TrackNameLinkVm {
                a11y_label: format!("Open the album {label}"),
                label,
                target,
            });
        }
        if let Some(publisher_feed_guid) = self.publisher_feed_guid.and_then(nonempty) {
            let label = self
                .publisher_display()
                .unwrap_or_else(|| PUBLISHER_LINK_FALLBACK.to_owned());
            links.push(TrackNameLinkVm {
                a11y_label: format!("Open the publisher {label}"),
                label,
                target: TrackNameLinkTarget::Publisher(publisher_feed_guid.to_owned()),
            });
        }
        links
    }

    /// The credits of the track in source order, each name and role one
    /// time. The Library list is the projected list of ADR 0076 task 006.
    /// A credit carries no provider label.
    #[must_use]
    pub fn credits(&self) -> Vec<TrackCreditVm> {
        let mut credits: Vec<TrackCreditVm> = Vec::new();
        for contributor in &self.track.contributors {
            let Some(name) = contributor.name.as_deref().and_then(nonempty) else {
                continue;
            };
            let role = contributor
                .role
                .as_deref()
                .and_then(nonempty)
                .unwrap_or(CREDIT_ROLE_FALLBACK);
            let credit = TrackCreditVm {
                role: role.to_owned(),
                name: name.to_owned(),
            };
            if !credits.contains(&credit) {
                credits.push(credit);
            }
        }
        credits
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

    /// The feed URL of the track's feed, from the feed record. `None` when
    /// the caller supplied no feed or the feed states no URL.
    #[must_use]
    pub fn feed_url(&self) -> Option<&'a str> {
        self.feed
            .and_then(|feed| feed.feed_url.as_deref())
            .and_then(nonempty)
    }

    /// The page actions of ADR 0083 Decision 5, with the main action of the
    /// 2026-10-07 amendment: the next curation step for the track state.
    #[must_use]
    pub fn page_actions(&self) -> TrackPageActions {
        let copy_feed_url = self
            .feed_url()
            .map(|_| TrackPageActionDisplay::new(TrackPageAction::CopyFeedUrl, true));
        let Some(state) = self.library_state else {
            let has_album = self.track.feed_guid.as_deref().and_then(nonempty).is_some();
            return TrackPageActions {
                filled: TrackPageActionDisplay::new(TrackPageAction::DownloadAlbum, has_album),
                plain: Vec::new(),
                menu: copy_feed_url.into_iter().collect(),
            };
        };
        if !state.downloaded {
            return TrackPageActions {
                filled: TrackPageActionDisplay::new(
                    TrackPageAction::DownloadTrack,
                    !state.subscription_busy,
                ),
                plain: vec![TrackPageActionDisplay::new(
                    TrackPageAction::AddToPlaylist,
                    true,
                )],
                menu: copy_feed_url.into_iter().collect(),
            };
        }
        let mut menu: Vec<_> = copy_feed_url.into_iter().collect();
        menu.push(TrackPageActionDisplay::new(
            TrackPageAction::MusicBrainzLookup,
            state.musicbrainz_available,
        ));
        menu.push(TrackPageActionDisplay::new(
            TrackPageAction::RemoveTrack,
            !state.subscription_busy,
        ));
        TrackPageActions {
            filled: TrackPageActionDisplay::new(TrackPageAction::AddToPlaylist, true),
            plain: Vec::new(),
            menu,
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

    /// The page actions of ADR 0083 Decision 5 (R83-42).
    #[must_use]
    pub fn page_actions(&self) -> TrackPageActions {
        self.detail.page_actions()
    }

    /// The feed URL that "Copy feed URL" copies.
    #[must_use]
    pub fn feed_url(&self) -> Option<&'a str> {
        self.detail.feed_url()
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

/// The Library state of a track that selects its page actions (ADR 0083
/// Decision 5).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LibraryTrackActionState {
    /// The track has a local subscription and a downloaded file.
    pub downloaded: bool,
    /// A download or a removal of the track is in progress.
    pub subscription_busy: bool,
    /// A `MusicBrainz` lookup can run for the downloaded file.
    pub musicbrainz_available: bool,
}

/// One action of a track page. The renderer maps each variant to its
/// command (ADR 0083 Decision 5).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum TrackPageAction {
    /// Download the album feed of a track that is not in the Library.
    DownloadAlbum,
    /// Download a Library track that has no local file.
    DownloadTrack,
    /// Add the track to a playlist.
    AddToPlaylist,
    /// Copy the feed URL of the track's feed.
    CopyFeedUrl,
    /// Look up the downloaded file in `MusicBrainz`.
    MusicBrainzLookup,
    /// Remove the track from the Library, after a confirmation.
    RemoveTrack,
}

impl TrackPageAction {
    /// The visible label. A label that opens a confirmation ends with "…".
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::DownloadAlbum => "Download album",
            Self::DownloadTrack => "Download track",
            Self::AddToPlaylist => "Add to playlist",
            Self::CopyFeedUrl => "Copy feed URL",
            Self::MusicBrainzLookup => "MusicBrainz lookup",
            Self::RemoveTrack => "Remove track…",
        }
    }

    /// The accessibility label.
    #[must_use]
    pub const fn a11y_label(self) -> &'static str {
        match self {
            Self::DownloadAlbum => "Download the album of this track",
            Self::DownloadTrack => "Download this track",
            Self::AddToPlaylist => "Add this track to a playlist",
            Self::CopyFeedUrl => "Copy the feed URL of this track",
            Self::MusicBrainzLookup => "Look up this track in MusicBrainz",
            Self::RemoveTrack => "Remove this track from the Library",
        }
    }

    /// A destructive action is last in its menu and asks for confirmation.
    #[must_use]
    pub const fn is_destructive(self) -> bool {
        matches!(self, Self::RemoveTrack)
    }
}

/// A display-ready track page action with its typed availability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrackPageActionDisplay {
    /// The action.
    pub action: TrackPageAction,
    /// The action can run now.
    pub available: bool,
}

impl TrackPageActionDisplay {
    /// Make a display for `action`.
    #[must_use]
    pub const fn new(action: TrackPageAction, available: bool) -> Self {
        Self { action, available }
    }
}

/// The actions of a track page in the order of ADR 0083 Decision 5: one
/// filled button, at most two plain buttons, and the "⋯" menu.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrackPageActions {
    /// The one filled button.
    pub filled: TrackPageActionDisplay,
    /// The plain buttons.
    pub plain: Vec<TrackPageActionDisplay>,
    /// The "⋯" menu items. A destructive item is last.
    pub menu: Vec<TrackPageActionDisplay>,
}

/// The label of a publisher link when the album states no publisher name.
const PUBLISHER_LINK_FALLBACK: &str = "Publisher";
/// The role of a credit that states no role.
const CREDIT_ROLE_FALLBACK: &str = "Credit";
/// The heading of the credits section.
pub const CREDITS_LABEL: &str = "Credits";

/// The page that a name link under the track title opens.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TrackNameLinkTarget {
    /// The Library album page of a feed id.
    LibraryAlbum(i64),
    /// The Index album page of a feed GUID.
    IndexAlbum {
        /// The feed GUID of the album.
        feed_guid: String,
    },
    /// The publisher page of a publisher feed GUID (ADR 0077 Decision 1).
    Publisher(String),
}

/// A name under the track title that links to a page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrackNameLinkVm {
    /// The visible name.
    pub label: String,
    /// The accessibility label.
    pub a11y_label: String,
    /// The page that the link opens.
    pub target: TrackNameLinkTarget,
}

/// One credit of a track: a role and a name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrackCreditVm {
    /// The role, for example "Vocals".
    pub role: String,
    /// The name of the person or group.
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{SourceEntityId, SourceEntityLink, SourceReleaseClaim};
    use crate::views::ContributorView;
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

    /// R50-06 (ADR 0075 packet 050): a track without its own date exposes a
    /// separate "Feed publication date", and it still gives no "Release
    /// Date" value.
    #[test]
    fn adr_0075_feed_dates_r50_06_track_without_own_date_shows_feed_publication_date() {
        let mut track = track();
        track.pub_date = None;
        let feed = Feed {
            channel_pub_date: Some(1_758_369_600),
            ..Default::default()
        };
        let vm = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Discover)
            .with_feed_identity(Some(&feed));
        let rows = vm.summary_rows();

        assert!(rows.iter().all(|row| row.label != "Release Date"));
        assert!(rows
            .iter()
            .any(|row| row.label == "Feed publication date" && row.value == "Sep 20, 2025 (RSS)"));
    }

    /// R50-06: a track with its own date shows no "Feed publication date"
    /// row, even when its feed has a publication date of its own.
    #[test]
    fn adr_0075_feed_dates_r50_06_track_with_own_date_shows_no_feed_publication_date() {
        let track = track();
        let feed = Feed {
            channel_pub_date: Some(1_758_369_600),
            ..Default::default()
        };
        let vm = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Discover)
            .with_feed_identity(Some(&feed));

        assert!(vm
            .summary_rows()
            .iter()
            .all(|row| row.label != "Feed publication date"));
    }

    /// R50-06: a feed publication date read from a MusicIndex
    /// `release_date` claim with the path `feed.pub_date` also supplies the
    /// track's fallback row.
    #[test]
    fn adr_0075_feed_dates_r50_06_feed_publication_date_claim_supplies_fallback() {
        let mut track = track();
        track.pub_date = None;
        let feed = Feed {
            source_release_claims: Some(vec![SourceReleaseClaim {
                claim_type: Some("release_date".into()),
                claim_value: Some("1704067200".into()),
                extraction_path: Some("feed.pub_date".into()),
                ..Default::default()
            }]),
            ..Default::default()
        };
        let vm = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Discover)
            .with_feed_identity(Some(&feed));

        assert!(vm
            .summary_rows()
            .iter()
            .any(|row| row.label == "Feed publication date"
                && row.value == "Jan 1, 2024 (MusicIndex)"));
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

    /// R83-44 and R4-03: the publisher name links to the publisher page of
    /// the album feed. The track alone gives no publisher link.
    #[test]
    fn adr_0083_r83_44_name_links_open_the_album_and_the_publisher() {
        let track = track_with_identity();

        let links = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Library)
            .with_album_feed_id(9)
            .with_publisher_feed_guid(Some("publisher-guid"))
            .name_links();
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].target, TrackNameLinkTarget::LibraryAlbum(9));
        assert_eq!(
            links[1].target,
            TrackNameLinkTarget::Publisher("publisher-guid".into())
        );

        let without_targets =
            TrackDetailVm::new(&track, TrackDetailSurfaceContext::Library).name_links();
        assert!(
            without_targets.is_empty(),
            "a name with no page target gives no link"
        );
    }

    /// R83-44: an Index track links its album name to the Index album page.
    #[test]
    fn adr_0083_r83_44_index_album_name_links_to_the_index_album() {
        let mut track = track();
        track.feed_guid = Some("feed-guid".into());
        let links = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Discover).name_links();
        assert_eq!(
            links[0].target,
            TrackNameLinkTarget::IndexAlbum {
                feed_guid: "feed-guid".into()
            }
        );
    }

    /// R83-45: the credits keep source order, show each name and role one
    /// time, and skip a credit with no name.
    #[test]
    fn adr_0083_r83_45_credits_keep_source_order_without_duplicates() {
        let mut track = track();
        let credit = |name: Option<&str>, role: Option<&str>| ContributorView {
            name: name.map(str::to_owned),
            role: role.map(str::to_owned),
            group_name: None,
            href: None,
            image_url: None,
            nostr_npub: None,
        };
        track.contributors = vec![
            credit(Some("Zed"), Some("Vocals")),
            credit(Some("Amy"), Some("Guitar")),
            credit(Some("Zed"), Some("Vocals")),
            credit(None, Some("Drums")),
            credit(Some("Bo"), None),
        ];
        let credits = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Library).credits();
        let pairs: Vec<_> = credits
            .iter()
            .map(|credit| (credit.role.as_str(), credit.name.as_str()))
            .collect();
        assert_eq!(
            pairs,
            vec![("Vocals", "Zed"), ("Guitar", "Amy"), ("Credit", "Bo")]
        );
    }

    fn feed_with_url() -> Feed {
        Feed {
            feed_url: Some("https://example.test/feed.xml".into()),
            ..Feed::default()
        }
    }

    fn actions_of(menu: &[TrackPageActionDisplay]) -> Vec<TrackPageAction> {
        menu.iter().map(|item| item.action).collect()
    }

    /// R83-42: a track that is not in the Library gets "Download album",
    /// and "Copy feed URL" only when its feed states a URL.
    #[test]
    fn adr_0083_r83_42_index_track_actions() {
        let track = track();
        let without_feed = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Discover);
        let actions = without_feed.page_actions();
        assert_eq!(actions.filled.action, TrackPageAction::DownloadAlbum);
        assert!(
            !actions.filled.available,
            "a track with no album feed GUID cannot download its album"
        );
        assert!(actions.plain.is_empty());
        assert!(actions.menu.is_empty());

        let feed = feed_with_url();
        let with_feed = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Discover)
            .with_feed_identity(Some(&feed));
        assert_eq!(
            actions_of(&with_feed.page_actions().menu),
            vec![TrackPageAction::CopyFeedUrl]
        );
    }

    /// R83-42: a Library track with no local file gets "Download track" and
    /// "Add to playlist". A busy download makes "Download track" unavailable.
    #[test]
    fn adr_0083_r83_42_library_track_not_downloaded_actions() {
        let track = track();
        let feed = feed_with_url();
        let busy = LibraryTrackActionState {
            subscription_busy: true,
            ..LibraryTrackActionState::default()
        };
        let actions = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Library)
            .with_feed_identity(Some(&feed))
            .with_library_state(busy)
            .page_actions();
        assert_eq!(actions.filled.action, TrackPageAction::DownloadTrack);
        assert!(!actions.filled.available);
        assert_eq!(
            actions_of(&actions.plain),
            vec![TrackPageAction::AddToPlaylist]
        );
        assert_eq!(
            actions_of(&actions.menu),
            vec![TrackPageAction::CopyFeedUrl]
        );
    }

    /// R83-42 and R83-43: a downloaded track gets "Add to playlist" as the
    /// filled action. "Remove track…" is the last menu item and destructive.
    #[test]
    fn adr_0083_r83_42_r83_43_downloaded_track_actions() {
        let track = track();
        let feed = feed_with_url();
        let state = LibraryTrackActionState {
            downloaded: true,
            subscription_busy: false,
            musicbrainz_available: false,
        };
        let actions = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Library)
            .with_feed_identity(Some(&feed))
            .with_library_state(state)
            .page_actions();
        assert_eq!(actions.filled.action, TrackPageAction::AddToPlaylist);
        assert!(actions.plain.is_empty());
        assert_eq!(
            actions_of(&actions.menu),
            vec![
                TrackPageAction::CopyFeedUrl,
                TrackPageAction::MusicBrainzLookup,
                TrackPageAction::RemoveTrack,
            ]
        );
        assert!(
            !actions.menu[1].available,
            "no MusicBrainz lookup is possible"
        );
        let last = actions.menu.last().expect("menu has items");
        assert!(last.action.is_destructive());
        assert!(last.action.label().ends_with('…'));
    }
}
