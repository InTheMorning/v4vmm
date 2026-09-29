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
//! The ten named constants below match the ten rows of packet 017's
//! required-profile table. A constant's name is its name; the module adds
//! no separate name field or name type.

/// The path shape one request profile requests.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum RequestPathShape {
    /// `/v1/feeds/{feed_guid}`
    Feed,
    /// `/v1/feeds/{feed_guid}/tracks/{item_guid}`
    ScopedTrack,
    /// `/v1/tracks/{item_guid}`
    UnscopedTrack,
    /// Scoped when the caller supplies a feed GUID, else unscoped. The
    /// Index track detail profiles name this same choice as two separate
    /// profiles. The inspector track detail profile names it as one
    /// profile with one include list.
    ScopedOrUnscopedTrack,
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
    /// An L0 profile returns `None`. The caller then sends no `include`
    /// query parameter.
    pub(crate) const fn include(&self) -> Option<&'static str> {
        self.include
    }
}

/// L1. Five collections. Used by the three Library track detail profiles
/// and the Library feed update profile.
const L1: &str = "source_links,source_ids,source_release_claims,source_contributors,payment_routes";
/// L2 and `publisher`. Eight collections. Used by the Index feed detail
/// profile. ADR 0077 Decision 5 added `publisher` to L2.
const L2_PUBLISHER: &str = "tracks,source_enclosures,source_links,source_ids,source_release_claims,source_contributors,payment_routes,publisher";
/// L3. Six collections. Used by the inspector track detail profile's
/// track request.
const L3: &str =
    "source_enclosures,source_links,source_ids,source_release_claims,source_contributors,payment_routes";
/// L4. Six collections. Used by the inspector track detail profile's
/// feed request.
const L4: &str =
    "tracks,source_enclosures,source_links,source_ids,source_release_claims,payment_routes";
/// L5 and `publisher`. Five collections. Used by the Library album
/// hydration profile. ADR 0077 Decision 5 added `publisher` to L5.
const L5_PUBLISHER: &str =
    "source_links,source_ids,source_release_claims,source_contributors,publisher";

/// Library track detail, scoped track. First. Used only when the row has
/// a feed GUID. Owner: `feed_service::fetch_library_track_detail_with_recorder`.
pub(crate) const LIBRARY_TRACK_DETAIL_SCOPED_TRACK: RequestProfile =
    RequestProfile::new(RequestPathShape::ScopedTrack, Some(L1));

/// Library track detail, unscoped track. Second. Used only when the first
/// request returns no track. Owner: the same function.
pub(crate) const LIBRARY_TRACK_DETAIL_UNSCOPED_TRACK: RequestProfile =
    RequestProfile::new(RequestPathShape::UnscopedTrack, Some(L1));

/// Library track detail, feed. Third. Owner: the same function.
pub(crate) const LIBRARY_TRACK_DETAIL_FEED: RequestProfile =
    RequestProfile::new(RequestPathShape::Feed, Some(L1));

/// Library feed update, feed. The only request of `feed_service::apply_feed_updates`.
pub(crate) const LIBRARY_FEED_UPDATE_FEED: RequestProfile =
    RequestProfile::new(RequestPathShape::Feed, Some(L1));

/// Library album hydration, feed. The only request of
/// `library::hydrate_album_identity_facts`.
pub(crate) const LIBRARY_ALBUM_HYDRATION_FEED: RequestProfile =
    RequestProfile::new(RequestPathShape::Feed, Some(L5_PUBLISHER));

/// Index feed detail. The only request of each of its three call sites:
/// `search::fetch_index_feed_result_rows`, `feed::fetch_recent_feed_result_rows`,
/// and `feed::fetch_feed_detail`.
pub(crate) const INDEX_FEED_DETAIL: RequestProfile =
    RequestProfile::new(RequestPathShape::Feed, Some(L2_PUBLISHER));

/// Index track detail, scoped. Used when the hit has a nonempty feed GUID.
/// Owner: `search::fetch_index_track_detail`.
pub(crate) const INDEX_TRACK_DETAIL_SCOPED: RequestProfile =
    RequestProfile::new(RequestPathShape::ScopedTrack, None);

