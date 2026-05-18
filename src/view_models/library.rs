//! Library screen view-models.
//!
//! Pure projections of [`db::TrackRow`] + library-screen-owned state
//! ([`MbTrackStatus`]) into the strings the library inspector and album
//! detail rows render. Same layer rules as [`super`]: no GPUI imports,
//! no service mutation; constructed fresh each render.
//!
//! The album detail track row was the first call site to migrate, so
//! its projection ([`LibraryTrackRowVm`]) lives here. Future entries
//! (artist node summary, playlist row, `MusicBrainz` panel header) will
//! join as `library.rs` is whittled down.
//!
//! Per ADR 0023, this layer must not import screen modules. The
//! per-track `MusicBrainz` lookup state therefore lives here as
//! [`MbTrackStatus`]; the library screen depends on the view-model for
//! the type, not the other way around.

#![warn(clippy::pedantic)]

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt::Write as _;

#[cfg(test)]
use crate::application::library_removal::LibraryRemovalImpact;
use crate::application::library_removal::{LibraryRemovalPlan, LibraryRemovalTarget};
use crate::db::{self, TrackRow};
use crate::feed_service;
use crate::metadata::MusicBrainzLookupResult;
use crate::view_models::artist_detail::{ArtistDetailFactVm, ArtistDetailPageVm};
use crate::view_models::entity_detail::{
    EntityActionTarget, EntityActionVm, PlaylistActionState, ReleaseActionState,
    ReleaseMembershipState, TrackActionState, TrackMembershipState,
};
use crate::view_models::format::{fmt_date, fmt_total_runtime_clock, plural};
use crate::view_models::library_removal::{
    LibraryRemovalConfirmationDisplay, LibraryRemovalConfirmationState,
};
use crate::view_models::playlist_detail::PlaylistDetailPageVm;
use crate::view_models::text_filter::{contains_normalized, normalize};
use crate::view_models::workspace::{ContentFilter, FilterChipStripDisplay};
use crate::view_models::{ActionStatusMessageDisplay, SplitPaneState};
use crate::views::{ArtistView, FeedRef, FeedView, LocalIdentityFacts, TrackRef};

const DEFAULT_SPLIT_PANE_WIDTH: f32 = 360.0;

/// Rendered-line threshold before descriptions start collapsed.
pub(crate) const DESCRIPTION_AUTO_COLLAPSE_LINES: usize = 5;

/// Tooltip shown when a track must be downloaded before metadata actions.
pub(crate) const DOWNLOAD_REQUIRED_METADATA_TOOLTIP: &str = "Download track to enable";

/// Explicit disclosure state for long feed and track descriptions.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum DescriptionState {
    /// Description exceeds [`DESCRIPTION_AUTO_COLLAPSE_LINES`] before user input.
    AutoCollapsed,
    /// Description fits within [`DESCRIPTION_AUTO_COLLAPSE_LINES`] before user input.
    #[default]
    AutoExpanded,
    /// User explicitly collapsed the description.
    UserCollapsed,
    /// User explicitly expanded the description.
    UserExpanded,
}

impl DescriptionState {
    /// Project automatic disclosure from an estimated rendered line count.
    #[must_use]
    pub(crate) const fn project(line_count: usize) -> Self {
        if line_count > DESCRIPTION_AUTO_COLLAPSE_LINES {
            Self::AutoCollapsed
        } else {
            Self::AutoExpanded
        }
    }

    /// Re-project automatic states while preserving explicit user choices.
    #[must_use]
    pub(crate) const fn project_sticky(self, line_count: usize) -> Self {
        match self {
            Self::AutoCollapsed | Self::AutoExpanded => Self::project(line_count),
            Self::UserCollapsed | Self::UserExpanded => self,
        }
    }

    /// Toggle disclosure, converting automatic state to sticky user state.
    #[must_use]
    pub(crate) const fn toggle(self) -> Self {
        match self {
            Self::AutoCollapsed | Self::UserCollapsed => Self::UserExpanded,
            Self::AutoExpanded | Self::UserExpanded => Self::UserCollapsed,
        }
    }

    /// Whether the full description body should be visible.
    #[must_use]
    pub(crate) const fn is_visible(self) -> bool {
        matches!(self, Self::AutoExpanded | Self::UserExpanded)
    }
}

/// Estimate rendered description lines before layout measurement exists.
#[must_use]
pub(crate) fn description_line_count(description: Option<&str>) -> usize {
    let Some(description) = display_description_text(description) else {
        return 0;
    };
    description.lines().count().max(1)
}

/// Return description text that is meaningful enough to display.
#[must_use]
pub(crate) fn display_description_text(description: Option<&str>) -> Option<&str> {
    description.map(str::trim).filter(|value| !value.is_empty())
}

/// Library track inspector panels that require explicit disclosure.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum InspectorPanelKind {
    /// Embedded-tag comparison panel.
    CompareId3,
    /// `MusicBrainz` lookup result panel.
    MusicBrainz,
}

/// Whether Compare ID3 controls are available for the track.
#[must_use]
pub(crate) const fn compare_id3_enabled(is_downloaded: bool) -> bool {
    is_downloaded
}

/// Whether `MusicBrainz` controls are available for the track.
#[must_use]
pub(crate) const fn musicbrainz_enabled(is_downloaded: bool) -> bool {
    is_downloaded
}

/// Stateful, GPUI-free track-inspector disclosure contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LibraryTrackInspectorState {
    pub(crate) inspector_expanded_panels: BTreeSet<InspectorPanelKind>,
    pub(crate) description_state: DescriptionState,
}

impl LibraryTrackInspectorState {
    /// Construct inspector state from an estimated description line count.
    #[must_use]
    pub(crate) fn new(description_line_count: usize) -> Self {
        Self {
            inspector_expanded_panels: BTreeSet::new(),
            description_state: DescriptionState::project(description_line_count),
        }
    }

    /// Mark an advanced panel as expanded.
    pub(crate) fn expand_panel(&mut self, kind: InspectorPanelKind) {
        self.inspector_expanded_panels.insert(kind);
    }

    /// Mark an advanced panel as collapsed.
    pub(crate) fn collapse_panel(&mut self, kind: InspectorPanelKind) {
        self.inspector_expanded_panels.remove(&kind);
    }

    /// Toggle an advanced panel.
    pub(crate) fn toggle_panel(&mut self, kind: InspectorPanelKind) {
        if self.is_panel_expanded(kind) {
            self.collapse_panel(kind);
        } else {
            self.expand_panel(kind);
        }
    }

    /// Return whether an advanced panel is expanded.
    #[must_use]
    pub(crate) fn is_panel_expanded(&self, kind: InspectorPanelKind) -> bool {
        self.inspector_expanded_panels.contains(&kind)
    }

    /// Toggle the track description disclosure state.
    pub(crate) fn toggle_description(&mut self) {
        self.description_state = self.description_state.toggle();
    }

    /// Re-project auto description state while preserving user choices.
    pub(crate) fn project_description(&mut self, line_count: usize) {
        self.description_state = self.description_state.project_sticky(line_count);
    }

    /// Display projection consumed by later inspector rendering tasks.
    #[must_use]
    pub(crate) fn display(&self, is_downloaded: bool) -> LibraryTrackInspectorDisplay {
        LibraryTrackInspectorDisplay {
            inspector_expanded_panels: self.inspector_expanded_panels.clone(),
            description_state: self.description_state,
            compare_id3_enabled: compare_id3_enabled(is_downloaded),
            musicbrainz_enabled: musicbrainz_enabled(is_downloaded),
        }
    }
}

impl Default for LibraryTrackInspectorState {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Display contract for track-inspector disclosure and panel availability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LibraryTrackInspectorDisplay {
    pub(crate) inspector_expanded_panels: BTreeSet<InspectorPanelKind>,
    pub(crate) description_state: DescriptionState,
    pub(crate) compare_id3_enabled: bool,
    pub(crate) musicbrainz_enabled: bool,
}

impl LibraryTrackInspectorDisplay {
    /// Returns whether the Compare ID3 panel should be visible.
    #[must_use]
    pub(crate) fn show_compare_id3_panel(&self) -> bool {
        self.compare_id3_enabled
            && self
                .inspector_expanded_panels
                .contains(&InspectorPanelKind::CompareId3)
    }

    /// Returns whether the `MusicBrainz` panel should be visible.
    #[must_use]
    pub(crate) fn show_musicbrainz_panel(&self) -> bool {
        self.musicbrainz_enabled
            && self
                .inspector_expanded_panels
                .contains(&InspectorPanelKind::MusicBrainz)
    }

    /// Tooltip for disabled Compare ID3 controls.
    #[must_use]
    pub(crate) const fn compare_id3_tooltip_text(&self) -> Option<&'static str> {
        if self.compare_id3_enabled {
            None
        } else {
            Some(DOWNLOAD_REQUIRED_METADATA_TOOLTIP)
        }
    }

    /// Tooltip for disabled `MusicBrainz` controls.
    #[must_use]
    pub(crate) const fn musicbrainz_tooltip_text(&self) -> Option<&'static str> {
        if self.musicbrainz_enabled {
            None
        } else {
            Some(DOWNLOAD_REQUIRED_METADATA_TOOLTIP)
        }
    }
}

/// Source-list entry for an in-memory saved search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SavedSearchEntry {
    pub(crate) id: i64,
    pub(crate) query: String,
    pub(crate) label: String,
    pub(crate) a11y_label: String,
}

/// Optional display section for saved searches beneath playlists.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SavedSearchesSectionDisplay {
    pub(crate) heading: &'static str,
    pub(crate) rows: Vec<SavedSearchEntry>,
}

/// Per-track `MusicBrainz` lookup state owned by the library screen and
/// projected into display by [`LibraryTrackRowVm`].
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) enum MbTrackStatus {
    Pending,
    Processing,
    Done(usize),
    Skipped(String),
}

/// Display-ready projection of a [`TrackRow`] in the library album
/// detail listing.
///
/// Borrow-only — constructed fresh each render and dropped before the
/// element tree is painted.
pub(crate) struct LibraryTrackRowVm<'a> {
    track: &'a TrackRow,
    mb: Option<&'a MbTrackStatus>,
}

/// Semantic colour bucket for the `MusicBrainz` status hint. The screen
/// maps each variant to a token at render time, keeping the VM free of
/// GPUI types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MbStatusKind {
    Success,
    Warning,
    Danger,
    Muted,
}

/// One artist node in the library sidebar tree. Owns a flat list of
/// album nodes; the screen handles expansion and rendering.
#[derive(Clone, Debug)]
pub(crate) struct ArtistNode {
    pub(crate) name: String,
    pub(crate) albums: Vec<AlbumNode>,
}

/// One album node — a feed in podcast terms — under an artist. Holds
/// the embedded track rows so the album-detail panel can render
/// without re-querying the DB.
#[derive(Clone, Debug)]
pub(crate) struct AlbumNode {
    pub(crate) name: String,
    pub(crate) feed_id: Option<i64>,
    pub(crate) feed_guid: Option<String>,
    pub(crate) feed_url: Option<String>,
    pub(crate) language: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) image_href: Option<String>,
    pub(crate) identity_facts: LocalIdentityFacts,
    pub(crate) tracks: Vec<TrackRow>,
}

/// Top-level structure of the library sidebar — a list of artist
/// nodes, each with their albums.
#[derive(Clone, Debug, Default)]
pub(crate) struct LibraryTree {
    pub(crate) artists: Vec<ArtistNode>,
}

impl ArtistNode {
    #[must_use]
    pub(crate) fn tree_display(&self, expanded: bool) -> LibraryArtistTreeDisplay {
        LibraryArtistTreeDisplay {
            element_id: format!("artist-{}", self.name),
            title: self.name.clone(),
            disclosure_glyph: disclosure_glyph(expanded),
            album_count_label: format!(
                "({} album{})",
                self.albums.len(),
                plural(self.albums.len())
            ),
        }
    }
}

impl AlbumNode {
    #[must_use]
    pub(crate) fn tree_display(
        &self,
        artist_name: &str,
        expanded: bool,
    ) -> LibraryAlbumTreeDisplay {
        LibraryAlbumTreeDisplay {
            element_id: format!("album-{artist_name}-{}", self.name),
            title: self.name.clone(),
            disclosure_glyph: disclosure_glyph(expanded),
            track_count_label: format!("({})", self.tracks.len()),
        }
    }
}

#[must_use]
const fn disclosure_glyph(expanded: bool) -> &'static str {
    if expanded {
        "\u{25BC}"
    } else {
        "\u{25B6}"
    }
}

/// Pure data snapshots loaded by the library screen.
///
/// This groups DB-derived read models and in-flight metadata/feed state
/// behind the view-model boundary. It intentionally carries no GPUI
/// framework values; image caches and subscriptions stay in `LibraryApp`.
#[derive(Clone, Debug, Default)]
pub(crate) struct LibrarySnapshot {
    tree: LibraryTree,
    playlists: Vec<db::Playlist>,
    playlist_tracks: Vec<TrackRow>,
    mb_status: BTreeMap<i64, MbTrackStatus>,
    staged_musicbrainz: BTreeMap<i64, MusicBrainzLookupResult>,
    in_flight_feed_checks: HashSet<i64>,
    feed_update_state: FeedUpdateState,
}

/// Projected library tree plus the expansion state needed to render it.
#[derive(Clone, Debug)]
pub(crate) struct LibraryTreeProjection {
    pub(crate) tree: LibraryTree,
    pub(crate) expanded_artists: HashSet<String>,
    pub(crate) expanded_albums: HashSet<(String, String)>,
}

impl LibraryTreeProjection {
    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.tree.artists.is_empty()
    }
}

/// Display contract for an artist row in the Library sidebar tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LibraryArtistTreeDisplay {
    pub(crate) element_id: String,
    pub(crate) title: String,
    pub(crate) disclosure_glyph: &'static str,
    pub(crate) album_count_label: String,
}

/// Display contract for an album row in the Library sidebar tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LibraryAlbumTreeDisplay {
    pub(crate) element_id: String,
    pub(crate) title: String,
    pub(crate) disclosure_glyph: &'static str,
    pub(crate) track_count_label: String,
}

/// Display contract for a track row in the Library sidebar tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LibraryTreeTrackDisplay {
    pub(crate) element_id: String,
    pub(crate) title: String,
}

/// Display-ready projection for the playlist sidebar section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlaylistSidebarVm {
    pub(crate) header_id: &'static str,
    pub(crate) sort_button_id: &'static str,
    pub(crate) add_button_id: &'static str,
    pub(crate) new_playlist_input_id: &'static str,
    pub(crate) new_playlist_add_button_id: &'static str,
    pub(crate) expanded: bool,
    pub(crate) disclosure_glyph: &'static str,
    pub(crate) heading: &'static str,
    pub(crate) sort_label: &'static str,
    pub(crate) add_label: &'static str,
    pub(crate) new_playlist_add_label: &'static str,
    pub(crate) creating_playlist: bool,
    pub(crate) rows: Vec<PlaylistSidebarRowVm>,
}

/// One playlist row in the library sidebar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlaylistSidebarRowVm {
    pub(crate) id: i64,
    pub(crate) element_id: String,
    pub(crate) name: String,
    pub(crate) track_count_label: String,
    pub(crate) selected: bool,
}

/// Static labels for the Library shell chrome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LibraryChromeDisplay {
    pub(crate) split_pane_id: &'static str,
    pub(crate) resize_handle_id: &'static str,
    pub(crate) search_button_id: &'static str,
    pub(crate) list_scroll_id: &'static str,
    pub(crate) artist_detail_scroll_id: &'static str,
    pub(crate) playlist_detail_scroll_id: &'static str,
    pub(crate) track_detail_scroll_id: &'static str,
    pub(crate) search_placeholder: &'static str,
    pub(crate) new_playlist_placeholder: &'static str,
    pub(crate) search_heading: &'static str,
    pub(crate) search_button_label: &'static str,
    pub(crate) empty_library_label: &'static str,
    pub(crate) empty_detail_label: &'static str,
}

impl LibraryChromeDisplay {
    const VALUE: Self = Self {
        split_pane_id: "library-pane-container",
        resize_handle_id: "library-resize-handle",
        search_button_id: "lib-search-btn",
        list_scroll_id: "library-list",
        artist_detail_scroll_id: "artist-detail-scroll",
        playlist_detail_scroll_id: "playlist-detail-scroll",
        track_detail_scroll_id: "track-detail-scroll",
        search_placeholder: "Search your library...",
        new_playlist_placeholder: "New playlist name\u{2026}",
        search_heading: "Search Library",
        search_button_label: "Search",
        empty_library_label: "No library tracks yet",
        empty_detail_label: "Select an item to view details",
    };
}

/// Status text plus severity for the Library shell.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LibraryStatusSnapshot {
    pub(crate) text: String,
    pub(crate) is_error: bool,
}

impl LibraryStatusSnapshot {
    #[must_use]
    fn from_text(text: &str) -> Self {
        Self {
            text: text.to_string(),
            is_error: text.starts_with("Error:"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AlbumThumbDisplay {
    pub(crate) fallback_icon: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HoverThumbDisplay {
    pub(crate) element_id: String,
}

/// Action kind for the feed-update toolbar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FeedUpdateActionKind {
    ApplyUpdates,
    CheckAllFeeds,
}

/// Display contract for the feed-update toolbar action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FeedUpdateActionDisplay {
    pub(crate) kind: FeedUpdateActionKind,
    pub(crate) button_id: &'static str,
    pub(crate) label: String,
    pub(crate) disabled: bool,
}

/// Display contract for the feed-update toolbar row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FeedUpdateDisplay {
    pub(crate) status_message: Option<String>,
    pub(crate) action: FeedUpdateActionDisplay,
}

/// Pure command intent for appending one or more library tracks to a playlist.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlaylistAppendIntent {
    playlist_id: i64,
    track_ids: Vec<i64>,
    playlist_name: String,
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

/// Pure result data for a completed library track subscription.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TrackSubscribeOutcome {
    path_label: String,
    format_warning: Option<String>,
}

impl TrackSubscribeOutcome {
    #[must_use]
    pub(crate) fn new(path_label: impl Into<String>, format_warning: Option<String>) -> Self {
        Self {
            path_label: path_label.into(),
            format_warning,
        }
    }
}

/// Display/status projection for the library track action row.
///
/// The screen owns click handlers and panel rendering; this VM owns the
/// button labels and subscription-message classification.
pub(crate) struct LibraryTrackActionVm<'a> {
    subscription_busy: bool,
    local_subscription: bool,
    subscription_message: Option<&'a str>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LibraryTrackPlaylistDisplay {
    pub(crate) popover_id: String,
    pub(crate) trigger_label: &'static str,
}

/// Display contract for a track row inside an album detail list.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LibraryTrackRowDisplay {
    pub(crate) row_id: String,
    pub(crate) toggle_button_id: String,
}

/// Source bucket for a row in the future workspace content-list frame.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ContentListRowSource {
    /// Row belongs to the local library.
    Library,
    /// Row is available from the remote index but is not local.
    Index,
}

#[allow(dead_code)]
impl ContentListRowSource {
    /// Returns whether this source is visible under the content filter.
    #[must_use]
    pub(crate) const fn matches_filter(self, filter: ContentFilter) -> bool {
        match filter {
            ContentFilter::All => true,
            ContentFilter::Library => matches!(self, Self::Library),
            ContentFilter::Index => matches!(self, Self::Index),
        }
    }
}

/// Display row cached by the GPUI-free content-list page VM.
#[allow(dead_code)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ContentListRowDisplay {
    /// Stable row identifier.
    pub(crate) id: String,
    /// Primary visible row title.
    pub(crate) title: String,
    /// Secondary visible row text.
    pub(crate) secondary_text: String,
    /// Local-vs-index provenance used by per-frame filtering.
    pub(crate) source: ContentListRowSource,
}

#[allow(dead_code)]
impl ContentListRowDisplay {
    /// Creates a content-list row display.
    #[must_use]
    pub(crate) fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        secondary_text: impl Into<String>,
        source: ContentListRowSource,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            secondary_text: secondary_text.into(),
            source,
        }
    }

    /// Projects a database track row into content-list display data.
    #[must_use]
    pub(crate) fn from_track(track: &TrackRow) -> Self {
        Self::new(
            track.id.to_string(),
            track
                .track_title
                .clone()
                .or_else(|| track.feed_title.clone())
                .unwrap_or_else(|| "Untitled".to_string()),
            track
                .album_artist_name
                .clone()
                .or_else(|| track.artist_name.clone())
                .or_else(|| track.album_title.clone())
                .unwrap_or_else(|| "Unknown Artist".to_string()),
            if track.is_in_library {
                ContentListRowSource::Library
            } else {
                ContentListRowSource::Index
            },
        )
    }

    #[must_use]
    fn matches_text_filter(&self, filter: &str) -> bool {
        filter.is_empty()
            || contains_normalized(&self.title, filter)
            || contains_normalized(&self.secondary_text, filter)
    }
}

/// Empty-state display for a filtered content-list frame.
#[allow(dead_code)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ContentListEmptyStateDisplay {
    /// Primary empty-state title.
    pub(crate) title: &'static str,
    /// Secondary empty-state explanation.
    pub(crate) secondary: &'static str,
    /// Optional command id that clears the active filter.
    pub(crate) clear_filter_action_id: Option<&'static str>,
}

#[allow(dead_code)]
impl ContentListEmptyStateDisplay {
    #[must_use]
    const fn for_filter(filter: ContentFilter) -> Self {
        match filter {
            ContentFilter::All => Self {
                title: "No content",
                secondary: "No rows are available for this frame.",
                clear_filter_action_id: None,
            },
            ContentFilter::Library => Self {
                title: "No library content",
                secondary: "No local rows match this frame filter.",
                clear_filter_action_id: Some("content-list.clear-filter"),
            },
            ContentFilter::Index => Self {
                title: "No index content",
                secondary: "No index rows match this frame filter.",
                clear_filter_action_id: Some("content-list.clear-filter"),
            },
        }
    }
}

/// GPUI-free page VM for a workspace content-list frame.
#[allow(dead_code)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ContentListPageVm {
    filter_state: ContentFilter,
    text_filter: Option<String>,
    cached_rows: Vec<ContentListRowDisplay>,
}

#[allow(dead_code)]
impl ContentListPageVm {
    /// Creates a content-list page VM with cached rows and the default filter.
    #[must_use]
    pub(crate) fn new(cached_rows: Vec<ContentListRowDisplay>) -> Self {
        Self {
            filter_state: ContentFilter::default(),
            text_filter: None,
            cached_rows,
        }
    }

    /// Projects database track rows into a content-list page VM.
    #[must_use]
    pub(crate) fn from_tracks(tracks: &[TrackRow]) -> Self {
        Self::new(
            tracks
                .iter()
                .map(ContentListRowDisplay::from_track)
                .collect(),
        )
    }

    /// Returns the selected content filter.
    #[must_use]
    pub(crate) const fn filter(&self) -> ContentFilter {
        self.filter_state
    }

