use anyhow::{anyhow, Context, Result};
use directories::ProjectDirs;
use id3::{Content, Tag, TagLike};
use rss::{
    extension::{Extension, ExtensionMap},
    Channel,
};
use rusqlite::Connection;
use serde::Deserialize;
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]

struct Config {
    /// Where v4vmm-managed audio files are stored.
    /// Example: "/home/user/V4VMusic"
    music_dir: PathBuf,

    /// Where the sqlite DB lives.
    /// Example: "/home/user/.local/share/v4vmm/v4vmm.sqlite"
    db_path: PathBuf,
}

#[derive(Debug)]
enum Command {
    ShowConfig,
    Id3Dump { path: PathBuf },
    Subscribe { feed_url: String },
    Help,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cfg_path = config_path()?;
    let cfg = load_config(&cfg_path)?;
    ensure_dirs(&cfg)?;

    // Keep DB connection alive for the whole run
    let mut db = open_db(&cfg)?;

    let cmd = parse_args()?;
    match cmd {
        Command::ShowConfig => cmd_show_config(&cfg, &cfg_path),

        // id3-dump doesn't need DB (yet)
        Command::Id3Dump { path } => cmd_id3_dump(&cfg, &path),

        // New vertical slice commands will usually need DB:
        Command::Subscribe { feed_url } => cmd_subscribe(&cfg, &mut db, &feed_url),

        Command::Help => {
            print_help();
            Ok(())
        }
    }
}

fn print_help() {
    println!(
        r#"v4vmm (early prototype)

Usage:
  v4vmm show-config
  v4vmm id3-dump <path-to-mp3>
  v4vmm subscribe <feed-url> 

Notes:
  - Config file: ~/.config/v4vmm/config.toml
"#
    );
}

/// Determine the config path.
/// For now, Linux-first: use XDG config dir via `directories` crate.
/// Typically: ~/.config/v4vmm/config.toml
fn config_path() -> Result<PathBuf> {
    let proj = ProjectDirs::from("xyz", "HeyCitizen", "v4vmm")
        .ok_or_else(|| anyhow!("could not determine user config directory"))?;

    // Linux: ~/.config/v4vmm/config.toml (the crate handles the base)
    let mut path = proj.config_dir().to_path_buf();
    fs::create_dir_all(&path).with_context(|| format!("create config dir {}", path.display()))?;

    path.push("config.toml");
    Ok(path)
}

/// Load config from TOML.
/// If missing, writes a default config and returns it.
fn load_config(cfg_path: &Path) -> Result<Config> {
    if !cfg_path.exists() {
        let default = default_config_toml()?;
        fs::write(cfg_path, default.as_bytes())
            .with_context(|| format!("write default config {}", cfg_path.display()))?;

        println!(
            "Created default config at {}\nEdit it if needed, then re-run.",
            cfg_path.display()
        );
    }

    let raw = fs::read_to_string(cfg_path)
        .with_context(|| format!("read config {}", cfg_path.display()))?;

    let cfg: Config =
        toml::from_str(&raw).with_context(|| format!("parse TOML {}", cfg_path.display()))?;

    if cfg.music_dir.as_os_str().is_empty() {
        return Err(anyhow!("config: music_dir is empty"));
    }
    if cfg.db_path.as_os_str().is_empty() {
        return Err(anyhow!("config: db_path is empty"));
    }

    Ok(cfg)
}

/// Default config content (TOML).
/// Uses your stated defaults.
fn default_config_toml() -> Result<String> {
    let proj = ProjectDirs::from("xyz", "HeyCitizen", "v4vmm")
        .ok_or_else(|| anyhow!("could not determine user directories"))?;

    // Default music dir: ~/V4VMusic
    let home = std::env::var("HOME").map_err(|e| anyhow!("HOME not set: {e}"))?;
    let music_dir = PathBuf::from(&home).join("V4VMusic");

    // Default DB: ~/.local/share/v4vmm/v4vmm.sqlite
    let db_path = proj.data_dir().join("v4vmm.sqlite");

    Ok(format!(
        r#"# v4vmm config

# V4V-only library root
music_dir = "{}"

# SQLite database path (app data)
db_path = "{}"
"#,
        music_dir.display(),
        db_path.display(),
    ))
}

/// Ensure the on-disk dirs exist:
/// - music_dir
/// - db_path parent dir
fn ensure_dirs(cfg: &Config) -> Result<()> {
    fs::create_dir_all(&cfg.music_dir)
        .with_context(|| format!("create music_dir {}", cfg.music_dir.display()))?;

    let parent = cfg
        .db_path
        .parent()
        .ok_or_else(|| anyhow!("db_path has no parent: {}", cfg.db_path.display()))?;
    fs::create_dir_all(parent)
        .with_context(|| format!("create db parent dir {}", parent.display()))?;

    Ok(())
}

fn open_db(cfg: &Config) -> Result<Connection> {
    let db_path = &cfg.db_path;

    let conn = Connection::open(db_path)
        .with_context(|| format!("open/create db {}", db_path.display()))?;

    // Basic sanity / good defaults
    conn.pragma_update(None, "foreign_keys", "ON")
        .context("enable foreign_keys pragma")?;

    init_schema(&conn)?;
    Ok(conn)
}

fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS feeds (
    id              INTEGER PRIMARY KEY,

    -- identity
    feed_url        TEXT NOT NULL UNIQUE,         -- where we fetch from
    feed_guid       TEXT NULL,                    -- <podcast:guid> if present

    -- basic metadata (mostly RSS-level)
    title           TEXT NULL,                    -- channel title
    link            TEXT NULL,                    -- channel <link>
    language        TEXT NULL,
    description     TEXT NULL,
    podcast_medium  TEXT NULL,                    -- podcast:medium (e.g. "music")

    -- images (feed-level)
    album_image_href TEXT NULL,                   -- podcast:image / itunes:image URL
    album_image_mime TEXT NULL,                   -- optional, if we know it

    -- people at feed level (hosts, artists, etc.)
    people_json     TEXT NULL,                    -- JSON array of podcast:person / itunes:author etc.

    -- value block (feed-level)
    podcast_value_json TEXT NULL,                 -- JSON of <podcast:value> tree

    -- subscription / state
    is_subscribed   INTEGER NOT NULL DEFAULT 0,   -- 0/1: should we refresh this feed?
    last_fetched_at TEXT NOT NULL DEFAULT (datetime('now')),

    extra_json      TEXT NOT NULL DEFAULT '{}'    -- future stuff
);

CREATE INDEX IF NOT EXISTS idx_feeds_guid          ON feeds(feed_guid);
CREATE INDEX IF NOT EXISTS idx_feeds_is_subscribed ON feeds(is_subscribed);

CREATE TABLE IF NOT EXISTS tracks (
    id              INTEGER PRIMARY KEY,

    -- identity
    feed_id         INTEGER NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
    item_guid       TEXT NOT NULL,                -- <guid> (stable ID)
    enclosure_url   TEXT NULL,                    -- original audio URL
    link            TEXT NULL,                    -- item <link>
    pub_date        TEXT NULL,                    -- item pubDate (for "recent" views)

    -- music library metadata (ID3-style, to be filled from tags later)
    track_title         TEXT NULL,                -- TIT2
    artist_name         TEXT NULL,                -- TPE1
    album_title         TEXT NULL,                -- TALB
    album_artist_name   TEXT NULL,                -- TPE2
    disc_number         INTEGER NULL,             -- TPOS (normalized)
    track_number        INTEGER NULL,             -- canonical ordering (podcast:episode / ID3)

    -- duration / explicit
    duration_seconds    INTEGER NULL,             -- normalized duration (from audio or itunes:duration)
    itunes_duration_raw TEXT NULL,                -- raw itunes:duration text
    itunes_explicit     TEXT NULL,                -- "yes"/"no"/"clean" etc.

    -- images (item-level artwork if present)
    track_image_href    TEXT NULL,                -- item-specific image URL
    track_image_mime    TEXT NULL,

    -- people at item level (guests, performers, etc.)
    people_json         TEXT NULL,                -- JSON array of podcast:person etc.

    -- value block (item-level overrides)
    item_value_json     TEXT NULL,                -- JSON of item-level <podcast:value>

    -- user/library state
    is_in_library       INTEGER NOT NULL DEFAULT 0, -- 0/1: user chose this for their library

    extra_json          TEXT NOT NULL DEFAULT '{}',

    UNIQUE(feed_id, item_guid)
);

CREATE INDEX IF NOT EXISTS idx_tracks_feed_id       ON tracks(feed_id);
CREATE INDEX IF NOT EXISTS idx_tracks_track_number  ON tracks(feed_id, track_number);
CREATE INDEX IF NOT EXISTS idx_tracks_is_in_library ON tracks(is_in_library);

CREATE TABLE IF NOT EXISTS local_files (
    id                  INTEGER PRIMARY KEY,

    path                TEXT NOT NULL UNIQUE,     -- absolute or library-relative
    track_id            INTEGER NULL REFERENCES tracks(id) ON DELETE SET NULL,

    added_at            TEXT NOT NULL DEFAULT (datetime('now')),

    file_size_bytes     INTEGER NULL,
    audio_duration_sec  INTEGER NULL,             -- measured from file, if you ever want it
    checksum            TEXT NULL,                -- optional: hash/etag/etc.

    extra_json          TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_local_files_track_id ON local_files(track_id);

CREATE INDEX IF NOT EXISTS idx_tracks_feed_id ON tracks(feed_id);
CREATE INDEX IF NOT EXISTS idx_tracks_track_number ON tracks(feed_id, track_number);
CREATE INDEX IF NOT EXISTS idx_tracks_is_in_library ON tracks(is_in_library);
CREATE INDEX IF NOT EXISTS idx_local_files_track_id ON local_files(track_id);
"#,
    )
    .context("create tables")?;

    Ok(())
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

