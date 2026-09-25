//! Comparison rules of the playlist RSS check (ADR 0076 Decision 3).
//!
//! Each compared element has one written rule. Formatting alone is not a
//! difference: a whitespace change, an HTML change with the same readable
//! text, or a URL with a default port is equal.
//!
//! - Descriptions compare as readable text, per the ADR 0075 comparison
//!   contract, version 1. HTML goes through the standards-based `html5ever`
//!   parser. Plain text is not parsed and not entity-decoded.
//! - URLs compare after the accepted normalization of scheme, host and
//!   default port. Path, query and fragment differences stay.
//! - Other text compares after the removal of outer whitespace.
//!
//! The functions here are pure. The check (`check_apply.rs`) and the
//! `MusicIndex` hold gate (`db::rss_field_holds`) share them.

#![warn(clippy::pedantic)]

use serde_json::Value;

/// The representation of one description value (comparison contract,
/// version 1). An RSS `<description>` carries HTML. A `MusicIndex`
/// description is plain text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TextRepresentation {
    PlainText,
    Html,
}

/// Text with outer whitespace removed. An empty result is no value.
#[must_use]
pub(crate) fn trimmed(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

/// Trimmed text comparison.
#[must_use]
pub(crate) fn text_equal(left: Option<&str>, right: Option<&str>) -> bool {
    trimmed(left) == trimmed(right)
}

/// Language tag comparison: trimmed and lowercase.
#[must_use]
pub(crate) fn language_equal(left: Option<&str>, right: Option<&str>) -> bool {
    trimmed(left).map(str::to_ascii_lowercase) == trimmed(right).map(str::to_ascii_lowercase)
}

/// The accepted URL normalization: the parser's scheme, host and
/// default-port form. An invalid URL stays as its trimmed text, so two
/// different invalid values are still different.
#[must_use]
pub(crate) fn normalized_url(value: Option<&str>) -> Option<String> {
    let value = trimmed(value)?;
    Some(
        reqwest::Url::parse(value).map_or_else(|_| value.to_owned(), |url| url.as_str().to_owned()),
    )
}

/// URL comparison after normalization.
#[must_use]
pub(crate) fn url_equal(left: Option<&str>, right: Option<&str>) -> bool {
    normalized_url(left) == normalized_url(right)
}

/// Readable text of one description value (comparison contract, version 1).
#[must_use]
pub(crate) fn readable_text(value: &str, representation: TextRepresentation) -> String {
    let text = match representation {
        TextRepresentation::PlainText => value.to_owned(),
        TextRepresentation::Html => html_readable_text(value),
    };
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Description comparison with the representation of each side.
#[must_use]
pub(crate) fn description_equal(
    left: Option<&str>,
    left_representation: TextRepresentation,
    right: Option<&str>,
    right_representation: TextRepresentation,
) -> bool {
    let left = trimmed(left)
        .map(|value| readable_text(value, left_representation))
        .filter(|value| !value.is_empty());
    let right = trimmed(right)
        .map(|value| readable_text(value, right_representation))
        .filter(|value| !value.is_empty());
    left == right
}

fn html_readable_text(value: &str) -> String {
    use html5ever::tendril::TendrilSink as _;
    use html5ever::{local_name, namespace_url, ns, parse_fragment, ParseOpts, QualName};
    use markup5ever_rcdom::RcDom;

    let dom = parse_fragment(
        RcDom::default(),
        ParseOpts::default(),
        QualName::new(None, ns!(html), local_name!("body")),
        Vec::new(),
    )
    .one(value);
    let mut output = String::new();
    append_readable(&dom.document, &mut output);
    output
}

fn append_readable(node: &markup5ever_rcdom::Handle, output: &mut String) {
    use markup5ever_rcdom::NodeData;
    match &node.data {
        NodeData::Text { contents } => output.push_str(&contents.borrow()),
        NodeData::Element { name, .. } => {
            let tag = name.local.as_ref();
            if matches!(tag, "script" | "style" | "template") {
                return;
            }
            let block = is_block(tag);
            if block {
                output.push(' ');
            }
            for child in node.children.borrow().iter() {
                append_readable(child, output);
            }
            if block {
                output.push(' ');
            }
        }
        NodeData::Document => {
            for child in node.children.borrow().iter() {
                append_readable(child, output);
            }
        }
        NodeData::Comment { .. }
        | NodeData::Doctype { .. }
        | NodeData::ProcessingInstruction { .. } => {}
    }
}

/// Block elements and `br` insert a word boundary (contract rule 6).
fn is_block(tag: &str) -> bool {
    matches!(
        tag,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "br"
            | "dd"
            | "details"
            | "dialog"
            | "div"
            | "dl"
            | "dt"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hgroup"
            | "hr"
            | "li"
            | "main"
            | "nav"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "summary"
            | "table"
            | "tbody"
            | "td"
            | "tfoot"
            | "th"
            | "thead"
            | "tr"
            | "ul"
    )
}

/// The recipient set of one `podcast:value` block, in the JSON form of
/// `helpers::value_block_json`. Each recipient gives its type, address,
/// split, fee, custom key, custom value and name. The set has no order.
#[must_use]
pub(crate) fn value_recipients(value_json: Option<&str>) -> Option<Vec<Vec<Option<String>>>> {
    let value: Value = serde_json::from_str(trimmed(value_json)?).ok()?;
    let mut recipients = value["children"]["valueRecipient"]
        .as_array()
        .map(|recipients| {
            recipients
                .iter()
                .map(|recipient| {
                    [
                        "type",
                        "address",
                        "split",
                        "fee",
                        "customKey",
                        "customValue",
                        "name",
                    ]
                    .iter()
                    .map(|attr| {
                        recipient["attrs"][*attr]
                            .as_str()
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .map(str::to_owned)
                    })
                    .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    recipients.sort();
    Some(recipients)
}

/// Payment route comparison: the recipient set of each value block.
#[must_use]
pub(crate) fn value_routes_equal(left: Option<&str>, right: Option<&str>) -> bool {
    let left = value_recipients(left).filter(|recipients| !recipients.is_empty());
    let right = value_recipients(right).filter(|recipients| !recipients.is_empty());
    left == right
}

/// One compared person: name, role, group, href and image, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ComparedPerson {
    pub(crate) name: Option<String>,
    pub(crate) role: Option<String>,
    pub(crate) group: Option<String>,
    pub(crate) href: Option<String>,
    pub(crate) image: Option<String>,
}

impl ComparedPerson {
    fn normalized(&self) -> [Option<String>; 5] {
        [
            trimmed(self.name.as_deref()).map(str::to_owned),
            trimmed(self.role.as_deref()).map(str::to_ascii_lowercase),
            trimmed(self.group.as_deref()).map(str::to_ascii_lowercase),
            normalized_url(self.href.as_deref()),
            normalized_url(self.image.as_deref()),
        ]
    }

    #[must_use]
    pub(crate) fn to_json(&self) -> Value {
        serde_json::json!({
            "name": self.name,
            "role": self.role,
            "group": self.group,
            "href": self.href,
            "image": self.image,
        })
    }

    #[must_use]
    pub(crate) fn from_json(value: &Value) -> Self {
        let text = |key: &str| value[key].as_str().map(str::to_owned);
        Self {
            name: text("name"),
            role: text("role"),
            group: text("group"),
            href: text("href"),
            image: text("image"),
        }
    }
}

/// Person list comparison, in order.
#[must_use]
pub(crate) fn persons_equal(left: &[ComparedPerson], right: &[ComparedPerson]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| left.normalized() == right.normalized())
}

/// Nostr identity comparison: the set of exact values.
#[must_use]
pub(crate) fn identities_equal(left: &[String], right: &[String]) -> bool {
    let normalize = |values: &[String]| {
        let mut values = values
            .iter()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        values.sort();
        values.dedup();
        values
    };
    normalize(left) == normalize(right)
}

/// `itunes:explicit` text as a boolean.
#[must_use]
pub(crate) fn explicit_flag(value: Option<&str>) -> Option<bool> {
    match trimmed(value)?.to_ascii_lowercase().as_str() {
        "explicit" | "yes" | "true" => Some(true),
        "clean" | "no" | "false" => Some(false),
        _ => None,
    }
}

/// An RSS `pubDate` as Unix seconds. RFC 2822 first, then RFC 3339.
#[must_use]
pub(crate) fn instant(value: Option<&str>) -> Option<i64> {
    let value = trimmed(value)?;
    chrono::DateTime::parse_from_rfc2822(value)
        .or_else(|_| chrono::DateTime::parse_from_rfc3339(value))
        .ok()
        .map(|date| date.timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R2-02: equal readable text with different HTML gives no difference.
    #[test]
    fn adr_0076_rss_comparison_equal_readable_text_is_equal() {
        use TextRepresentation::{Html, PlainText};
        for (html, plain) in [
            ("<p>Hello <b>world</b></p>", "Hello world"),
            ("A&nbsp;B", "A B"),
            ("<p>A</p><p>B</p>", "A B"),
            ("inter<b>national</b>", "international"),
            ("&lt;literal&gt;", "<literal>"),
            ("A &amp;amp; B", "A &amp; B"),
            ("<p>Hi</p><!-- note --><script>x()</script>", "Hi"),
        ] {
            assert!(
                description_equal(Some(html), Html, Some(plain), PlainText),
                "{html} and {plain}"
            );
        }
        assert!(description_equal(
            Some("<p>Same   text</p>"),
            Html,
            Some("<div>Same text</div>"),
            Html
        ));
        for (html, plain) in [
            ("A B", "AB"),
            ("Song", "song"),
            ("<p>A</p><p>B</p>", "AB"),
            ("A&nbsp;B", "A&nbsp;B"),
        ] {
            assert!(
                !description_equal(Some(html), Html, Some(plain), PlainText),
                "{html} and {plain}"
            );
        }
    }

    /// R2-03: a URL that differs only by a default port gives no difference.
    #[test]
    fn adr_0076_rss_comparison_default_port_url_is_equal() {
        assert!(url_equal(
            Some("https://Example.test:443/a.jpg"),
            Some(" https://example.test/a.jpg")
        ));
        assert!(url_equal(
            Some("HTTP://example.test:80/x?q=1"),
            Some("http://example.test/x?q=1")
        ));
        assert!(!url_equal(
            Some("https://example.test/a.jpg"),
            Some("https://example.test/A.jpg")
        ));
        assert!(!url_equal(
            Some("https://example.test:8443/a"),
            Some("https://example.test/a")
        ));
    }

    #[test]
    fn adr_0076_rss_comparison_recipient_set_ignores_order_and_formatting() {
        let one = r#"{"value":null,"attrs":{"type":"lightning"},"children":{"valueRecipient":[{"attrs":{"name":"A","type":"node","address":"a","split":"90"}},{"attrs":{"name":"B","type":"node","address":"b","split":"10"}}]}}"#;
        let two = r#"{"value":null,"attrs":{"type":"lightning","method":"keysend"},"children":{"valueRecipient":[{"attrs":{"name":"B ","type":"node","address":"b","split":"10"}},{"attrs":{"name":"A","type":"node","address":"a","split":"90"}}]}}"#;
        let three = r#"{"value":null,"attrs":{},"children":{"valueRecipient":[{"attrs":{"name":"A","type":"node","address":"a","split":"95"}},{"attrs":{"name":"B","type":"node","address":"b","split":"5"}}]}}"#;
        assert!(value_routes_equal(Some(one), Some(two)));
        assert!(!value_routes_equal(Some(one), Some(three)));
        assert!(!value_routes_equal(Some(one), None));
    }
}