    /// Sets the selected content filter for this frame-local page.
    pub(crate) fn set_filter(&mut self, filter: ContentFilter) {
        self.filter_state = filter;
    }

    /// Returns the active text filter, when set.
    #[must_use]
    pub(crate) fn text_filter(&self) -> Option<&str> {
        self.text_filter.as_deref()
    }

    /// Sets or clears the frame-local text filter.
    pub(crate) fn set_text_filter(&mut self, filter: Option<String>) {
        self.text_filter = normalize(filter);
    }

    /// Replaces the cached rows while preserving the frame-local filter.
    pub(crate) fn replace_rows(&mut self, cached_rows: Vec<ContentListRowDisplay>) {
        self.cached_rows = cached_rows;
    }

    /// Returns every cached row before filtering.
    #[must_use]
    pub(crate) fn cached_rows(&self) -> &[ContentListRowDisplay] {
        &self.cached_rows
    }

    /// Returns rows visible under the current frame-local filter.
    #[must_use]
    pub(crate) fn visible_rows(&self) -> Vec<&ContentListRowDisplay> {
        self.cached_rows
            .iter()
            .filter(|row| row.source.matches_filter(self.filter_state))
            .filter(|row| {
                self.text_filter
                    .as_deref()
                    .is_none_or(|filter| row.matches_text_filter(filter))
            })
            .collect()
    }

    /// Returns stable row identifiers visible under the current filter.
    #[must_use]
    pub(crate) fn visible_row_ids(&self) -> Vec<&str> {
        self.visible_rows()
            .into_iter()
            .map(|row| row.id.as_str())
            .collect()
    }

    /// Returns the active empty state when the current filter hides all rows.
    #[must_use]
    pub(crate) fn empty_state(&self) -> Option<ContentListEmptyStateDisplay> {
        self.visible_rows()
            .is_empty()
            .then(|| ContentListEmptyStateDisplay::for_filter(self.filter_state))
    }

    /// Returns the frame-local filter chip display for this content list.
    #[must_use]
    pub(crate) fn filter_chip_strip(&self) -> FilterChipStripDisplay {
        FilterChipStripDisplay::default_for_content_list(self.filter_state, true)
    }
}

impl<'a> LibraryTrackActionVm<'a> {
    #[must_use]
    pub(crate) fn new(
        subscription_busy: bool,
        local_subscription: bool,
        subscription_message: Option<&'a str>,
    ) -> Self {
        Self {
            subscription_busy,
            local_subscription,
            subscription_message,
        }
    }

    #[must_use]
    pub(crate) fn subscription_button_label(&self) -> &'static str {
        match (self.subscription_busy, self.local_subscription) {
            (true, true) => "Removing...",
            (true, false) => "Downloading...",
            (false, true) => "Remove Track",
            (false, false) => "Download Track",
        }
    }

    #[must_use]
    pub(crate) const fn subscription_busy_message(subscribe: bool) -> &'static str {
        if subscribe {
            "Downloading..."
        } else {
            "Removing..."
        }
    }

    #[must_use]
    pub(crate) fn subscription_error_message(
        subscribe: bool,
        error: impl std::fmt::Display,
    ) -> String {
        let action = if subscribe { "Download" } else { "Remove" };
        format!("{action} error: {error:#}")
    }

    #[must_use]
    pub(crate) const fn track_subscribe_begin_status() -> &'static str {
        "Downloading track..."
    }

    #[must_use]
    pub(crate) const fn track_subscribe_success_message() -> &'static str {
        "Downloaded track"
    }

    #[must_use]
    pub(crate) const fn add_to_playlist_label() -> &'static str {
        "Add to playlist"
    }

    #[must_use]
    pub(crate) fn playlist_display(track_id: i64) -> LibraryTrackPlaylistDisplay {
        LibraryTrackPlaylistDisplay {
            popover_id: format!("track-inspector-add:{track_id}"),
            trigger_label: Self::add_to_playlist_label(),
        }
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
        "Track actions"
    }
}

impl PlaylistAppendIntent {
    #[must_use]
    pub(crate) fn playlist_id(&self) -> i64 {
        self.playlist_id
    }

    #[must_use]
    pub(crate) fn playlist_name(&self) -> &str {
        &self.playlist_name
    }

    #[must_use]
    pub(crate) fn total_tracks(&self) -> usize {
        self.track_ids.len()
    }

    #[must_use]
    pub(crate) fn track_ids(&self) -> &[i64] {
        &self.track_ids
    }
}

/// Snapshot of the multi-feed update workflow exposed by
/// `feed_service`. Owned by the library view-model; the screen reads
/// `phase` to decide whether to show progress vs. results.
#[derive(Clone, Debug, Default)]
pub struct FeedUpdateState {
    pub phase: FeedUpdatePhase,
    pub status_message: Option<String>,
    pub stale: Vec<feed_service::StaleFeed>,
}

/// Lifecycle of a feed-update workflow.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum FeedUpdatePhase {
    #[default]
    Idle,
    Checking,
    Applying,
}

/// Sort order for the library's playlist sidebar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum PlaylistSort {
    /// Alphabetical by playlist name.
    #[default]
    Name,
    /// Most-recently-updated playlists first.
    RecentlyUpdated,
    /// Largest playlists first by track count.
    TrackCount,
}

impl PlaylistSort {
    /// Cycle to the next sort order. Wraps `TrackCount` back to `Name`.
    #[must_use]
    pub(crate) fn next(self) -> Self {
        match self {
            PlaylistSort::Name => PlaylistSort::RecentlyUpdated,
            PlaylistSort::RecentlyUpdated => PlaylistSort::TrackCount,
            PlaylistSort::TrackCount => PlaylistSort::Name,
        }
    }

    /// Short button label for the sort cycler.
    #[must_use]
    pub(crate) fn label(self) -> &'static str {
        match self {
            PlaylistSort::Name => "A–Z",
            PlaylistSort::RecentlyUpdated => "Recent",
            PlaylistSort::TrackCount => "Size",
        }
    }
}

/// Stateful screen view-model for the library tab.
///
/// `LibraryViewModel` is the [SwiftUI `@Observable`-style](https://developer.apple.com/documentation/observation)
/// adapter between the GPUI `LibraryApp` and the rest of the layered
/// architecture: it owns the screen's *pure* UI state (selection,
/// expansion sets, sort orders, picker toggles) so the screen file
/// shrinks to event wiring and `Render` glue.
///
/// Per ADR 0023 this struct must remain GPUI-free. Anything that
/// requires `gpui::Image`, `gpui::Entity`, or other framework types
/// stays in `LibraryApp` for now.
///
/// Fields move from `LibraryApp` into this VM in phases as the legacy
/// renderer migrates to projection VMs.
#[derive(Clone, Debug)]
pub(crate) struct LibraryViewModel {
    // Loaded snapshots — owned here so the screen can become a thin
    // Render impl. None of these carry GPUI types.
    snapshot: LibrarySnapshot,
    // Sidebar expansion + sort.
    expanded_artists: HashSet<String>,
    expanded_albums: HashSet<(String, String)>,
    playlists_expanded: bool,
    playlist_sort: PlaylistSort,
    saved_searches: Vec<SavedSearchEntry>,
    content_list_page: ContentListPageVm,
    album_description_states: BTreeMap<i64, DescriptionState>,
    track_description_states: BTreeMap<i64, DescriptionState>,
    // Selection / focus.
    selected_id: Option<i64>,
    selected_playlist_id: Option<i64>,
    hovered_thumb_url: Option<String>,
    // Operation state.
    busy_track: Option<i64>,
    busy_feed: Option<i64>,
    library_removal: LibraryRemovalConfirmationState,
    status: String,
    library_loading: bool,
    // Layout / drag state.
    split_pane: SplitPaneState,
    // Search + playlist creation.
    search_query: String,
    creating_playlist: bool,
    renaming_playlist_id: Option<i64>,
    // Phase 3: detail frame text filter state.
    detail_text_filter: Option<String>,
}

impl LibraryViewModel {
    /// Construct a view-model with the legacy `LibraryApp::new`
    /// defaults: empty tree + snapshots, empty expansion sets,
    /// playlists sidebar already expanded, sort by name, no selection,
    /// no operation in flight.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            snapshot: LibrarySnapshot::default(),
            expanded_artists: HashSet::new(),
            expanded_albums: HashSet::new(),
            playlists_expanded: true,
            playlist_sort: PlaylistSort::default(),
            saved_searches: Vec::new(),
            content_list_page: ContentListPageVm::new(Vec::new()),
            album_description_states: BTreeMap::new(),
            track_description_states: BTreeMap::new(),
            selected_id: None,
            selected_playlist_id: None,
            hovered_thumb_url: None,
            busy_track: None,
            busy_feed: None,
            library_removal: LibraryRemovalConfirmationState::new(),
            status: String::new(),
            library_loading: false,
            split_pane: SplitPaneState::new(DEFAULT_SPLIT_PANE_WIDTH),
            search_query: String::new(),
            creating_playlist: false,
            renaming_playlist_id: None,
            detail_text_filter: None,
        }
    }

    #[must_use]
    pub(crate) fn tree(&self) -> &LibraryTree {
        &self.snapshot.tree
    }

    pub(crate) fn replace_tree(&mut self, tree: LibraryTree) {
        self.content_list_page
            .replace_rows(content_list_rows_from_tree(&tree));
        self.snapshot.tree = tree;
    }

    pub(crate) fn update_album_identity_facts(
        &mut self,
        feed_id: i64,
        facts: &LocalIdentityFacts,
    ) -> bool {
        let mut updated = false;
        for album in self
            .snapshot
            .tree
            .artists
            .iter_mut()
            .flat_map(|artist| artist.albums.iter_mut())
            .filter(|album| album.feed_id == Some(feed_id))
        {
            album.identity_facts = facts.clone();
            updated = true;
        }
        updated
    }

    pub(crate) fn update_album_description(
        &mut self,
        feed_id: i64,
        description: Option<&str>,
    ) -> bool {
        let mut updated = false;
        let description = description.map(str::to_owned);
        for album in self
            .snapshot
            .tree
            .artists
            .iter_mut()
            .flat_map(|artist| artist.albums.iter_mut())
            .filter(|album| album.feed_id == Some(feed_id))
        {
            album.description.clone_from(&description);
            updated = true;
        }
        updated
    }

    pub(crate) fn finish_library_reload(&mut self, track_count: usize) {
        self.library_loading = false;
        self.status = format!("{track_count} library track{}", plural(track_count));
    }

    /// Mark the library tree as actively reloading.
    ///
    /// Render code consults [`Self::is_library_loading`] to decide
    /// whether to paint skeleton placeholders while the background
    /// reload is in flight. The status string is updated so the
    /// existing chrome reflects the in-progress state.
    pub(crate) fn begin_library_reload(&mut self) {
        self.library_loading = true;
        self.library_removal.cancel();
        self.status = "Loading library\u{2026}".to_string();
    }

    #[must_use]
    pub(crate) fn is_library_loading(&self) -> bool {
        self.library_loading
    }

    #[must_use]
    pub(crate) fn tree_projection(&self) -> LibraryTreeProjection {
        let query = self.search_query.trim();
        let tree = if query.is_empty() {
            self.snapshot.tree.clone()
        } else {
            filter_tree(&self.snapshot.tree, query)
        };

        if query.is_empty() {
            return LibraryTreeProjection {
                tree,
                expanded_artists: self.expanded_artists.clone(),
                expanded_albums: self.expanded_albums.clone(),
            };
        }

        let expanded_artists = tree
            .artists
            .iter()
            .map(|artist| artist.name.clone())
            .collect();
        let expanded_albums = tree
            .artists
            .iter()
            .flat_map(|artist| {
                artist
                    .albums
                    .iter()
                    .map(move |album| (artist.name.clone(), album.name.clone()))
            })
            .collect();

        LibraryTreeProjection {
            tree,
            expanded_artists,
            expanded_albums,
        }
    }

    #[must_use]
    pub(crate) const fn content_filter(&self) -> ContentFilter {
        self.content_list_page.filter()
    }

    pub(crate) fn set_content_filter(&mut self, filter: ContentFilter) {
        self.content_list_page.set_filter(filter);
    }

    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "tested in library_view_model_content_text_filter_uses_content_list_page_vm"
        )
    )]
    pub(crate) fn content_text_filter(&self) -> Option<&str> {
        self.content_list_page.text_filter()
    }

    #[allow(dead_code)]
    pub(crate) fn set_content_text_filter(&mut self, filter: Option<String>) {
        self.content_list_page.set_text_filter(filter);
    }

    #[allow(dead_code)]
    pub(crate) fn set_detail_text_filter(&mut self, filter: Option<String>) {
        self.detail_text_filter = filter;
    }

    #[allow(dead_code, reason = "part of public API for testing and diagnostic")]
    pub(crate) fn detail_text_filter(&self) -> Option<&String> {
        self.detail_text_filter.as_ref()
    }

    #[must_use]
    pub(crate) fn content_filter_chip_strip(&self) -> FilterChipStripDisplay {
        self.content_list_page.filter_chip_strip()
    }

    #[must_use]
    pub(crate) fn content_filter_empty_state(&self) -> Option<ContentListEmptyStateDisplay> {
        (self.content_filter() != ContentFilter::All)
            .then(|| self.content_list_page.empty_state())
            .flatten()
    }

    #[must_use]
    pub(crate) fn playlists(&self) -> &[db::Playlist] {
        &self.snapshot.playlists
    }

    pub(crate) fn replace_playlists(&mut self, mut playlists: Vec<db::Playlist>) {
        Self::sort_playlists_by(self.playlist_sort, &mut playlists);
        self.snapshot.playlists = playlists;
    }

    pub(crate) fn fail_playlist_load(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Error loading playlists: {error:#}");
    }

    pub(crate) fn sort_loaded_playlists(&mut self) {
        Self::sort_playlists_by(self.playlist_sort, &mut self.snapshot.playlists);
    }

    #[must_use]
    pub(crate) fn playlist_by_id(&self, playlist_id: i64) -> Option<db::Playlist> {
        self.snapshot
            .playlists
            .iter()
            .find(|playlist| playlist.id == playlist_id)
            .cloned()
    }

    #[must_use]
    pub(crate) fn playlist_sidebar(&self) -> PlaylistSidebarVm {
        PlaylistSidebarVm {
            header_id: "playlists-header",
            sort_button_id: "playlists-sort",
            add_button_id: "playlists-add",
            new_playlist_input_id: "playlist-new-input",
            new_playlist_add_button_id: "playlist-add-btn",
            expanded: self.playlists_expanded,
            disclosure_glyph: if self.playlists_expanded {
                "\u{25BC}"
            } else {
                "\u{25B6}"
            },
            heading: "Playlists",
            sort_label: self.playlist_sort_label(),
            add_label: "+",
            new_playlist_add_label: "Add",
            creating_playlist: self.creating_playlist,
            rows: self
                .snapshot
                .playlists
                .iter()
                .map(|playlist| PlaylistSidebarRowVm {
                    id: playlist.id,
                    element_id: format!("playlist-{}", playlist.id),
                    name: playlist.name.clone(),
                    track_count_label: format!("({})", playlist.track_count),
                    selected: self.selected_playlist_id == Some(playlist.id),
                })
                .collect(),
        }
    }

    #[must_use]
    pub(crate) fn saved_searches(&self) -> &[SavedSearchEntry] {
        &self.saved_searches
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "ADR 0047 Phase B adds the in-memory contract before the loader is wired"
        )
    )]
    pub(crate) fn set_saved_searches(&mut self, saved_searches: Vec<SavedSearchEntry>) {
        self.saved_searches = saved_searches;
    }

    #[must_use]
    pub(crate) fn saved_searches_section(&self) -> Option<SavedSearchesSectionDisplay> {
        (!self.saved_searches.is_empty()).then(|| SavedSearchesSectionDisplay {
            heading: "Saved Searches",
            rows: self.saved_searches.clone(),
        })
    }

    #[must_use]
    pub(crate) fn display_description_text(description: Option<&str>) -> Option<&str> {
        display_description_text(description)
    }

    #[must_use]
    pub(crate) fn album_description_state(
        &self,
        feed_id: i64,
        description: Option<&str>,
    ) -> DescriptionState {
        self.album_description_states
            .get(&feed_id)
            .copied()
            .unwrap_or_else(|| DescriptionState::project(description_line_count(description)))
    }

    #[must_use]
    pub(crate) fn track_description_state(
        &self,
        track_id: i64,
        description: Option<&str>,
    ) -> DescriptionState {
        self.track_description_states
            .get(&track_id)
            .copied()
            .unwrap_or_else(|| DescriptionState::project(description_line_count(description)))
    }

    pub(crate) fn set_track_description_state(&mut self, track_id: i64, state: DescriptionState) {
        self.track_description_states.insert(track_id, state);
    }

    pub(crate) fn toggle_album_description(&mut self, feed_id: i64, description: Option<&str>) {
        let current = self.album_description_state(feed_id, description);
        self.album_description_states
            .insert(feed_id, current.toggle());
    }

    #[must_use]
    pub(crate) const fn chrome_display() -> LibraryChromeDisplay {
        LibraryChromeDisplay::VALUE
    }

    #[must_use]
    pub(crate) fn deferred_panel_error_message(error: impl std::fmt::Display) -> String {
        format!("Error: {error}")
    }

    #[must_use]
    pub(crate) const fn album_thumb_display() -> AlbumThumbDisplay {
        AlbumThumbDisplay {
            fallback_icon: "\u{1F3B5}",
        }
    }

    #[must_use]
    pub(crate) fn hover_thumb_display(url: &str) -> HoverThumbDisplay {
        HoverThumbDisplay {
            element_id: format!("thumb-{url}"),
        }
    }

    #[must_use]
    pub(crate) fn status_snapshot(&self) -> LibraryStatusSnapshot {
        LibraryStatusSnapshot::from_text(&self.status)
    }

    #[must_use]
    pub(crate) fn should_show_empty_library(&self, filtered_empty: bool) -> bool {
        filtered_empty && !self.status_snapshot().is_error
    }

    #[must_use]
    pub(crate) fn feed_update_display(&self) -> FeedUpdateDisplay {
        let state = &self.snapshot.feed_update_state;
        let has_stale = !state.stale.is_empty();
        let disabled = state.phase != FeedUpdatePhase::Idle;
        let action = if has_stale {
            FeedUpdateActionDisplay {
                kind: FeedUpdateActionKind::ApplyUpdates,
                button_id: "apply-feed-updates",
                label: format!("Apply updates ({})", state.stale.len()),
                disabled,
            }
        } else {
            FeedUpdateActionDisplay {
                kind: FeedUpdateActionKind::CheckAllFeeds,
                button_id: "check-all-feeds",
                label: if state.phase == FeedUpdatePhase::Checking {
                    "Checking...".into()
                } else {
                    "Check all feeds".into()
                },
                disabled,
            }
        };
        FeedUpdateDisplay {
            status_message: state.status_message.clone(),
            action,
        }
    }

    pub(crate) fn replace_playlist_tracks(&mut self, tracks: Vec<TrackRow>) {
        self.snapshot.playlist_tracks = tracks;
    }

    pub(crate) fn fail_playlist_create(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Error creating playlist: {error:#}");
    }

    pub(crate) fn fail_playlist_rename(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Error renaming: {error:#}");
    }

    pub(crate) fn fail_playlist_delete(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Error deleting: {error:#}");
    }

    pub(crate) fn fail_playlist_track_remove(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Error removing track: {error:#}");
    }

    pub(crate) fn fail_playlist_track_reorder(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Error reordering: {error:#}");
    }

    #[must_use]
    pub(crate) fn confirm_library_removal(&mut self, plan: LibraryRemovalPlan) -> bool {
        self.library_removal.confirm_or_defer(plan)
    }

    #[must_use]
    pub(crate) fn pending_library_removal_confirmation(
        &self,
    ) -> Option<LibraryRemovalConfirmationDisplay> {
        self.library_removal.pending_display()
    }

    pub(crate) fn cancel_pending_library_removal(&mut self) {
        self.library_removal.cancel();
    }

    pub(crate) fn take_pending_library_removal(&mut self) -> Option<LibraryRemovalTarget> {
        self.library_removal.take_pending_target()
    }

    pub(crate) fn fail_album_tracks_load(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Error loading album tracks: {error:#}");
    }

    pub(crate) fn set_album_has_no_tracks(&mut self) {
        self.status = "Album has no tracks".into();
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
            .playlist_by_id(playlist_id)
            .map(|playlist| playlist.name)
            .unwrap_or_default();
        self.status = format!(
            "Downloading {} track{}...",
            track_ids.len(),
            plural(track_ids.len())
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
    pub(crate) fn selected_id(&self) -> Option<i64> {
        self.selected_id
    }

    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "screen migration uses playlist selection helpers before this accessor"
        )
    )]
    pub(crate) fn selected_playlist_id(&self) -> Option<i64> {
        self.selected_playlist_id
    }

    #[must_use]
    pub(crate) fn hovered_thumb_url(&self) -> Option<&str> {
        self.hovered_thumb_url.as_deref()
    }

    pub(crate) fn set_hovered_thumb_url(&mut self, url: Option<String>) -> bool {
        if self.hovered_thumb_url == url {
            return false;
        }
        self.hovered_thumb_url = url;
        true
    }

    #[must_use]
    pub(crate) fn busy_track(&self) -> Option<i64> {
        self.busy_track
    }

    #[must_use]
    pub(crate) fn has_busy_track(&self) -> bool {
        self.busy_track.is_some()
    }

    #[must_use]
    pub(crate) fn busy_feed(&self) -> Option<i64> {
        self.busy_feed
    }

    #[must_use]
    pub(crate) fn has_busy_feed(&self) -> bool {
        self.busy_feed.is_some()
    }

    pub(crate) fn begin_busy_track(&mut self, track_id: i64, status: impl Into<String>) {
        self.busy_track = Some(track_id);
        self.library_removal.cancel();
        self.status = status.into();
    }

    pub(crate) fn begin_busy_feed(&mut self, feed_id: i64, status: impl Into<String>) {
        self.busy_feed = Some(feed_id);
        self.library_removal.cancel();
        self.status = status.into();
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "kept as a focused escape hatch for future cancellable operations"
        )
    )]
    pub(crate) fn clear_busy_track(&mut self) {
        self.busy_track = None;
    }

    pub(crate) fn clear_busy_feed(&mut self) {
        self.busy_feed = None;
    }

    pub(crate) fn finish_track_subscribe(&mut self, outcome: TrackSubscribeOutcome) {
        self.busy_track = None;
        let mut message = format!("Downloaded track: {}", outcome.path_label);
        if let Some(warning) = outcome.format_warning {
            message.push_str(" — ");
            message.push_str(&warning);
        }
        self.status = message;
    }

    pub(crate) fn fail_track_subscribe(&mut self, error: impl std::fmt::Display) {
        self.busy_track = None;
        self.status = format!("Error downloading track: {error:#}");
    }

    pub(crate) fn finish_feed_download(
        &mut self,
        downloaded: usize,
        applied_edits: usize,
        skipped: usize,
    ) {
        self.busy_feed = None;
        let mut message = format!(
            "Downloaded feed: {} track{}, applied {} ID3 edit{}",
            downloaded,
            plural(downloaded),
            applied_edits,
            plural(applied_edits)
        );
        if skipped > 0 {
            write!(&mut message, ", skipped {skipped}").expect("writing to a String cannot fail");
        }
        self.status = message;
    }

    pub(crate) fn fail_feed_download(&mut self, error: impl std::fmt::Display) {
        self.busy_feed = None;
        self.status = format!("Error downloading feed: {error:#}");
    }

    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "kept as a focused state accessor for library view-model tests"
        )
    )]
    pub(crate) fn status(&self) -> &str {
        &self.status
    }

    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "kept as a focused state accessor for search-field migration tests"
        )
    )]
    pub(crate) fn search_query(&self) -> &str {
        &self.search_query
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "active-frame search dispatch lands source-list text state before toolbar routing"
        )
    )]
    pub(crate) fn set_source_text_filter(&mut self, filter: Option<String>) {
        self.search_query = normalize(filter).unwrap_or_default();
        self.selected_id = None;
    }

    pub(crate) fn set_error_status(&mut self, error: impl std::fmt::Display) {
        self.library_loading = false;
        self.library_removal.cancel();
        self.status = format!("Error: {error:#}");
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

    pub(crate) fn select_library_item(&mut self, id: i64) {
        self.selected_id = Some(id);
        self.selected_playlist_id = None;
        self.renaming_playlist_id = None;
    }

    pub(crate) fn clear_library_selection(&mut self) {
        self.selected_id = None;
    }

    pub(crate) fn select_playlist(&mut self, playlist_id: i64) {
        self.selected_id = None;
        self.selected_playlist_id = Some(playlist_id);
        if self.renaming_playlist_id != Some(playlist_id) {
            self.renaming_playlist_id = None;
        }
    }

    pub(crate) fn clear_playlist_selection_if(&mut self, playlist_id: i64) -> bool {
        if self.selected_playlist_id == Some(playlist_id) {
            self.selected_playlist_id = None;
            self.renaming_playlist_id = None;
            return true;
        }
        false
    }

    #[must_use]
    pub(crate) fn is_playlist_selected(&self, playlist_id: i64) -> bool {
        self.selected_playlist_id == Some(playlist_id)
    }

    #[must_use]
    pub(crate) fn mb_status(&self) -> &BTreeMap<i64, MbTrackStatus> {
        &self.snapshot.mb_status
    }

    #[must_use]
    pub(crate) fn has_mb_status(&self, track_id: i64) -> bool {
        self.snapshot.mb_status.contains_key(&track_id)
    }

    pub(crate) fn set_mb_status(&mut self, track_id: i64, status: MbTrackStatus) {
        self.snapshot.mb_status.insert(track_id, status);
    }

    pub(crate) fn mark_musicbrainz_pending(&mut self, track_ids: impl IntoIterator<Item = i64>) {
        for track_id in track_ids {
            self.set_mb_status(track_id, MbTrackStatus::Pending);
        }
    }

    pub(crate) fn clear_mb_status(&mut self) {
        self.snapshot.mb_status.clear();
    }

    pub(crate) fn begin_musicbrainz_track_lookup(&mut self, track_id: i64) -> bool {
        if self.has_mb_status(track_id) {
            return false;
        }
        self.set_mb_status(track_id, MbTrackStatus::Processing);
        self.status = "MusicBrainz lookup...".into();
        true
    }

    pub(crate) fn finish_musicbrainz_track_lookup(&mut self, track_id: i64, edit_count: usize) {
        self.set_mb_status(track_id, MbTrackStatus::Done(edit_count));
        self.status = format!(
            "MusicBrainz: staged {edit_count} edit{}",
            plural(edit_count)
        );
    }

    pub(crate) fn fail_musicbrainz_track_lookup(
        &mut self,
        track_id: i64,
        error: impl std::fmt::Display,
    ) {
        self.set_mb_status(track_id, MbTrackStatus::Skipped(format!("{error:#}")));
        self.status = format!("MusicBrainz error: {error:#}");
    }

    pub(crate) fn begin_musicbrainz_album_lookup(
        &mut self,
        track_ids: impl IntoIterator<Item = i64>,
    ) -> bool {
        let track_ids: Vec<i64> = track_ids.into_iter().collect();
        if track_ids.is_empty() {
            self.status = "No downloaded tracks to process".into();
            return false;
        }
        self.mark_musicbrainz_pending(track_ids.iter().copied());
        self.status = format!(
            "MusicBrainz: album lookup for {} tracks...",
            track_ids.len()
        );
        true
    }

    pub(crate) fn fail_musicbrainz_album_lookup_with_fallback(
        &mut self,
        error: impl std::fmt::Display,
    ) {
        self.status = format!("Album lookup failed ({error:#}), falling back to per-track...");
    }

    pub(crate) fn fallback_empty_musicbrainz_album_lookup(&mut self) {
        self.status = "Album lookup: no results, falling back to per-track...".into();
    }

    pub(crate) fn begin_musicbrainz_album_track_stage(
        &mut self,
        track_id: i64,
        progress: usize,
        total_count: usize,
    ) {
        self.set_mb_status(track_id, MbTrackStatus::Processing);
        self.status = format!("MusicBrainz: staging track {progress}/{total_count} ...");
    }

    pub(crate) fn finish_musicbrainz_album_track_stage(
        &mut self,
        track_id: i64,
        status: MbTrackStatus,
    ) {
        self.set_mb_status(track_id, status);
    }

    pub(crate) fn finish_musicbrainz_album_lookup(&mut self, total_edits: usize, processed: usize) {
        self.status = format!(
            "MusicBrainz: staged {total_edits} edit{} across {processed} tracks",
            plural(total_edits)
        );
    }

    #[must_use]
    pub(crate) fn staged_musicbrainz(&self, track_id: i64) -> Option<&MusicBrainzLookupResult> {
        self.snapshot.staged_musicbrainz.get(&track_id)
    }

    pub(crate) fn stage_musicbrainz(&mut self, track_id: i64, lookup: MusicBrainzLookupResult) {
        self.snapshot.staged_musicbrainz.insert(track_id, lookup);
    }

    #[must_use]
    pub(crate) fn feed_update_state(&self) -> &FeedUpdateState {
        &self.snapshot.feed_update_state
    }

    pub(crate) fn begin_feed_view_check(&mut self, feed_id: i64) -> bool {
        if self.snapshot.feed_update_state.phase == FeedUpdatePhase::Applying
            || self
                .snapshot
                .feed_update_state
                .stale
                .iter()
                .any(|entry| entry.feed_id == feed_id)
            || !self.snapshot.in_flight_feed_checks.insert(feed_id)
        {
            return false;
        }
        self.snapshot.feed_update_state.status_message = Some("Checking feed...".into());
        true
    }

    pub(crate) fn finish_feed_view_check(
        &mut self,
        feed_id: i64,
        result: Result<Option<feed_service::StaleFeed>, String>,
    ) {
        self.snapshot.in_flight_feed_checks.remove(&feed_id);
        match result {
            Ok(Some(entry)) => {
                if !self
                    .snapshot
                    .feed_update_state
                    .stale
                    .iter()
                    .any(|existing| existing.feed_id == entry.feed_id)
                {
                    self.snapshot.feed_update_state.stale.push(entry);
                }
                self.snapshot.feed_update_state.status_message = Some(
                    Self::pending_feed_update_label(self.snapshot.feed_update_state.stale.len()),
                );
            }
            Ok(None) => {
                if self.snapshot.feed_update_state.stale.is_empty()
                    && self.snapshot.in_flight_feed_checks.is_empty()
                {
                    self.snapshot.feed_update_state.status_message = Some("Feed up to date".into());
                }
            }
            Err(err) => {
                self.snapshot.feed_update_state.status_message =
                    Some(format!("Feed check error: {err}"));
            }
        }
    }

    pub(crate) fn finish_feed_view_check_error(
        &mut self,
        feed_id: i64,
        error: impl std::fmt::Display,
    ) {
        self.finish_feed_view_check(feed_id, Err(format!("{error:#}")));
    }

    pub(crate) fn set_feed_check_error(&mut self, message: impl std::fmt::Display) {
        self.snapshot.feed_update_state.status_message =
            Some(format!("Feed check error: {message:#}"));
    }

    pub(crate) fn set_no_subscribed_feeds(&mut self) {
        self.snapshot.feed_update_state.status_message =
            Some("No subscribed feeds to check".into());
    }

    pub(crate) fn begin_all_feed_check(&mut self, feed_count: usize) {
        self.snapshot.feed_update_state.phase = FeedUpdatePhase::Checking;
        self.snapshot.feed_update_state.stale.clear();
        self.snapshot.feed_update_state.status_message =
            Some(format!("Checking {feed_count} feeds..."));
    }

    pub(crate) fn finish_all_feed_check(&mut self, stale: Vec<feed_service::StaleFeed>) {
        self.snapshot.feed_update_state.phase = FeedUpdatePhase::Idle;
        self.snapshot.feed_update_state.stale = stale;
        self.snapshot.feed_update_state.status_message =
            Some(if self.snapshot.feed_update_state.stale.is_empty() {
                "All feeds up to date".into()
            } else {
                format!(
                    "{} feed update{} available",
                    self.snapshot.feed_update_state.stale.len(),
                    plural(self.snapshot.feed_update_state.stale.len())
                )
            });
    }

    pub(crate) fn begin_apply_feed_updates(&mut self) -> Option<Vec<feed_service::StaleFeed>> {
        if self.snapshot.feed_update_state.phase != FeedUpdatePhase::Idle
            || self.snapshot.feed_update_state.stale.is_empty()
        {
            return None;
        }
        let stale = self.snapshot.feed_update_state.stale.clone();
        self.snapshot.feed_update_state.phase = FeedUpdatePhase::Applying;
        self.snapshot.feed_update_state.status_message =
            Some(format!("Applying updates to {} feed(s)...", stale.len()));
        Some(stale)
    }

    pub(crate) fn finish_apply_feed_updates(&mut self, message: String) {
        self.snapshot.feed_update_state.phase = FeedUpdatePhase::Idle;
        self.snapshot.feed_update_state.stale.clear();
        self.snapshot.feed_update_state.status_message = Some(message);
    }

    pub(crate) fn finish_apply_feed_updates_error(&mut self, error: impl std::fmt::Display) {
        self.finish_apply_feed_updates(format!("Feed update error: {error:#}"));
    }

    #[cfg(test)]
    pub(crate) fn apply_search_query(&mut self, query: impl Into<String>) {
        self.set_source_text_filter(Some(query.into()));
    }

    pub(crate) fn toggle_creating_playlist(&mut self) {
        self.creating_playlist = !self.creating_playlist;
    }

    pub(crate) fn close_creating_playlist(&mut self) {
        self.creating_playlist = false;
    }

    pub(crate) fn begin_playlist_rename(&mut self, playlist_id: i64) {
        self.renaming_playlist_id = Some(playlist_id);
        self.creating_playlist = false;
    }

    pub(crate) fn cancel_playlist_rename(&mut self) {
        self.renaming_playlist_id = None;
    }

    #[must_use]
    pub(crate) fn renaming_playlist_id(&self) -> Option<i64> {
        self.renaming_playlist_id
    }

    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "kept as a focused state accessor for playlist rename tests and future shell guards"
        )
    )]
    pub(crate) fn is_renaming_playlist(&self, playlist_id: i64) -> bool {
        self.renaming_playlist_id == Some(playlist_id)
    }

    /// Toggle the expansion state of an artist node by name.
    pub(crate) fn toggle_artist(&mut self, name: &str) {
        if !self.expanded_artists.remove(name) {
            self.expanded_artists.insert(name.to_string());
        }
    }

    /// Toggle the expansion state of an album node, keyed on the
    /// `(artist, album)` pair to keep two albums with the same name
    /// from collapsing onto each other.
    pub(crate) fn toggle_album(&mut self, artist: &str, album: &str) {
        let key = (artist.to_string(), album.to_string());
        if !self.expanded_albums.remove(&key) {
            self.expanded_albums.insert(key);
        }
    }

    /// Flip the playlists sidebar expansion flag.
    pub(crate) fn toggle_playlists_expanded(&mut self) {
        self.playlists_expanded = !self.playlists_expanded;
    }

    /// Advance the playlist sort order one step.
    pub(crate) fn cycle_playlist_sort(&mut self) {
        self.playlist_sort = self.playlist_sort.next();
    }

    fn sort_playlists_by(sort: PlaylistSort, playlists: &mut [db::Playlist]) {
        match sort {
            PlaylistSort::Name => {
                playlists.sort_by_key(|playlist| playlist.name.to_lowercase());
            }
            PlaylistSort::RecentlyUpdated => {
                playlists.sort_by_key(|playlist| std::cmp::Reverse(playlist.updated_at));
            }
            PlaylistSort::TrackCount => {
                playlists.sort_by_key(|playlist| std::cmp::Reverse(playlist.track_count));
            }
        }
    }

    fn pending_feed_update_label(count: usize) -> String {
        format!("{count} feed update{} pending", plural(count))
    }

    // Both lookups are exercised by the unit tests below but not yet
    // by the legacy renderer (which still uses `LibraryTreeProjection`).
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "kept as focused expansion predicates for future tree rendering"
        )
    )]
    #[must_use]
    pub(crate) fn is_artist_expanded(&self, name: &str) -> bool {
        self.expanded_artists.contains(name)
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "kept as focused expansion predicates for future tree rendering"
        )
    )]
    #[must_use]
    pub(crate) fn is_album_expanded(&self, artist: &str, album: &str) -> bool {
        self.expanded_albums
            .contains(&(artist.to_string(), album.to_string()))
    }

    #[must_use]
    pub(crate) fn playlist_sort_label(&self) -> &'static str {
        self.playlist_sort.label()
    }
}