/// Index track detail, unscoped. Used when the hit has no feed GUID.
/// Owner: the same function.
pub(crate) const INDEX_TRACK_DETAIL_UNSCOPED: RequestProfile =
    RequestProfile::new(RequestPathShape::UnscopedTrack, None);

/// Inspector track detail, track. First. Owner: `feed::fetch_track_detail`,
/// through `fetch_scoped_track`.
pub(crate) const INSPECTOR_TRACK_DETAIL_TRACK: RequestProfile =
    RequestProfile::new(RequestPathShape::ScopedOrUnscopedTrack, Some(L3));

/// Inspector track detail, feed. Second. A failure returns no feed and
/// stops no request. Owner: `feed::fetch_track_detail`.
pub(crate) const INSPECTOR_TRACK_DETAIL_FEED: RequestProfile =
    RequestProfile::new(RequestPathShape::Feed, Some(L4));

/// Index publisher page. The one request of the Index and the Library
/// publisher page queries (ADR 0077 Task 003). Owners:
/// `feed::fetch_index_publisher_page_albums` and
/// `library::fetch_library_publisher_page`.
pub(crate) const INDEX_PUBLISHER_PAGE: RequestProfile =
    RequestProfile::new(RequestPathShape::Feed, Some("publisher"));

/// Index name-match tracks. The one request of the Index name-match track
/// page (ADR 0077 packet 006, Accepted Refinement "Index artist page by
/// name"). Owner: `search::fetch_name_match_tracks`.
pub(crate) const INDEX_NAME_MATCH_TRACKS: RequestProfile =
    RequestProfile::new(RequestPathShape::TracksByArtistName, None);

#[cfg(test)]
mod tests {
    use super::*;

    /// All ten registry profiles, in the packet's table order. Kept
    /// test-only: nothing in production code enumerates the registry
    /// today. Packet 018 is the first production consumer of a profile
    /// list.
    const ALL: [RequestProfile; 10] = [
        LIBRARY_TRACK_DETAIL_SCOPED_TRACK,
        LIBRARY_TRACK_DETAIL_UNSCOPED_TRACK,
        LIBRARY_TRACK_DETAIL_FEED,
        LIBRARY_FEED_UPDATE_FEED,
        LIBRARY_ALBUM_HYDRATION_FEED,
        INDEX_FEED_DETAIL,
        INDEX_TRACK_DETAIL_SCOPED,
        INDEX_TRACK_DETAIL_UNSCOPED,
        INSPECTOR_TRACK_DETAIL_TRACK,
        INSPECTOR_TRACK_DETAIL_FEED,
    ];

    /// R17-01: the registry exposes one named profile for each table row,
    /// naming its path shape and its include list. Position is proven at
    /// each owning call site (R17-04, R17-05, R17-06, R17-07), where the
    /// actual request order and fallback are the mechanical fact.
    #[test]
    fn adr_0075_request_profile_registry_names_ten_profiles() {
        assert_eq!(ALL.len(), 10, "one named profile for each table row");
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
        assert_eq!(
            INSPECTOR_TRACK_DETAIL_TRACK.path_shape(),
            RequestPathShape::ScopedOrUnscopedTrack
        );
        assert_eq!(
            INSPECTOR_TRACK_DETAIL_FEED.path_shape(),
            RequestPathShape::Feed
        );
    }

