//! Player adapter boundary (ADR 0021).
//!
//! The session in `playback.rs` owns canonical now-playing state. A
//! `PlaybackDriver` is a thin executor that loads files, seeks, pauses, and
//! reports observed playback facts back to the session.

use std::path::Path;

use anyhow::Result;

pub mod null;

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
