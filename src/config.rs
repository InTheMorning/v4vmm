//! Configuration decoding and persistence (ADRs 0010, 0046, 0051 and 0066).

use std::collections::BTreeSet;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{anyhow, Context, Result};
use directories::{BaseDirs, ProjectDirs};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::api::DEFAULT_BASE_URL;
use crate::broadcast::encoder::EncoderTarget;
use crate::broadcast::producer::DropFileProducer;
use crate::broadcast::transport::Transport;
use crate::theme_profile::ThemeProfile;
use crate::view_models::workspace::{ContentViewMode, WorkspaceLayoutConfig};

#[derive(Debug)]
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
    pub flac_path: Option<PathBuf>,

    /// Playback backend configuration. Missing config defaults to no playback
    /// driver so existing configs keep loading unchanged.
    pub playback: PlaybackConfig,

    /// Broadcast host configuration. Missing config defaults to one local host.
    pub broadcast: BroadcastConfig,

    /// Global UI scale factor. Mirrors iOS Dynamic Type's named steps.
    /// Missing value defaults to `medium` (1.0×).
    pub ui_scale: UiScale,

    /// Runtime theme profile. Missing value defaults to the existing dark
    /// profile so older config files keep their appearance.
    pub theme_profile: ThemeProfile,

    /// Additive ADR 0046 workspace layout persistence.
    ///
    /// Missing or malformed values fall back to the default workspace layout in
    /// the workspace VM, so older or manually edited configs keep loading.
    pub(crate) workspace_layout: Option<WorkspaceLayoutConfig>,

    /// Additive ADR 0051 workspace layout preferences.
    ///
    /// Missing or malformed values fall back to the default pane width in the
    /// app bootstrap, so older or manually edited configs keep loading.
    pub(crate) workspace: Option<WorkspaceConfig>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WorkspaceConfig {
    /// Forward-compatible workspace layout preferences.
    pub(crate) layout: Option<WorkspaceLayoutPrefs>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WorkspaceLayoutPrefs {
    /// Persisted content-pane width in logical pixels.
    pub(crate) content_pane_width: Option<f32>,
    /// Persisted Music content-list presentation mode.
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackConfig {
    pub driver: PlaybackDriver,
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

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
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
    /// Encoder server passed to the connect command.
    ///
    /// Absent means bare `-s`, which connects to the server the encoder already
    /// has selected. Set it only to name a server explicitly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_server_name: Option<String>,
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
        if self
            .default_server_name
            .as_ref()
            .is_some_and(|name| name.trim().is_empty())
        {
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

/// A safe, typed field error. Rejected values and serde excerpts are never stored.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConfigFieldIssue {
    pub field: &'static str,
    pub kind: ConfigIssueKind,
    pub explanation: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigIssueKind {
    Missing,
    InvalidValue,
    NotATable,
}

impl fmt::Display for ConfigFieldIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "App cannot use {}: {}",
            self.field, self.explanation
        )
    }
}

impl std::error::Error for ConfigFieldIssue {}

pub type ConfigField<T> = std::result::Result<T, ConfigFieldIssue>;

/// One document observation, including independent validation results.
///
/// Invalid core fields remain in the snapshot so an endpoint-only reader can
/// still require just its endpoint. The normal app must require both core paths.
/// The private source bytes/document are intentionally excluded from Debug.
pub struct ConfigSnapshot {
    path: PathBuf,
    original_bytes: Vec<u8>,
    document: toml::Table,
    pub music_dir: ConfigField<PathBuf>,
    pub db_path: ConfigField<PathBuf>,
    pub musicindex_endpoint: ConfigField<String>,
    pub flac_path: ConfigField<Option<PathBuf>>,
    pub playback_driver: ConfigField<PlaybackDriver>,
    pub mpv_path: ConfigField<Option<PathBuf>>,
    pub broadcast_hosts: ConfigField<Vec<BroadcastHostConfig>>,
    pub selected_host: ConfigField<Option<String>>,
    pub drop_directory: ConfigField<Option<PathBuf>>,
    pub drop_file_target: ConfigField<String>,
    pub encoder: ConfigField<Option<BroadcastEncoderConfig>>,
    pub ui_scale: ConfigField<UiScale>,
    pub theme_profile: ConfigField<ThemeProfile>,
    pub(crate) workspace_layout: ConfigField<Option<WorkspaceLayoutConfig>>,
    pub(crate) content_pane_width: ConfigField<Option<f32>>,
    pub(crate) content_list_view_mode: ConfigField<Option<ContentViewMode>>,
}

impl fmt::Debug for ConfigSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConfigSnapshot")
            .field("path", &self.path)
            .field("issues", &self.issues())
            .finish_non_exhaustive()
    }
}

type ConfigTable<'a> = ConfigField<Option<&'a toml::Table>>;

fn invalid_field(field: &'static str, explanation: &'static str) -> ConfigFieldIssue {
    ConfigFieldIssue {
        field,
        kind: ConfigIssueKind::InvalidValue,
        explanation,
    }
}

fn child_table<'a>(parent: ConfigTable<'a>, key: &str, field: &'static str) -> ConfigTable<'a> {
    match parent?.and_then(|table| table.get(key)) {
        None => Ok(None),
        Some(toml::Value::Table(table)) => Ok(Some(table)),
        Some(_) => Err(ConfigFieldIssue {
            field,
            kind: ConfigIssueKind::NotATable,
            explanation: "expected a TOML table",
        }),
    }
}

fn decode_field<T: DeserializeOwned>(
    table: ConfigTable<'_>,
    key: &'static str,
    field: &'static str,
    explanation: &'static str,
) -> ConfigField<Option<T>> {
    table?
        .and_then(|table| table.get(key))
        .map(|value| {
            value
                .clone()
                .try_into::<T>()
                .map_err(|_| invalid_field(field, explanation))
        })
        .transpose()
}