    /// R17-02: each profile's include string equals its recorded literal,
    /// character for character. These constants are retyped from the
    /// packet's literal table, independent of this module's own `L1`
    /// through `L5` constants, so the comparison is not circular.
    ///
    /// ADR 0077 Decision 5 changes this test on purpose. The Library album
    /// hydration sends L5 and `publisher`, and the Index feed detail sends
    /// L2 and `publisher` (packet ADR 0077 002, R2-04).
    #[test]
    fn adr_0075_request_profile_include_strings_match_recorded_literals() {
        const RECORDED_L1: &str =
            "source_links,source_ids,source_release_claims,source_contributors,payment_routes";
        const RECORDED_L2: &str = "tracks,source_enclosures,source_links,source_ids,source_release_claims,source_contributors,payment_routes";
        const RECORDED_L3: &str = "source_enclosures,source_links,source_ids,source_release_claims,source_contributors,payment_routes";
        const RECORDED_L4: &str =
            "tracks,source_enclosures,source_links,source_ids,source_release_claims,payment_routes";
        const RECORDED_L5: &str =
            "source_links,source_ids,source_release_claims,source_contributors";
        let recorded_l2_publisher = format!("{RECORDED_L2},publisher");
        let recorded_l5_publisher = format!("{RECORDED_L5},publisher");

        assert_eq!(
            LIBRARY_TRACK_DETAIL_SCOPED_TRACK.include(),
            Some(RECORDED_L1)
        );
        assert_eq!(
            LIBRARY_TRACK_DETAIL_UNSCOPED_TRACK.include(),
            Some(RECORDED_L1)
        );
        assert_eq!(LIBRARY_TRACK_DETAIL_FEED.include(), Some(RECORDED_L1));
        assert_eq!(LIBRARY_FEED_UPDATE_FEED.include(), Some(RECORDED_L1));
        assert_eq!(
            LIBRARY_ALBUM_HYDRATION_FEED.include(),
            Some(recorded_l5_publisher.as_str()),
            "ADR 0077 Decision 5: the Library album hydration sends L5 and publisher"
        );
        assert_eq!(
            INDEX_FEED_DETAIL.include(),
            Some(recorded_l2_publisher.as_str()),
            "ADR 0077 Decision 5: the Index feed detail sends L2 and publisher"
        );
        assert_eq!(INDEX_TRACK_DETAIL_SCOPED.include(), None);
        assert_eq!(INDEX_TRACK_DETAIL_UNSCOPED.include(), None);
        assert_eq!(INSPECTOR_TRACK_DETAIL_TRACK.include(), Some(RECORDED_L3));
        assert_eq!(INSPECTOR_TRACK_DETAIL_FEED.include(), Some(RECORDED_L4));
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

        // A different include list produces a different identity.
        assert_ne!(LIBRARY_TRACK_DETAIL_FEED, INDEX_FEED_DETAIL);
        assert_ne!(LIBRARY_ALBUM_HYDRATION_FEED, LIBRARY_TRACK_DETAIL_FEED);

        // A different path shape produces a different identity, even with
        // the same include list.
        assert_ne!(INDEX_TRACK_DETAIL_SCOPED, INDEX_TRACK_DETAIL_UNSCOPED);
    }

    /// R17-12: the registry reports the collection count of each profile.
    ///
    /// ADR 0077 Decision 5 changes this test on purpose. `publisher` makes
    /// the Library album hydration five collections and the Index feed
    /// detail eight collections.
    #[test]
    fn adr_0075_request_profile_reports_collection_counts() {
        fn collection_count(profile: &RequestProfile) -> usize {
            profile
                .include
                .map_or(0, |include| include.split(',').count())
        }

        assert_eq!(collection_count(&LIBRARY_TRACK_DETAIL_SCOPED_TRACK), 5);
        assert_eq!(collection_count(&LIBRARY_TRACK_DETAIL_UNSCOPED_TRACK), 5);
        assert_eq!(collection_count(&LIBRARY_TRACK_DETAIL_FEED), 5);
        assert_eq!(collection_count(&LIBRARY_FEED_UPDATE_FEED), 5);
        assert_eq!(collection_count(&LIBRARY_ALBUM_HYDRATION_FEED), 5);
        assert_eq!(collection_count(&INDEX_FEED_DETAIL), 8);
        assert_eq!(collection_count(&INDEX_TRACK_DETAIL_SCOPED), 0);
        assert_eq!(collection_count(&INDEX_TRACK_DETAIL_UNSCOPED), 0);
        assert_eq!(collection_count(&INSPECTOR_TRACK_DETAIL_TRACK), 6);
        assert_eq!(collection_count(&INSPECTOR_TRACK_DETAIL_FEED), 6);
    }

