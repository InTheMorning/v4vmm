//! Root application service wiring.

use std::fmt;
use std::sync::Arc;

use crate::application::application_event_bus::ApplicationEventBus;
use crate::application::application_query_service::ApplicationQueryService;
use crate::application::command_bus::CommandBus;
use crate::application::ports::download_manager::{DownloadManager, ServiceDownloadManager};

/// Explicit root wiring for application-layer dependencies.
#[derive(Clone, Debug)]
pub struct ApplicationServices {
    command_bus: Arc<CommandBus>,
    query_service: Arc<ApplicationQueryService>,
    event_bus: Arc<ApplicationEventBus>,
    download_manager: Arc<dyn DownloadManager>,
}

impl ApplicationServices {
    /// Builds local application services with the current service adapters.
    ///
    /// # Errors
    ///
    /// Returns an error if the root service graph is incomplete.
    pub fn local_with_service_adapters() -> Result<Self, ApplicationServicesBuildError> {
        Self::builder()
            .command_bus(Arc::new(CommandBus::new()))
            .query_service(Arc::new(ApplicationQueryService::new()))
            .event_bus(Arc::new(ApplicationEventBus::new()))
            .download_manager(Arc::new(ServiceDownloadManager::new()))
            .build()
    }

    /// Starts building application service wiring.
    #[must_use]
    pub const fn builder() -> ApplicationServicesBuilder {
        ApplicationServicesBuilder::new()
    }

    /// Returns the shared command bus.
    #[must_use]
    pub fn command_bus(&self) -> Arc<CommandBus> {
        Arc::clone(&self.command_bus)
    }

    /// Returns the local query service.
    #[must_use]
    pub fn query_service(&self) -> Arc<ApplicationQueryService> {
        Arc::clone(&self.query_service)
    }

    /// Returns the app-scoped event bus.
    #[must_use]
    pub fn event_bus(&self) -> Arc<ApplicationEventBus> {
        Arc::clone(&self.event_bus)
    }

    /// Returns the configured download manager port.
    #[must_use]
    pub fn download_manager(&self) -> Arc<dyn DownloadManager> {
        Arc::clone(&self.download_manager)
    }
}

/// Builder for application service wiring.
#[derive(Clone, Default)]
pub struct ApplicationServicesBuilder {
    command_bus: Option<Arc<CommandBus>>,
    query_service: Option<Arc<ApplicationQueryService>>,
    event_bus: Option<Arc<ApplicationEventBus>>,
    download_manager: Option<Arc<dyn DownloadManager>>,
}

impl ApplicationServicesBuilder {
    /// Creates an empty builder.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            command_bus: None,
            query_service: None,
            event_bus: None,
            download_manager: None,
        }
    }

    /// Sets the command bus.
    #[must_use]
    pub fn command_bus(mut self, command_bus: Arc<CommandBus>) -> Self {
        self.command_bus = Some(command_bus);
        self
    }

    /// Sets the local query service.
    #[must_use]
    pub fn query_service(mut self, query_service: Arc<ApplicationQueryService>) -> Self {
        self.query_service = Some(query_service);
        self
    }

    /// Sets the app-scoped event bus.
    #[must_use]
    pub fn event_bus(mut self, event_bus: Arc<ApplicationEventBus>) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    /// Sets the download manager port.
    #[must_use]
    pub fn download_manager(mut self, download_manager: Arc<dyn DownloadManager>) -> Self {
        self.download_manager = Some(download_manager);
        self
    }

    /// Builds application service wiring.
    ///
    /// # Errors
    ///
    /// Returns an error when a required dependency is missing.
    pub fn build(self) -> Result<ApplicationServices, ApplicationServicesBuildError> {
        Ok(ApplicationServices {
            command_bus: self
                .command_bus
                .ok_or(ApplicationServicesBuildError::MissingCommandBus)?,
            query_service: self
                .query_service
                .ok_or(ApplicationServicesBuildError::MissingQueryService)?,
            event_bus: self
                .event_bus
                .ok_or(ApplicationServicesBuildError::MissingEventBus)?,
            download_manager: self
                .download_manager
                .ok_or(ApplicationServicesBuildError::MissingDownloadManager)?,
        })
    }
}

impl fmt::Debug for ApplicationServicesBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApplicationServicesBuilder")
            .field("has_command_bus", &self.command_bus.is_some())
            .field("has_query_service", &self.query_service.is_some())
            .field("has_event_bus", &self.event_bus.is_some())
            .field("has_download_manager", &self.download_manager.is_some())
            .finish()
    }
}

/// Error returned when application service wiring is incomplete.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationServicesBuildError {
    /// Command bus was not provided.
    MissingCommandBus,
    /// Query service was not provided.
    MissingQueryService,
    /// Event bus was not provided.
    MissingEventBus,
    /// Download manager port was not provided.
    MissingDownloadManager,
}

impl fmt::Display for ApplicationServicesBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCommandBus => f.write_str("missing command bus"),
            Self::MissingQueryService => f.write_str("missing query service"),
            Self::MissingEventBus => f.write_str("missing event bus"),
            Self::MissingDownloadManager => f.write_str("missing download manager"),
        }
    }
}

impl std::error::Error for ApplicationServicesBuildError {}