fn parse_args() -> Result<Command> {
    let mut args = env::args().skip(1); // skip program name

    match args.next().as_deref() {
        Some("show-config") => Ok(Command::ShowConfig),
        Some("subscribe") => {
            let u = args
                .next()
                .ok_or_else(|| anyhow!("subscribe requires a feed URL"))?;
            Ok(Command::Subscribe { feed_url: u })
        }
        Some("id3-dump") => {
            let p = args
                .next()
                .ok_or_else(|| anyhow!("id3-dump requires a path argument"))?;
            Ok(Command::Id3Dump {
                path: PathBuf::from(p),
            })
        }

        Some("help") | Some("-h") | Some("--help") | None => Ok(Command::Help),

        Some(other) => Err(anyhow!("unknown command: {other} (try: v4vmm help)")),
    }
}

fn cmd_show_config(cfg: &Config, cfg_path: &Path) -> Result<()> {
    println!("Config path : {}", cfg_path.display());
    println!("music_dir   : {}", cfg.music_dir.display());
    println!("db_path     : {}", cfg.db_path.display());
    Ok(())
}

fn cmd_subscribe(_cfg: &Config, conn: &mut Connection, feed_url: &str) -> Result<()> {
    println!("Fetching: {feed_url}");

    // HTTP fetch
    let body = reqwest::blocking::Client::new()
        .get(feed_url)
        .send()
        .with_context(|| format!("GET {feed_url}"))?
        .error_for_status()
        .with_context(|| format!("HTTP error for {feed_url}"))?
        .bytes()
        .with_context(|| format!("read body {feed_url}"))?;

    // Parse RSS
    let feed = Channel::read_from(Cursor::new(body)).context("parse RSS")?;

    // ----- feed-level fields -----
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
    // podcast:guid, podcast:medium
    let feed_guid = find_ext_text(feed.extensions(), "podcast", "guid");
    let podcast_medium = find_ext_text(feed.extensions(), "podcast", "medium");

    // images: prefer podcast:image/@href, then itunes:image/@href, then <image> url
    let mut album_image_href = find_ext_attr(feed.extensions(), "podcast", "image", "href")
        .or_else(|| find_ext_attr(feed.extensions(), "itunes", "image", "href"));
    if album_image_href.is_none() {
        if let Some(img) = feed.image() {
            album_image_href = Some(img.url().to_string());
        }
    }
    let album_image_mime: Option<String> = None; // can be filled later if you care

    // people at feed level
    let feed_people_json = collect_people_json(feed.extensions());

    // feed-level value block
    let podcast_value_json = value_block_json(feed.extensions(), "podcast", "value");

    // ----- UPSERT feed row -----
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
            feed_guid         = excluded.feed_guid,
            title             = excluded.title,
            link              = excluded.link,
            language          = excluded.language,
            description       = excluded.description,
            podcast_medium    = excluded.podcast_medium,
            album_image_href  = excluded.album_image_href,
            album_image_mime  = excluded.album_image_mime,
            people_json       = excluded.people_json,
            podcast_value_json= excluded.podcast_value_json,
            is_subscribed     = 1,
            last_fetched_at   = datetime('now')
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

    // ----- tracks -----
    let mut upserted = 0usize;
    let tx = conn.transaction().context("begin transaction")?;

    for item in feed.items() {
        // identity
        let item_guid = match item.guid() {
            Some(g) => g.value().to_string(),
            None => continue, // skip items with no GUID
        };

        let enclosure_url = item.enclosure().map(|e| e.url().to_string());
        let item_link = item.link().map(|s| s.to_string());
        let pub_date = item.pub_date().map(|s| s.to_string());

        // provisional music metadata (can be overridden by ID3 later)
        let track_title = item.title().map(|s| s.to_string());
        let artist_name: Option<String> = None;
        let album_title: Option<String> = None;
        let album_artist_name: Option<String> = None;
        let disc_number: Option<i64> = None;

        // track_number from podcast:episode
        let track_number: Option<i64> = find_ext_text(item.extensions(), "podcast", "episode")
            .and_then(|s| s.trim().parse::<i64>().ok());

        // image / duration / explicit
        let itunes_duration_raw = find_ext_text(item.extensions(), "itunes", "duration")
            .or_else(|| find_ext_text(item.extensions(), "itunes", "itunes:duration"));
        let duration_seconds: Option<i64> = itunes_duration_raw
            .as_deref()
            .and_then(parse_itunes_duration);
        let itunes_explicit = find_ext_text(item.extensions(), "itunes", "explicit")
            .or_else(|| find_ext_text(item.extensions(), "itunes", "itunes:explicit"));

        let track_image_href = find_ext_attr(item.extensions(), "itunes", "image", "href")
            .or_else(|| find_ext_attr(item.extensions(), "itunes", "itunes:image", "href"))
            .or_else(|| find_ext_attr(item.extensions(), "podcast", "image", "href"))
            .or_else(|| find_ext_attr(item.extensions(), "podcast", "podcast:image", "href"));
        let track_image_mime: Option<String> = None;

        // item-level people & value
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

    println!(
        "Subscribed/updated feed: {} (tracks upserted: {})",
        feed_title, upserted
    );

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
