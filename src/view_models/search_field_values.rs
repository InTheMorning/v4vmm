//! The values of `artist:`, `release:` and `playlist:` in the field list
//! (ADR 0087 Decision 3, packet 005).
//!
//! The Library values come from the Library tree and the playlists in
//! memory. The `MusicIndex` names come from one `GET /v1/artist-names?from=`
//! request after each pause (ADR 0087 operator answer 1 of 2026-10-23).
//! This module gives the sorted Library values, the match of a typed
//! prefix, the key of the one request and the state of that request.
//!
//! Each Library text and each `MusicIndex` name is untrusted input. A text
//! with a control character, a bidirectional control character or more
//! than [`FIELD_VALUE_MAX_CHARS`] characters gives no value. A usable text
//! stays exact.
//!
//! The module reads the field syntax only through `search_query`. It has
//! no GPUI type, no SQL and no I/O.

#![warn(clippy::pedantic)]

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};

use crate::api::ArtistNameRow;
use crate::db;
use crate::view_models::artists_everything::classify_index_artist;
use crate::view_models::index_name_list::name_sort_key;
use crate::view_models::library::LibraryTree;
use crate::view_models::search_query::{
    cursor_term, unquote_field_value_prefix, CursorPartial, QueryFieldName, QueryScope, SearchQuery,
};

/// The most Library rows that one field shows.
pub(crate) const LIBRARY_VALUE_ROW_LIMIT: usize = 8;

/// The most `MusicIndex` rows that the `artist:` field shows.
pub(crate) const INDEX_VALUE_ROW_LIMIT: usize = 8;

/// The most characters of a usable value text.
pub(crate) const FIELD_VALUE_MAX_CHARS: usize = 200;

/// The source of one value row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FieldValueSource {
    /// The Library holds the value.
    Library,
    /// `MusicIndex` gave the value.
    Index,
}

impl FieldValueSource {
    /// The word of the source mark.
    #[must_use]
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Library => "Library",
            Self::Index => "MusicIndex",
        }
    }
}

/// One Library value and its sort key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FieldValueEntry {
    /// The value text, as the Library holds it.
    pub(crate) text: String,
    /// `name_sort_key` of the text.
    pub(crate) key: String,
}

/// The distinct Library values of `artist:`, `release:` and `playlist:`,
/// each list in key order.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct LibraryFieldValues {
    artists: Vec<FieldValueEntry>,
    releases: Vec<FieldValueEntry>,
    playlists: Vec<FieldValueEntry>,
}

/// The Library values with no entry. Only the tests use it.
#[cfg(test)]
const NO_LIBRARY_VALUES: LibraryFieldValues = LibraryFieldValues {
    artists: Vec::new(),
    releases: Vec::new(),
    playlists: Vec::new(),
};

impl LibraryFieldValues {
    /// Reads the values from the Library tree and the playlists.
    ///
    /// The artists come from `artist_name` and `album_artist_name` of each
    /// track. The releases come from `album_title` and `feed_title` of each
    /// track. The playlists come from the name of each playlist. These are
    /// the columns that the Library search matches (ADR 0087 packet 002).
    #[must_use]
    pub(crate) fn from_library(tree: &LibraryTree, playlists: &[db::Playlist]) -> Self {
        let mut artists: BTreeSet<&str> = BTreeSet::new();
        let mut releases: BTreeSet<&str> = BTreeSet::new();
        let tracks = tree
            .artists
            .iter()
            .flat_map(|artist| artist.albums.iter())
            .flat_map(|album| album.tracks.iter());
        for track in tracks {
            artists.extend(track.artist_name.as_deref());
            artists.extend(track.album_artist_name.as_deref());
            releases.extend(track.album_title.as_deref());
            releases.extend(track.feed_title.as_deref());
        }
        let artists: Vec<&str> = artists.into_iter().collect();
        let releases: Vec<&str> = releases.into_iter().collect();
        let playlists: Vec<&str> = playlists
            .iter()
            .map(|playlist| playlist.name.as_str())
            .collect();
        Self::from_texts(&artists, &releases, &playlists)
    }

    /// Makes the three lists from texts. Each list keeps one entry for each
    /// key, with the text that comes first in `String` order. A text that
    /// is not usable gives no entry.
    #[must_use]
    pub(crate) fn from_texts(artists: &[&str], releases: &[&str], playlists: &[&str]) -> Self {
        Self {
            artists: entries(artists),
            releases: entries(releases),
            playlists: entries(playlists),
        }
    }

