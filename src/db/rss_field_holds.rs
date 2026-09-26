//! RSS field holds, check differences and removed-track marks
//! (ADR 0076 Decisions 3 to 7, packet 002).
//!
//! Migration 16 creates `rss_field_holds` and `rss_check_differences`, adds
//! `tracks.removed_from_feed_at`, `tracks.removed_from_feed_confirmed_at`
//! and `feeds.album_artist`, fills `feeds.album_artist`, and drops the three
//! ADR 0075 selection and discrepancy tables that ADR 0076 superseded.
//!
//! A hold keeps an RSS value that the playlist check wrote. Each
//! `MusicIndex` writer of a compared slot calls [`musicindex_gate`] before
//! it writes that slot (ADR 0076 Decision 5).

#![warn(clippy::pedantic)]

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use crate::db::LocalContributorInput;
use crate::rss::compare::{self, ComparedPerson, TextRepresentation};

pub(super) const COLUMNS: &[(&str, &[&str])] = &[
    (
        "rss_field_holds",
        &[
            "owner_kind",
            "feed_id",
            "track_id",
            "field",
            "rss_value_json",
            "run_id",
            "checked_at_us",
        ],
    ),
    (
        "rss_check_differences",
        &[
            "id",
            "run_id",
            "feed_id",
            "track_id",
            "field",
            "kind",
            "old_value_json",
            "new_value_json",
            "recorded_at_us",
        ],
    ),
];

/// The read contract of `feeds` and `tracks` after migration 16: the frozen
/// version-11 columns, then the columns that migration 16 appends.
pub(super) const EXTENDED_COLUMNS: &[(&str, &[&str])] = &[
    (
        "feeds",
        &[
            "id",
            "feed_url",
            "feed_guid",
            "title",
            "link",
            "language",
            "description",
            "podcast_medium",
            "album_image_href",
            "album_image_mime",
            "people_json",
            "podcast_value_json",
            "is_subscribed",
            "last_fetched_at",
            "extra_json",
            "musicindex_updated_at",
            "album_artist",
        ],
    ),
    (
        "tracks",
        &[
            "id",
            "feed_id",
            "item_guid",
            "enclosure_url",
            "enclosure_type",
            "link",
            "pub_date",
            "track_title",
            "artist_name",
            "album_title",
            "album_artist_name",
            "disc_number",
            "track_number",
            "duration_seconds",
            "itunes_duration_raw",
            "itunes_explicit",
            "track_image_href",
            "track_image_mime",
            "people_json",
            "item_value_json",
            "is_in_library",
            "extra_json",
            "removed_from_feed_at",
            "removed_from_feed_confirmed_at",
        ],
    ),
];

/// Migration 16 drops these migration-12 tables (ADR 0076 supersedes ADR
/// 0075 Decisions F, G and H). No production code writes or reads them.
pub(crate) const SUPERSEDED_SELECTION_TABLES: [&str; 3] = [
    "metadata_discrepancy_transitions",
    "metadata_discrepancies",
    "metadata_field_selections",
];

const DDL: &str = r"
CREATE TABLE rss_field_holds (
    owner_kind TEXT NOT NULL CHECK (owner_kind IN ('feed', 'track')),
    feed_id INTEGER NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
    track_id INTEGER NULL REFERENCES tracks(id) ON DELETE CASCADE,
    field TEXT NOT NULL CHECK (field != ''),
    rss_value_json TEXT NULL CHECK (rss_value_json IS NULL OR json_valid(rss_value_json)),
    run_id INTEGER NULL REFERENCES rss_check_runs(id) ON DELETE SET NULL,
    checked_at_us INTEGER NOT NULL,
    CHECK ((owner_kind = 'feed' AND track_id IS NULL) OR (owner_kind = 'track' AND track_id IS NOT NULL))
);

CREATE UNIQUE INDEX rss_field_holds_owner_field
    ON rss_field_holds(owner_kind, feed_id, ifnull(track_id, 0), field);

CREATE TABLE rss_check_differences (
    id INTEGER NOT NULL PRIMARY KEY,
    run_id INTEGER NOT NULL REFERENCES rss_check_runs(id) ON DELETE CASCADE,
    feed_id INTEGER NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
    track_id INTEGER NULL REFERENCES tracks(id) ON DELETE CASCADE,
    field TEXT NOT NULL CHECK (field != ''),
    kind TEXT NOT NULL CHECK (kind IN ('changed', 'cleared', 'track_added', 'track_removed', 'track_returned')),
    old_value_json TEXT NULL CHECK (old_value_json IS NULL OR json_valid(old_value_json)),
    new_value_json TEXT NULL CHECK (new_value_json IS NULL OR json_valid(new_value_json)),
    recorded_at_us INTEGER NOT NULL
);

CREATE INDEX rss_check_differences_run ON rss_check_differences(run_id, id);

ALTER TABLE tracks ADD COLUMN removed_from_feed_at INTEGER NULL;
ALTER TABLE tracks ADD COLUMN removed_from_feed_confirmed_at INTEGER NULL;
ALTER TABLE feeds ADD COLUMN album_artist TEXT NULL;
";

