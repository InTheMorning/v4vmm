//! Read-only upgrade recognition and shared-registry candidate preparation (ADRs 0016, 0066).

#![warn(clippy::pedantic)]

use rusqlite::Connection;

/// Compare the interrupted database with the schema produced by the normal
/// authority. No source DDL or ledger write is performed during recognition.
pub(super) fn recognize_migration_11(conn: &Connection) -> rusqlite::Result<bool> {
    let expected = Connection::open_in_memory()?;
    super::init_schema(&expected).map_err(|_| rusqlite::Error::InvalidQuery)?;
    super::migrate_schema(&expected).map_err(|_| rusqlite::Error::InvalidQuery)?;
    for (table, _) in super::CURRENT_COLUMNS {
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
    conn.execute_batch(
        "DROP TABLE broadcast_event_selection; DELETE FROM schema_migrations WHERE version = 11;",
    )?;
    let result = super::migrate_schema_with(conn, |version, reached| {
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
    fn adr_0066_upgrade_schema_recognizes_legacy_column_order_without_ignoring_sql_values() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_schema(&conn).unwrap();
        migrate_schema(&conn).unwrap();
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
        let conn = crate::db::open_db(&path).unwrap();
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
            crate::db::init_schema(&conn).unwrap();
            migrate_schema(&conn).unwrap();
            interrupt_fixture(&conn, boundary).unwrap();
            let state = inspect_schema(&conn).unwrap();
            assert_eq!(
                state,
                match boundary {
                    MigrationBoundary::BeforeApply => SchemaCompatibility::UpgradeRequired {
                        applied: 10,
                        current: 11
                    },
                    MigrationBoundary::AfterApply => SchemaCompatibility::InterruptedUpgrade,
                    MigrationBoundary::AfterRecord => SchemaCompatibility::Current,
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
            let conn = crate::db::open_db(&path).unwrap();
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
}
