//! The window groups of New releases (ADR 0086 Decisions 1, 2, 3 and 5).
//!
//! A window group holds two or more singles of one [`SinglesGroupKey`]
//! whose release dates are in one window. This module holds the chain of
//! Decision 1 ([`window_groups`]), the view model of a group tile
//! ([`WindowGroupTileVm`]) and the layout of the New releases view
//! ([`NewReleasesLayout`]). The chain reads no tree and no row type, so a
//! saved search can use it (ADR 0086 packet 005).
//!
//! `LibraryViewModel::refresh_new_releases_layout` makes the layout one
//! time for each change of the rows, the tree or the window length. The
//! render and the handlers read that layout. They do not make the groups
//! again.

#![warn(clippy::pedantic)]

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use chrono::Datelike;

use crate::view_models::artist_identity::{
    ArtistIdentity, NameFeedMatch, NameFeedMatches, UNKNOWN_ARTIST_TEXT,
};
use crate::view_models::collection_art::{CollectionArtVm, CollectionKind};
use crate::view_models::format::{fmt_date, fmt_month_day};
use crate::view_models::index_name_list::name_sort_key;
use crate::view_models::library::{
    release_kind_of_feed_result, tree_name_feed_matches, AlbumNode, ArtistNode,
    ContentListLibraryBadgeState, ContentListRowDisplay, ContentListRowKind, LibraryTree,
    IN_LIBRARY_STATE_LABEL, NEW_RELEASE_LABEL,
};
use crate::view_models::list_selection::RowSelectionVm;
use crate::view_models::release_kind::ReleaseKind;
use crate::view_models::search_results::FeedResultDisplay;
use crate::view_models::singles_group::SinglesGroupKey;
use crate::view_models::workspace::ReleaseKey;

/// The number of seconds in one day.
const SECONDS_PER_DAY: i64 = 86_400;

/// The start of the element id of a group tile and of a group row. The
/// row id of the anchor follows it.
pub(crate) const WINDOW_GROUP_ELEMENT_PREFIX: &str = "music-window-group-";

/// The text after the count of a group whose window is open (ADR 0086
/// Decision 5), for example "13+ Singles".
pub(crate) const OPEN_COUNT_SUFFIX: &str = "+";

/// The kind badge of a group of `count` singles (ADR 0086 Decision 5). A
/// closed window gives "13 Singles". An open window gives "13+ Singles",
/// because an older page can add singles to the group.
#[must_use]
pub(crate) fn window_group_count_badge(count: usize, window_closed: bool) -> String {
    let suffix = if window_closed { "" } else { OPEN_COUNT_SUFFIX };
    format!("{count}{suffix} Singles")
}

/// The accessibility label of a group tile. An open window says "or more",
/// because a screen reader can read "+" as "plus" (packet 003 decision
/// D10).
fn window_group_a11y_label(
    title: &str,
    count: usize,
    date_span: &str,
    state: WindowGroupStateVm,
    window_closed: bool,
) -> String {
    let count_words = if window_closed { "" } else { " or more" };
    let mut label = format!(
        "{title}: {count}{count_words} singles, {date_span}, {}",
        state.word()
    );
    if let Some(count_text) = state.count_text() {
        label.push_str(", ");
        label.push_str(&count_text);
    }
    label
}

/// The window length of the groups, in days (ADR 0086 Decision 7). The
/// value is 1 to 60. The default is 14.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct WindowLength(u32);

impl WindowLength {
    /// The default window length: 14 days.
    pub(crate) const DEFAULT: Self = Self(14);
    /// The smallest window length in days.
    #[cfg(test)]
    pub(crate) const MIN_DAYS: u32 = 1;
    /// The largest window length in days.
    #[cfg(test)]
    pub(crate) const MAX_DAYS: u32 = 60;

    /// The window length of `days`. `None` when `days` is not 1 to 60.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn from_days(days: u32) -> Option<Self> {
        (Self::MIN_DAYS..=Self::MAX_DAYS)
            .contains(&days)
            .then_some(Self(days))
    }

    /// The window length in days.
    #[must_use]
    #[cfg(test)]
    pub(crate) const fn days(self) -> u32 {
        self.0
    }

    /// The window length in seconds.
    #[must_use]
    pub(crate) fn seconds(self) -> i64 {
        i64::from(self.0) * SECONDS_PER_DAY
    }
}

impl Default for WindowLength {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// One loaded row, as the input of [`window_groups`]. The entries come in
/// the order of the list.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WindowEntry {
    /// The position of the row in the list.
    pub(crate) position: usize,
    /// The release key of the row. A row with no feed GUID uses its row
    /// id as the GUID. Such a row has no `group_key`, so no group holds
    /// this key.
    pub(crate) key: ReleaseKey,
    /// The group key of a single. `None` for an album, an EP and a row
    /// with no artist.
    pub(crate) group_key: Option<SinglesGroupKey>,
    /// The release date of Decision 1, in Unix seconds.
    pub(crate) release_date: Option<i64>,
    /// The date that sets the order of the list: `newest_item_at`.
    pub(crate) order_date: Option<i64>,
    /// The row shows "In library".
    pub(crate) in_library: bool,
}

/// Two or more singles of one key in one window (ADR 0086 Decision 1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WindowGroup {
    /// The key of each single of the group.
    pub(crate) key: SinglesGroupKey,
    /// The position of the anchor: the newest single of the group.
    pub(crate) anchor: usize,
    /// The position of each single, newest first, with the anchor first.
    pub(crate) members: Vec<usize>,
    /// The release date of the anchor.
    pub(crate) anchor_date: i64,
    /// The release date of the oldest single of the group.
    pub(crate) oldest_date: i64,
    /// The start of the window: one window length before the anchor date.
    pub(crate) window_start: i64,
    /// The number of singles that show "In library".
    pub(crate) in_library_count: usize,
    /// The loaded rows close the window (ADR 0086 Decision 5). One entry
    /// has an order date before `window_start`, or no order date.
    pub(crate) closed_by_rows: bool,
}

/// One item of the list after the grouping.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum WindowItem {
    /// The row at this position shows as itself.
    Plain(usize),
    /// A group shows at the position of its anchor.
    Group(WindowGroup),
}

/// The key of one window group page (ADR 0086 Decision 4, packet 004
/// decision D3).
///
/// The key holds the key of the artist, the anchor single and the two
/// limits of the window when the page opened. Equality compares these four
/// fields. `artist` is a label only, as the label of `ArtistNameKey`.
#[derive(Clone, Debug)]
pub(crate) struct WindowGroupKey {
    /// The key of the artist.
    pub(crate) group: SinglesGroupKey,
    /// The anchor single when the page opened.
    pub(crate) anchor: ReleaseKey,
    /// The release date of the anchor when the page opened.
    pub(crate) anchor_date: i64,
    /// The start of the window when the page opened.
    pub(crate) window_start: i64,
    /// The tile title when the page opened. Equality does not compare it.
    pub(crate) artist: String,
    /// The list that gives the members of the page. Equality compares the
    /// kind of the list, not its rows.
    pub(crate) source: WindowGroupSource,
}

impl PartialEq for WindowGroupKey {
    fn eq(&self, other: &Self) -> bool {
        self.group == other.group
            && self.anchor == other.anchor
            && self.anchor_date == other.anchor_date
            && self.window_start == other.window_start
            && std::mem::discriminant(&self.source) == std::mem::discriminant(&other.source)
    }
}

