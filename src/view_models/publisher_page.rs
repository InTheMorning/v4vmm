//! Publisher page view model (ADR 0077 Task 003, ADR 0078, ADR 0077 Task 007).
//!
//! The Index publisher page query and the Library publisher page query, in
//! `src/application/queries`, each build one [`PublisherPageFacts`] value
//! from their own data. This module turns that value into the page type,
//! the album groups, the title, and the confirmed and unconfirmed artist
//! counts that the page shows. No renderer computes these facts.
//! [`PublisherPageVm`] computes each one, and packet 004 wires a screen to
//! it.

#![warn(clippy::pedantic)]

use crate::api::{PublisherLinkResolution, RoleSource};

/// The route that opened a publisher page (ADR 0077 packet 004). This value
/// selects which `PublisherPageVm` groups the screen shows. It decides no
/// page type, no role and no action availability: those come from
/// [`PublisherPageVm`] alone.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PublisherPageContext {
    /// Reached from the Library. The screen shows `library_albums` and
    /// `other_albums` as two groups.
    Library,
    /// Reached from the Index. The screen shows `owned_albums` and
    /// `listed_by_albums`, and marks each album with `in_library`.
    Index,
}

impl PublisherPageContext {
    /// The section captions for this page shape (Required Change 4). The
    /// screen decides no group label of its own.
    #[must_use]
    pub(crate) const fn group_labels(self) -> (&'static str, &'static str) {
        match self {
            Self::Library => ("Library Albums", "Other Albums"),
            Self::Index => ("Owned Albums", "Listed By"),
        }
    }

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
    /// `MusicIndex` derives it; it never selects the page type (ADR 0078).
    pub(crate) confirmed_release_artist_count: Option<i64>,
    pub(crate) confirmed_release_artists: Vec<String>,
    /// `unconfirmed_release_artist_count` of the publisher feed: the
    /// distinct artists of the albums that this publisher lists but that do
    /// not name it (Stophammer ADR 0061, ADR 0077 Task 007). `MusicIndex`
    /// derives it; it never selects the page type (ADR 0078).
    pub(crate) unconfirmed_release_artist_count: Option<i64>,
    pub(crate) unconfirmed_release_artists: Vec<String>,
    pub(crate) albums: Vec<PublisherPageAlbumFact>,
    /// R3-02a: the reason the request for the albums that are not in the
    /// Library failed. `None` when that request has not failed. The Library
    /// group in `albums` stays present either way.
    pub(crate) other_albums_failure: Option<String>,
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

/// The stated or conflicting role of one album/publisher pair (ADR 0077
/// Decision 3, R3-06, R3-07).
///
/// ADR 0082 Decision 2, Recorded Facts (2026-10-02): the Stophammer 0.7.0
/// node never sends a stated role with `role_source` `default`. It gives
/// no assumed default. `AlbumRoleDisplay` dropped its `Assumed` variant for
/// this reason (ADR 0082 packet 001, Required Change 4): no 0.7.0 response
/// can reach it. A role with no stated source still decodes, and
/// `role_display` reads it as `Unknown`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AlbumRoleDisplay {
    /// A feed stated this role. `source` names which feed stated it.
    Stated { role: String, source: RoleSource },
    /// The two feeds stated different roles. Neither value wins.
    Conflict {
        publisher_role: Option<String>,
        music_role: Option<String>,
    },
    /// No feed stated a role, or the row names no stated source for it.
    Unknown,
}

impl AlbumRoleDisplay {
    /// The word this role shows, when a feed stated it (ADR 0077 packet
    /// 004). `None` for a conflict or an unknown role: the screen reads
    /// `Self::Conflict`'s own two stated values instead of a single word.
    #[must_use]
    pub(crate) fn text(&self) -> Option<&str> {
        match self {
            Self::Stated { role, .. } => Some(role.as_str()),
            Self::Conflict { .. } | Self::Unknown => None,
        }
    }

    /// `true` only when a feed stated this role. `false` for a conflict or
    /// an unknown role (R3-07: a role with no stated source never shows as
    /// stated).
    #[must_use]
    pub(crate) const fn is_stated(&self) -> bool {
        matches!(self, Self::Stated { .. })
    }

