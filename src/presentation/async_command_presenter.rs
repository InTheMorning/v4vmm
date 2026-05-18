//! GPUI bridge for ADR 0040 async command completion callbacks.
//!
//! Screens use this presenter to keep existing success/error callback
//! ownership while command execution runs through [`AsyncCommandRunner`].

use gpui::{AsyncApp, Context, WeakEntity};

use crate::application::command_bus::ApplicationCommand;
use crate::application::command_context::CommandContext;
use crate::application::errors::command::CommandError;
use crate::application::AsyncCommandRunner;

/// Dispatches a command and presents its result back on the GPUI entity.
pub fn present_command<T, C, OnSuccess, OnError>(
    runner: &AsyncCommandRunner,
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
    let rx = runner.dispatch(command, context);
    cx.spawn(async move |this: WeakEntity<T>, cx: &mut AsyncApp| {
        let result = rx.await.unwrap_or_else(|_| {
            Err(CommandError::Other(
                "command result channel closed before completion".to_string(),
            ))
        });
        this.update(cx, move |this, cx| {
            match result {
                Ok(outcome) => {
                    let (value, _events) = outcome.into_parts();
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
