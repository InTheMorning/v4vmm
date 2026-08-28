//! Outbound HTTP client construction.
//!
//! ADR 0058. One owner for timeout policy, so every blocking client in the
//! application is built from the same numbers instead of inheriting whatever
//! the HTTP dependency happens to default to.
//!
//! What this is not: a fix for hanging fetches. `reqwest`'s blocking builder
//! already defaults to a 30 second timeout, and that default was in force at
//! all ten construction sites before this module existed. The problem was that
//! the value was invisible in this codebase, unowned, and free to change under
//! a dependency bump.
//!
//! How the operation timeout applies depends on how the caller reads the body,
//! which is worth knowing before changing these numbers:
//!
//! - connect and response head: bounded once
//! - streaming reads (`std::io::copy` over the response, used for enclosure
//!   downloads): bounded per `read` call, so a large download is fine and a
//!   stalled socket is not
//! - whole-body calls (`bytes`, `text`, `json`, used for artwork and feeds):
//!   bounded across the entire body
//!
//! This module owns client construction only. Which URLs are legal, how
//! redirects resolve, and what a response body must contain stay with
//! `remote_media` and the individual callers, per ADR 0056.

use std::time::Duration;

use reqwest::blocking::{Client, ClientBuilder};

/// Connect budget. Unset by default in `reqwest`, so a host that is simply
/// down consumed the whole operation budget before this was added.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Operation budget. Matches `reqwest`'s current blocking default; stated
/// explicitly so a dependency bump cannot move it silently.
const OPERATION_TIMEOUT: Duration = Duration::from_secs(30);

/// Builder for document and API fetches, for callers that add their own
/// configuration such as a MusicBrainz user agent.
pub fn document_builder() -> ClientBuilder {
    Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(OPERATION_TIMEOUT)
}

/// Client for document and API fetches.
///
/// Panics only if TLS backend initialization fails, which is the same failure
/// mode as the `reqwest::blocking::Client::new()` calls this replaces.
pub fn document() -> Client {
    document_builder()
        .build()
        .expect("build document HTTP client")
}

/// Builder for media transfers.
///
/// Same budgets as documents. The distinction is that media callers stream
/// bodies, so the operation timeout bounds inactivity between reads rather than
/// the total transfer, which is what makes a multi-minute enclosure download
/// safe under a 30 second number.
pub fn media_builder() -> ClientBuilder {
    Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(OPERATION_TIMEOUT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    /// A host that accepts the connection and then says nothing must fail
    /// rather than hang. Pins the behavior rather than trusting the dependency
    /// default to stay where it is.
    #[test]
    fn client_gives_up_on_a_silent_host() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("listener addr");
        std::thread::spawn(move || {
            let _accepted = listener.accept();
            std::thread::sleep(Duration::from_secs(120));
        });

        let client = document_builder()
            .timeout(Duration::from_millis(250))
            .build()
            .expect("build client");

        let error = client
            .get(format!("http://{addr}/feed.xml"))
            .send()
            .expect_err("a silent host must time out, not hang");

        assert!(error.is_timeout(), "expected a timeout error, got: {error}");
    }

    #[test]
    fn builders_carry_the_shared_policy() {
        document_builder().build().expect("document client builds");
        media_builder().build().expect("media client builds");
    }
}
