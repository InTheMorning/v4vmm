//! GPUI command runner.

use std::sync::Arc;

use gpui::{AsyncApp, Context, WeakEntity};

use crate::application::application_event_bus::ApplicationEventBus;
use crate::application::command_bus::{ApplicationCommand, CommandBus};
use crate::application::command_context::CommandContext;
use crate::application::errors::command::CommandError;

/// Runs blocking application commands off the GPUI event path.
#[derive(Clone, Debug)]
pub struct GpuiCommandRunner {
    command_bus: Arc<CommandBus>,
    event_bus: Arc<ApplicationEventBus>,
}

impl GpuiCommandRunner {
    /// Creates a GPUI command runner.
    #[must_use]
    pub const fn new(command_bus: Arc<CommandBus>, event_bus: Arc<ApplicationEventBus>) -> Self {
        Self {
            command_bus,
            event_bus,
        }
    }

    /// Runs a command on the background executor and updates the entity after completion.
    pub fn run<T, C, OnSuccess, OnError>(
        &self,
        command: C,
        context: CommandContext,
        cx: &mut Context<T>,
        on_success: OnSuccess,
        on_error: OnError,
    ) where
        T: 'static,
        C: ApplicationCommand,
        OnSuccess: FnOnce(&mut T, C::Output, &mut Context<T>) + 'static,
        OnError: FnOnce(&mut T, CommandError, &mut Context<T>) + 'static,
    {
        let command_bus = Arc::clone(&self.command_bus);
        let event_bus = Arc::clone(&self.event_bus);
        cx.spawn(async move |this: WeakEntity<T>, cx: &mut AsyncApp| {
            let result = cx
                .background_executor()
                .spawn(async move { command_bus.execute(command, &context) })
                .await;
            this.update(cx, move |this, cx| {
                match result {
                    Ok(outcome) => {
                        let (value, events) = outcome.into_parts();
                        event_bus.broadcast(&events);
                        on_success(this, value, cx);
                    }
                    Err(error) => on_error(this, error, cx),
                }
                cx.notify();
            })
            .ok();
            cx.refresh().ok();
        })
        .detach();
    }
}
