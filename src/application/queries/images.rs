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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    use reqwest::blocking::Client as ReqwestClient;

    use crate::application::command_bus::CommandBus;
    use crate::application::command_context::{CancellationToken, OperationId, TraceId};

    const TEST_IMAGE_BYTES: &[u8] = include_bytes!("../../assets/music_network_logo.png");

    fn cancelled_context() -> CommandContext {
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        CommandContext::new(OperationId::new(1), cancellation, TraceId::new(1))
    }

    fn serve_image_once(content_type: &'static str, body: &'static [u8]) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("listener addr");
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request);
            let headers = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(headers.as_bytes()).expect("write headers");
            stream.write_all(body).expect("write body");
        });
        format!("http://{addr}/cover.png")
    }

    #[test]
    fn fetch_thumbnail_fetches_image_through_cache() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cache = ImageCache::with_capacity(
            ReqwestClient::new(),
            temp.path().join("thumbnails"),
            2,
            512,
            1024 * 1024,
        );
        let url = serve_image_once("image/png", TEST_IMAGE_BYTES);

        let outcome = CommandBus::new()
            .execute(
                FetchThumbnail::new(Arc::clone(&cache), url.clone(), false),
                &CommandContext::next(),
            )
            .expect("thumbnail fetch succeeds");

        assert!(outcome.value().is_some(), "thumbnail should be fetched");
        assert!(outcome.events().is_empty(), "query should not emit events");
        assert!(
            cache.peek_static(&url).is_some(),
            "thumbnail should be retained by the cache"
        );
    }

    #[test]
    fn download_inspector_image_downloads_uncached_image() {
        let url = serve_image_once("image/png", TEST_IMAGE_BYTES);

        let outcome = CommandBus::new()
            .execute(
                DownloadInspectorImage::new("http://example.test", url),
                &CommandContext::next(),
            )
            .expect("inspector image download succeeds");

        assert!(
            outcome.value().is_some(),
            "inspector image should be downloaded"
        );
        assert!(outcome.events().is_empty(), "query should not emit events");
    }

    #[test]
    fn image_queries_honor_cancelled_context() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cache = ImageCache::with_capacity(
            ReqwestClient::new(),
            temp.path().join("thumbnails"),
            1,
            512,
            1024 * 1024,
        );
        let bus = CommandBus::new();

        let fetch_error = match bus.execute(
            FetchThumbnail::new(cache, "http://127.0.0.1:1/thumbnail.png", false),
            &cancelled_context(),
        ) {
            Ok(_) => panic!("cancelled thumbnail query should fail"),
            Err(error) => error,
        };
        let download_error = match bus.execute(
            DownloadInspectorImage::new("http://example.test", "http://127.0.0.1:1/inspector.png"),
            &cancelled_context(),
        ) {
            Ok(_) => panic!("cancelled inspector query should fail"),
            Err(error) => error,
        };

        assert_eq!(fetch_error, CommandError::Cancelled);
        assert_eq!(download_error, CommandError::Cancelled);
    }
}
