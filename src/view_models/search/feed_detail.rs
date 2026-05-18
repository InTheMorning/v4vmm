//! Feed, publisher, and payment-route detail projections.

#![warn(clippy::pedantic)]

use crate::api::{Feed, PaymentRoute, Publisher, Track};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PaymentRouteGroupDisplay {
    pub(crate) heading: &'static str,
}

/// Borrow-only projection of a [`Publisher`] inspector panel.
///
/// Owns the title fallback (`"Unknown publisher"`), the feed-count and
/// track-count fallbacks (explicit count → collection length → 0),
/// the detail-grid composition, and the feed-list visibility flag.
/// The screen still owns rendering of the feed-list section itself.
pub(crate) struct PublisherInspectorVm<'a> {
    publisher: &'a Publisher,
}

impl<'a> PublisherInspectorVm<'a> {
    #[must_use]
    pub(crate) fn new(publisher: &'a Publisher) -> Self {
        Self { publisher }
    }

    /// Display title — `publisher_text` if present, else
    /// `"Unknown publisher"`.
    #[must_use]
    pub(crate) fn title(&self) -> String {
        self.publisher
            .publisher_text
            .clone()
            .unwrap_or_else(|| "Unknown publisher".to_string())
    }

    /// Number of feeds — `feed_count` if present, else the length of
    /// the embedded `feeds` list, else `0`. Always non-negative; a
    /// negative `feed_count` is clamped to zero so the display never
    /// shows a leading minus.
    #[must_use]
    pub(crate) fn feed_count(&self) -> i32 {
        Self::resolve_count(self.publisher.feed_count, self.publisher.feeds.as_deref())
    }

    /// Number of tracks — same fallback chain as [`Self::feed_count`].
    #[must_use]
    pub(crate) fn track_count(&self) -> i32 {
        Self::resolve_count(self.publisher.track_count, self.publisher.tracks.as_deref())
    }

    fn resolve_count<T>(explicit: Option<i32>, collection: Option<&[T]>) -> i32 {
        explicit
            .or_else(|| collection.map(|c| i32::try_from(c.len()).unwrap_or(i32::MAX)))
            .unwrap_or(0)
            .max(0)
    }

    /// Detail-grid rows in display order: `Feeds`, `Tracks`.
    #[must_use]
    pub(crate) fn detail_rows(&self) -> Vec<(String, String)> {
        vec![
            ("Feeds".to_string(), self.feed_count().to_string()),
            ("Tracks".to_string(), self.track_count().to_string()),
        ]
    }

    /// Owned copy of the embedded feed list, or an empty `Vec` when
    /// the publisher carries no `feeds` field.
    #[must_use]
    pub(crate) fn feeds(&self) -> Vec<Feed> {
        self.publisher.feeds.clone().unwrap_or_default()
    }

    /// `true` when the publisher carries at least one embedded feed —
    /// used by the screen to decide whether to render the feed-list
    /// section.
    #[must_use]
    pub(crate) fn has_feed_list(&self) -> bool {
        self.publisher
            .feeds
            .as_ref()
            .is_some_and(|feeds| !feeds.is_empty())
    }
}

/// Borrow-only projection of one [`api::PaymentRoute`] entry inside the
/// inspector's value-routes panel. Owns the `"Unnamed recipient"` /
/// `"route"` fallbacks, the fee-vs-split classification, and the
/// `"Fees"` / `"Recipients"` group bucket the screen used to inline.
pub(crate) struct PaymentRouteVm<'a> {
    route: &'a PaymentRoute,
}

impl<'a> PaymentRouteVm<'a> {
    #[must_use]
    pub(crate) fn new(route: &'a PaymentRoute) -> Self {
        Self { route }
    }

    #[must_use]
    pub(crate) fn recipient_name(&self) -> String {
        self.route
            .recipient_name
            .clone()
            .unwrap_or_else(|| "Unnamed recipient".to_string())
    }

    #[must_use]
    pub(crate) fn route_type(&self) -> String {
        self.route
            .route_type
            .clone()
            .unwrap_or_else(|| "route".to_string())
    }

    /// Primary one-line payment-route summary.
    #[must_use]
    pub(crate) fn summary(&self) -> String {
        let name = self.recipient_name();
        let route_type = self.route_type();
        let split = self.split();
        let kind_label = self.kind_label();
        format!("{name} ({route_type} · {split}% · {kind_label})")
    }

    /// Optional route address display, preserving empty strings when present.
    #[must_use]
    pub(crate) fn address(&self) -> Option<String> {
        self.route.address.clone()
    }

    /// Optional route custom fields, preserving empty values when present.
    #[must_use]
    pub(crate) fn custom_fields(&self) -> Option<String> {
        let mut parts = Vec::new();
        if let Some(key) = &self.route.custom_key {
            parts.push(format!("key {key}"));
        }
        if let Some(value) = &self.route.custom_value {
            parts.push(format!("value {value}"));
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" · "))
        }
    }

    /// Split percentage; `0.0` when the route does not declare one.
    #[must_use]
    pub(crate) fn split(&self) -> f64 {
        self.route.split.unwrap_or_default()
    }

    #[must_use]
    pub(crate) fn is_fee(&self) -> bool {
        self.route.fee.unwrap_or_default()
    }

    /// `"fee"` when the route is marked as a fee, `"split"` otherwise.
    #[must_use]
    pub(crate) fn kind_label(&self) -> &'static str {
        if self.is_fee() {
            "fee"
        } else {
            "split"
        }
    }

    /// Group bucket key — `"Fees"` for fee routes, `"Recipients"`
    /// otherwise.
    #[must_use]
    pub(crate) fn group(&self) -> &'static str {
        if self.is_fee() {
            "Fees"
        } else {
            "Recipients"
        }
    }

    #[must_use]
    pub(crate) fn group_display(group: &'static str) -> PaymentRouteGroupDisplay {
        PaymentRouteGroupDisplay { heading: group }
    }
}

#[must_use]
pub(super) fn feed_inspector_tracks(feed: &Feed) -> Vec<Track> {
    feed.tracks.clone().unwrap_or_default()
}
