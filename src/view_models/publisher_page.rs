//! Publisher page view model (ADR 0077 Task 003, ADR 0082, ADR 0077 Task 007).
//!
//! The Index publisher page query and the Library publisher page query, in
//! `src/application/queries`, each build one [`PublisherPageFacts`] value
//! from their own data. This module turns that value into the album name
//! groups and role subgroups of ADR 0082 Decision 2, the row role text of
//! Decision 3, the header roles of Decision 4, the "Shares albums with"
//! entries of Decision 5, the title, and the confirmed and unconfirmed
//! artist counts. No renderer computes these facts. [`PublisherPageVm`]
//! computes each one, and packet 004 wires a screen to it.

#![warn(clippy::pedantic)]

use std::collections::{BTreeMap, BTreeSet};

use crate::api::{AlbumNamesAs, PublisherLinkResolution, RoleAgreement, RoleSource};

/// The route that opened a publisher page (ADR 0077 packet 004). This value
/// selects which `PublisherPageVm` groups the screen shows. It decides no
/// group, no role and no action availability: those come from
/// [`PublisherPageVm`] alone.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PublisherPageContext {
    /// Reached from the Library. The screen shows `library_groups` and
    /// `other_groups` as two scopes, each with the ADR 0082 Decision 2
    /// groups inside it.
    Library,
    /// Reached from the Index. The screen shows `index_groups`, and marks
    /// each album with `in_library`.
    Index,
}

impl PublisherPageContext {
    /// `true` when the screen marks each album row with `in_library`
    /// (Required Change 4: only the Index page shape does this).
    #[must_use]
    pub(crate) const fn marks_in_library(self) -> bool {
        matches!(self, Self::Index)
    }

    /// The route name for a status report (packet 004).
    #[must_use]
    pub(crate) const fn route_name(self) -> &'static str {
        match self {
            Self::Library => "Library",
            Self::Index => "Index",
        }
    }
}

/// One album row on a publisher page, gathered by the owning query.
///
/// On the Index publisher page, every fact comes from the `remote_*`
/// summary fields of one `publisher_to_music` entry (Stophammer ADR 0059).
/// On the Library publisher page, a Library album's fact set comes from the
/// stored local feed row instead, so `artist` and `artist_source` stay
/// `None` there: the app stores no `remote_release_artist` value.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PublisherPageAlbumFact {
    /// The album feed's own GUID.
    pub(crate) feed_guid: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) image_url: Option<String>,
    pub(crate) artist: Option<String>,
    pub(crate) artist_source: Option<String>,
    pub(crate) role: Option<String>,
    pub(crate) role_source: Option<RoleSource>,
    /// `true` when the album names this publisher (ADR 0077 Decision 2).
    pub(crate) music_names_publisher: Option<bool>,
    /// `true` when the publisher feed lists this album.
    pub(crate) publisher_lists_music: Option<bool>,
    pub(crate) publisher_link_resolution: Option<PublisherLinkResolution>,
    /// The raw `rel` the publisher feed item stated for this album.
    pub(crate) publisher_rel: Option<String>,
    /// The raw `rel` the album feed item stated for this publisher.
    pub(crate) music_rel: Option<String>,
    /// How this album names the publisher feed (ADR 0082 Decision 2).
    pub(crate) album_names_as: Option<AlbumNamesAs>,
    /// Whether the two feeds agree on `role` (ADR 0082 Decision 2).
    pub(crate) role_agreement: Option<RoleAgreement>,
    /// `true` when this album's own feed is in the Library.
    pub(crate) in_library: bool,
}

/// The raw facts one publisher page query gathers. [`PublisherPageVm`] turns
/// these into the page's display facts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PublisherPageFacts {
    /// The publisher feed's own GUID (ADR 0077 Decision 1). The page
    /// identity, and the fallback title text.
    pub(crate) publisher_feed_guid: String,
    /// The publisher feed's own `<title>`.
    pub(crate) feed_title: Option<String>,
    /// `confirmed_release_artist_count` of the publisher feed: the distinct
    /// artists of the albums that name this publisher and that this
    /// publisher also lists (Stophammer ADR 0061, ADR 0077 Task 007).
    /// `MusicIndex` derives it. The publisher page shows no type (ADR 0082
    /// Decision 1), and this count never selects one.
    pub(crate) confirmed_release_artist_count: Option<i64>,
    pub(crate) confirmed_release_artists: Vec<String>,
    /// `unconfirmed_release_artist_count` of the publisher feed: the
    /// distinct artists of the albums that this publisher lists but that do
    /// not name it (Stophammer ADR 0061, ADR 0077 Task 007). `MusicIndex`
    /// derives it.
    pub(crate) unconfirmed_release_artist_count: Option<i64>,
    pub(crate) unconfirmed_release_artists: Vec<String>,
    pub(crate) albums: Vec<PublisherPageAlbumFact>,
    /// Each other publisher feed that shares a confirmed album with this
    /// feed (ADR 0082 Decision 5). Empty when the response names none.
    pub(crate) co_credited_feeds: Vec<CoCreditedFeedFact>,
    /// R3-02a: the reason the request for the albums that are not in the
    /// Library failed. `None` when that request has not failed. The Library
    /// group in `albums` stays present either way.
    pub(crate) other_albums_failure: Option<String>,
}