fn required_path(table: &toml::Table, key: &'static str) -> ConfigField<PathBuf> {
    let path = decode_field::<PathBuf>(
        Ok(Some(table)),
        key,
        key,
        "expected a non-empty path string",
    )?
    .ok_or(ConfigFieldIssue {
        field: key,
        kind: ConfigIssueKind::Missing,
        explanation: "required path is missing",
    })?;
    if path.as_os_str().is_empty() {
        return Err(invalid_field(key, "required path is empty"));
    }
    Ok(path)
}

fn optional_path(
    table: ConfigTable<'_>,
    key: &'static str,
    field: &'static str,
) -> ConfigField<Option<PathBuf>> {
    let path = decode_field::<PathBuf>(table, key, field, "expected a non-empty path string")?;
    if path
        .as_ref()
        .is_some_and(|path| path.as_os_str().is_empty())
    {
        return Err(invalid_field(field, "configured path is empty"));
    }
    Ok(path)
}

fn snapshot_hosts(table: ConfigTable<'_>) -> ConfigField<Vec<BroadcastHostConfig>> {
    let hosts = decode_field::<Vec<BroadcastHostConfig>>(
        table,
        "hosts",
        "broadcast.hosts",
        "expected a list of valid broadcast hosts",
    )?
    .unwrap_or_else(default_broadcast_hosts);
    let mut names = BTreeSet::new();
    if hosts.is_empty()
        || hosts
            .iter()
            .any(|host| host.validate().is_err() || !names.insert(host.name.trim().to_owned()))
    {
        return Err(invalid_field(
            "broadcast.hosts",
            "hosts must be valid, non-empty and have distinct names",
        ));
    }
    Ok(hosts)
}

fn snapshot_encoder(table: ConfigTable<'_>) -> ConfigField<Option<BroadcastEncoderConfig>> {
    let encoder = decode_field::<BroadcastEncoderConfig>(
        table,
        "encoder",
        "broadcast.encoder",
        "expected valid encoder settings",
    )?;
    if encoder
        .as_ref()
        .is_some_and(|encoder| encoder.validate().is_err())
    {
        return Err(invalid_field(
            "broadcast.encoder",
            "encoder settings are invalid",
        ));
    }
    Ok(encoder)
}

fn snapshot_width(table: ConfigTable<'_>) -> ConfigField<Option<f32>> {
    // Keep ADR 0051's numeric decoding. The existing view-model/UI bounds still
    // own clamping; this loader does not introduce another width policy.
    let value = table?.and_then(|table| table.get("content_pane_width"));
    match value {
        None => Ok(None),
        Some(toml::Value::Float(width)) => Ok(Some(*width as f32)),
        Some(toml::Value::Integer(width)) => Ok(Some(*width as f32)),
        Some(_) => Err(invalid_field(
            "workspace.layout.content_pane_width",
            "expected an integer or float",
        )),
    }
}

impl ConfigSnapshot {
    /// Read an existing configuration, without first-run creation.
    ///
    /// # Errors
    /// Returns an error for an unreadable file or invalid UTF-8/TOML. Individual
    /// field failures are retained in the snapshot instead.
    pub fn read_existing(path: &Path) -> Result<Self> {
        Self::read_with(path, |path| fs::read(path))
    }

    fn read_with(path: &Path, read: impl FnOnce(&Path) -> io::Result<Vec<u8>>) -> Result<Self> {
        let bytes = read(path)
            .with_context(|| format!("App could not read configuration {}", path.display()))?;
        Self::from_bytes(path, bytes)
    }

    /// Parse one immutable observation; field access never rereads its file.
    ///
    /// # Errors
    /// Returns a safe path/location error for invalid UTF-8 or TOML.
    pub fn from_bytes(path: &Path, bytes: Vec<u8>) -> Result<Self> {
        let raw = std::str::from_utf8(&bytes)
            .with_context(|| format!("Configuration {} is not UTF-8", path.display()))?;
        let document = raw.parse::<toml::Table>().map_err(|error| {
            let offset = error.span().map_or(0, |span| span.start.min(raw.len()));
            let prefix = &raw[..raw.floor_char_boundary(offset)];
            let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
            let column = prefix.rsplit('\n').next().unwrap_or_default().chars().count() + 1;
            anyhow!(
                "App could not parse TOML in {} at line {line}, column {column}. Correct the document before saving.",
                path.display()
            )
        })?;
        let root = Ok(Some(&document));
        let playback = child_table(root, "playback", "playback");
        let broadcast = child_table(root, "broadcast", "broadcast");
        let workspace = child_table(root, "workspace", "workspace");
        let layout = child_table(workspace, "layout", "workspace.layout");
        let broadcast_hosts = snapshot_hosts(broadcast);
        let mut selected_host = decode_field::<String>(
            broadcast,
            "selected_host",
            "broadcast.selected_host",
            "expected a host name string",
        );
        // An unreadable host list is not evidence that a selected name is absent.
        if let (Ok(Some(selected)), Ok(hosts)) = (&selected_host, &broadcast_hosts) {
            if !hosts.iter().any(|host| host.name.trim() == selected.trim()) {
                selected_host = Err(invalid_field(
                    "broadcast.selected_host",
                    "selected name is not in broadcast.hosts",
                ));
            }
        }
        let drop_directory = optional_path(broadcast, "drop_directory", "broadcast.drop_directory");
        let mut drop_file_target = decode_field::<String>(
            broadcast,
            "drop_file_target",
            "broadcast.drop_file_target",
            "expected a target name string",
        )
        .map(|target| target.unwrap_or_else(default_drop_file_target));
        if matches!(&drop_directory, Ok(Some(_))) {
            if let Ok(target) = &drop_file_target {
                if DropFileProducer::validate_target_name(target).is_err() {
                    drop_file_target = Err(invalid_field(
                        "broadcast.drop_file_target",
                        "expected a visible target file name",
                    ));
                }
            }
        }
        let musicindex_endpoint = decode_field::<String>(
            root,
            "musicindex_endpoint",
            "musicindex_endpoint",
            "expected an HTTP or HTTPS URL string",
        )
        .and_then(|endpoint| {
            endpoint.map_or_else(
                || Ok(DEFAULT_BASE_URL.to_owned()),
                |endpoint| {
                    normalize_musicindex_endpoint(&endpoint).map_err(|_| {
                        invalid_field("musicindex_endpoint", "expected a valid HTTP or HTTPS URL")
                    })
                },
            )
        });
        Ok(Self {
            music_dir: required_path(&document, "music_dir"),
            db_path: required_path(&document, "db_path"),
            musicindex_endpoint,
            flac_path: optional_path(root, "flac_path", "flac_path"),
            playback_driver: decode_field::<PlaybackDriver>(
                playback,
                "driver",
                "playback.driver",
                "expected \"null\" or \"mpv\"",
            )
            .map(Option::unwrap_or_default),
            mpv_path: optional_path(playback, "mpv_path", "playback.mpv_path"),
            broadcast_hosts,
            selected_host,
            drop_directory,
            drop_file_target,
            encoder: snapshot_encoder(broadcast),
            ui_scale: decode_field::<UiScale>(
                root,
                "ui_scale",
                "ui_scale",
                "expected x-small, small, medium, large or x-large",
            )
            .map(Option::unwrap_or_default),
            theme_profile: decode_field::<ThemeProfile>(
                root,
                "theme_profile",
                "theme_profile",
                "expected a supported theme profile",
            )
            .map(Option::unwrap_or_default),
            workspace_layout: decode_field(
                root,
                "workspace_layout",
                "workspace_layout",
                "expected a valid workspace frame layout",
            ),
            content_pane_width: snapshot_width(layout),
            content_list_view_mode: decode_field(
                layout,
                "content_list_view_mode",
                "workspace.layout.content_list_view_mode",
                "expected list or tiles",
            ),
            path: path.to_path_buf(),
            original_bytes: bytes,
            document,
        })
    }

