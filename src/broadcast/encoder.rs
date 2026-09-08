#![warn(clippy::pedantic)]

//! Stream encoder control for ADR 0059.
//!
//! This module owns the blocking `butt` command-line control boundary. It
//! parses the status text into typed stream state and never sends song metadata
//! to the encoder.

use std::io::ErrorKind;

use anyhow::{anyhow, Result};

use crate::broadcast::control::{CommandOutput, CommandRunner, ProcessCommandRunner};

const DEFAULT_ENCODER_BINARY: &str = "butt";
const STATUS_OPTION: &str = "-S";
const CONNECT_OPTION: &str = "-s";
const DISCONNECT_OPTION: &str = "-d";
const START_RECORDING_OPTION: &str = "-r";
const STOP_RECORDING_OPTION: &str = "-t";
const ADDRESS_OPTION: &str = "-a";
const PORT_OPTION: &str = "-p";
const INSTANCE_NOT_REACHABLE_MARKERS: &[&str] = &[
    "connection refused",
    "connection timed out",
    "could not connect",
    "failed to connect",
    "no route to host",
    "network is unreachable",
    "operation timed out",
    "timed out",
];

/// Address of one controllable stream encoder.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncoderTarget {
    binary_path: String,
    address: Option<String>,
    port: Option<u16>,
}

impl EncoderTarget {
    /// Create a local encoder target.
    ///
    /// # Errors
    ///
    /// Returns an error when the binary path is empty.
    pub fn local(binary_path: impl Into<String>) -> Result<Self> {
        Self::new(binary_path, None, None)
    }

    /// Create an encoder target with optional network control address.
    ///
    /// # Errors
    ///
    /// Returns an error when the binary path or address is empty.
    pub fn new(
        binary_path: impl Into<String>,
        address: Option<String>,
        port: Option<u16>,
    ) -> Result<Self> {
        let binary_path = normalize_required("encoder binary path", &binary_path.into())?;
        let address = address
            .map(|address| normalize_required("encoder address", &address))
            .transpose()?;
        Ok(Self {
            binary_path,
            address,
            port,
        })
    }

    /// Return the default encoder binary name.
    #[must_use]
    pub const fn default_binary() -> &'static str {
        DEFAULT_ENCODER_BINARY
    }

    /// Return whether this target addresses a remote control instance.
    #[must_use]
    pub const fn is_addressed(&self) -> bool {
        self.address.is_some() || self.port.is_some()
    }

    /// Return the configured binary path.
    #[must_use]
    pub fn binary_path(&self) -> &str {
        &self.binary_path
    }

    fn args_with_control_options(&self, operation: &str) -> Vec<String> {
        let mut args = Vec::with_capacity(5);
        if let Some(address) = &self.address {
            args.push(ADDRESS_OPTION.to_owned());
            args.push(address.clone());
        }
        if let Some(port) = self.port {
            args.push(PORT_OPTION.to_owned());
            args.push(port.to_string());
        }
        args.push(operation.to_owned());
        args
    }

    fn connect_args(&self, server_name: &str) -> Result<Vec<String>> {
        let server_name = normalize_required("encoder server name", server_name)?;
        let mut args = self.args_with_control_options(CONNECT_OPTION);
        args.push(server_name);
        Ok(args)
    }
}

/// Current stream connection state reported by the encoder.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncoderState {
    /// The encoder is connected to its configured streaming server.
    Connected,
    /// The encoder is in the middle of connecting to its streaming server.
    Connecting,
    /// The encoder is running but disconnected.
    Disconnected,
    /// The encoder binary is absent or no encoder target is configured.
    NotInstalled,
    /// The addressed encoder control instance did not answer.
    NotReachable,
    /// The status output did not parse into a known connection state.
    Unknown,
}

/// Current recording state reported by the encoder.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecordingState {
    /// The encoder is recording.
    Recording {
        /// Recording elapsed time in seconds.
        seconds: Option<u64>,
        /// Path reported by the encoder for the recording file.
        path: Option<String>,
    },
    /// The encoder is not recording.
    Stopped {
        /// Last reported recording elapsed time in seconds.
        seconds: Option<u64>,
        /// Path reported by the encoder for the recording file.
        path: Option<String>,
    },
    /// The recording keys were missing or malformed.
    Unknown,
}

