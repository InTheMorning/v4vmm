#![warn(clippy::pedantic)]

//! Broadcast host command transport for ADR 0059.
//!
//! A transport chooses where a command runs. It does not know which broadcast
//! service operation the command represents, and it never builds a shell string.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::broadcast::control::{CommandOutput, CommandRunner};

const SSH: &str = "ssh";
const SSH_BATCH_MODE_OPTION: &str = "BatchMode=yes";
const SSH_CONNECT_TIMEOUT_OPTION: &str = "ConnectTimeout=5";
const SSH_CONNECT_FAILURE_EXIT_CODE: i32 = 255;
const SSH_CONNECT_FAILURE_MARKERS: &[&str] = &[
    "connection refused",
    "connection timed out",
    "could not resolve hostname",
    "host key verification failed",
    "kex_exchange_identification",
    "name or service not known",
    "network is unreachable",
    "no route to host",
    "operation timed out",
    "permission denied",
    "ssh_exchange_identification",
    "temporary failure in name resolution",
];

/// Command transport for one broadcast host.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "transport", rename_all = "snake_case")]
pub enum Transport {
    /// Run the command on this machine.
    #[default]
    Local,
    /// Run the command on another machine through the operator's SSH setup.
    Ssh {
        /// SSH destination passed as one argument to the `ssh` program.
        destination: String,
    },
}

impl Transport {
    /// Create a local transport.
    #[must_use]
    pub const fn local() -> Self {
        Self::Local
    }

    /// Create an SSH transport.
    ///
    /// # Errors
    ///
    /// Returns an error when the destination is empty after trimming.
    pub fn ssh(destination: impl Into<String>) -> Result<Self> {
        let destination = normalize_destination(&destination.into())?;
        Ok(Self::Ssh { destination })
    }

    /// Run one command through this transport.
    ///
    /// # Errors
    ///
    /// Returns an error when the local process cannot be spawned or waited on,
    /// or when an SSH destination is empty.
    pub fn run<R: CommandRunner + ?Sized>(
        &self,
        runner: &R,
        program: &str,
        args: &[String],
    ) -> Result<RunOutput> {
        let output = match self {
            Self::Local => runner.run(program, args)?,
            Self::Ssh { destination } => {
                let args = ssh_args(destination, program, args)?;
                runner.run(SSH, &args)?
            }
        };
        Ok(RunOutput {
            reachability: self.reachability(&output),
            output,
        })
    }

    /// Validate this transport.
    ///
    /// # Errors
    ///
    /// Returns an error when an SSH destination is empty after trimming.
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Local => Ok(()),
            Self::Ssh { destination } => normalize_destination(destination).map(|_| ()),
        }
    }

    fn reachability(&self, output: &CommandOutput) -> Reachability {
        if matches!(self, Self::Ssh { .. }) && ssh_connect_failed(output) {
            Reachability::NotReachable
        } else {
            Reachability::Reachable
        }
    }
}

/// Result of reaching the selected broadcast host.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reachability {
    /// The host accepted the command and returned command output.
    Reachable,
    /// The host could not be reached or authenticated by SSH.
    NotReachable,
}

impl Reachability {
    /// Return whether the transport reached the selected host.
    #[must_use]
    pub const fn is_reachable(self) -> bool {
        matches!(self, Self::Reachable)
    }
}

/// Output from a command after transport reachability classification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunOutput {
    /// Whether the selected host was reachable.
    pub reachability: Reachability,
    /// Raw process output from the local runner.
    pub output: CommandOutput,
}

fn ssh_args(destination: &str, program: &str, args: &[String]) -> Result<Vec<String>> {
    let destination = normalize_destination(destination)?;
    let mut ssh_args = Vec::with_capacity(args.len() + 6);
    ssh_args.extend(
        [
            "-o",
            SSH_BATCH_MODE_OPTION,
            "-o",
            SSH_CONNECT_TIMEOUT_OPTION,
            destination.as_str(),
            program,
        ]
        .into_iter()
        .map(str::to_owned),
    );
    ssh_args.extend(args.iter().cloned());
    Ok(ssh_args)
}

