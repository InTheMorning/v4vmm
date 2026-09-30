//! Feed and payment-route detail projections.

#![warn(clippy::pedantic)]
#![expect(
    dead_code,
    reason = "ADR 0060 packet 006 deletes this parked query layer"
)]

use crate::api::{Feed, PaymentRoute, Track};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PaymentRouteGroupDisplay {
    pub(crate) heading: &'static str,
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
