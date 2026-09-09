//! Command-line integration surface for non-UI workflows.

use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Context, Result};
use rusqlite::Connection;
use serde::Serialize;

use crate::application::commands::payment_routes::{
    RepairMissingPaymentRouteTags, RepairPaymentRoutesForTrack,
};
use crate::application::{ApplicationQueryService, CommandBus, CommandContext};
use crate::broadcast::publisher_targets;
use crate::broadcast::registry::BroadcastRegistry;
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
        [section, command, ..] if section == "liveitem" && command == "create" => Err(anyhow!(
            "liveitem create is retired; use `v4vmm broadcast events create --json`"
        )),
        [section, command, event_id, rest @ ..] if section == "liveitem" && command == "latest" => {
            print_liveitem_latest(event_id, rest)
        }
        [section, area, command, rest @ ..]
            if section == "broadcast" && area == "events" && command == "list" =>
        {
            print_broadcast_events(rest)
        }
        [section, area, command, rest @ ..]
            if section == "broadcast" && area == "events" && command == "create" =>
        {
            create_broadcast_event(rest)
        }
        [section, area, command, event_id, rest @ ..]
            if section == "broadcast" && area == "events" && command == "forget" =>
        {
            forget_broadcast_event(event_id, rest)
        }
        [section, area, command, event_id, rest @ ..]
            if section == "broadcast" && area == "events" && command == "check" =>
        {
            check_broadcast_event(event_id, rest)
        }
        [section, area, command, rest @ ..]
            if section == "broadcast" && area == "targets" && command == "list" =>
        {
            print_broadcast_targets(rest)
        }
        [section, area, command, event_id, rest @ ..]
            if section == "broadcast" && area == "targets" && command == "attach" =>
        {
            attach_broadcast_target(event_id, rest)
        }
        [section, command, rest @ ..] if section == "broadcast" && command == "readiness" => {
            print_broadcast_readiness(rest)
        }
        [section, command, flag]
            if section == "broadcast" && command == "repair-routes" && flag == "--json" =>
        {
            repair_broadcast_routes()
        }
        [section, command, track_id, flag]
            if section == "broadcast" && command == "repair-routes" && flag == "--json" =>
        {
            repair_broadcast_routes_for_track(parse_i64("track id", track_id)?)
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
        [section, command, flag]
            if section == "library" && command == "repair-paths" && flag == "--json" =>
        {
            repair_library_paths()
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
    open_configured_db_with_config().map(|(_, conn)| conn)
}

fn open_configured_db_with_config() -> Result<(config::Config, Connection)> {
    let (cfg, conn) = open_configured_db_with_config_without_repair()?;
    db::repair_local_file_paths(&conn, &cfg.music_dir)?;
    Ok((cfg, conn))
}

fn open_configured_db_with_config_without_repair() -> Result<(config::Config, Connection)> {
    let cfg_path = config::config_path()?;
    let cfg = config::load_config(&cfg_path)?;
    config::ensure_dirs(&cfg)?;
    let conn = db::open_db(&cfg)?;
    Ok((cfg, conn))
}

fn configured_musicindex_client(endpoint: Option<&str>) -> Result<api::Client> {
    let base_url = match endpoint {
        Some(endpoint) => config::normalize_musicindex_endpoint(endpoint)?,
        None => configured_musicindex_endpoint()?,
    };
    Ok(api::Client::new_with_base_url(base_url))
}

fn configured_musicindex_endpoint() -> Result<String> {
    let cfg_path = config::config_path()?;
    config::load_musicindex_endpoint(&cfg_path)
}

fn configured_broadcast_registry(conn: &Connection) -> Result<BroadcastRegistry<'_>> {
    let cfg_path = config::config_path()?;
    let endpoint = config::load_musicindex_endpoint(&cfg_path)?;
    BroadcastRegistry::new(conn, &endpoint)
}

fn configured_broadcast_host() -> Result<config::BroadcastHostConfig> {
    let cfg_path = config::config_path()?;
    let cfg = config::load_config(&cfg_path)?;
    cfg.broadcast.selected_host().cloned()
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
    anyhow::ensure!(!options.json, "liveitem health does not support --json");

    let client = configured_musicindex_client(options.endpoint.as_deref())?;
    println!("{}", client.health()?);
    Ok(())
}

