//! Named MusicIndex request profiles (ADR 0075 Decision I, packet 017).
//!
//! A profile names the path shape and the include list of one request that
//! the Library route or the Index route sends. It carries no provider
//! ownership. Under Decision I, a provider is transport evidence, not a
//! source, so a profile must not model which provider carried a value.
//!
//! Packet 018 will use a profile's own value as a cache key. A profile is
//! `Copy`, `PartialEq`, `Eq`, and `Hash`, so two equal requests already
//! produce equal, hashable values. This module adds no cache and no
//! expiry.
//!
//! The named constants below match the rows of packet 017's
//! required-profile table. ADR 0075, amendment of 2026-10-07, replaces their
//! include lists: each request asks for every collection of its endpoint. A constant's name is its name; the module
//! adds no separate name field or name type. The two inspector track detail
//! profiles are gone: their only caller was the parked Discover inspector,
//! which ADR 0060 removed.

/// The path shape one request profile requests.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum RequestPathShape {
    /// `/v1/feeds/{feed_guid}`
    Feed,
    /// `/v1/feeds/{feed_guid}/tracks/{item_guid}`
    ScopedTrack,
    /// `/v1/tracks/{item_guid}`
    UnscopedTrack,
    /// `/v1/artist-names` (Stophammer ADR 0070, ADR 0084 packet 016): one
    /// page of artist names in name order.
    ArtistNameList,
    /// `/v1/feeds/by-name` (Stophammer ADR 0070, ADR 0084 packet 016): one
    /// page of releases in name order.
    FeedNameList,
    /// `/v1/search` (Stophammer ADR 0076 section 3, ADR 0084 packet 029):
    /// one page of search results.
    Search,
    /// `/v1/feeds/recent` with `named_feed_guid` (Stophammer ADR 0076
    /// section 2, ADR 0084 packet 026): one page of the releases that name
    /// one feed, newest first.
    RecentFeedList,
}

/// One named MusicIndex request profile.
///
/// A profile carries the path shape and the include list that ADR 0075
/// packet 017 records for one caller. It carries no provider ownership
/// field. ADR 0075 Decision I keeps a provider as transport evidence,
/// never a source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct RequestProfile {
    path_shape: RequestPathShape,
    include: Option<&'static str>,
}

impl RequestProfile {
    const fn new(path_shape: RequestPathShape, include: Option<&'static str>) -> Self {
        Self {
            path_shape,
            include,
        }
    }

    /// Names the path shape this profile requests.
    pub(crate) const fn path_shape(&self) -> RequestPathShape {
        self.path_shape
    }

    /// Returns the include list this profile sends.
    ///
    /// A profile without an include list returns `None`. The caller then sends no `include`
    /// query parameter.
    pub(crate) const fn include(&self) -> Option<&'static str> {
        self.include
    }
}

/// Each collection that `/v1/feeds/{guid}` supports, and
/// `source_enclosures` for the nested tracks. ADR 0075, amendment of
/// 2026-10-07: each request asks for full data.
const FEED_FULL: &str = "tracks,source_enclosures,source_links,source_ids,source_release_claims,source_contributors,source_platforms,payment_routes,remote_items,publisher";
/// Each collection that the two track endpoints support. ADR 0075,
/// amendment of 2026-10-07.
const TRACK_FULL: &str = "payment_routes,value_time_splits,source_links,source_ids,source_contributors,source_release_claims,source_enclosures,source_transcripts,remote_items,publisher";

/// Library track detail, scoped track. First. Used only when the row has
/// a feed GUID. Owner: `feed_service::fetch_library_track_detail_with_recorder`.
pub(crate) const LIBRARY_TRACK_DETAIL_SCOPED_TRACK: RequestProfile =
    RequestProfile::new(RequestPathShape::ScopedTrack, Some(TRACK_FULL));

/// Library track detail, unscoped track. Second. Used only when the first
/// request returns no track. Owner: the same function.
pub(crate) const LIBRARY_TRACK_DETAIL_UNSCOPED_TRACK: RequestProfile =
    RequestProfile::new(RequestPathShape::UnscopedTrack, Some(TRACK_FULL));

/// Library track detail, feed. Third. Owner: the same function.
pub(crate) const LIBRARY_TRACK_DETAIL_FEED: RequestProfile =
    RequestProfile::new(RequestPathShape::Feed, Some(FEED_FULL));

/// Library feed update, feed. The only request of `feed_service::apply_feed_updates`.
pub(crate) const LIBRARY_FEED_UPDATE_FEED: RequestProfile =
    RequestProfile::new(RequestPathShape::Feed, Some(FEED_FULL));

