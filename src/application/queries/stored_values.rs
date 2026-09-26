//! Stored value projection (ADR 0076 Decisions 1 and 5, ADR 0075 packet 020).
//!
//! The app stores one current value for each field and shows that value.
//! This module is the one owner of the order that gives the current value
//! of a field. Views, tag frames and the playlist RSS check read it. No
//! caller selects a source at display time.
//!
//! The order for each field is fixed:
//!
//! 1. the held RSS value, when `rss_field_holds` has a row for the owner and
//!    the field. A cleared hold (a null value) gives no value.
//! 2. the `MusicIndex` fact, when one exists.
//! 3. the column. A field without a column uses its `rss` fact row as its
//!    column: the feed explicit flag, the feed owner name, and the track
//!    description.
//!
//! Each value names its owner: the channel or the item (ADR 0075 Decision I).
//! A feed value never becomes a track value. The album title and the album
//! artist of a track are channel values. The track copy of each is the
//! fallback only when the feed column is null.
//!
//! No value carries a provider label or a renderer type.
//!
//! The credit list of each owner follows the same order (ADR 0076 packet
//! 006). `entity_contributors` keeps one list for each source:
//!
//! 1. the `rss` list, when a hold exists for the persons slot. A cleared
//!    hold gives an empty list.
//! 2. the `musicindex` list, when it has a row.
//! 3. the `rss` list.
//!
//! The projection shows one list, in its stored order. It merges no person
//! across the two lists, and both lists stay in storage.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;

use anyhow::Result;
use rusqlite::Connection;
use serde_json::{json, Value};

use crate::db::rss_field_holds::{self as holds, HoldOwner, RssField};
use crate::db::{
    self, FeedRow, LocalContributorRow, LocalEntityOwner, StoredFeedColumns, TrackRow,
};
use crate::local_metadata::{self, SourcedFacts};
use crate::metadata::drop_placeholder_source_text;
use crate::views::{ContributorView, FeedMetadataFacts, TrackMetadataFacts};

/// The RSS element that states a value (ADR 0075 Decision I).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueOwner {
    /// The `<channel>` of the feed.
    Channel,
    /// One `<item>` of the feed.
    Item,
}

/// One projected value and its owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Owned<T> {
    pub value: Option<T>,
    pub owner: ValueOwner,
}

impl<T> Owned<T> {
    const fn channel(value: Option<T>) -> Self {
        Self {
            value,
            owner: ValueOwner::Channel,
        }
    }

    const fn item(value: Option<T>) -> Self {
        Self {
            value,
            owner: ValueOwner::Item,
        }
    }
}

impl<T> Default for Owned<T> {
    fn default() -> Self {
        Self::channel(None)
    }
}

/// The current stored values of one feed. Each value is a channel value.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FeedStoredValues {
    pub title: Owned<String>,
    pub description: Owned<String>,
    pub language: Owned<String>,
    pub explicit: Owned<bool>,
    /// The `itunes:owner` name, stored as the `publisher_text` fact.
    pub owner_name: Owned<String>,
    pub artwork: Owned<String>,
    pub album_artist: Owned<String>,
    /// A value that `MusicIndex` computes. The check does not compare it.
    pub release_date: Owned<i64>,
    /// A value that `MusicIndex` computes. The check does not compare it.
    pub release_kind: Owned<String>,
    /// The `podcast:person` credits of the channel, in their stored order.
    pub credits: Vec<ContributorView>,
}

/// The current stored values of one track.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackStoredValues {
    pub title: Owned<String>,
    pub artist: Owned<String>,
    /// The channel title, or the track copy when the feed column is null.
    pub album_title: Owned<String>,
    /// `feeds.album_artist`, or `tracks.album_artist_name` when the feed
    /// column is null.
    pub album_artist: Owned<String>,
    /// The item artwork, or the channel artwork when the item has none.
    pub artwork: Owned<String>,
    pub description: Owned<String>,
    pub pub_date: Owned<i64>,
    pub explicit: Owned<bool>,
    pub publisher_text: Owned<String>,
    /// The `podcast:person` credits of the item, in their stored order.
    pub credits: Vec<ContributorView>,
}

impl Default for TrackStoredValues {
    fn default() -> Self {
        Self {
            title: Owned::item(None),
            artist: Owned::item(None),
            album_title: Owned::channel(None),
            album_artist: Owned::channel(None),
            artwork: Owned::item(None),
            description: Owned::item(None),
            pub_date: Owned::item(None),
            explicit: Owned::item(None),
            publisher_text: Owned::item(None),
            credits: Vec::new(),
        }
    }
}

/// The step of the order that supplied a value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Selected<T> {
    /// Step 1: the held RSS value. `None` is a cleared hold.
    Held(Option<T>),
    /// Step 2: the `MusicIndex` fact.
    MusicIndex(T),
    /// Step 3: the column.
    Column(Option<T>),
}