/// One entry of "Shares albums with" (ADR 0082 Decision 5): another
/// publisher feed that shares a confirmed album with the feed this page
/// describes. The owning query builds one of these from each
/// `co_credited_feeds` item of the response.
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
}

/// The role and its source of one album/publisher link, ready to view (ADR
/// 0082 Decision 3). `PublisherPageVm::role_display` builds this from the
/// link's `role_agreement`, `role`, `role_source`, `publisher_rel` and
/// `music_rel` facts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AlbumRoleDisplay {
    /// Both feeds agree. `role` is the agreed role set.
    Agreed { role: String },
    /// One feed states a role; the other states none. `side` names which
    /// feed stated it: `PublisherRel` or `MusicRel`.
    OneSide { role: String, side: RoleSource },
    /// The two feeds state different role sets. Neither wins (ADR 0077
    /// Decision 3), and the row says that the link is not confirmed.
    Conflict {
        publisher_role: Option<String>,
        music_role: Option<String>,
    },
    /// No feed states a role.
    Unstated,
}

impl AlbumRoleDisplay {
    /// The source text that follows an agreed role.
    const AGREED_SOURCE: &'static str = "agreed by both feeds";
    /// The source text that follows a role the album feed alone stated.
    const ONE_SIDE_ALBUM_SOURCE: &'static str = "stated by the album only";
    /// The source text that follows a role the publisher feed alone
    /// stated.
    const ONE_SIDE_PUBLISHER_SOURCE: &'static str = "stated by the publisher only";
    /// The note on a conflict row: the link is not confirmed.
    const CONFLICT_NOTE: &'static str = "not confirmed";
    /// The word a conflict side shows when it states no role.
    const CONFLICT_UNSTATED_SIDE: &'static str = "no stated role";
    /// The label before the publisher feed's stated set, on a conflict row.
    const CONFLICT_PUBLISHER_LABEL: &'static str = "Publisher feed:";
    /// The label before the album feed's stated set, on a conflict row.
    const CONFLICT_MUSIC_LABEL: &'static str = "Album feed:";
    /// The text a row with no stated role shows (ADR 0082 Decision 2 and
    /// 3: the same text names the "Role not stated" subgroup).
    pub(crate) const UNSTATED_TEXT: &'static str = "Role not stated";

    /// `true` for an agreed or a one-side row: a row with a feed-stated
    /// role (ADR 0082 Decision 3).
    #[must_use]
    pub(crate) const fn is_stated(&self) -> bool {
        matches!(self, Self::Agreed { .. } | Self::OneSide { .. })
    }

    /// `true` for a conflict row (ADR 0082 Decision 3).
    #[must_use]
    pub(crate) const fn is_conflict(&self) -> bool {
        matches!(self, Self::Conflict { .. })
    }

    /// The role text and its source, ready to view (ADR 0082 Decision 3,
    /// Required Change 3). The screen composes no text of its own: this is
    /// the one row-role string it shows.
    #[must_use]
    pub(crate) fn display_text(&self) -> String {
        match self {
            Self::Agreed { role } => format!("{role} \u{2014} {}", Self::AGREED_SOURCE),
            Self::OneSide {
                role,
                side: RoleSource::MusicRel,
            } => format!("{role} \u{2014} {}", Self::ONE_SIDE_ALBUM_SOURCE),
            Self::OneSide {
                role,
                side: RoleSource::PublisherRel,
            } => format!("{role} \u{2014} {}", Self::ONE_SIDE_PUBLISHER_SOURCE),
            // `role_display` builds `OneSide` only with `PublisherRel` or
            // `MusicRel`. This arm stays only so the match is exhaustive
            // against a future `RoleSource` variant.
            Self::OneSide { role, .. } => role.clone(),
            Self::Conflict {
                publisher_role,
                music_role,
            } => format!(
                "{} {} / {} {} ({})",
                Self::CONFLICT_PUBLISHER_LABEL,
                publisher_role
                    .as_deref()
                    .unwrap_or(Self::CONFLICT_UNSTATED_SIDE),
                Self::CONFLICT_MUSIC_LABEL,
                music_role
                    .as_deref()
                    .unwrap_or(Self::CONFLICT_UNSTATED_SIDE),
                Self::CONFLICT_NOTE,
            ),
            Self::Unstated => Self::UNSTATED_TEXT.to_owned(),
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

/// One album row, ready to view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PublisherPageAlbumVm {
    pub(crate) feed_guid: Option<String>,
    pub(crate) title: TitleDisplay,
    pub(crate) image_url: Option<String>,
    pub(crate) artist: Option<AlbumArtistDisplay>,
    pub(crate) role: AlbumRoleDisplay,
    /// R3-09: an owned album with `publisher_lists_music = false` or an
    /// unresolved link.
    pub(crate) not_listed: bool,
    pub(crate) in_library: bool,
}

impl PublisherPageAlbumVm {
    /// The mark on an owned album that the publisher feed does not list
    /// (ADR 0077 Accepted Refinements, R3-09).
    pub(crate) const NOT_LISTED_LABEL: &'static str = "Not listed by the publisher";

    /// The mark on an Index album whose own feed is in the Library
    /// (packet 004, Required Change 4).
    pub(crate) const IN_LIBRARY_LABEL: &'static str = "In Library";
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
    /// The GUID of the publisher page this entry opens. The screen builds
    /// no action target of its own.
    pub(crate) publisher_feed_guid: String,
    /// The accessibility label of the open-publisher action.
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
    /// The report for a failed request for the other albums of a publisher.
    pub(crate) const UNAVAILABLE_REPORT: &'static str = "The app could not load the other albums of this publisher from MusicIndex. The albums in the Library stay on this page.";
}

/// Display state of the mounted publisher page while it loads, when its
/// fetch fails, or when none is open (ADR 0077 packet 004). The app layer
/// only selects which state applies; this module decides its text, the
/// same as `OtherAlbumsStatus`. A `Failed` state gives a report first, and
/// its technical detail after it, apart from the report (R3-02a). This
/// type carries no transport error: the app layer turns one into text
/// before it reaches here.
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
    /// The report for a publisher page that failed to load.
    pub(crate) const FAILED_REPORT: &'static str =
        "The app could not load this publisher page from MusicIndex.";

    /// The message shown when no publisher page is open.
    pub(crate) const EMPTY_MESSAGE: &'static str = "No publisher page is open.";

    /// The display for a publisher page in flight (packet 004).
    #[must_use]
    pub(crate) fn loading(context: PublisherPageContext, publisher_feed_guid: &str) -> Self {
        Self::Loading {
            message: format!(
                "Loading the {} publisher page {publisher_feed_guid}...",
                context.route_name()
            ),
        }
    }

    /// The display for a publisher page whose fetch failed. `detail` is
    /// the transport error, already turned into text by the app layer.
    #[must_use]
    pub(crate) fn failed(detail: impl Into<String>) -> Self {
        Self::Failed {
            report: Self::FAILED_REPORT,
            detail: detail.into(),
        }
    }

    /// The display for a frame with no mounted publisher page.
    #[must_use]
    pub(crate) fn empty() -> Self {
        Self::Empty {
            message: Self::EMPTY_MESSAGE.to_owned(),
        }
    }
}

/// The group of ADR 0082 Decision 2: how an album names this publisher
/// feed. A publisher page shows no type (Decision 1); it groups its
/// albums by this fact instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AlbumNameGroup {
    /// `album_names_as` is `publisher`.
    PublisherOf,
    /// `album_names_as` is null.
    ListedNotNamed,
}

