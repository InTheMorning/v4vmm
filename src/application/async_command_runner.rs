//! GPUI-free async command runner (ADR 0040 Phase D).
//!
//! [`AsyncCommandRunner`] dispatches a typed [`ApplicationCommand`] onto the
//! tokio runtime via `spawn_blocking` (commands are synchronous, blocking
//! I/O — DB, HTTP, audio tag writes), then broadcasts the resulting
//! [`ApplicationEvent`] batch through the existing [`ApplicationEventBus`].
//!
//! The caller receives a [`tokio::sync::oneshot::Receiver`] that resolves
//! with the final [`CommandResult`]. Drop the receiver to "fire and
//! forget" — the underlying task still completes and still broadcasts
//! its events.
//!
//! ### Layer rules
//!
//! * No `gpui` / `gpui_component` imports — runs anywhere a tokio
//!   runtime exists (CLI, daemon, GPUI app).
//! * Only `src/presentation/` glue may map a [`CommandResult`] back to a
//!   GPUI entity update.
//!
//! Gated behind the `async-runtime` Cargo feature.

#![cfg(feature = "async-runtime")]
#![warn(clippy::pedantic)]

use std::sync::Arc;

use tokio::sync::oneshot;

use crate::application::application_event_bus::ApplicationEventBus;
use crate::application::command_bus::{ApplicationCommand, CommandBus, CommandResult};
use crate::application::command_context::CommandContext;

/// Async wrapper that dispatches commands onto the tokio runtime.
#[derive(Clone, Debug)]
pub struct AsyncCommandRunner {
    command_bus: Arc<CommandBus>,
    event_bus: Arc<ApplicationEventBus>,
}

impl AsyncCommandRunner {
    /// Creates a new runner.
    #[must_use]
    pub const fn new(command_bus: Arc<CommandBus>, event_bus: Arc<ApplicationEventBus>) -> Self {
        Self {
            command_bus,
            event_bus,
        }
    }

    /// Dispatches `command` onto the tokio blocking pool.
    ///
    /// Returns a oneshot receiver that resolves with the final
    /// [`CommandResult`]. Events emitted by a successful command are
    /// broadcast on the [`ApplicationEventBus`] before the receiver
    /// resolves.
    ///
    /// Dropping the receiver does **not** cancel the command — the task
    /// runs to completion and still broadcasts events. This matches the
    /// "fire-and-forget" semantics of ADR 0040.
    ///
    /// # Panics
    ///
    /// Panics if no tokio runtime is active on the calling thread.
    pub fn dispatch<C>(
        &self,
        command: C,
        context: CommandContext,
    ) -> oneshot::Receiver<CommandResult<C::Output>>
    where
        C: ApplicationCommand,
    {
        let (tx, rx) = oneshot::channel();
        let command_bus = Arc::clone(&self.command_bus);
        let event_bus = Arc::clone(&self.event_bus);

        tokio::task::spawn_blocking(move || {
            let result = command_bus.execute(command, &context);
            // Broadcast events on the same blocking thread so subscribers
            // observe state changes before the caller learns about them.
            if let Ok(outcome) = result.as_ref() {
                event_bus.broadcast(outcome.events());
            }
            // Drop result to caller. If the receiver was dropped, the
            // send fails silently — that is the intended fire-and-forget
            // path.
            let _ = tx.send(result);
        });

        rx
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::command_bus::CommandOutcome;
    use crate::application::command_context::CommandContext;
    use crate::application::events::{playlist::PlaylistEvent, ApplicationEvent};
    use std::sync::Mutex;

    struct CountingSubscriber {
        seen: Arc<Mutex<Vec<ApplicationEvent>>>,
    }

    impl crate::application::application_event_bus::ApplicationEventSubscriber for CountingSubscriber {
        fn on_application_events(&self, events: &[ApplicationEvent]) {
            self.seen.lock().expect("lock").extend_from_slice(events);
        }
    }

    struct EchoCommand {
        value: i64,
        emit: Vec<ApplicationEvent>,
    }

    impl ApplicationCommand for EchoCommand {
        type Output = i64;

        fn execute(self, _context: &CommandContext) -> CommandResult<Self::Output> {
            Ok(CommandOutcome::new(self.value, self.emit))
        }
    }

    fn empty_context() -> CommandContext {
        CommandContext::next()
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn dispatch_returns_command_value() {
        let runner = AsyncCommandRunner::new(
            Arc::new(CommandBus::new()),
            Arc::new(ApplicationEventBus::new()),
        );
        let rx = runner.dispatch(
            EchoCommand {
                value: 42,
                emit: Vec::new(),
            },
            empty_context(),
        );
        let outcome = rx.await.expect("oneshot").expect("ok");
        assert_eq!(*outcome.value(), 42);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn dispatch_broadcasts_events_before_resolving() {
        let event_bus = Arc::new(ApplicationEventBus::new());
        let seen = Arc::new(Mutex::new(Vec::new()));
        event_bus.subscribe(Arc::new(CountingSubscriber {
            seen: Arc::clone(&seen),
        }));
        let runner = AsyncCommandRunner::new(Arc::new(CommandBus::new()), Arc::clone(&event_bus));

        let rx = runner.dispatch(
            EchoCommand {
                value: 1,
                emit: vec![ApplicationEvent::Playlist(PlaylistEvent::Changed)],
            },
            empty_context(),
        );
        let _ = rx.await.expect("oneshot");
        let recorded = seen.lock().expect("lock").clone();
        assert_eq!(recorded.len(), 1);
        assert!(matches!(
            recorded[0],
            ApplicationEvent::Playlist(PlaylistEvent::Changed)
        ));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn dropping_receiver_still_runs_command_and_broadcasts() {
        let event_bus = Arc::new(ApplicationEventBus::new());
        let seen = Arc::new(Mutex::new(Vec::new()));
        event_bus.subscribe(Arc::new(CountingSubscriber {
            seen: Arc::clone(&seen),
        }));
        let runner = AsyncCommandRunner::new(Arc::new(CommandBus::new()), Arc::clone(&event_bus));

        drop(runner.dispatch(
            EchoCommand {
                value: 0,
                emit: vec![ApplicationEvent::Playlist(PlaylistEvent::Changed)],
            },
            empty_context(),
        ));

        // Allow blocking task to complete.
        for _ in 0..50 {
            if !seen.lock().expect("lock").is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert_eq!(seen.lock().expect("lock").len(), 1);
    }
}
