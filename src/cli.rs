//! Command-line integration surface for non-UI workflows.

use anyhow::{anyhow, Context, Result};
use rusqlite::Connection;
use serde::Serialize;

use crate::playback_driver::ConfiguredPlaybackDriver;
use crate::{api, config, db, debug_contracts, playback};

pub fn run(args: &[String]) -> Result<()> {
    match args {
        [command] if command == "help" || command == "--help" || command == "-h" => {
            print_help();
            Ok(())
        }
        [command, rest @ ..] if command == "now-playing" => print_now_playing(rest),
        [section, command, rest @ ..] if section == "liveitem" && command == "health" => {
            check_liveitem_health(rest)
        }
        [section, command, rest @ ..] if section == "liveitem" && command == "create" => {
            create_liveitem(rest)
        }
        [section, command, event_id, rest @ ..] if section == "liveitem" && command == "latest" => {
            print_liveitem_latest(event_id, rest)
        }
        [section, command, event_id, rest @ ..]
            if section == "liveitem" && command == "publish" =>
        {
            publish_liveitem_metadata(event_id, rest)
        }
        [section, command, event_id, rest @ ..]
            if section == "liveitem" && command == "publish-now-playing" =>
        {
            publish_liveitem_now_playing(event_id, rest)
        }
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
        [section, command] if section == "player" && command == "ping" => ping_player(),
        [section, command, track_id, flag]
            if section == "track" && command == "inspect" && flag == "--json" =>
        {
            print_track_inspect(parse_i64("track id", track_id)?)
        }
        [section, command, rest @ ..] if section == "playlist" && command == "play" => {
            play_playlist(rest)
        }
        [section, command, track_id] if section == "playback" && command == "set-track" => {
            set_track(parse_i64("track id", track_id)?)
        }
        [section, command, position_ms] if section == "playback" && command == "position" => {
            update_position(parse_u64("position ms", position_ms)?)
        }
        [section, command] if section == "playback" && command == "next" => skip_next(),
        [section, command] if section == "playback" && command == "previous" => skip_previous(),
        [section, command] if section == "playback" && command == "pause" => pause_playback(true),
        [section, command] if section == "playback" && command == "resume" => pause_playback(false),
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

fn configured_musicindex_client(endpoint: Option<&str>) -> Result<api::Client> {
    let base_url = match endpoint {
        Some(endpoint) => config::normalize_musicindex_endpoint(endpoint)?,
        None => {
            let cfg_path = config::config_path()?;
            config::load_musicindex_endpoint(&cfg_path)?
        }
    };
    Ok(api::Client::new_with_base_url(base_url))
}

fn print_now_playing(args: &[String]) -> Result<()> {
    parse_now_playing_options(args)?;
    let conn = open_configured_db()?;
    let update = playback::now_playing_update(&conn, playback::DEFAULT_SESSION_ID)?
        .context("no current playback session")?;
    print_json(&update)
}

fn check_liveitem_health(args: &[String]) -> Result<()> {
    let options = parse_live_options(args)?;
    options.ensure_unused(&["--json", "--token", "--metadata-json", "--dry-run"])?;

    let client = configured_musicindex_client(options.endpoint.as_deref())?;
    println!("{}", client.health()?);
    Ok(())
}

fn create_liveitem(args: &[String]) -> Result<()> {
    let options = parse_live_options(args)?;
    anyhow::ensure!(options.json, "liveitem create requires --json");
    options.ensure_unused(&["--token", "--metadata-json", "--dry-run"])?;

    let client = configured_musicindex_client(options.endpoint.as_deref())?;
    let response = client.create_live_item()?;
    print_json(&response)
}

fn print_liveitem_latest(event_id: &str, args: &[String]) -> Result<()> {
    let options = parse_live_options(args)?;
    anyhow::ensure!(options.json, "liveitem latest requires --json");
    options.ensure_unused(&["--token", "--metadata-json", "--dry-run"])?;

    let client = configured_musicindex_client(options.endpoint.as_deref())?;
    if let Some(response) = client.fetch_live_metadata_optional(event_id)? {
        return print_json(&response);
    }

    print_json(&LiveMetadataMissing {
        event_id,
        found: false,
        error: "metadata_not_found",
        message: "no metadata has been published for this live item yet",
    })
}

fn publish_liveitem_metadata(event_id: &str, args: &[String]) -> Result<()> {
    let options = parse_live_options(args)?;
    options.ensure_unused(&["--json", "--dry-run"])?;
    let metadata = options.metadata_json_value()?;
    let token = options.token_or_env()?;

    let request = api::LiveMetadataPublishRequest {
        event_id: event_id.to_string(),
        metadata,
    };
    let client = configured_musicindex_client(options.endpoint.as_deref())?;
    let response = client.publish_live_metadata_with_token(event_id, &token, &request)?;
    print_json(&response)
}

fn publish_liveitem_now_playing(event_id: &str, args: &[String]) -> Result<()> {
    let options = parse_live_options(args)?;
    options.ensure_unused(&["--json", "--metadata-json"])?;

    let conn = open_configured_db()?;
    let update = playback::now_playing_update(&conn, playback::DEFAULT_SESSION_ID)?
        .context("no current playback session")?;
    let request = api::LiveMetadataPublishRequest {
        event_id: event_id.to_string(),
        metadata: serde_json::to_value(update).context("serialize now-playing metadata")?,
    };

    if options.dry_run {
        return print_json(&request);
    }

    let token = options.token_or_env()?;
    let client = configured_musicindex_client(options.endpoint.as_deref())?;
    let response = client.publish_live_metadata_with_token(event_id, &token, &request)?;
    print_json(&response)
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

fn ping_player() -> Result<()> {
    let cfg_path = config::config_path()?;
    let cfg = config::load_config(&cfg_path)?;
    let driver = ConfiguredPlaybackDriver::from_config(&cfg.playback)?;
    driver.ping()?;
    println!("ok {}", cfg.playback.driver.as_str());
    Ok(())
}

fn play_playlist(args: &[String]) -> Result<()> {
    let options = parse_playlist_play_options(args)?;
    let conn = open_configured_db()?;
    let update = if options.dry_run {
        playback::dry_run_playlist_at(
            &conn,
            options.playlist_id,
            options.position,
            playback::DEFAULT_SESSION_ID,
        )?
    } else {
        playback::play_playlist_at(
            &conn,
            options.playlist_id,
            options.position,
            playback::DEFAULT_SESSION_ID,
        )?
    };
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

fn pause_playback(paused: bool) -> Result<()> {
    let conn = open_configured_db()?;
    let update = playback::update_paused(&conn, paused, playback::DEFAULT_SESSION_ID)?;
    print_json(&update)
}

fn skip_next() -> Result<()> {
    let conn = open_configured_db()?;
    let update = playback::skip_next(&conn, playback::DEFAULT_SESSION_ID)?;
    print_json(&update)
}

fn skip_previous() -> Result<()> {
    let conn = open_configured_db()?;
    let update = playback::skip_previous(&conn, playback::DEFAULT_SESSION_ID)?;
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

#[derive(Debug, Default)]
struct LiveOptions {
    json: bool,
    dry_run: bool,
    endpoint: Option<String>,
    token: Option<String>,
    metadata_json: Option<String>,
}

#[derive(Debug, Default)]
struct NowPlayingOptions {
    json: bool,
}

#[derive(Debug)]
struct PlaylistPlayOptions {
    playlist_id: i64,
    position: i64,
    dry_run: bool,
}

#[derive(Debug, Serialize)]
struct LiveMetadataMissing<'a> {
    event_id: &'a str,
    found: bool,
    error: &'static str,
    message: &'static str,
}

impl LiveOptions {
    fn ensure_unused(&self, forbidden: &[&str]) -> Result<()> {
        for flag in forbidden {
            match *flag {
                "--json" if self.json => return Err(anyhow!("{flag} is not valid here")),
                "--dry-run" if self.dry_run => return Err(anyhow!("{flag} is not valid here")),
                "--token" if self.token.is_some() => {
                    return Err(anyhow!("{flag} is not valid here"))
                }
                "--metadata-json" if self.metadata_json.is_some() => {
                    return Err(anyhow!("{flag} is not valid here"));
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn metadata_json_value(&self) -> Result<serde_json::Value> {
        let raw = self
            .metadata_json
            .as_ref()
            .context("liveitem publish requires --metadata-json '<json>'")?;
        serde_json::from_str(raw).with_context(|| "parse --metadata-json")
    }

    fn token_or_env(&self) -> Result<String> {
        if let Some(token) = &self.token {
            return Ok(token.clone());
        }
        std::env::var("MUSICINDEX_LIVEITEM_TOKEN")
            .context("liveitem publish requires --token or MUSICINDEX_LIVEITEM_TOKEN")
    }
}

fn parse_live_options(args: &[String]) -> Result<LiveOptions> {
    let mut options = LiveOptions::default();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                anyhow::ensure!(!options.json, "duplicate --json");
                options.json = true;
                index += 1;
            }
            "--dry-run" => {
                anyhow::ensure!(!options.dry_run, "duplicate --dry-run");
                options.dry_run = true;
                index += 1;
            }
            "--endpoint" => {
                let value = option_value(args, index, "--endpoint")?;
                anyhow::ensure!(options.endpoint.is_none(), "duplicate --endpoint");
                options.endpoint = Some(value.to_string());
                index += 2;
            }
            "--token" => {
                let value = option_value(args, index, "--token")?;
                anyhow::ensure!(options.token.is_none(), "duplicate --token");
                options.token = Some(value.to_string());
                index += 2;
            }
            "--metadata-json" => {
                let value = option_value(args, index, "--metadata-json")?;
                anyhow::ensure!(options.metadata_json.is_none(), "duplicate --metadata-json");
                options.metadata_json = Some(value.to_string());
                index += 2;
            }
            flag => return Err(anyhow!("unsupported liveitem option {flag:?}")),
        }
    }
    Ok(options)
}

fn parse_now_playing_options(args: &[String]) -> Result<NowPlayingOptions> {
    let mut options = NowPlayingOptions::default();
    for arg in args {
        match arg.as_str() {
            "--json" => {
                anyhow::ensure!(!options.json, "duplicate --json");
                options.json = true;
            }
            flag => return Err(anyhow!("unsupported now-playing option {flag:?}")),
        }
    }
    anyhow::ensure!(options.json, "now-playing requires --json");
    Ok(options)
}

fn parse_playlist_play_options(args: &[String]) -> Result<PlaylistPlayOptions> {
    let mut dry_run = false;
    let mut position = 0;
    let mut position_seen = false;
    let mut playlist_id = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--dry-run" => {
                anyhow::ensure!(!dry_run, "duplicate --dry-run");
                dry_run = true;
                index += 1;
            }
            "--position" => {
                let value = option_value(args, index, "--position")?;
                anyhow::ensure!(!position_seen, "duplicate --position");
                position = parse_i64("playlist position", value)?;
                position_seen = true;
                index += 2;
            }
            value if value.starts_with("--") => {
                return Err(anyhow!("unsupported playlist play option {value:?}"));
            }
            value => {
                anyhow::ensure!(playlist_id.is_none(), "duplicate playlist id");
                playlist_id = Some(parse_i64("playlist id", value)?);
                index += 1;
            }
        }
    }
    let playlist_id = playlist_id.context("playlist play requires <playlist-id>")?;
    anyhow::ensure!(position >= 0, "playlist position cannot be negative");
    Ok(PlaylistPlayOptions {
        playlist_id,
        position,
        dry_run,
    })
}

fn option_value<'a>(args: &'a [String], index: usize, flag: &str) -> Result<&'a str> {
    let value = args
        .get(index + 1)
        .with_context(|| format!("{flag} requires a value"))?;
    anyhow::ensure!(!value.starts_with("--"), "{flag} requires a value");
    Ok(value)
}

