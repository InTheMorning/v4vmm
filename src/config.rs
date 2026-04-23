// src/config.rs
use anyhow::{anyhow, Context, Result};
use directories::{BaseDirs, ProjectDirs};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::api::DEFAULT_BASE_URL;

#[derive(Debug, Deserialize)]
pub struct Config {
    /// Where v4vmm-managed audio files are stored.
    /// Example: "/home/user/V4Vmusic"
    pub music_dir: PathBuf,

    /// Where the sqlite DB lives.
    /// Example: "/home/user/.local/share/v4vmm/v4vmm.sqlite"
    pub db_path: PathBuf,
}

/// Determine the config path.
/// For now, Linux-first: use XDG config dir via `directories` crate.
/// Typically: ~/.config/v4vmm/config.toml
pub fn config_path() -> Result<PathBuf> {
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
pub fn load_config(cfg_path: &Path) -> Result<Config> {
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

pub fn load_musicindex_endpoint(cfg_path: &Path) -> Result<String> {
    if !cfg_path.exists() {
        let _ = load_config(cfg_path)?;
    }

    let raw = fs::read_to_string(cfg_path)
        .with_context(|| format!("read config {}", cfg_path.display()))?;
    let table = raw
        .parse::<toml::Table>()
        .with_context(|| format!("parse TOML {}", cfg_path.display()))?;

    match table
        .get("musicindex_endpoint")
        .and_then(toml::Value::as_str)
    {
        Some(endpoint) => normalize_musicindex_endpoint(endpoint),
        None => Ok(DEFAULT_BASE_URL.to_string()),
    }
}

pub fn save_app_settings(
    cfg_path: &Path,
    endpoint: &str,
    music_dir: &str,
) -> Result<(String, PathBuf)> {
    let endpoint = normalize_musicindex_endpoint(endpoint)?;
    let music_dir = normalize_music_dir(music_dir)?;
    if !cfg_path.exists() {
        let _ = load_config(cfg_path)?;
    }

    let raw = fs::read_to_string(cfg_path)
        .with_context(|| format!("read config {}", cfg_path.display()))?;
    let mut table = raw
        .parse::<toml::Table>()
        .with_context(|| format!("parse TOML {}", cfg_path.display()))?;
    table.insert(
        "musicindex_endpoint".into(),
        toml::Value::String(endpoint.clone()),
    );
    table.insert(
        "music_dir".into(),
        toml::Value::String(music_dir.display().to_string()),
    );

    let updated = toml::to_string_pretty(&table).context("serialize config TOML")?;
    fs::write(cfg_path, updated.as_bytes())
        .with_context(|| format!("write config {}", cfg_path.display()))?;
    Ok((endpoint, music_dir))
}

pub fn normalize_musicindex_endpoint(endpoint: &str) -> Result<String> {
    let trimmed = endpoint.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(anyhow!("musicindex_endpoint is empty"));
    }

    let candidate = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    };
    let url = reqwest::Url::parse(&candidate)
        .with_context(|| format!("parse musicindex_endpoint {candidate:?}"))?;
    match url.scheme() {
        "http" | "https" => {}
        scheme => return Err(anyhow!("unsupported musicindex_endpoint scheme: {scheme}")),
    }
    if url.host_str().is_none() {
        return Err(anyhow!("musicindex_endpoint must include a host"));
    }

    Ok(url.as_str().trim_end_matches('/').to_string())
}

fn home_dir() -> Result<PathBuf> {
    let base_dirs =
        BaseDirs::new().ok_or_else(|| anyhow!("could not determine user home directory"))?;
    Ok(base_dirs.home_dir().to_path_buf())
}

pub fn default_music_dir() -> Result<PathBuf> {
    Ok(home_dir()?.join("V4Vmusic"))
}

pub fn normalize_music_dir(music_dir: &str) -> Result<PathBuf> {
    let trimmed = music_dir.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("music_dir is empty"));
    }

    if trimmed == "~" {
        return home_dir();
    }

    if let Some(rest) = trimmed.strip_prefix("~/") {
        return Ok(home_dir()?.join(rest));
    }

    Ok(PathBuf::from(trimmed))
}

