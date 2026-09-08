// src/config.rs
use anyhow::{anyhow, Context, Result};
use directories::{BaseDirs, ProjectDirs};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::api::DEFAULT_BASE_URL;
use crate::broadcast::encoder::EncoderTarget;
use crate::broadcast::producer::DropFileProducer;
use crate::broadcast::transport::Transport;
use crate::theme_profile::ThemeProfile;
use crate::view_models::workspace::{ContentViewMode, WorkspaceLayoutConfig};

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

    /// Broadcast host configuration. Missing config defaults to one local host.
    #[serde(default)]
    pub broadcast: BroadcastConfig,

    /// Global UI scale factor. Mirrors iOS Dynamic Type's named steps.
    /// Missing value defaults to `medium` (1.0×).
    #[serde(default, deserialize_with = "deserialize_ui_scale")]
    pub ui_scale: UiScale,

    /// Runtime theme profile. Missing value defaults to the existing dark
    /// profile so older config files keep their appearance.
    #[serde(default)]
    pub theme_profile: ThemeProfile,

    /// Additive ADR 0046 workspace layout persistence.
    ///
    /// Missing or malformed values fall back to the default workspace layout in
    /// the workspace VM, so older or manually edited configs keep loading.
    #[serde(default, deserialize_with = "deserialize_workspace_layout_config")]
    pub(crate) workspace_layout: Option<WorkspaceLayoutConfig>,

    /// Additive ADR 0051 workspace layout preferences.
    ///
    /// Missing or malformed values fall back to the default pane width in the
    /// app bootstrap, so older or manually edited configs keep loading.
    #[serde(default, deserialize_with = "deserialize_workspace_config")]
    pub(crate) workspace: Option<WorkspaceConfig>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub(crate) struct WorkspaceConfig {
    /// Forward-compatible workspace layout preferences.
    #[serde(default, deserialize_with = "deserialize_workspace_layout_prefs")]
    pub(crate) layout: Option<WorkspaceLayoutPrefs>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub(crate) struct WorkspaceLayoutPrefs {
    /// Persisted content-pane width in logical pixels.
    #[serde(
        default,
        deserialize_with = "deserialize_optional_f32",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) content_pane_width: Option<f32>,
    /// Persisted Music content-list presentation mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) content_list_view_mode: Option<ContentViewMode>,
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

fn deserialize_workspace_layout_config<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<WorkspaceLayoutConfig>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = toml::Value::deserialize(deserializer)?;
    match value.try_into::<WorkspaceLayoutConfig>() {
        Ok(config) => Ok(Some(config)),
        Err(error) => {
            eprintln!("v4vmm::config: ignoring malformed workspace_layout: {error}");
            Ok(None)
        }
    }
}

fn deserialize_workspace_config<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<WorkspaceConfig>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = toml::Value::deserialize(deserializer)?;
    match value.try_into::<WorkspaceConfig>() {
        Ok(config) => Ok(Some(config)),
        Err(error) => {
            eprintln!("v4vmm::config: ignoring malformed workspace: {error}");
            Ok(None)
        }
    }
}

fn deserialize_workspace_layout_prefs<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<WorkspaceLayoutPrefs>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = toml::Value::deserialize(deserializer)?;
    match value.try_into::<WorkspaceLayoutPrefs>() {
        Ok(config) => Ok(Some(config)),
        Err(error) => {
            eprintln!("v4vmm::config: ignoring malformed workspace.layout: {error}");
            Ok(None)
        }
    }
}