/// Library album hydration, feed. The only request of
/// `library::hydrate_album_identity_facts`.
pub(crate) const LIBRARY_ALBUM_HYDRATION_FEED: RequestProfile =
    RequestProfile::new(RequestPathShape::Feed, Some(FEED_FULL));

/// Index feed detail. The only request of each of its two call sites:
/// `search::owner_fetch_feed` (through the detail-on-open command
/// `search::FetchIndexFeedDetail`, ADR 0075 packet 047) and
/// `feed::fetch_recent_feed_result_rows`.
pub(crate) const INDEX_FEED_DETAIL: RequestProfile =
    RequestProfile::new(RequestPathShape::Feed, Some(FEED_FULL));

/// Index track detail, scoped. Used when the hit has a nonempty feed GUID.
/// Owner: `search::fetch_index_track_detail`.
pub(crate) const INDEX_TRACK_DETAIL_SCOPED: RequestProfile =
    RequestProfile::new(RequestPathShape::ScopedTrack, Some(TRACK_FULL));

/// Index track detail, unscoped. Used when the hit has no feed GUID.
/// Owner: the same function.
pub(crate) const INDEX_TRACK_DETAIL_UNSCOPED: RequestProfile =
    RequestProfile::new(RequestPathShape::UnscopedTrack, Some(TRACK_FULL));

/// Index publisher page. The one request of the Index and the Library
/// publisher page queries (ADR 0077 Task 003). Owners:
/// `feed::fetch_index_publisher_page_albums` and
/// `library::fetch_library_publisher_page`.
pub(crate) const INDEX_PUBLISHER_PAGE: RequestProfile =
    RequestProfile::new(RequestPathShape::Feed, Some(FEED_FULL));

/// MusicIndex releases of one name. Each request of the name level page
/// and of the feed page section "Matched by name" (ADR 0084 packet 017).
/// It sends `artist_key` and no include. Owner:
/// `search::fetch_name_level_releases`.
pub(crate) const INDEX_NAME_LEVEL_RELEASES: RequestProfile =
    RequestProfile::new(RequestPathShape::FeedNameList, None);

/// MusicIndex name check of one search name. The one request that tells
/// whether a search name row shows (ADR 0084 packet 017 decision D15). It
/// sends `from` and `limit=1` and no include. Owner:
/// `search::index_name_row_exists`.
pub(crate) const INDEX_ARTIST_NAME_CHECK: RequestProfile =
    RequestProfile::new(RequestPathShape::ArtistNameList, None);

/// MusicIndex artist name list. The one request of each page of the
/// Artists scope "Everything" (ADR 0084 packet 016). Owner:
/// `name_lists::FetchArtistNamePage`.
pub(crate) const INDEX_ARTIST_NAME_LIST: RequestProfile =
    RequestProfile::new(RequestPathShape::ArtistNameList, Some("link_facts"));

/// MusicIndex release name list. The one request of each page of the
/// Releases scope "Everything" (ADR 0084 packet 016). Owner:
/// `name_lists::FetchReleaseNamePage`.
pub(crate) const INDEX_RELEASE_NAME_LIST: RequestProfile =
    RequestProfile::new(RequestPathShape::FeedNameList, Some("named_feeds"));

/// MusicIndex feed search. The `type=publisher` request of the Artists tab
/// of search (ADR 0084 packet 029). The include gives the link facts of
/// each row, so that a row shows its badge with no second request. Owner:
/// `search::fetch_index_feed_page_result_rows`.
pub(crate) const INDEX_PUBLISHER_SEARCH: RequestProfile =
    RequestProfile::new(RequestPathShape::Search, Some("link_facts"));

/// MusicIndex releases that name one feed. Each request of the singles of
/// one artist feed in the Releases scope "Everything" (ADR 0084 packet
/// 026). The include gives the named feeds of each row, so that the app
/// finds the artist feed of a row with no second request. Owner:
/// `name_lists::fetch_feed_singles`.
pub(crate) const INDEX_FEED_SINGLES: RequestProfile =
    RequestProfile::new(RequestPathShape::RecentFeedList, Some("named_feeds"));

/// MusicIndex artist names of the field list. The one request of the
/// `artist:` values after each pause of the toolbar search field (ADR 0087
/// Decision 3, packet 005). It sends `from` and `limit=20`. The include
/// gives the link facts, so a feed that is not an artist gives no name.
/// Owner: `name_lists::FetchArtistNameValues`.
pub(crate) const INDEX_FIELD_LIST_ARTIST_NAMES: RequestProfile =
    RequestProfile::new(RequestPathShape::ArtistNameList, Some("link_facts"));

