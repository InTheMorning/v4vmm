//! Recent-feed and Discover feed-list display projections.

#![warn(clippy::pedantic)]

use crate::api::Feed;

use super::common::nonempty_text;
use super::feed_display_title;

/// Borrow-only projection for one recent-feed tile.
pub(crate) struct RecentFeedTileVm<'a> {
    feed: &'a Feed,
}

/// Display-ready content for one Discovery recent-feed tile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecentFeedTileDisplay {
    pub id: String,
    pub feed_list_tile_id: String,
    pub recent_tile_id: String,
    pub podroll_tile_id: String,
    pub title: String,
    pub a11y_label: String,
    pub subtitle: Option<String>,
    pub episode_note: Option<String>,
    pub image_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecentFeedTileOpenTarget {
    pub guid: String,
    pub title: String,
}

impl RecentFeedTileDisplay {
    #[must_use]
    pub fn open_target(&self) -> RecentFeedTileOpenTarget {
        RecentFeedTileOpenTarget {
            guid: self.id.clone(),
            title: self.title.clone(),
        }
    }

    #[must_use]
    pub fn take_recent_tile_id(&mut self) -> String {
        std::mem::take(&mut self.recent_tile_id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PodrollSectionDisplay {
    pub(crate) heading_label: &'static str,
    pub(crate) scroll_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SearchFeedListSectionDisplay {
    pub(crate) heading: &'static str,
}

impl<'a> RecentFeedTileVm<'a> {
    #[must_use]
    pub(crate) const fn new(feed: &'a Feed) -> Self {
        Self { feed }
    }

    #[must_use]
    pub(crate) fn display(&self) -> RecentFeedTileDisplay {
        let id = self.feed.feed_guid.clone().unwrap_or_default();
        let title = feed_display_title(self.feed);
        RecentFeedTileDisplay {
            feed_list_tile_id: format!("feed-tile:{id}"),
            recent_tile_id: format!("recent-tile:{id}"),
            podroll_tile_id: format!("podroll-tile:{id}"),
            id,
            a11y_label: format!("Feed: {title}"),
            title,
            subtitle: nonempty_text(self.feed.release_artist.as_deref())
                .or_else(|| nonempty_text(self.feed.publisher_text.as_deref()))
                .map(str::to_string),
            episode_note: self
                .feed
                .episode_count
                .map(|count| format!("{count} tracks")),
            image_url: self.feed.image_url.clone(),
        }
    }
}

impl PodrollSectionDisplay {
    #[must_use]
    pub(crate) fn new(entity_id: &str) -> Self {
        Self {
            heading_label: "Podroll",
            scroll_id: format!("podroll-scroll:{entity_id}"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RecentFeedsDisplay {
    pub(crate) load_more_button_id: &'static str,
    pub(crate) heading: &'static str,
    pub(crate) empty_label: &'static str,
    pub(crate) load_more_label: &'static str,
}

impl RecentFeedsDisplay {
    pub(super) const VALUE: Self = Self {
        load_more_button_id: "recent-load-more",
        heading: "Recent Feeds",
        empty_label: "No recent feeds",
        load_more_label: "Load more",
    };
}

/// Pure render snapshot for the recent-feeds root panel.
#[derive(Clone, Debug)]
pub(crate) struct RecentFeedsSnapshot {
    pub(crate) display: RecentFeedsDisplay,
    pub(crate) feeds: Vec<Feed>,
    pub(crate) status: String,
    pub(crate) has_more: bool,
    pub(crate) loading: bool,
    pub(crate) empty: bool,
}