fn deserialize_optional_f32<'de, D>(deserializer: D) -> std::result::Result<Option<f32>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<toml::Value>::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(toml::Value::Float(value)) => Ok(Some(value as f32)),
        Some(toml::Value::Integer(value)) => Ok(Some(value as f32)),
        Some(other) => Err(serde::de::Error::custom(format!(
            "expected integer or float, got {other}"
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

/// Broadcast host list configuration.
#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct BroadcastConfig {
    /// Optional selected host name. When absent, the first host is selected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_host: Option<String>,
    /// Optional publisher watch directory for the built-in mpv producer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_directory: Option<PathBuf>,
    /// Publisher target name written by the built-in mpv producer.
    #[serde(default = "default_drop_file_target")]
    pub drop_file_target: String,
    /// Configured broadcast hosts.
    #[serde(default = "default_broadcast_hosts")]
    pub hosts: Vec<BroadcastHostConfig>,
    /// Optional stream encoder control configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoder: Option<BroadcastEncoderConfig>,
}

impl Default for BroadcastConfig {
    fn default() -> Self {
        Self {
            selected_host: None,
            drop_directory: None,
            drop_file_target: default_drop_file_target(),
            hosts: default_broadcast_hosts(),
            encoder: None,
        }
    }
}

impl BroadcastConfig {
    /// Return the selected broadcast host.
    ///
    /// # Errors
    ///
    /// Returns an error when the host list is empty or the selected host name
    /// does not match a configured host.
    pub fn selected_host(&self) -> Result<&BroadcastHostConfig> {
        let Some(selected_host) = self.selected_host.as_deref() else {
            return self
                .hosts
                .first()
                .ok_or_else(|| anyhow!("config: broadcast.hosts is empty"));
        };
        let selected_host = selected_host.trim();
        self.hosts
            .iter()
            .find(|host| host.name.trim() == selected_host)
            .ok_or_else(|| anyhow!("config: broadcast selected_host {selected_host:?} not found"))
    }

    /// Build the built-in mpv drop-file producer when it is configured.
    ///
    /// # Errors
    ///
    /// Returns an error when the configured target cannot be used as a visible
    /// file name.
    pub fn drop_file_producer(&self) -> Result<Option<DropFileProducer>> {
        self.drop_directory
            .as_ref()
            .map(|drop_directory| {
                DropFileProducer::new(drop_directory.clone(), self.drop_file_target.clone())
            })
            .transpose()
    }

    /// Validate host-list shape and selected host.
    ///
    /// # Errors
    ///
    /// Returns an error when a host field is empty, names are duplicated, the
    /// list is empty, the transport is invalid, or the selected host is absent.
    pub fn validate(&self) -> Result<()> {
        let mut names = BTreeSet::new();
        for host in &self.hosts {
            host.validate()?;
            let name = host.name.trim();
            if !names.insert(name.to_owned()) {
                return Err(anyhow!("config: duplicate broadcast host name {name:?}"));
            }
        }
        let _ = self.selected_host()?;
        if self.drop_directory.is_some() {
            let _ = DropFileProducer::validate_target_name(&self.drop_file_target)?;
        }
        if let Some(encoder) = &self.encoder {
            encoder.validate()?;
        }
        Ok(())
    }
}

/// One configured host that can own publisher-side broadcast services.
#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct BroadcastHostConfig {
    /// Curator-facing host name.
    pub name: String,
    /// Local or SSH command transport.
    #[serde(flatten)]
    pub transport: Transport,
    /// Publisher instance name used in the user service unit.
    pub instance_name: String,
}

impl BroadcastHostConfig {
    /// Return the default local broadcast host.
    #[must_use]
    pub fn default_local() -> Self {
        Self {
            name: "Local".to_owned(),
            transport: Transport::local(),
            instance_name: default_broadcast_instance_name(),
        }
    }

    /// Validate this host entry.
    ///
    /// # Errors
    ///
    /// Returns an error when the name, instance name, or transport is invalid.
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(anyhow!("config: broadcast host name is empty"));
        }
        if self.instance_name.trim().is_empty() {
            return Err(anyhow!(
                "config: broadcast host {} instance_name is empty",
                self.name
            ));
        }
        self.transport.validate()
    }
}

