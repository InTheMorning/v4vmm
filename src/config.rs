// src/config.rs
use anyhow::{anyhow, Context, Result};
use directories::ProjectDirs;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::api::DEFAULT_BASE_URL;

#[derive(Debug, Deserialize)]
pub struct Config {
    /// Where v4vmm-managed audio files are stored.
    /// Example: "/home/user/V4VMusic"
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

pub fn save_musicindex_endpoint(cfg_path: &Path, endpoint: &str) -> Result<String> {
    let endpoint = normalize_musicindex_endpoint(endpoint)?;
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

    let updated = toml::to_string_pretty(&table).context("serialize config TOML")?;
    fs::write(cfg_path, updated.as_bytes())
        .with_context(|| format!("write config {}", cfg_path.display()))?;
    Ok(endpoint)
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
/// - db_path parent dir
pub fn ensure_dirs(cfg: &Config) -> Result<()> {
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
