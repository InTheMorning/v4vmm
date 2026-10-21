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
use crate::application::queries::broadcast::{
    BroadcastReadinessReport, BroadcastReadinessState, BroadcastReadinessTrack,
};
use crate::application::queries::stored_values::FeedStoredValues;
use crate::db::{self, TrackRow};
use crate::feed_service;
use crate::metadata::MusicBrainzLookupResult;
use crate::view_models::album_page::AlbumNameLinkVm;
use crate::view_models::artist_detail::{ArtistDetailFactVm, ArtistDetailPageVm};
use crate::view_models::artist_identity::ArtistIdentity;
use crate::view_models::collection_art::{CollectionArtVm, CollectionKind};
use crate::view_models::detail_selection::LibraryReleaseFacts;
use crate::view_models::download_progress::{DownloadActivity, DownloadPhase, FeedKey};
use crate::view_models::entity_detail::{
    EntityActionTarget, EntityActionVm, ReleaseMembershipState, TrackActionState,
    TrackMembershipState,
};
use crate::view_models::format::{fmt_date, fmt_total_runtime_clock, plural};
pub(crate) use crate::view_models::library_removal::RemovalConfirmation;
use crate::view_models::library_removal::{
    LibraryRemovalConfirmationDisplay, LibraryRemovalConfirmationState,
};
use crate::view_models::list_selection::{ListSelection, RowSelectionVm, SelectionMode};
#[cfg(test)]
use crate::view_models::list_selection::{RowCheckState, SelectionRow};
use crate::view_models::named_feeds::{FeedImages, NamedFeed};
use crate::view_models::pagination::pending_skeleton_count;
use crate::view_models::playlist_detail::PlaylistDetailPageVm;
use crate::view_models::recent_feeds::{
    RecentFeedResultRow, RecentFeedsPageState, RecentFeedsPageVm,
};
use crate::view_models::release_kind::ReleaseKind;
use crate::view_models::release_page::{
    LibraryTrackMenuAvailability, TrackRowMenuFacts, TrackRowMenuVm, TrackRowRemoveLabel,
};
use crate::view_models::search_results::{
    FeedResultDisplay, IndexDetailDisplay, SearchResultOrigin, TrackResultDisplay,
};
use crate::view_models::selection_action_bar::{SelectionActionBarInput, SelectionActionBarVm};
use crate::view_models::singles_group::{
    release_page_name_links, singles_groups, SinglesGroupKey, SinglesGroupPageVm, SinglesGroupRowVm,
};
use crate::view_models::text_filter::contains_normalized;
#[cfg(test)]
use crate::view_models::text_filter::normalize;
use crate::view_models::workspace::{
    ContentViewMode, ContentViewModeControlDisplay, DiscoverView, FrameHeaderActionDisplay,
    MusicDestination, MusicSidebarPreference, MusicSidebarState, ReleaseKey, TrackKey,
};
use crate::view_models::ActionStatusMessageDisplay;
use crate::views::{FeedMetadataFacts, LocalIdentityFacts, TrackRef};

const UPDATE_AVAILABLE_LABEL: &str = "Update available";
const NEW_RELEASE_LABEL: &str = "New";
const BROADCAST_ROUTE_REPAIR_AVAILABLE_LABEL: &str = "Fix routes";
const BROADCAST_ROUTE_REPAIR_WORKING_LABEL: &str = "Repairing\u{2026}";
const BROADCAST_ROUTE_REPAIRING_STATE_LABEL: &str = "Repairing routes";
const BROADCAST_ROUTE_PUBLISHER_DETAIL: &str = "The feed owner must add payment routes.";
/// ADR 0085 Decision 1: the fixed phrase of
/// `broadcast_route_repair_completion_message`'s `NoRoutesUpstream` text.
const BROADCAST_ROUTE_NO_ROUTES_UPSTREAM_PREFIX: &str =
    "The feed owner must add payment routes for";
/// ADR 0085 Decision 1: the trailing phrase of the route-repair summary
/// message, in place of "publisher routes".
const BROADCAST_ROUTE_NEEDS_PAYMENT_ROUTES_LABEL: &str = "payment routes from the feed owner";
/// ADR 0076 Decision 9: packet 004 owns the write of an out-of-date route.
/// The row has no repair action, and it names that fix.
const BROADCAST_ROUTE_OUT_OF_DATE_DETAIL: &str =
    "Payment route in file is out of date. The \"Update n file(s)\" button writes the stored route.";
const BROADCAST_REMOVED_CONFIRM_LABEL: &str = "Confirm";

/// Rendered-line threshold before descriptions start collapsed.
pub(crate) const DESCRIPTION_AUTO_COLLAPSE_LINES: usize = 5;

/// Estimated characters of one wrapped line, used to turn a source
/// line's character count into its estimated number of wrapped lines
/// (ADR 0047 item 8, amended 2026-10-16). The number is an estimate
/// before layout measurement exists, not a rendered character count.
pub(crate) const DESCRIPTION_CHARS_PER_LINE: usize = 90;

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
///
/// Each source line counts as its own estimated number of wrapped lines:
/// `ceil(chars / DESCRIPTION_CHARS_PER_LINE)`, with a minimum of one line
/// (ADR 0047 item 8, amended 2026-10-16). A long paragraph with no line
/// break thus collapses too, where a plain count of source lines never
/// would.
#[must_use]
pub(crate) fn description_line_count(description: Option<&str>) -> usize {
    let Some(description) = display_description_text(description) else {
        return 0;
    };
    description
        .lines()
        .map(|line| {
            line.chars()
                .count()
                .div_ceil(DESCRIPTION_CHARS_PER_LINE)
                .max(1)
        })
        .sum()
}

/// Return description text that is meaningful enough to display.
#[must_use]
pub(crate) fn display_description_text(description: Option<&str>) -> Option<&str> {
    description.map(str::trim).filter(|value| !value.is_empty())
}

/// "Show more" control of a collapsed, long description panel (ADR 0047
/// item 8, amended 2026-10-16).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DescriptionShowMoreDisplay {
    /// "Show more".
    pub(crate) label: &'static str,
    /// "Show the full description".
    pub(crate) a11y_label: &'static str,
}

const DESCRIPTION_SHOW_MORE_LABEL: &str = "Show more";
const DESCRIPTION_SHOW_MORE_A11Y_LABEL: &str = "Show the full description";

/// What a description panel shows (ADR 0047 item 8, amended 2026-10-16).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DescriptionDisclosureDisplay {
    /// The header shows the closed chevron and "show".
    pub(crate) collapsed: bool,
    /// The text to show. `None` shows no body.
    pub(crate) body: Option<String>,
    /// "Show more" at the end of a preview. `None` when the full text
    /// shows.
    pub(crate) show_more: Option<DescriptionShowMoreDisplay>,
}

impl DescriptionDisclosureDisplay {
    /// Project what a description panel shows, from its full text and its
    /// disclosure state (ADR 0047 item 8, amended 2026-10-16).
    ///
    /// A visible state (`AutoExpanded`, `UserExpanded`) always shows the
    /// full text, with an open header and no "Show more". A collapsed
    /// state (`AutoCollapsed`, `UserCollapsed`) shows a preview and "Show
    /// more" for a long description, or no body at all for a short one.
    #[must_use]
    pub(crate) fn project(text: &str, state: DescriptionState) -> Self {
        if state.is_visible() {
            return Self {
                collapsed: false,
                body: Some(text.to_string()),
                show_more: None,
            };
        }
        if description_line_count(Some(text)) > DESCRIPTION_AUTO_COLLAPSE_LINES {
            return Self {
                collapsed: true,
                body: Some(description_preview(text)),
                show_more: Some(DescriptionShowMoreDisplay {
                    label: DESCRIPTION_SHOW_MORE_LABEL,
                    a11y_label: DESCRIPTION_SHOW_MORE_A11Y_LABEL,
                }),
            };
        }
        Self {
            collapsed: true,
            body: None,
            show_more: None,
        }
    }
}

/// Cut `text` to its first [`DESCRIPTION_AUTO_COLLAPSE_LINES`] estimated
/// wrapped lines (ADR 0047 item 8, amended 2026-10-16).
///
/// The cut falls at the last word boundary inside that budget, and the
/// result ends with "…". The cut never falls inside a word, and it never
/// falls inside a multi-byte character: both the line budget and the
/// character budget walk `char` boundaries, never byte offsets.
fn description_preview(text: &str) -> String {
    let mut budget_lines = DESCRIPTION_AUTO_COLLAPSE_LINES;
    let mut kept_lines: Vec<&str> = Vec::new();
    for line in text.lines() {
        let wrapped = line
            .chars()
            .count()
            .div_ceil(DESCRIPTION_CHARS_PER_LINE)
            .max(1);
        if wrapped <= budget_lines {
            kept_lines.push(line);
            budget_lines -= wrapped;
            if budget_lines == 0 {
                break;
            }
            continue;
        }
        let char_budget = budget_lines * DESCRIPTION_CHARS_PER_LINE;
        let end_byte = line
            .char_indices()
            .nth(char_budget)
            .map_or(line.len(), |(byte_index, _)| byte_index);
        let candidate = &line[..end_byte];
        let cut_at = candidate
            .rfind(char::is_whitespace)
            .unwrap_or(candidate.len());
        kept_lines.push(candidate[..cut_at].trim_end());
        break;
    }
    format!("{}…", kept_lines.join("\n").trim_end())
}

/// Read the disclosure state of a keyed description, by its stored
/// toggle, or its auto projection when no toggle is stored yet (ADR 0084
/// Decision 4, ADR 0047 item 8 amended 2026-10-16).
///
/// The Index release and the Index track pages each keep one map of this
/// shape, for the session only. This function and
/// [`toggle_keyed_description_state`] carry no GPUI type, so a test calls
/// them directly, with a plain map.
#[must_use]
pub(crate) fn keyed_description_state(
    states: &BTreeMap<String, DescriptionState>,
    key: &str,
    description: &str,
) -> DescriptionState {
    states
        .get(key)
        .copied()
        .unwrap_or_else(|| DescriptionState::project(description_line_count(Some(description))))
}

/// Toggle the disclosure state of a keyed description, in place (ADR 0084
/// Decision 4). The caller's map holds the result for the session.
pub(crate) fn toggle_keyed_description_state(
    states: &mut BTreeMap<String, DescriptionState>,
    key: &str,
    description: &str,
) {
    let current = keyed_description_state(states, key, description);
    states.insert(key.to_string(), current.toggle());
}

/// Library track inspector panels that require explicit disclosure.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum InspectorPanelKind {
    /// Embedded-tag comparison panel.
    CompareId3,
    /// `MusicBrainz` lookup result panel.
    MusicBrainz,
    /// The "Inspect sources" disclosure of the track page (ADR 0083
    /// Decision 8). It is closed by default.
    Sources,
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

    /// Returns whether the "Inspect sources" disclosure is open.
    #[must_use]
    pub(crate) fn inspect_sources_open(&self) -> bool {
        self.inspector_expanded_panels
            .contains(&InspectorPanelKind::Sources)
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

/// Per-track `MusicBrainz` lookup state owned by the library screen and
/// projected into display by [`LibraryTrackRowVm`].
#[derive(Clone, Debug)]
pub(crate) enum MbTrackStatus {
    Pending,
    Processing,
    Done(usize),
    Skipped,
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
    /// The feeds that this album names, in their RSS sequence (ADR 0085
    /// Decisions 5 and 6, carries ADR 0077 Decision 2, packet 004). Empty
    /// when the album names no feed.
    pub(crate) named_feeds: Vec<NamedFeed>,
    pub(crate) identity_facts: LocalIdentityFacts,
    pub(crate) metadata_facts: Box<FeedMetadataFacts>,
    /// The stored values of the feed from the stored value projection
    /// (ADR 0075 packet 020). `None` for an album without a feed row.
    pub(crate) stored_values: Option<Box<FeedStoredValues>>,
    pub(crate) tracks: Vec<TrackRow>,
}

/// Top-level structure of the library sidebar — a list of artist
/// nodes, each with their albums.
#[derive(Clone, Debug, Default)]
pub(crate) struct LibraryTree {
    pub(crate) artists: Vec<ArtistNode>,
}

impl LibraryTree {
    /// The channel image of each feed that an album of the tree names
    /// (ADR 0084 Decision 12), in tree order. The Artists rows and the
    /// singles groups read the image of a feed from this one map.
    #[must_use]
    pub(crate) fn feed_images(&self) -> FeedImages<'_> {
        FeedImages::from_feeds(
            self.artists
                .iter()
                .flat_map(|artist| artist.albums.iter())
                .flat_map(|album| album.named_feeds.iter()),
        )
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
    /// The first four images of each playlist, by playlist id (ADR 0084
    /// Decision 12).
    playlist_covers: BTreeMap<i64, Vec<String>>,
    playlist_tracks: Vec<TrackRow>,
    mb_status: BTreeMap<i64, MbTrackStatus>,
    staged_musicbrainz: BTreeMap<i64, MusicBrainzLookupResult>,
    in_flight_feed_checks: HashSet<i64>,
    feed_update_state: FeedUpdateState,
}

/// Display-ready projection for the Playlists destination list (ADR 0084
/// Decision 2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlaylistSidebarVm {
    pub(crate) header_id: &'static str,
    pub(crate) sort_button_id: &'static str,
    pub(crate) add_button_id: &'static str,
    pub(crate) new_playlist_input_id: &'static str,
    pub(crate) new_playlist_add_button_id: &'static str,
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
    /// The art of the playlist in the logo frame (ADR 0084 Decision 12).
    pub(crate) art: CollectionArtVm,
}

/// Static labels for the Library shell chrome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LibraryChromeDisplay {
    pub(crate) playback_availability:
        Result<(), crate::application::capability::ExecutionUnavailable>,
    pub(crate) search_button_id: &'static str,
    pub(crate) artist_detail_scroll_id: &'static str,
    pub(crate) playlist_detail_scroll_id: &'static str,
    pub(crate) track_detail_scroll_id: &'static str,
    pub(crate) search_placeholder: &'static str,
    pub(crate) new_playlist_placeholder: &'static str,
    pub(crate) search_heading: &'static str,
    pub(crate) search_button_label: &'static str,
    pub(crate) empty_detail_label: &'static str,
}

impl LibraryChromeDisplay {
    #[must_use]
    pub(crate) const fn with_playback_availability(
        mut self,
        availability: Result<(), crate::application::capability::ExecutionUnavailable>,
    ) -> Self {
        self.playback_availability = availability;
        self
    }

    const VALUE: Self = Self {
        playback_availability: Ok(()),
        search_button_id: "lib-search-btn",
        artist_detail_scroll_id: "artist-detail-scroll",
        playlist_detail_scroll_id: "playlist-detail-scroll",
        track_detail_scroll_id: "track-detail-scroll",
        search_placeholder: "Search your library...",
        new_playlist_placeholder: "New playlist name\u{2026}",
        search_heading: "Search Library",
        search_button_label: "Search",
        // ADR 0084 Decision 1: the detail column's empty state names the
        // list column by its role, not the old split pane.
        empty_detail_label: "Select an item in the list.",
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
    pub(crate) state_label: Option<&'static str>,
    pub(crate) status_message: Option<String>,
    pub(crate) action: FeedUpdateActionDisplay,
}

/// Pure counts from a `Check all feeds` plus route-repair run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FeedCheckRouteRepairOutcome {
    /// Subscribed feeds checked for `MusicIndex` staleness.
    pub(crate) feeds_checked: usize,
    /// Feeds whose updates were applied before route repair.
    pub(crate) feeds_changed: usize,
    /// Feed or tag-write errors from applying feed updates.
    pub(crate) feed_update_failures: usize,
    /// Tracks whose route tag was written.
    pub(crate) route_tags_repaired: usize,
    /// Tracks that need the publisher to add routes upstream.
    pub(crate) route_tags_no_routes_upstream: usize,
    /// Tracks whose repair failed.
    pub(crate) route_tag_failures: usize,
}

impl FeedCheckRouteRepairOutcome {
    /// Creates route-repair counts for the feed-update status row.
    #[must_use]
    pub(crate) const fn new(
        feeds_checked: usize,
        feeds_changed: usize,
        feed_update_failures: usize,
        route_tags_repaired: usize,
        route_tags_no_routes_upstream: usize,
        route_tag_failures: usize,
    ) -> Self {
        Self {
            feeds_checked,
            feeds_changed,
            feed_update_failures,
            route_tags_repaired,
            route_tags_no_routes_upstream,
            route_tag_failures,
        }
    }
}

/// Completion state for one readiness-row route repair.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BroadcastRouteRepairCompletion {
    /// Local track database id.
    pub(crate) track_id: i64,
    /// Best available track title.
    pub(crate) title: Option<String>,
    /// Completion status.
    pub(crate) status: BroadcastRouteRepairCompletionStatus,
}

impl BroadcastRouteRepairCompletion {
    /// Creates a completion state for one row repair.
    #[must_use]
    pub(crate) fn new(
        track_id: i64,
        title: Option<String>,
        status: BroadcastRouteRepairCompletionStatus,
    ) -> Self {
        Self {
            track_id,
            title,
            status,
        }
    }
}

/// Typed completion status for one readiness-row route repair.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum BroadcastRouteRepairCompletionStatus {
    /// The tag was written.
    Repaired,
    /// `MusicIndex` still has no routes for the track or feed.
    NoRoutesUpstream,
    /// The file was already ready when the command checked it.
    Skipped,
    /// The command failed before it could repair the row.
    Failed {
        /// Failure reason from the command.
        reason: String,
    },
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
    pub(crate) state_label: Option<&'static str>,
}

/// Source bucket for a row in the workspace content-list frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ContentListRowSource {
    /// Row belongs to the local library.
    Library,
    /// Row is available from the remote index but is not local.
    Index,
}

impl ContentListRowSource {
    const fn from_search_result_origin(origin: SearchResultOrigin) -> Self {
        match origin {
            SearchResultOrigin::Library => Self::Library,
            SearchResultOrigin::Index => Self::Index,
        }
    }

    const fn as_search_result_origin(self) -> SearchResultOrigin {
        match self {
            Self::Library => SearchResultOrigin::Library,
            Self::Index => SearchResultOrigin::Index,
        }
    }
}

const fn row_state_label_for_source(source: ContentListRowSource) -> Option<&'static str> {
    match source {
        ContentListRowSource::Library => None,
        ContentListRowSource::Index => Some(NEW_RELEASE_LABEL),
    }
}

/// Entity kind carried by a mixed Music content row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ContentListEntityKind {
    /// Release row.
    Release,
    /// Track row.
    Track,
}

impl ContentListEntityKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Release => "Release",
            Self::Track => "Track",
        }
    }

    const fn a11y_label(self) -> &'static str {
        match self {
            Self::Release => "Entity type: Release",
            Self::Track => "Entity type: Track",
        }
    }
}

/// Display-ready entity badge for a mixed Music content row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ContentListEntityBadgeDisplay {
    /// Entity kind represented by the badge.
    pub(crate) kind: ContentListEntityKind,
    /// Visible badge label.
    pub(crate) label: &'static str,
    /// Accessibility label for assistive technologies and tooltips.
    pub(crate) a11y_label: &'static str,
}

impl ContentListEntityBadgeDisplay {
    /// The entity badge of one mixed content-list row. A release row's
    /// label names its resolved kind (ADR 0084 Decision 5): "Album", "EP"
    /// or "Single", in place of "Release". The badge kind, its color and
    /// its icon stay the row's entity type.
    fn for_kind(kind: &ContentListRowKind) -> Self {
        let entity_kind = kind.entity_kind();
        Self {
            kind: entity_kind,
            label: kind.badge_label(),
            a11y_label: entity_kind.a11y_label(),
        }
    }
}

/// Library-membership state for a mixed Music content row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ContentListLibraryBadgeState {
    /// Row is already in the local library.
    InLibrary,
    /// Row is available from the index and is not local.
    NotInLibrary,
}

impl ContentListLibraryBadgeState {
    const fn from_source(source: ContentListRowSource) -> Self {
        match source {
            ContentListRowSource::Library => Self::InLibrary,
            ContentListRowSource::Index => Self::NotInLibrary,
        }
    }

    /// The visible word. ADR 0084 Decision 8 gives the state word to each
    /// Library item only, so an Index item shows no word. The operator
    /// confirmed this on 2026-10-16: no word makes a Library item easy to
    /// see.
    const fn label(self) -> Option<&'static str> {
        match self {
            Self::InLibrary => Some("In library"),
            Self::NotInLibrary => None,
        }
    }

    const fn a11y_label(self) -> &'static str {
        match self {
            Self::InLibrary => "Library status: In library",
            Self::NotInLibrary => "Library status: Not in library",
        }
    }
}

/// Display-ready library badge for a mixed Music content row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ContentListLibraryBadgeDisplay {
    /// Library-membership state represented by the badge.
    pub(crate) state: ContentListLibraryBadgeState,
    /// Visible badge label. `None` shows no word (ADR 0084 Decision 8).
    pub(crate) label: Option<&'static str>,
    /// Accessibility label for assistive technologies and tooltips.
    pub(crate) a11y_label: &'static str,
}

impl ContentListLibraryBadgeDisplay {
    const fn for_source(source: ContentListRowSource) -> Self {
        let state = ContentListLibraryBadgeState::from_source(source);
        Self {
            state,
            label: state.label(),
            a11y_label: state.a11y_label(),
        }
    }
}

/// Typed activation for a mixed Music content row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ContentListRowActivation {
    /// The whole row activates its default drill-down.
    OpenContentRow,
    /// The row has no whole-row click target.
    None,
}

impl ContentListRowActivation {
    const fn accepts_click(self) -> bool {
        matches!(self, Self::OpenContentRow)
    }
}

/// Typed action available from one mixed Music content row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ContentListRowActionKind {
    /// Retry the payment-route tag repair for one local track.
    RepairBroadcastRoutes {
        /// Local track database id.
        track_id: i64,
    },
    /// Confirm that the show plays a track with the "removed from feed" mark
    /// (ADR 0076 Decision 7).
    ConfirmRemovedFromFeed {
        /// Local track database id.
        track_id: i64,
    },
}

/// Typed availability for a mixed Music content row action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ContentListRowActionAvailability {
    /// The action can be run.
    Available,
    /// The row already has this action in flight.
    Working,
}

impl ContentListRowActionAvailability {
    /// Returns whether a renderer should disable the control.
    #[must_use]
    pub(crate) const fn disabled(self) -> bool {
        matches!(self, Self::Working)
    }
}

/// Display-ready action for one mixed Music content row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ContentListRowActionDisplay {
    /// Typed action identity and target.
    pub(crate) kind: ContentListRowActionKind,
    /// Stable renderer id.
    pub(crate) id: String,
    /// Visible action label.
    pub(crate) label: String,
    /// Accessibility label for the action.
    pub(crate) a11y_label: String,
    /// Typed action availability.
    pub(crate) availability: ContentListRowActionAvailability,
}

impl ContentListRowActionDisplay {
    fn repair_broadcast_routes(track_id: i64, title: &str, working: bool) -> Self {
        let availability = if working {
            ContentListRowActionAvailability::Working
        } else {
            ContentListRowActionAvailability::Available
        };
        Self {
            kind: ContentListRowActionKind::RepairBroadcastRoutes { track_id },
            id: format!("broadcast-readiness-fix-routes-{track_id}"),
            label: if working {
                BROADCAST_ROUTE_REPAIR_WORKING_LABEL
            } else {
                BROADCAST_ROUTE_REPAIR_AVAILABLE_LABEL
            }
            .to_owned(),
            a11y_label: if working {
                format!("Repairing payment routes for {title}")
            } else {
                format!("Fix payment routes for {title}")
            },
            availability,
        }
    }

    fn confirm_removed_from_feed(track_id: i64, title: &str) -> Self {
        Self {
            kind: ContentListRowActionKind::ConfirmRemovedFromFeed { track_id },
            id: format!("broadcast-readiness-confirm-removed-{track_id}"),
            label: BROADCAST_REMOVED_CONFIRM_LABEL.to_owned(),
            a11y_label: format!(
                "Confirm that the show plays {title}, which was removed from its feed"
            ),
            availability: ContentListRowActionAvailability::Available,
        }
    }

    /// Returns whether a renderer should disable the control.
    #[must_use]
    pub(crate) const fn disabled(&self) -> bool {
        self.availability.disabled()
    }
}

/// Existing entity display carried by a mixed Music content row.
///
/// ADR 0062 scoped this to Release and Track rows. Artist rows, and the
/// expansion affordance both row kinds once carried, are deferred until the
/// index recency source exposes artist rows; see
/// `docs/adr/0062-content-list-tri-state-and-recency-rows.md`.
#[derive(Clone, Debug)]
pub(crate) enum ContentListRowKind {
    /// Release/feed search-result display.
    Release(FeedResultDisplay),
    /// Track search-result display.
    Track(TrackResultDisplay),
}

impl ContentListRowKind {
    /// Returns the entity kind for this row.
    #[must_use]
    pub(crate) const fn entity_kind(&self) -> ContentListEntityKind {
        match self {
            Self::Release(_) => ContentListEntityKind::Release,
            Self::Track(_) => ContentListEntityKind::Track,
        }
    }

    fn id(&self) -> &str {
        match self {
            Self::Release(display) => &display.id,
            Self::Track(display) => &display.id,
        }
    }

    fn label(&self) -> &str {
        match self {
            Self::Release(display) => &display.label,
            Self::Track(display) => &display.label,
        }
    }

    fn secondary_text(&self) -> &str {
        match self {
            Self::Release(display) => &display.secondary_text,
            Self::Track(display) => &display.secondary_text,
        }
    }

    fn thumbnail_href(&self) -> Option<&str> {
        match self {
            Self::Release(display) => display.thumbnail_href.as_deref(),
            Self::Track(display) => display.thumbnail_href.as_deref(),
        }
    }

    fn a11y_label(&self) -> &str {
        match self {
            Self::Release(display) => &display.a11y_label,
            Self::Track(display) => &display.a11y_label,
        }
    }

    const fn source(&self) -> ContentListRowSource {
        match self {
            Self::Release(display) => {
                ContentListRowSource::from_search_result_origin(display.origin)
            }
            Self::Track(display) => ContentListRowSource::from_search_result_origin(display.origin),
        }
    }

    /// The visible entity-badge label (ADR 0084 Decision 5): a release
    /// row names its resolved kind, and a track row names "Track".
    fn badge_label(&self) -> &'static str {
        match self {
            Self::Release(display) => release_kind_of_feed_result(display).label(),
            Self::Track(_) => ContentListEntityKind::Track.label(),
        }
    }
}

/// The kind of a release result row (ADR 0084 Decision 5). A row with no
/// loaded feed detail, for example a bare search fixture, resolves with
/// no stated value and no loaded track, which gives Album.
fn release_kind_of_feed_result(display: &FeedResultDisplay) -> ReleaseKind {
    match &display.remote_feed {
        Some(feed) => ReleaseKind::resolve(feed.release_kind.as_deref(), feed.track_count()),
        None => ReleaseKind::resolve(None, 0),
    }
}

/// The selection that a click on a content row or tile changes (ADR 0084
/// Decision 10, packet 020).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ContentListSelectionTarget {
    /// The Tracks or Needs attention list of the list column.
    ListColumn,
    /// The Discover "New releases" view of the detail column.
    DetailColumn,
}

/// Display row cached by the GPUI-free content-list page VM.
#[derive(Clone, Debug)]
pub(crate) struct ContentListRowDisplay {
    /// Stable row identifier.
    pub(crate) id: String,
    /// Existing entity display and kind represented by this row.
    pub(crate) kind: ContentListRowKind,
    /// Entity-type badge display.
    pub(crate) entity_badge: ContentListEntityBadgeDisplay,
    /// Library membership badge display.
    pub(crate) library_badge: ContentListLibraryBadgeDisplay,
    /// Local-vs-index provenance used by per-frame filtering.
    pub(crate) source: ContentListRowSource,
    /// Optional curator-facing state label for the row.
    pub(crate) state_label: Option<&'static str>,
    /// Whole-row activation behavior.
    pub(crate) activation: ContentListRowActivation,
    /// The one main fix of a Needs attention row. It shows at all times,
    /// outside the menu (ADR 0083 task 007, decision D10). `None` for each
    /// other row and in selection mode.
    pub(crate) fix: Option<ContentListRowActionDisplay>,
    /// The "⋯" menu and the right-click menu of a Tracks or Needs attention
    /// row (ADR 0083 task 007, decisions D9 and D10).
    /// [`LibraryViewModel::content_list_rows_with_selection`] fills it.
    /// `None` in selection mode.
    pub(crate) menu: Option<TrackRowMenuVm>,
    /// The readiness state of a Needs attention row. `None` for each other
    /// row.
    pub(crate) readiness_state: Option<BroadcastReadinessState>,
    /// The checkbox facts of the row (ADR 0084 Decision 10). Every row
    /// constructs with no selection; [`LibraryViewModel::content_list_rows_with_selection`]
    /// fills it for the Tracks and Needs attention lists.
    pub(crate) selection: RowSelectionVm,
}

