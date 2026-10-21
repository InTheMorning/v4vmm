//! Feed page screen (ADR 0077 packet 004, ADR 0082 packet 002, ADR 0085
//! Decisions 2 to 4 and 6 to 7).
//!
//! This screen composes [`PublisherPageVm`]. It decides no section, no
//! level text and no action availability: the view model decides each
//! one. `PublisherPageContext` (Library or Index) tells the screen which
//! loading message to show and whether to show the "other releases"
//! failure notice; the sections come from the view model alone, the same
//! for each context (ADR 0084 Decision 4). The release sections draw
//! through the shared composite `release_section_list`, which the name
//! level page also uses (ADR 0084 packet 017).

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    div, prelude::*, AnyElement, App, Image, IntoElement, ParentElement, SharedString, Styled,
    Window,
};

use crate::ui::composites::music_column_scroll::music_column_tracked_scrollbar;
use crate::ui::composites::page_scroll_content::page_scroll_content;
use crate::ui::composites::release_section_list::{
    render_release_section, title_display_text, ReleaseSectionRows,
};
use crate::ui::composites::{
    CollectionArt, CollectionArtImage, DetailHeader, DetailHeaderDataRow, DetailHeaderDisplay,
    EntityKind, ListRow, ListRowA11yLabel, ThumbnailSize,
};
use crate::ui::primitives::{Label, LabelVariant, SectionHeader};
use crate::ui::tokens::{color, FontSize, Radius, SemanticColor, Spacing};
use crate::view_models::collection_art::CollectionKind;
use crate::view_models::publisher_page::{
    CoCreditedFeedVm, FeedLevel, OtherAlbumsStatus, PublisherPageContext, PublisherPageLoadDisplay,
    PublisherPageVm,
};

/// Opens the feed page of a feed GUID (ADR 0082 Decision 5). The app layer
/// supplies one; the screen builds no navigation of its own.
type OpenPublisherCallback = Rc<dyn Fn(&str, &mut Window, &mut App)>;

/// Renders a feed page from `PublisherPageVm`.
///
/// `album_thumbs` holds each album artwork image the app layer already
/// resolved, keyed by its `image_url` (ADR 0077 packet 004, orchestrator
/// fix 5). `header_art` is the loaded header art of the page: the feed's
/// own channel image or the logo mosaic of its albums (ADR 0084 Decision
/// 12). The screen fetches no image of its own.
///
/// `on_open_publisher` opens the feed page of a "Shares albums with"
/// entry's feed GUID (ADR 0082 Decision 5). `rows` gives the release rows
/// with their selection facts and their clicks (Required Change 7, ADR
/// 0084 packet 018). The app layer supplies each callback; the screen
/// builds no navigation of its own.
#[must_use]
pub(crate) fn render_publisher_page(
    vm: &PublisherPageVm,
    context: PublisherPageContext,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    header_art: CollectionArtImage,
    on_open_publisher: impl Fn(&str, &mut Window, &mut App) + 'static,
    rows: &ReleaseSectionRows,
    cx: &App,
) -> AnyElement {
    let on_open_publisher: OpenPublisherCallback = Rc::new(on_open_publisher);
    let container = div()
        .id("publisher-page")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .p(Spacing::LG.scaled(cx))
        .child(
            page_scroll_content(cx)
                .gap(Spacing::LG.scaled(cx))
                .child(render_header(vm, header_art))
                .child(render_sections(vm, context, album_thumbs, rows, cx))
                .when_some(
                    render_shares_albums_with(vm.shares_albums_with(), &on_open_publisher, cx),
                    ParentElement::child,
                ),
        );
    // ADR 0084 packet 018: the page holds the scroll handle of the detail
    // column, so a key can keep the focused row in view.
    music_column_tracked_scrollbar(container, &rows.scroll, None)
}

/// The header entity icon for a feed's confirmation level (ADR 0085
/// Decision 7 design, Required Change 7): `Artist` for the `Artist` and
/// `UnverifiedArtist` levels, `Feed` for `None`. The view model carries no
/// renderer type, so this screen makes the one choice it needs.
const fn entity_kind_for_level(level: FeedLevel) -> EntityKind {
    match level {
        FeedLevel::Artist | FeedLevel::UnverifiedArtist => EntityKind::Artist,
        FeedLevel::None => EntityKind::Feed,
    }
}

fn render_header(vm: &PublisherPageVm, header_art: CollectionArtImage) -> AnyElement {
    let data_rows = vm
        .header_facts()
        .into_iter()
        .map(|fact| DetailHeaderDataRow {
            label: fact.label.into(),
            value: fact.value.into(),
            max_lines: 3,
        })
        .collect();
    DetailHeader::new(DetailHeaderDisplay {
        kind: entity_kind_for_level(vm.level()),
        title: vm.title_text().into(),
        subtitle: None,
        data_rows,
    })
    .art(CollectionArt::new(
        header_art,
        CollectionKind::Artist,
        ThumbnailSize::XXl,
    ))
    // ADR 0083 Decision 3: the feed page title uses Display.
    .title_size(FontSize::Display)
    .into_any_element()
}

