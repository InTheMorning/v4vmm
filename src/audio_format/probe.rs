//! Fresh bounded converter observations for maintenance and conversion (ADR 0066).

#![warn(clippy::pedantic)]

use std::io::{self, Read};
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub(crate) const CONVERTER_PROBE_TIMEOUT: Duration = Duration::from_secs(5);
pub(crate) const CONVERTER_PROBE_OUTPUT_CAP: usize = 16 * 1024;
const PROBE_POLL_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExecutableSource {
    Configured,
    Path,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProbeOutcome {
    Exited(Option<i32>),
    Missing,
    PermissionDenied,
    TimedOut,
    OutputLimit,
    IoFailure(io::ErrorKind),
}

/// Reports retain no child output: an arbitrary executable can print credentials.
#[derive(Clone, Debug)]
pub(crate) struct ConverterProbe {
    pub(crate) executable: PathBuf,
    pub(crate) source: ExecutableSource,
    pub(crate) recorded_at: chrono::DateTime<chrono::Utc>,
    pub(crate) outcome: ProbeOutcome,
}

impl ConverterProbe {
    pub(crate) fn flac(configured: Option<&Path>) -> Self {
        Self::run(
            configured.unwrap_or_else(|| Path::new("flac")),
            if configured.is_some() {
                ExecutableSource::Configured
            } else {
                ExecutableSource::Path
            },
            "--version",
            None,
        )
    }

    pub(crate) fn ffmpeg() -> Self {
        Self::run(
            Path::new("ffmpeg"),
            ExecutableSource::Path,
            "-version",
            None,
        )
    }

    pub(crate) fn available(&self) -> bool {
        self.outcome == ProbeOutcome::Exited(Some(0))
    }

    fn run(
        executable: &Path,
        source: ExecutableSource,
        argument: &str,
        search_path: Option<&Path>,
    ) -> Self {
        let mut command = Command::new(executable);
        command.arg(argument);
        if let Some(search_path) = search_path {
            command.env("PATH", search_path);
        }
        let outcome = bounded_probe(&mut command).unwrap_or_else(|error| match error.kind() {
            io::ErrorKind::NotFound => ProbeOutcome::Missing,
            io::ErrorKind::PermissionDenied => ProbeOutcome::PermissionDenied,
            kind => ProbeOutcome::IoFailure(kind),
        });
        Self {
            executable: executable.into(),
            source,
            recorded_at: chrono::Utc::now(),
            outcome,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ConverterObservation {
    pub(crate) flac: ConverterProbe,
    pub(crate) ffmpeg: ConverterProbe,
}

impl ConverterObservation {
    /// Every request replaces prior observations, including negative PATH results.
    pub(crate) fn refresh(configured: Option<&Path>) -> Self {
        Self {
            flac: ConverterProbe::flac(configured),
            ffmpeg: ConverterProbe::ffmpeg(),
        }
    }
}

fn bounded_probe(command: &mut Command) -> io::Result<ProbeOutcome> {
    // A nonblocking socket avoids reader threads that can outlive a child when
    // descendants retain stdout. Both streams share one finite byte budget.
    let (mut reader, writer) = UnixStream::pair()?;
    reader.set_nonblocking(true)?;
    let stderr = writer.try_clone()?;
    command
        .stdin(Stdio::null())
        .stdout(Stdio::from(OwnedFd::from(writer)))
        .stderr(Stdio::from(OwnedFd::from(stderr)));
    let mut child = command.spawn()?;
    let deadline = Instant::now() + CONVERTER_PROBE_TIMEOUT;
    let result = observe_child(&mut child, &mut reader, deadline);
    // try_wait reaps exited children. All other outcomes terminate and reap the
    // directly owned child before returning, including read and wait errors.
    if !matches!(result, Ok(ProbeOutcome::Exited(_))) {
        let _ = child.kill();
        child.wait()?;
    }
    result
}

fn observe_child(
    child: &mut Child,
    reader: &mut UnixStream,
    deadline: Instant,
) -> io::Result<ProbeOutcome> {
    let mut consumed = 0;
    let mut buffer = [0u8; 1024];
    let mut exited = None;
    loop {
        loop {
            if Instant::now() >= deadline {
                return Ok(ProbeOutcome::TimedOut);
            }
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => {
                    consumed += count;
                    if consumed >= CONVERTER_PROBE_OUTPUT_CAP {
                        return Ok(ProbeOutcome::OutputLimit);
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
        if let Some(status) = exited {
            return Ok(ProbeOutcome::Exited(status));
        }
        if let Some(status) = child.try_wait()? {
            exited = Some(status.code());
            continue;
        }
        std::thread::sleep(PROBE_POLL_INTERVAL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn executable(path: &Path, script: &str) {
        std::fs::write(path, format!("#!/bin/sh\n{script}\n")).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
    }

    #[test]
    fn adr_0066_converter_path_checks_refresh_in_one_process() {
        let temp = tempfile::tempdir().unwrap();
        for (name, arg) in [("flac", "--version"), ("ffmpeg", "-version")] {
            let check = || {
                ConverterProbe::run(
                    Path::new(name),
                    ExecutableSource::Path,
                    arg,
                    Some(temp.path()),
                )
            };
            assert_eq!(check().outcome, ProbeOutcome::Missing);
            let path = temp.path().join(name);
            executable(&path, "exit 0");
            let success = check();
            assert!(success.available());
            std::fs::remove_file(path).unwrap();
            let missing = check();
            assert_eq!(missing.outcome, ProbeOutcome::Missing);
            assert!(missing.recorded_at >= success.recorded_at);
        }
        let first = temp.path().join("first");
        executable(&first, "exit 0");
        assert!(ConverterProbe::flac(Some(&first)).available());
        let second = temp.path().join("second");
        let changed = ConverterProbe::flac(Some(&second));
        assert_eq!(changed.executable, second);
        assert_eq!(changed.source, ExecutableSource::Configured);
        assert_eq!(changed.outcome, ProbeOutcome::Missing);
    }

    #[test]
    fn adr_0066_converter_failures_are_distinct_and_output_is_not_retained() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("flac");
        executable(&path, "printf 'token=never-report-this' >&2\nexit 7");
        let failed = ConverterProbe::flac(Some(&path));
        assert_eq!(failed.outcome, ProbeOutcome::Exited(Some(7)));
        assert!(!format!("{failed:?}").contains("never-report-this"));
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(
            ConverterProbe::flac(Some(&path)).outcome,
            ProbeOutcome::PermissionDenied
        );
        executable(
            &path,
            "while :; do printf '0123456789012345678901234567890123456789'; done",
        );
        let start = Instant::now();
        assert_eq!(
            ConverterProbe::flac(Some(&path)).outcome,
            ProbeOutcome::OutputLimit
        );
        assert!(start.elapsed() < CONVERTER_PROBE_TIMEOUT);
    }

    #[test]
    fn adr_0066_converter_preserves_flac_first_and_existing_ffmpeg_fallback() {
        let temp = tempfile::tempdir().unwrap();
        let flac = temp.path().join("flac");
        let ffmpeg = temp.path().join("ffmpeg");
        let wav = temp.path().join("track.wav");
        let output = wav.with_extension("flac");
        // Different output bytes identify which encoder actually executed.
        executable(&flac, "if [ \"$1\" = --version ]; then exit 0; fi\nwhile [ \"$1\" != -o ]; do shift; done\nshift\nprintf 'fLaC-reference' > \"$1\"");
        executable(&ffmpeg, "if [ \"$1\" = -version ]; then exit 0; fi\nfor last do :; done\nprintf 'fLaC-fallback' > \"$last\"");
        let check = |path: &Path| ConverterObservation {
            flac: ConverterProbe::flac(Some(path)),
            ffmpeg: ConverterProbe::run(&ffmpeg, ExecutableSource::Path, "-version", None),
        };
        std::fs::write(&wav, b"RIFFfixtureWAVE").unwrap();
        super::super::transcode_observed(&wav, &check(&flac)).unwrap();
        assert_eq!(std::fs::read(&output).unwrap(), b"fLaC-reference");
        assert!(!wav.exists());
        // A valid configured FLAC rejecting the input still allows ffmpeg.
        executable(&flac, "if [ \"$1\" = --version ]; then exit 0; fi\nexit 9");
        std::fs::write(&wav, b"RIFFfixtureWAVE").unwrap();
        super::super::transcode_observed(&wav, &check(&flac)).unwrap();
        assert_eq!(std::fs::read(&output).unwrap(), b"fLaC-fallback");
        // An explicit missing path stays missing, while ffmpeg remains usable.
        std::fs::write(&wav, b"RIFFfixtureWAVE").unwrap();
        let missing = check(&temp.path().join("explicit-missing"));
        assert_eq!(missing.flac.outcome, ProbeOutcome::Missing);
        super::super::transcode_observed(&wav, &missing).unwrap();
        assert_eq!(std::fs::read(&output).unwrap(), b"fLaC-fallback");
        executable(&ffmpeg, "exit 8");
        std::fs::write(&wav, b"RIFFfixtureWAVE").unwrap();
        assert!(super::super::transcode_observed(
            &wav,
            &check(&temp.path().join("explicit-missing"))
        )
        .is_err());
        assert_eq!(std::fs::read(wav).unwrap(), b"RIFFfixtureWAVE");
    }

    #[test]
    fn adr_0066_converter_timeout_reaps_child() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("flac");
        let pid = temp.path().join("pid");
        executable(
            &path,
            &format!("echo $$ > '{}'\nexec /bin/sleep 30", pid.display()),
        );
        let start = Instant::now();
        assert_eq!(
            ConverterProbe::flac(Some(&path)).outcome,
            ProbeOutcome::TimedOut
        );
        assert!(start.elapsed() < CONVERTER_PROBE_TIMEOUT + Duration::from_secs(2));
        #[cfg(target_os = "linux")]
        assert!(!Path::new("/proc")
            .join(std::fs::read_to_string(pid).unwrap().trim())
            .exists());
    }
}
