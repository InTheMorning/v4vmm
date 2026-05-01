// src/config.rs
use anyhow::{anyhow, Context, Result};
use directories::{BaseDirs, ProjectDirs};
use serde::{Deserialize, Deserializer};
use std::fs;
use std::path::{Path, PathBuf};

use crate::api::DEFAULT_BASE_URL;
use crate::ui::theme_profile::ThemeProfile;

#[derive(Debug, Deserialize)]
pub struct Config {
    /// Where v4vmm-managed audio files are stored.
    /// Example: "/home/user/V4Vmusic"
    pub music_dir: PathBuf,

    /// Where the sqlite DB lives.
    /// Example: "/home/user/.local/share/v4vmm/v4vmm.sqlite"
    pub db_path: PathBuf,

    /// Override for the `flac` CLI used to re-encode WAV downloads. When
    /// `None`, v4vmm resolves `flac` via `$PATH`. Install via your package
    /// manager (e.g. `apt install flac`, `brew install flac`). Without it,
    /// WAV downloads are left untagged.
    #[serde(default)]
    pub flac_path: Option<PathBuf>,

    /// Playback backend configuration. Missing config defaults to no playback
    /// driver so existing configs keep loading unchanged.
    #[serde(default)]
    pub playback: PlaybackConfig,

    /// Global UI scale factor. Mirrors iOS Dynamic Type's named steps.
    /// Missing value defaults to `medium` (1.0×).
    #[serde(default, deserialize_with = "deserialize_ui_scale")]
    pub ui_scale: UiScale,

    /// Runtime theme profile. Missing value defaults to the existing dark
    /// profile so older config files keep their appearance.
    #[serde(default)]
    pub theme_profile: ThemeProfile,
}

/// Persisted UI scale enum — TOML representation is a lowercase string
/// (`"x-small"`, `"small"`, `"medium"`, `"large"`, `"x-large"`).
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum UiScale {
    #[serde(rename = "x-small")]
    XSmall,
    #[serde(rename = "small")]
    Small,
    #[default]
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "large")]
    Large,
    #[serde(rename = "x-large")]
    XLarge,
}

impl UiScale {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::XSmall => "x-small",
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
            Self::XLarge => "x-large",
        }
    }
}

fn deserialize_ui_scale<'de, D>(deserializer: D) -> std::result::Result<UiScale, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    match raw.as_str() {
        "x-small" => Ok(UiScale::XSmall),
        "small" => Ok(UiScale::Small),
        "medium" => Ok(UiScale::Medium),
        "large" => Ok(UiScale::Large),
        "x-large" => Ok(UiScale::XLarge),
        other => Err(serde::de::Error::custom(format!(
            "unknown ui_scale {other:?}; expected one of \
             \"x-small\", \"small\", \"medium\", \"large\", \"x-large\""
        ))),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PlaybackConfig {
    #[serde(default, deserialize_with = "deserialize_playback_driver")]
    pub driver: PlaybackDriver,

    #[serde(default)]
    pub mpv_path: Option<PathBuf>,
}

impl Default for PlaybackConfig {
    fn default() -> Self {
        Self {
            driver: PlaybackDriver::Null,
            mpv_path: None,
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackDriver {
    #[default]
    Null,
    Mpv,
}

impl PlaybackDriver {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Mpv => "mpv",
        }
    }
}

fn deserialize_playback_driver<'de, D>(
    deserializer: D,
) -> std::result::Result<PlaybackDriver, D::Error>
where
    D: Deserializer<'de>,
{
    let driver = String::deserialize(deserializer)?;
    match driver.as_str() {
        "null" => Ok(PlaybackDriver::Null),
        "mpv" => Ok(PlaybackDriver::Mpv),
        other => Err(serde::de::Error::custom(format!(
            "unknown playback driver {other:?}; expected \"null\" or \"mpv\""
        ))),
    }
}

