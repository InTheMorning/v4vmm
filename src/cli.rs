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
    let settings = &CliConfig::default();
    match args {
        [command] if command == "help" || command == "--help" || command == "-h" => {
            print_help();
            Ok(())
        }
        [command, rest @ ..] if command == "now-playing" => print_now_playing(settings, rest),
        [section, command, rest @ ..] if section == "liveitem" && command == "health" => {
            check_liveitem_health(settings, rest)
        }
        [section, command, ..] if section == "liveitem" && command == "create" => Err(anyhow!(
            "liveitem create is retired; use `v4vmm broadcast events create --json`"
        )),
        [section, command, event_id, rest @ ..] if section == "liveitem" && command == "latest" => {
            print_liveitem_latest(settings, event_id, rest)
        }
        [section, area, command, rest @ ..]
            if section == "broadcast" && area == "events" && command == "list" =>
        {
            print_broadcast_events(settings, rest)
        }
        [section, area, command, rest @ ..]
            if section == "broadcast" && area == "events" && command == "create" =>
        {
            create_broadcast_event(settings, rest)
        }
        [section, area, command, event_id, rest @ ..]
            if section == "broadcast" && area == "events" && command == "forget" =>
        {
            forget_broadcast_event(settings, event_id, rest)
        }
        [section, area, command, event_id, rest @ ..]
            if section == "broadcast" && area == "events" && command == "check" =>
        {
            check_broadcast_event(settings, event_id, rest)
        }
        [section, area, command, rest @ ..]
            if section == "broadcast" && area == "targets" && command == "list" =>
        {
            print_broadcast_targets(settings, rest)
        }
        [section, area, command, event_id, rest @ ..]
            if section == "broadcast" && area == "targets" && command == "attach" =>
        {
            attach_broadcast_target(settings, event_id, rest)
        }
        [section, command, rest @ ..] if section == "broadcast" && command == "readiness" => {
            print_broadcast_readiness(settings, rest)
        }
        [section, command, flag]
            if section == "broadcast" && command == "repair-routes" && flag == "--json" =>
        {
            repair_broadcast_routes(settings)
        }
        [section, command, track_id, flag]
            if section == "broadcast" && command == "repair-routes" && flag == "--json" =>
        {
            repair_broadcast_routes_for_track(settings, parse_i64("track id", track_id)?)
        }
        [section, command, flag]
            if section == "playlists" && command == "list" && flag == "--json" =>
        {
            print_playlists(settings)
        }
        [section, command, playlist_id, flag]
            if section == "playlist" && command == "tracks" && flag == "--json" =>
        {
            print_playlist_tracks(settings, parse_i64("playlist id", playlist_id)?)
        }
        [section, command, flag]
            if section == "library" && command == "tracks" && flag == "--json" =>
        {
            print_library_tracks(settings)
        }
        [section, command, flag]
            if section == "library" && command == "repair-paths" && flag == "--json" =>
        {
            repair_library_paths(settings)
        }
        [section, command] if section == "player" && command == "ping" => ping_player(settings),
        [section, command, track_id, flag]
            if section == "track" && command == "inspect" && flag == "--json" =>
        {
            print_track_inspect(settings, parse_i64("track id", track_id)?)
        }
        [section, command, rest @ ..] if section == "playlist" && command == "play" => {
            play_playlist(settings, rest)
        }
        [section, command, track_id] if section == "playback" && command == "set-track" => {
            set_track(settings, parse_i64("track id", track_id)?)
        }
        [section, command, position_ms] if section == "playback" && command == "position" => {
            update_position(settings, parse_u64("position ms", position_ms)?)
        }
        [section, command] if section == "playback" && command == "next" => skip_next(settings),
        [section, command] if section == "playback" && command == "previous" => {
            skip_previous(settings)
        }
        [section, command] if section == "playback" && command == "pause" => {
            pause_playback(settings, true)
        }
        [section, command] if section == "playback" && command == "resume" => {
            pause_playback(settings, false)
        }
        [section, command] if section == "playback" && command == "stop" => stop_playback(settings),
        _ => Err(anyhow!("unsupported command\n\n{}", help_text())),
    }
}