impl ContentListRowDisplay {
    /// Creates a track content-list row display from legacy row facts.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        secondary_text: impl Into<String>,
        source: ContentListRowSource,
    ) -> Self {
        Self::from_track_result(
            TrackResultDisplay::new(id, title, source.as_search_result_origin())
                .with_secondary_text(secondary_text),
        )
    }

    /// Projects a release/feed display into the mixed content-list row contract.
    #[must_use]
    pub(crate) fn from_release_result(display: FeedResultDisplay) -> Self {
        Self::from_kind(ContentListRowKind::Release(display))
    }

    /// Projects a track display into the mixed content-list row contract.
    #[must_use]
    pub(crate) fn from_track_result(display: TrackResultDisplay) -> Self {
        Self::from_kind(ContentListRowKind::Track(display))
    }

    /// Projects a database track row into content-list display data.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn from_track(track: &TrackRow) -> Self {
        Self::from_track_with_album_image(track, track.album_image_href.as_deref())
    }

    /// Projects a database track row into content-list display data with album artwork.
    #[must_use]
    pub(crate) fn from_track_with_album_image(
        track: &TrackRow,
        album_image_href: Option<&str>,
    ) -> Self {
        let source = if track.is_in_library {
            ContentListRowSource::Library
        } else {
            ContentListRowSource::Index
        };
        let thumbnail_href = track
            .track_image_href
            .clone()
            .or_else(|| track.album_image_href.clone())
            .or_else(|| album_image_href.map(str::to_owned));
        let mut display = TrackResultDisplay::new(
            track.id.to_string(),
            track
                .track_title
                .clone()
                .or_else(|| track.feed_title.clone())
                .unwrap_or_else(|| "Untitled".to_string()),
            source.as_search_result_origin(),
        )
        .with_secondary_text(
            track
                .album_artist_name
                .clone()
                .or_else(|| track.artist_name.clone())
                .or_else(|| track.album_title.clone())
                .unwrap_or_else(|| "Unknown Artist".to_string()),
        );
        if let Some(thumbnail_href) = thumbnail_href {
            display = display.with_thumbnail_href(thumbnail_href);
        }
        Self::from_track_result(display)
    }

    /// Projects a broadcast-readiness problem into a local Music content row.
    #[must_use]
    pub(crate) fn from_broadcast_readiness_track(
        track: &BroadcastReadinessTrack,
        repairing: bool,
    ) -> Self {
        let artist = track
            .artist
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("Unknown Artist");
        let reason = broadcast_readiness_row_reason(track, repairing);
        let mut display = TrackResultDisplay::new(
            track.track_id.to_string(),
            track.title.clone(),
            SearchResultOrigin::Library,
        )
        .with_secondary_text(format!("{artist} - {reason}"));
        display.a11y_label = format!("Track: {}. {reason}", track.title);

        let mut row = Self::from_track_result(display);
        row.state_label = Some(broadcast_readiness_row_state_label(track.state, repairing));
        row.activation = ContentListRowActivation::None;
        row.fix = broadcast_readiness_row_actions(track, repairing);
        row.readiness_state = Some(track.state);
        row
    }

    /// Returns the primary visible row title.
    #[must_use]
    pub(crate) fn title(&self) -> &str {
        self.kind.label()
    }

    /// Returns the secondary visible row text.
    #[must_use]
    pub(crate) fn secondary_text(&self) -> &str {
        self.kind.secondary_text()
    }

    /// Returns the entity kind represented by this row.
    #[must_use]
    pub(crate) const fn entity_kind(&self) -> ContentListEntityKind {
        self.kind.entity_kind()
    }

    /// Returns the accessibility label owned by the underlying entity display.
    #[must_use]
    pub(crate) fn a11y_label(&self) -> &str {
        self.kind.a11y_label()
    }

    /// Returns the optional thumbnail href owned by the underlying entity display.
    #[must_use]
    pub(crate) fn thumbnail_href(&self) -> Option<&str> {
        self.kind.thumbnail_href()
    }

    /// Returns a stable renderer id for this row.
    #[must_use]
    pub(crate) fn element_id(&self) -> String {
        format!("music-content-row-{}", self.id)
    }

    /// Returns whether the row should receive a whole-row click handler.
    #[must_use]
    pub(crate) const fn accepts_row_click(&self) -> bool {
        self.activation.accepts_click()
    }

    /// The feed GUID of a Discover release row, from its `index-feed:` row
    /// id (ADR 0084 packet 020). `None` for each other row and for an
    /// empty GUID.
    #[must_use]
    pub(crate) fn index_feed_guid(&self) -> Option<&str> {
        self.id
            .strip_prefix(ContentListPageVm::INDEX_FEED_PREFIX)
            .filter(|guid| !guid.is_empty())
    }

    fn from_kind(kind: ContentListRowKind) -> Self {
        let source = kind.source();
        let entity_badge = ContentListEntityBadgeDisplay::for_kind(&kind);
        Self {
            id: kind.id().to_string(),
            entity_badge,
            library_badge: ContentListLibraryBadgeDisplay::for_source(source),
            source,
            state_label: row_state_label_for_source(source),
            activation: ContentListRowActivation::OpenContentRow,
            kind,
            fix: None,
            menu: None,
            readiness_state: None,
            selection: RowSelectionVm::none(),
        }
    }

    /// Shows this row as a Library item: "In library", and no "New" mark
    /// (ADR 0084 Decision 8).
    fn mark_in_library(&mut self) {
        self.library_badge =
            ContentListLibraryBadgeDisplay::for_source(ContentListRowSource::Library);
        self.state_label = None;
    }

    #[must_use]
    fn matches_text_filter(&self, filter: &str) -> bool {
        filter.is_empty()
            || contains_normalized(self.title(), filter)
            || contains_normalized(self.secondary_text(), filter)
    }

    fn has_available_broadcast_route_repair(&self, track_id: i64) -> bool {
        self.fix.as_ref().is_some_and(|action| {
            action.kind == (ContentListRowActionKind::RepairBroadcastRoutes { track_id })
                && !action.disabled()
        })
    }

    fn set_broadcast_route_repairing(&mut self, track_id: i64, repairing: bool) {
        if self.id != track_id.to_string() {
            return;
        }
        let repair = ContentListRowActionKind::RepairBroadcastRoutes { track_id };
        if self.fix.as_ref().is_none_or(|action| action.kind != repair) {
            return;
        }
        let title = self.title().to_owned();
        self.fix = Some(ContentListRowActionDisplay::repair_broadcast_routes(
            track_id, &title, repairing,
        ));
        self.state_label = Some(if repairing {
            BROADCAST_ROUTE_REPAIRING_STATE_LABEL
        } else {
            BroadcastReadinessState::NoRouteTag.label()
        });
    }
}

fn broadcast_readiness_row_reason(track: &BroadcastReadinessTrack, repairing: bool) -> &str {
    if repairing {
        "Repairing payment-route tag."
    } else if track.state == BroadcastReadinessState::NoRoutesUpstream {
        BROADCAST_ROUTE_PUBLISHER_DETAIL
    } else if track.state == BroadcastReadinessState::RouteOutOfDate {
        BROADCAST_ROUTE_OUT_OF_DATE_DETAIL
    } else {
        &track.reason
    }
}

const fn broadcast_readiness_row_state_label(
    state: BroadcastReadinessState,
    repairing: bool,
) -> &'static str {
    if repairing {
        BROADCAST_ROUTE_REPAIRING_STATE_LABEL
    } else {
        state.label()
    }
}

fn broadcast_readiness_row_actions(
    track: &BroadcastReadinessTrack,
    repairing: bool,
) -> Option<ContentListRowActionDisplay> {
    // ADR 0076: a RouteOutOfDate row has no repair action, because packet
    // 004 owns that file write. ADR 0083 task 007, decision D10: a row
    // shows at most its one main fix. "Remove from Library…" of a removed
    // track is an item of the row menu.
    match track.state {
        BroadcastReadinessState::NoRouteTag => {
            Some(ContentListRowActionDisplay::repair_broadcast_routes(
                track.track_id,
                &track.title,
                repairing,
            ))
        }
        BroadcastReadinessState::RemovedFromFeed => Some(
            ContentListRowActionDisplay::confirm_removed_from_feed(track.track_id, &track.title),
        ),
        BroadcastReadinessState::Ready
        | BroadcastReadinessState::NoRoutesUpstream
        | BroadcastReadinessState::NotDownloaded
        | BroadcastReadinessState::FileMissing
        | BroadcastReadinessState::RouteOutOfDate => None,
    }
}

/// The menu of a Library track row of the Tracks list or of the Needs
/// attention list (ADR 0083 task 007).
///
/// - A Tracks list row gets the items of task 006 decision D2 (D9).
/// - A Needs attention row gets "Inspect sources", and "Remove from
///   Library…" when its state is `RemovedFromFeed` (D10). The menu never
///   holds the fix of the row.
///
/// "Remove…" shows only for a downloaded track with no work in progress
/// (D7).
#[must_use]
pub(crate) fn content_list_track_menu(
    row_id: &str,
    title: &str,
    track: &TrackRow,
    work: TrackRowWork,
    readiness_state: Option<BroadcastReadinessState>,
) -> Option<TrackRowMenuVm> {
    let availability = LibraryTrackMenuAvailability::of(track, work);
    let row_id = format!("content-list-{row_id}");
    let facts = match readiness_state {
        None => TrackRowMenuFacts {
            add_to_playlist: true,
            download: availability.download,
            inspect_sources: true,
            remove: availability.remove.then_some(TrackRowRemoveLabel::Remove),
            ..TrackRowMenuFacts::new(&row_id, title, false)
        },
        Some(state) => TrackRowMenuFacts {
            inspect_sources: true,
            remove: (state == BroadcastReadinessState::RemovedFromFeed && availability.remove)
                .then_some(TrackRowRemoveLabel::RemoveFromLibrary),
            ..TrackRowMenuFacts::new(&row_id, title, false)
        },
    };
    TrackRowMenuVm::project(&facts)
}

/// Empty-state display for the content-list frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ContentListEmptyStateDisplay {
    /// Primary empty-state title.
    pub(crate) title: &'static str,
    /// Secondary empty-state explanation.
    pub(crate) secondary: &'static str,
}

impl ContentListEmptyStateDisplay {
    /// ADR 0084 Decision 8 deleted the three-state filter: the
    /// content-list frame has no filter left to report an empty state
    /// for. This is the generic "no rows" fallback.
    #[must_use]
    const fn for_no_rows() -> Self {
        Self {
            title: "No content",
            secondary: "No rows are available for this frame.",
        }
    }

    #[must_use]
    const fn for_recent_music() -> Self {
        Self {
            title: "No recent music",
            secondary: "MusicIndex did not return recent music.",
        }
    }

    const fn for_broadcast_readiness() -> Self {
        Self {
            title: "No readiness issues",
            secondary: "Local library tracks with files have payment routes.",
        }
    }
}

/// Row population source currently projected into the content-list frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ContentListPageSource {
    /// Rows projected from the local source tree.
    Tree,
    /// Rows projected from the Recent Feeds index query.
    RecentMusic,
    /// Rows projected from the broadcast readiness report.
    BroadcastReadiness,
}

/// Load state for the content-list frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ContentListPageLoadState {
    /// The first recency page is loading.
    Loading,
    /// Rows are available.
    Loaded,
    /// The active source returned no rows.
    Empty,
    /// The active source failed.
    Failed {
        /// Short operator-facing failure summary.
        message: String,
        /// Specific failure detail from the command layer.
        detail: String,
    },
}

/// Loading display for the content-list frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ContentListLoadingStateDisplay {
    /// Stable accessibility/status label.
    pub(crate) label: &'static str,
    /// Placeholder row count to render.
    pub(crate) skeleton_count: usize,
}

/// Failed-state display for the content-list frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ContentListFailedStateDisplay {
    /// Short operator-facing failure summary.
    pub(crate) title: String,
    /// Specific failure detail from the command layer.
    pub(crate) detail: String,
}

/// Current page-state display for the content-list frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ContentListPageStateDisplay {
    /// The content list is loading.
    Loading(ContentListLoadingStateDisplay),
    /// The content list has no rows to show.
    Empty(ContentListEmptyStateDisplay),
    /// The content list failed to load.
    Failed(ContentListFailedStateDisplay),
}

/// Load-more control display for the content-list frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ContentListLoadMoreDisplay {
    /// Stable button id.
    pub(crate) button_id: &'static str,
    /// Visible button label.
    pub(crate) label: &'static str,
    /// Accessibility label for the load-more action.
    pub(crate) a11y_label: &'static str,
    /// Whether the action is currently unavailable.
    pub(crate) disabled: bool,
}

/// Index feed activation selected from the Music content list.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ContentListIndexFeedSelection {
    /// Remote feed GUID parsed from the index row id.
    pub(crate) feed_guid: String,
    /// Display label captured from the selected row.
    pub(crate) label: String,
}

/// A sidebar destination icon (ADR 0084 Decision 1), semantic and
/// GPUI-free. The renderer maps each name to a named `IconName`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MusicDestinationIcon {
    Discover,
    Release,
    Artist,
    Track,
    Playlist,
    Warning,
}

/// Display data for one sidebar destination row (ADR 0084 Decisions 1 and
/// 2). The Needs attention count has no database query on the render
/// path: the Library stores it from the events of "Design".
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DestinationRowVm {
    /// The destination this row opens.
    pub(crate) destination: MusicDestination,
    /// Stable element id.
    pub(crate) element_id: &'static str,
    /// The named icon the renderer maps to `IconName`.
    pub(crate) icon: MusicDestinationIcon,
    /// The visible destination name.
    pub(crate) label: &'static str,
    /// The tooltip text, with the count when the destination has one.
    pub(crate) tooltip: String,
    /// The accessibility label, with the count when the destination has one.
    pub(crate) a11y_label: String,
    /// The destination's count, when it has one (Needs attention,
    /// Playlists).
    pub(crate) count: Option<usize>,
    /// Whether this destination is the root of the current location.
    pub(crate) selected: bool,
    /// Whether the collapsed icon shows an attention mark (ADR 0084
    /// Decision 1). Never the only sign of the count: the tooltip and the
    /// accessibility label give it in text.
    pub(crate) shows_attention_mark: bool,
}

/// Display data for the sidebar collapse button (ADR 0084 Decision 1).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SidebarCollapseButtonVm {
    /// Stable element id.
    pub(crate) element_id: &'static str,
    /// The accessibility label, naming the action the next click takes.
    pub(crate) a11y_label: &'static str,
    /// The tooltip text, naming the action the next click takes.
    pub(crate) tooltip: &'static str,
    /// The preference the next click sets.
    pub(crate) next_preference: MusicSidebarPreference,
}

/// The navigation entry a destination-list row opens (ADR 0084 Decision 2).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum DestinationListRowTarget {
    Release(ReleaseKey),
    /// The name level, by artist text (ADR 0084 Decision 7).
    Artist(String),
    /// The feed level, by feed GUID (ADR 0085 Decisions 2, 6 and 7). The
    /// row opens the feed page; it names no "publisher" word.
    Publisher(String),
}

/// One row of the Releases or Artists destination list (ADR 0084 Decision
/// 2). The Library already holds this data in its loaded tree; giving
/// these rows runs no new database query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DestinationListRowVm {
    /// Stable element id.
    pub(crate) element_id: String,
    /// The primary row label: the release or artist name.
    pub(crate) label: String,
    /// The secondary row text.
    pub(crate) secondary_text: String,
    /// The accessibility label for the row.
    pub(crate) a11y_label: String,
    /// The row's thumbnail href, when it has one.
    pub(crate) thumbnail_href: Option<String>,
    /// The art of a collection row in the logo frame (ADR 0084 Decision
    /// 12). `None` for a release row, which shows its cover.
    pub(crate) art: Option<CollectionArtVm>,
    /// The entry the row opens.
    pub(crate) open: DestinationListRowTarget,
    /// The checkbox facts of the row (ADR 0084 Decision 10). The
    /// Artists list, which has no selection model, keeps this at
    /// [`RowSelectionVm::none`]. [`LibraryViewModel::releases_list_with_selection`]
    /// fills it for the Releases list.
    pub(crate) selection: RowSelectionVm,
}

/// One entry of the Releases destination list (ADR 0084 Decisions 2 and
/// 6).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ReleasesListEntryVm {
    /// A release that is not in a singles group.
    Release(DestinationListRowVm),
    /// A Library singles group.
    SinglesGroup(SinglesGroupRowVm),
}

impl ReleasesListEntryVm {
    /// The sort label: the release name or the display artist.
    #[must_use]
    pub(crate) fn label(&self) -> &str {
        match self {
            Self::Release(row) => &row.label,
            Self::SinglesGroup(row) => &row.label,
        }
    }
}

/// The row identity of the Releases list (Required correction item 2,
/// 2026-10-18): a release row, or a singles group row. Each is its own
/// keyboard focus row and its own Shift+click range endpoint. The
/// selected set holds [`ReleaseKey`] members, not this type: a release
/// row has one member, and a group row has each single of its group
/// (ADR 0084 Decision 6).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ReleasesRowId {
    /// A release row. An open group also gives this row for each of its
    /// singles, after the group row (Required correction item 1).
    Release(ReleaseKey),
    /// A singles group row, closed or open.
    Group(SinglesGroupKey),
}

/// What Right or Left does on the focused Releases row (ADR 0084
/// Decisions 6 and 10, operator check of 2026-10-19: a group opened only
/// with a click on its chevron).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ReleasesDisclosureStep {
    /// Right on a closed group row opens the group.
    Open(SinglesGroupKey),
    /// Left on an open group row closes the group.
    Close(SinglesGroupKey),
    /// Left on a single row of an open group moves the focus to the group
    /// row.
    FocusGroup(SinglesGroupKey),
    /// Each other case changes nothing.
    None,
}

/// The disclosure step of Right (`open` true) or Left (`open` false) on
/// the `focused` row. `entries` is the present Releases list.
#[must_use]
pub(crate) fn releases_disclosure_step(
    focused: Option<&ReleasesRowId>,
    open: bool,
    entries: &[ReleasesListEntryVm],
) -> ReleasesDisclosureStep {
    let groups = entries.iter().filter_map(|entry| match entry {
        ReleasesListEntryVm::SinglesGroup(group) => Some(group),
        ReleasesListEntryVm::Release(_) => None,
    });
    match focused {
        Some(ReleasesRowId::Group(key)) => {
            let Some(group) = groups.into_iter().find(|group| &group.open == key) else {
                return ReleasesDisclosureStep::None;
            };
            match (open, group.expanded) {
                (true, false) => ReleasesDisclosureStep::Open(key.clone()),
                (false, true) => ReleasesDisclosureStep::Close(key.clone()),
                _ => ReleasesDisclosureStep::None,
            }
        }
        Some(ReleasesRowId::Release(release)) if !open => groups
            .into_iter()
            .find(|group| group.expanded && group.single_keys.contains(release))
            .map_or(ReleasesDisclosureStep::None, |group| {
                ReleasesDisclosureStep::FocusGroup(group.open.clone())
            }),
        _ => ReleasesDisclosureStep::None,
    }
}

/// The Music list column's trailing header action (ADR 0084 Decision 10
/// Amendment, 2026-10-19): "Select" in [`SelectionMode::Normal`], "Done"
/// in [`SelectionMode::Selecting`]. Gives `None` for a destination with
/// no selection model, and for an empty list.
#[must_use]
pub(crate) fn list_column_header_action(
    destination: MusicDestination,
    mode: SelectionMode,
    list_has_rows: bool,
) -> Option<FrameHeaderActionDisplay> {
    let (element_id, noun) = match destination {
        MusicDestination::Releases => ("music-list-column-select-releases", "releases"),
        MusicDestination::Tracks => ("music-list-column-select-tracks", "tracks"),
        MusicDestination::NeedsAttention => ("music-list-column-select-needs-attention", "tracks"),
        MusicDestination::Discover | MusicDestination::Artists | MusicDestination::Playlists => {
            return None;
        }
    };
    if !list_has_rows {
        return None;
    }
    Some(match mode {
        SelectionMode::Normal => {
            FrameHeaderActionDisplay::new(element_id, "Select", format!("Select {noun}"))
        }
        SelectionMode::Selecting => {
            FrameHeaderActionDisplay::new(element_id, "Done", format!("Done selecting {noun}"))
        }
    })
}

/// The release row of one Library album in the Releases list (ADR 0084
/// Decisions 2 and 5). `None` when the album has no release key.
#[must_use]
pub(crate) fn destination_release_row(
    artist: &ArtistNode,
    album: &AlbumNode,
) -> Option<DestinationListRowVm> {
    let key = album.release_key()?;
    Some(DestinationListRowVm {
        element_id: format!("music-release-row-{}", release_row_slug(&key)),
        label: album.name.clone(),
        // ADR 0084 Decision 5: the row names the kind after the artist,
        // for example "Haleen · Single".
        secondary_text: format!("{} · {}", artist.name, album.release_kind().label()),
        a11y_label: format!("Release: {}", album.name),
        thumbnail_href: album.image_href.clone(),
        art: None,
        open: DestinationListRowTarget::Release(key),
        selection: RowSelectionVm::none(),
    })
}

/// One row of the Artists destination list, for one artist identity (ADR
/// 0084 Decision 7, ADR 0085 Decisions 2 and 6). `members` is each
/// `(artist, album)` pair of the Library tree that resolves to `identity`.
/// A feed row's label is the feed title, from the named feed of one of
/// its members; with no stored title, it is the feed GUID. The element id
/// of a feed row comes from its feed GUID (ADR 0085 Decision 2), so a
/// feed row and a name row with the same label stay two rows.
/// The mark of an Artists list row that groups releases by their artist
/// text (operator decision of 2026-10-18). It tells a name row from a feed
/// row with the same label. It names the source of the group, not a
/// confirmation level (ADR 0085 Decision 2).
const ARTIST_NAME_ROW_MARK: &str = "by name";
/// The separator between the parts of an Artists list row text.
const ARTIST_ROW_SEPARATOR: &str = " \u{b7} ";

/// The art of an Artists row in the logo frame (ADR 0084 Decision 12): the
/// newest four images of its member albums. The date of an album is the
/// latest `pub_date` of its tracks, and an album with no date comes last.
/// Packet 024 adds the image that an artist feed gives.
/// The art of one Artists row (ADR 0084 Decision 12). `provided` is the
/// channel image of the artist feed. A name row gives `None`.
fn artist_row_art(
    provided: Option<&str>,
    members: &[(&ArtistNode, &AlbumNode)],
) -> CollectionArtVm {
    let mut albums: Vec<&AlbumNode> = members.iter().map(|(_, album)| *album).collect();
    albums.sort_by_key(|album| {
        std::cmp::Reverse(album.tracks.iter().filter_map(|track| track.pub_date).max())
    });
    CollectionArtVm::resolve(
        provided,
        albums
            .iter()
            .filter_map(|album| album.image_href.as_deref()),
        CollectionKind::Artist,
    )
}

fn artist_list_row(
    identity: ArtistIdentity,
    members: &[(&ArtistNode, &AlbumNode)],
    feed_images: &FeedImages<'_>,
) -> DestinationListRowVm {
    let secondary_text = format!("{} release{}", members.len(), plural(members.len()));
    match identity {
        ArtistIdentity::Name(name) => DestinationListRowVm {
            element_id: format!("music-artist-row-name:{}", release_row_slug_text(&name)),
            label: name.clone(),
            secondary_text: format!("{secondary_text}{ARTIST_ROW_SEPARATOR}{ARTIST_NAME_ROW_MARK}"),
            a11y_label: format!("Artist: {name}, {ARTIST_NAME_ROW_MARK}"),
            thumbnail_href: None,
            art: Some(artist_row_art(None, members)),
            open: DestinationListRowTarget::Artist(name),
            // ADR 0084 Decision 10 Exclusions: the Artists list has no
            // selection model, so its row never shows a checkbox.
            selection: RowSelectionVm::none(),
        },
        ArtistIdentity::Publisher(feed_guid) => {
            let title = members
                .iter()
                .flat_map(|(_, album)| album.named_feeds.iter())
                .find_map(|feed| {
                    (feed.feed_guid == feed_guid)
                        .then(|| feed.title.clone())
                        .flatten()
                })
                .unwrap_or_else(|| feed_guid.clone());
            let art = Some(artist_row_art(feed_images.get(&feed_guid), members));
            DestinationListRowVm {
                element_id: format!("music-artist-row-publisher:{feed_guid}"),
                label: title.clone(),
                secondary_text,
                a11y_label: format!("Artist: {title}"),
                thumbnail_href: None,
                art,
                open: DestinationListRowTarget::Publisher(feed_guid),
                selection: RowSelectionVm::none(),
            }
        }
    }
}

/// One row of the Discover views list column (ADR 0084 Decision 3). "New
/// releases" is the only view today.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DiscoverViewRowVm {
    /// The view this row opens.
    pub(crate) view: DiscoverView,
    /// Stable element id.
    pub(crate) element_id: String,
    /// The visible view name.
    pub(crate) label: &'static str,
    /// The view's subtitle.
    pub(crate) subtitle: &'static str,
    /// The accessibility label for the row.
    pub(crate) a11y_label: String,
    /// Whether this view is the open page of the current location.
    pub(crate) selected: bool,
}

/// GPUI-free page VM for a workspace content-list frame.
#[derive(Clone, Debug)]
pub(crate) struct ContentListPageVm {
    view_mode: ContentViewMode,
    text_filter: Option<String>,
    cached_rows: Vec<ContentListRowDisplay>,
    library_rows: Vec<ContentListRowDisplay>,
    source: ContentListPageSource,
    load_state: ContentListPageLoadState,
    loading: bool,
    has_more: bool,
}

impl ContentListPageVm {
    const PAGE_ID: &'static str = "music-content-list-page";
    const ROWS_ID: &'static str = "music-content-list-rows";
    const TILES_ID: &'static str = "music-content-list-tiles";
    const LOAD_MORE_BUTTON_ID: &'static str = "music-content-list-load-more";
    const LOAD_MORE_LABEL: &'static str = "Load more";
    const LOAD_MORE_A11Y_LABEL: &'static str = "Load more recent music";
    const LOADING_LABEL: &'static str = "Loading recent music";
    const INDEX_FEED_PREFIX: &'static str = "index-feed:";

    /// Creates a content-list page VM with cached rows and the default filter.
    #[must_use]
    pub(crate) fn new(cached_rows: Vec<ContentListRowDisplay>) -> Self {
        let library_rows = cached_rows.clone();
        let load_state = if cached_rows.is_empty() {
            ContentListPageLoadState::Empty
        } else {
            ContentListPageLoadState::Loaded
        };
        Self {
            view_mode: ContentViewMode::default(),
            text_filter: None,
            cached_rows,
            library_rows,
            source: ContentListPageSource::Tree,
            load_state,
            loading: false,
            has_more: false,
        }
    }

    /// Projects database track rows into a content-list page VM.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn from_tracks(tracks: &[TrackRow]) -> Self {
        Self::new(
            tracks
                .iter()
                .map(ContentListRowDisplay::from_track)
                .collect(),
        )
    }

    /// Marks the content-list frame as loading recent music.
    pub(crate) fn begin_recent_music_load(&mut self, append: bool) {
        self.source = ContentListPageSource::RecentMusic;
        self.loading = true;
        if !append {
            self.cached_rows.clear();
            self.has_more = false;
            self.load_state = ContentListPageLoadState::Loading;
        }
    }

    /// Projects the current Recent Feeds page into Music content rows.
    pub(crate) fn replace_recent_feeds_page(&mut self, page: &RecentFeedsPageVm) {
        self.source = ContentListPageSource::RecentMusic;
        self.loading = page.is_loading();
        self.has_more = page.has_more();
        match page.state() {
            RecentFeedsPageState::Loading => {
                if self.cached_rows.is_empty() {
                    self.load_state = ContentListPageLoadState::Loading;
                }
            }
            RecentFeedsPageState::Loaded(rows) => {
                self.cached_rows = content_list_rows_from_recent_feeds(rows);
                self.load_state = if self.cached_rows.is_empty() {
                    ContentListPageLoadState::Empty
                } else {
                    ContentListPageLoadState::Loaded
                };
            }
            RecentFeedsPageState::Error { message, detail } => {
                self.cached_rows.clear();
                self.load_state = ContentListPageLoadState::Failed {
                    message: message.clone(),
                    detail: detail.clone(),
                };
            }
        }
    }

    /// Replaces tree-derived rows when the tree source is active.
    /// Returns whether `row_id` names a Library track row of this page
    /// (ADR 0084 Decision 4). Only such a row opens a track page by its
    /// Library track id.
    #[must_use]
    pub(crate) fn is_library_track_row(&self, row_id: &str) -> bool {
        self.cached_rows
            .iter()
            .chain(&self.library_rows)
            .any(|row| {
                row.id == row_id
                    && row.source == ContentListRowSource::Library
                    && matches!(row.kind, ContentListRowKind::Track(_))
            })
    }

    pub(crate) fn replace_tree_rows(&mut self, cached_rows: Vec<ContentListRowDisplay>) {
        if self.source == ContentListPageSource::Tree {
            self.replace_rows(cached_rows);
        } else {
            self.library_rows = cached_rows;
        }
    }

    /// Returns the selected content presentation mode.
    #[must_use]
    pub(crate) const fn view_mode(&self) -> ContentViewMode {
        self.view_mode
    }

    /// Sets the selected content presentation mode.
    pub(crate) fn set_view_mode(&mut self, view_mode: ContentViewMode) {
        self.view_mode = view_mode;
    }

    /// Returns the frame-local content-list view-mode control display.
    #[must_use]
    pub(crate) fn view_mode_control_display(&self) -> ContentViewModeControlDisplay {
        ContentViewModeControlDisplay::default_for_content_list(self.view_mode)
    }

    /// Replaces the cached rows while preserving the frame-local filter.
    pub(crate) fn replace_rows(&mut self, cached_rows: Vec<ContentListRowDisplay>) {
        self.library_rows.clone_from(&cached_rows);
        self.cached_rows = cached_rows;
        self.source = ContentListPageSource::Tree;
        self.load_state = if self.cached_rows.is_empty() {
            ContentListPageLoadState::Empty
        } else {
            ContentListPageLoadState::Loaded
        };
        self.loading = false;
        self.has_more = false;
    }

    /// Replaces the page with broadcast-readiness problem rows.
    pub(crate) fn replace_broadcast_readiness_rows(&mut self, rows: Vec<ContentListRowDisplay>) {
        self.library_rows.clone_from(&rows);
        self.cached_rows = rows;
        self.source = ContentListPageSource::BroadcastReadiness;
        self.text_filter = None;
        self.view_mode = ContentViewMode::List;
        self.load_state = if self.cached_rows.is_empty() {
            ContentListPageLoadState::Empty
        } else {
            ContentListPageLoadState::Loaded
        };
        self.loading = false;
        self.has_more = false;
    }

    /// Returns every cached row before filtering.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn cached_rows(&self) -> &[ContentListRowDisplay] {
        &self.cached_rows
    }

    /// Returns the active row population source.
    #[must_use]
    #[cfg(test)]
    pub(crate) const fn source(&self) -> ContentListPageSource {
        self.source
    }

    /// Returns whether the active source is loading.
    #[must_use]
    pub(crate) const fn is_loading(&self) -> bool {
        self.loading
    }

    /// Returns whether the active source reports another page.
    #[must_use]
    pub(crate) const fn has_more(&self) -> bool {
        self.has_more
    }

    /// Returns every visible row. ADR 0084 Decision 3: "New releases" (the
    /// `RecentMusic` source) shows the Index releases only, newest first.
    /// Decision 8: a release that is also in the Library shows its state
    /// word, through `mark_library_releases`. The Library tracks do not
    /// join the recency page.
    #[must_use]
    pub(crate) fn visible_rows(&self) -> Vec<&ContentListRowDisplay> {
        let mut rows = Vec::new();
        let mut seen_ids = BTreeSet::new();
        Self::push_rows(&mut rows, &mut seen_ids, &self.cached_rows);

        rows.into_iter()
            .filter(|row| {
                self.text_filter
                    .as_deref()
                    .is_none_or(|filter| row.matches_text_filter(filter))
            })
            .collect()
    }

    fn push_rows<'a>(
        target: &mut Vec<&'a ContentListRowDisplay>,
        seen_ids: &mut BTreeSet<&'a str>,
        source_rows: &'a [ContentListRowDisplay],
    ) {
        for row in source_rows {
            if seen_ids.insert(row.id.as_str()) {
                target.push(row);
            }
        }
    }

    /// Returns stable row identifiers visible under the current filter.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn visible_row_ids(&self) -> Vec<&str> {
        self.visible_rows()
            .into_iter()
            .map(|row| row.id.as_str())
            .collect()
    }

    /// Returns loading placeholders for the active content-list load.
    #[must_use]
    pub(crate) fn loading_display(&self) -> Option<ContentListLoadingStateDisplay> {
        self.is_loading().then(|| ContentListLoadingStateDisplay {
            label: Self::LOADING_LABEL,
            skeleton_count: pending_skeleton_count(self.is_loading(), !self.cached_rows.is_empty()),
        })
    }

    /// Returns the current content-list state display, when one should render.
    #[must_use]
    pub(crate) fn page_state_display(&self) -> Option<ContentListPageStateDisplay> {
        match &self.load_state {
            ContentListPageLoadState::Failed { message, detail } => Some(
                ContentListPageStateDisplay::Failed(ContentListFailedStateDisplay {
                    title: message.clone(),
                    detail: detail.clone(),
                }),
            ),
            ContentListPageLoadState::Loading
                if self.visible_rows().is_empty() && self.is_loading() =>
            {
                self.loading_display()
                    .map(ContentListPageStateDisplay::Loading)
            }
            ContentListPageLoadState::Loaded
            | ContentListPageLoadState::Empty
            | ContentListPageLoadState::Loading => self.visible_rows().is_empty().then(|| {
                let empty = if self.source == ContentListPageSource::BroadcastReadiness
                    && self.cached_rows.is_empty()
                {
                    ContentListEmptyStateDisplay::for_broadcast_readiness()
                } else if self.cached_rows.is_empty()
                    && self.source == ContentListPageSource::RecentMusic
                {
                    ContentListEmptyStateDisplay::for_recent_music()
                } else {
                    ContentListEmptyStateDisplay::for_no_rows()
                };
                ContentListPageStateDisplay::Empty(empty)
            }),
        }
    }

    /// Returns the load-more control display when the active source has another page.
    #[must_use]
    pub(crate) fn load_more_display(&self) -> Option<ContentListLoadMoreDisplay> {
        if self.has_more() {
            Some(ContentListLoadMoreDisplay {
                button_id: Self::LOAD_MORE_BUTTON_ID,
                label: Self::LOAD_MORE_LABEL,
                a11y_label: Self::LOAD_MORE_A11Y_LABEL,
                disabled: self.loading,
            })
        } else {
            None
        }
    }

    /// Returns row thumbnail URLs keyed by content-list row id.
    #[must_use]
    pub(crate) fn thumbnail_sources(&self) -> Vec<(String, String)> {
        let mut sources = Vec::new();
        let mut seen_ids = BTreeSet::new();
        Self::push_thumbnail_sources(&mut sources, &mut seen_ids, &self.cached_rows);
        sources
    }

    /// Marks each Index release row whose feed GUID the Library holds as
    /// "In library" (ADR 0084 Decision 8). Such a row is not "New".
    pub(crate) fn mark_library_releases(&mut self, library_feed_guids: &BTreeSet<String>) {
        for row in &mut self.cached_rows {
            let Some(feed_guid) = row.id.strip_prefix(Self::INDEX_FEED_PREFIX) else {
                continue;
            };
            if library_feed_guids.contains(feed_guid) {
                row.mark_in_library();
            }
        }
    }

    fn push_thumbnail_sources<'a>(
        target: &mut Vec<(String, String)>,
        seen_ids: &mut BTreeSet<&'a str>,
        rows: &'a [ContentListRowDisplay],
    ) {
        for row in rows {
            if !seen_ids.insert(row.id.as_str()) {
                continue;
            }
            if let Some(href) = row.thumbnail_href() {
                target.push((row.id.clone(), href.to_string()));
            }
        }
    }

    fn has_available_broadcast_route_repair(&self, track_id: i64) -> bool {
        self.cached_rows
            .iter()
            .chain(self.library_rows.iter())
            .any(|row| row.has_available_broadcast_route_repair(track_id))
    }

    fn set_broadcast_route_repairing(&mut self, track_id: i64, repairing: bool) {
        for row in &mut self.cached_rows {
            row.set_broadcast_route_repairing(track_id, repairing);
        }
        for row in &mut self.library_rows {
            row.set_broadcast_route_repairing(track_id, repairing);
        }
    }

    /// Returns an Index feed selection for a release row, when present.
    #[must_use]
    pub(crate) fn index_feed_selection(
        &self,
        row_id: &str,
    ) -> Option<ContentListIndexFeedSelection> {
        let row = self.cached_rows.iter().find(|row| row.id == row_id)?;
        let ContentListRowKind::Release(release) = &row.kind else {
            return None;
        };
        let feed_guid = row_id.strip_prefix(Self::INDEX_FEED_PREFIX)?.to_string();
        Some(ContentListIndexFeedSelection {
            feed_guid,
            label: release.label.clone(),
        })
    }

    /// Projects a selected recency row into remote Index feed detail.
    #[must_use]
    pub(crate) fn index_feed_detail(
        &self,
        activation_id: &str,
        fallback_id: &str,
        fallback_label: &str,
    ) -> Option<IndexDetailDisplay> {
        let row = self.cached_rows.iter().find_map(|row| match &row.kind {
            ContentListRowKind::Release(release) if row.id == activation_id => Some(release),
            ContentListRowKind::Release(_) | ContentListRowKind::Track(_) => None,
        })?;
        Some(IndexDetailDisplay::feed_or_fallback(
            Some(row),
            fallback_id,
            fallback_label,
        ))
    }

    /// Stable page element id for the renderer.
    #[must_use]
    pub(crate) const fn page_id() -> &'static str {
        Self::PAGE_ID
    }

    /// Stable scroll/list element id for the renderer.
    #[must_use]
    pub(crate) const fn rows_id() -> &'static str {
        Self::ROWS_ID
    }

    /// Stable tile-grid element id for the renderer.
    #[must_use]
    pub(crate) const fn tiles_id() -> &'static str {
        Self::TILES_ID
    }

    /// Stable skeleton row element id for the renderer.
    #[must_use]
    pub(crate) const fn skeleton_row_id(index: usize) -> (&'static str, usize) {
        ("music-content-loading-row", index)
    }

    /// Stable skeleton tile element id for the renderer.
    #[must_use]
    pub(crate) const fn skeleton_tile_id(index: usize) -> (&'static str, usize) {
        ("music-content-loading-tile", index)
    }
}

