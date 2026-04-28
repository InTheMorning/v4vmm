//! mpv JSON IPC playback driver.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};

use super::{DriverStatus, PlaybackDriver};

const SOCKET_READY_TIMEOUT: Duration = Duration::from_secs(3);
const IPC_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug)]
pub struct MpvDriver {
    mpv_path: PathBuf,
    runtime_dir: PathBuf,
    inner: Mutex<MpvState>,
}

#[derive(Debug)]
struct MpvState {
    child: Option<Child>,
    socket_path: Option<PathBuf>,
    stream: Option<BufReader<UnixStream>>,
    next_request_id: u64,
    status: DriverStatus,
}

impl MpvDriver {
    pub fn new(mpv_path: impl Into<PathBuf>) -> Result<Self> {
        Self::with_runtime_dir(mpv_path, default_runtime_dir()?)
    }

    pub fn with_runtime_dir(mpv_path: impl Into<PathBuf>, runtime_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&runtime_dir)
            .with_context(|| format!("create mpv runtime dir {}", runtime_dir.display()))?;
        fs::set_permissions(&runtime_dir, fs::Permissions::from_mode(0o700))
            .with_context(|| format!("secure mpv runtime dir {}", runtime_dir.display()))?;
        Ok(Self {
            mpv_path: mpv_path.into(),
            runtime_dir,
            inner: Mutex::new(MpvState {
                child: None,
                socket_path: None,
                stream: None,
                next_request_id: 1,
                status: DriverStatus::default(),
            }),
        })
    }

    pub fn ping(&self) -> Result<()> {
        let mut state = self.inner.lock().expect("mpv driver mutex");
        self.ensure_started(&mut state)?;
        self.command(&mut state, json!(["get_property", "mpv-version"]))
            .map(|_| ())
    }

    fn ensure_started(&self, state: &mut MpvState) -> Result<()> {
        if state.stream.is_some() {
            return Ok(());
        }

        let socket_path = self.socket_path();
        remove_stale_socket(&socket_path)?;
        let mut child = Command::new(&self.mpv_path)
            .arg("--idle")
            .arg("--no-video")
            .arg(format!("--input-ipc-server={}", socket_path.display()))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("spawn mpv at {}", self.mpv_path.display()))?;

        let stream = match wait_for_socket(&socket_path) {
            Ok(stream) => stream,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        };
        stream
            .set_read_timeout(Some(IPC_TIMEOUT))
            .context("set mpv IPC read timeout")?;
        stream
            .set_write_timeout(Some(IPC_TIMEOUT))
            .context("set mpv IPC write timeout")?;

        state.child = Some(child);
        state.socket_path = Some(socket_path);
        state.stream = Some(BufReader::new(stream));
        self.observe_property(state, 1, "time-pos")?;
        self.observe_property(state, 2, "pause")?;
        state.status = DriverStatus::default();
        Ok(())
    }

    fn observe_property(&self, state: &mut MpvState, observer_id: u64, name: &str) -> Result<()> {
        self.command(state, json!(["observe_property", observer_id, name]))
            .map(|_| ())
    }

    fn command(&self, state: &mut MpvState, command: Value) -> Result<Option<Value>> {
        let request_id = state.next_request_id;
        state.next_request_id = state
            .next_request_id
            .checked_add(1)
            .context("mpv request id overflow")?;
        let request = json!({
            "command": command,
            "request_id": request_id,
        });
        let line = serde_json::to_vec(&request).context("serialize mpv command")?;
        let stream = state
            .stream
            .as_mut()
            .context("mpv IPC stream is not open")?;
        stream
            .get_mut()
            .write_all(&line)
            .context("write mpv command")?;
        stream
            .get_mut()
            .write_all(b"\n")
            .context("write mpv command newline")?;
        stream.get_mut().flush().context("flush mpv command")?;

        loop {
            let message = read_message(stream)?;
            handle_event(&mut state.status, &message);
            if message
                .get("request_id")
                .and_then(Value::as_u64)
                .is_some_and(|id| id == request_id)
            {
                let error = message
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("missing error field");
                if error != "success" {
                    return Err(anyhow!("mpv command failed: {error}"));
                }
                return Ok(message.get("data").cloned());
            }
        }
    }

    fn socket_path(&self) -> PathBuf {
        self.runtime_dir
            .join(format!("mpv-{}.sock", std::process::id()))
    }

    pub fn shutdown(&self) {
        let mut state = self.inner.lock().expect("mpv driver mutex");
        let _ = send_quit(&mut state);
        if let Some(child) = state.child.as_mut() {
            let deadline = Instant::now() + Duration::from_millis(500);
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) if Instant::now() < deadline => {
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    _ => {
                        let _ = child.kill();
                        let _ = child.wait();
                        break;
                    }
                }
            }
        }
        if let Some(path) = &state.socket_path {
            let _ = fs::remove_file(path);
        }
        state.child = None;
        state.socket_path = None;
        state.stream = None;
    }
}

impl PlaybackDriver for MpvDriver {
    fn load(&self, path: &Path, start_ms: u64) -> Result<()> {
        let mut state = self.inner.lock().expect("mpv driver mutex");
        self.ensure_started(&mut state)?;
        state.status.eof = false;
        self.command(&mut state, json!(["loadfile", path.display().to_string()]))?;
        if start_ms > 0 {
            let seconds = start_ms as f64 / 1000.0;
            self.command(&mut state, json!(["seek", seconds, "absolute"]))?;
        }
        state.status.position_ms = start_ms;
        state.status.paused = false;
        Ok(())
    }