    #[must_use]
    pub fn original_bytes(&self) -> &[u8] {
        &self.original_bytes
    }

    /// All field issues, including errors hidden by legacy layout fallback.
    #[must_use]
    pub fn issues(&self) -> Vec<ConfigFieldIssue> {
        let mut issues = Vec::new();
        for issue in [
            self.music_dir.as_ref().err(),
            self.db_path.as_ref().err(),
            self.musicindex_endpoint.as_ref().err(),
            self.flac_path.as_ref().err(),
            self.playback_driver.as_ref().err(),
            self.mpv_path.as_ref().err(),
            self.broadcast_hosts.as_ref().err(),
            self.selected_host.as_ref().err(),
            self.drop_directory.as_ref().err(),
            self.drop_file_target.as_ref().err(),
            self.encoder.as_ref().err(),
            self.ui_scale.as_ref().err(),
            self.theme_profile.as_ref().err(),
            self.workspace_layout.as_ref().err(),
            self.content_pane_width.as_ref().err(),
            self.content_list_view_mode.as_ref().err(),
        ]
        .into_iter()
        .flatten()
        {
            if !issues.contains(issue) {
                issues.push(*issue);
            }
        }
        issues
    }

    fn require_saveable(&self) -> Result<()> {
        if let Some(issue) = self.issues().first() {
            return Err(anyhow!(
                "App did not save configuration {}. {issue}. Correct the file before saving settings.",
                self.path.display()
            ));
        }
        Ok(())
    }

    fn legacy_workspace(&self) -> Option<WorkspaceConfig> {
        let workspace = self.document.get("workspace")?.as_table()?;
        let layout = workspace
            .get("layout")
            .and_then(toml::Value::as_table)
            .and_then(|_| {
                Some(WorkspaceLayoutPrefs {
                    content_pane_width: self.content_pane_width.as_ref().ok().copied()?,
                    content_list_view_mode: self.content_list_view_mode.as_ref().ok().copied()?,
                })
            });
        Some(WorkspaceConfig { layout })
    }

    /// Strict adapter for existing Config consumers. Scoped callers use fields.
    ///
    /// # Errors
    /// Rejects invalid Config fields except the established workspace fallback.
    /// The endpoint has its own reader because Config never carried that field.
    pub fn legacy_config(&self) -> Result<Config> {
        for issue in self.issues().iter().filter(|issue| {
            issue.field == "workspace"
                || issue.field.starts_with("workspace.")
                || issue.field == "workspace_layout"
        }) {
            eprintln!("v4vmm::config: ignoring malformed {}: {issue}", issue.field);
        }
        Ok(Config {
            music_dir: self.music_dir.clone()?,
            db_path: self.db_path.clone()?,
            flac_path: self.flac_path.clone()?,
            playback: PlaybackConfig {
                driver: self.playback_driver?,
                mpv_path: self.mpv_path.clone()?,
            },
            broadcast: BroadcastConfig {
                hosts: self.broadcast_hosts.clone()?,
                selected_host: self.selected_host.clone()?,
                drop_directory: self.drop_directory.clone()?,
                drop_file_target: self.drop_file_target.clone()?,
                encoder: self.encoder.clone()?,
            },
            ui_scale: self.ui_scale?,
            theme_profile: self.theme_profile?,
            workspace_layout: self.workspace_layout.clone().unwrap_or_default(),
            workspace: self.legacy_workspace(),
        })
    }
}

/// Resolve the configuration path without creating any directory.
pub fn config_path() -> Result<PathBuf> {
    let proj = ProjectDirs::from("xyz", "HeyCitizen", "v4vmm")
        .ok_or_else(|| anyhow!("could not determine user config directory"))?;
    Ok(proj.config_dir().join("config.toml"))
}

/// Read one snapshot, creating defaults only for a genuinely absent entry.
///
/// # Errors
/// Reports read/parse or safe first-run creation failures without substituting
/// in-memory defaults. Field validation results stay in the snapshot.
pub fn load_config_snapshot(cfg_path: &Path) -> Result<ConfigSnapshot> {
    load_snapshot_with_defaults(cfg_path, default_config_toml)
}