/// `set_text_filter` and `text_filter` are ADR 0048 active-frame text-filter
/// infrastructure with no production caller: no screen wires a frame-local
/// search box to this page yet. Several of this type's own tests need them
/// as direct helpers to exercise the live `replace_rows`
/// composition, so they stay here, compiled only for tests.
#[cfg(test)]
impl ContentListPageVm {
    pub(crate) fn set_text_filter(&mut self, filter: Option<String>) {
        self.text_filter = normalize(filter);
    }

    /// Returns the active text filter, when set.
    #[must_use]
    pub(crate) fn text_filter(&self) -> Option<&str> {
        self.text_filter.as_deref()
    }
}

impl<'a> LibraryTrackActionVm<'a> {
    #[must_use]
    pub(crate) const fn new(subscription_message: Option<&'a str>) -> Self {
        Self {
            subscription_message,
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
    // Sidebar sort.
    playlist_sort: PlaylistSort,
    content_list_page: ContentListPageVm,
    /// The sidebar collapse preference (ADR 0084 Decision 1). Session
    /// state only; never written to `config.toml`.
    sidebar_preference: MusicSidebarPreference,
    /// The destination whose list the list column currently shows (ADR
    /// 0084 Decision 2). The top app sets this once, when a destination
    /// becomes the current location; it carries no history, so it is not
    /// the navigation stack `adr_0084_nav_library_screens_keep_no_navigation_stack`
    /// forbids.
    active_destination: MusicDestination,
    /// The Needs attention count (ADR 0084 Decision 2), kept off the
    /// render path: the Library stores it from `show_needs_attention_report`
    /// and `refresh_needs_attention_report`, at the load, feed-check,
    /// repair, download, and removal events "Design" names.
    needs_attention_count: usize,
    album_description_states: BTreeMap<i64, DescriptionState>,
    track_description_states: BTreeMap<i64, DescriptionState>,
    // Selection / focus.
    selected_id: Option<i64>,
    selected_playlist_id: Option<i64>,
    hovered_thumb_url: Option<String>,
    // Operation state.
    busy: BusyItems,
    in_flight_broadcast_route_repairs: BTreeSet<i64>,
    library_removal: LibraryRemovalConfirmationState,
    status: String,
    observation_receipts: BTreeMap<i64, crate::provider_observation::ObservationReceipt>,
    observation_failures:
        BTreeMap<i64, std::sync::Arc<crate::provider_observation::ObservationWriteFailure>>,
    library_loading: bool,
    // Playlist creation.
    creating_playlist: bool,
    renaming_playlist_id: Option<i64>,
}

impl LibraryViewModel {
    /// Construct a view-model with the legacy `LibraryApp::new`
    /// defaults: empty tree + snapshots, the sidebar auto-collapsing,
    /// sort by name, no selection, no operation in flight.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            snapshot: LibrarySnapshot::default(),
            playlist_sort: PlaylistSort::default(),
            content_list_page: ContentListPageVm::new(Vec::new()),
            sidebar_preference: MusicSidebarPreference::default(),
            active_destination: MusicDestination::Discover,
            needs_attention_count: 0,
            album_description_states: BTreeMap::new(),
            track_description_states: BTreeMap::new(),
            selected_id: None,
            selected_playlist_id: None,
            hovered_thumb_url: None,
            busy: BusyItems::default(),
            in_flight_broadcast_route_repairs: BTreeSet::new(),
            library_removal: LibraryRemovalConfirmationState::new(),
            status: String::new(),
            observation_failures: BTreeMap::new(),
            observation_receipts: BTreeMap::new(),
            library_loading: false,
            creating_playlist: false,
            renaming_playlist_id: None,
        }
    }

    #[must_use]
    pub(crate) fn tree(&self) -> &LibraryTree {
        &self.snapshot.tree
    }

    pub(crate) fn replace_tree(&mut self, tree: LibraryTree) {
        self.content_list_page
            .replace_tree_rows(content_list_rows_from_tree(&tree));
        self.snapshot.tree = tree;
        self.mark_recent_library_releases();
    }

    /// Gives each "New releases" row that the Library holds its state word
    /// (ADR 0084 Decision 8).
    fn mark_recent_library_releases(&mut self) {
        let feed_guids: BTreeSet<String> = self
            .snapshot
            .tree
            .artists
            .iter()
            .flat_map(|artist| &artist.albums)
            .filter_map(|album| album.feed_guid.clone())
            .collect();
        self.content_list_page.mark_library_releases(&feed_guids);
    }

    /// Shows the Library tracks, by the Tracks destination (ADR 0084
    /// Decision 2). It reuses the already-loaded tree: no new query.
    pub(crate) fn show_tracks_destination(&mut self) {
        self.content_list_page
            .replace_rows(content_list_rows_from_tree(&self.snapshot.tree));
    }

    #[must_use]
    pub(crate) fn sidebar_preference(&self) -> MusicSidebarPreference {
        self.sidebar_preference
    }

    pub(crate) fn set_sidebar_preference(&mut self, preference: MusicSidebarPreference) {
        self.sidebar_preference = preference;
    }

    /// The destination whose list the list column currently shows (ADR
    /// 0084 Decision 2).
    #[must_use]
    pub(crate) const fn active_destination(&self) -> MusicDestination {
        self.active_destination
    }

    /// Records the destination whose list the list column now shows (ADR
    /// 0084 Decision 2).
    pub(crate) fn set_active_destination(&mut self, destination: MusicDestination) {
        self.active_destination = destination;
    }

    /// Always shows the readiness rows of `report`, whatever the
    /// previously active destination list (ADR 0084 Decision 2: opening
    /// Needs attention, from the sidebar or from Show).
    pub(crate) fn show_needs_attention_report(&mut self, report: &BroadcastReadinessReport) {
        self.needs_attention_count = report.problem_count();
        self.content_list_page
            .replace_broadcast_readiness_rows(self.broadcast_readiness_rows(report));
    }

    /// Keeps the Needs attention count current at the ADR 0084 Decision 2
    /// events: on load, after a feed check or a repair, and after a
    /// download or a removal. Only replaces the active list rows when
    /// Needs attention is already open, so this never hijacks another
    /// open destination's list.
    pub(crate) fn refresh_needs_attention_report(&mut self, report: &BroadcastReadinessReport) {
        self.needs_attention_count = report.problem_count();
        if self.shows_broadcast_readiness_list() {
            self.content_list_page
                .replace_broadcast_readiness_rows(self.broadcast_readiness_rows(report));
        }
    }

    fn broadcast_readiness_rows(
        &self,
        report: &BroadcastReadinessReport,
    ) -> Vec<ContentListRowDisplay> {
        report
            .problem_tracks()
            .into_iter()
            .map(|track| {
                ContentListRowDisplay::from_broadcast_readiness_track(
                    track,
                    self.in_flight_broadcast_route_repairs
                        .contains(&track.track_id),
                )
            })
            .collect()
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

    pub(crate) fn update_album_metadata_facts(
        &mut self,
        feed_id: i64,
        facts: &FeedMetadataFacts,
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
            *album.metadata_facts = facts.clone();
            updated = true;
        }
        updated
    }

    /// Replace the projected stored values of each album of one feed.
    pub(crate) fn update_album_stored_values(
        &mut self,
        feed_id: i64,
        values: &FeedStoredValues,
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
            album.stored_values = Some(Box::new(values.clone()));
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

    /// No screen reads this since the sidebar tree's loading skeleton went
    /// with ADR 0084 Decision 2. This type's own tests still read it
    /// directly.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn is_library_loading(&self) -> bool {
        self.library_loading
    }

    #[must_use]
    pub(crate) const fn content_view_mode(&self) -> ContentViewMode {
        self.content_list_page.view_mode()
    }

    pub(crate) fn set_content_view_mode(&mut self, view_mode: ContentViewMode) {
        self.content_list_page.set_view_mode(view_mode);
    }

    #[must_use]
    pub(crate) fn content_view_mode_control(&self) -> ContentViewModeControlDisplay {
        self.content_list_page.view_mode_control_display()
    }

    #[must_use]
    pub(crate) const fn content_list_page(&self) -> &ContentListPageVm {
        &self.content_list_page
    }

    #[must_use]
    pub(crate) fn shows_broadcast_readiness_list(&self) -> bool {
        self.content_list_page.source == ContentListPageSource::BroadcastReadiness
    }

    pub(crate) fn begin_recent_music_content_load(&mut self, append: bool) {
        self.content_list_page.begin_recent_music_load(append);
    }

    pub(crate) fn replace_recent_music_content(&mut self, page: &RecentFeedsPageVm) {
        self.content_list_page.replace_recent_feeds_page(page);
        self.mark_recent_library_releases();
    }

    #[must_use]
    pub(crate) fn content_list_thumbnail_sources(&self) -> Vec<(String, String)> {
        self.content_list_page.thumbnail_sources()
    }

    #[must_use]
    pub(crate) fn content_list_index_feed_selection(
        &self,
        row_id: &str,
    ) -> Option<ContentListIndexFeedSelection> {
        self.content_list_page.index_feed_selection(row_id)
    }

    /// The track key of a content-list track row, when the row id names a
    /// Library track (ADR 0084 Decision 4, Required Change 5). An Index
    /// release row's id does not parse as a track id, so this gives `None`
    /// for it.
    #[must_use]
    pub(crate) fn content_list_track_row_key(&self, row_id: &str) -> Option<TrackKey> {
        if !self.content_list_page.is_library_track_row(row_id) {
            return None;
        }
        track_row_key_in_tree(self.tree(), row_id)
    }

    #[must_use]
    pub(crate) fn content_list_index_feed_detail(
        &self,
        activation_id: &str,
        fallback_id: &str,
        fallback_label: &str,
    ) -> Option<IndexDetailDisplay> {
        self.content_list_page
            .index_feed_detail(activation_id, fallback_id, fallback_label)
    }

    #[must_use]
    pub(crate) fn playlists(&self) -> &[db::Playlist] {
        &self.snapshot.playlists
    }

    pub(crate) fn replace_playlists(&mut self, mut playlists: Vec<db::Playlist>) {
        Self::sort_playlists_by(self.playlist_sort, &mut playlists);
        self.snapshot.playlists = playlists;
    }

