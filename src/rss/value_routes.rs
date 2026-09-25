//! The canonical payment route of one raw `podcast:value` block
//! (ADR 0076 Decision 9, packet 003).
//!
//! The database keeps the raw block in `tracks.item_value_json` and
//! `feeds.podcast_value_json`, in the JSON form of
//! `helpers::value_block_json`. This module converts that block to the
//! `api::PaymentRoute` list that the file frame `TXXX:MusicIndex Value
//! Routes` holds. Migration 17, the playlist RSS check and the subscribe
//! persist step use this one converter.

#![warn(clippy::pedantic)]

use serde_json::Value;

use crate::api::PaymentRoute;

/// Converts one raw `podcast:value` block to its payment route list.
///
/// Each `podcast:valueRecipient` gives one route: `name`, `type`, `address`,
/// `split`, `fee`, `customKey` and `customValue`. A block without a
/// recipient gives an empty list. A recipient without `fee` has the
/// namespace default `false`.
///
/// # Errors
///
/// Returns a reason when the block is not a JSON object, when its recipient
/// list is not an array, or when a `split` or `fee` value has no valid form.
pub(crate) fn payment_routes_from_value_block(raw: &str) -> Result<Vec<PaymentRoute>, String> {
    let value: Value =
        serde_json::from_str(raw).map_err(|error| format!("block is not JSON: {error}"))?;
    let Some(block) = value.as_object() else {
        return Err("block is not a JSON object".to_owned());
    };
    let recipients = match block.get("children").and_then(|children| {
        children
            .as_object()
            .and_then(|children| children.get("valueRecipient"))
    }) {
        None | Some(Value::Null) => return Ok(Vec::new()),
        Some(Value::Array(recipients)) => recipients,
        Some(_) => return Err("valueRecipient is not an array".to_owned()),
    };
    recipients.iter().map(recipient_route).collect()
}

/// The canonical JSON of one raw block, for a `payment_routes_json` column.
/// A missing block or a block that does not convert gives `None`.
#[must_use]
pub(crate) fn canonical_routes_json(raw: Option<&str>) -> Option<String> {
    let routes = payment_routes_from_value_block(raw?).ok()?;
    serde_json::to_string(&routes).ok()
}

fn recipient_route(recipient: &Value) -> Result<PaymentRoute, String> {
    let attrs = recipient.get("attrs").unwrap_or(&Value::Null);
    let text = |name: &str| -> Option<String> {
        attrs
            .get(name)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    };
    let split = text("split")
        .map(|split| {
            split
                .parse::<f64>()
                .ok()
                .filter(|split| split.is_finite())
                .ok_or_else(|| format!("split `{split}` is not a number"))
        })
        .transpose()?;
    let fee = match text("fee") {
        None => false,
        Some(fee) if fee.eq_ignore_ascii_case("true") => true,
        Some(fee) if fee.eq_ignore_ascii_case("false") => false,
        Some(fee) => return Err(format!("fee `{fee}` is not true or false")),
    };
    Ok(PaymentRoute {
        recipient_name: text("name"),
        route_type: text("type").map(|route_type| route_type.to_ascii_lowercase()),
        split,
        fee: Some(fee),
        address: text("address"),
        custom_key: text("customKey"),
        custom_value: text("customValue"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A block in the form that `helpers::value_block_json` writes for a
    /// recorded two-recipient `podcast:value` element.
    const RECORDED_BLOCK: &str = r#"{"value":null,"attrs":{"type":"lightning","method":"keysend","suggested":"0.00000005000"},"children":{"valueRecipient":[{"value":null,"attrs":{"name":"The Band","type":"node","address":"03ae9f91a0cb8ff43840e3c322c4c61f019d8c1c3cea15a25cfc425ac605e61a4a","split":"95"},"children":{}},{"value":null,"attrs":{"name":"Podcastindex.org","type":"node","address":"03ae9f91a0cb8ff43840e3c322c4c61f019d8c1c3cea15a25cfc425ac605e61a4a","split":"5","customKey":"696969","customValue":"eChoVKtO1KujpAA5HCoB","fee":"true"},"children":{}}]}}"#;

    /// R3-01: a recorded block converts to the expected list, and a block
    /// without a recipient converts to an empty list.
    #[test]
    fn adr_0076_route_readiness_value_block_converts_to_payment_routes() {
        let routes = payment_routes_from_value_block(RECORDED_BLOCK).unwrap();
        assert_eq!(routes.len(), 2);
        let band = &routes[0];
        assert_eq!(band.recipient_name.as_deref(), Some("The Band"));
        assert_eq!(band.route_type.as_deref(), Some("node"));
        assert_eq!(band.split, Some(95.0));
        assert_eq!(band.fee, Some(false));
        assert_eq!(
            band.address.as_deref(),
            Some("03ae9f91a0cb8ff43840e3c322c4c61f019d8c1c3cea15a25cfc425ac605e61a4a")
        );
        assert_eq!(band.custom_key, None);
        assert_eq!(band.custom_value, None);
        let index = &routes[1];
        assert_eq!(index.recipient_name.as_deref(), Some("Podcastindex.org"));
        assert_eq!(index.split, Some(5.0));
        assert_eq!(index.fee, Some(true));
        assert_eq!(index.custom_key.as_deref(), Some("696969"));
        assert_eq!(index.custom_value.as_deref(), Some("eChoVKtO1KujpAA5HCoB"));

        for empty in [
            r#"{"value":null,"attrs":{"type":"lightning"},"children":{}}"#,
            r#"{"value":null,"attrs":{"type":"lightning"},"children":{"valueRecipient":[]}}"#,
            r#"{"attrs":{}}"#,
        ] {
            assert_eq!(payment_routes_from_value_block(empty).unwrap().len(), 0);
            assert_eq!(canonical_routes_json(Some(empty)).as_deref(), Some("[]"));
        }
    }

    #[test]
    fn adr_0076_route_readiness_invalid_value_block_does_not_convert() {
        for invalid in [
            "not json",
            "[]",
            r#"{"children":{"valueRecipient":{}}}"#,
            r#"{"children":{"valueRecipient":[{"attrs":{"split":"most"}}]}}"#,
            r#"{"children":{"valueRecipient":[{"attrs":{"split":"50","fee":"maybe"}}]}}"#,
        ] {
            assert!(
                payment_routes_from_value_block(invalid).is_err(),
                "{invalid}"
            );
            assert_eq!(canonical_routes_json(Some(invalid)), None);
        }
        assert_eq!(canonical_routes_json(None), None);
    }
}
