//! Null driver: ADR 0020 simulated transport. Default when no audio backend
//! is configured. Records the last requested state in memory so tests and
//! relay smoke runs can assert driver calls without launching a process.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::Result;

use super::{DriverStatus, PlaybackDriver};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NullDriverState {
    pub loaded_path: Option<PathBuf>,
    pub position_ms: u64,
    pub paused: bool,
    pub stopped: bool,
}

#[derive(Debug, Default)]
pub struct NullDriver {
    state: Mutex<NullDriverState>,
}

impl NullDriver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> NullDriverState {
        self.state.lock().expect("null driver mutex").clone()
    }
}

impl PlaybackDriver for NullDriver {
    fn load(&self, path: &Path, start_ms: u64) -> Result<()> {
        let mut state = self.state.lock().expect("null driver mutex");
        state.loaded_path = Some(path.to_path_buf());
        state.position_ms = start_ms;
        state.paused = false;
        state.stopped = false;
        Ok(())
    }

    fn seek(&self, position_ms: u64) -> Result<()> {
        self.state.lock().expect("null driver mutex").position_ms = position_ms;
        Ok(())
    }

    fn pause(&self, paused: bool) -> Result<()> {
        self.state.lock().expect("null driver mutex").paused = paused;
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        let mut state = self.state.lock().expect("null driver mutex");
        state.stopped = true;
        state.paused = false;
        Ok(())
    }

    fn poll(&self) -> Result<DriverStatus> {
        let state = self.state.lock().expect("null driver mutex");
        Ok(DriverStatus {
            position_ms: state.position_ms,
            paused: state.paused,
            eof: false,
            error: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn load_records_path_and_start_position() {
        let driver = NullDriver::new();
        driver.load(&PathBuf::from("/tmp/song.mp3"), 1_500).unwrap();
        let snap = driver.snapshot();
        assert_eq!(snap.loaded_path, Some(PathBuf::from("/tmp/song.mp3")));
        assert_eq!(snap.position_ms, 1_500);
        assert!(!snap.paused);
        assert!(!snap.stopped);
    }

    #[test]
    fn seek_pause_stop_update_state() {
        let driver = NullDriver::new();
        driver.load(&PathBuf::from("/tmp/x.mp3"), 0).unwrap();
        driver.seek(42_000).unwrap();
        driver.pause(true).unwrap();
        let status = driver.poll().unwrap();
        assert_eq!(status.position_ms, 42_000);
        assert!(status.paused);
        driver.stop().unwrap();
        assert!(driver.snapshot().stopped);
    }
}