/// Optional stream encoder control configuration.
#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct BroadcastEncoderConfig {
    /// Path or binary name for the `butt` control command.
    #[serde(default = "default_encoder_binary_path")]
    pub binary_path: PathBuf,
    /// Optional network control address for an already-running encoder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    /// Optional network control port for an already-running encoder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// Default encoder server passed to the connect command.
    #[serde(default = "default_encoder_server_name")]
    pub default_server_name: String,
}

impl BroadcastEncoderConfig {
    /// Build the command target described by this config.
    ///
    /// # Errors
    ///
    /// Returns an error when a configured path is not UTF-8 or a required
    /// encoder field is empty.
    pub fn target(&self) -> Result<EncoderTarget> {
        let binary_path = self.binary_path.to_str().ok_or_else(|| {
            anyhow!(
                "config: broadcast.encoder binary_path must be UTF-8: {}",
                self.binary_path.display()
            )
        })?;
        EncoderTarget::new(binary_path.to_owned(), self.address.clone(), self.port)
    }

    /// Validate the encoder configuration.
    ///
    /// # Errors
    ///
    /// Returns an error when binary path, address, or default server name is
    /// empty.
    pub fn validate(&self) -> Result<()> {
        let _ = self.target()?;
        if self.default_server_name.trim().is_empty() {
            return Err(anyhow!(
                "config: broadcast.encoder default_server_name is empty"
            ));
        }
        Ok(())
    }
}

fn default_broadcast_hosts() -> Vec<BroadcastHostConfig> {
    vec![BroadcastHostConfig::default_local()]
}

fn default_broadcast_instance_name() -> String {
    "mixxx".to_owned()
}

fn default_drop_file_target() -> String {
    "default".to_owned()
}

fn default_encoder_binary_path() -> PathBuf {
    PathBuf::from(EncoderTarget::default_binary())
}