fn load_snapshot_with_defaults(
    cfg_path: &Path,
    defaults: impl FnOnce() -> Result<String>,
) -> Result<ConfigSnapshot> {
    match fs::symlink_metadata(cfg_path) {
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let default = defaults()?;
            // Validate the whole generated document before publishing any bytes.
            ConfigSnapshot::from_bytes(cfg_path, default.as_bytes().to_vec())?
                .require_saveable()?;
            let parent = cfg_path
                .parent()
                .filter(|path| !path.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."));
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "App could not create configuration directory {}",
                    parent.display()
                )
            })?;
            let created = publish_default_config(cfg_path, default.as_bytes())?;
            if created {
                eprintln!(
                    "App created default configuration at {}. Edit it if needed, then re-run.",
                    cfg_path.display()
                );
            }
        }
        Err(error) => {
            return Err(error).with_context(|| {
                format!("App could not inspect configuration {}", cfg_path.display())
            })
        }
    }
    ConfigSnapshot::read_existing(cfg_path)
}

// The counter avoids collisions between callers; create_new protects entries
// left by another process (including a previous process with this PID).
static DEFAULT_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);
// Bound collision handling rather than looping forever in an unusable folder.
const DEFAULT_TEMP_ATTEMPTS: usize = 32;

fn default_config_temporary(cfg_path: &Path) -> Result<(PathBuf, File)> {
    for _ in 0..DEFAULT_TEMP_ATTEMPTS {
        let sequence = DEFAULT_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = cfg_path.with_file_name(format!(
            ".v4vmm-config-{}-{sequence}.tmp",
            std::process::id(),
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "App could not create temporary configuration {}",
                        path.display()
                    )
                })
            }
        }
    }
    Err(anyhow!(
        "App could not reserve a temporary configuration beside {}",
        cfg_path.display()
    ))
}

fn publish_default_config(cfg_path: &Path, bytes: &[u8]) -> Result<bool> {
    publish_default_with(
        cfg_path,
        bytes,
        |file, bytes| {
            file.write_all(bytes)?;
            file.sync_all()
        },
        |temporary, destination| fs::hard_link(temporary, destination),
        |temporary| fs::remove_file(temporary),
    )
}

// Narrow first-run I/O seams allow write/publication/cleanup failures to be
// tested without changing global permissions or exhausting the filesystem.
fn publish_default_with(
    cfg_path: &Path,
    bytes: &[u8],
    write_and_sync: impl FnOnce(&mut File, &[u8]) -> io::Result<()>,
    publish: impl FnOnce(&Path, &Path) -> io::Result<()>,
    cleanup: impl FnOnce(&Path) -> io::Result<()>,
) -> Result<bool> {
    let (temporary, mut file) = default_config_temporary(cfg_path)?;
    let result = (|| {
        write_and_sync(&mut file, bytes).with_context(|| {
            format!(
                "App could not write and sync temporary configuration {}",
                temporary.display()
            )
        })?;
        match publish(&temporary, cfg_path) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(false),
            Err(error) => Err(error).with_context(|| {
                format!(
                    "App could not publish default configuration {}",
                    cfg_path.display()
                )
            }),
        }
    })();
    drop(file);
    if let Err(error) = cleanup(&temporary) {
        let state = match &result {
            Ok(true) => "The complete default configuration was published.".to_owned(),
            Ok(false) => "Another configuration entry already exists.".to_owned(),
            Err(error) => format!("Default creation failed: {error:#}."),
        };
        return Err(anyhow!(
            "{state} App could not remove temporary configuration {}: {error}. The temporary file remains.",
            temporary.display()
        ));
    }
    result
}

/// Compatibility reader; scoped consumers use load_config_snapshot.
///
/// # Errors
/// Rejects invalid operational Config fields and never replaces an existing
/// document. Existing workspace fallback remains available without save access.
pub fn load_config(cfg_path: &Path) -> Result<Config> {
    load_config_snapshot(cfg_path)?
        .legacy_config()
        .with_context(|| format!("App could not load configuration {}", cfg_path.display()))
}

pub fn load_musicindex_endpoint(cfg_path: &Path) -> Result<String> {
    load_config_snapshot(cfg_path)?
        .musicindex_endpoint
        .with_context(|| {
            format!(
                "App could not read MusicIndex settings from {}",
                cfg_path.display()
            )
        })
}

fn read_config_for_save(cfg_path: &Path) -> Result<ConfigSnapshot> {
    let snapshot = ConfigSnapshot::read_existing(cfg_path)?;
    snapshot.require_saveable()?;
    Ok(snapshot)
}

fn write_existing_config(cfg_path: &Path, table: &toml::Table) -> Result<()> {
    let updated = toml::to_string_pretty(table).context("serialize config TOML")?;
    // No create flag: deletion after validation must not turn an ordinary save
    // into first-run creation. Explicit conflict-protected repair is task 006.
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(cfg_path)
        .with_context(|| {
            format!(
                "App could not open existing configuration {} for saving",
                cfg_path.display()
            )
        })?;
    file.write_all(updated.as_bytes())
        .with_context(|| format!("App could not save configuration {}", cfg_path.display()))
}

pub fn save_app_settings(
    cfg_path: &Path,
    endpoint: &str,
    music_dir: &str,
    flac_path: &str,
    ui_scale: UiScale,
    theme_profile: ThemeProfile,
) -> Result<(String, PathBuf, Option<PathBuf>, UiScale, ThemeProfile)> {
    let mut table = read_config_for_save(cfg_path)?.document;
    let endpoint = normalize_musicindex_endpoint(endpoint)?;
    let music_dir = normalize_music_dir(music_dir)?;
    let flac_path = normalize_flac_path(flac_path)?;
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

    write_existing_config(cfg_path, &table)?;
    Ok((endpoint, music_dir, flac_path, ui_scale, theme_profile))
}

