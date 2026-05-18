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
//! The async runtime is always available in the desktop build.

#![warn(clippy::pedantic)]

use std::sync::Arc;

use tokio::runtime::Handle;
use tokio::sync::oneshot;

use crate::application::application_event_bus::ApplicationEventBus;
use crate::application::command_bus::{ApplicationCommand, CommandBus, CommandResult};
use crate::application::command_context::CommandContext;
use crate::application::events::ApplicationEvent;
use crate::runtime::{VmBus, VmEvent};

/// Async wrapper that dispatches commands onto the tokio runtime.
#[derive(Clone, Debug)]
#[allow(clippy::struct_field_names)]
pub struct AsyncCommandRunner {
    command_bus: Arc<CommandBus>,
    event_bus: Arc<ApplicationEventBus>,
    vm_bus: Option<VmBus>,
    runtime_handle: Handle,
}

impl AsyncCommandRunner {
    /// Creates a new runner without a `VmBus`.
    ///
    /// Callers that want command outcomes to invalidate VM caches should
    /// use [`Self::with_vm_bus`] instead.
    #[must_use]
    ///
    /// # Panics
    ///
    /// Panics if no tokio runtime is active on the calling thread. GPUI
    /// composition roots should prefer [`Self::with_runtime_handle`] or
    /// [`Self::with_vm_bus_on_handle`] using the process [`RuntimeHost`]
    /// handle.
    ///
    /// [`RuntimeHost`]: crate::presentation::RuntimeHost
    pub fn new(command_bus: Arc<CommandBus>, event_bus: Arc<ApplicationEventBus>) -> Self {
        Self::with_runtime_handle(command_bus, event_bus, Handle::current())
    }

    /// Creates a runner on an explicit tokio runtime handle.
    #[must_use]
    pub fn with_runtime_handle(
        command_bus: Arc<CommandBus>,
        event_bus: Arc<ApplicationEventBus>,
        runtime_handle: Handle,
    ) -> Self {
        Self {
            command_bus,
            event_bus,
            vm_bus: None,
            runtime_handle,
        }
    }

    /// Creates a runner that also republishes successful command events
    /// onto the runtime [`VmBus`] so paged actors can drop caches.
    #[must_use]
    pub fn with_vm_bus(
        command_bus: Arc<CommandBus>,
        event_bus: Arc<ApplicationEventBus>,
        vm_bus: VmBus,
    ) -> Self {
        Self::with_vm_bus_on_handle(command_bus, event_bus, vm_bus, Handle::current())
    }