/// The result of the `feeds.album_artist` fill of migration 16.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AlbumArtistFill {
    /// Feeds whose tracks all hold one equal `album_artist_name`.
    pub(crate) filled: i64,
    /// Feeds that stay null.
    pub(crate) left_null: i64,
}

/// Migration 16 (ADR 0016 registry, ADR 0076 packet 002).
pub(super) fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch(DDL)
        .context("Create RSS field hold and difference tables")?;
    let fill = fill_album_artist(conn)?;
    // The migration registry returns no value, so the count goes to the
    // error stream with the other startup messages.
    eprintln!(
        "Migration 16 set the album artist of {} feeds from their tracks. {} feeds have no single album artist in their tracks, and their album artist stays empty.",
        fill.filled, fill.left_null
    );
    // The two discrepancy tables refer to each other. Clear the reference
    // first, so the implicit delete of each drop meets no foreign key.
    conn.execute_batch("UPDATE metadata_discrepancies SET latest_transition_id = NULL")
        .context("Clear discrepancy transition references")?;
    for table in SUPERSEDED_SELECTION_TABLES {
        conn.execute_batch(&format!("DROP TABLE {table}"))
            .with_context(|| format!("drop table {table}"))?;
    }
    Ok(())
}

/// Fill `feeds.album_artist` for each feed whose tracks all hold one equal,
/// non-empty `album_artist_name`. `tracks.album_artist_name` stays unchanged.
pub(crate) fn fill_album_artist(conn: &Connection) -> Result<AlbumArtistFill> {
    let filled = conn
        .execute(
            "UPDATE feeds SET album_artist = (
                 SELECT min(t.album_artist_name) FROM tracks t WHERE t.feed_id = feeds.id
             )
             WHERE album_artist IS NULL
               AND EXISTS (SELECT 1 FROM tracks t WHERE t.feed_id = feeds.id)
               AND NOT EXISTS (
                   SELECT 1 FROM tracks t
                   WHERE t.feed_id = feeds.id
                     AND (t.album_artist_name IS NULL OR trim(t.album_artist_name) = '')
               )
               AND (SELECT count(DISTINCT t.album_artist_name) FROM tracks t WHERE t.feed_id = feeds.id) = 1",
            [],
        )
        .context("fill feeds.album_artist")?;
    let left_null: i64 = conn
        .query_row(
            "SELECT count(*) FROM feeds WHERE album_artist IS NULL",
            [],
            |row| row.get(0),
        )
        .context("count feeds without album_artist")?;
    Ok(AlbumArtistFill {
        filled: i64::try_from(filled).unwrap_or(i64::MAX),
        left_null,
    })
}

/// One compared element of the playlist RSS check (ADR 0076 Decision 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RssField {
    Title,
    Description,
    Artwork,
    Link,
    Enclosure,
    Duration,
    Date,
    Explicit,
    Language,
    AlbumArtist,
    Artist,
    Owner,
    Persons,
    Nostr,
    PaymentRoutes,
    Publisher,
    /// The item list: an added, removed or returned track.
    Track,
}

impl RssField {
    /// The stored token of the field.
    #[must_use]
    pub fn token(self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Description => "description",
            Self::Artwork => "artwork",
            Self::Link => "link",
            Self::Enclosure => "enclosure",
            Self::Duration => "duration",
            Self::Date => "date",
            Self::Explicit => "explicit",
            Self::Language => "language",
            Self::AlbumArtist => "album_artist",
            Self::Artist => "artist",
            Self::Owner => "owner",
            Self::Persons => "persons",
            Self::Nostr => "nostr",
            Self::PaymentRoutes => "payment_routes",
            Self::Publisher => "publisher",
            Self::Track => "track",
        }
    }

    /// The field of a stored token.
    #[must_use]
    pub fn from_token(token: &str) -> Option<Self> {
        [
            Self::Title,
            Self::Description,
            Self::Artwork,
            Self::Link,
            Self::Enclosure,
            Self::Duration,
            Self::Date,
            Self::Explicit,
            Self::Language,
            Self::AlbumArtist,
            Self::Artist,
            Self::Owner,
            Self::Persons,
            Self::Nostr,
            Self::PaymentRoutes,
            Self::Publisher,
            Self::Track,
        ]
        .into_iter()
        .find(|field| field.token() == token)
    }

    /// The comparison rule of the field. `rss` is the RSS value. `other`
    /// is a stored or `MusicIndex` value with its description representation.
    #[must_use]
    pub(crate) fn values_equal(
        self,
        rss: Option<&Value>,
        other: Option<&Value>,
        other_representation: TextRepresentation,
    ) -> bool {
        let rss = rss.filter(|value| !value.is_null());
        let other = other.filter(|value| !value.is_null());
        let text = |value: Option<&Value>| value.and_then(Value::as_str).map(str::to_owned);
        match self {
            Self::Title | Self::AlbumArtist | Self::Artist | Self::Owner => {
                compare::text_equal(text(rss).as_deref(), text(other).as_deref())
            }
            Self::Description => compare::description_equal(
                text(rss).as_deref(),
                TextRepresentation::Html,
                text(other).as_deref(),
                other_representation,
            ),
            Self::Artwork | Self::Link => {
                compare::url_equal(text(rss).as_deref(), text(other).as_deref())
            }
            Self::Language => compare::language_equal(text(rss).as_deref(), text(other).as_deref()),
            Self::Explicit => rss.and_then(Value::as_bool) == other.and_then(Value::as_bool),
            Self::Enclosure => {
                let part = |value: Option<&Value>, key: &str| {
                    value
                        .and_then(|value| value[key].as_str())
                        .map(str::to_owned)
                };
                compare::url_equal(part(rss, "url").as_deref(), part(other, "url").as_deref())
                    && compare::trimmed(part(rss, "type").as_deref())
                        == compare::trimmed(part(other, "type").as_deref())
            }
            Self::Duration => {
                let seconds =
                    |value: Option<&Value>| value.and_then(|value| value["seconds"].as_i64());
                seconds(rss) == seconds(other)
            }
            Self::Date => {
                let instant =
                    |value: Option<&Value>| value.and_then(|value| value["instant"].as_i64());
                match (instant(rss), instant(other)) {
                    (None, None) => {
                        let raw = |value: Option<&Value>| {
                            value
                                .and_then(|value| value["text"].as_str())
                                .map(str::to_owned)
                        };
                        compare::text_equal(raw(rss).as_deref(), raw(other).as_deref())
                    }
                    (left, right) => left == right,
                }
            }
            Self::Persons => persons_equal(rss, other),
            Self::Nostr => nostr_equal(rss, other),
            Self::PaymentRoutes => compare::value_routes_equal(
                rss.map(Value::to_string).as_deref(),
                other.map(Value::to_string).as_deref(),
            ),
            Self::Publisher => publisher_equal(rss, other),
            Self::Track => rss == other,
        }
    }
}