#[cfg(test)]
mod tests {
    use super::*;

    /// All ten registry profiles, in the packet's table order. Kept
    /// test-only: nothing in production code enumerates the registry
    /// today. Packet 018 is the first production consumer of a profile
    /// list. The two inspector track detail profiles are gone from this
    /// list: their only caller was the parked Discover inspector, which
    /// ADR 0060 removed. ADR 0084 packet 016 adds the artist name list
    /// profile and the release name list profile. ADR 0084 packet 029 adds
    /// the feed search profile. ADR 0084 packet 026 adds the feed singles
    /// profile.
    const ALL: [RequestProfile; 12] = [
        LIBRARY_TRACK_DETAIL_SCOPED_TRACK,
        LIBRARY_TRACK_DETAIL_UNSCOPED_TRACK,
        LIBRARY_TRACK_DETAIL_FEED,
        LIBRARY_FEED_UPDATE_FEED,
        LIBRARY_ALBUM_HYDRATION_FEED,
        INDEX_FEED_DETAIL,
        INDEX_TRACK_DETAIL_SCOPED,
        INDEX_TRACK_DETAIL_UNSCOPED,
        INDEX_ARTIST_NAME_LIST,
        INDEX_RELEASE_NAME_LIST,
        INDEX_PUBLISHER_SEARCH,
        INDEX_FEED_SINGLES,
    ];

    /// R17-01: the registry exposes one named profile for each table row,
    /// naming its path shape and its include list. Position is proven at
    /// each owning call site (R17-04, R17-05, R17-06, R17-07), where the
    /// actual request order and fallback are the mechanical fact.
    #[test]
    fn adr_0075_request_profile_registry_names_twelve_profiles() {
        assert_eq!(
            ALL.len(),
            12,
            "one named profile for each surviving table row"
        );
        assert_eq!(
            LIBRARY_TRACK_DETAIL_SCOPED_TRACK.path_shape(),
            RequestPathShape::ScopedTrack
        );
        assert_eq!(
            LIBRARY_TRACK_DETAIL_UNSCOPED_TRACK.path_shape(),
            RequestPathShape::UnscopedTrack
        );
        assert_eq!(
            LIBRARY_TRACK_DETAIL_FEED.path_shape(),
            RequestPathShape::Feed
        );
        assert_eq!(
            LIBRARY_FEED_UPDATE_FEED.path_shape(),
            RequestPathShape::Feed
        );
        assert_eq!(
            LIBRARY_ALBUM_HYDRATION_FEED.path_shape(),
            RequestPathShape::Feed
        );
        assert_eq!(INDEX_FEED_DETAIL.path_shape(), RequestPathShape::Feed);
        assert_eq!(
            INDEX_TRACK_DETAIL_SCOPED.path_shape(),
            RequestPathShape::ScopedTrack
        );
        assert_eq!(
            INDEX_TRACK_DETAIL_UNSCOPED.path_shape(),
            RequestPathShape::UnscopedTrack
        );
    }

    /// R52-01 and R52-02 (ADR 0075, amendment of 2026-10-07): each feed
    /// profile sends every feed collection, and each track profile sends
    /// every track collection. The two name level profiles of packet 017
    /// send no include. The literals are retyped from the
    /// stored contract, so the comparison is not circular.
    #[test]
    fn adr_0075_r52_profiles_send_full_include_lists() {
        const RECORDED_FEED: &str = "tracks,source_enclosures,source_links,source_ids,source_release_claims,source_contributors,source_platforms,payment_routes,remote_items,publisher";
        const RECORDED_TRACK: &str = "payment_routes,value_time_splits,source_links,source_ids,source_contributors,source_release_claims,source_enclosures,source_transcripts,remote_items,publisher";

        for profile in [
            LIBRARY_TRACK_DETAIL_FEED,
            LIBRARY_FEED_UPDATE_FEED,
            LIBRARY_ALBUM_HYDRATION_FEED,
            INDEX_FEED_DETAIL,
            INDEX_PUBLISHER_PAGE,
        ] {
            assert_eq!(profile.include(), Some(RECORDED_FEED));
        }
        for profile in [
            LIBRARY_TRACK_DETAIL_SCOPED_TRACK,
            LIBRARY_TRACK_DETAIL_UNSCOPED_TRACK,
            INDEX_TRACK_DETAIL_SCOPED,
            INDEX_TRACK_DETAIL_UNSCOPED,
        ] {
            assert_eq!(profile.include(), Some(RECORDED_TRACK));
        }
        assert_eq!(INDEX_NAME_LEVEL_RELEASES.include(), None);
        assert_eq!(INDEX_ARTIST_NAME_CHECK.include(), None);
    }

