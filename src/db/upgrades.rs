//! Read-only upgrade recognition and shared-registry candidate preparation (ADRs 0016, 0066).

#![warn(clippy::pedantic)]

use rusqlite::Connection;

/// Compare the interrupted database with the schema produced by the normal
/// authority. No source DDL or ledger write is performed during recognition.
pub(super) fn recognize_migration_11(conn: &Connection) -> rusqlite::Result<bool> {
    let expected = Connection::open_in_memory()?;
    super::init_schema(&expected).map_err(|_| rusqlite::Error::InvalidQuery)?;
    super::migrate_schema_to(&expected, 11).map_err(|_| rusqlite::Error::InvalidQuery)?;
    for (table, _) in super::VERSION_11_COLUMNS {
        if columns(conn, table)? != columns(&expected, table)? {
            return Ok(false);
        }
    }
    // Migration 11 has CHECK constraints and intentionally no foreign key.
    // Match its actual authority-produced definition as well as column types.
    // Reject extra schema objects, including triggers that could change data
    // when the migration ledger is recorded.
    Ok(objects(conn)? == objects(&expected)?
        && conn.query_row(
            "SELECT count(*) FROM broadcast_event_selection WHERE singleton != 1 OR event_id = '' OR revision <= 0 OR typeof(singleton) != 'integer' OR typeof(event_id) != 'text' OR typeof(revision) != 'integer'",
            [], |row| row.get::<_, i64>(0),
        )? == 0)
}

/// Compare a supported target against the normal registry's exact schema.
pub(crate) fn verify_target(conn: &Connection, target: i64) -> anyhow::Result<()> {
    let expected = Connection::open_in_memory()?;
    super::init_schema(&expected)?;
    super::migrate_schema_internal(&expected, target, |_, _| Ok(()), false)?;
    anyhow::ensure!(
        objects(conn)? == objects(&expected)?,
        "Database schema differs from its target"
    );
    for (table, _) in super::schema_contract(target) {
        anyhow::ensure!(
            columns(conn, table)? == columns(&expected, table)?,
            "Database columns differ from their target"
        );
    }
    let ledger = |connection: &Connection| -> rusqlite::Result<Vec<(i64, String)>> {
        connection
            .prepare("SELECT version,name FROM schema_migrations ORDER BY version")?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect()
    };
    anyhow::ensure!(
        ledger(conn)? == ledger(&expected)?,
        "Database ledger differs from its target"
    );
    if target == 11 {
        anyhow::ensure!(
            recognize_migration_11(conn)?,
            "Invalid migration 11 selection state"
        );
    }
    verify_integrity(conn)
}

pub(super) fn verify_integrity(conn: &Connection) -> anyhow::Result<()> {
    let results = conn
        .prepare("PRAGMA integrity_check")?
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    anyhow::ensure!(results == ["ok"], "Database integrity check failed");
    anyhow::ensure!(
        !conn.prepare("PRAGMA foreign_key_check")?.exists([])?,
        "Database foreign-key check failed"
    );
    Ok(())
}

/// Hash every version-11 row identity and value, including its complete ledger.
pub(super) fn legacy_digest(conn: &Connection) -> anyhow::Result<String> {
    version_11_digest(conn, super::VERSION_11_COLUMNS.iter())
}

/// Hash the version-11 rows that migration 13 keeps (ADR 0079).
pub(super) fn retained_digest(conn: &Connection) -> anyhow::Result<String> {
    version_11_digest(
        conn,
        super::VERSION_11_COLUMNS
            .iter()
            .filter(|(table, _)| !super::ARTIST_STORAGE_TABLES.contains(table)),
    )
}

fn version_11_digest<'a>(
    conn: &Connection,
    tables: impl Iterator<Item = &'a (&'a str, &'a [&'a str])>,
) -> anyhow::Result<String> {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    for (table, _) in tables {
        hash.update((table.len() as u64).to_le_bytes());
        hash.update(table.as_bytes());
        let filter = if *table == "schema_migrations" {
            " WHERE version <= 11"
        } else {
            ""
        };
        let mut statement = conn.prepare(&format!(
            "SELECT rowid,* FROM {table}{filter} ORDER BY rowid"
        ))?;
        let count = statement.column_count();
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            hash.update([255]);
            for index in 0..count {
                super::maintenance::restore::hash_value(&mut hash, row.get_ref(index)?);
            }
        }
    }
    Ok(format!("{:x}", hash.finalize()))
}

