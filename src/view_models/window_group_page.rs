//! The window group page of New releases (ADR 0086 Decision 4).
//!
//! A click on a group tile opens this page. The page lists the singles of
//! one window, newest first, with one main action and a link to the group
//! page of the artist (ADR 0084 Decision 6). The page is a new page kind
//! (packet 004 decision D1). It uses the row types, the main action and
//! the link of the ADR 0084 Decision 6 page (decision D2).
//!
//! `LibraryViewModel::window_group_page` builds the page at each render
//! from the cached layout, the tree, the loaded `MusicIndex` singles and
//! the busy items (decision D13). No other code resolves the members of a
//! window.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;

use crate::view_models::album_page::AlbumNameLinkVm;
use crate::view_models::collection_art::{CollectionArtVm, CollectionKind};
use crate::view_models::format::plural;
use crate::view_models::index_feed_singles::IndexFeedSingles;
use crate::view_models::library::{
    AlbumNode, ArtistNode, BusyItems, ContentListRowDisplay, ContentListRowKind, LibraryTree,
};
use crate::view_models::publisher_page::PublisherPageVm;
use crate::view_models::releases_everything::ReleasesDownloadPlan;
use crate::view_models::search_results::FeedResultDisplay;
use crate::view_models::singles_group::{
    find_group, library_single_row, singles_group_link, singles_groups_everything,
    singles_main_action, GroupSingle, MatchedByNameHeadingVm, SinglesGroupActionVm,
    SinglesGroupIndexRowVm, SinglesGroupPageEntryVm,
};
use crate::view_models::window_groups::{
    window_span_text, NewReleasesLayout, NewReleasesLayoutItem, WindowGroupKey,
};
use crate::view_models::workspace::ReleaseKey;

/// The empty state of a window group page whose window has no loaded row
/// (packet 004 decision D9).
pub(crate) const WINDOW_GROUP_EMPTY_MESSAGE: &str =
    "The loaded New releases hold no single of this window.";

/// The title of a window group page and its breadcrumb label, for example
/// "jSmovie · 9 singles" (packet 004 decision D14). No member gives the
/// artist name alone.
#[must_use]
pub(crate) fn window_group_title(artist: &str, count: usize) -> String {
    if count == 0 {
        return artist.to_owned();
    }
    format!("{artist} · {count} single{}", plural(count))
}

/// The page of one window group (ADR 0086 Decision 4).
#[derive(Clone, Debug)]
pub(crate) struct WindowGroupPageVm {
    /// "jSmovie · 9 singles". The breadcrumb label and the detail frame
    /// title are the same text.
    pub(crate) title: String,
    /// The date span of the members, for example "Oct 8 to Oct 20". `None`
    /// with no member.
    pub(crate) date_span: Option<String>,
    /// One row for each member: the other members newest first, then the
    /// members that a name joins to the feed.
    pub(crate) rows: Vec<SinglesGroupPageEntryVm>,
    /// The heading of the trailing "Matched by name" rows. `None` when no
    /// member has such a row.
    pub(crate) matched_by_name: Option<MatchedByNameHeadingVm>,
    /// The one main action. `None` with no member.
    pub(crate) main_action: Option<SinglesGroupActionVm>,
    /// The link to the group page of the artist, for example "All 23
    /// singles" (packet 004 decision D8).
    pub(crate) all_singles_link: Option<AlbumNameLinkVm>,
    /// The header art: the cover of the newest member. `None` with no
    /// member.
    pub(crate) art: Option<CollectionArtVm>,
    /// The empty state text, when the loaded rows hold no member.
    pub(crate) empty_message: Option<&'static str>,
}

/// One member of a window: a Library single or a `MusicIndex` single.
enum WindowMember<'a> {
    Library {
        single: GroupSingle<'a>,
        cover: Option<&'a str>,
    },
    Index {
        display: &'a FeedResultDisplay,
        key: ReleaseKey,
    },
}