impl<T> Selected<T> {
    pub(crate) fn into_value(self) -> Option<T> {
        match self {
            Self::Held(value) | Self::Column(value) => value,
            Self::MusicIndex(value) => Some(value),
        }
    }
}

/// The hold of one field: the input of step 1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Hold<T> {
    /// `rss_field_holds` has no row for the owner and the field.
    Absent,
    /// The held RSS value. `None` is a cleared field.
    Present(Option<T>),
}

/// ADR 0076 Decisions 1 and 5: the one order of the stored value of a field.
pub(crate) fn select<T>(held: Hold<T>, musicindex: Option<T>, column: Option<T>) -> Selected<T> {
    if let Hold::Present(held) = held {
        return Selected::Held(held);
    }
    if let Some(value) = musicindex {
        return Selected::MusicIndex(value);
    }
    Selected::Column(column)
}

type Holds = BTreeMap<RssField, Option<Value>>;

const MUSICINDEX_SOURCE: &str = "musicindex";
const RSS_SOURCE: &str = "rss";

fn held<T>(holds: &Holds, field: RssField, decode: impl Fn(&Value) -> Option<T>) -> Hold<T> {
    match holds.get(&field) {
        Some(value) => Hold::Present(value.as_ref().and_then(&decode)),
        None => Hold::Absent,
    }
}

fn text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

fn instant(value: &Value) -> Option<i64> {
    value["instant"].as_i64()
}

fn nonempty(value: Option<String>) -> Option<String> {
    drop_placeholder_source_text(
        value
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty()),
    )
}

/// The stored values of one feed.
///
/// # Errors
///
/// Returns an error when a database read fails.
pub(crate) fn feed_values(conn: &Connection, feed_id: i64) -> Result<FeedStoredValues> {
    let columns = db::stored_feed_columns(conn, feed_id)?.unwrap_or_default();
    let facts = local_metadata::sourced_feed_facts(conn, feed_id)?;
    let holds = holds::owner_holds(conn, HoldOwner::Feed(feed_id))?;
    let mut values = project_feed(&holds, &facts, columns);
    values.credits = project_credits(
        &holds,
        db::local_contributors(conn, LocalEntityOwner::Feed(feed_id))?,
    );
    Ok(values)
}

/// The column step alone, for a feed row that a caller has without a
/// database connection. It reads no hold and no fact.
#[must_use]
pub fn feed_values_from_columns(feed: &FeedRow) -> FeedStoredValues {
    project_feed(
        &Holds::new(),
        &SourcedFacts::default(),
        StoredFeedColumns {
            title: feed.title.clone(),
            description: feed.description.clone(),
            language: feed.language.clone(),
            album_image_href: feed.album_image_href.clone(),
            album_artist: None,
        },
    )
}

fn project_feed(
    holds: &Holds,
    facts: &SourcedFacts<FeedMetadataFacts>,
    columns: StoredFeedColumns,
) -> FeedStoredValues {
    let musicindex = &facts.musicindex;
    let rss = &facts.rss;
    FeedStoredValues {
        title: Owned::channel(
            select(held(holds, RssField::Title, text), None, columns.title).into_value(),
        ),
        description: Owned::channel(
            select(
                held(holds, RssField::Description, text),
                musicindex.description.clone(),
                columns.description,
            )
            .into_value(),
        ),
        language: Owned::channel(
            select(
                held(holds, RssField::Language, text),
                musicindex.language.clone(),
                nonempty(columns.language),
            )
            .into_value(),
        ),
        explicit: Owned::channel(
            select(
                held(holds, RssField::Explicit, Value::as_bool),
                musicindex.explicit,
                rss.explicit,
            )
            .into_value(),
        ),
        owner_name: Owned::channel(
            select(
                held(holds, RssField::Owner, text),
                musicindex.publisher_text.clone(),
                rss.publisher_text.clone(),
            )
            .into_value(),
        ),
        artwork: Owned::channel(
            select(
                held(holds, RssField::Artwork, text),
                None,
                columns.album_image_href,
            )
            .into_value(),
        ),
        album_artist: Owned::channel(
            select(
                held(holds, RssField::AlbumArtist, text),
                None,
                columns.album_artist,
            )
            .into_value(),
        ),
        release_date: Owned::channel(
            select(Hold::Absent, musicindex.release_date, None).into_value(),
        ),
        release_kind: Owned::channel(
            select(Hold::Absent, musicindex.release_kind.clone(), None).into_value(),
        ),
        credits: Vec::new(),
    }
}

/// The stored values of one track.
///
/// # Errors
///
/// Returns an error when a database read fails.
pub(crate) fn track_values(conn: &Connection, track: &TrackRow) -> Result<TrackStoredValues> {
    let facts = local_metadata::sourced_track_facts(conn, track.id)?;
    let item_holds = holds::owner_holds(
        conn,
        HoldOwner::Track {
            feed_id: track.feed_id,
            track_id: track.id,
        },
    )?;
    let channel_holds = holds::owner_holds(conn, HoldOwner::Feed(track.feed_id))?;
    let feed_album_artist =
        db::stored_feed_columns(conn, track.feed_id)?.and_then(|columns| columns.album_artist);
    let mut values = project_track(
        track,
        &item_holds,
        &channel_holds,
        &facts,
        feed_album_artist,
    );
    values.credits = project_credits(
        &item_holds,
        db::local_contributors(conn, LocalEntityOwner::Track(track.id))?,
    );
    Ok(values)
}

