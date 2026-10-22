//! The singles groups of the Library and of the scope "Everything" (ADR
//! 0084 Decision 6).
//!
//! Two or more singles of one artist identity (ADR 0084 Decision 7) make
//! one group. This module owns the functions that group singles:
//! [`singles_groups`] for the Library, and [`singles_groups_everything`] for
//! the Releases scope "Everything", which adds the `MusicIndex` singles of
//! each artist feed (packet 026). The Releases list, the group page and the
//! link on a single page each use them. The group page and the link are not
//! scoped (packet 026 decisions D12 and D14): they always read the groups of
//! "Everything". No other code groups releases by kind.

#![warn(clippy::pedantic)]

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use crate::api::Feed;
use crate::db::TrackRow;
use crate::view_models::album_page::{AlbumNameLinkTarget, AlbumNameLinkVm};
use crate::view_models::artist_identity::{ArtistIdentity, NameFeedMatch, UNKNOWN_ARTIST_TEXT};
use crate::view_models::collection_art::{CollectionArtVm, CollectionKind};
use crate::view_models::format::{fmt_date, plural};
use crate::view_models::index_feed_singles::{
    feed_singles_request, FeedSinglesState, IndexFeedSingles,
};
use crate::view_models::index_name_list::{
    name_sort_key, IndexListActionDisplay, IndexListStatusRowVm,
};
use crate::view_models::library::{
    tree_name_feed_matches, AlbumNode, ArtistNode, BusyItems, DestinationListRowVm, LibraryTree,
    TrackRowWork,
};
use crate::view_models::list_selection::RowSelectionVm;
use crate::view_models::named_feeds::{index_named_feeds, NamedFeed};
use crate::view_models::publisher_page::PublisherPageVm;
use crate::view_models::release_kind::ReleaseKind;
use crate::view_models::releases_everything::{
    index_release_row, index_release_title, ReleasesDownloadPlan,
};
use crate::view_models::track::fmt_dur;
use crate::view_models::workspace::ReleaseKey;
use crate::views::FeedView;

/// The separator between the count and the Library state of a group.
const SUMMARY_SEPARATOR: &str = " · ";
/// The Library state of a group when each single is in the Library.
const ALL_IN_LIBRARY_LABEL: &str = "In Library";
/// The accessibility label of a closed group row chevron.
const SHOW_SINGLES_LABEL: &str = "Show singles";
/// The accessibility label of an open group row chevron.
const HIDE_SINGLES_LABEL: &str = "Hide singles";
/// The note of a group at the name level (ADR 0084 Decision 7).
const NAME_LEVEL_NOTE: &str = "Grouped by the artist name. A name is not an identity.";
/// The note of a group at the feed level (ADR 0084 Decision 7, ADR 0085
/// Decision 1).
const PUBLISHER_LEVEL_NOTE: &str = "Grouped by the artist feed of each single.";
/// The identity level of a group at the name level, in its summary text.
const NAME_LEVEL_LABEL: &str = "By name";
/// The identity level of a group at the feed level, in its summary text
/// (ADR 0085 Decision 1).
const PUBLISHER_LEVEL_LABEL: &str = "Artist feed";
/// The empty state of a group page with fewer than two singles. The page is
/// not scoped (packet 026 decision D12).
const EMPTY_GROUP_MESSAGE: &str = "This artist has fewer than two singles.";
/// The row of a group page while the request for the `MusicIndex` singles
/// of its feed runs (packet 026 Part B).
const LOADING_INDEX_SINGLES_LABEL: &str = "Loading MusicIndex singles";
/// The report of the error row of a group page.
const INDEX_SINGLES_FAILED_MESSAGE: &str =
    "The MusicIndex singles did not load. The Library singles show.";
/// The accessibility label of "Try again" on the error row of a group page.
const RETRY_INDEX_SINGLES_A11Y_LABEL: &str =
    "Try again to load the MusicIndex singles of this artist feed";
/// The note of a group page when more releases name the feed than the app
/// read (packet 026 decision D11).
const MORE_INDEX_SINGLES_NOTE: &str =
    "MusicIndex has more releases of this artist. This page shows the singles of its 1,000 newest releases.";
/// The title of a group page with no group.
const EMPTY_GROUP_TITLE: &str = "Singles";

/// The key of a singles group page (ADR 0084 Decision 6). It holds no
/// label.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SinglesGroupKey(pub ArtistIdentity);

/// One single of a group.
#[derive(Clone, Debug)]
pub(crate) struct GroupSingle<'a> {
    /// The artist node that holds the single.
    pub(crate) artist: &'a ArtistNode,
    /// The single.
    pub(crate) album: &'a AlbumNode,
    /// The release key of the single.
    pub(crate) key: ReleaseKey,
    /// The latest `pub_date` of its tracks.
    pub(crate) date: Option<i64>,
    /// Each track of the single is in the Library.
    pub(crate) in_library: bool,
}

/// One `MusicIndex` single of a group in the scope "Everything" (ADR 0084
/// packet 026). The Library does not hold it.
#[derive(Clone, Debug)]
pub(crate) struct IndexGroupSingle<'a> {
    /// The release row of `GET /v1/feeds/recent?named_feed_guid=<guid>`.
    pub(crate) feed: &'a Feed,
    /// The release key of the single.
    pub(crate) key: ReleaseKey,
    /// `newest_item_at` of the release.
    pub(crate) date: Option<i64>,
    /// The sort key of the title.
    pub(crate) sort_key: String,
}

/// One member of a group: a Library single or a `MusicIndex` single.
#[derive(Clone, Copy, Debug)]
pub(crate) enum GroupMember<'g, 'a> {
    /// A Library single.
    Library(&'g GroupSingle<'a>),
    /// A `MusicIndex` single (packet 026).
    Index(&'g IndexGroupSingle<'a>),
}

impl GroupMember<'_, '_> {
    /// The release key of the member.
    #[must_use]
    pub(crate) fn key(&self) -> &ReleaseKey {
        match self {
            Self::Library(single) => &single.key,
            Self::Index(single) => &single.key,
        }
    }

    /// The date of the member: the latest `pub_date` of the tracks of a
    /// Library single, or `newest_item_at` of a `MusicIndex` single.
    fn date(&self) -> Option<i64> {
        match self {
            Self::Library(single) => single.date,
            Self::Index(single) => single.date,
        }
    }

    /// The sort key of the title of the member.
    fn sort_key(&self) -> String {
        match self {
            Self::Library(single) => name_sort_key(&single.album.name),
            Self::Index(single) => single.sort_key.clone(),
        }
    }

    /// The release image of the member.
    fn image(&self) -> Option<&str> {
        match self {
            Self::Library(single) => single.album.image_href.as_deref(),
            Self::Index(single) => single.feed.image_url.as_deref(),
        }
        .filter(|url| !url.is_empty())
    }
}

/// Two or more singles of one artist identity (ADR 0084 Decision 6),
/// newest first. A group of the scope "Library" holds Library singles
/// only. A group of an artist feed in the scope "Everything" also holds the
/// `MusicIndex` singles of that feed (packet 026).
#[derive(Clone, Debug)]
pub(crate) struct SinglesGroup<'a> {
    /// The artist identity of each single in the group.
    pub(crate) identity: ArtistIdentity,
    /// The label of the group: the artist node name of the newest Library
    /// single. A group with no Library single uses the title of the artist
    /// feed of its newest `MusicIndex` single, else the `release_artist` of
    /// that single (packet 026 decision D9).
    pub(crate) display_artist: &'a str,
    /// The Library singles, newest first.
    pub(crate) singles: Vec<GroupSingle<'a>>,
    /// The `MusicIndex` singles, newest first. Empty in the scope
    /// "Library".
    pub(crate) index_singles: Vec<IndexGroupSingle<'a>>,
    /// The Library singles of a name that matches this feed, newest
    /// first (ADR 0084 Decision 6, amended 2026-10-22, packet 032). Empty
    /// for a group of a name, and for a feed that no name matches. The
    /// group page lists them after `singles`, under "Matched by name".
    pub(crate) matched_by_name: Vec<GroupSingle<'a>>,
    /// The channel image of the artist feed of a feed-level group, from
    /// [`LibraryTree::feed_images`]. `None` for a group of a name.
    pub(crate) feed_image: Option<&'a str>,
}

impl<'a> SinglesGroup<'a> {
    /// The key of the group page.
    #[must_use]
    pub(crate) fn key(&self) -> SinglesGroupKey {
        SinglesGroupKey(self.identity.clone())
    }

    /// The number of singles in the group: the Library singles, the
    /// `MusicIndex` singles, and the singles that a matched name joins
    /// to this group (ADR 0084 Decision 6, amended 2026-10-22).
    #[must_use]
    pub(crate) fn count(&self) -> usize {
        self.singles.len() + self.index_singles.len() + self.matched_by_name.len()
    }

    /// Each member, newest first (packet 026): a member with no date comes
    /// last, and equal dates sort by the sort key of the title. A group
    /// with no `MusicIndex` single keeps the order of its Library singles.
    #[must_use]
    pub(crate) fn members(&self) -> Vec<GroupMember<'_, 'a>> {
        let mut members: Vec<GroupMember<'_, 'a>> =
            self.singles.iter().map(GroupMember::Library).collect();
        if self.index_singles.is_empty() {
            return members;
        }
        members.extend(self.index_singles.iter().map(GroupMember::Index));
        members.sort_by(|left, right| {
            newest_date_first(left.date(), right.date())
                .then_with(|| left.sort_key().cmp(&right.sort_key()))
        });
        members
    }

    /// The order position of the group row in the scope "Everything": the
    /// sort key of its first single in title order (packet 026 decision
    /// D5).
    #[must_use]
    pub(crate) fn first_title_sort_key(&self) -> String {
        self.position_members()
            .iter()
            .map(GroupMember::sort_key)
            .min()
            .unwrap_or_default()
    }

    /// The members that give the place and the art of the group row. The
    /// matched singles count only when the group has no other member, so
    /// a group of matched singles alone keeps a title and its images
    /// (packet 032 review). Else they do not move the row (decision D3).
    fn position_members(&self) -> Vec<GroupMember<'_, 'a>> {
        let members = self.members();
        if !members.is_empty() {
            return members;
        }
        self.matched_by_name
            .iter()
            .map(GroupMember::Library)
            .collect()
    }

    /// The number of singles in the Library: `singles` and
    /// `matched_by_name`, which are both Library singles.
    #[must_use]
    pub(crate) fn in_library_count(&self) -> usize {
        self.singles
            .iter()
            .chain(&self.matched_by_name)
            .filter(|single| single.in_library)
            .count()
    }

    /// The count, the Library state and the identity level, for example
    /// "7 singles · 5 in Library · By name" or "7 singles · In Library ·
    /// Artist feed". One name can have a group at each level (ADR 0084
    /// Decision 7), so the level tells the two rows apart. A group with no
    /// Library single has no Library state, for example "38 singles ·
    /// Artist feed" (packet 026 decision D9).
    #[must_use]
    pub(crate) fn summary_text(&self) -> String {
        match self.library_state_part() {
            Some(state) => format!(
                "{} singles{SUMMARY_SEPARATOR}{state}{SUMMARY_SEPARATOR}{}",
                self.count(),
                self.level_label()
            ),
            None => format!(
                "{} singles{SUMMARY_SEPARATOR}{}",
                self.count(),
                self.level_label()
            ),
        }
    }

    /// The Library state of [`Self::summary_text`]. `None` for a group with
    /// no Library single (packet 026 decision D9).
    fn library_state_part(&self) -> Option<String> {
        (!self.singles.is_empty() || !self.matched_by_name.is_empty())
            .then(|| self.library_state_text())
    }

    /// "In Library" when each single is in the Library, else "5 in
    /// Library".
    fn library_state_text(&self) -> String {
        let in_library = self.in_library_count();
        if in_library == self.count() {
            ALL_IN_LIBRARY_LABEL.to_owned()
        } else {
            format!("{in_library} in Library")
        }
    }

    /// The identity level of the group: "By name" or "Artist feed".
    #[must_use]
    pub(crate) const fn level_label(&self) -> &'static str {
        match self.identity {
            ArtistIdentity::Name(_) => NAME_LEVEL_LABEL,
            ArtistIdentity::Publisher(_) => PUBLISHER_LEVEL_LABEL,
        }
    }

    /// The art of the group in the logo frame (ADR 0084 Decision 12): the
    /// newest four images of its singles. A group of an artist feed gives
    /// the channel image of that feed first, when the Library knows it. A
    /// group of a name gives no image of its own.
    #[must_use]
    pub(crate) fn collection_art(&self) -> CollectionArtVm {
        let members = self.position_members();
        CollectionArtVm::resolve(
            self.feed_image,
            members.iter().filter_map(GroupMember::image),
            CollectionKind::SinglesGroup,
        )
    }

    /// Tells if the group holds the release `key`, in `singles`, in
    /// `index_singles`, or in `matched_by_name`.
    #[must_use]
    pub(crate) fn contains(&self, key: &ReleaseKey) -> bool {
        self.singles.iter().any(|single| single.key == *key)
            || self.index_singles.iter().any(|single| single.key == *key)
            || self.matched_by_name.iter().any(|single| single.key == *key)
    }
}

/// Groups the singles of the Library tree by artist identity (ADR 0084
/// Decisions 6 and 7).
///
/// A single is an album whose kind is [`ReleaseKind::Single`] and that has
/// a release key. A group holds two or more singles, counting its
/// `matched_by_name` singles (ADR 0084 Decision 6, amended 2026-10-22,
/// packet 032). One single makes no group. Each group gives its singles
/// newest first: the date of a single is the latest `pub_date` of its
/// tracks, a single with no date comes last, and equal dates sort by the
/// lowercase release name. The groups come in the order of their identity.
#[must_use]
pub(crate) fn singles_groups(tree: &LibraryTree) -> Vec<SinglesGroup<'_>> {
    let feed_images = tree.feed_images();
    let LibrarySinglesGrouping {
        mut own,
        mut matched_by_name,
    } = library_singles_grouped(tree);
    let identities: BTreeSet<ArtistIdentity> = own
        .keys()
        .cloned()
        .chain(
            matched_by_name
                .keys()
                .cloned()
                .map(ArtistIdentity::Publisher),
        )
        .collect();
    identities
        .into_iter()
        .filter_map(|identity| {
            let mut singles = own.remove(&identity).unwrap_or_default();
            let mut matched = feed_matched_singles(&identity, &mut matched_by_name);
            if singles.len() + matched.len() < 2 {
                return None;
            }
            singles.sort_by(newest_first);
            matched.sort_by(newest_first);
            let display_artist = singles.first().or(matched.first())?.artist.name.as_str();
            let feed_image = match &identity {
                ArtistIdentity::Publisher(feed_guid) => feed_images.get(feed_guid),
                ArtistIdentity::Name(_) => None,
            };
            Some(SinglesGroup {
                identity,
                display_artist,
                singles,
                index_singles: Vec::new(),
                matched_by_name: matched,
                feed_image,
            })
        })
        .collect()
}