    /// R2-04 (ADR 0077 Decision 5): the Library album hydration sends L5
    /// and `publisher`, and the Index feed detail sends L2 and `publisher`.
    /// No other profile requests `publisher`, and each other include list
    /// keeps its packet 017 literal.
    #[test]
    fn adr_0077_publisher_relationship_profiles_add_publisher_to_two_include_lists() {
        assert_eq!(
            LIBRARY_ALBUM_HYDRATION_FEED.include(),
            Some("source_links,source_ids,source_release_claims,source_contributors,publisher")
        );
        assert_eq!(
            INDEX_FEED_DETAIL.include(),
            Some("tracks,source_enclosures,source_links,source_ids,source_release_claims,source_contributors,payment_routes,publisher")
        );
        let unchanged = [
            (
                LIBRARY_TRACK_DETAIL_SCOPED_TRACK,
                Some("source_links,source_ids,source_release_claims,source_contributors,payment_routes"),
            ),
            (
                LIBRARY_TRACK_DETAIL_UNSCOPED_TRACK,
                Some("source_links,source_ids,source_release_claims,source_contributors,payment_routes"),
            ),
            (
                LIBRARY_TRACK_DETAIL_FEED,
                Some("source_links,source_ids,source_release_claims,source_contributors,payment_routes"),
            ),
            (
                LIBRARY_FEED_UPDATE_FEED,
                Some("source_links,source_ids,source_release_claims,source_contributors,payment_routes"),
            ),
            (INDEX_TRACK_DETAIL_SCOPED, None),
            (INDEX_TRACK_DETAIL_UNSCOPED, None),
            (
                INSPECTOR_TRACK_DETAIL_TRACK,
                Some("source_enclosures,source_links,source_ids,source_release_claims,source_contributors,payment_routes"),
            ),
            (
                INSPECTOR_TRACK_DETAIL_FEED,
                Some("tracks,source_enclosures,source_links,source_ids,source_release_claims,payment_routes"),
            ),
        ];
        for (profile, include) in unchanged {
            assert_eq!(profile.include(), include, "{profile:?} must not change");
        }
        let requesting = ALL
            .iter()
            .filter(|profile| {
                profile
                    .include()
                    .is_some_and(|include| include.split(',').any(|c| c == "publisher"))
            })
            .count();
        assert_eq!(requesting, 2, "only two profiles request publisher");
    }

    /// R3-01 (ADR 0077 Task 003): `INDEX_PUBLISHER_PAGE` requests the feed
    /// path shape with only `publisher` in its include list.
    #[test]
    fn adr_0077_publisher_page_index_publisher_page_profile_matches_recorded_literal() {
        assert_eq!(INDEX_PUBLISHER_PAGE.path_shape(), RequestPathShape::Feed);
        assert_eq!(INDEX_PUBLISHER_PAGE.include(), Some("publisher"));
    }

    /// R6-03 (ADR 0077 packet 006): `INDEX_NAME_MATCH_TRACKS` requests the
    /// tracks-by-artist-name path shape with no include list (L0).
    #[test]
    fn adr_0077_name_matches_index_name_match_tracks_profile_matches_recorded_literal() {
        assert_eq!(
            INDEX_NAME_MATCH_TRACKS.path_shape(),
            RequestPathShape::TracksByArtistName
        );
        assert_eq!(INDEX_NAME_MATCH_TRACKS.include(), None);
    }

    /// R17-13: no profile requests `source_transcripts`. A later addition
    /// must change this test on purpose.
    #[test]
    fn adr_0075_request_profile_none_request_source_transcripts() {
        for profile in ALL {
            if let Some(include) = profile.include() {
                assert!(
                    !include
                        .split(',')
                        .any(|collection| collection == "source_transcripts"),
                    "profile {profile:?} must not request source_transcripts"
                );
            }
        }
    }
}
