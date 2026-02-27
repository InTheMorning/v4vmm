use anyhow::{Context, Result};
use reqwest;
use rss::Channel;
use rusqlite::Connection;
use std::io::Cursor;

use super::helpers::*;
use crate::config::Config;

pub fn cmd_subscribe(_cfg: &Config, conn: &mut Connection, feed_url: &str) -> Result<()> {
    println!("Fetching: {feed_url}");

    // --- fetch ---
    let body = reqwest::blocking::Client::new()
        .get(feed_url)
        .send()
        .with_context(|| format!("GET {feed_url}"))?
        .error_for_status()
        .with_context(|| format!("HTTP error for {feed_url}"))?
        .bytes()
        .with_context(|| format!("read body {feed_url}"))?;

    // --- parse ---
    let feed = Channel::read_from(Cursor::new(body)).context("parse RSS")?;

    // --- feed-level fields (RSS channel + podcast extensions) ---
    let feed_title = feed.title().to_string();

    let feed_link = {
        let l = feed.link().trim();
        if l.is_empty() {
            None
        } else {
            Some(l.to_string())
        }
    };

    let language = feed.language().map(|s| s.to_string());

    let desc = feed.description().trim();
    let description = if desc.is_empty() {
        None
    } else {
        Some(desc.to_string())
    };

    // Podcasting 2.0 extensions: rss crate stores keys without prefix (guid, medium, value, ...)
    let feed_guid = find_ext_text(feed.extensions(), "podcast", "guid");
    let podcast_medium = find_ext_text(feed.extensions(), "podcast", "medium");

    // Album image (prefer podcast:image/@href; fall back to itunes channel image; then <image><url>)
    let mut album_image_href = find_ext_attr(feed.extensions(), "podcast", "image", "href")
        .or_else(|| {
            feed.itunes_ext()
                .and_then(|it| it.image())
                .map(|s| s.to_string())
        });
    if album_image_href.is_none() {
        if let Some(img) = feed.image() {
            album_image_href = Some(img.url().to_string());
        }
    }
    let album_image_mime: Option<String> = None;

    // People at feed level (podcast:person); ok if None
    let feed_people_json = collect_people_json(feed.extensions());

    // Full value block (including recipients) as JSON
    let podcast_value_json = value_block_json(feed.extensions(), "podcast", "value");

    // --- upsert feed row (always mark subscribed) ---
    conn.execute(
        r#"
        INSERT INTO feeds (
            feed_url,
            feed_guid,
            title,
            link,
            language,
            description,
            podcast_medium,
            album_image_href,
            album_image_mime,
            people_json,
            podcast_value_json,
            is_subscribed,
            last_fetched_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 1, datetime('now'))
        ON CONFLICT(feed_url) DO UPDATE SET
            feed_guid          = excluded.feed_guid,
            title              = excluded.title,
            link               = excluded.link,
            language           = excluded.language,
            description        = excluded.description,
            podcast_medium     = excluded.podcast_medium,
            album_image_href   = excluded.album_image_href,
            album_image_mime   = excluded.album_image_mime,
            people_json        = excluded.people_json,
            podcast_value_json = excluded.podcast_value_json,
            is_subscribed      = 1,
            last_fetched_at    = datetime('now')
        "#,
        rusqlite::params![
            feed_url,
            feed_guid,
            feed_title,
            feed_link,
            language,
            description,
            podcast_medium,
            album_image_href,
            album_image_mime,
            feed_people_json,
            podcast_value_json,
        ],
    )
    .context("upsert feed")?;

    let feed_id: i64 = conn
        .query_row(
            "SELECT id FROM feeds WHERE feed_url = ?1",
            rusqlite::params![feed_url],
            |row| row.get(0),
        )
        .context("lookup feed_id")?;

    // --- tracks: upsert all items in one transaction ---
    let tx = conn.transaction().context("begin transaction")?;
    let mut upserted = 0usize;

    for item in feed.items() {
        // Stable identity: item <guid>. If missing, skip (we need stable IDs).
        let item_guid = match item.guid() {
            Some(g) => g.value().to_string(),
            None => continue,
        };

        let enclosure_url = item.enclosure().map(|e| e.url().to_string());
        let item_link = item.link().map(|s| s.to_string());
        let pub_date = item.pub_date().map(|s| s.to_string());

        // Provisional music fields (ID3 will become canonical once downloaded)
        let track_title = item.title().map(|s| s.to_string());
        let artist_name: Option<String> = None;
        let album_title: Option<String> = None;
        let album_artist_name: Option<String> = None;
        let disc_number: Option<i64> = None;

        // Canonical ordering: podcast:episode
        let track_number: Option<i64> = find_ext_text(item.extensions(), "podcast", "episode")
            .and_then(|s| s.trim().parse::<i64>().ok());

        // iTunes item tags are NOT in extensions; rss crate exposes them via itunes_ext()
        let itunes = item.itunes_ext();
        let itunes_duration_raw = itunes.and_then(|it| it.duration()).map(|s| s.to_string());
        let duration_seconds: Option<i64> = itunes_duration_raw
            .as_deref()
            .and_then(parse_itunes_duration);
        let itunes_explicit = itunes.and_then(|it| it.explicit()).map(|s| s.to_string());
        let track_image_href = itunes.and_then(|it| it.image()).map(|s| s.to_string());
        let track_image_mime: Option<String> = None;

        // Item-level people/value (podcast:* extensions)
        let people_json = collect_people_json(item.extensions());
        let item_value_json = value_block_json(item.extensions(), "podcast", "value");

        let changed = tx.execute(
            r#"
            INSERT INTO tracks (
                feed_id,
                item_guid,
                enclosure_url,
                link,
                pub_date,
                track_title,
                artist_name,
                album_title,
                album_artist_name,
                disc_number,
                track_number,
                duration_seconds,
                itunes_duration_raw,
                itunes_explicit,
                track_image_href,
                track_image_mime,
                people_json,
                item_value_json
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
            ON CONFLICT(feed_id, item_guid) DO UPDATE SET
                enclosure_url       = excluded.enclosure_url,
                link                = excluded.link,
                pub_date            = excluded.pub_date,
                track_title         = excluded.track_title,
                artist_name         = excluded.artist_name,
                album_title         = excluded.album_title,
                album_artist_name   = excluded.album_artist_name,
                disc_number         = excluded.disc_number,
                track_number        = excluded.track_number,
                duration_seconds    = excluded.duration_seconds,
                itunes_duration_raw = excluded.itunes_duration_raw,
                itunes_explicit     = excluded.itunes_explicit,
                track_image_href    = excluded.track_image_href,
                track_image_mime    = excluded.track_image_mime,
                people_json         = excluded.people_json,
                item_value_json     = excluded.item_value_json
            "#,
            rusqlite::params![
                feed_id,
                item_guid,
                enclosure_url,
                item_link,
                pub_date,
                track_title,
                artist_name,
                album_title,
                album_artist_name,
                disc_number,
                track_number,
                duration_seconds,
                itunes_duration_raw,
                itunes_explicit,
                track_image_href,
                track_image_mime,
                people_json,
                item_value_json,
            ],
        )?;

        if changed > 0 {
            upserted += 1;
        }
    }

    tx.commit().context("commit tracks")?;

    println!("Subscribed/updated feed: {feed_title} (tracks upserted: {upserted})");
    Ok(())
}