fn print_liveitem_latest(event_id: &str, args: &[String]) -> Result<()> {
    let options = parse_live_options(args)?;
    anyhow::ensure!(options.json, "liveitem latest requires --json");

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

fn print_broadcast_events(args: &[String]) -> Result<()> {
    parse_json_only_options("broadcast events list", args)?;
    let conn = open_configured_db()?;
    let registry = configured_broadcast_registry(&conn)?;
    print_json(&registry.list_events()?)
}

fn create_broadcast_event(args: &[String]) -> Result<()> {
    let options = parse_broadcast_event_create_options(args)?;
    anyhow::ensure!(options.json, "broadcast events create requires --json");

    let conn = open_configured_db()?;
    let registry = configured_broadcast_registry(&conn)?;
    let created = registry.create_event(options.label.as_deref())?;
    print_json(&created)
}

fn forget_broadcast_event(event_id: &str, args: &[String]) -> Result<()> {
    anyhow::ensure!(
        args.is_empty(),
        "broadcast events forget does not accept extra arguments"
    );
    let conn = open_configured_db()?;
    let registry = configured_broadcast_registry(&conn)?;
    let forgotten = registry
        .forget_event(event_id)?
        .with_context(|| format!("broadcast event not found: {event_id}"))?;
    println!("forgot broadcast event {}", forgotten.event_id);
    Ok(())
}

fn check_broadcast_event(event_id: &str, args: &[String]) -> Result<()> {
    parse_json_only_options("broadcast events check", args)?;
    let conn = open_configured_db()?;
    let registry = configured_broadcast_registry(&conn)?;
    let checked = registry.check_event(event_id)?;
    print_json(&checked)
}

fn print_broadcast_targets(args: &[String]) -> Result<()> {
    parse_json_only_options("broadcast targets list", args)?;
    let host = configured_broadcast_host()?;
    let targets = publisher_targets::list_targets(&host.transport, &host.instance_name)?;
    print_json(&targets)
}

fn attach_broadcast_target(event_id: &str, args: &[String]) -> Result<()> {
    let options = parse_broadcast_target_attach_options(args)?;
    let conn = open_configured_db()?;
    let event = db::broadcast_event_by_event_id(&conn, event_id)?
        .with_context(|| format!("broadcast event not found: {event_id}"))?;
    let host = configured_broadcast_host()?;

    publisher_targets::attach_event(
        &host.transport,
        &host.instance_name,
        &options.target,
        &event.event_id,
        Path::new(&event.token_path),
    )?;

    println!(
        "attached broadcast event {} to publisher target {}",
        event.event_id, options.target
    );
    Ok(())
}

fn print_broadcast_readiness(args: &[String]) -> Result<()> {
    parse_json_only_options("broadcast readiness", args)?;
    let (cfg, conn) = open_configured_db_with_config()?;
    let report =
        ApplicationQueryService::new().broadcast_readiness_report(&conn, &cfg.music_dir)?;
    print_json(&report)
}

fn repair_broadcast_routes() -> Result<()> {
    let (cfg, conn) = open_configured_db_with_config()?;
    let endpoint = configured_musicindex_endpoint()?;
    let shared = Arc::new(Mutex::new(conn));
    let outcome = CommandBus::new().execute(
        RepairMissingPaymentRouteTags::new(shared, endpoint, cfg.music_dir),
        &CommandContext::next(),
    )?;
    print_json(outcome.value())
}

fn repair_broadcast_routes_for_track(track_id: i64) -> Result<()> {
    let (cfg, conn) = open_configured_db_with_config()?;
    let endpoint = configured_musicindex_endpoint()?;
    let shared = Arc::new(Mutex::new(conn));
    let outcome = CommandBus::new().execute(
        RepairPaymentRoutesForTrack::new(shared, endpoint, cfg.music_dir, track_id),
        &CommandContext::next(),
    )?;
    print_json(outcome.value())
}

fn print_playlists() -> Result<()> {
    let conn = open_configured_db()?;
    let rows = debug_contracts::playlists(&conn)?;
    print_json(&rows)
}

fn print_playlist_tracks(playlist_id: i64) -> Result<()> {
    let (cfg, conn) = open_configured_db_with_config()?;
    let rows = debug_contracts::playlist_tracks(&conn, playlist_id, &cfg.music_dir)?;
    print_json(&rows)
}

fn print_library_tracks() -> Result<()> {
    let (cfg, conn) = open_configured_db_with_config()?;
    let rows = debug_contracts::library_tracks(&conn, &cfg.music_dir)?;
    print_json(&rows)
}

fn repair_library_paths() -> Result<()> {
    let (cfg, conn) = open_configured_db_with_config_without_repair()?;
    let repair = db::repair_local_file_paths(&conn, &cfg.music_dir)?;
    print_json(&repair)
}

fn print_track_inspect(track_id: i64) -> Result<()> {
    let (cfg, conn) = open_configured_db_with_config()?;
    let row = debug_contracts::track_inspect(&conn, track_id, &cfg.music_dir)?;
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
    endpoint: Option<String>,
}

#[derive(Debug, Default)]
struct BroadcastEventCreateOptions {
    json: bool,
    label: Option<String>,
}

#[derive(Debug, Eq, PartialEq)]
struct BroadcastTargetAttachOptions {
    target: String,
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
            "--endpoint" => {
                let value = option_value(args, index, "--endpoint")?;
                anyhow::ensure!(options.endpoint.is_none(), "duplicate --endpoint");
                options.endpoint = Some(value.to_string());
                index += 2;
            }
            flag => return Err(anyhow!("unsupported liveitem option {flag:?}")),
        }
    }
    Ok(options)
}