/// The Library singles of the tree, split by the match-by-name rule (ADR
/// 0084 Decision 6, amended 2026-10-22, packet 032). `own` holds the
/// singles of each identity that keeps its own group: every publisher
/// identity, and a name identity whose match is `NoFeed` or `Ambiguous`.
/// `matched_by_name` holds the singles of a name whose match is
/// `MatchedFeed(guid)`, keyed by `guid`, in no order within each key.
struct LibrarySinglesGrouping<'a> {
    own: BTreeMap<ArtistIdentity, Vec<GroupSingle<'a>>>,
    matched_by_name: BTreeMap<String, Vec<GroupSingle<'a>>>,
}

/// Builds [`LibrarySinglesGrouping`] from `tree`, through
/// [`tree_name_feed_matches`], the one function that matches a name to a
/// Library artist feed. The Artists list and the feed page's "Matched by
/// name" section read that same function.
fn library_singles_grouped(tree: &LibraryTree) -> LibrarySinglesGrouping<'_> {
    let matches = tree_name_feed_matches(tree);
    let mut own: BTreeMap<ArtistIdentity, Vec<GroupSingle<'_>>> = BTreeMap::new();
    let mut matched_by_name: BTreeMap<String, Vec<GroupSingle<'_>>> = BTreeMap::new();
    for artist in &tree.artists {
        for album in &artist.albums {
            if album.release_kind() != ReleaseKind::Single {
                continue;
            }
            let Some(key) = album.release_key() else {
                continue;
            };
            let single = GroupSingle {
                artist,
                album,
                key,
                date: album.tracks.iter().filter_map(|track| track.pub_date).max(),
                in_library: !album.tracks.is_empty()
                    && album.tracks.iter().all(|track| track.is_in_library),
            };
            let identity = album.artist_identity(&artist.name);
            if let ArtistIdentity::Name(name_key) = &identity {
                if let NameFeedMatch::MatchedFeed(feed_guid) = matches.get(name_key) {
                    matched_by_name.entry(feed_guid).or_default().push(single);
                    continue;
                }
            }
            own.entry(identity).or_default().push(single);
        }
    }
    LibrarySinglesGrouping {
        own,
        matched_by_name,
    }
}

/// The singles that a matched name joins to the feed of `identity`, taken
/// out of `matched_by_name`. Empty for a name identity: a name never
/// matches a name.
fn feed_matched_singles<'a>(
    identity: &ArtistIdentity,
    matched_by_name: &mut BTreeMap<String, Vec<GroupSingle<'a>>>,
) -> Vec<GroupSingle<'a>> {
    match identity {
        ArtistIdentity::Publisher(guid) => matched_by_name.remove(guid).unwrap_or_default(),
        ArtistIdentity::Name(_) => Vec::new(),
    }
}

/// The GUID of each artist feed with one or more Library singles, its own
/// or joined by a matched name (ADR 0084 packet 026, amended 2026-10-22
/// by packet 032). Page 1 of the scope "Everything" reads the
/// `MusicIndex` singles of each of these feeds.
#[must_use]
pub(crate) fn library_single_artist_feeds(tree: &LibraryTree) -> Vec<String> {
    let grouping = library_singles_grouped(tree);
    let own_feeds = grouping
        .own
        .into_keys()
        .filter_map(|identity| match identity {
            ArtistIdentity::Publisher(feed_guid) => Some(feed_guid),
            ArtistIdentity::Name(_) => None,
        });
    own_feeds
        .chain(grouping.matched_by_name.into_keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// The artist identity of a `MusicIndex` release row: the rule of the
/// Library, with the named feeds of the row (ADR 0085 Decisions 2 and 6,
/// packet 026).
#[must_use]
pub(crate) fn index_release_identity(feed: &Feed) -> ArtistIdentity {
    ArtistIdentity::resolve(
        &index_named_feeds(feed.named_feeds.as_deref().unwrap_or_default()),
        feed.release_artist.as_deref().unwrap_or_default(),
    )
}

/// Tells if a `MusicIndex` release row is a single (ADR 0084 Decision 5):
/// `release_kind` when it names a kind, else `episode_count`.
fn index_release_is_single(feed: &Feed) -> bool {
    let track_count = feed
        .episode_count
        .and_then(|count| usize::try_from(count).ok())
        .unwrap_or(0);
    ReleaseKind::resolve(feed.release_kind.as_deref(), track_count) == ReleaseKind::Single
}

/// The artist feed of a `MusicIndex` single (packet 026). `None` for a row
/// that is not a single, and for a single at the name level, for example a
/// single whose link to its feed is not confirmed (decision D7).
#[must_use]
pub(crate) fn index_single_artist_feed(feed: &Feed) -> Option<String> {
    if !index_release_is_single(feed) {
        return None;
    }
    match index_release_identity(feed) {
        ArtistIdentity::Publisher(feed_guid) => Some(feed_guid),
        ArtistIdentity::Name(_) => None,
    }
}

/// The singles of the feed `feed_guid` in `rows`, the releases that name
/// the feed (packet 026). A single of the feed is a row that is a single
/// and whose identity is `ArtistIdentity::Publisher(feed_guid)`. A row that
/// names the feed at rank 2, for example as its label, has another
/// identity. A row with no feed GUID is not a single of the feed.
#[must_use]
pub(crate) fn index_singles_of_feed(feed_guid: &str, rows: Vec<Feed>) -> Vec<Feed> {
    rows.into_iter()
        .filter(|row| {
            row.feed_guid
                .as_deref()
                .is_some_and(|guid| !guid.is_empty())
        })
        .filter(|row| index_single_artist_feed(row).as_deref() == Some(feed_guid))
        .collect()
}

/// Groups the singles of the Releases scope "Everything" (ADR 0084
/// Decisions 6 and 7, packet 026).
///
/// - A group of the artist feed G holds the Library singles whose identity
///   is `ArtistIdentity::Publisher(G)`, and each loaded `MusicIndex` single
///   of G whose release key is not a Library release (decision D8).
/// - A group of a name holds Library singles only (decision D6).
/// - A group holds two or more members. The groups come in the order of
///   their identity.
///
/// `fills` holds the loaded `MusicIndex` singles of each feed. A member
/// counts also when its name list page did not load.
#[must_use]
pub(crate) fn singles_groups_everything<'a>(
    tree: &'a LibraryTree,
    fills: &'a IndexFeedSingles,
) -> Vec<SinglesGroup<'a>> {
    everything_groups(tree, fills, 2)
}

/// The groups of [`singles_groups_everything`] with `min_members` or more
/// members. The group page uses 1 while the request for its feed runs or
/// after it failed, so the Library single of the feed still shows.
fn everything_groups<'a>(
    tree: &'a LibraryTree,
    fills: &'a IndexFeedSingles,
    min_members: usize,
) -> Vec<SinglesGroup<'a>> {
    let library_keys: BTreeSet<ReleaseKey> = tree
        .artists
        .iter()
        .flat_map(|artist| artist.albums.iter())
        .filter_map(AlbumNode::release_key)
        .collect();
    let LibrarySinglesGrouping {
        mut own,
        mut matched_by_name,
    } = library_singles_grouped(tree);
    let mut index: BTreeMap<ArtistIdentity, Vec<IndexGroupSingle<'a>>> = BTreeMap::new();
    for (feed_guid, singles) in fills.iter() {
        let mut seen: BTreeSet<ReleaseKey> = BTreeSet::new();
        let members: Vec<IndexGroupSingle<'a>> = singles
            .iter()
            .filter_map(|feed| {
                let guid = feed.feed_guid.as_deref().filter(|guid| !guid.is_empty())?;
                let key = ReleaseKey::FeedGuid(guid.to_owned());
                if library_keys.contains(&key) || !seen.insert(key.clone()) {
                    return None;
                }
                Some(IndexGroupSingle {
                    feed,
                    key,
                    date: feed.newest_item_at,
                    sort_key: feed.sort_name.clone().unwrap_or_else(|| {
                        name_sort_key(feed.title.as_deref().unwrap_or_default())
                    }),
                })
            })
            .collect();
        if !members.is_empty() {
            index.insert(ArtistIdentity::Publisher(feed_guid.to_owned()), members);
        }
    }
    let identities: BTreeSet<ArtistIdentity> = own
        .keys()
        .cloned()
        .chain(
            matched_by_name
                .keys()
                .cloned()
                .map(ArtistIdentity::Publisher),
        )
        .chain(index.keys().cloned())
        .collect();
    let feed_images = tree.feed_images();
    identities
        .into_iter()
        .filter_map(|identity| {
            let mut singles = own.remove(&identity).unwrap_or_default();
            let mut index_singles = index.remove(&identity).unwrap_or_default();
            let mut matched = feed_matched_singles(&identity, &mut matched_by_name);
            if singles.len() + index_singles.len() + matched.len() < min_members.max(1) {
                return None;
            }
            singles.sort_by(newest_first);
            matched.sort_by(newest_first);
            index_singles.sort_by(|left, right| {
                newest_date_first(left.date, right.date)
                    .then_with(|| left.sort_key.cmp(&right.sort_key))
            });
            let feed_guid = match &identity {
                ArtistIdentity::Publisher(feed_guid) => Some(feed_guid.as_str()),
                ArtistIdentity::Name(_) => None,
            };
            let display_artist = match singles.first().or(matched.first()) {
                Some(newest) => newest.artist.name.as_str(),
                None => index_group_label(&index_singles[0], feed_guid),
            };
            let feed_image = feed_guid.and_then(|guid| feed_images.get(guid));
            Some(SinglesGroup {
                identity,
                display_artist,
                singles,
                index_singles,
                matched_by_name: matched,
                feed_image,
            })
        })
        .collect()
}

/// The label of a group with no Library single (packet 026 decision D9):
/// the `remote_feed_title` of the entry of the artist feed in the newest
/// `MusicIndex` single, else its `release_artist`.
fn index_group_label<'a>(newest: &IndexGroupSingle<'a>, feed_guid: Option<&str>) -> &'a str {
    let feed: &'a Feed = newest.feed;
    let feed_title = feed
        .named_feeds
        .as_deref()
        .unwrap_or_default()
        .iter()
        .find(|entry| Some(entry.publisher_feed_guid.as_str()) == feed_guid)
        .and_then(|entry| entry.remote_feed_title.as_deref());
    [feed_title, feed.release_artist.as_deref()]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|text| !text.is_empty())
        .unwrap_or(UNKNOWN_ARTIST_TEXT)
}

/// Newest date first, and a missing date last.
fn newest_date_first(left: Option<i64>, right: Option<i64>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => right.cmp(&left),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// The order of the singles in a group: newest first, a single with no
/// date last, and equal dates by the lowercase release name.
fn newest_first(left: &GroupSingle<'_>, right: &GroupSingle<'_>) -> Ordering {
    newest_date_first(left.date, right.date).then_with(|| {
        left.album
            .name
            .to_lowercase()
            .cmp(&right.album.name.to_lowercase())
    })
}

/// The group that has the key `key`, when one exists.
#[must_use]
pub(crate) fn find_group<'g, 'a>(
    groups: &'g [SinglesGroup<'a>],
    key: &SinglesGroupKey,
) -> Option<&'g SinglesGroup<'a>> {
    groups.iter().find(|group| group.identity == key.0)
}

/// The group row of the Releases list (ADR 0084 Decision 6).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SinglesGroupRowVm {
    /// Stable element id, from the group identity.
    pub(crate) element_id: String,
    /// The display artist, for example "Haleen".
    pub(crate) label: String,
    /// The count, the Library state and the identity level, for example
    /// "7 singles · 5 in Library · By name".
    pub(crate) secondary_text: String,
    /// The accessibility label, for example "Haleen, 7 singles, 5 in
    /// Library, By name".
    pub(crate) a11y_label: String,
    /// The art of the group in the logo frame (ADR 0084 Decision 12).
    pub(crate) art: CollectionArtVm,
    /// The key of the group page.
    pub(crate) open: SinglesGroupKey,
    /// The row shows its singles.
    pub(crate) expanded: bool,
    /// The element id of the chevron.
    pub(crate) disclosure_element_id: String,
    /// "Show singles" or "Hide singles".
    pub(crate) disclosure_a11y_label: &'static str,
    /// The release row of each single that the group row counts, when
    /// `expanded` is true. Empty when it is false. The feed's own singles
    /// come first, newest first. A single that a matched name joins to
    /// the group comes after them, with "Matched by name" added to its
    /// subtitle (orchestrator decision of 2026-10-22, packet 032, which
    /// replaces decision D2: the count and this list must agree).
    pub(crate) singles: Vec<DestinationListRowVm>,
    /// The release key of each single that the group row counts, in the
    /// same order as [`Self::singles`], whatever `expanded` is (ADR 0084
    /// Decision 6, amended 2026-10-22). The group row's checkbox toggles
    /// every one of these.
    pub(crate) single_keys: Vec<ReleaseKey>,
    /// The checkbox facts of the group row (ADR 0084 Decision 10). The
    /// group's own checkbox, not one for each single.
    pub(crate) selection: RowSelectionVm,
}

