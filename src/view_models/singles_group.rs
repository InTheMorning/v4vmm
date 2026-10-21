//! The singles groups of the Library and of the scope "Everything" (ADR
//! 0084 Decision 6).
//!
//! Two or more singles of one artist identity (ADR 0084 Decision 7) make
//! one group. This module owns the functions that group singles:
//! [`singles_groups`] for the Library, and [`singles_groups_everything`] for
//! the Releases scope "Everything", which adds the `MusicIndex` singles of
//! each artist feed (packet 026). The Releases list, the group page and the
//! link on a single page each use them. No other code groups releases by
//! kind.

#![warn(clippy::pedantic)]

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use crate::api::Feed;
use crate::db::TrackRow;
use crate::view_models::album_page::{AlbumNameLinkTarget, AlbumNameLinkVm};
use crate::view_models::artist_identity::{ArtistIdentity, UNKNOWN_ARTIST_TEXT};
use crate::view_models::collection_art::{CollectionArtVm, CollectionKind};
use crate::view_models::format::{fmt_date, plural};
use crate::view_models::index_feed_singles::IndexFeedSingles;
use crate::view_models::index_name_list::name_sort_key;
use crate::view_models::library::{
    AlbumNode, ArtistNode, BusyItems, DestinationListRowVm, LibraryTree, TrackRowWork,
};
use crate::view_models::list_selection::RowSelectionVm;
use crate::view_models::named_feeds::index_named_feeds;
use crate::view_models::release_kind::ReleaseKind;
use crate::view_models::releases_everything::index_release_row;
use crate::view_models::workspace::ReleaseKey;

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
/// The empty state of a group page with fewer than two singles.
const EMPTY_GROUP_MESSAGE: &str = "This artist has fewer than two singles in the Library.";
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

    /// The number of singles in the group: the Library singles and the
    /// `MusicIndex` singles.
    #[must_use]
    pub(crate) fn count(&self) -> usize {
        self.singles.len() + self.index_singles.len()
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
        self.members()
            .iter()
            .map(GroupMember::sort_key)
            .min()
            .unwrap_or_default()
    }

    /// The number of singles in the Library.
    #[must_use]
    pub(crate) fn in_library_count(&self) -> usize {
        self.singles
            .iter()
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
        (!self.singles.is_empty()).then(|| self.library_state_text())
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
        let members = self.members();
        CollectionArtVm::resolve(
            self.feed_image,
            members.iter().filter_map(GroupMember::image),
            CollectionKind::SinglesGroup,
        )
    }

    /// Tells if the group holds the release `key`.
    #[must_use]
    pub(crate) fn contains(&self, key: &ReleaseKey) -> bool {
        self.singles.iter().any(|single| single.key == *key)
            || self.index_singles.iter().any(|single| single.key == *key)
    }
}

/// Groups the singles of the Library tree by artist identity (ADR 0084
/// Decisions 6 and 7).
///
/// A single is an album whose kind is [`ReleaseKind::Single`] and that has
/// a release key. A group holds two or more singles. One single makes no
/// group. Each group gives its singles newest first: the date of a single
/// is the latest `pub_date` of its tracks, a single with no date comes
/// last, and equal dates sort by the lowercase release name. The groups
/// come in the order of their identity.
#[must_use]
pub(crate) fn singles_groups(tree: &LibraryTree) -> Vec<SinglesGroup<'_>> {
    let feed_images = tree.feed_images();
    library_singles_by_identity(tree)
        .into_iter()
        .filter(|(_, singles)| singles.len() >= 2)
        .map(|(identity, mut singles)| {
            singles.sort_by(newest_first);
            let display_artist = singles[0].artist.name.as_str();
            let feed_image = match &identity {
                ArtistIdentity::Publisher(feed_guid) => feed_images.get(feed_guid),
                ArtistIdentity::Name(_) => None,
            };
            SinglesGroup {
                identity,
                display_artist,
                singles,
                index_singles: Vec::new(),
                feed_image,
            }
        })
        .collect()
}

