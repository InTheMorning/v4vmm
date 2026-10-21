//! The Discover views list column (ADR 0084 Decision 3): "New releases",
//! then one row for each saved search (packet 012).
//!
//! Moved out of `destination_lists.rs` to keep that file under the ADR
//! 0038 Task 007 line ceiling. The view model gives each label; this shell
//! owns only layout and interaction wiring.

#![warn(clippy::pedantic)]

use gpui::{div, prelude::*, AnyElement, ClickEvent, Context, FontWeight, SharedString, Styled};

use crate::library::LibraryApp;
use crate::ui::composites::music_column_scroll::music_column_scrollbar;
use crate::ui::composites::page_scroll_content::page_scroll_content;
use crate::ui::composites::{ListRow, ListRowA11yLabel};
use crate::ui::icons::{Icon, IconName, IconSize};
use crate::ui::primitives::Label;
use crate::ui::style::spacing;
use crate::ui::tokens::{FontSize, SemanticColor};
use crate::view_models::library::{DiscoverViewIcon, DiscoverViewRowTarget, DiscoverViewRowVm};

/// Maps the semantic icon of a Discover view row to a named icon.
const fn icon_name(icon: DiscoverViewIcon) -> IconName {
    match icon {
        DiscoverViewIcon::NewReleases => IconName::Discover,
        DiscoverViewIcon::SavedSearch => IconName::Search,
    }
}

/// Renders the Discover views list column (ADR 0084 Decision 3): "New
/// releases", then the saved searches.
pub(crate) fn render_library_discover_views_list(
    rows: &[DiscoverViewRowVm],
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let container = div()
        .id("music-discover-views-list")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .px(spacing::MD)
        .pb(spacing::MD)
        .child(
            page_scroll_content(cx)
                .gap(spacing::XXS)
                .children(rows.iter().map(|row| render_discover_view_row(row, cx))),
        );
    music_column_scrollbar(container, "music-discover-views-list-scrollbar")
}

fn render_discover_view_row(row: &DiscoverViewRowVm, cx: &mut Context<LibraryApp>) -> AnyElement {
    let target = row.target.clone();

    ListRow::new(SharedString::from(row.element_id.clone()))
        .a11y_label(ListRowA11yLabel {
            label: SharedString::from(row.a11y_label.clone()),
        })
        .selected(row.selected)
        .on_click(
            cx.listener(move |this, _: &ClickEvent, _window, cx| match &target {
                DiscoverViewRowTarget::View(view) => this.open_discover_view(*view, cx),
                DiscoverViewRowTarget::SavedSearch(search) => {
                    this.open_saved_search_row(search, cx);
                }
            }),
        )
        .child(
            div()
                .flex_shrink_0()
                .pl(spacing::MD)
                .child(Icon::new(icon_name(row.icon)).size(IconSize::Action)),
        )
        .child(
            // ADR 0063: stacked column text calls no `truncate()`. The
            // container clips the text with `overflow_hidden()`.
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .pl(spacing::SM)
                .child(
                    Label::new(row.label.clone())
                        .color(if row.selected {
                            SemanticColor::Accent
                        } else {
                            SemanticColor::Label
                        })
                        .weight(FontWeight::MEDIUM),
                )
                .child(
                    Label::new(row.subtitle.clone())
                        .size(FontSize::Micro)
                        .color(SemanticColor::TertiaryLabel),
                ),
        )
        .into_any_element()
}
