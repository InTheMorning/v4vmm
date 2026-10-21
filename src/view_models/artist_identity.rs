//! The artist identity of a release (ADR 0084 Decision 7, ADR 0085
//! Decision 6).
//!
//! A release with a confirmed or unverified artist feed has the feed level.
//! Each other release has the name level, keyed by the sort key of its
//! artist text (ADR 0084 Decision 7, amendment of 2026-10-20).
//!
//! This module also gives the match of a name to the artist feeds of the
//! Library (the rule "Matched by name"). The Artists rows, the feed page
//! section, the name level page and the search rows all read this one
//! match.

#![warn(clippy::pedantic)]

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

use crate::view_models::index_name_list::name_sort_key;
use crate::view_models::named_feeds::{select_artist_feed, ArtistFeed, NamedFeed};

/// The mark of a name row of the Artists list and of the breadcrumb item of
/// a name level page (operator decision of 2026-10-18). It tells a name
/// row from a feed row with the same label. It names the source of the
/// group, not a confirmation level (ADR 0085 Decision 2).
pub(crate) const ARTIST_NAME_ROW_MARK: &str = "by name";
/// The separator between the parts of an Artists list row text.
pub(crate) const ARTIST_ROW_SEPARATOR: &str = " \u{b7} ";
/// The artist text of a Library track that states no artist. The key of
/// this text never matches a feed, and the name level page sends no
/// `MusicIndex` request for it.
pub(crate) const UNKNOWN_ARTIST_TEXT: &str = "Unknown Artist";

/// The artist identity of a release (ADR 0084 Decision 7).
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ArtistIdentity {
    /// The artist feed that the release names, by its feed GUID.
    Publisher(String),
    /// The sort key of the artist text of a release that has no artist
    /// feed.
    Name(String),
}

impl ArtistIdentity {
    /// The feed level when `named_feeds` gives a confirmed or unverified
    /// artist feed (ADR 0085 Decision 6). Otherwise the name level of the
    /// sort key of `artist_text`.
    ///
    /// Two texts that differ only in case or in white space give one name
    /// identity (ADR 0084 Decision 7, packet 017 decision D3).
    #[must_use]
    pub(crate) fn resolve(named_feeds: &[NamedFeed], artist_text: &str) -> Self {
        match select_artist_feed(named_feeds) {
            ArtistFeed::Confirmed(index) | ArtistFeed::Unverified(index) => {
                Self::Publisher(named_feeds[index].feed_guid.clone())
            }
            ArtistFeed::None => Self::Name(name_sort_key(artist_text)),
        }
    }
}

/// The key of one name level page, with the label that the operator
/// clicked (ADR 0084 Decision 7). Equality, order and hash use the key
/// only. Thus "Haleen" and "haleen" open one page, and the breadcrumb holds
/// that page one time.
#[derive(Clone, Debug)]
pub(crate) struct ArtistNameKey {
    /// The sort key of the name.
    pub(crate) key: String,
    /// The text that the operator clicked.
    pub(crate) label: String,
}

impl ArtistNameKey {
    /// The key of `text`, with `text` as its label.
    #[must_use]
    pub(crate) fn from_text(text: &str) -> Self {
        Self {
            key: name_sort_key(text),
            label: text.trim().to_owned(),
        }
    }

    /// The key `key`, with `label` as its label.
    #[must_use]
    pub(crate) fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
        }
    }
}

impl PartialEq for ArtistNameKey {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for ArtistNameKey {}

impl Hash for ArtistNameKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl PartialOrd for ArtistNameKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ArtistNameKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}

/// The label of a name identity (Stophammer ADR 0070 section 3): the raw
/// artist text that the most members state. With a tie, it is the first
/// raw text in code point order. `None` when `texts` is empty.
#[must_use]
pub(crate) fn name_label<'a>(texts: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for text in texts {
        *counts.entry(text).or_default() += 1;
    }
    let mut best: Option<(&str, usize)> = None;
    for (text, count) in counts {
        if best.is_none_or(|(_, best_count)| count > best_count) {
            best = Some((text, count));
        }
    }
    best.map(|(text, _)| text.to_owned())
}