impl SinglesGroupRowVm {
    /// Builds the group row. `release_row` gives the release row of one
    /// Library single, the same row that a release that is not in a group
    /// gets. A `MusicIndex` single gets the `MusicIndex` release row
    /// ([`index_release_row`], packet 026). A single that a matched name
    /// joins to the group gets the same Library release row, with
    /// [`mark_matched_by_name`] added (orchestrator decision of
    /// 2026-10-22, packet 032, which replaces decision D2).
    #[must_use]
    pub(crate) fn new(
        group: &SinglesGroup<'_>,
        expanded: bool,
        release_row: impl Fn(&ArtistNode, &AlbumNode) -> Option<DestinationListRowVm>,
    ) -> Self {
        let element_id = group_element_id(&group.identity);
        let count = group.count();
        let members = group.members();
        let a11y_label = match group.library_state_part() {
            Some(state) => format!(
                "{}, {count} singles, {state}, {}",
                group.display_artist,
                group.level_label()
            ),
            None => format!(
                "{}, {count} singles, {}",
                group.display_artist,
                group.level_label()
            ),
        };
        Self {
            disclosure_element_id: format!("{element_id}-disclosure"),
            element_id,
            label: group.display_artist.to_owned(),
            secondary_text: group.summary_text(),
            a11y_label,
            art: group.collection_art(),
            open: group.key(),
            expanded,
            disclosure_a11y_label: if expanded {
                HIDE_SINGLES_LABEL
            } else {
                SHOW_SINGLES_LABEL
            },
            singles: if expanded {
                let mut rows: Vec<DestinationListRowVm> = members
                    .iter()
                    .filter_map(|member| match member {
                        GroupMember::Library(single) => release_row(single.artist, single.album),
                        GroupMember::Index(single) => index_release_row(single.feed),
                    })
                    .collect();
                rows.extend(group.matched_by_name.iter().filter_map(|single| {
                    let mut row = release_row(single.artist, single.album)?;
                    mark_matched_by_name(&mut row);
                    Some(row)
                }));
                rows
            } else {
                Vec::new()
            },
            single_keys: members
                .iter()
                .map(|member| member.key().clone())
                .chain(
                    group
                        .matched_by_name
                        .iter()
                        .map(|single| single.key.clone()),
                )
                .collect(),
            selection: RowSelectionVm::none(),
        }
    }
}

/// Marks a disclosure row of the Releases list as a single that a
/// matched name joins to the group (orchestrator decision of
/// 2026-10-22, packet 032, which replaces decision D2). Unlike a feed
/// page row, this row has no field of its own for an inferred relation,
/// so the mark goes into the subtitle text the view model already
/// gives, the same words as the group page's "Matched by name" heading.
fn mark_matched_by_name(row: &mut DestinationListRowVm) {
    row.secondary_text = format!(
        "{}{SUMMARY_SEPARATOR}{}",
        row.secondary_text,
        PublisherPageVm::MATCHED_BY_NAME_HEADING
    );
    // Packet 032 review: a screen reader also hears the inferred relation.
    row.a11y_label = format!(
        "{}, {}",
        row.a11y_label,
        PublisherPageVm::MATCHED_BY_NAME_HEADING
    );
}

/// The element id of a group row. It carries the publisher GUID or the
/// name text as it is, so two names that differ only in case keep two ids.
fn group_element_id(identity: &ArtistIdentity) -> String {
    match identity {
        ArtistIdentity::Publisher(guid) => format!("music-singles-group-publisher:{guid}"),
        ArtistIdentity::Name(name) => format!("music-singles-group-name:{name}"),
    }
}

/// The name link from a single page to its group (ADR 0084 Decision 6),
/// for example "All 7 singles". `groups` are the groups of
/// [`singles_groups_everything`], so a group with `MusicIndex` members
/// gives its full count (packet 026 decision D14). `None` when the release
/// `key` is in no group.
#[must_use]
pub(crate) fn singles_group_name_link(
    groups: &[SinglesGroup<'_>],
    key: &ReleaseKey,
) -> Option<AlbumNameLinkVm> {
    let group = groups.iter().find(|group| group.contains(key))?;
    let count = group.count();
    Some(AlbumNameLinkVm {
        label: format!("All {count} singles"),
        a11y_label: format!("Open all {count} singles by {}", group.display_artist),
        note: None,
        target: AlbumNameLinkTarget::SinglesGroup(group.key()),
    })
}

/// The name links of a release page (ADR 0084 Decision 6): the links of
/// `base`, then the link to the group of the release, when the release
/// `key` is in one.
#[must_use]
pub(crate) fn release_page_name_links(
    mut base: Vec<AlbumNameLinkVm>,
    groups: &[SinglesGroup<'_>],
    key: Option<&ReleaseKey>,
) -> Vec<AlbumNameLinkVm> {
    if let Some(link) = key.and_then(|key| singles_group_name_link(groups, key)) {
        base.push(link);
    }
    base
}

/// The artist feed of a single page whose `MusicIndex` singles the page
/// must read: the artist feed of a single, when the app has no result for
/// it and no request runs (packet 026 decision D14). `None` for a release
/// that is not a single, and for a single at the name level.
#[must_use]
pub(crate) fn single_page_feed_to_read(
    kind: ReleaseKind,
    named_feeds: &[NamedFeed],
    fills: &IndexFeedSingles,
) -> Option<String> {
    if kind != ReleaseKind::Single {
        return None;
    }
    match ArtistIdentity::resolve(named_feeds, "") {
        ArtistIdentity::Publisher(feed_guid) if fills.needs_request(&feed_guid) => Some(feed_guid),
        ArtistIdentity::Publisher(_) | ArtistIdentity::Name(_) => None,
    }
}

/// [`single_page_feed_to_read`] for a Library release page. The Library
/// decides a Library release (packet 026 decision D8).
#[must_use]
pub(crate) fn library_page_feed_to_read(
    album: &AlbumNode,
    fills: &IndexFeedSingles,
) -> Option<String> {
    single_page_feed_to_read(album.release_kind(), &album.named_feeds, fills)
}

/// [`single_page_feed_to_read`] for an Index release page, from its loaded
/// feed.
#[must_use]
pub(crate) fn index_page_feed_to_read(feed: &FeedView, fills: &IndexFeedSingles) -> Option<String> {
    let kind = ReleaseKind::resolve(feed.release_kind.as_deref(), feed.track_count());
    single_page_feed_to_read(kind, &feed.named_feeds, fills)
}

/// The artist feed whose `MusicIndex` singles a group page must read: the
/// feed of a feed identity, when the app has no result for it and no
/// request runs (packet 026 Part B). `None` for a name identity.
#[must_use]
pub(crate) fn group_page_feed_to_read(
    key: &SinglesGroupKey,
    fills: &IndexFeedSingles,
) -> Option<String> {
    match &key.0 {
        ArtistIdentity::Publisher(feed_guid) if fills.needs_request(feed_guid) => {
            Some(feed_guid.clone())
        }
        ArtistIdentity::Publisher(_) | ArtistIdentity::Name(_) => None,
    }
}

/// The command of the main action of a group page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SinglesGroupAction {
    /// Download each single that is not in the Library, newest first: the
    /// Library feed id of each Library single, and the feed GUID and the
    /// title of each `MusicIndex` single (packet 026 Part B).
    DownloadSingles(ReleasesDownloadPlan),
    /// Add each single to a playlist, newest first, by Library feed id.
    AddToPlaylist(Vec<i64>),
}

/// The main action of a group page, with its availability and its labels.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SinglesGroupActionVm {
    /// The command.
    pub(crate) action: SinglesGroupAction,
    /// The visible label, for example "Download 2 singles".
    pub(crate) label: String,
    /// The accessibility label.
    pub(crate) a11y_label: String,
    /// The action can run now.
    pub(crate) available: bool,
    /// The accessibility label of the "new playlist" control of the
    /// playlist popover.
    pub(crate) new_playlist_a11y_label: &'static str,
    /// The accessibility label of the "back" control of the playlist
    /// popover.
    pub(crate) back_a11y_label: &'static str,
    /// The accessibility label of the "create" control of the playlist
    /// popover.
    pub(crate) create_a11y_label: &'static str,
    /// The accessibility label of the action row that holds the action.
    pub(crate) row_a11y_label: &'static str,
}

/// One single row of a group page. The page shows it as the shared
/// Library track row of a release page (ADR 0084 task 015, operator
/// decision of 2026-10-17). The toggle action of that row shows the
/// Library state, so the row has no state word.
#[derive(Clone, Debug)]
pub(crate) struct SinglesGroupPageRowVm {
    /// The track of the single that the shared track row shows: the
    /// first track of the single.
    pub(crate) track: TrackRow,
    /// The date text of the single: the latest `pub_date` of its tracks.
    /// `None` when the single has no date.
    pub(crate) date_text: Option<String>,
    /// The download or the removal of the track or of its single that is
    /// in progress.
    pub(crate) work: TrackRowWork,
    /// The release key that a click opens.
    pub(crate) key: ReleaseKey,
}

/// One `MusicIndex` single row of a group page: the same track row as a
/// Library single (operator decision of 2026-10-22, replacing packet 026
/// decision D13). A click opens the release page of the single.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SinglesGroupIndexRowVm {
    /// Stable element id, from the release GUID.
    pub(crate) element_id: String,
    /// The release key that a click opens.
    pub(crate) key: ReleaseKey,
    /// The title of the release.
    pub(crate) title: String,
    /// The date text of `newest_item_at`. `None` when the row has no date.
    pub(crate) date_text: Option<String>,
    /// The duration of the single's one track, when the `MusicIndex` data
    /// the app already holds gives one. `None` keeps the duration column
    /// empty, with its token width (operator decision of 2026-10-22).
    pub(crate) duration: Option<String>,
    /// The release image.
    pub(crate) image_url: Option<String>,
    /// The download or the removal of the release that is in progress.
    pub(crate) work: TrackRowWork,
}

impl SinglesGroupIndexRowVm {
    fn new(single: &IndexGroupSingle<'_>, work: TrackRowWork) -> Self {
        let title = index_release_title(single.feed);
        let date_text = single.date.and_then(fmt_date);
        let element_id = match &single.key {
            ReleaseKey::FeedGuid(guid) => format!("singles-group-index-row:{guid}"),
            ReleaseKey::LibraryFeed(feed_id) => format!("singles-group-index-row:{feed_id}"),
        };
        let duration = single
            .feed
            .tracks
            .as_deref()
            .and_then(|tracks| tracks.first())
            .and_then(|track| track.duration_secs)
            .map(fmt_dur);
        Self {
            element_id,
            key: single.key.clone(),
            title,
            date_text,
            duration,
            image_url: single
                .feed
                .image_url
                .clone()
                .filter(|url| !url.trim().is_empty()),
            work,
        }
    }
}

/// One row of a group page: a Library single or a `MusicIndex` single.
/// Both show through the same track row (operator decision of
/// 2026-10-22, replacing packet 026 decision D13), and share one list
/// and one date order.
#[derive(Clone, Debug)]
pub(crate) enum SinglesGroupPageEntryVm {
    /// A Library single keeps the shared Library track row. The row holds
    /// a full track, so it is boxed.
    Library(Box<SinglesGroupPageRowVm>),
    /// A `MusicIndex` single shows as a release row.
    Index(SinglesGroupIndexRowVm),
}

impl SinglesGroupPageEntryVm {
    /// The release key that a click opens.
    #[must_use]
    pub(crate) fn key(&self) -> &ReleaseKey {
        match self {
            Self::Library(row) => &row.key,
            Self::Index(row) => &row.key,
        }
    }
}

/// The page of one singles group (ADR 0084 Decision 6).
#[derive(Clone, Debug)]
pub(crate) struct SinglesGroupPageVm {
    /// "Singles by Haleen". The breadcrumb label is the same text.
    pub(crate) title: String,
    /// The same text as the group row, for example "7 singles · 5 in
    /// Library". `None` for the empty state.
    pub(crate) subtitle: Option<String>,
    /// The note that names the identity level of the group. `None` for
    /// the empty state.
    pub(crate) identity_note: Option<&'static str>,
    /// One row for each single: the feed's own Library and `MusicIndex`
    /// rows, newest first, then the rows of a matched name (packet 032).
    pub(crate) rows: Vec<SinglesGroupPageEntryVm>,
    /// The loading row or the error row of the request for the
    /// `MusicIndex` singles of the feed (packet 026 Part B). `None` for a
    /// name identity and when the app has the result.
    pub(crate) index_status: Option<IndexListStatusRowVm>,
    /// The note that more releases name the feed than the app read
    /// (packet 026 decision D11).
    pub(crate) more_note: Option<&'static str>,
    /// The one main action. `None` for the empty state.
    pub(crate) main_action: Option<SinglesGroupActionVm>,
    /// The empty state text, when no group has the key.
    pub(crate) empty_message: Option<&'static str>,
    /// The header art in the logo frame (ADR 0084 Decision 12). `None`
    /// for the empty state.
    pub(crate) art: Option<CollectionArtVm>,
    /// The heading and the note of the trailing "Matched by name" rows
    /// (ADR 0084 Decision 6, amended 2026-10-22, packet 032). `None` when
    /// the group has no such row. The heading goes before the last
    /// `MatchedByNameHeadingVm::count` rows of `rows`.
    pub(crate) matched_by_name: Option<MatchedByNameHeadingVm>,
}

/// The heading and the note of the trailing "Matched by name" rows of a
/// group page (ADR 0084 Decision 6, amended 2026-10-22, packet 032): the
/// same heading and note as the feed page's section of the same name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MatchedByNameHeadingVm {
    pub(crate) heading: &'static str,
    pub(crate) note: &'static str,
    /// The count of rows after this heading, at the end of `rows`.
    pub(crate) count: usize,
}

impl SinglesGroupPageVm {
    /// Builds the page of the group `key` from the current tree and the
    /// loaded `MusicIndex` singles. The page is not scoped (packet 026
    /// decision D12): it reads the groups of "Everything".
    ///
    /// - While the request for the feed runs, or after it failed, the
    ///   Library rows show with the loading row or the error row.
    /// - When no group has the key and no request is open, for example
    ///   after a removal leaves one single, the page is the empty state.
    ///
    /// `busy` holds the downloads and the removals in progress (ADR 0084
    /// Decision 10). While one single of the group is busy, the main
    /// action is not available.
    #[must_use]
    pub(crate) fn from_tree(
        tree: &LibraryTree,
        fills: &IndexFeedSingles,
        key: &SinglesGroupKey,
        busy: &BusyItems,
    ) -> Self {
        let state = match &key.0 {
            ArtistIdentity::Publisher(feed_guid) => {
                Some((feed_guid.as_str(), fills.state(feed_guid)))
            }
            ArtistIdentity::Name(_) => None,
        };
        let index_status = state.and_then(|(feed_guid, state)| index_status_row(feed_guid, state));
        let more_note = matches!(state, Some((_, FeedSinglesState::Loaded { more: true })))
            .then_some(MORE_INDEX_SINGLES_NOTE);
        let groups = everything_groups(tree, fills, 1);
        match find_group(&groups, key) {
            Some(group) if group.count() >= 2 => {
                let mut page = Self::new(group, busy);
                page.index_status = index_status;
                page.more_note = more_note;
                page
            }
            group if index_status.is_some() => {
                let mut rows = group
                    .map(|group| page_rows(group, busy))
                    .unwrap_or_default();
                let matched = group
                    .map(|group| matched_by_name_rows(group, busy))
                    .unwrap_or_default();
                let matched_by_name = matched_by_name_heading(&matched);
                rows.extend(matched);
                Self {
                    title: group.map_or_else(
                        || EMPTY_GROUP_TITLE.to_owned(),
                        |group| singles_group_title(group.display_artist),
                    ),
                    subtitle: None,
                    identity_note: Some(PUBLISHER_LEVEL_NOTE),
                    rows,
                    index_status,
                    more_note: None,
                    main_action: None,
                    empty_message: None,
                    art: group.map(SinglesGroup::collection_art),
                    matched_by_name,
                }
            }
            _ => Self {
                title: EMPTY_GROUP_TITLE.to_owned(),
                subtitle: None,
                identity_note: None,
                rows: Vec::new(),
                index_status: None,
                more_note: None,
                main_action: None,
                empty_message: Some(EMPTY_GROUP_MESSAGE),
                art: None,
                matched_by_name: None,
            },
        }
    }

