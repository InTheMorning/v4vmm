//! Publisher page view model (ADR 0077 Task 003, ADR 0078).
//!
//! The Index publisher page query and the Library publisher page query, in
//! `src/application/queries`, each build one [`PublisherPageFacts`] value
//! from their own data. This module turns that value into the page type,
//! the album groups, the title, and the artist count that the page shows.
//! No renderer computes these facts. [`PublisherPageVm`] computes each one,
//! and packet 004 wires a screen to it.

#![warn(clippy::pedantic)]
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "ADR 0077 packet 004 connects a screen to this module. Remove this expectation in that packet."
    )
)]

use crate::api::{PublisherLinkResolution, RoleSource};

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
    /// `distinct_release_artist_count` of the publisher feed. `MusicIndex`
    /// derives it; it never selects the page type (ADR 0078).
    pub(crate) distinct_release_artist_count: Option<i64>,
    pub(crate) distinct_release_artists: Vec<String>,
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

/// The stated, assumed, or conflicting role of one album/publisher pair
/// (ADR 0077 Decision 3, R3-06, R3-07).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AlbumRoleDisplay {
    /// A feed stated this role. `source` names which feed stated it.
    Stated { role: String, source: RoleSource },
    /// No feed stated a role. Stophammer's assumed default.
    Assumed { role: String },
    /// The two feeds stated different roles. Neither value wins.
    Conflict {
        publisher_role: Option<String>,
        music_role: Option<String>,
    },
    /// Neither a stated role nor an assumed one is present.
    Unknown,
}

/// The artist of one album, from `remote_release_artist` (R3-14). `None`
/// when the entry states no artist. The view model invents no placeholder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AlbumArtistDisplay {
    pub(crate) name: String,
    pub(crate) source: Option<String>,
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

/// The type of a publisher page (ADR 0078).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PublisherPageType {
    Artist,
    Label,
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
}

/// The publisher page view model (ADR 0077 Task 003).
pub(crate) struct PublisherPageVm {
    facts: PublisherPageFacts,
}

impl PublisherPageVm {
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

    /// R3-11: the derived artist count, apart from `page_type`.
    #[must_use]
    pub(crate) fn derived_artist_count(&self) -> Option<DerivedArtistCount> {
        self.facts
            .distinct_release_artist_count
            .map(|count| DerivedArtistCount {
                count,
                names: self.facts.distinct_release_artists.clone(),
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

    fn role_display(album: &PublisherPageAlbumFact) -> AlbumRoleDisplay {
        match (&album.role, &album.role_source) {
            (Some(role), Some(source @ (RoleSource::PublisherRel | RoleSource::MusicRel))) => {
                AlbumRoleDisplay::Stated {
                    role: role.clone(),
                    source: source.clone(),
                }
            }
            (Some(role), Some(RoleSource::Default)) => {
                AlbumRoleDisplay::Assumed { role: role.clone() }
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
        page_facts.distinct_release_artist_count = Some(5);
        page_facts.distinct_release_artists = vec!["A".into(), "B".into(), "C".into()];
        let vm = PublisherPageVm::new(page_facts);
        assert_eq!(vm.page_type(), PublisherPageType::Artist);
        assert_eq!(
            vm.derived_artist_count(),
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

    /// R3-07: a `default` role is exposed as assumed, never as stated.
    #[test]
    fn adr_0077_publisher_page_default_role_is_exposed_as_assumed() {
        let vm = PublisherPageVm::new(facts(vec![owned_album(
            Some("artist"),
            Some(RoleSource::Default),
        )]));
        let albums = vm.owned_albums();
        assert_eq!(
            albums[0].role,
            AlbumRoleDisplay::Assumed {
                role: "artist".into()
            }
        );
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
        page_facts.distinct_release_artist_count = Some(3);
        page_facts.distinct_release_artists = vec!["A".into(), "B".into(), "C".into()];
        let vm = PublisherPageVm::new(page_facts);

        assert_eq!(vm.page_type(), PublisherPageType::Label);
        let count = vm.derived_artist_count().expect("a count is present");
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
}
