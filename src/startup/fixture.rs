//! Debug-only isolated fixture support (ADR 0066). No GUI failure hooks in release.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{ensure, Context, Result};
use serde_json::json;

use crate::db::startup::prepare_database;

const FIXTURE_KIND: &str = "v4vmm-startup-recovery-v1";

/// Explicit debug activation is limited to this verified fixture's config path.
pub(crate) fn injected_failure(
    config_path: &Path,
    dependency: crate::application::capability::Dependency,
) -> bool {
    let Some(root) = std::env::var_os("V4VMM_STARTUP_FIXTURE") else {
        return false;
    };
    fixture_failure(Path::new(&root), config_path, dependency).unwrap_or(false)
}

fn fixture_failure(
    root: &Path,
    config_path: &Path,
    dependency: crate::application::capability::Dependency,
) -> Result<bool> {
    use crate::application::capability::Dependency;
    let root = verified_config_root(root, config_path)?;
    let state: serde_json::Value = serde_json::from_slice(&fs::read(root.join("case.json"))?)?;
    Ok(matches!(
        (state["case"].as_str(), dependency),
        (
            Some("runtime-unavailable" | "runtime-and-cache-unavailable"),
            Dependency::BackgroundRuntime
        ) | (
            Some("cache-worker-unavailable" | "runtime-and-cache-unavailable"),
            Dependency::ThumbnailMaintenance
        )
    ))
}

fn verified_config_root(root: &Path, config_path: &Path) -> Result<PathBuf> {
    let root = verified_root(&root.to_string_lossy())?;
    ensure!(
        config_path.canonicalize()? == root.join("config/v4vmm/config.toml").canonicalize()?,
        "Fixture config mismatch"
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("fixture.json"))?)?;
    ensure!(
        Path::new(manifest["binary"].as_str().context("fixture binary")?).canonicalize()?
            == std::env::current_exe()?.canonicalize()?,
        "Fixture binary mismatch"
    );
    Ok(root)
}

/// Keep mpv sockets inside a verified debug fixture, without changing the desktop runtime.
pub(crate) fn playback_runtime_directory(config_path: &Path) -> Option<PathBuf> {
    let root = std::env::var_os("V4VMM_STARTUP_FIXTURE")?;
    let root = verified_config_root(Path::new(&root), config_path).ok()?;
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("case.json")).ok()?).ok()?;
    Some(
        root.join(if state["case"] == "endpoint-and-player-unavailable" {
            "occupied-runtime"
        } else {
            "mpv-runtime"
        }),
    )
}

fn verified_root(path: &str) -> Result<PathBuf> {
    let root = Path::new(path)
        .canonicalize()
        .context("locate fixture directory")?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("fixture.json"))?)?;
    ensure!(
        manifest["kind"] == FIXTURE_KIND && manifest["root"].as_str() == root.to_str(),
        "Not an ADR 0066 startup fixture directory"
    );
    ensure!(
        root.file_name()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.starts_with("v4vmm-startup-")),
        "Unexpected fixture directory name"
    );
    Ok(root)
}