    /// Builds the page of one group. A Library single with no track has no
    /// row.
    #[must_use]
    pub(crate) fn new(group: &SinglesGroup<'_>, busy: &BusyItems) -> Self {
        let group_busy = group
            .singles
            .iter()
            .chain(&group.matched_by_name)
            .any(|single| busy.release_downloading(single.album) || busy.removing(single.album))
            || group
                .index_singles
                .iter()
                .any(|single| busy.index_release_downloading(&single.key));
        let mut rows = page_rows(group, busy);
        let matched = matched_by_name_rows(group, busy);
        let matched_by_name = matched_by_name_heading(&matched);
        rows.extend(matched);
        Self {
            title: singles_group_title(group.display_artist),
            subtitle: Some(group.summary_text()),
            identity_note: match group.identity {
                ArtistIdentity::Name(_) => Some(NAME_LEVEL_NOTE),
                ArtistIdentity::Publisher(_) => Some(PUBLISHER_LEVEL_NOTE),
            },
            rows,
            index_status: None,
            more_note: None,
            main_action: Some(main_action(group, group_busy)),
            empty_message: None,
            art: Some(group.collection_art()),
            matched_by_name,
        }
    }
}

/// The rows of the feed's own members of a group page, newest first by
/// the date rule of the group (packet 026 decision D13). The rows of a
/// matched name come after them, from [`matched_by_name_rows`].
fn page_rows(group: &SinglesGroup<'_>, busy: &BusyItems) -> Vec<SinglesGroupPageEntryVm> {
    group
        .members()
        .iter()
        .filter_map(|member| match member {
            GroupMember::Library(single) => library_single_row(single, busy),
            GroupMember::Index(single) => Some(SinglesGroupPageEntryVm::Index(
                SinglesGroupIndexRowVm::new(single, busy.index_release_work(&single.key)),
            )),
        })
        .collect()
}

/// The row of one Library single of a group page: the shared Library
/// track row. `None` when the single has no track.
fn library_single_row(
    single: &GroupSingle<'_>,
    busy: &BusyItems,
) -> Option<SinglesGroupPageEntryVm> {
    let track = single.album.tracks.first()?;
    Some(SinglesGroupPageEntryVm::Library(Box::new(
        SinglesGroupPageRowVm {
            track: track.clone(),
            date_text: single.date.and_then(fmt_date),
            work: busy.track_row_work(single.album, track),
            key: single.key.clone(),
        },
    )))
}

/// The rows of the trailing "Matched by name" section of a group page
/// (ADR 0084 Decision 6, amended 2026-10-22, packet 032), newest first.
/// Empty when the group has no matched single.
fn matched_by_name_rows(
    group: &SinglesGroup<'_>,
    busy: &BusyItems,
) -> Vec<SinglesGroupPageEntryVm> {
    group
        .matched_by_name
        .iter()
        .filter_map(|single| library_single_row(single, busy))
        .collect()
}

/// The heading of [`matched_by_name_rows`], when it has a row. The
/// heading and the note are the same text as the feed page's section
/// "Matched by name" (ADR 0084 Decision 7, amendment of 2026-10-20).
fn matched_by_name_heading(rows: &[SinglesGroupPageEntryVm]) -> Option<MatchedByNameHeadingVm> {
    (!rows.is_empty()).then_some(MatchedByNameHeadingVm {
        heading: PublisherPageVm::MATCHED_BY_NAME_HEADING,
        note: PublisherPageVm::MATCHED_BY_NAME_NOTE,
        count: rows.len(),
    })
}

/// The loading row or the error row of a group page for the state of the
/// request for its feed (packet 026 Part B). `None` when the app has the
/// result. A feed that the app did not read yet shows the loading row: the
/// page sends its request when it opens.
fn index_status_row(feed_guid: &str, state: FeedSinglesState<'_>) -> Option<IndexListStatusRowVm> {
    match state {
        FeedSinglesState::Loaded { .. } => None,
        FeedSinglesState::NotRead | FeedSinglesState::Running => {
            Some(IndexListStatusRowVm::Loading {
                element_id: "singles-group-index-loading".to_owned(),
                label: LOADING_INDEX_SINGLES_LABEL,
            })
        }
        FeedSinglesState::Failed(detail) => Some(IndexListStatusRowVm::Failed {
            element_id: "singles-group-index-error".to_owned(),
            first_page: true,
            message: INDEX_SINGLES_FAILED_MESSAGE.to_owned(),
            detail: format!("{}: {detail}", feed_singles_request(feed_guid)),
            retry: IndexListActionDisplay {
                id: "singles-group-index-retry".to_owned(),
                label: "Try again",
                a11y_label: RETRY_INDEX_SINGLES_A11Y_LABEL.to_owned(),
                available: true,
            },
        }),
    }
}

/// The title of a group page and its breadcrumb label, for example
/// "Singles by Haleen".
#[must_use]
pub(crate) fn singles_group_title(display_artist: &str) -> String {
    format!("Singles by {display_artist}")
}