/// Construct only disposable schemas through the current registry.
#[cfg(any(test, debug_assertions))]
pub(crate) fn create_fixture(conn: &Connection, target: i64) -> anyhow::Result<()> {
    anyhow::ensure!(
        super::inspect_schema(conn)? == super::SchemaCompatibility::Empty,
        "Fixture database must be empty"
    );
    anyhow::ensure!(
        matches!(target, 10..=14),
        "Unsupported fixture schema target"
    );
    conn.pragma_update(None, "foreign_keys", true)?;
    super::init_schema(conn)?;
    super::migrate_schema_to(conn, target)
}

type Column = (String, String, bool, Option<String>, i64, i64);
fn columns(conn: &Connection, table: &str) -> rusqlite::Result<Vec<Column>> {
    conn.prepare("SELECT name, type, \"notnull\", dflt_value, pk, hidden FROM pragma_table_xinfo(?1) ORDER BY name")?
        .query_map([table], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)))?
        .collect()
}

fn objects(conn: &Connection) -> rusqlite::Result<Vec<(String, String, String)>> {
    conn.prepare("SELECT type, name, coalesce(sql, '') FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%' ORDER BY type, name")?
        .query_map([], |row| {
            let kind: String = row.get(0)?;
            let name: String = row.get(1)?;
            let sql: String = row.get(2)?;
            let mut definition = schema_definition(&sql);
            // Migration 2 can append this column on legacy databases; fresh
            // initialization places it after enclosure_url. Its complete type
            // contract is checked above. Every other definition must match.
            if kind == "table" && name == "tracks" {
                for column in ["enclosure_typeTEXTNULL,", "enclosure_typeTEXT,", ",enclosure_typeTEXTNULL", ",enclosure_typeTEXT"] {
                    definition = definition.replace(column, "");
                }
            }
            Ok((kind, name, definition))
        })?.collect()
}

// Ignore formatting outside quoted SQL values and comments. In particular,
// CHECK(event_id != ' ') must never compare equal to CHECK(event_id != '').
fn schema_definition(sql: &str) -> String {
    let mut output = String::new();
    let mut chars = sql.chars().peekable();
    let mut quote = None;
    while let Some(character) = chars.next() {
        if let Some(end) = quote {
            output.push(character);
            if character == end {
                if chars.peek() == Some(&end) {
                    output.push(chars.next().expect("peeked quote"));
                } else {
                    quote = None;
                }
            }
        } else if character == '-' && chars.peek() == Some(&'-') {
            for next in chars.by_ref() {
                if next == '\n' {
                    break;
                }
            }
        } else if matches!(character, '\'' | '"' | '`' | '[') {
            quote = Some(if character == '[' { ']' } else { character });
            output.push(character);
        } else if !character.is_whitespace() {
            output.push(character);
        }
    }
    output
}

