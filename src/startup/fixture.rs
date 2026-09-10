//! Debug-only isolated fixture support (ADR 0066). No GUI failure hooks in release.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{ensure, Context, Result};
use serde_json::json;

use crate::db::startup::prepare_database;

const FIXTURE_KIND: &str = "v4vmm-startup-recovery-v1";

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
            println!(
                "{}",
                json!({"migration_versions": versions, "playlists": playlists, "database_probes": probes})
            );
        }
        _ => anyhow::bail!("Unknown fixture command; use seed or inspect"),
    }
    Ok(())
}
