//! Command-line integration surface for non-UI workflows.

use anyhow::{anyhow, Context, Result};
use rusqlite::Connection;
use serde::Serialize;

use crate::{config, db, debug_contracts, playback};

pub fn run(args: &[String]) -> Result<()> {
    match args {
        [command] if command == "help" || command == "--help" || command == "-h" => {
            print_help();
            Ok(())
        }
        [command, flag] if command == "now-playing" && flag == "--json" => print_now_playing(),
        [section, command, flag]
            if section == "playlists" && command == "list" && flag == "--json" =>
        {
            print_playlists()
        }
        [section, command, playlist_id, flag]
            if section == "playlist" && command == "tracks" && flag == "--json" =>
        {
            print_playlist_tracks(parse_i64("playlist id", playlist_id)?)
        }
        [section, command, flag]
            if section == "library" && command == "tracks" && flag == "--json" =>
        {
            print_library_tracks()
        }
        [section, command, track_id, flag]
            if section == "track" && command == "inspect" && flag == "--json" =>
        {
            print_track_inspect(parse_i64("track id", track_id)?)
        }
        [section, command, flag, playlist_id]
            if section == "playlist" && command == "play" && flag == "--dry-run" =>
        {
            dry_run_playlist(parse_i64("playlist id", playlist_id)?, 0)
        }
        [section, command, flag, playlist_id, position_flag, position]
            if section == "playlist"
                && command == "play"
                && flag == "--dry-run"
                && position_flag == "--position" =>
        {
            dry_run_playlist(
                parse_i64("playlist id", playlist_id)?,
                parse_i64("playlist position", position)?,
            )
        }
        [section, command, track_id] if section == "playback" && command == "set-track" => {
            set_track(parse_i64("track id", track_id)?)
        }
        [section, command, position_ms] if section == "playback" && command == "position" => {
            update_position(parse_u64("position ms", position_ms)?)
        }
        [section, command] if section == "playback" && command == "stop" => stop_playback(),
        _ => Err(anyhow!("unsupported command\n\n{}", help_text())),
    }
}

fn open_configured_db() -> Result<Connection> {
    let cfg_path = config::config_path()?;
    let cfg = config::load_config(&cfg_path)?;
    config::ensure_dirs(&cfg)?;
    db::open_db(&cfg)
}

fn print_now_playing() -> Result<()> {
    let conn = open_configured_db()?;
    let update = playback::now_playing_update(&conn, playback::DEFAULT_SESSION_ID)?
        .context("no current playback session")?;
    print_json(&update)
}

fn print_playlists() -> Result<()> {
    let conn = open_configured_db()?;
    let rows = debug_contracts::playlists(&conn)?;
    print_json(&rows)
}

fn print_playlist_tracks(playlist_id: i64) -> Result<()> {
    let conn = open_configured_db()?;
    let rows = debug_contracts::playlist_tracks(&conn, playlist_id)?;
    print_json(&rows)
}

fn print_library_tracks() -> Result<()> {
    let conn = open_configured_db()?;
    let rows = debug_contracts::library_tracks(&conn)?;
    print_json(&rows)
}

fn print_track_inspect(track_id: i64) -> Result<()> {
    let conn = open_configured_db()?;
    let row = debug_contracts::track_inspect(&conn, track_id)?;
    print_json(&row)
}

fn dry_run_playlist(playlist_id: i64, playlist_position: i64) -> Result<()> {
    anyhow::ensure!(
        playlist_position >= 0,
        "playlist position cannot be negative"
    );
    let conn = open_configured_db()?;
    let update = playback::dry_run_playlist_at(
        &conn,
        playlist_id,
        playlist_position,
        playback::DEFAULT_SESSION_ID,
    )?;
    print_json(&update)
}

fn set_track(track_id: i64) -> Result<()> {
    let conn = open_configured_db()?;
    let update = playback::set_track(&conn, track_id, playback::DEFAULT_SESSION_ID)?;
    print_json(&update)
}

fn update_position(position_ms: u64) -> Result<()> {
    let conn = open_configured_db()?;
    let update = playback::update_position(&conn, position_ms, playback::DEFAULT_SESSION_ID)?;
    print_json(&update)
}

fn stop_playback() -> Result<()> {
    let conn = open_configured_db()?;
    let session = playback::stop(&conn, playback::DEFAULT_SESSION_ID)?;
    println!(
        "stopped session {} at sequence {}",
        session.session_id, session.sequence
    );
    Ok(())
}

fn print_json<T: Serialize>(value: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(value).context("serialize JSON")?;
    println!("{json}");
    Ok(())
}

fn parse_i64(label: &str, value: &str) -> Result<i64> {
    value
        .parse::<i64>()
        .with_context(|| format!("parse {label} {value:?}"))
}

fn parse_u64(label: &str, value: &str) -> Result<u64> {
    value
        .parse::<u64>()
        .with_context(|| format!("parse {label} {value:?}"))
}

fn print_help() {
    println!("{}", help_text());
}

fn help_text() -> &'static str {
    "Usage:
  v4vmm
  v4vmm now-playing --json
  v4vmm playlists list --json
  v4vmm playlist tracks <playlist-id> --json
  v4vmm library tracks --json
  v4vmm track inspect <track-id> --json
  v4vmm playlist play --dry-run <playlist-id>
  v4vmm playlist play --dry-run <playlist-id> --position <zero-based-position>
  v4vmm playback set-track <track-id>
  v4vmm playback position <ms>
  v4vmm playback stop

No arguments starts the desktop UI. Phase 2 commands use the configured local
SQLite database and the default playback session."
}