/// Default config content (TOML).
/// Uses your stated defaults.
fn default_config_toml() -> Result<String> {
    let proj = ProjectDirs::from("xyz", "HeyCitizen", "v4vmm")
        .ok_or_else(|| anyhow!("could not determine user directories"))?;

    // Default music dir: ~/V4Vmusic on Unix-like systems, equivalent home dir elsewhere.
    let music_dir = default_music_dir()?;

    // Default DB: ~/.local/share/v4vmm/v4vmm.sqlite
    let db_path = proj.data_dir().join("v4vmm.sqlite");

    Ok(format!(
        r#"# v4vmm config

# V4V-only library root
music_dir = "{}"

# SQLite database path (app data)
db_path = "{}"

# MusicIndex API endpoint
musicindex_endpoint = "{}"
"#,
        music_dir.display(),
        db_path.display(),
        DEFAULT_BASE_URL,
    ))
}

/// Ensure the on-disk dirs exist:
/// - music_dir
/// - music_dir/artists
/// - db_path parent dir
pub fn ensure_dirs(cfg: &Config) -> Result<()> {
    fs::create_dir_all(&cfg.music_dir)
        .with_context(|| format!("create music_dir {}", cfg.music_dir.display()))?;
    let artists_dir = cfg.music_dir.join("artists");
    fs::create_dir_all(&artists_dir)
        .with_context(|| format!("create artists dir {}", artists_dir.display()))?;

    let parent = cfg
        .db_path
        .parent()
        .ok_or_else(|| anyhow!("db_path has no parent: {}", cfg.db_path.display()))?;
    fs::create_dir_all(parent)
        .with_context(|| format!("create db parent dir {}", parent.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_music_dir_uses_v4vmusic_in_home() {
        let default_dir = default_music_dir().expect("default music dir");

        assert_eq!(
            default_dir.file_name().and_then(|name| name.to_str()),
            Some("V4Vmusic")
        );
        assert!(
            default_dir.starts_with(home_dir().expect("home dir")),
            "default music dir should live under the user home directory"
        );
    }

    #[test]
    fn normalize_music_dir_expands_home_prefix() {
        assert_eq!(
            normalize_music_dir("~/Music/V4V").expect("normalized music dir"),
            home_dir().expect("home dir").join("Music").join("V4V")
        );
    }

    #[test]
    fn normalize_music_dir_accepts_home_alias() {
        assert_eq!(
            normalize_music_dir("~").expect("normalized music dir"),
            home_dir().expect("home dir")
        );
    }

    #[test]
    fn normalize_music_dir_rejects_empty_value() {
        assert!(
            normalize_music_dir(" \t ").is_err(),
            "blank music directory should be rejected"
        );
    }

    #[test]
    fn save_app_settings_persists_music_dir_without_dropping_existing_values() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cfg_path = temp.path().join("config.toml");
        fs::write(
            &cfg_path,
            r#"
music_dir = "/tmp/old"
db_path = "/tmp/v4vmm.sqlite"
musicindex_endpoint = "https://old.example"
extra = "keep"
"#,
        )
        .expect("write config");

        let (endpoint, music_dir) =
            save_app_settings(&cfg_path, "api.musicindex.org/", "~/V4Vmusic").expect("save");
        let raw = fs::read_to_string(&cfg_path).expect("read config");
        let table = raw.parse::<toml::Table>().expect("parse TOML");

        assert_eq!(endpoint, DEFAULT_BASE_URL);
        assert_eq!(music_dir, default_music_dir().expect("default music dir"));
        assert_eq!(
            table
                .get("music_dir")
                .and_then(toml::Value::as_str)
                .map(PathBuf::from),
            Some(default_music_dir().expect("default music dir"))
        );
        assert_eq!(
            table
                .get("musicindex_endpoint")
                .and_then(toml::Value::as_str),
            Some(DEFAULT_BASE_URL)
        );
        assert_eq!(
            table.get("extra").and_then(toml::Value::as_str),
            Some("keep")
        );
    }
}