fn parse_playback_config(raw: &str) -> Result<PlaybackConfig> {
    let table = raw.parse::<toml::Table>().context("parse TOML")?;
    match table.get("playback") {
        Some(value) => value.clone().try_into().context("parse playback config"),
        None => Ok(PlaybackConfig::default()),
    }
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

    let _playback = parse_playback_config(&raw)
        .with_context(|| format!("parse playback config {}", cfg_path.display()))?;
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
    flac_path: &str,
    ui_scale: UiScale,
    theme_profile: ThemeProfile,
) -> Result<(String, PathBuf, Option<PathBuf>, UiScale, ThemeProfile)> {
    let endpoint = normalize_musicindex_endpoint(endpoint)?;
    let music_dir = normalize_music_dir(music_dir)?;
    let flac_path = normalize_flac_path(flac_path)?;
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
    table.insert(
        "ui_scale".into(),
        toml::Value::String(ui_scale.as_str().to_string()),
    );
    table.insert(
        "theme_profile".into(),
        toml::Value::String(theme_profile.as_str().to_string()),
    );
    match &flac_path {
        Some(path) => {
            table.insert(
                "flac_path".into(),
                toml::Value::String(path.display().to_string()),
            );
        }
        None => {
            table.remove("flac_path");
        }
    }

    let updated = toml::to_string_pretty(&table).context("serialize config TOML")?;
    fs::write(cfg_path, updated.as_bytes())
        .with_context(|| format!("write config {}", cfg_path.display()))?;
    Ok((endpoint, music_dir, flac_path, ui_scale, theme_profile))
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

/// Blank input clears the override (use `$PATH`). Otherwise expand `~` and
/// return an absolute-ish path; presence and executability are probed lazily
/// by `audio_format::flac_cli_available`.
pub fn normalize_flac_path(flac_path: &str) -> Result<Option<PathBuf>> {
    let trimmed = flac_path.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    if trimmed == "~" {
        return Ok(Some(home_dir()?));
    }

    if let Some(rest) = trimmed.strip_prefix("~/") {
        return Ok(Some(home_dir()?.join(rest)));
    }

    Ok(Some(PathBuf::from(trimmed)))
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

# Visual profile. Runtime settings currently expose "dark" and "light".
theme_profile = "dark"

# Optional override for the `flac` CLI used to silently upgrade WAV downloads
# to FLAC so they can be tagged. Leave unset to resolve `flac` via $PATH.
# Install the `flac` package from your OS (e.g. `apt install flac`,
# `brew install flac`). Without it, WAV downloads are kept as WAV and are not
# tagged.
# flac_path = "/usr/bin/flac"

# Playback backend. The default "null" driver disables playback.
# Uncomment to use mpv; leave mpv_path unset to resolve `mpv` via $PATH.
# [playback]
# driver = "mpv"
# mpv_path = "/usr/bin/mpv"
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
    fn load_config_defaults_missing_playback_to_null_driver() {
        let cfg = parse_playback_config(
            r#"
music_dir = "/tmp/music"
db_path = "/tmp/v4vmm.sqlite"
"#,
        )
        .expect("parse playback config");

        assert_eq!(cfg.driver, PlaybackDriver::Null);
        assert_eq!(cfg.mpv_path, None);
    }

    #[test]
    fn load_config_defaults_missing_theme_profile_to_dark() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cfg_path = temp.path().join("config.toml");
        fs::write(
            &cfg_path,
            r#"
music_dir = "/tmp/music"
db_path = "/tmp/v4vmm.sqlite"
"#,
        )
        .expect("write config");

        let cfg = load_config(&cfg_path).expect("load config");

        assert_eq!(cfg.theme_profile, ThemeProfile::Dark);
    }

    #[test]
    fn load_config_parses_theme_profile() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cfg_path = temp.path().join("config.toml");
        fs::write(
            &cfg_path,
            r#"
music_dir = "/tmp/music"
db_path = "/tmp/v4vmm.sqlite"
theme_profile = "light"
"#,
        )
        .expect("write config");

        let cfg = load_config(&cfg_path).expect("load config");

        assert_eq!(cfg.theme_profile, ThemeProfile::Light);
    }

    #[test]
    fn load_config_rejects_unknown_theme_profile() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cfg_path = temp.path().join("config.toml");
        fs::write(
            &cfg_path,
            r#"
music_dir = "/tmp/music"
db_path = "/tmp/v4vmm.sqlite"
theme_profile = "solarized"
"#,
        )
        .expect("write config");

        let error = load_config(&cfg_path).expect_err("unknown theme profile should fail");
        let message = format!("{error:#}");

        assert!(
            message.contains("unknown variant `solarized`"),
            "unexpected error: {message}"
        );
    }

    #[test]
    fn load_config_parses_mpv_playback_config() {
        let cfg = parse_playback_config(
            r#"
music_dir = "/tmp/music"
db_path = "/tmp/v4vmm.sqlite"

[playback]
driver = "mpv"
mpv_path = "/usr/bin/mpv"
"#,
        )
        .expect("parse playback config");

        assert_eq!(cfg.driver, PlaybackDriver::Mpv);
        assert_eq!(cfg.mpv_path, Some(PathBuf::from("/usr/bin/mpv")));
    }

    #[test]
    fn load_config_rejects_unknown_playback_driver() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cfg_path = temp.path().join("config.toml");
        fs::write(
            &cfg_path,
            r#"
music_dir = "/tmp/music"
db_path = "/tmp/v4vmm.sqlite"

[playback]
driver = "vlc"
"#,
        )
        .expect("write config");

        let error = load_config(&cfg_path).expect_err("unknown driver should fail");
        let message = format!("{error:#}");

        assert!(
            message.contains("unknown playback driver \"vlc\""),
            "unexpected error: {message}"
        );
        assert!(
            message.contains("expected \"null\" or \"mpv\""),
            "unexpected error: {message}"
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

        let (endpoint, music_dir, flac_path, ui_scale, theme_profile) = save_app_settings(
            &cfg_path,
            "api.musicindex.org/",
            "~/V4Vmusic",
            "/usr/bin/flac",
            UiScale::Medium,
            ThemeProfile::Light,
        )
        .expect("save");
        let raw = fs::read_to_string(&cfg_path).expect("read config");
        let table = raw.parse::<toml::Table>().expect("parse TOML");

        assert_eq!(endpoint, DEFAULT_BASE_URL);
        assert_eq!(music_dir, default_music_dir().expect("default music dir"));
        assert_eq!(flac_path, Some(PathBuf::from("/usr/bin/flac")));
        assert_eq!(ui_scale, UiScale::Medium);
        assert_eq!(theme_profile, ThemeProfile::Light);
        assert_eq!(
            table.get("flac_path").and_then(toml::Value::as_str),
            Some("/usr/bin/flac")
        );
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
            table.get("theme_profile").and_then(toml::Value::as_str),
            Some("light")
        );
        assert_eq!(
            table.get("extra").and_then(toml::Value::as_str),
            Some("keep")
        );
    }
}