/// The JSON form of a credit list for the persons slot. An empty list gives
/// no value. The playlist RSS check, the subscribe and the `MusicIndex`
/// credit writer use this one form (ADR 0076 packets 002 and 006).
#[must_use]
pub(crate) fn persons_value(contributors: &[LocalContributorInput]) -> Option<Value> {
    (!contributors.is_empty()).then(|| {
        Value::Array(
            contributors
                .iter()
                .map(|contributor| {
                    ComparedPerson {
                        name: contributor.name.clone(),
                        role: contributor.role.clone(),
                        group: contributor.group_name.clone(),
                        href: contributor.href.clone(),
                        image: contributor.image_url.clone(),
                    }
                    .to_json()
                })
                .collect(),
        )
    })
}

fn persons_equal(rss: Option<&Value>, other: Option<&Value>) -> bool {
    let persons = |value: Option<&Value>| {
        value
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .map(ComparedPerson::from_json)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    compare::persons_equal(&persons(rss), &persons(other))
}

fn nostr_equal(rss: Option<&Value>, other: Option<&Value>) -> bool {
    let ids = |value: Option<&Value>| {
        value
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(|value| {
                        Some(format!(
                            "{} {}",
                            value["scheme"].as_str()?,
                            value["value"].as_str()?.trim()
                        ))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    compare::identities_equal(&ids(rss), &ids(other))
}

fn publisher_equal(rss: Option<&Value>, other: Option<&Value>) -> bool {
    let part = |value: Option<&Value>, key: &str| {
        value
            .and_then(|value| value[key].as_str())
            .map(str::to_owned)
    };
    compare::text_equal(
        part(rss, "feed_guid").as_deref(),
        part(other, "feed_guid").as_deref(),
    ) && compare::url_equal(
        part(rss, "feed_url").as_deref(),
        part(other, "feed_url").as_deref(),
    )
}

/// The owner of a compared value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoldOwner {
    Feed(i64),
    Track { feed_id: i64, track_id: i64 },
}

impl HoldOwner {
    fn parts(self) -> (&'static str, i64, Option<i64>) {
        match self {
            Self::Feed(feed_id) => ("feed", feed_id, None),
            Self::Track { feed_id, track_id } => ("track", feed_id, Some(track_id)),
        }
    }
}

/// One stored hold.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredHold {
    /// The held RSS value. `None` is a cleared field.
    pub(crate) value: Option<Value>,
    pub(crate) run_id: Option<i64>,
    pub(crate) checked_at_us: i64,
}

/// Insert or replace the hold of one owner and field.
///
/// # Errors
///
/// Returns an error when the database write fails.
pub(crate) fn write_hold(
    conn: &Connection,
    owner: HoldOwner,
    field: RssField,
    value: Option<&Value>,
    run_id: Option<i64>,
    checked_at_us: i64,
) -> Result<()> {
    delete_hold(conn, owner, field)?;
    let (kind, feed_id, track_id) = owner.parts();
    conn.execute(
        "INSERT INTO rss_field_holds(owner_kind, feed_id, track_id, field, rss_value_json, run_id, checked_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            kind,
            feed_id,
            track_id,
            field.token(),
            value.filter(|value| !value.is_null()).map(Value::to_string),
            run_id,
            checked_at_us
        ],
    )
    .context("insert rss_field_holds")?;
    Ok(())
}

/// Read the hold of one owner and field.
///
/// # Errors
///
/// Returns an error when the database query fails.
pub(crate) fn hold(
    conn: &Connection,
    owner: HoldOwner,
    field: RssField,
) -> Result<Option<StoredHold>> {
    let (kind, feed_id, track_id) = owner.parts();
    conn.query_row(
        "SELECT rss_value_json, run_id, checked_at_us FROM rss_field_holds
         WHERE owner_kind = ?1 AND feed_id = ?2 AND track_id IS ?3 AND field = ?4",
        params![kind, feed_id, track_id, field.token()],
        |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, i64>(2)?,
            ))
        },
    )
    .optional()
    .context("query rss_field_holds")?
    .map(|(value, run_id, checked_at_us)| {
        Ok(StoredHold {
            value: value
                .map(|value| serde_json::from_str(&value))
                .transpose()
                .context("decode rss_field_holds value")?,
            run_id,
            checked_at_us,
        })
    })
    .transpose()
}