impl WindowMember<'_> {
    fn key(&self) -> &ReleaseKey {
        match self {
            Self::Library { single, .. } => &single.key,
            Self::Index { key, .. } => key,
        }
    }

    fn date(&self) -> Option<i64> {
        match self {
            Self::Library { single, .. } => single.date,
            Self::Index { display, .. } => display.newest_item_at,
        }
    }

    fn cover(&self) -> Option<&str> {
        match self {
            Self::Library { cover, .. } => *cover,
            Self::Index { display, .. } => display.thumbnail_href.as_deref(),
        }
    }

    fn in_library(&self) -> bool {
        match self {
            Self::Library { single, .. } => single.in_library,
            Self::Index { .. } => false,
        }
    }

    fn busy(&self, busy: &BusyItems) -> bool {
        match self {
            Self::Library { single, .. } => {
                busy.release_downloading(single.album) || busy.removing(single.album)
            }
            Self::Index { key, .. } => busy.index_release_downloading(key),
        }
    }

    fn row(&self, busy: &BusyItems) -> Option<SinglesGroupPageEntryVm> {
        match self {
            Self::Library { single, .. } => library_single_row(single, busy),
            Self::Index { display, key } => Some(SinglesGroupPageEntryVm::Index(
                SinglesGroupIndexRowVm::from_feed_result(
                    display,
                    key.clone(),
                    display.newest_item_at,
                    busy.index_release_work(key),
                ),
            )),
        }
    }
}

impl WindowGroupPageVm {
    /// Builds the page of the window `key` (packet 004 "The rows of the
    /// page").
    ///
    /// `layout` gives the members of the window from its stored limits.
    /// `rows` are the loaded New releases rows. A member whose feed GUID
    /// the tree holds is a Library member, and each other member is an
    /// Index member (decision D5). `fills` gives the groups of the
    /// "Everything" scope for the link. `busy` makes the main action not
    /// available while one member downloads or one removal runs.
    #[must_use]
    pub(crate) fn build(
        key: &WindowGroupKey,
        layout: &NewReleasesLayout,
        rows: &[&ContentListRowDisplay],
        tree: &LibraryTree,
        fills: &IndexFeedSingles,
        busy: &BusyItems,
    ) -> Self {
        let albums = library_albums(tree);
        let rows_by_id: BTreeMap<&str, &ContentListRowDisplay> =
            rows.iter().map(|row| (row.id.as_str(), *row)).collect();
        let mut own: Vec<WindowMember<'_>> = Vec::new();
        let mut joined: Vec<WindowMember<'_>> = Vec::new();
        for row_id in layout.window_member_row_ids(key) {
            let Some(member) = rows_by_id
                .get(row_id)
                .and_then(|row| window_member(row, &albums))
            else {
                continue;
            };
            if layout.joined_by_name(row_id) {
                joined.push(member);
            } else {
                own.push(member);
            }
        }
        let count = own.len() + joined.len();
        let groups = singles_groups_everything(tree, fills);
        let group = find_group(&groups, &key.group);
        let members = || own.iter().chain(&joined);
        let all_singles_link = group
            .filter(|group| members().all(|member| group.contains(member.key())))
            .map(singles_group_link);
        let artist = layout
            .items()
            .iter()
            .find_map(|item| match item {
                NewReleasesLayoutItem::WindowGroup(tile) if tile.page_key == *key => {
                    Some(tile.title.as_str())
                }
                NewReleasesLayoutItem::WindowGroup(_) | NewReleasesLayoutItem::Release { .. } => {
                    None
                }
            })
            .unwrap_or(key.artist.as_str());
        if count == 0 {
            return Self {
                title: key.artist.clone(),
                date_span: None,
                rows: Vec::new(),
                matched_by_name: None,
                main_action: None,
                all_singles_link,
                art: None,
                empty_message: Some(WINDOW_GROUP_EMPTY_MESSAGE),
            };
        }

        let dates: Vec<i64> = members().filter_map(WindowMember::date).collect();
        let date_span = match (dates.iter().min(), dates.iter().max()) {
            (Some(oldest), Some(newest)) => Some(window_span_text(*oldest, *newest)),
            _ => None,
        };
        let art = CollectionArtVm::resolve(
            None,
            members().next().and_then(WindowMember::cover),
            CollectionKind::SinglesGroup,
        );
        let page_busy = members().any(|member| member.busy(busy));
        let main_action = main_action(members(), count, page_busy);
        let mut page_rows: Vec<SinglesGroupPageEntryVm> =
            own.iter().filter_map(|member| member.row(busy)).collect();
        let joined_rows: Vec<SinglesGroupPageEntryVm> = joined
            .iter()
            .filter_map(|member| member.row(busy))
            .collect();
        let matched_by_name = (!joined_rows.is_empty()).then_some(MatchedByNameHeadingVm {
            heading: PublisherPageVm::MATCHED_BY_NAME_HEADING,
            note: PublisherPageVm::MATCHED_BY_NAME_NOTE,
            count: joined_rows.len(),
        });
        page_rows.extend(joined_rows);
        Self {
            title: window_group_title(artist, count),
            date_span,
            rows: page_rows,
            matched_by_name,
            main_action: Some(main_action),
            all_singles_link,
            art: Some(art),
            empty_message: None,
        }
    }
}

