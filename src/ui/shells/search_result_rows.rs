//! The rows of the search results in the Music list column (ADR 0084
//! Decision 2, packet 010).
//!
//! The view model gives each text, each state word, each badge and the
//! selection facts of each row. This shell lays out the row in the order
//! of a Releases list row: the checkbox, the thumbnail, the title and the
//! second line, the state word and the badge.

#![warn(clippy::pedantic)]

use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    div, AnyElement, App, ClickEvent, FontWeight, Image, IntoElement, Modifiers, ParentElement,
    SharedString, Styled, Window,
};

use crate::ui::composites::release_download_state::render_release_download_state;
use crate::ui::composites::{
    CollectionArt, CollectionArtImage, EntityKind, ListRow, ListRowA11yLabel, TagBadge,
    TagBadgeDisplay, Thumbnail, ThumbnailSize,
};
use crate::ui::primitives::{Checkbox, CheckboxState, Label};
use crate::ui::tokens::{FontSize, SemanticColor};
use crate::view_models::collection_art::CollectionKind;
use crate::view_models::detail_selection::DetailRowKey;
use crate::view_models::list_selection::RowCheckState;
use crate::view_models::search_results::{
    SearchResultOrigin, SearchResultRowVm, SearchResultsTab, SearchRowBadge,
};

/// A click on the body of a search row: the tab, the row id, the selection
/// key of the row and the modifier keys. The app layer opens the page, or
/// changes the selection in selection mode.
pub(crate) type SearchResultSelectHandler = Rc<
    dyn Fn(SearchResultsTab, String, Option<DetailRowKey>, Modifiers, &mut Window, &mut App)
        + 'static,
>;

/// A click on the checkbox of a search row.
pub(crate) type SearchResultToggleHandler = Rc<dyn Fn(DetailRowKey, &mut Window, &mut App)>;

/// The callbacks of one search row.
#[derive(Clone, Copy)]
pub(crate) struct SearchResultRowHandlers<'a> {
    pub(crate) select: Option<&'a SearchResultSelectHandler>,
    pub(crate) toggle: Option<&'a SearchResultToggleHandler>,
}

/// The leading image of one search row: the thumbnail, or the logo frame of
/// a row that has art in its view model (packet 028 D3).
pub(crate) enum SearchRowLeading {
    /// The thumbnail, with its image when it loaded.
    Thumbnail(Option<Arc<Image>>),
    /// The logo frame of a Library name row, as the Artists list shows it.
    /// The view model gives the kind of the frame.
    Collection {
        image: CollectionArtImage,
        kind: CollectionKind,
    },
}

/// Renders one search row.
pub(crate) fn render_result_row(
    tab: SearchResultsTab,
    row: SearchResultRowVm,
    leading: SearchRowLeading,
    handlers: SearchResultRowHandlers<'_>,
    cx: &App,
) -> AnyElement {
    let kind = entity_kind_for_tab(tab);
    let row_id = row.id.clone();
    let element_id = format!("search-results-{}-{row_id}", tab_id(tab));
    let selection = row.selection.clone();
    let mut list_row = ListRow::new(SharedString::from(element_id.clone()))
        .a11y_label(ListRowA11yLabel {
            label: SharedString::from(row.a11y_label.clone()),
        })
        .selected(selection.is_marked())
        .focused(selection.focused);

    // ADR 0084 Decision 10 Amendment, 2026-10-19: the checkbox shows only
    // in selection mode, as on a Releases list row.
    if let (true, Some(key), Some(toggle)) =
        (selection.checkbox_visible, row.key.clone(), handlers.toggle)
    {
        let toggle = Rc::clone(toggle);
        list_row = list_row.checkbox(
            Checkbox::new(
                SharedString::from(format!("{element_id}-checkbox")),
                checkbox_state_for(selection.check_state),
            )
            .a11y_label(selection.checkbox_a11y_label.clone())
            .on_click(move |_event: &ClickEvent, window, cx| {
                cx.stop_propagation();
                toggle(key.clone(), window, cx);
            })
            .into_any_element(),
        );
    }

    list_row = match leading {
        SearchRowLeading::Thumbnail(thumbnail) => {
            list_row.child(Thumbnail::new(kind, ThumbnailSize::Sm).image(thumbnail))
        }
        SearchRowLeading::Collection { image, kind } => {
            list_row.child(CollectionArt::new(image, kind, ThumbnailSize::Sm))
        }
    };
    list_row = list_row.child(result_row_text(&row));
    if let Some(download_state) = &row.download_state {
        list_row = list_row.child(render_release_download_state(
            download_state,
            format!("{element_id}-download-state"),
            cx,
        ));
    } else if let Some(state_word) = row.state_word.clone() {
        list_row = list_row.child(
            Label::new(state_word)
                .size(FontSize::Micro)
                .color(SemanticColor::SecondaryLabel),
        );
    }
    if let Some(badge) = &row.badge {
        list_row = list_row.child(TagBadge::new(TagBadgeDisplay {
            kind: entity_kind_for_badge(badge),
            label: Some(SharedString::from(badge.label().to_owned())),
        }));
    }

    if let Some(handler) = handlers.select.cloned() {
        let key = row.key;
        list_row = list_row.on_click(move |event: &ClickEvent, window, cx| {
            handler(
                tab,
                row_id.clone(),
                key.clone(),
                event.modifiers(),
                window,
                cx,
            );
        });
    }

    list_row.into_any_element()
}

