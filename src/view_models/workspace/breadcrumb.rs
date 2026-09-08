//! Breadcrumb projections for workspace frame navigation.

#![warn(clippy::pedantic)]

use super::nav::{FrameNavigationEntry, FrameNavigationState};

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
        FrameNavigationEntry::SourceList => "source-list".to_string(),
        FrameNavigationEntry::PlaylistDetail(id) => format!("playlist-{id}"),
        FrameNavigationEntry::TrackDetail(id) => format!("track-{id}"),
        FrameNavigationEntry::AlbumDetail(id) => format!("album-{id}"),
        FrameNavigationEntry::ArtistDetail(name) => format!("artist-{}", slug_id(name)),
        FrameNavigationEntry::Search(query) => format!("search-{}", slug_id(query)),
        FrameNavigationEntry::IndexArtistFeedScope(name) => {
            format!("index-artist-{}", slug_id(name))
        }
        FrameNavigationEntry::IndexFeedDetail { id, .. } => {
            format!("index-feed-{}", slug_id(id))
        }
        FrameNavigationEntry::IndexTrackDetail { id, .. } => {
            format!("index-track-{}", slug_id(id))
        }
        FrameNavigationEntry::Settings => "settings".to_string(),
        FrameNavigationEntry::QueueNowPlaying => "queue-now-playing".to_string(),
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
