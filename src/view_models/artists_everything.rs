//! The Artists list in the scope "Everything" (ADR 0084 Decisions 7 and 8,
//! ADR 0085 Decision 2, packet 016).
//!
//! This module classifies each `MusicIndex` row of `GET /v1/artist-names`,
//! gives the row of each shown `MusicIndex` artist, and merges those rows with
//! the Library rows in one name order. The merge is a pure function of the
//! Library rows and the loaded pages, so a download changes a row in place.

#![warn(clippy::pedantic)]

use std::collections::BTreeSet;

use crate::api::{ArtistNameKind, ArtistNameRow};
use crate::view_models::artist_identity::{
    ArtistNameKey, NameFeedMatch, NameFeedMatches, ARTIST_NAME_ROW_MARK, ARTIST_ROW_SEPARATOR,
};
use crate::view_models::collection_art::{CollectionArtVm, CollectionKind};
use crate::view_models::index_name_list::{watermark_shows, IndexNameListPage};
use crate::view_models::library::{
    release_row_slug_text, DestinationListRowTarget, DestinationListRowVm,
};
use crate::view_models::list_selection::RowSelectionVm;
use crate::view_models::role_words::{role_contains_artist, role_words};

/// The secondary text of a `MusicIndex` row (packet 016 decision D12).
const NOT_IN_LIBRARY: &str = "Not in Library";

/// The kind of an Artists row, in the order of Stophammer: a name row
/// comes before a feed row with the same sort key.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum ArtistRowKind {
    /// A name row (the name level).
    Name,
    /// A feed row (the feed level).
    Feed,
}

/// The order position of one Artists row (packet 016 decision D6): the
/// sort key, then the kind, then the key. The key of a feed row is its
/// feed GUID. The key of a name row is its sort key.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct ArtistOrderKey {
    sort_key: String,
    kind: ArtistRowKind,
    key: String,
}

impl ArtistOrderKey {
    /// The position of a feed row with the sort key `sort_key`.
    #[must_use]
    pub(crate) fn feed(sort_key: String, feed_guid: &str) -> Self {
        Self {
            sort_key,
            kind: ArtistRowKind::Feed,
            key: feed_guid.to_owned(),
        }
    }

    /// The position of a name row with the sort key `sort_key`.
    #[must_use]
    pub(crate) fn name(sort_key: String) -> Self {
        Self {
            key: sort_key.clone(),
            sort_key,
            kind: ArtistRowKind::Name,
        }
    }

    /// The position of a `MusicIndex` row. `MusicIndex` gives the sort key, so
    /// this position is the position of the row in its pages.
    fn index(row: &ArtistNameRow) -> Self {
        let kind = match row.kind {
            ArtistNameKind::Name => ArtistRowKind::Name,
            ArtistNameKind::Publisher | ArtistNameKind::Unknown => ArtistRowKind::Feed,
        };
        Self {
            sort_key: row.sort_name.clone(),
            kind,
            key: row.key.clone(),
        }
    }
}

/// One Library row of the Artists list and its order position.
#[derive(Clone, Debug)]
pub(crate) struct LibraryArtistRow {
    pub(crate) order: ArtistOrderKey,
    pub(crate) row: DestinationListRowVm,
}

/// The class of one feed by its link facts (ADR 0085 Decision 2). The
/// Artists list of packet 016 and the Artists tab of search (packet 029)
/// use this one rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FeedLinkClass {
    /// The agreed roles hold `artist`.
    Artist,
    /// The agreed roles are not empty and hold no `artist`. The words are
    /// the role words of [`role_words`], for example `["label"]`.
    OtherRoles(Vec<String>),
    /// The feed has no agreed role, and a confirmed link names it first.
    UnverifiedArtist,
    /// The feed has no agreed role, and no confirmed link names it first.
    /// A null count gives this class.
    NotNamedFirst,
}

impl FeedLinkClass {
    /// Classifies a feed by its agreed roles and the count of the
    /// confirmed links that name it first. A blank role is not a role.
    #[must_use]
    pub(crate) fn from_link_facts(
        agreed_roles: Option<&[String]>,
        first_named_count: Option<i64>,
    ) -> Self {
        let combined = agreed_roles.unwrap_or_default().join(",");
        let words = role_words(Some(&combined));
        if role_contains_artist(Some(&combined)) {
            Self::Artist
        } else if !words.is_empty() {
            Self::OtherRoles(words)
        } else if first_named_count.unwrap_or(0) > 0 {
            Self::UnverifiedArtist
        } else {
            Self::NotNamedFirst
        }
    }
}

