#![warn(clippy::pedantic)]

//! Publisher target command control for ADR 0059.
//!
//! This module owns the `musicindex-live-publisher target` command boundary.
//! It runs every command through a task 010 transport, passes broadcaster
//! token file paths only, and restarts the selected publisher unit through
//! `broadcast::control` after a successful target mutation.

use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

use directories::BaseDirs;
use serde::{Deserialize, Serialize};

use crate::broadcast::control::{
    CommandOutput, CommandRunner, ProcessCommandRunner, ServiceControl, UnitRef,
};
use crate::broadcast::transport::{Reachability, Transport};

const PUBLISHER_BINARY: &str = "musicindex-live-publisher";
const TARGET_EXISTS_EXIT_CODE: i32 = 2;
const TARGET_NOT_FOUND_EXIT_CODE: i32 = 3;
const UNKNOWN_TARGET_SUBCOMMAND: &str = "unknown target subcommand";

type TargetResult<T> = std::result::Result<T, PublisherTargetCommandError>;

/// Redacted target summary returned by the publisher CLI.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PublisherTarget {
    /// Configured target name.
    pub name: String,
    /// Relay event identifier used by this target.
    pub event_id: String,
    /// Token file path on the publisher host.
    pub token_file: PathBuf,
    /// Configured stream delay in seconds.
    pub stream_delay_secs: f64,
}

/// Redacted target list returned by the publisher CLI.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct PublisherTargetList {
    /// Target rows from the publisher configuration.
    pub targets: Vec<PublisherTarget>,
}

/// Publisher target command failure with stable unavailable states.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PublisherTargetCommandError {
    /// The selected host cannot be reached through its transport.
    NotReachable,
    /// The installed publisher does not expose target commands yet.
    CommandsUnavailable,
    /// A target add tried to create an existing target.
    TargetExists {
        /// Target name reported by the caller.
        name: String,
    },
    /// A target remove tried to delete an absent target.
    TargetNotFound {
        /// Target name reported by the caller.
        name: String,
    },
    /// A caller supplied an invalid command input.
    InvalidInput {
        /// Validation failure detail.
        detail: String,
    },
    /// A command could not be spawned or waited on.
    CommandIo {
        /// Command runner failure detail.
        detail: String,
    },
    /// The publisher command failed without a narrower state.
    CommandFailed {
        /// Publisher subcommand that failed.
        operation: &'static str,
        /// Process exit code, when the platform reported one.
        code: Option<i32>,
        /// Stderr/stdout detail from the command.
        detail: String,
    },
    /// The publisher list output was not the shipped JSON contract.
    InvalidOutput {
        /// JSON parse failure detail.
        detail: String,
    },
    /// Target mutation succeeded but the publisher service restart failed.
    RestartFailed {
        /// Restart failure detail.
        detail: String,
    },
}

impl fmt::Display for PublisherTargetCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotReachable => formatter.write_str("publisher host is not reachable"),
            Self::CommandsUnavailable => {
                formatter.write_str("publisher target commands are unavailable")
            }
            Self::TargetExists { name } => write!(formatter, "publisher target {name} exists"),
            Self::TargetNotFound { name } => {
                write!(formatter, "publisher target {name} not found")
            }
            Self::InvalidInput { detail }
            | Self::CommandIo { detail }
            | Self::InvalidOutput { detail }
            | Self::RestartFailed { detail } => formatter.write_str(detail),
            Self::CommandFailed {
                operation,
                code,
                detail,
            } => match code {
                Some(code) if detail.is_empty() => {
                    write!(formatter, "{operation} failed with exit {code}")
                }
                Some(code) => write!(formatter, "{operation} failed with exit {code}: {detail}"),
                None if detail.is_empty() => write!(formatter, "{operation} failed"),
                None => write!(formatter, "{operation} failed: {detail}"),
            },
        }
    }
}

impl Error for PublisherTargetCommandError {}

/// Blocking publisher target control boundary.
#[derive(Clone, Debug)]
pub struct PublisherTargetControl<R> {
    runner: R,
}

