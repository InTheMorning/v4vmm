//! Feed page view model (ADR 0077 Task 003, ADR 0082, ADR 0077 Task 007,
//! ADR 0085 Decisions 2 to 4 and 6 to 7).
//!
//! The Index feed page query and the Library feed page query, in
//! `src/application/queries`, each build one [`PublisherPageFacts`] value
//! from their own data. This module turns that value into the
//! confirmation level of ADR 0085 Decision 2, the sections of Decision 3,
//! the frame title of Decision 7, the header roles of ADR 0082 Decision 4,
//! the "Shares albums with" entries of Decision 5, the title, and the
//! confirmed and unconfirmed artist counts. No renderer computes these
//! facts. [`PublisherPageVm`] computes each one, and
//! `src/ui/shells/publisher.rs` composes a screen from it.
//!
//! The type keeps the name `PublisherPageVm`: ADR 0085 Decision 1 bars the
//! word "publisher" from interface text, not from a code identifier.

#![warn(clippy::pedantic)]

use crate::api::{AlbumNamesAs, PublisherLinkResolution, RoleAgreement};
use crate::view_models::release_kind::ReleaseKind;
use crate::view_models::role_words::{
    capitalize_first, role_contains_artist, role_list_text, role_words, stated_sides_text,
};
use crate::view_models::workspace::ReleaseKey;
use std::collections::BTreeMap;

/// The route that opened a feed page (ADR 0077 packet 004). This value
/// selects which query built the facts, and the loading message. It
/// decides no section, no level and no action availability: those come
/// from [`PublisherPageVm`] alone, the same for each context (ADR 0085
/// Decision 4).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PublisherPageContext {
    /// Reached from the Library.
    Library,
    /// Reached from the Index.
    Index,
}

impl PublisherPageContext {
    /// The route name for a status report (packet 004).
    #[must_use]
    pub(crate) const fn route_name(self) -> &'static str {
        match self {
            Self::Library => "Library",
            Self::Index => "Index",
        }
    }
}

/// One album row on a feed page, gathered by the owning query.
///
/// On the Index feed page, every fact comes from the `remote_*` summary
/// fields of one `publisher_to_music` entry (Stophammer ADR 0059, ADR 0069
/// §3a). On the Library feed page, a Library album's fact set comes from
/// the stored local feed row instead, so `artist` and `artist_source` stay
/// `None` there: the app stores no `remote_release_artist` value.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PublisherPageAlbumFact {
    /// The album feed's own GUID.
    pub(crate) feed_guid: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) image_url: Option<String>,
    pub(crate) artist: Option<String>,
    pub(crate) artist_source: Option<String>,
    /// The role set of this link, sorted and joined by `", "`.
    pub(crate) role: Option<String>,
    /// `true` when the album names this feed (ADR 0077 Decision 2).
    pub(crate) music_names_publisher: Option<bool>,
    /// `true` when this feed lists this album.
    pub(crate) publisher_lists_music: Option<bool>,
    pub(crate) publisher_link_resolution: Option<PublisherLinkResolution>,
    /// The raw `rel` this feed's item stated for this album.
    pub(crate) publisher_rel: Option<String>,
    /// The raw `rel` the album feed item stated for this feed.
    pub(crate) music_rel: Option<String>,
    /// How this album names this feed (ADR 0082 Decision 2).
    pub(crate) album_names_as: Option<AlbumNamesAs>,
    /// Whether the two feeds agree on `role` (ADR 0082 Decision 2).
    pub(crate) role_agreement: Option<RoleAgreement>,
    /// `true` when this album's own feed is in the Library.
    pub(crate) in_library: bool,
    /// The rank of this feed in the named-feed sequence of the album, from
    /// 1 (ADR 0085 Decision 6, Stophammer `album_names_rank`). `None` when
    /// `album_names_as` is null, or when the rank is not known.
    pub(crate) named_rank: Option<i64>,
    /// The track count of the album's own feed (Stophammer
    /// `remote_episode_count`, an Index row; the Library track count, a
    /// Library row). `None` only when neither side states it.
    pub(crate) track_count: Option<i64>,
    /// `true` when the two feeds name each other (ADR 0085 Decision 2).
    pub(crate) two_way_validated: Option<bool>,
}

/// The raw facts one feed page query gathers. [`PublisherPageVm`] turns
/// these into the page's display facts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PublisherPageFacts {
    /// The feed's own GUID (ADR 0077 Decision 1). The page identity, and
    /// the fallback title text.
    pub(crate) publisher_feed_guid: String,
    /// The feed's own `<title>`.
    pub(crate) feed_title: Option<String>,
    /// The feed's own channel image (ADR 0085 Decision 7 design, Required
    /// Change 7). The Index path reads `api::Feed::image_url`. The Library
    /// path reads the stored image of the feed when the Library holds the
    /// feed, else this value.
    pub(crate) feed_image_url: Option<String>,
    /// `confirmed_release_artist_count` of this feed: the distinct artists
    /// of the albums that name this feed and that this feed also lists
    /// (Stophammer ADR 0061, ADR 0077 Task 007). `MusicIndex` derives it.
    pub(crate) confirmed_release_artist_count: Option<i64>,
    pub(crate) confirmed_release_artists: Vec<String>,
    /// `unconfirmed_release_artist_count` of this feed: the distinct
    /// artists of the albums that this feed lists but that do not name it
    /// (Stophammer ADR 0061, ADR 0077 Task 007). `MusicIndex` derives it.
    pub(crate) unconfirmed_release_artist_count: Option<i64>,
    pub(crate) unconfirmed_release_artists: Vec<String>,
    pub(crate) albums: Vec<PublisherPageAlbumFact>,
    /// Each other feed that shares a confirmed album with this feed (ADR
    /// 0082 Decision 5). Empty when the response names none.
    pub(crate) co_credited_feeds: Vec<CoCreditedFeedFact>,
    /// R3-02a: the reason the request for the albums that are not in the
    /// Library failed. `None` when that request has not failed. The
    /// Library rows stay present either way.
    pub(crate) other_albums_failure: Option<String>,
}

/// One entry of "Shares albums with" (ADR 0082 Decision 5): another feed
/// that shares a confirmed album with the feed this page describes. The
/// owning query builds one of these from each `co_credited_feeds` item of
/// the response.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CoCreditedFeedFact {
    /// The co-credited feed's own GUID. An entry with no GUID has no page
    /// to open, so [`PublisherPageVm::shares_albums_with`] drops it.
    pub(crate) feed_guid: Option<String>,
    /// The co-credited feed's own title, or `None` when the node holds no
    /// feed for it.
    pub(crate) title: Option<String>,
    /// The different raw `rel` values the shared albums give this feed,
    /// sorted. Empty when no album states one.
    pub(crate) roles: Vec<String>,
    /// The number of shared confirmed albums.
    pub(crate) album_count: Option<i64>,
}

/// A page or album title, ready to view (ADR 0077 Accepted Refinements,
/// R3-10, R3-14).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TitleDisplay {
    /// The feed's own stated title.
    Stated(String),
    /// The feed's GUID. The feed states no title.
    Missing(String),
}

impl TitleDisplay {
    /// The label the page shows next to a missing title.
    pub(crate) const MISSING_LABEL: &'static str = "No title";

    fn from_stated(title: Option<&str>, guid: &str) -> Self {
        match title.map(str::trim).filter(|title| !title.is_empty()) {
            Some(title) => Self::Stated(title.to_owned()),
            None => Self::Missing(guid.to_owned()),
        }
    }

    /// The title text, ready to view: the stated title, or the GUID when
    /// the feed states none.
    #[must_use]
    pub(crate) fn text(&self) -> &str {
        match self {
            Self::Stated(text) | Self::Missing(text) => text,
        }
    }
}

/// The artist of one album, from `remote_release_artist` (R3-14). `None`
/// when the entry states no artist. The view model invents no placeholder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AlbumArtistDisplay {
    pub(crate) name: String,
    pub(crate) source: Option<String>,
}

