//! The singles groups of the Library (ADR 0084 Decision 6).
//!
//! Two or more Library singles of one artist identity (ADR 0084 Decision 7)
//! make one group. This module owns the one function that groups singles:
//! [`singles_groups`]. The Releases list, the group page and the link on a
//! single page each use it. No other code groups releases by kind.

#![warn(clippy::pedantic)]

use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::view_models::album_page::{AlbumNameLinkTarget, AlbumNameLinkVm};
use crate::view_models::artist_identity::ArtistIdentity;
use crate::view_models::format::{fmt_date, plural};
use crate::view_models::library::{AlbumNode, ArtistNode, DestinationListRowVm, LibraryTree};
use crate::view_models::release_kind::ReleaseKind;
use crate::view_models::workspace::ReleaseKey;

/// The separator between the count and the Library state of a group.
const SUMMARY_SEPARATOR: &str = " · ";
/// The Library state of a group when each single is in the Library.
const ALL_IN_LIBRARY_LABEL: &str = "In Library";
/// The state word of a single in the Library.
const IN_LIBRARY_STATE_WORD: &str = "In Library";
/// The state word of a single that is not in the Library.
const NOT_DOWNLOADED_STATE_WORD: &str = "Not downloaded";
/// The accessibility label of a closed group row chevron.
const SHOW_SINGLES_LABEL: &str = "Show singles";
/// The accessibility label of an open group row chevron.
const HIDE_SINGLES_LABEL: &str = "Hide singles";
/// The note of a group at the name level (ADR 0084 Decision 7).
const NAME_LEVEL_NOTE: &str = "Grouped by the artist name. A name is not an identity.";
/// The note of a group at the publisher level (ADR 0084 Decision 7).
const PUBLISHER_LEVEL_NOTE: &str = "Grouped by the publisher feed that each single names.";
/// The identity level of a group at the name level, in its summary text.
const NAME_LEVEL_LABEL: &str = "By name";
/// The identity level of a group at the publisher level, in its summary
/// text.
const PUBLISHER_LEVEL_LABEL: &str = "Publisher feed";
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

/// Two or more Library singles of one artist identity (ADR 0084 Decision
/// 6), newest first.
#[derive(Clone, Debug)]
pub(crate) struct SinglesGroup<'a> {
    /// The artist identity of each single in the group.
    pub(crate) identity: ArtistIdentity,
    /// The artist node name of the newest single.
    pub(crate) display_artist: &'a str,
    /// The singles, newest first.
    pub(crate) singles: Vec<GroupSingle<'a>>,
}

