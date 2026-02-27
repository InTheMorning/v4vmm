mod cli;
mod config;
mod db;

use crate::config::Config;
use anyhow::anyhow;
use anyhow::{Context, Result};
use id3::{Content, Tag, TagLike};
use rss::{
    extension::{Extension, ExtensionMap},
    Channel,
};
use rusqlite::Connection;
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path; // so functions below can still use `Config`

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cfg_path = config::config_path()?;
    let cfg = config::load_config(&cfg_path)?;
    config::ensure_dirs(&cfg)?;
    let mut db = db::open_db(&cfg)?;

    let cmd = cli::parse_args()?;
    match cmd {
        cli::Command::ShowConfig => cli::cmd_show_config(&cfg, &cfg_path),

        cli::Command::Id3Dump { path } => cmd_id3_dump(&cfg, &path),

        cli::Command::Subscribe { feed_url } => cmd_subscribe(&cfg, &mut db, &feed_url),

        cli::Command::RssDump { feed_url } => cmd_rss_dump(&feed_url),

        cli::Command::Help => {
            cli::print_help();
            Ok(())
        }
    }
}

fn find_ext<'a>(exts: &'a ExtensionMap, ns: &str, name: &str) -> Option<&'a Extension> {
    exts.get(ns)?.get(name)?.first()
}

fn find_ext_text(exts: &ExtensionMap, ns: &str, name: &str) -> Option<String> {
    find_ext(exts, ns, name)?.value.clone()
}

fn find_ext_attr(exts: &ExtensionMap, ns: &str, name: &str, attr: &str) -> Option<String> {
    find_ext(exts, ns, name)?.attrs.get(attr).cloned()
}

// podcast:person -> JSON array [{ name, attrs }, ...]
fn collect_people_json(exts: &ExtensionMap) -> Option<String> {
    let persons = exts.get("podcast")?.get("person")?;
    let arr: Vec<JsonValue> = persons
        .iter()
        .map(|p| {
            json!({
                "name": p.value,
                "attrs": p.attrs,
            })
        })
        .collect();
    serde_json::to_string(&arr).ok()
}

fn ext_to_json(ext: &Extension) -> JsonValue {
    let children = ext
        .children
        .iter()
        .map(|(k, vec)| {
            let arr: Vec<JsonValue> = vec.iter().map(ext_to_json).collect();
            (k.clone(), JsonValue::Array(arr))
        })
        .collect::<serde_json::Map<String, JsonValue>>();

    json!({
        "value": ext.value,
        "attrs": ext.attrs,
        "children": children,
    })
}

fn value_block_json(exts: &rss::extension::ExtensionMap, ns: &str, name: &str) -> Option<String> {
    let ext = find_ext(exts, ns, name)?;
    serde_json::to_string(&ext_to_json(ext)).ok()
}

// "123", "03:45", "1:02:03" -> seconds
fn parse_itunes_duration(raw: &str) -> Option<i64> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    let parts: Vec<&str> = raw.split(':').collect();
    let nums: Vec<i64> = parts
        .iter()
        .map(|p| p.parse::<i64>().ok())
        .collect::<Option<_>>()?;

    let secs = match nums.len() {
        1 => nums[0],
        2 => nums[0] * 60 + nums[1],
        3 => nums[0] * 3600 + nums[1] * 60 + nums[2],
        _ => return None,
    };
    Some(secs)
}

fn cmd_subscribe(_cfg: &Config, conn: &mut Connection, feed_url: &str) -> Result<()> {
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

fn cmd_id3_dump(_cfg: &Config, path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(anyhow!("file not found: {}", path.display()));
    }

    let tag = Tag::read_from_path(path)
        .with_context(|| format!("read ID3 tag from {}", path.display()))?;

    println!("File  : {}", path.display());
    println!("Title : {:?}", tag.title());
    println!("Artist: {:?}", tag.artist());
    println!("Album : {:?}", tag.album());

    // Print track number (TRCK) if present
    if let Some(trck) = first_text_frame(&tag, "TRCK") {
        println!("TRCK  : {}", trck);
    }

    // Dump TXXX frames (custom fields), including any V4V_* keys
    let txxx = read_txxx_map(&tag);
    if txxx.is_empty() {
        println!("TXXX  : (none)");
    } else {
        println!("TXXX  : {} entr(y/ies)", txxx.len());
        for (k, v) in txxx {
            println!("  - {} = {}", k, v);
        }
    }

    // Optional: show how many other frame IDs exist
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for frame in tag.frames() {
        *counts.entry(frame.id()).or_insert(0) += 1;
    }
    println!("Frames: {} unique IDs", counts.len());

    Ok(())
}