impl AlbumArtistDisplay {
    /// The artist name, with its stored owner as supporting text (ADR 0075
    /// Decision I, packet 004 R4-05). A missing source shows the name
    /// alone. The screen composes no text of its own.
    #[must_use]
    pub(crate) fn display_text(&self) -> String {
        self.source.as_deref().map_or_else(
            || self.name.clone(),
            |source| format!("{} ({source})", self.name),
        )
    }
}

/// The open action of a release row (ADR 0085 Decision 7 design, Required
/// Change 7). The view model decides its availability: a row with no feed
/// GUID has no open action, so [`PublisherPageAlbumVm::open`] is `None`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReleaseRowOpenAction {
    /// The release this action opens. The one release route then selects
    /// the Library source or the Index source (ADR 0084 Decision 4).
    pub(crate) key: ReleaseKey,
    /// The accessibility label, for example "Open the release Face Pale".
    pub(crate) a11y_label: String,
}

/// One album row, ready to view (ADR 0085 Decision 3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PublisherPageAlbumVm {
    pub(crate) feed_guid: Option<String>,
    pub(crate) title: TitleDisplay,
    pub(crate) image_url: Option<String>,
    pub(crate) artist: Option<AlbumArtistDisplay>,
    /// R3-09: an owned album with `publisher_lists_music = false` or an
    /// unresolved link.
    pub(crate) not_listed: bool,
    pub(crate) in_library: bool,
    /// The detail line of a "Role not confirmed" row: who stated which
    /// role, by feed title (ADR 0085 Decision 3). `None` in every other
    /// section: its heading already tells the relation.
    pub(crate) role_line: Option<String>,
    /// The open action of this row. `None` when the row has no feed GUID.
    pub(crate) open: Option<ReleaseRowOpenAction>,
}

impl PublisherPageAlbumVm {
    /// The mark on an owned album that this feed does not list (ADR 0077
    /// Accepted Refinements R3-09, ADR 0085 Decision 3 V3 carry-over).
    pub(crate) const NOT_LISTED_LABEL: &'static str = "Not listed by this feed";

    /// The mark on an album whose own feed is in the Library.
    pub(crate) const IN_LIBRARY_LABEL: &'static str = "In Library";
}

/// One section of a feed page, ready to view (ADR 0085 Decision 3). A
/// release appears in exactly one section. An empty section is absent:
/// [`PublisherPageVm::sections`] never builds one with no row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PublisherPageSectionVm {
    pub(crate) heading: String,
    pub(crate) rows: Vec<PublisherPageAlbumVm>,
}

/// One "Shares albums with" entry, ready to view (ADR 0082 Decision 5).
/// `PublisherPageVm::shares_albums_with` builds one of these from each
/// `co_credited_feeds` item that carries a feed GUID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CoCreditedFeedVm {
    pub(crate) title: TitleDisplay,
    pub(crate) roles: Vec<String>,
    /// The number of shared confirmed albums. `None` when the response
    /// states no count: the view model invents no value for it.
    pub(crate) album_count: Option<i64>,
    /// The GUID of the feed page this entry opens. The screen builds no
    /// action target of its own.
    pub(crate) publisher_feed_guid: String,
    /// The accessibility label of the open-feed action.
    pub(crate) a11y_label: String,
}

impl CoCreditedFeedVm {
    /// The word a shared album count shows when no role is stated.
    const NO_ROLES_LABEL: &'static str = "No stated role";
    /// The word after the count when it is exactly one.
    const SHARED_ALBUM_LABEL: &'static str = "shared album";
    /// The word after the count when it is zero or more than one.
    const SHARED_ALBUMS_LABEL: &'static str = "shared albums";

    /// The roles and the shared album count, ready to view (ADR 0082
    /// Decision 5). The screen composes no text of its own. An absent
    /// count shows the roles alone: this method invents no count.
    #[must_use]
    pub(crate) fn roles_and_count_text(&self) -> String {
        let roles = if self.roles.is_empty() {
            Self::NO_ROLES_LABEL.to_owned()
        } else {
            self.roles.join(", ")
        };
        match self.album_count {
            Some(count) => {
                let album_word = if count == 1 {
                    Self::SHARED_ALBUM_LABEL
                } else {
                    Self::SHARED_ALBUMS_LABEL
                };
                format!("{roles} \u{2014} {count} {album_word}")
            }
            None => roles,
        }
    }
}

/// The state of the albums that are not in the Library (R3-02a).
///
/// The view model gives a display state, never a transport error. The
/// report comes first, and the technical detail follows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum OtherAlbumsStatus {
    /// The request for these albums succeeded, or the page needs none.
    Loaded,
    /// The request failed. The Library albums stay on the page.
    Unavailable {
        report: &'static str,
        detail: String,
    },
}

impl OtherAlbumsStatus {
    /// The report for a failed request for the other releases of a feed.
    pub(crate) const UNAVAILABLE_REPORT: &'static str = "The app could not load the other releases of this feed from MusicIndex. The releases in the Library stay on this page.";
}

/// Display state of the mounted feed page while it loads, when its fetch
/// fails, or when none is open (ADR 0077 packet 004). The app layer only
/// selects which state applies; this module decides its text, the same as
/// `OtherAlbumsStatus`. A `Failed` state gives a report first, and its
/// technical detail after it, apart from the report (R3-02a). This type
/// carries no transport error: the app layer turns one into text before it
/// reaches here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PublisherPageLoadDisplay {
    Loading {
        message: String,
    },
    Failed {
        report: &'static str,
        detail: String,
    },
    Empty {
        message: String,
    },
}

impl PublisherPageLoadDisplay {
    /// The report for a feed page that failed to load.
    pub(crate) const FAILED_REPORT: &'static str =
        "The app could not load this feed page from MusicIndex.";

    /// The message shown when no feed page is open.
    pub(crate) const EMPTY_MESSAGE: &'static str = "No feed page is open.";

    /// The display for a feed page in flight (packet 004).
    #[must_use]
    pub(crate) fn loading(context: PublisherPageContext, publisher_feed_guid: &str) -> Self {
        Self::Loading {
            message: format!(
                "Loading the {} feed page {publisher_feed_guid}...",
                context.route_name()
            ),
        }
    }

    /// The display for a feed page whose fetch failed. `detail` is the
    /// transport error, already turned into text by the app layer.
    #[must_use]
    pub(crate) fn failed(detail: impl Into<String>) -> Self {
        Self::Failed {
            report: Self::FAILED_REPORT,
            detail: detail.into(),
        }
    }

    /// The display for a frame with no mounted feed page.
    #[must_use]
    pub(crate) fn empty() -> Self {
        Self::Empty {
            message: Self::EMPTY_MESSAGE.to_owned(),
        }
    }
}

/// The confirmation level of a feed (ADR 0085 Decision 2). The Artists list
/// shows no level word; only the feed page header shows it (Decision 3).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FeedLevel {
    /// One row has `album_names_as` `publisher`, `role_agreement` `both`,
    /// and a role set that contains `artist`.
    Artist,
    /// No row has `role_agreement` `both`. A row with `two_way_validated`
    /// true has `album_names_as` `publisher` and `named_rank` 1.
    UnverifiedArtist,
    /// Each other page.
    None,
}

impl FeedLevel {
    /// The header text of the `Artist` level, and the frame title of the
    /// `Artist` and `UnverifiedArtist` levels (ADR 0085 Decision 7).
    pub(crate) const ARTIST_LABEL: &'static str = "Artist";
    /// The header text of the `UnverifiedArtist` level.
    pub(crate) const UNVERIFIED_ARTIST_LABEL: &'static str = "Unverified artist";

    /// The header fact value of this level. `None` adds no header fact.
    #[must_use]
    pub(crate) const fn header_text(self) -> Option<&'static str> {
        match self {
            Self::Artist => Some(Self::ARTIST_LABEL),
            Self::UnverifiedArtist => Some(Self::UNVERIFIED_ARTIST_LABEL),
            Self::None => None,
        }
    }
}

/// The derived artist count of a feed (ADR 0077 Task 007). `MusicIndex`
/// derives it from album credits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DerivedArtistCount {
    pub(crate) count: i64,
    pub(crate) names: Vec<String>,
}

