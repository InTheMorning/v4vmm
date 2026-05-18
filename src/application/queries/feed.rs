//! Feed local query family.

use anyhow::Result;
use rusqlite::Connection;

use crate::application::application_query_service::ApplicationQueryService;
use crate::application::command_bus::{ApplicationCommand, CommandOutcome, CommandResult};
use crate::application::command_context::CommandContext;
use crate::application::errors::command::CommandError;
use crate::db;
use crate::view_models::recent_feeds::RecentFeedsPageBatch;

use super::search::{
    index_feed_display, index_item_id, non_empty_str, INDEX_FEED_DETAIL_INCLUDE, INDEX_FEED_ID_BASE,
};

/// Fetches one remote Recent Feeds page for presentation.
#[derive(Clone, Debug)]
pub(crate) struct FetchRecentFeedsPage {
    endpoint: String,
    cursor: Option<String>,
    resume_after: usize,
}

impl FetchRecentFeedsPage {
    /// Creates a Recent Feeds page query command.
    #[must_use]
    pub(crate) fn new(
        endpoint: impl Into<String>,
        cursor: Option<String>,
        resume_after: usize,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            cursor,
            resume_after,
        }
    }
}

impl ApplicationCommand for FetchRecentFeedsPage {
    type Output = RecentFeedsPageBatch;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let batch = fetch_recent_feed_result_rows(
            &self.endpoint,
            self.cursor.as_deref(),
            self.resume_after,
        )
        .map_err(|error| query_error(&error))?;
        Ok(CommandOutcome::without_events(batch))
    }
}

fn fetch_recent_feed_result_rows(
    endpoint: &str,
    cursor: Option<&str>,
    start_index: usize,
) -> Result<RecentFeedsPageBatch> {
    let client = crate::api::Client::new_with_base_url(endpoint.to_string());
    let response = client.fetch_recent_feeds(Some(crate::api::PAGE_LIMIT), cursor)?;
    let rows = response
        .data
        .into_iter()
        .enumerate()
        .map(|(index, feed)| {
            let row_index = start_index + index;
            let feed_guid = recent_feed_activation_id(&feed, row_index);
            let detail = feed
                .feed_guid
                .as_deref()
                .and_then(|guid| {
                    client
                        .fetch_feed(guid, Some(INDEX_FEED_DETAIL_INCLUDE))
                        .ok()
                })
                .unwrap_or(feed);
            (
                index_item_id(INDEX_FEED_ID_BASE, row_index),
                index_feed_display(&feed_guid, Some(crate::api::EntityDetail::Feed(detail))),
            )
        })
        .collect();

    Ok(RecentFeedsPageBatch {
        rows,
        cursor: response.pagination.cursor,
        has_more: response.pagination.has_more,
    })
}

fn recent_feed_activation_id(feed: &crate::api::Feed, index: usize) -> String {
    [
        feed.feed_guid.as_deref(),
        feed.feed_url.as_deref(),
        feed.title.as_deref(),
        feed.name.as_deref(),
    ]
    .into_iter()
    .find_map(non_empty_str)
    .map_or_else(|| format!("recent-feed-{index}"), str::to_string)
}

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