fn print_help() {
    println!("{}", help_text());
}

fn help_text() -> &'static str {
    "Usage:
  v4vmm
  v4vmm now-playing --json
  v4vmm liveitem health [--endpoint <url>]
  v4vmm liveitem create --json [--endpoint <url>]
  v4vmm liveitem latest <event-id> --json [--endpoint <url>]
  v4vmm liveitem publish <event-id> --metadata-json '<json>' [--token <token>] [--endpoint <url>]
  v4vmm liveitem publish-now-playing <event-id> [--token <token>] [--endpoint <url>]
  v4vmm liveitem publish-now-playing <event-id> --dry-run
  v4vmm playlists list --json
  v4vmm playlist tracks <playlist-id> --json
  v4vmm library tracks --json
  v4vmm player ping
  v4vmm track inspect <track-id> --json
  v4vmm playlist play <playlist-id> [--position <zero-based-position>]
  v4vmm playlist play <playlist-id> --dry-run [--position <zero-based-position>]
  v4vmm playback set-track <track-id>
  v4vmm playback position <ms>
  v4vmm playback next
  v4vmm playback previous
  v4vmm playback pause
  v4vmm playback resume
  v4vmm playback stop

No arguments starts the desktop UI. Phase 2 commands use the configured local
SQLite database and the default playback session. playlist play simulates
playback state without controlling an audio player. liveitem publish commands
can read the broadcaster token from MUSICINDEX_LIVEITEM_TOKEN."
}