impl DerivedArtistCount {
    /// The label the page shows next to the count, naming it as derived,
    /// not stated (ADR 0075 Decision I).
    pub(crate) const LABEL: &'static str = "Derived from album credits";

    /// The count, its derived label, and its artist names, ready to view
    /// (packet 004). The screen composes no text of its own.
    #[must_use]
    pub(crate) fn display_text(&self) -> String {
        if self.names.is_empty() {
            format!("{} \u{2014} {}", self.count, Self::LABEL)
        } else {
            format!(
                "{} \u{2014} {} ({})",
                self.count,
                Self::LABEL,
                self.names.join(", ")
            )
        }
    }
}

/// One header fact of a feed page, ready to view (packet 004). The screen
/// shows each fact as a label/value pair. It decides no label and composes
/// no text of its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PublisherPageHeaderFact {
    pub(crate) label: &'static str,
    pub(crate) value: String,
}

/// The feed page view model (ADR 0077 Task 003). The type keeps its name:
/// ADR 0085 Decision 1 bars the word from interface text, not from a code
/// identifier.
pub(crate) struct PublisherPageVm {
    facts: PublisherPageFacts,
}

impl PublisherPageVm {
    /// Row label for the "No title" fact, shown only when the feed states
    /// no title.
    pub(crate) const TITLE_LABEL: &'static str = "Title";
    /// Row label for the confirmation level fact (ADR 0085 Decision 2).
    pub(crate) const LEVEL_LABEL: &'static str = "Confirmation";
    /// Row label for the agreed roles fact (ADR 0082 Decision 4).
    pub(crate) const ROLES_LABEL: &'static str = "Roles";
    /// Row label for the confirmed artist count fact (ADR 0077 Task 007):
    /// the artists of the albums that name this feed.
    pub(crate) const CONFIRMED_ARTISTS_LABEL: &'static str = "Artists that name this feed";
    /// Row label for the unconfirmed artist count fact (ADR 0077 Task 007):
    /// the artists of the albums that this feed lists without a link back.
    pub(crate) const UNCONFIRMED_ARTISTS_LABEL: &'static str =
        "Artists this feed lists without a link back";
    /// Caption of the "Shares albums with" section (ADR 0082 Decision 5).
    pub(crate) const SHARES_ALBUMS_WITH_LABEL: &'static str = "Shares albums with";

    /// Heading of the discography section for an Album or an EP (ADR 0085
    /// Decision 3, ADR 0084 Decision 6).
    pub(crate) const ALBUMS_AND_EPS_HEADING: &'static str = "Albums and EPs";
    /// Heading of the discography section for a Single.
    pub(crate) const SINGLES_HEADING: &'static str = "Singles";
    /// Heading of the section for a stated role that is not confirmed.
    pub(crate) const ROLE_NOT_CONFIRMED_HEADING: &'static str = "Role not confirmed";
    /// The fallback name of this feed in its "Listed by … only" heading,
    /// when the feed states no title.
    const LISTED_ONLY_FALLBACK_NAME: &'static str = "this feed";

    /// The frame title of the `Artist` and `UnverifiedArtist` levels (ADR
    /// 0085 Decision 7).
    pub(crate) const FRAME_TITLE_ARTIST: &'static str = FeedLevel::ARTIST_LABEL;
    /// The frame title of a feed that is never the first named feed of an
    /// album, with no agreed role and no side that states `artist` (ADR
    /// 0085 Decision 7). This is the one place that names a feed
    /// "publisher": the ADR 0085 Decision 1 guard permits this one
    /// constant, by its name, `FRAME_TITLE_PUBLISHER`.
    pub(crate) const FRAME_TITLE_PUBLISHER: &'static str = "Publisher";
    /// The frame title of each other page, including one that has not
    /// loaded (ADR 0085 Decision 7).
    pub(crate) const FRAME_TITLE_FEED: &'static str = "Feed";

    #[must_use]
    pub(crate) const fn new(facts: PublisherPageFacts) -> Self {
        Self { facts }
    }

    /// R3-10: the feed's own title, or its GUID with the "No title" label.
    #[must_use]
    pub(crate) fn title(&self) -> TitleDisplay {
        TitleDisplay::from_stated(
            self.facts.feed_title.as_deref(),
            &self.facts.publisher_feed_guid,
        )
    }

    /// The page title text, ready to view (R4-04). `Missing` gives the
    /// feed's own GUID: the same fallback `title` gives for the page.
    #[must_use]
    pub(crate) fn title_text(&self) -> String {
        self.title().text().to_owned()
    }

    /// The feed's own channel image URL, ready for the app layer to
    /// resolve through the shared thumbnail path (ADR 0085 Decision 7
    /// design, Required Change 7). `None` when neither side states one.
    #[must_use]
    pub(crate) fn header_image_url(&self) -> Option<String> {
        self.facts.feed_image_url.clone()
    }

    /// The confirmation level of this feed (ADR 0085 Decision 2). It reads
    /// only `album_names_as`, `role_agreement`, `role`, `two_way_validated`
    /// and `named_rank`.
    #[must_use]
    pub(crate) fn level(&self) -> FeedLevel {
        let artist_confirmed = self.facts.albums.iter().any(|album| {
            album.album_names_as == Some(AlbumNamesAs::Publisher)
                && album.role_agreement == Some(RoleAgreement::Both)
                && role_contains_artist(album.role.as_deref())
        });
        if artist_confirmed {
            return FeedLevel::Artist;
        }
        let any_agreed_role = self
            .facts
            .albums
            .iter()
            .any(|album| album.role_agreement == Some(RoleAgreement::Both));
        if any_agreed_role {
            return FeedLevel::None;
        }
        let unverified = self.facts.albums.iter().any(|album| {
            album.two_way_validated == Some(true)
                && album.album_names_as == Some(AlbumNamesAs::Publisher)
                && album.named_rank == Some(1)
        });
        if unverified {
            FeedLevel::UnverifiedArtist
        } else {
            FeedLevel::None
        }
    }

    /// The frame title of this feed page (ADR 0085 Decision 7).
    #[must_use]
    pub(crate) fn frame_title(&self) -> String {
        match self.level() {
            FeedLevel::Artist | FeedLevel::UnverifiedArtist => {
                return Self::FRAME_TITLE_ARTIST.to_owned();
            }
            FeedLevel::None => {}
        }
        let agreed_roles = self.agreed_role_words();
        if !agreed_roles.is_empty() {
            return capitalize_first(&role_list_text(&agreed_roles));
        }
        let any_side_states_artist = self.facts.albums.iter().any(|album| {
            role_contains_artist(album.publisher_rel.as_deref())
                || role_contains_artist(album.music_rel.as_deref())
        });
        let named_rows: Vec<&PublisherPageAlbumFact> = self
            .facts
            .albums
            .iter()
            .filter(|album| album.album_names_as == Some(AlbumNamesAs::Publisher))
            .collect();
        let each_named_rank_known_and_not_one = !named_rows.is_empty()
            && named_rows
                .iter()
                .all(|album| matches!(album.named_rank, Some(rank) if rank != 1));
        if !any_side_states_artist && each_named_rank_known_and_not_one {
            return Self::FRAME_TITLE_PUBLISHER.to_owned();
        }
        Self::FRAME_TITLE_FEED.to_owned()
    }

    /// The header facts of this page, ready to view (ADR 0085 Decision 2,
    /// ADR 0082 Decision 4). The screen shows each fact as a label/value
    /// pair; it decides no label and composes no text.
    #[must_use]
    pub(crate) fn header_facts(&self) -> Vec<PublisherPageHeaderFact> {
        let mut facts = Vec::new();
        if let Some(level_text) = self.level().header_text() {
            facts.push(PublisherPageHeaderFact {
                label: Self::LEVEL_LABEL,
                value: level_text.to_owned(),
            });
        }
        let agreed_roles = self.agreed_role_words();
        if !agreed_roles.is_empty() {
            facts.push(PublisherPageHeaderFact {
                label: Self::ROLES_LABEL,
                value: agreed_roles.join(", "),
            });
        }
        if matches!(self.title(), TitleDisplay::Missing(_)) {
            facts.push(PublisherPageHeaderFact {
                label: Self::TITLE_LABEL,
                value: TitleDisplay::MISSING_LABEL.to_owned(),
            });
        }
        if let Some(count) = self.confirmed_artist_count() {
            facts.push(PublisherPageHeaderFact {
                label: Self::CONFIRMED_ARTISTS_LABEL,
                value: count.display_text(),
            });
        }
        if let Some(count) = self.unconfirmed_artist_count() {
            facts.push(PublisherPageHeaderFact {
                label: Self::UNCONFIRMED_ARTISTS_LABEL,
                value: count.display_text(),
            });
        }
        facts
    }