fn parse_broadcast_event_create_options(args: &[String]) -> Result<BroadcastEventCreateOptions> {
    let mut options = BroadcastEventCreateOptions::default();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                anyhow::ensure!(!options.json, "duplicate --json");
                options.json = true;
                index += 1;
            }
            "--label" => {
                let value = option_value(args, index, "--label")?;
                anyhow::ensure!(options.label.is_none(), "duplicate --label");
                options.label = Some(value.to_string());
                index += 2;
            }
            flag => {
                return Err(anyhow!(
                    "unsupported broadcast events create option {flag:?}"
                ))
            }
        }
    }
    Ok(options)
}

fn parse_broadcast_target_attach_options(args: &[String]) -> Result<BroadcastTargetAttachOptions> {
    let mut target = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--target" => {
                let value = option_value(args, index, "--target")?;
                anyhow::ensure!(target.is_none(), "duplicate --target");
                target = Some(value.trim().to_owned());
                index += 2;
            }
            flag => {
                return Err(anyhow!(
                    "unsupported broadcast targets attach option {flag:?}"
                ))
            }
        }
    }

    let target = target.context("broadcast targets attach requires --target <name>")?;
    anyhow::ensure!(!target.is_empty(), "broadcast target name cannot be empty");
    Ok(BroadcastTargetAttachOptions { target })
}

fn parse_json_only_options(command: &str, args: &[String]) -> Result<()> {
    let mut json = false;
    for arg in args {
        match arg.as_str() {
            "--json" => {
                anyhow::ensure!(!json, "duplicate --json");
                json = true;
            }
            flag => return Err(anyhow!("unsupported {command} option {flag:?}")),
        }
    }
    anyhow::ensure!(json, "{command} requires --json");
    Ok(())
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
  v4vmm liveitem latest <event-id> --json [--endpoint <url>]
  v4vmm broadcast events list --json
  v4vmm broadcast events create --json [--label <text>]
  v4vmm broadcast events forget <event-id>
  v4vmm broadcast events check <event-id> --json
  v4vmm broadcast targets list --json
  v4vmm broadcast targets attach <event-id> --target <name>
  v4vmm broadcast readiness --json
  v4vmm broadcast repair-routes --json
  v4vmm broadcast repair-routes <track-id> --json
  v4vmm playlists list --json
  v4vmm playlist tracks <playlist-id> --json
  v4vmm library tracks --json
  v4vmm library repair-paths --json
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
playback state without controlling an audio player."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_lists_library_repair_paths_json_command() {
        assert!(help_text().contains("v4vmm library repair-paths --json"));
    }

    #[test]
    fn help_lists_broadcast_repair_routes_json_commands() {
        assert!(help_text().contains("v4vmm broadcast repair-routes --json"));
        assert!(help_text().contains("v4vmm broadcast repair-routes <track-id> --json"));
    }
}