/// The list that gives the members of a window group page (ADR 0086
/// Decisions 4 and 6).
#[derive(Clone, Debug)]
pub(crate) enum WindowGroupSource {
    /// The loaded New releases rows, through the cached layout.
    NewReleases,
    /// A group row of a saved search page (ADR 0086 packet 005). The rows
    /// are the release rows of each member when the curator clicked the
    /// group row. The page reads the Library facts of these rows at each
    /// render.
    SavedSearch(Vec<FeedResultDisplay>),
}

impl Eq for WindowGroupKey {}

/// The position of each member of the window `key`, newest first (packet
/// 004 "The key of a window").
///
/// A member is an entry of the key of the artist whose release date is in
/// the stored limits of the window. Equal dates keep the list order. The
/// result does not read the current groups, so a history entry shows the
/// window that the curator opened (packet 004 decision D4).
#[must_use]
pub(crate) fn window_members(entries: &[WindowEntry], key: &WindowGroupKey) -> Vec<usize> {
    let mut members: Vec<(i64, usize)> = entries
        .iter()
        .filter(|entry| entry.group_key.as_ref() == Some(&key.group))
        .filter_map(|entry| {
            let date = entry.release_date?;
            (key.window_start..=key.anchor_date)
                .contains(&date)
                .then_some((date, entry.position))
        })
        .collect();
    members.sort_by_key(|(date, position)| (Reverse(*date), *position));
    members.into_iter().map(|(_, position)| position).collect()
}

/// Makes the window groups of `entries` (ADR 0086 Decision 1).
///
/// For each key, the newest single that is in no group is the anchor of a
/// new group. The group takes each single of the key whose release date is
/// at or after one window length before the anchor date. Equal dates keep
/// the list order. A single with no release date and a row with no group
/// key are plain. A group with one single is plain.
///
/// The items come in the order of `entries`. A group comes at the place of
/// its anchor. Each other member gives no item.
#[must_use]
pub(crate) fn window_groups(entries: &[WindowEntry], length: WindowLength) -> Vec<WindowItem> {
    let mut by_key: BTreeMap<&SinglesGroupKey, Vec<(i64, &WindowEntry)>> = BTreeMap::new();
    for entry in entries {
        if let (Some(key), Some(date)) = (&entry.group_key, entry.release_date) {
            by_key.entry(key).or_default().push((date, entry));
        }
    }

    let mut groups_by_anchor: BTreeMap<usize, WindowGroup> = BTreeMap::new();
    let mut grouped: BTreeSet<usize> = BTreeSet::new();
    for (key, mut dated) in by_key {
        dated.sort_by_key(|(date, entry)| (Reverse(*date), entry.position));
        let mut start = 0;
        while let Some(&(anchor_date, anchor)) = dated.get(start) {
            let window_start = anchor_date.saturating_sub(length.seconds());
            let count = dated[start..]
                .iter()
                .take_while(|(date, _)| *date >= window_start)
                .count();
            let members = &dated[start..start + count];
            start += count;
            if members.len() < 2 {
                continue;
            }
            grouped.extend(members.iter().map(|(_, entry)| entry.position));
            let group = WindowGroup {
                key: key.clone(),
                anchor: anchor.position,
                members: members.iter().map(|(_, entry)| entry.position).collect(),
                anchor_date,
                oldest_date: members.last().map_or(anchor_date, |(date, _)| *date),
                window_start,
                in_library_count: members.iter().filter(|(_, entry)| entry.in_library).count(),
                closed_by_rows: entries
                    .iter()
                    .any(|entry| entry.order_date.is_none_or(|date| date < window_start)),
            };
            groups_by_anchor.insert(anchor.position, group);
        }
    }

    entries
        .iter()
        .filter_map(|entry| match groups_by_anchor.remove(&entry.position) {
            Some(group) => Some(WindowItem::Group(group)),
            None => {
                (!grouped.contains(&entry.position)).then_some(WindowItem::Plain(entry.position))
            }
        })
        .collect()
}

/// The date span of a group, from the oldest single to the anchor (ADR
/// 0086 Decision 3). The day is the calendar day in UTC.
///
/// - Two times on one day give one date, for example "Oct 20".
/// - Two days in one year give "Oct 8 to Oct 20".
/// - Two days in different years give the full date of each day.
#[must_use]
pub(crate) fn window_span_text(oldest: i64, anchor: i64) -> String {
    let day = |time: i64| chrono::DateTime::from_timestamp(time, 0).map(|date| date.date_naive());
    let (Some(oldest_day), Some(anchor_day)) = (day(oldest), day(anchor)) else {
        return String::new();
    };
    if oldest_day == anchor_day {
        return fmt_month_day(anchor).unwrap_or_default();
    }
    let (from, to) = if oldest_day.year() == anchor_day.year() {
        (fmt_month_day(oldest), fmt_month_day(anchor))
    } else {
        (fmt_date(oldest), fmt_date(anchor))
    };
    format!("{} to {}", from.unwrap_or_default(), to.unwrap_or_default())
}

/// The state of a group (ADR 0086 Decision 3, ADR 0084 Decision 8).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WindowGroupStateVm {
    /// One or more singles are not in the Library. `in_library` is the
    /// number of singles that are in the Library.
    New { in_library: usize },
    /// The Library holds each single of the group.
    InLibrary,
}

impl WindowGroupStateVm {
    /// The state of a group of `count` singles, `in_library` of them in
    /// the Library.
    #[must_use]
    fn for_counts(count: usize, in_library: usize) -> Self {
        if in_library >= count {
            Self::InLibrary
        } else {
            Self::New { in_library }
        }
    }

    /// The state word: "New" or "In library".
    #[must_use]
    pub(crate) fn word(&self) -> &'static str {
        match self {
            Self::New { .. } => NEW_RELEASE_LABEL,
            Self::InLibrary => IN_LIBRARY_STATE_LABEL,
        }
    }

    /// The Library count after "New", for example "5 in library". `None`
    /// when no single is in the Library, and for "In library".
    #[must_use]
    pub(crate) fn count_text(&self) -> Option<String> {
        match self {
            Self::New { in_library } if *in_library > 0 => Some(format!("{in_library} in library")),
            Self::New { .. } | Self::InLibrary => None,
        }
    }
}

/// The view model of a group tile and of a group row (ADR 0086 Decision
/// 3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WindowGroupTileVm {
    /// The element id: [`WINDOW_GROUP_ELEMENT_PREFIX`] and the anchor row id.
    pub(crate) element_id: String,
    /// The row id of the anchor. It identifies the tile. A click and Enter
    /// open the window group page, not this row (packet 004 decision D10).
    pub(crate) open_row_id: String,
    /// The key of the group.
    pub(crate) key: SinglesGroupKey,
    /// The release key of each single, newest first.
    pub(crate) member_keys: Vec<ReleaseKey>,
    /// The artist name of the key.
    pub(crate) title: String,
    /// The date span, for example "Oct 8 to Oct 20".
    pub(crate) date_span: String,
    /// The kind badge, for example "3 Singles", or "3+ Singles" for an
    /// open window. The renderer shows this text and adds nothing.
    pub(crate) kind_badge: String,
    /// The number of singles.
    pub(crate) count: usize,
    /// The state word and the Library count.
    pub(crate) state: WindowGroupStateVm,
    /// The accessibility label of the tile.
    pub(crate) a11y_label: String,
    /// The art: the anchor cover as a square, or the logo in the entity
    /// color (ADR 0084 Decision 12).
    pub(crate) art: CollectionArtVm,
    /// The loaded rows close the window (ADR 0086 Decision 5).
    pub(crate) window_closed: bool,
    /// The selection facts of the tile: a group row of each member key.
    pub(crate) selection: RowSelectionVm,
    /// The key of the window group page that a click opens (packet 004).
    pub(crate) page_key: WindowGroupKey,
}