    /// The different roles of the agreed rows, sorted (ADR 0082 Decision
    /// 4). Each agreed row's role set splits into its own role words, so
    /// an agreed "label, producer" row contributes both "label" and
    /// "producer". Empty when the page has no agreed row.
    fn agreed_role_words(&self) -> Vec<String> {
        let combined = self
            .facts
            .albums
            .iter()
            .filter(|album| album.role_agreement == Some(RoleAgreement::Both))
            .filter_map(|album| album.role.as_deref())
            .collect::<Vec<_>>()
            .join(",");
        role_words(Some(&combined))
    }

    /// The sections of this feed page, in the sequence of ADR 0085
    /// Decision 3. A section with no row is absent. Each release appears
    /// one time. The Library context and the Index context give the same
    /// sections for the same facts (ADR 0084 Decision 4): the owning query
    /// already joins the Library rows and the other rows into one list
    /// before this method runs.
    #[must_use]
    pub(crate) fn sections(&self) -> Vec<PublisherPageSectionVm> {
        let mut albums_and_eps = Vec::new();
        let mut singles = Vec::new();
        let mut as_roles: BTreeMap<String, Vec<PublisherPageAlbumVm>> = BTreeMap::new();
        let mut role_not_confirmed = Vec::new();
        let mut listed_only = Vec::new();
        let feed_title_text = self.title_text();

        for album in &self.facts.albums {
            match Self::section_bucket(album) {
                SectionBucket::Discography => {
                    let vm = Self::album_vm(album, None);
                    let track_count = usize::try_from(album.track_count.unwrap_or(0)).unwrap_or(0);
                    if ReleaseKind::resolve(None, track_count).is_single() {
                        singles.push(vm);
                    } else {
                        albums_and_eps.push(vm);
                    }
                }
                SectionBucket::AsRoles => {
                    let words = role_words(album.role.as_deref());
                    let heading = format!("As {}", role_list_text(&words));
                    as_roles
                        .entry(heading)
                        .or_default()
                        .push(Self::album_vm(album, None));
                }
                SectionBucket::RoleNotConfirmed => {
                    let role_line = stated_sides_text(
                        &feed_title_text,
                        album.publisher_rel.as_deref(),
                        album.music_rel.as_deref(),
                    );
                    role_not_confirmed.push(Self::album_vm(album, Some(role_line)));
                }
                SectionBucket::ListedOnly => {
                    listed_only.push(Self::album_vm(album, None));
                }
            }
        }

        let mut sections = Vec::new();
        if !albums_and_eps.is_empty() {
            sections.push(PublisherPageSectionVm {
                heading: Self::ALBUMS_AND_EPS_HEADING.to_owned(),
                rows: albums_and_eps,
            });
        }
        if !singles.is_empty() {
            sections.push(PublisherPageSectionVm {
                heading: Self::SINGLES_HEADING.to_owned(),
                rows: singles,
            });
        }
        for (heading, rows) in as_roles {
            sections.push(PublisherPageSectionVm { heading, rows });
        }
        if !role_not_confirmed.is_empty() {
            sections.push(PublisherPageSectionVm {
                heading: Self::ROLE_NOT_CONFIRMED_HEADING.to_owned(),
                rows: role_not_confirmed,
            });
        }
        if !listed_only.is_empty() {
            sections.push(PublisherPageSectionVm {
                heading: self.listed_only_heading(),
                rows: listed_only,
            });
        }
        sections
    }

    /// The heading "Listed by <feed title> only" (ADR 0085 Decision 3).
    /// With no title, it is "Listed by this feed only".
    fn listed_only_heading(&self) -> String {
        format!(
            "Listed by {} only",
            match self.title() {
                TitleDisplay::Stated(title) => title,
                TitleDisplay::Missing(_) => Self::LISTED_ONLY_FALLBACK_NAME.to_owned(),
            }
        )
    }

    /// The section a row belongs to (ADR 0085 Decision 3).
    fn section_bucket(album: &PublisherPageAlbumFact) -> SectionBucket {
        if album.album_names_as != Some(AlbumNamesAs::Publisher) {
            return SectionBucket::ListedOnly;
        }
        match &album.role_agreement {
            None => SectionBucket::Discography,
            Some(RoleAgreement::Both) => {
                if role_contains_artist(album.role.as_deref()) {
                    SectionBucket::Discography
                } else {
                    SectionBucket::AsRoles
                }
            }
            Some(RoleAgreement::OneSide | RoleAgreement::Conflict | RoleAgreement::Unknown(_)) => {
                SectionBucket::RoleNotConfirmed
            }
        }
    }

    fn album_vm(album: &PublisherPageAlbumFact, role_line: Option<String>) -> PublisherPageAlbumVm {
        let guid = album.feed_guid.clone().unwrap_or_default();
        let title = TitleDisplay::from_stated(album.title.as_deref(), &guid);
        let artist = album.artist.clone().map(|name| AlbumArtistDisplay {
            name,
            source: album.artist_source.clone(),
        });
        let not_listed = album.music_names_publisher == Some(true)
            && (album.publisher_lists_music == Some(false)
                || matches!(
                    album.publisher_link_resolution,
                    Some(PublisherLinkResolution::Unresolved)
                ));
        let open = album
            .feed_guid
            .clone()
            .map(|feed_guid| ReleaseRowOpenAction {
                key: ReleaseKey::from_index_feed_guid(feed_guid),
                a11y_label: format!("Open the release {}", title.text()),
            });
        PublisherPageAlbumVm {
            feed_guid: album.feed_guid.clone(),
            title,
            image_url: album.image_url.clone(),
            artist,
            not_listed,
            in_library: album.in_library,
            role_line,
            open,
        }
    }

    /// R3-11, R7-05 (ADR 0077 Task 007): the confirmed artist count.
    /// `None` when the response gives no confirmed count.
    #[must_use]
    pub(crate) fn confirmed_artist_count(&self) -> Option<DerivedArtistCount> {
        self.facts
            .confirmed_release_artist_count
            .map(|count| DerivedArtistCount {
                count,
                names: self.facts.confirmed_release_artists.clone(),
            })
    }

    /// R7-05 (ADR 0077 Task 007): the unconfirmed artist count. `None`
    /// when the response gives no unconfirmed count.
    #[must_use]
    pub(crate) fn unconfirmed_artist_count(&self) -> Option<DerivedArtistCount> {
        self.facts
            .unconfirmed_release_artist_count
            .map(|count| DerivedArtistCount {
                count,
                names: self.facts.unconfirmed_release_artists.clone(),
            })
    }

    /// R3-02a: the state of the albums that are not in the Library.
    #[must_use]
    pub(crate) fn other_albums_status(&self) -> OtherAlbumsStatus {
        match &self.facts.other_albums_failure {
            None => OtherAlbumsStatus::Loaded,
            Some(detail) => OtherAlbumsStatus::Unavailable {
                report: OtherAlbumsStatus::UNAVAILABLE_REPORT,
                detail: detail.clone(),
            },
        }
    }

    /// Every distinct album artwork URL on this page, in every section
    /// (ADR 0077 packet 004, orchestrator fix 5). The app layer resolves
    /// each one through the shared thumbnail path, and passes the resolved
    /// map to the screen. The screen fetches no image of its own.
    #[must_use]
    pub(crate) fn album_image_urls(&self) -> Vec<String> {
        let mut urls: Vec<String> = self
            .facts
            .albums
            .iter()
            .filter_map(|album| album.image_url.clone())
            .collect();
        urls.sort_unstable();
        urls.dedup();
        urls
    }