#[derive(Default)]
struct CliConfig {
    path: Option<std::path::PathBuf>,
    snapshot: std::cell::OnceCell<Result<config::ConfigSnapshot>>,
}

impl CliConfig {
    fn snapshot(&self) -> Result<&config::ConfigSnapshot> {
        self.snapshot
            .get_or_init(|| {
                let path = self.path.clone().map_or_else(config::config_path, Ok)?;
                config::load_config_snapshot(&path)
            })
            .as_ref()
            .map_err(|error| anyhow!("{error:#}"))
    }
}

fn open_configured_db(settings: &CliConfig) -> Result<Connection> {
    let snapshot = settings.snapshot()?;
    let db_path = snapshot.db_path.as_ref().map_err(|issue| *issue)?;
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).context("prepare database directory")?;
    }
    db::open_db(db_path)
}

fn open_configured_db_with_music_dir(
    settings: &CliConfig,
) -> Result<(std::path::PathBuf, Connection)> {
    let (music_dir, conn) = open_configured_db_with_music_dir_without_repair(settings)?;
    let db_path = settings
        .snapshot()?
        .db_path
        .as_ref()
        .map_err(|issue| *issue)?;
    match crate::startup::prepare_local_paths(&conn, &music_dir, db_path) {
        Ok(true) => eprintln!("App could not finish local path repair. App verified music storage and SQLite; completed changes and remaining bindings are retained. Only validated relative bindings can drive file operations."),
        Ok(false) => {},
        Err(outcome) => return Err(anyhow!("App cannot use core storage after path repair: {}", outcome.issues.iter().map(|issue| issue.cause.as_str()).collect::<Vec<_>>().join("; "))),
    }
    Ok((music_dir, conn))
}

fn open_configured_db_with_music_dir_without_repair(
    settings: &CliConfig,
) -> Result<(std::path::PathBuf, Connection)> {
    let snapshot = settings.snapshot()?;
    let music_dir = snapshot.music_dir.clone()?;
    let conn = open_configured_db(settings)?;
    Ok((music_dir, conn))
}

fn configured_musicindex_client(
    settings: &CliConfig,
    endpoint: Option<&str>,
) -> Result<api::Client> {
    let base_url = match endpoint {
        Some(endpoint) => config::normalize_musicindex_endpoint(endpoint)?,
        None => configured_musicindex_endpoint(settings)?,
    };
    Ok(api::Client::new_with_base_url(base_url))
}

fn configured_musicindex_endpoint(settings: &CliConfig) -> Result<String> {
    Ok(settings.snapshot()?.musicindex_endpoint.clone()?)
}

fn configured_broadcast_registry<'a>(
    settings: &CliConfig,
    conn: &'a Connection,
) -> Result<BroadcastRegistry<'a>> {
    let endpoint = settings.snapshot()?.musicindex_endpoint.clone()?;
    BroadcastRegistry::new(conn, &endpoint)
}

fn configured_broadcast_host(settings: &CliConfig) -> Result<config::BroadcastHostConfig> {
    settings.snapshot()?.broadcast().selected_host().cloned()
}

fn print_now_playing(settings: &CliConfig, args: &[String]) -> Result<()> {
    parse_now_playing_options(args)?;
    let conn = open_configured_db(settings)?;
    let update = playback::now_playing_update(&conn, playback::DEFAULT_SESSION_ID)?
        .context("no current playback session")?;
    print_json(&update)
}

fn check_liveitem_health(settings: &CliConfig, args: &[String]) -> Result<()> {
    let options = parse_live_options(args)?;
    anyhow::ensure!(!options.json, "liveitem health does not support --json");

    let client = configured_musicindex_client(settings, options.endpoint.as_deref())?;
    println!("{}", client.health()?);
    Ok(())
}

