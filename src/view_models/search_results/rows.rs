//! The visible rows of the search results in the list column (ADR 0084
//! Decisions 2, 4, 5 and 8, packet 010).
//!
//! The search page keeps the found rows. At each render, the app layer
//! gives the Library facts through [`SearchLibraryLookup`], and this module
//! makes the state word, the badge and the selection facts of each row.
//! A download or a removal thus changes the rows in place.

#![warn(clippy::pedantic)]

use crate::view_models::artists_everything::FeedLinkClass;
use crate::view_models::collection_art::{CollectionArtVm, CollectionKind};
use crate::view_models::detail_selection::{DetailRowKey, DetailSelection, LibraryReleaseFacts};
use crate::view_models::list_selection::RowSelectionVm;
use crate::view_models::release_kind::ReleaseKind;
use crate::view_models::role_words::{capitalize_first, role_list_text};
use crate::view_models::workspace::{ReleaseKey, TrackKey};

use super::{
    ArtistResultDisplay, FeedResultDisplay, SearchResultTarget, SearchResultsTab,
    TrackResultDisplay,
};

/// The Library facts of one release for a search row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SearchLibraryRelease {
    /// The facts of the selection model of packet 018.
    pub(crate) facts: LibraryReleaseFacts,
    /// The state word of ADR 0084 Decision 4, or the queue word.
    pub(crate) state_word: String,
    /// The kind of the Library release (ADR 0084 Decision 5).
    pub(crate) kind: ReleaseKind,
}

/// The Library facts of one track for a search row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SearchLibraryTrack {
    /// The Library track id.
    pub(crate) track_id: i64,
    /// The audio file of the track is downloaded.
    pub(crate) downloaded: bool,
    /// A download job of the release of the track is queued or in flight.
    pub(crate) download_active: bool,
    /// "In Library" or "Not downloaded", or the queue word.
    pub(crate) state_word: String,
}

/// The Library facts that the search rows read at each render. The
/// Library view model gives them. A test gives recorded facts.
pub(crate) trait SearchLibraryLookup {
    /// The Library facts of the release `key`. `None` when the Library
    /// holds no such release.
    fn release(&self, key: &ReleaseKey) -> Option<SearchLibraryRelease>;

    /// The Library facts of the track `key`. `None` when the Library holds
    /// no such track.
    fn track(&self, key: &TrackKey) -> Option<SearchLibraryTrack>;

    /// Tells if the Artists list of the Library has a feed level row of
    /// the feed `feed_guid` (packet 010 decision D5).
    fn is_artist_feed(&self, feed_guid: &str) -> bool;
}

/// The badge at the end of a search row (packet 010 decisions D5 and D11,
/// packet 029 decision D2).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SearchRowBadge {
    /// The kind of a release row (ADR 0084 Decision 5).
    Release(ReleaseKind),
    /// A feed row whose feed is an artist feed of the Library, or a confirmed
    /// or an unverified artist by its link facts.
    Artist,
    /// A feed row with agreed roles and no `artist` role. The text is the
    /// role words with a capital first letter, for example "Label" (ADR
    /// 0085 Decision 7, the frame title of the feed page).
    Roles(String),
    /// Each other feed row. No row text calls a feed a "publisher" (ADR
    /// 0085 Decision 1).
    Feed,
}

impl SearchRowBadge {
    /// The visible badge text.
    #[must_use]
    pub(crate) fn label(&self) -> &str {
        match self {
            Self::Release(kind) => kind.label(),
            Self::Artist => "Artist",
            Self::Roles(words) => words,
            Self::Feed => "Feed",
        }
    }

    /// The badge of a feed row, in the sequence of packet 029 "The badge".
    /// A feed of the Library Artists list is "Artist" whatever its link
    /// facts (decision D3). A row with no link class is "Feed".
    fn for_feed(is_library_artist: bool, link_class: Option<&FeedLinkClass>) -> Self {
        if is_library_artist {
            return Self::Artist;
        }
        match link_class {
            Some(FeedLinkClass::Artist | FeedLinkClass::UnverifiedArtist) => Self::Artist,
            Some(FeedLinkClass::OtherRoles(words)) => {
                Self::Roles(capitalize_first(&role_list_text(words)))
            }
            Some(FeedLinkClass::NotNamedFirst) | None => Self::Feed,
        }
    }
}

/// The place of the kind of a release row (walk of 2026-10-21, operator
/// decision). The view model decides it from the column of the page.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReleaseKindPlacement {
    /// The search list in the list column: the kind is the first word of
    /// the second line, for example `Album · HeyCitizen · 19 tracks`. The
    /// row has no badge, so the title has the width of the column.
    Subtitle,
    /// The saved-search page in the detail column (packet 012): the kind
    /// is a badge at the trailing edge.
    Badge,
}

/// The separator of the parts of the second line of a release row in the
/// search list.
const SEARCH_LIST_SUBTITLE_SEPARATOR: &str = " \u{b7} ";