impl WindowGroupTileVm {
    /// The tile of `group`. `entries` holds the entries that
    /// [`window_groups`] read. `anchor_row_id` is the row id of the anchor,
    /// `title` is the artist name of the key and `anchor_cover` is the
    /// cover of the anchor.
    #[must_use]
    pub(crate) fn new(
        group: &WindowGroup,
        entries: &[WindowEntry],
        anchor_row_id: &str,
        title: &str,
        anchor_cover: Option<&str>,
    ) -> Self {
        let member_keys: Vec<ReleaseKey> = group
            .members
            .iter()
            .filter_map(|position| {
                entries
                    .iter()
                    .find(|entry| entry.position == *position)
                    .map(|entry| entry.key.clone())
            })
            .collect();
        let count = group.members.len();
        let state = WindowGroupStateVm::for_counts(count, group.in_library_count);
        let date_span = window_span_text(group.oldest_date, group.anchor_date);
        let window_closed = group.closed_by_rows;
        let anchor = entries
            .iter()
            .find(|entry| entry.position == group.anchor)
            .map_or_else(
                || ReleaseKey::from_index_feed_guid(anchor_row_id),
                |entry| entry.key.clone(),
            );
        let page_key = WindowGroupKey {
            group: group.key.clone(),
            anchor,
            anchor_date: group.anchor_date,
            window_start: group.window_start,
            artist: title.to_owned(),
            source: WindowGroupSource::NewReleases,
        };
        Self {
            element_id: format!("{WINDOW_GROUP_ELEMENT_PREFIX}{anchor_row_id}"),
            open_row_id: anchor_row_id.to_owned(),
            key: group.key.clone(),
            member_keys,
            title: title.to_owned(),
            kind_badge: window_group_count_badge(count, window_closed),
            count,
            state,
            a11y_label: window_group_a11y_label(title, count, &date_span, state, window_closed),
            date_span,
            art: CollectionArtVm::resolve(None, anchor_cover, CollectionKind::SinglesGroup),
            window_closed,
            selection: RowSelectionVm::none(),
            page_key,
        }
    }

    /// Sets the window state, and the kind badge and the accessibility
    /// label that show it (ADR 0086 Decisions 5 and 6).
    pub(crate) fn set_window_closed(&mut self, closed: bool) {
        self.window_closed = closed;
        self.kind_badge = window_group_count_badge(self.count, closed);
        self.a11y_label =
            window_group_a11y_label(&self.title, self.count, &self.date_span, self.state, closed);
    }
}

/// One item of the New releases view in `SelectionMode::Normal`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum NewReleasesLayoutItem {
    /// The row with this id shows as itself.
    Release { row_id: String },
    /// A group tile at the place of its anchor.
    WindowGroup(Box<WindowGroupTileVm>),
}

/// The items of the New releases view, after the grouping (ADR 0086
/// Decisions 1 and 2).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct NewReleasesLayout {
    items: Vec<NewReleasesLayoutItem>,
    /// The window entry of each row, by position (packet 004).
    entries: Vec<WindowEntry>,
    /// The row id of each position.
    row_ids: Vec<String>,
    /// The positions of each single whose own identity is a name that the
    /// tree joins to an artist feed (ADR 0084 Decision 6, packet 004
    /// decision D6).
    joined_by_name: BTreeSet<usize>,
}

impl NewReleasesLayout {
    /// The layout of `rows`, in their visible order. `tree` gives the facts
    /// of a Library row and the "Matched by name" rule. `length` is the
    /// window length.
    #[must_use]
    pub(crate) fn build(
        rows: &[&ContentListRowDisplay],
        tree: &LibraryTree,
        length: WindowLength,
    ) -> Self {
        if rows.is_empty() {
            return Self::default();
        }
        let facts = window_entries(rows, tree);
        let entries: Vec<WindowEntry> = facts.iter().map(|fact| fact.entry.clone()).collect();
        let mut layout = Self {
            row_ids: rows.iter().map(|row| row.id.clone()).collect(),
            joined_by_name: facts
                .iter()
                .filter(|fact| fact.joined_by_name)
                .map(|fact| fact.entry.position)
                .collect(),
            ..Self::default()
        };
        for item in window_groups(&entries, length) {
            match item {
                WindowItem::Plain(position) => {
                    layout.items.push(NewReleasesLayoutItem::Release {
                        row_id: rows[position].id.clone(),
                    });
                }
                WindowItem::Group(group) => {
                    let anchor = rows[group.anchor];
                    let tile = WindowGroupTileVm::new(
                        &group,
                        &entries,
                        &anchor.id,
                        &facts[group.anchor].title,
                        anchor.thumbnail_href(),
                    );
                    layout
                        .items
                        .push(NewReleasesLayoutItem::WindowGroup(Box::new(tile)));
                }
            }
        }
        layout.entries = entries;
        layout
    }

    /// The layout of the window group page of a saved search (ADR 0086
    /// packet 005). `entries` holds the entry of each row, and `row_ids`
    /// the row id of each position. `joined_by_name` holds the positions
    /// of the singles that a name match joins to an artist feed. The
    /// layout has no item, because the page reads only the members.
    #[must_use]
    pub(crate) fn from_entries(
        entries: Vec<WindowEntry>,
        row_ids: Vec<String>,
        joined_by_name: BTreeSet<usize>,
    ) -> Self {
        Self {
            items: Vec::new(),
            entries,
            row_ids,
            joined_by_name,
        }
    }

    /// The row id of each member of the window `key`, newest first (packet
    /// 004). The members come from the stored limits of the key, not from
    /// the current groups. Empty when the loaded rows hold no single of
    /// the window.
    #[must_use]
    pub(crate) fn window_member_row_ids(&self, key: &WindowGroupKey) -> Vec<&str> {
        window_members(&self.entries, key)
            .into_iter()
            .filter_map(|position| self.row_ids.get(position).map(String::as_str))
            .collect()
    }

    /// Tells if the single of the row `row_id` joins its group through a
    /// name match (ADR 0084 Decision 6). Its own identity is a name, and
    /// the tree matches that name to one artist feed.
    #[must_use]
    pub(crate) fn joined_by_name(&self, row_id: &str) -> bool {
        self.row_ids
            .iter()
            .position(|id| id == row_id)
            .is_some_and(|position| self.joined_by_name.contains(&position))
    }

    /// The group tile whose anchor is `anchor`. `None` for a single that
    /// is not the anchor of a tile.
    #[must_use]
    pub(crate) fn group_tile_for_anchor(&self, anchor: &ReleaseKey) -> Option<&WindowGroupTileVm> {
        self.items.iter().find_map(|item| match item {
            NewReleasesLayoutItem::WindowGroup(tile) if &tile.page_key.anchor == anchor => {
                Some(tile.as_ref())
            }
            NewReleasesLayoutItem::WindowGroup(_) | NewReleasesLayoutItem::Release { .. } => None,
        })
    }

