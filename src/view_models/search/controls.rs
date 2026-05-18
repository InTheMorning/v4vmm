//! Search pane controls, filters, sections, and render snapshots.

#![warn(clippy::pedantic)]

use super::ResultRow;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SearchTypeFilterOptionDisplay {
    pub(crate) index: usize,
    pub(crate) button_id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) a11y_label: &'static str,
    pub(crate) value: Option<&'static str>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SearchStatusSnapshot {
    pub(crate) text: String,
    pub(crate) display_text: String,
    pub(crate) is_error: bool,
}

impl SearchStatusSnapshot {
    #[must_use]
    pub(super) fn from_text(text: &str) -> Self {
        let is_error = text.starts_with("Error:");
        Self {
            text: text.to_string(),
            display_text: if is_error {
                format!("\u{2717} {text}")
            } else {
                text.to_string()
            },
            is_error,
        }
    }

    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

/// Static labels and dynamic toggle label for the Discover results pane.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SearchPaneDisplay {
    pub(crate) split_pane_id: &'static str,
    pub(crate) resize_handle_id: &'static str,
    pub(crate) search_button_id: &'static str,
    pub(crate) fuzzy_toggle_id: &'static str,
    pub(crate) recents_button_id: &'static str,
    pub(crate) results_scroll_id: &'static str,
    pub(crate) load_more_button_id: &'static str,
    pub(crate) heading: &'static str,
    pub(crate) search_button_label: &'static str,
    pub(crate) refresh_button_label: &'static str,
    pub(crate) fuzzy_toggle_label: &'static str,
    pub(crate) recents_button_label: &'static str,
    pub(crate) empty_icon: &'static str,
    pub(crate) empty_label: &'static str,
    pub(crate) load_more_label: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SearchResultSectionDisplay {
    pub(crate) id: &'static str,
    pub(crate) heading: &'static str,
    pub(crate) empty_label: &'static str,
}

#[derive(Clone, Debug)]
pub(crate) struct SearchResultSection {
    pub(crate) display: SearchResultSectionDisplay,
    pub(crate) rows: Vec<ResultRow>,
    pub(crate) show_empty: bool,
}

impl SearchPaneDisplay {
    #[must_use]
    pub(super) const fn new(fuzzy_search: bool) -> Self {
        Self {
            split_pane_id: "pane-container",
            resize_handle_id: "resize-handle",
            search_button_id: "search-btn",
            fuzzy_toggle_id: "fuzzy-toggle",
            recents_button_id: "show-recents",
            results_scroll_id: "results-scroll",
            load_more_button_id: "load-more",
            heading: "Results",
            search_button_label: "Search Index",
            refresh_button_label: "Refresh",
            fuzzy_toggle_label: if fuzzy_search {
                "Fuzzy: On"
            } else {
                "Fuzzy: Off"
            },
            recents_button_label: "Recent Feeds",
            empty_icon: "\u{1F50D}",
            empty_label: "No results",
            load_more_label: "Load more",
        }
    }
}

/// Pure render snapshot for the Discover/Search results pane.
#[expect(
    clippy::struct_excessive_bools,
    reason = "render snapshots intentionally group screen flags for one render pass"
)]
#[derive(Clone, Debug)]
pub(crate) struct SearchRenderSnapshot {
    pub(crate) status: SearchStatusSnapshot,
    pub(crate) pane_display: SearchPaneDisplay,
    pub(crate) sections: Vec<SearchResultSection>,
    pub(crate) selected_key: Option<String>,
    pub(crate) type_filter: usize,
    pub(crate) index_controls: IndexControlsVisibility,
    pub(crate) show_recents_root: bool,
    pub(crate) show_recents_command: bool,
    pub(crate) loading: bool,
    pub(crate) empty: bool,
    pub(crate) has_more: bool,
    pub(crate) fuzzy_search: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum IndexControlsVisibility {
    Visible,
    Hidden,
}

impl IndexControlsVisibility {
    #[must_use]
    pub(crate) const fn is_visible(self) -> bool {
        matches!(self, Self::Visible)
    }
}

/// Static labels for the recent-feeds root panel.

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InspectorChromeDisplay {
    pub(crate) breadcrumb_root_button_id: &'static str,
    pub(crate) scroll_id: &'static str,
    pub(crate) breadcrumb_root_label: &'static str,
    pub(crate) empty_icon: &'static str,
    pub(crate) empty_label: &'static str,
}

impl InspectorChromeDisplay {
    pub(super) const VALUE: Self = Self {
        breadcrumb_root_button_id: "inspector-breadcrumb-root",
        scroll_id: "inspector-scroll",
        breadcrumb_root_label: "Results",
        empty_icon: "\u{1F50D}",
        empty_label: "Select a result to inspect",
    };
}

pub(crate) const TYPE_FILTER_OPTIONS: [SearchTypeFilterOptionDisplay; 4] = [
    SearchTypeFilterOptionDisplay {
        index: 0,
        button_id: "type-filter-all",
        label: "All",
        a11y_label: "Show all search result types",
        value: None,
    },
    SearchTypeFilterOptionDisplay {
        index: 1,
        button_id: "type-filter-artist",
        label: "Artist",
        a11y_label: "Show artist search results",
        value: Some("artist"),
    },
    SearchTypeFilterOptionDisplay {
        index: 2,
        button_id: "type-filter-feed",
        label: "Feed",
        a11y_label: "Show feed search results",
        value: Some("feed"),
    },
    SearchTypeFilterOptionDisplay {
        index: 3,
        button_id: "type-filter-track",
        label: "Track",
        a11y_label: "Show track search results",
        value: Some("track"),
    },
];
pub(crate) const TYPE_FILTER_LEN: usize = TYPE_FILTER_OPTIONS.len();