/// The class of one `MusicIndex` artist name row (ADR 0085 Decision 2,
/// packet 016 decision D11).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum IndexArtistClass {
    /// A feed whose agreed roles hold `artist`. Shown.
    Artist,
    /// A feed with no agreed role that a confirmed link names first.
    /// Shown.
    UnverifiedArtist,
    /// A feed whose agreed roles hold no `artist`, for example a label.
    /// Absent.
    OtherRole,
    /// A feed with no agreed role that no confirmed link names first.
    /// Absent.
    NotNamedFirst,
    /// A name row (the name level). Shown.
    Name,
    /// A row of a kind that this app does not know. Absent.
    UnknownKind,
}

impl IndexArtistClass {
    /// Whether the Artists list shows a row of this class.
    #[must_use]
    pub(crate) const fn is_shown(self) -> bool {
        matches!(self, Self::Artist | Self::UnverifiedArtist | Self::Name)
    }
}

/// Classifies one `MusicIndex` row by the table of packet 016 "The Artists
/// list in Everything".
#[must_use]
pub(crate) fn classify_index_artist(row: &ArtistNameRow) -> IndexArtistClass {
    match row.kind {
        ArtistNameKind::Name => IndexArtistClass::Name,
        ArtistNameKind::Unknown => IndexArtistClass::UnknownKind,
        ArtistNameKind::Publisher => {
            match FeedLinkClass::from_link_facts(row.agreed_roles.as_deref(), row.first_named_count)
            {
                FeedLinkClass::Artist => IndexArtistClass::Artist,
                FeedLinkClass::OtherRoles(_) => IndexArtistClass::OtherRole,
                FeedLinkClass::UnverifiedArtist => IndexArtistClass::UnverifiedArtist,
                FeedLinkClass::NotNamedFirst => IndexArtistClass::NotNamedFirst,
            }
        }
    }
}

/// The row of one shown `MusicIndex` artist (packet 016 decisions D12 and
/// D14). No row shows a level word. A feed row shows the image of its feed
/// (ADR 0084 Decision 12). A name row shows the entity color.
#[must_use]
pub(crate) fn index_artist_row(row: &ArtistNameRow) -> DestinationListRowVm {
    let no_covers = std::iter::empty::<&str>();
    match row.kind {
        ArtistNameKind::Name => DestinationListRowVm {
            element_id: format!(
                "music-artist-row-index-name:{}",
                release_row_slug_text(&row.key)
            ),
            label: row.name.clone(),
            secondary_text: format!("{NOT_IN_LIBRARY}{ARTIST_ROW_SEPARATOR}{ARTIST_NAME_ROW_MARK}"),
            a11y_label: format!("Artist: {}, {ARTIST_NAME_ROW_MARK}", row.name),
            thumbnail_href: None,
            art: Some(CollectionArtVm::resolve(
                None,
                no_covers,
                CollectionKind::Artist,
            )),
            open: DestinationListRowTarget::ArtistName(ArtistNameKey::new(
                row.key.clone(),
                row.name.clone(),
            )),
            selection: RowSelectionVm::none(),
            release_kind: None,
            menu: None,
        },
        ArtistNameKind::Publisher | ArtistNameKind::Unknown => {
            let feed_guid = row.feed_guid.clone().unwrap_or_else(|| row.key.clone());
            DestinationListRowVm {
                element_id: format!("music-artist-row-index-publisher:{feed_guid}"),
                label: row.name.clone(),
                secondary_text: NOT_IN_LIBRARY.to_owned(),
                a11y_label: format!("Artist: {}", row.name),
                thumbnail_href: None,
                art: Some(CollectionArtVm::resolve(
                    row.image_url.as_deref(),
                    no_covers,
                    CollectionKind::Artist,
                )),
                open: DestinationListRowTarget::IndexPublisher(feed_guid),
                selection: RowSelectionVm::none(),
                release_kind: None,
                menu: None,
            }
        }
    }
}