/// The match of one name key to the artist feeds of the Library (ADR 0084
/// Decision 7, amendment of 2026-10-20).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum NameFeedMatch {
    /// The key is in the key set of exactly one feed, by its feed GUID.
    MatchedFeed(String),
    /// The key is in the key sets of two or more feeds.
    Ambiguous,
    /// The key set of no feed holds the key.
    NoFeed,
}

/// One artist feed of the Library, as the input of [`name_feed_matches`].
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct NameFeedEntry {
    /// The feed GUID.
    pub(crate) feed_guid: String,
    /// The feed title, when the Library knows it.
    pub(crate) title: Option<String>,
    /// The key of the feed title and the key of the artist text of each
    /// Library release of the feed.
    pub(crate) keys: BTreeSet<String>,
}

/// The page that an artist name opens (ADR 0084 Decision 7).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ArtistNameDestination {
    /// The feed page of the one feed that the name matches, by feed GUID.
    FeedPage(String),
    /// The name level page of the name.
    NameLevel(ArtistNameKey),
}

/// The match of each name key (ADR 0084 Decision 7). [`name_feed_matches`]
/// gives this value.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct NameFeedMatches {
    feeds_by_key: BTreeMap<String, BTreeSet<String>>,
    titles: BTreeMap<String, String>,
}

impl NameFeedMatches {
    /// The match of `key`. A key with no entry, and the key of
    /// [`UNKNOWN_ARTIST_TEXT`], give [`NameFeedMatch::NoFeed`].
    #[must_use]
    pub(crate) fn get(&self, key: &str) -> NameFeedMatch {
        if key == name_sort_key(UNKNOWN_ARTIST_TEXT) {
            return NameFeedMatch::NoFeed;
        }
        let Some(feeds) = self.feeds_by_key.get(key) else {
            return NameFeedMatch::NoFeed;
        };
        let mut guids = feeds.iter();
        match (guids.next(), guids.next()) {
            (Some(guid), None) => NameFeedMatch::MatchedFeed(guid.clone()),
            (Some(_), Some(_)) => NameFeedMatch::Ambiguous,
            (None, _) => NameFeedMatch::NoFeed,
        }
    }

    /// Each key whose match is `MatchedFeed(feed_guid)`, in key order.
    #[must_use]
    pub(crate) fn matched_keys(&self, feed_guid: &str) -> Vec<String> {
        self.feeds_by_key
            .keys()
            .filter(|key| self.get(key) == NameFeedMatch::MatchedFeed(feed_guid.to_owned()))
            .cloned()
            .collect()
    }

    /// The title of a feed of the input. With no known title, it is the
    /// feed GUID.
    #[must_use]
    pub(crate) fn feed_title(&self, feed_guid: &str) -> String {
        self.titles
            .get(feed_guid)
            .cloned()
            .unwrap_or_else(|| feed_guid.to_owned())
    }

    /// The page that the name `key` opens: the feed page for
    /// `MatchedFeed`, else the name level page.
    #[must_use]
    pub(crate) fn destination(&self, key: ArtistNameKey) -> ArtistNameDestination {
        match self.get(&key.key) {
            NameFeedMatch::MatchedFeed(feed_guid) => ArtistNameDestination::FeedPage(feed_guid),
            NameFeedMatch::Ambiguous | NameFeedMatch::NoFeed => {
                ArtistNameDestination::NameLevel(key)
            }
        }
    }
}