    /// The two stated values of a role conflict, ready to view (ADR 0077
    /// Decision 3, R3-06). `None` when this is not a conflict.
    #[must_use]
    pub(crate) fn conflict_text(&self) -> Option<String> {
        match self {
            Self::Conflict {
                publisher_role,
                music_role,
            } => Some(format!(
                "Publisher feed: {} / Album feed: {}",
                publisher_role.as_deref().unwrap_or(Self::CONFLICT_UNSTATED),
                music_role.as_deref().unwrap_or(Self::CONFLICT_UNSTATED),
            )),
            Self::Stated { .. } | Self::Unknown => None,
        }
    }

    /// The word this display shows for a conflict side that states no role.
    const CONFLICT_UNSTATED: &'static str = "no stated role";
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

/// The type of a publisher page (ADR 0078).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PublisherPageType {
    Artist,
    Label,
}

impl PublisherPageType {
    /// The word this page type shows (ADR 0078, packet 004). This word
    /// never selects the type: `PublisherPageVm::page_type` alone does.
    #[must_use]
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Artist => "Artist",
            Self::Label => "Label",
        }
    }
}

/// The derived artist count of a publisher feed (ADR 0077, ADR 0078). It
/// never selects the page type.
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
    /// Row label for the page type fact (packet 004).
    pub(crate) const TYPE_LABEL: &'static str = "Type";
    /// Row label for the "No title" fact, shown only when the feed states
    /// no title.
    pub(crate) const TITLE_LABEL: &'static str = "Title";
    /// Row label for the confirmed artist count fact (ADR 0077 Task 007):
    /// the artists of the albums that name this feed.
    pub(crate) const CONFIRMED_ARTISTS_LABEL: &'static str = "Artists that name this feed";
    /// Row label for the unconfirmed artist count fact (ADR 0077 Task 007):
    /// the artists of the albums that this feed lists without a link back.
    pub(crate) const UNCONFIRMED_ARTISTS_LABEL: &'static str =
        "Artists this feed lists without a link back";

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

    /// The header facts of this page, ready to view (packet 004). The
    /// screen shows each fact as a label/value pair; it decides no label
    /// and composes no text.
    #[must_use]
    pub(crate) fn header_facts(&self) -> Vec<PublisherPageHeaderFact> {
        let mut facts = vec![PublisherPageHeaderFact {
            label: Self::TYPE_LABEL,
            value: self.page_type().label().to_owned(),
        }];
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

    /// ADR 0078: `Label` only when one owned album states the label role.
    /// R3-03, R3-04, R3-05, R3-06.
    #[must_use]
    pub(crate) fn page_type(&self) -> PublisherPageType {
        let is_label = self
            .facts
            .albums
            .iter()
            .filter(|album| album.music_names_publisher == Some(true))
            .any(|album| {
                album.role.as_deref() == Some("label")
                    && matches!(
                        album.role_source,
                        Some(RoleSource::PublisherRel | RoleSource::MusicRel)
                    )
            });
        if is_label {
            PublisherPageType::Label
        } else {
            PublisherPageType::Artist
        }
    }

    /// R3-08: albums with `music_names_publisher = true`.
    #[must_use]
    pub(crate) fn owned_albums(&self) -> Vec<PublisherPageAlbumVm> {
        self.facts
            .albums
            .iter()
            .filter(|album| album.music_names_publisher == Some(true))
            .map(Self::album_vm)
            .collect()
    }

    /// R3-08: albums with `music_names_publisher = false`, in their own
    /// group, apart from `owned_albums`.
    #[must_use]
    pub(crate) fn listed_by_albums(&self) -> Vec<PublisherPageAlbumVm> {
        self.facts
            .albums
            .iter()
            .filter(|album| album.music_names_publisher != Some(true))
            .map(Self::album_vm)
            .collect()
    }

    /// R3-11, R7-05 (ADR 0077 Task 007): the confirmed artist count, apart
    /// from `page_type`. `None` when the response gives no confirmed count.
    #[must_use]
    pub(crate) fn confirmed_artist_count(&self) -> Option<DerivedArtistCount> {
        self.facts
            .confirmed_release_artist_count
            .map(|count| DerivedArtistCount {
                count,
                names: self.facts.confirmed_release_artists.clone(),
            })
    }

    /// R7-05 (ADR 0077 Task 007): the unconfirmed artist count, apart from
    /// `page_type`. `None` when the response gives no unconfirmed count.
    #[must_use]
    pub(crate) fn unconfirmed_artist_count(&self) -> Option<DerivedArtistCount> {
        self.facts
            .unconfirmed_release_artist_count
            .map(|count| DerivedArtistCount {
                count,
                names: self.facts.unconfirmed_release_artists.clone(),
            })
    }

    /// Library page scope: the albums with a feed in the Library.
    #[must_use]
    pub(crate) fn library_albums(&self) -> Vec<PublisherPageAlbumVm> {
        self.facts
            .albums
            .iter()
            .filter(|album| album.in_library)
            .map(Self::album_vm)
            .collect()
    }

    /// Library page scope: the albums with no feed in the Library.
    #[must_use]
    pub(crate) fn other_albums(&self) -> Vec<PublisherPageAlbumVm> {
        self.facts
            .albums
            .iter()
            .filter(|album| !album.in_library)
            .map(Self::album_vm)
            .collect()
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

    /// ADR 0082 packet 001, Required Change 4: a 0.7.0 row never pairs a
    /// stated `role` with `role_source` `default`, so this function keeps
    /// no arm for that pairing. A row that still reaches it, for example
    /// from the fixture era before 2026-10-02, reads as `Unknown`, the same
    /// display that a role with no stated source already gave.
    fn role_display(album: &PublisherPageAlbumFact) -> AlbumRoleDisplay {
        match (&album.role, &album.role_source) {
            (Some(role), Some(source @ (RoleSource::PublisherRel | RoleSource::MusicRel))) => {
                AlbumRoleDisplay::Stated {
                    role: role.clone(),
                    source: source.clone(),
                }
            }
            (None, Some(RoleSource::Conflict)) => AlbumRoleDisplay::Conflict {
                publisher_role: album.publisher_rel.clone(),
                music_role: album.music_rel.clone(),
            },
            _ => AlbumRoleDisplay::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned_album(role: Option<&str>, role_source: Option<RoleSource>) -> PublisherPageAlbumFact {
        PublisherPageAlbumFact {
            feed_guid: Some("album-guid".into()),
            title: Some("Album Title".into()),
            music_names_publisher: Some(true),
            role: role.map(str::to_owned),
            role_source,
            ..PublisherPageAlbumFact::default()
        }
    }

    fn facts(albums: Vec<PublisherPageAlbumFact>) -> PublisherPageFacts {
        PublisherPageFacts {
            publisher_feed_guid: "publisher-guid".into(),
            feed_title: Some("Publisher Feed".into()),
            albums,
            ..PublisherPageFacts::default()
        }
    }

    /// R3-03 (ADR 0078): one owned album with a stated label role gives a
    /// label page.
    #[test]
    fn adr_0077_publisher_page_stated_label_role_gives_label_page() {
        let vm = PublisherPageVm::new(facts(vec![owned_album(
            Some("label"),
            Some(RoleSource::PublisherRel),
        )]));
        assert_eq!(vm.page_type(), PublisherPageType::Label);
    }

    /// R3-04 (ADR 0078): a count of five with each role `default` gives an
    /// artist page. The count never selects the type.
    #[test]
    fn adr_0077_publisher_page_default_role_with_high_artist_count_gives_artist_page() {
        let albums = (0..5)
            .map(|_| owned_album(Some("artist"), Some(RoleSource::Default)))
            .collect();
        let mut page_facts = facts(albums);
        page_facts.confirmed_release_artist_count = Some(5);
        page_facts.confirmed_release_artists = vec!["A".into(), "B".into(), "C".into()];
        let vm = PublisherPageVm::new(page_facts);
        assert_eq!(vm.page_type(), PublisherPageType::Artist);
        assert_eq!(
            vm.confirmed_artist_count(),
            Some(DerivedArtistCount {
                count: 5,
                names: vec!["A".into(), "B".into(), "C".into()],
            })
        );
    }

    /// R3-05: a "listed by" album with a stated label role does not give a
    /// label page.
    #[test]
    fn adr_0077_publisher_page_listed_by_label_role_does_not_give_label_page() {
        let mut listed_by = owned_album(Some("label"), Some(RoleSource::PublisherRel));
        listed_by.music_names_publisher = Some(false);
        let vm = PublisherPageVm::new(facts(vec![listed_by]));
        assert_eq!(vm.page_type(), PublisherPageType::Artist);
    }

    /// R3-06: a conflict pair with `label` on one side does not give a
    /// label page, and the view model exposes both stated values.
    #[test]
    fn adr_0077_publisher_page_conflict_pair_does_not_give_label_page() {
        let mut conflict = owned_album(None, Some(RoleSource::Conflict));
        conflict.publisher_rel = Some("label".into());
        conflict.music_rel = Some("artist".into());
        let vm = PublisherPageVm::new(facts(vec![conflict]));
        assert_eq!(vm.page_type(), PublisherPageType::Artist);
        let albums = vm.owned_albums();
        assert_eq!(
            albums[0].role,
            AlbumRoleDisplay::Conflict {
                publisher_role: Some("label".into()),
                music_role: Some("artist".into()),
            }
        );
    }

    /// ADR 0082 packet 001, Required Change 4: the recorded 0.7.0 DETOX
    /// shape, a null `role` with `role_source` `default`, is exposed as
    /// `Unknown`, never as stated. This replaces R3-07 (ADR 0077): the
    /// `Assumed` variant it checked is gone, because no 0.7.0 row pairs a
    /// stated role with a `default` source.
    #[test]
    fn adr_0082_link_facts_default_role_source_with_null_role_is_unknown() {
        let vm = PublisherPageVm::new(facts(vec![owned_album(None, Some(RoleSource::Default))]));
        let albums = vm.owned_albums();
        assert_eq!(albums[0].role, AlbumRoleDisplay::Unknown);
        assert_ne!(
            albums[0].role,
            AlbumRoleDisplay::Stated {
                role: "artist".into(),
                source: RoleSource::Default,
            },
            "a default role must never be exposed as stated"
        );
    }

    /// R3-08: "listed by" albums are a group apart from owned albums.
    #[test]
    fn adr_0077_publisher_page_owned_and_listed_by_are_separate_groups() {
        let mut owned = owned_album(Some("artist"), Some(RoleSource::Default));
        owned.feed_guid = Some("owned-guid".into());
        let mut listed_by = owned_album(Some("artist"), Some(RoleSource::Default));
        listed_by.feed_guid = Some("listed-by-guid".into());
        listed_by.music_names_publisher = Some(false);
        let vm = PublisherPageVm::new(facts(vec![owned, listed_by]));

        let owned_albums = vm.owned_albums();
        let listed_by_albums = vm.listed_by_albums();
        assert_eq!(owned_albums.len(), 1);
        assert_eq!(owned_albums[0].feed_guid.as_deref(), Some("owned-guid"));
        assert_eq!(listed_by_albums.len(), 1);
        assert_eq!(
            listed_by_albums[0].feed_guid.as_deref(),
            Some("listed-by-guid")
        );
    }

    /// R3-09: an owned album with `publisher_lists_music = false` or an
    /// unresolved link gets the "Not listed" mark.
    #[test]
    fn adr_0077_publisher_page_not_listed_mark_on_owned_albums() {
        let mut not_listed = owned_album(Some("artist"), Some(RoleSource::Default));
        not_listed.publisher_lists_music = Some(false);
        let mut unresolved = owned_album(Some("artist"), Some(RoleSource::Default));
        unresolved.publisher_link_resolution = Some(PublisherLinkResolution::Unresolved);
        let mut listed = owned_album(Some("artist"), Some(RoleSource::Default));
        listed.publisher_lists_music = Some(true);
        let vm = PublisherPageVm::new(facts(vec![not_listed, unresolved, listed]));

        let albums = vm.owned_albums();
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

    /// R3-11: the artist count is exposed with a derived label, apart from
    /// the page type.
    #[test]
    fn adr_0077_publisher_page_artist_count_is_derived_and_separate_from_page_type() {
        let mut page_facts = facts(vec![owned_album(
            Some("label"),
            Some(RoleSource::PublisherRel),
        )]);
        page_facts.confirmed_release_artist_count = Some(3);
        page_facts.confirmed_release_artists = vec!["A".into(), "B".into(), "C".into()];
        let vm = PublisherPageVm::new(page_facts);

        assert_eq!(vm.page_type(), PublisherPageType::Label);
        let count = vm.confirmed_artist_count().expect("a count is present");
        assert_eq!(count.count, 3);
        assert_eq!(DerivedArtistCount::LABEL, "Derived from album credits");
    }

    /// R3-14: an album entry with a null `remote_feed_title` exposes its
    /// GUID and "No title". A null `remote_release_artist` exposes no
    /// artist value.
    #[test]
    fn adr_0077_publisher_page_null_album_title_and_artist_expose_no_placeholder() {
        let mut album = owned_album(Some("artist"), Some(RoleSource::Default));
        album.title = None;
        album.artist = None;
        let vm = PublisherPageVm::new(facts(vec![album]));

        let albums = vm.owned_albums();
        assert_eq!(albums[0].title, TitleDisplay::Missing("album-guid".into()));
        assert_eq!(albums[0].artist, None);
    }

    /// R3-02a: when the request for the other albums fails, the Library
    /// group stays, and the view model reports the failure separately.
    #[test]
    fn adr_0077_publisher_page_other_albums_failure_keeps_library_group() {
        let mut album = owned_album(Some("artist"), Some(RoleSource::Default));
        album.in_library = true;
        let mut page_facts = facts(vec![album]);
        page_facts.other_albums_failure = Some("HTTP 503 from /v1/feeds/p".into());
        let vm = PublisherPageVm::new(page_facts);

        assert_eq!(vm.library_albums().len(), 1, "the Library group stays");
        assert!(vm.other_albums().is_empty());
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
    /// in the Library group or in the other group.
    #[test]
    fn adr_0077_publisher_page_library_groups_come_from_the_view_model() {
        let mut local = owned_album(Some("artist"), Some(RoleSource::Default));
        local.feed_guid = Some("local".into());
        local.in_library = true;
        let mut remote = owned_album(Some("artist"), Some(RoleSource::Default));
        remote.feed_guid = Some("remote".into());
        let vm = PublisherPageVm::new(facts(vec![local, remote]));

        let library: Vec<_> = vm
            .library_albums()
            .into_iter()
            .map(|a| a.feed_guid)
            .collect();
        let other: Vec<_> = vm.other_albums().into_iter().map(|a| a.feed_guid).collect();
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

    /// Packet 004: the header facts name the page type. A stated title
    /// adds no "Title" row.
    #[test]
    fn adr_0077_publisher_navigation_header_facts_name_the_page_type() {
        let vm = PublisherPageVm::new(facts(vec![owned_album(
            Some("label"),
            Some(RoleSource::PublisherRel),
        )]));

        let facts = vm.header_facts();

        assert_eq!(facts[0].label, PublisherPageVm::TYPE_LABEL);
        assert_eq!(facts[0].value, "Label");
        assert!(
            facts
                .iter()
                .all(|fact| fact.label != PublisherPageVm::TITLE_LABEL),
            "a stated title adds no Title row"
        );
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
        let mut page_facts = facts(vec![owned_album(Some("artist"), Some(RoleSource::Default))]);
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
        let mut with_art = owned_album(Some("artist"), Some(RoleSource::Default));
        with_art.image_url = Some("https://example.test/a.png".into());
        let mut duplicate_art = owned_album(Some("artist"), Some(RoleSource::Default));
        duplicate_art.image_url = Some("https://example.test/a.png".into());
        let mut no_art = owned_album(Some("artist"), Some(RoleSource::Default));
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
        let page_facts = facts(vec![owned_album(Some("artist"), Some(RoleSource::Default))]);
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

    /// R7-05: the page type stays equal for two pages that differ only in
    /// their confirmed and unconfirmed artist lists (ADR 0078).
    #[test]
    fn adr_0077_confirmed_artists_page_type_is_equal_across_different_artist_lists() {
        let mut few_artists = facts(vec![owned_album(Some("artist"), Some(RoleSource::Default))]);
        few_artists.confirmed_release_artist_count = Some(1);
        few_artists.confirmed_release_artists = vec!["A".into()];

        let mut many_artists = facts(vec![owned_album(Some("artist"), Some(RoleSource::Default))]);
        many_artists.confirmed_release_artist_count = Some(33);
        many_artists.confirmed_release_artists = (0..33).map(|n| format!("Artist {n}")).collect();
        many_artists.unconfirmed_release_artist_count = Some(10);
        many_artists.unconfirmed_release_artists = vec!["Other Artist".into()];

        let vm_few = PublisherPageVm::new(few_artists);
        let vm_many = PublisherPageVm::new(many_artists);
        assert_eq!(vm_few.page_type(), vm_many.page_type());
    }
}