/// Current audio-signal state reported by the encoder.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioSignalState {
    /// Audio is present at the encoder input.
    Present,
    /// No audio is present at the encoder input.
    Absent,
    /// The signal keys were missing or malformed.
    Unknown,
}

/// Listener count reported by the encoder.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ListenerCount {
    /// The encoder did not report a useful listener count.
    Unknown,
    /// A positive listener count was reported.
    Known(u64),
}

/// Parsed encoder status snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncoderStatus {
    /// Stream connection state.
    pub state: EncoderState,
    /// Recording state and recording file path.
    pub recording: RecordingState,
    /// Audio-signal state.
    pub signal: AudioSignalState,
    /// Listener count, when useful.
    pub listeners: ListenerCount,
    /// Song title currently shown by the encoder.
    pub song: Option<String>,
    /// Stream elapsed time in seconds.
    pub stream_seconds: Option<u64>,
}

impl EncoderStatus {
    /// Build the not-installed empty state.
    #[must_use]
    pub const fn not_installed() -> Self {
        Self {
            state: EncoderState::NotInstalled,
            recording: RecordingState::Unknown,
            signal: AudioSignalState::Unknown,
            listeners: ListenerCount::Unknown,
            song: None,
            stream_seconds: None,
        }
    }

    /// Build the not-reachable state.
    #[must_use]
    pub const fn not_reachable() -> Self {
        Self {
            state: EncoderState::NotReachable,
            recording: RecordingState::Unknown,
            signal: AudioSignalState::Unknown,
            listeners: ListenerCount::Unknown,
            song: None,
            stream_seconds: None,
        }
    }

    /// Build the unknown state.
    #[must_use]
    pub const fn unknown() -> Self {
        Self {
            state: EncoderState::Unknown,
            recording: RecordingState::Unknown,
            signal: AudioSignalState::Unknown,
            listeners: ListenerCount::Unknown,
            song: None,
            stream_seconds: None,
        }
    }
}

/// Blocking encoder control boundary.
#[derive(Clone, Debug)]
pub struct EncoderControl<R> {
    runner: R,
}

impl Default for EncoderControl<ProcessCommandRunner> {
    fn default() -> Self {
        Self {
            runner: ProcessCommandRunner,
        }
    }
}

impl<R: CommandRunner> EncoderControl<R> {
    /// Build encoder control with an explicit command runner.
    #[must_use]
    pub const fn new(runner: R) -> Self {
        Self { runner }
    }

    /// Read the encoder status.
    ///
    /// # Errors
    ///
    /// Returns an error when the command cannot run for a reason other than a
    /// missing binary.
    pub fn status(&self, target: &EncoderTarget) -> Result<EncoderStatus> {
        let args = target.args_with_control_options(STATUS_OPTION);
        match self.runner.run(target.binary_path(), &args) {
            Ok(output) => Ok(status_from_output(target, &output)),
            Err(error) if command_not_found(&error) => Ok(EncoderStatus::not_installed()),
            Err(error) => Err(error),
        }
    }

    /// Connect to a configured encoder server.
    ///
    /// # Errors
    ///
    /// Returns an error when `butt` rejects the command or cannot be run.
    pub fn connect(&self, target: &EncoderTarget, server_name: &str) -> Result<()> {
        let args = target.connect_args(server_name)?;
        self.run_checked(target, &args)
    }

    /// Disconnect from the active encoder server.
    ///
    /// # Errors
    ///
    /// Returns an error when `butt` rejects the command or cannot be run.
    pub fn disconnect(&self, target: &EncoderTarget) -> Result<()> {
        let args = target.args_with_control_options(DISCONNECT_OPTION);
        self.run_checked(target, &args)
    }