/// Read each hold of one owner in one query. The map value is the held RSS
/// value. `None` is a cleared field. The stored value projection of ADR 0075
/// packet 020 reads holds through this function.
///
/// # Errors
///
/// Returns an error when the database query fails or a stored value does
/// not decode.
pub(crate) fn owner_holds(
    conn: &Connection,
    owner: HoldOwner,
) -> Result<std::collections::BTreeMap<RssField, Option<Value>>> {
    let (kind, feed_id, track_id) = owner.parts();
    let mut statement = conn
        .prepare(
            "SELECT field, rss_value_json FROM rss_field_holds
             WHERE owner_kind = ?1 AND feed_id = ?2 AND track_id IS ?3",
        )
        .context("prepare owner rss_field_holds")?;
    let rows = statement
        .query_map(params![kind, feed_id, track_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .context("query owner rss_field_holds")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("collect owner rss_field_holds")?;
    let mut holds = std::collections::BTreeMap::new();
    for (token, value) in rows {
        let Some(field) = RssField::from_token(&token) else {
            continue;
        };
        let value = value
            .map(|value| serde_json::from_str(&value))
            .transpose()
            .context("decode rss_field_holds value")?;
        holds.insert(field, value);
    }
    Ok(holds)
}

/// Delete the hold of one owner and field.
///
/// # Errors
///
/// Returns an error when the database write fails.
pub(crate) fn delete_hold(conn: &Connection, owner: HoldOwner, field: RssField) -> Result<()> {
    let (kind, feed_id, track_id) = owner.parts();
    conn.execute(
        "DELETE FROM rss_field_holds
         WHERE owner_kind = ?1 AND feed_id = ?2 AND track_id IS ?3 AND field = ?4",
        params![kind, feed_id, track_id, field.token()],
    )
    .context("delete rss_field_holds")?;
    Ok(())
}

/// The decision of the `MusicIndex` gate for one slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GateDecision {
    /// Write the slot.
    Write,
    /// Keep the held RSS value in the slot. The `MusicIndex` fact row is
    /// still written as evidence.
    Skip,
}

impl GateDecision {
    #[must_use]
    pub(crate) fn writes(self) -> bool {
        self == Self::Write
    }
}

/// One value that a `MusicIndex` response supplies for a compared slot.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MusicIndexClaim {
    pub(crate) owner: HoldOwner,
    pub(crate) field: RssField,
    /// The `MusicIndex` value in the JSON form of the field.
    pub(crate) value: Option<Value>,
    /// The `updated_at` of the response, in Unix seconds. Stophammer sets
    /// it when it ingests a changed RSS document of the feed.
    pub(crate) updated_at: Option<i64>,
}

impl MusicIndexClaim {
    /// A feed description of a `MusicIndex` feed response.
    #[must_use]
    pub(crate) fn feed_description(
        feed_id: i64,
        description: Option<&str>,
        updated_at: Option<i64>,
    ) -> Self {
        Self {
            owner: HoldOwner::Feed(feed_id),
            field: RssField::Description,
            value: description.map(|value| Value::String(value.to_owned())),
            updated_at,
        }
    }
}

/// ADR 0076 Decision 5: the one gate that each `MusicIndex` writer of a
/// compared slot calls before it writes the slot.
///
/// - No hold: write.
/// - A hold, and the `MusicIndex` value equals the held value: delete the
///   hold, and write.
/// - A hold, and the response `updated_at` is after the check time of the
///   hold: delete the hold, and write.
/// - Otherwise: skip the slot.
///
/// # Errors
///
/// Returns an error when the database query or write fails.
pub(crate) fn musicindex_gate(conn: &Connection, claim: MusicIndexClaim) -> Result<GateDecision> {
    let MusicIndexClaim {
        owner,
        field,
        value,
        updated_at,
    } = claim;
    let Some(held) = hold(conn, owner, field)? else {
        return Ok(GateDecision::Write);
    };
    let agrees = field.values_equal(
        held.value.as_ref(),
        value.as_ref(),
        TextRepresentation::PlainText,
    );
    let newer = updated_at
        .and_then(|seconds| seconds.checked_mul(1_000_000))
        .is_some_and(|updated_at_us| updated_at_us > held.checked_at_us);
    if agrees || newer {
        delete_hold(conn, owner, field)?;
        Ok(GateDecision::Write)
    } else {
        Ok(GateDecision::Skip)
    }
}

