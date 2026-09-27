//! Publisher page screen (ADR 0077 packet 004, ADR 0078).
//!
//! This screen composes [`PublisherPageVm`]. It decides no page type, no
//! role label, no group and no action availability: the view model decides
//! each one. `PublisherPageContext` (Library or Index) tells the screen
//! which view model methods build which section; the screen invents no
//! group of its own.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui::{
    div, prelude::*, AnyElement, App, Image, IntoElement, ParentElement, SharedString, Styled,
};

use crate::ui::composites::{
    DetailHeader, DetailHeaderDataRow, DetailHeaderDisplay, EntityKind, ListRow, TagBadge,
    TagBadgeDisplay, Thumbnail, ThumbnailSize,
};
use crate::ui::primitives::{Label, LabelVariant, SectionHeader};
use crate::ui::tokens::{color, Radius, SemanticColor, Spacing};
use crate::view_models::publisher_page::{
    AlbumRoleDisplay, OtherAlbumsStatus, PublisherPageAlbumVm, PublisherPageContext,
    PublisherPageLoadDisplay, PublisherPageVm, TitleDisplay,
};

/// Renders a publisher page from `PublisherPageVm`.
///
/// On a Library page (`PublisherPageContext::Library`) the screen shows
/// `library_albums` and `other_albums` as two groups, and shows the report
/// of `other_albums_status` first, with its detail after it, when that
/// status is `Unavailable`. On an Index page (`PublisherPageContext::Index`)
/// the screen shows `owned_albums` and `listed_by_albums`, and marks each
/// album with `in_library`.
///
/// `album_thumbs` holds each album artwork image the app layer already
/// resolved, keyed by its `image_url` (ADR 0077 packet 004, orchestrator
/// fix 5). The screen fetches no image of its own.
#[must_use]
pub(crate) fn render_publisher_page(
    vm: &PublisherPageVm,
    context: PublisherPageContext,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    cx: &App,
) -> AnyElement {
    div()
        .id("publisher-page")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .overflow_y_scroll()
        .gap(Spacing::LG.scaled(cx))
        .p(Spacing::LG.scaled(cx))
        .child(render_header(vm))
        .child(render_groups(vm, context, album_thumbs, cx))
        .into_any_element()
}

fn render_header(vm: &PublisherPageVm) -> AnyElement {
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
        kind: EntityKind::Publisher,
        title: vm.title_text().into(),
        subtitle: None,
        data_rows,
    })
    .into_any_element()
}

fn render_groups(
    vm: &PublisherPageVm,
    context: PublisherPageContext,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    cx: &App,
) -> AnyElement {
    let (first_label, second_label) = context.group_labels();
    let mark_in_library = context.marks_in_library();
    let mut sections = div().flex().flex_col().gap(Spacing::LG.scaled(cx));

    if matches!(context, PublisherPageContext::Library) {
        if let OtherAlbumsStatus::Unavailable { report, detail } = vm.other_albums_status() {
            sections = sections.child(unavailable_notice(report, &detail, cx));
        }
    }

    let (first_albums, second_albums) = match context {
        PublisherPageContext::Library => (vm.library_albums(), vm.other_albums()),
        PublisherPageContext::Index => (vm.owned_albums(), vm.listed_by_albums()),
    };
    sections = sections.child(album_group(
        first_label,
        &first_albums,
        mark_in_library,
        album_thumbs,
        cx,
    ));
    sections = sections.child(album_group(
        second_label,
        &second_albums,
        mark_in_library,
        album_thumbs,
        cx,
    ));

    sections.into_any_element()
}

/// The report of a failed request for the albums that are not in the
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

fn album_group(
    label: &'static str,
    albums: &[PublisherPageAlbumVm],
    mark_in_library: bool,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    cx: &App,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(Spacing::SM.scaled(cx))
        .child(SectionHeader::new(label))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(Spacing::XXS.scaled(cx))
                .children(
                    albums
                        .iter()
                        .map(|album| render_album_row(album, mark_in_library, album_thumbs, cx)),
                ),
        )
        .into_any_element()
}

fn render_album_row(
    album: &PublisherPageAlbumVm,
    mark_in_library: bool,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    cx: &App,
) -> AnyElement {
    let (title, missing_note) = title_display_text(&album.title);
    let row_id = album
        .feed_guid
        .clone()
        .unwrap_or_else(|| format!("no-guid:{title}"));
    let thumb_image = album
        .image_url
        .as_deref()
        .and_then(|url| album_thumbs.get(url))
        .and_then(Clone::clone);

    // Column text does not call `truncate()` (ADR 0063,
    // docs/troubleshooting/column-text-truncation.md). `overflow_hidden()`
    // is the required mitigation for text stacked in a flex column.
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
    if let Some(artist) = &album.artist {
        text_block =
            text_block.child(Label::new(artist.display_text()).variant(LabelVariant::Caption));
    }
    if let Some(role_element) = render_role(&album.role) {
        text_block = text_block.child(role_element);
    }

    let mut row = ListRow::new(SharedString::from(format!("publisher-album:{row_id}")))
        .child(Thumbnail::new(EntityKind::Release, ThumbnailSize::Sm).image(thumb_image))
        .child(div().flex_1().min_w_0().child(text_block));

    let mut marks = Vec::new();
    if album.not_listed {
        marks.push(PublisherPageAlbumVm::NOT_LISTED_LABEL);
    }
    if mark_in_library && album.in_library {
        marks.push(PublisherPageAlbumVm::IN_LIBRARY_LABEL);
    }
    if !marks.is_empty() {
        let mut trailing = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(Spacing::XS.scaled(cx));
        for mark in marks {
            trailing = trailing.child(TagBadge::new(TagBadgeDisplay {
                kind: EntityKind::Generic,
                label: Some(SharedString::from(mark)),
            }));
        }
        row = row.child(trailing);
    }

    row.into_any_element()
}

fn title_display_text(title: &TitleDisplay) -> (String, Option<&'static str>) {
    match title {
        TitleDisplay::Stated(text) => (text.clone(), None),
        TitleDisplay::Missing(guid) => (guid.clone(), Some(TitleDisplay::MISSING_LABEL)),
    }
}

/// A role, ready to view. `Stated` and `Assumed` show through a different
/// color, so an assumed role never looks like a stated fact (R3-07).
fn render_role(role: &AlbumRoleDisplay) -> Option<AnyElement> {
    if let Some(conflict_text) = role.conflict_text() {
        return Some(
            Label::new(conflict_text)
                .variant(LabelVariant::Caption)
                .color(SemanticColor::WarningLabel)
                .into_any_element(),
        );
    }
    let text = role.text()?;
    let tone = if role.is_stated() {
        SemanticColor::Accent
    } else {
        SemanticColor::TertiaryLabel
    };
    Some(
        Label::new(text.to_owned())
            .variant(LabelVariant::Micro)
            .color(tone)
            .into_any_element(),
    )
}

/// Renders the publisher page's loading lifecycle (ADR 0077 packet 004).
/// The app layer selects `display`; this function only lays out its text.
/// A `Failed` display shows its report first, and its detail after it,
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