/// One visible search row. The shell lays it out and makes no text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SearchResultRowVm {
    /// The row id. [`SearchResultTarget::from_row_id`] parses it.
    pub(crate) id: String,
    /// The title.
    pub(crate) label: String,
    /// The second line.
    pub(crate) secondary_text: String,
    /// The thumbnail href, when the row has one.
    pub(crate) thumbnail_href: Option<String>,
    /// The logo frame art of a Library name row (packet 028 D3), with its
    /// frame kind (ADR 0084 task 030, decision D11). A row with art shows
    /// the logo frame in place of the thumbnail.
    pub(crate) art: Option<SearchRowArtVm>,
    /// The accessibility label of the row.
    pub(crate) a11y_label: String,
    /// The state word of a Library item. An Index item shows no word (the
    /// operator decision of 2026-10-16).
    pub(crate) state_word: Option<String>,
    /// The badge, when the row has one. A track row and a name row have
    /// none.
    pub(crate) badge: Option<SearchRowBadge>,
    /// The checkbox facts of the row (ADR 0084 Decision 10).
    pub(crate) selection: RowSelectionVm,
    /// The selection key of the row. `None` for a row with no checkbox.
    pub(crate) key: Option<DetailRowKey>,
}

/// The logo frame of a search row: the art and the kind of its frame (ADR
/// 0084 Decision 12, task 030 decision D11). The view model selects the
/// kind, so the renderer names no kind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SearchRowArtVm {
    /// The kind of the frame. It selects the frame color.
    pub(crate) kind: CollectionKind,
    /// The art in the frame.
    pub(crate) art: CollectionArtVm,
}

/// The visible row of a release result.
/// `placement` puts the kind in the second line or in the badge.
pub(super) fn release_row_vm(
    row: &FeedResultDisplay,
    lookup: &dyn SearchLibraryLookup,
    selection: &DetailSelection,
    placement: ReleaseKindPlacement,
) -> SearchResultRowVm {
    let library = row.key.as_ref().and_then(|key| lookup.release(key));
    let kind = library.as_ref().map_or(row.kind, |library| library.kind);
    let key = row.key.clone().map(DetailRowKey::Release);
    let (secondary_text, badge) = match placement {
        ReleaseKindPlacement::Subtitle => (list_subtitle(kind, row), None),
        ReleaseKindPlacement::Badge => (
            row.secondary_text.clone(),
            Some(SearchRowBadge::Release(kind)),
        ),
    };
    SearchResultRowVm {
        id: row.id.clone(),
        label: row.label.clone(),
        secondary_text,
        thumbnail_href: row.thumbnail_href.clone(),
        art: None,
        a11y_label: row.a11y_label.clone(),
        state_word: library.map(|library| library.state_word),
        badge,
        selection: selection.row_selection(key.as_ref(), &row.label),
        key,
    }
}

/// The second line of a release row in the search list: the kind, then
/// each part of the row, with one separator. A row with no parts uses its
/// secondary text as one part.
fn list_subtitle(kind: ReleaseKind, row: &FeedResultDisplay) -> String {
    let mut parts = vec![kind.label().to_owned()];
    if row.secondary_parts.is_empty() {
        if !row.secondary_text.is_empty() {
            parts.push(row.secondary_text.clone());
        }
    } else {
        parts.extend(row.secondary_parts.iter().cloned());
    }
    parts.join(SEARCH_LIST_SUBTITLE_SEPARATOR)
}

/// The visible row of a track result.
pub(super) fn track_row_vm(
    row: &TrackResultDisplay,
    lookup: &dyn SearchLibraryLookup,
    selection: &DetailSelection,
) -> SearchResultRowVm {
    let library = row.key.as_ref().and_then(|key| lookup.track(key));
    let key = row.key.clone().map(DetailRowKey::Track);
    SearchResultRowVm {
        id: row.id.clone(),
        label: row.label.clone(),
        secondary_text: row.secondary_text.clone(),
        thumbnail_href: row.thumbnail_href.clone(),
        art: None,
        a11y_label: row.a11y_label.clone(),
        state_word: library.map(|library| library.state_word),
        badge: None,
        selection: selection.row_selection(key.as_ref(), &row.label),
        key,
    }
}

/// The visible row of an artist result. A feed row has the badge "Artist",
/// the role words or "Feed" (packet 029 "The badge"). The accessibility
/// label of a feed row ends with the badge word (packet 029 decision D7).
/// A name row has no badge. The Artists tab has no selection model.
pub(super) fn artist_row_vm(
    row: &ArtistResultDisplay,
    lookup: &dyn SearchLibraryLookup,
) -> SearchResultRowVm {
    let badge = match SearchResultTarget::from_row_id(SearchResultsTab::Artists, &row.id) {
        Some(SearchResultTarget::FeedPage(feed_guid)) => Some(SearchRowBadge::for_feed(
            lookup.is_artist_feed(&feed_guid),
            row.link_class.as_ref(),
        )),
        _ => None,
    };
    let a11y_label = match &badge {
        Some(badge) => format!("{}, {}", row.a11y_label, badge.label()),
        None => row.a11y_label.clone(),
    };
    SearchResultRowVm {
        id: row.id.clone(),
        label: row.label.clone(),
        secondary_text: row.secondary_text.clone(),
        thumbnail_href: row.thumbnail_href.clone(),
        // A row with art is a Library name row of the Artists tab. Its
        // frame has the artist kind, as on its Artists list row.
        art: row.art.clone().map(|art| SearchRowArtVm {
            kind: CollectionKind::Artist,
            art,
        }),
        a11y_label,
        state_word: None,
        badge,
        selection: RowSelectionVm::none(),
        key: None,
    }
}