    /// Creates a VM-invalidating runner on an explicit tokio runtime handle.
    #[must_use]
    pub fn with_vm_bus_on_handle(
        command_bus: Arc<CommandBus>,
        event_bus: Arc<ApplicationEventBus>,
        vm_bus: VmBus,
        runtime_handle: Handle,
    ) -> Self {
        Self {
            command_bus,
            event_bus,
            vm_bus: Some(vm_bus),
            runtime_handle,
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
    /// Uses the runtime handle captured at construction time.
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
        let vm_bus = self.vm_bus.clone();
        let runtime_handle = self.runtime_handle.clone();

        runtime_handle.spawn_blocking(move || {
            let result = command_bus.execute(command, &context);
            // Broadcast events on the same blocking thread so subscribers
            // observe state changes before the caller learns about them.
            if let Ok(outcome) = result.as_ref() {
                event_bus.broadcast(outcome.events());
                if let Some(bus) = vm_bus.as_ref() {
                    publish_vm_invalidations(bus, outcome.events());
                }
            }
            // Drop result to caller. If the receiver was dropped, the
            // send fails silently — that is the intended fire-and-forget
            // path.
            let _ = tx.send(result);
        });

        rx
    }
}

/// Translate `ApplicationEvent`s emitted by a successful command into
/// coarse-grained `VmEvent` invalidations on the runtime bus.
///
/// Most application events are id-less today, so they map to
/// `InvalidateAll`; `PlaylistEvent::TracksChanged { playlist_id }`
/// carries an id and maps to a precise `PlaylistChanged`. Add finer
/// mappings here as the typed event surface grows.
fn publish_vm_invalidations(bus: &VmBus, events: &[ApplicationEvent]) {
    use crate::application::events::{
        download::DownloadEvent, feed::FeedEvent, library::LibraryEvent, metadata::MetadataEvent,
        playlist::PlaylistEvent,
    };

    for event in events {
        match event {
            ApplicationEvent::Library(LibraryEvent::Changed)
            | ApplicationEvent::Feed(FeedEvent::Changed)
            | ApplicationEvent::Download(DownloadEvent::Changed)
            | ApplicationEvent::Metadata(MetadataEvent::Changed)
            | ApplicationEvent::Playlist(PlaylistEvent::Changed) => {
                bus.publish(VmEvent::InvalidateAll);
            }
            ApplicationEvent::Metadata(MetadataEvent::TrackTagged { track_id }) => {
                bus.publish(VmEvent::TrackChanged {
                    track_id: *track_id,
                });
            }
            ApplicationEvent::Playlist(PlaylistEvent::TracksChanged { playlist_id }) => {
                bus.publish(VmEvent::PlaylistChanged {
                    playlist_id: *playlist_id,
                });
            }
            // Playback events are session-state, not VM-cached state.
            ApplicationEvent::Playback(_) => {}
        }
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

    struct CancellableCommand {
        max_iters: u32,
    }

    impl ApplicationCommand for CancellableCommand {
        type Output = u32;

        fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
            for i in 0..self.max_iters {
                if context.cancellation().is_cancelled() {
                    return Ok(CommandOutcome::without_events(i));
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Ok(CommandOutcome::without_events(self.max_iters))
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cancellation_token_short_circuits_long_running_command() {
        use crate::application::command_context::{CancellationToken, OperationId, TraceId};
        let token = CancellationToken::new();
        let context = CommandContext::new(OperationId::new(1), token.clone(), TraceId::new(1));
        let runner = AsyncCommandRunner::new(
            Arc::new(CommandBus::new()),
            Arc::new(ApplicationEventBus::new()),
        );
        let rx = runner.dispatch(CancellableCommand { max_iters: 1_000 }, context);
        // Give the command a moment to start, then cancel.
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        token.cancel();
        let outcome = rx.await.expect("oneshot").expect("ok");
        assert!(
            *outcome.value() < 1_000,
            "expected early termination, got {}",
            outcome.value()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn dispatch_publishes_vm_invalidations_when_bus_attached() {
        use crate::application::events::playlist::PlaylistEvent;
        let vm_bus = VmBus::new();
        let mut vm_rx = vm_bus.subscribe();
        let runner = AsyncCommandRunner::with_vm_bus(
            Arc::new(CommandBus::new()),
            Arc::new(ApplicationEventBus::new()),
            vm_bus,
        );

        let rx = runner.dispatch(
            EchoCommand {
                value: 0,
                emit: vec![
                    ApplicationEvent::Playlist(PlaylistEvent::TracksChanged { playlist_id: 7 }),
                    ApplicationEvent::Playlist(PlaylistEvent::Changed),
                ],
            },
            empty_context(),
        );
        let _ = rx.await.expect("oneshot");

        let first = vm_rx.recv().await.expect("vm event");
        assert!(matches!(first, VmEvent::PlaylistChanged { playlist_id: 7 }));
        let second = vm_rx.recv().await.expect("vm event");
        assert!(matches!(second, VmEvent::InvalidateAll));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn dispatch_maps_metadata_track_tagged_to_track_changed() {
        use crate::application::events::metadata::MetadataEvent;
        let vm_bus = VmBus::new();
        let mut vm_rx = vm_bus.subscribe();
        let runner = AsyncCommandRunner::with_vm_bus(
            Arc::new(CommandBus::new()),
            Arc::new(ApplicationEventBus::new()),
            vm_bus,
        );

        let rx = runner.dispatch(
            EchoCommand {
                value: 0,
                emit: vec![ApplicationEvent::Metadata(MetadataEvent::TrackTagged {
                    track_id: 42,
                })],
            },
            empty_context(),
        );
        let _ = rx.await.expect("oneshot");

        let event = vm_rx.recv().await.expect("vm event");
        assert!(matches!(event, VmEvent::TrackChanged { track_id: 42 }));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn dispatch_without_vm_bus_does_not_panic() {
        // Smoke check the default constructor path still works after the
        // optional vm_bus field was added.
        use crate::application::events::library::LibraryEvent;
        let runner = AsyncCommandRunner::new(
            Arc::new(CommandBus::new()),
            Arc::new(ApplicationEventBus::new()),
        );
        let rx = runner.dispatch(
            EchoCommand {
                value: 1,
                emit: vec![ApplicationEvent::Library(LibraryEvent::Changed)],
            },
            empty_context(),
        );
        let outcome = rx.await.expect("oneshot").expect("ok");
        assert_eq!(*outcome.value(), 1);
    }
}
