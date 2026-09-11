//! Shared URL redaction for operator reports (ADR 0066).

/// Formats a configured endpoint without credentials or rejected raw values.
pub(crate) fn endpoint_for_report(endpoint: &str) -> String {
    match reqwest::Url::parse(endpoint) {
        Ok(mut url) => {
            let _ = url.set_username("");
            let _ = url.set_password(None);
            url.set_query(None);
            url.set_fragment(None);
            url.to_string()
        }
        Err(_) => "[unreadable endpoint]".to_owned(),
    }
}

/// Removes URL credentials, query parameters and fragments before reports are stored.
pub(crate) fn redact_endpoint_details(detail: &str) -> String {
    detail
        .split_inclusive(char::is_whitespace)
        .map(|part| {
            let Some(start) = part.find("https://").or_else(|| part.find("http://")) else {
                return part.to_owned();
            };
            let raw = part[start..].trim_end_matches(|c: char| {
                c.is_whitespace() || matches!(c, '\'' | '"' | ')' | ',' | ';')
            });
            let suffix = &part[start + raw.len()..];
            let endpoint = endpoint_for_report(raw);
            format!("{}{endpoint}{suffix}", &part[..start])
        })
        .collect()
}
