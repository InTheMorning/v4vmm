//! Comparison and automatic apply of one checked RSS document
//! (ADR 0076 Decisions 3, 4, 6 and 7, packet 002).
//!
//! The playlist RSS check calls [`apply_checked_document`] for each RSS
//! document that it receives. The function parses the document with the
//! parse step of the subscribe command. It compares each element of the
//! element table with the stored slot that display and tag frames read.
//! For each difference it writes the RSS value to the slot, writes the
//! `rss` fact row of a fact-backed field, writes the hold of the field and
//! records the difference. All writes for one feed use one transaction.
//!
//! A parse failure writes nothing. The check writes no audio tag
//! (ADR 0076 Decision 8).

#![warn(clippy::pedantic)]

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde_json::{json, Value};

use super::compare::{self, TextRepresentation};
use super::subscribe::{
    parse_feed_document, upsert_item_columns, ParsedChannel, ParsedFeedDocument, ParsedItem,
    ParsedNostrId,
};
use crate::application::queries::stored_values;
use crate::db::rss_field_holds::{
    self as holds, persons_value, DifferenceKind, HoldOwner, NewDifference, RssField,
};
use crate::db::{
    self, LocalContributorInput, LocalEntityOwner, LocalIdentityIdInput, LocalIdentityOwner,
    LocalMetadataFactInput, LocalMetadataOwner, LocalMetadataValue,
};

const RSS_SOURCE: &str = "rss";
const NOSTR_SCHEMES: [&str; 2] = ["nostr_npub", "nostr_nprofile"];

/// The result of one applied document.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct AppliedDocument {
    /// The number of recorded differences.
    pub(crate) differences: usize,
}

/// Parse, compare and apply one checked RSS document of one feed.
///
/// # Errors
///
/// Returns an error when the document does not parse or a database
/// operation fails. The transaction then rolls back, so the feed keeps each
/// stored value.
pub(crate) fn apply_checked_document(
    conn: &Connection,
    run_id: i64,
    feed_id: i64,
    body: &[u8],
    checked_at_us: i64,
) -> Result<AppliedDocument> {
    let document = parse_feed_document(body)?;
    let tx = conn
        .unchecked_transaction()
        .context("begin RSS check apply")?;
    let mut apply = Apply {
        conn: &tx,
        run_id,
        feed_id,
        checked_at_us,
        differences: 0,
    };
    apply.channel(&document.channel)?;
    apply.items(&document.items)?;
    let differences = apply.differences;
    tx.commit().context("commit RSS check apply")?;
    Ok(AppliedDocument { differences })
}

struct Apply<'a> {
    conn: &'a Connection,
    run_id: i64,
    feed_id: i64,
    checked_at_us: i64,
    differences: usize,
}

/// The stored value of one slot and its description representation.
struct Stored {
    value: Option<Value>,
    representation: TextRepresentation,
}

impl Stored {
    fn rss(value: Option<Value>) -> Self {
        Self {
            value,
            representation: TextRepresentation::Html,
        }
    }
}

fn text(value: Option<&str>) -> Option<Value> {
    compare::trimmed(value).map(|value| Value::String(value.to_owned()))
}

fn enclosure_value(url: Option<&str>, mime: Option<&str>) -> Option<Value> {
    (compare::trimmed(url).is_some() || compare::trimmed(mime).is_some())
        .then(|| json!({"url": url, "type": mime}))
}

fn duration_value(seconds: Option<i64>, raw: Option<&str>) -> Option<Value> {
    (seconds.is_some() || compare::trimmed(raw).is_some())
        .then(|| json!({"seconds": seconds, "raw": raw}))
}

fn date_value(raw: Option<&str>) -> Option<Value> {
    compare::trimmed(raw).map(|raw| json!({"text": raw, "instant": compare::instant(Some(raw))}))
}

fn json_value(raw: Option<&str>) -> Option<Value> {
    compare::trimmed(raw).and_then(|raw| serde_json::from_str(raw).ok())
}

fn nostr_value(ids: &[(String, String)]) -> Option<Value> {
    (!ids.is_empty()).then(|| {
        Value::Array(
            ids.iter()
                .map(|(scheme, value)| json!({"scheme": scheme, "value": value}))
                .collect(),
        )
    })
}

fn parsed_nostr(ids: &[ParsedNostrId]) -> Vec<(String, String)> {
    ids.iter()
        .map(|id| (id.scheme.clone(), id.value.clone()))
        .collect()
}