/// The credit list of one feed, for a caller that needs no other value.
///
/// # Errors
///
/// Returns an error when a database read fails.
pub(crate) fn feed_credits(conn: &Connection, feed_id: i64) -> Result<Vec<ContributorView>> {
    Ok(project_credits(
        &holds::owner_holds(conn, HoldOwner::Feed(feed_id))?,
        db::local_contributors(conn, LocalEntityOwner::Feed(feed_id))?,
    ))
}

/// ADR 0076 packet 006: one credit list for each owner, in the order of
/// [`select`]. A row of another source token is evidence only.
fn project_credits(holds: &Holds, rows: Vec<LocalContributorRow>) -> Vec<ContributorView> {
    let mut musicindex = Vec::new();
    let mut rss = Vec::new();
    for row in rows {
        let list = match row.source.as_str() {
            MUSICINDEX_SOURCE => &mut musicindex,
            RSS_SOURCE => &mut rss,
            _ => continue,
        };
        list.push(ContributorView {
            name: row.name,
            role: row.role,
            group_name: row.group_name,
            href: row.href,
            image_url: row.image_url,
            nostr_npub: row.nostr_npub,
        });
    }
    let held = match holds.get(&RssField::Persons) {
        // The `rss` rows and the hold are written together. A cleared hold
        // gives an empty list, although `MusicIndex` supplied one.
        Some(Some(_)) => Hold::Present(Some(rss.clone())),
        Some(None) => Hold::Present(None),
        None => Hold::Absent,
    };
    select(
        held,
        (!musicindex.is_empty()).then_some(musicindex),
        Some(rss),
    )
    .into_value()
    .unwrap_or_default()
}

/// The column step alone, for a track row that a caller has without a
/// database connection. It reads no hold and no fact. The album artist is
/// the track copy, because the row does not carry `feeds.album_artist`.
#[must_use]
pub fn track_values_from_columns(track: &TrackRow) -> TrackStoredValues {
    project_track(
        track,
        &Holds::new(),
        &Holds::new(),
        &SourcedFacts::default(),
        None,
    )
}

fn project_track(
    track: &TrackRow,
    item_holds: &Holds,
    channel_holds: &Holds,
    facts: &SourcedFacts<TrackMetadataFacts>,
    feed_album_artist: Option<String>,
) -> TrackStoredValues {
    let musicindex = &facts.musicindex;
    let rss = &facts.rss;
    TrackStoredValues {
        title: Owned::item(
            select(
                held(item_holds, RssField::Title, text),
                None,
                track.track_title.clone(),
            )
            .into_value(),
        ),
        artist: Owned::item(
            select(
                held(item_holds, RssField::Artist, text),
                None,
                track.artist_name.clone(),
            )
            .into_value(),
        ),
        album_title: channel_or_track_copy(
            select(
                held(channel_holds, RssField::Title, text),
                None,
                track.feed_title.clone(),
            ),
            // The subscribe step copies the channel title into each track.
            Owned::channel(track.album_title.clone()),
        ),
        album_artist: channel_or_track_copy(
            select(
                held(channel_holds, RssField::AlbumArtist, text),
                None,
                feed_album_artist,
            ),
            // The track copy can hold the item author, so the item owns it.
            Owned::item(track.album_artist_name.clone()),
        ),
        artwork: item_or_channel(
            select(
                held(item_holds, RssField::Artwork, text),
                None,
                track.track_image_href.clone(),
            ),
            select(
                held(channel_holds, RssField::Artwork, text),
                None,
                track.album_image_href.clone(),
            ),
        ),
        description: Owned::item(
            select(
                held(item_holds, RssField::Description, text),
                musicindex.description.clone(),
                rss.description.clone(),
            )
            .into_value(),
        ),
        pub_date: Owned::item(
            select(
                held(item_holds, RssField::Date, instant),
                musicindex.pub_date,
                track.pub_date,
            )
            .into_value(),
        ),
        explicit: Owned::item(
            select(
                held(item_holds, RssField::Explicit, Value::as_bool),
                musicindex.explicit,
                track.explicit,
            )
            .into_value(),
        ),
        publisher_text: Owned::item(
            select(
                Hold::Absent,
                musicindex.publisher_text.clone(),
                rss.publisher_text.clone(),
            )
            .into_value(),
        ),
        credits: Vec::new(),
    }
}

/// A channel value of a track. The track copy of the value is the
/// fallback only when the channel has no hold and its column is null.
fn channel_or_track_copy<T>(channel: Selected<T>, track_copy: Owned<T>) -> Owned<T> {
    match channel {
        Selected::Column(None) => track_copy,
        channel => Owned::channel(channel.into_value()),
    }
}