fn render_sections(
    vm: &PublisherPageVm,
    context: PublisherPageContext,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    rows: &ReleaseSectionRows,
    cx: &App,
) -> AnyElement {
    let mut body = div().flex().flex_col().gap(Spacing::LG.scaled(cx));

    if matches!(context, PublisherPageContext::Library) {
        if let OtherAlbumsStatus::Unavailable { report, detail } = vm.other_albums_status() {
            body = body.child(unavailable_notice(report, &detail, cx));
        }
    }
    for section in &rows.sections {
        body = body.child(render_release_section(section, album_thumbs, rows, cx));
    }

    body.into_any_element()
}

/// The report of a failed request for the releases that are not in the
/// Library, with its technical detail shown after it (R3-02a).
fn unavailable_notice(report: &'static str, detail: &str, cx: &App) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(Spacing::XXS.scaled(cx))
        .p(Spacing::SM.scaled(cx))
        .rounded(Radius::MD.scaled(cx))
        .border_1()
        .border_color(color(cx, SemanticColor::Separator))
        .child(
            Label::new(report)
                .variant(LabelVariant::Body)
                .color(SemanticColor::WarningLabel),
        )
        .child(
            Label::new(detail.to_owned())
                .variant(LabelVariant::Caption)
                .color(SemanticColor::TertiaryLabel),
        )
        .into_any_element()
}

/// "Shares albums with" (ADR 0082 Decision 5): one row for each entry,
/// with its title, its roles and its album count. A click opens the
/// entry's feed page through `on_open_publisher`. `None` when `entries`
/// is empty: the section is then absent.
fn render_shares_albums_with(
    entries: Vec<CoCreditedFeedVm>,
    on_open_publisher: &OpenPublisherCallback,
    cx: &App,
) -> Option<AnyElement> {
    if entries.is_empty() {
        return None;
    }
    let rows = entries
        .into_iter()
        .map(|entry| {
            let on_open_publisher = Rc::clone(on_open_publisher);
            let guid = entry.publisher_feed_guid.clone();
            let (title, missing_note) = title_display_text(&entry.title);
            let row_id = SharedString::from(format!("shares-albums-with:{guid}"));
            let mut text_block = div()
                .flex()
                .flex_col()
                .min_w_0()
                .overflow_hidden()
                .gap(Spacing::XXS.scaled(cx))
                .child(Label::new(title).variant(LabelVariant::Headline));
            if let Some(missing_note) = missing_note {
                text_block = text_block.child(
                    Label::new(missing_note)
                        .variant(LabelVariant::Caption)
                        .color(SemanticColor::TertiaryLabel),
                );
            }
            text_block = text_block
                .child(Label::new(entry.roles_and_count_text()).variant(LabelVariant::Caption));
            ListRow::new(row_id)
                .a11y_label(ListRowA11yLabel {
                    label: entry.a11y_label.clone().into(),
                })
                .on_click(move |_event, window, cx| on_open_publisher(&guid, window, cx))
                .child(div().flex_1().min_w_0().child(text_block))
                .into_any_element()
        })
        .collect::<Vec<_>>();
    Some(
        div()
            .flex()
            .flex_col()
            .gap(Spacing::SM.scaled(cx))
            .child(SectionHeader::new(
                PublisherPageVm::SHARES_ALBUMS_WITH_LABEL,
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(Spacing::XXS.scaled(cx))
                    .children(rows),
            )
            .into_any_element(),
    )
}

/// Renders the feed page's loading lifecycle (ADR 0077 packet 004). The
/// app layer selects `display`; this function only lays out its text. A
/// `Failed` display shows its report first, and its detail after it,
/// apart from the report (R3-02a).
#[must_use]
pub(crate) fn render_publisher_page_status(
    display: &PublisherPageLoadDisplay,
    cx: &App,
) -> AnyElement {
    match display {
        PublisherPageLoadDisplay::Loading { message }
        | PublisherPageLoadDisplay::Empty { message } => status_message(message.clone(), cx),
        PublisherPageLoadDisplay::Failed { report, detail } => div()
            .flex()
            .flex_1()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(Spacing::XS.scaled(cx))
            .p(Spacing::LG.scaled(cx))
            .child(
                Label::new(*report)
                    .variant(LabelVariant::Body)
                    .color(SemanticColor::WarningLabel),
            )
            .child(
                Label::new(detail.clone())
                    .variant(LabelVariant::Caption)
                    .color(SemanticColor::TertiaryLabel),
            )
            .into_any_element(),
    }
}

fn status_message(message: String, cx: &App) -> AnyElement {
    div()
        .flex()
        .flex_1()
        .items_center()
        .justify_center()
        .p(Spacing::LG.scaled(cx))
        .text_color(color(cx, SemanticColor::TertiaryLabel))
        .child(message)
        .into_any_element()
}
