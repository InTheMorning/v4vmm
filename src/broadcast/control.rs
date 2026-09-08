#![warn(clippy::pedantic)]

//! Systemd service control for ADR 0059.
//!
//! This module owns the blocking `systemctl --user` and `journalctl --user`
//! boundary for publisher-side services. Callers pass unit names as values and
//! receive typed service states instead of parsing human status text.

use std::collections::BTreeMap;
use std::process::Command;

use anyhow::{anyhow, Context, Result};

const SYSTEMCTL: &str = "systemctl";
const JOURNALCTL: &str = "journalctl";
const SHOW_PROPERTIES: &str = "--property=LoadState,ActiveState,SubState,Result";
const PUBLISHER_UNIT_PREFIX: &str = "musicindex-live-publisher@";
const PUBLISHER_UNIT_SUFFIX: &str = ".service";

/// Reference to one systemd user unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitRef {
    unit: String,
}

impl UnitRef {
    /// Create a unit reference from a complete unit name.
    ///
    /// # Errors
    ///
    /// Returns an error when the unit name is empty.
    pub fn new(unit: impl Into<String>) -> Result<Self> {
        let unit = unit.into();
        let unit = unit.trim();
        if unit.is_empty() {
            return Err(anyhow!("systemd unit name cannot be empty"));
        }
        Ok(Self {
            unit: unit.to_owned(),
        })
    }

    /// Create the publisher instance unit name.
    ///
    /// # Errors
    ///
    /// Returns an error when the instance name is empty.
    pub fn publisher(instance: &str) -> Result<Self> {
        let instance = instance.trim();
        if instance.is_empty() {
            return Err(anyhow!("publisher instance name cannot be empty"));
        }
        Self::new(format!(
            "{PUBLISHER_UNIT_PREFIX}{instance}{PUBLISHER_UNIT_SUFFIX}"
        ))
    }

    /// Return the complete unit name.
    #[must_use]
    pub fn unit(&self) -> &str {
        &self.unit
    }
}

/// State of one publisher-side service unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServiceState {
    /// The service is installed and running.
    Active,
    /// The service is installed and stopped.
    Inactive,
    /// The service is failed and needs `reset-failed` before `start`.
    Failed {
        /// The systemd `Result` value for the failed unit.
        reason: String,
    },
    /// The unit file is absent from the user manager.
    NotInstalled,
    /// The host that owns the unit cannot be reached.
    NotReachable,
    /// The unit is in a state that this surface does not classify.
    Unknown,
}

impl ServiceState {
    /// Return whether `start` is useful for this state.
    #[must_use]
    pub const fn start_is_useful(&self) -> bool {
        !matches!(self, Self::Failed { .. })
    }
}

/// Exit status reported by a command runner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandExitStatus {
    success: bool,
    code: Option<i32>,
}

impl CommandExitStatus {
    /// Create a runner-neutral exit status.
    #[must_use]
    pub const fn new(success: bool, code: Option<i32>) -> Self {
        Self { success, code }
    }

    /// Return whether the command exited successfully.
    #[must_use]
    pub const fn success(self) -> bool {
        self.success
    }

    /// Return the process exit code, when the platform reported one.
    #[must_use]
    pub const fn code(self) -> Option<i32> {
        self.code
    }
}

/// Output from a command runner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandOutput {
    /// Runner-neutral exit status.
    pub status: CommandExitStatus,
    /// Standard output, decoded as UTF-8 with replacement.
    pub stdout: String,
    /// Standard error, decoded as UTF-8 with replacement.
    pub stderr: String,
}

impl CommandOutput {
    /// Build successful command output for tests and adapters.
    #[must_use]
    pub fn success(stdout: impl Into<String>) -> Self {
        Self {
            status: CommandExitStatus::new(true, Some(0)),
            stdout: stdout.into(),
            stderr: String::new(),
        }
    }

    /// Build failed command output for tests and adapters.
    #[must_use]
    pub fn failure(
        code: Option<i32>,
        stdout: impl Into<String>,
        stderr: impl Into<String>,
    ) -> Self {
        Self {
            status: CommandExitStatus::new(false, code),
            stdout: stdout.into(),
            stderr: stderr.into(),
        }
    }
}

/// Runs one program with argument-vector semantics.
pub trait CommandRunner {
    /// Run a program and return its exit status and output streams.
    ///
    /// # Errors
    ///
    /// Returns an error when the process cannot be spawned or waited on.
    fn run(&self, program: &str, args: &[String]) -> Result<CommandOutput>;
}

