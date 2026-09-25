//! Original HTTP entity capture and decoder inputs for ADR 0075.

use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use chrono::Utc;
use reqwest::blocking::RequestBuilder;
use serde_json::{json, Map, Value};

use super::{ObservationOutcome, ProviderObservation};

const HEADERS: &[&str] = &[
    "content-type",
    "content-encoding",
    "content-length",
    "content-range",
    "content-location",
    "location",
    "date",
    "age",
    "cache-control",
    "expires",
    "etag",
    "last-modified",
    "vary",
    "warning",
    "retry-after",
];

pub(crate) fn capture(request: RequestBuilder, decoder: &str) -> ProviderObservation {
    capture_with_timeout(request, decoder, crate::http_client::document_timeout())
}
/// Capture a conditional request of the playlist RSS check (ADR 0076).
///
/// A `304 Not Modified` response is not a failure. The observation keeps
/// `http_status` 304 and the response headers, with outcome `success`, no
/// body and the body state `absent`. Each other status keeps the rule of
/// `capture`.
pub(crate) fn capture_conditional(request: RequestBuilder, decoder: &str) -> ProviderObservation {
    not_modified_is_success(capture(request, decoder))
}

/// Build the conditional GET of the playlist RSS check (ADR 0076).
///
/// The request carries `If-None-Match` for a stored `ETag` and
/// `If-Modified-Since` for a stored `Last-Modified`. Without a stored
/// validator it carries neither header.
pub(crate) fn conditional_request(
    client: &reqwest::blocking::Client,
    url: &str,
    validators: Option<&super::RequestValidators>,
) -> RequestBuilder {
    let mut request = client.get(url);
    if let Some(validators) = validators {
        if let Some(etag) = validators.etag.as_deref() {
            request = request.header(reqwest::header::IF_NONE_MATCH, etag);
        }
        if let Some(last_modified) = validators.last_modified.as_deref() {
            request = request.header(reqwest::header::IF_MODIFIED_SINCE, last_modified);
        }
    }
    request
}

fn not_modified_is_success(mut observation: ProviderObservation) -> ProviderObservation {
    if observation.http_status == Some(304)
        && observation.failure == Some(json!({"reason": "http_status"}))
    {
        observation.outcome = ObservationOutcome::Success;
        observation.failure = None;
        observation.body = None;
        observation.interpretation["body_state"] = json!("absent");
    }
    observation
}

fn capture_with_timeout(
    request: RequestBuilder,
    decoder: &str,
    timeout: Duration,
) -> ProviderObservation {
    let mut observation = ProviderObservation {
        body: None,
        http_status: None,
        response_uri: None,
        interpretation: json!({"version":1,"media_type":null,"charset":null,"retained_body_codings":[],"body_state":"absent","effective_base_uri":null}),
        source_revision: None,
        source_times: json!({}),
        decoder_version: decoder.into(),
        outcome: ObservationOutcome::Success,
        failure: None,
        finished_at_us: 0,
        fetched_at_us: None,
        occurrence: json!({"version":1,"headers":{}}),
        coverage: Vec::new(),
    };
    match request.timeout(timeout).send() {
        Err(_) => observation.fail("transport"),
        Ok(mut response) => {
            observation.http_status = Some(response.status().as_u16());
            observation.response_uri = Some(response.url().to_string());
            let mut headers = Map::new();
            for name in HEADERS {
                let values = response
                    .headers()
                    .get_all(*name)
                    .iter()
                    .map(|value| {
                        Value::String(
                            base64::engine::general_purpose::STANDARD.encode(value.as_bytes()),
                        )
                    })
                    .collect::<Vec<_>>();
                if !values.is_empty() {
                    headers.insert((*name).into(), Value::Array(values));
                }
            }
            observation.occurrence["headers"] = Value::Object(headers);
            let mime = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<mime::Mime>().ok());
            observation.interpretation["media_type"] =
                json!(mime.as_ref().map(|m| m.essence_str().to_ascii_lowercase()));
            // Reqwest removes content-encoding when it decodes that entity body.
            observation.interpretation["retained_body_codings"] = json!(response
                .headers()
                .get_all("content-encoding")
                .iter()
                .filter_map(|v| v.to_str().ok())
                .flat_map(|s| s.split(','))
                .map(|s| s.trim().to_ascii_lowercase())
                .collect::<Vec<_>>());
            let mut bytes = Vec::new();
            match response.read_to_end(&mut bytes) {
                Ok(_) => {
                    observation.fetched_at_us = Some(Utc::now().timestamp_micros());
                    observation.interpretation["body_state"] = json!("complete");
                }
                Err(_) => {
                    observation.fail("body_read");
                    observation.interpretation["body_state"] = json!("truncated");
                }
            }
            observation.body = Some(Arc::from(bytes));
            if observation.outcome != ObservationOutcome::Failed && !response.status().is_success()
            {
                observation.fail("http_status");
            }
        }
    }
    observation.finished_at_us = Utc::now().timestamp_micros();
    observation
}