fn print_liveitem_latest(settings: &CliConfig, event_id: &str, args: &[String]) -> Result<()> {
    let options = parse_live_options(args)?;
    anyhow::ensure!(options.json, "liveitem latest requires --json");

    let client = configured_musicindex_client(settings, options.endpoint.as_deref())?;
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

fn print_broadcast_events(settings: &CliConfig, args: &[String]) -> Result<()> {
    parse_json_only_options("broadcast events list", args)?;
    let conn = open_configured_db(settings)?;
    print_json(&db::broadcast_events(&conn)?)
}

fn create_broadcast_event(settings: &CliConfig, args: &[String]) -> Result<()> {
    let options = parse_broadcast_event_create_options(args)?;
    anyhow::ensure!(options.json, "broadcast events create requires --json");

    let conn = open_configured_db(settings)?;
    let registry = configured_broadcast_registry(settings, &conn)?;
    let created = registry.create_event(options.label.as_deref())?;
    print_json(&created)
}

fn forget_broadcast_event(settings: &CliConfig, event_id: &str, args: &[String]) -> Result<()> {
    anyhow::ensure!(
        args.is_empty(),
        "broadcast events forget does not accept extra arguments"
    );
    let conn = open_configured_db(settings)?;
    let forgotten = crate::broadcast::registry::forget_local_event(&conn, event_id)?
        .with_context(|| format!("broadcast event not found: {event_id}"))?;
    println!("forgot broadcast event {}", forgotten.event_id);
    Ok(())
}

fn check_broadcast_event(settings: &CliConfig, event_id: &str, args: &[String]) -> Result<()> {
    parse_json_only_options("broadcast events check", args)?;
    let conn = open_configured_db(settings)?;
    let event =
        db::broadcast_event_by_event_id(&conn, event_id)?.context("broadcast event not found")?;
    let registry = BroadcastRegistry::new(&conn, &event.endpoint)?;
    let checked = registry.check_event(event_id)?;
    print_json(&checked)
}

fn print_broadcast_targets(settings: &CliConfig, args: &[String]) -> Result<()> {
    parse_json_only_options("broadcast targets list", args)?;
    let host = configured_broadcast_host(settings)?;
    let targets = publisher_targets::list_targets(&host.transport, &host.instance_name)?;
    print_json(&targets)
}

fn attach_broadcast_target(settings: &CliConfig, event_id: &str, args: &[String]) -> Result<()> {
    let options = parse_broadcast_target_attach_options(args)?;
    let conn = open_configured_db(settings)?;
    let event = db::broadcast_event_by_event_id(&conn, event_id)?
        .with_context(|| format!("broadcast event not found: {event_id}"))?;
    let host = configured_broadcast_host(settings)?;

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

fn print_broadcast_readiness(settings: &CliConfig, args: &[String]) -> Result<()> {
    parse_json_only_options("broadcast readiness", args)?;
    let (music_dir, conn) = open_configured_db_with_music_dir(settings)?;
    let report = ApplicationQueryService::new().broadcast_readiness_report(&conn, &music_dir)?;
    print_json(&report)
}

fn repair_broadcast_routes(settings: &CliConfig) -> Result<()> {
    let (music_dir, conn) = open_configured_db_with_music_dir(settings)?;
    let endpoint = configured_musicindex_endpoint(settings)?;
    let shared = Arc::new(Mutex::new(conn));
    let outcome = CommandBus::new().execute(
        RepairMissingPaymentRouteTags::new(shared, endpoint, music_dir),
        &CommandContext::next(),
    )?;
    print_json(outcome.value())
}

fn repair_broadcast_routes_for_track(settings: &CliConfig, track_id: i64) -> Result<()> {
    let (music_dir, conn) = open_configured_db_with_music_dir(settings)?;
    let endpoint = configured_musicindex_endpoint(settings)?;
    let shared = Arc::new(Mutex::new(conn));
    let outcome = CommandBus::new().execute(
        RepairPaymentRoutesForTrack::new(shared, endpoint, music_dir, track_id),
        &CommandContext::next(),
    )?;
    print_json(outcome.value())
}

fn print_playlists(settings: &CliConfig) -> Result<()> {
    let conn = open_configured_db(settings)?;
    let rows = debug_contracts::playlists(&conn)?;
    print_json(&rows)
}

fn print_playlist_tracks(settings: &CliConfig, playlist_id: i64) -> Result<()> {
    let (music_dir, conn) = open_configured_db_with_music_dir(settings)?;
    let rows = debug_contracts::playlist_tracks(&conn, playlist_id, &music_dir)?;
    print_json(&rows)
}

fn print_library_tracks(settings: &CliConfig) -> Result<()> {
    let (music_dir, conn) = open_configured_db_with_music_dir(settings)?;
    let rows = debug_contracts::library_tracks(&conn, &music_dir)?;
    print_json(&rows)
}

fn repair_library_paths(settings: &CliConfig) -> Result<()> {
    let (music_dir, conn) = open_configured_db_with_music_dir_without_repair(settings)?;
    let repair = db::repair_local_file_paths(&conn, &music_dir)?;
    print_json(&repair)
}

fn print_track_inspect(settings: &CliConfig, track_id: i64) -> Result<()> {
    let (music_dir, conn) = open_configured_db_with_music_dir(settings)?;
    let row = debug_contracts::track_inspect(&conn, track_id, &music_dir)?;
    print_json(&row)
}

fn ping_player(settings: &CliConfig) -> Result<()> {
    let playback = settings.snapshot()?.playback()?;
    let driver = ConfiguredPlaybackDriver::from_config(&playback)?;
    driver.ping()?;
    println!("ok {}", playback.driver.as_str());
    Ok(())
}

fn play_playlist(settings: &CliConfig, args: &[String]) -> Result<()> {
    let options = parse_playlist_play_options(args)?;
    let conn = open_configured_db(settings)?;
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

fn set_track(settings: &CliConfig, track_id: i64) -> Result<()> {
    let conn = open_configured_db(settings)?;
    let update = playback::set_track(&conn, track_id, playback::DEFAULT_SESSION_ID)?;
    print_json(&update)
}

fn update_position(settings: &CliConfig, position_ms: u64) -> Result<()> {
    let conn = open_configured_db(settings)?;
    let update = playback::update_position(&conn, position_ms, playback::DEFAULT_SESSION_ID)?;
    print_json(&update)
}

fn pause_playback(settings: &CliConfig, paused: bool) -> Result<()> {
    let conn = open_configured_db(settings)?;
    let update = playback::update_paused(&conn, paused, playback::DEFAULT_SESSION_ID)?;
    print_json(&update)
}

fn skip_next(settings: &CliConfig) -> Result<()> {
    let conn = open_configured_db(settings)?;
    let update = playback::skip_next(&conn, playback::DEFAULT_SESSION_ID)?;
    print_json(&update)
}

fn skip_previous(settings: &CliConfig) -> Result<()> {
    let conn = open_configured_db(settings)?;
    let update = playback::skip_previous(&conn, playback::DEFAULT_SESSION_ID)?;
    print_json(&update)
}

fn stop_playback(settings: &CliConfig) -> Result<()> {
    let conn = open_configured_db(settings)?;
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

    #[test]
    fn adr_0066_cli_shares_one_snapshot_and_requires_only_used_fields() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        let db_path = temp.path().join("library.sqlite");
        let original = format!("music_dir = false\ndb_path = {:?}\nmusicindex_endpoint = 'https://original.test'\n[playback]\ndriver = 'broken'\n", db_path);
        std::fs::write(&path, &original).unwrap();
        let settings = CliConfig {
            path: Some(path.clone()),
            ..CliConfig::default()
        };
        assert_eq!(
            configured_musicindex_endpoint(&settings).unwrap(),
            "https://original.test"
        );
        let conn = open_configured_db(&settings).unwrap();
        assert!(db::broadcast_events(&conn).unwrap().is_empty());
        assert!(open_configured_db_with_music_dir(&settings).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        std::fs::write(
            &path,
            "musicindex_endpoint = 'https://changed.test'\ndb_path = false",
        )
        .unwrap();
        assert_eq!(
            configured_musicindex_endpoint(&settings).unwrap(),
            "https://original.test"
        );
        assert!(open_configured_db(&settings).is_ok());
        let fresh = CliConfig {
            path: Some(path),
            ..CliConfig::default()
        };
        assert_eq!(
            configured_musicindex_endpoint(&fresh).unwrap(),
            "https://changed.test"
        );
        assert!(open_configured_db(&fresh).is_err());
    }

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
