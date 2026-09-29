//! Name-match track page screen (ADR 0077 packet 006, Accepted Refinement
//! "Index artist page by name").
//!
//! This screen composes [`NameMatchPageVm`]. The Index name query becomes a
//! search result: the screen shows no artist identity, no role, and no page
//! type (ADR 0077 Decision 1). Each track row reuses the shared Index
//! result row from `search_result_rows`.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui::{div, prelude::*, AnyElement, App, Image, IntoElement, ParentElement, Styled};

use crate::ui::composites::EntityKind;
use crate::ui::primitives::{Label, LabelVariant, SectionHeader};
use crate::ui::shells::search_result_rows::{
    render_result_row, track_fields, SearchResultSelectHandler,
};
use crate::ui::tokens::{color, SemanticColor, Spacing};
use crate::view_models::name_match_page::{NameMatchPageLoadDisplay, NameMatchPageVm};
use crate::view_models::search_results::SearchResultsTab;

/// Renders a name-match track page from `NameMatchPageVm`.
///
/// `track_thumbs` holds each track artwork image the app layer already
/// resolved, keyed by its thumbnail href. The screen fetches no image of
/// its own. `on_result_select` opens the existing Index track detail of the
/// clicked row's track (Required Change 3).
#[must_use]
pub(crate) fn render_name_match_page(
    vm: &NameMatchPageVm,
    track_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    on_result_select: &SearchResultSelectHandler,
    cx: &App,
) -> AnyElement {
    div()
        .id("name-match-page")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .overflow_y_scroll()
        .gap(Spacing::LG.scaled(cx))
        .p(Spacing::LG.scaled(cx))
        .child(SectionHeader::new(vm.title_text()))
        .child(render_body(vm, track_thumbs, on_result_select, cx))
        .into_any_element()
}

fn render_body(
    vm: &NameMatchPageVm,
    track_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    on_result_select: &SearchResultSelectHandler,
    cx: &App,
) -> AnyElement {
    if vm.is_empty() {
        return status_message(vm.no_track_message(), cx);
    }

    let mut list = div().flex().flex_col().gap(Spacing::XXS.scaled(cx));
    for row in vm.rows() {
        let thumbnail = row
            .thumbnail_href
            .as_deref()
            .and_then(|href| track_thumbs.get(href))
            .and_then(Clone::clone);
        list = list.child(render_result_row(
            SearchResultsTab::Tracks,
            EntityKind::Track,
            track_fields(&row),
            thumbnail,
            Some(on_result_select),
        ));
    }

    let mut body = div()
        .flex()
        .flex_col()
        .gap(Spacing::SM.scaled(cx))
        .child(list);
    if vm.has_more() {
        body = body.child(
            Label::new(NameMatchPageVm::MORE_TRACKS_MESSAGE)
                .variant(LabelVariant::Caption)
                .color(SemanticColor::TertiaryLabel),
        );
    }
    body.into_any_element()
}

/// Renders the name-match page's loading lifecycle (packet 006). The app
/// layer selects `display`; this function only lays out its text. A
/// `Failed` display shows its report first, and its detail after it, apart
/// from the report, the same order `render_publisher_page_status` uses.
#[must_use]
pub(crate) fn render_name_match_page_status(
    display: &NameMatchPageLoadDisplay,
    cx: &App,
) -> AnyElement {
    match display {
        NameMatchPageLoadDisplay::Loading { message }
        | NameMatchPageLoadDisplay::Empty { message } => status_message(message.clone(), cx),
        NameMatchPageLoadDisplay::Failed { report, detail } => div()
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
