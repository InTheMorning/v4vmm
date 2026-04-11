mod common;

use rusqlite::params;
use v4vmm::{config, db, rss};

fn sample_feed(title: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0"
     xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd"
     xmlns:podcast="https://podcastindex.org/namespace/1.0">
  <channel>
    <title>{title}</title>
    <link>https://example.com/artist</link>
    <language>en-us</language>
    <description>Example music feed</description>
    <podcast:guid>feed-guid-123</podcast:guid>
    <podcast:medium>music</podcast:medium>
    <podcast:image href="https://example.com/feed.jpg" />
    <item>
      <title>Track One</title>
      <guid>track-guid-1</guid>
      <link>https://example.com/track-one</link>
      <pubDate>Tue, 12 Mar 2024 10:00:00 GMT</pubDate>
      <enclosure url="https://example.com/track1.mp3" type="audio/mpeg" length="12345" />
      <itunes:duration>03:45</itunes:duration>
      <itunes:explicit>no</itunes:explicit>
      <itunes:image href="https://example.com/track1.jpg" />
      <podcast:episode>7</podcast:episode>
    </item>
    <item>
      <title>Track Without Guid</title>
      <enclosure url="https://example.com/skip.mp3" type="audio/mpeg" length="777" />
    </item>
  </channel>
</rss>"#
    )
}

#[test]
fn subscribe_persists_feed_and_tracks() {
    let (cfg, _dir) = common::test_config();
    config::ensure_dirs(&cfg).unwrap();
    let mut conn = db::open_db(&cfg).unwrap();
    let base_url =
        common::serve_http_sequence(vec![(sample_feed("Example Feed"), "application/rss+xml")]);
    let feed_url = format!("{base_url}/feed.xml");

    rss::cmd_subscribe(&cfg, &mut conn, &feed_url).unwrap();

    let feed_row: (String, String, String, String) = conn
        .query_row(
            "SELECT title, feed_guid, podcast_medium, album_image_href FROM feeds WHERE feed_url = ?1",
            params![feed_url],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(feed_row.0, "Example Feed");
    assert_eq!(feed_row.1, "feed-guid-123");
    assert_eq!(feed_row.2, "music");
    assert_eq!(feed_row.3, "https://example.com/feed.jpg");

    let track_row: (String, i64, String) = conn
        .query_row(
            "SELECT track_title, duration_seconds, itunes_duration_raw \
             FROM tracks WHERE item_guid = 'track-guid-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(track_row.0, "Track One");
    assert_eq!(track_row.1, 225);
    assert_eq!(track_row.2, "03:45");

    let track_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tracks", [], |row| row.get(0))
        .unwrap();
    assert_eq!(track_count, 1, "items without guid should be skipped");
}

#[test]
fn subscribe_is_idempotent_for_same_feed_url() {
    let (cfg, _dir) = common::test_config();
    config::ensure_dirs(&cfg).unwrap();
    let mut conn = db::open_db(&cfg).unwrap();
    let base_url = common::serve_http_sequence(vec![
        (sample_feed("First Title"), "application/rss+xml"),
        (sample_feed("Updated Title"), "application/rss+xml"),
    ]);
    let feed_url = format!("{base_url}/feed.xml");

    rss::cmd_subscribe(&cfg, &mut conn, &feed_url).unwrap();
    rss::cmd_subscribe(&cfg, &mut conn, &feed_url).unwrap();

    let counts: (i64, i64) = conn
        .query_row(
            "SELECT
                (SELECT COUNT(*) FROM feeds),
                (SELECT COUNT(*) FROM tracks)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(counts, (1, 1));

    let title: String = conn
        .query_row(
            "SELECT title FROM feeds WHERE feed_url = ?1",
            params![feed_url],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(title, "Updated Title");
}
