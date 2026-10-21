//! The name level page screen (ADR 0084 Decision 7, packet 017).
//!
//! This screen composes [`NameLevelPageVm`]. It decides no section, no
//! text and no action availability: the view model decides each one. The
//! release sections draw through the shared composite
//! `release_section_list`, the same as on the feed page.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui::{
    div, prelude::*, AnyElement, App, ClickEvent, Image, IntoElement, ParentElement, SharedString,
    Styled, Window,
};

use crate::ui::composites::music_column_scroll::music_column_tracked_scrollbar;
use crate::ui::composites::page_scroll_content::page_scroll_content;
use crate::ui::composites::release_section_list::{
    render_release_list_status, render_release_section, ReleaseSectionRows,
};
use crate::ui::composites::{
    CollectionArt, CollectionArtImage, DetailHeader, DetailHeaderDataRow, DetailHeaderDisplay,
    EntityKind, ThumbnailSize,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::{Button, Label, LabelVariant};
use crate::ui::tokens::{FontSize, SemanticColor, Spacing};
use crate::view_models::collection_art::CollectionKind;
use crate::view_models::name_level_page::{MatchedFeedNoticeVm, NameLevelPageVm};

/// Renders a name level page from `NameLevelPageVm`.
///
/// `album_thumbs` holds each cover that the app layer already resolved,
/// keyed by its URL. `header_art` is the loaded logo mosaic of the page
/// (ADR 0084 Decision 12). `rows` gives the release rows with their
/// selection facts and their clicks. `on_open_feed` opens the feed page of
/// the notice of decision D16. The screen fetches no image of its own and
/// builds no navigation of its own.
#[must_use]
pub(crate) fn render_name_level_page(
    vm: &NameLevelPageVm,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    header_art: CollectionArtImage,
    rows: &ReleaseSectionRows,
    on_open_feed: impl Fn(&str, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let mut body = page_scroll_content(cx)
        .gap(Spacing::LG.scaled(cx))
        .child(render_header(vm, header_art));
    if let Some(notice) = vm.matched_feed_notice() {
        body = body.child(render_matched_feed_notice(notice, on_open_feed, cx));
    }
    if let Some(status) = vm.index_status() {
        body = body.child(render_release_list_status(&status, cx));
    }
    if let Some(message) = vm.empty_message() {
        body = body.child(
            Label::new(message)
                .variant(LabelVariant::Body)
                .color(SemanticColor::TertiaryLabel),
        );
    }
    for section in &rows.sections {
        body = body.child(render_release_section(section, album_thumbs, rows, cx));
    }
    let container = div()
        .id("name-level-page")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .p(Spacing::LG.scaled(cx))
        .child(body);
    // ADR 0084 packet 018: the page holds the scroll handle of the detail
    // column, so a key can keep the focused row in view.
    music_column_tracked_scrollbar(container, &rows.scroll, None)
}

fn render_header(vm: &NameLevelPageVm, header_art: CollectionArtImage) -> AnyElement {
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
        kind: EntityKind::Artist,
        title: vm.title_text().into(),
        subtitle: None,
        data_rows,
    })
    .art(CollectionArt::new(
        header_art,
        CollectionKind::Artist,
        ThumbnailSize::XXl,
    ))
    // ADR 0083 Decision 3: an artist page title uses Display.
    .title_size(FontSize::Display)
    .into_any_element()
}

/// The notice of a name that now matches exactly one artist feed, with
/// the open action of that feed page (packet 017 decision D16).
fn render_matched_feed_notice(
    notice: MatchedFeedNoticeVm,
    on_open_feed: impl Fn(&str, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let feed_guid = notice.feed_guid.clone();
    div()
        .flex()
        .flex_col()
        .items_start()
        .gap(Spacing::XS.scaled(cx))
        .child(Label::new(notice.text).variant(LabelVariant::Body))
        .child(
            Button::styled(
                SharedString::from(format!("name-level-open-feed:{}", notice.feed_guid)),
                ControlStyle::Ghost,
            )
            .label(notice.action_label)
            .a11y_label(notice.a11y_label)
            .on_click(move |_event: &ClickEvent, window, cx| {
                on_open_feed(&feed_guid, window, cx);
            }),
        )
        .into_any_element()
}
