//! The named feeds of an album, and its artist feed (ADR 0085 Decisions 5
//! and 6).
//!
//! An album can name more than one feed. This module keeps their RSS
//! sequence and gives the artist feed by the read of Decision 6 ([ADR 0082
//! task 003](../../docs/tasks/archive/adr-0082-task-003-release-page-named-feeds.md)).
//! It is pure and has no renderer type.

#![warn(clippy::pedantic)]

use crate::view_models::role_words::role_contains_artist;

/// The `role_agreement` value that tells that both sides agree on the role
/// of a named feed link (ADR 0085 Decision 6).
const ROLE_AGREEMENT_BOTH: &str = "both";

/// One feed that an album names, in its RSS sequence (ADR 0085 Decision 6).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NamedFeed {
    pub(crate) feed_guid: String,
    /// The title of the feed, when the app knows it.
    pub(crate) title: Option<String>,
    pub(crate) position: Option<i64>,
    /// The role set of the link, joined by `", "`.
    pub(crate) role: Option<String>,
    /// `both`, `one_side`, `conflict`, or `None`.
    pub(crate) role_agreement: Option<String>,
    /// The raw `rel` of the feed side, and of the album side.
    pub(crate) feed_rel: Option<String>,
    pub(crate) album_rel: Option<String>,
    /// `two_way_validated` of the link: the two feeds name each other.
    pub(crate) confirmed: bool,
}

/// The artist feed of an album (ADR 0085 Decision 6, read above).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ArtistFeed {
    Confirmed(usize),
    Unverified(usize),
    None,
}

/// Sorts `feeds` by position (ADR 0085 Decision 6). A null position comes
/// after each known position. Equal positions sort by `feed_guid`. No other
/// value changes the sequence.
pub(crate) fn sort_named_feeds(feeds: &mut [NamedFeed]) {
    feeds.sort_by(|a, b| {
        (a.position.is_none(), a.position)
            .cmp(&(b.position.is_none(), b.position))
            .then_with(|| a.feed_guid.cmp(&b.feed_guid))
    });
}

/// The artist feed of `feeds`, already in the sequence of
/// [`sort_named_feeds`]. The read of ADR 0085 Decision 6, with the amendment
/// that the operator accepted on 2026-10-18 (ADR 0082 task 003):
///
/// 1. The artist feed is the first named feed whose link has
///    `role_agreement` `both` and a role set that contains `artist`. The
///    release has a **confirmed** artist feed.
/// 2. Else, the primary named feed is the artist feed when its link does
///    not have `role_agreement` `both` and is confirmed (`two_way_validated`
///    `true`). The release has an **unverified** artist feed.
/// 3. Else, the release has no artist feed.
///
/// A count, a name match and an observation time never select the artist
/// feed.
#[must_use]
pub(crate) fn select_artist_feed(feeds: &[NamedFeed]) -> ArtistFeed {
    if let Some(index) = feeds.iter().position(|feed| {
        feed.role_agreement.as_deref() == Some(ROLE_AGREEMENT_BOTH)
            && role_contains_artist(feed.role.as_deref())
    }) {
        return ArtistFeed::Confirmed(index);
    }
    let Some(primary) = feeds.first() else {
        return ArtistFeed::None;
    };
    if primary.role_agreement.as_deref() != Some(ROLE_AGREEMENT_BOTH) && primary.confirmed {
        return ArtistFeed::Unverified(0);
    }
    ArtistFeed::None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(feed_guid: &str, position: Option<i64>) -> NamedFeed {
        NamedFeed {
            feed_guid: feed_guid.to_owned(),
            title: None,
            position,
            role: None,
            role_agreement: None,
            feed_rel: None,
            album_rel: None,
            confirmed: false,
        }
    }

    /// R85-3-01: `sort_named_feeds` gives the position sequence. A null
    /// position comes after each known position. Equal positions sort by
    /// feed GUID.
    #[test]
    fn adr_0085_named_feeds_sort_orders_by_position_then_guid() {
        let mut feeds = vec![
            feed("c", None),
            feed("b", Some(1)),
            feed("a", Some(1)),
            feed("z", Some(0)),
        ];
        sort_named_feeds(&mut feeds);
        let guids: Vec<&str> = feeds.iter().map(|f| f.feed_guid.as_str()).collect();
        assert_eq!(guids, vec!["z", "a", "b", "c"]);
    }

    /// R85-3-02: a first feed with no role and a second feed with agreed
    /// `artist` give `Confirmed(1)`.
    #[test]
    fn adr_0085_named_feeds_select_artist_feed_confirms_the_agreed_artist_role() {
        let feeds = vec![
            NamedFeed {
                role_agreement: None,
                confirmed: false,
                ..feed("primary", Some(0))
            },
            NamedFeed {
                role: Some("artist".to_owned()),
                role_agreement: Some("both".to_owned()),
                confirmed: true,
                ..feed("second", Some(1))
            },
        ];
        assert_eq!(select_artist_feed(&feeds), ArtistFeed::Confirmed(1));
    }

    /// R85-3-03: no agreed `artist`, and a primary feed with no agreed role,
    /// give `Unverified(0)`. A primary feed with a one-side `artist` role
    /// also gives `Unverified(0)`. A primary feed with an unconfirmed link
    /// gives `None`.
    #[test]
    fn adr_0085_named_feeds_select_artist_feed_gives_unverified_or_none() {
        let no_role = vec![NamedFeed {
            role: None,
            role_agreement: None,
            confirmed: true,
            ..feed("primary", Some(0))
        }];
        assert_eq!(select_artist_feed(&no_role), ArtistFeed::Unverified(0));

        let one_side_artist = vec![NamedFeed {
            role: Some("artist".to_owned()),
            role_agreement: Some("one_side".to_owned()),
            confirmed: true,
            ..feed("primary", Some(0))
        }];
        assert_eq!(
            select_artist_feed(&one_side_artist),
            ArtistFeed::Unverified(0)
        );

        let unconfirmed = vec![NamedFeed {
            role: None,
            role_agreement: None,
            confirmed: false,
            ..feed("primary", Some(0))
        }];
        assert_eq!(select_artist_feed(&unconfirmed), ArtistFeed::None);
    }

    /// R85-3-04: a primary feed with agreed `label`, and no other agreed
    /// `artist`, gives `None`.
    #[test]
    fn adr_0085_named_feeds_select_artist_feed_gives_none_for_an_agreed_non_artist_role() {
        let feeds = vec![NamedFeed {
            role: Some("label".to_owned()),
            role_agreement: Some("both".to_owned()),
            confirmed: true,
            ..feed("primary", Some(0))
        }];
        assert_eq!(select_artist_feed(&feeds), ArtistFeed::None);
    }

    /// An empty list has no artist feed.
    #[test]
    fn adr_0085_named_feeds_select_artist_feed_gives_none_for_an_empty_list() {
        assert_eq!(select_artist_feed(&[]), ArtistFeed::None);
    }
}