/// Handle explicit debug CLI fixture commands without opening a desktop window.
pub fn run_cli(args: &[String]) -> Result<()> {
    ensure!(
        args.len() == 2,
        "usage: startup-fixture seed|inspect DIRECTORY"
    );
    let root = verified_root(&args[1])?;
    let db_path = root.join("data/library.sqlite");
    match args[0].as_str() {
        "seed" => {
            ensure!(
                !db_path.exists(),
                "Fixture database already exists; use a fresh setup"
            );
            fs::create_dir_all(root.join("music"))?;
            fs::create_dir_all(root.join("config/v4vmm"))?;
            let conn = prepare_database(&db_path)
                .map_err(|e| anyhow::anyhow!("{:?}: {}", e.stage, e.reason))?;
            conn.execute(
                "INSERT INTO playlists(name) VALUES ('Startup fixture playlist')",
                [],
            )?;
            fs::write(
                root.join("music/unchanged-audio.bin"),
                b"startup fixture music bytes\n",
            )?;
            seed_tracks(&conn, &root)?;
            let config = format!("music_dir = {}\ndb_path = {}\nmusicindex_endpoint = \"http://127.0.0.1:9\"\n[playback]\ndriver = \"null\"\n",
                toml::Value::String(root.join("music").display().to_string()),
                toml::Value::String(db_path.display().to_string()));
            fs::write(root.join("config/v4vmm/config.toml"), config)?;
            println!("{}", json!({"seeded": root}));
        }
        "inspect" => {
            let conn = rusqlite::Connection::open_with_flags(
                &db_path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )?;
            let versions = conn
                .prepare("SELECT version FROM schema_migrations ORDER BY version")?
                .query_map([], |r| r.get::<_, i64>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let playlists: i64 =
                conn.query_row("SELECT count(*) FROM playlists", [], |r| r.get(0))?;
            let probes: i64 = conn.query_row(
                "SELECT count(*) FROM sqlite_schema WHERE name LIKE 'v4vmm_startup_probe_%'",
                [],
                |r| r.get(0),
            )?;
            let bindings = conn
                .prepare("SELECT path FROM local_files ORDER BY track_id")?
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let tracks: i64 = conn.query_row("SELECT count(*) FROM tracks", [], |r| r.get(0))?;
            let playlist_tracks: i64 =
                conn.query_row("SELECT count(*) FROM playlist_tracks", [], |r| r.get(0))?;
            println!(
                "{}",
                json!({"migration_versions": versions, "playlists": playlists, "database_probes": probes,
                    "bindings": bindings, "tracks": tracks, "playlist_tracks": playlist_tracks})
            );
        }
        "paths" => {
            let conn = rusqlite::Connection::open(&db_path)?;
            conn.execute_batch("DROP TRIGGER IF EXISTS fixture_stop_second_path;")?;
            for (id, name) in [(1, "a.wav"), (2, "b.wav"), (3, "c.wav")] {
                conn.execute(
                    "UPDATE local_files SET path = ?1 WHERE track_id = ?2",
                    rusqlite::params![name, id],
                )?;
            }
            let state: serde_json::Value =
                serde_json::from_slice(&fs::read(root.join("case.json"))?)?;
            if state["case"] == "partial-path-repair" {
                conn.execute_batch("UPDATE local_files SET path = '/old/music/' || path WHERE track_id IN (1, 2);
                    CREATE TRIGGER fixture_stop_second_path BEFORE UPDATE OF path ON local_files
                    WHEN OLD.track_id = 2 BEGIN SELECT RAISE(FAIL, 'fixture path repair failure'); END;")?;
            }
        }
        _ => anyhow::bail!("Unknown fixture command; use seed or inspect"),
    }
    Ok(())
}

fn seed_tracks(conn: &rusqlite::Connection, root: &Path) -> Result<()> {
    conn.execute("INSERT INTO feeds (id, feed_url, feed_guid, title) VALUES (1, 'fixture:rss', 'fixture-feed', 'Startup fixture')", [])?;
    // A quiet 30-second tone makes actual playback audible in a desktop session.
    let sample_count = 8_000_u32 * 30;
    let data_bytes = sample_count * 2;
    let mut wav = b"RIFF".to_vec();
    wav.extend((36 + data_bytes).to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16_u32.to_le_bytes());
    wav.extend(1_u16.to_le_bytes());
    wav.extend(1_u16.to_le_bytes());
    wav.extend(8_000_u32.to_le_bytes());
    wav.extend(16_000_u32.to_le_bytes());
    wav.extend(2_u16.to_le_bytes());
    wav.extend(16_u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend(data_bytes.to_le_bytes());
    for sample in 0..sample_count {
        let value: i16 = if sample % 40 < 20 { 300 } else { -300 };
        wav.extend(value.to_le_bytes());
    }
    for (id, name) in [(1, "a.wav"), (2, "b.wav"), (3, "c.wav")] {
        fs::write(root.join("music").join(name), &wav)?;
        conn.execute("INSERT INTO tracks (id, feed_id, item_guid, track_title, duration_seconds, is_in_library) VALUES (?1, 1, ?2, ?2, 30, 1)", rusqlite::params![id, name])?;
        conn.execute(
            "INSERT INTO local_files (path, track_id) VALUES (?1, ?2)",
            rusqlite::params![name, id],
        )?;
        conn.execute(
            "INSERT INTO playlist_tracks (playlist_id, track_id, position) VALUES (1, ?1, ?1)",
            [id],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::capability::Dependency;

    #[test]
    fn adr_0066_fixture_failures_require_identity_and_follow_fresh_case() {
        let temp = tempfile::Builder::new()
            .prefix("v4vmm-startup-")
            .tempdir()
            .unwrap();
        let root = temp.path().canonicalize().unwrap();
        let config = root.join("config/v4vmm/config.toml");
        fs::create_dir_all(config.parent().unwrap()).unwrap();
        fs::write(&config, "music_dir = 'fixture'\n").unwrap();
        fs::write(
            root.join("fixture.json"),
            serde_json::to_vec(&json!({
                "kind": FIXTURE_KIND, "root": root, "binary": std::env::current_exe().unwrap(),
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            root.join("case.json"),
            br#"{"case":"runtime-and-cache-unavailable"}"#,
        )
        .unwrap();
        assert!(fixture_failure(&root, &config, Dependency::BackgroundRuntime).unwrap());
        assert!(fixture_failure(&root, &config, Dependency::ThumbnailMaintenance).unwrap());
        fs::write(
            root.join("case.json"),
            br#"{"case":"cache-worker-unavailable"}"#,
        )
        .unwrap();
        assert!(!fixture_failure(&root, &config, Dependency::BackgroundRuntime).unwrap());
        assert!(fixture_failure(&root, &config, Dependency::ThumbnailMaintenance).unwrap());
        let other = root.join("other.toml");
        fs::write(&other, "").unwrap();
        assert!(fixture_failure(&root, &other, Dependency::BackgroundRuntime).is_err());
        fs::write(root.join("fixture.json"), b"{}").unwrap();
        assert!(fixture_failure(&root, &config, Dependency::BackgroundRuntime).is_err());
    }
}
