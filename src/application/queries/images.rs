//! Presentation image query family.

use std::fmt;
use std::sync::Arc;

use crate::application::command_bus::{ApplicationCommand, CommandOutcome, CommandResult};
use crate::application::command_context::CommandContext;
use crate::application::errors::command::CommandError;
use crate::media::cover_color::CoverColor;
use crate::media::{CachedImage, ImageCache};

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

/// Fetches the main color of one cover for a page header backdrop (ADR
/// 0083 task 005).
#[derive(Clone)]
pub(crate) struct FetchCoverColor {
    cache: Arc<ImageCache>,
    url: String,
}

impl FetchCoverColor {
    /// Creates a cover color query command.
    #[must_use]
    pub(crate) fn new(cache: Arc<ImageCache>, url: impl Into<String>) -> Self {
        Self {
            cache,
            url: url.into(),
        }
    }
}

impl fmt::Debug for FetchCoverColor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FetchCoverColor")
            .field("url", &self.url)
            .finish_non_exhaustive()
    }
}

impl ApplicationCommand for FetchCoverColor {
    type Output = Option<CoverColor>;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        Ok(CommandOutcome::without_events(
            self.cache.fetch_cover_color_blocking(&self.url),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

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
        let cache = ImageCache::with_capacity(temp.path().join("thumbnails"), 2, 512, 1024 * 1024);
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
    fn fetch_cover_color_computes_and_keeps_the_color() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cache = ImageCache::with_capacity(temp.path().join("thumbnails"), 2, 512, 1024 * 1024);
        let url = serve_image_once("image/png", TEST_IMAGE_BYTES);

        let outcome = CommandBus::new()
            .execute(
                FetchCoverColor::new(Arc::clone(&cache), url.clone()),
                &CommandContext::next(),
            )
            .expect("cover color fetch succeeds");

        assert!(outcome.value().is_some(), "the cover should have a color");
        assert_eq!(
            cache.peek_cover_color(&url),
            *outcome.value(),
            "the cache should keep the color beside the image"
        );
    }

    #[test]
    fn image_queries_honor_cancelled_context() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cache = ImageCache::with_capacity(temp.path().join("thumbnails"), 1, 512, 1024 * 1024);
        let bus = CommandBus::new();

        let fetch_error = match bus.execute(
            FetchThumbnail::new(cache, "http://127.0.0.1:1/thumbnail.png", false),
            &cancelled_context(),
        ) {
            Ok(_) => panic!("cancelled thumbnail query should fail"),
            Err(error) => error,
        };

        assert_eq!(fetch_error, CommandError::Cancelled);
    }
}