    /// The entries of `field` that match the typed `prefix`.
    ///
    /// The match key is the sort key of the prefix with its quote and its
    /// escapes read. An entry matches when its key holds the match key. The
    /// entries whose key starts with the match key come first. Each group
    /// is in key order. The result has at most [`LIBRARY_VALUE_ROW_LIMIT`]
    /// entries. A field other than `artist:`, `release:` and `playlist:`
    /// gives no entry.
    #[must_use]
    pub(crate) fn matching(&self, field: QueryFieldName, prefix: &str) -> Vec<&FieldValueEntry> {
        let list = match field {
            QueryFieldName::Artist => &self.artists,
            QueryFieldName::Release => &self.releases,
            QueryFieldName::Playlist => &self.playlists,
            QueryFieldName::Title
            | QueryFieldName::Kind
            | QueryFieldName::In
            | QueryFieldName::After
            | QueryFieldName::Before
            | QueryFieldName::Duration
            | QueryFieldName::Is
            | QueryFieldName::Has => return Vec::new(),
        };
        let match_key = name_sort_key(&unquote_field_value_prefix(prefix));
        let starts = list
            .iter()
            .filter(|entry| entry.key.starts_with(&match_key));
        let inside = list
            .iter()
            .filter(|entry| !entry.key.starts_with(&match_key) && entry.key.contains(&match_key));
        starts.chain(inside).take(LIBRARY_VALUE_ROW_LIMIT).collect()
    }
}

/// The distinct usable entries of `texts`, in key order.
fn entries(texts: &[&str]) -> Vec<FieldValueEntry> {
    let mut by_key: BTreeMap<String, &str> = BTreeMap::new();
    for text in texts
        .iter()
        .copied()
        .filter(|text| value_text_is_usable(text))
    {
        match by_key.entry(name_sort_key(text)) {
            Entry::Vacant(slot) => {
                slot.insert(text);
            }
            Entry::Occupied(mut slot) => {
                if text < *slot.get() {
                    slot.insert(text);
                }
            }
        }
    }
    by_key
        .into_iter()
        .map(|(key, text)| FieldValueEntry {
            text: text.to_owned(),
            key,
        })
        .collect()
}

/// The state of the `MusicIndex` name request of the `artist:` field.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) enum IndexArtistValues {
    /// No request applies to the text.
    #[default]
    NotRequested,
    /// The pause or the request for `key` runs.
    Loading { key: String },
    /// The usable names of the answer for `key`.
    Loaded { key: String, names: Vec<String> },
    /// The request for `key` failed.
    Failed { key: String },
}

impl IndexArtistValues {
    /// Gives `Loaded` with the usable names of `rows` for `key`.
    ///
    /// A row gives a name only when the Artists list shows its class (ADR
    /// 0085 Decision 2), its sort key starts with `key`, and its name is
    /// usable. A name row and a feed row each give their name text. The
    /// result keeps one name for each sort key, in the order of the answer,
    /// and at most [`INDEX_VALUE_ROW_LIMIT`] names.
    #[must_use]
    pub(crate) fn loaded(key: String, rows: &[ArtistNameRow]) -> Self {
        let mut seen = BTreeSet::new();
        let names = rows
            .iter()
            .filter(|row| classify_index_artist(row).is_shown())
            .filter(|row| value_text_is_usable(&row.name))
            .filter(|row| {
                let row_key = name_sort_key(&row.name);
                row_key.starts_with(&key) && seen.insert(row_key)
            })
            .take(INDEX_VALUE_ROW_LIMIT)
            .map(|row| row.name.clone())
            .collect();
        Self::Loaded { key, names }
    }

    /// The key of the request, or `None` for `NotRequested`.
    #[must_use]
    pub(crate) fn key(&self) -> Option<&str> {
        match self {
            Self::NotRequested => None,
            Self::Loading { key } | Self::Loaded { key, .. } | Self::Failed { key } => Some(key),
        }
    }
}

/// The two sources of the value rows.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FieldValueRows<'a> {
    /// The Library values.
    pub(crate) library: &'a LibraryFieldValues,
    /// The state of the `MusicIndex` name request.
    pub(crate) index: &'a IndexArtistValues,
}

/// The empty sources have no production caller. The tests of packet 004
/// call them.
#[cfg(test)]
impl FieldValueRows<'static> {
    /// No Library value and no request.
    pub(crate) const EMPTY: Self = Self {
        library: &NO_LIBRARY_VALUES,
        index: &IndexArtistValues::NotRequested,
    };
}

/// The key of the one `MusicIndex` name request for the text and the
/// cursor.
///
/// The result is `Some` only when the cursor is in an `artist:` value, the
/// sort key of the typed prefix is not empty, and the query can show Index
/// rows. A Library-only query or a conflict gives `None` (ADR 0087 packet
/// 005 decision D8).
#[must_use]
pub(crate) fn artist_value_request_key(text: &str, cursor: usize) -> Option<String> {
    let CursorPartial::FieldValue {
        field: QueryFieldName::Artist,
        prefix,
        ..
    } = cursor_term(text, cursor).partial
    else {
        return None;
    };
    let key = name_sort_key(&unquote_field_value_prefix(&prefix));
    if key.is_empty() {
        return None;
    }
    match SearchQuery::parse(text).scope() {
        QueryScope::Everything | QueryScope::IndexOnly => Some(key),
        QueryScope::LibraryOnly | QueryScope::Conflict => None,
    }
}