impl AlbumNameGroup {
    /// The caption this group shows (ADR 0082 Decision 2). The screen
    /// decides no group label of its own.
    #[must_use]
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::PublisherOf => "Publisher of",
            Self::ListedNotNamed => "Listed, not named",
        }
    }
}

/// The caption of a role subgroup inside one [`AlbumNameGroup`] (ADR 0082
/// Decision 2). An agreed subgroup's caption is its own role set text; the
/// other three subgroups have a fixed caption.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RoleSubgroupCaption {
    /// `role_agreement` is `both`. One subgroup for each different role
    /// set.
    Agreed(String),
    /// `role_agreement` is `one_side`.
    OneSide,
    /// `role_agreement` is `conflict`.
    Conflict,
    /// `role_agreement` is null, or an unrecognized value.
    Unstated,
}

impl RoleSubgroupCaption {
    /// The caption this subgroup shows.
    const ONE_SIDE_LABEL: &'static str = "Role stated by one side";
    const CONFLICT_LABEL: &'static str = "Roles differ, not confirmed";

    #[must_use]
    fn label(&self) -> &str {
        match self {
            Self::Agreed(role_set) => role_set.as_str(),
            Self::OneSide => Self::ONE_SIDE_LABEL,
            Self::Conflict => Self::CONFLICT_LABEL,
            Self::Unstated => AlbumRoleDisplay::UNSTATED_TEXT,
        }
    }
}

/// One role subgroup, ready to view (ADR 0082 Decision 2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AlbumRoleSubgroupVm {
    pub(crate) caption: String,
    pub(crate) albums: Vec<PublisherPageAlbumVm>,
}

/// One album name group, with its role subgroups, ready to view (ADR 0082
/// Decision 2). An album appears in exactly one subgroup of exactly one
/// group. An empty group or subgroup is absent: `PublisherPageVm` never
/// builds one with no album.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AlbumNameGroupVm {
    pub(crate) group: AlbumNameGroup,
    pub(crate) subgroups: Vec<AlbumRoleSubgroupVm>,
}

/// The derived artist count of a publisher feed (ADR 0077 Task 007).
/// `MusicIndex` derives it from album credits.
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

/// One header fact of a publisher page, ready to view (packet 004). The
/// screen shows each fact as a label/value pair. It decides no label and
/// composes no text of its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PublisherPageHeaderFact {
    pub(crate) label: &'static str,
    pub(crate) value: String,
}

/// The publisher page view model (ADR 0077 Task 003).
pub(crate) struct PublisherPageVm {
    facts: PublisherPageFacts,
}

