mod common;

use v4vmm::{config, db};

#[test]
fn schema_creates_expected_tables() {
    let (cfg, _dir) = common::test_config();
    config::ensure_dirs(&cfg).unwrap();
    let conn = db::open_db(&cfg).unwrap();
    let tables = common::table_names(&conn);

    for table in ["feeds", "local_files", "schema_version", "tracks"] {
        assert!(
            tables.contains(&table.to_string()),
            "missing table: {table}"
        );
    }
}

#[test]
fn foreign_keys_are_enabled() {
    let (cfg, _dir) = common::test_config();
    config::ensure_dirs(&cfg).unwrap();
    let conn = db::open_db(&cfg).unwrap();
    let foreign_keys: i64 = conn
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .unwrap();

    assert_eq!(foreign_keys, 1);
}

#[test]
fn schema_open_is_idempotent() {
    let (cfg, _dir) = common::test_config();
    config::ensure_dirs(&cfg).unwrap();
    let conn = db::open_db(&cfg).unwrap();
    drop(conn);

    let conn2 = db::open_db(&cfg).unwrap();
    let feed_count: i64 = conn2
        .query_row("SELECT COUNT(*) FROM feeds", [], |row| row.get(0))
        .unwrap();

    assert_eq!(feed_count, 0);
}
