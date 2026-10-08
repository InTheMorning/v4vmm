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
    /// `/v1/tracks?artist={name}` (ADR 0077 packet 006). This shape
    /// matches a name query, never a track GUID.
    TracksByArtistName,
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

/// Index name-match tracks. The one request of the Index name-match track
/// page (ADR 0077 packet 006, Accepted Refinement "Index artist page by
/// name"). Owner: `search::fetch_name_match_tracks`.
pub(crate) const INDEX_NAME_MATCH_TRACKS: RequestProfile =
    RequestProfile::new(RequestPathShape::TracksByArtistName, None);

#[cfg(test)]
mod tests {
    use super::*;

    /// All eight registry profiles, in the packet's table order. Kept
    /// test-only: nothing in production code enumerates the registry
    /// today. Packet 018 is the first production consumer of a profile
    /// list. The two inspector track detail profiles are gone from this
    /// list: their only caller was the parked Discover inspector, which
    /// ADR 0060 removed.
    const ALL: [RequestProfile; 8] = [
        LIBRARY_TRACK_DETAIL_SCOPED_TRACK,
        LIBRARY_TRACK_DETAIL_UNSCOPED_TRACK,
        LIBRARY_TRACK_DETAIL_FEED,
        LIBRARY_FEED_UPDATE_FEED,
        LIBRARY_ALBUM_HYDRATION_FEED,
        INDEX_FEED_DETAIL,
        INDEX_TRACK_DETAIL_SCOPED,
        INDEX_TRACK_DETAIL_UNSCOPED,
    ];

    /// R17-01: the registry exposes one named profile for each table row,
    /// naming its path shape and its include list. Position is proven at
    /// each owning call site (R17-04, R17-05, R17-06, R17-07), where the
    /// actual request order and fallback are the mechanical fact.
    #[test]
    fn adr_0075_request_profile_registry_names_eight_profiles() {
        assert_eq!(
            ALL.len(),
            8,
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
    /// every track collection. The name-match profile sends no include,
    /// because `/v1/tracks` takes none. The literals are retyped from the
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
        assert_eq!(INDEX_NAME_MATCH_TRACKS.include(), None);
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

    /// R6-03 (ADR 0077 packet 006): `INDEX_NAME_MATCH_TRACKS` requests the
    /// tracks-by-artist-name path shape with no include list.
    #[test]
    fn adr_0077_name_matches_index_name_match_tracks_profile_matches_recorded_literal() {
        assert_eq!(
            INDEX_NAME_MATCH_TRACKS.path_shape(),
            RequestPathShape::TracksByArtistName
        );
        assert_eq!(INDEX_NAME_MATCH_TRACKS.include(), None);
    }
}