impl Apply<'_> {
    fn feed_owner(&self) -> HoldOwner {
        HoldOwner::Feed(self.feed_id)
    }

    /// The stored value of a fact-backed slot. The stored value projection
    /// of ADR 0075 packet 020 owns the order: the held value, then the
    /// `MusicIndex` fact, then the column. A field without a column passes
    /// `None`, and the projection reads its `rss` fact row.
    fn fact_backed(
        &self,
        owner: HoldOwner,
        field: RssField,
        column: Option<Value>,
    ) -> Result<Stored> {
        let slot = stored_values::compared_slot(self.conn, owner, field, column)?;
        Ok(Stored {
            value: slot.value,
            representation: if slot.musicindex {
                TextRepresentation::PlainText
            } else {
                TextRepresentation::Html
            },
        })
    }

    /// Compare one slot. On a difference, apply the RSS value, write the
    /// hold and record the difference. Returns `true` on a difference.
    fn element(
        &mut self,
        owner: HoldOwner,
        field: RssField,
        stored: Stored,
        rss: Option<Value>,
        write: impl FnOnce(&Connection) -> Result<()>,
    ) -> Result<bool> {
        if field.values_equal(rss.as_ref(), stored.value.as_ref(), stored.representation) {
            return Ok(false);
        }
        write(self.conn)?;
        holds::write_hold(
            self.conn,
            owner,
            field,
            rss.as_ref(),
            Some(self.run_id),
            self.checked_at_us,
        )?;
        let (feed_id, track_id) = match owner {
            HoldOwner::Feed(feed_id) => (feed_id, None),
            HoldOwner::Track { feed_id, track_id } => (feed_id, Some(track_id)),
        };
        self.record(&NewDifference {
            feed_id,
            track_id,
            field,
            kind: if rss.is_some() {
                DifferenceKind::Changed
            } else {
                DifferenceKind::Cleared
            },
            old_value: stored.value,
            new_value: rss,
        })?;
        Ok(true)
    }

    fn record(&mut self, difference: &NewDifference) -> Result<()> {
        holds::insert_difference(self.conn, self.run_id, difference, self.checked_at_us)?;
        self.differences += 1;
        Ok(())
    }

    fn feed_column(&self) -> Result<FeedColumns> {
        self.conn
            .query_row(
                "SELECT title, description, album_image_href, link, language, album_artist,
                        podcast_value_json
                 FROM feeds WHERE id = ?1",
                [self.feed_id],
                |row| {
                    Ok(FeedColumns {
                        title: row.get(0)?,
                        description: row.get(1)?,
                        album_image_href: row.get(2)?,
                        link: row.get(3)?,
                        language: row.get(4)?,
                        album_artist: row.get(5)?,
                        podcast_value_json: row.get(6)?,
                    })
                },
            )
            .context("read the stored feed values")
    }

    #[allow(clippy::too_many_lines)]
    fn channel(&mut self, channel: &ParsedChannel) -> Result<()> {
        let feed_id = self.feed_id;
        let owner = self.feed_owner();
        let fact_owner = LocalMetadataOwner::Feed(feed_id);
        let stored = self.feed_column()?;

        let rss_title = text(Some(&channel.title));
        let title = channel.title.clone();
        self.element(
            owner,
            RssField::Title,
            Stored::rss(text(stored.title.as_deref())),
            rss_title,
            |conn| set_feed_column(conn, feed_id, "title", Some(&title)),
        )?;

        let description = self.fact_backed(
            owner,
            RssField::Description,
            text(stored.description.as_deref()),
        )?;
        let [slot_description, slot_explicit, slot_language, slot_owner] =
            channel_fact_slots(channel);
        let rss_description = channel.description.clone();
        self.element(
            owner,
            RssField::Description,
            description,
            slot_description.rss.clone(),
            |conn| {
                set_feed_column(conn, feed_id, "description", rss_description.as_deref())?;
                slot_description.write_fact(conn, fact_owner)
            },
        )?;

        let artwork = channel.album_image_href.clone();
        self.element(
            owner,
            RssField::Artwork,
            Stored::rss(text(stored.album_image_href.as_deref())),
            text(artwork.as_deref()),
            |conn| set_feed_column(conn, feed_id, "album_image_href", artwork.as_deref()),
        )?;

        let link = channel.link.clone();
        self.element(
            owner,
            RssField::Link,
            Stored::rss(text(stored.link.as_deref())),
            text(link.as_deref()),
            |conn| set_feed_column(conn, feed_id, "link", link.as_deref()),
        )?;

        let explicit = self.fact_backed(owner, RssField::Explicit, None)?;
        self.element(
            owner,
            RssField::Explicit,
            explicit,
            slot_explicit.rss.clone(),
            |conn| slot_explicit.write_fact(conn, fact_owner),
        )?;

        let language =
            self.fact_backed(owner, RssField::Language, text(stored.language.as_deref()))?;
        let rss_language = channel.language.clone();
        self.element(
            owner,
            RssField::Language,
            language,
            slot_language.rss.clone(),
            |conn| {
                set_feed_column(conn, feed_id, "language", rss_language.as_deref())?;
                slot_language.write_fact(conn, fact_owner)
            },
        )?;

        let album_artist = channel.album_artist.clone();
        self.element(
            owner,
            RssField::AlbumArtist,
            Stored::rss(text(stored.album_artist.as_deref())),
            text(album_artist.as_deref()),
            |conn| set_feed_column(conn, feed_id, "album_artist", album_artist.as_deref()),
        )?;

        let owner_name = self.fact_backed(owner, RssField::Owner, None)?;
        self.element(
            owner,
            RssField::Owner,
            owner_name,
            slot_owner.rss.clone(),
            |conn| slot_owner.write_fact(conn, fact_owner),
        )?;

        self.persons(
            owner,
            LocalEntityOwner::Feed(feed_id),
            &channel.contributors,
            channel.people_json.as_deref(),
        )?;
        self.nostr(owner, LocalIdentityOwner::Feed(feed_id), &channel.nostr_ids)?;

        let routes = channel.podcast_value_json.clone();
        self.element(
            owner,
            RssField::PaymentRoutes,
            Stored::rss(json_value(stored.podcast_value_json.as_deref())),
            json_value(routes.as_deref()),
            |conn| {
                set_feed_column(conn, feed_id, "podcast_value_json", routes.as_deref())?;
                db::payment_routes::refresh_feed_route(conn, feed_id, routes.as_deref())
            },
        )?;

        let stored_publisher =
            db::publisher_relationships::music_to_publisher_remote(self.conn, feed_id)?
                .map(|(guid, url)| json!({"feed_guid": guid, "feed_url": url}));
        let publisher = channel
            .publisher
            .as_ref()
            .and_then(|publisher| Some((publisher.feed_guid.clone()?, publisher.feed_url.clone())));
        let observed_at = self.checked_at_us.div_euclid(1_000_000);
        self.element(
            owner,
            RssField::Publisher,
            Stored::rss(stored_publisher),
            publisher
                .as_ref()
                .map(|(guid, url)| json!({"feed_guid": guid, "feed_url": url})),
            |conn| {
                db::publisher_relationships::set_music_to_publisher_remote(
                    conn,
                    feed_id,
                    publisher
                        .as_ref()
                        .map(|(guid, url)| (guid.as_str(), url.as_deref())),
                    observed_at,
                )
            },
        )?;
        Ok(())
    }

    fn persons(
        &mut self,
        owner: HoldOwner,
        entity: LocalEntityOwner,
        contributors: &[LocalContributorInput],
        people_json: Option<&str>,
    ) -> Result<()> {
        let stored = db::local_contributors(self.conn, entity)?
            .into_iter()
            .filter(|row| row.source == RSS_SOURCE)
            .map(|row| LocalContributorInput {
                position: row.position,
                name: row.name,
                role: row.role,
                group_name: row.group_name,
                href: row.href,
                image_url: row.image_url,
                nostr_npub: row.nostr_npub,
                raw_json: row.raw_json,
                observed_at: row.observed_at,
            })
            .collect::<Vec<_>>();
        let people_json = people_json.map(ToOwned::to_owned);
        self.element(
            owner,
            RssField::Persons,
            Stored::rss(persons_value(&stored)),
            persons_value(contributors),
            |conn| {
                let (table, column, id) = match owner {
                    HoldOwner::Feed(feed_id) => ("feeds", "people_json", feed_id),
                    HoldOwner::Track { track_id, .. } => ("tracks", "people_json", track_id),
                };
                conn.execute(
                    &format!("UPDATE {table} SET {column} = ?1 WHERE id = ?2"),
                    params![people_json, id],
                )
                .context("write people_json")?;
                db::write_local_contributors(conn, entity, RSS_SOURCE, contributors)
            },
        )?;
        Ok(())
    }

    fn nostr(
        &mut self,
        owner: HoldOwner,
        identity: LocalIdentityOwner,
        parsed: &[ParsedNostrId],
    ) -> Result<()> {
        let stored = db::local_identity_ids(self.conn, identity)?
            .into_iter()
            .filter(|row| row.source == RSS_SOURCE)
            .filter_map(|row| {
                let scheme = row.scheme?;
                NOSTR_SCHEMES
                    .contains(&scheme.as_str())
                    .then_some((scheme, row.value?))
            })
            .collect::<Vec<_>>();
        let entity_type = match owner {
            HoldOwner::Feed(_) => "feed",
            HoldOwner::Track { .. } => "track",
        };
        let inputs = parsed
            .iter()
            .map(|id| LocalIdentityIdInput {
                entity_type: Some(entity_type.to_owned()),
                entity_id: None,
                position: Some(id.position),
                scheme: Some(id.scheme.clone()),
                value: Some(id.value.clone()),
                extraction_path: Some("podcast:txt[@purpose='npub']".to_owned()),
                observed_at: None,
                raw_json: None,
            })
            .collect::<Vec<_>>();
        self.element(
            owner,
            RssField::Nostr,
            Stored::rss(nostr_value(&stored)),
            nostr_value(&parsed_nostr(parsed)),
            |conn| {
                db::write_local_identity_ids(
                    conn,
                    identity,
                    RSS_SOURCE,
                    Some(&NOSTR_SCHEMES),
                    &inputs,
                )
            },
        )?;
        Ok(())
    }

    fn items(&mut self, items: &[ParsedItem]) -> Result<()> {
        let stored = stored_tracks(self.conn, self.feed_id)?;
        for item in items {
            match stored
                .iter()
                .find(|track| track.item_guid == item.item_guid)
            {
                Some(track) => {
                    if track.removed_from_feed_at.is_some() {
                        self.conn
                            .execute(
                                "UPDATE tracks SET removed_from_feed_at = NULL,
                                     removed_from_feed_confirmed_at = NULL
                                 WHERE id = ?1",
                                [track.id],
                            )
                            .context("clear removed_from_feed_at")?;
                        self.record(&NewDifference {
                            feed_id: self.feed_id,
                            track_id: Some(track.id),
                            field: RssField::Track,
                            kind: DifferenceKind::TrackReturned,
                            old_value: None,
                            new_value: text(item.track_title.as_deref()),
                        })?;
                    }
                    self.item(track, item)?;
                }
                None => self.add_item(item)?,
            }
        }
        for track in &stored {
            if track.removed_from_feed_at.is_some()
                || items.iter().any(|item| item.item_guid == track.item_guid)
            {
                continue;
            }
            self.conn
                .execute(
                    "UPDATE tracks SET removed_from_feed_at = ?2 WHERE id = ?1",
                    params![track.id, self.checked_at_us],
                )
                .context("set removed_from_feed_at")?;
            self.record(&NewDifference {
                feed_id: self.feed_id,
                track_id: Some(track.id),
                field: RssField::Track,
                kind: DifferenceKind::TrackRemoved,
                old_value: text(track.track_title.as_deref()),
                new_value: None,
            })?;
        }
        Ok(())
    }

    /// ADR 0076 Decision 6: a new item gets a track row with the mapping of
    /// the subscribe persist step. It gets no file, and `is_in_library`
    /// stays 0.
    fn add_item(&mut self, item: &ParsedItem) -> Result<()> {
        upsert_item_columns(self.conn, self.feed_id, item)?;
        let track_id: i64 = self
            .conn
            .query_row(
                "SELECT id FROM tracks WHERE feed_id = ?1 AND item_guid = ?2",
                params![self.feed_id, item.item_guid],
                |row| row.get(0),
            )
            .context("read the added track")?;
        db::write_local_contributors(
            self.conn,
            LocalEntityOwner::Track(track_id),
            RSS_SOURCE,
            &item.contributors,
        )?;
        db::write_local_identity_links(
            self.conn,
            LocalIdentityOwner::Track(track_id),
            RSS_SOURCE,
            &item.links,
        )?;
        self.record(&NewDifference {
            feed_id: self.feed_id,
            track_id: Some(track_id),
            field: RssField::Track,
            kind: DifferenceKind::TrackAdded,
            old_value: None,
            new_value: text(item.track_title.as_deref()),
        })
    }

    #[allow(clippy::too_many_lines)]
    fn item(&mut self, track: &StoredTrack, item: &ParsedItem) -> Result<()> {
        let track_id = track.id;
        let owner = HoldOwner::Track {
            feed_id: self.feed_id,
            track_id,
        };
        let fact_owner = LocalMetadataOwner::Track(track_id);

        let title = item.track_title.clone();
        self.element(
            owner,
            RssField::Title,
            Stored::rss(text(track.track_title.as_deref())),
            text(title.as_deref()),
            |conn| set_track_column(conn, track_id, "track_title", title.as_deref()),
        )?;

        let [slot_description, slot_date, slot_explicit] = item_fact_slots(item);
        let description = self.fact_backed(owner, RssField::Description, None)?;
        self.element(
            owner,
            RssField::Description,
            description,
            slot_description.rss.clone(),
            |conn| slot_description.write_fact(conn, fact_owner),
        )?;

        let artwork = item.track_image_href.clone();
        self.element(
            owner,
            RssField::Artwork,
            Stored::rss(text(track.track_image_href.as_deref())),
            text(artwork.as_deref()),
            |conn| set_track_column(conn, track_id, "track_image_href", artwork.as_deref()),
        )?;

        let link = item.link.clone();
        self.element(
            owner,
            RssField::Link,
            Stored::rss(text(track.link.as_deref())),
            text(link.as_deref()),
            |conn| set_track_column(conn, track_id, "link", link.as_deref()),
        )?;

        let (url, mime) = (item.enclosure_url.clone(), item.enclosure_type.clone());
        self.element(
            owner,
            RssField::Enclosure,
            Stored::rss(enclosure_value(
                track.enclosure_url.as_deref(),
                track.enclosure_type.as_deref(),
            )),
            enclosure_value(url.as_deref(), mime.as_deref()),
            |conn| {
                set_track_column(conn, track_id, "enclosure_url", url.as_deref())?;
                set_track_column(conn, track_id, "enclosure_type", mime.as_deref())
            },
        )?;

        let (seconds, raw) = (item.duration_seconds, item.itunes_duration_raw.clone());
        self.element(
            owner,
            RssField::Duration,
            Stored::rss(duration_value(
                track.duration_seconds,
                track.itunes_duration_raw.as_deref(),
            )),
            duration_value(seconds, raw.as_deref()),
            |conn| {
                conn.execute(
                    "UPDATE tracks SET duration_seconds = ?2, itunes_duration_raw = ?3 WHERE id = ?1",
                    params![track_id, seconds, raw],
                )
                .context("write track duration")?;
                Ok(())
            },
        )?;

        let date =
            self.fact_backed(owner, RssField::Date, date_value(track.pub_date.as_deref()))?;
        let pub_date = item.pub_date.clone();
        self.element(owner, RssField::Date, date, slot_date.rss.clone(), |conn| {
            set_track_column(conn, track_id, "pub_date", pub_date.as_deref())?;
            slot_date.write_fact(conn, fact_owner)
        })?;

        let explicit = self.fact_backed(
            owner,
            RssField::Explicit,
            compare::explicit_flag(track.itunes_explicit.as_deref()).map(Value::Bool),
        )?;
        let rss_explicit_raw = item.itunes_explicit.clone();
        self.element(
            owner,
            RssField::Explicit,
            explicit,
            slot_explicit.rss.clone(),
            |conn| {
                set_track_column(
                    conn,
                    track_id,
                    "itunes_explicit",
                    rss_explicit_raw.as_deref(),
                )?;
                slot_explicit.write_fact(conn, fact_owner)
            },
        )?;

        let artist = item.artist_name.clone();
        self.element(
            owner,
            RssField::Artist,
            Stored::rss(text(track.artist_name.as_deref())),
            text(artist.as_deref()),
            |conn| set_track_column(conn, track_id, "artist_name", artist.as_deref()),
        )?;

        self.persons(
            owner,
            LocalEntityOwner::Track(track_id),
            &item.contributors,
            item.people_json.as_deref(),
        )?;
        self.nostr(owner, LocalIdentityOwner::Track(track_id), &item.nostr_ids)?;

        let routes = item.item_value_json.clone();
        self.element(
            owner,
            RssField::PaymentRoutes,
            Stored::rss(json_value(track.item_value_json.as_deref())),
            json_value(routes.as_deref()),
            |conn| {
                set_track_column(conn, track_id, "item_value_json", routes.as_deref())?;
                db::payment_routes::refresh_track_route(conn, track_id, routes.as_deref())
            },
        )?;
        Ok(())
    }
}