impl Default for PublisherTargetControl<ProcessCommandRunner> {
    fn default() -> Self {
        Self {
            runner: ProcessCommandRunner,
        }
    }
}

impl<R: CommandRunner + Clone> PublisherTargetControl<R> {
    /// Build target control with an explicit command runner.
    #[must_use]
    pub const fn new(runner: R) -> Self {
        Self { runner }
    }

    /// List publisher targets for one service instance.
    ///
    /// # Errors
    ///
    /// Returns `CommandsUnavailable` for an older publisher without target
    /// commands, `NotReachable` for an unreachable host, or an output error if
    /// the JSON shape is not the shipped target-list contract.
    pub fn list_targets(
        &self,
        transport: &Transport,
        instance: &str,
    ) -> TargetResult<PublisherTargetList> {
        let args = target_list_args(transport, instance)?;
        let output = self.run_publisher_command(transport, "target list", &args)?;
        if !output.status.success() {
            return Err(classify_failure("target list", None, &output));
        }
        serde_json::from_str::<PublisherTargetList>(&output.stdout).map_err(|error| {
            PublisherTargetCommandError::InvalidOutput {
                detail: format!("parse publisher target list JSON: {error}"),
            }
        })
    }

    /// Attach an event to a publisher target and restart the publisher.
    ///
    /// # Errors
    ///
    /// Returns a stable target-command state or a restart failure. The token
    /// path is passed as a path argument; token contents are never read here.
    pub fn attach_event(
        &self,
        transport: &Transport,
        instance: &str,
        target_name: &str,
        event_id: &str,
        token_path: &Path,
    ) -> TargetResult<()> {
        let target_name = normalize_required("target name", target_name)?;
        let args = target_add_args(transport, instance, &target_name, event_id, token_path)?;
        let output = self.run_publisher_command(transport, "target add", &args)?;
        if !output.status.success() {
            return Err(classify_failure("target add", Some(&target_name), &output));
        }
        self.restart_publisher(transport, instance)
    }

    /// Detach a publisher target and restart the publisher.
    ///
    /// # Errors
    ///
    /// Returns a stable target-command state or a restart failure.
    pub fn detach_target(
        &self,
        transport: &Transport,
        instance: &str,
        target_name: &str,
    ) -> TargetResult<()> {
        let target_name = normalize_required("target name", target_name)?;
        let args = target_remove_args(transport, instance, &target_name)?;
        let output = self.run_publisher_command(transport, "target remove", &args)?;
        if !output.status.success() {
            return Err(classify_failure(
                "target remove",
                Some(&target_name),
                &output,
            ));
        }
        self.restart_publisher(transport, instance)
    }

    fn run_publisher_command(
        &self,
        transport: &Transport,
        operation: &'static str,
        args: &[String],
    ) -> TargetResult<CommandOutput> {
        let run = transport
            .run(&self.runner, PUBLISHER_BINARY, args)
            .map_err(|error| PublisherTargetCommandError::CommandIo {
                detail: format!("{operation}: {error:#}"),
            })?;
        if matches!(run.reachability, Reachability::NotReachable) {
            return Err(PublisherTargetCommandError::NotReachable);
        }
        Ok(run.output)
    }

    fn restart_publisher(&self, transport: &Transport, instance: &str) -> TargetResult<()> {
        let instance = normalize_required("publisher instance name", instance)?;
        let unit = UnitRef::publisher(&instance).map_err(|error| {
            PublisherTargetCommandError::InvalidInput {
                detail: format!("{error:#}"),
            }
        })?;
        ServiceControl::new(self.runner.clone())
            .restart(transport, &unit)
            .map_err(|error| PublisherTargetCommandError::RestartFailed {
                detail: format!("restart publisher after target change: {error:#}"),
            })
    }
}

/// List targets with the real command runner.
///
/// # Errors
///
/// Returns a stable target-command state when the host, command, or output
/// cannot satisfy the publisher control-surface contract.
pub fn list_targets(transport: &Transport, instance: &str) -> TargetResult<PublisherTargetList> {
    PublisherTargetControl::default().list_targets(transport, instance)
}

