//! The Library facts of the search fields that SQL does not hold (ADR 0087
//! Decision 2, packet 002).
//!
//! The release kind reads a stated value of the feed metadata facts, and
//! `pub_date` is RFC 2822 text. Thus the Library search reads these facts
//! from the Library tree. The release date of a Library release is the
//! latest `pub_date` of its tracks (ADR 0086 Decision 1). The ready set is
//! the set of the tracks with the state `Ready` in the last readiness report
//! (packet 002 decision D10).
//!
//! The module has no GPUI type and no I/O.

#![warn(clippy::pedantic)]

use std::collections::{BTreeMap, BTreeSet};

use crate::view_models::library::LibraryTree;
use crate::view_models::release_kind::ReleaseKind;

/// The search facts of one Library track.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LibraryTrackSearchFacts {
    /// The kind of the release of the track (ADR 0084 Decision 5).
    pub(crate) release_kind: ReleaseKind,
    /// The latest `pub_date` of the tracks of the release, in Unix seconds.
    pub(crate) release_date: Option<i64>,
    /// The `pub_date` of the track, in Unix seconds.
    pub(crate) pub_date: Option<i64>,
}

/// The search facts of each Library track, and the ready set, for one run.
#[derive(Clone, Debug, Default)]
pub(crate) struct LibrarySearchFacts {
    tracks: BTreeMap<i64, LibraryTrackSearchFacts>,
    ready: Option<BTreeSet<i64>>,
}

impl LibrarySearchFacts {
    /// Gives the facts of each track of each release of `tree`. `ready` is
    /// the ready set of the last readiness report, or `None` when no report
    /// is available.
    #[must_use]
    pub(crate) fn from_tree(tree: &LibraryTree, ready: Option<&BTreeSet<i64>>) -> Self {
        let mut tracks = BTreeMap::new();
        for album in tree.artists.iter().flat_map(|artist| artist.albums.iter()) {
            let release_kind = album.release_kind();
            let release_date = album.release_date();
            for track in &album.tracks {
                tracks.insert(
                    track.id,
                    LibraryTrackSearchFacts {
                        release_kind,
                        release_date,
                        pub_date: track.pub_date,
                    },
                );
            }
        }
        Self {
            tracks,
            ready: ready.cloned(),
        }
    }

    /// The facts of each track, in id order.
    pub(crate) fn tracks(&self) -> impl Iterator<Item = (i64, &LibraryTrackSearchFacts)> {
        self.tracks.iter().map(|(id, facts)| (*id, facts))
    }

    /// The ready set of the last readiness report, or `None` when no report
    /// is available.
    #[must_use]
    pub(crate) fn ready_track_ids(&self) -> Option<&BTreeSet<i64>> {
        self.ready.as_ref()
    }
}

/// The facts of one track have no production caller yet. Packet 003 reads
/// them.
#[cfg(test)]
impl LibrarySearchFacts {
    /// The facts of the track `id`, or `None` for a track that is not in
    /// the tree.
    #[must_use]
    pub(crate) fn track(&self, id: i64) -> Option<&LibraryTrackSearchFacts> {
        self.tracks.get(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::TrackRow;
    use crate::view_models::library::{AlbumNode, ArtistNode};
    use crate::views::{FeedMetadataFacts, LocalIdentityFacts};

    fn track(id: i64, pub_date: Option<i64>) -> TrackRow {
        TrackRow {
            id,
            feed_id: 1,
            item_guid: format!("item-{id}"),
            pub_date,
            is_in_library: true,
            ..TrackRow::default()
        }
    }

    fn album(name: &str, tracks: Vec<TrackRow>, stated_kind: Option<&str>) -> AlbumNode {
        AlbumNode {
            name: name.to_owned(),
            feed_id: None,
            feed_guid: None,
            feed_url: None,
            language: None,
            description: None,
            image_href: None,
            named_feeds: Vec::new(),
            identity_facts: LocalIdentityFacts::default(),
            metadata_facts: Box::new(FeedMetadataFacts {
                release_kind: stated_kind.map(str::to_owned),
                ..FeedMetadataFacts::default()
            }),
            stored_values: None,
            tracks,
        }
    }

    /// R87-2-27: the stated kind wins, else the count rule gives the kind.
    /// The release date is the latest `pub_date` of the album. The ready set
    /// is the given set.
    #[test]
    fn adr_0087_library_fields_r87_2_27_facts_come_from_the_tree() {
        let tree = LibraryTree {
            artists: vec![ArtistNode {
                name: "Artist".to_owned(),
                albums: vec![
                    album("Stated", vec![track(1, Some(10))], Some("EP")),
                    album("Counted", vec![track(2, Some(10))], None),
                    album(
                        "Album",
                        vec![track(3, Some(30)), track(4, None), track(5, Some(20))],
                        None,
                    ),
                ],
            }],
        };
        let ready = BTreeSet::from([4]);

        let facts = LibrarySearchFacts::from_tree(&tree, Some(&ready));

        assert_eq!(
            facts.track(1).map(|f| f.release_kind),
            Some(ReleaseKind::Ep)
        );
        assert!(facts.track(2).is_some_and(|f| f.release_kind.is_single()));
        let album_track = facts.track(4).copied();
        assert_eq!(
            album_track,
            Some(LibraryTrackSearchFacts {
                release_kind: ReleaseKind::Album,
                release_date: Some(30),
                pub_date: None,
            })
        );
        assert_eq!(facts.track(5).and_then(|f| f.pub_date), Some(20));
        assert_eq!(facts.ready_track_ids(), Some(&ready));
        assert_eq!(facts.tracks().count(), 5);

        let no_report = LibrarySearchFacts::from_tree(&tree, None);
        assert_eq!(no_report.ready_track_ids(), None);
    }
}