impl SinglesGroup<'_> {
    /// The key of the group page.
    #[must_use]
    pub(crate) fn key(&self) -> SinglesGroupKey {
        SinglesGroupKey(self.identity.clone())
    }

    /// The number of singles in the group.
    #[must_use]
    pub(crate) fn count(&self) -> usize {
        self.singles.len()
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
    /// Publisher feed". One name can have a group at each level (ADR 0084
    /// Decision 7), so the level tells the two rows apart.
    #[must_use]
    pub(crate) fn summary_text(&self) -> String {
        format!(
            "{} singles{SUMMARY_SEPARATOR}{}{SUMMARY_SEPARATOR}{}",
            self.count(),
            self.library_state_text(),
            self.level_label()
        )
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

    /// The identity level of the group: "By name" or "Publisher feed".
    #[must_use]
    pub(crate) const fn level_label(&self) -> &'static str {
        match self.identity {
            ArtistIdentity::Name(_) => NAME_LEVEL_LABEL,
            ArtistIdentity::Publisher(_) => PUBLISHER_LEVEL_LABEL,
        }
    }

    /// The image of the newest single.
    #[must_use]
    pub(crate) fn newest_image_href(&self) -> Option<String> {
        self.singles
            .first()
            .and_then(|single| single.album.image_href.clone())
    }

    /// Tells if the group holds the release `key`.
    #[must_use]
    pub(crate) fn contains(&self, key: &ReleaseKey) -> bool {
        self.singles.iter().any(|single| single.key == *key)
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
        .into_iter()
        .filter(|(_, singles)| singles.len() >= 2)
        .map(|(identity, mut singles)| {
            singles.sort_by(newest_first);
            let display_artist = singles[0].artist.name.as_str();
            SinglesGroup {
                identity,
                display_artist,
                singles,
            }
        })
        .collect()
}

/// The order of the singles in a group: newest first, a single with no
/// date last, and equal dates by the lowercase release name.
fn newest_first(left: &GroupSingle<'_>, right: &GroupSingle<'_>) -> Ordering {
    let by_date = match (left.date, right.date) {
        (Some(left), Some(right)) => right.cmp(&left),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    };
    by_date.then_with(|| {
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
    /// The image of the newest single.
    pub(crate) thumbnail_href: Option<String>,
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
}

impl SinglesGroupRowVm {
    /// Builds the group row. `release_row` gives the release row of one
    /// single, the same row that a release that is not in a group gets.
    #[must_use]
    pub(crate) fn new(
        group: &SinglesGroup<'_>,
        expanded: bool,
        release_row: impl Fn(&ArtistNode, &AlbumNode) -> Option<DestinationListRowVm>,
    ) -> Self {
        let element_id = group_element_id(&group.identity);
        let count = group.count();
        Self {
            disclosure_element_id: format!("{element_id}-disclosure"),
            element_id,
            label: group.display_artist.to_owned(),
            secondary_text: group.summary_text(),
            a11y_label: format!(
                "{}, {count} singles, {}, {}",
                group.display_artist,
                group.library_state_text(),
                group.level_label()
            ),
            thumbnail_href: group.newest_image_href(),
            open: group.key(),
            expanded,
            disclosure_a11y_label: if expanded {
                HIDE_SINGLES_LABEL
            } else {
                SHOW_SINGLES_LABEL
            },
            singles: if expanded {
                group
                    .singles
                    .iter()
                    .filter_map(|single| release_row(single.artist, single.album))
                    .collect()
            } else {
                Vec::new()
            },
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

/// One single row of a group page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SinglesGroupPageRowVm {
    /// Stable element id.
    pub(crate) element_id: String,
    /// The release name.
    pub(crate) name: String,
    /// The date text, when the single has a date.
    pub(crate) date_text: Option<String>,
    /// "In Library" or "Not downloaded".
    pub(crate) state_word: &'static str,
    /// The accessibility label of the row.
    pub(crate) a11y_label: String,
    /// The release key that a click opens.
    pub(crate) key: ReleaseKey,
}

/// The page of one singles group (ADR 0084 Decision 6).
#[derive(Clone, Debug, Eq, PartialEq)]
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
}

impl SinglesGroupPageVm {
    /// Builds the page of the group `key` from the current tree. When no
    /// group has the key, for example after a removal leaves one single,
    /// the page is the empty state. `busy` is a download or a removal in
    /// progress: the main action is then not available.
    #[must_use]
    pub(crate) fn from_tree(tree: &LibraryTree, key: &SinglesGroupKey, busy: bool) -> Self {
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
            },
        }
    }

    /// Builds the page of one group.
    #[must_use]
    pub(crate) fn new(group: &SinglesGroup<'_>, busy: bool) -> Self {
        let rows = group
            .singles
            .iter()
            .enumerate()
            .map(|(index, single)| {
                let state_word = if single.in_library {
                    IN_LIBRARY_STATE_WORD
                } else {
                    NOT_DOWNLOADED_STATE_WORD
                };
                let date_text = single.date.and_then(fmt_date);
                let a11y_label = match &date_text {
                    Some(date) => format!("Single: {}, {date}, {state_word}", single.album.name),
                    None => format!("Single: {}, {state_word}", single.album.name),
                };
                SinglesGroupPageRowVm {
                    element_id: format!("singles-group-page-row-{index}"),
                    name: single.album.name.clone(),
                    date_text,
                    state_word,
                    a11y_label,
                    key: single.key.clone(),
                }
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
            main_action: Some(main_action(group, busy)),
            empty_message: None,
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
    use super::*;
    use crate::db::TrackRow;
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
            publisher_feed_guid: None,
            publisher_link_role: None,
            publisher_role_agreement: None,
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

    fn with_publisher(mut album: AlbumNode, guid: &str) -> AlbumNode {
        album.publisher_feed_guid = Some(guid.to_owned());
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
            ArtistIdentity::Name("Haleen".to_owned())
        );
        assert_eq!(groups[0].display_artist, "Haleen");
        assert_eq!(groups[0].count(), 2);

        let tree = haleen_tree(1, 1);
        assert!(singles_groups(&tree).is_empty());
    }

    /// R15-03: names that differ only in case, a publisher identity and a
    /// name identity with the same text, and an album or an EP make no
    /// group.
    #[test]
    fn adr_0084_singles_group_r15_03_different_identities_and_kinds_do_not_group() {
        let case_tree = LibraryTree {
            artists: vec![
                artist("Haleen", vec![single("A", 1, Some(1), true)]),
                artist("haleen", vec![single("B", 2, Some(2), true)]),
            ],
        };
        assert!(singles_groups(&case_tree).is_empty());

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
        let key = SinglesGroupKey(ArtistIdentity::Name("Haleen".to_owned()));

        let page = SinglesGroupPageVm::from_tree(&haleen_tree(3, 2), &key, false);
        let action = page.main_action.expect("a group has a main action");
        assert_eq!(action.label, "Download 1 single");
        assert_eq!(
            action.a11y_label,
            "Download the 1 single that is not in the Library"
        );
        assert!(action.available);
        assert_eq!(action.action, SinglesGroupAction::DownloadSingles(vec![3]));

        let page = SinglesGroupPageVm::from_tree(&haleen_tree(3, 1), &key, false);
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

        let page = SinglesGroupPageVm::from_tree(&haleen_tree(7, 7), &key, false);
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
        assert_eq!(page.rows[0].name, "Single 7");
        assert_eq!(page.rows[0].state_word, "In Library");
        assert!(page.rows[0].date_text.is_some());
    }

    /// R15-07: each level gives its identity note and names its level in
    /// the subtitle. A key with no group gives the empty state.
    #[test]
    fn adr_0084_singles_group_r15_07_page_identity_note_and_empty_state() {
        let name_key = SinglesGroupKey(ArtistIdentity::Name("Haleen".to_owned()));
        let page = SinglesGroupPageVm::from_tree(&haleen_tree(2, 1), &name_key, false);
        assert_eq!(page.identity_note, Some(NAME_LEVEL_NOTE));
        assert_eq!(page.rows[0].state_word, "Not downloaded");
        assert_eq!(page.rows[1].state_word, "In Library");

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
        let page = SinglesGroupPageVm::from_tree(&publisher_tree, &publisher_key, false);
        assert_eq!(page.identity_note, Some(PUBLISHER_LEVEL_NOTE));
        assert_eq!(
            page.subtitle.as_deref(),
            Some("2 singles · In Library · Publisher feed")
        );
        assert_eq!(page.rows.len(), 2);

        let page = SinglesGroupPageVm::from_tree(&haleen_tree(1, 1), &name_key, false);
        assert_eq!(page.empty_message, Some(EMPTY_GROUP_MESSAGE));
        assert!(page.main_action.is_none());
        assert!(page.rows.is_empty());
    }

    /// R15-08: the release page name links of a single in a group of 7
    /// include "All 7 singles" with the group key. A single in no group and
    /// an album give no such link.
    #[test]
    fn adr_0084_singles_group_r15_08_single_page_links_to_its_group() {
        let base = vec![AlbumNameLinkVm {
            label: "Haleen".to_owned(),
            a11y_label: "Open the artist Haleen".to_owned(),
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
                "Haleen".to_owned()
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
}