/// This follows locked reqwest 0.13.2 `text_with_charset("utf-8")`.
pub(crate) fn decode_text(observation: &mut ProviderObservation) -> String {
    if observation.body.is_none() {
        return String::new();
    }
    let content_type = observation.occurrence["headers"]["content-type"]
        .get(0)
        .and_then(Value::as_str)
        .and_then(|s| base64::engine::general_purpose::STANDARD.decode(s).ok());
    let mime = content_type
        .as_deref()
        .and_then(|s| std::str::from_utf8(s).ok())
        .and_then(|s| s.parse::<mime::Mime>().ok());
    let encoding_name = mime
        .as_ref()
        .and_then(|mime| mime.get_param("charset").map(|c| c.as_str()))
        .unwrap_or("utf-8");
    let encoding =
        encoding_rs::Encoding::for_label(encoding_name.as_bytes()).unwrap_or(encoding_rs::UTF_8);
    let (text, actual_encoding, _) =
        encoding.decode(observation.body.as_deref().unwrap_or_default());
    observation.interpretation["charset"] = json!(actual_encoding.name());
    text.into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;
    use std::time::Instant;

    fn server(response: Vec<u8>, count: usize) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/private", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            for _ in 0..count {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0; 4096];
                stream.read(&mut request).unwrap();
                stream.write_all(&response).unwrap();
            }
        });
        (url, handle)
    }
    #[test]
    fn adr_0076_playlist_check_not_modified_is_a_success_without_body() {
        let response =
            b"HTTP/1.1 304 Not Modified\r\nETag: \"v2\"\r\nConnection: close\r\n\r\n".to_vec();
        let (url, worker) = server(response, 1);
        let observation = capture_conditional(crate::http_client::document().get(url), "test-v1");
        worker.join().unwrap();
        assert_eq!(observation.http_status, Some(304));
        assert_eq!(observation.outcome, ObservationOutcome::Success);
        assert_eq!(observation.failure, None);
        assert_eq!(observation.body, None);
        assert_eq!(observation.interpretation["body_state"], "absent");
        assert!(observation.occurrence["headers"].get("etag").is_some());
        let response =
            b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                .to_vec();
        let (url, worker) = server(response, 1);
        let failed = capture_conditional(crate::http_client::document().get(url), "test-v1");
        worker.join().unwrap();
        assert_eq!(failed.outcome, ObservationOutcome::Failed);
    }
    #[test]
    fn adr_0075_observation_charset_matches_locked_reqwest_text_decoder() {
        for (content_type, bytes) in [
            (
                "application/json; charset=windows-1252",
                b"caf\xe9".as_slice(),
            ),
            ("application/json; charset=UTF-8", b"a\xffz".as_slice()),
            (
                "application/json; charset=latin1",
                b"\xef\xbb\xbfhello".as_slice(),
            ),
            ("application/json; charset=unknown", b"a\xc3\xa9".as_slice()),
        ] {
            let mut response=format!("HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",bytes.len()).into_bytes();
            response.extend(bytes);
            let (url, worker) = server(response, 2);
            let client = crate::http_client::document();
            let expected = client.get(&url).send().unwrap().text().unwrap();
            let mut observation = capture(client.get(&url), "characterization-v1");
            assert_eq!(decode_text(&mut observation), expected);
            assert_eq!(observation.body.as_deref().unwrap(), bytes);
            worker.join().unwrap();
        }
    }
    #[test]
    fn adr_0075_observation_partial_body_and_continuous_stream_share_total_deadline() {
        let (url, worker) = server(
            b"HTTP/1.1 200 OK\r\nContent-Length: 999\r\nConnection: close\r\n\r\nprefix".to_vec(),
            1,
        );
        let observation = capture(crate::http_client::document().get(url), "test-v1");
        worker.join().unwrap();
        assert_eq!(observation.body.as_deref().unwrap(), b"prefix");
        assert_eq!(observation.interpretation["body_state"], "truncated");
        assert_eq!(observation.fetched_at_us, None);
        assert_eq!(observation.outcome, ObservationOutcome::Failed);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/slow", listener.local_addr().unwrap());
        let worker = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 4096];
            stream.read(&mut request).unwrap();
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1000\r\n\r\n")
                .unwrap();
            for _ in 0..40 {
                if stream.write_all(b"x").is_err() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
        });
        let start = Instant::now();
        let observation = capture_with_timeout(
            crate::http_client::document().get(url),
            "test-v1",
            Duration::from_millis(200),
        );
        assert!(start.elapsed() < Duration::from_millis(650));
        assert_eq!(observation.interpretation["body_state"], "truncated");
        assert!(!observation.body.unwrap().is_empty());
        worker.join().unwrap();
    }
    #[test]
    fn adr_0075_observation_allowed_header_bytes_and_no_response_are_distinct() {
        let response=b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nETag: \r\nETag: private\r\nSet-Cookie: hidden\r\nAuthorization: hidden\r\nConnection: close\r\n\r\n".to_vec();
        let (url, worker) = server(response, 1);
        let observation = capture(crate::http_client::document().get(url), "test-v1");
        worker.join().unwrap();
        assert_eq!(observation.body.as_deref(), Some(b"".as_slice()));
        assert!(observation.fetched_at_us.is_some());
        assert_eq!(
            observation.occurrence["headers"]["etag"],
            json!(["", "cHJpdmF0ZQ=="])
        );
        assert!(observation.occurrence["headers"]
            .get("authorization")
            .is_none());
        assert!(observation.occurrence["headers"]
            .get("set-cookie")
            .is_none());
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        drop(listener);
        let absent = capture(crate::http_client::document().get(url), "test-v1");
        assert_eq!(absent.body, None);
        assert_eq!(absent.http_status, None);
        assert_eq!(absent.response_uri, None);
        assert_eq!(absent.fetched_at_us, None);
        assert_eq!(absent.interpretation["body_state"], "absent");
    }
}