fn default_encoder_server_name() -> String {
    "default".to_owned()
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
    cfg.broadcast
        .validate()
        .with_context(|| format!("parse broadcast config {}", cfg_path.display()))?;

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

pub(crate) fn save_workspace_layout(
    cfg_path: &Path,
    workspace_layout: &WorkspaceLayoutConfig,
) -> Result<()> {
    if !cfg_path.exists() {
        let _ = load_config(cfg_path)?;
    }

    let raw = fs::read_to_string(cfg_path)
        .with_context(|| format!("read config {}", cfg_path.display()))?;
    let mut table = raw
        .parse::<toml::Table>()
        .with_context(|| format!("parse TOML {}", cfg_path.display()))?;
    let layout_value =
        toml::Value::try_from(workspace_layout).context("serialize workspace layout config")?;
    table.insert("workspace_layout".into(), layout_value);

    let updated = toml::to_string_pretty(&table).context("serialize config TOML")?;
    fs::write(cfg_path, updated.as_bytes())
        .with_context(|| format!("write config {}", cfg_path.display()))?;
    Ok(())
}

pub(crate) fn save_workspace_layout_prefs(
    cfg_path: &Path,
    workspace_layout_prefs: &WorkspaceLayoutPrefs,
) -> Result<()> {
    if !cfg_path.exists() {
        let _ = load_config(cfg_path)?;
    }

    let raw = fs::read_to_string(cfg_path)
        .with_context(|| format!("read config {}", cfg_path.display()))?;
    let mut table = raw
        .parse::<toml::Table>()
        .with_context(|| format!("parse TOML {}", cfg_path.display()))?;

    if !table.get("workspace").is_some_and(toml::Value::is_table) {
        table.insert("workspace".into(), toml::Value::Table(toml::Table::new()));
    }
    let workspace_table = table
        .get_mut("workspace")
        .and_then(toml::Value::as_table_mut)
        .expect("workspace was normalized to a table");
    if !workspace_table
        .get("layout")
        .is_some_and(toml::Value::is_table)
    {
        workspace_table.insert("layout".into(), toml::Value::Table(toml::Table::new()));
    }
    let layout_table = workspace_table
        .get_mut("layout")
        .and_then(toml::Value::as_table_mut)
        .expect("workspace.layout was normalized to a table");

    match workspace_layout_prefs.content_pane_width {
        Some(width) => {
            layout_table.insert(
                "content_pane_width".into(),
                toml::Value::Float(f64::from(width)),
            );
        }
        None => {
            layout_table.remove("content_pane_width");
        }
    }
    match workspace_layout_prefs.content_list_view_mode {
        Some(mode) => {
            layout_table.insert(
                "content_list_view_mode".into(),
                toml::Value::String(mode.id_suffix().to_string()),
            );
        }
        None => {
            layout_table.remove("content_list_view_mode");
        }
    }

    let updated = toml::to_string_pretty(&table).context("serialize config TOML")?;
    fs::write(cfg_path, updated.as_bytes())
        .with_context(|| format!("write config {}", cfg_path.display()))?;
    Ok(())
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

# Visual profile. Supported values: "system", "dark", "light",
# "high-contrast-dark", and "high-contrast-light".
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

# Broadcast host control. Missing section defaults to this local host.
# [broadcast]
# selected_host = "Local"
# mpv drop-file producer. Missing drop_directory disables this producer.
# drop_directory = "/run/user/1000/musicindex-live-publisher/mpv/nowplaying"
# drop_file_target = "default"
#
# [[broadcast.hosts]]
# name = "Local"
# transport = "local"
# instance_name = "mixxx"
#
# Stream encoder control. Missing group reports the encoder as not installed.
# [broadcast.encoder]
# binary_path = "butt"
# default_server_name = "default"
# address = "127.0.0.1"
# port = 1256

# Workspace layout is persisted automatically. Missing or malformed values
# fall back to the default layout.
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
    use crate::view_models::workspace::{WorkspaceFrameKind, WorkspaceFrameState, WorkspaceLayout};

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
    fn load_config_defaults_missing_broadcast_to_local_host() {
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
        let host = cfg.broadcast.selected_host().expect("selected host");

        assert_eq!(cfg.broadcast.hosts.len(), 1);
        assert_eq!(host.name, "Local");
        assert_eq!(host.transport, Transport::Local);
        assert_eq!(host.instance_name, "mixxx");
        assert_eq!(cfg.broadcast.drop_directory, None);
        assert_eq!(cfg.broadcast.drop_file_target, "default");
        assert!(cfg
            .broadcast
            .drop_file_producer()
            .expect("producer")
            .is_none());
        assert!(cfg.broadcast.encoder.is_none());
    }

    #[test]
    fn load_config_parses_broadcast_host_list_and_selection() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cfg_path = temp.path().join("config.toml");
        fs::write(
            &cfg_path,
            r#"
music_dir = "/tmp/music"
db_path = "/tmp/v4vmm.sqlite"

[broadcast]
selected_host = "Studio"

[[broadcast.hosts]]
name = "Local"
transport = "local"
instance_name = "mixxx"

[[broadcast.hosts]]
name = "Studio"
transport = "ssh"
destination = "studio-box"
instance_name = "remote-mixxx"
"#,
        )
        .expect("write config");

        let cfg = load_config(&cfg_path).expect("load config");
        let host = cfg.broadcast.selected_host().expect("selected host");

        assert_eq!(cfg.broadcast.hosts.len(), 2);
        assert_eq!(host.name, "Studio");
        assert_eq!(
            host.transport,
            Transport::Ssh {
                destination: "studio-box".to_owned()
            }
        );
        assert_eq!(host.instance_name, "remote-mixxx");
    }

    #[test]
    fn load_config_parses_broadcast_encoder_group() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cfg_path = temp.path().join("config.toml");
        fs::write(
            &cfg_path,
            r#"
music_dir = "/tmp/music"
db_path = "/tmp/v4vmm.sqlite"

[broadcast.encoder]
binary_path = "/usr/bin/butt"
address = "127.0.0.1"
port = 1256
default_server_name = "main"
"#,
        )
        .expect("write config");

        let cfg = load_config(&cfg_path).expect("load config");
        let encoder = cfg.broadcast.encoder.expect("encoder config");
        let target = encoder.target().expect("encoder target");

        assert_eq!(encoder.binary_path, PathBuf::from("/usr/bin/butt"));
        assert_eq!(encoder.address.as_deref(), Some("127.0.0.1"));
        assert_eq!(encoder.port, Some(1256));
        assert_eq!(encoder.default_server_name, "main");
        assert_eq!(target.binary_path(), "/usr/bin/butt");
        assert!(target.is_addressed());
    }

    #[test]
    fn load_config_parses_broadcast_drop_file_producer() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cfg_path = temp.path().join("config.toml");
        fs::write(
            &cfg_path,
            r#"
music_dir = "/tmp/music"
db_path = "/tmp/v4vmm.sqlite"

[broadcast]
drop_directory = "/tmp/musicindex-live-publisher/mpv/nowplaying"
drop_file_target = "stream-a"
"#,
        )
        .expect("write config");

        let cfg = load_config(&cfg_path).expect("load config");
        let producer = cfg
            .broadcast
            .drop_file_producer()
            .expect("producer config")
            .expect("active producer");

        assert_eq!(
            cfg.broadcast.drop_directory,
            Some(PathBuf::from(
                "/tmp/musicindex-live-publisher/mpv/nowplaying"
            ))
        );
        assert_eq!(producer.target(), "stream-a");
        assert_eq!(
            producer.path(),
            Path::new("/tmp/musicindex-live-publisher/mpv/nowplaying/stream-a.nowplaying.json")
        );
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
    fn load_config_defaults_missing_workspace_layout_to_none() {
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

        assert_eq!(
            cfg.workspace_layout, None,
            "missing workspace layout should keep old configs valid"
        );
    }

    #[test]
    fn load_config_defaults_missing_workspace_prefs_to_none() {
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

        assert_eq!(
            cfg.workspace, None,
            "missing workspace prefs should keep old configs valid"
        );
    }

    #[test]
    fn load_config_parses_workspace_layout() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cfg_path = temp.path().join("config.toml");
        fs::write(
            &cfg_path,
            r#"
music_dir = "/tmp/music"
db_path = "/tmp/v4vmm.sqlite"

[workspace_layout]
focused_frame_id = 8

[[workspace_layout.frames]]
id = 1
kind = "source_list"

[[workspace_layout.frames]]
id = 8
kind = "detail"
"#,
        )
        .expect("write config");

        let cfg = load_config(&cfg_path).expect("load config");
        let layout = WorkspaceLayout::from_config(cfg.workspace_layout.as_ref());

        assert_eq!(
            layout.focused_frame().map(WorkspaceFrameState::kind),
            Some(WorkspaceFrameKind::Detail),
            "workspace layout should load and focus the persisted frame"
        );
    }

    #[test]
    fn load_config_accepts_workspace_layout_with_queue_from_before_show_mount() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cfg_path = temp.path().join("config.toml");
        fs::write(
            &cfg_path,
            r#"
music_dir = "/tmp/music"
db_path = "/tmp/v4vmm.sqlite"

[workspace_layout]
focused_frame_id = 4

[[workspace_layout.frames]]
id = 2
kind = "content_list"

[[workspace_layout.frames]]
id = 4
kind = "queue_now_playing"
"#,
        )
        .expect("write config");

        let cfg = load_config(&cfg_path).expect("load config");
        let layout = WorkspaceLayout::from_config(cfg.workspace_layout.as_ref());
        let kinds: Vec<_> = layout
            .frames()
            .iter()
            .map(WorkspaceFrameState::kind)
            .collect();

        assert_eq!(
            kinds,
            [
                WorkspaceFrameKind::ContentList,
                WorkspaceFrameKind::QueueNowPlaying
            ],
            "configs written before ADR 0060 task 002 should still deserialize"
        );
        assert_eq!(
            layout.focused_frame().map(WorkspaceFrameState::kind),
            Some(WorkspaceFrameKind::QueueNowPlaying),
            "old queue-focused layouts should preserve focus in the stored model"
        );
    }

    #[test]
    fn load_config_parses_workspace_layout_prefs() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cfg_path = temp.path().join("config.toml");
        fs::write(
            &cfg_path,
            r#"
music_dir = "/tmp/music"
db_path = "/tmp/v4vmm.sqlite"

[workspace]
unknown = "keep"

[workspace.layout]
content_pane_width = 1400
other = true
"#,
        )
        .expect("write config");

        let cfg = load_config(&cfg_path).expect("load config");
        let prefs = cfg
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.layout.as_ref());

        assert_eq!(
            prefs.and_then(|prefs| prefs.content_pane_width),
            Some(1400.0),
            "workspace prefs should load the persisted pane width"
        );
    }

    #[test]
    fn load_config_parses_float_workspace_layout_prefs() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cfg_path = temp.path().join("config.toml");
        fs::write(
            &cfg_path,
            r#"
music_dir = "/tmp/music"
db_path = "/tmp/v4vmm.sqlite"

[workspace.layout]
content_pane_width = 1024.5
"#,
        )
        .expect("write config");

        let cfg = load_config(&cfg_path).expect("load config");
        let prefs = cfg
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.layout.as_ref());

        assert_eq!(
            prefs.and_then(|prefs| prefs.content_pane_width),
            Some(1024.5),
            "workspace prefs should accept float pane widths"
        );
    }

    #[test]
    fn load_config_ignores_malformed_workspace_layout() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cfg_path = temp.path().join("config.toml");
        fs::write(
            &cfg_path,
            r#"
music_dir = "/tmp/music"
db_path = "/tmp/v4vmm.sqlite"
workspace_layout = "not a layout"
"#,
        )
        .expect("write config");

        let cfg = load_config(&cfg_path).expect("load config");

        assert_eq!(
            cfg.workspace_layout, None,
            "malformed workspace layout should not make config loading fail"
        );
    }

    #[test]
    fn load_config_ignores_workspace_layout_with_unknown_frame_kind() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cfg_path = temp.path().join("config.toml");
        fs::write(
            &cfg_path,
            r#"
music_dir = "/tmp/music"
db_path = "/tmp/v4vmm.sqlite"

[workspace_layout]
focused_frame_id = 2

[[workspace_layout.frames]]
id = 1
kind = "source_list"

[[workspace_layout.frames]]
id = 2
kind = "future_frame"
"#,
        )
        .expect("write config");

        let cfg = load_config(&cfg_path).expect("load config");

        assert_eq!(
            cfg.workspace_layout, None,
            "unknown workspace frame kinds should fall back instead of failing config load"
        );
    }

    #[test]
    fn load_config_ignores_workspace_layout_with_removed_broadcast_frame() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cfg_path = temp.path().join("config.toml");
        fs::write(
            &cfg_path,
            r#"
music_dir = "/tmp/music"
db_path = "/tmp/v4vmm.sqlite"

[workspace_layout]
focused_frame_id = 5

[[workspace_layout.frames]]
id = 2
kind = "content_list"

[[workspace_layout.frames]]
id = 5
kind = "broadcast"
"#,
        )
        .expect("write config");

        let cfg = load_config(&cfg_path).expect("load config");

        assert_eq!(
            cfg.workspace_layout, None,
            "configs written with the removed ADR 0059 Broadcast frame should fall back"
        );
    }

    #[test]
    fn load_config_ignores_malformed_workspace_layout_prefs() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cfg_path = temp.path().join("config.toml");
        fs::write(
            &cfg_path,
            r#"
music_dir = "/tmp/music"
db_path = "/tmp/v4vmm.sqlite"

[workspace]
[workspace.layout]
content_pane_width = "wide"
"#,
        )
        .expect("write config");

        let cfg = load_config(&cfg_path).expect("load config");

        assert_eq!(
            cfg.workspace
                .as_ref()
                .and_then(|workspace| workspace.layout.as_ref()),
            None,
            "malformed workspace prefs should not make config loading fail"
        );
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
    fn load_config_parses_system_theme_profile() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cfg_path = temp.path().join("config.toml");
        fs::write(
            &cfg_path,
            r#"
music_dir = "/tmp/music"
db_path = "/tmp/v4vmm.sqlite"
theme_profile = "system"
"#,
        )
        .expect("write config");

        let cfg = load_config(&cfg_path).expect("load config");

        assert_eq!(cfg.theme_profile, ThemeProfile::System);
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

    #[test]
    fn save_workspace_layout_persists_without_dropping_existing_values() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cfg_path = temp.path().join("config.toml");
        fs::write(
            &cfg_path,
            r#"
music_dir = "/tmp/old"
db_path = "/tmp/v4vmm.sqlite"
extra = "keep"
"#,
        )
        .expect("write config");
        let mut layout = WorkspaceLayout::default_layout();
        let focused_id = layout
            .add_frame(WorkspaceFrameKind::Detail)
            .expect("add detail frame");

        save_workspace_layout(&cfg_path, &layout.to_config()).expect("save workspace layout");

        let raw = fs::read_to_string(&cfg_path).expect("read config");
        let table = raw.parse::<toml::Table>().expect("parse TOML");
        let cfg = load_config(&cfg_path).expect("load config");
        let restored = WorkspaceLayout::from_config(cfg.workspace_layout.as_ref());

        assert_eq!(
            table.get("extra").and_then(toml::Value::as_str),
            Some("keep"),
            "workspace layout save should preserve unrelated settings"
        );
        assert_eq!(
            restored.to_config(),
            layout.to_config(),
            "workspace layout should round-trip through config.toml"
        );
        assert_eq!(
            restored.focused_frame_id(),
            Some(focused_id),
            "workspace layout save should preserve focused frame id"
        );
    }

    #[test]
    fn save_workspace_layout_prefs_persists_without_dropping_existing_values() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cfg_path = temp.path().join("config.toml");
        fs::write(
            &cfg_path,
            r#"
music_dir = "/tmp/old"
db_path = "/tmp/v4vmm.sqlite"
extra = "keep"

[workspace]
workspace_extra = "keep"

[workspace.layout]
content_pane_width = 640.0
layout_extra = "keep"

[workspace_layout]
focused_frame_id = 8

[[workspace_layout.frames]]
id = 1
kind = "source_list"

[[workspace_layout.frames]]
id = 8
kind = "detail"
"#,
        )
        .expect("write config");

        save_workspace_layout_prefs(
            &cfg_path,
            &WorkspaceLayoutPrefs {
                content_pane_width: Some(1400.0),
                content_list_view_mode: Some(ContentViewMode::List),
            },
        )
        .expect("save workspace layout prefs");

        let raw = fs::read_to_string(&cfg_path).expect("read config");
        let table = raw.parse::<toml::Table>().expect("parse TOML");
        let cfg = load_config(&cfg_path).expect("load config");
        let prefs = cfg
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.layout.as_ref());

        assert_eq!(
            table.get("extra").and_then(toml::Value::as_str),
            Some("keep"),
            "workspace prefs save should preserve unrelated root settings"
        );
        assert!(
            table
                .get("workspace_layout")
                .and_then(toml::Value::as_table)
                .is_some(),
            "workspace prefs save should preserve existing workspace layout config"
        );
        assert!(
            cfg.workspace_layout.is_some(),
            "workspace prefs save should preserve the existing workspace layout data"
        );
        assert_eq!(
            prefs.and_then(|prefs| prefs.content_pane_width),
            Some(1400.0),
            "workspace prefs save should update the persisted pane width"
        );
        assert_eq!(
            prefs.and_then(|prefs| prefs.content_list_view_mode),
            Some(ContentViewMode::List),
            "workspace prefs save should update the persisted content-list view mode"
        );
        assert_eq!(
            table
                .get("workspace")
                .and_then(toml::Value::as_table)
                .and_then(|workspace| workspace.get("layout"))
                .and_then(toml::Value::as_table)
                .and_then(|layout| layout.get("content_list_view_mode"))
                .and_then(toml::Value::as_str),
            Some("list"),
            "workspace prefs save should serialize the content-list view mode"
        );
        assert_eq!(
            table
                .get("workspace")
                .and_then(toml::Value::as_table)
                .and_then(|workspace| workspace.get("workspace_extra"))
                .and_then(toml::Value::as_str),
            Some("keep"),
            "workspace prefs save should preserve unrelated workspace keys"
        );
        assert_eq!(
            table
                .get("workspace")
                .and_then(toml::Value::as_table)
                .and_then(|workspace| workspace.get("layout"))
                .and_then(toml::Value::as_table)
                .and_then(|layout| layout.get("layout_extra"))
                .and_then(toml::Value::as_str),
            Some("keep"),
            "workspace prefs save should preserve unrelated workspace.layout keys"
        );
    }

    #[test]
    fn save_workspace_layout_prefs_recovers_malformed_workspace_tables() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cfg_path = temp.path().join("config.toml");
        fs::write(
            &cfg_path,
            r#"
music_dir = "/tmp/old"
db_path = "/tmp/v4vmm.sqlite"
workspace = "not a table"
"#,
        )
        .expect("write config");

        save_workspace_layout_prefs(
            &cfg_path,
            &WorkspaceLayoutPrefs {
                content_pane_width: Some(900.0),
                content_list_view_mode: None,
            },
        )
        .expect("save workspace layout prefs");

        let cfg = load_config(&cfg_path).expect("load config");
        let prefs = cfg
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.layout.as_ref());

        assert_eq!(
            prefs.and_then(|prefs| prefs.content_pane_width),
            Some(900.0),
            "workspace prefs save should replace malformed workspace tables"
        );
    }

    #[test]
    fn load_config_defaults_missing_content_list_view_mode() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cfg_path = temp.path().join("config.toml");
        fs::write(
            &cfg_path,
            r#"
music_dir = "/tmp/old"
db_path = "/tmp/v4vmm.sqlite"

[workspace.layout]
content_pane_width = 720.0
"#,
        )
        .expect("write config");

        let cfg = load_config(&cfg_path).expect("load config");
        let prefs = cfg
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.layout.as_ref())
            .expect("workspace layout prefs");

        assert_eq!(prefs.content_pane_width, Some(720.0));
        assert_eq!(
            prefs.content_list_view_mode, None,
            "old configs without content_list_view_mode should keep loading"
        );
    }

    #[test]
    fn load_config_parses_content_list_view_mode() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cfg_path = temp.path().join("config.toml");
        fs::write(
            &cfg_path,
            r#"
music_dir = "/tmp/old"
db_path = "/tmp/v4vmm.sqlite"

[workspace.layout]
content_list_view_mode = "list"
"#,
        )
        .expect("write config");

        let cfg = load_config(&cfg_path).expect("load config");
        let prefs = cfg
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.layout.as_ref())
            .expect("workspace layout prefs");

        assert_eq!(
            prefs.content_list_view_mode,
            Some(ContentViewMode::List),
            "workspace prefs should deserialize the content-list view mode"
        );
    }
}