struct FeedColumns {
    title: Option<String>,
    description: Option<String>,
    album_image_href: Option<String>,
    link: Option<String>,
    language: Option<String>,
    album_artist: Option<String>,
    podcast_value_json: Option<String>,
}

struct StoredTrack {
    id: i64,
    item_guid: String,
    track_title: Option<String>,
    track_image_href: Option<String>,
    link: Option<String>,
    enclosure_url: Option<String>,
    enclosure_type: Option<String>,
    duration_seconds: Option<i64>,
    itunes_duration_raw: Option<String>,
    pub_date: Option<String>,
    itunes_explicit: Option<String>,
    artist_name: Option<String>,
    item_value_json: Option<String>,
    removed_from_feed_at: Option<i64>,
}

fn stored_tracks(conn: &Connection, feed_id: i64) -> Result<Vec<StoredTrack>> {
    let mut statement = conn
        .prepare(
            "SELECT id, item_guid, track_title, track_image_href, link, enclosure_url,
                    enclosure_type, duration_seconds, itunes_duration_raw, pub_date,
                    itunes_explicit, artist_name, item_value_json, removed_from_feed_at
             FROM tracks WHERE feed_id = ?1 ORDER BY id",
        )
        .context("prepare stored tracks")?;
    let rows = statement
        .query_map([feed_id], |row| {
            Ok(StoredTrack {
                id: row.get(0)?,
                item_guid: row.get(1)?,
                track_title: row.get(2)?,
                track_image_href: row.get(3)?,
                link: row.get(4)?,
                enclosure_url: row.get(5)?,
                enclosure_type: row.get(6)?,
                duration_seconds: row.get(7)?,
                itunes_duration_raw: row.get(8)?,
                pub_date: row.get(9)?,
                itunes_explicit: row.get(10)?,
                artist_name: row.get(11)?,
                item_value_json: row.get(12)?,
                removed_from_feed_at: row.get(13)?,
            })
        })
        .context("query stored tracks")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("collect stored tracks")?;
    Ok(rows)
}

/// The feed columns that the check writes.
const FEED_COLUMNS: [&str; 7] = [
    "title",
    "description",
    "album_image_href",
    "link",
    "language",
    "album_artist",
    "podcast_value_json",
];

/// The track columns that the check writes.
const TRACK_COLUMNS: [&str; 9] = [
    "track_title",
    "track_image_href",
    "link",
    "enclosure_url",
    "enclosure_type",
    "pub_date",
    "itunes_explicit",
    "artist_name",
    "item_value_json",
];

fn set_feed_column(
    conn: &Connection,
    feed_id: i64,
    column: &str,
    value: Option<&str>,
) -> Result<()> {
    anyhow::ensure!(
        FEED_COLUMNS.contains(&column),
        "unknown feed column {column}"
    );
    conn.execute(
        &format!("UPDATE feeds SET {column} = ?2 WHERE id = ?1"),
        params![feed_id, value],
    )
    .with_context(|| format!("write feeds.{column}"))?;
    Ok(())
}

fn set_track_column(
    conn: &Connection,
    track_id: i64,
    column: &str,
    value: Option<&str>,
) -> Result<()> {
    anyhow::ensure!(
        TRACK_COLUMNS.contains(&column),
        "unknown track column {column}"
    );
    conn.execute(
        &format!("UPDATE tracks SET {column} = ?2 WHERE id = ?1"),
        params![track_id, value],
    )
    .with_context(|| format!("write tracks.{column}"))?;
    Ok(())
}