    /// The items, in the order of the list.
    #[must_use]
    pub(crate) fn items(&self) -> &[NewReleasesLayoutItem] {
        &self.items
    }

    /// The row id of each item: the row of a plain item and the anchor row
    /// of a group, in item order. No production code calls it since
    /// packet 004, which gives the selection rows through
    /// `discover_grouped_release_rows`.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn shown_row_ids(&self) -> Vec<&str> {
        self.items
            .iter()
            .map(|item| match item {
                NewReleasesLayoutItem::Release { row_id } => row_id.as_str(),
                NewReleasesLayoutItem::WindowGroup(tile) => tile.open_row_id.as_str(),
            })
            .collect()
    }

    /// One or more groups have a window that the loaded rows do not close
    /// (ADR 0086 Decision 5).
    /// A single tile is not a group, so it never counts (packet 003
    /// decision D9).
    #[must_use]
    pub(crate) fn any_window_open(&self) -> bool {
        self.items.iter().any(|item| match item {
            NewReleasesLayoutItem::WindowGroup(tile) => !tile.window_closed,
            NewReleasesLayoutItem::Release { .. } => false,
        })
    }

    /// Closes the window of each group (ADR 0086 Decision 5). The list has
    /// no more pages, so no older single exists.
    pub(crate) fn mark_list_end(&mut self) {
        for item in &mut self.items {
            if let NewReleasesLayoutItem::WindowGroup(tile) = item {
                tile.set_window_closed(true);
            }
        }
    }
}

/// One item of a content list: a row, or a group tile of New releases
/// (ADR 0086 Decision 2). The Tiles mode and the List mode show the same
/// items.
#[derive(Clone, Debug)]
#[expect(
    clippy::large_enum_variant,
    reason = "a content list holds one item for each row, and a row was never boxed in that list"
)]
pub(crate) enum ContentListItemVm {
    /// One row or tile.
    Row(ContentListRowDisplay),
    /// One group tile or group row.
    WindowGroup(Box<WindowGroupTileVm>),
}

/// The window entry of one row, with the artist name of its key.
#[derive(Clone, Debug)]
struct RowWindowFacts {
    entry: WindowEntry,
    title: String,
    /// The group key comes from a name that the tree matches to one artist
    /// feed (packet 002 decision D4).
    joined_by_name: bool,
}

/// The window entry of each row (ADR 0086 Decision 1, packet 002
/// decisions D3, D4 and D5). The album map and the name matches are made
/// one time.
fn window_entries(rows: &[&ContentListRowDisplay], tree: &LibraryTree) -> Vec<RowWindowFacts> {
    let mut albums: BTreeMap<&str, (&ArtistNode, &AlbumNode)> = BTreeMap::new();
    for artist in &tree.artists {
        for album in &artist.albums {
            if let Some(feed_guid) = album.feed_guid.as_deref() {
                albums.entry(feed_guid).or_insert((artist, album));
            }
        }
    }
    let matches = tree_name_feed_matches(tree);
    rows.iter()
        .enumerate()
        .map(|(position, row)| row_window_facts(position, row, &albums, &matches))
        .collect()
}