fn filter_tree(tree: &LibraryTree, query: &str) -> LibraryTree {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return tree.clone();
    }
    let mut artists = Vec::new();
    for artist in &tree.artists {
        let artist_match = artist.name.to_lowercase().contains(&q);
        let mut albums = Vec::new();
        for album in &artist.albums {
            let album_match = album.name.to_lowercase().contains(&q);
            let keep_all = artist_match || album_match;
            let tracks: Vec<TrackRow> = if keep_all {
                album.tracks.clone()
            } else {
                album
                    .tracks
                    .iter()
                    .filter(|track| {
                        track
                            .track_title
                            .as_deref()
                            .unwrap_or("")
                            .to_lowercase()
                            .contains(&q)
                    })
                    .cloned()
                    .collect()
            };
            if keep_all || !tracks.is_empty() {
                albums.push(AlbumNode {
                    name: album.name.clone(),
                    feed_id: album.feed_id,
                    feed_guid: album.feed_guid.clone(),
                    feed_url: album.feed_url.clone(),
                    language: album.language.clone(),
                    description: album.description.clone(),
                    image_href: album.image_href.clone(),
                    identity_facts: album.identity_facts.clone(),
                    tracks,
                });
            }
        }
        if !albums.is_empty() {
            artists.push(ArtistNode {
                name: artist.name.clone(),
                albums,
            });
        }
    }
    LibraryTree { artists }
}

fn track_row_matches_text_filter(track: &TrackRow, filter: &str) -> bool {
    [
        track.track_title.as_deref(),
        track.artist_name.as_deref(),
        track.album_artist_name.as_deref(),
        track.album_title.as_deref(),
        track.feed_title.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|value| contains_normalized(value, filter))
}

fn content_list_rows_from_tree(tree: &LibraryTree) -> Vec<ContentListRowDisplay> {
    tree.artists
        .iter()
        .flat_map(|artist| &artist.albums)
        .flat_map(|album| album.tracks.iter())
        .map(ContentListRowDisplay::from_track)
        .collect()
}

impl Default for LibraryViewModel {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> LibraryTrackRowVm<'a> {
    #[must_use]
    pub(crate) fn new(track: &'a TrackRow, mb: Option<&'a MbTrackStatus>) -> Self {
        Self { track, mb }
    }

    /// Display title for inspector/header surfaces:
    /// `track_title -> feed_title -> "Untitled"`.
    #[must_use]
    pub(crate) fn display_title(&self) -> String {
        self.track
            .track_title
            .clone()
            .or_else(|| self.track.feed_title.clone())
            .unwrap_or_else(|| "Untitled".to_string())
    }

    /// Compact row title used by tree-style rows. Kept distinct from
    /// [`Self::display_title`] to preserve the legacy bracketed empty state.
    #[must_use]
    pub(crate) fn compact_title(&self) -> String {
        self.track
            .track_title
            .as_deref()
            .unwrap_or("[untitled]")
            .to_string()
    }

    /// Leading zero-padded tree-row number, empty when there is no track
    /// number.
    #[must_use]
    pub(crate) fn tree_number_prefix(&self) -> String {
        self.track
            .track_number
            .map(|n| format!("{n:02} - "))
            .unwrap_or_default()
    }

    /// Complete display contract for the track row inside the Library
    /// sidebar tree.
    #[must_use]
    pub(crate) fn tree_display(&self) -> LibraryTreeTrackDisplay {
        LibraryTreeTrackDisplay {
            element_id: format!("tree-track-{}", self.track.id),
            title: format!("{}{}", self.tree_number_prefix(), self.compact_title()),
        }
    }

    /// Artist grouping label:
    /// `album_artist_name -> artist_name -> "Unknown Artist"`.
    #[must_use]
    pub(crate) fn display_artist(&self) -> String {
        self.track
            .album_artist_name
            .clone()
            .or_else(|| self.track.artist_name.clone())
            .unwrap_or_else(|| "Unknown Artist".to_string())
    }

    /// Album grouping label: `album_title -> feed_title -> "Unknown Album"`.
    #[must_use]
    pub(crate) fn display_album(&self) -> String {
        self.track
            .album_title
            .clone()
            .or_else(|| self.track.feed_title.clone())
            .unwrap_or_else(|| "Unknown Album".to_string())
    }

    /// Leading `"{n}. "` segment, empty when there is no track number.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn number_prefix(&self) -> String {
        self.track
            .track_number
            .map(|n| format!("{n}. "))
            .unwrap_or_default()
    }

    /// Trailing `"  (M:SS)"` segment, empty when there is no
    /// duration.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn duration_suffix(&self) -> String {
        self.track
            .duration_seconds
            .map(|s| format!("  ({}:{:02})", s / 60, s % 60))
            .unwrap_or_default()
    }

    #[must_use]
    pub(crate) fn primary_action_vm(&self, is_busy: bool) -> EntityActionVm {
        self.track_action_state(is_busy, false)
            .primary_action(EntityActionTarget::Track(self.track_ref()))
    }

    #[must_use]
    pub(crate) fn row_display(&self) -> LibraryTrackRowDisplay {
        let track_id = self.track.id;
        LibraryTrackRowDisplay {
            row_id: format!("album-track-{track_id}"),
            toggle_button_id: format!("lib-toggle-{track_id}"),
        }
    }

    #[must_use]
    pub(crate) fn playlist_display(&self) -> LibraryTrackPlaylistDisplay {
        LibraryTrackPlaylistDisplay {
            popover_id: format!("album-track-add:{}", self.track.id),
            trigger_label: "+ Playlist",
        }
    }

    #[must_use]
    pub(crate) fn track_action_state(
        &self,
        is_busy: bool,
        playlist_open: bool,
    ) -> TrackActionState {
        let membership = match (self.track.is_in_library, is_busy) {
            (true, true) => TrackMembershipState::Removing,
            (true, false) => TrackMembershipState::InLibrary,
            (false, true) => TrackMembershipState::Downloading,
            (false, false) => TrackMembershipState::RemoteOnly,
        };
        let playlist = if playlist_open {
            PlaylistActionState::Open
        } else {
            PlaylistActionState::Closed
        };
        TrackActionState::new(membership, playlist)
    }

    #[must_use]
    fn track_ref(&self) -> TrackRef {
        TrackRef::LocalTrackId(self.track.id)
    }

    /// Human-readable `MusicBrainz` status hint, or `None` when no
    /// lookup has been started for this track.
    #[must_use]
    pub(crate) fn mb_status_text(&self) -> Option<&'static str> {
        match self.mb? {
            MbTrackStatus::Pending => Some("MB: pending"),
            MbTrackStatus::Processing => Some("MB: looking up..."),
            MbTrackStatus::Done(0) => Some("MB: no missing fields"),
            MbTrackStatus::Done(_) => Some("MB: done"),
            MbTrackStatus::Skipped(_) => Some("MB: skipped"),
        }
    }

    /// Semantic colour bucket for the status hint (or `None` when
    /// there is no hint).
    #[must_use]
    pub(crate) fn mb_status_kind(&self) -> Option<MbStatusKind> {
        match self.mb? {
            MbTrackStatus::Done(n) if *n > 0 => Some(MbStatusKind::Success),
            MbTrackStatus::Skipped(_) => Some(MbStatusKind::Danger),
            MbTrackStatus::Processing => Some(MbStatusKind::Warning),
            _ => Some(MbStatusKind::Muted),
        }
    }
}

/// Display-ready projection of a feed-row inside the library artist
/// detail. The screen looks up the actual thumbnail image by `thumb_url`
/// and wires the click handler by `feed_id`; the VM only carries plain
/// data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ArtistFeedSummaryVm {
    pub(crate) feed_id: i64,
    pub(crate) feed_name: String,
    pub(crate) thumb_url: Option<String>,
    pub(crate) track_count: usize,
}

/// Display contract for a feed summary row in an artist detail panel.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ArtistFeedSummaryDisplay {
    pub(crate) element_id: String,
    pub(crate) title: String,
    pub(crate) thumb_url: Option<String>,
    pub(crate) track_count_label: String,
}

impl ArtistFeedSummaryVm {
    #[must_use]
    pub(crate) fn display(&self) -> ArtistFeedSummaryDisplay {
        ArtistFeedSummaryDisplay {
            element_id: format!("artist-feed-{}", self.feed_name),
            title: self.feed_name.clone(),
            thumb_url: self.thumb_url.clone(),
            track_count_label: format!("{} track{}", self.track_count, plural(self.track_count)),
        }
    }
}

/// Display-ready projection of a library artist detail panel.
///
/// Borrow-only — constructed fresh each render and dropped before the
/// element tree is painted. The VM groups tracks by feed and applies the
/// "Untitled Feed" / "Unknown" fallbacks the legacy renderer used.
pub(crate) struct LibraryArtistDetailVm<'a> {
    name: &'a str,
    view: Option<&'a ArtistView>,
    tracks: &'a [TrackRow],
}