/// The kind of one recorded difference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DifferenceKind {
    Changed,
    Cleared,
    TrackAdded,
    TrackRemoved,
    TrackReturned,
}

impl DifferenceKind {
    #[must_use]
    pub fn token(self) -> &'static str {
        match self {
            Self::Changed => "changed",
            Self::Cleared => "cleared",
            Self::TrackAdded => "track_added",
            Self::TrackRemoved => "track_removed",
            Self::TrackReturned => "track_returned",
        }
    }

    fn from_token(token: &str) -> Option<Self> {
        match token {
            "changed" => Some(Self::Changed),
            "cleared" => Some(Self::Cleared),
            "track_added" => Some(Self::TrackAdded),
            "track_removed" => Some(Self::TrackRemoved),
            "track_returned" => Some(Self::TrackReturned),
            _ => None,
        }
    }
}

/// One difference to record.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NewDifference {
    pub(crate) feed_id: i64,
    pub(crate) track_id: Option<i64>,
    pub(crate) field: RssField,
    pub(crate) kind: DifferenceKind,
    pub(crate) old_value: Option<Value>,
    pub(crate) new_value: Option<Value>,
}

/// One stored difference with the names of its feed and track.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredDifference {
    pub id: i64,
    pub run_id: i64,
    pub feed_id: i64,
    pub feed_title: Option<String>,
    pub feed_url: Option<String>,
    pub track_id: Option<i64>,
    pub track_title: Option<String>,
    pub field: RssField,
    pub kind: DifferenceKind,
    pub old_value: Option<Value>,
    pub new_value: Option<Value>,
    pub recorded_at_us: i64,
}

/// Insert one difference row.
///
/// # Errors
///
/// Returns an error when the database write fails.
pub(crate) fn insert_difference(
    conn: &Connection,
    run_id: i64,
    difference: &NewDifference,
    recorded_at_us: i64,
) -> Result<()> {
    conn.execute(
        "INSERT INTO rss_check_differences(run_id, feed_id, track_id, field, kind, old_value_json, new_value_json, recorded_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            run_id,
            difference.feed_id,
            difference.track_id,
            difference.field.token(),
            difference.kind.token(),
            difference
                .old_value
                .as_ref()
                .filter(|value| !value.is_null())
                .map(Value::to_string),
            difference
                .new_value
                .as_ref()
                .filter(|value| !value.is_null())
                .map(Value::to_string),
            recorded_at_us
        ],
    )
    .context("insert rss_check_differences")?;
    Ok(())
}

/// Read the differences of one run, optionally for one feed, in record order.
///
/// # Errors
///
/// Returns an error when the database query fails.
pub fn run_differences(
    conn: &Connection,
    run_id: i64,
    feed_id: Option<i64>,
) -> Result<Vec<StoredDifference>> {
    let mut statement = conn
        .prepare(
            "SELECT d.id, d.run_id, d.feed_id, f.title, f.feed_url, d.track_id, t.track_title,
                    d.field, d.kind, d.old_value_json, d.new_value_json, d.recorded_at_us
             FROM rss_check_differences d
             LEFT JOIN feeds f ON f.id = d.feed_id
             LEFT JOIN tracks t ON t.id = d.track_id
             WHERE d.run_id = ?1 AND (?2 IS NULL OR d.feed_id = ?2)
             ORDER BY d.id",
        )
        .context("prepare rss_check_differences")?;
    let rows = statement
        .query_map(params![run_id, feed_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<i64>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, i64>(11)?,
            ))
        })
        .context("query rss_check_differences")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("collect rss_check_differences")?;
    rows.into_iter()
        .map(
            |(
                id,
                run_id,
                feed_id,
                feed_title,
                feed_url,
                track_id,
                track_title,
                field,
                kind,
                old_value,
                new_value,
                recorded_at_us,
            )| {
                let decode = |value: Option<String>| {
                    value
                        .map(|value| serde_json::from_str::<Value>(&value))
                        .transpose()
                        .context("decode rss_check_differences value")
                };
                Ok(StoredDifference {
                    id,
                    run_id,
                    feed_id,
                    feed_title,
                    feed_url,
                    track_id,
                    track_title,
                    field: RssField::from_token(&field)
                        .with_context(|| format!("unknown RSS check field {field:?}"))?,
                    kind: DifferenceKind::from_token(&kind)
                        .with_context(|| format!("unknown RSS check difference {kind:?}"))?,
                    old_value: decode(old_value)?,
                    new_value: decode(new_value)?,
                    recorded_at_us,
                })
            },
        )
        .collect()
}

