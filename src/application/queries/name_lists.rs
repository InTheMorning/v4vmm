//! The `MusicIndex` name list queries of the scope "Everything" (ADR 0084
//! Decision 8, packet 016).
//!
//! Each command sends one request for one page of a name list. It reads no
//! release detail for a row (ADR 0075 section 6, packet 016 decision D16).
//! `FetchReleaseNamePage` sends `GET /v1/feeds/by-name`, and
//! `FetchArtistNamePage` sends `GET /v1/artist-names`.

#![warn(clippy::pedantic)]

use crate::api::{ArtistNameRow, Client, Feed};
use crate::application::command_bus::{ApplicationCommand, CommandOutcome, CommandResult};
use crate::application::command_context::CommandContext;
use crate::application::errors::command::CommandError;
use crate::application::request_profiles::{INDEX_ARTIST_NAME_LIST, INDEX_RELEASE_NAME_LIST};
use crate::view_models::index_name_list::IndexNameListBatch;

/// The row count of one name list page (packet 016 decision D7).
pub(crate) const NAME_LIST_PAGE_LIMIT: i32 = 100;

/// Fetches one page of `MusicIndex` music releases in name order.
#[derive(Clone, Debug)]
pub(crate) struct FetchReleaseNamePage {
    endpoint: crate::config::MusicIndexEndpoint,
    cursor: Option<String>,
}

impl FetchReleaseNamePage {
    /// Creates the command for the page at `cursor`. `None` asks for page 1.
    #[must_use]
    pub(crate) fn new(
        endpoint: impl Into<crate::config::MusicIndexEndpoint>,
        cursor: Option<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            cursor,
        }
    }
}

impl ApplicationCommand for FetchReleaseNamePage {
    type Output = IndexNameListBatch<Feed>;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let response = Client::new_with_base_url(self.endpoint)
            .fetch_feeds_by_name(
                NAME_LIST_PAGE_LIMIT,
                self.cursor.as_deref(),
                None,
                &INDEX_RELEASE_NAME_LIST,
            )
            .map_err(|error| CommandError::Query(format!("{error:#}")))?;
        Ok(CommandOutcome::without_events(IndexNameListBatch {
            rows: response.data,
            cursor: response.pagination.cursor,
            has_more: response.pagination.has_more,
        }))
    }
}

/// Fetches one page of `MusicIndex` artist names in name order.
#[derive(Clone, Debug)]
pub(crate) struct FetchArtistNamePage {
    endpoint: crate::config::MusicIndexEndpoint,
    cursor: Option<String>,
}

impl FetchArtistNamePage {
    /// Creates the command for the page at `cursor`. `None` asks for page 1.
    #[must_use]
    pub(crate) fn new(
        endpoint: impl Into<crate::config::MusicIndexEndpoint>,
        cursor: Option<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            cursor,
        }
    }
}

impl ApplicationCommand for FetchArtistNamePage {
    type Output = IndexNameListBatch<ArtistNameRow>;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let response = Client::new_with_base_url(self.endpoint)
            .fetch_artist_names(
                NAME_LIST_PAGE_LIMIT,
                self.cursor.as_deref(),
                None,
                &INDEX_ARTIST_NAME_LIST,
            )
            .map_err(|error| CommandError::Query(format!("{error:#}")))?;
        Ok(CommandOutcome::without_events(IndexNameListBatch {
            rows: response.data,
            cursor: response.pagination.cursor,
            has_more: response.pagination.has_more,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    /// A local HTTP server that answers each request with `body` and
    /// records each request path.
    fn serve(body: String, expected_requests: usize) -> (String, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let received = Arc::clone(&requests);
        std::thread::spawn(move || {
            for _ in 0..expected_requests {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0; 4096];
                while !bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                    match stream.read(&mut buffer) {
                        Ok(0) | Err(_) => break,
                        Ok(read) => bytes.extend_from_slice(&buffer[..read]),
                    }
                }
                let request = String::from_utf8_lossy(&bytes);
                if let Some(path) = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                {
                    received.lock().unwrap().push(path.to_owned());
                }
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
            }
        });
        (format!("http://{address}"), requests)
    }

    /// R16-05: the artist name command sends one request for a page, with
    /// `limit=100`, the profile include and the cursor.
    #[test]
    fn adr_0084_scopes_r16_05_artist_page_sends_one_request() {
        let body = serde_json::json!({
            "data": [{"kind": "name", "name": "$2 Holla", "sort_name": "$2 holla",
                      "key": "$2 holla", "feed_guid": null, "image_url": null,
                      "release_count": 3}],
            "pagination": {"cursor": "next-cursor", "has_more": true}
        })
        .to_string();
        let (endpoint, requests) = serve(body, 1);

        let batch = FetchArtistNamePage::new(endpoint, Some("abc".to_owned()))
            .execute(&CommandContext::next())
            .unwrap()
            .into_parts()
            .0;

        assert_eq!(batch.rows.len(), 1);
        assert_eq!(batch.cursor.as_deref(), Some("next-cursor"));
        assert!(batch.has_more);
        assert_eq!(
            *requests.lock().unwrap(),
            ["/v1/artist-names?limit=100&cursor=abc&include=link_facts"]
        );
    }

    /// R16-05: the release name command sends one request for a page, with
    /// `limit=100`, the profile include and the cursor. It reads no release
    /// detail for a row.
    #[test]
    fn adr_0084_scopes_r16_05_release_page_sends_one_request() {
        let body = serde_json::json!({
            "data": [
                {"feed_guid": "motive-album", "title": "Motive to Kill",
                 "sort_name": "motive to kill", "release_kind": "unknown",
                 "episode_count": 1, "publisher_feed_guid": "motive-guid"},
                {"feed_guid": "liii-album", "title": "#LIII4PRESIDENT",
                 "sort_name": "#liii4president", "release_artist": "LIIINEKELA",
                 "publisher_feed_guid": null}
            ],
            "pagination": {"cursor": "next-cursor", "has_more": true}
        })
        .to_string();
        let (endpoint, requests) = serve(body, 1);

        let batch = FetchReleaseNamePage::new(endpoint, None)
            .execute(&CommandContext::next())
            .unwrap()
            .into_parts()
            .0;

        assert_eq!(batch.rows.len(), 2);
        assert_eq!(batch.cursor.as_deref(), Some("next-cursor"));
        assert!(batch.has_more);
        assert_eq!(
            *requests.lock().unwrap(),
            ["/v1/feeds/by-name?limit=100&include=named_feeds"]
        );
    }
}