    /// Start encoder-side recording.
    ///
    /// # Errors
    ///
    /// Returns an error when `butt` rejects the command or cannot be run.
    pub fn start_recording(&self, target: &EncoderTarget) -> Result<()> {
        let args = target.args_with_control_options(START_RECORDING_OPTION);
        self.run_checked(target, &args)
    }

    /// Stop encoder-side recording.
    ///
    /// # Errors
    ///
    /// Returns an error when `butt` rejects the command or cannot be run.
    pub fn stop_recording(&self, target: &EncoderTarget) -> Result<()> {
        let args = target.args_with_control_options(STOP_RECORDING_OPTION);
        self.run_checked(target, &args)
    }

    fn run_checked(&self, target: &EncoderTarget, args: &[String]) -> Result<()> {
        let output = self.runner.run(target.binary_path(), args)?;
        if output.status.success() {
            Ok(())
        } else {
            Err(command_failure(target, &output))
        }
    }
}

/// Read encoder status with the real command runner.
///
/// # Errors
///
/// Returns an error when the command cannot run for a reason other than a
/// missing binary.
pub fn status(target: &EncoderTarget) -> Result<EncoderStatus> {
    EncoderControl::default().status(target)
}

/// Connect to a configured encoder server with the real command runner.
///
/// # Errors
///
/// Returns an error when `butt` rejects the command or cannot be run.
pub fn connect(target: &EncoderTarget, server_name: &str) -> Result<()> {
    EncoderControl::default().connect(target, server_name)
}

/// Disconnect from the active encoder server with the real command runner.
///
/// # Errors
///
/// Returns an error when `butt` rejects the command or cannot be run.
pub fn disconnect(target: &EncoderTarget) -> Result<()> {
    EncoderControl::default().disconnect(target)
}

/// Start encoder-side recording with the real command runner.
///
/// # Errors
///
/// Returns an error when `butt` rejects the command or cannot be run.
pub fn start_recording(target: &EncoderTarget) -> Result<()> {
    EncoderControl::default().start_recording(target)
}

/// Stop encoder-side recording with the real command runner.
///
/// # Errors
///
/// Returns an error when `butt` rejects the command or cannot be run.
pub fn stop_recording(target: &EncoderTarget) -> Result<()> {
    EncoderControl::default().stop_recording(target)
}

fn status_from_output(target: &EncoderTarget, output: &CommandOutput) -> EncoderStatus {
    if !output.status.success() {
        return if target.is_addressed() && instance_not_reachable(output) {
            EncoderStatus::not_reachable()
        } else {
            EncoderStatus::unknown()
        };
    }

    parse_status(&output.stdout).unwrap_or_else(EncoderStatus::unknown)
}

fn parse_status(output: &str) -> Option<EncoderStatus> {
    let mut fields = StatusFields::default();
    for line in output.lines() {
        let (key, value) = line.split_once(':')?;
        fields.accept(key.trim(), value.trim())?;
    }

    Some(EncoderStatus {
        state: fields.connection_state()?,
        recording: fields.recording_state()?,
        signal: fields.signal_state()?,
        listeners: fields.listener_count(),
        song: fields.song,
        stream_seconds: fields.stream_seconds,
    })
}

#[derive(Default)]
struct StatusFields {
    connected: Option<bool>,
    connecting: Option<bool>,
    recording: Option<bool>,
    signal_present: Option<bool>,
    signal_absent: Option<bool>,
    stream_seconds: Option<u64>,
    record_seconds: Option<u64>,
    record_path: Option<String>,
    listeners: Option<u64>,
    song: Option<String>,
}

impl StatusFields {
    fn accept(&mut self, key: &str, value: &str) -> Option<()> {
        match key {
            "connected" => self.connected = Some(parse_flag(value)?),
            "connecting" => self.connecting = Some(parse_flag(value)?),
            "recording" => self.recording = Some(parse_flag(value)?),
            "signal present" => self.signal_present = Some(parse_flag(value)?),
            "signal absent" => self.signal_absent = Some(parse_flag(value)?),
            "stream seconds" => self.stream_seconds = Some(parse_u64(value)?),
            "record seconds" => self.record_seconds = Some(parse_u64(value)?),
            "record path" => self.record_path = nonempty(value),
            "listeners" => self.listeners = Some(parse_u64(value)?),
            "song" => self.song = nonempty(value),
            _ => {}
        }
        Some(())
    }

