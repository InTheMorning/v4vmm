//! Transport policy for remote media fetches.
//!
//! ADR 0056. Every remote media fetch in the application resolves redirects,
//! rejects unsupported URL schemes, and rejects non-success statuses here. The
//! rules used to live at each call site, which is how one of five media fetches
//! shipped without redirect handling while two others were being fixed in the
//! same file.
//!
//! This module owns transport only. What the bytes have to *be* -- a supported
//! audio container, an image, non-markup transcript text -- stays with the
//! artifact owner, because those rules genuinely differ per artifact.
//!
//! Feed and API fetches (`rss`, `musicbrainz`, `api`, `discover`) are out of
//! scope: they fetch documents, not media bytes.

use std::sync::OnceLock;

use anyhow::{anyhow, Context, Result};
use reqwest::blocking::{Client, Response};
use reqwest::header::LOCATION;
use reqwest::Url;

/// Maximum redirect hops for any media fetch.
const MAX_REDIRECTS: usize = 10;

/// Shared client for media fetches.
///
/// Redirects are disabled so this module is the only thing that resolves them.
/// A client that follows redirects itself stops following when a `Location`
/// value will not parse and hands back the 3xx response, and a 3xx status is not
/// an error status -- that is precisely how redirect bodies became local files.
fn client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        // No fallback client. A default `Client::new()` follows redirects, which
        // is the behavior this module exists to take ownership of; silently
        // substituting it would reintroduce the defect under a different name.
        crate::http_client::media_builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("build redirect-disabled media client")
    })
}

/// Fetch remote media, following redirects to a final success response.
///
/// `what` names the artifact in error messages, e.g. `"enclosure"`.
/// The response body is not read here, so callers can stream it.
pub fn fetch(url: &str, what: &str) -> Result<Response> {
    let mut parsed = parse_url(url, what)?;
    let mut redirects_remaining = MAX_REDIRECTS;

    loop {
        let response = client()
            .get(parsed.clone())
            .send()
            .with_context(|| format!("download {what} {parsed}"))?;

        if response.status().is_redirection() {
            if redirects_remaining == 0 {
                return Err(anyhow!("too many {what} redirects for {url}"));
            }
            redirects_remaining -= 1;
            parsed = redirect_url(&parsed, &response, what)
                .with_context(|| format!("follow {what} redirect from {parsed}"))?;
            continue;
        }

        if !response.status().is_success() {
            return Err(anyhow!(
                "download {what} {parsed} failed with HTTP {}",
                response.status()
            ));
        }

        return Ok(response);
    }
}

/// Declared content type of a response, lowercased with parameters stripped.
pub fn declared_content_type(response: &Response) -> Option<String> {
    response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
}

fn parse_url(url: &str, what: &str) -> Result<Url> {
    let parsed = Url::parse(url).with_context(|| format!("parse {what} URL {url}"))?;
    match parsed.scheme() {
        "http" | "https" => Ok(parsed),
        scheme => Err(anyhow!("unsupported {what} URL scheme: {scheme}")),
    }
}

fn redirect_url(base: &Url, response: &Response, what: &str) -> Result<Url> {
    let location = response
        .headers()
        .get(LOCATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| anyhow!("{what} redirect missing Location header"))?;
    base.join(location)
        .with_context(|| format!("parse {what} redirect Location {location:?}"))
        .and_then(|url| parse_url(url.as_str(), what))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the behavior the removed space-repair fallback used to guard.
    ///
    /// The URL parser percent-encodes raw spaces in a path, so the real
    /// `Location: http://host/Music/song file.mp3` values that motivated
    /// ADR 0056 resolve without repair. Kept as a test rather than as dead
    /// defensive code.
    #[test]
    fn location_with_raw_spaces_resolves_without_repair() {
        let base = Url::parse("http://example.test/song.mp3").expect("parse base");
        let resolved = base
            .join("http://www.example.test/Music/song file.mp3")
            .expect("join Location containing raw spaces");

        assert_eq!(
            resolved.as_str(),
            "http://www.example.test/Music/song%20file.mp3"
        );
    }

    #[test]
    fn rejects_unsupported_scheme() {
        let error = fetch("ftp://example.test/song.mp3", "enclosure")
            .expect_err("ftp is not a supported media scheme");

        assert!(
            error
                .to_string()
                .contains("unsupported enclosure URL scheme"),
            "error should name the scheme: {error}"
        );
    }
}