/// One fact-backed slot of a parsed document: the RSS value in the JSON form
/// of the comparison, and the `rss` fact row of the field. The check and the
/// subscribe command use the same slots (ADR 0076 Decision 5, ADR 0075
/// packet 020).
pub(crate) struct FactSlot {
    pub(crate) field: RssField,
    pub(crate) rss: Option<Value>,
    fact_key: &'static str,
    fact: Option<LocalMetadataValue>,
    extraction_path: &'static str,
}

impl FactSlot {
    fn write_fact(&self, conn: &Connection, owner: LocalMetadataOwner) -> Result<()> {
        write_rss_fact(
            conn,
            owner,
            self.fact_key,
            self.fact.clone(),
            self.extraction_path,
        )
    }
}

fn trimmed_text_fact(value: Option<&str>) -> Option<LocalMetadataValue> {
    compare::trimmed(value).map(|value| LocalMetadataValue::Text(value.to_owned()))
}

/// The fact-backed slots of a channel: description, explicit, language and
/// owner name.
fn channel_fact_slots(channel: &ParsedChannel) -> [FactSlot; 4] {
    let explicit = compare::explicit_flag(channel.explicit.as_deref());
    [
        FactSlot {
            field: RssField::Description,
            rss: text(channel.description.as_deref()),
            fact_key: "description",
            fact: channel
                .description
                .as_deref()
                .map(|value| LocalMetadataValue::Text(value.to_owned())),
            extraction_path: "channel/description",
        },
        FactSlot {
            field: RssField::Explicit,
            rss: explicit.map(Value::Bool),
            fact_key: "explicit",
            fact: explicit.map(LocalMetadataValue::Boolean),
            extraction_path: "channel/itunes:explicit",
        },
        FactSlot {
            field: RssField::Language,
            rss: text(channel.language.as_deref()),
            fact_key: "language",
            fact: trimmed_text_fact(channel.language.as_deref()),
            extraction_path: "channel/language",
        },
        FactSlot {
            field: RssField::Owner,
            rss: text(channel.owner_name.as_deref()),
            fact_key: "publisher_text",
            fact: trimmed_text_fact(channel.owner_name.as_deref()),
            extraction_path: "channel/itunes:owner/itunes:name",
        },
    ]
}

/// The fact-backed slots of an item: description, date and explicit.
fn item_fact_slots(item: &ParsedItem) -> [FactSlot; 3] {
    let explicit = compare::explicit_flag(item.itunes_explicit.as_deref());
    [
        FactSlot {
            field: RssField::Description,
            rss: text(item.description.as_deref()),
            fact_key: "description",
            fact: trimmed_text_fact(item.description.as_deref()),
            extraction_path: "item/description",
        },
        FactSlot {
            field: RssField::Date,
            rss: date_value(item.pub_date.as_deref()),
            fact_key: "pub_date",
            fact: compare::instant(item.pub_date.as_deref()).map(LocalMetadataValue::Integer),
            extraction_path: "item/pubDate",
        },
        FactSlot {
            field: RssField::Explicit,
            rss: explicit.map(Value::Bool),
            fact_key: "explicit",
            fact: explicit.map(LocalMetadataValue::Boolean),
            extraction_path: "item/itunes:explicit",
        },
    ]
}

/// The column-backed slots of a channel that the subscribe persist step
/// writes. `MusicIndex` writes none of them.
const SUBSCRIBED_CHANNEL_COLUMN_FIELDS: [RssField; 5] = [
    RssField::Title,
    RssField::Artwork,
    RssField::Link,
    RssField::AlbumArtist,
    RssField::PaymentRoutes,
];

/// The column-backed slots of an item that the subscribe persist step
/// writes. `MusicIndex` writes none of them.
const SUBSCRIBED_ITEM_COLUMN_FIELDS: [RssField; 7] = [
    RssField::Title,
    RssField::Artwork,
    RssField::Link,
    RssField::Enclosure,
    RssField::Duration,
    RssField::Artist,
    RssField::PaymentRoutes,
];

/// ADR 0076 Decision 5 for the subscribe command. A subscribe reads the RSS
/// document, so its values of the compared slots are RSS values, as the
/// values of a check are. After the persist step, this function writes the
/// `rss` fact row and the hold of each fact-backed slot, with the fresh RSS
/// value. A field that the document does not state gets a cleared hold. The
/// hold has no run, and the function records no difference.
///
/// The persist step writes the column of each column-backed slot. The
/// function deletes an earlier hold of each such slot, so an older held value
/// cannot hide the new column. No `MusicIndex` writer changes these slots,
/// and the projection then shows the column.
///
/// The persons slot also gets the hold of the RSS value. The persist step
/// writes the `rss` credit list, and the gated `MusicIndex` credit writer
/// keeps its own list (ADR 0076 packet 006).
///
/// # Errors
///
/// Returns an error when a database operation fails. The transaction then
/// rolls back.
pub(crate) fn record_subscribed_document(
    conn: &Connection,
    feed_id: i64,
    document: &ParsedFeedDocument,
    subscribed_at_us: i64,
) -> Result<()> {
    let tx = conn
        .unchecked_transaction()
        .context("begin subscribe hold write")?;
    let feed_owner = HoldOwner::Feed(feed_id);
    for slot in channel_fact_slots(&document.channel) {
        slot.write_fact(&tx, LocalMetadataOwner::Feed(feed_id))?;
        holds::write_hold(
            &tx,
            feed_owner,
            slot.field,
            slot.rss.as_ref(),
            None,
            subscribed_at_us,
        )?;
    }
    // ADR 0076 packet 006: the persist step wrote the `rss` credit list. The
    // `MusicIndex` credit list is a second stored list, so the persons slot
    // holds the RSS value like a fact-backed slot.
    holds::write_hold(
        &tx,
        feed_owner,
        RssField::Persons,
        persons_value(&document.channel.contributors).as_ref(),
        None,
        subscribed_at_us,
    )?;
    for field in SUBSCRIBED_CHANNEL_COLUMN_FIELDS {
        holds::delete_hold(&tx, feed_owner, field)?;
    }
    let stored = stored_tracks(&tx, feed_id)?;
    for item in &document.items {
        let Some(track) = stored
            .iter()
            .find(|track| track.item_guid == item.item_guid)
        else {
            continue;
        };
        let owner = HoldOwner::Track {
            feed_id,
            track_id: track.id,
        };
        for slot in item_fact_slots(item) {
            slot.write_fact(&tx, LocalMetadataOwner::Track(track.id))?;
            holds::write_hold(
                &tx,
                owner,
                slot.field,
                slot.rss.as_ref(),
                None,
                subscribed_at_us,
            )?;
        }
        holds::write_hold(
            &tx,
            owner,
            RssField::Persons,
            persons_value(&item.contributors).as_ref(),
            None,
            subscribed_at_us,
        )?;
        for field in SUBSCRIBED_ITEM_COLUMN_FIELDS {
            holds::delete_hold(&tx, owner, field)?;
        }
    }
    tx.commit().context("commit subscribe hold write")?;
    Ok(())
}