fn normalize_destination(destination: &str) -> Result<String> {
    let destination = destination.trim();
    if destination.is_empty() {
        return Err(anyhow!("ssh destination cannot be empty"));
    }
    Ok(destination.to_owned())
}

fn ssh_connect_failed(output: &CommandOutput) -> bool {
    if output.status.code() != Some(SSH_CONNECT_FAILURE_EXIT_CODE) {
        return false;
    }
    let stderr = output.stderr.to_ascii_lowercase();
    SSH_CONNECT_FAILURE_MARKERS
        .iter()
        .any(|marker| stderr.contains(marker))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use anyhow::Context;

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

    #[test]
    fn local_transport_preserves_program_and_arguments() -> Result<()> {
        let runner = StubRunner::with_output(CommandOutput::success("ok"));
        let args = vec!["--user".to_owned(), "status".to_owned()];
        let output = Transport::local().run(&&runner, "service-tool", &args)?;

        assert_eq!(output.reachability, Reachability::Reachable);
        assert_eq!(
            runner.calls(),
            vec![RecordedCall {
                program: "service-tool".to_owned(),
                args,
            }]
        );
        Ok(())
    }

    #[test]
    fn ssh_transport_wraps_program_after_options_and_destination() -> Result<()> {
        let runner = StubRunner::with_output(CommandOutput::success("ok"));
        let args = vec!["--user".to_owned(), "status".to_owned()];
        let output = Transport::ssh("studio-box")?.run(&&runner, "service-tool", &args)?;

        assert_eq!(output.reachability, Reachability::Reachable);
        assert_eq!(
            runner.calls(),
            vec![RecordedCall {
                program: "ssh".to_owned(),
                args: vec![
                    "-o".to_owned(),
                    "BatchMode=yes".to_owned(),
                    "-o".to_owned(),
                    "ConnectTimeout=5".to_owned(),
                    "studio-box".to_owned(),
                    "service-tool".to_owned(),
                    "--user".to_owned(),
                    "status".to_owned(),
                ],
            }]
        );
        Ok(())
    }

    #[test]
    fn ssh_destination_with_shell_characters_stays_one_argument() -> Result<()> {
        let runner = StubRunner::with_output(CommandOutput::success("ok"));
        let destination = "studio host; touch /tmp/injected";

        Transport::ssh(destination)?.run(&&runner, "cat", &["/tmp/drop".to_owned()])?;

        assert_eq!(
            runner.calls(),
            vec![RecordedCall {
                program: "ssh".to_owned(),
                args: vec![
                    "-o".to_owned(),
                    "BatchMode=yes".to_owned(),
                    "-o".to_owned(),
                    "ConnectTimeout=5".to_owned(),
                    destination.to_owned(),
                    "cat".to_owned(),
                    "/tmp/drop".to_owned(),
                ],
            }]
        );
        Ok(())
    }

    #[test]
    fn ssh_connection_failure_is_not_reachable() -> Result<()> {
        let runner = StubRunner::with_output(CommandOutput::failure(
            Some(255),
            "",
            "ssh: connect to host studio-box port 22: Connection timed out",
        ));
        let output = Transport::ssh("studio-box")?.run(&&runner, "service-tool", &[])?;

        assert_eq!(output.reachability, Reachability::NotReachable);
        Ok(())
    }

    #[test]
    fn remote_command_failure_after_connection_stays_reachable() -> Result<()> {
        let runner = StubRunner::with_output(CommandOutput::failure(
            Some(1),
            "",
            "Failed to get properties: Unit not found",
        ));
        let output = Transport::ssh("studio-box")?.run(&&runner, "service-tool", &[])?;

        assert_eq!(output.reachability, Reachability::Reachable);
        Ok(())
    }
}