    fn connection_state(&self) -> Option<EncoderState> {
        match (self.connected?, self.connecting?) {
            (true, _) => Some(EncoderState::Connected),
            (false, true) => Some(EncoderState::Connecting),
            (false, false) => Some(EncoderState::Disconnected),
        }
    }

    fn recording_state(&self) -> Option<RecordingState> {
        if self.recording? {
            Some(RecordingState::Recording {
                seconds: self.record_seconds,
                path: self.record_path.clone(),
            })
        } else {
            Some(RecordingState::Stopped {
                seconds: self.record_seconds,
                path: self.record_path.clone(),
            })
        }
    }

    fn signal_state(&self) -> Option<AudioSignalState> {
        match (self.signal_present?, self.signal_absent?) {
            (true, false) => Some(AudioSignalState::Present),
            (false, true) => Some(AudioSignalState::Absent),
            (false, false) | (true, true) => Some(AudioSignalState::Unknown),
        }
    }

    fn listener_count(&self) -> ListenerCount {
        match self.listeners {
            Some(count) if count > 0 => ListenerCount::Known(count),
            Some(_) | None => ListenerCount::Unknown,
        }
    }
}

fn parse_flag(value: &str) -> Option<bool> {
    match value {
        "0" => Some(false),
        "1" => Some(true),
        _ => None,
    }
}

fn parse_u64(value: &str) -> Option<u64> {
    value.parse().ok()
}

fn nonempty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

fn normalize_required(label: &str, value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(anyhow!("{label} cannot be empty"));
    }
    Ok(value.to_owned())
}

fn instance_not_reachable(output: &CommandOutput) -> bool {
    let stderr = output.stderr.to_ascii_lowercase();
    INSTANCE_NOT_REACHABLE_MARKERS
        .iter()
        .any(|marker| stderr.contains(marker))
}

fn command_not_found(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .is_some_and(|io_error| io_error.kind() == ErrorKind::NotFound)
    })
}