/// Attach an event with the real command runner.
///
/// # Errors
///
/// Returns a stable target-command state when attach or restart fails.
pub fn attach_event(
    transport: &Transport,
    instance: &str,
    target_name: &str,
    event_id: &str,
    token_path: &Path,
) -> TargetResult<()> {
    PublisherTargetControl::default().attach_event(
        transport,
        instance,
        target_name,
        event_id,
        token_path,
    )
}

/// Detach a target with the real command runner.
///
/// # Errors
///
/// Returns a stable target-command state when detach or restart fails.
pub fn detach_target(transport: &Transport, instance: &str, target_name: &str) -> TargetResult<()> {
    PublisherTargetControl::default().detach_target(transport, instance, target_name)
}

fn target_list_args(transport: &Transport, instance: &str) -> TargetResult<Vec<String>> {
    let config_path = publisher_config_path_arg(transport, instance)?;
    Ok(vec![
        "target".to_owned(),
        "list".to_owned(),
        "--config".to_owned(),
        config_path,
        "--json".to_owned(),
    ])
}

fn target_add_args(
    transport: &Transport,
    instance: &str,
    target_name: &str,
    event_id: &str,
    token_path: &Path,
) -> TargetResult<Vec<String>> {
    let config_path = publisher_config_path_arg(transport, instance)?;
    let event_id = normalize_required("event id", event_id)?;
    let token_path = path_arg("token file path", token_path)?;
    Ok(vec![
        "target".to_owned(),
        "add".to_owned(),
        "--config".to_owned(),
        config_path,
        "--name".to_owned(),
        target_name.to_owned(),
        "--event-id".to_owned(),
        event_id,
        "--token-file".to_owned(),
        token_path,
    ])
}

fn target_remove_args(
    transport: &Transport,
    instance: &str,
    target_name: &str,
) -> TargetResult<Vec<String>> {
    let config_path = publisher_config_path_arg(transport, instance)?;
    Ok(vec![
        "target".to_owned(),
        "remove".to_owned(),
        "--config".to_owned(),
        config_path,
        "--name".to_owned(),
        target_name.to_owned(),
    ])
}

fn publisher_config_path_arg(transport: &Transport, instance: &str) -> TargetResult<String> {
    let instance = normalize_required("publisher instance name", instance)?;
    let path = match transport {
        Transport::Local => local_publisher_config_path(&instance)?,
        Transport::Ssh { .. } => {
            format!("~/.config/musicindex-live-publisher/{instance}/config.toml")
        }
    };
    Ok(path)
}

fn local_publisher_config_path(instance: &str) -> TargetResult<String> {
    let home = BaseDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .ok_or_else(|| PublisherTargetCommandError::InvalidInput {
            detail: "could not determine user home directory".to_owned(),
        })?;
    path_arg(
        "publisher config path",
        &home
            .join(".config")
            .join("musicindex-live-publisher")
            .join(instance)
            .join("config.toml"),
    )
}

fn path_arg(label: &str, path: &Path) -> TargetResult<String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| PublisherTargetCommandError::InvalidInput {
            detail: format!("{label} must be UTF-8: {}", path.display()),
        })
}

fn normalize_required(label: &str, value: &str) -> TargetResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(PublisherTargetCommandError::InvalidInput {
            detail: format!("{label} cannot be empty"),
        });
    }
    Ok(value.to_owned())
}

fn classify_failure(
    operation: &'static str,
    target_name: Option<&str>,
    output: &CommandOutput,
) -> PublisherTargetCommandError {
    if target_commands_unavailable(output) {
        return PublisherTargetCommandError::CommandsUnavailable;
    }
    match (operation, output.status.code(), target_name) {
        ("target add", Some(TARGET_EXISTS_EXIT_CODE), Some(name)) => {
            PublisherTargetCommandError::TargetExists {
                name: name.to_owned(),
            }
        }
        ("target remove", Some(TARGET_NOT_FOUND_EXIT_CODE), Some(name)) => {
            PublisherTargetCommandError::TargetNotFound {
                name: name.to_owned(),
            }
        }
        _ => PublisherTargetCommandError::CommandFailed {
            operation,
            code: output.status.code(),
            detail: command_detail(output),
        },
    }
}