/// Real command runner backed by [`std::process::Command`].
#[derive(Clone, Copy, Debug, Default)]
pub struct ProcessCommandRunner;

impl CommandRunner for ProcessCommandRunner {
    fn run(&self, program: &str, args: &[String]) -> Result<CommandOutput> {
        let output = Command::new(program)
            .args(args)
            .output()
            .with_context(|| format!("run {program}"))?;
        Ok(CommandOutput {
            status: CommandExitStatus::new(output.status.success(), output.status.code()),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

/// Blocking service control boundary.
#[derive(Clone, Debug)]
pub struct ServiceControl<R> {
    runner: R,
}

impl Default for ServiceControl<ProcessCommandRunner> {
    fn default() -> Self {
        Self {
            runner: ProcessCommandRunner,
        }
    }
}

impl<R: CommandRunner> ServiceControl<R> {
    /// Build service control with an explicit command runner.
    #[must_use]
    pub const fn new(runner: R) -> Self {
        Self { runner }
    }

    /// Read one unit's service state.
    ///
    /// # Errors
    ///
    /// Returns an error when `systemctl show` cannot run or returns no output.
    pub fn show(&self, unit: &UnitRef) -> Result<ServiceState> {
        let output = self.runner.run(SYSTEMCTL, &systemctl_show_args(unit))?;
        if output.stdout.trim().is_empty() && !output.status.success() {
            return Err(command_failure(SYSTEMCTL, &output));
        }
        Ok(parse_service_state(&output.stdout))
    }

    /// Start one unit.
    ///
    /// # Errors
    ///
    /// Returns an error when `systemctl start` fails.
    pub fn start(&self, unit: &UnitRef) -> Result<()> {
        self.run_systemctl_unit("start", unit)
    }

    /// Stop one unit.
    ///
    /// # Errors
    ///
    /// Returns an error when `systemctl stop` fails.
    pub fn stop(&self, unit: &UnitRef) -> Result<()> {
        self.run_systemctl_unit("stop", unit)
    }

    /// Restart one unit.
    ///
    /// # Errors
    ///
    /// Returns an error when `systemctl restart` fails.
    pub fn restart(&self, unit: &UnitRef) -> Result<()> {
        self.run_systemctl_unit("restart", unit)
    }

    /// Reset one failed unit.
    ///
    /// # Errors
    ///
    /// Returns an error when `systemctl reset-failed` fails.
    pub fn reset(&self, unit: &UnitRef) -> Result<()> {
        self.run_systemctl_unit("reset-failed", unit)
    }

    /// Reload the user systemd manager.
    ///
    /// # Errors
    ///
    /// Returns an error when `systemctl daemon-reload` fails.
    pub fn daemon_reload(&self) -> Result<()> {
        let args = systemctl_args(["--user", "daemon-reload"]);
        self.run_checked(SYSTEMCTL, &args).map(|_| ())
    }

    /// Read journal text for one unit.
    ///
    /// # Errors
    ///
    /// Returns an error when `journalctl` fails.
    pub fn logs(&self, unit: &UnitRef, lines: usize) -> Result<String> {
        let line_count = lines.to_string();
        let args = vec![
            "--user".to_owned(),
            "-u".to_owned(),
            unit.unit().to_owned(),
            "-n".to_owned(),
            line_count,
            "--no-pager".to_owned(),
        ];
        let output = self.run_checked(JOURNALCTL, &args)?;
        Ok(output.stdout)
    }

    fn run_systemctl_unit(&self, operation: &str, unit: &UnitRef) -> Result<()> {
        let args = systemctl_args(["--user", operation, unit.unit()]);
        self.run_checked(SYSTEMCTL, &args).map(|_| ())
    }

    fn run_checked(&self, program: &str, args: &[String]) -> Result<CommandOutput> {
        let output = self.runner.run(program, args)?;
        if output.status.success() {
            return Ok(output);
        }
        Err(command_failure(program, &output))
    }
}

/// Read one unit's service state with the real command runner.
///
/// # Errors
///
/// Returns an error when `systemctl show` cannot run or returns no output.
pub fn show(unit: &UnitRef) -> Result<ServiceState> {
    ServiceControl::default().show(unit)
}

/// Start one unit with the real command runner.
///
/// # Errors
///
/// Returns an error when `systemctl start` fails.
pub fn start(unit: &UnitRef) -> Result<()> {
    ServiceControl::default().start(unit)
}

/// Stop one unit with the real command runner.
///
/// # Errors
///
/// Returns an error when `systemctl stop` fails.
pub fn stop(unit: &UnitRef) -> Result<()> {
    ServiceControl::default().stop(unit)
}

/// Restart one unit with the real command runner.
///
/// # Errors
///
/// Returns an error when `systemctl restart` fails.
pub fn restart(unit: &UnitRef) -> Result<()> {
    ServiceControl::default().restart(unit)
}

/// Reset one failed unit with the real command runner.
///
/// # Errors
///
/// Returns an error when `systemctl reset-failed` fails.
pub fn reset(unit: &UnitRef) -> Result<()> {
    ServiceControl::default().reset(unit)
}

/// Reload the user systemd manager with the real command runner.
///
/// # Errors
///
/// Returns an error when `systemctl daemon-reload` fails.
pub fn daemon_reload() -> Result<()> {
    ServiceControl::default().daemon_reload()
}

/// Read journal text with the real command runner.
///
/// # Errors
///
/// Returns an error when `journalctl` fails.
pub fn logs(unit: &UnitRef, lines: usize) -> Result<String> {
    ServiceControl::default().logs(unit, lines)
}

fn parse_service_state(output: &str) -> ServiceState {
    let properties = parse_properties(output);
    if properties
        .get("LoadState")
        .is_some_and(|state| *state == "not-found")
    {
        return ServiceState::NotInstalled;
    }

    match properties.get("ActiveState").copied().unwrap_or_default() {
        "active" => ServiceState::Active,
        "inactive" => ServiceState::Inactive,
        "failed" => ServiceState::Failed {
            reason: failed_reason(&properties),
        },
        _ => ServiceState::Unknown,
    }
}

fn parse_properties(output: &str) -> BTreeMap<&str, &str> {
    output
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim(), value.trim()))
        .collect()
}

fn failed_reason(properties: &BTreeMap<&str, &str>) -> String {
    properties
        .get("Result")
        .copied()
        .filter(|result| !result.is_empty() && *result != "success")
        .unwrap_or("failed")
        .to_owned()
}

fn systemctl_show_args(unit: &UnitRef) -> Vec<String> {
    systemctl_args(["--user", "show", unit.unit(), SHOW_PROPERTIES])
}

fn systemctl_args<const N: usize>(args: [&str; N]) -> Vec<String> {
    args.into_iter().map(str::to_owned).collect()
}

fn command_failure(program: &str, output: &CommandOutput) -> anyhow::Error {
    let status = match output.status.code() {
        Some(code) => format!("exit {code}"),
        None => "terminated by signal".to_owned(),
    };
    let detail = output.stderr.trim();
    if detail.is_empty() {
        anyhow!("{program} failed with {status}")
    } else {
        anyhow!("{program} failed with {status}: {detail}")
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct RecordedCall {
        program: String,
        args: Vec<String>,
    }

    #[derive(Debug)]
    struct StubRunner {
        output: RefCell<Option<CommandOutput>>,
        calls: RefCell<Vec<RecordedCall>>,
    }

    impl StubRunner {
        fn with_output(output: CommandOutput) -> Self {
            Self {
                output: RefCell::new(Some(output)),
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
                .context("stub command output missing")
        }
    }

    fn unit() -> UnitRef {
        UnitRef::new("musicindex-live-publisher@mixxx.service").expect("unit")
    }

    fn show_output(load_state: &str, active_state: &str, sub_state: &str, result: &str) -> String {
        format!(
            "LoadState={load_state}\nActiveState={active_state}\nSubState={sub_state}\nResult={result}\n"
        )
    }

    #[test]
    fn publisher_unit_name_uses_instance_input() -> Result<()> {
        let unit = UnitRef::publisher("mixxx")?;

        assert_eq!(unit.unit(), "musicindex-live-publisher@mixxx.service");
        Ok(())
    }

    #[test]
    fn unit_ref_rejects_empty_names() {
        assert!(UnitRef::new("  ").is_err());
        assert!(UnitRef::publisher("").is_err());
    }

    #[test]
    fn show_reads_exact_systemctl_properties() -> Result<()> {
        let runner = StubRunner::with_output(CommandOutput::success(show_output(
            "loaded", "active", "running", "success",
        )));
        let service = ServiceControl::new(&runner);

        assert_eq!(service.show(&unit())?, ServiceState::Active);
        assert_eq!(
            runner.calls(),
            vec![RecordedCall {
                program: "systemctl".to_owned(),
                args: vec![
                    "--user".to_owned(),
                    "show".to_owned(),
                    "musicindex-live-publisher@mixxx.service".to_owned(),
                    "--property=LoadState,ActiveState,SubState,Result".to_owned(),
                ],
            }]
        );
        Ok(())
    }

    #[test]
    fn show_maps_inactive_failed_not_installed_and_unknown() -> Result<()> {
        for (output, expected) in [
            (
                show_output("loaded", "inactive", "dead", "success"),
                ServiceState::Inactive,
            ),
            (
                show_output("loaded", "failed", "failed", "exit-code"),
                ServiceState::Failed {
                    reason: "exit-code".to_owned(),
                },
            ),
            (
                show_output("not-found", "inactive", "dead", "success"),
                ServiceState::NotInstalled,
            ),
            (
                show_output("loaded", "activating", "auto-restart", "success"),
                ServiceState::Unknown,
            ),
        ] {
            let runner = StubRunner::with_output(CommandOutput::success(output));
            let service = ServiceControl::new(&runner);

            assert_eq!(service.show(&unit())?, expected);
        }
        Ok(())
    }

    #[test]
    fn failed_state_uses_result_or_fallback_reason() -> Result<()> {
        for (output, expected_reason) in [
            (
                show_output("loaded", "failed", "failed", "exit-code"),
                "exit-code",
            ),
            (show_output("loaded", "failed", "failed", ""), "failed"),
            (
                show_output("loaded", "failed", "failed", "success"),
                "failed",
            ),
        ] {
            let runner = StubRunner::with_output(CommandOutput::success(output));
            let service = ServiceControl::new(&runner);

            assert_eq!(
                service.show(&unit())?,
                ServiceState::Failed {
                    reason: expected_reason.to_owned(),
                }
            );
        }
        Ok(())
    }

    #[test]
    fn failed_state_reports_start_as_not_useful_until_reset() {
        let failed = ServiceState::Failed {
            reason: "exit-code".to_owned(),
        };

        assert!(!failed.start_is_useful());
        assert!(ServiceState::Inactive.start_is_useful());
    }

    #[test]
    fn service_operations_use_user_systemctl_commands() -> Result<()> {
        for expected_operation in ["start", "stop", "restart", "reset-failed"] {
            let runner = StubRunner::with_output(CommandOutput::success(""));
            let service = ServiceControl::new(&runner);

            match expected_operation {
                "start" => service.start(&unit())?,
                "stop" => service.stop(&unit())?,
                "restart" => service.restart(&unit())?,
                "reset-failed" => service.reset(&unit())?,
                _ => unreachable!("test covers known operations only"),
            }

            assert_eq!(
                runner.calls(),
                vec![RecordedCall {
                    program: "systemctl".to_owned(),
                    args: vec![
                        "--user".to_owned(),
                        expected_operation.to_owned(),
                        "musicindex-live-publisher@mixxx.service".to_owned(),
                    ],
                }]
            );
        }
        Ok(())
    }

    #[test]
    fn daemon_reload_uses_user_manager() -> Result<()> {
        let runner = StubRunner::with_output(CommandOutput::success(""));
        let service = ServiceControl::new(&runner);

        service.daemon_reload()?;

        assert_eq!(
            runner.calls(),
            vec![RecordedCall {
                program: "systemctl".to_owned(),
                args: vec!["--user".to_owned(), "daemon-reload".to_owned()],
            }]
        );
        Ok(())
    }

    #[test]
    fn logs_read_journal_text_and_allow_empty_output() -> Result<()> {
        for expected in ["line one\nline two\n", ""] {
            let runner = StubRunner::with_output(CommandOutput::success(expected));
            let service = ServiceControl::new(&runner);

            assert_eq!(service.logs(&unit(), 50)?, expected);
            assert_eq!(
                runner.calls(),
                vec![RecordedCall {
                    program: "journalctl".to_owned(),
                    args: vec![
                        "--user".to_owned(),
                        "-u".to_owned(),
                        "musicindex-live-publisher@mixxx.service".to_owned(),
                        "-n".to_owned(),
                        "50".to_owned(),
                        "--no-pager".to_owned(),
                    ],
                }]
            );
        }
        Ok(())
    }

    #[test]
    fn failed_commands_return_stderr_context() {
        let runner = StubRunner::with_output(CommandOutput::failure(
            Some(1),
            "",
            "unit could not be started",
        ));
        let service = ServiceControl::new(&runner);

        let error = service.start(&unit()).expect_err("start should fail");

        assert!(error.to_string().contains("unit could not be started"));
    }
}
