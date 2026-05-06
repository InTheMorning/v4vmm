//! Typed command execution boundary.

use crate::application::command_context::CommandContext;
use crate::application::errors::command::CommandError;
use crate::application::events::ApplicationEvent;

/// Result returned by command execution.
pub type CommandResult<T> = Result<CommandOutcome<T>, CommandError>;

/// Successful command value and emitted state-change events.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandOutcome<T> {
    value: T,
    events: Vec<ApplicationEvent>,
}

impl<T> CommandOutcome<T> {
    /// Creates a command outcome from a value and event batch.
    #[must_use]
    pub fn new(value: T, events: Vec<ApplicationEvent>) -> Self {
        Self { value, events }
    }

    /// Creates a command outcome with no emitted events.
    #[must_use]
    pub fn without_events(value: T) -> Self {
        Self {
            value,
            events: Vec::new(),
        }
    }

    /// Returns the command value.
    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// Returns the emitted event batch.
    #[must_use]
    pub fn events(&self) -> &[ApplicationEvent] {
        &self.events
    }

    /// Consumes the outcome and returns its parts.
    #[must_use]
    pub fn into_parts(self) -> (T, Vec<ApplicationEvent>) {
        (self.value, self.events)
    }
}

/// Command value that can execute through the application command bus.
pub trait ApplicationCommand: Send + Sync + 'static {
    /// Successful command output.
    type Output: Send + Sync + 'static;

    /// Executes the command with a command context.
    ///
    /// # Errors
    ///
    /// Returns the command-specific error when execution fails.
    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output>;
}

/// Synchronous, GPUI-free command executor.
#[derive(Clone, Debug, Default)]
pub struct CommandBus;

impl CommandBus {
    /// Creates an empty command bus.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Executes a typed command.
    ///
    /// # Errors
    ///
    /// Returns the command-specific error when execution fails.
    pub fn execute<C>(&self, command: C, context: &CommandContext) -> CommandResult<C::Output>
    where
        C: ApplicationCommand,
    {
        command.execute(context)
    }
}