    /// "Shares albums with" (ADR 0082 Decision 5): one entry for each
    /// `co_credited_feeds` item, with its title, its roles, its album
    /// count, and the GUID of the feed page it opens. An entry with no
    /// feed GUID has no page to open, so this drops it.
    #[must_use]
    pub(crate) fn shares_albums_with(&self) -> Vec<CoCreditedFeedVm> {
        self.facts
            .co_credited_feeds
            .iter()
            .filter_map(Self::co_credited_feed_vm)
            .collect()
    }

    fn co_credited_feed_vm(entry: &CoCreditedFeedFact) -> Option<CoCreditedFeedVm> {
        let guid = entry.feed_guid.clone()?;
        let title = TitleDisplay::from_stated(entry.title.as_deref(), &guid);
        let title_text = title.text().to_owned();
        Some(CoCreditedFeedVm {
            title,
            roles: entry.roles.clone(),
            album_count: entry.album_count,
            publisher_feed_guid: guid,
            a11y_label: format!("Open the feed page of {title_text}"),
        })
    }
}

/// The section a row belongs to (ADR 0085 Decision 3), before it becomes a
/// [`PublisherPageSectionVm`]. Kept separate from [`PublisherPageAlbumVm`]
/// so a change to one row's shape never changes how a row is sorted into
/// its section.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SectionBucket {
    /// "Albums and EPs" or "Singles", split by kind.
    Discography,
    /// "As <roles>".
    AsRoles,
    /// "Role not confirmed".
    RoleNotConfirmed,
    /// "Listed by <feed title> only".
    ListedOnly,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A named feed link: the album names this feed, with the given GUID,
    /// `album_names_as` and `role_agreement`.
    fn album_with_guid(
        guid: &str,
        album_names_as: Option<AlbumNamesAs>,
        role_agreement: Option<RoleAgreement>,
        role: Option<&str>,
    ) -> PublisherPageAlbumFact {
        PublisherPageAlbumFact {
            feed_guid: Some(guid.to_owned()),
            title: Some(format!("Album {guid}")),
            music_names_publisher: Some(true),
            album_names_as,
            role_agreement,
            role: role.map(str::to_owned),
            ..PublisherPageAlbumFact::default()
        }
    }

    /// A single owned album, GUID `album-guid`, with no stated section or
    /// role fact. Tests that do not exercise the sections use this.
    fn owned_album() -> PublisherPageAlbumFact {
        album_with_guid("album-guid", None, None, None)
    }

    fn facts(albums: Vec<PublisherPageAlbumFact>) -> PublisherPageFacts {
        PublisherPageFacts {
            publisher_feed_guid: "publisher-guid".into(),
            feed_title: Some("Feed Title".into()),
            albums,
            ..PublisherPageFacts::default()
        }
    }

    /// Every album row in `sections`, flattened across every section, in
    /// the order the sections give them.
    fn flatten(sections: &[PublisherPageSectionVm]) -> Vec<PublisherPageAlbumVm> {
        sections
            .iter()
            .flat_map(|section| section.rows.iter().cloned())
            .collect()
    }

    /// R85-8-01: `level()` gives `Artist` for a row with `both` and
    /// `artist, producer`. A one-side `artist` row with rank 1 gives
    /// `UnverifiedArtist`. Only agreed `label` rows give `None`.
    #[test]
    fn adr_0085_feed_page_level_r85_8_01() {
        let mut agreed_artist = album_with_guid(
            "a1",
            Some(AlbumNamesAs::Publisher),
            Some(RoleAgreement::Both),
            Some("artist, producer"),
        );
        agreed_artist.two_way_validated = Some(true);
        let vm = PublisherPageVm::new(facts(vec![agreed_artist]));
        assert_eq!(vm.level(), FeedLevel::Artist);

        let mut one_side_artist_rank_one = album_with_guid(
            "a2",
            Some(AlbumNamesAs::Publisher),
            Some(RoleAgreement::OneSide),
            Some("artist"),
        );
        one_side_artist_rank_one.two_way_validated = Some(true);
        one_side_artist_rank_one.named_rank = Some(1);
        let vm = PublisherPageVm::new(facts(vec![one_side_artist_rank_one]));
        assert_eq!(vm.level(), FeedLevel::UnverifiedArtist);

        let agreed_label = album_with_guid(
            "a3",
            Some(AlbumNamesAs::Publisher),
            Some(RoleAgreement::Both),
            Some("label"),
        );
        let vm = PublisherPageVm::new(facts(vec![agreed_label]));
        assert_eq!(vm.level(), FeedLevel::None);
    }

    /// R85-8-02: `level()` gives `None` for a feed with rank 2 and no
    /// agreed row. A row with a null rank, and a rank 1 row that is not
    /// confirmed, also give `None`.
    #[test]
    fn adr_0085_feed_page_level_r85_8_02() {
        let mut rank_two = album_with_guid("a1", Some(AlbumNamesAs::Publisher), None, None);
        rank_two.two_way_validated = Some(true);
        rank_two.named_rank = Some(2);
        assert_eq!(
            PublisherPageVm::new(facts(vec![rank_two])).level(),
            FeedLevel::None
        );

        let mut null_rank = album_with_guid("a2", Some(AlbumNamesAs::Publisher), None, None);
        null_rank.two_way_validated = Some(true);
        assert_eq!(
            PublisherPageVm::new(facts(vec![null_rank])).level(),
            FeedLevel::None
        );

        let mut rank_one_unconfirmed =
            album_with_guid("a3", Some(AlbumNamesAs::Publisher), None, None);
        rank_one_unconfirmed.two_way_validated = Some(false);
        rank_one_unconfirmed.named_rank = Some(1);
        assert_eq!(
            PublisherPageVm::new(facts(vec![rank_one_unconfirmed])).level(),
            FeedLevel::None
        );
    }

    /// R85-8-03: the Sir Libre Records facts of 2026-10-18 give "As
    /// label", "Role not confirmed" and "Listed by Sir Libre Records
    /// only", in that sequence, with 3, 3 and 1 rows. No level fact.
    #[test]
    fn adr_0085_feed_page_sir_libre_records_sections_r85_8_03() {
        let mut albums = Vec::new();
        for index in 0..3 {
            albums.push(album_with_guid(
                &format!("agreed-{index}"),
                Some(AlbumNamesAs::Publisher),
                Some(RoleAgreement::Both),
                Some("label"),
            ));
        }
        for index in 0..3 {
            let mut conflict = album_with_guid(
                &format!("conflict-{index}"),
                Some(AlbumNamesAs::Publisher),
                Some(RoleAgreement::Conflict),
                None,
            );
            conflict.publisher_rel = Some("label".into());
            conflict.music_rel = Some("recordLabel".into());
            albums.push(conflict);
        }
        albums.push(album_with_guid("listed-only", None, None, None));

        let mut page_facts = facts(albums);
        page_facts.feed_title = Some("Sir Libre Records".into());
        let vm = PublisherPageVm::new(page_facts);

        let sections = vm.sections();
        assert_eq!(
            sections
                .iter()
                .map(|section| section.heading.clone())
                .collect::<Vec<_>>(),
            vec![
                "As label".to_owned(),
                "Role not confirmed".to_owned(),
                "Listed by Sir Libre Records only".to_owned(),
            ]
        );
        assert_eq!(sections[0].rows.len(), 3);
        assert_eq!(sections[1].rows.len(), 3);
        assert_eq!(sections[2].rows.len(), 1);
        assert!(
            sections[1].rows.iter().all(|row| row.role_line.is_some()),
            "each Role not confirmed row exposes a detail line"
        );
        assert!(vm
            .header_facts()
            .iter()
            .all(|fact| fact.label != PublisherPageVm::LEVEL_LABEL));
    }

    /// R85-8-04: agreed sets `label` and `label, producer` give two
    /// sections, "As label" and "As label and producer". `label` and
    /// `recordLabel` give two sections.
    #[test]
    fn adr_0085_feed_page_as_roles_split_by_role_set_r85_8_04() {
        let label = album_with_guid(
            "a1",
            Some(AlbumNamesAs::Publisher),
            Some(RoleAgreement::Both),
            Some("label"),
        );
        let label_producer = album_with_guid(
            "a2",
            Some(AlbumNamesAs::Publisher),
            Some(RoleAgreement::Both),
            Some("label, producer"),
        );
        let vm = PublisherPageVm::new(facts(vec![label, label_producer]));
        let headings: Vec<String> = vm
            .sections()
            .into_iter()
            .map(|section| section.heading)
            .collect();
        assert_eq!(
            headings,
            vec!["As label".to_owned(), "As label and producer".to_owned()]
        );

        let record_label = album_with_guid(
            "a3",
            Some(AlbumNamesAs::Publisher),
            Some(RoleAgreement::Both),
            Some("recordLabel"),
        );
        let label_again = album_with_guid(
            "a4",
            Some(AlbumNamesAs::Publisher),
            Some(RoleAgreement::Both),
            Some("label"),
        );
        let vm = PublisherPageVm::new(facts(vec![record_label, label_again]));
        let headings: Vec<String> = vm
            .sections()
            .into_iter()
            .map(|section| section.heading)
            .collect();
        assert_eq!(
            headings.len(),
            2,
            "label and recordLabel are different words"
        );
    }

    /// R85-8-05: a discography with two albums and one single gives
    /// "Albums and EPs" with 2 rows, then "Singles" with 1 row. An Index
    /// row with `remote_episode_count` 1 is a Single, and a null count
    /// gives an Album.
    #[test]
    fn adr_0085_feed_page_discography_kind_split_r85_8_05() {
        let mut album_one = album_with_guid("album-1", Some(AlbumNamesAs::Publisher), None, None);
        album_one.track_count = Some(8);
        let mut album_two = album_with_guid("album-2", Some(AlbumNamesAs::Publisher), None, None);
        album_two.track_count = None;
        let mut single = album_with_guid("single-1", Some(AlbumNamesAs::Publisher), None, None);
        single.track_count = Some(1);

        let vm = PublisherPageVm::new(facts(vec![album_one, album_two, single]));
        let sections = vm.sections();

        assert_eq!(sections[0].heading, PublisherPageVm::ALBUMS_AND_EPS_HEADING);
        assert_eq!(sections[0].rows.len(), 2);
        assert_eq!(sections[1].heading, PublisherPageVm::SINGLES_HEADING);
        assert_eq!(sections[1].rows.len(), 1);
        assert_eq!(sections[1].rows[0].feed_guid.as_deref(), Some("single-1"));
    }

    /// R85-8-06: a "Role not confirmed" row exposes the `stated_sides_text`
    /// detail. A row in each other section exposes no role line.
    #[test]
    fn adr_0085_feed_page_role_line_only_on_role_not_confirmed_r85_8_06() {
        let mut one_side = album_with_guid(
            "one-side",
            Some(AlbumNamesAs::Publisher),
            Some(RoleAgreement::OneSide),
            None,
        );
        one_side.music_rel = Some("label".into());
        let discography = album_with_guid("disc", Some(AlbumNamesAs::Publisher), None, None);

        let mut page_facts = facts(vec![one_side, discography]);
        page_facts.feed_title = Some("Sir Libre Records".into());
        let vm = PublisherPageVm::new(page_facts);
        let sections = vm.sections();

        let role_not_confirmed = sections
            .iter()
            .find(|section| section.heading == PublisherPageVm::ROLE_NOT_CONFIRMED_HEADING)
            .expect("a one-side row must give a Role not confirmed section");
        assert_eq!(
            role_not_confirmed.rows[0].role_line.as_deref(),
            Some("The album: label \u{b7} Sir Libre Records: no role")
        );
        let discography_section = sections
            .iter()
            .find(|section| section.heading == PublisherPageVm::ALBUMS_AND_EPS_HEADING)
            .expect("a null role_agreement row joins the discography");
        assert_eq!(discography_section.rows[0].role_line, None);
    }

    /// R85-8-07: an empty section is absent, and each release appears one
    /// time.
    #[test]
    fn adr_0085_feed_page_empty_section_absent_and_once_r85_8_07() {
        let agreed = album_with_guid(
            "a1",
            Some(AlbumNamesAs::Publisher),
            Some(RoleAgreement::Both),
            Some("artist"),
        );
        let vm = PublisherPageVm::new(facts(vec![agreed]));
        let sections = vm.sections();
        assert_eq!(sections.len(), 1, "every other section is empty, so absent");
        assert_eq!(flatten(&sections).len(), 1, "the album appears one time");
    }

    /// R85-8-08: the Library context and the Index context give the same
    /// sections for the same facts. A release in the Library list and the
    /// other list appears one time, with "In Library".
    #[test]
    fn adr_0085_feed_page_same_sections_both_contexts_r85_8_08() {
        let mut in_library = album_with_guid("local", Some(AlbumNamesAs::Publisher), None, None);
        in_library.in_library = true;
        let not_in_library = album_with_guid("remote", Some(AlbumNamesAs::Publisher), None, None);

        let vm = PublisherPageVm::new(facts(vec![in_library, not_in_library]));
        let rows = flatten(&vm.sections());
        assert_eq!(rows.len(), 2, "the release appears one time in each list");
        let local_row = rows
            .iter()
            .find(|row| row.feed_guid.as_deref() == Some("local"))
            .expect("the local row stays");
        assert!(local_row.in_library);
        let remote_row = rows
            .iter()
            .find(|row| row.feed_guid.as_deref() == Some("remote"))
            .expect("the remote row stays");
        assert!(!remote_row.in_library);
    }

    /// R85-8-09: a MusicIndex failure keeps the Library rows in their
    /// sections and gives the report.
    #[test]
    fn adr_0085_feed_page_failure_keeps_library_rows_r85_8_09() {
        let mut album = owned_album();
        album.in_library = true;
        let mut page_facts = facts(vec![album]);
        page_facts.other_albums_failure = Some("HTTP 503 from /v1/feeds/p".into());
        let vm = PublisherPageVm::new(page_facts);

        assert_eq!(flatten(&vm.sections()).len(), 1, "the Library row stays");
        assert_eq!(
            vm.other_albums_status(),
            OtherAlbumsStatus::Unavailable {
                report: OtherAlbumsStatus::UNAVAILABLE_REPORT,
                detail: "HTTP 503 from /v1/feeds/p".into(),
            },
            "the report comes first, and the transport detail follows it"
        );
    }

    /// R85-8-14: `frame_title()` gives "Artist" for the HeyCitizen and
    /// DETOX facts, and "Label" for the Sir Libre Records facts. A feed
    /// with rank 2 rows and no stated role gives "Publisher". A null rank
    /// gives "Feed", and so does a rank 2 row with a one-side `artist`
    /// role.
    #[test]
    fn adr_0085_feed_page_frame_title_r85_8_14() {
        let mut hey_citizen = album_with_guid(
            "a1",
            Some(AlbumNamesAs::Publisher),
            Some(RoleAgreement::Both),
            Some("artist"),
        );
        hey_citizen.two_way_validated = Some(true);
        assert_eq!(
            PublisherPageVm::new(facts(vec![hey_citizen])).frame_title(),
            "Artist"
        );

        let mut detox = album_with_guid("a2", Some(AlbumNamesAs::Publisher), None, None);
        detox.two_way_validated = Some(true);
        detox.named_rank = Some(1);
        assert_eq!(
            PublisherPageVm::new(facts(vec![detox])).frame_title(),
            "Artist"
        );

        let mut page_facts = facts(vec![album_with_guid(
            "a3",
            Some(AlbumNamesAs::Publisher),
            Some(RoleAgreement::Both),
            Some("label"),
        )]);
        page_facts.feed_title = Some("Sir Libre Records".into());
        assert_eq!(PublisherPageVm::new(page_facts).frame_title(), "Label");

        let mut rank_two = album_with_guid("a4", Some(AlbumNamesAs::Publisher), None, None);
        rank_two.named_rank = Some(2);
        assert_eq!(
            PublisherPageVm::new(facts(vec![rank_two])).frame_title(),
            "Publisher"
        );

        let mut null_rank = album_with_guid("a5", Some(AlbumNamesAs::Publisher), None, None);
        null_rank.named_rank = None;
        assert_eq!(
            PublisherPageVm::new(facts(vec![null_rank])).frame_title(),
            "Feed"
        );

        let mut one_side_artist_rank_two = album_with_guid(
            "a6",
            Some(AlbumNamesAs::Publisher),
            Some(RoleAgreement::OneSide),
            None,
        );
        one_side_artist_rank_two.publisher_rel = Some("artist".into());
        one_side_artist_rank_two.named_rank = Some(2);
        assert_eq!(
            PublisherPageVm::new(facts(vec![one_side_artist_rank_two])).frame_title(),
            "Feed"
        );
    }

    /// R85-8-15: each release row with a feed GUID exposes an available
    /// open action with the release key of that GUID and an accessibility
    /// label. A row with no feed GUID exposes an unavailable action.
    #[test]
    fn adr_0085_feed_page_open_action_r85_8_15() {
        let with_guid = album_with_guid("album-1", Some(AlbumNamesAs::Publisher), None, None);
        let mut without_guid =
            album_with_guid("no-guid", Some(AlbumNamesAs::Publisher), None, None);
        without_guid.feed_guid = None;
        let vm = PublisherPageVm::new(facts(vec![with_guid, without_guid]));
        let rows = flatten(&vm.sections());

        let available = rows
            .iter()
            .find(|row| row.feed_guid.as_deref() == Some("album-1"))
            .expect("the row with a GUID stays");
        let open = available
            .open
            .as_ref()
            .expect("the action must be available");
        assert_eq!(open.key, ReleaseKey::from_index_feed_guid("album-1"));
        assert_eq!(open.a11y_label, "Open the release Album album-1");

        let unavailable = rows
            .iter()
            .find(|row| row.feed_guid.is_none())
            .expect("the row with no GUID stays");
        assert!(
            unavailable.open.is_none(),
            "a row with no feed GUID has an unavailable action"
        );
    }

    /// R85-8-16: the Index facts carry `image_url` of the feed response as
    /// `feed_image_url`, and the view model exposes it for the header.
    #[test]
    fn adr_0085_feed_page_header_image_url_r85_8_16() {
        let mut page_facts = facts(vec![]);
        page_facts.feed_image_url = Some("https://example.test/feed.jpg".into());
        let vm = PublisherPageVm::new(page_facts);
        assert_eq!(
            vm.header_image_url(),
            Some("https://example.test/feed.jpg".to_owned())
        );

        let vm = PublisherPageVm::new(facts(vec![]));
        assert_eq!(vm.header_image_url(), None);
    }

    /// R3-09: an owned album with `publisher_lists_music = false` or an
    /// unresolved link gets the "Not listed" mark.
    #[test]
    fn adr_0077_publisher_page_not_listed_mark_on_owned_albums() {
        let mut not_listed = owned_album();
        not_listed.publisher_lists_music = Some(false);
        let mut unresolved = album_with_guid("album-guid", None, None, None);
        unresolved.publisher_link_resolution = Some(PublisherLinkResolution::Unresolved);
        let mut listed = album_with_guid("album-guid", None, None, None);
        listed.publisher_lists_music = Some(true);
        let vm = PublisherPageVm::new(facts(vec![not_listed, unresolved, listed]));

        let albums = flatten(&vm.sections());
        assert!(albums[0].not_listed);
        assert!(albums[1].not_listed);
        assert!(!albums[2].not_listed);
    }

    /// R3-10: a feed without a title exposes its GUID and "No title".
    #[test]
    fn adr_0077_publisher_page_missing_title_exposes_guid_and_no_title_label() {
        let mut page_facts = facts(vec![]);
        page_facts.feed_title = None;
        let vm = PublisherPageVm::new(page_facts);
        assert_eq!(
            vm.title(),
            TitleDisplay::Missing("publisher-guid".to_owned())
        );
        assert_eq!(TitleDisplay::MISSING_LABEL, "No title");
    }

    /// R3-14: an album entry with a null `remote_feed_title` exposes its
    /// GUID and "No title". A null `remote_release_artist` exposes no
    /// artist value.
    #[test]
    fn adr_0077_publisher_page_null_album_title_and_artist_expose_no_placeholder() {
        let mut album = owned_album();
        album.title = None;
        album.artist = None;
        let vm = PublisherPageVm::new(facts(vec![album]));

        let albums = flatten(&vm.sections());
        assert_eq!(albums[0].title, TitleDisplay::Missing("album-guid".into()));
        assert_eq!(albums[0].artist, None);
    }

    /// R4-04: the page title text is the view model title, from the feed's
    /// own `<title>`.
    #[test]
    fn adr_0077_publisher_navigation_title_text_uses_stated_title() {
        let vm = PublisherPageVm::new(facts(vec![]));
        assert_eq!(vm.title_text(), "Feed Title");
    }

    /// R4-04: a feed without a title shows its GUID as its title text, the
    /// same value `title` gives for the page.
    #[test]
    fn adr_0077_publisher_navigation_title_text_falls_back_to_guid() {
        let mut page_facts = facts(vec![]);
        page_facts.feed_title = None;
        let vm = PublisherPageVm::new(page_facts);
        assert_eq!(vm.title_text(), "publisher-guid");
    }

    /// Packet 004: a feed without a title adds a "Title" row naming the
    /// missing-title label. The screen writes no such text.
    #[test]
    fn adr_0077_publisher_navigation_header_facts_name_a_missing_title() {
        let mut page_facts = facts(vec![]);
        page_facts.feed_title = None;
        let vm = PublisherPageVm::new(page_facts);

        let header_facts = vm.header_facts();

        let title_row = header_facts
            .iter()
            .find(|fact| fact.label == PublisherPageVm::TITLE_LABEL)
            .expect("a missing title adds a Title row");
        assert_eq!(title_row.value, TitleDisplay::MISSING_LABEL);
    }

    /// Packet 004: the confirmed artist count header fact names the count
    /// as derived, and lists the artist names.
    #[test]
    fn adr_0077_publisher_navigation_header_facts_name_the_count_as_derived() {
        let mut page_facts = facts(vec![owned_album()]);
        page_facts.confirmed_release_artist_count = Some(2);
        page_facts.confirmed_release_artists = vec!["A".into(), "B".into()];
        let vm = PublisherPageVm::new(page_facts);

        let header_facts = vm.header_facts();

        let artists_row = header_facts
            .iter()
            .find(|fact| fact.label == PublisherPageVm::CONFIRMED_ARTISTS_LABEL)
            .expect("a confirmed count adds an artist row");
        assert!(artists_row.value.contains('2'));
        assert!(artists_row.value.contains(DerivedArtistCount::LABEL));
        assert!(artists_row.value.contains("A, B"));
    }

    /// Packet 004, R4-05: an album artist's display text names its stored
    /// owner as supporting text. A missing source shows the name alone.
    #[test]
    fn adr_0077_publisher_navigation_album_artist_display_text_names_its_source() {
        let with_source = AlbumArtistDisplay {
            name: "Stored Artist".into(),
            source: Some("channel".into()),
        };
        assert_eq!(with_source.display_text(), "Stored Artist (channel)");

        let without_source = AlbumArtistDisplay {
            name: "Stored Artist".into(),
            source: None,
        };
        assert_eq!(without_source.display_text(), "Stored Artist");
    }

    /// Packet 004: a `Failed` load display gives a report first, and its
    /// technical detail separately, the same order as `OtherAlbumsStatus`.
    #[test]
    fn adr_0077_publisher_navigation_load_display_failed_separates_report_and_detail() {
        let display = PublisherPageLoadDisplay::failed("HTTP 503 from /v1/feeds/p");

        assert_eq!(
            display,
            PublisherPageLoadDisplay::Failed {
                report: PublisherPageLoadDisplay::FAILED_REPORT,
                detail: "HTTP 503 from /v1/feeds/p".into(),
            },
            "the report comes first, and the transport detail follows it, apart from it"
        );
    }

    /// Packet 004: the loading and empty displays give fixed, view-model
    /// owned text. The app layer builds neither string.
    #[test]
    fn adr_0077_publisher_navigation_load_display_loading_and_empty_text() {
        let loading = PublisherPageLoadDisplay::loading(PublisherPageContext::Library, "p-guid");
        assert_eq!(
            loading,
            PublisherPageLoadDisplay::Loading {
                message: "Loading the Library feed page p-guid...".into(),
            }
        );

        assert_eq!(
            PublisherPageLoadDisplay::empty(),
            PublisherPageLoadDisplay::Empty {
                message: PublisherPageLoadDisplay::EMPTY_MESSAGE.to_owned(),
            }
        );
    }

    /// Orchestrator fix 5: the view model gathers every album's artwork URL,
    /// once each, so the app layer can resolve every image through the
    /// shared thumbnail path. An album with no artwork URL contributes none.
    #[test]
    fn adr_0077_publisher_navigation_album_image_urls_are_gathered_once_each() {
        let mut with_art = owned_album();
        with_art.image_url = Some("https://example.test/a.png".into());
        let mut duplicate_art = owned_album();
        duplicate_art.image_url = Some("https://example.test/a.png".into());
        let mut no_art = owned_album();
        no_art.image_url = None;

        let vm = PublisherPageVm::new(facts(vec![with_art, duplicate_art, no_art]));

        assert_eq!(
            vm.album_image_urls(),
            vec!["https://example.test/a.png".to_string()]
        );
    }

    /// R82-2-07: a `co_credited_feeds` entry gives a "Shares albums with"
    /// row with its title, its roles, its album count, and an action that
    /// opens its feed page, with no "publisher" text (ADR 0085 Decision
    /// 1).
    #[test]
    fn adr_0082_role_groups_shares_albums_with_entry_gives_title_roles_count_and_action() {
        let mut page_facts = facts(vec![]);
        page_facts.co_credited_feeds = vec![CoCreditedFeedFact {
            feed_guid: Some("other-feed".into()),
            title: Some("Other Label".into()),
            roles: vec!["artist".into(), "label".into()],
            album_count: Some(3),
        }];
        let vm = PublisherPageVm::new(page_facts);

        let entries = vm.shares_albums_with();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, TitleDisplay::Stated("Other Label".into()));
        assert_eq!(
            entries[0].roles,
            vec!["artist".to_owned(), "label".to_owned()]
        );
        assert_eq!(entries[0].album_count, Some(3));
        assert_eq!(entries[0].publisher_feed_guid, "other-feed");
        assert!(entries[0].a11y_label.contains("Other Label"));
        assert!(
            !entries[0].a11y_label.to_lowercase().contains("publisher"),
            "ADR 0085 Decision 1: the a11y label must not name the feed publisher"
        );
        assert_eq!(
            entries[0].roles_and_count_text(),
            "artist, label \u{2014} 3 shared albums"
        );
    }

    /// Orchestrator review: the view model must not invent a count. When
    /// the response states no `album_count`, the entry carries `None`, and
    /// its text shows the roles alone, with no count.
    #[test]
    fn adr_0082_role_groups_missing_count_shows_no_count() {
        let mut page_facts = facts(vec![]);
        page_facts.co_credited_feeds = vec![CoCreditedFeedFact {
            feed_guid: Some("other-feed".into()),
            title: Some("Other Label".into()),
            roles: vec!["artist".into()],
            album_count: None,
        }];
        let vm = PublisherPageVm::new(page_facts);

        let entries = vm.shares_albums_with();

        assert_eq!(
            entries[0].album_count, None,
            "the view model invents no count"
        );
        assert_eq!(
            entries[0].roles_and_count_text(),
            "artist",
            "an absent count shows the roles alone"
        );
    }

    /// R82-2-07: the section is absent (the view model gives an empty
    /// list) when `co_credited_feeds` is empty.
    #[test]
    fn adr_0082_role_groups_shares_albums_with_is_empty_without_co_credited_feeds() {
        let vm = PublisherPageVm::new(facts(vec![]));
        assert!(vm.shares_albums_with().is_empty());
    }

    /// An entry with no feed GUID has no page to open, so it is dropped.
    #[test]
    fn adr_0082_role_groups_shares_albums_with_drops_an_entry_with_no_feed_guid() {
        let mut page_facts = facts(vec![]);
        page_facts.co_credited_feeds = vec![CoCreditedFeedFact {
            feed_guid: None,
            title: Some("No GUID".into()),
            roles: vec![],
            album_count: Some(1),
        }];
        let vm = PublisherPageVm::new(page_facts);
        assert!(vm.shares_albums_with().is_empty());
    }

    /// R7-02 (ADR 0077 Task 007, Stophammer ADR 0061): confirmed 1 and
    /// unconfirmed 0 give two header facts, with the confirmed name and
    /// both counts.
    #[test]
    fn adr_0077_confirmed_artists_two_facts_show_confirmed_and_unconfirmed_counts() {
        let mut page_facts = facts(vec![]);
        page_facts.confirmed_release_artist_count = Some(1);
        page_facts.confirmed_release_artists = vec!["Official DETOX Music".into()];
        page_facts.unconfirmed_release_artist_count = Some(0);
        let vm = PublisherPageVm::new(page_facts);

        let header_facts = vm.header_facts();
        let confirmed = header_facts
            .iter()
            .find(|fact| fact.label == PublisherPageVm::CONFIRMED_ARTISTS_LABEL)
            .expect("a confirmed count adds a header fact");
        assert!(confirmed.value.contains('1'));
        assert!(confirmed.value.contains("Official DETOX Music"));

        let unconfirmed = header_facts
            .iter()
            .find(|fact| fact.label == PublisherPageVm::UNCONFIRMED_ARTISTS_LABEL)
            .expect("an unconfirmed count of 0 still adds a header fact");
        assert!(unconfirmed.value.contains('0'));
    }

    /// R7-03: confirmed 0 and unconfirmed 33 give two header facts with
    /// those counts, and the unconfirmed fact lists the unconfirmed names.
    #[test]
    fn adr_0077_confirmed_artists_unconfirmed_names_show_when_confirmed_is_zero() {
        let mut page_facts = facts(vec![]);
        page_facts.confirmed_release_artist_count = Some(0);
        page_facts.unconfirmed_release_artist_count = Some(33);
        page_facts.unconfirmed_release_artists = vec!["Artist One".into(), "Artist Two".into()];
        let vm = PublisherPageVm::new(page_facts);

        let header_facts = vm.header_facts();
        let confirmed = header_facts
            .iter()
            .find(|fact| fact.label == PublisherPageVm::CONFIRMED_ARTISTS_LABEL)
            .expect("a confirmed count of 0 still adds a header fact");
        assert!(confirmed.value.contains('0'));

        let unconfirmed = header_facts
            .iter()
            .find(|fact| fact.label == PublisherPageVm::UNCONFIRMED_ARTISTS_LABEL)
            .expect("an unconfirmed count adds a header fact");
        assert!(unconfirmed.value.contains("33"));
        assert!(unconfirmed.value.contains("Artist One"));
        assert!(unconfirmed.value.contains("Artist Two"));
    }

    /// R7-04: facts with none of the four fields give no artist fact.
    /// `PublisherPageFacts` holds no other artist count, so no fallback can
    /// supply one.
    #[test]
    fn adr_0077_confirmed_artists_absent_fields_give_no_artist_fact() {
        let page_facts = facts(vec![owned_album()]);
        let vm = PublisherPageVm::new(page_facts);

        let header_facts = vm.header_facts();
        assert!(
            header_facts.iter().all(
                |fact| fact.label != PublisherPageVm::CONFIRMED_ARTISTS_LABEL
                    && fact.label != PublisherPageVm::UNCONFIRMED_ARTISTS_LABEL
            ),
            "an absent confirmed and unconfirmed count must add no artist fact"
        );
    }
}