/// `true` when a Library text or a `MusicIndex` name can be one value row.
///
/// An empty text, a text of whitespace only, a text with a control
/// character or a bidirectional control character, and a text with more
/// than [`FIELD_VALUE_MAX_CHARS`] characters are not usable.
fn value_text_is_usable(text: &str) -> bool {
    !text.trim().is_empty()
        && text.chars().count() <= FIELD_VALUE_MAX_CHARS
        && !text
            .chars()
            .any(|ch| ch.is_control() || is_bidirectional_control(ch))
}

/// `true` for U+202A to U+202E and U+2066 to U+2069.
fn is_bidirectional_control(ch: char) -> bool {
    matches!(ch, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ArtistNameKind;
    use crate::db::TrackRow;
    use crate::view_models::library::{AlbumNode, ArtistNode};
    use crate::views::{FeedMetadataFacts, LocalIdentityFacts};

    fn library_h() -> LibraryFieldValues {
        LibraryFieldValues::from_texts(
            &["Haleen", "Haleen Moss", "Shaleen", "jSmovie"],
            &["Aged Friends"],
            &["Friday show", "Monday mix"],
        )
    }

    fn texts(entries: &[&FieldValueEntry]) -> Vec<String> {
        entries.iter().map(|entry| entry.text.clone()).collect()
    }

    fn name_row(name: &str) -> ArtistNameRow {
        ArtistNameRow {
            kind: ArtistNameKind::Name,
            name: name.to_owned(),
            ..ArtistNameRow::default()
        }
    }

    fn feed_row(name: &str, roles: Option<&[&str]>, first_named: i64) -> ArtistNameRow {
        ArtistNameRow {
            kind: ArtistNameKind::Publisher,
            name: name.to_owned(),
            agreed_roles: roles.map(|roles| roles.iter().map(|&role| role.to_owned()).collect()),
            first_named_count: Some(first_named),
            ..ArtistNameRow::default()
        }
    }

    fn loaded_names(values: &IndexArtistValues) -> Vec<String> {
        match values {
            IndexArtistValues::Loaded { names, .. } => names.clone(),
            other => panic!("{other:?} is not Loaded"),
        }
    }

    /// R87-5-01: the prefix matches come first, then the contains matches.
    /// Another field gives no entry. A list gives at most 8 entries.
    #[test]
    fn adr_0087_field_values_r87_5_01_matching_puts_prefix_matches_first() {
        let values = library_h();
        assert_eq!(
            texts(&values.matching(QueryFieldName::Artist, "hal")),
            vec!["Haleen", "Haleen Moss", "Shaleen"]
        );
        assert!(values.matching(QueryFieldName::Title, "hal").is_empty());
        assert_eq!(values.matching(QueryFieldName::Artist, "").len(), 4);

        let many: Vec<String> = (0..12).map(|index| format!("Artist {index:02}")).collect();
        let many: Vec<&str> = many.iter().map(String::as_str).collect();
        let values = LibraryFieldValues::from_texts(&many, &[], &[]);
        assert_eq!(
            values.matching(QueryFieldName::Artist, "art").len(),
            LIBRARY_VALUE_ROW_LIMIT
        );
    }

    /// R87-5-02: one entry for each key, with the first text in `String`
    /// order. `from_library` reads the four track columns and the playlist
    /// names.
    #[test]
    fn adr_0087_field_values_r87_5_02_entries_are_distinct_and_read_the_library() {
        let values = LibraryFieldValues::from_texts(&["Haleen", "HALEEN"], &[], &[]);
        assert_eq!(
            texts(&values.matching(QueryFieldName::Artist, "")),
            vec!["HALEEN"]
        );

        let track = TrackRow {
            artist_name: Some("Track Artist".to_owned()),
            album_artist_name: Some("Album Artist".to_owned()),
            album_title: Some("Album Title".to_owned()),
            feed_title: Some("Feed Title".to_owned()),
            ..TrackRow::default()
        };
        let tree = LibraryTree {
            artists: vec![ArtistNode {
                name: "Tree Artist".to_owned(),
                albums: vec![AlbumNode {
                    name: "Tree Album".to_owned(),
                    feed_id: None,
                    feed_guid: None,
                    feed_url: None,
                    language: None,
                    description: None,
                    image_href: None,
                    named_feeds: Vec::new(),
                    identity_facts: LocalIdentityFacts::default(),
                    metadata_facts: Box::new(FeedMetadataFacts::default()),
                    stored_values: None,
                    tracks: vec![track],
                }],
            }],
        };
        let playlists = [db::Playlist {
            id: 1,
            name: "Friday show".to_owned(),
            description: None,
            track_count: 0,
            created_at: 0,
            updated_at: 0,
        }];
        let values = LibraryFieldValues::from_library(&tree, &playlists);
        assert_eq!(
            texts(&values.matching(QueryFieldName::Artist, "")),
            vec!["Album Artist", "Track Artist"]
        );
        assert_eq!(
            texts(&values.matching(QueryFieldName::Release, "")),
            vec!["Album Title", "Feed Title"]
        );
        assert_eq!(
            texts(&values.matching(QueryFieldName::Playlist, "")),
            vec!["Friday show"]
        );
    }

    /// R87-5-05: `loaded` keeps the shown classes with the key prefix, one
    /// name for each key, and at most 8 names.
    #[test]
    fn adr_0087_field_values_r87_5_05_loaded_keeps_shown_artist_names() {
        let unknown = ArtistNameRow {
            kind: ArtistNameKind::Unknown,
            name: "Hal Unknown".to_owned(),
            ..ArtistNameRow::default()
        };
        let rows = [
            name_row("Haleen"),
            feed_row("Halo Child", Some(&["artist"]), 1),
            feed_row("Hal Unverified", None, 2),
            unknown,
            feed_row("Hal Label", Some(&["label"]), 1),
            name_row("Hamster"),
            feed_row("Haleen", Some(&["artist"]), 1),
        ];
        let values = IndexArtistValues::loaded("hal".to_owned(), &rows);
        assert_eq!(values.key(), Some("hal"));
        assert_eq!(
            loaded_names(&values),
            vec!["Haleen", "Halo Child", "Hal Unverified"]
        );

        let many: Vec<ArtistNameRow> = (0..12)
            .map(|index| name_row(&format!("Hal {index:02}")))
            .collect();
        let values = IndexArtistValues::loaded("hal".to_owned(), &many);
        assert_eq!(loaded_names(&values).len(), INDEX_VALUE_ROW_LIMIT);
        assert_eq!(IndexArtistValues::NotRequested.key(), None);
        assert_eq!(
            IndexArtistValues::Failed {
                key: "x".to_owned()
            }
            .key(),
            Some("x")
        );
    }

    /// R87-5-09: the request key needs an `artist:` value with text, and a
    /// scope that can show Index rows.
    #[test]
    fn adr_0087_field_values_r87_5_09_request_key_rules() {
        let key = |text: &str| artist_value_request_key(text, text.len());
        assert_eq!(
            artist_value_request_key("artist:Hal", 10).as_deref(),
            Some("hal")
        );
        assert_eq!(key(r#"artist:"Haleen  M"#).as_deref(), Some("haleen m"));
        assert_eq!(key("artist:"), None);
        assert_eq!(key("title:hal"), None);
        assert_eq!(key("in:library artist:hal"), None);
        assert_eq!(key("playlist:x artist:hal"), None);
        assert_eq!(key("in:index artist:hal").as_deref(), Some("hal"));
    }

    /// R87-5-14: a control character, a bidirectional control character
    /// or more than 200 characters gives no entry and no name. Markup stays
    /// text.
    #[test]
    fn adr_0087_field_values_r87_5_14_untrusted_text_checks() {
        let long_ok = "a".repeat(FIELD_VALUE_MAX_CHARS);
        let long_bad = "a".repeat(FIELD_VALUE_MAX_CHARS + 1);
        for bad in [
            "a\u{000A}b",
            "a\u{0009}b",
            "a\u{202E}b",
            long_bad.as_str(),
            " ",
        ] {
            let values = LibraryFieldValues::from_texts(&[bad], &[], &[]);
            assert!(
                values.matching(QueryFieldName::Artist, "").is_empty(),
                "{bad:?}"
            );
            let key = name_sort_key(bad).chars().next().map(String::from);
            let key = key.unwrap_or_default();
            let index = IndexArtistValues::loaded(key, &[name_row(bad)]);
            assert!(loaded_names(&index).is_empty(), "{bad:?}");
        }
        let values = LibraryFieldValues::from_texts(&[long_ok.as_str(), "<b>x</b>"], &[], &[]);
        assert_eq!(
            texts(&values.matching(QueryFieldName::Artist, "")),
            vec!["<b>x</b>".to_owned(), long_ok.clone()]
        );
        let index = IndexArtistValues::loaded("a".to_owned(), &[name_row(&long_ok)]);
        assert_eq!(loaded_names(&index), vec![long_ok]);
    }
}