fn target_commands_unavailable(output: &CommandOutput) -> bool {
    let message = format!("{}\n{}", output.stdout, output.stderr).to_ascii_lowercase();
    message.contains(UNKNOWN_TARGET_SUBCOMMAND)
}

fn command_detail(output: &CommandOutput) -> String {
    let detail = if output.stderr.trim().is_empty() {
        output.stdout.trim()
    } else {
        output.stderr.trim()
    };
    detail.to_owned()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::path::Path;

    use anyhow::{anyhow, Result};

    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct RecordedCall {
        program: String,
        args: Vec<String>,
    }

    #[derive(Debug)]
    struct StubRunner {
        outputs: RefCell<Vec<CommandOutput>>,
        calls: RefCell<Vec<RecordedCall>>,
    }

    impl StubRunner {
        fn new(outputs: Vec<CommandOutput>) -> Self {
            Self {
                outputs: RefCell::new(outputs),
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
            let mut outputs = self.outputs.borrow_mut();
            if outputs.is_empty() {
                return Err(anyhow!("stub command output missing"));
            }
            Ok(outputs.remove(0))
        }
    }

    #[test]
    fn list_targets_parses_publisher_json_envelope() -> Result<()> {
        let runner = StubRunner::new(vec![CommandOutput::success(
            r#"{"targets":[{"name":"default","event_id":"event-one","token_file":"/tmp/default.token","stream_delay_secs":0.0}]}"#,
        )]);
        let control = PublisherTargetControl::new(&runner);

        let targets = control.list_targets(&Transport::local(), "mixxx")?;

        assert_eq!(
            targets,
            PublisherTargetList {
                targets: vec![PublisherTarget {
                    name: "default".to_owned(),
                    event_id: "event-one".to_owned(),
                    token_file: PathBuf::from("/tmp/default.token"),
                    stream_delay_secs: 0.0,
                }],
            }
        );
        assert_eq!(runner.calls().len(), 1);
        assert_eq!(runner.calls()[0].program, PUBLISHER_BINARY);
        assert_eq!(runner.calls()[0].args[0..2], ["target", "list"]);
        assert!(runner.calls()[0].args.contains(&"--json".to_owned()));
        Ok(())
    }

    #[test]
    fn attach_event_passes_token_file_path_and_restarts_publisher() -> Result<()> {
        let runner = StubRunner::new(vec![
            CommandOutput::success("written"),
            CommandOutput::success(""),
        ]);
        let control = PublisherTargetControl::new(&runner);
        let token_path = Path::new("/tmp/event-one.token");

        control.attach_event(
            &Transport::local(),
            "mixxx",
            "late-night",
            "event-one",
            token_path,
        )?;

        let calls = runner.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].program, PUBLISHER_BINARY);
        assert_eq!(calls[0].args[0..2], ["target", "add"]);
        assert!(calls[0].args.contains(&"--name".to_owned()));
        assert!(calls[0].args.contains(&"late-night".to_owned()));
        assert!(calls[0].args.contains(&"--event-id".to_owned()));
        assert!(calls[0].args.contains(&"event-one".to_owned()));
        assert!(calls[0].args.contains(&"--token-file".to_owned()));
        assert!(calls[0].args.contains(&token_path.display().to_string()));
        assert!(
            !calls[0].args.contains(&"secret-token-text".to_owned()),
            "target attach must not pass token text"
        );
        assert_eq!(calls[1].program, format!("{}{}", "system", "ctl"));
        assert_eq!(
            calls[1].args,
            vec![
                "--user".to_owned(),
                "restart".to_owned(),
                "musicindex-live-publisher@mixxx.service".to_owned(),
            ]
        );
        Ok(())
    }

    #[test]
    fn detach_target_removes_by_name_and_restarts_publisher() -> Result<()> {
        let runner = StubRunner::new(vec![
            CommandOutput::success("removed"),
            CommandOutput::success(""),
        ]);
        let control = PublisherTargetControl::new(&runner);

        control.detach_target(&Transport::local(), "mixxx", "late-night")?;

        let calls = runner.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].program, PUBLISHER_BINARY);
        assert_eq!(calls[0].args[0..2], ["target", "remove"]);
        assert!(calls[0].args.contains(&"--name".to_owned()));
        assert!(calls[0].args.contains(&"late-night".to_owned()));
        assert_eq!(calls[1].program, format!("{}{}", "system", "ctl"));
        assert!(calls[1].args.contains(&"restart".to_owned()));
        Ok(())
    }

    #[test]
    fn missing_target_command_reports_commands_unavailable_without_restart() {
        let runner = StubRunner::new(vec![CommandOutput::failure(
            Some(1),
            "",
            "unknown target subcommand attach",
        )]);
        let control = PublisherTargetControl::new(&runner);

        let error = control
            .attach_event(
                &Transport::local(),
                "mixxx",
                "default",
                "event-one",
                Path::new("/tmp/event-one.token"),
            )
            .expect_err("old publisher should report command unavailability");

        assert_eq!(error, PublisherTargetCommandError::CommandsUnavailable);
        assert_eq!(
            runner.calls().len(),
            1,
            "unavailable command must not restart"
        );
    }

    #[test]
    fn publisher_not_reachable_is_separate_from_command_failure() -> Result<()> {
        let runner = StubRunner::new(vec![CommandOutput::failure(
            Some(255),
            "",
            "ssh: connect to host studio port 22: Connection refused",
        )]);
        let control = PublisherTargetControl::new(&runner);

        let error = control
            .list_targets(&Transport::ssh("studio")?, "mixxx")
            .expect_err("ssh connect failure should be reachability");

        assert_eq!(error, PublisherTargetCommandError::NotReachable);
        Ok(())
    }

    #[test]
    fn target_exists_and_not_found_exit_codes_are_not_commands_unavailable() -> Result<()> {
        let runner = StubRunner::new(vec![CommandOutput::failure(
            Some(TARGET_EXISTS_EXIT_CODE),
            "",
            "target default already exists",
        )]);
        let control = PublisherTargetControl::new(&runner);

        let error = control
            .attach_event(
                &Transport::local(),
                "mixxx",
                "default",
                "event-one",
                Path::new("/tmp/event-one.token"),
            )
            .expect_err("duplicate target should stay distinct");

        assert_eq!(
            error,
            PublisherTargetCommandError::TargetExists {
                name: "default".to_owned(),
            }
        );

        let runner = StubRunner::new(vec![CommandOutput::failure(
            Some(TARGET_NOT_FOUND_EXIT_CODE),
            "",
            "target default not found",
        )]);
        let control = PublisherTargetControl::new(&runner);

        let error = control
            .detach_target(&Transport::local(), "mixxx", "default")
            .expect_err("missing target should stay distinct");

        assert_eq!(
            error,
            PublisherTargetCommandError::TargetNotFound {
                name: "default".to_owned(),
            }
        );
        Ok(())
    }

    #[test]
    fn ssh_target_commands_use_remote_instance_config_path() -> Result<()> {
        let runner = StubRunner::new(vec![CommandOutput::success(r#"{"targets":[]}"#)]);
        let control = PublisherTargetControl::new(&runner);

        control.list_targets(&Transport::ssh("studio")?, "mixxx")?;

        let calls = runner.calls();
        assert_eq!(calls[0].program, "ssh");
        assert!(calls[0]
            .args
            .contains(&"~/.config/musicindex-live-publisher/mixxx/config.toml".to_owned()));
        Ok(())
    }

    #[test]
    fn list_rejects_bare_array_contract_drift() {
        let runner = StubRunner::new(vec![CommandOutput::success(r#"[]"#)]);
        let control = PublisherTargetControl::new(&runner);

        let error = control
            .list_targets(&Transport::local(), "mixxx")
            .expect_err("bare target array should not satisfy the contract");

        assert!(matches!(
            error,
            PublisherTargetCommandError::InvalidOutput { .. }
        ));
    }
}