impl<'a> LibraryArtistDetailVm<'a> {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "kept for legacy artist-detail VM tests and no-enrichment callers"
        )
    )]
    #[must_use]
    pub(crate) fn new(name: &'a str, tracks: &'a [TrackRow]) -> Self {
        Self {
            name,
            view: None,
            tracks,
        }
    }

    #[must_use]
    pub(crate) fn with_view(name: &'a str, view: &'a ArtistView, tracks: &'a [TrackRow]) -> Self {
        Self {
            name,
            view: Some(view),
            tracks,
        }
    }

    /// Artist name with the legacy `"Unknown"` fallback applied when
    /// empty.
    #[must_use]
    pub(crate) fn artist_name_or_unknown(&self) -> String {
        if self.name.is_empty() {
            "Unknown".to_string()
        } else {
            self.name.to_string()
        }
    }

    /// Number of distinct feeds (== albums) under this artist.
    #[must_use]
    pub(crate) fn album_count(&self) -> usize {
        let mut feeds = std::collections::BTreeSet::new();
        for t in self.tracks {
            feeds.insert(t.feed_id);
        }
        feeds.len()
    }

    /// Total track count.
    #[must_use]
    pub(crate) fn track_count(&self) -> usize {
        self.tracks.len()
    }

    /// Number of tracks that have been downloaded to disk.
    #[must_use]
    pub(crate) fn downloaded_count(&self) -> usize {
        self.tracks
            .iter()
            .filter(|t| t.local_path.is_some())
            .count()
    }

    /// Detail-grid rows in display order: `Albums`, `Tracks` (with
    /// pluralised count), and `Downloaded` (only when at least one track
    /// is downloaded).
    #[must_use]
    pub(crate) fn detail_rows(&self) -> Vec<(String, String)> {
        let mut rows = vec![
            ("Albums".to_string(), self.album_count().to_string()),
            (
                "Tracks".to_string(),
                format!("{} track{}", self.track_count(), plural(self.track_count())),
            ),
        ];
        let downloaded = self.downloaded_count();
        if downloaded > 0 {
            rows.push(("Downloaded".to_string(), downloaded.to_string()));
        }
        self.push_artist_source_rows(&mut rows);
        rows
    }

    fn push_artist_source_rows(&self, rows: &mut Vec<(String, String)>) {
        let Some(view) = self.view else {
            return;
        };
        if view.source_subjects.len() > 1 {
            rows.push((
                "Source Subjects".to_string(),
                format!("{} explicit subjects", view.source_subjects.len()),
            ));
            return;
        }
        push_string_row(rows, "Sort Name", view.sort_name.as_deref());
        push_string_row(rows, "Area", view.area.as_deref());
        if let Some(active) = artist_active_years(view.begin_year, view.end_year) {
            rows.push(("Active".to_string(), active));
        }
        push_string_row(rows, "Website", view.url.as_deref());
        if !view.aliases.is_empty() {
            rows.push(("Aliases".to_string(), view.aliases.join(", ")));
        }
    }

    #[must_use]
    pub(crate) fn page(&self) -> ArtistDetailPageVm {
        ArtistDetailPageVm::new(
            self.artist_name_or_unknown(),
            None::<String>,
            self.detail_rows()
                .into_iter()
                .map(|(key, value)| ArtistDetailFactVm::new(key, value, 6))
                .collect(),
            !self.feed_summaries().is_empty(),
        )
    }

    /// One [`ArtistFeedSummaryVm`] per distinct feed, ordered by
    /// `feed_id` (matches `BTreeMap` iteration of the legacy renderer).
    #[must_use]
    pub(crate) fn feed_summaries(&self) -> Vec<ArtistFeedSummaryVm> {
        let mut feed_map: BTreeMap<i64, (Option<String>, Vec<&TrackRow>)> = BTreeMap::new();
        for track in self.tracks {
            feed_map
                .entry(track.feed_id)
                .or_insert_with(|| (track.feed_title.clone(), Vec::new()))
                .1
                .push(track);
        }
        feed_map
            .into_iter()
            .map(|(feed_id, (feed_title, tracks))| {
                let feed_name = feed_title.unwrap_or_else(|| "Untitled Feed".to_string());
                let first = tracks.first();
                let thumb_url = first.and_then(|t| {
                    t.album_image_href
                        .clone()
                        .or_else(|| t.track_image_href.clone())
                });
                ArtistFeedSummaryVm {
                    feed_id,
                    feed_name,
                    thumb_url,
                    track_count: tracks.len(),
                }
            })
            .collect()
    }
}

fn push_string_row(rows: &mut Vec<(String, String)>, key: &str, value: Option<&str>) {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    rows.push((key.to_string(), value.to_string()));
}

fn artist_active_years(begin_year: Option<i32>, end_year: Option<i32>) -> Option<String> {
    match (begin_year, end_year) {
        (Some(begin), Some(end)) => Some(format!("{begin}-{end}")),
        (Some(begin), None) => Some(format!("{begin}-")),
        (None, Some(end)) => Some(format!("until {end}")),
        (None, None) => None,
    }
}

/// Display-ready action projection for a library album detail panel.
///
/// Borrow-only — constructed fresh each render and dropped before the element
/// tree is painted. ADR 0031 moved title, summary facts, and description
/// placement into the shared release-detail contract; this VM keeps only the
/// action-state projections the Library screen needs to bind handlers.
pub(crate) struct LibraryAlbumDetailVm<'a> {
    mb_status: &'a BTreeMap<i64, MbTrackStatus>,
    description_state: DescriptionState,
    has_library_tracks: bool,
}

/// Display contract for the Library album `MusicBrainz` action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LibraryAlbumMusicBrainzActionVm {
    pub(crate) label: &'static str,
    pub(crate) a11y_label: &'static str,
    pub(crate) disabled: bool,
}

/// Display contract for the Library album playlist popover trigger.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LibraryAlbumPlaylistDisplay {
    pub(crate) popover_id: String,
    pub(crate) trigger_label: String,
}

impl<'a> LibraryAlbumDetailVm<'a> {
    #[must_use]
    pub(crate) fn new(
        feed_view: &'a FeedView,
        tracks: &'a [TrackRow],
        mb_status: &'a BTreeMap<i64, MbTrackStatus>,
    ) -> Self {
        Self {
            mb_status,
            has_library_tracks: tracks.iter().any(|track| track.is_in_library),
            description_state: DescriptionState::project(description_line_count(
                feed_view.description.as_deref(),
            )),
        }
    }

    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "ADR 0047 Phase B projects description state before Phase C renders it"
        )
    )]
    pub(crate) const fn description_state(&self) -> DescriptionState {
        self.description_state
    }

    /// `true` when any track has an in-flight `MusicBrainz` lookup —
    /// used by the screen to disable the `MusicBrainz` action button.
    #[must_use]
    pub(crate) fn has_active_musicbrainz(&self) -> bool {
        self.mb_status
            .values()
            .any(|s| matches!(s, MbTrackStatus::Pending | MbTrackStatus::Processing))
    }

    #[must_use]
    pub(crate) fn primary_action_vm(&self, feed_id: i64, is_busy: bool) -> EntityActionVm {
        self.release_action_state(is_busy, PlaylistActionState::Hidden)
            .primary_action(EntityActionTarget::Feed(FeedRef::LocalFeedId(feed_id)))
    }

    #[must_use]
    pub(crate) fn track_row_busy(
        &self,
        track: &TrackRow,
        track_busy: bool,
        feed_busy: bool,
    ) -> bool {
        track_busy || (feed_busy && (!self.has_library_tracks || track.is_in_library))
    }

    #[must_use]
    pub(crate) fn playlist_action_vm(&self, feed_id: i64) -> Option<EntityActionVm> {
        self.release_action_state(false, PlaylistActionState::Closed)
            .playlist_action(EntityActionTarget::Feed(FeedRef::LocalFeedId(feed_id)))
    }

    #[must_use]
    pub(crate) fn musicbrainz_action_vm(&self) -> LibraryAlbumMusicBrainzActionVm {
        LibraryAlbumMusicBrainzActionVm {
            label: "MusicBrainz",
            a11y_label: "Look up missing MusicBrainz fields for this album",
            disabled: self.has_active_musicbrainz(),
        }
    }

    #[must_use]
    pub(crate) fn playlist_display(&self, feed_id: i64) -> Option<LibraryAlbumPlaylistDisplay> {
        self.playlist_action_vm(feed_id)
            .map(|action| LibraryAlbumPlaylistDisplay {
                popover_id: format!("album-feed-add:{feed_id}"),
                trigger_label: action.label,
            })
    }

    #[must_use]
    fn release_action_state(
        &self,
        is_busy: bool,
        playlist: PlaylistActionState,
    ) -> ReleaseActionState {
        let membership = if is_busy {
            if self.has_library_tracks {
                ReleaseMembershipState::Removing
            } else {
                ReleaseMembershipState::Downloading
            }
        } else if self.has_library_tracks {
            ReleaseMembershipState::InLibrary
        } else {
            ReleaseMembershipState::RemoteOnly
        };

        ReleaseActionState::new(membership, playlist)
    }
}

impl AlbumNode {
    /// Feed URL string required by the shared release-detail projection.
    /// Absence is centralized here so screens do not call `unwrap_or_default`
    /// on `feed_url`.
    #[must_use]
    pub(crate) fn feed_url_for_detail(&self) -> String {
        self.feed_url.clone().unwrap_or_default()
    }
}

/// Display-ready projection of a single row inside a playlist detail
/// listing. The screen owns the click handlers and control rendering;
/// the VM owns text fallbacks, duration formatting, and the reorder
/// availability contract.
pub(crate) struct PlaylistTrackRowVm<'a> {
    track: &'a TrackRow,
    position: usize,
    last_position: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlaylistTrackMenuItemDisplay {
    pub(crate) id: String,
    pub(crate) label: &'static str,
    pub(crate) a11y_label: &'static str,
    pub(crate) destructive: bool,
    pub(crate) disabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlaylistTrackControlsDisplay {
    pub(crate) row_id: String,
    pub(crate) row_body_id: String,
    pub(crate) drag_handle_id: String,
    pub(crate) drag_handle_a11y_label: &'static str,
    pub(crate) actions_menu_id: String,
    pub(crate) actions_menu_a11y_label: &'static str,
    pub(crate) play_button_id: String,
    pub(crate) play_label: &'static str,
    pub(crate) play_enabled: bool,
    pub(crate) move_up_menu_item: PlaylistTrackMenuItemDisplay,
    pub(crate) move_down_menu_item: PlaylistTrackMenuItemDisplay,
    pub(crate) remove_menu_item: PlaylistTrackMenuItemDisplay,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlaylistTrackRowDisplay {
    pub(crate) is_available: bool,
    pub(crate) position: i64,
    pub(crate) position_label: String,
    pub(crate) title: String,
    pub(crate) artist: String,
    pub(crate) availability_label: Option<&'static str>,
    pub(crate) duration_label: String,
    pub(crate) thumb_url: Option<String>,
    pub(crate) controls: PlaylistTrackControlsDisplay,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlaylistDetailActionsDisplay {
    pub(crate) rename_button_id: String,
    pub(crate) rename_label: &'static str,
    pub(crate) rename_a11y_label: &'static str,
    pub(crate) rename_input_id: String,
    pub(crate) rename_input_placeholder: &'static str,
    pub(crate) rename_save_button_id: String,
    pub(crate) rename_save_label: &'static str,
    pub(crate) rename_save_a11y_label: &'static str,
    pub(crate) rename_cancel_button_id: String,
    pub(crate) rename_cancel_label: &'static str,
    pub(crate) rename_cancel_a11y_label: &'static str,
    pub(crate) delete_button_id: String,
    pub(crate) delete_label: &'static str,
    pub(crate) delete_a11y_label: &'static str,
}

impl PlaylistDetailActionsDisplay {
    pub(crate) const RENAME_INPUT_PLACEHOLDER: &'static str = "Playlist name";
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlaylistDetailHeaderDisplay {
    pub(crate) title: String,
}

impl<'a> PlaylistTrackRowVm<'a> {
    /// Construct a row VM from a borrowed track and its position in
    /// the surrounding playlist.
    ///
    /// Used by both the eager projection (`PlaylistDetailVm::track_rows`)
    /// and the paged screen path which materialises one row at a time
    /// from a [`super::paged_playlist_detail::PagedPlaylistDetailVm`].
    #[must_use]
    #[cfg_attr(
        not(feature = "async-runtime"),
        expect(
            dead_code,
            reason = "constructor is consumed by the paged playlist screen which is gated on `async-runtime`"
        )
    )]
    pub(crate) const fn new(track: &'a TrackRow, position: usize, last_position: usize) -> Self {
        Self {
            track,
            position,
            last_position,
        }
    }

    #[must_use]
    pub(crate) fn track(&self) -> &TrackRow {
        self.track
    }

    #[must_use]
    pub(crate) fn position(&self) -> i64 {
        i64::try_from(self.position).unwrap_or(i64::MAX)
    }

    /// `"{n}."` where `n` is the 1-indexed position.
    #[must_use]
    pub(crate) fn position_label(&self) -> String {
        format!("{}.", self.position + 1)
    }

    /// Title with the legacy `"[untitled]"` fallback.
    #[must_use]
    pub(crate) fn title(&self) -> String {
        self.track
            .track_title
            .as_deref()
            .unwrap_or("[untitled]")
            .to_string()
    }

    /// Artist with the legacy `"Unknown"` fallback.
    #[must_use]
    pub(crate) fn artist(&self) -> String {
        self.track
            .artist_name
            .as_deref()
            .unwrap_or("Unknown")
            .to_string()
    }

    /// `"M:SS"` formatted duration, or `""` when the track has none.
    #[must_use]
    pub(crate) fn duration_label(&self) -> String {
        self.track
            .duration_seconds
            .map(|s| format!("{}:{:02}", s / 60, s % 60))
            .unwrap_or_default()
    }

    /// Preferred thumbnail URL — `track_image_href` first, then
    /// `album_image_href`. Matches the legacy renderer's lookup order.
    #[must_use]
    pub(crate) fn thumb_url(&self) -> Option<&str> {
        self.track
            .track_image_href
            .as_deref()
            .or(self.track.album_image_href.as_deref())
    }

    /// `true` when the track has a local file and can be played.
    #[must_use]
    pub(crate) fn can_play(&self) -> bool {
        self.track.is_in_library && self.track.local_path.is_some()
    }

    #[must_use]
    pub(crate) fn is_available(&self) -> bool {
        self.can_play()
    }

    #[must_use]
    pub(crate) fn availability_label(&self) -> Option<&'static str> {
        (!self.is_available()).then_some("Unavailable")
    }

    #[must_use]
    pub(crate) fn can_move_up(&self) -> bool {
        self.position > 0
    }

    #[must_use]
    pub(crate) fn can_move_down(&self) -> bool {
        self.position < self.last_position
    }

    #[must_use]
    pub(crate) fn controls_display(&self, playlist_id: i64) -> PlaylistTrackControlsDisplay {
        let position = self.position;
        let track_id = self.track.id;
        PlaylistTrackControlsDisplay {
            row_id: format!("playlist-track-{track_id}-{position}"),
            row_body_id: format!("playlist-row-body-{playlist_id}-{position}"),
            drag_handle_id: format!("playlist-drag-handle-{playlist_id}-{position}"),
            drag_handle_a11y_label: "Drag to reorder playlist track",
            actions_menu_id: format!("playlist-actions-{playlist_id}-{position}"),
            actions_menu_a11y_label: "Playlist track actions",
            play_button_id: format!("playlist-play-{playlist_id}-{position}"),
            play_label: "▶",
            play_enabled: self.can_play(),
            move_up_menu_item: PlaylistTrackMenuItemDisplay {
                id: format!("playlist-move-up-{playlist_id}-{position}"),
                label: "Move Up",
                a11y_label: "Move track up",
                destructive: false,
                disabled: !self.can_move_up(),
            },
            move_down_menu_item: PlaylistTrackMenuItemDisplay {
                id: format!("playlist-move-down-{playlist_id}-{position}"),
                label: "Move Down",
                a11y_label: "Move track down",
                destructive: false,
                disabled: !self.can_move_down(),
            },
            remove_menu_item: PlaylistTrackMenuItemDisplay {
                id: format!("playlist-remove-{playlist_id}-{position}"),
                label: "Remove",
                a11y_label: "Remove track from playlist",
                destructive: true,
                disabled: false,
            },
        }
    }

    #[must_use]
    pub(crate) fn display(&self, playlist_id: i64) -> PlaylistTrackRowDisplay {
        let position = i64::try_from(self.position).unwrap_or(i64::MAX);
        PlaylistTrackRowDisplay {
            is_available: self.is_available(),
            position,
            position_label: self.position_label(),
            title: self.title(),
            artist: self.artist(),
            availability_label: self.availability_label(),
            duration_label: self.duration_label(),
            thumb_url: self.thumb_url().map(str::to_string),
            controls: self.controls_display(playlist_id),
        }
    }
}

/// Display-ready projection of a playlist detail panel.
///
/// Borrow-only — constructed fresh each render and dropped before the
/// element tree is painted. The VM owns the duration roll-up,
/// detail-row composition, and per-track projections.
pub(crate) struct PlaylistDetailVm<'a> {
    playlist: &'a db::Playlist,
    tracks: &'a [TrackRow],
    text_filter: Option<String>,
}

impl<'a> PlaylistDetailVm<'a> {
    #[must_use]
    pub(crate) fn new(playlist: &'a db::Playlist, tracks: &'a [TrackRow]) -> Self {
        Self {
            playlist,
            tracks,
            text_filter: None,
        }
    }

    #[must_use]
    pub(crate) fn playlist_id(&self) -> i64 {
        self.playlist.id
    }

    #[must_use]
    pub(crate) fn page(self, scroll_id: &'static str) -> PlaylistDetailPageVm<'a> {
        PlaylistDetailPageVm::new(self, scroll_id)
    }

    #[must_use]
    pub(crate) fn header_display(&self) -> PlaylistDetailHeaderDisplay {
        PlaylistDetailHeaderDisplay {
            title: self.playlist.name.clone(),
        }
    }

    #[must_use]
    pub(crate) fn track_count(&self) -> usize {
        self.visible_tracks().len()
    }

    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.visible_tracks().is_empty()
    }

    #[must_use]
    pub(crate) fn text_filter(&self) -> Option<&str> {
        self.text_filter.as_deref()
    }

    pub(crate) fn set_text_filter(&mut self, filter: Option<String>) {
        self.text_filter = normalize(filter);
    }

    /// Sum of all track durations in seconds.
    #[must_use]
    pub(crate) fn total_duration_seconds(&self) -> i64 {
        self.visible_tracks()
            .into_iter()
            .filter_map(|(_, track)| track.duration_seconds)
            .sum()
    }

    /// `"M:SS"` for short playlists, `"Hh Mm"` once total runtime
    /// crosses an hour, or `None` when the total is zero (no track
    /// has a known duration). Matches the legacy renderer exactly.
    #[must_use]
    pub(crate) fn total_duration_label(&self) -> Option<String> {
        fmt_total_runtime_clock(self.total_duration_seconds())
    }

    /// Detail-grid rows in display order.
    #[must_use]
    pub(crate) fn detail_rows(&self) -> Vec<(String, String)> {
        let mut rows = vec![("Tracks".to_string(), self.track_count().to_string())];
        if let Some(label) = self.total_duration_label() {
            rows.push(("Duration".to_string(), label));
        }
        if self.playlist.created_at > 0 {
            if let Some(label) = fmt_date(self.playlist.created_at) {
                rows.push(("Created".to_string(), label));
            }
        }
        if self.playlist.updated_at > 0 {
            if let Some(label) = fmt_date(self.playlist.updated_at) {
                rows.push(("Modified".to_string(), label));
            }
        }
        if let Some(description) = self.playlist.description.as_deref().map(str::trim) {
            if !description.is_empty() {
                rows.push(("Description".to_string(), description.to_string()));
            }
        }
        rows
    }

