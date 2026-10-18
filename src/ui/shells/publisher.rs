//! Publisher page screen (ADR 0077 packet 004, ADR 0082 packet 002).
//!
//! This screen composes [`PublisherPageVm`]. It decides no group, no role
//! label and no action availability: the view model decides each one.
//! `PublisherPageContext` (Library or Index) tells the screen which view
//! model methods build which scope; the screen invents no scope of its
//! own.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    div, prelude::*, AnyElement, App, Image, IntoElement, ParentElement, SharedString, Styled,
    Window,
};

use crate::ui::composites::music_column_scroll::music_column_scrollbar;
use crate::ui::composites::page_scroll_content::page_scroll_content;
use crate::ui::composites::{
    DetailHeader, DetailHeaderDataRow, DetailHeaderDisplay, EntityKind, ListRow, ListRowA11yLabel,
    TagBadge, TagBadgeDisplay, Thumbnail, ThumbnailSize,
};
use crate::ui::primitives::{Label, LabelVariant, SectionHeader, SectionHeaderLevel};
use crate::ui::tokens::{color, FontSize, Radius, SemanticColor, Spacing};
use crate::view_models::publisher_page::{
    AlbumNameGroupVm, AlbumRoleDisplay, AlbumRoleSubgroupVm, CoCreditedFeedVm, OtherAlbumsStatus,
    PublisherPageAlbumVm, PublisherPageContext, PublisherPageLoadDisplay, PublisherPageVm,
    TitleDisplay,
};

/// Opens the publisher page of a feed GUID (ADR 0082 Decision 5). The app
/// layer supplies one; the screen builds no navigation of its own.
type OpenPublisherCallback = Rc<dyn Fn(&str, &mut Window, &mut App)>;

/// Renders a publisher page from `PublisherPageVm`.
///
/// On a Library page (`PublisherPageContext::Library`) the screen shows
/// `library_groups` and `other_groups` as two scopes, each with the ADR
/// 0082 Decision 2 groups inside it, and shows the report of
/// `other_albums_status` first, with its detail after it, when that
/// status is `Unavailable`. On an Index page (`PublisherPageContext::Index`)
/// the screen shows `index_groups`, and marks each album with
/// `in_library`.
///
/// `album_thumbs` holds each album artwork image the app layer already
/// resolved, keyed by its `image_url` (ADR 0077 packet 004, orchestrator
/// fix 5). The screen fetches no image of its own.
///
/// `on_open_publisher` opens the publisher page of a "Shares albums with"
/// entry's feed GUID (ADR 0082 Decision 5). The app layer supplies it; the
/// screen builds no navigation of its own.
#[must_use]
pub(crate) fn render_publisher_page(
    vm: &PublisherPageVm,
    context: PublisherPageContext,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    on_open_publisher: impl Fn(&str, &mut Window, &mut App) + 'static,
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
                .child(render_header(vm))
                .child(render_groups(vm, context, album_thumbs, cx))
                .when_some(
                    render_shares_albums_with(vm.shares_albums_with(), &on_open_publisher, cx),
                    ParentElement::child,
                ),
        );
    music_column_scrollbar(container, "publisher-page-scrollbar")
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
    // ADR 0083 Decision 3: the publisher page title uses Display.
    .title_size(FontSize::Display)
    .into_any_element()
}

fn render_groups(
    vm: &PublisherPageVm,
    context: PublisherPageContext,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    cx: &App,
) -> AnyElement {
    let mark_in_library = context.marks_in_library();
    let mut sections = div().flex().flex_col().gap(Spacing::LG.scaled(cx));

    match context {
        PublisherPageContext::Library => {
            if let OtherAlbumsStatus::Unavailable { report, detail } = vm.other_albums_status() {
                sections = sections.child(unavailable_notice(report, &detail, cx));
            }
            // ADR 0082 packet 002 correction: `library_scopes` already
            // drops an empty scope. This loop decides no emptiness of its
            // own.
            for scope in vm.library_scopes() {
                sections = sections.child(scope_section(
                    scope.label,
                    &scope.groups,
                    mark_in_library,
                    album_thumbs,
                    cx,
                ));
            }
        }
        PublisherPageContext::Index => {
            for group in &vm.index_groups() {
                sections = sections.child(album_name_group_section(
                    group,
                    mark_in_library,
                    album_thumbs,
                    cx,
                ));
            }
        }
    }

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

/// One Library page scope (ADR 0082 packet 002): the Library scope or the
/// other scope, each with the ADR 0082 Decision 2 groups inside it.
fn scope_section(
    label: &'static str,
    groups: &[AlbumNameGroupVm],
    mark_in_library: bool,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    cx: &App,
) -> AnyElement {
    let mut body = div().flex().flex_col().gap(Spacing::LG.scaled(cx));
    for group in groups {
        body = body.child(album_name_group_section(
            group,
            mark_in_library,
            album_thumbs,
            cx,
        ));
    }
    div()
        .flex()
        .flex_col()
        .gap(Spacing::SM.scaled(cx))
        .child(SectionHeader::new(label).level(SectionHeaderLevel::Container))
        .child(body)
        .into_any_element()
}

/// One album name group, with its role subgroups (ADR 0082 Decision 2).
fn album_name_group_section(
    group: &AlbumNameGroupVm,
    mark_in_library: bool,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    cx: &App,
) -> AnyElement {
    let mut body = div().flex().flex_col().gap(Spacing::SM.scaled(cx));
    for subgroup in &group.subgroups {
        body = body.child(role_subgroup_block(
            subgroup,
            mark_in_library,
            album_thumbs,
            cx,
        ));
    }
    div()
        .flex()
        .flex_col()
        .gap(Spacing::XS.scaled(cx))
        .child(SectionHeader::new(group.group.label()))
        .child(body)
        .into_any_element()
}

/// One role subgroup's caption and its album rows (ADR 0082 Decision 2).
fn role_subgroup_block(
    subgroup: &AlbumRoleSubgroupVm,
    mark_in_library: bool,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    cx: &App,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(Spacing::XXS.scaled(cx))
        .child(
            Label::new(subgroup.caption.clone())
                .variant(LabelVariant::Caption)
                .color(SemanticColor::SecondaryLabel),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(Spacing::XXS.scaled(cx))
                .children(
                    subgroup
                        .albums
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
    text_block = text_block.child(render_role(&album.role));

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

/// A row's role text and source, ready to view (ADR 0082 Decision 3). A
/// conflict shows in a warning tone. A stated role (agreed or one-side)
/// shows in a plain secondary tone, never the accent tone of a clickable
/// item: this text opens nothing. An unstated role shows in a quiet tone.
/// Color alone is not the only difference: the text itself names the
/// state.
fn render_role(role: &AlbumRoleDisplay) -> AnyElement {
    let tone = if role.is_conflict() {
        SemanticColor::WarningLabel
    } else if role.is_stated() {
        SemanticColor::SecondaryLabel
    } else {
        SemanticColor::TertiaryLabel
    };
    Label::new(role.display_text())
        .variant(LabelVariant::Micro)
        .color(tone)
        .into_any_element()
}

/// "Shares albums with" (ADR 0082 Decision 5): one row for each entry,
/// with its title, its roles and its album count. A click opens the
/// entry's publisher page through `on_open_publisher`. `None` when
/// `entries` is empty: the section is then absent.
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