/// The match of each key of `feeds` (ADR 0084 Decision 7, packet 017
/// decision D14). A key in the key set of one feed gives `MatchedFeed`. A
/// key in the key sets of two or more feeds gives `Ambiguous`.
#[must_use]
pub(crate) fn name_feed_matches(feeds: impl IntoIterator<Item = NameFeedEntry>) -> NameFeedMatches {
    let mut matches = NameFeedMatches::default();
    for feed in feeds {
        for key in feed.keys {
            matches
                .feeds_by_key
                .entry(key)
                .or_default()
                .insert(feed.feed_guid.clone());
        }
        if let Some(title) = feed.title {
            matches.titles.entry(feed.feed_guid).or_insert(title);
        }
    }
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(role: Option<&str>, role_agreement: Option<&str>, confirmed: bool) -> NamedFeed {
        NamedFeed {
            feed_guid: "pub-guid".to_owned(),
            title: None,
            position: Some(0),
            role: role.map(str::to_owned),
            role_agreement: role_agreement.map(str::to_owned),
            feed_rel: None,
            album_rel: None,
            confirmed,
            image_url: None,
        }
    }

    /// R15-01, carried by ADR 0085 Decision 6: a confirmed artist feed
    /// keeps the feed level, whether it is agreed or unverified. No named
    /// feed, or an agreed role without artist, gives the name level.
    #[test]
    fn adr_0084_singles_group_r15_01_resolve_follows_decision_7() {
        let publisher = ArtistIdentity::Publisher("pub-guid".to_owned());
        let name = ArtistIdentity::Name("haleen".to_owned());
        let resolve = |feeds: &[NamedFeed]| ArtistIdentity::resolve(feeds, "Haleen");

        assert_eq!(resolve(&[feed(Some("label"), None, true)]), publisher);
        assert_eq!(
            resolve(&[feed(Some("label"), Some("one_side"), true)]),
            publisher
        );
        assert_eq!(
            resolve(&[feed(Some("label"), Some("conflict"), true)]),
            publisher
        );
        assert_eq!(
            resolve(&[feed(Some("artist, producer"), Some("both"), true)]),
            publisher
        );
        assert_eq!(
            resolve(&[feed(Some("Producer , ARTIST"), Some("both"), true)]),
            publisher
        );
        assert_eq!(
            resolve(&[feed(Some("label"), Some("both"), true)]),
            name,
            "an agreed role without artist gives the name level"
        );
        assert_eq!(
            resolve(&[feed(None, Some("both"), true)]),
            name,
            "a null role with an agreed role_agreement gives the name level"
        );
        assert_eq!(resolve(&[]), name, "no named feed gives the name level");
        assert_eq!(
            resolve(&[feed(None, None, false)]),
            name,
            "an unconfirmed link gives the name level"
        );
    }

    /// R17-02 (replaces the case rule of R15-01): two texts that differ
    /// only in case or in white space give one name identity. A name
    /// identity and a feed identity with the same text stay two
    /// identities.
    #[test]
    fn adr_0084_name_level_r17_02_name_identity_uses_the_key() {
        assert_eq!(
            ArtistIdentity::resolve(&[], "Haleen"),
            ArtistIdentity::resolve(&[], " haleen ")
        );
        assert_ne!(
            ArtistIdentity::Publisher("haleen".to_owned()),
            ArtistIdentity::Name("haleen".to_owned())
        );
    }

    /// R17-01: the name key is the Stophammer sort key.
    #[test]
    fn adr_0084_name_level_r17_01_name_key() {
        assert_eq!(name_sort_key("the  best\tsong "), "the best song");
        assert_eq!(name_sort_key("\u{c9}lan"), "\u{e9}lan");
        assert_eq!(name_sort_key("  Jimmy   V "), "jimmy v");
        assert_eq!(name_sort_key("The Beatles"), "the beatles");
    }

    /// R17-05: the label is the raw text that the most members state. A
    /// tie gives the first raw text in code point order.
    #[test]
    fn adr_0084_name_level_r17_05_label_rule() {
        assert_eq!(
            name_label(["haleen", "Haleen", "Haleen"]),
            Some("Haleen".to_owned())
        );
        assert_eq!(
            name_label(["haleen", "Haleen"]),
            Some("Haleen".to_owned()),
            "'H' comes before 'h' in code point order"
        );
        assert_eq!(name_label(std::iter::empty()), None);
    }

    /// R17-13: two keys with one key text and two labels are equal.
    #[test]
    fn adr_0084_name_level_r17_13_key_equality_uses_the_key() {
        let first = ArtistNameKey::new("haleen", "Haleen");
        let second = ArtistNameKey::new("haleen", "HALEEN");
        assert_eq!(first, second);
        assert_eq!(first.cmp(&second), Ordering::Equal);
        assert_eq!(ArtistNameKey::from_text(" HALEEN "), first);
        assert_ne!(ArtistNameKey::new("haleen v", "Haleen V"), first);
    }

    fn entry(guid: &str, title: Option<&str>, texts: &[&str]) -> NameFeedEntry {
        let mut keys: BTreeSet<String> = texts.iter().map(|text| name_sort_key(text)).collect();
        if let Some(title) = title {
            keys.insert(name_sort_key(title));
        }
        NameFeedEntry {
            feed_guid: guid.to_owned(),
            title: title.map(str::to_owned),
            keys,
        }
    }

    /// R17-22: a key equal to the key of one feed title gives
    /// `MatchedFeed`. The key of the artist text of a release of one feed
    /// gives the same. "HALEEN " matches the feed title "Haleen".
    #[test]
    fn adr_0084_name_level_r17_22_exactly_one_feed() {
        let matches = name_feed_matches([
            entry("haleen-guid", Some("Haleen"), &[]),
            entry("ledbetter-guid", Some("Ledbetter Music"), &["Ledbetter"]),
        ]);
        assert_eq!(
            matches.get("haleen"),
            NameFeedMatch::MatchedFeed("haleen-guid".to_owned())
        );
        assert_eq!(
            matches.get(&name_sort_key("HALEEN ")),
            NameFeedMatch::MatchedFeed("haleen-guid".to_owned())
        );
        assert_eq!(
            matches.get("ledbetter"),
            NameFeedMatch::MatchedFeed("ledbetter-guid".to_owned())
        );
        assert_eq!(matches.matched_keys("haleen-guid"), ["haleen"]);
        assert_eq!(
            matches.destination(ArtistNameKey::from_text("Haleen")),
            ArtistNameDestination::FeedPage("haleen-guid".to_owned())
        );
        assert_eq!(matches.feed_title("haleen-guid"), "Haleen");
    }

    /// R17-23: a key in the key sets of two feeds gives `Ambiguous`. It
    /// is no matched key of either feed.
    #[test]
    fn adr_0084_name_level_r17_23_two_feeds() {
        let matches = name_feed_matches([
            entry("first-guid", Some("Haleen"), &[]),
            entry("second-guid", Some("Other"), &["haleen"]),
        ]);
        assert_eq!(matches.get("haleen"), NameFeedMatch::Ambiguous);
        assert!(matches.matched_keys("first-guid").is_empty());
        assert_eq!(matches.matched_keys("second-guid"), ["other"]);
        assert_eq!(
            matches.destination(ArtistNameKey::from_text("Haleen")),
            ArtistNameDestination::NameLevel(ArtistNameKey::from_text("Haleen"))
        );
    }

    /// R17-24: a key in no key set gives `NoFeed`, and so does a key with
    /// no entry. The key of "Unknown Artist" gives `NoFeed` when a feed
    /// has that title.
    #[test]
    fn adr_0084_name_level_r17_24_no_feed() {
        let matches = name_feed_matches([entry("unknown-guid", Some(UNKNOWN_ARTIST_TEXT), &[])]);
        assert_eq!(matches.get("survival guide"), NameFeedMatch::NoFeed);
        assert_eq!(
            matches.get(&name_sort_key(UNKNOWN_ARTIST_TEXT)),
            NameFeedMatch::NoFeed
        );
        assert_eq!(
            NameFeedMatches::default().get("haleen"),
            NameFeedMatch::NoFeed
        );
    }

    /// R85-3-17: `ArtistIdentity::resolve` gives `Publisher(guid)` of the
    /// second feed for R85-3-02, and the name level for R85-3-04.
    #[test]
    fn adr_0085_named_feeds_resolve_matches_select_artist_feed() {
        let confirmed_second = [
            NamedFeed {
                confirmed: false,
                ..feed(None, None, false)
            },
            NamedFeed {
                feed_guid: "second-guid".to_owned(),
                role: Some("artist".to_owned()),
                role_agreement: Some("both".to_owned()),
                confirmed: true,
                ..feed(None, None, false)
            },
        ];
        assert_eq!(
            ArtistIdentity::resolve(&confirmed_second, "Haleen"),
            ArtistIdentity::Publisher("second-guid".to_owned())
        );

        let agreed_non_artist = [feed(Some("label"), Some("both"), true)];
        assert_eq!(
            ArtistIdentity::resolve(&agreed_non_artist, "Haleen"),
            ArtistIdentity::Name("haleen".to_owned())
        );
    }
}
