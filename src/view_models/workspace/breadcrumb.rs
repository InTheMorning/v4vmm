//! Breadcrumb projections for workspace frame navigation.

#![warn(clippy::pedantic)]

use super::nav::{
    DiscoverView, FrameNavigationEntry, FrameNavigationState, MusicDestination, ReleaseKey,
    TrackKey,
};
use crate::view_models::artist_identity::ArtistIdentity;
use crate::view_models::singles_group::SinglesGroupKey;

/// Breadcrumb path truncation policy.
///
/// Breadcrumb renderers keep the origin and current segment visible when space
/// is constrained, collapsing the middle of longer paths.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BreadcrumbTruncation {
    /// Collapse middle segments when a renderer needs to abbreviate the path.
    MiddleEllipsis,
}

/// Display contract for one breadcrumb segment.
///
/// Current segments are labels only. Non-current segments carry the typed frame
/// destination so renderers can dispatch navigation without parsing strings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BreadcrumbSegment {
    /// Stable segment identifier.
    pub(crate) id: String,
    /// Visible segment label.
    pub(crate) label: String,
    /// Accessibility label for the segment.
    pub(crate) a11y_label: String,
    /// Whether this segment represents the current frame destination.
    pub(crate) is_current: bool,
    /// Target selected when activating a non-current segment.
    pub(crate) target: Option<FrameNavigationEntry>,
}

/// Display contract for a frame breadcrumb path.
///
/// The display is GPUI-free and projects from [`FrameNavigationState`]. Shells
/// or transitional surfaces can consume it without owning navigation policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BreadcrumbDisplay {
    /// Stable breadcrumb element identifier.
    pub(crate) id: String,
    /// Ordered breadcrumb segments from origin through current destination.
    pub(crate) segments: Vec<BreadcrumbSegment>,
    /// Renderer truncation guidance.
    pub(crate) truncation: BreadcrumbTruncation,
}

impl BreadcrumbDisplay {
    /// Projects a frame navigation state into display-ready breadcrumbs.
    #[must_use]
    pub(crate) fn project(
        id: impl Into<String>,
        nav: &FrameNavigationState,
        mut label_for: impl FnMut(&FrameNavigationEntry) -> String,
    ) -> Self {
        let entries = nav.path_entries();
        let segments = match entries.as_slice() {
            [current] => vec![breadcrumb_segment(current, true, &mut label_for)],
            [origin, current] => vec![
                breadcrumb_segment(origin, false, &mut label_for),
                breadcrumb_segment(current, true, &mut label_for),
            ],
            [origin, middle, current] => vec![
                breadcrumb_segment(origin, false, &mut label_for),
                breadcrumb_segment(middle, false, &mut label_for),
                breadcrumb_segment(current, true, &mut label_for),
            ],
            [origin, first, second, current] => vec![
                breadcrumb_segment(origin, false, &mut label_for),
                breadcrumb_segment(first, false, &mut label_for),
                breadcrumb_segment(second, false, &mut label_for),
                breadcrumb_segment(current, true, &mut label_for),
            ],
            [origin, .., parent, current] => vec![
                breadcrumb_segment(origin, false, &mut label_for),
                BreadcrumbSegment {
                    id: "breadcrumb-ellipsis".to_string(),
                    label: "…".to_string(),
                    a11y_label: "Collapsed breadcrumb segments".to_string(),
                    is_current: false,
                    target: None,
                },
                breadcrumb_segment(parent, false, &mut label_for),
                breadcrumb_segment(current, true, &mut label_for),
            ],
            [] => Vec::new(),
        };

        Self {
            id: id.into(),
            segments,
            truncation: BreadcrumbTruncation::MiddleEllipsis,
        }
    }
}