/// The main action of a group page: "Download n singles" when one or more
/// singles are not in the Library, else "Add to playlist". n counts the
/// Library singles that are not each in the Library and each `MusicIndex`
/// single (packet 026 Part B).
fn main_action(group: &SinglesGroup<'_>, busy: bool) -> SinglesGroupActionVm {
    // The Library part of each action reads `singles` and
    // `matched_by_name` together: both are Library singles of this group
    // (ADR 0084 Decision 6, amended 2026-10-22, packet 032).
    let library_singles = || group.singles.iter().chain(&group.matched_by_name);
    let missing = ReleasesDownloadPlan {
        library_feed_ids: library_singles()
            .filter(|single| !single.in_library)
            .filter_map(|single| single.album.feed_id)
            .collect(),
        index_releases: group
            .index_singles
            .iter()
            .filter_map(|single| {
                let ReleaseKey::FeedGuid(guid) = &single.key else {
                    return None;
                };
                Some((guid.clone(), Some(index_release_title(single.feed))))
            })
            .collect(),
    };
    let missing_count = group.count() - group.in_library_count();
    let (action, label, a11y_label) = if missing_count > 0 {
        let noun = format!("single{}", plural(missing_count));
        let a11y_label = if missing_count == 1 {
            "Download the 1 single that is not in the Library".to_owned()
        } else {
            format!("Download the {missing_count} singles that are not in the Library")
        };
        (
            SinglesGroupAction::DownloadSingles(missing),
            format!("Download {missing_count} {noun}"),
            a11y_label,
        )
    } else {
        let all: Vec<i64> = library_singles()
            .filter_map(|single| single.album.feed_id)
            .collect();
        (
            SinglesGroupAction::AddToPlaylist(all),
            "Add to playlist".to_owned(),
            format!("Add the {} singles to a playlist", group.count()),
        )
    };
    SinglesGroupActionVm {
        action,
        label,
        a11y_label,
        available: !busy,
        new_playlist_a11y_label: "Create a new playlist",
        back_a11y_label: "Back to playlist choices",
        create_a11y_label: "Create playlist and add singles",
        row_a11y_label: "Singles group actions",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::view_models::download_progress::DownloadActivity;
    use crate::views::{FeedMetadataFacts, LocalIdentityFacts};

    fn track(id: i64, feed_id: i64, pub_date: Option<i64>, in_library: bool) -> TrackRow {
        TrackRow {
            id,
            feed_id,
            item_guid: format!("item-{id}"),
            pub_date,
            is_in_library: in_library,
            ..TrackRow::default()
        }
    }

    fn release(
        name: &str,
        feed_id: i64,
        tracks: Vec<TrackRow>,
        stated_kind: Option<&str>,
    ) -> AlbumNode {
        AlbumNode {
            name: name.to_owned(),
            feed_id: Some(feed_id),
            feed_guid: Some(format!("guid-{feed_id}")),
            feed_url: None,
            language: None,
            description: None,
            image_href: Some(format!("{feed_id}.jpg")),
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

    fn single(name: &str, feed_id: i64, pub_date: Option<i64>, in_library: bool) -> AlbumNode {
        release(
            name,
            feed_id,
            vec![track(feed_id * 10, feed_id, pub_date, in_library)],
            None,
        )
    }

    /// Gives `album` a confirmed named feed, the same as a confirmed link
    /// with no agreed role (ADR 0085 Decision 6): the primary named feed
    /// becomes the unverified artist feed, so `artist_identity` still gives
    /// `ArtistIdentity::Publisher(guid)`, the same as before ADR 0085.
    fn with_publisher(mut album: AlbumNode, guid: &str) -> AlbumNode {
        album.named_feeds = vec![crate::view_models::named_feeds::NamedFeed {
            feed_guid: guid.to_owned(),
            title: None,
            position: Some(0),
            role: None,
            role_agreement: None,
            feed_rel: None,
            album_rel: None,
            confirmed: true,
            image_url: None,
        }];
        album
    }

    fn artist(name: &str, albums: Vec<AlbumNode>) -> ArtistNode {
        ArtistNode {
            name: name.to_owned(),
            albums,
        }
    }

    fn haleen_tree(count: i64, in_library: i64) -> LibraryTree {
        LibraryTree {
            artists: vec![artist(
                "Haleen",
                (1..=count)
                    .map(|n| single(&format!("Single {n}"), n, Some(n * 100), n <= in_library))
                    .collect(),
            )],
        }
    }

    /// The Library row of a page entry.
    fn library_row(entry: &SinglesGroupPageEntryVm) -> &SinglesGroupPageRowVm {
        match entry {
            SinglesGroupPageEntryVm::Library(row) => row,
            SinglesGroupPageEntryVm::Index(row) => panic!("a MusicIndex row: {}", row.title),
        }
    }

    fn names(group: &SinglesGroup<'_>) -> Vec<String> {
        group
            .singles
            .iter()
            .map(|single| single.album.name.clone())
            .collect()
    }

    /// R15-03: two singles of one name give one group. One single gives
    /// no group.
    #[test]
    fn adr_0084_singles_group_r15_03_two_singles_of_one_name_make_one_group() {
        let tree = haleen_tree(2, 2);
        let groups = singles_groups(&tree);
        assert_eq!(groups.len(), 1);
        assert_eq!(
            groups[0].identity,
            ArtistIdentity::Name("haleen".to_owned())
        );
        assert_eq!(groups[0].display_artist, "Haleen");
        assert_eq!(groups[0].count(), 2);

        let tree = haleen_tree(1, 1);
        assert!(singles_groups(&tree).is_empty());
    }

    /// R15-03: an album or an EP makes no group. R17-02 (packet 017
    /// decision D3): names that differ only in case make one group. A
    /// publisher identity and a name identity with the same text, but no
    /// feed that the name matches, stay two groups: see
    /// `adr_0084_singles_group_rows_r32_08_name_matched_to_one_feed_joins_its_group`
    /// (packet 032) for the name that matches one feed by text.
    #[test]
    fn adr_0084_singles_group_r15_03_different_identities_and_kinds_do_not_group() {
        let case_tree = LibraryTree {
            artists: vec![
                artist("Haleen", vec![single("A", 1, Some(1), true)]),
                artist("haleen", vec![single("B", 2, Some(2), true)]),
            ],
        };
        assert_eq!(singles_groups(&case_tree).len(), 1);

        let kind_tree = LibraryTree {
            artists: vec![artist(
                "Haleen",
                vec![
                    single("A", 1, Some(1), true),
                    release(
                        "Long Player",
                        2,
                        vec![track(20, 2, Some(2), true), track(21, 2, Some(2), true)],
                        None,
                    ),
                    release(
                        "Short Player",
                        3,
                        vec![track(30, 3, Some(3), true)],
                        Some("ep"),
                    ),
                ],
            )],
        };
        assert!(singles_groups(&kind_tree).is_empty());
    }

    /// R32-08 (ADR 0084 Decision 6, amended 2026-10-22, packet 032): a
    /// name that matches exactly one artist feed has no group of its
    /// own. Its singles join the group of that feed, in the Library
    /// scope. This is the tree of the old
    /// `adr_0084_singles_group_r15_03_different_identities_and_kinds_do_not_group`
    /// `level_tree`, which gave no group before this decision.
    #[test]
    fn adr_0084_singles_group_rows_r32_08_name_matched_to_one_feed_joins_its_group() {
        let tree = LibraryTree {
            artists: vec![artist(
                "Haleen",
                vec![
                    with_publisher(single("A", 1, Some(1), true), "Haleen"),
                    single("B", 2, Some(2), true),
                ],
            )],
        };
        let groups = singles_groups(&tree);
        assert_eq!(groups.len(), 1, "A and B join one group");
        assert_eq!(
            groups[0].identity,
            ArtistIdentity::Publisher("Haleen".to_owned())
        );
        assert_eq!(groups[0].count(), 2);
        assert_eq!(groups[0].singles.len(), 1);
        assert_eq!(groups[0].matched_by_name.len(), 1);
        assert_eq!(groups[0].matched_by_name[0].album.name, "B");
        let matched_key = groups[0].matched_by_name[0].key.clone();
        assert!(groups[0].contains(&matched_key));
    }

    /// R32-09: the same join also applies in the scope "Everything", with
    /// the `MusicIndex` singles of the feed added in.
    #[test]
    fn adr_0084_singles_group_rows_r32_09_matched_group_in_everything_scope() {
        let tree = LibraryTree {
            artists: vec![artist(
                "Haleen",
                vec![
                    with_publisher(single("A", 1, Some(1), true), HALEEN),
                    single("B", 2, Some(2), true),
                ],
            )],
        };
        let store = fills(HALEEN, vec![single_naming("i1", "Index One", HALEEN, 300)]);
        let groups = singles_groups_everything(&tree, &store);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].count(), 3, "A, B and the MusicIndex single");
        assert_eq!(groups[0].matched_by_name.len(), 1);
        assert_eq!(groups[0].index_singles.len(), 1);
    }

    /// R32-10: a name that matches two artist feeds keeps its own group.
    /// Neither feed's singles gain a `matched_by_name` member.
    #[test]
    fn adr_0084_singles_group_rows_r32_10_name_matched_to_two_feeds_keeps_its_own_group() {
        let tree = LibraryTree {
            artists: vec![
                artist(
                    "Haleen",
                    vec![with_publisher(single("A", 1, Some(1), true), HALEEN)],
                ),
                artist(
                    "Haleen",
                    vec![with_publisher(single("C", 3, Some(3), true), HALEEN_2)],
                ),
                artist(
                    "Haleen",
                    vec![single("B", 2, Some(2), true), single("D", 4, Some(4), true)],
                ),
            ],
        };
        let groups = singles_groups(&tree);
        assert_eq!(
            groups.len(),
            1,
            "A and C each have one single; only the name group has two"
        );
        assert_eq!(
            groups[0].identity,
            ArtistIdentity::Name("haleen".to_owned())
        );
        assert_eq!(groups[0].count(), 2);
        assert!(groups[0].matched_by_name.is_empty());
    }

    /// R32-11: the group page lists the matched singles under the
    /// heading and the note of the feed page's "Matched by name" section,
    /// after the feed's own singles.
    #[test]
    fn adr_0084_singles_group_rows_r32_11_page_lists_matched_singles_under_their_heading() {
        let tree = LibraryTree {
            artists: vec![artist(
                "Haleen",
                vec![
                    with_publisher(single("A", 1, Some(1), true), HALEEN),
                    single("B", 2, Some(2), true),
                ],
            )],
        };
        let groups = singles_groups(&tree);
        let page = SinglesGroupPageVm::new(&groups[0], &BusyItems::default());
        let heading = page.matched_by_name.expect("one matched row");
        assert_eq!(heading.heading, PublisherPageVm::MATCHED_BY_NAME_HEADING);
        assert_eq!(heading.note, PublisherPageVm::MATCHED_BY_NAME_NOTE);
        assert_eq!(heading.count, 1);
        assert_eq!(page.rows.len(), 2);
        assert_eq!(
            *page.rows[1].key(),
            ReleaseKey::FeedGuid("guid-2".to_owned()),
            "B's row comes after A's own row"
        );
    }

    /// R32-12: `library_single_artist_feeds` gives the feed GUID of a
    /// feed whose Library singles are only `matched_by_name`, once, with
    /// no duplicate when the feed also has its own singles.
    #[test]
    fn adr_0084_singles_group_rows_r32_12_library_single_artist_feeds_includes_matched_feeds() {
        let own_and_matched = LibraryTree {
            artists: vec![artist(
                "Haleen",
                vec![
                    with_publisher(single("A", 1, Some(1), true), HALEEN),
                    single("B", 2, Some(2), true),
                ],
            )],
        };
        assert_eq!(
            library_single_artist_feeds(&own_and_matched),
            vec![HALEEN.to_owned()],
            "once, with no duplicate"
        );

        let matched_only = LibraryTree {
            artists: vec![artist(
                "Haleen",
                vec![
                    with_publisher(
                        release(
                            "Long Player",
                            1,
                            vec![track(10, 1, Some(1), true), track(11, 1, Some(1), true)],
                            None,
                        ),
                        HALEEN,
                    ),
                    single("B", 2, Some(2), true),
                    single("C", 3, Some(3), true),
                ],
            )],
        };
        assert_eq!(
            library_single_artist_feeds(&matched_only),
            vec![HALEEN.to_owned()],
            "the feed has no own single, only matched ones"
        );
    }

    /// R32-13: "All n singles" on the release page of a matched single
    /// counts the joined group.
    #[test]
    fn adr_0084_singles_group_rows_r32_13_name_link_counts_the_joined_group() {
        let tree = LibraryTree {
            artists: vec![artist(
                "Haleen",
                vec![
                    with_publisher(single("A", 1, Some(1), true), HALEEN),
                    single("B", 2, Some(2), true),
                ],
            )],
        };
        let empty = IndexFeedSingles::default();
        let groups = singles_groups_everything(&tree, &empty);
        let link = singles_group_name_link(&groups, &ReleaseKey::FeedGuid("guid-2".to_owned()))
            .expect("B's key is in the joined group");
        assert_eq!(link.label, "All 2 singles");
        assert_eq!(
            link.target,
            AlbumNameLinkTarget::SinglesGroup(SinglesGroupKey(ArtistIdentity::Publisher(
                HALEEN.to_owned()
            )))
        );
    }

    /// R32-15: `main_action`'s download plan and "Add to playlist" list
    /// each include a `matched_by_name` single of the group.
    #[test]
    fn adr_0084_singles_group_rows_r32_15_main_action_includes_matched_singles() {
        let missing = LibraryTree {
            artists: vec![artist(
                "Haleen",
                vec![
                    with_publisher(single("A", 1, Some(1), true), HALEEN),
                    single("B", 2, Some(2), false),
                ],
            )],
        };
        let groups = singles_groups(&missing);
        let action = main_action(&groups[0], false);
        let SinglesGroupAction::DownloadSingles(plan) = action.action else {
            panic!("B is missing");
        };
        assert_eq!(
            plan.library_feed_ids,
            vec![2],
            "B's feed id, from matched_by_name"
        );

        let all_in = LibraryTree {
            artists: vec![artist(
                "Haleen",
                vec![
                    with_publisher(single("A", 1, Some(1), true), HALEEN),
                    single("B", 2, Some(2), true),
                ],
            )],
        };
        let groups = singles_groups(&all_in);
        let action = main_action(&groups[0], false);
        let SinglesGroupAction::AddToPlaylist(mut ids) = action.action else {
            panic!("all in Library");
        };
        ids.sort_unstable();
        assert_eq!(ids, vec![1, 2], "A and B");
    }

    /// R15-03: two singles that name one publisher feed make one group,
    /// also under two artist names.
    #[test]
    fn adr_0084_singles_group_r15_03_one_publisher_groups_across_names() {
        let tree = LibraryTree {
            artists: vec![
                artist(
                    "Haleen",
                    vec![with_publisher(single("A", 1, Some(1), true), "pub")],
                ),
                artist(
                    "Haleen Band",
                    vec![with_publisher(single("B", 2, Some(2), true), "pub")],
                ),
            ],
        };
        let groups = singles_groups(&tree);
        assert_eq!(groups.len(), 1);
        assert_eq!(
            groups[0].identity,
            ArtistIdentity::Publisher("pub".to_owned())
        );
        assert_eq!(
            groups[0].display_artist, "Haleen Band",
            "the display artist is the artist of the newest single"
        );
    }

    /// R15-04: the singles come newest first. A single with no date comes
    /// last, and equal dates sort by the lowercase release name.
    #[test]
    fn adr_0084_singles_group_r15_04_singles_come_newest_first() {
        let tree = LibraryTree {
            artists: vec![artist(
                "Haleen",
                vec![
                    single("Undated", 1, None, true),
                    single("Old", 2, Some(100), true),
                    single("beta", 3, Some(300), true),
                    single("Alpha", 4, Some(300), true),
                    single("Newest", 5, Some(500), true),
                ],
            )],
        };
        let groups = singles_groups(&tree);
        assert_eq!(
            names(&groups[0]),
            vec!["Newest", "Alpha", "beta", "Old", "Undated"]
        );
    }

    /// R15-07: one or two singles not in the Library give "Download 1
    /// single" or "Download 2 singles". None gives "Add to playlist". Each
    /// action is available and has its accessibility label.
    #[test]
    fn adr_0084_singles_group_r15_07_page_main_action_follows_the_library_state() {
        let key = SinglesGroupKey(ArtistIdentity::Name("haleen".to_owned()));

        let page = SinglesGroupPageVm::from_tree(
            &haleen_tree(3, 2),
            &IndexFeedSingles::default(),
            &key,
            &BusyItems::default(),
        );
        let action = page.main_action.expect("a group has a main action");
        assert_eq!(action.label, "Download 1 single");
        assert_eq!(
            action.a11y_label,
            "Download the 1 single that is not in the Library"
        );
        assert!(action.available);
        assert_eq!(
            action.action,
            SinglesGroupAction::DownloadSingles(ReleasesDownloadPlan {
                library_feed_ids: vec![3],
                index_releases: Vec::new(),
            })
        );

        let page = SinglesGroupPageVm::from_tree(
            &haleen_tree(3, 1),
            &IndexFeedSingles::default(),
            &key,
            &BusyItems::default(),
        );
        let action = page.main_action.expect("a group has a main action");
        assert_eq!(action.label, "Download 2 singles");
        assert_eq!(
            action.a11y_label,
            "Download the 2 singles that are not in the Library"
        );
        assert!(action.available);
        assert_eq!(
            action.action,
            SinglesGroupAction::DownloadSingles(ReleasesDownloadPlan {
                library_feed_ids: vec![3, 2],
                index_releases: Vec::new(),
            }),
            "the downloads run newest first"
        );

        let page = SinglesGroupPageVm::from_tree(
            &haleen_tree(7, 7),
            &IndexFeedSingles::default(),
            &key,
            &BusyItems::default(),
        );
        assert_eq!(page.title, "Singles by Haleen");
        assert_eq!(
            page.subtitle.as_deref(),
            Some("7 singles · In Library · By name")
        );
        let action = page.main_action.expect("a group has a main action");
        assert_eq!(action.label, "Add to playlist");
        assert_eq!(action.a11y_label, "Add the 7 singles to a playlist");
        assert!(action.available);
        assert_eq!(
            action.action,
            SinglesGroupAction::AddToPlaylist(vec![7, 6, 5, 4, 3, 2, 1])
        );
        assert_eq!(page.rows.len(), 7);
        assert_eq!(
            *page.rows[0].key(),
            ReleaseKey::FeedGuid("guid-7".to_owned())
        );
        assert!(library_row(&page.rows[0]).date_text.is_some());
    }

    /// R15-07: each level gives its identity note and names its level in
    /// the subtitle. A key with no group gives the empty state.
    #[test]
    fn adr_0084_singles_group_r15_07_page_identity_note_and_empty_state() {
        let name_key = SinglesGroupKey(ArtistIdentity::Name("haleen".to_owned()));
        let page = SinglesGroupPageVm::from_tree(
            &haleen_tree(2, 1),
            &IndexFeedSingles::default(),
            &name_key,
            &BusyItems::default(),
        );
        assert_eq!(page.identity_note, Some(NAME_LEVEL_NOTE));
        assert!(!library_row(&page.rows[0]).track.is_in_library);
        assert!(library_row(&page.rows[1]).track.is_in_library);

        let publisher_tree = LibraryTree {
            artists: vec![artist(
                "Haleen",
                vec![
                    with_publisher(single("A", 1, Some(1), true), "pub"),
                    with_publisher(single("B", 2, Some(2), true), "pub"),
                ],
            )],
        };
        let publisher_key = SinglesGroupKey(ArtistIdentity::Publisher("pub".to_owned()));
        let page = SinglesGroupPageVm::from_tree(
            &publisher_tree,
            &IndexFeedSingles::default(),
            &publisher_key,
            &BusyItems::default(),
        );
        assert_eq!(page.identity_note, Some(PUBLISHER_LEVEL_NOTE));
        assert_eq!(
            page.subtitle.as_deref(),
            Some("2 singles · In Library · Artist feed")
        );
        assert_eq!(page.rows.len(), 2);

        let page = SinglesGroupPageVm::from_tree(
            &haleen_tree(1, 1),
            &IndexFeedSingles::default(),
            &name_key,
            &BusyItems::default(),
        );
        assert_eq!(page.empty_message, Some(EMPTY_GROUP_MESSAGE));
        assert!(page.main_action.is_none());
        assert!(page.rows.is_empty());
    }

    /// R11-05 (ADR 0084 Decision 12): the group row and the group page
    /// give the newest four images of the singles in the logo mosaic. The
    /// empty state gives no art.
    #[test]
    fn adr_0084_logo_frame_r11_05_group_row_and_page_give_the_newest_four() {
        let key = SinglesGroupKey(ArtistIdentity::Name("haleen".to_owned()));
        let tree = haleen_tree(5, 5);
        let newest_four =
            CollectionArtVm::Mosaic(["5.jpg", "4.jpg", "3.jpg", "2.jpg"].map(str::to_owned));

        let page = SinglesGroupPageVm::from_tree(
            &tree,
            &IndexFeedSingles::default(),
            &key,
            &BusyItems::default(),
        );
        assert_eq!(page.art, Some(newest_four.clone()));

        let groups = singles_groups(&tree);
        let row = SinglesGroupRowVm::new(&groups[0], false, |_, _| None);
        assert_eq!(row.art, newest_four);

        let empty = SinglesGroupPageVm::from_tree(
            &haleen_tree(1, 1),
            &IndexFeedSingles::default(),
            &key,
            &BusyItems::default(),
        );
        assert_eq!(empty.art, None);
    }

    /// R24-05 (ADR 0084 packet 024): a singles group of a feed-level artist
    /// gives the channel image of that feed, the same as its Artists row,
    /// on the group row and the group page. A group of a name only gives
    /// none.
    #[test]
    fn adr_0084_artist_image_r24_05_feed_group_gives_the_feed_image() {
        let with_image = |album: AlbumNode, image: Option<&str>| {
            let mut album = with_publisher(album, "pub");
            album.named_feeds[0].image_url = image.map(str::to_owned);
            album
        };
        let tree = LibraryTree {
            artists: vec![artist(
                "Haleen",
                vec![
                    with_image(single("A", 1, Some(1), true), None),
                    with_image(
                        single("B", 2, Some(2), true),
                        Some(" https://img.test/pub.jpg "),
                    ),
                ],
            )],
        };
        let expected = CollectionArtVm::ProvidedImage {
            url: "https://img.test/pub.jpg".to_owned(),
            fallback: Box::new(CollectionArtVm::Mosaic(
                ["2.jpg", "1.jpg", "1.jpg", "2.jpg"].map(str::to_owned),
            )),
        };

        let groups = singles_groups(&tree);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].collection_art(), expected);
        let row = SinglesGroupRowVm::new(&groups[0], false, |_, _| None);
        assert_eq!(row.art, expected);
        let key = SinglesGroupKey(ArtistIdentity::Publisher("pub".to_owned()));
        let page = SinglesGroupPageVm::from_tree(
            &tree,
            &IndexFeedSingles::default(),
            &key,
            &BusyItems::default(),
        );
        assert_eq!(page.art, Some(expected));

        let name_tree = haleen_tree(2, 2);
        let name_groups = singles_groups(&name_tree);
        assert!(!matches!(
            name_groups[0].collection_art(),
            CollectionArtVm::ProvidedImage { .. }
        ));
    }

    /// Operator finding of 2026-10-20: the channel image belongs to the
    /// feed. Two albums name the feed "pub", and only the album "Long
    /// Player" carries its image. The singles group of the other albums,
    /// on the group row and the group page, and the Artists row of the
    /// feed each give `ProvidedImage`. The test also uses an agreed role
    /// other than `artist` for "Long Player". Then "Long Player" is not a
    /// member of the Artists row of the feed.
    #[test]
    fn adr_0084_row_fix_feed_image_comes_from_each_album_of_the_feed() {
        const IMAGE: &str = "https://img.test/pub.jpg";
        for role in [None, Some("label")] {
            let mut long_player = with_publisher(
                release(
                    "Long Player",
                    9,
                    vec![track(90, 9, Some(50), true), track(91, 9, Some(50), true)],
                    None,
                ),
                "pub",
            );
            long_player.named_feeds[0].role = role.map(str::to_owned);
            long_player.named_feeds[0].role_agreement = role.map(|_| "both".to_owned());
            long_player.named_feeds[0].image_url = Some(IMAGE.to_owned());
            let tree = LibraryTree {
                artists: vec![artist(
                    "Haleen",
                    vec![
                        with_publisher(single("A", 1, Some(100), true), "pub"),
                        with_publisher(single("B", 2, Some(200), true), "pub"),
                        long_player,
                    ],
                )],
            };
            let gives_image = |art: &CollectionArtVm| matches!(art, CollectionArtVm::ProvidedImage { url, .. } if url == IMAGE);

            let groups = singles_groups(&tree);
            assert_eq!(groups.len(), 1, "{role:?}");
            assert!(gives_image(&groups[0].collection_art()), "{role:?}");
            let row = SinglesGroupRowVm::new(&groups[0], false, |_, _| None);
            assert!(gives_image(&row.art), "{role:?}");
            let key = SinglesGroupKey(ArtistIdentity::Publisher("pub".to_owned()));
            let page = SinglesGroupPageVm::from_tree(
                &tree,
                &IndexFeedSingles::default(),
                &key,
                &BusyItems::default(),
            );
            assert!(page.art.as_ref().is_some_and(gives_image), "{role:?}");

            let mut vm = crate::view_models::library::LibraryViewModel::new();
            vm.replace_tree(tree.clone());
            let artist_row = vm
                .artists_list(
                    crate::view_models::index_name_list::MusicListScope::Library,
                    &crate::view_models::index_name_list::IndexNameListPage::default(),
                )
                .into_iter()
                .find(|row| row.element_id == "music-artist-row-publisher:pub")
                .expect("the feed has an Artists row");
            assert!(artist_row.art.as_ref().is_some_and(gives_image), "{role:?}");
        }
    }

    /// ADR 0084 task 015, operator decision of 2026-10-17: a page row
    /// gives the shared Library track row the track of its single, the
    /// date text of the single and the release key that a click opens. A
    /// single with no date gives no date text. A busy track or a busy feed
    /// makes its row busy, and only that row.
    #[test]
    fn adr_0084_singles_group_page_row_exposes_date_text_and_track() {
        let key = SinglesGroupKey(ArtistIdentity::Name("haleen".to_owned()));
        let tree = LibraryTree {
            artists: vec![artist(
                "Haleen",
                vec![
                    single("Dated", 1, Some(1_712_275_200), true),
                    single("Undated", 2, None, false),
                ],
            )],
        };
        let page = SinglesGroupPageVm::from_tree(
            &tree,
            &IndexFeedSingles::default(),
            &key,
            &BusyItems::default(),
        );
        assert_eq!(page.rows.len(), 2);

        let dated = library_row(&page.rows[0]);
        assert_eq!(dated.track.id, 10, "the row holds the track of its single");
        assert_eq!(dated.track.feed_id, 1);
        assert_eq!(dated.date_text.as_deref(), Some("Apr 5, 2024"));
        assert_eq!(dated.key, ReleaseKey::FeedGuid("guid-1".to_owned()));
        assert_eq!(dated.work, TrackRowWork::Idle);

        let undated = library_row(&page.rows[1]);
        assert_eq!(undated.track.id, 20);
        assert_eq!(undated.date_text, None);
        assert_eq!(undated.key, ReleaseKey::FeedGuid("guid-2".to_owned()));

        let busy_track = BusyItems::new(
            DownloadActivity {
                tracks: BTreeSet::from([20]),
                ..DownloadActivity::default()
            },
            BTreeSet::new(),
        );
        let page =
            SinglesGroupPageVm::from_tree(&tree, &IndexFeedSingles::default(), &key, &busy_track);
        assert_eq!(library_row(&page.rows[0]).work, TrackRowWork::Idle);
        assert_eq!(
            library_row(&page.rows[1]).work,
            TrackRowWork::Queued,
            "the queued track makes its row busy"
        );
        let busy_feed = BusyItems::new(DownloadActivity::default(), BTreeSet::from([1]));
        let page =
            SinglesGroupPageVm::from_tree(&tree, &IndexFeedSingles::default(), &key, &busy_feed);
        assert_eq!(
            library_row(&page.rows[0]).work,
            TrackRowWork::Removing,
            "the busy feed makes its row busy"
        );
        assert_eq!(library_row(&page.rows[1]).work, TrackRowWork::Idle);
    }

    /// R15-08: the release page name links of a single in a group of 7
    /// include "All 7 singles" with the group key. A single in no group and
    /// an album give no such link.
    #[test]
    fn adr_0084_singles_group_r15_08_single_page_links_to_its_group() {
        let base = vec![AlbumNameLinkVm {
            label: "Haleen".to_owned(),
            a11y_label: "Open the artist Haleen".to_owned(),
            note: None,
            target: AlbumNameLinkTarget::Artist("Haleen".to_owned()),
        }];
        let mut tree = haleen_tree(7, 5);
        tree.artists.push(artist(
            "Solo",
            vec![
                single("Only One", 50, Some(1), true),
                release(
                    "Long Player",
                    60,
                    vec![track(600, 60, Some(1), true), track(601, 60, Some(1), true)],
                    None,
                ),
            ],
        ));
        let groups = singles_groups(&tree);

        let links = release_page_name_links(
            base.clone(),
            &groups,
            Some(&ReleaseKey::FeedGuid("guid-3".to_owned())),
        );
        assert_eq!(links.len(), 2);
        assert_eq!(links[0], base[0], "the existing links come first");
        assert_eq!(links[1].label, "All 7 singles");
        assert_eq!(links[1].a11y_label, "Open all 7 singles by Haleen");
        assert_eq!(
            links[1].target,
            AlbumNameLinkTarget::SinglesGroup(SinglesGroupKey(ArtistIdentity::Name(
                "haleen".to_owned()
            )))
        );

        for key in ["guid-50", "guid-60"] {
            let links = release_page_name_links(
                base.clone(),
                &groups,
                Some(&ReleaseKey::FeedGuid(key.to_owned())),
            );
            assert_eq!(links, base, "{key} must get no group link");
        }
    }

    // ADR 0084 packet 026: the groups of the scope "Everything".

    use crate::view_models::index_feed_singles::tests::single_naming;
    use crate::view_models::index_feed_singles::{FeedReleaseList, IndexFeedSingles};

    /// The Library artist feed of Haleen (packet 008 record).
    const HALEEN: &str = "64569161-a8f7-52e5-8aae-9e0385043149";
    /// The second Haleen feed with the same title (packet 028 D4).
    const HALEEN_2: &str = "fe80a515-2826-4a46-aa37-b27f38d3c30f";
    /// Sir Libre Records.
    const SIR_LIBRE: &str = "34bf795f-f551-4db1-8877-ad19aa5f89c8";

    fn fills(feed_guid: &str, rows: Vec<Feed>) -> IndexFeedSingles {
        let mut store = IndexFeedSingles::default();
        let generation = store.begin_request();
        store.record(feed_guid, FeedReleaseList::complete(rows), generation);
        store
    }

    /// The recorded "You Are Enough" row: it names the second Haleen feed
    /// at rank 1 with `two_way_validated` false.
    fn you_are_enough() -> Feed {
        let mut feed = single_naming("you-are-enough", "You Are Enough", HALEEN_2, 50);
        feed.named_feeds.as_mut().unwrap()[0].two_way_validated = Some(false);
        feed
    }

    /// A recorded Sir Libre Records row: an artist feed at rank 1, and the
    /// label at rank 2 with `label` and `both`.
    fn sir_libre_row(release_guid: &str, artist_feed: &str) -> Feed {
        serde_json::from_value(serde_json::json!({
            "feed_guid": release_guid, "title": release_guid,
            "release_kind": "unknown", "episode_count": 1, "newest_item_at": 10,
            "named_feeds": [
                {"publisher_feed_guid": artist_feed, "album_names_rank": 1,
                 "two_way_validated": true},
                {"publisher_feed_guid": SIR_LIBRE, "remote_feed_title": "Sir Libre Records",
                 "album_names_rank": 2, "role": "label", "role_agreement": "both",
                 "two_way_validated": true}
            ]
        }))
        .unwrap()
    }

    /// R32-05 (ADR 0084 packet 032): `SinglesGroupIndexRowVm::new` reads
    /// the duration from the first track of its `Feed`, and gives `None`
    /// when the `Feed` has no `tracks`.
    #[test]
    fn adr_0084_singles_group_rows_r32_05_index_row_reads_duration_from_tracks() {
        let plain = single_naming("i9", "Night Drive", HALEEN, 1);
        assert!(
            plain.tracks.is_none(),
            "the recorded row has no tracks fact"
        );
        let no_duration = IndexGroupSingle {
            feed: &plain,
            key: ReleaseKey::FeedGuid("i9".to_owned()),
            date: Some(1),
            sort_key: "night drive".to_owned(),
        };
        let row = SinglesGroupIndexRowVm::new(&no_duration, TrackRowWork::Idle);
        assert_eq!(row.duration, None);

        let mut with_track = plain;
        with_track.tracks = Some(vec![crate::api::Track {
            duration_secs: Some(225),
            ..crate::api::Track::default()
        }]);
        let dated = IndexGroupSingle {
            feed: &with_track,
            key: ReleaseKey::FeedGuid("i9".to_owned()),
            date: Some(1),
            sort_key: "night drive".to_owned(),
        };
        let row = SinglesGroupIndexRowVm::new(&dated, TrackRowWork::Idle);
        assert_eq!(row.duration.as_deref(), Some("3:45"));
    }

    /// R26-04: the identity of a recorded Haleen single is the feed. "You
    /// Are Enough" and a release with no named feed have the name level.
    #[test]
    fn adr_0084_index_singles_r26_04_identity_of_index_rows() {
        let haleen = single_naming("h1", "A Haleen single", HALEEN, 1);
        assert_eq!(
            index_release_identity(&haleen),
            ArtistIdentity::Publisher(HALEEN.to_owned())
        );
        assert_eq!(index_single_artist_feed(&haleen).as_deref(), Some(HALEEN));
        assert_eq!(
            index_release_identity(&you_are_enough()),
            ArtistIdentity::Name("haleen".to_owned())
        );
        assert_eq!(index_single_artist_feed(&you_are_enough()), None);
        let mut empty = single_naming("e1", "No feed", HALEEN, 1);
        empty.named_feeds = Some(Vec::new());
        assert_eq!(
            index_release_identity(&empty),
            ArtistIdentity::Name("haleen".to_owned())
        );
        let mut album = single_naming("album", "An album", HALEEN, 1);
        album.episode_count = Some(11);
        assert_eq!(index_single_artist_feed(&album), None, "not a single");
    }

    /// R26-05: a rank 2 entry with `label` and `both` makes no group of Sir
    /// Libre Records, also when the app read the list of that feed.
    #[test]
    fn adr_0084_index_singles_r26_05_a_label_at_rank_2_makes_no_group() {
        let rows = vec![
            sir_libre_row("sl-1", "artist-a"),
            sir_libre_row("sl-2", "artist-b"),
            sir_libre_row("sl-3", "artist-c"),
        ];
        let store = fills(SIR_LIBRE, rows.clone());
        assert_eq!(store.iter().next().unwrap().1.len(), 0);
        let tree = LibraryTree::default();
        assert!(singles_groups_everything(&tree, &store).is_empty());
        assert_eq!(
            index_single_artist_feed(&rows[0]).as_deref(),
            Some("artist-a")
        );
    }

    /// R26-10, R26-14 and R26-16: one Library single and one `MusicIndex`
    /// single of one feed make one group. The count is the size of the
    /// union by release key. The `MusicIndex` row of a Library release is
    /// not a second member (decision D8).
    #[test]
    fn adr_0084_index_singles_r26_10_library_and_index_singles_make_one_group() {
        let tree = LibraryTree {
            artists: vec![artist(
                "Haleen",
                vec![
                    with_publisher(single("Library One", 1, Some(100), true), HALEEN),
                    with_publisher(single("Library Two", 2, Some(200), false), HALEEN),
                ],
            )],
        };
        let mut library_copy = single_naming("guid-1", "Library One", HALEEN, 100);
        library_copy.title = Some("Library One".to_owned());
        let store = fills(
            HALEEN,
            vec![
                single_naming("i1", "Index One", HALEEN, 300),
                library_copy,
                single_naming("i1", "Index One", HALEEN, 300),
            ],
        );
        let groups = singles_groups_everything(&tree, &store);
        assert_eq!(groups.len(), 1);
        let group = &groups[0];
        assert_eq!(group.identity, ArtistIdentity::Publisher(HALEEN.to_owned()));
        assert_eq!(
            group.count(),
            3,
            "two Library singles and one MusicIndex single"
        );
        assert_eq!(group.index_singles.len(), 1);
        assert!(group.contains(&ReleaseKey::FeedGuid("i1".to_owned())));
        assert_eq!(group.display_artist, "Haleen");
        assert_eq!(
            group.summary_text(),
            "3 singles \u{b7} 1 in Library \u{b7} Artist feed"
        );

        let one_library = LibraryTree {
            artists: vec![artist(
                "Haleen",
                vec![with_publisher(
                    single("Library One", 1, Some(100), true),
                    HALEEN,
                )],
            )],
        };
        let store = fills(HALEEN, vec![single_naming("i1", "Index One", HALEEN, 300)]);
        let groups = singles_groups_everything(&one_library, &store);
        assert_eq!(
            groups.len(),
            1,
            "one Library single and one MusicIndex single"
        );
        assert_eq!(
            groups[0].summary_text(),
            "2 singles \u{b7} 1 in Library \u{b7} Artist feed"
        );
    }

    /// R26-16 and decision D9: a group with no Library single shows no
    /// Library part, and its label is the title of the artist feed.
    #[test]
    fn adr_0084_index_singles_r26_16_group_text_with_no_library_single() {
        let store = fills(
            HALEEN_2,
            (1..=38)
                .map(|n| single_naming(&format!("h2-{n}"), &format!("Single {n}"), HALEEN_2, n))
                .collect(),
        );
        let tree = LibraryTree::default();
        let groups = singles_groups_everything(&tree, &store);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].summary_text(), "38 singles \u{b7} Artist feed");
        assert_eq!(groups[0].display_artist, "Haleen");
        let row = SinglesGroupRowVm::new(&groups[0], false, |_, _| None);
        assert_eq!(row.label, "Haleen");
        assert_eq!(row.secondary_text, "38 singles \u{b7} Artist feed");
        assert_eq!(row.a11y_label, "Haleen, 38 singles, Artist feed");
        assert_eq!(row.single_keys.len(), 38);

        let all_in = LibraryTree {
            artists: vec![artist(
                "Haleen",
                vec![
                    with_publisher(single("A", 1, Some(1), true), HALEEN),
                    with_publisher(single("B", 2, Some(2), true), HALEEN),
                ],
            )],
        };
        let empty = IndexFeedSingles::default();
        let groups = singles_groups_everything(&all_in, &empty);
        assert_eq!(
            groups[0].summary_text(),
            "2 singles \u{b7} In Library \u{b7} Artist feed"
        );

        let mut no_title = single_naming("x1", "X", "feed-x", 2);
        no_title.named_feeds.as_mut().unwrap()[0].remote_feed_title = None;
        let store = fills(
            "feed-x",
            vec![no_title, single_naming("x2", "Y", "feed-x", 1)],
        );
        let groups = singles_groups_everything(&tree, &store);
        assert_eq!(
            groups[0].display_artist, "Haleen",
            "the release_artist of the newest single"
        );
    }

    /// R26-12 and R26-13: one `MusicIndex` single of a feed makes no
    /// group. A single at the name level joins no group, also when a
    /// Library name group has the same key (decision D6).
    #[test]
    fn adr_0084_index_singles_r26_12_one_single_and_name_level_make_no_index_group() {
        let tree = LibraryTree::default();
        let store = fills(HALEEN, vec![single_naming("i1", "Only", HALEEN, 1)]);
        assert!(singles_groups_everything(&tree, &store).is_empty());

        let name_tree = haleen_tree(2, 2);
        let store = fills(HALEEN_2, vec![you_are_enough()]);
        let groups = singles_groups_everything(&name_tree, &store);
        assert_eq!(groups.len(), 1);
        assert_eq!(
            groups[0].identity,
            ArtistIdentity::Name("haleen".to_owned())
        );
        assert_eq!(
            groups[0].count(),
            2,
            "the name group keeps its Library singles"
        );
        assert!(!groups[0].contains(&ReleaseKey::FeedGuid("you-are-enough".to_owned())));
    }

    /// R26-17: the open group gives its members newest first. A member with
    /// no date comes last, and equal dates sort by the sort key of the
    /// title. A `MusicIndex` member uses the `MusicIndex` release row. The
    /// group art gives the feed image when the Library knows it, else the
    /// newest four images.
    #[test]
    fn adr_0084_index_singles_r26_17_open_group_gives_members_newest_first() {
        let tree = LibraryTree {
            artists: vec![artist(
                "Haleen",
                vec![
                    with_publisher(single("Library Mid", 1, Some(200), true), HALEEN),
                    with_publisher(single("Library Undated", 2, None, false), HALEEN),
                ],
            )],
        };
        let mut undated = single_naming("i-undated", "Index Undated", HALEEN, 0);
        undated.newest_item_at = None;
        let store = fills(
            HALEEN,
            vec![
                single_naming("i-new", "Index New", HALEEN, 300),
                single_naming("i-tie-b", "B tie", HALEEN, 200),
                undated,
            ],
        );
        let groups = singles_groups_everything(&tree, &store);
        let library_row = |artist: &ArtistNode, album: &AlbumNode| {
            Some(DestinationListRowVm {
                element_id: album.name.clone(),
                label: album.name.clone(),
                secondary_text: format!("{} \u{b7} Library", artist.name),
                a11y_label: String::new(),
                thumbnail_href: None,
                art: None,
                open: crate::view_models::library::DestinationListRowTarget::Release(
                    album.release_key().unwrap(),
                ),
                selection: RowSelectionVm::none(),
                release_kind: None,
                menu: None,
            })
        };
        let row = SinglesGroupRowVm::new(&groups[0], true, library_row);
        let labels: Vec<&str> = row.singles.iter().map(|row| row.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "Index New",
                "B tie",
                "Library Mid",
                "Index Undated",
                "Library Undated"
            ]
        );
        assert_eq!(row.singles[0].secondary_text, "Haleen \u{b7} Single");
        assert_eq!(row.singles[2].secondary_text, "Haleen \u{b7} Library");
        assert_eq!(row.single_keys.len(), 5);
        assert_eq!(row.single_keys[0], ReleaseKey::FeedGuid("i-new".to_owned()));
        assert_eq!(
            row.art,
            CollectionArtVm::Mosaic(
                [
                    "https://img.test/i-new.jpg",
                    "https://img.test/i-tie-b.jpg",
                    "1.jpg",
                    "https://img.test/i-undated.jpg"
                ]
                .map(str::to_owned)
            )
        );
        assert_eq!(groups[0].first_title_sort_key(), "b tie");
    }

    /// Orchestrator decision of 2026-10-22, which replaces D2 of packet
    /// 032: the Releases list disclosure of a group row lists every
    /// single the group row counts, so the two numbers agree. The
    /// matched-by-name single comes after the feed's own single, and its
    /// subtitle marks the inferred relation.
    #[test]
    fn adr_0084_singles_group_rows_d2_disclosure_lists_every_counted_member() {
        let tree = LibraryTree {
            artists: vec![artist(
                "Haleen",
                vec![
                    with_publisher(single("A", 1, Some(1), true), HALEEN),
                    single("B", 2, Some(2), true),
                ],
            )],
        };
        let groups = singles_groups(&tree);
        let group = &groups[0];
        assert_eq!(
            group.count(),
            2,
            "A, the feed's own single, and B, matched by name"
        );
        let library_row = |artist: &ArtistNode, album: &AlbumNode| {
            Some(DestinationListRowVm {
                element_id: album.name.clone(),
                label: album.name.clone(),
                secondary_text: format!("{} \u{b7} Single", artist.name),
                a11y_label: String::new(),
                thumbnail_href: None,
                art: None,
                open: crate::view_models::library::DestinationListRowTarget::Release(
                    album.release_key().unwrap(),
                ),
                selection: RowSelectionVm::none(),
                release_kind: None,
                menu: None,
            })
        };
        let row = SinglesGroupRowVm::new(group, true, library_row);
        assert_eq!(
            row.singles.len(),
            group.count(),
            "the disclosure lists every single the group row counts"
        );
        assert_eq!(row.single_keys.len(), group.count());
        assert_eq!(
            row.singles[0].label, "A",
            "the feed's own single comes first"
        );
        assert_eq!(
            row.singles[1].label, "B",
            "the matched single comes after it"
        );
        assert_eq!(
            row.singles[1].secondary_text, "Haleen \u{b7} Single \u{b7} Matched by name",
            "the matched single's subtitle marks the inferred relation"
        );
        assert_eq!(
            row.single_keys[1], group.matched_by_name[0].key,
            "the checkbox of the group row also toggles the matched single"
        );
        assert!(
            row.singles[1].a11y_label.ends_with(", Matched by name"),
            "a screen reader also hears the inferred relation"
        );
    }

    /// Packet 032 review: a group of matched singles alone takes its place
    /// and its art from those singles. A group with other members keeps
    /// its place from them (decision D3).
    #[test]
    fn adr_0084_singles_group_rows_matched_only_group_keeps_its_place() {
        let tree = LibraryTree {
            artists: vec![artist(
                "Haleen",
                vec![
                    with_publisher(single("Zed", 1, Some(1), true), HALEEN),
                    single("Beta", 2, Some(2), true),
                    single("Alpha", 3, Some(3), true),
                ],
            )],
        };
        let groups = singles_groups(&tree);
        let group = &groups[0];
        assert_eq!(group.matched_by_name.len(), 2);
        assert_eq!(
            group.first_title_sort_key(),
            name_sort_key("Zed"),
            "the own single gives the place; a matched single does not move the row"
        );
        let mut matched_only = group.clone();
        matched_only.singles.clear();
        assert_eq!(
            matched_only.first_title_sort_key(),
            name_sort_key("Alpha"),
            "with no other member, the matched singles give the place"
        );
    }

    /// R26-19: the scope "Library" keeps its groups: `singles_groups` gives
    /// no `MusicIndex` single, and a group of Library singles keeps its
    /// order of singles.
    #[test]
    fn adr_0084_index_singles_r26_19_library_groups_have_no_index_single() {
        let tree = haleen_tree(3, 2);
        let groups = singles_groups(&tree);
        assert!(groups[0].index_singles.is_empty());
        assert_eq!(names(&groups[0]), ["Single 3", "Single 2", "Single 1"]);
        let members: Vec<ReleaseKey> = groups[0]
            .members()
            .iter()
            .map(|member| member.key().clone())
            .collect();
        let singles: Vec<ReleaseKey> = groups[0]
            .singles
            .iter()
            .map(|single| single.key.clone())
            .collect();
        assert_eq!(members, singles);
    }

    // ADR 0084 packet 026 Part B: the group page and the link.

    /// The Haleen tree of the D12 example: three Library singles of the
    /// feed, each in the Library, with the dates 1000, 2000 and 3000.
    fn haleen_feed_tree() -> LibraryTree {
        LibraryTree {
            artists: vec![artist(
                "Haleen",
                (1..=3)
                    .map(|n| {
                        with_publisher(
                            single(&format!("Library {n}"), n, Some(n * 1000), true),
                            HALEEN,
                        )
                    })
                    .collect(),
            )],
        }
    }

    /// The 52 `MusicIndex` singles of the D12 example, with the dates 1 to
    /// 52 and the release GUIDs "i1" to "i52".
    fn haleen_index_rows() -> Vec<Feed> {
        (1..=52)
            .map(|n| single_naming(&format!("i{n}"), &format!("Index {n}"), HALEEN, n))
            .collect()
    }

    fn haleen_key() -> SinglesGroupKey {
        SinglesGroupKey(ArtistIdentity::Publisher(HALEEN.to_owned()))
    }

    fn row_keys(page: &SinglesGroupPageVm) -> Vec<String> {
        page.rows
            .iter()
            .map(|row| match row.key() {
                ReleaseKey::FeedGuid(guid) => guid.clone(),
                ReleaseKey::LibraryFeed(feed_id) => feed_id.to_string(),
            })
            .collect()
    }

    /// R26-20 and decision D12: the group page of a feed gives the Library
    /// rows and the `MusicIndex` rows, newest first. The page is not
    /// scoped: the group row of the scope "Library" opens the key of this
    /// same page, and the view model of the page reads no scope.
    #[test]
    fn adr_0084_index_singles_r26_20_group_page_gives_both_origins_newest_first() {
        let tree = haleen_feed_tree();
        let store = fills(HALEEN, haleen_index_rows());
        let page =
            SinglesGroupPageVm::from_tree(&tree, &store, &haleen_key(), &BusyItems::default());
        assert_eq!(page.rows.len(), 55);
        let keys = row_keys(&page);
        assert_eq!(keys[..4], ["guid-3", "guid-2", "guid-1", "i52"]);
        assert_eq!(keys[54], "i1");
        assert!(matches!(page.rows[0], SinglesGroupPageEntryVm::Library(_)));
        let SinglesGroupPageEntryVm::Index(index_row) = &page.rows[3] else {
            panic!("a MusicIndex row");
        };
        assert_eq!(index_row.title, "Index 52");
        assert_eq!(
            index_row.image_url.as_deref(),
            Some("https://img.test/i52.jpg")
        );
        assert_eq!(index_row.element_id, "singles-group-index-row:i52");
        assert_eq!(index_row.duration, None, "the row has no `tracks` fact");
        assert_eq!(page.title, "Singles by Haleen");
        assert_eq!(
            page.subtitle.as_deref(),
            Some("55 singles \u{b7} 3 in Library \u{b7} Artist feed")
        );
        assert_eq!(page.index_status, None, "the app has the result");
        assert_eq!(page.more_note, None);

        let mut vm = crate::view_models::library::LibraryViewModel::new();
        vm.replace_tree(tree.clone());
        let library_groups = singles_groups(&tree);
        let library_row = SinglesGroupRowVm::new(&library_groups[0], false, |_, _| None);
        assert_eq!(
            library_row.secondary_text, "3 singles \u{b7} In Library \u{b7} Artist feed",
            "the scope \"Library\" keeps its row (decision D15)"
        );
        assert_eq!(library_row.open, haleen_key());
        let generation = vm.index_feed_singles_mut().begin_request();
        vm.index_feed_singles_mut().record(
            HALEEN,
            FeedReleaseList::complete(haleen_index_rows()),
            generation,
        );
        let from_vm = vm.singles_group_page(&haleen_key());
        assert_eq!(row_keys(&from_vm), keys, "one page for each key");
    }

    /// R26-21: while the request for the feed runs, the Library rows show
    /// with the loading row. After a failure, they show with the error row
    /// and "Try again". A group of one Library single shows that row, and
    /// not the empty state, until the request has an answer.
    #[test]
    fn adr_0084_index_singles_r26_21_loading_and_error_rows_keep_the_library_rows() {
        let tree = haleen_feed_tree();
        let mut store = IndexFeedSingles::default();
        let unread =
            SinglesGroupPageVm::from_tree(&tree, &store, &haleen_key(), &BusyItems::default());
        assert!(matches!(
            unread.index_status,
            Some(IndexListStatusRowVm::Loading { .. })
        ));

        let generation = store.begin_feed_request(HALEEN).expect("one request");
        assert_eq!(store.begin_feed_request(HALEEN), None, "no second request");
        let page =
            SinglesGroupPageVm::from_tree(&tree, &store, &haleen_key(), &BusyItems::default());
        assert_eq!(
            page.index_status,
            Some(IndexListStatusRowVm::Loading {
                element_id: "singles-group-index-loading".to_owned(),
                label: "Loading MusicIndex singles",
            })
        );
        assert_eq!(row_keys(&page), ["guid-3", "guid-2", "guid-1"]);
        assert!(page.main_action.is_some());

        assert!(store.finish_feed_request(
            crate::view_models::index_feed_singles::FeedSinglesResult {
                feed_guid: HALEEN.to_owned(),
                rows: Err("connection refused".to_owned()),
            },
            generation,
        ));
        let page =
            SinglesGroupPageVm::from_tree(&tree, &store, &haleen_key(), &BusyItems::default());
        let Some(IndexListStatusRowVm::Failed {
            message,
            detail,
            retry,
            ..
        }) = &page.index_status
        else {
            panic!("the error row");
        };
        assert_eq!(
            message,
            "The MusicIndex singles did not load. The Library singles show."
        );
        assert_eq!(
            detail,
            &format!("GET /v1/feeds/recent?named_feed_guid={HALEEN}: connection refused")
        );
        assert_eq!(retry.label, "Try again");
        assert!(retry.available);
        assert_eq!(row_keys(&page), ["guid-3", "guid-2", "guid-1"]);
        assert!(
            store.needs_request(HALEEN),
            "\"Try again\" and a new open send the request again"
        );

        let one = LibraryTree {
            artists: vec![artist(
                "Haleen",
                vec![with_publisher(single("Only", 1, Some(5), true), HALEEN)],
            )],
        };
        let page = SinglesGroupPageVm::from_tree(
            &one,
            &IndexFeedSingles::default(),
            &haleen_key(),
            &BusyItems::default(),
        );
        assert_eq!(page.empty_message, None);
        assert_eq!(page.title, "Singles by Haleen");
        assert_eq!(row_keys(&page), ["guid-1"]);
        assert!(page.main_action.is_none());
        assert!(page.index_status.is_some());
    }

    /// R26-22: the main action of the D12 example is "Download 52
    /// singles". Its plan holds the Library feed id of each Library single
    /// that is not in the Library and the GUID and the title of each
    /// `MusicIndex` single, newest first.
    #[test]
    fn adr_0084_index_singles_r26_22_main_action_downloads_both_origins() {
        let store = fills(HALEEN, haleen_index_rows());
        let page = SinglesGroupPageVm::from_tree(
            &haleen_feed_tree(),
            &store,
            &haleen_key(),
            &BusyItems::default(),
        );
        let action = page.main_action.expect("a group has a main action");
        assert_eq!(action.label, "Download 52 singles");
        assert_eq!(
            action.a11y_label,
            "Download the 52 singles that are not in the Library"
        );
        assert!(action.available);
        let SinglesGroupAction::DownloadSingles(plan) = action.action else {
            panic!("the download action");
        };
        assert!(plan.library_feed_ids.is_empty());
        assert_eq!(plan.index_releases.len(), 52);
        assert_eq!(
            plan.index_releases[0],
            ("i52".to_owned(), Some("Index 52".to_owned()))
        );
        assert_eq!(plan.index_releases[51].0, "i1");

        let mut tree = haleen_feed_tree();
        tree.artists[0].albums[0].tracks[0].is_in_library = false;
        let page =
            SinglesGroupPageVm::from_tree(&tree, &store, &haleen_key(), &BusyItems::default());
        let action = page.main_action.expect("a group has a main action");
        assert_eq!(action.label, "Download 53 singles");
        let SinglesGroupAction::DownloadSingles(plan) = action.action else {
            panic!("the download action");
        };
        assert_eq!(plan.library_feed_ids, [1]);

        let queued = BusyItems::new(
            crate::view_models::download_progress::DownloadActivity {
                keys: BTreeSet::from([crate::view_models::download_progress::FeedKey::Guid(
                    "i7".to_owned(),
                )]),
                ..crate::view_models::download_progress::DownloadActivity::default()
            },
            BTreeSet::new(),
        );
        let page =
            SinglesGroupPageVm::from_tree(&haleen_feed_tree(), &store, &haleen_key(), &queued);
        assert!(
            !page.main_action.expect("the action").available,
            "a queued MusicIndex single makes the action wait"
        );
    }

    /// R26-23 and decision D14: the page of a single in a feed group of
    /// two or more members gives "All n singles", also for a `MusicIndex`
    /// single. The page plans one request when the app has no result, and
    /// none when a request runs or the result is loaded.
    #[test]
    fn adr_0084_index_singles_r26_23_single_page_links_to_its_feed_group() {
        let tree = LibraryTree {
            artists: vec![artist(
                "Haleen",
                vec![with_publisher(single("Only", 1, Some(5), true), HALEEN)],
            )],
        };
        let mut store = IndexFeedSingles::default();
        let library_single = &tree.artists[0].albums[0];
        assert_eq!(
            library_page_feed_to_read(library_single, &store).as_deref(),
            Some(HALEEN)
        );
        assert!(
            release_page_name_links(
                Vec::new(),
                &singles_groups_everything(&tree, &store),
                Some(&ReleaseKey::FeedGuid("guid-1".to_owned())),
            )
            .is_empty(),
            "one single makes no group before the result"
        );

        let generation = store.begin_feed_request(HALEEN).expect("one request");
        assert_eq!(library_page_feed_to_read(library_single, &store), None);
        assert!(store.finish_feed_request(
            crate::view_models::index_feed_singles::FeedSinglesResult {
                feed_guid: HALEEN.to_owned(),
                rows: Ok(FeedReleaseList::complete(haleen_index_rows())),
            },
            generation,
        ));
        assert_eq!(library_page_feed_to_read(library_single, &store), None);
        let groups = singles_groups_everything(&tree, &store);
        for key in ["guid-1", "i9"] {
            let links = release_page_name_links(
                Vec::new(),
                &groups,
                Some(&ReleaseKey::FeedGuid(key.to_owned())),
            );
            assert_eq!(links.len(), 1, "{key}");
            assert_eq!(links[0].label, "All 53 singles");
            assert_eq!(links[0].a11y_label, "Open all 53 singles by Haleen");
            assert_eq!(
                links[0].target,
                AlbumNameLinkTarget::SinglesGroup(haleen_key())
            );
        }

        let index_single = FeedView {
            release_kind: Some("unknown".to_owned()),
            episode_count: Some(1),
            named_feeds: vec![crate::view_models::named_feeds::NamedFeed {
                feed_guid: HALEEN_2.to_owned(),
                title: None,
                position: Some(1),
                role: None,
                role_agreement: None,
                feed_rel: None,
                album_rel: None,
                confirmed: true,
                image_url: None,
            }],
            ..FeedView::default()
        };
        assert_eq!(
            index_page_feed_to_read(&index_single, &store).as_deref(),
            Some(HALEEN_2)
        );
        let mut album = index_single.clone();
        album.episode_count = Some(9);
        assert_eq!(
            index_page_feed_to_read(&album, &store),
            None,
            "not a single"
        );
        let mut unconfirmed = index_single;
        unconfirmed.named_feeds[0].confirmed = false;
        assert_eq!(
            index_page_feed_to_read(&unconfirmed, &store),
            None,
            "a single at the name level (decision D7)"
        );
        let name_key = SinglesGroupKey(ArtistIdentity::Name("haleen".to_owned()));
        assert_eq!(group_page_feed_to_read(&name_key, &store), None);
        assert_eq!(
            group_page_feed_to_read(
                &SinglesGroupKey(ArtistIdentity::Publisher(HALEEN_2.to_owned())),
                &store
            )
            .as_deref(),
            Some(HALEEN_2)
        );
    }

    /// R26-24: an answer of an older generation changes nothing on the
    /// group page and on the single page. A page load recorded the feed
    /// first, so the late answer of the page request is dropped. A retry
    /// starts a new generation, so the late answer of the failed request is
    /// dropped too.
    #[test]
    fn adr_0084_index_singles_r26_24_old_answers_change_nothing() {
        use crate::view_models::index_feed_singles::FeedSinglesResult;
        let tree = haleen_feed_tree();
        let mut store = IndexFeedSingles::default();
        let page_request = store.begin_feed_request(HALEEN).expect("one request");
        let page_load = store.begin_request();
        assert!(store.record(
            HALEEN,
            FeedReleaseList::complete(haleen_index_rows()),
            page_load
        ));
        let before =
            SinglesGroupPageVm::from_tree(&tree, &store, &haleen_key(), &BusyItems::default());
        assert!(!store.finish_feed_request(
            FeedSinglesResult {
                feed_guid: HALEEN.to_owned(),
                rows: Ok(FeedReleaseList::complete(Vec::new())),
            },
            page_request,
        ));
        let after =
            SinglesGroupPageVm::from_tree(&tree, &store, &haleen_key(), &BusyItems::default());
        assert_eq!(row_keys(&after), row_keys(&before));
        assert_eq!(after.rows.len(), 55);

        let mut store = IndexFeedSingles::default();
        let first = store.begin_feed_request(HALEEN).expect("one request");
        assert!(store.finish_feed_request(
            FeedSinglesResult {
                feed_guid: HALEEN.to_owned(),
                rows: Err("timeout".to_owned()),
            },
            first,
        ));
        let retry = store.begin_feed_request(HALEEN).expect("the retry");
        assert!(!store.finish_feed_request(
            FeedSinglesResult {
                feed_guid: HALEEN.to_owned(),
                rows: Ok(FeedReleaseList::complete(haleen_index_rows())),
            },
            first,
        ));
        assert_eq!(store.state(HALEEN), FeedSinglesState::Running);
        let one = LibraryTree {
            artists: vec![artist(
                "Haleen",
                vec![with_publisher(single("Only", 1, Some(5), true), HALEEN)],
            )],
        };
        assert!(release_page_name_links(
            Vec::new(),
            &singles_groups_everything(&one, &store),
            Some(&ReleaseKey::FeedGuid("guid-1".to_owned())),
        )
        .is_empty());
        assert!(store.finish_feed_request(
            FeedSinglesResult {
                feed_guid: HALEEN.to_owned(),
                rows: Ok(FeedReleaseList::complete(haleen_index_rows())),
            },
            retry,
        ));
        assert_eq!(
            store.state(HALEEN),
            FeedSinglesState::Loaded { more: false }
        );
    }

    /// R26-25 and decision D12: the empty state text names no scope.
    #[test]
    fn adr_0084_index_singles_r26_25_empty_state_text() {
        let name_key = SinglesGroupKey(ArtistIdentity::Name("haleen".to_owned()));
        let page = SinglesGroupPageVm::from_tree(
            &haleen_tree(1, 1),
            &IndexFeedSingles::default(),
            &name_key,
            &BusyItems::default(),
        );
        assert_eq!(
            page.empty_message,
            Some("This artist has fewer than two singles.")
        );
        assert_eq!(page.index_status, None);

        let store = fills(HALEEN, Vec::new());
        let one = LibraryTree {
            artists: vec![artist(
                "Haleen",
                vec![with_publisher(single("Only", 1, Some(5), true), HALEEN)],
            )],
        };
        let page =
            SinglesGroupPageVm::from_tree(&one, &store, &haleen_key(), &BusyItems::default());
        assert_eq!(
            page.empty_message,
            Some("This artist has fewer than two singles."),
            "the loaded feed has no other single"
        );
    }

    /// Decision D11: when more releases name the feed than the app read,
    /// the group page gives the note.
    #[test]
    fn adr_0084_index_singles_d11_group_page_says_more_exist() {
        let mut store = IndexFeedSingles::default();
        let generation = store.begin_request();
        store.record(
            HALEEN,
            FeedReleaseList {
                rows: haleen_index_rows(),
                more: true,
            },
            generation,
        );
        assert_eq!(store.state(HALEEN), FeedSinglesState::Loaded { more: true });
        let page = SinglesGroupPageVm::from_tree(
            &haleen_feed_tree(),
            &store,
            &haleen_key(),
            &BusyItems::default(),
        );
        assert_eq!(page.more_note, Some(MORE_INDEX_SINGLES_NOTE));
        assert_eq!(page.index_status, None);
    }

    /// Packet 026 review: `MORE_INDEX_SINGLES_NOTE` must name the release
    /// count that `FEED_SINGLES_MAX_PAGES` and `FEED_SINGLES_PAGE_LIMIT`
    /// give together. A later change of either constant must update this
    /// text in the same change, or this test fails.
    #[test]
    fn adr_0084_more_index_singles_note_names_the_feed_singles_limit() {
        use crate::application::queries::name_lists::{
            FEED_SINGLES_MAX_PAGES, FEED_SINGLES_PAGE_LIMIT,
        };
        let page_limit =
            usize::try_from(FEED_SINGLES_PAGE_LIMIT).expect("a positive feed singles page limit");
        let total = FEED_SINGLES_MAX_PAGES * page_limit;
        let expected = format!("its {} newest releases.", format_with_commas(total));
        assert!(
            MORE_INDEX_SINGLES_NOTE.ends_with(&expected),
            "MORE_INDEX_SINGLES_NOTE must name {total} newest releases, the product of \
             FEED_SINGLES_MAX_PAGES and FEED_SINGLES_PAGE_LIMIT; got {MORE_INDEX_SINGLES_NOTE:?}"
        );
    }

    /// Formats `n` with a comma every three digits from the right, the
    /// same grouping that `MORE_INDEX_SINGLES_NOTE` uses for its number.
    fn format_with_commas(n: usize) -> String {
        let digits = n.to_string();
        let mut grouped = String::new();
        for (index, digit) in digits.chars().rev().enumerate() {
            if index > 0 && index % 3 == 0 {
                grouped.push(',');
            }
            grouped.push(digit);
        }
        grouped.chars().rev().collect()
    }
}