/// The window entry of one row.
///
/// - A row whose feed GUID the tree holds takes the kind, the identity and
///   the date of the Library album (D3).
/// - Each other row takes the kind and the identity of its `remote_feed`,
///   and the date `newest_item_at` (D5).
/// - A name key that matches one Library artist feed becomes the key of
///   that feed (D4).
fn row_window_facts(
    position: usize,
    row: &ContentListRowDisplay,
    albums: &BTreeMap<&str, (&ArtistNode, &AlbumNode)>,
    matches: &NameFeedMatches,
) -> RowWindowFacts {
    let display = match &row.kind {
        ContentListRowKind::Release(display) => Some(display.as_ref()),
        ContentListRowKind::Track(_) => None,
    };
    let remote = display.and_then(|display| display.remote_feed.as_ref());
    let feed_guid = remote
        .and_then(|feed| feed.feed_guid.as_deref())
        .filter(|guid| !guid.trim().is_empty());
    let mut entry = WindowEntry {
        position,
        key: ReleaseKey::from_index_feed_guid(feed_guid.unwrap_or(row.id.as_str())),
        group_key: None,
        release_date: None,
        order_date: display.and_then(|display| display.newest_item_at),
        in_library: row.library_badge.state == ContentListLibraryBadgeState::InLibrary,
    };
    let (Some(display), Some(remote), Some(feed_guid)) = (display, remote, feed_guid) else {
        return RowWindowFacts {
            entry,
            title: UNKNOWN_ARTIST_TEXT.to_owned(),
            joined_by_name: false,
        };
    };

    let (kind, identity, release_date, library_title) = match albums.get(feed_guid) {
        Some((artist, album)) => (
            album.release_kind(),
            album.artist_identity(&artist.name),
            album.tracks.iter().filter_map(|track| track.pub_date).max(),
            Some(artist.name.clone()),
        ),
        None => (
            release_kind_of_feed_result(display),
            ArtistIdentity::resolve(
                &remote.named_feeds,
                remote.artist.as_deref().unwrap_or_default(),
            ),
            display.newest_item_at,
            None,
        ),
    };
    entry.release_date = release_date;
    let mut joined_by_name = false;
    if kind == ReleaseKind::Single {
        (entry.group_key, joined_by_name) = group_key(identity, matches);
    }

    let title = library_title.unwrap_or_else(|| {
        let named_title = match &entry.group_key {
            Some(SinglesGroupKey(ArtistIdentity::Publisher(guid))) => remote
                .named_feeds
                .iter()
                .find(|feed| &feed.feed_guid == guid)
                .and_then(|feed| feed.title.clone()),
            _ => None,
        };
        named_title
            .or_else(|| {
                remote
                    .artist
                    .as_deref()
                    .map(str::trim)
                    .filter(|artist| !artist.is_empty())
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| UNKNOWN_ARTIST_TEXT.to_owned())
    });
    RowWindowFacts {
        entry,
        title,
        joined_by_name,
    }
}

/// The group key of an identity (packet 002 decision D4), and `true` when
/// a name match gives the key. An empty name and the name "Unknown
/// Artist" give no key, because they name no artist. A saved search uses
/// the same rule (ADR 0086 packet 005).
pub(crate) fn group_key(
    identity: ArtistIdentity,
    matches: &NameFeedMatches,
) -> (Option<SinglesGroupKey>, bool) {
    match identity {
        ArtistIdentity::Name(key) => {
            if key.is_empty() || key == name_sort_key(UNKNOWN_ARTIST_TEXT) {
                return (None, false);
            }
            match matches.get(&key) {
                NameFeedMatch::MatchedFeed(feed_guid) => (
                    Some(SinglesGroupKey(ArtistIdentity::Publisher(feed_guid))),
                    true,
                ),
                NameFeedMatch::Ambiguous | NameFeedMatch::NoFeed => {
                    (Some(SinglesGroupKey(ArtistIdentity::Name(key))), false)
                }
            }
        }
        publisher @ ArtistIdentity::Publisher(_) => (Some(SinglesGroupKey(publisher)), false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::TrackRow;
    use crate::view_models::named_feeds::NamedFeed;
    use crate::view_models::search_results::{FeedResultDisplay, SearchResultOrigin};
    use crate::views::{FeedMetadataFacts, FeedView, LocalIdentityFacts};

    /// The UTC midnight of `day` in `month` of 2026.
    fn day(month: u32, day: u32) -> i64 {
        chrono::NaiveDate::from_ymd_opt(2026, month, day)
            .and_then(|date| date.and_hms_opt(0, 0, 0))
            .expect("a valid date")
            .and_utc()
            .timestamp()
    }

    fn oct(date: u32) -> i64 {
        day(10, date)
    }

    fn publisher(guid: &str) -> SinglesGroupKey {
        SinglesGroupKey(ArtistIdentity::Publisher(guid.to_owned()))
    }

    /// One entry at `position`, with the order date equal to the release
    /// date.
    fn entry(position: usize, key: Option<SinglesGroupKey>, date: Option<i64>) -> WindowEntry {
        WindowEntry {
            position,
            key: ReleaseKey::FeedGuid(format!("guid-{position}")),
            group_key: key,
            release_date: date,
            order_date: date,
            in_library: false,
        }
    }

    /// Singles of one key at `dates`, in the list order.
    fn singles(dates: &[i64]) -> Vec<WindowEntry> {
        dates
            .iter()
            .enumerate()
            .map(|(position, date)| entry(position, Some(publisher("jsmovie")), Some(*date)))
            .collect()
    }

    fn groups(items: &[WindowItem]) -> Vec<&WindowGroup> {
        items
            .iter()
            .filter_map(|item| match item {
                WindowItem::Group(group) => Some(group),
                WindowItem::Plain(_) => None,
            })
            .collect()
    }

    fn artist_feed(guid: &str, title: &str) -> NamedFeed {
        NamedFeed {
            feed_guid: guid.to_owned(),
            title: Some(title.to_owned()),
            position: Some(0),
            role: Some("artist".to_owned()),
            role_agreement: Some("both".to_owned()),
            feed_rel: None,
            album_rel: None,
            confirmed: true,
            image_url: None,
        }
    }

    /// One Index row of New releases, as the detail read gives it.
    fn index_row(
        guid: &str,
        artist: &str,
        kind: &str,
        date: Option<i64>,
        named_feed: Option<NamedFeed>,
    ) -> ContentListRowDisplay {
        let feed = FeedView {
            feed_guid: Some(guid.to_owned()),
            artist: Some(artist.to_owned()),
            release_kind: Some(kind.to_owned()),
            named_feeds: named_feed.into_iter().collect(),
            ..FeedView::default()
        };
        ContentListRowDisplay::from_release_result(
            FeedResultDisplay::new(
                format!("index-feed:{guid}"),
                format!("Release {guid}"),
                SearchResultOrigin::Index,
            )
            .with_thumbnail_href(format!("https://example.test/{guid}.jpg"))
            .with_remote_feed(feed)
            .with_newest_item_at(date),
        )
    }

    fn jsmovie_single(guid: &str, date: i64) -> ContentListRowDisplay {
        index_row(
            guid,
            "jSmovie",
            "single",
            Some(date),
            Some(artist_feed("feed-jsmovie", "jSmovie")),
        )
    }

    fn in_library(mut row: ContentListRowDisplay) -> ContentListRowDisplay {
        row.library_badge.state = ContentListLibraryBadgeState::InLibrary;
        row
    }

    /// A Library album of artist `artist` with one track for each date.
    fn library_album(guid: &str, dates: &[i64], named_feed: Option<NamedFeed>) -> AlbumNode {
        AlbumNode {
            name: format!("Release {guid}"),
            feed_id: Some(1),
            feed_guid: Some(guid.to_owned()),
            feed_url: None,
            language: None,
            description: None,
            image_href: None,
            named_feeds: named_feed.into_iter().collect(),
            identity_facts: LocalIdentityFacts::default(),
            metadata_facts: Box::new(FeedMetadataFacts::default()),
            stored_values: None,
            tracks: dates
                .iter()
                .map(|date| TrackRow {
                    pub_date: Some(*date),
                    is_in_library: true,
                    ..TrackRow::default()
                })
                .collect(),
        }
    }

    fn tree(artist: &str, albums: Vec<AlbumNode>) -> LibraryTree {
        LibraryTree {
            artists: vec![ArtistNode {
                name: artist.to_owned(),
                albums,
            }],
        }
    }

    fn layout(rows: &[ContentListRowDisplay], tree: &LibraryTree) -> NewReleasesLayout {
        let rows: Vec<&ContentListRowDisplay> = rows.iter().collect();
        NewReleasesLayout::build(&rows, tree, WindowLength::DEFAULT)
    }

    fn tiles(layout: &NewReleasesLayout) -> Vec<&WindowGroupTileVm> {
        layout
            .items()
            .iter()
            .filter_map(|item| match item {
                NewReleasesLayoutItem::WindowGroup(tile) => Some(tile.as_ref()),
                NewReleasesLayoutItem::Release { .. } => None,
            })
            .collect()
    }

    /// R86-2-01: singles on Oct 20, Oct 15, Oct 8 and Oct 1 give one group
    /// of three and one single.
    #[test]
    fn adr_0086_window_groups_r01_four_singles_give_a_group_of_three() {
        let items = window_groups(
            &singles(&[oct(20), oct(15), oct(8), oct(1)]),
            WindowLength::DEFAULT,
        );
        assert_eq!(items.len(), 2);
        assert_eq!(groups(&items).len(), 1);
        assert_eq!(groups(&items)[0].members.len(), 3);
    }

    /// R86-2-02: the anchor is Oct 20, the members are Oct 20, Oct 15 and
    /// Oct 8, and Oct 1 is plain at its own position.
    #[test]
    fn adr_0086_window_groups_r02_anchor_members_and_plain_single() {
        let items = window_groups(
            &singles(&[oct(20), oct(15), oct(8), oct(1)]),
            WindowLength::DEFAULT,
        );
        let WindowItem::Group(group) = &items[0] else {
            panic!("the first item is the group");
        };
        assert_eq!(group.anchor, 0);
        assert_eq!(group.anchor_date, oct(20));
        assert_eq!(group.members, [0, 1, 2]);
        assert_eq!(group.oldest_date, oct(8));
        assert_eq!(items[1], WindowItem::Plain(3));
    }

    /// R86-2-03: the next window does not start from Oct 8.
    #[test]
    fn adr_0086_window_groups_r03_the_window_does_not_slide() {
        let items = window_groups(&singles(&[oct(20), oct(8), oct(1)]), WindowLength::DEFAULT);
        assert_eq!(groups(&items).len(), 1);
        assert_eq!(groups(&items)[0].members, [0, 1]);
        assert_eq!(items[1], WindowItem::Plain(2));
    }

    /// R86-2-04: the window boundary is inclusive.
    #[test]
    fn adr_0086_window_groups_r04_the_boundary_is_inclusive() {
        let length = WindowLength::DEFAULT.seconds();
        let joins = window_groups(
            &singles(&[oct(20), oct(20) - length]),
            WindowLength::DEFAULT,
        );
        assert_eq!(groups(&joins).len(), 1);

        let stays = window_groups(
            &singles(&[oct(20), oct(20) - length - 1]),
            WindowLength::DEFAULT,
        );
        assert_eq!(stays, [WindowItem::Plain(0), WindowItem::Plain(1)]);
    }

    /// R86-2-05: Oct 20, Oct 15, Oct 1 and Sep 25 give two groups.
    #[test]
    fn adr_0086_window_groups_r05_two_windows_give_two_groups() {
        let items = window_groups(
            &singles(&[oct(20), oct(15), oct(1), day(9, 25)]),
            WindowLength::DEFAULT,
        );
        let anchors: Vec<i64> = groups(&items)
            .iter()
            .map(|group| group.anchor_date)
            .collect();
        assert_eq!(anchors, [oct(20), oct(1)]);
    }

    /// R86-2-06: one single is plain. The layout gives its row as
    /// `Release`, and its kind badge stays "Single".
    #[test]
    fn adr_0086_window_groups_r06_one_single_stays_a_release() {
        assert_eq!(
            window_groups(&singles(&[oct(20)]), WindowLength::DEFAULT),
            [WindowItem::Plain(0)]
        );
        let rows = [jsmovie_single("a", oct(20))];
        let layout = layout(&rows, &LibraryTree::default());
        assert!(matches!(
            layout.items(),
            [NewReleasesLayoutItem::Release { row_id }] if row_id == "index-feed:a"
        ));
        assert_eq!(rows[0].entity_badge.label, "Single");
    }

    /// R86-2-07: an album and an EP of the key have no group key. They
    /// stay plain at their positions, and the two singles make one group.
    #[test]
    fn adr_0086_window_groups_r07_albums_and_eps_never_join() {
        let feed = || Some(artist_feed("feed-jsmovie", "jSmovie"));
        let rows = [
            jsmovie_single("a", oct(20)),
            index_row("album", "jSmovie", "album", Some(oct(18)), feed()),
            index_row("ep", "jSmovie", "ep", Some(oct(16)), feed()),
            jsmovie_single("b", oct(14)),
        ];
        let entries = window_entries(&rows.iter().collect::<Vec<_>>(), &LibraryTree::default());
        assert_eq!(entries[1].entry.group_key, None);
        assert_eq!(entries[2].entry.group_key, None);

        let layout = layout(&rows, &LibraryTree::default());
        assert_eq!(
            layout.shown_row_ids(),
            ["index-feed:a", "index-feed:album", "index-feed:ep"]
        );
        assert_eq!(tiles(&layout)[0].count, 2);
    }

    /// R86-2-08: a single with no release date never anchors and never
    /// joins.
    #[test]
    fn adr_0086_window_groups_r08_no_release_date_stays_plain() {
        let mut entries = singles(&[oct(20), oct(19), oct(18)]);
        entries[0].release_date = None;
        let items = window_groups(&entries, WindowLength::DEFAULT);
        assert_eq!(items[0], WindowItem::Plain(0));
        assert_eq!(groups(&items)[0].members, [1, 2]);
    }

    /// R86-2-09: two feeds with one artist text never share a group, and a
    /// name key never shares a group with a feed key.
    #[test]
    fn adr_0086_window_groups_r09_keys_never_mix() {
        let name = SinglesGroupKey(ArtistIdentity::Name("jsmovie".to_owned()));
        let entries = vec![
            entry(0, Some(publisher("a")), Some(oct(20))),
            entry(1, Some(publisher("b")), Some(oct(19))),
            entry(2, Some(name), Some(oct(18))),
        ];
        assert_eq!(
            window_groups(&entries, WindowLength::DEFAULT),
            [
                WindowItem::Plain(0),
                WindowItem::Plain(1),
                WindowItem::Plain(2)
            ]
        );
    }

    /// R86-2-10: the group has the position of its anchor. The other
    /// members give no item, and each other row keeps its order.
    #[test]
    fn adr_0086_window_groups_r10_group_takes_the_anchor_position() {
        let entries = vec![
            entry(0, None, Some(oct(21))),
            entry(1, Some(publisher("a")), Some(oct(20))),
            entry(2, None, Some(oct(19))),
            entry(3, Some(publisher("a")), Some(oct(18))),
            entry(4, None, Some(oct(17))),
        ];
        let items = window_groups(&entries, WindowLength::DEFAULT);
        assert_eq!(items.len(), 4);
        assert_eq!(items[0], WindowItem::Plain(0));
        assert!(matches!(&items[1], WindowItem::Group(group) if group.anchor == 1));
        assert_eq!(items[2], WindowItem::Plain(2));
        assert_eq!(items[3], WindowItem::Plain(4));
    }

    /// R86-2-11: with one date, the row with the lower position anchors.
    #[test]
    fn adr_0086_window_groups_r11_equal_dates_keep_the_list_order() {
        let items = window_groups(&singles(&[oct(20), oct(20)]), WindowLength::DEFAULT);
        assert!(matches!(
            &items[..],
            [WindowItem::Group(group)] if group.anchor == 0 && group.members == [0, 1]
        ));
    }

    /// R86-2-12: an order date before the window start, or no order date,
    /// closes the window.
    #[test]
    fn adr_0086_window_groups_r12_rows_close_the_window() {
        let open = window_groups(&singles(&[oct(20), oct(15)]), WindowLength::DEFAULT);
        assert!(!groups(&open)[0].closed_by_rows);

        let closed = window_groups(&singles(&[oct(20), oct(15), oct(1)]), WindowLength::DEFAULT);
        assert!(groups(&closed)[0].closed_by_rows);

        let mut entries = singles(&[oct(20), oct(15)]);
        entries.push(WindowEntry {
            order_date: None,
            ..entry(2, None, None)
        });
        assert!(groups(&window_groups(&entries, WindowLength::DEFAULT))[0].closed_by_rows);
    }

    /// R86-2-13: an Index row takes `Publisher(guid)` from its confirmed
    /// artist feed and its date from `newest_item_at`.
    #[test]
    fn adr_0086_window_groups_r13_index_row_reads_its_feed() {
        let rows = [jsmovie_single("a", oct(20))];
        let facts = window_entries(&rows.iter().collect::<Vec<_>>(), &LibraryTree::default());
        assert_eq!(facts[0].entry.group_key, Some(publisher("feed-jsmovie")));
        assert_eq!(facts[0].entry.release_date, Some(oct(20)));
        assert_eq!(facts[0].entry.order_date, Some(oct(20)));
        assert_eq!(facts[0].title, "jSmovie");
    }

    /// R86-2-14: a name that matches one Library artist feed joins the
    /// group of that feed. A name that matches two feeds keeps its name
    /// key.
    #[test]
    fn adr_0086_window_groups_r14_matched_by_name() {
        let rows = [
            jsmovie_single("a", oct(20)),
            index_row("b", "jSmovie", "single", Some(oct(19)), None),
        ];
        let one_feed = tree(
            "jSmovie",
            vec![library_album(
                "library",
                &[day(1, 1)],
                Some(artist_feed("feed-jsmovie", "jSmovie")),
            )],
        );
        assert_eq!(tiles(&layout(&rows, &one_feed)).len(), 1);

        let two_feeds = tree(
            "jSmovie",
            vec![
                library_album("one", &[day(1, 1)], Some(artist_feed("feed-1", "x"))),
                library_album("two", &[day(1, 1)], Some(artist_feed("feed-2", "y"))),
            ],
        );
        let facts = window_entries(&rows.iter().collect::<Vec<_>>(), &two_feeds);
        assert_eq!(
            facts[1].entry.group_key,
            Some(SinglesGroupKey(ArtistIdentity::Name("jsmovie".to_owned())))
        );
        assert!(tiles(&layout(&rows, &two_feeds)).is_empty());
    }

    /// R86-2-15: a Library row takes the latest track `pub_date`, not
    /// `newest_item_at`.
    #[test]
    fn adr_0086_window_groups_r15_library_row_takes_the_track_date() {
        let rows = [
            jsmovie_single("anchor", oct(20)),
            in_library(jsmovie_single("library", oct(19))),
        ];
        let library = tree(
            "jSmovie",
            vec![library_album(
                "library",
                &[oct(1)],
                Some(artist_feed("feed-jsmovie", "jSmovie")),
            )],
        );
        let facts = window_entries(&rows.iter().collect::<Vec<_>>(), &library);
        assert_eq!(facts[1].entry.release_date, Some(oct(1)));
        assert_eq!(facts[1].entry.order_date, Some(oct(19)));
        assert!(tiles(&layout(&rows, &library)).is_empty());
    }

    /// R86-2-16: an empty artist, "Unknown Artist" and no feed GUID give
    /// plain rows.
    #[test]
    fn adr_0086_window_groups_r16_rows_with_no_artist_stay_plain() {
        let mut no_guid = index_row("x", "Rain", "single", Some(oct(18)), None);
        if let ContentListRowKind::Release(display) = &mut no_guid.kind {
            if let Some(feed) = display.remote_feed.as_mut() {
                feed.feed_guid = None;
            }
        }
        let rows = [
            index_row("a", "", "single", Some(oct(20)), None),
            index_row("b", "", "single", Some(oct(20)), None),
            index_row("c", "Unknown Artist", "single", Some(oct(19)), None),
            index_row("d", "Unknown Artist", "single", Some(oct(19)), None),
            no_guid.clone(),
            no_guid,
        ];
        let facts = window_entries(&rows.iter().collect::<Vec<_>>(), &LibraryTree::default());
        assert!(facts.iter().all(|fact| fact.entry.group_key.is_none()));
        assert!(tiles(&layout(&rows, &LibraryTree::default())).is_empty());
    }

    /// R86-2-17: the tile of R86-2-01 gives the title, the span, the badge
    /// and the count.
    #[test]
    fn adr_0086_window_groups_r17_tile_facts() {
        let rows = [
            jsmovie_single("a", oct(20)),
            jsmovie_single("b", oct(15)),
            jsmovie_single("c", oct(8)),
            jsmovie_single("d", oct(1)),
        ];
        let layout = layout(&rows, &LibraryTree::default());
        let tile = tiles(&layout)[0];
        assert_eq!(tile.title, "jSmovie");
        assert_eq!(tile.date_span, "Oct 8 to Oct 20");
        assert_eq!(tile.kind_badge, "3 Singles");
        assert_eq!(tile.count, 3);
        assert_eq!(tile.key, publisher("feed-jsmovie"));
        assert_eq!(
            tile.member_keys,
            ["a", "b", "c"].map(|guid| ReleaseKey::FeedGuid(guid.to_owned()))
        );
        assert_eq!(tile.open_row_id, "index-feed:a");
        assert_eq!(layout.shown_row_ids(), ["index-feed:a", "index-feed:d"]);
    }

    /// The tile of nine singles, the first `library` of them in the
    /// Library.
    fn nine_singles(library: usize) -> WindowGroupTileVm {
        let mut entries = singles(&(0..9).map(|n| oct(20) - n).collect::<Vec<_>>());
        for entry in entries.iter_mut().take(library) {
            entry.in_library = true;
        }
        let items = window_groups(&entries, WindowLength::DEFAULT);
        WindowGroupTileVm::new(groups(&items)[0], &entries, "index-feed:a", "jSmovie", None)
    }

    /// R86-2-18: five Library rows of nine give "New" and "5 in library".
    /// Nine give "In library" and no count.
    #[test]
    fn adr_0086_window_groups_r18_state_and_library_count() {
        let some = nine_singles(5);
        assert_eq!(some.state.word(), "New");
        assert_eq!(some.state.count_text().as_deref(), Some("5 in library"));

        let all = nine_singles(9);
        assert_eq!(all.state, WindowGroupStateVm::InLibrary);
        assert_eq!(all.state.word(), "In library");
        assert_eq!(all.state.count_text(), None);
    }

    /// R86-2-19: a group with no Library row gives "New" and no count.
    #[test]
    fn adr_0086_window_groups_r19_no_library_row_gives_no_count() {
        let none = nine_singles(0);
        assert_eq!(none.state.word(), "New");
        assert_eq!(none.state.count_text(), None);
    }

    /// R86-2-20: the accessibility label and the element id. The album of
    /// Oct 1 closes the window (R86-3-29).
    #[test]
    fn adr_0086_window_groups_r20_a11y_label_and_element_id() {
        let rows = [
            in_library(jsmovie_single("a", oct(20))),
            jsmovie_single("b", oct(15)),
            jsmovie_single("c", oct(8)),
            index_row("old", "Rain", "album", Some(oct(1)), None),
        ];
        let layout = layout(&rows, &LibraryTree::default());
        let tile = tiles(&layout)[0];
        assert_eq!(
            tile.a11y_label,
            "jSmovie: 3 singles, Oct 8 to Oct 20, New, 1 in library"
        );
        assert!(tile.element_id.starts_with(WINDOW_GROUP_ELEMENT_PREFIX));
        assert_eq!(tile.element_id, "music-window-group-index-feed:a");
    }

    /// R86-3-13: the count badge of an open window and of a closed window.
    #[test]
    fn adr_0086_pager_windows_r13_count_badge() {
        assert_eq!(window_group_count_badge(13, false), "13+ Singles");
        assert_eq!(window_group_count_badge(13, true), "13 Singles");
        assert_eq!(OPEN_COUNT_SUFFIX, "+");
    }

    /// The rows of 13 jSmovie singles from Oct 20 to Oct 8, the first in
    /// the Library.
    fn thirteen_singles() -> Vec<ContentListRowDisplay> {
        let mut rows: Vec<ContentListRowDisplay> = (0..13)
            .map(|n| jsmovie_single(&format!("s{n}"), oct(20 - n)))
            .collect();
        rows[0] = in_library(rows[0].clone());
        rows
    }

    /// R86-3-14: an open group says "or more" in its accessibility label.
    /// A closed group keeps the label of R86-2-20.
    #[test]
    fn adr_0086_pager_windows_r14_open_a11y_label() {
        let mut rows = thirteen_singles();
        let open = layout(&rows, &LibraryTree::default());
        let tile = tiles(&open)[0];
        assert!(!tile.window_closed);
        assert_eq!(
            tile.a11y_label,
            "jSmovie: 13 or more singles, Oct 8 to Oct 20, New, 1 in library"
        );

        rows.push(index_row("old", "Rain", "album", Some(oct(1)), None));
        let closed = layout(&rows, &LibraryTree::default());
        assert_eq!(
            tiles(&closed)[0].a11y_label,
            "jSmovie: 13 singles, Oct 8 to Oct 20, New, 1 in library"
        );
    }

    /// R86-3-20: `set_window_closed` sets the badge and the label
    /// together.
    #[test]
    fn adr_0086_pager_windows_r20_set_window_closed() {
        let mut rows = thirteen_singles();
        rows.push(index_row("old", "Rain", "album", Some(oct(1)), None));
        let layout = layout(&rows, &LibraryTree::default());
        let mut tile = tiles(&layout)[0].clone();
        assert!(tile.window_closed);

        tile.set_window_closed(false);
        assert!(!tile.window_closed);
        assert_eq!(tile.kind_badge, "13+ Singles");
        assert!(tile.a11y_label.contains("13 or more singles"));

        tile.set_window_closed(true);
        assert!(tile.window_closed);
        assert_eq!(tile.kind_badge, "13 Singles");
        assert!(!tile.a11y_label.contains("or more"));
    }

    /// R86-2-21: the art is the anchor cover as a square, or the entity
    /// color with no cover.
    #[test]
    fn adr_0086_window_groups_r21_art_is_the_anchor_cover() {
        let entries = singles(&[oct(20), oct(19)]);
        let items = window_groups(&entries, WindowLength::DEFAULT);
        let group = groups(&items)[0];
        let cover = WindowGroupTileVm::new(group, &entries, "a", "jSmovie", Some("a.jpg"));
        assert_eq!(cover.art, CollectionArtVm::SingleCover("a.jpg".to_owned()));
        let none = WindowGroupTileVm::new(group, &entries, "a", "jSmovie", None);
        assert_eq!(
            none.art,
            CollectionArtVm::EntityColor(CollectionKind::SinglesGroup)
        );
    }

    /// R86-2-22: one day gives one date. Two years give the full dates.
    #[test]
    fn adr_0086_window_groups_r22_span_text() {
        assert_eq!(window_span_text(oct(20), oct(20) + 3_600), "Oct 20");
        let dec_28_2025 = day(1, 3) - 6 * SECONDS_PER_DAY;
        assert_eq!(
            window_span_text(dec_28_2025, day(1, 3)),
            "Dec 28, 2025 to Jan 3, 2026"
        );
    }

    /// R86-2-30 (the length part): `from_days` accepts 1 to 60.
    #[test]
    fn adr_0086_window_groups_r30_window_length_range() {
        assert_eq!(WindowLength::from_days(0), None);
        assert_eq!(WindowLength::from_days(61), None);
        assert_eq!(
            WindowLength::from_days(WindowLength::MIN_DAYS).map(WindowLength::days),
            Some(1)
        );
        assert_eq!(
            WindowLength::from_days(WindowLength::MAX_DAYS).map(WindowLength::days),
            Some(60)
        );
        assert_eq!(WindowLength::DEFAULT.days(), 14);
        assert_eq!(WindowLength::default(), WindowLength::DEFAULT);
    }

    /// Packet 003 reads `any_window_open`: a group with no older row is
    /// open.
    #[test]
    fn adr_0086_window_groups_any_window_open_reads_the_tiles() {
        let open = [jsmovie_single("a", oct(20)), jsmovie_single("b", oct(19))];
        assert!(layout(&open, &LibraryTree::default()).any_window_open());
        let closed = [
            jsmovie_single("a", oct(20)),
            jsmovie_single("b", oct(19)),
            index_row("old", "Rain", "album", Some(oct(1)), None),
        ];
        let closed = layout(&closed, &LibraryTree::default());
        assert!(!closed.any_window_open());
        assert!(tiles(&closed)[0].window_closed);
    }

    /// The key of the window of R86-2-01: Oct 20 back to Oct 6.
    fn r01_key(artist: &str) -> WindowGroupKey {
        WindowGroupKey {
            group: publisher("jsmovie"),
            anchor: ReleaseKey::FeedGuid("guid-0".to_owned()),
            anchor_date: oct(20),
            window_start: oct(20) - WindowLength::DEFAULT.seconds(),
            artist: artist.to_owned(),
            source: WindowGroupSource::NewReleases,
        }
    }

    /// R86-4-01: the members of the window of R86-2-01 are Oct 20, Oct 15
    /// and Oct 8, in that order.
    #[test]
    fn adr_0086_window_group_page_r01_window_members_newest_first() {
        let entries = singles(&[oct(8), oct(20), oct(1), oct(15)]);
        assert_eq!(window_members(&entries, &r01_key("jSmovie")), [1, 3, 0]);
    }

    /// R86-4-02: no entry of another key, no album and no single with no
    /// release date.
    #[test]
    fn adr_0086_window_group_page_r02_window_members_skip_other_entries() {
        let entries = vec![
            entry(0, Some(publisher("jsmovie")), Some(oct(20))),
            entry(1, Some(publisher("rain")), Some(oct(18))),
            entry(2, None, Some(oct(17))),
            entry(3, Some(publisher("jsmovie")), None),
            entry(4, Some(publisher("jsmovie")), Some(oct(15))),
        ];
        assert_eq!(window_members(&entries, &r01_key("jSmovie")), [0, 4]);
    }

    /// R86-4-03: the artist text is a label only. Each other field decides
    /// the equality.
    #[test]
    fn adr_0086_window_group_page_r03_key_equality_ignores_the_artist() {
        let key = r01_key("jSmovie");
        assert_eq!(key, r01_key("JSMOVIE"));
        let mut other = key.clone();
        other.anchor = ReleaseKey::FeedGuid("guid-1".to_owned());
        assert_ne!(key, other);
        let mut other = key.clone();
        other.anchor_date += 1;
        assert_ne!(key, other);
        let mut other = key.clone();
        other.window_start += 1;
        assert_ne!(key, other);
    }

    /// R86-4-04: the tile gives the page key of its group.
    #[test]
    fn adr_0086_window_group_page_r04_tile_gives_the_page_key() {
        let rows = [
            jsmovie_single("a", oct(20)),
            jsmovie_single("b", oct(15)),
            jsmovie_single("c", oct(8)),
        ];
        let layout = layout(&rows, &LibraryTree::default());
        let tile = tiles(&layout)[0];
        assert_eq!(tile.page_key.group, publisher("feed-jsmovie"));
        assert_eq!(tile.page_key.anchor, ReleaseKey::FeedGuid("a".to_owned()));
        assert_eq!(tile.page_key.anchor_date, oct(20));
        assert_eq!(
            tile.page_key.window_start,
            oct(20) - WindowLength::DEFAULT.seconds()
        );
        assert_eq!(tile.page_key.artist, "jSmovie");
        assert_eq!(
            layout.window_member_row_ids(&tile.page_key),
            ["index-feed:a", "index-feed:b", "index-feed:c"]
        );
    }

    /// R86-4-25: `group_tile_for_anchor` gives the tile of the anchor key
    /// only.
    #[test]
    fn adr_0086_window_group_page_r25_group_tile_for_anchor() {
        let rows = [jsmovie_single("a", oct(20)), jsmovie_single("b", oct(15))];
        let layout = layout(&rows, &LibraryTree::default());
        let anchor = ReleaseKey::FeedGuid("a".to_owned());
        assert_eq!(
            layout
                .group_tile_for_anchor(&anchor)
                .map(|tile| tile.open_row_id.as_str()),
            Some("index-feed:a")
        );
        assert!(layout
            .group_tile_for_anchor(&ReleaseKey::FeedGuid("b".to_owned()))
            .is_none());
    }

    /// R86-4-26: the default label of the entry is the artist of the key,
    /// and the page has release rows.
    #[test]
    fn adr_0086_window_group_page_r26_entry_label_and_list_kind() {
        use crate::view_models::detail_selection::DetailListKind;
        use crate::view_models::workspace::FrameNavigationEntry;
        let entry = FrameNavigationEntry::WindowGroup(r01_key("jSmovie"));
        assert_eq!(entry.display_label(), "jSmovie");
        assert!(entry.opens_detail_page());
        assert_eq!(
            DetailListKind::for_entry(&entry),
            Some(DetailListKind::Releases)
        );
    }
}
