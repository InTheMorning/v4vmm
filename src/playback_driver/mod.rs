//! Player adapter boundary (ADR 0021).
//!
//! The session in `playback.rs` owns canonical now-playing state. A
//! `PlaybackDriver` is a thin executor that loads files, seeks, pauses, and
//! reports observed playback facts back to the session.

use std::path::Path;

use anyhow::Result;

use crate::config::{PlaybackConfig, PlaybackDriver as PlaybackDriverConfig};

#[cfg(unix)]
pub mod mpv;
pub mod null;

#[cfg(unix)]
pub use mpv::MpvDriver;
pub use null::NullDriver;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DriverStatus {
    pub position_ms: u64,
    pub paused: bool,
    pub eof: bool,
    pub error: Option<String>,
}

pub trait PlaybackDriver: Send + Sync {
    fn load(&self, path: &Path, start_ms: u64) -> Result<()>;
    fn seek(&self, position_ms: u64) -> Result<()>;
    fn pause(&self, paused: bool) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn poll(&self) -> Result<DriverStatus>;
}

#[derive(Debug)]
pub enum ConfiguredPlaybackDriver {
    Null(NullDriver),
    #[cfg(unix)]
    Mpv(MpvDriver),
}

impl ConfiguredPlaybackDriver {
    pub fn from_config(config: &PlaybackConfig) -> Result<Self> {
        match config.driver {
            PlaybackDriverConfig::Null => Ok(Self::Null(NullDriver::new())),
            PlaybackDriverConfig::Mpv => {
                #[cfg(unix)]
                {
                    let mpv_path = config.mpv_path.clone().unwrap_or_else(|| "mpv".into());
                    MpvDriver::new(mpv_path).map(Self::Mpv)
                }
                #[cfg(not(unix))]
                {
                    Err(anyhow::anyhow!(
                        "mpv playback driver requires Unix-domain sockets"
                    ))
                }
            }
        }
    }

    pub fn ping(&self) -> Result<()> {
        match self {
            Self::Null(_) => Ok(()),
            #[cfg(unix)]
            Self::Mpv(driver) => driver.ping(),
        }
    }

    pub fn is_live_driver(&self) -> bool {
        match self {
            Self::Null(_) => false,
            #[cfg(unix)]
            Self::Mpv(_) => true,
        }
    }
}

impl PlaybackDriver for ConfiguredPlaybackDriver {
    fn load(&self, path: &Path, start_ms: u64) -> Result<()> {
        match self {
            Self::Null(driver) => driver.load(path, start_ms),
            #[cfg(unix)]
            Self::Mpv(driver) => driver.load(path, start_ms),
        }
    }

    fn seek(&self, position_ms: u64) -> Result<()> {
        match self {
            Self::Null(driver) => driver.seek(position_ms),
            #[cfg(unix)]
            Self::Mpv(driver) => driver.seek(position_ms),
        }
    }

    fn pause(&self, paused: bool) -> Result<()> {
        match self {
            Self::Null(driver) => driver.pause(paused),
            #[cfg(unix)]
            Self::Mpv(driver) => driver.pause(paused),
        }
    }

    fn stop(&self) -> Result<()> {
        match self {
            Self::Null(driver) => driver.stop(),
            #[cfg(unix)]
            Self::Mpv(driver) => driver.stop(),
        }
    }

    fn poll(&self) -> Result<DriverStatus> {
        match self {
            Self::Null(driver) => driver.poll(),
            #[cfg(unix)]
            Self::Mpv(driver) => driver.poll(),
        }
    }
}
