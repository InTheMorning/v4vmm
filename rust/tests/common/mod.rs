use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::thread;

use tempfile::TempDir;
use v4vmm::config::Config;

/// Builds a temp-backed config suitable for integration tests.
pub fn test_config() -> (Config, TempDir) {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let cfg = Config {
        music_dir: dir.path().join("music"),
        db_path: dir.path().join("data").join("v4vmm.sqlite"),
    };
    (cfg, dir)
}

/// Returns the list of application tables for schema assertions.
#[allow(
    dead_code,
    reason = "shared helper used selectively across integration tests"
)]
pub fn table_names(conn: &rusqlite::Connection) -> Vec<String> {
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .unwrap();
    stmt.query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// Serves a fixed list of HTTP 200 responses and returns the base URL.
#[allow(
    dead_code,
    reason = "shared helper used selectively across integration tests"
)]
pub fn serve_http_sequence(responses: Vec<(String, &'static str)>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let addr = listener.local_addr().expect("listener addr");

    thread::spawn(move || {
        for (body, content_type) in responses {
            let (mut stream, _) = listener.accept().expect("accept test request");

            let mut buf = [0_u8; 4096];
            let _ = stream.read(&mut buf);

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .expect("write test response");
            stream.flush().expect("flush test response");
        }
    });

    format!("http://{}", addr)
}

#[allow(
    dead_code,
    reason = "shared helper used selectively across integration tests"
)]
pub fn path_exists(path: &Path) -> bool {
    path.exists()
}
