//! Shared text-filter helpers for view-models.
//!
//! This module keeps normalization and case-insensitive containment in one
//! place so per-VM matchers can focus on field selection.

#![warn(clippy::pedantic)]

#[must_use]
pub(crate) fn normalize(filter: Option<String>) -> Option<String> {
    filter
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[must_use]
pub(crate) fn contains_normalized(value: &str, filter: &str) -> bool {
    let filter = filter.to_lowercase();
    value.to_lowercase().contains(&filter)
}

#[cfg(test)]
mod tests {
    use super::{contains_normalized, normalize};

    #[test]
    fn normalize_trims_and_drops_blank_filters() {
        assert_eq!(normalize(None), None);
        assert_eq!(
            normalize(Some("  Lead Singer  ".to_string())),
            Some("Lead Singer".into())
        );
        assert_eq!(normalize(Some("   ".to_string())), None);
    }

    #[test]
    fn contains_normalized_is_case_insensitive() {
        assert!(contains_normalized("Lead Singer", "lead"));
        assert!(contains_normalized("Lead Singer", "singer"));
        assert!(contains_normalized("Lead Singer", "LEAD"));
        assert!(!contains_normalized("Lead Singer", "drummer"));
    }
}