// tool to help us figure out how feeds are parsed
fn cmd_rss_dump(feed_url: &str) -> Result<()> {
    println!("Fetching: {feed_url}");

    let body = reqwest::blocking::Client::new()
        .get(feed_url)
        .send()
        .with_context(|| format!("GET {feed_url}"))?
        .error_for_status()
        .with_context(|| format!("HTTP error for {feed_url}"))?
        .bytes()
        .with_context(|| format!("read body {feed_url}"))?;

    let feed = Channel::read_from(Cursor::new(body)).context("parse RSS")?;

    println!("--- channel ---");
    println!("title: {}", feed.title());
    println!("link : {}", feed.link());
    println!("desc : {}", feed.description());
    dump_exts("channel extensions", feed.extensions());

    println!("--- items (first 3) ---");
    for (i, item) in feed.items().iter().take(3).enumerate() {
        println!("#{}", i + 1);
        println!("  title: {:?}", item.title());
        println!("  guid : {:?}", item.guid().map(|g| g.value()));
        println!("  enc  : {:?}", item.enclosure().map(|e| e.url()));
        println!("  pub  : {:?}", item.pub_date());

        // itunes tags
        let itunes = item.itunes_ext();

        let dur = itunes.and_then(|it| it.duration()).map(|s| s.to_string());
        let expl = itunes.and_then(|it| it.explicit()).map(|s| s.to_string());
        let img = itunes.and_then(|it| it.image()).map(|s| s.to_string());

        println!("  itunes:duration => {:?}", dur);
        println!("  itunes:explicit => {:?}", expl);
        println!("  itunes:image    => {:?}", img);

        dump_exts("  item extensions", item.extensions());
    }

    Ok(())
}

fn dump_exts(label: &str, exts: &ExtensionMap) {
    println!("{label}:");
    for (ns, keys) in exts {
        println!("  NS: {ns}");
        for (k, vec) in keys {
            println!("    key: {k} ({} value(s))", vec.len());
            for (j, ext) in vec.iter().enumerate() {
                dump_one_ext(j, ext, 6);
            }
        }
    }
}

fn dump_one_ext(idx: usize, ext: &Extension, indent: usize) {
    let pad = " ".repeat(indent);
    println!("{pad}[{idx}] value={:?} attrs={:?}", ext.value, ext.attrs);

    if !ext.children.is_empty() {
        // print only child keys + counts (keeps it readable)
        let child_summary: Vec<String> = ext
            .children
            .iter()
            .map(|(k, v)| format!("{k}={}", v.len()))
            .collect();
        println!("{pad}    children: {}", child_summary.join(", "));
    }
}

/// Return the first text-like value for a given frame id (e.g. "TRCK").
fn first_text_frame(tag: &Tag, id: &str) -> Option<String> {
    for f in tag.frames() {
        if f.id() != id {
            continue;
        }
        match f.content() {
            Content::Text(t) => return Some(t.to_string()),
            Content::ExtendedText(ext) => return Some(ext.value.to_string()),
            _ => {}
        }
    }
    None
}

/// Extract TXXX frames into a map: description -> value
fn read_txxx_map(tag: &Tag) -> HashMap<String, String> {
    let mut out = HashMap::new();

    for f in tag.frames() {
        if f.id() != "TXXX" {
            continue;
        }
        if let Content::ExtendedText(ext) = f.content() {
            // If duplicates exist, last one wins (fine for now)
            out.insert(ext.description.to_string(), ext.value.to_string());
        }
    }

    out
}