    /// Empty-state message rendered in place of the track list.
    #[expect(
        clippy::unused_self,
        reason = "kept as a method for API symmetry with the other accessors"
    )]
    #[must_use]
    pub(crate) fn empty_message(&self) -> &'static str {
        "Empty — add tracks from the library or search"
    }

    #[must_use]
    pub(crate) fn actions_display(&self) -> PlaylistDetailActionsDisplay {
        let playlist_id = self.playlist_id();
        PlaylistDetailActionsDisplay {
            rename_button_id: format!("playlist-rename-{playlist_id}"),
            rename_label: "Rename",
            rename_a11y_label: "Rename playlist",
            rename_input_id: format!("playlist-rename-input-{playlist_id}"),
            rename_input_placeholder: PlaylistDetailActionsDisplay::RENAME_INPUT_PLACEHOLDER,
            rename_save_button_id: format!("playlist-rename-save-{playlist_id}"),
            rename_save_label: "Save",
            rename_save_a11y_label: "Save playlist name",
            rename_cancel_button_id: format!("playlist-rename-cancel-{playlist_id}"),
            rename_cancel_label: "Cancel",
            rename_cancel_a11y_label: "Cancel playlist rename",
            delete_button_id: format!("playlist-delete-{playlist_id}"),
            delete_label: "Delete",
            delete_a11y_label: "Delete playlist",
        }
    }

    /// One [`PlaylistTrackRowVm`] per track, in stored order. Returns
    /// an empty vec when the playlist has no tracks (callers can use
    /// [`Self::is_empty`] to branch on the empty-state message).
    #[must_use]
    pub(crate) fn track_rows(&self) -> Vec<PlaylistTrackRowVm<'a>> {
        let last_position = self.tracks.len().saturating_sub(1);
        self.visible_tracks()
            .into_iter()
            .map(|(position, track)| PlaylistTrackRowVm {
                track,
                position,
                last_position,
            })
            .collect()
    }

    fn visible_tracks(&self) -> Vec<(usize, &'a TrackRow)> {
        self.tracks
            .iter()
            .enumerate()
            .filter(|(_, track)| {
                self.text_filter
                    .as_deref()
                    .is_none_or(|filter| track_row_matches_text_filter(track, filter))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_width_eq(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < f32::EPSILON);
    }

    fn row() -> TrackRow {
        TrackRow {
            id: 0,
            feed_id: 0,
            feed_guid: None,
            item_guid: String::new(),
            track_title: None,
            artist_name: None,
            album_title: None,
            album_artist_name: None,
            track_number: None,
            disc_number: None,
            duration_seconds: None,
            enclosure_url: None,
            enclosure_type: None,
            track_image_href: None,
            is_in_library: false,
            feed_title: None,
            album_image_href: None,
            local_path: None,
            pub_date: None,
            explicit: None,
            transcript_url: None,
        }
    }

    fn content_row(id: &str, source: ContentListRowSource) -> ContentListRowDisplay {
        ContentListRowDisplay::new(id, format!("{id} title"), format!("{id} secondary"), source)
    }

    #[test]
    fn content_list_page_vm_defaults_to_all_filter() {
        let page = ContentListPageVm::new(vec![
            content_row("library", ContentListRowSource::Library),
            content_row("index", ContentListRowSource::Index),
        ]);

        assert_eq!(
            page.filter(),
            ContentFilter::All,
            "content list pages should default to the all-content filter"
        );
        assert_eq!(
            page.cached_rows().len(),
            2,
            "cached rows should preserve the unfiltered row set"
        );
        assert_eq!(
            page.visible_rows()
                .iter()
                .map(|row| row.id.as_str())
                .collect::<Vec<_>>(),
            ["library", "index"],
            "all filter should show library and index rows"
        );
        assert_eq!(
            page.empty_state(),
            None,
            "all filter should not show an empty state when rows exist"
        );
    }

    #[test]
    fn content_list_page_vm_set_filter_updates_visible_rows() {
        let mut page = ContentListPageVm::new(vec![
            content_row("library", ContentListRowSource::Library),
            content_row("index", ContentListRowSource::Index),
        ]);

        page.set_filter(ContentFilter::Library);
        assert_eq!(
            page.visible_rows()
                .iter()
                .map(|row| row.id.as_str())
                .collect::<Vec<_>>(),
            ["library"],
            "library filter should show only library-sourced rows"
        );

        page.set_filter(ContentFilter::Index);
        assert_eq!(
            page.visible_rows()
                .iter()
                .map(|row| row.id.as_str())
                .collect::<Vec<_>>(),
            ["index"],
            "index filter should show only index-sourced rows"
        );
    }

    #[test]
    fn content_list_page_vm_set_text_filter_updates_visible_rows() {
        let mut page = ContentListPageVm::new(vec![
            ContentListRowDisplay::new(
                "library",
                "Local Mix",
                "Aphex Twin",
                ContentListRowSource::Library,
            ),
            ContentListRowDisplay::new(
                "index",
                "Remote Feed",
                "Detroit Techno",
                ContentListRowSource::Index,
            ),
        ]);

        page.set_text_filter(Some("  techno  ".to_string()));

        assert_eq!(page.text_filter(), Some("techno"));
        assert_eq!(
            page.visible_row_ids(),
            ["index"],
            "text filter should match display-owned secondary row text"
        );

        page.set_text_filter(Some("   ".to_string()));

        assert_eq!(page.text_filter(), None);
        assert_eq!(
            page.visible_row_ids(),
            ["library", "index"],
            "whitespace-only text filter should clear filtering"
        );
    }

    #[test]
    fn content_list_page_vm_text_filter_composes_with_source_filter() {
        let mut page = ContentListPageVm::new(vec![
            ContentListRowDisplay::new(
                "library",
                "Shared Title",
                "Local Artist",
                ContentListRowSource::Library,
            ),
            ContentListRowDisplay::new(
                "index",
                "Shared Title",
                "Remote Artist",
                ContentListRowSource::Index,
            ),
        ]);

        page.set_filter(ContentFilter::Library);
        page.set_text_filter(Some("shared".to_string()));

        assert_eq!(
            page.visible_row_ids(),
            ["library"],
            "content source filtering should remain active with text filtering"
        );

        page.set_text_filter(Some("remote".to_string()));

        assert!(
            page.visible_rows().is_empty(),
            "library source filter should hide matching index text"
        );
        assert_eq!(
            page.empty_state(),
            Some(ContentListEmptyStateDisplay {
                title: "No library content",
                secondary: "No local rows match this frame filter.",
                clear_filter_action_id: Some("content-list.clear-filter"),
            }),
            "empty state should still reflect the active source filter"
        );
    }

    #[test]
    fn content_list_page_vm_replace_rows_preserves_filter() {
        let mut page =
            ContentListPageVm::new(vec![content_row("library", ContentListRowSource::Library)]);
        page.set_filter(ContentFilter::Index);
        page.set_text_filter(Some("next".to_string()));

        page.replace_rows(vec![
            content_row("next-library", ContentListRowSource::Library),
            content_row("next-index", ContentListRowSource::Index),
        ]);

        assert_eq!(page.filter(), ContentFilter::Index);
        assert_eq!(page.text_filter(), Some("next"));
        assert_eq!(
            page.visible_row_ids(),
            ["next-index"],
            "row refresh should preserve frame-local source and text filters"
        );
    }

    #[test]
    fn content_list_page_vm_projects_track_rows_from_membership_source() {
        let mut local = row();
        local.id = 1;
        local.track_title = Some("Local Track".into());
        local.artist_name = Some("Local Artist".into());
        local.is_in_library = true;
        let mut remote = row();
        remote.id = 2;
        remote.feed_title = Some("Remote Feed".into());
        remote.album_title = Some("Remote Album".into());

        let tracks = vec![local, remote];
        let mut page = ContentListPageVm::from_tracks(&tracks);

        page.set_filter(ContentFilter::Library);
        assert_eq!(
            page.visible_rows().into_iter().cloned().collect::<Vec<_>>(),
            vec![ContentListRowDisplay::new(
                "1",
                "Local Track",
                "Local Artist",
                ContentListRowSource::Library,
            )],
            "TrackRow.is_in_library should project local rows as library content"
        );

        page.set_filter(ContentFilter::Index);
        assert_eq!(
            page.visible_rows().into_iter().cloned().collect::<Vec<_>>(),
            vec![ContentListRowDisplay::new(
                "2",
                "Remote Feed",
                "Remote Album",
                ContentListRowSource::Index,
            )],
            "non-library TrackRow values should project as index content"
        );
    }

    #[test]
    fn content_list_page_vm_projects_empty_filter_state() {
        let mut page =
            ContentListPageVm::new(vec![content_row("library", ContentListRowSource::Library)]);

        page.set_filter(ContentFilter::Index);

        assert_eq!(
            page.empty_state(),
            Some(ContentListEmptyStateDisplay {
                title: "No index content",
                secondary: "No index rows match this frame filter.",
                clear_filter_action_id: Some("content-list.clear-filter"),
            }),
            "filtering away every row should surface an empty-filter state"
        );
    }

    #[test]
    fn content_list_page_vm_filter_chip_strip_preserves_selected_filter() {
        let mut page = ContentListPageVm::new(Vec::new());

        page.set_filter(ContentFilter::Library);
        let strip = page.filter_chip_strip();

        assert_eq!(
            strip.selected,
            ContentFilter::Library,
            "content-list chip strip should reflect the page-local selected filter"
        );
        assert!(
            strip.narrow_collapse_to_pulldown,
            "content-list chip strip should opt into narrow pull-down collapse"
        );
    }

    #[test]
    fn number_prefix_renders_only_when_present() {
        assert_eq!(LibraryTrackRowVm::new(&row(), None).number_prefix(), "");
        let mut r = row();
        r.track_number = Some(7);
        assert_eq!(LibraryTrackRowVm::new(&r, None).number_prefix(), "7. ");
    }

    #[test]
    fn tree_number_prefix_preserves_legacy_zero_padded_display() {
        assert_eq!(
            LibraryTrackRowVm::new(&row(), None).tree_number_prefix(),
            ""
        );

        let mut r = row();
        r.track_number = Some(0);
        assert_eq!(
            LibraryTrackRowVm::new(&r, None).tree_number_prefix(),
            "00 - "
        );

        r.track_number = Some(7);
        assert_eq!(
            LibraryTrackRowVm::new(&r, None).tree_number_prefix(),
            "07 - "
        );
    }

    #[test]
    fn duration_suffix_pads_seconds_below_ten() {
        let mut r = row();
        r.duration_seconds = Some(65);
        assert_eq!(
            LibraryTrackRowVm::new(&r, None).duration_suffix(),
            "  (1:05)"
        );
    }

    #[test]
    fn duration_suffix_is_empty_when_absent() {
        assert_eq!(LibraryTrackRowVm::new(&row(), None).duration_suffix(), "");
    }

    #[test]
    fn description_state_projects_auto_threshold_boundary() {
        assert_eq!(
            DescriptionState::project(DESCRIPTION_AUTO_COLLAPSE_LINES),
            DescriptionState::AutoExpanded
        );
        assert!(DescriptionState::project(DESCRIPTION_AUTO_COLLAPSE_LINES).is_visible());
        assert_eq!(
            DescriptionState::project(DESCRIPTION_AUTO_COLLAPSE_LINES + 1),
            DescriptionState::AutoCollapsed
        );
        assert!(!DescriptionState::project(DESCRIPTION_AUTO_COLLAPSE_LINES + 1).is_visible());
    }

    #[test]
    fn display_description_text_trims_empty_without_inference() {
        assert_eq!(display_description_text(None), None);
        assert_eq!(display_description_text(Some("   ")), None);
        assert_eq!(display_description_text(Some("...")), Some("..."));
        assert_eq!(
            display_description_text(Some("  real description  ")),
            Some("real description")
        );
        assert_eq!(description_line_count(Some("...")), 1);
    }

    #[test]
    fn description_state_toggle_moves_to_user_variants() {
        assert_eq!(
            DescriptionState::AutoCollapsed.toggle(),
            DescriptionState::UserExpanded
        );
        assert_eq!(
            DescriptionState::AutoExpanded.toggle(),
            DescriptionState::UserCollapsed
        );
        assert_eq!(
            DescriptionState::UserCollapsed.toggle(),
            DescriptionState::UserExpanded
        );
        assert_eq!(
            DescriptionState::UserExpanded.toggle(),
            DescriptionState::UserCollapsed
        );
    }

    #[test]
    fn description_state_user_variants_are_sticky_on_reproject() {
        assert_eq!(
            DescriptionState::UserCollapsed.project_sticky(1),
            DescriptionState::UserCollapsed
        );
        assert_eq!(
            DescriptionState::UserExpanded.project_sticky(20),
            DescriptionState::UserExpanded
        );
        assert_eq!(
            DescriptionState::AutoExpanded.project_sticky(6),
            DescriptionState::AutoCollapsed
        );
    }

    #[test]
    fn track_inspector_state_starts_with_no_expanded_panels() {
        let state = LibraryTrackInspectorState::default();

        assert!(state.inspector_expanded_panels.is_empty());
        assert_eq!(state.description_state, DescriptionState::AutoExpanded);
        assert!(!state.is_panel_expanded(InspectorPanelKind::CompareId3));
        assert!(!state.is_panel_expanded(InspectorPanelKind::MusicBrainz));
    }

    #[test]
    fn track_inspector_state_expands_collapses_and_toggles_panels() {
        let mut state = LibraryTrackInspectorState::new(6);

        assert_eq!(state.description_state, DescriptionState::AutoCollapsed);
        state.expand_panel(InspectorPanelKind::CompareId3);
        assert!(state.is_panel_expanded(InspectorPanelKind::CompareId3));
        state.collapse_panel(InspectorPanelKind::CompareId3);
        assert!(!state.is_panel_expanded(InspectorPanelKind::CompareId3));

        state.toggle_panel(InspectorPanelKind::MusicBrainz);
        assert!(state.is_panel_expanded(InspectorPanelKind::MusicBrainz));
        state.toggle_panel(InspectorPanelKind::MusicBrainz);
        assert!(!state.is_panel_expanded(InspectorPanelKind::MusicBrainz));

        state.project_description(1);
        assert_eq!(state.description_state, DescriptionState::AutoExpanded);
    }

    #[test]
    fn track_inspector_predicates_follow_download_state() {
        assert!(!compare_id3_enabled(false));
        assert!(!musicbrainz_enabled(false));
        assert!(compare_id3_enabled(true));
        assert!(musicbrainz_enabled(true));

        let mut state = LibraryTrackInspectorState::default();
        state.expand_panel(InspectorPanelKind::CompareId3);
        state.toggle_description();
        let display = state.display(false);

        assert_eq!(
            display.inspector_expanded_panels,
            [InspectorPanelKind::CompareId3].into()
        );
        assert_eq!(display.description_state, DescriptionState::UserCollapsed);
        assert!(!display.compare_id3_enabled);
        assert!(!display.musicbrainz_enabled);
        assert!(!display.show_compare_id3_panel());
        assert!(!display.show_musicbrainz_panel());
        assert_eq!(
            display.compare_id3_tooltip_text(),
            Some(DOWNLOAD_REQUIRED_METADATA_TOOLTIP)
        );
        assert_eq!(
            display.musicbrainz_tooltip_text(),
            Some(DOWNLOAD_REQUIRED_METADATA_TOOLTIP)
        );

        let downloaded = state.display(true);
        assert!(downloaded.show_compare_id3_panel());
        assert_eq!(downloaded.compare_id3_tooltip_text(), None);
    }

    #[test]
    fn library_track_row_vm_owns_title_artist_and_album_fallbacks() {
        let mut r = row();
        let vm = LibraryTrackRowVm::new(&r, None);
        assert_eq!(vm.display_title(), "Untitled");
        assert_eq!(vm.compact_title(), "[untitled]");
        assert_eq!(vm.display_artist(), "Unknown Artist");
        assert_eq!(vm.display_album(), "Unknown Album");

        r.feed_title = Some("Feed Title".into());
        let vm = LibraryTrackRowVm::new(&r, None);
        assert_eq!(vm.display_title(), "Feed Title");
        assert_eq!(vm.display_album(), "Feed Title");

        r.track_title = Some("Track Title".into());
        r.artist_name = Some("Artist".into());
        r.album_artist_name = Some("Album Artist".into());
        r.album_title = Some("Album".into());
        let vm = LibraryTrackRowVm::new(&r, None);
        assert_eq!(vm.display_title(), "Track Title");
        assert_eq!(vm.compact_title(), "Track Title");
        assert_eq!(vm.display_artist(), "Album Artist");
        assert_eq!(vm.display_album(), "Album");
    }

    #[test]
    fn library_track_row_vm_primary_action_follows_membership_and_busy_state() {
        let mut r = row();
        let vm = LibraryTrackRowVm::new(&r, None);
        let action = vm.primary_action_vm(false);
        assert_eq!(
            action.kind,
            crate::view_models::entity_detail::EntityActionKind::Download
        );
        assert_eq!(action.label, "Download");
        assert!(action.enabled);

        let action = vm.primary_action_vm(true);
        assert_eq!(action.label, "Downloading...");
        assert!(!action.enabled);

        r.is_in_library = true;
        let vm = LibraryTrackRowVm::new(&r, None);
        let action = vm.primary_action_vm(false);
        assert_eq!(
            action.kind,
            crate::view_models::entity_detail::EntityActionKind::Remove
        );
        assert_eq!(action.label, "Remove");
        assert!(action.enabled);

        let action = vm.primary_action_vm(true);
        assert_eq!(action.label, "Removing...");
        assert!(!action.enabled);
    }

    #[test]
    fn library_track_row_vm_playlist_display_projects_album_track_controls() {
        let mut r = row();
        r.id = 42;

        assert_eq!(
            LibraryTrackRowVm::new(&r, None).playlist_display(),
            LibraryTrackPlaylistDisplay {
                popover_id: "album-track-add:42".into(),
                trigger_label: "+ Playlist",
            }
        );
    }

    #[test]
    fn library_track_row_vm_projects_row_and_toggle_ids() {
        let mut r = row();
        r.id = 42;

        assert_eq!(
            LibraryTrackRowVm::new(&r, None).row_display(),
            LibraryTrackRowDisplay {
                row_id: "album-track-42".into(),
                toggle_button_id: "lib-toggle-42".into(),
            }
        );
    }

    #[test]
    fn library_track_row_vm_projects_shared_primary_action() {
        let mut r = row();
        r.id = 42;
        let vm = LibraryTrackRowVm::new(&r, None);
        let action = vm.primary_action_vm(false);
        assert_eq!(
            action.target,
            EntityActionTarget::Track(TrackRef::LocalTrackId(42))
        );
        assert_eq!(
            action.kind,
            crate::view_models::entity_detail::EntityActionKind::Download
        );
        assert_eq!(
            action.tone,
            crate::view_models::entity_detail::EntityActionTone::Secondary
        );

        r.is_in_library = true;
        let vm = LibraryTrackRowVm::new(&r, None);
        let action = vm.primary_action_vm(false);
        assert_eq!(
            action.kind,
            crate::view_models::entity_detail::EntityActionKind::Remove
        );
        assert_eq!(
            action.tone,
            crate::view_models::entity_detail::EntityActionTone::DestructiveQuiet
        );
        assert!(action.enabled);

        let action = vm.primary_action_vm(true);
        assert_eq!(action.label, "Removing...");
        assert!(!action.enabled);
    }

    #[test]
    fn library_track_row_vm_local_path_does_not_change_row_action_text() {
        let mut r = row();
        r.local_path = Some("/music/track.mp3".into());
        let vm = LibraryTrackRowVm::new(&r, None);
        assert_eq!(vm.primary_action_vm(false).label, "Download");
    }

    #[test]
    fn mb_status_text_distinguishes_done_zero_and_done_nonzero() {
        let r = row();
        let pending = MbTrackStatus::Pending;
        let processing = MbTrackStatus::Processing;
        let done_zero = MbTrackStatus::Done(0);
        let done_some = MbTrackStatus::Done(2);
        let skipped = MbTrackStatus::Skipped("bad".into());

        assert_eq!(LibraryTrackRowVm::new(&r, None).mb_status_text(), None);
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&pending)).mb_status_text(),
            Some("MB: pending")
        );
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&processing)).mb_status_text(),
            Some("MB: looking up...")
        );
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&done_zero)).mb_status_text(),
            Some("MB: no missing fields")
        );
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&done_some)).mb_status_text(),
            Some("MB: done")
        );
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&skipped)).mb_status_text(),
            Some("MB: skipped")
        );
    }

    #[test]
    fn mb_status_kind_routes_done_zero_to_muted_not_success() {
        let r = row();
        let done_zero = MbTrackStatus::Done(0);
        let done_some = MbTrackStatus::Done(3);
        let processing = MbTrackStatus::Processing;
        let skipped = MbTrackStatus::Skipped("nope".into());
        let pending = MbTrackStatus::Pending;

        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&done_zero)).mb_status_kind(),
            Some(MbStatusKind::Muted)
        );
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&done_some)).mb_status_kind(),
            Some(MbStatusKind::Success)
        );
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&processing)).mb_status_kind(),
            Some(MbStatusKind::Warning)
        );
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&skipped)).mb_status_kind(),
            Some(MbStatusKind::Danger)
        );
        assert_eq!(
            LibraryTrackRowVm::new(&r, Some(&pending)).mb_status_kind(),
            Some(MbStatusKind::Muted)
        );
        assert_eq!(LibraryTrackRowVm::new(&r, None).mb_status_kind(), None);
    }

    fn track_for_feed(feed_id: i64, feed_title: Option<&str>) -> TrackRow {
        let mut r = row();
        r.feed_id = feed_id;
        r.feed_title = feed_title.map(str::to_string);
        r
    }

    #[test]
    fn artist_detail_vm_falls_back_to_unknown_for_empty_name() {
        let vm = LibraryArtistDetailVm::new("", &[]);
        assert_eq!(vm.artist_name_or_unknown(), "Unknown");
        let vm = LibraryArtistDetailVm::new("Aphex", &[]);
        assert_eq!(vm.artist_name_or_unknown(), "Aphex");
    }

    #[test]
    fn artist_detail_vm_counts_distinct_feeds_as_albums() {
        let tracks = vec![
            track_for_feed(1, Some("A")),
            track_for_feed(1, Some("A")),
            track_for_feed(2, Some("B")),
        ];
        let vm = LibraryArtistDetailVm::new("Artist", &tracks);
        assert_eq!(vm.album_count(), 2);
        assert_eq!(vm.track_count(), 3);
    }

    #[test]
    fn artist_detail_vm_omits_downloaded_row_when_zero() {
        let tracks = vec![track_for_feed(1, Some("A"))];
        let vm = LibraryArtistDetailVm::new("Artist", &tracks);
        let rows = vm.detail_rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], ("Albums".into(), "1".into()));
        assert_eq!(rows[1], ("Tracks".into(), "1 track".into()));
    }

    #[test]
    fn artist_detail_vm_pluralises_track_count_above_one() {
        let tracks = [track_for_feed(1, Some("A")), track_for_feed(1, Some("A"))];
        let vm = LibraryArtistDetailVm::new("Artist", &tracks);
        let rows = vm.detail_rows();
        assert_eq!(rows[1], ("Tracks".into(), "2 tracks".into()));
    }

    #[test]
    fn artist_detail_vm_includes_downloaded_row_when_any_local_path_present() {
        let mut t1 = track_for_feed(1, Some("A"));
        t1.local_path = Some("/x".into());
        let t2 = track_for_feed(1, Some("A"));
        let tracks = [t1, t2];
        let vm = LibraryArtistDetailVm::new("Artist", &tracks);
        let rows = vm.detail_rows();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[2], ("Downloaded".into(), "1".into()));
    }

    #[test]
    fn artist_detail_vm_projects_single_bound_artist_source_facts() {
        let tracks = [track_for_feed(1, Some("A"))];
        let view = ArtistView {
            sort_name: Some("Artist, The".into()),
            area: Some("Montreal".into()),
            begin_year: Some(2020),
            url: Some("https://example.test/artist".into()),
            aliases: vec!["A. Example".into()],
            source_subjects: vec![crate::views::ArtistSourceSubjectView {
                source: "musicindex".into(),
                source_artist_id: "artist-123".into(),
                name: Some("Remote Artist".into()),
            }],
            ..ArtistView::default()
        };
        let vm = LibraryArtistDetailVm::with_view("Artist", &view, &tracks);
        let rows = vm.detail_rows();

        assert!(rows.contains(&("Sort Name".into(), "Artist, The".into())));
        assert!(rows.contains(&("Area".into(), "Montreal".into())));
        assert!(rows.contains(&("Active".into(), "2020-".into())));
        assert!(rows.contains(&("Website".into(), "https://example.test/artist".into())));
        assert!(rows.contains(&("Aliases".into(), "A. Example".into())));
    }

    #[test]
    fn artist_detail_vm_surfaces_multi_subject_state_without_merging() {
        let tracks = [track_for_feed(1, Some("A"))];
        let view = ArtistView {
            sort_name: Some("Should Not Render".into()),
            source_subjects: vec![
                crate::views::ArtistSourceSubjectView {
                    source: "musicindex".into(),
                    source_artist_id: "artist-123".into(),
                    name: Some("Remote One".into()),
                },
                crate::views::ArtistSourceSubjectView {
                    source: "musicindex".into(),
                    source_artist_id: "artist-456".into(),
                    name: Some("Remote Two".into()),
                },
            ],
            ..ArtistView::default()
        };
        let vm = LibraryArtistDetailVm::with_view("Artist", &view, &tracks);
        let rows = vm.detail_rows();

        assert!(rows.contains(&("Source Subjects".into(), "2 explicit subjects".into())));
        assert!(
            !rows.iter().any(|(key, _)| key == "Sort Name"),
            "multi-subject views should not project one subject's scalar facts"
        );
    }

    #[test]
    fn artist_detail_page_projects_library_header_rows_and_feed_policy() {
        let tracks = [track_for_feed(1, Some("A"))];
        let vm = LibraryArtistDetailVm::new("", &tracks);
        let page = vm.page();

        assert_eq!(page.title, "Unknown");
        assert_eq!(page.subtitle, None);
        assert!(page.shows_feed_section);
        assert_eq!(page.detail_rows[0].key, "Albums");
        assert_eq!(page.detail_rows[0].value, "1");
        assert_eq!(page.detail_rows[0].max_lines, 6);
    }

    #[test]
    fn artist_detail_vm_feed_summaries_apply_untitled_fallback_and_track_counts() {
        let mut t1 = track_for_feed(1, None);
        t1.album_image_href = Some("img-1".into());
        let t2 = track_for_feed(1, None);
        let t3 = track_for_feed(2, Some("Real"));
        let tracks = [t1, t2, t3];
        let vm = LibraryArtistDetailVm::new("Artist", &tracks);
        let summaries = vm.feed_summaries();
        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].feed_id, 1);
        assert_eq!(summaries[0].feed_name, "Untitled Feed");
        assert_eq!(summaries[0].thumb_url.as_deref(), Some("img-1"));
        assert_eq!(summaries[0].track_count, 2);
        assert_eq!(summaries[1].feed_id, 2);
        assert_eq!(summaries[1].feed_name, "Real");
        assert_eq!(summaries[1].track_count, 1);
    }

    #[test]
    fn artist_feed_summary_display_projects_row_id_and_track_count() {
        let mut t1 = track_for_feed(1, Some("Real"));
        let t2 = track_for_feed(1, Some("Real"));
        let tracks = [t1.clone(), t2];
        let vm = LibraryArtistDetailVm::new("Artist", &tracks);
        let summaries = vm.feed_summaries();

        let display = summaries[0].display();

        assert_eq!(display.element_id, "artist-feed-Real");
        assert_eq!(display.title, "Real");
        assert_eq!(display.track_count_label, "2 tracks");

        t1.feed_id = 2;
        t1.feed_title = Some("Single".into());
        t1.album_image_href = Some("single-img".into());
        let tracks = [t1];
        let vm = LibraryArtistDetailVm::new("Artist", &tracks);
        let display = vm.feed_summaries()[0].display();
        assert_eq!(display.title, "Single");
        assert_eq!(display.thumb_url.as_deref(), Some("single-img"));
        assert_eq!(display.track_count_label, "1 track");
    }

    #[test]
    fn artist_detail_vm_thumb_url_falls_back_to_track_image_href() {
        let mut t = track_for_feed(1, Some("A"));
        t.track_image_href = Some("track-img".into());
        let tracks = [t];
        let vm = LibraryArtistDetailVm::new("Artist", &tracks);
        let summaries = vm.feed_summaries();
        assert_eq!(summaries[0].thumb_url.as_deref(), Some("track-img"));
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
    fn playlist_detail_vm_reports_empty_state() {
        let pl = playlist("Mix");
        let vm = PlaylistDetailVm::new(&pl, &[]);
        assert!(vm.is_empty());
        assert_eq!(vm.track_count(), 0);
        assert_eq!(vm.total_duration_seconds(), 0);
        assert_eq!(vm.total_duration_label(), None);
        assert_eq!(vm.detail_rows(), vec![("Tracks".into(), "0".into())]);
        assert_eq!(
            vm.header_display(),
            PlaylistDetailHeaderDisplay {
                title: "Mix".into()
            }
        );
    }

    #[test]
    fn playlist_detail_vm_text_filter_preserves_original_positions() {
        let pl = playlist("Mix");
        let mut first = row();
        first.id = 1;
        first.track_title = Some("Opening".into());
        first.artist_name = Some("Alice".into());
        first.duration_seconds = Some(60);
        let mut second = row();
        second.id = 2;
        second.track_title = Some("Middle".into());
        second.artist_name = Some("Bob".into());
        second.duration_seconds = Some(120);
        let mut third = row();
        third.id = 3;
        third.track_title = Some("Finale".into());
        third.artist_name = Some("Alice".into());
        third.duration_seconds = Some(180);
        let tracks = vec![first, second, third];
        let mut vm = PlaylistDetailVm::new(&pl, &tracks);

        vm.set_text_filter(Some("alice".to_string()));

        let rows = vm.track_rows();
        assert_eq!(vm.text_filter(), Some("alice"));
        assert_eq!(vm.track_count(), 2);
        assert_eq!(vm.total_duration_seconds(), 240);
        assert_eq!(
            rows.iter()
                .map(PlaylistTrackRowVm::position)
                .collect::<Vec<_>>(),
            [0, 2],
            "filtered playlist rows should keep original playlist positions for commands"
        );

        vm.set_text_filter(Some("   ".to_string()));

        assert_eq!(vm.text_filter(), None);
        assert_eq!(vm.track_count(), 3);
        assert_eq!(vm.track_rows().len(), 3);
    }

    #[test]
    fn playlist_detail_vm_projects_rename_and_delete_controls() {
        let mut pl = playlist("Mix");
        pl.id = 42;
        let vm = PlaylistDetailVm::new(&pl, &[]);

        assert_eq!(
            vm.actions_display(),
            PlaylistDetailActionsDisplay {
                rename_button_id: "playlist-rename-42".into(),
                rename_label: "Rename",
                rename_a11y_label: "Rename playlist",
                rename_input_id: "playlist-rename-input-42".into(),
                rename_input_placeholder: "Playlist name",
                rename_save_button_id: "playlist-rename-save-42".into(),
                rename_save_label: "Save",
                rename_save_a11y_label: "Save playlist name",
                rename_cancel_button_id: "playlist-rename-cancel-42".into(),
                rename_cancel_label: "Cancel",
                rename_cancel_a11y_label: "Cancel playlist rename",
                delete_button_id: "playlist-delete-42".into(),
                delete_label: "Delete",
                delete_a11y_label: "Delete playlist",
            }
        );
    }

    #[test]
    fn playlist_detail_vm_total_duration_uses_minutes_below_an_hour() {
        let pl = playlist("Mix");
        let mut t1 = row();
        t1.duration_seconds = Some(125); // 2:05
        let mut t2 = row();
        t2.duration_seconds = Some(180); // 3:00
        let tracks = [t1, t2];
        let vm = PlaylistDetailVm::new(&pl, &tracks);
        assert_eq!(vm.total_duration_seconds(), 305);
        assert_eq!(vm.total_duration_label().as_deref(), Some("5:05"));
    }

    #[test]
    fn playlist_detail_vm_total_duration_switches_to_hours_after_60_minutes() {
        let pl = playlist("Mix");
        let mut t = row();
        // 1h 23m == 4980 sec
        t.duration_seconds = Some(4980);
        let tracks = [t];
        let vm = PlaylistDetailVm::new(&pl, &tracks);
        assert_eq!(vm.total_duration_label().as_deref(), Some("1h 23m"));
    }

    #[test]
    fn playlist_detail_vm_total_duration_is_none_when_no_track_has_seconds() {
        let pl = playlist("Mix");
        let tracks = [row(), row()];
        let vm = PlaylistDetailVm::new(&pl, &tracks);
        assert_eq!(vm.total_duration_label(), None);
        assert_eq!(vm.detail_rows().len(), 1);
    }

    #[test]
    fn playlist_detail_vm_detail_rows_include_duration_when_known() {
        let pl = playlist("Mix");
        let mut t = row();
        t.duration_seconds = Some(60);
        let tracks = [t];
        let vm = PlaylistDetailVm::new(&pl, &tracks);
        let rows = vm.detail_rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], ("Tracks".into(), "1".into()));
        assert_eq!(rows[1], ("Duration".into(), "1:00".into()));
    }

    #[test]
    fn playlist_detail_vm_detail_rows_omit_sentinel_dates_and_blank_description() {
        let mut pl = playlist("Mix");
        pl.created_at = 0;
        pl.updated_at = -1;
        pl.description = Some("   ".into());
        let vm = PlaylistDetailVm::new(&pl, &[]);

        assert_eq!(vm.detail_rows(), vec![("Tracks".into(), "0".into())]);
    }

    #[test]
    fn playlist_detail_vm_detail_rows_include_formatted_local_metadata() {
        let mut pl = playlist("Mix");
        pl.created_at = 1_712_275_200;
        pl.updated_at = 1_715_040_000;
        pl.description = Some("  Weekend listening  ".into());
        let vm = PlaylistDetailVm::new(&pl, &[]);

        assert_eq!(
            vm.detail_rows(),
            vec![
                ("Tracks".into(), "0".into()),
                ("Created".into(), "Apr 5, 2024".into()),
                ("Modified".into(), "May 7, 2024".into()),
                ("Description".into(), "Weekend listening".into()),
            ]
        );
    }

    #[test]
    fn playlist_track_row_vm_applies_title_and_artist_fallbacks() {
        let pl = playlist("Mix");
        let tracks = [row()];
        let vm = PlaylistDetailVm::new(&pl, &tracks);
        let rows = vm.track_rows();
        assert_eq!(rows[0].title(), "[untitled]");
        assert_eq!(rows[0].artist(), "Unknown");
        assert_eq!(rows[0].duration_label(), "");
        assert_eq!(rows[0].position_label(), "1.");
    }

    #[test]
    fn playlist_track_row_vm_can_play_follows_local_path() {
        let pl = playlist("Mix");
        let mut t = row();
        t.is_in_library = true;
        t.local_path = Some("/x".into());
        let tracks = [t, row()];
        let vm = PlaylistDetailVm::new(&pl, &tracks);
        let rows = vm.track_rows();
        assert!(rows[0].can_play());
        assert!(!rows[1].can_play());
    }

    #[test]
    fn playlist_track_row_vm_move_enable_rules_at_boundaries() {
        let pl = playlist("Mix");
        let tracks = [row(), row(), row()];
        let vm = PlaylistDetailVm::new(&pl, &tracks);
        let rows = vm.track_rows();
        assert!(!rows[0].can_move_up());
        assert!(rows[0].can_move_down());
        assert!(rows[1].can_move_up());
        assert!(rows[1].can_move_down());
        assert!(rows[2].can_move_up());
        assert!(!rows[2].can_move_down());
    }

    #[test]
    fn playlist_track_row_vm_reorder_menu_availability_tracks_boundaries() {
        let pl = playlist("Mix");
        let tracks = [row(), row(), row()];
        let vm = PlaylistDetailVm::new(&pl, &tracks);
        let rows = vm.track_rows();

        let first = rows[0].controls_display(pl.id);
        assert!(first.move_up_menu_item.disabled);
        assert!(!first.move_down_menu_item.disabled);

        let middle = rows[1].controls_display(pl.id);
        assert!(!middle.move_up_menu_item.disabled);
        assert!(!middle.move_down_menu_item.disabled);

        let last = rows[2].controls_display(pl.id);
        assert!(!last.move_up_menu_item.disabled);
        assert!(last.move_down_menu_item.disabled);
    }

    #[test]
    fn playlist_track_row_vm_controls_display_projects_ids_labels_and_availability() {
        let pl = playlist("Mix");
        let mut t1 = row();
        t1.id = 42;
        t1.is_in_library = true;
        t1.local_path = Some("/x".into());
        let mut t2 = row();
        t2.id = 43;
        let tracks = [t1, t2];
        let vm = PlaylistDetailVm::new(&pl, &tracks);
        let rows = vm.track_rows();

        assert_eq!(
            rows[0].controls_display(7),
            PlaylistTrackControlsDisplay {
                row_id: "playlist-track-42-0".into(),
                row_body_id: "playlist-row-body-7-0".into(),
                drag_handle_id: "playlist-drag-handle-7-0".into(),
                drag_handle_a11y_label: "Drag to reorder playlist track",
                actions_menu_id: "playlist-actions-7-0".into(),
                actions_menu_a11y_label: "Playlist track actions",
                play_button_id: "playlist-play-7-0".into(),
                play_label: "▶",
                play_enabled: true,
                move_up_menu_item: PlaylistTrackMenuItemDisplay {
                    id: "playlist-move-up-7-0".into(),
                    label: "Move Up",
                    a11y_label: "Move track up",
                    destructive: false,
                    disabled: true,
                },
                move_down_menu_item: PlaylistTrackMenuItemDisplay {
                    id: "playlist-move-down-7-0".into(),
                    label: "Move Down",
                    a11y_label: "Move track down",
                    destructive: false,
                    disabled: false,
                },
                remove_menu_item: PlaylistTrackMenuItemDisplay {
                    id: "playlist-remove-7-0".into(),
                    label: "Remove",
                    a11y_label: "Remove track from playlist",
                    destructive: true,
                    disabled: false,
                },
            }
        );

        let second = rows[1].controls_display(7);
        assert!(!second.play_enabled);
        assert!(!second.move_up_menu_item.disabled);
        assert!(second.move_down_menu_item.disabled);
    }

    #[test]
    fn playlist_track_row_vm_display_projects_row_text_media_and_controls() {
        let mut pl = playlist("Mix");
        pl.id = 7;
        let mut t = row();
        t.id = 42;
        t.is_in_library = true;
        t.track_title = Some("Song".into());
        t.artist_name = Some("Artist".into());
        t.duration_seconds = Some(125);
        t.track_image_href = Some("track".into());
        t.local_path = Some("/x".into());
        let tracks = [t, row()];
        let vm = PlaylistDetailVm::new(&pl, &tracks);
        let display = vm.track_rows()[0].display(pl.id);

        assert_eq!(
            display,
            PlaylistTrackRowDisplay {
                is_available: true,
                position: 0,
                position_label: "1.".into(),
                title: "Song".into(),
                artist: "Artist".into(),
                availability_label: None,
                duration_label: "2:05".into(),
                thumb_url: Some("track".into()),
                controls: PlaylistTrackControlsDisplay {
                    row_id: "playlist-track-42-0".into(),
                    row_body_id: "playlist-row-body-7-0".into(),
                    drag_handle_id: "playlist-drag-handle-7-0".into(),
                    drag_handle_a11y_label: "Drag to reorder playlist track",
                    actions_menu_id: "playlist-actions-7-0".into(),
                    actions_menu_a11y_label: "Playlist track actions",
                    play_button_id: "playlist-play-7-0".into(),
                    play_label: "▶",
                    play_enabled: true,
                    move_up_menu_item: PlaylistTrackMenuItemDisplay {
                        id: "playlist-move-up-7-0".into(),
                        label: "Move Up",
                        a11y_label: "Move track up",
                        destructive: false,
                        disabled: true,
                    },
                    move_down_menu_item: PlaylistTrackMenuItemDisplay {
                        id: "playlist-move-down-7-0".into(),
                        label: "Move Down",
                        a11y_label: "Move track down",
                        destructive: false,
                        disabled: false,
                    },
                    remove_menu_item: PlaylistTrackMenuItemDisplay {
                        id: "playlist-remove-7-0".into(),
                        label: "Remove",
                        a11y_label: "Remove track from playlist",
                        destructive: true,
                        disabled: false,
                    },
                },
            }
        );
    }

    #[test]
    fn playlist_track_row_vm_marks_out_of_library_rows_unavailable() {
        let pl = playlist("Mix");
        let mut t = row();
        t.id = 42;
        t.local_path = Some("/x".into());
        let tracks = [t];
        let vm = PlaylistDetailVm::new(&pl, &tracks);
        let row = &vm.track_rows()[0];
        let display = row.display(pl.id);

        assert!(!row.is_available());
        assert!(!row.can_play());
        assert_eq!(row.availability_label(), Some("Unavailable"));
        assert!(!display.is_available);
        assert_eq!(display.availability_label, Some("Unavailable"));
        assert!(!display.controls.play_enabled);
    }

    #[test]
    fn playlist_track_row_vm_marks_missing_local_files_unavailable() {
        let pl = playlist("Mix");
        let mut t = row();
        t.id = 42;
        t.is_in_library = true;
        let tracks = [t];
        let vm = PlaylistDetailVm::new(&pl, &tracks);
        let row = &vm.track_rows()[0];
        let display = row.display(pl.id);

        assert!(!row.is_available());
        assert!(!row.can_play());
        assert_eq!(row.availability_label(), Some("Unavailable"));
        assert!(!display.is_available);
        assert_eq!(display.availability_label, Some("Unavailable"));
        assert!(!display.controls.play_enabled);
    }

    #[test]
    fn playlist_track_row_vm_thumb_prefers_track_image_then_album_image() {
        let pl = playlist("Mix");
        let mut t1 = row();
        t1.track_image_href = Some("track".into());
        t1.album_image_href = Some("album".into());
        let mut t2 = row();
        t2.album_image_href = Some("album-only".into());
        let tracks = [t1, t2];
        let vm = PlaylistDetailVm::new(&pl, &tracks);
        let rows = vm.track_rows();
        assert_eq!(rows[0].thumb_url(), Some("track"));
        assert_eq!(rows[1].thumb_url(), Some("album-only"));
    }

    fn feed_view_with(title: Option<&str>, artist: Option<&str>) -> FeedView {
        FeedView {
            title: title.map(str::to_string),
            artist: artist.map(str::to_string),
            ..FeedView::default()
        }
    }

    fn library_tree() -> LibraryTree {
        let mut rhubarb = row();
        rhubarb.id = 1;
        rhubarb.track_title = Some("Rhubarb".into());
        rhubarb.is_in_library = true;
        let mut cliffs = row();
        cliffs.id = 2;
        cliffs.track_title = Some("Cliffs".into());
        cliffs.is_in_library = true;
        let mut windowlicker = row();
        windowlicker.id = 3;
        windowlicker.track_title = Some("Windowlicker".into());
        windowlicker.is_in_library = true;
        let mut tri_repetae = row();
        tri_repetae.id = 4;
        tri_repetae.is_in_library = true;

        LibraryTree {
            artists: vec![
                ArtistNode {
                    name: "Aphex Twin".into(),
                    albums: vec![
                        AlbumNode {
                            name: "Selected Ambient Works".into(),
                            feed_id: Some(10),
                            feed_guid: Some("saw-guid".into()),
                            feed_url: Some("https://example.test/saw.xml".into()),
                            language: None,
                            description: None,
                            image_href: Some("saw.jpg".into()),
                            identity_facts: LocalIdentityFacts::default(),
                            tracks: vec![rhubarb, cliffs],
                        },
                        AlbumNode {
                            name: "Windowlicker".into(),
                            feed_id: Some(20),
                            feed_guid: None,
                            feed_url: None,
                            language: None,
                            description: None,
                            image_href: None,
                            identity_facts: LocalIdentityFacts::default(),
                            tracks: vec![windowlicker],
                        },
                    ],
                },
                ArtistNode {
                    name: "Autechre".into(),
                    albums: vec![AlbumNode {
                        name: "Tri Repetae".into(),
                        feed_id: Some(30),
                        feed_guid: None,
                        feed_url: None,
                        language: None,
                        description: None,
                        image_href: None,
                        identity_facts: LocalIdentityFacts::default(),
                        tracks: vec![tri_repetae],
                    }],
                },
            ],
        }
    }

    #[test]
    fn album_detail_vm_has_active_musicbrainz_when_any_track_pending_or_processing() {
        let view = feed_view_with(None, None);
        let mut tracks = [row(), row(), row()];
        tracks[0].id = 10;
        tracks[1].id = 20;
        tracks[2].id = 30;
        let mut mb: BTreeMap<i64, MbTrackStatus> = BTreeMap::new();
        mb.insert(10, MbTrackStatus::Done(2));
        let vm = LibraryAlbumDetailVm::new(&view, &tracks, &mb);
        assert!(!vm.has_active_musicbrainz());
        mb.insert(20, MbTrackStatus::Pending);
        let vm = LibraryAlbumDetailVm::new(&view, &tracks, &mb);
        assert!(vm.has_active_musicbrainz());
        mb.insert(20, MbTrackStatus::Processing);
        let vm = LibraryAlbumDetailVm::new(&view, &tracks, &mb);
        assert!(vm.has_active_musicbrainz());
        mb.insert(20, MbTrackStatus::Skipped("err".into()));
        let vm = LibraryAlbumDetailVm::new(&view, &tracks, &mb);
        assert!(!vm.has_active_musicbrainz());
    }

    #[test]
    fn album_detail_vm_musicbrainz_action_projects_label_and_disabled_state() {
        let view = feed_view_with(None, None);
        let mut mb = BTreeMap::new();
        let vm = LibraryAlbumDetailVm::new(&view, &[], &mb);
        let action = vm.musicbrainz_action_vm();
        assert_eq!(action.label, "MusicBrainz");
        assert!(!action.disabled);

        mb.insert(7, MbTrackStatus::Processing);
        let vm = LibraryAlbumDetailVm::new(&view, &[], &mb);
        let action = vm.musicbrainz_action_vm();
        assert_eq!(action.label, "MusicBrainz");
        assert!(action.disabled);
    }

    #[test]
    fn album_detail_vm_projects_description_state_from_feed_description() {
        let mut view = feed_view_with(None, None);
        view.description = Some("one\ntwo\nthree\nfour\nfive".into());
        let mb = BTreeMap::new();
        let vm = LibraryAlbumDetailVm::new(&view, &[], &mb);

        assert_eq!(vm.description_state(), DescriptionState::AutoExpanded);

        view.description = Some("one\ntwo\nthree\nfour\nfive\nsix".into());
        let vm = LibraryAlbumDetailVm::new(&view, &[], &mb);

        assert_eq!(vm.description_state(), DescriptionState::AutoCollapsed);
    }

    #[test]
    fn library_view_model_updates_album_identity_facts_by_feed_id() {
        let mut vm = LibraryViewModel::new();
        vm.replace_tree(library_tree());
        let facts = LocalIdentityFacts {
            source_links: vec![crate::views::IdentityLinkFact {
                link_type: Some("website".into()),
                url: Some("https://example.test/saw".into()),
                ..crate::views::IdentityLinkFact::default()
            }],
            source_ids: vec![crate::views::IdentityIdFact {
                scheme: Some("nostr_npub".into()),
                value: Some("npub1saw".into()),
                ..crate::views::IdentityIdFact::default()
            }],
            contributors: Vec::new(),
        };

        assert!(vm.update_album_identity_facts(10, &facts));
        let album = &vm.tree().artists[0].albums[0];
        assert_eq!(
            album.identity_facts.source_links[0].url.as_deref(),
            Some("https://example.test/saw")
        );
        assert_eq!(
            album.identity_facts.source_ids[0].value.as_deref(),
            Some("npub1saw")
        );
    }

    #[test]
    fn library_view_model_updates_album_description_by_feed_id() {
        let mut vm = LibraryViewModel::new();
        vm.replace_tree(library_tree());

        assert!(vm.update_album_description(10, Some("Source description")));
        let album = &vm.tree().artists[0].albums[0];
        assert_eq!(album.description.as_deref(), Some("Source description"));
    }

    #[test]
    fn library_view_model_starts_with_playlists_expanded_and_default_sort() {
        let vm = LibraryViewModel::new();
        assert!(vm.playlists_expanded);
        assert_eq!(vm.playlist_sort, PlaylistSort::Name);
        assert!(vm.expanded_artists.is_empty());
        assert!(vm.expanded_albums.is_empty());
    }

    #[test]
    fn library_view_model_starts_with_no_selection_or_operation_in_flight() {
        let vm = LibraryViewModel::new();
        assert_eq!(vm.selected_id, None);
        assert_eq!(vm.selected_playlist_id, None);
        assert_eq!(vm.hovered_thumb_url, None);
        assert_eq!(vm.busy_track, None);
        assert!(vm.status.is_empty());
        assert!(vm.search_query.is_empty());
        assert!(!vm.creating_playlist);
        assert!(!vm.is_resizing());
        assert_width_eq(vm.split_pane_width(), DEFAULT_SPLIT_PANE_WIDTH);
    }

    #[test]
    fn library_view_model_tracks_resize_lifecycle_and_width() {
        let mut vm = LibraryViewModel::new();

        vm.begin_resize();
        assert!(vm.is_resizing());

        vm.resize_split_pane(120.0, 200.0, 800.0);
        assert_width_eq(vm.split_pane_width(), 200.0);

        vm.resize_split_pane(900.0, 200.0, 800.0);
        assert_width_eq(vm.split_pane_width(), 800.0);

        vm.resize_split_pane(420.0, 200.0, 800.0);
        assert_width_eq(vm.split_pane_width(), 420.0);

        vm.end_resize();
        assert!(!vm.is_resizing());
    }

    #[test]
    fn library_view_model_starts_with_empty_snapshots_and_idle_feed_update() {
        let vm = LibraryViewModel::new();
        assert!(vm.tree().artists.is_empty());
        assert!(vm.playlists().is_empty());
        assert!(vm.snapshot.playlist_tracks.is_empty());
        assert!(vm.mb_status().is_empty());
        assert!(vm.snapshot.staged_musicbrainz.is_empty());
        assert!(vm.snapshot.in_flight_feed_checks.is_empty());
        assert_eq!(vm.feed_update_state().phase, FeedUpdatePhase::Idle);
        assert!(vm.feed_update_state().status_message.is_none());
        assert!(vm.feed_update_state().stale.is_empty());
    }

    #[test]
    fn library_view_model_tree_projection_uses_saved_expansion_without_query() {
        let mut vm = LibraryViewModel::new();
        vm.replace_tree(library_tree());
        vm.toggle_artist("Aphex Twin");
        vm.toggle_album("Aphex Twin", "Selected Ambient Works");

        let projection = vm.tree_projection();

        assert_eq!(projection.tree.artists.len(), 2);
        assert!(projection.expanded_artists.contains("Aphex Twin"));
        assert!(projection
            .expanded_albums
            .contains(&("Aphex Twin".into(), "Selected Ambient Works".into())));
    }

    #[test]
    fn library_view_model_tree_projection_filters_tracks_and_expands_matches() {
        let mut vm = LibraryViewModel::new();
        vm.replace_tree(library_tree());
        vm.apply_search_query("  cliff  ");

        let projection = vm.tree_projection();

        assert_eq!(projection.tree.artists.len(), 1);
        assert_eq!(projection.tree.artists[0].name, "Aphex Twin");
        assert_eq!(projection.tree.artists[0].albums.len(), 1);
        assert_eq!(
            projection.tree.artists[0].albums[0].tracks[0]
                .track_title
                .as_deref(),
            Some("Cliffs")
        );
        assert!(projection.expanded_artists.contains("Aphex Twin"));
        assert!(projection
            .expanded_albums
            .contains(&("Aphex Twin".into(), "Selected Ambient Works".into())));
    }

    #[test]
    fn library_view_model_tree_projection_album_match_keeps_all_album_tracks() {
        let mut vm = LibraryViewModel::new();
        vm.replace_tree(library_tree());
        vm.apply_search_query("ambient");

        let projection = vm.tree_projection();

        assert_eq!(projection.tree.artists.len(), 1);
        assert_eq!(projection.tree.artists[0].albums.len(), 1);
        assert_eq!(projection.tree.artists[0].albums[0].tracks.len(), 2);
    }

    #[test]
    fn library_view_model_content_filter_does_not_filter_source_tree() {
        let mut vm = LibraryViewModel::new();
        vm.replace_tree(library_tree());

        vm.set_content_filter(ContentFilter::Index);
        let projection = vm.tree_projection();

        assert_eq!(
            projection.tree.artists.len(),
            2,
            "content-source filters must not hide local Library source-tree rows"
        );
        assert_eq!(
            vm.content_filter_empty_state(),
            Some(ContentListEmptyStateDisplay {
                title: "No index content",
                secondary: "No index rows match this frame filter.",
                clear_filter_action_id: Some("content-list.clear-filter"),
            }),
            "empty-filter state should be owned by the content-list page VM"
        );

        vm.set_content_filter(ContentFilter::Library);
        let projection = vm.tree_projection();

        assert_eq!(projection.tree.artists.len(), 2);
        assert_eq!(
            vm.content_filter_chip_strip().selected,
            ContentFilter::Library,
            "frame chrome should reflect the selected content-list filter"
        );
    }

    #[test]
    fn library_view_model_content_text_filter_does_not_filter_source_tree() {
        let mut vm = LibraryViewModel::new();
        vm.replace_tree(library_tree());

        vm.set_content_text_filter(Some("cliff".to_string()));
        let projection = vm.tree_projection();

        assert_eq!(vm.content_text_filter(), Some("cliff"));
        assert_eq!(
            projection.tree.artists.len(),
            2,
            "content-list text filtering must not hide source-tree rows"
        );
        let visible_rows = vm.content_list_page.visible_rows();
        assert_eq!(visible_rows.len(), 1);
        assert_eq!(visible_rows[0].title, "Cliffs");

        vm.set_content_text_filter(None);
        let projection = vm.tree_projection();

        assert_eq!(vm.content_text_filter(), None);
        assert_eq!(
            vm.content_list_page.visible_rows().len(),
            4,
            "clearing the content-list text filter should restore page VM rows"
        );
        assert_eq!(
            projection
                .tree
                .artists
                .iter()
                .flat_map(|artist| &artist.albums)
                .flat_map(|album| &album.tracks)
                .count(),
            4,
            "source-tree rows should remain stable across content-list text filtering"
        );
    }

    #[test]
    fn library_view_model_source_text_filter_trims_and_clears_selection() {
        let mut vm = LibraryViewModel::new();
        vm.replace_tree(library_tree());
        vm.select_library_item(99);

        vm.set_source_text_filter(Some("  ambient  ".to_string()));

        assert_eq!(vm.search_query(), "ambient");
        assert_eq!(vm.selected_id(), None);
        assert_eq!(
            vm.tree_projection().tree.artists[0].albums[0].tracks.len(),
            2,
            "source-list text filter should reuse existing tree query projection"
        );

        vm.set_source_text_filter(Some("   ".to_string()));

        assert_eq!(vm.search_query(), "");
        assert_eq!(
            vm.tree_projection().tree.artists.len(),
            2,
            "whitespace source-list filter should clear the query"
        );
    }

    #[test]
    fn library_tree_artist_display_projects_row_chrome() {
        let tree = library_tree();
        let artist = &tree.artists[0];

        let expanded = artist.tree_display(true);
        assert_eq!(expanded.element_id, "artist-Aphex Twin");
        assert_eq!(expanded.title, "Aphex Twin");
        assert_eq!(expanded.disclosure_glyph, "\u{25BC}");
        assert_eq!(expanded.album_count_label, "(2 albums)");

        let collapsed = artist.tree_display(false);
        assert_eq!(collapsed.disclosure_glyph, "\u{25B6}");
    }

    #[test]
    fn library_tree_album_display_projects_row_chrome() {
        let tree = library_tree();
        let album = &tree.artists[0].albums[0];

        let expanded = album.tree_display("Aphex Twin", true);
        assert_eq!(
            expanded.element_id,
            "album-Aphex Twin-Selected Ambient Works"
        );
        assert_eq!(expanded.title, "Selected Ambient Works");
        assert_eq!(expanded.disclosure_glyph, "\u{25BC}");
        assert_eq!(expanded.track_count_label, "(2)");

        let collapsed = album.tree_display("Aphex Twin", false);
        assert_eq!(collapsed.disclosure_glyph, "\u{25B6}");
    }

    #[test]
    fn library_tree_track_display_projects_id_and_prefixed_title() {
        let mut track = row();
        track.id = 42;
        track.track_number = Some(7);
        track.track_title = Some("Pulsewidth".into());

        let display = LibraryTrackRowVm::new(&track, None).tree_display();

        assert_eq!(display.element_id, "tree-track-42");
        assert_eq!(display.title, "07 - Pulsewidth");
    }

    #[test]
    fn library_view_model_apply_search_query_clears_track_selection() {
        let mut vm = LibraryViewModel::new();
        vm.select_library_item(99);

        vm.apply_search_query("aphex");

        assert_eq!(vm.selected_id(), None);
        assert_eq!(vm.search_query(), "aphex");
    }

    #[test]
    fn library_view_model_toggle_artist_round_trip() {
        let mut vm = LibraryViewModel::new();
        assert!(!vm.is_artist_expanded("Aphex"));
        vm.toggle_artist("Aphex");
        assert!(vm.is_artist_expanded("Aphex"));
        vm.toggle_artist("Aphex");
        assert!(!vm.is_artist_expanded("Aphex"));
    }

    #[test]
    fn library_view_model_toggle_album_keys_on_artist_and_album() {
        let mut vm = LibraryViewModel::new();
        vm.toggle_album("Aphex", "SAW II");
        assert!(vm.is_album_expanded("Aphex", "SAW II"));
        // Different (artist, album) combination is independent.
        assert!(!vm.is_album_expanded("Aphex", "Drukqs"));
        assert!(!vm.is_album_expanded("Other", "SAW II"));
        vm.toggle_album("Aphex", "SAW II");
        assert!(!vm.is_album_expanded("Aphex", "SAW II"));
    }

    #[test]
    fn library_view_model_toggle_playlists_expanded_flips_flag() {
        let mut vm = LibraryViewModel::new();
        assert!(vm.playlist_sidebar().expanded);
        vm.toggle_playlists_expanded();
        assert!(!vm.playlist_sidebar().expanded);
        vm.toggle_playlists_expanded();
        assert!(vm.playlist_sidebar().expanded);
    }

    #[test]
    fn library_view_model_cycle_playlist_sort_advances_through_three_states() {
        let mut vm = LibraryViewModel::new();
        assert_eq!(vm.playlist_sort_label(), "A–Z");
        vm.cycle_playlist_sort();
        assert_eq!(vm.playlist_sort_label(), "Recent");
        vm.cycle_playlist_sort();
        assert_eq!(vm.playlist_sort_label(), "Size");
        vm.cycle_playlist_sort();
        assert_eq!(vm.playlist_sort_label(), "A–Z");
    }

    #[test]
    fn library_view_model_playlist_sort_label_reflects_active_sort() {
        let mut vm = LibraryViewModel::new();
        assert_eq!(vm.playlist_sort_label(), "A–Z");
        vm.cycle_playlist_sort();
        assert_eq!(vm.playlist_sort_label(), "Recent");
        vm.cycle_playlist_sort();
        assert_eq!(vm.playlist_sort_label(), "Size");
    }

    #[test]
    fn library_view_model_replaces_and_sorts_playlists_by_active_sort() {
        let mut vm = LibraryViewModel::new();
        let mut alpha = playlist("Alpha");
        alpha.id = 10;
        alpha.track_count = 3;
        alpha.updated_at = 1;
        let mut zed = playlist("zed");
        zed.id = 20;
        zed.track_count = 9;
        zed.updated_at = 5;

        vm.replace_playlists(vec![zed.clone(), alpha.clone()]);
        assert_eq!(vm.playlists()[0].id, alpha.id);
        assert_eq!(
            vm.playlist_by_id(zed.id).map(|playlist| playlist.name),
            Some("zed".into())
        );

        vm.cycle_playlist_sort();
        vm.sort_loaded_playlists();
        assert_eq!(vm.playlists()[0].id, zed.id);

        vm.cycle_playlist_sort();
        vm.sort_loaded_playlists();
        assert_eq!(vm.playlists()[0].track_count, 9);
    }

    #[test]
    fn library_view_model_playlist_sidebar_projects_rows_and_header_state() {
        let mut vm = LibraryViewModel::new();
        let mut alpha = playlist("Alpha");
        alpha.id = 10;
        alpha.track_count = 3;
        let mut zed = playlist("zed");
        zed.id = 20;
        zed.track_count = 9;
        vm.select_playlist(20);
        vm.toggle_creating_playlist();

        vm.replace_playlists(vec![zed, alpha]);
        let sidebar = vm.playlist_sidebar();

        assert_eq!(sidebar.header_id, "playlists-header");
        assert_eq!(sidebar.sort_button_id, "playlists-sort");
        assert_eq!(sidebar.add_button_id, "playlists-add");
        assert_eq!(sidebar.new_playlist_input_id, "playlist-new-input");
        assert_eq!(sidebar.new_playlist_add_button_id, "playlist-add-btn");
        assert!(sidebar.expanded);
        assert_eq!(sidebar.disclosure_glyph, "\u{25BC}");
        assert_eq!(sidebar.heading, "Playlists");
        assert_eq!(sidebar.sort_label, "A–Z");
        assert_eq!(sidebar.add_label, "+");
        assert_eq!(sidebar.new_playlist_add_label, "Add");
        assert!(sidebar.creating_playlist);
        assert_eq!(sidebar.rows.len(), 2);
        assert_eq!(sidebar.rows[0].id, 10);
        assert_eq!(sidebar.rows[0].element_id, "playlist-10");
        assert_eq!(sidebar.rows[0].name, "Alpha");
        assert_eq!(sidebar.rows[0].track_count_label, "(3)");
        assert!(!sidebar.rows[0].selected);
        assert!(sidebar.rows[1].selected);
    }

    #[test]
    fn library_view_model_saved_searches_start_empty_without_section() {
        let vm = LibraryViewModel::new();

        assert!(vm.saved_searches().is_empty());
        assert!(vm.saved_searches_section().is_none());
    }

    #[test]
    fn library_view_model_saved_searches_preserve_seeded_order() {
        let mut vm = LibraryViewModel::new();
        let first = SavedSearchEntry {
            id: 10,
            query: "artist:aphex".into(),
            label: "Aphex".into(),
            a11y_label: "Open saved search Aphex".into(),
        };
        let second = SavedSearchEntry {
            id: 20,
            query: "tag:ambient".into(),
            label: "Ambient".into(),
            a11y_label: "Open saved search Ambient".into(),
        };

        vm.set_saved_searches(vec![first.clone(), second.clone()]);

        assert_eq!(vm.saved_searches(), &[first.clone(), second.clone()]);
        assert_eq!(
            vm.saved_searches_section(),
            Some(SavedSearchesSectionDisplay {
                heading: "Saved Searches",
                rows: vec![first.clone(), second.clone()],
            })
        );
        assert_eq!(vm.saved_searches(), &[first, second]);
    }

    #[test]
    fn library_view_model_album_description_state_is_sticky_per_feed() {
        let mut vm = LibraryViewModel::new();
        let long = Some("one\ntwo\nthree\nfour\nfive\nsix");

        assert_eq!(
            vm.album_description_state(7, long),
            DescriptionState::AutoCollapsed
        );

        vm.toggle_album_description(7, long);
        assert_eq!(
            vm.album_description_state(7, Some("short")),
            DescriptionState::UserExpanded
        );
        assert_eq!(
            vm.album_description_state(8, Some("short")),
            DescriptionState::AutoExpanded
        );
    }

    #[test]
    fn library_view_model_track_description_state_is_sticky_per_track() {
        let mut vm = LibraryViewModel::new();
        let long = Some("one\ntwo\nthree\nfour\nfive\nsix");

        assert_eq!(
            vm.track_description_state(11, long),
            DescriptionState::AutoCollapsed
        );

        vm.set_track_description_state(11, DescriptionState::UserExpanded);

        assert_eq!(
            vm.track_description_state(11, Some("short")),
            DescriptionState::UserExpanded
        );
        assert_eq!(
            vm.track_description_state(12, Some("short")),
            DescriptionState::AutoExpanded
        );
    }

    #[test]
    fn library_view_model_playlist_sidebar_reflects_collapsed_state() {
        let mut vm = LibraryViewModel::new();
        vm.toggle_playlists_expanded();

        let sidebar = vm.playlist_sidebar();

        assert!(!sidebar.expanded);
        assert_eq!(sidebar.disclosure_glyph, "\u{25B6}");
    }

    #[test]
    fn library_chrome_display_projects_shell_labels() {
        let display = LibraryViewModel::chrome_display();
        assert_eq!(display.split_pane_id, "library-pane-container");
        assert_eq!(display.resize_handle_id, "library-resize-handle");
        assert_eq!(display.search_button_id, "lib-search-btn");
        assert_eq!(display.list_scroll_id, "library-list");
        assert_eq!(display.artist_detail_scroll_id, "artist-detail-scroll");
        assert_eq!(display.playlist_detail_scroll_id, "playlist-detail-scroll");
        assert_eq!(display.track_detail_scroll_id, "track-detail-scroll");
        assert_eq!(display.search_placeholder, "Search your library...");
        assert_eq!(
            display.new_playlist_placeholder,
            "New playlist name\u{2026}"
        );
        assert_eq!(display.search_heading, "Search Library");
        assert_eq!(display.search_button_label, "Search");
        assert_eq!(display.empty_library_label, "No library tracks yet");
        assert_eq!(display.empty_detail_label, "Select an item to view details");
    }

    #[test]
    fn library_status_snapshot_classifies_error_prefix() {
        let mut vm = LibraryViewModel::new();
        vm.set_error_status("offline");

        let status = vm.status_snapshot();

        assert_eq!(status.text, "Error: offline");
        assert!(status.is_error);
        assert!(!vm.should_show_empty_library(true));

        let mut vm = LibraryViewModel::new();
        vm.finish_library_reload(0);
        let status = vm.status_snapshot();
        assert_eq!(status.text, "0 library tracks");
        assert!(!status.is_error);
        assert!(vm.should_show_empty_library(true));
    }

    #[test]
    fn library_view_model_projects_hig_removal_confirmation_display() {
        let mut vm = LibraryViewModel::new();
        let plan = LibraryRemovalPlan::new(
            LibraryRemovalTarget::Track(7),
            LibraryRemovalImpact::Track {
                playlist_reference_count: 1,
            },
        );

        assert!(!vm.confirm_library_removal(plan));
        let display = vm
            .pending_library_removal_confirmation()
            .expect("playlist-referenced removal should require confirmation");

        assert_eq!(display.title, "Remove Track from Library?");
        assert_eq!(
            display.message,
            "This track is in 1 playlist. Removing it from the library will make it unavailable for playlist playback."
        );
        assert_eq!(display.cancel_label, "Cancel");
        assert_eq!(display.remove_label, "Remove");
    }

    #[test]
    fn feed_update_display_projects_toolbar_action_labels() {
        let mut vm = LibraryViewModel::new();
        let display = vm.feed_update_display();
        assert_eq!(display.status_message, None);
        assert_eq!(display.action.kind, FeedUpdateActionKind::CheckAllFeeds);
        assert_eq!(display.action.button_id, "check-all-feeds");
        assert_eq!(display.action.label, "Check all feeds");
        assert!(!display.action.disabled);

        vm.begin_all_feed_check(3);
        let display = vm.feed_update_display();
        assert_eq!(
            display.status_message.as_deref(),
            Some("Checking 3 feeds...")
        );
        assert_eq!(display.action.kind, FeedUpdateActionKind::CheckAllFeeds);
        assert_eq!(display.action.button_id, "check-all-feeds");
        assert_eq!(display.action.label, "Checking...");
        assert!(display.action.disabled);

        vm.finish_all_feed_check(vec![feed_service::StaleFeed {
            feed_id: 1,
            feed_guid: "feed-1".into(),
            title: Some("Feed".into()),
            new_updated_at: 10,
        }]);
        let display = vm.feed_update_display();
        assert_eq!(display.action.kind, FeedUpdateActionKind::ApplyUpdates);
        assert_eq!(display.action.button_id, "apply-feed-updates");
        assert_eq!(display.action.label, "Apply updates (1)");
        assert!(!display.action.disabled);
    }

    #[test]
    fn library_view_model_toggle_creating_playlist_flips_flag() {
        let mut vm = LibraryViewModel::new();
        assert!(!vm.playlist_sidebar().creating_playlist);
        vm.toggle_creating_playlist();
        assert!(vm.playlist_sidebar().creating_playlist);
        vm.close_creating_playlist();
        assert!(!vm.playlist_sidebar().creating_playlist);
        vm.toggle_creating_playlist();
        assert!(vm.playlist_sidebar().creating_playlist);
        vm.toggle_creating_playlist();
        assert!(!vm.playlist_sidebar().creating_playlist);
    }

    #[test]
    fn library_view_model_tracks_playlist_rename_state() {
        let mut vm = LibraryViewModel::new();
        vm.toggle_creating_playlist();

        vm.begin_playlist_rename(7);

        assert_eq!(vm.renaming_playlist_id(), Some(7));
        assert!(vm.is_renaming_playlist(7));
        assert!(!vm.is_renaming_playlist(8));
        assert!(!vm.playlist_sidebar().creating_playlist);

        vm.select_playlist(7);
        assert_eq!(vm.renaming_playlist_id(), Some(7));

        vm.select_playlist(8);
        assert_eq!(vm.renaming_playlist_id(), None);

        vm.begin_playlist_rename(9);
        vm.cancel_playlist_rename();
        assert_eq!(vm.renaming_playlist_id(), None);
    }

    #[test]
    fn library_view_model_tracks_playlist_snapshot_replacement() {
        let mut vm = LibraryViewModel::new();
        let mut track = row();
        track.id = 42;

        vm.replace_playlist_tracks(vec![track]);

        assert_eq!(vm.snapshot.playlist_tracks.len(), 1);
        assert_eq!(vm.snapshot.playlist_tracks[0].id, 42);
    }

    #[test]
    fn library_view_model_library_reload_status_pluralizes_track_count() {
        let mut vm = LibraryViewModel::new();

        vm.finish_library_reload(1);
        assert_eq!(vm.status(), "1 library track");

        vm.finish_library_reload(2);
        assert_eq!(vm.status(), "2 library tracks");
    }

    #[test]
    fn library_view_model_begin_library_reload_sets_loading_flag() {
        let mut vm = LibraryViewModel::new();
        assert!(!vm.is_library_loading());

        vm.begin_library_reload();
        assert!(vm.is_library_loading());
        assert_eq!(vm.status(), "Loading library\u{2026}");
        assert!(!vm.status_snapshot().is_error);

        vm.finish_library_reload(3);
        assert!(!vm.is_library_loading());
        assert_eq!(vm.status(), "3 library tracks");
    }

    #[test]
    fn library_view_model_set_error_status_clears_loading_flag() {
        let mut vm = LibraryViewModel::new();
        vm.begin_library_reload();
        assert!(vm.is_library_loading());

        vm.set_error_status("db down");

        assert!(!vm.is_library_loading());
        assert!(vm.status_snapshot().is_error);
    }

    #[test]
    fn library_view_model_playlist_failures_format_status_text() {
        let mut vm = LibraryViewModel::new();

        vm.fail_playlist_load("db");
        assert_eq!(vm.status(), "Error loading playlists: db");
        vm.fail_playlist_create("exists");
        assert_eq!(vm.status(), "Error creating playlist: exists");
        vm.fail_playlist_rename("bad");
        assert_eq!(vm.status(), "Error renaming: bad");
        vm.fail_playlist_delete("missing");
        assert_eq!(vm.status(), "Error deleting: missing");
        vm.fail_playlist_track_remove("locked");
        assert_eq!(vm.status(), "Error removing track: locked");
        vm.fail_playlist_track_reorder("position");
        assert_eq!(vm.status(), "Error reordering: position");
    }

    #[test]
    fn library_view_model_requires_explicit_confirmation_for_playlist_referenced_removals() {
        let mut vm = LibraryViewModel::new();

        assert!(vm.confirm_library_removal(LibraryRemovalPlan::new(
            LibraryRemovalTarget::Track(1),
            LibraryRemovalImpact::Track {
                playlist_reference_count: 0,
            },
        )));
        assert!(!vm.confirm_library_removal(LibraryRemovalPlan::new(
            LibraryRemovalTarget::Track(1),
            LibraryRemovalImpact::Track {
                playlist_reference_count: 2,
            },
        )));
        assert_eq!(
            vm.take_pending_library_removal(),
            Some(LibraryRemovalTarget::Track(1))
        );

        assert!(!vm.confirm_library_removal(LibraryRemovalPlan::new(
            LibraryRemovalTarget::Feed(5),
            LibraryRemovalImpact::Feed {
                playlist_track_count: 1,
            },
        )));
        let display = vm
            .pending_library_removal_confirmation()
            .expect("feed removal should require confirmation");
        assert_eq!(display.title, "Remove Feed from Library?");
        assert_eq!(
            display.message,
            "1 track from this feed is in playlists. Removing it from the library will make it unavailable for playlist playback."
        );
        vm.cancel_pending_library_removal();
        assert!(vm.pending_library_removal_confirmation().is_none());
    }

    #[test]
    fn library_view_model_album_track_load_status_helpers_format_text() {
        let mut vm = LibraryViewModel::new();

        vm.fail_album_tracks_load("db");
        assert_eq!(vm.status(), "Error loading album tracks: db");
        vm.set_album_has_no_tracks();
        assert_eq!(vm.status(), "Album has no tracks");
    }

    #[test]
    fn library_view_model_playlist_append_intent_sets_status_and_playlist_name() {
        let mut vm = LibraryViewModel::new();
        let mut playlist = playlist("Focus");
        playlist.id = 12;
        vm.replace_playlists(vec![playlist]);

        let intent = vm
            .begin_playlist_append(12, vec![7, 8])
            .expect("non-empty track ids should build an append intent");

        assert_eq!(intent.playlist_id(), 12);
        assert_eq!(intent.playlist_name(), "Focus");
        assert_eq!(intent.total_tracks(), 2);
        assert_eq!(intent.track_ids(), &[7, 8]);
        assert_eq!(vm.status(), "Downloading 2 tracks...");
    }

    #[test]
    fn library_view_model_playlist_append_ignores_empty_track_ids() {
        let mut vm = LibraryViewModel::new();
        vm.finish_library_reload(1);

        assert!(vm.begin_playlist_append(12, Vec::new()).is_none());
        assert_eq!(vm.status(), "1 library track");
    }

    #[test]
    fn library_view_model_playlist_append_finish_formats_counts() {
        let mut vm = LibraryViewModel::new();
        let mut playlist = playlist("Focus");
        playlist.id = 12;
        vm.replace_playlists(vec![playlist]);
        let intent = vm
            .begin_playlist_append(12, vec![7, 8, 9])
            .expect("non-empty track ids should build an append intent");

        vm.finish_playlist_append(&intent, PlaylistAppendOutcome::new(2, 1, 1));

        assert_eq!(
            vm.status(),
            "Added 2 of 3 to Focus (downloaded 1); 1 failed"
        );
    }

    #[test]
    fn library_view_model_playlist_append_finish_omits_zero_optional_counts() {
        let mut vm = LibraryViewModel::new();
        let mut playlist = playlist("Focus");
        playlist.id = 12;
        vm.replace_playlists(vec![playlist]);
        let intent = vm
            .begin_playlist_append(12, vec![7])
            .expect("non-empty track ids should build an append intent");

        vm.finish_playlist_append(&intent, PlaylistAppendOutcome::new(1, 0, 0));

        assert_eq!(vm.status(), "Added 1 of 1 to Focus");
    }

    #[test]
    fn library_view_model_playlist_append_failure_sets_error_status() {
        let mut vm = LibraryViewModel::new();

        vm.fail_playlist_append("offline");

        assert_eq!(vm.status(), "Error adding to playlist: offline");
    }

    #[test]
    fn library_view_model_selection_methods_keep_sidebar_and_tree_exclusive() {
        let mut vm = LibraryViewModel::new();
        vm.select_playlist(7);
        assert_eq!(vm.selected_id(), None);
        assert_eq!(vm.selected_playlist_id(), Some(7));
        assert!(vm.is_playlist_selected(7));

        vm.select_library_item(42);
        assert_eq!(vm.selected_id(), Some(42));
        assert_eq!(vm.selected_playlist_id(), None);

        vm.clear_library_selection();
        assert_eq!(vm.selected_id(), None);
    }

    #[test]
    fn library_view_model_clears_matching_playlist_selection_only() {
        let mut vm = LibraryViewModel::new();
        vm.select_playlist(7);

        assert!(!vm.clear_playlist_selection_if(8));
        assert_eq!(vm.selected_playlist_id(), Some(7));
        assert!(vm.clear_playlist_selection_if(7));
        assert_eq!(vm.selected_playlist_id(), None);
    }

    #[test]
    fn library_view_model_hovered_thumb_reports_only_changes() {
        let mut vm = LibraryViewModel::new();
        assert_eq!(vm.hovered_thumb_url(), None);
        assert!(vm.set_hovered_thumb_url(Some("img".into())));
        assert_eq!(vm.hovered_thumb_url(), Some("img"));
        assert!(!vm.set_hovered_thumb_url(Some("img".into())));
        assert!(vm.set_hovered_thumb_url(None));
        assert_eq!(vm.hovered_thumb_url(), None);
    }

    #[test]
    fn library_view_model_busy_track_and_status_transition_together() {
        let mut vm = LibraryViewModel::new();
        assert!(!vm.has_busy_track());
        assert_eq!(vm.busy_track(), None);

        vm.begin_busy_track(42, "Downloading track...");

        assert!(vm.has_busy_track());
        assert_eq!(vm.busy_track(), Some(42));
        assert_eq!(vm.status(), "Downloading track...");

        vm.clear_busy_track();
        assert_eq!(vm.busy_track(), None);
    }

    #[test]
    fn library_view_model_busy_feed_and_status_transition_together() {
        let mut vm = LibraryViewModel::new();
        assert!(!vm.has_busy_feed());
        assert_eq!(vm.busy_feed(), None);

        vm.begin_busy_feed(7, "Downloading feed...");

        assert!(vm.has_busy_feed());
        assert_eq!(vm.busy_feed(), Some(7));
        assert_eq!(vm.status(), "Downloading feed...");

        vm.clear_busy_feed();
        assert_eq!(vm.busy_feed(), None);
    }

    #[test]
    fn library_view_model_deferred_panel_error_message_owns_error_prefix() {
        assert_eq!(
            LibraryViewModel::deferred_panel_error_message("offline"),
            "Error: offline"
        );
    }

    #[test]
    fn library_view_model_projects_thumbnail_display_contracts() {
        assert_eq!(
            LibraryViewModel::album_thumb_display(),
            AlbumThumbDisplay {
                fallback_icon: "\u{1F3B5}",
            }
        );
        assert_eq!(
            LibraryViewModel::hover_thumb_display("https://example.test/a.png"),
            HoverThumbDisplay {
                element_id: "thumb-https://example.test/a.png".into(),
            }
        );
    }

    #[test]
    fn library_track_action_vm_formats_subscription_labels() {
        assert_eq!(
            LibraryTrackActionVm::new(false, false, None).subscription_button_label(),
            "Download Track"
        );
        assert_eq!(
            LibraryTrackActionVm::new(false, true, None).subscription_button_label(),
            "Remove Track"
        );
        assert_eq!(
            LibraryTrackActionVm::new(true, false, None).subscription_button_label(),
            "Downloading..."
        );
        assert_eq!(
            LibraryTrackActionVm::new(true, true, None).subscription_button_label(),
            "Removing..."
        );
        assert_eq!(
            LibraryTrackActionVm::subscription_busy_message(true),
            "Downloading..."
        );
        assert_eq!(
            LibraryTrackActionVm::subscription_busy_message(false),
            "Removing..."
        );
        assert_eq!(
            LibraryTrackActionVm::subscription_error_message(true, "offline"),
            "Download error: offline"
        );
        assert_eq!(
            LibraryTrackActionVm::subscription_error_message(false, "offline"),
            "Remove error: offline"
        );
        assert_eq!(
            LibraryTrackActionVm::track_subscribe_begin_status(),
            "Downloading track..."
        );
        assert_eq!(
            LibraryTrackActionVm::track_subscribe_success_message(),
            "Downloaded track"
        );
    }

    #[test]
    fn library_track_action_vm_formats_playlist_label_and_message_status() {
        let closed = LibraryTrackActionVm::new(false, false, Some("Subscribed"));
        assert_eq!(
            LibraryTrackActionVm::add_to_playlist_label(),
            "Add to playlist"
        );
        assert_eq!(
            LibraryTrackActionVm::playlist_display(7),
            LibraryTrackPlaylistDisplay {
                popover_id: "track-inspector-add:7".into(),
                trigger_label: "Add to playlist",
            }
        );
        assert_eq!(
            closed.subscription_message_display(),
            Some(ActionStatusMessageDisplay::neutral("Subscribed"))
        );

        let open = LibraryTrackActionVm::new(false, false, Some("Error: offline"));
        assert_eq!(
            LibraryTrackActionVm::add_to_playlist_label(),
            "Add to playlist"
        );
        assert_eq!(
            open.subscription_message_display(),
            Some(ActionStatusMessageDisplay::danger(
                "Error: offline",
                crate::view_models::ActionStatusMessageWidth::Status,
            ))
        );
    }

    #[test]
    fn library_view_model_track_subscribe_finish_clears_busy_and_formats_warning() {
        let mut vm = LibraryViewModel::new();
        vm.begin_busy_track(42, "Downloading track...");

        vm.finish_track_subscribe(TrackSubscribeOutcome::new(
            "/tmp/song.mp3",
            Some("converted from WAV".into()),
        ));

        assert_eq!(vm.busy_track(), None);
        assert_eq!(
            vm.status(),
            "Downloaded track: /tmp/song.mp3 — converted from WAV"
        );
    }

    #[test]
    fn library_view_model_track_subscribe_failure_clears_busy_and_sets_error() {
        let mut vm = LibraryViewModel::new();
        vm.begin_busy_track(42, "Downloading track...");

        vm.fail_track_subscribe("offline");

        assert_eq!(vm.busy_track(), None);
        assert_eq!(vm.status(), "Error downloading track: offline");
    }

    #[test]
    fn library_view_model_status_helper_sets_error_text() {
        let mut vm = LibraryViewModel::new();
        vm.set_error_status("broken");
        assert_eq!(vm.status(), "Error: broken");
    }

    #[test]
    fn library_view_model_tracks_musicbrainz_status_and_staged_lookup() {
        let mut vm = LibraryViewModel::new();
        vm.set_mb_status(7, MbTrackStatus::Processing);
        assert!(vm.has_mb_status(7));
        assert!(matches!(
            vm.mb_status().get(&7),
            Some(MbTrackStatus::Processing)
        ));

        vm.mark_musicbrainz_pending([8, 9]);
        assert!(matches!(
            vm.mb_status().get(&8),
            Some(MbTrackStatus::Pending)
        ));
        assert!(matches!(
            vm.mb_status().get(&9),
            Some(MbTrackStatus::Pending)
        ));

        let lookup = MusicBrainzLookupResult {
            lookup: crate::musicbrainz::MusicBrainzLookup {
                query: "track".into(),
                candidates: Vec::new(),
            },
            image: None,
        };
        vm.stage_musicbrainz(7, lookup);
        assert_eq!(
            vm.staged_musicbrainz(7)
                .map(|lookup| lookup.lookup.query.as_str()),
            Some("track")
        );

        vm.clear_mb_status();
        assert!(vm.mb_status().is_empty());
    }

    #[test]
    fn library_view_model_musicbrainz_track_lookup_transitions_are_pure() {
        let mut vm = LibraryViewModel::new();
        assert!(vm.begin_musicbrainz_track_lookup(7));
        assert!(!vm.begin_musicbrainz_track_lookup(7));
        assert_eq!(vm.status(), "MusicBrainz lookup...");
        assert!(matches!(
            vm.mb_status().get(&7),
            Some(MbTrackStatus::Processing)
        ));

        vm.finish_musicbrainz_track_lookup(7, 1);
        assert_eq!(vm.status(), "MusicBrainz: staged 1 edit");
        assert!(matches!(
            vm.mb_status().get(&7),
            Some(MbTrackStatus::Done(1))
        ));

        vm.fail_musicbrainz_track_lookup(8, "offline");
        assert_eq!(vm.status(), "MusicBrainz error: offline");
        assert!(matches!(
            vm.mb_status().get(&8),
            Some(MbTrackStatus::Skipped(message)) if message == "offline"
        ));
    }

    #[test]
    fn library_view_model_musicbrainz_album_lookup_transitions_are_pure() {
        let mut vm = LibraryViewModel::new();
        assert!(!vm.begin_musicbrainz_album_lookup([]));
        assert_eq!(vm.status(), "No downloaded tracks to process");

        assert!(vm.begin_musicbrainz_album_lookup([7, 8]));
        assert_eq!(vm.status(), "MusicBrainz: album lookup for 2 tracks...");
        assert!(matches!(
            vm.mb_status().get(&7),
            Some(MbTrackStatus::Pending)
        ));
        assert!(matches!(
            vm.mb_status().get(&8),
            Some(MbTrackStatus::Pending)
        ));

        vm.fail_musicbrainz_album_lookup_with_fallback("offline");
        assert_eq!(
            vm.status(),
            "Album lookup failed (offline), falling back to per-track..."
        );
        vm.fallback_empty_musicbrainz_album_lookup();
        assert_eq!(
            vm.status(),
            "Album lookup: no results, falling back to per-track..."
        );

        vm.begin_musicbrainz_album_track_stage(7, 1, 2);
        assert_eq!(vm.status(), "MusicBrainz: staging track 1/2 ...");
        assert!(matches!(
            vm.mb_status().get(&7),
            Some(MbTrackStatus::Processing)
        ));
        vm.finish_musicbrainz_album_track_stage(7, MbTrackStatus::Done(2));
        assert!(matches!(
            vm.mb_status().get(&7),
            Some(MbTrackStatus::Done(2))
        ));

        vm.finish_musicbrainz_album_lookup(1, 2);
        assert_eq!(vm.status(), "MusicBrainz: staged 1 edit across 2 tracks");
        vm.finish_musicbrainz_album_lookup(3, 2);
        assert_eq!(vm.status(), "MusicBrainz: staged 3 edits across 2 tracks");
    }

    #[test]
    fn library_view_model_single_feed_check_dedupes_and_records_stale() {
        let mut vm = LibraryViewModel::new();
        assert!(vm.begin_feed_view_check(42));
        assert!(!vm.begin_feed_view_check(42));
        assert_eq!(
            vm.feed_update_state().status_message.as_deref(),
            Some("Checking feed...")
        );

        vm.finish_feed_view_check(
            42,
            Ok(Some(feed_service::StaleFeed {
                feed_id: 42,
                feed_guid: "feed-guid".into(),
                title: Some("Feed".into()),
                new_updated_at: 100,
            })),
        );

        assert_eq!(vm.feed_update_state().stale.len(), 1);
        assert_eq!(
            vm.feed_update_state().status_message.as_deref(),
            Some("1 feed update pending")
        );
    }

    #[test]
    fn library_view_model_bulk_feed_check_and_apply_transitions_are_pure() {
        let mut vm = LibraryViewModel::new();
        vm.begin_all_feed_check(2);
        assert_eq!(vm.feed_update_state().phase, FeedUpdatePhase::Checking);
        assert_eq!(
            vm.feed_update_state().status_message.as_deref(),
            Some("Checking 2 feeds...")
        );

        vm.finish_all_feed_check(vec![feed_service::StaleFeed {
            feed_id: 1,
            feed_guid: "one".into(),
            title: None,
            new_updated_at: 9,
        }]);
        assert_eq!(vm.feed_update_state().phase, FeedUpdatePhase::Idle);
        assert_eq!(
            vm.feed_update_state().status_message.as_deref(),
            Some("1 feed update available")
        );

        let stale = vm
            .begin_apply_feed_updates()
            .expect("stale feeds should apply");
        assert_eq!(stale.len(), 1);
        assert_eq!(vm.feed_update_state().phase, FeedUpdatePhase::Applying);
        vm.finish_apply_feed_updates("Done".into());
        assert_eq!(vm.feed_update_state().phase, FeedUpdatePhase::Idle);
        assert!(vm.feed_update_state().stale.is_empty());
        assert_eq!(
            vm.feed_update_state().status_message.as_deref(),
            Some("Done")
        );

        vm.begin_all_feed_check(1);
        vm.set_feed_check_error("offline");
        assert_eq!(
            vm.feed_update_state().status_message.as_deref(),
            Some("Feed check error: offline")
        );

        vm.begin_feed_view_check(7);
        vm.finish_feed_view_check_error(7, "timeout");
        assert_eq!(
            vm.feed_update_state().status_message.as_deref(),
            Some("Feed check error: timeout")
        );

        vm.finish_all_feed_check(vec![feed_service::StaleFeed {
            feed_id: 2,
            feed_guid: "two".into(),
            title: None,
            new_updated_at: 10,
        }]);
        vm.begin_apply_feed_updates()
            .expect("stale feeds should apply");
        vm.finish_apply_feed_updates_error("offline");
        assert_eq!(vm.feed_update_state().phase, FeedUpdatePhase::Idle);
        assert_eq!(
            vm.feed_update_state().status_message.as_deref(),
            Some("Feed update error: offline")
        );
    }

    #[test]
    fn album_detail_vm_playlist_action_uses_shared_feed_vocabulary() {
        let view = feed_view_with(None, None);
        let mb = BTreeMap::new();
        let vm = LibraryAlbumDetailVm::new(&view, &[], &mb);
        let action = vm
            .playlist_action_vm(7)
            .expect("playlist action should render");

        assert_eq!(action.label, "Add feed to playlist ▾");
    }

    #[test]
    fn album_detail_vm_playlist_display_projects_popover_id_and_label() {
        let view = feed_view_with(None, None);
        let mb = BTreeMap::new();
        let vm = LibraryAlbumDetailVm::new(&view, &[], &mb);
        let display = vm
            .playlist_display(7)
            .expect("playlist display should render");

        assert_eq!(display.popover_id, "album-feed-add:7");
        assert_eq!(display.trigger_label, "Add feed to playlist ▾");
    }

    #[test]
    fn album_detail_vm_release_actions_use_shared_feed_vocabulary() {
        let view = feed_view_with(None, None);
        let mb = BTreeMap::new();
        let tracks = vec![TrackRow {
            is_in_library: true,
            ..TrackRow::default()
        }];
        let vm = LibraryAlbumDetailVm::new(&view, &tracks, &mb);
        let primary = vm.primary_action_vm(7, false);
        let busy = vm.primary_action_vm(7, true);
        let playlist = vm
            .playlist_action_vm(7)
            .expect("playlist action should render");

        assert_eq!(primary.label, "Remove Feed");
        assert!(primary.enabled);
        assert_eq!(busy.label, "Removing...");
        assert!(!busy.enabled);
        assert_eq!(playlist.label, "Add feed to playlist ▾");
    }

    #[test]
    fn album_detail_vm_empty_library_album_is_downloadable() {
        let view = feed_view_with(None, None);
        let mb = BTreeMap::new();
        let tracks = vec![TrackRow {
            is_in_library: false,
            ..TrackRow::default()
        }];
        let vm = LibraryAlbumDetailVm::new(&view, &tracks, &mb);

        let primary = vm.primary_action_vm(7, false);

        assert_eq!(
            primary.kind,
            crate::view_models::entity_detail::EntityActionKind::Download
        );
        assert_eq!(primary.label, "Download Feed");
    }

    #[test]
    fn album_detail_vm_empty_library_album_busy_action_is_downloading() {
        let view = feed_view_with(None, None);
        let mb = BTreeMap::new();
        let tracks = vec![TrackRow {
            is_in_library: false,
            ..TrackRow::default()
        }];
        let vm = LibraryAlbumDetailVm::new(&view, &tracks, &mb);

        let primary = vm.primary_action_vm(7, true);

        assert_eq!(
            primary.kind,
            crate::view_models::entity_detail::EntityActionKind::Download
        );
        assert_eq!(primary.label, "Downloading...");
        assert!(!primary.enabled);
    }

    #[test]
    fn album_detail_vm_feed_download_marks_remote_rows_busy() {
        let view = feed_view_with(None, None);
        let mb = BTreeMap::new();
        let track = TrackRow {
            is_in_library: false,
            ..TrackRow::default()
        };
        let tracks = vec![track.clone()];
        let vm = LibraryAlbumDetailVm::new(&view, &tracks, &mb);

        assert!(vm.track_row_busy(&track, false, true));
    }

    #[test]
    fn album_detail_vm_feed_removal_marks_only_library_rows_busy() {
        let view = feed_view_with(None, None);
        let mb = BTreeMap::new();
        let library_track = TrackRow {
            is_in_library: true,
            ..TrackRow::default()
        };
        let remote_track = TrackRow {
            is_in_library: false,
            ..TrackRow::default()
        };
        let tracks = vec![library_track.clone(), remote_track.clone()];
        let vm = LibraryAlbumDetailVm::new(&view, &tracks, &mb);

        assert!(vm.track_row_busy(&library_track, false, true));
        assert!(!vm.track_row_busy(&remote_track, false, true));
        assert!(vm.track_row_busy(&remote_track, true, false));
    }
}
