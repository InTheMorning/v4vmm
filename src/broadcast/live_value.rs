#![warn(clippy::pedantic)]

//! The `podcast:liveValue` URI of a broadcast event (ADR 0059, amendment of 2026-10-24).
//!
//! ADR 0059 task 019 owns this module. The URI names the relay that holds the
//! event, so a listener app can connect to it. The relay registers the
//! Socket.IO namespace `/event` at its root, thus the URI keeps only the scheme,
//! the host and the port of the stored endpoint. A feed is public, thus the URI
//! never holds the user information of the endpoint.

use std::fmt;

use reqwest::Url;

/// The Socket.IO namespace that the relay registers at its root.
const EVENT_PATH: &str = "/event";
/// The query key that names the event at the relay.
const EVENT_ID_KEY: &str = "event_id";

/// Why the app cannot make a liveValue URI from a stored endpoint and an event identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LiveValueUriError {
    /// The endpoint has no host.
    NoHost,
    /// The endpoint scheme is not `http` or `https`.
    UnsupportedScheme,
    /// The endpoint is not an absolute URL.
    InvalidEndpoint,
    /// The event identifier is empty.
    EmptyEventId,
}

impl fmt::Display for LiveValueUriError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NoHost => "relay endpoint has no host",
            Self::UnsupportedScheme => "relay endpoint scheme is not http or https",
            Self::InvalidEndpoint => "relay endpoint is not an absolute URL",
            Self::EmptyEventId => "event identifier is empty",
        })
    }
}

impl std::error::Error for LiveValueUriError {}

/// Make the listener URI of one event at the relay of its stored endpoint.
///
/// The result is `<scheme>://<host>[:<port>]/event?event_id=<EVENT_ID>`. The
/// function removes the user information, the path, the query and the fragment
/// of the endpoint. It encodes the event identifier as one query value. It does
/// no I/O.
///
/// # Errors
///
/// Returns a [`LiveValueUriError`] when the endpoint is not an `http` or
/// `https` URL with a host, or when the event identifier is empty.
pub(crate) fn live_value_uri(endpoint: &str, event_id: &str) -> Result<String, LiveValueUriError> {
    if event_id.trim().is_empty() {
        return Err(LiveValueUriError::EmptyEventId);
    }
    let endpoint = endpoint.trim();
    let mut url = Url::parse(endpoint).map_err(|_| {
        if has_empty_host(endpoint) {
            LiveValueUriError::NoHost
        } else {
            LiveValueUriError::InvalidEndpoint
        }
    })?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(LiveValueUriError::UnsupportedScheme);
    }
    if url.host_str().is_none_or(str::is_empty) {
        return Err(LiveValueUriError::NoHost);
    }
    // An http or https URL with a host always accepts these changes.
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_path(EVENT_PATH);
    url.set_query(None);
    url.set_fragment(None);
    url.query_pairs_mut().append_pair(EVENT_ID_KEY, event_id);
    Ok(url.into())
}

/// Whether the authority of an endpoint that did not parse has no host.
///
/// `reqwest` does not export the parse error type, thus this function examines
/// the text between `://` and the first `/`, `?` or `#`.
fn has_empty_host(endpoint: &str) -> bool {
    endpoint.split_once("://").is_some_and(|(_, rest)| {
        let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
        let host = authority
            .rsplit_once('@')
            .map_or(authority, |(_, host)| host);
        host.split(':').next().unwrap_or_default().is_empty()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R19-01: the URI names the relay host, the `/event` path and the identifier.
    #[test]
    fn adr_0059_task_019_https_endpoint_gives_event_url() {
        assert_eq!(
            live_value_uri("https://api.musicindex.org", "abc_DEF-123").as_deref(),
            Ok("https://api.musicindex.org/event?event_id=abc_DEF-123")
        );
    }

    /// R19-02: the scheme and the port of the endpoint stay.
    #[test]
    fn adr_0059_task_019_http_endpoint_keeps_scheme_and_port() {
        assert_eq!(
            live_value_uri("http://127.0.0.1:17863", "fixture-event-2").as_deref(),
            Ok("http://127.0.0.1:17863/event?event_id=fixture-event-2")
        );
    }

    /// R19-03: the URI drops the user information, the path, the query and the fragment.
    #[test]
    fn adr_0059_task_019_endpoint_user_path_query_and_fragment_are_removed() {
        let uri = live_value_uri("https://user:secret@relay.example:8443/api/v1?x=1#f", "e1")
            .expect("valid endpoint");
        assert_eq!(uri, "https://relay.example:8443/event?event_id=e1");
        for fragment in ["user", "secret", "/api", "x=1"] {
            assert!(!uri.contains(fragment), "{uri} contains {fragment}");
        }
    }

    /// R19-04: the identifier is one encoded query value.
    #[test]
    fn adr_0059_task_019_event_id_is_one_encoded_query_value() {
        let event_id = "a&b=c d#\"<>";
        let uri = live_value_uri("https://relay.example", event_id).expect("valid endpoint");
        let (_, query) = uri.split_once('?').expect("query");
        for character in ['&', ' ', '#', '"', '<', '>'] {
            assert!(!uri.contains(character), "{uri} contains {character:?}");
        }
        assert_eq!(query.matches('=').count(), 1);
        let pairs: Vec<(String, String)> = Url::parse(&uri)
            .expect("parsed uri")
            .query_pairs()
            .into_owned()
            .collect();
        assert_eq!(pairs, vec![("event_id".to_owned(), event_id.to_owned())]);
    }

    /// R19-05: each refused input gives its typed error value.
    #[test]
    fn adr_0059_task_019_refused_inputs_give_typed_errors() {
        assert_eq!(
            live_value_uri("ftp://relay.example", "e1"),
            Err(LiveValueUriError::UnsupportedScheme)
        );
        assert_eq!(
            live_value_uri("https://", "e1"),
            Err(LiveValueUriError::NoHost)
        );
        assert_eq!(
            live_value_uri("not a url", "e1"),
            Err(LiveValueUriError::InvalidEndpoint)
        );
        assert_eq!(
            live_value_uri("https://relay.example", ""),
            Err(LiveValueUriError::EmptyEventId)
        );
    }
}
