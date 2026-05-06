//! Feed local query family.

use rusqlite::Connection;

use crate::application::application_query_service::ApplicationQueryService;
use crate::application::errors::command::CommandError;
use crate::db;

impl ApplicationQueryService {
    /// Lists subscribed feeds that can be checked for remote updates.
    ///
    /// # Errors
    ///
    /// Returns an error when local feed state cannot be read.
    pub fn subscribed_feeds_for_stale_check(
        &self,
        conn: &Connection,
    ) -> Result<Vec<db::FeedStaleCheckRow>, CommandError> {
        db::subscribed_feeds_for_stale_check(conn).map_err(|error| query_error(&error))
    }
}

fn query_error(error: &anyhow::Error) -> CommandError {
    CommandError::Query(format!("{error:#}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> anyhow::Result<Connection> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        db::init_schema(&conn)?;
        db::migrate_schema(&conn)?;
        Ok(conn)
    }

    #[test]
    fn feed_queries_return_local_stale_check_rows() -> anyhow::Result<()> {
        let conn = setup_test_db()?;
        conn.execute(
            "INSERT INTO feeds (feed_url, feed_guid, title, is_subscribed)
             VALUES (?1, ?2, ?3, 1)",
            rusqlite::params!["https://example.test/feed.xml", "feed-guid", "Feed Title"],
        )?;

        let rows = ApplicationQueryService::new().subscribed_feeds_for_stale_check(&conn)?;

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].feed_guid, "feed-guid");

        Ok(())
    }
}