/// The tracks of a playlist with an unconfirmed "removed from feed" mark,
/// with the check time of each mark (ADR 0076 Decision 7). A confirmed track
/// is ready for the show (packet 003), so its row shows no error.
///
/// # Errors
///
/// Returns an error when the database query fails.
pub fn playlist_removed_marks(conn: &Connection, playlist_id: i64) -> Result<Vec<(i64, i64)>> {
    let mut statement = conn
        .prepare(
            "SELECT DISTINCT t.id, t.removed_from_feed_at
             FROM playlist_tracks pt
             JOIN tracks t ON t.id = pt.track_id
             WHERE pt.playlist_id = ?1 AND t.removed_from_feed_at IS NOT NULL
               AND t.removed_from_feed_confirmed_at IS NULL
             ORDER BY t.id",
        )
        .context("prepare playlist_removed_marks")?;
    let rows = statement
        .query_map([playlist_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .context("query playlist_removed_marks")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("collect playlist_removed_marks")?;
    Ok(rows)
}

/// The check time of the "removed from feed" mark of a track, when the
/// operator has not confirmed it (ADR 0076 Decision 7, packet 003).
///
/// # Errors
///
/// Returns an error when the database query fails.
pub(crate) fn unconfirmed_removed_at(conn: &Connection, track_id: i64) -> Result<Option<i64>> {
    conn.query_row(
        "SELECT removed_from_feed_at FROM tracks
         WHERE id = ?1
           AND removed_from_feed_at IS NOT NULL
           AND removed_from_feed_confirmed_at IS NULL",
        [track_id],
        |row| row.get(0),
    )
    .optional()
    .context("read the removed from feed mark")
}

/// Records that the operator confirmed a removed track for the show. The
/// function changes only a track with an unconfirmed mark, and it returns
/// whether it changed the track.
///
/// # Errors
///
/// Returns an error when the update fails.
pub(crate) fn confirm_removed_track(
    conn: &Connection,
    track_id: i64,
    confirmed_at_us: i64,
) -> Result<bool> {
    let changed = conn
        .execute(
            "UPDATE tracks SET removed_from_feed_confirmed_at = ?2
             WHERE id = ?1
               AND removed_from_feed_at IS NOT NULL
               AND removed_from_feed_confirmed_at IS NULL",
            params![track_id, confirmed_at_us],
        )
        .context("confirm the removed track")?;
    Ok(changed > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{inspect_schema, upgrades, SchemaCompatibility, CURRENT_VERSION, MIGRATIONS};

    fn table_exists(conn: &Connection, table: &str) -> bool {
        conn.query_row(
            "SELECT count(*) FROM sqlite_schema WHERE type = 'table' AND name = ?1",
            [table],
            |row| row.get::<_, i64>(0),
        )
        .unwrap()
            == 1
    }

    fn version_15_with_rows() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        upgrades::create_fixture(&conn, 15).unwrap();
        conn.execute_batch(
            "INSERT INTO feeds(id, feed_url, feed_guid, title) VALUES
                 (1, 'https://example.test/one.xml', 'f1', 'One'),
                 (2, 'https://example.test/two.xml', 'f2', 'Two'),
                 (3, 'https://example.test/three.xml', 'f3', 'Three');
             INSERT INTO tracks(id, feed_id, item_guid, track_title, album_artist_name, artist_name, is_in_library) VALUES
                 (1, 1, 'a', 'A', 'Band', 'Band', 1),
                 (2, 1, 'b', 'B', 'Band', 'Guest', 0),
                 (3, 2, 'c', 'C', 'Band', 'Band', 0),
                 (4, 2, 'd', 'D', 'Other', 'Other', 0);
             INSERT INTO playlists(id, name) VALUES (1, 'Show');
             INSERT INTO playlist_tracks(playlist_id, track_id, position) VALUES (1, 1, 0), (1, 3, 1);",
        )
        .unwrap();
        conn.execute_batch(crate::db::provider_snapshot_schema::RETAINED_ROWS)
            .unwrap();
        conn.execute_batch(crate::db::provider_snapshot_schema::SUPERSEDED_ROWS)
            .unwrap();
        conn
    }

    fn track_values(conn: &Connection) -> Vec<Vec<rusqlite::types::Value>> {
        let columns = crate::db::schema_contract(15)
            .find(|(table, _)| *table == "tracks")
            .unwrap()
            .1
            .join(", ");
        let mut statement = conn
            .prepare(&format!("SELECT {columns} FROM tracks ORDER BY id"))
            .unwrap();
        let count = statement.column_count();
        statement
            .query_map([], |row| {
                (0..count)
                    .map(|index| row.get::<_, rusqlite::types::Value>(index))
                    .collect()
            })
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    /// R2-17: a version 15 database migrates to version 16.
    #[test]
    fn adr_0076_rss_comparison_version_15_migrates_to_16() {
        let conn = version_15_with_rows();
        assert_eq!(
            inspect_schema(&conn).unwrap(),
            SchemaCompatibility::UpgradeRequired {
                applied: 15,
                current: MIGRATIONS.len()
            }
        );
        let before = track_values(&conn);
        let retained = upgrades::retained_digest(&conn).unwrap();

        crate::db::migrate_schema_to(&conn, 16).unwrap();

        // ADR 0076 packet 003: migration 17 follows, so version 16 is an
        // upgrade-required version after this migration.
        assert!(CURRENT_VERSION >= 16);
        for table in SUPERSEDED_SELECTION_TABLES {
            assert!(!table_exists(&conn, table), "{table} is dropped");
        }
        for table in ["rss_field_holds", "rss_check_differences"] {
            assert!(table_exists(&conn, table), "{table} exists");
            let count: i64 = conn
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, 0, "{table} is empty");
        }
        assert_eq!(track_values(&conn), before);
        assert_eq!(upgrades::retained_digest(&conn).unwrap(), retained);
        let marks: i64 = conn
            .query_row(
                "SELECT count(*) FROM tracks WHERE removed_from_feed_at IS NOT NULL OR removed_from_feed_confirmed_at IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(marks, 0);
        let name: String = conn
            .query_row(
                "SELECT name FROM schema_migrations WHERE version = 16",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(name, "rss_field_holds_and_differences");
        assert_eq!(
            inspect_schema(&conn).unwrap(),
            SchemaCompatibility::UpgradeRequired {
                applied: 16,
                current: MIGRATIONS.len()
            }
        );
        upgrades::verify_target(&conn, 16).unwrap();
        for (table, columns) in COLUMNS {
            let names = conn
                .prepare(&format!("SELECT * FROM {table}"))
                .unwrap()
                .column_names()
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>();
            assert_eq!(names, *columns);
        }
    }

    /// R2-21: the migration fills `feeds.album_artist` for a feed whose
    /// tracks share one value, and leaves it null for a feed with two.
    #[test]
    fn adr_0076_rss_comparison_migration_fills_album_artist() {
        let conn = version_15_with_rows();
        crate::db::migrate_schema_to(&conn, 16).unwrap();
        let album_artist = |feed_id: i64| -> Option<String> {
            conn.query_row(
                "SELECT album_artist FROM feeds WHERE id = ?1",
                [feed_id],
                |row| row.get(0),
            )
            .unwrap()
        };
        assert_eq!(album_artist(1).as_deref(), Some("Band"));
        assert_eq!(album_artist(2), None);
        assert_eq!(album_artist(3), None);
        let unchanged: i64 = conn
            .query_row(
                "SELECT count(*) FROM tracks WHERE album_artist_name IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(unchanged, 4);
        // The fill of a migrated database finds nothing more to fill.
        assert_eq!(
            fill_album_artist(&conn).unwrap(),
            AlbumArtistFill {
                filled: 0,
                left_null: 2
            }
        );
    }

    #[test]
    fn adr_0076_rss_comparison_failed_migration_16_leaves_version_15() {
        let conn = version_15_with_rows();
        let result = crate::db::migrate_schema_with(&conn, 16, |version, reached| {
            anyhow::ensure!(
                version != 16 || reached != crate::db::MigrationBoundary::AfterRecord,
                "fixture stop after migration 16"
            );
            Ok(())
        });
        assert!(result.is_err());
        upgrades::verify_target(&conn, 15).unwrap();
        for table in SUPERSEDED_SELECTION_TABLES {
            assert!(table_exists(&conn, table), "{table} stays");
        }
    }

    const CHECKED_AT_US: i64 = 1_790_000_000_000_000;

    /// A feed with a held RSS description, as a check writes it.
    fn held_description() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        upgrades::create_fixture(&conn, 16).unwrap();
        conn.execute_batch(
            "INSERT INTO feeds(id, feed_url, feed_guid, description) VALUES (1, 'https://example.test/feed.xml', 'f1', '<p>RSS notes</p>');",
        )
        .unwrap();
        write_hold(
            &conn,
            HoldOwner::Feed(1),
            RssField::Description,
            Some(&Value::from("<p>RSS notes</p>")),
            None,
            CHECKED_AT_US,
        )
        .unwrap();
        conn
    }

    /// The `MusicIndex` feed writers of a description: the gate, the slot
    /// write of `feed_service::apply_feed_updates`, and the fact write.
    fn musicindex_feed_update(conn: &mut Connection, description: &str, updated_at: i64) {
        if musicindex_gate(
            conn,
            MusicIndexClaim::feed_description(1, Some(description), Some(updated_at)),
        )
        .unwrap()
        .writes()
        {
            crate::db::set_feed_description(conn, 1, Some(description)).unwrap();
        }
        let feed = crate::api::Feed {
            feed_guid: Some("f1".into()),
            description: Some(description.into()),
            updated_at: Some(updated_at),
            ..crate::api::Feed::default()
        };
        crate::identity_ingest::persist_musicindex_feed(conn, 1, &feed).unwrap();
    }

    fn description(conn: &Connection) -> String {
        conn.query_row("SELECT description FROM feeds WHERE id = 1", [], |row| {
            row.get(0)
        })
        .unwrap()
    }

    fn musicindex_description(conn: &Connection) -> Option<crate::db::LocalMetadataValue> {
        crate::db::local_metadata_fact(
            conn,
            crate::db::LocalMetadataOwner::Feed(1),
            "musicindex",
            "description",
        )
        .unwrap()
        .map(|fact| fact.value)
    }

    /// R2-09: with a hold, a different `MusicIndex` value with an older
    /// `updated_at` does not write the slot. The `MusicIndex` fact row is
    /// written.
    #[test]
    fn adr_0076_rss_comparison_hold_keeps_slot_against_older_musicindex() {
        let mut conn = held_description();
        musicindex_feed_update(&mut conn, "Stale notes", CHECKED_AT_US / 1_000_000 - 60);
        assert_eq!(description(&conn), "<p>RSS notes</p>");
        assert_eq!(
            musicindex_description(&conn),
            Some(crate::db::LocalMetadataValue::Text("Stale notes".into()))
        );
        assert!(hold(&conn, HoldOwner::Feed(1), RssField::Description)
            .unwrap()
            .is_some());
    }

    /// R2-10: with a hold, an equal `MusicIndex` value writes the slot and
    /// deletes the hold.
    #[test]
    fn adr_0076_rss_comparison_equal_musicindex_value_releases_hold() {
        let mut conn = held_description();
        musicindex_feed_update(&mut conn, "RSS notes", CHECKED_AT_US / 1_000_000 - 60);
        assert_eq!(description(&conn), "RSS notes");
        assert!(hold(&conn, HoldOwner::Feed(1), RssField::Description)
            .unwrap()
            .is_none());
    }

    /// R2-11: with a hold, a different `MusicIndex` value with a newer
    /// `updated_at` writes the slot and deletes the hold.
    #[test]
    fn adr_0076_rss_comparison_newer_musicindex_record_releases_hold() {
        let mut conn = held_description();
        musicindex_feed_update(&mut conn, "Newer notes", CHECKED_AT_US / 1_000_000 + 1);
        assert_eq!(description(&conn), "Newer notes");
        assert!(hold(&conn, HoldOwner::Feed(1), RssField::Description)
            .unwrap()
            .is_none());
    }

    #[test]
    fn adr_0076_rss_comparison_gate_skips_publisher_and_nostr_rows() {
        let mut conn = held_description();
        write_hold(
            &conn,
            HoldOwner::Feed(1),
            RssField::Nostr,
            None,
            None,
            CHECKED_AT_US,
        )
        .unwrap();
        let feed = crate::api::Feed {
            feed_guid: Some("f1".into()),
            updated_at: Some(CHECKED_AT_US / 1_000_000 - 60),
            source_ids: Some(vec![crate::api::SourceEntityId {
                entity_type: Some("feed".into()),
                scheme: Some("nostr_npub".into()),
                value: Some("npub1stale".into()),
                source: Some("rss".into()),
                ..crate::api::SourceEntityId::default()
            }]),
            ..crate::api::Feed::default()
        };
        crate::identity_ingest::persist_musicindex_feed(&mut conn, 1, &feed).unwrap();
        assert!(
            crate::db::local_identity_ids(&conn, crate::db::LocalIdentityOwner::Feed(1))
                .unwrap()
                .iter()
                .all(|row| row.value.as_deref() != Some("npub1stale"))
        );
        assert!(hold(&conn, HoldOwner::Feed(1), RssField::Nostr)
            .unwrap()
            .is_some());
    }

    #[test]
    fn adr_0076_rss_comparison_hold_and_difference_constraints_hold() {
        let conn = Connection::open_in_memory().unwrap();
        upgrades::create_fixture(&conn, 16).unwrap();
        conn.execute_batch(
            "INSERT INTO feeds(id, feed_url) VALUES (1, 'https://example.test/feed.xml');
             INSERT INTO tracks(id, feed_id, item_guid) VALUES (1, 1, 'a');
             INSERT INTO playlists(id, name) VALUES (1, 'Show');
             INSERT INTO rss_check_runs(id, playlist_id, trigger, started_at_us) VALUES (1, 1, 'button', 10);",
        )
        .unwrap();
        let owner = HoldOwner::Track {
            feed_id: 1,
            track_id: 1,
        };
        write_hold(
            &conn,
            owner,
            RssField::Title,
            Some(&Value::from("New")),
            Some(1),
            20,
        )
        .unwrap();
        write_hold(&conn, owner, RssField::Title, None, Some(1), 30).unwrap();
        let stored = hold(&conn, owner, RssField::Title).unwrap().unwrap();
        assert_eq!(stored.value, None);
        assert_eq!(stored.checked_at_us, 30);
        for invalid in [
            "INSERT INTO rss_field_holds(owner_kind, feed_id, track_id, field, checked_at_us) VALUES ('feed', 1, 1, 'title', 1)",
            "INSERT INTO rss_field_holds(owner_kind, feed_id, track_id, field, checked_at_us) VALUES ('track', 1, 1, 'title', 1)",
            "INSERT INTO rss_field_holds(owner_kind, feed_id, field, rss_value_json, checked_at_us) VALUES ('feed', 1, 'title', 'not json', 1)",
            "INSERT INTO rss_check_differences(run_id, feed_id, field, kind, recorded_at_us) VALUES (1, 1, 'title', 'other', 1)",
            "INSERT INTO rss_check_differences(run_id, feed_id, field, kind, recorded_at_us) VALUES (9, 1, 'title', 'changed', 1)",
        ] {
            assert!(
                conn.execute_batch(invalid).is_err(),
                "accepted invalid SQL: {invalid}"
            );
        }
    }
}
