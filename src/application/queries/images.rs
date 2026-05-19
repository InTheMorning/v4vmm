//! Presentation image query family.

use std::fmt;
use std::sync::Arc;

use crate::api::Client;
use crate::application::command_bus::{ApplicationCommand, CommandOutcome, CommandResult};
use crate::application::command_context::CommandContext;
use crate::application::errors::command::CommandError;
use crate::media::{image_from_bytes, CachedImage, ImageCache};
use crate::subscribe_service::download_image;

/// Fetches one cached thumbnail for presentation.
#[derive(Clone)]
pub(crate) struct FetchThumbnail {
    cache: Arc<ImageCache>,
    url: String,
    animated: bool,
}

impl FetchThumbnail {
    /// Creates a thumbnail fetch query command.
    #[must_use]
    pub(crate) fn new(cache: Arc<ImageCache>, url: impl Into<String>, animated: bool) -> Self {
        Self {
            cache,
            url: url.into(),
            animated,
        }
    }
}

impl fmt::Debug for FetchThumbnail {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FetchThumbnail")
            .field("url", &self.url)
            .field("animated", &self.animated)
            .finish_non_exhaustive()
    }
}

impl ApplicationCommand for FetchThumbnail {
    type Output = Option<CachedImage>;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let image = if self.animated {
            self.cache.fetch_blocking(&self.url)
        } else {
            self.cache.fetch_static_blocking(&self.url)
        };
        Ok(CommandOutcome::without_events(image))
    }
}

/// Downloads and decodes one uncached inspector image.
#[derive(Clone, Debug)]
pub(crate) struct DownloadInspectorImage {
    endpoint: String,
    url: String,
}

impl DownloadInspectorImage {
    /// Creates an inspector image download query command.
    #[must_use]
    pub(crate) fn new(endpoint: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            url: url.into(),
        }
    }
}

impl ApplicationCommand for DownloadInspectorImage {
    type Output = Option<CachedImage>;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let client = Client::new_with_base_url(self.endpoint);
        let image = download_image(&client.client, &self.url).map(image_from_bytes);
        Ok(CommandOutcome::without_events(image))
    }
}