    fn seek(&self, position_ms: u64) -> Result<()> {
        let mut state = self.inner.lock().expect("mpv driver mutex");
        self.ensure_started(&mut state)?;
        let seconds = position_ms as f64 / 1000.0;
        self.command(&mut state, json!(["seek", seconds, "absolute"]))?;
        state.status.position_ms = position_ms;
        state.status.eof = false;
        Ok(())
    }

    fn pause(&self, paused: bool) -> Result<()> {
        let mut state = self.inner.lock().expect("mpv driver mutex");
        self.ensure_started(&mut state)?;
        self.command(&mut state, json!(["set_property", "pause", paused]))?;
        state.status.paused = paused;
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        let mut state = self.inner.lock().expect("mpv driver mutex");
        if state.stream.is_none() {
            return Ok(());
        }
        self.command(&mut state, json!(["stop"]))?;
        state.status = DriverStatus::default();
        Ok(())
    }

    fn poll(&self) -> Result<DriverStatus> {
        let mut state = self.inner.lock().expect("mpv driver mutex");
        if state.stream.is_none() {
            return Ok(state.status.clone());
        }
        if let Some(value) = self.command(&mut state, json!(["get_property", "time-pos"]))? {
            if let Some(seconds) = value.as_f64() {
                state.status.position_ms = seconds_to_ms(seconds);
            }
        }
        if let Some(value) = self.command(&mut state, json!(["get_property", "pause"]))? {
            if let Some(paused) = value.as_bool() {
                state.status.paused = paused;
            }
        }
        Ok(state.status.clone())
    }
}

impl Drop for MpvDriver {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn read_message(stream: &mut BufReader<UnixStream>) -> Result<Value> {
    let mut line = String::new();
    let bytes = stream
        .read_line(&mut line)
        .context("read mpv IPC message")?;
    anyhow::ensure!(bytes > 0, "mpv IPC socket closed");
    serde_json::from_str(line.trim_end()).context("parse mpv IPC message")
}

fn send_quit(state: &mut MpvState) -> Result<()> {
    let request_id = state.next_request_id;
    state.next_request_id = state
        .next_request_id
        .checked_add(1)
        .context("mpv request id overflow")?;
    let request = json!({
        "command": ["quit"],
        "request_id": request_id,
    });
    let line = serde_json::to_vec(&request).context("serialize mpv quit command")?;
    let Some(stream) = state.stream.as_mut() else {
        return Ok(());
    };
    stream
        .get_mut()
        .write_all(&line)
        .context("write mpv quit command")?;
    stream
        .get_mut()
        .write_all(b"\n")
        .context("write mpv quit command newline")?;
    stream.get_mut().flush().context("flush mpv quit command")
}

fn handle_event(status: &mut DriverStatus, message: &Value) {
    match message.get("event").and_then(Value::as_str) {
        Some("property-change") => handle_property_change(status, message),
        Some("end-file") => {
            status.eof = true;
        }
        _ => {}
    }
}

fn handle_property_change(status: &mut DriverStatus, message: &Value) {
    match message.get("name").and_then(Value::as_str) {
        Some("time-pos") => {
            if let Some(seconds) = message.get("data").and_then(Value::as_f64) {
                status.position_ms = seconds_to_ms(seconds);
            }
        }
        Some("pause") => {
            if let Some(paused) = message.get("data").and_then(Value::as_bool) {
                status.paused = paused;
            }
        }
        _ => {}
    }
}

fn seconds_to_ms(seconds: f64) -> u64 {
    if seconds.is_sign_negative() {
        return 0;
    }
    (seconds * 1000.0).round() as u64
}

fn wait_for_socket(socket_path: &Path) -> Result<UnixStream> {
    let deadline = Instant::now() + SOCKET_READY_TIMEOUT;
    loop {
        match UnixStream::connect(socket_path) {
            Ok(stream) => return Ok(stream),
            Err(error) if Instant::now() < deadline => {
                if error.kind() != std::io::ErrorKind::NotFound
                    && error.kind() != std::io::ErrorKind::ConnectionRefused
                {
                    return Err(error).context("connect mpv IPC socket");
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(error) => return Err(error).context("connect mpv IPC socket before timeout"),
        }
    }
}

fn remove_stale_socket(socket_path: &Path) -> Result<()> {
    if !socket_path.exists() {
        return Ok(());
    }
    match UnixStream::connect(socket_path) {
        Ok(_) => Err(anyhow!(
            "mpv IPC socket already active at {}",
            socket_path.display()
        )),
        Err(_) => fs::remove_file(socket_path)
            .with_context(|| format!("remove stale mpv socket {}", socket_path.display())),
    }
}

fn default_runtime_dir() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("v4vmm"));
    Ok(base.join("mpv"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn property_events_update_cached_status() {
        let mut status = DriverStatus::default();

        handle_event(
            &mut status,
            &json!({
                "event": "property-change",
                "name": "time-pos",
                "data": 12.345
            }),
        );
        handle_event(
            &mut status,
            &json!({
                "event": "property-change",
                "name": "pause",
                "data": true
            }),
        );

        assert_eq!(status.position_ms, 12_345);
        assert!(status.paused);
    }

    #[test]
    fn end_file_event_sets_eof() {
        let mut status = DriverStatus::default();

        handle_event(&mut status, &json!({"event": "end-file"}));

        assert!(status.eof);
    }

    #[test]
    fn negative_seconds_clamp_to_zero_ms() {
        assert_eq!(seconds_to_ms(-1.0), 0);
    }
}