impl PublisherPageVm {
    /// Row label for the "No title" fact, shown only when the feed states
    /// no title.
    pub(crate) const TITLE_LABEL: &'static str = "Title";
    /// Row label for the agreed roles fact (ADR 0082 Decision 4).
    pub(crate) const ROLES_LABEL: &'static str = "Roles";
    /// Row label for the confirmed artist count fact (ADR 0077 Task 007):
    /// the artists of the albums that name this feed.
    pub(crate) const CONFIRMED_ARTISTS_LABEL: &'static str = "Artists that name this feed";
    /// Row label for the unconfirmed artist count fact (ADR 0077 Task 007):
    /// the artists of the albums that this feed lists without a link back.
    pub(crate) const UNCONFIRMED_ARTISTS_LABEL: &'static str =
        "Artists this feed lists without a link back";
    /// Caption of the Library page's own scope (ADR 0082 packet 002: the
    /// Library page keeps its present Library/Other scope, and groups
    /// Decision 2's album name groups inside each one).
    pub(crate) const LIBRARY_SCOPE_LABEL: &'static str = "Library Albums";
    /// Caption of the Library page's other scope.
    pub(crate) const OTHER_SCOPE_LABEL: &'static str = "Other Albums";
    /// Caption of the "Shares albums with" section (ADR 0082 Decision 5).
    pub(crate) const SHARES_ALBUMS_WITH_LABEL: &'static str = "Shares albums with";

    #[must_use]
    pub(crate) const fn new(facts: PublisherPageFacts) -> Self {
        Self { facts }
    }