    /// Replaces the first four images of each playlist (ADR 0084
    /// Decision 12). One database read gives them for each playlist.
    pub(crate) fn replace_playlist_covers(&mut self, covers: BTreeMap<i64, Vec<String>>) {
        self.snapshot.playlist_covers = covers;
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
                    art: CollectionArtVm::resolve(
                        None,
                        self.snapshot
                            .playlist_covers
                            .get(&playlist.id)
                            .into_iter()
                            .flatten(),
                        CollectionKind::Playlist,
                    ),
                })
                .collect(),
        }
    }

    /// Entries of the Releases destination list, by label (ADR 0084
    /// Decisions 2 and 6). It reuses the already-loaded tree: no new query.
    ///
    /// Each Library singles group gives one group entry, and its singles
    /// give no release entry. Each other release gives one release entry.
    /// `expanded` is the set of group keys whose rows show their singles.
    /// The label of a release entry is the release name, and the label of
    /// a group entry is the display artist.
    #[must_use]
    pub(crate) fn releases_list(
        &self,
        expanded: &BTreeSet<SinglesGroupKey>,
    ) -> Vec<ReleasesListEntryVm> {
        let tree = &self.snapshot.tree;
        let groups = singles_groups(tree);
        let mut entries: Vec<ReleasesListEntryVm> =
            tree.artists
                .iter()
                .flat_map(|artist| artist.albums.iter().map(move |album| (artist, album)))
                .filter_map(|(artist, album)| destination_release_row(artist, album))
                .filter(|row| match &row.open {
                    DestinationListRowTarget::Release(key) => {
                        !groups.iter().any(|group| group.contains(key))
                    }
                    DestinationListRowTarget::Artist(_)
                    | DestinationListRowTarget::Publisher(_) => true,
                })
                .map(ReleasesListEntryVm::Release)
                .collect();
        entries.extend(groups.iter().map(|group| {
            ReleasesListEntryVm::SinglesGroup(SinglesGroupRowVm::new(
                group,
                expanded.contains(&group.key()),
                destination_release_row,
            ))
        }));
        entries.sort_by_key(|entry| entry.label().to_lowercase());
        entries
    }

    /// [`Self::releases_list`], with the Decision 10 checkbox facts of
    /// `selection` applied to each row (Required correction item 1, 2, 4
    /// and 5, 2026-10-18). A singles group row's own checkbox reads
    /// checked, mixed, or clear, from each of its singles. An open
    /// group's own single row gets its own checkbox and its own
    /// keyboard-focus fact, the same as a release row. `mode` decides
    /// whether a checkbox shows at all (ADR 0084 Decision 10 Amendment,
    /// 2026-10-19).
    #[must_use]
    pub(crate) fn releases_list_with_selection(
        &self,
        expanded: &BTreeSet<SinglesGroupKey>,
        selection: &ListSelection<ReleaseKey, ReleasesRowId>,
        mode: SelectionMode,
    ) -> Vec<ReleasesListEntryVm> {
        let selecting = mode.is_selecting();
        let focused_row = selection.focused().cloned();
        self.releases_list(expanded)
            .into_iter()
            .map(|entry| match entry {
                ReleasesListEntryVm::Release(mut row) => {
                    if let DestinationListRowTarget::Release(key) = &row.open {
                        let focused =
                            focused_row.as_ref() == Some(&ReleasesRowId::Release(key.clone()));
                        row.selection = RowSelectionVm::for_row(
                            &row.label,
                            selection.is_selected(key),
                            focused,
                            selecting,
                        );
                    }
                    ReleasesListEntryVm::Release(row)
                }
                ReleasesListEntryVm::SinglesGroup(mut group) => {
                    let check_state = selection.row_check_state(&group.single_keys);
                    let group_focused =
                        focused_row.as_ref() == Some(&ReleasesRowId::Group(group.open.clone()));
                    group.selection = RowSelectionVm::for_group_row(
                        &group.label,
                        check_state,
                        group_focused,
                        selecting,
                    );
                    // Required correction item 4: each single row of an
                    // open group shows its own checkbox, under the same
                    // visibility rule as each other row.
                    for single_row in &mut group.singles {
                        if let DestinationListRowTarget::Release(key) = &single_row.open {
                            let single_focused =
                                focused_row.as_ref() == Some(&ReleasesRowId::Release(key.clone()));
                            single_row.selection = RowSelectionVm::for_row(
                                &single_row.label,
                                selection.is_selected(key),
                                single_focused,
                                selecting,
                            );
                        }
                    }
                    ReleasesListEntryVm::SinglesGroup(group)
                }
            })
            .collect()
    }

    /// The release of `key`, when the tree still holds it.
    fn album_for_release_key(&self, key: &ReleaseKey) -> Option<&AlbumNode> {
        self.snapshot
            .tree
            .artists
            .iter()
            .flat_map(|artist| artist.albums.iter())
            .find(|album| album.release_key().as_ref() == Some(key))
    }

    /// Tells if every track of the release `key` is downloaded (ADR 0084
    /// Decision 4's "Not downloaded" state names the rest). Gives `true`
    /// for a key the tree no longer holds, so a stale selected key never
    /// blocks "Add to playlist" or "Clear".
    #[must_use]
    fn release_is_fully_downloaded(&self, key: &ReleaseKey) -> bool {
        self.album_for_release_key(key).is_none_or(|album| {
            !album.tracks.is_empty() && album.tracks.iter().all(|track| track.is_in_library)
        })
    }

    /// The Library facts of the release `key`, for the detail column
    /// selection (ADR 0084 packet 018). `None` when the tree does not hold
    /// the release, or when the release has no Library feed id.
    #[must_use]
    pub(crate) fn library_release_facts(&self, key: &ReleaseKey) -> Option<LibraryReleaseFacts> {
        let album = self.album_for_release_key(key)?;
        Some(LibraryReleaseFacts {
            feed_id: album.feed_id?,
            fully_downloaded: !album.tracks.is_empty()
                && album.tracks.iter().all(|track| track.is_in_library),
            download_active: self.busy.release_downloading(album),
        })
    }

    /// Tells if "Remove" can act on the release `key`: the release is
    /// fully downloaded, and no download job of its key is queued or in
    /// flight (orchestrator correction of 2026-10-20).
    #[must_use]
    fn release_is_removable(&self, key: &ReleaseKey) -> bool {
        self.release_is_fully_downloaded(key)
            && !self
                .album_for_release_key(key)
                .is_some_and(|album| self.busy.release_downloading(album))
    }

    /// The release that holds the Library track `track_id`, when the tree
    /// holds it.
    fn album_for_track_id(&self, track_id: i64) -> Option<&AlbumNode> {
        self.snapshot
            .tree
            .artists
            .iter()
            .flat_map(|artist| artist.albums.iter())
            .find(|album| album.tracks.iter().any(|track| track.id == track_id))
    }

    /// `true` while a download job of the release of the Library track
    /// `track_id` is queued or in flight. "Remove" of the track is then
    /// absent (orchestrator correction of 2026-10-20).
    #[must_use]
    pub(crate) fn track_release_download_active(&self, track_id: i64) -> bool {
        self.album_for_track_id(track_id)
            .is_some_and(|album| self.busy.release_downloading(album))
    }

    /// The Library track ids of each release with a queued or running
    /// download job (ADR 0084 packet 025). The playlist page reads this
    /// set one time for each render, not one tree search for each entry.
    #[must_use]
    pub(crate) fn release_download_active_track_ids(&self) -> BTreeSet<i64> {
        self.snapshot
            .tree
            .artists
            .iter()
            .flat_map(|artist| artist.albums.iter())
            .filter(|album| self.busy.release_downloading(album))
            .flat_map(|album| album.tracks.iter().map(|track| track.id))
            .collect()
    }

    /// The action bar of the Releases list selection (ADR 0084 Decision
    /// 10), or `None` outside [`SelectionMode::Selecting`] (Amendment,
    /// 2026-10-19).
    #[must_use]
    pub(crate) fn releases_selection_action_bar(
        &self,
        selection: &ListSelection<ReleaseKey, ReleasesRowId>,
        mode: SelectionMode,
    ) -> Option<SelectionActionBarVm> {
        let mut not_in_library_count = 0;
        let mut in_library_count = 0;
        for key in selection.selected_keys() {
            if !self.release_is_fully_downloaded(key) {
                not_in_library_count += 1;
            } else if self.release_is_removable(key) {
                in_library_count += 1;
            }
        }
        SelectionActionBarVm::new(
            mode,
            SelectionActionBarInput {
                selected_count: selection.selected_count(),
                item_noun: "release",
                not_in_library_count,
                in_library_count,
            },
        )
    }

    /// The Library feed id of each selected release, for "Add to
    /// playlist" (ADR 0084 Decision 10), which adds every selected
    /// release whatever its download state. Skips a key the tree no
    /// longer holds.
    #[must_use]
    pub(crate) fn releases_selected_feed_ids(
        &self,
        selection: &ListSelection<ReleaseKey, ReleasesRowId>,
    ) -> Vec<i64> {
        selection
            .selected_keys()
            .filter_map(|key| {
                self.album_for_release_key(key)
                    .and_then(|album| album.feed_id)
            })
            .collect()
    }

    /// The feed id and the release name of each selected release that is
    /// fully downloaded, for "Remove…" (ADR 0084 Decision 10: "Remove"
    /// removes only an item that is in the Library). A release with a
    /// queued or running download is not in the list.
    #[must_use]
    pub(crate) fn releases_removable_feed_ids(
        &self,
        selection: &ListSelection<ReleaseKey, ReleasesRowId>,
    ) -> Vec<(i64, String)> {
        selection
            .selected_keys()
            .filter(|key| self.release_is_removable(key))
            .filter_map(|key| {
                let album = self.album_for_release_key(key)?;
                Some((album.feed_id?, album.name.clone()))
            })
            .collect()
    }

    /// The feed id of each selected release that is not fully
    /// downloaded, newest first, for "Download n releases" (ADR 0084
    /// Decision 10).
    #[must_use]
    pub(crate) fn releases_downloadable_feed_ids(
        &self,
        selection: &ListSelection<ReleaseKey, ReleasesRowId>,
    ) -> Vec<i64> {
        let mut dated: Vec<(Option<i64>, i64)> = selection
            .selected_keys()
            .filter(|key| !self.release_is_fully_downloaded(key))
            .filter_map(|key| {
                let album = self.album_for_release_key(key)?;
                let feed_id = album.feed_id?;
                let newest = album.tracks.iter().filter_map(|track| track.pub_date).max();
                Some((newest, feed_id))
            })
            .collect();
        // Newest first; a release with no dated track sorts last.
        dated.sort_by(|left, right| match (left.0, right.0) {
            (Some(left), Some(right)) => right.cmp(&left),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });
        dated.into_iter().map(|(_, feed_id)| feed_id).collect()
    }

    /// [`ContentListPageVm::visible_rows`], with the Decision 10
    /// checkbox facts of `selection` applied to each Library track row.
    /// A row with no track key, for example an Index row, keeps
    /// [`RowSelectionVm::none`]. `mode` decides whether a checkbox shows
    /// at all (Amendment, 2026-10-19).
    #[must_use]
    pub(crate) fn content_list_rows_with_selection(
        &self,
        selection: &ListSelection<TrackKey>,
        mode: SelectionMode,
    ) -> Vec<ContentListRowDisplay> {
        let selecting = mode.is_selecting();
        self.content_list_page
            .visible_rows()
            .into_iter()
            .cloned()
            .map(|mut row| {
                if let Some(key) = self.content_list_track_row_key(&row.id) {
                    let focused = selection.focused() == Some(&key);
                    row.selection = RowSelectionVm::for_row(
                        row.title(),
                        selection.is_selected(&key),
                        focused,
                        selecting,
                    );
                    // ADR 0083 task 007, decision D12: selection mode shows
                    // no menu and no fix button.
                    if !selecting {
                        row.menu = self.content_list_row_menu(&row);
                    }
                }
                if selecting {
                    row.menu = None;
                    row.fix = None;
                }
                row
            })
            .collect()
    }

    /// The menu of a Library track row of the Tracks or Needs attention
    /// list (ADR 0083 task 007, decisions D9 and D10). The work of the row
    /// comes from [`BusyItems`]. A row whose track is not in the tree gets
    /// no menu.
    fn content_list_row_menu(&self, row: &ContentListRowDisplay) -> Option<TrackRowMenuVm> {
        let track_id: i64 = row.id.parse().ok()?;
        let album = self.album_for_track_id(track_id)?;
        let track = album.tracks.iter().find(|track| track.id == track_id)?;
        let work = self.busy.track_row_work(album, track);
        content_list_track_menu(&row.id, row.title(), track, work, row.readiness_state)
    }

    /// The work in progress on each Library track that has work, by track
    /// id (ADR 0083 task 007). The playlist page reads it one time for each
    /// render. A track that is absent has no work.
    #[must_use]
    pub(crate) fn library_track_works(&self) -> BTreeMap<i64, TrackRowWork> {
        self.snapshot
            .tree
            .artists
            .iter()
            .flat_map(|artist| artist.albums.iter())
            .flat_map(|album| {
                album
                    .tracks
                    .iter()
                    .map(move |track| (track.id, self.busy.track_row_work(album, track)))
            })
            .filter(|(_, work)| *work != TrackRowWork::Idle)
            .collect()
    }

    /// The track key of the Library track `track_id`, when the tree holds
    /// it. "Inspect sources" of a list row opens this track page.
    #[must_use]
    pub(crate) fn library_track_key(&self, track_id: i64) -> Option<TrackKey> {
        self.album_for_track_id(track_id)?
            .tracks
            .iter()
            .find(|track| track.id == track_id)
            .map(TrackRow::track_key)
    }

    /// The action bar of the Tracks or Needs attention list selection
    /// (ADR 0084 Decision 10), or `None` outside [`SelectionMode::Selecting`]
    /// (Amendment, 2026-10-19). Only a Library track row resolves a track
    /// key (`content_list_track_row_key`), so a selected track is always
    /// already in the Library, and "Download" is always absent. "Remove"
    /// counts only the tracks with no queued or running download of their
    /// release.
    #[must_use]
    pub(crate) fn content_list_selection_action_bar(
        &self,
        selection: &ListSelection<TrackKey>,
        mode: SelectionMode,
    ) -> Option<SelectionActionBarVm> {
        SelectionActionBarVm::new(
            mode,
            SelectionActionBarInput {
                selected_count: selection.selected_count(),
                item_noun: "track",
                not_in_library_count: 0,
                in_library_count: self.content_list_removable_tracks(selection).len(),
            },
        )
    }

    /// The selected tracks of [`Self::content_list_selected_tracks`] that
    /// "Remove…" can act on: each track with no queued or running download
    /// of its release (orchestrator correction of 2026-10-20).
    #[must_use]
    pub(crate) fn content_list_removable_tracks(
        &self,
        selection: &ListSelection<TrackKey>,
    ) -> Vec<(i64, String)> {
        self.content_list_selected_tracks(selection)
            .into_iter()
            .filter(|(track_id, _)| !self.track_release_download_active(*track_id))
            .collect()
    }

    /// The Library track id and the title of each selected row, in their
    /// visible sequence, for "Add to playlist" and "Remove…" (ADR 0084
    /// Decision 10).
    #[must_use]
    pub(crate) fn content_list_selected_tracks(
        &self,
        selection: &ListSelection<TrackKey>,
    ) -> Vec<(i64, String)> {
        self.content_list_page
            .visible_rows()
            .into_iter()
            .filter_map(|row| {
                let key = self.content_list_track_row_key(&row.id)?;
                if !selection.is_selected(&key) {
                    return None;
                }
                let track_id = row.id.parse::<i64>().ok()?;
                Some((track_id, row.title().to_owned()))
            })
            .collect()
    }

    /// The name links of a release page with the link to the Library
    /// singles group of the release, when its `key` is in one (ADR 0084
    /// Decision 6).
    #[must_use]
    pub(crate) fn release_page_name_links(
        &self,
        base: Vec<AlbumNameLinkVm>,
        key: Option<&ReleaseKey>,
    ) -> Vec<AlbumNameLinkVm> {
        release_page_name_links(base, &singles_groups(&self.snapshot.tree), key)
    }

    /// The page of a singles group, from the current tree (ADR 0084
    /// Decision 6). A download or a removal in progress makes the main
    /// action not available.
    #[must_use]
    pub(crate) fn singles_group_page(&self, key: &SinglesGroupKey) -> SinglesGroupPageVm {
        SinglesGroupPageVm::from_tree(&self.snapshot.tree, key, &self.busy)
    }

    /// Rows of the Artists destination list (ADR 0084 Decision 2, ADR 0085
    /// Decisions 2 and 6). It reuses the already-loaded tree: no new
    /// query.
    ///
    /// Each album's artist identity (ADR 0084 Decision 7) decides its
    /// row. `ArtistIdentity::resolve` gives the feed level only for a
    /// confirmed or an unverified artist feed (ADR 0085 Decision 2): a
    /// feed with only an agreed role other than `artist`, for example a
    /// label, never resolves to it, so it gets no row. An album whose
    /// artist feed has no row falls to the name row of its artist text,
    /// the same resolve. No row shows a level word.
    #[must_use]
    pub(crate) fn artists_list(&self) -> Vec<DestinationListRowVm> {
        let mut by_identity: BTreeMap<ArtistIdentity, Vec<(&ArtistNode, &AlbumNode)>> =
            BTreeMap::new();
        for artist in &self.snapshot.tree.artists {
            for album in &artist.albums {
                by_identity
                    .entry(album.artist_identity(&artist.name))
                    .or_default()
                    .push((artist, album));
            }
        }

        let feed_images = self.snapshot.tree.feed_images();
        let mut rows: Vec<DestinationListRowVm> = by_identity
            .into_iter()
            .map(|(identity, members)| artist_list_row(identity, &members, &feed_images))
            .collect();
        rows.sort_by_key(|row| row.label.to_lowercase());
        rows
    }

    /// Rows of the six sidebar destinations, in sidebar order (ADR 0084
    /// Decisions 1 and 2).
    #[must_use]
    pub(crate) fn destination_rows(&self) -> Vec<DestinationRowVm> {
        MusicDestination::ALL
            .into_iter()
            .map(|destination| self.destination_row(destination))
            .collect()
    }

    fn destination_row(&self, destination: MusicDestination) -> DestinationRowVm {
        // ADR 0084 Decision 1: a count of zero shows nothing. Each count
        // names its own items: tracks for Needs attention, playlists for
        // Playlists.
        let count = self
            .destination_count(destination)
            .filter(|count| *count > 0);
        let label = destination.label();
        let noun = match destination {
            MusicDestination::Playlists => "playlist",
            _ => "track",
        };
        let (tooltip, a11y_label) = match count {
            Some(count) => (
                format!("{label}, {count} {noun}{}", plural(count)),
                format!("{label}, {count} {noun}{}", plural(count)),
            ),
            None => (label.to_string(), label.to_string()),
        };
        DestinationRowVm {
            destination,
            element_id: destination_element_id(destination),
            icon: destination_icon(destination),
            label,
            tooltip,
            a11y_label,
            count,
            selected: self.active_destination == destination,
            shows_attention_mark: matches!(destination, MusicDestination::NeedsAttention)
                && count.is_some(),
        }
    }

    /// The destination's count, when it has one (ADR 0084 Decision 1):
    /// Needs attention and Playlists.
    #[must_use]
    fn destination_count(&self, destination: MusicDestination) -> Option<usize> {
        match destination {
            MusicDestination::NeedsAttention => Some(self.needs_attention_count),
            MusicDestination::Playlists => Some(self.snapshot.playlists.len()),
            MusicDestination::Discover
            | MusicDestination::Releases
            | MusicDestination::Artists
            | MusicDestination::Tracks => None,
        }
    }

    /// Rows of the Discover views list column, in list order (ADR 0084
    /// Decision 3). "New releases" is the only view today, so it is
    /// always the open page of the Discover location.
    #[expect(
        clippy::unused_self,
        reason = "kept as an instance method so a later view's selected state travels with the VM"
    )]
    #[must_use]
    pub(crate) fn discover_view_rows(&self) -> Vec<DiscoverViewRowVm> {
        DiscoverView::ALL
            .into_iter()
            .map(|view| DiscoverViewRowVm {
                view,
                element_id: discover_view_element_id(view),
                label: view.label(),
                subtitle: view.subtitle(),
                a11y_label: format!("{}, {}", view.label(), view.subtitle()),
                selected: true,
            })
            .collect()
    }

    /// Display data for the sidebar collapse button (ADR 0084 Decision 1).
    /// The next click sets the opposite of the currently resolved state.
    #[must_use]
    pub(crate) const fn sidebar_collapse_button(
        resolved: MusicSidebarState,
    ) -> SidebarCollapseButtonVm {
        match resolved {
            MusicSidebarState::Expanded => SidebarCollapseButtonVm {
                element_id: "music-sidebar-collapse",
                a11y_label: "Collapse the Music sidebar",
                tooltip: "Collapse sidebar",
                next_preference: MusicSidebarPreference::Collapsed,
            },
            MusicSidebarState::Collapsed => SidebarCollapseButtonVm {
                element_id: "music-sidebar-collapse",
                a11y_label: "Expand the Music sidebar",
                tooltip: "Expand sidebar",
                next_preference: MusicSidebarPreference::Expanded,
            },
        }
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
    pub(crate) fn status_snapshot(&self) -> LibraryStatusSnapshot {
        LibraryStatusSnapshot::from_text(&self.status)
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
            state_label: has_stale.then_some(UPDATE_AVAILABLE_LABEL),
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

    /// ADR 0076 Decision 2: the RSS check command did not start a check.
    pub(crate) fn fail_playlist_rss_check(&mut self, error: impl std::fmt::Display) {
        self.status = format!("Error starting the RSS check: {error:#}");
    }

    #[must_use]
    pub(crate) fn confirm_library_removal(
        &mut self,
        plan: LibraryRemovalPlan,
        confirmation: RemovalConfirmation,
    ) -> bool {
        self.library_removal.confirm_or_defer(plan, confirmation)
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

    /// Only this type's own tests read the selection back; the live screen
    /// uses the playlist selection helpers below instead.
    #[must_use]
    #[cfg(test)]
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

    /// The downloads and the removals that are in progress (ADR 0084
    /// Decision 10).
    #[must_use]
    pub(crate) fn busy_items(&self) -> &BusyItems {
        &self.busy
    }

    /// Replaces the download part of the busy state with the queue state
    /// (ADR 0084 Decision 10, packet 022).
    pub(crate) fn set_download_activity(&mut self, activity: DownloadActivity) {
        self.busy.downloads = activity;
    }

    /// Marks the Library feed `feed_id` as being removed.
    pub(crate) fn begin_feed_removal(&mut self, feed_id: i64) {
        self.busy.removing_feeds.insert(feed_id);
        self.library_removal.cancel();
        self.status = "Removing feed...".into();
    }

    /// Clears the removal mark of the Library feed `feed_id`.
    pub(crate) fn finish_feed_removal(&mut self, feed_id: i64) {
        self.busy.removing_feeds.remove(&feed_id);
    }

    /// Sets a report text in the status line.
    pub(crate) fn set_status(&mut self, text: impl Into<String>) {
        self.status = text.into();
    }

    pub(crate) fn finish_track_subscribe(&mut self, outcome: TrackSubscribeOutcome) {
        let mut message = format!("Downloaded track: {}", outcome.path_label);
        if let Some(warning) = outcome.format_warning {
            message.push_str(" — ");
            message.push_str(&warning);
        }
        self.status = message;
    }

    /// Only this type's own tests read status text back directly; the
    /// screen renders status through the typed report views instead.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn status(&self) -> &str {
        &self.status
    }

    pub(crate) fn set_error_status(&mut self, error: impl std::fmt::Display) {
        self.library_loading = false;
        self.library_removal.cancel();
        self.status = format!("Error: {error:#}");
    }

    #[cfg(test)]
    pub(crate) fn retained_observation_evidence_counts(&self) -> (usize, usize) {
        (
            self.observation_receipts.len(),
            self.observation_failures.len(),
        )
    }

    /// ADR 0075 retains all failed generations independently from track navigation.
    pub(crate) fn retain_observation_failure(
        &mut self,
        error: &crate::application::errors::command::CommandError,
    ) {
        self.status = format!("Error: {error}");
        self.retain_query_evidence(error);
    }

    /// ADR 0075 retains receipts without changing presentation or operation state.
    pub(crate) fn retain_observation_receipts(
        &mut self,
        receipts: &[crate::provider_observation::ObservationReceipt],
    ) {
        self.observation_receipts.extend(
            receipts
                .iter()
                .map(|receipt| (receipt.generation, receipt.clone())),
        );
    }

    /// ADR 0075 keeps ordinary query evidence separate from error presentation.
    pub(crate) fn retain_query_evidence(
        &mut self,
        error: &crate::application::errors::command::CommandError,
    ) {
        if let crate::application::errors::command::CommandError::ObservedQueryFailure(failure) =
            error
        {
            self.retain_observation_receipts(failure.receipts());
        }
        if let crate::application::errors::command::CommandError::ObservedCommandFailure(failure) =
            error
        {
            self.retain_observation_receipts(failure.receipts());
        }
        if let crate::application::errors::command::CommandError::ObservationWriteFailure(failure) =
            error
        {
            self.retain_observation_receipts(&failure.receipts);
            if let Some(capsule) = &failure.write_failure {
                self.observation_failures
                    .insert(capsule.token.generation(), std::sync::Arc::clone(capsule));
            }
        }
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
        self.snapshot.feed_update_state.phase = FeedUpdatePhase::Idle;
        self.snapshot.feed_update_state.status_message =
            Some(format!("Feed check error: {message:#}"));
    }

    pub(crate) fn begin_all_feed_check(&mut self, feed_count: usize) {
        self.snapshot.feed_update_state.phase = FeedUpdatePhase::Checking;
        self.snapshot.feed_update_state.stale.clear();
        self.snapshot.feed_update_state.status_message =
            Some(format!("Checking {feed_count} feeds..."));
    }

    pub(crate) fn finish_all_feed_check_with_route_repair(
        &mut self,
        outcome: FeedCheckRouteRepairOutcome,
    ) {
        self.snapshot.feed_update_state.phase = FeedUpdatePhase::Idle;
        self.snapshot.feed_update_state.stale.clear();
        self.snapshot.feed_update_state.status_message =
            Some(feed_check_route_repair_message(outcome));
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

    #[must_use]
    pub(crate) fn begin_broadcast_route_repair(&mut self, track_id: i64) -> bool {
        if !self
            .content_list_page
            .has_available_broadcast_route_repair(track_id)
            || !self.in_flight_broadcast_route_repairs.insert(track_id)
        {
            return false;
        }

        self.content_list_page
            .set_broadcast_route_repairing(track_id, true);
        self.status = broadcast_route_repair_started_message(track_id);
        true
    }

    pub(crate) fn finish_broadcast_route_repair(
        &mut self,
        completion: &BroadcastRouteRepairCompletion,
    ) {
        self.in_flight_broadcast_route_repairs
            .remove(&completion.track_id);
        self.content_list_page
            .set_broadcast_route_repairing(completion.track_id, false);
        self.status = broadcast_route_repair_completion_message(completion);
    }

    pub(crate) fn fail_broadcast_route_repair(
        &mut self,
        track_id: i64,
        error: impl std::fmt::Display,
    ) {
        let completion = BroadcastRouteRepairCompletion::new(
            track_id,
            None,
            BroadcastRouteRepairCompletionStatus::Failed {
                reason: format!("{error:#}"),
            },
        );
        self.finish_broadcast_route_repair(&completion);
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

    /// Only this type's own tests call this directly; the live screen reads
    /// `renaming_playlist_id` instead.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn is_renaming_playlist(&self, playlist_id: i64) -> bool {
        self.renaming_playlist_id == Some(playlist_id)
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
        feed_change_count_label(count)
    }

    #[must_use]
    pub(crate) fn playlist_sort_label(&self) -> &'static str {
        self.playlist_sort.label()
    }
}

fn feed_change_count_label(count: usize) -> String {
    format!("{count} feed{} changed", plural(count))
}

fn feed_check_route_repair_message(outcome: FeedCheckRouteRepairOutcome) -> String {
    let mut message = format!(
        "Checked {} feed{}",
        outcome.feeds_checked,
        plural(outcome.feeds_checked)
    );
    if outcome.feeds_changed == 0 {
        message.push_str("; all feeds up to date");
    } else {
        write!(
            &mut message,
            "; {} feed{} changed",
            outcome.feeds_changed,
            plural(outcome.feeds_changed)
        )
        .expect("writing to a String cannot fail");
    }
    if outcome.feed_update_failures > 0 {
        write!(
            &mut message,
            "; {} feed update issue{}",
            outcome.feed_update_failures,
            plural(outcome.feed_update_failures)
        )
        .expect("writing to a String cannot fail");
    }

    if outcome.route_tags_repaired == 0
        && outcome.route_tags_no_routes_upstream == 0
        && outcome.route_tag_failures == 0
    {
        message.push_str("; no route repairs needed");
        return message;
    }

    write!(
        &mut message,
        "; repaired {} route tag{}; {} track{} {} {}; {} failed",
        outcome.route_tags_repaired,
        plural(outcome.route_tags_repaired),
        outcome.route_tags_no_routes_upstream,
        plural(outcome.route_tags_no_routes_upstream),
        if outcome.route_tags_no_routes_upstream == 1 {
            "needs"
        } else {
            "need"
        },
        BROADCAST_ROUTE_NEEDS_PAYMENT_ROUTES_LABEL,
        outcome.route_tag_failures
    )
    .expect("writing to a String cannot fail");
    message
}

fn broadcast_route_repair_started_message(track_id: i64) -> String {
    format!("Repairing payment routes for track {track_id}...")
}

fn broadcast_route_repair_completion_message(
    completion: &BroadcastRouteRepairCompletion,
) -> String {
    let title = completion
        .title
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map_or_else(|| format!("track {}", completion.track_id), str::to_owned);
    match &completion.status {
        BroadcastRouteRepairCompletionStatus::Repaired => {
            format!("Repaired payment routes for {title}")
        }
        BroadcastRouteRepairCompletionStatus::NoRoutesUpstream => {
            format!("{BROADCAST_ROUTE_NO_ROUTES_UPSTREAM_PREFIX} {title}")
        }
        BroadcastRouteRepairCompletionStatus::Skipped => {
            format!("Payment routes already ready for {title}")
        }
        BroadcastRouteRepairCompletionStatus::Failed { reason } => {
            format!("Route repair failed for {title}: {reason}")
        }
    }
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
        .flat_map(|album| {
            let album_image_href = album.image_href.as_deref();
            album.tracks.iter().map(move |track| {
                ContentListRowDisplay::from_track_with_album_image(track, album_image_href)
            })
        })
        .collect()
}

fn content_list_rows_from_recent_feeds(rows: &[RecentFeedResultRow]) -> Vec<ContentListRowDisplay> {
    rows.iter()
        .map(|(_id, row)| ContentListRowDisplay::from_release_result(row.clone()))
        .collect()
}

/// A stable element-id fragment for a release key (ADR 0084 Decision 2).
fn release_row_slug(key: &ReleaseKey) -> String {
    match key {
        ReleaseKey::FeedGuid(feed_guid) => format!("guid-{}", release_row_slug_text(feed_guid)),
        ReleaseKey::LibraryFeed(feed_id) => format!("feed-{feed_id}"),
    }
}

/// A stable element-id fragment from free text (ADR 0084 Decision 2).
fn release_row_slug_text(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

/// The sidebar element id of a destination (ADR 0084 Decision 2).
const fn destination_element_id(destination: MusicDestination) -> &'static str {
    match destination {
        MusicDestination::Discover => "music-destination-discover",
        MusicDestination::Releases => "music-destination-releases",
        MusicDestination::Artists => "music-destination-artists",
        MusicDestination::Tracks => "music-destination-tracks",
        MusicDestination::NeedsAttention => "music-destination-needs-attention",
        MusicDestination::Playlists => "music-destination-playlists",
    }
}

/// The named icon of a destination (ADR 0084 Decision 1). The renderer
/// maps this to an `IconName`.
const fn destination_icon(destination: MusicDestination) -> MusicDestinationIcon {
    match destination {
        MusicDestination::Discover => MusicDestinationIcon::Discover,
        MusicDestination::Releases => MusicDestinationIcon::Release,
        MusicDestination::Artists => MusicDestinationIcon::Artist,
        MusicDestination::Tracks => MusicDestinationIcon::Track,
        MusicDestination::NeedsAttention => MusicDestinationIcon::Warning,
        MusicDestination::Playlists => MusicDestinationIcon::Playlist,
    }
}

/// The list-row element id of a Discover view (ADR 0084 Decision 3).
fn discover_view_element_id(view: DiscoverView) -> String {
    let slug = match view {
        DiscoverView::NewReleases => "new-releases",
    };
    format!("music-discover-view-{slug}")
}

/// The track key of a content-list track row, keyed by the Library track
/// id a row's own id carries (ADR 0084 Decision 4, Required Change 5).
/// `None` when `row_id` does not parse as a track id, or no track in
/// `tree` has that id.
fn track_row_key_in_tree(tree: &LibraryTree, row_id: &str) -> Option<TrackKey> {
    let track_id: i64 = row_id.parse().ok()?;
    tree.artists
        .iter()
        .flat_map(|artist| &artist.albums)
        .flat_map(|album| &album.tracks)
        .find(|track| track.id == track_id)
        .map(TrackRow::track_key)
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
    pub(crate) fn primary_action_vm(&self, work: TrackRowWork) -> EntityActionVm {
        self.track_action_state(work)
            .primary_action(EntityActionTarget::Track(self.track_ref()))
    }

    #[must_use]
    pub(crate) fn row_display(&self) -> LibraryTrackRowDisplay {
        let track_id = self.track.id;
        LibraryTrackRowDisplay {
            row_id: format!("album-track-{track_id}"),
            toggle_button_id: format!("lib-toggle-{track_id}"),
            state_label: (!self.track.is_in_library).then_some(NEW_RELEASE_LABEL),
        }
    }

    /// The action state of the row. A download of the row, or of its
    /// release, gives `Queued` or `Downloading` and no "Remove" (orchestrator
    /// correction of 2026-10-20).
    #[must_use]
    pub(crate) fn track_action_state(&self, work: TrackRowWork) -> TrackActionState {
        let membership = match (self.track.is_in_library, work) {
            (_, TrackRowWork::Removing) => TrackMembershipState::Removing,
            (_, TrackRowWork::Queued) => TrackMembershipState::Queued,
            (_, TrackRowWork::Downloading) => TrackMembershipState::Downloading,
            (true, TrackRowWork::Idle) => TrackMembershipState::InLibrary,
            (false, TrackRowWork::Idle) => TrackMembershipState::RemoteOnly,
        };
        TrackActionState::new(membership)
    }

    /// The `MusicBrainz` status of the row, or `None` when no lookup
    /// started for this track.
    #[must_use]
    pub(crate) const fn mb_status(&self) -> Option<&'a MbTrackStatus> {
        self.mb
    }

    #[must_use]
    fn track_ref(&self) -> TrackRef {
        TrackRef::LocalTrackId(self.track.id)
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
    tracks: &'a [TrackRow],
}

impl<'a> LibraryArtistDetailVm<'a> {
    #[must_use]
    pub(crate) fn new(name: &'a str, tracks: &'a [TrackRow]) -> Self {
        Self { name, tracks }
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
        rows
    }

    /// Page projection for the Library name grouping. The subtitle names
    /// the grouping itself: it has no publisher relationship, no role and
    /// no page type (ADR 0077 Decision 6, "Album without a publisher"
    /// refinement).
    #[must_use]
    pub(crate) fn page(&self) -> ArtistDetailPageVm {
        ArtistDetailPageVm::new(
            self.artist_name_or_unknown(),
            Some("Grouped by name"),
            self.detail_rows()
                .into_iter()
                .map(|(key, value)| ArtistDetailFactVm::new(key, value, 6))
                .collect(),
            !self.feed_summaries().is_empty(),
        )
    }

    /// The image URL of each release row, in row order, each one time. The
    /// Library loads these images before it draws the page, so a row shows
    /// its cover, not the monogram (operator check of 2026-10-18).
    #[must_use]
    pub(crate) fn thumbnail_urls(&self) -> Vec<String> {
        let mut urls: Vec<String> = Vec::new();
        for url in self
            .feed_summaries()
            .into_iter()
            .filter_map(|row| row.thumb_url)
        {
            if !urls.contains(&url) {
                urls.push(url);
            }
        }
        urls
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
                // The first track can have no image while another track of
                // the release has one. Read each track, as the Library tree
                // does for `AlbumNode::image_href`: the album image first,
                // then a track image.
                let thumb_url = tracks
                    .iter()
                    .find_map(|t| t.album_image_href.clone())
                    .or_else(|| tracks.iter().find_map(|t| t.track_image_href.clone()));
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

/// The downloads and the removals in progress (ADR 0084 Decision 10,
/// packet 022). The download queue gives the download part. The removal
/// part holds each Library feed whose removal runs.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct BusyItems {
    downloads: DownloadActivity,
    removing_feeds: BTreeSet<i64>,
}

impl BusyItems {
    /// The busy state of `downloads` and `removing_feeds`. Only tests
    /// build it directly. The view model changes each part in place.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn new(downloads: DownloadActivity, removing_feeds: BTreeSet<i64>) -> Self {
        Self {
            downloads,
            removing_feeds,
        }
    }

    /// `true` while any job of the album key is queued or in flight. The
    /// singles group page and the removal availability read it.
    #[must_use]
    pub(crate) fn release_downloading(&self, album: &AlbumNode) -> bool {
        self.release_download_phase(album).is_some()
    }

    /// The phase of the jobs of the album key. `InFlight` when a job of the
    /// key runs, `Queued` when a job of the key only waits. The membership
    /// of the album reads it.
    #[must_use]
    pub(crate) fn release_download_phase(&self, album: &AlbumNode) -> Option<DownloadPhase> {
        album
            .feed_key()
            .and_then(|key| self.downloads.key_phase(&key))
    }

    /// `true` while the removal of the album runs.
    #[must_use]
    pub(crate) fn removing(&self, album: &AlbumNode) -> bool {
        album
            .feed_id
            .is_some_and(|feed_id| self.removing_feeds.contains(&feed_id))
    }

    /// The work in progress on the row of `track` in `album` (ADR 0084
    /// Decision 10, orchestrator correction of 2026-10-20).
    ///
    /// - A removal of the album gives `Removing` to each track that the
    ///   removal changes.
    /// - A job of the track gives its own phase.
    /// - A track that is not in the Library gives the phase of a release
    ///   job of the album key.
    /// - A track in the Library gives the phase of each job of the album
    ///   key, so the row has no "Remove" while the key is active.
    #[must_use]
    pub(crate) fn track_row_work(&self, album: &AlbumNode, track: &TrackRow) -> TrackRowWork {
        let has_library_tracks = album.tracks.iter().any(|t| t.is_in_library);
        if self.removing(album) && (!has_library_tracks || track.is_in_library) {
            return TrackRowWork::Removing;
        }
        let phase = self.downloads.track_phase(track.id).or_else(|| {
            let key = album.feed_key()?;
            if track.is_in_library {
                self.downloads.key_phase(&key)
            } else {
                self.downloads.release_phase(&key)
            }
        });
        TrackRowWork::from_phase(phase)
    }

    /// The work in progress on the row of the Index track `track_guid` in
    /// the release `key` (ADR 0083 task 006, decision D8). The row reads
    /// the phase of its own job, then the phase of a release job of `key`.
    /// A job of another track of the release does not change the row.
    #[must_use]
    pub(crate) fn index_track_work(&self, key: &FeedKey, track_guid: &str) -> TrackRowWork {
        TrackRowWork::from_phase(
            self.downloads
                .index_track_phase(key, track_guid)
                .or_else(|| self.downloads.release_phase(key)),
        )
    }
}

/// The work in progress on one Library track row (ADR 0084 Decision 10).
/// It selects the state word and the primary action of the row.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum TrackRowWork {
    /// No work.
    #[default]
    Idle,
    /// A download job waits in the queue.
    Queued,
    /// A download job runs.
    Downloading,
    /// A removal runs.
    Removing,
}

impl TrackRowWork {
    /// The work of a download phase. `None` gives `Idle`.
    #[must_use]
    pub(crate) const fn from_phase(phase: Option<DownloadPhase>) -> Self {
        match phase {
            None => Self::Idle,
            Some(DownloadPhase::Queued) => Self::Queued,
            Some(DownloadPhase::InFlight) => Self::Downloading,
        }
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
    has_library_tracks: bool,
}

impl<'a> LibraryAlbumDetailVm<'a> {
    #[must_use]
    pub(crate) fn new(tracks: &'a [TrackRow], mb_status: &'a BTreeMap<i64, MbTrackStatus>) -> Self {
        Self {
            mb_status,
            has_library_tracks: tracks.iter().any(|track| track.is_in_library),
        }
    }

    /// `true` when any track has an in-flight `MusicBrainz` lookup —
    /// used by the screen to disable the `MusicBrainz` action button.
    #[must_use]
    pub(crate) fn has_active_musicbrainz(&self) -> bool {
        self.mb_status
            .values()
            .any(|s| matches!(s, MbTrackStatus::Pending | MbTrackStatus::Processing))
    }

    /// The Library membership of the album. `download` is the phase of a
    /// download of the album key (ADR 0084 Decision 10): a job that only
    /// waits gives `Queued`, and a job that runs gives `Downloading`.
    /// `removing` is a removal of the album in progress.
    #[must_use]
    pub(crate) fn membership(
        &self,
        download: Option<DownloadPhase>,
        removing: bool,
    ) -> ReleaseMembershipState {
        if removing {
            ReleaseMembershipState::Removing
        } else if let Some(phase) = download {
            match phase {
                DownloadPhase::Queued => ReleaseMembershipState::Queued,
                DownloadPhase::InFlight => ReleaseMembershipState::Downloading,
            }
        } else if self.has_library_tracks {
            ReleaseMembershipState::InLibrary
        } else {
            ReleaseMembershipState::RemoteOnly
        }
    }
}

impl AlbumNode {
    /// The download queue key of this album (ADR 0084 Decision 10): the
    /// feed GUID, then the feed URL, then the Library feed id.
    #[must_use]
    pub(crate) fn feed_key(&self) -> Option<FeedKey> {
        FeedKey::new(
            self.feed_guid.as_deref(),
            self.feed_url.as_deref(),
            self.feed_id,
        )
    }

    /// Feed URL string required by the shared release-detail projection.
    /// Absence is centralized here so screens do not call `unwrap_or_default`
    /// on `feed_url`.
    #[must_use]
    pub(crate) fn feed_url_for_detail(&self) -> String {
        self.feed_url.clone().unwrap_or_default()
    }

    /// The release key of this album (ADR 0084 Decision 4). An album with a
    /// feed GUID uses it, so the same release opened from the Index keys to
    /// the same navigation entry. An album with no feed GUID uses its
    /// Library feed id. `None` when the album has neither.
    #[must_use]
    pub(crate) fn release_key(&self) -> Option<ReleaseKey> {
        if let Some(feed_guid) = self.feed_guid.clone() {
            return Some(ReleaseKey::FeedGuid(feed_guid));
        }
        self.feed_id.map(ReleaseKey::LibraryFeed)
    }

    /// The release page's track rows (ADR 0084 Decisions 4 and 8,
    /// operator decision of 2026-10-16). The release page reads no list
    /// filter: it always shows each track of the release, each with its
    /// own state word.
    #[must_use]
    pub(crate) fn release_track_rows(&self) -> &[TrackRow] {
        &self.tracks
    }

    /// The kind of this release (ADR 0084 Decision 5).
    #[must_use]
    pub(crate) fn release_kind(&self) -> ReleaseKind {
        ReleaseKind::resolve(
            self.metadata_facts.release_kind.as_deref(),
            self.tracks.len(),
        )
    }

    /// The artist identity of this release (ADR 0084 Decision 7).
    /// `artist_text` is the name of the [`ArtistNode`] that holds the
    /// album.
    #[must_use]
    pub(crate) fn artist_identity(&self, artist_text: &str) -> ArtistIdentity {
        ArtistIdentity::resolve(&self.named_feeds, artist_text)
    }
}

impl TrackRow {
    /// The track key of this row (ADR 0084 Decision 4). A track with a
    /// feed GUID uses it, so the same track opened from the Index keys to
    /// the same navigation entry. A track in a feed with no feed GUID uses
    /// its Library feed id.
    #[must_use]
    pub(crate) fn track_key(&self) -> TrackKey {
        let release = match self.feed_guid.clone() {
            Some(feed_guid) => ReleaseKey::from_index_feed_guid(feed_guid),
            None => ReleaseKey::LibraryFeed(self.feed_id),
        };
        TrackKey {
            release: Some(release),
            item_guid: self.item_guid.clone(),
        }
    }
}

#[cfg(test)]
mod track_row_track_key_tests {
    use super::{ReleaseKey, TrackKey, TrackRow};

    fn track(feed_id: i64, feed_guid: Option<&str>, item_guid: &str) -> TrackRow {
        TrackRow {
            feed_id,
            feed_guid: feed_guid.map(str::to_string),
            item_guid: item_guid.to_string(),
            ..TrackRow::default()
        }
    }

    /// R3-01: the track key of a Library track with a feed GUID equals the
    /// track key of an Index target with the same feed GUID and item GUID.
    #[test]
    fn adr_0084_track_key_of_library_track_with_feed_guid_matches_the_index_key() {
        let key = track(10, Some("saw-guid"), "item-1").track_key();

        assert_eq!(
            key,
            TrackKey::from_index_track_target(Some("saw-guid"), "item-1"),
            "a Library track with a feed GUID must key the same as an Index target with that feed GUID and item GUID"
        );
    }

    /// R3-02: the track key of a Library track in a feed with no feed GUID
    /// carries `ReleaseKey::LibraryFeed`.
    #[test]
    fn adr_0084_track_key_of_library_track_with_no_feed_guid_uses_its_feed_id() {
        let key = track(20, None, "item-2").track_key();

        assert_eq!(
            key,
            TrackKey {
                release: Some(ReleaseKey::LibraryFeed(20)),
                item_guid: "item-2".to_string(),
            }
        );
    }
}

#[cfg(test)]
mod album_node_release_key_tests {
    use super::{AlbumNode, FeedMetadataFacts, LocalIdentityFacts, ReleaseKey};

    fn album(feed_id: Option<i64>, feed_guid: Option<&str>) -> AlbumNode {
        AlbumNode {
            name: "Selected Ambient Works".to_string(),
            feed_id,
            feed_guid: feed_guid.map(str::to_string),
            feed_url: None,
            language: None,
            description: None,
            image_href: None,
            named_feeds: Vec::new(),
            identity_facts: LocalIdentityFacts::default(),
            metadata_facts: Box::<FeedMetadataFacts>::default(),
            stored_values: None,
            tracks: Vec::new(),
        }
    }