/// Fixture setup uses the normal migration boundary; never duplicate its SQL.
#[cfg(any(test, debug_assertions))]
pub(crate) fn interrupt_fixture(
    conn: &Connection,
    boundary: super::MigrationBoundary,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        super::inspect_schema(conn)?
            == super::SchemaCompatibility::UpgradeRequired {
                applied: 10,
                current: super::MIGRATIONS.len()
            },
        "Fixture requires registry target 10"
    );
    let result = super::migrate_schema_with(conn, 11, |version, reached| {
        anyhow::ensure!(
            version != 11 || reached != boundary,
            "Fixture interrupted migration 11"
        );
        Ok(())
    });
    anyhow::ensure!(result.is_err(), "Fixture did not reach migration boundary");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{inspect_schema, migrate_schema, MigrationBoundary, SchemaCompatibility};

    #[test]
    fn adr_0075_migration_frozen_11_preserves_every_legacy_row_and_initializes_only_generation() {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", false).unwrap();
        conn.execute_batch(include_str!("fixtures/adr-0075-schema-11.sql"))
            .unwrap();
        conn.pragma_update(None, "foreign_keys", true).unwrap();
        conn.pragma_update(None, "foreign_keys", true).unwrap();
        let before = legacy_digest(&conn).unwrap();
        assert!(recognize_migration_11(&conn).unwrap());
        assert_eq!(
            inspect_schema(&conn).unwrap(),
            SchemaCompatibility::UpgradeRequired {
                applied: 11,
                current: 14
            }
        );
        crate::db::migrate_schema_to(&conn, 12).unwrap();
        verify_target(&conn, 12).unwrap();
        assert_eq!(legacy_digest(&conn).unwrap(), before);
        for (table, _) in crate::db::provider_snapshot_schema::COLUMNS {
            let count: i64 = conn
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, i64::from(*table == "metadata_generation"), "{table}");
        }
        assert_eq!(
            conn.query_row(
                "SELECT last_generation FROM metadata_generation",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert_eq!(
            conn.query_row(
                "SELECT revision FROM broadcast_event_selection",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            7
        );
    }

    #[test]
    fn adr_0075_migration_frozen_interruption_and_partial_12_remain_distinct() {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", false).unwrap();
        conn.execute_batch(include_str!("fixtures/adr-0075-interrupted-11.sql"))
            .unwrap();
        conn.pragma_update(None, "foreign_keys", true).unwrap();
        assert_eq!(
            inspect_schema(&conn).unwrap(),
            SchemaCompatibility::InterruptedUpgrade
        );
        for invalid in [
            "CREATE TABLE unexpected(value)",
            "CREATE TABLE metadata_providers(id INTEGER)",
            "PRAGMA ignore_check_constraints=ON; UPDATE broadcast_event_selection SET revision=0",
        ] {
            let conn = Connection::open_in_memory().unwrap();
            conn.pragma_update(None, "foreign_keys", false).unwrap();
            conn.execute_batch(include_str!("fixtures/adr-0075-interrupted-11.sql"))
                .unwrap();
            conn.pragma_update(None, "foreign_keys", true).unwrap();
            conn.execute_batch(invalid).unwrap();
            assert_eq!(
                inspect_schema(&conn).unwrap(),
                SchemaCompatibility::Unknown,
                "{invalid}"
            );
        }
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", false).unwrap();
        conn.execute_batch(include_str!("fixtures/adr-0075-schema-11.sql"))
            .unwrap();
        conn.pragma_update(None, "foreign_keys", true).unwrap();
        conn.execute_batch("CREATE TABLE metadata_bodies(value)")
            .unwrap();
        assert_eq!(inspect_schema(&conn).unwrap(), SchemaCompatibility::Unknown);
    }

    #[test]
    fn adr_0075_migration_each_registry_boundary_rolls_back_schema_and_ledger() {
        for boundary in [
            MigrationBoundary::BeforeApply,
            MigrationBoundary::AfterApply,
            MigrationBoundary::AfterRecord,
        ] {
            let conn = Connection::open_in_memory().unwrap();
            conn.pragma_update(None, "foreign_keys", false).unwrap();
            conn.execute_batch(include_str!("fixtures/adr-0075-schema-11.sql"))
                .unwrap();
            conn.pragma_update(None, "foreign_keys", true).unwrap();
            let before = legacy_digest(&conn).unwrap();
            let result = crate::db::migrate_schema_with(&conn, 12, |version, reached| {
                anyhow::ensure!(
                    version != 12 || reached != boundary,
                    "Injected migration failure"
                );
                Ok(())
            });
            assert!(result.is_err());
            assert!(conn.is_autocommit());
            verify_target(&conn, 11).unwrap();
            assert_eq!(legacy_digest(&conn).unwrap(), before);
        }
    }

    #[test]
    fn adr_0075_migration_interrupted_process() {
        let Some(path) = std::env::var_os("V4VMM_TEST_MIGRATION_INTERRUPTION") else {
            return;
        };
        let conn = Connection::open(path).unwrap();
        crate::db::migrate_schema_with(&conn, 12, |version, boundary| {
            if version == 12 && boundary == MigrationBoundary::AfterRecord {
                conn.cache_flush().unwrap();
                std::process::exit(77);
            }
            Ok(())
        })
        .unwrap();
        panic!("Migration did not reach the interruption");
    }

    #[test]
    fn adr_0075_migration_process_exit_recovers_uncommitted_schema() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("interrupted.sqlite");
        let conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "foreign_keys", false).unwrap();
        conn.execute_batch(include_str!("fixtures/adr-0075-schema-11.sql"))
            .unwrap();
        conn.pragma_update(None, "foreign_keys", true).unwrap();
        let before = legacy_digest(&conn).unwrap();
        drop(conn);
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "db::upgrades::tests::adr_0075_migration_interrupted_process",
                "--nocapture",
            ])
            .env("V4VMM_TEST_MIGRATION_INTERRUPTION", &path)
            .output()
            .unwrap()
            .status;
        assert_eq!(status.code(), Some(77));
        let conn = Connection::open(&path).unwrap();
        verify_target(&conn, 11).unwrap();
        assert_eq!(legacy_digest(&conn).unwrap(), before);
    }

    #[test]
    fn adr_0066_upgrade_schema_recognizes_legacy_column_order_without_ignoring_sql_values() {
        let conn = Connection::open_in_memory().unwrap();
        create_fixture(&conn, 10).unwrap();
        interrupt_fixture(&conn, MigrationBoundary::AfterApply).unwrap();
        conn.execute_batch("ALTER TABLE tracks DROP COLUMN enclosure_type; ALTER TABLE tracks ADD COLUMN enclosure_type TEXT").unwrap();
        assert_eq!(
            inspect_schema(&conn).unwrap(),
            SchemaCompatibility::InterruptedUpgrade
        );
        assert_ne!(
            schema_definition("CHECK (event_id != ' ' )"),
            schema_definition("CHECK(event_id != '')")
        );
    }

    #[test]
    fn adr_0066_upgrade_integrity_damage_never_authorizes_repair() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("damaged.sqlite");
        let conn = Connection::open(&path).unwrap();
        create_fixture(&conn, 10).unwrap();
        interrupt_fixture(&conn, MigrationBoundary::AfterApply).unwrap();
        drop(conn);
        let mut bytes = std::fs::read(&path).unwrap();
        bytes[32..36].copy_from_slice(&0x7fff_ffff_u32.to_be_bytes());
        bytes[36..40].copy_from_slice(&1_u32.to_be_bytes());
        std::fs::write(&path, &bytes).unwrap();
        let budget = crate::db::maintenance::Budget::new(std::sync::Arc::new(
            std::sync::atomic::AtomicBool::new(false),
        ));
        assert!(!crate::db::maintenance::inspect(&path, &budget).valid_snapshot());
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }

    #[test]
    fn adr_0066_upgrade_boundaries_share_normal_registry_and_preserve_selection() {
        for boundary in [
            MigrationBoundary::BeforeApply,
            MigrationBoundary::AfterApply,
            MigrationBoundary::AfterRecord,
        ] {
            let conn = Connection::open_in_memory().unwrap();
            create_fixture(&conn, 10).unwrap();
            interrupt_fixture(&conn, boundary).unwrap();
            let state = inspect_schema(&conn).unwrap();
            assert_eq!(
                state,
                match boundary {
                    MigrationBoundary::BeforeApply => SchemaCompatibility::UpgradeRequired {
                        applied: 10,
                        current: 14
                    },
                    MigrationBoundary::AfterApply => SchemaCompatibility::InterruptedUpgrade,
                    MigrationBoundary::AfterRecord => SchemaCompatibility::UpgradeRequired {
                        applied: 11,
                        current: 14
                    },
                }
            );
            if boundary != MigrationBoundary::BeforeApply {
                conn.execute(
                    "INSERT INTO broadcast_event_selection VALUES (1, 'saved-event', 7)",
                    [],
                )
                .unwrap();
            }
            migrate_schema(&conn).unwrap();
            migrate_schema(&conn).unwrap();
            assert_eq!(inspect_schema(&conn).unwrap(), SchemaCompatibility::Current);
            let ledger: Vec<(i64, String)> = conn
                .prepare("SELECT version,name FROM schema_migrations ORDER BY version")
                .unwrap()
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                .unwrap()
                .map(Result::unwrap)
                .collect();
            assert_eq!(
                ledger,
                crate::db::MIGRATIONS
                    .iter()
                    .map(|m| (m.version, m.name.to_owned()))
                    .collect::<Vec<_>>()
            );
            if boundary != MigrationBoundary::BeforeApply {
                assert_eq!(
                    conn.query_row(
                        "SELECT event_id,revision FROM broadcast_event_selection",
                        [],
                        |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
                    )
                    .unwrap(),
                    ("saved-event".into(), 7)
                );
            }
        }
    }

    #[test]
    fn adr_0066_upgrade_recognizer_rejects_inconsistent_schema_without_writes() {
        for change in [
            "DELETE FROM schema_migrations WHERE version=4",
            "UPDATE schema_migrations SET name='wrong' WHERE version=5",
            "INSERT INTO schema_migrations(version,name) VALUES (99,'future')",
            "ALTER TABLE broadcast_event_selection DROP COLUMN revision",
            "DROP TABLE broadcast_event_selection; CREATE TABLE broadcast_event_selection(singleton TEXT, event_id TEXT, revision TEXT)",
            "DROP TABLE broadcast_event_selection; CREATE TABLE broadcast_event_selection(singleton INTEGER PRIMARY KEY, event_id TEXT NOT NULL, revision INTEGER NOT NULL)",
            "DROP TABLE broadcast_event_selection; CREATE TABLE broadcast_event_selection(singleton INTEGER PRIMARY KEY CHECK(singleton=1), event_id TEXT NOT NULL CHECK(event_id != ' '), revision INTEGER NOT NULL CHECK(revision>0))",
            "DROP TABLE entity_metadata_facts",
            "CREATE TRIGGER surprise AFTER INSERT ON schema_migrations BEGIN DELETE FROM playlists; END",
            "PRAGMA ignore_check_constraints=ON; INSERT INTO broadcast_event_selection VALUES (2,'',0)",
        ] {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("db.sqlite");
            let conn = Connection::open(&path).unwrap();
        create_fixture(&conn, 10).unwrap();
            interrupt_fixture(&conn, MigrationBoundary::AfterApply).unwrap();
            conn.execute_batch(change).unwrap();
            drop(conn);
            let before = std::fs::read(&path).unwrap();
            let conn = Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
            assert_ne!(inspect_schema(&conn).ok(), Some(SchemaCompatibility::InterruptedUpgrade), "{change}");
            drop(conn);
            assert_eq!(std::fs::read(path).unwrap(), before);
        }
    }

    fn adr_0077_artist_tables(conn: &Connection) -> Vec<String> {
        crate::db::ARTIST_STORAGE_TABLES
            .iter()
            .filter(|table| {
                conn.query_row(
                    "SELECT count(*) FROM sqlite_schema WHERE type='table' AND name=?1",
                    [table],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap()
                    > 0
            })
            .map(|table| (*table).to_owned())
            .collect()
    }

    /// R1-04: the frozen version-11 fixture migrates to the current version.
    /// A failure in migration 13 rolls migration 12 back with it.
    #[test]
    fn adr_0077_remove_artist_storage_version_11_fixture_migrates_to_current() {
        let frozen = || {
            let conn = Connection::open_in_memory().unwrap();
            conn.pragma_update(None, "foreign_keys", false).unwrap();
            conn.execute_batch(include_str!("fixtures/adr-0075-schema-11.sql"))
                .unwrap();
            conn.pragma_update(None, "foreign_keys", true).unwrap();
            conn
        };
        let conn = frozen();
        assert_eq!(adr_0077_artist_tables(&conn).len(), 4);
        let retained = retained_digest(&conn).unwrap();
        migrate_schema(&conn).unwrap();
        assert_eq!(inspect_schema(&conn).unwrap(), SchemaCompatibility::Current);
        verify_target(&conn, crate::db::CURRENT_VERSION).unwrap();
        assert_eq!(retained_digest(&conn).unwrap(), retained);
        assert!(adr_0077_artist_tables(&conn).is_empty());

        for boundary in [
            MigrationBoundary::BeforeApply,
            MigrationBoundary::AfterApply,
            MigrationBoundary::AfterRecord,
        ] {
            let conn = frozen();
            let before = legacy_digest(&conn).unwrap();
            let result = crate::db::migrate_schema_with(&conn, 13, |version, reached| {
                anyhow::ensure!(
                    version != 13 || reached != boundary,
                    "Injected migration failure"
                );
                Ok(())
            });
            assert!(result.is_err());
            assert!(conn.is_autocommit());
            verify_target(&conn, 11).unwrap();
            assert_eq!(legacy_digest(&conn).unwrap(), before);
        }
    }
}