    /// R3-10: the publisher feed's own title, or its GUID with the "No
    /// title" label.
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
        match self.title() {
            TitleDisplay::Stated(text) => text,
            TitleDisplay::Missing(guid) => guid,
        }
    }

    /// The header facts of this page, ready to view (ADR 0082 Decision 4).
    /// The screen shows each fact as a label/value pair; it decides no
    /// label and composes no text.
    #[must_use]
    pub(crate) fn header_facts(&self) -> Vec<PublisherPageHeaderFact> {
        let mut facts = Vec::new();
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
        let mut roles: BTreeSet<String> = BTreeSet::new();
        for album in &self.facts.albums {
            if album.role_agreement == Some(RoleAgreement::Both) {
                if let Some(role) = &album.role {
                    roles.extend(role.split(", ").map(str::to_owned));
                }
            }
        }
        roles.into_iter().collect()
    }

    /// The Index page's groups (ADR 0082 Decision 2): every album on this
    /// page, grouped by `album_names_as`, then by role subgroup.
    #[must_use]
    pub(crate) fn index_groups(&self) -> Vec<AlbumNameGroupVm> {
        Self::album_name_groups(&self.facts.albums)
    }

    /// Library page scope: the groups of the albums with a feed in the
    /// Library (ADR 0082 packet 002: the Library scope stays, and
    /// Decision 2's groups apply inside it; see the packet report for the
    /// chosen shape).
    #[must_use]
    pub(crate) fn library_groups(&self) -> Vec<AlbumNameGroupVm> {
        let albums: Vec<PublisherPageAlbumFact> = self
            .facts
            .albums
            .iter()
            .filter(|album| album.in_library)
            .cloned()
            .collect();
        Self::album_name_groups(&albums)
    }

    /// Library page scope: the groups of the albums with no feed in the
    /// Library.
    #[must_use]
    pub(crate) fn other_groups(&self) -> Vec<AlbumNameGroupVm> {
        let albums: Vec<PublisherPageAlbumFact> = self
            .facts
            .albums
            .iter()
            .filter(|album| !album.in_library)
            .cloned()
            .collect();
        Self::album_name_groups(&albums)
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

    /// Every distinct album artwork URL on this page, in every group
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
    /// count, and the GUID of the publisher page it opens. An entry with
    /// no feed GUID has no page to open, so this drops it.
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
        let title_text = match &title {
            TitleDisplay::Stated(text) => text.clone(),
            TitleDisplay::Missing(guid) => guid.clone(),
        };
        Some(CoCreditedFeedVm {
            title,
            roles: entry.roles.clone(),
            album_count: entry.album_count,
            publisher_feed_guid: guid,
            a11y_label: format!("Open the publisher page of {title_text}"),
        })
    }

    /// Groups `albums` by `album_names_as`, then by role subgroup (ADR
    /// 0082 Decision 2). An album appears one time. An empty group or
    /// subgroup is absent.
    fn album_name_groups(albums: &[PublisherPageAlbumFact]) -> Vec<AlbumNameGroupVm> {
        [AlbumNameGroup::PublisherOf, AlbumNameGroup::ListedNotNamed]
            .into_iter()
            .filter_map(|group| {
                let matching: Vec<&PublisherPageAlbumFact> = albums
                    .iter()
                    .filter(|&album| Self::album_name_group(album) == group)
                    .collect();
                if matching.is_empty() {
                    return None;
                }
                Some(AlbumNameGroupVm {
                    group,
                    subgroups: Self::role_subgroups(&matching),
                })
            })
            .collect()
    }

    /// The group one album belongs to (ADR 0082 Decision 2). Every value
    /// other than `AlbumNamesAs::Publisher` is "Listed, not named": the
    /// live node sends only `publisher` or null (ADR 0082 Recorded Facts,
    /// 2026-10-02), and every album needs exactly one group.
    const fn album_name_group(album: &PublisherPageAlbumFact) -> AlbumNameGroup {
        match &album.album_names_as {
            Some(AlbumNamesAs::Publisher) => AlbumNameGroup::PublisherOf,
            _ => AlbumNameGroup::ListedNotNamed,
        }
    }

    /// Splits `albums` into the role subgroups of ADR 0082 Decision 2: the
    /// agreed role sets, sorted, then "Role stated by one side", "Roles
    /// differ, not confirmed" and "Role not stated". An empty subgroup is
    /// absent.
    ///
    /// The subgroup follows the same `AlbumRoleDisplay` variant that
    /// `role_display` gives the row, through `album_vm`. One function
    /// decides both facts, so a row's subgroup and its own role text
    /// never disagree. For example, a `OneSide` row with no stated role
    /// reads as `Unstated`. The same happens when its `role_source` is
    /// not `PublisherRel` or `MusicRel`. Each such row falls into "Role
    /// not stated", not "Role stated by one side".
    fn role_subgroups(albums: &[&PublisherPageAlbumFact]) -> Vec<AlbumRoleSubgroupVm> {
        let mut agreed: BTreeMap<String, Vec<PublisherPageAlbumVm>> = BTreeMap::new();
        let mut one_side = Vec::new();
        let mut conflict = Vec::new();
        let mut unstated = Vec::new();
        for &album in albums {
            let vm = Self::album_vm(album);
            match vm.role.clone() {
                AlbumRoleDisplay::Agreed { role } => {
                    agreed.entry(role).or_default().push(vm);
                }
                AlbumRoleDisplay::OneSide { .. } => one_side.push(vm),
                AlbumRoleDisplay::Conflict { .. } => conflict.push(vm),
                AlbumRoleDisplay::Unstated => unstated.push(vm),
            }
        }
        let mut subgroups: Vec<AlbumRoleSubgroupVm> = agreed
            .into_iter()
            .map(|(role_set, albums)| AlbumRoleSubgroupVm {
                caption: RoleSubgroupCaption::Agreed(role_set).label().to_owned(),
                albums,
            })
            .collect();
        if !one_side.is_empty() {
            subgroups.push(AlbumRoleSubgroupVm {
                caption: RoleSubgroupCaption::OneSide.label().to_owned(),
                albums: one_side,
            });
        }
        if !conflict.is_empty() {
            subgroups.push(AlbumRoleSubgroupVm {
                caption: RoleSubgroupCaption::Conflict.label().to_owned(),
                albums: conflict,
            });
        }
        if !unstated.is_empty() {
            subgroups.push(AlbumRoleSubgroupVm {
                caption: RoleSubgroupCaption::Unstated.label().to_owned(),
                albums: unstated,
            });
        }
        subgroups
    }

    fn album_vm(album: &PublisherPageAlbumFact) -> PublisherPageAlbumVm {
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
        PublisherPageAlbumVm {
            feed_guid: album.feed_guid.clone(),
            title,
            image_url: album.image_url.clone(),
            artist,
            role: Self::role_display(album),
            not_listed,
            in_library: album.in_library,
        }
    }

    /// The role and its source, ready to view (ADR 0082 Decision 3).
    /// `role_agreement` alone selects the display variant. A `Both` row
    /// with no stated role, or an unrecognized `role_agreement` value,
    /// reads as `Unstated`.
    fn role_display(album: &PublisherPageAlbumFact) -> AlbumRoleDisplay {
        match &album.role_agreement {
            Some(RoleAgreement::Both) => match &album.role {
                Some(role) => AlbumRoleDisplay::Agreed { role: role.clone() },
                None => AlbumRoleDisplay::Unstated,
            },
            Some(RoleAgreement::OneSide) => match (&album.role, &album.role_source) {
                (Some(role), Some(side @ (RoleSource::PublisherRel | RoleSource::MusicRel))) => {
                    AlbumRoleDisplay::OneSide {
                        role: role.clone(),
                        side: side.clone(),
                    }
                }
                _ => AlbumRoleDisplay::Unstated,
            },
            Some(RoleAgreement::Conflict) => AlbumRoleDisplay::Conflict {
                publisher_role: album.publisher_rel.clone(),
                music_role: album.music_rel.clone(),
            },
            None | Some(RoleAgreement::Unknown(_)) => AlbumRoleDisplay::Unstated,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A simple owned album: it names the publisher, with the given GUID,
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

    /// A single owned album, GUID `album-guid`, with no stated group or
    /// role fact. Tests that do not exercise the groups use this.
    fn owned_album() -> PublisherPageAlbumFact {
        album_with_guid("album-guid", None, None, None)
    }

    fn facts(albums: Vec<PublisherPageAlbumFact>) -> PublisherPageFacts {
        PublisherPageFacts {
            publisher_feed_guid: "publisher-guid".into(),
            feed_title: Some("Publisher Feed".into()),
            albums,
            ..PublisherPageFacts::default()
        }
    }

    /// Every album row in `groups`, flattened across every group and
    /// subgroup, in the order the groups give them.
    fn flatten(groups: &[AlbumNameGroupVm]) -> Vec<PublisherPageAlbumVm> {
        groups
            .iter()
            .flat_map(|group| group.subgroups.iter())
            .flat_map(|subgroup| subgroup.albums.iter().cloned())
            .collect()
    }

    /// R82-2-01: the view model exposes no page type. `PublisherPageType`
    /// and `PublisherPageVm::page_type` no longer exist in this module: a
    /// build of this file is itself proof. This test proves the other
    /// half: the header exposes no "Type" fact.
    #[test]
    fn adr_0082_role_groups_header_exposes_no_type_fact() {
        let vm = PublisherPageVm::new(facts(vec![album_with_guid(
            "a1",
            Some(AlbumNamesAs::Publisher),
            Some(RoleAgreement::Both),
            Some("label"),
        )]));
        assert!(
            vm.header_facts().iter().all(|fact| fact.label != "Type"),
            "the header must expose no Type fact"
        );
    }

    /// R82-2-02: an album with `album_names_as` `publisher` goes to
    /// "Publisher of". An album with a null `album_names_as` goes to
    /// "Listed, not named". The two groups keep that order.
    #[test]
    fn adr_0082_role_groups_album_names_as_selects_the_group() {
        let publisher_of = album_with_guid("a1", Some(AlbumNamesAs::Publisher), None, None);
        let listed_not_named = album_with_guid("a2", None, None, None);
        let vm = PublisherPageVm::new(facts(vec![publisher_of, listed_not_named]));

        let groups = vm.index_groups();

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].group, AlbumNameGroup::PublisherOf);
        assert_eq!(AlbumNameGroup::PublisherOf.label(), "Publisher of");
        assert_eq!(groups[1].group, AlbumNameGroup::ListedNotNamed);
        assert_eq!(AlbumNameGroup::ListedNotNamed.label(), "Listed, not named");
        let guids_of = |group: &AlbumNameGroupVm| -> Vec<Option<String>> {
            group
                .subgroups
                .iter()
                .flat_map(|subgroup| subgroup.albums.iter())
                .map(|album| album.feed_guid.clone())
                .collect()
        };
        assert_eq!(guids_of(&groups[0]), vec![Some("a1".to_owned())]);
        assert_eq!(guids_of(&groups[1]), vec![Some("a2".to_owned())]);
    }

    /// R82-2-03: rows with `role_agreement` `both` and role sets "artist"
    /// and "distributor" give two subgroups, sorted by role set. Rows with
    /// `one_side`, `conflict` and null give the other three subgroups, in
    /// that order.
    #[test]
    fn adr_0082_role_groups_role_agreement_selects_the_subgroup() {
        let agreed_distributor = album_with_guid(
            "agreed-distributor",
            Some(AlbumNamesAs::Publisher),
            Some(RoleAgreement::Both),
            Some("distributor"),
        );
        let agreed_artist = album_with_guid(
            "agreed-artist",
            Some(AlbumNamesAs::Publisher),
            Some(RoleAgreement::Both),
            Some("artist"),
        );
        let mut one_side = album_with_guid(
            "one-side",
            Some(AlbumNamesAs::Publisher),
            Some(RoleAgreement::OneSide),
            Some("label"),
        );
        one_side.role_source = Some(RoleSource::MusicRel);
        let mut conflict = album_with_guid(
            "conflict",
            Some(AlbumNamesAs::Publisher),
            Some(RoleAgreement::Conflict),
            None,
        );
        conflict.publisher_rel = Some("label".into());
        conflict.music_rel = Some("artist".into());
        let unstated = album_with_guid("unstated", Some(AlbumNamesAs::Publisher), None, None);

        let vm = PublisherPageVm::new(facts(vec![
            agreed_distributor,
            agreed_artist,
            one_side,
            conflict,
            unstated,
        ]));

        let groups = vm.index_groups();
        assert_eq!(groups.len(), 1, "every album names the publisher");
        let subgroups = &groups[0].subgroups;
        assert_eq!(
            subgroups.len(),
            5,
            "two agreed subgroups, plus one-side, conflict and unstated"
        );
        assert_eq!(subgroups[0].caption, "artist", "agreed subgroups sort");
        assert_eq!(subgroups[1].caption, "distributor");
        assert_eq!(subgroups[2].caption, "Role stated by one side");
        assert_eq!(subgroups[3].caption, "Roles differ, not confirmed");
        assert_eq!(subgroups[4].caption, "Role not stated");
    }

    /// Orchestrator review: the subgroup must follow the same
    /// `AlbumRoleDisplay` variant that the row text uses. A `OneSide` row
    /// with no stated role gives `AlbumRoleDisplay::Unstated`. `role_display`
    /// has no `OneSide` arm for a null role. The row must fall into "Role
    /// not stated", not "Role stated by one side".
    #[test]
    fn adr_0082_role_groups_subgroup_follows_row_display() {
        let one_side_no_role = album_with_guid(
            "a1",
            Some(AlbumNamesAs::Publisher),
            Some(RoleAgreement::OneSide),
            None,
        );

        let vm = PublisherPageVm::new(facts(vec![one_side_no_role]));

        let groups = vm.index_groups();
        assert_eq!(groups.len(), 1);
        let subgroups = &groups[0].subgroups;
        assert_eq!(
            subgroups.len(),
            1,
            "the row goes to one subgroup, matching its own row text"
        );
        assert_eq!(subgroups[0].caption, "Role not stated");
        assert_eq!(subgroups[0].albums[0].role, AlbumRoleDisplay::Unstated);
    }

    /// R82-2-04: each album appears one time, and an empty subgroup, or an
    /// empty group, is absent.
    #[test]
    fn adr_0082_role_groups_album_appears_once_and_empty_subgroup_is_absent() {
        let agreed = album_with_guid(
            "a1",
            Some(AlbumNamesAs::Publisher),
            Some(RoleAgreement::Both),
            Some("artist"),
        );
        let vm = PublisherPageVm::new(facts(vec![agreed]));

        let groups = vm.index_groups();

        assert_eq!(
            groups.len(),
            1,
            "the empty Listed, not named group is absent"
        );
        assert_eq!(
            groups[0].subgroups.len(),
            1,
            "each other subgroup is empty, so it is absent"
        );
        let rows = flatten(&groups);
        assert_eq!(rows.len(), 1, "the album appears one time");
    }

    /// R82-2-05: each `role_agreement` value gives the role text and
    /// source text of ADR 0082 Decision 3.
    #[test]
    fn adr_0082_role_groups_row_role_text_matches_decision_3() {
        let agreed = AlbumRoleDisplay::Agreed {
            role: "artist".into(),
        };
        assert_eq!(
            agreed.display_text(),
            "artist \u{2014} agreed by both feeds"
        );

        let one_side_album = AlbumRoleDisplay::OneSide {
            role: "label".into(),
            side: RoleSource::MusicRel,
        };
        assert_eq!(
            one_side_album.display_text(),
            "label \u{2014} stated by the album only"
        );

        let one_side_publisher = AlbumRoleDisplay::OneSide {
            role: "label".into(),
            side: RoleSource::PublisherRel,
        };
        assert_eq!(
            one_side_publisher.display_text(),
            "label \u{2014} stated by the publisher only"
        );

        let conflict = AlbumRoleDisplay::Conflict {
            publisher_role: Some("label".into()),
            music_role: Some("artist".into()),
        };
        assert_eq!(
            conflict.display_text(),
            "Publisher feed: label / Album feed: artist (not confirmed)"
        );

        let unstated = AlbumRoleDisplay::Unstated;
        assert_eq!(unstated.display_text(), "Role not stated");
    }

    /// R82-2-05: the role display built from facts, through `index_groups`,
    /// matches the direct `AlbumRoleDisplay` text for each `role_agreement`
    /// value.
    #[test]
    fn adr_0082_role_groups_role_display_built_from_facts() {
        let agreed = album_with_guid(
            "agreed",
            Some(AlbumNamesAs::Publisher),
            Some(RoleAgreement::Both),
            Some("artist"),
        );
        let mut one_side = album_with_guid(
            "one-side",
            Some(AlbumNamesAs::Publisher),
            Some(RoleAgreement::OneSide),
            Some("label"),
        );
        one_side.role_source = Some(RoleSource::PublisherRel);
        let mut conflict = album_with_guid(
            "conflict",
            Some(AlbumNamesAs::Publisher),
            Some(RoleAgreement::Conflict),
            None,
        );
        conflict.publisher_rel = Some("label".into());
        conflict.music_rel = Some("artist".into());
        let unstated = album_with_guid("unstated", Some(AlbumNamesAs::Publisher), None, None);

        let vm = PublisherPageVm::new(facts(vec![agreed, one_side, conflict, unstated]));
        let rows = flatten(&vm.index_groups());
        let text_for = |guid: &str| {
            rows.iter()
                .find(|album| album.feed_guid.as_deref() == Some(guid))
                .expect("the album must be in some subgroup")
                .role
                .display_text()
        };

        assert_eq!(text_for("agreed"), "artist \u{2014} agreed by both feeds");
        assert_eq!(
            text_for("one-side"),
            "label \u{2014} stated by the publisher only"
        );
        assert_eq!(
            text_for("conflict"),
            "Publisher feed: label / Album feed: artist (not confirmed)"
        );
        assert_eq!(text_for("unstated"), "Role not stated");
    }

    /// R82-2-06: the header exposes "Roles: artist, distributor" from two
    /// agreed rows, split from each role set, sorted.
    #[test]
    fn adr_0082_role_groups_header_roles_from_agreed_rows() {
        let agreed_artist = album_with_guid(
            "a1",
            Some(AlbumNamesAs::Publisher),
            Some(RoleAgreement::Both),
            Some("artist"),
        );
        let agreed_distributor = album_with_guid(
            "a2",
            Some(AlbumNamesAs::Publisher),
            Some(RoleAgreement::Both),
            Some("distributor"),
        );
        let vm = PublisherPageVm::new(facts(vec![agreed_artist, agreed_distributor]));

        let roles_fact = vm
            .header_facts()
            .into_iter()
            .find(|fact| fact.label == PublisherPageVm::ROLES_LABEL)
            .expect("two agreed rows must add a Roles fact");
        assert_eq!(roles_fact.value, "artist, distributor");
    }

    /// R82-2-06: a page with no agreed row exposes no Roles fact.
    #[test]
    fn adr_0082_role_groups_header_has_no_roles_fact_without_an_agreed_row() {
        let one_side = album_with_guid(
            "a1",
            Some(AlbumNamesAs::Publisher),
            Some(RoleAgreement::OneSide),
            Some("label"),
        );
        let vm = PublisherPageVm::new(facts(vec![one_side]));

        assert!(
            vm.header_facts()
                .iter()
                .all(|fact| fact.label != PublisherPageVm::ROLES_LABEL),
            "no agreed row must add no Roles fact"
        );
    }

    /// R82-2-07: a `co_credited_feeds` entry gives a "Shares albums with"
    /// row with its title, its roles, its album count, and an action that
    /// opens its publisher page.
    #[test]
    fn adr_0082_role_groups_shares_albums_with_entry_gives_title_roles_count_and_action() {
        let mut page_facts = facts(vec![]);
        page_facts.co_credited_feeds = vec![CoCreditedFeedFact {
            feed_guid: Some("other-feed".into()),
            title: Some("Other Publisher".into()),
            roles: vec!["artist".into(), "label".into()],
            album_count: Some(3),
        }];
        let vm = PublisherPageVm::new(page_facts);

        let entries = vm.shares_albums_with();

        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].title,
            TitleDisplay::Stated("Other Publisher".into())
        );
        assert_eq!(
            entries[0].roles,
            vec!["artist".to_owned(), "label".to_owned()]
        );
        assert_eq!(entries[0].album_count, Some(3));
        assert_eq!(entries[0].publisher_feed_guid, "other-feed");
        assert!(entries[0].a11y_label.contains("Other Publisher"));
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
            title: Some("Other Publisher".into()),
            roles: vec!["artist".into()],
            album_count: None,
        }];
        let vm = PublisherPageVm::new(page_facts);

        let entries = vm.shares_albums_with();

        assert_eq!(entries.len(), 1);
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

        let albums = flatten(&vm.index_groups());
        assert!(albums[0].not_listed);
        assert!(albums[1].not_listed);
        assert!(!albums[2].not_listed);
    }

    /// R3-10: a publisher feed without a title exposes its GUID and "No
    /// title".
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

        let albums = flatten(&vm.index_groups());
        assert_eq!(albums[0].title, TitleDisplay::Missing("album-guid".into()));
        assert_eq!(albums[0].artist, None);
    }

    /// R3-02a: when the request for the other albums fails, the Library
    /// group stays, and the view model reports the failure separately.
    #[test]
    fn adr_0077_publisher_page_other_albums_failure_keeps_library_group() {
        let mut album = owned_album();
        album.in_library = true;
        let mut page_facts = facts(vec![album]);
        page_facts.other_albums_failure = Some("HTTP 503 from /v1/feeds/p".into());
        let vm = PublisherPageVm::new(page_facts);

        assert_eq!(
            flatten(&vm.library_groups()).len(),
            1,
            "the Library group stays"
        );
        assert!(vm.other_groups().is_empty());
        assert_eq!(
            vm.other_albums_status(),
            OtherAlbumsStatus::Unavailable {
                report: OtherAlbumsStatus::UNAVAILABLE_REPORT,
                detail: "HTTP 503 from /v1/feeds/p".into(),
            },
            "the report comes first, and the transport detail follows it"
        );
    }

    /// Library page scope: the view model, not a renderer, puts each album
    /// in the Library scope or in the other scope.
    #[test]
    fn adr_0077_publisher_page_library_groups_come_from_the_view_model() {
        let mut local = album_with_guid("local", None, None, None);
        local.in_library = true;
        let remote = album_with_guid("remote", None, None, None);
        let vm = PublisherPageVm::new(facts(vec![local, remote]));

        let library: Vec<_> = flatten(&vm.library_groups())
            .into_iter()
            .map(|album| album.feed_guid)
            .collect();
        let other: Vec<_> = flatten(&vm.other_groups())
            .into_iter()
            .map(|album| album.feed_guid)
            .collect();
        assert_eq!(library, vec![Some("local".to_owned())]);
        assert_eq!(other, vec![Some("remote".to_owned())]);
        assert_eq!(vm.other_albums_status(), OtherAlbumsStatus::Loaded);
    }

    /// R4-04: the page title text is the view model title, from the
    /// publisher feed's own `<title>`.
    #[test]
    fn adr_0077_publisher_navigation_title_text_uses_stated_title() {
        let vm = PublisherPageVm::new(facts(vec![]));
        assert_eq!(vm.title_text(), "Publisher Feed");
    }

    /// R4-04: a publisher feed without a title shows its GUID as its title
    /// text, the same value `title` gives for the page.
    #[test]
    fn adr_0077_publisher_navigation_title_text_falls_back_to_guid() {
        let mut page_facts = facts(vec![]);
        page_facts.feed_title = None;
        let vm = PublisherPageVm::new(page_facts);
        assert_eq!(vm.title_text(), "publisher-guid");
    }

    /// Packet 004: a publisher feed without a title adds a "Title" row
    /// naming the missing-title label. The screen writes no such text.
    #[test]
    fn adr_0077_publisher_navigation_header_facts_name_a_missing_title() {
        let mut page_facts = facts(vec![]);
        page_facts.feed_title = None;
        let vm = PublisherPageVm::new(page_facts);

        let facts = vm.header_facts();

        let title_row = facts
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

        let facts = vm.header_facts();

        let artists_row = facts
            .iter()
            .find(|fact| fact.label == PublisherPageVm::CONFIRMED_ARTISTS_LABEL)
            .expect("a confirmed count adds an artist row");
        assert!(artists_row.value.contains("2"));
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
                message: "Loading the Library publisher page p-guid...".into(),
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