    /// R2-01: the release key of a Library album with a feed GUID equals
    /// the release key of an Index row with the same feed GUID.
    #[test]
    fn adr_0084_release_key_of_album_with_feed_guid_matches_the_index_key() {
        let key = album(Some(10), Some("saw-guid")).release_key();

        assert_eq!(
            key,
            Some(ReleaseKey::from_index_feed_guid("saw-guid")),
            "a Library album with a feed GUID must key the same as an Index row with that GUID"
        );
    }

    /// R2-02: the release key of a Library album with no feed GUID is
    /// `LibraryFeed` with its feed id.
    #[test]
    fn adr_0084_release_key_of_album_with_no_feed_guid_uses_its_feed_id() {
        let key = album(Some(20), None).release_key();

        assert_eq!(key, Some(ReleaseKey::LibraryFeed(20)));
    }

    /// An album with neither a feed GUID nor a feed id has no release key.
    #[test]
    fn adr_0084_release_key_of_album_with_neither_is_none() {
        let key = album(None, None).release_key();

        assert_eq!(key, None);
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
    /// The work in progress on the track. The host gives it from
    /// [`BusyItems`] (ADR 0083 task 007).
    work: TrackRowWork,
}

/// The "Add to playlist" choices of a playlist row: each playlist but the
/// open playlist (ADR 0083 task 007, decision D8). The bar of packet 025
/// uses the same rule.
#[must_use]
pub(crate) fn playlist_row_add_choices(
    playlists: &[db::Playlist],
    open_playlist_id: i64,
) -> Vec<crate::view_models::PlaylistOptionDisplayVm> {
    crate::view_models::playlist_option_displays(playlists)
        .into_iter()
        .filter(|option| option.id != open_playlist_id)
        .collect()
}

/// When a row cell shows (ADR 0083 Decision 6). A hidden cell keeps its
/// width.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RowCellVisibility {
    /// The cell shows at all times.
    Always,
    /// The cell shows on a hover of the row and on the keyboard focus of
    /// the row.
    OnHover,
}

/// What the playlist row's play control shows (ADR 0083 Decision 10).
///
/// `Play` draws the Play icon alone, with its tooltip and accessibility
/// label carrying the word. `RepairPlayback` draws a text word instead,
/// because the control no longer offers to play: it offers to repair
/// playback. Neither state is a character.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PlaylistPlayAffordance {
    Play,
    RepairPlayback,
}

