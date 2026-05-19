//! Shared Search/Index result row rendering.
//!
//! Search results and Recent Feeds both render `MusicIndex` result rows. This
//! shell helper keeps the row chrome in one place while each route owns its
//! page-level state and pagination.

#![warn(clippy::pedantic)]

use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    div, AnyElement, App, ClickEvent, FontWeight, Image, IntoElement, ParentElement, SharedString,
    Styled, Window,
};

use crate::ui::composites::{
    EntityKind, ListRow, ListRowA11yLabel, TagBadge, TagBadgeDisplay, Thumbnail, ThumbnailSize,
};
use crate::ui::primitives::Label;
use crate::ui::tokens::{FontSize, SemanticColor};
use crate::view_models::search_results::{
    ArtistResultDisplay, FeedResultDisplay, SearchResultOrigin, SearchResultsTab,
    TrackResultDisplay,
};

pub(crate) type SearchResultSelectHandler =
    Rc<dyn Fn(SearchResultsTab, String, &mut Window, &mut App) + 'static>;

#[derive(Clone, Copy)]
pub(crate) struct SearchResultRowFields<'a> {
    pub(crate) id: &'a str,
    pub(crate) label: &'a str,
    pub(crate) secondary_text: &'a str,
    pub(crate) thumbnail_href: Option<&'a str>,
    pub(crate) a11y_label: &'a str,
    pub(crate) origin: SearchResultOrigin,
}

pub(crate) fn artist_fields(row: &ArtistResultDisplay) -> SearchResultRowFields<'_> {
    SearchResultRowFields {
        id: &row.id,
        label: &row.label,
        secondary_text: &row.secondary_text,
        thumbnail_href: row.thumbnail_href.as_deref(),
        a11y_label: &row.a11y_label,
        origin: row.origin,
    }
}

pub(crate) fn feed_fields(row: &FeedResultDisplay) -> SearchResultRowFields<'_> {
    SearchResultRowFields {
        id: &row.id,
        label: &row.label,
        secondary_text: &row.secondary_text,
        thumbnail_href: row.thumbnail_href.as_deref(),
        a11y_label: &row.a11y_label,
        origin: row.origin,
    }
}

pub(crate) fn track_fields(row: &TrackResultDisplay) -> SearchResultRowFields<'_> {
    SearchResultRowFields {
        id: &row.id,
        label: &row.label,
        secondary_text: &row.secondary_text,
        thumbnail_href: row.thumbnail_href.as_deref(),
        a11y_label: &row.a11y_label,
        origin: row.origin,
    }
}

pub(crate) fn render_result_row(
    tab: SearchResultsTab,
    kind: EntityKind,
    fields: SearchResultRowFields<'_>,
    thumbnail: Option<Arc<Image>>,
    on_result_select: Option<&SearchResultSelectHandler>,
) -> AnyElement {
    let row_id = fields.id.to_string();
    let element_id = SharedString::from(format!("search-results-{}-{row_id}", tab_id(tab)));
    let mut row = ListRow::new(element_id)
        .a11y_label(ListRowA11yLabel {
            label: SharedString::from(fields.a11y_label.to_string()),
        })
        .child(Thumbnail::new(kind, ThumbnailSize::Sm).image(thumbnail))
        .child(result_row_text(fields))
        .child(origin_label(fields.origin))
        .child(TagBadge::new(TagBadgeDisplay {
            kind,
            label: Some(SharedString::from(kind.label())),
        }));

    if let Some(handler) = on_result_select.cloned() {
        row = row.on_click(move |_: &ClickEvent, window, cx| {
            handler(tab, row_id.clone(), window, cx);
        });
    }

    row.into_any_element()
}

pub(crate) fn render_pending_result_row(
    tab: SearchResultsTab,
    kind: EntityKind,
    index: usize,
) -> AnyElement {
    ListRow::new(SharedString::from(format!(
        "search-results-{}-pending-{index}",
        tab_id(tab)
    )))
    .a11y_label(ListRowA11yLabel {
        label: SharedString::from("Loading search result"),
    })
    .child(Thumbnail::new(kind, ThumbnailSize::Sm))
    .child(
        div().flex_1().min_w_0().child(
            Label::new("Loading result")
                .size(FontSize::Micro)
                .color(SemanticColor::TertiaryLabel)
                .truncated(),
        ),
    )
    .child(TagBadge::new(TagBadgeDisplay {
        kind,
        label: Some(SharedString::from(kind.label())),
    }))
    .into_any_element()
}

fn result_row_text(fields: SearchResultRowFields<'_>) -> AnyElement {
    let mut text = div().flex_1().min_w_0().child(
        Label::new(fields.label.to_string())
            .size(FontSize::Micro)
            .weight(FontWeight::MEDIUM)
            .truncated(),
    );

    if !fields.secondary_text.is_empty() {
        text = text.child(
            Label::new(fields.secondary_text.to_string())
                .size(FontSize::Micro)
                .color(SemanticColor::TertiaryLabel)
                .truncated(),
        );
    }

    text.into_any_element()
}

pub(crate) fn origin_label(origin: SearchResultOrigin) -> Label {
    Label::new(origin_label_text(origin))
        .size(FontSize::Micro)
        .color(origin_label_color(origin))
        .truncated()
}

pub(crate) const fn tab_id(tab: SearchResultsTab) -> &'static str {
    match tab {
        SearchResultsTab::Artists => "artists",
        SearchResultsTab::Feeds => "feeds",
        SearchResultsTab::Tracks => "tracks",
    }
}

const fn origin_label_text(origin: SearchResultOrigin) -> &'static str {
    match origin {
        SearchResultOrigin::Library => "In Library",
        SearchResultOrigin::Index => "Index",
    }
}

const fn origin_label_color(origin: SearchResultOrigin) -> SemanticColor {
    match origin {
        SearchResultOrigin::Library => SemanticColor::Accent,
        SearchResultOrigin::Index => SemanticColor::TertiaryLabel,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_ids_are_stable() {
        assert_eq!(tab_id(SearchResultsTab::Artists), "artists");
        assert_eq!(tab_id(SearchResultsTab::Feeds), "feeds");
        assert_eq!(tab_id(SearchResultsTab::Tracks), "tracks");
    }

    #[test]
    fn origin_labels_use_membership_language() {
        assert_eq!(origin_label_text(SearchResultOrigin::Library), "In Library");
        assert_eq!(origin_label_text(SearchResultOrigin::Index), "Index");
        assert_eq!(
            origin_label_color(SearchResultOrigin::Library),
            SemanticColor::Accent
        );
    }
}