/// Write or delete the `rss` fact row of a fact-backed field. The
/// `MusicIndex` fact row stays as evidence.
fn write_rss_fact(
    conn: &Connection,
    owner: LocalMetadataOwner,
    fact_key: &str,
    value: Option<LocalMetadataValue>,
    extraction_path: &str,
) -> Result<()> {
    match value {
        Some(value) => db::replace_local_metadata_fact(
            conn,
            owner,
            RSS_SOURCE,
            &LocalMetadataFactInput {
                fact_key: fact_key.to_owned(),
                value,
                extraction_path: Some(extraction_path.to_owned()),
                observed_at: None,
                raw_json: None,
            },
        ),
        None => db::delete_local_metadata_fact(conn, owner, RSS_SOURCE, fact_key),
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    //! Recorded RSS documents for tests of the check (ADR 0076 packet 002).

    pub(crate) const NPUB: &str = "npub10elfcs4fr0l0r8af98jlmgdh9c8tcxjvz9qkw038js35mp4dma8qzvjptg";
    pub(crate) const OTHER_NPUB: &str =
        "npub180cvv07tjdrrgpa0j7j7tmnyl2yr6yr7l8j4s3evf6u64th6gkwsyjh6w6";

    /// The values of one recorded document. `document` renders it.
    #[derive(Clone)]
    pub(crate) struct Doc {
        pub(crate) title: &'static str,
        pub(crate) link: &'static str,
        pub(crate) description: &'static str,
        pub(crate) language: Option<&'static str>,
        pub(crate) author: &'static str,
        pub(crate) explicit: &'static str,
        pub(crate) owner: &'static str,
        pub(crate) image: &'static str,
        pub(crate) person: &'static str,
        pub(crate) npub: &'static str,
        pub(crate) split: &'static str,
        pub(crate) publisher: (&'static str, &'static str),
        pub(crate) items: Vec<Item>,
    }

    #[derive(Clone)]
    pub(crate) struct Item {
        pub(crate) guid: &'static str,
        pub(crate) title: &'static str,
        pub(crate) link: &'static str,
        pub(crate) description: &'static str,
        pub(crate) date: &'static str,
        pub(crate) url: &'static str,
        pub(crate) duration: &'static str,
        pub(crate) explicit: &'static str,
        pub(crate) author: &'static str,
        pub(crate) image: &'static str,
        pub(crate) person: &'static str,
        pub(crate) npub: &'static str,
        pub(crate) split: &'static str,
    }

    pub(crate) fn item(guid: &'static str, title: &'static str) -> Item {
        Item {
            guid,
            title,
            link: "https://band.test/song",
            description: "&lt;p&gt;Song notes&lt;/p&gt;",
            date: "Tue, 01 Sep 2026 10:00:00 +0000",
            url: "https://cdn.band.test/song.mp3",
            duration: "3:05",
            explicit: "false",
            author: "The Band",
            image: "https://band.test/song.jpg",
            person: "Bob",
            npub: NPUB,
            split: "100",
        }
    }

    pub(crate) fn first() -> Doc {
        Doc {
            title: "Album One",
            link: "https://band.test/album",
            description: "&lt;p&gt;Album &lt;b&gt;notes&lt;/b&gt;&lt;/p&gt;",
            language: Some("en"),
            author: "The Band",
            explicit: "false",
            owner: "Label A",
            image: "https://band.test/cover.jpg",
            person: "Ann",
            npub: NPUB,
            split: "100",
            publisher: ("pub-guid-1", "https://label.test/pub.xml"),
            items: vec![item("item-1", "Song One"), item("item-2", "Song Two")],
        }
    }

    /// Each compared element of `first` changes. `item-2` leaves the
    /// document, and `item-3` joins it.
    pub(crate) fn second() -> Doc {
        Doc {
            title: "Album Two",
            link: "https://band.test/album-two",
            description: "&lt;p&gt;New album notes&lt;/p&gt;",
            language: Some("fr"),
            author: "The New Band",
            explicit: "true",
            owner: "Label B",
            image: "https://band.test/cover-two.jpg",
            person: "Cat",
            npub: OTHER_NPUB,
            split: "90",
            publisher: ("pub-guid-2", "https://label.test/pub-two.xml"),
            items: vec![
                Item {
                    guid: "item-1",
                    title: "Song One (Remaster)",
                    link: "https://band.test/song-remaster",
                    description: "&lt;p&gt;New song notes&lt;/p&gt;",
                    date: "Wed, 02 Sep 2026 10:00:00 +0000",
                    url: "https://cdn.band.test/song-remaster.mp3",
                    duration: "4:10",
                    explicit: "true",
                    author: "The Band feat. Dee",
                    image: "https://band.test/song-remaster.jpg",
                    person: "Dee",
                    npub: OTHER_NPUB,
                    split: "80",
                },
                item("item-3", "Song Three"),
            ],
        }
    }

    pub(crate) fn document(doc: &Doc) -> Vec<u8> {
        let items = doc
            .items
            .iter()
            .map(|item| {
                format!(
                    r#"<item><title>{title}</title><guid isPermaLink="false">{guid}</guid><link>{link}</link><description>{description}</description><pubDate>{date}</pubDate><enclosure url="{url}" type="audio/mpeg" length="100"/><itunes:duration>{duration}</itunes:duration><itunes:explicit>{explicit}</itunes:explicit><itunes:author>{author}</itunes:author><itunes:image href="{image}"/><podcast:person role="guitar">{person}</podcast:person><podcast:txt purpose="npub">{npub}</podcast:txt><podcast:value type="lightning" method="keysend"><podcast:valueRecipient name="Band" type="node" address="abc" split="{split}"/></podcast:value></item>"#,
                    title = item.title,
                    guid = item.guid,
                    link = item.link,
                    description = item.description,
                    date = item.date,
                    url = item.url,
                    duration = item.duration,
                    explicit = item.explicit,
                    author = item.author,
                    image = item.image,
                    person = item.person,
                    npub = item.npub,
                    split = item.split,
                )
            })
            .collect::<String>();
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd" xmlns:podcast="https://podcastindex.org/namespace/1.0"><channel><title>{title}</title><link>{link}</link><description>{description}</description>{language}<itunes:author>{author}</itunes:author><itunes:explicit>{explicit}</itunes:explicit><itunes:owner><itunes:name>{owner}</itunes:name><itunes:email>owner@label.test</itunes:email></itunes:owner><itunes:image href="{image}"/><podcast:guid>feed-guid-1</podcast:guid><podcast:medium>music</podcast:medium><podcast:person role="vocals" href="https://band.test/person">{person}</podcast:person><podcast:txt purpose="npub">{npub}</podcast:txt><podcast:value type="lightning" method="keysend"><podcast:valueRecipient name="Band" type="node" address="abc" split="{split}"/></podcast:value><podcast:publisher><podcast:remoteItem medium="publisher" feedGuid="{publisher_guid}" feedUrl="{publisher_url}"/></podcast:publisher>{items}</channel></rss>"#,
            title = doc.title,
            link = doc.link,
            description = doc.description,
            language = doc
                .language
                .map(|language| format!("<language>{language}</language>"))
                .unwrap_or_default(),
            author = doc.author,
            explicit = doc.explicit,
            owner = doc.owner,
            image = doc.image,
            person = doc.person,
            npub = doc.npub,
            split = doc.split,
            publisher_guid = doc.publisher.0,
            publisher_url = doc.publisher.1,
        )
        .into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{document, first, item, second};
    use super::*;

    const MUSICINDEX_SOURCE: &str = "musicindex";
    use crate::db::rss_check_runs::{self as runs, RssCheckTrigger};
    use crate::rss::subscribe::{persist_channel, persist_items};
    use rusqlite::types::Value as SqlValue;

    const FEED_URL: &str = "https://band.test/feed.xml";
    const T1: i64 = 1_790_000_000_000_000;
    const T2: i64 = 1_790_000_100_000_000;

    fn database() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        db::init_schema(&conn).unwrap();
        db::migrate_schema(&conn).unwrap();
        conn.execute("INSERT INTO playlists(id, name) VALUES (1, 'Show')", [])
            .unwrap();
        conn
    }

    fn run(conn: &Connection, at: i64) -> i64 {
        runs::insert_run(conn, 1, RssCheckTrigger::Button, at).unwrap()
    }

    /// The subscribe persist step of one recorded document.
    fn subscribed(body: &[u8]) -> (Connection, i64) {
        let mut conn = database();
        let parsed = parse_feed_document(body).unwrap();
        let feed_id = persist_channel(&mut conn, FEED_URL, &parsed.channel).unwrap();
        persist_items(&mut conn, FEED_URL, feed_id, &parsed.items).unwrap();
        (conn, feed_id)
    }

    fn track_id(conn: &Connection, guid: &str) -> i64 {
        conn.query_row(
            "SELECT id FROM tracks WHERE item_guid = ?1",
            [guid],
            |row| row.get(0),
        )
        .unwrap()
    }

    fn rows(conn: &Connection, sql: &str) -> Vec<Vec<SqlValue>> {
        let mut statement = conn.prepare(sql).unwrap();
        let count = statement.column_count();
        statement
            .query_map([], |row| (0..count).map(|index| row.get(index)).collect())
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    fn count(conn: &Connection, sql: &str) -> i64 {
        conn.query_row(sql, [], |row| row.get(0)).unwrap()
    }

    fn fields(differences: &[holds::StoredDifference], track: Option<i64>) -> Vec<RssField> {
        let mut fields = differences
            .iter()
            .filter(|difference| difference.track_id == track)
            .map(|difference| difference.field)
            .collect::<Vec<_>>();
        fields.sort();
        fields
    }

    /// The subscribe persist step of one recorded document at one time.
    fn resubscribe(conn: &mut Connection, doc: &test_support::Doc, at: i64) -> i64 {
        let parsed = parse_feed_document(&document(doc)).unwrap();
        crate::rss::subscribe::persist_subscribed_document(conn, FEED_URL, &parsed, at)
            .unwrap()
            .0
    }

    fn projected(conn: &Connection, feed_id: i64) -> stored_values::FeedStoredValues {
        stored_values::feed_values(conn, feed_id).unwrap()
    }

    fn musicindex_feed(language: &str, updated_at_us: i64) -> crate::api::Feed {
        crate::api::Feed {
            language: Some(language.to_owned()),
            updated_at: Some(updated_at_us.div_euclid(1_000_000)),
            ..Default::default()
        }
    }

    /// R20-09: after a check, a new subscribe with a changed description
    /// gives the new RSS value. It hides the older `musicindex` fact and the
    /// hold of the check.
    #[test]
    fn adr_0075_projection_resubscribe_replaces_check_hold() {
        let mut conn = database();
        let feed_id = resubscribe(&mut conn, &first(), T1);
        let run_id = run(&conn, T1);
        apply_checked_document(&conn, run_id, feed_id, &document(&second()), T1).unwrap();
        db::replace_local_metadata_fact(
            &conn,
            LocalMetadataOwner::Feed(feed_id),
            MUSICINDEX_SOURCE,
            &LocalMetadataFactInput {
                fact_key: "description".to_owned(),
                value: LocalMetadataValue::Text("Old index notes".to_owned()),
                extraction_path: Some("$.description".to_owned()),
                observed_at: Some(1),
                raw_json: None,
            },
        )
        .unwrap();
        assert_eq!(
            projected(&conn, feed_id).description.value.as_deref(),
            Some("<p>New album notes</p>")
        );

        let recorded = count(&conn, "SELECT count(*) FROM rss_check_differences");
        let mut third = first();
        third.description = "&lt;p&gt;Third notes&lt;/p&gt;";
        resubscribe(&mut conn, &third, T2);

        assert_eq!(
            projected(&conn, feed_id).description.value.as_deref(),
            Some("<p>Third notes</p>")
        );
        let hold = holds::hold(&conn, HoldOwner::Feed(feed_id), RssField::Description)
            .unwrap()
            .unwrap();
        assert_eq!(hold.run_id, None);
        assert_eq!(hold.checked_at_us, T2);
        assert_eq!(
            count(&conn, "SELECT count(*) FROM rss_check_differences"),
            recorded,
            "a subscribe records no difference"
        );
    }

    /// R20-10: after a check, a new subscribe of a document without the
    /// field gives no value, although a `musicindex` fact exists.
    #[test]
    fn adr_0075_projection_resubscribe_without_field_gives_no_value() {
        let mut conn = database();
        let feed_id = resubscribe(&mut conn, &first(), T1);
        let run_id = run(&conn, T1);
        apply_checked_document(&conn, run_id, feed_id, &document(&second()), T1).unwrap();
        identity_ingest_language(&mut conn, feed_id, "de", T1 - 10_000_000);
        assert_eq!(
            projected(&conn, feed_id).language.value.as_deref(),
            Some("fr")
        );

        let mut without_language = first();
        without_language.language = None;
        resubscribe(&mut conn, &without_language, T2);

        assert_eq!(projected(&conn, feed_id).language.value, None);
        let hold = holds::hold(&conn, HoldOwner::Feed(feed_id), RssField::Language)
            .unwrap()
            .unwrap();
        assert_eq!(hold.value, None);
    }

    fn identity_ingest_language(conn: &mut Connection, feed_id: i64, language: &str, at_us: i64) {
        crate::identity_ingest::persist_musicindex_feed(
            conn,
            feed_id,
            &musicindex_feed(language, at_us),
        )
        .unwrap();
    }

    /// R20-11: after a subscribe, a `MusicIndex` write with an older
    /// `updated_at` and a different value leaves the projected value. A
    /// newer `updated_at` changes it.
    #[test]
    fn adr_0075_projection_subscribe_hold_gates_musicindex() {
        let mut conn = database();
        let feed_id = resubscribe(&mut conn, &first(), T1);
        assert_eq!(
            projected(&conn, feed_id).language.value.as_deref(),
            Some("en")
        );

        identity_ingest_language(&mut conn, feed_id, "de", T1 - 10_000_000);
        assert_eq!(
            projected(&conn, feed_id).language.value.as_deref(),
            Some("en")
        );

        identity_ingest_language(&mut conn, feed_id, "de", T1 + 10_000_000);
        assert_eq!(
            projected(&conn, feed_id).language.value.as_deref(),
            Some("de")
        );
        assert!(
            holds::hold(&conn, HoldOwner::Feed(feed_id), RssField::Language)
                .unwrap()
                .is_none()
        );
    }

    /// ADR 0076 packet 006: a subscribe holds the RSS credit list of the
    /// channel and of each item. An older `MusicIndex` credit list is stored
    /// and does not replace the projected list.
    #[test]
    fn adr_0076_credit_list_subscribe_holds_the_rss_list() {
        let mut conn = database();
        let feed_id = resubscribe(&mut conn, &first(), T1);
        let track_id = track_id(&conn, "item-1");
        for owner in [
            HoldOwner::Feed(feed_id),
            HoldOwner::Track { feed_id, track_id },
        ] {
            assert!(holds::hold(&conn, owner, RssField::Persons)
                .unwrap()
                .is_some());
        }

        let older = crate::api::Feed {
            source_contributors: Some(vec![crate::api::Contributor {
                name: Some("Old Name".into()),
                ..crate::api::Contributor::default()
            }]),
            updated_at: Some((T1 - 10_000_000).div_euclid(1_000_000)),
            ..Default::default()
        };
        crate::identity_ingest::persist_musicindex_feed(&mut conn, feed_id, &older).unwrap();

        let names = projected(&conn, feed_id)
            .credits
            .into_iter()
            .filter_map(|credit| credit.name)
            .collect::<Vec<_>>();
        assert_eq!(names, ["Ann"]);
        assert_eq!(
            db::local_contributors(&conn, LocalEntityOwner::Feed(feed_id))
                .unwrap()
                .len(),
            2
        );
    }

    const FEED_COLUMNS_SQL: &str = "SELECT title, link, language, description, album_image_href, people_json, podcast_value_json, album_artist FROM feeds ORDER BY id";
    const TRACK_COLUMNS_SQL: &str = "SELECT item_guid, enclosure_url, enclosure_type, link, pub_date, track_title, artist_name, album_title, album_artist_name, disc_number, track_number, duration_seconds, itunes_duration_raw, itunes_explicit, track_image_href, track_image_mime, people_json, item_value_json, is_in_library, extra_json FROM tracks ORDER BY item_guid";

    /// R2-01: `subscribe_feed` and the check produce the same column values
    /// from one recorded document.
    #[test]
    fn adr_0076_rss_comparison_subscribe_and_check_write_equal_columns() {
        let body = document(&first());
        let (subscribed, _) = subscribed(&body);

        let checked = database();
        checked
            .execute(
                "INSERT INTO feeds(id, feed_url, feed_guid) VALUES (1, ?1, 'feed-guid-1')",
                [FEED_URL],
            )
            .unwrap();
        let run_id = run(&checked, T1);
        apply_checked_document(&checked, run_id, 1, &body, T1).unwrap();

        assert_eq!(
            rows(&checked, FEED_COLUMNS_SQL),
            rows(&subscribed, FEED_COLUMNS_SQL)
        );
        assert_eq!(
            rows(&checked, TRACK_COLUMNS_SQL),
            rows(&subscribed, TRACK_COLUMNS_SQL)
        );
        assert_eq!(rows(&checked, TRACK_COLUMNS_SQL).len(), 2);
    }

    /// R2-04: each compared element gives a difference when its value
    /// changes, on a recorded document pair. A repeated document gives none.
    #[test]
    fn adr_0076_rss_comparison_each_element_change_is_a_difference() {
        let (conn, feed_id) = subscribed(&document(&first()));
        let first_run = run(&conn, T1);
        apply_checked_document(&conn, first_run, feed_id, &document(&first()), T1).unwrap();
        let repeat = run(&conn, T1 + 1);
        assert_eq!(
            apply_checked_document(&conn, repeat, feed_id, &document(&first()), T1 + 1)
                .unwrap()
                .differences,
            0,
            "{:?}",
            holds::run_differences(&conn, repeat, None).unwrap()
        );

        let second_run = run(&conn, T2);
        apply_checked_document(&conn, second_run, feed_id, &document(&second()), T2).unwrap();
        let differences = holds::run_differences(&conn, second_run, None).unwrap();
        assert_eq!(
            fields(&differences, None),
            vec![
                RssField::Title,
                RssField::Description,
                RssField::Artwork,
                RssField::Link,
                RssField::Explicit,
                RssField::Language,
                RssField::AlbumArtist,
                RssField::Owner,
                RssField::Persons,
                RssField::Nostr,
                RssField::PaymentRoutes,
                RssField::Publisher,
            ]
        );
        assert_eq!(
            fields(&differences, Some(track_id(&conn, "item-1"))),
            vec![
                RssField::Title,
                RssField::Description,
                RssField::Artwork,
                RssField::Link,
                RssField::Enclosure,
                RssField::Duration,
                RssField::Date,
                RssField::Explicit,
                RssField::Artist,
                RssField::Persons,
                RssField::Nostr,
                RssField::PaymentRoutes,
            ]
        );
        assert!(differences
            .iter()
            .any(|difference| difference.kind == DifferenceKind::TrackRemoved
                && difference.track_id == Some(track_id(&conn, "item-2"))));
        assert!(differences
            .iter()
            .any(|difference| difference.kind == DifferenceKind::TrackAdded
                && difference.track_id == Some(track_id(&conn, "item-3"))));
    }

    /// R2-02 at the check: a description with the same readable text and
    /// different HTML gives no difference.
    #[test]
    fn adr_0076_rss_comparison_description_markup_change_is_equal() {
        let (conn, feed_id) = subscribed(&document(&first()));
        apply_checked_document(&conn, run(&conn, T1), feed_id, &document(&first()), T1).unwrap();
        let mut markup = first();
        markup.description = "&lt;div&gt;Album  &lt;i&gt;notes&lt;/i&gt;&lt;/div&gt;";
        markup.image = "https://band.test:443/cover.jpg";
        let run_id = run(&conn, T2);
        let applied =
            apply_checked_document(&conn, run_id, feed_id, &document(&markup), T2).unwrap();
        assert_eq!(applied.differences, 0);
    }

    /// R2-05: a changed derived value gives no difference.
    #[test]
    fn adr_0076_rss_comparison_derived_values_are_not_compared() {
        let (conn, feed_id) = subscribed(&document(&first()));
        apply_checked_document(&conn, run(&conn, T1), feed_id, &document(&first()), T1).unwrap();
        conn.execute(
            "UPDATE feed_publisher_relationships SET role = 'label', publisher_link_resolution = 'resolved', two_way_validated = 1, music_names_publisher = 1",
            [],
        )
        .unwrap();
        db::replace_local_metadata_fact(
            &conn,
            LocalMetadataOwner::Feed(feed_id),
            MUSICINDEX_SOURCE,
            &LocalMetadataFactInput {
                fact_key: "release_date".to_owned(),
                value: LocalMetadataValue::Integer(1),
                extraction_path: None,
                observed_at: None,
                raw_json: None,
            },
        )
        .unwrap();
        let run_id = run(&conn, T2);
        assert_eq!(
            apply_checked_document(&conn, run_id, feed_id, &document(&first()), T2)
                .unwrap()
                .differences,
            0
        );
        assert_eq!(
            count(
                &conn,
                "SELECT count(*) FROM feed_publisher_relationships WHERE role = 'label' AND two_way_validated = 1"
            ),
            1
        );
    }

    /// R2-06: a difference writes the slot, the `rss` fact and one hold.
    /// The old value is in the difference row.
    #[test]
    fn adr_0076_rss_comparison_difference_writes_slot_fact_and_hold() {
        let (conn, feed_id) = subscribed(&document(&first()));
        let owner = LocalMetadataOwner::Feed(feed_id);
        db::replace_local_metadata_fact(
            &conn,
            owner,
            MUSICINDEX_SOURCE,
            &LocalMetadataFactInput {
                fact_key: "description".to_owned(),
                value: LocalMetadataValue::Text("Old notes".to_owned()),
                extraction_path: Some("$.description".to_owned()),
                observed_at: Some(1),
                raw_json: None,
            },
        )
        .unwrap();
        let run_id = run(&conn, T1);
        apply_checked_document(&conn, run_id, feed_id, &document(&first()), T1).unwrap();

        let column: String = conn
            .query_row(
                "SELECT description FROM feeds WHERE id = ?1",
                [feed_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(column, "<p>Album <b>notes</b></p>");
        let rss = db::local_metadata_fact(&conn, owner, RSS_SOURCE, "description")
            .unwrap()
            .unwrap();
        assert_eq!(rss.value, LocalMetadataValue::Text(column.clone()));
        let musicindex = db::local_metadata_fact(&conn, owner, MUSICINDEX_SOURCE, "description")
            .unwrap()
            .unwrap();
        assert_eq!(
            musicindex.value,
            LocalMetadataValue::Text("Old notes".to_owned())
        );
        let hold = holds::hold(&conn, HoldOwner::Feed(feed_id), RssField::Description)
            .unwrap()
            .unwrap();
        assert_eq!(hold.value, Some(Value::String(column)));
        assert_eq!((hold.run_id, hold.checked_at_us), (Some(run_id), T1));
        assert_eq!(
            count(
                &conn,
                "SELECT count(*) FROM rss_field_holds WHERE field = 'description' AND owner_kind = 'feed'"
            ),
            1
        );
        let difference = holds::run_differences(&conn, run_id, None)
            .unwrap()
            .into_iter()
            .find(|difference| {
                difference.field == RssField::Description && difference.track_id.is_none()
            })
            .unwrap();
        assert_eq!(difference.kind, DifferenceKind::Changed);
        assert_eq!(difference.old_value, Some(Value::from("Old notes")));
        assert_eq!(
            difference.new_value,
            Some(Value::from("<p>Album <b>notes</b></p>"))
        );
    }

    /// R2-07: a field absent from the document is cleared, with a hold
    /// whose value is null.
    #[test]
    fn adr_0076_rss_comparison_absent_field_is_cleared_with_null_hold() {
        let (conn, feed_id) = subscribed(&document(&first()));
        apply_checked_document(&conn, run(&conn, T1), feed_id, &document(&first()), T1).unwrap();
        let mut without_language = first();
        without_language.language = None;
        let run_id = run(&conn, T2);
        apply_checked_document(&conn, run_id, feed_id, &document(&without_language), T2).unwrap();
        let language: Option<String> = conn
            .query_row(
                "SELECT language FROM feeds WHERE id = ?1",
                [feed_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(language, None);
        assert_eq!(
            db::local_metadata_fact(
                &conn,
                LocalMetadataOwner::Feed(feed_id),
                RSS_SOURCE,
                "language"
            )
            .unwrap(),
            None
        );
        let hold = holds::hold(&conn, HoldOwner::Feed(feed_id), RssField::Language)
            .unwrap()
            .unwrap();
        assert_eq!(hold.value, None);
        let differences = holds::run_differences(&conn, run_id, None).unwrap();
        assert_eq!(differences.len(), 1);
        assert_eq!(differences[0].kind, DifferenceKind::Cleared);
        assert_eq!(differences[0].old_value, Some(Value::from("en")));
        assert_eq!(differences[0].new_value, None);
    }

    /// R2-08 at the parse: a parse failure writes no slot, no hold and no
    /// difference.
    #[test]
    fn adr_0076_rss_comparison_parse_failure_writes_nothing() {
        let (conn, feed_id) = subscribed(&document(&first()));
        let before = (
            rows(&conn, FEED_COLUMNS_SQL),
            rows(&conn, TRACK_COLUMNS_SQL),
        );
        let run_id = run(&conn, T2);
        assert!(apply_checked_document(&conn, run_id, feed_id, b"<rss><channel>", T2).is_err());
        assert_eq!(
            (
                rows(&conn, FEED_COLUMNS_SQL),
                rows(&conn, TRACK_COLUMNS_SQL)
            ),
            before
        );
        assert_eq!(count(&conn, "SELECT count(*) FROM rss_field_holds"), 0);
        assert_eq!(
            count(&conn, "SELECT count(*) FROM rss_check_differences"),
            0
        );
    }

    /// R2-12: a new item inserts a track with no file and `is_in_library`
    /// 0, and a `track_added` difference.
    #[test]
    fn adr_0076_rss_comparison_new_item_is_stored_without_file() {
        let (conn, feed_id) = subscribed(&document(&first()));
        apply_checked_document(&conn, run(&conn, T1), feed_id, &document(&first()), T1).unwrap();
        let mut added = first();
        added.items.push(item("item-3", "Song Three"));
        let run_id = run(&conn, T2);
        apply_checked_document(&conn, run_id, feed_id, &document(&added), T2).unwrap();
        let track = track_id(&conn, "item-3");
        assert_eq!(
            count(
                &conn,
                &format!("SELECT is_in_library FROM tracks WHERE id = {track}")
            ),
            0
        );
        assert_eq!(
            count(
                &conn,
                &format!("SELECT count(*) FROM local_files WHERE track_id = {track}")
            ),
            0
        );
        let differences = holds::run_differences(&conn, run_id, None).unwrap();
        assert_eq!(differences.len(), 1);
        assert_eq!(differences[0].kind, DifferenceKind::TrackAdded);
        assert_eq!(differences[0].track_id, Some(track));
        assert_eq!(differences[0].track_title.as_deref(), Some("Song Three"));
    }

    /// R2-13 and R2-14: a missing item gets the mark and keeps its rows. A
    /// returned item clears both columns.
    #[test]
    fn adr_0076_rss_comparison_removed_item_is_marked_and_returns() {
        let (conn, feed_id) = subscribed(&document(&first()));
        apply_checked_document(&conn, run(&conn, T1), feed_id, &document(&first()), T1).unwrap();
        let removed = track_id(&conn, "item-2");
        conn.execute(
            "UPDATE tracks SET is_in_library = 1 WHERE id = ?1",
            [removed],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO local_files(path, track_id) VALUES ('Band/Album/Two.mp3', ?1)",
            [removed],
        )
        .unwrap();
        db::playlist_append(&conn, 1, removed).unwrap();
        let mut without = first();
        without.items.truncate(1);
        let run_id = run(&conn, T2);
        apply_checked_document(&conn, run_id, feed_id, &document(&without), T2).unwrap();
        assert_eq!(
            count(
                &conn,
                &format!("SELECT removed_from_feed_at FROM tracks WHERE id = {removed}")
            ),
            T2
        );
        assert_eq!(count(&conn, "SELECT count(*) FROM tracks"), 2);
        assert_eq!(
            count(
                &conn,
                &format!("SELECT count(*) FROM local_files WHERE track_id = {removed}")
            ),
            1
        );
        assert_eq!(
            count(
                &conn,
                &format!("SELECT count(*) FROM playlist_tracks WHERE track_id = {removed}")
            ),
            1
        );
        let differences = holds::run_differences(&conn, run_id, None).unwrap();
        assert_eq!(differences.len(), 1);
        assert_eq!(differences[0].kind, DifferenceKind::TrackRemoved);
        assert_eq!(
            holds::playlist_removed_marks(&conn, 1).unwrap(),
            vec![(removed, T2)]
        );
        // A second check of the same document records nothing new.
        let again = run(&conn, T2 + 1);
        assert_eq!(
            apply_checked_document(&conn, again, feed_id, &document(&without), T2 + 1)
                .unwrap()
                .differences,
            0
        );

        conn.execute(
            "UPDATE tracks SET removed_from_feed_confirmed_at = ?2 WHERE id = ?1",
            params![removed, T2 + 2],
        )
        .unwrap();
        let returned = run(&conn, T2 + 3);
        apply_checked_document(&conn, returned, feed_id, &document(&first()), T2 + 3).unwrap();
        assert_eq!(
            count(
                &conn,
                &format!("SELECT count(*) FROM tracks WHERE id = {removed} AND removed_from_feed_at IS NULL AND removed_from_feed_confirmed_at IS NULL")
            ),
            1
        );
        let differences = holds::run_differences(&conn, returned, None).unwrap();
        assert_eq!(differences.len(), 1);
        assert_eq!(differences[0].kind, DifferenceKind::TrackReturned);
        assert!(holds::playlist_removed_marks(&conn, 1).unwrap().is_empty());
    }

    /// R2-15: a changed `podcast:publisher` remote item updates the two
    /// remote columns of the relationship row and leaves the derived columns.
    #[test]
    fn adr_0076_rss_comparison_publisher_remote_updates_relationship() {
        let (conn, feed_id) = subscribed(&document(&first()));
        conn.execute(
            "INSERT INTO feed_publisher_relationships(feed_id, direction, publisher_feed_guid, remote_feed_guid, remote_feed_url, role, publisher_link_resolution, two_way_validated, observed_at)
             VALUES (?1, 'music_to_publisher', 'pub-guid-1', 'pub-guid-1', 'https://label.test/pub.xml', 'label', 'resolved', 1, 5)",
            [feed_id],
        )
        .unwrap();
        let mut moved = first();
        moved.publisher = ("pub-guid-2", "https://label.test/pub-two.xml");
        let run_id = run(&conn, T1);
        apply_checked_document(&conn, run_id, feed_id, &document(&moved), T1).unwrap();
        assert_eq!(
            rows(
                &conn,
                "SELECT publisher_feed_guid, remote_feed_guid, remote_feed_url, role, publisher_link_resolution, two_way_validated, observed_at FROM feed_publisher_relationships"
            ),
            vec![vec![
                SqlValue::Text("pub-guid-2".into()),
                SqlValue::Text("pub-guid-2".into()),
                SqlValue::Text("https://label.test/pub-two.xml".into()),
                SqlValue::Text("label".into()),
                SqlValue::Text("resolved".into()),
                SqlValue::Integer(1),
                SqlValue::Integer(5),
            ]]
        );
        let publisher = holds::run_differences(&conn, run_id, None)
            .unwrap()
            .into_iter()
            .filter(|difference| difference.field == RssField::Publisher)
            .collect::<Vec<_>>();
        assert_eq!(publisher.len(), 1);
        assert_eq!(
            publisher[0].old_value,
            Some(
                serde_json::json!({"feed_guid": "pub-guid-1", "feed_url": "https://label.test/pub.xml"})
            )
        );
    }

    /// R2-20: a changed channel artist text writes `feeds.album_artist` and
    /// one difference for the feed. No `tracks` column changes.
    #[test]
    fn adr_0076_rss_comparison_channel_artist_changes_only_the_feed() {
        let (conn, feed_id) = subscribed(&document(&first()));
        apply_checked_document(&conn, run(&conn, T1), feed_id, &document(&first()), T1).unwrap();
        let tracks = rows(&conn, "SELECT * FROM tracks ORDER BY id");
        let mut renamed = first();
        renamed.author = "The Renamed Band";
        let run_id = run(&conn, T2);
        apply_checked_document(&conn, run_id, feed_id, &document(&renamed), T2).unwrap();
        let differences = holds::run_differences(&conn, run_id, None).unwrap();
        assert_eq!(differences.len(), 1);
        assert_eq!(differences[0].field, RssField::AlbumArtist);
        assert_eq!(differences[0].track_id, None);
        let album_artist: String = conn
            .query_row(
                "SELECT album_artist FROM feeds WHERE id = ?1",
                [feed_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(album_artist, "The Renamed Band");
        assert_eq!(rows(&conn, "SELECT * FROM tracks ORDER BY id"), tracks);
    }

    /// ADR 0076 packet 003: the subscribe persist step and the check write
    /// the canonical route of each raw block that they write.
    #[test]
    fn adr_0076_route_readiness_subscribe_and_check_refresh_stored_route() {
        let splits = |conn: &Connection, sql: &str| -> Vec<Option<f64>> {
            let json: String = conn.query_row(sql, [], |row| row.get(0)).unwrap();
            serde_json::from_str::<Vec<crate::api::PaymentRoute>>(&json)
                .unwrap()
                .iter()
                .map(|route| route.split)
                .collect()
        };
        const FEED: &str = "SELECT payment_routes_json FROM feeds";
        const ITEM_ONE: &str = "SELECT payment_routes_json FROM tracks WHERE item_guid = 'item-1'";
        const ITEM_THREE: &str =
            "SELECT payment_routes_json FROM tracks WHERE item_guid = 'item-3'";

        let (mut conn, feed_id) = subscribed(&document(&first()));
        assert_eq!(splits(&conn, FEED), vec![Some(100.0)]);
        assert_eq!(splits(&conn, ITEM_ONE), vec![Some(100.0)]);

        let second_run = run(&conn, T2);
        apply_checked_document(&conn, second_run, feed_id, &document(&second()), T2).unwrap();
        assert_eq!(splits(&conn, FEED), vec![Some(90.0)]);
        assert_eq!(splits(&conn, ITEM_ONE), vec![Some(80.0)]);
        assert_eq!(
            splits(&conn, ITEM_THREE),
            vec![Some(100.0)],
            "an added item"
        );

        resubscribe(&mut conn, &first(), T2 + 1);
        assert_eq!(splits(&conn, FEED), vec![Some(100.0)]);
        assert_eq!(splits(&conn, ITEM_ONE), vec![Some(100.0)]);
    }
}