impl PlaylistPlayAffordance {
    /// The visible word, when this affordance shows one instead of an icon.
    #[must_use]
    pub(crate) const fn label(self) -> Option<&'static str> {
        match self {
            Self::Play => None,
            Self::RepairPlayback => Some("Repair playback"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlaylistTrackControlsDisplay {
    pub(crate) row_id: String,
    pub(crate) row_body_id: String,
    pub(crate) drag_handle_id: String,
    pub(crate) drag_handle_a11y_label: &'static str,
    pub(crate) play_button_id: String,
    pub(crate) play_affordance: PlaylistPlayAffordance,
    pub(crate) play_a11y_label: &'static str,
    pub(crate) play_enabled: bool,
    /// When the play cell shows. "Repair playback" shows at all times,
    /// and the Play icon shows on hover (ADR 0083 task 007, decision D4).
    pub(crate) play_visibility: RowCellVisibility,
    /// The "⋯" menu and the right-click menu of the row (ADR 0083 task
    /// 007, decision D5). `None` in selection mode.
    pub(crate) menu: Option<TrackRowMenuVm>,
    /// The id of the row checkbox of selection mode (packet 025).
    pub(crate) checkbox_id: String,
    /// The drag handle and the drop target show (ADR 0084 Decision 10,
    /// packet 025). True in `Normal` only. In `Selecting`, the checkbox
    /// takes the place of the drag handle.
    pub(crate) reorder_visible: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlaylistTrackRowDisplay {
    pub(crate) is_available: bool,
    pub(crate) position: i64,
    pub(crate) position_label: String,
    pub(crate) title: String,
    pub(crate) artist: String,
    pub(crate) availability_label: Option<&'static str>,
    /// ADR 0076 Decision 7: the row error and the two removal actions of a
    /// track with an unconfirmed "removed from feed" mark.
    pub(crate) removed_from_feed: Option<super::playlist_rss_check::PlaylistRemovedFromFeedDisplay>,
    pub(crate) duration_label: String,
    pub(crate) thumb_url: Option<String>,
    pub(crate) controls: PlaylistTrackControlsDisplay,
    /// The selection facts of the row (ADR 0084 Decision 10, packet 025).
    pub(crate) selection: RowSelectionVm,
}

impl PlaylistTrackRowDisplay {
    /// Applies the selection facts of the row (packet 025). In
    /// `Selecting`, the checkbox shows, and the drag handle, the drop
    /// target and the row menu hide (ADR 0083 task 007, decision D12).
    #[must_use]
    pub(crate) fn with_selection(mut self, selection: RowSelectionVm) -> Self {
        self.controls.reorder_visible = !selection.checkbox_visible;
        if selection.checkbox_visible {
            self.controls.menu = None;
        }
        self.selection = selection;
        self
    }
}

#[cfg(test)]
impl PlaylistTrackRowDisplay {
    /// Display-ready fixture for shared-shell geometry tests (ADR 0044).
    pub(crate) fn playback_repair_fixture() -> Self {
        let track = TrackRow {
            id: 1,
            track_title: Some("A long playlist track title".repeat(5)),
            is_in_library: true,
            artist_name: Some("Artist".into()),
            duration_seconds: Some(30),
            local_path: Some(crate::library_path::LibraryRelativePath::for_test("a.wav")),
            ..Default::default()
        };
        PlaylistTrackRowVm::new(&track, 0, 2).display_with_playback(
            1,
            Err(
                crate::application::capability::ExecutionUnavailable::configured(
                    crate::application::capability::Dependency::Playback,
                ),
            ),
        )
    }
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
    /// ADR 0076 Decision 2: the "Check RSS" action with typed availability.
    pub(crate) check_rss: super::playlist_rss_check::PlaylistCheckRssActionDisplay,
}

impl PlaylistDetailActionsDisplay {
    pub(crate) const RENAME_INPUT_PLACEHOLDER: &'static str = "Playlist name";
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlaylistDetailHeaderDisplay {
    pub(crate) title: String,
}

impl<'a> PlaylistTrackRowVm<'a> {
    /// Apply the prepared player and runtime facts to the file's play action (ADR 0066).
    #[must_use]
    pub(crate) fn display_with_playback(
        &self,
        playlist_id: i64,
        availability: Result<(), crate::application::capability::ExecutionUnavailable>,
    ) -> PlaylistTrackRowDisplay {
        let mut display = self.display(playlist_id);
        if display.controls.play_enabled && availability.is_err() {
            display.controls.play_affordance = PlaylistPlayAffordance::RepairPlayback;
            display.controls.play_a11y_label = "Repair playback for this playlist track";
            // ADR 0083 task 007, decision D4: "Repair playback" tells the
            // operator that playback does not work, so it always shows.
            display.controls.play_visibility = RowCellVisibility::Always;
        }
        display
    }

    /// Construct a row VM from a borrowed track and its position in
    /// the surrounding playlist.
    ///
    /// Used by both the eager projection (`PlaylistDetailVm::track_rows`)
    /// and the paged screen path which materialises one row at a time
    /// from a [`super::paged_playlist_detail::PagedPlaylistDetailVm`].
    #[must_use]
    pub(crate) const fn new(track: &'a TrackRow, position: usize, last_position: usize) -> Self {
        Self {
            track,
            position,
            last_position,
            work: TrackRowWork::Idle,
        }
    }

    /// The row with the work in progress on its track (ADR 0083 task 007).
    /// The work selects "Download" and "Remove from Library…" of the menu.
    #[must_use]
    pub(crate) const fn with_work(mut self, work: TrackRowWork) -> Self {
        self.work = work;
        self
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
            play_button_id: format!("playlist-play-{playlist_id}-{position}"),
            play_affordance: PlaylistPlayAffordance::Play,
            play_a11y_label: "Play this playlist track",
            play_enabled: self.can_play(),
            play_visibility: RowCellVisibility::OnHover,
            menu: self.menu(playlist_id),
            checkbox_id: format!("playlist-checkbox-{playlist_id}-{position}"),
            reorder_visible: true,
        }
    }

    /// The row menu in the order of ADR 0083 task 007 decision D5. An item
    /// that cannot act is absent. "Remove from playlist" is not
    /// destructive (decision D6). "Remove from Library…" is last and
    /// destructive (decision D7).
    fn menu(&self, playlist_id: i64) -> Option<TrackRowMenuVm> {
        let row_id = format!("playlist-actions-{playlist_id}-{}", self.position);
        let title = self.title();
        let availability = LibraryTrackMenuAvailability::of(self.track, self.work);
        TrackRowMenuVm::project(&TrackRowMenuFacts {
            add_to_playlist: true,
            download: availability.download,
            inspect_sources: true,
            move_up: self.can_move_up(),
            move_down: self.can_move_down(),
            remove_from_playlist: true,
            remove: availability
                .remove
                .then_some(TrackRowRemoveLabel::RemoveFromLibrary),
            ..TrackRowMenuFacts::new(&row_id, &title, false)
        })
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
            removed_from_feed: None,
            duration_label: self.duration_label(),
            thumb_url: self.thumb_url().map(str::to_string),
            controls: self.controls_display(playlist_id),
            selection: RowSelectionVm::none(),
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
    rss_check: Option<&'a crate::runtime::PlaylistRssCheckSnapshot>,
}

impl<'a> PlaylistDetailVm<'a> {
    #[must_use]
    pub(crate) fn new(playlist: &'a db::Playlist, tracks: &'a [TrackRow]) -> Self {
        Self {
            playlist,
            tracks,
            text_filter: None,
            rss_check: None,
        }
    }

    /// The header art of the playlist in the logo frame (ADR 0084 Decision
    /// 12): the first four images in playlist order. The image of a track
    /// is its own image, else its album image. A playlist gives no image of
    /// its own.
    #[must_use]
    pub(crate) fn collection_art(&self) -> CollectionArtVm {
        CollectionArtVm::resolve(
            None,
            self.tracks.iter().filter_map(|track| {
                track
                    .track_image_href
                    .as_deref()
                    .or(track.album_image_href.as_deref())
            }),
            CollectionKind::Playlist,
        )
    }

    /// Attach the playlist RSS check snapshot (ADR 0076 Decision 2).
    /// `None` means that the check actor is not available.
    #[must_use]
    pub(crate) fn with_rss_check(
        mut self,
        snapshot: Option<&'a crate::runtime::PlaylistRssCheckSnapshot>,
    ) -> Self {
        self.rss_check = snapshot;
        self
    }

    /// The report of the latest RSS check of this playlist.
    #[must_use]
    pub(crate) fn rss_check_report(
        &self,
    ) -> Option<super::playlist_rss_check::PlaylistRssCheckReportDisplay> {
        super::playlist_rss_check::report(self.playlist.id, self.rss_check)
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
            check_rss: super::playlist_rss_check::PlaylistCheckRssActionDisplay::new(
                playlist_id,
                super::playlist_rss_check::availability(
                    playlist_id,
                    self.tracks.len(),
                    self.rss_check,
                ),
            ),
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
            .map(|(position, track)| PlaylistTrackRowVm::new(track, position, last_position))
            .collect()
    }

    /// Each visible track with its position, in stored order. The detail
    /// selection reads its rows in this order (packet 025).
    pub(crate) fn visible_tracks(&self) -> Vec<(usize, &'a TrackRow)> {
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

/// `is_empty` has no caller outside tests of this type and its
/// `PlaylistDetailPageVm` wrapper: the playlist screen checks slot
/// emptiness instead. Compiled only for tests.
#[cfg(test)]
impl PlaylistDetailVm<'_> {
    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.visible_tracks().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn broadcast_readiness_track(
        id: i64,
        title: &str,
        state: crate::application::queries::broadcast::BroadcastReadinessState,
        reason: &str,
    ) -> BroadcastReadinessTrack {
        BroadcastReadinessTrack {
            track_id: id,
            title: title.to_owned(),
            artist: Some("Artist".to_owned()),
            album: Some("Album".to_owned()),
            path: Some("/tmp/track.mp3".to_owned()),
            state,
            reason: reason.to_owned(),
        }
    }

    fn release_result(id: &str, label: &str, origin: SearchResultOrigin) -> FeedResultDisplay {
        FeedResultDisplay::new(id, label, origin).with_secondary_text("12 tracks")
    }

    fn track_result(id: &str, label: &str, origin: SearchResultOrigin) -> TrackResultDisplay {
        TrackResultDisplay::new(id, label, origin).with_secondary_text("Artist")
    }

    fn recent_feed_batch(
        rows: Vec<(&str, &str)>,
        cursor: Option<&str>,
        has_more: bool,
    ) -> crate::view_models::recent_feeds::RecentFeedsPageBatch {
        crate::view_models::recent_feeds::RecentFeedsPageBatch {
            rows: rows
                .into_iter()
                .enumerate()
                .map(|(index, (id, label))| {
                    (
                        index as crate::view_models::search_results::SearchResultItemId,
                        release_result(id, label, SearchResultOrigin::Index)
                            .with_thumbnail_href(format!("https://example.test/{index}.jpg")),
                    )
                })
                .collect(),
            cursor: cursor.map(str::to_string),
            has_more,
        }
    }

    #[test]
    fn content_list_row_projects_release_result_contract() {
        let row = ContentListRowDisplay::from_release_result(release_result(
            "release-1",
            "Release",
            SearchResultOrigin::Index,
        ));

        assert_eq!(row.id, "release-1");
        assert_eq!(row.title(), "Release");
        assert_eq!(row.secondary_text(), "12 tracks");
        assert_eq!(row.entity_kind(), ContentListEntityKind::Release);
        // ADR 0084 Decision 5: with no loaded feed detail, the resolved
        // kind has no stated value and no loaded track, so it is Album.
        assert_eq!(
            row.entity_badge,
            ContentListEntityBadgeDisplay {
                kind: ContentListEntityKind::Release,
                label: "Album",
                a11y_label: "Entity type: Release",
            }
        );
        assert_eq!(
            row.library_badge,
            ContentListLibraryBadgeDisplay {
                state: ContentListLibraryBadgeState::NotInLibrary,
                label: None,
                a11y_label: "Library status: Not in library",
            }
        );
    }

    /// R7-03 (ADR 0084 Decision 5): a release row with one loaded track
    /// shows the badge label "Single". The badge kind, used for its color
    /// and its icon, stays `Release`.
    #[test]
    fn adr_0084_release_kind_r7_03_content_list_release_badge_names_the_kind() {
        let remote_feed = crate::views::FeedView {
            tracks: vec![crate::views::TrackView::default()],
            ..crate::views::FeedView::default()
        };
        let row = ContentListRowDisplay::from_release_result(
            release_result("release-1", "Release", SearchResultOrigin::Index)
                .with_remote_feed(remote_feed),
        );

        assert_eq!(
            row.entity_badge,
            ContentListEntityBadgeDisplay {
                kind: ContentListEntityKind::Release,
                label: "Single",
                a11y_label: "Entity type: Release",
            }
        );
    }

    #[test]
    fn content_list_row_projects_track_result_contract() {
        let row = ContentListRowDisplay::from_track_result(track_result(
            "track-1",
            "Track",
            SearchResultOrigin::Index,
        ));

        assert_eq!(row.id, "track-1");
        assert_eq!(row.title(), "Track");
        assert_eq!(row.secondary_text(), "Artist");
        assert_eq!(row.entity_kind(), ContentListEntityKind::Track);
        assert_eq!(
            row.entity_badge,
            ContentListEntityBadgeDisplay {
                kind: ContentListEntityKind::Track,
                label: "Track",
                a11y_label: "Entity type: Track",
            }
        );
        assert_eq!(
            row.library_badge,
            ContentListLibraryBadgeDisplay {
                state: ContentListLibraryBadgeState::NotInLibrary,
                label: None,
                a11y_label: "Library status: Not in library",
            }
        );
    }

    /// ADR 0084 Decision 8 (operator decision of 2026-10-16): with the
    /// three-state filter gone, every mixed row kind shows regardless of
    /// its source.
    #[test]
    fn adr_0084_discover_r6_05_shows_every_row_regardless_of_source() {
        let page = ContentListPageVm::new(vec![
            ContentListRowDisplay::from_release_result(release_result(
                "release",
                "Release",
                SearchResultOrigin::Library,
            )),
            ContentListRowDisplay::from_track_result(track_result(
                "track",
                "Track",
                SearchResultOrigin::Index,
            )),
        ]);

        assert_eq!(
            page.visible_row_ids(),
            ["release", "track"],
            "every mixed row kind must show, library and index alike"
        );
    }

    #[test]
    fn content_list_page_vm_shows_library_and_index_rows_with_state_labels() {
        let page = ContentListPageVm::new(vec![
            content_row("library", ContentListRowSource::Library),
            content_row("index", ContentListRowSource::Index),
        ]);

        assert_eq!(
            page.cached_rows().len(),
            2,
            "cached rows should preserve every row"
        );
        assert_eq!(
            page.visible_rows()
                .iter()
                .map(|row| (row.id.as_str(), row.state_label))
                .collect::<Vec<_>>(),
            [("library", None), ("index", Some("New"))],
            "every row should show with VM-owned state labels"
        );
    }

    #[test]
    fn content_list_page_vm_defaults_to_tiles_view_mode() {
        let page = ContentListPageVm::new(Vec::new());
        let display = page.view_mode_control_display();

        assert_eq!(
            page.view_mode(),
            ContentViewMode::Tiles,
            "Situational ADR 0062 content-list view mode guard: Music content should default to tiles"
        );
        assert_eq!(
            display.selected,
            ContentViewMode::Tiles,
            "Situational ADR 0062 content-list view mode guard: control display should mirror page selection"
        );
        assert_eq!(
            display.options.map(|option| option.mode),
            [ContentViewMode::Tiles, ContentViewMode::List],
            "Situational ADR 0062 content-list view mode guard: control should expose tile and list options"
        );
    }

    #[test]
    fn content_list_page_vm_set_view_mode_preserves_visible_rows() {
        let mut page = ContentListPageVm::new(vec![
            content_row("library", ContentListRowSource::Library),
            content_row("index", ContentListRowSource::Index),
        ]);
        let tile_rows = page
            .visible_row_ids()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();

        page.set_view_mode(ContentViewMode::List);

        assert_eq!(
            page.view_mode(),
            ContentViewMode::List,
            "Situational ADR 0062 content-list view mode guard: selected mode should update"
        );
        assert_eq!(
            page.visible_row_ids()
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>(),
            tile_rows,
            "Situational ADR 0062 content-list view mode guard: tile and list modes should expose the same visible rows"
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
    fn content_list_page_vm_replace_rows_preserves_text_filter() {
        let mut page =
            ContentListPageVm::new(vec![content_row("library", ContentListRowSource::Library)]);
        page.set_text_filter(Some("next".to_string()));

        page.replace_rows(vec![
            content_row("next-library", ContentListRowSource::Library),
            content_row("next-index", ContentListRowSource::Index),
        ]);

        assert_eq!(page.text_filter(), Some("next"));
        assert_eq!(
            page.visible_row_ids(),
            ["next-library", "next-index"],
            "row refresh should preserve the frame-local text filter"
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
        let page = ContentListPageVm::from_tracks(&tracks);

        let visible_rows = page.visible_rows();
        assert_eq!(visible_rows.len(), 2, "both tracks must show together");
        assert_eq!(visible_rows[0].id, "1");
        assert_eq!(visible_rows[0].title(), "Local Track");
        assert_eq!(visible_rows[0].secondary_text(), "Local Artist");
        assert_eq!(visible_rows[0].entity_kind(), ContentListEntityKind::Track);
        assert_eq!(visible_rows[0].source, ContentListRowSource::Library);
        assert_eq!(
            visible_rows[0].library_badge.state,
            ContentListLibraryBadgeState::InLibrary,
            "TrackRow.is_in_library should project local rows as library content"
        );

        assert_eq!(visible_rows[1].id, "2");
        assert_eq!(visible_rows[1].title(), "Remote Feed");
        assert_eq!(visible_rows[1].secondary_text(), "Remote Album");
        assert_eq!(visible_rows[1].entity_kind(), ContentListEntityKind::Track);
        assert_eq!(visible_rows[1].source, ContentListRowSource::Index);
        assert_eq!(
            visible_rows[1].library_badge.state,
            ContentListLibraryBadgeState::NotInLibrary,
            "non-library TrackRow values should project remote rows as index content"
        );
    }

    #[test]
    fn content_list_rows_from_tree_carry_album_artwork() {
        let rows = content_list_rows_from_tree(&library_tree());
        let row = rows
            .iter()
            .find(|row| row.id == "1")
            .expect("projected row with album artwork");

        assert_eq!(
            row.thumbnail_href(),
            Some("saw.jpg"),
            "Situational ADR 0062 content-list artwork guard: tree-derived rows should carry AlbumNode image_href into the shared row contract"
        );
    }

    /// R6a (ADR 0084 Decisions 4 and 8, operator decision of 2026-10-16):
    /// the release page's row projection gives every track of the
    /// release, library and index alike. The release page reads no list
    /// filter.
    #[test]
    fn adr_0084_discover_r6a_release_page_shows_every_track() {
        let mut local = row();
        local.id = 1;
        local.is_in_library = true;
        let mut remote = row();
        remote.id = 2;
        remote.is_in_library = false;
        let album = AlbumNode {
            name: "Selected Ambient Works".into(),
            feed_id: Some(10),
            feed_guid: Some("saw-guid".into()),
            feed_url: Some("https://example.test/saw.xml".into()),
            language: None,
            description: None,
            image_href: None,
            named_feeds: Vec::new(),
            identity_facts: LocalIdentityFacts::default(),
            metadata_facts: Box::<FeedMetadataFacts>::default(),
            stored_values: None,
            tracks: vec![local, remote],
        };

        assert_eq!(
            album.release_track_rows().len(),
            2,
            "the release page must show every track, library and index alike"
        );
    }

    /// ADR 0084 Decision 8: a New releases row that the Library holds
    /// shows "In library" and no "New" mark.
    #[test]
    fn adr_0084_discover_new_release_in_library_shows_its_state() {
        let mut recent = RecentFeedsPageVm::loading();
        recent.finish_load(
            recent_feed_batch(
                vec![
                    ("index-feed:held", "Held Release"),
                    ("index-feed:fresh", "Fresh Release"),
                ],
                None,
                true,
            ),
            false,
        );
        let mut page = ContentListPageVm::new(Vec::new());
        page.replace_recent_feeds_page(&recent);

        page.mark_library_releases(&BTreeSet::from(["held".to_string()]));

        let rows = page.visible_rows();
        assert_eq!(
            rows[0].library_badge.state,
            ContentListLibraryBadgeState::InLibrary,
            "a release in the Library must show its Library state"
        );
        assert_eq!(
            rows[0].state_label, None,
            "a release in the Library is not New"
        );
        assert_eq!(
            rows[1].library_badge.state,
            ContentListLibraryBadgeState::NotInLibrary,
            "a release outside the Library keeps its Index state"
        );
        assert_eq!(
            rows[1].library_badge.label, None,
            "ADR 0084 Decision 8: an Index item shows no state word"
        );
        assert!(
            rows[1].state_label.is_some(),
            "a release outside the Library stays New"
        );
    }

    #[test]
    fn content_list_page_vm_projects_first_recent_music_page() {
        let mut recent = RecentFeedsPageVm::loading();
        recent.finish_load(
            recent_feed_batch(
                vec![
                    ("index-feed:newest", "Newest Release"),
                    ("index-feed:older", "Older Release"),
                ],
                Some("next"),
                true,
            ),
            false,
        );
        let mut page =
            ContentListPageVm::new(vec![content_row("local", ContentListRowSource::Library)]);

        page.replace_recent_feeds_page(&recent);

        assert_eq!(page.source(), ContentListPageSource::RecentMusic);
        assert_eq!(
            page.visible_row_ids(),
            ["index-feed:newest", "index-feed:older"],
            "ADR 0084 Decision 3: New releases shows the Index releases only, newest first, with no Library track"
        );
        assert_eq!(
            page.visible_rows()[0].entity_kind(),
            ContentListEntityKind::Release,
            "Recent Feeds rows project as release content rows"
        );
        assert_eq!(
            page.visible_rows()[0].source,
            ContentListRowSource::Index,
            "Recent Feeds rows remain index-sourced"
        );
        assert_eq!(
            page.thumbnail_sources(),
            [
                (
                    "index-feed:newest".to_string(),
                    "https://example.test/0.jpg".to_string()
                ),
                (
                    "index-feed:older".to_string(),
                    "https://example.test/1.jpg".to_string()
                )
            ],
            "content-list thumbnail sources should be keyed by row id"
        );
        assert_eq!(
            page.load_more_display(),
            Some(ContentListLoadMoreDisplay {
                button_id: "music-content-list-load-more",
                label: "Load more",
                a11y_label: "Load more recent music",
                disabled: false,
            }),
            "has_more should expose a VM-owned load-more display without a cursor"
        );
        assert_eq!(page.page_state_display(), None);
    }

    #[test]
    fn content_list_page_vm_appends_recent_music_pages() {
        let mut recent = RecentFeedsPageVm::loading();
        recent.finish_load(
            recent_feed_batch(
                vec![("index-feed:first", "First Release")],
                Some("next"),
                true,
            ),
            false,
        );
        recent.finish_load(
            recent_feed_batch(vec![("index-feed:second", "Second Release")], None, false),
            true,
        );
        let mut page = ContentListPageVm::new(Vec::new());

        page.replace_recent_feeds_page(&recent);

        assert_eq!(
            page.visible_row_ids(),
            ["index-feed:first", "index-feed:second"],
            "appended recent music pages should extend the same content-list row set"
        );
        assert!(!page.has_more());
    }

    #[test]
    fn content_list_page_vm_hides_load_more_when_recent_music_has_no_more_pages() {
        let mut recent = RecentFeedsPageVm::loading();
        recent.finish_load(
            recent_feed_batch(vec![("index-feed:only", "Only Release")], None, false),
            false,
        );
        let mut page = ContentListPageVm::new(Vec::new());

        page.replace_recent_feeds_page(&recent);

        assert_eq!(page.load_more_display(), None);
        assert!(!page.has_more());
    }

    #[test]
    fn content_list_page_vm_projects_empty_recent_music_result() {
        let mut recent = RecentFeedsPageVm::loading();
        recent.finish_load(recent_feed_batch(Vec::new(), None, false), false);
        let mut page = ContentListPageVm::new(Vec::new());

        page.replace_recent_feeds_page(&recent);

        assert_eq!(
            page.page_state_display(),
            Some(ContentListPageStateDisplay::Empty(
                ContentListEmptyStateDisplay {
                    title: "No recent music",
                    secondary: "MusicIndex did not return recent music.",
                }
            )),
            "empty Recent Feeds results should use the recency empty display"
        );
    }

    #[test]
    fn content_list_page_vm_projects_failed_recent_music_load() {
        let mut recent = RecentFeedsPageVm::loading();
        recent.fail_load("Recent music unavailable", "network failed", false);
        let mut page = ContentListPageVm::new(Vec::new());

        page.replace_recent_feeds_page(&recent);

        assert_eq!(
            page.page_state_display(),
            Some(ContentListPageStateDisplay::Failed(
                ContentListFailedStateDisplay {
                    title: "Recent music unavailable".to_string(),
                    detail: "network failed".to_string(),
                }
            )),
            "failed recency loads should expose VM-owned failure copy"
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

    /// R13-01 (ADR 0084 task 013): `DescriptionDisclosureDisplay::project`
    /// gives the four cases of the design table.
    #[test]
    fn adr_0084_index_page_r13_01_description_disclosure_display_projects_four_cases() {
        let long = "word ".repeat(100);
        let short = "A short description.";

        let long_visible =
            DescriptionDisclosureDisplay::project(&long, DescriptionState::AutoExpanded);
        assert!(!long_visible.collapsed);
        assert_eq!(long_visible.body, Some(long.clone()));
        assert!(long_visible.show_more.is_none());

        let long_collapsed =
            DescriptionDisclosureDisplay::project(&long, DescriptionState::AutoCollapsed);
        assert!(long_collapsed.collapsed);
        let preview = long_collapsed
            .body
            .expect("a long, collapsed description shows a preview");
        assert!(preview.len() < long.len());
        assert_eq!(
            long_collapsed.show_more,
            Some(DescriptionShowMoreDisplay {
                label: "Show more",
                a11y_label: "Show the full description",
            })
        );

        let short_visible =
            DescriptionDisclosureDisplay::project(short, DescriptionState::UserExpanded);
        assert!(!short_visible.collapsed);
        assert_eq!(short_visible.body, Some(short.to_string()));
        assert!(short_visible.show_more.is_none());

        let short_collapsed =
            DescriptionDisclosureDisplay::project(short, DescriptionState::UserCollapsed);
        assert!(short_collapsed.collapsed);
        assert_eq!(short_collapsed.body, None);
        assert!(short_collapsed.show_more.is_none());
    }

    /// R13-02 (ADR 0084 task 013): the preview of a long description ends
    /// with "…", cuts at a word boundary, keeps a multi-byte character
    /// whole, and is shorter than the full text.
    #[test]
    fn adr_0084_index_page_r13_02_preview_cuts_at_word_boundary_and_keeps_multi_byte_chars() {
        // "é" is a two-byte UTF-8 character. The budget cut must fall on a
        // character boundary and on a word boundary, never inside "é".
        let long = "é ".repeat(300);

        let display = DescriptionDisclosureDisplay::project(&long, DescriptionState::AutoCollapsed);
        let body = display
            .body
            .expect("a long, collapsed description shows a preview");

        assert!(body.ends_with('…'), "preview must end with an ellipsis");
        assert!(
            body.chars().count() < long.chars().count(),
            "preview must be shorter than the full text"
        );
        let without_ellipsis = body.trim_end_matches('…');
        assert!(
            long.starts_with(without_ellipsis),
            "preview must be a prefix of the full text"
        );
        assert!(
            without_ellipsis.ends_with('é'),
            "preview must keep a multi-byte character whole, not split"
        );
    }

    /// R13-03 (ADR 0084 task 013): `description_line_count` counts a
    /// single 600-character paragraph as more than
    /// `DESCRIPTION_AUTO_COLLAPSE_LINES` lines, and a short paragraph as 1.
    #[test]
    fn adr_0084_index_page_r13_03_description_line_count_estimates_wrapped_lines() {
        let long = "a".repeat(600);
        assert!(description_line_count(Some(&long)) > DESCRIPTION_AUTO_COLLAPSE_LINES);

        let short = "a".repeat(10);
        assert_eq!(description_line_count(Some(&short)), 1);
    }

    /// R13-04 (ADR 0084 task 013): the Index description state of a feed
    /// GUID starts from `DescriptionState::project` and changes with each
    /// toggle, through a pure function with no GPUI.
    #[test]
    fn adr_0084_index_page_r13_04_keyed_description_state_is_sticky_per_key() {
        let mut states: BTreeMap<String, DescriptionState> = BTreeMap::new();
        let long = "a".repeat(600);

        assert_eq!(
            keyed_description_state(&states, "feed-a", &long),
            DescriptionState::AutoCollapsed
        );

        toggle_keyed_description_state(&mut states, "feed-a", &long);
        assert_eq!(
            keyed_description_state(&states, "feed-a", "short"),
            DescriptionState::UserExpanded
        );
        assert_eq!(
            keyed_description_state(&states, "feed-b", "short"),
            DescriptionState::AutoExpanded
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

    /// R83-46: "Inspect sources" is closed by default, and a toggle opens
    /// and closes it.
    #[test]
    fn adr_0083_r83_46_inspect_sources_starts_closed_and_toggles() {
        let mut state = LibraryTrackInspectorState::default();
        assert!(!state.display(true).inspect_sources_open());

        state.toggle_panel(InspectorPanelKind::Sources);
        assert!(state.display(true).inspect_sources_open());
        state.toggle_panel(InspectorPanelKind::Sources);
        assert!(!state.display(true).inspect_sources_open());
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
        let action = vm.primary_action_vm(TrackRowWork::Idle);
        assert_eq!(
            action.kind,
            crate::view_models::entity_detail::EntityActionKind::Download
        );
        assert_eq!(action.label, "Download");
        assert!(action.enabled);

        let action = vm.primary_action_vm(TrackRowWork::Downloading);
        assert_eq!(action.label, "Downloading...");
        assert!(!action.enabled);

        r.is_in_library = true;
        let vm = LibraryTrackRowVm::new(&r, None);
        let action = vm.primary_action_vm(TrackRowWork::Idle);
        assert_eq!(
            action.kind,
            crate::view_models::entity_detail::EntityActionKind::Remove
        );
        assert_eq!(action.label, "Remove");
        assert!(action.enabled);

        let action = vm.primary_action_vm(TrackRowWork::Removing);
        assert_eq!(action.label, "Removing...");
        assert!(!action.enabled);
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
                state_label: Some("New"),
            }
        );

        r.is_in_library = true;
        assert_eq!(
            LibraryTrackRowVm::new(&r, None).row_display().state_label,
            None
        );
    }

    #[test]
    fn library_track_row_vm_projects_shared_primary_action() {
        let mut r = row();
        r.id = 42;
        let vm = LibraryTrackRowVm::new(&r, None);
        let action = vm.primary_action_vm(TrackRowWork::Idle);
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
        let action = vm.primary_action_vm(TrackRowWork::Idle);
        assert_eq!(
            action.kind,
            crate::view_models::entity_detail::EntityActionKind::Remove
        );
        assert_eq!(
            action.tone,
            crate::view_models::entity_detail::EntityActionTone::DestructiveQuiet
        );
        assert!(action.enabled);

        let action = vm.primary_action_vm(TrackRowWork::Removing);
        assert_eq!(action.label, "Removing...");
        assert!(!action.enabled);
    }

    #[test]
    fn library_track_row_vm_local_path_does_not_change_row_action_text() {
        let mut r = row();
        r.local_path = Some(crate::library_path::LibraryRelativePath::for_test(
            "music/track.mp3",
        ));
        let vm = LibraryTrackRowVm::new(&r, None);
        assert_eq!(vm.primary_action_vm(TrackRowWork::Idle).label, "Download");
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

    /// R1-06: the Library artist rows come from local tracks only. No source
    /// subject, alias, area or active-years row exists (ADR 0079).
    #[test]
    fn adr_0077_remove_artist_storage_library_artist_rows_have_no_source_subject() {
        let mut downloaded = track_for_feed(1, Some("A"));
        downloaded.local_path = Some(crate::library_path::LibraryRelativePath::for_test("x"));
        let tracks = [downloaded, track_for_feed(2, Some("B"))];
        let vm = LibraryArtistDetailVm::new("Artist", &tracks);

        assert_eq!(
            vm.detail_rows(),
            vec![
                ("Albums".to_string(), "2".to_string()),
                ("Tracks".to_string(), "2 tracks".to_string()),
                ("Downloaded".to_string(), "1".to_string()),
            ]
        );
        let page = vm.page();
        assert_eq!(page.detail_rows.len(), 3);
        assert_eq!(page.subtitle.as_deref(), Some("Grouped by name"));
    }

    #[test]
    fn artist_detail_vm_includes_downloaded_row_when_any_local_path_present() {
        let mut t1 = track_for_feed(1, Some("A"));
        t1.local_path = Some(crate::library_path::LibraryRelativePath::for_test("x"));
        let t2 = track_for_feed(1, Some("A"));
        let tracks = [t1, t2];
        let vm = LibraryArtistDetailVm::new("Artist", &tracks);
        let rows = vm.detail_rows();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[2], ("Downloaded".into(), "1".into()));
    }

    #[test]
    fn artist_detail_page_projects_library_header_rows_and_feed_policy() {
        let tracks = [track_for_feed(1, Some("A"))];
        let vm = LibraryArtistDetailVm::new("", &tracks);
        let page = vm.page();

        assert_eq!(page.title, "Unknown");
        assert_eq!(page.subtitle.as_deref(), Some("Grouped by name"));
        assert!(page.shows_feed_section);
        assert_eq!(page.detail_rows[0].key, "Albums");
        assert_eq!(page.detail_rows[0].value, "1");
        assert_eq!(page.detail_rows[0].max_lines, 6);
    }

    /// R5-04 (ADR 0077 Decision 6, "Album without a publisher"
    /// refinement): the Library name-grouped artist page shows the
    /// "Grouped by name" label. The grouping has no publisher
    /// relationship, no role and no page type.
    #[test]
    fn adr_0077_feed_owner_library_name_grouping_shows_grouped_by_name() {
        let tracks = [track_for_feed(1, Some("A"))];
        let vm = LibraryArtistDetailVm::new("Artist", &tracks);

        assert_eq!(vm.page().subtitle.as_deref(), Some("Grouped by name"));
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

    /// The operator found on 2026-10-18 that "The Heycitizen Experience"
    /// showed no cover on the artist page, because its first track has no
    /// image. The cover comes from any track of the release.
    #[test]
    fn artist_detail_vm_thumb_url_reads_each_track_of_the_release() {
        let first = track_for_feed(1, Some("A"));
        let mut second = track_for_feed(1, Some("A"));
        second.track_image_href = Some("track-img".into());
        let mut third = track_for_feed(1, Some("A"));
        third.album_image_href = Some("album-img".into());
        let tracks = [first.clone(), second.clone()];
        let vm = LibraryArtistDetailVm::new("Artist", &tracks);
        assert_eq!(
            vm.feed_summaries()[0].thumb_url.as_deref(),
            Some("track-img")
        );

        let tracks = [first, second, third];
        let vm = LibraryArtistDetailVm::new("Artist", &tracks);
        assert_eq!(
            vm.feed_summaries()[0].thumb_url.as_deref(),
            Some("album-img")
        );
    }

    /// The operator found on 2026-10-18 that a release row showed "FD",
    /// because the Library loaded only the header image of the artist page.
    /// The page now gives the image URL of each release row.
    #[test]
    fn artist_detail_vm_thumbnail_urls_give_each_release_row_one_time() {
        let mut a1 = track_for_feed(1, Some("A"));
        a1.album_image_href = Some("cover-a".into());
        let mut a2 = track_for_feed(1, Some("A"));
        a2.album_image_href = Some("cover-a".into());
        let mut b = track_for_feed(2, Some("B"));
        b.track_image_href = Some("cover-b".into());
        let c = track_for_feed(3, Some("C"));
        let tracks = [a1, a2, b, c];
        let vm = LibraryArtistDetailVm::new("Artist", &tracks);
        assert_eq!(
            vm.thumbnail_urls(),
            vec!["cover-a".to_string(), "cover-b".to_string()]
        );
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
                check_rss: crate::view_models::playlist_rss_check::PlaylistCheckRssActionDisplay {
                    button_id: "playlist-check-rss-42".into(),
                    label: "Check RSS",
                    a11y_label: "The RSS check is not available because the background runtime did not start",
                    availability: crate::view_models::playlist_rss_check::PlaylistRssCheckAvailability::RuntimeUnavailable,
                    enabled: false,
                },
            }
        );
    }

    /// R1-13: the "Check RSS" action is unavailable during a run and for an
    /// empty playlist (ADR 0076 Decision 2).
    #[test]
    fn adr_0076_playlist_check_action_unavailable_while_running_or_empty() {
        use crate::runtime::playlist_rss_check::test_support::running_snapshot;
        use crate::view_models::playlist_rss_check::PlaylistRssCheckAvailability;

        let mut pl = playlist("Show");
        pl.id = 7;
        let tracks = vec![row()];
        let idle = crate::runtime::PlaylistRssCheckSnapshot::default();
        let running = running_snapshot(7);

        let action = PlaylistDetailVm::new(&pl, &tracks)
            .with_rss_check(Some(&idle))
            .actions_display()
            .check_rss;
        assert_eq!(action.availability, PlaylistRssCheckAvailability::Available);
        assert!(action.enabled);

        let vm = PlaylistDetailVm::new(&pl, &tracks).with_rss_check(Some(&running));
        let action = vm.actions_display().check_rss;
        assert_eq!(action.availability, PlaylistRssCheckAvailability::Running);
        assert!(!action.enabled);
        let report = vm.rss_check_report().expect("running report");
        assert!(report.running);
        assert_eq!(report.rows.len(), 1);

        let action = PlaylistDetailVm::new(&pl, &[])
            .with_rss_check(Some(&idle))
            .actions_display()
            .check_rss;
        assert_eq!(
            action.availability,
            PlaylistRssCheckAvailability::EmptyPlaylist
        );
        assert!(!action.enabled);
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
        t.local_path = Some(crate::library_path::LibraryRelativePath::for_test("x"));
        let tracks = [t, row()];
        let vm = PlaylistDetailVm::new(&pl, &tracks);
        let rows = vm.track_rows();
        assert!(rows[0].can_play());
        assert!(!rows[1].can_play());
        // Situational ADR 0066: player failure only limits Play, preserving row identity and reorder.
        let ready = rows[0].display_with_playback(pl.id, Ok(()));
        let unavailable = rows[0].display_with_playback(
            pl.id,
            Err(
                crate::application::capability::ExecutionUnavailable::configured(
                    crate::application::capability::Dependency::Playback,
                ),
            ),
        );
        assert!(ready.controls.play_enabled);
        assert!(unavailable.controls.play_enabled);
        assert_eq!(
            unavailable.controls.play_affordance,
            PlaylistPlayAffordance::RepairPlayback
        );
        let mut restored = unavailable;
        restored.controls.play_affordance = ready.controls.play_affordance;
        restored.controls.play_a11y_label = ready.controls.play_a11y_label;
        restored.controls.play_visibility = ready.controls.play_visibility;
        assert_eq!(restored, ready);
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

    fn playlist_menu_actions(
        controls: &PlaylistTrackControlsDisplay,
    ) -> Vec<crate::view_models::release_page::TrackRowMenuAction> {
        controls
            .menu
            .as_ref()
            .map(|menu| menu.items.iter().map(|item| item.action).collect())
            .unwrap_or_default()
    }

    #[test]
    fn playlist_track_row_vm_reorder_menu_availability_tracks_boundaries() {
        use crate::view_models::release_page::TrackRowMenuAction::{MoveDown, MoveUp};
        let pl = playlist("Mix");
        let tracks = [row(), row(), row()];
        let vm = PlaylistDetailVm::new(&pl, &tracks);
        let rows = vm.track_rows();

        let first = playlist_menu_actions(&rows[0].controls_display(pl.id));
        assert!(!first.contains(&MoveUp));
        assert!(first.contains(&MoveDown));

        let middle = playlist_menu_actions(&rows[1].controls_display(pl.id));
        assert!(middle.contains(&MoveUp));
        assert!(middle.contains(&MoveDown));

        let last = playlist_menu_actions(&rows[2].controls_display(pl.id));
        assert!(last.contains(&MoveUp));
        assert!(!last.contains(&MoveDown));
    }

    #[test]
    fn playlist_track_row_vm_controls_display_projects_ids_labels_and_availability() {
        let pl = playlist("Mix");
        let mut t1 = row();
        t1.id = 42;
        t1.is_in_library = true;
        t1.local_path = Some(crate::library_path::LibraryRelativePath::for_test("x"));
        let mut t2 = row();
        t2.id = 43;
        let tracks = [t1, t2];
        let vm = PlaylistDetailVm::new(&pl, &tracks);
        let rows = vm.track_rows();

        let controls = rows[0].controls_display(7);
        assert_eq!(controls.row_id, "playlist-track-42-0");
        assert_eq!(controls.row_body_id, "playlist-row-body-7-0");
        assert_eq!(controls.drag_handle_id, "playlist-drag-handle-7-0");
        assert_eq!(
            controls.drag_handle_a11y_label,
            "Drag to reorder playlist track"
        );
        assert_eq!(controls.play_button_id, "playlist-play-7-0");
        assert_eq!(controls.play_affordance, PlaylistPlayAffordance::Play);
        assert_eq!(controls.play_a11y_label, "Play this playlist track");
        assert!(controls.play_enabled);
        assert_eq!(controls.play_visibility, RowCellVisibility::OnHover);
        assert_eq!(controls.checkbox_id, "playlist-checkbox-7-0");
        assert!(controls.reorder_visible);
        let menu = controls.menu.expect("a row in Normal has a menu");
        assert_eq!(menu.id, "playlist-actions-7-0-menu");

        let second = rows[1].controls_display(7);
        assert!(!second.play_enabled);
        let actions = playlist_menu_actions(&second);
        assert!(actions.contains(&crate::view_models::release_page::TrackRowMenuAction::MoveUp));
        assert!(!actions.contains(&crate::view_models::release_page::TrackRowMenuAction::MoveDown));
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
        t.local_path = Some(crate::library_path::LibraryRelativePath::for_test("x"));
        let tracks = [t, row()];
        let vm = PlaylistDetailVm::new(&pl, &tracks);
        let rows = vm.track_rows();
        let display = rows[0].display(pl.id);

        assert_eq!(
            display,
            PlaylistTrackRowDisplay {
                is_available: true,
                position: 0,
                position_label: "1.".into(),
                title: "Song".into(),
                artist: "Artist".into(),
                availability_label: None,
                removed_from_feed: None,
                duration_label: "2:05".into(),
                thumb_url: Some("track".into()),
                controls: rows[0].controls_display(pl.id),
                selection: RowSelectionVm::none(),
            }
        );
    }

    /// R25-09: in `Selecting`, the checkbox shows and `reorder_visible` is
    /// false. In `Normal`, the opposite.
    #[test]
    fn adr_0084_playlist_selection_r25_09_reorder_follows_the_mode() {
        let pl = playlist("Mix");
        let tracks = [row()];
        let vm = PlaylistDetailVm::new(&pl, &tracks);
        let display = || vm.track_rows()[0].display(pl.id);

        let selecting =
            display().with_selection(RowSelectionVm::for_row("Song", true, false, true));
        assert!(selecting.selection.checkbox_visible);
        assert!(!selecting.controls.reorder_visible);

        let normal = display().with_selection(RowSelectionVm::for_row("Song", false, false, false));
        assert!(!normal.selection.checkbox_visible);
        assert!(normal.controls.reorder_visible);
        assert!(display().controls.reorder_visible);
    }

    #[test]
    fn playlist_track_row_vm_marks_out_of_library_rows_unavailable() {
        let pl = playlist("Mix");
        let mut t = row();
        t.id = 42;
        t.local_path = Some(crate::library_path::LibraryRelativePath::for_test("x"));
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
                            named_feeds: Vec::new(),
                            identity_facts: LocalIdentityFacts::default(),
                            metadata_facts: Box::<FeedMetadataFacts>::default(),
                            stored_values: None,
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
                            named_feeds: Vec::new(),
                            identity_facts: LocalIdentityFacts::default(),
                            metadata_facts: Box::<FeedMetadataFacts>::default(),
                            stored_values: None,
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
                        named_feeds: Vec::new(),
                        identity_facts: LocalIdentityFacts::default(),
                        metadata_facts: Box::<FeedMetadataFacts>::default(),
                        stored_values: None,
                        tracks: vec![tri_repetae],
                    }],
                },
            ],
        }
    }

    #[test]
    fn album_detail_vm_has_active_musicbrainz_when_any_track_pending_or_processing() {
        let mut tracks = [row(), row(), row()];
        tracks[0].id = 10;
        tracks[1].id = 20;
        tracks[2].id = 30;
        let mut mb: BTreeMap<i64, MbTrackStatus> = BTreeMap::new();
        mb.insert(10, MbTrackStatus::Done(2));
        let vm = LibraryAlbumDetailVm::new(&tracks, &mb);
        assert!(!vm.has_active_musicbrainz());
        mb.insert(20, MbTrackStatus::Pending);
        let vm = LibraryAlbumDetailVm::new(&tracks, &mb);
        assert!(vm.has_active_musicbrainz());
        mb.insert(20, MbTrackStatus::Processing);
        let vm = LibraryAlbumDetailVm::new(&tracks, &mb);
        assert!(vm.has_active_musicbrainz());
        mb.insert(20, MbTrackStatus::Skipped);
        let vm = LibraryAlbumDetailVm::new(&tracks, &mb);
        assert!(!vm.has_active_musicbrainz());
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
    fn library_view_model_updates_album_metadata_facts_by_feed_id() {
        let mut vm = LibraryViewModel::new();
        vm.replace_tree(library_tree());
        let facts = FeedMetadataFacts {
            publisher_text: Some("Example Publisher".into()),
            release_kind: Some("album".into()),
            release_date: Some(1_700_000_000),
            language: Some("en".into()),
            explicit: Some(true),
            description: Some("Fact description".into()),
            ..FeedMetadataFacts::default()
        };

        assert!(vm.update_album_metadata_facts(10, &facts));
        let album = &vm.tree().artists[0].albums[0];
        assert_eq!(
            album.metadata_facts.publisher_text.as_deref(),
            Some("Example Publisher")
        );
        assert_eq!(album.metadata_facts.release_kind.as_deref(), Some("album"));
        assert_eq!(album.metadata_facts.release_date, Some(1_700_000_000));
        assert_eq!(album.metadata_facts.language.as_deref(), Some("en"));
        assert_eq!(album.metadata_facts.explicit, Some(true));
        assert_eq!(
            album.metadata_facts.description.as_deref(),
            Some("Fact description")
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

    /// R3-04 (ADR 0084 Decision 4): the content-list view model gives a
    /// `TrackKey` open target for a Library track row, and gives no track
    /// key for an Index release row's id.
    #[test]
    fn adr_0084_track_r3_04_content_list_track_row_key_is_typed_by_row_kind() {
        let mut vm = LibraryViewModel::new();
        vm.replace_tree(library_tree());

        let rhubarb = &vm.tree().artists[0].albums[0].tracks[0];
        let rhubarb_key = rhubarb.track_key();
        assert_eq!(
            vm.content_list_track_row_key(&rhubarb.id.to_string()),
            Some(rhubarb_key),
            "a Library track row must give its TrackKey"
        );
        assert_eq!(
            vm.content_list_track_row_key("index-feed:saw-guid"),
            None,
            "an Index release row id must not parse as a track id"
        );

        // A release row with a numeric id is never a track row, even when a
        // Library track has that id (orchestrator review, 2026-10-11).
        let page = ContentListPageVm::new(vec![ContentListRowDisplay::from_release_result(
            release_result(
                &rhubarb.id.to_string(),
                "Release",
                SearchResultOrigin::Library,
            ),
        )]);
        assert!(
            !page.is_library_track_row(&rhubarb.id.to_string()),
            "a release row must not open a track page"
        );
    }

    // ADR 0083 task 007: the list and playlist row actions.

    fn menu_items(
        menu: Option<&TrackRowMenuVm>,
    ) -> Vec<crate::view_models::release_page::TrackRowMenuAction> {
        menu.map(|menu| menu.items.iter().map(|item| item.action).collect())
            .unwrap_or_default()
    }

    fn downloaded_track(id: i64, title: &str) -> TrackRow {
        let mut track = row();
        track.id = id;
        track.track_title = Some(title.into());
        track.is_in_library = true;
        track.local_path = Some(crate::library_path::LibraryRelativePath::for_test("x"));
        track
    }

    const ALL_WORK: [TrackRowWork; 4] = [
        TrackRowWork::Idle,
        TrackRowWork::Queued,
        TrackRowWork::Downloading,
        TrackRowWork::Removing,
    ];

    /// R83-7-01 (ADR 0083 Decision 6, task 007 decision D5): a downloaded
    /// idle track in the middle of a playlist gives each item in order.
    #[test]
    fn adr_0083_list_row_actions_r83_7_01_playlist_menu_order() {
        use crate::view_models::release_page::TrackRowMenuAction::{
            AddToPlaylist, InspectSources, MoveDown, MoveUp, Remove, RemoveFromPlaylist,
        };
        let track = downloaded_track(42, "Face Pale");
        let controls = PlaylistTrackRowVm::new(&track, 1, 2).controls_display(7);
        assert_eq!(
            menu_items(controls.menu.as_ref()),
            vec![
                AddToPlaylist,
                InspectSources,
                MoveUp,
                MoveDown,
                RemoveFromPlaylist,
                Remove
            ]
        );
        let labels: Vec<_> = controls
            .menu
            .expect("a row in Normal has a menu")
            .items
            .iter()
            .map(|item| item.label)
            .collect();
        assert_eq!(
            labels,
            vec![
                "Add to playlist",
                "Inspect sources",
                "Move up",
                "Move down",
                "Remove from playlist",
                "Remove from Library\u{2026}",
            ]
        );
    }

    /// R83-7-02 (decision D5): the first row has no "Move up", the last row
    /// has no "Move down", and a track that is not downloaded gives
    /// "Download" and no "Remove from Library…".
    #[test]
    fn adr_0083_list_row_actions_r83_7_02_playlist_menu_absent_items() {
        use crate::view_models::release_page::TrackRowMenuAction::{
            Download, MoveDown, MoveUp, Remove,
        };
        let track = downloaded_track(42, "Face Pale");
        let first = menu_items(
            PlaylistTrackRowVm::new(&track, 0, 2)
                .controls_display(7)
                .menu
                .as_ref(),
        );
        assert!(!first.contains(&MoveUp));
        assert!(first.contains(&MoveDown));
        let last = menu_items(
            PlaylistTrackRowVm::new(&track, 2, 2)
                .controls_display(7)
                .menu
                .as_ref(),
        );
        assert!(last.contains(&MoveUp));
        assert!(!last.contains(&MoveDown));

        let mut remote = row();
        remote.id = 43;
        remote.track_title = Some("Remote".into());
        let remote = menu_items(
            PlaylistTrackRowVm::new(&remote, 1, 2)
                .controls_display(7)
                .menu
                .as_ref(),
        );
        assert!(remote.contains(&Download));
        assert!(!remote.contains(&Remove));
    }

    /// R83-7-03 (decisions D6 and D7): "Remove from playlist" is not
    /// destructive. "Remove from Library…" is last and destructive, and
    /// its accessibility label names the track.
    #[test]
    fn adr_0083_list_row_actions_r83_7_03_remove_items() {
        use crate::view_models::release_page::TrackRowMenuAction::{Remove, RemoveFromPlaylist};
        let track = downloaded_track(42, "Face Pale");
        let menu = PlaylistTrackRowVm::new(&track, 1, 2)
            .controls_display(7)
            .menu
            .expect("a row in Normal has a menu");
        let from_playlist = menu
            .items
            .iter()
            .find(|item| item.action == RemoveFromPlaylist)
            .expect("\"Remove from playlist\" shows");
        assert!(!from_playlist.destructive);
        assert!(!from_playlist.label.ends_with('\u{2026}'));
        let last = menu.items.last().expect("the menu has items");
        assert_eq!(last.action, Remove);
        assert!(last.destructive);
        assert!(last.label.ends_with('\u{2026}'));
        assert_eq!(last.a11y_label, "Remove Face Pale from the Library\u{2026}");
        assert_eq!(
            menu.items.iter().filter(|item| item.destructive).count(),
            1,
            "only \"Remove from Library…\" is destructive"
        );
    }

    /// R83-7-04 (decision D7): "Remove from Library…" and "Download" are
    /// absent while a job or a removal runs.
    #[test]
    fn adr_0083_list_row_actions_r83_7_04_work_hides_download_and_remove() {
        use crate::view_models::release_page::TrackRowMenuAction::{Download, Remove};
        let downloaded = downloaded_track(42, "Face Pale");
        let mut remote = row();
        remote.id = 43;
        for work in [
            TrackRowWork::Queued,
            TrackRowWork::Downloading,
            TrackRowWork::Removing,
        ] {
            for track in [&downloaded, &remote] {
                let items = menu_items(
                    PlaylistTrackRowVm::new(track, 1, 2)
                        .with_work(work)
                        .controls_display(7)
                        .menu
                        .as_ref(),
                );
                assert!(!items.contains(&Download), "{work:?}");
                assert!(!items.contains(&Remove), "{work:?}");
            }
        }
    }

    /// R83-7-05 (decision D8): the "Add to playlist" choices of a playlist
    /// row do not hold the open playlist.
    #[test]
    fn adr_0083_list_row_actions_r83_7_05_choices_leave_out_open_playlist() {
        let mut open = playlist("Open");
        open.id = 7;
        let mut other = playlist("Other");
        other.id = 8;
        let ids: Vec<_> = playlist_row_add_choices(&[open, other], 7)
            .into_iter()
            .map(|option| option.id)
            .collect();
        assert_eq!(ids, vec![8]);
    }

    /// R83-7-06 (decision D4): "Repair playback" shows at all times, and
    /// the Play icon shows on hover.
    #[test]
    fn adr_0083_list_row_actions_r83_7_06_play_visibility() {
        let track = downloaded_track(42, "Face Pale");
        let vm = PlaylistTrackRowVm::new(&track, 0, 1);
        let ready = vm.display_with_playback(7, Ok(()));
        assert_eq!(ready.controls.play_affordance, PlaylistPlayAffordance::Play);
        assert_eq!(ready.controls.play_visibility, RowCellVisibility::OnHover);
        let repair = vm.display_with_playback(
            7,
            Err(
                crate::application::capability::ExecutionUnavailable::configured(
                    crate::application::capability::Dependency::Playback,
                ),
            ),
        );
        assert_eq!(
            repair.controls.play_affordance,
            PlaylistPlayAffordance::RepairPlayback
        );
        assert_eq!(repair.controls.play_visibility, RowCellVisibility::Always);
    }

    /// R83-7-07 (decision D12): in `Selecting`, the playlist row has no
    /// menu and no drag handle. In `Normal`, it has a menu.
    #[test]
    fn adr_0083_list_row_actions_r83_7_07_selection_mode_hides_the_menu() {
        let track = downloaded_track(42, "Face Pale");
        let display = || PlaylistTrackRowVm::new(&track, 0, 1).display(7);
        let selecting =
            display().with_selection(RowSelectionVm::for_row("Face Pale", true, false, true));
        assert!(selecting.controls.menu.is_none());
        assert!(!selecting.controls.reorder_visible);
        let normal =
            display().with_selection(RowSelectionVm::for_row("Face Pale", false, false, false));
        assert!(normal.controls.menu.is_some());
        assert!(normal.controls.reorder_visible);
    }

    /// R83-7-08 (decision D9): a Tracks list row gives the items of task
    /// 006 decision D2 for each pair of `is_in_library` and the work. A row
    /// with no track key gives no menu.
    #[test]
    fn adr_0083_list_row_actions_r83_7_08_tracks_list_menu() {
        use crate::view_models::release_page::TrackRowMenuAction::{
            AddToPlaylist, Download, InspectSources, Remove,
        };
        for in_library in [true, false] {
            for work in ALL_WORK {
                let mut track = downloaded_track(42, "Face Pale");
                track.is_in_library = in_library;
                let menu = content_list_track_menu("42", "Face Pale", &track, work, None);
                let idle = work == TrackRowWork::Idle;
                let mut expected = vec![AddToPlaylist];
                if !in_library && idle {
                    expected.push(Download);
                }
                expected.push(InspectSources);
                if in_library && idle {
                    expected.push(Remove);
                }
                assert_eq!(
                    menu_items(menu.as_ref()),
                    expected,
                    "in_library {in_library}, {work:?}"
                );
                if let Some(remove) = menu
                    .iter()
                    .flat_map(|menu| &menu.items)
                    .find(|item| item.action == Remove)
                {
                    assert_eq!(remove.label, "Remove\u{2026}");
                    assert!(remove.destructive);
                }
            }
        }

        let mut vm = LibraryViewModel::new();
        vm.replace_tree(library_tree());
        vm.content_list_page = ContentListPageVm::new(vec![
            ContentListRowDisplay::from_track(&vm.tree().artists[0].albums[0].tracks[0]),
            ContentListRowDisplay::from_release_result(release_result(
                "index-feed:saw-guid",
                "Release",
                SearchResultOrigin::Index,
            )),
        ]);
        let rows =
            vm.content_list_rows_with_selection(&ListSelection::new(), SelectionMode::Normal);
        assert!(rows[0].menu.is_some(), "a Library track row has a menu");
        assert!(
            rows[1].menu.is_none(),
            "a row with no track key has no menu"
        );
        assert!(rows.iter().all(|row| row.fix.is_none()));
    }

    /// R83-7-09 (decision D10): each Needs attention row gives its one fix
    /// and its menu. The menu never holds a fix.
    #[test]
    fn adr_0083_list_row_actions_r83_7_09_needs_attention_fix_and_menu() {
        use crate::application::queries::broadcast::BroadcastReadinessState as State;
        use crate::view_models::release_page::TrackRowMenuAction::{InspectSources, Remove};
        let track = downloaded_track(9, "Gone Song");
        for state in [
            State::Ready,
            State::NoRouteTag,
            State::NoRoutesUpstream,
            State::NotDownloaded,
            State::FileMissing,
            State::RouteOutOfDate,
            State::RemovedFromFeed,
        ] {
            let row = ContentListRowDisplay::from_broadcast_readiness_track(
                &broadcast_readiness_track(9, "Gone Song", state, "Reason."),
                false,
            );
            let fix = row.fix.as_ref().map(|fix| (fix.kind, fix.label.as_str()));
            let expected_fix = match state {
                State::NoRouteTag => Some((
                    ContentListRowActionKind::RepairBroadcastRoutes { track_id: 9 },
                    "Fix routes",
                )),
                State::RemovedFromFeed => Some((
                    ContentListRowActionKind::ConfirmRemovedFromFeed { track_id: 9 },
                    "Confirm",
                )),
                _ => None,
            };
            assert_eq!(fix, expected_fix, "{state:?}");

            let menu = content_list_track_menu(
                &row.id,
                row.title(),
                &track,
                TrackRowWork::Idle,
                Some(state),
            );
            let expected_menu = if state == State::RemovedFromFeed {
                vec![InspectSources, Remove]
            } else {
                vec![InspectSources]
            };
            assert_eq!(menu_items(menu.as_ref()), expected_menu, "{state:?}");
            if let Some(remove) = menu
                .iter()
                .flat_map(|menu| &menu.items)
                .find(|item| item.action == Remove)
            {
                assert_eq!(remove.label, "Remove from Library\u{2026}");
                assert!(remove.destructive);
            }
        }
    }

    /// R83-7-11 (decision D12): in `Selecting`, each Tracks and Needs
    /// attention row has no menu and no fix.
    #[test]
    fn adr_0083_list_row_actions_r83_7_11_selection_mode_hides_menu_and_fix() {
        use crate::application::queries::broadcast::BroadcastReadinessState as State;
        let mut vm = LibraryViewModel::new();
        vm.replace_tree(library_tree());
        let rhubarb = vm.tree().artists[0].albums[0].tracks[0].clone();
        let cliffs = vm.tree().artists[0].albums[0].tracks[1].clone();
        let mut needs_attention = ContentListRowDisplay::from_broadcast_readiness_track(
            &broadcast_readiness_track(cliffs.id, "Cliffs", State::NoRouteTag, "Reason."),
            false,
        );
        needs_attention.source = ContentListRowSource::Library;
        vm.content_list_page = ContentListPageVm::new(vec![
            ContentListRowDisplay::from_track(&rhubarb),
            needs_attention,
        ]);
        let normal =
            vm.content_list_rows_with_selection(&ListSelection::new(), SelectionMode::Normal);
        assert!(normal.iter().all(|row| row.menu.is_some()));
        assert!(normal[1].fix.is_some());
        let selecting =
            vm.content_list_rows_with_selection(&ListSelection::new(), SelectionMode::Selecting);
        for row in &selecting {
            assert!(row.menu.is_none(), "{}", row.id);
            assert!(row.fix.is_none(), "{}", row.id);
        }
    }

    #[test]
    fn library_view_model_starts_with_default_sort() {
        let vm = LibraryViewModel::new();
        assert_eq!(vm.playlist_sort, PlaylistSort::Name);
    }

    #[test]
    fn library_view_model_starts_with_no_selection_or_operation_in_flight() {
        let vm = LibraryViewModel::new();
        assert_eq!(vm.selected_id, None);
        assert_eq!(vm.selected_playlist_id, None);
        assert_eq!(vm.hovered_thumb_url, None);
        assert_eq!(vm.busy, BusyItems::default());
        assert!(vm.status.is_empty());
        assert!(!vm.creating_playlist);
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
    fn library_view_model_broadcast_readiness_report_replaces_content_list() {
        let mut vm = LibraryViewModel::new();
        vm.replace_tree(library_tree());
        let report = BroadcastReadinessReport {
            summary: crate::application::queries::broadcast::BroadcastReadinessSummary {
                ready: 1,
                no_route_tag: 1,
                no_routes_upstream: 0,
                file_missing: 1,
                not_downloaded: 0,
                ..Default::default()
            },
            tracks: vec![
                broadcast_readiness_track(
                    7,
                    "Missing Routes",
                    crate::application::queries::broadcast::BroadcastReadinessState::NoRouteTag,
                    "Embedded MusicIndex Value Routes tag is missing.",
                ),
                broadcast_readiness_track(
                    8,
                    "Missing File",
                    crate::application::queries::broadcast::BroadcastReadinessState::FileMissing,
                    "Recorded local file is missing.",
                ),
            ],
        };

        vm.show_needs_attention_report(&report);

        assert_eq!(
            vm.content_list_page.source(),
            ContentListPageSource::BroadcastReadiness
        );
        assert_eq!(vm.content_view_mode(), ContentViewMode::List);
        let rows = vm.content_list_page.visible_rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].title(), "Missing Routes");
        assert_eq!(rows[0].state_label, Some("Missing routes"));
        assert!(rows[0].fix.is_some());
        assert!(!rows[0].accepts_row_click());
        assert_eq!(rows[1].title(), "Missing File");
        assert_eq!(rows[1].state_label, Some("Missing file"));
        assert!(rows[1].fix.is_none());
        assert!(!rows[1].accepts_row_click());
    }

    #[test]
    fn broadcast_readiness_no_route_tag_row_carries_available_action() {
        let row = ContentListRowDisplay::from_broadcast_readiness_track(
            &broadcast_readiness_track(
                7,
                "Missing Routes",
                crate::application::queries::broadcast::BroadcastReadinessState::NoRouteTag,
                "Embedded MusicIndex Value Routes tag is missing.",
            ),
            false,
        );

        let action = row
            .fix
            .as_ref()
            .expect("NoRouteTag row carries a repair action");
        assert_eq!(
            action.kind,
            ContentListRowActionKind::RepairBroadcastRoutes { track_id: 7 }
        );
        assert_eq!(action.id, "broadcast-readiness-fix-routes-7");
        assert_eq!(action.label, "Fix routes");
        assert_eq!(action.a11y_label, "Fix payment routes for Missing Routes");
        assert!(!action.disabled());
        assert!(!row.accepts_row_click());
    }

    /// R3-10: the row of a removed track exposes a typed confirm action
    /// with an accessibility label. ADR 0083 task 007, decision D10: the
    /// confirm action is the one fix button, and "Remove from Library…"
    /// is an item of the row menu. The row of a `RouteOutOfDate` track
    /// exposes no repair action, and it names the "Update n file(s)"
    /// button.
    #[test]
    fn adr_0076_route_readiness_rows_expose_confirm_and_no_repair() {
        use crate::application::queries::broadcast::BroadcastReadinessState;

        let removed = ContentListRowDisplay::from_broadcast_readiness_track(
            &broadcast_readiness_track(
                9,
                "Gone Song",
                BroadcastReadinessState::RemovedFromFeed,
                "Removed from feed on 2026-09-24 10:00 UTC. Confirm to play it, or remove it from the playlist.",
            ),
            false,
        );
        assert_eq!(removed.state_label, Some("Removed from feed"));
        let confirm = removed.fix.as_ref().expect("confirm action");
        assert_eq!(
            confirm.kind,
            ContentListRowActionKind::ConfirmRemovedFromFeed { track_id: 9 }
        );
        assert_eq!(confirm.id, "broadcast-readiness-confirm-removed-9");
        assert_eq!(confirm.label, "Confirm");
        assert_eq!(
            confirm.a11y_label,
            "Confirm that the show plays Gone Song, which was removed from its feed"
        );
        assert!(!confirm.disabled());
        assert!(removed
            .secondary_text()
            .contains("Removed from feed on 2026-09-24"));
        assert!(!removed.accepts_row_click());

        let old = ContentListRowDisplay::from_broadcast_readiness_track(
            &broadcast_readiness_track(
                10,
                "Old Route",
                BroadcastReadinessState::RouteOutOfDate,
                "Payment route in file is out of date.",
            ),
            false,
        );
        assert_eq!(old.state_label, Some("Route out of date"));
        assert!(old.fix.is_none());
        assert!(old
            .secondary_text()
            .contains("Payment route in file is out of date."));
        assert!(old.secondary_text().contains("\"Update n file(s)\""));
    }

    #[test]
    fn broadcast_readiness_no_routes_upstream_row_names_the_feed_owner_without_action() {
        let row = ContentListRowDisplay::from_broadcast_readiness_track(
            &broadcast_readiness_track(
                8,
                "Feed Owner Work",
                crate::application::queries::broadcast::BroadcastReadinessState::NoRoutesUpstream,
                "MusicIndex has no payment routes.",
            ),
            false,
        );

        assert_eq!(row.state_label, Some("No upstream routes"));
        assert!(row.fix.is_none());
        assert!(
            row.secondary_text()
                .contains("The feed owner must add payment routes."),
            "NoRoutesUpstream rows must name the feed owner action"
        );
        assert!(!row.accepts_row_click());
    }

    #[test]
    fn broadcast_readiness_row_reports_repair_running() {
        let mut vm = LibraryViewModel::new();
        let report = BroadcastReadinessReport {
            summary: crate::application::queries::broadcast::BroadcastReadinessSummary {
                ready: 0,
                no_route_tag: 1,
                no_routes_upstream: 0,
                file_missing: 0,
                not_downloaded: 0,
                ..Default::default()
            },
            tracks: vec![broadcast_readiness_track(
                7,
                "Missing Routes",
                crate::application::queries::broadcast::BroadcastReadinessState::NoRouteTag,
                "Embedded MusicIndex Value Routes tag is missing.",
            )],
        };
        vm.show_needs_attention_report(&report);

        assert!(vm.begin_broadcast_route_repair(7));

        let rows = vm.content_list_page.visible_rows();
        assert_eq!(rows[0].state_label, Some("Repairing routes"));
        let action = rows[0].fix.as_ref().expect("repair action remains visible");
        // ADR 0083 task 007, decision D11: the working label.
        assert_eq!(action.label, "Repairing\u{2026}");
        assert!(action.disabled());
        assert_eq!(vm.status(), "Repairing payment routes for track 7...");
        assert!(
            !vm.begin_broadcast_route_repair(7),
            "running repair cannot be started again"
        );
    }

    #[test]
    fn feed_check_route_repair_status_reports_counts_separately() {
        let mut vm = LibraryViewModel::new();

        vm.finish_all_feed_check_with_route_repair(FeedCheckRouteRepairOutcome::new(
            3, 1, 0, 2, 1, 0,
        ));

        assert_eq!(vm.feed_update_state().phase, FeedUpdatePhase::Idle);
        assert_eq!(
            vm.feed_update_state().status_message.as_deref(),
            Some(
                "Checked 3 feeds; 1 feed changed; repaired 2 route tags; 1 track needs payment routes from the feed owner; 0 failed"
            )
        );

        vm.finish_all_feed_check_with_route_repair(FeedCheckRouteRepairOutcome::new(
            3, 0, 0, 0, 0, 0,
        ));

        assert_eq!(
            vm.feed_update_state().status_message.as_deref(),
            Some("Checked 3 feeds; all feeds up to date; no route repairs needed")
        );
    }

    #[test]
    fn library_view_model_content_view_mode_uses_content_list_page_vm() {
        let mut vm = LibraryViewModel::new();

        assert_eq!(
            vm.content_view_mode(),
            ContentViewMode::Tiles,
            "Situational ADR 0062 content-list view mode guard: LibraryViewModel should expose the page VM default"
        );

        vm.set_content_view_mode(ContentViewMode::List);

        assert_eq!(
            vm.content_view_mode(),
            ContentViewMode::List,
            "Situational ADR 0062 content-list view mode guard: LibraryViewModel should update the page VM mode"
        );
        assert_eq!(
            vm.content_view_mode_control().selected,
            ContentViewMode::List,
            "Situational ADR 0062 content-list view mode guard: LibraryViewModel should project the selected mode into chrome display"
        );
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

    /// D5-01: the view model gives the six destinations in Decision 2
    /// order, each with a label, a named icon, a tooltip and an
    /// accessibility label.
    #[test]
    fn adr_0084_destinations_d5_01_destination_rows_are_in_sidebar_order_with_full_display_data() {
        let vm = LibraryViewModel::new();
        let rows = vm.destination_rows();

        assert_eq!(
            rows.iter().map(|row| row.destination).collect::<Vec<_>>(),
            vec![
                MusicDestination::Discover,
                MusicDestination::Releases,
                MusicDestination::Artists,
                MusicDestination::Tracks,
                MusicDestination::NeedsAttention,
                MusicDestination::Playlists,
            ]
        );
        for row in &rows {
            assert_eq!(row.label, row.destination.label());
            assert!(!row.element_id.is_empty());
            assert!(!row.tooltip.is_empty());
            assert!(!row.a11y_label.is_empty());
        }
        assert_eq!(
            rows[0].icon,
            MusicDestinationIcon::Discover,
            "Discover must carry its named icon"
        );
        assert_eq!(rows[4].icon, MusicDestinationIcon::Warning);
    }

    /// D5-06: with a Needs attention count, the collapsed destination
    /// exposes a mark, and its tooltip and its accessibility label give
    /// the count. Without a count, no mark shows.
    #[test]
    fn adr_0084_destinations_d5_06_needs_attention_mark_and_count_text() {
        let mut vm = LibraryViewModel::new();
        let before = vm.destination_rows();
        let needs_attention = &before[4];
        assert_eq!(
            needs_attention.destination,
            MusicDestination::NeedsAttention
        );
        assert!(!needs_attention.shows_attention_mark);
        // A count of zero shows nothing (orchestrator review, 2026-10-16).
        assert_eq!(needs_attention.count, None);

        let report = BroadcastReadinessReport {
            summary: crate::application::queries::broadcast::BroadcastReadinessSummary {
                no_route_tag: 2,
                ..Default::default()
            },
            tracks: Vec::new(),
        };
        vm.refresh_needs_attention_report(&report);

        let after = vm.destination_rows();
        let needs_attention = &after[4];
        assert!(needs_attention.shows_attention_mark);
        assert_eq!(needs_attention.count, Some(2));
        assert!(needs_attention.tooltip.contains('2'));
        assert!(needs_attention.a11y_label.contains('2'));
    }

    /// D5-04: the Playlists destination list gives the playlist count as
    /// its row count, used by the sidebar destination row too.
    #[test]
    fn adr_0084_destinations_d5_04_playlists_destination_count_tracks_loaded_playlists() {
        let mut vm = LibraryViewModel::new();
        let mut alpha = playlist("Alpha");
        alpha.id = 10;
        vm.replace_playlists(vec![alpha]);

        let rows = vm.destination_rows();
        assert_eq!(rows[5].destination, MusicDestination::Playlists);
        assert_eq!(rows[5].count, Some(1));
        assert_eq!(
            rows[5].tooltip, "Playlists, 1 playlist",
            "the Playlists count names playlists, not tracks"
        );
        assert_eq!(
            rows[4].count, None,
            "a Needs attention count of zero shows nothing"
        );
        assert_eq!(rows[4].tooltip, "Needs attention");
    }

    /// D5-07: the collapse button has a typed action, an accessibility
    /// label and a tooltip, each from the view model, and the next
    /// preference is always the opposite of the resolved state.
    #[test]
    fn adr_0084_destinations_d5_07_collapse_button_is_typed_from_the_view_model() {
        let expanded_button =
            LibraryViewModel::sidebar_collapse_button(MusicSidebarState::Expanded);
        assert_eq!(
            expanded_button.next_preference,
            MusicSidebarPreference::Collapsed
        );
        assert!(!expanded_button.a11y_label.is_empty());
        assert!(!expanded_button.tooltip.is_empty());

        let collapsed_button =
            LibraryViewModel::sidebar_collapse_button(MusicSidebarState::Collapsed);
        assert_eq!(
            collapsed_button.next_preference,
            MusicSidebarPreference::Expanded
        );
        assert_ne!(collapsed_button.a11y_label, expanded_button.a11y_label);
    }

    /// The release rows of a Releases list with no singles group.
    fn release_rows(entries: Vec<ReleasesListEntryVm>) -> Vec<DestinationListRowVm> {
        entries
            .into_iter()
            .map(|entry| match entry {
                ReleasesListEntryVm::Release(row) => row,
                ReleasesListEntryVm::SinglesGroup(group) => {
                    panic!("the fixture has no singles group: {group:?}")
                }
            })
            .collect()
    }

    /// A tree with three singles of "Haleen", two in the Library, and one
    /// album of "Aphex Twin" (ADR 0084 Decision 6).
    fn singles_tree() -> LibraryTree {
        let single = |name: &str, feed_id: i64, pub_date: i64, in_library: bool| {
            let mut track = row();
            track.id = feed_id * 10;
            track.feed_id = feed_id;
            track.pub_date = Some(pub_date);
            track.is_in_library = in_library;
            AlbumNode {
                name: name.into(),
                feed_id: Some(feed_id),
                feed_guid: Some(format!("guid-{feed_id}")),
                feed_url: None,
                language: None,
                description: None,
                image_href: Some(format!("{feed_id}.jpg")),
                named_feeds: Vec::new(),
                identity_facts: LocalIdentityFacts::default(),
                metadata_facts: Box::<FeedMetadataFacts>::default(),
                stored_values: None,
                tracks: vec![track],
            }
        };
        let mut album = library_tree().artists.remove(0);
        album.albums.truncate(1);
        LibraryTree {
            artists: vec![
                album,
                ArtistNode {
                    name: "Haleen".into(),
                    albums: vec![
                        single("Middle", 1, 200, true),
                        single("Newest", 2, 300, false),
                        single("Oldest", 3, 100, true),
                    ],
                },
            ],
        }
    }

    /// R15-05 (ADR 0084 Decision 6): the singles of a group give one group
    /// entry and no release entry. The group entry sorts by the display
    /// artist and names the count and the Library state.
    #[test]
    fn adr_0084_singles_group_r15_05_releases_list_gives_one_group_entry() {
        let mut vm = LibraryViewModel::new();
        vm.replace_tree(singles_tree());

        let entries = vm.releases_list(&BTreeSet::new());

        assert_eq!(
            entries
                .iter()
                .map(ReleasesListEntryVm::label)
                .collect::<Vec<_>>(),
            vec!["Haleen", "Selected Ambient Works"],
            "the group entry replaces its singles and sorts by its label"
        );
        let ReleasesListEntryVm::SinglesGroup(group) = &entries[0] else {
            panic!("the first entry must be the Haleen group");
        };
        assert_eq!(group.label, "Haleen");
        assert_eq!(group.secondary_text, "3 singles · 2 in Library · By name");
        assert_eq!(group.a11y_label, "Haleen, 3 singles, 2 in Library, By name");
        // R11-05 (ADR 0084 Decision 12): the group row shows the newest
        // images of its singles in the logo mosaic.
        assert_eq!(
            group.art,
            CollectionArtVm::Mosaic(["2.jpg", "1.jpg", "3.jpg", "2.jpg"].map(str::to_owned))
        );
        assert_eq!(
            group.open,
            SinglesGroupKey(ArtistIdentity::Name("Haleen".into()))
        );

        let mut tree = singles_tree();
        for album in &mut tree.artists[1].albums {
            for track in &mut album.tracks {
                track.is_in_library = true;
            }
        }
        vm.replace_tree(tree);
        let entries = vm.releases_list(&BTreeSet::new());
        let ReleasesListEntryVm::SinglesGroup(group) = &entries[0] else {
            panic!("the first entry must be the Haleen group");
        };
        assert_eq!(group.secondary_text, "3 singles · In Library · By name");
    }

    /// R15-06 (ADR 0084 Decision 6): with the group key in the expanded
    /// set, the group entry gives its singles rows newest first and "Hide
    /// singles". Without the key, it gives no singles rows and "Show
    /// singles".
    #[test]
    fn adr_0084_singles_group_r15_06_expanded_group_gives_its_singles() {
        let mut vm = LibraryViewModel::new();
        vm.replace_tree(singles_tree());
        let key = SinglesGroupKey(ArtistIdentity::Name("Haleen".into()));

        let expanded = BTreeSet::from([key.clone()]);
        let entries = vm.releases_list(&expanded);
        let ReleasesListEntryVm::SinglesGroup(group) = &entries[0] else {
            panic!("the first entry must be the Haleen group");
        };
        assert!(group.expanded);
        assert_eq!(group.disclosure_a11y_label, "Hide singles");
        assert_eq!(
            group
                .singles
                .iter()
                .map(|row| row.label.as_str())
                .collect::<Vec<_>>(),
            vec!["Newest", "Middle", "Oldest"]
        );
        assert_eq!(
            group.singles[0].open,
            DestinationListRowTarget::Release(ReleaseKey::FeedGuid("guid-2".into()))
        );

        let entries = vm.releases_list(&BTreeSet::new());
        let ReleasesListEntryVm::SinglesGroup(group) = &entries[0] else {
            panic!("the first entry must be the Haleen group");
        };
        assert!(!group.expanded);
        assert_eq!(group.disclosure_a11y_label, "Show singles");
        assert!(group.singles.is_empty());
    }

    /// R9-13, R9-14 (Required correction, 2026-10-18): with the Haleen
    /// group open, `releases_list_with_selection` gives each single its
    /// own checkbox. Toggling one single checks only that single's row,
    /// and the group's own checkbox reads mixed.
    /// Operator check of 2026-10-19: a singles group opened only with a
    /// click on its chevron. Right opens a closed group, Left closes an
    /// open group, and Left on a single moves the focus to its group row.
    #[test]
    fn adr_0084_selection_right_and_left_open_and_close_a_group() {
        let mut vm = LibraryViewModel::new();
        vm.replace_tree(singles_tree());
        let group_key = SinglesGroupKey(ArtistIdentity::Name("Haleen".into()));
        let group_row = ReleasesRowId::Group(group_key.clone());
        let single_row = ReleasesRowId::Release(ReleaseKey::FeedGuid("guid-2".into()));

        let closed = vm.releases_list(&BTreeSet::new());
        assert_eq!(
            releases_disclosure_step(Some(&group_row), true, &closed),
            ReleasesDisclosureStep::Open(group_key.clone())
        );
        assert_eq!(
            releases_disclosure_step(Some(&group_row), false, &closed),
            ReleasesDisclosureStep::None
        );

        let open = vm.releases_list(&BTreeSet::from([group_key.clone()]));
        assert_eq!(
            releases_disclosure_step(Some(&group_row), false, &open),
            ReleasesDisclosureStep::Close(group_key.clone())
        );
        assert_eq!(
            releases_disclosure_step(Some(&single_row), false, &open),
            ReleasesDisclosureStep::FocusGroup(group_key)
        );
        assert_eq!(
            releases_disclosure_step(Some(&single_row), true, &open),
            ReleasesDisclosureStep::None
        );
        assert_eq!(
            releases_disclosure_step(None, true, &open),
            ReleasesDisclosureStep::None
        );
    }

    #[test]
    fn adr_0084_selection_r9_13_r9_14_single_row_checkbox_and_group_mixed_state() {
        let mut vm = LibraryViewModel::new();
        vm.replace_tree(singles_tree());
        let group_key = SinglesGroupKey(ArtistIdentity::Name("Haleen".into()));
        let expanded = BTreeSet::from([group_key]);
        let newest_key = ReleaseKey::FeedGuid("guid-2".into());

        let mut selection: ListSelection<ReleaseKey, ReleasesRowId> = ListSelection::new();
        selection.toggle(&SelectionRow::new(
            ReleasesRowId::Release(newest_key.clone()),
            vec![newest_key.clone()],
        ));

        let entries =
            vm.releases_list_with_selection(&expanded, &selection, SelectionMode::Selecting);
        let ReleasesListEntryVm::SinglesGroup(group) = &entries[0] else {
            panic!("the first entry must be the Haleen group");
        };
        assert_eq!(
            group.selection.check_state,
            RowCheckState::Mixed,
            "one selected single out of three gives the group a mixed checkbox"
        );

        let newest_row = group
            .singles
            .iter()
            .find(|row| row.open == DestinationListRowTarget::Release(newest_key.clone()))
            .expect("the Newest single has its own row");
        assert!(newest_row.selection.check_state.is_checked());
        assert!(!newest_row.selection.checkbox_a11y_label.is_empty());

        let other_row = group
            .singles
            .iter()
            .find(|row| row.open != DestinationListRowTarget::Release(newest_key.clone()))
            .expect("a second single's row exists");
        assert_eq!(other_row.selection.check_state, RowCheckState::Clear);
        assert!(
            !other_row.selection.checkbox_a11y_label.is_empty(),
            "every single row of an open group shows its own checkbox"
        );
    }

    /// R19-05: the header action reads "Select" in `Normal` and "Done"
    /// in `Selecting`, with an accessibility label. It is absent for an
    /// empty list and for Artists.
    #[test]
    fn adr_0084_selection_mode_r19_05_header_action_reads_select_and_done() {
        let normal =
            list_column_header_action(MusicDestination::Releases, SelectionMode::Normal, true)
                .expect("Releases with rows gives a header action");
        assert_eq!(normal.label, "Select");
        assert_eq!(normal.a11y_label, "Select releases");

        let selecting =
            list_column_header_action(MusicDestination::Releases, SelectionMode::Selecting, true)
                .expect("Releases with rows gives a header action");
        assert_eq!(selecting.label, "Done");
        assert_eq!(selecting.a11y_label, "Done selecting releases");

        assert!(list_column_header_action(
            MusicDestination::Releases,
            SelectionMode::Normal,
            false
        )
        .is_none());

        assert!(
            list_column_header_action(MusicDestination::Artists, SelectionMode::Normal, true)
                .is_none()
        );

        let tracks =
            list_column_header_action(MusicDestination::Tracks, SelectionMode::Normal, true)
                .expect("Tracks with rows gives a header action");
        assert_eq!(tracks.a11y_label, "Select tracks");

        let needs_attention = list_column_header_action(
            MusicDestination::NeedsAttention,
            SelectionMode::Normal,
            true,
        )
        .expect("Needs attention with rows gives a header action");
        assert_eq!(needs_attention.a11y_label, "Select tracks");
    }

    /// D5-04: the Releases destination list gives one row for each Library
    /// release, by name, keyed so a row opens its release page.
    #[test]
    fn adr_0084_destinations_d5_04_releases_list_is_sorted_by_name_with_release_targets() {
        let mut vm = LibraryViewModel::new();
        vm.replace_tree(library_tree());

        let rows = release_rows(vm.releases_list(&BTreeSet::new()));

        assert_eq!(
            rows.iter()
                .map(|row| row.label.as_str())
                .collect::<Vec<_>>(),
            vec!["Selected Ambient Works", "Tri Repetae", "Windowlicker"],
            "releases must sort by name"
        );
        // ADR 0084 Decision 5: "Selected Ambient Works" has two tracks and
        // no stated kind, which resolves to Album.
        assert_eq!(rows[0].secondary_text, "Aphex Twin · Album");
        assert_eq!(rows[0].thumbnail_href, Some("saw.jpg".to_string()));
        assert_eq!(
            rows[0].open,
            DestinationListRowTarget::Release(ReleaseKey::from_index_feed_guid("saw-guid")),
            "a release with a feed GUID must key by it, like the Index route"
        );
        assert_eq!(
            rows[2].open,
            DestinationListRowTarget::Release(ReleaseKey::LibraryFeed(20)),
            "a release with no feed GUID must key by its Library feed id"
        );
    }

    /// R7-04 (ADR 0084 Decision 5): the Releases list names the kind after
    /// the artist. "Windowlicker" has one track and no stated kind, which
    /// resolves to Single.
    #[test]
    fn adr_0084_release_kind_r7_04_releases_list_names_the_kind_after_the_artist() {
        let mut vm = LibraryViewModel::new();
        vm.replace_tree(library_tree());

        let rows = release_rows(vm.releases_list(&BTreeSet::new()));
        let windowlicker = rows
            .iter()
            .find(|row| row.label == "Windowlicker")
            .expect("the fixture names a Windowlicker release");

        assert_eq!(windowlicker.secondary_text, "Aphex Twin · Single");
    }

    /// D5-04: the Artists destination list gives one row for each Library
    /// artist, by name, keyed so a row opens its artist page.
    #[test]
    fn adr_0084_destinations_d5_04_artists_list_is_sorted_by_name_with_artist_targets() {
        let mut vm = LibraryViewModel::new();
        vm.replace_tree(library_tree());

        let rows = vm.artists_list();

        assert_eq!(
            rows.iter()
                .map(|row| row.label.as_str())
                .collect::<Vec<_>>(),
            vec!["Aphex Twin", "Autechre"],
            "artists must sort by name"
        );
        assert_eq!(
            rows[0].open,
            DestinationListRowTarget::Artist("Aphex Twin".to_string())
        );
        assert_eq!(rows[0].secondary_text, "2 releases \u{b7} by name");
        assert_eq!(rows[1].secondary_text, "1 release \u{b7} by name");
        assert_eq!(rows[0].a11y_label, "Artist: Aphex Twin, by name");
    }

    /// R11-06 (ADR 0084 Decision 12): each Artists row gives collection
    /// art, never the monogram. The newest album image comes first.
    #[test]
    fn adr_0084_logo_frame_r11_06_artists_rows_give_collection_art() {
        let mut tree = library_tree();
        let aphex = &mut tree.artists[0];
        aphex.albums[0].tracks[0].pub_date = Some(100);
        aphex.albums[1].image_href = Some("wl.jpg".into());
        aphex.albums[1].tracks[0].pub_date = Some(200);
        let mut vm = LibraryViewModel::new();
        vm.replace_tree(tree);

        let rows = vm.artists_list();

        assert_eq!(
            rows[0].art,
            Some(CollectionArtVm::Mosaic(
                ["wl.jpg", "saw.jpg", "saw.jpg", "wl.jpg"].map(str::to_owned)
            ))
        );
        assert_eq!(
            rows[1].art,
            Some(CollectionArtVm::EntityColor(CollectionKind::Artist))
        );
        assert!(rows.iter().all(|row| row.thumbnail_href.is_none()));

        let mut one_cover = LibraryViewModel::new();
        one_cover.replace_tree(library_tree());
        assert_eq!(
            one_cover.artists_list()[0].art,
            Some(CollectionArtVm::SingleCover("saw.jpg".into()))
        );
    }

    /// Gives `album` the image `image_url` and the named feed of
    /// `with_named_feed` with the channel image `feed_image`.
    fn with_feed_image(
        album: AlbumNode,
        image_url: &str,
        feed_guid: &str,
        role: Option<&str>,
        feed_image: Option<&str>,
    ) -> AlbumNode {
        let mut album = with_named_feed(album, feed_guid, feed_guid, role, Some("both"));
        album.image_href = Some(image_url.to_owned());
        album.named_feeds[0].image_url = feed_image.map(str::to_owned);
        album
    }

    /// R24-04 (ADR 0084 Decision 12, packet 024): a feed row of the Artists
    /// list gives `ProvidedImage` when its feed has a channel image, and the
    /// mosaic when it has none. A name row never gives `ProvidedImage`, also
    /// when one of its albums names a feed with an image.
    #[test]
    fn adr_0084_artist_image_r24_04_feed_row_gives_the_feed_image() {
        let with_image = |name: &str, id: i64, cover: &str, image: Option<&str>| {
            with_feed_image(
                minimal_album(name, id),
                cover,
                "artist-guid",
                Some("artist"),
                image,
            )
        };
        let label_album = with_feed_image(
            minimal_album("Label Release", 3),
            "c.jpg",
            "label-guid",
            Some("label"),
            Some("https://img.test/label.jpg"),
        );
        let tree = |image: Option<&'static str>| LibraryTree {
            artists: vec![
                ArtistNode {
                    name: "Echo Chamber".to_owned(),
                    albums: vec![
                        with_image("One", 1, "a.jpg", image),
                        with_image("Two", 2, "b.jpg", image),
                    ],
                },
                ArtistNode {
                    name: "Label Act".to_owned(),
                    albums: vec![label_album.clone()],
                },
            ],
        };
        let row_art = |vm: &LibraryViewModel, label: &str| {
            vm.artists_list()
                .into_iter()
                .find(|row| row.label == label)
                .and_then(|row| row.art)
                .expect("the row gives art")
        };

        let mut vm = LibraryViewModel::new();
        vm.replace_tree(tree(Some("  https://img.test/artist.jpg  ")));
        let mosaic =
            CollectionArtVm::Mosaic(["a.jpg", "b.jpg", "b.jpg", "a.jpg"].map(str::to_owned));
        assert_eq!(
            row_art(&vm, "artist-guid"),
            CollectionArtVm::ProvidedImage {
                url: "https://img.test/artist.jpg".to_owned(),
                fallback: Box::new(mosaic.clone()),
            }
        );
        assert_eq!(
            row_art(&vm, "Label Act"),
            CollectionArtVm::SingleCover("c.jpg".to_owned()),
            "a name row gives no provided image"
        );

        let mut no_image = LibraryViewModel::new();
        no_image.replace_tree(tree(None));
        assert_eq!(row_art(&no_image, "artist-guid"), mosaic);
    }

    /// R11-05 (ADR 0084 Decision 12): each Playlists row gives the art of
    /// the one playlist image read, and the playlist page gives the first
    /// four images in playlist order.
    #[test]
    fn adr_0084_logo_frame_r11_05_playlist_rows_and_page_give_collection_art() {
        let mut vm = LibraryViewModel::new();
        let mut alpha = playlist("Alpha");
        alpha.id = 10;
        let mut zed = playlist("zed");
        zed.id = 20;
        vm.replace_playlists(vec![zed, alpha.clone()]);
        vm.replace_playlist_covers(BTreeMap::from([(10, vec!["a".into(), "b".into()])]));

        let sidebar = vm.playlist_sidebar();

        assert_eq!(
            sidebar.rows[0].art,
            CollectionArtVm::Mosaic(["a", "b", "b", "a"].map(str::to_owned))
        );
        assert_eq!(
            sidebar.rows[1].art,
            CollectionArtVm::EntityColor(CollectionKind::Playlist)
        );

        let mut own_image = row();
        own_image.track_image_href = Some("track.jpg".into());
        own_image.album_image_href = Some("album-a.jpg".into());
        let mut album_only = row();
        album_only.album_image_href = Some("album-b.jpg".into());
        let tracks = [own_image, album_only, row()];
        assert_eq!(
            PlaylistDetailVm::new(&alpha, &tracks).collection_art(),
            CollectionArtVm::Mosaic(
                ["track.jpg", "album-b.jpg", "album-b.jpg", "track.jpg"].map(str::to_owned)
            )
        );
        assert_eq!(
            PlaylistDetailVm::new(&alpha, &[]).collection_art(),
            CollectionArtVm::EntityColor(CollectionKind::Playlist)
        );
    }

    /// One minimal `AlbumNode`, with no named feed. Tests that add a named
    /// feed call `with_named_feed` on the result.
    fn minimal_album(name: &str, feed_id: i64) -> AlbumNode {
        AlbumNode {
            name: name.to_owned(),
            feed_id: Some(feed_id),
            feed_guid: Some(format!("album-guid-{feed_id}")),
            feed_url: None,
            language: None,
            description: None,
            image_href: None,
            named_feeds: Vec::new(),
            identity_facts: LocalIdentityFacts::default(),
            metadata_facts: Box::new(FeedMetadataFacts::default()),
            stored_values: None,
            tracks: Vec::new(),
        }
    }

    /// Gives `album` one named feed with the given GUID, role and
    /// agreement, confirmed (`two_way_validated`), at position 0 (the
    /// primary named feed).
    fn with_named_feed(
        mut album: AlbumNode,
        feed_guid: &str,
        title: &str,
        role: Option<&str>,
        role_agreement: Option<&str>,
    ) -> AlbumNode {
        album.named_feeds = vec![NamedFeed {
            feed_guid: feed_guid.to_owned(),
            title: Some(title.to_owned()),
            position: Some(0),
            role: role.map(str::to_owned),
            role_agreement: role_agreement.map(str::to_owned),
            feed_rel: None,
            album_rel: None,
            confirmed: true,
            image_url: None,
        }];
        album
    }

    /// R85-8-10: `artists_list` gives a feed row for an `Artist` feed and
    /// for an `UnverifiedArtist` feed, and a name row for a name identity.
    /// An agreed `label` feed has no row. No row text contains a level
    /// word.
    #[test]
    fn adr_0085_feed_page_artists_list_feed_and_name_rows_r85_8_10() {
        let artist_feed_album = with_named_feed(
            minimal_album("Artist Release", 1),
            "artist-guid",
            "Echo Chamber",
            Some("artist"),
            Some("both"),
        );
        let unverified_album = with_named_feed(
            minimal_album("Unverified Release", 2),
            "unverified-guid",
            "Night Owl",
            None,
            None,
        );
        let agreed_label_album = with_named_feed(
            minimal_album("Label Release", 3),
            "label-guid",
            "Sir Libre Records",
            Some("label"),
            Some("both"),
        );
        let name_only_album = minimal_album("Name Release", 4);

        let mut vm = LibraryViewModel::new();
        vm.replace_tree(LibraryTree {
            artists: vec![
                ArtistNode {
                    name: "Echo Chamber Act".to_owned(),
                    albums: vec![artist_feed_album],
                },
                ArtistNode {
                    name: "Night Owl Act".to_owned(),
                    albums: vec![unverified_album],
                },
                ArtistNode {
                    name: "Label Act".to_owned(),
                    albums: vec![agreed_label_album],
                },
                ArtistNode {
                    name: "Name Act".to_owned(),
                    albums: vec![name_only_album],
                },
            ],
        });

        let rows = vm.artists_list();
        let labels: Vec<&str> = rows.iter().map(|row| row.label.as_str()).collect();

        assert!(labels.contains(&"Echo Chamber"), "{labels:?}");
        assert!(labels.contains(&"Night Owl"), "{labels:?}");
        assert!(labels.contains(&"Name Act"), "{labels:?}");
        assert!(
            !labels.contains(&"Sir Libre Records"),
            "an agreed label feed has no row: {labels:?}"
        );
        assert!(
            labels.contains(&"Label Act"),
            "the agreed-label album falls to its name row: {labels:?}"
        );

        let artist_row = rows
            .iter()
            .find(|row| row.label == "Echo Chamber")
            .expect("the confirmed artist feed has a row");
        // Operator decision of 2026-10-18: only a name row carries "by
        // name", so a feed row and a name row with one label differ.
        assert!(
            !artist_row.secondary_text.contains("by name"),
            "{artist_row:?}"
        );
        let name_row = rows
            .iter()
            .find(|row| row.label == "Name Act")
            .expect("the name identity has a row");
        assert!(
            name_row.secondary_text.ends_with("\u{b7} by name"),
            "{name_row:?}"
        );
        assert_eq!(
            artist_row.open,
            DestinationListRowTarget::Publisher("artist-guid".to_owned())
        );
        let unverified_row = rows
            .iter()
            .find(|row| row.label == "Night Owl")
            .expect("the unverified artist feed has a row");
        assert_eq!(
            unverified_row.open,
            DestinationListRowTarget::Publisher("unverified-guid".to_owned())
        );

        for row in &rows {
            assert!(
                !row.secondary_text.to_lowercase().contains("unverified")
                    && !row.secondary_text.to_lowercase().contains("confirm"),
                "no row text names a level: {row:?}"
            );
            assert!(
                !row.a11y_label.to_lowercase().contains("unverified")
                    && !row.a11y_label.to_lowercase().contains("confirm"),
                "no row text names a level: {row:?}"
            );
        }
    }

    /// R6-01 (ADR 0084 Decision 3): the Discover views view model gives
    /// "New releases" first, with its subtitle and its open entry.
    #[test]
    fn adr_0084_discover_r6_01_discover_view_rows_give_new_releases_first() {
        let vm = LibraryViewModel::new();

        let rows = vm.discover_view_rows();

        assert_eq!(rows.len(), 1, "Discover has one view today");
        assert_eq!(rows[0].view, DiscoverView::NewReleases);
        assert_eq!(rows[0].label, "New releases");
        assert_eq!(rows[0].subtitle, "Newest first, from the Index");
        assert!(
            rows[0].selected,
            "New releases is always the open view of the Discover location"
        );
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
    fn library_chrome_display_projects_shell_labels() {
        let display = LibraryViewModel::chrome_display();
        assert_eq!(display.search_button_id, "lib-search-btn");
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
        assert_eq!(display.empty_detail_label, "Select an item in the list.");
    }

    #[test]
    fn library_status_snapshot_classifies_error_prefix() {
        let mut vm = LibraryViewModel::new();
        vm.set_error_status("offline");

        let status = vm.status_snapshot();

        assert_eq!(status.text, "Error: offline");
        assert!(status.is_error);

        let mut vm = LibraryViewModel::new();
        vm.finish_library_reload(0);
        let status = vm.status_snapshot();
        assert_eq!(status.text, "0 library tracks");
        assert!(!status.is_error);
    }

    /// Builds a sample write failure for one generation. The test
    /// `adr_0075_observation_failures_survive_navigation_and_preserve_operation_state`
    /// uses it to keep its own body short.
    fn sample_observation_write_failure(
        generation: i64,
    ) -> crate::provider_observation::ObservationWriteFailure {
        use crate::provider_observation::{
            ObservationOutcome, ObservationStorageError, ObservationWriteFailure, ProviderKind,
            ProviderObservation, ProviderRequestSpec, RequestToken,
        };
        use serde_json::json;
        use std::sync::Arc;

        ObservationWriteFailure {
            token: RequestToken {
                key: "private request".into(),
                write_state: Arc::new(std::sync::Mutex::new(
                    crate::provider_observation::RequestWriteState::Ready,
                )),
                generation,
                provider_id: 1,
                resource_id: 1,
                subject_id: None,
                spec: Arc::new(ProviderRequestSpec {
                    provider: ProviderKind::MusicIndex,
                    provider_identity: "private endpoint".into(),
                    request_uri: "private URI".into(),
                    requested_subject: None,
                    requested_parameters: json!({}),
                    profile: json!({}),
                    started_at_us: 10,
                }),
            },
            observation: Arc::new(ProviderObservation {
                body: Some(Arc::from(b"private response".as_slice())),
                http_status: Some(200),
                response_uri: Some("private URI".into()),
                interpretation: json!({}),
                source_revision: None,
                source_times: json!({}),
                decoder_version: "test".into(),
                outcome: ObservationOutcome::Success,
                failure: None,
                finished_at_us: 20,
                fetched_at_us: Some(19),
                occurrence: json!({}),
                coverage: Vec::new(),
            }),
            operation: ObservationStorageError::ResponseWrite,
            retry: crate::provider_observation::StorageRetry::VerifiedRollback,
        }
    }

    /// Builds a sample observation receipt for one generation. The test
    /// `adr_0075_observation_failures_survive_navigation_and_preserve_operation_state`
    /// uses it to keep its own body short.
    fn sample_observation_receipt(
        generation: i64,
    ) -> crate::provider_observation::ObservationReceipt {
        use serde_json::json;

        crate::provider_observation::ObservationReceipt {
            observation_id: generation,
            generation: 100 + generation,
            provider_id: 1,
            resource_id: 1,
            request_uri: "private URI".into(),
            response_uri: None,
            body_key: None,
            started_at_us: 10,
            finished_at_us: 20,
            fetched_at_us: Some(19),
            occurrence: json!({}),
            outcome: crate::provider_observation::ObservationOutcome::Success,
            retention: crate::provider_observation::ObservationRetention::Unknown,
            collections: Vec::new(),
        }
    }

    #[test]
    fn adr_0075_observation_failures_survive_navigation_and_preserve_operation_state() {
        use crate::application::errors::command::CommandError;
        use crate::provider_observation::ObservationStorageError;
        use std::sync::Arc;
        let mut vm = LibraryViewModel::new();
        vm.selected_id = Some(42);
        vm.busy = sample_busy_items();
        vm.library_loading = true;
        let plan = LibraryRemovalPlan::new(
            LibraryRemovalTarget::Track(42),
            LibraryRemovalImpact::Track {
                playlist_reference_count: 1,
            },
        );
        assert!(!vm.confirm_library_removal(plan, RemovalConfirmation::WhenReferenced));
        let before = vm.pending_library_removal_confirmation();
        for generation in [1, 2] {
            let failure = sample_observation_write_failure(generation);
            vm.retain_observation_failure(&CommandError::ObservationWriteFailure(Arc::new(
                crate::provider_observation::ObservationCommandFailure {
                    read_error: None,
                    write_failure: Some(Arc::new(failure)),
                    storage_error: None,
                    receipts: Arc::from([sample_observation_receipt(generation)]),
                },
            )));
            assert_eq!(
                vm.observation_failures.len(),
                usize::try_from(generation).unwrap()
            );
            assert_eq!(vm.busy, sample_busy_items());
            assert!(vm.library_loading);
            assert_eq!(vm.pending_library_removal_confirmation(), before);
            assert!(vm.status_snapshot().is_error);
            assert!(!vm.status_snapshot().text.contains("private"));
            assert!(!vm.status_snapshot().text.contains("unavailable"));
            vm.selected_id = Some(43);
        }
        assert_eq!(vm.observation_receipts.len(), 2);
        assert_eq!(vm.selected_id, Some(43));
        assert_eq!(
            vm.observation_failures[&1].observation.fetched_at_us,
            Some(19)
        );
        vm.retain_observation_failure(&CommandError::Query(
            ObservationStorageError::RequestAllocation.to_string(),
        ));
        assert_eq!(vm.observation_failures.len(), 2);
        assert!(vm.library_loading);
        vm.finish_library_reload(2);
        assert_eq!(vm.observation_receipts.len(), 2);
        assert_eq!(vm.observation_failures.len(), 2);
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

        assert!(!vm.confirm_library_removal(plan, RemovalConfirmation::WhenReferenced));
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
        assert_eq!(display.state_label, None);
        assert_eq!(display.status_message, None);
        assert_eq!(display.action.kind, FeedUpdateActionKind::CheckAllFeeds);
        assert_eq!(display.action.button_id, "check-all-feeds");
        assert_eq!(display.action.label, "Check all feeds");
        assert!(!display.action.disabled);

        vm.begin_all_feed_check(3);
        let display = vm.feed_update_display();
        assert_eq!(display.state_label, None);
        assert_eq!(
            display.status_message.as_deref(),
            Some("Checking 3 feeds...")
        );
        assert_eq!(display.action.kind, FeedUpdateActionKind::CheckAllFeeds);
        assert_eq!(display.action.button_id, "check-all-feeds");
        assert_eq!(display.action.label, "Checking...");
        assert!(display.action.disabled);

        vm.finish_all_feed_check_with_route_repair(FeedCheckRouteRepairOutcome::new(
            3, 0, 0, 0, 0, 0,
        ));
        vm.begin_feed_view_check(1);
        vm.finish_feed_view_check(
            1,
            Ok(Some(feed_service::StaleFeed {
                feed_id: 1,
                feed_guid: "feed-1".into(),
                title: Some("Feed".into()),
                new_updated_at: 10,
            })),
        );
        let display = vm.feed_update_display();
        assert_eq!(display.state_label, Some("Update available"));
        assert_eq!(display.status_message.as_deref(), Some("1 feed changed"));
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
    fn adr_0075_library_observation_receipt_retention_preserves_confirmation_loading_and_busy_state(
    ) {
        use crate::application::errors::command::{CommandError, ObservedQueryFailure};
        use crate::provider_observation::{
            ObservationOutcome, ObservationReceipt, ObservationRetention,
        };
        let mut vm = LibraryViewModel::new();
        vm.selected_id = Some(42);
        vm.busy = sample_busy_items();
        vm.library_loading = true;
        let plan = LibraryRemovalPlan::new(
            LibraryRemovalTarget::Track(42),
            LibraryRemovalImpact::Track {
                playlist_reference_count: 1,
            },
        );
        assert!(!vm.confirm_library_removal(plan, RemovalConfirmation::WhenReferenced));
        let confirmation = vm.pending_library_removal_confirmation();
        let status = vm.status.clone();
        let receipt = ObservationReceipt {
            observation_id: 1,
            generation: 1,
            provider_id: 1,
            resource_id: 1,
            request_uri: "private URI".into(),
            response_uri: None,
            body_key: None,
            started_at_us: 10,
            finished_at_us: 20,
            fetched_at_us: Some(19),
            occurrence: serde_json::json!({}),
            outcome: ObservationOutcome::Success,
            retention: ObservationRetention::Unknown,
            collections: Vec::new(),
        };
        vm.retain_observation_receipts(std::slice::from_ref(&receipt));
        let mut later = receipt.clone();
        later.generation = 2;
        let failure = CommandError::ObservedQueryFailure(std::sync::Arc::new(
            ObservedQueryFailure::new("ordinary cause".into(), vec![later.clone()]),
        ));
        vm.retain_query_evidence(&failure);
        vm.retain_observation_receipts(std::slice::from_ref(&receipt));
        assert_eq!(vm.observation_receipts.len(), 2);
        assert_eq!(vm.observation_receipts[&1], receipt);
        assert_eq!(vm.observation_receipts[&2], later);
        assert_eq!(vm.selected_id, Some(42));
        assert_eq!(vm.busy, sample_busy_items());
        assert!(vm.library_loading);
        assert_eq!(vm.pending_library_removal_confirmation(), confirmation);
        assert_eq!(vm.status, status);
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

    fn confirm_when_referenced(vm: &mut LibraryViewModel, plan: LibraryRemovalPlan) -> bool {
        vm.confirm_library_removal(plan, RemovalConfirmation::WhenReferenced)
    }

    #[test]
    fn library_view_model_requires_explicit_confirmation_for_playlist_referenced_removals() {
        let mut vm = LibraryViewModel::new();

        assert!(confirm_when_referenced(
            &mut vm,
            LibraryRemovalPlan::new(
                LibraryRemovalTarget::Track(1),
                LibraryRemovalImpact::Track {
                    playlist_reference_count: 0,
                },
            )
        ));
        assert!(!confirm_when_referenced(
            &mut vm,
            LibraryRemovalPlan::new(
                LibraryRemovalTarget::Track(1),
                LibraryRemovalImpact::Track {
                    playlist_reference_count: 2,
                },
            )
        ));
        assert_eq!(
            vm.take_pending_library_removal(),
            Some(LibraryRemovalTarget::Track(1))
        );

        assert!(!confirm_when_referenced(
            &mut vm,
            LibraryRemovalPlan::new(
                LibraryRemovalTarget::Feed(5),
                LibraryRemovalImpact::Feed {
                    playlist_track_count: 1,
                },
            )
        ));
        let display = vm
            .pending_library_removal_confirmation()
            .expect("album removal should require confirmation");
        assert_eq!(display.title, "Remove Album from Library?");
        assert_eq!(
            display.message,
            "1 track from this album is in playlists. Removing it from the library will make it unavailable for playlist playback."
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
        assert_eq!(vm.selected_id, None);
        assert_eq!(vm.selected_playlist_id(), Some(7));
        assert!(vm.is_playlist_selected(7));

        vm.select_library_item(42);
        assert_eq!(vm.selected_id, Some(42));
        assert_eq!(vm.selected_playlist_id(), None);

        vm.clear_library_selection();
        assert_eq!(vm.selected_id, None);
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

    fn sample_busy_items() -> BusyItems {
        BusyItems::new(
            DownloadActivity {
                tracks: BTreeSet::from([99]),
                ..DownloadActivity::default()
            },
            BTreeSet::from([88]),
        )
    }

    fn track_of(id: i64, feed_id: i64, in_library: bool) -> TrackRow {
        TrackRow {
            id,
            feed_id,
            is_in_library: in_library,
            ..TrackRow::default()
        }
    }

    /// R22-09 with the orchestrator correction of 2026-10-20: a release
    /// shows `Queued` while a job of its key only waits, and `Downloading`
    /// while a job of its key runs. A track row follows its own job, or a
    /// release job of its key. A removal shows `Removing`.
    #[test]
    fn adr_0084_download_queue_r22_09_release_and_track_busy_follow_the_queue() {
        let mut album = minimal_album("Face Pale", 7);
        album.tracks = vec![track_of(70, 7, false), track_of(71, 7, false)];
        let key = album.feed_key().expect("a key");
        assert_eq!(key, FeedKey::Guid("album-guid-7".to_owned()));
        let mut vm = LibraryViewModel::new();
        assert!(!vm.busy_items().release_downloading(&album));

        vm.set_download_activity(DownloadActivity {
            tracks: BTreeSet::from([70]),
            keys: BTreeSet::from([key.clone()]),
            ..DownloadActivity::default()
        });
        let busy = vm.busy_items();
        assert!(busy.release_downloading(&album), "a queued track job");
        assert_eq!(
            busy.track_row_work(&album, &album.tracks[0]),
            TrackRowWork::Queued
        );
        assert_eq!(
            busy.track_row_work(&album, &album.tracks[1]),
            TrackRowWork::Idle,
            "no release job of the key"
        );
        let mb = BTreeMap::new();
        let album_vm = LibraryAlbumDetailVm::new(&album.tracks, &mb);
        assert_eq!(
            album_vm.membership(busy.release_download_phase(&album), busy.removing(&album)),
            ReleaseMembershipState::Queued
        );

        vm.set_download_activity(DownloadActivity {
            keys: BTreeSet::from([key.clone()]),
            release_keys: BTreeSet::from([key.clone()]),
            in_flight_keys: BTreeSet::from([key.clone()]),
            in_flight_release_keys: BTreeSet::from([key]),
            ..DownloadActivity::default()
        });
        let busy = vm.busy_items();
        assert_eq!(
            busy.track_row_work(&album, &album.tracks[1]),
            TrackRowWork::Downloading,
            "a running release job"
        );
        assert_eq!(
            album_vm.membership(busy.release_download_phase(&album), busy.removing(&album)),
            ReleaseMembershipState::Downloading
        );

        vm.set_download_activity(DownloadActivity::default());
        assert!(!vm.busy_items().release_downloading(&album));
        vm.begin_feed_removal(7);
        assert!(vm.busy_items().removing(&album));
        vm.finish_feed_removal(7);
        assert!(!vm.busy_items().removing(&album));
    }

    /// Orchestrator correction of 2026-10-20 (C1): of nine queued releases,
    /// three run and show `Downloading`. The six that wait show `Queued`.
    #[test]
    fn adr_0084_download_queue_nine_releases_three_downloading_six_queued() {
        let albums: Vec<AlbumNode> = (1..=9)
            .map(|feed_id| minimal_album(&format!("Release {feed_id}"), feed_id))
            .collect();
        let keys: BTreeSet<FeedKey> = albums
            .iter()
            .map(|album| album.feed_key().expect("a key"))
            .collect();
        let running: BTreeSet<FeedKey> = albums[..3]
            .iter()
            .map(|album| album.feed_key().expect("a key"))
            .collect();
        let mut vm = LibraryViewModel::new();
        vm.set_download_activity(DownloadActivity {
            keys: keys.clone(),
            release_keys: keys,
            in_flight_keys: running.clone(),
            in_flight_release_keys: running,
            ..DownloadActivity::default()
        });
        let busy = vm.busy_items();
        let states: Vec<ReleaseMembershipState> = albums
            .iter()
            .map(|album| {
                LibraryAlbumDetailVm::new(&album.tracks, vm.mb_status())
                    .membership(busy.release_download_phase(album), busy.removing(album))
            })
            .collect();
        let count = |state| states.iter().filter(|s| **s == state).count();
        assert_eq!(count(ReleaseMembershipState::Downloading), 3);
        assert_eq!(count(ReleaseMembershipState::Queued), 6);
    }

    /// Orchestrator correction of 2026-10-20 (C3): while a download job of
    /// a release key is queued or in flight, "Remove" of the release and
    /// of its tracks is absent. The Releases bar has no "Remove", the
    /// removal list is empty, and a Library track row has no "Remove".
    #[test]
    fn adr_0084_download_queue_remove_is_absent_while_the_key_is_active() {
        let mut album = minimal_album("Face Pale", 7);
        album.tracks = vec![track_of(70, 7, true), track_of(71, 7, true)];
        let release_key = album.release_key().expect("a release key");
        let feed_key = album.feed_key().expect("a key");
        let mut vm = LibraryViewModel::new();
        vm.replace_tree(LibraryTree {
            artists: vec![ArtistNode {
                name: "Haleen".to_owned(),
                albums: vec![album.clone()],
            }],
        });
        let mut selection: ListSelection<ReleaseKey, ReleasesRowId> = ListSelection::new();
        selection.toggle(&SelectionRow::new(
            ReleasesRowId::Release(release_key.clone()),
            vec![release_key.clone()],
        ));
        let remove_of = |vm: &LibraryViewModel| {
            vm.releases_selection_action_bar(&selection, SelectionMode::Selecting)
                .expect("a bar")
                .action(crate::view_models::selection_action_bar::SelectionActionKind::Remove)
                .map(|action| action.label.clone())
        };
        assert_eq!(remove_of(&vm).as_deref(), Some("Remove 1 release\u{2026}"));
        assert_eq!(vm.releases_removable_feed_ids(&selection).len(), 1);
        assert!(!vm.track_release_download_active(70));
        assert!(
            !vm.library_release_facts(&release_key)
                .expect("facts")
                .download_active
        );

        // A queued job of one track of the release.
        vm.set_download_activity(DownloadActivity {
            tracks: BTreeSet::from([71]),
            keys: BTreeSet::from([feed_key]),
            ..DownloadActivity::default()
        });
        assert_eq!(remove_of(&vm), None, "no Remove in the Releases bar");
        assert!(vm.releases_removable_feed_ids(&selection).is_empty());
        assert!(vm.track_release_download_active(70));
        let facts = vm.library_release_facts(&release_key).expect("facts");
        assert!(facts.download_active);
        assert!(!facts.removable());
        let busy = vm.busy_items();
        let work = busy.track_row_work(&album, &album.tracks[0]);
        assert_eq!(work, TrackRowWork::Queued);
        let action = LibraryTrackRowVm::new(&album.tracks[0], None).primary_action_vm(work);
        assert_ne!(
            action.kind,
            crate::view_models::entity_detail::EntityActionKind::Remove
        );
        assert_eq!(action.label, "Queued");
        assert!(!action.enabled);
        let album_vm = LibraryAlbumDetailVm::new(&album.tracks, vm.mb_status());
        assert_eq!(
            album_vm.membership(busy.release_download_phase(&album), busy.removing(&album)),
            ReleaseMembershipState::Queued
        );
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
    }

    #[test]
    fn library_track_action_vm_formats_subscription_labels() {
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
            LibraryTrackActionVm::track_subscribe_success_message(),
            "Downloaded track"
        );
    }

    #[test]
    fn library_track_action_vm_formats_playlist_label_and_message_status() {
        let closed = LibraryTrackActionVm::new(Some("Subscribed"));
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

        let open = LibraryTrackActionVm::new(Some("Error: offline"));
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
    fn library_view_model_track_subscribe_finish_formats_warning() {
        let mut vm = LibraryViewModel::new();

        vm.finish_track_subscribe(TrackSubscribeOutcome::new(
            "/tmp/song.mp3",
            Some("converted from WAV".into()),
        ));

        assert_eq!(
            vm.status(),
            "Downloaded track: /tmp/song.mp3 — converted from WAV"
        );
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
            Some("1 feed changed")
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

        vm.finish_all_feed_check_with_route_repair(FeedCheckRouteRepairOutcome::new(
            2, 1, 0, 0, 0, 0,
        ));
        assert_eq!(vm.feed_update_state().phase, FeedUpdatePhase::Idle);
        assert_eq!(
            vm.feed_update_state().status_message.as_deref(),
            Some("Checked 2 feeds; 1 feed changed; no route repairs needed")
        );

        vm.begin_feed_view_check(1);
        vm.finish_feed_view_check(
            1,
            Ok(Some(feed_service::StaleFeed {
                feed_id: 1,
                feed_guid: "one".into(),
                title: None,
                new_updated_at: 9,
            })),
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

        vm.begin_feed_view_check(2);
        vm.finish_feed_view_check(
            2,
            Ok(Some(feed_service::StaleFeed {
                feed_id: 2,
                feed_guid: "two".into(),
                title: None,
                new_updated_at: 10,
            })),
        );
        vm.begin_apply_feed_updates()
            .expect("stale feeds should apply");
        vm.finish_apply_feed_updates_error("offline");
        assert_eq!(vm.feed_update_state().phase, FeedUpdatePhase::Idle);
        assert_eq!(
            vm.feed_update_state().status_message.as_deref(),
            Some("Feed update error: offline")
        );
    }

    /// ADR 0083 task 005: the album membership selects the album page
    /// actions. A download job that waits shows `Queued`, a download job
    /// that runs shows `Downloading`, and a removal in progress shows
    /// `Removing` (ADR 0084 Decision 10).
    #[test]
    fn album_detail_vm_membership_follows_library_tracks_and_busy_state() {
        let mb = BTreeMap::new();
        let in_library = vec![TrackRow {
            is_in_library: true,
            ..TrackRow::default()
        }];
        let remote = vec![TrackRow {
            is_in_library: false,
            ..TrackRow::default()
        }];
        let vm = LibraryAlbumDetailVm::new(&in_library, &mb);
        assert_eq!(
            vm.membership(None, false),
            ReleaseMembershipState::InLibrary
        );
        assert_eq!(vm.membership(None, true), ReleaseMembershipState::Removing);
        assert_eq!(
            vm.membership(Some(DownloadPhase::InFlight), false),
            ReleaseMembershipState::Downloading,
            "a partly downloaded release with a running download"
        );
        let vm = LibraryAlbumDetailVm::new(&remote, &mb);
        assert_eq!(
            vm.membership(None, false),
            ReleaseMembershipState::RemoteOnly
        );
        assert_eq!(
            vm.membership(Some(DownloadPhase::InFlight), false),
            ReleaseMembershipState::Downloading
        );
        assert_eq!(
            vm.membership(Some(DownloadPhase::Queued), false),
            ReleaseMembershipState::Queued
        );
    }

    #[test]
    fn album_detail_vm_feed_download_marks_remote_rows_busy() {
        let mut album = minimal_album("Face Pale", 7);
        album.tracks = vec![track_of(70, 7, false)];
        let key = album.feed_key().expect("a key");
        let busy = BusyItems::new(
            DownloadActivity {
                keys: BTreeSet::from([key.clone()]),
                release_keys: BTreeSet::from([key]),
                ..DownloadActivity::default()
            },
            BTreeSet::new(),
        );

        assert_eq!(
            busy.track_row_work(&album, &album.tracks[0]),
            TrackRowWork::Queued
        );
    }

    #[test]
    fn album_detail_vm_feed_removal_marks_only_library_rows_busy() {
        let mut album = minimal_album("Face Pale", 7);
        album.tracks = vec![track_of(70, 7, true), track_of(71, 7, false)];
        let removing = BusyItems::new(DownloadActivity::default(), BTreeSet::from([7]));

        assert_eq!(
            removing.track_row_work(&album, &album.tracks[0]),
            TrackRowWork::Removing
        );
        assert_eq!(
            removing.track_row_work(&album, &album.tracks[1]),
            TrackRowWork::Idle
        );
        let track_job = BusyItems::new(
            DownloadActivity {
                tracks: BTreeSet::from([71]),
                in_flight_tracks: BTreeSet::from([71]),
                ..DownloadActivity::default()
            },
            BTreeSet::new(),
        );
        assert_eq!(
            track_job.track_row_work(&album, &album.tracks[1]),
            TrackRowWork::Downloading
        );
    }
}