/// The artist node and the album of each Library feed GUID.
fn library_albums(tree: &LibraryTree) -> BTreeMap<&str, (&ArtistNode, &AlbumNode)> {
    let mut albums = BTreeMap::new();
    for artist in &tree.artists {
        for album in &artist.albums {
            if let Some(feed_guid) = album.feed_guid.as_deref() {
                albums.entry(feed_guid).or_insert((artist, album));
            }
        }
    }
    albums
}

/// The member of the New releases row `row`. `None` for a row that is not
/// a release, and for a row with no feed GUID. The layout gives no such
/// row as a member, because a single with no feed GUID has no group key.
fn window_member<'a>(
    row: &'a ContentListRowDisplay,
    albums: &BTreeMap<&str, (&'a ArtistNode, &'a AlbumNode)>,
) -> Option<WindowMember<'a>> {
    let ContentListRowKind::Release(display) = &row.kind else {
        return None;
    };
    let feed_guid = display
        .remote_feed
        .as_ref()
        .and_then(|feed| feed.feed_guid.as_deref())
        .filter(|guid| !guid.trim().is_empty())?;
    Some(match albums.get(feed_guid) {
        Some((artist, album)) => WindowMember::Library {
            single: GroupSingle {
                artist,
                album,
                key: album
                    .release_key()
                    .unwrap_or_else(|| ReleaseKey::from_index_feed_guid(feed_guid)),
                date: album.tracks.iter().filter_map(|track| track.pub_date).max(),
                in_library: !album.tracks.is_empty()
                    && album.tracks.iter().all(|track| track.is_in_library),
            },
            cover: album
                .image_href
                .as_deref()
                .or(display.thumbnail_href.as_deref()),
        },
        None => WindowMember::Index {
            display: display.as_ref(),
            key: ReleaseKey::from_index_feed_guid(feed_guid),
        },
    })
}

/// The main action of the page, by the rule of the ADR 0084 Decision 6
/// page (packet 004 decision D7). A Library member is in the Library when
/// each of its tracks is in the Library. An Index member is never in the
/// Library.
fn main_action<'m, 'a: 'm>(
    members: impl Iterator<Item = &'m WindowMember<'a>>,
    count: usize,
    busy: bool,
) -> SinglesGroupActionVm {
    let mut missing = ReleasesDownloadPlan::default();
    let mut library_feed_ids = Vec::new();
    let mut missing_count = 0;
    for member in members {
        if !member.in_library() {
            missing_count += 1;
        }
        match member {
            WindowMember::Library { single, .. } => {
                if let Some(feed_id) = single.album.feed_id {
                    if !single.in_library {
                        missing.library_feed_ids.push(feed_id);
                    }
                    library_feed_ids.push(feed_id);
                }
            }
            WindowMember::Index { display, key } => {
                if let ReleaseKey::FeedGuid(guid) = key {
                    missing
                        .index_releases
                        .push((guid.clone(), Some(display.label.clone())));
                }
            }
        }
    }
    singles_main_action(missing, missing_count, library_feed_ids, count, busy)
}