/// Renders one row that waits for the `MusicIndex` results.
pub(crate) fn render_pending_result_row(tab: SearchResultsTab, index: usize) -> AnyElement {
    let kind = entity_kind_for_tab(tab);
    ListRow::new(SharedString::from(format!(
        "search-results-{}-pending-{index}",
        tab_id(tab)
    )))
    .a11y_label(ListRowA11yLabel {
        label: SharedString::from("Loading search result"),
    })
    .child(Thumbnail::new(kind, ThumbnailSize::Sm))
    .child(
        div().flex_1().min_w_0().overflow_hidden().child(
            Label::new("Loading result")
                .size(FontSize::Micro)
                .color(SemanticColor::TertiaryLabel),
        ),
    )
    .into_any_element()
}

/// The title and the second line. Column text does not call `truncate()`
/// (ADR 0063, docs/troubleshooting/column-text-truncation.md).
/// `overflow_hidden()` is the required mitigation for text stacked in a
/// flex column.
fn result_row_text(row: &SearchResultRowVm) -> AnyElement {
    let mut text = div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .overflow_hidden()
        .child(
            Label::new(row.label.clone())
                .size(FontSize::Micro)
                .weight(FontWeight::MEDIUM),
        );

    if !row.secondary_text.is_empty() {
        text = text.child(
            Label::new(row.secondary_text.clone())
                .size(FontSize::Micro)
                .color(SemanticColor::TertiaryLabel),
        );
    }

    text.into_any_element()
}

/// The origin word of an Index detail page header. A search row shows no
/// origin word (packet 010 decision D4).
pub(crate) fn origin_label(origin: SearchResultOrigin) -> Label {
    Label::new(origin_label_text(origin))
        .size(FontSize::Micro)
        .color(origin_label_color(origin))
}

pub(crate) const fn tab_id(tab: SearchResultsTab) -> &'static str {
    match tab {
        SearchResultsTab::Releases => "releases",
        SearchResultsTab::Artists => "artists",
        SearchResultsTab::Tracks => "tracks",
    }
}

const fn entity_kind_for_tab(tab: SearchResultsTab) -> EntityKind {
    match tab {
        SearchResultsTab::Releases => EntityKind::Release,
        SearchResultsTab::Artists => EntityKind::Artist,
        SearchResultsTab::Tracks => EntityKind::Track,
    }
}

/// The entity color of a badge. A role badge has the feed color. Its text
/// tells it apart from the "Feed" badge, so color is not the only
/// difference (ADR 0084 packet 029).
const fn entity_kind_for_badge(badge: &SearchRowBadge) -> EntityKind {
    match badge {
        SearchRowBadge::Release(_) => EntityKind::Release,
        SearchRowBadge::Artist => EntityKind::Artist,
        SearchRowBadge::Roles(_) | SearchRowBadge::Feed => EntityKind::Feed,
    }
}

/// Maps the typed check state of the view model to the state of the
/// checkbox primitive. A search row has one member, so it never reads
/// mixed.
fn checkbox_state_for(check_state: RowCheckState) -> CheckboxState {
    match check_state {
        RowCheckState::Checked => CheckboxState::Checked,
        RowCheckState::Mixed => CheckboxState::Mixed,
        RowCheckState::Clear => CheckboxState::NotChecked,
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
        assert_eq!(tab_id(SearchResultsTab::Releases), "releases");
        assert_eq!(tab_id(SearchResultsTab::Artists), "artists");
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