fn command_failure(target: &EncoderTarget, output: &CommandOutput) -> anyhow::Error {
    let status = match output.status.code() {
        Some(code) => format!("exit {code}"),
        None => "terminated by signal".to_owned(),
    };
    let detail = output.stderr.trim();
    if detail.is_empty() {
        anyhow!("{} failed with {status}", target.binary_path())
    } else {
        anyhow!("{} failed with {status}: {detail}", target.binary_path())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::io;

    use anyhow::Context;

    use super::*;

    const VERIFIED_STATUS_OUTPUT: &str = "\
connected: 1
connecting: 0
recording: 0
signal present: 1
signal absent: 0
stream seconds: 129883
stream kBytes: 2029310
record seconds: 0
record kBytes: 0
volume left: -4.6
volume right: -6.3
song: Mr. Bungle - Sweet Charity
record path:
listeners: 0
";

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct RecordedCall {
        program: String,
        args: Vec<String>,
    }

    #[derive(Debug)]
    struct StubRunner {
        output: RefCell<Option<Result<CommandOutput>>>,
        calls: RefCell<Vec<RecordedCall>>,
    }

    impl StubRunner {
        fn with_output(output: CommandOutput) -> Self {
            Self {
                output: RefCell::new(Some(Ok(output))),
                calls: RefCell::new(Vec::new()),
            }
        }

        fn with_error(error: anyhow::Error) -> Self {
            Self {
                output: RefCell::new(Some(Err(error))),
                calls: RefCell::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<RecordedCall> {
            self.calls.borrow().clone()
        }
    }

    impl CommandRunner for &StubRunner {
        fn run(&self, program: &str, args: &[String]) -> Result<CommandOutput> {
            self.calls.borrow_mut().push(RecordedCall {
                program: program.to_owned(),
                args: args.to_vec(),
            });
            self.output
                .borrow_mut()
                .take()
                .context("stub command output missing")?
        }
    }

    fn local_target() -> EncoderTarget {
        EncoderTarget::local("butt").expect("local target")
    }

    fn addressed_target() -> EncoderTarget {
        EncoderTarget::new("butt", Some("127.0.0.1".to_owned()), Some(12_556))
            .expect("addressed target")
    }

    #[test]
    fn status_parses_verified_connected_output() -> Result<()> {
        let runner = StubRunner::with_output(CommandOutput::success(VERIFIED_STATUS_OUTPUT));
        let control = EncoderControl::new(&runner);

        assert_eq!(
            control.status(&local_target())?,
            EncoderStatus {
                state: EncoderState::Connected,
                recording: RecordingState::Stopped {
                    seconds: Some(0),
                    path: None,
                },
                signal: AudioSignalState::Present,
                listeners: ListenerCount::Unknown,
                song: Some("Mr. Bungle - Sweet Charity".to_owned()),
                stream_seconds: Some(129_883),
            }
        );
        assert_eq!(
            runner.calls(),
            vec![RecordedCall {
                program: "butt".to_owned(),
                args: vec!["-S".to_owned()],
            }]
        );
        Ok(())
    }

    #[test]
    fn status_parses_disconnected_connecting_recording_and_signal_absent() -> Result<()> {
        for (output, expected_state, expected_recording, expected_signal) in [
            (
                status_text([
                    ("connected", "0"),
                    ("connecting", "0"),
                    ("recording", "0"),
                    ("signal present", "1"),
                    ("signal absent", "0"),
                    ("record seconds", "0"),
                ]),
                EncoderState::Disconnected,
                RecordingState::Stopped {
                    seconds: Some(0),
                    path: None,
                },
                AudioSignalState::Present,
            ),
            (
                status_text([
                    ("connected", "0"),
                    ("connecting", "1"),
                    ("recording", "0"),
                    ("signal present", "1"),
                    ("signal absent", "0"),
                    ("record seconds", "7"),
                ]),
                EncoderState::Connecting,
                RecordingState::Stopped {
                    seconds: Some(7),
                    path: None,
                },
                AudioSignalState::Present,
            ),
            (
                status_text([
                    ("connected", "1"),
                    ("connecting", "0"),
                    ("recording", "1"),
                    ("signal present", "0"),
                    ("signal absent", "1"),
                    ("record seconds", "42"),
                    ("record path", "/recordings/show.mp3"),
                ]),
                EncoderState::Connected,
                RecordingState::Recording {
                    seconds: Some(42),
                    path: Some("/recordings/show.mp3".to_owned()),
                },
                AudioSignalState::Absent,
            ),
        ] {
            let runner = StubRunner::with_output(CommandOutput::success(output));
            let control = EncoderControl::new(&runner);
            let status = control.status(&local_target())?;

            assert_eq!(status.state, expected_state);
            assert_eq!(status.recording, expected_recording);
            assert_eq!(status.signal, expected_signal);
        }
        Ok(())
    }

    #[test]
    fn unknown_key_does_not_fail_parse() -> Result<()> {
        let runner = StubRunner::with_output(CommandOutput::success(status_text([
            ("connected", "1"),
            ("connecting", "0"),
            ("recording", "0"),
            ("signal present", "1"),
            ("signal absent", "0"),
            ("record seconds", "0"),
            ("future key", "future value"),
        ])));
        let control = EncoderControl::new(&runner);

        assert_eq!(
            control.status(&local_target())?.state,
            EncoderState::Connected
        );
        Ok(())
    }

    #[test]
    fn malformed_or_incomplete_status_is_unknown() -> Result<()> {
        for output in ["not key value text", "connected: 1\nrecording: 0\n"] {
            let runner = StubRunner::with_output(CommandOutput::success(output));
            let control = EncoderControl::new(&runner);

            assert_eq!(control.status(&local_target())?, EncoderStatus::unknown());
        }
        Ok(())
    }

    #[test]
    fn missing_binary_maps_to_not_installed() -> Result<()> {
        let runner = StubRunner::with_error(io::Error::from(ErrorKind::NotFound).into());
        let control = EncoderControl::new(&runner);

        assert_eq!(
            control.status(&local_target())?.state,
            EncoderState::NotInstalled
        );
        Ok(())
    }

    #[test]
    fn addressed_instance_failure_maps_to_not_reachable() -> Result<()> {
        let runner = StubRunner::with_output(CommandOutput::failure(
            Some(1),
            "",
            "could not connect to 127.0.0.1:12556: connection refused",
        ));
        let control = EncoderControl::new(&runner);

        assert_eq!(
            control.status(&addressed_target())?.state,
            EncoderState::NotReachable
        );
        Ok(())
    }

    #[test]
    fn listener_count_zero_is_unknown_and_positive_is_known() -> Result<()> {
        for (listener_value, expected) in [
            ("0", ListenerCount::Unknown),
            ("17", ListenerCount::Known(17)),
        ] {
            let runner = StubRunner::with_output(CommandOutput::success(status_text([
                ("connected", "1"),
                ("connecting", "0"),
                ("recording", "0"),
                ("signal present", "1"),
                ("signal absent", "0"),
                ("record seconds", "0"),
                ("listeners", listener_value),
            ])));
            let control = EncoderControl::new(&runner);

            assert_eq!(control.status(&local_target())?.listeners, expected);
        }
        Ok(())
    }

    #[test]
    fn connect_disconnect_and_recording_commands_use_same_addressing_path() -> Result<()> {
        for (operation, expected_args) in [
            (
                EncoderOperation::Connect,
                vec![
                    "-a".to_owned(),
                    "127.0.0.1".to_owned(),
                    "-p".to_owned(),
                    "12556".to_owned(),
                    "-s".to_owned(),
                    "main".to_owned(),
                ],
            ),
            (
                EncoderOperation::Disconnect,
                vec![
                    "-a".to_owned(),
                    "127.0.0.1".to_owned(),
                    "-p".to_owned(),
                    "12556".to_owned(),
                    "-d".to_owned(),
                ],
            ),
            (
                EncoderOperation::StartRecording,
                vec![
                    "-a".to_owned(),
                    "127.0.0.1".to_owned(),
                    "-p".to_owned(),
                    "12556".to_owned(),
                    "-r".to_owned(),
                ],
            ),
            (
                EncoderOperation::StopRecording,
                vec![
                    "-a".to_owned(),
                    "127.0.0.1".to_owned(),
                    "-p".to_owned(),
                    "12556".to_owned(),
                    "-t".to_owned(),
                ],
            ),
        ] {
            let runner = StubRunner::with_output(CommandOutput::success(""));
            let control = EncoderControl::new(&runner);

            match operation {
                EncoderOperation::Connect => control.connect(&addressed_target(), "main")?,
                EncoderOperation::Disconnect => control.disconnect(&addressed_target())?,
                EncoderOperation::StartRecording => control.start_recording(&addressed_target())?,
                EncoderOperation::StopRecording => control.stop_recording(&addressed_target())?,
            }

            assert_eq!(
                runner.calls(),
                vec![RecordedCall {
                    program: "butt".to_owned(),
                    args: expected_args,
                }]
            );
        }
        Ok(())
    }

    #[test]
    fn local_connect_omits_network_address_options() -> Result<()> {
        let runner = StubRunner::with_output(CommandOutput::success(""));
        let control = EncoderControl::new(&runner);

        control.connect(&local_target(), "main")?;

        assert_eq!(
            runner.calls(),
            vec![RecordedCall {
                program: "butt".to_owned(),
                args: vec!["-s".to_owned(), "main".to_owned()],
            }]
        );
        Ok(())
    }

    #[derive(Clone, Copy)]
    enum EncoderOperation {
        Connect,
        Disconnect,
        StartRecording,
        StopRecording,
    }

    fn status_text<const N: usize>(fields: [(&str, &str); N]) -> String {
        let mut output = String::new();
        for (key, value) in fields {
            output.push_str(key);
            output.push_str(": ");
            output.push_str(value);
            output.push('\n');
        }
        output
    }
}