/// An item value with the channel value as its display fallback. ADR 0075
/// Decision C uses it for the artwork of a track.
fn item_or_channel<T>(item: Selected<T>, channel: Selected<T>) -> Owned<T> {
    match item.into_value() {
        Some(value) => Owned::item(Some(value)),
        None => Owned::channel(channel.into_value()),
    }
}

/// The compared stored value of one fact-backed slot, for the playlist RSS
/// check of ADR 0076 packet 002.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ComparedSlot {
    pub(crate) value: Option<Value>,
    /// `true` when the value is the `MusicIndex` fact. A `MusicIndex`
    /// description is plain text, and each other description is HTML.
    pub(crate) musicindex: bool,
}

/// The stored value of a fact-backed slot in its JSON form, in the order of
/// this module. `column` is the column value of a field with a column. A
/// field without a column reads its `rss` fact row, and `column` must be
/// `None`.
///
/// # Errors
///
/// Returns an error when a database read fails.
pub(crate) fn compared_slot(
    conn: &Connection,
    owner: HoldOwner,
    field: RssField,
    column: Option<Value>,
) -> Result<ComparedSlot> {
    let (musicindex, rss_column) = match owner {
        HoldOwner::Feed(feed_id) => {
            let facts = local_metadata::sourced_feed_facts(conn, feed_id)?;
            feed_fact_json(&facts, field)
        }
        HoldOwner::Track { track_id, .. } => {
            let facts = local_metadata::sourced_track_facts(conn, track_id)?;
            track_fact_json(&facts, field)
        }
    };
    let column = match rss_column {
        ColumnStep::Caller => column,
        ColumnStep::RssFact(value) => value,
    };
    let held = match holds::hold(conn, owner, field)? {
        Some(hold) => Hold::Present(hold.value),
        None => Hold::Absent,
    };
    let selected = select(held, musicindex, column);
    let from_musicindex = matches!(selected, Selected::MusicIndex(_));
    Ok(ComparedSlot {
        value: selected.into_value(),
        musicindex: from_musicindex,
    })
}

/// The column step of a compared slot.
enum ColumnStep {
    /// The field has a column. The caller supplies its value.
    Caller,
    /// The field has no column. Its `rss` fact row is the column.
    RssFact(Option<Value>),
}

/// The `MusicIndex` value of a feed field, and the `rss` fact value when the
/// field has no column.
fn feed_fact_json(
    facts: &SourcedFacts<FeedMetadataFacts>,
    field: RssField,
) -> (Option<Value>, ColumnStep) {
    let (musicindex, rss) = (&facts.musicindex, &facts.rss);
    match field {
        RssField::Description => (
            musicindex.description.clone().map(Value::String),
            ColumnStep::Caller,
        ),
        RssField::Language => (
            musicindex.language.clone().map(Value::String),
            ColumnStep::Caller,
        ),
        RssField::Explicit => (
            musicindex.explicit.map(Value::Bool),
            ColumnStep::RssFact(rss.explicit.map(Value::Bool)),
        ),
        RssField::Owner => (
            musicindex.publisher_text.clone().map(Value::String),
            ColumnStep::RssFact(rss.publisher_text.clone().map(Value::String)),
        ),
        _ => (None, ColumnStep::Caller),
    }
}