/// The Library singles of the tree by artist identity, one or more for
/// each identity, in no order.
fn library_singles_by_identity(
    tree: &LibraryTree,
) -> BTreeMap<ArtistIdentity, Vec<GroupSingle<'_>>> {
    let mut by_identity: BTreeMap<ArtistIdentity, Vec<GroupSingle<'_>>> = BTreeMap::new();
    for artist in &tree.artists {
        for album in &artist.albums {
            if album.release_kind() != ReleaseKind::Single {
                continue;
            }
            let Some(key) = album.release_key() else {
                continue;
            };
            by_identity
                .entry(album.artist_identity(&artist.name))
                .or_default()
                .push(GroupSingle {
                    artist,
                    album,
                    key,
                    date: album.tracks.iter().filter_map(|track| track.pub_date).max(),
                    in_library: !album.tracks.is_empty()
                        && album.tracks.iter().all(|track| track.is_in_library),
                });
        }
    }
    by_identity
}

/// The GUID of each artist feed with one or more Library singles (ADR 0084
/// packet 026). Page 1 of the scope "Everything" reads the `MusicIndex`
/// singles of each of these feeds.
#[must_use]
pub(crate) fn library_single_artist_feeds(tree: &LibraryTree) -> Vec<String> {
    library_singles_by_identity(tree)
        .into_keys()
        .filter_map(|identity| match identity {
            ArtistIdentity::Publisher(feed_guid) => Some(feed_guid),
            ArtistIdentity::Name(_) => None,
        })
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
    let library_keys: BTreeSet<ReleaseKey> = tree
        .artists
        .iter()
        .flat_map(|artist| artist.albums.iter())
        .filter_map(AlbumNode::release_key)
        .collect();
    let mut library = library_singles_by_identity(tree);
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
    let identities: BTreeSet<ArtistIdentity> =
        library.keys().chain(index.keys()).cloned().collect();
    let feed_images = tree.feed_images();
    identities
        .into_iter()
        .filter_map(|identity| {
            let mut singles = library.remove(&identity).unwrap_or_default();
            let mut index_singles = index.remove(&identity).unwrap_or_default();
            if singles.len() + index_singles.len() < 2 {
                return None;
            }
            singles.sort_by(newest_first);
            index_singles.sort_by(|left, right| {
                newest_date_first(left.date, right.date)
                    .then_with(|| left.sort_key.cmp(&right.sort_key))
            });
            let feed_guid = match &identity {
                ArtistIdentity::Publisher(feed_guid) => Some(feed_guid.as_str()),
                ArtistIdentity::Name(_) => None,
            };
            let display_artist = match singles.first() {
                Some(newest) => newest.artist.name.as_str(),
                None => index_group_label(&index_singles[0], feed_guid),
            };
            let feed_image = feed_guid.and_then(|guid| feed_images.get(guid));
            Some(SinglesGroup {
                identity,
                display_artist,
                singles,
                index_singles,
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
    /// The release row of each single, newest first, when `expanded` is
    /// true. Empty when it is false.
    pub(crate) singles: Vec<DestinationListRowVm>,
    /// The release key of each single of the group, in the same order,
    /// whatever `expanded` is (ADR 0084 Decision 6). The group row's
    /// checkbox toggles every one of these.
    pub(crate) single_keys: Vec<ReleaseKey>,
    /// The checkbox facts of the group row (ADR 0084 Decision 10). The
    /// group's own checkbox, not one for each single.
    pub(crate) selection: RowSelectionVm,
}

impl SinglesGroupRowVm {
    /// Builds the group row. `release_row` gives the release row of one
    /// Library single, the same row that a release that is not in a group
    /// gets. A `MusicIndex` single gets the `MusicIndex` release row
    /// ([`index_release_row`], packet 026).
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
                members
                    .iter()
                    .filter_map(|member| match member {
                        GroupMember::Library(single) => release_row(single.artist, single.album),
                        GroupMember::Index(single) => index_release_row(single.feed),
                    })
                    .collect()
            } else {
                Vec::new()
            },
            single_keys: members.iter().map(|member| member.key().clone()).collect(),
            selection: RowSelectionVm::none(),
        }
    }
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
/// for example "All 7 singles". `None` when the release `key` is in no
/// Library group.
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
/// `base`, then the link to the Library group of the release, when the
/// release `key` is in one.
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

/// The command of the main action of a group page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SinglesGroupAction {
    /// Download each single that is not in the Library, newest first, by
    /// Library feed id.
    DownloadSingles(Vec<i64>),
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
    /// One row for each single, newest first.
    pub(crate) rows: Vec<SinglesGroupPageRowVm>,
    /// The one main action. `None` for the empty state.
    pub(crate) main_action: Option<SinglesGroupActionVm>,
    /// The empty state text, when no group has the key.
    pub(crate) empty_message: Option<&'static str>,
    /// The header art in the logo frame (ADR 0084 Decision 12). `None`
    /// for the empty state.
    pub(crate) art: Option<CollectionArtVm>,
}

impl SinglesGroupPageVm {
    /// Builds the page of the group `key` from the current tree. When no
    /// group has the key, for example after a removal leaves one single,
    /// the page is the empty state. `busy` holds the downloads and the
    /// removals in progress (ADR 0084 Decision 10). While one single of
    /// the group is busy, the main action is not available.
    #[must_use]
    pub(crate) fn from_tree(tree: &LibraryTree, key: &SinglesGroupKey, busy: &BusyItems) -> Self {
        let groups = singles_groups(tree);
        match find_group(&groups, key) {
            Some(group) => Self::new(group, busy),
            None => Self {
                title: EMPTY_GROUP_TITLE.to_owned(),
                subtitle: None,
                identity_note: None,
                rows: Vec::new(),
                main_action: None,
                empty_message: Some(EMPTY_GROUP_MESSAGE),
                art: None,
            },
        }
    }

    /// Builds the page of one group. A single with no track has no row.
    #[must_use]
    pub(crate) fn new(group: &SinglesGroup<'_>, busy: &BusyItems) -> Self {
        let group_busy = group
            .singles
            .iter()
            .any(|single| busy.release_downloading(single.album) || busy.removing(single.album));
        let rows = group
            .singles
            .iter()
            .filter_map(|single| {
                let track = single.album.tracks.first()?;
                Some(SinglesGroupPageRowVm {
                    track: track.clone(),
                    date_text: single.date.and_then(fmt_date),
                    work: busy.track_row_work(single.album, track),
                    key: single.key.clone(),
                })
            })
            .collect();
        Self {
            title: singles_group_title(group.display_artist),
            subtitle: Some(group.summary_text()),
            identity_note: match group.identity {
                ArtistIdentity::Name(_) => Some(NAME_LEVEL_NOTE),
                ArtistIdentity::Publisher(_) => Some(PUBLISHER_LEVEL_NOTE),
            },
            rows,
            main_action: Some(main_action(group, group_busy)),
            empty_message: None,
            art: Some(group.collection_art()),
        }
    }
}

/// The title of a group page and its breadcrumb label, for example
/// "Singles by Haleen".
#[must_use]
pub(crate) fn singles_group_title(display_artist: &str) -> String {
    format!("Singles by {display_artist}")
}

/// The main action of a group page: "Download n singles" when one or more
/// singles are not in the Library, else "Add to playlist".
fn main_action(group: &SinglesGroup<'_>, busy: bool) -> SinglesGroupActionVm {
    let missing: Vec<i64> = group
        .singles
        .iter()
        .filter(|single| !single.in_library)
        .filter_map(|single| single.album.feed_id)
        .collect();
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
        let all: Vec<i64> = group
            .singles
            .iter()
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

    /// R15-03: a publisher identity and a name identity with the same
    /// text, and an album or an EP make no group. R17-02 (packet 017
    /// decision D3): names that differ only in case make one group.
    #[test]
    fn adr_0084_singles_group_r15_03_different_identities_and_kinds_do_not_group() {
        let case_tree = LibraryTree {
            artists: vec![
                artist("Haleen", vec![single("A", 1, Some(1), true)]),
                artist("haleen", vec![single("B", 2, Some(2), true)]),
            ],
        };
        assert_eq!(singles_groups(&case_tree).len(), 1);

        let level_tree = LibraryTree {
            artists: vec![artist(
                "Haleen",
                vec![
                    with_publisher(single("A", 1, Some(1), true), "Haleen"),
                    single("B", 2, Some(2), true),
                ],
            )],
        };
        assert!(singles_groups(&level_tree).is_empty());

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

        let page = SinglesGroupPageVm::from_tree(&haleen_tree(3, 2), &key, &BusyItems::default());
        let action = page.main_action.expect("a group has a main action");
        assert_eq!(action.label, "Download 1 single");
        assert_eq!(
            action.a11y_label,
            "Download the 1 single that is not in the Library"
        );
        assert!(action.available);
        assert_eq!(action.action, SinglesGroupAction::DownloadSingles(vec![3]));

        let page = SinglesGroupPageVm::from_tree(&haleen_tree(3, 1), &key, &BusyItems::default());
        let action = page.main_action.expect("a group has a main action");
        assert_eq!(action.label, "Download 2 singles");
        assert_eq!(
            action.a11y_label,
            "Download the 2 singles that are not in the Library"
        );
        assert!(action.available);
        assert_eq!(
            action.action,
            SinglesGroupAction::DownloadSingles(vec![3, 2]),
            "the downloads run newest first"
        );

        let page = SinglesGroupPageVm::from_tree(&haleen_tree(7, 7), &key, &BusyItems::default());
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
        assert_eq!(page.rows[0].key, ReleaseKey::FeedGuid("guid-7".to_owned()));
        assert!(page.rows[0].date_text.is_some());
    }

    /// R15-07: each level gives its identity note and names its level in
    /// the subtitle. A key with no group gives the empty state.
    #[test]
    fn adr_0084_singles_group_r15_07_page_identity_note_and_empty_state() {
        let name_key = SinglesGroupKey(ArtistIdentity::Name("haleen".to_owned()));
        let page =
            SinglesGroupPageVm::from_tree(&haleen_tree(2, 1), &name_key, &BusyItems::default());
        assert_eq!(page.identity_note, Some(NAME_LEVEL_NOTE));
        assert!(!page.rows[0].track.is_in_library);
        assert!(page.rows[1].track.is_in_library);

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
        let page =
            SinglesGroupPageVm::from_tree(&publisher_tree, &publisher_key, &BusyItems::default());
        assert_eq!(page.identity_note, Some(PUBLISHER_LEVEL_NOTE));
        assert_eq!(
            page.subtitle.as_deref(),
            Some("2 singles · In Library · Artist feed")
        );
        assert_eq!(page.rows.len(), 2);

        let page =
            SinglesGroupPageVm::from_tree(&haleen_tree(1, 1), &name_key, &BusyItems::default());
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

        let page = SinglesGroupPageVm::from_tree(&tree, &key, &BusyItems::default());
        assert_eq!(page.art, Some(newest_four.clone()));

        let groups = singles_groups(&tree);
        let row = SinglesGroupRowVm::new(&groups[0], false, |_, _| None);
        assert_eq!(row.art, newest_four);

        let empty = SinglesGroupPageVm::from_tree(&haleen_tree(1, 1), &key, &BusyItems::default());
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
        let page = SinglesGroupPageVm::from_tree(&tree, &key, &BusyItems::default());
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
            let page = SinglesGroupPageVm::from_tree(&tree, &key, &BusyItems::default());
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
        let page = SinglesGroupPageVm::from_tree(&tree, &key, &BusyItems::default());
        assert_eq!(page.rows.len(), 2);

        let dated = &page.rows[0];
        assert_eq!(dated.track.id, 10, "the row holds the track of its single");
        assert_eq!(dated.track.feed_id, 1);
        assert_eq!(dated.date_text.as_deref(), Some("Apr 5, 2024"));
        assert_eq!(dated.key, ReleaseKey::FeedGuid("guid-1".to_owned()));
        assert_eq!(dated.work, TrackRowWork::Idle);

        let undated = &page.rows[1];
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
        let page = SinglesGroupPageVm::from_tree(&tree, &key, &busy_track);
        assert_eq!(page.rows[0].work, TrackRowWork::Idle);
        assert_eq!(
            page.rows[1].work,
            TrackRowWork::Queued,
            "the queued track makes its row busy"
        );
        let busy_feed = BusyItems::new(DownloadActivity::default(), BTreeSet::from([1]));
        let page = SinglesGroupPageVm::from_tree(&tree, &key, &busy_feed);
        assert_eq!(
            page.rows[0].work,
            TrackRowWork::Removing,
            "the busy feed makes its row busy"
        );
        assert_eq!(page.rows[1].work, TrackRowWork::Idle);
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
    use crate::view_models::index_feed_singles::IndexFeedSingles;

    /// The Library artist feed of Haleen (packet 008 record).
    const HALEEN: &str = "64569161-a8f7-52e5-8aae-9e0385043149";
    /// The second Haleen feed with the same title (packet 028 D4).
    const HALEEN_2: &str = "fe80a515-2826-4a46-aa37-b27f38d3c30f";
    /// Sir Libre Records.
    const SIR_LIBRE: &str = "34bf795f-f551-4db1-8877-ad19aa5f89c8";

    fn fills(feed_guid: &str, rows: Vec<Feed>) -> IndexFeedSingles {
        let mut store = IndexFeedSingles::default();
        let generation = store.begin_request();
        store.record(feed_guid, rows, generation);
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
}
