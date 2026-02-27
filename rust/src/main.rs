use anyhow::{anyhow, Context, Result};
use directories::ProjectDirs;
use id3::{Content, Tag, TagLike};
use rss::Channel;
use rusqlite::params;
use rusqlite::Connection;
use serde::Deserialize;
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
            id INTEGER PRIMARY KEY,
            feed_url TEXT NOT NULL UNIQUE,
            feed_guid TEXT NULL,              -- podcast:guid if present
            title TEXT NULL,
            last_fetched_at TEXT NOT NULL DEFAULT (datetime('now')),
            extra_json TEXT NOT NULL DEFAULT '{}'
        );

        CREATE INDEX IF NOT EXISTS idx_feeds_guid ON feeds(feed_guid);

        CREATE TABLE IF NOT EXISTS tracks (
            id INTEGER PRIMARY KEY,
            feed_id INTEGER NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
            item_guid TEXT NOT NULL,          -- RSS <guid>
            title TEXT NULL,
            enclosure_url TEXT NULL,          -- <enclosure url=...>
            track_number INTEGER NULL,        -- podcast:episode (music-first ordering)
            extra_json TEXT NOT NULL DEFAULT '{}',
            UNIQUE(feed_id, item_guid)
        );

        CREATE TABLE IF NOT EXISTS local_files (
            id INTEGER PRIMARY KEY,
            path TEXT NOT NULL UNIQUE,
            track_id INTEGER NULL,
            added_at TEXT NOT NULL DEFAULT (datetime('now')),
            extra_json TEXT NOT NULL DEFAULT '{}'
            );
            
        CREATE INDEX IF NOT EXISTS idx_tracks_feed_id ON tracks(feed_id);
        CREATE INDEX IF NOT EXISTS idx_tracks_track_number ON tracks(feed_id, track_number);
        CREATE INDEX IF NOT EXISTS idx_local_files_track_id ON local_files(track_id);
        "#,
    )
    .context("create tables")?;

    Ok(())
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

    let body = reqwest::blocking::Client::new()
        .get(feed_url)
        .send()
        .with_context(|| format!("GET {feed_url}"))?
        .error_for_status()
        .with_context(|| format!("HTTP error for {feed_url}"))?
        .bytes()
        .with_context(|| format!("read body {feed_url}"))?;

    let feed = Channel::read_from(Cursor::new(&body[..])).context("parse RSS")?;
    let feed_title = feed.title().to_string();
    let feed_guid: Option<String> = find_ext_text(feed.extensions(), "podcast", "guid");

    // 1) upsert feed row
    conn.execute(
        r#"
        INSERT INTO feeds (feed_url, feed_guid, title, last_fetched_at)
        VALUES (?1, ?2, ?3, datetime('now'))
        ON CONFLICT(feed_url) DO UPDATE SET
            feed_guid = excluded.feed_guid,
            title = excluded.title,
            last_fetched_at = datetime('now')
        "#,
        params![feed_url, feed_guid, feed_title],
    )
    .context("upsert feed")?;

    let feed_id: i64 = conn
        .query_row(
            "SELECT id FROM feeds WHERE feed_url = ?1",
            params![feed_url],
            |row| row.get(0),
        )
        .context("lookup feed_id")?;

    // 2) upsert tracks
    let tx = conn.transaction().context("begin transaction")?;

    let mut updated = 0usize;

    for item in feed.items() {
        // item guid: RSS <guid> is the stable identity you want
        let item_guid = match item.guid() {
            Some(g) => g.value().to_string(),
            None => continue, // no stable identity, skip
        };

        let title: Option<String> = item.title().map(|s| s.to_string());

        // enclosure url (RSS has a real enclosure object)
        let enclosure_url: Option<String> = item.enclosure().map(|e| e.url().to_string());

        // podcast:episode as track number (best-effort parse)
        let track_number: Option<i64> = find_ext_text(item.extensions(), "podcast", "episode")
            .and_then(|s| s.trim().parse::<i64>().ok());

        let changed: usize = tx.execute(
            r#"
        INSERT INTO tracks (feed_id, item_guid, title, enclosure_url, track_number)
        VALUES (?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(feed_id, item_guid) DO UPDATE SET
            title = excluded.title,
            enclosure_url = excluded.enclosure_url,
            track_number = excluded.track_number
        "#,
            params![feed_id, item_guid, title, enclosure_url, track_number],
        )?;

        if changed > 0 {
            updated += 1;
        }
    }

    tx.commit().context("commit tracks")?;

    println!(
        "Subscribed/updated feed: {}{}",
        feed_title.clone(),
        feed_guid
            .as_ref()
            .map(|g| format!(" (podcast:guid={g})"))
            .unwrap_or_default()
    );
    println!("Tracks upserted: {}", updated);
    Ok(())
}

fn find_ext_text(exts: &rss::extension::ExtensionMap, ns: &str, name: &str) -> Option<String> {
    exts.get(ns)?.get(name)?.first()?.value.clone()
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
