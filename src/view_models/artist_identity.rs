//! The artist identity of a release (ADR 0084 Decision 7, ADR 0085
//! Decision 6).
//!
//! A release with a confirmed or unverified artist feed has the feed level.
//! Each other release has the name level, keyed by its artist text.

#![warn(clippy::pedantic)]

use crate::view_models::named_feeds::{select_artist_feed, ArtistFeed, NamedFeed};

/// The artist identity of a release (ADR 0084 Decision 7).
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ArtistIdentity {
    /// The artist feed that the release names, by its feed GUID.
    Publisher(String),
    /// The artist text of a release that has no artist feed.
    Name(String),
}

impl ArtistIdentity {
    /// The feed level when `named_feeds` gives a confirmed or unverified
    /// artist feed (ADR 0085 Decision 6). Otherwise the name level of
    /// `artist_text`.
    ///
    /// The name level keeps `artist_text` as it is. Two names that differ
    /// only in case are two identities.
    #[must_use]
    pub(crate) fn resolve(named_feeds: &[NamedFeed], artist_text: &str) -> Self {
        match select_artist_feed(named_feeds) {
            ArtistFeed::Confirmed(index) | ArtistFeed::Unverified(index) => {
                Self::Publisher(named_feeds[index].feed_guid.clone())
            }
            ArtistFeed::None => Self::Name(artist_text.to_owned()),
        }
    }
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
        }
    }

    /// R15-01, carried by ADR 0085 Decision 6: a confirmed artist feed
    /// keeps the feed level, whether it is agreed or unverified. No named
    /// feed, or an agreed role without artist, gives the name level.
    #[test]
    fn adr_0084_singles_group_r15_01_resolve_follows_decision_7() {
        let publisher = ArtistIdentity::Publisher("pub-guid".to_owned());
        let name = ArtistIdentity::Name("Haleen".to_owned());
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

    /// R15-01: the name level keeps the artist text as it is, so a case
    /// difference gives two identities.
    #[test]
    fn adr_0084_singles_group_r15_01_name_level_keeps_case() {
        assert_ne!(
            ArtistIdentity::resolve(&[], "Haleen"),
            ArtistIdentity::resolve(&[], "haleen")
        );
        assert_ne!(
            ArtistIdentity::Publisher("Haleen".to_owned()),
            ArtistIdentity::Name("Haleen".to_owned())
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
            ArtistIdentity::Name("Haleen".to_owned())
        );
    }
}