fn breadcrumb_segment(
    entry: &FrameNavigationEntry,
    is_current: bool,
    label_for: &mut impl FnMut(&FrameNavigationEntry) -> String,
) -> BreadcrumbSegment {
    let label = label_for(entry);
    BreadcrumbSegment {
        id: breadcrumb_entry_id(entry),
        a11y_label: if is_current {
            format!("Current location: {label}")
        } else {
            format!("Go to {label}")
        },
        label,
        is_current,
        target: (!is_current).then(|| entry.clone()),
    }
}

fn breadcrumb_entry_id(entry: &FrameNavigationEntry) -> String {
    match entry {
        FrameNavigationEntry::Destination(destination) => destination_id(*destination),
        FrameNavigationEntry::DiscoverView(view) => discover_view_id(*view),
        FrameNavigationEntry::SavedSearch { id, .. } => format!("saved-search-{}", id.0),
        FrameNavigationEntry::PlaylistDetail(id) => format!("playlist-{id}"),
        FrameNavigationEntry::Release(key) => release_key_id(key),
        FrameNavigationEntry::Track(key) => track_key_id(key),
        FrameNavigationEntry::ArtistName(key) => format!("artist-name-{}", slug_id(&key.key)),
        FrameNavigationEntry::PublisherDetail(publisher_feed_guid) => {
            format!("publisher-{}", slug_id(publisher_feed_guid))
        }
        FrameNavigationEntry::SinglesGroup(key) => singles_group_id(key),
        FrameNavigationEntry::Search(query) => format!("search-{}", slug_id(query)),
        FrameNavigationEntry::Settings => "settings".to_string(),
        FrameNavigationEntry::QueueNowPlaying => "queue-now-playing".to_string(),
    }
}

/// The breadcrumb element id of a destination entry (ADR 0084 Decision 2).
fn destination_id(destination: MusicDestination) -> String {
    let slug = match destination {
        MusicDestination::Discover => "discover",
        MusicDestination::Releases => "releases",
        MusicDestination::Artists => "artists",
        MusicDestination::Tracks => "tracks",
        MusicDestination::NeedsAttention => "needs-attention",
        MusicDestination::Playlists => "playlists",
    };
    format!("destination-{slug}")
}

/// The breadcrumb element id of a Discover view entry (ADR 0084 Decision
/// 3).
fn discover_view_id(view: DiscoverView) -> String {
    let slug = match view {
        DiscoverView::NewReleases => "new-releases",
    };
    format!("discover-view-{slug}")
}

/// The breadcrumb element id of a release entry (ADR 0084 Decision 4). The
/// id carries the key, so one release keeps one id whatever its origin.
fn release_key_id(key: &ReleaseKey) -> String {
    match key {
        ReleaseKey::FeedGuid(feed_guid) => format!("release-guid-{}", slug_id(feed_guid)),
        ReleaseKey::LibraryFeed(feed_id) => format!("release-feed-{feed_id}"),
    }
}

/// The breadcrumb element id of a track entry (ADR 0084 Decision 4). The
/// id carries the key, so one track keeps one id whatever its origin.
fn track_key_id(key: &TrackKey) -> String {
    let item = slug_id(&key.item_guid);
    match &key.release {
        Some(ReleaseKey::FeedGuid(feed_guid)) => {
            format!("track-guid-{}-{item}", slug_id(feed_guid))
        }
        Some(ReleaseKey::LibraryFeed(feed_id)) => format!("track-feed-{feed_id}-{item}"),
        None => format!("track-{item}"),
    }
}

/// The breadcrumb element id of a singles group entry (ADR 0084 Decision
/// 6). The id carries the identity level and its text.
fn singles_group_id(key: &SinglesGroupKey) -> String {
    match &key.0 {
        ArtistIdentity::Publisher(guid) => format!("singles-publisher-{}", slug_id(guid)),
        ArtistIdentity::Name(name) => format!("singles-name-{}", slug_id(name)),
    }
}

fn slug_id(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "root".to_string()
    } else {
        trimmed.to_string()
    }
}
