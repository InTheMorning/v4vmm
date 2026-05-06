//! UI-agnostic application layer.
//!
//! This module is the ADR 0024 boundary between presentation adapters and the
//! existing service/domain/infrastructure modules.

pub mod application_event_bus;
pub mod application_query_service;
pub mod application_services;
#[cfg(feature = "async-runtime")]
pub mod async_command_runner;
pub mod command_bus;
pub mod command_context;
pub mod commands;
pub mod errors;
pub mod events;
#[cfg(feature = "async-runtime")]
pub mod paged_track_list;
pub mod ports;
pub mod queries;

pub use application_event_bus::{ApplicationEventBus, ApplicationEventSubscriber};
pub use application_query_service::ApplicationQueryService;
pub use application_services::{ApplicationServices, ApplicationServicesBuildError};
pub use command_bus::{ApplicationCommand, CommandBus, CommandOutcome, CommandResult};
pub use command_context::{CancellationToken, CommandContext, OperationId, TraceId};
pub use errors::command::CommandError;
pub use events::ApplicationEvent;
pub use ports::download_manager::{
    DownloadError, DownloadManager, DownloadOutcome, DownloadRequest, ServiceDownloadManager,
    UnavailableDownloadManager,
};