/// The Artists rows in the scope "Everything" (packet 016 decisions D5 to
/// D8):
///
/// - While page 1 loads, the list shows no item row.
/// - A `MusicIndex` feed row and a Library feed row with the same feed GUID
///   give the Library row. A `MusicIndex` name row and a Library name row
///   with the same sort key give the Library row.
/// - The rows come in the order of [`ArtistOrderKey`].
/// - While `MusicIndex` has more pages, a row shows only at or before the
///   last loaded `MusicIndex` row.
/// - A `MusicIndex` name row whose key matches exactly one Library artist
///   feed gives no row (ADR 0084 Decision 7, "Matched by name", packet 017
///   decision D14). `matches` comes from the Library tree only.
#[must_use]
pub(crate) fn everything_rows(
    library: Vec<LibraryArtistRow>,
    page: &IndexNameListPage<ArtistNameRow>,
    matches: &NameFeedMatches,
) -> Vec<DestinationListRowVm> {
    if page.first_page_in_flight() {
        return Vec::new();
    }
    let library_keys: BTreeSet<(ArtistRowKind, &str)> = library
        .iter()
        .map(|entry| (entry.order.kind, entry.order.key.as_str()))
        .collect();
    let index_rows: Vec<LibraryArtistRow> = page
        .rows()
        .iter()
        .filter(|row| classify_index_artist(row).is_shown())
        .filter(|row| {
            row.kind != ArtistNameKind::Name
                || !matches!(matches.get(&row.key), NameFeedMatch::MatchedFeed(_))
        })
        .map(|row| LibraryArtistRow {
            order: ArtistOrderKey::index(row),
            row: index_artist_row(row),
        })
        .filter(|entry| !library_keys.contains(&(entry.order.kind, entry.order.key.as_str())))
        .collect();

    let last_loaded = page.rows().last().map(ArtistOrderKey::index);
    let mut rows: Vec<LibraryArtistRow> = library
        .into_iter()
        .chain(index_rows)
        .filter(|entry| watermark_shows(&entry.order, last_loaded.as_ref(), page.has_more()))
        .collect();
    rows.sort_by(|left, right| left.order.cmp(&right.order));
    rows.into_iter().map(|entry| entry.row).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_models::index_name_list::IndexNameListBatch;

    /// The recorded rows of `GET /v1/artist-names?include=link_facts` on
    /// 2026-10-20, with the fields that the classification reads.
    pub(super) fn recorded_rows() -> Vec<ArtistNameRow> {
        serde_json::from_value(serde_json::json!([
            {"kind": "name", "name": "$2 Holla", "sort_name": "$2 holla", "key": "$2 holla",
             "feed_guid": null, "image_url": null, "release_count": 3},
            {"kind": "publisher", "name": "Motive to Kill", "sort_name": "motive to kill",
             "key": "motive-guid", "feed_guid": "motive-guid",
             "image_url": "https://example.test/motive.jpg",
             "agreed_roles": ["artist"], "first_named_count": 1, "two_way_link_count": 1},
             {"kind": "publisher", "name": "Sir Libre Records", "sort_name": "sir libre records",
             "key": "34bf795f-f551-4db1-8877-ad19aa5f89c8",
             "feed_guid": "34bf795f-f551-4db1-8877-ad19aa5f89c8",
             "agreed_roles": ["label"], "first_named_count": 0, "two_way_link_count": 3},
            {"kind": "publisher", "name": "TJ Wong", "sort_name": "tj wong",
             "key": "a31f362c-d40b-5f56-ae2c-4ef9c548db9d",
             "feed_guid": "a31f362c-d40b-5f56-ae2c-4ef9c548db9d",
             "agreed_roles": [], "first_named_count": 0, "two_way_link_count": 0},
            {"kind": "publisher", "name": "Night Owl", "sort_name": "night owl",
             "key": "unverified-guid", "feed_guid": "unverified-guid",
             "agreed_roles": [], "first_named_count": 2}
        ]))
        .expect("the recorded rows decode")
    }

    fn loaded(rows: Vec<ArtistNameRow>, has_more: bool) -> IndexNameListPage<ArtistNameRow> {
        let mut page = IndexNameListPage::default();
        let intent = page.begin_load(false).unwrap();
        page.finish_load(
            IndexNameListBatch {
                rows,
                cursor: has_more.then(|| "next".to_owned()),
                has_more,
            },
            intent.generation,
        );
        page
    }

    fn no_match() -> NameFeedMatches {
        NameFeedMatches::default()
    }

    fn library_row(label: &str, order: ArtistOrderKey) -> LibraryArtistRow {
        LibraryArtistRow {
            order,
            row: DestinationListRowVm {
                element_id: format!("library:{label}"),
                label: label.to_owned(),
                secondary_text: "1 in Library".to_owned(),
                a11y_label: format!("Artist: {label}"),
                thumbnail_href: None,
                art: None,
                open: DestinationListRowTarget::ArtistName(ArtistNameKey::from_text(label)),
                selection: RowSelectionVm::none(),
                release_kind: None,
                menu: None,
            },
        }
    }

    /// R16-08: each row of the table of decision D11.
    #[test]
    fn adr_0084_scopes_r16_08_classification() {
        let classes: Vec<IndexArtistClass> =
            recorded_rows().iter().map(classify_index_artist).collect();
        assert_eq!(
            classes,
            [
                IndexArtistClass::Name,
                IndexArtistClass::Artist,
                IndexArtistClass::OtherRole,
                IndexArtistClass::NotNamedFirst,
                IndexArtistClass::UnverifiedArtist,
            ]
        );
        let null_count = ArtistNameRow {
            kind: ArtistNameKind::Publisher,
            first_named_count: None,
            ..ArtistNameRow::default()
        };
        assert_eq!(
            classify_index_artist(&null_count),
            IndexArtistClass::NotNamedFirst
        );

        let rows = everything_rows(Vec::new(), &loaded(recorded_rows(), false), &no_match());
        let labels: Vec<&str> = rows.iter().map(|row| row.label.as_str()).collect();
        assert_eq!(labels, ["$2 Holla", "Motive to Kill", "Night Owl"]);
        for row in &rows {
            for word in ["Artist,", "Unverified", "publisher", "Publisher"] {
                assert!(
                    !row.secondary_text.contains(word),
                    "no level word: {}",
                    row.secondary_text
                );
            }
            assert!(!row.a11y_label.contains("Unverified"));
        }
    }

    /// R16-09: one row for a Library feed row and a `MusicIndex` row of the
    /// same GUID, and one row for name rows of the same sort key. The
    /// order follows decision D6, and the watermark follows decision D7.
    #[test]
    fn adr_0084_scopes_r16_09_merge_order_and_watermark() {
        let library = vec![
            library_row(
                "Motive to Kill (Library)",
                ArtistOrderKey::feed("motive to kill".to_owned(), "motive-guid"),
            ),
            library_row("$2 HOLLA", ArtistOrderKey::name("$2 holla".to_owned())),
            library_row("Zed", ArtistOrderKey::name("zed".to_owned())),
            library_row(
                "Motive to Kill",
                ArtistOrderKey::name("motive to kill".to_owned()),
            ),
        ];

        let all = everything_rows(
            library.clone(),
            &loaded(recorded_rows(), false),
            &no_match(),
        );
        let labels: Vec<&str> = all.iter().map(|row| row.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "$2 HOLLA",
                "Motive to Kill",
                "Motive to Kill (Library)",
                "Night Owl",
                "Zed"
            ],
            "the Library rows win, and a name row comes before a feed row of the same key"
        );

        let partial = everything_rows(
            library,
            &loaded(recorded_rows()[..2].to_vec(), true),
            &no_match(),
        );
        let labels: Vec<&str> = partial.iter().map(|row| row.label.as_str()).collect();
        assert_eq!(
            labels,
            ["$2 HOLLA", "Motive to Kill", "Motive to Kill (Library)"],
            "a row after the last loaded MusicIndex row waits for its page"
        );
    }

    /// R16-10: the text and the target of the `MusicIndex` rows (decisions
    /// D12 and D14).
    #[test]
    fn adr_0084_scopes_r16_10_index_row_text_and_targets() {
        let rows = recorded_rows();
        let name = index_artist_row(&rows[0]);
        assert_eq!(name.secondary_text, "Not in Library \u{b7} by name");
        assert_eq!(
            name.open,
            DestinationListRowTarget::ArtistName(ArtistNameKey::new("$2 holla", "$2 Holla"))
        );
        assert_eq!(
            name.art,
            Some(CollectionArtVm::EntityColor(CollectionKind::Artist))
        );

        let feed = index_artist_row(&rows[1]);
        assert_eq!(feed.secondary_text, "Not in Library");
        assert_eq!(
            feed.open,
            DestinationListRowTarget::IndexPublisher("motive-guid".to_owned())
        );
        assert!(matches!(
            feed.art,
            Some(CollectionArtVm::ProvidedImage { ref url, .. }) if url == "https://example.test/motive.jpg"
        ));
    }

    /// R16-12: while page 1 loads, the list shows no item row. After an
    /// error on page 1, it shows the Library rows.
    #[test]
    fn adr_0084_scopes_r16_12_first_page_states() {
        let library = vec![library_row("Zed", ArtistOrderKey::name("zed".to_owned()))];
        let mut page = IndexNameListPage::<ArtistNameRow>::default();
        let intent = page.begin_load(false).unwrap();
        assert!(everything_rows(library.clone(), &page, &no_match()).is_empty());

        page.fail_load("connection refused", intent.generation);
        let rows = everything_rows(library, &page, &no_match());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].label, "Zed");
    }

    /// R29-05 (ADR 0084 packet 029): the class of each row of the table of
    /// "One classification for a feed", and of the recorded rows of
    /// `GET /v1/search?type=publisher&include=link_facts` of 2026-10-21.
    #[test]
    fn adr_0084_search_link_facts_r29_05_feed_link_class() {
        type Case = (
            &'static str,
            Option<Vec<String>>,
            Option<i64>,
            FeedLinkClass,
        );

        fn roles(words: &[&str]) -> Vec<String> {
            words.iter().map(|word| (*word).to_owned()).collect()
        }
        let cases: [Case; 11] = [
            (
                "Sir Libre Records",
                Some(roles(&["label"])),
                Some(0),
                FeedLinkClass::OtherRoles(roles(&["label"])),
            ),
            (
                "Kolomona Myer - AKA Sir Libre",
                Some(Vec::new()),
                Some(1),
                FeedLinkClass::UnverifiedArtist,
            ),
            (
                "Haleen fe80a515",
                Some(Vec::new()),
                Some(38),
                FeedLinkClass::UnverifiedArtist,
            ),
            (
                "Haleen 64569161",
                Some(Vec::new()),
                Some(56),
                FeedLinkClass::UnverifiedArtist,
            ),
            (
                "Paul Birken",
                Some(Vec::new()),
                Some(143),
                FeedLinkClass::UnverifiedArtist,
            ),
            (
                "HeyCitizen",
                Some(roles(&["artist", "author", "host", "label", "producer"])),
                Some(2),
                FeedLinkClass::Artist,
            ),
            (
                "Official DETOX Music",
                Some(Vec::new()),
                Some(30),
                FeedLinkClass::UnverifiedArtist,
            ),
            (
                "TJ Wong",
                Some(Vec::new()),
                Some(0),
                FeedLinkClass::NotNamedFirst,
            ),
            (
                "a null count",
                Some(Vec::new()),
                None,
                FeedLinkClass::NotNamedFirst,
            ),
            (
                "only a blank role, named first",
                Some(roles(&[" "])),
                Some(3),
                FeedLinkClass::UnverifiedArtist,
            ),
            (
                "two other roles",
                Some(roles(&["producer", "label"])),
                None,
                FeedLinkClass::OtherRoles(roles(&["label", "producer"])),
            ),
        ];
        for (name, agreed_roles, first_named_count, expected) in cases {
            assert_eq!(
                FeedLinkClass::from_link_facts(agreed_roles.as_deref(), first_named_count),
                expected,
                "{name}"
            );
        }
        assert_eq!(
            FeedLinkClass::from_link_facts(None, None),
            FeedLinkClass::NotNamedFirst
        );
        assert_eq!(
            FeedLinkClass::from_link_facts(Some(&roles(&["Artist"])), Some(0)),
            FeedLinkClass::Artist,
            "the artist role is read without case"
        );
    }
}