pub(crate) fn save_workspace_layout(
    cfg_path: &Path,
    workspace_layout: &WorkspaceLayoutConfig,
) -> Result<()> {
    let mut table = read_config_for_save(cfg_path)?.document;
    let layout_value =
        toml::Value::try_from(workspace_layout).context("serialize workspace layout config")?;
    table.insert("workspace_layout".into(), layout_value);

    write_existing_config(cfg_path, &table)
}

pub(crate) fn save_workspace_layout_prefs(
    cfg_path: &Path,
    workspace_layout_prefs: &WorkspaceLayoutPrefs,
) -> Result<()> {
    let mut table = read_config_for_save(cfg_path)?.document;

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

    write_existing_config(cfg_path, &table)
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
    let url = reqwest::Url::parse(&candidate).context("parse musicindex_endpoint URL")?;
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
music_dir = {}

# SQLite database path (app data)
db_path = {}

# MusicIndex API endpoint
musicindex_endpoint = {}

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
# Optional. Leave it out to connect with bare `-s`, which uses the server the
# encoder already has selected. Set it only to name a server explicitly.
# default_server_name = "my-server"
# address = "127.0.0.1"
# port = 1256

# Workspace layout is persisted automatically. Missing or malformed values
# fall back to the default layout.
"#,
        toml::Value::String(music_dir.display().to_string()),
        toml::Value::String(db_path.display().to_string()),
        toml::Value::String(DEFAULT_BASE_URL.to_owned()),
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
        let cfg = ConfigSnapshot::from_bytes(
            Path::new("config.toml"),
            r#"
music_dir = "/tmp/music"
db_path = "/tmp/v4vmm.sqlite"
"#
            .as_bytes()
            .to_vec(),
        )
        .expect("parse config snapshot")
        .legacy_config()
        .expect("valid config")
        .playback;

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
        assert_eq!(encoder.default_server_name.as_deref(), Some("main"));
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
            message.contains("theme_profile") && message.contains("supported theme profile"),
            "unexpected error: {message}"
        );
    }

    #[test]
    fn load_config_parses_mpv_playback_config() {
        let cfg = ConfigSnapshot::from_bytes(
            Path::new("config.toml"),
            r#"
music_dir = "/tmp/music"
db_path = "/tmp/v4vmm.sqlite"

[playback]
driver = "mpv"
mpv_path = "/usr/bin/mpv"
"#
            .as_bytes()
            .to_vec(),
        )
        .expect("parse config snapshot")
        .legacy_config()
        .expect("valid config")
        .playback;

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
            message.contains("playback.driver"),
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
    fn save_workspace_layout_prefs_rejects_malformed_workspace_tables() {
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

        let before = fs::read(&cfg_path).expect("original bytes");
        save_workspace_layout_prefs(
            &cfg_path,
            &WorkspaceLayoutPrefs {
                content_pane_width: Some(900.0),
                content_list_view_mode: None,
            },
        )
        .expect_err("ordinary saves cannot repair malformed settings");
        assert_eq!(fs::read(&cfg_path).expect("preserved bytes"), before);
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
    // Situational: ADR 0066 invariants 3–4. These are behavioral guards for
    // document preservation and independently decoded configuration facts.
    const SNAPSHOT_CORE: &str = "music_dir = \"/tmp/music\"\ndb_path = \"/tmp/library.sqlite\"\n";

    fn config_snapshot(extra: &str) -> ConfigSnapshot {
        ConfigSnapshot::from_bytes(
            Path::new("/tmp/config.toml"),
            format!("{SNAPSHOT_CORE}{extra}").into_bytes(),
        )
        .expect("parsed snapshot")
    }

    #[test]
    fn adr_0066_snapshot_defaults_and_required_fields_are_distinct() {
        let snapshot = config_snapshot("");
        assert!(snapshot.issues().is_empty());
        assert_eq!(
            snapshot.musicindex_endpoint.as_deref().unwrap(),
            DEFAULT_BASE_URL
        );
        assert_eq!(snapshot.playback_driver, Ok(PlaybackDriver::Null));
        assert_eq!(snapshot.flac_path, Ok(None));
        assert_eq!(snapshot.drop_directory, Ok(None));
        assert_eq!(snapshot.encoder, Ok(None));
        assert_eq!(
            snapshot.broadcast_hosts.as_ref().unwrap(),
            &default_broadcast_hosts()
        );

        for (raw, field, kind) in [
            ("", "music_dir", ConfigIssueKind::Missing),
            ("music_dir = 7", "music_dir", ConfigIssueKind::InvalidValue),
            (
                "music_dir = \"\"",
                "music_dir",
                ConfigIssueKind::InvalidValue,
            ),
            (
                "music_dir = \"/tmp/music\"",
                "db_path",
                ConfigIssueKind::Missing,
            ),
            (
                "music_dir = \"/tmp/music\"\ndb_path = []",
                "db_path",
                ConfigIssueKind::InvalidValue,
            ),
        ] {
            let snapshot =
                ConfigSnapshot::from_bytes(Path::new("config.toml"), raw.as_bytes().to_vec())
                    .unwrap();
            assert!(
                snapshot
                    .issues()
                    .iter()
                    .any(|issue| issue.field == field && issue.kind == kind),
                "{raw}"
            );
            assert!(snapshot.legacy_config().is_err());
        }
    }

    #[test]
    fn adr_0066_optional_fields_fail_without_poisoning_core_paths() {
        for (extra, field) in [
            ("musicindex_endpoint = 3", "musicindex_endpoint"),
            (
                "musicindex_endpoint = 'ftp://example.test'",
                "musicindex_endpoint",
            ),
            ("flac_path = false", "flac_path"),
            ("flac_path = ''", "flac_path"),
            ("ui_scale = 'huge'", "ui_scale"),
            ("theme_profile = false", "theme_profile"),
            ("workspace_layout = false", "workspace_layout"),
            ("[playback]\ndriver = 'other'", "playback.driver"),
            ("[playback]\nmpv_path = 8", "playback.mpv_path"),
            ("[broadcast]\nhosts = []", "broadcast.hosts"),
            (
                "[broadcast]\nselected_host = 'absent'",
                "broadcast.selected_host",
            ),
            (
                "[broadcast]\ndrop_directory = ''",
                "broadcast.drop_directory",
            ),
            (
                "[broadcast]\ndrop_file_target = 5",
                "broadcast.drop_file_target",
            ),
            (
                "[broadcast]\ndrop_directory = '/tmp/producer'\ndrop_file_target = '../hidden'",
                "broadcast.drop_file_target",
            ),
            ("[broadcast.encoder]\nport = 'bad'", "broadcast.encoder"),
            (
                "[workspace.layout]\ncontent_list_view_mode = 'other'",
                "workspace.layout.content_list_view_mode",
            ),
            (
                "[workspace.layout]\ncontent_pane_width = 'wide'",
                "workspace.layout.content_pane_width",
            ),
        ] {
            let snapshot = config_snapshot(extra);
            assert_eq!(
                snapshot.music_dir.as_ref().unwrap(),
                &PathBuf::from("/tmp/music")
            );
            assert_eq!(
                snapshot.db_path.as_ref().unwrap(),
                &PathBuf::from("/tmp/library.sqlite")
            );
            assert!(
                snapshot.issues().iter().any(|issue| issue.field == field),
                "{extra}"
            );
            assert!(snapshot.require_saveable().is_err(), "{extra}");
        }
    }

    #[test]
    fn adr_0066_malformed_optional_tables_keep_other_groups_available() {
        for group in ["playback", "broadcast", "workspace"] {
            let snapshot = config_snapshot(&format!("{group} = false"));
            assert!(snapshot.music_dir.is_ok());
            assert!(snapshot.db_path.is_ok());
            assert!(snapshot.musicindex_endpoint.is_ok());
            assert!(snapshot
                .issues()
                .iter()
                .any(|issue| issue.field == group && issue.kind == ConfigIssueKind::NotATable));
            match group {
                "playback" => {
                    assert!(snapshot.playback_driver.is_err());
                    assert!(snapshot.mpv_path.is_err());
                    assert!(snapshot.broadcast_hosts.is_ok());
                }
                "broadcast" => {
                    assert!(snapshot.broadcast_hosts.is_err());
                    assert!(snapshot.selected_host.is_err());
                    assert!(snapshot.drop_directory.is_err());
                    assert!(snapshot.encoder.is_err());
                    assert!(snapshot.playback_driver.is_ok());
                }
                _ => {
                    assert!(snapshot.content_pane_width.is_err());
                    assert!(snapshot.content_list_view_mode.is_err());
                    assert!(snapshot.playback_driver.is_ok());
                }
            }
        }
    }

    #[test]
    fn adr_0066_readable_tables_preserve_valid_sibling_fields() {
        let snapshot = config_snapshot(
            "[playback]\ndriver = 'broken'\nmpv_path = '/bin/mpv'\n\
             [broadcast]\nhosts = false\nselected_host = 'Local'\n\
             drop_directory = '/tmp/producer'\ndrop_file_target = 'default'\n\
             [broadcast.encoder]\nbinary_path = 'butt'\n\
             [workspace.layout]\ncontent_pane_width = 'wide'\ncontent_list_view_mode = 'list'\n",
        );
        assert!(snapshot.playback_driver.is_err());
        assert_eq!(snapshot.mpv_path.unwrap(), Some(PathBuf::from("/bin/mpv")));
        assert!(snapshot.broadcast_hosts.is_err());
        assert_eq!(snapshot.selected_host.unwrap().as_deref(), Some("Local"));
        assert_eq!(
            snapshot.drop_directory.unwrap(),
            Some(PathBuf::from("/tmp/producer"))
        );
        assert_eq!(snapshot.drop_file_target.unwrap(), "default");
        assert!(snapshot.encoder.unwrap().is_some());
        assert!(snapshot.content_pane_width.is_err());
        assert_eq!(
            snapshot.content_list_view_mode.unwrap(),
            Some(ContentViewMode::List)
        );

        let snapshot = config_snapshot(
            "[broadcast]\ndrop_directory = 3\n[broadcast.encoder]\nbinary_path = 'butt'",
        );
        assert!(snapshot.drop_directory.is_err());
        assert!(snapshot.encoder.unwrap().is_some());
        assert!(snapshot.broadcast_hosts.is_ok());

        // Keep task 017's deliberately unset publisher target representable.
        assert_eq!(
            config_snapshot("[broadcast]\ndrop_file_target = ''").drop_file_target,
            Ok(String::new())
        );
    }

    #[test]
    fn adr_0066_endpoint_reader_requires_only_its_own_field() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        let raw = "music_dir = false\ndb_path = []\nmusicindex_endpoint = 'https://index.test/'\n[playback]\ndriver = 'broken'";
        fs::write(&path, raw).unwrap();
        assert_eq!(
            load_musicindex_endpoint(&path).unwrap(),
            "https://index.test"
        );
        assert!(load_config(&path).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), raw);
        fs::write(&path, format!("{SNAPSHOT_CORE}musicindex_endpoint = 23")).unwrap();
        assert!(load_musicindex_endpoint(&path).is_err());
    }

    #[test]
    fn adr_0066_snapshot_keeps_one_read_even_if_the_file_changes() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        let old = format!("{SNAPSHOT_CORE}musicindex_endpoint = 'https://old.test'");
        fs::write(&path, &old).unwrap();
        let mut reads = 0;
        let snapshot = ConfigSnapshot::read_with(&path, |path| {
            reads += 1;
            let bytes = fs::read(path)?;
            fs::write(
                path,
                "music_dir = false\nmusicindex_endpoint = 'https://new.test'",
            )?;
            Ok(bytes)
        })
        .unwrap();
        assert_eq!(reads, 1);
        assert_eq!(snapshot.original_bytes(), old.as_bytes());
        assert_eq!(
            snapshot.musicindex_endpoint.as_ref().unwrap(),
            "https://old.test"
        );
        assert_eq!(
            snapshot.legacy_config().unwrap().music_dir,
            PathBuf::from("/tmp/music")
        );
        assert!(!ConfigSnapshot::read_existing(&path)
            .unwrap()
            .music_dir
            .is_ok());
    }

    #[test]
    fn adr_0066_config_diagnostics_exclude_rejected_values_and_source_bytes() {
        let secret = "private-credential-0066";
        let snapshot = config_snapshot(&format!(
            "musicindex_endpoint = 'http://user:{secret}@[invalid'\ntheme_profile = '{secret}'\n[playback]\ndriver = '{secret}'"
        ));
        let messages = format!(
            "{snapshot:?}\n{:#}\n{:#}",
            snapshot.require_saveable().unwrap_err(),
            snapshot.legacy_config().unwrap_err()
        );
        assert!(!messages.contains(secret));
        assert!(messages.contains("musicindex_endpoint"));
        let parse_error = ConfigSnapshot::from_bytes(
            Path::new("config.toml"),
            format!("secret = '{secret}'\n[broken").into_bytes(),
        )
        .unwrap_err();
        let text = format!("{parse_error:#?}");
        assert!(text.contains("line 2"));
        assert!(!text.contains(secret));
        let utf8_error =
            ConfigSnapshot::from_bytes(Path::new("config.toml"), vec![0xff]).unwrap_err();
        assert!(format!("{utf8_error:#}").contains("not UTF-8"));
        let url_error =
            normalize_musicindex_endpoint(&format!("http://user:{secret}@[invalid")).unwrap_err();
        assert!(!format!("{url_error:#?}").contains(secret));
    }

    fn ordinary_saves(path: &Path) -> [Result<()>; 3] {
        [
            save_app_settings(
                path,
                DEFAULT_BASE_URL,
                "/tmp/changed",
                "",
                UiScale::Large,
                ThemeProfile::Dark,
            )
            .map(|_| ()),
            save_workspace_layout(path, &WorkspaceLayout::default().to_config()),
            save_workspace_layout_prefs(
                path,
                &WorkspaceLayoutPrefs {
                    content_pane_width: Some(800.0),
                    content_list_view_mode: Some(ContentViewMode::List),
                },
            ),
        ]
    }

    #[test]
    fn adr_0066_all_ordinary_saves_preserve_broken_documents() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        let cases = [
            vec![0xff],
            b"[broken".to_vec(),
            b"music_dir = false\ndb_path = '/tmp/db'".to_vec(),
            b"music_dir = '/tmp/music'\ndb_path = ''".to_vec(),
            b"db_path = '/tmp/db'".to_vec(),
            format!("{SNAPSHOT_CORE}musicindex_endpoint = 4").into_bytes(),
            format!("{SNAPSHOT_CORE}workspace = 'broken'").into_bytes(),
            format!("{SNAPSHOT_CORE}workspace_layout = false").into_bytes(),
            format!("{SNAPSHOT_CORE}[workspace.layout]\ncontent_pane_width = 'wrong'").into_bytes(),
            format!("{SNAPSHOT_CORE}[broadcast.encoder]\nbinary_path = ''").into_bytes(),
            format!("{SNAPSHOT_CORE}[playback]\ndriver = 'wrong'").into_bytes(),
            format!("{SNAPSHOT_CORE}theme_profile = 'wrong'").into_bytes(),
            format!("{SNAPSHOT_CORE}ui_scale = 'wrong'").into_bytes(),
        ];
        for bytes in cases {
            fs::write(&path, &bytes).unwrap();
            for result in ordinary_saves(&path) {
                assert!(result.is_err());
                assert_eq!(fs::read(&path).unwrap(), bytes);
            }
        }
    }

    #[test]
    fn adr_0066_saves_never_recreate_a_missing_document() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("missing/config.toml");
        for result in ordinary_saves(&path) {
            assert!(result.is_err());
            assert!(!path.exists());
            assert!(!path.parent().unwrap().exists());
        }
        let path = temp.path().join("config.toml");
        fs::write(&path, SNAPSHOT_CORE).unwrap();
        let snapshot = read_config_for_save(&path).unwrap();
        fs::remove_file(&path).unwrap();
        assert!(write_existing_config(&path, &snapshot.document).is_err());
        assert!(
            !path.exists(),
            "save must not create even after a successful earlier read"
        );
    }

    #[test]
    fn adr_0066_saves_resume_after_a_fresh_valid_document() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        fs::write(&path, format!("{SNAPSHOT_CORE}ui_scale = 'broken'")).unwrap();
        assert!(ordinary_saves(&path).iter().all(Result::is_err));
        fs::write(
            &path,
            format!("{SNAPSHOT_CORE}custom_future_value = 'keep'"),
        )
        .unwrap();
        for result in ordinary_saves(&path) {
            result.unwrap();
        }
        let snapshot = ConfigSnapshot::read_existing(&path).unwrap();
        assert!(snapshot.issues().is_empty());
        assert_eq!(
            snapshot.document["custom_future_value"].as_str(),
            Some("keep")
        );
    }

    #[test]
    fn adr_0066_first_run_publishes_only_a_valid_complete_document() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("new/config.toml");
        let snapshot = load_snapshot_with_defaults(&path, || Ok(SNAPSHOT_CORE.to_owned())).unwrap();
        assert_eq!(snapshot.original_bytes(), SNAPSHOT_CORE.as_bytes());
        assert_eq!(fs::read(&path).unwrap(), SNAPSHOT_CORE.as_bytes());
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        let bad_path = temp.path().join("bad/config.toml");
        assert!(load_snapshot_with_defaults(&bad_path, || Ok("[broken".to_owned())).is_err());
        assert!(!bad_path.parent().unwrap().exists());
        assert!(
            load_snapshot_with_defaults(&bad_path, || Err(anyhow!("cannot derive defaults")))
                .is_err()
        );
        assert!(!bad_path.exists());

        let default = default_config_toml().unwrap();
        assert!(
            ConfigSnapshot::from_bytes(Path::new("config.toml"), default.into_bytes())
                .unwrap()
                .issues()
                .is_empty()
        );
    }

    #[test]
    fn adr_0066_first_run_reads_the_competing_creators_document() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        let winner = format!("{SNAPSHOT_CORE}musicindex_endpoint = 'https://winner.test'");
        let snapshot = load_snapshot_with_defaults(&path, || {
            fs::write(&path, &winner)?;
            Ok(SNAPSHOT_CORE.to_owned())
        })
        .unwrap();
        assert_eq!(snapshot.original_bytes(), winner.as_bytes());
        assert_eq!(snapshot.musicindex_endpoint.unwrap(), "https://winner.test");
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    }

    #[test]
    fn adr_0066_concurrent_first_run_writers_do_not_clobber() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        let barrier = std::sync::Barrier::new(4);
        let snapshots = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..4).map(|index| {
                let path = &path;
                let barrier = &barrier;
                scope.spawn(move || {
                    load_snapshot_with_defaults(path, || {
                        barrier.wait();
                        Ok(format!("{SNAPSHOT_CORE}musicindex_endpoint = 'https://writer-{index}.test'"))
                    }).unwrap()
                })
            }).collect();
            handles
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .collect::<Vec<_>>()
        });
        let saved = fs::read(&path).unwrap();
        for snapshot in snapshots {
            assert_eq!(snapshot.original_bytes(), saved);
        }
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn adr_0066_symlinks_and_unreadable_entries_never_invoke_defaults() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        let absent_target = temp.path().join("missing.toml");
        symlink(&absent_target, &path).unwrap();
        assert!(load_snapshot_with_defaults(&path, || panic!(
            "must not create defaults for a dangling symlink"
        ))
        .is_err());
        assert!(!absent_target.exists());
        assert!(fs::symlink_metadata(&path)
            .unwrap()
            .file_type()
            .is_symlink());
        assert!(ordinary_saves(&path).iter().all(Result::is_err));
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        assert!(load_snapshot_with_defaults(&path, || panic!(
            "must not create defaults for a directory"
        ))
        .is_err());
        assert!(path.is_dir());
        assert!(ordinary_saves(&path).iter().all(Result::is_err));
    }

    #[test]
    fn adr_0066_existing_bad_bytes_and_read_denial_do_not_create_defaults() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        for bytes in [b"[broken".as_slice(), &[0xff]] {
            fs::write(&path, bytes).unwrap();
            assert!(load_snapshot_with_defaults(&path, || panic!(
                "existing bytes must be preserved"
            ))
            .is_err());
            assert_eq!(fs::read(&path).unwrap(), bytes);
        }
        fs::write(&path, SNAPSHOT_CORE).unwrap();
        let error = ConfigSnapshot::read_with(&path, |_| {
            Err(io::Error::from(io::ErrorKind::PermissionDenied))
        })
        .unwrap_err();
        assert!(format!("{error:#}").contains(&path.display().to_string()));
        assert_eq!(fs::read_to_string(&path).unwrap(), SNAPSHOT_CORE);
    }

    #[cfg(unix)]
    #[test]
    fn adr_0066_permission_denial_preserves_config_across_load_and_saves() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        fs::write(&path, SNAPSHOT_CORE).unwrap();
        let permissions = fs::metadata(&path).unwrap().permissions();
        fs::set_permissions(&path, fs::Permissions::from_mode(0)).unwrap();
        // Privileged test runners can bypass permissions; the injected reader
        // test above still covers PermissionDenied on those hosts.
        let denied = fs::read(&path).is_err();
        let loaded = load_snapshot_with_defaults(&path, || panic!("existing entry"));
        let saves = ordinary_saves(&path);
        fs::set_permissions(&path, permissions).unwrap();
        if denied {
            assert!(loaded.is_err());
            assert!(saves.iter().all(Result::is_err));
            assert_eq!(fs::read_to_string(&path).unwrap(), SNAPSHOT_CORE);
        }
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    }

    #[test]
    fn adr_0066_failed_default_writes_and_publication_clean_owned_temporaries() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        let error = publish_default_with(
            &path,
            SNAPSHOT_CORE.as_bytes(),
            |file, _| {
                file.write_all(b"partial")?;
                Err(io::Error::from(io::ErrorKind::WriteZero))
            },
            |_, _| panic!("incomplete bytes must not be published"),
            |temporary| fs::remove_file(temporary),
        )
        .unwrap_err();
        assert!(format!("{error:#}").contains("temporary configuration"));
        assert!(!path.exists());
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);

        let error = publish_default_with(
            &path,
            SNAPSHOT_CORE.as_bytes(),
            |file, bytes| {
                file.write_all(bytes)?;
                file.sync_all()
            },
            |_, _| Err(io::Error::from(io::ErrorKind::PermissionDenied)),
            |temporary| fs::remove_file(temporary),
        )
        .unwrap_err();
        assert!(format!("{error:#}").contains(&path.display().to_string()));
        assert!(!path.exists());
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
    }

    #[test]
    fn adr_0066_failed_default_cleanup_reports_the_remaining_path() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.toml");
        let error = publish_default_with(
            &path,
            SNAPSHOT_CORE.as_bytes(),
            |file, bytes| {
                file.write_all(bytes)?;
                file.sync_all()
            },
            |temporary, destination| fs::hard_link(temporary, destination),
            |_| Err(io::Error::from(io::ErrorKind::PermissionDenied)),
        )
        .unwrap_err();
        let remaining = fs::read_dir(temp.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|candidate| candidate != &path)
            .unwrap();
        let report = format!("{error:#}");
        assert!(report.contains("complete default configuration was published"));
        assert!(report.contains(&remaining.display().to_string()));
        assert_eq!(fs::read(&remaining).unwrap(), SNAPSHOT_CORE.as_bytes());
        assert_eq!(fs::read(&path).unwrap(), SNAPSHOT_CORE.as_bytes());
    }
}