/// The `MusicIndex` value of a track field, and the `rss` fact value when the
/// field has no column.
fn track_fact_json(
    facts: &SourcedFacts<TrackMetadataFacts>,
    field: RssField,
) -> (Option<Value>, ColumnStep) {
    let (musicindex, rss) = (&facts.musicindex, &facts.rss);
    match field {
        RssField::Description => (
            musicindex.description.clone().map(Value::String),
            ColumnStep::RssFact(rss.description.clone().map(Value::String)),
        ),
        RssField::Date => (
            musicindex
                .pub_date
                .map(|instant| json!({ "instant": instant })),
            ColumnStep::Caller,
        ),
        RssField::Explicit => (musicindex.explicit.map(Value::Bool), ColumnStep::Caller),
        _ => (None, ColumnStep::Caller),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::db::rss_check_runs::{self as runs, RssCheckTrigger};
    use crate::db::{LocalMetadataFactInput, LocalMetadataOwner, LocalMetadataValue};
    use crate::rss::check_apply::apply_checked_document;
    use crate::rss::check_apply::test_support::{document, first, second};
    use crate::sources::{FetchMode, LocalSource, MetadataSource};
    use crate::views::{FeedRef, TrackRef};

    const FEED_ID: i64 = 1;
    const TRACK_ID: i64 = 11;
    const CHECKED_AT: i64 = 1_790_000_000_000_000;

    fn database() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        db::init_schema(&conn).unwrap();
        db::migrate_schema(&conn).unwrap();
        conn.execute("INSERT INTO playlists(id, name) VALUES (1, 'Show')", [])
            .unwrap();
        conn
    }

    /// One feed and one track with column values and no hold.
    fn stored() -> Connection {
        let conn = database();
        conn.execute(
            "INSERT INTO feeds(id, feed_url, feed_guid, title, description, language, album_image_href, album_artist)
             VALUES (?1, 'https://band.test/feed.xml', 'feed-guid-1', 'Column album', 'Column notes', 'en', 'https://band.test/cover.jpg', 'Channel Artist')",
            [FEED_ID],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tracks(id, feed_id, item_guid, track_title, artist_name, album_title, album_artist_name, pub_date, itunes_explicit)
             VALUES (?1, ?2, 'item-1', 'Column song', 'Item Artist', 'Copied album', 'Copied Artist', 'Tue, 01 Sep 2026 10:00:00 +0000', 'false')",
            [TRACK_ID, FEED_ID],
        )
        .unwrap();
        conn
    }

    fn musicindex_fact(
        conn: &Connection,
        owner: LocalMetadataOwner,
        key: &str,
        value: LocalMetadataValue,
    ) {
        db::replace_local_metadata_fact(
            conn,
            owner,
            "musicindex",
            &LocalMetadataFactInput {
                fact_key: key.to_owned(),
                value,
                extraction_path: Some(format!("$.{key}")),
                observed_at: Some(1),
                raw_json: None,
            },
        )
        .unwrap();
    }

    fn hold(conn: &Connection, owner: HoldOwner, field: RssField, value: Option<Value>) {
        holds::write_hold(conn, owner, field, value.as_ref(), None, CHECKED_AT).unwrap();
    }

    fn track_owner() -> HoldOwner {
        HoldOwner::Track {
            feed_id: FEED_ID,
            track_id: TRACK_ID,
        }
    }

    fn track_row(conn: &Connection, track_id: i64) -> TrackRow {
        db::track_row_by_id(conn, track_id).unwrap().unwrap()
    }

    /// R20-01: a hold gives the held value for its field. Another field of
    /// the same owner gives its `MusicIndex` fact.
    #[test]
    fn adr_0075_projection_hold_wins_for_its_field_only() {
        let conn = stored();
        let owner = LocalMetadataOwner::Feed(FEED_ID);
        musicindex_fact(
            &conn,
            owner,
            "description",
            LocalMetadataValue::Text("Index notes".into()),
        );
        musicindex_fact(
            &conn,
            owner,
            "language",
            LocalMetadataValue::Text("fr".into()),
        );
        hold(
            &conn,
            HoldOwner::Feed(FEED_ID),
            RssField::Description,
            Some(json!("<p>Held notes</p>")),
        );

        let values = feed_values(&conn, FEED_ID).unwrap();

        assert_eq!(
            values.description.value.as_deref(),
            Some("<p>Held notes</p>")
        );
        assert_eq!(values.language.value.as_deref(), Some("fr"));
    }

    /// R20-02: without a hold, the `MusicIndex` fact comes first, then the
    /// column.
    #[test]
    fn adr_0075_projection_musicindex_fact_then_column() {
        let conn = stored();
        let feed = LocalMetadataOwner::Feed(FEED_ID);
        let track = LocalMetadataOwner::Track(TRACK_ID);
        musicindex_fact(
            &conn,
            feed,
            "language",
            LocalMetadataValue::Text("fr".into()),
        );
        musicindex_fact(&conn, track, "explicit", LocalMetadataValue::Boolean(true));

        let feed_values = feed_values(&conn, FEED_ID).unwrap();
        let track_values = track_values(&conn, &track_row(&conn, TRACK_ID)).unwrap();

        assert_eq!(feed_values.language.value.as_deref(), Some("fr"));
        assert_eq!(
            feed_values.description.value.as_deref(),
            Some("Column notes")
        );
        assert_eq!(track_values.explicit.value, Some(true));
        assert_eq!(track_values.title.value.as_deref(), Some("Column song"));
        assert_eq!(
            track_values.pub_date.value,
            track_row(&conn, TRACK_ID).pub_date
        );

        db::delete_local_metadata_fact(&conn, feed, "musicindex", "language").unwrap();
        let feed_values = super::feed_values(&conn, FEED_ID).unwrap();
        assert_eq!(feed_values.language.value.as_deref(), Some("en"));
    }

    /// R20-03: a cleared hold gives no value, although the column and the
    /// `MusicIndex` fact hold one.
    #[test]
    fn adr_0075_projection_cleared_hold_gives_no_value() {
        let conn = stored();
        musicindex_fact(
            &conn,
            LocalMetadataOwner::Feed(FEED_ID),
            "language",
            LocalMetadataValue::Text("fr".into()),
        );
        hold(&conn, HoldOwner::Feed(FEED_ID), RssField::Language, None);
        hold(&conn, track_owner(), RssField::Title, None);

        let feed_values = feed_values(&conn, FEED_ID).unwrap();
        let track_values = track_values(&conn, &track_row(&conn, TRACK_ID)).unwrap();

        assert_eq!(feed_values.language.value, None);
        assert_eq!(track_values.title.value, None);
        assert_eq!(
            track_row(&conn, TRACK_ID).track_title.as_deref(),
            Some("Column song")
        );
    }

    /// R20-04: after a packet 002 apply, the feed view and the track view
    /// show the held RSS values, and not the older `MusicIndex` facts.
    #[test]
    fn adr_0075_projection_views_show_held_values_after_apply() {
        let conn = database();
        conn.execute(
            "INSERT INTO feeds(id, feed_url, feed_guid) VALUES (?1, 'https://band.test/feed.xml', 'feed-guid-1')",
            [FEED_ID],
        )
        .unwrap();
        let first_run = runs::insert_run(&conn, 1, RssCheckTrigger::Button, CHECKED_AT).unwrap();
        apply_checked_document(&conn, first_run, FEED_ID, &document(&first()), CHECKED_AT).unwrap();
        let track_id: i64 = conn
            .query_row(
                "SELECT id FROM tracks WHERE item_guid = 'item-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        musicindex_fact(
            &conn,
            LocalMetadataOwner::Feed(FEED_ID),
            "description",
            LocalMetadataValue::Text("Old index notes".into()),
        );
        musicindex_fact(
            &conn,
            LocalMetadataOwner::Track(track_id),
            "description",
            LocalMetadataValue::Text("Old index song notes".into()),
        );
        let later = CHECKED_AT + 1_000_000;
        let second_run = runs::insert_run(&conn, 1, RssCheckTrigger::Button, later).unwrap();
        apply_checked_document(&conn, second_run, FEED_ID, &document(&second()), later).unwrap();

        let source = LocalSource::new(Arc::new(Mutex::new(conn)));
        let feed = source
            .fetch_feed(&FeedRef::LocalFeedId(FEED_ID), FetchMode::WithTracks)
            .unwrap();
        let track = source
            .fetch_track(&TrackRef::LocalTrackId(track_id))
            .unwrap();

        assert_eq!(feed.title.as_deref(), Some(second().title));
        assert_eq!(feed.description.as_deref(), Some("<p>New album notes</p>"));
        assert_eq!(feed.language.as_deref(), Some("fr"));
        assert_eq!(track.title.as_deref(), Some("Song One (Remaster)"));
        assert_eq!(track.description.as_deref(), Some("<p>New song notes</p>"));
        assert_eq!(track.album.as_deref(), Some(second().title));
        let listed = feed
            .tracks
            .iter()
            .find(|view| view.track_guid.as_deref() == Some("item-1"))
            .unwrap();
        assert_eq!(listed.title.as_deref(), Some("Song One (Remaster)"));
    }

    /// R20-05: the expected ID3 frames of a track use the held value.
    #[test]
    fn adr_0075_projection_tag_frames_use_held_value() {
        let conn = stored();
        hold(
            &conn,
            track_owner(),
            RssField::Title,
            Some(json!("Held song")),
        );
        hold(
            &conn,
            HoldOwner::Feed(FEED_ID),
            RssField::Title,
            Some(json!("Held album")),
        );

        let context = crate::feed_service::track_row_to_track_context_with_local_identity(
            &conn,
            &track_row(&conn, TRACK_ID),
        )
        .unwrap();
        let edits = crate::metadata_service::id3_edits_for_track_context(&context);
        let frame = |label: &str| {
            edits
                .iter()
                .find(|edit| edit.frame_label == label)
                .map(|edit| edit.value.clone())
        };

        assert_eq!(frame("TIT2").as_deref(), Some("Held song"));
        assert_eq!(frame("TALB").as_deref(), Some("Held album"));
        assert_eq!(frame("TPE2").as_deref(), Some("Channel Artist"));
    }

    /// R20-06: each value names its owner, and no value carries a provider
    /// label.
    #[test]
    fn adr_0075_projection_values_name_owner_without_provider_label() {
        let conn = stored();
        musicindex_fact(
            &conn,
            LocalMetadataOwner::Feed(FEED_ID),
            "publisher_text",
            LocalMetadataValue::Text("Label A".into()),
        );
        musicindex_fact(
            &conn,
            LocalMetadataOwner::Track(TRACK_ID),
            "description",
            LocalMetadataValue::Text("Song notes".into()),
        );

        let feed = feed_values(&conn, FEED_ID).unwrap();
        let track = track_values(&conn, &track_row(&conn, TRACK_ID)).unwrap();

        for owner in [
            feed.title.owner,
            feed.description.owner,
            feed.language.owner,
            feed.explicit.owner,
            feed.owner_name.owner,
            feed.artwork.owner,
            feed.album_artist.owner,
            feed.release_date.owner,
            feed.release_kind.owner,
            track.album_title.owner,
            track.album_artist.owner,
        ] {
            assert_eq!(owner, ValueOwner::Channel);
        }
        for owner in [
            track.title.owner,
            track.artist.owner,
            track.description.owner,
            track.pub_date.owner,
            track.explicit.owner,
            track.publisher_text.owner,
        ] {
            assert_eq!(owner, ValueOwner::Item);
        }
        let text = format!("{feed:?} {track:?}").to_lowercase();
        for label in ["musicindex", "rss", "source", "provider"] {
            assert!(
                !text.contains(label),
                "a projected value names `{label}`: {text}"
            );
        }
    }

    fn credit_list(conn: &mut Connection, owner: LocalEntityOwner, source: &str, names: &[&str]) {
        let rows = names
            .iter()
            .enumerate()
            .map(|(position, name)| db::LocalContributorInput {
                position: i64::try_from(position).unwrap(),
                name: Some((*name).to_owned()),
                role: Some("vocals".into()),
                ..db::LocalContributorInput::default()
            })
            .collect::<Vec<_>>();
        db::replace_local_contributors(conn, owner, source, &rows).unwrap();
    }

    fn credit_names(credits: &[ContributorView]) -> Vec<&str> {
        credits
            .iter()
            .filter_map(|credit| credit.name.as_deref())
            .collect()
    }

    fn persons_hold(conn: &Connection, owner: HoldOwner, names: &[&str]) {
        let value = (!names.is_empty()).then(|| {
            Value::Array(
                names
                    .iter()
                    .map(|name| json!({"name": name, "role": "vocals"}))
                    .collect(),
            )
        });
        hold(conn, owner, RssField::Persons, value);
    }

    /// Both credit lists of the fixture feed and track, with the `rss` rows
    /// in a stored order that differs from the name order.
    fn with_both_credit_lists() -> Connection {
        let mut conn = stored();
        for owner in [
            LocalEntityOwner::Feed(FEED_ID),
            LocalEntityOwner::Track(TRACK_ID),
        ] {
            credit_list(&mut conn, owner, "rss", &["Zed", "Amy"]);
            credit_list(&mut conn, owner, "musicindex", &["Mia", "Noor"]);
        }
        conn
    }

    fn local_views(conn: Connection) -> (crate::views::FeedView, crate::views::TrackView) {
        let source = LocalSource::new(Arc::new(Mutex::new(conn)));
        let feed = source
            .fetch_feed(&FeedRef::LocalFeedId(FEED_ID), FetchMode::WithTracks)
            .unwrap();
        let track = source
            .fetch_track(&TrackRef::LocalTrackId(TRACK_ID))
            .unwrap();
        (feed, track)
    }

    /// R6-01: with a persons hold, the feed view and the track view show
    /// only the `rss` list, in its stored order.
    #[test]
    fn adr_0076_credit_list_hold_shows_only_the_rss_list_in_order() {
        let conn = with_both_credit_lists();
        persons_hold(&conn, HoldOwner::Feed(FEED_ID), &["Zed", "Amy"]);
        persons_hold(&conn, track_owner(), &["Zed", "Amy"]);

        let (feed, track) = local_views(conn);

        assert_eq!(credit_names(&feed.contributors), ["Zed", "Amy"]);
        assert_eq!(credit_names(&track.contributors), ["Zed", "Amy"]);
        assert_eq!(credit_names(&feed.tracks[0].contributors), ["Zed", "Amy"]);
    }

    /// R6-02: without a hold, the views show only the `musicindex` list.
    /// Without a `musicindex` list, they show the `rss` list.
    #[test]
    fn adr_0076_credit_list_without_hold_shows_only_the_musicindex_list() {
        let conn = with_both_credit_lists();
        let (feed, track) = local_views(conn);
        assert_eq!(credit_names(&feed.contributors), ["Mia", "Noor"]);
        assert_eq!(credit_names(&track.contributors), ["Mia", "Noor"]);

        let mut conn = stored();
        credit_list(
            &mut conn,
            LocalEntityOwner::Track(TRACK_ID),
            "rss",
            &["Zed", "Amy"],
        );
        credit_list(
            &mut conn,
            LocalEntityOwner::Track(TRACK_ID),
            "embedded",
            &["Evidence Only"],
        );
        let (feed, track) = local_views(conn);
        assert!(feed.contributors.is_empty());
        assert_eq!(credit_names(&track.contributors), ["Zed", "Amy"]);
    }

    /// R6-03: a cleared hold gives an empty list, although a `musicindex`
    /// list exists.
    #[test]
    fn adr_0076_credit_list_cleared_hold_gives_an_empty_list() {
        let conn = with_both_credit_lists();
        hold(&conn, HoldOwner::Feed(FEED_ID), RssField::Persons, None);
        hold(&conn, track_owner(), RssField::Persons, None);

        let values = track_values(&conn, &track_row(&conn, TRACK_ID)).unwrap();
        assert!(values.credits.is_empty());
        assert!(feed_credits(&conn, FEED_ID).unwrap().is_empty());
        let (feed, track) = local_views(conn);

        assert!(feed.contributors.is_empty());
        assert!(track.contributors.is_empty());
    }

    /// R6-04: after a check, `entity_contributors` keeps the `rss` list and
    /// the `musicindex` list. The view shows the held `rss` list.
    #[test]
    fn adr_0076_credit_list_check_keeps_both_stored_lists() {
        let mut conn = database();
        conn.execute(
            "INSERT INTO feeds(id, feed_url, feed_guid) VALUES (?1, 'https://band.test/feed.xml', 'feed-guid-1')",
            [FEED_ID],
        )
        .unwrap();
        let first_run = runs::insert_run(&conn, 1, RssCheckTrigger::Button, CHECKED_AT).unwrap();
        apply_checked_document(&conn, first_run, FEED_ID, &document(&first()), CHECKED_AT).unwrap();
        let track_id: i64 = conn
            .query_row(
                "SELECT id FROM tracks WHERE item_guid = 'item-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let track = LocalEntityOwner::Track(track_id);
        credit_list(&mut conn, track, "musicindex", &["Bob", "Index Guest"]);
        let later = CHECKED_AT + 1_000_000;
        let second_run = runs::insert_run(&conn, 1, RssCheckTrigger::Button, later).unwrap();
        apply_checked_document(&conn, second_run, FEED_ID, &document(&second()), later).unwrap();

        let stored = db::local_contributors(&conn, track).unwrap();
        let by_source = |source: &str| {
            stored
                .iter()
                .filter(|row| row.source == source)
                .filter_map(|row| row.name.as_deref())
                .collect::<Vec<_>>()
        };
        assert_eq!(by_source("rss"), ["Dee"]);
        assert_eq!(by_source("musicindex"), ["Bob", "Index Guest"]);

        let values = track_values(&conn, &track_row(&conn, track_id)).unwrap();
        assert_eq!(credit_names(&values.credits), ["Dee"]);
    }

    /// R6-06: no credit row carries a provider label. The projected list,
    /// the views and the track context of the tag frames name no source.
    #[test]
    fn adr_0076_credit_list_rows_carry_no_provider_label() {
        let conn = with_both_credit_lists();
        persons_hold(&conn, track_owner(), &["Zed", "Amy"]);
        let row = track_row(&conn, TRACK_ID);
        let values = track_values(&conn, &row).unwrap();
        let feed_list = feed_credits(&conn, FEED_ID).unwrap();
        let context =
            crate::feed_service::track_row_to_track_context_with_local_identity(&conn, &row)
                .unwrap();

        let context_credits = context
            .track
            .source_contributors
            .clone()
            .unwrap_or_default()
            .into_iter()
            .chain(
                context
                    .feed
                    .as_ref()
                    .and_then(|feed| feed.source_contributors.clone())
                    .unwrap_or_default(),
            )
            .collect::<Vec<_>>();
        assert_eq!(context_credits.len(), 4);
        for credit in &context_credits {
            assert_eq!(credit.source, None);
            assert_eq!(credit.extraction_path, None);
        }
        let names = context_credits
            .iter()
            .filter_map(|credit| credit.name.as_deref())
            .collect::<Vec<_>>();
        assert_eq!(names, ["Zed", "Amy", "Mia", "Noor"]);

        let (feed, track) = local_views(conn);
        let text = format!(
            "{:?} {:?} {:?} {:?}",
            values.credits, feed_list, feed.contributors, track.contributors
        )
        .to_lowercase();
        for label in ["musicindex", "rss", "source", "provider"] {
            assert!(
                !text.contains(label),
                "a credit row names `{label}`: {text}"
            );
        }
    }

    /// R20-08: the album artist of a track view is `feeds.album_artist`,
    /// owned by the channel. With a null feed column it is
    /// `tracks.album_artist_name`, owned by the item.
    #[test]
    fn adr_0075_projection_album_artist_is_the_channel_value() {
        let conn = stored();
        let values = track_values(&conn, &track_row(&conn, TRACK_ID)).unwrap();
        assert_eq!(values.album_artist.value.as_deref(), Some("Channel Artist"));
        assert_eq!(values.album_artist.owner, ValueOwner::Channel);

        conn.execute(
            "UPDATE feeds SET album_artist = NULL WHERE id = ?1",
            [FEED_ID],
        )
        .unwrap();
        let values = track_values(&conn, &track_row(&conn, TRACK_ID)).unwrap();
        assert_eq!(values.album_artist.value.as_deref(), Some("Copied Artist"));
        assert_eq!(values.album_artist.owner, ValueOwner::Item);

        conn.execute(
            "UPDATE feeds SET album_artist = 'Channel Artist' WHERE id = ?1",
            [FEED_ID],
        )
        .unwrap();
        let source = LocalSource::new(Arc::new(Mutex::new(conn)));
        let view = source
            .fetch_track(&TrackRef::LocalTrackId(TRACK_ID))
            .unwrap();
        assert_eq!(view.album_artist.as_deref(), Some("Channel Artist"));
    }
}