    /// R17-09: a profile's own value is its identity. `RequestProfile` is
    /// `Copy`, `PartialEq`, `Eq`, and `Hash`, so equal requests (same path
    /// shape and include list) produce equal identities, and a different
    /// include list produces a different identity. Packet 018 can use a
    /// profile value directly as a cache key component.
    #[test]
    fn adr_0075_request_profile_value_is_a_stable_identity() {
        // Equal requests produce equal identities. Two different named
        // profiles that request the same feed path shape with the same
        // include list are, correctly, one identity.
        assert_eq!(LIBRARY_TRACK_DETAIL_FEED, LIBRARY_FEED_UPDATE_FEED);
        assert_eq!(INDEX_TRACK_DETAIL_SCOPED, INDEX_TRACK_DETAIL_SCOPED);

        // Profiles with one path shape and one full include list are one
        // identity, so they share one cache entry (ADR 0075, amendment of
        // 2026-10-07).
        assert_eq!(LIBRARY_TRACK_DETAIL_FEED, INDEX_FEED_DETAIL);
        assert_eq!(LIBRARY_ALBUM_HYDRATION_FEED, INDEX_PUBLISHER_PAGE);

        // A different path shape produces a different identity, even with
        // the same include list.
        assert_ne!(INDEX_TRACK_DETAIL_SCOPED, INDEX_TRACK_DETAIL_UNSCOPED);
    }

    /// R16-04 (ADR 0084 packet 016): the two name list profiles name their
    /// path shape and their include list. The literals are retyped from
    /// `GET /v1/node/capabilities` of 2026-10-20.
    #[test]
    fn adr_0084_scopes_r16_04_name_list_profiles() {
        assert_eq!(
            INDEX_ARTIST_NAME_LIST.path_shape(),
            RequestPathShape::ArtistNameList
        );
        assert_eq!(INDEX_ARTIST_NAME_LIST.include(), Some("link_facts"));
        assert_eq!(
            INDEX_RELEASE_NAME_LIST.path_shape(),
            RequestPathShape::FeedNameList
        );
        assert_eq!(INDEX_RELEASE_NAME_LIST.include(), Some("named_feeds"));
    }

    /// ADR 0084 packet 017: the name level profile requests the release
    /// name list path shape, and the name check profile requests the
    /// artist name list path shape. Neither sends an include.
    #[test]
    fn adr_0084_name_level_profiles_name_their_path_shapes() {
        assert_eq!(
            INDEX_NAME_LEVEL_RELEASES.path_shape(),
            RequestPathShape::FeedNameList
        );
        assert_eq!(
            INDEX_ARTIST_NAME_CHECK.path_shape(),
            RequestPathShape::ArtistNameList
        );
    }

    /// R29-04 (ADR 0084 packet 029): the feed search profile names the
    /// search path shape and the include `link_facts`. The literal is
    /// retyped from `GET /v1/node/capabilities` of 2026-10-21.
    #[test]
    fn adr_0084_search_link_facts_r29_04_publisher_search_profile() {
        assert_eq!(
            INDEX_PUBLISHER_SEARCH.path_shape(),
            RequestPathShape::Search
        );
        assert_eq!(INDEX_PUBLISHER_SEARCH.include(), Some("link_facts"));
    }

    /// R26-06 (ADR 0084 packet 026): the feed singles profile names the
    /// recent feed list path shape and the include `named_feeds`.
    #[test]
    fn adr_0084_index_singles_r26_06_feed_singles_profile() {
        assert_eq!(
            INDEX_FEED_SINGLES.path_shape(),
            RequestPathShape::RecentFeedList
        );
        assert_eq!(INDEX_FEED_SINGLES.include(), Some("named_feeds"));
    }

    /// R87-5-17 (ADR 0087 packet 005): the field list profile names the
    /// artist name list path shape and the include `link_facts`. It stays
    /// out of the registry list, as the name check profile does.
    #[test]
    fn adr_0087_field_values_r87_5_17_field_list_profile() {
        assert_eq!(
            INDEX_FIELD_LIST_ARTIST_NAMES.path_shape(),
            RequestPathShape::ArtistNameList
        );
        assert_eq!(INDEX_FIELD_LIST_ARTIST_NAMES.include(), Some("link_facts"));
        assert_eq!(ALL.len(), 12);
    }
}
